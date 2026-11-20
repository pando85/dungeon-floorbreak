//! LAB-00 tests: the descriptor loads and rejects what it must, the profile
//! is validated against the real abilities, the leak guard holds, and the
//! headless runner loads, steps deterministically and restarts every room.

use std::path::Path;

use super::*;
use crate::cf02::Cf02Config;
use crate::combat_lab_fixtures::{PROBES, Probe};
use crate::donut_mana::DonutSetup;
use crate::donut_moves::MoveKind;
use crate::input::RawInput;
use crate::party::HeroId;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn catalog() -> LabCatalog {
    LabCatalog::load(root()).unwrap_or_else(|e| panic!("{e}"))
}

fn manifest_text() -> String {
    std::fs::read_to_string(root().join(MANIFEST_PATH)).unwrap()
}

/// Load the shipped manifest after `edit`.
fn mutate(edit: impl FnOnce(&mut Manifest)) -> Result<LabCatalog, LabError> {
    let mut manifest = Manifest::parse(&manifest_text()).unwrap();
    edit(&mut manifest);
    LabCatalog::from_manifest_text(root(), &serde_json::to_string(&manifest).unwrap())
}

fn room_mut<'a>(m: &'a mut Manifest, id: &str) -> &'a mut RoomSpec {
    m.rooms.iter_mut().find(|r| r.id == id).unwrap()
}

fn goblin(x: i32) -> EnemySpec {
    EnemySpec {
        kind: "goblin".into(),
        x,
        ai: "chase".into(),
    }
}

fn launch(room: &str, mode: PlayerMode) -> LabRun {
    catalog()
        .launch(EntryPoint::CombatLab, &LabConfig::new(room, mode))
        .unwrap_or_else(|e| panic!("{room} {mode:?}: {e}"))
}

// ------------------------------------------------------------ the manifest --

#[test]
fn the_manifest_lists_r00_to_r06_and_every_room_validates() {
    let c = catalog();
    let ids: Vec<&str> = c.rooms().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ROOM_IDS);
    // Every room and variant has a report with at least one pass (R06 links).
    let variants: usize = c
        .rooms()
        .iter()
        .map(|r| {
            if r.link.is_some() {
                1
            } else {
                r.variant_ids().len()
            }
        })
        .sum();
    assert_eq!(c.reports().len(), variants);
    assert_eq!(
        variants,
        1 + 4 + 3 + 4 + 1,
        "R01 and R05 have four variants each"
    );
    for report in c.reports() {
        assert!(!report.checks.is_empty(), "{}", report.room);
    }
    // R05's reports carry the honest pendings of unmerged dependencies.
    let pending: Vec<&str> = c
        .report("R05", "v1")
        .unwrap()
        .pending()
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(pending.contains(&"cf02_f_wave_contract"));
    assert!(pending.contains(&"platform_reachability_fixture"));
    // R06 names its open #120 blockers.
    let blockers: Vec<&str> = c
        .report("R06", "v1")
        .unwrap()
        .pending()
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(blockers.iter().any(|b| b.contains("boss-f9")));
    assert!(blockers.iter().any(|b| b.contains("boss-f11")));
}

#[test]
fn the_descriptor_records_the_required_fields_and_no_difficulty() {
    let c = catalog();
    let r01 = c.room("R01").unwrap();
    let resolved = r01.resolve("v1").unwrap();
    assert_ne!(resolved.layout_hash, 0);
    assert_ne!(resolved.config_hash(), 0);
    assert_ne!(resolved.content_hash(), resolved.config_hash());
    assert_eq!(r01.allowed_modes, ["solo_carl", "solo_donut", "coop_local"]);
    assert_eq!(r01.profiles, [PROFILE_ID]);
    assert_eq!(r01.checkpoints, [96]);
    assert_eq!(r01.segment.begin, "room_enter");
    assert!(!r01.encounter_id.is_empty());
    // Variants differ in content, so their hashes differ.
    let v2 = r01.resolve("v2").unwrap();
    assert_ne!(v2.config_hash(), resolved.config_hash());
    assert_eq!(v2.layout_hash, resolved.layout_hash);
    let heldout = r01.resolve("h1").unwrap();
    assert_ne!(heldout.layout_hash, resolved.layout_hash);
    // A `difficulty` key is a parse error (LAB-Q08).
    let text = manifest_text().replacen("\"schema\"", "\"difficulty\": 3, \"schema\"", 1);
    assert!(matches!(
        LabCatalog::from_manifest_text(root(), &text),
        Err(LabError::Parse(_))
    ));
    let text = manifest_text().replacen("\"ticks\"", "\"difficulty\": \"hard\", \"ticks\"", 1);
    assert!(matches!(
        LabCatalog::from_manifest_text(root(), &text),
        Err(LabError::Parse(_))
    ));
}

