//! Deterministic scripted runs of the six rooms: a small bot turns a behaviour
//! directive per actor into [`LabInput`] every tick as a pure function of the
//! simulation state. The bot never reads the camera or the coverage, so a run
//! reproduces exactly (state hash and event log) at every coverage.

use super::room::{PropKind, Room};
use super::sim::{CARL, DONUT, Foe, FoeMode, Hittable, LabInput, LabSim, Mode, PropMode};
use crate::combat::EnemyAi;
use crate::goblin::GoblinMode;
use crate::prototype::Rect;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Behavior {
    Idle,
    /// Fight the nearest live enemy on this level; with none left, walk to the
    /// exit.
    Hunt,
    /// Walk to this x (layout px).
    GoTo(i32),
    /// Climb onto platform `n` from its side.
    Climb(usize),
    /// Stand next to a destructible prop and hit it until it breaks.
    BreakProp(usize),
    /// Walk to the exit.
    Exit,
}

#[derive(Debug, Clone, Copy)]
pub struct Directive {
    pub from: u64,
    pub actor: usize,
    pub behavior: Behavior,
}

#[derive(Debug, Clone)]
pub struct Script {
    pub room: usize,
    pub name: &'static str,
    pub mode: Mode,
    pub seed: u64,
    pub ticks: u64,
    pub directives: Vec<Directive>,
}

const fn d(from: u64, actor: usize, behavior: Behavior) -> Directive {
    Directive {
        from,
        actor,
        behavior,
    }
}

/// One scripted run per room (2P with both actors scripted), plus a solo run of
/// every room where the player switches once.
pub fn scripts() -> Vec<Script> {
    use Behavior::*;
    let two = Mode::TwoPlayer;
    vec![
        Script {
            room: 0,
            name: "room01_first_rematch",
            mode: two,
            seed: 1,
            ticks: 1500,
            directives: vec![d(0, CARL, Hunt), d(0, DONUT, Climb(0)), d(420, DONUT, Hunt)],
        },
        Script {
            room: 1,
            name: "room02_hunters_from_above",
            mode: two,
            seed: 2,
            ticks: 1500,
            directives: vec![
                d(0, CARL, Hunt),
                d(0, DONUT, Climb(0)),
                d(300, DONUT, Climb(1)),
                d(700, DONUT, Hunt),
            ],
        },
        Script {
            room: 2,
            name: "room03_break_a_breach",
            mode: two,
            seed: 3,
            ticks: 1800,
            directives: vec![
                d(0, CARL, Hunt),
                d(0, DONUT, Climb(0)),
                d(360, DONUT, Hunt),
                d(560, CARL, BreakProp(2)),
            ],
        },
        Script {
            room: 3,
            name: "room04_hoarder_lair",
            mode: two,
            seed: 4,
            ticks: 1800,
            directives: vec![d(0, CARL, Hunt), d(0, DONUT, Climb(0)), d(500, DONUT, Hunt)],
        },
        Script {
            room: 4,
            name: "room05_gym_separation",
            mode: two,
            seed: 5,
            ticks: 2100,
            directives: vec![d(0, CARL, Hunt), d(0, DONUT, Climb(1)), d(900, DONUT, Exit)],
        },
        Script {
            room: 5,
            name: "room06_swine_boss",
            mode: two,
            seed: 6,
            ticks: 2400,
            directives: vec![
                d(0, CARL, GoTo(900)),
                d(0, DONUT, GoTo(1020)),
                d(300, CARL, Hunt),
                d(300, DONUT, Hunt),
            ],
        },
    ]
}

pub fn solo_script(room: usize) -> Script {
    let mut s = scripts().remove(room);
    s.mode = Mode::Solo;
    s.name = "solo";
    // The controlled actor is scripted through slot 0 (Carl's directives); the
    // companion is the built-in AI.
    s.directives.retain(|d| d.actor == CARL);
    s
}

fn behavior_at(script: &Script, actor: usize, tick: u64) -> Behavior {
    script
        .directives
        .iter()
        .rfind(|d| d.actor == actor && d.from <= tick)
        .map_or(Behavior::Idle, |d| d.behavior)
}

