//! Scripted party fixtures (WORK-005, WORK-006, HUD-DEBT-01, HUD-DEBT-04): named,
//! deterministic scenarios that drive the live party (two players, the solo
//! swap, Donut's damage) through [`PartyState`], [`SceneSim`] and
//! [`CombatState`] exactly like the interactive scene does, one raw input per
//! player slot per tick.
//!
//! Bevy-free. The ordinary single-hero fixtures in [`crate::combat_scene`] stay
//! legacy (Donut replays a trail, goblins chase Carl) so their hashes are
//! untouched; these fixtures are the ones that exercise the new behaviour and
//! carry their own goldens (see the tests).
//!
//! Tick numbering matches the combat fixtures: a tick is the number the scene
//! reports after the step, so a hold over `from..to` covers the steps that
//! produce `tick = from .. to - 1`.

use crate::boss_scene::{BOSS_FIXTURES, Pilot, pilot_inputs};
use crate::combat::EnemyAi;
use crate::combat_scene::{CombatState, EnemySpawn, Fixture};
use crate::companion_fixtures::COMPANION_FIXTURES;
use crate::corridor::SceneSim;
use crate::donut_fixtures::DONUT_FIXTURES;
use crate::donut_mana::DonutSetup;
use crate::env_kit::EnvKit;
use crate::input::{RawInput, SlotInputs};
use crate::party::{PROVISIONAL_TUNING, PadId, PartyState, PartyTuning};

/// Slot input for the ticks `from..to` (half-open, 1-based ticks).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotHold {
    pub from: u64,
    pub to: u64,
    /// 0 is P1 (Carl, or the controlled hero in solo), 1 is P2 (Donut).
    pub slot: usize,
    pub input: RawInput,
}

/// A held direction on `slot`.
pub(crate) const fn run(from: u64, to: u64, slot: usize, move_x: f32) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input: RawInput {
            move_x,
            ..RawInput::NONE
        },
    }
}

/// A held jump on `slot` (one press edge at `from`).
pub(crate) const fn jump(from: u64, to: u64, slot: usize) -> SlotHold {
    SlotHold {
        from,
        to,
        slot,
        input: RawInput {
            jump: true,
            ..RawInput::NONE
        },
    }
}

/// A held swap button (L1/LB) on P1: one press edge at `from`.
pub(crate) const fn swap(from: u64, to: u64) -> SlotHold {
    SlotHold {
        from,
        to,
        slot: 0,
        input: RawInput {
            swap: true,
            ..RawInput::NONE
        },
    }
}

/// A named party scenario.
#[derive(Debug)]
pub struct PartyFixture {
    /// The combat fixture it runs on: Carl's column, goblins and seed. Its own
    /// `script` is unused.
    pub base: &'static Fixture,
    /// Both players present (P2 joined with Start before tick 1).
    pub two_player: bool,
    pub tuning: PartyTuning,
    pub script: &'static [SlotHold],
    /// HUD-DEBT-03: a deterministic autopilot that plays the fixture (a pure
    /// function of the finished state) on top of the script. `Pilot::None` on
    /// every earlier fixture.
    pub pilot: Pilot,
    /// WORK-023, HUD-DEBT-02: Donut's declared biscuit state and the unlock
    /// zone (`DonutSetup::NONE` on every earlier fixture: locked, no zone).
    pub donut: DonutSetup,
}

impl PartyFixture {
    pub fn name(&self) -> &'static str {
        self.base.name
    }

    /// Whether Donut is a live hero in this fixture. Every party fixture has her,
    /// except the Carl-only `eng_*` Engineer fixtures (ENEMY-GAMEPLAY-ENGINEER:
    /// "Carl only, party not live"), whose rams, targets and lanes are Carl's.
    pub fn party_live(&self) -> bool {
        self.two_player || !crate::cf02_fixtures::carl_only(self.name())
    }

    /// The `drive` of this fixture's party ([`Self::party_live`]).
    pub fn drive(&self, party: &PartyState) -> crate::party::PartyDrive {
        party.drive(self.party_live())
    }

    /// Place Donut where the fixture declares she starts (the `eng_*` Engineer
    /// fixtures); a no-op for every other fixture.
    pub fn place_donut(&self, scene: &mut SceneSim) {
        if let Some(x) = crate::cf02_fixtures::donut_start_x(self.name()) {
            scene.world.donut_body.x = x;
            scene.world.sync_donut_for_tests();
        }
    }

    /// The raw input of each slot on `tick`: every hold that covers it, merged.
    pub fn slot_inputs(&self, tick: u64) -> SlotInputs {
        let mut out = [RawInput::default(); 2];
        for hold in self
            .script
            .iter()
            .filter(|h| (h.from..h.to).contains(&tick))
        {
            let raw = &mut out[hold.slot];
            raw.jump |= hold.input.jump;
            raw.swap |= hold.input.swap;
            raw.attack |= hold.input.attack;
            raw.physical |= hold.input.physical;
            raw.down |= hold.input.down;
            raw.ability |= hold.input.ability;
            if hold.input.move_x != 0.0 {
                raw.move_x = hold.input.move_x;
            }
        }
        out
    }

    /// The slot inputs of `tick`: the script, then (for a piloted fixture) the
    /// autopilot's choice from the finished state.
    pub fn inputs(&self, tick: u64, combat: &CombatState, scene: &SceneSim) -> SlotInputs {
        pilot_inputs(self.pilot, self.slot_inputs(tick), tick, combat, scene)
    }

    /// Every named party fixture: the seven party fixtures, then the boss ones,
    /// then Donut's Q10 and mana ones.
    pub fn all() -> impl Iterator<Item = &'static PartyFixture> {
        PARTY_FIXTURES
            .iter()
            .chain(BOSS_FIXTURES.iter())
            .chain(DONUT_FIXTURES.iter())
            .chain(COMPANION_FIXTURES.iter())
            .copied()
    }

    pub fn by_name(name: &str) -> Option<&'static PartyFixture> {
        Self::all().find(|f| f.name() == name)
    }

    pub fn names() -> String {
        Self::all().map(|f| f.name()).collect::<Vec<_>>().join(", ")
    }
}

