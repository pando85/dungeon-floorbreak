//! EVID-ROOM: scripted **room pilots**, the input source of the deterministic
//! room-scene capture path (`floor01_scene --scene f01_late --f01-room ROOM
//! --room-pilot PLAN[:2p]`, see `docs/evidence/f01-boiler/README.md`).
//!
//! A pilot is a pure function of the finished simulation state (the hero and
//! the enemies, the env props, the combat log) and the tick: no clock, no RNG,
//! no wall time, no camera, nothing the presentation owns. The same pilot on the
//! same room therefore replays to the same state hash in the headless smoke and
//! in a rendered capture, which is how the evidence shows that the presentation
//! is read-only. A pilot is **not** a player and not an evaluator of fun: it
//! plays a fixed plan so the room can be filmed.
//!
//! Bevy-free. The plans reuse the CF02 pilots (`cf02::arena::autopilot` fights
//! the nearest awake goblin; `donut_pilot` is P2 as a human would play the room)
//! and the Boiler's prop reach columns (`boiler_fixtures::scene_pilots`).

use crate::boiler_fixtures::scene_pilots::{BRACE_X, VALVE_X};
use crate::boss_scene::Pilot;
use crate::cf02::arena::{self, Plan as FightPlan};
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::env_prop::{MassState, ValveState};
use crate::goblin::GoblinMode;
use crate::input::RawInput;
use crate::party::{HeroId, PartyTuning};
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};
use crate::rot_sticker::StickerState;

/// Ticks between the two distinct jabs a prop needs (the scene pilots use 1 and 31).
const JAB_PERIOD: u64 = 30;
/// A goblin this close (px) is fought before anything else.
const FIGHT_RADIUS: f32 = 70.0;
/// Where the chain pilot waits for the goblin it will kick: left of the valve,
/// so the launched body flies right into the valve's box.
const KICK_STAND_X: f32 = 380.0;
/// Where the chain pilot walks to wake the goblins that are still asleep, and
/// how close (px) a goblin must be to count as following him.
const LURE_X: f32 = 560.0;
/// `walk_to` stops within this many px of its target.
const WALK_DEADZONE: f32 = 4.0;
/// How far inside `LURE_RANGE` the lure walks (px): past the dead zone of `walk_to`.
const LURE_MARGIN: f32 = 6.0;
const LURE_RANGE: f32 = 200.0;
/// Carl backs off while the follower is nearer than this (px).
const KITE_GAP: f32 = 110.0;
/// A kick reaches a goblin whose centre is this far ahead (px).
const KICK_REACH: f32 = 36.0;
/// A launched goblin must start within this many px left of the valve column to
/// reach its box (the BOIL-D launch test kicks it from 30 px away).
const KICK_MAX_GAP: f32 = 60.0;
/// The brace (prop 1) of the Boiler and its trigger weight.
const BRACE_PROP: u16 = 1;

/// Room 4's spectacular route (FA-Q21): the column Carl throws from (the lowest column of the
/// 8 px sweep over 640..720 whose floor landing, 111 px ahead of him, chains R2 under the
/// `hazfu05` reach of 76 px: pinned by `haz_r2_chain_sweep_over_the_throw_columns`), how far he
/// backs off during the 150-tick fuse, and the column pocket A ends at.
const HAZ_THROW_X: f32 = 648.0;
const HAZ_POCKET_A_END: f32 = 400.0;
const HAZ_BACK_OFF: f32 = 90.0;

/// What a room pilot does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    /// Fight everything straight through and walk to the exit.
    Direct,
    /// Fight on the way; when the valve is untouched and nobody is close, stand
    /// at the valve and jab it twice, then wait out the steam.
    Valve,
    /// The Boiler's chain: jab the brace once, walk back left of the valve and
    /// kick the goblin that follows into the valve (steam, brace, landing, drum).
    Chain,
    /// PT-GATE-A: the closing arena (`--f01-room mastery`): the PT-MASTERY pilot
    /// ([`crate::f01_mastery_fixtures::pilot`]) fights the roster with the base
    /// kit and walks to the exit. Only valid in the mastery room.
    Mastery,
    /// PT-GATE-A: the Juicer's room (`--f01-room juicer`): the PT-SCENE-B solo-Carl
    /// pilot ([`crate::boss_scene::pilot_inputs`] with [`Pilot::Carl`], jab phase 0)
    /// fights the boss and walks out. Only valid in the Juicer's room.
    Juicer,
    /// PT-GATE-D (room 4, the spectacular route): the mastery pilot clears pocket A, then
    /// throws dynamite so that its blast lights the latched Rot Sticker of pocket B
    /// ([`haz_spectacular`]). Only valid in the hazards room.
    HazSpectacular,
    /// PT-GATE-E (room 4, the tactical route, HAZ-07 pattern 2): the mastery pilot clears
    /// pocket A, spends the Rot Sticker of pocket B by bait and retreat, then baits the
    /// Rot Sticker R2 of pocket C so that its blast hits the slime S2 under it
    /// ([`haz_tactical`]). Only valid in the hazards room.
    HazTactical,
    /// PT-GATE-E (item 4b): the hazards room's library pilot
    /// ([`crate::hazards_fixtures::pilots::run`], `hazards:<who>:<seed>`). Only valid
    /// in the hazards room, and only aligned.
    Hazards,
    /// PT-GATE-E (item 4b): the Boiler's live-roster library pilot
    /// ([`crate::boiler_fixtures::scene_pilots::run_live`],
    /// `boiler:<who>:<direct|both>:<seed>`). Only valid in the Boiler's room, and only aligned.
    Boiler,
    /// FA-Q24 (room 1): the Hoarder's room pilot ([`crate::hoarder_fixtures::ClearPilot`],
    /// `hoarder:carl_solo:<seed>`, plain `hoarder` = seed 0): solo Carl walks in, fights her
    /// and walks out. Only valid in the Hoarder's room, solo, and only solo Carl.
    Hoarder,
    /// CORRAL-F (room 0): the Infernal Corral's route pilot
    /// ([`crate::f01_corral_fixtures::pilot`], `corral:<who>:<seed>`, plain `corral` =
    /// `carl_solo` at seed 0, `who` one of `carl_solo`, `donut_fights` (the swap on tick 30,
    /// Donut fights the room) and `companion`): fights M0 to M3 with the base kit and walks
    /// to the exit. Only valid in the corral, solo, and only aligned.
    Corral,
    /// CORRAL-F (the #280 review F4): the in-room rush. Carl clears M0 and M1 with the corral
    /// pilot, walks to [`CORRAL_PARK_X`] inside B's rush trigger and stands there, so B
    /// rushes him in the launched room (the scene twin of
    /// `a_parked_carl_inside_b_trigger_is_rushed_by_b`). An evidence plan, not a clear:
    /// it never fights B or A. Only valid in the corral, solo; its partner is the
    /// follow-and-jump bot (it never attacks, so B lives to rush: [`corral_rush_tuning`]).
    CorralRush,
    /// CORRAL-D: the partition's three states in the run. Carl clears M0 and M1 with the corral
    /// pilot, walks to [`CORRAL_PARTITION_HIT_X`], the face of the screen, and jabs it every
    /// [`CORRAL_PARTITION_JAB_TICKS`] ticks: the screen stands, is weakened by the first hit, and
    /// B's rush (or the third jab) breaks it. An evidence plan, not a clear (it never fights B or
    /// A); its partner is the follow-and-jump bot ([`corral_rush_tuning`]). Only valid in the
    /// corral, solo.
    CorralPartition,
    /// DR-A (room 3): run to the exit (`dozer:run`). Only valid in the Dozer's room, solo.
    DozerRun,
    /// DR-A (room 3): hold at [`crate::dozer_fixtures::HOLD_X`] until the wheel is
    /// [`crate::dozer_fixtures::CLOSE_GAP_PX`] away, then run (`dozer:close_call`, the
    /// DOZER-CORE close call). Only valid in the Dozer's room, solo.
    DozerCloseCall,
    /// DR-A (room 3): the same hold at [`crate::dozer_fixtures::WAIT_X`] (`dozer:wait`).
    DozerWait,
    /// DR-B (room 3, the reversal capture): the brace pilot of `dz_frame_disable_once` in the
    /// live room (`dozer:brace`). Carl walks past the brace and back to
    /// [`crate::dozer_fixtures::WAIT_X`] so he faces it, jabs it twice so the frame is
    /// released in the middle of the helper's useful window, holds while it lands on the
    /// machine, and runs to the exit once the machine is stunned or disabled (or, as a
    /// fallback, once the wheel is [`crate::dozer_fixtures::CLOSE_GAP_PX`] away). Only valid in
    /// the Dozer's room, solo, and only aligned (it keeps the first chase tick).
    DozerBrace,
    /// DR-D (room 3, the #289 review finding 4 retry capture): Carl walks to
    /// [`crate::dozer_fixtures::HOLD_X`] and stands there until the wheel takes him (one blow,
    /// DZ-Q04); after the room's defeat respawn he walks into the trigger, waits while the rear
    /// gate rumbles and runs to the exit once the machine is out (`dozer:retry`). Only valid
    /// in the Dozer's room, solo, and only aligned.
    DozerRetry,
    /// DR-D (room 3, the #289 review finding 4 crew capture): the `dozer:brace` pilot until the
    /// machine is disabled, then Carl fights the dismounted riders (the CF02 fight of the
    /// `direct` plan) and walks to the exit (`dozer:crew`). Only valid in the Dozer's room,
    /// solo, and only aligned.
    DozerCrew,
}

/// Where [`Plan::CorralRush`] parks Carl: 112 px left of B, inside her 128 px trigger
/// (the column of `a_parked_carl_inside_b_trigger_is_rushed_by_b`).
pub const CORRAL_PARK_X: f32 = (crate::corral_fixtures::room::B_X - 112) as f32;

/// Where [`Plan::CorralPartition`] walks to: Carl's 20 px body ends against the screen's left face
/// (1392) at 1382, so his jab reaches it (he stops within the walk deadzone of it, at 1378 or later).
pub const CORRAL_PARTITION_HIT_X: f32 = 1382.0;
/// Past this column (the entry shelf ends at 1216) [`Plan::CorralPartition`] no longer hops.
pub const CORRAL_PARTITION_NO_HOP_X: f32 = 1300.0;
/// The ticks between two jabs of [`Plan::CorralPartition`].
pub const CORRAL_PARTITION_JAB_TICKS: u64 = 24;

impl Plan {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Valve => "valve",
            Self::Chain => "chain",
            Self::Mastery => "mastery",
            Self::Juicer => "juicer",
            Self::HazSpectacular => "haz_spectacular",
            Self::HazTactical => "haz_tactical",
            Self::Hazards => "hazards",
            Self::Boiler => "boiler",
            Self::Hoarder => "hoarder",
            Self::Corral => "corral",
            Self::CorralRush => "corral_rush",
            Self::CorralPartition => "corral_partition",
            Self::DozerRun => "dozer:run",
            Self::DozerCloseCall => "dozer:close_call",
            Self::DozerWait => "dozer:wait",
            Self::DozerBrace => "dozer:brace",
            Self::DozerRetry => "dozer:retry",
            Self::DozerCrew => "dozer:crew",
        }
    }

    /// The room 3 plans.
    pub const fn is_dozer(self) -> bool {
        matches!(
            self,
            Self::DozerRun
                | Self::DozerCloseCall
                | Self::DozerWait
                | Self::DozerBrace
                | Self::DozerRetry
                | Self::DozerCrew
        )
    }
}

/// PT-GATE-C1: who plays an aligned room pilot (`mastery:<who>:<seed>`,
/// `juicer:<who>:<seed>`): the same four pilots as the library fixtures
/// ([`crate::f01_mastery_fixtures::Who`] and [`crate::f01_late_fixtures::JuicerWho`]),
/// so a scene run and a library run play the same fight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignedWho {
    /// Solo Carl; the other hero is the party's `cf02_assist` partner (mastery) or the
    /// follow-and-jump bot (Juicer).
    CarlSolo,
    /// Carl clears the room, then the swap, then Donut (solo) walks to the exit.
    DonutBySwap,
    /// Mastery only: the swap on tick 90, Donut (solo) fights the room.
    DonutFights,
    /// Carl is the pilot's, Donut is the autonomous `cf02_free` partner.
    Companion,
}

impl AlignedWho {
    pub const fn token(self) -> &'static str {
        match self {
            Self::CarlSolo => "carl_solo",
            Self::DonutBySwap => "donut_by_swap",
            Self::DonutFights => "donut_fights",
            Self::Companion => "companion",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        [
            Self::CarlSolo,
            Self::DonutBySwap,
            Self::DonutFights,
            Self::Companion,
        ]
        .into_iter()
        .find(|who| who.token() == value)
    }

    /// The library fixture's `Who` of the mastery room.
    pub const fn mastery(self) -> crate::f01_mastery_fixtures::Who {
        use crate::f01_mastery_fixtures::Who;
        match self {
            Self::CarlSolo => Who::CarlSolo,
            Self::DonutBySwap => Who::DonutBySwap,
            Self::DonutFights => Who::DonutFights,
            Self::Companion => Who::Companion,
        }
    }
}

/// An aligned room pilot: the library fixture's pilot, party and seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aligned {
    pub who: AlignedWho,
    pub seed: u64,
    /// The Boiler's plan (`boiler:<who>:<direct|both>:<seed>`); `None` for every other room.
    pub route: Option<BoilerRoute>,
}

/// PT-GATE-E: which of the two live Boiler plans an aligned `boiler` pilot plays.
pub type BoilerRoute = crate::boiler_fixtures::scene_pilots::Plan;

/// The token of a [`BoilerRoute`] (`direct` or `both`), `None` for the plans the live room has no pilot for.
fn route_token(route: BoilerRoute) -> &'static str {
    match route {
        BoilerRoute::Direct => "direct",
        _ => "both",
    }
}

/// What an aligned run carries from tick to tick: the "swap was pressed once" flag of
/// the mastery and Juicer pilots, and the Boiler's step machine (created on its first tick).
#[derive(Debug, Clone, Default)]
pub struct AlignedMemory {
    swap_sent: bool,
    boiler: Option<crate::boiler_fixtures::scene_pilots::LiveDriver>,
    hoarder: Option<crate::hoarder_fixtures::ClearPilot>,
    /// DR-B (`dozer:brace`): Carl has passed the brace and turned back to it, and has
    /// arrived at it (he stands still from then on, so he keeps facing it).
    dozer_turned: bool,
    dozer_arrived: bool,
    /// DR-B (`dozer:brace`): the first committed-chase tick and the aimed release R.
    dozer_chase: Option<(u64, u64)>,
}

/// DR-A (DRA-AD5): the aligned party of every `dozer:*` pilot: Carl steered, Donut the
/// autonomous `cf02_free` partner (K5 `carl_solo`'s party). The seed is nominal: the room is
/// built without `--cf02-seed`, like a player's run (DRA-AD2).
pub const DOZER_ALIGNED: Aligned = Aligned {
    who: AlignedWho::Companion,
    seed: 0,
    route: None,
};

/// A parsed `--room-pilot` value: `PLAN`, `PLAN:2p`, or `mastery|juicer:WHO:SEED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomPilot {
    pub plan: Plan,
    /// P2 plays Donut with the CF02 pilot (two local players); otherwise the
    /// follower bot drives her, as in every scripted solo run.
    pub two_player: bool,
    /// PT-GATE-C1: the run is the library fixture's (same party, same seed, same
    /// inputs). The plain `mastery` plan is `mastery:carl_solo:0`; the plain
    /// `juicer` plan stays the unaligned PT-GATE-A pilot.
    pub aligned: Option<Aligned>,
}

