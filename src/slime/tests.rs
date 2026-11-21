use super::*;
use crate::prototype::{GRAVITY, JUMP_SPEED, Material, TICK_SECONDS};
use serde_json::Value;

fn tuning() -> SlimeTuning {
    SlimeTuning::pinned().unwrap()
}

fn carl(x: f32) -> HeroView {
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

fn donut(x: f32) -> HeroView {
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

fn slime_at(x: f32, facing: i8) -> Slime {
    Slime::new(&tuning(), x, 0.0, facing, x)
}

fn ctx<'a>(heroes: &'a [HeroView], target: Option<HeroId>, solids: &'a [Block]) -> SlimeCtx<'a> {
    SlimeCtx {
        heroes,
        in_view: true,
        target,
        solids,
    }
}

/// Step with the scene's token rule: grant whenever the slime wants one.
fn step_once(s: &mut Slime, heroes: &[HeroView], target: Option<HeroId>) -> SlimeStep {
    let t = tuning();
    let c = ctx(heroes, target, &[]);
    let wants = s.wants_token(&t, &c);
    s.step(&t, &c, wants, |_| false)
}

fn wall(left: f32, width: f32) -> Block {
    Block {
        rect: Rect {
            left,
            bottom: 0.0,
            width,
            height: 200.0,
        },
        material: Material::Stone,
        intact: true,
    }
}

// ------------------------------------------------------------------ data

#[test]
fn the_file_is_pinned_and_round_trips() {
    assert_eq!(content_hash(TUNING_JSON), TUNING_HASH);
    let t = tuning();
    let original: Value = serde_json::from_str(TUNING_JSON).unwrap();
    assert_eq!(serde_json::to_value(&t).unwrap(), original);
    let drifted = TUNING_JSON.replace("\"hazq04_slime_health\": 3", "\"hazq04_slime_health\": 4");
    assert_ne!(drifted, TUNING_JSON);
    match SlimeTuning::from_json_pinned(&drifted, TUNING_HASH) {
        Err(SlimeTuningError::HashMismatch { expected, .. }) => assert_eq!(expected, TUNING_HASH),
        other => panic!("a drifted file must fail the pin: {other:?}"),
    }
    assert!(SlimeTuning::from_json(&drifted).is_ok());
    assert_eq!(active(), &t);
}

#[test]
fn the_adopted_values_are_the_sheet_values() {
    let t = tuning();
    assert_eq!(t.hazq04_slime_health, 3);
    assert_eq!(
        (t.hazq06_sight_px, t.hazq06_creep_px_s, t.hazq06_leash_px),
        (160, 30, 96)
    );
    assert_eq!((t.hazq08_trigger_px, t.hazq08_standoff_px), (44, 40));
    assert_eq!(
        (t.hazq10_damage, t.hazq10_knock_x, t.hazq10_knock_y),
        (1, 120, 160)
    );
    assert_eq!(
        (t.hazq19_damage, t.hazq19_pop_x, t.hazq19_pop_y),
        (1, 180, 200)
    );
    assert_eq!((t.hazq12_kick_slide_px, t.hazq12_kick_slide_ticks), (24, 8));
    assert_eq!(t.hazq21_drain_ticks, 60);
    assert_eq!(t.hazq18_volume_height_px, 6);
    assert!(t.hazq11_gel && t.hazq13_body_contact && t.hazq15_lunge_evadable);
    assert!(t.hazq20_acid_hurts_launched);
    assert!(t.enabled() && t.on(true) && !t.on(false));
    let mut off = t.clone();
    off.hazq02_slime_enabled = false;
    assert!(!off.on(true), "the kill switch turns every switch off");
}

fn field_of(text: &str) -> &'static str {
    match SlimeTuning::from_json(text) {
        Err(SlimeTuningError::Field { field, .. }) => field,
        other => panic!("expected a field error, got {other:?}"),
    }
}

#[test]
fn a_bad_value_names_its_field() {
    let cases: [(&str, &str, &str); 12] = [
        (
            "\"hazq04_slime_health\": 3",
            "\"hazq04_slime_health\": 0",
            "hazq04_slime_health",
        ),
        (
            "\"hazq08_standoff_px\": 40",
            "\"hazq08_standoff_px\": 50",
            "hazq08_standoff_px",
        ),
        (
            "\"hazq06_creep_px_s\": 30",
            "\"hazq06_creep_px_s\": 0",
            "hazq06_creep_px_s",
        ),
        (
            "\"hazq06_creep_px_s\": 30",
            "\"hazq06_creep_px_s\": 240",
            "hazq06_creep_px_s",
        ),
        // The first dangerous tick must fall 24..=36 ticks after the tell starts.
        (
            "\"hazq09_tell_hold_ticks\": 24",
            "\"hazq09_tell_hold_ticks\": 10",
            "hazq09_tell_rise_ticks",
        ),
        (
            "\"hazq09_tell_hold_ticks\": 24",
            "\"hazq09_tell_hold_ticks\": 40",
            "hazq09_tell_rise_ticks",
        ),
        (
            "\"hazq09_cooldown_ticks\": 48",
            "\"hazq09_cooldown_ticks\": 0",
            "hazq09_cooldown_ticks",
        ),
        (
            "\"hazq12_kick_slide_ticks\": 8",
            "\"hazq12_kick_slide_ticks\": 0",
            "hazq12_kick_slide_ticks",
        ),
        (
            "\"hazq18_width_min_px\": 32",
            "\"hazq18_width_min_px\": 90",
            "hazq18_width_min_px",
        ),
        // The pop must clear the widest patch inside the hurt lock.
        (
            "\"hazq19_pop_x\": 180",
            "\"hazq19_pop_x\": 120",
            "hazq19_pop_x",
        ),
        (
            "\"hazq21_drain_ticks\": 60",
            "\"hazq21_drain_ticks\": 0",
            "hazq21_drain_ticks",
        ),
        (
            "\"hazq05_lunge_box\": { \"x0\": 18, \"x1\": 39,",
            "\"hazq05_lunge_box\": { \"x0\": 39, \"x1\": 18,",
            "hazq05_lunge_box",
        ),
    ];
    for (from, to, field) in cases {
        assert!(TUNING_JSON.contains(from), "{from}");
        assert_eq!(field_of(&TUNING_JSON.replace(from, to)), field, "{to}");
    }
}

#[test]
fn unknown_and_missing_fields_are_parse_errors() {
    let mut v: Value = serde_json::from_str(TUNING_JSON).unwrap();
    v["hazq99_extra"] = serde_json::json!(1);
    assert!(matches!(
        SlimeTuning::from_json(&v.to_string()),
        Err(SlimeTuningError::Parse(_))
    ));
    let mut v: Value = serde_json::from_str(TUNING_JSON).unwrap();
    v.as_object_mut().unwrap().remove("hazq04_slime_health");
    assert!(matches!(
        SlimeTuning::from_json(&v.to_string()),
        Err(SlimeTuningError::Parse(_))
    ));
}

#[test]
fn the_slime_keys_are_named_after_a_question() {
    let v: Value = serde_json::from_str(TUNING_JSON).unwrap();
    for key in v.as_object().unwrap().keys() {
        let named = key.starts_with("hazq")
            && key[4..6].parse::<u8>().is_ok_and(|q| (1..=50).contains(&q))
            && key.as_bytes()[6] == b'_';
        assert!(named || key == "schema_version", "{key}");
    }
}

#[test]
fn the_sources_are_distinct_and_carry_the_instance() {
    assert_ne!(SLIME_SOURCE_ID, ACID_SOURCE_ID);
    assert_ne!(SLIME_SOURCE_ID, crate::boss::BOSS_SOURCE_ID);
    assert_ne!(SLIME_SOURCE_ID, crate::boss::RACK_SOURCE_ID);
    assert_ne!(ACID_SOURCE_ID, crate::cf02::arena::SHORE_BLOCK_SOURCE);
    assert_eq!(slime_source(0) & 0xffff, SLIME_SOURCE_ID);
    assert_eq!(slime_source(3) >> 16, 3);
    assert_eq!(acid_source(7) >> 16, 7);
    assert_ne!(acid_source(1), acid_source(2));
}

// -------------------------------------------------------------- timeline

#[test]
fn the_timeline_is_the_sheet_timeline() {
    let t = tuning();
    assert_eq!(t.tell_len(), 30);
    assert_eq!(t.launch_start(), 30);
    assert_eq!(t.impact_start(), 32, "the first harm is on attack tick 32");
    assert_eq!(t.recover_start(), 38);
    assert_eq!(t.cooldown_start(), 58);
    assert_eq!(t.total_ticks(), 106);
    assert_eq!(t.punish_window(), 68, "recover plus cooldown");
    for (tick, phase) in [
        (0, SlimePhase::TellRise),
        (5, SlimePhase::TellRise),
        (6, SlimePhase::TellHold),
        (29, SlimePhase::TellHold),
        (30, SlimePhase::Launch),
        (31, SlimePhase::Launch),
        (32, SlimePhase::Impact),
        (37, SlimePhase::Impact),
        (38, SlimePhase::Recover),
        (57, SlimePhase::Recover),
        (58, SlimePhase::Cooldown),
        (105, SlimePhase::Cooldown),
    ] {
        assert_eq!(phase_at(&t, tick), Some(phase), "attack tick {tick}");
    }
    assert_eq!(phase_at(&t, 106), None);
}

#[test]
fn the_stepped_timeline_runs_the_phases_on_their_ticks() {
    let t = tuning();
    let mut s = slime_at(400.0, 1);
    let heroes = [carl(444.0)];
    // Tick 0: the tell starts on this tick.
    let out = step_once(&mut s, &heroes, Some(HeroId::Carl));
    assert!(out.tell_started);
    assert_eq!(s.attack_tick(), Some(0));
    let mut seen: Vec<(u16, SlimePhase)> = vec![(0, s.phase(&t))];
    let mut hurt_tick = None;
    let mut ended = None;
    for tick in 1..130u16 {
        let out = step_once(&mut s, &heroes, Some(HeroId::Carl));
        assert!(!out.tell_started, "never a second tell, tick {tick}");
        if out.hits.iter().any(|h| h.kind == SlimeHurt::Lunge) {
            hurt_tick = Some(tick);
        }
        match s.attack_tick() {
            Some(a) => {
                assert_eq!(a, tick);
                seen.push((a, s.phase(&t)));
            }
            None => {
                ended = Some(tick);
                break;
            }
        }
    }
    assert_eq!(hurt_tick, Some(32), "the lunge hurts on attack tick 32");
    assert_eq!(ended, Some(106));
    let first = |p: SlimePhase| seen.iter().find(|(_, q)| *q == p).unwrap().0;
    assert_eq!(first(SlimePhase::TellRise), 0);
    assert_eq!(first(SlimePhase::TellHold), 6);
    assert_eq!(first(SlimePhase::Launch), 30);
    assert_eq!(first(SlimePhase::Impact), 32);
    assert_eq!(first(SlimePhase::Recover), 38);
    assert_eq!(first(SlimePhase::Cooldown), 58);
}

#[test]
fn the_lunge_hits_an_idle_carl_with_the_adopted_blow() {
    let t = tuning();
    let mut s = slime_at(400.0, 1);
    let mut heroes = vec![carl(444.0)];
    let mut hits = Vec::new();
    step_once(&mut s, &heroes, Some(HeroId::Carl));
    for _ in 0..40 {
        let out = step_once(&mut s, &heroes, Some(HeroId::Carl));
        for h in out.hits {
            hits.push((s.attack_tick().unwrap(), h));
            heroes[0].vulnerable = false;
        }
    }
    assert_eq!(hits.len(), 1, "one hurt for the one lunge: {hits:?}");
    let (tick, hit) = hits[0];
    assert_eq!(tick, 32);
    assert_eq!(hit.hero, HeroId::Carl);
    assert_eq!(hit.kind, SlimeHurt::Lunge);
    assert_eq!(hit.blow.damage, 1);
    assert_eq!(hit.blow.knock_vx, 120.0, "signed away from the slime");
    assert_eq!(hit.blow.knock_vy, 160.0);
    assert_eq!(hit.blow.from_x, 400.0);
    assert_eq!(hit.blow.source, slime_source(0));
    // A hero behind the slime is knocked the other way.
    let behind = carl(380.0);
    assert_eq!(s.lunge_blow(&t, &behind).knock_vx, -120.0);
}

// ---------------------------------------------------------- reach tables

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.overlaps(b)
}

