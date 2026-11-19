//! Audio cue domain for Floor 01 (AUDIO-01, ADR-0008).
//!
//! Pure, headless data and rules: no Bevy audio types, no device, no clock of
//! its own. Gameplay events become IP-neutral [`CueId`]s, music and ambience
//! state changes become [`AudioCommand`]s, and [`CueBank`] maps both to files
//! through data (`assets/audio/floor01/cue_map.json` plus the pack's
//! `audio_events.json` and `asset_manifest.json`). The Bevy adapter
//! (`audio_adapter`) plays what this module resolves.
//!
//! Presentation only: nothing here feeds back into the simulation. The variant
//! picker uses [`PresentationRng`], never the gameplay RNG, and none of it is
//! hashed or replayed.

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;

use crate::combat::CombatEvent;

/// Directory of the Floor 01 pack, relative to the repository root.
pub const PACK_DIR: &str = "assets/audio/floor01";

// ------------------------------------------------------------------- cue ids --

/// A one-shot sound the game can ask for. IP-neutral on purpose (ADR-0004):
/// the fan-layer event ids live only in `cue_map.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CueId {
    /// The player's melee move started, hit or miss. The greybox moves (jab,
    /// haymaker, push kick) all use the kick whoosh until punch audio exists.
    PlayerKickSwing,
    /// A strike was verified as a hit (a `CombatEvent::Hit`), never a swing.
    PlayerKickHitConfirmed,
    /// An enemy took damage (nonverbal placeholder in the current pack).
    EnemyHitReaction,
    /// An enemy body hit another enemy or a prop hard enough to count.
    EnemyBodyCollision,
    /// A destructible wall broke.
    WallBreak,
    /// A heavy prop (for example gym weights) crashed.
    HeavyPropCrash,
    /// A warning siren (future hook, no gameplay event emits it yet).
    Warning,
}

impl CueId {
    pub const ALL: [CueId; 7] = [
        CueId::PlayerKickSwing,
        CueId::PlayerKickHitConfirmed,
        CueId::EnemyHitReaction,
        CueId::EnemyBodyCollision,
        CueId::WallBreak,
        CueId::HeavyPropCrash,
        CueId::Warning,
    ];

    /// Key used in `cue_map.json`.
    pub fn name(self) -> &'static str {
        match self {
            Self::PlayerKickSwing => "PlayerKickSwing",
            Self::PlayerKickHitConfirmed => "PlayerKickHitConfirmed",
            Self::EnemyHitReaction => "EnemyHitReaction",
            Self::EnemyBodyCollision => "EnemyBodyCollision",
            Self::WallBreak => "WallBreak",
            Self::HeavyPropCrash => "HeavyPropCrash",
            Self::Warning => "Warning",
        }
    }

    /// Loud contact transients. At most [`Mixer::max_major_contacts`] may
    /// overlap, so a pile-up never turns into a wall of noise.
    pub fn is_major_contact(self) -> bool {
        matches!(
            self,
            Self::PlayerKickHitConfirmed
                | Self::EnemyBodyCollision
                | Self::WallBreak
                | Self::HeavyPropCrash
        )
    }

    /// Cues that briefly duck the music bus.
    pub fn ducks_music(self) -> bool {
        matches!(self, Self::PlayerKickHitConfirmed)
    }
}

/// Looping music state. Each maps to one looped track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MusicState {
    /// Floor scene start (intro, tension).
    Tension,
    /// A fight is running (the combat fixtures).
    Combat,
    /// A pursuit (future hook).
    Chase,
    /// The gym section (future hook).
    Gym,
}

impl MusicState {
    pub const ALL: [MusicState; 4] = [Self::Tension, Self::Combat, Self::Chase, Self::Gym];

    pub fn name(self) -> &'static str {
        match self {
            Self::Tension => "Tension",
            Self::Combat => "Combat",
            Self::Chase => "Chase",
            Self::Gym => "Gym",
        }
    }
}

/// Looping ambience bed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AmbienceState {
    Ventilation,
    Gym,
}

impl AmbienceState {
    pub const ALL: [AmbienceState; 2] = [Self::Ventilation, Self::Gym];

