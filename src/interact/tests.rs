use super::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn manifest(file: &str) -> serde_json::Value {
    let text = std::fs::read_to_string(root().join("assets/manifests").join(file)).unwrap();
    serde_json::from_str(&text).unwrap()
}

const SITE: InteractSite = InteractSite {
    box_faces: Some((340, 364)),
    guide_x: Some(380),
};

fn ctx(x: f32, facing: i8) -> HeroCtx {
    HeroCtx {
        x,
        facing,
        grounded: true,
        can_start: true,
        interrupted: false,
        frozen: false,
        bot: false,
        present: true,
    }
}

fn press() -> RawInput {
    RawInput {
        interact: true,
        ..RawInput::NONE
    }
}

fn run(it: &mut Interact, tick: &mut u64, ctxs: [HeroCtx; 2], raws: [RawInput; 2]) -> Outcome {
    *tick += 1;
    it.step(*tick, ctxs, raws)
}

/// The verb tables are the export manifests' tick contract (spec section 3).
#[test]
fn the_verb_tables_are_the_manifest_tick_contract() {
    for (file, verbs) in [
        (
            "carl_book1_h52_interact.json",
            [Verb::Hand, Verb::Floor, Verb::Push],
        ),
        (
            "donut_h40_interact.json",
            [Verb::Paw, Verb::Nudge, Verb::Nudge],
        ),
    ] {
        let m = manifest(file);
        for verb in verbs {
            let mv = &m["interact"]["moves"][verb.tag()];
            assert_eq!(mv["total_ticks"], verb.total_ticks(), "{}", verb.tag());
            assert_eq!(mv["contact_tick"], verb.contact_tick(), "{}", verb.tag());
            let frames = mv["frames"].as_array().unwrap();
            assert_eq!(frames.len(), 4);
            for (i, frame) in frames.iter().enumerate() {
                let first = frame["ticks"][0].as_u64().unwrap() as u32;
                let last = frame["ticks"][1].as_u64().unwrap() as u32;
                assert_eq!(verb.frame_at(first), Some(i), "{} first of {i}", verb.tag());
                assert_eq!(verb.frame_at(last), Some(i), "{} last of {i}", verb.tag());
            }
            assert_eq!(verb.frame_at(verb.total_ticks()), None);
            let key = frames
                .iter()
                .position(|f| f["phase"] == "contact")
                .expect("a contact frame");
            assert_eq!(
                verb.frame_at(verb.contact_tick()),
                Some(key),
                "{}",
                verb.tag()
            );
        }
    }
}

/// The verb is chosen by the height class and the hero; a cat has none above her low class.
#[test]
fn the_verb_follows_the_height_class_and_the_hero() {
    use HeightClass::*;
    assert_eq!(verb_for(HeroId::Carl, Hand), Some(Verb::Hand));
    assert_eq!(verb_for(HeroId::Carl, Waist), Some(Verb::Push));
    assert_eq!(verb_for(HeroId::Carl, Floor), Some(Verb::Floor));
    assert_eq!(verb_for(HeroId::Carl, Low), Some(Verb::Floor));
    assert_eq!(verb_for(HeroId::Donut, Floor), Some(Verb::Nudge));
    assert_eq!(verb_for(HeroId::Donut, Low), Some(Verb::Paw));
    assert_eq!(verb_for(HeroId::Donut, Waist), None);
    assert_eq!(verb_for(HeroId::Donut, Hand), None);
    for verb in Verb::ALL {
        let m = manifest(match verb {
            Verb::Hand | Verb::Floor | Verb::Push => "carl_book1_h52_interact.json",
            Verb::Paw | Verb::Nudge => "donut_h40_interact.json",
        });
        assert!(
            m["animation_tags"]
                .as_array()
                .unwrap()
                .iter()
                .any(|t| *t == verb.tag())
        );
    }
}

