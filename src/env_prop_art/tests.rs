use super::*;
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::env_prop::Stimulus;
use crate::f01_boiler::BoilerRun;
use crate::input::RawInput;
use std::path::PathBuf;

const VALVE: usize = 0;
const WEIGHT: usize = 1;
const DRUM: usize = 2;

fn rig() -> (BoilerRun, SceneSim, CombatState) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate::f01_late::boiler_start(
        &root,
        360,
        crate::carl_prog::CarlProgFlags::OFF,
        crate::donut_prog::DonutProgFlags::OFF,
        None,
    )
    .expect("the Boiler launches")
}

fn step(scene: &mut SceneSim, combat: &mut CombatState) {
    combat.step_party(scene, [RawInput::NONE, RawInput::NONE]);
}

fn two_hits(combat: &mut CombatState, prop: usize) {
    for token in 1..=2 {
        combat.env_stimulate(
            prop,
            Stimulus::MeleeHit {
                token,
                player: 0,
                weight: 1,
            },
            0,
        );
    }
}

/// Set the drum off the way a kicked goblin does: a launched body, one tick on.
fn light_drum(scene: &mut SceneSim, combat: &mut CombatState) {
    let at = scene.world.tick + 1;
    combat.env_stimulate(
        DRUM,
        Stimulus::LaunchedBody {
            launch_id: 1,
            impulse_px_s: 300,
            initiator: Some(0),
        },
        at,
    );
}

fn view(run: &BoilerRun, combat: &CombatState, tick: u64) -> Vec<Piece> {
    pieces(&run.loaded.encounter, combat.env().expect("env"), tick)
}

fn sprite_view(run: &BoilerRun, combat: &CombatState, tick: u64) -> Vec<SpriteDraw> {
    sprites(
        &run.loaded.encounter,
        combat.env().expect("env"),
        &combat.charges,
        tick,
    )
}

fn has(p: &[Piece], label: &str) -> bool {
    p.iter().any(|x| x.label == label)
}

fn tone_of(p: &[Piece], label: &str) -> Option<Tone> {
    p.iter().find(|x| x.label == label).map(|x| x.tone)
}

#[test]
fn the_untouched_room_draws_its_stand_ins_and_no_warning() {
    let (run, _scene, combat) = rig();
    let p = view(&run, &combat, 0);
    for label in [
        "boiler landmark (stand-in)",
        "valve body (stand-in)",
        "valve wheel",
        "brace (stand-in)",
        "hanging weight (stand-in)",
        "weight chain",
        "fuel drum (stand-in)",
    ] {
        assert!(has(&p, label), "{label}");
    }
    assert_eq!(tone_of(&p, "valve body (stand-in)"), Some(Tone::Copper));
    assert!(
        p.iter().all(|x| x.layer != Layer::Tell),
        "nothing warns before a prop is set off"
    );
    assert!(p.len() <= MAX_PIECES, "{}", p.len());
    // Every stand-in is labelled as one (K10): no piece claims to be art.
    assert!(p.iter().all(|x| !x.label.is_empty()));
}

#[test]
fn the_landmark_stands_between_the_valve_and_the_steam_outlet_on_the_lane() {
    let (run, _scene, combat) = rig();
    let p = view(&run, &combat, 0);
    let body = p
        .iter()
        .find(|x| x.label == "boiler landmark (stand-in)")
        .unwrap()
        .rect;
    let enc = &run.loaded.encounter;
    assert_eq!(body.x, enc.props[VALVE].target.right());
    assert_eq!(body.bottom(), enc.props[VALVE].target.bottom());
    assert_eq!(body.h, BOILER_HEIGHT_PX);
    let outlet = enc.hazards[0].spec.rect.x;
    assert_eq!(body.right(), outlet, "it ends where the steam starts");
    assert_eq!(
        p.iter().find(|x| x.rect == body).map(|x| x.layer),
        Some(Layer::Back),
        "it is a backdrop, never in front of an actor"
    );
}

