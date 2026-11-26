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
        RenderPlugin,
        diagnostic::RenderDiagnosticsPlugin,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::screenshot::{Screenshot, save_to_disk},
    },
    sprite::Anchor,
    window::{PresentMode, PrimaryWindow, WindowResolution},
};
use dungeon_floorbreak::{
    actor_sprite::{
        ActorAssets, ActorSpec, FrameSource, actor_center, invulnerability_alpha, load_carl,
        load_donut, shadow_center_x,
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
    carl_prog::{CarlProgFlags, CarlProgTuning},
    carl_prog_fixtures,
    cf02::{self, Cf02Config, Cf02Tuning},
    cf02_fixtures::{find_party_fixture, list_fixtures},
    cine::{CineTuning, ENV_CINEMATIC_FINISHES, FrameContext, PlannerState, PresentationSettings},
    cine_adapter::CineAdapter,
    combat::{Attacker, ENEMY_HALF_HEIGHT, ENEMY_HALF_WIDTH, Phase, hit_box},
    combat_anim::{
        CombatAnimators, GOBLIN_VARIANTS, SpriteClips, carl_above_goblin,
        cf02::ActingArt,
        dynamite::{ring_points, stick_frame},
        goblin_variant, llama_rush_pose,
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
        DeathPresenter, DrawLayer, FLOOR_DECAL_POOL_CAPACITY, FLOOR_DECAL_POOL_SLOTS, Finish,
        PART_POOL_CAPACITY, PART_POOL_SLOTS, WALL_DECAL_POOL_CAPACITY, WALL_DECAL_POOL_SLOTS,
        blast_vfx_frame, charge_draw_frame, draw_z, explosion_table,
    },
    demo::{self, SelectAction, SelectControl},
    donut_art, donut_fixtures,
    donut_mana::{DonutSetup, StartState},
    donut_prog::{self, DonutProgFlags, DonutProgTuning},
    env_kit::{
        EnvKit, IRect, PieceDraw, animation_frame, parallax_copies, parallax_start, to_world_y,
    },
    f01_act1, f01_boiler, f01_corral, f01_dozer, f01_hazards, f01_hoarder, f01_late, f01_mastery,
    f01_rest, f01_ring, f01_run,
    feel::{LLAMA_GLOB_MOUTH_DX, LLAMA_GLOB_MOUTH_DY, LLAMA_GLOB_SPEED},
    goblin::{AttackPhase, GoblinMode},
    gore::{DEATH_VARIANTS, DeathArt, ENGINEER_VARIANT, GorePreset, LLAMA_VARIANT, Sheet},
    hud::{EventCursor, GlyphSet, HudConfig, HudLayout, HudPreview, HudRuntime, SpecialPhase},
    hud_adapter::{HudPlugin, HudState},
    hud_settings::{HudSettings, parse_switch},
    input::RawInput,
    key_bindings::{self, KeyAction, KeyName},
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
    refuge, room_pilots,
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
// DONUT-MOBILITY-01 DM-C: Donut's reflex cues (presentation only, no new art).
#[path = "floor01_scene/blink_cues.rs"]
mod blink_cues;
#[path = "floor01_scene/donut_cues.rs"]
mod donut_cues;
// F01 danger FX: the acid pool, the blast reach rings, the blast wave, the room 5 gate.
#[path = "floor01_scene/danger_fx_art.rs"]
mod danger_fx_art;
#[path = "floor01_scene/env_prop_art.rs"]
mod env_prop_art;
#[path = "floor01_scene/gym_art.rs"]
mod gym_art;
// AREA-HAZARDS-01B: the corrosion, Rot nest and spent drip decals.
#[path = "floor01_scene/hazard_decals.rs"]
mod hazard_decals;
#[path = "floor01_scene/hazards_art.rs"]
mod hazards_art;
// FA-Q23 (HRD-P): the Hoarder's stand-in presentation (HOARD-01 frames, labelled greybox).
#[path = "floor01_scene/hoarder_art.rs"]
mod hoarder_art;
#[path = "floor01_scene/pierce_spark.rs"]
mod pierce_spark;
// CARL-GAUNTLET-01: Carl's reinforced-fist overlay (presentation only, capability-gated).
#[path = "floor01_scene/carl_gauntlet.rs"]
mod carl_gauntlet;
// ROOM-PORTALS-01: the exit door of rooms 2, 4, 4a, 4b and 6 (the grate down while the exit gate stands).
#[path = "floor01_scene/room_exit_art.rs"]
mod room_exit_art;
// CORRAL-WIRE and CORRAL-E: room 0's M3 gate grate and the rush stand-in (greybox).
#[path = "floor01_scene/corral_art.rs"]
mod corral_art;
// DR-A and DOZ-02: room 3's machine (the DOZ-01 rig and its damage states) and its room pieces.
#[path = "floor01_scene/dozer_art.rs"]
mod dozer_art;
// AREA-FASTFOOD-01: the refuge's staff door, Light on Your Feet box, sparkle and bell (presenter frames).
#[path = "floor01_scene/camp_art.rs"]
mod camp_art;
#[path = "floor01_scene/fastfood_art.rs"]
mod fastfood_art;

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
    /// real render loop, print `FLOOR1_SCENE_STATE` and exit. The run compiles
    /// render pipelines synchronously (an engine teardown crash workaround), so
    /// a pipeline first used mid-run stalls one frame: it is not for frame-time
    /// evidence (use `--perf-frames`, which keeps them asynchronous).
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
    /// Fixture only, evidence lab (LLAMA-DEATH-WIRE): the presenter plays
    /// every block kill as a block-edge cut (`DeathPresenter::with_lab_edge`).
    /// Presentation only: the simulation, its log and its hash are untouched.
    /// Remove it when a simulation cause can behead a llama (precedent:
    /// DEATH-D removed `--death-lab-finish` once the goblin had one).
    death_lab_edge: bool,
    /// Evidence lab (ENG-02, `--death-lab-engineer NAME`): the presenter plays this finish for every Engineer
    /// defeat (`DeathPresenter::with_lab_engineer`; the Floor 1 pilots only ever kill him with a jab). Presentation
    /// only: the simulation, its log and its hash are untouched. Not a recording run.
    death_lab_engineer: Option<Finish>,
    /// HUD-HYBRID-A-01: draw a labelled HUD preview fixture. Presentation
    /// only: it never reaches the simulation, the combat state or a replay.
    hud_preview: Option<HudPreview>,
    /// HUD glyph set override (`--glyphs=xbox|dualsense|keyboard`; `keyboard` forces P1's plate only). `None` follows the
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
    /// DONUT-PROG-01 DP-A (`--donut-prog`, `--donut-light-feet`,
    /// `--donut-puddle`, DP-Q10): the same rules for the second hero's program.
    /// The two start flags imply `--donut-biscuit` (DP-Q06).
    donut: DonutProgFlags,
    /// AUDIENCE-01 QA probe (`--audience-probe`): unlock the per-hero viewer
    /// counts at step 1 (both heroes at the baseline) with one authored test
    /// cue at `audience_cue_x`, and print the `AUDIENCE_STATE` digest line.
    /// Floor 01 itself has no unlock (spec F15); the audience is an observer
    /// that never touches the simulation, so every other line is unchanged.
    audience_probe: bool,
    audience_cue_x: i32,
    /// AUDIENCE-RUN (`--audience[=on|off]`, AR-AD6): the per-hero viewer counts
    /// of a floor run. `None` takes the run default (on in a live `--f01-run`
    /// only, see [`Args::run_broadcast_default`]); `Some(true)` needs
    /// `--f01-run`; `Some(false)` locks the audience everywhere.
    audience: Option<bool>,
    /// AUDIENCE-RUN (`--cine-showcase[=on|off]`, AR-AD7): the CINE-01 showcase
    /// feats (a thrown multi-kill, a llama kill). `None` takes the run default.
    cine_showcase: Option<bool>,
    /// AUDIENCE-RUN: `--f01-audience C:P:D:Q`, the viewer counts the previous
    /// room of the run carried (written by the relaunch).
    f01_audience: Option<f01_run::AudienceCarry>,
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
    /// BOIL-A wire: `--scene f01_boiler`. It implies `cf02_rules` (set when the
    /// arguments are resolved) without the `--cf02` DEMO forcing: the room is the
    /// Goblin Boiler hall, not the microarena.
    f01_boiler: bool,
    /// AREA-REFUGE-01: `--scene refuge_guild`, Mordecai's Tutorial Guild room: a
    /// review pose (`--reference-frame` rules) in its own room, Carl and Donut on
    /// the room's marks.
    refuge: bool,
    /// `--refuge-empty`: the guild room without its cast (the empty-room evidence).
    refuge_empty: bool,
    /// The launched Boiler room (`main` fills it right after parsing).
    boiler_run: Option<Box<f01_boiler::BoilerRun>>,
    /// HRD-A2b: `--scene f01_hoarder` (or `--scene f01_late --f01-room hoarder`, FA-Q24):
    /// the Hoarder's room. Like the Boiler it implies `cf02_rules` and is its own
    /// room, and its boss is a slot enemy loaded from the sidecar roster.
    f01_hoarder: bool,
    /// The launched Hoarder room (`main` fills it right after parsing).
    hoarder_run: Option<Box<f01_hoarder::HoarderRun>>,
    /// HAZ-A2b: `--scene f01_hazards` (or `--scene f01_late --f01-room hazards`):
    /// the slime, acid and Rot Sticker pockets, under the same room rules.
    f01_hazards: bool,
    /// The launched hazard room (`main` fills it right after parsing).
    hazards_run: Option<Box<f01_hazards::HazardsRun>>,
    /// PT-MASTERY: `--scene f01_late --f01-room mastery`, the closing arena (the
    /// CF02-F geometry, the sidecar roster, full kits). Under the room rules.
    f01_mastery: bool,
    /// The launched mastery arena (`main` fills it right after parsing).
    mastery_run: Option<Box<f01_mastery::MasteryRun>>,
    /// RING-A/B wire: `--scene f01_late --f01-room ring_court` (or `ring_split`),
    /// a room of the Broken Ring (KR22: no scene of its own). Under the room rules.
    f01_ring: Option<f01_ring::RingRoom>,
    /// The launched Ring room (`main` fills it right after parsing).
    ring_run: Option<Box<f01_ring::RingRun>>,
    /// REST-WIRE: `--scene f01_late --f01-room rest_refuge`, the fast-food refuge
    /// (no scene of its own, like the Ring rooms). Under the room rules; a playable
    /// greybox (the AREA-FASTFOOD-01 art is a separate work order).
    f01_rest: Option<f01_rest::RestRoom>,
    /// The launched quiet room (`main` fills it right after parsing).
    rest_run: Option<Box<f01_rest::RestRun>>,
    /// ACT1-A: `--scene f01_late --f01-room act1_shore` (or `act1_guild`), Act I's shore and
    /// guild (no scene of its own). Under the room rules; a playable greybox.
    f01_act1: Option<f01_act1::Act1Room>,
    /// The launched Act I room (`main` fills it right after parsing).
    act1_run: Option<Box<f01_act1::Act1Run>>,
    /// ACT1-A evidence: `--act1-pilot walk|donut|strike` plays an Act I room's scripted run
    /// (`act1_fixtures::pilot`) instead of the walk-right script. Scripted runs only.
    act1_pilot: Option<dungeon_floorbreak::act1_fixtures::Pilot>,
    /// AREA-RINGS-01 evidence: `--ring-pilot` plays a Ring room's scripted run with
    /// the library pilot (`f01_ring_fixtures::pilot`: the step, the walkway, the
    /// fight, the exit) instead of the walk-right script, so a capture can stand the
    /// hero in every zone. The Ring rooms have no `--room-pilot`. Scripted runs only.
    ring_pilot: bool,
    /// REST-WIRE evidence: `--rest-pilot donut|carl_door|base_jump|walk|blink|punch` plays a quiet room's scripted
    /// run (`rest_fixtures::pilot`) instead of the walk-right script, so a capture can stand a
    /// hero in every beat. Scripted runs only.
    rest_pilot: Option<dungeon_floorbreak::rest_fixtures::Pilot>,
    /// CORRAL-WIRE: `--scene f01_late --f01-room corral`, room 0 (the Infernal
    /// Corral). The first listed room since CORRAL-F (plan section 14.7; it
    /// launched unlisted under CORRAL-WIRE).
    f01_corral: bool,
    /// The launched corral (`main` fills it right after parsing).
    corral_run: Option<Box<f01_corral::CorralRun>>,
    /// DR-A: `--scene f01_dozer` (or `--scene f01_late --f01-room dozer`): room
    /// 3, the revenge Dozer's chase. Under the room rules; the machine is a slot
    /// enemy loaded from the sidecar roster.
    f01_dozer: bool,
    /// The launched room 3 (`main` fills it right after parsing).
    dozer_run: Option<Box<f01_dozer::DozerRun>>,
    /// PT-SCENE: `--scene f01_late`. It is a launcher: the room (`--f01-room`)
    /// and the loadout (`--f01-loadout`) resolve, once the arguments are read,
    /// to the scene that runs the room (`f01_boiler` or the boss arena) and the
    /// program flags of the loadout.
    f01_late: bool,
    /// The late-Floor-1 room of the run: the `--f01-room` argument, resolved to
    /// the listed room (the Boiler also names itself under `--scene f01_boiler`).
    /// It is the descriptor's `f01_room`.
    f01_room: Option<String>,
    /// The `--f01-loadout` argument, resolved (the descriptor's `f01_loadout`).
    f01_loadout: Option<f01_late::Loadout>,
    /// F01-RUN: `--f01-run`, the in-session floor run (`f01_run`): it starts at
    /// the first listed room, and an exit shows the room-clear panel and
    /// relaunches with the next listed room.
    f01_run: bool,
    /// F01-RUN: `--f01-carry JSON`, the ledger the run carried into this room
    /// (written by the relaunch; the descriptor's `f01_carry`).
    f01_carry: Option<dungeon_floorbreak::story_unlock::LedgerSnapshot>,
    /// F01-RUN: `--f01-run-tally`, the rooms cleared so far (written by the
    /// relaunch; `None` in the run's first room).
    f01_tally: Option<f01_run::RunTally>,
    /// F01-HUD-ROOM: the room's manifest title and whether the HUD shows the
    /// TEST LOADOUT tag (resolved with the room; `--scene f01_late` only).
    f01_hud: Option<(String, bool)>,
    /// EVID-ROOM: `--room-pilot PLAN[:2p]`, the scripted input of a room scene
    /// (a deterministic function of the sim state): the capture path of the
    /// `f01_late` rooms. It needs a scripted run (`--capture-sequence`,
    /// `--capture-tick`, `--headless-floor01-smoke` or `--exit-after-ticks`).
    room_pilot: Option<room_pilots::RoomPilot>,
    /// PT-GATE-C1: an aligned room pilot's own state: the "swap was pressed once" flag (the
    /// library fixtures' `swap_pressed`) and, since PT-GATE-E, the Boiler pilot's step
    /// machine. Shared by the clones of the arguments, so the presim, the capture sequence
    /// and the window loop continue one run; `build_scene` resets it.
    aligned_swap: Arc<std::sync::Mutex<room_pilots::AlignedMemory>>,
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
    /// COMPANION-AI-02 (CA2-Q01): the autonomous partner: `cf02_assist` plus the
    /// goal layer (her own spacing, targets and retreats inside the camera).
    Cf02Free,
}

impl CompanionChoice {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "off" => Ok(Self::Off),
            "cf02_assist" => Ok(Self::Cf02Assist),
            "v0" => Ok(Self::V0),
            "cf02_free" => Ok(Self::Cf02Free),
            other => Err(format!(
                "unknown --companion {other:?} (expected off, cf02_assist, cf02_free or v0)"
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
            && (self.exit_after_ticks.is_none() || self.live_room_exit())
            && self.capture_tick.is_none()
            && self.capture_sequence.is_none()
            && !self.reference_frame
            && self.perf_frames.is_none()
            && self.perf_from_tick.is_none()
            && !self.view_trace
            && self.hud_preview.is_none()
            && self.camera_test_room.is_none()
            && !self.death_stress
            && !self.death_lab_edge
            && self.death_lab_engineer.is_none()
            && !self.audience_probe
            && self.list_fixtures.is_none()
    }

    /// AUDIENCE-RUN (AR-AD6, AR-AD7): the run default of both new behaviours.
    /// On exactly for a live floor run: `live_input()` already excludes every
    /// scripted, headless, capture, perf and probe path, and the two extra
    /// terms switch it off for the scripted room and ring pilots, so every
    /// evidence script that passes `--f01-run` keeps its output.
    fn run_broadcast_default(&self) -> bool {
        self.f01_run
            && self.live_input()
            && self.room_pilot.is_none()
            && !self.ring_pilot
            && self.rest_pilot.is_none()
            && self.act1_pilot.is_none()
    }

    /// AUDIENCE-RUN: whether the per-hero viewer counts are enabled for this
    /// run (an explicit flag always wins over the run default). The audience
    /// still goes live only in the rooms after the corral
    /// ([`f01_run::audience_live_in`]).
    fn audience_enabled(&self) -> bool {
        self.audience
            .unwrap_or_else(|| self.run_broadcast_default())
    }

    /// AUDIENCE-RUN: the CINE-01 tuning of this run. It does not depend on the
    /// room: showcase slow motion is live in the corral too.
    fn cine_tuning(&self) -> CineTuning {
        CineTuning::DEFAULT.with_showcase(
            self.cine_showcase
                .unwrap_or_else(|| self.run_broadcast_default()),
        )
    }

    /// FIX-TOOL: a live room run (an `f01_late` room, the Juicer included) with a scripted
    /// exit and no room pilot: `--exit-after-ticks N` only ends the window after N ticks, the
    /// run keeps its live input source (the keyboard and the pads, and the room's live
    /// partner), so it is still a live-input session and records as one. With a room pilot
    /// the exit belongs to the scripted run, as before.
    fn live_room_exit(&self) -> bool {
        self.exit_after_ticks.is_some()
            && self.manual
            && self.room_pilot.is_none()
            && (self.f01_room_scene() || (self.boss && self.f01_room.as_deref() == Some("juicer")))
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
    /// `--companion`, else `cf02_free` in an `f01_late` room scene (PT-GATE-C1,
    /// plan sections 8 and 9), else `cf02_assist` under `--cf02` (Q13, Q33),
    /// else the follow-and-jump bot, so every other default run is as before.
    fn uncontrolled_hero(&self) -> UncontrolledHero {
        // PT-GATE-C1: an aligned room pilot runs the library fixture's party.
        if let Some(tuning) = self.room_pilot.and_then(|p| p.tuning()) {
            return tuning.uncontrolled_hero;
        }
        // FA-Q08 (FA-Q07 met): room 5 defaults to `cf02_free` like rooms 2, 4 and 6;
        // `--companion off` restores the follow-and-jump bot.
        match self
            .companion
            .unwrap_or(if self.f01_room_scene() || self.juicer_room() {
                CompanionChoice::Cf02Free
            } else if self.cf02.rules {
                CompanionChoice::Cf02Assist
            } else {
                CompanionChoice::Off
            }) {
            CompanionChoice::Off => UncontrolledHero::FollowAndJumpBot,
            CompanionChoice::Cf02Assist => UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
            CompanionChoice::V0 => UncontrolledHero::Companion(ProfileSetId::CompanionV0),
            CompanionChoice::Cf02Free => UncontrolledHero::Companion(ProfileSetId::Cf02Free),
        }
    }

    /// Whether the chosen companion stays idle in this run: a scripted run (the
    /// autopilot, a scenario, a headless or timed fixture) drives the other hero
    /// itself. `--demo` and `--cf02` fill in a fixture, so a live human run of
    /// them (`manual`) is not scripted and the companion runs.
    fn companion_inactive(&self) -> bool {
        // An aligned room pilot plays through the live party: its partner runs.
        if self.room_pilot.is_some_and(|p| p.aligned.is_some()) {
            return false;
        }
        self.autopilot
            || self.scenario.is_some()
            || self.party_fixture.is_some()
            || (self.fixture.is_some() && !self.manual)
    }

    /// Whether the run holds a fixture the user (or another scene such as the
    /// boss arena or the lab) supplied, as opposed to the placeholder fixture a
    /// room scene sets for itself (PT-GATE-C1): only the former refuses
    /// `--companion`. Inferred, not tracked. **Invariant:** every room scene refuses a user
    /// `--fixture` or `--party-fixture` up front (`resolve_f01_late_args` and
    /// `resolve_boiler_args`: "--scene ... replaces ... --fixture, --party-fixture"), before
    /// this method is ever asked, so a fixture inside a room scene is always the scene's own
    /// placeholder. The test `every_room_scene_refuses_a_user_fixture_up_front` asserts the
    /// refusal text; a room scene that ever accepted a user fixture would have to track the
    /// fixture's origin instead of inferring it here.
    fn user_fixture(&self) -> bool {
        // FA-Q08: the Juicer's room (the boss scene under `--f01-room juicer`) sets the
        // boss fixture as its own placeholder, like every other room scene; it too
        // refuses a user `--fixture` or `--party-fixture` up front.
        self.party_fixture.is_some()
            || (self.fixture.is_some() && !self.f01_room_scene() && !self.juicer_room())
    }

    /// FA-Q08: this run is the Juicer's room (`--scene f01_late --f01-room juicer`), room 5.
    fn juicer_room(&self) -> bool {
        self.boss && self.f01_room.as_deref() == Some("juicer")
    }

    /// Whether this run is one of the `f01_late` room scenes with its own room
    /// (the Boiler, the Hoarder or the hazard room): each implies `cf02_rules`
    /// without the microarena.
    fn f01_room_scene(&self) -> bool {
        self.f01_boiler
            || self.f01_hoarder
            || self.f01_hazards
            || self.f01_mastery
            || self.f01_ring.is_some()
            || self.f01_rest.is_some()
            || self.f01_act1.is_some()
            || self.f01_corral
            || self.f01_dozer
    }

    /// CF02-F: this run is in the microarena (`--cf02`, or a CF02-F room fixture).
    fn arena_room(&self) -> bool {
        (self.cf02.rules && !self.f01_room_scene())
            || self
                .fixture
                .is_some_and(|f| f.room == dungeon_floorbreak::combat_scene::FixtureRoom::Arena)
    }

    /// The party of a live run without a party fixture.
    fn live_party(&self) -> PartyState {
        // PT-GATE-C1: the library fixture's own tuning (the 12-tick swap included).
        if let Some(tuning) = self.room_pilot.and_then(|p| p.tuning()) {
            return PartyState::new(tuning);
        }
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
            && (self.exit_after_ticks.is_none() || self.live_room_exit());
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
        self.camera.unwrap_or_else(|| {
            CameraChoice::default_for(self.demo || self.lab_scene || self.f01_room_scene())
        })
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
        .ok_or_else(|| format!("unknown glyph set {value:?} (expected xbox, dualsense, keyboard)"))
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
    if parsed.donut.any() {
        return Err("--donut-prog, --donut-light-feet and --donut-puddle are refused with --scene combat_lab (the lab profile keeps the jump boost and the blink off, DP-Q19)".into());
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

/// The Boiler scene's own argument rules (BOIL-A wire): `--scene f01_boiler`
/// implies `cf02_rules` (the run configuration of every Floor 01 setpiece) and
/// is its own room, so the run modes that pick another room or script are
/// refused. It starts with the biscuit (the post-Hoarder state, K1), the story
/// pin applies under `--carl-prog` and `--donut-prog`, and the scene is recorded
/// by PLAYTRACE since PT-SCENE.
fn resolve_boiler_args(parsed: &mut Args) -> Result<(), String> {
    // HRD-A2b and HAZ-A2b: the Hoarder's and the hazard room's scenes take the
    // Boiler's argument rules, under their own scene name and room id.
    let (scene, room) = if parsed.f01_boiler {
        (f01_boiler::SCENE_NAME, "boiler")
    } else if parsed.f01_hoarder {
        (f01_hoarder::SCENE_NAME, "hoarder")
    } else if parsed.f01_hazards {
        (f01_hazards::SCENE_NAME, "hazards")
    } else if parsed.f01_mastery {
        (f01_mastery::SCENE_NAME, f01_mastery::ROOM_NAME)
    } else if let Some(ring) = parsed.f01_ring {
        (ring.scene_name(), ring.id())
    } else if let Some(rest) = parsed.f01_rest {
        (rest.scene_name(), rest.id())
    } else if let Some(act1) = parsed.f01_act1 {
        (act1.scene_name(), act1.id())
    } else if parsed.f01_corral {
        (f01_corral::SCENE_NAME, f01_corral::ROOM_ID)
    } else if parsed.f01_dozer {
        (f01_dozer::SCENE_NAME, "dozer")
    } else {
        return Ok(());
    };
    if parsed.cf02.rules {
        return Err(format!(
            "--scene {scene} already implies cf02_rules: drop --cf02 (it selects the microarena)"
        ));
    }
    if parsed.cf02_engineer
        || parsed.demo
        || parsed.autopilot
        || parsed.demo_beat != 0
        || parsed.fixture.is_some()
        || parsed.party_fixture.is_some()
        || parsed.scenario.is_some()
        || parsed.camera_test_room.is_some()
        || parsed.reference_frame
    {
        return Err(format!(
            "--scene {scene} replaces --demo, --cf02, --cf02-engineer, --autopilot, --demo-beat, --fixture, --party-fixture, --scenario, --camera-test-room and --reference-frame (it is its own room)"
        ));
    }
    if parsed.f01_hoarder && parsed.cf02.seed.is_some() {
        return Err("--scene f01_hoarder takes no --cf02-seed: the room has no goblin to draw a temperament for".into());
    }
    // PT-SCENE: the room has a replay source (`f01_late::boiler_start`,
    // `hoarder_start`, `hazards_start`), so it records like every live room.
    parsed.f01_room.get_or_insert_with(|| room.to_owned());
    parsed.cf02.rules = true;
    // The launch builds the real fixture; a placeholder keeps the generic
    // argument rules (`driven`) honest until then, like the lab.
    parsed.fixture = Some(&PENDING_FIXTURE);
    // The post-Hoarder state: Donut holds the biscuit with a full pool (the
    // Hoarder's room is the post-corral state, with the biscuit held too). ACT1-A: the Act I
    // rooms are the exception: Donut starts locked (the guild's biscuit is earned in play), so
    // only a flag that implies the biscuit sets it (ACT1-AD13).
    parsed.donut_biscuit = parsed.f01_act1.is_none() || parsed.donut.implies_biscuit();
    // A live run without a script is the player's. FIX-TOOL: `--exit-after-ticks` alone (no
    // room pilot) is a scripted exit, not a script: the run stays the player's.
    parsed.manual = !(parsed.headless
        || parsed.capture_tick.is_some()
        || parsed.capture_sequence.is_some()
        || (parsed.exit_after_ticks.is_some() && parsed.room_pilot.is_some())
        || parsed.freeze
        || parsed.perf_frames.is_some());
    Ok(())
}

/// AREA-REFUGE-01: `--scene refuge_guild`. The room is its own scene (not an
/// `f01_late` room): a lit review pose, the `--reference-frame` rules, with Carl
/// and Donut on the room's marks. Nothing is simulated, recorded or scripted, so
/// every run mode that picks another room or script is refused.
fn resolve_refuge_args(parsed: &mut Args) -> Result<(), String> {
    if !parsed.refuge {
        if parsed.refuge_empty {
            return Err("--refuge-empty needs --scene refuge_guild".into());
        }
        return Ok(());
    }
    if parsed.cf02.rules
        || parsed.cf02_engineer
        || parsed.demo
        || parsed.autopilot
        || parsed.demo_beat != 0
        || parsed.fixture.is_some()
        || parsed.party_fixture.is_some()
        || parsed.scenario.is_some()
        || parsed.camera_test_room.is_some()
        || parsed.reference_frame
        || parsed.camera.is_some()
        || parsed.capture_tick.is_some()
        || parsed.capture_sequence.is_some()
        || parsed.exit_after_ticks.is_some()
        || parsed.perf_frames.is_some()
        || parsed.f01_room.is_some()
        || parsed.f01_loadout.is_some()
    {
        return Err("--scene refuge_guild is a fixed review pose: it replaces --demo, --cf02, --autopilot, --fixture, --party-fixture, --scenario, --camera, --camera-test-room, --reference-frame, --capture-*, --exit-after-ticks and --perf-frames, and takes no --f01-room".into());
    }
    parsed.reference_frame = true;
    Ok(())
}

/// PT-SCENE: `--scene f01_late [--f01-room ID] [--f01-loadout story|all]`.
/// The scene is a launcher: it resolves the room against the manifest (an
/// unknown or unlisted room is refused with the listed rooms), applies the
/// loadout's program flags and selects the scene that runs the room, so the rest
/// of the argument rules and the run are those of that scene. The run
/// configuration, the story pin and the loadout come from the scene and the
/// loadout, never from the flags that would set them by hand.
fn resolve_f01_late_args(parsed: &mut Args) -> Result<(), String> {
    if parsed.f01_audience.is_some() && parsed.audience == Some(false) {
        return Err("--f01-audience carries viewer counts into a room, so it needs the audience on: drop --audience=off (or drop --f01-audience)".into());
    }
    if !parsed.f01_late {
        if parsed.f01_room.is_some() || parsed.f01_loadout.is_some() {
            return Err("--f01-room and --f01-loadout need --scene f01_late".into());
        }
        if parsed.f01_run || parsed.f01_carry.is_some() || parsed.f01_tally.is_some() {
            return Err("--f01-run, --f01-carry and --f01-run-tally need --scene f01_late".into());
        }
        if parsed.f01_audience.is_some() || parsed.audience == Some(true) {
            return Err("--f01-audience and --audience=on need --scene f01_late --f01-run".into());
        }
        return Ok(());
    }
    if !parsed.f01_run && (parsed.f01_carry.is_some() || parsed.f01_tally.is_some()) {
        return Err("--f01-carry and --f01-run-tally belong to a floor run: add --f01-run".into());
    }
    if !parsed.f01_run && (parsed.f01_audience.is_some() || parsed.audience == Some(true)) {
        return Err("--f01-audience and --audience=on belong to a floor run: add --f01-run".into());
    }
    if parsed.f01_run && parsed.cf02.seed.is_some() {
        // Adopted F01RUN-AD3: the rooms of a run draw their default seeds (the
        // Hoarder and the Juicer take none); a seeded room is launched on its own.
        return Err(
            "--f01-run takes no --cf02-seed: launch a seeded room on its own with --f01-room"
                .into(),
        );
    }
    if parsed.cf02.rules {
        return Err(
            "--scene f01_late sets its own run rules per room: drop --cf02 (it selects the microarena)"
                .into(),
        );
    }
    if parsed.cf02_engineer
        || parsed.demo
        || parsed.autopilot
        || parsed.demo_beat != 0
        || parsed.fixture.is_some()
        || parsed.party_fixture.is_some()
        || parsed.scenario.is_some()
        || parsed.camera_test_room.is_some()
        || parsed.reference_frame
    {
        return Err("--scene f01_late replaces --demo, --cf02, --cf02-engineer, --autopilot, --demo-beat, --fixture, --party-fixture, --scenario, --camera-test-room and --reference-frame (a room is its own scene)".into());
    }
    if parsed.carl.dynamite
        || parsed.carl.gauntlet
        || parsed.donut.light_feet
        || parsed.donut.puddle
    {
        return Err("--scene f01_late refuses --carl-dynamite, --carl-gauntlet, --donut-light-feet and --donut-puddle: the loadout sets them (--f01-loadout all)".into());
    }
    let loadout = parsed.f01_loadout.unwrap_or(f01_late::Loadout::Story);
    if loadout == f01_late::Loadout::All && (parsed.carl.prog || parsed.donut.prog) {
        return Err("--f01-loadout all is a TEST LOADOUT with the story triggers off: drop --carl-prog and --donut-prog".into());
    }
    let root = std::env::current_dir().map_err(|e| format!("no working directory: {e}"))?;
    let manifest = f01_late::Manifest::load(&root)?;
    let row = match parsed.f01_room.as_deref() {
        // CORRAL-WIRE: an unlisted launchable room launches on its own; a floor
        // run walks the listed rooms only (CW-AD1).
        Some(id) if parsed.f01_run && manifest.is_unlisted(id) => {
            return Err(format!(
                "--f01-run walks the listed rooms only: {id:?} is launchable but unlisted (launch it without --f01-run)"
            ));
        }
        Some(id) => manifest
            .launchable(id)
            .map_err(|e| format!("--f01-room: {e}"))?,
        // F01-RUN: a floor run starts at the first listed room; a bare launch at
        // the Boiler (HRDA2B-AD1, unchanged).
        None if parsed.f01_run => f01_run::first_room(&manifest),
        None => manifest.default_room(),
    };
    if parsed.f01_audience.is_some() && !f01_run::audience_live_in(&manifest, &row.id) {
        return Err(format!(
            "--f01-audience: the broadcast is not live in {:?} (it goes live in the rooms after {:?})",
            row.id,
            f01_run::AUDIENCE_UNLOCK_AFTER
        ));
    }
    if loadout == f01_late::Loadout::All && parsed.f01_carry.is_some() {
        return Err("--f01-carry is the story loadout's ledger: the TEST LOADOUT (--f01-loadout all) carries nothing".into());
    }
    // F01-HUD-ROOM: the title and the TEST LOADOUT tag (PT-SCENE U5: the `all`
    // loadout only).
    parsed.f01_hud = Some((row.title.clone(), f01_run::test_loadout_tag(Some(loadout))));
    parsed.f01_room = Some(row.id.clone());
    parsed.f01_loadout = Some(loadout);
    let (carl, donut) = loadout.flags();
    parsed.carl.prog |= carl.prog;
    parsed.carl.dynamite |= carl.dynamite;
    parsed.carl.gauntlet |= carl.gauntlet;
    parsed.donut.prog |= donut.prog;
    parsed.donut.puddle |= donut.puddle;
    parsed.donut_biscuit |= parsed.donut.implies_biscuit();
    match row.scene {
        f01_late::RoomScene::Boiler => parsed.f01_boiler = true,
        f01_late::RoomScene::Hoarder => parsed.f01_hoarder = true,
        f01_late::RoomScene::Hazards => parsed.f01_hazards = true,
        f01_late::RoomScene::Mastery => parsed.f01_mastery = true,
        f01_late::RoomScene::RingCourt | f01_late::RoomScene::RingSplit => {
            parsed.f01_ring = row.scene.ring();
        }
        f01_late::RoomScene::RestRefuge
        | f01_late::RoomScene::RestCamp
        | f01_late::RoomScene::RestAftermath => parsed.f01_rest = row.scene.rest(),
        f01_late::RoomScene::Act1Chase
        | f01_late::RoomScene::Act1Shore
        | f01_late::RoomScene::Act1Guild => {
            parsed.f01_act1 = row.scene.act1();
        }
        f01_late::RoomScene::Corral => parsed.f01_corral = true,
        f01_late::RoomScene::Dozer => parsed.f01_dozer = true,
        f01_late::RoomScene::Boss => {
            parsed.boss = true;
            parsed.fixture = Some(boss_scene::BOSS01_SCENE);
        }
    }
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
    let overlay = match (&args.lab, &args.lab_run) {
        (Some(selection), Some(_)) => {
            let menu_open = selection.room == "R00" && !selection.confirmed;
            LabOverlay::new(view_height(args.deck), selection.clone(), menu_open)
        }
        // F01-RUN: the floor run draws its panels with the lab's overlay.
        _ if args.f01_run => LabOverlay::floor_run(view_height(args.deck)),
        _ => return Ok(None),
    };
    let path = root.join(dungeon_floorbreak::combat_lab_overlay::FONT_PATH);
    let font = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Some(LabOverlayPlugin { font, overlay }))
}

/// Mark every descriptor above stderr close-on-exec before the relaunch
/// `exec`, so the kernel closes them atomically when the new image starts. The
/// GPU driver opens device and dmabuf descriptors without close-on-exec, so
/// each relaunch would otherwise leave its predecessor's four behind
/// (measured: 55 open descriptors at the first launch, 103 at the thirteenth).
/// Nothing is closed here: other threads keep working until the `exec`.
/// `close_range(.., CLOSE_RANGE_CLOEXEC)` does it in one call (Linux 5.11);
/// an older kernel falls back to `fcntl(F_SETFD)` per listed descriptor.
///
/// The syscall is made directly, not through `libc::close_range`: that wrapper
/// links the `close_range@GLIBC_2.34` symbol, and the whole binary would then
/// fail to load on an older glibc (the Steam Linux Runtime "sniper" container
/// ships 2.31) before this fallback could ever run. The binary is only that
/// portable when built in the Steam Runtime SDK: a host build still links newer
/// glibc symbols from the toolchain (note it in the release notes).
#[cfg(target_os = "linux")]
fn close_inherited_fds() {
    // SAFETY: `close_range` only changes descriptor flags and touches no
    // memory of this process; an unknown syscall just returns -1 (ENOSYS).
    let done = unsafe {
        libc::syscall(
            libc::SYS_close_range,
            3u32,
            u32::MAX,
            libc::CLOSE_RANGE_CLOEXEC,
        )
    } == 0;
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

#[cfg(not(target_os = "linux"))]
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
    exec_next(&argv, &next, "lab room");
}

/// Replace this process with `next` (program name first): the relaunch of the
/// lab and of the floor run.
fn exec_next(argv: &[String], next: &[String], what: &str) -> ! {
    let program = std::env::current_exe().unwrap_or_else(|_| PathBuf::from(&argv[0]));
    close_inherited_fds();
    let mut command = std::process::Command::new(program);
    command.args(&next[1..]);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = command.exec();
        eprintln!("floor01_scene: cannot start the next {what}: {error}");
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    {
        match command.spawn() {
            Ok(_) => std::process::exit(0),
            Err(error) => {
                eprintln!("floor01_scene: cannot start the next {what}: {error}");
                std::process::exit(1);
            }
        }
    }
}

/// F01-RUN: what the floor run decided when the room's exit was reached.
#[derive(Default)]
enum RunNext {
    /// The room is still being played.
    #[default]
    Playing,
    /// The panel is up; its confirmation does this.
    Decided(f01_run::Decision),
    /// The exit could not be resolved (reported once); the room stays.
    Failed,
}

/// F01-RUN: frames the room-clear and floor-end panels ignore a confirmation
/// for after they open, so a jump press carried through the exit (either pad's
/// South is also Confirm) does not skip the facts.
const RUN_PANEL_ARM_FRAMES: u16 = 30;

/// F01-RUN: one fixed tick of the floor run, after `fixed_step`. When the
/// room's exit is reached (the room's own rule: `world.complete`), the pure
/// decision of `f01_run::on_exit` goes on the panel; the panel's confirmation
/// relaunches, or quits after the last listed room. Reads the finished
/// simulation; nothing flows back. A scripted run shows the panel
/// (presentation only) and never leaves the room.
#[allow(clippy::too_many_arguments)]
fn f01_run_step(
    sim: Res<Sim>,
    combat: Res<Combat>,
    setup: Res<Setup>,
    hud: Res<HudState>,
    mut overlay: ResMut<LabOverlay>,
    mut rec: ResMut<Recorder>,
    mut next: Local<RunNext>,
    mut exit: MessageWriter<AppExit>,
) {
    let (Some(combat), Some(room)) = (combat.0.as_ref(), setup.args.f01_room.as_deref()) else {
        return;
    };
    if sim.0.world.complete && matches!(*next, RunNext::Playing) {
        let decided = std::env::current_dir()
            .map_err(|e| e.to_string())
            .and_then(|root| f01_late::Manifest::load(&root))
            .and_then(|manifest| {
                let argv: Vec<String> = std::env::args().collect();
                let recording = lab_rec_status(&setup.args, rec.0.as_ref()).badge().0;
                f01_run::on_exit(&f01_run::ExitInput {
                    manifest: &manifest,
                    room,
                    loadout: setup.args.f01_loadout.unwrap_or(f01_late::Loadout::Story),
                    combat,
                    carried: setup.args.f01_carry.as_ref(),
                    tally: setup.args.f01_tally.unwrap_or_default(),
                    argv: &argv,
                    recording: &recording,
                    audience: hud
                        .0
                        .audience_carry()
                        .map(f01_run::AudienceCarry::from_counts),
                    audience_enabled: setup.args.audience_enabled(),
                })
            });
        match decided {
            Ok((facts, panel, decision)) => {
                let tally = setup.args.f01_tally.unwrap_or_default().with(&facts);
                println!(
                    "F01_RUN_ROOM_CLEARED room={} ticks={} deaths={} restarts={} grants={} run_rooms={} run_ticks={}",
                    facts.id,
                    facts.ticks,
                    facts.deaths,
                    facts.restarts,
                    facts.grants.len(),
                    tally.rooms,
                    tally.ticks
                );
                overlay.end_heading = panel.heading;
                overlay.end_hint = panel.hint;
                overlay.end_lock_frames = RUN_PANEL_ARM_FRAMES;
                overlay.end = Some(EndPanel { lines: panel.lines });
                *next = RunNext::Decided(decision);
            }
            Err(error) => {
                eprintln!("F01_RUN cannot resolve the exit of {room}: {error}");
                *next = RunNext::Failed;
            }
        }
    }
    if overlay.request.take() == Some(LabRequest::Continue) && setup.args.manual {
        match std::mem::take(&mut *next) {
            RunNext::Decided(f01_run::Decision::Relaunch { room, argv }) => {
                // `exec` bypasses the exit path: close and flush the session first, so
                // it is saved as completed and not recovered as a crash.
                if let Some(recorder) = rec.0.as_mut() {
                    recorder.finish("f01_run_relaunch");
                }
                rec.flush();
                println!("F01_RUN_RELAUNCH room={room}");
                let current: Vec<String> = std::env::args().collect();
                exec_next(&current, &argv, "floor run room");
            }
            RunNext::Decided(f01_run::Decision::Quit) => {
                println!("F01_RUN_FLOOR_END");
                exit.write(AppExit::Success);
            }
            other => *next = other,
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
        death_lab_edge: false,
        death_lab_engineer: None,
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
        donut: DonutProgFlags::OFF,
        audience_probe: false,
        audience_cue_x: AUDIENCE_PROBE_CUE_X,
        audience: None,
        cine_showcase: None,
        f01_audience: None,
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
        f01_boiler: false,
        refuge: false,
        refuge_empty: false,
        boiler_run: None,
        room_pilot: None,
        aligned_swap: Arc::default(),
        f01_hoarder: false,
        hoarder_run: None,
        f01_hazards: false,
        hazards_run: None,
        f01_mastery: false,
        mastery_run: None,
        f01_ring: None,
        ring_run: None,
        f01_rest: None,
        rest_run: None,
        ring_pilot: false,
        rest_pilot: None,
        f01_act1: None,
        act1_run: None,
        act1_pilot: None,
        f01_corral: false,
        corral_run: None,
        f01_dozer: false,
        dozer_run: None,
        f01_late: false,
        f01_room: None,
        f01_loadout: None,
        f01_run: false,
        f01_carry: None,
        f01_tally: None,
        f01_hud: None,
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
            "--donut-prog" => parsed.donut.prog = true,
            "--donut-light-feet" => {
                parsed.donut.light_feet = true;
                parsed.donut_biscuit |= parsed.donut.implies_biscuit();
            }
            "--donut-puddle" => {
                parsed.donut.puddle = true;
                parsed.donut_biscuit |= parsed.donut.implies_biscuit();
            }
            "--audience-probe" => parsed.audience_probe = true,
            "--audience" => parsed.audience = Some(true),
            other if other.starts_with("--audience=") => {
                let value = &other["--audience=".len()..];
                parsed.audience = Some(parse_switch(value).ok_or_else(|| {
                    format!("bad value {value:?} for --audience (expected on or off)")
                })?);
            }
            "--cine-showcase" => parsed.cine_showcase = Some(true),
            other if other.starts_with("--cine-showcase=") => {
                let value = &other["--cine-showcase=".len()..];
                parsed.cine_showcase = Some(parse_switch(value).ok_or_else(|| {
                    format!("bad value {value:?} for --cine-showcase (expected on or off)")
                })?);
            }
            f01_run::FLAG_AUDIENCE => {
                let value = iter
                    .next()
                    .ok_or("--f01-audience needs carl:carl_peak:donut:donut_peak (written by the floor run)")?;
                parsed.f01_audience = Some(f01_run::AudienceCarry::parse(value)?);
            }
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
                let value = iter
                    .next()
                    .ok_or("--glyphs needs xbox, dualsense or keyboard")?;
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
                    .ok_or("--scene needs a name (boss_f01, combat_lab, f01_boiler, f01_hoarder, f01_dozer, f01_hazards, f01_late, refuge_guild)")?;
                if parsed.boss
                    || parsed.lab_scene
                    || parsed.f01_room_scene()
                    || parsed.f01_late
                    || parsed.refuge
                {
                    return Err(
                        "--scene takes one scene: boss_f01, combat_lab, f01_boiler, f01_hoarder, f01_dozer, f01_hazards, f01_late and refuge_guild are exclusive"
                            .into(),
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
                    f01_boiler::SCENE_NAME => parsed.f01_boiler = true,
                    f01_hoarder::SCENE_NAME => parsed.f01_hoarder = true,
                    f01_dozer::SCENE_NAME => parsed.f01_dozer = true,
                    f01_hazards::SCENE_NAME => parsed.f01_hazards = true,
                    f01_late::SCENE_NAME => parsed.f01_late = true,
                    refuge::SCENE_NAME => parsed.refuge = true,
                    _ => {
                        return Err(format!(
                            "unknown scene {value:?} (expected boss_f01, combat_lab, f01_boiler, f01_hoarder, f01_dozer, f01_hazards, f01_late, refuge_guild)"
                        ));
                    }
                }
            }
            other if lab_flag(&mut parsed.lab, other, &mut iter)? => {
                parsed.lab_variant_set |= other == "--lab-variant";
            }
            "--refuge-empty" => parsed.refuge_empty = true,
            "--f01-room" => {
                let value = iter.next().ok_or("--f01-room needs a room id")?;
                parsed.f01_room = Some(value.clone());
            }
            "--ring-pilot" => parsed.ring_pilot = true,
            "--rest-pilot" => {
                let value = iter.next().ok_or(
                    "--rest-pilot needs donut, carl_door, base_jump, walk, blink or punch",
                )?;
                parsed.rest_pilot = Some(dungeon_floorbreak::rest_fixtures::Pilot::parse(value)?);
            }
            "--act1-pilot" => {
                let value = iter
                    .next()
                    .ok_or("--act1-pilot needs walk, donut or strike")?;
                parsed.act1_pilot = Some(dungeon_floorbreak::act1_fixtures::Pilot::parse(value)?);
            }
            "--room-pilot" => {
                let value = iter
                    .next()
                    .ok_or("--room-pilot needs direct, valve or chain")?;
                parsed.room_pilot = Some(room_pilots::RoomPilot::parse(value)?);
            }
            f01_run::FLAG_RUN => parsed.f01_run = true,
            f01_run::FLAG_CARRY => {
                let value = iter
                    .next()
                    .ok_or("--f01-carry needs a ledger snapshot (written by the floor run)")?;
                parsed.f01_carry = Some(f01_run::decode_carry(value)?);
            }
            f01_run::FLAG_TALLY => {
                let value = iter
                    .next()
                    .ok_or("--f01-run-tally needs rooms:ticks:deaths:restarts")?;
                parsed.f01_tally = Some(f01_run::RunTally::parse(value)?);
            }
            "--f01-loadout" => {
                let value = iter.next().ok_or("--f01-loadout needs story or all")?;
                parsed.f01_loadout =
                    Some(f01_late::Loadout::parse(value).ok_or_else(|| {
                        format!("unknown loadout {value:?} (expected story or all)")
                    })?);
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
            "--death-lab-edge" => parsed.death_lab_edge = true,
            "--death-lab-engineer" => {
                let value = iter.next().ok_or("--death-lab-engineer needs a finish")?;
                parsed.death_lab_engineer = Some(Finish::lab_engineer(value).ok_or_else(|| {
                    format!(
                        "bad --death-lab-engineer {value:?} (expected crushed, decapitated, exploded, slammed_stomped, slammed_wall or slammed_flung)"
                    )
                })?);
            }
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
    resolve_f01_late_args(&mut parsed)?;
    resolve_boiler_args(&mut parsed)?;
    resolve_refuge_args(&mut parsed)?;
    if let Some(kind) = parsed.rest_pilot {
        if parsed.f01_rest.is_none() {
            return Err(
                "--rest-pilot needs a quiet room (--scene f01_late --f01-room rest_refuge, rest_camp or rest_aftermath)"
                    .into(),
            );
        }
        if let Some(room) = parsed.f01_rest
            && !kind.fits(room)
        {
            return Err(format!(
                "--rest-pilot {kind:?} does not fit the room {}: donut, carl_door and base_jump play in rest_refuge, blink and punch in rest_aftermath, walk in any quiet room",
                room.id()
            ));
        }
        if parsed.room_pilot.is_some() || parsed.ring_pilot {
            return Err("--rest-pilot replaces --room-pilot and --ring-pilot".into());
        }
        // A floor run relaunches the binary per room and keeps every other flag, so a pilot
        // would play on in the next room's launch: the pilot is for one scripted room only.
        if parsed.f01_run {
            return Err(
                "--rest-pilot is a one-room evidence pilot: drop --f01-run (a floor run relaunches into the next room)"
                    .into(),
            );
        }
    }
    if let Some(kind) = parsed.act1_pilot {
        let Some(room) = parsed.f01_act1 else {
            return Err(
                "--act1-pilot needs an Act I room (--scene f01_late --f01-room act1_chase, act1_shore or act1_guild)"
                    .into(),
            );
        };
        if !kind.fits(room) {
            return Err(format!(
                "--act1-pilot {kind:?} does not fit the room {}: strike plays in act1_shore, donut in act1_guild, hide, skip and watch in act1_chase, walk in any",
                room.id()
            ));
        }
        if parsed.room_pilot.is_some() || parsed.ring_pilot || parsed.rest_pilot.is_some() {
            return Err("--act1-pilot replaces --room-pilot, --ring-pilot and --rest-pilot".into());
        }
        // A floor run relaunches the binary per room and keeps every other flag, so a pilot
        // would play on in the next room's launch: the pilot is for one scripted room only.
        if parsed.f01_run {
            return Err(
                "--act1-pilot is a one-room evidence pilot: drop --f01-run (a floor run relaunches into the next room)"
                    .into(),
            );
        }
    }
    if parsed.ring_pilot {
        if parsed.f01_ring.is_none() {
            return Err("--ring-pilot needs a Ring room (--scene f01_late --f01-room ring_court or ring_split)".into());
        }
        if parsed.room_pilot.is_some() {
            return Err(
                "--ring-pilot replaces --room-pilot (the Ring rooms take only --ring-pilot)".into(),
            );
        }
    }
    if let Some(pilot) = parsed.room_pilot {
        let juicer = parsed.boss && parsed.f01_room.as_deref() == Some("juicer");
        if !(parsed.f01_boiler
            || parsed.f01_hoarder
            || parsed.f01_hazards
            || parsed.f01_mastery
            || parsed.f01_corral
            || parsed.f01_dozer
            || juicer)
        {
            return Err("--room-pilot needs a room scene: --scene f01_late with --f01-room corral, boiler, hoarder, dozer, hazards, mastery or juicer (or --scene f01_boiler, f01_hoarder, f01_dozer, f01_hazards)".into());
        }
        // CORRAL-F: the corral's pilots play room 0, and room 0 takes only its own pilots.
        let corral_plan = matches!(
            pilot.plan,
            room_pilots::Plan::Corral
                | room_pilots::Plan::CorralRush
                | room_pilots::Plan::CorralPartition
        );
        if corral_plan != parsed.f01_corral {
            return Err("--room-pilot corral, corral_rush and corral_partition need the corral (--f01-room corral), and the corral takes only its own plans".into());
        }
        // DR-A: the `dozer:*` plans play room 3, and room 3 takes only them.
        if pilot.plan.is_dozer() != parsed.f01_dozer {
            return Err("--room-pilot dozer:run, dozer:close_call, dozer:wait, dozer:brace, dozer:retry and dozer:crew need the Dozer's room (--f01-room dozer or --scene f01_dozer), and that room takes only them".into());
        }
        // PT-GATE-E: the hazards library pilot plays the hazards room, the Boiler's the Boiler.
        if (pilot.plan == room_pilots::Plan::Hazards && !parsed.f01_hazards)
            || (pilot.plan == room_pilots::Plan::Boiler && !parsed.f01_boiler)
        {
            return Err(format!(
                "--room-pilot {} needs its own room (hazards: --f01-room hazards, boiler: --f01-room boiler)",
                pilot.plan.name()
            ));
        }
        // FA-Q24: the Hoarder's pilot plays room 1 and nothing else.
        if pilot.plan == room_pilots::Plan::Hoarder && !parsed.f01_hoarder {
            return Err(
                "--room-pilot hoarder needs the Hoarder's room (--f01-room hoarder or --scene f01_hoarder)"
                    .into(),
            );
        }
        // PT-GATE-A: the mastery and juicer plans play their own room and nothing else.
        if (pilot.plan == room_pilots::Plan::Mastery) != parsed.f01_mastery
            || (pilot.plan == room_pilots::Plan::Juicer) != juicer
        {
            return Err("--room-pilot mastery needs --f01-room mastery and --room-pilot juicer needs --f01-room juicer, and each of those rooms takes only its own plan".into());
        }
        // PT-GATE-D: the room 4 route plays the hazards room and nothing else.
        if matches!(
            pilot.plan,
            room_pilots::Plan::HazSpectacular | room_pilots::Plan::HazTactical
        ) && !parsed.f01_hazards
        {
            return Err(format!(
                "--room-pilot {} needs the hazards room (--f01-room hazards)",
                pilot.plan.name()
            ));
        }
        if !(parsed.headless
            || parsed.capture_tick.is_some()
            || parsed.capture_sequence.is_some()
            || parsed.exit_after_ticks.is_some()
            || parsed.perf_from_tick.is_some())
        {
            return Err("--room-pilot needs a scripted run: --capture-sequence, --capture-tick, --headless-floor01-smoke, --exit-after-ticks or --perf-from-tick".into());
        }
        // The mastery and juicer plans are solo: slot 1 would play the CF02 Donut pilot, which is
        // neither the PT-MASTERY nor the PT-SCENE-B pilot, and the run would pass for theirs.
        if pilot.two_player
            && matches!(
                pilot.plan,
                room_pilots::Plan::Mastery
                    | room_pilots::Plan::Juicer
                    | room_pilots::Plan::HazSpectacular
                    | room_pilots::Plan::HazTactical
            )
        {
            return Err(
                "--room-pilot mastery, juicer, haz_spectacular and haz_tactical are solo plans (no :2p)".into(),
            );
        }
        // PT-GATE-C1: an aligned pilot is the library fixture's run: its seed is the
        // room's seed (the mastery room draws temperaments from it), and its partner
        // is the fixture's, so `--companion` and a different `--cf02-seed` are refused.
        if let Some(aligned) = pilot.aligned {
            if parsed.companion.is_some() {
                return Err(format!(
                    "--room-pilot {} fixes its partner (the library fixture's): drop --companion",
                    pilot.label()
                ));
            }
            // The mastery, hazards and Boiler rooms draw their goblins' temperaments from the
            // seed (`--cf02-seed`); the Juicer's seed is its swing offset.
            if matches!(
                pilot.plan,
                room_pilots::Plan::Mastery
                    | room_pilots::Plan::Hazards
                    | room_pilots::Plan::Boiler
                    | room_pilots::Plan::Corral
                    | room_pilots::Plan::CorralRush
                    | room_pilots::Plan::CorralPartition
            ) {
                match parsed.cf02.seed {
                    Some(seed) if seed != aligned.seed => {
                        return Err(format!(
                            "--room-pilot {} plays seed {}, not --cf02-seed {seed}",
                            pilot.label(),
                            aligned.seed
                        ));
                    }
                    _ => parsed.cf02.seed = Some(aligned.seed),
                }
            }
        }
        // The perf probe steps one slot-0 pilot tick per frame outside the live
        // party, so a two-player pilot (Donut's slot) is not measured.
        if parsed.perf_from_tick.is_some() && pilot.two_player {
            return Err("--perf-from-tick takes a solo --room-pilot (no :2p)".into());
        }
    }
    if parsed.cf02.seed.is_some() && !parsed.cf02.rules && !parsed.lab_scene {
        return Err("--cf02-seed needs --cf02 (or --scene combat_lab)".into());
    }
    if parsed.cf02.rules && !parsed.f01_room_scene() {
        // CF02-F: `--cf02` is the microarena. It runs on the DEMO machinery (the
        // checkpoint respawn, the Select tap and hold, the autopilot flag), with
        // the arena's own room, fixture and rules. `--scene f01_boiler` implies
        // `cf02_rules` but is its own room (BOIL-A wire): no DEMO forcing.
        parsed.demo = true;
    }
    if parsed.companion.is_some() && parsed.user_fixture() {
        // A fixture the user supplied declares its own tuning (and so its own
        // companion mode); the flag is for the live scene only (CA-22). The
        // placeholder a room scene sets is not the user's fixture (PT-GATE-C1).
        return Err("--companion cannot be combined with --fixture or --party-fixture".into());
    }
    parsed.boss |= parsed.fixture.is_some_and(|f| f.boss.is_some());
    if parsed.boss && (parsed.demo || parsed.scenario.is_some() || parsed.reference_frame) {
        return Err("the boss arena replaces --demo, --scenario and --reference-frame".into());
    }
    if parsed.boss && parsed.fixture == Some(boss_scene::BOSS01_SCENE) {
        // A live run without a script is the player's. FIX-TOOL: `--exit-after-ticks` alone
        // on the Juicer's room scene (no room pilot) is a scripted exit, not a script.
        let room_exit = parsed.f01_room.as_deref() == Some("juicer") && parsed.room_pilot.is_none();
        parsed.manual = !(parsed.headless
            || parsed.capture_tick.is_some()
            || parsed.capture_sequence.is_some()
            || (parsed.exit_after_ticks.is_some() && !room_exit)
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
    if parsed.death_lab_edge && parsed.fixture.is_none() {
        return Err("--death-lab-edge needs --fixture".into());
    }
    if parsed.death_trace && !parsed.headless {
        return Err("--death-trace needs --headless-floor01-smoke".into());
    }
    if parsed.manual
        && (parsed.headless
            || parsed.capture_tick.is_some()
            || parsed.capture_sequence.is_some()
            || (parsed.exit_after_ticks.is_some() && !parsed.live_room_exit())
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
    let death = DeathPresenter::with_variants(art, &variant_b, args.gore_preset)?
        .with_llama(Arc::new(DeathArt::load_llama(root)?))?
        .with_engineer(Arc::new(DeathArt::load_engineer(root)?))?;
    let death = if args.death_lab_edge {
        death.with_lab_edge()
    } else {
        death
    };
    Ok(match args.death_lab_engineer {
        Some(finish) => death.with_lab_engineer(finish),
        None => death,
    })
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
    // F01-HUD-ROOM: a late-Floor-1 room shows its title, the attempt and the
    // TEST LOADOUT tag (PT-SCENE U5).
    if let Some((title, tag)) = &args.f01_hud {
        hud.set_room(title, *tag);
    }
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
    } else if args.audience_enabled() && run_room_has_broadcast(args) {
        // AUDIENCE-RUN (AR-AD1, AR-AD4): the broadcast is live in the run
        // rooms after the corral, at the carried counts or, without a carry,
        // at the baseline. Floor 01 authors no anticipation cue (AR-AD19).
        match args.f01_audience {
            Some(carry) => hud
                .resume_audience(carry.counts())
                .expect("the carry was validated when it was parsed"),
            None => hud
                .unlock_audience(Vec::new())
                .expect("an empty cue table is valid"),
        }
    }
    hud
}

/// AUDIENCE-RUN: whether the run's room is one the broadcast is live in
/// (positional, after the corral; fails closed when the manifest or the room
/// cannot be read).
fn run_room_has_broadcast(args: &Args) -> bool {
    let Some(room) = args.f01_room.as_deref() else {
        return false;
    };
    std::env::current_dir()
        .map_err(|e| e.to_string())
        .and_then(|root| f01_late::Manifest::load(&root))
        .is_ok_and(|manifest| f01_run::audience_live_in(&manifest, room))
}

/// The `AUDIENCE_STATE` line: printed only while the audience is unlocked and
/// only under the QA probe or an explicit `--audience=on` (AR-AD22), never by a
/// default run, and never part of the combat hash.
fn print_audience_state(args: &Args, hud: &HudRuntime) {
    if !(args.audience_probe || args.audience == Some(true)) {
        return;
    }
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
    if let Some(run) = &args.boiler_run {
        // BOIL-A wire: the sidecar's story pin (the post-Hoarder state).
        return f01_boiler::story_setup(&run.loaded.encounter.story_pin, args.carl, args.donut)
            .expect("the launch accepted the sidecar's pin");
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
    // One call for both programs: the per-hero grant filter sees the whole run.
    donut_prog::story_setup(args.carl, args.donut, scene)
}

/// CARL-PROG-01 CP-B2: whether the run can hold Carl's thrown charge, so its
/// presentation (the ANIM-CARL-03 sheet, the stick and burst sprites, the debug
/// ring) is loaded: a `carl_prog_*` fixture, `--carl-prog` or `--carl-dynamite`.
/// Presentation only; a run that cannot hold it reads no extra atlas.
fn presents_dynamite(args: &Args) -> bool {
    args.carl.prog
        || args.carl.dynamite
        || args
            .party_fixture
            .is_some_and(|pf| pf.name().starts_with("carl_prog_"))
}

/// One HUD step after a finished simulation tick: the real health values and
/// the newest combat events. Read-only on the simulation.
fn step_hud(hud: &mut HudRuntime, sim: &SceneSim, combat: Option<&CombatState>) {
    let (events, dropped) =
        combat.map_or((&[][..], 0), |c| (c.events.as_slice(), c.dropped_events));
    // Melee input is live exactly when the combat state exists (see the
    // manual input gate in `fixed_step`).
    hud.set_melee_live(combat.is_some());
    // CARL-PROG-01 CP-Q32: Carl's special socket (padlock, ready, live, recharging)
    // and its data label; read-only on the simulation.
    hud.set_special_label(&CarlProgTuning::active().cpq32_socket_label);
    hud.set_special(combat.map_or(SpecialPhase::Locked, SpecialPhase::of));
    // DONUT-PROG-01 DP-Q33: Donut's blink socket (padlock, equipped, cooling with
    // its underline, ready); the same read-only rule. Her flash comes from the
    // event log inside `step`.
    hud.set_donut_special(
        combat.map_or(dungeon_floorbreak::hud::SpecialSlot::Locked, |c| {
            dungeon_floorbreak::donut_hud::blink_socket_of(c, sim.world.tick)
        }),
    );
    // AUDIENCE-01: the heroes' columns, for authored anticipation cues (read
    // only; the observer is inert while the audience is locked).
    hud.set_hero_x(sim.world.hero.x as i32, sim.world.donut.x as i32);
    // REST-WIRE (REST-AD7): the refuge's staff door, for the solo swap hint (read only).
    hud.set_rest_door(
        combat
            .and_then(|c| c.env())
            .and_then(|env| env.surface_doors().first().copied())
            .map(|(rect, open)| dungeon_floorbreak::hud::RestDoorView {
                left_x: rect.x,
                open,
            }),
    );
    // ACT1-B: the chase's alcove hint after a checkpoint respawn (read only; false elsewhere).
    hud.set_alcove_hint(
        combat.is_some_and(|c| dungeon_floorbreak::f01_act1::alcove_hint_live(c, sim.world.tick)),
    );
    // HUD-DEBT-02: Donut's real mana (None while the biscuit is locked).
    hud.set_donut_mana(combat.and_then(CombatState::donut_mana_view));
    if let Some(combat) = combat {
        hud.set_empty_press(combat.donut_mana_tuning().empty_press);
        // F01-HUD-ROOM: the attempt counter (inert outside a room). HUD-TRY-01
        // (HT-AD1): a checkpoint respawn is an attempt too.
        hud.set_attempt(combat.hud_try());
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
        } else if let Some(run) = &args.hoarder_run {
            // HRD-A2b: the Hoarder spawns from the sidecar roster (ENEMY-SLOT);
            // the library builds the whole state, the verifier's constructor.
            let mut sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
            let combat = f01_hoarder::combat(run, &mut sim, args.carl, args.donut)?;
            (sim, combat)
        } else if let Some(run) = &args.hazards_run {
            // HAZ-A2b: the slimes, stickers and acid, the same way.
            let mut sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
            let combat =
                f01_hazards::combat_for(run, &mut sim, args.carl, args.donut, args.cf02.seed)?;
            (sim, combat)
        } else if let Some(run) = &args.mastery_run {
            // PT-MASTERY: the closing arena, the verifier's constructor.
            let mut sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
            let combat =
                f01_mastery::combat_for(run, &mut sim, args.carl, args.donut, args.cf02.seed)?;
            (sim, combat)
        } else if let Some(run) = &args.ring_run {
            // RING-A/B wire: a Ring room, the verifier's constructor.
            let mut sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
            let combat =
                f01_ring::combat_for(run, &mut sim, args.carl, args.donut, args.cf02.seed)?;
            (sim, combat)
        } else if let Some(run) = &args.rest_run {
            // REST-WIRE: the refuge, the verifier's constructor.
            let mut sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
            let combat =
                f01_rest::combat_for(run, &mut sim, args.carl, args.donut, args.cf02.seed)?;
            (sim, combat)
        } else if let Some(run) = &args.act1_run {
            // ACT1-A: an Act I room, the verifier's constructor.
            let mut sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
            let combat =
                f01_act1::combat_for(run, &mut sim, args.carl, args.donut, args.cf02.seed)?;
            (sim, combat)
        } else if let Some(run) = &args.corral_run {
            // CORRAL-WIRE: room 0, the verifier's constructor.
            let mut sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
            let combat =
                f01_corral::combat_for(run, &mut sim, args.carl, args.donut, args.cf02.seed)?;
            (sim, combat)
        } else if let Some(run) = &args.dozer_run {
            // DR-A: room 3, the verifier's constructor (the machine from the roster).
            let mut sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
            let combat =
                f01_dozer::combat_for(run, &mut sim, args.carl, args.donut, args.cf02.seed)?;
            (sim, combat)
        } else {
            let sim = SceneSim::new_at(kit, view_height(args.deck), fixture.carl_x)?;
            let combat = CombatState::new(fixture, kit, &sim)?
                .with_donut(donut_setup(args))
                .try_with_story(story_setup(args))
                .map_err(|e| e.to_string())?;
            (sim, combat)
        };
        if args.hoarder_run.is_some()
            || args.hazards_run.is_some()
            || args.mastery_run.is_some()
            || args.ring_run.is_some()
            || args.rest_run.is_some()
            || args.act1_run.is_some()
            || args.corral_run.is_some()
            || args.dozer_run.is_some()
        {
            // HRD-A2b, HAZ-A2b, PT-MASTERY, RING-A/B wire, CORRAL-WIRE and DR-A: the library built the state with its rules,
            // env and slot. A builder run after the slot would reset the attempt.
        } else if let Some(run) = &args.boiler_run {
            // BOIL-B/C wire: the sidecar's props and hazards are live.
            combat = combat
                .with_demo(run.rules)
                .with_env(f01_boiler::env_runtime(run)?);
            combat.sync_env(&mut sim);
        } else if args.cf02.rules {
            combat = combat.with_demo(&cf02::arena::RULES);
        } else if args.demo {
            combat = combat.with_demo(&demo::DEMO_RULES);
        }
        // COMBAT-FUN-02: `--cf02` runs the CF02 rules, llama targets (C1)
        // included (every `cf02_*` fixture already does, by its name).
        if args.cf02.rules
            && args.hoarder_run.is_none()
            && args.hazards_run.is_none()
            && args.mastery_run.is_none()
            && args.ring_run.is_none()
            && args.rest_run.is_none()
            && args.act1_run.is_none()
            && args.corral_run.is_none()
            && args.dozer_run.is_none()
        {
            combat = combat.with_cf02().with_cf02_seed(args.cf02.seed);
        }
        // F01-RUN: the ledger the floor run carried into this room, applied after
        // every builder (the verifier applies the descriptor's `f01_carry` after the
        // room's constructor, the same order).
        if let Some(snapshot) = &args.f01_carry {
            combat
                .carry_story(snapshot)
                .map_err(|e| format!("--f01-carry: {e}"))?;
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
        let aligned_pilot = args.room_pilot.filter(|p| p.aligned.is_some());
        if aligned_pilot.is_some() {
            // PT-GATE-C1: the library fixture's party (Donut is a real hero, the swap
            // and the partner are the party's; each tick sets the drive before it
            // steps, so the tick-0 state stays the legacy-drive one a recording hashes).
            *args.aligned_swap.lock().expect("the aligned memory") =
                room_pilots::AlignedMemory::default();
        }
        if args.room_pilot.is_some_and(|p| p.two_player) {
            // EVID-ROOM: two local players, so the pilot's second slot plays Donut.
            party.pad_connected(PadId(1));
            party.pad_start_pressed(PadId(2));
            sim.set_drive(party.drive(true));
        }
        if let Some(pf) = args.party_fixture {
            pf.place_donut(&mut sim);
            sim.set_drive(pf.drive(&party));
        } else if args.lab_run.is_some() {
            sim.set_drive(party.drive(true));
        }
        let mut feat_feed = AudienceFeatFeed::new(args);
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
            } else if let Some(pilot) = aligned_pilot {
                aligned_routed(args, pilot, &combat, &mut sim, &mut party)
            } else if let Some(kind) = args.act1_pilot {
                // ACT1-A evidence: the Act I pilot's slot input routed through the party (the
                // guild pilot's first press is the solo swap to Donut).
                let slots = [
                    dungeon_floorbreak::act1_fixtures::pilot(
                        kind,
                        &combat,
                        &sim,
                        room_exit_x(args),
                    ),
                    RawInput::default(),
                ];
                let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
                party.request_swap(slots[0].swap, hurt);
                sim.set_drive(party.drive(true));
                party.hero_inputs(slots)
            } else if let Some(kind) = args.rest_pilot {
                // REST-WIRE evidence: the refuge pilot's slot input routed through the party
                // (its first press is the solo swap to Donut), like the aligned pilots.
                let slots = [
                    dungeon_floorbreak::rest_fixtures::pilot(
                        kind,
                        &combat,
                        &sim,
                        room_exit_x(args),
                    ),
                    RawInput::default(),
                ];
                let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
                party.request_swap(slots[0].swap, hurt);
                sim.set_drive(party.drive(true));
                party.hero_inputs(slots)
            } else if args.room_pilot.is_some() {
                scripted_inputs(args, fixture, &sim, &combat)
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
    if let Some(pilot) = args.room_pilot {
        // EVID-ROOM: the scripted pilot of a room scene (slot 0; slot 1 below).
        pilot.inputs(combat, sim, room_exit_x(args))[0]
    } else if let (Some(kind), Some(_)) = (args.rest_pilot, &args.rest_run) {
        // REST-WIRE evidence: the refuge pilots (a pure function of the finished state).
        dungeon_floorbreak::rest_fixtures::pilot(kind, combat, sim, room_exit_x(args))
    } else if let (Some(kind), Some(_)) = (args.act1_pilot, &args.act1_run) {
        // ACT1-A evidence: the Act I pilots (a pure function of the finished state).
        dungeon_floorbreak::act1_fixtures::pilot(kind, combat, sim, room_exit_x(args))
    } else if let (true, Some(run)) = (args.ring_pilot, &args.ring_run) {
        // AREA-RINGS-01 evidence: the Ring library pilot (a pure function of the state).
        dungeon_floorbreak::f01_ring_fixtures::pilot(
            &dungeon_floorbreak::f01_ring_fixtures::Route::of(run, false),
            combat,
            sim,
            HeroId::Carl,
        )
    } else if args.lab_autopilot
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

/// EVID-ROOM: the column a room pilot walks to once nothing is left alive: the
/// middle of the room's protected exit.
fn room_exit_x(args: &Args) -> f32 {
    let exit = if let Some(run) = &args.boiler_run {
        run.loaded.encounter.exit
    } else if let Some(run) = &args.hoarder_run {
        run.loaded.encounter.exit
    } else if let Some(run) = &args.hazards_run {
        run.loaded.encounter.exit
    } else if let Some(run) = &args.ring_run {
        run.loaded.encounter.exit
    } else if let Some(run) = &args.rest_run {
        run.loaded.encounter.exit
    } else if let Some(run) = &args.act1_run {
        run.loaded.encounter.exit
    } else if let Some(run) = &args.corral_run {
        run.loaded.encounter.exit
    } else if let Some(run) = &args.dozer_run {
        run.loaded.encounter.exit
    } else {
        return 0.0;
    };
    (exit.x + exit.w / 2) as f32
}

/// PT-GATE-C1: the slot inputs of an aligned room pilot at the next tick (the
/// library fixture's pilot for the hero the party controls), advancing the run's
/// swap flag.
fn aligned_slots(
    args: &Args,
    pilot: room_pilots::RoomPilot,
    combat: &CombatState,
    sim: &SceneSim,
    party: &PartyState,
) -> [RawInput; 2] {
    let mut memory = args.aligned_swap.lock().expect("the aligned memory");
    pilot.aligned_inputs(
        combat,
        sim,
        party.controlled_hero(),
        room_exit_x(args),
        &mut memory,
    )
}

/// [`aligned_slots`] routed through the party like `PartyFixtureSim::step_slots`:
/// the swap, the scene drive and the slot to hero routing. The hero inputs of the tick.
fn aligned_routed(
    args: &Args,
    pilot: room_pilots::RoomPilot,
    combat: &CombatState,
    sim: &mut SceneSim,
    party: &mut PartyState,
) -> [RawInput; 2] {
    let slots = aligned_slots(args, pilot, combat, sim, party);
    let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
    party.request_swap(slots[0].swap, hurt);
    sim.set_drive(party.drive(true));
    party.hero_inputs(slots)
}

/// The slot inputs of a scripted run: the fixture's or autopilot's first slot,
/// and the lab autopilot's follower in a two-player lab run.
fn scripted_inputs(
    args: &Args,
    fixture: &Fixture,
    sim: &SceneSim,
    combat: &CombatState,
) -> [RawInput; 2] {
    if let Some(pilot) = args.room_pilot {
        // EVID-ROOM: Donut's slot is the CF02 pilot's in a two-player run.
        return pilot.inputs(combat, sim, room_exit_x(args));
    }
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
    // AUDIENCE-RUN: the showcase species' target ids, from the state the
    // events came from (a no-op unless the tuning has showcase feats on).
    if let Some(cine) = cine.as_deref_mut() {
        bridge_showcase_targets(cine, combat);
    }
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
struct AudienceFeatFeed {
    adapter: Option<CineAdapter>,
    cursor: EventCursor,
    /// AR-AD8: the same CINE tuning as the live adapter (showcase included).
    tuning: CineTuning,
    /// An explicit `--cine-showcase=on`: the feed runs, and prints the
    /// observer `CINE01_MOMENT` lines, even while the audience is locked
    /// (AR-AD22). Without it a locked audience keeps today's early return.
    explicit_showcase: bool,
}

impl AudienceFeatFeed {
    fn new(args: &Args) -> Self {
        Self {
            adapter: None,
            cursor: EventCursor::default(),
            tuning: args.cine_tuning(),
            explicit_showcase: args.cine_showcase == Some(true),
        }
    }

    /// One finished tick: call before [`step_hud`] (the live loop also queues
    /// the feats before its HUD step).
    fn observe(&mut self, combat: &CombatState, hud: &mut HudRuntime) {
        let unlocked = hud.audience().is_unlocked();
        if !unlocked && !self.explicit_showcase {
            return;
        }
        let tuning = self.tuning;
        let adapter = self
            .adapter
            .get_or_insert_with(|| CineAdapter::with_tuning(PresentationSettings::OFF, tuning));
        bridge_showcase_targets(adapter, combat);
        // CA-29: the CINE filter does not apply here. AUDIENCE credit stays
        // factual per hero (it never triggers a slow-motion moment).
        for event in self.cursor.next(&combat.events, combat.dropped_events) {
            adapter.push(event);
        }
        for emission in adapter.flush() {
            if self.explicit_showcase {
                // Observer line (presentation OFF, so no CINE01_SLOWMO line).
                // The feed runs in the pre-simulation, so on a headless or
                // capture run these lines are NOT tick-ordered against the
                // `COMBAT_EVENT` lines (those print afterwards, as one batch):
                // consumers filter by the `CINE01_MOMENT` prefix.
                println!("{}", cine_moment_line(&emission.moment));
            }
            if unlocked {
                hud.push_chain_feat(chain_feat(&emission.moment));
            }
        }
    }
}

/// AUDIENCE-RUN: tell the CINE adapter which target ids are showcase species
/// (the llamas, which follow the goblins in the target id space), read from the
/// authoritative combat state. A no-op for the evaluator while the tuning has
/// showcase feats off.
fn bridge_showcase_targets(adapter: &mut CineAdapter, combat: &CombatState) {
    let goblins = u32::try_from(combat.enemies.len()).unwrap_or(u32::MAX);
    let llamas = u32::try_from(combat.llamas.len()).unwrap_or(u32::MAX);
    adapter.set_showcase_targets(goblins, llamas);
}

/// The `CINE01_MOMENT` line of an emitted moment.
fn cine_moment_line(m: &dungeon_floorbreak::cine::CinematicMoment) -> String {
    format!(
        "CINE01_MOMENT tick={} tier={} score={} credited={} target={} precision_edge={} mixed_chain={} contact_setup={} extra_kills={}{}{}{}{}",
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
        // AR-AD21: only when set, before ` upgrade=1`, so a legacy line is
        // byte-identical.
        if m.feats.thrown_multi {
            " thrown_multi=1"
        } else {
            ""
        },
        if m.feats.showcase_kill {
            " showcase_kill=1"
        } else {
            ""
        },
        if m.feats.showcase_bonus {
            " showcase_bonus=1"
        } else {
            ""
        },
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
    // PT-SCENE: the launcher names its room, the scene it runs on and the loadout.
    if args.f01_late
        && let Ok(root) = std::env::current_dir()
        && let Ok(manifest) = f01_late::Manifest::load(&root)
        && let Some(room) = args
            .f01_room
            .as_deref()
            .and_then(|id| manifest.launchable(id).ok())
    {
        println!("{}", f01_late::entry_line(room, args.f01_loadout));
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
    // BOIL-A wire: load the Goblin Boiler room and its sidecar, with the pin.
    if args.f01_boiler {
        // BOIL-E (K16): the story launcher runs the sidecar's roster live; the
        // plain `--scene f01_boiler` stays the inert greybox.
        let roster = f01_boiler::Roster::for_loadout(args.f01_loadout.map(|l| l.name()));
        match f01_boiler::launch_with(&root, roster) {
            Ok(run) => {
                println!("{}", f01_boiler::entry_line(&run));
                args.fixture = Some(run.fixture);
                args.boiler_run = Some(Box::new(run));
            }
            Err(error) => {
                eprintln!("floor01_scene: {error}");
                return ExitCode::from(2);
            }
        }
    }
    // HRD-A2b: load the Hoarder's room and its sidecar.
    if args.f01_hoarder {
        match f01_hoarder::launch(&root) {
            Ok(run) => {
                println!("{}", f01_hoarder::entry_line(&run));
                args.fixture = Some(run.fixture);
                args.hoarder_run = Some(Box::new(run));
            }
            Err(error) => {
                eprintln!("floor01_scene: {error}");
                return ExitCode::from(2);
            }
        }
    }
    // HAZ-A2b: load the hazard room and its sidecar.
    if args.f01_hazards {
        match f01_hazards::launch(&root) {
            Ok(run) => {
                println!("{}", f01_hazards::entry_line(&run));
                args.fixture = Some(run.fixture);
                args.hazards_run = Some(Box::new(run));
            }
            Err(error) => {
                eprintln!("floor01_scene: {error}");
                return ExitCode::from(2);
            }
        }
    }
    // PT-MASTERY: load the closing arena and its sidecar.
    if args.f01_mastery {
        match f01_mastery::launch(&root) {
            Ok(run) => {
                println!("{}", f01_mastery::entry_line(&run));
                args.fixture = Some(run.fixture);
                args.mastery_run = Some(Box::new(run));
            }
            Err(error) => {
                eprintln!("floor01_scene: {error}");
                return ExitCode::from(2);
            }
        }
    }
    // RING-A/B wire: load the Ring room and its sidecar.
    if let Some(room) = args.f01_ring {
        match f01_ring::launch(&root, room) {
            Ok(run) => {
                println!("{}", f01_ring::entry_line(&run));
                args.fixture = Some(run.fixture);
                args.ring_run = Some(Box::new(run));
            }
            Err(error) => {
                eprintln!("floor01_scene: {error}");
                return ExitCode::from(2);
            }
        }
    }
    // REST-WIRE: load the refuge and its sidecar. The entry line reads the room's tick-0
    // state (the staff door and the boost), so a throwaway combat state is built the way
    // the scene builds it, with the run's carried ledger applied.
    if let Some(room) = args.f01_rest {
        let launched = f01_rest::launch(&root, room).and_then(|run| {
            let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x)?;
            let mut combat =
                f01_rest::combat_for(&run, &mut scene, args.carl, args.donut, args.cf02.seed)?;
            if let Some(snapshot) = &args.f01_carry {
                combat
                    .carry_story(snapshot)
                    .map_err(|e| format!("--f01-carry: {e}"))?;
            }
            let line = f01_rest::entry_line(&run, &combat);
            Ok((run, line))
        });
        match launched {
            Ok((run, line)) => {
                println!("{line}");
                args.fixture = Some(run.fixture);
                args.rest_run = Some(Box::new(run));
            }
            Err(error) => {
                eprintln!("floor01_scene: {error}");
                return ExitCode::from(2);
            }
        }
    }
    // ACT1-A: load the Act I room and its sidecar. The entry line reads the room's tick-0
    // state (the door and the biscuit), so a throwaway combat state is built the way the scene
    // builds it, with the run's carried (empty) ledger applied.
    if let Some(room) = args.f01_act1 {
        let launched = f01_act1::launch(&root, room).and_then(|run| {
            let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x)?;
            let mut combat =
                f01_act1::combat_for(&run, &mut scene, args.carl, args.donut, args.cf02.seed)?;
            if let Some(snapshot) = &args.f01_carry {
                combat
                    .carry_story(snapshot)
                    .map_err(|e| format!("--f01-carry: {e}"))?;
            }
            let line = f01_act1::entry_line(&run, &combat);
            Ok((run, line))
        });
        match launched {
            Ok((run, line)) => {
                println!("{line}");
                args.fixture = Some(run.fixture);
                args.act1_run = Some(Box::new(run));
            }
            Err(error) => {
                eprintln!("floor01_scene: {error}");
                return ExitCode::from(2);
            }
        }
    }
    // CORRAL-WIRE: load room 0 and its sidecar.
    if args.f01_corral {
        match f01_corral::launch(&root) {
            Ok(run) => {
                println!("{}", f01_corral::entry_line(&run));
                args.fixture = Some(run.fixture);
                args.corral_run = Some(Box::new(run));
            }
            Err(error) => {
                eprintln!("floor01_scene: {error}");
                return ExitCode::from(2);
            }
        }
    }
    // DR-A: load room 3 and its sidecar.
    if args.f01_dozer {
        match f01_dozer::launch(&root) {
            Ok(run) => {
                println!("{}", f01_dozer::entry_line(&run));
                args.fixture = Some(run.fixture);
                args.dozer_run = Some(Box::new(run));
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
    } else if let Some(run) = &args.boiler_run {
        Ok(run.kit.clone())
    } else if let Some(run) = &args.hoarder_run {
        Ok(run.kit.clone())
    } else if let Some(run) = &args.hazards_run {
        Ok(run.kit.clone())
    } else if let Some(run) = &args.mastery_run {
        Ok(run.kit.clone())
    } else if let Some(run) = &args.ring_run {
        Ok(run.kit.clone())
    } else if let Some(run) = &args.rest_run {
        Ok(run.kit.clone())
    } else if let Some(run) = &args.act1_run {
        Ok(run.kit.clone())
    } else if let Some(run) = &args.corral_run {
        Ok(run.kit.clone())
    } else if let Some(run) = &args.dozer_run {
        Ok(run.kit.clone())
    } else if args.refuge {
        refuge::load(&root).map(|(kit, _)| kit)
    } else if args.arena_room() {
        // DP-F: a `donut_prog_arena_ledge_*` capture fixture adds its 128 px step.
        cf02::arena::load_arena_for(&root, args.fixture.map_or("", |f| f.name))
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
    let presents_dynamite = presents_dynamite(&args);
    let draws_blast = !args.headless
        && (args.fixture.is_some_and(|f| !f.charges.is_empty()) || presents_dynamite);
    // CF02-I: the CF02 sheets are read only by a run under `cf02_rules`.
    let runs_cf02 =
        dungeon_floorbreak::cf02::run_uses_cf02(args.cf02.rules, args.fixture.map(|f| f.name));
    let actors = match Actors::load(
        &root,
        source,
        args.fixture.is_some(),
        draws_blast,
        runs_cf02,
        presents_dynamite,
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
    /// LLAMA-03: the Bad Llama rush atlas, drawn instead of the stand-in body during a corral rush. `None`
    /// (with one `LLAMA_RUSH_ART fallback` log line) when it is missing or invalid: the scene then keeps the
    /// CORRAL-E stand-in body and never fails.
    llama_rush: Option<SpriteClips>,
    /// ENEMY-GAMEPLAY-ENGINEER: the ENG-01 atlas, loaded for a `cf02_rules` run
    /// (the only runs that can have an Engineer).
    engineer: Option<SpriteClips>,
    /// CARL-PROG-01 CP-B2: the ANIM-CARL-03 sheet, loaded for a run that can hold
    /// the thrown charge (`None` everywhere else: no atlas is read or drawn).
    dynamite: Option<SpriteClips>,
}

impl Actors {
    fn load(
        root: &Path,
        source: FrameSource,
        combat: bool,
        blast: bool,
        cf02: bool,
        dynamite: bool,
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
                llama_rush: match SpriteClips::load_llama_rush(root, source) {
                    Ok(clips) => {
                        println!("LLAMA_RUSH_ART on frames={}", clips.frame_count);
                        Some(clips)
                    }
                    Err(error) => {
                        println!("LLAMA_RUSH_ART fallback to the stand-in body: {error}");
                        None
                    }
                },
                engineer: if cf02 {
                    Some(SpriteClips::load_engineer(root, source)?)
                } else {
                    None
                },
                dynamite: if dynamite {
                    Some(SpriteClips::load_carl_dynamite(root, source)?)
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
            if let Some(dynamite) = art.dynamite.as_ref() {
                animators = animators.with_dynamite(dynamite.clone());
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
    /// `<repository root>/generated/art`: the directory the asset server reads
    /// (the presenters that probe for an optional atlas look here).
    art_dir: PathBuf,
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

/// AREA-REFUGE-01: Mordecai in the guild room (`None` outside it, or while his mark is off).
#[derive(Resource)]
struct MordecaiRes(Option<(ActorSpec, refuge::Mordecai)>);

/// The marker of Mordecai's sprite.
#[derive(Component)]
struct MordecaiVisual;

/// Lighting footprint of Mordecai's contact shadow: the standing body, not the 52 px canvas.
const MORDECAI_SHADOW_WIDTH: i32 = 34;

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
    /// EquippedSpecial (keyboard L).
    ability_tap: bool,
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

/// LLAMA-03: the two image and layout pairs of a llama sprite, stored once at spawn: the LLAMA-01 atlas and
/// the rush atlas (`llama_rush_h64`, same frame size and pivot). `sync_llama_visuals` swaps the sprite's image
/// and atlas layout between them in place (same entity, flip, flash tint and z). Absent when the rush atlas
/// is missing: the stand-in body is drawn then.
#[derive(Resource)]
struct LlamaRushLook {
    base_image: Handle<Image>,
    base_layout: Handle<TextureAtlasLayout>,
    rush_image: Handle<Image>,
    rush_layout: Handle<TextureAtlasLayout>,
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

/// CARL-PROG-01 CP-B2: Carl's dynamite throw sprite (the ANIM-CARL-03 atlas:
/// the standing throw and the moving overlay frames). It replaces the locomotion
/// sprite while a throw frame is drawn. Presentation only.
#[derive(Component)]
struct CarlThrowSprite;

/// Points of the `--debug-combat` blast ring (CP-B2): a 64 px circle, one every
/// 9 degrees.
const BLAST_RING_POINTS: usize = 40;

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
    /// CP-B2: point `n` of the blast radius ring (drawn by
    /// [`sync_blast_ring`]).
    BlastRing(usize),
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
    } else if args.f01_boiler {
        // PT-SCENE: the Boiler has its own replay source.
        (f01_late::SESSION_SOURCE, f01_boiler::SCENE_NAME)
    } else if args.f01_hoarder {
        // HRD-A2b: the Hoarder's room records under the same source.
        (f01_late::SESSION_SOURCE, f01_hoarder::SCENE_NAME)
    } else if args.f01_hazards {
        // HAZ-A2b: so does the hazard room.
        (f01_late::SESSION_SOURCE, f01_hazards::SCENE_NAME)
    } else if args.f01_mastery {
        // PT-MASTERY: and the closing arena.
        (f01_late::SESSION_SOURCE, f01_mastery::SCENE_NAME)
    } else if let Some(ring) = args.f01_ring {
        // RING-A/B wire: and the two Ring rooms, under their own fixture names.
        (f01_late::SESSION_SOURCE, ring.scene_name())
    } else if let Some(rest) = args.f01_rest {
        // REST-WIRE: and the refuge, under its fixture name.
        (f01_late::SESSION_SOURCE, rest.scene_name())
    } else if let Some(act1) = args.f01_act1 {
        // ACT1-A: and the Act I rooms, under their fixture names.
        (f01_late::SESSION_SOURCE, act1.scene_name())
    } else if args.f01_corral {
        // CORRAL-WIRE: and room 0, under its fixture name.
        (f01_late::SESSION_SOURCE, f01_corral::SCENE_NAME)
    } else if args.f01_dozer {
        // DR-A: and room 3, under its fixture name.
        (f01_late::SESSION_SOURCE, f01_dozer::SCENE_NAME)
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
        donut_motion: party.tuning().donut_motion,
        donut_prog: args.donut.prog,
        donut_light_feet: args.donut.light_feet,
        donut_puddle: args.donut.puddle,
        f01_room: args.f01_room.clone(),
        f01_loadout: args.f01_loadout.map(|l| l.name().to_owned()),
        // PR-C (BOIL-FU): the Boiler's roster, explicit (the verifier reads it).
        f01_roster: args.f01_boiler.then(|| {
            f01_boiler::Roster::for_loadout(args.f01_loadout.map(|l| l.name()))
                .name()
                .to_owned()
        }),
        // F01-RUN: the carried ledger, so the verifier rebuilds this start state.
        f01_carry: args.f01_carry.clone(),
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
        ("donut_prog", args.donut.prog),
        ("donut_light_feet", args.donut.light_feet),
        ("donut_puddle", args.donut.puddle),
    ] {
        if value {
            manifest
                .feature_flags
                .insert(key.to_owned(), "true".to_owned());
        }
    }
    // PT-SCENE: the room and the loadout, next to the descriptor fields.
    if let Some(room) = &args.f01_room {
        manifest
            .feature_flags
            .insert("f01_room".to_owned(), room.clone());
    }
    if let Some(loadout) = args.f01_loadout {
        manifest
            .feature_flags
            .insert("f01_loadout".to_owned(), loadout.name().to_owned());
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
    shutdown_signal::install();
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
    let asset_directory_path = root.join("generated/art");
    let asset_directory = asset_directory_path.to_string_lossy().into_owned();
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
    let floor_run = args.f01_run;
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
    // ACT1-A (ACT1-AD11): the guild's guide is Mordecai's existing `mordecai_h44` sprite on the
    // refuge's mark (x 380, facing left), at the guide decor's column. Presentation only.
    let act1_guide = args
        .act1_run
        .as_ref()
        .filter(|run| run.room == f01_act1::Act1Room::Guild)
        .and_then(|run| {
            f01_act1::decor(run.room)
                .iter()
                .find(|d| d.label == "guide")
        })
        .map(|d| d.rect.x + d.rect.w / 2);
    let mordecai = if (args.refuge && !args.refuge_empty) || act1_guide.is_some() {
        match refuge::Room::load(root).and_then(|room| room.mordecai(root)) {
            Ok(found) => found.map(|mut m| {
                if let Some(x) = act1_guide {
                    m.x = x;
                    m.facing = refuge::Facing::Left;
                }
                let spec = ActorSpec {
                    name: "MORDECAI",
                    width: m.frame_w,
                    height: m.frame_h,
                    pivot_x: m.pivot_x,
                    pivot_y: m.pivot_y,
                    shadow_width: MORDECAI_SHADOW_WIDTH,
                    shadow_pivot_x: m.pivot_x,
                };
                (spec, m)
            }),
            Err(message) => {
                println!("FLOOR1_ASSET_FAILURE: {message}");
                eprintln!("FLOOR1_ASSET_FAILURE: {message}");
                std::process::exit(1);
            }
        }
    } else {
        None
    };
    let reference = if args.refuge {
        // AREA-REFUGE-01: Carl and Donut on the room's marks (already checked by the kit load).
        match refuge::Room::load(root) {
            Ok(room) => {
                let cast = room.cast_x();
                // Empty room: the cast stands far outside the 640 px view.
                let away = if args.refuge_empty { -400.0 } else { 0.0 };
                Some(RefPose {
                    carl_x: cast.carl + away,
                    donut_x: cast.donut + away,
                })
            }
            Err(message) => {
                println!("FLOOR1_ASSET_FAILURE: {message}");
                std::process::exit(1);
            }
        }
    } else {
        args.reference_frame.then_some(RefPose {
            carl_x: REFERENCE_CARL_X,
            donut_x: REFERENCE_DONUT_X,
        })
    };
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
        .set(RenderPlugin {
            synchronous_pipeline_compilation: exit_after.is_some() && args.perf_frames.is_none(),
            ..default()
        })
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
        adapter: CineAdapter::with_tuning(cine_settings, args.cine_tuning()),
        speed: 1.0,
        run: None,
        last_elapsed: 0,
    })
    .insert_resource(ExitAfter(exit_after))
    .insert_resource(LightingOn(lighting_on))
    .insert_resource(ReferencePose(reference))
    .insert_resource(MordecaiRes(mordecai))
    .insert_resource(LiveScenario(scenario))
    .insert_resource(Combat(combat))
    .insert_resource(BossArtRes(boss_art))
    .insert_resource(Death(death))
    .insert_resource(camera)
    .init_resource::<Interest>()
    .insert_resource(LiveFixture(live_fixture))
    .insert_resource(HudAtlas(hud_atlas, hud_emblem))
    .insert_resource(Setup {
        kit,
        args,
        art_dir: asset_directory_path,
    })
    .init_resource::<Controls>()
    .insert_resource(Pads(PadSlots::new(party)))
    .insert_resource(Recorder(recorder))
    .init_resource::<AfterExitTicks>()
    .add_systems(Startup, setup)
    // After Bevy's window-exit systems: closing the window with its X writes `AppExit` in
    // `Last`, and the session must be finished in the same frame (FIX-TOOL).
    .add_systems(Last, finish_recording.after(bevy::window::ExitSystems))
    .add_systems(
        Update,
        (
            cine_frame,
            perf_step,
            sync_visuals,
            sync_mordecai,
            sync_combat_visuals,
            sync_donut_visuals,
            sync_boss_visuals,
            sync_llama_visuals,
            sync_dust_visuals,
            sync_shore_visuals,
            sync_blast_visuals,
            sync_death_visuals,
            sync_cf02_acting_visuals,
            sync_carl_throw_visual,
            sync_blast_ring,
            (
                // CARL-GAUNTLET-01: after every Carl body sprite is synced this frame.
                carl_gauntlet::sync_carl_gauntlet,
                donut_cues::sync_donut_cues,
                blink_cues::sync_blink_cues,
                pierce_spark::sync_pierce_spark,
                env_prop_art::sync_prop_art,
                env_prop_art::sync_ring_shutter,
                hazards_art::sync_hazard_art,
                danger_fx_art::sync_danger_fx,
                danger_fx_art::sync_acid_splash,
                hazard_decals::sync_hazard_decals,
                hazards_art::sync_hazard_debug,
                hoarder_art::sync_hoarder_art,
                dozer_art::sync_dozer_art,
                room_exit_art::sync_room_exit,
                gym_art::sync_gym_art,
                corral_art::sync_corral_art,
                corral_art::sync_partition_art,
                (fastfood_art::sync_fastfood_art, camp_art::sync_bag_art),
                sync_act1_door,
                sync_act1_chase,
            )
                .chain(),
            update_lighting,
            watchdog,
            perf_probe,
            capture_sequence,
        )
            .chain(),
    );
    // Every window kind honours SIGINT and SIGTERM (the replay viewer included): the handler
    // is installed for all of them in `run_window` (FIX-TOOL).
    app.add_systems(Update, shutdown_on_signal);
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
        app.add_plugins(lab);
        if floor_run {
            app.add_systems(FixedUpdate, f01_run_step.after(fixed_step));
        } else {
            app.add_systems(FixedUpdate, lab_step.after(fixed_step));
        }
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
    mordecai: Res<MordecaiRes>,
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

    // Placed pieces (decals, structure, props, foreground). ROOM-PORTALS-01: in
    // rooms 2, 4, 4a, 4b and 6 the exit door takes the Guild door's place (its z and
    // layer); the end torch it covers but for the flame tip is not drawn either.
    // Its sprite is spawned with the combat art below.
    let exit_art = room_exit_art::replaced_exit(args).and_then(|exit| {
        kit.pieces
            .iter()
            .find(|p| room_exit_art::is_replaced(&p.piece))
            .map(|p| (exit, p.z, layers_of(&p.layer)))
    });
    for piece in &kit.pieces {
        if exit_art
            .as_ref()
            .is_some_and(|(exit, _, _)| room_exit_art::skips(*exit, piece))
        {
            continue;
        }
        let mut entity = commands.spawn((
            atlas_sprite(&atlas(&piece.atlas), piece.rect),
            Anchor::TOP_LEFT,
            Transform::from_xyz(piece.x as f32, to_world_y(height, piece.y), piece.z),
            layers_of(&piece.layer),
        ));
        if piece.frames.len() > 1 {
            entity.insert(torch_component(piece));
        }
        // AREA-GYM-01: the exit door's state pair (`gym_art`).
        if let Some(marker) = gym_art::marker(&piece.piece) {
            entity.insert((marker, gym_art::initial_visibility(&piece.piece)));
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

    // AREA-REFUGE-01: Mordecai, behind Donut and Carl, on his mark (first idle frame).
    let mut refuge_handles: Vec<(String, Handle<Image>)> = Vec::new();
    if let Some((_, m)) = mordecai.0.as_ref() {
        let texture: Handle<Image> = asset_server.load(m.atlas_asset.clone());
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(m.frame_w as u32, m.frame_h as u32),
            m.frame_count as u32,
            1,
            None,
            None,
        ));
        commands.spawn((
            MordecaiVisual,
            Sprite::from_atlas_image(
                texture.clone(),
                TextureAtlas {
                    layout,
                    index: m.idle_first,
                },
            ),
            Transform::from_xyz(0.0, 0.0, kit.actor_z - 0.1),
            layers_of("actors"),
        ));
        refuge_handles.push((m.atlas_asset.clone(), texture));
    }

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
        // CP-B2: Carl's dynamite throw (hidden until a throw frame is drawn).
        if let Some(clips) = art.dynamite.as_ref() {
            let texture: Handle<Image> = asset_server.load(clips.atlas_asset.clone());
            commands.spawn((
                CarlThrowSprite,
                Sprite::from_atlas_image(
                    texture.clone(),
                    TextureAtlas {
                        layout: layout_of(clips),
                        index: clips.tag("dynamite_throw").0,
                    },
                ),
                Transform::from_xyz(0.0, 0.0, kit.actor_z + CARL_Z_OFFSET),
                Visibility::Hidden,
                layers_of("actors"),
            ));
            combat_handles.push((clips.atlas_asset.clone(), texture));
            combat_entities += 1;
        }
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
            // ENG-02: the Engineer's slammed sheet (his own belt and no weapon).
            let engineer_slammed_texture = load(&acting.engineer_slammed);
            for (index, enemy) in state.enemies.iter().enumerate() {
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
                let (slammed_clips, slammed_texture) = if enemy.is_engineer() {
                    (&acting.engineer_slammed, &engineer_slammed_texture)
                } else {
                    (&acting.slammed[variant], &slammed_textures[variant])
                };
                commands.spawn((
                    GoblinSlammedSprite { index },
                    sprite(slammed_clips, slammed_texture, "slammed_stomped"),
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
            if let Some(rush) = art.llama_rush.as_ref() {
                let rush_image: Handle<Image> = asset_server.load(rush.atlas_asset.clone());
                combat_handles.push((rush.atlas_asset.clone(), rush_image.clone()));
                commands.insert_resource(LlamaRushLook {
                    base_image: texture.clone(),
                    base_layout: layout.clone(),
                    rush_image,
                    rush_layout: layout_of(rush),
                });
            }
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
            // CP-B2 (CP-Q33): the blast radius ring of a live thrown charge, a
            // pool of small amber points (the 64 px reach is wider than its art).
            if presents_dynamite(args) {
                for point in 0..BLAST_RING_POINTS {
                    overlay(
                        CombatDebugBox::BlastRing(point),
                        Color::srgba(1.0, 0.75, 0.1, 0.9),
                        Vec2::new(2.0, 2.0),
                    );
                }
            }
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
        // DONUT-MOVES-ART-01: the moves, tail-swap and Puddle Jumper atlases (a missing
        // atlas leaves its cue undrawn); the handles join the loading watchdog.
        let (moves_art, moves_handles) = donut_cues::load_art(&asset_server, &mut layouts);
        combat_handles.extend(moves_handles);
        donut_cues::spawn(
            &mut commands,
            donut_texture.clone(),
            layers_of("actors"),
            kit.actor_z,
            &moves_art,
        );
        // DP-D (DP-Q34): the Puddle Jumper puddles, streak and her blink frames, only
        // where the blink can exist (the same condition as the cues' pool).
        blink_cues::spawn(&mut commands, layers_of("actors"), kit.actor_z, &moves_art);
        // CORRAL-WIRE (CW-AD5) and CORRAL-E (CRL-Q38): room 0's grate and the rush
        // stand-in, only where a gate or a corral llama exists.
        let rush_llamas = combat.0.as_ref().map_or(0, |c| {
            c.llamas.iter().filter(|l| l.corral.is_some()).count()
        });
        let gate = args
            .corral_run
            .as_ref()
            .and_then(|run| run.m3_gate.as_ref().map(|(rect, _)| *rect));
        corral_art::spawn(
            &mut commands,
            layers_of("actors"),
            kit.actor_z + 0.02,
            gate,
            rush_llamas,
        );
    }

    // BOIL-E (K10) / AREA-GOBLINWORKSHOP-01: the Boiler's prop art (the kit atlases;
    // code-drawn footprints and placeholders), only in a room that has props (RING-A/B
    // wire: the court's greybox shutter stand-in, KR17).
    let prop_room = args.boiler_run.as_ref().map(|r| &r.loaded).or_else(|| {
        args.ring_run
            .as_ref()
            .filter(|r| r.shutter().is_some())
            .map(|r| &r.loaded)
    });
    if let Some(loaded) = prop_room {
        env_prop_art::spawn(
            &mut commands,
            &asset_server,
            &mut layouts,
            &mut combat_handles,
            loaded.encounter.clone(),
            layers_of("actors"),
            kit.actor_z,
            kit.foreground_z,
        );
    }

    // AREA-RINGS-01: the court's stone shutter (the key cropped by `edge_px` plus the
    // extras of spec section 6) in the greybox stand-in's place. The slab sits just
    // under the dressing's housing piece (a prop-layer piece), so the housing draws
    // over its top rows. Presentation only.
    if let Some(run) = args.ring_run.as_ref().filter(|r| r.shutter().is_some()) {
        let slab_z = kit
            .pieces
            .iter()
            .find(|p| p.piece == "ring_shutter_housing")
            .map_or(kit.actor_z - 1.0, |p| p.z - 0.0005);
        env_prop_art::spawn_ring_shutter(
            &mut commands,
            &atlas("floor01_ring_shutter"),
            &run.loaded.encounter,
            slab_z,
            kit.foreground_z,
            layers_of("prop"),
            layers_of("foreground"),
        );
    }

    // CORRAL-D (`HEAT-AD1`): room 0's breakable partition, the frame of the prop's state
    // from the heat atlas (above the dressing's props, below the actors). Presentation only.
    if let Some(run) = args.corral_run.as_ref()
        && let Some((_, texture)) = handles.iter().find(|(n, _)| n == "floor01_heat_partition")
    {
        let z = kit
            .pieces
            .iter()
            .filter(|p| p.layer == "prop")
            .map(|p| p.z)
            .fold(kit.actor_z - 1.0, f32::max)
            + 0.001;
        corral_art::spawn_partition(
            &mut commands,
            &texture.clone(),
            &run.loaded.encounter,
            z.min(kit.actor_z - 0.01),
            layers_of("prop"),
        );
    }

    // REST-WIRE: the quiet rooms. The refuge is dressed by AREA-FASTFOOD-01 (its kit pieces come from
    // the dressing and its staff door, box, sparkle and bell are the presenter's sprites); the camp and
    // the aftermath are dressed by AREA-CAMP-01 (the aftermath's heavy bag is the presenter's sprite).
    // Presentation only. Only an undressed room keeps the flat labelled greybox decor.
    if let Some(run) = args.rest_run.as_ref() {
        let prop_z = kit
            .pieces
            .iter()
            .filter(|p| p.layer == "prop")
            .map(|p| p.z)
            .fold(kit.actor_z - 1.0, f32::max)
            + 0.001;
        match run.room {
            f01_rest::RestRoom::Refuge => {
                if let (Some((_, door)), Some((_, pickup))) = (
                    handles
                        .iter()
                        .find(|(n, _)| n == dungeon_floorbreak::fastfood_kit::DOOR_ATLAS),
                    handles
                        .iter()
                        .find(|(n, _)| n == dungeon_floorbreak::fastfood_kit::PICKUP_ATLAS),
                ) {
                    fastfood_art::spawn(
                        &mut commands,
                        door,
                        pickup,
                        height,
                        kit.actor_z - 0.1,
                        prop_z.min(kit.actor_z - 0.01),
                        layers_of("prop"),
                    );
                }
            }
            f01_rest::RestRoom::Camp => {}
            f01_rest::RestRoom::Aftermath => {
                if let Some((_, bag)) = handles
                    .iter()
                    .find(|(n, _)| n == dungeon_floorbreak::camp_kit::BAG_ATLAS)
                {
                    camp_art::spawn(
                        &mut commands,
                        bag,
                        height,
                        prop_z.min(kit.actor_z - 0.01),
                        layers_of("prop"),
                    );
                }
            }
        }
    }

    // ACT1-A: the Act I rooms' flat labelled greybox decor and the guild door (no art yet:
    // ACT1-Q7). Presentation only.
    if let Some(run) = args.act1_run.as_ref() {
        spawn_act1_greybox(
            &mut commands,
            run,
            kit,
            height,
            layers_of("prop"),
            mordecai.0.is_some(),
        );
    }

    // HAZ-B: the slimes, the Rot Stickers, the acid strip, the warning ring and the
    // spent drip, only in a room that has them (presentation only).
    if let Some(state) = combat.0.as_ref() {
        let debug_layers = args.debug_combat.then(|| {
            if lighting.is_some() {
                RenderLayers::layer(LAYER_DEBUG)
            } else {
                RenderLayers::default()
            }
        });
        hazards_art::spawn(
            &mut commands,
            &asset_server,
            &mut layouts,
            state,
            layers_of("actors"),
            debug_layers,
            kit,
            &mut combat_handles,
            &mut combat_entities,
        );
        // F01 danger FX: the acid pool, the reach rings, the blast wave and the
        // room 5 gate art (presentation only; one pool for every combat scene).
        danger_fx_art::spawn(
            &mut commands,
            &asset_server,
            &mut combat_handles,
            layers_of("actors"),
            kit,
            &mut combat_entities,
        );
        // AREA-HAZARDS-01B: the corrosion under the acid, the Rot nests and the spent
        // sticker's drip (presentation only; nothing for a room without acid or stickers).
        hazard_decals::spawn(
            &mut commands,
            &asset_server,
            &mut combat_handles,
            layers_of("actors"),
            kit,
            state,
            &mut combat_entities,
        );
    }

    // ROOM-PORTALS-01: the exit door in the Guild door's place.
    if let Some((exit, z, layer)) = exit_art {
        room_exit_art::spawn(
            &mut commands,
            &asset_server,
            &mut combat_handles,
            layer,
            z,
            exit,
        );
    }

    // FA-Q23 (HRD-P) and HOARD-02: the Hoarder and her room kit (door, gate, bank A, the J-B post
    // and stack, the lair mound, the dressing heaps), only in her room (presentation only).
    if let Some(run) = &args.hoarder_run {
        hoarder_art::spawn(
            &mut commands,
            &asset_server,
            &mut layouts,
            run.loaded.encounter.clone(),
            layers_of("actors"),
            kit,
            &mut combat_handles,
            &mut combat_entities,
        );
    }

    // DR-A and DOZ-02: the Dozer's rig and its damage states, its smoke, the crew's tumbles, the rear
    // gate, the buffer stop and the stop frame, only in room 3 (presentation only).
    if let Some(run) = &args.dozer_run {
        // The tunnel lintel's tiles and the Guild door piece they sit behind.
        let lintel = run
            .loaded
            .encounter
            .dozer
            .as_ref()
            .map_or_else(Vec::new, |site| {
                // Room 3 always holds its lintel (the sidecar's rule 4).
                site.lintel.map_or_else(Vec::new, |lintel| {
                    dungeon_floorbreak::dozer_art::lintel_sprites(
                        to_world_y(height, kit.base_walk_line_y) as i32,
                        kit.base_walk_line_y,
                        lintel,
                    )
                })
            });
        let door = kit
            .pieces
            .iter()
            .find(|p| p.piece == dungeon_floorbreak::room_exit_art::PORTAL_PIECE)
            .map(|p| (p.z, layers_of(&p.layer)));
        dozer_art::spawn(
            &mut commands,
            &asset_server,
            &mut layouts,
            layers_of("actors"),
            &lintel,
            door,
            kit,
            &mut combat_handles,
            &mut combat_entities,
        );
    } else if args
        .act1_run
        .as_ref()
        .is_some_and(|run| run.room == f01_act1::Act1Room::Chase)
    {
        // ACT1-B: the chase's machine is room 3's DOZ-01 rig (no tunnel, no Guild door piece);
        // its room pieces, the timber and barricade bursts and the wedge are the pure half's.
        dozer_art::spawn(
            &mut commands,
            &asset_server,
            &mut layouts,
            layers_of("actors"),
            &[],
            None,
            kit,
            &mut combat_handles,
            &mut combat_entities,
        );
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
        // CP-B2: one more pair for the reserved thrown-charge slot (it is created
        // on the first throw, after the authored charges).
        let pairs = state.charges.len() + usize::from(presents_dynamite(args));
        for index in 0..pairs {
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
        // CP-Q42: the pierce spark (one pre-spawned sprite).
        pierce_spark::spawn(
            &mut commands,
            &asset_server,
            &mut combat_handles,
            layers_of("actors"),
            kit.actor_z,
        );
    }

    // CARL-GAUNTLET-01: one overlay sprite per Carl body sprite (hidden until the
    // run holds `fist_reinforced`); presentation only, nothing is read back.
    if let (Some(_), Some(art)) = (combat.0.as_ref(), actors.art.as_ref()) {
        let z = kit.actor_z + CARL_Z_OFFSET;
        let cell = |spec: &ActorSpec| UVec2::new(spec.width as u32, spec.height as u32);
        let mut bodies = vec![
            (
                carl_gauntlet::CarlGauntlet::Locomotion,
                actors.carl.atlas_asset.as_str(),
                cell(&actors.carl.spec),
                actors.carl.clips.frame_count,
                z,
            ),
            (
                carl_gauntlet::CarlGauntlet::Melee,
                art.carl.atlas_asset.as_str(),
                cell(&art.carl.spec),
                art.carl.frame_count,
                z,
            ),
        ];
        if let Some(clips) = art.dynamite.as_ref() {
            bodies.push((
                carl_gauntlet::CarlGauntlet::Throw,
                clips.atlas_asset.as_str(),
                cell(&clips.spec),
                clips.frame_count,
                z,
            ));
        }
        if let Some(acting) = actors.cf02.as_ref() {
            bodies.push((
                carl_gauntlet::CarlGauntlet::Cf02,
                acting.carl.atlas_asset.as_str(),
                cell(&acting.carl.spec),
                acting.carl.frame_count,
                z,
            ));
        }
        combat_entities += carl_gauntlet::spawn(
            &mut commands,
            &asset_server,
            &mut layouts,
            &mut combat_handles,
            &layers_of("actors"),
            &bodies,
            &setup.art_dir,
        );
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
        // Goblin A, goblin B, (with the llama art) the llama and (with the Engineer art) the
        // Engineer: one sheet each, at the presenter's variant indices.
        let variants = if presenter.has_engineer() {
            ENGINEER_VARIANT + 1
        } else if presenter.has_llama() {
            LLAMA_VARIANT + 1
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
    handles.extend(refuge_handles);
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
        held: pad_buttons(|button| pad.pressed(button)),
        pressed: pad_buttons(|button| pad.just_pressed(button)),
    }
}

/// The logical buttons of one pad from a physical-button query (held or
/// just-pressed). The physical mapping lives here only: South jump, West
/// PrimaryAttack, North PhysicalAttack (North no longer restarts), East
/// EquippedSpecial (Circle/B), LeftTrigger swap, Select restart, Start menu.
/// The same mapping serves every pad slot (P1 and P2).
fn pad_buttons(is: impl Fn(GamepadButton) -> bool) -> PadButtons {
    PadButtons {
        jump: is(GamepadButton::South),
        attack: is(GamepadButton::West),
        physical: is(GamepadButton::North),
        ability: is(GamepadButton::East),
        swap: is(GamepadButton::LeftTrigger),
        restart: is(GamepadButton::Select),
        start: is(GamepadButton::Start),
    }
}

/// The engine key code of a key the binding table names (`key_bindings`). The
/// table is the single source of the keyboard bindings: this is the only place a
/// `KeyName` meets a Bevy `KeyCode`.
const fn key_code(name: KeyName) -> KeyCode {
    match name {
        KeyName::KeyJ => KeyCode::KeyJ,
        KeyName::KeyK => KeyCode::KeyK,
        KeyName::KeyL => KeyCode::KeyL,
        KeyName::KeyQ => KeyCode::KeyQ,
        KeyName::Space => KeyCode::Space,
        KeyName::ArrowLeft => KeyCode::ArrowLeft,
        KeyName::ArrowRight => KeyCode::ArrowRight,
        KeyName::ArrowDown => KeyCode::ArrowDown,
        KeyName::KeyA => KeyCode::KeyA,
        KeyName::KeyD => KeyCode::KeyD,
        KeyName::KeyS => KeyCode::KeyS,
    }
}

/// Any key bound to `action` is held.
fn key_held(keys: &ButtonInput<KeyCode>, action: KeyAction) -> bool {
    key_bindings::keys_of(action)
        .iter()
        .any(|name| keys.pressed(key_code(*name)))
}

/// Any key bound to `action` went down this frame.
fn key_tapped(keys: &ButtonInput<KeyCode>, action: KeyAction) -> bool {
    key_bindings::keys_of(action)
        .iter()
        .any(|name| keys.just_pressed(key_code(*name)))
}

/// Latch the keyboard's just-pressed edges between fixed steps (a tap shorter
/// than a fixed tick is not lost; `fixed_step` consumes them). The keys come from
/// the binding table (`key_bindings`): Jump Space, Primary J, Physical K, the
/// equipped Special L (next to J and K, the keyboard's Circle/B) and the
/// temporary solo Swap Q (the contract only fixes L1/LB; the keyboard stays P1).
fn latch_keyboard_taps(keys: &ButtonInput<KeyCode>, controls: &mut Controls) {
    controls.jump_tap |= key_tapped(keys, KeyAction::Jump);
    controls.attack_tap |= key_tapped(keys, KeyAction::Primary);
    controls.physical_tap |= key_tapped(keys, KeyAction::Physical);
    controls.ability_tap |= key_tapped(keys, KeyAction::Special);
    controls.swap_tap |= key_tapped(keys, KeyAction::Swap);
}

/// P1's keyboard input for one fixed tick: held keys plus the latched taps,
/// which are consumed. Every binding is read from `key_bindings`.
fn keyboard_input(keys: &ButtonInput<KeyCode>, controls: &mut Controls) -> RawInput {
    let jump_tap = std::mem::take(&mut controls.jump_tap);
    let attack_tap = std::mem::take(&mut controls.attack_tap);
    let physical_tap = std::mem::take(&mut controls.physical_tap);
    let ability_tap = std::mem::take(&mut controls.ability_tap);
    let swap_tap = std::mem::take(&mut controls.swap_tap);
    let left = key_held(keys, KeyAction::MoveLeft);
    let right = key_held(keys, KeyAction::MoveRight);
    RawInput {
        move_x: f32::from(i8::from(right) - i8::from(left)),
        jump: key_held(keys, KeyAction::Jump) || jump_tap,
        attack: key_held(keys, KeyAction::Primary) || attack_tap,
        physical: key_held(keys, KeyAction::Physical) || physical_tap,
        ability: key_held(keys, KeyAction::Special) || ability_tap,
        swap: swap_tap,
        // CF02-Q01: Down (the Down arrow or S). Only Carl's Physical press
        // under `--cf02` reads it.
        down: key_held(keys, KeyAction::Down),
        ..RawInput::NONE
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
    latch_keyboard_taps(&keys, &mut controls);
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
    // Keyboard is P1. Pad edges are consumed here too, so a tap made during a
    // pause is dropped and nothing fires on resume.
    let keyboard = keyboard_input(&keys, &mut controls);
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
    if (setup.args.demo || setup.args.lab_scene || setup.args.f01_room_scene()) && setup.args.manual
    {
        restart = false;
        // F01-RUN: the room-clear panel owns the input, Select included.
        let panel_up =
            setup.args.f01_run && sinks.lab.as_ref().is_some_and(|lab| lab.blocks_gameplay());
        let down = !panel_up && (keys.pressed(KeyCode::KeyR) || controls.select_held || select_tap);
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
    } else if let (Some(fixture), Some(combat), true) = (
        fixture.0,
        combat.0.as_ref(),
        setup.args.room_pilot.is_some(),
    ) {
        // EVID-ROOM: both slots come from the room pilot (Donut's in a 2P run),
        // the same call as the presim, the capture sequence and the headless run.
        // PT-GATE-C1: an aligned pilot plays the hero the party controls.
        match setup.args.room_pilot.filter(|p| p.aligned.is_some()) {
            Some(pilot) => aligned_slots(&setup.args, pilot, combat, &sim.0, pads.0.party()),
            None => scripted_inputs(&setup.args, fixture, &sim.0, combat),
        }
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
    // EVID-ROOM: a two-player room pilot plays Donut as a real hero (the drive the
    // presim and the headless run set once), so the window must keep it.
    let live_party = (scenario.0.is_none() && fixture.0.is_none())
        || setup
            .args
            .room_pilot
            .is_some_and(|p| p.two_player || p.aligned.is_some());
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
        if setup.args.f01_room_scene() {
            // BOIL-A wire: nothing to announce yet (the exit is the room's own).
        } else if setup.args.cf02.rules {
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
        print_audience_state(&setup.args, &hud.0);
        if let Some(death) = death.0.as_ref() {
            println!("{}", death.state_line());
        }
        exit.write(AppExit::Success);
    }
}

/// FIX-TOOL: a clean exit on SIGINT or SIGTERM. The handler only sets a flag (an atomic
/// store, async-signal-safe); [`shutdown_on_signal`] turns the flag into an `AppExit` on the
/// next frame, so the window closes through the ordinary exit path and a PLAYTRACE session
/// is closed with its final chunk and trailer (`scope=full`) instead of being killed and
/// crash-recovered (`scope=prefix`).
mod shutdown_signal {
    use std::sync::atomic::{AtomicBool, Ordering};

    static REQUESTED: AtomicBool = AtomicBool::new(false);

    #[cfg(unix)]
    extern "C" fn on_signal(_signal: libc::c_int) {
        REQUESTED.store(true, Ordering::SeqCst);
    }

    /// Route SIGINT and SIGTERM to the flag, **once**: `SA_RESETHAND` (with `SA_RESTART`, like glibc `signal`) puts the default
    /// disposition back as the handler starts, so the first signal asks for a clean exit
    /// and a second one (a stuck frame loop, a slow shutdown, `timeout` without `-k`)
    /// kills the process as before. A no-op off Unix.
    pub fn install() {
        #[cfg(unix)]
        for signal in [libc::SIGINT, libc::SIGTERM] {
            // SAFETY: a zeroed `sigaction` is valid; the handler does one atomic store and
            // touches no other memory.
            unsafe {
                let mut action: libc::sigaction = std::mem::zeroed();
                action.sa_sigaction = on_signal as *const () as libc::sighandler_t;
                action.sa_flags = libc::SA_RESETHAND | libc::SA_RESTART;
                libc::sigemptyset(&mut action.sa_mask);
                libc::sigaction(signal, &action, std::ptr::null_mut());
            }
        }
    }

    /// Back to the default disposition (tests; the handler resets itself when it fires).
    #[cfg(test)]
    pub fn restore_default() {
        #[cfg(unix)]
        for signal in [libc::SIGINT, libc::SIGTERM] {
            // SAFETY: restores the default disposition.
            unsafe {
                libc::signal(signal, libc::SIG_DFL);
            }
        }
        REQUESTED.store(false, Ordering::SeqCst);
    }

    /// Whether a signal arrived since the last call.
    pub fn take() -> bool {
        REQUESTED.swap(false, Ordering::SeqCst)
    }
}

/// FIX-TOOL: end the window on SIGINT or SIGTERM (see [`shutdown_signal`]).
fn shutdown_on_signal(mut exit: MessageWriter<AppExit>) {
    if shutdown_signal::take() {
        println!("FLOOR1_SIGNAL_EXIT the window ends on a signal");
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

/// FA-Q38 (i-frame cue): the colour a hero's sprite is drawn with, for both heroes
/// and every sprite that shows them (locomotion, melee, cast, throw, acting): white,
/// dimmed to alpha 0.5 on alternate 4 tick phases of the simulation's post-hit
/// invulnerability, and never while that hero is downed (the other hero keeps flickering). No tint. Presentation only.
fn hero_cue(sim: &SceneSim, combat: Option<&CombatState>, hero: HeroId) -> Color {
    let downed = combat.is_some_and(|c| c.hero_downed(hero));
    let ticks = sim.world.body(hero).invulnerable_ticks;
    Color::srgba(1.0, 1.0, 1.0, invulnerability_alpha(ticks, downed))
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
        sprite.color = if reference.0.is_some() {
            Color::WHITE
        } else {
            let id = if actor.is_hero {
                HeroId::Carl
            } else {
                HeroId::Donut
            };
            hero_cue(state, combat.0.as_ref(), id)
        };
    }

    let hero = &state.world.hero;
    for mut transform in &mut hero_box {
        transform.translation.x = hero.x.round();
        transform.translation.y = hero.y.round();
    }
}

/// AREA-REFUGE-01: Mordecai on his mark, standing on the walking line of his column, mirrored
/// about his pivot when he faces left (he is authored facing right).
fn sync_mordecai(
    setup: Res<Setup>,
    mordecai: Res<MordecaiRes>,
    mut sprites: Query<(&mut Transform, &mut Sprite), With<MordecaiVisual>>,
) {
    let Some((spec, m)) = mordecai.0.as_ref() else {
        return;
    };
    let kit = &setup.kit;
    let line = kit.walking_line_at(m.x).unwrap_or(kit.base_walk_line_y);
    let flip = m.facing == refuge::Facing::Left;
    let (cx, cy) = actor_center(*spec, m.x as f32, to_world_y(kit.height, line), flip);
    for (mut transform, mut sprite) in &mut sprites {
        transform.translation.x = cx;
        transform.translation.y = cy;
        sprite.flip_x = flip;
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
                sprite.color = hero_cue(&sim.0, Some(state), HeroId::Carl);
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
            // Donut's own boxes belong to `sync_donut_visuals`, the ring to
            // `sync_blast_ring`.
            CombatDebugBox::DonutHit | CombatDebugBox::Bolt(_) | CombatDebugBox::BlastRing(_) => {}
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

    // Her damage reaction (hurt, thrown, knocked down) wins over everything: a hit
    // ends her move and the reaction frames are the melee atlas's own.
    let hurt_frame = anim
        .0
        .art
        .as_ref()
        .and_then(|art| art.donut_hurt_frame())
        .filter(|_| party);
    let melee_frame = hurt_frame.or_else(|| {
        state
            .donut_move_view()
            .filter(|_| party)
            .map(|view| donut_art::melee_frame(&view))
    });
    // (The blink's windup frames are drawn by `blink_cues::sync_blink_cues` from the moves atlas.)
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
        // The thrown frame settles onto the body by the apex, like Carl's knockdown.
        let blend = dungeon_floorbreak::combat_anim::knockdown_blend(sim.world.donut_body.vy);
        let foot = donut.foot_y
            - donut_art::melee_foot_drop(frame)
            - donut_art::knockback_foot_drop(frame, blend);
        let (cx, cy) = actor_center(spec, donut.x, foot, flip);
        transform.translation.x = cx;
        transform.translation.y = cy;
        sprite.flip_x = flip;
        sprite.color = hero_cue(sim, Some(state), HeroId::Donut);
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
        sprite.color = hero_cue(sim, Some(state), HeroId::Donut);
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
    danger: Option<Res<danger_fx_art::DangerFxArt>>,
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
                // The F01 danger FX gate art replaces the slab when its atlas loaded.
                boss.gate_closed() && !danger.as_ref().is_some_and(|d| d.draws_gate(boss))
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
    rush_look: Option<Res<LlamaRushLook>>,
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
        // LLAMA-03: during a corral rush the llama draws the rush atlas (the bonk's first ticks, the LLAMA-01
        // hurt frame); without the rush atlas the CORRAL-E stand-in body (the idle frames) stays.
        let rush_frame = rush_look
            .as_ref()
            .and_then(|_| llama_rush_pose::llama_rush_frame(llama));
        // CORRAL-E (CRL-Q38): the rush stand-in's brace, charge and bonk lean (LLAMA-03: only the bonk's, the
        // art's poses carry the brace and the charge).
        let cx = cx
            + dungeon_floorbreak::combat_anim::rush_stand_in::rush_stand_in_art(
                llama,
                CombatAnimators::llama_foot_y(llama),
                rush_look.is_some(),
            )
            .map_or(0.0, |s| s.lean_px as f32);
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
        // Which atlas and frame: a rush-atlas frame, or the LLAMA-01 atlas with the hurt frame (bonk) or the
        // animator's own pose.
        let (use_rush, frame) = match rush_frame {
            Some(llama_rush_pose::RushFrame::Rush(i)) => (true, i),
            Some(llama_rush_pose::RushFrame::Base(i)) => (false, i),
            None => (false, pose.frame),
        };
        if let Some(look) = rush_look.as_ref() {
            let (image, layout) = if use_rush {
                (&look.rush_image, &look.rush_layout)
            } else {
                (&look.base_image, &look.base_layout)
            };
            if sprite.image != *image {
                sprite.image = image.clone();
            }
            if let Some(atlas) = sprite.texture_atlas.as_mut()
                && atlas.layout != *layout
            {
                atlas.layout = layout.clone();
            }
        }
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = frame;
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

/// ACT1-A: the guild door sprite (a solid greybox rectangle while Donut's mana is locked,
/// hidden once it unlocks). Presentation only.
#[derive(Component)]
struct Act1DoorSprite;

/// ACT1-B: the chase's barricade sprite (a solid greybox rectangle until the machine's front
/// reaches it, then hidden). Presentation only.
#[derive(Component)]
struct Act1BarricadeSprite;

/// ACT1-B: a timber decor rectangle of the chase, hidden once the front has smashed it
/// (`TimberSmashed`). Presentation only.
#[derive(Component)]
struct Act1TimberSprite(usize);

/// The colour of an Act I decor rectangle by its label (flat, no art).
fn act1_decor_colour(label: &str) -> Color {
    match label {
        // The machine wedged in the arch: dark, it is not lethal and not in the sim.
        "wedged_dozer" => Color::srgb(0.28, 0.24, 0.22),
        // The guide: a plain muted figure (ACT1-AD11: the sprite is the allowed upgrade).
        "guide" => Color::srgb(0.62, 0.52, 0.34),
        // The chase (ACT1-B): the lowered soffit, the timber sets, the window, the arch and the lit
        // alcove, the brightest shape of the S4 lane (spotting it is the player's decision).
        "soffit" => Color::srgb(0.20, 0.17, 0.18),
        "timber_a" | "timber_b" => Color::srgb(0.46, 0.30, 0.16),
        "window" => Color::srgb(0.30, 0.22, 0.16),
        "arch" => Color::srgb(0.13, 0.12, 0.14),
        "alcove" => Color::srgb(1.0, 0.88, 0.50),
        _ => Color::srgb(0.58, 0.72, 0.80),
    }
}

/// ACT1-A: the decor table of an Act I room as flat rectangles behind the actors, and the
/// guild door in front of the lane while closed. Never a solid, never read by the
/// simulation (the door's solid is the env runtime's).
fn spawn_act1_greybox(
    commands: &mut Commands,
    run: &f01_act1::Act1Run,
    kit: &EnvKit,
    height: i32,
    layers: RenderLayers,
    guide_sprite: bool,
) {
    for decor in f01_act1::decor(run.room) {
        let rect = decor.rect;
        println!(
            "F01_ACT1_DECOR room={} label={} rect={},{},{},{}",
            run.room.id(),
            decor.label,
            rect.x,
            rect.y,
            rect.w,
            rect.h
        );
        // The guide is Mordecai's sprite when it is loaded; the rectangle is the fallback.
        if guide_sprite && decor.label == "guide" {
            continue;
        }
        let mut entity = commands.spawn((
            Sprite::from_color(
                act1_decor_colour(decor.label),
                Vec2::new(rect.w as f32, rect.h as f32),
            ),
            Anchor::TOP_LEFT,
            Transform::from_xyz(rect.x as f32, to_world_y(height, rect.y), kit.actor_z - 0.2),
            layers.clone(),
        ));
        if let Some(id) = decor
            .label
            .strip_prefix("timber_")
            .and_then(|k| k.bytes().next())
            .map(|b| usize::from(b - b'a'))
        {
            entity.insert(Act1TimberSprite(id));
        }
    }
    if let Some(barricade) = run.barricade() {
        commands.spawn((
            Act1BarricadeSprite,
            Sprite::from_color(
                Color::srgb(0.40, 0.26, 0.14),
                Vec2::new(barricade.w as f32, barricade.h as f32),
            ),
            Anchor::TOP_LEFT,
            Transform::from_xyz(
                barricade.x as f32,
                to_world_y(height, barricade.y),
                kit.actor_z - 0.1,
            ),
            layers.clone(),
        ));
    }
    if let Some(door) = run.door() {
        commands.spawn((
            Act1DoorSprite,
            Sprite::from_color(
                Color::srgb(0.46, 0.30, 0.20),
                Vec2::new(door.w as f32, door.h as f32),
            ),
            Anchor::TOP_LEFT,
            Transform::from_xyz(door.x as f32, to_world_y(height, door.y), kit.actor_z - 0.1),
            layers,
        ));
    }
}

/// ACT1-B: the barricade disappears when the machine's front reaches it and a smashed timber set
/// disappears with the front's passing (both read the machine, one tick behind the solid).
fn sync_act1_chase(
    combat: Res<Combat>,
    mut barricades: Query<&mut Visibility, (With<Act1BarricadeSprite>, Without<Act1TimberSprite>)>,
    mut timbers: Query<(&Act1TimberSprite, &mut Visibility), Without<Act1BarricadeSprite>>,
    mut wedge_logged: Local<bool>,
) {
    let Some(d) = combat
        .0
        .as_ref()
        .and_then(dungeon_floorbreak::dozer_art::dozer_of)
    else {
        return;
    };
    // The wedge is a labelled greybox (ACT1-Q3: the end-on art is a later work order).
    if d.wedged() && !*wedge_logged {
        *wedge_logged = true;
        println!(
            "F01_ACT1_WEDGE label=\"GREYBOX WEDGE\" clip_right={}",
            d.site.stop_x
        );
    } else if !d.wedged() {
        *wedge_logged = false;
    }
    for mut visibility in &mut barricades {
        *visibility = if d.barrier_hit() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for (timber, mut visibility) in &mut timbers {
        *visibility = if d.timber_smashed(timber.0) {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

/// ACT1-A: the guild door disappears when Donut's mana unlocks (the env solid follows the same
/// state one tick later).
fn sync_act1_door(combat: Res<Combat>, mut doors: Query<&mut Visibility, With<Act1DoorSprite>>) {
    let Some(state) = combat.0.as_ref() else {
        return;
    };
    let open = state.donut_unlocked();
    for mut visibility in &mut doors {
        *visibility = if open {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
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

/// CP-B2 (CP-Q33): the `--debug-combat` blast ring around the thrown charge while
/// it is live, at its profile's outer radius: the real reach, wider than the
/// burst art. Reads the simulation only.
fn sync_blast_ring(
    combat: Res<Combat>,
    mut points: Query<(&CombatDebugBox, &mut Transform, &mut Visibility)>,
) {
    let Some(state) = combat.0.as_ref() else {
        return;
    };
    let live = state.charges.get(state.dynamite_slot()).filter(|c| {
        !c.detonated && c.profile.outer_radius > 0.0 && state.thrower().stick().is_some()
    });
    for (kind, mut transform, mut visibility) in &mut points {
        let CombatDebugBox::BlastRing(point) = kind else {
            continue;
        };
        match live {
            Some(charge) => {
                let (cx, cy) = charge.centre();
                let ring = ring_points(charge.profile.outer_radius, BLAST_RING_POINTS);
                let (dx, dy) = ring[*point];
                transform.translation.x = cx.round() + dx as f32;
                transform.translation.y = cy.round() + dy as f32;
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// CP-B2: draw Carl's dynamite throw (ANIM-CARL-03) over his locomotion sprite:
/// the standing frames, or the moving overlay frames while he runs. Where a throw
/// frame is drawn the locomotion sprite is hidden; a moving draw, brace or settle
/// (no throw frame) leaves the plain run frame. Reads the animators only.
#[allow(clippy::type_complexity)]
fn sync_carl_throw_visual(
    sim: Res<Sim>,
    combat: Res<Combat>,
    anim: Res<Anim>,
    mut throw: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (With<CarlThrowSprite>, Without<ActorVisual>),
    >,
    mut actors: Query<(&ActorVisual, &mut Visibility), Without<CarlThrowSprite>>,
) {
    let Some(art) = anim.0.art.as_ref() else {
        return;
    };
    let Some(clips) = art.dynamite_clips() else {
        return;
    };
    let hero = &sim.0.world.hero;
    let pose = art.carl_throw_pose();
    for (mut transform, mut sprite, mut visibility) in &mut throw {
        match pose {
            Some(pose) => {
                let flip = hero.facing < 0;
                let (cx, cy) = actor_center(clips.spec, hero.x, hero.y - HERO_HALF_HEIGHT, flip);
                transform.translation.x = cx;
                transform.translation.y = cy;
                sprite.flip_x = flip;
                sprite.color = hero_cue(&sim.0, combat.0.as_ref(), HeroId::Carl);
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = pose.frame;
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    if pose.is_some() {
        for (actor, mut visibility) in &mut actors {
            if actor.is_hero {
                *visibility = Visibility::Hidden;
            }
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
        } else if charge.detonated || state.env_owns_charge(index) {
            // A prop's blast (the Boiler's drum) draws no stick: the prop is
            // its own art (BOIL-E); its burst is the one above.
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
            // CP-B2: the thrown stick tumbles in flight (`dynamite_flight`, from the
            // release) and lies burning once it landed (`dynamite_fizzle`).
            let frame = match state.thrower().stick() {
                Some(stick) if index == state.dynamite_slot() => {
                    let fuse = u64::from(CarlProgTuning::active().cpq18_fuse_ticks);
                    let released = stick.detonate_tick.saturating_sub(fuse);
                    stick_frame(
                        stick.landed,
                        if stick.landed {
                            tick
                        } else {
                            tick.saturating_sub(released)
                        },
                    )
                }
                _ => charge_draw_frame(charge.lit(), lit_for),
            };
            sprite.rect = Some(to_rect(art.dynamite_rect(frame)));
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
                sprite.color = hero_cue(&sim.0, Some(state), HeroId::Carl);
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
                sprite.color = hero_cue(&sim.0, Some(state), HeroId::Donut);
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
        let Some(g) = state.enemies.get(index) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        // ENG-02: an Engineer draws his own slammed sheet (the belt, no pot: the pot is a launched part).
        let (pose, slammed_clips) = if g.is_engineer() {
            (
                art.engineer_slammed_pose(index, slam_of(index)),
                &cf02.art.engineer_slammed,
            )
        } else {
            (
                art.goblin_slammed_pose(index, slam_of(index)),
                &cf02.art.slammed[goblin_variant(index)],
            )
        };
        let Some(pose) = pose else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let flip = g.facing < 0;
        let (cx, cy) = actor_center(
            slammed_clips.spec,
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
    // DR-C: a rider carried by the Dozer is parked off the view and not drawn.
    let hide_goblin = |index: usize| {
        presenter.pose_of(index as u32).is_some()
            || !presenter.draws_goblin(index as u32)
            || state.is_carried(index)
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
    // LLAMA-DEATH-WIRE: an exploded llama (presets 2 to 4) is replaced by its
    // pieces, as the goblin.
    let goblin_count = state.enemies.len();
    for (llama, mut visibility, mut sprite) in &mut llamas {
        let target = (goblin_count + llama.index) as u32;
        *visibility = if presenter.pose_of(target).is_some() || !presenter.draws_goblin(target) {
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
    mordecai: Res<MordecaiRes>,
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
            // F01-FU-B10 (#259): Donut's lying frames (`d0`, `d1`) do the same
            // (her reaction frames are drawn only in a live party, as in
            // `sync_donut_visuals`).
            if !pose.is_hero
                && reference.0.is_none()
                && state.party_live()
                && let Some((width, pivot)) = anim
                    .0
                    .art
                    .as_ref()
                    .and_then(|art| art.donut_hurt_frame())
                    .and_then(donut_art::hurt_shadow_footprint)
            {
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
        // AREA-REFUGE-01: Mordecai's contact shadow on the walking line of his column.
        if let Some((spec, m)) = mordecai.0.as_ref() {
            let line = kit.walking_line_at(m.x).unwrap_or(kit.base_walk_line_y);
            shadows.push(contact_shadow(
                profile,
                shadow_center_x(*spec, m.x as f32, m.facing == refuge::Facing::Left),
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
                // No shadow under a goblin whose body is hidden (an explosion),
                // retired or carried by the Dozer (DR-C): nothing is drawn
                // above it.
                if combat.is_carried(index)
                    || death
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
                let goblin_count = combat.enemies.len();
                for (index, llama) in combat.llamas.iter().enumerate() {
                    // No shadow under an exploded or retired llama either.
                    if death.0.as_ref().is_some_and(|presenter| {
                        !presenter.draws_goblin((goblin_count + index) as u32)
                    }) {
                        continue;
                    }
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
    p99_ms: f32,
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
        p99_ms: rank(0.99),
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
    mut hud: ResMut<HudState>,
    mut pads: ResMut<Pads>,
) {
    let Some(mut perf) = perf else { return };
    if !perf.step_ticks || loading.is_some_and(|l| !l.reported) || perf.seen < perf.warmup {
        return;
    }
    let (Some(fixture), Some(combat)) = (setup.args.fixture, combat.0.as_mut()) else {
        return;
    };
    let started = std::time::Instant::now();
    // PT-GATE-A: a room scene is driven by its room pilot (solo, slot 0 only, as the
    // headless run and the presim do), with the HUD stepped as in the live loop.
    let aligned = setup.args.room_pilot.filter(|p| p.aligned.is_some());
    let inputs = if let Some(pilot) = aligned {
        // PT-GATE-C1: an aligned room pilot plays through the live party.
        aligned_routed(&setup.args, pilot, combat, &mut sim.0, pads.0.party_mut())
    } else if setup.args.room_pilot.is_some() {
        scripted_inputs(&setup.args, fixture, &sim.0, combat)
    } else {
        [
            scripted_input(&setup.args, fixture, &sim.0, combat),
            RawInput::default(),
        ]
    };
    let restarts = combat.restarts;
    let _ = step_combat(&mut sim.0, combat, &mut anim.0, death.0.as_mut(), inputs);
    if setup.args.room_pilot.is_some() {
        if combat.restarts != restarts {
            hud.0.restart();
            if aligned.is_some() {
                pads.0.party_mut().restart();
            }
        }
        step_hud(&mut hud.0, &sim.0, Some(&*combat));
    }
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
                "FLOOR1_PERF frames={} median_ms={:.2} p95_ms={:.2} p99_ms={:.2} mean_ms={:.2} max_ms={:.2} lighting={} window={size} present=AutoNoVsync warmup_frames={}",
                stats.frames,
                stats.median_ms,
                stats.p95_ms,
                stats.p99_ms,
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
/// Frames to wait before the **first** screenshot of a sequence. Textures are
/// loaded when the loading report comes out, but the scene is not drawn yet: with
/// the usual 4-frame settle the first screenshot came out near-black (the HUD text
/// over a dark frame, 102 colours) in five of six xvfb runs and fully rendered in
/// the sixth (5252 colours), so the palette a GIF samples from frame 0, and with it
/// every byte of the GIF, changed from run to run. Measured: 10, 30 and 60 frames
/// all gave the rendered frame in 6 of 6 runs; 60 leaves a margin (about 1 s).
const CAPTURE_FIRST_SETTLE_FRAMES: u32 = 60;
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
    mut hud: ResMut<HudState>,
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
            // PT-GATE-A: the first screenshot waits for the first real render (see
            // `CAPTURE_FIRST_SETTLE_FRAMES`); later ones settle for a few frames.
            CapturePhase::Settle(CAPTURE_FIRST_SETTLE_FRAMES)
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
                } else if let (Some(pilot), Some(combat)) = (
                    setup.args.room_pilot.filter(|p| p.aligned.is_some()),
                    combat.0.as_mut(),
                ) {
                    // PT-GATE-C1: an aligned room pilot plays through the live
                    // party, like the presim of `build_scene`.
                    let party = pads.0.party_mut();
                    let inputs = aligned_routed(&setup.args, pilot, combat, &mut sim.0, party);
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
                // One HUD step per simulated tick, as in the live loop (DP-D: the
                // captured HUD no longer stays on the state of the first frame, so
                // the socket, its flash and the notices are real in the evidence).
                step_hud(&mut hud.0, &sim.0, combat.0.as_ref());
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
    if let Some(run) = &args.boiler_run {
        return headless_boiler(kit, args, actors, root, run);
    }
    if args.hoarder_run.is_some()
        || args.hazards_run.is_some()
        || args.mastery_run.is_some()
        || args.ring_run.is_some()
        || args.rest_run.is_some()
        || args.act1_run.is_some()
        || args.corral_run.is_some()
        || args.dozer_run.is_some()
    {
        return headless_slot_room(kit, args, actors, root);
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
    print_audience_state(args, &hud);
    Ok(())
}

/// Headless fixture replay (and the demo): the scripted run for
/// `--capture-tick` ticks (default the fixture's own length), every event,
/// then the state lines.
/// BOIL-A wire: the Boiler room's smoke. The fixture's scripted walk runs on the
/// real scene and combat state under `cf02_rules`; the run must reach the
/// protected exit through the checkpoint, which prints `F01_BOILER_SMOKE_OK`.
fn headless_boiler(
    kit: &EnvKit,
    args: &Args,
    actors: &Actors,
    root: &Path,
    run: &f01_boiler::BoilerRun,
) -> Result<(), String> {
    println!(
        "FLOOR1_HEADLESS_OK: boiler hall {} tiles, {} pieces, {} merged solids",
        kit.tiles.len(),
        kit.pieces.len(),
        kit.solid_rects.len()
    );
    let mut args = args.clone();
    args.capture_tick.get_or_insert(run.fixture.ticks);
    let mut hud = new_hud(&args, kit, actors);
    let (sim, anim, combat, _death, party) = build_scene(kit, &args, actors, root, &mut hud)?;
    let mut combat = combat.ok_or("the Boiler fixture has no combat state")?;
    print_combat_events(&mut combat, None, None);
    println!("{}", sim.state_line());
    println!("{}", anim.state_line(sim.world.tick));
    print_combat_state(&sim, &anim, &combat);
    // PT-GATE-E: an aligned `boiler:` pilot prints the library fixture's hash.
    print_room_pilot_state(&args, &combat, &sim, &party);
    if run.roster == f01_boiler::Roster::Live {
        // BOIL-E: the scripted walk is no fighter, so it need not reach the exit
        // through live goblins. The smoke proves the roster loaded and the run is
        // deterministic (the CI line replays it twice).
        let engineers = combat.enemies.iter().filter(|g| g.is_engineer()).count();
        println!(
            "F01_BOILER_SMOKE_OK ticks={} checkpoint={} complete={} pin={} env=live roster_inert=false \
             enemies={} engineers={engineers}",
            sim.world.tick,
            combat.checkpoint,
            sim.world.complete,
            run.loaded.encounter.story_pin,
            combat.enemies.len(),
        );
        return Ok(());
    }
    if !sim.world.complete {
        return Err(format!(
            "the scripted walk did not reach the protected exit in {} ticks",
            sim.world.tick
        ));
    }
    println!(
        "F01_BOILER_SMOKE_OK ticks={} checkpoint={} complete=true pin={} env=live roster_inert=true",
        sim.world.tick, combat.checkpoint, run.loaded.encounter.story_pin
    );
    Ok(())
}

/// HRD-A2b and HAZ-A2b: the smoke of the Hoarder's room and the hazard room. The
/// scene is built through the library constructor (the verifier's) and run for
/// the room's smoke ticks on the real scene and combat state under `cf02_rules`;
/// it prints `F01_HOARDER_SMOKE_OK` or `F01_HAZARDS_SMOKE_OK` with the pin, the
/// slot roster and the state hash, and the CI line replays it twice. Neither
/// scripted walk is a fighter, so neither smoke claims the room is cleared.
fn headless_slot_room(
    kit: &EnvKit,
    args: &Args,
    actors: &Actors,
    root: &Path,
) -> Result<(), String> {
    let (label, marker, pin, ticks) = if let Some(run) = &args.hoarder_run {
        (
            "hoarder",
            "F01_HOARDER_SMOKE_OK",
            run.loaded.encounter.story_pin.clone(),
            f01_hoarder::SMOKE_TICKS,
        )
    } else if let Some(run) = &args.hazards_run {
        (
            "hazards",
            "F01_HAZARDS_SMOKE_OK",
            run.loaded.encounter.story_pin.clone(),
            f01_hazards::SMOKE_TICKS,
        )
    } else if let Some(run) = &args.mastery_run {
        (
            "mastery",
            "F01_MASTERY_SMOKE_OK",
            run.loaded.encounter.story_pin.clone(),
            f01_mastery::SMOKE_TICKS,
        )
    } else if let Some(run) = &args.ring_run {
        (
            run.room.id(),
            match run.room {
                f01_ring::RingRoom::Court => "F01_RING_COURT_SMOKE_OK",
                f01_ring::RingRoom::Split => "F01_RING_SPLIT_SMOKE_OK",
            },
            run.loaded.encounter.story_pin.clone(),
            f01_ring::SMOKE_TICKS,
        )
    } else if let Some(run) = &args.rest_run {
        (
            run.room.id(),
            match run.room {
                f01_rest::RestRoom::Refuge => "F01_REST_REFUGE_SMOKE_OK",
                f01_rest::RestRoom::Camp => "F01_REST_CAMP_SMOKE_OK",
                f01_rest::RestRoom::Aftermath => "F01_REST_AFTERMATH_SMOKE_OK",
            },
            run.loaded.encounter.story_pin.clone(),
            f01_rest::SMOKE_TICKS,
        )
    } else if let Some(run) = &args.act1_run {
        (
            run.room.id(),
            match run.room {
                f01_act1::Act1Room::Chase => "F01_ACT1_CHASE_SMOKE_OK",
                f01_act1::Act1Room::Shore => "F01_ACT1_SHORE_SMOKE_OK",
                f01_act1::Act1Room::Guild => "F01_ACT1_GUILD_SMOKE_OK",
            },
            run.loaded.encounter.story_pin.clone(),
            f01_act1::SMOKE_TICKS,
        )
    } else if let Some(run) = &args.corral_run {
        (
            f01_corral::ROOM_ID,
            "F01_CORRAL_SMOKE_OK",
            run.loaded.encounter.story_pin.clone(),
            f01_corral::SMOKE_TICKS,
        )
    } else if let Some(run) = &args.dozer_run {
        (
            "dozer",
            "F01_DOZER_SMOKE_OK",
            run.loaded.encounter.story_pin.clone(),
            f01_dozer::SMOKE_TICKS,
        )
    } else {
        return Err("no room".into());
    };
    println!(
        "FLOOR1_HEADLESS_OK: {label} room {} tiles, {} pieces, {} merged solids",
        kit.tiles.len(),
        kit.pieces.len(),
        kit.solid_rects.len()
    );
    let mut args = args.clone();
    args.capture_tick.get_or_insert(ticks);
    let mut hud = new_hud(&args, kit, actors);
    let (sim, anim, combat, _death, party) = build_scene(kit, &args, actors, root, &mut hud)?;
    let mut combat = combat.ok_or("the room fixture has no combat state")?;
    print_combat_events(&mut combat, None, None);
    println!("{}", sim.state_line());
    println!("{}", anim.state_line(sim.world.tick));
    print_combat_state(&sim, &anim, &combat);
    print_room_pilot_state(&args, &combat, &sim, &party);
    println!(
        "{marker} ticks={} checkpoint={} pin={pin} env=live slot_enemies={} enemies={} state_hash={:#018x}",
        sim.world.tick,
        combat.checkpoint,
        combat.slot_enemies().len(),
        combat.enemies.len(),
        combat.state_hash(&sim),
    );
    // DR-A: room 3's nav rebuild count (the AI partner's hazard feed changes: each one rebuilds its
    // navigation around the machine) and the machine's state, for the frame-time evidence.
    if (args.dozer_run.is_some()
        || args
            .act1_run
            .as_ref()
            .is_some_and(|r| r.room == f01_act1::Act1Room::Chase))
        && let Some(d) = dungeon_floorbreak::dozer_art::dozer_of(&combat)
    {
        let stats = combat.companion_stats();
        println!(
            "F01_DOZER_NAV ticks={} state={} nav_rebuilds={} hazard_feed_errors={} wheel_hits={} rescues={:?}",
            sim.world.tick,
            d.state.name(),
            stats.hazard_feed_changes,
            stats.hazard_feed_errors,
            d.stats.wheel_hits,
            d.stats.rescues,
        );
    }
    Ok(())
}

/// PT-GATE-C1: the final state of an aligned room pilot, one line. The hash is
/// the library fixture's (`party_hash`: the combat hash and the party's swap
/// timers), so a scene run and a library run compare directly.
fn print_room_pilot_state(args: &Args, combat: &CombatState, sim: &SceneSim, party: &PartyState) {
    if let Some(pilot) = args.room_pilot
        && let Some(aligned) = pilot.aligned
    {
        println!(
            "ROOM_PILOT_STATE pilot={} who={} seed={} tick={} controlled={:?} deaths={} hash={:016x}",
            pilot.label(),
            aligned.who.token(),
            aligned.seed,
            sim.world.tick,
            party.controlled_hero(),
            combat.deaths,
            party_scene::party_hash(combat, sim, party)
        );
    }
}

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
        print_room_pilot_state(&args, &combat, &sim, &party);
        print_audience_state(&args, &hud);
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
    print_audience_state(args, &hud);
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
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
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

    /// AUDIENCE-RUN (AR-T21, AR-AD6, AR-AD7): the broadcast and the showcase feats
    /// are on by default only for a live floor run. Every argument set the
    /// evidence scripts pass with `--f01-run` (the combat CI's corral entry, the
    /// heat, ring and Hoarder-cue evidence, the owner commands) keeps both off
    /// except the owner commands, which are live play; an explicit flag wins.
    #[test]
    fn broadcast_defaults_are_on_only_for_a_live_f01_run() {
        let both = |a: &Args| (a.audience_enabled(), a.cine_tuning().showcase_feats);
        // On: the owner's command, with or without a room, scale, loadout or exit.
        for live in [
            &["--scene", "f01_late", "--f01-run"][..],
            &["--scene", "f01_late", "--scale", "2", "--f01-run"],
            &["--scene", "f01_late", "--f01-run", "--f01-loadout", "all"],
            &[
                "--scene",
                "f01_late",
                "--f01-run",
                "--f01-room",
                "ring_court",
            ],
            &["--scene", "f01_late", "--f01-run", "--f01-room", "hoarder"],
            &[
                "--scene",
                "f01_late",
                "--f01-run",
                "--exit-after-ticks",
                "30",
            ],
        ] {
            let parsed = args(live).unwrap_or_else(|e| panic!("{live:?}: {e}"));
            assert_eq!(both(&parsed), (true, true), "{live:?}");
        }
        // Off: everything scripted, headless, captured, piloted or opted out.
        for off in [
            // scripts/ci_floor01_combat.sh (the corral entry)
            &[
                "--headless-floor01-smoke",
                "--scene",
                "f01_late",
                "--f01-run",
            ][..],
            // scripts/make_heat01_evidence.sh
            &[
                "--no-audio",
                "--no-record",
                "--scale",
                "1",
                "--scene",
                "f01_late",
                "--f01-run",
                "--capture-sequence",
                "60:30:2",
                "--capture-dir",
                "x",
            ],
            &[
                "--headless-floor01-smoke",
                "--scene",
                "f01_late",
                "--f01-run",
                "--capture-tick",
                "90",
            ],
            // scripts/make_rings01_evidence.sh
            &[
                "--scale",
                "1",
                "--scene",
                "f01_late",
                "--f01-run",
                "--f01-room",
                "ring_court",
                "--ring-pilot",
            ],
            &[
                "--headless-floor01-smoke",
                "--scene",
                "f01_late",
                "--f01-run",
                "--f01-room",
                "ring_split",
                "--ring-pilot",
            ],
            // scripts/make_hrd_cues_evidence.sh (scripts/room_capture.sh with the floor plan)
            &[
                "--f01-run",
                "--scale",
                "1",
                "--scene",
                "f01_late",
                "--f01-room",
                "hoarder",
                "--room-pilot",
                "hoarder:carl_solo:0",
                "--capture-sequence",
                "60:1:2",
                "--capture-dir",
                "x",
            ],
            &[
                "--headless-floor01-smoke",
                "--f01-run",
                "--scene",
                "f01_late",
                "--f01-room",
                "hoarder",
                "--room-pilot",
                "hoarder:carl_solo:0",
                "--capture-tick",
                "120",
            ],
            // the explicit opt-outs persist
            &[
                "--scene",
                "f01_late",
                "--f01-run",
                "--audience=off",
                "--cine-showcase=off",
            ],
        ] {
            let parsed = args(off).unwrap_or_else(|e| panic!("{off:?}: {e}"));
            assert_eq!(both(&parsed), (false, false), "{off:?}");
        }
        // Each opt-out works alone, and an explicit flag beats the default.
        let a = args(&["--scene", "f01_late", "--f01-run", "--audience=off"]).unwrap();
        assert_eq!(both(&a), (false, true));
        let a = args(&["--scene", "f01_late", "--f01-run", "--cine-showcase=off"]).unwrap();
        assert_eq!(both(&a), (true, false));
        let a = args(&["--scene", "f01_late", "--f01-run", "--audience"]).unwrap();
        assert_eq!(a.audience, Some(true));
        // Not a floor run: both default off, and showcase may be asked for anywhere.
        assert_eq!(both(&args(&[]).unwrap()), (false, false));
        assert_eq!(
            both(&args(&["--scene", "f01_late"]).unwrap()),
            (false, false)
        );
        let a = args(&[
            "--headless-floor01-smoke",
            "--party-fixture",
            "carl_prog_dyn_llama_pack",
            "--cine-showcase=on",
        ])
        .unwrap();
        assert_eq!(both(&a), (false, true));
        // Parse errors: the broadcast belongs to a floor run (AR-AD6).
        for bad in [
            &["--audience=on"][..],
            &["--audience"],
            &["--scene", "f01_late", "--audience=on"],
            &["--f01-audience", "1400:1900:600:800"],
            &[
                "--scene",
                "f01_late",
                "--f01-room",
                "boiler",
                "--f01-audience",
                "1400:1900:600:800",
            ],
            // the corral never holds a carry
            &[
                "--scene",
                "f01_late",
                "--f01-run",
                "--f01-audience",
                "1400:1900:600:800",
            ],
            &[
                "--scene",
                "f01_late",
                "--f01-run",
                "--f01-room",
                "corral",
                "--f01-audience",
                "300:300:300:300",
            ],
            // a malformed carry, a missing value, a bad switch
            &[
                "--scene",
                "f01_late",
                "--f01-run",
                "--f01-room",
                "boiler",
                "--f01-audience",
                "100:100:300:300",
            ],
            &[
                "--scene",
                "f01_late",
                "--f01-run",
                "--f01-room",
                "boiler",
                "--f01-audience",
                "1:2:3",
            ],
            &[
                "--scene",
                "f01_late",
                "--f01-run",
                "--f01-room",
                "boiler",
                "--f01-audience",
            ],
            &["--scene", "f01_late", "--f01-run", "--audience=maybe"],
            &["--scene", "f01_late", "--f01-run", "--cine-showcase=2"],
            // `--audience=off` and a carry contradict each other
            &[
                "--scene",
                "f01_late",
                "--f01-run",
                "--f01-room",
                "boiler",
                "--audience=off",
                "--f01-audience",
                "1400:1900:600:800",
            ],
        ] {
            assert!(args(bad).is_err(), "{bad:?} must be refused");
        }
        // The opt-out is allowed anywhere, and a valid carry parses in a later room.
        assert!(args(&["--audience=off"]).is_ok());
        let ok = args(&[
            "--scene",
            "f01_late",
            "--f01-run",
            "--f01-room",
            "boiler",
            "--audience=on",
            "--f01-audience",
            "1400:1900:600:800",
        ])
        .unwrap();
        assert_eq!(
            ok.f01_audience.map(f01_run::AudienceCarry::counts),
            Some([(1_400, 1_900), (600, 800)])
        );
    }

    /// AUDIENCE-RUN: the run rooms after the corral unlock the audience at the
    /// carried counts (or the baseline), the corral and every non-run path stay
    /// locked, and an opt-out locks even a later room.
    #[test]
    fn the_run_unlocks_the_audience_after_the_corral_at_the_carried_counts() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(root, LAYOUT_PATH).unwrap();
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        let counts = |list: &[&str]| {
            let hud = new_hud(&args(list).unwrap(), &kit, &actors);
            (
                hud.audience().is_unlocked(),
                hud.audience().carry(),
                hud.audience().count(HeroId::Carl),
            )
        };
        let run = ["--scene", "f01_late", "--f01-run"];
        let with = |extra: &[&'static str]| -> Vec<&'static str> {
            let mut v = run.to_vec();
            v.extend(extra);
            v
        };
        // The corral (the run's first room): locked, even with an explicit flag.
        assert_eq!(counts(&run), (false, None, None));
        assert_eq!(counts(&with(&["--audience=on"])), (false, None, None));
        // A later room without a carry: the baseline, one peak.
        assert_eq!(
            counts(&with(&["--f01-room", "hoarder"])),
            (true, Some([(250, 250), (250, 250)]), Some(250))
        );
        // With a carry: exactly the carried counts.
        assert_eq!(
            counts(&with(&[
                "--f01-room",
                "boiler",
                "--f01-audience",
                "1400:1900:600:800"
            ])),
            (true, Some([(1_400, 1_900), (600, 800)]), Some(1_400))
        );
        // The opt-out and every non-run path stay locked.
        assert_eq!(
            counts(&with(&["--f01-room", "boiler", "--audience=off"])),
            (false, None, None)
        );
        assert_eq!(
            counts(&["--scene", "f01_late", "--f01-room", "boiler"]),
            (false, None, None)
        );
        assert_eq!(counts(&[]), (false, None, None));
        // A scripted room pilot is off by default; an explicit flag wins.
        assert_eq!(
            counts(&with(&["--f01-room", "ring_court", "--ring-pilot"])),
            (false, None, None)
        );
        assert!(
            counts(&with(&[
                "--f01-room",
                "ring_court",
                "--ring-pilot",
                "--audience=on"
            ]))
            .0
        );
    }

    /// AUDIENCE-RUN (AR-AD5, evidence item 5): in a run room that resumed from a carry,
    /// a checkpoint respawn keeps the counts and the dedupe set, and a restart
    /// restores the counts the room began with (so it never farms viewers and never
    /// takes away the carried ones). Prints one `AUDIENCE_STATE` line per step:
    /// `cargo test --bin floor01_scene the_carried_counts -- --nocapture`.
    #[test]
    fn the_carried_counts_survive_a_respawn_and_a_restart_in_a_run_room() {
        use dungeon_floorbreak::combat::{CombatEvent, DefeatCause};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(root, LAYOUT_PATH).unwrap();
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        let parsed = args(&[
            "--scene",
            "f01_late",
            "--f01-run",
            "--f01-room",
            "boiler",
            "--audience=on",
            "--f01-audience",
            "1400:1900:600:800",
        ])
        .unwrap();
        let mut hud = new_hud(&parsed, &kit, &actors);
        let mut log: Vec<CombatEvent> = Vec::new();
        let mut step = |hud: &mut HudRuntime, label: &str, event: Option<CombatEvent>| {
            log.extend(event);
            hud.step(4, 4, &log, 0);
            let audience = hud.audience();
            let (carl, donut) = (
                audience.count(HeroId::Carl).unwrap(),
                audience.count(HeroId::Donut).unwrap(),
            );
            println!(
                "AUDIENCE_RESTART_LOG {label:<26} carl={carl} donut={donut} carry={:?}",
                audience.carry()
            );
            (carl, donut)
        };
        let kill = |tick| CombatEvent::enemy_defeated(tick, 3, DefeatCause::Melee, Some(0), None);
        assert_eq!(step(&mut hud, "room start (carried)", None), (1_400, 600));
        let after_kill = step(&mut hud, "goblin 3 defeated by Carl", Some(kill(10))).0;
        assert!(after_kill > 1_400, "a defeat pays");
        let repeat = step(&mut hud, "goblin 3 again (same id)", Some(kill(11))).0;
        assert_eq!(repeat, after_kill, "one award per target per attempt");
        let respawn = CombatEvent::Respawn {
            tick: 20,
            checkpoint: 1,
        };
        assert_eq!(
            step(&mut hud, "checkpoint Respawn", Some(respawn)).0,
            after_kill,
            "respawn keeps"
        );
        let again = step(&mut hud, "goblin 3 after Respawn", Some(kill(21))).0;
        assert_eq!(again, after_kill, "the dedupe set survives a respawn");
        let restart = CombatEvent::Restart {
            tick: 30,
            deaths: 1,
        };
        assert_eq!(
            step(&mut hud, "Restart", Some(restart)),
            (1_400, 600),
            "restart restores the carried counts"
        );
        let farmed = step(&mut hud, "goblin 3 after Restart", Some(kill(5))).0;
        assert_eq!(farmed, after_kill, "the same award again, no more");
        assert_eq!(
            hud.audience().carry().map(|c| c[0].1),
            Some(farmed.max(1_900)),
            "the peak never drops"
        );
    }

    /// AUDIENCE-RUN (AR-T22, AR-AD8): a thrown triple reaches the audience on the
    /// headless fast-forward (`AudienceFeatFeed`) and the live fan-out
    /// (`print_combat_events`) the same way; a locked audience with an explicit
    /// showcase still computes the moment (printed, never pushed to the HUD),
    /// and with both off the feed returns early as on main.
    #[test]
    fn a_thrown_triple_reaches_the_audience_on_headless_and_live_paths() {
        const FIXTURE: [&str; 6] = [
            "--headless-floor01-smoke",
            "--party-fixture",
            "carl_prog_dyn_thrown_triple",
            "--capture-tick",
            "260",
            "--cine-showcase=on",
        ];
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(root, LAYOUT_PATH).unwrap();
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        let probe = |extra: &[&str]| {
            let mut list = FIXTURE.to_vec();
            list.push("--audience-probe");
            list.extend(extra);
            args(&list).unwrap()
        };

        // Headless fast-forward: the feed runs inside `build_scene` (probe unlocks it).
        let fast = probe(&[]);
        let mut hud_fast = new_hud(&fast, &kit, &actors);
        let (_, _, combat, _, _) = build_scene(&kit, &fast, &actors, root, &mut hud_fast).unwrap();
        let combat = combat.unwrap();
        let blast_defeats = combat
            .events
            .iter()
            .filter(|e| e.to_string().contains("kind=enemy_defeated"))
            .count();
        assert_eq!(blast_defeats, 3, "the triple dies on the one blast tick");

        // Live fan-out: the same events through `print_combat_events`, with the
        // live adapter's tuning, pushed into a probe-unlocked HUD.
        let plain = args(&FIXTURE[..5]).unwrap();
        let mut hud_plain = new_hud(&plain, &kit, &actors);
        let (sim, _, live_combat, _, _) =
            build_scene(&kit, &plain, &actors, root, &mut hud_plain).unwrap();
        let mut live_combat = live_combat.unwrap();
        assert!(
            !hud_plain.audience().is_unlocked(),
            "locked: the feed did nothing"
        );
        let mut hud_live = new_hud(&probe(&[]), &kit, &actors);
        let mut adapter =
            CineAdapter::with_tuning(PresentationSettings::OFF, probe(&[]).cine_tuning());
        let feats = print_combat_events(&mut live_combat, None, Some(&mut adapter));
        assert_eq!(feats.len(), 1, "one thrown-multi moment: {feats:?}");
        assert_eq!((feats[0].credited, feats[0].score), (0, 5));
        for feat in feats {
            hud_live.push_chain_feat(feat);
        }
        step_hud(&mut hud_live, &sim, Some(&live_combat));
        assert_eq!(
            hud_fast.audience().count(HeroId::Carl),
            hud_live.audience().count(HeroId::Carl),
            "the same chain award on both paths"
        );
        assert!(hud_fast.audience().count(HeroId::Carl).unwrap() > 250 + 3 * 1_800);
        assert_eq!(hud_fast.audience().count(HeroId::Donut), Some(250));

        // The default tuning (showcase off) gives no moment on the same events.
        let mut off_adapter = CineAdapter::new(PresentationSettings::OFF);
        let mut replay = build_scene(
            &kit,
            &plain,
            &actors,
            root,
            &mut new_hud(&plain, &kit, &actors),
        )
        .unwrap()
        .2
        .unwrap();
        assert!(print_combat_events(&mut replay, None, Some(&mut off_adapter)).is_empty());

        // A locked audience with an explicit showcase: the feed still evaluates.
        let (_, _, locked_combat, _, _) = build_scene(
            &kit,
            &plain,
            &actors,
            root,
            &mut new_hud(&plain, &kit, &actors),
        )
        .unwrap();
        let locked_combat = locked_combat.unwrap();
        let mut locked_hud = new_hud(&plain, &kit, &actors);
        let explicit = args(&FIXTURE).unwrap();
        let mut feed = AudienceFeatFeed::new(&explicit);
        feed.observe(&locked_combat, &mut locked_hud);
        assert_eq!(feed.adapter.as_ref().map(CineAdapter::emitted), Some(1));
        assert!(!locked_hud.audience().is_unlocked() && locked_hud.audience().digest().is_none());
        // Both off (no probe, no explicit showcase): the early return of main.
        let both_off = args(&FIXTURE[..5]).unwrap();
        let mut feed = AudienceFeatFeed::new(&both_off);
        feed.observe(&locked_combat, &mut new_hud(&both_off, &kit, &actors));
        assert!(feed.adapter.is_none());
    }

    /// AUDIENCE-RUN (AR-T17, section 5): the audience award and the death finish
    /// read the same authoritative pair independently, so the gore preset never
    /// changes an `AUDIENCE_STATE` digest or the combat log.
    #[test]
    fn audience_digest_ignores_the_gore_preset() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(root, LAYOUT_PATH).unwrap();
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        let run = |preset: &str| {
            let parsed = args(&[
                "--headless-floor01-smoke",
                "--party-fixture",
                "carl_prog_dyn_llama_pack",
                "--capture-tick",
                "330",
                "--audience-probe",
                "--cine-showcase=on",
                "--gore-preset",
                preset,
            ])
            .unwrap();
            let mut hud = new_hud(&parsed, &kit, &actors);
            let (sim, _, combat, _, _) =
                build_scene(&kit, &parsed, &actors, root, &mut hud).unwrap();
            let combat = combat.unwrap();
            let events: Vec<String> = combat.events.iter().map(ToString::to_string).collect();
            (hud.audience().digest(), events, combat.state_hash(&sim))
        };
        let (digest_1, events_1, hash_1) = run("1");
        let (digest_4, events_4, hash_4) = run("4");
        assert!(digest_1.is_some());
        assert_eq!(digest_1, digest_4, "the AUDIENCE_STATE digest");
        assert_eq!(events_1, events_4, "the COMBAT_EVENT lines");
        assert_eq!(hash_1, hash_4, "the simulation hash");
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
        assert_eq!(
            args(&["--glyphs=keyboard"]).unwrap().glyphs,
            Some(GlyphSet::Keyboard)
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
        // ENG-02: the Engineer evidence lab names one of the six finishes and is never a recording run.
        assert!(args(&["--death-lab-engineer"]).is_err());
        assert!(args(&["--death-lab-engineer", "knockback"]).is_err());
        assert!(
            args(&["--death-lab-engineer", "crushed"])
                .unwrap()
                .death_lab_engineer
                == Some(Finish::Crushed)
        );
        for name in [
            "crushed",
            "decapitated",
            "exploded",
            "slammed_stomped",
            "slammed_wall",
            "slammed_flung",
        ] {
            assert!(
                args(&[
                    "--scene",
                    "f01_late",
                    "--f01-room",
                    "boiler",
                    "--death-lab-engineer",
                    name
                ])
                .is_ok(),
                "{name}"
            );
        }
        assert!(
            !args(&[
                "--scene",
                "f01_late",
                "--f01-room",
                "boiler",
                "--death-lab-engineer",
                "crushed",
                "--manual"
            ])
            .unwrap()
            .recording_enabled()
        );
        // LLAMA-DEATH-WIRE: the block-edge evidence lab needs a fixture and is
        // never a recording run.
        assert!(args(&["--death-lab-edge"]).is_err());
        let lab = args(&["--fixture", "cp05_llama_crush", "--death-lab-edge"]).unwrap();
        assert!(lab.death_lab_edge);
        assert!(
            !args(&[
                "--fixture",
                "cp05_llama_crush",
                "--death-lab-edge",
                "--manual"
            ])
            .unwrap()
            .recording_enabled()
        );
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

    /// FIX-TOOL: `--exit-after-ticks N` on a live room run with no room pilot is a scripted exit
    /// that keeps the live input source: the run stays the player's (`manual`, `live_input`,
    /// the default recording and the live presentation); with a room pilot, or on any other
    /// run, nothing changes; a pilot-less exit without a room is still refused.
    #[test]
    fn a_live_room_takes_a_scripted_exit_without_a_pilot() {
        for room in ["boiler", "hoarder", "hazards", "mastery", "juicer"] {
            let list = [
                "--scene",
                "f01_late",
                "--f01-room",
                room,
                "--exit-after-ticks",
                "300",
            ];
            let live = args(&list).unwrap_or_else(|e| panic!("{room}: {e}"));
            assert_eq!(live.exit_after_ticks, Some(300));
            assert!(live.manual, "{room}: the run stays the player's");
            assert!(live.live_input() && live.recording_enabled(), "{room}");
            assert!(live.live_room_exit(), "{room}");
            // A room pilot owns the exit: scripted, not live, not recorded by default.
            let plan = match room {
                "boiler" => "direct",
                "hazards" => "direct",
                other => other,
            };
            let mut scripted = list.to_vec();
            scripted.extend_from_slice(&["--room-pilot", plan]);
            let scripted = args(&scripted).unwrap_or_else(|e| panic!("{room}: {e}"));
            assert!(!scripted.manual && !scripted.live_input(), "{room}");
            assert!(!scripted.live_room_exit(), "{room}");
        }
        // The scene entry point `--scene f01_boiler` takes the flag too.
        let boiler = args(&["--scene", "f01_boiler", "--exit-after-ticks", "300"]).unwrap();
        assert!(boiler.manual && boiler.live_input());
        // Any other run: a pilot-less exit needs a scenario or fixture, as before, and does
        // not become a live-input run.
        assert!(args(&["--exit-after-ticks", "300"]).is_err());
        let scenario = args(&["--scenario", "walk_right", "--exit-after-ticks", "60"]).unwrap();
        assert!(!scenario.live_input() && !scenario.live_room_exit());
        // Headless and captured room runs are still scripted.
        let headless = args(&[
            "--scene",
            "f01_late",
            "--f01-room",
            "mastery",
            "--headless-floor01-smoke",
            "--exit-after-ticks",
            "300",
        ])
        .unwrap();
        assert!(!headless.manual && !headless.live_input());
    }

    /// FIX-TOOL: SIGINT and SIGTERM only set the flag [`shutdown_signal`] hands to the exit
    /// system; one signal is seen once, and the handler is one-shot (the disposition is the
    /// default again after it fires, so a second signal would kill). The test puts the
    /// default back before it ends, so nothing is left installed in the test binary.
    #[cfg(unix)]
    #[test]
    fn a_shutdown_signal_sets_the_exit_flag_once() {
        let disposition = |signal: libc::c_int| {
            // SAFETY: reads the current disposition (and puts it straight back).
            unsafe {
                let mut old: libc::sigaction = std::mem::zeroed();
                libc::sigaction(signal, std::ptr::null(), &mut old);
                old.sa_sigaction
            }
        };
        for signal in [libc::SIGINT, libc::SIGTERM] {
            shutdown_signal::install();
            assert!(!shutdown_signal::take());
            // SAFETY: raises a signal whose handler was just installed (one atomic store).
            assert_eq!(unsafe { libc::raise(signal) }, 0);
            assert!(shutdown_signal::take(), "signal {signal}");
            assert!(!shutdown_signal::take(), "signal {signal} is taken once");
            assert_eq!(
                disposition(signal),
                libc::SIG_DFL,
                "signal {signal}: one-shot, the default is back after the first"
            );
        }
        shutdown_signal::restore_default();
        for signal in [libc::SIGINT, libc::SIGTERM] {
            assert_eq!(disposition(signal), libc::SIG_DFL);
        }
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

    /// EVID-ROOM: `--room-pilot` is a room scene's scripted input: it needs a room
    /// and a scripted run, parses the plan and `:2p`, and is refused elsewhere.
    #[test]
    fn the_room_pilot_flag_needs_a_room_scene_and_a_scripted_run() {
        let room = ["--scene", "f01_late", "--f01-room", "boiler"];
        let with = |extra: &[&str]| {
            let mut list: Vec<&str> = room.to_vec();
            list.extend_from_slice(extra);
            args(&list)
        };
        let a = with(&["--room-pilot", "chain:2p", "--headless-floor01-smoke"]).unwrap();
        let pilot = a.room_pilot.unwrap();
        assert_eq!(pilot.label(), "chain:2p");
        assert!(
            with(&[
                "--room-pilot",
                "direct",
                "--capture-sequence",
                "0:6:3",
                "--capture-dir",
                "f"
            ])
            .is_ok()
        );
        // PT-GATE-A: the mastery plan plays the closing arena only, and the perf
        // probe (`--perf-from-tick`) takes a solo room pilot.
        let mastery = ["--scene", "f01_late", "--f01-room", "mastery"];
        let on_mastery = |extra: &[&str]| {
            let mut list: Vec<&str> = mastery.to_vec();
            list.extend_from_slice(extra);
            args(&list)
        };
        assert!(on_mastery(&["--room-pilot", "mastery", "--headless-floor01-smoke"]).is_ok());
        assert!(on_mastery(&["--room-pilot", "direct", "--headless-floor01-smoke"]).is_err());
        assert!(with(&["--room-pilot", "mastery", "--headless-floor01-smoke"]).is_err());
        let juicer = [
            "--scene",
            "f01_late",
            "--f01-room",
            "juicer",
            "--headless-floor01-smoke",
        ];
        assert!(args(&[&juicer[..], &["--room-pilot", "juicer"]].concat()).is_ok());
        assert!(args(&[&juicer[..], &["--room-pilot", "direct"]].concat()).is_err());
        assert!(with(&["--room-pilot", "juicer", "--headless-floor01-smoke"]).is_err());
        // PT-GATE-D: the room 4 route plays the hazards room only, and solo.
        let hazards = [
            "--scene",
            "f01_late",
            "--f01-room",
            "hazards",
            "--headless-floor01-smoke",
        ];
        assert!(args(&[&hazards[..], &["--room-pilot", "haz_spectacular"]].concat()).is_ok());
        assert!(args(&[&hazards[..], &["--room-pilot", "haz_spectacular:2p"]].concat()).is_err());
        assert!(
            with(&[
                "--room-pilot",
                "haz_spectacular",
                "--headless-floor01-smoke"
            ])
            .is_err()
        );
        assert!(
            on_mastery(&[
                "--room-pilot",
                "haz_spectacular",
                "--headless-floor01-smoke"
            ])
            .is_err()
        );
        // PT-GATE-E: haz_tactical is the same solo hazards-room plan; the aligned
        // `hazards:` and `boiler:` library pilots play their own room, take the seed as the
        // room's seed and fix their partner.
        assert!(args(&[&hazards[..], &["--room-pilot", "haz_tactical"]].concat()).is_ok());
        assert!(args(&[&hazards[..], &["--room-pilot", "haz_tactical:2p"]].concat()).is_err());
        assert!(on_mastery(&["--room-pilot", "haz_tactical", "--headless-floor01-smoke"]).is_err());
        let aligned_hazards =
            args(&[&hazards[..], &["--room-pilot", "hazards:donut_by_swap:4"]].concat()).unwrap();
        assert_eq!(aligned_hazards.cf02.seed, Some(4));
        assert!(
            args(
                &[
                    &hazards[..],
                    &["--room-pilot", "hazards:carl_solo:2", "--cf02-seed", "3"]
                ]
                .concat()
            )
            .is_err()
        );
        assert!(
            args(
                &[
                    &hazards[..],
                    &["--room-pilot", "hazards:carl_solo:2", "--companion", "off"]
                ]
                .concat()
            )
            .is_err()
        );
        assert!(
            args(&[&hazards[..], &["--room-pilot", "boiler:carl_solo:both:2"]].concat()).is_err()
        );
        assert!(
            on_mastery(&[
                "--room-pilot",
                "hazards:carl_solo:2",
                "--headless-floor01-smoke"
            ])
            .is_err()
        );
        let boiler = [
            "--scene",
            "f01_late",
            "--f01-room",
            "boiler",
            "--headless-floor01-smoke",
        ];
        let aligned_boiler =
            args(&[&boiler[..], &["--room-pilot", "boiler:companion:both:6"]].concat()).unwrap();
        assert_eq!(aligned_boiler.cf02.seed, Some(6));
        assert!(args(&[&boiler[..], &["--room-pilot", "hazards:carl_solo:6"]].concat()).is_err());
        assert!(args(&[&boiler[..], &["--room-pilot", "haz_tactical"]].concat()).is_err());
        assert!(args(&[&boiler[..], &["--room-pilot", "boiler"]].concat()).is_err());
        // :2p is refused for the two solo plans.
        assert!(on_mastery(&["--room-pilot", "mastery:2p", "--headless-floor01-smoke"]).is_err());
        assert!(args(&[&juicer[..], &["--room-pilot", "juicer:2p"]].concat()).is_err());
        let perf = ["--perf-frames", "50", "--perf-from-tick", "0"];
        let solo: Vec<&str> = ["--room-pilot", "chain"].into_iter().chain(perf).collect();
        assert!(with(&solo).is_ok());
        let duo: Vec<&str> = ["--room-pilot", "chain:2p"]
            .into_iter()
            .chain(perf)
            .collect();
        assert!(with(&duo).is_err());
        assert!(
            on_mastery(&[
                "--room-pilot",
                "mastery",
                "--perf-frames",
                "50",
                "--perf-from-tick",
                "0"
            ])
            .is_ok()
        );
        // A live (unscripted) room run, an unknown plan and a missing value.
        assert!(with(&["--room-pilot", "direct"]).is_err());
        assert!(with(&["--room-pilot", "sprint", "--headless-floor01-smoke"]).is_err());
        assert!(with(&["--room-pilot"]).is_err());
        // Every other scene refuses it.
        assert!(
            args(&[
                "--demo",
                "--room-pilot",
                "direct",
                "--headless-floor01-smoke"
            ])
            .is_err()
        );
        assert!(
            args(&[
                "--scene",
                "f01_late",
                "--f01-room",
                "juicer",
                "--room-pilot",
                "direct",
                "--headless-floor01-smoke"
            ])
            .is_err()
        );
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
        assert_eq!(s.p99_ms, 99.0);
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

    /// K16 (BOIL-E): the Engineer outside the microarena. The story route,
    /// `--demo`, plain `--cf02`, the lab (every room, every mode), the boss arena
    /// and every `combat_scene::FIXTURES` entry never load him (the party,
    /// boss and room-owned fixture lists are not iterated: only the two
    /// `Roster::Live` callers, both through `Roster::for_loadout`, build an
    /// Engineer outside the microarena); only a **sidecar roster**
    /// does, and only the Boiler's story launch (`--scene f01_late`, a live
    /// roster) names one. The plain `--scene f01_boiler` greybox has no enemy.
    #[test]
    fn the_engineer_loads_only_from_a_sidecar_roster_and_the_old_routes_never_load_him() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let none = |fixture: Option<&dungeon_floorbreak::combat_scene::Fixture>| {
            fixture.is_none_or(|f| f.engineers.is_empty())
        };
        for list in [
            &[][..],
            &["--demo"][..],
            &["--cf02"][..],
            &["--cf02", "--demo-beat", "1"][..],
            &["--scene", "boss_f01"][..],
            &["--scene", "combat_lab"][..],
            &["--scene", "f01_boiler"][..],
        ] {
            let parsed = args(list).unwrap();
            assert!(
                none(parsed.fixture),
                "{list:?} resolved an Engineer fixture"
            );
            assert!(!parsed.cf02_engineer, "{list:?}");
        }
        for fixture in dungeon_floorbreak::combat_scene::FIXTURES {
            assert!(fixture.engineers.is_empty(), "{}", fixture.name);
        }
        for beat in 0..6 {
            assert!(demo::fixture_for_beat(beat).engineers.is_empty());
            assert!(
                dungeon_floorbreak::cf02::arena::fixture_for_run(
                    beat,
                    false,
                    dungeon_floorbreak::engineer::active()
                )
                .engineers
                .is_empty()
            );
        }
        assert!(boss_scene::BOSS01_SCENE.engineers.is_empty());
        // The lab: every listed room under every player mode.
        let catalog = LabCatalog::load(&root).unwrap();
        let mut launched = 0;
        for room in catalog.rooms() {
            for mode in [
                combat_lab::PlayerMode::SoloCarl,
                combat_lab::PlayerMode::SoloDonut,
                combat_lab::PlayerMode::CoopLocal,
            ] {
                // A room linked to the boss arena (R06) refuses to launch in the
                // lab; it has no lab fixture to load an Engineer into.
                let Ok(run) =
                    catalog.launch(EntryPoint::CombatLab, &LabConfig::new(&room.id, mode))
                else {
                    continue;
                };
                launched += 1;
                assert!(
                    run.info.fixture.engineers.is_empty(),
                    "{} {mode:?}",
                    room.id
                );
            }
        }
        assert!(
            launched >= 18,
            "the lab launched {launched} room and mode runs"
        );
        // The only Engineer outside the microarena is the Boiler's live roster.
        let greybox = f01_boiler::launch(&root).unwrap();
        assert!(greybox.fixture.engineers.is_empty() && greybox.fixture.enemies.is_empty());
        let story = f01_boiler::launch_with(&root, f01_boiler::Roster::Live).unwrap();
        assert_eq!(story.fixture.engineers.len(), 1);
        let late = args(&["--scene", "f01_late", "--f01-room", "boiler"]).unwrap();
        assert!(late.f01_boiler && late.f01_loadout.is_some());
        assert_eq!(
            f01_boiler::Roster::for_loadout(late.f01_loadout.map(|l| l.name())),
            f01_boiler::Roster::Live
        );
        assert!(
            !late.cf02_engineer,
            "--cf02-engineer stays the microarena's"
        );
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

    /// FA-Q08: room 5 (the Juicer's room) accepts `--companion` like rooms 2, 4 and 6,
    /// defaults to `cf02_free` (FA-Q07 met) and `--companion off` restores the follow bot.
    #[test]
    fn the_juicers_room_accepts_companion_and_defaults_to_cf02_free() {
        let juicer = ["--scene", "f01_late", "--f01-room", "juicer"];
        let with = |extra: &[&str]| args(&[&juicer[..], extra].concat());
        let who = |a: &Args| a.uncontrolled_hero();
        assert_eq!(
            who(&with(&[]).unwrap()),
            UncontrolledHero::Companion(ProfileSetId::Cf02Free),
            "the default"
        );
        assert_eq!(
            who(&with(&["--companion", "off"]).unwrap()),
            UncontrolledHero::FollowAndJumpBot
        );
        assert_eq!(
            who(&with(&["--companion", "cf02_assist"]).unwrap()),
            UncontrolledHero::Companion(ProfileSetId::Cf02Assist)
        );
        assert_eq!(
            who(&with(&["--companion", "v0"]).unwrap()),
            UncontrolledHero::Companion(ProfileSetId::CompanionV0)
        );
        // A user fixture is still refused, by the room rule.
        assert!(with(&["--fixture", "cp01_kick", "--companion", "cf02_free"]).is_err());
        // The plain boss scene keeps its follow bot and refuses `--companion`.
        let boss = args(&["--scene", "boss_f01"]).unwrap();
        assert_eq!(who(&boss), UncontrolledHero::FollowAndJumpBot);
    }

    /// FIX-TOOL (PT-FU-B8 item 3): `user_fixture()` is inferred from "a fixture inside a room
    /// scene is the scene's own", which holds only while every room scene refuses a user
    /// `--fixture` or `--party-fixture` up front. Assert the refusal and its text for every
    /// room scene, including `--fixture cp01_kick --companion cf02_free`, which is refused
    /// by the room rule ("replaces ... --fixture"), not by the companion rule.
    #[test]
    fn every_room_scene_refuses_a_user_fixture_up_front() {
        let rooms: [&[&str]; 8] = [
            &["--scene", "f01_boiler"],
            &["--scene", "f01_hoarder"],
            &["--scene", "f01_hazards"],
            &["--scene", "f01_late", "--f01-room", "boiler"],
            &["--scene", "f01_late", "--f01-room", "hoarder"],
            &["--scene", "f01_late", "--f01-room", "hazards"],
            &["--scene", "f01_late", "--f01-room", "mastery"],
            &["--scene", "f01_late", "--f01-room", "juicer"],
        ];
        let late = "--scene f01_late replaces --demo, --cf02, --cf02-engineer, --autopilot, --demo-beat, --fixture, --party-fixture, --scenario, --camera-test-room and --reference-frame (a room is its own scene)";
        for room in rooms {
            let scene = room[1];
            let expected = if scene == "f01_late" {
                late.to_owned()
            } else {
                format!(
                    "--scene {scene} replaces --demo, --cf02, --cf02-engineer, --autopilot, --demo-beat, --fixture, --party-fixture, --scenario, --camera-test-room and --reference-frame (it is its own room)"
                )
            };
            for extra in [
                &["--fixture", "cp01_kick"][..],
                &["--fixture", "cp01_kick", "--companion", "cf02_free"][..],
                &["--party-fixture", "companion_duo_no_ai"][..],
                &[
                    "--party-fixture",
                    "companion_duo_no_ai",
                    "--headless-floor01-smoke",
                ][..],
            ] {
                // The user's flags come before and after the scene: both orders are refused.
                for before in [false, true] {
                    let mut list: Vec<&str> = Vec::new();
                    if before {
                        list.extend_from_slice(extra);
                        list.extend_from_slice(room);
                    } else {
                        list.extend_from_slice(room);
                        list.extend_from_slice(extra);
                    }
                    let refusal = args(&list).expect_err(&format!("{list:?} must be refused"));
                    assert_eq!(refusal, expected, "{list:?}");
                    assert!(
                        !refusal.contains("--companion cannot be combined"),
                        "the room rule refuses first, not the companion rule: {list:?}"
                    );
                }
            }
            // The scene's own placeholder is not a user fixture; a real `--fixture` is
            // (FA-Q08: the Juicer's room too).
            let own = args(room).unwrap();
            assert!(
                !own.user_fixture(),
                "{room:?}: the placeholder is the scene's own"
            );
        }
        assert!(args(&["--fixture", "cp01_kick"]).unwrap().user_fixture());
        assert!(
            args(&[
                "--party-fixture",
                "companion_duo_no_ai",
                "--headless-floor01-smoke"
            ])
            .unwrap()
            .user_fixture()
        );
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
        // COMPANION-AI-02 (CA2-Q01): the autonomous set, never a default.
        let free = UncontrolledHero::Companion(ProfileSetId::Cf02Free);
        assert_eq!(hero(&["--companion", "cf02_free"]), free);
        assert_eq!(hero(&["--cf02", "--companion", "cf02_free"]), free);
        assert_eq!(hero(&["--cf02"]), assist);
        assert_eq!(hero(&["--cf02", "--companion", "off"]), bot);
        assert_eq!(hero(&["--cf02", "--companion", "v0"]), v0);
        assert!(args(&["--companion"]).is_err());
        assert!(args(&["--companion", "fast"]).is_err());
        // A fixture declares its own tuning: the companion rule itself refuses it.
        let refusal = args(&["--fixture", "cp01_kick", "--companion", "v0"]).unwrap_err();
        assert!(
            refusal.contains("--companion cannot be combined"),
            "{refusal}"
        );
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
        // PT-GATE-C1: an `f01_late` room scene defaults the partner to
        // `cf02_free` and `--companion` overrides it; the placeholder fixture the
        // room scene sets is not a user fixture. The `--cf02` arena stays `cf02_assist`.
        let room = |extra: &[&str]| {
            let mut list = vec!["--scene", "f01_late", "--f01-room", "mastery"];
            list.extend_from_slice(extra);
            args(&list).unwrap()
        };
        for name in ["boiler", "hoarder", "hazards", "mastery"] {
            let parsed = args(&["--scene", "f01_late", "--f01-room", name]).unwrap();
            assert_eq!(
                parsed.uncontrolled_hero(),
                free,
                "{name} defaults to cf02_free"
            );
            let parsed = args(&[
                "--scene",
                "f01_late",
                "--f01-room",
                name,
                "--companion",
                "cf02_assist",
            ])
            .unwrap();
            assert_eq!(parsed.uncontrolled_hero(), assist, "{name} override");
        }
        assert_eq!(room(&["--companion", "off"]).uncontrolled_hero(), bot);
        assert_eq!(
            args(&["--scene", "f01_hoarder"])
                .unwrap()
                .uncontrolled_hero(),
            free,
            "the direct room scene flags default alike"
        );
        // A user fixture is still refused next to --companion, in a room scene too.
        assert!(
            args(&[
                "--scene",
                "f01_late",
                "--f01-room",
                "mastery",
                "--fixture",
                "cp01_kick",
                "--companion",
                "cf02_free"
            ])
            .is_err(),
            "refused by the room scene's own fixture rule"
        );
        assert!(
            args(&["--scene", "boss_f01", "--companion", "cf02_free"]).is_err(),
            "the boss arena's fixture is not a room placeholder"
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

    /// BOIL-A wire: `--scene f01_boiler` implies `cf02_rules` without the
    /// `--cf02` DEMO forcing, is its own room, refuses the modes that pick
    /// another room, is never recorded, and applies the sidecar's story pin only
    /// under the program flags.
    #[test]
    fn the_boiler_scene_implies_cf02_rules_without_the_demo_forcing() {
        let boiler = args(&["--scene", "f01_boiler"]).unwrap();
        assert!(boiler.f01_boiler && boiler.cf02.rules, "implies cf02_rules");
        assert!(!boiler.demo, "no DEMO forcing");
        assert!(!boiler.arena_room(), "not the microarena");
        assert!(boiler.donut_biscuit, "the post-Hoarder state");
        assert!(boiler.manual, "a live run is the player's");
        assert_eq!(boiler.fixture, Some(&PENDING_FIXTURE));
        // `--cf02` is the microarena, so asking for both is an error.
        let error = args(&["--scene", "f01_boiler", "--cf02"]).expect_err("--cf02");
        assert!(error.contains("already implies cf02_rules"), "{error}");
        assert!(args(&["--cf02", "--scene", "f01_boiler"]).is_err());
        // The seed flag works because cf02_rules is implied.
        assert_eq!(
            args(&["--scene", "f01_boiler", "--cf02-seed", "7"])
                .unwrap()
                .cf02
                .seed,
            Some(7)
        );
        // Another room or script is refused, and the scenes are exclusive.
        for extra in [
            &["--demo"][..],
            &["--cf02-engineer"],
            &["--autopilot"],
            &["--demo-beat", "2"],
            &["--fixture", "cp01_kick"],
            &["--party-fixture", "duo_independent_moves"],
            &["--scenario", "walk_right"],
            &["--camera-test-room", "1"],
            &["--reference-frame"],
        ] {
            let mut list = vec!["--scene", "f01_boiler"];
            list.extend_from_slice(extra);
            assert!(args(&list).is_err(), "{list:?}");
        }
        assert!(args(&["--scene", "f01_boiler", "--scene", "boss_f01"]).is_err());
        assert!(args(&["--scene", "combat_lab", "--scene", "f01_boiler"]).is_err());
        assert!(args(&["--scene", "boss_f01", "--scene", "f01_boiler"]).is_err());
        // A scripted run is not the player's. (`--exit-after-ticks` alone is not a script
        // since FIX-TOOL: it only ends a live run, see
        // `a_live_room_takes_a_scripted_exit_without_a_pilot`; with a room pilot it is.)
        for scripted in [
            &["--headless-floor01-smoke"][..],
            &["--capture-tick", "300"],
            &["--exit-after-ticks", "300", "--room-pilot", "direct"],
        ] {
            let mut list = vec!["--scene", "f01_boiler"];
            list.extend_from_slice(scripted);
            assert!(!args(&list).unwrap().manual, "{list:?}");
        }
        assert!(
            args(&["--scene", "f01_boiler", "--exit-after-ticks", "300"])
                .unwrap()
                .manual
        );
        // The other scenes are unchanged: `--cf02` still forces the DEMO machinery.
        assert!(args(&["--cf02"]).unwrap().demo);
        assert!(!args(&["--cf02"]).unwrap().f01_boiler);
    }

    /// PT-SCENE: the Boiler has a replay source now, so it records like every
    /// live room (`--record` is accepted) and its descriptor names the room.
    #[test]
    fn the_boiler_scene_is_recorded_since_pt_scene() {
        assert!(
            args(&["--scene", "f01_boiler"])
                .unwrap()
                .recording_enabled()
        );
        assert!(
            args(&["--scene", "f01_boiler", "--record"])
                .unwrap()
                .recording_enabled()
        );
        assert!(
            !args(&["--scene", "f01_boiler", "--no-record"])
                .unwrap()
                .recording_enabled()
        );
        // A scripted run records nothing by default.
        assert!(
            !args(&["--scene", "f01_boiler", "--headless-floor01-smoke"])
                .unwrap()
                .recording_enabled()
        );
        assert_eq!(
            args(&["--scene", "f01_boiler"])
                .unwrap()
                .f01_room
                .as_deref(),
            Some("boiler")
        );
        // Every other live run records by default, and names no room.
        assert!(args(&["--cf02"]).unwrap().recording_enabled());
        assert!(args(&["--cf02"]).unwrap().f01_room.is_none());
        assert!(args(&["--scene", "boss_f01"]).unwrap().f01_room.is_none());
    }

    /// HRD-A2b and HAZ-A2b: the Hoarder's room and the hazard room are listed
    /// rooms of `--scene f01_late` and scenes of their own. Each implies
    /// `cf02_rules` like the Boiler (no microarena, no demo), names its room,
    /// records by default, and refuses what replaces it.
    #[test]
    fn the_slot_room_scenes_resolve_like_the_boiler() {
        // FA-Q24 (reverses PTG-AD1): the Hoarder is a listed `f01_late` room again and
        // still a scene of its own.
        for (room, direct) in [("hoarder", "f01_hoarder"), ("hazards", "f01_hazards")] {
            let late = args(&["--scene", "f01_late", "--f01-room", room]).unwrap();
            assert!(late.f01_late && late.cf02.rules && !late.demo && !late.f01_boiler);
            assert_eq!(late.f01_hoarder, room == "hoarder");
            assert_eq!(late.f01_hazards, room == "hazards");
            assert_eq!(late.f01_room.as_deref(), Some(room));
            assert_eq!(late.f01_loadout, Some(f01_late::Loadout::Story));
            assert!(late.carl.prog && late.donut.prog && late.donut_biscuit);
            assert!(late.recording_enabled());
            assert!(!late.arena_room(), "no microarena");
            // The TEST LOADOUT holds no program flag.
            let all = args(&[
                "--scene",
                "f01_late",
                "--f01-room",
                room,
                "--f01-loadout",
                "all",
            ])
            .unwrap();
            assert!(all.carl.dynamite && all.donut.puddle && !all.carl.prog);
            // The direct scene names the room and no loadout.
            let direct_args = args(&["--scene", direct]).unwrap();
            assert_eq!(direct_args.f01_room.as_deref(), Some(room));
            assert!(direct_args.f01_loadout.is_none() && direct_args.cf02.rules);
            assert!(direct_args.recording_enabled());
            assert!(
                !args(&["--scene", direct, "--no-record"])
                    .unwrap()
                    .recording_enabled()
            );
            // What the scene replaces is refused, with its own scene name.
            let error = args(&["--scene", direct, "--cf02"]).expect_err("--cf02");
            assert!(
                error.contains(direct) && error.contains("drop --cf02"),
                "{error}"
            );
            for flag in ["--demo", "--autopilot", "--cf02-engineer"] {
                assert!(args(&["--scene", direct, flag]).is_err(), "{direct} {flag}");
            }
            for other in ["boss_f01", "combat_lab", "f01_boiler", "f01_late"] {
                assert!(args(&["--scene", direct, "--scene", other]).is_err());
                assert!(args(&["--scene", other, "--scene", direct]).is_err());
            }
        }
        // The two scenes are exclusive of each other.
        assert!(args(&["--scene", "f01_hoarder", "--scene", "f01_hazards"]).is_err());
        // The Hoarder's room has no goblin, so no seed; the hazard room's goblins
        // take one.
        for list in [
            &["--scene", "f01_hoarder", "--cf02-seed", "7"][..],
            &[
                "--scene",
                "f01_late",
                "--f01-room",
                "hoarder",
                "--cf02-seed",
                "7",
            ][..],
        ] {
            assert!(args(list).expect_err("a seed").contains("no --cf02-seed"));
        }
        assert_eq!(
            args(&["--scene", "f01_hazards", "--cf02-seed", "7"])
                .unwrap()
                .cf02
                .seed,
            Some(7)
        );
        // A bare `--scene f01_late` still launches the Boiler (HRDA2B-AD1).
        let bare = args(&["--scene", "f01_late"]).unwrap();
        assert!(bare.f01_boiler && !bare.f01_hoarder && !bare.f01_hazards);
        // DR-A lists the Dozer; an unknown room stays refused with the listed ones named.
        assert!(
            args(&["--scene", "f01_late", "--f01-room", "dozer"])
                .unwrap()
                .f01_dozer
        );
        let error = args(&["--scene", "f01_late", "--f01-room", "cellar"]).expect_err("unknown");
        assert!(
            error.contains(
                "corral, rest_refuge, hoarder, boiler, dozer, hazards, ring_court, ring_split, rest_camp, juicer, rest_aftermath, mastery"
            ),
            "{error}"
        );
        // No other run sets either scene.
        assert!(!args(&["--cf02"]).unwrap().f01_room_scene());
        assert!(!args(&["--scene", "boss_f01"]).unwrap().f01_room_scene());
    }

    /// CORRAL-F: room 0 is the first listed room of `--scene f01_late` (plan
    /// section 14.7; CORRAL-WIRE launched it unlisted): `--f01-room corral`
    /// launches it under the room rules with a seed, records under the
    /// late-Floor-1 source with its own fixture name and pins, and its tick-0
    /// state in the binary is the verifier's constructor; a floor run starts in
    /// it and relaunches into the Hoarder with its ledger, and it has no scene arm.
    #[test]
    fn the_corral_resolves_through_the_late_launcher_listed_first() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        let id = f01_corral::ROOM_ID;
        let late = args(&["--scene", "f01_late", "--f01-room", id]).unwrap();
        assert!(late.f01_late && late.cf02.rules && !late.demo);
        assert!(late.f01_corral && late.f01_ring.is_none());
        assert!(!late.f01_boiler && !late.f01_hoarder && !late.f01_hazards && !late.f01_mastery);
        assert!(late.f01_room_scene() && !late.arena_room());
        assert_eq!(late.f01_room.as_deref(), Some(id));
        assert!(late.carl.prog && late.donut.prog && late.donut_biscuit);
        assert!(late.recording_enabled());
        let run = args(&["--scene", "f01_late", "--f01-room", id, "--f01-run"]).unwrap();
        assert!(run.f01_run && run.f01_corral);
        // ACT1-F: the run starts in the first Act I room (the chase), and the corral follows the
        // guild.
        let first = args(&["--scene", "f01_late", "--f01-run"]).unwrap();
        assert!(!first.f01_corral && first.f01_room.as_deref() == Some("act1_chase"));
        assert_eq!(first.f01_hud, Some(("The First Chase".to_owned(), false)));
        let direct = args(&["--scene", "f01_late", "--f01-run", "--f01-room", id]).unwrap();
        assert!(direct.f01_corral);
        assert_eq!(
            direct.f01_hud,
            Some(("The Infernal Corral".to_owned(), false))
        );
        for flag in ["--cf02", "--demo", "--autopilot", "--cf02-engineer"] {
            assert!(
                args(&["--scene", "f01_late", "--f01-room", id, flag]).is_err(),
                "{flag}"
            );
        }
        assert!(
            args(&["--scene", f01_corral::SCENE_NAME]).is_err(),
            "no scene arm"
        );
        for list in [
            &["--scene", "f01_late", "--f01-room", id][..],
            &[
                "--scene",
                "f01_late",
                "--f01-room",
                id,
                "--f01-loadout",
                "all",
            ][..],
            &["--scene", "f01_late", "--f01-room", id, "--cf02-seed", "7"][..],
        ] {
            let mut run = args(list).unwrap();
            let launched = f01_corral::launch(root).unwrap();
            run.fixture = Some(launched.fixture);
            let kit = launched.kit.clone();
            run.corral_run = Some(Box::new(launched));
            let (_, scene, library) =
                f01_late::corral_start(root, 360, run.carl, run.donut, run.cf02.seed).unwrap();
            let mut hud = new_hud(&run, &kit, &actors);
            let (sim, _, combat, _, _) = build_scene(&kit, &run, &actors, root, &mut hud).unwrap();
            let combat = combat.expect("a combat state");
            assert_eq!(
                combat.state_hash(&sim),
                library.state_hash(&scene),
                "{list:?}"
            );
            assert_eq!(sim.world.state_hash(), scene.world.state_hash(), "{list:?}");
            assert!(!combat.exit_gate_open(), "the M3 gate stands");
            let party = run.live_party();
            let manifest = recording_manifest(&run, &sim, Some(&combat), &party, root);
            assert_eq!(manifest.reset.source, f01_late::SESSION_SOURCE);
            assert_eq!(manifest.reset.name, f01_corral::SCENE_NAME);
            assert_eq!(manifest.reset.f01_room.as_deref(), Some(id));
            assert_eq!(manifest.reset.cf02_seed, run.cf02.seed);
            for key in f01_late::SCOPED_PIN_KEYS {
                assert_eq!(
                    manifest.content_hashes.contains_key(key),
                    f01_late::pin_applies(key, Some(id)),
                    "{key}"
                );
            }
            assert_eq!(manifest.reset.start_hash, library.state_hash(&scene));
        }
        // CORRAL-F: the run's next room is the Hoarder, carried the corral's
        // ledger. The binary's carried Hoarder is her pinned build: the corral
        // holds nothing her pin does not (KC11).
        let (_, _, corral) =
            f01_late::corral_start(root, 360, late.carl, late.donut, None).unwrap();
        let snapshot = f01_run::carry_for(
            f01_late::Loadout::Story,
            f01_late::RoomScene::Corral,
            &corral,
        )
        .expect("the story loadout carries (the empty ledger, CF-AD3)");
        let carry = f01_run::encode_carry(&snapshot);
        let hoarder = |carried: bool| {
            let mut list = vec!["--scene", "f01_late", "--f01-run", "--f01-room", "hoarder"];
            if carried {
                list.extend([
                    "--f01-carry",
                    carry.as_str(),
                    "--f01-run-tally",
                    "1:600:0:0",
                ]);
            }
            let mut run = args(&list).unwrap();
            let launched = f01_hoarder::launch(root).unwrap();
            run.fixture = Some(launched.fixture);
            let kit = launched.kit.clone();
            run.hoarder_run = Some(Box::new(launched));
            let mut hud = new_hud(&run, &kit, &actors);
            let (sim, _, combat, _, _) = build_scene(&kit, &run, &actors, root, &mut hud).unwrap();
            let combat = combat.expect("a combat state");
            (
                run.f01_carry.is_some(),
                combat.state_hash(&sim),
                combat.story_snapshot(),
            )
        };
        let (with_carry, carried_hash, carried_ledger) = hoarder(true);
        let (without, pinned_hash, pinned_ledger) = hoarder(false);
        assert!(with_carry && !without);
        assert_eq!(carried_ledger, pinned_ledger);
        assert_eq!(carried_hash, pinned_hash);
    }

    /// RING-A/B wire: the two Ring rooms are listed rooms of `--scene f01_late`
    /// only (KR22: no scene arm). Each implies `cf02_rules`, takes a seed (its
    /// goblins draw temperaments), records under the late-Floor-1 source with its
    /// own fixture name, stamps its own room pins and the manifest's, and its
    /// tick-0 state in the binary is the verifier's constructor.
    #[test]
    fn the_ring_rooms_resolve_through_the_late_launcher() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        for room in f01_ring::RingRoom::ALL {
            let id = room.id();
            let late = args(&["--scene", "f01_late", "--f01-room", id]).unwrap();
            assert!(late.f01_late && late.cf02.rules && !late.demo, "{id}");
            assert_eq!(late.f01_ring, Some(room));
            assert!(
                !late.f01_boiler && !late.f01_hoarder && !late.f01_hazards && !late.f01_mastery
            );
            assert!(
                late.f01_room_scene() && !late.arena_room(),
                "{id}: its own room"
            );
            assert_eq!(late.f01_room.as_deref(), Some(id));
            assert!(late.carl.prog && late.donut.prog && late.donut_biscuit);
            assert!(late.recording_enabled());
            for flag in ["--cf02", "--demo", "--autopilot", "--cf02-engineer"] {
                assert!(
                    args(&["--scene", "f01_late", "--f01-room", id, flag]).is_err(),
                    "{id} {flag}"
                );
            }
            assert!(
                args(&["--scene", room.scene_name()]).is_err(),
                "no scene arm"
            );
            for list in [
                &["--scene", "f01_late", "--f01-room", id][..],
                &[
                    "--scene",
                    "f01_late",
                    "--f01-room",
                    id,
                    "--f01-loadout",
                    "all",
                ][..],
                &["--scene", "f01_late", "--f01-room", id, "--cf02-seed", "7"][..],
            ] {
                let mut run = args(list).unwrap();
                let launched = f01_ring::launch(root, room).unwrap();
                run.fixture = Some(launched.fixture);
                let kit = launched.kit.clone();
                run.ring_run = Some(Box::new(launched));
                let (_, scene, library) =
                    f01_late::ring_start(root, room, 360, run.carl, run.donut, run.cf02.seed)
                        .unwrap();
                let mut hud = new_hud(&run, &kit, &actors);
                let (sim, _, combat, _, _) =
                    build_scene(&kit, &run, &actors, root, &mut hud).unwrap();
                let combat = combat.expect("a combat state");
                assert_eq!(
                    combat.state_hash(&sim),
                    library.state_hash(&scene),
                    "{list:?}"
                );
                assert_eq!(sim.world.state_hash(), scene.world.state_hash(), "{list:?}");
                let party = run.live_party();
                let manifest = recording_manifest(&run, &sim, Some(&combat), &party, root);
                assert_eq!(manifest.reset.source, f01_late::SESSION_SOURCE);
                assert_eq!(manifest.reset.name, room.scene_name());
                assert_eq!(manifest.reset.f01_room.as_deref(), Some(id));
                assert_eq!(manifest.reset.cf02_seed, run.cf02.seed);
                for key in f01_late::SCOPED_PIN_KEYS {
                    assert_eq!(
                        manifest.content_hashes.contains_key(key),
                        f01_late::pin_applies(key, Some(id)),
                        "{key}"
                    );
                }
                assert_eq!(manifest.reset.start_hash, library.state_hash(&scene));
            }
        }
    }

    /// The refuge's evidence pilot is a one-room scripted run: it needs the refuge and is
    /// refused with `--f01-run`, `--room-pilot` and `--ring-pilot`.
    #[test]
    fn the_rest_pilot_is_a_one_room_evidence_pilot() {
        let ok = args(&[
            "--scene",
            "f01_late",
            "--f01-room",
            "rest_refuge",
            "--rest-pilot",
            "donut",
        ])
        .unwrap();
        assert!(ok.rest_pilot.is_some());
        let err = args(&[
            "--scene",
            "f01_late",
            "--f01-run",
            "--f01-room",
            "rest_refuge",
            "--rest-pilot",
            "donut",
        ])
        .expect_err("a pilot cannot ride a floor run");
        assert!(err.contains("drop --f01-run"), "{err}");
        let err = args(&[
            "--scene",
            "f01_late",
            "--f01-room",
            "boiler",
            "--rest-pilot",
            "donut",
        ])
        .expect_err("needs the refuge");
        assert!(err.contains("needs a quiet room"), "{err}");
        // A pilot that does not fit the room is refused, a fitting one parses.
        let err = args(&[
            "--scene",
            "f01_late",
            "--f01-room",
            "rest_camp",
            "--rest-pilot",
            "donut",
        ])
        .expect_err("the shelf pilot needs the refuge");
        assert!(err.contains("does not fit"), "{err}");
        assert!(
            args(&[
                "--scene",
                "f01_late",
                "--f01-room",
                "rest_camp",
                "--rest-pilot",
                "walk",
            ])
            .is_ok()
        );
        assert!(
            args(&[
                "--scene",
                "f01_late",
                "--f01-room",
                "rest_aftermath",
                "--rest-pilot",
                "blink",
            ])
            .is_ok()
        );
        assert!(
            args(&[
                "--scene",
                "f01_late",
                "--f01-room",
                "rest_refuge",
                "--rest-pilot",
                "nope"
            ])
            .is_err()
        );
    }

    /// ACT1-A: the two Act I rooms (the first rooms of the run, before the corral) are listed
    /// rooms of `--scene f01_late` (no scene arm of their own). They imply `cf02_rules`, take a
    /// seed, record under the late-Floor-1 source with their own fixture names, stamp their own
    /// room pins and the manifest's, start on the verifier's constructor
    /// (`f01_late::act1_start`), keep Donut locked under the Story loadout (the biscuit flag is
    /// set only by a flag that implies it) and a floor run starts in the shore.
    #[test]
    fn the_act1_rooms_resolve_through_the_late_launcher_and_the_run() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        for room in f01_act1::Act1Room::ALL {
            let id = room.id();
            let late = args(&["--scene", "f01_late", "--f01-room", id]).unwrap();
            assert!(late.f01_late && late.cf02.rules && !late.demo);
            assert_eq!(late.f01_act1, Some(room));
            assert!(late.f01_rest.is_none() && !late.f01_corral && !late.f01_hoarder);
            assert!(late.f01_room_scene() && !late.arena_room());
            assert_eq!(late.f01_room.as_deref(), Some(id));
            assert!(late.carl.prog && late.donut.prog);
            assert!(
                !late.donut_biscuit,
                "Donut starts locked under Story (ACT1-AD13)"
            );
            assert!(late.recording_enabled());
            assert_eq!(
                late.uncontrolled_hero(),
                UncontrolledHero::Companion(ProfileSetId::Cf02Free)
            );
            let all = args(&[
                "--scene",
                "f01_late",
                "--f01-room",
                id,
                "--f01-loadout",
                "all",
            ])
            .unwrap();
            assert!(all.donut_biscuit, "the All loadout implies the biscuit");
            for flag in ["--cf02", "--demo", "--autopilot", "--cf02-engineer"] {
                assert!(
                    args(&["--scene", "f01_late", "--f01-room", id, flag]).is_err(),
                    "{flag}"
                );
            }
            assert!(
                args(&["--scene", room.scene_name()]).is_err(),
                "no scene arm"
            );
            let run = args(&["--scene", "f01_late", "--f01-run", "--f01-room", id]).unwrap();
            assert!(run.f01_run && run.f01_act1 == Some(room));
            let title = match room {
                f01_act1::Act1Room::Chase => "The First Chase",
                f01_act1::Act1Room::Shore => "The Rotten Shore",
                f01_act1::Act1Room::Guild => "The Guild Door",
            };
            assert_eq!(run.f01_hud, Some((title.to_owned(), false)));
            for list in [
                &["--scene", "f01_late", "--f01-room", id][..],
                &[
                    "--scene",
                    "f01_late",
                    "--f01-room",
                    id,
                    "--f01-loadout",
                    "all",
                ][..],
                &["--scene", "f01_late", "--f01-room", id, "--cf02-seed", "7"][..],
            ] {
                let mut run = args(list).unwrap();
                let launched = f01_act1::launch(root, room).unwrap();
                run.fixture = Some(launched.fixture);
                let kit = launched.kit.clone();
                run.act1_run = Some(Box::new(launched));
                let (_, scene, library) =
                    f01_late::act1_start(root, room, 360, run.carl, run.donut, run.cf02.seed)
                        .unwrap();
                let mut hud = new_hud(&run, &kit, &actors);
                let (sim, _, combat, _, _) =
                    build_scene(&kit, &run, &actors, root, &mut hud).unwrap();
                let combat = combat.expect("a combat state");
                assert_eq!(
                    combat.state_hash(&sim),
                    library.state_hash(&scene),
                    "{list:?}"
                );
                assert_eq!(sim.world.state_hash(), scene.world.state_hash(), "{list:?}");
                let party = run.live_party();
                let manifest = recording_manifest(&run, &sim, Some(&combat), &party, root);
                assert_eq!(manifest.reset.source, f01_late::SESSION_SOURCE);
                assert_eq!(manifest.reset.name, room.scene_name());
                assert_eq!(manifest.reset.f01_room.as_deref(), Some(id));
                assert_eq!(manifest.reset.cf02_seed, run.cf02.seed);
                for key in f01_late::SCOPED_PIN_KEYS {
                    assert_eq!(
                        manifest.content_hashes.contains_key(key),
                        f01_late::pin_applies(key, Some(id)),
                        "{key}"
                    );
                }
                assert_eq!(manifest.reset.start_hash, library.state_hash(&scene));
            }
        }
    }

    /// The Act I evidence pilot is a one-room scripted run: it needs an Act I room, fits its own
    /// room and is refused with `--f01-run`.
    #[test]
    fn the_act1_pilot_is_a_one_room_evidence_pilot() {
        let base = ["--scene", "f01_late", "--f01-room"];
        let with = |room: &'static str, pilot: &'static str| {
            let mut v = base.to_vec();
            v.extend([room, "--act1-pilot", pilot]);
            args(&v)
        };
        assert!(with("act1_shore", "strike").unwrap().act1_pilot.is_some());
        assert!(with("act1_guild", "donut").is_ok());
        assert!(with("act1_shore", "walk").is_ok() && with("act1_guild", "walk").is_ok());
        assert!(with("act1_chase", "hide").is_ok() && with("act1_chase", "skip").is_ok());
        assert!(with("act1_chase", "walk").is_ok());
        assert!(
            with("act1_shore", "hide")
                .unwrap_err()
                .contains("does not fit")
        );
        assert!(
            with("act1_chase", "strike")
                .unwrap_err()
                .contains("does not fit")
        );
        assert!(
            with("act1_guild", "strike")
                .unwrap_err()
                .contains("does not fit")
        );
        assert!(
            with("act1_shore", "donut")
                .unwrap_err()
                .contains("does not fit")
        );
        assert!(
            with("hoarder", "walk")
                .unwrap_err()
                .contains("needs an Act I room")
        );
        assert!(with("act1_shore", "nope").is_err());
        let err = args(&[
            "--scene",
            "f01_late",
            "--f01-run",
            "--f01-room",
            "act1_shore",
            "--act1-pilot",
            "walk",
        ])
        .expect_err("a pilot cannot ride a floor run");
        assert!(err.contains("drop --f01-run"), "{err}");
    }

    /// REST-WIRE: the three quiet rooms (the refuge right after the corral, the camp after
    /// the Ring, the aftermath after the Juicer) are listed rooms of `--scene f01_late` (no
    /// scene arm of their own). It implies `cf02_rules`, takes a seed, records
    /// under the late-Floor-1 source with its own fixture name, stamps its own room pins and
    /// the manifest's, starts on the verifier's constructor (`f01_late::rest_start`), and a
    /// floor run walks into it from the corral.
    #[test]
    fn the_rest_rooms_resolve_through_the_late_launcher_and_the_run() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        for room in f01_rest::RestRoom::ALL {
            let id = room.id();
            let late = args(&["--scene", "f01_late", "--f01-room", id]).unwrap();
            assert!(late.f01_late && late.cf02.rules && !late.demo);
            assert_eq!(late.f01_rest, Some(room));
            assert!(late.f01_ring.is_none() && !late.f01_corral && !late.f01_hoarder);
            assert!(late.f01_room_scene() && !late.arena_room());
            assert_eq!(late.f01_room.as_deref(), Some(id));
            assert!(late.carl.prog && late.donut.prog && late.donut_biscuit);
            assert!(late.recording_enabled());
            // The default uncontrolled hero of a room scene is the `cf02_free` companion.
            assert_eq!(
                late.uncontrolled_hero(),
                UncontrolledHero::Companion(ProfileSetId::Cf02Free)
            );
            for flag in ["--cf02", "--demo", "--autopilot", "--cf02-engineer"] {
                assert!(
                    args(&["--scene", "f01_late", "--f01-room", id, flag]).is_err(),
                    "{flag}"
                );
            }
            assert!(
                args(&["--scene", room.scene_name()]).is_err(),
                "no scene arm"
            );
            // The run: each room walks in with its title in the HUD.
            let refuge_run = args(&["--scene", "f01_late", "--f01-run", "--f01-room", id]).unwrap();
            assert!(refuge_run.f01_run && refuge_run.f01_rest == Some(room));
            let title = match room {
                f01_rest::RestRoom::Refuge => "The Wrong Restaurant",
                f01_rest::RestRoom::Camp => "The Survivors' Camp",
                f01_rest::RestRoom::Aftermath => "After the Juicer",
            };
            assert_eq!(refuge_run.f01_hud, Some((title.to_owned(), false)));
            for list in [
                &["--scene", "f01_late", "--f01-room", id][..],
                &[
                    "--scene",
                    "f01_late",
                    "--f01-room",
                    id,
                    "--f01-loadout",
                    "all",
                ][..],
                &["--scene", "f01_late", "--f01-room", id, "--cf02-seed", "7"][..],
            ] {
                let mut run = args(list).unwrap();
                let launched = f01_rest::launch(root, room).unwrap();
                run.fixture = Some(launched.fixture);
                let kit = launched.kit.clone();
                run.rest_run = Some(Box::new(launched));
                let (_, scene, library) =
                    f01_late::rest_start(root, room, 360, run.carl, run.donut, run.cf02.seed)
                        .unwrap();
                let mut hud = new_hud(&run, &kit, &actors);
                let (sim, _, combat, _, _) =
                    build_scene(&kit, &run, &actors, root, &mut hud).unwrap();
                let combat = combat.expect("a combat state");
                assert_eq!(
                    combat.state_hash(&sim),
                    library.state_hash(&scene),
                    "{list:?}"
                );
                assert_eq!(sim.world.state_hash(), scene.world.state_hash(), "{list:?}");
                let party = run.live_party();
                let manifest = recording_manifest(&run, &sim, Some(&combat), &party, root);
                assert_eq!(manifest.reset.source, f01_late::SESSION_SOURCE);
                assert_eq!(manifest.reset.name, room.scene_name());
                assert_eq!(manifest.reset.f01_room.as_deref(), Some(id));
                assert_eq!(manifest.reset.cf02_seed, run.cf02.seed);
                for key in f01_late::SCOPED_PIN_KEYS {
                    assert_eq!(
                        manifest.content_hashes.contains_key(key),
                        f01_late::pin_applies(key, Some(id)),
                        "{key}"
                    );
                }
                assert_eq!(manifest.reset.start_hash, library.state_hash(&scene));
            }
        }
    }

    /// DR-A: room 3 (the Dozer revenge) is a listed room of `--scene f01_late` and has its own
    /// scene arm (`--scene f01_dozer`). Both start on the verifier's constructor
    /// (`f01_late::dozer_start`), record under the late-Floor-1 source, and stamp exactly the
    /// pins scoped to the room (DRA-AD1: the dozer tuning pin is room 3's only).
    #[test]
    fn the_dozer_room_resolves_through_the_late_launcher_and_its_scene() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        for list in [
            &["--scene", "f01_late", "--f01-room", "dozer"][..],
            &[
                "--scene",
                "f01_late",
                "--f01-room",
                "dozer",
                "--f01-loadout",
                "all",
            ][..],
            &["--scene", "f01_dozer"][..],
        ] {
            let mut run = args(list).unwrap();
            assert!(run.f01_dozer && run.f01_room_scene(), "{list:?}");
            assert!(
                !run.f01_boiler && !run.f01_hoarder && !run.f01_hazards && !run.f01_mastery,
                "{list:?}"
            );
            assert!(run.recording_enabled(), "{list:?}");
            let launched = f01_dozer::launch(root).unwrap();
            run.fixture = Some(launched.fixture);
            let kit = launched.kit.clone();
            run.dozer_run = Some(Box::new(launched));
            let (_, scene, library) =
                f01_late::dozer_start(root, 360, run.carl, run.donut, run.cf02.seed).unwrap();
            let mut hud = new_hud(&run, &kit, &actors);
            let (sim, _, combat, _, _) = build_scene(&kit, &run, &actors, root, &mut hud).unwrap();
            let combat = combat.expect("a combat state");
            assert_eq!(
                combat.state_hash(&sim),
                library.state_hash(&scene),
                "{list:?}"
            );
            assert_eq!(sim.world.state_hash(), scene.world.state_hash(), "{list:?}");
            assert!(
                dungeon_floorbreak::dozer_art::dozer_of(&combat).is_some(),
                "{list:?}: the machine"
            );
            let party = run.live_party();
            let manifest = recording_manifest(&run, &sim, Some(&combat), &party, root);
            assert_eq!(manifest.reset.source, f01_late::SESSION_SOURCE, "{list:?}");
            assert_eq!(manifest.reset.name, f01_dozer::SCENE_NAME, "{list:?}");
            for key in f01_late::SCOPED_PIN_KEYS {
                assert_eq!(
                    manifest.content_hashes.contains_key(key),
                    f01_late::pin_applies(key, Some("dozer")),
                    "{list:?} {key}"
                );
            }
            assert_eq!(manifest.reset.start_hash, library.state_hash(&scene));
        }
        // The dozer plans play room 3 only, and room 3 takes only them.
        let smoke = ["--headless-floor01-smoke"];
        let with = |scene: &[&str], plan: &str| {
            let mut list: Vec<&str> = scene.to_vec();
            list.extend_from_slice(&["--room-pilot", plan]);
            list.extend_from_slice(&smoke);
            args(&list)
        };
        let room3 = ["--scene", "f01_late", "--f01-room", "dozer"];
        for plan in [
            "dozer:run",
            "dozer:close_call",
            "dozer:wait",
            "dozer:brace",
            "dozer:retry",
            "dozer:crew",
        ] {
            assert_eq!(
                with(&room3, plan).unwrap().room_pilot.unwrap().label(),
                plan
            );
            assert!(with(&["--scene", "f01_dozer"], plan).is_ok(), "{plan}");
            assert!(
                with(&["--scene", "f01_late", "--f01-room", "boiler"], plan).is_err(),
                "{plan}"
            );
        }
        assert!(with(&room3, "direct").is_err());
        assert!(with(&room3, "dozer:run:2p").is_err());
    }

    /// PT-MASTERY: the closing arena is a listed room of `--scene f01_late` only
    /// (no scene of its own). It implies `cf02_rules` like the other rooms, takes
    /// a seed (its goblins draw temperaments), records under the late-Floor-1
    /// source with its own fixture name, stamps its own room pins, and its tick-0
    /// state in the binary is the verifier's constructor.
    #[test]
    fn the_mastery_arena_resolves_through_the_late_launcher() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let late = args(&["--scene", "f01_late", "--f01-room", "mastery"]).unwrap();
        assert!(late.f01_late && late.f01_mastery && late.cf02.rules && !late.demo);
        assert!(!late.f01_boiler && !late.f01_hoarder && !late.f01_hazards);
        assert!(late.f01_room_scene() && !late.arena_room(), "its own room");
        assert_eq!(late.f01_room.as_deref(), Some("mastery"));
        assert_eq!(late.f01_loadout, Some(f01_late::Loadout::Story));
        assert!(late.carl.prog && late.donut.prog && late.donut_biscuit);
        assert!(late.recording_enabled());
        let all = args(&[
            "--scene",
            "f01_late",
            "--f01-room",
            "mastery",
            "--f01-loadout",
            "all",
        ])
        .unwrap();
        assert!(all.carl.dynamite && all.donut.puddle && !all.carl.prog);
        // The goblins draw temperaments, so the seed is accepted; what the room
        // replaces is refused; there is no scene of that name.
        let seeded = args(&[
            "--scene",
            "f01_late",
            "--f01-room",
            "mastery",
            "--cf02-seed",
            "7",
        ])
        .unwrap();
        assert_eq!(seeded.cf02.seed, Some(7));
        for flag in ["--cf02", "--demo", "--autopilot", "--cf02-engineer"] {
            assert!(
                args(&["--scene", "f01_late", "--f01-room", "mastery", flag]).is_err(),
                "{flag}"
            );
        }
        assert!(args(&["--scene", "f01_mastery"]).is_err());
        // The descriptor and the tick-0 state.
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        for list in [
            &["--scene", "f01_late", "--f01-room", "mastery"][..],
            &[
                "--scene",
                "f01_late",
                "--f01-room",
                "mastery",
                "--f01-loadout",
                "all",
            ][..],
            &[
                "--scene",
                "f01_late",
                "--f01-room",
                "mastery",
                "--cf02-seed",
                "7",
            ][..],
        ] {
            let mut run = args(list).unwrap();
            let launched = f01_mastery::launch(root).unwrap();
            run.fixture = Some(launched.fixture);
            let kit = launched.kit.clone();
            run.mastery_run = Some(Box::new(launched));
            let (_, scene, library) =
                f01_late::mastery_start(root, 360, run.carl, run.donut, run.cf02.seed).unwrap();
            let mut hud = new_hud(&run, &kit, &actors);
            let (sim, _, combat, _, _) = build_scene(&kit, &run, &actors, root, &mut hud).unwrap();
            let combat = combat.expect("a combat state");
            assert_eq!(
                combat.state_hash(&sim),
                library.state_hash(&scene),
                "{list:?}"
            );
            assert_eq!(sim.world.state_hash(), scene.world.state_hash(), "{list:?}");
            let party = run.live_party();
            let manifest = recording_manifest(&run, &sim, Some(&combat), &party, root);
            assert_eq!(manifest.reset.source, f01_late::SESSION_SOURCE);
            assert_eq!(manifest.reset.name, "f01_mastery");
            assert_eq!(manifest.reset.f01_room.as_deref(), Some("mastery"));
            assert_eq!(manifest.reset.cf02_seed, run.cf02.seed);
            assert!(manifest.reset.cf02_rules && manifest.reset.donut_biscuit);
            for key in f01_late::SCOPED_PIN_KEYS {
                assert_eq!(
                    manifest.content_hashes.contains_key(key),
                    f01_late::pin_applies(key, Some("mastery")),
                    "{key}"
                );
            }
            assert_eq!(manifest.reset.start_hash, library.state_hash(&scene));
        }
    }

    /// The binary's start state of each slot room is the one the verifier's
    /// constructors build (the same library builders): `build_scene` builds nothing of its own for them and runs
    /// no builder after the slot (a reset after the slot would mark the Hoarder
    /// as retried), so the smoke's tick-0 hash is the verifier's.
    #[test]
    fn the_slot_rooms_binary_state_is_the_verifiers_constructor() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        // The Story loadout, the TEST LOADOUT and (hazard room) a seed (review of #229).
        let cases: [(&str, &[&str]); 5] = [
            ("hoarder", &[]),
            ("hoarder", &["--f01-loadout", "all"]),
            ("hazards", &[]),
            ("hazards", &["--f01-loadout", "all"]),
            ("hazards", &["--cf02-seed", "7"]),
        ];
        for (room, extra) in cases {
            let mut list = vec!["--scene", "f01_late", "--f01-room", room];
            list.extend_from_slice(extra);
            let mut run = args(&list).unwrap();
            let (library, scene_hash) = if room == "hoarder" {
                let launched = f01_hoarder::launch(root).unwrap();
                run.fixture = Some(launched.fixture);
                run.hoarder_run = Some(Box::new(launched));
                let (_, scene, combat) =
                    f01_late::hoarder_start(root, 360, run.carl, run.donut).unwrap();
                (combat.state_hash(&scene), scene.world.state_hash())
            } else {
                let launched = f01_hazards::launch(root).unwrap();
                run.fixture = Some(launched.fixture);
                run.hazards_run = Some(Box::new(launched));
                let (_, scene, combat) =
                    f01_late::hazards_start(root, 360, run.carl, run.donut, run.cf02.seed).unwrap();
                (combat.state_hash(&scene), scene.world.state_hash())
            };
            let kit = run
                .hoarder_run
                .as_ref()
                .map(|r| r.kit.clone())
                .or_else(|| run.hazards_run.as_ref().map(|r| r.kit.clone()))
                .unwrap();
            let mut hud = new_hud(&run, &kit, &actors);
            let (sim, _, combat, _, _) = build_scene(&kit, &run, &actors, root, &mut hud).unwrap();
            let combat = combat.expect("a combat state");
            assert_eq!(combat.state_hash(&sim), library, "{room} {extra:?}");
            assert_eq!(sim.world.state_hash(), scene_hash, "{room} {extra:?}");
            assert!(!combat.slot_enemies().is_empty(), "{room} {extra:?}");
        }
    }

    /// F01-RUN: the relaunch chain of a floor run, like the lab's: `--f01-run`
    /// starts at the first listed room, and each relaunch's arguments parse into
    /// the next listed room with the ledger and the tally, until the floor end. The
    /// HUD's TEST LOADOUT tag is off along the story chain and on under `all`.
    #[test]
    fn the_floor_run_relaunch_chain_walks_every_listed_room() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest = f01_late::Manifest::load(root).unwrap();
        let snapshot = dungeon_floorbreak::f01_late_fixtures::hoarder_defeat(root)
            .unwrap()
            .snapshot()
            .unwrap();
        for (extra, carried) in [(&[][..], true), (&["--f01-loadout", "all"][..], false)] {
            let mut argv: Vec<String> = ["floor01_scene", "--scene", "f01_late", "--f01-run"]
                .iter()
                .chain(extra)
                .chain(&["--scale", "3", "--no-record"])
                .map(|a| (*a).to_owned())
                .collect();
            let mut visited = Vec::new();
            let mut tally = f01_run::RunTally::default();
            loop {
                let parsed = parse_args_with(&argv, HudSettings::default())
                    .unwrap_or_else(|e| panic!("{argv:?}: {e}"));
                assert!(parsed.f01_run && parsed.manual && parsed.scale == 3);
                let room = parsed.f01_room.clone().unwrap();
                let (title, tag) = parsed.f01_hud.clone().unwrap();
                assert_eq!(title, manifest.room(&room).unwrap().title);
                // The TEST LOADOUT tag is the `all` loadout's (U5), the one that carries nothing.
                assert_eq!(tag, !carried, "{room} {extra:?}");
                assert_eq!(parsed.f01_carry.is_some(), carried && !visited.is_empty());
                assert_eq!(parsed.f01_tally, (!visited.is_empty()).then_some(tally));
                visited.push(room.clone());
                match f01_run::next_room(&manifest, &room).unwrap() {
                    f01_run::NextStep::Room(row) => {
                        tally.rooms += 1;
                        tally.ticks += 600;
                        let carry = carried.then_some(&snapshot);
                        argv = f01_run::relaunch_argv(&argv, &row.id, carry, tally, None);
                    }
                    f01_run::NextStep::FloorEnd => break,
                }
            }
            assert_eq!(visited, manifest.listed(), "{extra:?}");
        }
        // A bare `--scene f01_late` is unchanged: the Boiler, no run; a story room
        // launched from its pin is the story state, so it has no TEST LOADOUT tag.
        let bare = args(&["--scene", "f01_late"]).unwrap();
        assert!(!bare.f01_run && bare.f01_room.as_deref() == Some("boiler"));
        assert_eq!(bare.f01_hud, Some(("Goblin Boiler".to_owned(), false)));
        let mid = args(&["--scene", "f01_late", "--f01-run", "--f01-room", "juicer"]).unwrap();
        assert!(!mid.f01_hud.unwrap().1);
        let all = args(&[
            "--scene",
            "f01_late",
            "--f01-room",
            "juicer",
            "--f01-loadout",
            "all",
        ])
        .unwrap();
        assert!(all.f01_hud.unwrap().1);
    }

    /// F01-RUN: the run's flags are refused outside their place.
    #[test]
    fn the_floor_run_flags_are_refused_outside_a_run() {
        let carry = f01_run::encode_carry(
            &dungeon_floorbreak::f01_late_fixtures::hoarder_defeat(Path::new(env!(
                "CARGO_MANIFEST_DIR"
            )))
            .unwrap()
            .snapshot()
            .unwrap(),
        );
        for list in [
            &["--f01-run"][..],
            &["--cf02", "--f01-run"],
            &["--scene", "f01_boiler", "--f01-run"],
            &["--scene", "f01_late", "--f01-run-tally", "1:0:0:0"],
            &["--scene", "f01_late", "--f01-carry", carry.as_str()],
            &["--scene", "f01_late", "--f01-run", "--cf02-seed", "3"],
            &[
                "--scene",
                "f01_late",
                "--f01-run",
                "--f01-loadout",
                "all",
                "--f01-carry",
                carry.as_str(),
            ],
            &["--scene", "f01_late", "--f01-run", "--f01-carry", "{"],
            &["--scene", "f01_late", "--f01-run", "--f01-run-tally", "x"],
        ] {
            assert!(args(list).is_err(), "{list:?}");
        }
        let ok = args(&[
            "--scene",
            "f01_late",
            "--f01-run",
            "--f01-carry",
            carry.as_str(),
        ])
        .unwrap();
        // ACT1-F: the first listed room is the chase.
        assert_eq!(ok.f01_room.as_deref(), Some("act1_chase"));
    }

    /// F01-RUN: a carried room's tick-0 state is the verifier's (the room's
    /// constructor, then `carry_story` with the descriptor's `f01_carry`), in the
    /// hazard room, the Boiler (its extra `with_demo`/`with_env` builders) and the
    /// Juicer (the generic boss path): the binary's carried build equals its own
    /// pinned build carried afterwards, and the library constructor carried.
    #[test]
    fn a_carried_rooms_binary_state_is_the_verifiers_constructor() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        let snapshot = dungeon_floorbreak::f01_late_fixtures::hoarder_defeat(root)
            .unwrap()
            .snapshot()
            .unwrap();
        let carry = f01_run::encode_carry(&snapshot);
        let build = |room: &str, carried: bool| {
            let mut list = vec!["--scene", "f01_late", "--f01-run", "--f01-room", room];
            if carried {
                list.extend([
                    "--f01-carry",
                    carry.as_str(),
                    "--f01-run-tally",
                    "2:1200:0:1",
                ]);
            }
            let mut run = args(&list).unwrap();
            let kit = match room {
                "hazards" => {
                    let launched = f01_hazards::launch(root).unwrap();
                    run.fixture = Some(launched.fixture);
                    let kit = launched.kit.clone();
                    run.hazards_run = Some(Box::new(launched));
                    kit
                }
                "boiler" => {
                    let launched = f01_boiler::launch_with(root, f01_boiler::Roster::Live).unwrap();
                    run.fixture = Some(launched.fixture);
                    let kit = launched.kit.clone();
                    run.boiler_run = Some(Box::new(launched));
                    kit
                }
                _ => boss_scene::load_arena(root).unwrap(),
            };
            let mut hud = new_hud(&run, &kit, &actors);
            let (sim, _, combat, _, _) = build_scene(&kit, &run, &actors, root, &mut hud).unwrap();
            (run, sim, combat.expect("a combat state"), hud)
        };
        for room in ["hazards", "boiler", "juicer"] {
            let (run, sim, combat, hud) = build(room, true);
            let (_, pinned_sim, mut pinned, _) = build(room, false);
            let pinned_hash = pinned.state_hash(&pinned_sim);
            pinned.carry_story(&snapshot).unwrap();
            assert_eq!(
                combat.state_hash(&sim),
                pinned.state_hash(&pinned_sim),
                "{room}"
            );
            assert_ne!(
                pinned_hash,
                combat.state_hash(&sim),
                "{room}: the receipt is hashed"
            );
            // The library constructor the verifier calls, then the carry.
            let library = match room {
                "hazards" => Some(
                    f01_late::hazards_start(root, 360, run.carl, run.donut, None)
                        .map(|(_, scene, combat)| (scene, combat)),
                ),
                "boiler" => Some(
                    f01_late::boiler_start_with(
                        root,
                        360,
                        run.carl,
                        run.donut,
                        None,
                        f01_boiler::Roster::Live,
                    )
                    .map(|(_, scene, combat)| (scene, combat)),
                ),
                _ => None,
            };
            if let Some(built) = library {
                let (scene, mut lib) = built.unwrap();
                lib.carry_story(&snapshot).unwrap();
                assert_eq!(combat.state_hash(&sim), lib.state_hash(&scene), "{room}");
            }
            // The descriptor carries it; the HUD shows the room and no tag.
            let manifest = recording_manifest(&run, &sim, Some(&combat), &run.live_party(), root);
            assert_eq!(manifest.reset.f01_carry.as_ref(), Some(&snapshot), "{room}");
            assert!(!hud.snapshot().loadout_tag, "{room}");
        }
        let (_, _, _, hud) = build("hazards", true);
        assert_eq!(
            hud.snapshot().location.as_deref(),
            Some("SLIMES AND ROT STICKERS \u{b7} TRY 1")
        );
    }

    /// PT-SCENE: `--scene f01_late` resolves a room and a loadout into the scene
    /// that runs the room and the loadout's flags.
    #[test]
    fn the_f01_late_scene_resolves_rooms_and_loadouts() {
        // The default room is the first listed one, under the story loadout.
        let late = args(&["--scene", "f01_late"]).unwrap();
        assert!(late.f01_late && late.f01_boiler && late.cf02.rules && !late.demo);
        assert_eq!(late.f01_room.as_deref(), Some("boiler"));
        assert_eq!(late.f01_loadout, Some(f01_late::Loadout::Story));
        assert!(
            late.carl.prog && late.donut.prog,
            "story implies both programs"
        );
        assert!(!late.carl.dynamite && !late.donut.puddle);
        assert!(late.manual, "a live run is the player's");
        // The same room by name equals the default.
        let boiler = args(&["--scene", "f01_late", "--f01-room", "boiler"]).unwrap();
        assert!(boiler.f01_boiler && boiler.carl.prog);
        // The Juicer is the boss scene under the loadout.
        let juicer = args(&["--scene", "f01_late", "--f01-room", "juicer"]).unwrap();
        assert!(juicer.boss && !juicer.f01_boiler && !juicer.cf02.rules);
        assert_eq!(juicer.fixture, Some(boss_scene::BOSS01_SCENE));
        assert_eq!(juicer.f01_room.as_deref(), Some("juicer"));
        assert!(juicer.carl.prog && juicer.donut.prog);
        // `all`: the debug start flags, the story programs off.
        let all = args(&["--scene", "f01_late", "--f01-loadout", "all"]).unwrap();
        assert!(all.carl.dynamite && all.carl.gauntlet && all.donut.puddle);
        assert!(!all.carl.prog && !all.donut.prog, "story triggers off");
        assert!(all.donut_biscuit, "the puddle implies the biscuit");
        let all_juicer = args(&[
            "--scene",
            "f01_late",
            "--f01-room",
            "juicer",
            "--f01-loadout",
            "all",
        ])
        .unwrap();
        assert!(all_juicer.boss && !all_juicer.carl.prog);
        // A scripted run is not the player's.
        assert!(
            !args(&["--scene", "f01_late", "--headless-floor01-smoke"])
                .unwrap()
                .manual
        );
        // The story setup is each room's own pin.
        assert_eq!(
            story_setup(&juicer).holds().len(),
            2,
            "the Juicer's two pins"
        );
        assert_eq!(
            story_setup(&juicer).trigger_ids(),
            ["brute_boss_f01_defeated"]
        );
        assert!(story_setup(&all_juicer).trigger_ids().is_empty());
        assert_eq!(story_setup(&all_juicer).holds().len(), 4);
    }

    /// PT-SCENE acceptance: each refusal has a test.
    #[test]
    fn the_f01_late_scene_refuses_what_it_replaces() {
        let refused = |extra: &[&str]| {
            let mut list = vec!["--scene", "f01_late"];
            list.extend_from_slice(extra);
            args(&list).expect_err(&format!("{list:?}"))
        };
        for extra in [
            &["--cf02"][..],
            &["--cf02-engineer"],
            &["--demo"],
            &["--scenario", "walk_right"],
            &["--fixture", "cp01_kick"],
            &["--party-fixture", "duo_independent_moves"],
            &["--reference-frame"],
            &["--carl-dynamite"],
            &["--carl-gauntlet"],
            &["--donut-light-feet"],
            &["--donut-puddle"],
            &["--autopilot"],
            &["--demo-beat", "2"],
            &["--camera-test-room", "1"],
        ] {
            refused(extra);
        }
        // `--cf02` first is refused too, and the scenes are exclusive.
        assert!(args(&["--cf02", "--scene", "f01_late"]).is_err());
        for other in ["boss_f01", "combat_lab", "f01_boiler"] {
            assert!(args(&["--scene", "f01_late", "--scene", other]).is_err());
            assert!(args(&["--scene", other, "--scene", "f01_late"]).is_err());
        }
        // The story programs are the loadout's: `all` refuses them.
        for extra in [&["--carl-prog"][..], &["--donut-prog"]] {
            let mut list = vec!["--scene", "f01_late", "--f01-loadout", "all"];
            list.extend_from_slice(extra);
            assert!(args(&list).is_err(), "{list:?}");
        }
        // The room and loadout flags need the scene.
        for list in [
            &["--f01-room", "boiler"][..],
            &["--f01-loadout", "story"],
            &["--scene", "boss_f01", "--f01-room", "juicer"],
            &["--scene", "f01_boiler", "--f01-loadout", "all"],
            &["--cf02", "--f01-room", "boiler"],
        ] {
            let error = args(list).expect_err(&format!("{list:?}"));
            assert!(error.contains("need --scene f01_late"), "{error}");
        }
        // An unknown or unlisted room names the listed rooms; a bad loadout fails.
        let error = refused(&["--f01-room", "cellar"]);
        assert!(
            error.contains("unknown room")
                && error.contains(
                    "corral, rest_refuge, hoarder, boiler, dozer, hazards, ring_court, ring_split, rest_camp, juicer, rest_aftermath, mastery"
                ),
            "{error}"
        );
        // CORRAL-F listed the corral and DR-A the dozer, so no story-order room is refused
        // as unlisted any more (`f01_late`'s tests cover that refusal with a made-up room).
        assert!(refused(&["--f01-loadout", "everything"]).contains("unknown loadout"));
        assert!(refused(&["--f01-room"]).contains("needs a room id"));
        // Other scenes are unchanged: no room label on the legacy runs.
        assert!(!args(&["--scene", "boss_f01"]).unwrap().f01_late);
    }

    /// PT-GATE-C1: an aligned room pilot (`mastery|juicer:WHO:SEED`) run through
    /// `build_scene` (the presim of every headless smoke, capture and window run)
    /// ends on the library fixture's final hash: the same party, seed and inputs.
    /// The plain `mastery` plan is `carl_solo` at seed 0.
    #[test]
    fn an_aligned_room_pilot_in_the_scene_ends_on_the_library_fixture_hash() {
        use dungeon_floorbreak::f01_late_fixtures::{JuicerWho, juicer_pilot};
        use dungeon_floorbreak::f01_mastery_fixtures::{self as mastery, Who};
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, false, false).unwrap();
        let cases: [(&str, &str, u64); 15] = [
            ("mastery", "carl_solo", 0),
            ("mastery", "donut_by_swap", 3),
            ("mastery", "donut_fights", 1),
            ("mastery", "companion", 5),
            ("mastery", "plain", 0),
            ("juicer", "carl_solo", 0),
            ("juicer", "donut_by_swap", 1),
            ("juicer", "companion", 2),
            ("juicer", "carl_solo", 7),
            // PT-GATE-E: the live room 4 and room 2 library pilots (the Boiler's token carries
            // its plan).
            ("hazards", "carl_solo", 0),
            ("hazards", "donut_by_swap", 4),
            ("hazards", "companion", 7),
            ("boiler", "carl_solo:direct", 0),
            ("boiler", "donut_by_swap:both", 3),
            ("boiler", "companion:both", 8),
        ];
        for (plan, who, seed) in cases {
            let (library_hash, ticks) = if plan == "hazards" {
                use dungeon_floorbreak::hazards_fixtures::pilots as hazards;
                let who = match who {
                    "donut_by_swap" => hazards::Who::DonutBySwap,
                    "companion" => hazards::Who::Companion,
                    _ => hazards::Who::CarlSolo,
                };
                let report = hazards::run(root, who, seed, true).unwrap();
                (report.hash, report.ticks)
            } else if plan == "boiler" {
                use dungeon_floorbreak::boiler_fixtures::scene_pilots as boiler;
                let (who, route) = who.split_once(':').unwrap();
                let who = match who {
                    "donut_by_swap" => boiler::Who::DonutBySwap,
                    "companion" => boiler::Who::Companion,
                    _ => boiler::Who::CarlSolo,
                };
                let route = if route == "direct" {
                    boiler::Plan::Direct
                } else {
                    boiler::Plan::Both
                };
                let report = boiler::run_live(root, who, route, seed).unwrap();
                (report.hash, report.exit_tick.unwrap())
            } else if plan == "mastery" {
                let who = match who {
                    "donut_by_swap" => Who::DonutBySwap,
                    "donut_fights" => Who::DonutFights,
                    "companion" => Who::Companion,
                    _ => Who::CarlSolo,
                };
                let report = mastery::run(root, who, seed).unwrap();
                (report.hash, report.ticks)
            } else {
                let who = match who {
                    "donut_by_swap" => JuicerWho::DonutBySwap,
                    "companion" => JuicerWho::Companion,
                    _ => JuicerWho::CarlSolo,
                };
                let report = juicer_pilot(root, who, seed).unwrap();
                (report.hash, report.ticks)
            };
            let pilot = if who == "plain" {
                plan.to_owned()
            } else {
                format!("{plan}:{who}:{seed}")
            };
            let ticks = ticks.to_string();
            let mut run = args(&[
                "--scene",
                "f01_late",
                "--f01-room",
                plan,
                "--room-pilot",
                &pilot,
                "--headless-floor01-smoke",
                "--capture-tick",
                &ticks,
            ])
            .unwrap();
            let label = format!("{plan} {who} {seed}");
            let kit = if plan == "mastery" {
                let launched = f01_mastery::launch(root).unwrap();
                run.fixture = Some(launched.fixture);
                let kit = launched.kit.clone();
                run.mastery_run = Some(Box::new(launched));
                kit
            } else if plan == "hazards" {
                let launched = f01_hazards::launch(root).unwrap();
                run.fixture = Some(launched.fixture);
                let kit = launched.kit.clone();
                run.hazards_run = Some(Box::new(launched));
                kit
            } else if plan == "boiler" {
                let launched = f01_boiler::launch_with(root, f01_boiler::Roster::Live).unwrap();
                run.fixture = Some(launched.fixture);
                let kit = launched.kit.clone();
                run.boiler_run = Some(Box::new(launched));
                kit
            } else {
                boss_scene::load_arena(root).unwrap()
            };
            let mut hud = new_hud(&run, &kit, &actors);
            let (sim, _, combat, _, party) =
                build_scene(&kit, &run, &actors, root, &mut hud).unwrap();
            let combat = combat.expect("a combat state");
            assert_eq!(
                party_scene::party_hash(&combat, &sim, &party),
                library_hash,
                "{label}"
            );
        }
    }

    /// PT-SCENE: the recorded descriptor names the room and the loadout, the
    /// Boiler has its own replay source, and the Juicer records as `boss`.
    #[test]
    fn the_f01_late_descriptor_names_the_room_and_the_loadout() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest_of = |list: &[&str]| {
            let run = args(list).unwrap();
            let kit = if run.f01_boiler {
                f01_boiler::launch(root).unwrap().kit
            } else if run.f01_hoarder {
                f01_hoarder::launch(root).unwrap().kit
            } else if run.f01_hazards {
                f01_hazards::launch(root).unwrap().kit
            } else {
                boss_scene::load_arena(root).unwrap()
            };
            let sim = SceneSim::new_at(&kit, 360, 112).unwrap();
            let party = run.live_party();
            recording_manifest(&run, &sim, None, &party, root)
        };
        let boiler = manifest_of(&["--scene", "f01_late", "--f01-room", "boiler"]);
        assert_eq!(boiler.reset.source, f01_late::SESSION_SOURCE);
        assert_eq!(boiler.reset.f01_room.as_deref(), Some("boiler"));
        assert_eq!(boiler.reset.f01_loadout.as_deref(), Some("story"));
        assert!(boiler.reset.carl_prog && boiler.reset.donut_prog && boiler.reset.cf02_rules);
        assert_eq!(
            boiler.feature_flags.get("f01_room").map(String::as_str),
            Some("boiler")
        );
        let juicer = manifest_of(&[
            "--scene",
            "f01_late",
            "--f01-room",
            "juicer",
            "--f01-loadout",
            "all",
        ]);
        assert_eq!(juicer.reset.source, "boss");
        assert_eq!(juicer.reset.f01_room.as_deref(), Some("juicer"));
        assert_eq!(juicer.reset.f01_loadout.as_deref(), Some("all"));
        assert!(juicer.reset.carl_dynamite && juicer.reset.donut_puddle && !juicer.reset.carl_prog);
        // HRD-A2b and HAZ-A2b: the two rooms record under the Boiler's source with
        // their own fixture names (the names the verifier's source rule binds).
        for (room, name) in [("hoarder", "f01_hoarder"), ("hazards", "f01_hazards")] {
            let m = manifest_of(&["--scene", "f01_late", "--f01-room", room]);
            assert_eq!(m.reset.source, f01_late::SESSION_SOURCE, "{room}");
            assert_eq!(m.reset.name, name, "{room}");
            assert_eq!(m.reset.f01_room.as_deref(), Some(room));
            assert_eq!(m.reset.f01_loadout.as_deref(), Some("story"));
            assert!(m.reset.carl_prog && m.reset.donut_prog && m.reset.cf02_rules);
            assert!(m.reset.donut_biscuit && m.reset.cf02_seed.is_none());
            assert_eq!(
                m.feature_flags.get("f01_room").map(String::as_str),
                Some(room)
            );
            // Her room pins are stamped, the Boiler's are not (scoped by room).
            assert!(m.content_hashes.contains_key(if room == "hoarder" {
                f01_hoarder::ROOM_PIN_KEY
            } else {
                f01_hazards::ROOM_PIN_KEY
            }));
            assert!(!m.content_hashes.contains_key(f01_boiler::ROOM_PIN_KEY));
        }
        // The Hoarder's own scene (`--scene f01_hoarder`) still records, with her room name
        // and no loadout label and no story programs.
        let own = manifest_of(&["--scene", "f01_hoarder"]);
        assert_eq!(own.reset.f01_room.as_deref(), Some("hoarder"));
        assert_eq!(own.reset.f01_loadout, None);
        assert!(own.reset.cf02_rules && !own.reset.carl_prog);
        // A direct Boiler launch names the room and no loadout; a legacy run neither.
        let direct = manifest_of(&["--scene", "f01_boiler", "--carl-prog"]);
        assert_eq!(direct.reset.f01_room.as_deref(), Some("boiler"));
        assert_eq!(direct.reset.f01_loadout, None);
        let boss = manifest_of(&["--scene", "boss_f01"]);
        assert_eq!((boss.reset.f01_room, boss.reset.f01_loadout), (None, None));
        // The binary's start state is the library constructor the verifier calls,
        // for the story launcher's **live** roster and in the binary's own order
        // (the env first, the CF02 rules after it, as `build_scene` does; the
        // library applies the rules before the env), so the order is pinned too.
        let run = args(&["--scene", "f01_late"]).unwrap();
        let roster = f01_boiler::Roster::for_loadout(run.f01_loadout.map(|l| l.name()));
        assert_eq!(roster, f01_boiler::Roster::Live);
        let launched = f01_boiler::launch_with(root, roster).unwrap();
        let mut scene = SceneSim::new_at(&launched.kit, 360, launched.fixture.carl_x).unwrap();
        let mut with_run = run.clone();
        with_run.boiler_run = Some(Box::new(launched.clone()));
        let mut by_args = CombatState::new(launched.fixture, &launched.kit, &scene)
            .unwrap()
            .with_donut(donut_setup(&with_run))
            .try_with_story(story_setup(&with_run))
            .unwrap()
            .with_demo(launched.rules)
            .with_env(f01_boiler::env_runtime(&launched).unwrap());
        by_args.sync_env(&mut scene);
        let by_args = by_args.with_cf02().with_cf02_seed(run.cf02.seed);
        let by_lib =
            f01_late::boiler_combat(&launched, &mut scene, run.carl, run.donut, run.cf02.seed)
                .unwrap();
        assert_eq!(by_args.enemies.len(), 4, "the live roster is in the state");
        assert_eq!(by_args.enemies.len(), by_lib.enemies.len());
        assert_eq!(by_args.state_hash(&scene), by_lib.state_hash(&scene));
        // And through a stepped run, so a later divergence of the order shows.
        let (mut a, mut b) = (by_args, by_lib);
        let (mut sa, mut sb) = (scene.clone(), scene);
        for tick in 1..=120u64 {
            let raw = launched.fixture.input(tick);
            a.step_party(&mut sa, [raw, RawInput::default()]);
            b.step_party(&mut sb, [raw, RawInput::default()]);
        }
        assert_eq!(a.state_hash(&sa), b.state_hash(&sb));
    }

    #[test]
    fn the_boiler_story_pin_applies_only_under_the_program_flags() {
        let launched = |list: &[&str]| {
            let mut run = args(list).unwrap();
            let launched = f01_boiler::launch(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
            run.fixture = Some(launched.fixture);
            run.boiler_run = Some(Box::new(launched));
            run
        };
        let plain = launched(&["--scene", "f01_boiler"]);
        assert!(story_setup(&plain).is_inert(), "no flag: nothing held");
        let carl = launched(&["--scene", "f01_boiler", "--carl-prog"]);
        assert_eq!(
            story_setup(&carl).holds(),
            [dungeon_floorbreak::story_unlock::SPECIAL_THROWN_CHARGE],
            "Carl holds the post-Hoarder dynamite"
        );
        assert!(
            story_setup(&carl).trigger_ids().is_empty(),
            "no trigger armed"
        );
        let donut = launched(&["--scene", "f01_boiler", "--donut-prog"]);
        assert_eq!(
            story_setup(&donut).holds(),
            [dungeon_floorbreak::donut_prog::JUMP_REACH_BOOST]
        );
        let both = launched(&["--scene", "f01_boiler", "--carl-prog", "--donut-prog"]);
        assert_eq!(story_setup(&both).holds().len(), 2);
        // The combat state accepts the pin (the biscuit is held in this room).
        let kit = &both.boiler_run.as_ref().unwrap().kit;
        let scene = SceneSim::new_at(kit, 360, 112).unwrap();
        assert!(
            CombatState::new(both.fixture.unwrap(), kit, &scene)
                .unwrap()
                .with_donut(donut_setup(&both))
                .try_with_story(story_setup(&both))
                .is_ok()
        );
    }

    /// DONUT-PROG-01 DP-A (DP-Q06, DP-Q10): the three flags parse, the two
    /// start flags imply the biscuit, they are refused with the lab, both
    /// programs go through one setup, and a scripted fixture never reads them.
    #[test]
    fn the_donut_flags_parse_imply_the_biscuit_are_refused_with_the_lab_and_share_one_setup() {
        let plain = args(&["--cf02"]).unwrap();
        assert!(!plain.donut.any());
        let prog = args(&["--donut-prog", "--scene", "boss_f01"]).unwrap();
        assert!(prog.donut.prog && !prog.donut.light_feet && !prog.donut.puddle);
        assert!(!prog.carl.any());
        assert_eq!(
            story_setup(&prog).holds(),
            [dungeon_floorbreak::donut_prog::JUMP_REACH_BOOST],
            "Donut's boss pin"
        );
        assert_eq!(
            story_setup(&prog).trigger_ids(),
            ["brute_boss_f01_defeated"]
        );
        // `--donut-prog` alone holds and arms nothing on the demo hall or the arena.
        for extra in [&["--demo"][..], &["--cf02"][..]] {
            let mut list = vec!["--donut-prog"];
            list.extend_from_slice(extra);
            assert!(story_setup(&args(&list).unwrap()).is_inert(), "{list:?}");
        }
        // Both programs: one setup, the union of the pins.
        let both = args(&["--carl-prog", "--donut-prog", "--scene", "boss_f01"]).unwrap();
        assert_eq!(story_setup(&both).holds().len(), 2);
        assert_eq!(story_setup(&both).grant_heroes(), Some(&[0u8, 1][..]));
        // The start flags imply the biscuit and, for the blink, the boost.
        assert!(
            !prog.donut_biscuit,
            "`--donut-prog` alone: the biscuit comes from the story"
        );
        let light = args(&["--cf02", "--donut-light-feet"]).unwrap();
        assert!(light.donut.light_feet && light.donut_biscuit);
        let puddle = args(&["--cf02", "--donut-puddle"]).unwrap();
        assert!(puddle.donut.puddle && puddle.donut_biscuit);
        assert_eq!(
            story_setup(&puddle).holds().len(),
            2,
            "the blink and the boost"
        );
        // A scripted fixture declares its own pin and ignores every flag.
        let fixture = args(&["--fixture", "cp01_kick", "--donut-puddle", "--donut-prog"]).unwrap();
        assert!(story_setup(&fixture).is_inert());
        let party = args(&[
            "--headless-floor01-smoke",
            "--party-fixture",
            "donut_prog_boss_pins",
            "--donut-puddle",
        ])
        .unwrap();
        assert_eq!(
            story_setup(&party).holds(),
            [dungeon_floorbreak::donut_prog::JUMP_REACH_BOOST],
            "its own pin, not the flag"
        );
        for flag in ["--donut-prog", "--donut-light-feet", "--donut-puddle"] {
            let error = args(&["--scene", "combat_lab", flag]).expect_err(flag);
            assert!(
                error.contains("refused with --scene combat_lab"),
                "{flag}: {error}"
            );
        }
        assert!(list_fixtures("donut_prog_").contains(&"donut_prog_story_unlocks"));
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

    /// AREA-REFUGE-01: `--scene refuge_guild` is a fixed review pose in its own room.
    #[test]
    fn refuge_guild_scene_is_a_review_pose_and_its_own_scene() {
        let refuge = args(&["--scene", "refuge_guild"]).unwrap();
        assert!(refuge.refuge && refuge.reference_frame);
        assert!(!refuge.f01_late && !refuge.f01_boiler && !refuge.boss && !refuge.demo);
        assert!(refuge.f01_room.is_none() && refuge.fixture.is_none());
        // Never recorded, never live input: a review pose.
        assert!(!refuge.live_input() && !refuge.recording_enabled());
        // The deck framing is the layout's 640x400 view.
        assert!(args(&["--scene", "refuge_guild", "--deck"]).unwrap().deck);
        // Exclusive with the other scenes, and refuses the modes that pick another room or script.
        for other in ["boss_f01", "combat_lab", "f01_boiler", "f01_late"] {
            assert!(args(&["--scene", "refuge_guild", "--scene", other]).is_err());
            assert!(args(&["--scene", other, "--scene", "refuge_guild"]).is_err());
        }
        for flags in [
            &["--demo"][..],
            &["--cf02"],
            &["--reference-frame"],
            &["--scenario", "walk_right"],
            &["--fixture", "cp01_kick"],
            &["--f01-room", "boiler"],
            &["--capture-tick", "10"],
        ] {
            let mut list = vec!["--scene", "refuge_guild"];
            list.extend_from_slice(flags);
            assert!(args(&list).is_err(), "{flags:?}");
        }
        assert!(
            args(&["--scene", "refuge_guild", "--refuge-empty"])
                .unwrap()
                .refuge_empty
        );
        assert!(args(&["--refuge-empty"]).is_err());
        // The f01_late manifest is untouched: the room is not one of its rooms.
        assert!(args(&["--scene", "f01_late", "--f01-room", "refuge_guild"]).is_err());
    }

    /// The room loads through the scene's own loader and its view names are the
    /// ones the scene asks the layout for.
    #[test]
    fn refuge_guild_room_exposes_the_reference_views() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let (kit, room) = refuge::load(root).unwrap();
        assert!(kit.view(REFERENCE_VIEW).is_some() && kit.view(REFERENCE_VIEW_DECK).is_some());
        assert_eq!(kit.view("refuge_640x360"), kit.view(REFERENCE_VIEW));
        assert_eq!(room.cast_x().carl, 258.0);
    }

    // BUG-SPECIAL-INPUT: the equipped special was unreachable from live input
    // (no keyboard key and no pad East button reached `RawInput::ability`).

    /// East (Circle/B) is the EquippedSpecial and nothing else; the other
    /// buttons keep their bindings (North stays the kick, not a special).
    #[test]
    fn the_pad_maps_east_to_the_equipped_special_and_only_east() {
        let only = |button: GamepadButton| pad_buttons(|b| b == button);
        assert_eq!(
            only(GamepadButton::East),
            PadButtons {
                ability: true,
                ..PadButtons::default()
            }
        );
        assert!(only(GamepadButton::South).jump && !only(GamepadButton::South).ability);
        assert!(only(GamepadButton::West).attack && !only(GamepadButton::West).ability);
        let north = only(GamepadButton::North);
        assert!(north.physical && !north.ability);
        assert!(only(GamepadButton::LeftTrigger).swap);
        assert!(only(GamepadButton::Select).restart);
        assert!(only(GamepadButton::Start).start);
        // East restarts, quits, swaps and starts nothing.
        let east = only(GamepadButton::East);
        assert!(!east.restart && !east.start && !east.swap && !east.jump);
        assert_eq!(pad_buttons(|_| false), PadButtons::default());
    }

    /// Keyboard L is the special, next to J and K, and the other keys do not
    /// set it.
    #[test]
    fn keyboard_l_is_the_equipped_special_and_no_other_key_is() {
        let mut keys = ButtonInput::<KeyCode>::default();
        let mut controls = Controls::default();
        for key in [
            KeyCode::Space,
            KeyCode::KeyJ,
            KeyCode::KeyK,
            KeyCode::KeyQ,
            KeyCode::KeyA,
            KeyCode::KeyD,
            KeyCode::KeyS,
            KeyCode::KeyR,
            KeyCode::Escape,
        ] {
            keys.press(key);
        }
        latch_keyboard_taps(&keys, &mut controls);
        assert!(!keyboard_input(&keys, &mut controls).ability);
        keys.release_all();
        keys.clear();
        keys.press(KeyCode::KeyL);
        latch_keyboard_taps(&keys, &mut controls);
        let raw = keyboard_input(&keys, &mut controls);
        assert!(raw.ability);
        assert!(!raw.attack && !raw.physical && !raw.jump && !raw.swap);
        assert_eq!(raw.move_x, 0.0);
    }

    /// HUD-ART-02: the keyboard adapter reads its keys from the binding table (the
    /// single source the HUD's key caps also come from). Every key of every row
    /// drives exactly its action, and no key outside the table drives anything.
    #[test]
    fn the_keyboard_adapter_reads_its_keys_from_the_binding_table() {
        let fire = |key: KeyCode| {
            let mut keys = ButtonInput::<KeyCode>::default();
            let mut controls = Controls::default();
            keys.press(key);
            latch_keyboard_taps(&keys, &mut controls);
            keyboard_input(&keys, &mut controls)
        };
        let reads = |raw: &RawInput, action: KeyAction| match action {
            KeyAction::Primary => raw.attack,
            KeyAction::Physical => raw.physical,
            KeyAction::Special => raw.ability,
            KeyAction::Swap => raw.swap,
            KeyAction::Jump => raw.jump,
            KeyAction::MoveLeft => raw.move_x < 0.0,
            KeyAction::MoveRight => raw.move_x > 0.0,
            KeyAction::Down => raw.down,
        };
        let mut table_keys = Vec::new();
        for (action, _cap, names) in key_bindings::KEY_BINDINGS {
            for name in names {
                table_keys.push(key_code(*name));
                let raw = fire(key_code(*name));
                for other in key_bindings::all_actions() {
                    assert_eq!(reads(&raw, other), other == action, "{name:?} -> {other:?}");
                }
            }
        }
        // The legacy bindings are unchanged: J K L Q Space, A D and the arrows, S.
        for (name, action) in [
            (KeyName::KeyJ, KeyAction::Primary),
            (KeyName::KeyK, KeyAction::Physical),
            (KeyName::KeyL, KeyAction::Special),
            (KeyName::KeyQ, KeyAction::Swap),
            (KeyName::Space, KeyAction::Jump),
            (KeyName::KeyA, KeyAction::MoveLeft),
            (KeyName::ArrowLeft, KeyAction::MoveLeft),
            (KeyName::KeyD, KeyAction::MoveRight),
            (KeyName::ArrowRight, KeyAction::MoveRight),
            (KeyName::KeyS, KeyAction::Down),
            (KeyName::ArrowDown, KeyAction::Down),
        ] {
            assert!(key_bindings::keys_of(action).contains(&name));
        }
        // A key outside the table drives nothing.
        for key in [
            KeyCode::KeyR,
            KeyCode::Escape,
            KeyCode::KeyW,
            KeyCode::KeyF,
            KeyCode::KeyG,
            KeyCode::Enter,
            KeyCode::Tab,
            KeyCode::ArrowUp,
        ] {
            assert!(!table_keys.contains(&key));
            let raw = fire(key);
            assert!(
                key_bindings::all_actions()
                    .iter()
                    .all(|action| !reads(&raw, *action)),
                "{key:?}"
            );
        }
    }

    /// A press shorter than a fixed tick (down and up between two steps) is
    /// latched, read once, and not read again; a held key reads every tick.
    #[test]
    fn a_short_special_tap_between_fixed_steps_is_latched_once() {
        let mut keys = ButtonInput::<KeyCode>::default();
        let mut controls = Controls::default();
        keys.press(KeyCode::KeyL);
        latch_keyboard_taps(&keys, &mut controls);
        keys.release(KeyCode::KeyL);
        keys.clear();
        latch_keyboard_taps(&keys, &mut controls);
        assert!(controls.ability_tap, "the edge survives the release");
        assert!(keyboard_input(&keys, &mut controls).ability);
        assert!(!controls.ability_tap, "consumed by the step");
        assert!(!keyboard_input(&keys, &mut controls).ability);
        // Held: no new edge, but the level keeps reading (the press edge is
        // derived downstream from the held state).
        keys.press(KeyCode::KeyL);
        keys.clear_just_pressed(KeyCode::KeyL);
        assert!(keyboard_input(&keys, &mut controls).ability);
        assert!(keyboard_input(&keys, &mut controls).ability);
    }

    /// Settle the room, then run `ticks` live ticks the way `fixed_step` builds
    /// them: slot inputs from the keyboard and the pads, the party routing and
    /// the combat step.
    fn live_special_tick(
        sim: &mut SceneSim,
        combat: &mut CombatState,
        pads: &mut PadSlots,
        keyboard: RawInput,
    ) {
        let slot_inputs = pads.take_inputs(keyboard);
        pads.party_mut().request_swap(slot_inputs[0].swap, false);
        let hero_inputs = pads.party().hero_inputs(slot_inputs);
        sim.set_drive(pads.party().drive(true));
        combat.step_party(sim, hero_inputs);
    }

    fn special_starts(combat: &CombatState, hero: u8) -> usize {
        combat
            .events
            .iter()
            .filter(|e| {
                matches!(e, dungeon_floorbreak::combat::CombatEvent::SpecialStarted { hero: h, .. } if *h == hero)
            })
            .count()
    }

    fn late_boiler_all() -> (SceneSim, CombatState) {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = f01_late::Loadout::All.flags();
        let (_, scene, combat) = f01_late::boiler_start(root, 360, carl, donut, None).unwrap();
        (scene, combat)
    }

    /// A live keyboard L (through the real mapping, tap latch and party
    /// routing) makes Carl throw in the `f01_late` Boiler; without it he does not.
    #[test]
    fn live_keyboard_l_makes_carl_throw_in_the_late_boiler() {
        let (mut sim, mut combat) = late_boiler_all();
        let mut pads = PadSlots::default();
        let mut keys = ButtonInput::<KeyCode>::default();
        let mut controls = Controls::default();
        for _ in 0..30 {
            live_special_tick(&mut sim, &mut combat, &mut pads, RawInput::NONE);
        }
        assert_eq!(special_starts(&combat, 0), 0, "no press, no throw");
        // A tap that is up again before the step still throws.
        keys.press(KeyCode::KeyL);
        latch_keyboard_taps(&keys, &mut controls);
        keys.release(KeyCode::KeyL);
        keys.clear();
        let keyboard = keyboard_input(&keys, &mut controls);
        assert!(keyboard.ability);
        live_special_tick(&mut sim, &mut combat, &mut pads, keyboard);
        assert_eq!(special_starts(&combat, 0), 1, "Carl threw the dynamite");
    }

    /// A live pad East makes Carl throw from P1's pad and Donut blink from
    /// P2's pad (the story-loadout `all` room holds both specials).
    #[test]
    fn live_pad_east_drives_carls_throw_on_p1_and_donuts_blink_on_p2() {
        let (mut sim, mut combat) = late_boiler_all();
        let mut pads = PadSlots::default();
        let (a, b) = (PadId(10), PadId(11));
        pads.apply(PadEvent::Connected {
            pad: a,
            vendor_id: None,
        });
        pads.apply(PadEvent::Connected {
            pad: b,
            vendor_id: None,
        });
        let frame = |pad: PadId, ability: bool, start: bool| PadFrame {
            pad,
            vendor_id: None,
            move_x: 0.0,
            down: false,
            held: PadButtons {
                ability,
                start,
                ..PadButtons::default()
            },
            pressed: PadButtons {
                ability,
                start,
                ..PadButtons::default()
            },
        };
        pads.sample(frame(b, false, true));
        assert_eq!(
            pads.party().mode(),
            dungeon_floorbreak::party::PartyMode::TwoPlayer
        );
        for _ in 0..30 {
            live_special_tick(&mut sim, &mut combat, &mut pads, RawInput::NONE);
        }
        assert_eq!(
            (special_starts(&combat, 0), special_starts(&combat, 1)),
            (0, 0)
        );
        // P2's East: Donut blinks, Carl does not throw.
        pads.sample(frame(b, true, false));
        live_special_tick(&mut sim, &mut combat, &mut pads, RawInput::NONE);
        assert_eq!(special_starts(&combat, 1), 1, "Donut started her blink");
        assert_eq!(special_starts(&combat, 0), 0);
        // P1's East: Carl throws.
        pads.sample(frame(a, true, false));
        live_special_tick(&mut sim, &mut combat, &mut pads, RawInput::NONE);
        assert_eq!(special_starts(&combat, 0), 1, "Carl threw the dynamite");
    }
}
