//! DONUT-PROG-01 DP-E fixtures: the autonomous Donut's escape blink (spec
//! section 9, DP-Q37 to DP-Q41, DP-Q46). Test harness only, in the shape of
//! DP-C's blink fixtures: a rig over the real combat scene in the CF02
//! microarena under `cf02_free` (the fixture names carry the `cf02_free_`
//! prefix, which is what turns `cf02_rules` on), exact tick asserts and a
//! committed final hash per fixture, each replayed twice. Not registered under
//! a CLI name and no committed replay file (the `cf02_free_*` registry and its
//! replays stay byte-identical).
//!
//! Tick numbering is the one of [`crate::combat_scene`]: the step that produces
//! `tick = n` is the n-th `step_slots` call, and an event carries the tick of
//! the step that logged it.

use crate::boss_scene::Pilot;
use crate::carl_prog::CarlProgFlags;
use crate::cf02::arena;
use crate::combat::{CombatEvent, EnemyAi};
use crate::combat_scene::{CombatState, EnemySpawn, Fixture};
use crate::corridor::SceneSim;
use crate::donut_mana::{DonutSetup, ManaEvent};
use crate::donut_prog::{DonutProgFlags, story_setup as program_setup};
use crate::input::RawInput;
use crate::party::{HeroId, PartyTuning};
use crate::party_scene::{PartyFixture, PartyFixtureSim};
use crate::story_unlock::{SceneKind, StorySetup};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[derive(Clone)]
struct Rig {
    name: &'static str,
    enemies: &'static [EnemySpawn],
    charges: u8,
    carl_x: i32,
    donut_x: f32,
    tuning: PartyTuning,
    /// Hold the blink (the program with `puddle`, which implies the boost), or
    /// nothing.
    blink: bool,
    /// The follow layer's jump-dodge radius (`PartyTuning::dodge_radius_px`).
    dodge_radius: u16,
    /// A room `blink_barriers` rectangle `(x, w)`, 100 px tall from the floor
    /// (never collision).
    barrier: Option<(i32, i32)>,
}

impl Rig {
    fn new(name: &'static str, enemies: &'static [EnemySpawn]) -> Self {
        Self {
            name,
            enemies,
            charges: 8,
            carl_x: 400,
            donut_x: 440.0,
            tuning: crate::cf02_free_fixtures::FREE,
            blink: true,
            dodge_radius: crate::party::PROVISIONAL_TUNING.dodge_radius_px,
            barrier: None,
        }
    }

    fn no_dodge(mut self) -> Self {
        self.dodge_radius = 0;
        self
    }

    fn barrier(mut self, x: i32, w: i32) -> Self {
        self.barrier = Some((x, w));
        self
    }

    fn control(mut self) -> Self {
        self.blink = false;
        self
    }

    /// FA-Q19 (T26): the profile without the armed attempt start, so a control
    /// that must be hit is not saved by the opener's armed reflex.
    fn unarmed(mut self) -> Self {
        self.tuning.donut_reflex = crate::agility::ReflexProfile::ShippedNoOpener;
        self
    }

    fn charges(mut self, charges: u8) -> Self {
        self.charges = charges;
        self
    }

    fn setup(&self) -> StorySetup {
        if self.blink {
            let puddle = DonutProgFlags {
                puddle: true,
                ..DonutProgFlags::OFF
            };
            program_setup(CarlProgFlags::OFF, puddle, SceneKind::Corridor)
        } else {
            StorySetup::NONE
        }
    }

