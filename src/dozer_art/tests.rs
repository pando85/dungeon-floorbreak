use super::*;
use crate::dozer_fixtures::{HOLD_X, Mode, start};
use crate::input::RawInput;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Every sheet constant is its manifest's (frame size, pivot, frame count,
/// atlas path) and every part's anchor and z are the rig file's.
#[test]
fn the_sheets_and_the_rig_are_the_files() {
    let root = root();
    for sheet in Sheet::ALL {
        let name = sheet
            .path()
            .strip_prefix("generated/art/")
            .and_then(|p| p.strip_suffix(".png"))
            .unwrap();
        let m = json(&root.join(format!("assets/manifests/{name}.json")));
        assert_eq!(m["output_atlas"], sheet.path(), "{name}");
        let (w, h) = sheet.frame_size();
        assert_eq!(m["frame_size"], serde_json::json!([w, h]), "{name}");
        let (px, py) = sheet.pivot();
        assert_eq!(m["pivot"], serde_json::json!([px, py]), "{name}");
        let frames = m["dozer"]["frames"].as_array().unwrap().len() as u32;
        assert_eq!(frames, sheet.frames(), "{name}");
    }
    let rig = json(&root.join("assets/source/enemies/dozer/dozer_rig_h120.json"));
    let parts = rig["parts"].as_array().unwrap();
    for part in Part::ALL {
        let key = match part {
            Part::Hull => "hull_body",
            Part::Tread => "tread",
            Part::Engineer => "engineer",
            Part::RiderA => "rider_a",
            Part::RiderB => "rider_b",
            Part::FrontCab => "hull_front_cab",
            Part::FrontTub => "hull_front_tub",
            Part::Wheel => "wheel",
            Part::Yoke => "yoke",
        };
        let row = parts.iter().find(|p| p["part"] == key).unwrap();
        let (ax, ay) = part.anchor();
        assert_eq!(row["anchor"], serde_json::json!([ax, ay]), "{key}");
        assert_eq!(row["z"], u64::from(part.z()), "{key}");
        let (px, py) = part.sheet().pivot();
        assert_eq!(row["pivot"], serde_json::json!([px, py]), "{key}");
    }
    for (name, (x, y)) in [
        ("stack", STACK),
        ("rear_exhaust", REAR_EXHAUST),
        ("kettle", KETTLE),
    ] {
        assert_eq!(rig["emitters"][name], serde_json::json!([x, y]), "{name}");
    }
    let rider_tags = json(&root.join("assets/manifests/dozer_riders_h36.json"));
    let tag = |i: u8| rider_tags["dozer"]["frames"][usize::from(i)]["tag"].clone();
    for (frame, want) in [
        (rider::ENGINEER_POINT, "engineer_point"),
        (rider::ENGINEER_HOLD, "engineer_hold"),
        (rider::ENGINEER_IDLE_1, "engineer_idle_1"),
        (rider::ENGINEER_LEVER_PULL, "engineer_lever_pull"),
        (rider::ENGINEER_LURCH_FORWARD, "engineer_lurch_forward"),
        (rider::A_HOLD, "rider_a_hold"),
        (rider::A_IDLE_1, "rider_a_idle_1"),
        (rider::A_BRANDISH, "rider_a_brandish"),
        (rider::A_YELL_1, "rider_a_yell_1"),
        (rider::A_LURCH_FORWARD, "rider_a_lurch_forward"),
        (rider::B_HOLD, "rider_b_hold"),
        (rider::B_IDLE_1, "rider_b_idle_1"),
        (rider::B_BRANDISH, "rider_b_brandish"),
        (rider::B_YELL_1, "rider_b_yell_1"),
        (rider::B_LURCH_FORWARD, "rider_b_lurch_forward"),
    ] {
        assert_eq!(tag(frame), want);
    }
}

