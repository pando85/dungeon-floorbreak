//! Floor 1 corridor scene: COMP-01 integration plus the COMP-02 lighting pass
//! and the ANIM-INT-01 locomotion animation.
//!
//! Renders the ENV-01 kit layout with the approved Carl 52 px and Donut 40 px
//! locomotion atlases (idle, run, jump, land; the deterministic state machine
//! lives in `anim` / `scene_anim`, this binary only maps its state to an atlas
//! cell). Pixel layers stay at the virtual resolution; the A1 lighting pass
//! (ambient, torch pools, shadow, haze, glow, flame bloom) runs at output
//! resolution (`light_pass`) and is presentation only. `--no-lighting` draws
//! the COMP-01 look. The normal route has no enemies and no new mechanics; the
//! COMBAT-INT-01 `--fixture` scenarios add Carl's melee and placeholder goblins
//! on clean ground (see `combat_scene`). Simulation,
//! camera and collision come from the pure library modules; this binary only
//! reads input, then draws that state. Character art is fan-concept material
//! and does not imply game distribution rights.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use bevy::{
    app::AppExit,
    asset::AssetPlugin,
    asset::LoadState,
    asset::RenderAssetUsages,
    camera::{RenderTarget, ScalingMode, Viewport, visibility::RenderLayers},
    diagnostic::DiagnosticsStore,
    input::gamepad::{GamepadConnection, GamepadEvent},
    prelude::*,
    render::{
        diagnostic::RenderDiagnosticsPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::screenshot::{Screenshot, save_to_disk},
    },
    sprite::Anchor,
    window::{PresentMode, PrimaryWindow, WindowResolution},
};
use dungeon_floorbreak::{
    actor_sprite::{
        ActorAssets, ActorSpec, FrameSource, actor_center, load_carl, load_donut, shadow_center_x,
    },
    anim::{Animator, Tag},
    audience::{AnticipationCue, AnticipationTrigger, ChainFeat},
    audio::{
        AudioCommand, AudioSettings, StyleBank, cues_from_combat_event,
        cues_from_combat_event_cf02, scene_ambience, scene_music,
    },
    audio_adapter::{AudioQueue, FloorAudioPlugin, resolve_pack_dir},
    blast::BLAST_CENTRE_DY,
    blast_art::{BURST_IMAGE, BlastArt, DYNAMITE_IMAGE},
    boss::{BossMode, RackState},
    boss_art::{self, BossArt, FxCtx, Gait, Layer, SheetId},
    boss_scene,
    camera_lab::camera_op::{Aspect, Bounds, View},
    carl_prog::CarlProgFlags,
    carl_prog_fixtures,
    cf02::{self, Cf02Config, Cf02Tuning},
    cf02_fixtures::{find_party_fixture, list_fixtures},
    cine::{ENV_CINEMATIC_FINISHES, FrameContext, PlannerState, PresentationSettings},
    cine_adapter::CineAdapter,
    combat::{Attacker, ENEMY_HALF_HEIGHT, ENEMY_HALF_WIDTH, Phase, hit_box},
    combat_anim::{
        CombatAnimators, GOBLIN_VARIANTS, SpriteClips, carl_above_goblin, cf02::ActingArt,
        goblin_variant,
    },
    combat_lab::{
        self, EntryPoint, LabCatalog, LabConfig, LabRun,
        flow::{self as lab_flow, ExitState, LabSelection, NextRoom},
    },
    combat_lab_fixtures::PENDING_FIXTURE,
    combat_lab_overlay::{EndPanel, LabOverlay, LabOverlayPlugin, LabRequest, RecStatus},
    combat_scene::{CombatState, Fixture},
    companion::profile::ProfileSetId,
    corridor::{CorridorWorld, Scenario, SceneSim},
    death_present::{
        DeathPresenter, DrawLayer, FLOOR_DECAL_POOL_CAPACITY, FLOOR_DECAL_POOL_SLOTS,
        PART_POOL_CAPACITY, PART_POOL_SLOTS, WALL_DECAL_POOL_CAPACITY, WALL_DECAL_POOL_SLOTS,
        blast_vfx_frame, charge_draw_frame, draw_z, explosion_table,
    },
    demo::{self, SelectAction, SelectControl},
    donut_art, donut_fixtures,
    donut_mana::{DonutSetup, StartState},
    env_kit::{
        EnvKit, IRect, PieceDraw, animation_frame, parallax_copies, parallax_start, to_world_y,
    },
    feel::{
        HERO_HURT_TICKS, IMPACT_FLASH_TICKS, LLAMA_GLOB_MOUTH_DX, LLAMA_GLOB_MOUTH_DY,
        LLAMA_GLOB_SPEED,
    },
    goblin::{AttackPhase, GoblinMode},
    gore::{DEATH_VARIANTS, DeathArt, GorePreset, LLAMA_VARIANT, Sheet},
    hud::{EventCursor, GlyphSet, HudConfig, HudLayout, HudPreview, HudRuntime},
    hud_adapter::{HudPlugin, HudState},
    hud_settings::HudSettings,
    input::RawInput,
    light_pass::{
        LAYER_DEBUG, LightPassPlugin, LightRig, LightingMaterial, PassFrame, SceneCamera,
        slot_layer, spawn_rig, update_rig,
    },
    lighting::{MAX_SHADOWS, ShadowEllipse, ViewRect, contact_shadow},
    llama::LlamaMode,
    pad_slots::{PadButtons, PadEvent, PadFrame, PadSlots},
    party::{HeroId, PROVISIONAL_TUNING, PadId, PartyState, PartyTuning, UncontrolledHero},
    party_scene::{self, PartyFixture},
    playtrace::{self, RECORD_DEFAULT_ON, ResetDescriptor, SessionManifest, live::LiveRecorder},
    prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Hero, TICK_SECONDS},
    scene_anim::SceneAnimators,
    scene_camera::{
        CameraChoice, CameraInterest, RigConfig, SceneCameraRig, llama_interest,
        synthetic_interest, test_room_kit,
    },
    shore_art::{ShoreArt, ShoreSlot, landed_age},
    story_unlock::{SceneKind, StorySetup},
};

// PTR-R1. A crate-root file looks for child modules beside itself, so the path is explicit.
#[path = "floor01_scene/replay_view.rs"]
mod replay_view;

/// Draw order above the actor plane. Carl is drawn above every goblin, except
/// that a goblin in its smear or impact is drawn over him (its club lands on
/// him) while he is not in his own active window; during Carl's active frames he
/// is above his target (the melee spec's integration rule: his fist overlaps the
/// goblin's head by up to 6 px). Lying goblins (down, corpse) sit below Donut and everything else.
const CARL_Z_OFFSET: f32 = 0.1;
const GOBLIN_Z_OFFSET: f32 = 0.05;
const GOBLIN_SWING_Z_OFFSET: f32 = 0.15;
const GOBLIN_LYING_Z_OFFSET: f32 = -0.01;
/// The falling block over a crushed llama (LLAMA-02 gate ruling, data
/// `hazard_draw: "over"`): above every lying body (-0.01 and below), below
/// every player actor (Donut at 0, Carl at +0.1).
const HAZARD_OVER_Z_OFFSET: f32 = -0.005;
/// The spit glob is drawn above the actors (it hits Carl).
const GLOB_Z_OFFSET: f32 = 0.2;
/// Pre-spawned glob sprites per llama (one glob flies at a time).
const GLOB_SLOTS_PER_LLAMA: usize = 2;
/// Per-goblin z step so two goblins never share a depth: the earlier goblin is
/// in front (a stacked later one is drawn behind it).
const GOBLIN_Z_STEP: f32 = 0.001;
/// Impact flash (presentation): the sprite is drawn this much brighter for the
/// flash ticks after a hit.
const FLASH_TINT: Color = Color::linear_rgb(2.0, 2.0, 2.0);

const LAYOUT_PATH: &str = "assets/levels/floor01/corridor_abc_sample.json";
const VIRTUAL_WIDTH: i32 = 640;
const WATCHDOG_SECONDS: f32 = 10.0;
const TITLE: &str = "THE CRAWL - Floor 1 Corridor";

#[derive(Debug, Clone)]
struct Args {
    deck: bool,
    headless: bool,
    freeze: bool,
    debug_collision: bool,
    scenario: Option<Scenario>,
    /// COMBAT-INT-01 combat fixture (mutually exclusive with `--scenario`).
    fixture: Option<&'static Fixture>,
    /// WORK-005/006 party fixture (headless replay or a `--capture-tick`
    /// capture): its base combat fixture is also stored in `fixture`.
    party_fixture: Option<&'static PartyFixture>,
    /// Fixture only: the keyboard or pad drives Carl instead of the script.
    manual: bool,
    /// Fixture only: draw hit and hurt boxes (debug overlay).
    debug_combat: bool,
    capture_tick: Option<u64>,
    scale: u32,
    /// Draw without the lighting pass (the COMP-01 look).
    no_lighting: bool,
    /// Review capture: camera and actors pinned to the ENV-01 lit preview frame.
    reference_frame: bool,
    /// Measure this many frame intervals after warm-up, print them and exit.
    perf_frames: Option<usize>,
    /// DEATH-E1, fixture only (with `--perf-frames`): pre-simulate this many
    /// ticks, then step exactly one tick per measured frame (instead of the
    /// real-time fixed step), so every frame covers the same tick on every run
    /// and the probe sees the same deaths whatever the frame time.
    perf_from_tick: Option<u64>,
    /// Live run: step the scripted scenario for this many fixed ticks in the
    /// real render loop, print `FLOOR1_SCENE_STATE` and exit.
    exit_after_ticks: Option<u64>,
    /// Review capture sequence `first:step:count`: pre-simulate `first` ticks
    /// of the scenario, then save `count` window screenshots `step` ticks
    /// apart into `capture_dir` and exit.
    capture_sequence: Option<(u64, u64, u64)>,
    capture_dir: Option<PathBuf>,
    /// Headless only: print one `FLOOR1_ANIM_TRACE` line per simulated tick of a
    /// scenario, or one `COMBAT_ANIM_TRACE` line per tick of a fixture.
    anim_trace: bool,
    /// Fixture only, headless: one `COMBAT_TRACE` row per simulated tick.
    combat_trace: bool,
    /// No audio device and no sound (AUDIO-01); `FLOORBREAK_AUDIO=off` does the same.
    no_audio: bool,
    /// Style bank override (`arcade`, `cinematic`, `hybrid`); F9 cycles it live.
    audio_style: Option<StyleBank>,
    /// DEATH-C: the gore preset 1 to 4 (presentation only; default 4, full).
    /// It never reaches the simulation: every replay hash is the same at any
    /// preset.
    gore_preset: GorePreset,
    /// Fixture only, lab: keep the death presenter's pools loaded with
    /// synthetic defeats (frame-time measurement at max load).
    death_stress: bool,
    /// Headless fixture only: print the presenter's `DEATH_PRESENT` summary.
    death_trace: bool,
    /// HUD-HYBRID-A-01: draw a labelled HUD preview fixture. Presentation
    /// only: it never reaches the simulation, the combat state or a replay.
    hud_preview: Option<HudPreview>,
    /// HUD glyph set override (`--glyphs=xbox|dualsense`). `None` follows the
    /// last-used gamepad (Xbox until one is seen).
    glyphs: Option<GlyphSet>,
    /// HUD-DEBT-06: reduced UI motion and high contrast. Environment first
    /// (`FLOORBREAK_HUD_*`), then `--hud-reduced-motion[=on|off]` and
    /// `--hud-high-contrast[=on|off]`. Presentation only.
    hud_settings: HudSettings,
    /// CINE-01: `cinematic_finishes` on or off. Environment first
    /// (`FLOORBREAK_CINEMATIC_FINISHES`), then `--cinematic-finishes[=on|off]`.
    /// `cinematic_set` records that either one spoke: when neither did, the
    /// effective setting is on for live play and off for scripted fixtures,
    /// scenarios and captures ([`Args::cinematic_settings`]). Presentation
    /// only.
    cinematic: PresentationSettings,
    cinematic_set: bool,
    /// DEMO-01 PR C: the presentation camera (`auto`, `scroll`, `1.0`, `1.25`,
    /// `1.5`). `None` is the default of the run (`scroll` outside `--demo`).
    camera: Option<CameraChoice>,
    /// Evidence: run on room 1 to 6 of the camera lab (a tall greybox
    /// composition from the approved kit) instead of the 400 px corridor.
    camera_test_room: Option<usize>,
    /// Evidence: a synthetic foe that has taken aim, `X:FIRST:LAST` (world
    /// column, first and last elapsed tick). It only feeds the camera.
    camera_test_threat: Option<(f32, u64, u64)>,
    /// Headless: print one `FLOOR1_VIEW` line per tick of the presentation view.
    view_trace: bool,
    /// DEMO-01: the continuous demo route on the demo hall (`--demo`).
    demo: bool,
    /// DEMO-01: start at this beat's checkpoint (`--demo-beat N`, 0 to 6).
    demo_beat: usize,
    /// DEMO-01: the deterministic autopilot drives Carl (probes, evidence).
    autopilot: bool,
    /// HUD-DEBT-03: the Floor 01 boss arena (`--scene boss_f01`, or a boss
    /// fixture): the arena kit replaces the corridor.
    boss: bool,
    /// WORK-023: Donut starts with the biscuit and a full pool (live play and
    /// captures). Without it she gets it at the placeholder Guild zone.
    donut_biscuit: bool,
    /// CARL-PROG-01 CP-A (`--carl-prog`, `--carl-dynamite`, `--carl-gauntlet`,
    /// CP-Q09): the story pins and triggers, and the debug grants at start. They
    /// do nothing to a scripted fixture (it declares its own pin) and are
    /// refused with `--scene combat_lab`.
    carl: CarlProgFlags,
    /// AUDIENCE-01 QA probe (`--audience-probe`): unlock the per-hero viewer
    /// counts at step 1 (both heroes at the baseline) with one authored test
    /// cue at `audience_cue_x`, and print the `AUDIENCE_STATE` digest line.
    /// Floor 01 itself has no unlock (spec F15); the audience is an observer
    /// that never touches the simulation, so every other line is unchanged.
    audience_probe: bool,
    audience_cue_x: i32,
    /// COMBAT-FUN-02 (`--cf02`, `--cf02-seed N`): the CF02 run configuration.
    /// CF02-00 only carries it (the room is still the DEMO hall and no rule
    /// reads it), so a `--cf02` run replays exactly like `--demo`.
    cf02: Cf02Config,
    /// ENEMY-GAMEPLAY-ENGINEER (`--cf02-engineer`, needs `--cf02`): the microarena
    /// with the Goblin Engineer in the llama yard (`cf02_arena_eng`). Active only
    /// while the data kill switch `engq25_enabled` is on; with it off the flag runs
    /// plain `cf02_arena`.
    cf02_engineer: bool,
    /// COMPANION-AI-01 (`--companion off|cf02_assist|v0`, CA-22): who drives the
    /// uncontrolled hero of a live solo run. `None` is the default: the
    /// follow-and-jump bot, except that `--cf02` implies `cf02_assist`.
    companion: Option<CompanionChoice>,
    /// `--list-fixtures PREFIX`: print the fixture names with that prefix over
    /// every registry and exit before anything else starts.
    list_fixtures: Option<String>,
    /// PLAYTRACE-01: `--record` / `--no-record` (default: on for a live-input
    /// run, off for scripted, headless, capture and presim paths).
    record: RecordMode,
    /// `--record-dir DIR`: base directory of the sessions (default: the XDG
    /// data directory).
    record_dir: Option<PathBuf>,
    /// `--record-strict`: refuse to start when the recorder cannot start.
    record_strict: bool,
    /// `--record-other-tester`: the session includes a tester who is not the
    /// owner (PT-Q16; such a session is never exported without consent).
    record_other_tester: bool,
    /// COMBAT-LAB-01 LAB-00-wire: `--scene combat_lab`. The selection comes
    /// from the `--lab-*` flags (the Setup Cage hands the next room over by
    /// starting the scene again with them).
    lab: Option<LabSelection>,
    /// Set by `--scene combat_lab`; the `--lab-*` flags need it.
    lab_scene: bool,
    /// `--lab-autopilot`: the lab's scripted stand-in player drives Carl
    /// (evidence captures and probes; needs a scripted run, never a live one).
    lab_autopilot: bool,
    /// `--lab-variant` was given explicitly (it must agree with `--cf02-seed`).
    lab_variant_set: bool,
    /// The launched room (`main` fills it right after parsing).
    lab_run: Option<Box<LabRun>>,
    /// PLAYTRACE-RENDER-01 PTR-R1: `--replay-session ID`: the offline replay
    /// viewer. It takes the simulation from the recording alone, so every
    /// gameplay flag and every `--record*` flag is a parse error with it.
    replay: Option<replay_view::ReplayArgs>,
}

/// The PLAYTRACE-01 recording switch of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordMode {
    /// The default: [`RECORD_DEFAULT_ON`] for a live-input run only.
    Auto,
    On,
    Off,
}

/// `--companion` (CA-22).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompanionChoice {
    /// The rollback: the follow-and-jump bot (also `--cf02 --companion off`).
    Off,
    /// CF02's restrained assistant: AI Donut active, AI Carl passive.
    Cf02Assist,
    /// Both roles active.
    V0,
}

impl CompanionChoice {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "off" => Ok(Self::Off),
            "cf02_assist" => Ok(Self::Cf02Assist),
            "v0" => Ok(Self::V0),
            other => Err(format!(
                "unknown --companion {other:?} (expected off, cf02_assist or v0)"
            )),
        }
    }
}

impl Args {
    /// PLAYTRACE-01 (PT-Q12): the run has a live input source, so its
    /// session cannot be reproduced from the command line. False for every
    /// scripted, headless, capture, perf, view-trace, HUD-preview and QA-probe
    /// path, which are reproducible from their flags and goldens.
    fn live_input(&self) -> bool {
        !self.headless
            && self.replay.is_none()
            && !self.freeze
            && self.scenario.is_none()
            && self.party_fixture.is_none()
            && (self.fixture.is_none() || self.manual)
            && !self.autopilot
            && self.exit_after_ticks.is_none()
            && self.capture_tick.is_none()
            && self.capture_sequence.is_none()
            && !self.reference_frame
            && self.perf_frames.is_none()
            && self.perf_from_tick.is_none()
            && !self.view_trace
            && self.hud_preview.is_none()
            && self.camera_test_room.is_none()
            && !self.death_stress
            && !self.audience_probe
            && self.list_fixtures.is_none()
    }

    /// Whether this run records a PLAYTRACE-01 session by default (tests).
    #[cfg(test)]
    fn recording_enabled(&self) -> bool {
        match self.record {
            RecordMode::On => true,
            RecordMode::Off => false,
            RecordMode::Auto => RECORD_DEFAULT_ON && self.live_input(),
        }
    }

    /// Who drives the uncontrolled hero of a live run (CA-22): the explicit
    /// `--companion`, else `cf02_assist` under `--cf02` (Q13, Q33), else the
    /// follow-and-jump bot, so every default run is as before.
    fn uncontrolled_hero(&self) -> UncontrolledHero {
        match self.companion.unwrap_or(if self.cf02.rules {
            CompanionChoice::Cf02Assist
        } else {
            CompanionChoice::Off
        }) {
            CompanionChoice::Off => UncontrolledHero::FollowAndJumpBot,
            CompanionChoice::Cf02Assist => UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
            CompanionChoice::V0 => UncontrolledHero::Companion(ProfileSetId::CompanionV0),
        }
    }

    /// Whether the chosen companion stays idle in this run: a scripted run (the
    /// autopilot, a scenario, a headless or timed fixture) drives the other hero
    /// itself. `--demo` and `--cf02` fill in a fixture, so a live human run of
    /// them (`manual`) is not scripted and the companion runs.
    fn companion_inactive(&self) -> bool {
        self.autopilot
            || self.scenario.is_some()
            || self.party_fixture.is_some()
            || (self.fixture.is_some() && !self.manual)
    }

    /// CF02-F: this run is in the microarena (`--cf02`, or a CF02-F room fixture).
    fn arena_room(&self) -> bool {
        self.cf02.rules
            || self
                .fixture
                .is_some_and(|f| cf02::arena::fixture_uses_arena(f.name))
    }

    /// The party of a live run without a party fixture.
    fn live_party(&self) -> PartyState {
        let tuning = PartyTuning {
            uncontrolled_hero: self.uncontrolled_hero(),
            ..PROVISIONAL_TUNING
        };
        // CF02-Q19, Q20: the 12-tick swap cooldown and the eased pan, `--cf02` only.
        PartyState::new(if self.cf02.rules {
            tuning.with_cf02_swap()
        } else {
            tuning
        })
    }

    /// The CINE-01 setting this run uses. An explicit environment variable or
    /// flag wins. Otherwise it is on for live play (no scripted scenario,
    /// fixture script, autopilot, capture, perf probe or scripted exit) and
    /// off for every automated run, so fixtures and goldens never slow down
    /// unless a CINE-01 visual probe asks for it.
    fn cinematic_settings(&self) -> PresentationSettings {
        let live = self.scenario.is_none()
            && (self.fixture.is_none() || self.manual)
            && !self.autopilot
            && !self.freeze
            && !self.reference_frame
            && self.capture_tick.is_none()
            && self.capture_sequence.is_none()
            && self.perf_frames.is_none()
            && self.exit_after_ticks.is_none();
        if self.cinematic_set {
            self.cinematic
        } else if live {
            PresentationSettings::DEFAULT
        } else {
            PresentationSettings::OFF
        }
    }

    fn hud_config(&self) -> HudConfig {
        HudConfig {
            preview: self.hud_preview,
            glyphs: self.glyphs,
            settings: self.hud_settings,
        }
    }

    /// The camera of this run: `--camera`, else `auto` for `--demo` and the
    /// scroll camera for everything else (every fixture, scenario and capture).
    fn camera_choice(&self) -> CameraChoice {
        if self.reference_frame {
            return CameraChoice::Scroll;
        }
        self.camera
            .unwrap_or_else(|| CameraChoice::default_for(self.demo || self.lab_scene))
    }
}

fn parse_hud_preview(value: &str) -> Result<HudPreview, String> {
    HudPreview::parse(value).ok_or_else(|| {
        format!(
            "unknown HUD preview {value:?} (expected {})",
            HudPreview::names()
        )
    })
}

fn parse_glyphs(value: &str) -> Result<GlyphSet, String> {
    GlyphSet::parse(value)
        .ok_or_else(|| format!("unknown glyph set {value:?} (expected xbox, dualsense)"))
}

/// Parse `first:step:count` of `--capture-sequence`.
fn parse_sequence(value: &str) -> Result<(u64, u64, u64), String> {
    let bad = || format!("bad capture sequence {value:?} (expected first:step:count)");
    let mut parts = value.split(':');
    let mut next =
        || -> Result<u64, String> { parts.next().ok_or_else(bad)?.parse().map_err(|_| bad()) };
    let (first, step, count) = (next()?, next()?, next()?);
    if parts.next().is_some() || step == 0 || !(1..=1000).contains(&count) {
        return Err(bad());
    }
    Ok((first, step, count))
}

// ----------------------------------------------------------- combat lab wire

/// One `--lab-*` flag (`lab_flow` owns the list and the values).
fn lab_flag<'a>(
    slot: &mut Option<LabSelection>,
    flag: &str,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<bool, String> {
    if !lab_flow::is_lab_flag(flag) {
        return Ok(false);
    }
    let selection = slot.get_or_insert_with(LabSelection::default);
    lab_flow::apply_flag(selection, flag, || {
        iter.next()
            .cloned()
            .ok_or_else(|| format!("{flag} needs a value"))
    })
}

/// The lab's own argument rules (LAB-Q01 leak guard): `--scene combat_lab`
/// replaces every other run mode, and the `--lab-*` flags exist only with it.
fn resolve_lab_args(parsed: &mut Args) -> Result<(), String> {
    if !parsed.lab_scene {
        if parsed.lab.is_some() || parsed.lab_autopilot {
            return Err("--lab-room, --lab-variant, --lab-mode, --lab-next and --lab-confirmed need --scene combat_lab".into());
        }
        return Ok(());
    }
    if parsed.carl.any() {
        return Err("--carl-prog, --carl-dynamite and --carl-gauntlet are refused with --scene combat_lab (the lab profile pins the dynamite and the gauntlet off, LAB-Q01)".into());
    }
    if parsed.cf02.rules
        || parsed.demo
        || parsed.autopilot
        || parsed.demo_beat != 0
        || parsed.fixture.is_some()
        || parsed.party_fixture.is_some()
        || parsed.scenario.is_some()
        || parsed.camera_test_room.is_some()
        || parsed.reference_frame
        || parsed.headless
    {
        return Err("--scene combat_lab replaces --demo, --cf02, --autopilot, --demo-beat, --fixture, --party-fixture, --scenario, --camera-test-room, --reference-frame and the headless smoke (the headless lab runs in the tests)".into());
    }
    let selection = parsed.lab.get_or_insert_with(LabSelection::default);
    // R00 has one layout: its `--lab-variant` names the variant of the start
    // room (the Setup Cage's choice), not R00's own.
    if selection.room == "R00" && selection.variant != "v1" {
        selection.next_variant = std::mem::replace(&mut selection.variant, "v1".into());
    }
    // On R01 the seed is the variant (LAB-R-Q06): an explicit variant that
    // disagrees with the seed is refused here, not after a relaunch.
    if selection.room == "R01"
        && parsed.lab_variant_set
        && let Some(seed) = parsed.cf02.seed
        && selection.variant != format!("v{seed}")
    {
        return Err(format!(
            "--cf02-seed {seed} selects variant v{seed} of R01, but --lab-variant is {}",
            selection.variant
        ));
    }
    if parsed.lab_autopilot && selection.mode == combat_lab::PlayerMode::SoloDonut {
        return Err("--lab-autopilot drives Carl: use solo_carl or coop_local".into());
    }
    // The launch resolves the real fixture of the room; until then a
    // placeholder keeps the generic argument rules (`driven`) honest.
    parsed.fixture = Some(&PENDING_FIXTURE);
    // The post-biscuit profile: Donut starts with the biscuit and a full pool.
    parsed.donut_biscuit = true;
    // A live run without a script is the player's.
    parsed.manual = !(parsed.capture_tick.is_some()
        || parsed.capture_sequence.is_some()
        || parsed.exit_after_ticks.is_some()
        || parsed.freeze
        || parsed.perf_frames.is_some()
        || parsed.lab_autopilot);
    Ok(())
}

/// The entry point of a parsed run, for the leak guard (LAB-Q01): derived from
/// the arguments, never hard-coded at the call site.
fn entry_point(args: &Args) -> EntryPoint {
    if args.lab_scene {
        EntryPoint::CombatLab
    } else if args.boss {
        EntryPoint::Boss
    } else if args.cf02.rules {
        EntryPoint::Cf02
    } else if args.demo {
        EntryPoint::Demo
    } else if args.fixture.is_some() || args.party_fixture.is_some() {
        EntryPoint::Fixture
    } else {
        EntryPoint::Story
    }
}

/// The launch request of the parsed lab arguments. On R01 the seed is the
/// variant (LAB-R-Q06): `--cf02-seed 2` alone means variant `v2`.
fn lab_config(args: &Args) -> LabConfig {
    let selection = args.lab.clone().unwrap_or_default();
    let mut cfg = LabConfig::new(&selection.room, selection.mode);
    cfg.variant = selection.variant.clone();
    cfg.cf02 = args.cf02;
    cfg.view_h = view_height(args.deck);
    if selection.room == "R01"
        && selection.variant == "v1"
        && let Some(seed) = cfg.cf02.seed.filter(|s| (2..=3).contains(s))
    {
        cfg.variant = format!("v{seed}");
    }
    cfg
}

/// Put the human back on the mode's hero after a `party.restart()` (the rule
/// is `PlayerMode::seat`).
fn lab_reseat(args: &Args, party: &mut PartyState) {
    if let Some(run) = &args.lab_run {
        run.info.mode.seat(party);
    }
}

/// The party of a lab scene. A live co-op run starts from a fresh party so the
/// real pads join through `PadSlots` (Start on the second pad joins P2); a
/// scripted capture keeps the runner's two-player party.
fn lab_party(args: &Args, run: &LabRun) -> PartyState {
    if run.info.mode == combat_lab::PlayerMode::CoopLocal && args.manual {
        PartyState::default()
    } else {
        run.party.clone()
    }
}

/// The overlay and its plugin, when the run is a lab run.
fn lab_overlay_plugin(args: &Args, root: &Path) -> Result<Option<LabOverlayPlugin>, String> {
    let (Some(selection), Some(_)) = (&args.lab, &args.lab_run) else {
        return Ok(None);
    };
    let path = root.join(dungeon_floorbreak::combat_lab_overlay::FONT_PATH);
    let font = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let menu_open = selection.room == "R00" && !selection.confirmed;
    Ok(Some(LabOverlayPlugin {
        font,
        overlay: LabOverlay::new(view_height(args.deck), selection.clone(), menu_open),
    }))
}

/// Mark every descriptor above stderr close-on-exec before the relaunch
/// `exec`, so the kernel closes them atomically when the new image starts. The
/// GPU driver opens device and dmabuf descriptors without close-on-exec, so
/// each relaunch would otherwise leave its predecessor's four behind
/// (measured: 55 open descriptors at the first launch, 103 at the thirteenth).
/// Nothing is closed here: other threads keep working until the `exec`.
/// `close_range(.., CLOSE_RANGE_CLOEXEC)` does it in one call (Linux 5.11);
/// an older kernel falls back to `fcntl(F_SETFD)` per listed descriptor.
#[cfg(unix)]
fn close_inherited_fds() {
    // SAFETY: `close_range` only changes descriptor flags and touches no
    // memory of this process.
    let done = unsafe { libc::close_range(3, u32::MAX, libc::CLOSE_RANGE_CLOEXEC as _) } == 0;
    if done {
        return;
    }
    let open: Vec<i32> = std::fs::read_dir("/proc/self/fd")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
        .filter(|fd| *fd > 2)
        .collect();
    for fd in open {
        // SAFETY: `fcntl(F_SETFD)` on a listed number; a number that is gone
        // (the listing's own handle) just returns EBADF.
        unsafe {
            libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
        }
    }
}

#[cfg(not(unix))]
fn close_inherited_fds() {}

/// Start the scene again with the next selection: one room per launch (the
/// kit, the camera and the art are built once per process).
fn relaunch(selection: &LabSelection) -> ! {
    let argv: Vec<String> = std::env::args().collect();
    let next = lab_flow::relaunch_argv(&argv, selection);
    println!(
        "LAB_RELAUNCH room={} variant={} mode={} next={} confirmed={}",
        selection.room,
        selection.variant,
        selection.mode.id(),
        selection.next,
        selection.confirmed
    );
    let program = std::env::current_exe().unwrap_or_else(|_| PathBuf::from(&argv[0]));
    close_inherited_fds();
    let mut command = std::process::Command::new(program);
    command.args(&next[1..]);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = command.exec();
        eprintln!("floor01_scene: cannot start the next lab room: {error}");
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    {
        match command.spawn() {
            Ok(_) => std::process::exit(0),
            Err(error) => {
                eprintln!("floor01_scene: cannot start the next lab room: {error}");
                std::process::exit(1);
            }
        }
    }
}

/// The REC badge state (PTI-09): recording, failing, or off with the reason.
fn lab_rec_status(args: &Args, recorder: Option<&LiveRecorder>) -> RecStatus {
    if let Some(recorder) = recorder {
        let health = recorder.health();
        return if health.incomplete() {
            RecStatus::Error(
                health
                    .last_error
                    .unwrap_or_else(|| "writes failing".to_owned()),
            )
        } else {
            RecStatus::Recording
        };
    }
    match args.record {
        RecordMode::Off => RecStatus::Off("--no-record".into()),
        _ if !args.live_input() => RecStatus::Off("scripted run".into()),
        RecordMode::Auto if record_env_off().is_some() => {
            RecStatus::Off(format!("env {}", record_env_off().unwrap_or_default()))
        }
        // The recorder was wanted and is not there: it failed to start.
        _ => RecStatus::Error("recorder did not start".into()),
    }
}

/// One fixed tick of the lab flow, after `fixed_step`: the strip text, the
/// exit rule and the requests of the overlay. Reads the finished simulation;
/// nothing flows back.
fn lab_step(
    sim: Res<Sim>,
    combat: Res<Combat>,
    pads: Res<Pads>,
    setup: Res<Setup>,
    mut overlay: ResMut<LabOverlay>,
    mut rec: ResMut<Recorder>,
) {
    let (Some(run), Some(combat)) = (setup.args.lab_run.as_deref(), combat.0.as_ref()) else {
        return;
    };
    let status = lab_rec_status(&setup.args, rec.0.as_ref());
    if overlay.rec != status {
        overlay.rec = status;
    }
    let info = &run.info;
    let party = pads.0.party();
    let confirmed = overlay.selection.confirmed || overlay.selection.room != "R00";
    let exit = lab_flow::exit_state(info.goal, confirmed, combat);
    let seconds = combat.elapsed / 60;
    let party_text = match party.mode() {
        dungeon_floorbreak::party::PartyMode::TwoPlayer => "2P: P1 Carl, P2 Donut".to_owned(),
        dungeon_floorbreak::party::PartyMode::Solo
            if info.mode == combat_lab::PlayerMode::CoopLocal =>
        {
            "P2: Start on pad 2 (again each room)".to_owned()
        }
        dungeon_floorbreak::party::PartyMode::Solo => {
            format!("solo, Q/LB swaps ({:?})", party.controlled_hero())
        }
    };
    let exit_text = match exit {
        ExitState::Open => "EXIT open".to_owned(),
        ExitState::Closed if overlay.menu.is_some() => "EXIT closed: confirm the menu".to_owned(),
        ExitState::Closed => format!("EXIT closed: {} left", lab_flow::enemies_left(combat)),
    };
    let title = format!(
        "LAB {} {} | {} | legacy | seed {}",
        info.room,
        info.variant,
        info.mode.id(),
        info.seed
    );
    let status = format!(
        "{exit_text} | {party_text} | {:02}:{:02} deaths {} | R/Select: tap respawn, hold restart",
        seconds / 60,
        seconds % 60,
        combat.deaths
    );
    if overlay.title != title {
        overlay.title = title;
    }
    if overlay.status != status {
        overlay.status = status;
    }
    // A scripted run (capture, autopilot) shows the exit state but never
    // leaves the room: only a live player run transitions.
    if setup.args.manual
        && !overlay.blocks_gameplay()
        && exit == ExitState::Open
        && lab_flow::heroes_in_exit(&sim.0, party, info.exit)
    {
        match lab_flow::next_room(&overlay.selection) {
            NextRoom::Room(next) => overlay.request = Some(LabRequest::Relaunch(next)),
            NextRoom::End => {
                overlay.end = Some(EndPanel {
                    lines: vec![
                        format!(
                            "{} cleared: {} ticks ({}:{:02}), deaths {}",
                            info.room,
                            combat.elapsed,
                            seconds / 60,
                            seconds % 60,
                            combat.deaths
                        ),
                        format!(
                            "mode {}, variant {}, seed {}",
                            info.mode.id(),
                            info.variant,
                            info.seed
                        ),
                        format!("recording: {}", overlay.rec.badge().0),
                        "Facts only: no score, no rating.".to_owned(),
                    ],
                });
            }
        }
    }
    if let Some(LabRequest::Relaunch(next)) = overlay.request.take() {
        // `exec` bypasses the exit path: close and flush the session first, so
        // it is saved as completed and not recovered as a crash.
        if let Some(recorder) = rec.0.as_mut() {
            recorder.finish("lab_relaunch");
        }
        rec.flush();
        relaunch(&next);
    }
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    parse_args_full(
        args,
        HudSettings::from_env(),
        (
            PresentationSettings::from_env(),
            std::env::var_os(ENV_CINEMATIC_FINISHES).is_some(),
        ),
    )
}

/// `parse_args` with the HUD settings the environment supplied (a parameter so
/// tests do not depend on the process environment).
#[cfg(test)]
fn parse_args_with(args: &[String], hud_settings: HudSettings) -> Result<Args, String> {
    parse_args_full(args, hud_settings, (PresentationSettings::DEFAULT, false))
}

