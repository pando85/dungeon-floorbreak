//! DM-C: the pure cue state against the real scene (the biscuit fixtures of
//! DM-B), the arc, the atlas analysis and the proof that drawing the cues never
//! changes a state hash.

use super::*;
use crate::agility::ReflexProfile;
use crate::agility_fixtures::{DM_EVASION_UNLOCKS_AT_GUILD, DM_REFLEX_EVADES_SWING};
use crate::boss_scene::Pilot;
use crate::donut_mana::DonutSetup;
use crate::env_kit::EnvKit;
use crate::party::{PROVISIONAL_TUNING, PartyTuning};
use crate::party_scene::{PartyFixture, PartyFixtureSim, base};

fn corridor() -> EnvKit {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    EnvKit::load(&root, "assets/levels/floor01/corridor_abc_sample.json").unwrap()
}

/// An empty room with the biscuit from tick 0: only what a test injects hurts her.
fn empty_sim() -> PartyFixtureSim {
    static BASE: crate::combat_scene::Fixture =
        base("dm_cues_empty", "empty corridor", 0x0D4D_00C0, &[], 2000);
    let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        base: &BASE,
        two_player: true,
        // The glint timing is read from the arm tick: the profile without the
        // FA-Q19 armed start arms at 720 in this room.
        tuning: PartyTuning {
            donut_reflex: ReflexProfile::ShippedNoOpener,
            ..PROVISIONAL_TUNING
        },
        script: &[],
        pilot: Pilot::None,
        donut: DonutSetup::unlocked(8),
    }));
    PartyFixtureSim::new(&corridor(), 360, fixture).unwrap()
}

/// Step the sim and the cues together; returns the cues stepped every tick.
fn drive(sim: &mut PartyFixtureSim, cues: &mut DonutCues, ticks: u64) {
    for _ in 0..ticks {
        sim.step_scripted();
        cues.step(
            sim.scene.world.tick,
            &sim.combat,
            sim.scene.world.donut_body.x,
        );
    }
}

#[test]
fn the_arc_rises_to_ten_pixels_and_lands_in_twelve_ticks() {
    let lifts: Vec<i32> = (0..=EVADE_TICKS).map(arc_lift_px).collect();
    assert_eq!(lifts[0], 0);
    assert_eq!(lifts[6], ARC_PEAK_PX, "the peak is the middle tick");
    assert_eq!(lifts[12], 0, "she is back on the ground at EVADE_TICKS");
    assert_eq!(arc_lift_px(EVADE_TICKS + 5), 0);
    assert!(lifts.iter().all(|l| (0..=ARC_PEAK_PX).contains(l)));
    assert!(lifts[..=6].windows(2).all(|w| w[0] <= w[1]), "{lifts:?}");
    assert!(lifts[6..].windows(2).all(|w| w[0] >= w[1]), "{lifts:?}");
    for age in 0..=EVADE_TICKS {
        assert_eq!(
            arc_lift_px(age),
            arc_lift_px(EVADE_TICKS - age),
            "symmetric"
        );
    }
}

/// Spec 3.4: while armed, the glint blinks 4 ticks on and 26 off from the arm
/// tick; the flick (DONUT-MOVES-ART-01 spec 3) is `flick_up` for 2 ticks, then
/// `flick_snap` for 2, then the base locomotion frame; nothing before the arm.
#[test]
fn the_glint_blinks_four_on_twenty_six_off_from_the_arm_and_the_flick_is_six_ticks() {
    let mut sim = empty_sim();
    let mut cues = DonutCues::default();
    let mut glint = Vec::new();
    let mut flick = Vec::new();
    let mut twinkle = Vec::new();
    for _ in 0..800 {
        drive(&mut sim, &mut cues, 1);
        let t = sim.scene.world.tick;
        glint.push((t, cues.glint_on(t)));
        flick.push((t, cues.flick(t)));
        twinkle.push((t, cues.glint_age(t)));
    }
    let on: Vec<u64> = glint.iter().filter(|g| g.1).map(|g| g.0).collect();
    // Armed on tick 720 (DM-B): on for 720..=723, off 724..=749, on again 750..=753.
    assert_eq!(on[..4], [720, 721, 722, 723], "{on:?}");
    assert_eq!(on[4..8], [750, 751, 752, 753], "{on:?}");
    assert!(on.iter().all(|t| *t >= 720), "nothing before the arm");
    // The glint's age inside its window: 0, 1, 2, then 3 (the small twinkle).
    let ages: Vec<u64> = twinkle.iter().filter_map(|g| g.1).take(8).collect();
    assert_eq!(ages, [0, 1, 2, 3, 0, 1, 2, 3]);
    let flicks: Vec<(u64, usize)> = flick.iter().filter_map(|f| f.1.map(|o| (f.0, o))).collect();
    assert_eq!(
        flicks,
        [(720, 0), (721, 0), (722, 1), (723, 1)],
        "flick_up 2 ticks, flick_snap 2 ticks, then the base frame (FLICK_TICKS stays {FLICK_TICKS})"
    );
}