#[test]
fn the_lunge_reach_table() {
    let t = tuning();
    let mut s = slime_at(0.0, 1);
    // Drive to the first impact tick (attack tick 32).
    s.attack = Some(32);
    let lunge = s.lunge_box(&t).expect("live on the impact tick");
    assert_eq!((lunge.left, lunge.right()), (18.0, 39.0));
    assert_eq!((lunge.bottom, lunge.top()), (0.0, 15.0));
    let hits = |hero: HeroView| rects_overlap(lunge, hero.hurt);
    // Carl: `6 < d < 51`.
    assert!(hits(carl(50.99)));
    assert!(!hits(carl(51.0)));
    assert!(hits(carl(6.01)));
    assert!(!hits(carl(6.0)));
    // Donut: `8 < d < 49`.
    assert!(hits(donut(48.99)));
    assert!(!hits(donut(49.0)));
    assert!(hits(donut(8.01)));
    assert!(!hits(donut(8.0)));
    // The lunge box is live on the impact ticks only.
    for tick in (0..32).chain(38..106) {
        s.attack = Some(tick);
        assert!(s.lunge_box(&t).is_none(), "attack tick {tick}");
    }
    // Mirrored about the pivot column.
    let mut back = slime_at(0.0, -1);
    back.attack = Some(34);
    let l = back.lunge_box(&t).unwrap();
    assert_eq!((l.left, l.right()), (-39.0, -18.0));
}

#[test]
fn the_body_contact_reach_table() {
    let t = tuning();
    let mut s = slime_at(0.0, 1);
    let standing = |s: &Slime| s.contact_box(&t).unwrap();
    // Standing box (idle, creep, recover, cooldown): Carl `d < 35`, Donut `d < 33`.
    let b = standing(&s);
    assert_eq!((b.left, b.right(), b.top()), (-28.0, 23.0, 24.0));
    assert!(b.overlaps(carl(34.99).hurt));
    assert!(!b.overlaps(carl(35.0).hurt));
    assert!(b.overlaps(donut(32.99).hurt));
    assert!(!b.overlaps(donut(33.0).hurt));
    // Gathered box (tell, launch, impact): Carl `d < 16`, Donut `d < 14`.
    s.attack = Some(10);
    let g = s.contact_box(&t).unwrap();
    assert_eq!((g.left, g.right(), g.top()), (-25.0, 4.0, 24.0));
    assert!(g.overlaps(carl(15.99).hurt));
    assert!(!g.overlaps(carl(16.0).hurt));
    assert!(g.overlaps(donut(13.99).hurt));
    assert!(!g.overlaps(donut(14.0).hurt));
}

