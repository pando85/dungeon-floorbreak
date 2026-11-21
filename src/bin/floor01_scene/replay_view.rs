//! PTR-R1: the offline replay viewer of `floor01_scene` (PLAYTRACE-RENDER-01).
//!
//! `--replay-session ID` plays a recorded PLAYTRACE session through the
//! **same** Bevy presentation as a live run: the `ReplayCursor` of the
//! playtrace library re-derives the simulation tick by tick from the recorded
//! per-hero inputs (checking every hash, event and effective input), and this
//! module presents each tick exactly like `fixed_step` does (animators, death
//! presenter, HUD, audio, CINE, camera). The cursor's `SceneSim` and
//! `CombatState` live in the existing `Sim` / `Combat` resources, which stay
//! the single owners of the world.
//!
//! Read-only and offline: the live pad and keyboard-gameplay systems are not
//! registered (a pad cannot join P2, a disconnect cannot pause the replay),
//! `Recorder` is `None`, and the only keys are the viewer's own (pause, step,
//! quit). One `PLAYTRACE_REPLAY ...` result line is printed, never a
//! `PLAYTRACE_VERIFY ` line (see `playtrace::replay_result_line`).
//!
//! Limits (R1): presentation settings are not recorded (defaults are used);
//! particles and animation advance by wall clock while paused (approximate);
//! restart is relaunching the process; sources `demo`, `fixture`, `boss` and
//! `story` are supported (party-fixture and lab sessions report
//! `unsupported_source`).

use std::sync::atomic::{AtomicBool, Ordering};

use super::*;
use bevy::text::{FontHinting, FontSmoothing, LineHeight};
use dungeon_floorbreak::{
    hud::{Control as HudControl, PartyView},
    playtrace::{
        LoadedSession, ReplayCursor, ReplayPolicy, ReplayResult, TickOutcome, Trust,
        replay_result_line, resolve_session,
    },
};

/// Set when a replay diverged: the process then exits non-zero (PTR-Q15).
static DIVERGED: AtomicBool = AtomicBool::new(false);

/// The exit code of a replay run: non-zero after a divergence or an error exit.
pub(super) fn exit_code(exit: &AppExit) -> ExitCode {
    if DIVERGED.load(Ordering::Relaxed) || exit.is_error() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

// ------------------------------------------------------------------ arguments

/// The `--replay-*` flags.
#[derive(Debug, Clone, Default)]
pub(super) struct ReplayArgs {
    /// A session id, a unique prefix, or a session directory.
    pub session: String,
    /// `--replay-compatibility checked`: waive the source fingerprint only.
    pub checked: bool,
    /// Exit when the recording ends (the smoke test).
    pub exit_at_end: bool,
    /// Evidence: pause at this tick, save one window screenshot, resume.
    pub capture_tick: Option<u64>,
    pub capture_file: Option<PathBuf>,
}

const REPLAY_FLAGS: [&str; 5] = [
    "--replay-session",
    "--replay-compatibility",
    "--replay-exit-at-end",
    "--replay-capture-tick",
    "--replay-capture-file",
];

/// Every flag a replay run accepts besides the `--replay-*` ones: the session
/// directory and the presentation flags (accepted at their live defaults).
/// Anything else (every gameplay flag, `--record*`, `--capture-sequence`) is a
/// parse error: the simulation comes from the recording alone.
const ALLOWED_WITH_REPLAY: [&str; 6] = [
    "--record-dir",
    "--scale",
    "--camera",
    "--gore-preset",
    "--cinematic-finishes",
    "--no-audio",
];

/// One `--replay-*` flag. Returns false when `flag` is not one.
pub(super) fn replay_flag<'a>(
    slot: &mut Option<ReplayArgs>,
    flag: &str,
    iter: &mut impl Iterator<Item = &'a String>,
) -> Result<bool, String> {
    if !REPLAY_FLAGS.contains(&flag) {
        return Ok(false);
    }
    let replay = slot.get_or_insert_with(ReplayArgs::default);
    let mut value = |what: &str| {
        iter.next()
            .cloned()
            .ok_or_else(|| format!("{flag} needs {what}"))
    };
    match flag {
        "--replay-session" => replay.session = value("a session id")?,
        "--replay-compatibility" => {
            let v = value("`checked`")?;
            if v != "checked" {
                return Err(format!(
                    "bad --replay-compatibility {v:?} (the only value is checked)"
                ));
            }
            replay.checked = true;
        }
        "--replay-exit-at-end" => replay.exit_at_end = true,
        "--replay-capture-tick" => {
            let v = value("a tick")?;
            replay.capture_tick = Some(
                v.parse()
                    .map_err(|_| format!("bad --replay-capture-tick {v:?}"))?,
            );
        }
        _ => replay.capture_file = Some(PathBuf::from(value("a file name")?)),
    }
    Ok(true)
}

