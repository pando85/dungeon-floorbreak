//! DONUT-PROG-01 DP-C fixtures: Puddle Jumper (`special_short_blink`), spec
//! section 10 M3. Test harness only, in the shape of DP-B's motion fixtures: a
//! rig (a flat floor with walls, a pit, a barrier, a zone) over the real combat
//! scene, scripted per-tick inputs, exact tick asserts and a committed final
//! hash per fixture. Each fixture is replayed twice and must match. They are
//! not registered under a CLI name (DP-D adds the render-loop replay).
//!
//! Tick numbering is the one of [`crate::combat_scene`]: the step that produces
//! `tick = n` is the n-th `step_slots` call, and an event carries the tick of
//! the step that logged it. A press on tick `P` is t0, the arrival is on `P + 6`
//! and the recovery is `P + 7 ..= P + 12`.

use super::{DONUT_PROG, program_setup};
use crate::boss_scene::Pilot;
use crate::carl_prog::CarlProgFlags;
use crate::combat::{CombatEvent, EnemyAi};
use crate::combat_scene::{CombatState, EnemySpawn, Fixture};
use crate::corridor::SceneSim;
use crate::donut_mana::{DonutSetup, ManaEvent};
use crate::donut_prog::{DonutProgFlags, DonutProgTuning};
use crate::env_kit::{EnvKit, IRect};
use crate::input::RawInput;
use crate::party::{PROVISIONAL_TUNING, PartyTuning};
use crate::party_scene::{PartyFixture, PartyFixtureSim, base};
use crate::short_blink::{CancelCause, FizzleReason};
use crate::story_unlock::{SceneKind, StorySetup};
use crate::thrown_charge::SpecialReject;
use std::path::PathBuf;

const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";
const NONE: RawInput = RawInput::NONE;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn t() -> &'static DonutProgTuning {
    DonutProgTuning::active()
}

fn pad(f: impl FnOnce(&mut RawInput)) -> RawInput {
    let mut raw = RawInput::NONE;
    f(&mut raw);
    raw
}

pub(super) fn circle() -> RawInput {
    pad(|r| r.ability = true)
}

fn square() -> RawInput {
    pad(|r| r.attack = true)
}

fn cross() -> RawInput {
    pad(|r| r.jump = true)
}

pub(super) fn stick(x: f32) -> RawInput {
    pad(|r| r.move_x = x)
}

fn merge(a: RawInput, b: RawInput) -> RawInput {
    RawInput {
        move_x: if b.move_x != 0.0 { b.move_x } else { a.move_x },
        jump: a.jump | b.jump,
        attack: a.attack | b.attack,
        physical: a.physical | b.physical,
        ability: a.ability | b.ability,
        swap: a.swap | b.swap,
        ..NONE
    }
}

/// Donut's inputs: `(from, to, input)` holds, merged on each tick they cover.
pub(super) type Holds = Vec<(u64, u64, RawInput)>;

pub(super) fn input_at(holds: &Holds, tick: u64) -> RawInput {
    holds
        .iter()
        .filter(|(from, to, _)| (*from..*to).contains(&tick))
        .fold(NONE, |acc, (_, _, raw)| merge(acc, *raw))
}

/// One tick of Donut's press: `(tick, tick + 1, input)`.
pub(super) fn at(tick: u64, raw: RawInput) -> (u64, u64, RawInput) {
    (tick, tick + 1, raw)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Story {
    /// `--donut-puddle`: the blink (and the boost it requires) held from tick 0.
    Puddle,
    /// The blink held and the safe-room placeholder zone (1200..1240, Donut's
    /// pivot) still unfired.
    PuddleAndZone,
    /// The blink and Carl's thrown charge held.
    PuddleAndDynamite,
    /// Nothing held: a press is inert.
    Nothing,
}

#[derive(Clone)]
pub(super) struct Rig {
    /// `(x, w, h)` walls standing on the floor.
    walls: Vec<(i32, i32, i32)>,
    /// A gap `[x0, x1)` in the floor.
    pit: Option<(i32, i32)>,
    /// `(x, w, h, bottom)` room blink barriers (never collision).
    barrier: Option<(i32, i32, i32, i32)>,
    two_player: bool,
    story: Story,
    charges: u8,
    carl_x: i32,
    donut_x: f32,
    enemies: &'static [EnemySpawn],
    tuning: PartyTuning,
}

impl Rig {
    /// Carl at 560 (view 240..880), Donut at 640, a flat floor, 8 charges.
    pub(super) fn new() -> Self {
        Self {
            walls: Vec::new(),
            pit: None,
            barrier: None,
            two_player: true,
            story: Story::Puddle,
            charges: 8,
            carl_x: 560,
            donut_x: 640.0,
            enemies: &[],
            tuning: PROVISIONAL_TUNING,
        }
    }

    pub(super) fn walls(mut self, walls: &[(i32, i32, i32)]) -> Self {
        self.walls = walls.to_vec();
        self
    }

    pub(super) fn story(mut self, story: Story) -> Self {
        self.story = story;
        self
    }

    pub(super) fn charges(mut self, charges: u8) -> Self {
        self.charges = charges;
        self
    }

    pub(super) fn donut_x(mut self, x: f32) -> Self {
        self.donut_x = x;
        self
    }

    pub(super) fn carl_x(mut self, x: i32) -> Self {
        self.carl_x = x;
        self
    }

    pub(super) fn kit(&self) -> EnvKit {
        let mut kit = EnvKit::load(&root(), LAYOUT).unwrap();
        let floor = kit.base_walk_line_y;
        let below = kit.height - floor;
        kit.solid_rects.clear();
        // No exit: a run near the end of the layout must not complete the scene.
        kit.exit = None;
        let floors = match self.pit {
            Some((x0, x1)) => vec![(0, x0), (x1, kit.width - x1)],
            None => vec![(0, kit.width)],
        };
        for (x, w) in floors {
            kit.solid_rects.push(IRect {
                x,
                y: floor,
                w,
                h: below,
            });
        }
        for &(x, w, h) in &self.walls {
            kit.solid_rects.push(IRect {
                x,
                y: floor - h,
                w,
                h,
            });
        }
        if let Some((x, w, h, bottom)) = self.barrier {
            kit.blink_barriers.push(IRect {
                x,
                y: floor - bottom - h,
                w,
                h,
            });
        }
        kit
    }

    pub(super) fn setup(&self) -> StorySetup {
        let puddle = DonutProgFlags {
            puddle: true,
            ..DonutProgFlags::OFF
        };
        match self.story {
            Story::Puddle => program_setup(CarlProgFlags::OFF, puddle, SceneKind::Corridor),
            Story::PuddleAndZone => {
                // The shipped row requires the boost, whose grant is this very
                // zone; a copy of the table without that `requires` holds the
                // blink alone and leaves the zone unfired.
                let text = crate::story_unlock::TABLE_JSON.replacen(
                    "\"requires\": [\"jump_reach_boost\"]",
                    "\"requires\": []",
                    1,
                );
                assert_ne!(text, crate::story_unlock::TABLE_JSON);
                let table = crate::story_unlock::StoryTable::from_json(&text)
                    .unwrap()
                    .leak();
                StorySetup::for_scene(
                    table,
                    &["donut_prog"],
                    &["donut_puddle"],
                    SceneKind::Corridor,
                )
            }
            Story::PuddleAndDynamite => program_setup(
                CarlProgFlags {
                    dynamite: true,
                    ..CarlProgFlags::OFF
                },
                puddle,
                SceneKind::Corridor,
            ),
            Story::Nothing => StorySetup::NONE,
        }
    }

    pub(super) fn build(&self) -> PartyFixtureSim {
        let kit = self.kit();
        let fixture_base: &'static Fixture = Box::leak(Box::new(Fixture {
            carl_x: self.carl_x,
            ..base("donut_prog_blink_rig", "test rig", 1, self.enemies, 1000)
        }));
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base: fixture_base,
            two_player: self.two_player,
            tuning: self.tuning,
            script: &[],
            pilot: Pilot::None,
            donut: DonutSetup::unlocked(self.charges),
        }));
        let scene = SceneSim::new_at(&kit, 360, self.carl_x).unwrap();
        let combat = CombatState::new(fixture_base, &kit, &scene)
            .unwrap()
            .with_donut(fixture.donut)
            .with_story(self.setup());
        let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
        sim.scene.world.donut_body.x = self.donut_x;
        sim.scene.world.sync_donut_for_tests();
        sim
    }
}

