//! ACT1-B: the opening profile's domain tests (ACT1-T11 and the tuning's own rules): the commit
//! rule, the charge through the barricade, the opening rescue rules, the pocket's coordinates.

use super::*;
use crate::camera_lab::room::IRect;

const FLOOR: f32 = 92.0;
const LEDGE: f32 = 16.0;

fn opening_site() -> DozerSite {
    DozerSite {
        trigger_x: 1072,
        idle_trigger_ticks: 216_000,
        emerge_front_x: 888,
        stop_x: 2856,
        lintel: None,
        profile: SiteProfile::Opening,
        trigger_by: TriggerBy::Rearmost,
        pockets: vec![PocketSite {
            id: "alcove".into(),
            rect: IRect::new(2072, 460, 80, 104),
            ledge: "alcove_ledge".into(),
        }],
        timbers: vec![
            TimberSite {
                id: "timber_a".into(),
                x: 1104,
            },
            TimberSite {
                id: "timber_b".into(),
                x: 1424,
            },
        ],
        wedge: true,
        floor_px: 580,
        barricade_x: Some(2312),
    }
}

fn charge() -> ChargeTuning {
    DozerTuning::opening().charge.expect("the opening's charge")
}

fn machine() -> Dozer {
    Dozer::with_tuning(DozerTuning::opening(), opening_site(), 2928.0, FLOOR)
}

/// A machine in the cruise with its wheel front at `front`.
fn cruising(front: f32) -> Dozer {
    let mut d = machine();
    d.o = front - d.tuning.wheel_front();
    d.state = DozerState::CommittedChase;
    d.band = Some(Band::Cruise);
    d.speed = d.profile.cruise;
    d
}

fn view(id: HeroId, x: f32, foot: f32, bot: bool) -> HeroView {
    HeroView {
        id,
        present: true,
        x,
        foot_y: foot,
        grounded: true,
        hurt: body_rect(x, foot),
        vulnerable: true,
        bot,
    }
}

fn on_lane(id: HeroId, x: f32, bot: bool) -> HeroView {
    view(id, x, FLOOR, bot)
}

fn in_pocket(id: HeroId, x: f32, bot: bool) -> HeroView {
    view(id, x, FLOOR + LEDGE, bot)
}

fn absent(id: HeroId) -> HeroView {
    HeroView {
        present: false,
        ..on_lane(id, 3000.0, false)
    }
}

fn ctx<'a>(tick: u64, heroes: [HeroView; 2], solids: &'a [Block], solo: bool) -> DozerCtx<'a> {
    DozerCtx {
        tick,
        heroes,
        solids,
        solo,
        view_x: (0.0, 4000.0),
    }
}

fn step(d: &mut Dozer, tick: u64, heroes: [HeroView; 2], solo: bool) -> DozerOut {
    step_with(d, tick, heroes, &[], solo)
}

fn step_with(
    d: &mut Dozer,
    tick: u64,
    heroes: [HeroView; 2],
    solids: &[Block],
    solo: bool,
) -> DozerOut {
    let mut out = DozerOut::default();
    d.step(&ctx(tick, heroes, solids, solo), &mut out);
    out
}

/// A wall behind the hull's rear, so a push-out would put a body into it (the pinned rule).
fn wall_behind(d: &Dozer) -> Block {
    let rear = d.push_box().left;
    Block {
        rect: Rect {
            left: rear - 60.0,
            bottom: FLOOR,
            width: 60.0,
            height: 200.0,
        },
        material: crate::prototype::Material::Stone,
        intact: true,
    }
}

const POCKET: (f32, f32) = (2072.0, 2152.0);

fn carl_at(x: f32) -> CommitHero {
    CommitHero {
        x,
        run_px_s: hero_run_px_s(HeroId::Carl),
    }
}

fn donut_at(x: f32) -> CommitHero {
    CommitHero {
        x,
        run_px_s: hero_run_px_s(HeroId::Donut),
    }
}

fn holds(front: f32, v: f32, heroes: &[CommitHero]) -> bool {
    commit_rule_holds(&charge(), POCKET, Some(2312.0), front, v, heroes)
}

// ---------------------------------------------------------------- the data --