#[test]
fn the_valve_warns_then_vents_then_stays_ruptured_and_the_boiler_shows_it() {
    let (run, mut scene, mut combat) = rig();
    two_hits(&mut combat, VALVE);
    assert_eq!(
        combat.env().unwrap().valve_state(VALVE),
        Some(ValveState::Warning)
    );
    // The warning flashes the valve and marks the steam area, above everything.
    let a = view(&run, &combat, 0);
    let b = view(&run, &combat, FLASH_TICKS);
    assert_ne!(
        tone_of(&a, "valve body (stand-in)"),
        tone_of(&b, "valve body (stand-in)"),
        "the tell flashes"
    );
    assert!(has(&a, "steam warning area") && has(&a, "steam warning edge"));
    assert!(!has(&a, "steam plume"));
    let undamaged = tone_of(&a, "boiler landmark (stand-in)");
    // Run to the vent.
    let mut vent = None;
    let mut spent = None;
    for t in 1..=200u64 {
        step(&mut scene, &mut combat);
        let state = combat.env().unwrap().valve_state(VALVE).unwrap();
        if state == ValveState::Venting && vent.is_none() {
            vent = Some(t);
            let p = view(&run, &combat, t);
            assert!(has(&p, "steam plume"));
            assert!(
                !has(&p, "valve wheel"),
                "the wheel is gone once it ruptured"
            );
            assert!(!has(&p, "steam warning area"));
            assert_eq!(
                tone_of(&p, "boiler landmark (stand-in)"),
                Some(Tone::CopperDark)
            );
            assert_ne!(undamaged, Some(Tone::CopperDark));
            assert!(has(&p, "boiler damage notch"));
        }
        if state == ValveState::Spent {
            spent = Some(t);
            break;
        }
    }
    let (vent, spent) = (vent.expect("it vented"), spent.expect("it spent"));
    assert_eq!(spent - vent, 72, "the vent window of the sidecar");
    let p = view(&run, &combat, spent);
    assert_eq!(tone_of(&p, "valve body (stand-in)"), Some(Tone::Scorch));
    assert!(!has(&p, "steam plume") && !has(&p, "steam warning area"));
    assert!(has(&p, "boiler damage notch"), "the damage persists");
}

#[test]
fn the_weight_cracks_sways_falls_and_leaves_the_low_remnant() {
    let (run, mut scene, mut combat) = rig();
    let config = *combat.env().unwrap().mass(WEIGHT).unwrap().config();
    let hang = view(&run, &combat, 0);
    let weight_at = |p: &[Piece]| {
        p.iter()
            .find(|x| x.label == "hanging weight (stand-in)")
            .map(|x| x.rect)
    };
    let hanging = weight_at(&hang).unwrap();
    assert_eq!((hanging.y, hanging.h), (config.hanging.y, config.hanging.h));
    // One hit weakens: the brace shows a crack.
    combat.env_stimulate(
        WEIGHT,
        Stimulus::MeleeHit {
            token: 1,
            player: 0,
            weight: 1,
        },
        0,
    );
    assert_eq!(
        combat.env().unwrap().mass_state(WEIGHT),
        Some(MassState::Weakened)
    );
    let p = view(&run, &combat, 0);
    assert!(has(&p, "brace crack"));
    assert_eq!(tone_of(&p, "brace (stand-in)"), Some(Tone::Rust));
    combat.env_stimulate(
        WEIGHT,
        Stimulus::MeleeHit {
            token: 2,
            player: 0,
            weight: 1,
        },
        0,
    );
    assert_eq!(
        combat.env().unwrap().mass_state(WEIGHT),
        Some(MassState::Released)
    );
    // Released: the footprint shadow is drawn above everything, the chain sways,
    // the brace is a stump.
    let p = view(&run, &combat, 0);
    let shadow = p
        .iter()
        .find(|x| x.label == "weight footprint shadow")
        .unwrap();
    assert_eq!(shadow.layer, Layer::Tell);
    assert!(has(&p, "brace stump") && !has(&p, "brace (stand-in)"));
    let xs: Vec<i32> = [0, SWAY_TICKS, 2 * SWAY_TICKS, 3 * SWAY_TICKS]
        .iter()
        .map(|t| weight_at(&view(&run, &combat, *t)).unwrap().x)
        .collect();
    assert_eq!(xs[0], xs[2], "centre, out, centre, back");
    assert_ne!(xs[1], xs[3]);
    // Fall and land.
    let mut fell = false;
    let mut landed = false;
    for t in 1..=400u64 {
        step(&mut scene, &mut combat);
        match combat.env().unwrap().mass_state(WEIGHT).unwrap() {
            MassState::Falling => {
                let w = weight_at(&view(&run, &combat, t)).unwrap();
                assert!(w.y >= hanging.y, "it only goes down");
                fell |= w.y > hanging.y;
                assert!(!has(&view(&run, &combat, t), "weight chain"));
            }
            MassState::Landed => {
                let p = view(&run, &combat, t);
                assert_eq!(weight_at(&p), None, "the mass became the remnant");
                let remnant = p
                    .iter()
                    .find(|x| x.label == "landed remnant (stand-in)")
                    .unwrap();
                assert_eq!(remnant.rect, config.remnant);
                assert_eq!(remnant.tone, Tone::Rubble);
                landed = true;
                break;
            }
            _ => {}
        }
    }
    assert!(fell && landed);
}

