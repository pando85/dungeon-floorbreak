use super::*;
use crate::combat::CombatEvent;
use crate::hoarder_fixtures::{
    ClearPilot, Control, IDLE as IDLE_SLOTS, Plan, Rig, attack_input, carl_input, room_run,
};
use crate::pile_boss::PileEvent;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn art_of(rig: &Rig, tick: u64, look: HoarderLook) -> HoarderArt {
    draw(
        &rig.combat,
        &rig.run.loaded.encounter,
        rig.scene.world.layout_height(),
        tick,
        look,
    )
    .expect("her room")
}

const MOUND_LABEL: &str = "lair mound (low backdrop dressing, no collision)";

fn labels(art: &HoarderArt) -> Vec<&'static str> {
    art.pieces.iter().map(|p| p.label).collect()
}

/// The sheet's stand-in table (section 4.1, 4.2, 4.3, 4.4), mode by mode, and the swipe's
/// 8-tick gather then rear-up hold.
#[test]
fn the_frame_table_follows_the_sheets_stand_in_rows() {
    let look = HoarderLook::default();
    let f = |mode, mt| frame_for(mode, mt, 0, look);
    // Swipe: gather 7 (attack ticks 0..7 = mode ticks 1..8), rear-up 8, smear 9, impact 10,
    // recover 11.
    for mt in 1..=8 {
        assert_eq!(f(PileMode::SwipeTell, mt), 7, "gather at mode tick {mt}");
    }
    for mt in [9, 20, 34] {
        assert_eq!(f(PileMode::SwipeTell, mt), 8, "rear-up at mode tick {mt}");
    }
    assert_eq!(f(PileMode::SwipeSmear, 1), 9);
    assert_eq!(f(PileMode::SwipeImpact, 1), 10);
    assert_eq!(f(PileMode::SwipeRecover, 1), 11);
    // Shove: the tell on the gather frame, the lunge on the smear frame, recover 11; a jam
    // shows the hurt frame for 12 ticks, then the recovery.
    assert_eq!(f(PileMode::ShoveTell, 1), 7);
    assert_eq!(f(PileMode::ShoveTell, 40), 7);
    assert_eq!(f(PileMode::Lunge, 5), 9);
    assert_eq!(f(PileMode::ShoveRecover, 1), 11);
    assert_eq!(f(PileMode::Jammed, 1), 12);
    assert_eq!(f(PileMode::Jammed, 12), 12);
    assert_eq!(f(PileMode::Jammed, 13), 11);
    // Flinch 12; Exposed and Get-up hold the recovery frame, alive with eyes open.
    assert_eq!(f(PileMode::Flinch, 1), 12);
    assert_eq!(f(PileMode::Exposed, 1), 11);
    assert_eq!(f(PileMode::GetUp, 1), 11);
    // The idle loop is 16, 12, 16 ticks; the walk loop 8 ticks a frame.
    let idle: Vec<u8> = (0..44)
        .map(|t| frame_for(PileMode::Neutral, 1, t, look))
        .collect();
    assert_eq!(idle[0], 0);
    assert_eq!(idle[15], 0);
    assert_eq!(idle[16], 1);
    assert_eq!(idle[27], 1);
    assert_eq!(idle[28], 2);
    assert_eq!(idle[43], 2);
    assert_eq!(
        frame_for(PileMode::Neutral, 1, 44, look),
        0,
        "the loop repeats"
    );
    let walk: Vec<u8> = (0..32)
        .step_by(8)
        .map(|t| frame_for(PileMode::Walk, 1, t, look))
        .collect();
    assert_eq!(walk, [3, 4, 5, 6]);
}

/// The `down` frames mean defeat and nothing else (the Engineer U1 lesson): 13 for 8 ticks, 14
/// for 12, then 15 held; no live mode ever shows one.
#[test]
fn the_down_frames_are_only_for_the_defeated() {
    for mode in PileMode::ALL {
        for mt in [1, 12, 100] {
            for tick in [0, 17, 90] {
                let frame = frame_for(mode, mt, tick, HoarderLook::default());
                assert_eq!(
                    DOWN.contains(&frame),
                    mode == PileMode::Defeated,
                    "{mode:?} mode tick {mt} tick {tick}: frame {frame}"
                );
            }
        }
    }
    let at = |age| {
        frame_for(
            PileMode::Defeated,
            1,
            0,
            HoarderLook {
                defeated_age: Some(age),
                ..HoarderLook::default()
            },
        )
    };
    assert_eq!(
        [at(0), at(7), at(8), at(19), at(20), at(5000)],
        [13, 13, 14, 14, 15, 15]
    );
}

