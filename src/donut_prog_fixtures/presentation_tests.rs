//! DONUT-PROG-01 DP-D tests: the presentation of the blink over the DP-C rig.
//! Presentation never changes an outcome: the HUD, the cues and the drawn camera
//! only read; the camera test is the arbiter of spec 5.2 rule 5 (DP-Q46) for a
//! human-controlled Donut.

use super::blink_fixtures::{Rig, Story, at, circle, input_at, stick};
use crate::blink_cues::BlinkCues;
use crate::camera_lab::camera_op::{Aspect, Bounds};
use crate::camera_lab::sim::DONUT_HALF_WIDTH;
use crate::combat::CombatEvent;
use crate::donut_hud;
use crate::hud::{HudConfig, HudRuntime, SpecialSlot};
use crate::input::RawInput;
use crate::party_scene::PartyFixtureSim;
use crate::scene_camera::{
    CameraChoice, CameraInterest, InterestKind, InterestThreat, RigConfig, SceneCameraRig,
};

const NONE: RawInput = RawInput::NONE;

fn resolved_ticks(sim: &PartyFixtureSim) -> Vec<u64> {
    sim.combat
        .events
        .iter()
        .filter_map(|e| match e {
            CombatEvent::BlinkResolved { tick, .. } => Some(*tick),
            _ => None,
        })
        .collect()
}

/// One run of the drawn camera over a human Donut's blinks: the worst run of
/// consecutive ticks her body was outside the drawn view after an arrival, and
/// the number of arrivals.
fn drawn_view_worst(
    carl_x: i32,
    donut_x: f32,
    aspect: Aspect,
    out_w: u32,
    dir: f32,
    threat: bool,
) -> (u64, usize, i32) {
    let rig = Rig::new()
        .carl_x(carl_x)
        .donut_x(donut_x)
        .story(Story::Puddle);
    let kit = rig.kit();
    let mut sim = rig.build();
    let mut interest = CameraInterest::default();
    let mut cam = SceneCameraRig::new(
        RigConfig {
            choice: CameraChoice::Auto,
            aspect,
            out_w,
            bounds: Bounds {
                w: kit.width,
                // The 400 px corridor caps the view at 711 px: a taller camera room
                // lets the 960 rest exist (presentation bounds only).
                h: kit.height.max(1080),
            },
        },
        &sim.scene,
        Some(&sim.combat),
        &interest,
        0,
    )
    .expect("the automatic camera");
    // Two blinks, the second after the cooldown, each with the stick held.
    let presses = [120u64, 120 + 6 + 540 + 20];
    let mut holds = Vec::new();
    for p in presses {
        holds.push((p, p + 1, circle()));
        holds.push((p, p + 8, stick(dir)));
    }
    let mut outside_run = 0u64;
    let mut worst = 0u64;
    let mut width = 0;
    for tick in 1..=presses[1] + 80 {
        if threat {
            // A far readable threat ahead of both heroes widens the camera to its
            // 960 rest (a static stand-in for a goblin tell; presentation input only).
            interest.clear();
            interest.push(InterestThreat {
                kind: InterestKind::Tell { ticks_left: 600 },
                x: carl_x.max(donut_x as i32) as f32 + 300.0,
                y: sim.scene.world.donut.foot_y + 30.0,
                half_w: 20.0,
                half_h: 30.0,
                dir: -1,
                lane: 0.0,
            });
        }
        sim.step_slots([NONE, input_at(&holds, tick)]);
        let view = cam.sync(&sim.scene, Some(&sim.combat), &interest, tick);
        width = width.max(view.w);
        let x = sim.scene.world.donut.x;
        let inside = x - DONUT_HALF_WIDTH >= view.left as f32
            && x + DONUT_HALF_WIDTH <= (view.left + view.w) as f32;
        let after_arrival = resolved_ticks(&sim).iter().any(|t| tick >= *t);
        if after_arrival && !inside {
            outside_run += 1;
            worst = worst.max(outside_run);
        } else {
            outside_run = 0;
        }
    }
    (worst, resolved_ticks(&sim).len(), width)
}