#[test]
fn the_gathered_contact_box_lasts_through_the_impact() {
    let t = tuning();
    let mut s = slime_at(0.0, 1);
    for tick in 0..106u16 {
        s.attack = Some(tick);
        let b = s.contact_box(&t).unwrap();
        let gathered = tick < 38;
        assert_eq!(
            b.right(),
            if gathered { 4.0 } else { 23.0 },
            "attack tick {tick}"
        );
    }
    // No attack: the standing box.
    s.attack = None;
    assert_eq!(s.contact_box(&t).unwrap().right(), 23.0);
}

#[test]
fn the_hurtbox_pulls_back_in_the_tell_and_launch_only() {
    let t = tuning();
    let mut s = slime_at(0.0, 1);
    s.attack = None;
    let idle = s.hurtbox(&t).unwrap();
    assert_eq!((idle.left, idle.right(), idle.top()), (-32.0, 28.0, 32.0));
    for tick in 0..106u16 {
        s.attack = Some(tick);
        let b = s.hurtbox(&t).unwrap();
        if tick < 32 {
            assert_eq!(
                (b.left, b.right(), b.top()),
                (-25.0, 13.0, 37.0),
                "attack tick {tick}"
            );
        } else {
            assert_eq!(b, idle, "attack tick {tick}");
        }
    }
}

#[test]
fn the_melee_reach_table_against_the_hurtboxes() {
    let t = tuning();
    // Carl's boxes face the slime from the +x side; the slime faces +x. A jab,
    // haymaker or push kick box has its front 22 px ahead of the pivot, a swat 30.
    let melee = |from_x: f32, toward: f32, front: f32| Rect {
        left: if toward < 0.0 { from_x - front } else { from_x },
        bottom: 29.0,
        width: front,
        height: 6.0,
    };
    let mut s = slime_at(0.0, 1);
    let idle = s.hurtbox(&t).unwrap();
    s.attack = Some(10);
    let tell = s.hurtbox(&t).unwrap();
    // Reaches the idle hurtbox for d < 50, the tell hurtbox for d < 35.
    assert!(melee(49.99, -1.0, 22.0).overlaps(idle));
    assert!(!melee(50.0, -1.0, 22.0).overlaps(idle));
    assert!(melee(34.99, -1.0, 22.0).overlaps(tell));
    assert!(!melee(35.0, -1.0, 22.0).overlaps(tell));
    // From behind (the back edge is -32): d < 54.
    assert!(melee(-53.99, 1.0, 22.0).overlaps(idle));
    assert!(!melee(-54.0, 1.0, 22.0).overlaps(idle));
    // Donut's swat (front 30): the idle hurtbox d < 58, the tell hurtbox d < 43.
    assert!(melee(57.99, -1.0, 30.0).overlaps(idle));
    assert!(!melee(58.0, -1.0, 30.0).overlaps(idle));
    assert!(melee(42.99, -1.0, 30.0).overlaps(tell));
    assert!(!melee(43.0, -1.0, 30.0).overlaps(tell));
    // The consequence by design: Carl cannot jab the idle slime from outside its
    // lunge reach (50 < 51) and cannot touch the reared body from outside it.
    // The +2 px idle height: the jab (heights 29..35) and haymaker (27..38)
    // connect on a mound that art draws 30 px tall.
    assert_eq!(idle.top(), 32.0);
}

// ----------------------------------------------------------- jump and hop

/// Feet height after `n` physics ticks of a jump launched at `launch` (tick 1 is
/// the press tick), in the scene's order: velocity set, then gravity, then move.
fn feet_after(n: u32, launch: f32) -> f32 {
    let (mut y, mut vy) = (0.0_f32, launch);
    for _ in 0..n {
        vy -= GRAVITY * TICK_SECONDS;
        y += vy * TICK_SECONDS;
        if y <= 0.0 {
            return 0.0;
        }
    }
    y
}

/// Whether a jump of `launch` pressed on attack tick `press` keeps the feet at
/// or above the lunge box's 15 px on every impact tick.
fn clears_lunge(t: &SlimeTuning, press: i32, launch: f32) -> bool {
    let top = f32::from(t.hazq05_lunge_box.y1);
    (t.impact_start()..t.recover_start()).all(|a| {
        let n = i32::from(a) - press + 1;
        n >= 1 && feet_after(n as u32, launch) >= top
    })
}

#[test]
fn the_plain_jump_window_edge_is_pinned() {
    let t = tuning();
    // Feet on the press tick and the two after: 7.04, 13.79, 20.25 px.
    assert!((feet_after(1, JUMP_SPEED) - 7.04).abs() < 0.01);
    assert!((feet_after(2, JUMP_SPEED) - 13.79).abs() < 0.01);
    assert!((feet_after(3, JUMP_SPEED) - 20.25).abs() < 0.01);
    // Pinned boundary: the last press tick whose feet are above 15 px on every
    // impact tick. The sheet predicts 29 (its arithmetic counts the press tick
    // as tick 0); counting the press tick as the first physics tick, as the
    // scene and the Engineer's `eng_jump_window_edges` do, the boundary is 30.
    let last = (0..64)
        .rev()
        .find(|&p| clears_lunge(&t, p, JUMP_SPEED))
        .unwrap();
    assert_eq!(last, 30, "the last safe press tick");
    assert!(clears_lunge(&t, 30, JUMP_SPEED));
    assert!(!clears_lunge(&t, 31, JUMP_SPEED));
    // About 20 px on the first impact tick for the last safe press.
    assert!((feet_after(3, JUMP_SPEED) - 20.0).abs() < 0.5);
    // A jump lasts 50 ticks: the landing is in the cooldown, not the recover.
    let air = crate::agility::air_ticks(JUMP_SPEED) as i32;
    assert_eq!(air, 50);
    for press in 12..=30 {
        let landing = press + air;
        assert!(
            landing >= i32::from(t.cooldown_start()),
            "press {press} lands on attack tick {landing}"
        );
    }
}

#[test]
fn the_bail_out_hop_clears_the_lunge_too() {
    let t = tuning();
    let hop = f32::from(crate::cf02::active().q54_bailout_vy);
    let apex = (1..60).map(|n| feet_after(n, hop)).fold(0.0_f32, f32::max);
    // The sheet's 32 px is the continuous 260^2 / 2g; the discrete arc is 30.04.
    assert!((apex - 30.04).abs() < 0.05, "apex {apex}");
    let windows: Vec<i32> = (0..40).filter(|&p| clears_lunge(&t, p, hop)).collect();
    assert!(!windows.is_empty(), "the hop answers the slime");
    assert!(windows.contains(&20));
}

#[test]
fn a_jump_over_the_body_lands_behind_it_during_the_cooldown() {
    let t = tuning();
    // The standing contact box is 24 px tall: a hero whose feet are at 24 or
    // more clears the body; the jump is above 24 px from tick 3 to tick 47.
    let top = f32::from(t.hazq05_contact_standing.y1);
    let above: Vec<u32> = (1..=50)
        .filter(|&n| feet_after(n, JUMP_SPEED) >= top)
        .collect();
    assert_eq!((above[0], *above.last().unwrap()), (4, 45));
    // The 15 px lunge box lies below it, forward along the locked facing.
    assert!(f32::from(t.hazq05_lunge_box.y1) < top);
}

// ------------------------------------------------------------- the brain

