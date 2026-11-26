use super::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sim(case: Case) -> Act1Sim {
    run_sim(&root(), case).unwrap_or_else(|e| panic!("{}: {e}", case.name()))
}

/// Every case runs twice to an identical final hash, pinned here (ACT1-A: new constants).
#[test]
fn every_case_is_deterministic_and_pinned() {
    assert_eq!(PINNED.len(), Case::ALL.len());
    for case in Case::ALL {
        let (a, b) = (sim(case), sim(case));
        assert_eq!(a.hash(), b.hash(), "{}", case.name());
        assert_eq!(a.tick(), b.tick(), "{}", case.name());
        let pinned = PINNED
            .iter()
            .find(|(name, _)| *name == case.name())
            .unwrap_or_else(|| panic!("{} is not pinned", case.name()));
        assert_eq!(a.hash(), pinned.1, "{}: {:#018x}", case.name(), a.hash());
        assert_eq!(a.run.room, case.room());
    }
}

/// ACT1-F01: companion off, Carl punches both goblins and walks out; Donut never has a
/// missile (no unlock, no cast).
#[test]
fn act1_shore_solo_carl() {
    let s = sim(Case::ShoreSoloCarl);
    assert!(s.complete(), "the exit is reached");
    assert!(s.roster_cleared());
    let defeats = s.defeats();
    assert_eq!(defeats.len(), 2);
    assert!(defeats.iter().all(|(_, c)| matches!(c, DefeatCause::Melee)));
    assert!(s.unlocks().is_empty() && s.casts() == 0 && !s.combat.donut_unlocked());
}

/// ACT1-F02: three jabs at the strike column while the first goblin stands in the footprint:
/// the block falls and the goblin's defeat is credited to the block (Carl, player 0).
#[test]
fn act1_shore_block_kill() {
    let s = sim(Case::ShoreBlockKill);
    let first = s.defeats()[0];
    assert_eq!(first.0, 0, "gob_a");
    assert!(
        matches!(first.1, DefeatCause::ShoreBlock { player: 0 }),
        "{first:?}"
    );
    assert!(
        s.combat
            .events
            .iter()
            .any(|e| matches!(e, CombatEvent::ShoreLanded { .. }))
    );
    assert_eq!(
        s.combat
            .events
            .iter()
            .filter(|e| matches!(e, CombatEvent::ShoreBroken { .. }))
            .count(),
        1,
        "the third jab breaks the beam once"
    );
    assert!(s.complete() && s.casts() == 0);
}

/// ACT1-F03: two players, and the default companion: the exit is reached and Donut never
/// casts (CF02-Q17: her AI does not cast before the biscuit).
#[test]
fn act1_shore_2p_and_the_default_companion() {
    for case in [Case::Shore2P, Case::ShoreSoloCompanion] {
        let s = sim(case);
        assert!(s.complete(), "{}", case.name());
        assert!(s.roster_cleared());
        assert!(s.unlocks().is_empty() && s.casts() == 0, "{}", case.name());
    }
}

/// ACT1-F04: a party defeat respawns at the checkpoint (local 56), the living goblins are
/// restored to their spawn columns and health (they had moved and one was hurt) and no machine
/// stands anywhere in the room.
#[test]
fn act1_shore_retry_from_post_chase() {
    // The fixture itself refuses to run without a hurt, moved goblin before the respawn.
    let s = sim(Case::ShoreRetryFromPostChase);
    assert!(!s.complete());
    assert_eq!(s.run.rules.checkpoints, [56]);
    assert!(s.combat.slot_enemies().is_empty(), "no machine anywhere");
    assert!(s.defeats().is_empty());
    // Step by step: just before and just after the respawn.
    let mut s = Act1Sim::new(
        &root(),
        Act1Room::Shore,
        CarlProgFlags::default(),
        DonutProgFlags::default(),
        Party::Solo { uncontrolled: BOT },
    )
    .unwrap();
    let ex = s.exit_x();
    while s.tick() < 400 && s.combat.enemies.iter().all(|g| g.health == 2) {
        let input = clear_and_exit(&s.combat, &s.scene, ex);
        s.step([input, RawInput::NONE]);
    }
    assert!(
        s.combat.enemies.iter().any(|g| g.health < 2),
        "one goblin is hurt"
    );
    assert!(
        s.combat
            .enemies
            .iter()
            .any(|g| (g.x - 528.0).abs() > 1.0 && (g.x - 568.0).abs() > 1.0),
        "a goblin has walked off its column"
    );
    s.respawn();
    let columns: Vec<f32> = s.combat.enemies.iter().map(|g| g.x).collect();
    assert_eq!(
        columns,
        [528.0, 568.0],
        "the goblins are back on their columns"
    );
    assert!(s.combat.enemies.iter().all(|g| g.alive() && g.health == 2));
    assert!(
        (s.x_of(HeroId::Carl) - 56.0).abs() < 1.0,
        "{}",
        s.x_of(HeroId::Carl)
    );
}

