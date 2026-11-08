//! The settings: where the PS2 disc and the game are, and one setting per
//! row of `docs/differences.md` (described in `docs/config.md`). FFX
//! Demaster has them all in one `demaster.toml`; each mod of ffx-mods has its
//! own in its `settings.toml` (`mods.rs`).
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

/// `audio.music_engine`: what plays when the game's soundtrack is
/// "Original" (`docs/music.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicEngine {
    /// The original soundtrack bank rendered by our PS2 sound driver.
    #[default]
    Ps2,
    /// The PC's recordings.
    Hd,
    /// The PS2 driver playing inside the game (`demaster_live.dll`),
    /// driven by the game's music commands; the rendered bank stays as
    /// the fallback (and for the three tracks with streamed parts).
    Live,
}

impl MusicEngine {
    /// Whether the rendered bank is built.
    pub fn bank(self) -> bool {
        self != MusicEngine::Hd
    }
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
    /// ffmpeg (for the movies). Empty: `ffmpeg` on the PATH.
    pub ffmpeg: String,
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
    pub texture_quality: Look,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// `"ps2"`: the PS2's "SQUARESOFT® presents" screen at boot. Off
    /// (`"hd"`) by default: it adds 6.5 s before the project logo.
    pub boot_logo: Look,
}

impl Default for Ui {
    fn default() -> Self {
        Ui {
            menus: Look::Ps2,
            portraits: Look::Ps2,
            font: Look::Ps2,
            title: Look::Ps2,
            loading: Look::Ps2,
            battle_subtitles: Look::Ps2,
            battle_menu_delay: Look::Ps2,
            button_glyphs: Look::Ps2,
            boot_logo: Look::Hd,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Audio {
    pub soundtrack: Look,
    /// `"ps2"`: the original soundtrack rendered from the PS2 disc's
    /// sequences by our PS2 sound driver, instead of the PC's recordings;
    /// `"live"`: that driver playing in the game.
    pub music_engine: MusicEngine,
}

/// Game speed (`docs/timing.md`). These touch play, not only the picture,
/// so they default to the PC's `"hd"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Timing {
    /// `"ps2"`: the PS2's default run speed (34; the PC's is 38).
    pub run_speed: Look,
}

impl Default for Timing {
    fn default() -> Self {
        Timing {
            run_speed: Look::Ps2,
        }
    }
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

/// `minimap.style`: the field minimap (`docs/minimap.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MinimapStyle {
    /// Round, centred on the player, turned with the camera, in the game's
    /// colours on a small CRT screen.
    #[default]
    Crt,
    /// The same without the CRT screen (no scanlines, band or sheen).
    Plain,
    /// The game's own minimap.
    Game,
}

/// `minimap.size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MinimapSize {
    Small,
    #[default]
    Medium,
    Large,
}

/// `minimap.corner`: where on the picture the Modern Minimap sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MinimapCorner {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
}

/// The Modern Minimap (mod `modern-minimap`, all applied by the mod).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Minimap {
    pub style: MinimapStyle,
    pub size: MinimapSize,
    pub corner: MinimapCorner,
}

/// `frames.target`: how fast FPS unlock lets the game draw (`docs/fps-unlock.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameTarget {
    /// The game's own limiter (30 fps in the world).
    Game,
    /// The refresh rate of the monitor the window is on.
    #[default]
    Monitor,
    /// `frames.fixed_fps`.
    Fixed,
    /// No limiter.
    Unlimited,
}

/// `frames.vsync`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameVsync {
    /// As the game's own setting has it.
    #[default]
    Game,
    On,
    Off,
}

/// `frames.between_ticks`: what a drawn frame without a world tick does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BetweenTicks {
    /// The game's draw again with the world held still.
    #[default]
    Redraw,
    /// The last frame again.
    Repeat,
}

/// `frames.effects`: the effects on frames between ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameEffects {
    /// The last tick's effect draws drawn again.
    #[default]
    Replay,
    /// The effects run every drawn frame (too fast).
    Run,
    /// Held: missing between ticks.
    Hold,
}

/// `frames.speed`: the time scale.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameSpeed {
    pub time_scale: f64,
    pub fast: f64,
    pub slow: f64,
    pub mute_when_fast: bool,
    pub booster_limits: bool,
    pub scene_time: bool,
}