#[test]
fn the_opening_data_matches_its_pin_and_leaves_room_3_alone() {
    let t = DozerTuning::opening_pinned().expect("pinned");
    assert_eq!(&t, DozerTuning::opening());
    assert!(
        DozerTuning::active().charge.is_none(),
        "room 3 has no charge"
    );
    assert_eq!(DozerTuning::active().revenge.dzq06_cruise_px_s, 170 - 10);
    let drifted = format!("{OPENING_TUNING_JSON} ");
    assert!(matches!(
        DozerTuning::opening_from_json_pinned(&drifted, OPENING_TUNING_HASH),
        Err(DozerError::HashMismatch { .. })
    ));
    // The merged tuning keeps the shared machine values and lays the speed band over them.
    let (a, o) = (DozerTuning::active(), DozerTuning::opening());
    assert_eq!(a.dzq01_wheel_axle, o.dzq01_wheel_axle);
    assert_eq!(a.dzq02_push_box, o.dzq02_push_box);
    assert_eq!(a.dzq14_brake_ticks, o.dzq14_brake_ticks);
    let p = o.revenge();
    assert_eq!(
        (
            p.cruise,
            p.catch_up,
            p.mercy,
            p.catch_up_gap,
            p.mercy_gap,
            p.accel
        ),
        (160.0, 185.0, 150.0, 260.0, 96.0, 4.0)
    );
    assert_eq!(
        TUNING_HASH, 0x146c_8ef7_91c8_e9e9,
        "room 3's pin does not move"
    );
}

#[test]
fn the_opening_tuning_rules_name_their_field() {
    let json = |edit: &dyn Fn(&mut serde_json::Value)| {
        let mut v: serde_json::Value = serde_json::from_str(OPENING_TUNING_JSON).unwrap();
        edit(&mut v);
        DozerTuning::opening_from_json(&v.to_string())
    };
    let field_of = |r: Result<DozerTuning, DozerError>| match r.unwrap_err() {
        DozerError::Field { field, .. } => field,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        field_of(json(&|v| v["dzq10_charge"]["tell_ticks"] = 0.into())),
        "dzq10_charge.tell_ticks"
    );
    assert_eq!(
        field_of(json(&|v| v["dzq10_charge"]["speed_px_s"] = 100.into())),
        "dzq10_charge.speed_px_s"
    );
    // K5 holds for the band speeds (the charge is exempt).
    let ceiling = k5_speed_ceiling() as u64;
    assert_eq!(
        field_of(json(&|v| v["dzq07_catch_up_px_s"] = (ceiling + 1).into())),
        "revenge.dzq07_catch_up_px_s"
    );
    assert!(json(&|v| v["dzq10_charge"]["speed_px_s"] = 400.into()).is_ok());
    assert!(matches!(
        json(&|v| v["schema"] = "x".into()),
        Err(DozerError::Field {
            field: "schema",
            ..
        })
    ));
}

// ------------------------------------------------------------ the commit rule --