/// `donut_prog_blink_drawn_view_human` (DP-Q46, spec 5.2 rule 5): after every
/// `BlinkResolved` of a human-controlled Donut (a framing subject) her camera body
/// is back inside the drawn automatic-camera view within
/// `dpq46_drawn_view_grace_ticks` (12) ticks, at 640 and 960 rest widths and at
/// 16:9 and 16:10. The measured worst case is printed (`--nocapture`) and goes in
/// the PR body.
#[test]
fn donut_prog_blink_drawn_view_human() {
    let grace =
        u64::from(crate::donut_prog::DonutProgTuning::active().dpq46_drawn_view_grace_ticks);
    let mut table = Vec::new();
    let mut widths = std::collections::BTreeSet::new();
    let mut total_blinks = 0;
    for aspect in [Aspect::Wide16x9, Aspect::Deck16x10] {
        for out_w in [1920u32, 3840, 1280] {
            // Close pairs (a 640 rest) and far pairs (a 960 rest), both ways, the
            // blink toward and away from Carl.
            for (carl_x, donut_x) in [
                (560, 640.0),
                (560, 700.0),
                (440, 760.0),
                (300, 640.0),
                (700, 640.0),
                (700, 340.0),
                (240, 640.0),
                (200, 620.0),
                (700, 270.0),
            ] {
                for (dir, threat) in [(1.0, false), (-1.0, false), (1.0, true), (-1.0, true)] {
                    let (worst, blinks, width) =
                        drawn_view_worst(carl_x, donut_x, aspect, out_w, dir, threat);
                    widths.insert(width);
                    // A press toward a view edge may be refused (DP-Q27 rule 5), so
                    // count what happened rather than demand both.
                    total_blinks += blinks;
                    assert!(blinks >= 1, "{aspect:?} {out_w} {carl_x} {donut_x} {dir}");
                    table.push((aspect, out_w, carl_x, donut_x as i32, dir as i32, worst));
                    assert!(
                        worst <= grace,
                        "{aspect:?} out_w {out_w} carl {carl_x} donut {donut_x} dir {dir}: \
                         {worst} ticks outside (grace {grace})"
                    );
                }
            }
        }
    }
    // The spec requires the 640 and the 960 rest: a later camera change must not
    // silently drop one.
    assert!(
        widths.contains(&640) && widths.contains(&960),
        "rest widths reached: {widths:?}"
    );
    let worst = table.iter().map(|t| t.5).max().unwrap();
    eprintln!(
        "DONUT_PROG_BLINK_DRAWN_VIEW worst_ticks_outside={worst} widths={widths:?} cases={} blinks={total_blinks}",
        table.len()
    );
}

/// Presentation never changes an outcome: a run that steps the HUD, the blink
/// cues and the drawn camera every tick ends on the hash of a run that steps none
/// of them, and the event logs match. (The HUD and the cues read `&CombatState`
/// only; this is the end-to-end proof.)
#[test]
fn the_blink_presentation_changes_no_hash() {
    let holds = vec![at(10, circle()), at(300, circle()), at(640, circle())];
    let ticks = 700u64;
    let plain = {
        let mut sim = Rig::new().build();
        for tick in 1..=ticks {
            sim.step_slots([NONE, input_at(&holds, tick)]);
        }
        sim
    };
    let rig = Rig::new();
    let kit = rig.kit();
    let mut sim = rig.build();
    let mut hud = HudRuntime::new(HudConfig::default());
    hud.set_melee_live(true);
    let mut cues = BlinkCues::default();
    let interest = CameraInterest::default();
    let mut cam = SceneCameraRig::new(
        RigConfig {
            choice: CameraChoice::Auto,
            aspect: Aspect::Wide16x9,
            out_w: 1920,
            bounds: Bounds {
                w: kit.width,
                h: kit.height,
            },
        },
        &sim.scene,
        Some(&sim.combat),
        &interest,
        0,
    )
    .unwrap();
    for tick in 1..=ticks {
        sim.step_slots([NONE, input_at(&holds, tick)]);
        hud.set_donut_special(donut_hud::blink_socket_of(
            &sim.combat,
            sim.scene.world.tick,
        ));
        hud.step(
            sim.scene.world.hero.health,
            sim.scene.world.donut.health,
            &sim.combat.events,
            sim.combat.dropped_events,
        );
        let donut = &sim.scene.world.donut;
        cues.step(
            tick,
            &sim.combat,
            donut.x.round() as i32,
            donut.foot_y.round() as i32,
        );
        let _ = cues.view(tick);
        cam.sync(&sim.scene, Some(&sim.combat), &interest, tick);
    }
    assert_eq!(sim.state_hash(), plain.state_hash());
    assert_eq!(sim.combat.events, plain.combat.events);
    assert!(!resolved_ticks(&sim).is_empty());
}

