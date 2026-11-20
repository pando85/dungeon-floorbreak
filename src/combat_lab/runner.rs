//! The headless runner: [`LabCatalog::launch`] builds a [`LabRun`] over the
//! real [`CombatState`], [`PartyState`] and [`SceneSim`] path, and
//! [`LabSession`] adds the room selection and restart on top of it.
//!
//! A [`LabRun`] steps exactly like the interactive scene and
//! [`crate::party_scene::PartyFixtureSim`] do: the solo swap, the party's
//! input routing, the scene drive and the combat step, in that order, one raw
//! input per player slot per tick. Nothing here owns a rule, an RNG or a
//! clock: a run is a pure function of its [`LabConfig`] and its inputs, so
//! the same inputs give the same hash twice.
//!
//! **Seed (LAB-Q06).** The seed of a run is the CF02-Q58 effective seed:
//! `--cf02-seed` if given, else the descriptor's fixture seed
//! ([`effective_seed`]). On main the seed is mixed into the state hash and
//! nothing else reads it yet, so two seeds give two hashes but the same play
//! until a rule (CF02-G temperaments) consumes it.

use std::path::{Path, PathBuf};

use super::descriptor::{
    EnemySpec, GoalKind, Manifest, Resolved, RoomSpec, validate_manifest, validate_roster,
};
use super::entry::{EntryPoint, denied};
use super::profile::{CapabilityProfile, PROFILE_ID};
use super::validate::{ValidationReport, load_room, read_room_file, validate_all};
use super::{CF02_RULES_INTEGRATED, LabError, MANIFEST_PATH};
use crate::cf02::Cf02Config;
use crate::combat_lab_fixtures::{FixtureKey, intern_fixture};
use crate::combat_scene::{CombatState, Fixture};
use crate::corridor::SceneSim;
use crate::donut_mana::EmptyPress;
use crate::env_kit::EnvKit;
use crate::input::SlotInputs;
use crate::party::{PROVISIONAL_TUNING, PadId, PartyState};
use crate::party_scene::party_hash;

/// Who drives which hero (the modes of every room).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerMode {
    /// Carl human, Donut AI (the follower bot).
    SoloCarl,
    /// Donut human, Carl AI (the follower bot).
    SoloDonut,
    /// P1 Carl and P2 Donut, two independent pads.
    CoopLocal,
}

impl PlayerMode {
    pub const ALL: [PlayerMode; 3] = [Self::SoloCarl, Self::SoloDonut, Self::CoopLocal];

    pub const fn id(self) -> &'static str {
        match self {
            Self::SoloCarl => "solo_carl",
            Self::SoloDonut => "solo_donut",
            Self::CoopLocal => "coop_local",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.id() == id)
    }

    /// The declared policy of the AI seat (`None` in two-player). Today's
    /// uncontrolled hero is the legacy follow-and-jump bot of `follower.rs`.
    pub const fn ai_policy(self) -> Option<&'static str> {
        match self {
            Self::SoloCarl | Self::SoloDonut => Some("follow_and_jump_bot_v0"),
            Self::CoopLocal => None,
        }
    }
}

/// Probe tags: a tagged probe is never natural player demonstration data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeTag {
    Natural,
    /// Direct room select with a configured roster.
    Sandbox,
    /// A density probe: the concurrency cap of LAB-Q04 is lifted.
    Stress,
}

impl ProbeTag {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Natural => "natural",
            Self::Sandbox => "sandbox",
            Self::Stress => "stress",
        }
    }
}

/// What a launch needs. Every field is a fixed-list choice (R00's selection):
/// there is no difficulty, no per-ability toggle and no item.
#[derive(Debug, Clone, PartialEq)]
pub struct LabConfig {
    pub room: String,
    pub variant: String,
    pub mode: PlayerMode,
    pub profile: String,
    /// `rules` stays off until CF02-I; `seed` is the Q58 override.
    pub cf02: Cf02Config,
    /// 360 or 400: the scene camera view height.
    pub view_h: i32,
    /// Index into the room's checkpoints (direct entry at a checkpoint).
    pub checkpoint: usize,
    pub probe: ProbeTag,
    /// A configured roster, for a sandbox or stress probe only.
    pub enemies_override: Option<Vec<EnemySpec>>,
}

