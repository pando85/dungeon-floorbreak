use super::*;
use crate::combat::CombatEvent;
use crate::corridor::SceneSim;
use crate::f01_hazards;
use crate::f01_mastery;
use crate::hazards_fixtures::{Layout, Rig, SlimeAt, walk};
use crate::input::RawInput;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn json(path: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(root().join(path)).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The env runtime and the layout height of room 4 (`f01_hazards`).
fn room4() -> (EnvRuntime, i32, &'static str) {
    let run = f01_hazards::launch(&root()).unwrap();
    let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
    let combat = f01_hazards::combat_of(&run, &mut scene).unwrap();
    let h = scene.world.layout_height();
    (combat.env().unwrap().clone(), h, run.fixture.name)
}

/// The env runtime and the layout height of room 6 (`mastery`).
fn room6() -> (EnvRuntime, i32, &'static str) {
    let run = f01_mastery::launch(&root()).unwrap();
    let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
    let combat = f01_mastery::combat_of(&run, &mut scene).unwrap();
    let h = scene.world.layout_height();
    (combat.env().unwrap().clone(), h, run.fixture.name)
}

fn of(decals: &[Decal], sheet: DecalSheet) -> Vec<Decal> {
    decals
        .iter()
        .copied()
        .filter(|d| d.sheet == sheet)
        .collect()
}

/// Every sheet's frame size, frame count, pivot and tag rows are the manifest's
/// (`scripts/author_danger_fx.py` writes both).
#[test]
fn the_decal_sheets_match_the_manifests() {
    for sheet in DecalSheet::ALL {
        let m = json(&format!("assets/manifests/{}.json", sheet.manifest()));
        assert_eq!(m["output_atlas"], sheet.atlas(), "{sheet:?}");
        let (w, h) = sheet.frame_size();
        assert_eq!(m["frame_size"], serde_json::json!([w, h]), "{sheet:?}");
        let frames = m["prop_kit"]["frames"].as_array().unwrap();
        assert_eq!(frames.len(), sheet.frames(), "{sheet:?}");
    }
    // The drip: four tags, in the order the frame constants assume.
    let m = json("assets/manifests/f01-dfx-rot-drip.json");
    let rows: Vec<(String, u64)> = m["prop_kit"]["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["tag"].as_str().unwrap().to_owned(),
                f["ticks"].as_u64().unwrap(),
            )
        })
        .collect();
    let expect = |tag: &str, first: usize, n: usize, ticks: u64| {
        for k in 0..n {
            assert_eq!(rows[first + k], (tag.to_owned(), ticks), "{tag} {k}");
        }
    };
    expect("drip_run_c", DRIP_RUN_C, DRIP_RUN_FRAMES, DRIP_RUN_TICK);
    expect("drip_drop_c", DRIP_DROP_C, DRIP_DROP_FRAMES, DRIP_DROP_TICK);
    expect("drip_run_w", DRIP_RUN_W, DRIP_RUN_FRAMES, DRIP_RUN_TICK);
    expect("drip_drop_w", DRIP_DROP_W, DRIP_DROP_FRAMES, DRIP_DROP_TICK);
    // The pivots of the spec: the nest (16, 0), the trail (8, 0), the wall trail (0, 23).
    let pivot = |name: &str| json(&format!("assets/manifests/{name}.json"))["pivot"].clone();
    assert_eq!(pivot("f01-dfx-rot-nest"), serde_json::json!([16, 0]));
    assert_eq!(pivot("f01-dfx-rot-trail"), serde_json::json!([8, 0]));
    assert_eq!(pivot("f01-dfx-corr-wall"), serde_json::json!([0, 23]));
    assert_eq!(pivot("f01-dfx-corr-top"), serde_json::json!([16, 5]));
    assert_eq!(pivot("f01-dfx-corr-face"), serde_json::json!([16, 0]));
    assert_eq!(pivot("f01-dfx-rot-drip"), serde_json::json!([4, 0]));
}