/// The replay rules of the command line: `--replay-session` is required by the
/// other replay flags and rejects every gameplay flag.
pub(super) fn check_args(raw: &[String], replay: &Option<ReplayArgs>) -> Result<(), String> {
    let Some(replay) = replay else {
        return Ok(());
    };
    if replay.session.is_empty() {
        return Err("the --replay-* flags need --replay-session ID".into());
    }
    if replay.capture_tick.is_some() != replay.capture_file.is_some() {
        return Err("--replay-capture-tick and --replay-capture-file go together".into());
    }
    for arg in raw.iter().skip(1) {
        let Some(flag) = arg.strip_prefix("--") else {
            continue;
        };
        let name = format!("--{}", flag.split('=').next().unwrap_or(flag));
        if !REPLAY_FLAGS.contains(&name.as_str()) && !ALLOWED_WITH_REPLAY.contains(&name.as_str()) {
            return Err(format!(
                "{name} cannot be combined with --replay-session: the simulation comes from the recording (allowed: {})",
                ALLOWED_WITH_REPLAY.join(", ")
            ));
        }
    }
    Ok(())
}

// ------------------------------------------------------------------- opening

/// A recording the viewer cannot show: the window never opens.
pub(super) struct Unsupported {
    session: String,
    code: String,
    message: String,
}

impl Unsupported {
    fn new(session: &str, code: &str, message: impl Into<String>) -> Self {
        Self {
            session: session.to_owned(),
            code: code.to_owned(),
            message: message.into(),
        }
    }

    /// Print the result line (and why) and return the exit code.
    pub(super) fn report(&self) -> ExitCode {
        println!(
            "{}",
            replay_result_line(
                &self.session,
                &ReplayResult::Unsupported {
                    code: self.code.clone()
                },
                0,
                None
            )
        );
        eprintln!("floor01_scene: replay: {}: {}", self.code, self.message);
        ExitCode::from(1)
    }
}

/// A session opened and its first tick rebuilt.
pub(super) struct Prepared {
    pub kit: EnvKit,
    loaded: LoadedSession,
    cursor: ReplayCursor,
    scene: SceneSim,
    combat: Option<CombatState>,
    trust: Trust,
}

