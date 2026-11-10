// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods;

/// <summary>
/// Who applies a setting: files written by demaster.exe, code in the mod, or both. Not read by this
/// library; kept for mods that list their settings by hand (ffx-remix's). The family's mods take
/// theirs from settings.default.toml (<see cref="SettingInfo.defaults_of"/>), and the converter
/// knows which settings its jobs read (demaster/src/mods.rs).
/// </summary>
public enum AppliedBy { Converter, Mod, Both }

/// <summary>One setting of a mod's settings.toml, with its default.</summary>
public sealed record SettingInfo(string Key, AppliedBy By, string Default) {
    /// <summary>
    /// The settings of a mod of the family: the keys and values of its settings.default.toml, which
    /// mods/Directory.Build.targets embeds into its DLL. That file is the one list of the mod's
    /// settings and their defaults; the converter reads the same file (demaster/src/mods.rs).
    /// None for a mod without the file.
    /// </summary>
    public static SettingInfo[] defaults_of(Type module) {
        using Stream? s = module.Assembly.GetManifestResourceStream(ModSettings.DEFAULTS);
        if (s is null) return [];
        using StreamReader reader = new(s);
        return from_defaults(reader.ReadToEnd());
    }

    /// <summary>The settings in a settings.default.toml's text (not [source] and [build]: where the disc and game are).</summary>
    public static SettingInfo[] from_defaults(string toml) =>
        [.. MiniToml.parse(toml).Values
            .Where(kv => !kv.Key.StartsWith("source.", StringComparison.Ordinal) && !kv.Key.StartsWith("build.", StringComparison.Ordinal))
            .Select(kv => new SettingInfo(kv.Key, AppliedBy.Mod, kv.Value))];
}

/// <summary>
/// A mod's settings.toml as the mod sees it. The mod's settings and their defaults are those of its
/// settings.default.toml (<see cref="SettingInfo.defaults_of"/>; the converter reads the same file).
/// The zips ship only settings.default.toml; install.cmd
/// makes settings.toml from it when there is none, so unpacking a newer zip keeps the player's
/// settings. A key the file leaves out (a setting newer than the file) has its default.
/// </summary>
public sealed class ModSettings {
    /// <summary>The player's settings in every mod folder (made by install.cmd, never in a zip).</summary>
    public const string FILE = "settings.toml";
    /// <summary>The defaults, shipped in the zip next to it (every update replaces this one).</summary>
    public const string DEFAULTS = "settings.default.toml";

    private static readonly HashSet<string> NON_SETTINGS = ["source.iso", "source.game", "source.ffmpeg", "build.mod_dir"];

    public readonly SettingInfo[] All;
    public readonly string Path;
    public readonly bool   Found;
    /// <summary>
    /// The file is missing although the mod has settings (a mod without settings needs none): install.cmd
    /// has not run yet. The defaults apply; not a problem in itself.
    /// </summary>
    public bool Missing => !Found && All.Length > 0;
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

    /// <summary>
    /// Writes values into the settings file at <paramref name="path"/> (the file is created when
    /// missing): each key's value replaced on its line, the line's comment and every other line kept;
    /// a key the file lacks is added at the end of its table. A missing file starts as a copy of
    /// <see cref="DEFAULTS"/> next to it (with its comments), as install.cmd makes it.
    /// <paramref name="values"/> are TOML literals ("\"monitor\"", "true", "120.0"). For a mod that
    /// changes its settings while the game runs; this object keeps the values it read at startup.
    /// </summary>
    /// <exception cref="IOException">The file cannot be read or written.</exception>
    public static void write(string path, IEnumerable<KeyValuePair<string, string>> values) {
        string defaults = System.IO.Path.Combine(System.IO.Path.GetDirectoryName(path) ?? "", DEFAULTS);
        bool found = File.Exists(path);
        string text = found ? File.ReadAllText(path) : File.Exists(defaults) ? File.ReadAllText(defaults) : "";
        string next = text;
        foreach ((string key, string literal) in values) next = MiniToml.set(next, key, literal);
        if (next == text && (found || text == "")) return;
        string temp = path + ".tmp";
        File.WriteAllText(temp, next);
        File.Move(temp, path, overwrite: true);
    }

    /// <summary>The setting's value, or its default when the file doesn't set it.</summary>
    public string get(string key) =>
        Values.TryGetValue(key, out string? v) ? v : All.First(s => s.Key == key).Default;
}