#[test]
fn it_wakes_only_in_view_and_within_sight() {
    let t = tuning();
    let s = slime_at(400.0, 1);
    let near = [carl(560.0)];
    // Carl's hurtbox starts at 548: 148 px from the pivot.
    assert!(s.awake(&t, &ctx(&near, Some(HeroId::Carl), &[])));
    let far = [carl(580.0)];
    assert!(!s.awake(&t, &ctx(&far, Some(HeroId::Carl), &[])));
    let mut off = ctx(&near, Some(HeroId::Carl), &[]);
    off.in_view = false;
    assert!(!s.awake(&t, &off), "never wakes off-screen");
    let gone = [HeroView {
        present: false,
        ..carl(420.0)
    }];
    assert!(!s.awake(&t, &ctx(&gone, None, &[])));
}

#[test]
fn eligibility_is_feet_near_its_floor_line() {
    let t = tuning();
    let s = slime_at(400.0, 1);
    let mut h = carl(430.0);
    assert!(s.eligible(&t, &h));
    h.foot_y = 8.0;
    assert!(s.eligible(&t, &h), "8 px is the tolerance");
    h.foot_y = 8.5;
    assert!(!s.eligible(&t, &h), "a jumping hero is not a target");
    h.foot_y = 40.0;
    assert!(!s.eligible(&t, &h), "a hero on a ledge is not a target");
    h.foot_y = 0.0;
    h.present = false;
    assert!(!s.eligible(&t, &h));
}

#[test]
fn the_token_candidate_rule() {
    let t = tuning();
    let s = slime_at(400.0, 1);
    let want = |h: HeroView| s.wants_token(&t, &ctx(&[h], Some(h.id), &[]));
    assert!(want(carl(444.0)), "d <= 44");
    assert!(!want(carl(444.5)), "d > 44");
    assert!(want(carl(356.0)), "either side");
    let mut jumping = carl(430.0);
    jumping.foot_y = 20.0;
    assert!(!want(jumping), "an ineligible target takes no token");
    // No target at all.
    assert!(!s.wants_token(&t, &ctx(&[carl(430.0)], None, &[])));
    // Never during an attack, cooldown included.
    let mut busy = slime_at(400.0, 1);
    for tick in 0..106 {
        busy.attack = Some(tick);
        assert!(!busy.wants_token(&t, &ctx(&[carl(430.0)], Some(HeroId::Carl), &[])));
    }
    // A defeated slime wants nothing.
    let mut dead = slime_at(400.0, 1);
    dead.hit(&t, 9);
    assert!(!dead.wants_token(&t, &ctx(&[carl(430.0)], Some(HeroId::Carl), &[])));
}

#[test]
fn it_creeps_to_the_standoff_never_backward_and_within_the_leash() {
    let t = tuning();
    let mut s = Slime::new(&t, 500.0, 0.0, -1, 500.0);
    let heroes = [carl(440.0)];
    let mut x_prev = s.x;
    for _ in 0..20 {
        // Carl stands off the token distance (60 px): the slime creeps in.
        step_once(&mut s, &heroes, Some(HeroId::Carl));
        assert_eq!(s.x, x_prev - 0.5, "30 px/s is half a pixel per tick");
        x_prev = s.x;
    }
    // It stops at the standoff (40 px from the target on its side), not before.
    for _ in 0..400 {
        step_once(&mut s, &heroes, Some(HeroId::Carl));
        if s.attack_tick().is_some() {
            break;
        }
    }
    // Within the trigger it tells; the point is it never moved backward or closer
    // than the standoff before telling.
    assert!(s.x >= 440.0 + 40.0 - 0.001, "{}", s.x);

    // Never backward: a hero closer than the standoff does not push it away.
    let mut s = slime_at(400.0, -1);
    let before = s.x;
    for _ in 0..10 {
        let mut c = [carl(430.0)];
        c[0].vulnerable = false;
        let cx = ctx(&c, Some(HeroId::Carl), &[]);
        s.step(&t, &cx, false, |_| false);
    }
    assert_eq!(s.x, before, "at 30 px it holds: the standoff is 40");

    // The leash: never more than 96 px from home.
    let mut s = Slime::new(&t, 400.0, 0.0, -1, 400.0);
    let far = [carl(100.0)];
    let mut cx = ctx(&far, Some(HeroId::Carl), &[]);
    cx.in_view = true;
    // 300 px away: out of sight, it does not creep at all.
    for _ in 0..50 {
        s.step(&t, &cx, false, |_| false);
    }
    assert_eq!(s.x, 400.0);
    // Carl at 262: 126 px from the pivot (awake); the standoff point 302 lies
    // beyond the 96 px leash, so it stops at 304.
    let mid = [carl(262.0)];
    for _ in 0..600 {
        let cx = ctx(&mid, Some(HeroId::Carl), &[]);
        s.step(&t, &cx, false, |_| false);
    }
    assert_eq!(s.x, 400.0 - 96.0, "stopped at the leash");
}

#[test]
fn a_solid_stops_the_creep() {
    let t = tuning();
    let blocks = [wall(380.0, 16.0)];
    let mut s = slime_at(470.0, -1);
    let heroes = [carl(300.0)];
    let mut c = ctx(&heroes, Some(HeroId::Carl), &blocks);
    // 170 px: out of sight; bring the hero near enough to wake it.
    let near = [carl(340.0)];
    c.heroes = &near;
    for _ in 0..300 {
        s.step(&t, &c, false, |_| false);
    }
    // The standing contact box's left edge is x - 28: it stops against the wall.
    assert!(s.x - 23.0 >= 396.0 - 0.5, "{}", s.x);
}

#[test]
fn the_target_and_facing_lock_from_the_tell_start_and_turn_at_the_cooldown() {
    let t = tuning();
    let mut s = slime_at(400.0, 1);
    let mut heroes = vec![carl(444.0)];
    step_once(&mut s, &heroes, Some(HeroId::Carl));
    assert_eq!(s.locked_target(), Some(HeroId::Carl));
    assert_eq!(s.facing, 1);
    // Carl jumps over it and ends up behind it before the recover ends.
    heroes[0].vulnerable = false;
    let mut turned_at = None;
    for _ in 0..70 {
        let a = s.attack_tick().unwrap() + 1;
        if a >= 20 {
            heroes[0] = HeroView {
                vulnerable: false,
                ..carl(340.0)
            };
        }
        step_once(&mut s, &heroes, Some(HeroId::Carl));
        if s.facing == -1 && turned_at.is_none() {
            turned_at = s.attack_tick();
        }
        if a < 58 {
            assert_eq!(
                s.facing, 1,
                "facing is locked through the recover (tick {a})"
            );
            assert_eq!(s.locked_target(), Some(HeroId::Carl));
        }
    }
    assert_eq!(
        turned_at,
        Some(t.cooldown_start()),
        "turns on attack tick 58"
    );
}

#[test]
fn with_no_eligible_hero_it_holds_facing_the_nearest_hero() {
    let t = tuning();
    let mut s = slime_at(400.0, 1);
    let mut jumper = carl(380.0);
    jumper.foot_y = 40.0;
    let heroes = [jumper];
    let c = ctx(&heroes, None, &[]);
    let out = s.step(&t, &c, false, |_| false);
    assert!(!out.tell_started);
    assert_eq!(s.facing, -1, "it faces the nearest hero");
    assert_eq!(s.x, 400.0, "and takes no token and does not creep");
}

// -------------------------------------------------------- gel and kicks