/// Open the session, preflight it under the requested policy, rebuild tick 0
/// and set `args` to what the recording says (deck, rules, room). Everything
/// sim-relevant comes from the manifest; the command line only chose the
/// session and the presentation.
pub(super) fn prepare(args: &mut Args, root: &Path) -> Result<Prepared, Unsupported> {
    let replay = args.replay.clone().expect("a replay run");
    let id = replay.session.as_str();
    let base = match args.record_dir.clone() {
        Some(base) => base,
        None => playtrace::default_record_dir(
            std::env::var("XDG_DATA_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
        )
        .map_err(|e| Unsupported::new(id, "invalid_session", e.to_string()))?,
    };
    let dir = resolve_session(&base, id)
        .map_err(|message| Unsupported::new(id, "invalid_session", message))?;
    let loaded = playtrace::read_session(&dir, true)
        .map_err(|e| Unsupported::new(id, "invalid_session", e.to_string()))?;
    let session = loaded.manifest.session_id.clone();
    let policy = if replay.checked {
        ReplayPolicy::CheckedCrossBuild
    } else {
        ReplayPolicy::Strict
    };
    let (cursor, scene, combat, _tuning, trust) = ReplayCursor::open(&loaded, root, policy)
        .map_err(|f| Unsupported::new(&session, f.code, f.message))?;

    let reset = &loaded.manifest.reset;
    let unsupported = |code: &str, message: String| Unsupported::new(&session, code, message);
    args.deck = match reset.view_height {
        360 => false,
        400 => true,
        other => {
            return Err(unsupported(
                "unsupported_view",
                format!("view height {other} has no deck"),
            ));
        }
    };
    args.cf02.rules = reset.cf02_rules;
    args.cf02.seed = reset.cf02_seed;
    args.donut_biscuit = reset.donut_biscuit;
    args.carl = dungeon_floorbreak::carl_prog::CarlProgFlags {
        prog: reset.carl_prog,
        dynamite: reset.carl_dynamite,
        gauntlet: reset.carl_gauntlet,
    };
    args.donut = dungeon_floorbreak::donut_prog::DonutProgFlags {
        prog: reset.donut_prog,
        light_feet: reset.donut_light_feet,
        puddle: reset.donut_puddle,
    };
    let load = |result: Result<EnvKit, String>| {
        result.map_err(|e| {
            unsupported(
                "content_mismatch",
                format!("{e} (run from the repository root)"),
            )
        })
    };
    let kit = match reset.source.as_str() {
        "demo" => {
            args.demo = true;
            args.demo_beat = reset.demo_beat as usize;
            if reset.cf02_rules {
                // ENEMY-GAMEPLAY-ENGINEER: a recorded Engineer session names its
                // room; the plain beat match would describe `cf02_arena`.
                args.fixture = Some(cf02::arena::fixture_for_recorded(
                    &reset.name,
                    args.demo_beat,
                ));
                load(cf02::arena::load_arena(root))?
            } else {
                args.fixture = Some(demo::fixture_for_beat(args.demo_beat));
                load(demo::load_hall(root))?
            }
        }
        "boss" => {
            args.boss = true;
            args.fixture = Some(boss_scene::BOSS01_SCENE);
            load(boss_scene::load_arena(root))?
        }
        "fixture" => {
            args.fixture = Some(Fixture::by_name(&reset.name).ok_or_else(|| {
                unsupported(
                    "unsupported_source",
                    format!("unknown fixture {:?}", reset.name),
                )
            })?);
            load(EnvKit::load(root, LAYOUT_PATH).map_err(|e| e.to_string()))?
        }
        "story" => load(EnvKit::load(root, LAYOUT_PATH).map_err(|e| e.to_string()))?,
        other => {
            return Err(unsupported(
                "unsupported_source",
                format!("the viewer does not show {other:?} sessions yet"),
            ));
        }
    };
    Ok(Prepared {
        kit,
        loaded,
        cursor,
        scene,
        combat,
        trust,
    })
}

/// The scene of a replay run: the cursor's own `SceneSim` and `CombatState`
/// plus the presentation half of the live setup (no second sim construction).
/// The returned party only seeds the (unused) pad table.
#[allow(clippy::type_complexity)]
pub(super) fn scene_of(
    prepared: Prepared,
    args: &Args,
    actors: &Actors,
    root: &Path,
    hud: &mut HudRuntime,
) -> Result<
    (
        SceneSim,
        SceneAnimators,
        Option<CombatState>,
        Option<DeathPresenter>,
        PartyState,
        ReplayView,
    ),
    String,
> {
    let Prepared {
        loaded,
        cursor,
        scene,
        combat,
        trust,
        ..
    } = prepared;
    let (anim, death) = match combat.as_ref() {
        Some(combat) => {
            let (anim, death) = combat_presentation(actors, root, args, &scene, combat, hud)?;
            (anim, Some(death))
        }
        None => (actors.animators(&scene.world), None),
    };
    let party = PartyState::new(*cursor.tuning());
    let font_path = root.join(OVERLAY_FONT_PATH);
    let font = std::fs::read(&font_path).map_err(|e| format!("{}: {e}", font_path.display()))?;
    let view = ReplayView {
        session_id: loaded.manifest.session_id.clone(),
        total: loaded.ticks.len() as u64,
        loaded,
        cursor,
        trust,
        exit_at_end: args.replay.as_ref().is_some_and(|r| r.exit_at_end),
        capture: args
            .replay
            .as_ref()
            .and_then(|r| r.capture_tick.zip(r.capture_file.clone()))
            .map(|(tick, file)| Capture {
                tick,
                file,
                phase: CapturePhase::WaitTick,
            }),
        paused: false,
        step_requests: 0,
        stepped: false,
        diverged: None,
        ended: false,
        printed: false,
        shown: None,
        font: Some(font),
    };
    Ok((scene, anim, combat, death, party, view))
}

// ------------------------------------------------------------------- runtime

/// What the overlay shows of the last presented tick.
#[derive(Debug, Clone, Copy)]
struct Shown {
    t: u64,
    ep: u32,
    et: u64,
    src: [playtrace::ControlSource; 2],
}

/// The evidence screenshot request (`--replay-capture-tick`).
struct Capture {
    tick: u64,
    file: PathBuf,
    phase: CapturePhase,
}

enum CapturePhase {
    WaitTick,
    Settle(u32),
    Pending(super::PendingFile),
}

/// The viewer state: the recording, its cursor and the playback controls.
#[derive(Resource)]
pub(super) struct ReplayView {
    session_id: String,
    total: u64,
    loaded: LoadedSession,
    cursor: ReplayCursor,
    trust: Trust,
    exit_at_end: bool,
    capture: Option<Capture>,
    paused: bool,
    step_requests: u32,
    /// A tick was presented in the current fixed step (the camera follows it).
    stepped: bool,
    diverged: Option<playtrace::VerifyFailure>,
    ended: bool,
    printed: bool,
    shown: Option<Shown>,
    font: Option<Vec<u8>>,
}

impl ReplayView {
    fn frozen(&self) -> bool {
        self.paused || self.diverged.is_some() || self.ended
    }

    /// The post-state hash of the last tick the replay matched.
    fn post(&self) -> Option<u64> {
        let done = self.cursor.ticks_done() as usize;
        done.checked_sub(1)
            .and_then(|i| self.loaded.ticks.get(i))
            .map(|t| t.post)
    }

    /// Print the single result line, once.
    fn print_result(&mut self, result: &ReplayResult) {
        if self.printed {
            return;
        }
        self.printed = true;
        let mut line = replay_result_line(
            &self.session_id,
            result,
            self.cursor.ticks_done(),
            self.post(),
        );
        if !self.ended && self.diverged.is_none() {
            // Closed before the end: not a verdict on the whole recording.
            line.push_str(&format!(" partial=true total={}", self.total));
        }
        println!("{line}");
    }
}

fn source_word(source: playtrace::ControlSource) -> &'static str {
    use playtrace::ControlSource as S;
    match source {
        S::HumanP1 => "human P1",
        S::HumanP2 => "human P2",
        S::CompanionAi => "AI companion",
        S::ScriptedBot => "bot",
        S::None => "idle",
    }
}

