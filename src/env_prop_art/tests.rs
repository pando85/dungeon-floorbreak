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

/// Presentation only: the pieces are read from a shared reference, so drawing
/// them every tick cannot change the state hash. Compared on the inert room and
/// on the live roster, through the valve, the weight and the drum chain.
#[test]
fn drawing_the_pieces_never_changes_the_state_hash() {
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
            let _ = labels::tags(&view(&run, &combat, t));
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

// ----------------------------------------------------------- K10 labels ----

fn texts(p: &[Piece]) -> Vec<&'static str> {
    labels::tags(p).iter().map(|t| t.text).collect()
}

/// K10: every code-drawn stand-in carries a visible "STAND-IN: <name>" label,
/// each piece whose label says "(stand-in)" has one, and the part pieces do not.
#[test]
fn every_stand_in_prop_gets_a_visible_stand_in_label() {
    let (run, _scene, combat) = rig();
    let p = view(&run, &combat, 0);
    assert_eq!(
        texts(&p),
        [
            "STAND-IN: BOILER",
            "STAND-IN: VALVE",
            "STAND-IN: BRACE",
            "STAND-IN: WEIGHT",
            "STAND-IN: DRUM",
        ]
    );
    // The mapping covers every "(stand-in)" piece label the module can draw.
    for (label, text) in labels::STAND_IN_TEXTS {
        assert!(text.starts_with("STAND-IN: "), "{text}");
        assert!(
            label.ends_with("(stand-in)") || label == "brace stump",
            "{label}"
        );
    }
    for label in [
        "boiler landmark (stand-in)",
        "valve body (stand-in)",
        "brace (stand-in)",
        "hanging weight (stand-in)",
        "landed remnant (stand-in)",
        "fuel drum (stand-in)",
        "drum scorch (stand-in)",
    ] {
        assert!(
            labels::STAND_IN_TEXTS.iter().any(|(l, _)| *l == label),
            "{label}"
        );
    }
    // A part piece (a band, a chain, a wheel) never gets a label of its own.
    for piece in p.iter().filter(|x| !x.label.contains("(stand-in)")) {
        assert!(
            labels::tags(std::slice::from_ref(piece)).is_empty(),
            "{}",
            piece.label
        );
    }
}

/// The labels follow the chain: the broken brace keeps its label on the stump,
/// the landed weight is labelled as the remnant and the spent drum as a scorch,
/// every label is inside the room and the pool is never exceeded.
#[test]
fn the_labels_follow_the_props_through_the_whole_chain() {
    let (run, mut scene, mut combat) = rig();
    let width = run.kit.width;
    two_hits(&mut combat, VALVE);
    two_hits(&mut combat, WEIGHT);
    light_drum(&mut scene, &mut combat);
    let mut seen = std::collections::BTreeSet::new();
    for t in 0..360u64 {
        step(&mut scene, &mut combat);
        let p = view(&run, &combat, t);
        let tags = labels::tags(&p);
        assert!(tags.len() <= labels::MAX_TAGS);
        assert_eq!(tags, labels::tags(&p), "pure at tick {t}");
        let texts: Vec<&str> = tags.iter().map(|x| x.text).collect();
        let unique: std::collections::BTreeSet<&str> = texts.iter().copied().collect();
        assert_eq!(texts.len(), unique.len(), "one label per text at {t}");
        for (i, a) in tags.iter().enumerate() {
            for b in &tags[i + 1..] {
                assert!(
                    !a.overlaps(b),
                    "{} and {} overlap at tick {t}: {:?} {:?}",
                    a.text,
                    b.text,
                    a.ink_box(),
                    b.ink_box()
                );
            }
        }
        for tag in &tags {
            assert!(
                tag.centre_x >= 0 && tag.centre_x <= width,
                "{} at {t}",
                tag.text
            );
            seen.insert(tag.text);
        }
    }
    for text in [
        "STAND-IN: BOILER",
        "STAND-IN: VALVE",
        "STAND-IN: BRACE",
        "STAND-IN: WEIGHT",
        "STAND-IN: RUBBLE",
        "STAND-IN: DRUM",
        "STAND-IN: SCORCH",
    ] {
        assert!(seen.contains(text), "{text} never shown: {seen:?}");
    }
}

/// A label sits above its piece (the landmark's inside, below the dome) and
/// centred on it.
#[test]
fn a_label_is_centred_on_its_piece_and_sits_above_it() {
    let (run, _scene, combat) = rig();
    let p = view(&run, &combat, 0);
    let tags = labels::tags(&p);
    for (label, text) in labels::STAND_IN_TEXTS {
        let (Some(piece), Some(tag)) = (
            p.iter().find(|x| x.label == label),
            tags.iter().find(|t| t.text == text),
        ) else {
            continue;
        };
        assert_eq!(tag.centre_x, piece.rect.x + piece.rect.w / 2, "{text}");
        if label == "boiler landmark (stand-in)" {
            assert_eq!(tag.bottom_y, piece.rect.y + labels::LANDMARK_TAG_DROP_PX);
        } else {
            assert_eq!(tag.bottom_y, piece.rect.y - 1, "{text}");
        }
    }
}

