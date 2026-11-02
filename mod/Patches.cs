// SPDX-License-Identifier: MIT

namespace Yuta.Demaster;

/// <summary>
/// A change to the game's code, switched by one setting of demaster.toml (AppliedBy.Mod).
/// Settings are read once at startup, like the converter's files.
/// </summary>
/// <param name="Key">The setting, as in demaster.toml ("render.glow").</param>
/// <param name="Off">The value that keeps the game as it is ("hd", "game", "false").</param>
/// <param name="What">What the patch does, for the log and the window.</param>
/// <param name="Apply">
/// Installs the patch for the setting's value (hooks are queued from init, like any
/// FhModule's); returns whether it worked. Null until the patch exists.
/// </param>
internal sealed record CodePatch(string Key, string Off, string What, Func<DemasterModule, string, bool>? Apply);

internal enum PatchState { Off, On, NotImplemented, Failed }

/// <summary>Where the code patches go: add the Apply function of a patch here when it's written.</summary>
internal static class Patches {
    public static readonly CodePatch[] ALL = [
        new("render.glow",           "hd",    "glow/blow-out pass off",                  null),
        new("render.lighting",       "hd",    "PS2-like character lighting",             null),
        new("render.shadows",        "hd",    "blob shadows only",                       null),
        new("render.aspect",         "hd",    "4:3 with the PS2 projection",             null),
        new("render.texture_filter", "game",  "nearest-neighbour texture filtering",     null),
        new("ui.loading",            "hd",    "no loading screen, no splash videos",     null),
        new("ui.battle_subtitles",   "hd",    "battle dialogue subtitles",               null),
        new("ui.battle_menu_delay",  "hd",    "battle menu without input delays",        null),
        new("audio.soundtrack",      "hd",    "original soundtrack by default",          null),
        new("fixes.npc_mouths",      "false", "NPC mouths closed in cutscenes",          null),
        new("fixes.farplane_video",  "false", "Farplane background video plays",         null),
        new("fixes.lulu_fury",       "false", "Lulu's Fury spins register",              null),
    ];

    /// <summary>Applies every patch its setting asks for. Returns each patch's state.</summary>
    public static List<(CodePatch Patch, string Value, PatchState State)> apply(DemasterModule module, DemasterSettings settings, FhLogger log) {
        List<(CodePatch, string, PatchState)> result = [];
        foreach (CodePatch p in ALL) {
            string value = settings.get(p.Key);
            PatchState state;
            if (value == p.Off) state = PatchState.Off;
            else if (p.Apply is null) state = PatchState.NotImplemented;
            else {
                try { state = p.Apply(module, value) ? PatchState.On : PatchState.Failed; }
                catch (Exception e) {
                    log.Error($"{p.Key}: {e}");
                    state = PatchState.Failed;
                }
            }
            switch (state) {
                case PatchState.On:             log.Info($"{p.Key} = {value}: {p.What}"); break;
                case PatchState.Failed:         log.Warning($"{p.Key} = {value}: could not apply ({p.What}); the game stays as it is"); break;
                case PatchState.NotImplemented: log.Info($"{p.Key} = {value}: not implemented yet ({p.What})"); break;
            }
            result.Add((p, value, state));
        }
        return result;
    }
}
