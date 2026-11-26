use super::*;
use crate::dozer::DozerState;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sim(case: Case) -> ChaseSim {
    run(&root(), case).unwrap_or_else(|e| panic!("{}: {e}", case.name()))
}

/// The ordered subsequence check: every item of `want` appears in `log` in this order.
fn in_order(log: &[(u64, DozerEvent)], want: &[&dyn Fn(&DozerEvent) -> bool]) -> bool {
    let mut at = 0;
    for pick in want {
        match log[at..].iter().position(|(_, e)| pick(e)) {
            Some(i) => at += i + 1,
            None => return false,
        }
    }
    true
}

fn is(name: &'static str) -> impl Fn(&DozerEvent) -> bool {
    move |e| e.name() == name
}

fn timber(id: u8) -> impl Fn(&DozerEvent) -> bool {
    move |e| matches!(e, DozerEvent::TimberSmashed { id: i } if *i == id)
}

/// N3: a run's bot rescues are few (F15: at most two; the sweep's stalls with the default
/// companion reach three), each to the pocket or behind the hull.
fn assert_rescues_bounded(s: &ChaseSim, what: &str, limit: usize) {
    assert!(s.rescues() <= limit, "{what}: {} rescues", s.rescues());
    for (t, e) in &s.log {
        if let DozerEvent::AiRescue { spot, cause, .. } = e {
            assert!(
                matches!(spot, RescueSpot::Pocket | RescueSpot::Behind),
                "{what}: spot {spot:?}"
            );
            // The `Charge` cause exists only on the tell's first tick, and puts the bot in the pocket.
            if *cause == crate::dozer::RescueCause::Charge {
                assert_eq!(*spot, RescueSpot::Pocket, "{what}");
                assert!(
                    s.log
                        .iter()
                        .any(|(tt, ev)| tt == t && *ev == DozerEvent::ChargeTell),
                    "{what}: a Charge rescue off the tell tick {t}"
                );
            }
        }
    }
}

/// Every case runs twice to an identical final hash, pinned here (ACT1-B: new constants).
#[test]
fn every_case_is_deterministic_and_pinned() {
    let cases = all_cases();
    assert_eq!(PINNED.len(), cases.len());
    for case in cases {
        let (a, b) = (sim(case), sim(case));
        assert_eq!(a.hash(), b.hash(), "{}", case.name());
        assert_eq!(a.tick(), b.tick(), "{}", case.name());
        assert_eq!(a.log, b.log, "{}", case.name());
        assert_eq!(a.sim.combat.dropped_events, 0, "{}", case.name());
        let pinned = PINNED
            .iter()
            .find(|(name, _)| *name == case.name())
            .unwrap_or_else(|| panic!("{} is not pinned", case.name()));
        assert_eq!(a.hash(), pinned.1, "{}: {:#018x}", case.name(), a.hash());
    }
}