/// ACT1-T11 `the_charge_commit_rule`: the rows of integration plan section 5.3.
#[test]
fn the_charge_commit_rule() {
    let both = [carl_at(2100.0), donut_at(2084.0)];
    // Rule 4's boundary at the cruise: front 1855 fails, 1856 holds (96 + 360 px of reach).
    assert!(!holds(1855.0, 160.0, &both));
    assert!(holds(1856.0, 160.0, &both));
    // At the catch-up speed the commit front is 1841 (111 px of tell).
    assert!(!holds(1840.0, 185.0, &both));
    assert!(holds(1841.0, 185.0, &both));
    // At the mercy speed 1862 (90 px of tell): the window's late edge, 2072 - 140 = 1932.
    assert!(!holds(1861.0, 150.0, &both));
    assert!(holds(1862.0, 150.0, &both));
    assert!(holds(1932.0, 160.0, &both));
    // Rule 2: the gap to the rearmost body's left edge is at least 140 (1933 leaves 139).
    assert!(!holds(1933.0, 160.0, &both));
    // Rule 1: every living hero is at or past the arm.
    assert!(!holds(1856.0, 160.0, &[carl_at(2100.0), carl_at(1887.0)]));
    // No living hero: no commit.
    assert!(!holds(1856.0, 160.0, &[]));
    // The wheel ticks of the first run: 66 at the cruise, 36 of tell and 30 of charge.
    assert!((66..=67).contains(&wheel_ticks(&charge(), 1856.0, 160.0, 2072.0)));
    assert!((66..=67).contains(&wheel_ticks(&charge(), 1841.0, 185.0, 2072.0)));
    // The walk-back case: Carl (185 px/s) waits at the barricade, 160 px from the in-pocket
    // pivot 2140: 52 walk ticks and the 20 margin exceed 66, so there is no charge.
    assert_eq!(walk_ticks(carl_at(2300.0), POCKET), 52);
    assert!(!holds(1856.0, 160.0, &[carl_at(2300.0), donut_at(2090.0)]));
    // Donut (207 px/s) at the same spot is 47 walk ticks: 67 > 66, still no charge (the rule-3
    // boundary: the fixtures put Carl there, and the rule's margin is what is left).
    assert_eq!(walk_ticks(donut_at(2300.0), POCKET), 47);
    // A hero already inside the pocket has no walk.
    assert_eq!(walk_ticks(carl_at(2084.0), POCKET), 0);
    assert_eq!(walk_ticks(carl_at(2140.0), POCKET), 0);
    // The pocket-behind-the-wheel case: with the front already at the pocket's left edge, a
    // hero beyond it is inside the 140 gap, and a hero behind the front gives a negative gap.
    assert!(!holds(2072.0, 160.0, &[carl_at(2200.0), carl_at(2300.0)]));
    assert!(!holds(1900.0, 160.0, &[carl_at(1895.0), carl_at(2100.0)]));
    // Rule 3 fails when the hero is too far from the pocket for the wheel's tell and charge:
    // gap 140 at 1892, a hero at 2032 (left edge exactly 140 ahead) walks 52 px, 17 ticks.
    assert_eq!(walk_ticks(carl_at(2032.0), POCKET), 17);
    // Without a barricade rule 4 is vacuous.
    assert!(commit_rule_holds(
        &charge(),
        POCKET,
        None,
        1700.0,
        160.0,
        &both
    ));
    // O9's slowest-speed window: today 1862..1932 (the plan's arithmetic).
    let c = charge();
    let reach =
        (f32::from(c.tell_ticks) * 150.0 + f32::from(c.speed_px_s) * f32::from(c.max_ticks)) / 60.0;
    assert_eq!(
        (2312.0 - reach, 2072.0 - c.min_gap_px as f32),
        (1862.0, 1932.0)
    );
}

// ------------------------------------------------------- the machine's charge --

/// Step a machine whose heroes stand in the pocket until `done` says stop, returning the log.
fn run_until(
    d: &mut Dozer,
    heroes: [HeroView; 2],
    mut tick: u64,
    max: u64,
    done: impl Fn(&Dozer, &[(u64, DozerEvent)]) -> bool,
) -> Vec<(u64, DozerEvent)> {
    let mut log = Vec::new();
    while tick < max {
        tick += 1;
        let out = step(d, tick, heroes, false);
        log.extend(out.events.into_iter().map(|e| (tick, e)));
        if done(d, &log) {
            break;
        }
    }
    log
}

fn names(log: &[(u64, DozerEvent)]) -> Vec<&'static str> {
    log.iter().map(|(_, e)| e.name()).collect()
}

/// ACT1-T11 `the_charge_runs_through_the_barricade`: the instant charge speed, the run past the
/// impact to `stop_x` with one brake, the wedge, the timbers.
#[test]
fn the_charge_runs_through_the_barricade() {
    let heroes = [
        in_pocket(HeroId::Carl, 2100.0, false),
        in_pocket(HeroId::Donut, 2124.0, false),
    ];
    let mut d = cruising(900.0);
    let log = run_until(&mut d, heroes, 0, 4000, |d, _| d.wedged());
    let seq = names(&log);
    let at = |name: &str| {
        seq.iter()
            .position(|n| *n == name)
            .unwrap_or_else(|| panic!("{name}: {seq:?}"))
    };
    assert_eq!(
        seq.iter().filter(|n| **n == "brake").count(),
        1,
        "exactly one brake: {seq:?}"
    );
    assert!(at("timber_smashed") < at("charge_tell"));
    assert!(at("charge_tell") < at("charge_committed"));
    assert!(at("charge_committed") < at("barrier_impact"));
    assert!(at("barrier_impact") < at("charge_ended"));
    assert!(at("charge_ended") < at("brake"));
    assert!(at("brake") < at("escaped") && at("escaped") < at("wedged"));
    assert_eq!(
        seq.iter().filter(|n| **n == "timber_smashed").count(),
        2,
        "both timbers are ahead of 900"
    );
    // The commit front lies in the window 1841..1932 and the tell lasts 36 ticks.
    let commit = d.commit.expect("a commit was recorded");
    assert!((1841.0..=1932.0).contains(&commit.front), "{commit:?}");
    let tick_of = |name: &str| log.iter().find(|(_, e)| e.name() == name).unwrap().0;
    assert_eq!(tick_of("charge_committed") - tick_of("charge_tell"), 36);
    // The impact lands inside the 90 charge ticks; the charge then runs on at 240 to stop_x
    // (by_ticks counts the whole run) and the machine wedges at the arch.
    assert!(tick_of("barrier_impact") - tick_of("charge_committed") <= 90);
    assert!(d.barrier_hit() && d.wedged());
    let ended = log.iter().find_map(|(_, e)| match e {
        DozerEvent::ChargeEnded { by_ticks } => Some(*by_ticks),
        _ => None,
    });
    assert!(ended.is_some_and(|n| n > 90), "{ended:?}");
    assert_eq!(d.state, DozerState::Escaped(EscapePhase::Idle));
    assert_eq!(d.wheel_front().round() as i32, 2856);
    // One charge per attempt.
    assert_eq!(d.charges_used(), 1);
}