/// Inputs of both players for the next tick of a script.
pub fn script_inputs(sim: &LabSim, script: &Script) -> [LabInput; 2] {
    let t = sim.tick;
    let a = bot_input(sim, CARL, behavior_at(script, CARL, t));
    let b = if script.mode == Mode::TwoPlayer {
        bot_input(sim, DONUT, behavior_at(script, DONUT, t))
    } else {
        LabInput::default()
    };
    // Solo: one controlled actor through slot 0; a scripted switch halfway.
    if script.mode == Mode::Solo {
        let mut a = a;
        if t == script.ticks / 3 {
            a.switch = true;
        }
        return [a, b];
    }
    [a, b]
}

fn foe_live(t: &Hittable) -> Option<(f32, f32, f32)> {
    // (x, centre y, half height)
    match t {
        Hittable::Goblin(g) if g.alive() && g.mode != GoblinMode::Corpse => Some((g.x, g.y, 19.0)),
        Hittable::Foe(f) if f.alive() && f.mode != FoeMode::Corpse => Some((f.x, f.y, f.spec.hh)),
        _ => None,
    }
}

fn goal_x(room: &Room) -> f32 {
    room.exit.x as f32 + room.exit.w as f32 / 2.0
}

pub fn bot_input(sim: &LabSim, actor: usize, behavior: Behavior) -> LabInput {
    let me = &sim.actors[actor];
    let mut input = LabInput::default();
    if me.down() {
        return input;
    }
    let my_feet = me.body.feet_y();
    let (stop, reach) = if actor == CARL {
        (26.0, 40.0)
    } else {
        (22.0, 34.0)
    };
    let mut tx = me.body.x;
    let mut attack_target: Option<(f32, f32)> = None;
    match behavior {
        Behavior::Idle => {}
        Behavior::GoTo(x) => tx = x as f32,
        Behavior::Exit => tx = goal_x(&sim.room),
        Behavior::Climb(n) => {
            let plat = sim.room.platforms[n].rect;
            let top = sim.height - plat.y as f32;
            if my_feet >= top - 4.0 {
                tx = plat.x as f32 + plat.w as f32 / 2.0;
            } else {
                // Approach from the nearer side, then hop in.
                let left = plat.x as f32 - 44.0;
                let right = plat.right() as f32 + 44.0;
                let from_left = (me.body.x - left).abs() <= (me.body.x - right).abs();
                if sim.room.steps.iter().any(|s| {
                    s.x as f32 >= plat.x as f32 - 200.0 && (s.right() as f32) <= plat.x as f32
                }) && actor == CARL
                {
                    tx = plat.x as f32 + plat.w as f32 / 2.0;
                } else {
                    tx = if from_left { left + 30.0 } else { right - 30.0 };
                    let near = (me.body.x - tx).abs() < 24.0 && me.body.grounded;
                    if near {
                        input.jump = true;
                        input.move_x = if from_left { 1.0 } else { -1.0 };
                        return input;
                    }
                }
            }
        }
        Behavior::BreakProp(id) => {
            if let Some(Hittable::Prop(p)) = sim.targets.get(sim.room.enemies.len() + id)
                && p.mode == PropMode::Intact
            {
                let (cx, cy) = p.rect.center();
                attack_target = Some((cx, cy));
                let side = if me.body.x < cx { -1.0 } else { 1.0 };
                tx = cx + side * (p.rect.width / 2.0 + reach - 8.0);
            } else {
                return bot_input(sim, actor, Behavior::Hunt);
            }
        }
        Behavior::Hunt => {
            // Nearest live enemy on about this level.
            let mut best: Option<(f32, f32, f32)> = None;
            for t in &sim.targets {
                if let Some((x, y, hh)) = foe_live(t) {
                    let feet = y - hh;
                    if (feet - my_feet).abs() < 70.0 {
                        let d = (x - me.body.x).abs();
                        if best.is_none_or(|b| d < (b.0 - me.body.x).abs()) {
                            best = Some((x, y, hh));
                        }
                    }
                }
            }
            match best {
                Some((x, y, _)) => {
                    attack_target = Some((x, y));
                    tx = x;
                }
                None => tx = goal_x(&sim.room),
            }
        }
    }
    let dx = tx - me.body.x;
    let close = attack_target.is_some();
    let stop_at = if close { reach - 14.0 } else { stop };
    if dx.abs() > stop_at {
        input.move_x = dx.signum();
    }
    // Face the target even when standing still next to it.
    let mut engaged = false;
    if let Some((ax, ay)) = attack_target {
        let ahead = (ax - me.body.x) * f32::from(me.body.facing) > 0.0;
        let dist = (ax - me.body.x).abs();
        let vertical = (ay - me.body.y).abs() < 50.0;
        if dist <= reach + 14.0 && vertical {
            engaged = true;
            if !ahead {
                input.move_x = (ax - me.body.x).signum() * 0.3;
            }
            // Strike rhythm: a press every 10 ticks (jab, then the haymaker as the
            // second press); a kick on every sixth cycle.
            let t = sim.tick;
            if actor == CARL {
                if (t / 60) % 5 == 4 {
                    input.physical = t % 60 == 2;
                } else {
                    input.primary = t.is_multiple_of(10);
                }
            } else {
                input.primary = t.is_multiple_of(14);
            }
        }
    }
    // A wall of an intact breakable prop in the way is struck, not jumped.
    if input.move_x != 0.0 && !engaged {
        let probe = Rect {
            left: me.body.x + input.move_x.signum() * (me.body.hw + 14.0) - 8.0,
            bottom: me.body.y - me.body.hh + 3.0,
            width: 16.0,
            height: 6.0,
        };
        let enemies = sim.room.enemies.len();
        let wall = sim.targets[enemies..].iter().find_map(|t| match t {
            Hittable::Prop(p)
                if p.def.breakable
                    && p.mode == PropMode::Intact
                    && p.rect.overlaps(probe)
                    && p.rect.height > 80.0 =>
            {
                Some(p.rect)
            }
            _ => None,
        });
        if let Some(rect) = wall
            && (actor == CARL || rect.height < 60.0)
        {
            let t = sim.tick;
            input.move_x = input.move_x.signum() * 0.3;
            if actor == CARL {
                input.primary = t.is_multiple_of(10);
            } else {
                input.primary = t.is_multiple_of(14);
            }
            return input;
        }
    }
    // Hop what blocks the way.
    if input.move_x != 0.0 && me.body.grounded {
        let probe = Rect {
            left: me.body.x + input.move_x.signum() * (me.body.hw + 5.0) - 2.0,
            bottom: me.body.y - me.body.hh + 3.0,
            width: 4.0,
            height: 6.0,
        };
        if sim
            .blocks()
            .iter()
            .any(|b| b.intact && probe.overlaps(b.rect))
        {
            input.jump = true;
        }
    }
    input
}