/// ACT1-F05: exactly one `UnlockGranted { hero: 1 }` when Donut's pivot enters the zone, the
/// door's opening logged in the same tick, the door standing before it and gone after it, the
/// exit reached and the panel fact `(1, primary_ranged)`.
#[test]
fn act1_guild_crossings_grant_once() {
    for case in [Case::GuildSoloCarl, Case::GuildSoloDonut, Case::Guild2P] {
        let s = sim(case);
        assert!(s.complete(), "{}", case.name());
        assert_eq!(s.unlocks().len(), 1, "{}", case.name());
        let (tick, hero) = s.unlocks()[0];
        assert_eq!(hero, 1);
        assert_eq!(s.door_events(), [tick], "{}: the same tick", case.name());
        assert_eq!(s.door_stands(), Some(false));
        assert_eq!(s.casts(), 0);
        let row = crate::f01_late::Manifest::load(&root())
            .unwrap()
            .room("act1_guild")
            .unwrap()
            .clone();
        let facts = crate::f01_run::RoomFacts::of(&row, &s.combat, None);
        assert_eq!(
            facts.grants,
            [(1, "primary_ranged".to_owned())],
            "{}",
            case.name()
        );
    }
}

/// ACT1-F05, the tick order: the door stands until the tick of the biscuit and is not a
/// collision from then on (one-tick lag for the heroes, who collided earlier in that tick).
#[test]
fn the_guild_door_stands_until_the_unlock_tick() {
    let (carl, donut) = story();
    let mut s = Act1Sim::new(
        &root(),
        Act1Room::Guild,
        carl,
        donut,
        Party::Solo { uncontrolled: BOT },
    )
    .unwrap();
    let door = s.door().expect("the door");
    assert_eq!((door.x, door.w), (528, 16));
    assert_eq!(s.door_stands(), Some(true));
    s.step([swap(), RawInput::NONE]);
    let ex = s.exit_x();
    let mut zone = None;
    for _ in 0..400 {
        let x = s.x_of(HeroId::Donut);
        s.step([walk_in(x, ex), RawInput::NONE]);
        if s.unlocks().is_empty() {
            assert_eq!(s.door_stands(), Some(true), "tick {}", s.tick());
            assert!(
                s.x_of(HeroId::Donut) < 376.0,
                "still short of the zone's end"
            );
        } else {
            if zone.is_none() {
                assert!(
                    (336.0..376.0).contains(&s.x_of(HeroId::Donut)),
                    "the unlock tick: Donut's pivot is in the zone, {}",
                    s.x_of(HeroId::Donut)
                );
            }
            zone.get_or_insert(s.tick());
            assert_eq!(s.door_stands(), Some(false));
            assert_eq!(s.door_events().len(), 1);
        }
        if s.complete() {
            break;
        }
    }
    assert!(zone.is_some() && s.complete());
}

/// ACT1-F06: the default companion with Carl active: one grant, the exit, never sealed in.
#[test]
fn act1_guild_companion_crossing() {
    let s = sim(Case::GuildCompanionCrossing);
    assert!(s.complete());
    assert_eq!(s.unlocks().len(), 1);
    assert_eq!(s.door_events().len(), 1);
}

