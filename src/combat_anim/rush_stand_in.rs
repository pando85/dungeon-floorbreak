//! CORRAL-E (CRL-Q38): the code-drawn stand-in of the corral rush, until the
//! LLAMA-03 art exists. The llama keeps its LLAMA-01 idle frames through the
//! whole rush (CORRAL-B wire: a rush **never** shows the throat glow or the spit
//! swing, see [`super::llama_pose`]); this module adds what tells the rush apart
//! from an idle llama, read from the rush state alone:
//!
//! * **brace** (the tell): the body leans back from the rush direction and the
//!   hind hoof scrapes the floor, a scrape streak with two flecks of dirt and a
//!   small dust puff kicked back on every scrape;
//! * **charge** (the committed rush): the body leans into the rush and a trail of
//!   dust puffs rises behind the hind hoof;
//! * **stop** (recover): the trail settles and shrinks at the stopping point;
//! * **bonk** (a solid or the partition): the body rocks back and a dust burst
//!   stands at the front of the box.
//!
//! **Presentation only, no new art.** Nothing here mutates a state, draws a random
//! number or is hashed, replayed or read back by the simulation: every rectangle
//! is a pure function of one llama's finished tick, so a frozen sim clock holds the
//! stand-in for free. A stand-in is labelled as one in the evidence and is replaced
//! by LLAMA-03. Coordinates are world px, y up; a rectangle gives its bottom-left
//! corner.

use crate::llama::{LLAMA_HALF_WIDTH, Llama, LlamaMode};
use crate::llama_rush::RushPhase;

/// The three tones of the stand-in: the floor scrape, the dirt flecks and the
/// dust (the hoarder room's dust and the kit's floor shadow family, no new hue).
pub const PALETTE: [[u8; 4]; 3] = [
    // Scrape: a dark floor gouge.
    [46, 34, 30, 220],
    // Fleck: kicked dirt.
    [120, 92, 66, 255],
    // Dust: the pale puff.
    [242, 209, 158, 210],
];

/// How far the body leans in px: back during the brace, into the charge.
pub const BRACE_LEAN_PX: i32 = 2;
pub const CHARGE_LEAN_PX: i32 = 2;
/// The bonk rocks the body back this far for [`BONK_ROCK_TICKS`].
pub const BONK_ROCK_PX: i32 = 3;
pub const BONK_ROCK_TICKS: u16 = 8;
/// The hind hoof scrapes once per this many tell ticks, for half of them.
pub const SCRAPE_PERIOD: u16 = 8;
/// The hind hoof sits this far behind the pivot (the body's back third).
pub const HIND_HOOF_PX: i32 = 14;
/// Puffs in the charge trail (one per 4 active ticks).
pub const TRAIL_PUFFS: u16 = 6;
/// The settling trail and the bonk burst last this many ticks.
pub const SETTLE_TICKS: u16 = 12;

/// The tone of a stand-in rectangle (an index into [`PALETTE`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Scrape,
    Fleck,
    Dust,
}

impl Tone {
    pub const fn index(self) -> usize {
        match self {
            Self::Scrape => 0,
            Self::Fleck => 1,
            Self::Dust => 2,
        }
    }
}

/// One code-drawn rectangle (world px, y up, bottom-left corner).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub tone: Tone,
}

/// What the stand-in draws for one llama this tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RushStandIn {
    /// The phase it was drawn for.
    pub phase: RushPhase,
    /// Added to the llama sprite's x (world px): the brace and the bonk lean back,
    /// the charge leans in.
    pub lean_px: i32,
    pub pieces: Vec<Piece>,
}

/// The most pieces one llama draws on one tick (the pool size per llama).
pub const MAX_PIECES: usize = 2 * TRAIL_PUFFS as usize + 4;

