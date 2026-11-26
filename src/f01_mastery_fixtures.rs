//! PT-MASTERY: scripted pilot fixtures of the closing arena (`f01_mastery`).
//!
//! Bevy-free and library-only (not in the `--party-fixture` registries: no
//! committed replay file, no CI loop; the scene's own headless smoke and the
//! PLAYTRACE tests cover the launched room). Each run is one launch of the room
//! built the way the scene builds it ([`crate::f01_late::mastery_start`], the
//! story loadout: the room's pin holds every capability), driven by a pilot
//! that plays the fight and walks the cleared room to the exit:
//!
//! | who | what plays |
//! | --- | --- |
//! | [`Who::CarlSolo`] | solo Carl (the live party's `cf02_assist` partner follows) |
//! | [`Who::DonutBySwap`] | Carl clears the room, then the swap, then Donut (solo) walks to the exit |
//! | [`Who::DonutFights`] | the swap on [`SWAP_TICK`], then Donut (solo) fights the whole room and walks |
//! | [`Who::Companion`] | Carl is the pilot's, Donut is the autonomous partner (`--companion cf02_free`) |
//! | [`Who::CarlKit`] | D7, report-only: solo Carl who also presses the held dynamite (`ability`) |
//! | [`Who::DonutKit`] | D7, report-only: the swapped solo Donut who also presses the held blink (`ability`) |
//!
//! The four base pilots press the base kit only (jab, haymaker, kick, Donut's
//! swat and pounce); the two kit pilots add the `ability` press edge the
//! progression fixtures use (`carl_prog_fixtures::special`), nothing else. Since
//! PT-TUNE (PTT-Q13) every pilot also plays the room's hazards: it baits the
//! slime's lunge and punishes it in the 68-tick window, never stands in a live
//! acid patch, and keeps out of an awake Rot Sticker's trigger column while a
//! wave enemy is left. [`Report::difficulty`] is the headless difficulty measure
//! of `docs/03-tech/pt-tune-design.md` section 3 (D1 to D5). A hero is
//! **stuck** when it is told to walk and its column does not change; the
//! longest such run must stay at most [`STUCK_LIMIT`] ticks (plan section 10,
//! item 5). The companion's own walking is not the pilot's, so its stuck run is
//! counted as consecutive ticks **left behind**: standing still more than
//! [`APART_PX`] from Carl while he walks on.

use std::path::Path;

use crate::boss_scene::Pilot;
use crate::combat::{CombatEvent, EnemyAi};
use crate::combat_scene::CombatState;
use crate::companion::profile::ProfileSetId;
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::f01_late::{Loadout, mastery_start};
use crate::input::RawInput;
use crate::party::{HeroId, PROVISIONAL_TUNING, PartyTuning, UncontrolledHero};
use crate::party_scene::{PartyFixture, PartyFixtureSim};
use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Rect};

/// A walking hero that does not move for this many ticks is stuck: the gate
/// allows at most 300 (plan section 10, item 5).
pub const STUCK_LIMIT: u32 = 300;
/// FA-Q13: the stall measure reads hurts to the controlled hero this far back.
pub const STALL_HURT_WINDOW: u64 = 300;
/// FA-Q13: the most a room 6 pilot may stall (asserted for the pilots that
/// clear the room).
pub const STALL_LIMIT: u32 = 120;
/// The seeds every fixture covers (`--cf02-seed`, plan section 10, item 5).
pub const SEEDS: std::ops::Range<u64> = 0..9;
/// The run length of every pilot (100 s of game time).
pub const RUN_TICKS: u64 = 6000;
/// The tick on which [`Who::DonutBySwap`] swaps to Donut (PTM-AD8).
pub const SWAP_TICK: u64 = 90;
/// The companion is counted left behind only while this far from Carl.
pub const APART_PX: f32 = 96.0;

/// FIX-TOOL (PT-FU-B8 item 10): the companion's "left behind" measure, applied to **the
/// hero the pilot does not steer** in every mode (the `cf02_assist` partner of the solo
/// runs included, which the walking-input stuck rule cannot see, since no input is held
/// for it). A tick counts while the partner stands still (its column changed by less than
/// 0.1 px), is more than [`APART_PX`] from the steered hero, and the steered hero moved
/// that tick; the longest run of consecutive counted ticks is [`Self::max`]. A measure,
/// not a rule: it never changes a run, and the partner's AI is not tuned from it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LeftBehind {
    run: u32,
    max: u32,
}

impl LeftBehind {
    /// One tick: whether the partner stood still, its distance from the steered hero and
    /// whether the steered hero moved.
    pub fn observe(&mut self, partner_still: bool, apart: f32, steered_moved: bool) {
        if partner_still && apart > APART_PX && steered_moved {
            self.run += 1;
            self.max = self.max.max(self.run);
        } else {
            self.run = 0;
        }
    }

    /// The longest run of counted ticks.
    pub fn max(&self) -> u32 {
        self.max
    }
}
/// The column a hero walks to once the roster is clear: inside the exit gate
/// (1072..1136), and the column that counts as reaching it.
pub const EXIT_GOAL_X: f32 = 1110.0;
pub const EXIT_X: f32 = 1090.0;
/// A jab and a kick reach (the arena autopilot's values).
const JAB_REACH: f32 = 30.0;
const KICK_REACH: f32 = 36.0;
/// Ticks between two presses of the pilot.
const PRESS_CADENCE: u64 = 4;

/// Who plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    CarlSolo,
    DonutBySwap,
    DonutFights,
    Companion,
    /// D7: solo Carl with the dynamite (report-only, [`Who::KITS`]).
    CarlKit,
    /// D7: the swapped solo Donut with the blink (report-only, [`Who::KITS`]).
    DonutKit,
}

impl Who {
    /// The four base-kit pilots, pinned by `GOLDEN`.
    pub const ALL: [Who; 4] = [
        Who::CarlSolo,
        Who::DonutBySwap,
        Who::DonutFights,
        Who::Companion,
    ];
    /// The two D7 kit pilots, pinned by `KIT_GOLDEN`.
    pub const KITS: [Who; 2] = [Who::CarlKit, Who::DonutKit];

    pub const fn name(self) -> &'static str {
        match self {
            Self::CarlSolo => "carl_solo",
            Self::DonutBySwap => "donut_by_swap",
            Self::DonutFights => "donut_fights",
            Self::Companion => "companion_cf02_free",
            Self::CarlKit => "carl_kit",
            Self::DonutKit => "donut_kit",
        }
    }
}

/// The party of a run: the live room's (`cf02_assist` partner and the 12-tick
/// swap), or the autonomous partner under `--companion cf02_free`.
pub fn tuning(who: Who) -> PartyTuning {
    let profile = match who {
        Who::Companion => ProfileSetId::Cf02Free,
        _ => ProfileSetId::Cf02Assist,
    };
    PartyTuning {
        uncontrolled_hero: UncontrolledHero::Companion(profile),
        ..PROVISIONAL_TUNING
    }
    .with_cf02_swap()
}

/// Whether a solid stands within 14 px ahead of a body at ledge height (the
/// arena autopilot's probe).
fn blocked_ahead(scene: &SceneSim, x: f32, y: f32, dir: f32) -> bool {
    let probe = Rect {
        left: x - HERO_HALF_WIDTH + dir * 14.0,
        bottom: y - HERO_HALF_HEIGHT + 4.0,
        width: HERO_HALF_WIDTH * 2.0,
        height: 8.0,
    };
    crate::combat::solid_at(&scene.world.solids, probe)
}

/// Whether a solid stands between `x` and `target_x` at the height Donut's bolt
/// flies (19 px above the feet, the bolt box 6 px tall): a cast would end on it.
fn bolt_blocked(scene: &SceneSim, x: f32, y: f32, target_x: f32) -> bool {
    let probe = Rect {
        left: x.min(target_x),
        bottom: y - HERO_HALF_HEIGHT + 16.0,
        width: (target_x - x).abs(),
        height: 6.0,
    };
    crate::combat::solid_at(&scene.world.solids, probe)
}

/// The slime's bait range (PTT-Q13): the pilot stands this far from an idle
/// slime so that the creeping slime enters its trigger distance (44 px) and
/// starts its tell, while a lunge (reach 39 px plus the hero's half width) falls
/// short.
const SLIME_BAIT_MIN: f32 = 40.0;
const SLIME_BAIT_MAX: f32 = 70.0;
/// The distance the pilot keeps from a slime that is telling, launching or in
/// its impact: outside the lunge box.
const SLIME_SAFE_PX: f32 = 64.0;
/// The distance a punishing jab is thrown from (the slime's hurtbox edge is
/// 28 to 32 px from its pivot; the corrosive contact box reaches 23 px).
const SLIME_PUNISH_PX: f32 = 46.0;
/// A slime this near is the pilot's business before any goblin.
const SLIME_ENGAGE_PX: f32 = 200.0;
/// An awake Rot Sticker's trigger column (its trigger is 32 px) plus the hero's
/// half width and a margin: the pilot does not stand in it while a wave
/// enemy is left.
const ROT_AVOID_PX: f32 = 56.0;
/// A hero centre this far above a goblin's centre (plus the 7 px the two
/// centres differ on the floor) stands on a ledge.
const ELEVATED_PX: f32 = 20.0;
const FLOOR_CENTRE_DELTA: f32 = 7.0;
/// A held ability is pressed at most once in this many ticks (D7).
const KIT_CADENCE: u64 = 150;

/// FA-Q12: Donut's cast stand-off. She casts only at least this far from her
/// target (the bolt spawns 27 px ahead); inside it she steps back when the step
/// back is legal.
const DONUT_STANDOFF_PX: f32 = 40.0;
/// FA-Q12: a target this near, or a dry pool, is the pounce's (Physical).
const DONUT_POINT_BLANK_PX: f32 = 32.0;
/// FA-Q12 detail adopted here: with no mana she pounces only inside this reach
/// (the pounce box ends 25 px ahead and she leaps forward), else she walks on.
const DONUT_POUNCE_REACH_PX: f32 = 60.0;
/// FA-Q12: a live wave entry beyond this is not in cast reach (the bolt flies
/// 240 px); she walks toward the nearest one.
const DONUT_CAST_REACH_PX: f32 = 160.0;
/// FA-Q12 (kit pilot only): a gap above this to the nearest wave entry is
/// closed with the blink.
const DONUT_BLINK_GAP_PX: f32 = 160.0;
/// FA-Q12 (kit pilot only): this many hurts within [`DONUT_ESCAPE_WINDOW`] ticks
/// is an escape blink.
const DONUT_ESCAPE_HURTS: usize = 2;
const DONUT_ESCAPE_WINDOW: u64 = 120;
/// The blink's mana cost (DP-Q23), pressed only with this many charges.
const BLINK_COST: u8 = 2;
/// After a back step toward away facing, she keeps going until this far (the
/// stand-off plus a turn tick's step), so the turn does not undo it.
const DONUT_TURN_MARGIN_PX: f32 = 8.0;

/// The pilot's rules, each behaviour change behind a knob (the fix-all plan's
/// flag-gating rule: `EARLIER` reproduces the hashes recorded before it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PilotRules {
    /// FA-Q12: the Donut rules (stand-off, pounce at point-blank or dry,
    /// approach, the kit pilot's blink). Carl's branch never reads it.
    pub donut_rules: bool,
    /// FA-Q17: the jab phase is offset by `seed % 4` ticks and by one more tick
    /// at each new engagement (one per roster entry already defeated), as the
    /// Juicer pilot offsets its phase. (The first form tried, `seed % 4` times
    /// the engagement count, gave 4 distinct outcomes at seeds 0..8; this
    /// additive form gives 7. FA-Q30 iteration 2 of 2.)
    pub jab_phase_by_seed: bool,
}

impl PilotRules {
    /// The closing arena (room 6): the Donut rules and the seed-offset jab phase.
    pub const SHIPPED: Self = Self {
        donut_rules: true,
        jab_phase_by_seed: true,
    };
    /// Every other room that plays the PT-TUNE pilot (room 4's Carl and Donut,
    /// the room pilots of the hazards plans): the Donut rules, and **no** jab
    /// phase. FA-Q17 scopes the seed phase to "the arena pilots" (room 6).
    pub const OTHER_ROOMS: Self = Self {
        donut_rules: true,
        jab_phase_by_seed: false,
    };
    /// The pilot before FA-Q12 and FA-Q17.
    pub const EARLIER: Self = Self {
        donut_rules: false,
        jab_phase_by_seed: false,
    };
}