#[test]
fn variants_follow_lab_q07() {
    let c = catalog();
    for id in ["R01", "R05"] {
        let room = c.room(id).unwrap();
        assert_eq!(room.variant_ids(), ["v1", "v2", "v3", "h1"], "{id}");
    }
    for id in ["R00", "R02", "R03", "R04"] {
        assert_eq!(c.room(id).unwrap().variant_ids(), ["v1"], "{id}");
    }
    assert!(
        mutate(|m| {
            let variants = room_mut(m, "R01").variants.clone();
            room_mut(m, "R02").variants = variants;
        })
        .is_err()
    );
}

// --------------------------------------------------------------- the profile --

#[test]
fn the_profile_is_the_post_biscuit_kit_with_no_swat() {
    let c = catalog();
    let p = c.profile(PROFILE_ID).unwrap();
    // The post-biscuit state is the one `--donut-biscuit` starts with.
    assert_eq!(p.donut_setup(), DonutSetup::unlocked(8));
    assert!(p.spec().donut_moves.iter().all(|m| m != "swat"));
    assert_eq!(
        p.effective_moves(false),
        ["jab", "haymaker", "push_kick", "magic_missile", "pounce"]
    );
    // CF02 moves only behind cf02_rules.
    let with = p.effective_moves(true);
    for m in ["grounded_stomp", "follow_through", "dive_kick", "bail_out"] {
        assert!(with.contains(&m));
        assert!(!p.effective_moves(false).contains(&m));
    }
    // Dynamite, the gauntlet fist and the explosives are off.
    for off in ["dynamite", "gauntlet_fist", "explosive", "puddle_jumper"] {
        assert!(!p.allows_move(off));
    }
}

fn profile_error(edit: impl FnOnce(&mut ProfileSpec)) -> LabError {
    match mutate(|m| edit(&mut m.profiles[0])) {
        Err(e) => e,
        Ok(_) => panic!("the profile should have been rejected"),
    }
}

#[test]
fn the_parser_rejects_impossible_and_unimplemented_abilities() {
    for id in [
        "dynamite",
        "gauntlet_fist",
        "puddle_jumper",
        "advanced_donut_spells",
        "dodge",
    ] {
        let e = profile_error(|p| p.carl_moves.push(id.into()));
        assert!(
            matches!(e, LabError::UnimplementedAbility { .. }),
            "{id}: {e}"
        );
        let e = profile_error(|p| p.donut_moves.push(id.into()));
        assert!(
            matches!(e, LabError::UnimplementedAbility { .. }),
            "{id}: {e}"
        );
    }
    let e = profile_error(|p| p.carl_moves.push("fireball".into()));
    assert!(matches!(e, LabError::UnknownAbility { .. }), "{e}");
    assert!(e.to_string().contains("fireball"));
}

#[test]
fn the_parser_rejects_swat_and_misplaced_moves() {
    let e = profile_error(|p| p.donut_moves.push("swat".into()));
    assert!(
        matches!(&e, LabError::IllegalAbility { id, .. } if id == "swat"),
        "{e}"
    );
    assert!(e.to_string().contains("no swat"));
    // A CF02 move is never always-on.
    let e = profile_error(|p| p.carl_moves.push("grounded_stomp".into()));
    assert!(matches!(e, LabError::IllegalAbility { .. }), "{e}");
    // One hero's move in the other's list.
    let e = profile_error(|p| p.carl_moves.push("pounce".into()));
    assert!(matches!(e, LabError::IllegalAbility { .. }), "{e}");
    let e = profile_error(|p| p.donut_moves.push("jab".into()));
    assert!(matches!(e, LabError::IllegalAbility { .. }), "{e}");
    // A non-CF02 move in the gated list.
    let e = profile_error(|p| p.carl_cf02_moves.push("jab".into()));
    assert!(matches!(e, LabError::IllegalAbility { .. }), "{e}");
    // Magic Missile and the pounce are the kit.
    let e = profile_error(|p| p.donut_moves.retain(|m| m != "pounce"));
    assert!(matches!(e, LabError::Schema { .. }), "{e}");
}

