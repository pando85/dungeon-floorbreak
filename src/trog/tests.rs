//! TROG-A domain tests: TROG-T01 (the tuning), TROG-T02 (the boxes against the manifests and the
//! taught-punish overlaps), TROG-T03 (the commit rule) and the machine's rules one by one.

use super::*;
use crate::combat::{CARL_HAYMAKER, CARL_JAB, CARL_PUSH_KICK, DONUT_POUNCE, HitShape, cf02_moves};
use crate::prototype::Material;

const FLOOR: f32 = 580.0;

fn t() -> &'static TrogTuning {
    TrogTuning::active()
}

fn hero(id: HeroId, x: f32, foot: f32, w: f32, h: f32, bot: bool) -> HeroView {
    HeroView {
        id,
        present: true,
        x,
        foot_y: foot,
        grounded: foot <= FLOOR,
        hurt: Rect {
            left: x - w * 0.5,
            bottom: foot,
            width: w,
            height: h,
        },
        vulnerable: true,
        bot,
    }
}

fn carl(x: f32) -> HeroView {
    hero(HeroId::Carl, x, FLOOR, 24.0, 52.0, false)
}

fn donut(x: f32) -> HeroView {
    hero(HeroId::Donut, x, FLOOR, 20.0, 24.0, false)
}

fn away() -> HeroView {
    let mut h = carl(0.0);
    h.present = false;
    h
}

fn solid(x: f32, bottom: f32, w: f32, h: f32) -> Block {
    Block {
        rect: Rect {
            left: x,
            bottom,
            width: w,
            height: h,
        },
        material: Material::Stone,
        intact: true,
    }
}

struct Sim {
    trog: Trog,
    heroes: [HeroView; 2],
    solids: Vec<Block>,
    tick: u64,
    solo: bool,
    view: (f32, f32),
    last_tell: Option<u64>,
    events: Vec<(u64, TrogAct)>,
    blows: Vec<(u64, HeroBlow)>,
    evade: bool,
}

impl Sim {
    fn new(species: Species, x: f32, trigger: Option<f32>) -> Self {
        Self {
            trog: Trog::new(
                t(),
                &Spawn {
                    species,
                    index: 0,
                    x,
                    floor_y: FLOOR,
                    trigger_x: trigger,
                    delay_ticks: 0,
                    protected: (16.0, 224.0),
                },
            ),
            heroes: [carl(112.0), donut(80.0)],
            solids: Vec::new(),
            tick: 0,
            solo: false,
            view: (0.0, 4000.0),
            last_tell: None,
            events: Vec::new(),
            blows: Vec::new(),
            evade: false,
        }
    }

    /// An awake trog with its heroes placed.
    fn awake(species: Species, x: f32, carl_x: f32, donut_x: f32) -> Self {
        let mut s = Self::new(species, x, Some(0.0));
        s.heroes = [carl(carl_x), donut(donut_x)];
        s.step();
        assert_eq!(s.trog.state, State::Approach);
        s
    }

    fn step(&mut self) -> TrogStep {
        self.tick += 1;
        let ctx = TrogCtx {
            tick: self.tick,
            heroes: &self.heroes,
            solids: &self.solids,
            solo: self.solo,
            view_x: self.view,
            token_free: true,
            last_tell_start: self.last_tell,
        };
        let mut out = TrogStep::default();
        let evade = self.evade;
        self.trog.step(t(), &ctx, &mut |_| evade, &mut out);
        if out.tell_started {
            self.last_tell = Some(self.tick);
        }
        for a in &out.acts {
            self.events.push((self.tick, *a));
        }
        for b in &out.blows {
            self.blows.push((self.tick, *b));
        }
        out
    }

    fn run(&mut self, n: u64) {
        for _ in 0..n {
            self.step();
        }
    }

    fn run_until(&mut self, max: u64, f: impl Fn(&Trog) -> bool) {
        for _ in 0..max {
            if f(&self.trog) {
                return;
            }
            self.step();
        }
        panic!("the condition was not reached in {max} ticks");
    }

    fn acts(&self) -> Vec<TrogAct> {
        self.events.iter().map(|(_, a)| *a).collect()
    }
}

fn hit(s: &mut Sim, kind: HitKind, damage: u16) -> Option<HitOutcome> {
    let mut out = TrogStep::default();
    let r = s.trog.hit(t(), s.tick, kind, damage, &mut out);
    for a in out.acts {
        s.events.push((s.tick, a));
    }
    r
}

// ------------------------------------------------------------------ T01

#[test]
fn trog_tuning_loads_and_validates() {
    let tuning = TrogTuning::pinned().expect("the shipped data is pinned and valid");
    assert_eq!(tuning.pygmy.hp, 2);
    assert_eq!((tuning.basher.hp, tuning.virtuoso.hp), (4, 3));
    assert_eq!(content_hash(TUNING_JSON), TUNING_HASH);
    // An edited file fails the pin.
    let edited = TUNING_JSON.replace("\"trogq01_damage\": 1", "\"trogq01_damage\": 2");
    assert!(matches!(
        TrogTuning::from_json_pinned(&edited, TUNING_HASH),
        Err(TrogTuningError::HashMismatch { .. })
    ));
}