/// Every frame of her carries the visible stand-in label: whenever her sprite is drawn the
/// label tag is, at her pivot, above her head, with the sheet's text; while she is Dormant
/// (under the mound) neither is. A run of the room pilot covers every mode she plays.
#[test]
fn every_drawn_frame_carries_the_stand_in_label() {
    let mut rig = Rig::new(&root(), clear_plan()).unwrap();
    let mut pilot = ClearPilot::new(0);
    let mut modes = std::collections::BTreeSet::new();
    let mut drawn = 0;
    for _ in 0..1400 {
        let input = pilot.input(&rig.combat, &rig.scene);
        rig.step(carl_input(input));
        let art = art_of(&rig, rig.scene.world.tick, HoarderLook::default());
        let tag = art.tags.iter().find(|t| t.kind == TagKind::Hoarder);
        match (art.hoarder, tag) {
            (Some(h), Some(t)) => {
                drawn += 1;
                modes.insert(format!("{:?}", rig.boss().boss().mode()));
                assert_eq!(t.kind.text(), "STAND-IN UNTIL HOARD-02");
                assert_eq!(t.centre_x, h.x);
                // Above the top of her tallest frame (the rear-up, about 121 px) while she
                // stands, just over her body while she is down.
                let floor = rig.boss().boss().setup().floor_y;
                if rig.boss().boss().mode() == PileMode::Defeated {
                    assert_eq!(t.bottom_y, floor + PRONE_LABEL_ABOVE_FEET_PX);
                } else {
                    assert!(t.bottom_y >= floor + 121.0);
                }
            }
            (None, None) => {
                assert_eq!(rig.boss().boss().mode(), PileMode::Dormant);
            }
            other => panic!("sprite and tag disagree: {other:?}"),
        }
    }
    assert!(drawn > 1000, "she was drawn on {drawn} ticks");
    // The run covers the reveal, a whole swipe, the cooldown and the defeat.
    for want in [
        "Reveal",
        "SwipeTell",
        "SwipeSmear",
        "SwipeImpact",
        "SwipeRecover",
        "Cooldown",
        "Defeated",
    ] {
        assert!(modes.contains(want), "{want} not drawn: {modes:?}");
    }
}

fn clear_plan() -> Plan {
    Plan {
        carl_x: 112,
        donut_x: 80,
        ..Plan::carl_at(60.0)
    }
    .dormant()
    .with_control(Control::SoloCarlAssist)
}

/// The shove's tell is drawn 4 px back (the wind-up), toward the side she faces away from.
#[test]
fn the_shove_tell_is_drawn_four_px_back() {
    let mut rig = Rig::new(
        &root(),
        Plan {
            shove_cooldown: Some(0),
            ..Plan::carl_at(150.0)
        },
    )
    .unwrap();
    let mut seen = false;
    for _ in 0..120 {
        rig.step(IDLE_SLOTS);
        if rig.boss().boss().mode() == PileMode::ShoveTell {
            let art = art_of(&rig, rig.scene.world.tick, HoarderLook::default());
            let draw = art.hoarder.unwrap();
            let boss = rig.boss().boss();
            let at_pivot = actor_center(SPEC, boss.x(), boss.setup().floor_y, boss.facing() < 0);
            // She faces -x, so the wind-up is toward +x.
            assert_eq!(boss.facing(), -1);
            assert_eq!(draw.centre.0 - at_pivot.0, SHOVE_WINDUP_PX);
            assert_eq!(draw.frame, GATHER);
            seen = true;
            break;
        }
    }
    assert!(seen, "she shoved");
}