#[test]
fn gel_every_hit_leaves_the_state_and_timer_unchanged() {
    let t = tuning();
    for tick in [
        None,
        Some(0),
        Some(5),
        Some(20),
        Some(31),
        Some(33),
        Some(45),
        Some(70),
    ] {
        let mut s = slime_at(400.0, 1);
        s.attack = tick;
        s.locked = Some(HeroId::Carl);
        let before = (
            s.attack_tick(),
            s.phase(&t),
            s.facing,
            s.x,
            s.locked_target(),
        );
        let out = s.hit(&t, 1);
        assert_eq!(s.health(), 2, "damage lands at {tick:?}");
        assert!(out.flash && !out.defeated && !out.interrupted);
        assert_eq!(s.flash_left(), 2);
        assert_eq!(
            (
                s.attack_tick(),
                s.phase(&t),
                s.facing,
                s.x,
                s.locked_target()
            ),
            before,
            "a gel hit changes nothing at {tick:?}"
        );
        // The hurt frame plays only when the hit lands in idle.
        assert_eq!(out.hurt_frame, tick.is_none(), "{tick:?}");
        assert_eq!(s.hurt_frame_left(), if tick.is_none() { 12 } else { 0 });
    }
}

#[test]
fn gel_a_tell_cannot_be_jabbed_out_and_the_lunge_still_hits() {
    let t = tuning();
    let mut s = slime_at(400.0, 1);
    let heroes = [carl(430.0)];
    step_once(&mut s, &heroes, Some(HeroId::Carl));
    let mut hit_tick = None;
    for tick in 1..40u16 {
        if tick == 4 || tick == 20 {
            let out = s.hit(&t, 1);
            assert!(!out.defeated);
        }
        let out = step_once(&mut s, &heroes, Some(HeroId::Carl));
        if out.hits.iter().any(|h| h.kind == SlimeHurt::Lunge) {
            hit_tick = Some(tick);
            break;
        }
    }
    assert_eq!(s.health(), 1, "both hits damage it");
    assert_eq!(hit_tick, Some(32));
}

#[test]
fn a_lethal_hit_defeats_it_in_place_and_leaves_an_inert_remains() {
    let t = tuning();
    let mut s = slime_at(400.0, 1);
    s.attack = Some(10);
    assert!(!s.hit(&t, 1).defeated);
    assert!(!s.hit(&t, 1).defeated);
    let out = s.hit(&t, 1);
    assert!(out.defeated && out.flash && !out.hurt_frame);
    assert!(s.is_defeated());
    assert_eq!(s.x, 400.0, "in place, no knock arc");
    assert_eq!(s.attack_tick(), None);
    // Inert: no hurtbox, no contact, no lunge, no step.
    assert!(s.hurtbox(&t).is_none());
    assert!(s.contact_box(&t).is_none());
    assert!(s.lunge_box(&t).is_none());
    let heroes = [carl(410.0)];
    for _ in 0..200 {
        let out = step_once(&mut s, &heroes, Some(HeroId::Carl));
        assert_eq!(
            out,
            SlimeStep::default(),
            "walking through the remains does nothing"
        );
    }
    // A further hit does nothing and never defeats twice.
    let again = s.hit(&t, 5);
    assert!(!again.defeated && !again.flash);
    assert_eq!(s.health(), 0);
}

#[test]
fn with_the_gel_switch_off_a_hit_interrupts_the_attack() {
    let mut t = tuning();
    t.hazq11_gel = false;
    let mut s = Slime::new(&t, 400.0, 0.0, 1, 400.0);
    s.attack = Some(10);
    let out = s.hit(&t, 1);
    assert!(out.interrupted && out.hurt_frame);
    assert_eq!(s.attack_tick(), None);
}

#[test]
fn the_push_kick_slides_24_px_over_8_ticks_away_from_the_kicker() {
    let t = tuning();
    let mut s = slime_at(400.0, -1);
    let heroes = [carl(440.0)];
    s.attack = Some(10);
    s.push_kick(&t, 440.0);
    assert!(s.sliding());
    // It slides in the tell, away from the kicker (toward -x), 3 px per tick.
    for k in 1..=8 {
        let c = ctx(&heroes, Some(HeroId::Carl), &[]);
        s.step(&t, &c, false, |_| false);
        assert_eq!(s.x, 400.0 - 3.0 * k as f32, "tick {k}");
    }
    assert!(!s.sliding());
    assert_eq!(s.x, 376.0);
    // The attack clock kept running through the slide: the timer never changed.
    assert_eq!(s.attack_tick(), Some(18));
}

#[test]
fn the_slide_is_stopped_by_a_solid_and_by_the_leash() {
    let t = tuning();
    let heroes = [carl(500.0)];
    // A solid: the wall's face at 360; the standing box's left edge is x - 28
    // after the move, so the slime stops before it overlaps.
    let blocks = [wall(340.0, 20.0)];
    let mut s = slime_at(400.0, -1);
    s.push_kick(&t, 440.0);
    for _ in 0..8 {
        let c = ctx(&heroes, Some(HeroId::Carl), &blocks);
        s.step(&t, &c, false, |_| false);
    }
    assert!(s.x - 28.0 >= 360.0, "{}", s.x);
    assert!(s.x < 400.0 && !s.sliding());
    // The leash: home 400, leash 96: a kick at 484 would carry it past 304.
    let mut s = Slime::new(&t, 310.0, 0.0, -1, 400.0);
    s.push_kick(&t, 440.0);
    for _ in 0..8 {
        let c = ctx(&heroes, Some(HeroId::Carl), &[]);
        s.step(&t, &c, false, |_| false);
    }
    assert!(s.x >= 304.0, "never past the leash: {}", s.x);
    // Kicked while the kicker stands on the other side: the other direction.
    let mut s = slime_at(400.0, 1);
    s.push_kick(&t, 360.0);
    let c = ctx(&heroes, Some(HeroId::Carl), &[]);
    s.step(&t, &c, false, |_| false);
    assert_eq!(s.x, 403.0);
}

#[test]
fn a_kick_by_attack_tick_24_whiffs_the_lunge() {
    // `haz_slime_kick_slide` in the domain: Carl at 40 px, slime kicked on
    // attack tick 6 and it ends 24 px away: 64 px, past the lunge reach (51).
    let t = tuning();
    let mut s = slime_at(400.0, 1);
    let mut heroes = vec![carl(440.0)];
    step_once(&mut s, &heroes, Some(HeroId::Carl));
    let mut hurt = false;
    for tick in 1..50u16 {
        if tick == 6 {
            s.push_kick(&t, 440.0);
        }
        let c = ctx(&heroes, Some(HeroId::Carl), &[]);
        let out = s.step(&t, &c, false, |_| false);
        hurt |= !out.hits.is_empty();
        if s.attack_tick() == Some(34) {
            // Impact: the slime is 24 px back; d = 64 >= 51.
            assert_eq!(s.x, 376.0);
            assert!(heroes[0].x - s.x >= 51.0);
        }
        heroes[0].vulnerable = true;
    }
    assert!(!hurt, "the lunge whiffs");
}

// ------------------------------------------------- the impact tick order

fn impact_slime() -> Slime {
    let t = tuning();
    let mut s = slime_at(0.0, 1);
    s.attack = Some(t.impact_start());
    s
}

#[test]
fn an_invulnerable_hero_is_skipped_by_the_lunge() {
    let t = tuning();
    let mut s = impact_slime();
    let mut h = carl(40.0);
    h.vulnerable = false;
    let out = s.step(&t, &ctx(&[h], Some(HeroId::Carl), &[]), false, |_| false);
    assert!(out.hits.is_empty());
}