/// Viewer keys (keyboard only, read in `Update`): Space or P pauses and
/// resumes, `.` or the right arrow steps one tick while paused, Escape quits.
pub(super) fn replay_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mut view: ResMut<ReplayView>,
    mut frozen: ResMut<Frozen>,
    mut exit: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
    if (keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::KeyP))
        && view.diverged.is_none()
        && !view.ended
    {
        view.paused = !view.paused;
    }
    if (keys.just_pressed(KeyCode::Period) || keys.just_pressed(KeyCode::ArrowRight))
        && view.paused
        && view.diverged.is_none()
        && !view.ended
    {
        view.step_requests += 1;
    }
    // The CINE planner treats a frozen sim like a capture: no slow-motion over it.
    frozen.0 = view.frozen();
}

/// The replay `fixed_step`: it **replaces** the live one. One tick per fixed
/// step, driven by the cursor, then the same presentation calls the live loop
/// makes after a step.
#[allow(clippy::too_many_arguments)]
pub(super) fn replay_step(
    mut view: ResMut<ReplayView>,
    loading: Option<Res<Loading>>,
    setup: Res<Setup>,
    mut combat: ResMut<Combat>,
    mut death: ResMut<Death>,
    mut controls: ResMut<Controls>,
    mut sim: ResMut<Sim>,
    mut anim: ResMut<Anim>,
    mut after_exit: ResMut<AfterExitTicks>,
    mut sinks: Sinks,
    mut hud: ResMut<HudState>,
    mut exit: MessageWriter<AppExit>,
) {
    view.stepped = false;
    // Wait for the textures like the live loop effectively does.
    if !loading.is_none_or(|l| l.reported) {
        return;
    }
    if view.diverged.is_some() || view.ended {
        return;
    }
    if view.paused {
        if view.step_requests == 0 {
            return;
        }
        view.step_requests -= 1;
    } else {
        view.step_requests = 0;
    }
    let was_complete = sim.0.world.complete;
    let v = &mut *view;
    match v.cursor.next_tick(&v.loaded, &mut sim.0, combat.0.as_mut()) {
        Ok(Some(outcome)) => {
            present_tick(
                &outcome,
                &setup,
                &mut sim.0,
                &mut combat,
                &mut anim.0,
                &mut death,
                &mut controls,
                &mut after_exit,
                &mut sinks,
                &mut hud.0,
                was_complete,
            );
            v.stepped = true;
            v.shown = Some(Shown {
                t: outcome.t,
                ep: outcome.ep,
                et: outcome.et,
                src: outcome.src,
            });
            if v.capture.as_ref().is_some_and(|c| c.tick == outcome.t) {
                // Hold the frame for the screenshot (the capture system resumes).
                v.paused = true;
            }
        }
        Ok(None) => {
            v.ended = true;
            let result = ReplayResult::of_trust(v.trust);
            v.print_result(&result);
            if v.exit_at_end {
                exit.write(AppExit::Success);
            }
        }
        Err(failure) => {
            DIVERGED.store(true, Ordering::Relaxed);
            let result = ReplayResult::Diverged {
                code: failure.code.to_owned(),
                tick: failure.tick,
            };
            eprintln!("floor01_scene: replay diverged: {failure}");
            v.diverged = Some(failure);
            v.print_result(&result);
            if v.exit_at_end {
                exit.write(AppExit::error());
            }
        }
    }
}