    pub fn name(self) -> &'static str {
        match self {
            Self::Ventilation => "Ventilation",
            Self::Gym => "Gym",
        }
    }
}

/// What the adapter is asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCommand {
    Cue(CueId),
    Music(MusicState),
    Ambience(AmbienceState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bus {
    Music,
    Ambience,
    Sfx,
}

/// Sound-family bank for the hit-confirm impacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleBank {
    Arcade,
    Cinematic,
    Hybrid,
}

impl StyleBank {
    pub fn name(self) -> &'static str {
        match self {
            Self::Arcade => "arcade",
            Self::Cinematic => "cinematic",
            Self::Hybrid => "hybrid",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "arcade" => Some(Self::Arcade),
            "cinematic" => Some(Self::Cinematic),
            "hybrid" => Some(Self::Hybrid),
            _ => None,
        }
    }

    /// The next bank in the A/B/C order, for the debug key.
    pub fn next(self) -> Self {
        match self {
            Self::Arcade => Self::Cinematic,
            Self::Cinematic => Self::Hybrid,
            Self::Hybrid => Self::Arcade,
        }
    }
}

// ------------------------------------------------- gameplay event -> cues --

/// Cues for one combat event. Misses count: `MoveStarted` fires on every swing.
/// Only `Hit`, which `resolve_strike` emits for a verified overlap, produces the
/// hit-confirm and the enemy reaction.
pub fn cues_from_combat_event(event: &CombatEvent) -> Vec<CueId> {
    match event {
        CombatEvent::MoveStarted { .. } => vec![CueId::PlayerKickSwing],
        CombatEvent::Hit { .. } => vec![CueId::PlayerKickHitConfirmed, CueId::EnemyHitReaction],
        // DEMO-01: existing cues only, so the shore, the block and the
        // goblin-on-goblin contact are not silent.
        CombatEvent::GoblinContact { .. } => vec![CueId::EnemyBodyCollision],
        CombatEvent::ShoreBroken { .. } => vec![CueId::WallBreak],
        CombatEvent::ShoreLanded { .. } => vec![CueId::HeavyPropCrash],
        _ => Vec::new(),
    }
}

/// What touched what in a physical contact (a future physics hook).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactKind {
    EnemyEnemy,
    EnemyProp,
    /// A heavy prop landing or crashing (gym weights).
    HeavyProp,
}

/// A physical contact with its strength (any consistent unit; the threshold is
/// [`MAJOR_CONTACT_MIN_IMPULSE`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContactEvent {
    pub kind: ContactKind,
    pub impulse: f32,
}

/// Contacts weaker than this make no sound.
pub const MAJOR_CONTACT_MIN_IMPULSE: f32 = 1.0;

/// Cues for one contact. The greybox has no enemy/prop physics yet, so nothing
/// emits [`ContactEvent`]s at runtime; the contract is fixed here and tested.
pub fn cues_from_contact(contact: &ContactEvent) -> Vec<CueId> {
    if contact.impulse.is_nan() || contact.impulse < MAJOR_CONTACT_MIN_IMPULSE {
        return Vec::new();
    }
    match contact.kind {
        ContactKind::EnemyEnemy | ContactKind::EnemyProp => vec![CueId::EnemyBodyCollision],
        ContactKind::HeavyProp => vec![CueId::HeavyPropCrash],
    }
}

/// Cues for `count` destructible blocks broken in one tick: one crash, however
/// many blocks fell at once.
pub fn cues_from_wall_break(count: u32) -> Vec<CueId> {
    if count == 0 {
        Vec::new()
    } else {
        vec![CueId::WallBreak]
    }
}

/// Music for a scene: the fight track when the scene has a combat fixture, the
/// tension track otherwise.
pub fn scene_music(has_combat: bool) -> MusicState {
    if has_combat {
        MusicState::Combat
    } else {
        MusicState::Tension
    }
}

/// Ambience for the corridor scene (the vent bed under everything).
pub fn scene_ambience() -> AmbienceState {
    AmbienceState::Ventilation
}