impl LabConfig {
    /// A natural run of `room` in `mode`: the baseline profile, variant v1,
    /// no CF02 rules, the 640x360 view, the room start.
    pub fn new(room: &str, mode: PlayerMode) -> Self {
        Self {
            room: room.to_owned(),
            variant: "v1".to_owned(),
            mode,
            profile: PROFILE_ID.to_owned(),
            cf02: Cf02Config::OFF,
            view_h: 360,
            checkpoint: 0,
            probe: ProbeTag::Natural,
            enemies_override: None,
        }
    }

    /// The Setup Cage's choice, from fixed lists (names as the menu shows
    /// them): unknown names are rejected, not guessed.
    pub fn from_selection(
        catalog: &LabCatalog,
        profile: &str,
        mode: &str,
        room: &str,
        variant: &str,
    ) -> Result<Self, LabError> {
        catalog.profile(profile)?;
        let mode = PlayerMode::parse(mode).ok_or_else(|| LabError::Unknown {
            what: "player mode".into(),
            id: mode.into(),
        })?;
        let spec = catalog.room(room)?;
        if !spec.variant_ids().contains(&variant) && spec.link.is_none() {
            return Err(LabError::Unknown {
                what: format!("variant of {room}"),
                id: variant.into(),
            });
        }
        let mut cfg = Self::new(room, mode);
        cfg.profile = profile.to_owned();
        cfg.variant = variant.to_owned();
        Ok(cfg)
    }
}

/// The CF02-Q58 effective seed: the override if given, else the fixture seed.
pub fn effective_seed(cf02: Cf02Config, fixture_seed: u64) -> u64 {
    cf02.seed.unwrap_or(fixture_seed)
}

/// The fixed lists of R00.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupOptions {
    pub profiles: Vec<String>,
    pub modes: Vec<PlayerMode>,
    /// `(room id, variants)`; the linked room lists none.
    pub rooms: Vec<(String, Vec<String>)>,
}

/// The loaded and validated lab: manifest, profiles and the validation
/// reports (with their explicit pendings).
#[derive(Debug, Clone)]
pub struct LabCatalog {
    root: PathBuf,
    manifest: Manifest,
    profiles: Vec<CapabilityProfile>,
    reports: Vec<ValidationReport>,
}