/// The door stands as a closed slab until her defeat tick and is only its open frame after
/// it, on exactly that tick (the sim's own door state, read through the slot enemy).
#[test]
fn the_door_is_drawn_closed_until_her_defeat_and_open_after() {
    let mut rig = Rig::new(
        &root(),
        Plan {
            hp: Some(1),
            ..Plan::carl_at(44.0)
        },
    )
    .unwrap();
    rig.step(IDLE_SLOTS);
    let before = art_of(&rig, rig.scene.world.tick, HoarderLook::default());
    assert!(labels(&before).contains(&"exit door (stand-in)"));
    assert!(!labels(&before).contains(&"door frame, open (stand-in)"));
    let mut defeat_tick = None;
    for _ in 0..30 {
        rig.step(carl_input(attack_input()));
        let art = art_of(&rig, rig.scene.world.tick, HoarderLook::default());
        let closed = labels(&art).contains(&"exit door (stand-in)");
        if defeat_tick.is_none() && !rig.defeats().is_empty() {
            defeat_tick = Some(rig.scene.world.tick);
        }
        assert_eq!(
            closed,
            defeat_tick.is_none(),
            "tick {}",
            rig.scene.world.tick
        );
    }
    assert!(defeat_tick.is_some(), "she fell");
    let after = art_of(&rig, rig.scene.world.tick, HoarderLook::default());
    assert!(labels(&after).contains(&"door frame, open (stand-in)"));
    // The door's slab is the sidecar's lock solid, in world px.
    let lock = rig.run.loaded.encounter.locks[0].solid;
    let layout_h = rig.scene.world.layout_height();
    let slab = before
        .pieces
        .iter()
        .find(|p| p.label == "exit door (stand-in)")
        .unwrap();
    assert_eq!(
        (slab.x, slab.y, slab.w, slab.h),
        (lock.x, layout_h - lock.bottom(), lock.w, lock.h)
    );
}

/// The entry gate is drawn as bars while the sim's gate stands, and as its faint footprint
/// while open (before the engage, after her defeat); the footprint is the gate solid's.
#[test]
fn the_gate_is_drawn_as_the_sims_gate() {
    let mut rig = Rig::new(&root(), clear_plan()).unwrap();
    let mut pilot = ClearPilot::new(0);
    let (mut closed_seen, mut open_seen) = (false, false);
    for _ in 0..1400 {
        let input = pilot.input(&rig.combat, &rig.scene);
        rig.step(carl_input(input));
        let art = art_of(&rig, rig.scene.world.tick, HoarderLook::default());
        let stands = rig.boss().boss().gate_solid();
        let drawn_closed = labels(&art).contains(&"entry gate, closed (stand-in)");
        let drawn_open = labels(&art).contains(&"entry gate, open (stand-in)");
        assert_eq!(
            drawn_closed,
            stands.is_some(),
            "tick {}",
            rig.scene.world.tick
        );
        assert_eq!(
            drawn_open,
            stands.is_none(),
            "tick {}",
            rig.scene.world.tick
        );
        if let Some(rect) = stands {
            let p = art
                .pieces
                .iter()
                .find(|p| p.label == "entry gate, closed (stand-in)")
                .unwrap();
            assert_eq!(
                (p.x, p.y, p.w, p.h),
                (
                    rect.left as i32,
                    rect.bottom as i32,
                    rect.width as i32,
                    rect.height as i32
                )
            );
        }
        closed_seen |= drawn_closed;
        open_seen |= drawn_open;
    }
    assert!(closed_seen && open_seen);
}

/// Bank A and the post are the sidecar's statics in world px, standing on the walking line,
/// each with its visible label; the mound is behind everything, at her home column.
#[test]
fn bank_a_the_post_and_the_mound_come_from_the_sidecar_and_the_tuning() {
    let rig = Rig::new(&root(), clear_plan()).unwrap();
    let art = art_of(&rig, 0, HoarderLook::default());
    let floor = rig.boss().boss().setup().floor_y as i32;
    let bank = art
        .pieces
        .iter()
        .find(|p| p.label == "bank A (stand-in)")
        .unwrap();
    assert_eq!((bank.x, bank.y, bank.w, bank.h), (352, floor, 64, 48));
    let post = art
        .pieces
        .iter()
        .find(|p| p.label == "J-B post (stand-in)")
        .unwrap();
    assert_eq!((post.x, post.y, post.w, post.h), (808, floor, 16, 64));
    let mound = art.pieces.iter().find(|p| p.label == MOUND_LABEL).unwrap();
    assert_eq!(mound.layer, Layer::Back);
    assert_eq!(mound.x + mound.w / 2, 608, "her home column");
    // Interim readability (the owner's report): the collision-free mound is low, dark and
    // translucent dressing behind the actors, not a solid labelled slab.
    assert!(mound.h <= 10, "low: {}", mound.h);
    assert!(PALETTE[mound.tone as usize][3] < 0xff, "translucent");
    let stack = art
        .pieces
        .iter()
        .find(|p| p.label == "J-B stack (stand-in)")
        .unwrap();
    assert_eq!(
        (stack.x, stack.y, stack.w, stack.h),
        (752, floor + 96, 128, 64),
        "the sheet's stack [752, 880) x [96, 160)"
    );
    for kind in [
        TagKind::BankA,
        TagKind::PostB,
        TagKind::StackB,
        TagKind::Door,
        TagKind::Gate,
    ] {
        assert!(art.tags.iter().any(|t| t.kind == kind), "{kind:?}");
    }
    // Once she is up the mound is still there, unlabelled, in every mode.
    let mut up = Rig::new(&root(), Plan::carl_at(60.0)).unwrap();
    up.step(IDLE_SLOTS);
    let awake = art_of(&up, 1, HoarderLook::default());
    assert!(awake.pieces.iter().any(|p| p.label == MOUND_LABEL));
    assert!(art.pieces.len() <= MAX_PIECES);
    // Dormant: no sprite, no label of hers.
    assert!(art.hoarder.is_none());
    assert!(!art.tags.iter().any(|t| t.kind == TagKind::Hoarder));
}