impl RoomPilot {
    pub fn parse(value: &str) -> Result<Self, String> {
        let (plan, rest) = value.split_once(':').map_or((value, ""), |(a, b)| (a, b));
        if plan == "dozer" {
            // DR-A (DRA-AD2): `dozer:run|close_call|wait`, solo only (a stateless pilot,
            // no seed: the input is a function of the hero's column and the machine).
            // DRA-AD5: aligned, so Donut is the live party's autonomous `cf02_free` partner (the
            // mode a solo player gets, K5 `carl_solo`), not the legacy follower.
            let plan = match rest {
                "run" => Plan::DozerRun,
                "close_call" => Plan::DozerCloseCall,
                "wait" => Plan::DozerWait,
                "brace" => Plan::DozerBrace,
                "retry" => Plan::DozerRetry,
                "crew" => Plan::DozerCrew,
                other => {
                    return Err(format!(
                        "bad --room-pilot {value:?} (expected dozer:run, dozer:close_call, dozer:wait, dozer:brace, dozer:retry or dozer:crew; solo only{})",
                        if other.ends_with("2p") {
                            ", no :2p"
                        } else {
                            ""
                        }
                    ));
                }
            };
            return Ok(Self {
                plan,
                two_player: false,
                aligned: Some(DOZER_ALIGNED),
            });
        }
        if rest.contains(':')
            && matches!(
                plan,
                "mastery" | "juicer" | "hazards" | "boiler" | "hoarder" | "corral"
            )
        {
            // PT-GATE-C1: `mastery:<who>:<seed>` and `juicer:<who>:<seed>`; PT-GATE-E:
            // `hazards:<who>:<seed>` and `boiler:<who>:<direct|both>:<seed>`.
            let parts: Vec<&str> = rest.split(':').collect();
            let (who, route, seed) = match (plan, parts.as_slice()) {
                ("boiler", [who, route, seed]) => (*who, Some(*route), *seed),
                ("boiler", _) => {
                    return Err(format!(
                        "bad --room-pilot {value:?} (expected boiler:WHO:direct|both:SEED)"
                    ));
                }
                (_, [who, seed]) => (*who, None, *seed),
                _ => {
                    return Err(format!(
                        "bad --room-pilot {value:?} (expected {plan}:WHO:SEED)"
                    ));
                }
            };
            let fights = matches!(plan, "mastery" | "corral");
            let who = AlignedWho::parse(who)
                .filter(|who| fights || *who != AlignedWho::DonutFights)
                .filter(|who| plan != "hoarder" || *who == AlignedWho::CarlSolo)
                .filter(|who| plan != "corral" || *who != AlignedWho::DonutBySwap)
                .ok_or_else(|| {
                    if plan == "hoarder" {
                        return format!("bad --room-pilot who {who:?} (room 1 has only carl_solo)");
                    }
                    if plan == "corral" {
                        return format!(
                            "bad --room-pilot who {who:?} (room 0 has carl_solo, donut_fights and companion)"
                        );
                    }
                    format!(
                        "bad --room-pilot who {who:?} (expected carl_solo, donut_by_swap, {}companion)",
                        if fights { "donut_fights, " } else { "" }
                    )
                })?;
            let route = route
                .map(|token| match token {
                    "direct" => Ok(BoilerRoute::Direct),
                    "both" => Ok(BoilerRoute::Both),
                    other => Err(format!(
                        "bad --room-pilot boiler plan {other:?} (expected direct or both)"
                    )),
                })
                .transpose()?;
            let seed = seed
                .parse::<u64>()
                .map_err(|_| format!("bad --room-pilot seed {seed:?} (an integer)"))?;
            return Ok(Self {
                plan: match plan {
                    "juicer" => Plan::Juicer,
                    "hazards" => Plan::Hazards,
                    "boiler" => Plan::Boiler,
                    "hoarder" => Plan::Hoarder,
                    "corral" => Plan::Corral,
                    _ => Plan::Mastery,
                },
                two_player: false,
                aligned: Some(Aligned { who, seed, route }),
            });
        }
        let plan = match plan {
            "direct" => Plan::Direct,
            "valve" => Plan::Valve,
            "chain" => Plan::Chain,
            "mastery" => Plan::Mastery,
            "juicer" => Plan::Juicer,
            "haz_spectacular" => Plan::HazSpectacular,
            "haz_tactical" => Plan::HazTactical,
            "hoarder" => Plan::Hoarder,
            "corral" => Plan::Corral,
            "corral_rush" => Plan::CorralRush,
            "corral_partition" => Plan::CorralPartition,
            "hazards" | "boiler" => {
                return Err(format!(
                    "--room-pilot {plan} is an aligned plan: {plan}:WHO:{}SEED",
                    if plan == "boiler" { "direct|both:" } else { "" }
                ));
            }
            other => {
                return Err(format!(
                    "unknown --room-pilot plan {other:?} (expected direct, valve, chain, mastery, juicer, hoarder, corral, corral_rush, corral_partition, haz_spectacular or haz_tactical, optionally :2p, or mastery|juicer|hazards|corral:WHO:SEED, hoarder:carl_solo:SEED, or boiler:WHO:direct|both:SEED)"
                ));
            }
        };
        let two_player = match rest {
            "" => false,
            "2p" => true,
            other => {
                return Err(format!(
                    "bad --room-pilot suffix {other:?} (the only one is :2p)"
                ));
            }
        };
        if matches!(
            plan,
            Plan::Hoarder | Plan::Corral | Plan::CorralRush | Plan::CorralPartition
        ) && two_player
        {
            return Err(format!(
                "--room-pilot {} is a solo plan (no :2p)",
                plan.name()
            ));
        }
        // The plain mastery plan is the library's `carl_solo` at seed 0 (PT-GATE-C1); so are the
        // plain hoarder plan (FA-Q24) and the plain corral plans (CORRAL-F).
        let aligned = (plain_aligned(plan) && !two_player).then_some(Aligned {
            who: AlignedWho::CarlSolo,
            seed: 0,
            route: None,
        });
        Ok(Self {
            plan,
            two_player,
            aligned,
        })
    }

    /// The value that parses back to this pilot.
    pub fn label(self) -> String {
        if self.plan.is_dozer() {
            return self.plan.name().to_owned();
        }
        if let Some(a) = self.aligned
            && !(plain_aligned(self.plan) && a.who == AlignedWho::CarlSolo && a.seed == 0)
        {
            match a.route {
                Some(route) => format!(
                    "{}:{}:{}:{}",
                    self.plan.name(),
                    a.who.token(),
                    route_token(route),
                    a.seed
                ),
                None => format!("{}:{}:{}", self.plan.name(), a.who.token(), a.seed),
            }
        } else if self.two_player {
            format!("{}:2p", self.plan.name())
        } else {
            self.plan.name().to_owned()
        }
    }

    /// The two slot inputs of the next tick. `exit_x` is the column Carl walks
    /// to once no enemy is left (the room's protected exit).
    pub fn inputs(self, combat: &CombatState, sim: &SceneSim, exit_x: f32) -> [RawInput; 2] {
        let carl = match self.plan {
            Plan::Direct => fight_or_walk(combat, sim, exit_x),
            Plan::Valve => valve(combat, sim, exit_x),
            Plan::Chain => chain(combat, sim, exit_x),
            Plan::Mastery => crate::f01_mastery_fixtures::arena_pilot(combat, sim, HeroId::Carl),
            Plan::Juicer => crate::boss_scene::pilot_inputs(
                Pilot::Carl,
                [RawInput::NONE; 2],
                sim.world.tick + 1,
                combat,
                sim,
            )[0],
            Plan::HazSpectacular => haz_spectacular(combat, sim),
            Plan::HazTactical => haz_tactical(combat, sim),
            Plan::Hazards => {
                crate::hazards_fixtures::pilots::route_pilot(combat, sim, HeroId::Carl)
            }
            Plan::Boiler => unreachable!("the live Boiler pilot is aligned only (aligned_inputs)"),
            Plan::Hoarder => unreachable!("the Hoarder's pilot is aligned only (aligned_inputs)"),
            Plan::Corral | Plan::CorralRush | Plan::CorralPartition => {
                unreachable!("the corral's pilots are aligned only (aligned_inputs)")
            }
            Plan::DozerRun | Plan::DozerCloseCall | Plan::DozerWait => {
                self.dozer_input(combat, sim, exit_x)
            }
            Plan::DozerBrace | Plan::DozerRetry | Plan::DozerCrew => {
                unreachable!("the brace, retry and crew pilots are aligned only (aligned_inputs)")
            }
        };
        let donut = if self.two_player {
            arena::donut_pilot(combat, sim)
        } else {
            RawInput::NONE
        };
        [carl, donut]
    }
}

/// The plans whose plain name is an aligned run (`carl_solo` at seed 0).
const fn plain_aligned(plan: Plan) -> bool {
    matches!(
        plan,
        Plan::Mastery | Plan::Hoarder | Plan::Corral | Plan::CorralRush | Plan::CorralPartition
    )
}

/// CORRAL-F: the corral fixture's `Who` of an aligned corral pilot.
pub const fn corral_who(who: AlignedWho) -> crate::f01_corral_fixtures::Who {
    use crate::f01_corral_fixtures::Who;
    match who {
        AlignedWho::DonutFights => Who::DonutSolo,
        AlignedWho::Companion => Who::Companion,
        AlignedWho::CarlSolo | AlignedWho::DonutBySwap => Who::CarlSolo,
    }
}

/// CORRAL-F: the party of [`Plan::CorralRush`]: the late slice's 12-tick swap with the
/// follow-and-jump bot as the partner, which never attacks (the `cf02_assist` partner of
/// `carl_solo` shoots B down before she can rush the parked Carl).
pub fn corral_rush_tuning() -> PartyTuning {
    PartyTuning {
        uncontrolled_hero: crate::party::UncontrolledHero::FollowAndJumpBot,
        ..crate::party::PROVISIONAL_TUNING
    }
    .with_cf02_swap()
}

impl RoomPilot {
    /// DR-A: Carl's input of a `dozer:*` plan (see [`dozer_escape`]).
    fn dozer_input(self, combat: &CombatState, sim: &SceneSim, exit_x: f32) -> RawInput {
        let hold_at = match self.plan {
            Plan::DozerCloseCall => Some(crate::dozer_fixtures::HOLD_X),
            Plan::DozerWait => Some(crate::dozer_fixtures::WAIT_X),
            _ => None,
        };
        dozer_escape(combat, sim, exit_x, hold_at)
    }
}

/// PT-GATE-C1: the party tuning of an aligned run, the library fixture's own
/// ([`crate::f01_mastery_fixtures::tuning`]; the Juicer's room party, or its
/// `cf02_free` partner under [`AlignedWho::Companion`]).
pub fn aligned_tuning(plan: Plan, who: AlignedWho) -> PartyTuning {
    use crate::f01_late_fixtures::{JUICER_COMPANION_PARTY, JUICER_ROOM_PARTY};
    match plan {
        Plan::Juicer => {
            if who == AlignedWho::Companion {
                JUICER_COMPANION_PARTY.tuning
            } else {
                JUICER_ROOM_PARTY.tuning
            }
        }
        // FA-Q24: the room's own party (`cf02_assist`, 12-tick swap): what the PLAYTRACE verifier
        // rebuilds from a room recording, and the library fixture's.
        Plan::Hoarder => crate::hoarder_fixtures::room_party_tuning(),
        // CORRAL-F: the corral fixture's party; the rush plan's partner never attacks.
        Plan::Corral => corral_who(who).tuning(),
        Plan::CorralRush | Plan::CorralPartition => corral_rush_tuning(),
        _ => crate::f01_mastery_fixtures::tuning(who.mastery()),
    }
}

impl RoomPilot {
    /// The party tuning of an aligned run, or `None` for an unaligned pilot.
    pub fn tuning(self) -> Option<PartyTuning> {
        self.aligned.map(|a| aligned_tuning(self.plan, a.who))
    }

    /// The slot inputs of an aligned run at the next tick: exactly the library
    /// fixture's (`f01_mastery_fixtures::run_for`, `f01_late_fixtures::juicer_run`,
    /// `hazards_fixtures::pilots::run`, `boiler_fixtures::scene_pilots::run_live`),
    /// for the hero `controlled` by the party. `memory` is the run's own state (the
    /// "swap was pressed once" flag, the Boiler's step machine); `exit_x` is the column
    /// the Boiler's pilot walks to (the other plans never read it). Panics on an
    /// unaligned pilot.
    pub fn aligned_inputs(
        self,
        combat: &CombatState,
        sim: &SceneSim,
        controlled: HeroId,
        exit_x: f32,
        memory: &mut AlignedMemory,
    ) -> [RawInput; 2] {
        use crate::f01_mastery_fixtures as mastery;
        let Aligned { who, seed, route } = self.aligned.expect("an aligned room pilot");
        let tick = sim.world.tick + 1;
        if self.plan.is_dozer() {
            // DR-A (DRA-AD5): the escape steers Carl; the party's `cf02_free` partner drives Donut.
            let input = if controlled != HeroId::Carl {
                RawInput::NONE
            } else if self.plan == Plan::DozerBrace {
                dozer_brace(combat, sim, exit_x, memory)
            } else if self.plan == Plan::DozerRetry {
                dozer_retry(combat, sim, exit_x)
            } else if self.plan == Plan::DozerCrew {
                dozer_crew(combat, sim, exit_x, memory)
            } else {
                self.dozer_input(combat, sim, exit_x)
            };
            return [input, RawInput::NONE];
        }
        if self.plan == Plan::Boiler {
            use crate::boiler_fixtures::scene_pilots::{LiveDriver, Who as BoilerWho};
            let boiler_who = match who {
                AlignedWho::DonutBySwap => BoilerWho::DonutBySwap,
                AlignedWho::Companion => BoilerWho::Companion,
                _ => BoilerWho::CarlSolo,
            };
            let driver = memory.boiler.get_or_insert_with(|| {
                LiveDriver::new(boiler_who, route.expect("a boiler route"))
                    .expect("a live boiler plan")
            });
            return driver.next(combat, sim, controlled, exit_x).0;
        }
        if self.plan == Plan::Hoarder {
            // FA-Q24: Carl's room 1 pilot; the other hero is the party's `cf02_assist` partner.
            let pilot = memory
                .hoarder
                .get_or_insert_with(|| crate::hoarder_fixtures::ClearPilot::new(seed));
            let input = if controlled == HeroId::Carl {
                pilot.input(combat, sim)
            } else {
                RawInput::NONE
            };
            return [input, RawInput::NONE];
        }
        if matches!(
            self.plan,
            Plan::Corral | Plan::CorralRush | Plan::CorralPartition
        ) {
            // CORRAL-F: the corral fixture's run (`f01_corral_fixtures::run`), tick for tick.
            use crate::f01_corral_fixtures as corral;
            let route = corral::Route::with_exit(exit_x);
            let mut input = if self.plan == Plan::CorralRush {
                corral_rush(&route, combat, sim, controlled)
            } else if self.plan == Plan::CorralPartition {
                corral_partition(&route, combat, sim, controlled)
            } else {
                corral::pilot(&route, combat, sim, controlled)
            };
            if who == AlignedWho::DonutFights
                && tick >= corral::SWAP_TICK
                && !memory.swap_sent
                && controlled == HeroId::Carl
            {
                input.swap = true;
                memory.swap_sent = true;
            }
            return [input, RawInput::NONE];
        }
        let swap_sent = &mut memory.swap_sent;
        let hero_pilot = |hero: HeroId| {
            if self.plan == Plan::Hazards {
                crate::hazards_fixtures::pilots::route_pilot(combat, sim, hero)
            } else {
                mastery::arena_pilot(combat, sim, hero)
            }
        };
        let mut input = if self.plan == Plan::Juicer {
            if controlled == HeroId::Carl {
                crate::boss_scene::pilot_inputs(
                    Pilot::Carl,
                    [RawInput::NONE; 2],
                    tick + seed % crate::f01_late_fixtures::SWING_PERIOD,
                    combat,
                    sim,
                )[0]
            } else {
                // Donut (swapped in after the defeat) walks to the exit.
                RawInput {
                    move_x: 1.0,
                    ..RawInput::NONE
                }
            }
        } else {
            match who {
                AlignedWho::CarlSolo | AlignedWho::Companion => hero_pilot(HeroId::Carl),
                AlignedWho::DonutBySwap | AlignedWho::DonutFights => hero_pilot(controlled),
            }
        };
        let due = if self.plan == Plan::Juicer {
            who == AlignedWho::DonutBySwap && combat.boss.as_ref().is_some_and(|b| b.released())
        } else {
            match who {
                AlignedWho::DonutFights => tick >= mastery::SWAP_TICK,
                AlignedWho::DonutBySwap => combat.roster_cleared(),
                _ => false,
            }
        };
        if due && !*swap_sent && controlled == HeroId::Carl {
            input.swap = true;
            *swap_sent = true;
        }
        [input, RawInput::NONE]
    }
}