/// An evade spends the arm: the glint is gone from that tick on. (In this room
/// the goblin goes on to hurt her every swing, each hurt restarting the wait, so
/// it never returns inside the run; the 720 tick rule itself is DM-B's.)
#[test]
fn the_glint_goes_with_the_evade() {
    let mut sim = PartyFixtureSim::new(&corridor(), 360, &DM_REFLEX_EVADES_SWING).unwrap();
    let mut cues = DonutCues::default();
    let mut lit = Vec::new();
    for _ in 0..1700 {
        drive(&mut sim, &mut cues, 1);
        let t = sim.scene.world.tick;
        if cues.glint_on(t) {
            lit.push(t);
        }
    }
    // FA-Q19: the fixture starts with the biscuit, so it is armed from tick 1.
    // The evade on 729 spends the arm: lit 4 on and 26 off from tick 1 to the last
    // window 721..=724, then dark until the hurt restarts the attempt, which
    // starts armed again on its tick 1 (every restart is a new attempt).
    let first: Vec<u64> = (1..=729).filter(|t| (t - 1) % 30 < 4).collect();
    assert_eq!(
        lit[..first.len()],
        first[..],
        "lit while armed (4 on), gone after"
    );
    assert_eq!(
        lit[first.len()..first.len() + 4],
        [1, 2, 3, 4],
        "re-armed by the restart"
    );
}

/// Spec 3.4 evade response: 12 ticks exactly (last tick in, first tick out),
/// the swish for its first 2 and the arc (the afterimages are retired, DMA-Q06).
#[test]
fn the_evade_response_runs_exactly_twelve_ticks_with_a_two_tick_streak() {
    let mut sim = PartyFixtureSim::new(&corridor(), 360, &DM_REFLEX_EVADES_SWING).unwrap();
    let mut cues = DonutCues::default();
    let mut views = Vec::new();
    for _ in 0..760 {
        drive(&mut sim, &mut cues, 1);
        let t = sim.scene.world.tick;
        views.push((t, cues.evade(t)));
    }
    let shown: Vec<u64> = views
        .iter()
        .filter(|v| v.1.is_some())
        .map(|v| v.0)
        .collect();
    assert_eq!(
        shown,
        (729..=740).collect::<Vec<_>>(),
        "the event on 729, 12 ticks"
    );
    let view = |t: u64| views.iter().find(|v| v.0 == t).unwrap().1;
    assert_eq!(view(728), None, "the tick before");
    assert_eq!(view(741), None, "the first tick out");
    let first = view(729).unwrap();
    assert_eq!((first.age, first.lift_px), (0, 0));
    assert!(first.streak.is_some() && view(730).unwrap().streak.is_some());
    assert_eq!(view(731).unwrap().streak, None, "two ticks");
    assert_eq!(view(735).unwrap().lift_px, ARC_PEAK_PX);
    assert_eq!(view(740).unwrap().lift_px, arc_lift_px(11));
    // The goblin stood to her right or left: the streak says which.
    let g = sim.combat.enemies[0].x;
    let _ = g;
    assert!(
        matches!(first.streak, Some(-1) | Some(1)),
        "{:?}",
        first.streak
    );
    // The locomotion sprite is replaced exactly while the arc runs.
    assert!(cues.replaces_locomotion(740) && !cues.replaces_locomotion(741));
}