/// The two ordered subsequences of F10/F10b and the counts (integration plan section 10).
fn assert_first_run(s: &ChaseSim, name: &str) {
    assert!(s.complete(), "{name}: the exit is reached");
    // (a) the emergence and the timbers.
    assert!(
        in_order(
            &s.log,
            &[&is("rumble"), &is("emerged"), &timber(0), &timber(1)]
        ),
        "{name}: {:?}",
        s.log
    );
    // (b) the charge, the barricade and the wedge.
    assert!(
        in_order(
            &s.log,
            &[
                &is("charge_armed"),
                &is("charge_tell"),
                &is("charge_committed"),
                &is("barrier_impact"),
                &is("charge_ended"),
                &is("brake"),
                &is("wedged"),
            ]
        ),
        "{name}: {:?}",
        s.log
    );
    // Both heroes enter the pocket (either may come before the tell).
    assert_eq!(
        s.count(|e| matches!(e, DozerEvent::PocketEntered { .. })),
        2,
        "{name}"
    );
    let committed = s.first(|e| *e == DozerEvent::ChargeCommitted).unwrap();
    let impact = s.first(|e| *e == DozerEvent::BarrierImpact).unwrap();
    for hero in HeroId::ALL {
        let t = s
            .first(|e| matches!(e, DozerEvent::PocketEntered { hero: h } if *h == hero))
            .unwrap_or_else(|| panic!("{name}: {hero:?} never entered the pocket"));
        // Each hero is in before the charge starts and before the impact.
        assert!(
            t <= committed && t < impact,
            "{name}: {hero:?} entered at {t}"
        );
    }
    assert_eq!(
        s.count(|e| matches!(e, DozerEvent::BrakeStarted { .. })),
        1,
        "{name}: one brake"
    );
    assert_eq!(
        s.count(|e| *e == DozerEvent::ChargeTell),
        1,
        "{name}: one charge"
    );
    // The commit front lies in the rule-4 window 1841..1932.
    let commit = s.dozer().commit.expect("a commit");
    assert!(
        (1841.0..=1932.0).contains(&commit.front),
        "{name}: {commit:?}"
    );
    // Zero wheel contacts, no hero defeat, no respawn, the hint never shown on a fresh run.
    assert_eq!(s.wheel_hits(), 0, "{name}");
    assert_eq!(s.sim.combat.respawns, 0, "{name}");
    assert!(!s.hint_seen, "{name}: the hint never shows on a fresh run");
    assert_eq!(
        s.count(|e| matches!(e, DozerEvent::PinnedHold { .. })),
        0,
        "{name}"
    );
    // The barricade opened on the impact tick and stays open (no flag: the machine reports it).
    assert!(s.dozer().barrier_hit() && s.dozer().wedged());
    assert!(!s.barricade_stands(), "{name}: the barricade is down");
}

/// ACT1-F10: Carl scripted into the pocket, Donut the follow bot (companion off): the events
/// in order, the commit front in the window, zero wheel contacts, and no `AiRescue` for a bot
/// that reached the pocket before the tell.
#[test]
fn act1_chase_solo_first_run() {
    let s = sim(Case::SoloFirstRun);
    assert_first_run(&s, "F10");
    let tell = s.first(|e| *e == DozerEvent::ChargeTell).unwrap();
    let bot_in = s
        .first(|e| {
            matches!(
                e,
                DozerEvent::PocketEntered {
                    hero: HeroId::Donut
                }
            )
        })
        .unwrap();
    assert!(bot_in <= tell, "the bot reached the pocket before the tell");
    assert_eq!(
        s.rescues(),
        0,
        "no AiRescue for a bot that reached the pocket first"
    );
}

/// ACT1-F10b: the Donut-active twin.
#[test]
fn act1_chase_solo_first_run_donut() {
    let s = sim(Case::SoloFirstRunDonut);
    assert_first_run(&s, "F10b");
    let tell = s.first(|e| *e == DozerEvent::ChargeTell).unwrap();
    let bot_in = s
        .first(|e| matches!(e, DozerEvent::PocketEntered { hero: HeroId::Carl }))
        .unwrap();
    assert!(bot_in <= tell);
    assert_eq!(s.rescues(), 0);
}

/// ACT1-F11: Carl's jabs on the passing hull are `Clang`s only: the machine's hp is constant, it
/// is never stunned or disabled, and the run is F10's.
#[test]
fn act1_chase_hull_cannot_be_hurt() {
    let s = sim(Case::HullCannotBeHurt);
    assert_first_run(&s, "F11");
    let clangs = s.count(|e| matches!(e, DozerEvent::Clang { .. }));
    assert!(clangs >= 1, "the jabs reach the hull");
    assert_eq!(
        s.count(|e| matches!(e, DozerEvent::Stunned { .. } | DozerEvent::Disabled)),
        0
    );
    let enemy = &s.sim.combat.slot_enemies()[0];
    assert_eq!(enemy.hp(), crate::dozer::DOZER_HP);
    assert!(enemy.alive());
    assert!(
        s.sim
            .combat
            .events
            .iter()
            .all(|e| !matches!(e, CombatEvent::EnemyDefeated { .. })),
        "the machine is never killed"
    );
}

/// ACT1-F12: both heroes in the pocket together: zero wheel contacts.
#[test]
fn act1_chase_2p_together() {
    let s = sim(Case::TwoPlayerTogether);
    assert!(s.complete());
    assert_eq!(s.wheel_hits(), 0);
    assert_eq!(
        s.count(|e| matches!(e, DozerEvent::PocketEntered { .. })),
        2
    );
    assert!(s.dozer().barrier_hit() && s.dozer().wedged());
}

