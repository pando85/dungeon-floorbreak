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
        // The ENV-02 splinter burst is a kit piece: its frames are its grid's.
        let frames = match m["dozer"]["frames"].as_array() {
            Some(frames) => frames.len(),
            None => json(&root.join(m["grid_source"].as_str().unwrap()))["frames"]
                .as_array()
                .unwrap()
                .len(),
        } as u32;
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
    // The hit-spark is the rig's `hit_spark` row (z 8, 2 ticks a frame); the
    // splinter burst's 67 ms frames are 4 ticks at 60 Hz.
    let spark = parts.iter().find(|p| p["part"] == "hit_spark").unwrap();
    assert_eq!(spark["z"], u64::from(FX_Z));
    assert_eq!(spark["atlas"], "vfx_hit_spark_metal");
    assert_eq!(
        rig["timing"]["frame_ticks"]["vfx_hit_spark_metal"],
        serde_json::json!([SPARK_FRAME_TICKS; 3].to_vec())
    );
    let kit = json(&root.join("assets/source/terrain/floor01/floor01_opening_kit.json"));
    let burst = kit
        .pointer("/pieces")
        .and_then(|p| p.as_array())
        .and_then(|p| p.iter().find(|p| p["id"] == "timber_splinter_burst"))
        .expect("the ENV-02 kit lists timber_splinter_burst");
    assert_eq!(burst["atlas"], "floor01_splinter_burst");
    assert_eq!(burst["anchor"], serde_json::json!([24, 24]));
    for f in burst["animation"]["frames"].as_array().unwrap() {
        let ms = f["duration"].as_u64().unwrap();
        assert_eq!((ms * 60 + 500) / 1000, SPLINTER_FRAME_TICKS);
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
    let sprites = rig_sprites(&d, 0, false, [true; 2]);
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
    assert_eq!(rig_sprites(&d, 1, false, [true; 2]), sprites);
}

#[test]
fn the_chase_rattles_the_hull_and_the_riders_late_never_the_wheel() {
    let (sim, _) = start(&root(), Mode::CarlSolo, 0).unwrap();
    let mut d = crate::dozer_fixtures::dozer_of(&sim).clone();
    d.state = DozerState::CommittedChase;
    let bottoms = |tick| {
        rig_sprites(&d, tick, false, [true; 2])
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

/// The gate and the buffer stop are atlas sprites (DOZ-02): eight gate tiles stack to the lane ceiling
/// with the last one cropped, the frame follows the machine's state, and nothing is a code-drawn rectangle.
#[test]
fn the_room_pieces_are_atlas_sprites() {
    let (sim, _) = start(&root(), Mode::CarlSolo, 0).unwrap();
    let base = crate::dozer_fixtures::dozer_of(&sim).clone();
    for (state, gate_frame) in [
        (DozerState::Dormant { ticks: 0 }, 0),
        (DozerState::Approaching { ticks_left: 40 }, 1),
        (DozerState::CommittedChase, 3),
        (DozerState::Escaped(EscapePhase::Brake { k: 3 }), 3),
        (
            DozerState::Escaped(EscapePhase::Strain { ticks_left: 50 }),
            3,
        ),
        (DozerState::Escaped(EscapePhase::Idle), 3),
        (DozerState::ImpactStunned { ticks_left: 10 }, 3),
        (DozerState::Disabled, 3),
    ] {
        let mut d = base.clone();
        d.state = state;
        let mut pieces = Vec::new();
        room_pieces(&d, 0, &mut pieces);
        let gate: Vec<_> = pieces.iter().filter(|p| p.sheet == Sheet::Gate).collect();
        assert_eq!(gate.len(), 8, "{state:?}");
        assert!(gate.iter().all(|t| t.frame == gate_frame && t.z == Z_BACK));
        let height: u32 = gate.iter().map(|t| t.visible().1).sum();
        assert_eq!(
            height, LANE_H_PX as u32,
            "the gate stops at the lane ceiling"
        );
        assert_eq!(gate.iter().filter(|t| t.crop_top > 0).count(), 1);
        let buffer: Vec<_> = pieces.iter().filter(|p| p.sheet == Sheet::Buffer).collect();
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0].left, BUFFER_X.0);
        assert_eq!(buffer[0].bottom, d.floor_y.round() as i32);
        assert_eq!(pieces.len(), 9);
        assert!(puffs(&d, 7).len() <= MAX_PUFFS);
    }
    // The rumble shakes the gate: shake_0 then shake_1, 3 ticks each.
    let mut d = base.clone();
    d.state = DozerState::Approaching { ticks_left: 40 };
    let frame_at = |tick| {
        let mut p = Vec::new();
        room_pieces(&d, tick, &mut p);
        p[0].frame
    };
    assert_eq!(
        (0..12).map(frame_at).collect::<Vec<_>>(),
        [1, 1, 1, 2, 2, 2, 1, 1, 1, 2, 2, 2]
    );
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
    let mut gate_bursts = 0;
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
        // The rig's own parts (z 1..=7, the tumbling crew's Z_TUMBLE): none before the emergence.
        let machine = art
            .sprites
            .iter()
            .filter(|s| (1..=7).contains(&s.z) || s.z == Z_TUMBLE)
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
        // DR-D: the gate bursts as the machine comes out (two staggered bursts).
        for s in art.sprites.iter().filter(|s| s.sheet == Sheet::Splinter) {
            assert_eq!(s.left + 24, (GATE_X.0 + GATE_X.1) / 2);
            gate_bursts += 1;
        }
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
    assert_eq!(gate_bursts, 2 * 4 * SPLINTER_FRAME_TICKS as usize);
}