// -------------------------------------------------------------- settings --

/// Runtime settings, from defaults, then environment, then flags.
///
/// Environment: `FLOORBREAK_AUDIO` (`off`/`0`/`false`/`mute` disables),
/// `FLOORBREAK_AUDIO_STYLE` (`arcade`/`cinematic`/`hybrid`),
/// `FLOORBREAK_VOLUME_{MASTER,MUSIC,AMBIENCE,SFX}` (linear, 0 to 2) and
/// `FLOORBREAK_AUDIO_SEED`.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioSettings {
    pub enabled: bool,
    pub style: StyleBank,
    pub master: f32,
    pub music: f32,
    pub ambience: f32,
    pub sfx: f32,
    /// Seed of the presentation-only variant RNG.
    pub seed: u64,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            style: StyleBank::Hybrid,
            master: 1.0,
            music: 0.8,
            ambience: 0.5,
            sfx: 1.0,
            seed: 0x0A0D_1001,
        }
    }
}

impl AudioSettings {
    /// Defaults overridden by whatever `get` returns (an env lookup in the game,
    /// a map in tests). Unparsable values are ignored.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        let mut s = Self::default();
        if let Some(v) = get("FLOORBREAK_AUDIO") {
            let v = v.trim().to_ascii_lowercase();
            if matches!(v.as_str(), "off" | "0" | "false" | "mute" | "none") {
                s.enabled = false;
            }
        }
        if let Some(style) = get("FLOORBREAK_AUDIO_STYLE").and_then(|v| StyleBank::parse(&v)) {
            s.style = style;
        }
        let volume = |key: &str| {
            get(key)
                .and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .map(|v| v.clamp(0.0, 2.0))
        };
        if let Some(v) = volume("FLOORBREAK_VOLUME_MASTER") {
            s.master = v;
        }
        if let Some(v) = volume("FLOORBREAK_VOLUME_MUSIC") {
            s.music = v;
        }
        if let Some(v) = volume("FLOORBREAK_VOLUME_AMBIENCE") {
            s.ambience = v;
        }
        if let Some(v) = volume("FLOORBREAK_VOLUME_SFX") {
            s.sfx = v;
        }
        if let Some(seed) = get("FLOORBREAK_AUDIO_SEED").and_then(|v| v.trim().parse().ok()) {
            s.seed = seed;
        }
        s
    }

    pub fn from_env() -> Self {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Linear gain of a bus including the master.
    pub fn bus_gain(&self, bus: Bus) -> f32 {
        self.master
            * match bus {
                Bus::Music => self.music,
                Bus::Ambience => self.ambience,
                Bus::Sfx => self.sfx,
            }
    }
}

// ------------------------------------------------------------- cue bank --

#[derive(Debug)]
pub enum BankError {
    Io(String),
    Parse(String),
    Missing(String),
}

impl fmt::Display for BankError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(m) | Self::Parse(m) | Self::Missing(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for BankError {}

#[derive(Debug, Deserialize)]
struct EventsFile {
    mixer: MixerFile,
    events: Vec<EventDef>,
}

#[derive(Debug, Deserialize)]
struct MixerFile {
    default_variant: String,
    gain_fade_on_big_hit_db: f32,
    duck_ms: u32,
    max_concurrent_major_contacts_initial: usize,
}

#[derive(Debug, Deserialize)]
struct EventDef {
    id: String,
    bus: String,
    #[serde(default)]
    assets: Vec<String>,
    #[serde(default)]
    style_banks: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    assets: Vec<ManifestAsset>,
}

#[derive(Debug, Deserialize)]
struct ManifestAsset {
    path: String,
    duration_s: f32,
}

#[derive(Debug, Deserialize)]
struct CueMapFile {
    cues: HashMap<String, String>,
    music: HashMap<String, String>,
    ambience: HashMap<String, String>,
}

/// The pack's mixer section, clamped to the brief (duck 100 to 180 ms).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mixer {
    pub default_style: StyleBank,
    /// Music duck depth on a hit-confirm, dB (negative).
    pub duck_db: f32,
    pub duck_seconds: f32,
    pub max_major_contacts: usize,
}