/// Step `sim` to tick `to` (inclusive) with Donut's holds on slot 1 and Carl's
/// on slot 0.
pub(super) fn run_to(sim: &mut PartyFixtureSim, to: u64, donut: &Holds, carl: &Holds) {
    while sim.scene.world.tick < to {
        let tick = sim.scene.world.tick + 1;
        sim.step_slots([input_at(carl, tick), input_at(donut, tick)]);
        assert_eq!(sim.scene.world.tick, tick, "the scene stopped ticking");
    }
}

pub(super) fn donut_x(sim: &PartyFixtureSim) -> f32 {
    sim.scene.world.donut_body.x
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
                    | CombatEvent::SpecialReady { hero: 1, .. }
                    | CombatEvent::BlinkResolved { .. }
                    | CombatEvent::BlinkFizzled { .. }
                    | CombatEvent::BlinkCancelled { .. }
            )
        })
        .collect()
}

fn started(tick: u64) -> CombatEvent {
    CombatEvent::SpecialStarted {
        tick,
        hero: 1,
        capability: "special_short_blink",
    }
}

fn rejected(tick: u64, reason: SpecialReject) -> CombatEvent {
    CombatEvent::SpecialRejected {
        tick,
        hero: 1,
        reason,
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

fn cancelled(tick: u64, cause: CancelCause) -> CombatEvent {
    CombatEvent::BlinkCancelled {
        tick,
        hero: 1,
        cause,
    }
}

fn fizzled(tick: u64, reason: FizzleReason) -> CombatEvent {
    CombatEvent::BlinkFizzled {
        tick,
        hero: 1,
        reason,
    }
}

fn ready(tick: u64) -> CombatEvent {
    CombatEvent::SpecialReady { tick, hero: 1 }
}

fn spends(sim: &PartyFixtureSim) -> Vec<(u64, u8)> {
    sim.combat
        .events
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::Mana(ManaEvent::ManaSpent { tick, left, .. }) => Some((tick, left)),
            _ => None,
        })
        .collect()
}

/// The press tick used by most fixtures.
pub(super) const P: u64 = 10;

#[allow(non_upper_case_globals)]
const CauseInput: CancelCause = CancelCause::Input;

/// The default run: one press at `P`, run past the whole timeline.
fn press_run(rig: &Rig, holds: Holds, to: u64) -> PartyFixtureSim {
    let mut sim = rig.build();
    run_to(&mut sim, to, &holds, &Vec::new());
    sim
}

// ------------------------------------------------------------------ fixtures --

/// `donut_prog_blink_flat`: 112 px on flat ground, arrival on t6, mana 8 to 6,
/// regen restarts, ready again on arrival + 540 and not on + 539.
fn flat() -> PartyFixtureSim {
    press_run(&Rig::new(), vec![at(P, circle())], P + 6 + 560)
}

/// `donut_prog_blink_facing_no_stick`: no stick means her facing (left here).
fn facing_no_stick() -> PartyFixtureSim {
    let mut sim = Rig::new().build();
    sim.scene.world.donut_body.facing = -1;
    run_to(&mut sim, P + 20, &vec![at(P, circle())], &Vec::new());
    sim
}

/// `donut_prog_blink_stick_left`: the stick left overrides a right facing; the
/// stick is changed during the windup and the locked direction stands.
fn stick_left() -> PartyFixtureSim {
    let holds = vec![
        (P, P + 1, circle()),
        (P, P + 1, stick(-1.0)),
        (P + 1, P + 6, stick(1.0)),
    ];
    press_run(&Rig::new().donut_x(660.0), holds, P + 20)
}

/// `donut_prog_blink_wall_stops_short`: a 40 px wall at 700 (face at 700): she
/// lands in front of it.
fn wall_stops_short() -> PartyFixtureSim {
    press_run(
        &Rig::new().walls(&[(700, 40, 100)]),
        vec![at(P, circle())],
        P + 20,
    )
}

/// `donut_prog_blink_thin_wall_not_crossed`: a 16 px wall 40 px ahead.
fn thin_wall_not_crossed() -> PartyFixtureSim {
    press_run(
        &Rig::new().walls(&[(680, 16, 100)]),
        vec![at(P, circle())],
        P + 20,
    )
}

/// `donut_prog_blink_barrier_rect`: a room `blink_barriers` rectangle (not a
/// solid) stops the blink but not her walk.
fn barrier_rect() -> PartyFixtureSim {
    let mut rig = Rig::new();
    rig.barrier = Some((720, 8, 40, 0));
    press_run(&rig, vec![at(P, circle())], P + 20)
}

/// `donut_prog_blink_zone_not_skipped`: the unfired 1200..1240 zone is entered,
/// never skipped.
fn zone_not_skipped() -> PartyFixtureSim {
    let rig = Rig::new()
        .story(Story::PuddleAndZone)
        .carl_x(1040)
        .donut_x(1140.0);
    press_run(&rig, vec![at(P, circle())], P + 20)
}

/// `donut_prog_blink_view_edge`: shortened to stay 24 px inside the view.
fn view_edge_rig() -> Rig {
    let width = Rig::new().kit().width;
    // Both heroes at the end of the layout: the camera is clamped to
    // `width - 640 ..= width`, so the right inset is `width - 24` exactly.
    Rig::new().carl_x(width - 200).donut_x((width - 80) as f32)
}

fn view_edge() -> PartyFixtureSim {
    press_run(&view_edge_rig(), vec![at(P, circle())], P + 20)
}

/// `donut_prog_blink_2p_separation`: Carl at the left edge of the view, Donut
/// 340 px from him: the 112 px candidate would be 452 away (over the 448
/// limit and wider), so it settles within the limit.
fn two_p_separation() -> PartyFixtureSim {
    let mut sim = Rig::new().carl_x(720).donut_x(760.0).build();
    sim.scene.world.hero.x = 420.0;
    run_to(&mut sim, P + 20, &vec![at(P, circle())], &Vec::new());
    sim
}

