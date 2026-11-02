# FFX: PS2 (PAL/International, 2002) vs. FFX HD Remaster (PC/Steam, 2016): Differences Catalogue

Compiled 2026-11-02. The baseline is the **2002 PAL German PS2 disc**. That disc is built on *FFX International* and already has the Dark Aeons, Penance and the Expert Sphere Grid. The target is the **Steam HD Remaster** (Virtuos/Square Enix, PhyreEngine, D3D11, FMOD audio, 32-bit exe).

Confidence markers: **[confirmed]** means a reliable or primary source, or several sources agree. **[uncertain]** means a single forum or user claim, or an inference.

Main sources, with short IDs used in the tables:

| ID | Source |
|---|---|
| FFW | Final Fantasy Wiki, *FFX version differences*: https://finalfantasy.fandom.com/wiki/Final_Fantasy_X_version_differences (read via the MediaWiki API) |
| PCGW | PCGamingWiki: https://www.pcgamingwiki.com/wiki/Final_Fantasy_X/X-2_HD_Remaster (read via the API) |
| FAM | Famitsu developer interview (Kitase/Imaizumi, Apr 2013): https://www.famitsu.com/news/201304/01030942.html |
| SK | Special K wiki, UnX page: https://wiki.special-k.info/SpecialK/Custom/UnX |
| UNXV | UnX changelog: https://github.com/Kaldaien/UnX/blob/master/version.ini |
| RE | ResetEra, "Restored & Refined PS2 Models": https://www.resetera.com/threads/restored-refined-ps2-models-for-final-fantasy-x-remaster-pc-mod.1118226/ |
| NX/n | Nexus Mods FFX/X-2 mod *n*: https://www.nexusmods.com/finalfantasyxx2hdremaster/mods/n |
| WP | Wikipedia, *Final Fantasy X*: https://en.wikipedia.org/wiki/Final_Fantasy_X |

---

## 1. Character models (main cast)

| Difference | PS2 (PAL Intl.) | HD (PC) | Source |
|---|---|---|---|
| Playable characters remodelled | Original models | **All playable characters were reworked.** Imaizumi: 「操作キャラクターは全員手を加えています」 ("every playable character has been worked on") [confirmed] | FAM |
| Two model tiers | High-poly event models and low-poly field/battle models (fan theory: the PS2 used the more expressive HP faces in more cutscenes) [uncertain] | Same split: **HP models for cutscenes, LP models for field/battle**. Many HD cutscenes appear to fall back to LP models, which explains the loss of expression [partly confirmed] | FAM (HP vs LP models shown); NX/227 (HP/LP explanation); https://steviegamingworld.wordpress.com/2013/03/24/final-fantasy-x-hd-remaster-comparison-images-and-views/ ; RE |
| Faces: Tidus and Yuna worst affected | Softer, more expressive faces | New faces, often called "plastic"; Tidus has a "wonky eye texture", eyes are too bright, lips too thick; facial animation is less expressive; "duplicate eyeballs" reported [confirmed as widespread criticism] | RE; NX/143; https://www.gamesradar.com/10-years-later-final-fantasy-x-fans-are-still-lamenting-the-damage-done-by-the-hd-remaster/ |
| Wakka | Original face | Redone face; teeth too prominent when talking | RE |
| Auron | Original | Seen as the most successful remodel, especially his hair, which matches his CG model | RE; NX/232 |
| Kimahri | Original | Remodelled, but reportedly "not hit as hard"; it is the only party member without a "Restore PS2" mod | RE |
| Yuna body | — | Model is thinner | steviegamingworld (2013 blog) |
| Rikku | Earrings present | Reportedly **missing her earrings in non-CG scenes** [uncertain] | https://steamcommunity.com/app/359870/discussions/0/600774870011216270/ |
| Hair, weapons | Original | Tidus's hair and weapon look better; the water inside the Brotherhood sword animates (PS3+) | steviegamingworld |
| Hair/foliage shading (PS4/PC vs PS3) | n/a | PS4 base adds extra hair and foliage shading that the PS3 lacks | DF face-off quoted at https://www.neogaf.com/threads/digital-foundry-final-fantasy-x-x-2-hd-remaster-ps4-face-off.1047220/ |
| Spirit Lance (Kimahri) | Correct | **PC-only glitch:** the feathers and outer prongs don't render when the spear is seen from the left [confirmed] | FFW |