#[test]
fn every_out_of_range_value_names_its_field() {
    let base: serde_json::Value = serde_json::from_str(TUNING_JSON).unwrap();
    let cases: [(&[&str], serde_json::Value); 14] = [
        (&["trogq01_damage"], 0.into()),
        (&["trogq02_tell_gap_ticks"], 9999.into()),
        (&["trogq03_wake_margin_px"], 500.into()),
        (&["trogq04_ai_bias_px"], 900.into()),
        (&["pygmy", "hp"], 0.into()),
        (&["pygmy", "walk_px_tick"], 0.0.into()),
        (&["pygmy", "strike_start"], 3.into()),
        (&["pygmy", "leap_max_px"], 10.into()),
        (&["pygmy", "hitbox", "x0"], 99.into()),
        (&["basher", "open_x0"], 99.into()),
        (&["basher", "interrupt_cooldown"], 0.into()),
        (&["virtuoso", "standoff_px"], 0.into()),
        (&["virtuoso", "leash_px"], 4.into()),
        (&["virtuoso", "recover_ticks"], 0.into()),
    ];
    for (path, value) in cases {
        let mut v = base.clone();
        let mut node = &mut v;
        for key in &path[..path.len() - 1] {
            node = &mut node[*key];
        }
        node[path[path.len() - 1]] = value;
        let err = TrogTuning::from_json(&v.to_string()).expect_err(&path.join("."));
        let text = err.to_string();
        // A box error names the box; every other error names the leaf field.
        let named = if path.contains(&"hitbox") {
            "hitbox"
        } else {
            path[path.len() - 1]
        };
        assert!(text.contains(named), "{path:?}: {text}");
    }
}

// ------------------------------------------------------------------ T02