/// What the room's hazards look like to the pilot this tick.
struct Hazards {
    /// The live acid patches' current bounds (a draining pool still counts).
    acid: Vec<(f32, f32)>,
    /// The armed Rot Sticker's dome column (latched, scuttling or warning).
    rot: Option<f32>,
    /// The living slime: its pivot, and its attack tick when it is attacking.
    slime: Option<(f32, Option<u16>)>,
}

fn hazards(combat: &CombatState) -> Hazards {
    let mut acid = Vec::new();
    let mut rot = None;
    if let Some(env) = combat.env() {
        for patch in env.acid().patches() {
            if let Some((x0, x1)) = patch.bounds() {
                acid.push((x0 as f32, x1 as f32));
            }
        }
        let t = crate::rot_sticker::active();
        for (_, sticker, _) in env.stickers() {
            if !sticker.is_spent() {
                rot = Some(sticker.dome(t).0);
            }
        }
    }
    let slime = combat
        .slot_enemies()
        .iter()
        .filter(|e| e.kind() == crate::combat_scene::slime::KIND && e.alive())
        .find_map(|e| {
            e.as_any()
                .and_then(|a| a.downcast_ref::<crate::combat_scene::slime::SlimeEnemy>())
        })
        .map(|e| (e.slime().x, e.slime().attack_tick()));
    Hazards { acid, rot, slime }
}

/// The pilot: a pure function of the finished state, for hero `hero`. It baits
/// and punishes the slime, fights the nearest awake goblin or Engineer with jabs
/// and haymakers, kicks the nearest llama, walks on to the nearest enemy that is
/// left, and walks to the exit once the roster is clear. It jumps what stands in
/// its way, never walks into a live acid patch and keeps out of an armed Rot
/// Sticker's column while a wave enemy is left. It never reads the camera.
pub fn pilot(combat: &CombatState, scene: &SceneSim, hero: HeroId) -> RawInput {
    pilot_with(combat, scene, hero, false)
}

/// [`pilot`] as the closing arena plays it ([`PilotRules::SHIPPED`]: with the FA-Q17
/// seed-offset jab phase). Room 6's runs, metrics and room pilot use this one.
pub fn arena_pilot(combat: &CombatState, scene: &SceneSim, hero: HeroId) -> RawInput {
    pilot_rules(combat, scene, hero, false, PilotRules::SHIPPED)
}

/// [`pilot`], optionally pressing the held ability (D7: Carl's dynamite or
/// Donut's blink) on the `ability` press edge, the input the progression
/// fixtures use.
pub fn pilot_with(combat: &CombatState, scene: &SceneSim, hero: HeroId, kit: bool) -> RawInput {
    pilot_rules(combat, scene, hero, kit, PilotRules::OTHER_ROOMS)
}

/// [`pilot_with`] under explicit [`PilotRules`].
pub fn pilot_rules(
    combat: &CombatState,
    scene: &SceneSim,
    hero: HeroId,
    kit: bool,
    rules: PilotRules,
) -> RawInput {
    pilot_phased(combat, scene, hero, kit, rules, 0)
}

/// [`pilot_rules`] with `extra_phase` ticks added to the press phase (0 is
/// [`pilot_rules`] byte for byte). FA-Q14 follow-up: the hazards room's solo-Donut
/// fighter offsets her press (her cast and her pounce) by `seed % 4` ticks, so its
/// nine seeds are not one fight, without turning on the arena's engagement-additive
/// jab phase for the other pilots of that room.
pub fn pilot_phased(
    combat: &CombatState,
    scene: &SceneSim,
    hero: HeroId,
    kit: bool,
    rules: PilotRules,
    extra_phase: u64,
) -> RawInput {
    let body = scene.world.body(hero);
    let tick = scene.world.tick + 1;
    let defeated = combat.enemies.iter().filter(|g| !g.alive()).count()
        + combat.llamas.iter().filter(|l| !l.alive()).count();
    // FA-Q17: the jab phase of this engagement (one per entry already down).
    let phase = if rules.jab_phase_by_seed {
        (combat.cf02_seed().unwrap_or(0) % PRESS_CADENCE + defeated as u64) % PRESS_CADENCE
    } else {
        0
    };
    let phase = (phase + extra_phase) % PRESS_CADENCE;
    let press = (tick + phase).is_multiple_of(PRESS_CADENCE);
    let mut raw = RawInput::default();
    if combat.hero_defeated {
        return raw;
    }
    let donut = hero == HeroId::Donut && rules.donut_rules;
    let hz = hazards(combat);
    let enemies_left = combat.enemies.iter().any(|g| g.alive())
        || combat.llamas.iter().any(|l| l.alive())
        || hz.slime.is_some();
    let towards = |x: f32| {
        let dx = x - body.x;
        if dx.abs() < 4.0 { 0.0 } else { dx.signum() }
    };
    // A step is refused when it would put the hero into a live acid patch (or
    // into an armed Rot Sticker's column while a wave enemy is left), unless it
    // is already inside (it then walks on).
    let refused = |dir: f32| {
        let next = body.x + dir * 2.0;
        let inside = |x: f32, a: f32, b: f32| x > a - HERO_HALF_WIDTH && x < b + HERO_HALF_WIDTH;
        let acid = hz
            .acid
            .iter()
            .any(|(a, b)| inside(next, *a, *b) && !inside(body.x, *a, *b));
        let rot = enemies_left
            && hz.rot.is_some_and(|x| {
                (next - x).abs() < ROT_AVOID_PX && (body.x - x).abs() >= ROT_AVOID_PX
            });
        acid || rot
    };
    let walk = |raw: &mut RawInput, dir: f32| {
        if dir != 0.0 && refused(dir) {
            raw.move_x = 0.0;
            return;
        }
        raw.move_x = dir;
        if body.grounded && dir != 0.0 && blocked_ahead(scene, body.x, body.y, dir) {
            raw.jump = true;
        }
    };
    let ability = |raw: &mut RawInput, ready: bool| {
        if kit && ready && tick.is_multiple_of(KIT_CADENCE) {
            raw.ability = true;
        }
    };
    // A llama in kick reach is kicked first, whatever else is awake: it stands
    // in the lane and blocks the walk. (Donut's point-blank answer is the
    // pounce of her own rules below.)
    if !donut
        && let Some(l) = combat
            .llamas
            .iter()
            .find(|l| l.alive() && (l.x - body.x).abs() <= KICK_REACH)
    {
        walk(&mut raw, (l.x - body.x).signum());
        if press {
            raw.physical = true;
        }
        return raw;
    }
    // PTT-Q13: the slime. Bait its lunge, step back on the tell, punish it in
    // the recover and the cooldown (68 ticks) with jabs.
    if let Some((sx, attack)) = hz.slime
        && (sx - body.x).abs() < SLIME_ENGAGE_PX
    {
        let d = sx - body.x;
        let away = -d.signum();
        let t = crate::slime::active();
        match attack {
            Some(a) if a < t.recover_start() => {
                if d.abs() < SLIME_SAFE_PX {
                    walk(&mut raw, away);
                }
            }
            Some(_) => {
                if d.abs() > SLIME_PUNISH_PX {
                    walk(&mut raw, d.signum());
                } else if press {
                    raw.attack = true;
                }
            }
            None => {
                if d.abs() > SLIME_BAIT_MAX {
                    walk(&mut raw, d.signum());
                } else if d.abs() < SLIME_BAIT_MIN {
                    walk(&mut raw, away);
                }
            }
        }
        return raw;
    }
    if donut && let Some(input) = donut_rules(combat, scene, kit, tick, press, &walk, &refused) {
        return input;
    }
    let threat = combat
        .enemies
        .iter()
        .filter(|g| g.alive() && g.ai == EnemyAi::Chase)
        .min_by(|a, b| (a.x - body.x).abs().total_cmp(&(b.x - body.x).abs()));
    if let Some(g) = threat
        && (g.x - body.x).abs() < 220.0
    {
        let dx = g.x - body.x;
        // On a ledge (the parapet) above the enemy: hop, then dive onto it (Down
        // plus Physical in the air, the base move of CF02-Q01).
        if body.y - (g.y + FLOOR_CENTRE_DELTA) > ELEVATED_PX && dx.abs() < 30.0 {
            raw.move_x = towards(g.x);
            if body.grounded {
                raw.jump = true;
            } else {
                raw.down = true;
                raw.physical = press;
            }
            return raw;
        }
        walk(&mut raw, dx.signum());
        if dx.abs() <= JAB_REACH && press {
            raw.attack = true;
        }
        ability(&mut raw, dx.abs() > 60.0);
        return raw;
    }
    let llama = combat
        .llamas
        .iter()
        .filter(|l| l.alive())
        .min_by(|a, b| (a.x - body.x).abs().total_cmp(&(b.x - body.x).abs()));
    if let Some(l) = llama
        && (l.x - body.x).abs() < 320.0
    {
        let dx = l.x - body.x;
        walk(&mut raw, dx.signum());
        if dx.abs() <= KICK_REACH && press {
            raw.physical = true;
        }
        ability(&mut raw, dx.abs() > 60.0);
        return raw;
    }
    // Nothing awake: on to the nearest enemy that is left, else to the exit.
    let goal = combat
        .enemies
        .iter()
        .filter(|g| g.alive())
        .map(|g| g.x)
        .chain(combat.llamas.iter().filter(|l| l.alive()).map(|l| l.x))
        .chain(hz.slime.map(|(x, _)| x))
        .min_by(f32::total_cmp)
        .map_or(EXIT_GOAL_X, |x| x - 150.0);
    walk(&mut raw, towards(goal));
    raw
}