## 2. NPC, monster and summon models

| Difference | PS2 | HD | Source |
|---|---|---|---|
| NPCs and monsters | Original | Mostly **new textures, not new models** (PS3 base) [confirmed] | FAM |
| Important NPCs (PS4, and therefore PC) | Original | Important NPCs got improved models and textures; NPCs "no longer have low-quality hands and fingers" | FFW (PS4 section; PC is "based on the PS4 version") |
| Specific creatures and NPCs rebuilt | — | DF: "specific NPCs and creatures do benefit from being rebuilt from scratch" | NeoGAF DF quote (above) |
| Aeons | Original | Bahamut and Yojimbo mentioned as getting HD-quality models [confirmed in press coverage] | WP |
| NPC mouths | Correct | **Several NPCs erroneously have their mouths open in some cutscenes** (introduced bug) | FFW |
| Seymour Flux intro | Stutters in place before the fight (Intl.) | No longer teleports or stutters | FFW |
| Evrae | Body misaligned during Swooping Scythe (Intl.) | Unchanged (not listed as fixed) [uncertain] | FFW |

## 3. Textures (field, battle, UI)

| Difference | PS2 | HD | Source |
|---|---|---|---|
| General | Low-res PS2 textures | Textures "digitally enhanced" and more vibrant; quality is **uneven**: many textures stay low-res even on "high", and sometimes an NPC's clothing is HD but the face isn't | NeoGAF DF quote; https://steamcommunity.com/app/359870/discussions/0/2266943550312098927/ |
| Ground and environment textures | — | "Most ground textures still look very much the same" (user) [uncertain] | https://steamcommunity.com/app/359870/discussions/0/1621724915813077923/ (search summary) |
| Pre-rendered 2D backgrounds (indoor/fixed cameras) | 4:3 | Extended to 16:9 with **"a lot of additional painting"** (「かなり描き足しています」); upscale artefacts are visible | FAM; https://steamcommunity.com/app/359870/discussions/0/600787749709037968/ |
| Texture quality option | n/a | PC "Texture Quality" setting | PCGW; https://wccftech.com/final-fantasy-x2-hd-remaster-pc-graphics-options-revealed-framerate-locked-30fps/ |
| Storage | PS2 formats | `.dds.phyre` textures (DXT1/3/5) and `.dae.phyre` models inside VBF archives (`ffx_data/gamedata/ps3data/...`); PS3 paths kept | NX/293; NX/315; NX/150 |
| Banding | — | Block-compression banding on gradients (menus, skies) [modder claim] | NX/315 |

## 4. Lighting, shading, bloom, post-processing, shadows