/// The stand-in of `llama`, standing on `foot_y` (world px, y up), or `None` when
/// the llama is not in a rush (every legacy llama, every spit, every non-attack
/// mode).
pub fn rush_stand_in(llama: &Llama, foot_y: f32) -> Option<RushStandIn> {
    if llama.mode != LlamaMode::Attack {
        return None;
    }
    let rush = llama.rush_state()?;
    let dir = i32::from(rush.dir.signum());
    let x = llama.x.round() as i32;
    let foot = foot_y.round() as i32;
    let hind = x - dir * HIND_HOOF_PX;
    let front = x + dir * LLAMA_HALF_WIDTH.round() as i32;
    let mut pieces = Vec::new();
    let lean_px = match rush.phase {
        RushPhase::Tell => {
            // The brace settles in over the first 4 ticks, then holds.
            let lean = if rush.tick < 4 { 1 } else { BRACE_LEAN_PX };
            let beat = rush.tick % SCRAPE_PERIOD;
            if rush.tick >= 4 && beat < SCRAPE_PERIOD / 2 {
                let reach = i32::from(beat) + 2;
                // The gouge grows backward under the hind hoof.
                let (left, w) = span(hind, -dir, reach + 2);
                pieces.push(Piece {
                    x: left,
                    y: foot,
                    w,
                    h: 1,
                    tone: Tone::Scrape,
                });
                // Two flecks kicked back and up, higher as the scrape runs.
                for (k, rise) in [(1, 1), (2, 3)] {
                    pieces.push(Piece {
                        x: hind - dir * (reach + 2 * k),
                        y: foot + rise + i32::from(beat) / 2,
                        w: 1,
                        h: 1,
                        tone: Tone::Fleck,
                    });
                }
                // A small pale puff behind the flecks: the dark gouge and the
                // flecks sink into the lit floor at 1x (the first window
                // capture), the puff carries the scrape's beat.
                let (left, w) = span(hind - dir * (reach + 4), -dir, 2);
                pieces.push(Piece {
                    x: left,
                    y: foot + 1 + i32::from(beat) / 2,
                    w,
                    h: 2,
                    tone: Tone::Dust,
                });
            }
            -dir * lean
        }
        RushPhase::Committed => {
            trail(&mut pieces, hind, foot, dir, rush.active, 0);
            dir * CHARGE_LEAN_PX
        }
        RushPhase::Recover => {
            let age = settle_age(llama, rush.left);
            if age < SETTLE_TICKS {
                trail(&mut pieces, hind, foot, dir, TRAIL_PUFFS * 4, age);
            }
            0
        }
        RushPhase::Bonk => {
            let age = settle_age(llama, rush.left);
            if age < SETTLE_TICKS {
                // The burst stands at the front of the box and spreads.
                let size = 2 + i32::from(age.min(6)) / 2;
                for (dx, dy) in [(0, 0), (-3, 4), (3, 3), (-1, 8)] {
                    pieces.push(Piece {
                        x: front + dir * dx - size / 2,
                        y: foot + dy + i32::from(age) / 3,
                        w: size,
                        h: size,
                        tone: Tone::Dust,
                    });
                }
            }
            if age < BONK_ROCK_TICKS {
                -dir * BONK_ROCK_PX
            } else {
                0
            }
        }
    };
    debug_assert!(pieces.len() <= MAX_PIECES);
    Some(RushStandIn {
        phase: rush.phase,
        lean_px,
        pieces,
    })
}

/// Ticks since the recover or the bonk began: its length less what is left.
fn settle_age(llama: &Llama, left: u16) -> u16 {
    let total = match llama.rush_state().map(|r| r.phase) {
        Some(RushPhase::Bonk) => llama
            .corral
            .as_ref()
            .map_or(0, |c| c.profile.crlq10_bonk_ticks),
        _ => llama
            .corral
            .as_ref()
            .map_or(0, |c| c.profile.crlq09_recover_ticks),
    };
    total.saturating_sub(left)
}

/// The charge trail: one puff per 4 active ticks, oldest farthest behind the hind
/// hoof, each rising and growing with its age; `settle` shrinks and lowers the
/// whole trail after the stop.
fn trail(pieces: &mut Vec<Piece>, hind: i32, foot: i32, dir: i32, active: u16, settle: u16) {
    let puffs = (active / 4 + 1).min(TRAIL_PUFFS);
    for p in 0..puffs {
        let age = i32::from(p);
        let size = (2 + age / 2 - i32::from(settle) / 4).max(1);
        let rise = (age + 1) / 2;
        let x = hind - dir * (4 + 6 * age);
        pieces.push(Piece {
            x: x - size / 2,
            y: foot + rise,
            w: size,
            h: size,
            tone: Tone::Dust,
        });
        // A fleck under every other puff, kicked from the hoof.
        if p % 2 == 0 && settle == 0 {
            pieces.push(Piece {
                x: x + dir,
                y: foot,
                w: 1,
                h: 1,
                tone: Tone::Fleck,
            });
        }
    }
}