/// The charge speed is set at once: the first charge tick moves 4 px, no acceleration ramp.
#[test]
fn the_charge_speed_is_instant() {
    let heroes = [
        in_pocket(HeroId::Carl, 2100.0, false),
        in_pocket(HeroId::Donut, 2124.0, false),
    ];
    let mut d = cruising(1860.0);
    d.speed = 160.0;
    d.charge = ChargeStage::Tell { t: 35 };
    let o0 = d.o;
    let out = step(&mut d, 1, heroes, false);
    assert!(out.events.contains(&DozerEvent::ChargeCommitted));
    assert_eq!(d.speed, 240.0);
    assert!(
        (d.o - o0 - 160.0 / 60.0).abs() < 1e-3,
        "the committing tick still moves at the held speed"
    );
    let before = d.o;
    step(&mut d, 2, heroes, false);
    assert!(
        (d.o - before - 4.0).abs() < 1e-3,
        "the first charge tick moves 4 px"
    );
}

/// An impact on the last charge tick counts (the impact is tested before `max_ticks`).
#[test]
fn an_impact_on_the_last_charge_tick_counts() {
    let heroes = [
        in_pocket(HeroId::Carl, 2100.0, false),
        in_pocket(HeroId::Donut, 2124.0, false),
    ];
    let mut d = cruising(2312.0 - 90.0 * 4.0);
    d.speed = 240.0;
    d.charge = ChargeStage::Charge { t: 0 };
    d.armed = true;
    d.charges_used = 1;
    let log = run_until(&mut d, heroes, 0, 90, |_, _| false);
    let seq = names(&log);
    assert!(seq.contains(&"barrier_impact"), "{seq:?}");
    assert!(
        !seq.contains(&"charge_ended"),
        "the impact is not an overshoot: {seq:?}"
    );
    assert_eq!(
        log.iter()
            .find(|(_, e)| *e == DozerEvent::BarrierImpact)
            .unwrap()
            .0,
        90
    );
    assert!(matches!(d.charge, ChargeStage::Charge { .. }));
}

/// The defensive path: a charge that never meets the barricade ends at `max_ticks`, brakes
/// with its slide, strains and returns to the cruise (a synthetic tuning, no barricade in
/// reach: rule 4 makes it unreachable in `act1_chase`).
#[test]
fn a_charge_short_of_the_barricade_overshoots_and_returns_to_the_cruise() {
    let mut t = DozerTuning::opening().clone();
    let mut c = t.charge.unwrap();
    c.max_ticks = 20;
    t.charge = Some(c);
    let tuning: &'static DozerTuning = Box::leak(Box::new(t));
    let mut d = Dozer::with_tuning(tuning, opening_site(), 2928.0, FLOOR);
    d.o = 1500.0 - tuning.wheel_front();
    d.state = DozerState::CommittedChase;
    d.speed = 160.0;
    d.charge = ChargeStage::Tell { t: 35 };
    d.armed = true;
    d.charges_used = 1;
    let heroes = [
        in_pocket(HeroId::Carl, 2100.0, false),
        in_pocket(HeroId::Donut, 2124.0, false),
    ];
    let log = run_until(&mut d, heroes, 0, 400, |d, log| {
        d.charge == ChargeStage::Cruise && names(log).contains(&"charge_ended")
    });
    let seq = names(&log);
    assert_eq!(
        seq.iter()
            .filter(|n| **n == "charge_committed" || **n == "charge_ended")
            .count(),
        2,
        "{seq:?}"
    );
    assert!(
        seq.contains(&"charge_ended")
            && !seq.contains(&"barrier_impact")
            && !seq.contains(&"brake")
    );
    let by = log.iter().find_map(|(_, e)| match e {
        DozerEvent::ChargeEnded { by_ticks } => Some(*by_ticks),
        _ => None,
    });
    assert_eq!(by, Some(20));
    assert_eq!(d.state, DozerState::CommittedChase, "back in the chase");
    // The wheel was lethal in every sub-state (it stays CommittedChase).
    assert!(d.wheel_lethal());
    // One charge only: the cruise resumes and no second tell begins.
    run_until(&mut d, heroes, 400, 700, |_, _| false);
    assert_eq!(d.charges_used(), 1);
}

