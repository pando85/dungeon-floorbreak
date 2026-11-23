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

fn props_of(art: &HoarderArt, prop: Prop) -> Vec<PropDraw> {
    art.props
        .iter()
        .copied()
        .filter(|p| p.prop == prop)
        .collect()
}

/// The spec's state table (section 4), mode by mode and at both edges of every window.
#[test]
fn the_frame_table_follows_the_hoard_02_spec() {
    let look = HoarderLook::default();
    let f = |mode, mt| frame_for(mode, mt, 0, look);
    // Swipe (unchanged): gather 7 for mode ticks 1..8, rear-up 8 for 9..34, smear 9, impact 10,
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
    assert_eq!(f(PileMode::ShoveRecover, 1), 11);
    // Shove tell: the settle 16 for ticks 1..10, the brace 17 for 11..40 (never the swipe's gather).
    assert_eq!(f(PileMode::ShoveTell, 1), 16);
    assert_eq!(f(PileMode::ShoveTell, 10), 16);
    assert_eq!(f(PileMode::ShoveTell, 11), 17);
    assert_eq!(f(PileMode::ShoveTell, 40), 17);
    // Lunge: 18 and 19 alternate every 6 ticks, starting on 18.
    for (mt, frame) in [(1, 18), (6, 18), (7, 19), (12, 19), (13, 18), (48, 19)] {
        assert_eq!(f(PileMode::Lunge, mt), frame, "lunge tick {mt}");
    }
    // Jammed: the impact 20 for ticks 1..12, the stagger 21 from 13 to the end of the 72.
    for (mt, frame) in [(1, 20), (12, 20), (13, 21), (72, 21)] {
        assert_eq!(f(PileMode::Jammed, mt), frame, "jammed tick {mt}");
    }
    // Flinch 12. Exposed: 22 and 23 alternate every 20 ticks; Get-up: 24 for 1..12, 25 for 13..24.
    assert_eq!(f(PileMode::Flinch, 1), 12);
    for (mt, frame) in [(1, 22), (20, 22), (21, 23), (40, 23), (41, 22), (150, 23)] {
        assert_eq!(f(PileMode::Exposed, mt), frame, "exposed tick {mt}");
    }
    for (mt, frame) in [(1, 24), (12, 24), (13, 25), (24, 25)] {
        assert_eq!(f(PileMode::GetUp, mt), frame, "get-up tick {mt}");
    }
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

/// The Reveal is cut into thirds of its length: 26, 27, 28 for 90 ticks (30 each) and for the 30-tick
/// retry (10 each), and every frame of the table is inside the atlas.
#[test]
fn the_reveal_shows_the_emerge_in_thirds() {
    for (len, third) in [(90u32, 30u32), (30, 10)] {
        let look = HoarderLook {
            reveal_ticks: len,
            ..HoarderLook::default()
        };
        for mt in 1..=len {
            let expected = 26 + (mt - 1) / third;
            assert_eq!(
                u32::from(frame_for(PileMode::Reveal, mt, 0, look)),
                expected,
                "reveal {len}, tick {mt}"
            );
        }
    }
    // An unknown length is the first Reveal's 90.
    let unknown = HoarderLook::default();
    assert_eq!(frame_for(PileMode::Reveal, 31, 0, unknown), 27);
    assert_eq!(frame_for(PileMode::Reveal, 61, 0, unknown), 28);
    for mode in PileMode::ALL {
        for mt in [1, 12, 13, 100] {
            for tick in [0, 17, 90] {
                assert!(u32::from(frame_for(mode, mt, tick, HoarderLook::default())) < FRAMES);
            }
        }
    }
}

/// The `down` frames mean defeat and nothing else (the Engineer U1 lesson): 13 for 8 ticks, 14
/// for 12, then 15 held; no live mode ever shows one, Exposed (pinned) included.
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

/// The room pilot's run, tick by tick: her sprite and the room kit are drawn from the sim's own states,
/// every frame is inside the atlas, and no stand-in text, label or code-drawn piece remains.
#[test]
fn every_drawn_frame_is_inside_the_atlas_and_nothing_is_a_stand_in() {
    let mut rig = Rig::new(&root(), clear_plan()).unwrap();
    let mut pilot = ClearPilot::new(0);
    let mut modes = std::collections::BTreeSet::new();
    let mut drawn = 0;
    for _ in 0..1400 {
        let input = pilot.input(&rig.combat, &rig.scene);
        rig.step(carl_input(input));
        let art = art_of(&rig, rig.scene.world.tick, HoarderLook::default());
        if let Some(h) = art.hoarder {
            drawn += 1;
            modes.insert(format!("{:?}", rig.boss().boss().mode()));
            assert!(u32::from(h.frame) < FRAMES);
            assert_eq!(h.x, rig.boss().boss().x());
        }
        for p in &art.props {
            assert!(
                u32::from(p.frame) < p.prop.frames(),
                "{:?} frame {}",
                p.prop,
                p.frame
            );
        }
        assert!(art.props.len() <= MAX_PROPS && art.motes.len() <= MAX_MOTES);
    }
    assert!(drawn > 600, "she was drawn for most of the run: {drawn}");
    assert!(modes.contains("Reveal") && modes.contains("Defeated"));
    assert!(modes.len() >= 6, "the pilot plays her modes: {modes:?}");
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

/// The door stands as a closed sprite on the lock solid until her defeat tick and is the open frame from
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
    let lock = rig.run.loaded.encounter.locks[0].solid;
    let layout_h = rig.scene.world.layout_height();
    let door = props_of(&before, Prop::Door);
    assert_eq!(door.len(), 1);
    assert_eq!(
        (door[0].frame, door[0].x, door[0].y),
        (0, lock.x, layout_h - lock.bottom()),
        "closed, on the lock solid"
    );
    assert_eq!(Prop::Door.frame_size(), (lock.w as u32, lock.h as u32));
    let mut defeat_tick = None;
    for _ in 0..30 {
        rig.step(carl_input(attack_input()));
        let art = art_of(&rig, rig.scene.world.tick, HoarderLook::default());
        let closed = props_of(&art, Prop::Door)[0].frame == 0;
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
    assert_eq!(props_of(&after, Prop::Door)[0].frame, 1, "open");
}

/// The entry gate is the closed frame while the sim's gate stands and the open frame otherwise
/// (before the engage, after her defeat); the sprite is the gate solid's rectangle.
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
        let gate = props_of(&art, Prop::Gate);
        assert_eq!(gate.len(), 1);
        assert_eq!(
            gate[0].frame == 1,
            stands.is_some(),
            "tick {}",
            rig.scene.world.tick
        );
        if let Some(rect) = stands {
            assert_eq!(
                (gate[0].x, gate[0].y),
                (rect.left as i32, rect.bottom as i32)
            );
            assert_eq!(
                Prop::Gate.frame_size(),
                (rect.width as u32, rect.height as u32)
            );
        }
        closed_seen |= gate[0].frame == 1;
        open_seen |= gate[0].frame == 0;
    }
    assert!(closed_seen && open_seen);
}