#[test]
fn the_tread_and_the_wheel_turn_by_distance_only() {
    assert_eq!(
        (0..10).map(|d| tread_frame(d as f32)).collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 0]
    );
    assert_eq!(tread_frame(8.9), 8);
    let arc = 44.0 * std::f32::consts::PI / 24.0;
    assert_eq!(wheel_frame(0.0, 44.0), 0);
    assert_eq!(wheel_frame(arc * 1.01, 44.0), 1);
    assert_eq!(wheel_frame(arc * 4.01, 44.0), 0);
    // The cruise rate (2.67 px a tick) shows a new phase about every 2 ticks, the rig's
    // cruise `wheel: 2`.
    let per_tick = 160.0 * crate::prototype::TICK_SECONDS;
    assert!((arc / per_tick - 2.0).abs() < 0.3, "{}", arc / per_tick);
}

/// The rig's parts sit on their anchors: the hull's pivot on O and the walking
/// line, the wheel's centre on the tuning's axle, in the rig's z order.
#[test]
fn the_rig_parts_sit_on_their_anchors() {
    let (sim, _) = start(&root(), Mode::CarlSolo, 0).unwrap();
    let mut d = crate::dozer_fixtures::dozer_of(&sim).clone();
    d.state = DozerState::Escaped(EscapePhase::Idle);
    d.o = 812.0;
    let sprites = rig_sprites(&d, 0, false);
    assert_eq!(sprites.len(), Part::ALL.len());
    assert!(sprites.windows(2).all(|w| w[0].z <= w[1].z));
    let floor = d.floor_y as i32;
    let hull = sprites[0];
    assert_eq!(hull.sheet, Sheet::Hull);
    assert_eq!((hull.left + 87, hull.bottom), (812, floor));
    let wheel = sprites.iter().find(|s| s.sheet == Sheet::Wheel).unwrap();
    let (ax, ay) = d.axle();
    assert_eq!(
        (wheel.left + 49, wheel.bottom + 99 - 1 - 49),
        (ax as i32, ay as i32)
    );
    for s in &sprites {
        assert!(u32::from(s.frame) < s.sheet.frames(), "{s:?}");
    }
    // At rest there is no rattle: a later tick draws the same rig.
    assert_eq!(rig_sprites(&d, 1, false), sprites);
}

#[test]
fn the_chase_rattles_the_hull_and_the_riders_late_never_the_wheel() {
    let (sim, _) = start(&root(), Mode::CarlSolo, 0).unwrap();
    let mut d = crate::dozer_fixtures::dozer_of(&sim).clone();
    d.state = DozerState::CommittedChase;
    let bottoms = |tick| {
        rig_sprites(&d, tick, false)
            .iter()
            .map(|s| s.bottom)
            .collect::<Vec<_>>()
    };
    let (rest, up) = (bottoms(5), bottoms(0));
    for (n, part) in Part::ALL.into_iter().enumerate() {
        let lifted = up[n] - rest[n];
        match part.rattles() {
            Rattles::Hull => assert_eq!(lifted, 1, "{part:?}"),
            // The riders follow 2 ticks late: still down on tick 0 (tick 4 of the previous
            // cycle), up on ticks 2..=4.
            Rattles::Rider => {
                assert_eq!(lifted, 0, "{part:?}");
                assert_eq!(bottoms(2)[n] - rest[n], 1, "{part:?}");
            }
            Rattles::No => assert_eq!(lifted, 0, "{part:?}"),
        }
    }
}

#[test]
fn every_stand_in_is_labelled_and_the_machine_has_none() {
    let (sim, _) = start(&root(), Mode::CarlSolo, 0).unwrap();
    let base = crate::dozer_fixtures::dozer_of(&sim).clone();
    for state in [
        DozerState::Dormant { ticks: 0 },
        DozerState::Approaching { ticks_left: 40 },
        DozerState::CommittedChase,
        DozerState::Escaped(EscapePhase::Brake { k: 3 }),
        DozerState::Escaped(EscapePhase::Strain { ticks_left: 50 }),
        DozerState::Escaped(EscapePhase::Idle),
        DozerState::ImpactStunned { ticks_left: 10 },
        DozerState::Disabled,
    ] {
        let mut d = base.clone();
        d.state = state;
        let (pieces, tags) = stand_ins(&d, 7);
        assert!(pieces.len() <= MAX_STAND_INS && tags.len() <= MAX_TAGS);
        assert!(!tags.is_empty());
        for p in &pieces {
            assert!(p.label.contains("stand-in until DOZ-02"), "{p:?}");
            // Only the gate, the buffer stop and the damage marker: nothing is drawn over
            // the machine's body except the jam bar of a damage state.
            let gate_or_buffer = p.left + p.w <= 80 || p.left >= BUFFER_X.0;
            assert!(gate_or_buffer || p.label == DAMAGE_LABEL, "{p:?}");
        }
        assert!(puffs(&d, 7).len() <= MAX_PUFFS);
    }
    // The label font draws the tag.
    assert!(crate::label_font::text_pixels(TAG_TEXT).is_some());
}