/// [`parse_args_with`] plus the CINE-01 setting the environment supplied and
/// whether the environment set it at all.
fn parse_args_full(
    args: &[String],
    hud_settings: HudSettings,
    (cinematic, cinematic_set): (PresentationSettings, bool),
) -> Result<Args, String> {
    let mut parsed = Args {
        deck: false,
        headless: false,
        freeze: false,
        debug_collision: false,
        scenario: None,
        fixture: None,
        party_fixture: None,
        manual: false,
        debug_combat: false,
        capture_tick: None,
        scale: 2,
        no_lighting: false,
        reference_frame: false,
        perf_frames: None,
        perf_from_tick: None,
        exit_after_ticks: None,
        capture_sequence: None,
        capture_dir: None,
        anim_trace: false,
        combat_trace: false,
        no_audio: false,
        audio_style: None,
        gore_preset: GorePreset::default(),
        death_stress: false,
        death_trace: false,
        hud_preview: None,
        glyphs: None,
        hud_settings,
        cinematic,
        cinematic_set,
        camera: None,
        camera_test_room: None,
        camera_test_threat: None,
        view_trace: false,
        demo: false,
        demo_beat: 0,
        autopilot: false,
        boss: false,
        donut_biscuit: false,
        carl: CarlProgFlags::OFF,
        audience_probe: false,
        audience_cue_x: AUDIENCE_PROBE_CUE_X,
        cf02: Cf02Config::OFF,
        cf02_engineer: false,
        companion: None,
        list_fixtures: None,
        record: RecordMode::Auto,
        record_dir: None,
        record_strict: false,
        record_other_tester: false,
        lab: None,
        lab_scene: false,
        lab_autopilot: false,
        lab_variant_set: false,
        lab_run: None,
        replay: None,
    };
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--cf02" => parsed.cf02.rules = true,
            "--cf02-engineer" => parsed.cf02_engineer = true,
            "--record" => parsed.record = RecordMode::On,
            "--no-record" => parsed.record = RecordMode::Off,
            "--record-strict" => parsed.record_strict = true,
            "--record-other-tester" => parsed.record_other_tester = true,
            "--record-dir" => {
                let value = iter.next().ok_or("--record-dir needs a directory")?;
                parsed.record_dir = Some(PathBuf::from(value));
            }
            "--cf02-seed" => {
                let value = iter.next().ok_or("--cf02-seed needs a u64 seed")?;
                parsed.cf02.seed = Some(
                    value
                        .parse()
                        .map_err(|_| format!("bad --cf02-seed {value:?} (an unsigned integer)"))?,
                );
            }
            "--companion" => {
                let value = iter
                    .next()
                    .ok_or("--companion needs off, cf02_assist or v0")?;
                parsed.companion = Some(CompanionChoice::parse(value)?);
            }
            "--list-fixtures" => {
                let value = iter.next().ok_or("--list-fixtures needs a name prefix")?;
                parsed.list_fixtures = Some(value.clone());
            }
            "--deck" => parsed.deck = true,
            "--headless-floor01-smoke" => parsed.headless = true,
            "--freeze" => parsed.freeze = true,
            "--debug-collision" => parsed.debug_collision = true,
            "--manual" => parsed.manual = true,
            "--donut-biscuit" => parsed.donut_biscuit = true,
            "--carl-prog" => parsed.carl.prog = true,
            "--carl-dynamite" => parsed.carl.dynamite = true,
            "--carl-gauntlet" => parsed.carl.gauntlet = true,
            "--audience-probe" => parsed.audience_probe = true,
            "--audience-cue-x" => {
                let value = iter.next().ok_or("--audience-cue-x needs a scene x")?;
                parsed.audience_cue_x = value
                    .parse()
                    .map_err(|_| format!("bad --audience-cue-x {value:?} (an integer)"))?;
            }
            "--debug-combat" => parsed.debug_combat = true,
            "--fixture" => {
                let value = iter.next().ok_or("--fixture needs a name")?;
                if parsed.party_fixture.is_some() {
                    return Err("--fixture and --party-fixture are mutually exclusive".into());
                }
                parsed.fixture = Some(Fixture::by_name(value).ok_or_else(|| {
                    format!("unknown fixture {value:?} (expected {})", Fixture::names())
                })?);
            }
            "--party-fixture" => {
                let value = iter.next().ok_or("--party-fixture needs a name")?;
                if parsed.fixture.is_some() {
                    return Err("--fixture and --party-fixture are mutually exclusive".into());
                }
                let party = find_party_fixture(value).ok_or_else(|| {
                    format!(
                        "unknown party fixture {value:?} (expected {})",
                        PartyFixture::names()
                    )
                })?;
                parsed.party_fixture = Some(party);
                parsed.fixture = Some(party.base);
            }
            "--hud-preview" => {
                let value = iter.next().ok_or("--hud-preview needs a name")?;
                parsed.hud_preview = Some(parse_hud_preview(value)?);
            }
            other if other.starts_with("--hud-preview=") => {
                parsed.hud_preview = Some(parse_hud_preview(&other["--hud-preview=".len()..])?);
            }
            "--glyphs" => {
                let value = iter.next().ok_or("--glyphs needs xbox or dualsense")?;
                parsed.glyphs = Some(parse_glyphs(value)?);
            }
            other if other.starts_with("--glyphs=") => {
                parsed.glyphs = Some(parse_glyphs(&other["--glyphs=".len()..])?);
            }
            other if parsed.hud_settings.apply_flag(other)? => {}
            other if parsed.cinematic.apply_flag(other)? => parsed.cinematic_set = true,
            other if replay_view::replay_flag(&mut parsed.replay, other, &mut iter)? => {}
            "--no-lighting" => parsed.no_lighting = true,
            "--demo" => parsed.demo = true,
            "--scene" => {
                let value = iter
                    .next()
                    .ok_or("--scene needs a name (boss_f01, combat_lab)")?;
                if parsed.boss || parsed.lab_scene {
                    return Err(
                        "--scene takes one scene: boss_f01 and combat_lab are exclusive".into(),
                    );
                }
                match value.as_str() {
                    "boss_f01" => {
                        if parsed.fixture.is_some() || parsed.party_fixture.is_some() {
                            return Err(
                                "--scene boss_f01 replaces --fixture and --party-fixture".into()
                            );
                        }
                        parsed.boss = true;
                        parsed.fixture = Some(boss_scene::BOSS01_SCENE);
                    }
                    "combat_lab" => parsed.lab_scene = true,
                    _ => {
                        return Err(format!(
                            "unknown scene {value:?} (expected boss_f01, combat_lab)"
                        ));
                    }
                }
            }
            other if lab_flag(&mut parsed.lab, other, &mut iter)? => {
                parsed.lab_variant_set |= other == "--lab-variant";
            }
            "--lab-autopilot" => parsed.lab_autopilot = true,
            "--autopilot" => parsed.autopilot = true,
            "--demo-beat" => {
                let value = iter.next().ok_or("--demo-beat needs a beat 0 to 6")?;
                parsed.demo_beat = value
                    .parse()
                    .ok()
                    .filter(|beat| *beat < demo::BEATS.len())
                    .ok_or_else(|| format!("bad beat {value:?} (0 to 6)"))?;
            }
            "--no-audio" => parsed.no_audio = true,
            "--audio-style" => {
                let value = iter.next().ok_or("--audio-style needs a name")?;
                parsed.audio_style = Some(StyleBank::parse(value).ok_or_else(|| {
                    format!("unknown audio style {value:?} (expected arcade, cinematic, hybrid)")
                })?);
            }
            "--gore-preset" => {
                let value = iter.next().ok_or("--gore-preset needs 1, 2, 3 or 4")?;
                parsed.gore_preset = GorePreset::parse(value).ok_or_else(|| {
                    format!(
                        "bad gore preset {value:?} (expected 1 to 4, or off, reduced, medium, full)"
                    )
                })?;
            }
            "--death-stress" => parsed.death_stress = true,
            "--death-trace" => parsed.death_trace = true,
            "--camera" => {
                let value = iter.next().ok_or("--camera needs a mode")?;
                parsed.camera = Some(CameraChoice::parse(value)?);
            }
            "--camera-test-room" => {
                let value = iter.next().ok_or("--camera-test-room needs 1 to 6")?;
                parsed.camera_test_room = Some(
                    value
                        .parse()
                        .ok()
                        .filter(|n| (1..=6).contains(n))
                        .ok_or_else(|| format!("bad camera test room {value:?} (1 to 6)"))?,
                );
            }
            "--camera-test-threat" => {
                let value = iter
                    .next()
                    .ok_or("--camera-test-threat needs X:FIRST:LAST")?;
                let bad = || format!("bad camera test threat {value:?} (expected X:FIRST:LAST)");
                let mut parts = value.split(':');
                let x: f32 = parts.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
                let first: u64 = parts.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
                let last: u64 = parts.next().and_then(|v| v.parse().ok()).ok_or_else(bad)?;
                if parts.next().is_some() || last <= first {
                    return Err(bad());
                }
                parsed.camera_test_threat = Some((x, first, last));
            }
            "--view-trace" => parsed.view_trace = true,
            "--reference-frame" => parsed.reference_frame = true,
            "--anim-trace" => parsed.anim_trace = true,
            "--combat-trace" => parsed.combat_trace = true,
            "--capture-sequence" => {
                let value = iter
                    .next()
                    .ok_or("--capture-sequence needs first:step:count")?;
                parsed.capture_sequence = Some(parse_sequence(value)?);
            }
            "--capture-dir" => {
                let value = iter.next().ok_or("--capture-dir needs a directory")?;
                parsed.capture_dir = Some(PathBuf::from(value));
            }
            "--exit-after-ticks" => {
                let value = iter.next().ok_or("--exit-after-ticks needs a tick count")?;
                parsed.exit_after_ticks = Some(
                    value
                        .parse()
                        .ok()
                        .filter(|n| *n > 0)
                        .ok_or_else(|| format!("bad tick count {value:?}"))?,
                );
            }
            "--perf-frames" => {
                let value = iter.next().ok_or("--perf-frames needs a frame count")?;
                parsed.perf_frames = Some(
                    value
                        .parse()
                        .ok()
                        .filter(|n| (30..=100_000).contains(n))
                        .ok_or_else(|| format!("bad frame count {value:?} (30..=100000)"))?,
                );
            }
            "--perf-from-tick" => {
                let value = iter.next().ok_or("--perf-from-tick needs a tick")?;
                parsed.perf_from_tick =
                    Some(value.parse().map_err(|_| format!("bad tick {value:?}"))?);
            }
            "--scenario" => {
                let value = iter.next().ok_or("--scenario needs a name")?;
                parsed.scenario = Some(value.parse()?);
            }
            "--capture-tick" => {
                let value = iter.next().ok_or("--capture-tick needs a tick count")?;
                parsed.capture_tick = Some(
                    value
                        .parse()
                        .map_err(|_| format!("bad tick count {value:?}"))?,
                );
            }
            "--scale" => {
                let value = iter.next().ok_or("--scale needs an integer")?;
                parsed.scale = value
                    .parse()
                    .ok()
                    .filter(|s| (1..=8).contains(s))
                    .ok_or_else(|| format!("bad scale {value:?} (1..=8)"))?;
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    replay_view::check_args(args, &parsed.replay)?;
    if parsed.replay.is_some() {
        // A replay takes its room, rules and deck from the recording.
        return Ok(parsed);
    }
    if parsed.cf02_engineer {
        // ENG-G-Q25: the Engineer exists only in the `--cf02` microarena. The story
        // route, `--demo`, the lab, the boss arena and every fixture never load him.
        if !parsed.cf02.rules {
            return Err(
                "--cf02-engineer needs --cf02 (the Engineer lives only in the CF02 microarena)"
                    .into(),
            );
        }
        if parsed.demo
            || parsed.lab_scene
            || parsed.boss
            || parsed.fixture.is_some()
            || parsed.party_fixture.is_some()
            || parsed.scenario.is_some()
        {
            return Err("--cf02-engineer replaces --demo, --scene, --fixture, --party-fixture and --scenario (it is the --cf02 room with the Engineer)".into());
        }
    }
    resolve_lab_args(&mut parsed)?;
    if parsed.cf02.seed.is_some() && !parsed.cf02.rules && !parsed.lab_scene {
        return Err("--cf02-seed needs --cf02 (or --scene combat_lab)".into());
    }
    if parsed.cf02.rules {
        // CF02-F: `--cf02` is the microarena. It runs on the DEMO machinery (the
        // checkpoint respawn, the Select tap and hold, the autopilot flag), with
        // the arena's own room, fixture and rules.
        parsed.demo = true;
    }
    if parsed.companion.is_some() && (parsed.fixture.is_some() || parsed.party_fixture.is_some()) {
        // A fixture declares its own tuning (and so its own companion mode);
        // the flag is for the live scene only (CA-22).
        return Err("--companion cannot be combined with --fixture or --party-fixture".into());
    }
    parsed.boss |= parsed.fixture.is_some_and(|f| f.boss.is_some());
    if parsed.boss && (parsed.demo || parsed.scenario.is_some() || parsed.reference_frame) {
        return Err("the boss arena replaces --demo, --scenario and --reference-frame".into());
    }
    if parsed.boss && parsed.fixture == Some(boss_scene::BOSS01_SCENE) {
        // A live run without a script is the player's.
        parsed.manual = !(parsed.headless
            || parsed.capture_tick.is_some()
            || parsed.capture_sequence.is_some()
            || parsed.exit_after_ticks.is_some()
            || parsed.freeze
            || parsed.perf_frames.is_some());
    }
    if parsed.demo {
        if parsed.fixture.is_some() || parsed.scenario.is_some() || parsed.reference_frame {
            return Err("--demo replaces --fixture, --scenario and --reference-frame".into());
        }
        parsed.fixture = Some(if parsed.cf02.rules {
            cf02::arena::fixture_for_run(
                parsed.demo_beat,
                parsed.cf02_engineer,
                dungeon_floorbreak::engineer::active(),
            )
        } else {
            demo::fixture_for_beat(parsed.demo_beat)
        });
        // A live run without a script or autopilot is the player's.
        parsed.manual = !parsed.autopilot
            && !(parsed.headless
                || parsed.capture_tick.is_some()
                || parsed.capture_sequence.is_some()
                || parsed.exit_after_ticks.is_some()
                || parsed.freeze
                || parsed.perf_frames.is_some());
    } else if parsed.autopilot || parsed.demo_beat != 0 {
        return Err("--autopilot and --demo-beat need --demo".into());
    }
    if parsed.demo && parsed.camera_test_room.is_some() {
        return Err(
            "--camera-test-room and --demo are mutually exclusive (the demo has its own hall)"
                .into(),
        );
    }
    if parsed.party_fixture.is_some()
        && !(parsed.headless || parsed.capture_tick.is_some() || parsed.capture_sequence.is_some())
        || parsed.party_fixture.is_some()
            && (parsed.manual
                || parsed.exit_after_ticks.is_some()
                || parsed.perf_frames.is_some()
                || parsed.death_stress
                || parsed.combat_trace
                || parsed.anim_trace)
    {
        return Err(
            "--party-fixture is a headless replay, a --capture-tick capture or a --capture-sequence only (no --manual, --exit-after-ticks, --perf-frames, --death-stress, --combat-trace or --anim-trace)"
                .into(),
        );
    }
    let driven = parsed.scenario.is_some() || parsed.fixture.is_some();
    if parsed.scenario.is_some() && parsed.fixture.is_some() {
        return Err("--scenario and --fixture are mutually exclusive".into());
    }
    if (parsed.manual || parsed.debug_combat) && parsed.fixture.is_none() {
        return Err("--manual and --debug-combat need --fixture".into());
    }
    if (parsed.death_stress || parsed.death_trace) && parsed.fixture.is_none() {
        return Err("--death-stress and --death-trace need --fixture".into());
    }
    if parsed.death_stress && (parsed.headless || parsed.capture_tick.is_some()) {
        return Err(
            "--death-stress is a live run: drop --headless-floor01-smoke and --capture-tick".into(),
        );
    }
    if parsed.death_trace && !parsed.headless {
        return Err("--death-trace needs --headless-floor01-smoke".into());
    }
    if parsed.manual
        && (parsed.headless
            || parsed.capture_tick.is_some()
            || parsed.capture_sequence.is_some()
            || parsed.exit_after_ticks.is_some()
            || parsed.freeze)
    {
        return Err("--manual is a live keyboard run: drop the scripted-run flags".into());
    }
    if parsed.capture_tick.is_some() && !driven {
        return Err("--capture-tick needs --scenario or --fixture".into());
    }
    if parsed.exit_after_ticks.is_some()
        && (!driven || parsed.capture_tick.is_some() || parsed.freeze || parsed.reference_frame)
    {
        return Err(
            "--exit-after-ticks needs --scenario or --fixture and a live run (no --capture-tick, --freeze or --reference-frame)"
                .into(),
        );
    }
    if parsed.perf_from_tick.is_some()
        && (parsed.fixture.is_none()
            || parsed.perf_frames.is_none()
            || parsed.headless
            || parsed.manual
            || parsed.capture_tick.is_some()
            || parsed.capture_sequence.is_some()
            || parsed.exit_after_ticks.is_some()
            || parsed.freeze
            || parsed.reference_frame)
    {
        return Err(
            "--perf-from-tick needs --fixture and --perf-frames, and no other run mode".into(),
        );
    }
    if parsed.capture_sequence.is_some() != parsed.capture_dir.is_some() {
        return Err("--capture-sequence and --capture-dir go together".into());
    }
    if parsed.capture_sequence.is_some()
        && (!driven
            || parsed.capture_tick.is_some()
            || parsed.exit_after_ticks.is_some()
            || parsed.freeze
            || parsed.reference_frame
            || parsed.perf_frames.is_some())
    {
        return Err(
            "--capture-sequence needs --scenario or --fixture and no --capture-tick, --exit-after-ticks, --freeze, --reference-frame or --perf-frames"
                .into(),
        );
    }
    if parsed.combat_trace && (!parsed.headless || parsed.fixture.is_none()) {
        return Err("--combat-trace needs --headless-floor01-smoke and --fixture".into());
    }
    if parsed.anim_trace && (!parsed.headless || !driven) {
        return Err(
            "--anim-trace needs --headless-floor01-smoke and --scenario or --fixture".into(),
        );
    }
    if parsed.view_trace && !parsed.headless {
        return Err("--view-trace needs --headless-floor01-smoke".into());
    }
    if parsed.camera_test_threat.is_some() && parsed.camera_test_room.is_none() && !parsed.demo {
        return Err("--camera-test-threat needs --camera-test-room or --demo (a tall room)".into());
    }
    if parsed.reference_frame && parsed.camera.is_some_and(|c| c != CameraChoice::Scroll) {
        return Err("--reference-frame pins the scroll camera; drop --camera".into());
    }
    if parsed.reference_frame && (driven || parsed.capture_tick.is_some()) {
        return Err(
            "--reference-frame pins its own state; drop --scenario/--fixture/--capture-tick".into(),
        );
    }
    if parsed.record == RecordMode::On
        && (parsed.headless
            || parsed.list_fixtures.is_some()
            || parsed.capture_tick.is_some()
            || parsed.capture_sequence.is_some()
            || parsed.perf_from_tick.is_some())
    {
        return Err(
            "--record needs a windowed run without a presim (no --headless-floor01-smoke, --capture-tick, --capture-sequence or --perf-from-tick)"
                .into(),
        );
    }
    Ok(parsed)
}

fn view_height(deck: bool) -> i32 {
    if deck { 400 } else { 360 }
}

/// Ticks a scenario is pre-simulated for: the capture tick, or the first tick
/// of a capture sequence.
fn presim_ticks(args: &Args) -> Option<u64> {
    args.capture_tick
        .or(args.capture_sequence.map(|(first, _, _)| first))
        .or(args.perf_from_tick)
}

/// Build the simulation and its animators in the requested state (scenario
/// pre-simulated). The animators are stepped from the finished state of every
/// tick and never feed back into the simulation.
type Scene = (
    SceneSim,
    SceneAnimators,
    Option<CombatState>,
    Option<DeathPresenter>,
    PartyState,
);

/// The cosmetic death presenter of a fixture run: the goblin death art at the
/// requested preset. Presentation only.
fn death_presenter(root: &Path, args: &Args) -> Result<DeathPresenter, String> {
    let art = Arc::new(DeathArt::load(root)?);
    let variant_b = DeathArt::load_variant(root, 1)?;
    DeathPresenter::with_variants(art, &variant_b, args.gore_preset)?
        .with_llama(Arc::new(DeathArt::load_llama(root)?))
}

/// Where the QA probe's one authored anticipation cue fires (any hero's x
/// reaching it, scene units). The fixtures start Carl at 400.
const AUDIENCE_PROBE_CUE_X: i32 = 420;
/// The cue id and award (inside the tuning range) of the QA probe.
const AUDIENCE_PROBE_CUE_ID: u32 = 1;
const AUDIENCE_PROBE_CUE_AWARD: u64 = 4_000;

/// The HUD runtime with its real identity: the actor manifests' display names
/// and the floor number of the layout. With `--audience-probe` it also unlocks
/// the per-hero viewer counts (the QA fixture of AUDIENCE-01; no Floor 01 run
/// unlocks them).
fn new_hud(args: &Args, kit: &EnvKit, actors: &Actors) -> HudRuntime {
    let mut hud = HudRuntime::new(args.hud_config());
    hud.set_identity(
        &actors.carl.display_name,
        &actors.donut.display_name,
        &kit.layout_id,
    );
    if args.audience_probe {
        let cue = AnticipationCue {
            id: AUDIENCE_PROBE_CUE_ID,
            trigger: AnticipationTrigger::HeroCrossesX {
                x: args.audience_cue_x,
            },
            heroes: vec![HeroId::Carl, HeroId::Donut],
            award: AUDIENCE_PROBE_CUE_AWARD,
        };
        hud.unlock_audience(vec![cue])
            .expect("the probe cue is inside the tuning range");
    }
    hud
}

/// The `AUDIENCE_STATE` line: printed only while the audience is unlocked (the
/// QA probe), never by a Floor 01 run, and never part of the combat hash.
fn print_audience_state(hud: &HudRuntime) {
    let audience = hud.audience();
    if let (Some(digest), Some(carl), Some(donut)) = (
        audience.digest(),
        audience.view(HeroId::Carl),
        audience.view(HeroId::Donut),
    ) {
        println!(
            "AUDIENCE_STATE steps={} carl={} donut={} carl_trend={:?} donut_trend={:?} digest={digest:016x}",
            audience.steps(),
            audience.count(HeroId::Carl).unwrap_or(0),
            audience.count(HeroId::Donut).unwrap_or(0),
            carl.trend,
            donut.trend,
        );
    }
}

/// WORK-023, HUD-DEBT-02: Donut's biscuit state for this run. A party fixture
/// declares its own; the boss arena (after the Guild) and `--donut-biscuit`
/// start with it and a full pool; a live run elsewhere gets it at the
/// placeholder Guild zone (an x range of the layout, until the Guild room
/// exists); the DEMO-01 hall has no zone and stays locked. Scripted runs never
/// read it (the party is not live there).
fn donut_setup(args: &Args) -> DonutSetup {
    if let Some(pf) = args.party_fixture {
        return pf.donut;
    }
    if args.donut_biscuit || args.boss {
        return DonutSetup::unlocked(8);
    }
    if args.demo {
        // DEMO-01 is documented as "without the Guild or Donut's magic": the
        // hall has no unlock zone, so the biscuit stays locked (the swat and
        // the pounce stay available).
        return DonutSetup::NONE;
    }
    DonutSetup {
        start: StartState::LOCKED,
        guild_zone: Some(donut_fixtures::GUILD_PLACEHOLDER_ZONE),
    }
}

/// CARL-PROG-01 CP-A: the story a combat run starts under. A scripted fixture
/// (`--party-fixture`, `--fixture`) declares its own pin and never reads the
/// flags; the DEMO-01 hall, the CF02 microarena and the boss arena read them
/// (`--carl-prog` holds and arms nothing in the first two). The scene kinds
/// match the recording replay (`playtrace::replay::story_scene`).
fn story_setup(args: &Args) -> StorySetup {
    if let Some(pf) = args.party_fixture {
        return carl_prog_fixtures::story_setup(pf.name());
    }
    let Some(fixture) = args.fixture else {
        return StorySetup::NONE;
    };
    let scene = if args.demo && args.cf02.rules {
        SceneKind::CfArena
    } else if args.demo {
        SceneKind::Demo
    } else if fixture == boss_scene::BOSS01_SCENE {
        SceneKind::Boss
    } else {
        return carl_prog_fixtures::story_setup(fixture.name);
    };
    args.carl.story_setup(scene)
}

/// One HUD step after a finished simulation tick: the real health values and
/// the newest combat events. Read-only on the simulation.
fn step_hud(hud: &mut HudRuntime, sim: &SceneSim, combat: Option<&CombatState>) {
    let (events, dropped) =
        combat.map_or((&[][..], 0), |c| (c.events.as_slice(), c.dropped_events));
    // Melee input is live exactly when the combat state exists (see the
    // manual input gate in `fixed_step`).
    hud.set_melee_live(combat.is_some());
    // AUDIENCE-01: the heroes' columns, for authored anticipation cues (read
    // only; the observer is inert while the audience is locked).
    hud.set_hero_x(sim.world.hero.x as i32, sim.world.donut.x as i32);
    // HUD-DEBT-02: Donut's real mana (None while the biscuit is locked).
    hud.set_donut_mana(combat.and_then(CombatState::donut_mana_view));
    if let Some(combat) = combat {
        hud.set_empty_press(combat.donut_mana_tuning().empty_press);
    }
    // HUD-DEBT-03: the boss strip reads the simulation's boss state.
    match combat.and_then(|c| c.boss.as_ref()) {
        Some(boss) => {
            let data = boss.data();
            hud.set_boss(
                Some(boss.state()),
                &data.hud.display_name,
                &data.hud.emblem,
                data.exit_slide_delay,
            );
        }
        None => hud.set_boss(None, "", "", 0),
    }
    hud.step(
        sim.world.hero.health,
        sim.world.donut.health,
        events,
        dropped,
    );
}

/// The presentation half of the combat branch of [`build_scene`]: the animators
/// with the melee and CF02 art, the cosmetic death presenter and the HUD's
/// melee flag. It builds no simulation (PTR-R1 calls it with the replay
/// cursor's own state); presentation only.
fn combat_presentation(
    actors: &Actors,
    root: &Path,
    args: &Args,
    sim: &SceneSim,
    combat: &CombatState,
    hud: &mut HudRuntime,
) -> Result<(SceneAnimators, DeathPresenter), String> {
    let mut anim = actors.animators(&sim.world);
    anim.art = actors.combat_art(combat);
    let mut death = death_presenter(root, args)?;
    // CF02-D (Q36): the death overlay and the gore budget under cf02_rules.
    if let Some(tuning) = combat.cf02() {
        death = death.with_cf02(tuning);
    }
    hud.set_melee_live(true);
    Ok((anim, death))
}

fn build_scene(
    kit: &EnvKit,
    args: &Args,
    actors: &Actors,
    root: &Path,
    hud: &mut HudRuntime,
) -> Result<Scene, String> {
    if let Some(fixture) = args.fixture {
        // A combat fixture: Carl starts at the fixture column; the script is
        // pre-simulated for a capture, otherwise left for the live loop.
        // A lab run reuses the state the runner built (demo checkpoint rules,
        // the post-biscuit Donut, the legacy rules); nothing of it is rebuilt.
        let (mut sim, mut combat) = if let Some(run) = &args.lab_run {
            (run.scene.clone(), run.combat.clone())
        } else {
            let sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
            let combat = CombatState::new(fixture, kit, &sim)?
                .with_donut(donut_setup(args))
                .with_story(story_setup(args));
            (sim, combat)
        };
        if args.cf02.rules {
            combat = combat.with_demo(&cf02::arena::RULES);
        } else if args.demo {
            combat = combat.with_demo(&demo::DEMO_RULES);
        }
        // COMBAT-FUN-02: `--cf02` runs the CF02 rules, llama targets (C1)
        // included (every `cf02_*` fixture already does, by its name).
        if args.cf02.rules {
            combat = combat.with_cf02().with_cf02_seed(args.cf02.seed);
        }
        let (mut anim, mut death) = combat_presentation(actors, root, args, &sim, &combat, hud)?;
        // A party fixture runs the live party exactly like the interactive
        // scene (slot inputs, solo swap, drive); a plain fixture stays legacy.
        let mut party = if let Some(run) = &args.lab_run {
            lab_party(args, run)
        } else {
            args.party_fixture.map_or_else(
                || args.live_party(),
                |pf| {
                    let mut party = PartyState::new(pf.tuning);
                    if pf.two_player {
                        party.pad_connected(PadId(1));
                        party.pad_start_pressed(PadId(2));
                    }
                    party
                },
            )
        };
        if let Some(pf) = args.party_fixture {
            pf.place_donut(&mut sim);
            sim.set_drive(pf.drive(&party));
        } else if args.lab_run.is_some() {
            sim.set_drive(party.drive(true));
        }
        let mut feat_feed = AudienceFeatFeed::default();
        for _ in 0..presim_ticks(args).unwrap_or(0) {
            let inputs = if let Some(pf) = args.party_fixture {
                let slots = pf.inputs(sim.world.tick + 1, &combat, &sim);
                let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
                party.request_swap(slots[0].swap, hurt);
                sim.set_drive(pf.drive(&party));
                party.hero_inputs(slots)
            } else if args.lab_run.is_some() {
                // A lab run routes the scripted slot through the party, like
                // the live scene (a solo Donut run drives Donut).
                let slots = scripted_inputs(args, fixture, &sim, &combat);
                let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
                party.request_swap(false, hurt);
                sim.set_drive(party.drive(true));
                party.hero_inputs(slots)
            } else {
                [
                    scripted_input(args, fixture, &sim, &combat),
                    RawInput::default(),
                ]
            };
            let restarts = combat.restarts;
            let _ = step_combat(&mut sim, &mut combat, &mut anim, Some(&mut death), inputs);
            if combat.restarts != restarts {
                party.restart();
                lab_reseat(args, &mut party);
            }
            feat_feed.observe(&combat, hud);
            step_hud(hud, &sim, Some(&combat));
            if args.death_stress && args.perf_from_tick.is_some() {
                // Lab: load the pools before the probe starts measuring.
                death.stress_tick(sim.world.hero.x, sim.world.hero.y - HERO_HALF_HEIGHT);
            }
            if args.combat_trace {
                println!("{}", combat.trace_line(&sim));
            }
            if args.anim_trace
                && let Some(art) = anim.art.as_ref()
            {
                println!("{}", art.trace_line(&sim, &combat));
            }
        }
        return Ok((sim, anim, Some(combat), Some(death), party));
    }
    let mut sim = SceneSim::new(kit, view_height(args.deck))?;
    let mut anim = actors.animators(&sim.world);
    if let (Some(scenario), Some(ticks)) = (args.scenario, presim_ticks(args)) {
        let trace = args.anim_trace;
        sim.run_scenario_observed(scenario, ticks, |state| {
            anim.step(&state.world);
            step_hud(hud, state, None);
            if trace {
                println!("{}", trace_line(state, &anim));
            }
        });
    }
    Ok((sim, anim, None, None, args.live_party()))
}

/// One fixed tick of a combat fixture, and the animation that follows it. A
/// restart inside the step (Carl was defeated) restarts the animation too.
/// Returns true when a DEMO-01 respawn happened inside the step.
fn step_combat(
    sim: &mut SceneSim,
    combat: &mut CombatState,
    anim: &mut SceneAnimators,
    death: Option<&mut DeathPresenter>,
    inputs: [RawInput; 2],
) -> bool {
    let expected = sim.world.tick + 1;
    combat.step_party(sim, inputs);
    // A demo respawn keeps the tick counter but moves Carl: restart his
    // animation like a fixture restart does.
    let respawned = combat.take_respawned();
    if sim.world.tick == expected && !respawned {
        anim.step(&sim.world);
        anim.step_art(sim, combat);
    } else {
        anim.reset(&sim.world);
        anim.reset_art(combat);
    }
    // Cosmetics follow the finished tick; nothing flows back.
    if let Some(death) = death {
        death.observe(combat, sim);
    }
    respawned
}

/// The input of the scripted run of a fixture at the next tick: the fixture's
/// script, or the DEMO-01 autopilot (a pure function of the finished state).
fn scripted_input(
    args: &Args,
    fixture: &Fixture,
    sim: &SceneSim,
    combat: &CombatState,
) -> RawInput {
    if args.lab_autopilot
        && let Some(run) = &args.lab_run
    {
        combat_lab::autopilot::clear_and_exit(combat, sim, run.info.exit.x as f32 + 32.0)
    } else if args.autopilot && args.cf02.rules {
        cf02::arena::autopilot(combat, sim, cf02::arena::Plan::Direct)
    } else if args.autopilot {
        demo::autopilot(combat, sim)
    } else {
        fixture.input(sim.world.tick + 1)
    }
}

/// The slot inputs of a scripted run: the fixture's or autopilot's first slot,
/// and the lab autopilot's follower in a two-player lab run.
fn scripted_inputs(
    args: &Args,
    fixture: &Fixture,
    sim: &SceneSim,
    combat: &CombatState,
) -> [RawInput; 2] {
    let second = if args.lab_autopilot && sim.drive().two_player {
        combat_lab::autopilot::follow(sim)
    } else {
        RawInput::default()
    };
    [scripted_input(args, fixture, sim, combat), second]
}

/// The combat state lines of a fixture: the sim state, then the frames drawn.
fn print_combat_state(sim: &SceneSim, anim: &SceneAnimators, combat: &CombatState) {
    println!("{}", combat.state_line(sim));
    if let Some(art) = anim.art.as_ref() {
        println!("{}", art.state_line(sim.world.tick));
    }
}

/// Ticks stepped so far: over every attempt in a fixture (a restart brings
/// the scene tick back to 0), the scene tick otherwise.
fn elapsed_ticks(sim: &SceneSim, combat: Option<&CombatState>) -> u64 {
    combat.map_or(sim.world.tick, |combat| combat.elapsed)
}

/// Print the combat events recorded since the last call, queue their sounds
/// and hand them to CINE-01. This is the **single fan-out** of the combat log
/// (spec section 2): the log cursor is advanced here and nowhere else, so no
/// second cursor has to follow the 512-event drain. Everything it feeds is
/// presentation only: the audio queue and the cinematic observer are never
/// read back by the simulation.
///
/// Call sites: `fixed_step` (the respawn and the step, both pass the audio
/// queue and the observer), the `watchdog` (the events of the load, before any
/// tick) and `capture_sequence` (scripted review captures, which the spec
/// excludes from CINE) pass `None`.
fn print_combat_events(
    combat: &mut CombatState,
    mut audio: Option<&mut AudioQueue>,
    mut cine: Option<&mut CineAdapter>,
) -> Vec<ChainFeat> {
    let mut feats = Vec::new();
    // CF02-AUDIO (Q24, Q36): the cue overlay under cf02_rules, else the global map.
    let cf02 = combat.cf02();
    // CA-29: the heroes the companion drove on the tick these events came from.
    let driven = combat.companion_driven();
    for event in combat.take_new_events() {
        println!("{event}");
        if let Some(audio) = audio.as_deref_mut() {
            match cf02 {
                Some(tuning) => audio.push_cues(cues_from_combat_event_cf02(event, tuning)),
                None => audio.push_cues(cues_from_combat_event(event)),
            }
        }
        if let Some(cine) = cine.as_deref_mut() {
            cine.push_driven(event, driven);
        }
    }
    if let Some(cine) = cine {
        for emission in cine.flush() {
            // Deterministic: a function of the events only, printed with CINE
            // on or off (the planner outcome is not part of the line).
            println!("{}", cine_moment_line(&emission.moment));
            // AUDIENCE-01: the same moment as plain data for the audience
            // observer (CINE never reads audience data; one-way).
            feats.push(chain_feat(&emission.moment));
        }
    }
    feats
}

/// A CINE moment as the plain-data feat the audience observer takes.
fn chain_feat(m: &dungeon_floorbreak::cine::CinematicMoment) -> ChainFeat {
    ChainFeat {
        credited: m.credited,
        target: m.target,
        score: m.score,
    }
}

/// AUDIENCE-01: routes the CINE chain feats of a pre-simulated run (headless
/// replays and the fast-forward of every capture) into the audience. The live
/// loop gets them from the single fan-out in [`print_combat_events`]; this
/// feed reads the same events through its own cursor, so the combat log's
/// print cursor is untouched and nothing is printed. It keeps its own
/// [`CineAdapter`] (the evaluator is a function of the events only, whatever
/// the presentation setting), reads the log and returns nothing to the
/// simulation. Inert while the audience is locked.
#[derive(Default)]
struct AudienceFeatFeed {
    adapter: Option<CineAdapter>,
    cursor: EventCursor,
}

impl AudienceFeatFeed {
    /// One finished tick: call before [`step_hud`] (the live loop also queues
    /// the feats before its HUD step).
    fn observe(&mut self, combat: &CombatState, hud: &mut HudRuntime) {
        if !hud.audience().is_unlocked() {
            return;
        }
        let adapter = self
            .adapter
            .get_or_insert_with(|| CineAdapter::new(PresentationSettings::OFF));
        // CA-29: the CINE filter does not apply here. AUDIENCE credit stays
        // factual per hero (it never triggers a slow-motion moment).
        for event in self.cursor.next(&combat.events, combat.dropped_events) {
            adapter.push(event);
        }
        for emission in adapter.flush() {
            hud.push_chain_feat(chain_feat(&emission.moment));
        }
    }
}

/// The `CINE01_MOMENT` line of an emitted moment.
fn cine_moment_line(m: &dungeon_floorbreak::cine::CinematicMoment) -> String {
    format!(
        "CINE01_MOMENT tick={} tier={} score={} credited={} target={} precision_edge={} mixed_chain={} contact_setup={} extra_kills={}{}",
        m.tick,
        match m.tier {
            dungeon_floorbreak::cine::CineTier::Ordinary => "ordinary",
            dungeon_floorbreak::cine::CineTier::Legendary => "legendary",
        },
        m.score,
        m.credited,
        m.target,
        u8::from(m.feats.precision_edge),
        u8::from(m.feats.mixed_chain),
        u8::from(m.feats.contact_setup),
        m.feats.extra_kills,
        if m.upgrade { " upgrade=1" } else { "" },
    )
}

/// One `--anim-trace` row: simulation values the timeline needs, and what each
/// animator shows.
fn trace_line(sim: &SceneSim, anim: &SceneAnimators) -> String {
    let hero = &sim.world.hero;
    let donut = &sim.world.donut;
    format!(
        "FLOOR1_ANIM_TRACE tick={} hero_x={:.3} hero_foot_y={:.3} vx={:.3} vy={:.3} grounded={} donut_x={:.3} donut_foot_y={:.3} carl={} donut={}",
        sim.world.tick,
        hero.x,
        hero.y - HERO_HALF_HEIGHT,
        hero.vx,
        hero.vy,
        u8::from(hero.grounded),
        donut.x,
        donut.foot_y,
        anim.carl.label(),
        anim.donut.label(),
    )
}

fn failure(message: &str) -> ExitCode {
    println!("FLOOR1_ASSET_FAILURE: {message}");
    eprintln!("FLOOR1_ASSET_FAILURE: {message}");
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut args = match parse_args(&args) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("floor01_scene: {message}");
            return ExitCode::from(2);
        }
    };
    // A listing needs no scene, no assets and no window.
    if let Some(prefix) = &args.list_fixtures {
        for name in list_fixtures(prefix) {
            println!("{name}");
        }
        return ExitCode::SUCCESS;
    }
    if args.cf02.rules {
        match Cf02Tuning::pinned() {
            Ok(tuning) => println!(
                "{} schema_version={} tuning_hash={:#018x}",
                args.cf02.log_line(),
                tuning.schema_version,
                dungeon_floorbreak::cf02::TUNING_HASH
            ),
            Err(error) => {
                eprintln!("floor01_scene: {error}");
                return ExitCode::from(2);
            }
        }
    }
    if let UncontrolledHero::Companion(set) = args.uncontrolled_hero() {
        // The companion runs only in a live party run: a scripted run (the
        // autopilot, a scenario, a fixture) drives the other hero itself.
        let inactive = args.companion_inactive();
        println!(
            "COMPANION_CONFIG: set={}{}",
            set.name(),
            if inactive { " inactive" } else { "" }
        );
    }
    let root = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(error) => return failure(&format!("no working directory: {error}")),
    };
    // COMBAT-LAB-01: launch the room through the catalog (the leak guard sees
    // the entry point derived from the arguments).
    if args.lab_scene {
        let launched = LabCatalog::load(&root)
            .and_then(|catalog| catalog.launch(entry_point(&args), &lab_config(&args)));
        match launched {
            Ok(run) => {
                println!(
                    "LAB_ENTRY room={} variant={} mode={} profile={} seed={} rules=legacy view_h={}",
                    run.info.room,
                    run.info.variant,
                    run.info.mode.id(),
                    run.info.profile,
                    run.info.seed,
                    run.info.view_h
                );
                args.fixture = Some(run.info.fixture);
                args.lab_run = Some(Box::new(run));
            }
            Err(error) => {
                eprintln!("floor01_scene: {error}");
                return ExitCode::from(2);
            }
        }
    }
    // PTR-R1: open the recording before anything else; a recording the viewer
    // cannot show opens no window.
    let prepared = if args.replay.is_some() {
        match replay_view::prepare(&mut args, &root) {
            Ok(prepared) => Some(prepared),
            Err(unsupported) => return unsupported.report(),
        }
    } else {
        None
    };
    let kit = if let Some(prepared) = &prepared {
        Ok(prepared.kit.clone())
    } else if let Some(run) = &args.lab_run {
        Ok(run.kit.clone())
    } else if args.arena_room() {
        cf02::arena::load_arena(&root)
    } else if args.demo {
        demo::load_hall(&root)
    } else if args.boss {
        boss_scene::load_arena(&root)
    } else if let Some(room) = args.camera_test_room {
        test_room_kit(&root, room)
    } else {
        EnvKit::load(&root, LAYOUT_PATH).map_err(|error| error.to_string())
    };
    let kit = match kit {
        Ok(kit) => kit,
        Err(error) => return failure(&format!("{error} (run from the repository root)")),
    };
    // The windowed scene draws the exported atlases; the headless smoke needs
    // no export and reads the same frame data from the committed pixel grids.
    let source = if args.headless {
        FrameSource::GridSource
    } else {
        FrameSource::ExportedAtlas
    };
    // The blast atlases are exported art: only a rendered fixture with charges
    // needs them (the headless replay draws nothing).
    let draws_blast = !args.headless && args.fixture.is_some_and(|f| !f.charges.is_empty());
    // CF02-I: the CF02 sheets are read only by a run under `cf02_rules`.
    let runs_cf02 = args.cf02.rules
        || args
            .fixture
            .is_some_and(|f| dungeon_floorbreak::cf02::fixture_runs_cf02(f.name));
    let actors = match Actors::load(
        &root,
        source,
        args.fixture.is_some(),
        draws_blast,
        runs_cf02,
    ) {
        Ok(actors) => actors,
        Err(error) => {
            return failure(&format!(
                "{error} (run bash scripts/export_assets.sh from the repository root)"
            ));
        }
    };
    if args.headless {
        return match headless_smoke(&kit, &args, &actors, &root) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                println!("FLOOR1_HEADLESS_FAILURE: {message}");
                ExitCode::from(1)
            }
        };
    }
    let mut hud = new_hud(&args, &kit, &actors);
    let scene = match prepared {
        Some(prepared) => replay_view::scene_of(prepared, &args, &actors, &root, &mut hud).map(
            |(sim, anim, combat, death, party, view)| (sim, anim, combat, death, party, Some(view)),
        ),
        None => build_scene(&kit, &args, &actors, &root, &mut hud)
            .map(|(sim, anim, combat, death, party)| (sim, anim, combat, death, party, None)),
    };
    let (sim, anim, combat, death, party, replay) = match scene {
        Ok(scene) => scene,
        Err(message) => return failure(&message),
    };
    let is_replay = replay.is_some();
    let exit = run_window(
        kit, sim, anim, combat, death, party, actors, hud, args, &root, replay,
    );
    if is_replay {
        return replay_view::exit_code(&exit);
    }
    ExitCode::SUCCESS
}