impl LabCatalog {
    /// Load `assets/levels/combat_lab/manifest.json` under `root` and validate
    /// the descriptor, every profile and every room.
    pub fn load(root: &Path) -> Result<Self, LabError> {
        let path = root.join(MANIFEST_PATH);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| LabError::Io(format!("{}: {e}", path.display())))?;
        Self::from_manifest_text(root, &text)
    }

    /// [`Self::load`] for a manifest text (the rejection tests edit one).
    pub fn from_manifest_text(root: &Path, text: &str) -> Result<Self, LabError> {
        let manifest = Manifest::parse(text)?;
        let profiles = validate_manifest(&manifest)?;
        let reports = validate_all(root, &manifest)?;
        Ok(Self {
            root: root.to_owned(),
            manifest,
            profiles,
            reports,
        })
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn rooms(&self) -> &[RoomSpec] {
        &self.manifest.rooms
    }

    pub fn room(&self, id: &str) -> Result<&RoomSpec, LabError> {
        self.manifest
            .rooms
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| LabError::Unknown {
                what: "room".into(),
                id: id.into(),
            })
    }

    pub fn profile(&self, id: &str) -> Result<&CapabilityProfile, LabError> {
        self.profiles
            .iter()
            .find(|p| p.id() == id)
            .ok_or_else(|| LabError::Unknown {
                what: "capability profile".into(),
                id: id.into(),
            })
    }

    pub fn reports(&self) -> &[ValidationReport] {
        &self.reports
    }

    pub fn report(&self, room: &str, variant: &str) -> Option<&ValidationReport> {
        self.reports
            .iter()
            .find(|r| r.room == room && r.variant == variant)
    }

    /// The fixed lists R00 offers.
    pub fn setup_options(&self) -> SetupOptions {
        SetupOptions {
            profiles: self.profiles.iter().map(|p| p.id().to_owned()).collect(),
            modes: PlayerMode::ALL.to_vec(),
            rooms: self
                .manifest
                .rooms
                .iter()
                .map(|r| {
                    let variants = if r.link.is_some() {
                        Vec::new()
                    } else {
                        r.variant_ids().into_iter().map(str::to_owned).collect()
                    };
                    (r.id.clone(), variants)
                })
                .collect(),
        }
    }

    /// Start a run. `entry` is the leak guard: only [`EntryPoint::CombatLab`]
    /// gets a lab run (LAB-Q01).
    pub fn launch(&self, entry: EntryPoint, cfg: &LabConfig) -> Result<LabRun, LabError> {
        if !entry.may_load_lab_profile() {
            return Err(denied(entry, &cfg.profile));
        }
        let profile = self.profile(&cfg.profile)?;
        if cfg.cf02.rules && !CF02_RULES_INTEGRATED {
            return Err(LabError::Cf02NotIntegrated);
        }
        let room = self.room(&cfg.room)?;
        if let Some(link) = &room.link {
            return Err(LabError::Linked {
                room: room.id.clone(),
                scene: link.scene.clone(),
                blockers: link.blockers.clone(),
            });
        }
        let not_allowed = |reason: String| LabError::NotAllowed {
            room: room.id.clone(),
            reason,
        };
        if !room.profiles.contains(&cfg.profile) {
            return Err(not_allowed(format!(
                "profile {} is not legal here",
                cfg.profile
            )));
        }
        if !room.allowed_modes.iter().any(|m| m == cfg.mode.id()) {
            return Err(not_allowed(format!(
                "mode {} is not allowed",
                cfg.mode.id()
            )));
        }
        if cfg.view_h != 360 && cfg.view_h != 400 {
            return Err(not_allowed("view_h must be 360 or 400".into()));
        }
        if cfg.enemies_override.is_some() && cfg.probe == ProbeTag::Natural {
            return Err(not_allowed(
                "a configured roster needs a sandbox or stress probe tag".into(),
            ));
        }
        let resolved = room.resolve(&cfg.variant)?;
        let carl_x = *room.checkpoints.get(cfg.checkpoint).ok_or_else(|| {
            not_allowed(format!(
                "checkpoint {} (the room has {})",
                cfg.checkpoint,
                room.checkpoints.len()
            ))
        })?;
        let enemies: &[EnemySpec] = cfg.enemies_override.as_deref().unwrap_or(resolved.enemies);
        validate_roster(
            &format!("room {} roster", room.id),
            enemies,
            room.max_concurrent,
            cfg.probe == ProbeTag::Stress,
        )?;

        // The geometry, re-checked against the manifest hash at launch.
        let text = read_room_file(&self.root, resolved.room_file)?;
        let actual = super::fnv64(text.as_bytes());
        if actual != resolved.layout_hash {
            return Err(LabError::HashMismatch {
                what: format!("room file {}", resolved.room_file),
                expected: format!("{:016x}", resolved.layout_hash),
                actual: format!("{actual:016x}"),
            });
        }
        let loaded = load_room(&self.root, &text, room.goal.kind, &room.id)?;
        let kit = loaded.kit;
        for e in enemies {
            if kit.walking_line_at(e.x) != Some(kit.base_walk_line_y) {
                return Err(LabError::Geometry {
                    room: room.id.clone(),
                    reason: format!("{} at column {} is not on the walking line", e.kind, e.x),
                });
            }
        }

        let seed = effective_seed(cfg.cf02, resolved.seed);
        let fixture = build_fixture(room, &resolved, enemies, carl_x, seed, cfg);
        let info = RunInfo {
            room: room.id.clone(),
            room_name: room.name.clone(),
            variant: cfg.variant.clone(),
            mode: cfg.mode,
            profile: cfg.profile.clone(),
            probe: cfg.probe,
            cf02: cfg.cf02,
            seed,
            layout_hash: resolved.layout_hash,
            config_hash: resolved.config_hash(),
            content_hash: resolved.content_hash(),
            view_h: cfg.view_h,
            carl_x,
            goal: room.goal.kind,
            segment: room.segment.id.clone(),
            encounter: room.encounter_id.clone(),
            default_ticks: room.ticks,
            moves: profile
                .effective_moves(cfg.cf02.rules)
                .into_iter()
                .map(str::to_owned)
                .collect(),
            fixture,
            donut_start_mana: profile.spec().donut_start_mana,
        };
        let (scene, combat, party) = build_state(&kit, &info)?;
        Ok(LabRun {
            kit,
            scene,
            combat,
            party,
            info,
        })
    }
}