/// FA-Q12: Donut's rules in the shared pilot, for the hero Donut only. `None`
/// when no wave entry lives (the pilot walks on to the exit). The pilot holds
/// the stand-off, answers point-blank or a dry pool with the pounce, walks to
/// the nearest wave entry that is out of cast reach (the llama at 616), and (the
/// kit pilot) blinks to close a long gap or to escape after two hurts.
fn donut_rules(
    combat: &CombatState,
    scene: &SceneSim,
    kit: bool,
    tick: u64,
    press: bool,
    walk: &dyn Fn(&mut RawInput, f32),
    refused: &dyn Fn(f32) -> bool,
) -> Option<RawInput> {
    let body = scene.world.body(HeroId::Donut);
    let mut raw = RawInput::default();
    let target = combat
        .enemies
        .iter()
        .filter(|g| g.alive())
        .map(|g| g.x)
        .chain(combat.llamas.iter().filter(|l| l.alive()).map(|l| l.x))
        .min_by(|a, b| (a - body.x).abs().total_cmp(&(b - body.x).abs()))?;
    let blink = combat.donut_blink();
    // A blink in its windup is cancelled by any press: stand still through it.
    if blink.windup().is_some() {
        return Some(raw);
    }
    let d = target - body.x;
    let dist = d.abs();
    let toward = d.signum();
    // In the air (a hop over the parapet, a pounce): steer on, press nothing.
    if !body.grounded {
        walk(&mut raw, toward);
        return Some(raw);
    }
    let mana = combat.donut_mana_view().map_or(0, |(m, _)| m);
    // A hurt, a cast or a swat running ignores the press (a busy hero cannot
    // start the blink), so the kit presses only on a free tick.
    let free = combat.hurt_ticks_of(HeroId::Donut) == 0
        && combat.donut_cast().is_none()
        && combat.donut_move_view().is_none();
    if kit && free && mana >= BLINK_COST && !blink.cooling(tick) && !blink.recovering(tick) {
        let recent_hurts = combat
            .events
            .iter()
            .rev()
            .take_while(|e| match e {
                CombatEvent::HeroHurt { tick: t, .. } => {
                    tick.saturating_sub(*t) <= DONUT_ESCAPE_WINDOW
                }
                _ => true,
            })
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::HeroHurt {
                        hero: HeroId::Donut,
                        tick: t,
                        ..
                    } if tick.saturating_sub(*t) <= DONUT_ESCAPE_WINDOW
                )
            })
            .count();
        let escape = recent_hurts >= DONUT_ESCAPE_HURTS;
        let close = dist > DONUT_BLINK_GAP_PX && tick.is_multiple_of(KIT_CADENCE);
        if escape || close {
            // The blink goes the way she holds (facing when none): away from the
            // target to escape, toward it to close the gap. One press, nothing
            // else on the tick (a press in the windup would cancel it).
            let dir = if escape { -toward } else { toward };
            if !refused(dir) {
                raw.move_x = dir;
                raw.ability = true;
                return Some(raw);
            }
        }
    }
    let facing = f32::from(body.facing);
    let facing_target = facing * toward > 0.0;
    let step_back = |raw: &mut RawInput| -> bool {
        let away = -toward;
        if refused(away) || blocked_ahead(scene, body.x, body.y, away) {
            return false;
        }
        raw.move_x = away;
        true
    };
    // Point-blank or dry: the pounce (Physical), from its reach.
    let walled = bolt_blocked(scene, body.x, body.y, target);
    if !walled && (dist <= DONUT_POINT_BLANK_PX || (mana == 0 && dist <= DONUT_POUNCE_REACH_PX)) {
        if !facing_target {
            raw.move_x = toward;
        } else if press {
            raw.physical = true;
        }
        return Some(raw);
    }
    if mana == 0 || walled {
        walk(&mut raw, toward);
        return Some(raw);
    }
    // Facing away after a back step: keep going until the turn tick's step
    // cannot undo it.
    if !facing_target {
        if dist < DONUT_STANDOFF_PX + DONUT_TURN_MARGIN_PX && step_back(&mut raw) {
            return Some(raw);
        }
        raw.move_x = toward;
        return Some(raw);
    }
    if dist < DONUT_STANDOFF_PX {
        if !step_back(&mut raw) && press {
            // No legal back step: pounce rather than cast inside the stand-off.
            raw.physical = true;
        }
        return Some(raw);
    }
    // Out of cast reach, or a wall between (the parapet): walk on, and jump what
    // stands in the way.
    if dist > DONUT_CAST_REACH_PX || walled {
        walk(&mut raw, toward);
        return Some(raw);
    }
    if press {
        raw.attack = true;
    }
    Some(raw)
}

/// FA-Q17: the number of distinct [`Outcome`]s of `reports` (one per seed).
pub fn distinct_outcomes(reports: &[Report]) -> usize {
    let mut seen: Vec<Outcome> = Vec::new();
    for r in reports {
        let o = r.outcome();
        if !seen.contains(&o) {
            seen.push(o);
        }
    }
    seen.len()
}

/// What a pilot run saw.
#[derive(Debug, Clone)]
pub struct Report {
    pub who: Who,
    pub seed: u64,
    /// The tick the last roster enemy was defeated on (the exit gate opens).
    pub cleared_tick: Option<u64>,
    /// The tick a hero first stood in the exit gate's column after the clear.
    pub exit_tick: Option<u64>,
    /// The longest stuck run per hero (see the module docs).
    pub max_stuck: [u32; 2],
    /// FA-Q13: the longest run of ticks with no movement input (a walk or a
    /// jump) by the controlled hero while a wave entry lives and a hurt to that
    /// hero lies in the trailing [`STALL_HURT_WINDOW`] ticks.
    pub max_stall_ticks: u32,
    pub deaths: u32,
    pub hero_hurts: usize,
    pub hash: u64,
    pub ticks: u64,
    pub events: Vec<CombatEvent>,
    /// The headless difficulty measure of the run (PT-TUNE, D1 to D5).
    pub difficulty: Difficulty,
}

/// FA-Q13's measure: the longest run of ticks with no movement input by the
/// controlled hero while a wave entry lives and a hurt to that hero lies in the
/// trailing [`STALL_HURT_WINDOW`] ticks.
#[derive(Debug, Default)]
struct Stall {
    run: u32,
    max: u32,
    last_hurt: [Option<u64>; 2],
}

impl Stall {
    fn hurt(&mut self, hero: HeroId, tick: u64) {
        self.last_hurt[hero.index()] = Some(tick);
    }

    /// One tick, after the step: `tick` is the tick just run.
    fn step(&mut self, tick: u64, hero: HeroId, wave_lives: bool, moving: bool) {
        let hurt_lately = self.last_hurt[hero.index()]
            .is_some_and(|t| tick.saturating_sub(t) <= STALL_HURT_WINDOW);
        self.run = if wave_lives && hurt_lately && !moving {
            self.run + 1
        } else {
            0
        };
        self.max = self.max.max(self.run);
    }
}

/// One seed's outcome for the distinct-outcome count (FA-Q17): the clear tick,
/// the hero hurts and the order the roster fell in.
pub type Outcome = (Option<u64>, usize, Vec<u32>);

impl Report {
    /// This run's [`Outcome`].
    pub fn outcome(&self) -> Outcome {
        let order = self
            .events
            .iter()
            .filter_map(|e| match e {
                CombatEvent::EnemyDefeated { target, .. } => Some(*target),
                _ => None,
            })
            .collect();
        (self.cleared_tick, self.hero_hurts, order)
    }

    /// The one line a table prints.
    pub fn line(&self) -> String {
        format!(
            "{} seed={} cleared={:?} exit={:?} stuck={:?} stall={} deaths={} hurts={} {} hash={:#018x}",
            self.who.name(),
            self.seed,
            self.cleared_tick,
            self.exit_tick,
            self.max_stuck,
            self.max_stall_ticks,
            self.deaths,
            self.hero_hurts,
            self.difficulty.line(),
            self.hash
        )
    }
}

/// Engagement thresholds of the difficulty measure (PTT-Q8): a melee enemy is
/// engaged within this horizontal gap of a present hero on the same side of the
/// parapet; the slime within its trigger distance (44 px) plus this gap.
pub const ENGAGE_MELEE_PX: f32 = 96.0;
const SLIME_TRIGGER_PX: f32 = 44.0;
/// The parapet (736..768): a melee enemy and a hero on its two sides are not
/// engaged.
const PARAPET_MID: f32 = 752.0;
/// A glob line this much cover blocks (the spit line is 42 to 48 px up).
const COVER_PX: f32 = 48.0;
/// A Rot Sticker is awake within this distance of a hero (HAZ-Q25).
const ROT_WAKE_PX: f32 = 144.0;

/// The five threat kinds of D2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Threat {
    Goblin = 0,
    Engineer = 1,
    Llama = 2,
    Slime = 3,
    Rot = 4,
}

/// One wave's measure (D1, D2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WaveDifficulty {
    /// Ticks from the first engaged threat of the wave until its last roster
    /// entry is defeated.
    pub active_ticks: u32,
    /// Of those, the ticks with at least two engaged threats.
    pub overlap_ticks: u32,
    /// The most distinct kinds engaged on one tick.
    pub peak_kinds: u8,
    /// Bit set of the kinds ever engaged in the wave (PTT-Q18); see
    /// [`Self::kinds_seen`].
    pub seen_mask: u8,
}

impl WaveDifficulty {
    /// D2 (PTT-Q18): the distinct kinds engaged at any time over the wave.
    pub fn kinds_seen(&self) -> u8 {
        self.seen_mask.count_ones() as u8
    }

    /// D1: the overlap share in whole percent (0 for a wave that never ran).
    pub fn overlap_percent(&self) -> u32 {
        (self.overlap_ticks * 100)
            .checked_div(self.active_ticks)
            .unwrap_or(0)
    }
}

/// The headless difficulty measure of a run (`pt-tune-design.md` section 3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Difficulty {
    /// W1 and W2.
    pub waves: [WaveDifficulty; 2],
    /// D5: the most engaged enemies (the Rot Sticker excluded) on one tick.
    pub peak_engaged: u8,
    /// D4: hero hurts taken up to and after the tick W1's last entry fell.
    pub hurts: [u32; 2],
}

impl Difficulty {
    pub fn line(&self) -> String {
        let w = |d: &WaveDifficulty| {
            format!(
                "{}%/{}t/{}k/{}s",
                d.overlap_percent(),
                d.active_ticks,
                d.peak_kinds,
                d.kinds_seen()
            )
        };
        format!(
            "W1={} W2={} peak={} hurts={}/{}",
            w(&self.waves[0]),
            w(&self.waves[1]),
            self.peak_engaged,
            self.hurts[0],
            self.hurts[1]
        )
    }
}

/// Samples [`Difficulty`] once per fixed tick, from the roster's beats.
struct Meter {
    /// The beat (0 = W1, 1 = W2) of each goblin, then each Engineer, then each
    /// llama of the fixture (roster order per kind), and the slime's.
    goblin_beats: Vec<usize>,
    engineer_beats: Vec<usize>,
    llama_beats: Vec<usize>,
    slime_beat: Option<usize>,
    started: [bool; 2],
    /// The tick W1's last entry was defeated.
    w1_done: Option<u64>,
    out: Difficulty,
}

impl Meter {
    fn new(root: &Path) -> Result<Self, String> {
        use crate::camera_lab::room::EnemyKind;
        use crate::encounter::RosterKind;
        let run = crate::f01_mastery::launch(root)?;
        let mut m = Meter {
            goblin_beats: Vec::new(),
            engineer_beats: Vec::new(),
            llama_beats: Vec::new(),
            slime_beat: None,
            started: [false; 2],
            w1_done: None,
            out: Difficulty::default(),
        };
        for e in &run.loaded.encounter.roster {
            let beat = usize::from(e.beat == "W2");
            match e.kind {
                RosterKind::Slime => m.slime_beat = Some(beat),
                RosterKind::Room(EnemyKind::Llama) => m.llama_beats.push(beat),
                RosterKind::Room(EnemyKind::Engineer) => m.engineer_beats.push(beat),
                RosterKind::Room(_) => m.goblin_beats.push(beat),
                // DOZER-CORE: no mastery room holds a machine.
                RosterKind::Dozer | RosterKind::Trog(_) => {}
            }
        }
        Ok(m)
    }

