use super::*;
use crate::combat::CombatEvent;
use crate::hazards_fixtures::{Layout, Rig, SlimeAt, walk};
use crate::input::RawInput;
use crate::slime::acid_source;

/// The pool of the tests: 700..764 on the kit's walking line (580), id 1, centre 732.
const POOL: (i32, i32) = (700, 764);

fn pool_rig(carl_x: i32) -> Rig {
    Layout {
        acid: vec![(POOL.0, POOL.1, None)],
        carl_x,
        ..Layout::default()
    }
    .sim(None)
}

fn hero_x(rig: &Rig) -> impl Fn(HeroId) -> f32 + '_ {
    move |hero| rig.x_of(hero)
}

fn layout_h(rig: &Rig) -> i32 {
    rig.scene().world.layout_height()
}

/// Step the rig one tick and the cues after it, like the presenter does.
fn tick(rig: &mut Rig, cues: &mut AcidSplashes, input: RawInput) {
    rig.carl(input);
    let t = rig.tick();
    cues.step(t, rig.combat(), hero_x(rig));
}

fn view(rig: &Rig, cues: &AcidSplashes) -> Vec<SplashDraw> {
    cues.view(rig.tick(), rig.combat().env().unwrap(), layout_h(rig))
}

/// Walk Carl into the pool until the acid hurts him; returns the tick of the hurt.
fn walk_into(rig: &mut Rig, cues: &mut AcidSplashes, dir: f32) -> u64 {
    for _ in 0..200 {
        tick(rig, cues, walk(dir));
        if let Some(h) = rig.hurts(HeroId::Carl).first() {
            return h.0;
        }
    }
    panic!("the pool never hurt Carl");
}

/// Push a synthetic acid event into the log (the cue reads only the log).
fn inject(rig: &mut Rig, event: CombatEvent) {
    rig.combat_mut().events.push(event);
}

fn acid_hurt(tick: u64, patch: u16, hero: HeroId) -> CombatEvent {
    CombatEvent::HeroHurt {
        tick,
        source: acid_source(patch),
        health_left: 3,
        hero,
    }
}

#[test]
fn the_splash_is_16_ticks_over_5_frames() {
    assert_eq!(TOTAL_TICKS, 16);
    assert_eq!(SPLASH_TICKS, [2, 3, 3, 4, 4]);
    let frames: Vec<Option<usize>> = (0..18).map(splash_frame).collect();
    let want: Vec<Option<usize>> = [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4]
        .into_iter()
        .map(Some)
        .chain([None, None])
        .collect();
    assert_eq!(frames, want);
    assert_eq!(KEY_FRAME, 2, "the crown is the key");
}

/// The hurt of Carl entering the pool starts one splash on that tick; it plays its five
/// frames once and is gone after 16 ticks.
#[test]
fn a_hero_entering_the_pool_splashes_once() {
    let mut rig = pool_rig(POOL.0 - 50);
    let mut cues = AcidSplashes::default();
    let at = walk_into(&mut rig, &mut cues, 1.0);
    assert_eq!(rig.hurts(HeroId::Carl)[0].1, acid_source(1));
    // On the hurt tick: the first frame, one splash.
    let draws = view(&rig, &cues);
    assert_eq!(rig.tick(), at);
    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].frame, 0);
    let mut frames = vec![draws[0].frame];
    for _ in 1..16 {
        tick(&mut rig, &mut cues, RawInput::NONE);
        let d = view(&rig, &cues);
        assert_eq!(d.len(), 1, "tick {}", rig.tick());
        frames.push(d[0].frame);
    }
    assert_eq!(frames, [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4]);
    tick(&mut rig, &mut cues, RawInput::NONE);
    assert!(view(&rig, &cues).is_empty(), "played once, then gone");
    // No second hurt, no second splash.
    assert_eq!(rig.hurts(HeroId::Carl).len(), 1);
}

/// Mirrored when the actor's x is below the patch centre (the sign of the pop), not on
/// the centre tie.
#[test]
fn the_splash_is_mirrored_below_the_centre() {
    // From the left: the hero enters below the centre.
    let mut rig = pool_rig(POOL.0 - 50);
    let mut cues = AcidSplashes::default();
    walk_into(&mut rig, &mut cues, 1.0);
    assert!(view(&rig, &cues)[0].flip_x, "entered from the left");
    // From the right: x at or above the centre.
    let mut rig = pool_rig(POOL.1 + 50);
    let mut cues = AcidSplashes::default();
    walk_into(&mut rig, &mut cues, -1.0);
    assert!(!view(&rig, &cues)[0].flip_x, "entered from the right");
    // The centre tie pops toward +x: not mirrored.
    let mut rig = pool_rig(732);
    let mut cues = AcidSplashes::default();
    tick(&mut rig, &mut cues, RawInput::NONE);
    assert_eq!(rig.hurts(HeroId::Carl)[0].0, 1);
    assert!(!view(&rig, &cues)[0].flip_x);
}