/// A run to `BarrierImpact` and then a respawn (`slot_respawn` rebuilds the machine from its
/// pristine copy) closes the barricade again by construction: `barrier_hit()` is false and the
/// machine is dormant. The scene-level half (the `dozer_contact` solid standing again) is the
/// chase fixtures'.
#[test]
fn a_respawned_machine_has_no_barrier_hit() {
    let pristine = machine();
    let heroes = [
        in_pocket(HeroId::Carl, 2100.0, false),
        in_pocket(HeroId::Donut, 2124.0, false),
    ];
    let mut d = pristine.clone();
    d.state = DozerState::CommittedChase;
    d.band = Some(Band::Cruise);
    d.speed = d.profile.cruise;
    d.o = 900.0 - d.tuning.wheel_front();
    run_until(&mut d, heroes, 0, 4000, |d, _| d.barrier_hit());
    assert!(d.barrier_hit());
    let again = pristine.clone();
    assert!(!again.barrier_hit() && !again.wedged() && again.charges_used() == 0);
    assert_eq!(again.state.name(), "dormant");
    assert_ne!(d.state.name(), "dormant");
}

/// The wake is the rearmost hero's x (ACT1-AD6); room 3's lead rule is untouched.
#[test]
fn the_opening_wakes_on_the_rearmost_hero() {
    let mut d = machine();
    let leader_only = [
        on_lane(HeroId::Carl, 1500.0, false),
        on_lane(HeroId::Donut, 500.0, false),
    ];
    let out = step(&mut d, 1, leader_only, false);
    assert!(out.events.is_empty() && matches!(d.state, DozerState::Dormant { .. }));
    let both = [
        on_lane(HeroId::Carl, 1500.0, false),
        on_lane(HeroId::Donut, 1072.0, false),
    ];
    let out = step(&mut d, 2, both, false);
    assert_eq!(out.events, vec![DozerEvent::Rumble { by_trigger: true }]);
    // A defeated hero does not hold the wake back.
    let mut d = machine();
    let one = [on_lane(HeroId::Carl, 1100.0, false), absent(HeroId::Donut)];
    let out = step(&mut d, 1, one, false);
    assert_eq!(out.events, vec![DozerEvent::Rumble { by_trigger: true }]);
    // Room 3's lead rule: the leader alone wakes it.
    let mut r3 = Dozer::new(
        DozerSite::revenge(256, 300, 32, 1000, IRect::new(1040, 352, 304, 116)),
        1264.0,
        FLOOR,
    );
    let out = step(
        &mut r3,
        1,
        [
            on_lane(HeroId::Carl, 300.0, false),
            on_lane(HeroId::Donut, 40.0, false),
        ],
        false,
    );
    assert_eq!(out.events, vec![DozerEvent::Rumble { by_trigger: true }]);
}

// -------------------------------------------------------------- the pocket --

