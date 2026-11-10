// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.FpsUnlock;

/// <summary>
/// FPS unlock (mod yuta.fps-unlock): draws the game at the monitor's refresh rate, a fixed rate or
/// uncapped while the world keeps its 30 ticks a second, draws the camera and the characters
/// between ticks, and scales game time (slow motion, fast-forward) without drawing more frames
/// (docs/mods/fps-unlock.md). The settings and their defaults are those of settings.default.toml in this
/// folder and of FrameSettings (a test checks both); the converter's list of this mod
/// (demaster/src/mods.rs) must agree (a converter test checks). Every setting is read live: the
/// mod's part of the F8 window changes them while the game runs and writes
/// them back into settings.toml.
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class FpsUnlockModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("frames.enabled",                  AppliedBy.Mod, "true"),
        new("frames.target",                   AppliedBy.Mod, "monitor"),
        new("frames.fixed_fps",                AppliedBy.Mod, "120.0"),
        new("frames.vsync",                    AppliedBy.Mod, "game"),
        new("frames.overlay",                  AppliedBy.Mod, "true"),
        new("frames.between_ticks",            AppliedBy.Mod, "redraw"),
        new("frames.effects",                  AppliedBy.Mod, "replay"),
        new("frames.speed.time_scale",         AppliedBy.Mod, "1.0"),
        new("frames.speed.fast",               AppliedBy.Mod, "10.0"),
        new("frames.speed.slow",               AppliedBy.Mod, "0.25"),
        new("frames.speed.mute_when_fast",     AppliedBy.Mod, "true"),
        new("frames.speed.booster_limits",     AppliedBy.Mod, "true"),
        new("frames.speed.scene_time",         AppliedBy.Mod, "true"),
        new("frames.interpolate.camera",       AppliedBy.Mod, "true"),
        new("frames.interpolate.characters",   AppliedBy.Mod, "true"),
        new("frames.interpolate.bones",        AppliedBy.Mod, "true"),
        new("frames.interpolate.cloth",        AppliedBy.Mod, "true"),
        new("frames.interpolate.objects",      AppliedBy.Mod, "true"),
        new("frames.interpolate.repeats",      AppliedBy.Mod, "true"),
        new("frames.cuts.camera_distance",     AppliedBy.Mod, "100.0"),
        new("frames.cuts.camera_degrees",      AppliedBy.Mod, "45.0"),
        new("frames.cuts.game_sets",           AppliedBy.Mod, "true"),
        new("frames.cuts.character_distance",  AppliedBy.Mod, "50.0"),
        new("frames.cuts.character_degrees",   AppliedBy.Mod, "120.0"),
        new("frames.hold.characters",          AppliedBy.Mod, "true"),
        new("frames.hold.field_animation",     AppliedBy.Mod, "true"),
        new("frames.hold.fades",               AppliedBy.Mod, "true"),
        new("frames.hold.cloth",               AppliedBy.Mod, "true"),
        new("frames.budget.max_real_time",     AppliedBy.Mod, "0.3"),
        new("frames.budget.max_ticks",         AppliedBy.Mod, "100"),
        new("frames.budget.max_update_ms",     AppliedBy.Mod, "40.0"),
        new("frames.budget.whole_updates",     AppliedBy.Mod, "4"),
    ];

    // The first installs every hook; the mod reads all settings live, so none is ever "off" here
    // (frames.enabled = false keeps the hooks in and passes everything through, so the F8 window
    // can turn the mod on again). The lines show each setting as the game started.
    public static readonly CodePatch[] PATCHES = [
        new("frames.enabled",                 "", "the frame timing hooks (the frame limiter, the ticks, the draw between ticks)", FpsUnlock.install),
        new("frames.target",                  "", "frame rate target", FpsUnlock.live),
        new("frames.fixed_fps",               "", "frame rate of target fixed", FpsUnlock.live),
        new("frames.vsync",                   "", "vsync", FpsUnlock.live),
        new("frames.overlay",                 "", "the fps counter in the corner", FpsUnlock.live),
        new("frames.between_ticks",           "", "frames without a tick", FpsUnlock.live),
        new("frames.effects",                 "", "effects between ticks", FpsUnlock.live),
        new("frames.speed.time_scale",        "", "game time per real second", FpsUnlock.live),
        new("frames.speed.fast",              "", "the Ctrl+F9 scale", FpsUnlock.live),
        new("frames.speed.slow",              "", "the Shift+F9 scale", FpsUnlock.live),
        new("frames.speed.mute_when_fast",    "", "sound off above x1", FpsUnlock.live),
        new("frames.speed.booster_limits",    "", "x1 where the booster won't speed up", FpsUnlock.live),
        new("frames.speed.scene_time",        "", "water, sky and other scene animations in game time", FpsUnlock.live),
        new("frames.interpolate.camera",      "", "the camera between ticks", FpsUnlock.live),
        new("frames.interpolate.characters",  "", "the characters between ticks", FpsUnlock.live),
        new("frames.interpolate.bones",       "", "the characters' bones between ticks", FpsUnlock.live),
        new("frames.interpolate.cloth",       "", "the characters' cloth between ticks", FpsUnlock.live),
        new("frames.interpolate.objects",     "", "the map's moving objects between ticks", FpsUnlock.live),
        new("frames.interpolate.repeats",     "", "the camera on repeated frames", FpsUnlock.live),
        new("frames.cuts.camera_distance",    "", "camera cut: moved more than", FpsUnlock.live),
        new("frames.cuts.camera_degrees",     "", "camera cut: turned more than", FpsUnlock.live),
        new("frames.cuts.game_sets",          "", "the game's instant camera sets are cuts", FpsUnlock.live),
        new("frames.cuts.character_distance", "", "character and object warp: moved more than", FpsUnlock.live),
        new("frames.cuts.character_degrees",  "", "character and object warp: turned more than", FpsUnlock.live),
        new("frames.hold.characters",         "", "held on redrawn frames: bones and motion events", FpsUnlock.live),
        new("frames.hold.field_animation",    "", "held on redrawn frames: field animation", FpsUnlock.live),
        new("frames.hold.fades",              "", "held on redrawn frames: fade counters", FpsUnlock.live),
        new("frames.hold.cloth",              "", "held on redrawn frames: cloth", FpsUnlock.live),
        new("frames.budget.max_real_time",    "", "most real time one frame counts (s)", FpsUnlock.live),
        new("frames.budget.max_ticks",        "", "most ticks one frame runs", FpsUnlock.live),
        new("frames.budget.max_update_ms",    "", "most update time one frame may take (ms)", FpsUnlock.live),
        new("frames.budget.whole_updates",    "", "whole game updates a frame runs before exec frames", FpsUnlock.live),
    ];

    public static readonly ModInfo INFO = new("fps-unlock", "FPS unlock", SETTINGS, PATCHES, Builds: false, Section: FpsUnlock.section);

    private bool _ui_failed;

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }

    public override void render_imgui() {
        // Fahrenheit calls this without a try/catch, so an exception would end the game.
        if (_ui_failed) return;
        try {
            FpsUnlock.render();
        }
        catch (Exception e) {
            _ui_failed = true;
            _logger.Error($"The hotkeys and the fps counter failed and are now off: {e}");
        }
    }
}
