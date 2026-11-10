// SPDX-License-Identifier: MIT

namespace Yuta.FfxMods;

public enum BuildState {
    /// <summary>build/stamp.json matches settings.toml and the files are there.</summary>
    Ok,
    /// <summary>No stamp: never built, or a build was interrupted.</summary>
    NotBuilt,
    /// <summary>Built, but with settings or a converter version other than the current ones.</summary>
    Outdated,
    /// <summary>The last build had failed jobs.</summary>
    Incomplete,
}

/// <summary>
/// Compares build/stamp.json (written by demaster.exe) with the mod's settings.toml: whether the
/// PS2 files in efl/x were built with the settings the file has now.
/// </summary>
public sealed class BuildStatus {
    public BuildState   State = BuildState.NotBuilt;
    public string       Summary = "";
    /// <summary>Settings changed since the build: key, built value, value now.</summary>
    public readonly List<(string Key, string Built, string Now)> Changed = [];
    public readonly List<string> Failed = [];
    /// <summary>Jobs that need the PS2 disc and were left out because the build had none.</summary>
    public readonly List<string> NoDisc = [];
    public string?  ConverterVersion;
    public DateTime? BuiltAt;
    public int      Files;
    public int      FilesFound;
    /// <summary>Converter jobs, as "name: status, summary".</summary>
    public readonly List<string> Jobs = [];

    private sealed class Stamp {
        public string converter_version { get; set; } = "";
        public string config_hash { get; set; } = "";
        public long built_at { get; set; }
        public Dictionary<string, string> settings { get; set; } = [];
        public Dictionary<string, string> defaults { get; set; } = [];
        public Dictionary<string, StampForeign> foreign { get; set; } = [];
        public List<StampJob> jobs { get; set; } = [];
        public int files { get; set; }
        public bool complete { get; set; }
    }

    /// <summary>A setting of another mod the build read (demaster/src/mods.rs, Foreign).</summary>
    private sealed class StampForeign {
        [System.Text.Json.Serialization.JsonPropertyName("mod")]
        public string mod_id { get; set; } = "";
        public string value { get; set; } = "";
        public string @default { get; set; } = "";
        public string absent { get; set; } = "";
    }

    private sealed class StampJob {
        public string name { get; set; } = "";
        public string status { get; set; } = "";
        public int files { get; set; }
        public string summary { get; set; } = "";
    }

    /// <summary>The folder of the loaded mod with this id, or null when Fahrenheit didn't load it.</summary>
    private static string? loaded_mod_dir(string id) =>
        FhApi.Mods.get_mods().FirstOrDefault(m => m.Manifest.Id == id)?.Paths.ModDir.FullName;

    /// <summary>
    /// What another mod's setting is now: from its settings.toml when that mod is loaded (its
    /// default when the file leaves the key out), else the value the game has without it.
    /// </summary>
    public static string foreign_now(string? other_dir, string key, string fallback_default, string absent) {
        if (other_dir is null) return absent;
        string path = Path.Combine(other_dir, ModSettings.FILE);
        if (!File.Exists(path)) return fallback_default;
        MiniToml.Result r = MiniToml.parse(File.ReadAllText(path));
        return r.Values.TryGetValue(key, out string? v) ? v : fallback_default;
    }