fn build_fixture(
    room: &RoomSpec,
    resolved: &Resolved<'_>,
    enemies: &[EnemySpec],
    carl_x: i32,
    seed: u64,
    cfg: &LabConfig,
) -> &'static Fixture {
    let split = |kind: &str| -> Vec<(i32, bool)> {
        enemies
            .iter()
            .filter(|e| e.kind == kind)
            .map(|e| (e.x, e.ai == "passive"))
            .collect()
    };
    intern_fixture(FixtureKey {
        name: format!("lab_{}_{}", room.id.to_lowercase(), resolved.variant),
        seed,
        carl_x,
        goblins: split("goblin"),
        llamas: split("llama"),
        shore_dx: room.hazards.first().map(|h| h.x - carl_x),
        ticks: if cfg.probe == ProbeTag::Natural {
            room.ticks
        } else {
            room.ticks.max(1)
        },
    })
}

/// The scene, the combat state and the party of a run, fresh at tick 0.
fn build_state(
    kit: &EnvKit,
    info: &RunInfo,
) -> Result<(SceneSim, CombatState, PartyState), LabError> {
    let geometry = |reason: String| LabError::Geometry {
        room: info.room.clone(),
        reason,
    };
    let mut scene = SceneSim::new_at(kit, info.view_h, info.carl_x).map_err(geometry)?;
    let setup = crate::donut_mana::DonutSetup::unlocked(info.donut_start_mana);
    let combat = CombatState::new(info.fixture, kit, &scene)
        .map_err(geometry)?
        .with_donut(setup);
    // "No swat": at 0 mana the press is dry, never a swat.
    if combat.donut_mana_tuning().empty_press != EmptyPress::Dry {
        return Err(LabError::IllegalAbility {
            at: "the Donut mana tuning".into(),
            id: "swat".into(),
            reason: "the empty press must be dry in the post-biscuit lab profile".into(),
        });
    }
    let mut party = PartyState::new(PROVISIONAL_TUNING);
    if info.mode == PlayerMode::CoopLocal {
        party.pad_connected(PadId(1));
        party.pad_start_pressed(PadId(2));
    }
    scene.set_drive(party.drive(true));
    seat(&mut party, info.mode);
    scene.set_drive(party.drive(true));
    Ok((scene, combat, party))
}

/// Put the human on the mode's hero (the party restarts with Carl).
fn seat(party: &mut PartyState, mode: PlayerMode) {
    if mode == PlayerMode::SoloDonut {
        party.request_swap(true, false);
    }
}