#[test]
fn one_hero_per_lunge_closer_edge_first_ties_to_carl() {
    let t = tuning();
    // Both overlap the lunge box [18, 39): Donut's near edge (2) is closer than
    // Carl's (18): Donut is hit, Carl is not, and the lunge stops testing.
    let mut s = impact_slime();
    let heroes = [carl(30.0), donut(12.0)];
    let out = s.step(&t, &ctx(&heroes, Some(HeroId::Carl), &[]), false, |_| false);
    let lunges: Vec<_> = out
        .hits
        .iter()
        .filter(|h| h.kind == SlimeHurt::Lunge)
        .collect();
    assert_eq!(lunges.len(), 1);
    assert_eq!(lunges[0].hero, HeroId::Donut);
    // Next tick of the same lunge: nobody else is hit by it.
    let out = s.step(&t, &ctx(&heroes, Some(HeroId::Carl), &[]), false, |_| false);
    assert!(out.hits.iter().all(|h| h.kind != SlimeHurt::Lunge));

    // Equal near edges: the tie goes to Carl.
    let mut s = impact_slime();
    let heroes = [
        HeroView {
            hurt: Rect {
                left: 20.0,
                bottom: 0.0,
                width: 24.0,
                height: 24.0,
            },
            ..donut(32.0)
        },
        carl(32.0),
    ];
    let out = s.step(&t, &ctx(&heroes, Some(HeroId::Carl), &[]), false, |_| false);
    let lunge = out
        .hits
        .iter()
        .find(|h| h.kind == SlimeHurt::Lunge)
        .unwrap();
    assert_eq!(lunge.hero, HeroId::Carl);
}

#[test]
fn the_reflex_callback_skips_an_evaded_hero_and_carl_behind_is_chosen() {
    let t = tuning();
    let mut s = impact_slime();
    // Donut at 12 (hurtbox [2, 22), body-contact range of the gathered box) and
    // Carl at 30 on the same tick.
    let heroes = [carl(30.0), donut(12.0)];
    let mut asked = Vec::new();
    let out = s.step(&t, &ctx(&heroes, Some(HeroId::Carl), &[]), false, |h| {
        asked.push(h);
        h == HeroId::Donut
    });
    assert_eq!(
        asked,
        [HeroId::Donut, HeroId::Carl],
        "Donut first (closer edge), then Carl, who is not evaded and is hit"
    );
    assert_eq!(out.evaded, [HeroId::Donut]);
    let hits: Vec<_> = out.hits.iter().map(|h| (h.hero, h.kind)).collect();
    assert_eq!(hits, [(HeroId::Carl, SlimeHurt::Lunge)]);
    assert!(
        out.hits.iter().all(|h| h.hero != HeroId::Donut),
        "an evade is never voided by the body on the same tick"
    );
    // The exemption lasts the lunge's impact ticks: still no body burn next tick.
    let out = s.step(&t, &ctx(&heroes, Some(HeroId::Carl), &[]), false, |_| true);
    assert!(out.hits.is_empty() && out.evaded.is_empty());
}

#[test]
fn the_evade_callback_is_not_asked_with_the_switch_off() {
    let mut t = tuning();
    t.hazq15_lunge_evadable = false;
    let mut s = Slime::new(&t, 0.0, 0.0, 1, 0.0);
    s.attack = Some(t.impact_start());
    let heroes = [donut(30.0)];
    let out = s.step(&t, &ctx(&heroes, Some(HeroId::Donut), &[]), false, |_| {
        panic!("the lunge is not evadable with the switch off")
    });
    assert_eq!(out.hits.len(), 1);
}

#[test]
fn a_body_hurts_once_per_entry_with_a_pop_and_rearms_outside() {
    let t = tuning();
    let mut s = slime_at(400.0, 1);
    let mut hero = carl(430.0); // d = 30 < 35: inside the standing box
    let mut pops = Vec::new();
    // Not awake (out of view): the body still burns every tick.
    for tick in 0..10 {
        let mut c = ctx(std::slice::from_ref(&hero), None, &[]);
        c.in_view = false;
        let out = s.step(&t, &c, false, |_| false);
        for h in &out.hits {
            pops.push((tick, h.kind, h.blow));
        }
    }
    assert_eq!(pops.len(), 1, "one hurt for the entry: {pops:?}");
    assert_eq!(pops[0].0, 0, "on the first overlap tick");
    assert_eq!(pops[0].1, SlimeHurt::Body);
    let blow = pops[0].2;
    assert_eq!(
        (blow.damage, blow.knock_vx, blow.knock_vy),
        (1, 180.0, 200.0)
    );
    assert_eq!(blow.from_x, 400.0);
    // Standing in it after the hurt does nothing; leaving one tick rearms it.
    hero.x = 470.0;
    hero.hurt = carl(470.0).hurt;
    let mut c = ctx(std::slice::from_ref(&hero), None, &[]);
    c.in_view = false;
    assert!(s.step(&t, &c, false, |_| false).hits.is_empty());
    hero.x = 430.0;
    hero.hurt = carl(430.0).hurt;
    let mut c = ctx(std::slice::from_ref(&hero), None, &[]);
    c.in_view = false;
    let out = s.step(&t, &c, false, |_| false);
    assert_eq!(out.hits.len(), 1, "a second entry hurts again");
    // A hero on the other side is popped the other way.
    let behind = carl(370.0);
    assert_eq!(s.body_blow(&t, &behind).knock_vx, -180.0);
    // Exactly at the pivot pops toward +x.
    assert_eq!(s.body_blow(&t, &carl(400.0)).knock_vx, 180.0);
}

#[test]
fn an_invulnerable_body_entry_is_pending_until_it_ends_inside() {
    let t = tuning();
    let mut s = slime_at(400.0, 1);
    let mut hero = carl(430.0);
    hero.vulnerable = false;
    for _ in 0..20 {
        let c = ctx(std::slice::from_ref(&hero), None, &[]);
        assert!(s.step(&t, &c, false, |_| false).hits.is_empty());
    }
    hero.vulnerable = true;
    let c = ctx(std::slice::from_ref(&hero), None, &[]);
    assert_eq!(s.step(&t, &c, false, |_| false).hits.len(), 1);
}

#[test]
fn the_body_switch_turns_the_corrosive_body_off() {
    let mut t = tuning();
    t.hazq13_body_contact = false;
    let mut s = Slime::new(&t, 400.0, 0.0, 1, 400.0);
    assert!(s.contact_box(&t).is_none());
    let heroes = [carl(420.0)];
    let c = ctx(&heroes, None, &[]);
    assert!(s.step(&t, &c, false, |_| false).hits.is_empty());
}

#[test]
fn the_lunge_never_touches_a_hero_it_only_overlaps_while_airborne_above_it() {
    let t = tuning();
    let mut s = impact_slime();
    let mut h = carl(40.0);
    h.foot_y = 16.0;
    h.hurt.bottom = 16.0;
    let out = s.step(&t, &ctx(&[h], Some(HeroId::Carl), &[]), false, |_| false);
    assert!(
        out.hits.is_empty(),
        "feet at 16 px clear the 15 px lunge box"
    );
}

// --------------------------------------------------------------- determinism

fn hash_of(s: &Slime) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    s.mix_into(&mut |v| h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3));
    h
}