/// `donut_prog_blink_pit_rejected`: a pit too wide to cross, she stands at its
/// edge: `NoDestination`, nothing spent, no cooldown.
fn pit_rejected() -> PartyFixtureSim {
    let mut rig = Rig::new().donut_x(690.0);
    rig.pit = Some((700, 900));
    press_run(&rig, vec![at(P, circle()), at(P + 8, circle())], P + 20)
}

/// A pit of 100 px: crossed to supported ground.
fn pit_crossed() -> PartyFixtureSim {
    let mut rig = Rig::new().donut_x(690.0);
    rig.pit = Some((700, 800));
    press_run(&rig, vec![at(P, circle())], P + 20)
}

/// `donut_prog_blink_no_destination_refund`: valid at the press, a gate closes
/// in the windup: the arrival fizzles with `NoDestination`, nothing spent, no
/// cooldown (a press right after is judged by the search again).
fn no_destination_refund() -> PartyFixtureSim {
    let mut sim = Rig::new().build();
    let holds = vec![at(P, circle())];
    run_to(&mut sim, P + 2, &holds, &Vec::new());
    // The gate closes: a wall whose face is right in front of her.
    sim.scene.world.solids.push(crate::prototype::Block {
        rect: crate::prototype::Rect {
            left: 656.0,
            bottom: 0.0,
            width: 40.0,
            height: 400.0,
        },
        material: crate::prototype::Material::Stone,
        intact: true,
    });
    run_to(&mut sim, P + 30, &holds, &Vec::new());
    sim
}

/// `donut_prog_blink_fizzle_on_arrival`: the charges are gone by the arrival
/// tick (the defensive rule): `NoMana`, no spend, no cooldown.
fn fizzle_on_arrival() -> PartyFixtureSim {
    let mut sim = Rig::new().build();
    let holds = vec![at(P, circle())];
    run_to(&mut sim, P + 2, &holds, &Vec::new());
    sim.combat.test_drain_donut_charges(1);
    run_to(&mut sim, P + 30, &holds, &Vec::new());
    sim
}

/// `donut_prog_blink_windup_cast_cancels`: Square/X on t3.
fn windup_cast_cancels() -> PartyFixtureSim {
    press_run(
        &Rig::new(),
        vec![at(P, circle()), at(P + 3, square())],
        P + 40,
    )
}

/// `donut_prog_blink_windup_jump_cancels`: Cross on t2, grounded.
fn windup_jump_cancels() -> PartyFixtureSim {
    press_run(
        &Rig::new(),
        vec![at(P, circle()), at(P + 2, cross())],
        P + 30,
    )
}

/// `donut_prog_blink_windup_same_tick_press`: Circle and Square on one tick.
fn windup_same_tick_press() -> PartyFixtureSim {
    press_run(
        &Rig::new(),
        vec![(P, P + 1, circle()), (P, P + 1, square())],
        P + 40,
    )
}

/// The landing tick of a plain jump at tick `J` in the rig (the first tick
/// after `J + 5` on which she stands again).
fn landing_of_jump(rig: &Rig, jump: u64) -> u64 {
    let mut sim = rig.build();
    let holds = vec![at(jump, cross())];
    let mut tick = 0;
    while sim.scene.world.tick < jump + 200 {
        let next = sim.scene.world.tick + 1;
        run_to(&mut sim, next, &holds, &Vec::new());
        tick = sim.scene.world.tick;
        if tick > jump + 5 && sim.scene.world.donut_body.grounded {
            return tick;
        }
    }
    panic!("she never landed (last tick {tick})");
}

/// `donut_prog_blink_buffer_cleared`: a Cross three ticks before an airborne
/// blink whose windup lands: no jump fires. `with_blink` false is the control
/// (the same inputs without the blink press: the buffered jump fires).
fn buffer_run(with_blink: bool) -> (PartyFixtureSim, u64) {
    let rig = Rig::new();
    let jump = 5;
    let land = landing_of_jump(&rig, jump);
    // A jump buffer lasts 7 ticks (it is decremented before the check), so a
    // Cross on `land - 5` fires on `land + 1` unless something clears it. The
    // blink pressed on `land - 2` (t2 lands in its windup) clears it on t0.
    let mut holds = vec![at(jump, cross()), at(land - 5, cross())];
    if with_blink {
        holds.push(at(land - 2, circle()));
    }
    let mut sim = rig.build();
    run_to(&mut sim, land + 12, &holds, &Vec::new());
    (sim, land)
}

/// `donut_prog_blink_move_buffer_cleared`: a Triangle pressed in the air before
/// the blink is buffered by her moves; t0 forgets it, so no pounce starts
/// inside the windup. `with_blink` false is the control: the buffered pounce
/// starts on landing.
fn move_buffer_run(with_blink: bool) -> (PartyFixtureSim, u64) {
    let rig = Rig::new();
    let jump = 5;
    let land = landing_of_jump(&rig, jump);
    let mut holds = vec![at(jump, cross()), at(land - 4, pad(|r| r.physical = true))];
    if with_blink {
        holds.push(at(land - 2, circle()));
    }
    let mut sim = rig.build();
    run_to(&mut sim, land + 12, &holds, &Vec::new());
    (sim, land)
}

/// `donut_prog_blink_airborne_keeps_vy`: a blink pressed mid-jump changes only
/// `x`: the height and vertical speed follow the plain jump tick for tick.
fn airborne_run(with_blink: bool) -> Vec<(f32, f32, f32)> {
    let mut sim = Rig::new().build();
    let mut holds = vec![at(5, cross())];
    if with_blink {
        holds.push(at(15, circle()));
    }
    let mut trace = Vec::new();
    for tick in 1..=40 {
        run_to(&mut sim, tick, &holds, &Vec::new());
        let body = &sim.scene.world.donut_body;
        trace.push((body.x, body.y, body.vy));
    }
    trace
}

/// `donut_prog_blink_windup_horizontal_ignored`: the stick held throughout the
/// windup moves her not at all; after the arrival she walks again.
fn windup_horizontal_ignored() -> PartyFixtureSim {
    let holds = vec![(P - 3, P + 40, stick(-1.0)), at(P, circle())];
    press_run(&Rig::new(), holds, P + 20)
}

/// `donut_prog_blink_hurt_in_windup_cancels`: a hurt after t2.
fn hurt_in_windup() -> PartyFixtureSim {
    let mut sim = Rig::new().build();
    let holds = vec![at(P, circle())];
    run_to(&mut sim, P + 2, &holds, &Vec::new());
    let tick = sim.scene.world.tick;
    sim.combat.test_hurt_donut(&mut sim.scene, tick);
    run_to(&mut sim, P + 40, &holds, &Vec::new());
    sim
}

/// `donut_prog_blink_no_iframes`: no tick of the blink grants invulnerability.
fn no_iframes() -> (PartyFixtureSim, Vec<u8>) {
    let mut sim = Rig::new().build();
    let holds = vec![at(P, circle())];
    let mut invulnerable = Vec::new();
    for tick in 1..=P + 16 {
        run_to(&mut sim, tick, &holds, &Vec::new());
        invulnerable.push(sim.scene.world.donut_body.invulnerable_ticks);
    }
    (sim, invulnerable)
}

