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
/// The settings live in Mod.cs (the list with defaults, which the converter's test checks against converter/src/mods.rs
/// and docs/config.md), settings.toml and <see cref="FrameSettings"/>: these must agree.
/// </summary>
public class SettingsFileTests {
    private static string ModFile(string name) => File.ReadAllText(Path.Combine(Toml.Repo(), "mods", "fps-unlock", name));

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
    public void Settings_toml_holds_the_defaults() {
        MiniToml.Result r = MiniToml.parse(ModFile("settings.toml"));
        Assert.Empty(r.Problems);
        Assert.Equal(FrameSettings.Keys.Order(), r.Values.Keys.Order());
        Assert.Equal(new FrameSettings().ToValues().ToDictionary(kv => kv.Key, kv => kv.Value), r.Values);
    }

    [Fact]
    public void Saving_into_the_shipped_file_keeps_its_comments_and_reads_back() {
        string shipped = ModFile("settings.toml");
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