impl Mixer {
    /// Linear gain of the duck.
    pub fn duck_gain(&self) -> f32 {
        10f32.powf(self.duck_db / 20.0)
    }
}

/// A music track plus the one-shots that start with it (the chase engine).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicPlan {
    pub looped: String,
    pub one_shots: Vec<String>,
}

/// Cue and state names resolved to pack-relative file paths.
#[derive(Debug)]
pub struct CueBank {
    pub mixer: Mixer,
    events: HashMap<String, EventDef>,
    cues: HashMap<CueId, String>,
    music: HashMap<MusicState, String>,
    ambience: HashMap<AmbienceState, String>,
    durations: HashMap<String, f32>,
}

impl CueBank {
    /// Load the pack at `pack_dir` (normally [`PACK_DIR`] under the repo root).
    pub fn load(pack_dir: &Path) -> Result<Self, BankError> {
        let read = |rel: &str| {
            std::fs::read_to_string(pack_dir.join(rel))
                .map_err(|e| BankError::Io(format!("cannot read {rel}: {e}")))
        };
        Self::from_json(
            &read("docs/audio_events.json")?,
            &read("docs/asset_manifest.json")?,
            &read("cue_map.json")?,
        )
    }

    pub fn from_json(events: &str, manifest: &str, cue_map: &str) -> Result<Self, BankError> {
        let parse = |e: serde_json::Error| BankError::Parse(e.to_string());
        let events: EventsFile = serde_json::from_str(events).map_err(parse)?;
        let manifest: ManifestFile = serde_json::from_str(manifest).map_err(parse)?;
        let map: CueMapFile = serde_json::from_str(cue_map).map_err(parse)?;
        let default_style = StyleBank::parse(&events.mixer.default_variant)
            .ok_or_else(|| BankError::Parse("unknown default_variant".into()))?;
        let mixer = Mixer {
            default_style,
            duck_db: events.mixer.gain_fade_on_big_hit_db.min(0.0),
            duck_seconds: events.mixer.duck_ms.clamp(100, 180) as f32 / 1000.0,
            max_major_contacts: events.mixer.max_concurrent_major_contacts_initial.max(1),
        };
        let events: HashMap<String, EventDef> = events
            .events
            .into_iter()
            .map(|e| (e.id.clone(), e))
            .collect();
        let lookup = |table: &HashMap<String, String>, name: &str| {
            let id = table
                .get(name)
                .ok_or_else(|| BankError::Missing(format!("cue_map has no entry for {name}")))?;
            if events.contains_key(id) {
                Ok(id.clone())
            } else {
                Err(BankError::Missing(format!(
                    "cue_map entry {name} points at unknown event {id}"
                )))
            }
        };
        let mut cues = HashMap::new();
        for cue in CueId::ALL {
            cues.insert(cue, lookup(&map.cues, cue.name())?);
        }
        let mut music = HashMap::new();
        for state in MusicState::ALL {
            music.insert(state, lookup(&map.music, state.name())?);
        }
        let mut ambience = HashMap::new();
        for state in AmbienceState::ALL {
            ambience.insert(state, lookup(&map.ambience, state.name())?);
        }
        let durations = manifest
            .assets
            .into_iter()
            .map(|a| (a.path, a.duration_s))
            .collect();
        Ok(Self {
            mixer,
            events,
            cues,
            music,
            ambience,
            durations,
        })
    }

    fn event_assets(&self, id: &str, style: StyleBank) -> &[String] {
        let Some(event) = self.events.get(id) else {
            return &[];
        };
        if event.style_banks.is_empty() {
            return &event.assets;
        }
        event
            .style_banks
            .get(style.name())
            .or_else(|| event.style_banks.get(self.mixer.default_style.name()))
            .map_or(&[][..], Vec::as_slice)
    }

    /// The takes of a cue for a style bank (pack-relative paths).
    pub fn cue_assets(&self, cue: CueId, style: StyleBank) -> &[String] {
        self.event_assets(&self.cues[&cue], style)
    }