// ------------------------------------------------------------------ resources

#[derive(Resource)]
struct Sim(SceneSim);

/// The boss atlas, when it is exported (HUD-DEBT-03); `None` draws placeholders.
#[derive(Resource)]
struct BossArtRes(Option<BossArt>);

/// One placeholder rect of the boss encounter (spec sizes, from the sim).
#[derive(Component, Clone, Copy)]
enum BossPiece {
    Body,
    Eye,
    Rack(usize),
    Strut,
    Gate,
    Ripple(usize),
}

/// One pooled sprite of the boss' art (the body, the racks, the effects): the
/// system assigns each tick's [`boss_art::draws`] to the pool in order.
#[derive(Component)]
struct BossFxSprite;

/// The texture and the atlas layout of each boss sheet, by [`SheetId`].
#[derive(Resource, Default)]
struct BossTextures([Option<(Handle<Image>, Handle<TextureAtlasLayout>)>; 6]);

/// Sprites in the boss art pool (the body, two racks with pulleys, the strut,
/// the ledge, shadows, up to four ripples, dust, crack and finish effects).
const BOSS_FX_POOL: usize = 40;

/// Presentation-only gait memory: whether the boss moved since the previous
/// simulated tick, observed once per tick (never per render frame).
#[derive(Default)]
struct WalkTrack {
    tick: u64,
    x: f32,
    walking: bool,
}

/// The boss sits just above Donut and below Carl.
const BOSS_Z_OFFSET: f32 = 0.06;
/// Boss art depth bands (relative to the actors' plane): the floor shadow and
/// pool over the floor, the pulley, strut, ledge, wall crack and roar ring
/// behind the actors, and the racks and effects of the body over the boss and
/// under Carl.
const BOSS_Z_FLOOR: f32 = -0.4;
const BOSS_Z_BACK: f32 = -0.2;
const BOSS_Z_FRONT: f32 = 0.075;

/// Asset path of the HUD atlas when its art is exported (the loading
/// watchdog waits for it like for every other atlas).
#[derive(Resource)]
struct HudAtlas(Option<&'static str>, Option<&'static str>);

/// Presentation-only animation state, stepped once per fixed tick from the
/// finished simulation state.
#[derive(Resource)]
struct Anim(SceneAnimators);

/// Carl's and Donut's frame data, read from the manifests and atlas JSON. A
/// combat fixture also loads the melee atlas and the two goblin atlases.
#[derive(Resource, Clone)]
struct Actors {
    carl: ActorAssets,
    donut: ActorAssets,
    art: Option<CombatArt>,
    /// The ENV-02 shore and block art of a combat fixture (DEATH-C).
    shore: Option<ShoreArt>,
    /// DEATH-D: the QA charge and blast frames of a fixture with charges
    /// (unreviewed prototype art, drawn in the QA fixtures only).
    blast: Option<BlastArt>,
    /// CF02-I: the ART-CARL, ART-DONUT and ART-GOB CF02 sheets of a run under
    /// `cf02_rules` (`None` everywhere else: no atlas is read or drawn).
    cf02: Option<ActingArt>,
}

#[derive(Clone)]
struct CombatArt {
    carl: SpriteClips,
    goblins: [SpriteClips; GOBLIN_VARIANTS],
    /// The Bad Llama atlas (LLAMA-01).
    llama: SpriteClips,
    /// ENEMY-GAMEPLAY-ENGINEER: the ENG-01 atlas, loaded for a `cf02_rules` run
    /// (the only runs that can have an Engineer).
    engineer: Option<SpriteClips>,
}

impl Actors {
    fn load(
        root: &Path,
        source: FrameSource,
        combat: bool,
        blast: bool,
        cf02: bool,
    ) -> Result<Self, String> {
        let blast = if blast {
            Some(BlastArt::load(root)?)
        } else {
            None
        };
        let shore = if combat {
            Some(ShoreArt::load(root)?)
        } else {
            None
        };
        let art = if combat {
            Some(CombatArt {
                carl: SpriteClips::load_carl_melee(root, source)?,
                goblins: [
                    SpriteClips::load_goblin(root, 0, source)?,
                    SpriteClips::load_goblin(root, 1, source)?,
                ],
                llama: SpriteClips::load_llama(root, source)?,
                engineer: if cf02 {
                    Some(SpriteClips::load_engineer(root, source)?)
                } else {
                    None
                },
            })
        } else {
            None
        };
        let cf02 = if combat && cf02 {
            Some(ActingArt::load(root, source)?)
        } else {
            None
        };
        Ok(Self {
            carl: load_carl(root, source)?,
            donut: load_donut(root, source)?,
            art,
            shore,
            blast,
            cf02,
        })
    }

    /// The melee and goblin animators of a combat fixture, with the CF02 acting
    /// frames and overlays when the run is under `cf02_rules`.
    fn combat_art(&self, combat: &CombatState) -> Option<CombatAnimators> {
        self.art.as_ref().map(|art| {
            let mut animators = CombatAnimators::new(art.carl.clone(), art.goblins.clone(), combat)
                .with_llama(art.llama.clone(), combat);
            if let Some(engineer) = art.engineer.as_ref() {
                animators = animators.with_engineer(engineer.clone(), combat);
            }
            match (self.cf02.as_ref(), combat.cf02()) {
                (Some(acting), Some(tuning)) => {
                    animators.with_acting(acting.clone(), tuning, combat)
                }
                _ => animators,
            }
        })
    }

    fn animators(&self, world: &CorridorWorld) -> SceneAnimators {
        SceneAnimators::new(
            Animator::new(self.carl.clips.clone(), self.carl.pacing),
            Animator::new(self.donut.clips.clone(), self.donut.pacing),
            world,
        )
    }
}

#[derive(Resource)]
struct Setup {
    kit: EnvKit,
    args: Args,
}

#[derive(Resource)]
struct Frozen(bool);

/// CINE-01: the cinematic observer and presentation clock (presentation only,
/// never read by the simulation). The planner is the sole owner of the
/// temporary `Time<Virtual>` speed override.
#[derive(Resource)]
struct Cine {
    adapter: CineAdapter,
    /// The relative speed last applied to the virtual clock.
    speed: f32,
    /// Measurements of the running moment, printed when it ends (evidence for
    /// CINE-T11 to T13: real length, fixed ticks run, the worst frame and the
    /// most fixed ticks any one frame ran).
    run: Option<MomentRun>,
    last_elapsed: u64,
}

#[derive(Debug, Clone, Copy)]
struct MomentRun {
    started_s: f64,
    ticks: u64,
    frames: u32,
    max_frame_ms: f64,
    max_ticks_in_frame: u64,
}

/// The two presentation sinks of the combat log (the 16-parameter system limit
/// of `fixed_step` is why they travel together).
#[derive(bevy::ecs::system::SystemParam)]
struct Sinks<'w> {
    audio: ResMut<'w, AudioQueue>,
    cine: ResMut<'w, Cine>,
    rec: ResMut<'w, Recorder>,
    /// PLAYTRACE-01 PT-C: read only, for the recorder's diagnostic rig view.
    cam: Res<'w, SceneCam>,
    /// COMBAT-LAB-01: the overlay (only a lab run has one).
    lab: Option<Res<'w, LabOverlay>>,
}

/// PLAYTRACE-01: the live session recorder (`None`: not recording). It only
/// reads the simulation; see `playtrace::live`.
#[derive(Resource)]
struct Recorder(Option<LiveRecorder>);

impl Recorder {
    /// Print the recorder's `PLAYTRACE_` lines (never part of any golden: they
    /// exist only in a recorded run).
    fn flush(&mut self) {
        if let Some(rec) = self.0.as_mut() {
            for line in rec.drain_messages() {
                println!("{line}");
            }
        }
    }
}

/// Fixed tick after which a live scripted run prints its state and exits.
#[derive(Resource)]
struct ExitAfter(Option<u64>);

/// Whether the lighting pass is in use (layout has a profile and the flag is
/// not set).
#[derive(Resource)]
struct LightingOn(bool);

/// Review-capture pose of the ENV-01 lit preview (`preview.actor_check` of the
/// sample layout): fixed camera, Carl and Donut standing at fixed x, first
/// frames, presentation tick 0. Pure presentation: the simulation is untouched.
#[derive(Resource)]
struct ReferencePose(Option<RefPose>);

#[derive(Clone, Copy)]
struct RefPose {
    carl_x: f32,
    donut_x: f32,
}

/// Preview actor positions of the sample layout (`preview.actor_check`, first
/// Carl and Donut pair); a test checks them against the layout file.
const REFERENCE_CARL_X: f32 = 392.0;
const REFERENCE_DONUT_X: f32 = 454.0;
/// Name of the layout view the preview uses.
const REFERENCE_VIEW: &str = "abc_640x360";
const REFERENCE_VIEW_DECK: &str = "abc_640x400";

/// Frame-interval probe for `--perf-frames`.
#[derive(Resource)]
struct Perf {
    target: usize,
    warmup: usize,
    seen: usize,
    intervals_ms: Vec<f32>,
    /// `--perf-from-tick`: one fixture tick per measured frame.
    step_ticks: bool,
    /// Per measured frame: (tick, ms, live parts, live floor decals, live wall
    /// decals, visible death sprites); printed with the stats.
    series: Vec<(u64, f32, usize, usize, usize, usize)>,
    peak_visible_death_sprites: usize,
    /// Per measured frame: summed GPU pass time (ms) and CPU time of the death
    /// presentation (the fixed tick of the fixture and the sprite sync, in us).
    gpu_ms: Vec<f32>,
    tick_us: Vec<f32>,
    death_sync_us: Vec<f32>,
}

/// Presentation ticks since the simulation stopped at the exit. The sim tick
/// stops there (so state hashes stay stable), but torches and idle frames keep
/// animating from `sim tick + this`. Never read by the simulation.
#[derive(Resource, Default)]
struct AfterExitTicks(u64);

#[derive(Resource)]
struct LiveScenario(Option<Scenario>);

/// Combat fixture state (COMBAT-INT-01); `None` in the normal story run, where
/// `attack` and `ability` stay ignored.
#[derive(Resource)]
struct Combat(Option<CombatState>);

/// DEATH-C: the cosmetic death presenter of a combat fixture (presentation
/// only: nothing in it is read by the simulation).
#[derive(Resource)]
struct Death(Option<DeathPresenter>);

/// DEMO-01 PR C: the presentation camera of the run, `None` for the scroll
/// camera (every existing capture). Presentation only: it reads the simulation
/// after the tick and nothing reads it back.
#[derive(Resource)]
struct SceneCam {
    rig: Option<SceneCameraRig>,
    /// Size in world pixels of the lighting slot targets: the widest view of
    /// the chosen camera (640 x 360 or 640 x 400 for the scroll camera).
    slot: (i32, i32),
    /// The rest width last printed as `FLOOR1_CAMERA_REST`.
    reported_rest: Option<i32>,
    /// CINE-01 camera emphasis in virtual pixels (signed, 0 outside a moment).
    /// It is added to the scene camera's **kick** input only (spec, review
    /// R7): never to its zoom or its pan, and `presented` still clamps the
    /// view to the room.
    emphasis: i32,
}

/// The read-only threat list gameplay feeds the camera (a llama that has taken
/// aim, its tell, a spit in flight). Empty in this PR unless `--camera-test-threat`
/// is given.
#[derive(Resource, Default)]
struct Interest(CameraInterest);

/// The fixture whose script drives the live run (not set for `--manual`).
#[derive(Resource)]
struct LiveFixture(Option<&'static Fixture>);

/// Keyboard taps latched between fixed steps (pad edges live in [`Pads`]).
#[derive(Resource, Default)]
struct Controls {
    jump_tap: bool,
    attack_tap: bool,
    physical_tap: bool,
    swap_tap: bool,
    restart: bool,
    /// DEMO-01 PR C: Carl was moved on purpose (a respawn, a restart): the
    /// presentation camera cuts to the new framing instead of panning.
    camera_cut: bool,
    /// DEMO-01: Select (R or the pad's Select) went down since the last tick.
    select_tap: bool,
    /// DEMO-01: a pad's Select is held (polled every frame).
    select_held: bool,
    select: SelectControl,
    /// DEMO-01: the furthest beat announced and whether the end was.
    beat_printed: Option<usize>,
    complete_printed: bool,
}

/// Local-player slot ownership (`pad_slots`): which pad drives P1 and P2, the
/// solo swap, the disconnect pause and the per-plate glyph families.
#[derive(Resource, Default)]
struct Pads(PadSlots);

#[derive(Resource)]
struct Loading {
    handles: Vec<(String, Handle<Image>)>,
    reported: bool,
    counts: String,
}

#[derive(Component)]
struct Parallax {
    strip: usize,
    copy: usize,
}

#[derive(Component)]
struct Torch {
    rects: Vec<Rect>,
    durations: Vec<u32>,
    offset: usize,
}

#[derive(Component)]
struct ActorVisual {
    is_hero: bool,
}

#[derive(Component)]
struct HeroBox;

/// One goblin, drawn from its variant's atlas (index parity: A, B, A, ...).
#[derive(Component)]
struct GoblinSprite {
    index: usize,
}

/// One Bad Llama (DEMO-01 PR B), drawn from the LLAMA-01 atlas.
#[derive(Component)]
struct LlamaSprite {
    index: usize,
}

/// One pooled spit glob sprite (the `spit` frames of the llama atlas), drawn
/// for the glob in slot `slot` of the simulation's list.
#[derive(Component)]
struct GlobSprite {
    slot: usize,
}

/// Carl's second sprite: the melee atlas (punches, kick, hurt, knockdown). It
/// replaces the locomotion sprite while a melee pose is drawn.
#[derive(Component)]
struct CarlMelee;

/// CF02-I: Carl's CF02 sprite (the ART-CARL-CF02 atlas): the stomp, air kick,
/// dive, bail-out, heavy landing, flinch and frustration. It replaces the
/// locomotion sprite while a CF02 frame is drawn. Presentation only.
#[derive(Component)]
struct CarlCf02Sprite;

/// CF02-I: Donut's startle and smug sprite (the ART-DONUT-CF02 atlas).
#[derive(Component)]
struct DonutCf02Sprite;

/// CF02-I: one goblin's acting sprite (hesitate, panic, taunt) from its
/// variant's ART-GOB-CF02 sheet; it replaces the GOB-01 sprite while shown.
#[derive(Component)]
struct GoblinActingSprite {
    index: usize,
}

/// CF02-I: one goblin's slammed finish sprite (stomped, wall crumple, flung).
#[derive(Component)]
struct GoblinSlammedSprite {
    index: usize,
}

/// Donut's melee sprite (the ANIM-DONUT-02 atlas): shown instead of her
/// locomotion sprite while a swat or pounce runs. Presentation only.
#[derive(Component)]
struct DonutMelee;

/// Donut's cast sprite (the ANIM-DONUT-03 head-swap atlas): shown instead of
/// her locomotion sprite during a Magic Missile cast and the dry cue.
#[derive(Component)]
struct DonutCast;

/// One pooled Magic Missile bolt sprite, drawn for the bolt in slot `slot` of
/// the simulation's list (the VFX atlas, or an 8 x 6 magenta rectangle when
/// the atlas is missing).
#[derive(Component)]
struct BoltSprite {
    slot: usize,
}

/// Bolt sprites in the pool (the cast cadence keeps at most three in flight).
const BOLT_SLOTS: usize = 8;

/// One pooled sprite of the death presenter (DEATH-C).
#[derive(Component, Clone, Copy)]
enum DeathPiece {
    /// The death pose of goblin `index` (replaces its GOB-01 sprite).
    Pose(usize),
    /// The death pose of llama `index` (replaces its LLAMA-01 sprite).
    LlamaPose(usize),
    /// A launched part or blood drop (pool slot).
    Part(usize),
    /// A floor decal; `second` is the right cell of a two-cell pool.
    Floor { slot: usize, second: bool },
    /// A back-wall spatter.
    Wall(usize),
}

/// DEATH-D: a sprite of the QA charge `n` of the fixture: the fizzling charge
/// until it detonates, then its blast burst. Presentation only.
#[derive(Component, Clone, Copy)]
enum BlastPiece {
    Charge(usize),
    Burst(usize),
}

/// The loaded `vfx_dynamite` and `vfx-01` atlas images.
#[derive(Resource)]
struct BlastImages {
    dynamite: Handle<Image>,
    burst: Handle<Image>,
}

/// One sprite of the ENV-02 shore (post, block, cavity, crumbs, impact dust).
#[derive(Component, Clone, Copy)]
struct ShorePiece(ShoreSlot);

/// One pooled impact-dust sprite of a llama slam (the ENV-02 block impact dust).
#[derive(Component, Clone, Copy)]
struct DustSprite {
    slot: usize,
}

/// Dust sprites per run: slams that overlap in time (four frames of 5 ticks).
const DUST_SLOTS: usize = 4;

/// The render layers the landed block switches between: `prop` normally, and
/// `actors` while a crushed enemy's contact frames say `hazard_draw: "over"`
/// (the prop slot is composed below the actors slot, so a z above the llama is
/// not enough: the stone has to join the actors layer).
#[derive(Resource)]
struct ShoreLayers {
    prop: RenderLayers,
    actors: RenderLayers,
}

/// The render layers of the falling block: the actors layer while the stone is
/// drawn over a crushed enemy, else the prop layer.
fn block_layers<'a>(
    over: bool,
    prop: &'a RenderLayers,
    actors: &'a RenderLayers,
) -> &'a RenderLayers {
    if over { actors } else { prop }
}

/// The loaded ENV-02 images, in the order of `ShoreArt::images`.
#[derive(Resource)]
struct ShoreImages(Vec<Handle<Image>>);

/// Geometry the death sprites need (set at startup).
#[derive(Resource)]
struct DeathLook {
    /// The composed sheets by death variant (goblin A and B, then the llama):
    /// death poses and parts. The parts layout differs per set.
    poses_images: Vec<Handle<Image>>,
    parts_images: Vec<Handle<Image>>,
    parts_layouts: Vec<Handle<TextureAtlasLayout>>,
    pose_spec: ActorSpec,
    /// The llama's death-pose frame (96 x 72, pivot (44, 71)).
    llama_pose_spec: ActorSpec,
    /// z of the `backwall_decal` layer.
    wall_z: f32,
}

/// `--debug-combat` overlay boxes (hit and hurt boxes; nothing else is drawn
/// outside the art).
#[derive(Component)]
enum CombatDebugBox {
    /// Carl's active hit box.
    HitBox,
    /// A goblin's 24 x 38 hurt box.
    Goblin(usize),
    /// A goblin's swing box while it is active.
    Swing(usize),
    /// Carl's 24 x 52 hurt box.
    CarlHurt,
    /// Donut's own hurtbox (Q4), shown while the party is live.
    DonutHurt,
    /// Donut's active swat or pounce box (drawn by [`sync_donut_visuals`]).
    DonutHit,
    /// The 8 x 6 hit box of the bolt in this pool slot (drawn by
    /// [`sync_donut_visuals`]).
    Bolt(usize),
}

/// The presentation camera of a run: the rig (none for the scroll camera) and
/// the lighting slot target size. The output width decides the automatic
/// camera's integer-scale rest widths.
fn scene_camera_of(
    kit: &EnvKit,
    args: &Args,
    sim: &SceneSim,
    combat: Option<&CombatState>,
    window: (u32, u32),
) -> SceneCam {
    let choice = args.camera_choice();
    let aspect = Aspect::from_output(window.0, window.1);
    let rig = SceneCameraRig::new(
        RigConfig {
            choice,
            aspect,
            out_w: window.0,
            bounds: Bounds {
                w: kit.width,
                h: kit.height,
            },
        },
        sim,
        combat,
        &test_interest(kit, args, sim, combat, elapsed_ticks(sim, combat)),
        elapsed_ticks(sim, combat),
    );
    // The slot targets cover the widest view this rig can show in this room at
    // this output (the scroll camera: 640, as always).
    let slot = rig
        .as_ref()
        .map_or_else(|| choice.slot_target(aspect, window.0), |r| r.slot_target());
    SceneCam {
        rig,
        slot,
        reported_rest: None,
        emphasis: 0,
    }
}

/// The threats of this tick: the synthetic one of `--camera-test-threat`, if
/// any, and the Bad Llamas and their globs from the finished simulation tick
/// (DEMO-01 PR B, [`llama_interest`]).
fn test_interest(
    kit: &EnvKit,
    args: &Args,
    sim: &SceneSim,
    combat: Option<&CombatState>,
    elapsed: u64,
) -> CameraInterest {
    let mut interest = match args.camera_test_threat {
        Some((x, first, last)) => synthetic_interest(
            x,
            to_world_y(kit.height, kit.base_walk_line_y),
            first,
            last,
            elapsed,
        ),
        None => CameraInterest::default(),
    };
    if let Some(combat) = combat {
        llama_interest(sim, combat, &mut interest);
    }
    interest
}

/// The view as drawn this frame (layout px, y down): the presentation camera
/// with the CP-01 kick scaled for its width, or the scroll camera with the kick
/// at 640 wide.
fn presentation_view(
    state: &SceneSim,
    setup: &Setup,
    reference: &ReferencePose,
    combat: &Combat,
    cam: &SceneCam,
) -> View {
    let kick = camera_kick(state, combat) + cam.emphasis;
    match cam.rig.as_ref() {
        Some(rig) => rig.presented(kick),
        None => {
            let (left, top) = view_origin(state, setup, reference, kick);
            View {
                left,
                top,
                w: VIRTUAL_WIDTH,
                h: view_height(setup.args.deck),
            }
        }
    }
}

/// The PLAYTRACE-01 session manifest of this run: identity and configuration
/// only, no path, host or user name.
fn recording_manifest(
    args: &Args,
    sim: &SceneSim,
    combat: Option<&CombatState>,
    party: &PartyState,
    root: &Path,
) -> SessionManifest {
    let (source, name) = if args.lab_scene {
        // COMBAT-LAB-01: a lab session is `combat_lab`, never `fixture`.
        (
            lab_flow::SESSION_SOURCE,
            args.fixture.map_or("", |f| f.name),
        )
    } else if args.demo {
        ("demo", args.fixture.map_or("", |f| f.name))
    } else if args.boss {
        ("boss", args.fixture.map_or("", |f| f.name))
    } else if let Some(fixture) = args.fixture {
        ("fixture", fixture.name)
    } else {
        ("story", "")
    };
    let kit_hash = dungeon_floorbreak::autoplay::replay::kit_content_hash(root).unwrap_or(0);
    let start_hash = combat.map_or_else(|| sim.state_hash(), |c| c.state_hash(sim));
    let reset = ResetDescriptor {
        source: source.to_owned(),
        name: name.to_owned(),
        demo: args.demo,
        demo_beat: args.demo_beat as u32,
        cf02_rules: args.cf02.rules,
        cf02_seed: args.cf02.seed,
        view_height: view_height(args.deck),
        donut_biscuit: args.donut_biscuit,
        carl_prog: args.carl.prog,
        carl_dynamite: args.carl.dynamite,
        carl_gauntlet: args.carl.gauntlet,
        kit_hash,
        start_hash,
    };
    let mut manifest = SessionManifest::new(playtrace::generate_session_id(), reset);
    playtrace::live::stamp_manifest(&mut manifest, kit_hash);
    manifest.seed_requested = args.cf02.seed;
    // A lab run knows its effective seed (R01: the variant's seed).
    manifest.seed_effective = args
        .lab_run
        .as_deref()
        .map_or(args.cf02.seed, |run| Some(run.info.seed));
    manifest.run_kind = if args.live_input() {
        playtrace::RunKind::Human
    } else {
        playtrace::RunKind::ScriptedBot
    };
    if args.record_other_tester {
        manifest.participants = playtrace::Participants::IncludesOtherTester;
    }
    // A live co-op lab run starts as a solo party until the second pad joins:
    // the chosen lab mode is the truth.
    let coop = match args.lab_run.as_deref() {
        Some(run) => run.info.mode == combat_lab::PlayerMode::CoopLocal,
        None => party.mode() == dungeon_floorbreak::party::PartyMode::TwoPlayer,
    };
    manifest.mode = if coop { "local_coop" } else { "solo_human" }.to_owned();
    if let Some(run) = args.lab_run.as_deref() {
        for (key, value) in [
            ("lab_room", run.info.room.clone()),
            ("lab_variant", run.info.variant.clone()),
            ("lab_mode", run.info.mode.id().to_owned()),
            ("lab_profile", run.info.profile.clone()),
        ] {
            manifest.feature_flags.insert(key.to_owned(), value);
        }
    }
    for (key, value) in [
        ("cf02_rules", args.cf02.rules),
        ("demo", args.demo),
        ("boss", args.boss),
        ("donut_biscuit", args.donut_biscuit),
        ("manual", args.manual),
        ("autopilot", args.autopilot),
    ] {
        manifest
            .feature_flags
            .insert(key.to_owned(), value.to_string());
    }
    // CARL-PROG-01: only a set flag is written, so a run without them records
    // exactly what it recorded before.
    for (key, value) in [
        ("carl_prog", args.carl.prog),
        ("carl_dynamite", args.carl.dynamite),
        ("carl_gauntlet", args.carl.gauntlet),
    ] {
        if value {
            manifest
                .feature_flags
                .insert(key.to_owned(), "true".to_owned());
        }
    }
    manifest
}

/// Start the PLAYTRACE-01 session of a windowed run, before the first tick.
/// Prints `PLAYTRACE_` lines only for a recorded run, a `--no-record` opt-out
/// or a failure, so every scripted run prints exactly what it printed before.
/// A failing recorder never stops the game unless `--record-strict` asks for it.
fn start_recording(
    args: &Args,
    sim: &SceneSim,
    combat: Option<&CombatState>,
    party: &PartyState,
    root: &Path,
) -> Option<LiveRecorder> {
    // Opt-outs are never silent for a run that would have been recorded.
    match args.record {
        RecordMode::Off => {
            println!("PLAYTRACE_DISABLED reason=--no-record this session is not recorded");
            return refuse_if_strict(args, "--no-record");
        }
        RecordMode::Auto => {
            if !(RECORD_DEFAULT_ON && args.live_input()) {
                // Scripted, headless and capture paths print nothing.
                return None;
            }
            if let Some(reason) = record_env_off() {
                println!(
                    "PLAYTRACE_DISABLED reason=env:{reason} this session is not recorded (use --record to force it)"
                );
                return refuse_if_strict(args, reason);
            }
        }
        RecordMode::On => {}
    }
    let base = match args.record_dir.clone().map(Ok).unwrap_or_else(|| {
        playtrace::default_record_dir(
            std::env::var("XDG_DATA_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
        )
    }) {
        Ok(base) => base,
        Err(error) => return recording_failed(args, &error.to_string()),
    };
    let size = playtrace::dir_size_bytes(&base);
    if size > playtrace::WARN_BYTES {
        println!(
            "PLAYTRACE_WARN the playtrace directory holds {} GB; nothing is deleted automatically (use `playtrace delete`)",
            size >> 30
        );
    }
    let manifest = recording_manifest(args, sim, combat, party, root);
    // The location is printed relative to the data directory, never as an
    // absolute home path (console logs get pasted into evidence).
    let location = match args.record_dir.as_ref() {
        Some(dir) => format!("{}/{}", dir.display(), manifest.session_id),
        None if std::env::var("XDG_DATA_HOME").is_ok_and(|v| v.starts_with('/')) => {
            format!(
                "$XDG_DATA_HOME/dungeon-floorbreak/playtrace/{}",
                manifest.session_id
            )
        }
        None => format!(
            "~/.local/share/dungeon-floorbreak/playtrace/{}",
            manifest.session_id
        ),
    };
    match LiveRecorder::start(&base, manifest, sim, combat, party, !args.live_input()) {
        Ok(mut recorder) => {
            for line in recorder.drain_messages() {
                println!("{line} dir={location}");
            }
            Some(recorder)
        }
        Err(error) => recording_failed(args, &error.to_string()),
    }
}

/// Why the environment turns automatic recording off: `CI` (set by GitHub
/// Actions and many harnesses) or `FLOORBREAK_RECORD` being `0`/`off` (the CI
/// scripts export it). A live-input run says so on stdout (`PLAYTRACE_DISABLED
/// reason=env:...`); scripted runs never reach this check and print nothing.
/// `--record` still forces recording on.
fn record_env_off() -> Option<&'static str> {
    if std::env::var("FLOORBREAK_RECORD").is_ok_and(|v| v == "0" || v.eq_ignore_ascii_case("off")) {
        return Some("FLOORBREAK_RECORD");
    }
    std::env::var("CI")
        .is_ok_and(|v| !v.is_empty() && v != "0")
        .then_some("CI")
}

/// `--record-strict` refuses to run unrecorded, whatever the reason.
fn refuse_if_strict(args: &Args, why: &str) -> Option<LiveRecorder> {
    if args.record_strict {
        eprintln!("floor01_scene: --record-strict: refusing to start unrecorded ({why})");
        std::process::exit(2);
    }
    None
}

fn recording_failed(args: &Args, why: &str) -> Option<LiveRecorder> {
    println!("PLAYTRACE_ERROR the session is NOT being recorded: {why}");
    if args.record_strict {
        eprintln!("floor01_scene: --record-strict: refusing to start without a recording");
        std::process::exit(2);
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn run_window(
    kit: EnvKit,
    sim: SceneSim,
    anim: SceneAnimators,
    combat: Option<CombatState>,
    death: Option<DeathPresenter>,
    party: PartyState,
    actors: Actors,
    hud: HudRuntime,
    args: Args,
    root: &Path,
    replay: Option<replay_view::ReplayView>,
) -> AppExit {
    // HUD-DEBT-03: the boss atlas when its manifest and export exist; the
    // scene draws spec-size placeholder rects otherwise. Wrong art is fatal.
    let boss_art = match BossArt::load(root) {
        Ok(art) => art,
        Err(error) => {
            println!("FLOOR1_ASSET_FAILURE: {error}");
            std::process::exit(1);
        }
    };
    // Bevy 0.20 resolves a relative AssetPlugin path from the binary directory,
    // so give it an absolute directory below the repository root.
    let asset_directory = root.join("generated/art").to_string_lossy().into_owned();
    let window_size = (
        VIRTUAL_WIDTH as u32 * args.scale,
        view_height(args.deck) as u32 * args.scale,
    );
    let capture = args
        .capture_sequence
        .zip(args.capture_dir.clone())
        .map(|((first, step, count), dir)| CaptureSequence::new(first, step, count, dir));
    // A replay never records: the recorder is forced off.
    let recorder = if replay.is_some() {
        None
    } else {
        start_recording(&args, &sim, combat.as_ref(), &party, root)
    };
    let freeze = args.freeze
        || args.capture_tick.is_some()
        || args.reference_frame
        || capture.is_some()
        || args.perf_from_tick.is_some();
    let scenario = if freeze { None } else { args.scenario };
    let live_fixture = if freeze || args.manual || replay.is_some() {
        None
    } else {
        args.fixture
    };
    let exit_after = args.exit_after_ticks;
    let lab_plugin = match lab_overlay_plugin(&args, root) {
        Ok(plugin) => plugin,
        Err(message) => {
            println!("FLOOR1_ASSET_FAILURE: {message}");
            eprintln!("FLOOR1_ASSET_FAILURE: {message}");
            std::process::exit(1);
        }
    };
    let cine_settings = args.cinematic_settings();
    let camera = scene_camera_of(&kit, &args, &sim, combat.as_ref(), window_size);
    let lighting_on = !args.no_lighting && kit.lighting.is_some();
    let reference = args.reference_frame.then_some(RefPose {
        carl_x: REFERENCE_CARL_X,
        donut_x: REFERENCE_DONUT_X,
    });
    let present_mode = if args.perf_frames.is_some() {
        PresentMode::AutoNoVsync
    } else {
        PresentMode::AutoVsync
    };
    let perf = args.perf_frames.map(|target| Perf {
        target,
        warmup: 120,
        seen: 0,
        intervals_ms: Vec::with_capacity(target),
        step_ticks: args.perf_from_tick.is_some(),
        series: Vec::with_capacity(target),
        peak_visible_death_sprites: 0,
        gpu_ms: Vec::with_capacity(target),
        tick_us: Vec::with_capacity(target),
        death_sync_us: Vec::with_capacity(target),
    });
    // AUDIO-01: sound is presentation only. Screenshot, reference and perf runs
    // stay silent and open no device, like `--no-audio` and `FLOORBREAK_AUDIO=off`.
    let mut audio_settings = AudioSettings::from_env();
    if let Some(style) = args.audio_style {
        audio_settings.style = style;
    }
    audio_settings.enabled &= !args.no_audio && !freeze && args.perf_frames.is_none();
    let audio_initial = vec![
        AudioCommand::Music(scene_music(combat.is_some())),
        AudioCommand::Ambience(scene_ambience()),
    ];
    // The HUD camera draws after the last scene camera: one above the debug
    // overlay when lit (`first_post + 4`), one above the plain camera otherwise.
    let view_h = view_height(args.deck);
    // The unlit scene camera keeps Bevy's default sample count; the lit cameras
    // are all `Msaa::Off`. The HUD camera must match its target's cameras.
    let hud_msaa = if lighting_on {
        Msaa::Off
    } else {
        Msaa::default()
    };
    let hud_order = match kit.lighting.as_ref().filter(|_| lighting_on) {
        Some(lighting) => lighting.plan.slots.len() as isize + 5,
        None => 1,
    };
    let view_key = if args.deck {
        REFERENCE_VIEW_DECK
    } else {
        REFERENCE_VIEW
    };
    let Some(view) = kit.view(view_key) else {
        let message = format!("layout has no view {view_key} for the HUD");
        println!("FLOOR1_ASSET_FAILURE: {message}");
        eprintln!("FLOOR1_ASSET_FAILURE: {message}");
        std::process::exit(1);
    };
    let view_top = view.y;
    let hud_layout = HudLayout::new(view_h, kit.base_walk_line_y - view_top);
    let hud_plugin = match HudPlugin::load(root, hud, hud_layout, hud_order, hud_msaa) {
        Ok(plugin) => plugin.hidden(args.reference_frame),
        Err(message) => {
            println!("FLOOR1_ASSET_FAILURE: {message}");
            eprintln!("FLOOR1_ASSET_FAILURE: {message}");
            std::process::exit(1);
        }
    };
    let hud_atlas = hud_plugin.atlas_asset();
    let hud_emblem = hud_plugin.emblem_asset();
    let mut plugins = DefaultPlugins
        .set(ImagePlugin::default_nearest())
        .set(AssetPlugin {
            file_path: asset_directory,
            ..default()
        });
    if !audio_settings.enabled {
        plugins = plugins.disable::<bevy::audio::AudioPlugin>();
    }
    let mut app = App::new();
    app.add_plugins(
        plugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: TITLE.into(),
                resolution: WindowResolution::new(window_size.0, window_size.1)
                    // ADR-0007: window = virtual size x scale in real pixels;
                    // an OS scale factor must not change the integer ratio.
                    .with_scale_factor_override(1.0),
                // ADR-0007: integer scaling only; the window is exactly
                // virtual size x scale (letterboxing is deferred).
                resizable: false,
                present_mode,
                ..default()
            }),
            ..default()
        }),
    )
    .add_plugins(FloorAudioPlugin {
        settings: audio_settings,
        pack_dir: resolve_pack_dir(),
        initial: audio_initial,
    })
    .add_plugins(hud_plugin)
    .insert_resource(ClearColor(Color::srgb_u8(6, 5, 12)))
    .insert_resource(Time::<Fixed>::from_hz(60.0))
    .insert_resource(Sim(sim))
    .insert_resource(Anim(anim))
    .insert_resource(actors)
    .insert_resource(Frozen(freeze))
    .insert_resource(Cine {
        adapter: CineAdapter::new(cine_settings),
        speed: 1.0,
        run: None,
        last_elapsed: 0,
    })
    .insert_resource(ExitAfter(exit_after))
    .insert_resource(LightingOn(lighting_on))
    .insert_resource(ReferencePose(reference))
    .insert_resource(LiveScenario(scenario))
    .insert_resource(Combat(combat))
    .insert_resource(BossArtRes(boss_art))
    .insert_resource(Death(death))
    .insert_resource(camera)
    .init_resource::<Interest>()
    .insert_resource(LiveFixture(live_fixture))
    .insert_resource(HudAtlas(hud_atlas, hud_emblem))
    .insert_resource(Setup { kit, args })
    .init_resource::<Controls>()
    .insert_resource(Pads(PadSlots::new(party)))
    .insert_resource(Recorder(recorder))
    .init_resource::<AfterExitTicks>()
    .add_systems(Startup, setup)
    .add_systems(Last, finish_recording)
    .add_systems(
        Update,
        (
            cine_frame,
            perf_step,
            sync_visuals,
            sync_combat_visuals,
            sync_donut_visuals,
            sync_boss_visuals,
            sync_llama_visuals,
            sync_dust_visuals,
            sync_shore_visuals,
            sync_blast_visuals,
            sync_death_visuals,
            sync_cf02_acting_visuals,
            update_lighting,
            watchdog,
            perf_probe,
            capture_sequence,
        )
            .chain(),
    );
    if let Some(view) = replay {
        // PTR-R1: the replay viewer. No pad, no keyboard gameplay, no live
        // `fixed_step`, no `sync_party_hud`: the cursor drives the tick, the
        // recorded drive drives the HUD, and only the viewer's own keys are read.
        app.insert_resource(view)
            .add_systems(Startup, replay_view::spawn_overlay)
            .add_systems(
                Update,
                (
                    replay_view::replay_keys.before(cine_frame),
                    replay_view::replay_hud.after(cine_frame).before(perf_step),
                    replay_view::replay_capture.after(sync_visuals),
                    replay_view::draw_overlay.after(replay_view::replay_hud),
                ),
            )
            // After Bevy's window-exit systems: closing with the window X writes
            // `AppExit` in `Last`, and the result line must see it that frame.
            .add_systems(
                Last,
                replay_view::replay_finish.after(bevy::window::ExitSystems),
            )
            .add_systems(
                FixedUpdate,
                (replay_view::replay_step, replay_view::replay_camera).chain(),
            );
    } else {
        // The live input systems keep their order: pads, input, CINE, party HUD,
        // then the rest of the chain above.
        app.add_systems(
            Update,
            (pad_connections, capture_input).chain().before(cine_frame),
        )
        .add_systems(Update, sync_party_hud.after(cine_frame).before(perf_step))
        .add_systems(FixedUpdate, (fixed_step, update_scene_camera).chain());
    }
    if lighting_on {
        app.add_plugins(LightPassPlugin);
    }
    if let Some(lab) = lab_plugin {
        app.add_plugins(lab)
            .add_systems(FixedUpdate, lab_step.after(fixed_step));
    }
    if let Some(perf) = perf {
        // GPU pass timings (Vulkan timestamp queries) for the probe's report.
        app.add_plugins(RenderDiagnosticsPlugin);
        app.insert_resource(perf);
    }
    if let Some(capture) = capture {
        app.insert_resource(capture);
    }
    app.run()
}