/// ACT1-F07: a retry keeps the biscuit and the open door, a fixture restart closes the door,
/// the second crossing grants once more (the case itself asserts the intermediate states).
#[test]
fn act1_guild_retry_keeps_the_biscuit() {
    let s = sim(Case::GuildRetryKeepsTheBiscuit);
    assert!(s.complete());
    assert_eq!(
        s.unlocks().len(),
        2,
        "one grant per crossing, none on the retry"
    );
    assert_eq!(s.door_events().len(), 2);
    assert_eq!(s.combat.restarts, 1);
}

/// ACT1-F08: under the All loadout the biscuit is held at tick 0, the door is open and no
/// grant is emitted.
#[test]
fn act1_guild_all_loadout_is_open() {
    let (carl, donut) = Loadout::All.flags();
    let s = Act1Sim::new(
        &root(),
        Act1Room::Guild,
        carl,
        donut,
        Party::Solo { uncontrolled: BOT },
    )
    .unwrap();
    assert!(s.combat.donut_unlocked());
    assert_eq!(s.door_stands(), Some(false));
    let s = sim(Case::GuildAllLoadoutIsOpen);
    assert!(s.complete());
    assert!(s.unlocks().is_empty() && s.door_events().is_empty());
    let row = crate::f01_late::Manifest::load(&root())
        .unwrap()
        .room("act1_guild")
        .unwrap()
        .clone();
    assert!(
        crate::f01_run::RoomFacts::of(&row, &s.combat, None)
            .grants
            .is_empty()
    );
}

/// The closed door is a real wall for Carl: with P2 (Donut) idle he walks into it and stops at
/// x 516 (the door's left edge 528 minus his half width), for as long as her mana is locked and
/// the room never completes. When she walks the zone, Carl is still held at the door at the end
/// of the unlock tick (he collided earlier in that tick) and is through it by two ticks later.
#[test]
fn the_closed_door_blocks_carl_until_the_tick_after_the_unlock() {
    let (carl, donut) = story();
    let mut s = Act1Sim::new(&root(), Act1Room::Guild, carl, donut, Party::TwoPlayer).unwrap();
    let ex = s.exit_x();
    let limit = 528.0 - crate::prototype::HERO_HALF_WIDTH;
    let (mut max_x, mut at_the_door) = (0.0f32, 0u32);
    for _ in 0..200 {
        let x = s.x_of(HeroId::Carl);
        s.step([walk_in(x, ex), RawInput::NONE]);
        max_x = max_x.max(s.x_of(HeroId::Carl));
        if s.x_of(HeroId::Carl) >= limit - 1.0 {
            at_the_door += 1;
        }
    }
    assert!(
        s.unlocks().is_empty() && !s.complete(),
        "Donut idles: the door stays shut"
    );
    assert!(max_x <= limit + 0.5, "the door blocks Carl: max x {max_x}");
    assert!(
        at_the_door >= 50,
        "Carl stood at the door for {at_the_door} ticks"
    );
    // Donut walks the zone; Carl keeps pressing right.
    while s.unlocks().is_empty() && s.tick() < 600 {
        let (cx, dx) = (s.x_of(HeroId::Carl), s.x_of(HeroId::Donut));
        s.step([walk_in(cx, ex), walk_in(dx, ex)]);
    }
    assert_eq!(s.unlocks().len(), 1);
    let unlock_tick = s.unlocks()[0].0;
    assert_eq!(s.tick(), unlock_tick);
    assert!(
        s.x_of(HeroId::Carl) <= limit + 0.5,
        "still held at the end of the unlock tick: {}",
        s.x_of(HeroId::Carl)
    );
    assert_eq!(
        s.door_stands(),
        Some(false),
        "the door solid is already gone"
    );
    for _ in 0..2 {
        let (cx, dx) = (s.x_of(HeroId::Carl), s.x_of(HeroId::Donut));
        s.step([walk_in(cx, ex), walk_in(dx, ex)]);
    }
    assert!(
        s.x_of(HeroId::Carl) > limit + 0.5,
        "through the door two ticks after the unlock: {}",
        s.x_of(HeroId::Carl)
    );
    while !s.complete() && s.tick() < 1200 {
        let (cx, dx) = (s.x_of(HeroId::Carl), s.x_of(HeroId::Donut));
        s.step([walk_in(cx, ex), walk_in(dx, ex)]);
    }
    assert!(s.complete());
}