/// The presentation calls of the live loop for one finished tick, in the
/// order of `fixed_step`: animators (or their reset), death presenter, camera
/// cut flag, audio and CINE fan-out, HUD. On a lifecycle tick (a restart or
/// respawn control, a respawn or defeat restart inside the step) the resets
/// run on the post-step state and the animator step of that tick is skipped:
/// at most one animation frame differs from live, the simulation not at all
/// (adopted default N-2).
#[allow(clippy::too_many_arguments)]
fn present_tick(
    outcome: &TickOutcome,
    setup: &Setup,
    sim: &mut SceneSim,
    combat: &mut Combat,
    anim: &mut SceneAnimators,
    death: &mut Death,
    controls: &mut Controls,
    after_exit: &mut AfterExitTicks,
    sinks: &mut Sinks,
    hud: &mut HudRuntime,
    was_complete: bool,
) {
    let respawn_control = outcome
        .applied
        .iter()
        .any(|c| matches!(c, playtrace::ControlEvent::DemoRespawn));
    if outcome.restarted {
        controls.beat_printed = None;
        controls.complete_printed = false;
        after_exit.0 = 0;
        if let Some(death) = death.0.as_mut() {
            death.clear();
        }
        hud.restart();
        println!("FLOOR1_RESTART");
    }
    match combat.0.as_mut() {
        Some(combat) => {
            if outcome.lifecycle_reset() {
                anim.reset(&sim.world);
                anim.reset_art(combat);
                controls.camera_cut = true;
            } else {
                anim.step(&sim.world);
                anim.step_art(sim, combat);
            }
            if let Some(death) = death.0.as_mut() {
                death.observe(combat, sim);
            }
            let feats = print_combat_events(
                combat,
                Some(&mut sinks.audio),
                Some(&mut sinks.cine.adapter),
            );
            for feat in feats {
                hud.push_chain_feat(feat);
            }
            if outcome.respawned_in_step || respawn_control {
                println!("DEMO01_RESPAWN checkpoint={}", combat.checkpoint);
            }
            if setup.args.cf02.rules {
                if cf02::arena::complete(combat) && !controls.complete_printed {
                    controls.complete_printed = true;
                    println!("CF02_ARENA_COMPLETE tick={}", sim.world.tick);
                }
            } else if setup.args.demo {
                let printed = controls.beat_printed.get_or_insert(combat.checkpoint);
                while *printed < combat.checkpoint {
                    *printed += 1;
                    let beat = demo::BEATS[*printed];
                    println!("DEMO01_BEAT {} {}", beat.id, beat.name);
                }
                if demo::complete(combat) && !controls.complete_printed {
                    controls.complete_printed = true;
                    println!("DEMO01_COMPLETE tick={}", sim.world.tick);
                }
            }
        }
        None => {
            if outcome.restarted {
                anim.reset(&sim.world);
                controls.camera_cut = true;
            } else {
                anim.step(&sim.world);
            }
        }
    }
    step_hud(hud, sim, combat.0.as_ref());
    if sim.world.complete && !was_complete {
        println!("FLOOR1_EXIT_REACHED tick={}", sim.world.tick);
    }
    if was_complete {
        after_exit.0 += 1;
    }
}