#[test]
fn the_drum_flashes_its_area_tell_then_leaves_a_scorch() {
    let (run, mut scene, mut combat) = rig();
    let t = run.loaded.encounter.props[DRUM].target;
    light_drum(&mut scene, &mut combat);
    let mut told = false;
    let mut blown = false;
    for tick in 1..=200u64 {
        step(&mut scene, &mut combat);
        match combat.env().unwrap().drum_state(DRUM).unwrap() {
            DrumState::Telegraph => {
                let p = view(&run, &combat, tick);
                assert!(has(&p, "drum blast tell"), "the area tell is drawn");
                assert!(
                    p.iter()
                        .filter(|x| x.label == "drum blast tell")
                        .all(|x| x.layer == Layer::Tell)
                );
                let a = tone_of(&view(&run, &combat, 0), "fuel drum (stand-in)");
                let b = tone_of(&view(&run, &combat, FLASH_TICKS), "fuel drum (stand-in)");
                assert_ne!(a, b, "the drum flashes");
                told = true;
            }
            DrumState::Detonated => {
                let p = view(&run, &combat, tick);
                assert!(!has(&p, "fuel drum (stand-in)"), "the drum is gone");
                let scorch = p
                    .iter()
                    .find(|x| x.label == "drum scorch (stand-in)")
                    .unwrap();
                assert_eq!(scorch.tone, Tone::Scorch);
                assert_eq!(scorch.layer, Layer::Back, "under the blast");
                assert!(scorch.rect.bottom() <= t.bottom(), "flat on the lane");
                assert!(!has(&p, "drum blast tell"));
                blown = true;
                break;
            }
            DrumState::Armed => {}
        }
    }
    assert!(told && blown);
}

#[test]
fn the_layers_are_ordered_and_a_warning_is_never_under_a_prop() {
    let (run, mut scene, mut combat) = rig();
    two_hits(&mut combat, VALVE);
    two_hits(&mut combat, WEIGHT);
    for t in 0..120u64 {
        step(&mut scene, &mut combat);
        let p = view(&run, &combat, t);
        assert!(
            p.windows(2).all(|w| w[0].layer <= w[1].layer),
            "back to front"
        );
        let first_tell = p.iter().position(|x| x.layer == Layer::Tell);
        if let Some(at) = first_tell {
            assert!(p[at..].iter().all(|x| x.layer == Layer::Tell));
        }
    }
}

/// The whole chain, every tick: every piece is a real rectangle inside the room,
/// the count stays under the pool, and the same state at the same tick draws the
/// same pieces (a replay seek or a skipped frame draws the same picture).
#[test]
fn every_piece_of_every_state_is_inside_the_room_and_a_pure_function_of_state_and_tick() {
    let (run, mut scene, mut combat) = rig();
    let width = run.kit.width;
    two_hits(&mut combat, VALVE);
    two_hits(&mut combat, WEIGHT);
    light_drum(&mut scene, &mut combat);
    let mut labels = std::collections::BTreeSet::new();
    for t in 0..360u64 {
        step(&mut scene, &mut combat);
        let p = view(&run, &combat, t);
        assert_eq!(p, view(&run, &combat, t), "pure at tick {t}");
        assert!(p.len() <= MAX_PIECES, "{} pieces at {t}", p.len());
        for x in &p {
            assert!(x.rect.w > 0 && x.rect.h > 0, "{} at {t}", x.label);
            assert!(
                x.rect.x >= 0 && x.rect.right() <= width,
                "{} {:?} at {t}",
                x.label,
                x.rect
            );
            labels.insert(x.label);
        }
    }
    for label in [
        "steam plume",
        "weight footprint shadow",
        "landed remnant (stand-in)",
        "drum scorch (stand-in)",
    ] {
        assert!(labels.contains(label), "{label} never drawn: {labels:?}");
    }
}