/// ACT1-F13: P2 0, 96, 224 and 448 px behind the leader at the arm: zero wheel contacts, the
/// commit front in the window in every case and the impact inside the charge. The commit table
/// is printed by `print_the_chase_tables` (evidence).
#[test]
fn act1_chase_2p_lag() {
    for lag in [0u16, 96, 224, 448] {
        let s = sim(Case::TwoPlayerLag(lag));
        let name = format!("lag {lag}");
        assert!(s.complete(), "{name}");
        assert_eq!(s.wheel_hits(), 0, "{name}");
        let commit = s
            .dozer()
            .commit
            .unwrap_or_else(|| panic!("{name}: no commit"));
        assert!(
            (1841.0..=1932.0).contains(&commit.front),
            "{name}: {commit:?}"
        );
        let committed = s.first(|e| *e == DozerEvent::ChargeCommitted).unwrap();
        let impact = s.first(|e| *e == DozerEvent::BarrierImpact).unwrap();
        assert!(
            impact - committed <= 90,
            "{name}: the impact lies inside the charge"
        );
        assert!(
            commit.margin.iter().flatten().all(|m| *m >= 20),
            "{name}: {commit:?}"
        );
        // The separation at the arm follows the lag (the 448 case is held at the leash).
        let sep = s.arm_separation.expect("armed");
        let want = f32::from(lag.min(440));
        assert!((sep - want).abs() <= 8.0, "{name}: separation {sep}");
    }
}

/// ACT1-F14: Carl waits at the closed barricade: no charge (rule 3 fails), a wheel defeat
/// before the front reaches it, the respawn at the checkpoint rebuilds the machine (dormant, the
/// barricade standing again, `barrier_hit` false), the hint shows, and a clean run exits.
#[test]
fn act1_chase_skip_pocket() {
    let s = sim(Case::SkipPocket);
    assert!(s.complete());
    assert_eq!(s.sim.combat.respawns, 1, "one failure, one respawn");
    // The first attempt: no tell (rule 3 fails for Carl at the barricade).
    let respawn_tick = s
        .sim
        .combat
        .events
        .iter()
        .find_map(|e| match *e {
            CombatEvent::Respawn { tick, .. } => Some(tick),
            _ => None,
        })
        .expect("a respawn");
    let before: Vec<_> = s.log.iter().filter(|(t, _)| *t < respawn_tick).collect();
    assert!(
        !before.iter().any(|(_, e)| *e == DozerEvent::ChargeTell),
        "no charge: {before:?}"
    );
    let carl_hit = before
        .iter()
        .find(|(_, e)| matches!(e, DozerEvent::WheelHit { hero: HeroId::Carl }))
        .expect("the wheel defeats Carl at the barricade")
        .0;
    // The wheel front leads the push box by 88 px: it reaches the hero at 2300 first.
    assert!(
        !before
            .iter()
            .any(|(_, e)| matches!(e, DozerEvent::PinnedHold { .. }))
    );
    // The machine keeps running through the defeat delay (ACT1-IMP-11): the impact follows the hit
    // and precedes the respawn, which then rebuilds the machine.
    let impact = before
        .iter()
        .find(|(_, e)| *e == DozerEvent::BarrierImpact)
        .expect("IMP-11: the machine runs on through the defeat delay")
        .0;
    assert!(
        carl_hit < impact && impact < respawn_tick,
        "{carl_hit} < {impact} < {respawn_tick}"
    );
    // The 21 `WheelHit` events are two episodes (the bot and Carl), logged every tick while a
    // downed hero lies under the wheel.
    assert_eq!(s.wheel_hit_episodes(), 2);
    let view = s.respawn_view.expect("the respawn view");
    assert_eq!(view.machine, "dormant");
    assert!(!view.barrier_hit, "barrier_hit is false after the respawn");
    assert!(view.barricade_stands, "the barricade stands again");
    assert!(view.hint, "the alcove hint shows after the failure");
    // The second attempt (the clean run) charges, wedges and exits.
    let after: Vec<_> = s.log.iter().filter(|(t, _)| *t >= respawn_tick).collect();
    for name in [
        "rumble",
        "emerged",
        "charge_tell",
        "barrier_impact",
        "wedged",
    ] {
        assert!(
            after.iter().any(|(_, e)| e.name() == name),
            "{name} after the respawn"
        );
    }
    // The hint fades: no longer live at the end of the clean run.
    assert!(!f01_act1::alcove_hint_live(&s.sim.combat, s.tick()));
}

