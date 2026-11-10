//! The mods of ffx-mods that the converter builds files for, and the ones
//! without files (code only), with the settings and jobs that belong to each.
//! `demaster build --mod <name>` (or a `demaster.exe` in a mod folder, found
//! by the mod's manifest there) runs only that mod's jobs, reads only its
//! settings from its `settings.toml` and keeps the build bookkeeping
//! (`build/files.json`, `build/stamp.json`) in its folder. Without a mod the
//! converter builds everything into one folder (`demaster.toml`, for
//! development); `build --all` builds every installed mod (the installer of
//! the PS2 bundle, `mods/ps2-bundle`).
//!
//! The C# side keeps the same lists (`mods/<name>/Mod.cs`); the tests below
//! check that they agree, and that `mods/<name>/settings.default.toml` holds
//! the defaults. The zips ship that file; the installer makes the player's
//! `settings.toml` from it when there is none (an update never replaces the
//! player's), and a key the player's file leaves out gets the mod's default
//! ([`Config::parse_with_defaults`] over [`Profile::default_toml`]).

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

/// A setting of another mod that one of this mod's jobs reads: its output
/// depends on what that mod does in the game (PS2 movies are 4:3 when PS2
/// aspect ratio draws a 4:3 picture). The build takes the value from that
/// mod's `settings.toml` when the mod is installed, else [`Foreign::absent`],
/// and records it in the stamp, so the F8 window says "rebuild needed" when
/// that mod or its setting changes.
pub struct Foreign {
    pub key: &'static str,
    /// The mod the setting belongs to (`ps2-aspect-ratio`).
    pub from: &'static str,
    /// What the game does without that mod (`hd`).
    pub absent: &'static str,
}

const ASPECT: Foreign = Foreign { key: "render.aspect", from: "ps2-aspect-ratio", absent: "hd" };

pub struct Profile {
    /// The folder in ffx-mods (`mods/<group>/<name>`); the Fahrenheit id is `yuta.<name>`.
    pub name: &'static str,
    pub title: &'static str,
    /// Every setting of its `settings.toml`.
    pub settings: &'static [&'static str],
    /// Its converter jobs (names in `jobs::registry()`).
    pub jobs: &'static [&'static str],
    pub disc: DiscUse,
    /// Its defaults, `settings.default.toml` in the mod folder (and the zip).
    pub default_toml: &'static str,
    /// What stays HD without the disc ([`DiscUse::Optional`]).
    pub without_disc: &'static str,
    /// Settings of other mods its jobs read.
    pub reads: &'static [Foreign],
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
            .expect("the shipped settings.default.toml parses")
            .0;
        let all = config.settings();
        self.settings.iter().map(|k| (k.to_string(), all[*k].clone())).collect()
    }
}

