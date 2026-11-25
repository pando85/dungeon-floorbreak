use super::*;
use crate::combat_scene::EnvSolidKind;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn hash(path: &str) -> u64 {
    crate::donut_mana::content_hash(
        &std::fs::read_to_string(root().join(path)).expect("the file reads"),
    )
}

#[test]
fn the_pinned_hashes_are_the_files() {
    assert_eq!(hash(ROOM_PATH), ROOM_HASH, "{:#018x}", hash(ROOM_PATH));
    assert_eq!(
        hash(ENCOUNTER_PATH),
        ENCOUNTER_HASH,
        "{:#018x}",
        hash(ENCOUNTER_PATH)
    );
    let run = launch(&root()).expect("the room launches");
    assert_eq!(run.loaded.room_hash, ROOM_HASH);
    assert_eq!(run.loaded.sidecar_hash, ENCOUNTER_HASH);
    for (key, value) in data_pins() {
        assert_ne!(value, 0, "{key}");
    }
}

/// An edited sidecar is refused until it is re-pinned on purpose.
#[test]
fn an_unpinned_edit_is_refused() {
    let dir = std::env::temp_dir().join(format!("corralw-edit-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("assets/levels/floor01")).unwrap();
    std::fs::create_dir_all(dir.join("assets/data/gameplay")).unwrap();
    let text = std::fs::read_to_string(root().join(ENCOUNTER_PATH)).unwrap();
    let edited = text.replace("\"x\": 416", "\"x\": 420");
    assert_ne!(edited, text);
    std::fs::write(dir.join(ENCOUNTER_PATH), edited).unwrap();
    for rel in [ROOM_PATH, crate::encounter::HEIGHT_CLASSES_PATH] {
        std::fs::copy(root().join(rel), dir.join(rel)).unwrap();
    }
    let err = launch(&dir).expect_err("an unpinned edit");
    assert!(err.contains("re-pin src/f01_corral.rs"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The launched room is the CORRAL-C room: its kit has the authored steps and
/// the lane ceiling, its llamas carry the CORRAL-C opt-ins (the same name hook
/// the evaluator's `corral_m1` and `corral_m2` use) and the M3 gate stands.
#[test]
fn the_room_builds_its_roster_opt_ins_steps_and_gate() {
    let run = launch(&root()).unwrap();
    assert_eq!(run.fixture.name, SCENE_NAME);
    assert_eq!(run.fixture.enemies.len(), 2, "M0's two goblins");
    assert_eq!(run.fixture.llamas.len(), 3, "M1's llama, B and A");
    assert_eq!(
        run.steps.len(),
        3,
        "the entry shelf, the bumper and the dock"
    );
    assert_eq!(run.rules.checkpoints, &[112, 1072]);
    let (gate, llamas) = run.m3_gate.clone().expect("the CRL-Q26 switch is on");
    assert_eq!(llamas, vec![1, 2]);
    assert_eq!((gate.x, gate.right()), (2048, 2064));
    assert_eq!(gate.bottom(), run.loaded.room.floor_y);
    let opt = room::llama_opt_ins(SCENE_NAME).expect("the name hook");
    assert!(opt.party_target);
    assert_eq!(opt.opt_ins.len(), 3);
    assert!(opt.opt_ins[0].rush.is_none(), "the M1 llama spits only");
    assert!(opt.opt_ins[1].rush.is_some() && opt.opt_ins[2].rush.is_some());
    assert!(opt.opt_ins[1].dormant && opt.opt_ins[2].dormant);
    assert!(opt.opt_ins[2].feet.is_some(), "A stands on the dock");
    let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
    let combat = combat_of(&run, &mut scene).unwrap();
    assert_eq!(combat.llamas.len(), 3);
    let env = combat.env().expect("the env step");
    let gates: Vec<_> = env
        .solids()
        .iter()
        .filter(|s| s.kind == EnvSolidKind::ExitGate)
        .collect();
    assert_eq!(gates.len(), 1);
    assert!(!combat.exit_gate_open());
    let line = entry_line(&run);
    assert!(
        line.contains("F01_CORRAL_ENTRY") && line.contains("listed=false"),
        "{line}"
    );
}

/// The wired llamas are the CORRAL-C rush llamas: Carl parked still on the pen
/// floor inside B's trigger (CRL-Q05) is rushed by B, while the M1 llama, which
/// has no profile, never rushes. This is the wiring check; the route pilots
/// close in fast and usually win before a rush, so they do not assert one.
#[test]
fn a_parked_carl_inside_b_trigger_is_rushed_by_b() {
    let run = launch(&root()).unwrap();
    let park = room::B_X - 112;
    let mut scene = SceneSim::new_at(&run.kit, 360, park).unwrap();
    let mut combat = combat_of(&run, &mut scene).unwrap();
    assert!(combat.llamas[0].corral.is_none());
    assert!(combat.llamas[1].corral.is_some() && combat.llamas[2].corral.is_some());
    for _ in 0..240 {
        combat.step(&mut scene, RawInput::NONE);
    }
    let commits: Vec<u32> = combat
        .events
        .iter()
        .filter_map(|e| match *e {
            crate::combat::CombatEvent::LlamaRushCommit { llama, .. } => Some(llama),
            _ => None,
        })
        .collect();
    assert!(
        commits.contains(&1),
        "B rushes the parked Carl: {commits:?}"
    );
    assert!(!commits.contains(&0), "the M1 llama never rushes");
}

/// CRL-Q26: the gate waits on the M2 llamas only. M0's goblins and the M1
/// llama alive do not hold it; it opens on the tick the second M2 llama is
/// defeated, and its world block stops standing on that sync.
#[test]
fn the_m3_gate_opens_on_the_second_m2_defeat_only() {
    let run = launch(&root()).unwrap();
    let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
    let mut combat = combat_of(&run, &mut scene).unwrap();
    let block = |combat: &CombatState, scene: &SceneSim| {
        let env = combat.env().unwrap();
        let i = env
            .solids()
            .iter()
            .position(|s| s.kind == EnvSolidKind::ExitGate)
            .unwrap();
        scene.world.solids[env.solid_range().start + i].intact
    };
    assert!(block(&combat, &scene));
    combat.llamas[1].health = 0;
    combat.sync_env(&mut scene);
    assert!(!combat.exit_gate_open(), "one M2 llama left");
    assert!(block(&combat, &scene));
    combat.llamas[2].health = 0;
    combat.sync_env(&mut scene);
    assert!(combat.exit_gate_open(), "both M2 llamas down");
    assert!(!block(&combat, &scene));
    assert!(combat.llamas[0].alive() && combat.enemies.iter().any(|g| g.alive()));
}

#[test]
fn the_room_replays_to_the_same_hash_twice_and_the_seed_matters() {
    let run_once = |seed: Option<u64>| {
        let run = launch(&root()).unwrap();
        let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
        let mut combat = combat_for(
            &run,
            &mut scene,
            CarlProgFlags::default(),
            DonutProgFlags::default(),
            seed,
        )
        .unwrap();
        for tick in 1..=400u64 {
            combat.step(&mut scene, run.fixture.input(tick.min(SMOKE_TICKS)));
        }
        combat.state_hash(&scene)
    };
    assert_eq!(run_once(None), run_once(None));
    assert_eq!(run_once(Some(3)), run_once(Some(3)));
    assert_ne!(run_once(None), run_once(Some(3)));
}

/// KC9, KC11: room 0 holds nothing and arms nothing under the story loadout.
#[test]
fn the_story_loadout_holds_nothing_in_room_0() {
    use crate::story_unlock::SPECIAL_THROWN_CHARGE;
    let run = launch(&root()).unwrap();
    let (carl, donut) = crate::f01_late::Loadout::Story.flags();
    let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
    let combat = combat_for(&run, &mut scene, carl, donut, None).unwrap();
    assert!(!combat.story_holds(SPECIAL_THROWN_CHARGE));
    assert!(!StoryTable::active().has_pin(&run.loaded.encounter.story_pin));
}

#[test]
fn a_roster_off_the_authored_columns_is_refused() {
    let run = launch(&root()).unwrap();
    let mut roster = run.loaded.encounter.roster.clone();
    assert_eq!(check_roster(&roster), Ok(vec![1, 2]));
    roster[3].x += 8;
    let err = check_roster(&roster).unwrap_err();
    assert!(err.contains("authored for"), "{err}");
    let mut roster = run.loaded.encounter.roster.clone();
    roster[0].kind = RosterKind::Room(EnemyKind::Hoarder);
    assert!(check_roster(&roster).unwrap_err().contains(&roster[0].id));
}
