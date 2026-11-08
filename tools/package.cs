// SPDX-License-Identifier: MIT
//
// Zips the release: one zip per mod, yuta.<name>-<version>.zip, ready to unpack into the
// Fahrenheit folder. Each holds
//   mods/yuta.ffx-mods/   the shared library every mod needs (its DLL and manifest)
//   mods/yuta.<name>/     the mod: DLL and manifest, settings.toml, README.txt, LICENSES.txt,
//                         install.cmd + install.ps1; for mods with PS2 files demaster.exe
//                         (the converter), for ps2-music-engine also demaster_live.dll
// No game data: install.cmd builds that with demaster.exe from the player's own disc.
//
// LICENSES.txt is tools/package/LICENSES.txt, then (for mods with demaster.exe) every crate
// compiled into it with its licence and the licence files those crates ship (from `cargo
// metadata`, so cargo must be able to fetch ffx-rust), then THIRD-PARTY-NOTICES.
//
// usage: dotnet run tools/package.cs -- <out dir> <version> [demaster.exe]
//   after: dotnet build -c Release ffx-mods.slnx
//   and:   tools/build-windows.sh and tools/build-live.sh (Linux/WSL), or the cargo builds
//          for x86_64-pc-windows-msvc and i686-pc-windows-msvc (Windows)

using System.Diagnostics;
using System.IO.Compression;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;

if (args.Length is < 2 or > 3) {
    Console.Error.WriteLine("usage: dotnet run tools/package.cs -- <out dir> <version> [demaster.exe]");
    return 1;
}

const string COMMON = "yuta.ffx-mods";
// The mods, and which of them build PS2 files (ship the converter and install it with install.cmd).
(string Name, bool Builds, bool Live)[] MODS = [
    ("ps2-music-engine", true,  true),
    ("ps2-characters",   true,  false),
    ("ps2-lighting",     true,  false),
    ("ps2-interface",    true,  false),
    ("ps2-aspect-ratio", false, false),
    ("ps2-game-feel",    false, false),
    ("bug-fixes",        false, false),
    ("modern-minimap",    false, false),
];

string root    = Path.GetFullPath(Path.Combine(Path.GetDirectoryName(this_file())!, ".."));
string deploy  = Path.Combine(root, "artifacts", "deploy", "rel", "mods");
string out_dir = Path.GetFullPath(args[0]);
string version = args[1].TrimStart('v');
string exe = args.Length > 2
    ? Path.GetFullPath(args[2])
    : Path.Combine(root, "target", "x86_64-pc-windows-msvc", "release", "demaster.exe");
string live = Path.Combine(root, "target", "i686-pc-windows-msvc", "release", "demaster_live.dll");

foreach (string id in MODS.Select(m => $"yuta.{m.Name}").Prepend(COMMON)) {
    if (!File.Exists(Path.Combine(deploy, id, $"{id}.dll"))) {
        Console.Error.WriteLine($"{id} is not built: dotnet build -c Release ffx-mods.slnx");
        return 1;
    }
}
if (!File.Exists(exe)) {
    Console.Error.WriteLine($"{exe} is missing: build the converter for Windows first (tools/build-windows.sh)");
    return 1;
}
if (!File.Exists(live)) {
    Console.Error.WriteLine($"{live} is missing: build it first (tools/build-live.sh)");
    return 1;
}

string header, notices, crates;
try {
    header  = File.ReadAllText(Path.Combine(root, "tools", "package", "LICENSES.txt"));
    notices = File.ReadAllText(Path.Combine(root, "THIRD-PARTY-NOTICES"));
    crates  = crate_licences(root);
} catch (Exception e) {
    Console.Error.WriteLine($"LICENSES.txt: {e.Message}");
    return 1;
}

Directory.CreateDirectory(out_dir);
foreach (var m in MODS) {
    string id = $"yuta.{m.Name}";
    string zip_path = Path.Combine(out_dir, $"{id}-{version}.zip");
    File.Delete(zip_path);
    using (ZipArchive zip = ZipFile.Open(zip_path, ZipArchiveMode.Create)) {
        // Entry names use '/' on every OS, so the zip unpacks the same on Windows.
        void add(string file, string name) => zip.CreateEntryFromFile(file, name);
        // Text files get CRLF, for Notepad.
        void add_text(string text, string name) {
            ZipArchiveEntry entry = zip.CreateEntry(name);
            using StreamWriter w = new(entry.Open(), new UTF8Encoding(false));
            w.Write(text.ReplaceLineEndings("\r\n"));
        }
        // What a mod build may contain; anything else (efl/, build/ from a local test) stays out.
        void add_dll(string mod_id) {
            foreach (string ext in new[] { ".dll", ".deps.json", ".runtimeconfig.json", ".manifest.json", ".pdb" }) {
                string p = Path.Combine(deploy, mod_id, mod_id + ext);
                if (File.Exists(p)) add(p, $"mods/{mod_id}/{mod_id}{ext}");
            }
        }
        string dir = $"mods/{id}";
        string src = Path.Combine(root, "mods", m.Name);
        add_dll(COMMON);
        add_dll(id);
        add_text(File.ReadAllText(Path.Combine(src, "settings.toml")), $"{dir}/settings.toml");
        add_text(File.ReadAllText(Path.Combine(src, "README.txt")), $"{dir}/README.txt");
        add_text(File.ReadAllText(Path.Combine(root, "tools", "install", "install.cmd")), $"{dir}/install.cmd");
        add_text(File.ReadAllText(Path.Combine(root, "tools", "install", "install.ps1")), $"{dir}/install.ps1");
        if (m.Builds) add(exe, $"{dir}/demaster.exe");
        if (m.Live) add(live, $"{dir}/demaster_live.dll");
        add_text(header + (m.Builds ? crates : "(none)\n") + "\n\n" + notices, $"{dir}/LICENSES.txt");
    }
    Console.WriteLine(zip_path);
}
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

    // demaster.exe and demaster_live.dll (which links the demaster crate and anyhow, a subset).
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
    sb.AppendLine("demaster.exe (and demaster_live.dll) are built with the Rust standard library (MIT OR");
    sb.AppendLine("Apache-2.0) and these crates (name, version, licence, source):");
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