    /// The loop of a music state and the one-shots that start with it.
    pub fn music_plan(&self, state: MusicState) -> Option<MusicPlan> {
        let assets = self.event_assets(&self.music[&state], self.mixer.default_style);
        let (looped, rest) = assets.split_first()?;
        Some(MusicPlan {
            looped: looped.clone(),
            one_shots: rest.to_vec(),
        })
    }

    pub fn ambience_asset(&self, state: AmbienceState) -> Option<&str> {
        self.event_assets(&self.ambience[&state], self.mixer.default_style)
            .first()
            .map(String::as_str)
    }

    /// Bus of a cue's event as written in the pack.
    pub fn cue_bus(&self, cue: CueId) -> Option<&str> {
        self.events.get(&self.cues[&cue]).map(|e| e.bus.as_str())
    }

    /// Every file any cue, state or style bank can play, sorted and unique.
    pub fn referenced_assets(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for id in self
            .cues
            .values()
            .chain(self.music.values())
            .chain(self.ambience.values())
        {
            let event = &self.events[id];
            out.extend(event.assets.iter().cloned());
            out.extend(event.style_banks.values().flatten().cloned());
        }
        out.sort();
        out.dedup();
        out
    }

    /// Duration from the manifest, 0.5 s when unknown.
    pub fn duration(&self, path: &str) -> f32 {
        self.durations.get(path).copied().unwrap_or(0.5)
    }
}

// ----------------------------------------------------- presentation rules --

/// Presentation-only RNG (xorshift64*). Never the gameplay RNG.
#[derive(Debug, Clone)]
pub struct PresentationRng(u64);

impl PresentationRng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A take index in `0..len`, never the same as `last` when `len > 1`.
    pub fn pick_variant(&mut self, len: usize, last: Option<usize>) -> usize {
        if len <= 1 {
            return 0;
        }
        match last.filter(|l| *l < len) {
            Some(last) => {
                let n = (self.next_u64() % (len as u64 - 1)) as usize;
                if n >= last { n + 1 } else { n }
            }
            None => (self.next_u64() % len as u64) as usize,
        }
    }
}

/// Caps overlapping major contact transients (time is the caller's clock, in
/// seconds).
#[derive(Debug, Clone)]
pub struct VoiceLimiter {
    max: usize,
    ends: Vec<f32>,
}

impl VoiceLimiter {
    pub fn new(max: usize) -> Self {
        Self {
            max: max.max(1),
            ends: Vec::new(),
        }
    }

    /// Reserve a voice for `duration` seconds from `now`; false when full.
    pub fn try_acquire(&mut self, now: f32, duration: f32) -> bool {
        self.ends.retain(|end| *end > now);
        if self.ends.len() >= self.max {
            return false;
        }
        self.ends.push(now + duration.max(0.0));
        true
    }

    pub fn active(&self, now: f32) -> usize {
        self.ends.iter().filter(|end| **end > now).count()
    }
}

/// Short music duck: full depth for the duration, then a quick linear release.
#[derive(Debug, Clone)]
pub struct Ducker {
    gain: f32,
    remaining: f32,
}

/// Release ramp at the end of a duck, seconds.
const DUCK_RELEASE: f32 = 0.04;

impl Ducker {
    pub fn new(gain: f32) -> Self {
        Self {
            gain: gain.clamp(0.0, 1.0),
            remaining: 0.0,
        }
    }

    /// Start (or extend) a duck of `seconds`.
    pub fn trigger(&mut self, seconds: f32) {
        self.remaining = self.remaining.max(seconds);
    }

    pub fn tick(&mut self, dt: f32) {
        self.remaining = (self.remaining - dt).max(0.0);
    }

    /// Linear music gain right now: 1.0 idle, the duck gain while ducked.
    pub fn gain(&self) -> f32 {
        if self.remaining <= 0.0 {
            1.0
        } else if self.remaining >= DUCK_RELEASE {
            self.gain
        } else {
            self.gain + (1.0 - self.gain) * (1.0 - self.remaining / DUCK_RELEASE)
        }
    }
}