/// The x is the actor's, clamped to `[left + 6, right - 6]` of the live rectangle.
#[test]
fn the_splash_x_is_clamped_inside_the_pool() {
    let mut rig = pool_rig(POOL.0 - 50);
    let mut cues = AcidSplashes::default();
    // A hero far to the left of the pool hurt by it (a synthetic event on a placed hero).
    rig.place(HeroId::Carl, 600.0);
    let t = rig.tick();
    inject(&mut rig, acid_hurt(t, 1, HeroId::Carl));
    cues.step(t, rig.combat(), hero_x(&rig));
    assert_eq!(view(&rig, &cues)[0].x, POOL.0 + CLAMP_MARGIN_PX);
    // ... and far to the right.
    let mut cues = AcidSplashes::default();
    rig.place(HeroId::Carl, 900.0);
    inject(&mut rig, acid_hurt(t, 1, HeroId::Carl));
    cues.step(t + 1, rig.combat(), hero_x(&rig));
    let d = &view(&rig, &cues);
    assert!(d.iter().all(|d| d.x == POOL.1 - CLAMP_MARGIN_PX), "{d:?}");
    // Inside the margins it is the actor's own column.
    let mut cues = AcidSplashes::default();
    rig.place(HeroId::Carl, 740.0);
    let t = rig.tick();
    inject(&mut rig, acid_hurt(t, 1, HeroId::Carl));
    cues.step(t + 2, rig.combat(), hero_x(&rig));
    assert!(view(&rig, &cues).iter().all(|d| d.x == 740));
}

/// Rows 16..23 are cropped to the live rectangle; the crown (rows 0..15) is not.
#[test]
fn the_lip_is_cropped_to_the_live_pool() {
    let mut rig = pool_rig(POOL.0 - 50);
    let mut cues = AcidSplashes::default();
    rig.place(HeroId::Carl, 600.0);
    let t = rig.tick();
    inject(&mut rig, acid_hurt(t, 1, HeroId::Carl));
    cues.step(t, rig.combat(), hero_x(&rig));
    let h = layout_h(&rig);
    let draw = view(&rig, &cues)[0];
    let sprites = draw.sprites();
    assert_eq!(sprites.len(), 2);
    let (crown, lip) = (sprites[0], sprites[1]);
    // The crown is the whole width on rows 0..15 (the top 16 rows of the 24).
    assert_eq!((crown.src_x, crown.src_y, crown.w, crown.h), (0, 0, 32, 16));
    assert_eq!(crown.y, (h - 580) + 8);
    assert_eq!(crown.layer, Layer::Splash);
    // The lip is rows 16..23 only inside the pool: from the pool's left edge to the right
    // edge of the canvas (the hero was clamped to left + 6, mirrored: the canvas spans
    // x - 15 .. x + 17).
    assert_eq!((lip.src_y, lip.h, lip.y), (16, 8, h - 580));
    assert!(lip.x >= POOL.0 && lip.x + lip.w <= POOL.1);
    assert_eq!(lip.x, POOL.0, "cropped at the pool's left end");
    assert!(lip.w < 32);
    // A mirrored splash takes its crop from the other end of the unmirrored cell.
    assert!(draw.flip_x);
    assert_eq!(lip.src_x, SPLASH_W - (lip.x - draw.left() + lip.w));
}

/// A drained patch gives no splash, even one that was triggered while it was alive.
#[test]
fn a_drained_patch_gives_no_splash() {
    let mut rig = Layout {
        acid: vec![(POOL.0, POOL.1, Some(0))],
        slimes: vec![SlimeAt {
            x: 400,
            facing: 1,
            home: 732,
        }],
        carl_x: 112,
        ..Layout::default()
    }
    .sim(None);
    let mut cues = AcidSplashes::default();
    rig.idle(1);
    rig.kill_slime(0);
    for _ in 0..90 {
        tick(&mut rig, &mut cues, RawInput::NONE);
    }
    assert!(
        rig.combat()
            .env()
            .unwrap()
            .acid()
            .get(1)
            .unwrap()
            .rect()
            .is_none()
    );
    let t = rig.tick();
    inject(&mut rig, acid_hurt(t, 1, HeroId::Carl));
    cues.step(t, rig.combat(), hero_x(&rig));
    assert!(view(&rig, &cues).is_empty(), "no pool, no splash");
}