| Difference | PS2 | HD | Source |
|---|---|---|---|
| Renderer | PS2 GS custom engine | PhyreEngine, D3D11 | PCGW; https://steamcommunity.com/app/359870/discussions/0/570371162116235475/ |
| Scene lighting | Original grade | Relit, sometimes darker and sometimes brighter (e.g. the boat to Luca is much brighter); mod authors call it "mostly for the worse" | steviegamingworld; RE |
| Face shadows | Present in compared scenes | "Face shadows missing in remaster" in at least one compared cutscene [uncertain] | search summary of Steam thread 600774870011216270 (not visible on fetch) |
| Glow/bloom around light sources | Not present in this form | **Engine glow/blur post-process** in Besaid, Mi'ihen, Bikanel and similar areas (blown-out sand, clouds, fire). It can be disabled by blocking vertex shader `7e29e6ac` in UnX/Special K | NX/358 |
| PS2-style bloom and depth of field | PS2 had DOF/bloom-like softness | Some PS2 bloom is "absent"; the remaster "lacks proper Depth of Field" [modder claims] | RE; NX/241 |
| Shadows | Simpler PS2 shadows (exact technique not verified) [uncertain] | Shadow maps (PC up to 4096²); PS4 base adds "dynamic shadows" | https://gamingtrend.com/reviews/final-fantasy-xx-2-hd-remastered-steam-version-review/ ; https://gamingbolt.com/final-fantasy-xx-2-hd-remaster-visual-analysis-ps3-vs-ps4 |
| PC post options | n/a | Brightness, Colour Correction (on/off; exact effect undocumented), SMAA post AA, MSAA ≤8x, Unsharp Mask, HDAO, AF ≤16x | PCGW; wccftech (above) |
| Dream/flashback haze filter | Present | Present; the HD "Hide HUD" booster may remove it | PCGW |
| Fire, smoke, water | Original | Improved (PS4) | GamingBolt |

## 5. Effects, magic, summons

| Difference | PS2 | HD | Source |
|---|---|---|---|
| Effect code location | In PS2 binaries | Per-effect native DLLs: `magicFiles/FFX/magicNNNN.dll` (e.g. `magic0386.dll` = Requiem). UnX blocks the game from unloading `magic_*.dll` | NX/266; UNXV 0.9.0.3 |
| Summon sounds (PS4 base) | Correct | Some Valefor summon/Energy Blast SFX missing; Anima's shortened summon roar and Oblivion's explosion play twice | FFW (PS4) |
| Ahriman Gaze | Sound present | Initial Gaze sound is sometimes silent | FFW |
| Distillers, Mix | No text label | Shows "Power/Magic/Speed/Ability"; Mix shows result text ("Max HP x2" etc.) | FFW |
| Summon length option | Fixed | Vita/Switch only (not PC) | FFW |

## 6. FMVs

| Difference | PS2 | HD | Source |
|---|---|---|---|
| Aspect ratio | 4:3 | **Cropped to 16:9** [confirmed] | FFW; PCGW; NX/343 |
| Handling | — | Visual Works (Ikumori) re-adjusted framing, so not a naive crop; whether FMVs were re-rendered from masters or upscaled is **not stated** [uncertain] | FAM |
| Quality | PS2 MPEG, "muddy" | Much clearer | Steam 2266943550312098927 |
| PAL timing | Re-encoded for 50 Hz; forced 60 Hz cuts audio out at regular intervals | No issue | https://gamefaqs.gamespot.com/ps2/197344-final-fantasy-x/faqs/24330 ; https://ask.metafilter.com/46201/How-to-play-FFX-on-my-US-PS2 |
| Audio | **5.1 in some movies** | Unified to 2-channel stereo; no surround | FAM; PCGW |
| Language-specific FMVs | EN only on PAL | JP/US FMV sets (`video` folder; UnX `Video=jp/us`) | SK; NX/121 |
| Skipping | Not possible | Some FMVs skippable on PC (not all) | PCGW; FFW |
| Farplane background video | Plays | **Never played on PC** (bug); fixed by NX/355 | NX/355 |
| Splash logos | Square, SCEE etc. | Square Enix, Virtuos, CRI, Dolby splash videos (unskippable) | NX/353 |
| Framerate and codec | PS2 ≈30 fps NTSC, 25 fps PAL [uncertain] | Not verified; 4K replacement FMVs stutter in the in-game player | NX/121 |

## 7. Aspect ratio and camera