// ------------------------------------------------------------------ tests --

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::EnemyAi;
    use crate::combat::{CombatEvent, MoveId};
    use crate::combat_scene::{EnemySpawn, FIXTURES, Fixture, FixtureSim};
    use crate::env_kit::EnvKit;
    use std::path::PathBuf;

    fn hit() -> CombatEvent {
        CombatEvent::Hit {
            tick: 1,
            move_id: MoveId::PushKick,
            instance: 1,
            target: 0,
            damage: 1,
            health_left: 1,
            hit_stop: 0,
        }
    }

    #[test]
    fn a_swing_is_a_swing_cue_hit_or_miss() {
        let start = CombatEvent::MoveStarted {
            tick: 1,
            move_id: MoveId::PushKick,
            instance: 1,
        };
        assert_eq!(cues_from_combat_event(&start), vec![CueId::PlayerKickSwing]);
    }

    #[test]
    fn only_a_verified_hit_confirms_and_reacts() {
        assert_eq!(
            cues_from_combat_event(&hit()),
            vec![CueId::PlayerKickHitConfirmed, CueId::EnemyHitReaction]
        );
        for quiet in [
            CombatEvent::GoblinTell { tick: 1, goblin: 0 },
            CombatEvent::GoblinSwing { tick: 1, goblin: 0 },
            CombatEvent::GoblinInterrupted { tick: 1, goblin: 0 },
            CombatEvent::HeroDefeated {
                tick: 1,
                hero: crate::party::HeroId::Carl,
            },
            CombatEvent::Restart { tick: 1, deaths: 1 },
        ] {
            assert!(cues_from_combat_event(&quiet).is_empty(), "{quiet:?}");
        }
    }

    /// DEMO-01: the shore, the falling block and a goblin knocked into another
    /// reuse the existing crash cues (no new sound, no new file).
    #[test]
    fn demo_events_reuse_the_existing_crash_cues() {
        assert_eq!(
            cues_from_combat_event(&CombatEvent::GoblinContact {
                tick: 1,
                launch: 1,
                source: 0,
                target: 1,
            }),
            vec![CueId::EnemyBodyCollision]
        );
        assert_eq!(
            cues_from_combat_event(&CombatEvent::ShoreBroken { tick: 1, player: 0 }),
            vec![CueId::WallBreak]
        );
        assert_eq!(
            cues_from_combat_event(&CombatEvent::ShoreLanded { tick: 1 }),
            vec![CueId::HeavyPropCrash]
        );
        assert!(
            cues_from_combat_event(&CombatEvent::ShoreHit {
                tick: 1,
                instance: 1,
                hits: 1
            })
            .is_empty()
        );
        assert!(
            cues_from_combat_event(&CombatEvent::Respawn {
                tick: 1,
                checkpoint: 1
            })
            .is_empty()
        );
    }

    #[test]
    fn enemy_contacts_and_heavy_props_map_to_their_cues() {
        let c = |kind, impulse| cues_from_contact(&ContactEvent { kind, impulse });
        assert_eq!(
            c(ContactKind::EnemyEnemy, 2.0),
            vec![CueId::EnemyBodyCollision]
        );
        assert_eq!(
            c(ContactKind::EnemyProp, 1.0),
            vec![CueId::EnemyBodyCollision]
        );
        assert_eq!(c(ContactKind::HeavyProp, 3.0), vec![CueId::HeavyPropCrash]);
        assert!(
            c(ContactKind::EnemyEnemy, 0.2).is_empty(),
            "a graze is silent"
        );
        assert!(c(ContactKind::EnemyEnemy, f32::NAN).is_empty());
    }

    #[test]
    fn many_blocks_in_one_tick_make_one_wall_break() {
        assert!(cues_from_wall_break(0).is_empty());
        assert_eq!(cues_from_wall_break(1), vec![CueId::WallBreak]);
        assert_eq!(cues_from_wall_break(5), vec![CueId::WallBreak]);
    }

    #[test]
    fn a_real_bomb_breaking_a_timber_reports_the_wall_cue() {
        let mut world = crate::prototype::World::new();
        let before = world.intact_timbers();
        let (x, y) = world
            .blocks
            .iter()
            .find(|b| b.intact && b.material.destructible())
            .expect("the stage has a timber barrier")
            .rect
            .center();
        world.bombs.push(crate::prototype::Bomb { x, y, fuse: 1 });
        world.step(&Default::default());
        assert!(world.intact_timbers() < before);
        assert_eq!(
            cues_from_wall_break(world.timbers_broken_last_step),
            vec![CueId::WallBreak]
        );
        world.step(&Default::default());
        assert_eq!(world.timbers_broken_last_step, 0, "one cue per break");
    }

    #[test]
    fn every_fixture_emits_a_swing_per_move_and_a_confirm_only_per_real_hit() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(&root, "assets/levels/floor01/corridor_abc_sample.json").unwrap();
        let mut total_hits = 0;
        for fixture in FIXTURES {
            let mut sim = FixtureSim::new(&kit, 360, fixture).unwrap();
            sim.run_scripted(fixture.ticks);
            let events = sim.combat.events.clone();
            let cues: Vec<CueId> = events.iter().flat_map(cues_from_combat_event).collect();
            let starts = events
                .iter()
                .filter(|e| matches!(e, CombatEvent::MoveStarted { .. }))
                .count();
            let hits = events
                .iter()
                .filter(|e| matches!(e, CombatEvent::Hit { .. }))
                .count();
            let count = |c| cues.iter().filter(|x| **x == c).count();
            assert_eq!(count(CueId::PlayerKickSwing), starts, "{}", fixture.name);
            assert_eq!(
                count(CueId::PlayerKickHitConfirmed),
                hits,
                "{}",
                fixture.name
            );
            assert_eq!(count(CueId::EnemyHitReaction), hits, "{}", fixture.name);
            total_hits += hits;
        }
        assert!(total_hits > 0, "some fixture lands a hit");
    }

    /// A stationary goblin at column `x` (Carl at column 400), no script.
    macro_rules! dummy {
        ($name:ident, $x:expr) => {
            static $name: Fixture = Fixture {
                name: "audio_test_dummy",
                summary: "test",
                seed: 7,
                carl_x: 400,
                enemies: &[EnemySpawn {
                    x: $x,
                    ai: EnemyAi::Passive,
                }],
                shore_dx: None,
                script: &[],
                ticks: 100,
            };
        };
    }
    dummy!(IN_REACH, 428);
    dummy!(OUT_OF_REACH, 560);

    /// Run one whole push kick (press, then every tick of the move) against a
    /// stationary goblin and return the combat events.
    fn whole_kick(fixture: &'static Fixture) -> Vec<CombatEvent> {
        use crate::combat::{MoveId, carl_move};
        use crate::input::RawInput;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(&root, "assets/levels/floor01/corridor_abc_sample.json").unwrap();
        let mut sim = FixtureSim::new(&kit, 360, fixture).unwrap();
        let total = carl_move(MoveId::PushKick).unwrap().total();
        sim.step(RawInput {
            physical: true,
            ..Default::default()
        });
        for _ in 0..total {
            sim.step(RawInput::default());
        }
        sim.combat.events.clone()
    }

    fn count_of(events: &[CombatEvent], cue: CueId) -> usize {
        events
            .iter()
            .flat_map(cues_from_combat_event)
            .filter(|c| *c == cue)
            .count()
    }

    #[test]
    fn a_whole_kick_out_of_reach_swings_without_a_confirm_and_in_reach_confirms_once() {
        // The same move, once against a goblin in reach and once against one 160
        // px away: the hit/miss split comes from `resolve_strike`, not from the
        // cue mapping.
        let hits = |e: &[CombatEvent]| {
            e.iter()
                .filter(|e| matches!(e, CombatEvent::Hit { .. }))
                .count()
        };
        let hit = whole_kick(&IN_REACH);
        let miss = whole_kick(&OUT_OF_REACH);
        assert_eq!(hits(&miss), 0, "the kick out of reach hits nothing");
        assert_eq!(count_of(&miss, CueId::PlayerKickSwing), 1);
        assert_eq!(count_of(&miss, CueId::PlayerKickHitConfirmed), 0);
        assert_eq!(count_of(&miss, CueId::EnemyHitReaction), 0);
        assert_eq!(hits(&hit), 1, "the kick in reach lands exactly once");
        assert_eq!(count_of(&hit, CueId::PlayerKickSwing), 1);
        assert_eq!(count_of(&hit, CueId::PlayerKickHitConfirmed), 1);
        assert_eq!(count_of(&hit, CueId::EnemyHitReaction), 1);
    }

    #[test]
    fn scene_state_picks_the_fight_or_the_tension_track() {
        assert_eq!(scene_music(true), MusicState::Combat);
        assert_eq!(scene_music(false), MusicState::Tension);
        assert_eq!(scene_ambience(), AmbienceState::Ventilation);
    }

    fn settings(pairs: &[(&str, &str)]) -> AudioSettings {
        AudioSettings::from_lookup(|k| {
            pairs
                .iter()
                .find(|(key, _)| *key == k)
                .map(|(_, v)| v.to_string())
        })
    }

    #[test]
    fn settings_default_to_hybrid_and_take_env_overrides() {
        let d = settings(&[]);
        assert!(d.enabled);
        assert_eq!(d.style, StyleBank::Hybrid);
        let s = settings(&[
            ("FLOORBREAK_AUDIO_STYLE", "Arcade"),
            ("FLOORBREAK_VOLUME_MUSIC", "0.25"),
            ("FLOORBREAK_VOLUME_MASTER", "0.5"),
            ("FLOORBREAK_VOLUME_SFX", "nonsense"),
        ]);
        assert_eq!(s.style, StyleBank::Arcade);
        assert!((s.bus_gain(Bus::Music) - 0.125).abs() < 1e-6);
        assert!((s.sfx - 1.0).abs() < 1e-6, "bad value ignored");
        assert!(!settings(&[("FLOORBREAK_AUDIO", "off")]).enabled);
        let loud = settings(&[("FLOORBREAK_VOLUME_SFX", "99")]);
        assert!((loud.sfx - 2.0).abs() < 1e-6, "clamped");
    }

    #[test]
    fn style_banks_cycle_arcade_cinematic_hybrid() {
        assert_eq!(StyleBank::Arcade.next(), StyleBank::Cinematic);
        assert_eq!(StyleBank::Cinematic.next(), StyleBank::Hybrid);
        assert_eq!(StyleBank::Hybrid.next(), StyleBank::Arcade);
        assert_eq!(StyleBank::parse("nope"), None);
    }

    #[test]
    fn rng_is_deterministic_and_never_repeats_the_last_take() {
        let mut a = PresentationRng::new(7);
        let mut b = PresentationRng::new(7);
        let mut last = None;
        for _ in 0..200 {
            let x = a.pick_variant(3, last);
            assert_eq!(x, b.pick_variant(3, last));
            assert!(x < 3 && Some(x) != last);
            last = Some(x);
        }
        assert_eq!(a.pick_variant(1, Some(0)), 0);
    }

    #[test]
    fn at_most_two_major_contacts_overlap() {
        let mut v = VoiceLimiter::new(2);
        assert!(v.try_acquire(0.0, 0.3));
        assert!(v.try_acquire(0.05, 0.3));
        assert!(
            !v.try_acquire(0.1, 0.3),
            "third overlapping contact dropped"
        );
        assert_eq!(v.active(0.1), 2);
        assert!(v.try_acquire(0.31, 0.3), "first one finished");
    }

    #[test]
    fn the_duck_is_about_three_db_for_a_short_while() {
        let mut d = Ducker::new(10f32.powf(-3.2 / 20.0));
        assert_eq!(d.gain(), 1.0);
        d.trigger(0.15);
        assert!((d.gain() - 0.692).abs() < 0.01);
        d.tick(0.1);
        assert!((d.gain() - 0.692).abs() < 0.01, "still ducked at 100 ms");
        d.tick(0.03);
        assert!(d.gain() > 0.692 && d.gain() < 1.0, "releasing");
        d.tick(0.1);
        assert_eq!(d.gain(), 1.0);
    }
}