#[test]
fn the_parser_rejects_systems_mana_and_story_pin_cheats() {
    for key in [
        "audience",
        "boons",
        "momentum",
        "viewers",
        "sponsors",
        "shoulder_mode",
        "mongo",
        "new_dodge_button",
        "gear_inventory",
    ] {
        let e = profile_error(|p| {
            p.systems.insert(key.into(), true);
        });
        assert!(
            matches!(&e, LabError::IllegalAbility { id, .. } if id == key),
            "{key}: {e}"
        );
    }
    let e = profile_error(|p| {
        p.systems.remove("audience");
    });
    assert!(matches!(e, LabError::Schema { .. }), "{e}");
    let e = profile_error(|p| {
        p.systems.insert("free_loot".into(), false);
    });
    assert!(matches!(e, LabError::UnknownAbility { .. }), "{e}");
    assert!(matches!(
        profile_error(|p| p.donut_mana = "infinite".into()),
        LabError::Schema { .. }
    ));
    assert!(matches!(
        profile_error(|p| p.donut_start_mana = 9),
        LabError::Schema { .. }
    ));
    assert!(matches!(
        profile_error(|p| p.donut_biscuit = false),
        LabError::Schema { .. }
    ));
    assert!(matches!(
        profile_error(|p| p.story_pin = "floor02".into()),
        LabError::Schema { .. }
    ));
    // The pre-biscuit profile is reserved and not defined in v1.
    let e = profile_error(|p| p.id = "floor01-prebiscuit-lab-v1".into());
    assert!(e.to_string().contains("reserved"), "{e}");
}

// ----------------------------------------------------------------- the rooms --

fn room_error(edit: impl FnOnce(&mut Manifest)) -> LabError {
    match mutate(edit) {
        Err(e) => e,
        Ok(_) => panic!("the manifest should have been rejected"),
    }
}

#[test]
fn the_parser_rejects_unknown_enemy_and_move_ids() {
    let e = room_error(|m| {
        room_mut(m, "R01").enemies.push(EnemySpec {
            kind: "blorp".into(),
            x: 600,
            ai: "chase".into(),
        })
    });
    assert!(
        matches!(&e, LabError::UnknownEnemy { id, .. } if id == "blorp"),
        "{e}"
    );
    // Approved art only: no gameplay on main.
    for kind in ["rat", "bomb_bard", "troglodyte", "scatterer", "juicer"] {
        let e = room_error(|m| room_mut(m, "R01").enemies[0].kind = kind.into());
        assert!(
            matches!(&e, LabError::UnimplementedEnemy { id, .. } if id == kind),
            "{kind}: {e}"
        );
    }
    let e = room_error(|m| room_mut(m, "R01").required_moves.push("fireball".into()));
    assert!(matches!(e, LabError::UnknownAbility { .. }), "{e}");
    let e = room_error(|m| room_mut(m, "R01").required_moves.push("dynamite".into()));
    assert!(matches!(e, LabError::UnimplementedAbility { .. }), "{e}");
    // The swat is implemented, but the profile does not allow it.
    let e = room_error(|m| room_mut(m, "R01").required_moves.push("swat".into()));
    assert!(matches!(e, LabError::IllegalAbility { .. }), "{e}");
    let e = room_error(|m| room_mut(m, "R01").enemies[0].ai = "berserk".into());
    assert!(matches!(e, LabError::Schema { .. }), "{e}");
    // A variant roster is checked like the room's.
    let e = room_error(|m| {
        room_mut(m, "R01").variants.as_mut().unwrap()[1]
            .enemies
            .as_mut()
            .unwrap()[0]
            .kind = "rat".into();
    });
    assert!(matches!(e, LabError::UnimplementedEnemy { .. }), "{e}");
}

#[test]
fn the_parser_rejects_gated_hazards_and_unknown_capabilities() {
    for kind in [
        "destructible_cover",
        "destructible_bridge",
        "explosive_trap",
    ] {
        let e = room_error(|m| room_mut(m, "R03").hazards[0].kind = kind.into());
        assert!(e.to_string().contains("TERRAIN-L1/L3"), "{kind}: {e}");
    }
    let e = room_error(|m| room_mut(m, "R03").hazards[0].kind = "lava".into());
    assert!(matches!(e, LabError::Unknown { .. }), "{e}");
    let e = room_error(|m| {
        room_mut(m, "R03")
            .optional_capabilities
            .push("jetpack".into())
    });
    assert!(matches!(e, LabError::Unknown { .. }), "{e}");
    // A gated capability is declared, and the room still loads without it.
    let c = catalog();
    assert!(
        c.room("R03")
            .unwrap()
            .optional_capabilities
            .iter()
            .any(|c| c.starts_with("terrain_l1"))
    );
}