/// A draining pool still splashes inside its live (shrunk) rectangle.
#[test]
fn a_draining_pool_clamps_to_its_live_rectangle() {
    let mut rig = Layout {
        acid: vec![(POOL.0, POOL.1, Some(0))],
        slimes: vec![SlimeAt {
            x: 400,
            facing: 1,
            home: 732,
        }],
        carl_x: 112,
        ..Layout::default()
    }
    .sim(None);
    let mut cues = AcidSplashes::default();
    rig.idle(1);
    rig.kill_slime(0);
    for _ in 0..30 {
        tick(&mut rig, &mut cues, RawInput::NONE);
    }
    let live = rig
        .combat()
        .env()
        .unwrap()
        .acid()
        .get(1)
        .unwrap()
        .bounds()
        .unwrap();
    assert!(live.0 > POOL.0 && live.1 < POOL.1, "half drained: {live:?}");
    rig.place(HeroId::Carl, 600.0);
    let t = rig.tick();
    inject(&mut rig, acid_hurt(t, 1, HeroId::Carl));
    cues.step(t, rig.combat(), hero_x(&rig));
    let d = view(&rig, &cues);
    assert_eq!(d[0].x, live.0 + CLAMP_MARGIN_PX);
    assert_eq!(d[0].live, live);
}

/// At most four play at once; a full set replaces the oldest; a repeat for the same
/// actor restarts it; two events of the same tick, patch and actor make one.
#[test]
fn the_slots_dedupe_restart_and_replace_the_oldest() {
    let mut c = AcidSplashes::default();
    // Same tick, patch and actor: one.
    c.trigger(10, 1, Actor::Enemy(0), 710.0);
    c.trigger(10, 1, Actor::Enemy(0), 712.0);
    assert_eq!(c.slots.len(), 1);
    // The same actor on a later tick restarts it.
    c.trigger(14, 1, Actor::Enemy(0), 720.0);
    assert_eq!(c.slots.len(), 1);
    assert_eq!(c.slots[0].start, 14);
    assert_eq!(c.slots[0].x, 720.0);
    // The same tick but another patch is a restart too (the actor changed pools).
    c.trigger(14, 2, Actor::Enemy(0), 20.0);
    assert_eq!((c.slots[0].start, c.slots[0].patch), (14, 2));
    // Four slots fill, the fifth replaces the oldest.
    let mut c = AcidSplashes::default();
    for i in 0..SLOTS as u32 {
        c.trigger(10 + u64::from(i), 1, Actor::Enemy(i), 700.0);
    }
    assert_eq!(c.slots.len(), SLOTS);
    c.trigger(20, 1, Actor::Enemy(9), 700.0);
    assert_eq!(c.slots.len(), SLOTS, "never more than four");
    assert!(
        c.slots.iter().all(|s| s.actor != Actor::Enemy(0)),
        "the oldest went"
    );
    assert!(c.slots.iter().any(|s| s.actor == Actor::Enemy(9)));
    // Finished splashes free their slots.
    let mut c = AcidSplashes::default();
    for i in 0..SLOTS as u32 {
        c.trigger(10, 1, Actor::Enemy(i), 700.0);
    }
    c.trigger(10 + TOTAL_TICKS, 1, Actor::Enemy(7), 700.0);
    assert_eq!(c.slots.len(), 1, "the four that were over are gone");
}

/// Never more than four splashes (eight sprites) on one tick.
#[test]
fn a_view_never_has_more_than_four() {
    let mut rig = pool_rig(112);
    let mut cues = AcidSplashes::default();
    let t = rig.tick();
    for i in 0..9 {
        cues.trigger(t, 1, Actor::Enemy(i), 710.0 + i as f32 * 4.0);
    }
    let d = view(&rig, &cues);
    assert_eq!(d.len(), SLOTS);
    let sprites: usize = d.iter().map(|d| d.sprites().len()).sum();
    assert!(sprites <= SLOTS * SPRITES_PER_SPLASH);
    rig.idle(1);
}

