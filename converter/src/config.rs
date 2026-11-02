//! `demaster.toml`: where the PS2 disc and the game are, and one setting per
//! row of `docs/differences.md` (described in `docs/config.md`).
//!
//! The rule for defaults: presentation from the PS2, logic from the PC. A
//! presentation setting is `"ps2"` or `"hd"`; a fix is `true` (fixed) or
//! `false` (the PC's behaviour).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// The default file, shipped next to `demaster.exe`.
pub const DEFAULT_TOML: &str = include_str!("../demaster.toml");

/// Which version of a part of the game to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Look {
    #[default]
    Ps2,
    Hd,
}

impl Look {
    pub fn ps2(self) -> bool {
        self == Look::Ps2
    }
}

/// `render.texture_filter`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Filter {
    /// Whatever the game's graphics options say.
    #[default]
    Game,
    Nearest,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Source {
    /// The PS2 disc image (`.iso`).
    pub iso: String,
    /// The game folder (holds `FFX.exe` and `data/FFX_Data.vbf`). Empty: look
    /// in the usual Steam folders.
    pub game: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Build {
    /// The mod folder the files go into (`<mod>/efl/x/...`). Empty: the
    /// folder `demaster.exe` is in.
    pub mod_dir: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Models {
    pub party: Look,
    pub npc: Look,
    pub monster: Look,
    pub aeon: Look,
    pub weapon: Look,
    pub object: Look,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Maps {
    pub field: Look,
    pub geometry: Look,
    pub backgrounds: Look,
    pub battle: Look,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Effects {
    pub textures: Look,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Render {
    pub glow: Look,
    pub lighting: Look,
    pub shadows: Look,
    pub aspect: Look,
    pub texture_filter: Filter,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Ui {
    pub menus: Look,
    pub portraits: Look,
    pub font: Look,
    pub title: Look,
    pub loading: Look,
    pub battle_subtitles: Look,
    pub battle_menu_delay: Look,
    pub button_glyphs: Look,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Audio {
    pub soundtrack: Look,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Fixes {
    #[serde(default = "yes")]
    pub npc_mouths: bool,
    #[serde(default = "yes")]
    pub farplane_video: bool,
    #[serde(default = "yes")]
    pub lulu_fury: bool,
}

impl Default for Fixes {
    fn default() -> Self {
        Fixes {
            npc_mouths: true,
            farplane_video: true,
            lulu_fury: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Top-level key, before the tables in the file.
    pub movies: Look,
    pub source: Source,
    pub build: Build,
    pub models: Models,
    pub maps: Maps,
    pub effects: Effects,
    pub render: Render,
    pub ui: Ui,
    pub audio: Audio,
    pub fixes: Fixes,
}

/// Who applies a setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppliedBy {
    /// Files written by `demaster build`; changing it needs a rebuild.
    Converter,
    /// Code in the C# mod, read from `demaster.toml` when the game starts.
    Mod,
}

/// Every setting with who applies it. `docs/config.md` describes them; the
/// test below checks this list against [`Config`].
pub const SETTINGS: &[(&str, AppliedBy)] = {
    use AppliedBy::*;
    &[
        ("models.party", Converter),
        ("models.npc", Converter),
        ("models.monster", Converter),
        ("models.aeon", Converter),
        ("models.weapon", Converter),
        ("models.object", Converter),
        ("maps.field", Converter),
        ("maps.geometry", Converter),
        ("maps.backgrounds", Converter),
        ("maps.battle", Converter),
        ("effects.textures", Converter),
        ("render.glow", Mod),
        ("render.lighting", Mod),
        ("render.shadows", Mod),
        ("render.aspect", Mod),
        ("render.texture_filter", Mod),
        ("ui.menus", Converter),
        ("ui.portraits", Converter),
        ("ui.font", Converter),
        ("ui.title", Converter),
        ("ui.loading", Mod),
        ("ui.battle_subtitles", Mod),
        ("ui.battle_menu_delay", Mod),
        ("ui.button_glyphs", Converter),
        ("audio.soundtrack", Mod),
        ("movies", Converter),
        ("fixes.npc_mouths", Mod),
        ("fixes.farplane_video", Mod),
        ("fixes.lulu_fury", Mod),
    ]
};

pub struct Loaded {
    pub config: Config,
    /// The file it came from; relative paths in it are relative to its folder.
    pub path: PathBuf,
    /// Keys the converter doesn't know (typos, settings of a newer version).
    pub unknown: Vec<String>,
}

impl Config {
    /// Parses a config file's text. Unknown keys are collected, not errors.
    pub fn parse(text: &str) -> Result<(Config, Vec<String>)> {
        let mut unknown = Vec::new();
        let de = toml::Deserializer::new(text);
        let config = serde_ignored::deserialize(de, |path| unknown.push(path.to_string()))?;
        Ok((config, unknown))
    }

    pub fn load(path: &Path) -> Result<Loaded> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let (config, unknown) =
            Config::parse(&text).with_context(|| format!("in {}", path.display()))?;
        Ok(Loaded {
            config,
            path: path.to_path_buf(),
            unknown,
        })
    }

    /// Every setting (not `source`/`build`) as `key → value`, values written
    /// as plain text (`ps2`, `true`).
    pub fn settings(&self) -> BTreeMap<String, String> {
        let value = toml::Value::try_from(self).expect("Config serialises");
        let mut out = BTreeMap::new();
        flatten("", &value, &mut out);
        out.retain(|k, _| !k.starts_with("source.") && !k.starts_with("build."));
        out
    }
}

fn flatten(prefix: &str, v: &toml::Value, out: &mut BTreeMap<String, String>) {
    match v {
        toml::Value::Table(t) => {
            for (k, v) in t {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&key, v, out);
            }
        }
        toml::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

impl Loaded {
    /// A path from the file, relative to the file's folder; `None` if empty.
    pub fn resolve(&self, p: &str) -> Option<PathBuf> {
        if p.trim().is_empty() {
            return None;
        }
        let p = PathBuf::from(p.trim());
        Some(if p.is_absolute() {
            p
        } else {
            self.path.parent().unwrap_or(Path::new(".")).join(p)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_file_is_the_defaults() {
        let (c, unknown) = Config::parse(DEFAULT_TOML).unwrap();
        assert!(unknown.is_empty(), "{unknown:?}");
        assert_eq!(c.settings(), Config::default().settings());
    }

    #[test]
    fn settings_list_matches_config() {
        let keys: Vec<String> = Config::default().settings().into_keys().collect();
        let mut listed: Vec<String> = SETTINGS.iter().map(|(k, _)| k.to_string()).collect();
        listed.sort();
        assert_eq!(keys, listed);
    }

    /// The C# mod keeps its own copy of the list (mod/Settings.cs) and has a
    /// code patch for each setting it applies (mod/Patches.cs); docs/config.md
    /// describes each.
    #[test]
    fn mod_and_docs_know_every_setting() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../mod");
        let settings = std::fs::read_to_string(dir.join("Settings.cs")).unwrap();
        let patches = std::fs::read_to_string(dir.join("Patches.cs")).unwrap();
        let docs = std::fs::read_to_string(dir.join("../docs/config.md")).unwrap();
        let defaults = Config::default().settings();
        for (key, by) in SETTINGS {
            let by_cs = match by {
                AppliedBy::Converter => "Converter",
                AppliedBy::Mod => "Mod",
            };
            let line = settings
                .lines()
                .find(|l| l.contains(&format!("new(\"{key}\",")))
                .unwrap_or_else(|| panic!("{key} missing from mod/Settings.cs"));
            assert!(line.contains(&format!("AppliedBy.{by_cs},")), "{key}: {line}");
            assert!(line.contains(&format!("\"{}\")", defaults[*key])), "{key}: {line}");
            assert!(docs.contains(&format!("`{key}`")), "{key} missing from docs/config.md");
            assert_eq!(
                patches.contains(&format!("new(\"{key}\",")),
                *by == AppliedBy::Mod,
                "{key} in mod/Patches.cs"
            );
        }
    }

    #[test]
    fn defaults_follow_the_rule() {
        for (k, v) in Config::default().settings() {
            let ok = if k.starts_with("fixes.") {
                v == "true"
            } else if k == "render.texture_filter" {
                v == "game"
            } else {
                v == "ps2"
            };
            assert!(ok, "{k} = {v}");
        }
    }

    #[test]
    fn unknown_keys_and_bad_values() {
        let (c, unknown) =
            Config::parse("movies = \"hd\"\n[models]\nmonstr = \"hd\"\n[render]\nglow = \"hd\"\n")
                .unwrap();
        assert_eq!(unknown, ["models.monstr"]);
        assert_eq!(c.movies, Look::Hd);
        assert_eq!(c.render.glow, Look::Hd);
        assert_eq!(c.models.monster, Look::Ps2);
        assert!(Config::parse("[models]\nmonster = \"psx\"\n").is_err());
    }
}