/// A fixture restart (R / Select) rebuilds the machine and closes the barricade too.
#[test]
fn a_restart_rebuilds_the_machine_and_closes_the_barricade() {
    let mut s = first_run_until_barrier(&root());
    assert!(s.dozer().barrier_hit() && !s.barricade_stands());
    s.sim.restart();
    s.step([RawInput::NONE, RawInput::NONE]);
    s.step([RawInput::NONE, RawInput::NONE]);
    assert_eq!(s.state().name(), "dormant");
    assert!(!s.dozer().barrier_hit() && s.barricade_stands());
    // A checkpoint respawn after an impact does the same.
    let mut s = first_run_until_barrier(&root());
    s.respawn();
    s.step([RawInput::NONE, RawInput::NONE]);
    assert_eq!(s.state().name(), "dormant");
    assert!(!s.dozer().barrier_hit() && s.barricade_stands());
}

fn first_run_until_barrier(root: &Path) -> ChaseSim {
    let mut sim = ChaseSim::new(root, Party::Solo { uncontrolled: BOT }).unwrap();
    while sim.tick() < MAX_TICKS && !sim.dozer().barrier_hit() {
        let grounded = sim.sim.scene.world.body(HeroId::Carl).grounded;
        let input = Runner { target: POCKET_X }.input(sim.x_of(HeroId::Carl), grounded);
        sim.step([input, RawInput::NONE]);
    }
    // One more tick: heroes collide with the open state from the next tick.
    sim.step([RawInput::NONE, RawInput::NONE]);
    sim
}

/// The barricade's one-tick lag: `barrier_hit` is set in the slot step of tick N and the solid is
/// already down in the env step of the same tick (heroes collide with the open state from N + 1).
#[test]
fn the_barricade_opens_in_the_impact_tick() {
    let mut sim = ChaseSim::new(&root(), Party::Solo { uncontrolled: BOT }).unwrap();
    let mut stood_before = false;
    while sim.tick() < MAX_TICKS {
        let grounded = sim.sim.scene.world.body(HeroId::Carl).grounded;
        let input = Runner { target: POCKET_X }.input(sim.x_of(HeroId::Carl), grounded);
        let was = sim.barricade_stands();
        sim.step([input, RawInput::NONE]);
        if sim.dozer().barrier_hit() {
            stood_before = was;
            assert!(
                !sim.barricade_stands(),
                "the solid is down in the impact tick"
            );
            break;
        }
        assert!(sim.barricade_stands(), "closed until the impact");
    }
    assert!(stood_before, "it stood on the tick before the impact");
}

/// ACT1-F15: the default companion with Carl or Donut active: zero wheel contacts on the bot.
/// The rescue counts by spot are evidence (`print_the_chase_tables`).
#[test]
fn act1_chase_solo_bot_carl_and_donut_active() {
    for case in [Case::BotCarlActive, Case::BotDonutActive] {
        let s = sim(case);
        assert!(s.complete(), "{}", case.name());
        assert_eq!(s.wheel_hits(), 0, "{}", case.name());
        assert_eq!(s.wheel_hit_episodes(), 0);
        assert_eq!(s.sim.combat.respawns, 0);
        assert_rescues_bounded(&s, case.name().as_str(), 2);
        assert_eq!(s.rescues_by_spot(RescueSpot::Lane), 0, "{}", case.name());
        assert_eq!(
            s.rescues_by_spot(RescueSpot::Tunnel),
            0,
            "never the tunnel spot before the impact"
        );
        assert_eq!(s.count(|e| matches!(e, DozerEvent::PinnedHold { .. })), 0);
    }
}

