// SPDX-License-Identifier: MIT
//
// Zips a release: mods/yuta.demaster/ with the mod DLL and manifest, demaster.exe,
// demaster_live.dll (the live music engine), the default
// demaster.toml, the players' README.txt (tools/package/README.txt), config.md and LICENSES.txt,
// ready to unpack into the Fahrenheit folder. No game data: the player builds that with
// demaster.exe from their own disc.
//
// LICENSES.txt is tools/package/LICENSES.txt, then every crate compiled into demaster.exe with its
// licence and the licence files those crates ship (from `cargo metadata`, so cargo must be able to
// fetch ffx-rust), then THIRD-PARTY-NOTICES.
//
// usage: dotnet run tools/package.cs -- <out.zip> [demaster.exe]
//   after: dotnet build -c Release ffx-demaster.slnx
//   and:   tools/build-windows.sh (Linux/WSL) or
//          cargo build --release -p demaster --target x86_64-pc-windows-msvc (Windows)

using System.Diagnostics;
using System.IO.Compression;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;

if (args.Length is < 1 or > 2) {
    Console.Error.WriteLine("usage: dotnet run tools/package.cs -- <out.zip> [demaster.exe]");
    return 1;
}

const string ID = "yuta.demaster";
string root = Path.GetFullPath(Path.Combine(Path.GetDirectoryName(this_file())!, ".."));
string mod  = Path.Combine(root, "mod", "artifacts", "deploy", "rel", "mods", ID);
if (!File.Exists(Path.Combine(mod, $"{ID}.dll"))) {
    Console.Error.WriteLine("build the mod first: dotnet build -c Release ffx-demaster.slnx");
    return 1;
}
string exe = args.Length > 1
    ? Path.GetFullPath(args[1])
    : Path.Combine(root, "target", "x86_64-pc-windows-msvc", "release", "demaster.exe");
if (!File.Exists(exe)) {
    Console.Error.WriteLine($"{exe} is missing: build the converter for Windows first (tools/build-windows.sh)");
    return 1;
}

// The live music engine (audio.music_engine = "live"), 32-bit like FFX.exe: tools/build-live.sh.
string live = Path.Combine(root, "target", "i686-pc-windows-msvc", "release", "demaster_live.dll");
if (!File.Exists(live)) {
    Console.Error.WriteLine($"{live} is missing: build it first (tools/build-live.sh)");
    return 1;
}

string licences;
try {
    licences = File.ReadAllText(Path.Combine(root, "tools", "package", "LICENSES.txt"))
             + crate_licences(root)
             + "\n\n" + File.ReadAllText(Path.Combine(root, "THIRD-PARTY-NOTICES"));
} catch (Exception e) {
    Console.Error.WriteLine($"LICENSES.txt: {e.Message}");
    return 1;
}

// What the mod build may contain; anything else (efl/, build/ from a local test) stays out.
string[] mod_files = [$"{ID}.dll", $"{ID}.deps.json", $"{ID}.runtimeconfig.json", $"{ID}.manifest.json", $"{ID}.pdb"];

string out_path = Path.GetFullPath(args[0]);
File.Delete(out_path);
using (ZipArchive zip = ZipFile.Open(out_path, ZipArchiveMode.Create)) {
    // Entry names use '/' on every OS, so the zip unpacks the same on Windows.
    void add(string file, string name) {
        zip.CreateEntryFromFile(file, $"mods/{ID}/{name}");
        Console.WriteLine($"mods/{ID}/{name}");
    }
    // Text files get CRLF, for Notepad.
    void add_text(string text, string name) {
        ZipArchiveEntry entry = zip.CreateEntry($"mods/{ID}/{name}");
        using (StreamWriter w = new(entry.Open(), new UTF8Encoding(false)))
            w.Write(text.ReplaceLineEndings("\r\n"));
        Console.WriteLine($"mods/{ID}/{name}");
    }
    string text(params string[] path) => File.ReadAllText(Path.Combine([root, .. path]));

    foreach (string f in mod_files) {
        string p = Path.Combine(mod, f);
        if (File.Exists(p)) add(p, f);
    }
    add(exe, "demaster.exe");
    add(live, "demaster_live.dll");
    add_text(text("converter", "demaster.toml"), "demaster.toml");
    add_text(text("tools", "package", "README.txt"), "README.txt");
    add_text(text("docs", "config.md"), "config.md");
    add_text(licences, "LICENSES.txt");
}
Console.WriteLine(out_path);
return 0;