    fn build(&self) -> PartyFixtureSim {
        let mut kit = arena::load_arena(&root()).unwrap();
        if let Some((x, w)) = self.barrier {
            kit.blink_barriers.push(crate::env_kit::IRect {
                x,
                y: arena::FLOOR_Y - 100,
                w,
                h: 100,
            });
        }
        let base: &'static Fixture = Box::leak(Box::new(Fixture {
            enemies: self.enemies,
            llamas: &[],
            shore_dx: None,
            carl_x: self.carl_x,
            ..arena::fixture(self.name, "DP-E AI blink rig", 0xD9E0_0001, 1000)
        }));
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base,
            two_player: false,
            tuning: PartyTuning {
                dodge_radius_px: self.dodge_radius,
                ..self.tuning
            },
            script: &[],
            pilot: Pilot::None,
            donut: DonutSetup::unlocked(self.charges),
        }));
        let scene = SceneSim::new_at(&kit, 360, self.carl_x).unwrap();
        let combat = CombatState::new(base, &kit, &scene)
            .unwrap()
            .with_donut(fixture.donut)
            .with_story(self.setup());
        let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
        sim.scene.world.donut_body.x = self.donut_x;
        sim.scene.world.sync_donut_for_tests();
        sim
    }
}

/// One scripted swing: on the step after `tick`, goblin `index` stands `dx` px
/// right of Donut at tell tick `at`, aimed at her, facing her.
#[derive(Clone, Copy)]
struct Swing {
    tick: u64,
    index: usize,
    dx: f32,
    at: u16,
}

/// Carl runs right on `from..to` (slot 0).
type CarlRun = Option<(u64, u64)>;

fn run(rig: &Rig, swings: &[Swing], carl: CarlRun, to: u64) -> PartyFixtureSim {
    run_with(rig, swings, carl, to, |_| {})
}

/// [`run`], calling `each` on the sim after every step.
fn run_with(
    rig: &Rig,
    swings: &[Swing],
    carl: CarlRun,
    to: u64,
    mut each: impl FnMut(&PartyFixtureSim),
) -> PartyFixtureSim {
    let mut sim = rig.build();
    while sim.scene.world.tick < to {
        let tick = sim.scene.world.tick;
        for sw in swings.iter().filter(|s| s.tick == tick) {
            let x = sim.scene.world.donut_body.x + sw.dx;
            sim.combat
                .test_stage_goblin_swing(sw.index, HeroId::Donut, x, -1, sw.at);
        }
        let carl_in = match carl {
            Some((from, to)) if (from..to).contains(&(tick + 1)) => RawInput {
                move_x: 1.0,
                ..RawInput::NONE
            },
            _ => RawInput::NONE,
        };
        sim.step_slots([carl_in, RawInput::NONE]);
        each(&sim);
    }
    sim
}

fn blink_events(sim: &PartyFixtureSim) -> Vec<CombatEvent> {
    sim.combat
        .events
        .iter()
        .copied()
        .filter(|e| {
            matches!(
                e,
                CombatEvent::SpecialStarted { hero: 1, .. }
                    | CombatEvent::SpecialRejected { hero: 1, .. }
                    | CombatEvent::BlinkResolved { .. }
                    | CombatEvent::BlinkFizzled { .. }
                    | CombatEvent::BlinkCancelled { .. }
            )
        })
        .collect()
}

fn donut_hurts(sim: &PartyFixtureSim) -> usize {
    sim.combat
        .events
        .iter()
        .filter(|e| {
            matches!(
                e,
                CombatEvent::HeroHurt {
                    hero: HeroId::Donut,
                    ..
                }
            )
        })
        .count()
}

fn mana(sim: &PartyFixtureSim) -> u8 {
    sim.combat.donut_mana_view().map_or(0, |m| m.0)
}

fn started(tick: u64) -> CombatEvent {
    CombatEvent::SpecialStarted {
        tick,
        hero: 1,
        capability: "special_short_blink",
    }
}

fn resolved(tick: u64, from_x: i32, to_x: i32) -> CombatEvent {
    CombatEvent::BlinkResolved {
        tick,
        hero: 1,
        from_x,
        to_x,
    }
}

static ONE_PASSIVE: [EnemySpawn; 1] = [EnemySpawn {
    x: 1000,
    ai: EnemyAi::Passive,
}];