/// A render that skips frames, or a viewer that seeks, draws the same picture for
/// the same tick: the cues are recorded from the events' own ticks.
#[test]
fn the_cues_depend_on_event_ticks_not_on_when_they_were_noticed() {
    let mut sim = PartyFixtureSim::new(&corridor(), 360, &DM_REFLEX_EVADES_SWING).unwrap();
    let mut every = DonutCues::default();
    let mut views = Vec::new();
    for _ in 0..745 {
        drive(&mut sim, &mut every, 1);
        let t = sim.scene.world.tick;
        views.push((t, every.evade(t), every.flick(t)));
    }
    // A second presenter that first looks at the run on tick 745.
    let mut late = DonutCues::default();
    late.step(
        sim.scene.world.tick,
        &sim.combat,
        sim.scene.world.donut_body.x,
    );
    for (t, evade, flick) in views {
        assert_eq!(late.evade(t), evade, "tick {t}");
        assert_eq!(late.flick(t), flick, "tick {t}");
    }
}

/// A restart (the tick goes back and the log is shorter) clears every cue.
#[test]
fn a_restart_or_a_seek_back_clears_the_cues() {
    let mut sim = PartyFixtureSim::new(&corridor(), 360, &DM_REFLEX_EVADES_SWING).unwrap();
    let mut cues = DonutCues::default();
    drive(&mut sim, &mut cues, 735);
    assert!(cues.evade(735).is_some());
    let fresh = PartyFixtureSim::new(&corridor(), 360, &DM_REFLEX_EVADES_SWING).unwrap();
    cues.step(5, &fresh.combat, 0.0);
    assert_eq!(cues.evade(735), None);
    assert!(!cues.glint_on(5));
}

/// Carl's evade (he has none) and the guild flow: the biscuit unlock arms her
/// 720 ticks later and the glint follows that, not the unlock.
#[test]
fn the_glint_follows_the_arm_not_the_unlock() {
    let mut sim = PartyFixtureSim::new(&corridor(), 360, &DM_EVASION_UNLOCKS_AT_GUILD).unwrap();
    let mut cues = DonutCues::default();
    let mut first_lit = None;
    for _ in 0..760 {
        drive(&mut sim, &mut cues, 1);
        let t = sim.scene.world.tick;
        if first_lit.is_none() && cues.glint_on(t) {
            first_lit = Some(t);
        }
    }
    assert_eq!(first_lit, Some(740), "unlock on 20 plus 720");
}

/// Spec 7: a replay with the presentation on and off gives an identical state
/// hash. The cues read `&CombatState` only; stepping them every tick (or never)
/// leaves the hash of every tick equal, on a run that arms, evades and is hurt.
#[test]
fn stepping_the_cues_never_changes_a_state_hash() {
    let mut with = PartyFixtureSim::new(&corridor(), 360, &DM_REFLEX_EVADES_SWING).unwrap();
    let mut without = PartyFixtureSim::new(&corridor(), 360, &DM_REFLEX_EVADES_SWING).unwrap();
    let mut cues = DonutCues::default();
    for tick in 1..=1000_u64 {
        with.step_scripted();
        cues.step(
            with.scene.world.tick,
            &with.combat,
            with.scene.world.donut_body.x,
        );
        // Ask for every view too: reading is free as well.
        let _ = (cues.glint_on(tick), cues.flick(tick), cues.evade(tick));
        without.step_scripted();
        assert_eq!(with.state_hash(), without.state_hash(), "tick {tick}");
    }
    assert!(
        with.combat
            .events
            .iter()
            .any(|e| matches!(e, CombatEvent::HeroEvaded { .. })),
        "the run did evade"
    );
}

// ------------------------------------------------------------- the atlas ----