#[test]
fn the_parser_rejects_unresolvable_geometry_camera_and_hashes() {
    // A goblin inside R02's access steps (not on the walking line).
    let e = room_error(|m| room_mut(m, "R02").enemies[0].x = 430);
    assert!(matches!(e, LabError::Geometry { .. }), "{e}");
    // A checkpoint inside the wall column.
    let e = room_error(|m| room_mut(m, "R01").checkpoints = vec![8]);
    assert!(matches!(e, LabError::Geometry { .. }), "{e}");
    // A stale layout hash names both values.
    let e = room_error(|m| room_mut(m, "R01").layout_hash = Some("0000000000000001".into()));
    assert!(matches!(e, LabError::HashMismatch { .. }), "{e}");
    // A missing room file.
    let e = room_error(|m| room_mut(m, "R01").room_file = Some("nope.room.json".into()));
    assert!(matches!(e, LabError::Io(_)), "{e}");
    // A path is not a file name.
    let e = room_error(|m| {
        room_mut(m, "R01").room_file = Some("../floor01/demo01_hall.room.json".into())
    });
    assert!(matches!(e, LabError::Schema { .. }), "{e}");
    // Camera: the party wall is 448 px, the views are the scene's.
    let e = room_error(|m| room_mut(m, "R01").camera.max_separation_px = 416);
    assert!(matches!(e, LabError::Schema { .. }), "{e}");
    let e = room_error(|m| room_mut(m, "R01").camera.views = vec!["abc_1920x1080".into()]);
    assert!(matches!(e, LabError::Schema { .. }), "{e}");
    // The shore needs its flat stretch and masonry behind it.
    let e = room_error(|m| room_mut(m, "R03").hazards[0].x = 20);
    assert!(matches!(e, LabError::Geometry { .. }), "{e}");
    // R06 links only the existing boss arena.
    let e = room_error(|m| {
        room_mut(m, "R06").link.as_mut().unwrap().arena =
            "assets/levels/floor01/demo01_hall.room.json".into()
    });
    assert!(matches!(e, LabError::Geometry { .. }), "{e}");
}

#[test]
fn the_parser_rejects_structural_mistakes() {
    let e = room_error(|m| {
        m.rooms.pop();
    });
    assert!(matches!(e, LabError::Schema { .. }), "{e}");
    let e = room_error(|m| room_mut(m, "R01").allowed_modes.push("duo_donut".into()));
    assert!(matches!(e, LabError::Unknown { .. }), "{e}");
    let e = room_error(|m| room_mut(m, "R01").profiles = vec!["floor01-prebiscuit-lab-v1".into()]);
    assert!(matches!(e, LabError::Unknown { .. }), "{e}");
    let e = room_error(|m| room_mut(m, "R01").max_concurrent = 5);
    assert!(matches!(e, LabError::Schema { .. }), "{e}");
    let e = room_error(|m| {
        let r = room_mut(m, "R01");
        r.enemies = vec![
            goblin(480),
            goblin(500),
            goblin(520),
            goblin(540),
            goblin(560),
        ];
    });
    assert!(e.to_string().contains("LAB-Q04"), "{e}");
    let e = room_error(|m| room_mut(m, "R01").segment.end = vec!["win".into()]);
    assert!(matches!(e, LabError::Schema { .. }), "{e}");
    let e = room_error(|m| {
        let link = room_mut(m, "R06").link.clone();
        room_mut(m, "R01").link = link;
    });
    assert!(matches!(e, LabError::Schema { .. }), "{e}");
    // The only manifest schema is v1.
    let e = room_error(|m| m.schema = "combat-lab-manifest-v2".into());
    assert!(matches!(e, LabError::Schema { .. }), "{e}");
}

// ------------------------------------------------------------- the leak guard --

#[test]
fn the_story_demo_cf02_and_boss_entries_cannot_load_the_profile() {
    let c = catalog();
    let cfg = LabConfig::new("R01", PlayerMode::SoloCarl);
    for entry in [
        EntryPoint::Story,
        EntryPoint::Demo,
        EntryPoint::Cf02,
        EntryPoint::Boss,
        EntryPoint::Fixture,
    ] {
        match c.launch(entry, &cfg) {
            Err(LabError::EntryDenied { entry: name, .. }) => assert_eq!(name, entry.name()),
            other => panic!(
                "{entry:?} loaded the lab profile: {:?}",
                other.map(|r| r.state_line())
            ),
        }
        // The same through a session.
        assert!(LabSession::start(c.clone(), entry, cfg.clone()).is_err());
        assert!(reject_lab_profile_arg(entry, PROFILE_ID).is_err());
    }
    assert!(c.launch(EntryPoint::CombatLab, &cfg).is_ok());
    // Plain `--cf02` stays refused even with the Q58 seed set.
    let mut seeded = cfg;
    seeded.cf02 = Cf02Config {
        rules: false,
        seed: Some(7),
    };
    assert!(c.launch(EntryPoint::Cf02, &seeded).is_err());
}