/// ACT1-F16: P1 waits in the pocket, P2 shadows the front (body left edge 100..130 px ahead),
/// so rule 2 never holds: no tell, the cruise passes the pocket, the barricade breaks, the machine
/// wedges, the heroes exit, zero wheel contacts.
#[test]
fn act1_chase_no_charge_path() {
    let s = sim(Case::NoChargePath);
    assert!(s.complete());
    assert_eq!(
        s.count(|e| *e == DozerEvent::ChargeTell),
        0,
        "rule 2 never holds"
    );
    assert_eq!(s.count(|e| *e == DozerEvent::ChargeCommitted), 0);
    assert_eq!(
        s.count(|e| matches!(e, DozerEvent::PocketEntered { .. })),
        2
    );
    assert_eq!(s.wheel_hits(), 0);
    assert!(in_order(
        &s.log,
        &[&is("barrier_impact"), &is("brake"), &is("wedged")]
    ));
    assert_eq!(s.dozer().charges_used(), 0);
    assert!(s.dozer().wedged() && !s.barricade_stands());
    // The shadow stayed in the 100..130 band from the moment it closed in until it entered the
    // pocket (the first sample is the emergence gap, before it has closed).
    let closed: Vec<f32> = s
        .donut_gap
        .iter()
        .map(|(_, g)| *g)
        .skip_while(|g| *g > 130.0)
        .collect();
    assert!(
        closed.len() > 300,
        "the shadow held for {} ticks",
        closed.len()
    );
    assert!(
        closed.iter().all(|g| (100.0..=130.0).contains(g)),
        "{closed:?}"
    );
}

/// ACT1-F17: the presentation is read-only: drawing the machine and the hint every tick leaves
/// the event log and the final hash of F10 unchanged.
#[test]
fn act1_chase_presentation_is_read_only() {
    let plain = sim(Case::SoloFirstRun);
    let mut shown = ChaseSim::new(&root(), Party::Solo { uncontrolled: BOT }).unwrap();
    let exit = shown.exit_x();
    let mut sprites = 0usize;
    while shown.tick() < MAX_TICKS && !shown.complete() {
        let target = if shown.dozer().wedged() {
            exit
        } else {
            POCKET_X
        };
        let grounded = shown.sim.scene.world.body(HeroId::Carl).grounded;
        let input = Runner { target }.input(shown.x_of(HeroId::Carl), grounded);
        shown.step([input, RawInput::NONE]);
        // The presenter's pure half and the hint, every tick.
        let w = &shown.sim.scene.world;
        if let Some(art) =
            crate::dozer_art::draw(&shown.sim.combat, [w.hero.x, w.donut_body.x], w.tick)
        {
            sprites += art.sprites.len();
        }
        let _ = f01_act1::alcove_hint_live(&shown.sim.combat, shown.tick());
    }
    assert!(sprites > 0, "the machine was drawn");
    assert_eq!(shown.log, plain.log);
    assert_eq!(shown.hash(), plain.hash());
}

/// The wedge is clipped to the arch: no drawn rig sprite reaches past `stop_x`, and room 3's
/// pieces (the rear gate, the buffer stop) are not drawn in the chase.
#[test]
fn the_opening_draws_no_room_3_pieces_and_the_wedge_is_clipped() {
    use crate::dozer_art::{Sheet, draw};
    let s = sim(Case::SoloFirstRun);
    let w = &s.sim.scene.world;
    let art = draw(&s.sim.combat, [w.hero.x, w.donut_body.x], w.tick).expect("drawn");
    assert!(art.wedged);
    let arch_right = s.dozer().site.stop_x;
    for sprite in &art.sprites {
        assert!(
            !matches!(sprite.sheet, Sheet::Gate | Sheet::Buffer),
            "{sprite:?}"
        );
        if sprite.z != crate::dozer_art::FX_Z {
            let (fw, _) = sprite.sheet.frame_size();
            let right = sprite.left + (fw - u32::from(sprite.crop_right)) as i32;
            assert!(
                right <= arch_right,
                "{sprite:?} reaches past the arch ({arch_right})"
            );
        }
    }
}

