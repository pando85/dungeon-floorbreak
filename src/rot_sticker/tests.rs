use super::*;
use crate::blast::BlastCharge;
use serde_json::Value;

fn tuning() -> RotTuning {
    RotTuning::pinned().unwrap()
}

fn carl_at(x: f32) -> HeroView {
    HeroView {
        id: HeroId::Carl,
        present: true,
        x,
        foot_y: 0.0,
        grounded: true,
        hurt: Rect {
            left: x - 12.0,
            bottom: 0.0,
            width: 24.0,
            height: 52.0,
        },
        vulnerable: true,
        bot: false,
    }
}

fn donut_at(x: f32) -> HeroView {
    HeroView {
        id: HeroId::Donut,
        present: true,
        x,
        foot_y: 0.0,
        grounded: true,
        hurt: Rect {
            left: x - 10.0,
            bottom: 0.0,
            width: 20.0,
            height: 24.0,
        },
        vulnerable: true,
        bot: false,
    }
}

/// A ceiling track on a surface `h` px above the floor, x `lo..=hi`.
fn ceiling(h: i32, lo: i32, hi: i32, start: i32) -> Track {
    Track::new(Orientation::Ceiling, h, lo, hi, start).unwrap()
}

fn ctx<'a>(tick: u64, heroes: &'a [HeroView]) -> StickerCtx<'a> {
    StickerCtx {
        tick,
        heroes,
        players: [Some(0), Some(1)],
        in_view: true,
        solids: &[],
        stimuli: &[],
        missiles: &[],
    }
}

fn stepped(s: &mut RotSticker, tick: u64, heroes: &[HeroView]) -> StickerStep {
    s.step(&tuning(), &ctx(tick, heroes))
}

/// A hero collider whose right edge is `gap` px left of the dome at x = 0.
fn carl_left_of_dome(gap: f32) -> HeroView {
    carl_at(-(gap + 12.0))
}

// ------------------------------------------------------------------ data

#[test]
fn the_file_is_pinned_and_round_trips() {
    assert_eq!(content_hash(TUNING_JSON), TUNING_HASH);
    let t = tuning();
    let original: Value = serde_json::from_str(TUNING_JSON).unwrap();
    assert_eq!(serde_json::to_value(&t).unwrap(), original);
    let drifted = TUNING_JSON.replace(
        "\"hazq28_warning_ticks\": 24",
        "\"hazq28_warning_ticks\": 20",
    );
    assert_ne!(drifted, TUNING_JSON);
    match RotTuning::from_json_pinned(&drifted, TUNING_HASH) {
        Err(RotTuningError::HashMismatch { expected, .. }) => assert_eq!(expected, TUNING_HASH),
        other => panic!("a drifted file must fail the pin: {other:?}"),
    }
    assert!(RotTuning::from_json(&drifted).is_ok());
    assert_eq!(active(), &t);
}

#[test]
fn the_adopted_values_are_the_sheet_values() {
    let t = tuning();
    assert_eq!(t.hazq28_warning_ticks, 24);
    assert_eq!(t.hazq27_trigger_px, 32);
    assert_eq!(t.hazq27_sense_box, SenseBox::Collider);
    assert_eq!(
        (t.hazq25_wake_px, t.hazq25_sleep_px, t.hazq25_sleep_ticks),
        (144, 176, 120)
    );
    assert_eq!((t.hazq26_speed_px_s, t.hazq26_switch_px), (60, 16));
    assert_eq!(t.scuttle_px_per_tick(), 1);
    assert_eq!(t.hazq24_dome_offset_px, 6);
    let b = t.hazq24_hurtbox;
    assert_eq!((b.x0, b.x1, b.y0, b.y1), (-10, 10, 0, 14));
    assert!(t.hazq29_hit_provokes && !t.hazq35_defuse && t.hazq38_ring);
    let p = t.blast_profile();
    assert_eq!(p.core_radius, 0, "no core band");
    assert_eq!(p.outer_radius, 48);
    assert!(
        p.outer_radius >= 48,
        "at least 48 while room C ships (S2 at 47.5 px)"
    );
    assert_eq!((p.damage, p.launch_x, p.launch_y), (1, 160, 180));
    assert_eq!(p.hero_damage, HeroDamagePolicy::AllHeroes);
    assert!(p.line_of_sight && !p.breaks_structures);
    assert_eq!((p.hero_knock_x, p.hero_knock_y), (120, 160));
}

fn field_of(text: &str) -> &'static str {
    match RotTuning::from_json(text) {
        Err(RotTuningError::Field { field, .. }) => field,
        other => panic!("expected a field error, got {other:?}"),
    }
}

#[test]
fn a_bad_value_names_its_field() {
    let cases: [(&str, &str, &str); 12] = [
        (
            "\"hazq28_warning_ticks\": 24",
            "\"hazq28_warning_ticks\": 17",
            "hazq28_warning_ticks",
        ),
        (
            "\"hazq28_warning_ticks\": 24",
            "\"hazq28_warning_ticks\": 25",
            "hazq28_warning_ticks",
        ),
        (
            "\"hazq30_outer_radius_px\": 48",
            "\"hazq30_outer_radius_px\": 39",
            "hazq30_outer_radius_px",
        ),
        (
            "\"hazq30_outer_radius_px\": 48",
            "\"hazq30_outer_radius_px\": 57",
            "hazq30_outer_radius_px",
        ),
        (
            "\"hazq30_core_radius_px\": 0",
            "\"hazq30_core_radius_px\": 49",
            "hazq30_core_radius_px",
        ),
        (
            "\"hazq26_speed_px_s\": 60",
            "\"hazq26_speed_px_s\": 90",
            "hazq26_speed_px_s",
        ),
        (
            "\"hazq26_speed_px_s\": 60",
            "\"hazq26_speed_px_s\": 0",
            "hazq26_speed_px_s",
        ),
        (
            "\"hazq25_sleep_px\": 176",
            "\"hazq25_sleep_px\": 100",
            "hazq25_sleep_px",
        ),
        (
            "\"hazq27_trigger_px\": 32",
            "\"hazq27_trigger_px\": 0",
            "hazq27_trigger_px",
        ),
        ("\"all_heroes\"", "\"owner_reduced\"", "hazq30_hero_damage"),
        ("\"all_heroes\"", "\"everyone\"", "hazq30_hero_damage"),
        (
            "\"hazq24_hurtbox\": { \"x0\": -10, \"x1\": 10,",
            "\"hazq24_hurtbox\": { \"x0\": 10, \"x1\": -10,",
            "hazq24_hurtbox",
        ),
    ];
    for (from, to, field) in cases {
        assert!(TUNING_JSON.contains(from), "{from}");
        assert_eq!(field_of(&TUNING_JSON.replace(from, to)), field, "{to}");
    }
    let mut v: Value = serde_json::from_str(TUNING_JSON).unwrap();
    v["hazq99_extra"] = serde_json::json!(1);
    assert!(matches!(
        RotTuning::from_json(&v.to_string()),
        Err(RotTuningError::Parse(_))
    ));
}