    /// One tick, after the step.
    fn observe(&mut self, combat: &CombatState, scene: &SceneSim) {
        let mut heroes: Vec<f32> = Vec::new();
        if !combat.hero_defeated {
            heroes.push(scene.world.hero.x);
        }
        heroes.push(scene.world.donut_body.x);
        let same_side = |a: f32, b: f32| (a < PARAPET_MID) == (b < PARAPET_MID);
        // (beat, kind) of every engaged threat, and of every living entry.
        let mut engaged: Vec<(usize, Threat)> = Vec::new();
        let mut alive = [0u32; 2];
        let (mut goblin_i, mut engineer_i) = (0, 0);
        for g in &combat.enemies {
            let (beat, kind) = if g.is_engineer() {
                engineer_i += 1;
                (self.engineer_beats.get(engineer_i - 1), Threat::Engineer)
            } else {
                goblin_i += 1;
                (self.goblin_beats.get(goblin_i - 1), Threat::Goblin)
            };
            let Some(&beat) = beat else { continue };
            if !g.alive() {
                continue;
            }
            alive[beat] += 1;
            let attacking = g.mode == crate::goblin::GoblinMode::Attack;
            if attacking
                || heroes
                    .iter()
                    .any(|h| (h - g.x).abs() <= ENGAGE_MELEE_PX && same_side(g.x, *h))
            {
                engaged.push((beat, kind));
            }
        }
        let left = scene.camera.left();
        let view = crate::llama::FireView {
            left: left as f32,
            right: (left + scene.camera.config().view_w) as f32,
        };
        let foot = scene.world.hero_foot_y();
        for (i, l) in combat.llamas.iter().enumerate() {
            let Some(&beat) = self.llama_beats.get(i) else {
                continue;
            };
            if !l.alive() {
                continue;
            }
            alive[beat] += 1;
            let covered = |hx: f32| {
                let (lo, hi) = (l.x.min(hx), l.x.max(hx));
                scene.world.solids.iter().any(|b| {
                    b.intact
                        && b.rect.left < hi
                        && b.rect.right() > lo
                        && b.rect.bottom <= foot + 4.0
                        && b.rect.bottom + b.rect.height - foot >= COVER_PX
                })
            };
            let in_flight = combat.globs.iter().any(|g| g.owner as usize == i);
            let fires = heroes.iter().any(|h| l.may_fire(*h, view) && !covered(*h));
            if l.mode == crate::llama::LlamaMode::Attack || in_flight || fires {
                engaged.push((beat, Threat::Llama));
            }
        }
        if let Some(beat) = self.slime_beat {
            for e in combat.slot_enemies() {
                let Some(sl) = e
                    .as_any()
                    .and_then(|a| a.downcast_ref::<crate::combat_scene::slime::SlimeEnemy>())
                else {
                    continue;
                };
                if !e.alive() {
                    continue;
                }
                alive[beat] += 1;
                let sx = sl.slime().x;
                if sl.slime().attack_tick().is_some()
                    || heroes
                        .iter()
                        .any(|h| (h - sx).abs() <= SLIME_TRIGGER_PX + ENGAGE_MELEE_PX)
                {
                    engaged.push((beat, Threat::Slime));
                }
            }
        }
        // The Rot Sticker belongs to the yard (W2): engaged while it is armed
        // and a hero is within its wake distance (or it already acts).
        if let Some(env) = combat.env() {
            let t = crate::rot_sticker::active();
            for (_, st, _) in env.stickers() {
                if st.is_spent() {
                    continue;
                }
                let x = st.dome(t).0;
                if st.state() != crate::rot_sticker::StickerState::Latched
                    || heroes.iter().any(|h| (h - x).abs() <= ROT_WAKE_PX)
                {
                    engaged.push((1, Threat::Rot));
                }
            }
        }
        let enemies = engaged.iter().filter(|(_, k)| *k != Threat::Rot).count() as u8;
        self.out.peak_engaged = self.out.peak_engaged.max(enemies);
        for (beat, &alive_now) in alive.iter().enumerate() {
            let here: Vec<Threat> = engaged
                .iter()
                .filter(|(b, _)| *b == beat)
                .map(|(_, k)| *k)
                .collect();
            if !here.is_empty() {
                self.started[beat] = true;
            }
            if self.started[beat] && alive_now == 0 && beat == 0 && self.w1_done.is_none() {
                self.w1_done = Some(scene.world.tick);
            }
            if !self.started[beat] || alive_now == 0 {
                continue;
            }
            let w = &mut self.out.waves[beat];
            w.active_ticks += 1;
            if here.len() >= 2 {
                w.overlap_ticks += 1;
            }
            let mut kinds = [false; 5];
            for k in &here {
                kinds[*k as usize] = true;
            }
            w.peak_kinds = w.peak_kinds.max(kinds.iter().filter(|k| **k).count() as u8);
            for (i, k) in kinds.iter().enumerate() {
                w.seen_mask |= u8::from(*k) << i;
            }
        }
    }
}

/// A launched pilot sim: the room built the way the scene builds it, under the
/// party of `who`, with the arena's Carl autopilot as the fixture's pilot (the
/// companion metrics harness steps through it).
pub fn start(
    root: &Path,
    who: Who,
    seed: u64,
) -> Result<(PartyFixtureSim, &'static PartyFixture), String> {
    start_with(root, seed, tuning(who))
}

/// [`start`] under an explicit party tuning (a test reproduces the earlier
/// reflex profile through it).
fn start_with(
    root: &Path,
    seed: u64,
    party: PartyTuning,
) -> Result<(PartyFixtureSim, &'static PartyFixture), String> {
    let (carl, donut) = Loadout::Story.flags();
    let (run, scene, combat) = mastery_start(root, 360, carl, donut, Some(seed))?;
    let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        base: run.fixture,
        two_player: false,
        tuning: party,
        script: &[],
        pilot: Pilot::ArenaDirect,
        donut: DonutSetup::unlocked(8),
    }));
    Ok((PartyFixtureSim::from_parts(scene, combat, fixture), fixture))
}

/// Run one pilot for [`RUN_TICKS`] (or until the exit is reached) at seed
/// `seed`.
pub fn run(root: &Path, who: Who, seed: u64) -> Result<Report, String> {
    run_for(root, who, seed, RUN_TICKS)
}

pub fn run_for(root: &Path, who: Who, seed: u64, ticks: u64) -> Result<Report, String> {
    let (sim, _fixture) = start(root, who, seed)?;
    run_sim(root, who, seed, ticks, PilotRules::SHIPPED, sim)
}

/// [`run_for`] on a started `sim` under explicit [`PilotRules`].
fn run_sim(
    root: &Path,
    who: Who,
    seed: u64,
    ticks: u64,
    rules: PilotRules,
    mut sim: PartyFixtureSim,
) -> Result<Report, String> {
    let mut meter = Meter::new(root)?;
    let kit = matches!(who, Who::CarlKit | Who::DonutKit);
    let mut last_x = [sim.scene.world.hero.x, sim.scene.world.donut_body.x];
    let mut stuck = [0u32; 2];
    let mut max_stuck = [0u32; 2];
    let mut cleared_tick = None;
    let mut exit_tick = None;
    // RING-FU-A (review F1 of #271): a hero defeat in a room with checkpoints
    // respawns the party without a restart, so the count is the combat state's
    // own `deaths`, never the restart counter.
    let deaths_at_start = sim.combat.deaths;
    let mut swap_pressed = false;
    // FA-Q13: the stall run of the controlled hero.
    let mut stall = Stall::default();
    let mut events_seen = 0usize;
    for _ in 0..ticks {
        let tick = sim.scene.world.tick + 1;
        let controlled = sim.party.controlled_hero();
        let (slots, walking) = match who {
            Who::CarlSolo | Who::CarlKit => {
                let input = pilot_rules(&sim.combat, &sim.scene, HeroId::Carl, kit, rules);
                ([input, RawInput::NONE], [input.move_x != 0.0, false])
            }
            Who::DonutBySwap | Who::DonutFights | Who::DonutKit => {
                // The swap is pressed once: on `SWAP_TICK` for the fighter, the
                // tick after the clear for the walker. The pilot then plays
                // whoever is controlled (a restart returns control to Carl).
                let due = match who {
                    Who::DonutFights | Who::DonutKit => tick >= SWAP_TICK,
                    _ => sim.combat.roster_cleared(),
                };
                let mut input = pilot_rules(&sim.combat, &sim.scene, controlled, kit, rules);
                if due && !swap_pressed && controlled == HeroId::Carl {
                    input.swap = true;
                    swap_pressed = true;
                }
                let mut walking = [false; 2];
                let at = HeroId::ALL.iter().position(|h| *h == controlled);
                walking[at.unwrap_or(0)] = input.move_x != 0.0;
                ([input, RawInput::NONE], walking)
            }
            Who::Companion => {
                // Carl is the pilot's; Donut is the autonomous partner's.
                let input = pilot_rules(&sim.combat, &sim.scene, HeroId::Carl, false, rules);
                ([input, RawInput::NONE], [input.move_x != 0.0, false])
            }
        };
        sim.step_slots(slots);
        meter.observe(&sim.combat, &sim.scene);
        for e in &sim.combat.events[events_seen..] {
            if let CombatEvent::HeroHurt { hero, tick, .. } = e {
                stall.hurt(*hero, *tick);
            }
        }
        events_seen = sim.combat.events.len();
        let wave_lives = sim.combat.enemies.iter().any(|g| g.alive())
            || sim.combat.llamas.iter().any(|l| l.alive())
            || sim.combat.slot_enemies().iter().any(|e| e.alive());
        stall.step(
            sim.scene.world.tick,
            controlled,
            wave_lives,
            slots[0].move_x != 0.0 || slots[0].jump,
        );
        let carl_x = sim.scene.world.hero.x;
        let carl_moved = (carl_x - last_x[0]).abs() >= 0.1;
        for (i, id) in HeroId::ALL.into_iter().enumerate() {
            let x = sim.scene.world.body(id).x;
            let still = (x - last_x[i]).abs() < 0.1;
            let counts = if who == Who::Companion && id == HeroId::Donut {
                // Left behind: she stands still, apart from Carl, while he walks on.
                still && (x - carl_x).abs() > APART_PX && carl_moved
            } else {
                walking[i] && still
            };
            stuck[i] = if counts { stuck[i] + 1 } else { 0 };
            max_stuck[i] = max_stuck[i].max(stuck[i]);
            last_x[i] = x;
        }
        if cleared_tick.is_none() && sim.combat.roster_cleared() {
            cleared_tick = Some(sim.scene.world.tick);
        }
        if cleared_tick.is_some()
            && exit_tick.is_none()
            && HeroId::ALL
                .into_iter()
                .any(|id| sim.scene.world.body(id).x >= EXIT_X)
        {
            exit_tick = Some(sim.scene.world.tick);
        }
        if exit_tick.is_some() {
            break;
        }
    }
    let hero_hurts = sim
        .combat
        .events
        .iter()
        .filter(|e| matches!(e, CombatEvent::HeroHurt { .. }))
        .count();
    Ok(Report {
        who,
        seed,
        cleared_tick,
        exit_tick,
        max_stuck,
        max_stall_ticks: stall.max,
        deaths: sim.combat.deaths - deaths_at_start,
        hero_hurts,
        hash: sim.state_hash(),
        ticks: sim.scene.world.tick,
        events: sim.combat.events.clone(),
        difficulty: {
            let mut d = meter.out;
            let done = meter.w1_done.unwrap_or(u64::MAX);
            for e in &sim.combat.events {
                if let CombatEvent::HeroHurt { tick, .. } = e {
                    d.hurts[usize::from(*tick > done)] += 1;
                }
            }
            d
        },
    })
}

/// The CA2 metrics of the autonomous partner in the closing arena (spec 7.1,
/// M1 to M16): the room is a `cf02_rules` room, so CA2's harness runs here. The
/// run is [`METRICS_TICKS`] long (the `cf02_free_arena_direct` length) and is
/// driven by the room's Carl pilot ([`pilot`]; the arena autopilot cannot bait
/// the slime, PT-TUNE).
pub const METRICS_TICKS: u64 = 1800;

/// One seed's metrics and the `cf02_assist` baseline M13 reads.
pub fn metrics(
    root: &Path,
    seed: u64,
) -> Result<(crate::cf02_free_fixtures::AutonomyMetrics, u32), String> {
    // PT-TUNE: Carl is the room's pilot (it baits the slime), the same inputs
    // under both partners.
    let step = |sim: &mut PartyFixtureSim| {
        let input = arena_pilot(&sim.combat, &sim.scene, HeroId::Carl);
        sim.step_slots([input, RawInput::NONE]);
    };
    let (mut free, _) = start(root, Who::Companion, seed)?;
    let m = crate::cf02_free_fixtures::autonomy_metrics_with(&mut free, METRICS_TICKS, step);
    // M13's baseline: the same inputs under the assist partner.
    let (mut assist, _) = start(root, Who::CarlSolo, seed)?;
    let baseline =
        crate::cf02_free_fixtures::autonomy_metrics_with(&mut assist, METRICS_TICKS, step);
    Ok((m, baseline.her_landed()))
}

/// One printable line of the figures (M1 to M16 as the spec words them).
pub fn metrics_line(
    seed: u64,
    m: &crate::cf02_free_fixtures::AutonomyMetrics,
    assist_hits: u32,
) -> String {
    format!(
        "seed={seed} M1={}%/{:.0}px M2={}%(escort/idle {}% of {}t) M3={}%/{}t M4={}/{} M5={}+{}inv M6={}/{}ahead \
         M10={}x{:?}/{}h M12={} M13={}hits({}%) vs assist {assist_hits} M15={}% \
         M16={}/{}/{}",
        m.spread_percent(),
        m.median_distance_px,
        m.shadow_percent(),
        m.shadow_escort_percent(),
        m.both_moving_escort_ticks,
        m.distinct_percent(),
        m.same_target_ticks,
        m.steals,
        m.start_steals,
        m.episodes_self_caused,
        m.episodes_involuntary,
        m.outside_ticks,
        m.ahead_exits,
        m.retreats,
        m.retreat_distances
            .iter()
            .map(|d| d.map(|d| d.round()))
            .collect::<Vec<_>>(),
        m.hits_in_retreat,
        m.regroup_teleports,
        m.her_landed(),
        m.hit_share_percent(),
        m.retreat_share_percent(),
        m.interrupt_drift_skip_ticks,
        m.goblins_targeted_her,
        m.hunt_drift_rejected,
    )
}

