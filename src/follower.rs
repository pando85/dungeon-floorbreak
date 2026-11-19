//! Q9: the uncontrolled hero's follow-and-jump bot (adopted, game-feel
//! pending). Bevy-free, deterministic and RNG-free: a pure function of the two
//! heroes, the terrain and the goblins' tells.
//!
//! The bot steers to a spot [`PartyTuning::follow_distance_px`] behind the
//! leader on the side it already stands on, using its own run and jump (it
//! never replays the leader's path, so a hero that cannot make a jump simply
//! waits at it). It jumps when the leader stands on a ledge it can reach, and
//! it dodges a goblin tell that starts within
//! [`PartyTuning::dodge_radius_px`] with a jump (D1 mitigation M3). It never
//! attacks: only movement and jump are ever set.
//!
//! The off-screen regroup rule lives with the camera in
//! [`crate::corridor::SceneSim`].

use crate::boss::BossThreat;
use crate::input::{InputFrame, RawInput};
use crate::party::HeroId;
use crate::party::PartyTuning;
use crate::prototype::{Block, GRAVITY, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, Hero, JUMP_SPEED, Rect};

/// Distance band of the "behind the leader" spot: closer than this the bot
/// stands still (no jitter), farther than `follow_distance_px + FOLLOW_SLACK_PX`
/// it closes in.
const FOLLOW_MIN_PX: f32 = 32.0;
const FOLLOW_SLACK_PX: f32 = 8.0;
/// The tallest obstacle (above the bot's feet) its jump clears: the apex of
/// `JUMP_SPEED` under `GRAVITY` (about 92 px) less a safety margin.
const JUMP_REACH_PX: f32 = JUMP_SPEED * JUMP_SPEED / (2.0 * GRAVITY) - 4.0;
/// A tell is dodged only while the swing is still far enough away for the
/// jump to be above the 30 px swing box on its active ticks (26..30): a jump
/// started on attack tick 0..=20 is above it on every one of them.
const DODGE_LATEST_TELL_TICK: u16 = 20;
/// The follow goal's leader height difference that makes the bot hop onto a
/// ledge the leader stands on.
const LEDGE_STEP_PX: f32 = 8.0;
/// Horizontal reach of the "leader is on a ledge above me" hop.
const LEDGE_NEAR_PX: f32 = 64.0;
/// The bot steps out of a pound tell aimed at it within this many px.
const POUND_STEP_OUT_PX: f32 = 84.0;

/// A goblin attack in progress, as the bot sees it (a tell or swing).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Threat {
    pub x: f32,
    /// Ticks since the tell started.
    pub attack_tick: u16,
}

/// The follower's input for one tick. `frame` is the bot hero's own input
/// frame: a held jump never repeats, so the bot releases it first.
pub fn follower_input(
    me: &Hero,
    frame: &InputFrame,
    leader: &Hero,
    solids: &[Block],
    threats: &[Threat],
    tuning: &PartyTuning,
) -> RawInput {
    let mut raw = RawInput::default();
    let foot_y = me.y - HERO_HALF_HEIGHT;

    // Steer: stay on this side of the leader, FOLLOW_MIN..=follow+slack away.
    let gap = leader.x - me.x;
    let distance = gap.abs();
    let upper = f32::from(tuning.follow_distance_px) + FOLLOW_SLACK_PX;
    if distance > upper {
        raw.move_x = gap.signum();
    }
    debug_assert!(FOLLOW_MIN_PX <= f32::from(tuning.follow_distance_px));

    // Obstacle in the way: jump it if the jump reaches, otherwise wait.
    if raw.move_x != 0.0
        && let Some(top) = obstacle_top(me, raw.move_x, solids)
    {
        if top - foot_y <= JUMP_REACH_PX {
            raw.jump = me.grounded;
        } else {
            raw.move_x = 0.0;
        }
    }

    // The leader stands on a ledge above: hop up when close enough.
    let leader_foot = leader.y - HERO_HALF_HEIGHT;
    if me.grounded
        && leader.grounded
        && leader_foot > foot_y + LEDGE_STEP_PX
        && leader_foot - foot_y <= JUMP_REACH_PX
        && distance <= LEDGE_NEAR_PX
    {
        raw.jump = true;
    }

    // M3: a tell starting close by: jump so the swing passes underneath.
    let dodge = f32::from(tuning.dodge_radius_px);
    if me.grounded
        && threats
            .iter()
            .any(|t| t.attack_tick <= DODGE_LATEST_TELL_TICK && (t.x - me.x).abs() <= dodge)
    {
        raw.jump = true;
    }

    if frame.held.jump {
        raw.jump = false;
    }
    raw
}

/// The top (y-up) of the highest intact solid directly ahead of the hero's
/// leading edge, probed like the scripted walk bot: a 6 px wide, 8 px tall
/// window one pixel above the feet.
/// [`follower_input`] plus the boss dodges (HUD-DEBT-03, D1 safeguard M3): the
/// bot steps out of a pound tell aimed at it within 72 px, and jumps when a
/// running charge's ram box will reach it within `lead_ticks`. It never
/// attacks and never triggers a rack. `me_id` is the hero the bot drives.
pub fn follower_input_boss(
    (me_id, me): (HeroId, &Hero),
    frame: &InputFrame,
    leader: &Hero,
    solids: &[Block],
    threats: &[Threat],
    boss: Option<&BossThreat>,
    tuning: &PartyTuning,
) -> RawInput {
    let mut raw = follower_input(me, frame, leader, solids, threats, tuning);
    let Some(boss) = boss else {
        return raw;
    };
    match *boss {
        BossThreat::Pound { x, facing, target } => {
            let gap = me.x - x;
            let ahead = gap * f32::from(facing);
            if target.is_none_or(|t| t == me_id) && ahead > 0.0 && gap.abs() <= POUND_STEP_OUT_PX {
                raw.move_x = if gap >= 0.0 { 1.0 } else { -1.0 };
                raw.jump = false;
            }
        }
        BossThreat::Charge {
            x,
            dir,
            running,
            front,
            depth,
            speed,
            lead_ticks,
        } => {
            if running && me.grounded && !frame.held.jump {
                let lead = x + f32::from(dir) * front;
                let gap = (me.x - lead) * f32::from(dir) - HERO_HALF_WIDTH;
                let in_path = gap > -(depth + 2.0 * HERO_HALF_WIDTH);
                let arrival = gap.max(0.0) / speed;
                if in_path && arrival <= f32::from(lead_ticks) {
                    raw.jump = true;
                }
            }
        }
    }
    raw
}