#[test]
fn every_question_has_a_knob_or_a_recorded_reason() {
    let slime: Value = serde_json::from_str(crate::slime::TUNING_JSON).unwrap();
    let rot: Value = serde_json::from_str(TUNING_JSON).unwrap();
    let keys: Vec<&String> = slime
        .as_object()
        .unwrap()
        .keys()
        .chain(rot.as_object().unwrap().keys())
        .collect();
    let exempt: Vec<u8> = crate::slime::NO_KNOB.iter().map(|(q, _)| *q).collect();
    for q in 1..=50u8 {
        let prefix = format!("hazq{q:02}_");
        let has_key = keys.iter().any(|k| k.starts_with(&prefix));
        if exempt.contains(&q) {
            assert!(!has_key, "HAZ-Q{q:02} is listed as knob-less but has a key");
        } else {
            assert!(has_key, "HAZ-Q{q:02} has no knob and is not in NO_KNOB");
        }
    }
    // Every reason is recorded in English, once.
    let mut seen = std::collections::BTreeSet::new();
    for (q, why) in crate::slime::NO_KNOB {
        assert!(seen.insert(q), "HAZ-Q{q:02} twice");
        assert!(why.is_ascii() && why.len() > 8, "{why}");
    }
    for key in rot.as_object().unwrap().keys() {
        let named = key.starts_with("hazq")
            && key[4..6].parse::<u8>().is_ok_and(|q| (1..=50).contains(&q))
            && key.as_bytes()[6] == b'_';
        assert!(named || key == "schema_version", "{key}");
    }
}

#[test]
fn the_source_is_distinct_from_every_other_hero_blow_source() {
    for other in [
        crate::slime::SLIME_SOURCE_ID,
        crate::slime::ACID_SOURCE_ID,
        crate::boss::BOSS_SOURCE_ID,
        crate::boss::RACK_SOURCE_ID,
        crate::cf02::arena::SHORE_BLOCK_SOURCE,
    ] {
        assert_ne!(ROT_SOURCE_ID, other);
    }
    assert_eq!(rot_source(2) >> 16, 2);
    assert_eq!(rot_source(2) & 0xffff, ROT_SOURCE_ID);
}

// -------------------------------------------------------------- geometry

#[test]
fn the_dome_is_six_px_off_the_surface_along_its_normal() {
    let t = tuning();
    let off = t.hazq24_dome_offset_px;
    assert_eq!(ceiling(68, 576, 664, 664).dome(664, off), (664.0, 62.0));
    let wl = Track::new(Orientation::WallLeft, 960, 20, 60, 32).unwrap();
    assert_eq!(
        wl.dome(32, off),
        (966.0, 32.0),
        "a wall sticker's dome is in front of the face"
    );
    let wr = Track::new(Orientation::WallRight, 100, 20, 60, 32).unwrap();
    assert_eq!(wr.dome(32, off), (94.0, 32.0));
}

#[test]
fn the_shell_is_the_drawn_box_rotated_with_the_orientation() {
    let t = tuning();
    let b = &t.hazq24_hurtbox;
    // A ceiling shell hangs 14 px below the underside, 20 px wide.
    let c = ceiling(68, 576, 664, 600).shell(600, b);
    assert_eq!(
        (c.left, c.right(), c.bottom, c.top()),
        (590.0, 610.0, 54.0, 68.0)
    );
    // A wall-left shell sticks 14 px out of the face (+x), 20 px tall.
    let wl = Track::new(Orientation::WallLeft, 960, 0, 100, 32)
        .unwrap()
        .shell(32, b);
    assert_eq!(
        (wl.left, wl.right(), wl.bottom, wl.top()),
        (960.0, 974.0, 22.0, 42.0)
    );
    let wr = Track::new(Orientation::WallRight, 960, 0, 100, 32)
        .unwrap()
        .shell(32, b);
    assert_eq!(
        (wr.left, wr.right(), wr.bottom, wr.top()),
        (946.0, 960.0, 22.0, 42.0)
    );
    // The dome centre is inside the 32 x 32 frame.
    let tr = ceiling(68, 576, 664, 600);
    let f = tr.frame(600, 6);
    assert_eq!((f.width, f.height), (32.0, 32.0));
    assert_eq!(f.center(), tr.dome(600, 6));
}

#[test]
fn a_track_is_validated() {
    assert_eq!(
        Track::new(Orientation::Ceiling, 68, 100, 50, 60),
        Err(TrackError::Reversed)
    );
    assert_eq!(
        Track::new(Orientation::Ceiling, 68, 0, 161, 0),
        Err(TrackError::TooLong { len: 161 })
    );
    assert!(Track::new(Orientation::Ceiling, 68, 0, 160, 0).is_ok());
    assert_eq!(
        Track::new(Orientation::Ceiling, 68, 0, 100, 101),
        Err(TrackError::StartOffTrack)
    );
    let fixed = Track::new(Orientation::WallLeft, 960, 40, 40, 40).unwrap();
    assert!(fixed.is_fixed());
    assert_eq!(fixed.length(), 0);
    // From two end points and a start point; in either order.
    let a = Track::from_points(Orientation::Ceiling, (824, 68), (760, 68), (760, 68)).unwrap();
    assert_eq!((a.lo, a.hi, a.start, a.fixed), (760, 824, 760, 68));
    assert_eq!(
        Track::from_points(Orientation::Ceiling, (760, 68), (824, 70), (760, 68)),
        Err(TrackError::NotAxisAligned)
    );
    assert_eq!(
        Track::from_points(Orientation::WallLeft, (960, 0), (961, 40), (960, 0)),
        Err(TrackError::NotAxisAligned)
    );
    for o in Orientation::ALL {
        assert_eq!(Orientation::parse(o.name()), Some(o));
    }
    assert_eq!(Orientation::parse("floor"), None);
}

// ------------------------------------------------------------ line of sight

#[test]
fn a_segment_is_blocked_by_an_interior_never_by_an_edge() {
    let slab = Rect {
        left: 0.0,
        bottom: 68.0,
        width: 100.0,
        height: 12.0,
    };
    // Through the slab: blocked.
    assert!(segment_blocked((50.0, 60.0), (50.0, 90.0), &[slab]));
    // Ending on its underside or running along it: not blocked.
    assert!(!segment_blocked((50.0, 60.0), (50.0, 68.0), &[slab]));
    assert!(!segment_blocked((-10.0, 68.0), (110.0, 68.0), &[slab]));
    assert!(!segment_blocked((-10.0, 80.0), (110.0, 80.0), &[slab]));
    // Beside it.
    assert!(!segment_blocked((-20.0, 60.0), (-20.0, 90.0), &[slab]));
    // A point strictly inside blocks; the same point on the edge does not.
    assert!(segment_blocked((50.0, 70.0), (50.0, 70.0), &[slab]));
    assert!(!segment_blocked((50.0, 68.0), (50.0, 68.0), &[slab]));
    // A diagonal that clips a corner region.
    assert!(segment_blocked((-10.0, 60.0), (30.0, 100.0), &[slab]));
    assert!(!segment_blocked((-10.0, 60.0), (10.0, 60.0), &[]));
}

#[test]
fn circle_reaches_uses_the_nearest_point_and_the_shield() {
    let body = Rect {
        left: 30.0,
        bottom: 0.0,
        width: 24.0,
        height: 52.0,
    };
    // Dome at (0, 62): nearest point (30, 52): sqrt(900 + 100).
    assert!(circle_reaches((0.0, 62.0), 32.0, body, true, &[]));
    assert!(!circle_reaches((0.0, 62.0), 31.0, body, true, &[]));
    let wall = Rect {
        left: 10.0,
        bottom: 0.0,
        width: 10.0,
        height: 200.0,
    };
    assert!(
        !circle_reaches((0.0, 62.0), 32.0, body, true, &[wall]),
        "a wall between shields it"
    );
    assert!(circle_reaches((0.0, 62.0), 32.0, body, false, &[wall]));
}

// ----------------------------------------------------------- the machine