// ---------------------------------------------------------------------- setup

fn image_rect(r: IRect) -> Rect {
    Rect::new(
        r.x as f32,
        r.y as f32,
        (r.x + r.w) as f32,
        (r.y + r.h) as f32,
    )
}

fn atlas_sprite(image: &Handle<Image>, rect: IRect) -> Sprite {
    Sprite {
        image: image.clone(),
        rect: Some(image_rect(rect)),
        ..default()
    }
}

#[allow(clippy::too_many_arguments)]
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Option<ResMut<Assets<LightingMaterial>>>,
    lighting_on: Res<LightingOn>,
    setup: Res<Setup>,
    actors: Res<Actors>,
    combat: Res<Combat>,
    death: Res<Death>,
    hud_atlas: Res<HudAtlas>,
    boss_art: Res<BossArtRes>,
    cam: Res<SceneCam>,
) {
    let kit = &setup.kit;
    let args = &setup.args;
    let height = kit.height;
    let view_h = view_height(args.deck);
    // The lighting slot targets and the parallax copies cover the widest view
    // of the chosen camera (the scroll camera: 640 wide, as always).
    let (slot_w, slot_h) = cam.slot;
    let lighting = kit.lighting.as_ref().filter(|_| lighting_on.0);
    // Render layer of a contract layer: its lighting slot, or the default
    // layer when the pass is off.
    let layers_of = |layer: &str| -> RenderLayers {
        lighting
            .and_then(|l| l.plan.layer_slot.get(layer))
            .map_or_else(RenderLayers::default, |slot| slot_layer(*slot))
    };

    let mut handles: Vec<(String, Handle<Image>)> = Vec::new();
    for (name, asset) in &kit.atlases {
        handles.push((name.clone(), asset_server.load(asset.clone())));
    }
    if let Some(asset) = hud_atlas.0 {
        handles.push((asset.to_owned(), asset_server.load(asset)));
    }
    if let Some(asset) = hud_atlas.1 {
        handles.push((asset.to_owned(), asset_server.load(asset)));
    }
    let atlas = |name: &str| -> Handle<Image> {
        handles
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, h)| h.clone())
            .expect("atlas named by the contract")
    };
    let carl_texture: Handle<Image> = asset_server.load(actors.carl.atlas_asset.clone());
    let donut_texture: Handle<Image> = asset_server.load(actors.donut.atlas_asset.clone());

    if let (Some(lighting), Some(mut materials)) = (lighting, materials) {
        // Lighting on: slot cameras render the pixel layers at virtual
        // resolution, the post stages light them at window resolution.
        let rig = spawn_rig(
            &mut commands,
            &mut images,
            &mut meshes,
            &mut materials,
            lighting,
            UVec2::new(slot_w as u32, slot_h as u32),
        );
        commands.insert_resource(rig);
    } else {
        // Lighting off: the COMP-01 camera draws the sprites straight to the
        // window.
        commands.spawn((
            Camera2d,
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: VIRTUAL_WIDTH as f32,
                    height: view_h as f32,
                },
                ..OrthographicProjection::default_2d()
            }),
            Transform::from_xyz(0.0, 0.0, 0.0),
            SceneCamera,
        ));
    }

    // Tiles: one sprite per non-empty cell, top-left anchored on whole pixels.
    for tile in &kit.tiles {
        commands.spawn((
            atlas_sprite(&atlas(&tile.atlas), tile.rect),
            Anchor::TOP_LEFT,
            Transform::from_xyz(
                (tile.col * kit.tile) as f32,
                to_world_y(height, tile.row * kit.tile),
                tile.z,
            ),
            layers_of(&tile.layer),
        ));
    }

    // Placed pieces (decals, structure, props, foreground).
    for piece in &kit.pieces {
        let mut entity = commands.spawn((
            atlas_sprite(&atlas(&piece.atlas), piece.rect),
            Anchor::TOP_LEFT,
            Transform::from_xyz(piece.x as f32, to_world_y(height, piece.y), piece.z),
            layers_of(&piece.layer),
        ));
        if piece.frames.len() > 1 {
            entity.insert(torch_component(piece));
        }
    }

    // Parallax strips: enough copies to cover the view at any camera offset.
    let mut strip_copies = 0usize;
    for (strip_index, strip) in kit.strips.iter().enumerate() {
        let copies = (slot_w / strip.rect.w + 2) as usize;
        for copy in 0..copies {
            commands.spawn((
                atlas_sprite(&atlas(&strip.atlas), strip.rect),
                Anchor::TOP_LEFT,
                Transform::from_xyz(0.0, to_world_y(height, strip.y), strip.z),
                Parallax {
                    strip: strip_index,
                    copy,
                },
                layers_of(&strip.layer),
            ));
            strip_copies += 1;
        }
    }

    // Actors at the contract's actor draw order; Carl in front of Donut.
    let mut spawn_actor = |assets: &ActorAssets, texture: &Handle<Image>, is_hero: bool, z: f32| {
        // One row of `frame_count` equally sized, unpadded frames.
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(assets.spec.width as u32, assets.spec.height as u32),
            assets.clips.frame_count as u32,
            1,
            None,
            None,
        ));
        commands.spawn((
            ActorVisual { is_hero },
            Sprite::from_atlas_image(
                texture.clone(),
                TextureAtlas {
                    layout,
                    index: assets.clips.clip(Tag::Idle).first,
                },
            ),
            Transform::from_xyz(0.0, 0.0, z),
            layers_of("actors"),
        ));
    };
    spawn_actor(&actors.donut, &donut_texture, false, kit.actor_z);
    spawn_actor(
        &actors.carl,
        &carl_texture,
        true,
        kit.actor_z + CARL_Z_OFFSET,
    );

    // Combat fixture only: Carl's melee sprite, one sprite per goblin (variant A
    // or B by index parity) and the optional debug boxes.
    let mut combat_entities = 0usize;
    let mut combat_handles: Vec<(String, Handle<Image>)> = Vec::new();
    if let (Some(state), Some(art)) = (combat.0.as_ref(), actors.art.as_ref()) {
        let mut layout_of = |clips: &SpriteClips| {
            layouts.add(TextureAtlasLayout::from_grid(
                UVec2::new(clips.spec.width as u32, clips.spec.height as u32),
                clips.frame_count as u32,
                1,
                None,
                None,
            ))
        };
        let carl_melee_texture: Handle<Image> = asset_server.load(art.carl.atlas_asset.clone());
        commands.spawn((
            CarlMelee,
            Sprite::from_atlas_image(
                carl_melee_texture.clone(),
                TextureAtlas {
                    layout: layout_of(&art.carl),
                    index: art.carl.tag("punch_a").0,
                },
            ),
            Transform::from_xyz(0.0, 0.0, kit.actor_z + CARL_Z_OFFSET),
            Visibility::Hidden,
            layers_of("actors"),
        ));
        combat_handles.push((art.carl.atlas_asset.clone(), carl_melee_texture));
        combat_entities += 1;
        let goblin_textures: Vec<(Handle<Image>, Handle<TextureAtlasLayout>)> = art
            .goblins
            .iter()
            .map(|clips| {
                let texture: Handle<Image> = asset_server.load(clips.atlas_asset.clone());
                combat_handles.push((clips.atlas_asset.clone(), texture.clone()));
                (texture, layout_of(clips))
            })
            .collect();
        // ENEMY-GAMEPLAY-ENGINEER: the Engineer is drawn from the ENG-01 atlas.
        let engineer_texture: Option<(Handle<Image>, Handle<TextureAtlasLayout>)> =
            art.engineer.as_ref().map(|clips| {
                let texture: Handle<Image> = asset_server.load(clips.atlas_asset.clone());
                combat_handles.push((clips.atlas_asset.clone(), texture.clone()));
                (texture, layout_of(clips))
            });
        for (index, enemy) in state.enemies.iter().enumerate() {
            let variant = goblin_variant(index);
            let (texture, layout, first) = match (&engineer_texture, art.engineer.as_ref()) {
                (Some((texture, layout)), Some(clips)) if enemy.is_engineer() => {
                    (texture.clone(), layout.clone(), clips.tag("idle").0)
                }
                _ => {
                    let (texture, layout) = goblin_textures[variant].clone();
                    (texture, layout, art.goblins[variant].tag("idle").0)
                }
            };
            commands.spawn((
                GoblinSprite { index },
                Sprite::from_atlas_image(
                    texture,
                    TextureAtlas {
                        layout,
                        index: first,
                    },
                ),
                Transform::from_xyz(
                    0.0,
                    0.0,
                    kit.actor_z + GOBLIN_Z_OFFSET - index as f32 * GOBLIN_Z_STEP,
                ),
                layers_of("actors"),
            ));
            combat_entities += 1;
        }
        // CF02-I: the CF02 sprites (hidden until a CF02 frame is drawn).
        if let (Some(acting), Some(_)) = (actors.cf02.as_ref(), state.cf02()) {
            let mut sprite = |clips: &SpriteClips, texture: &Handle<Image>, first: &str| {
                Sprite::from_atlas_image(
                    texture.clone(),
                    TextureAtlas {
                        layout: layout_of(clips),
                        index: clips.tag(first).0,
                    },
                )
            };
            let mut load = |clips: &SpriteClips| -> Handle<Image> {
                let texture: Handle<Image> = asset_server.load(clips.atlas_asset.clone());
                combat_handles.push((clips.atlas_asset.clone(), texture.clone()));
                texture
            };
            let carl_texture = load(&acting.carl);
            commands.spawn((
                CarlCf02Sprite,
                sprite(&acting.carl, &carl_texture, "stomp"),
                Transform::from_xyz(0.0, 0.0, kit.actor_z + CARL_Z_OFFSET),
                Visibility::Hidden,
                layers_of("actors"),
            ));
            let donut_texture = load(&acting.donut);
            commands.spawn((
                DonutCf02Sprite,
                sprite(&acting.donut, &donut_texture, "startle"),
                Transform::from_xyz(0.0, 0.0, kit.actor_z),
                Visibility::Hidden,
                layers_of("actors"),
            ));
            let acting_textures: Vec<Handle<Image>> =
                acting.goblins.iter().map(&mut load).collect();
            let slammed_textures: Vec<Handle<Image>> =
                acting.slammed.iter().map(&mut load).collect();
            for index in 0..state.enemies.len() {
                let variant = goblin_variant(index);
                let z = kit.actor_z + GOBLIN_Z_OFFSET - index as f32 * GOBLIN_Z_STEP;
                commands.spawn((
                    GoblinActingSprite { index },
                    sprite(
                        &acting.goblins[variant],
                        &acting_textures[variant],
                        "hesitate",
                    ),
                    Transform::from_xyz(0.0, 0.0, z),
                    Visibility::Hidden,
                    layers_of("actors"),
                ));
                commands.spawn((
                    GoblinSlammedSprite { index },
                    sprite(
                        &acting.slammed[variant],
                        &slammed_textures[variant],
                        "slammed_stomped",
                    ),
                    Transform::from_xyz(0.0, 0.0, z),
                    Visibility::Hidden,
                    layers_of("actors"),
                ));
                combat_entities += 2;
            }
            combat_entities += 2;
        }
        // The Bad Llamas (DEMO-01 PR B) and their pooled glob sprites.
        if !state.llamas.is_empty() {
            let clips = &art.llama;
            let texture: Handle<Image> = asset_server.load(clips.atlas_asset.clone());
            combat_handles.push((clips.atlas_asset.clone(), texture.clone()));
            let layout = layout_of(clips);
            for index in 0..state.llamas.len() {
                commands.spawn((
                    LlamaSprite { index },
                    Sprite::from_atlas_image(
                        texture.clone(),
                        TextureAtlas {
                            layout: layout.clone(),
                            index: clips.tag("idle").0,
                        },
                    ),
                    Transform::from_xyz(
                        0.0,
                        0.0,
                        kit.actor_z + GOBLIN_Z_OFFSET
                            - (state.enemies.len() + index) as f32 * GOBLIN_Z_STEP,
                    ),
                    layers_of("actors"),
                ));
                combat_entities += 1;
            }
            for slot in 0..state.llamas.len() * GLOB_SLOTS_PER_LLAMA {
                commands.spawn((
                    GlobSprite { slot },
                    Sprite::from_atlas_image(
                        texture.clone(),
                        TextureAtlas {
                            layout: layout.clone(),
                            index: clips.tag("spit").0,
                        },
                    ),
                    Transform::from_xyz(0.0, 0.0, kit.actor_z + GLOB_Z_OFFSET),
                    Visibility::Hidden,
                    layers_of("actors"),
                ));
                combat_entities += 1;
            }
        }
        if args.debug_combat {
            let debug_layers = if lighting.is_some() {
                RenderLayers::layer(LAYER_DEBUG)
            } else {
                RenderLayers::default()
            };
            let z = kit.foreground_z + 1.0;
            let mut overlay = |kind: CombatDebugBox, colour: Color, size: Vec2| {
                commands.spawn((
                    kind,
                    Sprite::from_color(colour, size),
                    Transform::from_xyz(0.0, 0.0, z),
                    Visibility::Hidden,
                    debug_layers.clone(),
                ));
                combat_entities += 1;
            };
            overlay(
                CombatDebugBox::HitBox,
                Color::srgba(1.0, 0.1, 0.1, 0.55),
                Vec2::ONE,
            );
            overlay(
                CombatDebugBox::CarlHurt,
                Color::srgba(0.3, 0.5, 1.0, 0.3),
                Vec2::new(HERO_HALF_WIDTH * 2.0, HERO_HALF_HEIGHT * 2.0),
            );
            overlay(
                CombatDebugBox::DonutHurt,
                Color::srgba(1.0, 0.5, 0.9, 0.35),
                Vec2::ONE,
            );
            for index in 0..state.enemies.len() {
                overlay(
                    CombatDebugBox::Goblin(index),
                    Color::srgba(0.2, 1.0, 0.3, 0.3),
                    Vec2::new(ENEMY_HALF_WIDTH * 2.0, ENEMY_HALF_HEIGHT * 2.0),
                );
                overlay(
                    CombatDebugBox::Swing(index),
                    Color::srgba(1.0, 0.6, 0.1, 0.55),
                    Vec2::ONE,
                );
            }
        }
    }

    // Donut's melee, cast and bolt sprites (presentation only; a missing atlas
    // keeps her locomotion sprite and draws the bolts as magenta rectangles).
    if combat.0.is_some() {
        let debug_layers = if lighting.is_some() {
            RenderLayers::layer(LAYER_DEBUG)
        } else {
            RenderLayers::default()
        };
        combat_handles.extend(spawn_donut_art(
            &mut commands,
            &asset_server,
            &mut layouts,
            &DonutArtLayers {
                actors: layers_of("actors"),
                debug: debug_layers,
            },
            kit.actor_z,
            kit.foreground_z + 1.0,
            args.debug_combat,
        ));
    }

    // HUD-DEBT-03: the boss, its racks, strut, gate and ripples. Placeholder
    // rects at the spec sizes; the atlas sprite replaces the body when the boss
    // art exists (`boss_art`).
    if combat.0.as_ref().is_some_and(|c| c.boss.is_some()) {
        let z = kit.actor_z + BOSS_Z_OFFSET;
        let mut piece = |kind: BossPiece, colour: Color| {
            commands.spawn((
                kind,
                Sprite::from_color(colour, Vec2::ONE),
                Transform::from_xyz(0.0, 0.0, z),
                Visibility::Hidden,
                layers_of("actors"),
            ));
            combat_entities += 1;
        };
        piece(BossPiece::Body, Color::srgb(0.42, 0.1, 0.34));
        piece(BossPiece::Eye, Color::srgb(0.95, 0.9, 0.8));
        for index in 0..2 {
            piece(BossPiece::Rack(index), Color::srgb(0.3, 0.3, 0.34));
        }
        piece(BossPiece::Strut, Color::srgb(0.42, 0.28, 0.14));
        piece(BossPiece::Gate, Color::srgb(0.2, 0.17, 0.2));
        for slot in 0..4 {
            piece(BossPiece::Ripple(slot), Color::srgb(0.95, 0.7, 0.2));
        }
        if let Some(art) = boss_art.0.as_ref() {
            let mut textures = BossTextures::default();
            for sheet in art.sheets() {
                let texture: Handle<Image> = asset_server.load(sheet.asset.clone());
                let layout = layouts.add(TextureAtlasLayout::from_grid(
                    UVec2::new(sheet.frame_size.0 as u32, sheet.frame_size.1 as u32),
                    sheet.count as u32,
                    1,
                    None,
                    None,
                ));
                combat_handles.push((sheet.asset.clone(), texture.clone()));
                textures.0[sheet.id as usize] = Some((texture, layout));
            }
            if let Some((texture, layout)) = textures.0[SheetId::Base as usize].clone() {
                for _ in 0..BOSS_FX_POOL {
                    commands.spawn((
                        BossFxSprite,
                        Sprite::from_atlas_image(
                            texture.clone(),
                            TextureAtlas {
                                layout: layout.clone(),
                                index: 0,
                            },
                        ),
                        Transform::from_xyz(0.0, 0.0, z + 0.01),
                        Visibility::Hidden,
                        layers_of("actors"),
                    ));
                    combat_entities += 1;
                }
            }
            commands.insert_resource(textures);
        }
    }

    // DEATH-C: the ENV-02 shore, block and impact dust of a fixture with a
    // shore (presentation only, non-collidable, driven by the simulated state).
    // The slam of a llama reuses the same impact dust, so a fixture with
    // llamas loads the shore images too.
    let has_shore = combat.0.as_ref().is_some_and(|c| c.shore.is_some());
    let has_llamas = combat.0.as_ref().is_some_and(|c| !c.llamas.is_empty());
    if let (Some(shore_art), true) = (actors.shore.as_ref(), has_shore || has_llamas) {
        let images: Vec<Handle<Image>> = shore_art
            .images
            .iter()
            .map(|name| {
                let handle: Handle<Image> = asset_server.load(name.clone());
                combat_handles.push((name.clone(), handle.clone()));
                handle
            })
            .collect();
        for slot in ShoreSlot::ALL.into_iter().filter(|_| has_shore) {
            let layer = if slot == ShoreSlot::Cavity {
                layers_of("backwall_decal")
            } else {
                layers_of("prop")
            };
            commands.spawn((
                ShorePiece(slot),
                Sprite {
                    image: images[0].clone(),
                    rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, kit.actor_z - 1.0),
                Visibility::Hidden,
                layer,
            ));
            combat_entities += 1;
        }
        // The block is drawn on the actors layer (over a crushed llama) or on
        // the prop layer: both render layers are kept to switch between.
        commands.insert_resource(ShoreLayers {
            prop: layers_of("prop"),
            actors: layers_of("actors"),
        });
        if has_llamas {
            for slot in 0..DUST_SLOTS {
                commands.spawn((
                    DustSprite { slot },
                    Sprite {
                        image: images[0].clone(),
                        rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.0, kit.actor_z - 1.0),
                    Visibility::Hidden,
                    layers_of("actors"),
                ));
                combat_entities += 1;
            }
        }
        commands.insert_resource(ShoreImages(images));
    }

    // DEATH-D: the QA charge (the fizzle loop) and its blast burst, one pair of
    // pre-spawned sprites per charge of the fixture.
    if let (Some(blast_art), Some(state)) = (actors.blast.as_ref(), combat.0.as_ref()) {
        let _ = blast_art;
        let dynamite: Handle<Image> = asset_server.load(DYNAMITE_IMAGE);
        let burst: Handle<Image> = asset_server.load(BURST_IMAGE);
        combat_handles.push((DYNAMITE_IMAGE.to_owned(), dynamite.clone()));
        combat_handles.push((BURST_IMAGE.to_owned(), burst.clone()));
        for index in 0..state.charges.len() {
            for (kind, image) in [
                (BlastPiece::Charge(index), &dynamite),
                (BlastPiece::Burst(index), &burst),
            ] {
                commands.spawn((
                    kind,
                    Sprite {
                        image: image.clone(),
                        rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.0, kit.actor_z - 1.0),
                    Visibility::Hidden,
                    layers_of("actors"),
                ));
                combat_entities += 1;
            }
        }
        commands.insert_resource(BlastImages { dynamite, burst });
    }

    // DEATH-C: the death presenter's sprites, pre-spawned hidden and recycled
    // one to one with its fixed pools (nothing is spawned at runtime).
    if let Some(presenter) = death.0.as_ref() {
        let sheets = presenter.sheets();
        // One image per goblin variant (A, B): the layout is shared (the same
        // frame and cell sizes, checked when the presenter is built).
        let mut image_of = |sheet: &Sheet| -> Handle<Image> {
            images.add(Image::new(
                Extent3d {
                    width: sheet.width() as u32,
                    height: sheet.cell_h as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                sheet.rgba.clone(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            ))
        };
        // Goblin A, goblin B and (with the llama art) the llama: one sheet each.
        let variants = if presenter.has_llama() {
            DEATH_VARIANTS + 1
        } else {
            DEATH_VARIANTS
        };
        let poses_images: Vec<Handle<Image>> = (0..variants)
            .map(|v| image_of(&presenter.sheets_of(v).poses))
            .collect();
        let parts_images: Vec<Handle<Image>> = (0..variants)
            .map(|v| image_of(&presenter.sheets_of(v).parts))
            .collect();
        let (poses_image, parts_image) = (poses_images[0].clone(), parts_images[0].clone());
        let mut layout_of = |sheet: &Sheet| {
            layouts.add(TextureAtlasLayout::from_grid(
                UVec2::new(sheet.cell_w as u32, sheet.cell_h as u32),
                sheet.frames as u32,
                1,
                None,
                None,
            ))
        };
        let poses_layout = layout_of(&sheets.poses);
        let parts_layout = layout_of(&sheets.parts);
        // The parts layout of each set (the llama has more parts than a goblin).
        let parts_layouts: Vec<Handle<TextureAtlasLayout>> = (0..variants)
            .map(|v| layout_of(&presenter.sheets_of(v).parts))
            .collect();
        let sprite_of = |image: &Handle<Image>, layout: &Handle<TextureAtlasLayout>| {
            Sprite::from_atlas_image(
                image.clone(),
                TextureAtlas {
                    layout: layout.clone(),
                    index: 0,
                },
            )
        };
        let mut piece = |kind: DeathPiece, sprite: Sprite, layer: RenderLayers| {
            commands.spawn((
                kind,
                sprite,
                Transform::from_xyz(0.0, 0.0, kit.actor_z - 1.0),
                Visibility::Hidden,
                layer,
            ));
            combat_entities += 1;
        };
        for index in 0..combat.0.as_ref().map_or(0, |c| c.enemies.len()) {
            piece(
                DeathPiece::Pose(index),
                sprite_of(&poses_image, &poses_layout),
                layers_of("actors"),
            );
        }
        if presenter.has_llama() {
            let llama_poses = &presenter.sheets_of(LLAMA_VARIANT).poses;
            let llama_layout = layout_of(llama_poses);
            for index in 0..combat.0.as_ref().map_or(0, |c| c.llamas.len()) {
                piece(
                    DeathPiece::LlamaPose(index),
                    sprite_of(&poses_images[LLAMA_VARIANT], &llama_layout),
                    layers_of("actors"),
                );
            }
        }
        for slot in 0..PART_POOL_SLOTS {
            piece(
                DeathPiece::Part(slot),
                sprite_of(&parts_image, &parts_layout),
                layers_of("actors"),
            );
        }
        for slot in 0..FLOOR_DECAL_POOL_SLOTS {
            for second in [false, true] {
                piece(
                    DeathPiece::Floor { slot, second },
                    sprite_of(&parts_image, &parts_layout),
                    layers_of("actors"),
                );
            }
        }
        for slot in 0..WALL_DECAL_POOL_SLOTS {
            piece(
                DeathPiece::Wall(slot),
                sprite_of(&parts_image, &parts_layout),
                layers_of("backwall_decal"),
            );
        }
        let wall_z = kit
            .pieces
            .iter()
            .find(|p| p.layer == "backwall_decal")
            .map_or(3.0, |p| p.z.floor());
        commands.insert_resource(DeathLook {
            poses_images,
            parts_images,
            parts_layouts,
            pose_spec: ActorSpec {
                name: "goblin_death",
                width: presenter.art().pose_frame_size.0,
                height: presenter.art().pose_frame_size.1,
                pivot_x: presenter.art().pose_pivot.0,
                pivot_y: presenter.art().pose_pivot.1,
                shadow_width: 0,
                shadow_pivot_x: 0,
            },
            llama_pose_spec: {
                let art = presenter.art_of(LLAMA_VARIANT);
                ActorSpec {
                    name: "llama_death",
                    width: art.pose_frame_size.0,
                    height: art.pose_frame_size.1,
                    pivot_x: art.pose_pivot.0,
                    pivot_y: art.pose_pivot.1,
                    shadow_width: 0,
                    shadow_pivot_x: 0,
                }
            },
            wall_z,
        });
    }

    // Optional collision overlay: merged solids and walking lines.
    let mut debug_entities = 0usize;
    if args.debug_collision {
        let overlay_z = kit.foreground_z + 1.0;
        let debug_layers = if lighting.is_some() {
            RenderLayers::layer(LAYER_DEBUG)
        } else {
            RenderLayers::default()
        };
        for rect in &kit.solid_rects {
            commands.spawn((
                Sprite::from_color(
                    Color::srgba(0.95, 0.15, 0.2, 0.38),
                    Vec2::new(rect.w as f32, rect.h as f32),
                ),
                Anchor::TOP_LEFT,
                Transform::from_xyz(rect.x as f32, to_world_y(height, rect.y), overlay_z),
                debug_layers.clone(),
            ));
            debug_entities += 1;
        }
        // Walking lines: runs of columns that share a standing surface.
        let mut run: Option<(i32, i32)> = None; // (start x, line y)
        for x in 0..=kit.width {
            let line = if x < kit.width {
                kit.walking_line_at(x)
            } else {
                None
            };
            if let Some((start, y)) = run
                && line != Some(y)
            {
                commands.spawn((
                    Sprite::from_color(
                        Color::srgb(0.2, 1.0, 0.95),
                        Vec2::new((x - start) as f32, 1.0),
                    ),
                    Anchor::TOP_LEFT,
                    Transform::from_xyz(start as f32, to_world_y(height, y), overlay_z + 0.1),
                    debug_layers.clone(),
                ));
                debug_entities += 1;
                run = None;
            }
            if run.is_none()
                && let Some(y) = line
            {
                run = Some((x, y));
            }
        }
        commands.spawn((
            HeroBox,
            Sprite::from_color(
                Color::srgba(0.3, 1.0, 0.3, 0.4),
                Vec2::new(HERO_HALF_WIDTH * 2.0, HERO_HALF_HEIGHT * 2.0),
            ),
            Transform::from_xyz(0.0, 0.0, overlay_z + 0.2),
            debug_layers.clone(),
        ));
        debug_entities += 1;
    }

    handles.push((actors.carl.atlas_asset.clone(), carl_texture));
    handles.push((actors.donut.atlas_asset.clone(), donut_texture));
    handles.extend(combat_handles);
    let tiles = kit.tiles.len();
    let pieces = kit.pieces.len();
    let counts = format!(
        "tiles={tiles} pieces={pieces} parallax_copies={strip_copies} actors=2 debug={debug_entities} sprites={} solids={}",
        tiles + pieces + strip_copies + 2 + debug_entities + combat_entities,
        kit.solid_rects.len()
    );
    println!("FLOOR1_ENTITY_COUNTS {counts}");
    match lighting {
        Some(l) => println!(
            "FLOOR1_LIGHTING on: profile={} slots={} emitters={}",
            l.profile.profile_id,
            l.plan.slots.len(),
            l.emitters.len()
        ),
        None => println!("FLOOR1_LIGHTING off"),
    }
    commands.insert_resource(Loading {
        handles,
        reported: false,
        counts,
    });
}

fn torch_component(piece: &PieceDraw) -> Torch {
    Torch {
        rects: piece.frames.iter().map(|(r, _)| image_rect(*r)).collect(),
        durations: piece.frames.iter().map(|(_, d)| *d).collect(),
        offset: piece.frame_offset,
    }
}

// -------------------------------------------------------------------- systems

/// The opaque pad identity `pad_slots` keys on: the Bevy gamepad entity.
fn pad_id(entity: Entity) -> PadId {
    PadId(entity.to_bits())
}

/// Route gamepad connection messages to the slot owner table (D5/D6): the
/// first pad is P1, a later pad waits for Start, an owning pad that goes away
/// pauses the fixed step until a pad reclaims its slot. Presentation and input
/// routing only; the vendor id comes from `GamepadConnection::Connected`
/// (bevy_input 0.20.1, filled from the OS by bevy_gilrs).
fn pad_connections(mut events: MessageReader<GamepadEvent>, mut pads: ResMut<Pads>) {
    for event in events.read() {
        if let GamepadEvent::Connection(connection) = event {
            let pad = pad_id(connection.gamepad);
            match &connection.connection {
                GamepadConnection::Connected { vendor_id, .. } => {
                    pads.0.apply(PadEvent::Connected {
                        pad,
                        vendor_id: *vendor_id,
                    });
                }
                GamepadConnection::Disconnected => {
                    pads.0.apply(PadEvent::Disconnected { pad });
                }
            }
        }
    }
}

/// CF02-Q01: Down is the D-pad down, or the left stick down to at least 0.5 with
/// the vertical axis dominant (`abs(y) >= abs(x)`), so a diagonal run is not a
/// stomp. Sampled once per frame; the simulation reads it on the press tick.
fn pad_down(dpad_y: f32, stick: Vec2) -> bool {
    dpad_y <= -0.5 || (stick.y <= -0.5 && stick.y.abs() >= stick.x.abs())
}

/// One pad's state for this frame, by logical action.
fn pad_frame(entity: Entity, pad: &Gamepad) -> PadFrame {
    let stick = pad.left_stick().x;
    PadFrame {
        pad: pad_id(entity),
        vendor_id: pad.vendor_id(),
        move_x: if stick.abs() > 0.18 {
            stick
        } else {
            pad.dpad().x
        },
        down: pad_down(pad.dpad().y, pad.left_stick()),
        held: PadButtons {
            jump: pad.pressed(GamepadButton::South),
            attack: pad.pressed(GamepadButton::West),
            // PhysicalAttack is Triangle/Y (North); North no longer restarts.
            physical: pad.pressed(GamepadButton::North),
            swap: pad.pressed(GamepadButton::LeftTrigger),
            restart: pad.pressed(GamepadButton::Select),
            start: pad.pressed(GamepadButton::Start),
        },
        pressed: PadButtons {
            jump: pad.just_pressed(GamepadButton::South),
            attack: pad.just_pressed(GamepadButton::West),
            physical: pad.just_pressed(GamepadButton::North),
            swap: pad.just_pressed(GamepadButton::LeftTrigger),
            restart: pad.just_pressed(GamepadButton::Select),
            start: pad.just_pressed(GamepadButton::Start),
        },
    }
}

fn capture_input(
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut controls: ResMut<Controls>,
    mut pads: ResMut<Pads>,
    mut exit: MessageWriter<AppExit>,
    mut lab: Option<ResMut<LabOverlay>>,
) {
    controls.jump_tap |= keys.just_pressed(KeyCode::Space);
    controls.attack_tap |= keys.just_pressed(KeyCode::KeyJ);
    controls.physical_tap |= keys.just_pressed(KeyCode::KeyK);
    // Temporary keyboard default for the solo swap (the contract only fixes
    // L1/LB); the keyboard stays P1.
    controls.swap_tap |= keys.just_pressed(KeyCode::KeyQ);
    controls.restart |= keys.just_pressed(KeyCode::KeyR);
    controls.select_tap |= keys.just_pressed(KeyCode::KeyR);
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    // Each pad is polled on its own and routed by `pad_slots`: the owner of
    // P1 or P2 drives its hero, any other pad is ignored except for Start
    // (which joins P2). A pad that vanishes (hot-unplug) leaves the query, so
    // nothing sticks.
    controls.select_held = false;
    for (entity, pad) in &gamepads {
        controls.select_held |= pad.pressed(GamepadButton::Select);
        let commands = pads.0.sample(pad_frame(entity, pad));
        controls.restart |= commands.restart;
        controls.select_tap |= commands.restart;
        if commands.exit {
            // The lab's panels use Start as Confirm (the pad-only owner reads
            // START on the menu row): it never quits while one is open.
            if !lab.as_mut().is_some_and(|lab| lab.pad_start()) {
                exit.write(AppExit::Success);
            }
        }
    }
}

/// Push the real party state to the HUD every frame: who drives each plate,
/// each plate's own pad glyph family (HUD-DEBT-05 per slot, provisional Q11)
/// and the reconnect request of a disconnect pause. `set_party` bumps the HUD
/// revision only on a change, and this runs in `Update` so the message shows
/// while the fixed step is paused. Presentation only.
fn sync_party_hud(pads: Res<Pads>, mut hud: ResMut<HudState>) {
    hud.0.set_party(pads.0.hud_view());
}

/// The controlled hero is in a hit reaction (blocks the solo swap, Q8): Carl's
/// or Donut's own hurt state.
fn controlled_hero_hurt(party: &PartyState, combat: Option<&CombatState>) -> bool {
    combat.is_some_and(|c| c.hurt_ticks_of(party.controlled_hero()) > 0)
}

/// Ticks of hit-stop still to run on the attackers and targets of the fight
/// (Carl's move clock, goblins, llamas). CINE-01 waits for them before it
/// starts, counted in simulation ticks, so the moment never stacks a second
/// freeze on the lethal contact's own hit-stop.
fn hit_stop_left(sim: &SceneSim, combat: &CombatState) -> u8 {
    let goblins = combat.enemies.iter().map(|g| g.hit_stop);
    let llamas = combat.llamas.iter().map(|l| l.hit_stop);
    goblins
        .chain(llamas)
        .fold(sim.world.hero.melee.hit_stop_left(), u8::max)
}

/// CINE-01: one frame of the presentation clock. Runs in `Update` after the
/// fixed ticks of the frame: it steps the planner with the real (unscaled)
/// delta, applies the planned speed to `Time<Virtual>` for the next frame and
/// the camera emphasis to the scene camera's kick. The fixed timestep itself is
/// untouched (`Time<Fixed>` stays at 60 Hz), so a slowed frame simply runs
/// fewer ticks; the tick delta, the step order and every outcome are the same.
///
/// The moment is cancelled and 1.0x restored at once on the D6 disconnect
/// pause and on a `Frozen` capture (the fixed step returns early there while
/// the virtual clock keeps running, so a real-time duration would expire
/// unseen), on a restart, respawn or party defeat (the adapter flags those)
/// and on app exit.
#[allow(clippy::too_many_arguments)]
fn cine_frame(
    real: Res<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut cine: ResMut<Cine>,
    mut cam: ResMut<SceneCam>,
    sim: Res<Sim>,
    combat: Res<Combat>,
    death: Res<Death>,
    pads: Res<Pads>,
    frozen: Res<Frozen>,
    mut exit: MessageReader<AppExit>,
) {
    let exiting = exit.read().count() > 0;
    let moment = cine.adapter.moment_target();
    let finish_active = match (moment, death.0.as_ref()) {
        (Some(target), Some(death)) => death.finish_active(target),
        _ => false,
    };
    let ctx = FrameContext {
        real_dt: real.delta_secs_f64(),
        hit_stop_left: combat
            .0
            .as_ref()
            .map_or(0, |combat| hit_stop_left(&sim.0, combat)),
        finish_active,
        paused: pads.0.is_paused(),
        locked: frozen.0 || exiting,
    };
    let plan = cine.adapter.frame(ctx);
    let elapsed = combat.0.as_ref().map_or(sim.0.world.tick, |c| c.elapsed);
    let ticks_this_frame = elapsed.saturating_sub(cine.last_elapsed);
    cine.last_elapsed = elapsed;
    if plan.state == PlannerState::Active && cine.run.is_none() {
        cine.run = Some(MomentRun {
            started_s: real.elapsed_secs_f64(),
            ticks: 0,
            frames: 0,
            max_frame_ms: 0.0,
            max_ticks_in_frame: 0,
        });
        println!("CINE01_SLOWMO start tick_elapsed={elapsed}");
    }
    if let Some(run) = cine.run.as_mut() {
        run.ticks += ticks_this_frame;
        run.frames += 1;
        run.max_frame_ms = run.max_frame_ms.max(ctx.real_dt * 1000.0);
        run.max_ticks_in_frame = run.max_ticks_in_frame.max(ticks_this_frame);
    }
    if let Some(reason) = plan.ended {
        if let Some(run) = cine.run.take() {
            println!(
                "CINE01_SLOWMO end={reason:?} real_ms={:.0} ticks={} frames={} max_frame_ms={:.1} max_ticks_in_frame={}",
                (real.elapsed_secs_f64() - run.started_s) * 1000.0,
                run.ticks,
                run.frames,
                run.max_frame_ms,
                run.max_ticks_in_frame,
            );
        } else {
            println!("CINE01_SLOWMO end={reason:?}");
        }
    }
    if (plan.relative_speed - cine.speed).abs() > f32::EPSILON {
        virtual_time.set_relative_speed(plan.relative_speed);
        cine.speed = plan.relative_speed;
    }
    let facing = i32::from(sim.0.world.hero.facing.signum());
    cam.emphasis = i32::from(plan.camera_emphasis_px) * if facing == 0 { 1 } else { facing };
}

#[allow(clippy::too_many_arguments)]
fn fixed_step(
    keys: Res<ButtonInput<KeyCode>>,
    mut pads: ResMut<Pads>,
    frozen: Res<Frozen>,
    exit_after: Res<ExitAfter>,
    scenario: Res<LiveScenario>,
    fixture: Res<LiveFixture>,
    mut combat: ResMut<Combat>,
    mut death: ResMut<Death>,
    setup: Res<Setup>,
    mut controls: ResMut<Controls>,
    mut sim: ResMut<Sim>,
    mut anim: ResMut<Anim>,
    mut after_exit: ResMut<AfterExitTicks>,
    mut sinks: Sinks,
    mut hud: ResMut<HudState>,
    mut exit: MessageWriter<AppExit>,
) {
    let mut restart = std::mem::take(&mut controls.restart);
    let select_tap = std::mem::take(&mut controls.select_tap);
    let jump_tap = std::mem::take(&mut controls.jump_tap);
    let attack_tap = std::mem::take(&mut controls.attack_tap);
    let physical_tap = std::mem::take(&mut controls.physical_tap);
    let swap_tap = std::mem::take(&mut controls.swap_tap);
    // Keyboard is P1. Pad edges are consumed here too, so a tap made during a
    // pause is dropped and nothing fires on resume.
    let keyboard = {
        let left = keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft);
        let right = keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight);
        RawInput {
            move_x: f32::from(i8::from(right) - i8::from(left)),
            jump: keys.pressed(KeyCode::Space) || jump_tap,
            attack: keys.pressed(KeyCode::KeyJ) || attack_tap,
            physical: keys.pressed(KeyCode::KeyK) || physical_tap,
            swap: swap_tap,
            // CF02-Q01: Down (S or the Down arrow). Only Carl's Physical press
            // under `--cf02` reads it.
            down: keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown),
            ..RawInput::NONE
        }
    };
    let mut live_slots = pads.0.take_inputs(keyboard);
    // The lab menu and the end panel own the input: gameplay ignores it.
    if sinks.lab.as_ref().is_some_and(|lab| lab.blocks_gameplay()) {
        live_slots = [RawInput::NONE; 2];
    }
    // PLAYTRACE-01: pad lifecycle and the D6 pause happen between steps, so
    // they are observed before any early return.
    if let Some(rec) = sinks.rec.0.as_mut() {
        rec.observe_party(pads.0.party());
    }
    if frozen.0 {
        return;
    }
    // D6: an owning pad is gone; the sim waits for it (the HUD says so).
    if pads.0.is_paused() {
        return;
    }
    if let Some(limit) = exit_after.0
        && elapsed_ticks(&sim.0, combat.0.as_ref()) >= limit
    {
        // Already reported; further fixed steps of this frame do nothing.
        return;
    }
    // DEMO-01 live run: Select tap respawns, Select held for a second restarts
    // the demo (R on the keyboard is the same button).
    let mut respawn = false;
    if (setup.args.demo || setup.args.lab_scene) && setup.args.manual {
        restart = false;
        let down = keys.pressed(KeyCode::KeyR) || controls.select_held || select_tap;
        match controls.select.tick(down) {
            SelectAction::Respawn => respawn = true,
            SelectAction::Restart => restart = true,
            SelectAction::None => {}
        }
    }
    if respawn && let Some(combat) = combat.0.as_mut() {
        combat.respawn(&mut sim.0);
        let _ = combat.take_respawned();
        controls.camera_cut = true;
        pads.0.party_mut().restart();
        lab_reseat(&setup.args, pads.0.party_mut());
        if let Some(rec) = sinks.rec.0.as_mut() {
            rec.note_demo_respawn(&sim.0, Some(&*combat), pads.0.party());
        }
        anim.0.reset(&sim.0.world);
        anim.0.reset_art(combat);
        let feats = print_combat_events(
            combat,
            Some(&mut sinks.audio),
            Some(&mut sinks.cine.adapter),
        );
        for feat in feats {
            hud.0.push_chain_feat(feat);
        }
        println!("DEMO01_RESPAWN checkpoint={}", combat.checkpoint);
    }
    if restart {
        match combat.0.as_mut() {
            Some(combat) => combat.restart(&mut sim.0),
            None => sim.0.reset(),
        }
        controls.beat_printed = None;
        controls.complete_printed = false;
        anim.0.reset(&sim.0.world);
        if let Some(combat) = combat.0.as_ref() {
            anim.0.reset_art(combat);
        }
        if let Some(death) = death.0.as_mut() {
            death.clear();
        }
        hud.0.restart();
        pads.0.party_mut().restart();
        lab_reseat(&setup.args, pads.0.party_mut());
        if let Some(rec) = sinks.rec.0.as_mut() {
            rec.note_restart(&sim.0, combat.0.as_ref(), pads.0.party());
        }
        controls.camera_cut = true;
        after_exit.0 = 0;
        println!("FLOOR1_RESTART");
    }
    // Melee exists only in a combat fixture (`--fixture ... --manual`).
    if combat.0.is_none() {
        for slot in &mut live_slots {
            slot.attack = false;
            slot.physical = false;
        }
    }
    let slot_inputs = if let Some(scenario) = scenario.0 {
        [scenario.input(&sim.0.world), RawInput::default()]
    } else if let Some(fixture) = fixture.0 {
        let raw = match combat.0.as_ref() {
            Some(combat) => scripted_input(&setup.args, fixture, &sim.0, combat),
            None => fixture.input(sim.0.world.tick + 1),
        };
        let second = if setup.args.lab_autopilot && sim.0.drive().two_player {
            combat_lab::autopilot::follow(&sim.0)
        } else {
            RawInput::default()
        };
        [raw, second]
    } else {
        live_slots
    };
    // Solo swap (L1/LB, Q on the keyboard), then slot -> hero routing. With no
    // pad and no swap this is the identity on Carl, so scripted runs and
    // replays are unchanged.
    let hurt = controlled_hero_hurt(pads.0.party(), combat.0.as_ref());
    pads.0.party_mut().request_swap(slot_inputs[0].swap, hurt);
    let hero_inputs = pads.0.party().hero_inputs(slot_inputs);
    // A live party (Donut is a real hero, a bot or P2) exists only in
    // interactive play; scripted scenario and fixture runs keep the legacy
    // single-hero simulation, so every recorded hash is unchanged.
    let live_party = scenario.0.is_none() && fixture.0.is_none();
    sim.0.set_drive(pads.0.party().drive(live_party));
    if let Some(rec) = sinks.rec.0.as_mut() {
        // The presentation rig's view before this tick: a diagnostic field of
        // the privileged stream. The observation uses the sim camera only.
        rec.set_rig_view(sinks.cam.rig.as_ref().map(|rig| {
            let view = rig.view();
            [view.left, view.top, view.w, view.h]
        }));
        rec.begin_tick(
            &sim.0,
            combat.0.as_ref(),
            pads.0.party(),
            slot_inputs,
            hero_inputs,
            pads.0.party().drive(live_party),
        );
    }
    let was_complete = sim.0.world.complete;
    let mut respawned_in_step = false;
    if let Some(combat) = combat.0.as_mut() {
        let restarts = combat.restarts;
        let respawned = step_combat(
            &mut sim.0,
            combat,
            &mut anim.0,
            death.0.as_mut(),
            hero_inputs,
        );
        respawned_in_step = respawned;
        if combat.restarts != restarts || respawned {
            // A party defeat restarted the attempt (or the demo respawned
            // Carl): Carl is controlled again (a lab mode seats its hero).
            pads.0.party_mut().restart();
            lab_reseat(&setup.args, pads.0.party_mut());
            controls.camera_cut = true;
        }
        let feats = print_combat_events(
            combat,
            Some(&mut sinks.audio),
            Some(&mut sinks.cine.adapter),
        );
        for feat in feats {
            hud.0.push_chain_feat(feat);
        }
        if respawned {
            println!("DEMO01_RESPAWN checkpoint={}", combat.checkpoint);
        }
        if setup.args.cf02.rules {
            // CF02-F: the room is one beat; the exit opens when all six enemies
            // are defeated (console only, presentation).
            if cf02::arena::complete(combat) && !controls.complete_printed {
                controls.complete_printed = true;
                println!("CF02_ARENA_COMPLETE tick={}", sim.0.world.tick);
            }
        } else if setup.args.demo {
            // Console progress of the demo (presentation only).
            // The first tick only records the beat the run started at.
            let printed = controls.beat_printed.get_or_insert(combat.checkpoint);
            while *printed < combat.checkpoint {
                *printed += 1;
                let beat = demo::BEATS[*printed];
                println!("DEMO01_BEAT {} {}", beat.id, beat.name);
            }
            if demo::complete(combat) && !controls.complete_printed {
                controls.complete_printed = true;
                println!("DEMO01_COMPLETE tick={}", sim.0.world.tick);
            }
        }
        if setup.args.death_stress
            && let Some(death) = death.0.as_mut()
        {
            // Lab: keep the pools loaded (presentation only).
            let hero = &sim.0.world.hero;
            death.stress_tick(hero.x, hero.y - HERO_HALF_HEIGHT);
        }
    } else {
        sim.0.step_party(hero_inputs);
        anim.0.step(&sim.0.world);
    }
    if let Some(rec) = sinks.rec.0.as_mut() {
        rec.end_tick(&sim.0, combat.0.as_ref(), pads.0.party(), respawned_in_step);
    }
    sinks.rec.flush();
    step_hud(&mut hud.0, &sim.0, combat.0.as_ref());
    if sim.0.world.complete && !was_complete {
        println!("FLOOR1_EXIT_REACHED tick={}", sim.0.world.tick);
    }
    if was_complete {
        after_exit.0 += 1;
    }
    if let Some(limit) = exit_after.0
        && elapsed_ticks(&sim.0, combat.0.as_ref()) >= limit
    {
        println!("{}", sim.0.state_line());
        println!("{}", anim.0.state_line(sim.0.world.tick));
        if let Some(combat) = combat.0.as_ref() {
            print_combat_state(&sim.0, &anim.0, combat);
        }
        print_audience_state(&hud.0);
        if let Some(death) = death.0.as_ref() {
            println!("{}", death.state_line());
        }
        exit.write(AppExit::Success);
    }
}