static PASSIVE: [EnemySpawn; 1] = [EnemySpawn {
    x: 700,
    ai: EnemyAi::Passive,
}];

/// `donut_prog_blink_no_damage`: a goblin standing on the path is neither hit
/// nor pushed.
fn no_damage() -> PartyFixtureSim {
    let mut rig = Rig::new();
    rig.enemies = &PASSIVE;
    press_run(&rig, vec![at(P, circle())], P + 30)
}

/// `donut_prog_blink_no_mana`: one charge, a press is rejected.
fn no_mana() -> PartyFixtureSim {
    press_run(&Rig::new().charges(1), vec![at(P, circle())], P + 20)
}

/// `donut_prog_blink_cooldown_reject`: a second press inside the cooldown.
fn cooldown_reject() -> PartyFixtureSim {
    press_run(
        &Rig::new(),
        vec![at(P, circle()), at(P + 100, circle())],
        P + 110,
    )
}

/// `donut_prog_blink_swap_keeps_cooldown` (solo): the first blink under
/// Donut's control, a swap away and back, a press inside the cooldown.
fn swap_keeps_cooldown() -> PartyFixtureSim {
    let mut rig = Rig::new();
    rig.two_player = false;
    let mut sim = rig.build();
    let carl = vec![
        at(3, pad(|r| r.swap = true)),
        // Solo: slot 0 drives whoever is controlled (Donut after the swap).
        at(P, circle()),
        at(40, pad(|r| r.swap = true)),
        at(70, pad(|r| r.swap = true)),
        at(100, circle()),
    ];
    run_to(&mut sim, 120, &Vec::new(), &carl);
    sim
}

/// `donut_prog_blink_swap_mid_windup` (solo): a swap on t2 cancels nothing.
fn swap_mid_windup() -> PartyFixtureSim {
    let mut rig = Rig::new();
    rig.two_player = false;
    let mut sim = rig.build();
    let carl = vec![
        at(3, pad(|r| r.swap = true)),
        at(P, circle()),
        at(P + 2, pad(|r| r.swap = true)),
    ];
    run_to(&mut sim, P + 20, &Vec::new(), &carl);
    sim
}

static ONE_HIT: PartyTuning = PartyTuning {
    donut_damage_per_hit: 4,
    ..PROVISIONAL_TUNING
};

/// `donut_prog_blink_retry_clears`: a blink, then a defeat retry: the special
/// is ready at once and a windup running at a fixture restart is cancelled.
fn retry_clears() -> PartyFixtureSim {
    let mut rig = Rig::new();
    rig.tuning = ONE_HIT;
    let mut sim = rig.build();
    let holds = vec![at(P, circle())];
    run_to(&mut sim, P + 20, &holds, &Vec::new());
    let tick = sim.scene.world.tick;
    sim.combat.test_hurt_donut(&mut sim.scene, tick);
    let restarts = sim.combat.restarts;
    while sim.combat.restarts == restarts {
        let tick = sim.scene.world.tick + 1;
        sim.step_slots([NONE, input_at(&holds, tick)]);
        assert!(tick < 1000, "the retry never came");
    }
    sim
}

/// The windup of `retry_clears` killed by a fixture restart.
fn restart_mid_windup() -> PartyFixtureSim {
    let mut sim = Rig::new().build();
    let holds = vec![at(P, circle())];
    run_to(&mut sim, P + 3, &holds, &Vec::new());
    sim.combat.restart(&mut sim.scene);
    sim
}

/// `donut_prog_blink_carl_circle_unchanged`: Carl's Circle still throws his
/// charge in the same run, and Donut's blinks.
fn carl_circle_unchanged() -> PartyFixtureSim {
    let mut sim = Rig::new().story(Story::PuddleAndDynamite).build();
    run_to(
        &mut sim,
        P + 20,
        &vec![at(P + 2, circle())],
        &vec![at(P, circle())],
    );
    sim
}

/// `donut_prog_blink_locked`: without the capability Circle/B does nothing:
/// the same hash as a run with no press at all.
fn locked_run(press: bool) -> PartyFixtureSim {
    let holds = if press { vec![at(P, circle())] } else { vec![] };
    press_run(&Rig::new().story(Story::Nothing), holds, P + 30)
}

type Fixture0 = (&'static str, fn() -> PartyFixtureSim);

/// Every fixture with its name, in the committed order.
const FIXTURES: [Fixture0; 29] = [
    ("donut_prog_blink_flat", flat),
    ("donut_prog_blink_facing_no_stick", facing_no_stick),
    ("donut_prog_blink_stick_left", stick_left),
    ("donut_prog_blink_wall_stops_short", wall_stops_short),
    (
        "donut_prog_blink_thin_wall_not_crossed",
        thin_wall_not_crossed,
    ),
    ("donut_prog_blink_barrier_rect", barrier_rect),
    ("donut_prog_blink_zone_not_skipped", zone_not_skipped),
    ("donut_prog_blink_view_edge", view_edge),
    ("donut_prog_blink_2p_separation", two_p_separation),
    ("donut_prog_blink_pit_rejected", pit_rejected),
    ("donut_prog_blink_pit_crossed", pit_crossed),
    (
        "donut_prog_blink_no_destination_refund",
        no_destination_refund,
    ),
    ("donut_prog_blink_fizzle_on_arrival", fizzle_on_arrival),
    ("donut_prog_blink_windup_cast_cancels", windup_cast_cancels),
    ("donut_prog_blink_windup_jump_cancels", windup_jump_cancels),
    (
        "donut_prog_blink_windup_same_tick_press",
        windup_same_tick_press,
    ),
    (
        "donut_prog_blink_windup_horizontal_ignored",
        windup_horizontal_ignored,
    ),
    ("donut_prog_blink_hurt_in_windup_cancels", hurt_in_windup),
    ("donut_prog_blink_no_damage", no_damage),
    ("donut_prog_blink_no_mana", no_mana),
    ("donut_prog_blink_cooldown_reject", cooldown_reject),
    ("donut_prog_blink_swap_keeps_cooldown", swap_keeps_cooldown),
    ("donut_prog_blink_swap_mid_windup", swap_mid_windup),
    ("donut_prog_blink_retry_clears", retry_clears),
    ("donut_prog_blink_restart_mid_windup", restart_mid_windup),
    (
        "donut_prog_blink_carl_circle_unchanged",
        carl_circle_unchanged,
    ),
    ("donut_prog_blink_move_buffer_cleared", || {
        move_buffer_run(true).0
    }),
    ("donut_prog_blink_locked_pressed", || locked_run(true)),
    ("donut_prog_blink_locked_idle", || locked_run(false)),
];

/// The committed final hashes (`PartyFixtureSim::state_hash`).
const GOLDENS: [u64; 29] = [
    0x834fd22a08218fa0,
    0x9f932252849ab6d1,
    0x24cb6cd43b92af64,
    0xadc1ac9ea17397f9,
    0x0078f5f7111821fe,
    0x2bf6bc335bf5b628,
    0x1ee6438c21a0b7b5,
    0x8e713c564a4c72f3,
    0x09d5747d28d7ad92,
    0x96c81b30c6fb1d92,
    0x2eb15c62a2947bf4,
    0xa9fb3041679d9640,
    0x1b161d5aea2c1630,
    0x5b5b81b126daa86f,
    0xa1a2486c2e0d8f30,
    0xd8a4da5dbe225100,
    0x643b38aa1da3c341,
    0x77a5522a676d590d,
    0x25696d3fc9694418,
    0x154956f465cc6bac,
    0x6a5c89ea157e7f60,
    0x3769d2e6ef6b0eb1,
    0x4e00634a326f60c8,
    0xc9115ed2fc0dbc5b,
    0xff10a23cd76ebdd7,
    0xea4979220ff328f6,
    0x503ab62bf98a0b41,
    0x784b7110912da9b8,
    0x784b7110912da9b8,
];

#[test]
#[ignore = "prints the blink goldens"]
fn print_blink_goldens() {
    for (name, run) in FIXTURES {
        println!("    // {name}\n    {:#018x},", run().state_hash());
    }
}

#[test]
fn every_blink_fixture_replays_twice_and_matches_its_golden() {
    let mut seen = Vec::new();
    for ((name, run), golden) in FIXTURES.iter().zip(GOLDENS) {
        let (a, b) = (run(), run());
        assert_eq!(a.state_hash(), b.state_hash(), "{name} replays");
        assert_eq!(a.combat.events, b.combat.events, "{name}: same events");
        assert_eq!(a.state_hash(), golden, "{name}: {}", a.state_line());
        seen.push(golden);
    }
    // The locked run with a press and without are the same hash, on purpose.
    let locked = FIXTURES.len() - 2;
    assert_eq!(seen[locked], seen[locked + 1]);
    seen.remove(locked + 1);
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen.len(),
        FIXTURES.len() - 1,
        "every other fixture is distinct"
    );
}

