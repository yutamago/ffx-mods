// SPDX-License-Identifier: MIT

namespace Yuta.Demaster;

internal enum BuildState {
    /// <summary>build/stamp.json matches demaster.toml and the files are there.</summary>
    Ok,
    /// <summary>No stamp: never built, or a build was interrupted.</summary>
    NotBuilt,
    /// <summary>Built, but with settings or a converter version other than the current ones.</summary>
    Outdated,
    /// <summary>The last build had failed jobs.</summary>
    Incomplete,
}

/// <summary>
/// Compares build/stamp.json (written by demaster.exe) with demaster.toml: whether the PS2 files
/// in efl/x were built with the settings the file has now.
/// </summary>
internal sealed class BuildStatus {
    public BuildState   State = BuildState.NotBuilt;
    public string       Summary = "";
    /// <summary>Settings changed since the build: key, built value, value now.</summary>
    public readonly List<(string Key, string Built, string Now)> Changed = [];
    public readonly List<string> Failed = [];
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
        public List<StampJob> jobs { get; set; } = [];
        public int files { get; set; }
        public bool complete { get; set; }
    }

    private sealed class StampJob {
        public string name { get; set; } = "";
        public string status { get; set; } = "";
        public int files { get; set; }
        public string summary { get; set; } = "";
    }

    public static BuildStatus check(string mod_dir, DemasterSettings settings, string mod_version) {
        BuildStatus s = new();
        string stamp_path = Path.Combine(mod_dir, "build", "stamp.json");
        string efl = Path.Combine(mod_dir, "efl", "x");
        s.FilesFound = Directory.Exists(efl) ? Directory.EnumerateFiles(efl, "*", SearchOption.AllDirectories).Count() : 0;
        if (!File.Exists(stamp_path)) {
            s.State   = BuildState.NotBuilt;
            s.Summary = s.FilesFound == 0
                ? "The PS2 files are not built yet."
                : "The PS2 files are not built, or the last build was interrupted.";
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
        }
        foreach ((string key, string built) in stamp.settings.OrderBy(p => p.Key, StringComparer.Ordinal)) {
            string now = settings.Values.TryGetValue(key, out string? v) ? v
                       : stamp.defaults.TryGetValue(key, out string? d) ? d
                       : built;
            if (now != built) s.Changed.Add((key, built, now));
        }

        if (s.Changed.Count > 0) {
            s.State   = BuildState.Outdated;
            s.Summary = $"demaster.toml changed since the last build ({s.Changed.Count} setting{(s.Changed.Count == 1 ? "" : "s")}).";
        }
        else if (!same_version(stamp.converter_version, mod_version)) {
            s.State   = BuildState.Outdated;
            s.Summary = $"The PS2 files were built by converter {stamp.converter_version}; this mod is {mod_version}.";
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
            s.Summary = $"The PS2 files match demaster.toml ({stamp.files} files).";
        }
        return s;
    }

    /// <summary>Versions without build metadata ("0.1.0-alpha+abc" and "0.1.0-alpha" are the same).</summary>
    private static bool same_version(string a, string b) =>
        string.Equals(a.Split('+')[0], b.Split('+')[0], StringComparison.OrdinalIgnoreCase);
}
