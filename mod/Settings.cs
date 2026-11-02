// SPDX-License-Identifier: MIT

namespace Yuta.Demaster;

/// <summary>Who applies a setting: files written by demaster.exe, code in this mod, or both.</summary>
internal enum AppliedBy { Converter, Mod, Both }

internal sealed record SettingInfo(string Key, AppliedBy By, string Default);

/// <summary>
/// demaster.toml as this mod sees it. Every setting of docs/config.md is listed here with its
/// default; the converter's list (converter/src/config.rs) is the reference and a converter
/// test checks that every key in it appears in this file.
/// </summary>
internal sealed class DemasterSettings {
    public static readonly SettingInfo[] ALL = [
        new("movies",                AppliedBy.Converter, "ps2"),
        new("models.party",          AppliedBy.Converter, "ps2"),
        new("models.npc",            AppliedBy.Converter, "ps2"),
        new("models.monster",        AppliedBy.Converter, "ps2"),
        new("models.aeon",           AppliedBy.Converter, "ps2"),
        new("models.weapon",         AppliedBy.Converter, "ps2"),
        new("models.object",         AppliedBy.Converter, "ps2"),
        new("maps.field",            AppliedBy.Converter, "ps2"),
        new("maps.geometry",         AppliedBy.Converter, "ps2"),
        new("maps.backgrounds",      AppliedBy.Converter, "ps2"),
        new("maps.battle",           AppliedBy.Converter, "ps2"),
        new("effects.textures",      AppliedBy.Converter, "ps2"),
        new("render.glow",           AppliedBy.Mod,       "ps2"),
        new("render.lighting",       AppliedBy.Mod,       "ps2"),
        new("render.shadows",        AppliedBy.Mod,       "ps2"),
        new("render.aspect",         AppliedBy.Mod,       "ps2"),
        new("render.texture_filter", AppliedBy.Mod,       "game"),
        new("ui.menus",              AppliedBy.Both,      "ps2"),
        new("ui.portraits",          AppliedBy.Converter, "ps2"),
        new("ui.font",               AppliedBy.Converter, "ps2"),
        new("ui.title",              AppliedBy.Converter, "ps2"),
        new("ui.loading",            AppliedBy.Mod,       "ps2"),
        new("ui.battle_subtitles",   AppliedBy.Mod,       "ps2"),
        new("ui.battle_menu_delay",  AppliedBy.Mod,       "ps2"),
        new("ui.button_glyphs",      AppliedBy.Converter, "ps2"),
        new("audio.soundtrack",      AppliedBy.Mod,       "ps2"),
        new("audio.music_engine",    AppliedBy.Converter, "ps2"),
        new("fixes.npc_mouths",      AppliedBy.Mod,       "true"),
        new("fixes.farplane_video",  AppliedBy.Mod,       "true"),
        new("fixes.lulu_fury",       AppliedBy.Mod,       "true"),
    ];

    private static readonly HashSet<string> NON_SETTINGS = ["source.iso", "source.game", "source.ffmpeg", "build.mod_dir"];

    public readonly string Path;
    public readonly bool   Found;
    /// <summary>Values as written in the file ("ps2", "true"); missing keys are absent.</summary>
    public readonly Dictionary<string, string> Values;
    public readonly List<string> Unknown;
    public readonly List<string> Problems;

    private DemasterSettings(string path, bool found, Dictionary<string, string> values, List<string> problems) {
        Path     = path;
        Found    = found;
        Values   = values;
        Problems = problems;
        Unknown  = values.Keys.Where(k => !NON_SETTINGS.Contains(k) && ALL.All(s => s.Key != k)).Order().ToList();
    }

    public static DemasterSettings load(string path) {
        if (!File.Exists(path)) return new(path, false, [], []);
        MiniToml.Result r = MiniToml.parse(File.ReadAllText(path));
        return new(path, true, r.Values, r.Problems);
    }

    /// <summary>The setting's value, or its default when the file doesn't set it.</summary>
    public string get(string key) =>
        Values.TryGetValue(key, out string? v) ? v : ALL.First(s => s.Key == key).Default;
}