#[test]
fn the_same_inputs_give_the_same_hashes_tick_by_tick() {
    let run = || {
        let mut s = slime_at(400.0, 1);
        let mut heroes = vec![carl(444.0), donut(300.0)];
        let mut hashes = Vec::new();
        for tick in 0..260u32 {
            if tick == 50 {
                s.push_kick(&tuning(), 450.0);
            }
            if tick == 70 {
                heroes[0] = carl(420.0);
            }
            if tick % 41 == 0 {
                s.hit(&tuning(), 0);
            }
            step_once(&mut s, &heroes, Some(HeroId::Carl));
            hashes.push(hash_of(&s));
        }
        hashes
    };
    let a = run();
    assert_eq!(a, run());
    let unique: std::collections::BTreeSet<u64> = a.iter().copied().collect();
    assert!(unique.len() > 20, "the hash follows the state");
}

#[test]
fn the_hash_changes_with_every_field() {
    let t = tuning();
    let base = slime_at(400.0, 1);
    let mut seen = vec![hash_of(&base)];
    let mut push = |s: Slime| seen.push(hash_of(&s));
    let mut s = base.clone();
    s.x += 0.5;
    push(s);
    let mut s = base.clone();
    s.facing = -1;
    push(s);
    let mut s = base.clone();
    s.hit(&t, 1);
    push(s);
    let mut s = base.clone();
    s.attack = Some(5);
    push(s);
    let mut s = base.clone();
    s.push_kick(&t, 300.0);
    push(s);
    let mut s = base.clone();
    s.locked = Some(HeroId::Donut);
    push(s);
    let mut s = base.clone();
    s.lunge_done = true;
    push(s);
    let unique: std::collections::BTreeSet<u64> = seen.iter().copied().collect();
    assert_eq!(unique.len(), seen.len(), "{seen:?}");
}

#[test]
fn reset_returns_to_the_authored_start() {
    let t = tuning();
    let mut s = slime_at(400.0, -1);
    s.hit(&t, 2);
    s.attack = Some(40);
    s.push_kick(&t, 450.0);
    s.reset(&t, 400.0, -1);
    assert_eq!(hash_of(&s), hash_of(&slime_at(400.0, -1)));
}

// -------------------------------------------------------------- acid patch

fn patch_spec(id: u16, x0: i32, x1: i32, linked: Option<u8>) -> AcidPatchSpec {
    AcidPatchSpec {
        id,
        x0,
        x1,
        floor_y: 580,
        linked_slime: linked,
    }
}

fn patch(x0: i32, x1: i32, linked: Option<u8>) -> AcidPatch {
    AcidPatch::new(&tuning(), patch_spec(1, x0, x1, linked)).unwrap()
}

/// A hero hurtbox in layout px (y down) standing on the floor line 580.
fn body(x: i32, half: i32, h: i32) -> IRect {
    IRect::new(x - half, 580 - h, 2 * half, h)
}

#[test]
fn a_patch_is_a_persistent_acid_volume_six_px_tall() {
    let p = patch(248, 312, Some(0));
    let r = p.rect().unwrap();
    assert_eq!(r, IRect::new(248, 574, 64, 6));
    assert_eq!(p.bounds(), Some((248, 312)));
    let feed = p.nav_rect().unwrap();
    assert_eq!(feed.class, HazardClass::Acid);
    assert_eq!(feed.ticks_left, None, "persistent: no telegraph, no clock");
    assert!(feed.hurts_heroes && feed.hurts_enemies);
    assert!(!feed.class.evadable(), "acid is never evadable");
    assert_eq!(p.centre_x(), 280.0);
    // A hero more than 6 px up (any jump, the hop) is outside.
    let mut p = patch(248, 312, None);
    assert!(!p.contact(HeroId::Carl, IRect::new(260, 550, 24, 24), true));
    assert!(p.contact(HeroId::Carl, body(280, 12, 52), true));
}

#[test]
fn the_width_is_32_to_80() {
    let t = tuning();
    for (w, ok) in [(31, false), (32, true), (64, true), (80, true), (81, false)] {
        let r = AcidPatch::new(&t, patch_spec(1, 200, 200 + w, None));
        assert_eq!(r.is_ok(), ok, "width {w}");
        if !ok {
            assert!(matches!(r, Err(AcidError::Width { .. })));
        }
    }
}

#[test]
fn the_pop_sign_follows_the_centre() {
    let p = patch(200, 264, None);
    assert_eq!(p.centre_x(), 232.0);
    let right = p.pop_blow(HeroId::Carl, 240.0);
    assert_eq!(
        (right.knock_vx, right.knock_vy, right.damage),
        (180.0, 200.0, 1)
    );
    assert_eq!(right.from_x, 232.0);
    assert_eq!(right.source, acid_source(1));
    assert_eq!(p.pop_blow(HeroId::Carl, 220.0).knock_vx, -180.0);
    assert_eq!(
        p.pop_blow(HeroId::Donut, 232.0).knock_vx,
        180.0,
        "a hero exactly at the centre pops toward +x"
    );
}

#[test]
fn the_pop_clears_the_widest_patch_inside_the_hurt_lock() {
    let t = tuning();
    // 18 ticks at 3 px per tick carry the hero 54 px; leaving an 80 px patch from
    // its exact centre takes 40 + the hero's half-width.
    let carried = f32::from(t.hazq19_pop_x) / 60.0 * f32::from(crate::feel::HERO_HURT_TICKS);
    assert_eq!(carried, 54.0);
    let widest = f32::from(t.hazq18_width_max_px);
    assert!(carried >= widest / 2.0 + 12.0, "Carl: 52 px");
    assert!(carried >= widest / 2.0 + 10.0, "Donut: 50 px");
    // Played out against the real overlap: an 80 px pool 200..280, the hero one
    // pixel left of the centre, popped -x at 3 px per tick for the lock.
    for half in [12, 10] {
        let mut p = AcidPatch::new(&t, patch_spec(1, 200, 280, None)).unwrap();
        assert!(p.contact(HeroId::Carl, body(239, half, 52), true));
        let blow = p.pop_blow(HeroId::Carl, 239.0);
        assert_eq!(blow.knock_vx, -180.0);
        let landed = 239 - 54;
        assert!(
            !p.rect().unwrap().overlaps(body(landed, half, 24)),
            "half-width {half} lands outside"
        );
        // No second hurt: the hero never re-enters.
        assert!(!p.contact(HeroId::Carl, body(landed, half, 24), true));
    }
}

#[test]
fn one_hurt_per_entry_rearm_after_one_tick_outside() {
    let mut p = patch(200, 264, None);
    let inside = body(232, 12, 52);
    let outside = body(150, 12, 52);
    assert!(p.contact(HeroId::Carl, inside, true), "tick 1 of the entry");
    for _ in 0..30 {
        assert!(!p.contact(HeroId::Carl, inside, true), "no per-tick drain");
    }
    assert!(!p.contact(HeroId::Carl, outside, true));
    assert!(
        p.contact(HeroId::Carl, inside, true),
        "a new entry hurts again"
    );
    // Per victim: Donut's token is her own.
    assert!(p.contact(HeroId::Donut, body(232, 10, 24), true));
}

#[test]
fn an_invulnerable_entry_leaves_the_token_pending() {
    let mut p = patch(200, 264, None);
    let inside = body(232, 12, 52);
    for _ in 0..30 {
        assert!(!p.contact(HeroId::Carl, inside, false));
    }
    assert!(
        p.contact(HeroId::Carl, inside, true),
        "hurts when it ends inside"
    );
}