/// The camera step of the tick the replay just presented: the same
/// `camera_tick` the live schedule runs after `fixed_step`, but only after a
/// real tick (a paused replay must not advance the eased pan).
pub(super) fn replay_camera(
    view: Res<ReplayView>,
    mut cam: ResMut<SceneCam>,
    mut interest: ResMut<Interest>,
    sim: Res<Sim>,
    combat: Res<Combat>,
    setup: Res<Setup>,
    mut controls: ResMut<Controls>,
) {
    if !view.stepped {
        return;
    }
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

/// The HUD's party plates from the recorded drive (never from a pad): two
/// players are Carl P1 and Donut P2; solo is the controlled hero as P1 and the
/// other as AI. Default glyphs, no reconnect request, chips from the tuning.
pub(super) fn replay_hud(view: Res<ReplayView>, mut hud: ResMut<HudState>) {
    let party = match view.cursor.drive() {
        Some(drive) => {
            let (carl, donut) = if drive.two_player {
                (HudControl::Player(1), HudControl::Player(2))
            } else if drive.controlled == HeroId::Carl {
                (HudControl::Player(1), HudControl::Ai)
            } else {
                (HudControl::Ai, HudControl::Player(1))
            };
            PartyView {
                carl,
                donut,
                carl_glyphs: GlyphSet::default(),
                donut_glyphs: GlyphSet::default(),
                reconnect: None,
                donut_chips: view.cursor.tuning().donut_ability_chips(),
            }
        }
        None => PartyView::default(),
    };
    hud.0.set_party(party);
}

/// Print the one result line when the window closes before the end.
pub(super) fn replay_finish(mut exit: MessageReader<AppExit>, mut view: ResMut<ReplayView>) {
    if exit.read().next().is_none() {
        return;
    }
    let result = match view.diverged.as_ref() {
        Some(f) => ReplayResult::Diverged {
            code: f.code.to_owned(),
            tick: f.tick,
        },
        None => ReplayResult::of_trust(view.trust),
    };
    view.print_result(&result);
}

// -------------------------------------------------------------------- capture

/// `--replay-capture-tick N --replay-capture-file F`: when tick N has been
/// presented the replay holds still, one window screenshot is saved, and the
/// replay resumes. Evidence only (R3 owns frame export).
pub(super) fn replay_capture(
    mut commands: Commands,
    mut view: ResMut<ReplayView>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(mut capture) = view.capture.take() else {
        return;
    };
    let phase = std::mem::replace(&mut capture.phase, CapturePhase::WaitTick);
    let held = view.shown.is_some_and(|s| s.t == capture.tick) && view.paused;
    capture.phase = match phase {
        CapturePhase::WaitTick if held => CapturePhase::Settle(CAPTURE_SETTLE_FRAMES),
        CapturePhase::WaitTick => CapturePhase::WaitTick,
        CapturePhase::Settle(frames) if frames > 0 => CapturePhase::Settle(frames - 1),
        CapturePhase::Settle(_) => {
            if let Some(parent) = capture.file.parent().filter(|p| !p.as_os_str().is_empty()) {
                let _ = std::fs::create_dir_all(parent);
            }
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(capture.file.clone()));
            CapturePhase::Pending(super::PendingFile::default())
        }
        CapturePhase::Pending(mut file) => {
            let len = std::fs::metadata(&capture.file).map_or(0, |m| m.len());
            if len > 0 && len == file.last_len {
                file.stable += 1;
            } else {
                file.stable = 0;
            }
            file.last_len = len;
            if file.stable < CAPTURE_STABLE_FRAMES {
                file.waited += 1;
                if file.waited > CAPTURE_TIMEOUT_FRAMES {
                    println!("FLOOR1_CAPTURE_FAILURE: the screenshot was never written");
                    exit.write(AppExit::error());
                    return;
                }
                CapturePhase::Pending(file)
            } else {
                println!(
                    "PLAYTRACE_REPLAY_CAPTURE tick={} file={}",
                    capture.tick,
                    capture
                        .file
                        .file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                );
                // Resume: the capture is done.
                view.paused = false;
                return;
            }
        }
    };
    view.capture = Some(capture);
}

// -------------------------------------------------------------------- overlay

/// Silkscreen, the HUD's small-text font (the same file the HUD loads).
const OVERLAY_FONT_PATH: &str = "assets/fonts/silkscreen/Silkscreen-Regular.ttf";
const FONT_PX: f32 = 8.0;
const LINE_PX: f32 = 10.0;
const OVERLAY_LINES: usize = 4;
const OVERLAY_X: f32 = 8.0;
/// Below the HUD plates (they end near y 55): top-left of the virtual view.
const OVERLAY_Y: i32 = 58;
const OVERLAY_Z: f32 = 90.0;

#[derive(Component)]
pub(super) struct ReplayText(usize);

#[derive(Component)]
pub(super) struct ReplayBackdrop;