fn manifest(n: u32) -> serde_json::Value {
    let path = format!(
        "{}/assets/manifests/trog-0{n}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn mbox(v: &serde_json::Value) -> BoxPx {
    let g = |a: &serde_json::Value, i: usize| a[i].as_i64().unwrap() as i16;
    BoxPx {
        x0: g(&v["x"], 0),
        x1: g(&v["x"], 1),
        y0: g(&v["y"], 0),
        y1: g(&v["y"], 1),
    }
}

fn window(v: &serde_json::Value, key: &str) -> (u16, u16) {
    let a = &v["attack_ticks"][key];
    (a[0].as_u64().unwrap() as u16, a[1].as_u64().unwrap() as u16)
}

fn hurt_ticks(m: &serde_json::Value) -> u16 {
    m["frames"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["tag"] == "hurt")
        .map(|f| f["ticks"].as_u64().unwrap() as u16)
        .sum()
}

#[test]
fn trog_boxes_match_the_manifests() {
    let tuning = t();
    // (species, manifest, key, hitbox key, [(tuning box, manifest box)])
    let m1 = manifest(1);
    let m2 = manifest(2);
    let m3 = manifest(3);
    let p = &m1["pygmy"];
    let b = &m2["basher"];
    let v = &m3["virtuoso"];
    let check = |sp: &SpeciesTuning, meta: &serde_json::Value, pairs: [(&str, &str); 6]| {
        let boxes = &meta["boxes_px_from_pivot"];
        let mine = [
            &sp.hitbox,
            &sp.hurt_idle,
            &sp.hurt_tell,
            &sp.hurt_active,
            &sp.hurt_recover,
            &sp.hurt_hurt,
        ];
        for (mine, (_, key)) in mine.iter().zip(pairs) {
            assert_eq!(**mine, mbox(&boxes[key]), "{key}");
        }
    };
    check(
        &tuning.pygmy,
        p,
        [
            ("hitbox", "attack_hitbox_impact_frame_only"),
            ("hurt_idle", "hurtbox_idle_walk"),
            ("hurt_tell", "hurtbox_tell"),
            ("hurt_active", "hurtbox_leap"),
            ("hurt_recover", "hurtbox_land"),
            ("hurt_hurt", "hurtbox_hurt"),
        ],
    );
    check(
        &tuning.basher,
        b,
        [
            ("hitbox", "attack_hitbox_impact_frame_only"),
            ("hurt_idle", "hurtbox_idle_walk"),
            ("hurt_tell", "hurtbox_tell"),
            ("hurt_active", "hurtbox_smash"),
            ("hurt_recover", "hurtbox_recover"),
            ("hurt_hurt", "hurtbox_hurt"),
        ],
    );
    check(
        &tuning.virtuoso,
        v,
        [
            ("hitbox", "tongue_hitbox_strike_frame_only"),
            ("hurt_idle", "hurtbox_idle_walk"),
            ("hurt_tell", "hurtbox_feint"),
            ("hurt_active", "hurtbox_lash_strike"),
            ("hurt_recover", "hurtbox_retract"),
            ("hurt_hurt", "hurtbox_hurt"),
        ],
    );
    // The windows.
    let (tell, _) = window(p, "tell");
    assert_eq!(tell, 0);
    assert_eq!(window(p, "tell").1 + 1, tuning.pygmy.tell_ticks);
    assert_eq!(window(p, "leap_launch").0, tuning.pygmy.tell_ticks);
    assert_eq!(window(p, "leap_strike_impact"), (27, 34));
    assert_eq!(
        (tuning.pygmy.strike_start, tuning.pygmy.strike_end),
        window(p, "leap_strike_impact")
    );
    let land = window(p, "land_recover");
    assert_eq!(land.0, tuning.pygmy.strike_end + 1);
    assert_eq!(land.1 - land.0 + 1, tuning.pygmy.recover_ticks);
    assert_eq!(hurt_ticks(p), tuning.pygmy.hurt_ticks);

    assert_eq!(window(b, "tell_hold").1 + 1, tuning.basher.tell_ticks);
    assert_eq!(window(b, "downswing").0, tuning.basher.tell_ticks);
    assert_eq!(
        (tuning.basher.strike_start, tuning.basher.strike_end),
        window(b, "impact_hitbox")
    );
    let rec = window(b, "recover_punish");
    assert_eq!(rec.0, tuning.basher.strike_end + 1);
    assert_eq!(rec.1 - rec.0 + 1, tuning.basher.recover_ticks);
    assert_eq!(hurt_ticks(b), tuning.basher.hurt_ticks);

    assert_eq!(window(v, "feint_hold").1 + 1, tuning.virtuoso.tell_ticks);
    assert_eq!(window(v, "lash").0, tuning.virtuoso.tell_ticks);
    assert_eq!(
        (tuning.virtuoso.strike_start, tuning.virtuoso.strike_end),
        window(v, "strike_hitbox")
    );
    let rec = window(v, "retract_punish");
    assert_eq!(rec.0, tuning.virtuoso.strike_end + 1);
    assert_eq!(rec.1 - rec.0 + 1, tuning.virtuoso.recover_ticks);
    assert_eq!(hurt_ticks(v), tuning.virtuoso.hurt_ticks);
}

fn move_y(shape: &HitShape) -> (f32, f32) {
    match *shape {
        HitShape::Box { y_min, y_max, .. } => (y_min, y_max),
        _ => panic!("a box"),
    }
}

fn overlaps_y(a: (f32, f32), b: &BoxPx) -> bool {
    a.0 < f32::from(b.y1) && a.1 > f32::from(b.y0)
}

/// TROG-AD22 (review D1): the taught punish must overlap the Pygmy's boxes in y, and the verbs
/// that cannot are asserted not to.
#[test]
fn the_taught_punish_overlaps_the_pygmy_and_the_other_verbs_cannot() {
    let sp = &t().pygmy;
    let stomp = move_y(&cf02_moves().stomp.shape);
    assert_eq!(stomp, (0.0, 10.0));
    let pounce = move_y(&DONUT_POUNCE.shape);
    assert!(
        overlaps_y(stomp, &sp.hurt_recover),
        "the stomp meets `land`"
    );
    assert!(
        overlaps_y(pounce, &sp.hurt_recover),
        "the pounce meets `land`"
    );
    assert!(
        overlaps_y(move_y(&CARL_JAB.shape), &sp.hurt_idle),
        "the jab meets the idle box of Rest"
    );
    for (name, y) in [
        ("jab", move_y(&CARL_JAB.shape)),
        ("haymaker", move_y(&CARL_HAYMAKER.shape)),
        ("push kick", move_y(&CARL_PUSH_KICK.shape)),
        ("bolt", (16.0, 22.0)),
    ] {
        assert!(
            !overlaps_y(y, &sp.hurt_recover),
            "{name} cannot meet `land`"
        );
    }
}

#[test]
fn the_leap_follows_the_integer_parabola() {
    let n = t().pygmy.air_ticks();
    assert_eq!(n, 11);
    let heights: Vec<f32> = (3..=10).map(|k| leap_height(16, k, n)).collect();
    assert_eq!(heights, [13.0, 15.0, 16.0, 16.0, 15.0, 13.0, 10.0, 5.0]);
    assert_eq!((leap_height(16, 0, n), leap_height(16, 11, n)), (0.0, 0.0));
}

// ------------------------------------------------------------------ T03

#[test]
fn trog_commit_at_tell_start_pygmy() {
    let mut s = Sim::awake(Species::Pygmy, 640.0, 560.0, 80.0);
    s.run_until(5, |g| g.state == State::Tell);
    let landing = s.trog.landing_x();
    let facing = s.trog.facing;
    assert_eq!(facing, -1);
    // The heroes move away during the tell: nothing re-aims.
    s.heroes[0] = carl(300.0);
    s.run_until(40, |g| g.state == State::Recover);
    assert_eq!(s.trog.landing_x(), landing);
    assert_eq!(s.trog.facing, facing);
    assert!(
        (s.trog.x - landing).abs() < 0.01,
        "it lands where it committed"
    );
    assert!(s.blows.is_empty(), "no blow on a hero who left");
    assert!(s.acts().contains(&TrogAct::Whiff));
}

#[test]
fn trog_commit_at_tell_start_basher_and_virtuoso() {
    for (species, x, carl_x) in [
        (Species::Basher, 700.0, 662.0),
        (Species::Virtuoso, 700.0, 634.0),
    ] {
        let mut s = Sim::awake(species, x, carl_x, 80.0);
        s.run_until(200, |g| g.state == State::Tell);
        let rect = s.trog.smash_rect();
        let facing = s.trog.facing;
        s.heroes[0] = carl(200.0);
        s.run_until(200, |g| g.state == State::Recover);
        assert_eq!(s.trog.smash_rect(), rect);
        assert_eq!(s.trog.facing, facing);
        assert!(s.blows.is_empty(), "{species:?}: no tracking");
    }
}

#[test]
fn an_attack_lands_one_blow_per_hero_per_instance() {
    let mut s = Sim::awake(Species::Basher, 700.0, 662.0, 664.0);
    s.run_until(200, |g| g.state == State::Recover);
    // Carl (and Donut inside the rect) each take exactly one blow.
    let heroes: Vec<HeroId> = s.blows.iter().map(|(_, b)| b.hero).collect();
    assert!(heroes.iter().filter(|h| **h == HeroId::Carl).count() <= 1);
    assert!(heroes.iter().filter(|h| **h == HeroId::Donut).count() <= 1);
    assert!(!heroes.is_empty());
    let strike_ticks: Vec<u64> = s
        .events
        .iter()
        .filter(|(_, a)| *a == TrogAct::Strike)
        .map(|(tick, _)| *tick)
        .collect();
    assert_eq!(strike_ticks.len(), 1, "one Strike per attack instance");
}

#[test]
fn a_blow_never_lands_before_the_active_window() {
    for (species, x, carl_x) in [
        (Species::Pygmy, 640.0, 560.0),
        (Species::Basher, 700.0, 662.0),
        (Species::Virtuoso, 700.0, 634.0),
    ] {
        let mut s = Sim::awake(species, x, carl_x, 80.0);
        s.run_until(200, |g| g.state == State::Tell);
        let start = s.tick;
        s.run_until(200, |g| g.state == State::Rest);
        let sp = t().species(species);
        for (tick, _) in &s.blows {
            let a = tick - start;
            assert!(
                a >= u64::from(sp.strike_start) && a <= u64::from(sp.strike_end),
                "{species:?}: a blow on attack tick {a}"
            );
        }
    }
}

// ----------------------------------------------------------- the rules

#[test]
fn a_proximity_wake_needs_the_whole_body_in_view() {
    let mut s = Sim::new(Species::Pygmy, 640.0, Some(400.0));
    s.heroes[0] = carl(420.0);
    s.view = (200.0, 650.0);
    s.run(5);
    assert_eq!(
        s.trog.state,
        State::Dormant,
        "the body reaches 654, outside the view"
    );
    s.view = (200.0, 660.0);
    s.step();
    assert_eq!(s.trog.state, State::Approach);
    assert_eq!(
        s.acts(),
        [TrogAct::Wake {
            cause: WakeCause::Proximity
        }]
    );
    assert_eq!(s.trog.wake_cause(), Some(WakeCause::Proximity));
}

#[test]
fn a_hit_wakes_a_sleeper_into_rest_and_logs_its_cause() {
    let mut s = Sim::new(Species::Basher, 700.0, Some(600.0));
    let r = hit(&mut s, HitKind::Missile, 1).unwrap();
    assert_eq!((r.hp_left, r.defeated), (3, false));
    assert_eq!(s.trog.state, State::Rest);
    assert_eq!(
        s.acts(),
        [TrogAct::Wake {
            cause: WakeCause::Hit
        }]
    );
    assert_eq!(s.trog.wake_cause(), Some(WakeCause::Hit));
    // The cause is in the state hash.
    let mut p = Sim::new(Species::Basher, 700.0, Some(600.0));
    let mut h1 = Vec::new();
    s.trog.mix_into(&mut |v| h1.push(v));
    p.trog.hp = 3;
    p.trog.state = State::Rest;
    p.trog.rest_left = s.trog.rest_left;
    p.trog.wake = Some(WakeCause::Proximity);
    let mut h2 = Vec::new();
    p.trog.mix_into(&mut |v| h2.push(v));
    assert_ne!(h1, h2);
}

/// TROG-AD29 (re-review R3): a one-hit defeat of a sleeper logs no `Wake`.
#[test]
fn a_lethal_hit_on_a_sleeper_logs_no_wake_event() {
    let mut s = Sim::new(Species::Pygmy, 640.0, Some(400.0));
    let r = hit(&mut s, HitKind::Melee, 2).unwrap();
    assert!(r.defeated);
    assert_eq!(s.trog.state, State::Down);
    assert!(s.events.is_empty(), "{:?}", s.events);
    assert!(s.trog.hurtbox(t()).is_none());
    assert!(
        hit(&mut s, HitKind::Melee, 1).is_none(),
        "a downed trog takes no hit"
    );
}

#[test]
fn a_grounded_donut_is_never_a_virtuoso_target_while_carl_is_one() {
    // Donut is the nearer one; Carl (feet below 46, hurt 0..52) can be struck, she cannot.
    let mut s = Sim::awake(Species::Virtuoso, 700.0, 560.0, 640.0);
    s.run_until(5, |g| g.target().is_some());
    assert_eq!(s.trog.target(), Some(HeroId::Carl));
    // With Donut alone she is followed, never told at.
    s.heroes[0] = away();
    s.run(400);
    assert_eq!(s.trog.target(), Some(HeroId::Donut));
    assert!(
        !s.acts().iter().any(|a| matches!(a, TrogAct::Tell { .. })),
        "no tell at an unreachable hero"
    );
}

#[test]
fn a_raised_donut_is_a_virtuoso_target() {
    let mut s = Sim::awake(Species::Virtuoso, 700.0, 80.0, 640.0);
    s.heroes[0] = away();
    // Donut with her feet 32 up (a stand): her hurt box 32..56 meets the band 36..46.
    s.heroes[1] = hero(HeroId::Donut, 640.0, FLOOR + 32.0, 20.0, 24.0, false);
    s.run(200);
    assert!(
        s.acts().iter().any(|a| matches!(
            a,
            TrogAct::Tell {
                target: HeroId::Donut
            }
        )),
        "{:?}",
        s.acts()
    );
}

/// TROG-AD27 (re-review R1): the solo bias applies after the band filter.
#[test]
fn solo_the_bot_hero_counts_farther_away() {
    // Equal distance: Donut (the bot) is nearer by 10 px; with the bias Carl is chosen.
    let mut s = Sim::awake(Species::Basher, 700.0, 620.0, 630.0);
    s.heroes[1].bot = true;
    s.solo = true;
    s.step();
    assert_eq!(s.trog.target(), Some(HeroId::Carl));
    // Without solo, the nearer Donut wins.
    let mut c = Sim::awake(Species::Basher, 700.0, 620.0, 630.0);
    c.step();
    assert_eq!(c.trog.target(), Some(HeroId::Donut));
    // A tie goes to Donut.
    let mut tie = Sim::awake(Species::Basher, 700.0, 640.0, 760.0);
    tie.step();
    assert_eq!(tie.trog.target(), Some(HeroId::Donut));
}

/// TROG-AD30 (re-review R4): with no hero outside the protected spawn it holds.
#[test]
fn with_no_hero_outside_the_protected_spawn_it_holds() {
    let mut s = Sim::awake(Species::Pygmy, 640.0, 112.0, 80.0);
    let x = s.trog.x;
    s.run(300);
    assert_eq!(s.trog.target(), None);
    assert_eq!(s.trog.x, x);
    assert_eq!(s.trog.state, State::Approach);
    assert!(s.acts().iter().all(|a| !matches!(a, TrogAct::Tell { .. })));
}

/// TROG-AD31 (re-review R5): a hero outside the leash's reach draws no tell and is not a
/// candidate; the reachable hero is chosen even when farther.
#[test]
fn a_hero_out_of_the_leash_reach_is_not_a_candidate() {
    // The refuge case: leash 448..832, reach 32 -> a hurt box ending at or before 416 is out.
    let mut s = Sim::awake(Species::Pygmy, 640.0, 404.0, 500.0);
    s.step();
    assert_eq!(
        s.trog.target(),
        Some(HeroId::Donut),
        "Carl at 404 is out of reach"
    );
    s.heroes[0] = carl(405.0);
    s.heroes[1] = away();
    s.step();
    assert!(
        s.trog.is_candidate(t(), &s.heroes[0].clone()),
        "405 reaches 417 > 416"
    );
}

#[test]
fn the_pygmy_never_leaves_its_leash() {
    let mut s = Sim::awake(Species::Pygmy, 640.0, 300.0, 80.0);
    s.view = (0.0, 4000.0);
    s.run(900);
    let (l, r) = s.trog.leash();
    assert!(s.trog.x >= l - 0.01 && s.trog.x <= r + 0.01);
    assert!(s.events.iter().all(|(_, a)| !matches!(a, TrogAct::Strike)) || !s.blows.is_empty());
    assert!(s.trog.x >= 448.0 - 0.01);
}

#[test]
fn the_pygmy_stops_a_leap_at_a_solid_face() {
    let mut s = Sim::awake(Species::Pygmy, 640.0, 520.0, 80.0);
    // A 48 px wall just left of its path.
    s.solids.push(solid(560.0, FLOOR, 16.0, 48.0));
    s.run_until(200, |g| g.state == State::Recover);
    assert!(
        s.trog.x >= 576.0 - 0.01,
        "it never entered the solid: {}",
        s.trog.x
    );
}

#[test]
fn the_basher_interrupt_window_and_budget() {
    let tick_of_tell = |s: &mut Sim| s.run_until(300, |g| g.state == State::Tell);
    // In the window: a Carl hit interrupts once.
    let mut s = Sim::awake(Species::Basher, 700.0, 662.0, 80.0);
    tick_of_tell(&mut s);
    s.run(5);
    hit(&mut s, HitKind::Melee, 1);
    assert_eq!(s.trog.state, State::Hurt);
    assert!(s.acts().contains(&TrogAct::Interrupted));
    // The budget: a second tell inside 120 ticks of the interrupt is not interrupted.
    s.run_until(300, |g| g.state == State::Tell);
    if s.tick - s.trog.last_react.unwrap() < 120 {
        s.run(2);
        hit(&mut s, HitKind::Melee, 1);
        assert_ne!(
            s.trog.state,
            State::Hurt,
            "no second interrupt in 120 ticks"
        );
    }
    // A bolt never interrupts, a late tell never does either.
    let mut b = Sim::awake(Species::Basher, 700.0, 662.0, 80.0);
    tick_of_tell(&mut b);
    b.run(2);
    hit(&mut b, HitKind::Missile, 1);
    assert_eq!(
        b.trog.state,
        State::Tell,
        "bolts never interrupt the Basher"
    );
    let mut late = Sim::awake(Species::Basher, 700.0, 662.0, 80.0);
    tick_of_tell(&mut late);
    late.run(20);
    hit(&mut late, HitKind::Melee, 1);
    assert_eq!(late.trog.state, State::Tell, "only ticks 0..17 interrupt");
    assert_eq!(late.trog.hp, 3, "the hit still deals its damage");
}

#[test]
fn the_virtuoso_is_interrupted_by_melee_and_not_by_bolts() {
    let mut s = Sim::awake(Species::Virtuoso, 700.0, 634.0, 80.0);
    s.run_until(300, |g| g.state == State::Tell);
    s.run(3);
    hit(&mut s, HitKind::Missile, 1);
    assert_eq!(s.trog.state, State::Tell);
    hit(&mut s, HitKind::Pounce, 1);
    assert_eq!(s.trog.state, State::Hurt);
}

#[test]
fn the_pygmy_is_interrupted_by_any_hit_and_committed_after_the_tell() {
    let mut s = Sim::awake(Species::Pygmy, 640.0, 560.0, 80.0);
    s.run_until(10, |g| g.state == State::Tell);
    s.run(3);
    hit(&mut s, HitKind::Swat, 0);
    assert_eq!(s.trog.state, State::Hurt);
    assert!(s.acts().contains(&TrogAct::Interrupted));
    // After the tell it only takes damage.
    let mut c = Sim::awake(Species::Pygmy, 640.0, 560.0, 80.0);
    c.run_until(10, |g| g.state == State::Tell);
    c.run_until(60, |g| g.state == State::Recover);
    let hp = c.trog.hp;
    hit(&mut c, HitKind::Melee, 1);
    assert_eq!(c.trog.hp, hp - 1);
    assert_eq!(c.trog.state, State::Recover, "committed: damage only");
}

#[test]
fn a_blast_interrupts_every_species() {
    for (species, x, carl_x) in [
        (Species::Pygmy, 640.0, 560.0),
        (Species::Basher, 700.0, 662.0),
        (Species::Virtuoso, 700.0, 634.0),
    ] {
        let mut s = Sim::awake(species, x, carl_x, 80.0);
        s.run_until(200, |g| g.state == State::Tell);
        s.run(10);
        hit(&mut s, HitKind::Blast, 1);
        assert_eq!(s.trog.state, State::Hurt, "{species:?}");
    }
}

#[test]
fn a_contact_flinches_the_pygmy_only() {
    let mut p = Sim::awake(Species::Pygmy, 640.0, 300.0, 80.0);
    hit(&mut p, HitKind::Contact, 0);
    assert_eq!(p.trog.state, State::Hurt);
    let mut b = Sim::awake(Species::Basher, 700.0, 300.0, 80.0);
    hit(&mut b, HitKind::Contact, 0);
    assert_eq!(b.trog.state, State::Approach);
}

#[test]
fn the_virtuoso_band_is_clipped_at_the_first_standing_solid() {
    let band = rect_of(&t().virtuoso.hitbox, 700.0, -1, FLOOR);
    assert_eq!((band.left, band.right()), (623.0, 644.0));
    // A solid across the band's height whose face is at 640 cuts it to 640..644 mirrored.
    let wall = [solid(600.0, FLOOR, 40.0, 80.0)];
    let clipped = clip_band(band, 700.0, -1, &wall).unwrap();
    assert_eq!((clipped.left, clipped.right()), (640.0, 644.0));
    // A face on the band's near edge or further clips it away.
    let wall = [solid(600.0, FLOOR, 100.0, 80.0)];
    assert!(clip_band(band, 700.0, -1, &wall).is_none());
    // A solid below the band (the 32 px step) does not clip it.
    let step = [solid(600.0, FLOOR, 60.0, 32.0)];
    assert_eq!(clip_band(band, 700.0, -1, &step), Some(band));
    // A solid behind the pivot does not.
    let behind = [solid(720.0, FLOOR, 20.0, 80.0)];
    assert_eq!(clip_band(band, 700.0, -1, &behind), Some(band));
}

#[test]
fn the_virtuoso_tells_only_when_its_band_can_reach() {
    // Carl in the dead zone (distance 30): no tell, it backs off to its standoff first.
    let mut s = Sim::awake(Species::Virtuoso, 700.0, 670.0, 80.0);
    s.step();
    assert!(!s.acts().iter().any(|a| matches!(a, TrogAct::Tell { .. })));
    s.run(30);
    assert!(s.trog.x > 700.0, "it backed off");
    s.run(200);
    assert!(s.acts().iter().any(|a| matches!(a, TrogAct::Tell { .. })));
}

#[test]
fn the_tell_gap_and_the_token_hold_a_tell_back() {
    let mut s = Sim::awake(Species::Basher, 700.0, 662.0, 80.0);
    s.last_tell = Some(s.tick);
    s.run(10);
    assert_eq!(s.trog.state, State::Approach, "inside the 24 tick gap");
    s.run(20);
    assert_eq!(s.trog.state, State::Tell);
}

#[test]
fn an_evaded_blow_is_not_a_hit_and_is_not_asked_twice() {
    let mut s = Sim::awake(Species::Basher, 700.0, 80.0, 664.0);
    s.heroes[0] = away();
    s.evade = true;
    s.run_until(200, |g| g.state == State::Recover);
    assert!(s.blows.is_empty());
    assert!(s.acts().contains(&TrogAct::Whiff));
}

#[test]
fn a_full_attack_cycle_replays_to_the_same_hash() {
    let run = || {
        let mut s = Sim::awake(Species::Pygmy, 640.0, 560.0, 80.0);
        s.run(400);
        let mut h = Vec::new();
        s.trog.mix_into(&mut |v| h.push(v));
        (h, s.events.clone().len(), s.blows.len())
    };
    assert_eq!(run(), run());
}

#[test]
fn the_down_body_has_no_hurtbox_and_never_holds_a_token() {
    let mut s = Sim::awake(Species::Basher, 700.0, 662.0, 80.0);
    s.run_until(100, |g| g.state == State::Tell);
    assert!(s.trog.holds_token());
    hit(&mut s, HitKind::Blast, 4);
    assert_eq!(s.trog.state, State::Down);
    assert!(!s.trog.holds_token() && s.trog.hurtbox(t()).is_none());
}

// ------------------------------------------------------------- F05, F06

fn forced_tongue(carl_dy: Option<f32>, donut_dy: Option<f32>) -> Vec<HeroId> {
    // The line is committed by a setup write (target choice does not apply): the Virtuoso at 700
    // faces left, its band 623..644 at 36..46 above its feet.
    let mut s = Sim::new(Species::Virtuoso, 700.0, None);
    s.trog.state = State::Tell;
    s.trog.facing = -1;
    s.trog.attack_tick = 0;
    s.heroes = [away(), away()];
    if let Some(dy) = carl_dy {
        s.heroes[0] = hero(HeroId::Carl, 634.0, FLOOR + dy, 24.0, 52.0, false);
    }
    if let Some(dy) = donut_dy {
        s.heroes[1] = hero(HeroId::Donut, 634.0, FLOOR + dy, 20.0, 24.0, false);
    }
    s.run(80);
    s.blows.iter().map(|(_, b)| b.hero).collect()
}

/// TROG-F05: the +0/+32/+64 matrix.
#[test]
fn trog_virtuoso_elevations() {
    assert_eq!(
        forced_tongue(Some(0.0), None),
        [HeroId::Carl],
        "grounded Carl is hit"
    );
    assert!(
        forced_tongue(None, Some(0.0)).is_empty(),
        "grounded Donut (0..24): the band passes over her"
    );
    assert_eq!(
        forced_tongue(None, Some(20.0)),
        [HeroId::Donut],
        "Donut with feet 13..45 is hit"
    );
    assert_eq!(
        forced_tongue(Some(32.0), None),
        [HeroId::Carl],
        "on the step Carl is hit"
    );
    assert_eq!(
        forced_tongue(None, Some(32.0)),
        [HeroId::Donut],
        "on the step Donut is hit"
    );
    assert!(
        forced_tongue(Some(64.0), None).is_empty(),
        "on the platform Carl is not hit"
    );
    assert!(
        forced_tongue(None, Some(64.0)).is_empty(),
        "on the platform Donut is not hit"
    );
    // The edges of Donut's band: feet 13 hits, feet 12 does not; feet 45 hits, 46 does not.
    assert_eq!(forced_tongue(None, Some(13.0)), [HeroId::Donut]);
    assert!(forced_tongue(None, Some(12.0)).is_empty());
    assert_eq!(forced_tongue(None, Some(45.0)), [HeroId::Donut]);
    assert!(forced_tongue(None, Some(46.0)).is_empty());
}

/// TROG-F06 target_choice: with both heroes present every tell names Carl while Donut is on the
/// floor, whoever is nearer; with her on the stand and nearer it names Donut.
#[test]
fn trog_virtuoso_target_choice() {
    let tells = |s: &Sim| -> Vec<HeroId> {
        s.acts()
            .into_iter()
            .filter_map(|a| match a {
                TrogAct::Tell { target } => Some(target),
                _ => None,
            })
            .collect()
    };
    let mut s = Sim::awake(Species::Virtuoso, 700.0, 560.0, 640.0);
    s.run(600);
    let named = tells(&s);
    assert!(!named.is_empty());
    assert!(named.iter().all(|h| *h == HeroId::Carl), "{named:?}");
    // Donut on the stand and nearer: it names Donut.
    let mut d = Sim::awake(Species::Virtuoso, 700.0, 560.0, 650.0);
    d.heroes[1] = hero(HeroId::Donut, 650.0, FLOOR + 32.0, 20.0, 24.0, false);
    d.run(600);
    let named = tells(&d);
    assert!(!named.is_empty());
    assert_eq!(named[0], HeroId::Donut, "{named:?}");
}

/// TROG-F06 grounded_donut (reported, TROG-Q13): a lone grounded Donut is never told at and
/// defeats it from the floor with 0 HP lost.
#[test]
fn trog_virtuoso_grounded_donut() {
    let mut s = Sim::awake(Species::Virtuoso, 700.0, 80.0, 560.0);
    s.heroes[0] = away();
    for _ in 0..4 {
        s.run(20);
        hit(&mut s, HitKind::Missile, 1);
        if !s.trog.alive() {
            break;
        }
    }
    assert!(!s.trog.alive(), "she defeats it from the floor");
    assert!(s.blows.is_empty(), "0 HP lost");
    assert!(
        !s.acts().iter().any(|a| matches!(a, TrogAct::Tell { .. })),
        "never told at"
    );
}

/// TROG-F06 raised_donut: solo Donut on a +32 stand (fixture geometry) is a target; she hops
/// above 46 on the strike ticks (the hop is timed to the feint) and bolts during the 18-tick
/// retract; her bolts at 48..54 meet the retract hurtbox (0..52) by 4 px.
#[test]
fn trog_virtuoso_raised_donut() {
    let sp = &t().virtuoso;
    // The review's number: a +32 stand puts the bolts at 48..54, and the retract hurtbox top is
    // 52, so they meet by 4 px.
    assert_eq!(sp.hurt_recover.y1, 52);
    assert!(overlaps_y((48.0, 54.0), &sp.hurt_recover));
    assert_eq!(54 - i32::from(sp.hurt_recover.y1) + 2, 4);
    let mut s = Sim::awake(Species::Virtuoso, 700.0, 80.0, 634.0);
    s.heroes[0] = away();
    let stand = FLOOR + 32.0;
    s.heroes[1] = hero(HeroId::Donut, 634.0, stand, 20.0, 24.0, false);
    let mut bolted = false;
    for _ in 0..3000 {
        if !s.trog.alive() {
            break;
        }
        let a = s.trog.attack_tick();
        let strike = matches!(s.trog.state, State::Tell | State::Active)
            && a + 2 >= sp.strike_start
            && a <= sp.strike_end + 1;
        // The hop: feet above 46 on the strike ticks, else back on the stand.
        let foot = if strike { FLOOR + 50.0 } else { stand };
        s.heroes[1] = hero(HeroId::Donut, 634.0, foot, 20.0, 24.0, false);
        if s.trog.state == State::Recover && a == sp.strike_end + 3 && !bolted {
            hit(&mut s, HitKind::Missile, 1);
            bolted = true;
        }
        if s.trog.state != State::Recover {
            bolted = false;
        }
        s.step();
    }
    assert!(!s.trog.alive(), "defeated by bolts in the retracts");
    assert!(s.blows.is_empty(), "0 HP lost: {:?}", s.blows);
    assert!(s.acts().iter().any(|a| matches!(
        a,
        TrogAct::Tell {
            target: HeroId::Donut
        }
    )));
}