/// The staged swing: stood at tick 10, tell tick 0 on the first step after.
const SWING30: Swing = Swing {
    tick: 10,
    index: 0,
    dx: 30.0,
    at: 0,
};

/// A swing already in its tell when she sees it: tell tick 18 (impact 8 ticks
/// out), the goblin 8 px from her. Walking away at her run speed cannot clear
/// the 30 px swing box in the 8 ticks left, so only the blink saves her.
const SWING_CLOSE: Swing = Swing {
    tick: 10,
    index: 0,
    dx: 8.0,
    at: 18,
};

// ------------------------------------------------------------------ fixtures --

/// `cf02_free_blink_escape`: a goblin 30 px away begins its tell aimed at her;
/// the follow layer's jump dodge is off (radius 0), so nothing clears it. On
/// the first tick she sees it (tell tick 18, impact 8 ticks out: more than the
/// windup, inside windup + 6) she presses the blink (tick 11), arrives six ticks
/// later (tick 17, tell tick 24) and the swing at tell tick 26 finds no one.
fn escape() -> PartyFixtureSim {
    run(
        &Rig::new("cf02_free_blink_escape", &ONE_PASSIVE).no_dodge(),
        &[SWING_CLOSE],
        None,
        90,
    )
}

/// The control of the escape: the same swing, the blink not held. She is hurt.
fn escape_control() -> PartyFixtureSim {
    run(
        &Rig::new("cf02_free_blink_escape", &ONE_PASSIVE)
            .no_dodge()
            .control()
            .unarmed(),
        &[SWING_CLOSE],
        None,
        90,
    )
}

/// `cf02_free_blink_jump_dodge_wins`: the same swing with the dodge on: the
/// follow layer's jump clears it and the blink is never pressed (DP-Q39 b).
fn jump_dodge_wins() -> PartyFixtureSim {
    run(
        &Rig::new("cf02_free_blink_jump_dodge_wins", &ONE_PASSIVE),
        &[SWING30],
        None,
        90,
    )
}

static ONE_PASSIVE_NEAR: [EnemySpawn; 1] = [EnemySpawn {
    x: 500,
    ai: EnemyAi::Passive,
}];

/// `cf02_free_blink_not_offense`: a standing goblin she can reach (60 px) and
/// that never attacks: she works it (a missile, a pounce, a swat) and never
/// presses the blink, with the capability held and eight charges.
fn not_offense() -> PartyFixtureSim {
    run(
        &Rig::new("cf02_free_blink_not_offense", &ONE_PASSIVE_NEAR),
        &[],
        None,
        400,
    )
}

/// `cf02_free_blink_reserve`: she holds exactly the reserve (2 charges, what a
/// cast-down leaves, see [`cast_down_stops_at_the_reserve`]) and the blink
/// still fires, paid from the reserve: 2 charges to 0.
fn reserve() -> PartyFixtureSim {
    run(
        &Rig::new("cf02_free_blink_reserve", &ONE_PASSIVE)
            .no_dodge()
            .charges(2),
        &[SWING30],
        None,
        90,
    )
}

static ONE_PASSIVE_IN_RANGE: [EnemySpawn; 1] = [EnemySpawn {
    x: 580,
    ai: EnemyAi::Passive,
}];

/// The director's cast reserve (CA2: it casts down to 2 and keeps them): from
/// four charges, with a goblin in the missile band, she stops at 2.
fn cast_down_stops_at_the_reserve() -> PartyFixtureSim {
    run(
        &Rig::new("cf02_free_blink_reserve", &ONE_PASSIVE_IN_RANGE).charges(4),
        &[],
        None,
        25,
    )
}