fn obstacle_top(me: &Hero, dir: f32, solids: &[Block]) -> Option<f32> {
    let foot_y = me.y - HERO_HALF_HEIGHT;
    let left = if dir > 0.0 {
        me.x + HERO_HALF_WIDTH
    } else {
        me.x - HERO_HALF_WIDTH - 6.0
    };
    let probe = Rect {
        left,
        bottom: foot_y + 1.0,
        width: 6.0,
        height: 8.0,
    };
    solids
        .iter()
        .filter(|b| b.intact && probe.overlaps(b.rect))
        .map(|b| b.rect.top())
        .max_by(f32::total_cmp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::party::PROVISIONAL_TUNING;
    use crate::prototype::Material;

    fn wall(left: f32, height: f32) -> Block {
        Block {
            rect: Rect {
                left,
                bottom: 0.0,
                width: 16.0,
                height,
            },
            material: Material::Stone,
            intact: true,
        }
    }

    fn floor() -> Block {
        Block {
            rect: Rect {
                left: -1000.0,
                bottom: -40.0,
                width: 4000.0,
                height: 40.0,
            },
            material: Material::Stone,
            intact: true,
        }
    }

    fn hero(x: f32) -> Hero {
        Hero::at(x, HERO_HALF_HEIGHT)
    }

    fn input(me: &Hero, leader: &Hero, solids: &[Block], threats: &[Threat]) -> RawInput {
        follower_input(
            me,
            &InputFrame::default(),
            leader,
            solids,
            threats,
            &PROVISIONAL_TUNING,
        )
    }

    #[test]
    fn it_closes_in_beyond_the_band_and_stands_still_inside_it() {
        let solids = [floor()];
        let leader = hero(300.0);
        assert_eq!(input(&hero(100.0), &leader, &solids, &[]).move_x, 1.0);
        assert_eq!(input(&hero(500.0), &leader, &solids, &[]).move_x, -1.0);
        for x in [252.0, 260.0, 300.0, 340.0, 348.0] {
            assert_eq!(input(&hero(x), &leader, &solids, &[]).move_x, 0.0, "{x}");
        }
    }

    #[test]
    fn it_never_attacks_and_never_sets_special_inputs() {
        let solids = [floor()];
        let raw = input(&hero(0.0), &hero(400.0), &solids, &[]);
        assert!(!raw.attack && !raw.physical && !raw.ability && !raw.swap && !raw.pause);
    }

    #[test]
    fn it_jumps_a_step_its_jump_reaches_and_waits_at_a_wall_it_cannot_clear() {
        let me = hero(100.0);
        let leader = hero(400.0);
        let low = [floor(), wall(me.x + 12.0, 16.0)];
        let raw = input(&me, &leader, &low, &[]);
        assert!(raw.jump && raw.move_x == 1.0, "a 16 px step is jumped");
        let high = [floor(), wall(me.x + 12.0, 120.0)];
        let raw = input(&me, &leader, &high, &[]);
        assert!(!raw.jump && raw.move_x == 0.0, "a 120 px wall is waited at");
    }

    #[test]
    fn it_hops_onto_a_reachable_ledge_the_leader_stands_on() {
        let mut leader = hero(120.0);
        leader.y += 32.0;
        let raw = input(&hero(100.0), &leader, &[floor()], &[]);
        assert!(raw.jump);
        leader.y += 100.0;
        let raw = input(&hero(100.0), &leader, &[floor()], &[]);
        assert!(!raw.jump, "a ledge beyond its jump is not attempted");
    }

    #[test]
    fn it_dodges_an_early_tell_nearby_and_ignores_far_or_late_ones() {
        let solids = [floor()];
        let me = hero(300.0);
        let leader = hero(310.0);
        let near = Threat {
            x: 320.0,
            attack_tick: 2,
        };
        assert!(input(&me, &leader, &solids, &[near]).jump);
        let far = Threat {
            x: 400.0,
            attack_tick: 2,
        };
        assert!(!input(&me, &leader, &solids, &[far]).jump);
        let late = Threat {
            x: 320.0,
            attack_tick: 22,
        };
        assert!(!input(&me, &leader, &solids, &[late]).jump);
    }

    #[test]
    fn a_held_jump_is_released_first_and_an_airborne_bot_does_not_jump() {
        let solids = [floor()];
        let near = Threat {
            x: 300.0,
            attack_tick: 1,
        };
        let mut frame = InputFrame::default();
        frame.advance(RawInput {
            jump: true,
            ..RawInput::default()
        });
        let me = hero(300.0);
        let leader = hero(300.0);
        let raw = follower_input(&me, &frame, &leader, &solids, &[near], &PROVISIONAL_TUNING);
        assert!(!raw.jump);
        let mut air = hero(300.0);
        air.grounded = false;
        assert!(!input(&air, &leader, &solids, &[near]).jump);
    }
}