impl Default for FrameSpeed {
    fn default() -> Self {
        FrameSpeed {
            time_scale: 1.0,
            fast: 10.0,
            slow: 0.25,
            mute_when_fast: true,
            booster_limits: true,
            scene_time: true,
        }
    }
}

/// `frames.interpolate`: what is drawn between the last two ticks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameInterpolate {
    pub camera: bool,
    pub characters: bool,
    pub bones: bool,
    pub cloth: bool,
    pub repeats: bool,
}

impl Default for FrameInterpolate {
    fn default() -> Self {
        FrameInterpolate {
            camera: true,
            characters: true,
            bones: true,
            cloth: true,
            repeats: true,
        }
    }
}

/// `frames.cuts`: when the camera cut or a character warped (drawn as the tick has it).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameCuts {
    pub camera_distance: f64,
    pub camera_degrees: f64,
    pub game_sets: bool,
    pub character_distance: f64,
    pub character_degrees: f64,
}

impl Default for FrameCuts {
    fn default() -> Self {
        FrameCuts {
            camera_distance: 100.0,
            camera_degrees: 45.0,
            game_sets: true,
            character_distance: 50.0,
            character_degrees: 120.0,
        }
    }
}

/// `frames.hold`: what redrawn frames hold still.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameHold {
    pub characters: bool,
    pub field_animation: bool,
    pub fades: bool,
    pub cloth: bool,
}

impl Default for FrameHold {
    fn default() -> Self {
        FrameHold {
            characters: true,
            field_animation: true,
            fades: true,
            cloth: true,
        }
    }
}

/// `frames.budget`: how many ticks one drawn frame may run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FrameBudget {
    pub max_real_time: f64,
    pub max_ticks: u32,
    pub max_update_ms: f64,
    pub whole_updates: u32,
}

impl Default for FrameBudget {
    fn default() -> Self {
        FrameBudget {
            max_real_time: 0.3,
            max_ticks: 100,
            max_update_ms: 40.0,
            whole_updates: 4,
        }
    }
}

/// FPS unlock (mod `fps-unlock`, all applied by the mod, live): frame rate,
/// drawing between ticks, time scale (`docs/fps-unlock.md`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Frames {
    pub enabled: bool,
    pub target: FrameTarget,
    pub fixed_fps: f64,
    pub vsync: FrameVsync,
    pub overlay: bool,
    pub between_ticks: BetweenTicks,
    pub effects: FrameEffects,
    pub speed: FrameSpeed,
    pub interpolate: FrameInterpolate,
    pub cuts: FrameCuts,
    pub hold: FrameHold,
    pub budget: FrameBudget,
}