/// DR-B and DOZ-02: the stop frame's sprites follow the frame through every state (the brace intact,
/// weakened, snapped and swinging; the hanging frame on its pulley and haul line, the warning shake and its
/// growing floor shadow, the fall, the remnant and its dust) and the rear glance plays after a late
/// landing. The pools hold, and drawing never changes the hash.
#[test]
fn the_frame_sprites_follow_the_frame_and_the_rear_glance() {
    use crate::dozer_fixtures::{Stage, chase_at};
    use crate::env_prop::Stimulus;
    use crate::f01_late::Loadout;
    let flags = Loadout::Story.flags();
    let mut st = Stage::new(
        &root(),
        Mode::TwoPlayer,
        flags,
        chase_at(800.0),
        1100.0,
        1150.0,
    )
    .unwrap();
    let mut twin = st.clone();
    let mut seen = std::collections::HashSet::new();
    let mut glance_beams = 0;
    let mut glance_shadows = 0;
    let mut splinters = 0;
    let mut warn_frames = Vec::new();
    for k in 0..240u64 {
        let tick = st.tick();
        if k == 2 || k == 4 {
            // One melee hit weakens the brace; then a blast releases the frame.
            let stimulus = if k == 2 {
                Stimulus::MeleeHit {
                    token: 9001,
                    player: crate::shore::PLAYER_ONE,
                    weight: 1,
                }
            } else {
                Stimulus::Blast {
                    blast_id: 9002,
                    owner: Some(crate::shore::PLAYER_ONE),
                    depth: 0,
                }
            };
            for s in [&mut st, &mut twin] {
                s.sim.combat.env_stimulate(FRAME_PROP, stimulus, tick);
            }
        }
        let world = &st.sim.scene.world;
        let art = draw(&st.sim.combat, [world.hero.x, world.donut_body.x], tick).unwrap();
        for s in &art.sprites {
            assert!(u32::from(s.frame) < s.sheet.frames(), "{s:?}");
            seen.insert((s.sheet, s.frame));
        }
        glance_beams += art
            .sprites
            .iter()
            .filter(|s| s.sheet == Sheet::StopGlance && s.frame < 3)
            .count();
        glance_shadows += art
            .sprites
            .iter()
            .filter(|s| s.sheet == Sheet::StopGlance && s.frame == 3)
            .count();
        if let Some(f) = art
            .sprites
            .iter()
            .find(|s| s.sheet == Sheet::StopFrame && (2..=3).contains(&s.frame))
        {
            warn_frames.push(f.frame);
        }
        // The warning's floor shadow lies behind the actors (their feet stay clear of it).
        assert!(
            art.sprites
                .iter()
                .filter(|s| s.sheet == Sheet::StopShadow)
                .all(|s| s.z == Z_BACK)
        );
        // The pulley stays on the ceiling anchor, in every state.
        assert_eq!(
            art.sprites
                .iter()
                .filter(|s| s.sheet == Sheet::StopFrame && s.frame == 1)
                .count(),
            1,
            "tick {tick}"
        );
        // DR-D: the splinter burst sits on the centre of the brace (its pivot).
        let d = dozer_of(&st.sim.combat).unwrap();
        let brace = brace_centre(d, st.sim.combat.env().unwrap()).unwrap();
        for s in art.sprites.iter().filter(|s| s.sheet == Sheet::Splinter) {
            assert_eq!((s.left + 24, s.bottom + 23), brace, "tick {tick}");
            assert_eq!(s.z, FX_Z);
            splinters += 1;
        }
        assert!(art.sprites.len() <= MAX_SPRITES);
        st.step(RawInput::NONE);
        twin.step(RawInput::NONE);
        assert_eq!(st.hash(), twin.hash(), "tick {tick}");
    }
    for (sheet, frame) in [
        (Sheet::StopBrace, 0),
        (Sheet::StopBrace, 1),
        (Sheet::StopBrace, 2),
        (Sheet::StopBrace, 3),
        (Sheet::StopFrame, 0),
        (Sheet::StopFrame, 1),
        (Sheet::StopFrame, 2),
        (Sheet::StopFrame, 3),
        (Sheet::StopFrame, 4),
        (Sheet::StopShadow, 0),
        (Sheet::StopShadow, 1),
        (Sheet::StopShadow, 2),
        (Sheet::StopShadow, 3),
        (Sheet::StopRemnant, 0),
        (Sheet::Haul, 0),
        (Sheet::Haul, 1),
        (Sheet::Haul, 2),
        (Sheet::StopGlance, 0),
        (Sheet::StopGlance, 1),
        (Sheet::StopGlance, 2),
        (Sheet::StopGlance, 3),
    ] {
        assert!(
            seen.contains(&(sheet, frame)),
            "{sheet:?} {frame} not drawn"
        );
    }
    // Landing dust: the smoke atlas in front of the actors.
    assert!(seen.contains(&(Sheet::Smoke, SMOKE_STEAM_FIRST)));
    // The warning alternates warn_0 and warn_1 every 3 ticks.
    assert!(warn_frames.windows(2).any(|w| w[0] != w[1]));
    let mut runs: Vec<usize> = Vec::new();
    for (n, f) in warn_frames.iter().enumerate() {
        if n > 0 && *f == warn_frames[n - 1] {
            *runs.last_mut().unwrap() += 1;
        } else {
            runs.push(1);
        }
    }
    // Every complete run of a warning frame is 3 ticks (the first and the last may be cut by the
    // warning's edges).
    assert!(runs.len() >= 8, "{runs:?}");
    assert!(runs[1..runs.len() - 1].iter().all(|r| *r == 3), "{runs:?}");
    assert_eq!(glance_beams, GLANCE_TICKS as usize);
    assert_eq!(
        glance_shadows, 8,
        "the shadow lies under the first two frames"
    );
    // The melee hit and the blast's release: one 4-frame burst each.
    assert_eq!(splinters, 2 * 4 * SPLINTER_FRAME_TICKS as usize);
}