/// Presentation only: the pieces and sprites are read from a shared reference, so drawing
/// them every tick cannot change the state hash. Compared on the inert room and
/// on the live roster, through the valve, the weight and the drum chain.
#[test]
fn drawing_the_pieces_and_sprites_never_changes_the_state_hash() {
    for roster in [
        crate::f01_boiler::Roster::Inert,
        crate::f01_boiler::Roster::Live,
    ] {
        let make = || {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            crate::f01_late::boiler_start_with(
                &root,
                360,
                crate::carl_prog::CarlProgFlags::OFF,
                crate::donut_prog::DonutProgFlags::OFF,
                None,
                roster,
            )
            .unwrap()
        };
        let (run, mut scene, mut combat) = make();
        let (_r2, mut scene2, mut combat2) = make();
        for c in [&mut combat, &mut combat2] {
            two_hits(c, VALVE);
            two_hits(c, WEIGHT);
        }
        light_drum(&mut scene, &mut combat);
        light_drum(&mut scene2, &mut combat2);
        for t in 0..300u64 {
            let _ = view(&run, &combat, t);
            let _ = sprite_view(&run, &combat, t);
            step(&mut scene, &mut combat);
            step(&mut scene2, &mut combat2);
            assert_eq!(
                combat.state_hash(&scene),
                combat2.state_hash(&scene2),
                "{roster:?} tick {t}"
            );
        }
    }
}

#[test]
fn the_palette_covers_every_tone_and_steam_is_translucent() {
    for tone in [
        Tone::Copper,
        Tone::CopperDark,
        Tone::Iron,
        Tone::Rubble,
        Tone::Rust,
        Tone::Warn,
        Tone::Scorch,
        Tone::Steam,
        Tone::SteamHint,
        Tone::Shadow,
    ] {
        let [_, _, _, a] = tone.rgba();
        let translucent = matches!(tone, Tone::Steam | Tone::SteamHint | Tone::Shadow);
        assert_eq!(a < 0xff, translucent, "{tone:?}");
    }
}

/// BOIL-E: the death presenter's origin of an env blow is the centre of the
/// owning prop's hazard volume (steam 688..768, footprint 784..848, ring 816..912).
#[test]
fn the_hazard_origins_are_the_volume_centres_by_prop_and_class() {
    use crate::env_hazard::HazardClass::{Blast, FallingMass, Steam};
    let (_run, _scene, combat) = rig();
    let mut got = combat.env().unwrap().hazard_origins();
    got.sort_by_key(|(prop, _, _)| *prop);
    assert_eq!(
        got,
        [
            (0, Steam, 728.0),
            (1, FallingMass, 816.0),
            (2, Blast, 864.0)
        ]
    );
}

// ------------------------------------------------- the Goblin Workshop kit ----

fn manifest(sheet: Sheet) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/manifests")
        .join(format!("{}.json", sheet.manifest()));
    serde_json::from_str(&std::fs::read_to_string(&path).expect("the manifest exists"))
        .expect("the manifest parses")
}

/// The tag table of the code is the tag table of the committed manifests: names,
/// first frame, frame count, ticks per frame, frame size and atlas path.
#[test]
fn the_tag_table_matches_the_manifests() {
    for sheet in Sheet::ALL {
        let m = manifest(sheet);
        assert_eq!(m["output_atlas"], sheet.atlas(), "{sheet:?}");
        let size = m["frame_size"].as_array().unwrap();
        let (w, h) = sheet.frame_size();
        assert_eq!(
            (size[0].as_i64().unwrap(), size[1].as_i64().unwrap()),
            (i64::from(w), i64::from(h)),
            "{sheet:?} frame size"
        );
        let frames = m["prop_kit"]["frames"].as_array().unwrap();
        assert_eq!(frames.len(), sheet.frames(), "{sheet:?} frame count");
        let names: Vec<&str> = m["animation_tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            sheet.tags().iter().map(|t| t.name).collect::<Vec<_>>(),
            "{sheet:?} tags"
        );
        for tag in sheet.tags() {
            let rows: Vec<&serde_json::Value> =
                frames.iter().filter(|f| f["tag"] == tag.name).collect();
            assert_eq!(rows.len(), tag.frames, "{} frames", tag.name);
            for (k, row) in rows.iter().enumerate() {
                assert_eq!(row["index"].as_u64().unwrap() as usize, tag.first + k);
                assert_eq!(
                    row["ticks"].as_u64().unwrap_or(0),
                    tag.ticks,
                    "{} ticks",
                    tag.name
                );
                if tag.frames > 1 {
                    assert_eq!(
                        row["duration_ms"].as_u64().unwrap(),
                        (tag.ticks * 1000 + 30) / 60,
                        "{} duration",
                        tag.name
                    );
                }
            }
        }
    }
}

