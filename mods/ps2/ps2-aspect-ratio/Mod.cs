// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods.AspectRatio;

/// <summary>
/// PS2 aspect ratio (mod yuta.ps2-aspect-ratio): The PS2 picture: 4:3 with black bars and the PS2 projection.
/// The settings and their defaults are those of settings.default.toml in this folder; the converter's
/// list of this mod (demaster/src/mods.rs) must agree (a converter test checks).
/// </summary>
[FhLoad(FhGameId.FFX)]
public sealed class AspectRatioModule : FhModule {
    public static readonly SettingInfo[] SETTINGS = [
        new("render.aspect", AppliedBy.Mod,       "ps2"),
    ];

    public static readonly CodePatch[] PATCHES = [
        new("render.aspect", "hd", "4:3 with the PS2 projection", AspectPatches.aspect),
    ];

    public static readonly ModInfo INFO = new("ps2-aspect-ratio", "PS2 aspect ratio", SETTINGS, PATCHES, Builds: false);

    private bool _failed;

    public override bool init(FhModContext mod_context, FileStream global_state_file) {
        ModHost.start(this, _logger, mod_context, INFO);
        return true;
    }

    /// <summary>Called once a frame: the movie check of render.aspect (AspectPatches.per_frame).</summary>
    public override void render_imgui() {
        // Fahrenheit calls this without a try/catch, so an exception would end the game.
        if (_failed) return;
        try { AspectPatches.per_frame(); }
        catch (Exception e) {
            _failed = true;
            _logger.Error($"render.aspect: the movie check failed and is now off: {e}");
        }
    }
}
