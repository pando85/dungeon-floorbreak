//! CORRAL-E (CRL-Q38): the code-drawn stand-in of the corral rush, until the
//! LLAMA-03 art exists. The llama keeps its LLAMA-01 idle frames through the
//! whole rush (CORRAL-B wire: a rush **never** shows the throat glow or the spit
//! swing, see [`super::llama_pose`]); this module adds what tells the rush apart
//! from an idle llama, read from the rush state alone:
//!
//! * **brace** (the tell): the body leans back from the rush direction and
//!   pumps (CORRAL-F, the #280 review F3: a 4 px lean that rocks 1 px forward
//!   between scrapes, so the brace reads at 1x as motion an idle llama never
//!   makes), and the hind hoof scrapes the floor, a scrape streak with two flecks
//!   of dirt and a 3x3 dust puff kicked back on every scrape;
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
/// CORRAL-F (#280 review F3): the brace was 2 px and did not read at 1x; it is
/// now 4 px on the scrape beat and [`BRACE_PUMP_PX`] less between scrapes.
pub const BRACE_LEAN_PX: i32 = 4;
/// The brace pump: between two scrapes the body rocks forward by this much.
pub const BRACE_PUMP_PX: i32 = 1;
/// The side of the square dust puff kicked back on every scrape.
pub const BRACE_PUFF_PX: i32 = 3;
pub const CHARGE_LEAN_PX: i32 = 2;
/// The bonk rocks the body back this far for [`BONK_ROCK_TICKS`].
pub const BONK_ROCK_PX: i32 = 3;
pub const BONK_ROCK_TICKS: u16 = 8;
/// The hind hoof scrapes once per this many tell ticks, for half of them.
pub const SCRAPE_PERIOD: u16 = 8;
/// The hind hoof sits this far behind the pivot (the body's back third).
pub const HIND_HOOF_PX: i32 = 14;
/// LLAMA-03 (LL3-AD3): the pawing near fore hoof of the `scrape` frame sits this far ahead of the pivot
/// (frame column 50 less pivot column 44); the scrape gouge and its flecks start there when the art is on.
pub const SCRAPE_HOOF_PX: i32 = 6;
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
    rush_stand_in_art(llama, foot_y, false)
}

