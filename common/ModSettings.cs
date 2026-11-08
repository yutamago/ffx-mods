// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods;

/// <summary>Who applies a setting: files written by demaster.exe, code in the mod, or both.</summary>
public enum AppliedBy { Converter, Mod, Both }

/// <summary>One setting of a mod's settings.toml, with its default.</summary>
public sealed record SettingInfo(string Key, AppliedBy By, string Default);

/// <summary>
/// A mod's settings.toml as the mod sees it. Each mod lists its settings with their defaults
/// (mods/&lt;name&gt;/Mod.cs); the converter's list of the mod (converter/src/mods.rs) is the
/// reference and a converter test checks that the two agree, and that the mod's settings.toml
/// holds the defaults.
/// </summary>
public sealed class ModSettings {
    /// <summary>The file in every mod folder.</summary>
    public const string FILE = "settings.toml";

    private static readonly HashSet<string> NON_SETTINGS = ["source.iso", "source.game", "source.ffmpeg", "build.mod_dir"];

    public readonly SettingInfo[] All;
    public readonly string Path;
    public readonly bool   Found;
    /// <summary>Values as written in the file ("ps2", "true"); missing keys are absent.</summary>
    public readonly Dictionary<string, string> Values;
    /// <summary>Keys in the file that are not this mod's (a typo, or another mod's setting).</summary>
    public readonly List<string> Unknown;
    public readonly List<string> Problems;

    private ModSettings(SettingInfo[] all, string path, bool found, Dictionary<string, string> values, List<string> problems) {
        All      = all;
        Path     = path;
        Found    = found;
        Values   = values;
        Problems = problems;
        Unknown  = values.Keys.Where(k => !NON_SETTINGS.Contains(k) && all.All(s => s.Key != k)).Order().ToList();
    }

    public static ModSettings load(string path, SettingInfo[] all) {
        if (!File.Exists(path)) return new(all, path, false, [], []);
        MiniToml.Result r = MiniToml.parse(File.ReadAllText(path));
        return new(all, path, true, r.Values, r.Problems);
    }

    /// <summary>The setting's value, or its default when the file doesn't set it.</summary>
    public string get(string key) =>
        Values.TryGetValue(key, out string? v) ? v : All.First(s => s.Key == key).Default;
}