/// Spawn the overlay's pooled lines and backdrop on the HUD layer.
pub(super) fn spawn_overlay(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    mut view: ResMut<ReplayView>,
) {
    let layers = RenderLayers::layer(dungeon_floorbreak::hud_adapter::LAYER_HUD);
    let bytes = view.font.take().unwrap_or_default();
    let handle = fonts.add(Font::from_bytes(bytes));
    commands.spawn((
        ReplayBackdrop,
        Sprite::default(),
        Anchor::TOP_LEFT,
        Transform::default(),
        Visibility::Hidden,
        layers.clone(),
    ));
    for index in 0..OVERLAY_LINES {
        commands.spawn((
            ReplayText(index),
            Text2d::new(""),
            TextFont {
                font: handle.clone().into(),
                font_size: FontSize::Px(FONT_PX),
                font_smoothing: FontSmoothing::None,
                ..default()
            },
            LineHeight::Px(LINE_PX),
            FontHinting::Enabled,
            TextColor(Color::WHITE),
            Anchor::TOP_LEFT,
            Transform::default(),
            Visibility::Hidden,
            layers.clone(),
        ));
    }
}

/// The overlay's lines and tones for the current state (also what a test
/// reads): trust banner, position, controls and the honesty notes.
fn overlay_lines(view: &ReplayView) -> Vec<(String, Color)> {
    let white = Color::WHITE;
    let amber = Color::srgb_u8(255, 196, 64);
    let red = Color::srgb_u8(255, 80, 80);
    let mut lines = Vec::new();
    if let Some(failure) = &view.diverged {
        lines.push((
            format!(
                "DIVERGED t={} code={}",
                failure
                    .tick
                    .map_or_else(|| "-".to_owned(), |t| t.to_string()),
                failure.code
            ),
            red,
        ));
        lines.push(("this frame is not the recording".to_owned(), red));
    } else {
        lines.push(match view.trust {
            Trust::Strict => (
                "REPLAY STRICT (same source as the recording)".to_owned(),
                white,
            ),
            Trust::CheckedCrossBuild => (
                "CHECKED CROSS-BUILD (NOT ORIGINAL-BUILD VERIFIED)".to_owned(),
                amber,
            ),
        });
    }
    let state = if view.diverged.is_some() {
        "STOPPED"
    } else if view.ended {
        "END OF RECORDING"
    } else if view.paused {
        "PAUSED"
    } else {
        "PLAYING"
    };
    match view.shown {
        Some(s) => lines.push((
            format!(
                "TICK {}/{} (t={})  EP {}  ET {}  {state}  CARL {}  DONUT {}",
                s.t + 1,
                view.total,
                s.t,
                s.ep,
                s.et,
                source_word(s.src[0]),
                source_word(s.src[1]),
            ),
            white,
        )),
        None => lines.push((format!("TICK 0/{}  {state}", view.total), white)),
    }
    lines.push((
        "presentation settings not recorded: defaults".to_owned(),
        white,
    ));
    lines.push((
        "SPACE pause  . step  ESC quit  (particles use wall clock: approximate when paused)"
            .to_owned(),
        white,
    ));
    lines
}

type BackdropQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Sprite,
        &'static mut Transform,
        &'static mut Visibility,
    ),
    (With<ReplayBackdrop>, Without<ReplayText>),
>;