/// The span of `len` px starting at `from` and running toward `dir`: its left
/// edge and width.
fn span(from: i32, dir: i32, len: i32) -> (i32, i32) {
    if dir < 0 {
        (from - len, len)
    } else {
        (from, len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::EnemyAi;
    use crate::llama::Corral;
    use crate::llama_rush::{self, Activation, Leash, RushState};
    use crate::party::HeroId;

    const X: f32 = 1488.0;
    const FOOT: f32 = 92.0;

    fn llama_in(dir: i8, phase: RushPhase, tick: u16, active: u16, left: u16) -> Llama {
        let mut llama = Llama::new(X, FOOT + crate::llama::LLAMA_HALF_HEIGHT, EnemyAi::Chase);
        llama.mode = LlamaMode::Attack;
        llama.facing = dir;
        let mut state = RushState::start(X, dir, HeroId::Carl, 1);
        state.phase = phase;
        state.tick = tick;
        state.active = active;
        state.left = left;
        llama.corral = Some(Corral {
            profile: llama_rush::active(),
            leash: Leash {
                min: 1248.0,
                max: 1830.0,
            },
            activation: Activation::Awake,
            rush_cooldown: 0,
            state: Some(state),
        });
        llama
    }

    fn centre(p: &Piece) -> i32 {
        p.x + p.w / 2
    }

    #[test]
    fn a_llama_without_a_rush_draws_nothing() {
        let legacy = Llama::new(848.0, 111.5, EnemyAi::Chase);
        assert_eq!(rush_stand_in(&legacy, FOOT), None);
        let mut spit = legacy.clone();
        spit.mode = LlamaMode::Attack;
        assert_eq!(rush_stand_in(&spit, FOOT), None, "a spit is not a rush");
        let mut hurt = llama_in(1, RushPhase::Tell, 8, 0, 0);
        hurt.mode = LlamaMode::Hurt;
        assert_eq!(rush_stand_in(&hurt, FOOT), None);
    }

    #[test]
    fn the_brace_leans_back_and_scrapes_behind_the_hind_hoof() {
        let tell = llama_rush::active().crlq06_tell_ticks;
        for dir in [-1i8, 1] {
            let (mut scrapes, mut puffs) = (0, 0);
            for tick in 0..tell {
                let s = rush_stand_in(&llama_in(dir, RushPhase::Tell, tick, 0, 0), FOOT)
                    .expect("a rush");
                assert_eq!(
                    s.lean_px.signum(),
                    -i32::from(dir),
                    "tick {tick}: lean back"
                );
                for p in &s.pieces {
                    assert!((centre(p) - X as i32) * i32::from(dir) < 0, "{tick} {p:?}");
                    assert!(p.y >= FOOT as i32, "{tick} {p:?}: above the floor");
                    scrapes += usize::from(p.tone == Tone::Scrape);
                    puffs += usize::from(p.tone == Tone::Dust);
                }
                assert!(s.pieces.len() <= MAX_PIECES);
            }
            assert!(scrapes >= 3, "the hoof scrapes more than once: {scrapes}");
            assert_eq!(puffs, scrapes, "every scrape kicks one readable puff");
        }
    }

    #[test]
    fn the_charge_leans_in_and_trails_dust_behind() {
        let active = llama_rush::active().crlq07_active_ticks;
        for dir in [-1i8, 1] {
            let mut last = 0;
            for n in 0..active {
                let s = rush_stand_in(&llama_in(dir, RushPhase::Committed, 0, n, 0), FOOT).unwrap();
                assert_eq!(s.lean_px, i32::from(dir) * CHARGE_LEAN_PX);
                let dust = s.pieces.iter().filter(|p| p.tone == Tone::Dust).count();
                assert!(dust >= last && dust >= 1, "the trail grows");
                last = dust;
                assert!(s.pieces.len() <= MAX_PIECES);
                for p in &s.pieces {
                    assert!((centre(p) - X as i32) * i32::from(dir) < 0, "behind: {p:?}");
                }
            }
        }
    }

    #[test]
    fn the_stop_settles_and_the_bonk_bursts_at_the_front() {
        let p = llama_rush::active();
        let recover = rush_stand_in(
            &llama_in(1, RushPhase::Recover, 0, 0, p.crlq09_recover_ticks),
            FOOT,
        )
        .unwrap();
        assert_eq!(recover.lean_px, 0);
        assert!(!recover.pieces.is_empty());
        let late = rush_stand_in(&llama_in(1, RushPhase::Recover, 0, 0, 1), FOOT).unwrap();
        assert!(late.pieces.is_empty(), "the dust has settled");
        let bonk = rush_stand_in(
            &llama_in(1, RushPhase::Bonk, 0, 0, p.crlq10_bonk_ticks),
            FOOT,
        )
        .unwrap();
        assert_eq!(bonk.lean_px, -BONK_ROCK_PX, "rocked back");
        assert!(
            bonk.pieces.iter().all(|q| centre(q) > X as i32),
            "at the front"
        );
        let later = rush_stand_in(&llama_in(1, RushPhase::Bonk, 0, 0, 1), FOOT).unwrap();
        assert_eq!(later.lean_px, 0);
        assert!(later.pieces.is_empty());
    }

    /// CRL-Q38: the stand-in never brings back the throat glow: every rush phase
    /// keeps the idle frames of the llama pose.
    #[test]
    fn a_rush_never_shows_the_throat_glow() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let clips = super::super::SpriteClips::load_llama(
            root,
            crate::actor_sprite::FrameSource::GridSource,
        )
        .expect("the LLAMA-01 clips");
        for attack_tick in 0..80u16 {
            let pose = super::super::llama_pose(
                &clips,
                &super::super::LlamaInput {
                    mode: LlamaMode::Attack,
                    attack_tick,
                    rush: true,
                    down_ticks: 0,
                    walking: false,
                    walk_milli_px: 0,
                    idle_tick: u64::from(attack_tick),
                },
            );
            assert_eq!(pose.tag, "idle", "attack tick {attack_tick}");
        }
    }
}
