// SPDX-License-Identifier: MIT

using System.Reflection;

namespace Yuta.FfxMods;

/// <summary>What a mod of the family is: its name, its settings and its code patches.</summary>
/// <param name="Name">The mod's name in ffx-mods (mods/&lt;Name&gt;), "ps2-characters"; its Fahrenheit id is yuta.&lt;Name&gt;.</param>
/// <param name="Title">For the window and the log, "PS2 characters".</param>
/// <param name="Settings">Every setting of its settings.toml, with the default.</param>
/// <param name="Patches">Its code patches, one per setting applied by the mod.</param>
/// <param name="Builds">Whether demaster.exe builds files for it (it has converter jobs and an install script).</param>
/// <param name="Section">
/// Optional: draws the mod's own part of its F8 section (live settings, what it does now), after
/// its build status and before its code patches. Render thread; an exception turns it off.
/// </param>
public sealed record ModInfo(string Name, string Title, SettingInfo[] Settings, CodePatch[] Patches, bool Builds, Action<ModHost>? Section = null);

/// <summary>
/// The shared start of every mod of the family: reads the mod's settings.toml, checks its build
/// (build/stamp.json, written by demaster.exe) when it has one, applies its code patches and
/// puts it in the F8 window (<see cref="FamilyWindow"/>). A mod's FhModule calls
/// <see cref="start"/> from its init; the code patches get the host, which converts to the
/// FhModule for hooks.
/// </summary>
public sealed class ModHost {
    public readonly ModInfo  Info;
    public readonly FhModule Module;
    public readonly FhLogger log;
    /// <summary>The mod folder (settings.toml, efl, build, music, ...).</summary>
    public readonly string   mod_dir;
    public readonly string   Version;
    public readonly ModSettings Settings;
    public BuildStatus? Status { get; private set; }
    public List<PatchResult> Patches { get; private set; } = [];

    public static implicit operator FhModule(ModHost h) => h.Module;

    private ModHost(ModInfo info, FhModule module, FhLogger log, string mod_dir) {
        Info         = info;
        Module       = module;
        this.log     = log;
        this.mod_dir = mod_dir;
        Version      = module.GetType().Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion ?? "?";
        Settings     = ModSettings.load(Path.Combine(mod_dir, ModSettings.FILE), info.Settings);
    }

    /// <summary>The command that builds this mod's files again (shown in the window and the log).</summary>
    public string rebuild_command => $"\"{Path.Combine(mod_dir, "install.cmd")}\"";

    public static ModHost start(FhModule module, FhLogger log, FhModContext ctx, ModInfo info) {
        ModHost h = new(info, module, log, ctx.Paths.ModDir.FullName);
        log.Info($"{info.Title} {h.Version}, mod folder {h.mod_dir}");
        if (h.Settings.Missing) log.Warning($"{h.Settings.Path} is missing; using the defaults.");
        foreach (string p in h.Settings.Problems) log.Warning($"{ModSettings.FILE}: {p}");
        foreach (string k in h.Settings.Unknown)  log.Warning($"{ModSettings.FILE}: unknown setting {k}, ignored");

        if (info.Builds) {
            BuildStatus s = BuildStatus.check(h.mod_dir, h.Settings, h.Version);
            h.Status = s;
            if (s.State == BuildState.Ok) log.Info(s.Summary);
            else {
                log.Warning($"{s.Summary} To build: run {h.rebuild_command}, then restart the game.");
                foreach (var c in s.Changed) log.Warning($"  {c.Key}: built {c.Built}, now {c.Now}");
            }
            foreach (string j in s.Jobs) log.Info($"  {j}");
        }
        h.Patches = CodePatches.apply(h, info.Patches);
        FamilyWindow.register(h);
        return h;
    }
}