/// [`rush_stand_in`] for a scene that may draw the LLAMA-03 rush art: with `art_on` the body lean of the tell and
/// the committed rush is 0 and the scrape gouge and flecks start at the pawing fore hoof ([`SCRAPE_HOOF_PX`]).
pub fn rush_stand_in_art(llama: &Llama, foot_y: f32, art_on: bool) -> Option<RushStandIn> {
    if llama.mode != LlamaMode::Attack {
        return None;
    }
    let rush = llama.rush_state()?;
    let dir = i32::from(rush.dir.signum());
    let x = llama.x.round() as i32;
    let foot = foot_y.round() as i32;
    let hind = x - dir * HIND_HOOF_PX;
    // The scrape's dirt starts under the hind hoof (the stand-in) or the pawing fore hoof (the art).
    let paw = if art_on {
        x + dir * SCRAPE_HOOF_PX
    } else {
        hind
    };
    let front = x + dir * LLAMA_HALF_WIDTH.round() as i32;
    let mut pieces = Vec::new();
    let lean_px = match rush.phase {
        RushPhase::Tell => {
            // The brace settles in over the first 4 ticks, then pumps: full
            // lean on the scrape half of each beat, rocked forward between.
            let beat = rush.tick % SCRAPE_PERIOD;
            let lean = if rush.tick < 4 {
                1 + i32::from(rush.tick) * (BRACE_LEAN_PX - 1) / 4
            } else if beat < SCRAPE_PERIOD / 2 {
                BRACE_LEAN_PX
            } else {
                BRACE_LEAN_PX - BRACE_PUMP_PX
            };
            if rush.tick >= 4 && beat < SCRAPE_PERIOD / 2 {
                let reach = i32::from(beat) + 2;
                // The gouge grows backward under the scraping hoof.
                let (left, w) = span(paw, -dir, reach + 2);
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
                        x: paw - dir * (reach + 2 * k),
                        y: foot + rise + i32::from(beat) / 2,
                        w: 1,
                        h: 1,
                        tone: Tone::Fleck,
                    });
                }
                // A pale puff behind the flecks: the dark gouge and the flecks
                // sink into the lit floor at 1x (the first window capture), the
                // puff carries the scrape's beat (3x3 since CORRAL-F).
                let (left, w) = span(hind - dir * (reach + 4), -dir, BRACE_PUFF_PX);
                pieces.push(Piece {
                    x: left,
                    y: foot + 1 + i32::from(beat) / 2,
                    w,
                    h: BRACE_PUFF_PX,
                    tone: Tone::Dust,
                });
            }
            if art_on { 0 } else { -dir * lean }
        }
        RushPhase::Committed => {
            trail(&mut pieces, hind, foot, dir, rush.active, 0);
            if art_on { 0 } else { dir * CHARGE_LEAN_PX }
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

/// The profile's recover and bonk lengths in ticks of `llama` (0 and 0 without a corral profile): the data the
/// stand-in's settle age and the LLAMA-03 pose map ([`super::llama_rush_pose`]) both read.
pub(super) fn corral_ticks(llama: &Llama) -> (u16, u16) {
    llama.corral.as_ref().map_or((0, 0), |c| {
        (c.profile.crlq09_recover_ticks, c.profile.crlq10_bonk_ticks)
    })
}

/// Ticks since the recover or the bonk began: its length less what is left.
fn settle_age(llama: &Llama, left: u16) -> u16 {
    let (recover, bonk) = corral_ticks(llama);
    let total = match llama.rush_state().map(|r| r.phase) {
        Some(RushPhase::Bonk) => bonk,
        _ => recover,
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

    /// CORRAL-F (#280 review F3): the brace reads at 1x. An idle llama never
    /// moves its sprite off its pivot; the brace leans 3-4 px back once settled
    /// and pumps by 1 px between scrapes, and every scrape puff is 3x3.
    #[test]
    fn the_brace_reads_at_1x_as_a_pumping_lean_with_3x3_puffs() {
        let tell = llama_rush::active().crlq06_tell_ticks;
        for dir in [-1i8, 1] {
            let mut leans = std::collections::BTreeSet::new();
            for tick in 4..tell {
                let s = rush_stand_in(&llama_in(dir, RushPhase::Tell, tick, 0, 0), FOOT).unwrap();
                let back = -s.lean_px * i32::from(dir);
                assert!((3..=4).contains(&back), "tick {tick}: {back} px back");
                leans.insert(back);
                for p in s.pieces.iter().filter(|p| p.tone == Tone::Dust) {
                    assert_eq!((p.w, p.h), (BRACE_PUFF_PX, BRACE_PUFF_PX), "{p:?}");
                }
            }
            assert_eq!(leans.len(), 2, "the brace pumps between 3 and 4 px");
            for tick in 0..4 {
                let s = rush_stand_in(&llama_in(dir, RushPhase::Tell, tick, 0, 0), FOOT).unwrap();
                let back = -s.lean_px * i32::from(dir);
                assert!((1..=BRACE_LEAN_PX).contains(&back), "settling {tick}");
            }
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

    /// With the rush art on, the body lean of the tell and the committed rush is 0 (LL3-AD2) and the scrape
    /// gouge and flecks start at the pawing fore hoof, `SCRAPE_HOOF_PX` ahead of the pivot (LL3-AD3); the 3 x 3
    /// puff stays behind the hind hoof, and every other phase draws what it drew.
    #[test]
    fn with_the_art_on_the_lean_is_zero_and_the_gouge_starts_at_the_paw() {
        let tell = llama_rush::active().crlq06_tell_ticks;
        let active = llama_rush::active().crlq07_active_ticks;
        let p = llama_rush::active();
        for dir in [-1i8, 1] {
            let d = i32::from(dir);
            let mut scrapes = 0;
            for tick in 0..tell {
                let llama = llama_in(dir, RushPhase::Tell, tick, 0, 0);
                let on = rush_stand_in_art(&llama, FOOT, true).unwrap();
                let off = rush_stand_in_art(&llama, FOOT, false).unwrap();
                assert_eq!(on.lean_px, 0, "tell tick {tick}");
                assert_eq!(on.pieces.len(), off.pieces.len(), "same beat, tick {tick}");
                for q in on.pieces.iter().filter(|q| q.tone == Tone::Scrape) {
                    scrapes += 1;
                    // The gouge starts at the paw and grows backward (-dir).
                    let paw = X as i32 + d * SCRAPE_HOOF_PX;
                    let (near, far) = (q.x + q.w, q.x);
                    let start = if dir > 0 { near } else { far };
                    assert_eq!(start, paw, "tick {tick}: the gouge starts at x + dir * 6");
                    assert_eq!(q.y, FOOT as i32);
                }
                // The flecks sit behind the paw, higher as the scrape runs.
                for q in on.pieces.iter().filter(|q| q.tone == Tone::Fleck) {
                    assert!((q.x - (X as i32 + d * SCRAPE_HOOF_PX)) * d < 0, "{q:?}");
                }
                // The puff stays where it was (behind the hind hoof).
                let dust = |s: &RushStandIn| {
                    s.pieces
                        .iter()
                        .filter(|q| q.tone == Tone::Dust)
                        .copied()
                        .collect::<Vec<_>>()
                };
                assert_eq!(
                    dust(&on),
                    dust(&off),
                    "tell tick {tick}: the puff is unchanged"
                );
            }
            assert!(scrapes >= 3, "{scrapes}");
            for n in 1..=active {
                let llama = llama_in(dir, RushPhase::Committed, 0, n, 0);
                let on = rush_stand_in_art(&llama, FOOT, true).unwrap();
                let off = rush_stand_in_art(&llama, FOOT, false).unwrap();
                assert_eq!(on.lean_px, 0, "active {n}");
                assert_eq!(on.pieces, off.pieces, "the trail is unchanged, active {n}");
            }
            // Recover and bonk are the stand-in's, art on or off (the bonk rock stays).
            for (phase, left) in [
                (RushPhase::Recover, p.crlq09_recover_ticks),
                (RushPhase::Recover, 1),
                (RushPhase::Bonk, p.crlq10_bonk_ticks),
                (RushPhase::Bonk, 1),
            ] {
                let llama = llama_in(dir, phase, 0, 0, left);
                assert_eq!(
                    rush_stand_in_art(&llama, FOOT, true),
                    rush_stand_in_art(&llama, FOOT, false),
                    "{phase:?} {left}"
                );
            }
        }
    }

    /// Art off keeps `rush_stand_in` exactly as it was (the missing-atlas fallback).
    #[test]
    fn with_the_art_off_the_stand_in_is_the_old_stand_in() {
        for dir in [-1i8, 1] {
            for (phase, tick, active, left) in [
                (RushPhase::Tell, 9, 0, 0),
                (RushPhase::Committed, 0, 7, 0),
                (RushPhase::Recover, 0, 0, 30),
                (RushPhase::Bonk, 0, 0, 55),
            ] {
                let llama = llama_in(dir, phase, tick, active, left);
                assert_eq!(
                    rush_stand_in(&llama, FOOT),
                    rush_stand_in_art(&llama, FOOT, false)
                );
            }
        }
    }

    /// LLAMA-03 (A10, replacing CRL-Q38's `a_rush_never_shows_the_throat_glow`): walking a rush through all
    /// four phases, every drawn frame is a rush-atlas frame or the LLAMA-01 hurt frame (11), never an
    /// `attack_tell` or `attack_swing` frame (7 to 10) of LLAMA-01.
    #[test]
    fn a_rush_never_shows_the_throat_glow() {
        use super::super::llama_rush_pose::{BASE_HURT_FRAME, RushFrame, llama_rush_frame};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = crate::actor_sprite::FrameSource::GridSource;
        let base = super::super::SpriteClips::load_llama(root, source).expect("the LLAMA-01 clips");
        let hurt = base.frame_in("hurt", 0);
        assert_eq!(
            hurt, BASE_HURT_FRAME,
            "the bonk's Base frame is the LLAMA-01 hurt frame"
        );
        let p = llama_rush::active();
        let glow_frames: Vec<usize> = ["attack_tell", "attack_swing"]
            .into_iter()
            .flat_map(|tag| {
                let (first, count) = base.tag(tag);
                first..first + count
            })
            .collect();
        assert_eq!(glow_frames, vec![7, 8, 9, 10]);
        let mut seen = std::collections::BTreeSet::new();
        for dir in [-1i8, 1] {
            let mut walk = |llama: Llama| {
                let frame = llama_rush_frame(&llama).expect("a rush is drawn");
                match frame {
                    RushFrame::Rush(i) => assert!(i < 10, "{frame:?}"),
                    RushFrame::Base(i) => {
                        assert_eq!(i, hurt, "only the hurt frame comes from LLAMA-01");
                        assert!(!glow_frames.contains(&i));
                    }
                }
                seen.insert(frame);
            };
            for tick in 0..p.crlq06_tell_ticks {
                walk(llama_in(dir, RushPhase::Tell, tick, 0, 0));
            }
            for n in 1..=p.crlq07_active_ticks {
                walk(llama_in(dir, RushPhase::Committed, 0, n, 0));
            }
            for left in (1..=p.crlq09_recover_ticks).rev() {
                walk(llama_in(dir, RushPhase::Recover, 0, 0, left));
            }
            for left in (1..=p.crlq10_bonk_ticks).rev() {
                walk(llama_in(dir, RushPhase::Bonk, 0, 0, left));
            }
        }
        // The whole atlas is reachable and the bonk's hurt frame is drawn.
        for i in 0..10 {
            assert!(
                seen.contains(&RushFrame::Rush(i)),
                "rush frame {i} is never drawn"
            );
        }
        assert!(seen.contains(&RushFrame::Base(BASE_HURT_FRAME)));
    }

    /// The missing-atlas fallback keeps CRL-Q38's rule: `llama_pose` holds the LLAMA-01 idle frames through
    /// the whole rush, never the throat glow or the spit swing.
    #[test]
    fn the_fallback_body_never_shows_the_throat_glow() {
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