/// A room with no Hoarder in the slot draws nothing of hers.
#[test]
fn a_room_without_her_draws_nothing() {
    let run = crate::f01_hazards::launch(&root()).unwrap();
    let mut scene = crate::corridor::SceneSim::new_at(&run.kit, 360, run.fixture.carl_x).unwrap();
    let (carl, donut) = Default::default();
    let combat = crate::f01_hazards::combat_for(&run, &mut scene, carl, donut, None).unwrap();
    let hoarder = crate::f01_hoarder::launch(&root()).unwrap();
    assert!(
        draw(
            &combat,
            &hoarder.loaded.encounter,
            660,
            0,
            HoarderLook::default()
        )
        .is_none()
    );
}

/// FA-Q23: the presenter has no authoritative effect. The room pilot's run with every function of
/// this module called on every tick (and every label rasterised) ends on the same state hash
/// as the library fixture's run that never calls it, and the hash after each tick is the
/// same in both: the hashes are pinned (`hoarder_fixtures::tests::ROOM_GOLDENS`) and never move.
#[test]
fn the_presenter_does_not_change_the_state_hash() {
    for seed in [0u64, 7] {
        let report = room_run(&root(), seed).unwrap();
        let mut with_art = Rig::new(&root(), clear_plan()).unwrap();
        let mut plain = Rig::new(&root(), clear_plan()).unwrap();
        let (mut a, mut b) = (ClearPilot::new(seed), ClearPilot::new(seed));
        let mut look = HoarderLook::default();
        for tick in 0..report.ticks {
            let (ia, ib) = (
                a.input(&with_art.combat, &with_art.scene),
                b.input(&plain.combat, &plain.scene),
            );
            with_art.step(carl_input(ia));
            plain.step(carl_input(ib));
            let art = art_of(&with_art, tick, look);
            for tag in &art.tags {
                let _ = text_pixels(tag.kind.text()).expect("every label rasterises");
            }
            look.flash = tick % 7 == 0;
            look.defeated_age = (!with_art.boss().boss().alive()).then_some(tick % 40);
            assert_eq!(with_art.hash(), plain.hash(), "seed {seed} tick {tick}");
        }
        assert_eq!(
            with_art.hash(),
            report.hash,
            "seed {seed}: the pinned library hash"
        );
        // Her one defeat and one grant happened under the drawn run too.
        assert_eq!(
            with_art
                .combat
                .events
                .iter()
                .filter(|e| matches!(e, CombatEvent::CapabilityGranted { .. }))
                .count(),
            1
        );
        assert!(with_art.combat.events.iter().any(|e| matches!(
            e,
            CombatEvent::PileBoss {
                event: PileEvent::Engaged,
                ..
            }
        )));
    }
}

