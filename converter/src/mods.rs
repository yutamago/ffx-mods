//! The mods of ffx-mods that the converter builds files for, and the ones
//! without files (code only), with the settings and jobs that belong to each.
//! `demaster build --mod <name>` (or a `demaster.exe` in a mod folder, found
//! by the mod's manifest there) runs only that mod's jobs, reads only its
//! settings from its `settings.toml` and keeps the build bookkeeping
//! (`build/files.json`, `build/stamp.json`) in its folder. Without a mod the
//! converter builds everything into one folder, as FFX Demaster does.
//!
//! The C# side keeps the same lists (`mods/<name>/Mod.cs`); the tests below
//! check that they agree, and that `mods/<name>/settings.toml` holds the
//! defaults.

use std::collections::BTreeMap;
use std::path::Path;

use crate::config::Config;

/// Whether a mod's files come from the PS2 disc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscUse {
    /// Every job reads the disc.
    Needed,
    /// Some jobs build from the PC game alone; without the disc the others
    /// are left out and their part stays HD.
    Optional,
    /// No converter jobs: the mod is code only.
    None,
}

pub struct Profile {
    /// The folder in ffx-mods (`mods/<name>`); the Fahrenheit id is `yuta.<name>`.
    pub name: &'static str,
    pub title: &'static str,
    /// Every setting of its `settings.toml`.
    pub settings: &'static [&'static str],
    /// Its converter jobs (names in `jobs::registry()`).
    pub jobs: &'static [&'static str],
    pub disc: DiscUse,
    /// Its default `settings.toml`, shipped in the mod folder.
    pub default_toml: &'static str,
    /// What stays HD without the disc ([`DiscUse::Optional`]).
    pub without_disc: &'static str,
}

impl Profile {
    /// The Fahrenheit mod id (`yuta.ps2-characters`), also the name of its
    /// manifest (`<id>.manifest.json`) and DLL.
    pub fn id(&self) -> String {
        format!("yuta.{}", self.name)
    }

    /// Its default settings (`key` -> value) as in [`Profile::default_toml`].
    pub fn defaults(&self) -> BTreeMap<String, String> {
        let config = Config::parse_with_defaults(self.default_toml, self.default_toml)
            .expect("the shipped settings.toml parses")
            .0;
        let all = config.settings();
        self.settings.iter().map(|k| (k.to_string(), all[*k].clone())).collect()
    }
}