type Fixture0 = (&'static str, fn() -> PartyFixtureSim);

/// Every fixture with its name, in the committed order.
const FIXTURES: [Fixture0; 4] = [
    ("cf02_free_blink_escape", escape),
    ("cf02_free_blink_jump_dodge_wins", jump_dodge_wins),
    ("cf02_free_blink_not_offense", not_offense),
    ("cf02_free_blink_reserve", reserve),
];

/// The committed final hashes (`PartyFixtureSim::state_hash`).
const GOLDENS: [u64; 4] = [
    0xe0c6ba28a040bf88,
    0x532db55c2b81aaa2,
    0xd8f21303ceefa993,
    0x8253563539477545,
];

#[test]
#[ignore = "prints the goldens"]
fn print_ai_blink_goldens() {
    for (name, f) in FIXTURES {
        println!("    // {name}\n    {:#018x},", f().state_hash());
    }
}

#[test]
fn every_ai_blink_fixture_replays_twice_and_matches_its_golden() {
    for ((name, f), golden) in FIXTURES.iter().zip(GOLDENS) {
        let (a, b) = (f(), f());
        assert_eq!(a.state_hash(), b.state_hash(), "{name} replays");
        assert_eq!(a.combat.events, b.combat.events, "{name}: same events");
        assert_eq!(a.state_hash(), golden, "{name}: {}", a.state_line());
    }
}

// ------------------------------------------------------------------- asserts --

#[test]
fn cf02_free_blink_escape() {
    let sim = escape();
    // Press on the first tick (tick 11), arrival on t6, 100 px away (the hull
    // of Carl's 400 less 63 caps the 112 px search at 100: 440 to 340).
    assert_eq!(
        blink_events(&sim),
        vec![started(11), resolved(17, 440, 340)]
    );
    assert_eq!(donut_hurts(&sim), 0, "the swing found no one");
    let paid: Vec<(u64, u8)> = sim
        .combat
        .events
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::Mana(ManaEvent::ManaSpent { tick, left, .. }) => Some((tick, left)),
            _ => None,
        })
        .collect();
    assert!(
        paid.iter().any(|(tick, _)| *tick == 17),
        "paid on the arrival: {paid:?}"
    );
    let state = sim.combat.autonomy_state(HeroId::Donut);
    assert_eq!(state.last_blink(), Some(11));
    assert_eq!(state.stats().blink_presses_committed, 1);
    assert_eq!(state.stats().blink_presses_retreat, 0);
    // It gained at least `blink_min_gain_px` from the goblin (448).
    let gain = (340.0f32 - 448.0).abs() - (440.0f32 - 448.0).abs();
    assert!(gain >= 64.0, "{gain}");
    // The control: the same swing without the blink hurts her.
    let control = escape_control();
    assert!(blink_events(&control).is_empty());
    assert!(donut_hurts(&control) >= 1, "the control is hit");
}

#[test]
fn cf02_free_blink_jump_dodge_wins() {
    let sim = jump_dodge_wins();
    assert!(blink_events(&sim).is_empty(), "{:?}", blink_events(&sim));
    assert_eq!(donut_hurts(&sim), 0);
    assert_eq!(mana(&sim), 8);
}

#[test]
fn cf02_free_blink_not_offense() {
    let sim = not_offense();
    assert!(
        blink_events(&sim).is_empty(),
        "no press, no rejection: {:?}",
        blink_events(&sim)
    );
    assert!(
        mana(&sim) < 8 || sim.combat.enemies[0].health < 2,
        "she worked it"
    );
    assert_eq!(sim.combat.autonomy_state(HeroId::Donut).last_blink(), None);
}

#[test]
fn cf02_free_blink_reserve() {
    let sim = reserve();
    let events = blink_events(&sim);
    assert_eq!(events, vec![started(25), resolved(31, 440, 340)]);
    assert_eq!(mana(&sim), 0, "the reserve is spent on the blink");
    assert_eq!(donut_hurts(&sim), 0);
    // The reserve is what a cast-down leaves.
    let down = cast_down_stops_at_the_reserve();
    assert_eq!(mana(&down), 2);
    assert!(blink_events(&down).is_empty());
}