// ---------------------------------------------------------------- the runner --

fn idle(run: &mut LabRun, ticks: u64) {
    for _ in 0..ticks {
        run.step_slots([RawInput::default(); 2]);
    }
}

#[test]
fn every_room_and_mode_loads_and_steps_deterministically() {
    let c = catalog();
    for id in ["R00", "R01", "R02", "R03", "R04", "R05"] {
        let room = c.room(id).unwrap();
        for variant in room.variant_ids() {
            for mode in PlayerMode::ALL {
                let mut cfg = LabConfig::new(id, mode);
                cfg.variant = variant.to_owned();
                let mut a = c
                    .launch(EntryPoint::CombatLab, &cfg)
                    .unwrap_or_else(|e| panic!("{id} {variant} {mode:?}: {e}"));
                let mut b = c.launch(EntryPoint::CombatLab, &cfg).unwrap();
                assert_eq!(
                    a.state_hash(),
                    b.state_hash(),
                    "{id} {variant} {mode:?} at tick 0"
                );
                idle(&mut a, 240);
                idle(&mut b, 240);
                assert_eq!(a.state_hash(), b.state_hash(), "{id} {variant} {mode:?}");
                assert_eq!(a.tick(), 240);
                assert_eq!(a.state_line(), b.state_line());
            }
        }
    }
}

#[test]
fn the_modes_seat_the_right_hero() {
    let carl = launch("R01", PlayerMode::SoloCarl);
    assert_eq!(carl.party.controlled_hero(), HeroId::Carl);
    assert!(!carl.scene.drive().two_player);
    let donut = launch("R01", PlayerMode::SoloDonut);
    assert_eq!(donut.party.controlled_hero(), HeroId::Donut);
    assert!(donut.scene.drive().is_bot(HeroId::Carl) && !donut.scene.drive().is_bot(HeroId::Donut));
    let duo = launch("R01", PlayerMode::CoopLocal);
    assert!(duo.scene.drive().two_player);
    assert_eq!(duo.scene.drive().bots, [false, false]);
    assert_eq!(PlayerMode::CoopLocal.ai_policy(), None);
    assert_eq!(
        PlayerMode::SoloDonut.ai_policy(),
        Some("follow_and_jump_bot_v0")
    );
}

#[test]
fn the_rooms_hold_their_rosters_and_the_shore() {
    let r01 = launch("R01", PlayerMode::SoloCarl);
    assert_eq!(r01.combat.enemies.len(), 3);
    assert!(r01.combat.llamas.is_empty());
    let r02 = launch("R02", PlayerMode::SoloCarl);
    assert_eq!((r02.combat.enemies.len(), r02.combat.llamas.len()), (2, 1));
    let r03 = launch("R03", PlayerMode::SoloCarl);
    assert!(r03.combat.shore.is_some(), "R03 carries the authored shore");
    assert!(launch("R01", PlayerMode::SoloCarl).combat.shore.is_none());
    let r00 = launch("R00", PlayerMode::CoopLocal);
    assert!(r00.combat.enemies.is_empty() && r00.combat.llamas.is_empty());
    assert!(!r00.goal_met(), "the selection room has no goal");
    // At most four concurrent enemies in every natural room (LAB-Q04).
    for room in catalog().rooms() {
        assert!(room.enemies.len() <= LAB_MAX_CONCURRENT, "{}", room.id);
    }
}

#[test]
fn a_probe_replays_to_the_same_hash_twice_and_the_donut_kit_is_post_biscuit() {
    for probe in PROBES {
        let mut cfg = LabConfig::new(probe.room, probe.mode);
        cfg.variant = probe.variant.to_owned();
        let c = catalog();
        let mut a = c.launch(EntryPoint::CombatLab, &cfg).unwrap();
        let mut b = c.launch(EntryPoint::CombatLab, &cfg).unwrap();
        assert_eq!(a.combat.donut_mana_view(), Some((8, 8)), "{}", probe.name);
        let mut swats = 0;
        let mut min_mana = 8;
        for _ in 0..probe.ticks {
            let tick = a.tick() + 1;
            a.step_slots(probe.slot_inputs(tick));
            b.step_slots(probe.slot_inputs(tick));
            if let Some(view) = a.combat.donut_move_view()
                && view.mv == MoveKind::Swat
            {
                swats += 1;
            }
            if let Some((mana, _)) = a.combat.donut_mana_view() {
                min_mana = min_mana.min(mana);
            }
        }
        assert_eq!(a.state_hash(), b.state_hash(), "{}", probe.name);
        assert_eq!(a.state_line(), b.state_line(), "{}", probe.name);
        assert_eq!(swats, 0, "{}: the lab profile has no swat", probe.name);
        assert!(
            !a.combat.events.is_empty(),
            "{}: nothing happened",
            probe.name
        );
        if probe.mode != PlayerMode::SoloCarl {
            assert!(min_mana < 8, "{}: Donut never spent real mana", probe.name);
        }
    }
    assert!(Probe::by_name("lab_r01_duo_missiles").is_some());
}