/// The pocket test's y conversion: the rect is layout px (y down), the hero world px (y up).
#[test]
fn the_pocket_test_converts_layout_to_world() {
    let d = machine();
    let on_ledge = in_pocket(HeroId::Carl, 2112.0, false);
    let on_lane = on_lane(HeroId::Donut, 2112.0, false);
    assert_eq!(d.pocket_of(&on_ledge), Some(0), "feet on the ledge top: in");
    assert_eq!(
        d.pocket_of(&on_lane),
        None,
        "on the lane in front of the ledge: out"
    );
    // Airborne over the pocket counts; the body must lie inside the pocket's x span.
    let airborne = view(HeroId::Carl, 2112.0, FLOOR + 60.0, false);
    assert_eq!(d.pocket_of(&airborne), Some(0));
    assert_eq!(
        d.pocket_of(&in_pocket(HeroId::Carl, 2083.0, false)),
        None,
        "body left of the span"
    );
    assert_eq!(
        d.pocket_of(&in_pocket(HeroId::Carl, 2084.0, false)),
        Some(0)
    );
    assert_eq!(
        d.pocket_of(&in_pocket(HeroId::Carl, 2140.0, false)),
        Some(0)
    );
    assert_eq!(
        d.pocket_of(&in_pocket(HeroId::Carl, 2141.0, false)),
        None,
        "body right of the span"
    );
}

/// The wheel test, the push-out and the rescue all skip a hero in the pocket.
#[test]
fn a_hero_in_the_pocket_is_out_of_the_machines_reach() {
    let mut d = cruising(2060.0);
    // The wheel centre and the hull sweep right over the pocket.
    let heroes = [
        in_pocket(HeroId::Carl, 2112.0, false),
        in_pocket(HeroId::Donut, 2100.0, true),
    ];
    for tick in 1..=120 {
        let out = step(&mut d, tick, heroes, true);
        assert!(
            !out.events
                .iter()
                .any(|e| matches!(e, DozerEvent::WheelHit { .. } | DozerEvent::AiRescue { .. })),
            "tick {tick}: {:?}",
            out.events
        );
        assert!(out.shifts.is_empty() && out.place.is_empty() && out.blows.is_empty());
    }
    // A lane hero at the same x is hit.
    let mut d = cruising(2060.0);
    let heroes = [on_lane(HeroId::Carl, 2112.0, false), absent(HeroId::Donut)];
    let mut hit = false;
    for tick in 1..=120 {
        let out = step(&mut d, tick, heroes, false);
        hit |= out
            .events
            .iter()
            .any(|e| matches!(e, DozerEvent::WheelHit { .. }));
    }
    assert!(hit, "the wheel kills a hero on the lane");
}

// ------------------------------------------------------------------ rescue --

fn rescue_of(out: &DozerOut) -> Option<(RescueSpot, RescueCause, f32)> {
    out.events.iter().find_map(|e| match *e {
        DozerEvent::AiRescue { spot, cause, x, .. } => Some((spot, cause, x as f32)),
        _ => None,
    })
}

