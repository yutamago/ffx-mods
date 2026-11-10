// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods;

/// <summary>
/// A change to the game's code, switched by one setting of the mod's settings.toml
/// (AppliedBy.Mod or Both). Settings are read once at startup, like the converter's files.
/// </summary>
/// <param name="Key">The setting, as in settings.toml ("render.lighting").</param>
/// <param name="Off">The value that keeps the game as it is ("hd", "game", "false").</param>
/// <param name="What">What the patch does, for the log and the window.</param>
/// <param name="Apply">
/// Installs the patch for the setting's value (hooks are queued from init, like any
/// FhModule's); returns whether it worked. Null until the patch exists.
/// </param>
/// <param name="Only">When set, the only value that turns the patch on (other values are off).</param>
public sealed record CodePatch(string Key, string Off, string What, Func<ModHost, string, bool>? Apply, string? Only = null);

public enum PatchState { Off, On, NotImplemented, Failed }

public sealed record PatchResult(CodePatch Patch, string Value, PatchState State);

public static class CodePatches {
    /// <summary>Applies every patch its setting asks for. Returns each patch's state.</summary>
    public static List<PatchResult> apply(ModHost host, IEnumerable<CodePatch> patches) {
        FhLogger log = host.log;
        List<PatchResult> result = [];
        foreach (CodePatch p in patches) {
            string value = host.Settings.get(p.Key);
            PatchState state;
            if (value == p.Off || (p.Only is not null && value != p.Only)) state = PatchState.Off;
            else if (p.Apply is null) state = PatchState.NotImplemented;
            else {
                try { state = p.Apply(host, value) ? PatchState.On : PatchState.Failed; }
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
            result.Add(new(p, value, state));
        }
        return result;
    }
}