/// Draw the overlay (only when the viewer state changed).
pub(super) fn draw_overlay(
    view: Res<ReplayView>,
    setup: Res<Setup>,
    mut texts: Query<
        (
            &ReplayText,
            &mut Text2d,
            &mut TextColor,
            &mut Transform,
            &mut Visibility,
        ),
        Without<ReplayBackdrop>,
    >,
    mut backdrop: BackdropQuery,
) {
    if !view.is_changed() {
        return;
    }
    let view_h = view_height(setup.args.deck) as f32;
    let lines = overlay_lines(&view);
    let to_world = |x: f32, y: f32, z: f32| Vec3::new(x, view_h - y, z);
    for (index, mut text, mut color, mut tr, mut vis) in &mut texts {
        match lines.get(index.0) {
            Some((line, tone)) => {
                if text.0 != *line {
                    text.0.clone_from(line);
                }
                color.set_if_neq(TextColor(*tone));
                tr.translation = to_world(
                    OVERLAY_X,
                    OVERLAY_Y as f32 + LINE_PX * index.0 as f32,
                    OVERLAY_Z + 1.0,
                );
                vis.set_if_neq(Visibility::Inherited);
            }
            None => {
                vis.set_if_neq(Visibility::Hidden);
            }
        }
    }
    for (mut sprite, mut tr, mut vis) in &mut backdrop {
        sprite.color = if view.diverged.is_some() {
            Color::srgba_u8(70, 0, 0, 200)
        } else {
            Color::srgba_u8(8, 7, 14, 170)
        };
        sprite.custom_size = Some(Vec2::new(
            VIRTUAL_WIDTH as f32 - 2.0 * (OVERLAY_X - 4.0),
            LINE_PX * lines.len() as f32 + 4.0,
        ));
        tr.translation = to_world(OVERLAY_X - 4.0, OVERLAY_Y as f32 - 2.0, OVERLAY_Z);
        vis.set_if_neq(Visibility::Inherited);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Args, String> {
        let args: Vec<String> = std::iter::once("floor01_scene")
            .chain(args.iter().copied())
            .map(str::to_owned)
            .collect();
        parse_args_with(&args, HudSettings::default())
    }

    #[test]
    fn replay_flags_parse_and_default_to_strict() {
        let args = parse(&["--replay-session", "abc", "--scale", "3"]).unwrap();
        let replay = args.replay.unwrap();
        assert_eq!(replay.session, "abc");
        assert!(!replay.checked && !replay.exit_at_end);
        let args = parse(&[
            "--replay-session",
            "abc",
            "--replay-compatibility",
            "checked",
            "--replay-exit-at-end",
            "--record-dir",
            "x",
            "--camera",
            "scroll",
            "--gore-preset",
            "2",
            "--cinematic-finishes=off",
            "--no-audio",
        ])
        .unwrap();
        let replay = args.replay.unwrap();
        assert!(replay.checked && replay.exit_at_end);
        assert!(args.record_dir.is_some());
    }

    #[test]
    fn every_gameplay_and_record_flag_is_a_parse_error_with_replay() {
        let rejected: [&[&str]; 28] = [
            &["--cf02"],
            &["--cf02-seed", "3"],
            &["--demo"],
            &["--demo-beat", "1"],
            &["--donut-biscuit"],
            &["--carl-prog"],
            &["--carl-dynamite"],
            &["--carl-gauntlet"],
            &["--donut-prog"],
            &["--donut-light-feet"],
            &["--donut-puddle"],
            &["--fixture", "cp01_single_goblin"],
            &["--party-fixture", "solo_swap_continuity"],
            &["--companion", "off"],
            &["--manual"],
            &["--deck"],
            &["--scene", "boss_f01"],
            &["--autopilot"],
            &["--lab-room", "R01"],
            &["--capture-sequence", "0:1:1"],
            &["--record"],
            &["--no-record"],
            &["--record-strict"],
            &["--record-other-tester"],
            &["--exit-after-ticks", "10"],
            &["--scenario", "idle"],
            &["--headless-floor01-smoke"],
            &["--perf-frames", "60"],
        ];
        for flags in rejected {
            let mut args = vec!["--replay-session", "abc"];
            args.extend_from_slice(flags);
            let error = parse(&args).expect_err(&format!("{flags:?} must be rejected"));
            assert!(
                error.contains("cannot be combined with --replay-session"),
                "{flags:?}: {error}"
            );
        }
        // Also before the session flag.
        assert!(parse(&["--record", "--replay-session", "abc"]).is_err());
        assert!(parse(&["--cf02=1", "--replay-session", "abc"]).is_err());
    }

    #[test]
    fn replay_only_flags_need_a_session_and_a_valid_value() {
        assert!(parse(&["--replay-exit-at-end"]).is_err());
        assert!(parse(&["--replay-compatibility", "checked"]).is_err());
        assert!(parse(&["--replay-session", "abc", "--replay-compatibility", "loose"]).is_err());
        assert!(parse(&["--replay-session"]).is_err());
        assert!(parse(&["--replay-session", "abc", "--replay-capture-tick", "5"]).is_err());
        assert!(
            parse(&[
                "--replay-session",
                "abc",
                "--replay-capture-tick",
                "5",
                "--replay-capture-file",
                "f.png"
            ])
            .is_ok()
        );
    }

    #[test]
    fn a_replay_run_never_records_and_is_not_a_live_input_run() {
        let args = parse(&["--replay-session", "abc"]).unwrap();
        // The default (`Auto`) would record a live-input run; a replay is not one.
        assert_eq!(args.record, RecordMode::Auto);
        assert!(!args.live_input());
        assert!(!args.recording_enabled());
    }
}