/// DOZ-02: the presenter's constants are the contract's numbers.
#[test]
fn the_damage_contract_is_the_presenters() {
    let c = json(&root().join("assets/source/enemies/dozer/dozer_damage_h120.json"));
    assert_eq!(c["format"], "dozer-damage-v0");
    let hull = c["timing"]["impact_stunned"]["hull"].as_array().unwrap();
    assert_eq!(
        hull[0],
        serde_json::json!(["pitch_0", 0, PITCH_TICKS[0] - 1])
    );
    assert_eq!(
        hull[1],
        serde_json::json!([
            "pitch_1",
            PITCH_TICKS[0],
            PITCH_TICKS[0] + PITCH_TICKS[1] - 1
        ])
    );
    assert_eq!(hull[2][1], PITCH_TICKS[0] + PITCH_TICKS[1]);
    for (n, state) in ["pitch_0", "pitch_1", "disabled"].into_iter().enumerate() {
        let seats = &c["pitch"]["rider_seat_dy"][state];
        assert_eq!(
            [
                seats["engineer"].as_i64(),
                seats["rider_a"].as_i64(),
                seats["rider_b"].as_i64()
            ],
            SEAT_DY[n].map(|v| Some(i64::from(v))),
            "{state}"
        );
    }
    assert_eq!(
        c["emitters"]["steam_leak"],
        serde_json::json!([STEAM_LEAK.0, STEAM_LEAK.1])
    );
    assert_eq!(
        c["emitters"]["stack_crooked"],
        serde_json::json!([STACK_CROOKED.0, STACK_CROOKED.1])
    );
    let sf = &c["stop_frame"];
    assert_eq!(sf["warning"]["alternate_ticks"], FRAME_SHAKE_TICKS);
    let glance = sf["glance"]["frames"].as_array().unwrap();
    for (n, f) in glance.iter().enumerate() {
        assert_eq!(f["ticks"], GLANCE_FRAME_TICKS[n]);
        assert_eq!(
            f["offset"],
            serde_json::json!([GLANCE_OFFSETS[n].0, GLANCE_OFFSETS[n].1])
        );
    }
    assert_eq!(GLANCE_FRAME_TICKS.iter().sum::<u64>(), GLANCE_TICKS);
    assert_eq!(sf["brace"]["swing"]["ticks_each"], BRACE_SWING_TICKS);
    assert_eq!(sf["brace"]["swing"]["for_ticks"], BRACE_SWING_FOR);
    let tumble = &c["crew"]["tumble"];
    assert_eq!(tumble["start_tick"]["rider_a"], TUMBLE_START[0]);
    assert_eq!(tumble["start_tick"]["rider_b"], TUMBLE_START[1]);
    assert_eq!(tumble["airborne_ticks"], TUMBLE_AIR_TICKS);
    assert_eq!(c["crew"]["slump"]["ticks_each"], SLUMP_FRAME_TICKS);
    assert_eq!(c["room_pieces"]["gate"]["tile_px"], GATE_TILE_PX);
    // The roster's landing columns (DZ-Q20) are the tumble's ends.
    let encounter = json(&root().join("assets/levels/floor01/f01_dozer.encounter.json"));
    for (n, id) in ["rider_a", "rider_b"].into_iter().enumerate() {
        let row = encounter["roster"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .unwrap();
        assert_eq!(row["dzq20_land_dx"], TUMBLE_LAND_DX[n], "{id}");
    }
    // The contract's glance anchor is the presenter's rule at rig scale: the beam's left end 8 px in from the
    // push box's rear, the beam centre 40 px on, its bottom on the deck.
    let rear = -84;
    assert_eq!(
        c["stop_frame"]["glance"]["anchor_rig"][0],
        rear + GLANCE_INSET_PX + 40
    );
}

/// DOZ-02: a struck machine draws the damage atlases on the contract's schedule: the pitch frames (6 + 6
/// ticks, then level) on the hull, cab and tub, the jammed wheel and the bent yoke from the first tick,
/// the seats on the pitch, the crew thrown out and the driver slumped.
#[test]
fn a_struck_machine_draws_the_damage_atlases_on_the_schedule() {
    let (sim, _) = start(&root(), Mode::CarlSolo, 0).unwrap();
    let mut d = crate::dozer_fixtures::dozer_of(&sim).clone();
    d.o = 700.0;
    let total = d.tuning.dzq15_stunned_ticks;
    assert_eq!(total, 48);
    let floor = d.floor_y as i32;
    for age in 0..=u64::from(total) {
        d.state = if age < u64::from(total) {
            DozerState::ImpactStunned {
                ticks_left: total - age as u16,
            }
        } else {
            DozerState::Disabled
        };
        assert_eq!(stun_age(&d), Some(age));
        let sprites = rig_sprites(&d, age, false, [true; 2]);
        let pitch = pitch_frame(age);
        assert_eq!(
            pitch,
            if age < 6 {
                0
            } else if age < 12 {
                1
            } else {
                2
            }
        );
        let find = |sheet: Sheet| sprites.iter().find(|s| s.sheet == sheet);
        for sheet in [
            Sheet::HullDamage,
            Sheet::FrontCabDamage,
            Sheet::FrontTubDamage,
        ] {
            assert_eq!(find(sheet).unwrap().frame, pitch, "{sheet:?} age {age}");
        }
        assert_eq!(find(Sheet::WheelJammed).unwrap().frame, 0);
        assert_eq!(find(Sheet::YokeBent).unwrap().frame, 0);
        for undamaged in [
            Sheet::Hull,
            Sheet::FrontCab,
            Sheet::FrontTub,
            Sheet::Wheel,
            Sheet::Yoke,
        ] {
            assert!(find(undamaged).is_none(), "{undamaged:?} age {age}");
        }
        // The tread holds still on the floor (never the hull's pitch).
        assert_eq!(find(Sheet::Tread).unwrap().bottom, floor);
        // The seats follow the pitch: the hull's seat rows move with the columns under them.
        let engineer = if age >= LURCH_TICKS {
            find(Sheet::Slump).unwrap()
        } else {
            sprites
                .iter()
                .find(|s| s.sheet == Sheet::Riders && s.frame == rider::ENGINEER_LURCH_FORWARD)
                .unwrap()
        };
        assert_eq!(
            engineer.bottom,
            floor + Part::Engineer.anchor().1 + SEAT_DY[usize::from(pitch)][0]
        );
        assert_eq!(age >= LURCH_TICKS, find(Sheet::Slump).is_some());
        for (i, start) in TUMBLE_START.into_iter().enumerate() {
            let tumbling = sprites
                .iter()
                .any(|s| s.sheet == Sheet::Tumble && (usize::from(s.frame) / 3 == i));
            assert_eq!(tumbling, age >= start, "rider {i} age {age}");
            let lurching = sprites.iter().any(|s| {
                s.sheet == Sheet::Riders
                    && s.frame == [rider::A_LURCH_FORWARD, rider::B_LURCH_FORWARD][i]
            });
            assert_eq!(lurching, age < start, "rider {i} age {age}");
        }
    }
}

/// DOZ-02 (DZ2-Q1): a thrown rider leaves its seat, peaks 12 px above the line from seat to landing, and lies
/// on the floor at the column where the simulation's goblin lands (the wheel front plus `dzq20_land_dx`),
/// in the three poses at 6, 24 and every tick after.
#[test]
fn a_thrown_rider_ends_where_the_goblin_lands() {
    let (sim, _) = start(&root(), Mode::CarlSolo, 0).unwrap();
    let mut d = crate::dozer_fixtures::dozer_of(&sim).clone();
    d.o = 700.0;
    d.state = DozerState::Disabled;
    let floor = d.floor_y.round() as i32;
    let seat = (
        700 + Part::RiderA.anchor().0,
        floor + Part::RiderA.anchor().1,
    );
    assert!(tumble_sprite(&d, 0, TUMBLE_START[0] - 1, seat).is_none());
    let mut poses = Vec::new();
    let mut highest = 0;
    for t in 0..40 {
        let s = tumble_sprite(&d, 0, TUMBLE_START[0] + t, seat).unwrap();
        assert_eq!(s.sheet, Sheet::Tumble);
        assert_eq!(s.z, Z_TUMBLE);
        poses.push(s.frame);
        highest = highest.max(s.bottom - floor);
    }
    assert_eq!(poses[0], 0);
    assert_eq!(poses[TUMBLE_POSE_TICKS as usize - 1], 0);
    assert_eq!(poses[TUMBLE_POSE_TICKS as usize], 1);
    assert_eq!(poses[TUMBLE_AIR_TICKS as usize - 1], 1);
    assert_eq!(poses[TUMBLE_AIR_TICKS as usize], 2);
    assert!(
        highest <= seat.1 - floor + TUMBLE_ARC_UP_PX as i32,
        "{highest}"
    );
    for (rider, dx) in TUMBLE_LAND_DX.into_iter().enumerate() {
        let a = Part::ALL[3 + rider].anchor();
        let seat = (700 + a.0, floor + a.1);
        let s = tumble_sprite(&d, rider, TUMBLE_START[rider] + 40, seat).unwrap();
        // Pivot x is 32 px in from the left; the feet are on the walking line.
        assert_eq!(s.left + 32, (d.wheel_front() + dx as f32).round() as i32);
        assert_eq!(s.bottom, floor);
        assert_eq!(u32::from(s.frame), 2 + 3 * rider as u32);
    }
}

/// DOZ-02: the struck machine's steam and smoke: a puff every 6 ticks from the boiler crack over the stun
/// and one black puff from the knocked stack on the first tick; a disabled machine gives one thin puff
/// every 40 ticks and one wisp every 60, nothing else.
#[test]
fn the_leak_and_the_stack_follow_the_damage_schedule() {
    let (sim, _) = start(&root(), Mode::CarlSolo, 0).unwrap();
    let mut d = crate::dozer_fixtures::dozer_of(&sim).clone();
    d.o = 700.0;
    let total = d.tuning.dzq15_stunned_ticks;
    let count = |d: &Dozer, tick: u64, first: u8, last: u8| {
        puffs(d, tick)
            .iter()
            .filter(|p| (first..=last).contains(&p.frame))
            .count()
    };
    let black = SMOKE_BLACK_FIRST;
    let steam = SMOKE_STEAM_FIRST;
    for age in 0..u64::from(total) {
        d.state = DozerState::ImpactStunned {
            ticks_left: total - age as u16,
        };
        let drawn = puffs(&d, age);
        assert!(drawn.len() <= MAX_PUFFS);
        // A steam puff is born every 6 ticks and lives 15: two or three alive at any time.
        let alive = count(&d, age, steam, steam + 4);
        assert!((2..=3).contains(&alive) || age < 12, "age {age}: {alive}");
        assert_eq!(
            count(&d, age, black, black + 5),
            usize::from(age < BLACK_LIFE)
        );
    }
    d.state = DozerState::Disabled;
    let mut steam_puffs = 0;
    let mut wisps = 0;
    for tick in 0..120u64 {
        let drawn = puffs(&d, tick);
        assert!(drawn.len() <= 2);
        steam_puffs += usize::from(tick % DISABLED_STEAM_EVERY == 0);
        wisps += usize::from(tick % DISABLED_WISP_EVERY == 0);
        for p in drawn {
            // Only steam_0..2 and black_3..5 (thin and slow).
            assert!(
                (steam..=steam + 2).contains(&p.frame)
                    || (black + 3..=black + 5).contains(&p.frame),
                "tick {tick}: frame {}",
                p.frame
            );
        }
    }
    assert_eq!((steam_puffs, wisps), (3, 2));
    assert_eq!(count(&d, 0, steam, steam), 1);
    assert_eq!(count(&d, 0, black + 3, black + 3), 1);
    assert_eq!(count(&d, 20, steam, black + 5), 0);
}

/// DR-D: a melee clang sparks on the hull face nearest the hitter, at chest
/// height, for 3 frames of 2 ticks (a blast's clang does not); a defeat retry
/// (a `Restart` in the log) forgets every effect and the rear glance of the
/// earlier attempt, whose ticks the new attempt counts again.
#[test]
fn a_melee_clang_sparks_on_the_hull_face_and_a_retry_forgets_it() {
    use crate::dozer_fixtures::{Stage, idle_at_rest};
    let flags = crate::f01_late::Loadout::Story.flags();
    let st = Stage::new(
        &root(),
        Mode::CarlSolo,
        flags,
        idle_at_rest(),
        600.0,
        1180.0,
    )
    .unwrap();
    let d = dozer_of(&st.sim.combat).unwrap().clone();
    let floor = d.floor_y.round() as i32;
    let rear = d.push_box().left;
    let front = d.wheel_front();
    let at = st.tick();
    let clang = |blast| CombatEvent::Dozer {
        tick: at,
        event: DozerEvent::Clang {
            player: crate::shore::PLAYER_ONE,
            blast,
        },
    };
    let sparks = |c: &CombatState, heroes: [f32; 2], tick: u64| -> Vec<SpriteDraw> {
        draw(c, heroes, tick)
            .unwrap()
            .sprites
            .into_iter()
            .filter(|s| s.sheet == Sheet::Spark)
            .collect()
    };
    let mut c = st.sim.combat.clone();
    c.events.push(clang(true));
    assert!(sparks(&c, [rear - 30.0, 1180.0], at).is_empty(), "a blast");
    c.events.push(clang(false));
    let behind = [rear - 30.0, 1180.0];
    for age in 0..8u64 {
        let drawn = sparks(&c, behind, at + age);
        if age < 3 * SPARK_FRAME_TICKS {
            assert_eq!(drawn.len(), 1, "age {age}");
            let s = drawn[0];
            assert_eq!(u64::from(s.frame), age / SPARK_FRAME_TICKS);
            assert_eq!(
                (s.left + 8, s.bottom + 7),
                (rear.round() as i32, floor + SPARK_CHEST_PX)
            );
        } else {
            assert!(drawn.is_empty(), "age {age}");
        }
    }
    // From ahead (Donut nearer): the wheel front.
    let s = sparks(&c, [rear - 300.0, front + 20.0], at)[0];
    assert_eq!(s.left + 8, front.round() as i32);
    // A retry: the same tick of the next attempt draws nothing.
    c.events.push(CombatEvent::Restart {
        tick: at + 1,
        deaths: 1,
    });
    assert!(sparks(&c, behind, at).is_empty());
    c.events.push(CombatEvent::Dozer {
        tick: at,
        event: DozerEvent::Glance { front_x: 600 },
    });
    assert_eq!(last_glance(&c), Some((at, 600)));
    c.events.push(CombatEvent::Restart {
        tick: at + 2,
        deaths: 2,
    });
    assert_eq!(last_glance(&c), None);
}

/// DR-C: the rig draws a rider on the hull while it is carried and stops on
/// the dismount tick (the goblin presenter draws it from then on), so a rider
/// is never drawn twice; a checkpoint respawn seats the living rider again
/// and leaves a dead rider's seat empty.
#[test]
fn the_rig_draws_a_rider_only_while_it_is_carried() {
    use crate::dozer_fixtures::frame::{brace_pilot, brace_release_r};
    let riders_drawn = |c: &CombatState| {
        let d = dozer_of(c).unwrap();
        let seated = [Seat::RiderA, Seat::RiderB].map(|s| c.seat_taken(s));
        rig_sprites(d, 0, false, seated)
            .iter()
            .filter(|s| matches!(s.sheet, Sheet::Riders | Sheet::Slump | Sheet::Tumble))
            .count()
    };
    let r = root();
    let (mut st, _) = brace_pilot(&r, brace_release_r(&r).unwrap()).unwrap();
    for _ in 0..600 {
        let disabled = dozer_of(&st.sim.combat).unwrap().state == DozerState::Disabled;
        let carried = [0, 1].map(|i| st.sim.combat.is_carried(i));
        assert_eq!(carried, [!disabled; 2], "tick {}", st.tick());
        // The engineer, plus the riders aboard.
        assert_eq!(
            riders_drawn(&st.sim.combat),
            if disabled { 1 } else { 3 },
            "tick {}",
            st.tick()
        );
        if disabled {
            break;
        }
        st.step_slots([RawInput::NONE; 2]);
    }
    assert_eq!(riders_drawn(&st.sim.combat), 1, "never disabled");
    // Rider B dies; the respawn seats rider A only.
    st.sim.combat.enemies[1].health = 0;
    st.sim.combat.enemies[1].mode = crate::goblin::GoblinMode::Corpse;
    st.sim.combat.respawn(&mut st.sim.scene);
    let c = &st.sim.combat;
    assert!(c.seat_taken(Seat::RiderA) && !c.seat_taken(Seat::RiderB));
    assert_eq!(riders_drawn(c), 2);
    // A run without a crew keeps the DR-A/DR-B rig: both seats drawn.
    let (sim, _) = start(&r, Mode::CarlSolo, 0).unwrap();
    let mut bare = sim.combat.clone();
    bare = bare.with_crew(Vec::new());
    assert!(bare.seat_taken(Seat::RiderA) && bare.seat_taken(Seat::RiderB));
}

/// DOZ-02 (DZ-Q22): the tunnel lintel is drawn: `mouth` tiles along the 112 px underside, `fill` tiles up to the
/// lane ceiling, cropped to the rect (304 x 116 px over x 1040).
#[test]
fn the_tunnel_lintel_is_drawn_from_its_rect() {
    let floor = 100;
    let tiles = lintel_sprites(floor, 580, IRect::new(1040, 352, 304, 116));
    assert_eq!(tiles.len(), 10 * 4);
    assert!(
        tiles
            .iter()
            .all(|t| t.sheet == Sheet::Lintel && t.z == Z_BACK)
    );
    for t in &tiles {
        assert_eq!(t.frame, u8::from(t.bottom > floor + 112), "{t:?}");
        assert!(
            t.left >= 1040 && t.left + t.visible().0 as i32 <= 1344,
            "{t:?}"
        );
        assert!(
            t.bottom >= floor + 112 && t.bottom + t.visible().1 as i32 <= floor + 228,
            "{t:?}"
        );
    }
    // The tiles cover the rect exactly: 304 x 116 px.
    let area: u32 = tiles.iter().map(|t| t.visible().0 * t.visible().1).sum();
    assert_eq!(area, 304 * 116);
    // The mouth's lowest tile row sits on the 112 px clear height.
    assert_eq!(tiles.iter().map(|t| t.bottom).min(), Some(floor + 112));
}

/// DOZ-02 (DZ2-Q2): a fall that will glance off the rear hull ends on the rear deck and slides to where the strike
/// frame begins, so the beam never drops in front of the hull and pops up onto it.
#[test]
fn a_glancing_fall_ends_on_the_deck() {
    use crate::dozer_fixtures::{Stage, chase_at};
    use crate::env_prop::Stimulus;
    use crate::f01_late::Loadout;
    let mut st = Stage::new(
        &root(),
        Mode::TwoPlayer,
        Loadout::Story.flags(),
        chase_at(800.0),
        1100.0,
        1150.0,
    )
    .unwrap();
    let tick = st.tick();
    st.sim.combat.env_stimulate(
        FRAME_PROP,
        Stimulus::Blast {
            blast_id: 9002,
            owner: Some(crate::shore::PLAYER_ONE),
            depth: 0,
        },
        tick,
    );
    let mut falling = Vec::new();
    let mut strike = None;
    for _ in 0..120 {
        let tick = st.tick();
        let world = &st.sim.scene.world;
        let art = draw(&st.sim.combat, [world.hero.x, world.donut_body.x], tick).unwrap();
        let d = dozer_of(&st.sim.combat).unwrap();
        let deck = d.push_box().top().round() as i32 - GLANCE_DECK_DROP_PX;
        if let Some(f) = art
            .sprites
            .iter()
            .find(|s| s.sheet == Sheet::StopFrame && s.frame == 4)
        {
            falling.push((*f, deck, d.push_box().left.round() as i32 + GLANCE_INSET_PX));
        }
        if strike.is_none()
            && let Some(g) = art
                .sprites
                .iter()
                .find(|s| s.sheet == Sheet::StopGlance && s.frame == 0)
        {
            strike = Some((*g, deck));
        }
        st.step(RawInput::NONE);
    }
    assert!(!falling.is_empty() && strike.is_some());
    // Every falling frame stays on or above the rear deck, and the last one is on it, within the strike's reach.
    for (f, deck, _) in &falling {
        assert!(f.bottom >= *deck, "{f:?} under the deck {deck}");
    }
    let (last, deck, left) = *falling.last().unwrap();
    assert_eq!(last.bottom, deck);
    let (g, _) = strike.unwrap();
    // The strike frame's beam spans pivot x -40..+40: its left end is the clamp's target (within the hull's drift
    // of the last ticks).
    assert!(
        (g.left + 44 - 40 - left).abs() <= 8,
        "{} vs {left}",
        g.left + 4
    );
}

/// ACT1-ART (AD10): the shore's wedged machine is the rig at the chase's wedge pose, re-based into
/// the shore's columns, and it fits the shore decor rect {56, 440, 192, 140}: nothing left of
/// x 56, nothing past the arch's right edge (local 248), nothing above the rect's top.
#[test]
fn the_shore_wedge_fits_the_decor_rect() {
    let floor = 580.0;
    let sprites = wedged_rig(floor);
    assert!(sprites.len() >= Part::ALL.len() - 6, "the rig is drawn");
    let (mut x0, mut x1, mut top) = (i32::MAX, i32::MIN, i32::MIN);
    for s in &sprites {
        let (vw, vh) = s.visible();
        let left = s.left + i32::from(s.crop_left);
        x0 = x0.min(left);
        x1 = x1.max(left + vw as i32);
        top = top.max(s.bottom + vh as i32);
    }
    assert!(x0 >= 56, "no sprite left of the decor rect: {x0}");
    assert!(x1 <= 56 + 192, "no sprite past the arch's right edge: {x1}");
    assert!(
        top - 580 <= 140,
        "no sprite above the decor rect: {}",
        top - 580
    );
}