#[test]
fn the_face_tiles_are_centred_alternately_mirrored_and_cropped() {
    // A 64 px patch: exactly two tiles, the second mirrored, none cropped.
    let t = tiles(248, 312, FACE_W, FACE_MARGIN_PX);
    assert_eq!(t, vec![(248, false, 0, 32), (280, true, 0, 32)]);
    // A 40 px patch: two tiles centred on it (308..372), cropped to [x0 - 4, x1 + 4] =
    // [316, 364]; the mirrored tile shows its source columns 8..32 mirrored.
    let t = tiles(320, 360, FACE_W, FACE_MARGIN_PX);
    assert_eq!(t, vec![(316, false, 8, 24), (340, true, 8, 24)]);
    // A 96 px patch: three tiles, the middle one mirrored.
    let t = tiles(100, 196, FACE_W, FACE_MARGIN_PX);
    assert_eq!(t.len(), 3);
    assert_eq!(
        t.iter().map(|x| x.1).collect::<Vec<_>>(),
        vec![false, true, false]
    );
    // The tiles cover the whole authored span.
    for (x0, x1) in [(248, 312), (320, 360), (100, 196), (10, 11), (0, 33)] {
        let t = tiles(x0, x1, FACE_W, FACE_MARGIN_PX);
        assert_eq!(t.first().unwrap().0.min(x0), t.first().unwrap().0, "{x0}");
        let right = t.iter().map(|(x, _, _, w)| x + w).max().unwrap();
        let left = t.iter().map(|(x, ..)| *x).min().unwrap();
        assert!(left <= x0 && right >= x1, "({x0}, {x1}) -> {left}..{right}");
        assert!(left >= x0 - FACE_MARGIN_PX && right <= x1 + FACE_MARGIN_PX);
    }
}

/// Room 4: both patches have their face tiles and wall trail, the entry stain is at
/// x 150, each ceiling track has its nest and its scuffs.
#[test]
fn room_4_placements_follow_the_authored_data() {
    let (env, layout_h, fixture) = room4();
    let decals = static_decals(&env, layout_h, fixture);
    let line = layout_h - 580;
    // The acid patches (authored 248..312 and 856..920, walking line 580).
    let faces = of(&decals, DecalSheet::CorrFace);
    assert_eq!(faces.len(), 4, "two tiles per 64 px patch");
    for x0 in [248, 856] {
        let mine: Vec<&Decal> = faces
            .iter()
            .filter(|d| d.x >= x0 && d.x < x0 + 64)
            .collect();
        assert_eq!(mine.len(), 2, "patch at {x0}");
        assert_eq!(
            (mine[0].x, mine[0].flip_x, mine[1].x, mine[1].flip_x),
            (x0, false, x0 + 32, true)
        );
        for d in mine {
            // Row 0 is the first solid row under the pool's bottom row.
            assert_eq!((d.y, d.h, d.w), (line - FACE_H, FACE_H, 32));
            assert_eq!(d.layer, DecalLayer::Floor);
        }
    }
    let walls = of(&decals, DecalSheet::CorrWall);
    assert_eq!(
        walls
            .iter()
            .map(|d| (d.x, d.y, d.layer))
            .collect::<Vec<_>>(),
        vec![
            (248 + WALL_INSET_PX, line, DecalLayer::Wall),
            (856 + WALL_INSET_PX, line, DecalLayer::Wall)
        ]
    );
    // The entry stain of room 4, at x 150, bottom row on the walking line.
    let tops = of(&decals, DecalSheet::CorrTop);
    assert_eq!(tops.len(), 1);
    assert_eq!((tops[0].x, tops[0].y), (150 - TOP_W / 2, line));
    // The ceiling tracks r1 (512..600, start 600) and r2 (760..816, start 760).
    let nests = of(&decals, DecalSheet::RotNest);
    let starts: Vec<i32> = nests.iter().map(|d| d.x + NEST_W / 2).collect();
    assert_eq!(starts, vec![600, 760]);
    let trails: Vec<i32> = of(&decals, DecalSheet::RotTrail)
        .iter()
        .map(|d| d.x + TRAIL_W / 2)
        .collect();
    assert_eq!(trails, vec![512, 528, 544, 560, 576, 792, 808]);
    // The nest hangs from the ceiling's underside line.
    for (_, s, _) in env.stickers() {
        let under = s.track().fixed;
        let nest = nests
            .iter()
            .find(|d| d.x + NEST_W / 2 == s.track().start)
            .unwrap();
        assert_eq!((nest.y, nest.h), (under - NEST_H, NEST_H));
        for t in of(&decals, DecalSheet::RotTrail) {
            if t.y + TRAIL_H == under {
                assert_eq!(t.h, TRAIL_H);
            }
        }
    }
}

/// Room 6: the one patch has its stain and trail, no entry stain, one nest and no scuff
/// (its track is 16 px long and the nest sits on its start).
#[test]
fn room_6_placements_follow_the_authored_data() {
    let (env, layout_h, fixture) = room6();
    let decals = static_decals(&env, layout_h, fixture);
    let line = layout_h - 580;
    let faces = of(&decals, DecalSheet::CorrFace);
    // The authored 320..360 patch: two tiles cropped to [316, 364].
    assert_eq!(
        faces
            .iter()
            .map(|d| (d.x, d.w, d.src_x, d.flip_x))
            .collect::<Vec<_>>(),
        vec![(316, 24, 8, false), (340, 24, 8, true)]
    );
    assert_eq!(
        of(&decals, DecalSheet::CorrWall)
            .iter()
            .map(|d| (d.x, d.y))
            .collect::<Vec<_>>(),
        vec![(320 + WALL_INSET_PX, line)]
    );
    assert!(
        of(&decals, DecalSheet::CorrTop).is_empty(),
        "no entry stain here"
    );
    let nests = of(&decals, DecalSheet::RotNest);
    assert_eq!(nests.len(), 1);
    assert_eq!(nests[0].x + NEST_W / 2, 912);
    assert!(of(&decals, DecalSheet::RotTrail).is_empty());
}