#[test]
fn a_provoke_at_t_detonates_at_t_plus_24_exactly_once() {
    let t = tuning();
    let mut s = RotSticker::new(ceiling(68, -40, 40, 0));
    let heroes = [carl_left_of_dome(20.0)];
    for tick in 1..5 {
        // Out of range: nothing.
        let far = [carl_left_of_dome(200.0)];
        assert!(stepped(&mut s, tick, &far).provoked.is_none());
    }
    let out = stepped(&mut s, 5, &heroes);
    let p = out.provoked.expect("provoked on this tick");
    assert_eq!(p.cause, RotCause::Proximity);
    assert_eq!(p.player, Some(0));
    assert_eq!(s.state(), StickerState::Warning);
    assert_eq!(s.detonate_tick(), 5 + 24, "detonate_tick == t + 24");
    assert_eq!(s.owner(), Some(0));
    assert_eq!(s.remaining(5), Some(24));
    // Warning is committed and stationary: nothing changes it, a hero walking
    // away included, until the detonation tick.
    let mut detonations = Vec::new();
    for tick in 6..=60u64 {
        let out = stepped(&mut s, tick, &[carl_left_of_dome(300.0)]);
        assert!(out.provoked.is_none(), "no second provoke");
        if let Some(d) = out.detonated {
            detonations.push(d);
        }
    }
    assert_eq!(detonations.len(), 1, "one detonation per sticker");
    assert_eq!(detonations[0].tick, 29);
    assert_eq!(detonations[0].owner, Some(0));
    assert_eq!(detonations[0].dome, (0.0, 62.0));
    assert!(s.is_spent());
    assert!(s.hurtbox(&t).is_none(), "the body is gone");
    // Spent: no hurtbox, no trigger, no motion, never again.
    for tick in 61..120 {
        let out = stepped(&mut s, tick, &[carl_left_of_dome(10.0)]);
        assert_eq!(out, StickerStep::default());
    }
    assert!(s.chain(130, Some(0)).is_none());
    assert!(s.is_spent());
}

#[test]
fn hits_and_proximity_during_warning_change_nothing() {
    let mut s = RotSticker::new(ceiling(68, -40, 40, 0));
    stepped(&mut s, 10, &[carl_left_of_dome(20.0)]);
    assert_eq!(s.detonate_tick(), 34);
    let hit = [Stimulus::MeleeHit {
        token: 1,
        player: 1,
        weight: 2,
    }];
    for tick in 11..34 {
        let crowd = [carl_left_of_dome(5.0), donut_at(30.0)];
        let mut c = ctx(tick, &crowd);
        c.stimuli = &hit;
        let out = s.step(&tuning(), &c);
        assert!(out.provoked.is_none() && out.detonated.is_none());
        assert_eq!(s.detonate_tick(), 34);
        assert_eq!(s.owner(), Some(0), "the first initiator keeps the credit");
    }
    assert!(stepped(&mut s, 34, &[]).detonated.is_some());
}

#[test]
fn the_trigger_circle_against_the_collider_the_section_10_tables() {
    // H 68 slab underside: dome centre at 62; collider top 52: trigger 30.4.
    let provokes = |gap: f32| {
        let mut s = RotSticker::new(ceiling(68, 0, 0, 0));
        stepped(&mut s, 1, &[carl_left_of_dome(gap)])
            .provoked
            .is_some()
    };
    assert!(provokes(30.39), "sqrt(32^2 - 10^2) = 30.397");
    assert!(!provokes(30.41));
    // H 80: 23.2; H 84: 18.6.
    let at = |h: i32, gap: f32| {
        let mut s = RotSticker::new(ceiling(h, 0, 0, 0));
        stepped(&mut s, 1, &[carl_left_of_dome(gap)])
            .provoked
            .is_some()
    };
    assert!(at(80, 23.22) && !at(80, 23.26));
    assert!(at(84, 18.63) && !at(84, 18.67));
    // H 96: never, grounded (the dome is 90 px up, 38 above the collider).
    for gap in [0.0, 5.0, 20.0] {
        assert!(!at(96, gap), "no grounded trigger at H 96, gap {gap}");
    }
    // A jump under it triggers: feet 20 px up put the collider top at 72.
    let mut s = RotSticker::new(ceiling(96, 0, 0, 0));
    let mut jumper = carl_left_of_dome(20.0);
    jumper.foot_y = 20.0;
    jumper.grounded = false;
    assert!(stepped(&mut s, 1, &[jumper]).provoked.is_some());
    // The collider is 24 x 52 for both heroes: Donut sets it off the same way
    // (her hurtbox is smaller, the collider is not).
    let mut s = RotSticker::new(ceiling(68, 0, 0, 0));
    let d = donut_at(-(30.39 + 12.0));
    assert!(stepped(&mut s, 1, &[d]).provoked.is_some(), "F16");
}

#[test]
fn a_wall_sticker_triggers_within_38_px_of_the_face() {
    // Wall on the sticker's left, face at x 0, dome at x 6, height 32.
    let track = Track::new(Orientation::WallLeft, 0, 32, 32, 32).unwrap();
    let provokes = |left_edge: f32| {
        let mut s = RotSticker::new(track);
        let hero = carl_at(left_edge + 12.0);
        stepped(&mut s, 1, &[hero]).provoked.is_some()
    };
    assert!(provokes(37.99), "32 + 6 from the face");
    assert!(!provokes(38.01));
}

#[test]
fn a_hero_on_top_of_the_slab_is_shielded() {
    // The slab 560..680 (underside 68, top 80), the sticker under it at 620.
    let slab = Rect {
        left: 560.0,
        bottom: 68.0,
        width: 120.0,
        height: 12.0,
    };
    let mut s = RotSticker::new(ceiling(68, 576, 664, 620));
    let mut on_top = carl_at(620.0);
    on_top.foot_y = 80.0;
    on_top.hurt.bottom = 80.0;
    on_top.grounded = true;
    for tick in 1..240 {
        let mut c = ctx(tick, std::slice::from_ref(&on_top));
        let solids = [slab];
        c.solids = &solids;
        assert!(
            s.step(&tuning(), &c).provoked.is_none(),
            "line of sight is blocked, tick {tick}"
        );
    }
    // The same hero standing under it is not shielded.
    let mut s = RotSticker::new(ceiling(68, 576, 664, 620));
    let under = carl_at(620.0);
    let mut c = ctx(1, std::slice::from_ref(&under));
    let solids = [slab];
    c.solids = &solids;
    assert!(s.step(&tuning(), &c).provoked.is_some());
}

#[test]
fn it_never_triggers_off_screen_or_for_an_absent_hero() {
    let mut s = RotSticker::new(ceiling(68, 0, 0, 0));
    let near = [carl_left_of_dome(10.0)];
    let mut c = ctx(1, &near);
    c.in_view = false;
    assert!(s.step(&tuning(), &c).provoked.is_none());
    assert_eq!(s.state(), StickerState::Latched, "and it does not wake");
    let gone = [HeroView {
        present: false,
        ..carl_left_of_dome(10.0)
    }];
    assert!(stepped(&mut s, 2, &gone).provoked.is_none());
    // With no hero at all nothing ever happens (an enemy cannot be passed).
    for tick in 3..300 {
        assert_eq!(stepped(&mut s, tick, &[]), StickerStep::default());
    }
    assert_eq!(s.state(), StickerState::Latched);
}