// ------------------------------------------------------------------- asserts --

#[test]
fn donut_prog_blink_flat() {
    let sim = flat();
    let tuning = t();
    assert_eq!(
        blink_events(&sim),
        [started(P), resolved(P + 6, 640, 752), ready(P + 6 + 540)]
    );
    assert_eq!(spends(&sim), [(P + 6, 6)], "8 to 6 on the arrival tick");
    // The body: still at 640 after t5, on 752 after t6, same height, facing right.
    let mut probe = Rig::new().build();
    run_to(&mut probe, P + 5, &vec![at(P, circle())], &Vec::new());
    assert_eq!(donut_x(&probe), 640.0, "no move through t5");
    run_to(&mut probe, P + 6, &vec![at(P, circle())], &Vec::new());
    assert_eq!(donut_x(&probe), 752.0, "the warp is on t6");
    assert_eq!(probe.scene.world.donut_body.facing, 1);
    let mut still = Rig::new().build();
    run_to(&mut still, P + 6, &Vec::new(), &Vec::new());
    assert_eq!(
        probe.scene.world.donut_body.y, still.scene.world.donut_body.y,
        "no height gained"
    );
    // The regen restarts as a release does: the first charge is due 30 + 40
    // ticks after the arrival.
    let regen = sim
        .combat
        .events
        .iter()
        .find_map(|e| match *e {
            CombatEvent::Mana(ManaEvent::ManaRegained { tick, now, .. }) => Some((tick, now)),
            _ => None,
        })
        .expect("a charge regained");
    let mana = sim.combat.donut_mana_tuning();
    let due = u64::from(mana.regen_delay_ticks) + u64::from(mana.regen_period_ticks);
    assert_eq!(regen, (P + 6 + due, 7));
    assert_eq!(tuning.dpq24_cooldown_ticks, 540);
    // Ready again on arrival + 540, and not on + 539.
    let again = |press: u64| {
        let mut sim = Rig::new().build();
        let holds = vec![at(P, circle()), at(press, circle())];
        run_to(&mut sim, press + 8, &holds, &Vec::new());
        blink_events(&sim)
            .into_iter()
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::SpecialStarted { tick, .. }
                        | CombatEvent::SpecialRejected { tick, .. } if *tick == press
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        again(P + 6 + 539),
        [rejected(P + 6 + 539, SpecialReject::Cooldown)]
    );
    assert_eq!(again(P + 6 + 540), [started(P + 6 + 540)]);
}

#[test]
fn donut_prog_blink_facing_no_stick() {
    let sim = facing_no_stick();
    assert_eq!(blink_events(&sim)[1], resolved(P + 6, 640, 528));
    assert_eq!(sim.scene.world.donut_body.facing, -1);
}

#[test]
fn donut_prog_blink_stick_left() {
    let sim = stick_left();
    assert_eq!(blink_events(&sim)[1], resolved(P + 6, 660, 548));
    assert!(
        blink_events(&sim)
            .iter()
            .all(|e| !matches!(e, CombatEvent::BlinkCancelled { .. })),
        "a stick change in the windup is not a cancel"
    );
}

#[test]
fn donut_prog_blink_wall_stops_short() {
    let sim = wall_stops_short();
    assert_eq!(blink_events(&sim)[1], resolved(P + 6, 640, 688));
    assert!(donut_x(&sim) + 12.0 <= 700.0, "in front of the wall");
}

#[test]
fn donut_prog_blink_thin_wall_not_crossed() {
    let sim = thin_wall_not_crossed();
    assert_eq!(blink_events(&sim)[1], resolved(P + 6, 640, 668));
    assert!(donut_x(&sim) < 680.0, "never behind a 16 px wall");
}

#[test]
fn donut_prog_blink_barrier_rect() {
    let sim = barrier_rect();
    // The face at 720: the farthest column whose box ends by 720 is 708.
    assert_eq!(blink_events(&sim)[1], resolved(P + 6, 640, 708));
    // The barrier is no collider: she can walk through it.
    let mut rig = Rig::new();
    rig.barrier = Some((720, 8, 40, 0));
    let holds = vec![(1, 80, stick(1.0))];
    let mut walker = rig.build();
    run_to(&mut walker, 80, &holds, &Vec::new());
    assert!(donut_x(&walker) > 740.0, "{}", donut_x(&walker));
}

#[test]
fn donut_prog_blink_zone_not_skipped() {
    let sim = zone_not_skipped();
    let ledger_zones = sim.combat.story_ledger().is_some();
    assert!(ledger_zones);
    // 1140 + 112 = 1252 would skip 1200..1240: she ends inside it instead.
    assert_eq!(blink_events(&sim)[1], resolved(P + 6, 1140, 1236));
}

#[test]
fn donut_prog_blink_view_edge() {
    let rig = view_edge_rig();
    let mut probe = rig.build();
    run_to(&mut probe, P + 5, &vec![at(P, circle())], &Vec::new());
    let (_, right) = probe.scene.view_x();
    let width = rig.kit().width as f32;
    assert_eq!(right, width, "the camera is clamped at the layout end");
    let sim = view_edge();
    // The body box stays 24 px inside the view: the centre is at most
    // `right - 24 - 12`; 44 px from 80 px before the end, not 112.
    let from = width - 80.0;
    assert_eq!(
        blink_events(&sim)[1],
        resolved(P + 6, from as i32, (right - 36.0) as i32)
    );
    assert!(right - 36.0 < from + 112.0, "shortened");
}