| Difference | PS2 | HD | Source |
|---|---|---|---|
| Output | 4:3; PAL is letterboxed with black bars top and bottom | 16:9 native; narrower ratios are letterboxed; no 4:3 option | FFW; PCGW; https://www.neowin.net/forum/topic/31699-bad-pal-conversion-final-fantasy-x/ |
| 3D field and battle | 4:3 framing | Handled by camera adjustment (wider view), so characters "waiting offstage" can show and were adjusted | FAM; Steam 600787749709037968 |
| Fixed-camera and 2D-background rooms | 4:3 | Widened backgrounds, or cropped vertically in places (e.g. outside the Zanarkand stadium) | FAM; Steam 600787749709037968 |
| Indoor vs outdoor | — | The engine uses **different aspect handling indoors and outdoors** | PCGW (Anima Edition note) |
| Ultrawide | n/a | Hackable via Kamiccolo's SUWSF "Anima Edition" (Hor+ outdoors, Vert− indoors; "true scrolling on 4x3 added to FFX") | https://community.pcgamingwiki.com/files/file/2432-final-fantasy-xx-2-hd-remaster-anima-edition/ |
| FOV | — | Changing it breaks effects | PCGW |
| 4:3 gameplay mod | — | **None found** (Steam threads conclude none exists) | Steam 600787749709037968 |

## 8. UI (menus, fonts, HUD, sphere grid, title/loading)