/// T15: the CA2 metrics the closing arena had not measured (spec 7.1: M7, M8,
/// the M9 cause split, M11, and M14's cost), on the same run as [`metrics`]
/// (`cf02_free`, Carl on the room pilot, [`METRICS_TICKS`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArenaExtras {
    /// M7, for her position shifted by -200 and +200 px: the automatic camera's
    /// rest-width series, its zoom changes and its per-tick width are equal to
    /// the unperturbed run's.
    pub m7_rest_equal: [bool; 2],
    pub m7_zoom_equal: [bool; 2],
    pub m7_width_equal: [bool; 2],
    /// M8: per configuration of [`CAMERA_CONFIGS`], the ticks her body (12 px
    /// either side of her centre) is not inside the automatic camera's view,
    /// and the first such tick and her column.
    pub m8_misses: [u32; 4],
    pub m8_first: Option<(usize, u64, f32)>,
    /// M9: the hurts to her by cause (`HeroHurt` carries a source id).
    pub m9: HurtSplit,
    /// M11: show-off holds started, those started with a standing enemy within
    /// 128 px of her, and those started while a goblin's tell was running.
    pub m11_starts: u32,
    pub m11_enemy_near_starts: u32,
    pub m11_tell_starts: u32,
}

/// M9's hurts to Donut by cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HurtSplit {
    pub acid: u32,
    pub rot: u32,
    pub shore: u32,
    pub slime: u32,
    /// Goblin, Engineer and llama (every other source).
    pub enemy: u32,
}

impl HurtSplit {
    /// M9 counts the hazards (acid, Rot, the shore block), not the slime or the
    /// enemies, whose attacks the other metrics cover.
    pub fn hazard(&self) -> u32 {
        self.acid + self.rot + self.shore
    }
}

/// M14: the wall time of one fixed step of the whole simulation, median and
/// worst, in nanoseconds. Autonomy planning is not separable from the step, so
/// the figure is an upper bound; the same inputs under the `cf02_assist`
/// partner give the cost without the autonomy layer. No budget is claimed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StepCost {
    pub median_ns: u64,
    pub max_ns: u64,
}

impl StepCost {
    fn of(mut samples: Vec<u64>) -> Self {
        samples.sort_unstable();
        Self {
            median_ns: samples.get(samples.len() / 2).copied().unwrap_or(0),
            max_ns: samples.last().copied().unwrap_or(0),
        }
    }
}

/// The M8 configurations (aspect, output width), as the camera tests run them.
pub const CAMERA_CONFIGS: [(crate::camera_lab::camera_op::Aspect, u32); 4] = [
    (crate::camera_lab::camera_op::Aspect::Wide16x9, 1280),
    (crate::camera_lab::camera_op::Aspect::Wide16x9, 1920),
    (crate::camera_lab::camera_op::Aspect::Deck16x10, 1280),
    (crate::camera_lab::camera_op::Aspect::Deck16x10, 1920),
];

/// M11's radius: a show-off starts only with no standing enemy this near.
const SHOW_OFF_QUIET_PX: f32 = 128.0;

