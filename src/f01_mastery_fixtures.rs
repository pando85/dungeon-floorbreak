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
/// The seeds every fixture covers (`--cf02-seed`, plan section 10, item 5).
pub const SEEDS: std::ops::Range<u64> = 0..9;
/// The run length of every pilot (100 s of game time).
pub const RUN_TICKS: u64 = 6000;
/// The tick on which [`Who::DonutBySwap`] swaps to Donut (PTM-AD8).
pub const SWAP_TICK: u64 = 90;
/// The companion is counted left behind only while this far from Carl.
pub const APART_PX: f32 = 96.0;
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

/// [`pilot`], optionally pressing the held ability (D7: Carl's dynamite or
/// Donut's blink) on the `ability` press edge, the input the progression
/// fixtures use.
pub fn pilot_with(combat: &CombatState, scene: &SceneSim, hero: HeroId, kit: bool) -> RawInput {
    let body = scene.world.body(hero);
    let tick = scene.world.tick + 1;
    let press = tick.is_multiple_of(PRESS_CADENCE);
    let mut raw = RawInput::default();
    if combat.hero_defeated {
        return raw;
    }
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
    // in the lane and blocks the walk.
    if let Some(l) = combat
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
    pub deaths: u32,
    pub hero_hurts: usize,
    pub hash: u64,
    pub ticks: u64,
    pub events: Vec<CombatEvent>,
    /// The headless difficulty measure of the run (PT-TUNE, D1 to D5).
    pub difficulty: Difficulty,
}