/// Every sprite of every state of the whole chain names a real frame of a real
/// tag, stays inside the room, and the list is a pure function of state and tick.
#[test]
fn every_state_maps_to_an_existing_tag_and_the_sprite_list_is_pure() {
    let (run, mut scene, mut combat) = rig();
    let width = run.kit.width;
    let mut seen: std::collections::BTreeSet<(Sheet, &'static str)> = Default::default();
    // The untouched room first (the idle loop), then the whole chain.
    for t in 0..40u64 {
        for s in sprite_view(&run, &combat, t) {
            let tag = s
                .sheet
                .tags()
                .iter()
                .find(|g| (g.first..g.first + g.frames).contains(&s.frame))
                .expect("a tag");
            seen.insert((s.sheet, tag.name));
        }
    }
    // One hit weakens the brace (the creak loop), the second releases the weight.
    let hit = |combat: &mut CombatState, token| {
        combat.env_stimulate(
            WEIGHT,
            Stimulus::MeleeHit {
                token,
                player: 0,
                weight: 1,
            },
            0,
        );
    };
    hit(&mut combat, 1);
    for t in 0..24u64 {
        for s in sprite_view(&run, &combat, t) {
            let tag = s
                .sheet
                .tags()
                .iter()
                .find(|g| (g.first..g.first + g.frames).contains(&s.frame))
                .expect("a tag");
            seen.insert((s.sheet, tag.name));
        }
    }
    hit(&mut combat, 2);
    two_hits(&mut combat, VALVE);
    light_drum(&mut scene, &mut combat);
    for t in 0..400u64 {
        step(&mut scene, &mut combat);
        let list = sprite_view(&run, &combat, t);
        assert_eq!(list, sprite_view(&run, &combat, t), "pure at tick {t}");
        assert!(list.len() <= MAX_SPRITES, "{} sprites at {t}", list.len());
        assert!(list.windows(2).all(|w| w[0].layer <= w[1].layer));
        for s in &list {
            let tag = s
                .sheet
                .tags()
                .iter()
                .find(|g| (g.first..g.first + g.frames).contains(&s.frame))
                .unwrap_or_else(|| panic!("{:?} frame {} is in no tag", s.sheet, s.frame));
            seen.insert((s.sheet, tag.name));
            assert!(s.frame < s.sheet.frames());
            let (w, h) = s.sheet.frame_size();
            assert_eq!((s.rect.w, s.rect.h), (w, h));
            assert!(s.rect.x >= 0 && s.rect.right() <= width, "{s:?} at {t}");
            assert!(s.rect.y >= 0, "{s:?} at {t}");
        }
    }
    for (sheet, name) in [
        (Sheet::Boiler, "boiler_idle"),
        (Sheet::Boiler, "boiler_strain"),
        (Sheet::Boiler, "boiler_damaged"),
        (Sheet::Valve, "valve_warning"),
        (Sheet::Valve, "valve_venting"),
        (Sheet::Valve, "valve_spent"),
        (Sheet::Brace, "brace_weakened"),
        (Sheet::Brace, "brace_stump"),
        (Sheet::Weight, "weight_fall"),
        (Sheet::Remnant, "remnant"),
        (Sheet::Rig, "rig_run"),
        (Sheet::Chain, "chain"),
        (Sheet::Drum, "drum_telegraph"),
        (Sheet::Scorch, "scorch"),
        (Sheet::Sputter, "steam_sputter"),
        (Sheet::Plume, "steam_plume"),
        (Sheet::Shadow, "fall_shadow_tell"),
        (Sheet::Shadow, "fall_shadow"),
        (Sheet::Ring, "drum_blast_tell"),
    ] {
        assert!(seen.contains(&(sheet, name)), "{name} never drawn");
    }
}

fn first(list: &[SpriteDraw], sheet: Sheet) -> Option<SpriteDraw> {
    list.iter().copied().find(|s| s.sheet == sheet)
}

/// The anchors of the spec table (section 3) match the encounter's rectangles.
#[test]
fn the_anchors_match_the_encounter_rects() {
    let (run, mut scene, mut combat) = rig();
    let enc = &run.loaded.encounter;
    let list = sprite_view(&run, &combat, 0);
    let at = |s: Sheet| first(&list, s).map(|d| (d.rect.x, d.rect.y)).unwrap();
    assert_eq!(at(Sheet::Boiler), (496, 372));
    assert_eq!(at(Sheet::Valve), (464, 532));
    assert_eq!(at(Sheet::Brace), (744, 540));
    assert_eq!(at(Sheet::Weight), (784, 404));
    assert_eq!(at(Sheet::Drum), (846, 544));
    assert_eq!(at(Sheet::Rig), (744, 352));
    // The art stands on the same base lines as the rects it replaces.
    let bottom = |s: Sheet| first(&list, s).unwrap().rect.bottom();
    assert_eq!(bottom(Sheet::Boiler), enc.props[VALVE].target.bottom());
    assert_eq!(bottom(Sheet::Valve), enc.props[VALVE].target.bottom());
    assert_eq!(bottom(Sheet::Brace), enc.props[WEIGHT].target.bottom());
    assert_eq!(bottom(Sheet::Drum), enc.props[DRUM].target.bottom());
    let mass = combat
        .env()
        .unwrap()
        .mass(WEIGHT)
        .unwrap()
        .mass_rect()
        .unwrap();
    let weight = first(&list, Sheet::Weight).unwrap().rect;
    assert_eq!((weight.x, weight.bottom()), (mass.x, mass.bottom()));
    // The chains: brace chain x 758 and weight chain x 814, from y 376.
    let chains: Vec<&SpriteDraw> = list.iter().filter(|s| s.sheet == Sheet::Chain).collect();
    assert!(chains.iter().any(|c| c.rect.x == 758 && c.rect.y == 376));
    assert!(chains.iter().any(|c| c.rect.x == 814 && c.rect.y == 376));
    assert!(chains.iter().all(|c| c.layer == Layer::Back));
    // The tells sit on their rects once set off.
    two_hits(&mut combat, VALVE);
    two_hits(&mut combat, WEIGHT);
    light_drum(&mut scene, &mut combat);
    let steam = enc.hazards[0].spec.rect;
    let mut saw = [false; 5];
    for t in 0..400u64 {
        step(&mut scene, &mut combat);
        let list = sprite_view(&run, &combat, t);
        if let Some(d) = first(&list, Sheet::Sputter) {
            assert_eq!((d.rect.x, d.rect.y), (steam.x, steam.y + SPUTTER_DY));
            assert_eq!(d.alpha, 255);
            saw[0] = true;
        }
        if let Some(d) = first(&list, Sheet::Plume) {
            assert_eq!(d.rect, steam, "the plume covers the lane rect");
            assert_eq!(d.alpha, STEAM_ALPHA);
            saw[1] = true;
        }
        if let Some(d) = first(&list, Sheet::Shadow) {
            assert_eq!((d.rect.x, d.rect.y), (784, 572));
            assert_eq!(d.alpha, SHADOW_ALPHA);
            saw[2] = true;
        }
        if let Some(d) = first(&list, Sheet::Ring) {
            assert_eq!((d.rect.x, d.rect.y), (816, 572));
            saw[3] = true;
        }
        if let Some(d) = first(&list, Sheet::Remnant) {
            assert_eq!((d.rect.x, d.rect.y), (784, 548));
            saw[4] = true;
        }
        if let Some(d) = first(&list, Sheet::Scorch) {
            assert_eq!((d.rect.x, d.rect.y), (844, 572));
        }
    }
    assert!(saw.iter().all(|s| *s), "{saw:?}");
}

/// The chains hang only while their end does: the brace chain while the brace
/// stands, the weight chain until the weight falls; none after the landing.
#[test]
fn the_chains_follow_the_brace_and_the_weight() {
    let (run, mut scene, mut combat) = rig();
    let count = |list: &[SpriteDraw]| list.iter().filter(|s| s.sheet == Sheet::Chain).count();
    let hung = count(&sprite_view(&run, &combat, 0));
    assert!(hung > 20 && hung <= MAX_SPRITES - 8, "{hung}");
    two_hits(&mut combat, WEIGHT);
    let mut falling_seen = false;
    for t in 1..=400u64 {
        step(&mut scene, &mut combat);
        let list = sprite_view(&run, &combat, t);
        match combat.env().unwrap().mass_state(WEIGHT).unwrap() {
            MassState::Falling => {
                assert_eq!(count(&list), 0, "no chain while it falls");
                assert_eq!(
                    first(&list, Sheet::Weight).unwrap().frame,
                    WEIGHT_FALL.first
                );
                falling_seen = true;
            }
            MassState::Landed => {
                assert_eq!(count(&list), 0);
                assert!(first(&list, Sheet::Weight).is_none());
                assert_eq!(first(&list, Sheet::Rig).unwrap().frame, RIG_RUN.last());
                break;
            }
            MassState::Released => {
                assert_eq!(count(&list), hung - brace_chain_tiles(&run));
                assert!(RIG_RUN.frame_at(t) >= RIG_RUN.first);
            }
            _ => {}
        }
    }
    assert!(falling_seen);
}

fn brace_chain_tiles(run: &BoilerRun) -> usize {
    let brace = run.loaded.encounter.props[WEIGHT].target;
    ((brace.y - CHAIN_TOP_Y) as usize).div_ceil(8)
}

/// The placeholder rule: a piece that stands in for a sheet names it, and the
/// pieces that name none are exactly the footprints that stay drawn with the art.
#[test]
fn only_the_footprints_have_no_sheet() {
    let (run, mut scene, mut combat) = rig();
    two_hits(&mut combat, VALVE);
    two_hits(&mut combat, WEIGHT);
    light_drum(&mut scene, &mut combat);
    for t in 0..400u64 {
        step(&mut scene, &mut combat);
        for p in view(&run, &combat, t) {
            let footprint = matches!(
                p.label,
                "steam warning area" | "steam warning edge" | "drum blast tell"
            );
            assert_eq!(p.sheet().is_none(), footprint, "{} at {t}", p.label);
        }
    }
}

/// The kit covers every prop the Boiler draws: with every atlas present the
/// untouched room draws a sprite for the landmark, valve, brace, weight, drum,
/// rig and both chains, and nothing in it is a "stand-in" without a sheet.
#[test]
fn the_untouched_room_is_drawn_by_the_kit() {
    let (run, _scene, combat) = rig();
    let list = sprite_view(&run, &combat, 0);
    for sheet in [
        Sheet::Boiler,
        Sheet::Valve,
        Sheet::Brace,
        Sheet::Weight,
        Sheet::Drum,
        Sheet::Rig,
        Sheet::Chain,
    ] {
        assert!(first(&list, sheet).is_some(), "{sheet:?}");
    }
    assert!(list.iter().all(|s| s.layer != Layer::Tell));
    for p in view(&run, &combat, 0) {
        assert!(p.sheet().is_some(), "{} has no sheet", p.label);
    }
}

#[test]
fn a_tag_loops_on_the_tick_and_a_single_frame_holds() {
    assert_eq!(BOILER_IDLE.frame_at(0), 0);
    assert_eq!(BOILER_IDLE.frame_at(8), 1);
    assert_eq!(BOILER_IDLE.frame_at(31), 3);
    assert_eq!(BOILER_IDLE.frame_at(32), 0, "the loop wraps");
    assert_eq!(VALVE_WARNING.frame_at(FLASH_TICKS - 1), VALVE_WARNING.first);
    assert_eq!(
        VALVE_WARNING.frame_at(FLASH_TICKS),
        VALVE_WARNING.first + 1,
        "the warning swaps every FLASH_TICKS"
    );
    assert_eq!(REMNANT.frame_at(12345), 0);
    assert_eq!(RIG_RUN.last(), 2);
    // A tag played once from its event: 10 ticks a frame, then the last holds.
    assert_eq!(SCORCH.frame_once(0), 0);
    assert_eq!(SCORCH.frame_once(9), 0);
    assert_eq!(SCORCH.frame_once(10), 1);
    assert_eq!(SCORCH.frame_once(29), 2);
    assert_eq!(SCORCH.frame_once(10_000), SCORCH.last());
    assert_eq!(REMNANT.frame_once(50), 0);
}

/// The scorch's embers fade from the drum charge's own detonation tick: the key
/// (last embers) from the blast tick until the VFX-01 burst over it has ended, then
/// 10 ticks a frame to the cold frame, which holds. Without the charges (no known
/// tick) the cold frame holds at once.
#[test]
fn the_scorch_embers_fade_from_the_drum_detonation_tick() {
    let (run, mut scene, mut combat) = rig();
    light_drum(&mut scene, &mut combat);
    let mut blast = None;
    for _ in 0..300 {
        step(&mut scene, &mut combat);
        if combat.env().unwrap().drum_state(DRUM) == Some(DrumState::Detonated) {
            let charge = combat
                .env()
                .unwrap()
                .drum_charge(DRUM as u16)
                .expect("the drum's charge");
            blast = Some(combat.charges[charge].detonate_tick);
            break;
        }
    }
    let at = blast.expect("the drum blew");
    let frame = |t: u64| {
        first(&sprite_view(&run, &combat, t), Sheet::Scorch)
            .unwrap()
            .frame
    };
    assert_eq!(
        frame(at),
        SCORCH.first,
        "the key, with its embers, on the blast tick"
    );
    let burst = crate::death_present::burst_ticks();
    assert_eq!(frame(at + burst - 1), SCORCH.first, "hot under the burst");
    assert_eq!(frame(at + burst + 9), SCORCH.first);
    assert_eq!(frame(at + burst + 10), SCORCH.first + 1);
    assert_eq!(frame(at + burst + 20), SCORCH.last());
    assert_eq!(frame(at + 500), SCORCH.last(), "the cold scorch holds");
    // A floor scar: on `Back`, so the presenter keeps it under the blast's fireball
    // and wave (the Prop z is above the blast layer).
    let scorch = first(&sprite_view(&run, &combat, at), Sheet::Scorch).unwrap();
    assert_eq!(scorch.layer, Layer::Back);
    let blind = sprites(&run.loaded.encounter, combat.env().unwrap(), &[], at);
    assert_eq!(first(&blind, Sheet::Scorch).unwrap().frame, SCORCH.last());
}

/// RING-A/B wire (KR17): the court's static closed shutter has a labelled
/// greybox stand-in over its whole closed rectangle (with no sheet, so it always
/// draws unless the scene shows the stone shutter instead: the scene skips it
/// while the ring atlas is loaded), the split draws nothing, and drawing never
/// changes the state hash.
#[test]
fn the_static_shutter_draws_its_greybox_stand_in_and_the_hash_stays() {
    use crate::f01_ring::{GateMode, RingRoom, combat_of, launch, launch_mode};
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // RING-C wire: the static closed gate is the `--ring-static-gate` mode.
    let run = launch_mode(&root, RingRoom::Court, true, GateMode::Static).unwrap();
    let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
    let combat = combat_of(&run, &mut scene).unwrap();
    let before = combat.state_hash(&scene);
    let env = combat.env().unwrap();
    let drawn = pieces(&run.loaded.encounter, env, 0);
    let slab: Vec<_> = drawn
        .iter()
        .filter(|p| p.label == crate::ring_shutter_art::STANDIN_LABEL)
        .collect();
    assert_eq!(slab.len(), 1);
    assert_eq!(slab[0].rect, run.shutter().unwrap());
    assert!(
        slab[0].sheet().is_none(),
        "no atlas: the stand-in always draws"
    );
    assert_eq!(drawn.len(), 1, "the slab only");
    assert!(sprites(&run.loaded.encounter, env, &[], 0).is_empty());
    assert_eq!(combat.state_hash(&scene), before);
    let split = launch(&root, RingRoom::Split).unwrap();
    let mut scene = SceneSim::new_at(&split.kit, 360, split.fixture.carl_x).unwrap();
    let combat = combat_of(&split, &mut scene).unwrap();
    assert!(pieces(&split.loaded.encounter, combat.env().unwrap(), 0).is_empty());
}