/// A barrier right at her left side (a 12 px column at 410..422, her box left
/// edge is 428): every leftward candidate sweeps it, so the search finds
/// nothing, and she never presses (no press, no rejection). The same rig with
/// the barrier far away blinks.
#[test]
fn the_ai_blink_respects_blink_barriers() {
    let rig = Rig::new("cf02_free_blink_barrier", &ONE_PASSIVE).no_dodge();
    let blocked = run(&rig.clone().barrier(410, 12), &[SWING_CLOSE], None, 60);
    assert!(
        blink_events(&blocked).is_empty(),
        "{:?}",
        blink_events(&blocked)
    );
    assert_eq!(
        blocked.combat.autonomy_state(HeroId::Donut).last_blink(),
        None
    );
    let free = run(&rig.barrier(100, 12), &[SWING_CLOSE], None, 60);
    assert_eq!(blink_events(&free).len(), 2);
}

/// Her blink never walks her into a wall or past the world: against the left
/// world edge the leftward search has nowhere to land, so a swing from the
/// right is not blinked (no destination), and nothing is rejected at the gate.
#[test]
fn no_destination_means_no_press() {
    let mut rig = Rig::new("cf02_free_blink_edge", &ONE_PASSIVE).no_dodge();
    rig.carl_x = 60;
    rig.donut_x = 60.0;
    let sim = run(&rig, &[SWING_CLOSE], None, 60);
    assert!(blink_events(&sim).is_empty(), "{:?}", blink_events(&sim));
}

/// Mana and cooldown: with fewer than 2 charges she never presses (no
/// rejection either), and after a blink the 540-tick cooldown holds (a second
/// swing 100 ticks later is not blinked).
#[test]
fn no_mana_and_cooldown_mean_no_press() {
    let rig = Rig::new("cf02_free_blink_mana", &ONE_PASSIVE).no_dodge();
    let poor = run(&rig.clone().charges(1), &[SWING_CLOSE], None, 60);
    assert!(blink_events(&poor).is_empty());
    let again = Swing {
        tick: 110,
        ..SWING_CLOSE
    };
    let sim = run(&rig, &[SWING_CLOSE, again], None, 200);
    let events = blink_events(&sim);
    assert_eq!(events.len(), 2, "one blink only: {events:?}");
}

/// The goal layer does not deadlock: after the blink she keeps acting (the
/// recovery ends, the goal layer steers, she follows Carl who walks on), and
/// the blink's lock never outlasts its 13 ticks.
#[test]
fn the_goal_layer_resumes_after_the_blink() {
    let rig = Rig::new("cf02_free_blink_resume", &ONE_PASSIVE).no_dodge();
    let mut locked_ticks = 0u32;
    let mut sim = run_with(&rig, &[SWING_CLOSE], Some((40, 400)), 400, |sim| {
        let tick = sim.scene.world.tick;
        if sim.combat.donut_blink().windup().is_some() || sim.combat.donut_blink().recovering(tick)
        {
            locked_ticks += 1;
        }
    });
    // 6 windup ticks (11..16), arrival on 17, 6 recovery ticks: 13 ticks.
    assert!((12..=13).contains(&locked_ticks), "{locked_ticks}");
    let carl = sim.scene.world.hero.x;
    let donut = sim.scene.world.donut_body.x;
    assert!(
        (carl - donut).abs() < 260.0 && carl > 450.0,
        "she kept up with Carl: carl {carl:.0} donut {donut:.0}"
    );
    sim.step_slots([RawInput::NONE; 2]);
}