/// ACT1-F18: room 3 is unchanged by the opening: its tuning, room and sidecar hashes are the
/// committed ones and its site is a plain `revenge` site (every DOZR fixture hash is pinned in
/// the room 3 suite, which passes unedited).
#[test]
fn room3_unchanged_by_the_opening() {
    assert_eq!(crate::dozer::TUNING_HASH, 0x146c_8ef7_91c8_e9e9);
    assert_eq!(crate::f01_dozer::ENCOUNTER_HASH, 0x3dd3_2eec_2779_bdda);
    assert_eq!(crate::f01_dozer::ROOM_HASH, 0xaf54_bc91_c52d_6f83);
    assert!(crate::dozer::DozerTuning::active().charge.is_none());
    // Room 3's own machine, built through the same slot path, mixes nothing of the opening: its
    // fixtures (`dozer_fixtures::tests`, every DOZR hash pinned there, committed before this PR)
    // pass unedited in the same suite.
    let run = crate::f01_dozer::launch(&root()).unwrap();
    let site = run.loaded.encounter.dozer.as_ref().unwrap();
    assert_eq!(site.profile, crate::dozer::SiteProfile::Revenge);
    assert!(site.pockets.is_empty() && site.timbers.is_empty() && !site.wedge);
    assert!(site.lintel.is_some() && site.barricade_x.is_none());
}

/// Evidence helper (ignored): the commit table (F13), the F10/F10b/F15 rescue counts by spot.
#[test]
#[ignore = "evidence helper: prints the chase tables"]
fn print_the_chase_tables() {
    println!("F13 commit table (commit tick, commit front, margin per hero in ticks, gap):");
    for lag in [0u16, 96, 224, 448] {
        let s = sim(Case::TwoPlayerLag(lag));
        let commit = s.dozer().commit.expect("commit");
        let tick = s.first(|e| *e == DozerEvent::ChargeTell).unwrap();
        println!(
            "  lag {lag:>3}: separation_at_arm={:.0} commit_tick={tick} commit_front={:.1} margin(carl,donut)={:?} gap={:.1} hits={}",
            s.arm_separation.unwrap(),
            commit.front,
            commit.margin,
            commit.gap,
            s.wheel_hits()
        );
    }
    println!("rescues by spot (pocket, behind, lane, tunnel) and wheel hits:");
    for case in all_cases() {
        let s = sim(case);
        println!(
            "  {:<36} ticks={:<5} rescues={} ({}, {}, {}, {}) hit_events={} hit_episodes={} commit_front={:?}",
            case.name(),
            s.tick(),
            s.rescues(),
            s.rescues_by_spot(RescueSpot::Pocket),
            s.rescues_by_spot(RescueSpot::Behind),
            s.rescues_by_spot(RescueSpot::Lane),
            s.rescues_by_spot(RescueSpot::Tunnel),
            s.wheel_hits(),
            s.wheel_hit_episodes(),
            s.dozer().commit.map(|c| c.front)
        );
    }
    for case in all_cases() {
        let s = sim(case);
        println!("    (\"{}\", {:#018x}),", case.name(), s.hash());
    }
    for case in all_cases() {
        let s = sim(case);
        let at = |pick: &dyn Fn(&DozerEvent) -> bool| {
            s.first(pick).map_or("-".into(), |t| t.to_string())
        };
        println!(
            "ACT1_FIXTURE name={} room=act1_chase ticks={} exit={} respawns={} wheel_hit_events={} wheel_hit_episodes={} ai_rescues={} rescue_spots(pocket,behind,lane,tunnel)=({},{},{},{}) rumble={} emerged={} tell={} committed={} impact={} brake={} wedged={} commit_front={} hash={:#018x}",
            case.name(),
            s.tick(),
            s.complete(),
            s.sim.combat.respawns,
            s.wheel_hits(),
            s.wheel_hit_episodes(),
            s.rescues(),
            s.rescues_by_spot(RescueSpot::Pocket),
            s.rescues_by_spot(RescueSpot::Behind),
            s.rescues_by_spot(RescueSpot::Lane),
            s.rescues_by_spot(RescueSpot::Tunnel),
            at(&|e| matches!(e, DozerEvent::Rumble { .. })),
            at(&|e| matches!(e, DozerEvent::Emerged { .. })),
            at(&|e| *e == DozerEvent::ChargeTell),
            at(&|e| *e == DozerEvent::ChargeCommitted),
            at(&|e| *e == DozerEvent::BarrierImpact),
            at(&|e| matches!(e, DozerEvent::BrakeStarted { .. })),
            at(&|e| *e == DozerEvent::Wedged),
            s.dozer()
                .commit
                .map_or("none".into(), |c| format!("{:.1}", c.front)),
            s.hash()
        );
    }
    let _ = DozerState::Disabled;
}