/// CORRAL-F: [`Plan::CorralRush`]: the corral pilot while M0's goblins or the M1 llama
/// live, then the walk to [`CORRAL_PARK_X`] and nothing more (no attack, no jump), so B's
/// rush trigger finds a still hero.
fn corral_rush(
    route: &crate::f01_corral_fixtures::Route,
    combat: &CombatState,
    sim: &SceneSim,
    hero: HeroId,
) -> RawInput {
    let m1 = combat
        .llamas
        .iter()
        .any(|l| l.alive() && l.corral.is_none());
    let goblins = combat.enemies.iter().any(|g| g.alive());
    if m1 || goblins {
        return crate::f01_corral_fixtures::pilot(route, combat, sim, hero);
    }
    let x = sim.world.body(hero).x;
    if x < CORRAL_PARK_X - WALK_DEADZONE {
        let mut raw = walk_to(sim, CORRAL_PARK_X);
        raw.attack = false;
        raw.ability = false;
        raw
    } else {
        RawInput::NONE
    }
}

/// CORRAL-D: [`Plan::CorralPartition`]: the corral pilot while M0's goblins or the M1 llama live,
/// then the walk to [`CORRAL_PARTITION_HIT_X`] (no hop over the screen) and a jab every
/// [`CORRAL_PARTITION_JAB_TICKS`] ticks (one press per move instance: each jab is one counted hit).
fn corral_partition(
    route: &crate::f01_corral_fixtures::Route,
    combat: &CombatState,
    sim: &SceneSim,
    hero: HeroId,
) -> RawInput {
    let m1 = combat
        .llamas
        .iter()
        .any(|l| l.alive() && l.corral.is_none());
    let goblins = combat.enemies.iter().any(|g| g.alive());
    if m1 || goblins {
        return crate::f01_corral_fixtures::pilot(route, combat, sim, hero);
    }
    let x = sim.world.body(hero).x;
    if x < CORRAL_PARTITION_HIT_X - WALK_DEADZONE {
        // Walk up to the face (hopping the entry shelf on the way); past the shelf never
        // hop: the screen is what this plan is about.
        let mut raw = walk_to(sim, CORRAL_PARTITION_HIT_X);
        raw.attack = false;
        raw.ability = false;
        if x > CORRAL_PARTITION_NO_HOP_X {
            raw.jump = false;
        }
        raw
    } else {
        RawInput {
            attack: sim.world.tick.is_multiple_of(CORRAL_PARTITION_JAB_TICKS),
            ..RawInput::NONE
        }
    }
}

fn alive_any(combat: &CombatState) -> bool {
    combat.enemies.iter().any(|g| g.alive()) || combat.llamas.iter().any(|l| l.alive())
}

/// A solid that rises above the feet a step ahead (the same probe as the CF02
/// pilot's, so a drum or a remnant is hopped).
fn blocked_ahead(sim: &SceneSim, dir: f32) -> bool {
    let hero = &sim.world.hero;
    let probe = Rect {
        left: hero.x - HERO_HALF_WIDTH + dir * 14.0,
        bottom: hero.y - HERO_HALF_HEIGHT + 4.0,
        width: HERO_HALF_WIDTH * 2.0,
        height: 8.0,
    };
    crate::combat::solid_at(&sim.world.solids, probe)
}

fn walk_to(sim: &SceneSim, x: f32) -> RawInput {
    let hero = &sim.world.hero;
    let dx = x - hero.x;
    let dir = if dx.abs() < WALK_DEADZONE {
        0.0
    } else {
        dx.signum()
    };
    RawInput {
        move_x: dir,
        jump: hero.grounded && dir != 0.0 && blocked_ahead(sim, dir),
        ..RawInput::NONE
    }
}

/// A goblin whose body overlaps Carl's column is out of reach of his own jab
/// (the jab starts 16 px ahead): Carl steps back from it instead of swinging at
/// nothing (the CF02 pilot does not, because its room never lets a goblin land
/// on him after a hop over a prop).
const TOO_CLOSE: f32 = 10.0;

/// The CF02 fight, with the step back from a goblin that stands on Carl.
fn fight(combat: &CombatState, sim: &SceneSim) -> RawInput {
    let hero = &sim.world.hero;
    if !hero.grounded {
        // Over a goblin after a hop (the drum cuts the room): dive on it
        // (Down + Physical in the air, CF02-Q01).
        let below = combat
            .enemies
            .iter()
            .filter(|g| g.alive() && !matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse))
            .filter(|g| (g.x - hero.x).abs() < 40.0 && hero.y - g.y > 16.0)
            .min_by(|a, b| (a.x - hero.x).abs().total_cmp(&(b.x - hero.x).abs()));
        if let Some(g) = below {
            let dx = g.x - hero.x;
            return RawInput {
                move_x: if dx.abs() < 6.0 { 0.0 } else { dx.signum() },
                physical: true,
                down: true,
                ..RawInput::NONE
            };
        }
    }
    if hero.grounded
        && let Some(g) = combat
            .enemies
            .iter()
            .filter(|g| g.alive() && !matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse))
            .filter(|g| (g.x - hero.x).abs() < 40.0 && hero.y - g.y > 24.0)
            .min_by(|a, b| (a.x - hero.x).abs().total_cmp(&(b.x - hero.x).abs()))
    {
        // Standing on a prop above a goblin: hop and dive (the jab would pass over it).
        return RawInput {
            move_x: (g.x - hero.x).signum() * 0.5,
            jump: true,
            ..RawInput::NONE
        };
    }
    let on_top = combat
        .enemies
        .iter()
        .filter(|g| {
            g.alive()
                && !matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse)
                && (g.x - hero.x).abs() < TOO_CLOSE
                && (g.y - hero.y).abs() < 30.0
        })
        .min_by(|a, b| (a.x - hero.x).abs().total_cmp(&(b.x - hero.x).abs()));
    if let Some(g) = on_top
        && hero.grounded
    {
        let away = if g.x >= hero.x { -1.0 } else { 1.0 };
        return RawInput {
            move_x: away,
            ..RawInput::NONE
        };
    }
    arena::autopilot(combat, sim, FightPlan::Direct)
}

/// The CF02 fight while anything lives, the walk to the exit afterwards.
fn fight_or_walk(combat: &CombatState, sim: &SceneSim, exit_x: f32) -> RawInput {
    if alive_any(combat) {
        fight(combat, sim)
    } else {
        walk_to(sim, exit_x)
    }
}

/// An awake goblin within [`FIGHT_RADIUS`] of Carl.
fn threatened(combat: &CombatState, sim: &SceneSim) -> bool {
    let x = sim.world.hero.x;
    combat.enemies.iter().any(|g| {
        g.alive()
            && !matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse)
            && g.ai == crate::combat::EnemyAi::Chase
            && (g.x - x).abs() < FIGHT_RADIUS
    })
}

/// Walk to a prop's reach column and arrive facing right: `None` once Carl
/// stands on it (within one stride, from the left). From the right of it he
/// first steps back left of it, so the jab that follows faces the prop.
fn approach(sim: &SceneSim, column: f32) -> Option<RawInput> {
    let x = sim.world.hero.x;
    if x > column + 4.0 {
        Some(walk_to(sim, column - 24.0))
    } else if x < column - 4.0 {
        Some(walk_to(sim, column))
    } else if sim.world.hero.facing < 0 {
        // FA-Q32 (room 2 retune): a fight carried him past the column and he stepped back
        // into its band facing left; jabbing there never reaches the prop. Step left of it
        // again and come back facing right.
        Some(walk_to(sim, column - 24.0))
    } else {
        None
    }
}

fn jab(tick: u64) -> RawInput {
    RawInput {
        attack: tick % JAB_PERIOD == 1,
        ..RawInput::NONE
    }
}

fn valve(combat: &CombatState, sim: &SceneSim, exit_x: f32) -> RawInput {
    let tick = sim.world.tick + 1;
    let Some(env) = combat.env() else {
        return fight_or_walk(combat, sim, exit_x);
    };
    if threatened(combat, sim) {
        return fight(combat, sim);
    }
    match env.valve_state(0) {
        Some(ValveState::Intact) => approach(sim, VALVE_X).unwrap_or_else(|| jab(tick)),
        // The warning and the vent: stand and let the steam run.
        Some(ValveState::Warning | ValveState::Venting) => RawInput::NONE,
        _ => fight_or_walk(combat, sim, exit_x),
    }
}

/// A goblin the kick can still launch into the valve: awake, ahead of Carl,
/// inside the kick's reach, and close enough to the valve to reach its box.
fn kick_target(combat: &CombatState, sim: &SceneSim) -> bool {
    let hero = &sim.world.hero;
    combat.enemies.iter().any(|g| {
        g.alive()
            && g.ai == crate::combat::EnemyAi::Chase
            && !matches!(g.mode, GoblinMode::Down | GoblinMode::Corpse)
            && g.x > hero.x
            && g.x - hero.x <= KICK_REACH
            && g.x >= VALVE_X - KICK_MAX_GAP
            && g.x < VALVE_X
    })
}

/// Ticks after which the chain pilot gives up waiting for a goblin to kick and
/// jabs the valve itself (the fallback of the Valve plan).
const KICK_GIVE_UP_TICK: u64 = 900;

/// The Boiler's full chain, in the order of the `scene_pilots` "both" plan
/// reversed (the valve first, then the brace): wait left of the valve, kick the
/// goblin that comes through into the valve (steam, the brace only weakens),
/// then walk to the brace and jab it: the weight falls, lands on the drum and
/// the drum goes off.
fn chain(combat: &CombatState, sim: &SceneSim, exit_x: f32) -> RawInput {
    let tick = sim.world.tick + 1;
    let Some(env) = combat.env() else {
        return fight_or_walk(combat, sim, exit_x);
    };
    let hero = &sim.world.hero;
    match env.valve_state(0) {
        Some(ValveState::Intact) => {
            if tick > KICK_GIVE_UP_TICK {
                return valve(combat, sim, exit_x);
            }
            if kick_target(combat, sim) {
                return RawInput {
                    move_x: 1.0,
                    physical: tick.is_multiple_of(4),
                    ..RawInput::NONE
                };
            }
            if threatened(combat, sim) {
                return fight(combat, sim);
            }
            // Lure and kite: walk right to wake the sleepers, then back off left
            // of the valve while the nearest one follows, and kick it there.
            let follower = combat
                .enemies
                .iter()
                .filter(|g| {
                    g.alive()
                        && g.ai == crate::combat::EnemyAi::Chase
                        && g.x > hero.x
                        && g.x - hero.x < LURE_RANGE
                })
                .map(|g| g.x - hero.x)
                .min_by(f32::total_cmp);
            // A live party with an AI hero (not two players): the partner takes part.
            let ai_partner = sim.drive().live && !sim.drive().two_player;
            // The nearest live goblin ahead that no follower rule has claimed yet.
            let sleeper_ahead = combat
                .enemies
                .iter()
                .filter(|g| g.alive() && g.ai == crate::combat::EnemyAi::Chase && g.x > hero.x)
                .map(|g| g.x)
                .min_by(f32::total_cmp);
            match follower {
                Some(gap) if gap < KITE_GAP && hero.x > KICK_STAND_X => walk_to(sim, KICK_STAND_X),
                // With an AI partner on the field a goblin may target her and never come:
                // close in until it is a kite gap away. (An unaccompanied run, and the
                // two-player run, are unchanged: their committed pilot hashes pin them.)
                Some(gap) if ai_partner && gap > KITE_GAP + 10.0 => walk_to(sim, hero.x + 20.0),
                Some(_) => RawInput::NONE,
                // PTG fix (the chain wake step overshoot): `walk_to` stops within 4 px of its
                // target, so the old `hero.x < LURE_X` arm parked Carl on x 556..560 and held
                // him there forever. This arm ends where `walk_to` does; Carl then walks on
                // until a sleeper is within `LURE_RANGE` (a goblin wakes on a hero within 220
                // px, so a trigger column of the room data is not what wakes it).
                None if hero.x < LURE_X - WALK_DEADZONE => walk_to(sim, LURE_X),
                // FIX-TOOL: the walk-on is not gated on a live one-player party any more; with no
                // AI partner (the unaccompanied run, two players) it never fires in the
                // committed runs (their hashes did not move) and only guards the dead band.
                None => match sleeper_ahead {
                    Some(gx) if hero.x < gx - LURE_RANGE + LURE_MARGIN => {
                        walk_to(sim, gx - LURE_RANGE + 2.0 * LURE_MARGIN)
                    }
                    _ => RawInput::NONE,
                },
            }
        }
        // The steam runs: whoever comes is fought, nothing else.
        Some(ValveState::Warning | ValveState::Venting) => {
            if threatened(combat, sim) {
                fight(combat, sim)
            } else {
                RawInput::NONE
            }
        }
        _ => match env.mass_state(BRACE_PROP as usize) {
            Some(MassState::Supported | MassState::Weakened) => {
                if threatened(combat, sim) {
                    fight(combat, sim)
                } else {
                    approach(sim, BRACE_X).unwrap_or_else(|| jab(tick))
                }
            }
            // Released and falling: stand clear of the footprint and watch.
            Some(MassState::Released | MassState::Falling) => RawInput::NONE,
            _ => fight_or_walk(combat, sim, exit_x),
        },
    }
}

// ---------------------------------------------------------------- room 4 routes (PT-GATE-D)

/// Whether a slime of pocket A is alive (the second slime sleeps in pocket C).
fn haz_slime_alive(combat: &CombatState) -> bool {
    combat.slot_enemies().iter().any(|e| {
        e.kind() == crate::combat_scene::slime::KIND
            && e.alive()
            && e.as_any()
                .and_then(|a| a.downcast_ref::<crate::combat_scene::slime::SlimeEnemy>())
                .is_some_and(|s| s.slime().x < HAZ_POCKET_A_END)
    })
}

/// Spectacular route (room 4, FA-Q21): the PT-TUNE pilot clears pocket A; Carl then baits R1 and
/// backs off out of its blast (the tactical route's rule for sticker 0), walks to
/// [`HAZ_THROW_X`] and throws his one dynamite at the floor under R2: the stick lands within the
/// 76 px reach (`hazfu05`) of R2, its blast lights R2 (the CP-Q34 chain, 8 ticks later, credited
/// to the stick) and S2 at home stands inside R2's blast. Carl backs off down the lane while the
/// 150-tick fuse runs, and hands over to the PT-TUNE pilot once the dynamite has gone off.
fn haz_spectacular(combat: &CombatState, sim: &SceneSim) -> RawInput {
    let pilot = || crate::f01_mastery_fixtures::pilot(combat, sim, HeroId::Carl);
    let hero = &sim.world.hero;
    let pocket_a_clear = !haz_slime_alive(combat)
        && combat
            .enemies
            .iter()
            .filter(|g| g.x < HAZ_POCKET_A_END)
            .all(|g| !g.alive());
    if !pocket_a_clear {
        return pilot();
    }
    if let Some(stick) = combat.charges.get(combat.dynamite_slot()) {
        if stick.detonated {
            return pilot();
        }
        return walk_to(sim, HAZ_THROW_X - HAZ_BACK_OFF);
    }
    // R1 first: bait and back off out of its blast.
    if let Some((state, dome)) = haz_sticker_at(combat, 0) {
        match state {
            StickerState::Spent => {}
            StickerState::Warning => return walk_to(sim, dome - HAZ_RETREAT_PX),
            StickerState::Latched | StickerState::Scuttling => {
                return RawInput {
                    move_x: 1.0,
                    jump: hero.grounded && blocked_ahead(sim, 1.0),
                    ..RawInput::NONE
                };
            }
        }
    }
    // R2 spent already (nothing to chain): the PT-TUNE pilot finishes the room.
    if haz_sticker_at(combat, 1).is_none_or(|(state, _)| state == StickerState::Spent) {
        return pilot();
    }
    if hero.x < HAZ_THROW_X - WALK_DEADZONE {
        return walk_to(sim, HAZ_THROW_X);
    }
    RawInput {
        ability: true,
        ..RawInput::NONE
    }
}

/// Room 4's tactical route (HAZ-07 pattern 2): where Carl waits out a lit Rot Sticker
/// (the sheet's latest safe reversal is 19 ticks after the provoke; from the provoke
/// column the retreat is 40 px, about 13 ticks) and how far from the dome.
const HAZ_RETREAT_PX: f32 = 70.0;

/// The Rot Sticker `index` of the hazards room: its state and its dome column.
fn haz_sticker_at(combat: &CombatState, index: u16) -> Option<(StickerState, f32)> {
    let sticker = combat.env()?.sticker(index)?;
    Some((
        sticker.state(),
        sticker.dome(crate::rot_sticker::active()).0,
    ))
}

