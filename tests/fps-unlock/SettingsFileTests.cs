// SPDX-License-Identifier: MIT

using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;

using Xunit;

namespace Yuta.FfxMods.FpsUnlock.Tests;

/// <summary>settings.toml text to <see cref="FrameSettings"/> and back, through the shared library's reader and writer.</summary>
internal static class Toml {
    public static FrameSettings Settings(string text, List<string>? problems = null) {
        MiniToml.Result r = MiniToml.parse(text);
        Assert.Empty(r.Problems);
        return FrameSettings.FromValues(k => r.Values.TryGetValue(k, out string? v) ? v : null, problems);
    }

    /// <summary>Every setting of <paramref name="s"/> written into an empty file.</summary>
    public static string Text(FrameSettings s) {
        string text = "";
        foreach ((string key, string value) in s.ToValues()) text = MiniToml.set(text, key, FrameSettings.TomlLiteral(key, value));
        return text;
    }

    public static string Repo([CallerFilePath] string here = "") => Path.GetFullPath(Path.Combine(Path.GetDirectoryName(here)!, "..", ".."));
}

/// <summary>
/// The settings live in Mod.cs (the list with defaults, which the converter's test checks against demaster/src/mods.rs
/// and docs/config.md), settings.toml and <see cref="FrameSettings"/>: these must agree.
/// </summary>
public class SettingsFileTests {
    private static string ModFile(string name) => File.ReadAllText(Path.Combine(Toml.Repo(), "mods", "enhancements", "fps-unlock", name));

    [Fact]
    public void Mod_cs_lists_every_setting_with_its_default_in_order() {
        var listed = Regex.Matches(ModFile("Mod.cs"), @"new\(""([a-z_.]+)"",\s*AppliedBy\.Mod,\s*""([^""]*)""\)")
            .Select(m => new KeyValuePair<string, string>(m.Groups[1].Value, m.Groups[2].Value))
            .ToList();
        Assert.Equal(new FrameSettings().ToValues(), listed);
        Assert.Equal(FrameSettings.Keys, listed.Select(kv => kv.Key));
    }