/// What a run is, for logs, the recorder's manifest and the overlay.
#[derive(Debug, Clone)]
pub struct RunInfo {
    pub room: String,
    pub room_name: String,
    pub variant: String,
    pub mode: PlayerMode,
    pub profile: String,
    pub probe: ProbeTag,
    pub cf02: Cf02Config,
    /// The CF02-Q58 effective seed.
    pub seed: u64,
    pub layout_hash: u64,
    pub config_hash: u64,
    pub content_hash: u64,
    pub view_h: i32,
    /// Carl's start column (the room start or a checkpoint).
    pub carl_x: i32,
    pub goal: GoalKind,
    /// The recording segment id of the room.
    pub segment: String,
    pub encounter: String,
    pub default_ticks: u64,
    /// The moves this run has (CF02 moves only under `cf02_rules`).
    pub moves: Vec<String>,
    pub fixture: &'static Fixture,
    donut_start_mana: u8,
}

/// A running room. The public parts are what an adapter drives and draws.
#[derive(Debug, Clone)]
pub struct LabRun {
    pub kit: EnvKit,
    pub scene: SceneSim,
    pub combat: CombatState,
    pub party: PartyState,
    pub info: RunInfo,
}

impl LabRun {
    /// One tick with explicit slot inputs: the swap, the party routing, the
    /// scene drive and the combat step, the order of the interactive scene.
    pub fn step_slots(&mut self, slots: SlotInputs) {
        let hurt = self.combat.hurt_ticks_of(self.party.controlled_hero()) > 0;
        self.party.request_swap(slots[0].swap, hurt);
        let inputs = self.party.hero_inputs(slots);
        self.scene.set_drive(self.party.drive(true));
        let restarts = self.combat.restarts;
        self.combat.step_party(&mut self.scene, inputs);
        if self.combat.restarts != restarts {
            // The party's defeat retry: back to the mode's seat.
            self.party.restart();
            seat(&mut self.party, self.info.mode);
            self.scene.set_drive(self.party.drive(true));
        }
    }

    /// Step a [`crate::combat_lab_fixtures::Probe`] script for `ticks` ticks.
    pub fn run_probe(&mut self, probe: &crate::combat_lab_fixtures::Probe, ticks: u64) {
        for _ in 0..ticks {
            let tick = self.scene.world.tick + 1;
            self.step_slots(probe.slot_inputs(tick));
        }
    }

    /// Room restart: a fresh state from the same descriptor, seed and start
    /// column (the hash equals the launch's). Never touches a save.
    pub fn restart(&mut self) -> Result<(), LabError> {
        let (scene, combat, party) = build_state(&self.kit, &self.info)?;
        self.scene = scene;
        self.combat = combat;
        self.party = party;
        Ok(())
    }

    /// The combat hash plus the party's swap timers: the value the goldens pin.
    pub fn state_hash(&self) -> u64 {
        party_hash(&self.combat, &self.scene, &self.party)
    }

    /// The tick the scene reports (attempt-local).
    pub fn tick(&self) -> u64 {
        self.scene.world.tick
    }

    /// Whether the room's goal is met (R00 and the linked room have none).
    pub fn goal_met(&self) -> bool {
        match self.info.goal {
            GoalKind::ClearEnemies => crate::demo::complete(&self.combat),
            GoalKind::ReachExit => self.scene.world.complete,
            GoalKind::None | GoalKind::Link => false,
        }
    }

    /// The line a headless replay and CI compare.
    pub fn state_line(&self) -> String {
        let i = &self.info;
        format!(
            "LAB_STATE room={} variant={} mode={} profile={} probe={} seed={} tick={} controlled={:?} carl_hp={} donut_hp={} hash={:016x}",
            i.room,
            i.variant,
            i.mode.id(),
            i.profile,
            i.probe.id(),
            i.seed,
            self.tick(),
            self.party.controlled_hero(),
            self.scene.world.hero.health,
            self.scene.world.donut.health,
            self.state_hash(),
        )
    }
}

/// How an episode ended (the recording segment boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpisodeEnd {
    Clear,
    Retry,
    RoomChange,
    Abandon,
}