/// Carl at the box's left stand-off presses: the hero is snapped, locked for the whole verb, the
/// pose follows the clock and the box opens once, on the contact tick.
#[test]
fn the_box_opens_once_on_the_contact_tick() {
    let mut it = Interact::new(SITE);
    let mut tick = 100;
    let idle = [RawInput::NONE; 2];
    let carl = |x| [ctx(x, 1), ctx(80.0, 1)];
    // Not pressed: nothing.
    run(&mut it, &mut tick, carl(321.0), idle);
    assert!(it.active(HeroId::Carl).is_none() && !it.box_open());
    // The press: 19 px from the left face, inside the window; snapped to 340 - 17.
    let out = run(&mut it, &mut tick, carl(321.0), [press(), RawInput::NONE]);
    assert_eq!(out.snap_x[0], Some(323.0));
    assert!(out.locked[0] && !out.locked[1]);
    assert_eq!(it.pose(HeroId::Carl), Some((Verb::Hand, 0)));
    assert!(!it.box_open());
    // Clock k after the k-th following tick; the box opens when the clock reaches 5.
    let contact = Verb::Hand.contact_tick();
    for k in 1..Verb::Hand.total_ticks() {
        // Held button: no second verb, no second effect.
        let out = run(&mut it, &mut tick, carl(323.0), [press(), RawInput::NONE]);
        assert!(out.locked[0], "locked through clock {k}");
        assert_eq!(it.active(HeroId::Carl).unwrap().clock, k);
        assert_eq!(it.box_open(), k >= contact, "clock {k}");
    }
    // The verb ends: the lock is released on the tick the clock reaches the total.
    let out = run(&mut it, &mut tick, carl(323.0), [press(), RawInput::NONE]);
    assert!(!out.locked[0] && it.active(HeroId::Carl).is_none());
    assert!(it.box_open());
    // A new press on the opened box is not a box verb (the guide is 57 px away: it talks).
    let before = it.clone();
    let out = run(&mut it, &mut tick, carl(323.0), idle);
    assert_eq!(out, Outcome::default());
    run(&mut it, &mut tick, carl(323.0), [press(), RawInput::NONE]);
    assert!(it.active(HeroId::Carl).is_none());
    assert!(it.guide_talking(tick));
    assert_eq!(before.presses + 1, it.presses);
}

/// From the right face the hero faces left and is snapped to face + stand-off.
#[test]
fn the_box_is_opened_from_the_right_too() {
    let mut it = Interact::new(SITE);
    let out = it.step(1, [ctx(390.0, -1), ctx(80.0, 1)], [press(), RawInput::NONE]);
    assert_eq!(out.snap_x[0], Some(381.0));
    assert_eq!(out.face[0], Some(-1));
    assert!(out.locked[0]);
    // Facing away from the box, or too far, is no box press.
    let mut it = Interact::new(SITE);
    let away = it.step(1, [ctx(390.0, 1), ctx(80.0, 1)], [press(), RawInput::NONE]);
    assert!(!away.locked[0] && it.active(HeroId::Carl).is_none());
    let mut it = Interact::new(SITE);
    let far = it.step(1, [ctx(250.0, 1), ctx(80.0, 1)], [press(), RawInput::NONE]);
    assert_eq!(far, Outcome::default());
    assert_eq!(it.presses, 0);
}

/// A direction pressed with Interact turns the hero toward the box first.
#[test]
fn a_direction_with_the_press_turns_the_hero() {
    let mut it = Interact::new(SITE);
    let raw = RawInput {
        move_x: 1.0,
        interact: true,
        ..RawInput::NONE
    };
    let out = it.step(1, [ctx(321.0, -1), ctx(80.0, 1)], [raw, RawInput::NONE]);
    assert_eq!(out.face[0], Some(1));
    assert!(out.locked[0]);
}

/// The guide talks on a press within reach, for the talk window; nobody is locked.
#[test]
fn the_guide_talks_on_interact() {
    let mut it = Interact::new(InteractSite {
        box_faces: None,
        guide_x: Some(380),
    });
    let out = it.step(10, [ctx(330.0, 1), ctx(80.0, 1)], [press(), RawInput::NONE]);
    assert_eq!(out.locked, [false, false]);
    assert_eq!(out.face[0], Some(1));
    assert!(it.guide_talking(10) && it.guide_talking(10 + TALK_TICKS - 1));
    assert!(!it.guide_talking(10 + TALK_TICKS));
    // Donut (the player) talks to him as well; a bot does not.
    let mut it = Interact::new(SITE);
    it.step(1, [ctx(80.0, 1), ctx(400.0, -1)], [RawInput::NONE, press()]);
    assert!(it.guide_talking(2));
    let mut bot = ctx(400.0, -1);
    bot.bot = true;
    let mut it = Interact::new(SITE);
    it.step(1, [ctx(80.0, 1), bot], [RawInput::NONE, press()]);
    assert!(!it.guide_talking(2) && it.presses == 0);
}

/// Donut has no verb for the hand-height box: her press on it does not open it.
#[test]
fn donut_cannot_open_the_box() {
    let mut it = Interact::new(InteractSite {
        box_faces: Some((340, 364)),
        guide_x: None,
    });
    let out = it.step(1, [ctx(80.0, 1), ctx(321.0, 1)], [RawInput::NONE, press()]);
    assert_eq!(out, Outcome::default());
    assert!(!it.box_open() && it.active(HeroId::Donut).is_none());
}