/// The socket follows the blink through a whole cycle, from the real simulation:
/// equipped and ready, muted in the windup, cooling with a full underline on the
/// arrival tick and a shrinking one after, ready again on the ready tick with the
/// one-shot flash.
#[test]
fn the_socket_follows_a_whole_blink_cycle() {
    let holds = vec![at(10, circle())];
    let mut sim = Rig::new().build();
    let mut hud = HudRuntime::new(HudConfig::default());
    hud.set_melee_live(true);
    let mut slots = Vec::new();
    let mut flashes = Vec::new();
    for tick in 1..=10 + 6 + 560 {
        sim.step_slots([NONE, input_at(&holds, tick)]);
        hud.set_donut_special(donut_hud::blink_socket_of(
            &sim.combat,
            sim.scene.world.tick,
        ));
        hud.step(
            sim.scene.world.hero.health,
            sim.scene.world.donut.health,
            &sim.combat.events,
            sim.combat.dropped_events,
        );
        slots.push(hud.real_inputs().donut_special);
        flashes.push(hud.snapshot().plates[1].socket_flash.map(|f| f.kind));
    }
    let at_tick = |t: usize| slots[t - 1];
    let chip = |muted| crate::hud::Chip {
        action: crate::hud::LogicalAction::Special,
        label: crate::hud::LabelId::Blink,
        muted,
    };
    assert_eq!(at_tick(9), SpecialSlot::Equipped(chip(false)), "ready");
    // t0 = tick 10 through t5: the windup, muted, no underline yet.
    for t in 10..16 {
        assert_eq!(at_tick(t), SpecialSlot::Equipped(chip(true)), "windup {t}");
    }
    // The arrival (t6 = 16): the cooldown starts, 540 left: a full underline.
    let cooling = |bar_px| SpecialSlot::Cooling {
        chip: chip(true),
        bar_px,
    };
    assert_eq!(at_tick(16), cooling(11));
    // 270 left on tick 16 + 270 = 286: 6 px; 1 left on tick 555: 1 px.
    assert_eq!(at_tick(286), cooling(6));
    assert_eq!(at_tick(555), cooling(1));
    // Ready on arrival + 540 = 556 (and not on 555), with the flash.
    assert_eq!(at_tick(556), SpecialSlot::Equipped(chip(false)));
    assert_eq!(flashes[556 - 1], Some(crate::hud::SocketFlashKind::Ready));
    assert_eq!(flashes[555 - 1], None);
    assert_eq!(flashes[561 - 1], Some(crate::hud::SocketFlashKind::Ready));
    assert_eq!(flashes[562 - 1], None, "the flash is six ticks, 556..=561");
}