/// Bank A, the post and the stack come from the sidecar's statics in world px, standing on the walking
/// line at their standing states; the mound is behind everything, centred on her home column, sealed
/// (back and front) while she is Dormant; the heaps are behind everything, clear of the lane.
#[test]
fn the_room_kit_comes_from_the_sidecar_and_the_tuning() {
    let rig = Rig::new(&root(), clear_plan()).unwrap();
    let art = art_of(&rig, 0, HoarderLook::default());
    let floor = rig.boss().boss().setup().floor_y as i32;
    assert_eq!(
        props_of(&art, Prop::Bank),
        [PropDraw {
            prop: Prop::Bank,
            frame: 0,
            x: 352,
            y: floor,
            layer: Layer::Prop
        }],
        "bank A stable on [352, 416) x [0, 48)"
    );
    assert_eq!(
        props_of(&art, Prop::Post),
        [PropDraw {
            prop: Prop::Post,
            frame: 0,
            x: 808,
            y: floor,
            layer: Layer::Prop
        }],
        "the post intact on [808, 824) x [0, 64)"
    );
    // The stack's footprint is [752, 880) x [96, 160); its sprite starts 32 px lower with its strut.
    assert_eq!(
        props_of(&art, Prop::Stack),
        [PropDraw {
            prop: Prop::Stack,
            frame: 0,
            x: 752,
            y: floor + 64,
            layer: Layer::Prop
        }]
    );
    let mound = props_of(&art, Prop::Mound);
    assert_eq!(
        mound.iter().map(|p| (p.frame, p.layer)).collect::<Vec<_>>(),
        [(0, Layer::Back), (1, Layer::Front)],
        "sealed: the back layer behind everything, the lip in front"
    );
    assert!(
        mound.iter().all(|p| p.x + 88 == 608 && p.y == floor),
        "her home column"
    );
    let heaps = props_of(&art, Prop::Heap);
    assert_eq!(heaps.len(), 3);
    assert!(heaps.iter().all(|p| p.layer == Layer::Back && p.y == floor));
    assert!(art.props.len() <= MAX_PROPS);
    // Dormant: no sprite, no dust.
    assert!(art.hoarder.is_none());
    assert!(art.motes.is_empty());
}