/// One finished episode of a session. Every retry is an episode of the same
/// session and is kept, successful or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeRecord {
    pub episode: u32,
    pub room: String,
    pub variant: String,
    pub mode: PlayerMode,
    pub segment: String,
    /// Ticks simulated in the episode (restarts inside it included).
    pub ticks: u64,
    pub deaths: u32,
    pub end: EpisodeEnd,
    pub hash: u64,
}

/// A lab session: the room selection and restart on top of [`LabRun`]. Multiple
/// rooms are one session but separate measurable episodes; nothing here reads
/// or writes a campaign save.
#[derive(Debug, Clone)]
pub struct LabSession {
    catalog: LabCatalog,
    entry: EntryPoint,
    cfg: LabConfig,
    run: LabRun,
    episode: u32,
    history: Vec<EpisodeRecord>,
}

impl LabSession {
    pub fn start(catalog: LabCatalog, entry: EntryPoint, cfg: LabConfig) -> Result<Self, LabError> {
        let run = catalog.launch(entry, &cfg)?;
        Ok(Self {
            catalog,
            entry,
            cfg,
            run,
            episode: 1,
            history: Vec::new(),
        })
    }

    pub fn run(&self) -> &LabRun {
        &self.run
    }

    pub fn run_mut(&mut self) -> &mut LabRun {
        &mut self.run
    }

    pub fn config(&self) -> &LabConfig {
        &self.cfg
    }

    pub fn catalog(&self) -> &LabCatalog {
        &self.catalog
    }

    /// The number of the running episode (1-based).
    pub fn episode(&self) -> u32 {
        self.episode
    }

    /// The episodes that ended, oldest first.
    pub fn history(&self) -> &[EpisodeRecord] {
        &self.history
    }

    fn end_episode(&mut self, end: EpisodeEnd) {
        let end = if self.run.goal_met() {
            EpisodeEnd::Clear
        } else {
            end
        };
        self.history.push(EpisodeRecord {
            episode: self.episode,
            room: self.run.info.room.clone(),
            variant: self.run.info.variant.clone(),
            mode: self.run.info.mode,
            segment: self.run.info.segment.clone(),
            ticks: self.run.combat.elapsed,
            deaths: self.run.combat.deaths,
            end,
            hash: self.run.state_hash(),
        });
        self.episode += 1;
    }

    /// Restart the room (retry): the same room, variant, seed and mode.
    pub fn restart(&mut self) -> Result<(), LabError> {
        let fresh = self.catalog.launch(self.entry, &self.cfg)?;
        self.end_episode(EpisodeEnd::Retry);
        self.run = fresh;
        Ok(())
    }

    /// Reconfigure (another room, variant, mode or checkpoint) at a safe
    /// boundary. On an error the session is unchanged.
    pub fn reconfigure(&mut self, change: impl FnOnce(&mut LabConfig)) -> Result<(), LabError> {
        let mut cfg = self.cfg.clone();
        change(&mut cfg);
        let fresh = self.catalog.launch(self.entry, &cfg)?;
        self.end_episode(EpisodeEnd::RoomChange);
        self.cfg = cfg;
        self.run = fresh;
        Ok(())
    }

    /// Direct room entry: the room's first variant at its start.
    pub fn select_room(&mut self, room: &str) -> Result<(), LabError> {
        self.reconfigure(|cfg| {
            cfg.room = room.to_owned();
            cfg.variant = "v1".to_owned();
            cfg.checkpoint = 0;
            cfg.enemies_override = None;
        })
    }

    pub fn select_variant(&mut self, variant: &str) -> Result<(), LabError> {
        self.reconfigure(|cfg| cfg.variant = variant.to_owned())
    }

    /// Leave the running episode (the tester quits the room); the same room
    /// starts again as a new episode.
    pub fn abandon(&mut self) -> Result<(), LabError> {
        let fresh = self.catalog.launch(self.entry, &self.cfg)?;
        self.end_episode(EpisodeEnd::Abandon);
        self.run = fresh;
        Ok(())
    }
}