/// The 3 x 5 font draws every label text: ink and shadow only, the right size,
/// and a character without a glyph is refused rather than drawn blank.
#[test]
fn every_label_text_rasterises_in_the_pixel_font() {
    for (_, text) in labels::STAND_IN_TEXTS {
        let img = labels::text_pixels(text).unwrap_or_else(|| panic!("{text}"));
        assert_eq!(
            img.w,
            text.chars().count() as u32 * labels::GLYPH_ADVANCE_PX
        );
        assert_eq!(img.h, labels::GLYPH_HEIGHT_PX + 1);
        assert_eq!(img.rgba.len(), (img.w * img.h * 4) as usize);
        let px = |i: usize| {
            [
                img.rgba[i * 4],
                img.rgba[i * 4 + 1],
                img.rgba[i * 4 + 2],
                img.rgba[i * 4 + 3],
            ]
        };
        let ink = (0..(img.w * img.h) as usize)
            .filter(|&i| px(i) == labels::TAG_INK)
            .count();
        let shadow = (0..(img.w * img.h) as usize)
            .filter(|&i| px(i) == labels::TAG_SHADOW)
            .count();
        assert!(ink > text.len() * 2, "{text}: {ink} ink pixels");
        assert!(shadow > 0, "{text}");
        assert!(
            (0..(img.w * img.h) as usize)
                .all(|i| px(i) == labels::TAG_INK || px(i) == labels::TAG_SHADOW || px(i)[3] == 0),
            "{text}: only ink, shadow and clear pixels"
        );
    }
    assert!(labels::text_pixels("").is_none());
    assert!(labels::text_pixels("STAND-IN: @").is_none());
    // The shadow never covers ink: the first glyph's top-left ink pixel is ink.
    let t = labels::text_pixels("S").unwrap();
    assert_eq!(
        &t.rgba[4..8],
        &labels::TAG_INK,
        "row 0, column 1 of S is ink"
    );
}

/// Labels that would cross are lifted apart, deterministically and in piece
/// order; a label that does not cross anything stays where it was put.
#[test]
fn overlapping_labels_are_lifted_apart_in_piece_order() {
    let piece = |label: &'static str, x: i32, y: i32| Piece {
        rect: IRect { x, y, w: 16, h: 16 },
        tone: Tone::Iron,
        layer: Layer::Prop,
        label,
    };
    // The remnant and the drum stand side by side: their 64 px labels cross.
    let p = [
        piece("landed remnant (stand-in)", 100, 200),
        piece("fuel drum (stand-in)", 120, 200),
        piece("brace (stand-in)", 110, 200),
    ];
    let tags = labels::tags(&p);
    assert_eq!(
        texts(&p),
        ["STAND-IN: RUBBLE", "STAND-IN: DRUM", "STAND-IN: BRACE"]
    );
    assert_eq!(tags[0].bottom_y, 199, "the first keeps its place");
    assert_eq!(
        tags[1].bottom_y,
        199 - labels::LIFT_PX,
        "the second is lifted once"
    );
    assert_eq!(
        tags[2].bottom_y,
        199 - 2 * labels::LIFT_PX,
        "the third twice"
    );
    for (i, a) in tags.iter().enumerate() {
        for b in &tags[i + 1..] {
            assert!(!a.overlaps(b), "{} / {}", a.text, b.text);
        }
    }
    assert_eq!(tags, labels::tags(&p), "deterministic");
    // Far apart: nothing moves.
    let apart = [
        piece("landed remnant (stand-in)", 0, 200),
        piece("fuel drum (stand-in)", 300, 200),
    ];
    assert!(labels::tags(&apart).iter().all(|t| t.bottom_y == 199));
    // Touching boxes do not count as overlapping (the gap is the lift's own line).
    let a = labels::Tag {
        text: "STAND-IN: DRUM",
        centre_x: 100,
        bottom_y: 50,
    };
    let b = labels::Tag {
        text: "STAND-IN: DRUM",
        centre_x: 100,
        bottom_y: 50 - 6,
    };
    assert!(!a.overlaps(&b) && a.overlaps(&a));
}

/// The chain that overlapped on screen (the review of #254): the weight falls
/// and lands while the drum still stands, so the RUBBLE and DRUM labels are
/// both due beside each other. They must be shown together and never cross.
#[test]
fn the_rubble_and_drum_labels_are_shown_together_without_crossing() {
    let (run, mut scene, mut combat) = rig();
    two_hits(&mut combat, WEIGHT);
    let mut together = 0;
    for t in 0..400u64 {
        step(&mut scene, &mut combat);
        let tags = labels::tags(&view(&run, &combat, t));
        let rubble = tags.iter().find(|x| x.text == "STAND-IN: RUBBLE");
        let drum = tags.iter().find(|x| x.text == "STAND-IN: DRUM");
        if let (Some(r), Some(d)) = (rubble, drum) {
            together += 1;
            assert!(
                !r.overlaps(d),
                "tick {t}: {:?} {:?}",
                r.ink_box(),
                d.ink_box()
            );
        }
    }
    assert!(
        together > 0,
        "the landed weight and the standing drum never coexisted"
    );
}