    [Fact]
    public void Mod_cs_has_a_code_patch_for_every_setting() {
        string cs = ModFile("Mod.cs");
        foreach (string key in FrameSettings.Keys) {
            Assert.Matches(new Regex($@"new\(""{Regex.Escape(key)}"",\s*"""",\s*""[^""]+"",\s*FpsUnlock\.(install|live)\)"), cs);
        }
    }

    [Fact]
    public void Settings_default_toml_holds_the_defaults() {
        MiniToml.Result r = MiniToml.parse(ModFile("settings.default.toml"));
        Assert.Empty(r.Problems);
        Assert.Equal(FrameSettings.Keys.Order(), r.Values.Keys.Order());
        Assert.Equal(new FrameSettings().ToValues().ToDictionary(kv => kv.Key, kv => kv.Value), r.Values);
    }

    [Fact]
    public void Saving_into_the_shipped_file_keeps_its_comments_and_reads_back() {
        string shipped = ModFile("settings.default.toml");
        FrameSettings changed = new() { Target = FpsTarget.Unlimited, TimeScale = 18.42, Vsync = VsyncMode.Off, MaxTicksPerFrame = 50, FrameBudgetSeconds = 0.06 };
        string text = shipped;
        Dictionary<string, string> defaults = new FrameSettings().ToValues().ToDictionary(kv => kv.Key, kv => kv.Value);
        foreach ((string key, string value) in changed.ToValues()) {
            if (defaults[key] != value) text = MiniToml.set(text, key, FrameSettings.TomlLiteral(key, value));
        }
        Assert.Equal(changed, Toml.Settings(text));
        string[] before = shipped.Split('\n'), after = text.Split('\n');
        Assert.Equal(before.Length, after.Length);
        Assert.Equal(before.Where(l => l.StartsWith('#')), after.Where(l => l.StartsWith('#')));
        Assert.Equal(5, before.Zip(after).Count(p => p.First != p.Second));
        Assert.Contains("target = \"unlimited\"\n", text);
        Assert.Contains("time_scale = 18.42\n", text);
        Assert.Contains("max_update_ms = 60.0\n", text);
    }
}

/// <summary>
/// The zips ship settings.default.toml only; install.cmd makes settings.toml from it. A mod that saves its settings
/// in the game (FPS unlock) before install.cmd made the file starts from the defaults, with their comments.
/// </summary>
public class SettingsWriteTests {
    [Fact]
    public void A_missing_settings_toml_starts_as_the_defaults() {
        string dir = Path.Combine(Path.GetTempPath(), $"ffxmods-settings-{Environment.ProcessId}-{Guid.NewGuid():N}");
        Directory.CreateDirectory(dir);
        try {
            string shipped = File.ReadAllText(Path.Combine(Toml.Repo(), "mods", "enhancements", "fps-unlock", ModSettings.DEFAULTS));
            File.WriteAllText(Path.Combine(dir, ModSettings.DEFAULTS), shipped);
            string path = Path.Combine(dir, ModSettings.FILE);
            SettingInfo[] all = [.. new FrameSettings().ToValues().Select(kv => new SettingInfo(kv.Key, AppliedBy.Mod, kv.Value))];

            ModSettings missing = ModSettings.load(path, all);
            Assert.True(missing.Missing);
            Assert.Equal("monitor", missing.get("frames.target"));

            ModSettings.write(path, [new("frames.target", "\"unlimited\"")]);
            string text = File.ReadAllText(path);
            Assert.Equal(MiniToml.set(shipped, "frames.target", "\"unlimited\""), text);
            ModSettings loaded = ModSettings.load(path, all);
            Assert.False(loaded.Missing);
            Assert.Empty(loaded.Problems);
            Assert.Equal("unlimited", loaded.get("frames.target"));
            Assert.Equal(File.ReadAllText(Path.Combine(dir, ModSettings.DEFAULTS)), shipped);
        }
        finally { Directory.Delete(dir, true); }
    }

    [Fact]
    public void A_key_the_file_lacks_has_its_default() {
        string dir = Path.Combine(Path.GetTempPath(), $"ffxmods-settings-{Environment.ProcessId}-{Guid.NewGuid():N}");
        Directory.CreateDirectory(dir);
        try {
            string path = Path.Combine(dir, ModSettings.FILE);
            File.WriteAllText(path, "[frames]\ntarget = \"game\"\n");
            SettingInfo[] all = [new("frames.target", AppliedBy.Mod, "monitor"), new("frames.overlay", AppliedBy.Mod, "true")];
            ModSettings s = ModSettings.load(path, all);
            Assert.Equal("game", s.get("frames.target"));
            Assert.Equal("true", s.get("frames.overlay"));
        }
        finally { Directory.Delete(dir, true); }
    }
}

public class TomlWriterTests {
    [Fact]
    public void Replaces_the_value_and_keeps_the_comment_and_spacing() {
        string text = "# top\n[frames]\nenabled = true   # on\n  target =  \"game\"\n";
        Assert.Equal("# top\n[frames]\nenabled = false   # on\n  target =  \"game\"\n", MiniToml.set(text, "frames.enabled", "false"));
        Assert.Equal("# top\n[frames]\nenabled = true   # on\n  target =  \"fixed\"\n", MiniToml.set(text, "frames.target", "\"fixed\""));
    }

    [Fact]
    public void Adds_a_missing_key_at_the_end_of_its_table_and_a_missing_table_at_the_end() {
        string text = "[frames]\nenabled = true\n\n[frames.speed]\nfast = 10.0\n";
        Assert.Equal("[frames]\nenabled = true\nvsync = \"off\"\n\n[frames.speed]\nfast = 10.0\n", MiniToml.set(text, "frames.vsync", "\"off\""));
        Assert.Equal("[frames]\nenabled = true\n\n[frames.speed]\nfast = 10.0\n\n[frames.hold]\ncloth = false\n", MiniToml.set(text, "frames.hold.cloth", "false"));
        Assert.Equal("[frames.speed]\nslow = 0.5\n", MiniToml.set("", "frames.speed.slow", "0.5"));
    }

    [Fact]
    public void Finds_dotted_keys_and_keeps_crlf() {
        Assert.Equal("[frames]\r\nspeed.time_scale = 2.0\r\n", MiniToml.set("[frames]\r\nspeed.time_scale = 1.0\r\n", "frames.speed.time_scale", "2.0"));
        Assert.Equal("a = 1\n[t]\nb = 2\n", MiniToml.set("[t]\nb = 2\n", "a", "1"));
    }

    [Fact]
    public void A_hash_inside_a_string_is_not_a_comment() {
        Assert.Equal("[t]\nk = \"x\" # c\n", MiniToml.set("[t]\nk = \"a#b\" # c\n", "t.k", "\"x\""));
    }
}

/// <summary>
///     The window's sliders give floats: written back as doubles they were "time_scale = 0.05000000074505806" (2026-11-08).
///     A value is written in the shortest text that reads back as itself.
/// </summary>
public class FloatTextTests {
    [Theory]
    [InlineData(0.05f, "0.05")]
    [InlineData(0.25f, "0.25")]
    [InlineData(18.42f, "18.42")]
    [InlineData(120f, "120.0")]
    [InlineData(37.3f, "37.3")]
    [InlineData(100f, "100.0")]
    public void A_sliders_float_is_written_as_its_shortest_text(float slider, string text) {
        Assert.Equal(text, FrameSettings.Float(slider));
        Assert.Equal(text, FrameSettings.Float(FrameSettings.FromFloat(slider)));
    }

    [Theory]
    [InlineData(0.1, "0.1")]
    [InlineData(18.42, "18.42")]
    [InlineData(0.3, "0.3")]
    [InlineData(1e-7, "1E-07")]
    [InlineData(1.0000000000000002, "1.0000000000000002")]
    public void A_double_keeps_its_own_shortest_text(double value, string text) {
        Assert.Equal(text, FrameSettings.Float(value));
        Assert.Equal(value, double.Parse(text, System.Globalization.CultureInfo.InvariantCulture));
    }

    [Fact]
    public void The_slider_values_are_the_doubles_they_show() {
        Assert.Equal(0.05, FrameSettings.FromFloat(0.05f));
        Assert.Equal(37.3, FrameSettings.FromFloat(37.3f));
        Assert.Equal(0.0373, FrameSettings.FromFloat(37.3f) / 1000.0);
    }

    [Fact]
    public void Time_scale_from_the_slider_is_saved_as_0_05_and_reads_back() {
        // What the window did: TimeScale = scale (a float), then save.
        FrameSettings widened = new() { TimeScale = 0.05f, FixedFps = 143.7f, SlowScale = 0.35f };
        string text = Toml.Text(widened);
        Assert.Contains("time_scale = 0.05\n", text);
        Assert.Contains("fixed_fps = 143.7\n", text);
        Assert.Contains("slow = 0.35\n", text);
        FrameSettings read = Toml.Settings(text);
        Assert.Equal(0.05, read.TimeScale);
        Assert.Equal(143.7, read.FixedFps);
        // Saving what was read gives the same text: nothing changes on the next save.
        Assert.Equal(text, Toml.Text(read));
    }

    [Fact]
    public void The_budget_from_its_slider_is_saved_in_milliseconds_as_shown() {
        FrameSettings s = new() { FrameBudgetSeconds = FrameSettings.FromFloat(37.3f) / 1000.0 };
        Assert.Contains("max_update_ms = 37.3\n", Toml.Text(s));
    }
}