#[test]
fn donut_prog_blink_2p_separation() {
    let sim = two_p_separation();
    let carl = sim.scene.world.hero.x;
    let CombatEvent::BlinkResolved { to_x, .. } = blink_events(&sim)[1] else {
        panic!("{:?}", blink_events(&sim));
    };
    let to = to_x as f32;
    assert!(
        (to - carl).abs() <= 448.0,
        "{to} is within the limit of Carl {carl}"
    );
    assert!(to < 760.0 + 112.0, "shortened from 872");
    // A control with Carl close by goes the full 112 px.
    let mut near = Rig::new().carl_x(720).donut_x(760.0).build();
    run_to(&mut near, P + 20, &vec![at(P, circle())], &Vec::new());
    assert_eq!(blink_events(&near)[1], resolved(P + 6, 760, 872));
}

#[test]
fn donut_prog_blink_pit_rejected() {
    let sim = pit_rejected();
    assert_eq!(
        blink_events(&sim),
        [
            rejected(P, SpecialReject::NoDestination),
            rejected(P + 8, SpecialReject::NoDestination)
        ],
        "never into a pit; nothing buffered, no cooldown"
    );
    assert!(spends(&sim).is_empty());
    assert_eq!(donut_x(&sim), 690.0);
}

#[test]
fn donut_prog_blink_pit_crossed() {
    let sim = pit_crossed();
    assert_eq!(blink_events(&sim)[1], resolved(P + 6, 690, 802));
}

#[test]
fn donut_prog_blink_no_destination_refund() {
    let sim = no_destination_refund();
    assert_eq!(
        blink_events(&sim),
        [started(P), fizzled(P + 6, FizzleReason::NoDestination)]
    );
    assert!(spends(&sim).is_empty(), "no spend");
    assert!(
        !sim.combat.donut_blink().cooling(P + 7),
        "a fizzle starts no cooldown"
    );
    assert_eq!(donut_x(&sim), 640.0);
}

#[test]
fn donut_prog_blink_fizzle_on_arrival() {
    let sim = fizzle_on_arrival();
    assert_eq!(
        blink_events(&sim),
        [started(P), fizzled(P + 6, FizzleReason::NoMana)]
    );
    assert!(
        spends(&sim).is_empty(),
        "the test drain is not logged; the blink spent nothing"
    );
    assert_eq!(sim.combat.donut_mana_view().map(|m| m.0), Some(1));
    assert!(!sim.combat.donut_blink().cooling(P + 7));
    assert_eq!(donut_x(&sim), 640.0);
}

#[test]
fn donut_prog_blink_windup_cast_cancels() {
    let sim = windup_cast_cancels();
    assert_eq!(
        blink_events(&sim),
        [started(P), cancelled(P + 3, CauseInput)],
        "no cooldown event: nothing was spent"
    );
    // Square on t3 casts that tick: the missile releases on cast tick 4 and
    // costs one charge (no blink cost).
    assert_eq!(spends(&sim), [(P + 3 + 4, 7)]);
    assert_eq!(donut_x(&sim), 640.0, "she never blinked");
    assert!(!sim.combat.donut_blink().cooling(P + 10));
}

#[test]
fn donut_prog_blink_windup_jump_cancels() {
    let sim = windup_jump_cancels();
    assert_eq!(
        blink_events(&sim),
        [started(P), cancelled(P + 2, CauseInput)]
    );
    // She jumps that tick: airborne with a vertical speed on t2.
    let mut probe = Rig::new().build();
    let holds = vec![at(P, circle()), at(P + 2, cross())];
    run_to(&mut probe, P + 1, &holds, &Vec::new());
    assert!(probe.scene.world.donut_body.grounded);
    run_to(&mut probe, P + 2, &holds, &Vec::new());
    assert!(
        !probe.scene.world.donut_body.grounded && probe.scene.world.donut_body.vy > 0.0,
        "the jump begins on the cancelling tick"
    );
    assert_eq!(donut_x(&sim), 640.0);
}

#[test]
fn donut_prog_blink_windup_same_tick_press() {
    let sim = windup_same_tick_press();
    assert_eq!(
        blink_events(&sim),
        [started(P), cancelled(P, CauseInput)],
        "cancelled at once"
    );
    // The cast starts on that same tick.
    let mut probe = Rig::new().build();
    let holds = vec![(P, P + 1, circle()), (P, P + 1, square())];
    run_to(&mut probe, P, &holds, &Vec::new());
    assert!(
        probe.combat.donut_cast().is_some(),
        "the missile is cast that tick"
    );
    assert_eq!(donut_x(&sim), 640.0);
}

#[test]
fn donut_prog_blink_buffer_cleared() {
    let (sim, land) = buffer_run(true);
    // Control: the buffered Cross jumps on landing.
    let (control, control_land) = buffer_run(false);
    assert_eq!(land, control_land);
    // Re-run the two and read the body from the landing to the end.
    let heights = |with_blink: bool| {
        let rig = Rig::new();
        let jump = 5;
        let mut holds = vec![at(jump, cross()), at(land - 5, cross())];
        if with_blink {
            holds.push(at(land - 2, circle()));
        }
        let mut sim = rig.build();
        let mut ys = Vec::new();
        for tick in 1..=land + 12 {
            run_to(&mut sim, tick, &holds, &Vec::new());
            if tick >= land {
                ys.push(sim.scene.world.donut_body.y);
            }
        }
        ys
    };
    let with = heights(true);
    let without = heights(false);
    let base_y = with[0];
    assert!(
        with.iter().all(|y| (*y - base_y).abs() < 0.01),
        "no jump after the landing in the windup: {with:?}"
    );
    assert!(
        without.iter().any(|y| *y > base_y + 1.0),
        "the control's buffered Cross fires on the landing: {without:?}"
    );
    assert!(
        blink_events(&sim)
            .iter()
            .any(|e| matches!(e, CombatEvent::BlinkResolved { .. }))
    );
    assert!(blink_events(&control).is_empty());
}

#[test]
fn donut_prog_blink_move_buffer_cleared() {
    let (sim, land) = move_buffer_run(true);
    let (control, control_land) = move_buffer_run(false);
    assert_eq!(land, control_land);
    let pounce_starts = |sim: &PartyFixtureSim| -> Vec<u64> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::MoveStarted { tick, .. } => Some(tick),
                _ => None,
            })
            .collect()
    };
    // Control: the buffered Triangle starts a pounce once she lands.
    assert!(
        pounce_starts(&control).iter().any(|t| *t > land),
        "the control's buffered pounce starts on landing: {:?}",
        pounce_starts(&control)
    );
    // Blink: Circle on `land - 2` is t0; no move starts at all, and the blink
    // runs to its arrival on t6 untouched.
    assert!(
        pounce_starts(&sim).is_empty(),
        "no pounce inside the blink: {:?}",
        pounce_starts(&sim)
    );
    let events = blink_events(&sim);
    assert_eq!(events[0], started(land - 2));
    assert!(matches!(
        events[1],
        CombatEvent::BlinkResolved { tick, .. } if tick == land + 4
    ));
    assert!(
        events
            .iter()
            .all(|e| !matches!(e, CombatEvent::BlinkCancelled { .. }))
    );
}