impl Default for Frames {
    fn default() -> Self {
        Frames {
            enabled: true,
            target: FrameTarget::default(),
            fixed_fps: 120.0,
            vsync: FrameVsync::default(),
            overlay: true,
            between_ticks: BetweenTicks::default(),
            effects: FrameEffects::default(),
            speed: FrameSpeed::default(),
            interpolate: FrameInterpolate::default(),
            cuts: FrameCuts::default(),
            hold: FrameHold::default(),
            budget: FrameBudget::default(),
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
    pub timing: Timing,
    pub fixes: Fixes,
    pub minimap: Minimap,
    pub frames: Frames,
}

/// Who applies a setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppliedBy {
    /// Files written by `demaster build`; changing it needs a rebuild.
    Converter,
    /// Code in the C# mod, read from `demaster.toml` when the game starts.
    Mod,
    /// Both: files written by `demaster build` (a rebuild when it changes)
    /// and code in the mod that draws with them.
    Both,
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
        ("render.lighting", Both),
        ("render.shadows", Mod),
        ("render.aspect", Mod),
        ("render.texture_filter", Mod),
        ("render.texture_quality", Mod),
        ("ui.menus", Both),
        ("ui.portraits", Converter),
        ("ui.font", Converter),
        ("ui.title", Both),
        ("ui.loading", Both),
        ("ui.battle_subtitles", Mod),
        ("ui.battle_menu_delay", Mod),
        ("ui.button_glyphs", Converter),
        ("ui.boot_logo", Converter),
        ("audio.soundtrack", Mod),
        ("audio.music_engine", Both),
        ("movies", Converter),
        ("timing.run_speed", Mod),
        ("fixes.npc_mouths", Mod),
        ("fixes.farplane_video", Mod),
        ("fixes.lulu_fury", Mod),
        ("minimap.style", Mod),
        ("minimap.size", Mod),
        ("minimap.corner", Mod),
        ("frames.enabled", Mod),
        ("frames.target", Mod),
        ("frames.fixed_fps", Mod),
        ("frames.vsync", Mod),
        ("frames.overlay", Mod),
        ("frames.between_ticks", Mod),
        ("frames.effects", Mod),
        ("frames.speed.time_scale", Mod),
        ("frames.speed.fast", Mod),
        ("frames.speed.slow", Mod),
        ("frames.speed.mute_when_fast", Mod),
        ("frames.speed.booster_limits", Mod),
        ("frames.speed.scene_time", Mod),
        ("frames.interpolate.camera", Mod),
        ("frames.interpolate.characters", Mod),
        ("frames.interpolate.bones", Mod),
        ("frames.interpolate.cloth", Mod),
        ("frames.interpolate.repeats", Mod),
        ("frames.cuts.camera_distance", Mod),
        ("frames.cuts.camera_degrees", Mod),
        ("frames.cuts.game_sets", Mod),
        ("frames.cuts.character_distance", Mod),
        ("frames.cuts.character_degrees", Mod),
        ("frames.hold.characters", Mod),
        ("frames.hold.field_animation", Mod),
        ("frames.hold.fades", Mod),
        ("frames.hold.cloth", Mod),
        ("frames.budget.max_real_time", Mod),
        ("frames.budget.max_ticks", Mod),
        ("frames.budget.max_update_ms", Mod),
        ("frames.budget.whole_updates", Mod),
    ]
};

pub struct Loaded {
    pub config: Config,
    /// The file it came from; relative paths in it are relative to its folder.
    pub path: PathBuf,
    /// Keys the converter doesn't know (typos, settings of a newer version).
    pub unknown: Vec<String>,
    /// The file's text.
    pub text: String,
}

impl Config {
    /// Parses a config file's text. Unknown keys are collected, not errors.
    pub fn parse(text: &str) -> Result<(Config, Vec<String>)> {
        let mut unknown = Vec::new();
        let de = toml::Deserializer::new(text);
        let config = serde_ignored::deserialize(de, |path| unknown.push(path.to_string()))?;
        Ok((config, unknown))
    }

    /// Parses a mod's settings file over that mod's defaults (its shipped
    /// `settings.toml`): keys the file leaves out get the mod's defaults, not
    /// [`Config::default`]'s. Unknown keys are those of `text`.
    pub fn parse_with_defaults(text: &str, defaults: &str) -> Result<(Config, Vec<String>)> {
        // first alone, for its errors (with line numbers) and unknown keys
        let (_, unknown) = Config::parse(text)?;
        let mut base: toml::Table = defaults.parse().context("the default settings")?;
        let user: toml::Table = text.parse()?;
        merge(&mut base, user);
        let config: Config = toml::Value::Table(base).try_into()?;
        Ok((config, unknown))
    }

    /// The keys a settings file sets (`source.iso`, `models.npc`), sorted.
    pub fn present_keys(text: &str) -> Result<Vec<String>> {
        let table: toml::Table = text.parse()?;
        let mut out = BTreeMap::new();
        flatten("", &toml::Value::Table(table), &mut out);
        Ok(out.into_keys().collect())
    }

    pub fn load(path: &Path) -> Result<Loaded> {
        Config::load_with_defaults(path, None)
    }