/// The stain is dressing: no floor-top stain lies within 24 px of an acid patch's ends,
/// no stain covers the pool's own x range except the face tiles (which sit under it).
#[test]
fn no_floor_top_stain_near_a_patch() {
    for (env, layout_h, fixture) in [room4(), room6()] {
        let decals = static_decals(&env, layout_h, fixture);
        for patch in env.acid().patches() {
            let (x0, x1) = (patch.spec().x0, patch.spec().x1);
            for d in of(&decals, DecalSheet::CorrTop) {
                assert!(
                    d.x + d.w + TOP_PATCH_GAP_PX <= x0 || d.x >= x1 + TOP_PATCH_GAP_PX,
                    "corr_top {}..{} within {TOP_PATCH_GAP_PX} px of patch {x0}..{x1}",
                    d.x,
                    d.x + d.w
                );
            }
        }
    }
}

/// HZB-AD6: a wall track gets no nest (the drip keeps both orientations).
#[test]
fn a_wall_track_has_no_nest() {
    let mut rig = Layout {
        stickers: vec![Layout::on_pillar(60)],
        carl_x: 112,
        ..Layout::default()
    }
    .sim(None);
    rig.idle(1);
    let env = rig.combat().env().unwrap();
    let decals = static_decals(env, rig.scene().world.layout_height(), "haz_fixture");
    assert!(of(&decals, DecalSheet::RotNest).is_empty());
    assert!(of(&decals, DecalSheet::RotTrail).is_empty());
    assert!(decals.is_empty(), "{decals:?}");
}

/// The decals follow the authored extents, not the live pool: a drained patch keeps its
/// stain (HZB-AD8), and the list is the same every time it is computed.
#[test]
fn the_decals_persist_after_a_drain() {
    let mut rig = Layout {
        acid: vec![(700, 764, Some(0))],
        slimes: vec![SlimeAt {
            x: 400,
            facing: 1,
            home: 732,
        }],
        carl_x: 112,
        ..Layout::default()
    }
    .sim(None);
    let layout_h = rig.scene().world.layout_height();
    let before = static_decals(rig.combat().env().unwrap(), layout_h, "haz_fixture");
    assert_eq!(of(&before, DecalSheet::CorrFace).len(), 2);
    rig.idle(1);
    rig.kill_slime(0);
    for _ in 0..90 {
        rig.carl(RawInput::NONE);
    }
    let env = rig.combat().env().unwrap();
    assert!(env.acid().get(1).unwrap().rect().is_none(), "drained away");
    assert_eq!(static_decals(env, layout_h, "haz_fixture"), before);
    assert_eq!(static_capacity(env, "haz_fixture"), before.len());
}

#[test]
fn the_drip_runs_once_then_the_drop_loops() {
    // Ceiling: run frames on ages 0..24, the drop loop after.
    let ceiling: Vec<usize> = (0..24).map(|a| drip_frame(a, false)).collect();
    assert_eq!(&ceiling[..6], &[0; 6]);
    assert_eq!(&ceiling[6..12], &[1; 6]);
    assert_eq!(&ceiling[12..18], &[2; 6]);
    assert_eq!(&ceiling[18..24], &[3; 6]);
    assert_eq!(drip_frame(24, false), 4);
    assert_eq!(drip_frame(43, false), 4);
    assert_eq!(drip_frame(44, false), 5);
    assert_eq!(drip_frame(64, false), 6);
    assert_eq!(drip_frame(84, false), 4, "the drop loops every 60 ticks");
    assert_eq!(drip_frame(24 + 60 * 5 + 21, false), 5);
    // The wall tags are the same beats in their own frames.
    for age in [0, 6, 12, 18, 24, 44, 64, 84, 1000] {
        assert_eq!(
            drip_frame(age, true),
            drip_frame(age, false) + 7,
            "age {age}"
        );
    }
    assert!(drip_frame(u64::MAX / 2, false) < DRIP_FRAMES);
}