#[test]
fn hit_stimuli_provoke_with_the_right_cause_and_credit() {
    let cases: [(Stimulus, Option<RotCause>, Option<u32>); 8] = [
        (
            Stimulus::MeleeHit {
                token: 1,
                player: 1,
                weight: 1,
            },
            Some(RotCause::Hit),
            Some(1),
        ),
        (
            Stimulus::Projectile {
                projectile_id: 4,
                owner: Some(1),
                weight: 1,
            },
            Some(RotCause::Projectile),
            Some(1),
        ),
        (
            Stimulus::LaunchedBody {
                launch_id: 2,
                impulse_px_s: 10,
                initiator: Some(0),
            },
            Some(RotCause::Flier),
            Some(0),
        ),
        (
            Stimulus::LaunchedBody {
                launch_id: 2,
                impulse_px_s: 300,
                initiator: None,
            },
            Some(RotCause::Flier),
            None,
        ),
        // Ignored: a blast, a prop contact, a boss shove, a zero-weight hit.
        (
            Stimulus::Blast {
                blast_id: 1,
                owner: Some(0),
            },
            None,
            None,
        ),
        (
            Stimulus::PropContact {
                source: 1,
                initiator: Some(0),
                depth: 1,
            },
            None,
            None,
        ),
        (
            Stimulus::Shove {
                shove_id: 1,
                impulse_px_s: 500,
                source: 9,
                credit: Some(0),
            },
            None,
            None,
        ),
        (
            Stimulus::MeleeHit {
                token: 1,
                player: 0,
                weight: 0,
            },
            None,
            None,
        ),
    ];
    for (stimulus, cause, player) in cases {
        let mut s = RotSticker::new(ceiling(68, 0, 0, 0));
        let stimuli = [stimulus];
        let mut c = ctx(1, &[]);
        c.stimuli = &stimuli;
        let out = s.step(&tuning(), &c);
        assert_eq!(out.provoked.map(|p| p.cause), cause, "{stimulus:?}");
        assert_eq!(out.provoked.and_then(|p| p.player), player, "{stimulus:?}");
        if cause.is_some() {
            assert_eq!(s.detonate_tick(), 25);
            assert_eq!(s.owner(), player);
            assert_eq!(s.flash_left(), 2, "the 2-tick tint, never a hurt frame");
        } else {
            assert_eq!(s.state(), StickerState::Latched, "{stimulus:?}");
        }
    }
}

#[test]
fn the_hit_switch_turns_every_hit_provoke_off() {
    let mut t = tuning();
    t.hazq29_hit_provokes = false;
    let mut s = RotSticker::new(ceiling(68, 0, 0, 0));
    let stimuli = [Stimulus::MeleeHit {
        token: 1,
        player: 0,
        weight: 3,
    }];
    let bolts = [Missile {
        id: 3,
        rect: Rect {
            left: -4.0,
            bottom: 60.0,
            width: 8.0,
            height: 4.0,
        },
        owner: Some(1),
    }];
    let mut c = ctx(1, &[]);
    c.stimuli = &stimuli;
    c.missiles = &bolts;
    let out = s.step(&t, &c);
    assert!(out.provoked.is_none() && out.consumed.is_empty());
}

#[test]
fn rule_b_a_bolt_within_the_trigger_circle_provokes_and_is_consumed() {
    let bolt = |id: u32, left: f32, bottom: f32| Missile {
        id,
        rect: Rect {
            left,
            bottom,
            width: 12.0,
            height: 6.0,
        },
        owner: Some(1),
    };
    let t = tuning();
    // R1 at H 68 (dome 62): a grounded bolt (heights 16..22) is 40 px below the
    // dome and never provokes it; R2 at H 84 (dome 78) is 56 px.
    for h in [68, 84] {
        let mut s = RotSticker::new(ceiling(h, 0, 0, 0));
        let bolts = [bolt(1, -6.0, 16.0)];
        let mut c = ctx(1, &[]);
        c.missiles = &bolts;
        let out = s.step(&t, &c);
        assert!(out.provoked.is_none() && out.consumed.is_empty(), "H {h}");
    }
    // A near miss at dome height, without touching the shell, provokes it.
    let mut s = RotSticker::new(ceiling(84, 0, 0, 0));
    let bolts = [bolt(7, 20.0, 74.0)];
    let mut c = ctx(9, &[]);
    c.missiles = &bolts;
    let out = s.step(&t, &c);
    assert_eq!(out.consumed, [7]);
    let p = out.provoked.unwrap();
    assert_eq!((p.cause, p.player), (RotCause::Projectile, Some(1)));
    assert_eq!(s.detonate_tick(), 33);
    // A bolt just outside the 32 px circle does not.
    let mut s = RotSticker::new(ceiling(84, 0, 0, 0));
    let bolts = [bolt(8, 32.01, 72.0)];
    let mut c = ctx(9, &[]);
    c.missiles = &bolts;
    assert!(s.step(&t, &c).provoked.is_none());
    // With the line of sight blocked by a solid it does not provoke.
    let soffit_side = Rect {
        left: 10.0,
        bottom: 60.0,
        width: 6.0,
        height: 40.0,
    };
    let mut s = RotSticker::new(ceiling(84, 0, 0, 0));
    let bolts = [bolt(8, 20.0, 74.0)];
    let solids = [soffit_side];
    let mut c = ctx(9, &[]);
    c.missiles = &bolts;
    c.solids = &solids;
    let out = s.step(&t, &c);
    assert!(out.provoked.is_none() && out.consumed.is_empty());
    // Only the first bolt is consumed.
    let mut s = RotSticker::new(ceiling(84, 0, 0, 0));
    let bolts = [bolt(1, 20.0, 74.0), bolt(2, 22.0, 74.0)];
    let mut c = ctx(9, &[]);
    c.missiles = &bolts;
    assert_eq!(s.step(&t, &c).consumed, [1]);
}

// ------------------------------------------------- wake, scuttle and sleep

#[test]
fn it_wakes_only_in_view_within_144_px_of_the_dome() {
    let mut s = RotSticker::new(ceiling(68, 0, 100, 100));
    // The hurtbox's nearest point to the dome (100, 62).
    let hero = |gap: f32| {
        // Hero to the left of the dome: its hurtbox right edge `gap` px away.
        let mut h = carl_at(100.0 - gap - 12.0);
        h.hurt.bottom = 10.0;
        h.foot_y = 10.0;
        h
    };
    let far = [hero(150.0)];
    for tick in 1..10 {
        stepped(&mut s, tick, &far);
    }
    assert_eq!(s.state(), StickerState::Latched, "beyond 144 px");
    let near = [hero(100.0)];
    let mut off = ctx(10, &near);
    off.in_view = false;
    s.step(&tuning(), &off);
    assert_eq!(s.state(), StickerState::Latched, "never wakes off-screen");
    stepped(&mut s, 11, &near);
    assert_eq!(
        s.state(),
        StickerState::Scuttling,
        "wakes on the first tick in view"
    );
    assert_eq!(s.target(), Some(HeroId::Carl));
}

#[test]
fn it_scuttles_one_px_per_tick_clamped_and_stops_on_the_projection() {
    let mut s = RotSticker::new(ceiling(68, 100, 200, 200));
    // Carl's centre is at x 130; the sticker starts at 200.
    let carl = [carl_at(130.0)];
    stepped(&mut s, 1, &carl);
    assert_eq!(s.state(), StickerState::Scuttling);
    let mut xs = vec![s.s()];
    for tick in 2..90 {
        stepped(&mut s, tick, &carl);
        xs.push(s.s());
        if s.state() == StickerState::Warning {
            break;
        }
    }
    for pair in xs.windows(2) {
        assert!(
            pair[1] - pair[0] <= 0 && pair[0] - pair[1] <= 1,
            "1 px per tick: {xs:?}"
        );
    }
    assert!(s.s() < 200, "it moved toward the hero");
}