/// ACT1-T11 `the_opening_rescue_rules`: no cause in the pocket or fully behind the hull, the
/// Pocket/Behind fallbacks and the tell's rescue, never the tunnel spot before the impact.
#[test]
fn the_opening_rescue_rules() {
    let leader = on_lane(HeroId::Carl, 2500.0, false);
    // A bot fully behind the hull (its right edge at the hull's rear less the zone margin) has
    // no cause, and neither does one that touches the hull's rear without overlapping it.
    let d = cruising(1500.0);
    let rear = d.o + d.tuning.dzq02_push_box.x0 as f32;
    for x in [
        rear - f32::from(d.tuning.dzq17_rescue_zone_margin_px) - HERO_HALF_WIDTH,
        rear - HERO_HALF_WIDTH,
        rear - 200.0,
    ] {
        let mut d = cruising(1500.0);
        let out = step(&mut d, 1, [leader, on_lane(HeroId::Donut, x, true)], true);
        assert_eq!(rescue_of(&out), None, "bot at {x}");
    }
    // A bot whose body overlaps the hull (centre behind the axle) and that the push-out cannot
    // shift (a wall behind: the pinned rule) is rescued: BehindAxle, to the pocket while its
    // left edge is far ahead of the front.
    let mut d = cruising(1500.0);
    let bot_x = d.o - 20.0; // the hull's rear half: a push-out would shift it back
    let walls = [wall_behind(&d)];
    let out = step_with(
        &mut d,
        1,
        [leader, on_lane(HeroId::Donut, bot_x, true)],
        &walls,
        true,
    );
    let (spot, cause, x) = rescue_of(&out).unwrap_or_else(|| panic!("a rescue: {:?}", out.events));
    assert_eq!(
        (spot, cause, x),
        (RescueSpot::Pocket, RescueCause::BehindAxle, 2112.0)
    );
    // The pocket's foot is the ledge top.
    let place = out.place[0];
    assert_eq!(place.2, FLOOR + LEDGE);
    // Near the pocket (the front within the band 24 plus two body widths, 48 px, of its left edge) the
    // spot is the lane right behind the hull.
    let mut d = cruising(2072.0 - 30.0);
    let bot_x = d.o - 20.0; // the hull's rear half: a push-out would shift it back
    let walls = [wall_behind(&d)];
    let out = step_with(
        &mut d,
        1,
        [leader, on_lane(HeroId::Donut, bot_x, true)],
        &walls,
        true,
    );
    let (spot, _, x) = rescue_of(&out).expect("a rescue");
    assert_eq!(spot, RescueSpot::Behind);
    let behind = d.o + d.tuning.dzq02_push_box.x0 as f32
        - f32::from(d.tuning.dzq17_rescue_zone_margin_px)
        - HERO_HALF_WIDTH;
    assert!((x - behind).abs() <= 1.0, "{x} vs {behind}");
    // A spot outside the view margin gives way to the other; with both outside, the pocket.
    let mut d = cruising(2072.0 - 30.0);
    let bot_x = d.o - 20.0; // the hull's rear half: a push-out would shift it back
    let walls = [wall_behind(&d)];
    let mut c = ctx(
        1,
        [leader, on_lane(HeroId::Donut, bot_x, true)],
        &walls,
        true,
    );
    c.view_x = (1900.0, 2300.0); // the pocket (2112) inside, the behind spot (< 1800) outside
    let mut out = DozerOut::default();
    d.step(&c, &mut out);
    assert_eq!(rescue_of(&out).map(|r| r.0), Some(RescueSpot::Pocket));
    let mut d = cruising(2072.0 - 30.0);
    let mut c = ctx(
        1,
        [leader, on_lane(HeroId::Donut, bot_x, true)],
        &walls,
        true,
    );
    c.view_x = (0.0, 600.0); // both outside
    let mut out = DozerOut::default();
    d.step(&c, &mut out);
    assert_eq!(rescue_of(&out).map(|r| r.0), Some(RescueSpot::Pocket));
    // Never the tunnel spot before BarrierImpact, whatever the lane spot says.
    let mut d = cruising(1500.0);
    let out = step(
        &mut d,
        1,
        [leader, on_lane(HeroId::Donut, 1520.0, true)],
        true,
    );
    assert!(
        rescue_of(&out).is_none_or(|r| r.0 != RescueSpot::Tunnel && r.0 != RescueSpot::Lane),
        "{:?}",
        out.events
    );
    // The tell's first tick puts a bot outside the pocket into it (cause Charge), exempt from
    // the spacing check; a bot already inside is left alone.
    let mut d = cruising(1856.0);
    let heroes = [
        in_pocket(HeroId::Carl, 2130.0, false),
        on_lane(HeroId::Donut, 2040.0, true),
    ];
    let mut tell = None;
    for tick in 1..=4 {
        let out = step(&mut d, tick, heroes, true);
        if out.events.contains(&DozerEvent::ChargeTell) {
            tell = Some(out);
            break;
        }
    }
    let out = tell.expect("the commit rule held");
    assert_eq!(
        rescue_of(&out).map(|r| (r.0, r.1)),
        Some((RescueSpot::Pocket, RescueCause::Charge))
    );
    let mut d = cruising(1856.0);
    let heroes = [
        in_pocket(HeroId::Carl, 2130.0, false),
        in_pocket(HeroId::Donut, 2090.0, true),
    ];
    for tick in 1..=4 {
        let out = step(&mut d, tick, heroes, true);
        assert_eq!(rescue_of(&out), None);
    }
    // Room 3 (revenge) keeps its causes: a bot behind the axle is rescued without the overlap rule.
    let mut r3 = Dozer::new(
        DozerSite::revenge(256, 300, 32, 1000, IRect::new(1040, 352, 304, 116)),
        1264.0,
        FLOOR,
    );
    r3.o = 400.0 - r3.tuning.wheel_front();
    r3.state = DozerState::CommittedChase;
    r3.band = Some(Band::Cruise);
    r3.speed = 160.0;
    let rear = r3.o + r3.tuning.dzq02_push_box.x0 as f32;
    let out = step(
        &mut r3,
        1,
        [
            on_lane(HeroId::Carl, 900.0, false),
            on_lane(HeroId::Donut, rear - 100.0, true),
        ],
        true,
    );
    assert_eq!(rescue_of(&out).map(|r| r.1), Some(RescueCause::BehindAxle));
}