/// A real provoked sticker: no drip while it lives, the run starts on its detonation
/// tick (age 0 is frame 0), the drop loop follows, and the drip hangs from the
/// ceiling's underside at the dome column.
#[test]
fn a_spent_sticker_drips_from_its_detonation_tick() {
    let mut rig = Layout {
        stickers: vec![Layout::under_slab(600, 600, 600)],
        carl_x: 590,
        ..Layout::default()
    }
    .sim(None);
    let mut seen_spent = None;
    for _ in 0..80 {
        rig.carl(RawInput::NONE);
        let tick = rig.tick();
        let spent = rig
            .combat()
            .env()
            .unwrap()
            .stickers()
            .next()
            .is_some_and(|(_, s, _)| s.is_spent());
        let list = drips(rig.combat(), tick);
        if !spent {
            assert!(list.is_empty(), "no drip before the blast (tick {tick})");
            continue;
        }
        let (_, s, _) = rig.combat().env().unwrap().stickers().next().unwrap();
        let age = tick - s.detonate_tick();
        let drip = list[0];
        if seen_spent.is_none() {
            seen_spent = Some(tick);
            assert_eq!(age, 0, "the first spent tick is the detonation tick");
            assert_eq!(drip.frame, 0, "the run starts on the detonation tick");
        }
        assert_eq!(drip.frame, drip_frame(age, false));
        assert_eq!(drip.sheet, DecalSheet::RotDrip);
        assert_eq!(drip.layer, DecalLayer::Drip);
        // Hanging from the underside at the dome's column.
        let t = rot_sticker::active();
        assert_eq!(drip.x, s.dome(t).0.round() as i32 - DRIP_W / 2);
        assert_eq!(drip.y + drip.h, s.track().fixed);
        assert!(!drip.flip_x);
    }
    assert!(seen_spent.is_some(), "the sticker blew within 80 ticks");
}

/// A wall sticker's drip is the `_w` set, authored for `WallLeft`: from the face, down
/// from the dome's height; the right wall mirrors it.
#[test]
fn a_wall_drip_is_mirrored_on_the_right_wall() {
    // `on_pillar` is a WallRight sticker on the pillar's left face (x 960): Carl stands
    // 17 px from its dome and runs away (the Rot jab-and-run fixture).
    let mut rig = Layout {
        stickers: vec![Layout::on_pillar(32)],
        solids: vec![Layout::pillar()],
        carl_x: 925,
        ..Layout::default()
    }
    .sim(None);
    let mut found = None;
    for _ in 0..80 {
        rig.carl(walk(-1.0));
        let spent = rig
            .combat()
            .env()
            .unwrap()
            .stickers()
            .next()
            .is_some_and(|(_, s, _)| s.is_spent());
        if spent {
            found = drips(rig.combat(), rig.tick()).first().copied();
            break;
        }
    }
    let drip = found.expect("the wall sticker blew and left a drip");
    let (_, s, _) = rig.combat().env().unwrap().stickers().next().unwrap();
    assert_eq!(s.track().orientation, Orientation::WallRight);
    assert!(drip.flip_x, "the wall sheet is authored for the left wall");
    assert_eq!(
        drip.frame, DRIP_RUN_W,
        "the wall run starts on the detonation tick"
    );
    assert_eq!(
        drip.x + DRIP_W,
        s.track().fixed,
        "the mirrored sprite ends on the face"
    );
    let t = rot_sticker::active();
    assert_eq!(
        drip.y + drip.h,
        s.dome(t).1.round() as i32,
        "down from the dome's height"
    );
}

fn walk_event_count(rig: &Rig) -> usize {
    rig.events()
        .iter()
        .filter(|e| matches!(e, CombatEvent::HeroHurt { .. }))
        .count()
}

/// Rendering never changes authoritative state: a run that computes every decal and drip
/// on every tick has, at every tick, the state hash of a run that never does.
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
            carl_x: 560,
            ..Layout::default()
        }
        .sim(Some(true))
    };
    let (mut plain, mut drawn) = (build(), build());
    let layout_h = drawn.scene().world.layout_height();
    for k in 0..300u64 {
        let input = if k < 120 { walk(1.0) } else { RawInput::NONE };
        plain.step(input, RawInput::NONE);
        drawn.step(input, RawInput::NONE);
        let tick = drawn.tick();
        if let Some(env) = drawn.combat().env() {
            let _ = static_decals(env, layout_h, "haz_fixture");
            let _ = static_capacity(env, "haz_fixture");
        }
        let _ = drips(drawn.combat(), tick);
        assert_eq!(plain.hash(), drawn.hash(), "tick {k}");
        assert_eq!(plain.events().len(), drawn.events().len(), "tick {k}");
    }
    assert!(walk_event_count(&plain) == walk_event_count(&drawn));
}