#[test]
fn a_dry_press_at_zero_mana_is_not_a_swat() {
    let c = catalog();
    let mut cfg = LabConfig::new("R01", PlayerMode::CoopLocal);
    cfg.checkpoint = 0;
    let mut run = c.launch(EntryPoint::CombatLab, &cfg).unwrap();
    // Spend the pool with presses spaced past the refire time, then press dry.
    let press = RawInput {
        attack: true,
        ..RawInput::NONE
    };
    let mut dry_swat = false;
    let mut emptied = false;
    for tick in 1..=1600u64 {
        let slot1 = if tick % 20 < 2 {
            press
        } else {
            RawInput::default()
        };
        run.step_slots([RawInput::default(), slot1]);
        if let Some(view) = run.combat.donut_move_view() {
            dry_swat |= view.mv == MoveKind::Swat;
        }
        emptied |= run
            .combat
            .donut_mana_view()
            .is_some_and(|(mana, _)| mana == 0);
    }
    assert!(
        emptied,
        "the pool never reached 0, so no dry press was tested"
    );
    assert!(!dry_swat);
}

#[test]
fn a_restart_returns_to_the_launch_state_and_keeps_the_seed() {
    let mut run = launch("R01", PlayerMode::CoopLocal);
    let fresh = run.state_hash();
    let probe = &crate::combat_lab_fixtures::LAB_R01_DUO_MISSILES;
    run.run_probe(probe, 300);
    assert_ne!(run.state_hash(), fresh);
    run.restart().unwrap();
    assert_eq!(run.state_hash(), fresh);
    assert_eq!(run.tick(), 0);
    // And it plays out the same again.
    run.run_probe(probe, 300);
    let mut again = launch("R01", PlayerMode::CoopLocal);
    again.run_probe(probe, 300);
    assert_eq!(run.state_hash(), again.state_hash());
    // A solo Donut restart keeps Donut seated.
    let mut donut = launch("R01", PlayerMode::SoloDonut);
    let start = donut.state_hash();
    idle(&mut donut, 50);
    donut.restart().unwrap();
    assert_eq!(donut.state_hash(), start);
    assert_eq!(donut.party.controlled_hero(), HeroId::Donut);
}

#[test]
fn the_session_selects_rooms_restarts_and_keeps_every_episode() {
    let mut s = LabSession::start(
        catalog(),
        EntryPoint::CombatLab,
        LabConfig::new("R00", PlayerMode::SoloCarl),
    )
    .unwrap();
    assert_eq!((s.episode(), s.run().info.room.as_str()), (1, "R00"));
    s.select_room("R01").unwrap();
    assert_eq!((s.episode(), s.run().info.room.as_str()), (2, "R01"));
    let launch_hash = s.run().state_hash();
    s.run_mut()
        .run_probe(&crate::combat_lab_fixtures::LAB_R01_CARL_BRAWL, 120);
    s.restart().unwrap();
    assert_eq!(s.episode(), 3);
    assert_eq!(s.run().state_hash(), launch_hash);
    s.select_variant("v2").unwrap();
    assert_eq!(s.run().info.variant, "v2");
    s.abandon().unwrap();
    let ends: Vec<_> = s
        .history()
        .iter()
        .map(|r| (r.room.as_str(), r.end))
        .collect();
    assert_eq!(
        ends,
        [
            ("R00", EpisodeEnd::RoomChange),
            ("R01", EpisodeEnd::Retry),
            ("R01", EpisodeEnd::RoomChange),
            ("R01", EpisodeEnd::Abandon),
        ]
    );
    assert_eq!(s.history()[1].ticks, 120);
    assert_eq!(s.history()[0].segment, "seg-r00");
    // A refused change leaves the session as it was.
    let before = s.run().state_hash();
    assert!(s.select_room("R09").is_err());
    assert!(s.select_room("R06").is_err());
    assert_eq!(
        (s.run().state_hash(), s.config().room.as_str()),
        (before, "R01")
    );
}