/// A press with no target, a bot, a hero that cannot start, or one in the air does nothing.
#[test]
fn no_effect_without_a_target() {
    let nowhere = InteractSite {
        box_faces: None,
        guide_x: None,
    };
    let mut it = Interact::new(nowhere);
    assert_eq!(
        it.step(1, [ctx(350.0, 1), ctx(350.0, 1)], [press(), press()]),
        Outcome::default()
    );
    assert_eq!(it.presses, 0);
    let mut bits = Vec::new();
    it.mix_into(&mut |v| bits.push(v));
    assert!(
        bits.is_empty(),
        "nothing is mixed without an accepted press"
    );
    for tweak in [
        |c: &mut HeroCtx| c.bot = true,
        |c: &mut HeroCtx| c.can_start = false,
        |c: &mut HeroCtx| c.grounded = false,
        |c: &mut HeroCtx| c.present = false,
    ] {
        let mut it = Interact::new(SITE);
        let mut c = ctx(321.0, 1);
        tweak(&mut c);
        let out = it.step(1, [c, ctx(80.0, 1)], [press(), RawInput::NONE]);
        assert_eq!(out, Outcome::default());
        assert_eq!(it.presses, 0);
    }
    // After an accepted press the state is mixed.
    let mut it = Interact::new(SITE);
    it.step(1, [ctx(321.0, 1), ctx(80.0, 1)], [press(), RawInput::NONE]);
    let mut bits = Vec::new();
    it.mix_into(&mut |v| bits.push(v));
    assert!(!bits.is_empty());
}

/// A hit before the contact tick cancels the verb and the box never opens; a hit-stop holds the
/// clock; a hit after the contact tick leaves the effect.
#[test]
fn a_hit_cancels_before_contact_and_hit_stop_holds_the_clock() {
    let mut it = Interact::new(SITE);
    let mut tick = 0;
    run(
        &mut it,
        &mut tick,
        [ctx(321.0, 1), ctx(80.0, 1)],
        [press(), RawInput::NONE],
    );
    let mut frozen = ctx(323.0, 1);
    frozen.frozen = true;
    for _ in 0..10 {
        let out = run(
            &mut it,
            &mut tick,
            [frozen, ctx(80.0, 1)],
            [RawInput::NONE; 2],
        );
        assert!(out.locked[0]);
    }
    assert_eq!(
        it.active(HeroId::Carl).unwrap().clock,
        0,
        "hit-stop holds the clock"
    );
    run(
        &mut it,
        &mut tick,
        [ctx(323.0, 1), ctx(80.0, 1)],
        [RawInput::NONE; 2],
    );
    run(
        &mut it,
        &mut tick,
        [ctx(323.0, 1), ctx(80.0, 1)],
        [RawInput::NONE; 2],
    );
    let mut hurt = ctx(323.0, 1);
    hurt.interrupted = true;
    let out = run(
        &mut it,
        &mut tick,
        [hurt, ctx(80.0, 1)],
        [RawInput::NONE; 2],
    );
    assert!(!out.locked[0] && it.active(HeroId::Carl).is_none());
    assert!(
        !it.box_open(),
        "cancelled before the contact tick: no effect"
    );

    // After the contact tick the effect stays.
    let mut it = Interact::new(SITE);
    let mut tick = 0;
    run(
        &mut it,
        &mut tick,
        [ctx(321.0, 1), ctx(80.0, 1)],
        [press(), RawInput::NONE],
    );
    for _ in 0..8 {
        run(
            &mut it,
            &mut tick,
            [ctx(323.0, 1), ctx(80.0, 1)],
            [RawInput::NONE; 2],
        );
    }
    run(
        &mut it,
        &mut tick,
        [hurt, ctx(80.0, 1)],
        [RawInput::NONE; 2],
    );
    assert!(it.box_open() && it.active(HeroId::Carl).is_none());
}

/// Two heroes cannot both open the box, and a respawn cancels a verb but keeps the box.
#[test]
fn one_verb_per_box_and_a_respawn_cancels() {
    let mut it = Interact::new(SITE);
    it.step(1, [ctx(321.0, 1), ctx(321.0, 1)], [press(), press()]);
    assert!(it.active(HeroId::Carl).is_some());
    assert!(it.active(HeroId::Donut).is_none());
    it.cancel();
    assert!(it.active(HeroId::Carl).is_none() && !it.box_open());
    it.reset();
    assert_eq!(it, Interact::new(SITE));
}