pub(crate) const fn base(
    name: &'static str,
    summary: &'static str,
    seed: u64,
    enemies: &'static [EnemySpawn],
    ticks: u64,
) -> Fixture {
    Fixture {
        name,
        summary,
        seed,
        carl_x: 400,
        enemies,
        shore_dx: None,
        boss: None,
        engineers: &[],
        llamas: &[],
        charges: &[],
        script: &[],
        ticks,
    }
}

const DUO_MOVE_BASE: Fixture = base(
    "duo_independent_moves",
    "Two players: Carl runs right while Donut jumps in place, then runs left",
    0x0D02_0001,
    &[],
    160,
);

/// 2P: each slot moves its own hero; nothing crosses over.
pub const DUO_INDEPENDENT_MOVES: PartyFixture = PartyFixture {
    base: &DUO_MOVE_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[
        run(10, 130, 0, 1.0),
        jump(20, 21, 1),
        jump(60, 61, 1),
        run(70, 130, 1, -1.0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DUO_WALL_BASE: Fixture = base(
    "duo_separation_wall",
    "Two players: Carl runs right and Donut stands still; the camera follows the midpoint and a wall stops Carl at 448 px",
    0x0D02_0002,
    &[],
    420,
);

/// 2P camera: the midpoint, then the separation wall.
pub const DUO_SEPARATION_WALL: PartyFixture = PartyFixture {
    base: &DUO_WALL_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[run(10, 420, 0, 1.0)],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const SOLO_SWAP_BASE: Fixture = base(
    "solo_swap_continuity",
    "Solo: Carl runs, the player swaps to Donut mid-run (Carl keeps following as the bot), then swaps back",
    0x0D02_0003,
    &[],
    320,
);

/// Solo swap: control moves, the swapped-away hero continues as the bot.
pub const SOLO_SWAP_CONTINUITY: PartyFixture = PartyFixture {
    base: &SOLO_SWAP_BASE,
    two_player: false,
    tuning: PROVISIONAL_TUNING,
    script: &[
        run(10, 90, 0, 1.0),
        swap(90, 91),
        run(100, 200, 0, 1.0),
        swap(200, 201),
        run(210, 260, 0, 1.0),
    ],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DONUT_HURT_BASE: Fixture = base(
    "donut_hurt_iframes",
    "Two players, Donut stands still: two goblins swing 12 ticks apart; she is hurt once and the second swing meets her invulnerability",
    0x0D02_0004,
    &[
        EnemySpawn {
            x: 300,
            ai: EnemyAi::Chase,
        },
        EnemySpawn {
            x: 290,
            ai: EnemyAi::Chase,
        },
    ],
    140,
);

/// Donut hurt: one hit, i-frames absorb the second goblin.
pub const DONUT_HURT_IFRAMES: PartyFixture = PartyFixture {
    base: &DONUT_HURT_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const DONUT_DEFEAT_BASE: Fixture = base(
    "donut_defeat_party",
    "Two players, Donut stands still while one goblin keeps hitting her: at 0 HP the whole party restarts at full health",
    0x0D02_0005,
    &[EnemySpawn {
        x: 300,
        ai: EnemyAi::Chase,
    }],
    520,
);

/// D1: Donut at 0 HP is a party defeat with Carl's restart.
pub const DONUT_DEFEAT_PARTY: PartyFixture = PartyFixture {
    base: &DONUT_DEFEAT_BASE,
    two_player: true,
    tuning: PROVISIONAL_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const SOLO_GUARD_BASE: Fixture = base(
    "solo_guard_floors_at_one",
    "Solo, the bot cannot dodge: the goblin hits Donut over and over but she is guarded at 1 HP and the party never loses",
    0x0D02_0006,
    &[EnemySpawn {
        x: 300,
        ai: EnemyAi::Chase,
    }],
    520,
);

/// A tuning whose bot never dodges and whose goblin targets the nearest hero
/// (Donut), so the last-HP guard is what keeps her up.
const NO_DODGE_TUNING: PartyTuning = PartyTuning {
    dodge_radius_px: 0,
    target_ai_bias_px: 0,
    ..PROVISIONAL_TUNING
};

/// D1 M1: the AI hero floors at 1 HP in solo.
pub const SOLO_GUARD_FLOORS_AT_ONE: PartyFixture = PartyFixture {
    base: &SOLO_GUARD_BASE,
    two_player: false,
    tuning: NO_DODGE_TUNING,
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

const SOLO_DODGE_BASE: Fixture = base(
    "solo_follower_dodge",
    "Solo, Carl stands: the Donut bot jumps over a goblin's swing instead of taking it",
    0x0D02_0007,
    &[EnemySpawn {
        x: 300,
        ai: EnemyAi::Chase,
    }],
    200,
);

/// Q9 M3: the bot dodges a tell with a jump. The goblin is placed so Donut
/// (the bot, 32 px behind Carl) is the one in its swing.
pub const SOLO_FOLLOWER_DODGE: PartyFixture = PartyFixture {
    base: &SOLO_DODGE_BASE,
    two_player: false,
    tuning: PartyTuning {
        // Carl is shielded from the goblin's attention so the bot is the target.
        target_ai_bias_px: 0,
        ..PROVISIONAL_TUNING
    },
    script: &[],
    pilot: Pilot::None,
    donut: DonutSetup::NONE,
};

pub const PARTY_FIXTURES: [&PartyFixture; 7] = [
    &DUO_INDEPENDENT_MOVES,
    &DUO_SEPARATION_WALL,
    &SOLO_SWAP_CONTINUITY,
    &DONUT_HURT_IFRAMES,
    &DONUT_DEFEAT_PARTY,
    &SOLO_GUARD_FLOORS_AT_ONE,
    &SOLO_FOLLOWER_DODGE,
];

/// A party fixture's scene, combat and party state: what the headless replay
/// and the tests drive.
#[derive(Debug, Clone)]
pub struct PartyFixtureSim {
    pub scene: SceneSim,
    pub combat: CombatState,
    pub party: PartyState,
    fixture: &'static PartyFixture,
}

impl PartyFixtureSim {
    pub fn new(kit: &EnvKit, view_h: i32, fixture: &'static PartyFixture) -> Result<Self, String> {
        let scene = SceneSim::new_at(kit, view_h, fixture.base.carl_x)?;
        let combat = CombatState::new(fixture.base, kit, &scene)?
            .with_donut(fixture.donut)
            .with_story(crate::carl_prog_fixtures::story_setup(fixture.name()));
        let mut party = PartyState::new(fixture.tuning);
        if fixture.two_player {
            party.pad_connected(PadId(1));
            party.pad_start_pressed(PadId(2));
        }
        let mut sim = Self {
            scene,
            combat,
            party,
            fixture,
        };
        fixture.place_donut(&mut sim.scene);
        sim.scene.set_drive(fixture.drive(&sim.party));
        Ok(sim)
    }

    /// Tests only: a sim from a scene and combat state built by the caller (for
    /// example one run under an edited CF02 tuning).
    #[doc(hidden)]
    pub fn from_parts(
        scene: SceneSim,
        combat: CombatState,
        fixture: &'static PartyFixture,
    ) -> Self {
        let mut party = PartyState::new(fixture.tuning);
        if fixture.two_player {
            party.pad_connected(PadId(1));
            party.pad_start_pressed(PadId(2));
        }
        let mut sim = Self {
            scene,
            combat,
            party,
            fixture,
        };
        sim.scene.set_drive(fixture.drive(&sim.party));
        sim
    }

    /// One tick with explicit slot inputs: the swap, the party routing, the
    /// scene drive and the combat step, the same order as the interactive scene.
    pub fn step_slots(&mut self, slots: SlotInputs) {
        let hurt = self.combat.hurt_ticks_of(self.party.controlled_hero()) > 0;
        self.party.request_swap(slots[0].swap, hurt);
        let inputs = self.party.hero_inputs(slots);
        self.scene.set_drive(self.fixture.drive(&self.party));
        let restarts = self.combat.restarts;
        self.combat.step_party(&mut self.scene, inputs);
        if self.combat.restarts != restarts {
            self.party.restart();
        }
    }

    /// One tick of the fixture's own script.
    pub fn step_scripted(&mut self) {
        let slots = self
            .fixture
            .inputs(self.scene.world.tick + 1, &self.combat, &self.scene);
        self.step_slots(slots);
    }

    pub fn run_scripted(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.step_scripted();
        }
    }

    /// The combat hash plus the party's swap timers.
    pub fn state_hash(&self) -> u64 {
        party_hash(&self.combat, &self.scene, &self.party)
    }

    pub fn state_line(&self) -> String {
        party_state_line(self.fixture, &self.combat, &self.scene, &self.party)
    }
}

/// The combat hash plus the party's swap timers: the value the goldens pin.
pub fn party_hash(combat: &CombatState, scene: &SceneSim, party: &PartyState) -> u64 {
    let (cooldown, buffer) = party.swap_timers();
    let mut hash = combat.state_hash(scene);
    for value in [
        0x5041_5254_5953_5441,
        u64::from(cooldown),
        u64::from(buffer),
    ] {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// The line CI compares between the headless replay and the committed one.
pub fn party_state_line(
    fixture: &PartyFixture,
    combat: &CombatState,
    scene: &SceneSim,
    party: &PartyState,
) -> String {
    format!(
        "PARTY_STATE fixture={} mode={} controlled={:?} carl_hp={} donut_hp={} deaths={} hash={:016x}",
        fixture.name(),
        if fixture.two_player { "2p" } else { "solo" },
        party.controlled_hero(),
        scene.world.hero.health,
        scene.world.donut.health,
        combat.deaths,
        party_hash(combat, scene, party)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::CombatEvent;
    use crate::party::HeroId;
    use std::path::PathBuf;

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    fn kit() -> EnvKit {
        EnvKit::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")), LAYOUT).unwrap()
    }

    fn sim_of(fixture: &'static PartyFixture) -> PartyFixtureSim {
        PartyFixtureSim::new(&kit(), 360, fixture).unwrap()
    }

    fn hurts(sim: &PartyFixtureSim) -> Vec<(u64, u8, HeroId)> {
        sim.combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::HeroHurt {
                    tick,
                    health_left,
                    hero,
                    ..
                } => Some((*tick, *health_left, *hero)),
                _ => None,
            })
            .collect()
    }

    /// Committed goldens: the final state hash of each party fixture at its
    /// default length.
    const GOLDENS: [(&str, u64); 7] = [
        ("duo_independent_moves", 0x81ca_65ec_029b_6e63),
        ("duo_separation_wall", 0x1f40_a48b_874e_1dc9),
        ("solo_swap_continuity", 0x0a3b_cbef_fb97_ab5b),
        ("donut_hurt_iframes", 0x6b13_a15a_e7fc_186c),
        ("donut_defeat_party", 0xd481_5dd6_9ec4_8fff),
        ("solo_guard_floors_at_one", 0x78d4_244f_d621_6569),
        ("solo_follower_dodge", 0x8f8b_7dcc_2cc5_dc20),
    ];

    fn run(fixture: &'static PartyFixture) -> PartyFixtureSim {
        let mut sim = sim_of(fixture);
        sim.run_scripted(fixture.base.ticks);
        sim
    }

    /// Step `ticks` scripted ticks, collecting `read` after each one.
    fn trace<T>(
        fixture: &'static PartyFixture,
        mut read: impl FnMut(&PartyFixtureSim) -> T,
    ) -> Vec<T> {
        let mut sim = sim_of(fixture);
        (0..fixture.base.ticks)
            .map(|_| {
                sim.step_scripted();
                read(&sim)
            })
            .collect()
    }

    fn idle() -> RawInput {
        RawInput::default()
    }

    #[test]
    fn the_party_fixtures_match_their_committed_goldens_and_replay_identically() {
        assert_eq!(GOLDENS.len(), PARTY_FIXTURES.len());
        for (name, expected) in GOLDENS {
            let fixture = PartyFixture::by_name(name).unwrap();
            let (a, b) = (run(fixture), run(fixture));
            assert_eq!(a.state_hash(), b.state_hash(), "{name} replays");
            assert_eq!(a.state_hash(), expected, "{name}: {}", a.state_line());
        }
        let mut hashes: Vec<u64> = GOLDENS.iter().map(|g| g.1).collect();
        hashes.sort_unstable();
        hashes.dedup();
        assert_eq!(hashes.len(), GOLDENS.len(), "every fixture is distinct");
    }

    #[test]
    fn two_players_move_independently() {
        let steps = trace(&DUO_INDEPENDENT_MOVES, |s| {
            let w = &s.scene.world;
            (w.hero.x, w.hero.y, w.donut.x, w.donut.foot_y)
        });
        let carl_floor = steps[0].1;
        // Carl only runs: his height never changes while Donut jumps twice.
        assert!(steps.iter().all(|s| s.1 == carl_floor), "Carl never jumps");
        let donut_floor = steps[0].3;
        let peak = steps.iter().map(|s| s.3).fold(donut_floor, f32::max);
        assert!(peak > donut_floor + 80.0, "Donut jumped: {peak}");
        // Donut stands still in x until her own run (tick 70), Carl runs on.
        assert!(steps[..60].iter().all(|s| s.2 == 368.0), "no trail follow");
        assert!(steps[60].0 > 540.0);
        assert!(steps[129].2 < 260.0, "Donut ran left on P2's input");
        assert!(steps[129].0 > 680.0, "Carl ran right on P1's input");
        // The two players are never mixed up: P2 never moves Carl.
        let mut only_p2 = sim_of(&DONUT_HURT_IFRAMES);
        for _ in 0..30 {
            only_p2.step_slots([
                idle(),
                RawInput {
                    move_x: 1.0,
                    ..RawInput::NONE
                },
            ]);
        }
        assert_eq!(only_p2.scene.world.hero.x, 400.0);
        assert!(only_p2.scene.world.donut.x > 400.0);
    }

    #[test]
    fn q7_the_camera_follows_the_midpoint_and_a_wall_stops_the_leader_at_448_px() {
        let steps = trace(&DUO_SEPARATION_WALL, |s| {
            (
                s.scene.world.hero.x,
                s.scene.world.donut.x,
                s.scene.camera.left(),
            )
        });
        let max = PROVISIONAL_TUNING.max_separation_px(640) as f32;
        assert_eq!(max, 448.0);
        assert!(steps.iter().all(|s| s.0 - s.1 <= max), "never beyond 448");
        let (carl, donut, left) = *steps.last().unwrap();
        assert!(carl - donut > max - 4.0, "Carl reached the wall: {carl}");
        // Both heroes keep the 96 px edge margin the Decisions table states
        // (rounding allows one pixel): the 2P camera has no deadzone.
        let margin = PROVISIONAL_TUNING.camera_edge_margin_px as f32;
        let screen = |x: f32| x - left as f32;
        assert!(screen(donut) >= margin - 1.0, "{}", screen(donut));
        assert!(640.0 - screen(carl) >= margin - 1.0, "{}", screen(carl));
        // The camera centres the midpoint exactly (within a pixel).
        let mid = (carl + donut) * 0.5;
        assert!((left as f32 + 320.0 - mid).abs() <= 1.0, "{left} {mid}");
        // Donut did not move: the wall belongs to the leader.
        assert_eq!(donut, 368.0);
        // The camera never outruns its speed cap, and never zooms (cfg fixed).
        assert!(steps.windows(2).all(|w| (w[1].2 - w[0].2).abs() <= 4));
    }

    #[test]
    fn s4_a_join_with_the_bot_far_behind_puts_both_heroes_on_screen() {
        let mut sim = sim_of(&SOLO_SWAP_CONTINUITY);
        // Solo: the bot waits far behind Carl (past the 448 px limit).
        sim.scene.world.hero.x = 900.0;
        sim.scene.world.donut_body.x = 300.0;
        sim.scene.world.sync_donut_for_tests();
        // The solo camera already follows Carl there.
        let cfg = sim.scene.camera.config();
        sim.scene.camera = crate::scroll_camera::ScrollCamera::new(cfg, 900.0, 1);
        sim.step_slots([idle(), idle()]);
        let gap = sim.scene.world.hero.x - sim.scene.world.donut_body.x;
        assert!(gap > 448.0, "still solo: {gap}");
        // P2 joins: Donut is placed beside Carl on the first 2P tick.
        sim.party.pad_connected(PadId(1));
        sim.party.pad_start_pressed(PadId(2));
        sim.step_slots([idle(), idle()]);
        let (carl, donut) = (sim.scene.world.hero.x, sim.scene.world.donut_body.x);
        let max = PROVISIONAL_TUNING.max_separation_px(640) as f32;
        assert!((carl - donut).abs() <= max, "{carl} {donut}");
        let left = sim.scene.camera.left() as f32;
        assert!(donut >= left && donut <= left + 640.0, "on screen {donut}");
        assert!(carl >= left && carl <= left + 640.0, "on screen {carl}");
        // A join inside the limit leaves her where she is.
        let mut sim = sim_of(&SOLO_SWAP_CONTINUITY);
        sim.scene.world.donut_body.x = sim.scene.world.hero.x - 200.0;
        sim.scene.world.sync_donut_for_tests();
        let before = sim.scene.world.donut_body.x;
        sim.party.pad_connected(PadId(1));
        sim.party.pad_start_pressed(PadId(2));
        sim.step_slots([idle(), idle()]);
        assert!((sim.scene.world.donut_body.x - before).abs() < 1.0);
    }

    #[test]
    fn q7_the_wall_lets_a_hero_move_back_toward_the_other() {
        let mut sim = sim_of(&DUO_SEPARATION_WALL);
        for _ in 0..400 {
            sim.step_slots([
                RawInput {
                    move_x: 1.0,
                    ..RawInput::NONE
                },
                idle(),
            ]);
        }
        let gap = sim.scene.world.hero.x - sim.scene.world.donut.x;
        assert!(gap > 440.0 && gap <= 448.0, "{gap}");
        sim.step_slots([
            RawInput {
                move_x: -1.0,
                ..RawInput::NONE
            },
            idle(),
        ]);
        assert!(sim.scene.world.hero.x - sim.scene.world.donut.x < gap);
        // Donut can step away from the wall too, but not past it.
        for _ in 0..20 {
            sim.step_slots([
                idle(),
                RawInput {
                    move_x: -1.0,
                    ..RawInput::NONE
                },
            ]);
        }
        let gap = sim.scene.world.hero.x - sim.scene.world.donut.x;
        assert!(gap <= 448.0, "{gap}");
    }

    #[test]
    fn solo_swap_moves_control_and_the_swapped_hero_keeps_following() {
        let steps = trace(&SOLO_SWAP_CONTINUITY, |s| {
            (
                s.party.controlled_hero(),
                s.scene.world.hero.x,
                s.scene.world.donut.x,
                s.scene.camera.left(),
            )
        });
        // One press, one swap: controlled flips on tick 90 and back on 200.
        let flips: Vec<usize> = steps
            .windows(2)
            .enumerate()
            .filter(|(_, w)| w[0].0 != w[1].0)
            .map(|(i, _)| i + 2)
            .collect();
        assert_eq!(flips, vec![90, 200]);
        assert_eq!(steps[89].0, HeroId::Donut);
        // No teleport at the swap: nobody moves more than one run step a tick
        // (Carl's 185 px/s is 3.08, Donut's agile 207 px/s is 3.45).
        for pair in steps.windows(2) {
            assert!((pair[1].1 - pair[0].1).abs() <= 3.2);
            assert!((pair[1].2 - pair[0].2).abs() <= 3.5);
        }
        // While Donut is controlled (ticks 100..200) she runs ahead and Carl
        // follows as the bot, staying within the follow band.
        // DM-A (spec U4): the bot Carl follows at his own 185 px/s while she
        // runs at 207, so the band of 52 px opens by at most the 22 px/s
        // difference over the 90 ticks since the swap, and no more.
        let (carl, donut) = (steps[190].1, steps[190].2);
        let band = 52.0 + 90.0 * (207.0 - 185.0) / 60.0;
        assert!(donut > carl && donut - carl <= band, "{carl} {donut}");
        assert!(donut - carl > 52.0, "the agile gap is real: {carl} {donut}");
        assert!(steps[199].2 > steps[100].2 + 250.0);
        // The camera pan stays within the swap cap.
        assert!(steps.windows(2).all(|w| (w[1].3 - w[0].3).abs() <= 8));
    }

    #[test]
    fn q7_a_swap_pan_may_exceed_the_normal_camera_cap_but_only_briefly() {
        let mut sim = sim_of(&SOLO_SWAP_CONTINUITY);
        // Carl and Donut far apart: run Carl away, then swap to the trailing hero.
        sim.scene.world.donut_body.x = 100.0;
        sim.scene.world.sync_donut_for_tests();
        for _ in 0..30 {
            sim.step_slots([
                RawInput {
                    move_x: 1.0,
                    ..RawInput::NONE
                },
                idle(),
            ]);
        }
        let mut fastest = 0;
        sim.step_slots([
            RawInput {
                swap: true,
                ..RawInput::NONE
            },
            idle(),
        ]);
        assert_eq!(sim.party.controlled_hero(), HeroId::Donut);
        let mut last = sim.scene.camera.left();
        for _ in 0..80 {
            sim.step_slots([idle(), idle()]);
            let now = sim.scene.camera.left();
            fastest = fastest.max((now - last).abs());
            last = now;
        }
        assert!(fastest > 4 && fastest <= 8, "pan speed {fastest}");
        // Once caught up the normal cap applies again.
        assert!(!sim.scene.pan_boosted());
    }

    #[test]
    fn q2_q4_donut_is_hurt_once_and_her_invulnerability_absorbs_the_second_goblin() {
        let sim = run(&DONUT_HURT_IFRAMES);
        assert_eq!(hurts(&sim), vec![(64, 3, HeroId::Donut)]);
        assert_eq!(sim.scene.world.donut.health, 3);
        assert_eq!(sim.scene.world.hero.health, 4, "Carl is untouched");
        let swings: Vec<u64> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::GoblinSwing { tick, .. } => Some(*tick),
                _ => None,
            })
            .collect();
        assert_eq!(
            swings,
            vec![64, 76],
            "the second swing lands in her i-frames"
        );
    }

    #[test]
    fn q2_donut_flinches_for_18_ticks_after_the_hit_stop_and_cannot_move_meanwhile() {
        let mut sim = sim_of(&DONUT_HURT_IFRAMES);
        while sim.scene.world.tick < 64 {
            sim.step_slots([idle(), idle()]);
        }
        assert_eq!(sim.combat.hurt_ticks_of(HeroId::Donut), 18);
        assert_eq!(sim.combat.hurt_ticks_of(HeroId::Carl), 0);
        let x = sim.scene.world.donut.x;
        let mut free_at = None;
        for tick in 65..130 {
            sim.step_slots([
                idle(),
                RawInput {
                    move_x: 1.0,
                    jump: true,
                    ..RawInput::NONE
                },
            ]);
            if sim.scene.world.donut.x != x && free_at.is_none() {
                free_at = Some(tick);
            }
            if free_at.is_none() {
                assert_eq!(sim.scene.world.donut.foot_y, 92.0, "no jump while hurt");
            }
        }
        // 4 ticks of hit-stop freeze the timer, then 18 ticks of flinch.
        assert_eq!(free_at, Some(64 + 4 + 18));
        // Carl moved freely the whole time (his slot never touched Donut's).
        assert_eq!(sim.scene.world.hero.x, 400.0);
    }

    #[test]
    fn d1_donut_at_zero_health_is_a_party_defeat_and_both_heroes_restart_full() {
        let steps = trace(&DONUT_DEFEAT_PARTY, |s| {
            (
                s.scene.world.hero.health,
                s.scene.world.donut.health,
                s.combat.hero_defeated,
                s.combat.restarts,
            )
        });
        let sim = run(&DONUT_DEFEAT_PARTY);
        let events: Vec<String> = sim.combat.events.iter().map(|e| e.to_string()).collect();
        assert!(events.contains(
            &"COMBAT_EVENT tick=349 kind=hero_hurt source=0 health_left=0 hero=donut".to_string()
        ));
        assert!(
            events.contains(&"COMBAT_EVENT tick=349 kind=hero_defeated hero=donut".to_string())
        );
        assert!(events.contains(&"COMBAT_EVENT tick=398 kind=restart deaths=1".to_string()));
        // Carl never lost health; Donut walked 4 -> 0 and the party is "defeated"
        // from her last hit until the restart 49 ticks later.
        assert!(steps.iter().all(|s| s.0 == 4));
        assert!(steps[348].2 && steps[396].2 && !steps[397].2);
        assert_eq!(steps[397].3, 1, "restarted once");
        // After the restart: full health, spawn positions, Carl controlled.
        let after = &steps[399];
        assert_eq!((after.0, after.1), (4, 4));
        assert_eq!(sim.combat.deaths, 1);
        assert_eq!(sim.party.controlled_hero(), HeroId::Carl);
        assert!(sim.scene.world.tick < 130, "a fresh attempt is running");
        assert_eq!(sim.scene.world.hero.x, 400.0);
    }

    #[test]
    fn d1_either_hero_defeat_uses_the_same_flow_carl_first() {
        // The same restart flow also serves Carl: solo with the default
        // tuning, the goblin chases Carl (the controlled hero) and defeats him.
        let mut sim = sim_of(&SOLO_BIAS);
        sim.run_scripted(SOLO_BIAS.base.ticks);
        let defeats: Vec<HeroId> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::HeroDefeated { hero, .. } => Some(*hero),
                _ => None,
            })
            .collect();
        assert_eq!(defeats, vec![HeroId::Carl]);
        assert!(sim.combat.restarts >= 1);
    }

    #[test]
    fn d1_m1_the_uncontrolled_hero_floors_at_one_health_in_solo() {
        let sim = run(&SOLO_GUARD_FLOORS_AT_ONE);
        let seen = hurts(&sim);
        let seen: Vec<(u64, u8)> = seen.iter().map(|h| (h.0, h.1)).collect();
        assert_eq!(seen, vec![(64, 3), (159, 2), (254, 1), (349, 1), (444, 1)]);
        assert!(hurts(&sim).iter().all(|h| h.2 == HeroId::Donut));
        assert_eq!(sim.scene.world.donut.health, 1);
        assert!(!sim.combat.hero_defeated && sim.combat.deaths == 0);
        assert_eq!(sim.combat.restarts, 0);
        // Without the guard the same run is a defeat at tick 349.
        const UNGUARDED: PartyFixture = PartyFixture {
            tuning: PartyTuning {
                solo_last_hp_guard: false,
                ..NO_DODGE_TUNING
            },
            ..SOLO_GUARD_FLOORS_AT_ONE
        };
        let unguarded = run(&UNGUARDED);
        assert_eq!(unguarded.combat.deaths, 1, "the guard is what saved her");
    }

    #[test]
    fn d1_the_guard_follows_the_ai_after_a_swap_and_never_applies_in_two_player() {
        // A swapped Carl is the AI hero: he floors at 1 HP too.
        let mut sim = sim_of(&SOLO_BIAS);
        sim.scene.world.hero.health = 1;
        sim.step_slots([
            RawInput {
                swap: true,
                ..RawInput::NONE
            },
            idle(),
        ]);
        assert_eq!(sim.party.controlled_hero(), HeroId::Donut);
        let drive = sim.party.drive(true);
        assert!(drive.is_guarded(HeroId::Carl) && !drive.is_guarded(HeroId::Donut));
        // Two players: Donut dies at 0 (no guard); covered by the defeat fixture.
        let duo = sim_of(&DONUT_DEFEAT_PARTY);
        assert!(!duo.party.drive(true).is_guarded(HeroId::Donut));
    }

    #[test]
    fn d1_m4_a_guarded_hero_outside_the_view_cannot_be_hurt_but_one_on_screen_can() {
        for (donut_x, hurt) in [(40.0, false), (250.0, true)] {
            let mut sim = sim_of(&SOLO_GUARD_FLOORS_AT_ONE);
            // The goblin stands beside Donut at the start of an attack.
            sim.scene.world.donut_body.x = donut_x;
            sim.scene.world.sync_donut_for_tests();
            let g = &mut sim.combat.enemies[0];
            g.x = donut_x - 24.0;
            g.facing = 1;
            g.mode = crate::goblin::GoblinMode::Attack;
            g.attack_tick = 24;
            // View left is 104: x = 40 is more than 16 px outside it.
            assert_eq!(sim.scene.is_offscreen(donut_x, 16.0), !hurt);
            for _ in 0..6 {
                sim.step_slots([idle(), idle()]);
            }
            assert_eq!(!hurts(&sim).is_empty(), hurt, "donut at {donut_x}");
        }
    }

    #[test]
    fn q9_m3_the_follower_jumps_over_a_swing_aimed_at_her() {
        let steps = trace(&SOLO_FOLLOWER_DODGE, |s| {
            (s.scene.world.tick, s.scene.world.donut.foot_y)
        });
        let sim = run(&SOLO_FOLLOWER_DODGE);
        assert!(hurts(&sim).is_empty(), "she is never hit");
        assert_eq!(sim.scene.world.donut.health, 4);
        // At every swing impact (ticks 64..68 and 155..159) she is airborne
        // above the 30 px swing box.
        for (from, to) in [(64, 68), (155, 159)] {
            for (tick, foot) in &steps {
                if (from..to).contains(tick) {
                    assert!(*foot > 92.0 + 30.0, "tick {tick}: foot {foot}");
                }
            }
        }
    }

    #[test]
    fn q3_a_solo_goblin_prefers_the_controlled_hero_even_when_the_bot_is_nearer() {
        let mut solo = sim_of(&SOLO_BIAS);
        solo.run_scripted(30);
        assert_eq!(solo.combat.target_of(0), HeroId::Carl);
        // In two players, with Donut nearer, the same goblin chases Donut.
        let mut duo = sim_of(&DONUT_HURT_IFRAMES);
        duo.run_scripted(30);
        assert_eq!(duo.combat.target_of(0), HeroId::Donut);
    }

    #[test]
    fn q3_a_goblin_switches_target_only_when_the_other_hero_is_24_px_closer() {
        let mut sim = sim_of(&DUO_TARGETING);
        let place = |sim: &mut PartyFixtureSim, carl: f32, donut: f32| {
            sim.scene.world.hero.x = carl;
            sim.scene.world.donut_body.x = donut;
            sim.step_slots([idle(), idle()]);
        };
        // Goblin (passive) at 560. Tie: Carl.
        place(&mut sim, 576.0, 544.0);
        assert_eq!(sim.combat.target_of(0), HeroId::Carl);
        // Donut closer by 10: held (hysteresis).
        place(&mut sim, 576.0, 566.0);
        assert_eq!(sim.combat.target_of(0), HeroId::Carl);
        // Closer by 23: still held. By 24 or more: switch.
        place(&mut sim, 590.0, 567.0);
        assert_eq!(sim.combat.target_of(0), HeroId::Carl);
        place(&mut sim, 590.0, 566.0);
        assert_eq!(sim.combat.target_of(0), HeroId::Donut);
        // And back only with the same margin.
        place(&mut sim, 570.0, 590.0);
        assert_eq!(sim.combat.target_of(0), HeroId::Donut);
        place(&mut sim, 560.0, 590.0);
        assert_eq!(sim.combat.target_of(0), HeroId::Carl);
    }

    #[test]
    fn q3_the_target_is_locked_through_the_whole_attack() {
        let mut sim = sim_of(&DONUT_HURT_IFRAMES);
        // Run to the middle of goblin 0's tell: it chases Donut.
        sim.run_scripted(44);
        assert_eq!(
            sim.combat.enemies[0].mode,
            crate::goblin::GoblinMode::Attack
        );
        assert_eq!(sim.combat.target_of(0), HeroId::Donut);
        // Carl walks right up to the goblin: nearer by far, but the goblin keeps
        // aiming at Donut until the attack ends.
        sim.scene.world.hero.x = sim.combat.enemies[0].x + 5.0;
        for _ in 0..40 {
            sim.step_slots([idle(), idle()]);
            if sim.combat.enemies[0].mode == crate::goblin::GoblinMode::Attack {
                assert_eq!(sim.combat.target_of(0), HeroId::Donut);
            }
        }
    }

    /// The attack-less Donut of the party PR (#114): the Q10-01 knob off.
    const NO_DONUT_MOVES: PartyFixture = PartyFixture {
        tuning: PartyTuning {
            donut_moves: false,
            ..PROVISIONAL_TUNING
        },
        ..DUO_INDEPENDENT_MOVES
    };

    #[test]
    fn with_donut_moves_off_donut_has_no_attacks_and_carl_keeps_his_in_two_player() {
        let mut sim = sim_of(&NO_DONUT_MOVES);
        sim.step_slots([
            idle(),
            RawInput {
                attack: true,
                physical: true,
                ..RawInput::NONE
            },
        ]);
        assert!(sim.combat.events.is_empty(), "P2's buttons start no move");
        assert!(sim.combat.donut_move_view().is_none());
        sim.step_slots([
            RawInput {
                attack: true,
                ..RawInput::NONE
            },
            idle(),
        ]);
        assert!(sim.scene.world.hero.melee.current().is_some(), "P1 jabs");
    }

    #[test]
    fn with_donut_moves_on_the_second_player_swats_and_carl_keeps_his_jab() {
        let mut sim = sim_of(&DUO_INDEPENDENT_MOVES);
        sim.step_slots([
            idle(),
            RawInput {
                attack: true,
                ..RawInput::NONE
            },
        ]);
        assert!(
            matches!(
                sim.combat.events.first(),
                Some(CombatEvent::MoveStarted {
                    move_id: crate::combat::MoveId::Swat,
                    ..
                })
            ),
            "P2's Primary is the swat before the biscuit"
        );
        sim.step_slots([
            RawInput {
                attack: true,
                ..RawInput::NONE
            },
            idle(),
        ]);
        assert!(sim.scene.world.hero.melee.current().is_some(), "P1 jabs");
    }

    #[test]
    fn q8_a_swap_never_cancels_the_outgoing_heros_move() {
        let mut sim = sim_of(&SOLO_SWAP_CONTINUITY);
        sim.step_slots([
            RawInput {
                attack: true,
                ..RawInput::NONE
            },
            idle(),
        ]);
        let started = sim.scene.world.hero.melee.current().map(|m| m.instance);
        assert!(started.is_some());
        sim.step_slots([
            RawInput {
                swap: true,
                ..RawInput::NONE
            },
            idle(),
        ]);
        assert_eq!(sim.party.controlled_hero(), HeroId::Donut);
        let still = sim.scene.world.hero.melee.current().map(|m| m.instance);
        assert_eq!(still, started, "the AI Carl finishes his jab");
        for _ in 0..30 {
            sim.step_slots([idle(), idle()]);
        }
        assert!(sim.scene.world.hero.melee.current().is_none());
        let starts = sim
            .combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::MoveStarted { .. }))
            .count();
        assert_eq!(starts, 1);
    }

    #[test]
    fn q8_a_swap_is_blocked_while_hurt_and_a_late_press_fires_when_the_flinch_ends() {
        let mut sim = sim_of(&SOLO_BIAS);
        let mut swapped_at = None;
        let mut pressed = false;
        for tick in 1..300u64 {
            let hurt = sim.combat.hurt_ticks_of(HeroId::Carl);
            // Press once, with 3 flinch ticks left.
            let press = !pressed && hurt == 3;
            pressed |= press;
            sim.step_slots([
                RawInput {
                    swap: press,
                    ..RawInput::NONE
                },
                idle(),
            ]);
            if sim.party.controlled_hero() == HeroId::Donut && swapped_at.is_none() {
                swapped_at = Some(tick);
                // It fired on the first tick Carl could act again.
                assert_eq!(sim.combat.hurt_ticks_of(HeroId::Carl), 0);
            }
            if hurt > 0 && !press {
                assert_eq!(sim.party.controlled_hero(), HeroId::Carl);
            }
        }
        assert!(pressed && swapped_at.is_some());
    }

    #[test]
    fn the_follower_regroups_when_it_has_been_off_screen_for_120_ticks() {
        let mut sim = sim_of(&SOLO_SWAP_CONTINUITY);
        sim.scene.world.donut_body.x = 20.0;
        sim.scene.world.sync_donut_for_tests();
        let ctx = crate::corridor::PartyStepCtx {
            can_move: [true, false],
            ..Default::default()
        };
        let (left, _) = (sim.scene.camera.left() as f32, 0);
        for tick in 1..=119 {
            sim.scene.step_party_ctx([idle(), idle()], &ctx);
            assert!(
                sim.scene.world.donut.x <= 21.0,
                "still stuck at tick {tick}"
            );
        }
        sim.scene.step_party_ctx([idle(), idle()], &ctx);
        let x = sim.scene.world.donut.x;
        assert!(
            (x - (left - 14.0)).abs() < 1.5,
            "placed just outside the view: {x} vs left {left}"
        );
        // Free again, she walks back in behind Carl.
        for _ in 0..120 {
            sim.step_slots([idle(), idle()]);
        }
        assert!((sim.scene.world.hero.x - sim.scene.world.donut.x).abs() <= 52.0);
    }

    #[test]
    fn the_legacy_drive_ignores_slot_one_and_replays_donuts_trail() {
        // A scripted fixture (no live party) must not read inputs[1].
        let k = kit();
        let mut legacy = SceneSim::new_at(&k, 360, 400).unwrap();
        let mut ignoring = SceneSim::new_at(&k, 360, 400).unwrap();
        for tick in 0..90 {
            let carl = RawInput {
                move_x: 1.0,
                jump: tick % 40 == 5,
                ..RawInput::NONE
            };
            legacy.step(carl);
            ignoring.step_party([
                carl,
                RawInput {
                    move_x: -1.0,
                    jump: true,
                    ..RawInput::NONE
                },
            ]);
            assert_eq!(legacy.state_hash(), ignoring.state_hash());
        }
        assert!(!ignoring.party_live());
    }

    #[test]
    fn the_party_fixture_names_are_unique_and_listed() {
        let names = PartyFixture::names();
        for fixture in PARTY_FIXTURES {
            assert!(names.contains(fixture.name()));
            assert_eq!(
                PartyFixture::by_name(fixture.name()).unwrap().name(),
                fixture.name()
            );
        }
        assert!(PartyFixture::by_name("nope").is_none());
    }

    // ---- test-local fixtures

    const SOLO_BIAS_BASE: Fixture = base(
        "solo_bias",
        "Solo, default tuning: the goblin chases Carl",
        0x0D02_0100,
        &[EnemySpawn {
            x: 300,
            ai: EnemyAi::Chase,
        }],
        520,
    );
    const SOLO_BIAS: PartyFixture = PartyFixture {
        base: &SOLO_BIAS_BASE,
        two_player: false,
        tuning: PROVISIONAL_TUNING,
        script: &[],
        pilot: Pilot::None,
        donut: DonutSetup::NONE,
    };
    const DUO_TARGET_BASE: Fixture = base(
        "duo_targeting",
        "Two players and a passive goblin for target switching",
        0x0D02_0101,
        &[EnemySpawn {
            x: 560,
            ai: EnemyAi::Passive,
        }],
        10,
    );
    const DUO_TARGETING: PartyFixture = PartyFixture {
        base: &DUO_TARGET_BASE,
        two_player: true,
        tuning: PROVISIONAL_TUNING,
        script: &[],
        pilot: Pilot::None,
        donut: DonutSetup::NONE,
    };
}