| Difference | PS2 | HD | Source |
|---|---|---|---|
| Typeface | PS2 font (International damage-number font) | **New typeface** for game text | FFW |
| Menus | PS2 blue-window UI, PS2 cursor | **New UI and new cursor**. Some submenus use vertex-drawn gradients (can't be fixed by texture swap) | FFW; NX/348 |
| Portraits | PS2 portraits | New portraits | NX/41 |
| Pause text | PS2 style | Different (`flash/pausemenu.swf`) | NX/107 |
| Loading screen | — | Rotating Zanarkand Abes emblem over "Loading" in Spiran script (`menu/loading/.../loadingbg.dds.phyre`) | FFW; NX/108 |
| Title screen | PlayOnline URL shown; plain FFX logo | PlayOnline removed; logo carries "HD Remaster" text; main menu has a looping water header animation | FFW; NX/261; NX/18 |
| Battle quotes/subtitles | **Battle dialogue subtitled on screen** (Intl.) | **Not displayed** | FFW |
| Battle menu | Responsive | **Added delay** (0.30/0.45 s input delays, 0.20 s animation delays) | FFW; NX/360 |
| Battle HUD | PS2 | New style; Active/Wait indicator "blob" | NX/34 |
| Sphere grid | PS2 grid art | HD grid art and icons (several replacement mods exist) | NX/19, NX/5, NX/201 |
| "Miss/Immune/Overkill" | PS2 graphic text | New graphic text | NX/149 |
| Flash UI | n/a | Some UI is SWF (Flash) | NX/107 |

## 9. Audio: music, voice, SFX

| Difference | PS2 | HD | Source |
|---|---|---|---|
| Soundtrack | PS2 sound-chip sequenced OST | **Arranged by default** (60+ tracks rearranged by Hamauzu, Nakano, Narita, Noda, Yamazaki; "switched to live sound"). Hymn variants, vocal tracks and the Ending Theme were not rearranged | FAM; https://finalfantasy.fandom.com/wiki/Final_Fantasy_X_HD_Remaster_Original_Soundtrack ; https://www.rpgfan.com/music-review/final-fantasy-x-hd-remaster-original-soundtrack/ |
| OST switch | n/a | PC/PS4 can switch **Original vs Arranged** (PS3 was arranged-only) | FFW; PCGW; https://steamcommunity.com/app/359870/discussions/0/364042703864293738/ |
| Braska's Final Aeon battle | Alternate "Otherworld" version (extra percussion and bass line) | FMV version of "Otherworld" used. Whether this also applies in Original mode is **unverified** [uncertain] | FFW; HD OST wiki |
| Music after battle | Resets | Continues (PS4 1.01; probably inherited by PC) [uncertain for PC] | FFW |
| Fidelity of "Original" mode vs PS2 | — | Not documented; may be re-captured or compressed [uncertain] | — |
| Voices | **English only** (PAL); no JP subtitles | EN + JP. Vanilla PC ties the voice to the text language (German text gives English voice); UnX unlocks any combination | FFW; SK; https://wccftech.com/final-fantasy-x2-hd-remaster-pc-untitled-project-mod-adds-audio-language-selection-dpi-scaling-fixes/ |
| Missing EN lines | Present | Yuna/Lulu/Rikku grunts on Blitz Ace, "Let's ask around!" (Luca) and Tidus's "I'm not gonna let Yuna die!" are missing; Rikku's "WOO HOO" in Blitz Ace is reportedly gone [last item uncertain] | FFW; Steam 600774870011216270 |
| Enemy SFX | EN-version grunts | JP-version grunts (Seymour, BFA); new enemy SFX on PS4/PC | FFW |
| Unused battle voice lines | Unused in both | About 100 lines exist in the banks but never play (script bugs in `system_01.bin`); NX/361 restores them | NX/361 |
| Turbo audio | n/a | Vanilla Turbo mutes SFX and voice (BGM stays) | PCGW; FFW |
| Middleware | PS2 SPU2 | FMOD | PCGW |

## 10. Framerate, timing, PAL

| Difference | PS2 PAL | HD PC | Source |
|---|---|---|---|
| Refresh/speed | **50 Hz, ~16.7% slower** than NTSC (Wiki says "20% slower"); no official 60 Hz; cheat-device 60 Hz codes exist (FMV audio breaks) | Full speed | FFW; GameFAQs 60 Hz FAQ; MetaFilter |
| Borders | Black letterbox bars (unoptimised PAL) | None | Neowin; https://www.resetera.com/threads/help-determining-which-ps2-pal-releases-are-optimized-for-50hz.1324030/ |
| PAL gameplay side effect | Wakka's Attack Reels timer won't finish at the "2 Hit" slot | n/a | FFW |
| Internal rate | 30 fps NTSC / 25 fps PAL, logic tied to framerate [user consensus, uncertain] | **Capped at 30 fps**, logic at 30 Hz; some menus run at 60; capping at 30 externally slows loads | PCGW; SK; https://steamcommunity.com/app/359870/discussions/0/364041517005647198/ |
| 60+ fps | — | Only via mods: NX/363 (interpolated extra frame, logic stays at 30), NX/365 (60–240 fps ReShade add-on), NX/353 (experimental) | NX/363, 365, 353 |
| Loading | Disc | Faster loads; occasional stuck loads (UnX "KickStart") | FFW; SK |

## 11. Controls and prompts

| Difference | PS2 | HD PC | Source |
|---|---|---|---|
| Confirm/cancel | ✕ confirm / ○ cancel (non-JP) | Same layout | FFW |
| Prompts | PS2 glyphs | Xbox-style glyphs without A/B/X/Y colours; UnX `TextureSet` = PlayStation_Glossy/PS3/PS4/Xbox360/XboxOne; NX/278 ports these to EFL | FFW; PCGW; SK; NX/278 |
| Remap | No | Keyboard and pad remap | FFW |
| Lulu's Fury (analog spin) | Works | **Often unresponsive on PC** (unresolved); NX/357 bypasses it | PCGW; NX/357 |
| L3/R3 bug | — | L3/R3 "globally killed inputs" until the Sep 2026 update [user claim] | https://steamcommunity.com/app/359870/discussions/0/581683835681481848/ |

## 12. Logic and content (keep "PC logic")

| Difference | PS2 PAL (International) | HD PC | Source |
|---|---|---|---|
| Base content | Intl.: Dark Aeons, Penance, Expert grid, new abilities, rebalanced fiends | Same International base; game balance explicitly unchanged | FFW; FAM |
| Eternal Calm | Not on PAL disc (EU got it on the Unlimited Saga CE DVD) | Included, plus *Last Mission* and *-Will-* audio drama | WP; FFW |
| Bonus disc | *Beyond Final Fantasy* DVD | n/a | WP |
| Glitches fixed in PAL | Home glitch fixed; Dark Valefor crash fixed | — | FFW |
| Fenrir | Free | Fighting Fenrir now costs gil | FFW |
| Text changes | — | "Stolen Fayth Cavern", "Flower Sceptre", "not up to scratch" etc. (EN); German changes not catalogued. One unverified claim: the German ending line changed from "thank you" to "I love you" [uncertain] | FFW; search summary of Steam 600774870011216270 |
| RNG | Seeded/deterministic | PS4 1.01 says "truly random" battle RNG; the PC RNG is still trackable (speedrunners; the Fahrenheit True RNG mod exists to make it untrackable) [uncertain for PC] | FFW; https://github.com/fahrenheit-crew/fh-mods-truerng |
| Tidus line in Baaj | Normal | "Mysteries & oddities…" can play randomly during a fade-out [user claim] | Steam 600774870011216270 |
| Censorship | — | **None documented** | https://steamcommunity.com/app/359870/discussions/0/600774870011216270/ |
| Minigames | — | No documented rule changes found; achievements added (Lightning Dodger etc.) [uncertain] | FFW (nothing listed) |

## 13. PC-only features

| Feature | Detail | Source |
|---|---|---|
| Auto-save | Separate slot written on area change | PCGW; FFW |
| Boosters (**at launch in May 2016**, not added by a 2019 update) | F1 Turbo 2x/4x (BGM only); F2 Supercharge (full HP/MP/OD each turn); F3 Encounters normal/many/none; F4 Auto-battle (attacks the right-most target; Yojimbo paid 1 gil); F5 Hide HUD | PCGW; FFW; https://steamcommunity.com/app/359870/discussions/0/364040961452411626/ |
| Parameter changes (Esc menu, permanent) | 99 of every item, all skills, max gil (999,999,999). There is **no max-stats booster** | PCGW; FFW |
| Graphics options | Resolution up to 4K, AA, AF, shadows, HDAO, colour correction, unsharp mask | PCGW |
| FMV skip | Partial | PCGW |
| 2019 updates | 3 Mar 2019 update added a Steam DRM requirement that broke offline play; reverted 27 Mar 2019 | PCGW; https://www.pcgamesn.com/final-fantasy-xx-2-hd-remaster/steam-drm |
| Sep/Oct 2026 update (build 25501027) | Official notes: "Several bugs fixed". Users report new exes, **Large Address Aware** (4GB patch no longer needed), L3/R3 fix, and that **UnX/Special K and older mods broke**; the claim that Switch 2 features were added is unverified | https://steamdb.info/patchnotes/25501027/ ; Steam 581683835681481848 |
| Switch 2 (Jul 2026) | First console version with high-speed and no-encounter options; mentioned for context only | https://gameinformer.com/2026/06/09/final-fantasy-xx-2-hd-remaster-hits-nintendo-switch-2-next-month-with-high-speed-mode-no |

---

## 14. Restoration and modding toolchain (what's feasible)

### 14.1 Frameworks and loaders

| Tool | What it does | Notes | Source |
|---|---|---|---|
| **UnX ("Untitled" Project X), Kaldaien**: plugin `unx.dll` on top of Special K's `dxgi.dll` | Texture **dump and injection** (`UnX_Res/inject/textures/<hash>.dds`, `Inject=true`; legacy-hash `UnX_Old/` folder), texture reload; audio language split (Voice/SFX/FMV each jp/us); gamepad glyph sets; button remap and F1–F5 pad combos; **SpeedBoost 1/2/4/8x for FFX, with audio; 8x skips cutscenes**; time stop; **free-look camera** (Ctrl+Shift+F); quicksave; AP to inactive members; permanent Sensor; play as Seymour; soft reset; stuck-load KickStart; borderless flip / FSE, DPI fix, LAA patch via installer; ReShade plugin; Special K shader tools (block individual shaders, as in NX/358) | **Doesn't change aspect ratio or framerate** (don't cap at 30). Steam 2021 update crashes with SK's Steam features. **Reported broken by the Sep 2026 Steam update.** Releases are now only at https://github.com/Kaldaien/UnX/releases | SK; UNXV; wccftech; https://github.com/Kaldaien/UnX ; Steam 581683835681481848; https://www.nexusmods.com/finalfantasyxx2hdremaster/mods/150?tab=posts |
| **FFX/X-2 HD External File Loader (EFL), ffgriever** | `dinput8.dll` module loader plus a loose-file override from `data/mods/ffx_data/...` instead of the VBF archives; compatible with UnX in principle | Source: https://gitlab.com/ffgriever/ffx-x-2-hd-external-file-loader | NX/150 |
| **Fahrenheit** | RE framework: hooks game functions and hosts .NET 10 mods (x86); has its own file loader; works with Roelin's converter for model and texture mods; currently alpha (~alpha12) | **Incompatible with UnX and ffgriever's EFL.** The ffx-vrr-mod built on it was tested on build 25501027 | https://github.com/fahrenheit-crew/fahrenheit ; https://github.com/Exoridus/ffx-vrr-mod |
| UNX→EFL converter (NX/293) | Converts UnX hash-named DDS files to EFL `*.dds.phyre` paths (hash↔path tables for every FFX texture) | Useful as a hash-to-path map | NX/293 |
| FFXII Asset Converter (Roelin) | `.dae.phyre` ↔ FBX, `.dds.phyre` ↔ PNG; works inside VBF | FFX models only partly supported | https://www.nexusmods.com/finalfantasy12/mods/288 |
| FFX Phyre Tool (NfgOdin, NX/315) | GUI wrapper: phyre ↔ glTF 2.0 for Blender; uses FFXIIConvert, **Noesis**, texconv/Compressonator | — | https://github.com/NfgOdin/FFX-Phyre-Tool |
| VBF Browser (VaanSkyPirate), DDS Phyre tool (ffgriever) | Archive browsing; dds ↔ dds.phyre | — | NX/168; https://www.nexusmods.com/finalfantasy12/mods/289 |
| FFXDataParser (Karifean) | Script/room dumps (used for the Farplane fix and the voice-line work) | — | NX/355; NX/361 |
| Spira Mod Manager (NX/327), Spira Modifier (NX/339), Cheat Engine tables (NX/184, NX/186) | Mod management, data editing, memory tweaks | — | Nexus API listing |

### 14.2 Mods that restore PS2 look or sound

| Mod | What it restores | How | Source |
|---|---|---|---|
| Restore PS2 Tidus / Yuna / Wakka (Impulserina) | **Original PS2 models**, direct port | Model conversion (Roelin converter) via EFL; physics/transparency issues; incompatible with HD texture mods; makes save spheres partly invisible | NX/163, NX/162, NX/165 |
| Restore PS2 *X* "but Fully Refined" (vyacharin): Tidus, Yuna, Rikku, Lulu, Wakka, Auron, Seymour; also FFX-2 Yuna | PS2 head and chest on the HD body, with upscaled textures; HP and LP variants (LP versions can't blink); optional PS2 or HD hair | EFL model replacement; Kimahri, Wedding Yuna and Wedding Seymour are missing | NX/227–233, NX/237; RE |
| PS2 Original Face (Tidus/Yuna), PS2 Like Face Textures, Tidus Old Face, Tidus best HD (ps2 like), Better faces – Tidus, Tidus Face Mix | PS2-like **face textures** on HD models (texture only) | UnX injection | NX/143, 47, 46, 45, 7, 211 |
| FFX PS2 UI Overhaul (randomherogames, 2026) | **PS2 menus, fonts and icons** (mostly English); optional "Final Fantasy X Project" title | UnX injection; some vertex-drawn submenus can't be changed | NX/348 |
| PS2 Portraits | PS2 menu portraits (2x upscaled) | UnX | NX/41 |
| PS2 Style Pause Text | PS2 pause text | EFL (`pausemenu.swf`) or UnX | NX/107 |
| Simple Load Screen | Removes "Loading" text | EFL/UnX | NX/108 |
| Clean Opening Logo | Removes "HD Remaster" from the logo, as on PS2 | UnX (`52D2838D.dds`, `F95B4E0C.dds`) | NX/261 |
| OG Aspect Ratio FMV Restoration Project | **Uncropped 4:3 PS2 FMVs**, AI-upscaled to 4K (Topaz Starlight) | EFL `.../ps3data/video`; needs the 4GB patch (green-FMV fix) | NX/343 |
| FMVs Remastered v2.0 | Cleaned-up AI upscale of the HD FMVs (still 16:9) | EFL | NX/121 |
| Disable Glow and Blur around light sources | Removes the HD bloom/blowout for a PS2 feel | Special K/UnX shader blocking, VS `7e29e6ac` "never draw" | NX/358 |
| PS2-Inspired ReShade / PS2-like ReShade | Adds DOF, bloom and AO to approximate the PS2 look | ReShade preset | NX/241, NX/316 |
| Battle Voice Line Restoration | Restores about 90 lines that never played (on PS2 either) | `system_01.bin` script fixes | NX/361 |
| No more battle menu delay | PS2-like battle menu responsiveness | ASI/DLL or hex patch | NX/360 |
| Farplane Fix | Farplane background video that never played on PC | exe patch | NX/355 |
| 60 fps mods | Smoother motion with logic kept at 30 Hz | dbghelp.dll proxy / ReShade add-on | NX/363, NX/365 |
| **Original soundtrack** | Built-in Original/Arranged switch; **no dedicated "PS2-exact OST" mod found** | — | PCGW |
| **Original lighting / backgrounds / 4:3 gameplay** | **No mod found** | — | https://steamcommunity.com/app/359870/discussions/0/4423183923848625777 (search summary); Steam 600787749709037968 |

Other notable texture packs (not PS2 restoration): Better Texture (Waifu2x upscale of NPCs and monsters, NX/3), Catachrism 4K/8K character packs (NX/32 etc.), Re-Remastered all-graphics AI upscale (NX/244), high-poly characters (NX/226), HD Remaster – Ultra Remaster (NX/168).

### 14.3 Feasibility notes for "PS2 look, PC logic"
- **Textures and UI:** proven feasible through UnX hash injection or EFL `.dds.phyre` overrides. EFL is the more future-proof route since UnX broke after the 2026 update (user reports). Vertex-drawn menu gradients need code or data changes.
- **Models:** proven feasible. PS2 meshes can be converted into `.dae.phyre` (Roelin converter, Noesis, Blender). Physics and blink limits apply on LP models.
- **FMVs:** proven feasible as 4:3 replacements (NX/343).
- **Post-processing:** individual HD shaders can be suppressed through Special K's shader blocking (NX/358). Recreating the PS2 grade needs ReShade or shader replacement.
- **4:3 gameplay:** no existing mod. The engine has separate indoor and outdoor aspect paths and widened 2D backgrounds (FAM, PCGW). SUWSF-style memory patches show that aspect values can be patched.
- **Audio:** the Original OST is built in. Differences in PS2-specific variants (e.g. the BFA "Otherworld") would need audio replacement through EFL.

Raw API dumps (Nexus mod list and descriptions, wiki wikitext) are saved in `~/ffx-work/demaster/research/dl/`.