#[test]
fn the_scuttle_never_leaves_the_track_and_stops_on_the_projection() {
    // The hero far off the track end: clamped to the end, then holds.
    let mut s = RotSticker::new(ceiling(112, 100, 160, 100));
    let mut hero = carl_at(230.0);
    hero.foot_y = 0.0;
    let heroes = [hero];
    for tick in 1..300 {
        let mut c = ctx(tick, &heroes);
        // Keep proximity away: the hero stands 140 px along the floor, 112 up.
        c.in_view = true;
        s.step(&tuning(), &c);
        assert!((100..=160).contains(&s.s()), "tick {tick}: {}", s.s());
    }
    assert_eq!(s.s(), 160, "clamped to the near end");
    // A length-0 track never moves.
    let mut fixed = RotSticker::new(ceiling(112, 120, 120, 120));
    for tick in 1..100 {
        stepped(&mut fixed, tick, &[carl_at(150.0)]);
        assert_eq!(fixed.s(), 120);
    }
    assert_eq!(fixed.state(), StickerState::Scuttling, "awake but fixed");
}

#[test]
fn it_never_moves_in_warning_or_asleep() {
    let mut s = RotSticker::new(ceiling(68, 0, 100, 50));
    // Provoke by a hit: warning at 50, stays there while a hero stands at 0.
    let stimuli = [Stimulus::MeleeHit {
        token: 1,
        player: 0,
        weight: 1,
    }];
    let mut c = ctx(1, &[]);
    c.stimuli = &stimuli;
    s.step(&tuning(), &c);
    for tick in 2..20 {
        stepped(&mut s, tick, &[carl_at(0.0)]);
        assert_eq!(s.s(), 50, "stationary in warning");
    }
    // Asleep: a latched sticker stays where it stopped.
    let mut sleepy = RotSticker::new(ceiling(68, 0, 100, 50));
    for tick in 1..200 {
        stepped(&mut sleepy, tick, &[carl_at(900.0)]);
    }
    assert_eq!((sleepy.state(), sleepy.s()), (StickerState::Latched, 50));
}

#[test]
fn it_sleeps_after_120_ticks_with_no_hero_within_176_px_and_stays_put() {
    let mut s = RotSticker::new(ceiling(112, 100, 200, 200));
    let near = [carl_at(150.0)];
    stepped(&mut s, 1, &near);
    for tick in 2..40 {
        stepped(&mut s, tick, &near);
    }
    assert_eq!(s.state(), StickerState::Scuttling);
    let stopped_at = s.s();
    assert!(stopped_at < 200);
    // The hero leaves: 119 ticks keep it awake, the 120th puts it to sleep.
    let gone = [carl_at(1000.0)];
    for i in 0..119u64 {
        stepped(&mut s, 100 + i, &gone);
        assert_eq!(s.state(), StickerState::Scuttling, "tick {i}");
    }
    stepped(&mut s, 300, &gone);
    assert_eq!(s.state(), StickerState::Latched);
    assert_eq!(s.s(), s.s());
    let rest = s.s();
    for tick in 301..400 {
        stepped(&mut s, tick, &gone);
        assert_eq!(s.s(), rest, "it stays where it stopped");
    }
    // A hero coming back inside 176 px resets the counter.
    let mut w = RotSticker::new(ceiling(112, 100, 200, 200));
    stepped(&mut w, 1, &near);
    for i in 0..100u64 {
        stepped(&mut w, 10 + i, &gone);
    }
    let back = [carl_at(260.0)];
    stepped(&mut w, 200, &back);
    for i in 0..100u64 {
        stepped(&mut w, 210 + i, &gone);
    }
    assert_eq!(w.state(), StickerState::Scuttling, "the counter restarted");
}

#[test]
fn it_picks_the_nearest_hero_ties_to_carl_with_16_px_of_hysteresis() {
    let mut s = RotSticker::new(ceiling(112, 0, 160, 80));
    // Both heroes equally near (their centres mirror about the dome): Carl wins.
    let dome_x = 80.0;
    let heroes = [carl_at(dome_x - 60.0), donut_at(dome_x + 60.0)];
    stepped(&mut s, 1, &heroes);
    assert_eq!(s.target(), Some(HeroId::Carl));
    // Donut a little nearer, but not 16 px nearer: it keeps Carl.
    let heroes = [carl_at(dome_x - 60.0), donut_at(dome_x + 50.0)];
    let before = s.target();
    stepped(&mut s, 2, &heroes);
    assert_eq!(before, s.target());
    // The centres: Carl's is 26 up, Donut's 12 up; compare by distance.
    let mut s2 = RotSticker::new(ceiling(112, 0, 160, 80));
    let far_carl = [carl_at(dome_x - 100.0), donut_at(dome_x + 20.0)];
    stepped(&mut s2, 1, &far_carl);
    assert_eq!(s2.target(), Some(HeroId::Donut), "the nearest at the wake");
    // Now Carl is much nearer: it re-picks.
    let near_carl = [carl_at(dome_x - 5.0), donut_at(dome_x + 120.0)];
    stepped(&mut s2, 2, &near_carl);
    assert_eq!(s2.target(), Some(HeroId::Carl));
}

// ------------------------------------------------------ chain and credit

#[test]
fn a_dynamite_chain_sets_now_plus_8_and_pulls_a_later_one_forward() {
    // A latched sticker: lit at now + 8, owner = the dynamite's owner.
    let mut s = RotSticker::new(ceiling(84, 0, 60, 0));
    let p = s.chain(100, Some(0)).expect("lit by the chain");
    assert_eq!((p.cause, p.player), (RotCause::Chain, Some(0)));
    assert_eq!(s.detonate_tick(), 108);
    assert_eq!(s.owner(), Some(0));
    assert_eq!(s.state(), StickerState::Warning);
    // Detonation on tick 108 exactly.
    assert!(stepped(&mut s, 107, &[]).detonated.is_none());
    assert_eq!(
        stepped(&mut s, 108, &[]).detonated.map(|d| d.tick),
        Some(108)
    );
    // One due within 8 ticks is left alone.
    let mut s = RotSticker::new(ceiling(84, 0, 60, 0));
    s.chain(100, Some(0));
    assert!(s.chain(103, Some(1)).is_none());
    assert_eq!(s.detonate_tick(), 108, "due sooner than 111: unchanged");
    // One due later is pulled forward to now + 8.
    let mut s = RotSticker::new(ceiling(84, 0, 60, 0));
    stepped(&mut s, 100, &[carl_left_of_dome(10.0)]);
    assert_eq!(s.detonate_tick(), 124);
    assert!(s.chain(110, Some(1)).is_none(), "already provoked");
    assert_eq!(s.detonate_tick(), 118);
    // Scuttling is lit too.
    let mut s = RotSticker::new(ceiling(112, 0, 60, 60));
    stepped(&mut s, 1, &[carl_at(30.0)]);
    assert_eq!(s.state(), StickerState::Scuttling);
    assert!(s.chain(5, None).is_some());
    assert_eq!(s.detonate_tick(), 13);
}

#[test]
fn the_first_initiator_wins_a_chain_never_overwrites_it() {
    // Provoked by Donut's collider (player 1), then Carl's dynamite pulls it
    // forward: the owner stays Donut's player.
    let mut s = RotSticker::new(ceiling(84, 0, 60, 0));
    let mut c = ctx(10, &[]);
    let donut = [donut_at(-30.0)];
    c.heroes = &donut;
    let out = s.step(&tuning(), &c);
    assert_eq!(out.provoked.unwrap().player, Some(1));
    s.chain(20, Some(0));
    assert_eq!(s.owner(), Some(1));
    assert_eq!(s.detonate_tick(), 28, "pulled forward");
    // No initiator (a launch nobody credited): the dynamite's owner is written.
    let mut s = RotSticker::new(ceiling(84, 0, 60, 0));
    let stimuli = [Stimulus::LaunchedBody {
        launch_id: 1,
        impulse_px_s: 200,
        initiator: None,
    }];
    let mut c = ctx(10, &[]);
    c.stimuli = &stimuli;
    s.step(&tuning(), &c);
    assert_eq!(s.owner(), None);
    s.chain(12, Some(0));
    assert_eq!(
        s.owner(),
        Some(0),
        "an owner is written only on a charge without one"
    );
}