/// PLAYTRACE-01: close the session when the app exits (an `AppExit` message of
/// this frame). A session that never gets here (a crash) is recovered by the
/// reader as `crashed_recovered`; a clean drop is marked `abandoned`.
fn finish_recording(mut exit: MessageReader<AppExit>, mut rec: ResMut<Recorder>) {
    if exit.read().next().is_none() {
        return;
    }
    if let Some(recorder) = rec.0.as_mut() {
        recorder.finish("app_quit");
    }
    rec.flush();
}

/// DEMO-01 PR C: one presentation camera step for the tick the simulation just
/// finished. Reads the simulation and the interest list through shared
/// references only; nothing flows back. Called after every simulated tick: by
/// the fixed schedule (right after `fixed_step`), and inside the loops that step
/// the simulation by hand (`--capture-sequence`, the `--perf-from-tick` probe),
/// so the drawn camera is the per-tick camera of `--view-trace` at any frame rate.
fn camera_tick(
    cam: &mut SceneCam,
    interest: &mut Interest,
    setup: &Setup,
    sim: &SceneSim,
    combat: Option<&CombatState>,
    cut: bool,
) {
    let Some(rig) = cam.rig.as_mut() else {
        return;
    };
    if cut {
        // The explicit signal; the distance check inside `sync` is the fallback.
        rig.request_cut();
    }
    let elapsed = elapsed_ticks(sim, combat);
    interest.0 = test_interest(&setup.kit, &setup.args, sim, combat, elapsed);
    let view = rig.sync(sim, combat, &interest.0, elapsed);
    if cam.reported_rest != Some(rig.operator().rest_target()) {
        cam.reported_rest = Some(rig.operator().rest_target());
        println!(
            "FLOOR1_CAMERA_REST tick={elapsed} rest={} view={}x{}",
            rig.operator().rest_target(),
            view.w,
            view.h
        );
    }
}

/// The fixed-schedule camera step: chained after `fixed_step`, it runs once per
/// simulated tick, with that tick's threat list.
fn update_scene_camera(
    mut cam: ResMut<SceneCam>,
    mut interest: ResMut<Interest>,
    sim: Res<Sim>,
    combat: Res<Combat>,
    setup: Res<Setup>,
    mut controls: ResMut<Controls>,
) {
    let cut = std::mem::take(&mut controls.camera_cut);
    camera_tick(
        &mut cam,
        &mut interest,
        &setup,
        &sim.0,
        combat.0.as_ref(),
        cut,
    );
}

/// Where one actor stands this frame, as drawn (y-up world feet height).
#[derive(Clone, Copy)]
struct ActorPose {
    spec: ActorSpec,
    is_hero: bool,
    foot_x: f32,
    foot_y: f32,
    flip: bool,
    frame: usize,
}

/// View top-left in layout px: the scroll camera plus the combat camera kick
/// (presentation only, kept inside the layout), or the reference view.
fn view_origin(
    state: &SceneSim,
    setup: &Setup,
    reference: &ReferencePose,
    kick: i32,
) -> (i32, i32) {
    if reference.0.is_some() {
        let name = if setup.args.deck {
            REFERENCE_VIEW_DECK
        } else {
            REFERENCE_VIEW
        };
        if let Some(view) = setup.kit.view(name) {
            return (view.x, view.y);
        }
    }
    scroll_origin(state, setup.kit.width, kick)
}

/// The scroll camera's view top-left plus the kick, kept inside the layout.
fn scroll_origin(state: &SceneSim, layout_width: i32, kick: i32) -> (i32, i32) {
    let widest = (layout_width - VIRTUAL_WIDTH).max(0);
    (
        (state.camera.left() + kick).clamp(0, widest),
        state.camera.top(),
    )
}

/// The camera kick of the combat fixture at the current tick (0 outside one).
fn camera_kick(sim: &SceneSim, combat: &Combat) -> i32 {
    combat
        .0
        .as_ref()
        .map_or(0, |state| state.camera_kick(sim.world.tick))
}

/// Carl and Donut as drawn: the simulation state, or the reference pose.
fn actor_poses(
    state: &SceneSim,
    setup: &Setup,
    reference: &ReferencePose,
    actors: &Actors,
    anim: &SceneAnimators,
) -> [ActorPose; 2] {
    if let Some(pose) = reference.0 {
        // Both stand on the walking line of their column, first idle frame,
        // facing right (the ENV-01 preview's `actor_check`).
        let kit = &setup.kit;
        let at = |spec: ActorSpec, is_hero: bool, x: f32| {
            let line = kit
                .walking_line_at(x as i32)
                .unwrap_or(kit.base_walk_line_y);
            ActorPose {
                spec,
                is_hero,
                foot_x: x,
                foot_y: to_world_y(kit.height, line),
                flip: false,
                // First idle frame (the ENV-01 preview's neutral pose).
                frame: 0,
            }
        };
        return [
            at(actors.carl.spec, true, pose.carl_x),
            at(actors.donut.spec, false, pose.donut_x),
        ];
    }
    let hero = &state.world.hero;
    let donut = &state.world.donut;
    [
        ActorPose {
            spec: actors.carl.spec,
            is_hero: true,
            foot_x: hero.x,
            // The physics foot position, also while airborne: no snapping.
            foot_y: hero.y - HERO_HALF_HEIGHT,
            flip: hero.facing < 0,
            frame: anim.carl.atlas_index(),
        },
        ActorPose {
            spec: actors.donut.spec,
            is_hero: false,
            foot_x: donut.x,
            foot_y: donut.foot_y,
            flip: donut.facing < 0,
            frame: anim.donut.atlas_index(),
        },
    ]
}

/// Presentation tick: torches and light flicker. The reference frame pins it
/// to 0 (the preview's `time_ms = 0`). Actor frames come from [`Anim`].
fn presentation_tick(
    state: &SceneSim,
    after_exit: &AfterExitTicks,
    reference: &ReferencePose,
) -> u64 {
    if reference.0.is_some() {
        0
    } else {
        state.world.tick + after_exit.0
    }
}

