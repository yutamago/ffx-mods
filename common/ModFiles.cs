// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods;

/// <summary>
/// The files the installed mods put in place of the game's: every mod's efl/x, in load order
/// (Fahrenheit's file loader indexes them the same way, a later mod's file wins). The PS2 files
/// are those whose winning mod was built by demaster.exe: a mod of this family (any mod
/// folder with build/files.json, also the old all-in-one FFX Demaster).
/// </summary>
public static class ModFiles {
    /// <summary>Mod ids of the family (the shared library and the mods that depend on it, or a folder demaster.exe built into).</summary>
    public static bool is_family(FhModContext m) =>
        m.Manifest.Id == "yuta.ffx-mods" || m.Manifest.Dependencies.Contains("yuta.ffx-mods") || built_by_converter(m.Paths.ModDir.FullName);

    /// <summary>Whether demaster.exe built files into this mod folder.</summary>
    public static bool built_by_converter(string mod_dir) => File.Exists(Path.Combine(mod_dir, "build", "files.json"));

    private static string norm(string path) => path.Replace('\\', '/').ToLowerInvariant();

    /// <summary>
    /// The PS2 files of every installed mod (paths under efl/x, lower case with '/') with the
    /// extension <paramref name="ext"/> (".phyre"), and per mod how many of them it serves.
    /// </summary>
    public static (HashSet<string> Files, List<(string Mod, int Count)> PerMod) ps2_files(string ext) {
        Dictionary<string, (string Mod, bool Ps2)> winner = [];
        List<string> order = [];
        foreach (FhModContext m in FhApi.Mods.get_mods()) {
            string root = Path.Combine(m.Paths.EflDir.FullName, "x");
            if (!Directory.Exists(root)) continue;
            bool ps2 = built_by_converter(m.Paths.ModDir.FullName);
            order.Add(m.Manifest.Id);
            foreach (string f in Directory.EnumerateFiles(root, "*" + ext, SearchOption.AllDirectories))
                winner[norm(Path.GetRelativePath(root, f))] = (m.Manifest.Id, ps2);
        }
        HashSet<string> files = [.. winner.Where(p => p.Value.Ps2).Select(p => p.Key)];
        List<(string, int)> per = [.. order.Select(id => (id, winner.Values.Count(w => w.Ps2 && w.Mod == id))).Where(x => x.Item2 > 0)];
        return (files, per);
    }
}