#[test]
fn the_warning_frame_follows_the_ticks_left() {
    // Provoke at t: frame 7 on t..t+5, 8 on t+6..t+22, 9 on t+23, 10 on t+24.
    for k in 0..=24u64 {
        let remaining = 24 - k;
        let want = match k {
            0..=5 => 7,
            6..=22 => 8,
            23 => 9,
            _ => 10,
        };
        assert_eq!(warning_frame(remaining), want, "t+{k}");
    }
    // A chain at c: frame 8 on c..c+6, 9 on c+7, 10 on c+8.
    for k in 0..=8u64 {
        let want = match k {
            0..=6 => 8,
            7 => 9,
            _ => 10,
        };
        assert_eq!(warning_frame(8 - k), want, "c+{k}");
    }
    // 17 ticks of the fully lit frame instead of the authored 24.
    assert_eq!((6..=22).count(), 17);
    // Through the machine.
    let mut s = RotSticker::new(ceiling(84, 0, 60, 0));
    stepped(&mut s, 50, &[carl_left_of_dome(10.0)]);
    assert_eq!(s.frame_index(50), Some(7));
    assert_eq!(s.frame_index(56), Some(8));
    assert_eq!(s.frame_index(73), Some(9));
    assert_eq!(s.frame_index(74), Some(10));
    let latched = RotSticker::new(ceiling(84, 0, 60, 0));
    assert_eq!(latched.frame_index(5), None);
}

// ------------------------------------------------ bait, retreat, run through

/// A hero standing with its collider's near edge `edge` px from the dome at x 0
/// (the sticker at H 68), who starts moving away on provoke tick + `reaction`
/// at `speed` px per tick and moves on ticks `t + reaction ..= t + 24` (heroes
/// step before the charges step). Whether the blast on tick `t + 24` hits.
fn retreat_is_hit(speed: f32, reaction: u64, hurt_half: f32, hurt_h: f32) -> bool {
    let t = tuning();
    let mut s = RotSticker::new(ceiling(68, 0, 0, 0));
    let mut x = -(30.39 + 12.0);
    let provoke = 100u64;
    let heroes = [carl_at(x)];
    assert!(stepped(&mut s, provoke, &heroes).provoked.is_some());
    for tick in provoke + 1..=provoke + 24 {
        if tick >= provoke + reaction {
            x -= speed;
        }
    }
    let mut hero = carl_at(x);
    hero.hurt = Rect {
        left: x - hurt_half,
        bottom: 0.0,
        width: hurt_half * 2.0,
        height: hurt_h,
    };
    let mut blast = RotBlast::new(s.dome(&t), t.blast_profile(), s.owner(), 0);
    let targets = [BlastTarget {
        id: 0,
        kind: TargetKind::Hero(HeroId::Carl),
        body: hero.hurt,
        x,
    }];
    !blast.evaluate(&targets, &[]).is_empty()
}

#[test]
fn bait_and_back_off_the_latest_safe_start_is_19() {
    // Carl at 185 px/s: the reaction boundary pinned (the sheet predicts 19).
    let speed = 185.0 / 60.0;
    assert!(!retreat_is_hit(speed, 19, 12.0, 52.0), "L = 19 escapes");
    assert!(retreat_is_hit(speed, 20, 12.0, 52.0), "L = 20 is hit");
    // 18 ticks (300 ms) to react: the margin the 24-tick warning buys.
    assert_eq!(24 - 19 + 1, 6, "six running ticks");
}

#[test]
fn running_straight_through_hits_carl_and_barely_clears_donut() {
    let t = tuning();
    let blast_dome = (0.0_f32, 62.0_f32);
    // Carl at 185 px/s from the provoke edge (collider edge at 30.4): hit.
    let carl_x = -(30.39 + 12.0) + 24.0 * (185.0 / 60.0);
    let carl = carl_at(carl_x);
    assert!(circle_reaches(blast_dome, 48.0, carl.hurt, true, &[]));
    // Donut at 207 px/s: her collider provokes at 30.4, her hurtbox is 2 px
    // behind it; 24 ticks later her hurtbox's far side is past the 29.3 px reach.
    let donut_x = -(30.39 + 12.0) + 24.0 * (207.0 / 60.0);
    let donut = donut_at(donut_x);
    assert!(
        !circle_reaches(blast_dome, 48.0, donut.hurt, true, &[]),
        "Donut barely escapes (hurtbox left edge {})",
        donut.hurt.left
    );
    assert!(donut.hurt.left - 29.3 < 2.0, "about a pixel of margin");
    // And one tick fewer is a hit: the margin is exactly this thin.
    let early = donut_at(-(30.39 + 12.0) + 23.0 * (207.0 / 60.0));
    assert!(circle_reaches(blast_dome, 48.0, early.hurt, true, &[]));
    let _ = t;
}

#[test]
fn the_blast_reach_tables() {
    let t = tuning();
    let dome = (0.0_f32, 62.0_f32);
    let reach = |body: Rect| circle_reaches(dome, 48.0, body, true, &[]);
    // H 68: Carl is hit within 46.9 px, Donut (hurtbox top 24) within 29.3.
    let carl = |gap: f32| carl_left_of_dome(gap).hurt;
    assert!(reach(carl(46.94)) && !reach(carl(46.96)));
    let donut = |gap: f32| {
        let x = -(gap + 10.0);
        donut_at(x).hurt
    };
    assert!(reach(donut(29.31)) && !reach(donut(29.34)));
    // H 80 / 84: Carl 42.7 / 40.3 and Donut never, grounded.
    let at = |h: f32, body: Rect| circle_reaches((0.0, h - 6.0), 48.0, body, true, &[]);
    assert!(at(80.0, carl(42.65)) && !at(80.0, carl(42.68)));
    assert!(at(84.0, carl(40.33)) && !at(84.0, carl(40.37)));
    assert!(!at(80.0, donut(0.0)) && !at(84.0, donut(0.0)));
    // H 96: the blast still reaches Carl within 29.3 px (it needs a jump to be
    // triggered, see the trigger table, but a hero under it when it goes off is hit).
    assert!(at(96.0, carl(29.31)) && !at(96.0, carl(29.34)));
    // Wall sticker: hit within 54 px of the face when the hurtbox spans the dome.
    let wall_dome = (6.0_f32, 32.0_f32);
    let body = |left: f32| Rect {
        left,
        bottom: 0.0,
        width: 24.0,
        height: 52.0,
    };
    assert!(circle_reaches(wall_dome, 48.0, body(53.99), true, &[]));
    assert!(!circle_reaches(wall_dome, 48.0, body(54.01), true, &[]));
    let _ = t;
}

// ------------------------------------------------------------ the blast

fn blast_at(dome: (f32, f32), owner: Option<u32>) -> RotBlast {
    RotBlast::new(dome, tuning().blast_profile(), owner, 3)
}

fn body_at(x: f32, w: f32, h: f32) -> Rect {
    Rect {
        left: x - w / 2.0,
        bottom: 0.0,
        width: w,
        height: h,
    }
}