/// The first run holds across start delays and stalls (a K5-style sweep of the scripted hero,
/// either hero active, the follow bot or the default companion as the other): 96 runs, none
/// touched by the wheel, none respawned, every one reaches the exit.
#[test]
fn the_first_run_holds_across_delays_and_stalls() {
    let mut runs = 0;
    let mut rescues = 0;
    for unc in [BOT, FREE] {
        for donut in [false, true] {
            for delay in [0u64, 23, 61, 130] {
                for stall in [
                    (0.0f32, 0u64),
                    (1500.0, 40),
                    (1700.0, 60),
                    (1900.0, 30),
                    (1950.0, 60),
                    (2040.0, 60),
                ] {
                    let s = varied_run(&root(), donut, unc, (delay, stall.0, stall.1)).unwrap();
                    let what = format!("{unc:?} donut={donut} delay={delay} stall={stall:?}");
                    assert!(s.complete(), "{what}");
                    assert_eq!(s.wheel_hits(), 0, "{what}");
                    assert_eq!(s.sim.combat.respawns, 0, "{what}");
                    assert_eq!(s.rescues_by_spot(RescueSpot::Tunnel), 0, "{what}");
                    assert_eq!(s.rescues_by_spot(RescueSpot::Lane), 0, "{what}");
                    assert_rescues_bounded(&s, &what, 3);
                    rescues += s.rescues();
                    runs += 1;
                }
            }
        }
    }
    assert_eq!(runs, 96);
    println!("sweep: {runs} runs, {rescues} bot rescues in all, zero wheel contacts");
}

/// ACT1-AD9: the opening machine is drawn without riders (the shore goblins stand in for them):
/// the presenter's rig is the rider-less rig, two parts short of the seated one.
#[test]
fn the_opening_machine_is_drawn_without_riders() {
    use crate::dozer_art::rig_sprites;
    let s = sim(Case::SoloFirstRun);
    let w = &s.sim.scene.world;
    let drawn = |seated: [bool; 2]| rig_sprites(s.dozer(), w.tick, false, seated);
    assert_eq!(drawn([true, true]).len() - drawn([false, false]).len(), 2);
    let art = crate::dozer_art::draw(&s.sim.combat, [w.hero.x, w.donut_body.x], w.tick).unwrap();
    for sprite in drawn([false, false]) {
        // (the wedge clips some sprites at the arch: compare the unclipped ones by sheet/frame)
        assert!(
            art.sprites.iter().any(|a| a.sheet == sprite.sheet
                && a.frame == sprite.frame
                && a.left == sprite.left),
            "{sprite:?}"
        );
    }
}

/// Evidence helper (ignored): the per-tick trace of F13 at lag 448 (tick, hero columns, the
/// machine's wheel front and state, the charge stage), for the schematic GIF.
#[test]
#[ignore = "evidence helper: prints the F13 lag 448 trace"]
fn print_the_f13_trace() {
    let lag = 448u16;
    let held = f32::from(lag).min(LEASH_HOLD_PX);
    let arm = 1888.0;
    let mut sim = ChaseSim::new(&root(), Party::TwoPlayer).unwrap();
    let exit = 2960.0;
    while sim.tick() < MAX_TICKS && !sim.complete() {
        let carl = sim.x_of(HeroId::Carl);
        let (tc, td) = if sim.dozer().wedged() {
            (exit, exit)
        } else {
            let donut = if carl < arm && !sim.dozer().armed_once() {
                (carl - held).max(48.0)
            } else {
                POCKET_X - 40.0
            };
            (POCKET_X, donut)
        };
        let inputs = [
            runner(&sim, HeroId::Carl, tc),
            runner(&sim, HeroId::Donut, td),
        ];
        sim.step(inputs);
        let d = sim.dozer();
        println!(
            "TRACE {} {:.1} {:.1} {:.1} {} {} {}",
            sim.tick(),
            sim.x_of(HeroId::Carl),
            sim.x_of(HeroId::Donut),
            d.wheel_front(),
            d.state.name(),
            d.charge.name(),
            u8::from(d.barrier_hit())
        );
    }
}