/// A 2-frame, 8 x 6 atlas: frame 0 has a tail rising on the left, frame 1 is empty.
fn atlas() -> Vec<u8> {
    let (w, h, frames) = (8_usize, 6_usize, 2_usize);
    let mut rgba = vec![0_u8; w * frames * h * 4];
    let mut put = |frame: usize, x: usize, y: usize, c: [u8; 4]| {
        let at = (y * w * frames + frame * w + x) * 4;
        rgba[at..at + 4].copy_from_slice(&c);
    };
    // Tail: a column on the left whose top is (1, 0), its outer edge at x = 0; body
    // on the right (the ears of a head are higher but further forward).
    for y in 0..4 {
        put(0, 1, y, [200, 120, 40, 255]);
    }
    put(0, 0, 3, [90, 60, 20, 255]);
    for x in 3..8 {
        for y in 3..6 {
            put(0, x, y, [70, 40, 30, 255]);
        }
    }
    put(0, 6, 4, [250, 240, 230, 255]); // the brightest colour
    put(0, 2, 5, [255, 255, 255, 100]); // translucent: ignored
    rgba
}

#[test]
fn the_tail_tip_is_the_topmost_pixel_of_the_two_leftmost_columns_and_the_highlight_the_brightest() {
    let cues = analyze_atlas(&atlas(), 16, 8, 6, 2);
    let tip = cues.tips[0].as_ref().expect("a tip");
    assert_eq!((tip.x, tip.y), (1, 0));
    assert_eq!(cues.tips[1], None, "an empty frame has no tip");
    assert_eq!(cues.highlight, [250, 240, 230]);
    // The cluster is the opaque 3 x 3 neighbourhood of the tip: (1,0) and (1,1).
    let mut cells: Vec<(u32, u32)> = tip.cluster.iter().map(|p| (p.0, p.1)).collect();
    cells.sort_unstable();
    assert_eq!(cells, [(1, 0), (1, 1)]);
}

#[test]
fn a_truncated_atlas_gives_empty_frames_not_a_panic() {
    let cues = analyze_atlas(&atlas()[..40], 16, 8, 6, 2);
    assert_eq!(cues.tips.len(), 2);
}

/// The evidence room (`cf02_dm_reflex_cues`): armed 720, evaded 729, nothing
/// hurts her, the goblin dies, and the glint comes back with the second arming
/// 720 ticks after the evade.
#[test]
fn the_cue_room_arms_evades_and_arms_again() {
    use crate::agility_fixtures::CF02_DM_REFLEX_CUES;
    let mut sim = PartyFixtureSim::new(&corridor(), 360, &CF02_DM_REFLEX_CUES).unwrap();
    let mut cues = DonutCues::default();
    let mut lit = Vec::new();
    for _ in 0..1800 {
        drive(&mut sim, &mut cues, 1);
        let t = sim.scene.world.tick;
        if cues.glint_on(t) {
            lit.push(t);
        }
    }
    let ticks_of = |pick: fn(&CombatEvent) -> Option<u64>| -> Vec<u64> {
        sim.combat.events.iter().filter_map(pick).collect()
    };
    assert_eq!(
        ticks_of(|e| match *e {
            CombatEvent::EvadeArmed { tick, .. } => Some(tick),
            _ => None,
        }),
        [1, 1449]
    );
    assert_eq!(
        ticks_of(|e| match *e {
            CombatEvent::HeroEvaded { tick, .. } => Some(tick),
            _ => None,
        }),
        [729]
    );
    assert!(
        sim.combat.events.iter().all(|e| !matches!(
            e,
            CombatEvent::HeroHurt { .. } | CombatEvent::Restart { .. }
        )),
        "nobody is hurt and nothing restarts"
    );
    assert!(!sim.combat.enemies[0].alive(), "Carl put the goblin down");
    // FA-Q19: armed from tick 1, so the glint blinks 4 on and 26 off (1..=4, 31..=34
    // and so on, the last window 721..=724) until the evade on 729 spends it; dark
    // to the second arming on 1449, then blinking again.
    let first: Vec<u64> = (1..=729).filter(|t| (t - 1) % 30 < 4).collect();
    assert_eq!(lit[..first.len()], first[..], "{lit:?}");
    let rest = &lit[first.len()..];
    assert_eq!(rest[..4], [1449, 1450, 1451, 1452]);
    assert_eq!(rest[4..8], [1479, 1480, 1481, 1482]);
}