#[test]
fn both_heroes_are_hurt_once_each_by_one_blast_and_nothing_is_evadable() {
    let mut b = blast_at((100.0, 62.0), Some(0));
    let targets = [
        BlastTarget {
            id: 0,
            kind: TargetKind::Hero(HeroId::Carl),
            body: body_at(90.0, 24.0, 52.0),
            x: 90.0,
        },
        BlastTarget {
            id: 1,
            kind: TargetKind::Hero(HeroId::Donut),
            body: body_at(105.0, 20.0, 24.0),
            x: 105.0,
        },
    ];
    let out = b.evaluate(&targets, &[]);
    let hurts: Vec<&HeroBlow> = out
        .iter()
        .filter_map(|e| match e {
            BlastEffect::HeroHurt(h) => Some(h),
            _ => None,
        })
        .collect();
    assert_eq!(hurts.len(), 2, "K3: every hero in range is hurt");
    assert_eq!(hurts[0].hero, HeroId::Carl);
    assert_eq!(hurts[1].hero, HeroId::Donut);
    for h in &hurts {
        assert_eq!((h.damage, h.knock_vy, h.from_x), (1, 160.0, 100.0));
        assert_eq!(h.source, rot_source(3));
    }
    assert_eq!(hurts[0].knock_vx, -120.0, "signed away from the centre");
    assert_eq!(hurts[1].knock_vx, 120.0);
    // One evaluation per victim per blast: the same blast hurts nobody twice.
    assert!(b.evaluate(&targets, &[]).is_empty());
    assert_eq!(b.evaluated(), 2);
    // The evaluation set grows only with victims inside the radius.
    let outside = [BlastTarget {
        id: 5,
        kind: TargetKind::Goblin,
        body: body_at(300.0, 24.0, 38.0),
        x: 300.0,
    }];
    assert!(b.evaluate(&outside, &[]).is_empty());
    assert_eq!(b.evaluated(), 2);
}

#[test]
fn the_world_blast_policy_decides_who_is_hurt_not_the_initiator() {
    let t = tuning();
    let mut p = t.blast_profile();
    p.hero_damage = HeroDamagePolicy::NoHeroes;
    let mut b = RotBlast::new((0.0, 30.0), p, Some(0), 0);
    let targets = [BlastTarget {
        id: 0,
        kind: TargetKind::Hero(HeroId::Carl),
        body: body_at(0.0, 24.0, 52.0),
        x: 0.0,
    }];
    assert!(
        b.evaluate(&targets, &[]).is_empty(),
        "no_heroes spares every hero"
    );
    // The hero who set it off is hurt like anyone else under all_heroes.
    let mut b = blast_at((0.0, 30.0), Some(0));
    assert_eq!(
        b.evaluate(&targets, &[]).len(),
        1,
        "credit never spares the initiator"
    );
}

#[test]
fn a_blast_selects_by_radius_and_launches_goblins() {
    // HAZ-05: goblins at 40 and 60 px from the dome centre: only the inner one.
    let mut b = blast_at((0.0, 19.0), Some(1));
    let goblin = |id: u32, x: f32| BlastTarget {
        id,
        kind: TargetKind::Goblin,
        body: Rect {
            left: x - 12.0,
            bottom: 0.0,
            width: 24.0,
            height: 38.0,
        },
        x,
    };
    // Centres 52 and 72 px away: the near edges are 40 and 60.
    let out = b.evaluate(&[goblin(1, 52.0), goblin(2, 72.0)], &[]);
    assert_eq!(out.len(), 1);
    match out[0] {
        BlastEffect::EnemyHit {
            id,
            damage,
            reaction,
            credit,
            kind,
        } => {
            assert_eq!(
                (id, damage, credit, kind),
                (1, 1, Some(1), TargetKind::Goblin)
            );
            assert_eq!(
                reaction,
                EnemyReaction::Launch {
                    vx: 160.0,
                    vy: 180.0
                }
            );
        }
        other => panic!("{other:?}"),
    }
    // Launched away from the centre on the other side.
    let mut b = blast_at((0.0, 19.0), None);
    let out = b.evaluate(&[goblin(1, -40.0)], &[]);
    match out[0] {
        BlastEffect::EnemyHit {
            reaction, credit, ..
        } => {
            assert_eq!(
                reaction,
                EnemyReaction::Launch {
                    vx: -160.0,
                    vy: 180.0
                }
            );
            assert_eq!(credit, None, "no initiator: no credit");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_victim_kind_has_its_effect() {
    let mut b = blast_at((0.0, 19.0), Some(0));
    let near = |kind, id| BlastTarget {
        id,
        kind,
        body: body_at(10.0, 24.0, 38.0),
        x: 10.0,
    };
    let out = b.evaluate(
        &[
            near(TargetKind::Engineer, 1),
            near(TargetKind::Llama, 2),
            near(TargetKind::Slime, 3),
            near(TargetKind::Sticker, 4),
        ],
        &[],
    );
    assert_eq!(out.len(), 3, "another sticker is unaffected");
    let reactions: Vec<EnemyReaction> = out
        .iter()
        .map(|e| match e {
            BlastEffect::EnemyHit {
                reaction, damage, ..
            } => {
                assert_eq!(*damage, 1);
                *reaction
            }
            other => panic!("{other:?}"),
        })
        .collect();
    assert!(matches!(reactions[0], EnemyReaction::Launch { .. }));
    assert_eq!(reactions[1], EnemyReaction::Kick);
    assert_eq!(reactions[2], EnemyReaction::None, "gel: no state change");
    // The sticker was still evaluated (once) but takes nothing.
    assert_eq!(b.evaluated(), 4);
}

#[test]
fn core_zero_never_point_blank() {
    // A goblin whose body contains the dome centre (sq == 0) takes the outer
    // result: 1 damage and a launch, never a point-blank kill.
    let mut b = blast_at((10.0, 19.0), None);
    let hugging = BlastTarget {
        id: 1,
        kind: TargetKind::Goblin,
        body: body_at(10.0, 24.0, 38.0),
        x: 10.0,
    };
    assert!(hugging.body.squared_distance_to(10.0, 19.0) == 0.0);
    let out = b.evaluate(&[hugging], &[]);
    match out[0] {
        BlastEffect::EnemyHit {
            damage, reaction, ..
        } => {
            assert_eq!(damage, 1);
            assert!(matches!(reaction, EnemyReaction::Launch { .. }));
        }
        other => panic!("{other:?}"),
    }
    // And the effect type has no point-blank flag to set.
    assert_eq!(tuning().hazq30_core_radius_px, 0);
}

#[test]
fn line_of_sight_shields_a_victim_from_the_blast() {
    let wall = Rect {
        left: 20.0,
        bottom: 0.0,
        width: 8.0,
        height: 200.0,
    };
    let mut b = blast_at((0.0, 30.0), None);
    let behind = BlastTarget {
        id: 1,
        kind: TargetKind::Hero(HeroId::Carl),
        body: body_at(40.0, 24.0, 52.0),
        x: 40.0,
    };
    assert!(b.evaluate(&[behind], &[wall]).is_empty());
    let mut b = blast_at((0.0, 30.0), None);
    assert_eq!(b.evaluate(&[behind], &[]).len(), 1);
}

#[test]
fn the_charge_position_puts_the_blast_centre_on_the_dome() {
    let t = tuning();
    let s = RotSticker::new(ceiling(84, 760, 824, 760));
    let (x, foot_y) = s.charge_xy(&t);
    let charge = BlastCharge::new(x, foot_y, 0, None);
    assert_eq!(charge.centre(), s.dome(&t));
    assert_eq!(s.dome(&t), (760.0, 78.0));
}

#[test]
fn the_feed_rows_are_the_circles_bounding_boxes() {
    let t = tuning();
    let s = RotSticker::new(ceiling(84, 760, 824, 760));
    let trig = s.trigger_bounds(&t);
    assert_eq!(
        (trig.left, trig.bottom, trig.width, trig.height),
        (728.0, 46.0, 64.0, 64.0)
    );
    let blast = s.blast_bounds(&t);
    assert_eq!((blast.width, blast.height), (96.0, 96.0));
    let frame = s.frame(&t);
    assert_eq!(frame.center(), s.dome(&t));
}

// ------------------------------------------------- respawn and determinism

#[test]
fn a_defeat_respawn_keeps_spent_spent_and_latches_the_rest() {
    let t = tuning();
    // Warning: back to latched at its start with the charge written back.
    let mut s = RotSticker::new(ceiling(84, 0, 60, 0));
    stepped(&mut s, 5, &[carl_left_of_dome(10.0)]);
    assert_eq!(s.state(), StickerState::Warning);
    s.defeat_respawn();
    assert_eq!(s.state(), StickerState::Latched);
    assert_eq!((s.detonate_tick(), s.owner(), s.s()), (0, None, 0));
    // Scuttling: back to latched at the start point.
    let mut s = RotSticker::new(ceiling(112, 0, 60, 60));
    stepped(&mut s, 1, &[carl_at(20.0)]);
    for tick in 2..20 {
        stepped(&mut s, tick, &[carl_at(20.0)]);
    }
    assert!(s.s() < 60);
    s.defeat_respawn();
    assert_eq!((s.state(), s.s()), (StickerState::Latched, 60));
    // Spent stays spent.
    let mut s = RotSticker::new(ceiling(84, 0, 60, 0));
    s.chain(1, Some(0));
    stepped(&mut s, 9, &[]);
    assert!(s.is_spent());
    s.defeat_respawn();
    assert!(s.is_spent());
    // A full restart restores even a spent sticker: the hash equals a fresh one.
    s.reset();
    assert_eq!(
        hash_of(&s),
        hash_of(&RotSticker::new(ceiling(84, 0, 60, 0)))
    );
    assert!(s.hurtbox(&t).is_some());
}

fn hash_of(s: &RotSticker) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    s.mix_into(&mut |v| h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3));
    h
}

#[test]
fn the_same_inputs_give_the_same_hashes_tick_by_tick() {
    let run = || {
        let mut s = RotSticker::new(ceiling(84, 700, 860, 760));
        let mut out = Vec::new();
        for tick in 1..160u64 {
            let x = 900.0 - tick as f32 * 2.0;
            let heroes = [carl_at(x), donut_at(x - 80.0)];
            if tick == 90 {
                s.chain(tick, Some(1));
            }
            stepped(&mut s, tick, &heroes);
            out.push(hash_of(&s));
        }
        out
    };
    let a = run();
    assert_eq!(a, run());
    let unique: std::collections::BTreeSet<u64> = a.iter().copied().collect();
    assert!(
        unique.len() >= 6,
        "the hash follows the state: {}",
        unique.len()
    );
}

#[test]
fn the_blast_hash_follows_its_evaluations() {
    let hash = |b: &RotBlast| {
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        b.mix_into(&mut |v| h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3));
        h
    };
    let mut b = blast_at((0.0, 30.0), Some(0));
    let before = hash(&b);
    b.evaluate(
        &[BlastTarget {
            id: 1,
            kind: TargetKind::Goblin,
            body: body_at(10.0, 24.0, 38.0),
            x: 10.0,
        }],
        &[],
    );
    assert_ne!(before, hash(&b));
    assert_eq!(b.dome(), (0.0, 30.0));
    assert_eq!(b.owner(), Some(0));
}