/// M8 with blinks (DP-Q46): her body is inside the automatic camera's drawn
/// view on every tick of every AI blink fixture, at the 640 and 960 rest widths
/// and both aspects, with Carl standing and with Carl walking away from the
/// swing (the case the hull rule is for).
#[test]
fn donut_prog_blink_drawn_view() {
    use crate::camera_lab::camera_op::Aspect;
    use crate::scene_camera::{
        CameraChoice, CameraInterest, RigConfig, SceneCameraRig, llama_interest,
    };
    let rigs: [(Rig, Vec<Swing>, CarlRun); 4] = [
        (
            Rig::new("cf02_free_blink_escape", &ONE_PASSIVE).no_dodge(),
            vec![SWING_CLOSE],
            None,
        ),
        (
            Rig::new("cf02_free_blink_escape", &ONE_PASSIVE).no_dodge(),
            vec![SWING_CLOSE],
            Some((11, 200)),
        ),
        (
            Rig::new("cf02_free_blink_reserve", &ONE_PASSIVE)
                .no_dodge()
                .charges(2),
            vec![SWING30],
            Some((20, 200)),
        ),
        (
            Rig::new("cf02_free_blink_escape", &ONE_PASSIVE).no_dodge(),
            vec![SWING_CLOSE],
            Some((16, 200)),
        ),
    ];
    let mut blinks = 0;
    for (rig, swings, carl) in &rigs {
        for (aspect, out_w) in [
            (Aspect::Wide16x9, 1280),
            (Aspect::Wide16x9, 1920),
            (Aspect::Deck16x10, 1280),
            (Aspect::Deck16x10, 1920),
        ] {
            let kit = arena::load_arena(&root()).unwrap();
            let mut camera: Option<SceneCameraRig> = None;
            let mut interest = CameraInterest::default();
            let sim = run_with(rig, swings, *carl, 160, |sim| {
                interest.clear();
                llama_interest(&sim.scene, &sim.combat, &mut interest);
                let cfg = RigConfig {
                    choice: CameraChoice::Auto,
                    aspect,
                    out_w,
                    bounds: crate::camera_lab::camera_op::Bounds {
                        w: kit.width,
                        h: kit.height,
                    },
                };
                let rig_ref = camera.get_or_insert_with(|| {
                    SceneCameraRig::new(cfg, &sim.scene, Some(&sim.combat), &interest, 0).unwrap()
                });
                let v = rig_ref.sync(&sim.scene, Some(&sim.combat), &interest, sim.combat.elapsed);
                let x = sim.scene.world.donut_body.x;
                assert!(
                    x - 12.0 >= v.left as f32 && x + 12.0 <= v.right() as f32,
                    "{} {aspect:?} out {out_w}: tick {} her x {x:.1} outside {v:?}",
                    rig.name,
                    sim.scene.world.tick
                );
            });
            blinks += blink_events(&sim)
                .iter()
                .filter(|e| matches!(e, CombatEvent::BlinkResolved { .. }))
                .count();
        }
    }
    assert!(blinks >= 16, "every run blinked: {blinks}");
}

/// DP-Q38 (CA2 Revision 3) with the boost held: the blink story implies the
/// boost, so her ordinary jump launches at 547 px/s, the apex grows by 23.03 px
/// and the jump-edge rise limit the autonomy layer feeds the nav graph
/// (`jump_edge_rise_limit_px(apex)`) shrinks from about 137 to about 114 px: a
/// 128 px anchor is under the 137 px limit without the boost and over the
/// 114 px limit with it (a drop edge is how she leaves it).
#[test]
fn the_rise_limit_follows_the_boosted_apex() {
    use crate::agility::discrete_apex_px;
    use crate::companion::steer::jump_edge_rise_limit_px;
    let limit_of = |rig: &Rig| {
        let mut sim = rig.build();
        sim.step_slots([RawInput::NONE; 2]);
        jump_edge_rise_limit_px(discrete_apex_px(sim.scene.donut_motion_now().jump_speed))
    };
    let rig = Rig::new("cf02_free_blink_apex", &ONE_PASSIVE);
    let boosted = limit_of(&rig);
    let plain = limit_of(&rig.clone().control());
    assert!((boosted - 114.0).abs() < 0.2, "{boosted}");
    assert!((plain - 137.0).abs() < 0.2, "{plain}");
    assert!(128.0 <= plain && 128.0 > boosted);
}
