//! The pure half of the TROG atlas presentation: the tables equal the manifests, every state maps
//! to a frame of its tag, the sprite stands on the pivot, and drawing never changes the state hash.

use super::*;
use crate::trog::{Spawn, TrogAct, TrogCtx, TrogStep};

fn manifest(n: u32) -> serde_json::Value {
    let path = format!(
        "{}/assets/manifests/trog-0{n}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn the_tables_equal_the_manifests() {
    for (n, species, key) in [
        (1, Species::Pygmy, "pygmy"),
        (2, Species::Basher, "basher"),
        (3, Species::Virtuoso, "virtuoso"),
    ] {
        let m = manifest(n);
        let size = m["frame_size"].as_array().unwrap();
        assert_eq!(
            (
                size[0].as_u64().unwrap() as u32,
                size[1].as_u64().unwrap() as u32
            ),
            frame_size(species)
        );
        let pv = m["pivot"].as_array().unwrap();
        assert_eq!(
            (
                pv[0].as_u64().unwrap() as u32,
                pv[1].as_u64().unwrap() as u32
            ),
            pivot(species)
        );
        assert_eq!(m["output_atlas"].as_str().unwrap(), atlas(species));
        let frames = m[key]["frames"].as_array().unwrap();
        assert_eq!(frames.len() as u32, FRAMES);
        let ticks: Vec<u16> = frames
            .iter()
            .map(|f| f["ticks"].as_u64().map_or(0, |t| t as u16))
            .collect();
        assert_eq!(ticks, frame_ticks(species), "{key}");
        // The tags: idle 0..=2, walk 3..=6, the tell 7..=8, the active 9..=10, land 11, hurt 12,
        // down 13..=15.
        let tags: Vec<&str> = frames.iter().map(|f| f["tag"].as_str().unwrap()).collect();
        assert!(tags[0..3].iter().all(|t| *t == "idle"));
        assert!(tags[3..7].iter().all(|t| *t == "walk"));
        assert!(tags[7..9].iter().all(|t| *t == "attack_tell"));
        assert!(
            tags[9..11]
                .iter()
                .all(|t| t.starts_with("attack_") && *t != "attack_tell")
        );
        assert!(tags[11].starts_with("attack_"));
        assert_eq!(tags[12], "hurt");
        assert!(tags[13..16].iter().all(|t| *t == "down"));
    }
}

fn spawn(species: Species) -> Trog {
    let t = TrogTuning::active();
    Trog::new(
        t,
        &Spawn {
            species,
            index: 0,
            x: 700.0,
            floor_y: 92.0,
            trigger_x: Some(0.0),
            delay_ticks: 0,
            protected: (16.0, 224.0),
        },
    )
}

#[test]
fn every_state_maps_to_a_frame_of_its_tag() {
    let t = TrogTuning::active();
    for species in Species::ALL {
        let mut trog = spawn(species);
        let look = TrogLook::default();
        assert!(frame(t, &trog, 0, look) <= 2, "{species:?} dormant is idle");
        for (state, lo, hi) in [
            (State::Rest, 0, 2),
            (State::Recover, 11, 11),
            (State::Hurt, 12, 12),
        ] {
            trog.state = state;
            let f = frame(t, &trog, 5, look);
            assert!((lo..=hi).contains(&f), "{species:?} {state:?}: {f}");
        }
        trog.state = State::Approach;
        assert!(
            frame(
                t,
                &trog,
                5,
                TrogLook {
                    moving: true,
                    down_age: None
                }
            ) >= 3
        );
        assert!(
            frame(
                t,
                &trog,
                5,
                TrogLook {
                    moving: true,
                    down_age: None
                }
            ) <= 6
        );
        assert!(frame(t, &trog, 5, look) <= 2);
        // The tell: frame 7 for the onset ticks, 8 after; the active: 9 then 10.
        let sp = t.species(species);
        trog.state = State::Tell;
        for (a, want) in [
            (0u16, 7u16),
            (frame_ticks(species)[7] - 1, 7),
            (frame_ticks(species)[7], 8),
            (sp.tell_ticks - 1, 8),
        ] {
            trog.attack_tick = a;
            assert_eq!(frame(t, &trog, 0, look), want, "{species:?} tell {a}");
        }
        trog.state = State::Active;
        for (a, want) in [
            (sp.tell_ticks, 9u16),
            (sp.strike_start - 1, 9),
            (sp.strike_start, 10),
            (sp.strike_end, 10),
        ] {
            trog.attack_tick = a;
            assert_eq!(frame(t, &trog, 0, look), want, "{species:?} active {a}");
        }
        // Down plays 13, 14 and holds 15.
        trog.state = State::Down;
        let ft = frame_ticks(species);
        for (age, want) in [
            (0u64, 13u16),
            (u64::from(ft[13]), 14),
            (u64::from(ft[13] + ft[14]), 15),
            (9999, 15),
        ] {
            let f = frame(
                t,
                &trog,
                0,
                TrogLook {
                    moving: false,
                    down_age: Some(age),
                },
            );
            assert_eq!(f, want, "{species:?} down {age}");
        }
    }
}

#[test]
fn the_idle_loop_walks_through_its_frames_on_their_ticks() {
    let t = TrogTuning::active();
    let trog = spawn(Species::Pygmy);
    let seen: Vec<u16> = (0..32)
        .map(|k| frame(t, &trog, k, TrogLook::default()))
        .collect();
    assert_eq!(seen[0], 0);
    assert_eq!(seen[11], 0);
    assert_eq!(seen[12], 1);
    assert_eq!(seen[19], 1);
    assert_eq!(seen[20], 2);
    assert_eq!(seen[31], 2);
    assert_eq!(frame(t, &trog, 32, TrogLook::default()), 0);
}

#[test]
fn the_sprite_stands_on_the_pivot_and_mirrors_with_the_facing() {
    let t = TrogTuning::active();
    for species in Species::ALL {
        let mut trog = spawn(species);
        let (w, h) = frame_size(species);
        let (px, py) = pivot(species);
        trog.facing = 1;
        let d = draw(t, &trog, 0, TrogLook::default());
        let r = sprite_rect(&trog, &d);
        // The pivot pixel column sits on the troglodyte's x, its bottom row on the feet.
        assert_eq!(r.left + px as f32, trog.x, "{species:?}");
        assert_eq!(r.bottom + (h - 1 - py) as f32, trog.foot_y, "{species:?}");
        assert!(!d.flip_x);
        trog.facing = -1;
        let m = draw(t, &trog, 0, TrogLook::default());
        let r = sprite_rect(&trog, &m);
        assert!(m.flip_x);
        assert_eq!(r.left + (w - px) as f32, trog.x, "{species:?} mirrored");
    }
}

#[test]
fn a_leaping_pygmy_is_drawn_at_its_airborne_pivot() {
    let t = TrogTuning::active();
    let mut trog = spawn(Species::Pygmy);
    let ground = draw(t, &trog, 0, TrogLook::default()).centre.1;
    trog.lift = 16.0;
    assert_eq!(
        draw(t, &trog, 0, TrogLook::default()).centre.1,
        ground + 16.0
    );
}

/// Drawing reads the domain state and never writes it: the state hash of a full attack cycle is
/// the same with the presenter's calls interleaved.
#[test]
fn drawing_never_changes_the_state_hash() {
    let t = TrogTuning::active();
    let heroes = {
        let h = |id, x: f32, w: f32, hh: f32| crate::boss::HeroView {
            id,
            present: true,
            x,
            foot_y: 92.0,
            grounded: true,
            hurt: Rect {
                left: x - w / 2.0,
                bottom: 92.0,
                width: w,
                height: hh,
            },
            vulnerable: true,
            bot: false,
        };
        [
            h(crate::party::HeroId::Carl, 620.0, 24.0, 52.0),
            h(crate::party::HeroId::Donut, 80.0, 20.0, 24.0),
        ]
    };
    let run = |draw_it: bool| {
        let mut trog = spawn(Species::Pygmy);
        let mut hash = Vec::new();
        for tick in 1..400u64 {
            let ctx = TrogCtx {
                tick,
                heroes: &heroes,
                solids: &[],
                solo: false,
                view_x: (0.0, 4000.0),
                token_free: true,
                last_tell_start: None,
            };
            let mut out = TrogStep::default();
            trog.step(t, &ctx, &mut |_| false, &mut out);
            if draw_it {
                let d = draw(
                    t,
                    &trog,
                    tick,
                    TrogLook {
                        moving: tick % 3 == 0,
                        down_age: None,
                    },
                );
                let _ = (sprite_rect(&trog, &d), out.acts.contains(&TrogAct::Whiff));
            }
            trog.mix_into(&mut |v| hash.push(v));
        }
        hash
    };
    assert_eq!(run(false), run(true));
}

// ------------------------------------------------------------------ TROG-FX-01: the attack telegraph

/// Carl 80 px in front of a troglodyte at `x`, Donut far behind.
fn heroes_before(x: f32) -> [crate::boss::HeroView; 2] {
    let h = |id, x: f32, w: f32, hh: f32| crate::boss::HeroView {
        id,
        present: true,
        x,
        foot_y: 92.0,
        grounded: true,
        hurt: Rect {
            left: x - w / 2.0,
            bottom: 92.0,
            width: w,
            height: hh,
        },
        vulnerable: true,
        bot: false,
    };
    [
        h(crate::party::HeroId::Carl, x - 80.0, 24.0, 52.0),
        h(crate::party::HeroId::Donut, 80.0, 20.0, 24.0),
    ]
}

/// A troglodyte `species` stepped for `ticks` ticks against a Carl 80 px in front of it (so it has
/// woken and committed an attack).
fn tell_of(species: Species, ticks: u64) -> Trog {
    let t = TrogTuning::active();
    let mut trog = spawn(species);
    let heroes = heroes_before(trog.x);
    for tick in 1..ticks {
        let ctx = TrogCtx {
            tick,
            heroes: &heroes,
            solids: &[],
            solo: true,
            view_x: (0.0, 4000.0),
            token_free: true,
            last_tell_start: None,
        };
        trog.step(t, &ctx, &mut |_| false, &mut TrogStep::default());
    }
    trog
}

#[test]
fn the_telegraph_sheet_is_the_approved_weight_shadow_sheet() {
    let m: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{}/assets/manifests/f01-gw-tell-shadow.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(m["output_atlas"].as_str().unwrap(), TELL_SHEET);
    assert_eq!(
        (
            m["frame_size"][0].as_u64().unwrap() as u32,
            m["frame_size"][1].as_u64().unwrap() as u32
        ),
        TELL_FRAME
    );
    assert_eq!(
        m["prop_kit"]["frames"].as_array().unwrap().len() as u32,
        TELL_FRAMES
    );
    // The tile is inside the frame and a whole number of dither periods wide.
    assert!(TELL_TILE_X + TELL_TILE_W <= TELL_FRAME.0 && TELL_TILE_W.is_multiple_of(2));
    // The approved sheet's own tags and clock: two flashing phases, then the solid patch.
    assert_eq!(
        (
            FALL_SHADOW_TELL.first,
            FALL_SHADOW_TELL.frames,
            FALL_SHADOW_TELL.ticks
        ),
        (0, 2, 4)
    );
    assert_eq!(FALL_SHADOW.first, 2);
}

/// Every tile column of the grid source of the sheet is a flat 8 x 8 pattern: full height in the tell
/// frames (rows 0 and 7 solid, the inside a checker) and all solid in the live frame, so the crop shows the
/// approved pixels and nothing of the ellipse's rounded ends.
#[test]
fn the_tile_columns_are_a_flat_full_height_pattern_of_the_approved_frames() {
    let g: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{}/assets/source/props/floor01/gw_tell_shadow.grid.json",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    )
    .unwrap();
    for frame in 0..3usize {
        let rows: Vec<&str> = g["frames"][frame]["cels"]["body"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap())
            .collect();
        for (y, row) in rows.iter().enumerate() {
            let tile = &row[TELL_TILE_X as usize..(TELL_TILE_X + TELL_TILE_W) as usize];
            if frame == 2 || y == 0 || y == 7 {
                assert!(
                    tile.chars().all(|c| c == '$'),
                    "frame {frame} row {y}: {tile}"
                );
            } else {
                // The dither: alternate columns, no gold tick key.
                assert!(
                    tile.chars().all(|c| c == '$' || c == '.'),
                    "frame {frame} row {y}: {tile}"
                );
                assert_eq!(
                    tile.chars().filter(|c| *c == '$').count(),
                    4,
                    "frame {frame} row {y}"
                );
            }
        }
    }
}

#[test]
fn the_pieces_cover_exactly_the_width_of_the_rectangle() {
    for width in [1.0f32, 7.0, 8.0, 9.0, 17.0, 21.0, 64.0, 177.5, TELL_MAX_PX] {
        let pieces = tell_pieces(width);
        let total: u32 = pieces.iter().map(|p| p.width).sum();
        assert_eq!(total, width.round() as u32, "{width}");
        for (k, p) in pieces.iter().enumerate() {
            assert_eq!(p.dx, k as u32 * TELL_TILE_W);
            assert!(p.width >= 1 && p.width <= TELL_TILE_W);
            assert_eq!(p.src_x, TELL_TILE_X);
        }
        assert!(pieces.len() <= TELL_MAX_PIECES, "{width}");
    }
}

#[test]
fn the_frame_follows_the_hitbox_not_the_art() {
    // Dither before the hitbox is live (both phases, on the approved 4 tick clock), solid while it is.
    assert_eq!(
        (0..16).map(|k| tell_frame(5, k)).collect::<Vec<_>>(),
        [0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 1]
    );
    assert!((0..40).all(|k| tell_frame(0, k) == 2));
}

#[test]
fn the_telegraph_exists_exactly_while_the_simulation_has_a_threat_and_spans_its_rectangle() {
    let t = TrogTuning::active();
    for species in Species::ALL {
        let sp = t.species(species);
        let mut trog = spawn(species);
        // Outside the attack window: nothing.
        assert_eq!(tell_draw(t, &trog, &[], 0), None, "{species:?} dormant");
        let mut seen_tell = false;
        let mut seen_live = false;
        let mut cursor = spawn(species);
        // Walk the machine until it has been through a tell and a live hitbox.
        let heroes = heroes_before(cursor.x);
        for tick in 1..700u64 {
            let ctx = TrogCtx {
                tick,
                heroes: &heroes,
                solids: &[],
                solo: true,
                view_x: (0.0, 4000.0),
                token_free: true,
                last_tell_start: None,
            };
            cursor.step(t, &ctx, &mut |_| false, &mut TrogStep::default());
            let draw = tell_draw(t, &cursor, &[], tick);
            let threat = cursor.threat(t, &[]);
            // Present exactly when the simulation's threat is (tell and active), never otherwise.
            assert_eq!(draw.is_some(), threat.is_some(), "{species:?} tick {tick}");
            if let (Some(d), Some((rect, left))) = (draw, threat) {
                assert_eq!(d.x, rect.left, "{species:?} tick {tick}");
                assert_eq!(
                    d.width,
                    rect.width.min(TELL_MAX_PX),
                    "{species:?} tick {tick}"
                );
                assert_eq!(d.frame == 2, left == 0, "{species:?} tick {tick}");
                assert_eq!(d.height, TELL_FRAME.1 as f32);
                // Floor attacks sit on the floor, the tongue band stays inside its own rectangle.
                match species {
                    Species::Pygmy | Species::Basher => assert_eq!(d.y, cursor.foot_y),
                    Species::Virtuoso => {
                        assert!(
                            d.y >= rect.bottom
                                && d.y + d.height <= rect.bottom + rect.height + 0.01
                        );
                    }
                }
                seen_tell |= left > 0;
                seen_live |= left == 0;
            }
        }
        trog.state = State::Tell;
        assert!(
            seen_tell && seen_live,
            "{species:?}: tell {seen_tell} live {seen_live}"
        );
        // The tell is exactly the data's: the live window is `strike_start..=strike_end`.
        assert!(sp.strike_start < sp.strike_end);
    }
}

#[test]
fn the_telegraph_is_a_pure_function_of_shared_references() {
    let t = TrogTuning::active();
    let trog = tell_of(Species::Basher, 40);
    let hash = |trog: &Trog| {
        let mut words = Vec::new();
        trog.mix_into(&mut |v| words.push(v));
        words
    };
    let before = hash(&trog);
    for tick in 0..200 {
        let _ = tell_draw(t, &trog, &[], tick);
    }
    assert_eq!(hash(&trog), before);
}
