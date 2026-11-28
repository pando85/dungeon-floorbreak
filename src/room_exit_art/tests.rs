use super::*;
use crate::carl_prog::CarlProgFlags;
use crate::corridor::SceneSim;
use crate::donut_prog::DonutProgFlags;
use crate::encounter::Goal;
use crate::env_kit::EnvKit;
use crate::f01_ring::{self, RingRoom};
use crate::input::RawInput;
use crate::party::HeroId;
use crate::party_scene::PartyFixtureSim;
use crate::{f01_boiler, f01_hazards, f01_late, f01_mastery, f01_mastery_fixtures};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn json(path: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(root().join(path)).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The canvas, the pivot and the two held frames are the manifest's
/// (`scripts/author_danger_fx.py` writes both).
#[test]
fn the_frames_match_the_manifest() {
    let m = json(&format!("assets/manifests/{MANIFEST}.json"));
    assert_eq!(m["output_atlas"], ATLAS);
    assert_eq!(m["frame_size"], serde_json::json!([EXIT_W, EXIT_H]));
    assert_eq!(m["pivot"], serde_json::json!([0, EXIT_H - 1]));
    let frames = m["prop_kit"]["frames"].as_array().unwrap();
    assert_eq!(frames.len(), 2);
    for frame in [ExitFrame::Closed, ExitFrame::Open] {
        assert_eq!(frames[frame.index()]["tag"], frame.tag());
        assert_eq!(m["prop_kit"]["playback"][frame.tag()], "hold");
    }
}

/// The Guild door piece the art replaces, in a room's kit: `(x, y, w, h)`, layout px.
fn portal(kit: &EnvKit) -> (i32, i32, i32, i32) {
    let p: Vec<_> = kit
        .pieces
        .iter()
        .filter(|p| p.piece == PORTAL_PIECE)
        .collect();
    assert_eq!(p.len(), 1, "one exit marker");
    (p[0].x, p[0].y, p[0].rect.w, p[0].rect.h)
}

/// In rooms 2, 4, 4a, 4b and 6 the opening is the sidecar's exit (in rooms 4 and 6 the
/// `defeat_roster` exit gate's collider), the canvas stands on the walking line,
/// centred in the replaced Guild door's place and as tall, inside the room.
#[test]
fn the_opening_is_the_exit_and_the_doorway_takes_the_guild_door_place() {
    let boiler = f01_boiler::launch(&root()).unwrap();
    let hazards = f01_hazards::launch(&root()).unwrap();
    let mastery = f01_mastery::launch(&root()).unwrap();
    let court = f01_ring::launch(&root(), RingRoom::Court).unwrap();
    let split = f01_ring::launch(&root(), RingRoom::Split).unwrap();
    let rooms = [
        ("boiler", &boiler.kit, &boiler.loaded, Goal::ReachExit),
        ("hazards", &hazards.kit, &hazards.loaded, Goal::DefeatRoster),
        ("ring_court", &court.kit, &court.loaded, Goal::ReachExit),
        ("ring_split", &split.kit, &split.loaded, Goal::ReachExit),
        ("mastery", &mastery.kit, &mastery.loaded, Goal::DefeatRoster),
    ];
    for (name, kit, loaded, goal) in rooms {
        let exit = loaded.encounter.exit;
        assert!(loaded.encounter.goal == goal, "{name}");
        assert_eq!((exit.w, exit.h), (OPENING_W, OPENING_H), "{name}");
        assert_eq!(
            exit.bottom(),
            kit.base_walk_line_y,
            "{name}: on the walking line"
        );
        let (px, py, pw, ph) = portal(kit);
        let (left, right) = (exit.x - JAMB_W, exit.x - JAMB_W + EXIT_W);
        assert_eq!(
            left + EXIT_W / 2,
            px + pw / 2,
            "{name}: centred on the Guild door"
        );
        assert_eq!(
            py + ph,
            kit.base_walk_line_y,
            "{name}: both on the walking line"
        );
        assert_eq!(ph, EXIT_H, "{name}: the Guild door's height");
        assert!(
            left >= px && right <= px + pw,
            "{name}: inside the Guild door's place"
        );
        assert!(
            exit.x - JAMB_W >= 0 && exit.right() + JAMB_W <= kit.width,
            "{name}"
        );
    }
    // Rooms 4 and 6: the exit gate's block is the opening.
    for (name, run_combat) in [
        ("hazards", {
            let mut scene = SceneSim::new_at(&hazards.kit, 360, hazards.fixture.carl_x).unwrap();
            f01_hazards::combat_of(&hazards, &mut scene).unwrap()
        }),
        ("mastery", {
            let mut scene = SceneSim::new_at(&mastery.kit, 360, mastery.fixture.carl_x).unwrap();
            f01_mastery::combat_of(&mastery, &mut scene).unwrap()
        }),
    ] {
        let env = run_combat.env().unwrap();
        let gates: Vec<_> = env
            .solids()
            .iter()
            .filter(|s| matches!(s.kind, EnvSolidKind::ExitGate))
            .collect();
        assert_eq!(gates.len(), 1, "{name}");
        let exit = if name == "hazards" {
            hazards.loaded.encounter.exit
        } else {
            mastery.loaded.encounter.exit
        };
        assert_eq!(gates[0].rect, exit, "{name}: the exit gate is the opening");
    }
}

/// EXIT-AD7: in every replaced room the composer's end torch stands behind the
/// doorway with its flame tip above the cap; it is the one piece the doorway hides,
/// and nothing else is hidden (the Boiler's pipe in front of the door stays).
#[test]
fn the_doorway_hides_only_the_end_torch_under_its_cap() {
    let boiler = f01_boiler::launch(&root()).unwrap();
    let hazards = f01_hazards::launch(&root()).unwrap();
    let mastery = f01_mastery::launch(&root()).unwrap();
    let court = f01_ring::launch(&root(), RingRoom::Court).unwrap();
    let split = f01_ring::launch(&root(), RingRoom::Split).unwrap();
    for (name, kit, exit) in [
        ("boiler", &boiler.kit, boiler.loaded.encounter.exit),
        ("hazards", &hazards.kit, hazards.loaded.encounter.exit),
        ("ring_court", &court.kit, court.loaded.encounter.exit),
        ("ring_split", &split.kit, split.loaded.encounter.exit),
        ("mastery", &mastery.kit, mastery.loaded.encounter.exit),
    ] {
        let hidden: Vec<_> = kit
            .pieces
            .iter()
            .filter(|p| hides_piece(exit, &p.piece, p.x, p.y, p.rect.w, p.rect.h))
            .collect();
        assert_eq!(hidden.len(), 1, "{name}: the end torch only");
        let torch = hidden[0];
        assert_eq!(torch.piece, TORCH_PIECE, "{name}");
        let canvas_top = exit.bottom() - EXIT_H;
        assert!(
            torch.y < canvas_top && torch.y + torch.rect.h > canvas_top,
            "{name}: its flame tip stood over the cap"
        );
    }
    // A torch outside the doorway's columns is never hidden.
    let exit = boiler.loaded.encounter.exit;
    assert!(!hides_piece(
        exit,
        TORCH_PIECE,
        exit.x - JAMB_W - 1,
        400,
        16,
        64
    ));
    assert!(!hides_piece(exit, "pipe_v", exit.x, 400, 16, 64));
}

/// The world block of the room's exit gate (its `intact` flag), if the room has one.
fn gate_block(sim: &PartyFixtureSim) -> Option<bool> {
    let env = sim.combat.env()?;
    let range = env.solid_range();
    env.solids()
        .iter()
        .position(|s| matches!(s.kind, EnvSolidKind::ExitGate))
        .map(|i| sim.scene.world.solids[range.start + i].intact)
}

/// Step a cleared run of a `defeat_roster` room twice, one twin drawing the exit on
/// every tick: the drawn frame is `exit_closed` exactly while the simulation's gate
/// block stands, both frames are seen (the roster is cleared), and the drawing never
/// changes the state hash or the event count.
fn check_cleared_run(
    name: &str,
    exit: IRect,
    start: impl Fn() -> PartyFixtureSim,
    pilot: fn(&crate::combat_scene::CombatState, &SceneSim, HeroId) -> RawInput,
) {
    let (mut plain, mut drawn) = (start(), start());
    let layout_h = drawn.scene.world.layout_height();
    let mut seen = [false; 2];
    let mut opened = None;
    let mut lifted = 0u64;
    for k in 0..900u64 {
        let input = pilot(&drawn.combat, &drawn.scene, HeroId::Carl);
        assert_eq!(input, pilot(&plain.combat, &plain.scene, HeroId::Carl));
        plain.step_slots([input, RawInput::NONE]);
        drawn.step_slots([input, RawInput::NONE]);
        let tick = drawn.scene.world.tick;
        let draw = exit_draw(&drawn.combat, exit, layout_h, tick).expect("a 64 x 96 exit");
        let block = gate_block(&drawn).expect("the room has an exit gate");
        assert_eq!(
            draw.frame == ExitFrame::Closed,
            block,
            "{name} tick {k}: the drawn grate is the gate block"
        );
        seen[draw.frame.index()] = true;
        if draw.frame == ExitFrame::Open && opened.is_none() {
            opened = Some(drawn.scene.world.tick);
        }
        // EXIT-LIFT-01: no lift (and no opening tick) before the gate opens; from the
        // opening tick the lift follows the age, then it is gone.
        match opened {
            None => {
                assert_eq!(draw.lift, None, "{name} tick {k}: no draw change before");
                assert_eq!(drawn.combat.env().and_then(|e| e.exit_open_tick()), None);
            }
            Some(open) => {
                assert_eq!(
                    drawn.combat.env().and_then(|e| e.exit_open_tick()),
                    Some(open),
                    "{name} tick {k}: the opening tick is the first open tick"
                );
                let age = tick - open;
                match draw.lift {
                    Some(l) => {
                        assert!(age < LIFT_TICKS, "{name} age {age}");
                        assert_eq!(l.shift, lift_shift(age));
                        // A rigid shift clipped by the lintel: the closed frame's opening
                        // from the kept rows down, on the opening's top, `shift` above the
                        // walking line; the whole opening on the opening tick.
                        assert_eq!(l.src_y + l.h, EXIT_H);
                        assert_eq!(l.y + l.h, draw.y + OPENING_H);
                        assert_eq!((l.x, l.y), (exit.x, draw.y + l.shift));
                        assert!(l.h >= RAISED_ROWS, "the teeth stay under the lintel");
                        if age == 0 {
                            assert_eq!((l.shift, l.h), (0, OPENING_H));
                        }
                        lifted += 1;
                    }
                    None => assert!(age >= LIFT_TICKS, "{name} age {age}: lifting"),
                }
            }
        }
        assert_eq!(plain.state_hash(), drawn.state_hash(), "{name} tick {k}");
        assert_eq!(
            plain.combat.events.len(),
            drawn.combat.events.len(),
            "{name} tick {k}"
        );
    }
    assert!(seen[0] && seen[1], "{name}: closed, then open ({opened:?})");
    assert_eq!(
        lifted, LIFT_TICKS,
        "{name}: the lift lasts LIFT_TICKS ticks"
    );
}

/// Rendering never changes authoritative state, and the drawn exit is the sim's gate
/// (room 6, the aligned `mastery:carl_solo:0` pilot, which clears the room).
#[test]
fn drawing_never_changes_the_state_hash_and_follows_the_gate_in_room_6() {
    let run = f01_mastery::launch(&root()).unwrap();
    check_cleared_run(
        "mastery",
        run.loaded.encounter.exit,
        || {
            f01_mastery_fixtures::start(&root(), f01_mastery_fixtures::Who::CarlSolo, 0)
                .unwrap()
                .0
        },
        f01_mastery_fixtures::pilot,
    );
}

/// The same for room 4 (the aligned `hazards:carl_solo:0` route pilot).
#[test]
fn drawing_never_changes_the_state_hash_and_follows_the_gate_in_room_4() {
    use crate::hazards_fixtures::pilots;
    let run = f01_hazards::launch(&root()).unwrap();
    check_cleared_run(
        "hazards",
        run.loaded.encounter.exit,
        || {
            pilots::start(&root(), pilots::Who::CarlSolo, 0, true)
                .unwrap()
                .0
        },
        pilots::route_pilot,
    );
}

/// Room 2 has no exit gate (`reach_exit`): its exit is drawn open from the first tick.
#[test]
fn the_boiler_exit_is_open() {
    let (run, scene, combat) = f01_late::boiler_start(
        &root(),
        360,
        CarlProgFlags::default(),
        DonutProgFlags::default(),
        None,
    )
    .unwrap();
    let draw = exit_draw(
        &combat,
        run.loaded.encounter.exit,
        scene.world.layout_height(),
        0,
    )
    .unwrap();
    assert_eq!(draw.frame, ExitFrame::Open);
    // EXIT-LIFT-01: a room without an exit gate never lifts.
    assert_eq!(draw.lift, None);
    assert_eq!(combat.env().and_then(|e| e.exit_open_tick()), None);
    assert!(!exit_closed(&combat));
    // World px, y up: the canvas's bottom row on the walking line.
    assert_eq!(
        draw.y,
        scene.world.layout_height() - run.kit.base_walk_line_y
    );
}

/// Another exit size keeps the Guild door (the art is not stretched).
#[test]
fn another_exit_size_is_not_drawn() {
    let (run, scene, combat) = f01_late::boiler_start(
        &root(),
        360,
        CarlProgFlags::default(),
        DonutProgFlags::default(),
        None,
    )
    .unwrap();
    let mut exit = run.loaded.encounter.exit;
    exit.w = 48;
    assert_eq!(
        exit_draw(&combat, exit, scene.world.layout_height(), 0),
        None
    );
}

/// EXIT-LIFT-01 (EXITL-AD1): the lift offset by tick: it starts at 0 on the opening
/// tick, is monotone and eased, never reaches the full rise before `LIFT_TICKS`, and
/// is the full rise from then on.
#[test]
fn the_lift_offset_follows_the_age() {
    assert_eq!(LIFT_TICKS, 16);
    assert_eq!(LIFT_RISE, 80);
    assert_eq!(lift_shift(0), 0);
    assert_eq!(lift_shift(LIFT_TICKS), LIFT_RISE);
    assert_eq!(lift_shift(LIFT_TICKS + 100), LIFT_RISE);
    let shifts: Vec<i32> = (0..=LIFT_TICKS).map(lift_shift).collect();
    assert!(shifts.windows(2).all(|w| w[0] <= w[1]), "{shifts:?}");
    assert!(shifts[(LIFT_TICKS - 1) as usize] < LIFT_RISE, "{shifts:?}");
    // Eased: the first and the last steps are shorter than the middle one.
    let step = |a: usize| shifts[a + 1] - shifts[a];
    assert!(step(0) < step(7) && step(15) < step(7), "{shifts:?}");
}