/// Solo with the follow bot: Carl alone at the closed door is never a softlock, because the
/// trailing bot crosses the zone before he reaches the door.
#[test]
fn the_trailing_bot_walks_the_zone_so_the_room_never_softlocks() {
    let (carl, donut) = story();
    let mut s = Act1Sim::new(
        &root(),
        Act1Room::Guild,
        carl,
        donut,
        Party::Solo { uncontrolled: BOT },
    )
    .unwrap();
    let ex = s.exit_x();
    while !s.complete() && s.tick() < 600 {
        let x = s.x_of(HeroId::Carl);
        s.step([walk_in(x, ex), RawInput::NONE]);
    }
    assert!(s.complete() && s.unlocks().len() == 1);
}

/// ACT1-F05: Donut's prompt reads `SWAT` before the biscuit and `MISSILE` from the unlock tick on
/// (the pre-existing `PreBiscuit` switch, now played in the guild).
#[test]
fn donuts_prompt_switches_from_swat_to_missile_at_the_biscuit() {
    use crate::donut_hud::{donut_ability_row, magic_of};
    use crate::donut_mana::EmptyPress;
    use crate::hud::{AbilityRow, Control, LabelId, SpecialSlot};
    let (carl, donut) = story();
    let mut s = Act1Sim::new(
        &root(),
        Act1Room::Guild,
        carl,
        donut,
        Party::Solo { uncontrolled: BOT },
    )
    .unwrap();
    let label = |s: &Act1Sim| {
        let row = donut_ability_row(
            Control::Player(0),
            magic_of(s.combat.donut_mana_view(), None),
            EmptyPress::Dry,
            true,
            true,
            SpecialSlot::Locked,
        );
        match row {
            AbilityRow::Chips { a, .. } => a.label,
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(label(&s), LabelId::Swat);
    s.step([swap(), RawInput::NONE]);
    let ex = s.exit_x();
    while s.unlocks().is_empty() && s.tick() < 400 {
        assert_eq!(label(&s), LabelId::Swat, "tick {}", s.tick());
        let x = s.x_of(HeroId::Donut);
        s.step([walk_in(x, ex), RawInput::NONE]);
    }
    assert_eq!(label(&s), LabelId::Missile, "from the unlock tick on");
}

/// The Pilot parser and fits.
#[test]
fn the_pilots_parse_and_fit_their_rooms() {
    assert_eq!(Pilot::parse("walk"), Ok(Pilot::Walk));
    assert_eq!(Pilot::parse("donut"), Ok(Pilot::Donut));
    assert_eq!(Pilot::parse("strike"), Ok(Pilot::Strike));
    assert!(Pilot::parse("run").is_err());
    assert!(Pilot::Donut.fits(Act1Room::Guild) && !Pilot::Donut.fits(Act1Room::Shore));
    assert!(Pilot::Strike.fits(Act1Room::Shore) && !Pilot::Strike.fits(Act1Room::Guild));
    assert!(Pilot::Walk.fits(Act1Room::Shore) && Pilot::Walk.fits(Act1Room::Guild));
}

/// Evidence helper (ignored): one line per case with the exit tick, the grants, the door and the
/// final hash. `cargo test --lib -- --ignored print_the_case_outcomes --nocapture`.
#[test]
#[ignore = "evidence helper: prints the case outcomes"]
fn print_the_case_outcomes() {
    for case in Case::ALL {
        let s = sim(case);
        println!(
            "ACT1_FIXTURE name={} room={} ticks={} exit={} defeats={:?} unlocks={:?} door_events={:?} casts={} door_stands={:?} hash={:#018x}",
            case.name(),
            case.room().id(),
            s.tick(),
            s.complete(),
            s.defeats(),
            s.unlocks(),
            s.door_events(),
            s.casts(),
            s.door_stands(),
            s.hash(),
        );
    }
    // The shore's block kill: the break, the landing and the defeats by tick.
    let s = sim(Case::ShoreBlockKill);
    for e in &s.combat.events {
        if matches!(
            e,
            CombatEvent::ShoreBroken { .. }
                | CombatEvent::ShoreLanded { .. }
                | CombatEvent::EnemyDefeated { .. }
        ) {
            println!("ACT1_BLOCK_KILL {e}");
        }
    }
}