/// The wedge: from `Idle` the push box and the nav rows are off, the clang hurtbox stays.
#[test]
fn a_wedged_machine_pushes_nothing_and_has_no_nav_rows() {
    let mut d = machine();
    d.state = DozerState::Escaped(EscapePhase::Strain { ticks_left: 2 });
    d.o = 2856.0 - 4.0 - d.tuning.wheel_front();
    let hero = on_lane(HeroId::Carl, d.o + 10.0, false);
    assert!(!d.nav_rows(660).is_empty() || matches!(d.state, DozerState::Escaped(_)));
    let mut wedged_at = None;
    for tick in 1..=3 {
        let out = step(&mut d, tick, [hero, absent(HeroId::Donut)], false);
        if out.events.contains(&DozerEvent::Wedged) {
            wedged_at = Some(tick);
        }
    }
    assert_eq!(wedged_at, Some(2), "Idle on the strain's last tick");
    assert!(d.wedged() && d.nav_rows(660).is_empty());
    let out = step(&mut d, 4, [hero, absent(HeroId::Donut)], false);
    assert!(out.shifts.is_empty(), "a wedged machine pushes nothing");
    // Room 3's `Idle` (no wedge) still pushes.
    let mut idle = machine();
    idle.site.wedge = false;
    idle.state = DozerState::Escaped(EscapePhase::Idle);
    idle.o = 2856.0 - 4.0 - idle.tuning.wheel_front();
    let out = step(&mut idle, 1, [hero, absent(HeroId::Donut)], false);
    assert!(!out.shifts.is_empty());
}

/// The nav rows are cut around the pocket so the planner may stand in it.
#[test]
fn the_nav_rows_leave_the_pocket_free() {
    let mut d = cruising(1950.0);
    d.state = DozerState::CommittedChase;
    let rows = d.nav_rows(660);
    assert!(!rows.is_empty());
    let (p0, p1) = (2072, 2152);
    for r in &rows {
        assert!(
            r.rect.right() <= p0 || r.rect.x >= p1,
            "{r:?} overlaps the pocket"
        );
    }
    // The telegraph row ahead of the tips reaches the pocket and is cut into one piece left of it.
    assert!(rows.iter().any(|r| r.rect.right() == p0), "{rows:?}");
}

/// The event codes append after room 3's twelve, each hashed (the codes are part of the hash).
#[test]
fn the_appended_events_have_their_codes() {
    let h = HeroId::Carl;
    let events = [
        (DozerEvent::ChargeArmed, 13),
        (DozerEvent::ChargeTell, 14),
        (DozerEvent::ChargeCommitted, 15),
        (DozerEvent::ChargeEnded { by_ticks: 90 }, 16),
        (DozerEvent::TimberSmashed { id: 1 }, 17),
        (DozerEvent::PocketEntered { hero: h }, 18),
        (DozerEvent::BarrierImpact, 19),
        (DozerEvent::Wedged, 20),
    ];
    for (e, code) in events {
        let mut mixed = Vec::new();
        e.mix_into(&mut |v| mixed.push(v));
        assert_eq!(mixed[0], code, "{}", e.name());
    }
    assert_eq!(RescueSpot::Pocket as u8, 2);
    assert_eq!(RescueSpot::Behind as u8, 3);
    assert_eq!(RescueCause::Charge as u8, 3);
    // Room 3's machine mixes nothing of the opening's state.
    let (mut a, mut b) = (Vec::new(), Vec::new());
    let r3 = Dozer::new(
        DozerSite::revenge(256, 300, 32, 1000, IRect::new(1040, 352, 304, 116)),
        1264.0,
        FLOOR,
    );
    r3.mix_into(&mut |v| a.push(v));
    let mut r3b = r3.clone();
    r3b.armed = true;
    r3b.barrier_hit = true;
    r3b.timbers_smashed = 3;
    r3b.mix_into(&mut |v| b.push(v));
    assert_eq!(a, b, "no charge block, no extra mixing");
    let (mut c, mut e) = (Vec::new(), Vec::new());
    let o = machine();
    o.mix_into(&mut |v| c.push(v));
    let mut o2 = o.clone();
    o2.barrier_hit = true;
    o2.mix_into(&mut |v| e.push(v));
    assert_ne!(c, e, "the opening mixes the barricade state");
}