/// The mods. The seven PS2 ones' settings and defaults are FFX Demaster's, except
/// `audio.music_engine`, whose default is `"live"` here: the mod is the live
/// engine (the rendered bank, `"ps2"`, stays FFX Demaster's).
pub const PROFILES: &[Profile] = &[
    Profile {
        name: "ps2-music-engine",
        title: "PS2 music engine",
        settings: &["audio.music_engine"],
        jobs: &["audio.live"],
        disc: DiscUse::Needed,
        default_toml: include_str!("../../mods/ps2-music-engine/settings.toml"),
        without_disc: "",
    },
    Profile {
        name: "ps2-characters",
        title: "PS2 characters",
        settings: &[
            "models.party",
            "models.npc",
            "models.monster",
            "models.aeon",
            "models.weapon",
            "models.object",
            "render.texture_filter",
            "render.texture_quality",
        ],
        jobs: &[
            "chr.party",
            "chr.monster",
            "chr.npc",
            "chr.aeon",
            "chr.weapon",
            "chr.object",
            "models.party",
            "models.monster",
            "models.npc",
            "models.aeon",
            "models.weapon",
            "models.object",
        ],
        disc: DiscUse::Needed,
        default_toml: include_str!("../../mods/ps2-characters/settings.toml"),
        without_disc: "",
    },
    Profile {
        name: "ps2-lighting",
        title: "PS2 lighting",
        settings: &["render.lighting"],
        jobs: &["render.lighting", "render.lighting.events"],
        disc: DiscUse::Optional,
        default_toml: include_str!("../../mods/ps2-lighting/settings.toml"),
        without_disc: "the event lights (the PS2's light colours in the 6 cutscene scripts the HD retuned) stay HD",
    },
    Profile {
        name: "ps2-interface",
        title: "PS2 interface",
        settings: &[
            "ui.menus",
            "ui.portraits",
            "ui.font",
            "ui.title",
            "ui.loading",
            "ui.button_glyphs",
            "ui.boot_logo",
        ],
        jobs: &["ui.menus", "ui.title", "ui.loading", "ui.glyphs", "ui.font", "ui.boot_logo"],
        disc: DiscUse::Needed,
        default_toml: include_str!("../../mods/ps2-interface/settings.toml"),
        without_disc: "",
    },
    Profile {
        name: "ps2-aspect-ratio",
        title: "PS2 aspect ratio",
        settings: &["render.aspect"],
        jobs: &[],
        disc: DiscUse::None,
        default_toml: include_str!("../../mods/ps2-aspect-ratio/settings.toml"),
        without_disc: "",
    },
    Profile {
        name: "ps2-game-feel",
        title: "PS2 game feel",
        settings: &["timing.run_speed", "ui.battle_menu_delay", "ui.battle_subtitles"],
        jobs: &[],
        disc: DiscUse::None,
        default_toml: include_str!("../../mods/ps2-game-feel/settings.toml"),
        without_disc: "",
    },
    Profile {
        name: "bug-fixes",
        title: "Bug fixes",
        settings: &["fixes.npc_mouths", "fixes.farplane_video", "fixes.lulu_fury"],
        jobs: &[],
        disc: DiscUse::None,
        default_toml: include_str!("../../mods/bug-fixes/settings.toml"),
        without_disc: "",
    },
    Profile {
        name: "modern-minimap",
        title: "Modern Minimap",
        settings: &["minimap.style", "minimap.size", "minimap.corner"],
        jobs: &[],
        disc: DiscUse::None,
        default_toml: include_str!("../../mods/modern-minimap/settings.toml"),
        without_disc: "",
    },
    Profile {
        name: "fps-unlock",
        title: "FPS unlock",
        settings: &[
            "frames.enabled",
            "frames.target",
            "frames.fixed_fps",
            "frames.vsync",
            "frames.overlay",
            "frames.between_ticks",
            "frames.effects",
            "frames.speed.time_scale",
            "frames.speed.fast",
            "frames.speed.slow",
            "frames.speed.mute_when_fast",
            "frames.speed.booster_limits",
            "frames.speed.scene_time",
            "frames.interpolate.camera",
            "frames.interpolate.characters",
            "frames.interpolate.bones",
            "frames.interpolate.cloth",
            "frames.interpolate.repeats",
            "frames.cuts.camera_distance",
            "frames.cuts.camera_degrees",
            "frames.cuts.game_sets",
            "frames.cuts.character_distance",
            "frames.cuts.character_degrees",
            "frames.hold.characters",
            "frames.hold.field_animation",
            "frames.hold.fades",
            "frames.hold.cloth",
            "frames.budget.max_real_time",
            "frames.budget.max_ticks",
            "frames.budget.max_update_ms",
            "frames.budget.whole_updates",
        ],
        jobs: &[],
        disc: DiscUse::None,
        default_toml: include_str!("../../mods/fps-unlock/settings.toml"),
        without_disc: "",
    },
];

/// A mod by its name (`ps2-characters`) or id (`yuta.ps2-characters`).
pub fn find(name: &str) -> Option<&'static Profile> {
    let name = name.strip_prefix("yuta.").unwrap_or(name);
    PROFILES.iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