/// Tactical route (room 4, HAZ-07 pattern 2: bait R2 over the slime S2). The PT-TUNE
/// pilot clears pocket A; then, for each Rot Sticker in lane order (R1 of pocket B, R2
/// of pocket C), while it is not spent: walk right until it is provoked by proximity
/// (the 32 px trigger circle), and the moment it is `Warning` walk back left to
/// [`HAZ_RETREAT_PX`] from its dome and hold there until it blows. R2's blast then hits
/// S2 at home (47.5 px from the dome, inside the 48 px circle; the sheet has no goblin
/// collateral, Revision 4). Carl never jabs S2 before the blast. Once R2 is spent the
/// PT-TUNE pilot punishes the damaged slime, fights the C goblin and exits. A pure
/// function of the state.
fn haz_tactical(combat: &CombatState, sim: &SceneSim) -> RawInput {
    let pilot = || crate::f01_mastery_fixtures::pilot(combat, sim, HeroId::Carl);
    let hero = &sim.world.hero;
    let pocket_a_clear = !haz_slime_alive(combat)
        && combat
            .enemies
            .iter()
            .filter(|g| g.x < HAZ_POCKET_A_END)
            .all(|g| !g.alive());
    if !pocket_a_clear {
        return pilot();
    }
    for index in 0..2u16 {
        let Some((state, dome)) = haz_sticker_at(combat, index) else {
            continue;
        };
        match state {
            StickerState::Spent => continue,
            StickerState::Warning => return walk_to(sim, dome - HAZ_RETREAT_PX),
            StickerState::Latched | StickerState::Scuttling => {
                return RawInput {
                    move_x: 1.0,
                    jump: hero.grounded && blocked_ahead(sim, 1.0),
                    ..RawInput::NONE
                };
            }
        }
    }
    pilot()
}

/// DR-A: room 3's escape pilot. Run to `exit_x`; with a `hold_at` column, stop there until the
/// wheel front has come within [`crate::dozer_fixtures::CLOSE_GAP_PX`] of the hold column's body
/// edge, then run. Stateless: the release test reads the machine's wheel front, which only moves
/// forward, so a released pilot never stops again.
fn dozer_escape(
    combat: &CombatState,
    sim: &SceneSim,
    exit_x: f32,
    hold_at: Option<f32>,
) -> RawInput {
    use crate::dozer_fixtures::CLOSE_GAP_PX;
    let x = sim.world.hero.x;
    if let Some(hold) = hold_at
        && x >= hold
        && let Some(d) = crate::dozer_art::dozer_of(combat)
        && !(d.state.emerged() && d.wheel_front() >= hold - HERO_HALF_WIDTH - CLOSE_GAP_PX)
    {
        return RawInput::NONE;
    }
    walk_to(sim, exit_x)
}

/// DR-B: how far past the brace column Carl walks before he turns back to it.
pub const BRACE_TURN_PX: f32 = 24.0;

/// DR-B: the `dozer:brace` pilot (the live twin of `dz_frame_disable_once`'s brace pilot,
/// [`crate::dozer_fixtures::frame::brace_pilot`]). Carl walks to
/// [`crate::dozer_fixtures::WAIT_X`] + [`BRACE_TURN_PX`] and back to the brace column, so he
/// stands there facing the brace. On the first committed-chase tick it aims the release at the
/// middle of the helper's useful window ([`crate::dozer::release_window`] for a hero at the
/// brace, read from the live frame) and presses attack on the two ticks that release it then.
/// Once the machine is stunned, disabled or escaped Carl runs to the exit; a wheel within
/// [`crate::dozer_fixtures::CLOSE_GAP_PX`] of him sends him running too (the fallback of a
/// missed landing).
fn dozer_brace(
    combat: &CombatState,
    sim: &SceneSim,
    exit_x: f32,
    memory: &mut AlignedMemory,
) -> RawInput {
    use crate::dozer::DozerState;
    use crate::dozer_fixtures::frame::{JAB_PERIOD, JAB_TO_RELEASE};
    use crate::dozer_fixtures::{CLOSE_GAP_PX, WAIT_X};
    let Some(d) = crate::dozer_art::dozer_of(combat) else {
        return walk_to(sim, exit_x);
    };
    match d.state {
        DozerState::ImpactStunned { .. } | DozerState::Disabled | DozerState::Escaped(_) => {
            return walk_to(sim, exit_x);
        }
        _ if d.state.emerged() && d.wheel_front() >= WAIT_X - HERO_HALF_WIDTH - CLOSE_GAP_PX => {
            return walk_to(sim, exit_x);
        }
        DozerState::CommittedChase if memory.dozer_chase.is_none() => {
            let aim = crate::f01_dozer::stop_frame_of(combat).and_then(|frame| {
                let w = crate::dozer::release_window(
                    &d.profile,
                    &frame,
                    WAIT_X,
                    u32::from(d.tuning.dzq23_melee_recovery_ticks),
                );
                Some(u64::from(w.first_tick? + w.last_tick?) / 2)
            });
            // No window: aim past the run's end (the fallback runs him out).
            memory.dozer_chase = Some((sim.world.tick, aim.unwrap_or(u64::MAX / 2)));
        }
        _ => {}
    }
    if !memory.dozer_turned {
        if sim.world.hero.x < WAIT_X + BRACE_TURN_PX - WALK_DEADZONE {
            return walk_to(sim, WAIT_X + BRACE_TURN_PX);
        }
        memory.dozer_turned = true;
    }
    if !memory.dozer_arrived {
        if (sim.world.hero.x - WAIT_X).abs() >= WALK_DEADZONE {
            return walk_to(sim, WAIT_X);
        }
        memory.dozer_arrived = true;
    }
    let tick = sim.world.tick + 1;
    let jab = memory.dozer_chase.is_some_and(|(chase0, r)| {
        let second = (chase0 + r).saturating_sub(JAB_TO_RELEASE);
        tick == second || tick + JAB_PERIOD == second
    });
    RawInput {
        attack: jab,
        ..RawInput::NONE
    }
}

/// DR-D: the `dozer:retry` pilot. Before the first defeat Carl walks to
/// [`crate::dozer_fixtures::HOLD_X`] and stands there (the wheel takes him); once the room has
/// counted a defeat (`deaths`, kept across the respawn) he walks on from the respawn column into
/// the trigger, stands while the gate rumbles until the machine is out of it again (the gate's
/// splinter bursts are on screen) and then runs to the exit. Stateless.
fn dozer_retry(combat: &CombatState, sim: &SceneSim, exit_x: f32) -> RawInput {
    use crate::dozer::DozerState;
    use crate::dozer_fixtures::HOLD_X;
    if combat.deaths > 0 {
        let rumbling = crate::dozer_art::dozer_of(combat)
            .is_some_and(|d| matches!(d.state, DozerState::Approaching { .. }));
        return if rumbling {
            RawInput::NONE
        } else {
            walk_to(sim, exit_x)
        };
    }
    if (sim.world.hero.x - HOLD_X).abs() >= WALK_DEADZONE {
        return walk_to(sim, HOLD_X);
    }
    RawInput::NONE
}