/// A finished scripted run.
#[derive(Debug, Clone)]
pub struct Trace {
    pub final_hash: u64,
    pub state_line: String,
    pub events: Vec<String>,
    pub tick_hashes: Vec<u64>,
    pub alive_foes: usize,
    pub broken_props: usize,
    pub exits: [bool; 2],
    pub downs: [u32; 2],
}

/// Run a script headlessly for its tick count.
pub fn run(room: &Room, script: &Script) -> Trace {
    let mut sim = LabSim::new(room, script.mode, script.seed);
    let mut hashes = Vec::new();
    for _ in 0..script.ticks {
        let inputs = script_inputs(&sim, script);
        sim.step(inputs);
        if sim.tick.is_multiple_of(60) {
            hashes.push(sim.state_hash());
        }
    }
    Trace {
        final_hash: sim.state_hash(),
        state_line: sim.state_line(),
        events: sim.events.iter().map(|e| e.to_string()).collect(),
        tick_hashes: hashes,
        alive_foes: sim.alive_foes(),
        broken_props: sim.broken_props(),
        exits: [
            sim.actors[CARL].exit_reached,
            sim.actors[DONUT].exit_reached,
        ],
        downs: [sim.actors[CARL].downs, sim.actors[DONUT].downs],
    }
}

// Silence unused warnings for the helper types the binary also uses.
#[allow(dead_code)]
fn _types(_: &Foe, _: EnemyAi, _: PropKind) {}
