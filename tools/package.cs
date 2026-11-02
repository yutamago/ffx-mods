// SPDX-License-Identifier: MIT
//
// Zips a release: mods/yuta.demaster/ with the mod DLL and manifest, demaster.exe, the default
// demaster.toml and the docs, ready to unpack into the Fahrenheit folder. No game data: the
// player builds that with demaster.exe from their own disc.
//
// usage: dotnet run tools/package.cs -- <out.zip> [demaster.exe]
//   after: dotnet build -c Release ffx-demaster.slnx
//   and:   tools/build-windows.sh (Linux/WSL) or
//          cargo build --release -p demaster --target x86_64-pc-windows-msvc (Windows)

using System.IO.Compression;
using System.Runtime.CompilerServices;

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
    foreach (string f in mod_files) {
        string p = Path.Combine(mod, f);
        if (File.Exists(p)) add(p, f);
    }
    add(exe, "demaster.exe");
    add(Path.Combine(root, "converter", "demaster.toml"), "demaster.toml");
    add(Path.Combine(root, "README.md"), "README.md");
    add(Path.Combine(root, "docs", "config.md"), "config.md");
    add(Path.Combine(root, "THIRD-PARTY-NOTICES"), "THIRD-PARTY-NOTICES");
}
Console.WriteLine(out_path);
return 0;

static string this_file([CallerFilePath] string path = "") => path;