#[test]
fn the_kill_switch_stops_a_sticker_from_provoking() {
    let mut t = tuning();
    t.hazq02_rot_enabled = false;
    let mut s = RotSticker::new(ceiling(68, 0, 0, 0));
    let near = [carl_left_of_dome(10.0)];
    let out = s.step(&t, &ctx(1, &near));
    assert!(out.provoked.is_none());
    assert!(!t.enabled() && !t.on(true));
}

// ------------------------------------------------ Q9: Donut's airborne cast

/// Donut at x 708 jumps on tick 0 (launch 500, the DM-A arc) and presses the
/// cast on jump tick `press`; the bolt (8 x 6) spawns 27 px ahead of her on the
/// release tick `press + 4`, 16 px above her feet, and flies 6 px per tick
/// toward R2 (track 760..816 under the pocket C soffit: a full-height solid
/// 744..824, heights 84..228, with no standable top, PR #204 review F1).
/// Returns the tick on which the bolt provokes R2 by rule B, `None` when it
/// meets the soffit's side face, passes under it, or misses.
fn q9_provoke_tick(press: u32) -> Option<u64> {
    let t = tuning();
    let mut s = RotSticker::new(ceiling(84, 760, 816, 760));
    let soffit = Rect {
        left: 744.0,
        bottom: 84.0,
        width: 80.0,
        height: 144.0,
    };
    let feet = |tick: u32| {
        let (mut y, mut vy) = (0.0_f32, 500.0_f32);
        for _ in 0..=tick {
            vy -= 1050.0 / 60.0;
            y += vy / 60.0;
            if y <= 0.0 {
                y = 0.0;
            }
        }
        y
    };
    let release = press + 4;
    let mut left = 708.0 + 27.0;
    let bottom = feet(release) + 16.0;
    for tick in release..release + 60 {
        let bolt = Rect {
            left,
            bottom,
            width: 8.0,
            height: 6.0,
        };
        if bolt.overlaps(soffit) {
            return None;
        }
        let donut = donut_at(708.0);
        let mut airborne = donut;
        airborne.foot_y = feet(tick);
        let heroes = [airborne];
        let missiles = [Missile {
            id: 1,
            rect: bolt,
            owner: Some(1),
        }];
        let solids = [soffit];
        let mut c = ctx(u64::from(tick), &heroes);
        c.missiles = &missiles;
        c.solids = &solids;
        let out = s.step(&t, &c);
        if let Some(p) = out.provoked {
            assert_eq!(
                p.cause,
                RotCause::Projectile,
                "never by proximity (press {press})"
            );
            assert_eq!(out.consumed, [1]);
            assert_eq!(p.player, Some(1));
            return Some(u64::from(tick));
        }
        left += 6.0;
    }
    None
}

#[test]
fn q9_the_airborne_cast_window_into_r2_is_a_handful_of_press_ticks() {
    let rise: Vec<u32> = (0..12).filter(|&p| q9_provoke_tick(p).is_some()).collect();
    let fall: Vec<u32> = (36..56).filter(|&p| q9_provoke_tick(p).is_some()).collect();
    // The sheet predicts cast presses on jump ticks 0..=6 and 42..=49 (7 + 8
    // press ticks) from the scene's exact bolt step. This domain model, with the
    // bolt spawning on the release tick without moving that tick and the jump
    // tick as the first physics tick, finds the same two windows one tick
    // earlier: 0..=5 and 41..=48. The scene fixture pins the exact edges
    // (HAZ-C); the open question (HAZ-0 section 21, item 9) is whether a player
    // can find a window this narrow.
    assert_eq!(rise, (0..=5).collect::<Vec<_>>());
    assert_eq!(fall, (41..=48).collect::<Vec<_>>());
    // Just past the rise window the bolt meets the soffit's side face.
    assert_eq!(q9_provoke_tick(6), None);
    assert_eq!(q9_provoke_tick(7), None);
    // Her collider stays 40 px from the dome at every height: never proximity.
    let t = tuning();
    let dome = ceiling(84, 760, 816, 760).dome(760, t.hazq24_dome_offset_px);
    for feet in [0.0, 30.0, 60.0, 90.0, 114.0] {
        let mut d = donut_at(708.0);
        d.foot_y = feet;
        let c = collider(&d);
        assert!(
            !circle_reaches(dome, 32.0, c, true, &[]),
            "feet {feet}: the collider is {} px from the dome",
            dome.0 - c.right()
        );
    }
}