    public static BuildStatus check(string mod_dir, ModSettings settings, string mod_version, Func<string, string?>? mod_dir_of = null) {
        BuildStatus s = new();
        string stamp_path = Path.Combine(mod_dir, "build", "stamp.json");
        s.FilesFound = files_found(mod_dir);
        if (!File.Exists(stamp_path)) {
            s.State   = BuildState.NotBuilt;
            s.Summary = s.FilesFound == 0
                ? "The files are not built yet."
                : "The files are not built, or the last build was interrupted.";
            return s;
        }
        Stamp stamp;
        try {
            stamp = JsonSerializer.Deserialize<Stamp>(File.ReadAllText(stamp_path)) ?? throw new InvalidDataException("empty");
        }
        catch (Exception e) {
            s.State   = BuildState.NotBuilt;
            s.Summary = $"build/stamp.json can't be read ({e.Message}).";
            return s;
        }
        s.ConverterVersion = stamp.converter_version;
        s.BuiltAt          = DateTimeOffset.FromUnixTimeSeconds(stamp.built_at).LocalDateTime;
        s.Files            = stamp.files;
        foreach (StampJob j in stamp.jobs) {
            s.Jobs.Add($"{j.name}: {j.status}{(j.status is "built" or "unchanged" ? $", {j.files} files" : "")}{(j.summary.Length > 0 ? $" ({j.summary})" : "")}");
            if (j.status == "failed") s.Failed.Add(j.name);
            if (j.status == "no disc") s.NoDisc.Add(j.name);
        }
        foreach ((string key, string built) in stamp.settings.OrderBy(p => p.Key, StringComparer.Ordinal)) {
            string now = settings.Values.TryGetValue(key, out string? v) ? v
                       : stamp.defaults.TryGetValue(key, out string? d) ? d
                       : built;
            if (now != built) s.Changed.Add((key, built, now));
        }
        // Settings of another mod the files depend on (render.aspect for the PS2 movies).
        foreach ((string key, StampForeign f) in stamp.foreign.OrderBy(p => p.Key, StringComparer.Ordinal)) {
            string? other = (mod_dir_of ?? loaded_mod_dir)(f.mod_id);
            string now = foreign_now(other, key, f.@default, f.absent);
            if (now != f.value) s.Changed.Add(($"{key} ({f.mod_id}{(other is null ? ", not installed" : "")})", f.value, now));
        }

        if (s.Changed.Count > 0) {
            s.State   = BuildState.Outdated;
            s.Summary = s.Changed.All(c => c.Key.Contains(' '))
                ? $"Another mod's setting these files depend on changed since the last build ({string.Join(", ", s.Changed.Select(c => c.Key))})."
                : $"settings.toml changed since the last build ({s.Changed.Count} setting{(s.Changed.Count == 1 ? "" : "s")}).";
        }
        else if (!same_version(stamp.converter_version, mod_version)) {
            s.State   = BuildState.Outdated;
            s.Summary = $"The files were built by converter {stamp.converter_version}; this mod is {mod_version}.";
        }
        else if (s.FilesFound < stamp.files) {
            s.State   = BuildState.Outdated;
            s.Summary = $"{stamp.files - s.FilesFound} of the {stamp.files} built files are missing.";
        }
        else if (!stamp.complete) {
            s.State   = BuildState.Incomplete;
            s.Summary = $"The last build had errors in: {string.Join(", ", s.Failed)}. The game shows the HD version of those.";
        }
        else {
            s.State   = BuildState.Ok;
            s.Summary = $"The files match settings.toml ({stamp.files} files)."
                      + (s.NoDisc.Count > 0 ? $" Built without the PS2 disc: {string.Join(", ", s.NoDisc)} not built, that part stays HD." : "");
        }
        return s;
    }

    /// <summary>
    /// How many of the files the last build lists (build/files.json: efl/x, and music/ for the
    /// music engine) are there; before any build, the files in efl/x.
    /// </summary>
    private static int files_found(string mod_dir) {
        string list = Path.Combine(mod_dir, "build", "files.json");
        try {
            using JsonDocument doc = JsonDocument.Parse(File.ReadAllText(list));
            int n = 0;
            foreach (JsonProperty job in doc.RootElement.GetProperty("jobs").EnumerateObject())
                foreach (JsonElement f in job.Value.GetProperty("files").EnumerateArray())
                    if (File.Exists(Path.Combine(mod_dir, f[0].GetString() ?? ""))) n++;
            return n;
        }
        catch (Exception) {
            string efl = Path.Combine(mod_dir, "efl", "x");
            return Directory.Exists(efl) ? Directory.EnumerateFiles(efl, "*", SearchOption.AllDirectories).Count() : 0;
        }
    }

    /// <summary>Versions without build metadata ("0.1.0-alpha+abc" and "0.1.0-alpha" are the same).</summary>
    private static bool same_version(string a, string b) =>
        string.Equals(a.Split('+')[0], b.Split('+')[0], StringComparison.OrdinalIgnoreCase);
}