static string this_file([CallerFilePath] string path = "") => path;

// The crates linked into demaster.exe (normal dependencies, Windows target), sorted by name.
static string crate_licences(string root) {
    ProcessStartInfo psi = new("cargo", ["metadata", "--format-version", "1", "--locked",
                                         "--filter-platform", "x86_64-pc-windows-msvc"]) {
        WorkingDirectory       = root,
        RedirectStandardOutput = true,
    };
    using Process p = Process.Start(psi) ?? throw new Exception("cannot run cargo");
    string json = p.StandardOutput.ReadToEnd();
    p.WaitForExit();
    if (p.ExitCode != 0) throw new Exception($"cargo metadata failed ({p.ExitCode})");

    using JsonDocument doc = JsonDocument.Parse(json);
    JsonElement meta = doc.RootElement;
    Dictionary<string, JsonElement> packages = [];
    foreach (JsonElement pkg in meta.GetProperty("packages").EnumerateArray())
        packages[pkg.GetProperty("id").GetString()!] = pkg;
    Dictionary<string, JsonElement> nodes = [];
    foreach (JsonElement n in meta.GetProperty("resolve").GetProperty("nodes").EnumerateArray())
        nodes[n.GetProperty("id").GetString()!] = n;

    string start = packages.Values.First(pkg => pkg.GetProperty("name").GetString() == "demaster"
                                             && pkg.GetProperty("source").ValueKind == JsonValueKind.Null)
                                  .GetProperty("id").GetString()!;
    HashSet<string> seen = [start];
    Stack<string> todo = new([start]);
    while (todo.Count > 0) {
        foreach (JsonElement dep in nodes[todo.Pop()].GetProperty("deps").EnumerateArray()) {
            bool normal = dep.GetProperty("dep_kinds").EnumerateArray()
                             .Any(k => k.GetProperty("kind").ValueKind == JsonValueKind.Null);
            string id = dep.GetProperty("pkg").GetString()!;
            if (normal && seen.Add(id)) todo.Push(id);
        }
    }
    seen.Remove(start);

    static string str(JsonElement e, string name) =>
        e.TryGetProperty(name, out JsonElement v) && v.ValueKind == JsonValueKind.String ? v.GetString()! : "";
    List<JsonElement> crates = seen.Select(id => packages[id])
                                   .OrderBy(pkg => str(pkg, "name"), StringComparer.Ordinal)
                                   .ThenBy(pkg => str(pkg, "version"), StringComparer.Ordinal)
                                   .ToList();
    StringBuilder sb = new();
    sb.AppendLine("demaster.exe is built with the Rust standard library (MIT OR Apache-2.0) and these crates");
    sb.AppendLine("(name, version, licence, source):");
    sb.AppendLine();
    // The licence files each crate ships, one copy per distinct text, in first-use order.
    List<(string text, List<string> crates)> texts = [];
    foreach (JsonElement pkg in crates) {
        string name = $"{str(pkg, "name")} {str(pkg, "version")}";
        string licence = str(pkg, "license");
        if (licence == "") licence = str(pkg, "license_file") is "" ? "unknown" : "see its licence file";
        sb.AppendLine($"  {name}  {licence}  {str(pkg, "repository")}");
        string dir = Path.GetDirectoryName(str(pkg, "manifest_path"))!;
        IEnumerable<string> files = Directory.EnumerateFiles(dir)
            .Where(f => Path.GetFileName(f).ToUpperInvariant() is var n
                        && (n.StartsWith("LICENSE") || n.StartsWith("LICENCE") || n.StartsWith("COPYING")
                            || n.StartsWith("NOTICE") || n.StartsWith("UNLICENSE")))
            .Order(StringComparer.Ordinal);
        foreach (string f in files) {
            string t = File.ReadAllText(f).ReplaceLineEndings("\n").Trim();
            int i = texts.FindIndex(x => x.text == t);
            if (i < 0) texts.Add((t, [name]));
            else if (!texts[i].crates.Contains(name)) texts[i].crates.Add(name);
        }
    }
    sb.AppendLine();
    sb.AppendLine("Licence texts shipped with these crates");
    sb.AppendLine("---------------------------------------");
    foreach ((string t, List<string> users) in texts) {
        sb.AppendLine();
        sb.AppendLine(new string('=', 78));
        sb.AppendLine($"Used by: {string.Join(", ", users)}");
        sb.AppendLine(new string('-', 78));
        sb.AppendLine(t);
    }
    return sb.ToString();
}