/// A backward seek (or a shorter log) resets the cue state.
#[test]
fn the_cues_reset_on_a_backward_seek() {
    let mut rig = pool_rig(POOL.0 - 50);
    let mut cues = AcidSplashes::default();
    walk_into(&mut rig, &mut cues, 1.0);
    assert_eq!(cues.slots.len(), 1);
    assert_eq!(cues.seen_events, rig.combat().events.len());
    // A tick that went backwards forgets everything and re-reads the log it is given.
    cues.slots.clear();
    cues.step(1, rig.combat(), hero_x(&rig));
    assert_eq!(cues.last_tick, 1);
    assert_eq!(cues.slots.len(), 1, "the log was read again from its start");
    // A shorter log (a restart of the attempt) resets as well.
    rig.combat_mut().events.clear();
    cues.step(2, rig.combat(), hero_x(&rig));
    assert!(cues.slots.is_empty());
    assert_eq!(cues.seen_events, 0);
}

/// The trigger table: an acid `HeroHurt` (Donut too), an acid `EnvironmentHurt` and an
/// acid `EnvironmentKill` splash; every other hurt and class does not.
#[test]
fn only_acid_events_trigger() {
    let mut rig = Layout {
        acid: vec![(POOL.0, POOL.1, None)],
        goblins: vec![720, 740],
        carl_x: 112,
        ..Layout::default()
    }
    .sim(Some(false));
    let mut cues = AcidSplashes::default();
    let t = rig.tick();
    // Not acid: a goblin blow, a slime lunge, a Rot blast.
    for source in [
        0u32,
        crate::slime::slime_source(0),
        crate::rot_sticker::rot_source(0),
    ] {
        inject(
            &mut rig,
            CombatEvent::HeroHurt {
                tick: t,
                source,
                health_left: 3,
                hero: HeroId::Carl,
            },
        );
    }
    // Not acid: another hazard class on a goblin.
    inject(
        &mut rig,
        CombatEvent::EnvironmentHurt {
            tick: t,
            target: 0,
            prop: 1,
            class: HazardClass::Steam,
            damage: 1,
            health_left: 1,
            player: None,
        },
    );
    cues.step(t, rig.combat(), hero_x(&rig));
    assert!(cues.slots.is_empty(), "{:?}", cues.slots);
    // Acid on Donut, on a goblin that lived, on a goblin that died.
    inject(&mut rig, acid_hurt(t, 1, HeroId::Donut));
    inject(
        &mut rig,
        CombatEvent::EnvironmentHurt {
            tick: t,
            target: 0,
            prop: 1,
            class: HazardClass::Acid,
            damage: 1,
            health_left: 1,
            player: Some(0),
        },
    );
    inject(
        &mut rig,
        CombatEvent::EnvironmentKill {
            tick: t,
            target: 1,
            prop: 1,
            class: HazardClass::Acid,
            player: Some(0),
        },
    );
    cues.step(t + 1, rig.combat(), hero_x(&rig));
    let actors: Vec<Actor> = cues.slots.iter().map(|s| s.actor).collect();
    assert_eq!(
        actors,
        vec![Actor::Hero(HeroId::Donut), Actor::Enemy(0), Actor::Enemy(1)]
    );
    // A goblin's x is its own.
    let gx = rig.combat().enemies[0].x;
    assert_eq!(cues.slots[1].x, gx);
    // The log is scanned once: stepping again adds nothing.
    cues.step(t + 2, rig.combat(), hero_x(&rig));
    assert_eq!(cues.slots.len(), 3);
}

/// Rendering never changes authoritative state: a run that steps and views the cues on
/// every tick has, at every tick, the state hash of a run that never does.
#[test]
fn drawing_never_changes_the_state_hash() {
    let build = || {
        Layout {
            acid: vec![(POOL.0, POOL.1, Some(0))],
            slimes: vec![SlimeAt {
                x: 400,
                facing: 1,
                home: 400,
            }],
            carl_x: 640,
            ..Layout::default()
        }
        .sim(Some(true))
    };
    let (mut plain, mut drawn) = (build(), build());
    let layout_h = drawn.scene().world.layout_height();
    let mut cues = AcidSplashes::default();
    let mut splashed = 0;
    for k in 0..300u64 {
        let input = if k < 120 { walk(1.0) } else { RawInput::NONE };
        plain.step(input, RawInput::NONE);
        drawn.step(input, RawInput::NONE);
        let tick = drawn.tick();
        cues.step(tick, drawn.combat(), |h| drawn.x_of(h));
        let d = cues.view(tick, drawn.combat().env().unwrap(), layout_h);
        splashed += d.len();
        for draw in &d {
            let _ = draw.sprites();
        }
        assert_eq!(plain.hash(), drawn.hash(), "tick {k}");
        assert_eq!(plain.events().len(), drawn.events().len(), "tick {k}");
    }
    assert!(splashed > 0, "the run did splash");
}