/// R7: she is drawn one ramp step darker with dust sliding off her and no hit flash while she has no
/// hurtbox (the Reveal), inside the sealed mound (the front lip is drawn); on the first tick she can be
/// hit the mound is the low `opened` spill (no lip), she is at full value, and a 4-tick dust burst runs.
#[test]
fn the_reveal_is_shadowed_and_lip_covered_then_she_is_out() {
    let mut rig = Rig::new(&root(), clear_plan()).unwrap();
    let mut pilot = ClearPilot::new(0);
    let mut memo = HoarderMemo::default();
    let (mut reveal_ticks, mut out_ticks, mut burst_ticks) = (0, 0, 0);
    let mut seen_reveal = false;
    for _ in 0..600 {
        let input = pilot.input(&rig.combat, &rig.scene);
        rig.step(carl_input(input));
        let tick = rig.scene.world.tick;
        let boss = rig.boss().boss();
        let hidden = matches!(boss.mode(), PileMode::Dormant | PileMode::Reveal);
        let mut look = memo.observe(boss.hp(), boss.alive(), tick);
        look.flash = true; // a flash must never show on her while she is hidden
        look.out_age = memo.observe_out(hidden, tick);
        let art = art_of(&rig, tick, look);
        let mound = props_of(&art, Prop::Mound);
        if hidden {
            assert!(
                mound
                    .iter()
                    .any(|p| p.frame == MOUND_FRONT && p.layer == Layer::Front)
            );
            assert!(
                mound
                    .iter()
                    .any(|p| p.frame == MOUND_BACK && p.layer == Layer::Back)
            );
        } else {
            assert_eq!(
                mound.iter().map(|p| (p.frame, p.layer)).collect::<Vec<_>>(),
                [(MOUND_OPENED, Layer::Back)]
            );
        }
        match (boss.mode(), art.hoarder) {
            (PileMode::Dormant, h) => assert!(h.is_none()),
            (PileMode::Reveal, Some(h)) => {
                seen_reveal = true;
                reveal_ticks += 1;
                assert_eq!(h.shade, Shade::Shadow, "tick {tick}");
                assert!(!h.flash, "no hit flash on her during the Reveal");
                assert!((26..=28).contains(&h.frame));
                assert!(!art.motes.is_empty(), "dust slides off her");
            }
            (_, Some(h)) if seen_reveal && boss.alive() && boss.mode() != PileMode::Defeated => {
                out_ticks += 1;
                assert_eq!(h.shade, Shade::Full, "{:?} at tick {tick}", boss.mode());
                if look.out_age.is_some_and(|a| a < OUT_BURST_TICKS) {
                    burst_ticks += 1;
                    assert!(!art.motes.is_empty());
                } else {
                    assert!(art.motes.is_empty());
                }
            }
            _ => {}
        }
        if seen_reveal && out_ticks > 40 {
            break;
        }
    }
    assert_eq!(reveal_ticks, 90, "the pilot's first Reveal is 90 ticks");
    assert!(out_ticks > 40);
    assert_eq!(burst_ticks, OUT_BURST_TICKS, "a 4-tick burst");
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
/// this module called on every tick ends on the same state hash as the library fixture's run that
/// never calls it, and the hash after each tick is the same in both: the hashes are pinned
/// (`hoarder_fixtures::tests::ROOM_GOLDENS`) and never move.
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
            let _ = art_of(&with_art, tick, look);
            look.flash = tick % 7 == 0;
            look.defeated_age = (!with_art.boss().boss().alive()).then_some(tick % 40);
            look.out_age = Some(tick % 9);
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

/// The states HRD-B and HRD-C wire later have their frame tables ready: bank A stable 0, disturbed 1/2
/// alternating, slid 3/4, settled 5; the post 0/1/2; the stack 0..3, and its landed remnant is drawn in
/// front of the pinned boss. Every frame is inside its atlas, and the sizes match the manifests.
#[test]
fn the_later_prop_states_map_to_frames_inside_their_atlases() {
    let rect = crate::camera_lab::room::IRect {
        x: 352,
        y: 532,
        w: 64,
        h: 48,
    };
    for (state, ticks, frames) in [
        (BankState::Stable, [0, 1], [0, 0]),
        (BankState::Disturbed, [0, 1], [1, 2]),
        (BankState::Slid, [0, 1], [3, 4]),
        (BankState::Settled, [0, 1], [5, 5]),
    ] {
        for (tick, frame) in ticks.into_iter().zip(frames) {
            assert_eq!(bank_prop(rect, state, 580, tick).frame, frame, "{state:?}");
            assert!(u32::from(frame) < Prop::Bank.frames());
        }
    }
    let post = crate::camera_lab::room::IRect {
        x: 808,
        y: 516,
        w: 16,
        h: 64,
    };
    for (state, frame) in [
        (PostState::Intact, 0),
        (PostState::Cracked, 1),
        (PostState::Broken, 2),
    ] {
        assert_eq!(post_prop(post, state, 580).frame, frame);
    }
    // The sheet's hanging stack `[752, 880) x [96, 160)` and its landed remnant `[752, 880) x [0, 16)`
    // (layout y down, floor 580): the hanging sprite carries the strut (E1), so its bottom is 32 px
    // under the body; the remnant's rows are the frame's bottom rows.
    let hanging = hanging_over(post);
    assert_eq!(
        (hanging.x, hanging.y, hanging.w, hanging.h),
        (752, 420, 128, 64)
    );
    let remnant = crate::camera_lab::room::IRect {
        x: 752,
        y: 564,
        w: 128,
        h: 16,
    };
    for (state, body, frame, layer, y) in [
        (StackState::Supported, hanging, 0, Layer::Prop, 64),
        (StackState::Released, hanging, 1, Layer::Prop, 64),
        (StackState::Falling, hanging, 2, Layer::Prop, 64),
        (StackState::Landed, remnant, 3, Layer::Front, 0),
    ] {
        let d = stack_prop(body, state, 580);
        assert_eq!(
            (d.frame, d.layer, d.x, d.y),
            (frame, layer, 752, y),
            "{state:?}"
        );
        assert!(u32::from(d.frame) < Prop::Stack.frames());
    }
}

/// HRD-B and HRD-C wired (the PR that lands second, spec section 11): bank A and the J-B pieces are
/// drawn from their machines. The bank stands on its solid, tumbles on the sim's moving rectangle and
/// settles on the sim's remnant; the post cracks, then breaks on the release; the stack warns and falls
/// on the sim's mass rectangle with the footprint's floor shadow, then lands in front of the actors.
#[test]
fn the_piles_are_drawn_from_their_machines() {
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
    let bank = props_of(&art, Prop::Bank);
    assert_eq!(
        (bank.len(), bank[0].frame, bank[0].x),
        (1, 0, 352),
        "standing"
    );
    let (mut disturbed, mut slid, mut settled) = (false, false, false);
    for _ in 0..220 {
        rig.step(IDLE_SLOTS);
        let t = rig.next_tick() - 1;
        let art = art_of(&rig, t, HoarderLook::default());
        let bank = props_of(&art, Prop::Bank);
        assert_eq!(bank.len(), 1);
        let slide = rig.combat.env().unwrap().slide(0).unwrap();
        match slide.state() {
            crate::env_prop::SlideState::Disturbed => {
                assert!(matches!(bank[0].frame, 1 | 2));
                assert_eq!(bank[0].x, 352, "it shudders in place");
                disturbed = true;
            }
            crate::env_prop::SlideState::Slid => {
                assert!(matches!(bank[0].frame, 3 | 4));
                assert_eq!(bank[0].x, slide.mass_rect().unwrap().x, "on the sim's mass");
                assert!(bank[0].x <= 352, "moving toward -x: {}", bank[0].x);
                slid = true;
            }
            crate::env_prop::SlideState::Settled => {
                assert_eq!((bank[0].frame, bank[0].x), (5, 296), "the remnant");
                settled = true;
            }
            crate::env_prop::SlideState::Stable => {}
        }
    }
    assert!(disturbed && slid && settled);
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
    let floor = rig.boss().boss().setup().floor_y as i32;
    let (mut shadow, mut fell, mut landed) = (false, false, false);
    for _ in 0..260 {
        rig.step(IDLE_SLOTS);
        let t = rig.next_tick() - 1;
        let art = art_of(&rig, t, HoarderLook::default());
        let post = props_of(&art, Prop::Post);
        let stack = props_of(&art, Prop::Stack);
        assert_eq!((post.len(), stack.len()), (1, 1));
        let state = rig.combat.env().unwrap().mass_state(1).unwrap();
        match state {
            crate::env_prop::MassState::Supported => {
                assert_eq!(
                    (post[0].frame, stack[0].frame, stack[0].y),
                    (0, 0, floor + 64)
                );
                assert!(art.shadow.is_none());
            }
            crate::env_prop::MassState::Weakened => {
                assert_eq!(
                    (post[0].frame, stack[0].frame),
                    (1, 0),
                    "cracked, still hanging"
                );
            }
            crate::env_prop::MassState::Released | crate::env_prop::MassState::Falling => {
                assert_eq!(post[0].frame, 2, "broken from the release on");
                let s = art.shadow.expect("the footprint's floor shadow");
                assert_eq!((s.x, s.y, s.w, s.h), (752, floor, 128, FLOOR_SHADOW_PX));
                assert!(stack[0].y <= floor + 64, "never above the hanging stack");
                assert_eq!(stack[0].layer, Layer::Prop);
                shadow = true;
                fell |= state == crate::env_prop::MassState::Falling && stack[0].frame == 2;
            }
            crate::env_prop::MassState::Landed => {
                assert_eq!(
                    (stack[0].frame, stack[0].x, stack[0].y, stack[0].layer),
                    (3, 752, floor, Layer::Front)
                );
                assert!(art.shadow.is_none());
                landed = true;
            }
        }
        assert!(art.props.len() <= MAX_PROPS);
    }
    // Her shove releases the stack outright (no `Weakened` tick in this bait): `cracked` is checked
    // whenever that state shows up, the release, fall and landing always.
    assert!(shadow && fell && landed, "{shadow} {fell} {landed}");
}

/// The room kit's manifests agree with the sizes and frame counts the presenter slices the atlases by,
/// and every exported atlas is one row of frames.
#[test]
fn the_prop_sizes_match_the_manifests() {
    let manifests = [
        (Prop::Mound, "floor01_hoarder_mound"),
        (Prop::Bank, "floor01_hoarder_bank_a"),
        (Prop::Post, "floor01_hoarder_post_b"),
        (Prop::Stack, "floor01_hoarder_stack_b"),
        (Prop::Door, "floor01_hoarder_door"),
        (Prop::Gate, "floor01_hoarder_gate"),
        (Prop::Heap, "floor01_hoarder_heap"),
    ];
    for (prop, name) in manifests {
        assert_eq!(prop.atlas(), format!("generated/art/{name}.png"));
        let text = std::fs::read_to_string(root().join(format!("assets/manifests/{name}.json")))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let size = &json["frame_size"];
        assert_eq!(
            (size[0].as_u64().unwrap(), size[1].as_u64().unwrap()),
            (
                u64::from(prop.frame_size().0),
                u64::from(prop.frame_size().1)
            ),
            "{name}"
        );
        assert_eq!(
            json["frame_names"].as_array().unwrap().len() as u32,
            prop.frames(),
            "{name}"
        );
    }
    let text = std::fs::read_to_string(root().join("assets/manifests/hoard-02.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        json["frame_size"][0].as_u64().unwrap(),
        u64::from(FRAME_SIZE.0)
    );
    assert_eq!(
        json["hoarder"]["frames"].as_array().unwrap().len() as u32,
        FRAMES
    );
    assert_eq!(json["output_atlas"].as_str().unwrap(), ATLAS);
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

/// R7: the out age counts from the first tick she is no longer hidden, restarts at 0 when she is hidden
/// again, and is forgotten by a tick regression (a retry).
#[test]
fn the_out_age_counts_from_the_first_tick_she_can_be_hit() {
    let mut memo = HoarderMemo::default();
    memo.observe(30, true, 10);
    assert_eq!(memo.observe_out(true, 10), None);
    memo.observe(30, true, 100);
    assert_eq!(memo.observe_out(false, 100), Some(0));
    memo.observe(30, true, 103);
    assert_eq!(memo.observe_out(false, 103), Some(3));
    // Hidden again (a restart without the tick going back): the age restarts.
    memo.observe(30, true, 104);
    assert_eq!(memo.observe_out(true, 104), None);
    memo.observe(30, true, 105);
    assert_eq!(memo.observe_out(false, 105), Some(0));
    // A retry resets the scene tick: the memo forgets.
    memo.observe(30, true, 2);
    assert_eq!(memo.observe_out(true, 2), None);
}