/// The cue state reads the real event log: the puddle opens on the press tick at
/// her feet, the arrival opens the destination puddle, closes the origin one and
/// adds the one-frame streak and the rising frames, a seek back clears it.
#[test]
fn the_cues_read_the_real_event_log() {
    let holds = vec![at(10, circle())];
    let mut sim = Rig::new().build();
    let mut cues = BlinkCues::default();
    let mut views = Vec::new();
    let start_x = sim.scene.world.donut.x.round() as i32;
    for tick in 1..=30u64 {
        sim.step_slots([NONE, input_at(&holds, tick)]);
        let donut = &sim.scene.world.donut;
        cues.step(
            tick,
            &sim.combat,
            donut.x.round() as i32,
            donut.foot_y.round() as i32,
        );
        views.push(cues.view(tick));
    }
    assert!(views[8].puddles.is_empty(), "before the press");
    assert_eq!(views[9].puddles.len(), 1, "the press tick");
    assert_eq!(views[9].puddles[0].x, start_x);
    assert_eq!(views[9].puddles[0].frame, 0, "ring_open_0");
    assert!(views[9].streak.is_none());
    // The arrival is tick 16: the streak for one tick, a puddle at the destination
    // (ring_open_0), the origin puddle closing (ring_close_0) and her pop frame.
    let s = views[15].streak.expect("the streak on the arrival tick");
    assert_eq!((s.from_x, s.to_x), (start_x, start_x + 112));
    assert!(
        views[15]
            .puddles
            .iter()
            .any(|r| r.x == start_x + 112 && r.frame == 0)
    );
    assert!(
        views[15]
            .puddles
            .iter()
            .any(|r| r.x == start_x && r.frame == 4)
    );
    assert_eq!(views[15].arrive, Some(8), "age 6: blink_pop is frame 8");
    assert!(views[16].streak.is_none());
    assert_eq!(views[17].arrive, Some(9), "age 8: the blink_arrive key");
    cues.step(5, &sim.combat, 0, 0);
    assert!(cues.view(5).puddles.is_empty(), "a seek back resets");
}

/// DP-Q12 end to end over the real grants fixture: the HUD, fed as the scene feeds
/// it, shows LIGHT FEET on the grant tick, then PUDDLE JUMPER 136 + 30 ticks later,
/// and her socket turns from the padlock to the equipped chip on the grant tick.
#[test]
fn the_grants_fixture_raises_both_notices_and_equips_the_socket() {
    let fixture = crate::cf02_fixtures::find_party_fixture("donut_prog_grants_2p").unwrap();
    let kit = crate::env_kit::EnvKit::load(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
        "assets/levels/floor01/corridor_abc_sample.json",
    )
    .unwrap();
    let mut sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
    let mut hud = HudRuntime::new(HudConfig::default());
    hud.set_melee_live(true);
    // Two players: P2 drives Donut, so her plate has the chips and the socket.
    hud.set_party(crate::hud::PartyView {
        donut: crate::hud::Control::Player(2),
        donut_chips: true,
        ..crate::hud::PartyView::default()
    });
    let mut notices = Vec::new();
    let mut socket_at_245 = None;
    let mut socket_at_246 = None;
    for tick in 1..=fixture.base.ticks {
        sim.step_scripted();
        hud.set_donut_special(donut_hud::blink_socket_of(
            &sim.combat,
            sim.scene.world.tick,
        ));
        hud.step(
            sim.scene.world.hero.health,
            sim.scene.world.donut.health,
            &sim.combat.events,
            sim.combat.dropped_events,
        );
        let snap = hud.snapshot();
        if let Some(n) = snap.notice
            && n.age_ticks == 0
        {
            notices.push((tick, n.headline));
        }
        let socket = match snap.plates[1].abilities {
            crate::hud::AbilityRow::Chips { special, .. }
            | crate::hud::AbilityRow::NoActions { special } => Some(special),
            crate::hud::AbilityRow::AiCompanion => None,
        };
        match tick {
            245 => socket_at_245 = socket,
            246 => socket_at_246 = socket,
            _ => {}
        }
    }
    assert_eq!(
        notices,
        [
            (246, crate::hud::LabelId::LightFeet),
            (246 + 136 + 30, crate::hud::LabelId::PuddleJumper)
        ]
    );
    assert_eq!(socket_at_245, Some(SpecialSlot::Locked));
    assert!(matches!(socket_at_246, Some(SpecialSlot::Equipped(_))));
}