/// The label font: every label of the room rasterises, with its width from the glyph advance, a
/// shadow row and column, ink and shadow pixels and nothing outside the image; an unknown
/// character has no glyph.
#[test]
fn the_labels_rasterise_in_the_pixel_font() {
    for kind in TagKind::ALL {
        let image = text_pixels(kind.text()).unwrap_or_else(|| panic!("{}", kind.text()));
        let chars = kind.text().chars().count() as u32;
        assert_eq!(image.w, chars * GLYPH_ADVANCE_PX, "{kind:?}");
        assert_eq!(image.h, GLYPH_HEIGHT_PX + 1);
        assert_eq!(image.rgba.len(), (image.w * image.h * 4) as usize);
        let ink = image.rgba.chunks(4).filter(|p| *p == TAG_INK).count();
        let shadow = image.rgba.chunks(4).filter(|p| *p == TAG_SHADOW).count();
        assert!(
            ink > 0 && shadow > 0,
            "{kind:?}: {ink} ink, {shadow} shadow"
        );
    }
    // "I" is a full top and bottom bar over a one-pixel stem: 3 + 1 + 1 + 1 + 3 = 9 ink pixels.
    let i = text_pixels("I").unwrap();
    assert_eq!(i.rgba.chunks(4).filter(|p| *p == TAG_INK).count(), 9);
    // The shadow sits one pixel down and right of the ink and never covers it.
    assert_eq!(&i.rgba[0..4], &TAG_INK, "top left is ink");
    assert!(text_pixels("a").is_none(), "lower case has no glyph");
    assert!(text_pixels("").is_none());
    assert_eq!(
        text_pixels(STAND_IN_LABEL).unwrap().w,
        23 * GLYPH_ADVANCE_PX,
        "the sheet's label is 23 characters"
    );
}

/// The tones are opaque except the open gate's faint footprint, the low mound and the stack's
/// floor shadow.
#[test]
fn the_palette_is_opaque_except_the_open_gates_footprint_the_mound_and_the_shadow() {
    let faint = [Tone::GateOpen, Tone::Mound, Tone::Shadow].map(|t| t as usize);
    for (i, c) in PALETTE.iter().enumerate() {
        assert_eq!(c[3] == 0xff, !faint.contains(&i), "tone {i}");
    }
}

/// F3: the hit flash follows her hit points, lasts `FLASH_TICKS`, and a tick that goes backwards (a
/// restart or a retry resets the scene tick to 0) clears the memo, so the next attempt is not flashed
/// by the last hit of the old one.
#[test]
fn the_flash_memo_runs_four_ticks_and_forgets_on_a_tick_regression() {
    let mut memo = HoarderMemo::default();
    assert!(!memo.observe(30, true, 1000).flash);
    // A hit on tick 1010: the flash runs for FLASH_TICKS ticks, from the tick of the drop.
    for (tick, flash) in [(1009, false), (1010, true), (1013, true), (1014, false)] {
        let hp = if tick >= 1010 { 29 } else { 30 };
        assert_eq!(memo.observe(hp, true, tick).flash, flash, "tick {tick}");
    }
    // Hit again, then a restart: the tick goes back to 5 and her hit points reset. No flash.
    assert!(memo.observe(28, true, 1020).flash);
    let after = memo.observe(30, true, 5);
    assert!(!after.flash, "no stale flash after a restart");
    assert!(!memo.observe(30, true, 6).flash);
    assert_eq!(memo, {
        let mut fresh = HoarderMemo::default();
        fresh.observe(30, true, 5);
        fresh.observe(30, true, 6);
        fresh
    });
    // The defeat age counts from the first defeated frame and is forgotten by a regression too.
    assert_eq!(memo.observe(0, false, 100).defeated_age, Some(0));
    assert_eq!(memo.observe(0, false, 130).defeated_age, Some(30));
    assert_eq!(memo.observe(30, true, 3).defeated_age, None);
}

/// Interim readability (the owner's report that she "looks transparent from the mechanics too"):
/// while she has no hurtbox (the 90-tick Reveal) she is drawn as a dim silhouette, and solid on the
/// first hittable tick. The flag is read from the simulation's own hurtbox, never invented.
#[test]
fn she_is_a_dim_silhouette_until_the_first_hittable_tick() {
    let plan = Plan {
        carl_x: 428,
        ..Plan::carl_at(60.0)
    }
    .dormant();
    let mut rig = Rig::new(&root(), plan).unwrap();
    let (mut silhouette_ticks, mut first_hittable) = (0, None);
    for _ in 0..200 {
        let tick = rig.next_tick();
        rig.step(carl_input(crate::hoarder_fixtures::run_input(1.0)));
        let tick_after = rig.next_tick() - 1;
        let _ = tick;
        let art = art_of(&rig, tick_after, HoarderLook::default());
        let Some(her) = art.hoarder else {
            continue;
        };
        let hittable = !rig.boss().boss().hurt_rects().is_empty();
        if rig.boss().boss().mode() == PileMode::Reveal {
            assert!(!hittable, "no hurtbox in the Reveal");
            assert!(her.silhouette, "dim while she cannot be hit");
            assert!(
                her.alpha <= REVEAL_DIM_ALPHA + f32::EPSILON,
                "{}",
                her.alpha
            );
            silhouette_ticks += 1;
        } else if hittable {
            assert!(!her.silhouette);
            assert!(
                (her.alpha - 1.0).abs() < f32::EPSILON,
                "solid once hittable"
            );
            first_hittable.get_or_insert(tick_after);
        }
    }
    assert_eq!(silhouette_ticks, 90, "the whole 90-tick Reveal");
    assert!(first_hittable.is_some(), "she becomes hittable");
}