    /// [`Config::load`], over `defaults` ([`Config::parse_with_defaults`]) when given.
    pub fn load_with_defaults(path: &Path, defaults: Option<&str>) -> Result<Loaded> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let (config, unknown) = match defaults {
            Some(d) => Config::parse_with_defaults(&text, d),
            None => Config::parse(&text),
        }
        .with_context(|| format!("in {}", path.display()))?;
        Ok(Loaded {
            config,
            path: path.to_path_buf(),
            unknown,
            text,
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

/// Puts `over`'s keys into `base`, table by table.
fn merge(base: &mut toml::Table, over: toml::Table) {
    for (k, v) in over {
        match (base.get_mut(&k), v) {
            (Some(toml::Value::Table(b)), toml::Value::Table(o)) => merge(b, o),
            (_, v) => {
                base.insert(k, v);
            }
        }
    }
}

/// `text` with `key = 'value'` in its `[source]` table: the line replaced
/// when there is one, else added after the table header (and the table
/// added at the end when there is none). Comments and the rest stay.
pub fn set_source(text: &str, key: &str, value: &str) -> String {
    let quoted = if value.contains('\'') || value.contains('\n') {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
    } else {
        format!("'{value}'")
    };
    let line = format!("{key} = {quoted}");
    let crlf = text.contains("\r\n");
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let header = lines.iter().position(|l| l.trim() == "[source]");
    match header {
        None => {
            if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push("[source]".into());
            lines.push(line);
        }
        Some(h) => {
            let end = lines[h + 1..]
                .iter()
                .position(|l| l.trim_start().starts_with('['))
                .map_or(lines.len(), |e| h + 1 + e);
            let found = (h + 1..end).find(|&i| {
                let l = lines[i].trim_start();
                l.strip_prefix(key).is_some_and(|r| r.trim_start().starts_with('='))
            });
            match found {
                Some(i) => lines[i] = line,
                None => lines.insert(h + 1, line),
            }
        }
    }
    let nl = if crlf { "\r\n" } else { "\n" };
    let mut out = lines.join(nl);
    out.push_str(nl);
    out
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

    /// docs/config.md describes every setting (the mods' own lists are
    /// checked in `mods.rs`).
    #[test]
    fn docs_know_every_setting() {
        let docs = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/config.md")).unwrap();
        for (key, _) in SETTINGS {
            assert!(docs.contains(&format!("`{key}`")), "{key} missing from docs/config.md");
        }
    }

    #[test]
    fn mod_defaults_and_source() {
        let defaults = "[render]\nlighting = \"ps2\"\n[audio]\nmusic_engine = \"live\"\n";
        let (c, unknown) = Config::parse_with_defaults("[render]\nlighting = \"hd\"\n[ui]\nmenus = \"hd\"\n", defaults).unwrap();
        assert_eq!(c.render.lighting, Look::Hd);
        assert_eq!(c.audio.music_engine, MusicEngine::Live);
        assert_eq!(c.ui.menus, Look::Hd);
        assert!(unknown.is_empty());
        assert_eq!(Config::present_keys("a = 1\n[source]\niso = ''\n").unwrap(), ["a", "source.iso"]);

        let text = "# c\n[source]\n# the disc\niso = ''\ngame = ''\n\n[render]\nlighting = \"ps2\"\n";
        let t = set_source(text, "iso", r"D:\PS2\FFX.iso");
        assert!(t.contains("iso = 'D:\\PS2\\FFX.iso'\ngame = ''"), "{t}");
        let t = set_source(&t, "game", "C:\\it's here");
        assert!(t.contains(r#"game = "C:\\it's here""#), "{t}");
        let (c, _) = Config::parse(&t).unwrap();
        assert_eq!(c.source.iso, r"D:\PS2\FFX.iso");
        assert_eq!(c.source.game, r"C:\it's here");
        let t = set_source("[render]\nlighting = \"ps2\"\n", "iso", "x.iso");
        assert!(t.ends_with("[source]\niso = 'x.iso'\n"), "{t}");
        let t = set_source("[source]\r\niso = ''\r\n", "iso", "y");
        assert_eq!(t, "[source]\r\niso = 'y'\r\n");
    }

    #[test]
    fn defaults_follow_the_rule() {
        for (k, v) in Config::default().settings() {
            let ok = if k.starts_with("fixes.") {
                v == "true"
            } else if k == "render.texture_filter" {
                v == "game"
            } else if k.starts_with("minimap.") {
                // not a PS2 part: the Modern Minimap is the mod's point
                matches!(k.as_str(), "minimap.style" if v == "crt")
                    || matches!(k.as_str(), "minimap.size" if v == "medium")
                    || matches!(k.as_str(), "minimap.corner" if v == "top_right")
            } else if k.starts_with("frames.") {
                // not a PS2 part: FPS unlock's defaults are the mod's (mods/fps-unlock/Mod.cs,
                // settings.toml; checked against each other in mods.rs and the mod's tests)
                true
            } else if k == "ui.boot_logo" {
                // an extra screen at every start: off unless asked for
                v == "hd"
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