#[test]
fn a_cleared_room_ends_as_a_clear() {
    let mut s = LabSession::start(
        catalog(),
        EntryPoint::CombatLab,
        LabConfig::new("R01", PlayerMode::SoloCarl),
    )
    .unwrap();
    // Defeat the roster by hand: the goal reads the real combat state.
    assert!(!s.run().goal_met());
    for g in &mut s.run_mut().combat.enemies {
        g.health = 0;
    }
    assert!(s.run().goal_met());
    s.restart().unwrap();
    assert_eq!(s.history()[0].end, EpisodeEnd::Clear);
}

#[test]
fn the_seed_is_the_cf02_effective_seed_and_the_lab_has_no_rng() {
    let c = catalog();
    let base = LabConfig::new("R01", PlayerMode::SoloCarl);
    let fixture_seed = c.room("R01").unwrap().resolve("v1").unwrap().seed;
    let a = c.launch(EntryPoint::CombatLab, &base).unwrap();
    assert_eq!(a.info.seed, fixture_seed);
    assert_eq!(effective_seed(Cf02Config::OFF, 5), 5);
    assert_eq!(
        effective_seed(
            Cf02Config {
                rules: false,
                seed: Some(9)
            },
            5
        ),
        9
    );
    let mut seeded = base.clone();
    seeded.cf02.seed = Some(42);
    let (mut s1, mut s2) = (
        c.launch(EntryPoint::CombatLab, &seeded).unwrap(),
        c.launch(EntryPoint::CombatLab, &seeded).unwrap(),
    );
    assert_eq!(s1.info.seed, 42);
    idle(&mut s1, 100);
    idle(&mut s2, 100);
    assert_eq!(s1.state_hash(), s2.state_hash());
    // Another seed is another hash (the seed is in the hash on main).
    assert_ne!(s1.state_hash(), {
        let mut o = a;
        idle(&mut o, 100);
        o.state_hash()
    });
    // The same effective seed on the same room reuses one fixture.
    let again = c.launch(EntryPoint::CombatLab, &seeded).unwrap();
    assert!(std::ptr::eq(again.info.fixture, s1.info.fixture));
}

#[test]
fn cf02_rules_are_refused_until_they_are_integrated() {
    let c = catalog();
    let mut cfg = LabConfig::new("R01", PlayerMode::SoloCarl);
    cfg.cf02 = Cf02Config {
        rules: true,
        seed: None,
    };
    assert_eq!(
        c.launch(EntryPoint::CombatLab, &cfg).err(),
        Some(LabError::Cf02NotIntegrated)
    );
    // The legacy run lists no CF02 move.
    let run = launch("R01", PlayerMode::SoloCarl);
    assert!(run.info.moves.iter().all(|m| m != "grounded_stomp"));
    assert!(run.info.moves.iter().any(|m| m == "magic_missile"));
}

#[test]
fn modes_probes_and_rooms_are_gated() {
    let c = catalog();
    // R06 links the existing boss scene and names its blockers.
    match c.launch(
        EntryPoint::CombatLab,
        &LabConfig::new("R06", PlayerMode::SoloCarl),
    ) {
        Err(LabError::Linked {
            scene, blockers, ..
        }) => {
            assert_eq!(scene, "boss_f01");
            assert_eq!(blockers.len(), 2);
        }
        other => panic!("{:?}", other.map(|r| r.state_line())),
    }
    assert!(matches!(
        c.launch(
            EntryPoint::CombatLab,
            &LabConfig::new("R99", PlayerMode::SoloCarl)
        ),
        Err(LabError::Unknown { .. })
    ));
    let mut cfg = LabConfig::new("R01", PlayerMode::SoloCarl);
    cfg.variant = "v9".into();
    assert!(matches!(
        c.launch(EntryPoint::CombatLab, &cfg),
        Err(LabError::Unknown { .. })
    ));
    cfg.variant = "v1".into();
    cfg.checkpoint = 3;
    assert!(matches!(
        c.launch(EntryPoint::CombatLab, &cfg),
        Err(LabError::NotAllowed { .. })
    ));
    cfg.checkpoint = 0;
    cfg.view_h = 1080;
    assert!(matches!(
        c.launch(EntryPoint::CombatLab, &cfg),
        Err(LabError::NotAllowed { .. })
    ));
    // A room that allows one mode only.
    let narrow = mutate(|m| room_mut(m, "R01").allowed_modes = vec!["solo_carl".into()]).unwrap();
    assert!(
        narrow
            .launch(
                EntryPoint::CombatLab,
                &LabConfig::new("R01", PlayerMode::CoopLocal)
            )
            .is_err()
    );
    assert!(
        narrow
            .launch(
                EntryPoint::CombatLab,
                &LabConfig::new("R01", PlayerMode::SoloCarl)
            )
            .is_ok()
    );
}