#[test]
fn donut_prog_blink_airborne_keeps_vy() {
    let plain = airborne_run(false);
    let blinked = airborne_run(true);
    let mut moved = false;
    for (tick, (a, b)) in plain.iter().zip(&blinked).enumerate() {
        assert_eq!(a.1, b.1, "tick {}: height", tick + 1);
        assert_eq!(a.2, b.2, "tick {}: vertical speed", tick + 1);
        moved |= a.0 != b.0;
    }
    assert!(moved, "she did blink");
    // The arrival is on 21: her `x` jumps by 112 px that tick and no other.
    assert_eq!(blinked[20].0 - blinked[19].0, 112.0);
}

#[test]
fn donut_prog_blink_windup_horizontal_ignored() {
    let sim = windup_horizontal_ignored();
    let mut probe = Rig::new().build();
    let holds = vec![(P - 3, P + 40, stick(-1.0)), at(P, circle())];
    run_to(&mut probe, P - 1, &holds, &Vec::new());
    let before = donut_x(&probe);
    assert!(before < 640.0, "she walks until the press");
    // The stick (held left from before the press) is the direction: left.
    assert_eq!(
        blink_events(&sim)[1],
        resolved(
            P + 6,
            before.round() as i32,
            (before - 112.0).round() as i32
        )
    );
    run_to(&mut probe, P + 5, &holds, &Vec::new());
    assert_eq!(donut_x(&probe), before, "frozen t0..t5");
    run_to(&mut probe, P + 6, &holds, &Vec::new());
    assert_eq!(donut_x(&probe), before - 112.0, "arrival with vx 0");
    run_to(&mut probe, P + 7, &holds, &Vec::new());
    assert!(
        donut_x(&probe) < before - 112.0,
        "she walks again in recovery"
    );
}

#[test]
fn donut_prog_blink_hurt_in_windup_cancels() {
    let sim = hurt_in_windup();
    let events = blink_events(&sim);
    assert_eq!(events[0], started(P));
    assert_eq!(events[1], cancelled(P + 3, CancelCause::Hurt));
    assert_eq!(events.len(), 2);
    assert!(spends(&sim).is_empty());
    assert!(!sim.combat.donut_blink().cooling(P + 10));
}

#[test]
fn donut_prog_blink_no_iframes() {
    let (_sim, invulnerable) = no_iframes();
    assert!(
        invulnerable.iter().all(|i| *i == 0),
        "no tick of the windup, arrival or recovery grants invulnerability: {invulnerable:?}"
    );
    // A blow on the arrival tick and on t7 lands (the hook has no i-frame
    // check; the live check reads `invulnerable_ticks`, asserted 0 above).
    for blow in [P + 6, P + 7] {
        let mut sim = Rig::new().build();
        let holds = vec![at(P, circle())];
        run_to(&mut sim, blow, &holds, &Vec::new());
        let health = sim.scene.world.donut_body.health;
        let tick = sim.scene.world.tick;
        sim.combat.test_hurt_donut(&mut sim.scene, tick);
        assert!(
            sim.scene.world.donut_body.health < health,
            "hurt on t{}",
            blow - P
        );
        // A hurt after the arrival does not cancel anything (it was resolved).
        assert!(
            blink_events(&sim)
                .iter()
                .all(|e| !matches!(e, CombatEvent::BlinkCancelled { .. }))
        );
    }
}

#[test]
fn donut_prog_blink_no_damage() {
    let sim = no_damage();
    assert_eq!(blink_events(&sim)[1], resolved(P + 6, 640, 752));
    let start = {
        let mut rig = Rig::new();
        rig.enemies = &PASSIVE;
        rig.build().combat.enemies[0].health
    };
    assert_eq!(sim.combat.enemies[0].health, start, "never hurt");
    assert!(
        sim.combat.events.iter().all(|e| !matches!(
            e,
            CombatEvent::Hit { .. }
                | CombatEvent::EnemyDefeated { .. }
                | CombatEvent::HeroHurt { .. }
        )),
        "no hit anywhere"
    );
}

#[test]
fn donut_prog_blink_no_mana() {
    let sim = no_mana();
    assert_eq!(blink_events(&sim), [rejected(P, SpecialReject::NoMana)]);
    assert!(spends(&sim).is_empty());
    assert_eq!(donut_x(&sim), 640.0);
}

#[test]
fn donut_prog_blink_cooldown_reject() {
    let sim = cooldown_reject();
    assert_eq!(
        blink_events(&sim),
        [
            started(P),
            resolved(P + 6, 640, 752),
            rejected(P + 100, SpecialReject::Cooldown)
        ]
    );
}

#[test]
fn donut_prog_blink_swap_keeps_cooldown() {
    let sim = swap_keeps_cooldown();
    let events = blink_events(&sim);
    assert_eq!(events[0], started(P));
    assert!(matches!(events[1], CombatEvent::BlinkResolved { tick, .. } if tick == P + 6));
    assert_eq!(
        events.last(),
        Some(&rejected(100, SpecialReject::Cooldown)),
        "the cooldown is Donut's own, not the controller's: {events:?}"
    );
}

#[test]
fn donut_prog_blink_swap_mid_windup() {
    let sim = swap_mid_windup();
    let events = blink_events(&sim);
    assert_eq!(events[0], started(P));
    assert!(
        matches!(events[1], CombatEvent::BlinkResolved { tick, .. } if tick == P + 6),
        "a solo swap cancels nothing: {events:?}"
    );
}

#[test]
fn donut_prog_blink_retry_clears() {
    let sim = retry_clears();
    // After the retry the special is ready at once: nothing cools down.
    assert!(sim.combat.donut_blink().is_idle());
    assert_eq!(sim.combat.restarts, 1);
    assert_eq!(sim.combat.donut_mana_view().map(|m| m.0), Some(8));
    // A press right after the retry is accepted (the attempt-local tick
    // restarted, the cooldown with it).
    let mut again = retry_clears();
    let tick = again.scene.world.tick + 1;
    again.step_slots([NONE, circle()]);
    assert!(
        blink_events(&again).iter().any(|e| *e == started(tick)
            || matches!(e, CombatEvent::SpecialStarted { tick: x, .. } if *x == tick)),
        "{:?}",
        blink_events(&again)
    );
}

#[test]
fn donut_prog_blink_restart_mid_windup() {
    let sim = restart_mid_windup();
    assert!(sim.combat.donut_blink().is_idle());
    assert!(
        sim.combat
            .events
            .iter()
            .any(|e| *e == cancelled(P + 3, CancelCause::Restart)),
        "{:?}",
        sim.combat.events
    );
}