/// Bank A is drawn as the sim's solid by state: the slab while it stands, the moving mass while it
/// slides, the low remnant after; the J-B stack hangs at its sheet rectangle, drops, and leaves the
/// remnant (HRD-B, HRD-C; stand-in drawing until HOARD-02).
#[test]
fn the_piles_are_drawn_from_their_machines() {
    let floor = |rig: &Rig| rig.boss().boss().setup().floor_y as i32;
    let pieces_with = |art: &HoarderArt, label: &str| {
        art.pieces
            .iter()
            .filter(|p| p.label == label)
            .copied()
            .collect::<Vec<_>>()
    };
    // Bank A: baited slide.
    let mut rig = Rig::new(
        &root(),
        Plan {
            carl_x: 330,
            awake: Some((470.0, -1)),
            ..Plan::carl_at(140.0)
        },
    )
    .unwrap();
    let art = art_of(&rig, 1, HoarderLook::default());
    assert_eq!(
        pieces_with(&art, "bank A (stand-in)").len(),
        2,
        "body and top bar"
    );
    let (mut slid, mut settled) = (false, false);
    for _ in 0..220 {
        rig.step(IDLE_SLOTS);
        let t = rig.next_tick() - 1;
        let art = art_of(&rig, t, HoarderLook::default());
        match rig.combat.env().unwrap().slide(0).unwrap().state() {
            crate::env_prop::SlideState::Slid => {
                let m = pieces_with(&art, "bank A, sliding (stand-in)");
                assert_eq!(m.len(), 1, "one moving mass");
                assert!(m[0].x <= 352, "moving toward -x: {}", m[0].x);
                assert!(pieces_with(&art, "bank A (stand-in)").is_empty());
                slid = true;
            }
            crate::env_prop::SlideState::Settled => {
                let r = pieces_with(&art, "bank A remnant (stand-in)");
                assert_eq!(r.len(), 2, "body and top bar");
                assert!(r.iter().all(|p| p.h <= 16));
                assert!(art.tags.iter().all(|tg| tg.kind != TagKind::BankA));
                settled = true;
            }
            _ => {}
        }
    }
    assert!(slid && settled);
    // The J-B stack: baited into the post.
    let mut rig = Rig::new(
        &root(),
        Plan {
            carl_x: 852,
            awake: Some((700.0, 1)),
            ..Plan::carl_at(60.0)
        },
    )
    .unwrap();
    let f = floor(&rig);
    let (mut shadow, mut remnant, mut gone_post) = (false, false, false);
    for _ in 0..260 {
        rig.step(IDLE_SLOTS);
        let t = rig.next_tick() - 1;
        let art = art_of(&rig, t, HoarderLook::default());
        match rig.combat.env().unwrap().mass_state(1).unwrap() {
            crate::env_prop::MassState::Released | crate::env_prop::MassState::Falling => {
                shadow |= !pieces_with(&art, "J-B footprint shadow (stand-in)").is_empty();
                gone_post |= pieces_with(&art, "J-B post (stand-in)").is_empty();
                let s = pieces_with(&art, "J-B stack (stand-in)");
                assert_eq!(s.len(), 2, "body and top bar");
                assert!(s[0].y <= f + 96 + 2, "never above the hanging stack");
            }
            crate::env_prop::MassState::Landed => {
                let r = pieces_with(&art, "J-B stack remnant (stand-in)");
                assert_eq!((r[0].x, r[0].w, r[0].h), (752, 128, 16));
                assert!(pieces_with(&art, "J-B stack (stand-in)").is_empty());
                remnant = true;
            }
            _ => {}
        }
    }
    assert!(shadow && remnant && gone_post);
}