#[test]
fn a_jump_clears_the_patch() {
    let mut p = patch(200, 264, None);
    // Feet 8 px up the whole way across.
    for x in (150..320).step_by(3) {
        let b = IRect::new(x - 12, 580 - 8 - 52, 24, 52);
        assert!(!p.contact(HeroId::Carl, b, true), "x {x}");
    }
}

#[test]
fn the_drain_bounds_are_whole_pixels_and_never_zero_width() {
    // The sheet's formula: both edges move inward by floor(w / 2 * k / 60).
    for w in 32..=80 {
        let (x0, x1) = (200, 200 + w);
        let mut prev_inset = 0;
        for k in 0..60u16 {
            let (a, b) = drain_bounds(x0, x1, k, 60).expect("a volume until tick 60");
            assert!(b > a, "width {w} tick {k} would reach 0");
            let inset = a - x0;
            assert_eq!(x1 - b, inset, "symmetric");
            assert!(inset >= prev_inset, "monotonic, width {w} tick {k}");
            assert_eq!(i64::from(inset), i64::from(w) * i64::from(k) / 120);
            prev_inset = inset;
        }
        assert_eq!(drain_bounds(x0, x1, 60, 60), None, "removed on tick 60");
        assert_eq!(drain_bounds(x0, x1, 61, 60), None);
    }
    assert_eq!(drain_bounds(200, 264, 0, 60), Some((200, 264)));
    assert_eq!(drain_bounds(200, 264, 30, 60), Some((216, 248)));
    assert_eq!(drain_bounds(200, 264, 59, 60), Some((231, 233)));
}

#[test]
fn a_linked_pool_drains_over_60_ticks_and_is_removed_on_tick_60() {
    let t = tuning();
    let mut p = patch(200, 264, Some(0));
    p.step();
    assert_eq!(
        p.pool_state(),
        PoolState::Full,
        "stepping alone never drains"
    );
    // Defeat tick: drain tick 0, the bounds are still the full ones.
    p.on_slime_defeated();
    assert_eq!(p.pool_state(), PoolState::Draining);
    let mut last_width = 64;
    for k in 0..60u16 {
        p.step();
        let (a, b) = p.bounds().expect("present until tick 60");
        let expect = drain_bounds(200, 264, k, 60).unwrap();
        assert_eq!((a, b), expect, "the volume tracks the formula on tick {k}");
        assert!(b - a <= last_width && b > a);
        last_width = b - a;
        // The feed (and so the stain) shares the current bounds every tick.
        let feed = p.nav_rect().unwrap();
        assert_eq!((feed.rect.x, feed.rect.right()), (a, b));
        // No hurt outside the current bounds even inside the original ones.
        if k >= 30 {
            let original_only = body(205, 4, 24);
            assert!(!p.clone().contact(HeroId::Carl, original_only, true));
        }
    }
    p.step();
    assert_eq!(
        p.pool_state(),
        PoolState::Removed,
        "removed on drain tick 60"
    );
    assert!(p.rect().is_none() && p.nav_rect().is_none());
    assert!(!p.contact(HeroId::Carl, body(232, 12, 52), true));
    // Restored with the drain cancelled when the slime respawns.
    p.on_slime_respawn(&t);
    assert_eq!(p.pool_state(), PoolState::Full);
    assert_eq!(p.bounds(), Some((200, 264)));
    // Mid-drain respawn cancels it too.
    let mut q = patch(200, 264, Some(0));
    q.on_slime_defeated();
    for _ in 0..20 {
        q.step();
    }
    q.on_slime_respawn(&t);
    assert_eq!(q.bounds(), Some((200, 264)));
    q.step();
    assert_eq!(q.bounds(), Some((200, 264)), "no drain is left running");
}

#[test]
fn an_unlinked_pool_is_permanent_and_a_defeat_drains_only_its_own() {
    let t = tuning();
    let mut p = patch(200, 264, None);
    p.on_slime_defeated();
    for _ in 0..200 {
        p.step();
    }
    assert_eq!(p.pool_state(), PoolState::Full);
    p.on_slime_respawn(&t);
    assert_eq!(p.bounds(), Some((200, 264)));

    let mut set = AcidSet::new(
        &t,
        vec![
            patch_spec(1, 248, 312, Some(0)),
            patch_spec(2, 768, 832, Some(2)),
            patch_spec(3, 500, 540, None),
        ],
    )
    .unwrap();
    set.slime_defeated(0);
    for _ in 0..61 {
        set.step();
    }
    assert_eq!(set.get(1).unwrap().pool_state(), PoolState::Removed);
    assert_eq!(set.get(2).unwrap().pool_state(), PoolState::Full);
    assert_eq!(set.get(3).unwrap().pool_state(), PoolState::Full);
    assert_eq!(set.nav_feed().len(), 2, "removed pools leave the feed");
    set.slime_respawned(&t, 0);
    assert_eq!(set.get(1).unwrap().pool_state(), PoolState::Full);
    assert!(
        AcidSet::new(
            &t,
            vec![patch_spec(1, 248, 312, None), patch_spec(1, 400, 464, None)]
        )
        .is_err()
    );
}

#[test]
fn a_launched_body_is_hurt_once_per_launch() {
    let t = tuning();
    let mut p = patch(200, 264, None);
    let lying = body(232, 12, 38);
    assert!(p.launched_landing(7, lying));
    for _ in 0..30 {
        assert!(
            !p.launched_landing(7, lying),
            "lying in acid does nothing further"
        );
    }
    assert!(p.launched_landing(8, lying), "a new launch is a new token");
    // A landing outside spends nothing.
    assert!(!p.launched_landing(9, body(100, 12, 38)));
    assert!(p.launched_landing(9, lying));
    // The switch.
    let mut off = t.clone();
    off.hazq20_acid_hurts_launched = false;
    let mut q = AcidPatch::new(&off, patch_spec(1, 200, 264, None)).unwrap();
    assert!(!q.launched_landing(1, lying));
}

#[test]
fn the_acid_patch_hash_follows_its_state_and_is_deterministic() {
    let hash = |p: &AcidPatch| {
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        p.mix_into(&mut |v| h = (h ^ v).wrapping_mul(0x0000_0100_0000_01b3));
        h
    };
    let t = tuning();
    let run = || {
        let mut p = patch(200, 264, Some(0));
        let mut out = vec![hash(&p)];
        p.contact(HeroId::Carl, body(232, 12, 52), true);
        out.push(hash(&p));
        p.launched_landing(3, body(232, 12, 38));
        out.push(hash(&p));
        p.on_slime_defeated();
        for _ in 0..62 {
            p.step();
            out.push(hash(&p));
        }
        p.reset(&t);
        out.push(hash(&p));
        out
    };
    let a = run();
    assert_eq!(a, run());
    assert_eq!(a.last(), a.first(), "a reset is a fresh patch");
    let unique: std::collections::BTreeSet<u64> = a.iter().copied().collect();
    assert!(unique.len() > 60);
}

#[test]
fn the_acid_volume_is_never_among_the_room_hazards() {
    // The patch lives in its own array: a hazard spec of this module is a
    // persistent volume the room's `hazards` never lists.
    let p = patch(200, 264, None);
    assert!(p.nav_rect().is_some());
    // The volume data is the plan's vocabulary: all heroes, enemies too.
    let spec = AcidPatch::volume_spec(p.spec(), &tuning(), 6);
    assert_eq!(spec.hero_damage, HeroDamagePolicy::AllHeroes);
    assert!(spec.hurts_enemies && spec.is_persistent());
    assert_eq!(spec.class, HazardClass::Acid);
}