/// One run through every reachable state: hold at the close call's column until
/// the wheel is 64 px away, run into the tunnel, wait there until the machine
/// idles, then exit.
fn scripted(d: &Dozer, x: f32, exit_x: f32) -> RawInput {
    let run = |to: f32| RawInput {
        move_x: if x < to - 2.0 {
            1.0
        } else if x > to + 2.0 {
            -1.0
        } else {
            0.0
        },
        ..RawInput::NONE
    };
    let close = d.state.emerged() && d.wheel_front() >= HOLD_X - HERO_HALF_WIDTH - 64.0;
    if (HOLD_X..HOLD_X + 64.0).contains(&x) && !close {
        return RawInput::NONE;
    }
    if d.state == DozerState::Escaped(EscapePhase::Idle) {
        run(exit_x)
    } else {
        run(1120.0)
    }
}

/// DR-A acceptance: drawing never changes the hash. A run that draws every tick
/// and a twin that never does stay hash-equal on every tick, through the rumble,
/// the chase (with the riders' yell), the brake, the strain and the idle.
#[test]
fn drawing_never_changes_the_state_hash() {
    let root = root();
    let (mut drawn, run) = start(&root, Mode::CarlSolo, 0).unwrap();
    let (mut plain, _) = start(&root, Mode::CarlSolo, 0).unwrap();
    let exit = run.loaded.encounter.exit;
    let exit_x = (exit.x + exit.w / 2) as f32;
    let mut states = std::collections::BTreeSet::new();
    let mut yelled = false;
    let mut tags_seen = 0;
    for _ in 0..2400 {
        if drawn.scene.world.complete {
            break;
        }
        let world = &drawn.scene.world;
        let heroes = [world.hero.x, world.donut_body.x];
        let art = draw(&drawn.combat, heroes, world.tick).expect("room 3 holds the machine");
        let d = crate::dozer_fixtures::dozer_of(&drawn);
        states.insert(match d.state {
            DozerState::Escaped(EscapePhase::Brake { .. }) => "brake",
            DozerState::Escaped(EscapePhase::Strain { .. }) => "strain",
            DozerState::Escaped(EscapePhase::Idle) => "idle",
            other => other.name(),
        });
        let machine = art
            .sprites
            .iter()
            .filter(|s| s.sheet != Sheet::Smoke)
            .count();
        assert_eq!(machine > 0, d.state.emerged(), "{}", art.state);
        assert!(
            art.sprites
                .iter()
                .filter(|s| s.sheet == Sheet::Smoke)
                .count()
                <= MAX_PUFFS
        );
        yelled |= art
            .sprites
            .iter()
            .any(|s| s.sheet == Sheet::Riders && s.frame == rider::A_YELL_1)
            && d.state == DozerState::CommittedChase;
        tags_seen = tags_seen.max(art.tags.len());
        let input = scripted(d, drawn.scene.world.hero.x, exit_x);
        let input_plain = scripted(
            crate::dozer_fixtures::dozer_of(&plain),
            plain.scene.world.hero.x,
            exit_x,
        );
        assert_eq!(input, input_plain);
        drawn.step_slots([input, RawInput::NONE]);
        plain.step_slots([input_plain, RawInput::NONE]);
        assert_eq!(
            drawn.combat.state_hash(&drawn.scene),
            plain.combat.state_hash(&plain.scene),
            "tick {}",
            drawn.scene.world.tick
        );
    }
    assert!(drawn.scene.world.complete, "the scripted run exits");
    for state in [
        "dormant",
        "approaching",
        "committed_chase",
        "brake",
        "strain",
        "idle",
    ] {
        assert!(states.contains(state), "{state} not drawn: {states:?}");
    }
    assert!(yelled, "the riders yell at the close call");
    assert_eq!(tags_seen, 2);
}