/// Donut's hit reaction as a tint (presentation only, live party only): the
/// impact flash for the first ticks of her flinch, then a blink through her
/// invulnerability. She has no hurt art yet; Carl has his own.
fn donut_tint(sim: &SceneSim, combat: Option<&CombatState>) -> Color {
    let Some(combat) = combat.filter(|_| sim.party_live()) else {
        return Color::WHITE;
    };
    let hurt = combat.hurt_ticks_of(HeroId::Donut);
    let invulnerable = sim.world.donut_body.invulnerable_ticks;
    if hurt > HERO_HURT_TICKS.saturating_sub(IMPACT_FLASH_TICKS) {
        FLASH_TINT
    } else if invulnerable > 0 && (invulnerable / 4).is_multiple_of(2) {
        Color::srgba(1.0, 1.0, 1.0, 0.5)
    } else {
        Color::WHITE
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_visuals(
    sim: Res<Sim>,
    combat: Res<Combat>,
    anim: Res<Anim>,
    actors: Res<Actors>,
    after_exit: Res<AfterExitTicks>,
    setup: Res<Setup>,
    reference: Res<ReferencePose>,
    cam: Res<SceneCam>,
    mut cameras: Query<
        (
            &mut Transform,
            Option<&mut Camera>,
            Option<&mut Projection>,
            Option<&RenderTarget>,
        ),
        With<SceneCamera>,
    >,
    mut parallax: Query<
        (&Parallax, &mut Transform, &mut Visibility),
        (Without<SceneCamera>, Without<ActorVisual>, Without<HeroBox>),
    >,
    mut torches: Query<(&Torch, &mut Sprite), Without<ActorVisual>>,
    mut actor_sprites: Query<
        (&ActorVisual, &mut Transform, &mut Sprite),
        (Without<SceneCamera>, Without<Parallax>, Without<HeroBox>),
    >,
    mut hero_box: Query<
        &mut Transform,
        (
            With<HeroBox>,
            Without<SceneCamera>,
            Without<Parallax>,
            Without<ActorVisual>,
        ),
    >,
) {
    let kit = &setup.kit;
    let state = &sim.0;
    let height = kit.height;
    let view = presentation_view(state, &setup, &reference, &combat, &cam);
    let (left, top) = (view.left, view.top);
    let anim_tick = presentation_tick(state, &after_exit, &reference);

    // Every scene camera (the plain one, or the lighting slot cameras and the
    // debug overlay camera) sits on the same whole-pixel position.
    for (mut transform, camera, projection, target) in &mut cameras {
        transform.translation.x = (left + view.w / 2) as f32;
        transform.translation.y = to_world_y(height, top + view.h / 2);
        if cam.rig.is_none() {
            continue;
        }
        // The presentation camera changes how much world the view shows: the
        // projection covers exactly the current view, and a lighting slot
        // camera draws it 1:1 into the top-left region of its (largest-view)
        // target. The composite pass lights that region at output resolution.
        if let Some(mut projection) = projection
            && let Projection::Orthographic(ortho) = projection.as_mut()
        {
            ortho.scaling_mode = ScalingMode::Fixed {
                width: view.w as f32,
                height: view.h as f32,
            };
        }
        if let (Some(mut camera), Some(RenderTarget::Image(_))) = (camera, target) {
            camera.viewport = Some(Viewport {
                physical_position: UVec2::ZERO,
                physical_size: UVec2::new(view.w as u32, view.h as u32),
                ..default()
            });
        }
    }

    for (strip_marker, mut transform, mut visibility) in &mut parallax {
        let strip = &kit.strips[strip_marker.strip];
        let start = parallax_start(strip.x_origin, left, strip.factor);
        let copies = parallax_copies(start, strip.rect.w, left, view.w);
        match copies.get(strip_marker.copy) {
            Some(x) => {
                transform.translation.x = *x as f32;
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }

    for (torch, mut sprite) in &mut torches {
        let frame = animation_frame(&torch.durations, torch.offset, anim_tick);
        sprite.rect = Some(torch.rects[frame]);
    }

    let poses = actor_poses(state, &setup, &reference, &actors, &anim.0);
    for (actor, mut transform, mut sprite) in &mut actor_sprites {
        let pose = poses
            .iter()
            .find(|p| p.is_hero == actor.is_hero)
            .expect("one pose per actor");
        let (cx, cy) = actor_center(pose.spec, pose.foot_x, pose.foot_y, pose.flip);
        transform.translation.x = cx;
        transform.translation.y = cy;
        sprite.flip_x = pose.flip;
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = pose.frame;
        }
        if !actor.is_hero {
            sprite.color = donut_tint(state, combat.0.as_ref());
        }
    }

    let hero = &state.world.hero;
    for mut transform in &mut hero_box {
        transform.translation.x = hero.x.round();
        transform.translation.y = hero.y.round();
    }
}

/// Combat fixture presentation (reads the simulation and the animators only):
/// the goblin sprites with their frame, flip, depth and impact flash, Carl's
/// melee sprite (shown instead of the locomotion sprite while a melee, hurt or
/// knockdown pose is drawn) and the `--debug-combat` boxes (Donut's hurtbox
/// while the party is live).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_combat_visuals(
    sim: Res<Sim>,
    combat: Res<Combat>,
    anim: Res<Anim>,
    setup: Res<Setup>,
    mut goblins: Query<
        (&GoblinSprite, &mut Transform, &mut Sprite),
        (
            Without<CarlMelee>,
            Without<ActorVisual>,
            Without<CombatDebugBox>,
        ),
    >,
    mut melee: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (
            With<CarlMelee>,
            Without<GoblinSprite>,
            Without<ActorVisual>,
            Without<CombatDebugBox>,
        ),
    >,
    mut actors: Query<
        (&ActorVisual, &mut Visibility),
        (
            Without<GoblinSprite>,
            Without<CarlMelee>,
            Without<CombatDebugBox>,
        ),
    >,
    mut boxes: Query<
        (&CombatDebugBox, &mut Transform, &mut Visibility),
        (
            Without<GoblinSprite>,
            Without<CarlMelee>,
            Without<ActorVisual>,
        ),
    >,
) {
    let Some(state) = combat.0.as_ref() else {
        return;
    };
    let hero = &sim.0.world.hero;
    let tick = sim.0.world.tick;
    let actor_z = setup.kit.actor_z;
    let carl_active = CombatAnimators::carl_active(&sim.0);

    if let Some(art) = anim.0.art.as_ref() {
        for (goblin_sprite, mut transform, mut sprite) in &mut goblins {
            let index = goblin_sprite.index;
            let goblin = &state.enemies[index];
            let clips = art.goblin_clips(index);
            let pose = art.goblin_pose(index);
            // Airborne frames sit on the bottom of the sim body (no hover, no
            // sink); grounded ones keep the pivot on the physics foot.
            let foot = clips.anchor_foot_y(&pose, goblin.y - ENEMY_HALF_HEIGHT);
            let flip = goblin.facing < 0;
            // Drawn a little toward Carl while he strikes it in hit-stop, and
            // away from a goblin it would stand on (presentation only).
            let draw_x = goblin.x + art.goblin_draw_dx(index) as f32;
            let (cx, cy) = actor_center(clips.spec, draw_x, foot, flip);
            let lying = matches!(goblin.mode, GoblinMode::Down | GoblinMode::Corpse);
            let swinging = matches!(
                goblin.attack_phase(),
                Some(AttackPhase::Smear | AttackPhase::Impact)
            );
            let z_offset = if lying {
                GOBLIN_LYING_Z_OFFSET
            } else if carl_above_goblin(carl_active, swinging) {
                GOBLIN_Z_OFFSET
            } else {
                GOBLIN_SWING_Z_OFFSET
            };
            transform.translation.x = cx;
            transform.translation.y = cy;
            transform.translation.z = actor_z + z_offset - index as f32 * GOBLIN_Z_STEP;
            sprite.flip_x = flip;
            sprite.color = if state.flash(index, tick) {
                FLASH_TINT
            } else {
                Color::WHITE
            };
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = pose.frame;
            }
        }
    }

    // Carl: the melee atlas while a combat pose is drawn, otherwise the
    // locomotion sprite (idle, run, jump, land).
    let carl_pose = anim.0.art.as_ref().and_then(|art| art.carl_pose());
    for (mut transform, mut sprite, mut visibility) in &mut melee {
        match (carl_pose, anim.0.art.as_ref()) {
            (Some(pose), Some(art)) => {
                let foot = art.carl_anchor_foot_y(hero.y - HERO_HALF_HEIGHT);
                let flip = hero.facing < 0;
                let (cx, cy) = actor_center(art.carl.spec, hero.x, foot, flip);
                transform.translation.x = cx;
                transform.translation.y = cy;
                sprite.flip_x = flip;
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = pose.frame;
                }
                *visibility = Visibility::Inherited;
            }
            _ => *visibility = Visibility::Hidden,
        }
    }
    // ENEMY-GAMEPLAY-ENGINEER: in the Carl-only `eng_*` fixtures Donut is not a
    // live hero, so her trail sprite is not drawn: it would stand over the
    // parapet or the lane and read as the thing Carl or the Engineer hit.
    let donut_hidden = setup.args.party_fixture.is_some_and(|pf| !pf.party_live());
    for (actor, mut visibility) in &mut actors {
        *visibility = if (actor.is_hero && carl_pose.is_some()) || (!actor.is_hero && donut_hidden)
        {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }

    let attacker = attacker_of(hero);
    let hit = hero
        .melee
        .current()
        .filter(|m| m.phase == Phase::Active)
        .and_then(|m| hit_box(&m.spec.shape, &attacker));
    for (kind, mut transform, mut visibility) in &mut boxes {
        let rect = |transform: &mut Transform, rect: dungeon_floorbreak::prototype::Rect| {
            let (cx, cy) = rect.center();
            transform.translation.x = cx.round();
            transform.translation.y = cy.round();
            transform.scale = Vec3::new(rect.width, rect.height, 1.0);
        };
        match kind {
            CombatDebugBox::HitBox => match hit {
                Some(r) => {
                    rect(&mut transform, r);
                    *visibility = Visibility::Inherited;
                }
                None => *visibility = Visibility::Hidden,
            },
            CombatDebugBox::CarlHurt => {
                transform.translation.x = hero.x.round();
                transform.translation.y = hero.y.round();
                *visibility = Visibility::Inherited;
            }
            CombatDebugBox::DonutHurt => {
                if sim.0.party_live() {
                    rect(&mut transform, CombatState::hurtbox(&sim.0, HeroId::Donut));
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }
            CombatDebugBox::Goblin(index) => {
                let enemy = &state.enemies[*index];
                transform.translation.x = enemy.x.round();
                transform.translation.y = enemy.y.round();
                *visibility = if enemy.alive() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
            CombatDebugBox::Swing(index) => {
                let enemy = &state.enemies[*index];
                if enemy.attack_phase() == Some(AttackPhase::Impact) {
                    rect(&mut transform, enemy.swing_box());
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }
            // Donut's own boxes belong to `sync_donut_visuals`.
            CombatDebugBox::DonutHit | CombatDebugBox::Bolt(_) => {}
        }
    }
}

/// Render layers of Donut's extra sprites.
struct DonutArtLayers {
    actors: RenderLayers,
    debug: RenderLayers,
}

/// Spawn Donut's melee, cast and bolt sprites plus her `--debug-combat` boxes,
/// and return the atlas handles the loading watchdog waits for. A melee or
/// missile atlas that is not on disk is skipped (she keeps her locomotion
/// sprite); a missing VFX atlas makes the bolts 8 x 6 magenta rectangles.
fn spawn_donut_art(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    layers: &DonutArtLayers,
    actor_z: f32,
    debug_z: f32,
    debug: bool,
) -> Vec<(String, Handle<Image>)> {
    let art_dir = std::env::current_dir()
        .ok()
        .map(|root| root.join("generated/art"));
    let on_disk = |name: &str| art_dir.as_ref().is_some_and(|dir| dir.join(name).is_file());
    let mut handles = Vec::new();
    let mut grid = |name: &str, w: u32, h: u32, count: usize| {
        let texture: Handle<Image> = asset_server.load(name.to_owned());
        handles.push((name.to_owned(), texture.clone()));
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(w, h),
            count as u32,
            1,
            None,
            None,
        ));
        (texture, layout)
    };
    // Donut's sprites sit a hair above her locomotion sprite (z = actor_z) and
    // below Carl's (+0.1); the bolts draw in front of everything of the party.
    let z = actor_z + 0.01;
    if on_disk(donut_art::MELEE_ATLAS) {
        let (texture, layout) = grid(
            donut_art::MELEE_ATLAS,
            donut_art::FRAME_W,
            donut_art::FRAME_H,
            donut_art::MELEE_FRAMES,
        );
        commands.spawn((
            DonutMelee,
            Sprite::from_atlas_image(texture, TextureAtlas { layout, index: 0 }),
            Transform::from_xyz(0.0, 0.0, z),
            Visibility::Hidden,
            layers.actors.clone(),
        ));
    }
    if on_disk(donut_art::MISSILE_ATLAS) {
        let (texture, layout) = grid(
            donut_art::MISSILE_ATLAS,
            donut_art::FRAME_W,
            donut_art::FRAME_H,
            donut_art::MISSILE_FRAMES,
        );
        commands.spawn((
            DonutCast,
            Sprite::from_atlas_image(texture, TextureAtlas { layout, index: 0 }),
            Transform::from_xyz(0.0, 0.0, z),
            Visibility::Hidden,
            layers.actors.clone(),
        ));
    }
    let vfx = on_disk(donut_art::VFX_ATLAS).then(|| {
        grid(
            donut_art::VFX_ATLAS,
            donut_art::VFX_SIZE,
            donut_art::VFX_SIZE,
            donut_art::VFX_FRAMES,
        )
    });
    for slot in 0..BOLT_SLOTS {
        let sprite = match vfx.clone() {
            Some((texture, layout)) => {
                Sprite::from_atlas_image(texture, TextureAtlas { layout, index: 1 })
            }
            None => Sprite::from_color(Color::srgb(1.0, 0.0, 1.0), Vec2::new(8.0, 6.0)),
        };
        commands.spawn((
            BoltSprite { slot },
            sprite,
            Transform::from_xyz(0.0, 0.0, actor_z + GLOB_Z_OFFSET),
            Visibility::Hidden,
            layers.actors.clone(),
        ));
    }
    if debug {
        let mut overlay = |kind: CombatDebugBox, colour: Color| {
            commands.spawn((
                kind,
                Sprite::from_color(colour, Vec2::ONE),
                Transform::from_xyz(0.0, 0.0, debug_z),
                Visibility::Hidden,
                layers.debug.clone(),
            ));
        };
        overlay(CombatDebugBox::DonutHit, Color::srgba(1.0, 0.1, 0.9, 0.55));
        for slot in 0..BOLT_SLOTS {
            overlay(CombatDebugBox::Bolt(slot), Color::srgba(0.6, 0.2, 1.0, 0.6));
        }
    }
    handles
}

/// Donut's melee, cast and bolt presentation (reads the simulation's
/// read-only views and the animators only; nothing here feeds back). While a
/// swat or pounce runs her melee sprite replaces the locomotion one; during a
/// cast, or the 8-tick dry cue, the head-swap sprite does, on her current
/// locomotion frame. The bolts and the `--debug-combat` boxes follow the
/// simulation's list. Runs after `sync_combat_visuals`, which owns the
/// visibility of her locomotion sprite.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_donut_visuals(
    sim: Res<Sim>,
    combat: Res<Combat>,
    anim: Res<Anim>,
    actors: Res<Actors>,
    reference: Res<ReferencePose>,
    mut melee: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (
            With<DonutMelee>,
            Without<DonutCast>,
            Without<BoltSprite>,
            Without<ActorVisual>,
            Without<CombatDebugBox>,
        ),
    >,
    mut cast: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (
            With<DonutCast>,
            Without<DonutMelee>,
            Without<BoltSprite>,
            Without<ActorVisual>,
            Without<CombatDebugBox>,
        ),
    >,
    mut bolts: Query<
        (&BoltSprite, &mut Transform, &mut Sprite, &mut Visibility),
        (
            Without<DonutMelee>,
            Without<DonutCast>,
            Without<ActorVisual>,
            Without<CombatDebugBox>,
        ),
    >,
    mut locomotion: Query<
        (&ActorVisual, &mut Visibility),
        (
            Without<DonutMelee>,
            Without<DonutCast>,
            Without<BoltSprite>,
            Without<CombatDebugBox>,
        ),
    >,
    mut boxes: Query<
        (&CombatDebugBox, &mut Transform, &mut Visibility),
        (
            Without<DonutMelee>,
            Without<DonutCast>,
            Without<BoltSprite>,
            Without<ActorVisual>,
        ),
    >,
) {
    let Some(state) = combat.0.as_ref() else {
        return;
    };
    let sim = &sim.0;
    let tick = sim.world.tick;
    let party = sim.party_live() && reference.0.is_none();
    let donut = &sim.world.donut;
    let flip = donut.facing < 0;
    let spec = actors.donut.spec;
    let locomotion_frame = anim.0.donut.atlas_index();
    let tint = donut_tint(sim, Some(state));

    let melee_frame = state
        .donut_move_view()
        .filter(|_| party)
        .map(|view| donut_art::melee_frame(&view));
    // Melee wins over a cast; a real cast over the dry cue.
    let cast_frame = if melee_frame.is_some() || !party {
        None
    } else {
        state
            .donut_cast()
            .map(|c| donut_art::cast_index(c.age, locomotion_frame))
            .or_else(|| donut_art::dry_cue_index(tick, state.donut_dry_tick(), locomotion_frame))
    };

    let mut shown = false;
    for (mut transform, mut sprite, mut visibility) in &mut melee {
        let Some(frame) = melee_frame else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let foot = donut.foot_y - donut_art::melee_foot_drop(frame);
        let (cx, cy) = actor_center(spec, donut.x, foot, flip);
        transform.translation.x = cx;
        transform.translation.y = cy;
        sprite.flip_x = flip;
        sprite.color = tint;
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = frame;
        }
        *visibility = Visibility::Inherited;
        shown = true;
    }
    for (mut transform, mut sprite, mut visibility) in &mut cast {
        let Some(frame) = cast_frame else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let (cx, cy) = actor_center(spec, donut.x, donut.foot_y, flip);
        transform.translation.x = cx;
        transform.translation.y = cy;
        sprite.flip_x = flip;
        sprite.color = tint;
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = frame;
        }
        *visibility = Visibility::Inherited;
        shown = true;
    }
    if shown {
        for (actor, mut visibility) in &mut locomotion {
            if !actor.is_hero {
                *visibility = Visibility::Hidden;
            }
        }
    }

    let bolt_list = if party { state.donut_bolts() } else { &[] };
    let speed = f32::from(state.donut_mana_tuning().projectile_speed_px_per_tick);
    for (slot, mut transform, mut sprite, mut visibility) in &mut bolts {
        let Some(bolt) = bolt_list.get(slot.slot) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        transform.translation.x = bolt.x.round();
        transform.translation.y = bolt.y.round();
        sprite.flip_x = bolt.dir < 0;
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = donut_art::bolt_frame(bolt.travelled, speed);
        } else {
            // The magenta fallback is the hit box itself.
            let (cx, cy) = bolt.rect().center();
            transform.translation.x = cx.round();
            transform.translation.y = cy.round();
        }
        *visibility = Visibility::Inherited;
    }

    let hit = state.donut_hitbox().filter(|_| party);
    for (kind, mut transform, mut visibility) in &mut boxes {
        let rect = match kind {
            CombatDebugBox::DonutHit => hit,
            CombatDebugBox::Bolt(slot) => bolt_list.get(*slot).map(|b| b.rect()),
            _ => continue,
        };
        match rect {
            Some(r) => {
                let (cx, cy) = r.center();
                transform.translation.x = cx.round();
                transform.translation.y = cy.round();
                transform.scale = Vec3::new(r.width, r.height, 1.0);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// HUD-DEBT-03: the boss encounter as the simulation has it. Placeholder rects
/// at the spec sizes (the hurtbox, the racks, the strut, the gate, the
/// ripples); with the boss atlas the body is the atlas frame of
/// [`boss_art::pose_of`]. Reads the simulation only.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_boss_visuals(
    combat: Res<Combat>,
    sim: Res<Sim>,
    death: Res<Death>,
    setup: Res<Setup>,
    art: Res<BossArtRes>,
    textures: Option<Res<BossTextures>>,
    mut walk: Local<WalkTrack>,
    mut pieces: Query<
        (&BossPiece, &mut Transform, &mut Sprite, &mut Visibility),
        Without<BossFxSprite>,
    >,
    mut fx: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (With<BossFxSprite>, Without<BossPiece>),
    >,
) {
    let Some(state) = combat.0.as_ref() else {
        return;
    };
    let Some(boss) = state.boss.as_ref() else {
        return;
    };
    let data = boss.data();
    let floor = boss.floor_y();
    let set = |transform: &mut Transform, rect: dungeon_floorbreak::prototype::Rect| {
        let (cx, cy) = rect.center();
        transform.translation.x = cx.round();
        transform.translation.y = cy.round();
        transform.scale = Vec3::new(rect.width, rect.height, 1.0);
    };
    let ripples: Vec<(f32, i8)> = boss.ripples().collect();
    // What the atlases cover is drawn from them (`boss_art::draws`); what a
    // missing atlas leaves out falls back to its placeholder rect.
    let has = |id: SheetId| art.0.as_ref().is_some_and(|a| a.sheet(id).is_some());
    let has_art = has(SheetId::Base) && has(SheetId::Enc);
    for (piece, mut transform, mut sprite, mut visibility) in &mut pieces {
        let shown = match *piece {
            BossPiece::Body => match boss.hurtbox() {
                Some(rect) if !has_art => {
                    set(&mut transform, rect);
                    sprite.color = match boss.mode {
                        BossMode::PoundTell | BossMode::SecondTell | BossMode::ChargeTell => {
                            Color::srgb(0.95, 0.6, 0.12)
                        }
                        BossMode::Stun | BossMode::Rebound => Color::srgb(0.3, 0.3, 0.7),
                        BossMode::Pinned => Color::srgb(0.35, 0.35, 0.35),
                        _ => Color::srgb(0.42, 0.1, 0.34),
                    };
                    true
                }
                None if boss.mode == BossMode::Defeated && !has_art => {
                    // The corpse: the pinned box, flat on the floor.
                    set(
                        &mut transform,
                        dungeon_floorbreak::prototype::Rect {
                            left: boss.x - 50.0,
                            bottom: floor,
                            width: 100.0,
                            height: 14.0,
                        },
                    );
                    sprite.color = Color::srgb(0.25, 0.08, 0.1);
                    true
                }
                _ => false,
            },
            BossPiece::Eye => match boss.hurtbox() {
                Some(rect) if !has_art => {
                    let front = if boss.facing >= 0 {
                        rect.right() - 12.0
                    } else {
                        rect.left + 4.0
                    };
                    set(
                        &mut transform,
                        dungeon_floorbreak::prototype::Rect {
                            left: front,
                            bottom: rect.top() - 22.0,
                            width: 8.0,
                            height: 8.0,
                        },
                    );
                    true
                }
                _ => false,
            },
            BossPiece::Rack(_) if has(SheetId::Rack) => false,
            BossPiece::Strut if has(SheetId::Strut) => false,
            BossPiece::Ripple(_) if has(SheetId::Vfx) => false,
            BossPiece::Rack(index) => {
                let rack = &boss.racks[index];
                let hanging = 189.0;
                let bottom = match rack.state {
                    RackState::Idle { .. } => hanging,
                    RackState::Falling { ticks, .. } => {
                        let t = f32::from(ticks) / f32::from(data.rack.fall_ticks);
                        hanging * (1.0 - t * t)
                    }
                    RackState::Landed => 0.0,
                };
                set(
                    &mut transform,
                    dungeon_floorbreak::prototype::Rect {
                        left: data.footprint_centre(rack.facing) - data.rack.footprint_px * 0.5,
                        bottom: floor + bottom,
                        width: data.rack.footprint_px,
                        height: data.rack.footprint_height,
                    },
                );
                rack.enabled
            }
            BossPiece::Strut => {
                set(&mut transform, boss.strut_rect());
                data.rack.left.enabled && matches!(boss.racks[0].state, RackState::Idle { .. })
            }
            BossPiece::Gate => {
                set(
                    &mut transform,
                    dungeon_floorbreak::prototype::Rect {
                        left: 0.0,
                        bottom: floor,
                        width: data.gate.solid_x,
                        height: 224.0,
                    },
                );
                boss.gate_closed()
            }
            BossPiece::Ripple(slot) => match ripples.get(slot) {
                Some(&(x, _)) => {
                    set(
                        &mut transform,
                        dungeon_floorbreak::prototype::Rect {
                            left: x - data.shockwave.width * 0.5,
                            bottom: floor,
                            width: data.shockwave.width,
                            height: data.shockwave.height,
                        },
                    );
                    true
                }
                None => false,
            },
        };
        *visibility = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let (Some(art), Some(textures)) = (art.0.as_ref(), textures) else {
        return;
    };
    // The gait: observed once per simulated tick, not per render frame.
    let tick = sim.0.world.tick;
    if walk.tick != tick {
        walk.walking = boss.mode == BossMode::Neutral && (boss.x - walk.x).abs() > 1e-3;
        walk.tick = tick;
        walk.x = boss.x;
    }
    let gore_full = death
        .0
        .as_ref()
        .is_none_or(|presenter| presenter.preset() == GorePreset::Full);
    let list = boss_art::draws(
        art,
        boss,
        FxCtx {
            events: &state.events,
            now: tick,
            gait: Gait {
                clock: tick,
                walking: walk.walking,
                ..Gait::default()
            },
            gore_full,
        },
    );
    let actor_z = setup.kit.actor_z;
    let mut drawn = list.iter();
    for (index, (mut transform, mut sprite, mut visibility)) in fx.iter_mut().enumerate() {
        let Some(draw) = drawn.next() else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let (Some(sheet), Some((texture, layout))) = (
            art.sheet(draw.sheet),
            textures.0[draw.sheet as usize].as_ref(),
        ) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let pivot = sheet.pivot(draw.frame);
        let spec = ActorSpec {
            name: "brute_boss_f01",
            width: sheet.frame_size.0,
            height: sheet.frame_size.1,
            pivot_x: pivot.0,
            pivot_y: pivot.1,
            shadow_width: 0,
            shadow_pivot_x: 0,
        };
        let (cx, cy) = actor_center(spec, draw.x, draw.y, draw.flip);
        let z = actor_z
            + match draw.layer {
                Layer::Floor => BOSS_Z_FLOOR,
                Layer::Back => BOSS_Z_BACK,
                Layer::Body => BOSS_Z_OFFSET,
                Layer::Front => BOSS_Z_FRONT,
            }
            + index as f32 * 0.0001;
        transform.translation = Vec3::new(cx, cy, z);
        sprite.flip_x = draw.flip;
        if sprite.image != *texture {
            sprite.image = texture.clone();
        }
        sprite.texture_atlas = Some(TextureAtlas {
            layout: layout.clone(),
            index: draw.frame,
        });
        *visibility = Visibility::Inherited;
    }
}

/// DEMO-01 PR B: the Bad Llamas and their spit globs as the simulation has them
/// (frame by the finished tick, whole-pixel placement like every actor). Reads
/// the simulation and the animators only.
#[allow(clippy::type_complexity)]
fn sync_llama_visuals(
    sim: Res<Sim>,
    combat: Res<Combat>,
    anim: Res<Anim>,
    setup: Res<Setup>,
    mut llamas: Query<(&LlamaSprite, &mut Transform, &mut Sprite), Without<GlobSprite>>,
    mut globs: Query<
        (&GlobSprite, &mut Transform, &mut Sprite, &mut Visibility),
        Without<LlamaSprite>,
    >,
) {
    let (Some(state), Some(art)) = (combat.0.as_ref(), anim.0.art.as_ref()) else {
        return;
    };
    let Some(clips) = art.llama_clips() else {
        return;
    };
    let tick = sim.0.world.tick;
    let actor_z = setup.kit.actor_z;
    let goblins = state.enemies.len();
    for (llama_sprite, mut transform, mut sprite) in &mut llamas {
        let index = llama_sprite.index;
        let (Some(llama), Some(pose)) = (state.llamas.get(index), art.llama_pose(index)) else {
            continue;
        };
        let flip = llama.facing < 0;
        let (cx, cy) = actor_center(
            clips.spec,
            llama.x,
            CombatAnimators::llama_foot_y(llama),
            flip,
        );
        let lying = matches!(llama.mode, LlamaMode::Down | LlamaMode::Corpse);
        let z_offset = if lying {
            GOBLIN_LYING_Z_OFFSET
        } else {
            GOBLIN_Z_OFFSET
        };
        transform.translation = Vec3::new(
            cx,
            cy,
            actor_z + z_offset - (goblins + index) as f32 * GOBLIN_Z_STEP,
        );
        sprite.flip_x = flip;
        sprite.color = if state.flash(goblins + index, tick) {
            FLASH_TINT
        } else {
            Color::WHITE
        };
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = pose.frame;
        }
    }
    // The glob is drawn with the llama's pivot, as the spawn contract says: the
    // sprite sits so that its core (pivot offset +46, -45) is on the glob.
    let loop_ticks = clips.tag_ticks("spit").unwrap_or(8).max(1);
    for (glob_sprite, mut transform, mut sprite, mut visibility) in &mut globs {
        match state.globs.get(glob_sprite.slot) {
            Some(glob) => {
                let flip = glob.dir < 0;
                let foot_x = glob.x - f32::from(glob.dir) * LLAMA_GLOB_MOUTH_DX;
                let foot_y = glob.y - LLAMA_GLOB_MOUTH_DY;
                let (cx, cy) = actor_center(clips.spec, foot_x, foot_y, flip);
                transform.translation = Vec3::new(cx, cy, actor_z + GLOB_Z_OFFSET);
                sprite.flip_x = flip;
                let age = (glob.travelled / (LLAMA_GLOB_SPEED * TICK_SECONDS)).round() as u32;
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = clips.frame_at("spit", age % loop_ticks);
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// The impact dust of a llama slam, at the floor under the slam's `hit_origin`
/// (the ENV-02 block impact dust, four frames). Reads the presenter only.
fn sync_dust_visuals(
    death: Res<Death>,
    actors: Res<Actors>,
    setup: Res<Setup>,
    images: Option<Res<ShoreImages>>,
    mut dust: Query<(&DustSprite, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(presenter), Some(art), Some(images)) =
        (death.0.as_ref(), actors.shore.as_ref(), images)
    else {
        return;
    };
    let list = presenter.impact_dusts();
    for (sprite_slot, mut transform, mut sprite, mut visibility) in &mut dust {
        let draw = list
            .get(sprite_slot.slot)
            .and_then(|d| art.impact_dust_draw(d.age, (d.x, d.ground_y)));
        match draw {
            Some(draw) => {
                transform.translation = Vec3::new(
                    draw.centre.0,
                    draw.centre.1,
                    setup.kit.actor_z + HAZARD_OVER_Z_OFFSET,
                );
                sprite.image = images.0[draw.image].clone();
                sprite.rect = Some(Rect::new(
                    draw.rect[0] as f32,
                    draw.rect[1] as f32,
                    (draw.rect[0] + draw.rect[2]) as f32,
                    (draw.rect[1] + draw.rect[3]) as f32,
                ));
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// DEATH-C: the ENV-02 shore and block as the simulation has them (hits, block
/// state, block height, landing). Reads the simulation only.
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn sync_shore_visuals(
    combat: Res<Combat>,
    death: Res<Death>,
    sim: Res<Sim>,
    actors: Res<Actors>,
    setup: Res<Setup>,
    images: Option<Res<ShoreImages>>,
    layers: Option<Res<ShoreLayers>>,
    mut pieces: Query<(
        &ShorePiece,
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
        &mut RenderLayers,
    )>,
) {
    let (Some(state), Some(art), Some(images)) = (combat.0.as_ref(), actors.shore.as_ref(), images)
    else {
        return;
    };
    let Some(shore) = state.shore.as_ref() else {
        return;
    };
    let tick = sim.0.world.tick;
    // A crushed enemy whose contact frames carry `hazard_draw: "over"` (the
    // llama): the block's anchor goes on the mirrored `hit_origin` and the
    // stone is drawn over that body, still below the players (presentation
    // only; the block's simulated position never moves).
    let over = death.0.as_ref().and_then(|p| p.hazard_over_anchor());
    let list = art.draw_list_with(shore, landed_age(&state.events, tick), tick, over);
    let prop_z = setup.kit.actor_z - 1.0;
    let wall_z = setup
        .kit
        .pieces
        .iter()
        .find(|p| p.layer == "backwall_decal")
        .map_or(3.0, |p| p.z.floor());
    for (piece, mut transform, mut sprite, mut visibility, mut render_layers) in &mut pieces {
        if piece.0 == ShoreSlot::Block
            && let Some(layers) = layers.as_ref()
        {
            let want = block_layers(over.is_some(), &layers.prop, &layers.actors);
            if *render_layers != *want {
                *render_layers = want.clone();
            }
        }
        match list.iter().find(|d| d.slot == piece.0) {
            Some(draw) => {
                let z = if draw.slot == ShoreSlot::Cavity {
                    wall_z + 0.3
                } else if draw.slot == ShoreSlot::Block && over.is_some() {
                    setup.kit.actor_z + HAZARD_OVER_Z_OFFSET
                } else {
                    prop_z + draw.slot.z_bias()
                };
                transform.translation = Vec3::new(draw.centre.0, draw.centre.1, z);
                sprite.image = images.0[draw.image].clone();
                sprite.rect = Some(Rect::new(
                    draw.rect[0] as f32,
                    draw.rect[1] as f32,
                    (draw.rect[0] + draw.rect[2]) as f32,
                    (draw.rect[1] + draw.rect[3]) as f32,
                ));
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// DEATH-D: the QA charge as the `dynamite_fizzle` loop from the first tick of
/// the attempt until it detonates, then the VFX-01 `burst` once, centred on the
/// blast centre. Both sit on the blast layer (`actor_z - 0.45`: over the floor
/// decals and the props, under the debris and every actor). Reads the
/// simulation only.
#[allow(clippy::type_complexity)]
fn sync_blast_visuals(
    combat: Res<Combat>,
    death: Res<Death>,
    sim: Res<Sim>,
    actors: Res<Actors>,
    setup: Res<Setup>,
    images: Option<Res<BlastImages>>,
    mut pieces: Query<(&BlastPiece, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(state), Some(art), Some(images)) = (combat.0.as_ref(), actors.blast.as_ref(), images)
    else {
        return;
    };
    let tick = sim.0.world.tick;
    let actor_z = setup.kit.actor_z;
    let wall_z = setup
        .kit
        .pieces
        .iter()
        .find(|p| p.layer == "backwall_decal")
        .map_or(3.0, |p| p.z.floor());
    // Preset 4 punch-up (presentation only): the burst is bigger, drawn over
    // the debris and flashes bright for its first ticks.
    let table = explosion_table(death.0.as_ref().map_or(GorePreset::Full, |p| p.preset()));
    let punched = table.burst_scale > 1.0;
    let dynamite_spec = ActorSpec {
        name: "qa_charge",
        width: 24,
        height: 24,
        pivot_x: 12,
        pivot_y: 12,
        shadow_width: 0,
        shadow_pivot_x: 0,
    };
    let to_rect = |r: [i32; 4]| {
        Rect::new(
            r[0] as f32,
            r[1] as f32,
            (r[0] + r[2]) as f32,
            (r[1] + r[3]) as f32,
        )
    };
    for (piece, mut transform, mut sprite, mut visibility) in &mut pieces {
        let (index, burst) = match *piece {
            BlastPiece::Charge(i) => (i, false),
            BlastPiece::Burst(i) => (i, true),
        };
        let Some(charge) = state.charges.get(index) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        if burst {
            let layer = if punched {
                DrawLayer::BlastFront
            } else {
                DrawLayer::Blast
            };
            let z = draw_z(layer, index, actor_z, wall_z);
            let frame = charge
                .detonated
                .then(|| blast_vfx_frame(tick, charge.detonate_tick))
                .flatten();
            match frame {
                Some(frame) => {
                    let (cx, cy) = (charge.x, charge.foot_y + BLAST_CENTRE_DY);
                    transform.translation = Vec3::new(cx.round(), cy.round(), z);
                    transform.scale = Vec3::new(table.burst_scale, table.burst_scale, 1.0);
                    sprite.image = images.burst.clone();
                    sprite.rect = Some(to_rect(art.burst_rect(frame)));
                    // The flash: the existing impact tint for the first ticks.
                    let age = tick.saturating_sub(charge.detonate_tick);
                    sprite.color = if age < table.flash_ticks {
                        FLASH_TINT
                    } else {
                        Color::WHITE
                    };
                    *visibility = Visibility::Inherited;
                }
                None => *visibility = Visibility::Hidden,
            }
        } else if charge.detonated {
            *visibility = Visibility::Hidden;
        } else {
            // The lit charge: above the goblins, below Carl (it was under every
            // actor and hidden by the goblin standing on it).
            let z = draw_z(DrawLayer::ChargeLit, index, actor_z, wall_z);
            let (cx, cy) = actor_center(dynamite_spec, charge.x, charge.foot_y, false);
            transform.translation = Vec3::new(cx, cy, z);
            sprite.image = images.dynamite.clone();
            // A Carl-armed charge lies still until he passes; its fizzle loop
            // counts from the arming.
            let lit_for = tick.saturating_sub(charge.armed_tick.unwrap_or(0));
            sprite.rect = Some(to_rect(
                art.dynamite_rect(charge_draw_frame(charge.lit(), lit_for)),
            ));
            *visibility = Visibility::Inherited;
        }
    }
}

/// CF02-I: draw the CF02 frames (ART-CARL-CF02, ART-DONUT-CF02, ART-GOB-CF02 and
/// the slammed sheets) over the sprites the other systems placed. Runs last of the
/// actor systems: where a CF02 frame is drawn it hides the sprite it replaces and
/// copies the base sprite's z, tint and (for a goblin) alpha, so the hit flash and
/// the corpse fade follow. Reads the animators and the death presenter only;
/// nothing flows back to the simulation.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_cf02_acting_visuals(
    sim: Res<Sim>,
    combat: Res<Combat>,
    anim: Res<Anim>,
    death: Res<Death>,
    mut carl: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (
            With<CarlCf02Sprite>,
            Without<DonutCf02Sprite>,
            Without<GoblinActingSprite>,
            Without<GoblinSlammedSprite>,
            Without<ActorVisual>,
            Without<GoblinSprite>,
        ),
    >,
    mut donut: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (
            With<DonutCf02Sprite>,
            Without<CarlCf02Sprite>,
            Without<GoblinActingSprite>,
            Without<GoblinSlammedSprite>,
            Without<ActorVisual>,
            Without<GoblinSprite>,
        ),
    >,
    mut actors: Query<
        (&ActorVisual, &mut Visibility),
        (
            Without<CarlCf02Sprite>,
            Without<DonutCf02Sprite>,
            Without<GoblinActingSprite>,
            Without<GoblinSlammedSprite>,
            Without<GoblinSprite>,
        ),
    >,
    mut acting: Query<
        (
            &GoblinActingSprite,
            &mut Transform,
            &mut Sprite,
            &mut Visibility,
        ),
        (
            Without<CarlCf02Sprite>,
            Without<DonutCf02Sprite>,
            Without<GoblinSlammedSprite>,
            Without<ActorVisual>,
            Without<GoblinSprite>,
        ),
    >,
    mut slammed: Query<
        (
            &GoblinSlammedSprite,
            &mut Transform,
            &mut Sprite,
            &mut Visibility,
        ),
        (
            Without<CarlCf02Sprite>,
            Without<DonutCf02Sprite>,
            Without<GoblinActingSprite>,
            Without<ActorVisual>,
            Without<GoblinSprite>,
        ),
    >,
    mut base: Query<
        (&GoblinSprite, &Transform, &Sprite, &mut Visibility),
        (
            Without<CarlCf02Sprite>,
            Without<DonutCf02Sprite>,
            Without<GoblinActingSprite>,
            Without<GoblinSlammedSprite>,
            Without<ActorVisual>,
        ),
    >,
) {
    let (Some(state), Some(art)) = (combat.0.as_ref(), anim.0.art.as_ref()) else {
        return;
    };
    let Some(cf02) = art.acting() else {
        return;
    };
    let hero = &sim.0.world.hero;

    // Carl.
    let carl_pose = art.carl_cf02_pose();
    for (mut transform, mut sprite, mut visibility) in &mut carl {
        match carl_pose {
            Some(pose) => {
                let flip = hero.facing < 0;
                let (cx, cy) =
                    actor_center(cf02.art.carl.spec, hero.x, hero.y - HERO_HALF_HEIGHT, flip);
                transform.translation.x = cx;
                transform.translation.y = cy;
                sprite.flip_x = flip;
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = pose.frame;
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    // Donut's overlay (her locomotion sprite stays hidden while it shows).
    let donut_pose = art.donut_cf02_pose();
    let donut_world = &sim.0.world.donut;
    for (mut transform, mut sprite, mut visibility) in &mut donut {
        match donut_pose {
            Some(pose) => {
                let flip = donut_world.facing < 0;
                let (cx, cy) =
                    actor_center(cf02.art.donut.spec, donut_world.x, donut_world.foot_y, flip);
                transform.translation.x = cx;
                transform.translation.y = cy;
                sprite.flip_x = flip;
                sprite.color = donut_tint(&sim.0, Some(state));
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = pose.frame;
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    for (actor, mut visibility) in &mut actors {
        let replaced = if actor.is_hero {
            carl_pose.is_some()
        } else {
            donut_pose.is_some()
        };
        if replaced {
            *visibility = Visibility::Hidden;
        }
    }

    // Goblins: an acting frame or a slammed finish replaces the GOB-01 sprite.
    let slam_of = |index: usize| {
        death
            .0
            .as_ref()
            .and_then(|presenter| presenter.slam_pose_of(index as u32))
    };
    let mut hide: Vec<usize> = Vec::new();
    for (goblin, mut transform, mut sprite, mut visibility) in &mut acting {
        let index = goblin.index;
        let (Some(pose), Some(g)) = (art.goblin_cf02_pose(index), state.enemies.get(index)) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let variant = goblin_variant(index);
        let flip = pose.flip.unwrap_or(g.facing < 0);
        let (cx, cy) = actor_center(
            cf02.art.goblins[variant].spec,
            g.x + art.goblin_draw_dx(index) as f32,
            g.y - ENEMY_HALF_HEIGHT,
            flip,
        );
        transform.translation.x = cx;
        transform.translation.y = cy;
        sprite.flip_x = flip;
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = pose.pose.frame;
        }
        *visibility = Visibility::Inherited;
        hide.push(index);
    }
    let mut hide_slammed: Vec<usize> = Vec::new();
    for (goblin, mut transform, mut sprite, mut visibility) in &mut slammed {
        let index = goblin.index;
        let (Some(pose), Some(g)) = (
            art.goblin_slammed_pose(index, slam_of(index)),
            state.enemies.get(index),
        ) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let variant = goblin_variant(index);
        let flip = g.facing < 0;
        let (cx, cy) = actor_center(
            cf02.art.slammed[variant].spec,
            g.x + art.goblin_draw_dx(index) as f32,
            g.y - ENEMY_HALF_HEIGHT,
            flip,
        );
        transform.translation.x = cx;
        transform.translation.y = cy;
        sprite.flip_x = flip;
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = pose.frame;
        }
        *visibility = Visibility::Inherited;
        hide_slammed.push(index);
    }
    // Copy the base sprite's z and tint (flash, corpse fade); a goblin the death
    // presenter hides (retired, exploded) stays hidden, so its replacement does.
    let mut keep: Vec<(usize, Transform, Color, bool)> = Vec::new();
    for (goblin, transform, sprite, mut visibility) in &mut base {
        let index = goblin.index;
        let hidden_by_presenter = *visibility == Visibility::Hidden;
        keep.push((index, *transform, sprite.color, hidden_by_presenter));
        if !hidden_by_presenter && (hide.contains(&index) || hide_slammed.contains(&index)) {
            *visibility = Visibility::Hidden;
        }
    }
    for (goblin, mut transform, mut sprite, mut visibility) in &mut acting {
        if let Some((_, base_transform, colour, hidden)) =
            keep.iter().find(|(i, ..)| *i == goblin.index)
        {
            transform.translation.z = base_transform.translation.z;
            sprite.color = *colour;
            if *hidden {
                *visibility = Visibility::Hidden;
            }
        }
    }
    for (goblin, mut transform, mut sprite, mut visibility) in &mut slammed {
        if let Some((_, base_transform, colour, hidden)) =
            keep.iter().find(|(i, ..)| *i == goblin.index)
        {
            transform.translation.z = base_transform.translation.z;
            sprite.color = *colour;
            if *hidden {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

/// DEATH-C: draw the presenter's state. Reads the presenter only; the death
/// pose replaces the goblin's GOB-01 sprite, and every piece sits under the
/// actors (`draw_z`), so nothing is ever drawn over Carl or Donut.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn sync_death_visuals(
    mut perf: Option<ResMut<Perf>>,
    death: Res<Death>,
    look: Option<Res<DeathLook>>,
    combat: Res<Combat>,
    sim: Res<Sim>,
    setup: Res<Setup>,
    mut goblins: Query<
        (&GoblinSprite, &mut Visibility, &mut Sprite),
        (Without<DeathPiece>, Without<LlamaSprite>),
    >,
    mut llamas: Query<
        (&LlamaSprite, &mut Visibility, &mut Sprite),
        (Without<DeathPiece>, Without<GoblinSprite>),
    >,
    mut pieces: Query<(&DeathPiece, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(presenter), Some(look), Some(state)) = (death.0.as_ref(), look, combat.0.as_ref())
    else {
        return;
    };
    let started = std::time::Instant::now();
    let actor_z = setup.kit.actor_z;
    let tick = sim.0.world.tick;

    // A death pose replaces the goblin sprite; a retired corpse is drawn no
    // more and a retiring one fades out (the GOB-01 corpse sprite included).
    // DEATH-D: an exploded goblin (presets 2 to 4) is replaced by its pieces.
    let hide_goblin = |index: usize| {
        presenter.pose_of(index as u32).is_some() || !presenter.draws_goblin(index as u32)
    };
    for (goblin, mut visibility, mut sprite) in &mut goblins {
        *visibility = if hide_goblin(goblin.index) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        sprite.color = sprite
            .color
            .with_alpha(presenter.corpse_alpha(goblin.index as u32));
    }
    // A llama's death pose replaces its sprite; a retired corpse is not drawn.
    let goblin_count = state.enemies.len();
    for (llama, mut visibility, mut sprite) in &mut llamas {
        let target = (goblin_count + llama.index) as u32;
        *visibility = if presenter.pose_of(target).is_some() || presenter.is_retired(target) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        sprite.color = sprite.color.with_alpha(presenter.corpse_alpha(target));
    }
    // Variant B draws from its own composed sheet (same layout, same frames);
    // the llama's parts have their own sheet and layout.
    let use_image = |sprite: &mut Sprite, image: &Handle<Image>| {
        if sprite.image != *image {
            sprite.image = image.clone();
        }
    };
    let use_sheet =
        |sprite: &mut Sprite, image: &Handle<Image>, layout: &Handle<TextureAtlasLayout>| {
            if sprite.image != *image {
                sprite.image = image.clone();
            }
            if let Some(atlas) = sprite.texture_atlas.as_mut()
                && atlas.layout != *layout
            {
                atlas.layout = layout.clone();
            }
        };
    let set_atlas = |sprite: &mut Sprite, frame: usize| {
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = frame;
        }
    };
    for (piece, mut transform, mut sprite, mut visibility) in &mut pieces {
        match *piece {
            DeathPiece::Pose(index) => match presenter.pose_of(index as u32) {
                Some(pose) => {
                    let (cx, cy) =
                        actor_center(look.pose_spec, pose.foot_x, pose.ground_y, pose.flip);
                    transform.translation = Vec3::new(
                        cx,
                        cy,
                        actor_z + GOBLIN_LYING_Z_OFFSET - index as f32 * GOBLIN_Z_STEP,
                    );
                    sprite.flip_x = pose.flip;
                    use_image(&mut sprite, &look.poses_images[pose.variant]);
                    sprite.color = if state.flash(index, tick) {
                        FLASH_TINT
                    } else {
                        Color::WHITE
                    }
                    .with_alpha(pose.alpha);
                    set_atlas(&mut sprite, pose.frame);
                    *visibility = Visibility::Inherited;
                }
                None => *visibility = Visibility::Hidden,
            },
            DeathPiece::LlamaPose(index) => {
                let target = (goblin_count + index) as u32;
                match presenter.pose_of(target) {
                    Some(pose) => {
                        let (cx, cy) = actor_center(
                            look.llama_pose_spec,
                            pose.foot_x,
                            pose.ground_y,
                            pose.flip,
                        );
                        transform.translation = Vec3::new(
                            cx,
                            cy,
                            actor_z + GOBLIN_LYING_Z_OFFSET - target as f32 * GOBLIN_Z_STEP,
                        );
                        sprite.flip_x = pose.flip;
                        sprite.color = if state.flash(target as usize, tick) {
                            FLASH_TINT
                        } else {
                            Color::WHITE
                        }
                        .with_alpha(pose.alpha);
                        set_atlas(&mut sprite, pose.frame);
                        *visibility = Visibility::Inherited;
                    }
                    None => *visibility = Visibility::Hidden,
                }
            }
            DeathPiece::Part(slot) => match presenter.part_at(slot) {
                Some(part) => {
                    transform.translation = Vec3::new(
                        part.x.round(),
                        part.y.round(),
                        draw_z(DrawLayer::Debris, slot, actor_z, look.wall_z),
                    );
                    transform.rotation = Quat::from_rotation_z(part.angle_deg.to_radians());
                    sprite.flip_x = part.flip;
                    use_sheet(
                        &mut sprite,
                        &look.parts_images[part.variant],
                        &look.parts_layouts[part.variant],
                    );
                    sprite.color = Color::srgba(1.0, 1.0, 1.0, presenter.part_alpha(slot, part));
                    set_atlas(&mut sprite, part.frame);
                    *visibility = Visibility::Inherited;
                }
                None => *visibility = Visibility::Hidden,
            },
            DeathPiece::Floor { slot, second } => {
                let decal = presenter.floor_decal_at(slot);
                let shown = decal.and_then(|d| {
                    let art = presenter.art_of(d.variant);
                    let meta = if second { d.pair_meta? } else { d.meta };
                    let offset = if second {
                        art.parts[d.meta]
                            .pair_offset
                            .map_or(0.0, |o| o[0] as f32 * d.scale)
                    } else {
                        0.0
                    };
                    Some((d, art.parts[meta].frame, offset))
                });
                match shown {
                    Some((d, frame, offset)) => {
                        transform.translation = Vec3::new(
                            (d.x + offset).round(),
                            d.y.round(),
                            draw_z(DrawLayer::FloorDecal, slot, actor_z, look.wall_z),
                        );
                        transform.scale = Vec3::new(d.scale, d.scale, 1.0);
                        sprite.flip_x = d.flip;
                        use_sheet(
                            &mut sprite,
                            &look.parts_images[d.variant],
                            &look.parts_layouts[d.variant],
                        );
                        sprite.color =
                            Color::srgba(1.0, 1.0, 1.0, presenter.floor_decal_alpha(slot, d));
                        set_atlas(&mut sprite, frame);
                        *visibility = Visibility::Inherited;
                    }
                    None => *visibility = Visibility::Hidden,
                }
            }
            DeathPiece::Wall(slot) => match presenter.wall_decal_at(slot) {
                Some(d) => {
                    transform.translation = Vec3::new(
                        d.x.round(),
                        d.y.round(),
                        draw_z(DrawLayer::WallDecal, slot, actor_z, look.wall_z),
                    );
                    transform.scale = Vec3::new(d.scale, d.scale, 1.0);
                    sprite.flip_x = d.flip;
                    use_sheet(
                        &mut sprite,
                        &look.parts_images[d.variant],
                        &look.parts_layouts[d.variant],
                    );
                    sprite.color = Color::srgba(1.0, 1.0, 1.0, presenter.wall_decal_alpha(slot, d));
                    set_atlas(&mut sprite, presenter.art_of(d.variant).parts[d.meta].frame);
                    *visibility = Visibility::Inherited;
                }
                None => *visibility = Visibility::Hidden,
            },
        }
    }
    if let Some(perf) = perf.as_mut()
        && perf.step_ticks
        && perf.seen >= perf.warmup
    {
        perf.death_sync_us
            .push(started.elapsed().as_secs_f32() * 1e6);
    }
}

fn attacker_of(hero: &Hero) -> Attacker {
    Attacker {
        x: hero.x,
        y: hero.y,
        half_height: HERO_HALF_HEIGHT,
        facing: hero.facing,
        player: 0,
    }
}

/// Derive this frame's lights from the view, the presentation tick and the
/// actors' positions, and hand them to the lighting stages. Reads simulation
/// state through shared references only; nothing flows back.
#[allow(clippy::too_many_arguments)]
fn update_lighting(
    rig: Option<Res<LightRig>>,
    sim: Res<Sim>,
    anim: Res<Anim>,
    actors: Res<Actors>,
    after_exit: Res<AfterExitTicks>,
    setup: Res<Setup>,
    reference: Res<ReferencePose>,
    combat: Res<Combat>,
    death: Res<Death>,
    cam: Res<SceneCam>,
    windows: Query<&Window, With<PrimaryWindow>>,
    materials: Option<ResMut<Assets<LightingMaterial>>>,
    mut dropped_reported: Local<bool>,
) {
    let (Some(rig), Some(mut materials), Ok(window)) = (rig, materials, windows.single()) else {
        return;
    };
    let kit = &setup.kit;
    let Some(scene) = kit.lighting.as_ref() else {
        return;
    };
    let state = &sim.0;
    let drawn = presentation_view(state, &setup, &reference, &combat, &cam);
    let tick = presentation_tick(state, &after_exit, &reference);
    let view = ViewRect {
        left: drawn.left,
        top: drawn.top,
        w: drawn.w,
        h: drawn.h,
    };
    let (gathered, dropped) = scene.gather(kit, view, tick);
    if dropped > 0 && !*dropped_reported {
        println!("FLOOR1_LIGHTING_WARNING: {dropped} lights over the pass limit were dropped");
        *dropped_reported = true;
    }
    let mut shadows: Vec<ShadowEllipse> = Vec::new();
    if let Some(profile) = scene.profile.actor_contact_shadow.as_ref() {
        for pose in actor_poses(state, &setup, &reference, &actors, &anim.0) {
            // The shadow lies on the walking line under the actor (also while
            // it jumps), centred on the footprint of the old idle canvas so the
            // wider locomotion frames do not change the lighting.
            let mut spec = pose.spec;
            if let (true, Some(art)) = (pose.is_hero, anim.0.art.as_ref())
                && let Some(carl_pose) = art.carl_pose()
            {
                // Carl's sprawl (knockdown) casts the shadow of the body on
                // the floor; every other frame keeps the idle footprint.
                let (width, pivot) = art.carl.shadow_footprint(&carl_pose);
                spec.shadow_width = width;
                spec.shadow_pivot_x = pivot;
            }
            let centre_x = shadow_center_x(spec, pose.foot_x, pose.flip);
            let line = kit
                .walking_line_at(pose.foot_x.round() as i32)
                .unwrap_or(kit.height - pose.foot_y.round() as i32);
            shadows.push(contact_shadow(
                profile,
                centre_x,
                line as f32,
                spec.shadow_width as f32,
                view,
            ));
        }
        // Combat fixture: goblins get the same contact shadow on the walking
        // line (also while airborne), shaped by the frame on the floor; Carl's
        // knocked-down sprawl follows the body the same way.
        if let (Some(art), Some(combat)) = (anim.0.art.as_ref(), combat.0.as_ref()) {
            let mut put = |clips: &SpriteClips,
                           pose: &dungeon_floorbreak::combat_anim::ArtPose,
                           x: f32,
                           flip: bool| {
                if shadows.len() >= MAX_SHADOWS {
                    return;
                }
                let (width, pivot) = clips.shadow_footprint(pose);
                let spec = ActorSpec {
                    shadow_width: width,
                    shadow_pivot_x: pivot,
                    ..clips.spec
                };
                let line = kit
                    .walking_line_at(x.round() as i32)
                    .unwrap_or(kit.base_walk_line_y);
                shadows.push(contact_shadow(
                    profile,
                    shadow_center_x(spec, x, flip),
                    line as f32,
                    width as f32,
                    view,
                ));
            };
            for (index, goblin) in combat.enemies.iter().enumerate() {
                // No shadow under a goblin whose body is hidden (an explosion)
                // or retired: nothing is drawn above it.
                if death
                    .0
                    .as_ref()
                    .is_some_and(|presenter| !presenter.draws_goblin(index as u32))
                {
                    continue;
                }
                put(
                    art.goblin_clips(index),
                    &art.goblin_pose(index),
                    goblin.x + art.goblin_draw_dx(index) as f32,
                    goblin.facing < 0,
                );
            }
            if let Some(clips) = art.llama_clips() {
                for (index, llama) in combat.llamas.iter().enumerate() {
                    if let Some(pose) = art.llama_pose(index) {
                        put(clips, &pose, llama.x, llama.facing < 0);
                    }
                }
            }
        }
    }
    let frame = PassFrame {
        scene,
        gathered: &gathered,
        shadows: &shadows,
        view_w: drawn.w as u32,
        view_h: drawn.h as u32,
        view_top: drawn.top,
    };
    let window_px = Vec2::new(
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    update_rig(&rig, &mut materials, &frame, window_px);
}

/// Frame-interval statistics for `--perf-frames`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FrameStats {
    frames: usize,
    median_ms: f32,
    p95_ms: f32,
    mean_ms: f32,
    max_ms: f32,
}

fn frame_stats(intervals_ms: &[f32]) -> Option<FrameStats> {
    if intervals_ms.is_empty() {
        return None;
    }
    let mut sorted = intervals_ms.to_vec();
    sorted.sort_by(f32::total_cmp);
    let n = sorted.len();
    // Nearest-rank percentiles.
    let rank = |q: f32| sorted[(((q * n as f32).ceil() as usize).max(1) - 1).min(n - 1)];
    Some(FrameStats {
        frames: n,
        median_ms: rank(0.5),
        p95_ms: rank(0.95),
        mean_ms: sorted.iter().sum::<f32>() / n as f32,
        max_ms: sorted[n - 1],
    })
}

/// GPU time (ms) of the last finished frame: for every timed pass, the sum of
/// all the measurements that share the newest timestamp. Several cameras (the
/// slot cameras, the post-stage quads) record the same pass names in one frame
/// and `Diagnostic::value` keeps only the last of them, so it is summed here.
fn gpu_ms_of_last_frame(diagnostics: &DiagnosticsStore) -> f64 {
    diagnostics
        .iter()
        .filter(|d| d.path().as_str().ends_with("elapsed_gpu"))
        .filter_map(|d| {
            let newest = d.measurement()?.time;
            Some(
                d.measurements()
                    .filter(|m| m.time == newest)
                    .map(|m| m.value)
                    .sum::<f64>(),
            )
        })
        .sum()
}

/// `--perf-from-tick`: after the warm-up, advance the fixture exactly one tick
/// per frame, before the frame is drawn (the frame time then covers the tick's
/// own presentation). Runs ahead of the visuals in the chain.
#[allow(clippy::too_many_arguments)]
fn perf_step(
    perf: Option<ResMut<Perf>>,
    loading: Option<Res<Loading>>,
    setup: Res<Setup>,
    mut sim: ResMut<Sim>,
    mut anim: ResMut<Anim>,
    mut combat: ResMut<Combat>,
    mut death: ResMut<Death>,
    mut cam: ResMut<SceneCam>,
    mut interest: ResMut<Interest>,
) {
    let Some(mut perf) = perf else { return };
    if !perf.step_ticks || loading.is_some_and(|l| !l.reported) || perf.seen < perf.warmup {
        return;
    }
    let (Some(fixture), Some(combat)) = (setup.args.fixture, combat.0.as_mut()) else {
        return;
    };
    let started = std::time::Instant::now();
    let raw = scripted_input(&setup.args, fixture, &sim.0, combat);
    let _ = step_combat(
        &mut sim.0,
        combat,
        &mut anim.0,
        death.0.as_mut(),
        [raw, RawInput::default()],
    );
    if setup.args.death_stress
        && let Some(death) = death.0.as_mut()
    {
        let hero = &sim.0.world.hero;
        death.stress_tick(hero.x, hero.y - HERO_HALF_HEIGHT);
    }
    perf.tick_us.push(started.elapsed().as_secs_f32() * 1e6);
    // This probe steps the simulation outside the fixed schedule.
    camera_tick(
        &mut cam,
        &mut interest,
        &setup,
        &sim.0,
        Some(&*combat),
        false,
    );
}

/// Resident and peak resident memory of this process in MiB (Linux
/// `/proc/self/status`; `None` elsewhere).
fn resident_memory_mib() -> Option<(f32, f32)> {
    let text = std::fs::read_to_string("/proc/self/status").ok()?;
    let kib = |key: &str| -> Option<f32> {
        let line = text.lines().find(|l| l.starts_with(key))?;
        line.split_whitespace().nth(1)?.parse::<f32>().ok()
    };
    Some((kib("VmRSS:")? / 1024.0, kib("VmHWM:")? / 1024.0))
}

#[allow(clippy::too_many_arguments)]
fn perf_probe(
    perf: Option<ResMut<Perf>>,
    time: Res<Time<Real>>,
    loading: Option<Res<Loading>>,
    window: Query<&Window, With<PrimaryWindow>>,
    lighting_on: Res<LightingOn>,
    death: Res<Death>,
    sim: Res<Sim>,
    entities: Query<Entity>,
    death_sprites: Query<&Visibility, With<DeathPiece>>,
    diagnostics: Res<DiagnosticsStore>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut perf) = perf else { return };
    // Measure only once the textures are in and the loading report is out.
    if loading.is_some_and(|l| !l.reported) {
        return;
    }
    perf.seen += 1;
    if perf.seen <= perf.warmup {
        return;
    }
    let ms = time.delta_secs() * 1000.0;
    perf.intervals_ms.push(ms);
    perf.gpu_ms.push(gpu_ms_of_last_frame(&diagnostics) as f32);
    if perf.step_ticks {
        let visible = death_sprites
            .iter()
            .filter(|v| **v != Visibility::Hidden)
            .count();
        perf.peak_visible_death_sprites = perf.peak_visible_death_sprites.max(visible);
        let (parts, floor, wall) = death.0.as_ref().map_or((0, 0, 0), |d| {
            (d.live_parts(), d.live_floor_decals(), d.live_wall_decals())
        });
        let tick = sim.0.world.tick;
        perf.series.push((tick, ms, parts, floor, wall, visible));
    }
    if perf.intervals_ms.len() >= perf.target {
        if let Some((rss, peak)) = resident_memory_mib() {
            println!(
                "FLOOR1_PERF_MEM rss_mib={rss:.1} peak_rss_mib={peak:.1} entities={} death_sprites_spawned={} peak_visible_death_sprites={}",
                entities.iter().count(),
                death_sprites.iter().count(),
                perf.peak_visible_death_sprites,
            );
        }
        let mean = |v: &[f32]| (!v.is_empty()).then(|| v.iter().sum::<f32>() / v.len() as f32);
        let gpu = frame_stats(&perf.gpu_ms);
        let tick = frame_stats(&perf.tick_us);
        let sync = frame_stats(&perf.death_sync_us);
        let gpu_paths: Vec<String> = diagnostics
            .iter()
            .filter(|d| d.path().as_str().ends_with("elapsed_gpu"))
            .map(|d| d.path().as_str().to_string())
            .collect();
        println!(
            "FLOOR1_PERF_CPU_GPU gpu_ms_mean={:.3} gpu_ms_p95={:.3} gpu_ms_max={:.3} tick_us_mean={:.1} tick_us_p95={:.1} tick_us_max={:.1} death_sync_us_mean={:.1} death_sync_us_p95={:.1} death_sync_us_max={:.1} gpu_passes={}",
            mean(&perf.gpu_ms).unwrap_or(0.0),
            gpu.map_or(0.0, |s| s.p95_ms),
            gpu.map_or(0.0, |s| s.max_ms),
            mean(&perf.tick_us).unwrap_or(0.0),
            tick.map_or(0.0, |s| s.p95_ms),
            tick.map_or(0.0, |s| s.max_ms),
            mean(&perf.death_sync_us).unwrap_or(0.0),
            sync.map_or(0.0, |s| s.p95_ms),
            sync.map_or(0.0, |s| s.max_ms),
            gpu_paths.len(),
        );
        println!("FLOOR1_PERF_GPU_PASSES {}", gpu_paths.join(" "));
        for (tick, ms, parts, floor, wall, visible) in &perf.series {
            println!(
                "FLOOR1_PERF_TICK tick={tick} ms={ms:.2} parts={parts} floor_decals={floor} wall_decals={wall} visible_death_sprites={visible}"
            );
        }
        if let Some(stats) = frame_stats(&perf.intervals_ms) {
            let size = window
                .single()
                .map(|w| format!("{}x{}", w.physical_width(), w.physical_height()))
                .unwrap_or_default();
            println!(
                "FLOOR1_PERF frames={} median_ms={:.2} p95_ms={:.2} mean_ms={:.2} max_ms={:.2} lighting={} window={size} present=AutoNoVsync warmup_frames={}",
                stats.frames,
                stats.median_ms,
                stats.p95_ms,
                stats.mean_ms,
                stats.max_ms,
                if lighting_on.0 { "on" } else { "off" },
                perf.warmup,
            );
            if let Some(death) = death.0.as_ref() {
                // The pools at the end of the run (loaded by `--death-stress`).
                println!(
                    "{} part_capacity={} floor_decal_capacity={} wall_decal_capacity={} recycled={:?}",
                    death.state_line(),
                    PART_POOL_CAPACITY,
                    FLOOR_DECAL_POOL_CAPACITY,
                    WALL_DECAL_POOL_CAPACITY,
                    death.recycled(),
                );
            }
        }
        exit.write(AppExit::Success);
    }
}

#[allow(clippy::too_many_arguments)]
fn watchdog(
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    time: Res<Time<Real>>,
    sim: Res<Sim>,
    anim: Res<Anim>,
    mut combat: ResMut<Combat>,
    loading: Option<ResMut<Loading>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut loading) = loading else { return };
    if loading.reported {
        return;
    }
    for (name, handle) in &loading.handles {
        if let LoadState::Failed(error) = asset_server.load_state(handle) {
            println!("FLOOR1_ASSET_FAILURE: {name}: {error}");
            exit.write(AppExit::error());
            loading.reported = true;
            return;
        }
    }
    if loading
        .handles
        .iter()
        .all(|(_, handle)| images.get(handle).is_some())
    {
        println!(
            "FLOOR1_TEXTURES_READY: {} atlases loaded; {}",
            loading.handles.len(),
            loading.counts
        );
        if let Some(combat) = combat.0.as_mut() {
            print_combat_events(combat, None, None);
        }
        println!("{}", sim.0.state_line());
        println!("{}", anim.0.state_line(sim.0.world.tick));
        if let Some(combat) = combat.0.as_ref() {
            print_combat_state(&sim.0, &anim.0, combat);
        }
        loading.reported = true;
    } else if time.elapsed_secs() > WATCHDOG_SECONDS {
        println!("FLOOR1_ASSET_FAILURE: textures not loaded within {WATCHDOG_SECONDS} s");
        exit.write(AppExit::error());
        loading.reported = true;
    }
}

// ------------------------------------------------------------ capture sequence

/// Render frames to wait after a state change before the screenshot request,
/// and the most frames to wait for the file to appear.
const CAPTURE_SETTLE_FRAMES: u32 = 4;
const CAPTURE_TIMEOUT_FRAMES: u32 = 900;
/// Frames the saved file must keep its size before the next step.
const CAPTURE_STABLE_FRAMES: u32 = 2;

#[derive(Debug, Default)]
struct PendingFile {
    waited: u32,
    last_len: u64,
    stable: u32,
}

#[derive(Debug)]
enum CapturePhase {
    /// Textures not loaded yet.
    WaitReady,
    /// Frames left before the next screenshot request.
    Settle(u32),
    /// Requested; waiting for the file, with the frames waited so far.
    Pending(PathBuf, PendingFile),
}

/// `--capture-sequence`: step the scenario by hand in the real render loop and
/// save a window screenshot every `step` ticks (the fixed step is frozen).
#[derive(Resource)]
struct CaptureSequence {
    step: u64,
    count: u64,
    dir: PathBuf,
    index: u64,
    phase: CapturePhase,
}

impl CaptureSequence {
    fn new(_first: u64, step: u64, count: u64, dir: PathBuf) -> Self {
        Self {
            step,
            count,
            dir,
            index: 0,
            phase: CapturePhase::WaitReady,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn capture_sequence(
    mut commands: Commands,
    seq: Option<ResMut<CaptureSequence>>,
    loading: Option<Res<Loading>>,
    setup: Res<Setup>,
    mut sim: ResMut<Sim>,
    mut anim: ResMut<Anim>,
    mut combat: ResMut<Combat>,
    mut death: ResMut<Death>,
    mut cam: ResMut<SceneCam>,
    mut interest: ResMut<Interest>,
    mut pads: ResMut<Pads>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut seq) = seq else { return };
    let (scenario, fixture) = (setup.args.scenario, setup.args.fixture);
    if scenario.is_none() && fixture.is_none() {
        return;
    }
    let fail = |exit: &mut MessageWriter<AppExit>, message: &str| {
        println!("FLOOR1_CAPTURE_FAILURE: {message}");
        exit.write(AppExit::error());
    };
    let phase = std::mem::replace(&mut seq.phase, CapturePhase::WaitReady);
    seq.phase = match phase {
        CapturePhase::WaitReady if loading.is_some_and(|l| l.reported) => {
            CapturePhase::Settle(CAPTURE_SETTLE_FRAMES)
        }
        CapturePhase::WaitReady => CapturePhase::WaitReady,
        CapturePhase::Settle(frames) if frames > 0 => CapturePhase::Settle(frames - 1),
        CapturePhase::Settle(_) => {
            if let Err(error) = std::fs::create_dir_all(&seq.dir) {
                fail(&mut exit, &format!("{}: {error}", seq.dir.display()));
                return;
            }
            let path = seq.dir.join(format!(
                "frame_{:04}_tick_{:05}.png",
                seq.index, sim.0.world.tick
            ));
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path.clone()));
            CapturePhase::Pending(path, PendingFile::default())
        }
        CapturePhase::Pending(path, mut file) => {
            // The file must exist, be non-empty and keep the same size for
            // CAPTURE_STABLE_FRAMES frames, so a still-growing PNG is not read.
            let len = std::fs::metadata(&path).map_or(0, |m| m.len());
            if len > 0 && len == file.last_len {
                file.stable += 1;
            } else {
                file.stable = 0;
            }
            file.last_len = len;
            if file.stable < CAPTURE_STABLE_FRAMES {
                file.waited += 1;
                if file.waited > CAPTURE_TIMEOUT_FRAMES {
                    fail(
                        &mut exit,
                        &format!("screenshot {} was never written", path.display()),
                    );
                    return;
                }
                seq.phase = CapturePhase::Pending(path, file);
                return;
            }
            println!(
                "FLOOR1_CAPTURE index={} tick={} file={}",
                seq.index,
                sim.0.world.tick,
                path.display()
            );
            if let Some(combat) = combat.0.as_mut() {
                print_combat_events(combat, None, None);
            }
            println!("{}", sim.0.state_line());
            println!("{}", anim.0.state_line(sim.0.world.tick));
            if let Some(combat) = combat.0.as_ref() {
                print_combat_state(&sim.0, &anim.0, combat);
            }
            seq.index += 1;
            if seq.index >= seq.count {
                println!("FLOOR1_CAPTURE_SEQUENCE_DONE count={}", seq.count);
                exit.write(AppExit::Success);
                return;
            }
            for _ in 0..seq.step {
                if let (Some(pf), Some(combat)) = (setup.args.party_fixture, combat.0.as_mut()) {
                    // CF02-F: a party fixture plays through the live party, like
                    // the presim of `build_scene` (slot inputs, solo swap, drive).
                    let party = pads.0.party_mut();
                    let slots = pf.inputs(sim.0.world.tick + 1, combat, &sim.0);
                    let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
                    party.request_swap(slots[0].swap, hurt);
                    sim.0.set_drive(pf.drive(party));
                    let inputs = party.hero_inputs(slots);
                    let restarts = combat.restarts;
                    let _ = step_combat(&mut sim.0, combat, &mut anim.0, death.0.as_mut(), inputs);
                    if combat.restarts != restarts {
                        party.restart();
                    }
                } else if let (Some(fixture), Some(combat)) = (fixture, combat.0.as_mut()) {
                    let inputs = scripted_inputs(&setup.args, fixture, &sim.0, combat);
                    let _ = step_combat(&mut sim.0, combat, &mut anim.0, death.0.as_mut(), inputs);
                } else if let Some(scenario) = scenario {
                    let raw = scenario.input(&sim.0.world);
                    sim.0.step(raw);
                    anim.0.step(&sim.0.world);
                }
                // One camera step per simulated tick, as in the live loop.
                camera_tick(
                    &mut cam,
                    &mut interest,
                    &setup,
                    &sim.0,
                    combat.0.as_ref(),
                    false,
                );
            }
            CapturePhase::Settle(CAPTURE_SETTLE_FRAMES)
        }
    };
}

// ------------------------------------------------------------ headless smoke

fn headless_smoke(kit: &EnvKit, args: &Args, actors: &Actors, root: &Path) -> Result<(), String> {
    if kit.solid_rects.is_empty() || kit.tiles.is_empty() {
        return Err("layout produced no tiles or no solids".into());
    }
    if args.view_trace {
        // The view trace runs on any layout (the corridor checks below assume
        // the sample corridor, not a camera test room).
        return view_trace(kit, args, actors, root);
    }
    if args.boss {
        println!(
            "FLOOR1_HEADLESS_OK: boss arena {} tiles, {} pieces, {} merged solids",
            kit.tiles.len(),
            kit.pieces.len(),
            kit.solid_rects.len()
        );
        return headless_fixture(kit, args, actors, root);
    }
    if args.demo || args.arena_room() {
        // The demo hall and the CF02 microarena have no exit and a different
        // spawn: the story-run checks below do not apply. Replay the fixture.
        println!(
            "FLOOR1_HEADLESS_OK: demo hall {} tiles, {} pieces, {} merged solids",
            kit.tiles.len(),
            kit.pieces.len(),
            kit.solid_rects.len()
        );
        return headless_fixture(kit, args, actors, root);
    }
    // Idle: Carl stays grounded on the walking line.
    let mut idle = SceneSim::new(kit, 360)?;
    idle.run_scenario(Scenario::Idle, 600);
    if !idle.world.hero.grounded || idle.world.hero.x != 72.0 {
        return Err("idle hero left the spawn".into());
    }
    // Scripted run: the same replay on both views reaches the exit.
    let mut desktop = SceneSim::new(kit, 360)?;
    let mut deck = SceneSim::new(kit, 400)?;
    let mut ticks = 0u64;
    while !desktop.world.complete {
        if ticks >= 2400 {
            return Err("walk_right did not reach the exit in 2400 ticks".into());
        }
        let raw = Scenario::WalkRight.input(&desktop.world);
        desktop.step(raw);
        let raw = Scenario::WalkRight.input(&deck.world);
        deck.step(raw);
        ticks += 1;
    }
    if desktop.world.state_hash() != deck.world.state_hash() {
        return Err("640x360 and 640x400 simulations diverged".into());
    }
    println!(
        "FLOOR1_HEADLESS_OK: {} tiles, {} pieces, {} merged solids, exit reached in {ticks} ticks",
        kit.tiles.len(),
        kit.pieces.len(),
        kit.solid_rects.len()
    );
    if args.fixture.is_some() {
        return headless_fixture(kit, args, actors, root);
    }
    let mut hud = new_hud(args, kit, actors);
    let (sim, anim, _, _, _) = build_scene(kit, args, actors, root, &mut hud)?;
    println!("{}", sim.state_line());
    println!("{}", anim.state_line(sim.world.tick));
    print_audience_state(&hud);
    Ok(())
}

/// Headless fixture replay (and the demo): the scripted run for
/// `--capture-tick` ticks (default the fixture's own length), every event,
/// then the state lines.
fn headless_fixture(kit: &EnvKit, args: &Args, actors: &Actors, root: &Path) -> Result<(), String> {
    if let Some(fixture) = args.fixture {
        let mut args = args.clone();
        args.capture_tick.get_or_insert(fixture.ticks);
        let mut hud = new_hud(&args, kit, actors);
        let (sim, anim, combat, death, party) = build_scene(kit, &args, actors, root, &mut hud)?;
        let mut combat = combat.ok_or("fixture without combat state")?;
        print_combat_events(&mut combat, None, None);
        println!("{}", sim.state_line());
        println!("{}", anim.state_line(sim.world.tick));
        print_combat_state(&sim, &anim, &combat);
        if let Some(party_fixture) = args.party_fixture {
            println!(
                "{}",
                party_scene::party_state_line(party_fixture, &combat, &sim, &party)
            );
        }
        print_audience_state(&hud);
        if args.death_trace
            && let Some(death) = death
        {
            println!("{}", death.state_line());
        }
        return Ok(());
    }
    let mut hud = new_hud(args, kit, actors);
    let (sim, anim, _, _, _) = build_scene(kit, args, actors, root, &mut hud)?;
    println!("{}", sim.state_line());
    println!("{}", anim.state_line(sim.world.tick));
    print_audience_state(&hud);
    Ok(())
}

/// `--view-trace`: one `FLOOR1_VIEW` line per simulated tick with the view the
/// presentation would draw (the CP-01 kick included), for the fixture's script
/// or the scenario (default: walk right for 600 ticks), then the final state
/// hash. The trace is the same for any lighting, audio or gore setting, and the
/// hash is the same for any `--camera`.
fn view_trace(kit: &EnvKit, args: &Args, actors: &Actors, root: &Path) -> Result<(), String> {
    let mut start = args.clone();
    start.capture_tick = Some(0);
    start.capture_sequence = None;
    start.perf_from_tick = None;
    let mut hud = new_hud(&start, kit, actors);
    let (mut sim, mut anim, mut combat, mut death, _party) =
        build_scene(kit, &start, actors, root, &mut hud)?;
    let ticks = args
        .capture_tick
        .or(args.fixture.map(|f| f.ticks))
        .unwrap_or(600);
    let window = (
        VIRTUAL_WIDTH as u32 * args.scale,
        view_height(args.deck) as u32 * args.scale,
    );
    let scene_camera = scene_camera_of(kit, args, &sim, combat.as_ref(), window);
    let mut rig = scene_camera.rig;
    println!(
        "FLOOR1_VIEW_TRACE camera={} output={}x{} slot={}x{}",
        args.camera_choice().label(),
        window.0,
        window.1,
        scene_camera.slot.0,
        scene_camera.slot.1
    );
    let scenario = args.scenario.unwrap_or(Scenario::WalkRight);
    for _ in 0..ticks {
        let mut cut = false;
        match (args.fixture, combat.as_mut()) {
            (Some(fixture), Some(combat)) => {
                // The demo trace is the autopilot's recorded run of the route.
                let raw = if args.demo && args.cf02.rules {
                    cf02::arena::autopilot(combat, &sim, cf02::arena::Plan::Direct)
                } else if args.demo {
                    demo::autopilot(combat, &sim)
                } else {
                    fixture.input(sim.world.tick + 1)
                };
                cut = step_combat(
                    &mut sim,
                    combat,
                    &mut anim,
                    death.as_mut(),
                    [raw, RawInput::default()],
                );
            }
            _ => {
                let raw = scenario.input(&sim.world);
                sim.step(raw);
                anim.step(&sim.world);
            }
        }
        let elapsed = elapsed_ticks(&sim, combat.as_ref());
        let kick = combat.as_ref().map_or(0, |c| c.camera_kick(sim.world.tick));
        match rig.as_mut() {
            Some(rig) => {
                let interest = test_interest(kit, args, &sim, combat.as_ref(), elapsed);
                if cut {
                    rig.request_cut();
                }
                rig.sync(&sim, combat.as_ref(), &interest, elapsed);
                println!("{}", rig.trace_line(elapsed, kick));
            }
            None => {
                let (left, top) = scroll_origin(&sim, kit.width, kick);
                println!(
                    "FLOOR1_VIEW tick={elapsed} left={left} top={top} w={VIRTUAL_WIDTH} h={} rest=0 kick={kick} kick_px={kick}",
                    view_height(args.deck)
                );
            }
        }
    }
    println!("{}", sim.state_line());
    if let Some(combat) = combat.as_ref() {
        println!(
            "FLOOR1_VIEW_TRACE_HASH combat={:016x}",
            combat.state_hash(&sim)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LLAMA-02 gate ruling 13.1, the blocker of the PR B review: the stone is
    /// drawn over the crushed llama only if it is on the actors render layer
    /// (the prop slot is composed below the actors slot, so a higher z alone
    /// would leave the llama in front). Over: the actors layer; otherwise the
    /// prop layer. Also the z order inside the actors layer: above every
    /// lying body, below Donut (0) and Carl.
    #[test]
    fn the_block_joins_the_actors_layer_while_it_is_drawn_over_a_crushed_enemy() {
        let (prop, actors) = (RenderLayers::layer(3), RenderLayers::layer(4));
        assert_eq!(block_layers(true, &prop, &actors), &actors);
        assert_eq!(block_layers(false, &prop, &actors), &prop);
        assert_ne!(prop, actors);
        // Lying bodies sit at GOBLIN_LYING_Z_OFFSET minus a step per target; the
        // block above all of them (even 64 targets) and below Donut and Carl.
        const {
            assert!(HAZARD_OVER_Z_OFFSET > GOBLIN_LYING_Z_OFFSET);
            assert!(HAZARD_OVER_Z_OFFSET < 0.0 && HAZARD_OVER_Z_OFFSET < CARL_Z_OFFSET);
        }
    }

    fn args(list: &[&str]) -> Result<Args, String> {
        let mut full = vec!["floor01_scene".to_owned()];
        full.extend(list.iter().map(|s| (*s).to_owned()));
        parse_args_with(&full, HudSettings::default())
    }

    /// PLAYTRACE-01 (PT-Q12): which runs record by default.
    #[test]
    fn recording_defaults_on_for_live_input_and_off_for_scripted_paths() {
        let records = |list: &[&str]| args(list).unwrap().recording_enabled();
        // Live-input runs: story play, the manual demo, CF02, a manual fixture.
        assert!(records(&[]));
        assert!(records(&["--scale", "3"]));
        assert!(records(&["--demo"]));
        assert!(records(&["--demo", "--demo-beat", "3"]));
        assert!(records(&["--cf02"]));
        assert!(records(&["--fixture", "cp01_single_goblin", "--manual"]));
        assert!(records(&["--scene", "boss_f01"]));
        // Scripted, headless, capture, perf and probe paths: off.
        for scripted in [
            &["--headless-floor01-smoke"][..],
            &[
                "--headless-floor01-smoke",
                "--scenario",
                "walk_right",
                "--capture-tick",
                "80",
            ],
            &[
                "--headless-floor01-smoke",
                "--fixture",
                "cp01_single_goblin",
            ],
            &[
                "--headless-floor01-smoke",
                "--party-fixture",
                "duo_independent_moves",
            ],
            &["--scenario", "walk_right"],
            &["--scenario", "walk_right", "--exit-after-ticks", "60"],
            &["--fixture", "cp01_single_goblin"],
            &[
                "--fixture",
                "cp01_single_goblin",
                "--exit-after-ticks",
                "60",
            ],
            &["--demo", "--autopilot"],
            &["--demo", "--autopilot", "--exit-after-ticks", "600"],
            &["--demo", "--exit-after-ticks", "600"],
            &[
                "--scenario",
                "run_jump",
                "--capture-sequence",
                "10:1:5",
                "--capture-dir",
                "x",
            ],
            &["--reference-frame", "--scale", "3"],
            &["--freeze"],
            &["--perf-frames", "100"],
            &["--hud-preview", "coop"],
            &["--audience-probe"],
            &["--camera-test-room", "1"],
            &["--headless-floor01-smoke", "--view-trace"],
        ] {
            let parsed = args(scripted).unwrap_or_else(|e| panic!("{scripted:?}: {e}"));
            assert!(!parsed.recording_enabled(), "{scripted:?} must not record");
        }
        // Explicit switches.
        assert!(!records(&["--no-record"]));
        assert!(!records(&["--demo", "--no-record"]));
        assert!(records(&["--scenario", "walk_right", "--record"]));
        assert!(args(&["--headless-floor01-smoke", "--record"]).is_err());
        assert!(
            args(&[
                "--scenario",
                "walk_right",
                "--capture-tick",
                "5",
                "--record"
            ])
            .is_err()
        );
        assert!(args(&["--record-dir"]).is_err());
        let dir = args(&["--record-dir", "somewhere"]).unwrap();
        assert_eq!(
            dir.record_dir.as_deref(),
            Some(std::path::Path::new("somewhere"))
        );
    }

    /// AUDIENCE-01 R1: a CINE chain award reaches the viewer count on every
    /// path. The fast-forward of a headless replay (and of every capture)
    /// and the live loop (the single fan-out of `fixed_step`) must end in the
    /// same audience state for the same inputs. `cp03_goblin_pileup` earns a
    /// mixed-chain CINE moment (score 7, credited to Carl). Without the feed
    /// in `build_scene` the headless Carl is 500 short of the live one.
    #[test]
    fn a_cine_chain_award_reaches_the_audience_on_the_headless_and_live_paths() {
        const TICKS: u64 = 260;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(root, LAYOUT_PATH).unwrap();
        let actors = Actors::load(root, FrameSource::GridSource, true, false, false).unwrap();
        let base = [
            "--headless-floor01-smoke",
            "--fixture",
            "cp03_goblin_pileup",
            "--audience-probe",
        ];

        // Headless: the fast-forward of `build_scene`.
        let mut fast = base.to_vec();
        let ticks = TICKS.to_string();
        fast.extend(["--capture-tick", ticks.as_str()]);
        let fast = args(&fast).unwrap();
        let mut hud_fast = new_hud(&fast, &kit, &actors);
        build_scene(&kit, &fast, &actors, root, &mut hud_fast).unwrap();

        // Live-equivalent: the order of `fixed_step` (step, single fan-out,
        // chain feats queued, HUD step), with the presentation observer.
        let live = args(&base).unwrap();
        let mut hud_live = new_hud(&live, &kit, &actors);
        let (mut sim, mut anim, combat, death, _) =
            build_scene(&kit, &live, &actors, root, &mut hud_live).unwrap();
        let (mut combat, mut death) = (combat.unwrap(), death.unwrap());
        let fixture = live.fixture.unwrap();
        let mut adapter = CineAdapter::new(PresentationSettings::DEFAULT);
        let mut awards = 0;
        for _ in 0..TICKS {
            let input = scripted_input(&live, fixture, &sim, &combat);
            step_combat(
                &mut sim,
                &mut combat,
                &mut anim,
                Some(&mut death),
                [input, RawInput::default()],
            );
            for feat in print_combat_events(&mut combat, None, Some(&mut adapter)) {
                awards += 1;
                hud_live.push_chain_feat(feat);
            }
            step_hud(&mut hud_live, &sim, Some(&combat));
        }

        assert!(awards > 0, "the fixture must earn a CINE chain feat");
        let (fast_view, live_view) = (hud_fast.audience(), hud_live.audience());
        assert_eq!(
            (
                fast_view.count(HeroId::Carl),
                fast_view.count(HeroId::Donut)
            ),
            (
                live_view.count(HeroId::Carl),
                live_view.count(HeroId::Donut)
            ),
        );
        assert_eq!(fast_view.steps(), live_view.steps());
        assert_eq!(fast_view.digest(), live_view.digest());
    }

    #[test]
    fn a_party_fixture_is_a_headless_or_capture_run_and_excludes_a_plain_fixture() {
        let a = args(&[
            "--headless-floor01-smoke",
            "--party-fixture",
            "donut_defeat_party",
        ])
        .unwrap();
        assert_eq!(
            a.party_fixture.map(|f| f.name()),
            Some("donut_defeat_party")
        );
        assert_eq!(a.fixture.map(|f| f.name), Some("donut_defeat_party"));
        assert!(
            args(&[
                "--party-fixture",
                "duo_independent_moves",
                "--capture-tick",
                "60"
            ])
            .is_ok()
        );
        assert!(args(&["--party-fixture", "nope", "--capture-tick", "60"]).is_err());
        // A live party fixture run, or one mixed with other modes, is refused.
        assert!(args(&["--party-fixture", "duo_independent_moves"]).is_err());
        assert!(
            args(&[
                "--party-fixture",
                "duo_independent_moves",
                "--fixture",
                "cp01_kick",
                "--capture-tick",
                "60"
            ])
            .is_err()
        );
        assert!(
            args(&[
                "--fixture",
                "cp01_kick",
                "--party-fixture",
                "duo_independent_moves",
                "--capture-tick",
                "60"
            ])
            .is_err()
        );
        assert!(
            args(&[
                "--party-fixture",
                "duo_independent_moves",
                "--capture-tick",
                "60",
                "--manual"
            ])
            .is_err()
        );
    }

    #[test]
    fn the_hud_flags_parse_both_forms_and_reject_unknown_values() {
        assert_eq!(args(&[]).unwrap().hud_preview, None);
        assert_eq!(args(&[]).unwrap().glyphs, None);
        assert_eq!(
            args(&["--glyphs=xbox"]).unwrap().glyphs,
            Some(GlyphSet::Xbox)
        );
        assert_eq!(args(&[]).unwrap().hud_settings, HudSettings::default());
        for preview in HudPreview::ALL {
            let spaced = args(&["--hud-preview", preview.name()]).unwrap();
            let equals = args(&[&format!("--hud-preview={}", preview.name())]).unwrap();
            assert_eq!(spaced.hud_preview, Some(preview));
            assert_eq!(equals.hud_preview, Some(preview));
        }
        assert!(
            args(&["--hud-preview", "nope"])
                .unwrap_err()
                .contains("expected")
        );
        assert!(args(&["--hud-preview=nope"]).is_err());
        assert!(args(&["--hud-preview"]).is_err());
        assert_eq!(
            args(&["--glyphs=dualsense"]).unwrap().glyphs,
            Some(GlyphSet::DualSense)
        );
        assert_eq!(
            args(&["--glyphs", "dualsense"]).unwrap().glyphs,
            Some(GlyphSet::DualSense)
        );
        assert!(args(&["--glyphs=ps5"]).is_err());
        assert!(
            args(&["--hud-reduced-motion"])
                .unwrap()
                .hud_settings
                .reduced_motion
        );
        let both = args(&["--hud-reduced-motion=on", "--hud-high-contrast"]).unwrap();
        assert!(both.hud_settings.reduced_motion && both.hud_settings.high_contrast);
        let env = HudSettings {
            reduced_motion: true,
            high_contrast: true,
        };
        let full: Vec<String> = ["floor01_scene", "--hud-high-contrast=off"]
            .iter()
            .map(|a| a.to_string())
            .collect();
        let overridden = parse_args_with(&full, env).unwrap().hud_settings;
        assert!(overridden.reduced_motion && !overridden.high_contrast);
        assert!(args(&["--hud-high-contrast=maybe"]).is_err());
    }

    #[test]
    fn the_gore_preset_flag_defaults_to_full_and_is_strict() {
        assert_eq!(args(&[]).unwrap().gore_preset, GorePreset::Full);
        for (value, preset) in [
            ("1", GorePreset::Off),
            ("2", GorePreset::Reduced),
            ("3", GorePreset::Medium),
            ("4", GorePreset::Full),
        ] {
            let parsed = args(&["--fixture", "cp03_shore_kill", "--gore-preset", value]).unwrap();
            assert_eq!(parsed.gore_preset, preset);
        }
        for bad in ["0", "5", "gory", ""] {
            assert!(args(&["--gore-preset", bad]).is_err(), "{bad:?}");
        }
        assert!(args(&["--gore-preset"]).is_err());
        // The lab flags need a fixture; stress is live only; the trace is headless only.
        assert!(args(&["--death-stress"]).is_err());
        // DEATH-D: the decapitation lab flag is gone (it is an unknown argument).
        assert!(args(&["--death-lab-finish", "decapitated"]).is_err());
        assert!(
            args(&[
                "--fixture",
                "cp03_shore_kill",
                "--death-lab-finish",
                "decapitated"
            ])
            .is_err()
        );
        assert!(args(&["--fixture", "cp03_shore_kill", "--death-trace"]).is_err());
        assert!(
            args(&[
                "--fixture",
                "cp03_shore_kill",
                "--headless-floor01-smoke",
                "--death-trace"
            ])
            .is_ok()
        );
        assert!(
            args(&[
                "--fixture",
                "cp03_shore_kill",
                "--headless-floor01-smoke",
                "--death-stress"
            ])
            .is_err()
        );
    }

    #[test]
    fn the_death_layers_draw_under_every_scene_actor() {
        // The scene's own actor offsets: Carl +0.1, Donut 0, a goblin
        // +0.05 / +0.15 (swinging), a lying goblin -0.01 minus 0.001 per index
        // (64 goblins is far more than any fixture).
        let lowest_actor = 7.0 + GOBLIN_LYING_Z_OFFSET - GOBLIN_Z_STEP * 64.0;
        let slots = PART_POOL_SLOTS.max(FLOOR_DECAL_POOL_SLOTS);
        for slot in [0, slots - 1] {
            for layer in [DrawLayer::Debris, DrawLayer::FloorDecal, DrawLayer::Blast] {
                assert!(draw_z(layer, slot, 7.0, 3.0) < lowest_actor, "{layer:?}");
            }
        }
        // DEATH-D: the charge and the burst sit between the floor decals and
        // the debris (every charge slot is below the first debris piece).
        for slot in 0..8 {
            let blast = draw_z(DrawLayer::Blast, slot, 7.0, 3.0);
            assert!(blast > draw_z(DrawLayer::FloorDecal, slots - 1, 7.0, 3.0));
            assert!(blast < draw_z(DrawLayer::Debris, 0, 7.0, 3.0));
        }
        // The preset 4 burst is over the debris and under every actor; the lit
        // charge is over a standing goblin (+0.05) and under Carl (+0.1).
        let debris_top = draw_z(DrawLayer::Debris, PART_POOL_SLOTS, 7.0, 3.0);
        for slot in 0..8 {
            let front = draw_z(DrawLayer::BlastFront, slot, 7.0, 3.0);
            assert!(front > debris_top && front < lowest_actor);
            let lit = draw_z(DrawLayer::ChargeLit, slot, 7.0, 3.0);
            assert!(lit > 7.0 + 0.05 && lit < 7.0 + CARL_Z_OFFSET);
        }
        assert!(draw_z(DrawLayer::WallDecal, WALL_DECAL_POOL_SLOTS, 7.0, 3.0) < 4.0);
    }

    #[test]
    fn lighting_flags_parse() {
        let a = args(&["--no-lighting", "--scale", "3"]).unwrap();
        assert!(a.no_lighting && a.scale == 3);
        let a = args(&["--reference-frame", "--perf-frames", "300"]).unwrap();
        assert!(a.reference_frame && a.perf_frames == Some(300));
        assert!(!args(&[]).unwrap().no_lighting);
        assert!(args(&["--perf-frames", "3"]).is_err());
        assert!(args(&["--perf-frames"]).is_err());
        assert!(args(&["--reference-frame", "--scenario", "idle"]).is_err());
    }

    #[test]
    fn cinematic_finishes_is_on_for_live_play_and_off_for_automated_runs() {
        // Live play (and a live manual fixture): on by default.
        assert!(args(&[]).unwrap().cinematic_settings().cinematic_finishes);
        assert!(
            args(&["--fixture", "cp03_goblin_pileup", "--manual"])
                .unwrap()
                .cinematic_settings()
                .cinematic_finishes
        );
        // Scripted fixtures, scenarios, autopilot runs and captures: off, so
        // no committed golden slows down unless the run asks for it.
        for list in [
            &["--fixture", "cp03_goblin_pileup"][..],
            &["--scenario", "walk_right"],
            &["--scenario", "walk_right", "--exit-after-ticks", "250"],
            &["--demo", "--autopilot"],
            &["--perf-frames", "40"],
        ] {
            let a = args(list).unwrap();
            assert!(!a.cinematic_settings().cinematic_finishes, "{list:?}");
        }
        // The flag wins in either direction and `=value` parses.
        let on = args(&["--fixture", "cp03_goblin_pileup", "--cinematic-finishes"]).unwrap();
        assert!(on.cinematic_settings().cinematic_finishes);
        let off = args(&["--cinematic-finishes=off"]).unwrap();
        assert!(!off.cinematic_settings().cinematic_finishes);
        assert!(args(&["--cinematic-finishes=sometimes"]).is_err());
        // The HUD flag is a different setting (review F5).
        let hud = args(&["--hud-reduced-motion"]).unwrap();
        assert!(hud.cinematic_settings().cinematic_finishes);
    }

    #[test]
    fn exit_after_ticks_needs_a_live_scenario() {
        let a = args(&["--scenario", "walk_right", "--exit-after-ticks", "250"]).unwrap();
        assert_eq!(a.exit_after_ticks, Some(250));
        assert!(args(&["--exit-after-ticks", "250"]).is_err());
        assert!(args(&["--scenario", "idle", "--exit-after-ticks", "0"]).is_err());
        assert!(
            args(&[
                "--scenario",
                "idle",
                "--capture-tick",
                "5",
                "--exit-after-ticks",
                "9"
            ])
            .is_err()
        );
        assert!(args(&["--scenario", "idle", "--freeze", "--exit-after-ticks", "9"]).is_err());
    }

    #[test]
    fn capture_sequence_flags_parse_and_validate() {
        let a = args(&[
            "--scenario",
            "run_jump",
            "--capture-sequence",
            "142:3:81",
            "--capture-dir",
            "frames",
        ])
        .unwrap();
        assert_eq!(a.capture_sequence, Some((142, 3, 81)));
        assert_eq!(a.capture_dir, Some(PathBuf::from("frames")));
        assert_eq!(a.scenario, Some(Scenario::RunJump));
        let bad = |list: &[&str]| args(list).is_err();
        // Needs both flags and a scenario.
        assert!(bad(&[
            "--scenario",
            "run_jump",
            "--capture-sequence",
            "0:3:5"
        ]));
        assert!(bad(&["--capture-sequence", "0:3:5", "--capture-dir", "d"]));
        // Malformed or degenerate sequences.
        for sequence in ["0:3", "0:0:5", "0:3:0", "0:3:5:7", "a:b:c", "0:3:1001"] {
            assert!(
                bad(&[
                    "--scenario",
                    "run_jump",
                    "--capture-sequence",
                    sequence,
                    "--capture-dir",
                    "d"
                ]),
                "{sequence}"
            );
        }
        // Mutually exclusive with the frozen and live modes.
        for extra in [
            ["--capture-tick", "5"],
            ["--exit-after-ticks", "9"],
            ["--freeze", ""],
        ] {
            let mut list = vec![
                "--scenario",
                "run_jump",
                "--capture-sequence",
                "0:3:5",
                "--capture-dir",
                "d",
            ];
            list.extend(extra.iter().filter(|s| !s.is_empty()));
            assert!(bad(&list), "{extra:?}");
        }
    }

    #[test]
    fn the_depth_order_follows_the_melee_integration_rule() {
        // Lying goblins below Donut (z 0), then Donut, then standing goblins
        // (the earlier goblin in front of a stacked later one), then Carl (his
        // fist must not be hidden by a goblin's head), then a goblin in its
        // smear or impact (its club lands on him).
        const { assert!(GOBLIN_LYING_Z_OFFSET + 8.0 * GOBLIN_Z_STEP < 0.0) };
        const { assert!(0.0 < GOBLIN_Z_OFFSET - 8.0 * GOBLIN_Z_STEP) };
        const { assert!(GOBLIN_Z_OFFSET < CARL_Z_OFFSET) };
        const { assert!(CARL_Z_OFFSET < GOBLIN_SWING_Z_OFFSET) };
        // Which side wins is the library's rule.
        assert!(carl_above_goblin(true, true));
        assert!(!carl_above_goblin(false, true));
        assert!(carl_above_goblin(false, false));
    }

    #[test]
    fn combat_trace_is_headless_fixture_only() {
        let a = args(&[
            "--headless-floor01-smoke",
            "--fixture",
            "cp01_two_hits",
            "--combat-trace",
        ])
        .unwrap();
        assert!(a.combat_trace);
        assert!(args(&["--combat-trace"]).is_err());
        assert!(args(&["--fixture", "cp01_two_hits", "--combat-trace"]).is_err());
    }

    #[test]
    fn fixture_flags_parse_and_validate() {
        let a = args(&["--fixture", "cp01_single_goblin", "--debug-combat"]).unwrap();
        assert_eq!(a.fixture.map(|f| f.name), Some("cp01_single_goblin"));
        assert!(a.debug_combat && !a.manual);
        assert!(args(&["--fixture", "cp01_single_goblin", "--manual"]).is_ok());
        assert!(args(&["--fixture", "nope"]).is_err());
        assert!(args(&["--fixture"]).is_err());
        // Needs a fixture; exclusive with a scenario; manual is a live run.
        assert!(args(&["--manual"]).is_err());
        assert!(args(&["--debug-combat"]).is_err());
        assert!(args(&["--fixture", "cp01_two_hits", "--scenario", "idle"]).is_err());
        assert!(
            args(&[
                "--fixture",
                "cp01_two_hits",
                "--manual",
                "--exit-after-ticks",
                "9"
            ])
            .is_err()
        );
        assert!(args(&["--fixture", "cp01_two_hits", "--capture-tick", "9"]).is_ok());
        assert!(args(&["--fixture", "cp01_two_hits", "--exit-after-ticks", "9"]).is_ok());
        assert!(args(&["--fixture", "cp01_two_hits", "--reference-frame"]).is_err());
    }

    #[test]
    fn anim_trace_is_headless_only() {
        let a = args(&[
            "--headless-floor01-smoke",
            "--scenario",
            "run_jump",
            "--anim-trace",
        ])
        .unwrap();
        assert!(a.anim_trace);
        assert!(args(&["--scenario", "run_jump", "--anim-trace"]).is_err());
        assert!(args(&["--headless-floor01-smoke", "--anim-trace"]).is_err());
    }

    #[test]
    fn presim_covers_the_capture_tick_and_the_sequence_start() {
        let a = args(&["--scenario", "idle", "--capture-tick", "7"]).unwrap();
        assert_eq!(presim_ticks(&a), Some(7));
        let a = args(&[
            "--scenario",
            "idle",
            "--capture-sequence",
            "142:3:2",
            "--capture-dir",
            "d",
        ])
        .unwrap();
        assert_eq!(presim_ticks(&a), Some(142));
        assert_eq!(presim_ticks(&args(&[]).unwrap()), None);
    }

    #[test]
    fn frame_stats_use_nearest_rank_percentiles() {
        assert_eq!(frame_stats(&[]), None);
        let values: Vec<f32> = (1..=100).map(|v| v as f32).collect();
        let s = frame_stats(&values).unwrap();
        assert_eq!(
            (s.frames, s.median_ms, s.p95_ms, s.max_ms),
            (100, 50.0, 95.0, 100.0)
        );
        assert_eq!(s.mean_ms, 50.5);
    }

    #[test]
    fn reference_pose_matches_the_layout_preview() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(LAYOUT_PATH),
        )
        .unwrap();
        let layout: serde_json::Value = serde_json::from_str(&text).unwrap();
        let check = layout["preview"]["actor_check"].as_array().unwrap();
        assert_eq!(check[0]["x"].as_f64().unwrap() as f32, REFERENCE_CARL_X);
        assert_eq!(check[1]["x"].as_f64().unwrap() as f32, REFERENCE_DONUT_X);
        assert_eq!(layout["preview"]["primary_view"], REFERENCE_VIEW);
        assert!(layout["views"][REFERENCE_VIEW_DECK].is_array());
    }

    #[test]
    fn flags_parse() {
        let a = args(&[
            "--deck",
            "--scenario",
            "walk_right",
            "--capture-tick",
            "420",
            "--debug-collision",
        ])
        .unwrap();
        assert!(a.deck && a.debug_collision);
        assert_eq!(a.capture_tick, Some(420));
        assert_eq!(a.scenario, Some(Scenario::WalkRight));
        assert!(args(&["--capture-tick", "5"]).is_err());
        assert!(args(&["--scenario", "fly"]).is_err());
        assert!(args(&["--bogus"]).is_err());
        assert_eq!(args(&[]).unwrap().scale, 2);
    }

    #[test]
    fn the_automatic_camera_is_the_demo_default_and_nothing_else_changes() {
        let demo = args(&["--demo", "--scale", "3"]).unwrap();
        assert_eq!(demo.camera_choice(), CameraChoice::Auto);
        let scroll = args(&["--demo", "--camera", "scroll", "--scale", "3"]).unwrap();
        assert_eq!(scroll.camera_choice(), CameraChoice::Scroll);
        // Every fixture, scenario and capture keeps the scroll camera.
        for list in [
            vec!["--fixture", "cp01_kick"],
            vec!["--scenario", "walk_right"],
            vec![],
        ] {
            assert_eq!(args(&list).unwrap().camera_choice(), CameraChoice::Scroll);
        }
        // The test room and the demo hall are two different rooms.
        assert!(args(&["--demo", "--camera-test-room", "1"]).is_err());
        assert!(args(&["--camera-test-room", "1"]).is_ok());
    }

    #[test]
    fn the_demo_flags_parse_and_validate() {
        let a = args(&["--demo", "--scale", "3"]).unwrap();
        assert!(a.demo && a.manual && !a.autopilot);
        assert_eq!(a.fixture.map(|f| f.name), Some("demo01"));
        assert_eq!(a.gore_preset, GorePreset::Full, "preset 4 by default");
        let a = args(&["--demo", "--demo-beat", "3", "--scale", "3"]).unwrap();
        assert_eq!(a.fixture.map(|f| f.name), Some("demo01_b3"));
        // A scripted run is never manual.
        let a = args(&[
            "--demo",
            "--demo-beat",
            "1",
            "--autopilot",
            "--headless-floor01-smoke",
        ])
        .unwrap();
        assert!(!a.manual && a.autopilot && a.headless);
        let a = args(&["--demo", "--autopilot", "--exit-after-ticks", "9"]).unwrap();
        assert!(!a.manual);
        assert!(
            args(&[
                "--demo",
                "--autopilot",
                "--capture-sequence",
                "5:1:1",
                "--capture-dir",
                "d"
            ])
            .is_ok()
        );
        // Beat and autopilot need --demo; the beat is 0 to 6; --demo replaces the others.
        assert!(args(&["--demo-beat", "3"]).is_err());
        assert!(args(&["--autopilot"]).is_err());
        assert!(args(&["--demo", "--demo-beat", "7"]).is_err());
        assert!(args(&["--demo", "--demo-beat"]).is_err());
        assert!(args(&["--demo", "--fixture", "cp01_kick"]).is_err());
        assert!(args(&["--demo", "--scenario", "idle"]).is_err());
        assert!(args(&["--demo", "--reference-frame"]).is_err());
    }

    #[test]
    fn cf02_flags_select_the_microarena_and_carry_the_seed() {
        let plain = args(&["--demo", "--scale", "3"]).unwrap();
        assert_eq!(plain.cf02, Cf02Config::OFF);
        let cf02 = args(&["--cf02", "--scale", "3"]).unwrap();
        assert!(cf02.cf02.rules && cf02.cf02.seed.is_none());
        // CF02-F: the microarena fixture on the demo machinery (the checkpoint
        // respawn, Select, the autopilot), with the demo's default camera.
        assert!(cf02.demo);
        assert_eq!(cf02.fixture.map(|f| f.name), Some("cf02_arena"));
        assert!(cf02.arena_room() && !plain.arena_room());
        assert_eq!(cf02.camera_choice(), plain.camera_choice());
        let later = args(&["--cf02", "--demo-beat", "1"]).unwrap();
        assert_eq!(later.fixture.map(|f| f.name), Some("cf02_arena_cp2"));
        let seeded = args(&["--cf02", "--cf02-seed", "18446744073709551615"]).unwrap();
        assert_eq!(seeded.cf02.seed, Some(u64::MAX));
        assert!(
            args(&["--cf02-seed", "7"]).is_err(),
            "the seed needs --cf02"
        );
        assert!(args(&["--cf02", "--cf02-seed", "-1"]).is_err());
        assert!(args(&["--cf02", "--cf02-seed"]).is_err());
        // The demo options keep working under the flag.
        assert!(args(&["--cf02", "--demo-beat", "3"]).is_ok());
        assert!(args(&["--cf02", "--fixture", "cp01_kick"]).is_err());
    }

    /// ENEMY-GAMEPLAY-ENGINEER (ENG-G-Q25): `--cf02-engineer` is the `--cf02` room
    /// with the Engineer. It needs `--cf02` and is refused by `--demo`, the lab,
    /// the boss arena and every fixture; plain `--cf02` never loads him.
    #[test]
    fn cf02_engineer_selects_the_engineer_room_and_nothing_else_loads_him() {
        let plain = args(&["--cf02"]).unwrap();
        assert!(!plain.cf02_engineer);
        assert_eq!(plain.fixture.map(|f| f.name), Some("cf02_arena"));
        let eng = args(&["--cf02", "--cf02-engineer"]).unwrap();
        assert!(eng.cf02_engineer && eng.arena_room());
        assert_eq!(eng.fixture.map(|f| f.name), Some("cf02_arena_eng"));
        let later = args(&["--cf02", "--cf02-engineer", "--demo-beat", "1"]).unwrap();
        assert_eq!(later.fixture.map(|f| f.name), Some("cf02_arena_eng_cp2"));
        assert!(args(&["--cf02-engineer"]).is_err(), "needs --cf02");
        for other in [
            &["--demo"][..],
            &["--scene", "combat_lab"][..],
            &["--scene", "boss_f01"][..],
            &["--fixture", "cp01_kick"][..],
            &["--party-fixture", "eng_snap_rush"][..],
            &["--scenario", "walk_right_400"][..],
        ] {
            let mut list = vec!["--cf02", "--cf02-engineer"];
            list.extend_from_slice(other);
            assert!(args(&list).is_err(), "{list:?} must be rejected");
        }
        // The story route never loads him (it has no flag to).
        assert!(!args(&[]).unwrap().cf02_engineer);
    }

    /// COMBAT-LAB-01 LAB-00-wire: the lab entry is its own run mode. Story,
    /// `--demo` and plain `--cf02` cannot select the lab profile or any
    /// `--lab-*` flag, and `--scene combat_lab` cannot combine with them.
    #[test]
    fn the_lab_entry_is_exclusive_and_the_other_entries_cannot_load_it() {
        let lab = args(&["--scene", "combat_lab"]).unwrap();
        assert!(lab.lab_scene && lab.manual && lab.donut_biscuit && !lab.demo);
        assert_eq!(entry_point(&lab), EntryPoint::CombatLab);
        // The lab opens on the Setup Cage in solo Carl, legacy rules.
        let sel = lab.lab.clone().unwrap();
        assert_eq!(
            (sel.room.as_str(), sel.mode),
            ("R00", combat_lab::PlayerMode::SoloCarl)
        );
        // Auto camera like the demo (LAB-R-Q10), the scroll camera for captures.
        assert_eq!(lab.camera_choice(), CameraChoice::default_for(true));
        assert_eq!(
            args(&["--scene", "combat_lab", "--camera", "scroll"])
                .unwrap()
                .camera_choice(),
            CameraChoice::Scroll
        );
        // Flags of the lab, in any order.
        let a = args(&[
            "--lab-room",
            "R02",
            "--scene",
            "combat_lab",
            "--lab-mode",
            "coop_local",
            "--lab-variant",
            "v1",
            "--deck",
        ])
        .unwrap();
        let sel = a.lab.unwrap();
        assert_eq!(
            (sel.room.as_str(), sel.mode),
            ("R02", combat_lab::PlayerMode::CoopLocal)
        );
        assert!(args(&["--scene", "combat_lab", "--lab-mode", "duo"]).is_err());
        assert!(args(&["--scene", "combat_lab", "--lab-room"]).is_err());
        // Story, demo, plain --cf02 and the boss scene take no lab flag.
        for plain in [
            &[][..],
            &["--demo"][..],
            &["--cf02"][..],
            &["--scene", "boss_f01"][..],
            &["--fixture", "cp01_kick"][..],
        ] {
            for flag in [
                &["--lab-room", "R01"][..],
                &["--lab-mode", "coop_local"][..],
                &["--lab-variant", "v2"][..],
                &["--lab-next", "R02"][..],
                &["--lab-confirmed"][..],
                &["--lab-autopilot"][..],
            ] {
                let mut list = plain.to_vec();
                list.extend_from_slice(flag);
                assert!(args(&list).is_err(), "{list:?} must be rejected");
            }
            assert_ne!(entry_point(&args(plain).unwrap()), EntryPoint::CombatLab);
        }
        // The lab cannot combine with any other run mode.
        for other in [
            &["--demo"][..],
            &["--cf02"][..],
            &["--cf02", "--cf02-seed", "2"][..],
            &["--autopilot"][..],
            &["--demo-beat", "2"][..],
            &["--fixture", "cp01_kick"][..],
            &["--party-fixture", "party_walk_right"][..],
            &["--scenario", "walk_right_400"][..],
            &["--scene", "boss_f01"][..],
            &["--camera-test-room", "2"][..],
            &["--reference-frame"][..],
            &["--headless-floor01-smoke"][..],
        ] {
            let mut forward = vec!["--scene", "combat_lab"];
            forward.extend_from_slice(other);
            let mut backward = other.to_vec();
            backward.extend_from_slice(&["--scene", "combat_lab"]);
            assert!(args(&forward).is_err(), "{forward:?}");
            assert!(args(&backward).is_err(), "{backward:?}");
        }
        assert!(args(&["--scene", "combat_lab", "--scene", "boss_f01"]).is_err());
        assert!(args(&["--scene", "boss_f01", "--scene", "combat_lab"]).is_err());
    }

    /// The leak guard is a mechanism of the launch: whatever entry the parsed
    /// arguments derive, only the lab entry may launch a lab run, and the lab
    /// entry derives only from `--scene combat_lab`.
    #[test]
    fn the_entry_derived_from_the_arguments_gates_the_catalog() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let catalog = LabCatalog::load(root).unwrap();
        let cfg = LabConfig::new("R00", combat_lab::PlayerMode::SoloCarl);
        for (list, entry) in [
            (&[][..], EntryPoint::Story),
            (&["--demo"][..], EntryPoint::Demo),
            (&["--cf02"][..], EntryPoint::Cf02),
            (&["--scene", "boss_f01"][..], EntryPoint::Boss),
            (&["--fixture", "cp01_kick"][..], EntryPoint::Fixture),
        ] {
            let parsed = args(list).unwrap();
            assert_eq!(entry_point(&parsed), entry, "{list:?}");
            assert!(catalog.launch(entry_point(&parsed), &cfg).is_err());
        }
        let lab = args(&["--scene", "combat_lab"]).unwrap();
        assert!(catalog.launch(entry_point(&lab), &cfg).is_ok());
    }

    /// The Q58 seed override is reachable from the lab entry (LAB-R-Q06): on
    /// R01 the seed names the variant, any other seed is refused with the
    /// valid list, and plain `--cf02-seed` still needs `--cf02`.
    #[test]
    fn the_lab_seed_is_cf02_seed_and_on_r01_it_selects_the_variant() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let catalog = LabCatalog::load(root).unwrap();
        let launch = |list: &[&str]| {
            let parsed = args(list).unwrap();
            catalog.launch(entry_point(&parsed), &lab_config(&parsed))
        };
        let r01 = launch(&[
            "--scene",
            "combat_lab",
            "--lab-room",
            "R01",
            "--cf02-seed",
            "3",
        ])
        .unwrap();
        assert_eq!((r01.info.variant.as_str(), r01.info.seed), ("v3", 3));
        let by_flag = launch(&[
            "--scene",
            "combat_lab",
            "--lab-room",
            "R01",
            "--lab-variant",
            "v2",
        ])
        .unwrap();
        assert_eq!(
            (by_flag.info.variant.as_str(), by_flag.info.seed),
            ("v2", 2)
        );
        let e = launch(&[
            "--scene",
            "combat_lab",
            "--lab-room",
            "R01",
            "--cf02-seed",
            "9",
        ])
        .unwrap_err();
        assert!(e.to_string().contains("valid seeds are 1, 2 and 3"), "{e}");
        // A room with one layout records the seed and plays the same room.
        let r02 = launch(&[
            "--scene",
            "combat_lab",
            "--lab-room",
            "R02",
            "--cf02-seed",
            "9",
        ])
        .unwrap();
        assert_eq!(r02.info.seed, 9);
        assert!(args(&["--cf02-seed", "3"]).is_err());
    }

    /// Co-op uses the real pad join path: a live co-op run starts from a fresh
    /// party (no fake pad ids), a scripted capture keeps the runner's party.
    #[test]
    fn live_coop_starts_from_a_fresh_party_and_a_capture_keeps_the_runners() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let catalog = LabCatalog::load(root).unwrap();
        for (list, live) in [
            (
                &["--scene", "combat_lab", "--lab-mode", "coop_local"][..],
                true,
            ),
            (
                &[
                    "--scene",
                    "combat_lab",
                    "--lab-mode",
                    "coop_local",
                    "--capture-tick",
                    "5",
                ][..],
                false,
            ),
        ] {
            let parsed = args(list).unwrap();
            let run = catalog
                .launch(entry_point(&parsed), &lab_config(&parsed))
                .unwrap();
            let party = lab_party(&parsed, &run);
            let two = party.mode() == dungeon_floorbreak::party::PartyMode::TwoPlayer;
            assert_eq!(two, !live, "{list:?}");
        }
    }

    /// Every Setup Cage combination (3 modes x 3 variants x 2 start rooms),
    /// with and without a first-launch `--cf02-seed`, walks the whole room
    /// chain R00 -> start room -> R02 -> end -> Setup Cage. Every launch is
    /// built from the argument list the previous room wrote, parsed by the
    /// real parser and launched through the catalog.
    #[test]
    fn every_menu_combination_launches_the_whole_room_chain() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let catalog = LabCatalog::load(root).unwrap();
        let launch = |argv: &[String]| -> (Args, LabRun) {
            let parsed = parse_args_with(argv, HudSettings::default())
                .unwrap_or_else(|e| panic!("{argv:?}: {e}"));
            let run = catalog
                .launch(entry_point(&parsed), &lab_config(&parsed))
                .unwrap_or_else(|e| panic!("{argv:?}: {e}"));
            (parsed, run)
        };
        let first_launches: [&[&str]; 3] = [
            &[],
            &["--lab-room", "R00", "--cf02-seed", "9"],
            &["--lab-room", "R00", "--lab-variant", "v3"],
        ];
        for first in first_launches {
            for mode in combat_lab::PlayerMode::ALL {
                for variant in lab_flow::MENU_VARIANTS {
                    for next in lab_flow::START_ROOMS {
                        // The Setup Cage as the first launch.
                        let mut argv: Vec<String> = ["floor01_scene", "--scene", "combat_lab"]
                            .iter()
                            .map(|a| (*a).to_owned())
                            .collect();
                        argv.extend(first.iter().map(|a| (*a).to_owned()));
                        let (parsed, run) = launch(&argv);
                        assert_eq!(run.info.room, "R00");
                        // The player picks the combination and confirms.
                        let picked = LabSelection {
                            mode,
                            next: next.into(),
                            next_variant: variant.into(),
                            ..parsed.lab.clone().unwrap()
                        };
                        let mut sel = lab_flow::SetupMenu::new(&picked).selection();
                        let mut visited = Vec::new();
                        loop {
                            argv = lab_flow::relaunch_argv(&argv, &sel);
                            assert!(
                                !argv.iter().any(|a| a == "--cf02-seed"),
                                "the seed override never survives a relaunch: {argv:?}"
                            );
                            let (parsed, run) = launch(&argv);
                            assert_eq!(run.info.mode, mode, "{argv:?}");
                            visited.push((run.info.room.clone(), run.info.variant.clone()));
                            match lab_flow::next_room(&parsed.lab.unwrap()) {
                                NextRoom::Room(next) => sel = next,
                                NextRoom::End => break,
                            }
                        }
                        let want_variant = if next == "R01" { variant } else { "v1" };
                        let mut chain = vec![
                            ("R00".to_owned(), "v1".to_owned()),
                            (next.to_owned(), want_variant.to_owned()),
                        ];
                        if next == "R01" {
                            chain.push(("R02".to_owned(), "v1".to_owned()));
                        }
                        assert_eq!(visited, chain, "{mode:?} {variant} {next} {first:?}");
                        // Back to the Setup Cage: it reopens on the same choice.
                        let (last, _) = launch(&argv);
                        let cage = lab_flow::back_to_cage(&last.lab.unwrap());
                        argv = lab_flow::relaunch_argv(&argv, &cage);
                        let (again, run) = launch(&argv);
                        assert_eq!(run.info.room, "R00");
                        let menu = lab_flow::SetupMenu::new(&again.lab.unwrap());
                        assert_eq!(
                            (menu.mode(), menu.variant(), menu.start_room()),
                            (mode, want_variant, next)
                        );
                    }
                }
            }
        }
    }

    /// The first-launch seed override and an explicit variant must agree, and
    /// the refusal is at the argument parser, before any window.
    #[test]
    fn a_seed_that_conflicts_with_an_explicit_variant_is_refused_when_parsing() {
        let lab = |extra: &[&str]| {
            let mut list = vec!["--scene", "combat_lab", "--lab-room", "R01"];
            list.extend_from_slice(extra);
            args(&list)
        };
        assert!(lab(&["--lab-variant", "v2", "--cf02-seed", "3"]).is_err());
        assert!(lab(&["--lab-variant", "v1", "--cf02-seed", "2"]).is_err());
        assert!(lab(&["--lab-variant", "v3", "--cf02-seed", "3"]).is_ok());
        assert!(lab(&["--cf02-seed", "3"]).is_ok());
        // Other rooms record the seed and play their one layout.
        let r02 = args(&[
            "--scene",
            "combat_lab",
            "--lab-room",
            "R02",
            "--lab-variant",
            "v1",
            "--cf02-seed",
            "9",
        ]);
        assert!(r02.is_ok());
        // R00 takes the variant for the start room it hands over to.
        let cage = args(&[
            "--scene",
            "combat_lab",
            "--lab-room",
            "R00",
            "--lab-variant",
            "v3",
        ])
        .unwrap()
        .lab
        .unwrap();
        assert_eq!(
            (cage.variant.as_str(), cage.next_variant.as_str()),
            ("v1", "v3")
        );
    }

    /// PLAYTRACE-01: a lab session is recorded as `combat_lab` with the real
    /// seed and the chosen mode (a live co-op run starts as a solo party until
    /// the second pad joins), and the badge follows the recorder.
    #[test]
    fn a_lab_session_manifest_names_the_lab_source_seed_and_mode() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let catalog = LabCatalog::load(root).unwrap();
        for (list, mode, seed) in [
            (
                &[
                    "--scene",
                    "combat_lab",
                    "--lab-room",
                    "R01",
                    "--lab-variant",
                    "v3",
                    "--lab-mode",
                    "coop_local",
                ][..],
                "local_coop",
                Some(3),
            ),
            (
                &["--scene", "combat_lab", "--lab-room", "R02"][..],
                "solo_human",
                Some(1),
            ),
        ] {
            let mut parsed = args(list).unwrap();
            let run = catalog
                .launch(entry_point(&parsed), &lab_config(&parsed))
                .unwrap();
            parsed.fixture = Some(run.info.fixture);
            parsed.lab_run = Some(Box::new(run.clone()));
            let party = lab_party(&parsed, &run);
            let manifest = recording_manifest(&parsed, &run.scene, Some(&run.combat), &party, root);
            assert_eq!(manifest.reset.source, "combat_lab", "{list:?}");
            assert_eq!(manifest.mode, mode, "{list:?}");
            assert_eq!(manifest.seed_effective, seed, "{list:?}");
            assert_eq!(
                manifest.feature_flags.get("lab_mode").map(String::as_str),
                Some(run.info.mode.id())
            );
        }
        // A fixture run is still a fixture.
        let fixture = args(&["--fixture", "cp01_kick", "--manual"]).unwrap();
        let run = LabCatalog::load(root)
            .unwrap()
            .launch(
                EntryPoint::CombatLab,
                &LabConfig::new("R00", combat_lab::PlayerMode::SoloCarl),
            )
            .unwrap();
        let manifest =
            recording_manifest(&fixture, &run.scene, Some(&run.combat), &run.party, root);
        assert_eq!(manifest.reset.source, "fixture");
    }

    #[test]
    fn the_rec_badge_names_why_a_lab_run_is_not_recording() {
        let live = args(&["--scene", "combat_lab"]).unwrap();
        let scripted = args(&["--scene", "combat_lab", "--lab-autopilot"]).unwrap();
        let off = args(&["--scene", "combat_lab", "--no-record"]).unwrap();
        assert_eq!(
            lab_rec_status(&off, None),
            RecStatus::Off("--no-record".into())
        );
        assert_eq!(
            lab_rec_status(&scripted, None),
            RecStatus::Off("scripted run".into())
        );
        // A live run without a recorder (and no opt-out) says it failed.
        if record_env_off().is_none() {
            assert!(matches!(lab_rec_status(&live, None), RecStatus::Error(_)));
        }
    }

    /// CA-22: `--companion off|cf02_assist|v0`; `--cf02` implies `cf02_assist`
    /// unless `--companion` is given; the default is the rollback bot.
    #[test]
    fn the_companion_flag_selects_who_drives_the_uncontrolled_hero() {
        let hero = |list: &[&str]| args(list).unwrap().uncontrolled_hero();
        let bot = UncontrolledHero::FollowAndJumpBot;
        let assist = UncontrolledHero::Companion(ProfileSetId::Cf02Assist);
        let v0 = UncontrolledHero::Companion(ProfileSetId::CompanionV0);
        assert_eq!(hero(&[]), bot, "off by default");
        assert_eq!(hero(&["--demo"]), bot);
        assert_eq!(hero(&["--companion", "off"]), bot);
        assert_eq!(hero(&["--companion", "cf02_assist"]), assist);
        assert_eq!(hero(&["--companion", "v0"]), v0);
        assert_eq!(hero(&["--cf02"]), assist);
        assert_eq!(hero(&["--cf02", "--companion", "off"]), bot);
        assert_eq!(hero(&["--cf02", "--companion", "v0"]), v0);
        assert!(args(&["--companion"]).is_err());
        assert!(args(&["--companion", "fast"]).is_err());
        // A fixture declares its own tuning.
        assert!(args(&["--fixture", "cp01_kick", "--companion", "v0"]).is_err());
        assert!(
            args(&[
                "--party-fixture",
                "companion_duo_no_ai",
                "--headless-floor01-smoke",
                "--companion",
                "v0"
            ])
            .is_err()
        );
        // The live party takes the choice into its tuning (and nothing else).
        let party = args(&["--demo", "--companion", "v0"]).unwrap().live_party();
        assert_eq!(party.tuning().uncontrolled_hero, v0);
        let default = args(&[]).unwrap().live_party();
        assert_eq!(*default.tuning(), PROVISIONAL_TUNING);
    }

    /// CF02-REVIEW: the `COMPANION_CONFIG` line said `inactive` for every
    /// `--cf02` run, because `--demo` fills in a fixture; a live human run of the
    /// room must read as active, scripted runs as inactive.
    #[test]
    fn the_companion_config_line_is_active_only_in_a_live_human_run() {
        let inactive = |list: &[&str]| args(list).unwrap().companion_inactive();
        assert!(!inactive(&["--cf02"]), "a live human --cf02 run");
        assert!(!inactive(&["--cf02", "--companion", "v0"]));
        assert!(!inactive(&["--demo", "--companion", "v0"]));
        assert!(inactive(&["--cf02", "--autopilot"]));
        assert!(inactive(&["--cf02", "--exit-after-ticks", "900"]));
        assert!(inactive(&[
            "--cf02",
            "--headless-floor01-smoke",
            "--autopilot"
        ]));
        assert!(inactive(&[
            "--party-fixture",
            "cf02_arena_direct",
            "--headless-floor01-smoke"
        ]));
    }

    /// CARL-PROG-01 CP-A (CP-Q09): the three flags parse, are refused with the lab,
    /// and pick the story by scene; a scripted fixture never reads them.
    #[test]
    fn the_carl_flags_parse_are_refused_with_the_lab_and_pick_the_story_by_scene() {
        let plain = args(&["--cf02"]).unwrap();
        assert!(!plain.carl.any());
        assert!(story_setup(&plain).is_inert());
        let prog = args(&["--carl-prog", "--scene", "boss_f01"]).unwrap();
        assert!(prog.carl.prog && !prog.carl.dynamite && !prog.carl.gauntlet);
        assert_eq!(story_setup(&prog).holds().len(), 1, "the boss pin");
        assert_eq!(
            story_setup(&prog).trigger_ids(),
            ["brute_boss_f01_defeated"]
        );
        // `--carl-prog` alone holds and arms nothing on the demo hall or the arena.
        for extra in [&["--demo"][..], &["--cf02"][..]] {
            let mut list = vec!["--carl-prog"];
            list.extend_from_slice(extra);
            let run = args(&list).unwrap();
            assert!(story_setup(&run).is_inert(), "{list:?}");
        }
        let both = args(&["--cf02", "--carl-dynamite", "--carl-gauntlet"]).unwrap();
        assert_eq!(story_setup(&both).holds().len(), 2);
        // A scripted fixture declares its own pin and ignores every flag.
        let fixture = args(&["--fixture", "cp01_kick", "--carl-dynamite", "--carl-prog"]).unwrap();
        assert!(story_setup(&fixture).is_inert());
        let party = args(&[
            "--headless-floor01-smoke",
            "--party-fixture",
            "carl_prog_juicer_grants_fist",
            "--carl-gauntlet",
        ])
        .unwrap();
        assert_eq!(
            story_setup(&party).holds().len(),
            1,
            "its own pin, not the flag"
        );
        assert_eq!(
            story_setup(&party).holds()[0],
            dungeon_floorbreak::story_unlock::SPECIAL_THROWN_CHARGE
        );
        for flag in ["--carl-prog", "--carl-dynamite", "--carl-gauntlet"] {
            let error = args(&["--scene", "combat_lab", flag]).expect_err(flag);
            assert!(
                error.contains("refused with --scene combat_lab"),
                "{flag}: {error}"
            );
        }
        assert!(args(&["--scene", "combat_lab"]).is_ok());
    }

    #[test]
    fn list_fixtures_is_parsed_without_starting_a_scene() {
        let a = args(&["--list-fixtures", "cf02_"]).unwrap();
        assert_eq!(a.list_fixtures.as_deref(), Some("cf02_"));
        assert!(args(&["--list-fixtures"]).is_err());
        // CF02-C1's llama fixtures and CF02-A's Carl fixtures are `cf02_*` names.
        assert!(list_fixtures("cf02_").contains(&"cf02_llama_pounce"));
        assert!(list_fixtures("cf02_").contains(&"cf02_kick_then_stomp"));
        assert!(list_fixtures("companion_").contains(&"companion_duo_no_ai"));
        assert!(list_fixtures("cf02_assist_").contains(&"cf02_assist_swat_interrupt"));
        assert!(list_fixtures("cp01_").contains(&"cp01_kick"));
        assert!(list_fixtures("carl_prog_").contains(&"carl_prog_juicer_grants_fist"));
    }

    /// CF02-Q01: Down is the D-pad down, or the left stick down to 0.5 with the
    /// vertical axis dominant.
    #[test]
    fn pad_down_needs_dpad_down_or_a_dominant_stick_down() {
        let stick = |x: f32, y: f32| Vec2::new(x, y);
        assert!(pad_down(-1.0, stick(0.0, 0.0)), "D-pad down");
        assert!(!pad_down(0.0, stick(0.0, 0.0)));
        assert!(
            pad_down(0.0, stick(0.0, -0.5)),
            "stick down at the threshold"
        );
        assert!(!pad_down(0.0, stick(0.0, -0.49)), "below the threshold");
        assert!(pad_down(0.0, stick(0.4, -0.6)), "vertical axis dominates");
        assert!(
            !pad_down(0.0, stick(0.9, -0.6)),
            "a diagonal run is not a stomp"
        );
        assert!(!pad_down(0.0, stick(0.0, 1.0)), "up is not down");
    }
}