impl Report {
    /// The one line a table prints.
    pub fn line(&self) -> String {
        format!(
            "{} seed={} cleared={:?} exit={:?} stuck={:?} deaths={} hurts={} {} hash={:#018x}",
            self.who.name(),
            self.seed,
            self.cleared_tick,
            self.exit_tick,
            self.max_stuck,
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
}

impl WaveDifficulty {
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
                "{}%/{}t/{}k",
                d.overlap_percent(),
                d.active_ticks,
                d.peak_kinds
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
        for beat in 0..2 {
            let here: Vec<Threat> = engaged
                .iter()
                .filter(|(b, _)| *b == beat)
                .map(|(_, k)| *k)
                .collect();
            if !here.is_empty() {
                self.started[beat] = true;
            }
            if self.started[beat] && alive[beat] == 0 && beat == 0 && self.w1_done.is_none() {
                self.w1_done = Some(scene.world.tick);
            }
            if !self.started[beat] || alive[beat] == 0 {
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
    let (carl, donut) = Loadout::Story.flags();
    let (run, scene, combat) = mastery_start(root, 360, carl, donut, Some(seed))?;
    let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        base: run.fixture,
        two_player: false,
        tuning: tuning(who),
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
    let (mut sim, _fixture) = start(root, who, seed)?;
    let mut meter = Meter::new(root)?;
    let kit = matches!(who, Who::CarlKit | Who::DonutKit);
    let mut last_x = [sim.scene.world.hero.x, sim.scene.world.donut_body.x];
    let mut stuck = [0u32; 2];
    let mut max_stuck = [0u32; 2];
    let mut cleared_tick = None;
    let mut exit_tick = None;
    let mut deaths = 0u32;
    let mut last_restarts = sim.combat.restarts;
    let mut swap_pressed = false;
    for _ in 0..ticks {
        let tick = sim.scene.world.tick + 1;
        let controlled = sim.party.controlled_hero();
        let (slots, walking) = match who {
            Who::CarlSolo | Who::CarlKit => {
                let input = pilot_with(&sim.combat, &sim.scene, HeroId::Carl, kit);
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
                let mut input = pilot_with(&sim.combat, &sim.scene, controlled, kit);
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
                let input = pilot(&sim.combat, &sim.scene, HeroId::Carl);
                ([input, RawInput::NONE], [input.move_x != 0.0, false])
            }
        };
        sim.step_slots(slots);
        meter.observe(&sim.combat, &sim.scene);
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
        if sim.combat.restarts != last_restarts {
            deaths += 1;
            last_restarts = sim.combat.restarts;
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
        deaths,
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
        let input = pilot(&sim.combat, &sim.scene, HeroId::Carl);
        sim.step_slots([input, RawInput::NONE]);
    };
    let (mut free, _) = start(root, Who::Companion, seed)?;
    let m = crate::cf02_free_fixtures::autonomy_metrics_with(&mut free, METRICS_TICKS, step);
    // M13's baseline: the same inputs under the assist partner.
    let (mut assist, _) = start(root, Who::CarlSolo, seed)?;
    let baseline =
        crate::cf02_free_fixtures::autonomy_metrics_with(&mut assist, METRICS_TICKS, step);
    Ok((m, baseline.her_hits))
}

/// One printable line of the figures (M1 to M16 as the spec words them).
pub fn metrics_line(
    seed: u64,
    m: &crate::cf02_free_fixtures::AutonomyMetrics,
    assist_hits: u32,
) -> String {
    format!(
        "seed={seed} M1={}%/{:.0}px M2={}% M3={}%/{}t M4={}/{} M5={}+{}inv M6={}/{}ahead \
         M10={}x{:?}/{}h M12={} M13={}hits({}%) vs assist {assist_hits} M15={}% \
         M16={}/{}/{}",
        m.spread_percent(),
        m.median_distance_px,
        m.shadow_percent(),
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
        m.her_hits,
        m.hit_share_percent(),
        m.retreat_share_percent(),
        m.interrupt_drift_skip_ticks,
        m.goblins_targeted_her,
        m.hunt_drift_rejected,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// The final state hash of every pilot run, `[who][seed]`, in [`Who::ALL`]
    /// and [`SEEDS`] order. A change to the room, the sidecar, the pilot or the
    /// rules is a deliberate re-pin.
    const GOLDEN: [[u64; 9]; 4] = [
        [
            0x7ec5_aa72_22e5_e86f,
            0xa5f4_840f_1b9f_6654,
            0x677b_7b25_d8e6_922d,
            0x8373_01c3_b1dd_e18e,
            0xc6bc_df2a_9a39_021b,
            0xc615_8010_7de3_0a02,
            0xca6d_f5b7_43f2_ba72,
            0x1de2_0f99_66a2_4ada,
            0xb5a4_ae42_5a75_6945,
        ],
        [
            0x7ec5_aa72_22e5_e86f,
            0x7e92_1d4b_fcf4_d14e,
            0x57f4_ae5f_f3ca_53a3,
            0x06e6_1bdd_9c14_12a6,
            0xd4b8_86d4_daf2_f2d1,
            0x8b81_147b_cde5_c630,
            0x956f_a293_a55a_a292,
            0x47eb_b757_1c20_0bf3,
            0xab93_ddb6_e889_b06a,
        ],
        [
            0x01ed_4e0c_805e_0d36,
            0xf3f0_35b8_6a56_e3c6,
            0x17f1_ad1f_86b7_c3ba,
            0xc0f5_77aa_3911_e357,
            0x19b4_6d5d_e020_3cbb,
            0x75c4_1240_a4cb_4e05,
            0xfd4a_60be_e837_0593,
            0x31ed_3aa0_9745_30a6,
            0xb5eb_6b5c_f0d0_8903,
        ],
        [
            0x0515_7f13_db26_b92a,
            0xcda4_b704_7085_71a9,
            0x52df_dc14_9a8f_6140,
            0x15d2_ae4b_eb0f_07d0,
            0xeb93_b45d_e513_53c0,
            0xae6d_a93a_2561_3d82,
            0x8dfd_07c5_5401_2174,
            0x86b1_0140_92dd_f2dd,
            0x7cee_12de_28dc_80f2,
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
                let input = pilot(&sim.combat, &sim.scene, HeroId::Carl);
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
                let mut input = pilot(&sim.combat, &sim.scene, controlled);
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
    /// it is deterministic and never stuck, and it is **not** claimed to clear
    /// the room (the pilot presses the base kit only; her kit is PT-TUNE's).
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
            0xfd0d_6840_bb20_b919,
            0x98a0_ae01_7ffb_87aa,
            0xf3ab_fc01_81ce_11ac,
            0x0850_6b25_549d_a530,
            0xe0c4_1748_655b_296c,
            0x98e4_d63e_09ac_d50c,
            0x8442_e560_d21a_0b12,
            0x1f4a_70cd_dcb9_4270,
            0x82e2_335c_a2da_2e69,
        ],
        [
            0x2d84_fbec_9952_5d82,
            0xe7e9_c4d5_2e4d_f6ba,
            0x8db2_5e0b_f11c_1e23,
            0x68c2_e20b_5642_0e12,
            0x5672_70fc_a466_d65e,
            0xc52a_3c4d_da7b_96a8,
            0x90c0_4556_a1ea_b870,
            0x1f85_0da1_a409_2eb4,
            0xfe49_73c9_1ed6_7cc4,
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
            // D1: the median share at least 40 %, and every seed at least 20 %.
            if median(overlap.clone()) < 40 || overlap.iter().any(|p| *p < 20) {
                misses.push(d1);
            }
            // D2: at least three distinct engaged kinds on one tick, every seed.
            if reports
                .iter()
                .any(|r| r.difficulty.waves[wave].peak_kinds < 3)
            {
                misses.push(d2);
            }
        }
        // D3: every pilot clears; the median clear tick in 900..=3000, every seed
        // at most 4800.
        let cleared: Vec<u64> = reports.iter().filter_map(|r| r.cleared_tick).collect();
        assert_eq!(cleared.len(), reports.len(), "every seed must clear");
        let m = median(cleared.clone());
        if !(900..=3000).contains(&m) || cleared.iter().any(|t| *t > 4800) {
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

    /// The targets this composition misses as measured (PTT-Q10: after two
    /// composition iterations the numbers ship as measured and the misses are
    /// listed in the PR body, a target is never adjusted to fit a result). The
    /// test fails both when a target that passed starts to miss and when a miss
    /// starts to pass, so the list is always the truth.
    const EXPECTED_MISSES: [&str; 3] = ["D1.W1", "D2.W1", "D3"];

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
    /// stand left of the pool until it is gone, and the sticker wakes.
    #[test]
    fn the_hazards_run_in_the_clear_the_slime_dies_and_its_pool_drains() {
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
            let input = pilot(&sim.combat, &sim.scene, HeroId::Carl);
            sim.step_slots([input, RawInput::NONE]);
            let t = sim.scene.world.tick;
            let live = sim
                .combat
                .env()
                .unwrap()
                .acid()
                .patches()
                .any(|p| p.bounds().is_some());
            if slime_dead_at.is_none() && sim.combat.slot_enemies().iter().all(|e| !e.alive()) {
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
            if sim.combat.roster_cleared() {
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
            // pool for about 100 ticks (measured 18 % to 65 %); bounded.
            assert!(m.shadow_percent() <= 70, "M2 seed {seed}");
        }
    }
}