/// The mod whose manifest is in `dir` (a Fahrenheit mod folder).
pub fn in_dir(dir: &Path) -> Option<&'static Profile> {
    PROFILES
        .iter()
        .find(|p| dir.join(format!("{}.manifest.json", p.id())).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppliedBy, SETTINGS};
    use crate::jobs;
    use std::collections::BTreeSet;

    fn repo() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    #[test]
    fn settings_and_jobs_exist_and_belong_to_one_mod() {
        let known: BTreeSet<&str> = SETTINGS.iter().map(|(k, _)| *k).collect();
        let registry: Vec<Box<dyn jobs::Job>> = jobs::registry();
        let mut settings = BTreeSet::new();
        let mut job_names = BTreeSet::new();
        for p in PROFILES {
            for s in p.settings {
                assert!(known.contains(s), "{}: no setting {s}", p.name);
                assert!(settings.insert(*s), "{s} is in two mods");
            }
            for j in p.jobs {
                let job = registry
                    .iter()
                    .find(|x| x.name() == *j)
                    .unwrap_or_else(|| panic!("{}: no job {j}", p.name));
                assert!(job_names.insert(*j), "{j} is in two mods");
                // a job may read only its own mod's settings
                for s in job.settings() {
                    assert!(p.settings.contains(s), "{}: job {j} reads {s}, another mod's setting", p.name);
                }
            }
            let reads_disc = p.jobs.iter().filter(|j| registry.iter().any(|x| x.name() == **j && x.needs_disc())).count();
            let expected = match p.disc {
                DiscUse::None => p.jobs.is_empty(),
                DiscUse::Needed => reads_disc == p.jobs.len() && !p.jobs.is_empty(),
                DiscUse::Optional => reads_disc > 0 && reads_disc < p.jobs.len() && !p.without_disc.is_empty(),
            };
            assert!(expected, "{}: disc use {:?} does not match its jobs", p.name, p.disc);
        }
    }

    /// Each mod's settings.toml holds exactly its settings with FFX Demaster's
    /// defaults (`audio.music_engine` excepted, see [`PROFILES`]); `[source]`
    /// only for mods that build.
    #[test]
    fn default_files_hold_the_defaults() {
        let base = Config::default().settings();
        for p in PROFILES {
            let (_, unknown) = Config::parse(p.default_toml).unwrap();
            assert!(unknown.is_empty(), "{}: {unknown:?}", p.name);
            let keys = Config::present_keys(p.default_toml).unwrap();
            let settings: Vec<&str> = keys
                .iter()
                .map(String::as_str)
                .filter(|k| !k.starts_with("source.") && !k.starts_with("build."))
                .collect();
            let mut want: Vec<&str> = p.settings.to_vec();
            want.sort_unstable();
            assert_eq!(settings, want, "{}: settings.toml", p.name);
            assert_eq!(keys.iter().any(|k| k == "source.iso"), p.disc != DiscUse::None, "{}: [source]", p.name);
            for (k, v) in p.defaults() {
                let expected = if k == "audio.music_engine" { "live" } else { base[&k].as_str() };
                assert_eq!(v, expected, "{}: {k}", p.name);
            }
        }
    }

    /// The C# mod keeps its own copy of the list (`mods/<name>/Mod.cs`: the
    /// settings with who applies them and their defaults, a code patch for
    /// each setting the mod applies); `docs/config.md` describes each.
    #[test]
    fn mods_and_docs_know_every_setting() {
        let docs = std::fs::read_to_string(repo().join("docs/config.md")).unwrap();
        for p in PROFILES {
            let dir = repo().join("mods").join(p.name);
            let cs = std::fs::read_to_string(dir.join("Mod.cs")).unwrap();
            assert!(cs.contains(&format!("new(\"{}\",", p.name)), "{}: ModInfo name", p.name);
            assert!(
                cs.contains(&format!("Builds: {}", p.disc != DiscUse::None)),
                "{}: ModInfo Builds",
                p.name
            );
            assert!(
                dir.join(format!("{}.csproj", p.id())).is_file(),
                "{}: no {}.csproj",
                p.name,
                p.id()
            );
            let defaults = p.defaults();
            let lines: Vec<&str> = cs.lines().filter(|l| l.contains("AppliedBy.")).collect();
            assert_eq!(lines.len(), p.settings.len(), "{}: Mod.cs SETTINGS", p.name);
            for key in p.settings {
                let by = SETTINGS.iter().find(|(k, _)| k == key).unwrap().1;
                let by_cs = match by {
                    AppliedBy::Converter => "Converter",
                    AppliedBy::Mod => "Mod",
                    AppliedBy::Both => "Both",
                };
                let line = lines
                    .iter()
                    .find(|l| l.contains(&format!("new(\"{key}\",")))
                    .unwrap_or_else(|| panic!("{key} missing from mods/{}/Mod.cs", p.name));
                assert!(line.contains(&format!("AppliedBy.{by_cs},")), "{key}: {line}");
                assert!(line.contains(&format!("\"{}\")", defaults[*key])), "{key}: {line}");
                let patch = cs
                    .lines()
                    .filter(|l| !l.contains("AppliedBy."))
                    .any(|l| l.contains(&format!("new(\"{key}\",")));
                assert_eq!(patch, by != AppliedBy::Converter, "{key}: code patch in mods/{}/Mod.cs", p.name);
                assert!(docs.contains(&format!("`{key}`")), "{key} missing from docs/config.md");
            }
        }
    }

    #[test]
    fn find_by_name_or_id() {
        assert_eq!(find("ps2-lighting").unwrap().name, "ps2-lighting");
        assert_eq!(find("yuta.bug-fixes").unwrap().name, "bug-fixes");
        assert!(find("demaster").is_none());
    }
}