/// The mods, in the order `demaster build --all` builds them and the
/// installer lists them in `mods\loadorder`. Their defaults are
/// [`Config::default`]'s.
pub const PROFILES: &[Profile] = &[
    Profile {
        name: "ps2-music-engine",
        title: "PS2 music engine",
        settings: &["audio.music_engine"],
        jobs: &["audio.live"],
        disc: DiscUse::Needed,
        default_toml: include_str!("../../mods/ps2/ps2-music-engine/settings.default.toml"),
        without_disc: "",
        reads: &[],
    },
    Profile {
        name: "ps2-soundtrack",
        title: "PS2 soundtrack",
        settings: &["audio.music", "audio.soundtrack"],
        jobs: &["audio.music"],
        disc: DiscUse::Needed,
        default_toml: include_str!("../../mods/ps2/ps2-soundtrack/settings.default.toml"),
        without_disc: "",
        reads: &[],
    },
    Profile {
        name: "ps2-movies",
        title: "PS2 movies",
        settings: &["movies"],
        jobs: &["movies"],
        disc: DiscUse::Needed,
        default_toml: include_str!("../../mods/ps2/ps2-movies/settings.default.toml"),
        without_disc: "",
        reads: &[ASPECT],
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
        default_toml: include_str!("../../mods/ps2/ps2-characters/settings.default.toml"),
        without_disc: "",
        reads: &[],
    },
    Profile {
        name: "ps2-maps",
        title: "PS2 maps",
        settings: &["maps.field", "maps.battle", "maps.backgrounds"],
        jobs: &["maps.field", "maps.battle", "maps.backgrounds"],
        disc: DiscUse::Needed,
        default_toml: include_str!("../../mods/ps2/ps2-maps/settings.default.toml"),
        without_disc: "",
        reads: &[ASPECT],
    },
    Profile {
        name: "ps2-effects",
        title: "PS2 effects",
        settings: &["effects.textures"],
        jobs: &["fx.textures"],
        disc: DiscUse::Needed,
        default_toml: include_str!("../../mods/ps2/ps2-effects/settings.default.toml"),
        without_disc: "",
        reads: &[],
    },
    Profile {
        name: "ps2-lighting",
        title: "PS2 lighting",
        settings: &["render.lighting"],
        jobs: &["render.lighting", "render.lighting.events"],
        disc: DiscUse::Optional,
        default_toml: include_str!("../../mods/ps2/ps2-lighting/settings.default.toml"),
        without_disc: "the event lights (the PS2's light colours in the 6 cutscene scripts the HD retuned) stay HD",
        reads: &[],
    },
    Profile {
        name: "no-hd-post-effects",
        title: "No HD post-effects",
        settings: &["render.glow", "render.shadows"],
        jobs: &[],
        disc: DiscUse::None,
        default_toml: include_str!("../../mods/ps2/no-hd-post-effects/settings.default.toml"),
        without_disc: "",
        reads: &[],
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
        default_toml: include_str!("../../mods/ps2/ps2-interface/settings.default.toml"),
        without_disc: "",
        reads: &[],
    },
    Profile {
        name: "ps2-aspect-ratio",
        title: "PS2 aspect ratio",
        settings: &["render.aspect"],
        jobs: &[],
        disc: DiscUse::None,
        default_toml: include_str!("../../mods/ps2/ps2-aspect-ratio/settings.default.toml"),
        without_disc: "",
        reads: &[],
    },
    Profile {
        name: "ps2-game-feel",
        title: "PS2 game feel",
        settings: &["timing.run_speed", "ui.battle_menu_delay", "ui.battle_subtitles"],
        jobs: &[],
        disc: DiscUse::None,
        default_toml: include_str!("../../mods/ps2/ps2-game-feel/settings.default.toml"),
        without_disc: "",
        reads: &[],
    },
    Profile {
        name: "bug-fixes",
        title: "Bug fixes",
        settings: &["fixes.npc_mouths", "fixes.farplane_video", "fixes.lulu_fury"],
        jobs: &[],
        disc: DiscUse::None,
        default_toml: include_str!("../../mods/ps2/bug-fixes/settings.default.toml"),
        without_disc: "",
        reads: &[],
    },
    Profile {
        name: "modern-minimap",
        title: "Modern Minimap",
        settings: &["minimap.style", "minimap.size", "minimap.corner"],
        jobs: &[],
        disc: DiscUse::None,
        default_toml: include_str!("../../mods/enhancements/modern-minimap/settings.default.toml"),
        without_disc: "",
        reads: &[],
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
            "frames.interpolate.objects",
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
        default_toml: include_str!("../../mods/enhancements/fps-unlock/settings.default.toml"),
        without_disc: "",
        reads: &[],
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

    /// The mod's folder: `mods/<group>/<name>`.
    fn mod_dir(name: &str) -> std::path::PathBuf {
        std::fs::read_dir(repo().join("mods"))
            .unwrap()
            .map(|g| g.unwrap().path().join(name))
            .find(|d| d.is_dir())
            .unwrap_or_else(|| panic!("no mods/<group>/{name}"))
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
                // a job may read only its own mod's settings, and those of
                // other mods it declares (`reads`)
                for s in job.settings() {
                    assert!(
                        p.settings.contains(s) || p.reads.iter().any(|f| f.key == *s),
                        "{}: job {j} reads {s}, another mod's setting",
                        p.name
                    );
                }
            }
            let reads_disc = p.jobs.iter().filter(|j| registry.iter().any(|x| x.name() == **j && x.needs_disc())).count();
            let expected = match p.disc {
                DiscUse::None => p.jobs.is_empty(),
                DiscUse::Needed => reads_disc == p.jobs.len() && !p.jobs.is_empty(),
                DiscUse::Optional => reads_disc > 0 && reads_disc < p.jobs.len() && !p.without_disc.is_empty(),
            };
            assert!(expected, "{}: disc use {:?} does not match its jobs", p.name, p.disc);
            for f in p.reads {
                let owner = find(f.from).unwrap_or_else(|| panic!("{}: reads {} from no mod {}", p.name, f.key, f.from));
                assert!(owner.settings.contains(&f.key), "{}: {} is not {}'s", p.name, f.key, f.from);
                assert!(!p.settings.contains(&f.key), "{}: reads its own {}", p.name, f.key);
                assert!(
                    registry.iter().any(|x| p.jobs.contains(&x.name()) && x.settings().contains(&f.key)),
                    "{}: no job reads {}",
                    p.name,
                    f.key
                );
            }
        }
    }

    /// Each mod's settings.default.toml holds exactly its settings with the
    /// defaults of [`Config::default`]; `[source]` only for mods that build.
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
            assert_eq!(settings, want, "{}: settings.default.toml", p.name);
            assert_eq!(keys.iter().any(|k| k == "source.iso"), p.disc != DiscUse::None, "{}: [source]", p.name);
            for (k, v) in p.defaults() {
                assert_eq!(v, base[&k], "{}: {k}", p.name);
            }
        }
    }

    /// A player's settings.toml from an older version lacks the newer
    /// settings: those get the mod's defaults, the player's values stay.
    #[test]
    fn an_older_settings_file_gets_the_new_defaults() {
        for p in PROFILES {
            let defaults = p.defaults();
            // the file without the last setting, and nothing at all
            let lines: Vec<&str> = p.default_toml.lines().collect();
            let last = lines.iter().rposition(|l| !l.starts_with('#') && l.contains('=')).unwrap();
            let older: String = lines.iter().enumerate().filter(|(i, _)| *i != last).map(|(_, l)| format!("{l}\n")).collect();
            for text in [older.as_str(), ""] {
                let (config, unknown) = Config::parse_with_defaults(text, p.default_toml).unwrap();
                assert!(unknown.is_empty(), "{}: {unknown:?}", p.name);
                let all = config.settings();
                for (k, v) in &defaults {
                    assert_eq!(&all[k], v, "{}: {k}", p.name);
                }
            }
        }
        // a value the player changed stays; the setting the file lacks is the default
        let p = find("ps2-interface").unwrap();
        let (config, _) = Config::parse_with_defaults("[ui]\nmenus = \"hd\"\n", p.default_toml).unwrap();
        assert_eq!(config.settings()["ui.menus"], "hd");
        assert_eq!(config.settings()["ui.boot_logo"], p.defaults()["ui.boot_logo"]);
    }

    /// The C# mod keeps its own copy of the list (`mods/<name>/Mod.cs`: the
    /// settings with who applies them and their defaults, a code patch for
    /// each setting the mod applies); `docs/config.md` describes each.
    #[test]
    fn mods_and_docs_know_every_setting() {
        let docs = std::fs::read_to_string(repo().join("docs/config.md")).unwrap();
        for p in PROFILES {
            let dir = mod_dir(p.name);
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

    /// Every setting has a mod (`maps.geometry` has nothing to build yet).
    #[test]
    fn every_setting_has_a_mod() {
        for (k, _) in SETTINGS {
            let owners = PROFILES.iter().filter(|p| p.settings.contains(k)).count();
            assert_eq!(owners, usize::from(*k != "maps.geometry"), "{k}");
        }
    }

    #[test]
    fn find_by_name_or_id() {
        assert_eq!(find("ps2-lighting").unwrap().name, "ps2-lighting");
        assert_eq!(find("yuta.bug-fixes").unwrap().name, "bug-fixes");
        assert!(find("demaster").is_none());
    }
}