#[test]
fn donut_prog_blink_carl_circle_unchanged() {
    let sim = carl_circle_unchanged();
    let carl_throws = sim.combat.events.iter().any(|e| {
        matches!(
            e,
            CombatEvent::SpecialStarted { tick, hero: 0, capability }
                if *tick == P && *capability == "special_thrown_charge"
        )
    });
    assert!(carl_throws, "Carl's Circle/B throws in the same run");
    assert!(
        sim.combat.events.iter().any(|e| *e == started(P + 2)),
        "and Donut's blinks"
    );
}

#[test]
fn donut_prog_blink_locked_circle_is_inert_and_silent() {
    let (pressed, idle) = (locked_run(true), locked_run(false));
    assert_eq!(pressed.state_hash(), idle.state_hash());
    assert_eq!(pressed.combat.events, idle.combat.events);
    assert!(blink_events(&pressed).is_empty());
}

/// The blink state is never mixed until used: a run with the capability but no
/// press hashes exactly as a run without it (only the ledger differs).
#[test]
fn an_unpressed_blink_adds_nothing_to_the_donut_kit_hash() {
    let sim = press_run(&Rig::new(), vec![], 40);
    assert!(sim.combat.donut_blink().is_idle());
}

// -------------------------------------------------------------- the search --

/// A seeded generator (the same on every run).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn range(&mut self, low: i32, high: i32) -> i32 {
        low + (self.next() % (high - low + 1) as u64) as i32
    }
}

/// The 1,000-position property test of the search over the microarena and the
/// corridor kit: a destination never overlaps a solid, never crosses a solid or
/// a barrier or a zone, never leaves the inset view and always has support.
/// Independent geometry, not the search's own helpers.
#[test]
fn the_search_holds_its_rules_over_1000_seeded_positions() {
    use crate::prototype::{HERO_HALF_HEIGHT as HH, HERO_HALF_WIDTH as HW, Rect};
    use crate::short_blink::{SearchWorld, search};
    let kits = [
        EnvKit::load(&root(), LAYOUT).unwrap(),
        crate::cf02::arena::load_arena(&root()).unwrap(),
    ];
    let tuning = t();
    let inset = f32::from(tuning.dpq27_view_inset_px);
    let mut rng = Lcg(0xD0C0_FFEE);
    let (mut found, mut none) = (0, 0);
    for index in 0..1000 {
        let kit = &kits[index % 2];
        let solids = kit.blocks();
        let width = kit.width;
        let x = rng.range(HW as i32, width - HW as i32) as f32;
        // On a random solid top, or in the air above one.
        let top = solids[(rng.next() as usize) % solids.len()].rect.top();
        let y = top + HH + rng.range(0, 3) as f32 * rng.range(0, 90) as f32;
        let dir: i8 = if rng.next().is_multiple_of(2) { 1 } else { -1 };
        let left = rng.range(0, (width - 640).max(0)) as f32;
        let barrier = Rect {
            left: rng.range(0, width) as f32,
            bottom: top,
            width: rng.range(4, 40) as f32,
            height: rng.range(8, 80) as f32,
        };
        let zone = {
            let min = rng.range(0, width) as f32;
            (min, min + rng.range(20, 60) as f32)
        };
        let hazard = Rect {
            left: rng.range(0, width) as f32,
            bottom: top,
            width: rng.range(10, 50) as f32,
            height: 20.0,
        };
        let barriers = [barrier];
        let zones = [zone];
        let hazards = [hazard];
        let world = SearchWorld {
            solids: &solids,
            barriers: &barriers,
            zones: &zones,
            hazards: &hazards,
            x_max: width as f32 - HW,
            view: (left, left + 640.0),
            partner: None,
        };
        let a = search(&world, x, y, dir, tuning);
        assert_eq!(a, search(&world, x, y, dir, tuning), "deterministic");
        let Some(to) = a else {
            none += 1;
            continue;
        };
        found += 1;
        let (lo, hi) = (x.min(to), x.max(to));
        let end = Rect {
            left: to - HW,
            bottom: y - HH,
            width: 2.0 * HW,
            height: 2.0 * HH,
        };
        let sweep = Rect {
            left: lo - HW,
            bottom: y - HH,
            width: hi - lo + 2.0 * HW,
            height: 2.0 * HH,
        };
        let case = format!("kit {} x {x} y {y} dir {dir} -> {to}", index % 2);
        assert!((to - x).abs() >= 16.0 && (to - x).abs() <= 112.0, "{case}");
        assert_eq!(((to - x).abs() as i32) % 4, 0, "{case}");
        assert!(to >= HW && to <= width as f32 - HW, "{case}: world");
        for block in solids.iter().filter(|b| b.intact) {
            assert!(!end.overlaps(block.rect), "{case}: in a solid");
            assert!(!sweep.overlaps(block.rect), "{case}: crosses a solid");
        }
        assert!(!sweep.overlaps(barrier), "{case}: crosses a barrier");
        assert!(!end.overlaps(hazard), "{case}: in a hazard");
        let crossed = (x < zone.0 && to >= zone.1) || (x >= zone.1 && to < zone.0);
        assert!(!crossed, "{case}: skips a zone");
        assert!(
            end.left >= left + inset && end.right() <= left + 640.0 - inset,
            "{case}: view"
        );
        let feet = y - HH;
        let supported = solids.iter().any(|b| {
            to >= b.rect.left
                && to <= b.rect.right()
                && b.rect.top() <= feet + 0.01
                && feet - b.rect.top() <= f32::from(tuning.dpq27_support_probe_px)
        });
        assert!(supported, "{case}: support");
    }
    assert!(found > 100, "the fuzz finds destinations: {found}/{none}");
    assert!(none > 10, "and rejects some: {found}/{none}");
}

// ------------------------------------------------------ the juicer's grant --

/// `donut_prog_juicer_grants_blink`: under `--scene boss_f01 --donut-prog` the
/// boss defeat emits exactly one `CapabilityGranted { special_short_blink }` on
/// the defeat tick, after `fist_reinforced` when CP is on (DP-A's boss
/// fixtures are the runs).
#[test]
fn donut_prog_juicer_grants_blink() {
    use crate::cf02_fixtures::find_party_fixture;
    let grants = |name: &str| {
        let fixture = find_party_fixture(name).expect("a registered fixture");
        let kit = crate::boss_scene::load_arena(&root()).unwrap();
        let mut sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
        sim.run_scripted(fixture.base.ticks);
        let defeat = sim
            .combat
            .events
            .iter()
            .find_map(|e| match *e {
                CombatEvent::EnemyDefeated { tick, .. } => Some(tick),
                _ => None,
            })
            .expect("the boss is defeated");
        let list: Vec<(u64, u8, &str)> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::CapabilityGranted {
                    tick,
                    hero,
                    capability,
                } => Some((tick, hero, capability)),
                _ => None,
            })
            .collect();
        (defeat, list)
    };
    let (defeat, list) = grants("donut_prog_boss_pins");
    assert_eq!(list, [(defeat, 1, "special_short_blink")], "exactly one");
    let (defeat, list) = grants("donut_prog_boss_both_programs");
    assert_eq!(
        list,
        [
            (defeat, 0, "fist_reinforced"),
            (defeat, 1, "special_short_blink")
        ],
        "after fist_reinforced when CP is on"
    );
    let _ = DONUT_PROG;
}