#[test]
fn sandbox_and_stress_are_tagged_probes() {
    let c = catalog();
    let mut cfg = LabConfig::new("R01", PlayerMode::SoloCarl);
    cfg.enemies_override = Some(vec![goblin(700)]);
    assert!(matches!(
        c.launch(EntryPoint::CombatLab, &cfg),
        Err(LabError::NotAllowed { .. })
    ));
    cfg.probe = ProbeTag::Sandbox;
    let run = c.launch(EntryPoint::CombatLab, &cfg).unwrap();
    assert_eq!(run.combat.enemies.len(), 1);
    assert_eq!(run.info.probe, ProbeTag::Sandbox);
    assert!(run.state_line().contains("probe=sandbox"));
    // Density above the cap is a stress probe only.
    let many: Vec<EnemySpec> = (0..6).map(|i| goblin(600 + 40 * i)).collect();
    cfg.enemies_override = Some(many);
    assert!(c.launch(EntryPoint::CombatLab, &cfg).is_err());
    cfg.probe = ProbeTag::Stress;
    assert_eq!(
        c.launch(EntryPoint::CombatLab, &cfg)
            .unwrap()
            .combat
            .enemies
            .len(),
        6
    );
    // A configured roster is validated like a descriptor's.
    cfg.enemies_override = Some(vec![EnemySpec {
        kind: "rat".into(),
        x: 600,
        ai: "chase".into(),
    }]);
    assert!(matches!(
        c.launch(EntryPoint::CombatLab, &cfg),
        Err(LabError::UnimplementedEnemy { .. })
    ));
    cfg.enemies_override = Some(vec![goblin(8)]);
    assert!(matches!(
        c.launch(EntryPoint::CombatLab, &cfg),
        Err(LabError::Geometry { .. })
    ));
}

#[test]
fn direct_entry_at_a_checkpoint_and_the_selection_lists() {
    let c = catalog();
    let mut cfg = LabConfig::new("R05", PlayerMode::SoloCarl);
    cfg.checkpoint = 1;
    let run = c.launch(EntryPoint::CombatLab, &cfg).unwrap();
    assert_eq!(run.info.carl_x, 1000);
    assert!((run.scene.world.hero.x - 1000.0).abs() < 1.0);
    // R00's fixed lists.
    let options = c.setup_options();
    assert_eq!(options.profiles, [PROFILE_ID]);
    assert_eq!(options.modes.len(), 3);
    assert_eq!(options.rooms.len(), 7);
    assert!(options.rooms[6].1.is_empty());
    assert_eq!(options.rooms[1].1, ["v1", "v2", "v3", "h1"]);
    let picked = LabConfig::from_selection(&c, PROFILE_ID, "coop_local", "R05", "h1").unwrap();
    assert_eq!(
        (picked.mode, picked.variant.as_str()),
        (PlayerMode::CoopLocal, "h1")
    );
    assert!(c.launch(EntryPoint::CombatLab, &picked).is_ok());
    for bad in [
        LabConfig::from_selection(&c, "floor01-prebiscuit-lab-v1", "coop_local", "R05", "v1"),
        LabConfig::from_selection(&c, PROFILE_ID, "hard_mode", "R05", "v1"),
        LabConfig::from_selection(&c, PROFILE_ID, "solo_carl", "R09", "v1"),
        LabConfig::from_selection(&c, PROFILE_ID, "solo_carl", "R05", "v7"),
    ] {
        assert!(bad.is_err());
    }
}

#[test]
fn a_reach_exit_goal_keeps_the_protected_exit() {
    let c = mutate(|m| room_mut(m, "R02").goal.kind = GoalKind::ReachExit).unwrap();
    let run = c
        .launch(
            EntryPoint::CombatLab,
            &LabConfig::new("R02", PlayerMode::SoloCarl),
        )
        .unwrap();
    assert!(run.kit.exit.is_some());
    assert!(!run.goal_met());
    let clear = launch("R02", PlayerMode::SoloCarl);
    assert!(clear.kit.exit.is_none());
}

#[test]
fn the_rooms_have_both_camera_views_and_donut_only_height() {
    for id in ["R00", "R01", "R02", "R03", "R04", "R05"] {
        let run = launch(id, PlayerMode::SoloCarl);
        for view in ["abc_640x360", "abc_640x400"] {
            assert!(run.kit.view(view).is_some(), "{id} {view}");
        }
    }
    let mut deck = LabConfig::new("R02", PlayerMode::SoloCarl);
    deck.view_h = 400;
    assert!(catalog().launch(EntryPoint::CombatLab, &deck).is_ok());
}