/// DR-D: the `dozer:crew` pilot. The `dozer:brace` pilot until the machine is disabled; then
/// the CF02 fight against the dismounted riders while one lives, and the walk to the exit.
fn dozer_crew(
    combat: &CombatState,
    sim: &SceneSim,
    exit_x: f32,
    memory: &mut AlignedMemory,
) -> RawInput {
    use crate::dozer::DozerState;
    let disabled =
        crate::dozer_art::dozer_of(combat).is_some_and(|d| d.state == DozerState::Disabled);
    if disabled {
        fight_or_walk(combat, sim, exit_x)
    } else {
        dozer_brace(combat, sim, exit_x, memory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::combat::{CombatEvent, DefeatCause, PropPhase};
    use crate::f01_boiler::Roster;
    use crate::f01_late::{Loadout, boiler_start_with};
    use std::path::{Path, PathBuf};

    /// The Boiler under the story loadout with its live roster, played by `plan`
    /// until the exit or 1500 ticks: the scene, the combat state and the exit column.
    fn play(plan: Plan) -> (SceneSim, CombatState) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = Loadout::Story.flags();
        let (run, mut scene, mut combat) =
            boiler_start_with(&root, 360, carl, donut, None, Roster::Live).unwrap();
        let exit = run.loaded.encounter.exit;
        let exit_x = (exit.x + exit.w / 2) as f32;
        let pilot = RoomPilot {
            plan,
            two_player: false,
            aligned: None,
        };
        for _ in 0..1500 {
            if scene.world.complete {
                break;
            }
            let inputs = pilot.inputs(&combat, &scene, exit_x);
            combat.step_party(&mut scene, inputs);
        }
        (scene, combat)
    }

    fn phase_ticks(combat: &CombatState, prop: u16, phase: PropPhase) -> Vec<u64> {
        combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::PropPhase {
                    tick,
                    prop: p,
                    phase: ph,
                    ..
                } if *p == prop && *ph == phase => Some(*tick),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn every_plan_clears_the_live_boiler_without_a_defeat_and_replays_identically() {
        for plan in [Plan::Direct, Plan::Valve, Plan::Chain] {
            let (scene, combat) = play(plan);
            assert!(scene.world.complete, "{plan:?} reaches the exit");
            assert_eq!(combat.deaths, 0, "{plan:?} is never defeated");
            assert!(
                combat.enemies.iter().all(|g| !g.alive()),
                "{plan:?} leaves nobody alive behind it"
            );
            let (again, combat_again) = play(plan);
            assert_eq!(
                combat.state_hash(&scene),
                combat_again.state_hash(&again),
                "{plan:?} replays to the same hash"
            );
        }
    }

    /// FIX-TOOL (PT-FU-B8 item 10): the chain plan's walk-on arm is no longer gated on a live
    /// one-player party. On the live Boiler with no AI partner on the field (the unaccompanied
    /// legacy run, and two players) at seeds 0..=8, the plan reaches the exit within 3000
    /// ticks, Carl never stands still for more than 300 ticks, and never parks in the old
    /// dead band (x 556 to 560, short of the room data's 560 trigger) for more than 30 ticks.
    /// Ungating moved no pin: the 18 runs hash as before (the committed `pilot_hashes.txt`).
    #[test]
    fn the_chain_plan_without_an_ai_partner_never_parks_in_the_dead_band() {
        use crate::party::PROVISIONAL_TUNING;
        use crate::party_scene::{PartyFixture, PartyFixtureSim};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = Loadout::Story.flags();
        for two_player in [false, true] {
            for seed in 0..=8u64 {
                let (run, mut scene, mut combat) =
                    boiler_start_with(&root, 360, carl, donut, Some(seed), Roster::Live).unwrap();
                let exit = run.loaded.encounter.exit;
                let exit_x = (exit.x + exit.w / 2) as f32;
                let pilot = RoomPilot {
                    plan: Plan::Chain,
                    two_player,
                    aligned: None,
                };
                let (mut last, mut still, mut max_still, mut band) =
                    (scene.world.hero.x, 0u32, 0u32, 0u32);
                let mut max_band = 0u32;
                let mut watch = |x: f32| {
                    still = if (x - last).abs() < 0.1 { still + 1 } else { 0 };
                    band = if still > 0 && (LURE_X - 4.0..=LURE_X).contains(&x) {
                        band + 1
                    } else {
                        0
                    };
                    max_still = max_still.max(still);
                    max_band = max_band.max(band);
                    last = x;
                };
                let complete = if two_player {
                    let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
                        base: run.fixture,
                        two_player: true,
                        tuning: PROVISIONAL_TUNING,
                        script: &[],
                        pilot: crate::boss_scene::Pilot::ArenaDirect,
                        donut: crate::donut_mana::DonutSetup::unlocked(8),
                    }));
                    let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
                    for _ in 0..3000 {
                        if sim.scene.world.complete {
                            break;
                        }
                        let slots = pilot.inputs(&sim.combat, &sim.scene, exit_x);
                        sim.step_slots(slots);
                        watch(sim.scene.world.hero.x);
                    }
                    sim.scene.world.complete
                } else {
                    for _ in 0..3000 {
                        if scene.world.complete {
                            break;
                        }
                        let inputs = pilot.inputs(&combat, &scene, exit_x);
                        combat.step_party(&mut scene, inputs);
                        watch(scene.world.hero.x);
                    }
                    scene.world.complete
                };
                let who = format!("two_player={two_player} seed {seed}");
                assert!(complete, "{who}: no exit in 3000 ticks");
                assert!(
                    max_still <= 300,
                    "{who}: Carl stood still {max_still} ticks"
                );
                assert!(
                    max_band <= 30,
                    "{who}: parked {max_band} ticks in the dead band"
                );
            }
        }
    }

    /// PTG fix (the chain wake step overshoot): with a live party the chain plan used to
    /// park Carl on x 556..560, short of the room data's 560 wake trigger, and never
    /// reached the exit. On the live Boiler roster at seeds 0..=8, with the `cf02_assist`
    /// partner and with the autonomous one, the plan reaches the exit within 3000 ticks and
    /// Carl is never told to walk and stays still for more than 300 ticks.
    ///
    /// F01-FU-B10 (#250 review): runs with FA-Q16 (prop-safe fire) on, the shipped
    /// value, and off. PR-A pinned it off because seed 0 stayed at the valve; on
    /// the current main the plan's own fallback (`KICK_GIVE_UP_TICK`: Carl jabs the
    /// valve himself) clears every run with the knob on, so the override is gone.
    #[test]
    fn the_chain_plan_with_a_party_reaches_the_exit_on_the_live_boiler_at_every_seed() {
        for fa_q16 in [true, false] {
            crate::combat_scene::with_profile_override(
                |f| {
                    f.cf02_assist.fix_all.fa_q16_prop_safe_fire = fa_q16;
                    f.cf02_free.fix_all.fa_q16_prop_safe_fire = fa_q16;
                },
                || chain_plan_reaches_the_exit_at_every_seed(fa_q16),
            );
        }
    }

    fn chain_plan_reaches_the_exit_at_every_seed(fa_q16: bool) {
        use crate::f01_mastery_fixtures::{Who, tuning};
        use crate::party_scene::{PartyFixture, PartyFixtureSim};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = Loadout::Story.flags();
        let pilot = RoomPilot {
            plan: Plan::Chain,
            two_player: false,
            aligned: None,
        };
        // The live room's partners, as the `boiler_fixtures` live pilots run them:
        // the `cf02_assist` partner and the autonomous `cf02_free` one.
        for tuning in [tuning(Who::CarlSolo), tuning(Who::Companion)] {
            for seed in 0..=8u64 {
                let (run, scene, combat) =
                    boiler_start_with(&root, 360, carl, donut, Some(seed), Roster::Live).unwrap();
                let exit = run.loaded.encounter.exit;
                let exit_x = (exit.x + exit.w / 2) as f32;
                let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
                    base: run.fixture,
                    two_player: false,
                    tuning,
                    script: &[],
                    pilot: crate::boss_scene::Pilot::ArenaDirect,
                    donut: crate::donut_mana::DonutSetup::unlocked(8),
                }));
                let mut sim = PartyFixtureSim::from_parts(scene, combat, fixture);
                let (mut last_x, mut still, mut max_still) = (sim.scene.world.hero.x, 0u32, 0u32);
                for _ in 0..3000 {
                    if sim.scene.world.complete {
                        break;
                    }
                    let slots = pilot.inputs(&sim.combat, &sim.scene, exit_x);
                    sim.step_slots(slots);
                    let x = sim.scene.world.hero.x;
                    let walking = slots[0].move_x != 0.0;
                    still = if walking && (x - last_x).abs() < 0.1 {
                        still + 1
                    } else {
                        0
                    };
                    max_still = max_still.max(still);
                    last_x = x;
                }
                assert!(
                    sim.scene.world.complete,
                    "FA-Q16 {fa_q16} seed {seed} {tuning:?}: no exit in 3000 ticks (Carl at x {})",
                    sim.scene.world.hero.x
                );
                assert!(
                    max_still <= 300,
                    "FA-Q16 {fa_q16} seed {seed}: Carl stuck for {max_still} ticks"
                );
                println!(
                    "CHAIN_LIVE fa_q16={fa_q16} who={tuning:?} seed={seed} exit_tick={}",
                    sim.scene.world.tick
                );
            }
        }
    }

    #[test]
    fn the_valve_plan_spends_the_valve_and_the_direct_plan_touches_no_prop() {
        let (_, valve) = play(Plan::Valve);
        assert_eq!(phase_ticks(&valve, 0, PropPhase::Spent).len(), 1);
        let (_, direct) = play(Plan::Direct);
        assert!(
            !direct.events.iter().any(|e| matches!(
                e,
                CombatEvent::PropHit { .. } | CombatEvent::PropPhase { .. }
            )),
            "the direct plan walks past every prop"
        );
    }

    /// The whole chain in order, each link once: a launched goblin sets the valve
    /// off, the steam reaches the brace, a jab lets it go, the weight lands on the
    /// drum and the drum's blast defeats a goblin, credited to Carl.
    #[test]
    fn the_chain_plan_runs_launch_steam_brace_landing_drum_in_order() {
        let (_, chain) = play(Plan::Chain);
        let launched = chain.events.iter().any(|e| {
            matches!(e, CombatEvent::PropHit { prop: 0, kind, .. }
                if format!("{kind:?}") == "LaunchedBody")
        });
        assert!(launched, "a launched goblin, not a jab, sets the valve off");
        let at = |prop, phase| {
            let ticks = phase_ticks(&chain, prop, phase);
            assert_eq!(ticks.len(), 1, "{prop} {phase:?} once: {ticks:?}");
            ticks[0]
        };
        let order = [
            at(0, PropPhase::Warning),
            at(0, PropPhase::Venting),
            at(1, PropPhase::Released),
            at(1, PropPhase::Falling),
            at(1, PropPhase::Landed),
            at(2, PropPhase::Detonated),
        ];
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
        let blast_kills = chain
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::EnemyDefeated {
                        cause: DefeatCause::Explosion { .. },
                        ..
                    }
                )
            })
            .count();
        assert_eq!(blast_kills, 1, "the drum defeats exactly one goblin");
    }

    /// PT-GATE-A: the juicer plan is the PT-SCENE-B solo-Carl pilot (seed 0): the
    /// same inputs, so the same final state as `juicer_pilot`, the gauntlet and the
    /// blink granted once each at the defeat.
    #[test]
    fn the_juicer_plan_is_the_pt_scene_b_solo_carl_pilot() {
        use crate::f01_late_fixtures::{JuicerWho, juicer_pilot, juicer_room};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let report = juicer_pilot(&root, JuicerWho::CarlSolo, 0).unwrap();
        let pilot = RoomPilot {
            plan: Plan::Juicer,
            two_player: false,
            aligned: None,
        };
        let mut sim = juicer_room(&root).unwrap();
        for _ in 0..report.ticks {
            let inputs = pilot.inputs(&sim.combat, &sim.scene, 0.0);
            sim.step_slots(inputs);
        }
        assert_eq!(sim.state_hash(), report.hash);
        assert!(report.defeated_tick.is_some() && report.exit_tick.is_some());
        assert_eq!(
            report.grants.len(),
            2,
            "the gauntlet and the blink: {:?}",
            report.grants
        );
    }

    /// PT-GATE-A: the mastery plan is the PT-MASTERY Carl pilot, so a scene run by
    /// it clears the roster, reaches the exit and replays to the same hash.
    #[test]
    fn the_mastery_plan_is_the_pt_mastery_carl_pilot_and_replays_identically() {
        use crate::f01_late::mastery_start;
        use crate::f01_mastery_fixtures::{EXIT_X, RUN_TICKS};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let play = || {
            let (carl, donut) = Loadout::Story.flags();
            let (_run, mut scene, mut combat) =
                mastery_start(&root, 360, carl, donut, Some(0)).unwrap();
            let pilot = RoomPilot {
                plan: Plan::Mastery,
                two_player: false,
                aligned: None,
            };
            for _ in 0..RUN_TICKS {
                if combat.roster_cleared() && scene.world.hero.x >= EXIT_X {
                    break;
                }
                let inputs = pilot.inputs(&combat, &scene, 0.0);
                combat.step_party(&mut scene, inputs);
            }
            (
                combat.roster_cleared(),
                scene.world.hero.x,
                combat.state_hash(&scene),
            )
        };
        let (cleared, x, hash) = play();
        assert!(cleared && x >= EXIT_X, "cleared={cleared} x={x}");
        assert_eq!(play().2, hash);
    }

    /// PT-GATE-C1: an aligned room pilot plays the library fixture's fight: for
    /// every (who, seed) of the Juicer the room pilot's final state hash equals
    /// the PT-SCENE-B fixture's (27 runs), and for every (who, seed) of the
    /// closing arena it equals the PT-TUNE fixture's (36 runs). The room pilot
    /// reads the party's controlled hero and its own swap flag, nothing else.
    #[test]
    fn an_aligned_room_pilot_equals_the_library_fixture_for_every_who_and_seed() {
        use crate::f01_late_fixtures::{
            JUICER_COMPANION_PARTY, JUICER_ROOM_PARTY, JUICER_SEEDS, JuicerWho, juicer_pilot,
            juicer_room_for,
        };
        use crate::f01_mastery_fixtures as mastery;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut equal = 0;
        for (who, library_who) in [
            (AlignedWho::CarlSolo, JuicerWho::CarlSolo),
            (AlignedWho::DonutBySwap, JuicerWho::DonutBySwap),
            (AlignedWho::Companion, JuicerWho::Companion),
        ] {
            for seed in JUICER_SEEDS {
                let report = juicer_pilot(&root, library_who, seed).unwrap();
                let pilot = RoomPilot::parse(&format!("juicer:{}:{seed}", who.token())).unwrap();
                assert_eq!(pilot.label(), format!("juicer:{}:{seed}", who.token()));
                let party = if who == AlignedWho::Companion {
                    &JUICER_COMPANION_PARTY
                } else {
                    &JUICER_ROOM_PARTY
                };
                assert_eq!(pilot.tuning(), Some(party.tuning));
                let mut sim = juicer_room_for(&root, party).unwrap();
                let mut sent = AlignedMemory::default();
                for _ in 0..report.ticks {
                    let controlled = sim.party.controlled_hero();
                    let slots =
                        pilot.aligned_inputs(&sim.combat, &sim.scene, controlled, 0.0, &mut sent);
                    sim.step_slots(slots);
                }
                assert_eq!(sim.state_hash(), report.hash, "juicer {who:?} seed {seed}");
                equal += 1;
            }
        }
        for who in [
            AlignedWho::CarlSolo,
            AlignedWho::DonutBySwap,
            AlignedWho::DonutFights,
            AlignedWho::Companion,
        ] {
            for seed in mastery::SEEDS {
                let report = mastery::run(&root, who.mastery(), seed).unwrap();
                let pilot = RoomPilot::parse(&format!("mastery:{}:{seed}", who.token())).unwrap();
                assert_eq!(pilot.tuning(), Some(mastery::tuning(who.mastery())));
                let (mut sim, _) = mastery::start(&root, who.mastery(), seed).unwrap();
                let mut sent = AlignedMemory::default();
                for _ in 0..report.ticks {
                    let controlled = sim.party.controlled_hero();
                    let slots =
                        pilot.aligned_inputs(&sim.combat, &sim.scene, controlled, 0.0, &mut sent);
                    sim.step_slots(slots);
                }
                assert_eq!(sim.state_hash(), report.hash, "mastery {who:?} seed {seed}");
                equal += 1;
            }
        }
        assert_eq!(equal, 63, "27 Juicer runs and 36 arena runs");
    }

    /// PT-GATE-E: the three heroes' modes the hazards and Boiler library pilots play.
    const LIVE_WHO: [(AlignedWho, crate::hazards_fixtures::pilots::Who); 3] = [
        (
            AlignedWho::CarlSolo,
            crate::hazards_fixtures::pilots::Who::CarlSolo,
        ),
        (
            AlignedWho::DonutBySwap,
            crate::hazards_fixtures::pilots::Who::DonutBySwap,
        ),
        (
            AlignedWho::Companion,
            crate::hazards_fixtures::pilots::Who::Companion,
        ),
    ];

    fn boiler_who(who: AlignedWho) -> crate::boiler_fixtures::scene_pilots::Who {
        use crate::boiler_fixtures::scene_pilots::Who;
        match who {
            AlignedWho::DonutBySwap => Who::DonutBySwap,
            AlignedWho::Companion => Who::Companion,
            _ => Who::CarlSolo,
        }
    }

    /// The hazards room played by the aligned room pilot `hazards:<who>:<seed>` for
    /// `ticks` ticks, the way the scene steps it: (the party hash the library reports,
    /// the combat hash the window prints).
    fn replay_hazards(root: &Path, who: AlignedWho, seed: u64, ticks: u64) -> (u64, u64) {
        use crate::hazards_fixtures::pilots;
        let library_who = LIVE_WHO.iter().find(|(w, _)| *w == who).unwrap().1;
        let pilot = RoomPilot::parse(&format!("hazards:{}:{seed}", who.token())).unwrap();
        assert_eq!(
            pilot.tuning(),
            Some(crate::f01_mastery_fixtures::tuning(who.mastery()))
        );
        let (mut sim, _) = pilots::start(root, library_who, seed, true).unwrap();
        let mut memory = AlignedMemory::default();
        for _ in 0..ticks {
            let controlled = sim.party.controlled_hero();
            let slots = pilot.aligned_inputs(&sim.combat, &sim.scene, controlled, 0.0, &mut memory);
            sim.step_slots(slots);
        }
        (sim.state_hash(), sim.combat.state_hash(&sim.scene))
    }

    /// The Boiler's live room played by `boiler:<who>:<route>:<seed>` for `ticks` ticks.
    fn replay_boiler(
        root: &Path,
        who: AlignedWho,
        route: BoilerRoute,
        seed: u64,
        ticks: u64,
    ) -> (u64, u64) {
        use crate::boiler_fixtures::scene_pilots::live_start;
        let pilot = RoomPilot::parse(&format!(
            "boiler:{}:{}:{seed}",
            who.token(),
            route_token(route)
        ))
        .unwrap();
        let (mut sim, exit_x) = live_start(root, boiler_who(who), seed).unwrap();
        let mut memory = AlignedMemory::default();
        for _ in 0..ticks {
            let controlled = sim.party.controlled_hero();
            let slots =
                pilot.aligned_inputs(&sim.combat, &sim.scene, controlled, exit_x, &mut memory);
            sim.step_slots(slots);
        }
        (sim.state_hash(), sim.combat.state_hash(&sim.scene))
    }

    /// PT-GATE-E (item 4b): the room-pilot hash equals the library fixture's for the 27
    /// live room 4 runs (`hazards_fixtures::pilots::run`, which also equals `SOLO_CARL_PINNED`
    /// and `DIRECT_PINNED` for solo Carl) and for the 54 live room 2 runs
    /// (`boiler_fixtures::scene_pilots::run_live`).
    #[test]
    fn the_aligned_hazards_and_boiler_pilots_equal_their_library_fixtures() {
        use crate::boiler_fixtures::scene_pilots::run_live;
        use crate::hazards_fixtures::pilots;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut equal = 0;
        for (who, library_who) in LIVE_WHO {
            for seed in pilots::SEEDS {
                let report = pilots::run(&root, library_who, seed, true).unwrap();
                let (hash, _) = replay_hazards(&root, who, seed, report.ticks);
                assert_eq!(hash, report.hash, "hazards {who:?} seed {seed}");
                if who == AlignedWho::CarlSolo {
                    // The aligned direct route (HAZ-06) is the pinned `haz_room_direct_route`.
                    assert_eq!(
                        hash,
                        crate::hazards_fixtures::tests_c::SOLO_CARL_PINNED[seed as usize],
                        "hazards:carl_solo:{seed} equals the pinned direct route"
                    );
                }
                equal += 1;
            }
        }
        assert_eq!(equal, 27, "3 modes x 9 seeds on room 4");
        for who in [
            AlignedWho::CarlSolo,
            AlignedWho::DonutBySwap,
            AlignedWho::Companion,
        ] {
            for route in [BoilerRoute::Direct, BoilerRoute::Both] {
                for seed in 0..=8 {
                    let report = run_live(&root, boiler_who(who), route, seed).unwrap();
                    let ticks = report.exit_tick.expect("a live run reaches the exit");
                    let (hash, _) = replay_boiler(&root, who, route, seed, ticks);
                    assert_eq!(hash, report.hash, "boiler {who:?} {route:?} seed {seed}");
                    equal += 1;
                }
            }
        }
        assert_eq!(equal, 81, "27 room 4 runs and 54 room 2 runs");
    }

    /// PT-GATE-D: the library side of the render-loop replay
    /// (`scripts/make_f01_gate_d_replay.sh`): one `ALIGNED_LIBRARY` line for each of
    /// the 63 aligned runs with the tick it ends on, the library fixture's final hash
    /// (the value `ROOM_PILOT_STATE` prints) and the combat hash of the same state
    /// (the value `COMBAT_STATE` prints, in the headless run and in the window run).
    /// The aligned pilot is replayed here and asserted equal to the library hash.
    /// Ignored: it only prints.
    #[test]
    #[ignore = "prints the library hashes for scripts/make_f01_gate_d_replay.sh"]
    fn prints_the_aligned_library_hashes() {
        use crate::f01_late_fixtures::{
            JUICER_COMPANION_PARTY, JUICER_ROOM_PARTY, JUICER_SEEDS, JuicerWho, juicer_pilot,
            juicer_room_for,
        };
        use crate::f01_mastery_fixtures as mastery;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        println!("ALIGNED_LIBRARY_BEGIN");
        for (who, library_who) in [
            (AlignedWho::CarlSolo, JuicerWho::CarlSolo),
            (AlignedWho::DonutBySwap, JuicerWho::DonutBySwap),
            (AlignedWho::Companion, JuicerWho::Companion),
        ] {
            for seed in JUICER_SEEDS {
                let report = juicer_pilot(&root, library_who, seed).unwrap();
                let pilot = RoomPilot::parse(&format!("juicer:{}:{seed}", who.token())).unwrap();
                let party = if who == AlignedWho::Companion {
                    &JUICER_COMPANION_PARTY
                } else {
                    &JUICER_ROOM_PARTY
                };
                let mut sim = juicer_room_for(&root, party).unwrap();
                let mut sent = AlignedMemory::default();
                for _ in 0..report.ticks {
                    let controlled = sim.party.controlled_hero();
                    let slots =
                        pilot.aligned_inputs(&sim.combat, &sim.scene, controlled, 0.0, &mut sent);
                    sim.step_slots(slots);
                }
                assert_eq!(sim.state_hash(), report.hash, "juicer {who:?} seed {seed}");
                println!(
                    "ALIGNED_LIBRARY room=juicer who={} seed={seed} ticks={} hash={:016x} combat_hash={:016x}",
                    who.token(),
                    report.ticks,
                    report.hash,
                    sim.combat.state_hash(&sim.scene)
                );
            }
        }
        for who in [
            AlignedWho::CarlSolo,
            AlignedWho::DonutBySwap,
            AlignedWho::DonutFights,
            AlignedWho::Companion,
        ] {
            for seed in mastery::SEEDS {
                let report = mastery::run(&root, who.mastery(), seed).unwrap();
                let pilot = RoomPilot::parse(&format!("mastery:{}:{seed}", who.token())).unwrap();
                let (mut sim, _) = mastery::start(&root, who.mastery(), seed).unwrap();
                let mut sent = AlignedMemory::default();
                for _ in 0..report.ticks {
                    let controlled = sim.party.controlled_hero();
                    let slots =
                        pilot.aligned_inputs(&sim.combat, &sim.scene, controlled, 0.0, &mut sent);
                    sim.step_slots(slots);
                }
                assert_eq!(sim.state_hash(), report.hash, "mastery {who:?} seed {seed}");
                println!(
                    "ALIGNED_LIBRARY room=mastery who={} seed={seed} ticks={} hash={:016x} combat_hash={:016x}",
                    who.token(),
                    report.ticks,
                    report.hash,
                    sim.combat.state_hash(&sim.scene)
                );
            }
        }
        // PT-GATE-E: the live room 4 and room 2 runs of #240.
        for (who, library_who) in LIVE_WHO {
            for seed in crate::hazards_fixtures::pilots::SEEDS {
                let report =
                    crate::hazards_fixtures::pilots::run(&root, library_who, seed, true).unwrap();
                let (hash, combat_hash) = replay_hazards(&root, who, seed, report.ticks);
                assert_eq!(hash, report.hash, "hazards {who:?} seed {seed}");
                println!(
                    "ALIGNED_LIBRARY room=hazards who={} seed={seed} ticks={} hash={hash:016x} combat_hash={combat_hash:016x}",
                    who.token(),
                    report.ticks,
                );
            }
        }
        for who in [
            AlignedWho::CarlSolo,
            AlignedWho::DonutBySwap,
            AlignedWho::Companion,
        ] {
            for route in [BoilerRoute::Direct, BoilerRoute::Both] {
                for seed in 0..=8 {
                    let report = crate::boiler_fixtures::scene_pilots::run_live(
                        &root,
                        boiler_who(who),
                        route,
                        seed,
                    )
                    .unwrap();
                    let ticks = report.exit_tick.expect("a live run reaches the exit");
                    let (hash, combat_hash) = replay_boiler(&root, who, route, seed, ticks);
                    assert_eq!(hash, report.hash, "boiler {who:?} {route:?} seed {seed}");
                    println!(
                        "ALIGNED_LIBRARY room=boiler who={} route={} seed={seed} ticks={ticks} hash={hash:016x} combat_hash={combat_hash:016x}",
                        who.token(),
                        route_token(route),
                    );
                }
            }
        }
        // FA-Q24: room 1, the Hoarder (solo Carl, nine seeds).
        for seed in crate::hoarder_fixtures::ROOM_SEEDS {
            let report = crate::hoarder_fixtures::room_run(&root, seed).unwrap();
            let (hash, combat_hash) = replay_hoarder(&root, seed, report.ticks);
            assert_eq!(hash, report.hash, "hoarder seed {seed}");
            println!(
                "ALIGNED_LIBRARY room=hoarder who=carl_solo seed={seed} ticks={} hash={hash:016x} combat_hash={combat_hash:016x}",
                report.ticks
            );
        }
        println!("ALIGNED_LIBRARY_END");
    }

    /// Room 1 played by the aligned room pilot `hoarder:carl_solo:<seed>` for `ticks` ticks the way
    /// the scene steps it (the `f01_late` constructor, the pilot's party, the story loadout): the
    /// party hash the library reports and the combat hash the window prints.
    fn replay_hoarder(root: &Path, seed: u64, ticks: u64) -> (u64, u64) {
        let pilot = RoomPilot::parse(&format!("hoarder:carl_solo:{seed}")).unwrap();
        let (carl, donut) = crate::f01_late::Loadout::Story.flags();
        let (_, mut scene, mut combat) =
            crate::f01_late::hoarder_start(root, 360, carl, donut).unwrap();
        let mut party = crate::party::PartyState::new(pilot.tuning().unwrap());
        let mut memory = AlignedMemory::default();
        for _ in 0..ticks {
            let slots =
                pilot.aligned_inputs(&combat, &scene, party.controlled_hero(), 0.0, &mut memory);
            let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
            party.request_swap(slots[0].swap, hurt);
            let inputs = party.hero_inputs(slots);
            scene.set_drive(party.drive(true));
            let restarts = combat.restarts;
            combat.step_party(&mut scene, inputs);
            if combat.restarts != restarts {
                party.restart();
            }
        }
        (
            crate::party_scene::party_hash(&combat, &scene, &party),
            combat.state_hash(&scene),
        )
    }

    /// Room 0 played by the aligned room pilot `plan` for `ticks` ticks the way the scene steps it
    /// (the `f01_late` constructor at the pilot's seed, the pilot's party, the story loadout): the
    /// party hash, the combat hash and the combat state.
    fn replay_corral(root: &Path, plan: &str, ticks: u64) -> (u64, u64, CombatState) {
        replay_corral_observed(root, plan, ticks, |_, _| {})
    }

    /// [`replay_corral`] calling `observe(tick, combat)` after each of the `ticks` steps (`tick` is the
    /// 1-based step count, the tick number a capture of that step carries).
    fn replay_corral_observed(
        root: &Path,
        plan: &str,
        ticks: u64,
        mut observe: impl FnMut(u64, &CombatState),
    ) -> (u64, u64, CombatState) {
        let pilot = RoomPilot::parse(plan).unwrap();
        let seed = pilot.aligned.unwrap().seed;
        let (carl, donut) = crate::f01_late::Loadout::Story.flags();
        let (run, mut scene, mut combat) =
            crate::f01_late::corral_start(root, 360, carl, donut, Some(seed)).unwrap();
        let exit_x = crate::f01_corral_fixtures::Route::of(&run).exit_x;
        let mut party = crate::party::PartyState::new(pilot.tuning().unwrap());
        let mut memory = AlignedMemory::default();
        for tick in 1..=ticks {
            let slots = pilot.aligned_inputs(
                &combat,
                &scene,
                party.controlled_hero(),
                exit_x,
                &mut memory,
            );
            let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
            party.request_swap(slots[0].swap, hurt);
            let inputs = party.hero_inputs(slots);
            scene.set_drive(party.drive(true));
            let restarts = combat.restarts;
            combat.step_party(&mut scene, inputs);
            if combat.restarts != restarts {
                party.restart();
            }
            observe(tick, &combat);
        }
        (
            crate::party_scene::party_hash(&combat, &scene, &party),
            combat.state_hash(&scene),
            combat,
        )
    }

    /// CORRAL-F: the room 0 pilots parse, label back and refuse what they cannot play; the plain
    /// `corral` plan is `corral:carl_solo:0`, and each who runs the corral fixture's party.
    #[test]
    fn the_corral_plans_parse_label_and_refuse_what_they_cannot_play() {
        let plain = RoomPilot::parse("corral").unwrap();
        assert_eq!(plain.plan, Plan::Corral);
        assert_eq!(
            plain.aligned,
            Some(Aligned {
                who: AlignedWho::CarlSolo,
                seed: 0,
                route: None
            })
        );
        assert_eq!(plain.label(), "corral");
        let rush = RoomPilot::parse("corral_rush").unwrap();
        assert_eq!(rush.plan, Plan::CorralRush);
        assert_eq!(rush.label(), "corral_rush");
        assert_eq!(rush.aligned, plain.aligned);
        assert_eq!(rush.tuning(), Some(corral_rush_tuning()));
        for value in [
            "corral:carl_solo:7",
            "corral:donut_fights:3",
            "corral:companion:8",
        ] {
            let pilot = RoomPilot::parse(value).unwrap();
            assert_eq!(pilot.label(), value);
            assert_eq!(RoomPilot::parse(&pilot.label()).unwrap(), pilot);
            let who = corral_who(pilot.aligned.unwrap().who);
            assert_eq!(pilot.tuning(), Some(who.tuning()), "{value}");
        }
        for bad in [
            "corral:donut_by_swap:1",
            "corral:2p",
            "corral_rush:2p",
            "corral_rush:carl_solo:1",
            "corral:carl_solo:x",
            "corral:carl_solo",
        ] {
            assert!(RoomPilot::parse(bad).is_err(), "{bad}");
        }
    }

    /// CORRAL-F (plan section 10, item 5): the corral room pilot replays the library run of
    /// `f01_corral_fixtures` tick for tick for solo Carl, solo Donut (the swap on tick 30) and
    /// the `cf02_free` companion at seeds 0..=8: the same party hash on the exit tick, the
    /// roster cleared, no defeat, no grant, no hero stuck past 300 ticks, nobody past the
    /// closed M3 gate.
    #[test]
    fn the_corral_room_pilot_replays_the_library_run_at_every_gate_seed() {
        use crate::f01_corral_fixtures::{self as corral, STUCK_LIMIT};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for who in [
            AlignedWho::CarlSolo,
            AlignedWho::DonutFights,
            AlignedWho::Companion,
        ] {
            for seed in corral::GATE_SEEDS {
                let tag = format!("corral:{}:{seed}", who.token());
                let report = corral::run(&root, corral_who(who), seed).unwrap();
                assert!(report.exit_tick.is_some(), "{tag}: reaches the exit");
                assert_eq!(report.deaths, 0, "{tag}");
                assert_eq!(report.capability_grants, 0, "{tag}");
                assert!(!report.passed_closed_gate, "{tag}");
                assert!(
                    report.max_stuck.iter().all(|&s| s <= STUCK_LIMIT),
                    "{tag}: stuck {:?}",
                    report.max_stuck
                );
                let (hash, combat_hash, _) = replay_corral(&root, &tag, report.ticks);
                assert_eq!(hash, report.party_hash, "{tag}");
                assert_eq!(combat_hash, report.hash, "{tag}");
            }
        }
    }

    /// CORRAL-F (the #280 review F4): `corral_rush` parks Carl inside B's trigger in the launched
    /// room, and B (never the M1 llama, never A) commits a rush at him.
    #[test]
    fn the_corral_rush_plan_parks_carl_and_b_rushes_him_in_the_room() {
        use crate::combat::CombatEvent;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // CORRAL-D: the first rush (tell 634, commit 666) ends on the intact partition between
        // them (tick 673), which breaks; the next one reaches Carl. Nobody is defeated by 800.
        let (_, _, combat) = replay_corral(&root, "corral_rush", 800);
        let commits: Vec<u32> = combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::LlamaRushCommit { llama, .. } => Some(llama),
                _ => None,
            })
            .collect();
        assert!(
            commits.contains(&1),
            "B rushes the parked Carl: {commits:?}"
        );
        assert!(!commits.contains(&0), "the M1 llama never rushes");
        assert!(
            !combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::HeroDefeated { .. })),
            "the rush lands on a live Carl"
        );
        assert!(!combat.llamas[0].alive(), "M1 was cleared first");
        assert!(combat.llamas[1].alive(), "the plan never fights B");
    }

    /// LLAMA-03 (B5), CORRAL-D: in the launched corral room the `corral_rush` plan's first rush draws, at the
    /// 52 px column Carl is parked on (112 px left of B, with the intact partition between them), the tick map's
    /// frames: the tell starts on 634 (`brace_in`), 642 is a scrape beat, the commit tick 666 draws the push, 668
    /// the reach, and the box meets the partition on 673: the bonk tick draws the LLAMA-01 hurt frame (bonk elapsed
    /// 0) while the partition breaks on that same tick, and 685 (elapsed 12) the blown stand. Before CORRAL-D the
    /// tell started on 742 (B walked up to the 112 px trigger) and the rush reached Carl and drew the skid on 791. It reads the finished state only.
    #[test]
    fn the_corral_rush_plan_draws_the_llama_rush_tick_map() {
        use crate::combat_anim::llama_rush_pose::{RushFrame, llama_rush_frame};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut drawn: std::collections::BTreeMap<u64, Option<RushFrame>> = Default::default();
        replay_corral_observed(&root, "corral_rush", 800, |tick, combat| {
            drawn.insert(tick, llama_rush_frame(&combat.llamas[1]));
        });
        for (tick, want) in [
            (634, RushFrame::Rush(0)),
            (642, RushFrame::Rush(2)),
            (666, RushFrame::Rush(6)),
            (668, RushFrame::Rush(7)),
            (673, RushFrame::Base(11)),
            (684, RushFrame::Base(11)),
            (685, RushFrame::Rush(9)),
        ] {
            assert_eq!(drawn[&tick], Some(want), "tick {tick}");
        }
        assert_eq!(
            drawn[&633], None,
            "before the tell the llama is not in a rush"
        );
    }

    /// CORRAL-D (CD-AD10, the #298 review F1): in the `corral_rush` run Carl dies twice with B in or
    /// past the screen's column (B at 1412 after the second rush) and respawns; the broken screen
    /// stays broken (no wall is rebuilt around B) until the run ends.
    #[test]
    fn a_checkpoint_respawn_keeps_the_broken_partition_in_the_corral_rush_run() {
        use crate::env_prop::PartitionState;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut after_break: Vec<PartitionState> = Vec::new();
        let (_, _, combat) = replay_corral_observed(&root, "corral_rush", 1800, |tick, c| {
            if tick >= 673 {
                after_break.push(c.env().unwrap().partition(0).unwrap().state());
            }
        });
        assert!(
            combat
                .events
                .iter()
                .any(|e| matches!(e, crate::combat::CombatEvent::Respawn { .. })),
            "the run respawns Carl"
        );
        assert!(
            after_break.iter().all(|s| *s == PartitionState::Broken),
            "the screen never stands again after the rush broke it"
        );
    }

    /// CORRAL-D: the `corral_partition` plan parses like the other corral plans and shows the
    /// partition in all three states in the launched room: it stands, Carl's first jab weakens it
    /// and the rush of B breaks it on 673, each state long enough for a capture at 1x to see it.
    #[test]
    fn the_corral_partition_plan_shows_the_screen_standing_weakened_and_broken() {
        use crate::env_prop::PartitionState;
        let plan = RoomPilot::parse("corral_partition").unwrap();
        assert_eq!(plan.plan, Plan::CorralPartition);
        assert_eq!(plan.label(), "corral_partition");
        assert_eq!(plan.tuning(), Some(corral_rush_tuning()));
        assert!(RoomPilot::parse("corral_partition:2p").is_err());
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut seen: Vec<(u64, PartitionState)> = Vec::new();
        let (_, _, combat) = replay_corral_observed(&root, "corral_partition", 900, |tick, c| {
            let state = c.env().unwrap().partition(0).unwrap().state();
            if seen.last().is_none_or(|&(_, s)| s != state) {
                seen.push((tick, state));
            }
        });
        // The first jab lands on 652 (weakened for 21 ticks), B's rush bonks the screen on 673.
        assert_eq!(
            seen,
            [
                (1, PartitionState::Intact),
                (652, PartitionState::Weakened),
                (673, PartitionState::Broken)
            ]
        );
        assert!(
            !combat
                .events
                .iter()
                .any(|e| matches!(e, crate::combat::CombatEvent::HeroDefeated { .. }))
        );
    }

    /// FA-Q24: the room 1 pilot parses, labels back and refuses what it cannot play; the plain
    /// `hoarder` plan is `hoarder:carl_solo:0`.
    #[test]
    fn the_hoarder_plan_parses_labels_and_refuses_what_it_cannot_play() {
        let plain = RoomPilot::parse("hoarder").unwrap();
        assert_eq!(plain.plan, Plan::Hoarder);
        assert_eq!(
            plain.aligned,
            Some(Aligned {
                who: AlignedWho::CarlSolo,
                seed: 0,
                route: None
            })
        );
        assert_eq!(plain.label(), "hoarder");
        assert_eq!(RoomPilot::parse(&plain.label()).unwrap(), plain);
        let seeded = RoomPilot::parse("hoarder:carl_solo:7").unwrap();
        assert_eq!(seeded.aligned.unwrap().seed, 7);
        assert_eq!(seeded.label(), "hoarder:carl_solo:7");
        assert_eq!(RoomPilot::parse(&seeded.label()).unwrap(), seeded);
        // Room 1 has one pilot: solo Carl.
        for bad in [
            "hoarder:donut_by_swap:1",
            "hoarder:companion:1",
            "hoarder:donut_fights:1",
            "hoarder:2p",
            "hoarder:carl_solo:x",
            "hoarder:carl_solo",
        ] {
            assert!(RoomPilot::parse(bad).is_err(), "{bad}");
        }
        // Its party is the library fixture's.
        assert_eq!(
            plain.tuning(),
            Some(crate::hoarder_fixtures::room_party_tuning())
        );
    }

    /// FA-Q24: the room pilot's run is the library fixture's for every seed: the same party hash
    /// on the exit tick, her defeat once, one `CapabilityGranted`, no hero defeat (nine of nine).
    #[test]
    fn the_hoarder_room_pilot_replays_the_library_run_and_defeats_her_nine_of_nine() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut defeated = 0;
        for seed in crate::hoarder_fixtures::ROOM_SEEDS {
            let report = crate::hoarder_fixtures::room_run(&root, seed).unwrap();
            let (hash, _) = replay_hoarder(&root, seed, report.ticks);
            assert_eq!(hash, report.hash, "seed {seed}");
            assert_eq!(report.grants, 1, "seed {seed}");
            assert_eq!(report.deaths, 0, "seed {seed}");
            assert!(report.stuck <= 300, "seed {seed}: stuck {}", report.stuck);
            assert!(report.defeat_tick < report.ticks, "seed {seed}");
            defeated += 1;
        }
        assert_eq!(defeated, 9);
    }

    /// One throw of Carl's dynamite from column `x` of pocket C with R2 latched and S2 at home
    /// (the sweep scene of FA-Q21: the scene starts at `x`, nothing else moves): the distance
    /// from the landed stick's blast centre to R2's dome, whether R2 was chained (a `Chain`
    /// provoke of sticker 1 on the stick's blast tick, R2's own blast 8 ticks later), and S2's
    /// hit points before and after R2's blast.
    fn r2_throw(x: i32) -> (f32, Option<(u64, u64)>, (u8, u8)) {
        use crate::f01_hazards;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = Loadout::Story.flags();
        let run = f01_hazards::launch(&root).unwrap();
        let mut scene = SceneSim::new_at(&run.kit, 360, x).unwrap();
        let mut combat = f01_hazards::combat_for(&run, &mut scene, carl, donut, None).unwrap();
        let slime_hp = |c: &CombatState| {
            c.slot_enemies()[1]
                .as_any()
                .and_then(|a| a.downcast_ref::<crate::combat_scene::slime::SlimeEnemy>())
                .map(|s| s.slime().health())
                .unwrap()
        };
        let mut s2 = (0, 0);
        for k in 0..230u64 {
            let before = slime_hp(&combat);
            combat.step_party(
                &mut scene,
                [
                    RawInput {
                        ability: k == 2,
                        ..RawInput::NONE
                    },
                    RawInput::NONE,
                ],
            );
            if combat
                .events
                .iter()
                .any(|e| matches!(e, CombatEvent::Blast { charge: 1, tick } if *tick == scene.world.tick))
            {
                s2 = (before, slime_hp(&combat));
            }
        }
        let stick = combat.charges[combat.dynamite_slot()];
        let (cx, cy) = stick.centre();
        let (tx, ty) = combat.charges[1].centre();
        let d = ((tx - cx).powi(2) + (ty - cy).powi(2)).sqrt();
        let blast = |charge: u32| {
            combat.events.iter().find_map(|e| match e {
                CombatEvent::Blast { tick, charge: c } if *c == charge => Some(*tick),
                _ => None,
            })
        };
        let chained = combat.events.iter().any(|e| {
            matches!(
                e,
                CombatEvent::RotProvoked {
                    sticker: 1,
                    cause: crate::rot_sticker::RotCause::Chain,
                    ..
                }
            )
        });
        let ticks = blast(1).zip(blast(combat.dynamite_slot() as u32));
        (d, chained.then(|| ticks.expect("both blasts")), s2)
    }

    /// FA-Q21 (T28, ruling `hazfu05_sticker_chain_reach_px = 76`): the 8 px sweep over the
    /// throw columns 640..=720. **Knob off** (reach 64): no column chains R2, and the nearest
    /// floor landing is 70.0..70.1 px from its dome (the finding that needed the knob). **Knob
    /// on** (reach 76): the lowest chaining column is 648 (landing 759, 70.007 px, a margin of
    /// 5.99 px), the chaining columns are the contiguous run 648..=672 (landing 759..783, within
    /// 76 px of the dome) and R2's blast is the stick's blast + 8. (S2 is not asserted here: in
    /// the route the stick's own blast defeats it, see the spectacular test.)
    #[test]
    fn haz_r2_chain_sweep_over_the_throw_columns() {
        let columns: Vec<i32> = (640..=720).step_by(8).collect();
        let off: Vec<_> = crate::slime::with_knobs_for_test(
            |t| t.hazfu05_sticker_chain_reach_px = 0,
            || columns.iter().map(|x| r2_throw(*x)).collect(),
        );
        for (x, (d, chain, _)) in columns.iter().zip(&off) {
            assert!(chain.is_none(), "knob off, column {x}: chained at {d}");
        }
        let nearest = off.iter().map(|o| o.0).fold(f32::MAX, f32::min);
        assert!((70.0..70.1).contains(&nearest), "{nearest}");
        let on: Vec<_> = columns.iter().map(|x| r2_throw(*x)).collect();
        let chaining: Vec<i32> = columns
            .iter()
            .zip(&on)
            .filter(|(_, o)| o.1.is_some())
            .map(|(x, _)| *x)
            .collect();
        eprintln!("R2_SWEEP chaining={chaining:?} {on:?}");
        assert_eq!(chaining.first(), Some(&648), "{chaining:?}");
        assert!(
            chaining.windows(2).all(|w| w[1] - w[0] == 8),
            "contiguous: {chaining:?}"
        );
        for (x, (d, chain, _)) in columns.iter().zip(&on) {
            assert_eq!(chain.is_some(), *d <= 76.0, "column {x}: distance {d}");
            if let Some((r2, stick)) = chain {
                assert_eq!(*r2, stick + 8, "column {x}: the chain is +8");
            }
        }
    }

    /// FA-Q21 (room 4, the spectacular route, onto R2): `haz_spectacular` clears pocket A, baits
    /// R1 and backs off out of its blast, then throws exactly one dynamite from column 648
    /// (the sweep's lowest chaining column): the stick lands 111 px ahead, within the 76 px
    /// reach (`hazfu05`) of R2, its blast lights R2 (a `Chain` provoke of sticker 1 credited to
    /// Carl on the stick's blast tick, R2's own blast 8 ticks later, CP-Q34), S2, which stands
    /// inside R2's circle and on the stick's landing, is defeated by the stick's own blast on
    /// that tick (credited to Carl; R2's 1 damage finds it already down), R1 hurts no hero (and
    /// neither does R2), and the room is cleared to its exit with no defeat. The run replays to
    /// the same hash, which is pinned.
    #[test]
    fn the_haz_spectacular_plan_chains_the_dynamite_into_r2_over_the_slime() {
        use crate::f01_late::hazards_start;
        use crate::rot_sticker::{ROT_SOURCE_ID, RotCause};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let pilot = RoomPilot::parse("haz_spectacular").unwrap();
        assert_eq!(pilot.label(), "haz_spectacular");
        assert!(pilot.aligned.is_none() && !pilot.two_player);
        let s2_hp = |c: &CombatState| {
            c.slot_enemies()[1]
                .as_any()
                .and_then(|a| a.downcast_ref::<crate::combat_scene::slime::SlimeEnemy>())
                .map(|s| (s.slime().health(), s.slime().x))
                .unwrap()
        };
        let play = || {
            let (carl, donut) = Loadout::Story.flags();
            let (run, mut scene, mut combat) =
                hazards_start(&root, 360, carl, donut, None).unwrap();
            let exit = run.loaded.encounter.exit;
            let exit_x = (exit.x + exit.w / 2) as f32;
            // S2's (hit points, x) the tick before and the tick after R2's blast.
            let mut s2_around = None;
            for _ in 0..2500 {
                if scene.world.complete {
                    break;
                }
                let before = s2_hp(&combat);
                let inputs = pilot.inputs(&combat, &scene, exit_x);
                combat.step_party(&mut scene, inputs);
                if s2_around.is_none()
                    && combat
                        .events
                        .iter()
                        .any(|e| matches!(e, CombatEvent::Blast { charge: 1, .. }))
                {
                    s2_around = Some((before, s2_hp(&combat)));
                }
            }
            (scene, combat, s2_around)
        };
        let (scene, combat, s2_around) = play();
        assert!(
            scene.world.complete,
            "the room is cleared and its exit reached"
        );
        assert_eq!(combat.deaths, 0);
        let throws = combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::ChargeThrown { .. }))
            .count();
        assert_eq!(throws, 1, "exactly one throw");
        let blast = |charge: u32| {
            combat.events.iter().find_map(|e| match e {
                CombatEvent::Blast { tick, charge: c } if *c == charge => Some(*tick),
                _ => None,
            })
        };
        let dynamite = blast(combat.dynamite_slot() as u32).expect("the dynamite goes off");
        let r2 = blast(1).expect("R2 goes off");
        assert_eq!(r2, dynamite + 8, "the chain is +8 ticks (CP-Q34)");
        assert!(
            combat.events.iter().any(|e| matches!(
                e,
                CombatEvent::RotProvoked {
                    tick,
                    sticker: 1,
                    cause: RotCause::Chain,
                    player: Some(0),
                    ..
                } if *tick == dynamite
            )),
            "R2 is lit by the chain on the stick's blast tick, credited to Carl"
        );
        // R1 was baited by proximity and went off on its own, before the throw.
        assert!(combat.events.iter().any(|e| matches!(
            e,
            CombatEvent::RotProvoked {
                sticker: 0,
                cause: RotCause::Proximity,
                ..
            }
        )));
        let r1 = blast(0).expect("R1 goes off");
        assert!(r1 < dynamite, "R1 first");
        let rot_hurts: Vec<u64> = combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::HeroHurt { tick, source, .. } if source & 0xffff == ROT_SOURCE_ID => {
                    Some(*tick)
                }
                _ => None,
            })
            .collect();
        assert!(
            !rot_hurts.iter().any(|t| *t == r1 || *t == r2),
            "no Rot blast hurts a hero: {rot_hurts:?}"
        );
        assert!(
            !combat.events.iter().any(|e| matches!(
                e,
                CombatEvent::HeroHurt { tick, .. } if (dynamite..=r2).contains(tick)
            )),
            "neither blast reaches Carl"
        );
        // S2 at home (it has crept at most to x 768.5 while Carl stood in view) lies inside R2's
        // 48 px blast circle (dome 760) *and* on the stick's landing (759): the stick's own
        // point-blank/ring blast defeats it on the dynamite's tick, credited to Carl, so R2's
        // 1 damage finds it already down. Ruling FA-Q21 asked for "S2 takes R2's blast damage":
        // that cannot be met, because every chaining landing is within 24 px of S2's home body.
        let ((hp_before, x_before), (hp_after, _)) = s2_around.expect("R2's blast");
        eprintln!("SPECT s2={s2_around:?} r1={r1} dyn={dynamite} r2={r2}");
        assert!(
            (x_before - 760.0).abs() < 48.0,
            "S2 stands inside R2's 48 px circle: {x_before}"
        );
        assert_eq!((hp_before, hp_after), (0, 0), "defeated before R2's blast");
        assert!(
            combat.events.iter().any(|e| matches!(
                e,
                CombatEvent::EnemyDefeated {
                    tick,
                    target,
                    cause: DefeatCause::Explosion { .. },
                    instigator: Some(0),
                    ..
                } if *tick == dynamite && *target == crate::combat_scene::slime::target_id(2)
            )),
            "S2 falls to the stick's blast, credited to Carl"
        );
        let (again, combat_again) = {
            let (s, c, _) = play();
            (s, c)
        };
        assert_eq!(
            combat.state_hash(&scene),
            combat_again.state_hash(&again),
            "the route replays to the same hash"
        );
        // The committed final hash as this test drives the room (slot 0 inputs only; the binary's
        // run, with the follower bot, ends on the scorecard's `80cfaca8118ce58e`): a drift is a
        // deliberate re-pin.
        assert_eq!(
            combat.state_hash(&scene),
            0x4da6_3d15_32a5_5302,
            "the committed final hash"
        );
    }

    /// PT-GATE-E (room 4, the tactical route, HAZ-07 pattern 2): `haz_tactical` clears
    /// pocket A, spends R1 by bait and retreat, then walks right until R2 is provoked by
    /// proximity (`RotProvoked { sticker: 1, Proximity }`), walks back left and holds while
    /// the 24-tick warning runs: R2's blast (`Blast { charge: 1 }`, provoke + 24) takes
    /// exactly 1 hit point off the slime S2 at home (full health the tick before, one less
    /// the tick after: Carl never jabbed it) and no Rot blast hurts Carl. The room is then
    /// cleared to its exit with no defeat. The final hash is pinned and replays twice.
    #[test]
    fn the_haz_tactical_plan_baits_r2_over_the_slime_and_the_blast_hits_s2() {
        use crate::f01_late::hazards_start;
        use crate::rot_sticker::{ROT_SOURCE_ID, RotCause};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let pilot = RoomPilot::parse("haz_tactical").unwrap();
        assert_eq!(pilot.label(), "haz_tactical");
        assert!(pilot.aligned.is_none() && !pilot.two_player);
        assert!(RoomPilot::parse("haz_tactical:2p").unwrap().two_player);
        let slime_hp = |combat: &CombatState, roster: usize| {
            combat.slot_enemies()[roster]
                .as_any()
                .and_then(|a| a.downcast_ref::<crate::combat_scene::slime::SlimeEnemy>())
                .map(|s| s.slime().health())
        };
        let play = || {
            let (carl, donut) = Loadout::Story.flags();
            let (run, mut scene, mut combat) =
                hazards_start(&root, 360, carl, donut, None).unwrap();
            let exit = run.loaded.encounter.exit;
            let exit_x = (exit.x + exit.w / 2) as f32;
            // S2's health the tick before and the tick after R2's blast.
            let mut s2_around_blast = None;
            for _ in 0..2500 {
                if scene.world.complete {
                    break;
                }
                let before = slime_hp(&combat, 1);
                let inputs = pilot.inputs(&combat, &scene, exit_x);
                combat.step_party(&mut scene, inputs);
                if combat
                    .events
                    .iter()
                    .any(|e| matches!(e, CombatEvent::Blast { charge: 1, .. }))
                    && s2_around_blast.is_none()
                {
                    s2_around_blast = Some((before, slime_hp(&combat, 1)));
                }
            }
            (scene, combat, s2_around_blast)
        };
        let (scene, combat, s2_around_blast) = play();
        assert!(
            scene.world.complete,
            "the room is cleared and its exit reached"
        );
        assert_eq!(combat.deaths, 0);
        let provoke = |sticker: u16| {
            combat.events.iter().find_map(|e| match e {
                CombatEvent::RotProvoked {
                    tick,
                    sticker: s,
                    cause: RotCause::Proximity,
                    ..
                } if *s == sticker => Some(*tick),
                _ => None,
            })
        };
        let blast = |charge: u32| {
            combat.events.iter().find_map(|e| match e {
                CombatEvent::Blast { tick, charge: c } if *c == charge => Some(*tick),
                _ => None,
            })
        };
        // R1 (pocket B) and R2 (pocket C) are both spent by proximity: bait and retreat.
        for (sticker, charge) in [(0u16, 0u32), (1, 1)] {
            let provoked = provoke(sticker).expect("provoked by proximity");
            assert_eq!(
                blast(charge),
                Some(provoked + 24),
                "the warning is 24 ticks"
            );
        }
        assert!(provoke(0) < provoke(1), "R1 first, then R2");
        assert!(
            !combat.events.iter().any(|e| matches!(
                e,
                CombatEvent::RotProvoked { cause, .. } if *cause != RotCause::Proximity
            )),
            "no dynamite, no bolt: only the bait"
        );
        // R2's blast hits S2 for exactly 1 (HAZ `haz_rot_hits_slime`: 1 damage), the
        // tick it goes off, with S2 untouched before it.
        assert_eq!(s2_around_blast, Some((Some(3), Some(2))));
        // No Rot blast hurts Carl (a hurt from the pool or the slime is reported, not asserted).
        assert!(
            !combat.events.iter().any(|e| matches!(
                e,
                CombatEvent::HeroHurt { source, .. } if source & 0xffff == ROT_SOURCE_ID
            )),
            "no Rot hurt"
        );
        // The committed final hash of this new fixture, as this test drives the room (slot 0
        // only; the binary's run, with the room scene's follower bot, ends on the scorecard's
        // hash instead): a drift is a deliberate re-pin.
        assert_eq!(
            combat.state_hash(&scene),
            0x383a_5307_0f66_546a,
            "the committed final hash"
        );
        let (again, combat_again, _) = play();
        assert_eq!(
            combat.state_hash(&scene),
            combat_again.state_hash(&again),
            "the route replays to the same hash"
        );
    }

    #[test]
    fn the_plain_mastery_plan_is_carl_solo_at_seed_zero() {
        let plain = RoomPilot::parse("mastery").unwrap();
        let named = RoomPilot::parse("mastery:carl_solo:0").unwrap();
        assert_eq!(plain, named);
        assert_eq!(plain.label(), "mastery");
        assert_eq!(
            plain.aligned,
            Some(Aligned {
                who: AlignedWho::CarlSolo,
                seed: 0,
                route: None
            })
        );
        // The plain Juicer plan stays the unaligned PT-GATE-A pilot; the rest of the
        // plans are unaligned too.
        for text in ["juicer", "direct", "valve", "chain:2p"] {
            assert_eq!(RoomPilot::parse(text).unwrap().aligned, None, "{text}");
        }
        // A two-player mastery plan is not the library's solo pilot (the `:2p` refusal stays).
        assert!(RoomPilot::parse("mastery:2p").unwrap().two_player);
        assert_eq!(RoomPilot::parse("mastery:2p").unwrap().aligned, None);
        for bad in [
            "mastery:carl_solo",
            "mastery:carl_solo:x",
            "mastery:nobody:1",
            "juicer:donut_fights:1",
            "mastery:carl_solo:1:2p",
            "direct:carl_solo:1",
            // PT-GATE-E: the hazards and Boiler library pilots are aligned only.
            "hazards",
            "boiler",
            "hazards:carl_solo",
            "hazards:donut_fights:1",
            "hazards:carl_solo:1:2p",
            "boiler:carl_solo:0",
            "boiler:carl_solo:weight:0",
            "boiler:carl_solo:valve:0",
            "boiler:donut_fights:direct:0",
            "boiler:carl_solo:both:x",
            "boiler:carl_solo:both:1:2p",
        ] {
            assert!(RoomPilot::parse(bad).is_err(), "{bad}");
        }
    }

    /// PT-GATE-E: `hazards:<who>:<seed>` and `boiler:<who>:<direct|both>:<seed>` parse,
    /// round-trip through the label and carry the library fixture's party.
    #[test]
    fn the_aligned_hazards_and_boiler_flags_parse_and_round_trip() {
        for (text, plan, who, seed, route) in [
            (
                "hazards:carl_solo:0",
                Plan::Hazards,
                AlignedWho::CarlSolo,
                0,
                None,
            ),
            (
                "hazards:donut_by_swap:8",
                Plan::Hazards,
                AlignedWho::DonutBySwap,
                8,
                None,
            ),
            (
                "hazards:companion:3",
                Plan::Hazards,
                AlignedWho::Companion,
                3,
                None,
            ),
            (
                "boiler:carl_solo:direct:0",
                Plan::Boiler,
                AlignedWho::CarlSolo,
                0,
                Some(BoilerRoute::Direct),
            ),
            (
                "boiler:donut_by_swap:both:5",
                Plan::Boiler,
                AlignedWho::DonutBySwap,
                5,
                Some(BoilerRoute::Both),
            ),
            (
                "boiler:companion:both:8",
                Plan::Boiler,
                AlignedWho::Companion,
                8,
                Some(BoilerRoute::Both),
            ),
        ] {
            let pilot = RoomPilot::parse(text).unwrap();
            assert_eq!(pilot.plan, plan, "{text}");
            assert!(!pilot.two_player, "{text}");
            assert_eq!(pilot.aligned, Some(Aligned { who, seed, route }), "{text}");
            assert_eq!(pilot.label(), text);
            assert_eq!(
                pilot.tuning(),
                Some(crate::f01_mastery_fixtures::tuning(who.mastery())),
                "{text}: the library fixture's party"
            );
        }
    }

    #[test]
    fn the_flag_parses_and_round_trips() {
        for (text, plan, two) in [
            ("direct", Plan::Direct, false),
            ("valve:2p", Plan::Valve, true),
            ("chain", Plan::Chain, false),
            ("mastery", Plan::Mastery, false),
            ("juicer", Plan::Juicer, false),
        ] {
            let pilot = RoomPilot::parse(text).unwrap();
            assert_eq!((pilot.plan, pilot.two_player), (plan, two));
            assert_eq!(pilot.label(), text);
        }
        assert!(RoomPilot::parse("sprint").is_err());
        assert!(RoomPilot::parse("direct:3p").is_err());
        assert!(RoomPilot::parse("").is_err());
    }

    /// DR-A: the `dozer:*` plans parse, print back and refuse a 2P suffix.
    #[test]
    fn the_dozer_plans_parse_and_print_back() {
        for (value, plan) in [
            ("dozer:run", Plan::DozerRun),
            ("dozer:close_call", Plan::DozerCloseCall),
            ("dozer:wait", Plan::DozerWait),
            ("dozer:brace", Plan::DozerBrace),
            ("dozer:retry", Plan::DozerRetry),
            ("dozer:crew", Plan::DozerCrew),
        ] {
            let pilot = RoomPilot::parse(value).unwrap();
            assert_eq!(pilot.plan, plan);
            // DRA-AD5: aligned, with the `cf02_free` partner of K5 `carl_solo`.
            assert!(!pilot.two_player && pilot.aligned == Some(DOZER_ALIGNED));
            assert_eq!(
                pilot.tuning().unwrap().uncontrolled_hero,
                crate::party::UncontrolledHero::Companion(
                    crate::companion::profile::ProfileSetId::Cf02Free
                )
            );
            assert!(plan.is_dozer());
            assert_eq!(pilot.label(), value);
        }
        for bad in ["dozer", "dozer:run:2p", "dozer:2p", "dozer:stall"] {
            assert!(RoomPilot::parse(bad).is_err(), "{bad}");
        }
        assert!(!Plan::Direct.is_dozer());
    }

    /// DR-A: each room 3 pilot escapes under the story loadout with nobody hit by the wheel,
    /// and the close call and the wait come within the yell range of the wheel. The pilots are
    /// stepped the way the scene steps an aligned pilot (DRA-AD5): through the party, with Donut
    /// the autonomous `cf02_free` partner, whose hazard feed sees the machine's rows.
    #[test]
    fn the_dozer_pilots_escape_room_3() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = Loadout::Story.flags();
        for plan in [Plan::DozerRun, Plan::DozerCloseCall, Plan::DozerWait] {
            let (run, mut scene, mut combat) =
                crate::f01_late::dozer_start(&root, 360, carl, donut, None).unwrap();
            let exit = run.loaded.encounter.exit;
            let exit_x = (exit.x + exit.w / 2) as f32;
            let pilot = RoomPilot::parse(plan.name()).unwrap();
            let mut party = crate::party::PartyState::new(pilot.tuning().unwrap());
            let mut memory = AlignedMemory::default();
            let mut closest = f32::MAX;
            for _ in 0..1500 {
                if scene.world.complete {
                    break;
                }
                let slots = pilot.aligned_inputs(
                    &combat,
                    &scene,
                    party.controlled_hero(),
                    exit_x,
                    &mut memory,
                );
                let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
                party.request_swap(slots[0].swap, hurt);
                let inputs = party.hero_inputs(slots);
                scene.set_drive(party.drive(true));
                combat.step_party(&mut scene, inputs);
                let d = crate::dozer_art::dozer_of(&combat).unwrap();
                if d.state.emerged() {
                    closest = closest.min(scene.world.hero.x - HERO_HALF_WIDTH - d.wheel_front());
                }
            }
            assert!(scene.world.complete, "{}: no exit", plan.name());
            let d = crate::dozer_art::dozer_of(&combat).unwrap();
            assert_eq!(d.stats.wheel_hits, 0, "{}", plan.name());
            if plan != Plan::DozerRun {
                assert!(
                    closest <= crate::dozer_art::YELL_RANGE_PX,
                    "{}: closest {closest}",
                    plan.name()
                );
            }
            // The machine's hazard-feed rows reach the partner's nav: the
            // graph rebuilds while it rolls (the perf note's rebuild ticks).
            let rebuilds = combat.companion_stats().hazard_feed_changes;
            assert!(rebuilds > 0, "{}: no nav rebuild", plan.name());
            println!(
                "DOZER_PILOT plan={} ticks={} closest_gap={closest:.1} state={} nav_rebuilds={rebuilds} carl_x={:.1} donut_x={:.1}",
                plan.name(),
                scene.world.tick,
                d.state.name(),
                scene.world.hero.x,
                scene.world.donut_body.x
            );
        }
    }
    /// DR-D (the #289 review finding 4 captures), stepped like an aligned pilot with the
    /// `cf02_free` partner: `dozer:retry` is run over once (one blow, DZ-Q04), respawns at the
    /// checkpoint, waits for the machine to leave the gate again and escapes; `dozer:crew`
    /// reverses the chase, puts both dismounted riders down and leaves with nobody run over.
    #[test]
    fn the_dozer_retry_and_crew_pilots_finish_room_3() {
        use crate::combat::CombatEvent;
        use crate::dozer::{DozerEvent, DozerState};
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = Loadout::Story.flags();
        for plan in [Plan::DozerRetry, Plan::DozerCrew] {
            let (run, mut scene, mut combat) =
                crate::f01_late::dozer_start(&root, 360, carl, donut, None).unwrap();
            let exit = run.loaded.encounter.exit;
            let exit_x = (exit.x + exit.w / 2) as f32;
            let pilot = RoomPilot::parse(plan.name()).unwrap();
            let mut party = crate::party::PartyState::new(pilot.tuning().unwrap());
            let mut memory = AlignedMemory::default();
            for _ in 0..2000 {
                if scene.world.complete {
                    break;
                }
                let slots = pilot.aligned_inputs(
                    &combat,
                    &scene,
                    party.controlled_hero(),
                    exit_x,
                    &mut memory,
                );
                let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
                party.request_swap(slots[0].swap, hurt);
                let inputs = party.hero_inputs(slots);
                scene.set_drive(party.drive(true));
                combat.step_party(&mut scene, inputs);
            }
            assert!(scene.world.complete, "{}: no exit", plan.name());
            // The whole run is in the log (the ring never dropped an event).
            assert!(combat.events.len() < crate::combat_scene::EVENT_LOG_CAP);
            let emerged = combat
                .events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        CombatEvent::Dozer {
                            event: DozerEvent::Emerged { .. },
                            ..
                        }
                    )
                })
                .count();
            let defeated = combat
                .events
                .iter()
                .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
                .count();
            let d = crate::dozer_art::dozer_of(&combat).unwrap();
            // HUD-TRY-01 (HT-AD1): the checkpoint respawn is the HUD's second try.
            let attempt = if plan == Plan::DozerRetry { 2 } else { 1 };
            assert_eq!(combat.hud_try(), attempt, "{}: TRY", plan.name());
            if plan == Plan::DozerRetry {
                assert_eq!(combat.deaths, 1);
                // The machine's stats restart with the respawn; the log keeps the blow.
                let blows = combat
                    .events
                    .iter()
                    .filter(|e| {
                        matches!(
                            e,
                            CombatEvent::Dozer {
                                event: DozerEvent::WheelHit { .. },
                                ..
                            }
                        )
                    })
                    .count();
                assert_eq!((blows, d.stats.wheel_hits), (1, 0));
                assert_eq!(
                    emerged, 2,
                    "the machine leaves the gate again after the respawn"
                );
            } else {
                assert_eq!(combat.deaths, 0);
                assert_eq!(d.stats.wheel_hits, 0);
                assert_eq!(d.state, DozerState::Disabled);
                assert_eq!(defeated, 2, "both riders down");
            }
            println!(
                "DOZER_PILOT plan={} ticks={} deaths={} emerged={emerged} riders_down={defeated} state={}",
                plan.name(),
                scene.world.tick,
                combat.deaths,
                d.state.name()
            );
        }
    }
    /// DR-B (the reversal capture): `dozer:brace` in the live room, stepped like an aligned
    /// pilot with the `cf02_free` partner: Carl's two jabs release the frame on the aimed R
    /// (the middle of the helper's window), the landing stuns the machine once, credited to
    /// him, it ends disabled, nobody is hurt or run over, and the room completes.
    #[test]
    fn the_dozer_brace_pilot_reverses_the_chase() {
        use crate::combat::CombatEvent;
        use crate::dozer::DozerEvent;
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (carl, donut) = Loadout::Story.flags();
        let (run, mut scene, mut combat) =
            crate::f01_late::dozer_start(&root, 360, carl, donut, None).unwrap();
        let exit = run.loaded.encounter.exit;
        let exit_x = (exit.x + exit.w / 2) as f32;
        let pilot = RoomPilot::parse("dozer:brace").unwrap();
        let mut party = crate::party::PartyState::new(pilot.tuning().unwrap());
        let mut memory = AlignedMemory::default();
        let mut hurts = 0;
        let mut disabled_at = None;
        // #287 review F1: the remnant stays out of the partner's nav until the
        // frame lands, and is in it from the landing on.
        let mut hidden_ticks = 0u64;
        let mut landed_at = None;
        let mut nav_tops = Vec::new();
        for _ in 0..1500 {
            if scene.world.complete {
                break;
            }
            let hides = combat.env().unwrap().nav_hides_any();
            let stunned = combat.events.iter().any(|e| {
                matches!(
                    e,
                    CombatEvent::Dozer {
                        event: DozerEvent::Stunned { .. },
                        ..
                    }
                )
            });
            if scene.world.tick > 0 {
                assert_eq!(hides, !stunned, "tick {}", scene.world.tick);
            }
            if hides {
                hidden_ticks += 1;
            } else if landed_at.is_none() && scene.world.tick > 0 {
                landed_at = Some(scene.world.tick);
            }
            let slots = pilot.aligned_inputs(
                &combat,
                &scene,
                party.controlled_hero(),
                exit_x,
                &mut memory,
            );
            let hurt = combat.hurt_ticks_of(party.controlled_hero()) > 0;
            party.request_swap(slots[0].swap, hurt);
            let inputs = party.hero_inputs(slots);
            scene.set_drive(party.drive(true));
            combat.step_party(&mut scene, inputs);
            // The open anchors of the partner's nav over the remnant's middle
            // (x 680) after the step: their standing heights.
            let graph = combat.companion_nav_graph().expect("the partner's nav");
            let tops: Vec<f32> = graph
                .anchors()
                .filter(|a| graph.anchor_open(*a))
                .filter(|a| {
                    graph
                        .anchor_span(*a)
                        .is_some_and(|(l, r)| l <= 680.0 && r >= 680.0)
                })
                .filter_map(|a| graph.anchor_pos(a).map(|(_, y)| y))
                .collect();
            nav_tops.push((scene.world.tick, hides, tops));
            for hero in [HeroId::Carl, HeroId::Donut] {
                if combat.hurt_ticks_of(hero) > 0 {
                    hurts += 1;
                }
            }
            let d = crate::dozer_art::dozer_of(&combat).unwrap();
            if d.state == crate::dozer::DozerState::Disabled && disabled_at.is_none() {
                disabled_at = Some(scene.world.tick);
            }
        }
        let (chase0, aimed) = memory.dozer_chase.expect("the chase was seen");
        let stuns: Vec<(u64, Option<u32>)> = combat
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::Dozer {
                    tick,
                    event: DozerEvent::Stunned { by, .. },
                } => Some((*tick, *by)),
                _ => None,
            })
            .collect();
        let d = crate::dozer_art::dozer_of(&combat).unwrap();
        println!(
            "DOZER_BRACE ticks={} chase0={chase0} aimed_r={aimed} stuns={stuns:?} disabled_at={disabled_at:?} state={}",
            scene.world.tick,
            d.state.name()
        );
        assert_eq!(stuns.len(), 1, "{stuns:?}");
        assert!(
            hidden_ticks > 300,
            "the remnant was hidden for {hidden_ticks} ticks"
        );
        assert_eq!(
            landed_at,
            Some(stuns[0].0),
            "the remnant joins the nav on the landing tick"
        );
        assert_eq!(
            stuns[0],
            (
                chase0 + aimed + crate::dozer_fixtures::frame::LAND_DELAY,
                Some(crate::shore::PLAYER_ONE)
            )
        );
        // #287 re-review: the nav graph itself, not only the env flag. The
        // remnant's top (the frame fixtures' rule: an intact solid over 640..720
        // resting on the walking line).
        let floor = d.floor_y;
        let remnant = scene
            .world
            .solids
            .iter()
            .filter(|b| b.intact && b.rect.left < 720.0 && b.rect.right() > 640.0)
            .filter(|b| b.rect.bottom <= floor + 0.5)
            .map(|b| b.rect.top())
            .fold(f32::MIN, f32::max);
        assert!(remnant > floor, "no remnant in the lane");
        for (tick, hides, tops) in &nav_tops {
            let on_remnant = tops.iter().any(|t| (t - remnant).abs() <= 0.5);
            assert_eq!(
                on_remnant, !hides,
                "tick {tick}: hidden {hides}, the nav's anchors over x 680 {tops:?}, the remnant top {remnant}"
            );
        }
        assert!(disabled_at.is_some(), "never disabled");
        assert_eq!(d.stats.wheel_hits, 0);
        assert_eq!(hurts, 0, "hurt ticks");
        assert!(scene.world.complete, "no exit");
    }
}
