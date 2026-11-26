use super::*;
use crate::blast::{BlastCharge, BlastProfile};
use crate::boss::{self, BossCtx, HeroView};
use crate::combat::CombatEvent;
use crate::hazards_art::{self, Tone};
use crate::hazards_fixtures::{Layout, Rig, SlimeAt, walk};
use crate::input::RawInput;
use crate::party::HeroId;
use crate::prototype::Rect;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn json(path: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(root().join(path)).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// Every sheet's frame size, frame count and tag rows (first frame, count, ticks)
/// are the manifest's (`scripts/author_danger_fx.py` writes both).
#[test]
fn the_tag_table_matches_the_manifests() {
    for sheet in Sheet::ALL {
        let m = json(&format!("assets/manifests/{}.json", sheet.manifest()));
        assert_eq!(m["output_atlas"], sheet.atlas(), "{sheet:?}");
        let (w, h) = sheet.frame_size();
        assert_eq!(m["frame_size"], serde_json::json!([w, h]), "{sheet:?}");
        let frames = m["prop_kit"]["frames"].as_array().unwrap();
        assert_eq!(frames.len(), sheet.frames(), "{sheet:?}");
        for t in sheet.tags() {
            let rows: Vec<&serde_json::Value> =
                frames.iter().filter(|f| f["tag"] == t.name).collect();
            assert_eq!(rows.len(), t.frames, "{sheet:?} {}", t.name);
            assert_eq!(rows[0]["index"], t.first, "{sheet:?} {}", t.name);
            if *t == BLAST_WAVE {
                let ticks: Vec<u64> = rows.iter().map(|f| f["ticks"].as_u64().unwrap()).collect();
                assert_eq!(ticks, WAVE_TICKS);
            } else if *t == SLAM_DUST {
                let ticks: Vec<u64> = rows.iter().map(|f| f["ticks"].as_u64().unwrap()).collect();
                assert_eq!(ticks, DUST_TICKS);
            } else if *t == SPLASH {
                let ticks: Vec<u64> = rows.iter().map(|f| f["ticks"].as_u64().unwrap()).collect();
                assert_eq!(ticks, SPLASH_TICKS);
            } else if t.ticks > 0 {
                assert!(rows.iter().all(|f| f["ticks"] == t.ticks), "{}", t.name);
            } else {
                assert!(rows.iter().all(|f| f["ticks"].is_null()), "{}", t.name);
            }
        }
    }
}

/// The opaque pixels of `frame` of a sheet's source grid, as (dx, dy) from the
/// canvas centre with their palette names.
fn grid_pixels(sheet: Sheet, frame: usize) -> Vec<(i32, i32, String)> {
    let stem = sheet
        .manifest()
        .trim_start_matches("f01-")
        .replace('-', "_");
    let g = json(&format!("assets/source/props/floor01/{stem}.grid.json"));
    let names: std::collections::HashMap<char, String> = g["palette"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["key"].as_str().unwrap().chars().next().unwrap(),
                p["name"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let (w, _) = sheet.frame_size();
    let c = (w - 1) / 2;
    let rows = g["frames"][frame]["cels"]["body"].as_array().unwrap();
    let mut out = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        for (x, k) in row.as_str().unwrap().chars().enumerate() {
            if k != '.' {
                out.push((x as i32 - c, y as i32 - c, names[&k].clone()));
            }
        }
    }
    out
}

/// The drawn reach is the simulation's: every ring and wave pixel lies within
/// one pixel outside the radius, the ring paint straddles it, and the wave's
/// reach frame puts its bright edge on `R - 2..R` with a soot rim just outside.
#[test]
fn the_rings_and_waves_end_on_the_sim_radius() {
    for (ring, wave) in [
        (Sheet::RingR48, Sheet::WaveR48),
        (Sheet::RingR64, Sheet::WaveR64),
    ] {
        let r = ring.radius().unwrap() as f32;
        for frame in 0..ring.frames() {
            for (dx, dy, name) in grid_pixels(ring, frame) {
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                assert!(d < r + 2.0 && d >= r - 4.0, "{ring:?} {frame} {d}");
                if name != "iron_00" {
                    assert!(d >= r - 3.0 && d < r + 1.0, "{ring:?} {frame} paint at {d}");
                }
            }
        }
        for frame in 0..wave.frames() {
            for (dx, dy, _) in grid_pixels(wave, frame) {
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                assert!(
                    d < r + 1.0,
                    "{wave:?} frame {frame} pixel beyond the reach at {d}"
                );
            }
        }
        let reach = grid_pixels(wave, WAVE_REACH_FRAME);
        let edge = reach
            .iter()
            .filter(|(_, _, n)| n == "flame_03")
            .map(|(dx, dy, _)| ((dx * dx + dy * dy) as f32).sqrt())
            .fold(0.0f32, f32::max);
        assert!(
            edge >= r - 1.0 && edge < r,
            "{wave:?} bright edge at {edge}"
        );
        assert!(
            reach
                .iter()
                .any(|(dx, dy, n)| n == "iron_00" && ((dx * dx + dy * dy) as f32).sqrt() >= r),
            "{wave:?} soot rim on the reach"
        );
    }
}

/// A ring or wave is drawn only for a radius that has its own canvas.
#[test]
fn only_a_drawn_radius_gets_a_ring_or_a_wave() {
    assert_eq!(ring_sheet(48.0), Some(Sheet::RingR48));
    assert_eq!(ring_sheet(64.0), Some(Sheet::RingR64));
    assert_eq!(wave_sheet(48.0), Some(Sheet::WaveR48));
    assert_eq!(wave_sheet(64.0), Some(Sheet::WaveR64));
    for other in [20.0, 40.0, 47.5, 56.0, 60.0] {
        assert_eq!(ring_sheet(other), None, "{other}");
        assert_eq!(wave_sheet(other), None, "{other}");
    }
    // The active Rot tuning (48 px) and the thrown dynamite's (64 px) both have one.
    assert_eq!(rot_ring_sheet(), Some(Sheet::RingR48));
    let thrown = crate::thrown_charge::blast_profile(crate::carl_prog::CarlProgTuning::active());
    assert_eq!(
        (thrown.core_radius, thrown.outer_radius),
        (20.0, 64.0),
        "the owner note: a 20 px core, a 64 px ring"
    );
    assert_eq!(wave_sheet(thrown.outer_radius), Some(Sheet::WaveR64));
}

/// The wave plays once on the charge's own detonation tick: 22 ticks, the reach
/// frame on ticks 6..11, centred on the blast centre; the QA profile (20 px, no
/// outer band) draws no wave.
#[test]
fn the_wave_plays_once_from_the_detonation_tick() {
    let frames: Vec<Option<usize>> = (0..24).map(wave_frame).collect();
    assert_eq!(frames[0], Some(0));
    assert_eq!(frames[6], Some(WAVE_REACH_FRAME));
    assert_eq!(frames[11], Some(WAVE_REACH_FRAME));
    assert_eq!(frames[21], Some(5));
    assert_eq!(frames[22], None);
    let wide = BlastProfile {
        core_radius: 20.0,
        outer_radius: 64.0,
        ..BlastProfile::QA
    };
    let mut charge = BlastCharge::new(300.0, 100.0, 50, None).with_profile(wide);
    assert_eq!(blast_wave(&charge, 50), None, "not detonated yet");
    assert!(charge.detonate(50));
    let draw = blast_wave(&charge, 56).unwrap();
    assert_eq!(draw.sheet, Sheet::WaveR64);
    assert_eq!(draw.frame, WAVE_REACH_FRAME);
    assert_eq!(draw.layer, Layer::Blast);
    // The centre pixel of the 135 px cell is the blast centre (300, 108).
    assert_eq!((draw.x + 67, draw.y + 67), (300, 108));
    assert_eq!((draw.w, draw.h), (135, 135));
    assert_eq!(blast_wave(&charge, 72), None, "over");
    let mut qa = BlastCharge::new(300.0, 100.0, 50, None);
    assert!(qa.detonate(50));
    assert_eq!(blast_wave(&qa, 50), None);
}

fn acid_rig() -> Rig {
    Layout {
        acid: vec![(700, 764, Some(0))],
        slimes: vec![SlimeAt {
            x: 400,
            facing: 1,
            home: 732,
        }],
        carl_x: 112,
        ..Layout::default()
    }
    .sim(None)
}

/// The drawn pool is the volume: the cropped parts cover exactly the live
/// rectangle (x and width) and its 6 rows sit on the floor, full and draining,
/// with the drain loop while it drains; a drained pool draws nothing.
#[test]
fn the_acid_covers_exactly_the_live_volume() {
    // The canvas is drawn for the active volume height (another height keeps the
    // code-drawn strip).
    assert!(acid_drawn());
    assert_eq!(ACID_VOLUME_H, ACID_H - 4);
    let mut rig = acid_rig();
    let layout_h = rig.scene().world.layout_height();
    let check = |rig: &Rig, want_tag: Tag| {
        let env = rig.combat().env().unwrap();
        let rect = env.acid().get(1).unwrap().rect().unwrap();
        let parts = acid(env, layout_h, rig.tick());
        let x0 = parts.iter().map(|d| d.x).min().unwrap();
        let x1 = parts.iter().map(|d| d.x + d.w).max().unwrap();
        assert_eq!((x0, x1), (rect.x, rect.right()));
        assert_eq!(parts.iter().map(|d| d.w).sum::<i32>(), rect.w, "no overlap");
        for d in &parts {
            // The canvas bottom row is the volume's bottom row; its top 4 rows
            // (fumes) stand above the 6 px volume.
            assert_eq!(d.y, layout_h - rect.bottom());
            assert_eq!(rect.h, ACID_H - 4);
            assert!(d.frame >= want_tag.first && d.frame < want_tag.first + want_tag.frames);
            assert!(d.src_x >= 0 && d.src_x + d.w <= ACID_W);
        }
        // Both caps are drawn: the left part starts at column 0, the right one
        // ends on the canvas' last column.
        assert!(parts.iter().any(|d| d.src_x == 0 && d.x == rect.x));
        assert!(
            parts
                .iter()
                .any(|d| d.src_x + d.w == ACID_W && d.x + d.w == rect.right())
        );
    };
    check(&rig, ACID_IDLE);
    rig.idle(1);
    rig.kill_slime(0);
    for k in 0..70 {
        rig.carl(RawInput::NONE);
        let env = rig.combat().env().unwrap();
        match env.acid().get(1).unwrap().rect() {
            Some(rect) => {
                check(&rig, ACID_DRAIN);
                // An odd width splits one column more to the left part.
                let parts = acid(env, layout_h, rig.tick());
                assert_eq!(parts[0].w, (rect.w + 1) / 2, "drain tick {k}");
            }
            None => assert!(acid(env, layout_h, rig.tick()).is_empty()),
        }
    }
}

/// The art replaces a stand-in only where the stand-in drew: a pool wider than
/// the canvas is filled from the middle columns, never stretched.
#[test]
fn a_wide_pool_is_tiled_not_stretched() {
    let mut out = Vec::new();
    acid_strip(10, 150, 0, 0, &mut out);
    assert_eq!(out.iter().map(|d| d.w).sum::<i32>(), 150);
    assert_eq!(out.first().unwrap().src_x, 0);
    assert_eq!(out.last().unwrap().src_x + out.last().unwrap().w, ACID_W);
    assert!(out.iter().all(|d| d.w <= ACID_W / 2));
    let mut one = Vec::new();
    acid_strip(5, 1, 0, 0, &mut one);
    assert_eq!(one.iter().map(|d| d.w).sum::<i32>(), 1);
    assert!(one.iter().all(|d| d.w >= 0));
}

fn provoked_rig() -> (Rig, u64) {
    let mut rig = Layout {
        stickers: vec![Layout::under_slab(600, 600, 600)],
        carl_x: 590,
        ..Layout::default()
    }
    .sim(None);
    let mut at = None;
    for _ in 0..10 {
        rig.carl(RawInput::NONE);
        at = rig.provokes().iter().find_map(|e| match e {
            CombatEvent::RotProvoked { tick, .. } => Some(*tick),
            _ => None,
        });
        if at.is_some() {
            break;
        }
    }
    (rig, at.expect("provoked"))
}

/// The reach ring of a warning sticker is drawn on exactly the ticks the
/// stand-in ring is (HAZ-Q38), centred on the dome, marching, then hot for the
/// last 6 ticks; then the 48 px wave plays from the blast tick.
#[test]
fn the_rot_ring_follows_the_warning_and_the_wave_follows_the_blast() {
    let (mut rig, t) = provoked_rig();
    let layout_h = rig.scene().world.layout_height();
    let tuning = crate::rot_sticker::active();
    let mut waves = 0;
    loop {
        let tick = rig.tick();
        let k = tick.saturating_sub(t);
        let combat = rig.combat();
        let env = combat.env().unwrap();
        let stand_in = hazards_art::pieces(env, layout_h, tick)
            .iter()
            .any(|p| p.tone == Tone::Ring);
        let rings = rot_rings(env, tick);
        assert_eq!(!rings.is_empty(), stand_in, "t+{k}");
        if let Some(ring) = rings.first() {
            let dome = env.stickers().next().unwrap().1.dome(tuning);
            assert_eq!(ring.sheet, Sheet::RingR48);
            assert_eq!(ring.layer, Layer::Tell);
            assert_eq!(
                (ring.x + 51, ring.y + 51),
                (dome.0.round() as i32, dome.1.round() as i32)
            );
            let hot = 24 - k <= RING_HOT_TICKS;
            let tag = if hot { RING_HOT } else { RING_MARCH };
            assert!(
                ring.frame >= tag.first && ring.frame < tag.first + tag.frames,
                "t+{k}"
            );
        }
        let drawn_waves: Vec<FxDraw> = draws(combat, layout_h, tick)
            .into_iter()
            .filter(|d| d.layer == Layer::Blast)
            .collect();
        if !drawn_waves.is_empty() {
            assert_eq!(drawn_waves[0].sheet, Sheet::WaveR48);
            waves += 1;
        }
        if k > 60 {
            break;
        }
        rig.carl(RawInput::NONE);
    }
    assert_eq!(
        waves,
        WAVE_TICKS.iter().sum::<u64>(),
        "the whole wave, once"
    );
}

/// Carl's landed, burning stick shows the 64 px ring (hot for its last 6 ticks)
/// until the blast, then the 64 px wave plays; none while it flies.
#[test]
fn the_landed_stick_shows_the_reach_then_the_wave() {
    let mut rig = Layout {
        carl_x: 200,
        dynamite: true,
        ..Layout::default()
    }
    .sim(None);
    rig.carl(RawInput {
        ability: true,
        ..RawInput::NONE
    });
    let layout_h = rig.scene().world.layout_height();
    let (mut flew, mut rings, mut hot, mut waves) = (0, 0, 0, 0);
    for _ in 0..240 {
        rig.carl(RawInput::NONE);
        let combat = rig.combat();
        let tick = rig.tick();
        let ring = dynamite_ring(combat, tick);
        match combat.thrower().stick() {
            Some(stick) if !stick.landed => {
                flew += 1;
                assert_eq!(ring, None, "no ring while it flies");
            }
            _ => {}
        }
        if let Some(ring) = ring {
            let charge = &combat.charges[combat.dynamite_slot()];
            let (cx, cy) = charge.centre();
            assert_eq!(ring.sheet, Sheet::RingR64);
            assert_eq!(
                (ring.x + 67, ring.y + 67),
                (cx.round() as i32, cy.round() as i32)
            );
            rings += 1;
            if ring.frame >= RING_HOT.first {
                hot += 1;
            }
        }
        if draws(combat, layout_h, tick)
            .iter()
            .any(|d| d.sheet == Sheet::WaveR64)
        {
            waves += 1;
        }
    }
    assert!(flew > 0, "the stick flew");
    assert!(
        rings > RING_HOT_TICKS as usize,
        "the ring burned while the stick lay"
    );
    assert_eq!(hot, RING_HOT_TICKS as usize);
    assert_eq!(waves, WAVE_TICKS.iter().sum::<u64>());
}

fn view(id: HeroId, x: f32, floor: f32) -> HeroView {
    HeroView {
        id,
        present: true,
        x,
        foot_y: floor,
        grounded: true,
        hurt: Rect {
            left: x - 10.0,
            bottom: floor,
            width: 20.0,
            height: 50.0,
        },
        vulnerable: true,
        bot: false,
    }
}

/// The gate is raised (behind the actors, under the lintel) until the
/// simulation closes it, then the closed grate covers exactly its collider.
#[test]
fn the_gate_follows_the_sims_gate_over_its_collider() {
    let floor = 100.0;
    let mut b = crate::boss::Boss::new(boss::builtin(), floor);
    let open = gate(&b, 0).unwrap();
    assert_eq!((open.frame, open.layer), (GATE_RAISED.first, Layer::Back));
    let ctx = BossCtx {
        heroes: [
            view(HeroId::Carl, 200.0, floor),
            view(HeroId::Donut, 60.0, floor),
        ],
        solids: &[],
        solo: false,
        tick: 1,
    };
    for _ in 0..30 {
        let _ = b.step(&ctx);
    }
    assert!(b.gate_closed());
    let closed = gate(&b, 1 + GATE_IMPACT_AGE).unwrap();
    assert_eq!(
        (closed.frame, closed.layer),
        (GATE_CLOSED.first, Layer::Gate)
    );
    let solid = b.data().gate.solid_x;
    assert_eq!(
        (closed.x, closed.y, closed.w, closed.h),
        (0, 100, solid as i32, 224),
        "the closed gate's collider: x 0..solid_x, floor up 224 px"
    );
}

/// F01-SIM-TICKS: the gate drops on the close tick (four frames, 2 ticks each), the
/// closed grate and its slam dust start on close + 8, the dust plays its 20 ticks
/// once from the gate's arena face, and nothing of it draws before the close.
#[test]
fn the_gate_drops_from_its_close_tick_and_slams_dust_on_the_impact() {
    let floor = 100.0;
    let mut b = crate::boss::Boss::new(boss::builtin(), floor);
    let mut closed_on = None;
    for tick in 40..120u64 {
        let ctx = BossCtx {
            heroes: [
                view(HeroId::Carl, 200.0, floor),
                view(HeroId::Donut, 60.0, floor),
            ],
            solids: &[],
            solo: false,
            tick,
        };
        let out = b.step(&ctx);
        if out
            .events
            .iter()
            .any(|e| matches!(e, CombatEvent::BossEngaged { .. }))
        {
            closed_on = Some(tick);
            break;
        }
        assert_eq!(gate(&b, tick).unwrap().frame, GATE_RAISED.first);
        assert_eq!(gate_dust(&b, tick), None);
    }
    let t = closed_on.expect("the gate closes");
    assert_eq!(b.gate_closed_tick(), Some(t));
    assert_eq!(GATE_IMPACT_AGE, 8);
    let frames: Vec<usize> = (t..t + 10).map(|k| gate(&b, k).unwrap().frame).collect();
    assert_eq!(frames, [2, 2, 3, 3, 4, 4, 5, 5, 0, 0]);
    for k in t..t + 10 {
        assert_eq!(gate(&b, k).unwrap().layer, Layer::Gate);
    }
    assert!(Sheet::Gate.frames() == 6);
    for k in t..t + 8 {
        assert_eq!(gate_dust(&b, k), None, "no dust before the impact");
    }
    let first = gate_dust(&b, t + 8).expect("the impact throws dust");
    assert_eq!(first.sheet, Sheet::SlamDust);
    assert_eq!(first.frame, 0);
    assert_eq!(first.layer, Layer::Dust);
    assert!(!first.flip_x, "it rolls into the arena");
    assert_eq!(
        (first.x + DUST_PIVOT_X, first.y),
        (GATE_W, 100),
        "pivot column on the gate's arena face, on the floor"
    );
    assert_eq!(gate_dust(&b, t + 9).unwrap().frame, 0);
    assert_eq!(gate_dust(&b, t + 10).unwrap().frame, DUST_KEY_FRAME);
    assert_eq!(gate_dust(&b, t + 8 + 19).unwrap().frame, 4);
    assert_eq!(gate_dust(&b, t + 8 + 20), None, "played once");
}

/// F01-SIM-TICKS: the slam dust plays once over 20 ticks (2, 3, 4, 5, 6) and a
/// mirrored plume keeps its pivot column on the impact edge.
#[test]
fn the_slam_dust_plays_once_and_mirrors_about_its_pivot() {
    assert_eq!(DUST_TICKS.iter().sum::<u64>(), 20);
    assert_eq!(Sheet::SlamDust.frame_size(), (DUST_W, DUST_H));
    let seen: Vec<Option<usize>> = (0..21).map(dust_frame).collect();
    assert_eq!(seen[0], Some(0));
    assert_eq!(seen[2], Some(1));
    assert_eq!(seen[5], Some(2));
    assert_eq!(seen[9], Some(3));
    assert_eq!(seen[14], Some(4));
    assert_eq!(seen[19], Some(4));
    assert_eq!(seen[20], None);
    let right = slam_dust(848, 40, 1, 0).unwrap();
    let left = slam_dust(783, 40, -1, 0).unwrap();
    assert_eq!((right.x + DUST_PIVOT_X, right.flip_x), (848, false));
    // Mirrored, the cell's column c lands on the screen column w - 1 - c.
    assert_eq!(
        (left.x + DUST_W - 1 - DUST_PIVOT_X, left.flip_x),
        (783, true)
    );
    assert!(Layer::Dust > Layer::Floor && Layer::Dust < Layer::Splash);
}

/// F01-SIM-TICKS: in the Boiler, the weight's landing tick is the scene tick on
/// which it lands, and the landing dust draws both plumes from that very tick, at
/// the remnant's two edges on its floor, for 20 ticks, never before.
#[test]
fn the_boiler_weight_throws_its_landing_dust_on_the_landing_tick() {
    const WEIGHT: usize = 1;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (_run, mut scene, mut combat) = crate::f01_late::boiler_start(
        &root,
        360,
        crate::carl_prog::CarlProgFlags::OFF,
        crate::donut_prog::DonutProgFlags::OFF,
        None,
    )
    .expect("the Boiler launches");
    for token in 1..=2 {
        combat.env_stimulate(
            WEIGHT,
            crate::env_prop::Stimulus::MeleeHit {
                token,
                player: 0,
                weight: 1,
            },
            0,
        );
    }
    let layout_h = scene.world.layout_height();
    let dust_of = |c: &CombatState, tick: u64| -> Vec<FxDraw> {
        draws(c, layout_h, tick)
            .into_iter()
            .filter(|d| d.sheet == Sheet::SlamDust)
            .collect()
    };
    let mut landed_on = None;
    let mut landed_events = Vec::new();
    for _ in 0..400 {
        combat.step_party(&mut scene, [RawInput::NONE, RawInput::NONE]);
        let tick = scene.world.tick;
        landed_events.extend(combat.take_new_events().iter().filter_map(|e| match e {
            CombatEvent::PropPhase {
                tick,
                prop,
                phase: crate::combat::PropPhase::Landed,
                ..
            } if *prop as usize == WEIGHT => Some(*tick),
            _ => None,
        }));
        let mass = combat.env().unwrap().mass(WEIGHT).unwrap();
        if mass.remnant().is_some() {
            landed_on = Some(tick);
            break;
        }
        assert_eq!(mass.landed_tick(), None);
        assert!(
            dust_of(&combat, tick).is_empty(),
            "no dust before the landing"
        );
    }
    let t = landed_on.expect("the weight lands");
    let mass = combat.env().unwrap().mass(WEIGHT).unwrap();
    assert_eq!(mass.landed_tick(), Some(t), "the landing step's scene tick");
    assert_eq!(landed_events, [t], "the same tick as the one Landed event");
    let remnant = mass.remnant().unwrap();
    let floor = layout_h - remnant.bottom();
    let at = dust_of(&combat, t);
    assert_eq!(at.len(), 2, "one plume on each side");
    let right = at.iter().find(|d| !d.flip_x).unwrap();
    let left = at.iter().find(|d| d.flip_x).unwrap();
    assert_eq!(
        (right.x + DUST_PIVOT_X, right.y, right.frame),
        (remnant.x + remnant.w, floor, 0)
    );
    assert_eq!(
        (left.x + DUST_W - 1 - DUST_PIVOT_X, left.y, left.frame),
        (remnant.x - 1, floor, 0)
    );
    assert_eq!(dust_of(&combat, t + 2)[0].frame, DUST_KEY_FRAME);
    assert_eq!(dust_of(&combat, t + 19).len(), 2);
    assert!(dust_of(&combat, t + 20).is_empty(), "played once");
}

/// F01-SIM-TICKS (review finding 7): a restart rebuilds the boss and resets the
/// masses, so neither the gate drop nor any slam dust survives into the next attempt.
#[test]
fn a_restart_clears_the_gate_drop_and_the_slam_dust() {
    use crate::room_pilots::{Plan, RoomPilot};
    let root = root();
    let dust = |c: &CombatState, layout_h: i32, tick: u64| {
        draws(c, layout_h, tick)
            .into_iter()
            .filter(|d| d.sheet == Sheet::SlamDust)
            .count()
    };
    // Room 5: the juicer plan closes the gate; stop on the impact's dust.
    let mut sim = crate::f01_late_fixtures::juicer_room(&root).unwrap();
    let pilot = RoomPilot {
        plan: Plan::Juicer,
        two_player: false,
        aligned: None,
    };
    let layout_h = sim.scene.world.layout_height();
    let mut closed = None;
    for _ in 0..600 {
        let inputs = pilot.inputs(&sim.combat, &sim.scene, 0.0);
        sim.step_slots(inputs);
        closed = sim.combat.boss.as_ref().unwrap().gate_closed_tick();
        if closed.is_some_and(|t| sim.scene.world.tick >= t + GATE_IMPACT_AGE) {
            break;
        }
    }
    let t = closed.expect("the juicer plan closes the gate");
    assert_eq!(sim.scene.world.tick, t + GATE_IMPACT_AGE);
    assert_eq!(dust(&sim.combat, layout_h, sim.scene.world.tick), 1);
    sim.combat.restart(&mut sim.scene);
    let boss = sim.combat.boss.as_ref().unwrap();
    assert_eq!(boss.gate_closed_tick(), None);
    let tick = sim.scene.world.tick;
    assert_eq!(gate(boss, tick).unwrap().frame, GATE_RAISED.first);
    assert_eq!(dust(&sim.combat, layout_h, tick), 0);
    // The Boiler: land the weight, restart inside the dust's 20 ticks.
    let (_run, mut scene, mut combat) = crate::f01_late::boiler_start(
        &root,
        360,
        crate::carl_prog::CarlProgFlags::OFF,
        crate::donut_prog::DonutProgFlags::OFF,
        None,
    )
    .expect("the Boiler launches");
    for token in 1..=2 {
        combat.env_stimulate(
            1,
            crate::env_prop::Stimulus::MeleeHit {
                token,
                player: 0,
                weight: 1,
            },
            0,
        );
    }
    let layout_h = scene.world.layout_height();
    for _ in 0..400 {
        combat.step_party(&mut scene, [RawInput::NONE, RawInput::NONE]);
        if combat
            .env()
            .unwrap()
            .mass(1)
            .unwrap()
            .landed_tick()
            .is_some()
        {
            break;
        }
    }
    assert_eq!(dust(&combat, layout_h, scene.world.tick), 2);
    combat.restart(&mut scene);
    assert_eq!(combat.env().unwrap().mass(1).unwrap().landed_tick(), None);
    assert_eq!(dust(&combat, layout_h, scene.world.tick), 0);
}

/// Rendering never changes authoritative state: a run that calls every function
/// here on every tick has, at every tick, the state hash of a run that never does.
#[test]
fn drawing_never_changes_the_state_hash() {
    let build = || {
        Layout {
            acid: vec![(700, 764, Some(0))],
            slimes: vec![SlimeAt {
                x: 400,
                facing: 1,
                home: 400,
            }],
            stickers: vec![Layout::under_slab(600, 600, 600)],
            carl_x: 380,
            dynamite: true,
            ..Layout::default()
        }
        .sim(Some(true))
    };
    let (mut plain, mut drawn) = (build(), build());
    let layout_h = drawn.scene().world.layout_height();
    for k in 0..300u64 {
        let input = match k {
            5 => RawInput {
                ability: true,
                ..RawInput::NONE
            },
            0..120 => walk(1.0),
            _ => RawInput::NONE,
        };
        plain.step(input, RawInput::NONE);
        drawn.step(input, RawInput::NONE);
        let tick = drawn.tick();
        let _ = draws(drawn.combat(), layout_h, tick);
        let _ = dynamite_ring(drawn.combat(), tick);
        assert_eq!(plain.hash(), drawn.hash(), "tick {k}");
        assert_eq!(plain.events().len(), drawn.events().len(), "tick {k}");
    }
    assert!(!plain.provokes().is_empty() || !plain.blasts().is_empty());
}

/// AREA-HAZARDS-01B: the splash plays once over 16 ticks (2, 3, 3, 4, 4), its key is
/// frame 2 and its canvas is 32 x 24.
#[test]
fn the_splash_plays_once_over_sixteen_ticks() {
    assert_eq!(SPLASH_TICKS.iter().sum::<u64>(), 16);
    assert_eq!(SPLASH_KEY_FRAME, 2);
    assert_eq!(Sheet::Splash.frame_size(), (32, 24));
    assert_eq!(Sheet::Splash.frames(), 5);
    let seen: Vec<Option<usize>> = (0..17).map(splash_frame).collect();
    assert_eq!(seen[0], Some(0));
    assert_eq!(seen[2], Some(1));
    assert_eq!(seen[5], Some(2));
    assert_eq!(seen[8], Some(3));
    assert_eq!(seen[12], Some(4));
    assert_eq!(seen[15], Some(4));
    assert_eq!(seen[16], None, "played once, then gone");
    assert!(Layer::Splash > Layer::Floor && Layer::Splash < Layer::Blast);
    // The 40-sprite pool is shared by the existing visuals only: the splash has its own.
    assert_eq!(MAX_DRAWS, 40);
}