/// Measure [`ArenaExtras`] at `seed`, and the step cost with the autonomous
/// partner and with the assist partner (M14).
pub fn arena_extras(root: &Path, seed: u64) -> Result<(ArenaExtras, [StepCost; 2]), String> {
    use crate::camera_lab::camera_op::Bounds;
    use crate::companion::autonomy::Goal;
    use crate::scene_camera::{
        CameraChoice, CameraInterest, RigConfig, SceneCameraRig, llama_interest,
    };
    let run = crate::f01_mastery::launch(root)?;
    let bounds = Bounds {
        w: run.kit.width,
        h: run.kit.height,
    };
    let (mut sim, _) = start(root, Who::Companion, seed)?;
    let rig_cfg = |(aspect, out_w): (crate::camera_lab::camera_op::Aspect, u32)| RigConfig {
        choice: CameraChoice::Auto,
        aspect,
        out_w,
        bounds,
    };
    let mut interest = CameraInterest::default();
    // The M8 rigs, then the M7 base rig (wide, 1920) and its two shifts.
    let mut rigs: Vec<SceneCameraRig> = CAMERA_CONFIGS
        .into_iter()
        .map(|c| SceneCameraRig::new(rig_cfg(c), &sim.scene, Some(&sim.combat), &interest, 0))
        .collect::<Option<_>>()
        .ok_or("the automatic camera has no rig")?;
    let shifts = [0.0_f32, -200.0, 200.0];
    let mut m7: Vec<SceneCameraRig> = shifts
        .iter()
        .map(|_| {
            SceneCameraRig::new(
                rig_cfg(CAMERA_CONFIGS[1]),
                &sim.scene,
                Some(&sim.combat),
                &interest,
                0,
            )
        })
        .collect::<Option<_>>()
        .ok_or("the automatic camera has no rig")?;
    let mut series: Vec<(Vec<i32>, Vec<i32>)> = vec![(Vec::new(), Vec::new()); 3];
    let mut out = ArenaExtras::default();
    let mut costs = Vec::new();
    let mut goal = Goal::Idle;
    for _ in 0..METRICS_TICKS {
        let input = arena_pilot(&sim.combat, &sim.scene, HeroId::Carl);
        let t0 = std::time::Instant::now();
        sim.step_slots([input, RawInput::NONE]);
        costs.push(t0.elapsed().as_nanos() as u64);
        let tick = sim.scene.world.tick;
        interest.clear();
        llama_interest(&sim.scene, &sim.combat, &mut interest);
        let elapsed = sim.combat.elapsed;
        let body = sim.scene.world.donut_body.x;
        for (i, rig) in rigs.iter_mut().enumerate() {
            let v = rig.sync(&sim.scene, Some(&sim.combat), &interest, elapsed);
            let inside = body - HERO_HALF_WIDTH >= v.left as f32
                && body + HERO_HALF_WIDTH <= v.right() as f32;
            if !inside {
                out.m8_misses[i] += 1;
                out.m8_first.get_or_insert((i, tick, body));
            }
        }
        for (i, shift) in shifts.iter().enumerate() {
            let real = sim.scene.world.donut.x;
            sim.scene.world.donut.x += shift;
            let v = m7[i].sync(&sim.scene, Some(&sim.combat), &interest, elapsed);
            sim.scene.world.donut.x = real;
            series[i].0.push(m7[i].operator().rest_target());
            series[i].1.push(v.w);
        }
        // M11: a show-off hold starts.
        let now = sim.combat.autonomy_state(HeroId::Donut).goal();
        if now == Goal::ShowOff && goal != Goal::ShowOff {
            out.m11_starts += 1;
            let near = sim
                .combat
                .enemies
                .iter()
                .filter(|g| g.alive() && g.mode != crate::goblin::GoblinMode::Hurt)
                .any(|g| (g.x - body).abs() <= SHOW_OFF_QUIET_PX);
            out.m11_enemy_near_starts += u32::from(near);
            let tell = sim
                .combat
                .enemies
                .iter()
                .any(|g| g.alive() && g.mode == crate::goblin::GoblinMode::Attack);
            out.m11_tell_starts += u32::from(tell);
        }
        goal = now;
    }
    for (k, sign) in [(1usize, 0usize), (2, 1)] {
        out.m7_rest_equal[sign] = series[k].0 == series[0].0;
        out.m7_zoom_equal[sign] = m7[k].operator().zoom_changes == m7[0].operator().zoom_changes;
        out.m7_width_equal[sign] = series[k].1 == series[0].1;
    }
    for e in &sim.combat.events {
        let CombatEvent::HeroHurt {
            hero: HeroId::Donut,
            source,
            ..
        } = e
        else {
            continue;
        };
        let h = &mut out.m9;
        match source & 0xffff {
            crate::slime::ACID_SOURCE_ID => h.acid += 1,
            crate::rot_sticker::ROT_SOURCE_ID => h.rot += 1,
            crate::slime::SLIME_SOURCE_ID => h.slime += 1,
            _ if *source == crate::cf02::arena::SHORE_BLOCK_SOURCE => h.shore += 1,
            _ => h.enemy += 1,
        }
    }
    // M14's baseline: the same inputs under the assist partner.
    let (mut assist, _) = start(root, Who::CarlSolo, seed)?;
    let mut base = Vec::new();
    for _ in 0..METRICS_TICKS {
        let input = arena_pilot(&assist.combat, &assist.scene, HeroId::Carl);
        let t0 = std::time::Instant::now();
        assist.step_slots([input, RawInput::NONE]);
        base.push(t0.elapsed().as_nanos() as u64);
    }
    Ok((out, [StepCost::of(costs), StepCost::of(base)]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::MoveId;
    use std::path::PathBuf;

    /// FIX-TOOL: the left-behind measure counts only a still partner, strictly farther than
    /// `APART_PX`, while the steered hero moves; any other tick resets the run, and the
    /// maximum keeps the longest run.
    #[test]
    fn left_behind_counts_a_still_distant_partner_while_the_steered_hero_moves() {
        let mut m = LeftBehind::default();
        // On the bound (96 px) is not apart; just past it is.
        m.observe(true, APART_PX, true);
        assert_eq!(m.max(), 0, "exactly APART_PX is not apart");
        m.observe(true, APART_PX + 0.5, true);
        m.observe(true, APART_PX + 0.5, true);
        assert_eq!(m.max(), 2);
        // A moving partner, a standing steered hero and a near partner each reset the run.
        m.observe(false, 200.0, true);
        m.observe(true, 200.0, true);
        assert_eq!(m.max(), 2, "the run restarted at one");
        m.observe(true, 200.0, false);
        m.observe(true, 10.0, true);
        m.observe(true, 200.0, true);
        m.observe(true, 200.0, true);
        m.observe(true, 200.0, true);
        assert_eq!(m.max(), 3, "a longer run replaces the maximum");
        m.observe(true, 200.0, true);
        assert_eq!(m.max(), 4);
    }

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// RING-FU-A (review F1 of #271): `Report::deaths` sees a defeat that the
    /// checkpoint respawns without a restart. Carl solo at seed 2 is defeated
    /// once, logs no `Restart`, and the report says 1 (the restart counter said
    /// 0); at seed 0 he is never defeated.
    #[test]
    fn the_death_count_sees_a_defeat_the_checkpoint_respawns_without_a_restart() {
        let count = |r: &Report, restart: bool| {
            r.events
                .iter()
                .filter(|e| {
                    if restart {
                        matches!(e, CombatEvent::Restart { .. })
                    } else {
                        matches!(e, CombatEvent::HeroDefeated { .. })
                    }
                })
                .count()
        };
        let r = run(&root(), Who::CarlSolo, 2).unwrap();
        assert_eq!(
            (r.deaths, count(&r, false), count(&r, true)),
            (1, 1, 0),
            "{}",
            r.line()
        );
        let r = run(&root(), Who::CarlSolo, 0).unwrap();
        assert_eq!((r.deaths, count(&r, false)), (0, 0), "{}", r.line());
    }

    /// The room as `--scene f01_late --f01-room mastery` builds it under the story
    /// loadout (seed 0), with the follow-and-jump bot as the other hero so that only
    /// Carl's script fights.
    fn story_room_with_a_bot_partner() -> PartyFixtureSim {
        let (carl, donut) = Loadout::Story.flags();
        let (run, scene, combat) = mastery_start(&root(), 360, carl, donut, Some(0)).unwrap();
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base: run.fixture,
            two_player: false,
            tuning: PROVISIONAL_TUNING.with_cf02_swap(),
            script: &[],
            pilot: Pilot::ArenaDirect,
            donut: DonutSetup::unlocked(8),
        }));
        PartyFixtureSim::from_parts(scene, combat, fixture)
    }

    /// PT-GATE-C1 (gate item 1, the gauntlet in `f01_late`): under the mastery
    /// room's story pin the reinforced fist is held, and Carl's haymaker on a
    /// goblin that is telling produces the CP-D stagger and pierce events: the
    /// 6-tick hit stop (CP-Q39: 5, +1 with the fist), the 28-tick stagger (18
    /// without it), `goblin_interrupted` on the same tick and one pierce
    /// (CP-Q40, CP-Q42). The script is a staged duel: goblin 0 is awake and its
    /// neighbour (goblin 1) is already telling when Carl is stood 40 px right of
    /// goblin 0 on tick 160; he jabs on tick 173 and chains the haymaker on 185
    /// (the same string as the `carl_prog_pierce_goblin` fixture, pressed from
    /// that spot), and it lands on tick 190.
    #[test]
    fn the_gauntlet_staggers_and_pierces_in_the_mastery_room_under_the_story_pin() {
        const STAGE: u64 = 160;
        const JAB: u64 = 13;
        const HAYMAKER: u64 = 25;
        let mut sim = story_room_with_a_bot_partner();
        let held = |sim: &PartyFixtureSim| -> Vec<(u8, &'static str)> {
            sim.combat
                .story_ledger()
                .expect("a story runtime")
                .held()
                .iter()
                .map(|(hero, id)| (*hero, id.as_str()))
                .collect()
        };
        assert!(
            held(&sim).contains(&(0, "fist_reinforced")),
            "the f01_late_mastery pin holds the reinforced fist: {:?}",
            held(&sim)
        );
        let mut stagger_after_hit = None;
        for _ in 0..(STAGE + 40) {
            let tick = sim.scene.world.tick + 1;
            let at = tick.saturating_sub(STAGE);
            let mut input = RawInput::NONE;
            if tick >= STAGE && at < HAYMAKER {
                let gx = sim.combat.enemies[0].x;
                sim.scene.world.hero.x = gx + 40.0;
                sim.scene.world.hero.y = 118.0;
            }
            if tick == STAGE + 1 {
                // Face the goblin (a one-tick step toward it).
                input.move_x = -1.0;
            }
            if tick > STAGE + 1 && (at == JAB || at == HAYMAKER) {
                input.attack = true;
            }
            sim.step_slots([input, RawInput::NONE]);
            if tick == STAGE + HAYMAKER + 5 {
                stagger_after_hit = Some(sim.combat.enemies[0].stagger_left);
            }
        }
        let hits: Vec<_> = sim
            .combat
            .events
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Hit {
                    tick,
                    move_id: MoveId::Haymaker,
                    target,
                    hit_stop,
                    health_left,
                    ..
                } => Some((tick, target, hit_stop, health_left)),
                _ => None,
            })
            .collect();
        // The haymaker lands on tick 190 on both goblins in reach, with the fist's
        // extra stop frame (CP-Q39) and without defeating either.
        assert_eq!(hits, vec![(190, 0, 6, 1), (190, 1, 6, 1)], "{hits:?}");
        // The stagger read on the hit tick is the fist's 28 (18 without it, CP-Q39).
        assert_eq!(stagger_after_hit, Some(28));
        // CP-Q40: the telling goblin's committed swing is cancelled on the hit tick.
        assert!(
            sim.combat.events.iter().any(|e| matches!(
                e,
                CombatEvent::GoblinInterrupted {
                    tick: 190,
                    goblin: 1
                }
            )),
            "goblin 1's swing is cancelled by the haymaker"
        );
        assert_eq!(sim.combat.pierce_count(), 1, "one pierce (CP-Q42)");
        assert_eq!(sim.combat.pierce_cue().map(|c| c.tick), Some(190));
        assert!(
            held(&sim).contains(&(0, "fist_reinforced")),
            "the pin still holds"
        );
    }

    /// The final state hash of every pilot run, `[who][seed]`, in [`Who::ALL`]
    /// and [`SEEDS`] order. A change to the room, the sidecar, the pilot or the
    /// rules is a deliberate re-pin.
    const GOLDEN: [[u64; 9]; 4] = [
        [
            0xee25_fa69_fcdc_e0ea,
            0xc832_8504_cb7e_b986,
            0x5dcf_f6b2_8acc_9637,
            0xac16_1287_5acc_12e9,
            0xea91_d2fc_bf04_1610,
            0x62df_2d10_92c4_5420,
            0x67af_70c7_66a3_4fbc,
            0x56d0_a2d9_49f3_5a6c,
            0xf230_7efc_baf8_3993,
        ],
        [
            0x6b6c_5ba7_f32a_61d1,
            0x78a4_4ce0_1846_6c48,
            0x5538_fdae_9809_2d9b,
            0x8186_5a1b_9774_63f1,
            0x62fd_b9a1_774e_6bef,
            0x50e8_80b8_350a_8491,
            0x0258_0976_5343_12f6,
            0x7af9_c904_da26_8ee5,
            0x0344_443f_658c_83ac,
        ],
        [
            0x85ab_2857_e91c_f573,
            0x3b71_375b_a40e_38b7,
            0x33af_f462_3746_182d,
            0x72b3_072d_d462_7f01,
            0x5d45_0ab3_babb_e1a4,
            0xa55d_b1ba_dfdc_f4c4,
            0x656c_a235_b39d_4d23,
            0x8264_6dfa_f301_43ef,
            0x7edf_5fc4_fa0a_1990,
        ],
        [
            0xfaa2_a0ee_3dca_42ae,
            0x27e9_7530_db5a_88ec,
            0x7827_ae2f_afb7_8b93,
            0xea8a_e27a_52f7_59d8,
            0xed76_07ce_0ac9_2ea0,
            0xc288_461a_bc42_bdf8,
            0xf20e_75c9_b105_f7e1,
            0xc252_8edc_f628_bf2d,
            0x83a2_685d_3cac_ff08,
        ],
    ];

    #[test]
    fn every_pilot_replays_twice_and_no_hero_is_stuck_for_more_than_300_ticks() {
        for who in Who::ALL {
            for seed in SEEDS {
                let a = run(&root(), who, seed).unwrap();
                let b = run(&root(), who, seed).unwrap();
                assert_eq!(a.hash, b.hash, "{}", a.line());
                assert_eq!(a.events, b.events, "{}", a.line());
                assert_eq!(a.line(), b.line());
                assert!(
                    a.max_stuck.iter().all(|s| *s <= STUCK_LIMIT),
                    "{}",
                    a.line()
                );
            }
        }
    }

    /// Carl solo, Donut by swap (after Carl clears the room) and the companion
    /// pair all clear the arena and reach the exit at every seed: the exit gate
    /// opens on the clear and no destruction state of the shore blocks the way.
    #[test]
    fn the_room_is_cleared_and_the_exit_reached_in_every_party_at_every_seed() {
        for who in [Who::CarlSolo, Who::DonutBySwap, Who::Companion] {
            for seed in SEEDS {
                let r = run(&root(), who, seed).unwrap();
                let (cleared, exit) = (r.cleared_tick, r.exit_tick);
                assert!(cleared.is_some() && exit.is_some(), "{}", r.line());
                assert!(exit > cleared, "the gate opens on the clear: {}", r.line());
            }
        }
    }

    /// The exit gate is the one runtime solid: closed on every tick before the
    /// last roster enemy falls and open from the tick after (event, then step),
    /// and no hero stands in its column until then.
    #[test]
    fn the_exit_gate_is_closed_until_the_clear_and_open_after_it() {
        for seed in [0, 4, 5] {
            let (mut sim, _) = start(&root(), Who::CarlSolo, seed).unwrap();
            let gate = sim.combat.env().unwrap().solids().len();
            assert_eq!(gate, 1);
            let mut cleared_at = None;
            for _ in 0..RUN_TICKS {
                let input = arena_pilot(&sim.combat, &sim.scene, HeroId::Carl);
                sim.step_slots([input, RawInput::NONE]);
                let at = sim.combat.env().unwrap().solid_range().start;
                let intact = sim.scene.world.solids[at].intact;
                let tick = sim.scene.world.tick;
                if cleared_at.is_none() {
                    // Before the clear: closed, and nobody inside its column.
                    assert!(
                        intact || sim.combat.roster_cleared(),
                        "seed {seed} tick {tick}"
                    );
                    assert!(
                        sim.scene.world.hero.x < 1072.0 || sim.combat.roster_cleared(),
                        "seed {seed} tick {tick}: Carl passed the closed gate"
                    );
                    if sim.combat.roster_cleared() {
                        cleared_at = Some(tick);
                    }
                } else {
                    assert!(!intact, "seed {seed}: the gate is open after the clear");
                    break;
                }
            }
            assert!(cleared_at.is_some(), "seed {seed}");
        }
    }

    /// Donut by swap is the solo Donut: after the swap the controlled hero is
    /// Donut, and she is the one who walks through the opened gate.
    #[test]
    fn the_swapped_donut_is_the_hero_who_reaches_the_exit() {
        for seed in [0, 4] {
            let (mut sim, _) = start(&root(), Who::DonutBySwap, seed).unwrap();
            let mut at_exit = None;
            for _ in 0..RUN_TICKS {
                let controlled = sim.party.controlled_hero();
                let mut input = arena_pilot(&sim.combat, &sim.scene, controlled);
                input.swap = sim.combat.roster_cleared() && controlled == HeroId::Carl;
                sim.step_slots([input, RawInput::NONE]);
                if sim.combat.roster_cleared() && sim.scene.world.donut_body.x >= EXIT_X {
                    at_exit = Some(sim.party.controlled_hero());
                    break;
                }
            }
            assert_eq!(at_exit, Some(HeroId::Donut), "seed {seed}");
        }
    }

    /// Solo Donut with the base-kit pilot fights the whole room from the swap:
    /// it is deterministic and never stuck. Since FA-Q12 the pilot plays her
    /// rules (stand-off, pounce at point-blank or dry, approach) and clears the
    /// room (see `the_donut_pilots_clear_the_room_and_never_stall`).
    #[test]
    fn solo_donut_fighting_is_replayable_and_never_stuck() {
        for seed in SEEDS {
            let r = run(&root(), Who::DonutFights, seed).unwrap();
            assert!(
                r.max_stuck.iter().all(|s| *s <= STUCK_LIMIT),
                "{}",
                r.line()
            );
        }
    }

    #[test]
    fn the_pilot_hashes_are_pinned() {
        let mut got = [[0u64; 9]; 4];
        for (w, who) in Who::ALL.into_iter().enumerate() {
            for seed in SEEDS {
                got[w][seed as usize] = run(&root(), who, seed).unwrap().hash;
            }
        }
        assert_eq!(got, GOLDEN, "{got:#018x?}");
    }

    /// The final state hash of every D7 kit-pilot run, `[kit][seed]`, in
    /// [`Who::KITS`] and [`SEEDS`] order (PT-TUNE, new in this PR).
    const KIT_GOLDEN: [[u64; 9]; 2] = [
        [
            0xf998_0cf4_8783_4182,
            0x7869_2730_cd07_5d32,
            0x26e8_78df_d5a2_e1ed,
            0xc11c_9e86_8bbe_e094,
            0xc20e_2d56_9dd6_4759,
            0x28f6_d3df_21f5_fb95,
            0x72b6_f402_d8fc_6ef6,
            0x01ac_6311_92ff_a79d,
            0x5e8c_ac54_545b_3c9f,
        ],
        [
            0xc12f_5e9d_8e08_3c77,
            0xd85b_4015_6a99_7c69,
            0x33af_f462_3746_182d,
            0x781e_bebd_e54c_e599,
            0x4e61_0f87_0ce9_5b92,
            0xa55d_b1ba_dfdc_f4c4,
            0x221c_dc96_93a5_4656,
            0xc818_a13c_1f5b_eef6,
            0x68b3_b1b1_132a_77fd,
        ],
    ];

    #[test]
    fn the_kit_pilot_hashes_are_pinned_replay_twice_and_are_never_stuck() {
        let mut got = [[0u64; 9]; 2];
        for (w, who) in Who::KITS.into_iter().enumerate() {
            for seed in SEEDS {
                let a = run(&root(), who, seed).unwrap();
                let b = run(&root(), who, seed).unwrap();
                assert_eq!((a.hash, &a.events), (b.hash, &b.events), "{}", a.line());
                assert!(
                    a.max_stuck.iter().all(|s| *s <= STUCK_LIMIT),
                    "{}",
                    a.line()
                );
                got[w][seed as usize] = a.hash;
            }
        }
        assert_eq!(got, KIT_GOLDEN, "{got:#018x?}");
    }

    fn median<T: Copy + Ord>(mut v: Vec<T>) -> T {
        v.sort_unstable();
        v[v.len() / 2]
    }

    /// PT-TUNE D1 to D5 for `CarlSolo` at seeds 0..8 (`pt-tune-design.md`
    /// section 3). Returns the ids of the targets that miss, and prints the
    /// per-seed table (the PR body's).
    fn carl_solo_misses() -> Vec<&'static str> {
        let reports: Vec<Report> = SEEDS
            .map(|seed| run(&root(), Who::CarlSolo, seed).unwrap())
            .collect();
        for r in &reports {
            println!("{}", r.line());
        }
        let mut misses = Vec::new();
        for (wave, (d1, d2)) in [("W1", ("D1.W1", "D2.W1")), ("W2", ("D1.W2", "D2.W2"))]
            .into_iter()
            .enumerate()
            .map(|(i, (_, ids))| (i, ids))
        {
            let overlap: Vec<u32> = reports
                .iter()
                .map(|r| r.difficulty.waves[wave].overlap_percent())
                .collect();
            // D1 (PTT-Q19): the median share at least 30 % in W1 (the teaching
            // wave) and 40 % in W2, and every seed at least 20 %.
            let want = if wave == 0 { 30 } else { 40 };
            if median(overlap.clone()) < want || overlap.iter().any(|p| *p < 20) {
                misses.push(d1);
            }
            // D2 (PTT-Q18): W1 at least 2 distinct engaged kinds at one tick and
            // at least 3 over the wave (the acid bars the walkers, so W1 is
            // sequential pairs); W2 at least 3 at one tick. Every seed.
            let (peak, seen) = if wave == 0 { (2, 3) } else { (3, 0) };
            if reports.iter().any(|r| {
                let w = &r.difficulty.waves[wave];
                w.peak_kinds < peak || w.kinds_seen() < seen
            }) {
                misses.push(d2);
            }
        }
        // D3 (PTT-Q20): the bot check. Every seed clears, the median clear tick is
        // at least 1.25 times main's 517 on the same pilot (647), every seed at
        // most 4800. The 900..=3000 human band (15 to 50 s) is the owner
        // scorecard's line (OQ4); the bot's figure is printed beside it.
        let cleared: Vec<u64> = reports.iter().filter_map(|r| r.cleared_tick).collect();
        assert_eq!(cleared.len(), reports.len(), "every seed must clear");
        let m = median(cleared.clone());
        println!(
            "D3 bot median clear {m} ticks (main 517, target at least 647; human band 900..3000: {})",
            if (900..=3000).contains(&m) {
                "inside"
            } else {
                "outside"
            }
        );
        if m < MAIN_MEDIAN_CLEAR * 5 / 4 || cleared.iter().any(|t| *t > 4800) {
            misses.push("D3");
        }
        // D4: the hero hurts, median 3..=10, at least one in each wave, deaths at
        // most 2 on every seed.
        let hurts: Vec<usize> = reports.iter().map(|r| r.hero_hurts).collect();
        if !(3..=10).contains(&median(hurts))
            || reports
                .iter()
                .any(|r| r.difficulty.hurts.contains(&0) || r.deaths > 2)
        {
            misses.push("D4");
        }
        // D5: LAB-Q04 holds in play: at most four engaged enemies on any tick, for
        // every pilot (the kit pilots included).
        for who in Who::ALL.into_iter().chain(Who::KITS) {
            for seed in SEEDS {
                let r = run(&root(), who, seed).unwrap();
                if r.difficulty.peak_engaged > 4 {
                    misses.push("D5");
                }
            }
        }
        misses
    }

    /// Main's median `CarlSolo` clear tick before PT-TUNE (517, the discrimination
    /// run of the PR), the base of the D3 bot check.
    const MAIN_MEDIAN_CLEAR: u64 = 517;

    /// The targets this composition misses as measured (PTT-Q10: a target is
    /// never adjusted to fit a result). The test fails both when a target that
    /// passed starts to miss and when a miss starts to pass, so the list is
    /// always the truth. PTT-Q17 to Q20 (review of PR 236) leave it empty.
    const EXPECTED_MISSES: [&str; 0] = [];

    #[test]
    fn the_mastery_waves_meet_the_difficulty_targets() {
        let misses = carl_solo_misses();
        assert_eq!(
            misses, EXPECTED_MISSES,
            "D1 to D5 (D6 is `mastery_enemy_hp_equals_base_kinds`): update the list and the \
             PR body on purpose"
        );
    }

    /// D7 (report-only): the kit pilots' clear rate and clear tick per seed. A
    /// miss goes in the owner packet and does not block the merge.
    #[test]
    fn the_kit_pilots_report_their_clear_rate() {
        for who in Who::KITS {
            let mut cleared = 0;
            for seed in SEEDS {
                let r = run(&root(), who, seed).unwrap();
                println!("D7 {}", r.line());
                cleared += usize::from(r.cleared_tick.is_some());
            }
            println!("D7 {} cleared {cleared} of {}", who.name(), SEEDS.end);
        }
    }

    /// PTT protected-spawn proof: with both heroes idle on tick 0 nothing hurts
    /// either of them for 600 ticks (the slime is dormant, the llamas' leash
    /// floors are out of fire range, the goblins sleep).
    #[test]
    fn both_heroes_idle_at_spawn_take_no_hurt_for_600_ticks() {
        for seed in SEEDS {
            let (mut sim, _) = start(&root(), Who::CarlSolo, seed).unwrap();
            for _ in 0..600 {
                sim.step_slots([RawInput::NONE, RawInput::NONE]);
            }
            let hurts = sim
                .combat
                .events
                .iter()
                .filter(|e| matches!(e, CombatEvent::HeroHurt { .. }))
                .count();
            assert_eq!(hurts, 0, "seed {seed}: {:?}", sim.combat.events);
            assert!(!sim.combat.hero_defeated, "seed {seed}");
        }
    }

    /// The composition runs as designed: the slime is a slot enemy that is
    /// defeated, its pool drains (the patch is gone by the end), the walkers
    /// stand right of the pool until it is gone, and the sticker wakes.
    #[test]
    fn the_hazards_run_in_the_clear_the_slime_dies_and_its_pool_drains() {
        // FA-Q15 off: this composition needs the assistant to walk under the sticker
        // and wake it (the fenced assistant does not).
        crate::combat_scene::with_profile_override(
            |f| f.cf02_assist.fix_all = crate::companion::profile::FixAll::default(),
            || {
                let (mut sim, _) = start(&root(), Who::CarlSolo, 0).unwrap();
                assert_eq!(
                    sim.combat.slot_enemies().len(),
                    1,
                    "the slime is a slot enemy"
                );
                assert!(sim.combat.env().unwrap().has_acid(), "the pool is live");
                let (mut slime_dead_at, mut pool_gone_at) = (None, None);
                let mut walker_in_live_pool = false;
                for _ in 0..RUN_TICKS {
                    let input = arena_pilot(&sim.combat, &sim.scene, HeroId::Carl);
                    sim.step_slots([input, RawInput::NONE]);
                    let t = sim.scene.world.tick;
                    let live = sim
                        .combat
                        .env()
                        .unwrap()
                        .acid()
                        .patches()
                        .any(|p| p.bounds().is_some());
                    if slime_dead_at.is_none()
                        && sim.combat.slot_enemies().iter().all(|e| !e.alive())
                    {
                        slime_dead_at = Some(t);
                    }
                    if live {
                        // Walkers (goblins) never stand inside the live pool's current span.
                        let span = sim
                            .combat
                            .env()
                            .unwrap()
                            .acid()
                            .patches()
                            .find_map(|p| p.bounds());
                        if let Some((x0, x1)) = span {
                            walker_in_live_pool |= sim.combat.enemies.iter().any(|g| {
                                g.alive()
                                    && g.mode == crate::goblin::GoblinMode::Neutral
                                    && g.x > x0 as f32 + 2.0
                                    && g.x < x1 as f32 - 2.0
                            });
                        }
                    } else if pool_gone_at.is_none() && slime_dead_at.is_some() {
                        pool_gone_at = Some(t);
                    }
                    // To the exit, not to the clear: since FA-Q17 the seed-0 fight can end
                    // before the sticker is walked under, and its blast is the walk's.
                    if sim.combat.roster_cleared() && sim.scene.world.hero.x >= EXIT_X {
                        break;
                    }
                }
                // The sticker woke on proximity and its blast went off (the yard's
                // environmental answer and its hazard to a hero who walks under it).
                let rot = |e: &&CombatEvent| matches!(e, CombatEvent::RotProvoked { .. });
                let blast = |e: &&CombatEvent| matches!(e, CombatEvent::Blast { .. });
                assert!(
                    sim.combat.events.iter().any(|e| rot(&e)),
                    "the sticker wakes"
                );
                assert!(
                    sim.combat.events.iter().any(|e| blast(&e)),
                    "the sticker blasts"
                );
                let (dead, gone) = (slime_dead_at.unwrap(), pool_gone_at.unwrap());
                assert!(
                    gone >= dead + 55,
                    "the pool drains over about 60 ticks: {dead} {gone}"
                );
                assert!(!walker_in_live_pool, "walkers stop at the pool (HAZ-Q48)");
            },
        );
    }

    /// CA2 M1 to M16 in the closing arena. The hard targets of the spec hold at
    /// every seed; the two figures that miss their target on some seeds (M2
    /// shadowing and M10's retreat distance) are bounded here, M13's comparison with
    /// the assist partner is only reported, and all three are listed as design
    /// notes in the PR (a miss is a design note, not a silent retune).
    #[test]
    fn the_ca2_metrics_run_in_the_closing_arena() {
        for seed in SEEDS {
            let (m, assist_hits) = metrics(&root(), seed).unwrap();
            println!("{}", metrics_line(seed, &m, assist_hits));
            // M14: the figures replay twice-identical.
            let (again, _) = metrics(&root(), seed).unwrap();
            assert_eq!(m, again, "M14 seed {seed}");
            assert_eq!(m.ticks, METRICS_TICKS);
            // M1: spread at least 40 % with a median of at least 96 px.
            assert!(
                m.spread_percent() >= 40 && m.median_distance_px >= 96.0,
                "M1 seed {seed}"
            );
            // M3: distinct targets at least 70 %; the same target at most 15 %
            // of the ticks.
            assert!(m.distinct_percent() >= 70, "M3 seed {seed}");
            assert!(
                m.same_target_ticks * 100 <= 15 * m.counted_ticks,
                "M3 same seed {seed}"
            );
            // M4 (PT-TUNE: a design note): the spec target is no steal. With the
            // slime and the Rot Sticker in the room the partner steals one kill on
            // most seeds (measured 0 to 1 at every seed); bounded, not asserted
            // to be zero, and listed in the PR for the owner.
            assert!(m.steals + m.start_steals <= 1, "M4 seed {seed}");
            // M5, M6, M12: nothing self-caused leaves the view, no one exits
            // ahead, nothing teleports.
            assert_eq!(m.episodes_self_caused, 0, "M5 seed {seed}");
            assert_eq!((m.outside_ticks, m.ahead_exits), (0, 0), "M6 seed {seed}");
            assert_eq!(m.regroup_teleports, 0, "M12 seed {seed}");
            // M13: at most half the landed hits. (Her hits against the assist
            // partner's on the same inputs miss the target on every seed: reported
            // beside the figure, a design note, not asserted.)
            assert!(m.hit_share_percent() <= 50, "M13 share seed {seed}");
            // M15: at most CA2-Q33's 25 %.
            assert!(m.retreat_share_percent() <= 25, "M15 seed {seed}");
            // M10: at most one hit during a retreat; the distance after 60 ticks
            // is at least 96 px on most seeds (a measured 91 px on two).
            assert!(m.hits_in_retreat <= 1, "M10 hits seed {seed}");
            for d in m.retreat_distances.iter().flatten() {
                assert!(*d >= 90.0, "M10 distance seed {seed}");
            }
            // M2 (target at most 20 %; PT-TUNE: a design note): the partner
            // shadows Carl more in the new room, where he stands at the slime's
            // pool for about 100 ticks. Measured 31 % to 60 % over seeds 0..8; the bound
            // is a regression pin (the measured maximum rounded up to 5 %).
            assert!(m.shadow_percent() <= 65, "M2 seed {seed}");
            // T15: M7, M8, M9 (cause split), M11 and M14, measured on the same run.
            let (x, cost) = arena_extras(&root(), seed).unwrap();
            println!(
                "seed={seed} M7 rest/zoom/width equal at -200/+200: {:?}/{:?}/{:?} M8 outside-view ticks per config {:?} M9 hurts to her {:?} (hazard {}) M11 show-offs {} (enemy within 128 px at the start {}, a tell running {}) M14 step median/max ns: autonomy {}/{}, assist {}/{}",
                x.m7_rest_equal,
                x.m7_zoom_equal,
                x.m7_width_equal,
                x.m8_misses,
                x.m9,
                x.m9.hazard(),
                x.m11_starts,
                x.m11_enemy_near_starts,
                x.m11_tell_starts,
                cost[0].median_ns,
                cost[0].max_ns,
                cost[1].median_ns,
                cost[1].max_ns,
            );
            let (again, _) = arena_extras(&root(), seed).unwrap();
            assert_eq!(x, again, "M14 determinism, seed {seed}");
            // M7: she never moves the camera's rest width or zoom (measured equal
            // under a 200 px shift of her position at every seed).
            assert!(
                x.m7_rest_equal
                    .iter()
                    .chain(&x.m7_zoom_equal)
                    .chain(&x.m7_width_equal)
                    .all(|e| *e),
                "M7 seed {seed}: {x:?}"
            );
            // M8: her body is inside the automatic view on every tick, at both
            // aspects and both output widths.
            assert_eq!(
                x.m8_misses, [0; 4],
                "M8 seed {seed}: first {:?}",
                x.m8_first
            );
            // M9: no hazard hurt (acid, Rot, shore) reaches her.
            assert_eq!(x.m9.hazard(), 0, "M9 seed {seed}: {:?}", x.m9);
            // M11: a show-off hold never starts beside a standing enemy or under
            // a tell.
            assert_eq!(
                (x.m11_enemy_near_starts, x.m11_tell_starts),
                (0, 0),
                "M11 seed {seed}"
            );
        }
    }

    /// The hashes of the pilot runs recorded before FA-Q12, FA-Q14, FA-Q17 and
    /// FA-Q19: `(who, seed, hash)`, three samples of Carl, two of the swapped
    /// Donut, two of the companion pair and one of the kit pilot. Every knob of
    /// the plan off reproduces them (the one-line revert).
    const EARLIER_HASHES: [(Who, u64, u64); 8] = [
        (Who::CarlSolo, 0, 0x8a0f_e792_7530_65f6),
        (Who::CarlSolo, 1, 0xd369_6e2d_8ad6_f931),
        (Who::CarlSolo, 2, 0x3255_f01a_be82_377d),
        (Who::DonutFights, 0, 0x5490_1a93_b6dc_4969),
        (Who::DonutFights, 1, 0x7bed_a1ce_1223_3403),
        (Who::Companion, 0, 0x5d4b_d23c_4866_23ac),
        (Who::Companion, 1, 0x594e_3503_067b_8480),
        (Who::DonutKit, 0, 0x741c_c2a2_6dd8_84c3),
    ];

    /// One run with every knob of this PR off: the pilot rules (FA-Q12, FA-Q17),
    /// the missile spawn sweep (FA-Q14) and the armed attempt start (FA-Q19).
    fn run_earlier(who: Who, seed: u64) -> Report {
        let mut party = tuning(who);
        party.donut_reflex = crate::agility::ReflexProfile::ShippedNoOpener;
        let (mut sim, _) = start_with(&root(), seed, party).unwrap();
        sim.combat.set_missile_spawn_sweep_for_test(false);
        run_sim(&root(), who, seed, RUN_TICKS, PilotRules::EARLIER, sim).unwrap()
    }

    #[test]
    fn every_knob_off_reproduces_the_recorded_hashes() {
        // The PR-A fix-all knobs (FA-Q15, FA-Q25 to FA-Q28) are off with the others.
        crate::combat_scene::with_profile_override(crate::combat_scene::fix_all_off, || {
            for (who, seed, hash) in EARLIER_HASHES {
                let r = run_earlier(who, seed);
                assert_eq!(r.hash, hash, "{}", r.line());
            }
        });
    }

    /// Each knob moves the run on its own: the shipped pilot differs from the
    /// earlier one (so the knobs above are not vacuous).
    #[test]
    fn the_shipped_rules_change_the_runs_the_knobs_guard() {
        let carl = run(&root(), Who::CarlSolo, 2).unwrap();
        assert_ne!(carl.hash, EARLIER_HASHES[2].2);
        let donut = run(&root(), Who::DonutFights, 0).unwrap();
        assert_ne!(donut.hash, EARLIER_HASHES[3].2);
        // The pilot rules alone (Carl's branch reads only the jab phase).
        let (sim, _) = start(&root(), Who::CarlSolo, 2).unwrap();
        let no_phase = run_sim(
            &root(),
            Who::CarlSolo,
            2,
            RUN_TICKS,
            PilotRules {
                jab_phase_by_seed: false,
                ..PilotRules::SHIPPED
            },
            sim,
        )
        .unwrap();
        assert_ne!(no_phase.hash, carl.hash, "the jab phase moves Carl's run");
    }

    /// FA-Q12, FA-Q13 (T18): the base-kit and the kit Donut pilots clear at least
    /// 7 of 9 seeds (the D7 target of PTT-Q21, met: asserted), and no room 6
    /// pilot stalls for more than [`STALL_LIMIT`] ticks.
    #[test]
    fn the_donut_pilots_clear_the_room_and_never_stall() {
        for who in [Who::DonutFights, Who::DonutKit] {
            let reports: Vec<Report> = SEEDS.map(|seed| run(&root(), who, seed).unwrap()).collect();
            let clears = reports.iter().filter(|r| r.cleared_tick.is_some()).count();
            println!("D7 {} cleared {clears} of {}", who.name(), SEEDS.end);
            assert!(clears >= 7, "{}: {clears} of 9", who.name());
        }
        for who in [Who::CarlSolo, Who::DonutFights, Who::DonutKit] {
            for seed in SEEDS {
                let r = run(&root(), who, seed).unwrap();
                println!("stall {}", r.line());
                assert!(r.max_stall_ticks <= STALL_LIMIT, "{}", r.line());
            }
        }
    }

    /// FA-Q13: the stall run counts ticks with no walk and no jump while a wave
    /// entry lives and the controlled hero was hurt in the trailing 300 ticks;
    /// checked at both edges of each condition.
    #[test]
    fn the_stall_run_counts_exactly_its_conditions() {
        let mut s = Stall::default();
        s.hurt(HeroId::Carl, 10);
        // Ticks 11..=310 are inside the window (310 - 10 = 300), 311 is out.
        for tick in 11..=310 {
            s.step(tick, HeroId::Carl, true, false);
        }
        assert_eq!((s.run, s.max), (300, 300));
        s.step(311, HeroId::Carl, true, false);
        assert_eq!(
            (s.run, s.max),
            (0, 300),
            "the 301st tick is out of the window"
        );
        // A hurt to the other hero does not count for this one.
        let mut s = Stall::default();
        s.hurt(HeroId::Donut, 10);
        s.step(11, HeroId::Carl, true, false);
        assert_eq!(s.max, 0);
        s.step(12, HeroId::Donut, true, false);
        assert_eq!(s.max, 1);
        // No live wave entry, or any movement input, ends the run.
        s.step(13, HeroId::Donut, false, false);
        assert_eq!(s.run, 0);
        s.step(14, HeroId::Donut, true, false);
        s.step(15, HeroId::Donut, true, true);
        assert_eq!((s.run, s.max), (0, 1));
        // No hurt at all: nothing counts.
        let mut s = Stall::default();
        s.step(5, HeroId::Carl, true, false);
        assert_eq!(s.max, 0);
    }

    /// FA-Q17: the jab phase varies the fights; carl_solo has at least 5 distinct
    /// outcomes (clear tick, hurts, kill order) over seeds 0..8 (7 measured).
    #[test]
    fn the_arena_seeds_give_at_least_five_distinct_outcomes() {
        let reports: Vec<Report> = SEEDS
            .map(|seed| run(&root(), Who::CarlSolo, seed).unwrap())
            .collect();
        let n = distinct_outcomes(&reports);
        println!("carl_solo distinct outcomes: {n}");
        assert!(n >= 5, "{n}");
        // The count is the number of distinct tuples, not of hashes.
        assert!(n <= reports.len());
    }

    /// T06 (DM-FU-C): the test-only `attempts` counter reads the same number the
    /// event tally gives on a real run (the agreement is asserted in
    /// `agility_reflex_tests`), and never takes part in the reflex's equality.
    #[test]
    fn the_reflex_attempts_counter_is_asserted_on_a_real_run() {
        let (mut sim, _) = start(&root(), Who::DonutFights, 0).unwrap();
        for _ in 0..600 {
            let controlled = sim.party.controlled_hero();
            let mut input = arena_pilot(&sim.combat, &sim.scene, controlled);
            input.swap = controlled == HeroId::Carl;
            sim.step_slots([input, RawInput::NONE]);
        }
        let reflex = *sim.combat.reflex();
        let counted = reflex.attempts_for_test();
        let evadable = sim
            .combat
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::HeroEvaded { .. }
                        | CombatEvent::HeroHurt {
                            hero: HeroId::Donut,
                            ..
                        }
                )
            })
            .count() as u32;
        // Every evade and every evadable hurt reached the check; hazard hurts
        // (acid, Rot) did not, so the counter is at most the log's tally.
        assert!(counted <= evadable, "{counted} against {evadable}");
        let mut other = reflex;
        other.restart();
        assert_eq!(other.attempts_for_test(), counted, "restart keeps it");
    }

    /// FA-Q17 is the closing arena's alone: `pilot` (room 4's Carl and Donut, the
    /// hazards room pilots) never applies the seed-offset jab phase, while
    /// `arena_pilot` does. Checked along a real room 6 run at a seed whose phase is not 0, and the plain `pilot` equals
    /// the rules with the phase off at every tick.
    #[test]
    fn the_seed_phase_is_scoped_to_the_arena() {
        let no_phase = PilotRules {
            jab_phase_by_seed: false,
            ..PilotRules::SHIPPED
        };
        assert_eq!(PilotRules::OTHER_ROOMS, no_phase);
        let (mut sim, _) = start(&root(), Who::CarlSolo, 3).unwrap();
        let mut differs = 0;
        for _ in 0..700 {
            let plain = pilot(&sim.combat, &sim.scene, HeroId::Carl);
            let arena = arena_pilot(&sim.combat, &sim.scene, HeroId::Carl);
            assert_eq!(
                plain,
                pilot_rules(&sim.combat, &sim.scene, HeroId::Carl, false, no_phase),
                "tick {}",
                sim.scene.world.tick
            );
            if plain != arena {
                differs += 1;
            }
            sim.step_slots([arena, RawInput::NONE]);
        }
        assert!(differs > 0, "the arena phase changes the presses");
        // The plain pilot reads no seed: from two states that differ only by the seed
        // its input is the same.
        let (a, _) = start(&root(), Who::CarlSolo, 0).unwrap();
        let (b, _) = start(&root(), Who::CarlSolo, 3).unwrap();
        assert_eq!(
            pilot(&a.combat, &a.scene, HeroId::Carl),
            pilot(&b.combat, &b.scene, HeroId::Carl)
        );
    }
}
