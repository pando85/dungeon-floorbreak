//! DONUT-MOBILITY-01 DM-B: the cat reflex in the real scene (spec section 7,
//! items 9 to 12), through the public API of the fixtures: the evade, the
//! biscuit gate, the 720-tick boundary, human and AI parity, and the share
//! measurement (section 3.3, no target). The hooks that need private access
//! (resets, guards, the non-evadable sources, the ram and the glob) are in
//! `combat_scene/evade/tests.rs`.

use std::path::PathBuf;

use crate::agility::ReflexProfile;
use crate::agility_fixtures::*;
use crate::combat::CombatEvent;
use crate::env_kit::EnvKit;
use crate::party::{HeroId, PROVISIONAL_TUNING, PartyTuning};
use crate::party_scene::{PartyFixture, PartyFixtureSim};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn corridor() -> EnvKit {
    EnvKit::load(&root(), "assets/levels/floor01/corridor_abc_sample.json").unwrap()
}

fn run(fixture: &'static PartyFixture) -> PartyFixtureSim {
    let mut sim = PartyFixtureSim::new(&corridor(), 360, fixture).unwrap();
    sim.run_scripted(fixture.base.ticks);
    sim
}

/// `(tick, hero_is_donut)` of every Donut hurt.
fn donut_hurts(sim: &PartyFixtureSim) -> Vec<(u64, u8)> {
    sim.combat
        .events
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::HeroHurt {
                tick,
                hero: HeroId::Donut,
                health_left,
                ..
            } => Some((tick, health_left)),
            _ => None,
        })
        .collect()
}

fn evades(sim: &PartyFixtureSim) -> Vec<u64> {
    sim.combat
        .events
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::HeroEvaded { tick, .. } => Some(tick),
            _ => None,
        })
        .collect()
}

fn armed(sim: &PartyFixtureSim) -> Vec<u64> {
    sim.combat
        .events
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::EvadeArmed { tick, .. } => Some(tick),
            _ => None,
        })
        .collect()
}

/// The goldens of the DM-B party fixtures, pinned from their first run; the
/// event assertions below are the independent check of what each hash means.
const GOLDENS: [(&str, u64); 4] = [
    ("dm_reflex_evades_swing", 0x4aca_1b4e_e7b3_9bbe),
    ("dm_reflex_not_armed", 0x8281_2a44_46dd_6c78),
    ("dm_evasion_locked_before_biscuit", 0x5a19_8a6c_5553_6f0d),
    ("dm_evasion_unlocks_at_guild", 0x1f1c_3dfb_5495_417e),
];

const FIXTURES: [&PartyFixture; 4] = [
    &DM_REFLEX_EVADES_SWING,
    &DM_REFLEX_NOT_ARMED,
    &DM_EVASION_LOCKED_BEFORE_BISCUIT,
    &DM_EVASION_UNLOCKS_AT_GUILD,
];

#[test]
fn the_reflex_fixtures_replay_identically_and_match_their_goldens() {
    for (fixture, (name, golden)) in FIXTURES.iter().zip(GOLDENS) {
        assert_eq!(fixture.name(), name);
        assert!(crate::cf02_fixtures::find_party_fixture(name).is_some());
        let hash = run(fixture).state_hash();
        assert_eq!(hash, run(fixture).state_hash(), "{name} twice");
        assert_eq!(hash, golden, "{name}: {hash:#018x}");
    }
}

/// Spec test 9: an armed Donut evades one swing (no `HeroHurt`, one
/// `HeroEvaded`, health unchanged), the second swing on the next tell lands.
#[test]
fn an_armed_donut_evades_one_swing_and_the_next_lands() {
    let sim = run(&DM_REFLEX_EVADES_SWING);
    assert_eq!(armed(&sim), vec![720], "armed at exactly 720");
    assert_eq!(evades(&sim), vec![729], "one evade, on the first swing");
    let hurts = donut_hurts(&sim);
    assert_eq!(
        hurts.first(),
        Some(&(820, 3)),
        "the evaded swing did no hurt; the second lands: {hurts:?}"
    );
    // The evade logged the attacker (goblin 0) and its kind.
    assert!(sim.combat.events.iter().any(|e| matches!(
        e,
        CombatEvent::HeroEvaded {
            tick: 729,
            hero: HeroId::Donut,
            source: 0,
            kind: crate::agility::EvadeSource::GoblinSwing,
        }
    )));
    // Health: untouched by the evade, and one point lost on 820.
    let mut check = PartyFixtureSim::new(&corridor(), 360, &DM_REFLEX_EVADES_SWING).unwrap();
    let full = check.scene.world.donut.health;
    for _ in 0..819 {
        check.step_scripted();
    }
    assert_eq!(
        check.scene.world.donut.health, full,
        "no health lost by 819"
    );
    check.step_scripted();
    assert_eq!(check.scene.world.donut.health, full - 1);
    // The hurt on 820 reset the wait: no second arm inside this fixture.
    assert_eq!(armed(&sim).len(), 1);
}

/// Spec test 9: with the reflex armed only after 720 hurt-free ticks, a swing
/// at tick 719 lands and one at tick 720 is evaded. The goblin's swing tick
/// moves one tick per start tick of Donut's walk, so a sweep crosses the
/// boundary one tick at a time.
#[test]
fn the_reflex_evades_exactly_from_tick_720() {
    let mut seen = std::collections::BTreeMap::new();
    for start in 600..=640_u64 {
        let script: &'static [crate::party_scene::SlotHold] =
            Box::leak(Box::new([crate::party_scene::run(
                start,
                start + 80,
                1,
                1.0,
            )]));
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            script,
            ..clone_of(&DM_REFLEX_EVADES_SWING)
        }));
        let mut sim = PartyFixtureSim::new(&corridor(), 360, fixture).unwrap();
        sim.run_scripted(start + 160);
        let first_swing = sim
            .combat
            .events
            .iter()
            .find_map(|e| match *e {
                CombatEvent::GoblinSwing { tick, .. } => Some(tick),
                _ => None,
            })
            .expect("a swing");
        let evaded = evades(&sim).first().copied();
        let hurt = donut_hurts(&sim).first().map(|h| h.0);
        if first_swing >= 720 {
            assert_eq!(evaded, Some(first_swing), "swing on {first_swing}");
            assert_ne!(hurt, Some(first_swing), "swing on {first_swing}");
        } else {
            assert_eq!(hurt, Some(first_swing), "swing on {first_swing}");
            assert_eq!(evaded, None, "swing on {first_swing}");
        }
        seen.insert(first_swing, evaded.is_some());
    }
    assert_eq!(seen.get(&719), Some(&false), "719 lands");
    assert_eq!(seen.get(&720), Some(&true), "720 is evaded");
}

/// Spec test 10: before the biscuit the reflex never evades, no event is
/// logged and no evasion state is hashed (the run hashes like one with the
/// evasion off); the guild zone (or a fixture start) unlocks it, it first arms
/// exactly 720 ticks after the unlock tick, and her mobility is the same
/// before and after.
#[test]
fn the_reflex_is_a_biscuit_unlock_and_nothing_else() {
    // Locked: 900 hurt-free ticks, then the swing lands.
    let locked = run(&DM_EVASION_LOCKED_BEFORE_BISCUIT);
    assert!(armed(&locked).is_empty() && evades(&locked).is_empty());
    assert_eq!(donut_hurts(&locked).first(), Some(&(909, 3)));
    assert!(!locked.combat.reflex().ever_armed());
    let off: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        tuning: PartyTuning {
            donut_reflex: ReflexProfile::Off,
            ..PROVISIONAL_TUNING
        },
        ..clone_of(&DM_EVASION_LOCKED_BEFORE_BISCUIT)
    }));
    assert_eq!(
        run(off).state_hash(),
        locked.state_hash(),
        "a locked reflex hashes nothing"
    );
    // The guild: the unlock fires on tick 20, the reflex arms on 740 = 20 + 720,
    // and the swing on 741 is evaded.
    let guild = run(&DM_EVASION_UNLOCKS_AT_GUILD);
    let unlock = guild
        .combat
        .events
        .iter()
        .find_map(|e| match e {
            CombatEvent::Mana(crate::donut_mana::ManaEvent::UnlockGranted { tick, .. }) => {
                Some(*tick)
            }
            _ => None,
        })
        .expect("the guild zone grants the biscuit");
    assert_eq!(unlock, 20);
    assert_eq!(armed(&guild), vec![unlock + 720]);
    assert_eq!(evades(&guild), vec![741]);
    assert!(donut_hurts(&guild).first().is_some_and(|h| h.0 > 741));
    // A fixture that starts unlocked arms at 720 (the unlock tick is 0).
    assert_eq!(armed(&run(&DM_REFLEX_EVADES_SWING)), vec![720]);
    // Mobility: the same run script moves her identically locked and unlocked
    // (the reflex touches no body).
    let trail = |fixture: &'static PartyFixture| {
        let mut sim = PartyFixtureSim::new(&corridor(), 360, fixture).unwrap();
        let mut xs = Vec::new();
        for _ in 0..700 {
            sim.step_scripted();
            xs.push(sim.scene.world.donut_body.x.to_bits());
        }
        xs
    };
    let locked_run: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
        donut: crate::donut_mana::DonutSetup::NONE,
        ..clone_of(&DM_REFLEX_EVADES_SWING)
    }));
    assert_eq!(
        trail(locked_run),
        trail(&DM_REFLEX_EVADES_SWING),
        "her motion is the same before and after the unlock"
    );
}

fn clone_of(f: &'static PartyFixture) -> PartyFixture {
    PartyFixture {
        base: f.base,
        two_player: f.two_player,
        tuning: f.tuning,
        script: f.script,
        pilot: f.pilot,
        donut: f.donut,
    }
}

/// DM-C F2: the pinned `state_hash()` of the two parity sims below.
const DM_PARITY_HUMAN_HASH: u64 = 0xd78f_99e0_b01d_cdb9;
const DM_PARITY_AI_HASH: u64 = 0x831f_3727_988f_62b3;

/// Spec test 11: the reflex decides on the attack instance and her state only.
/// The same room with Donut human-driven and AI-driven, at identical body
/// columns, gives the same evade and hurt events, tick for tick.
#[test]
fn a_human_donut_and_an_ai_donut_get_the_same_evade_decisions() {
    let play = |fixture: &'static PartyFixture| {
        let mut sim = PartyFixtureSim::new(&corridor(), 360, fixture).unwrap();
        sim.scene.world.donut_body.x = 360.0;
        sim.scene.world.sync_donut_for_tests();
        let mut columns = Vec::new();
        for tick in 1..=1000_u64 {
            if tick == 691 {
                // The goblin walks in from the left of Donut, who is its nearest hero.
                sim.combat.enemies[0].x = 250.0;
            }
            sim.step_scripted();
            columns.push(sim.scene.world.donut_body.x.to_bits());
        }
        (sim, columns)
    };
    let (human, human_columns) = play(&DM_AI_REFLEX_HUMAN);
    let (ai, ai_columns) = play(&DM_AI_REFLEX_SAME_AS_HUMAN);
    assert!(
        ai.scene.drive().is_bot(HeroId::Donut) && !human.scene.drive().is_bot(HeroId::Donut),
        "one run is AI-driven, the other human-driven"
    );
    assert_eq!(human_columns, ai_columns, "identical body columns");
    let evade_events = |sim: &PartyFixtureSim| -> Vec<String> {
        sim.combat
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    CombatEvent::HeroEvaded { .. }
                        | CombatEvent::EvadeArmed { .. }
                        | CombatEvent::HeroHurt {
                            hero: HeroId::Donut,
                            ..
                        }
                )
            })
            .map(ToString::to_string)
            .collect()
    };
    // DM-C F2: both hand-driven sims are deterministic, so their hashes are pinned
    // (they differ: the party state differs, the decisions do not).
    assert_eq!(
        human.state_hash(),
        DM_PARITY_HUMAN_HASH,
        "{:#018x}",
        human.state_hash()
    );
    assert_eq!(
        ai.state_hash(),
        DM_PARITY_AI_HASH,
        "{:#018x}",
        ai.state_hash()
    );
    let (human_events, ai_events) = (evade_events(&human), evade_events(&ai));
    assert_eq!(human_events, ai_events);
    assert!(
        human_events.iter().any(|l| l.contains("hero_evaded")),
        "the scenario evades: {human_events:?}"
    );
}

// ------------------------------------------------------ the share (3.3) --

/// The rotten shore's block source id in `HeroHurt` (never evadable).
const SHORE_BLOCK_SOURCE: u32 = crate::cf02::arena::SHORE_BLOCK_SOURCE;

fn kit_of(fixture: &PartyFixture) -> EnvKit {
    if fixture.base.boss.is_some() {
        crate::boss_scene::load_arena(&root()).unwrap()
    } else if crate::cf02::arena::fixture_uses_arena(fixture.name()) {
        crate::cf02::arena::load_arena(&root()).unwrap()
    } else {
        corridor()
    }
}

/// The fixture with the biscuit and a full pool (evasion exists only after it)
/// and the reflex at `rearm` ticks, the `with_tuning` overlay of
/// `donut_fixtures.rs`.
fn overlay(f: &'static PartyFixture, rearm: u32) -> &'static PartyFixture {
    overlay_with_bias(f, rearm, f.tuning.target_ai_bias_px)
}

/// [`overlay`] with the AI target bias set (DM-C F6: 0 makes an AI Donut draw
/// attacks like a human one, the owner's `--cf02 --donut-biscuit` line).
fn overlay_with_bias(f: &'static PartyFixture, rearm: u32, bias_px: u16) -> &'static PartyFixture {
    Box::leak(Box::new(PartyFixture {
        tuning: PartyTuning {
            donut_reflex: ReflexProfile::Rearm(rearm),
            target_ai_bias_px: bias_px,
            ..f.tuning
        },
        donut: crate::donut_mana::DonutSetup::unlocked(8),
        ..clone_of(f)
    }))
}

fn play(fixture: &'static PartyFixture, seed: Option<u64>, prearmed: bool) -> PartyFixtureSim {
    let kit = kit_of(fixture);
    let mut sim = match seed {
        None => PartyFixtureSim::new(&kit, 360, fixture).unwrap(),
        Some(seed) => {
            let mut scene =
                crate::corridor::SceneSim::new_at(&kit, 360, fixture.base.carl_x).unwrap();
            fixture.place_donut(&mut scene);
            let combat = crate::combat_scene::CombatState::new(fixture.base, &kit, &scene)
                .unwrap()
                .with_donut(fixture.donut)
                .with_story(crate::carl_prog_fixtures::story_setup(fixture.name()))
                .with_cf02_seed(Some(seed));
            PartyFixtureSim::from_parts(scene, combat, fixture)
        }
    };
    if prearmed {
        // As if the biscuit had been held and she had been hurt-free for 12 s or
        // more when the room began: the reflex is ready at tick 0.
        sim.combat.arm_reflex_for_test();
    }
    sim.run_scripted(fixture.base.ticks);
    sim
}

/// The microarena's waves by enemy target id: goblins 0..=2 are the brawl lane
/// (wave 1); goblins 3 and 4, the Engineer that replaces the yard goblin and the
/// llama are the yard (wave 2). **Meaningful only for the three microarena
/// pilots** (the split is the arena's); the other rows use the totals.
fn wave_of(source: u32) -> usize {
    usize::from(source >= 3)
}

#[derive(Default, Debug, Clone, PartialEq)]
struct Tally {
    attempts: u32,
    evades: u32,
    /// Per wave `[wave 1, wave 2]`: attempts and evades.
    wave_attempts: [u32; 2],
    wave_evades: [u32; 2],
    /// The first evadable attempt on Donut of each wave of each attempt of the
    /// run (the opener, spec U1), and how many of those were evaded.
    openers: u32,
    opener_evades: u32,
    /// The attempts (a run is one attempt plus one per restart) that held at
    /// least one evade, and the number of attempts.
    attempts_with_evade: u32,
    segments: u32,
    /// The earliest and latest tick of the first evadable attempt on Donut of
    /// each segment that had one.
    first_tick_min: Option<u64>,
    first_tick_max: Option<u64>,
}

impl Tally {
    fn add(&mut self, other: &Self) {
        self.attempts += other.attempts;
        self.evades += other.evades;
        for w in 0..2 {
            self.wave_attempts[w] += other.wave_attempts[w];
            self.wave_evades[w] += other.wave_evades[w];
        }
        self.openers += other.openers;
        self.opener_evades += other.opener_evades;
        self.attempts_with_evade += other.attempts_with_evade;
        self.segments += other.segments;
        self.first_tick_min = [self.first_tick_min, other.first_tick_min]
            .into_iter()
            .flatten()
            .min();
        self.first_tick_max = [self.first_tick_max, other.first_tick_max]
            .into_iter()
            .flatten()
            .max();
    }
}

/// DM-C F5: whether a hurt's `source` is an evadable instance. Evadable sources
/// are the enemy ids (goblin swings and Engineer rams: the enemy index; globs:
/// the goblin count plus the llama index); every other id (the shore block, a
/// boss blow, a later rush, contact or hazard) is excluded.
fn is_evadable_source(sim: &PartyFixtureSim, source: u32) -> bool {
    source != SHORE_BLOCK_SOURCE
        && source != crate::boss::BOSS_SOURCE_ID
        && (source as usize) < sim.combat.enemies.len() + sim.combat.llamas.len()
}

/// Count the evadable attempts on Donut from the event log: a `HeroEvaded`, or a
/// `HeroHurt` on her from an evadable source ([`is_evadable_source`]). The
/// test-only counter of `Reflex::check` (the spec's definition: instances that
/// reach the evasion check) must agree.
fn tally(sim: &PartyFixtureSim) -> Tally {
    let mut t = Tally {
        segments: 1,
        ..Tally::default()
    };
    let mut opened = [false; 2];
    let mut evaded_this_attempt = false;
    let mut first_of_segment = true;
    for event in &sim.combat.events {
        let (source, evaded, tick) = match *event {
            CombatEvent::HeroEvaded { source, tick, .. } => (source, true, tick),
            CombatEvent::HeroHurt {
                hero: HeroId::Donut,
                source,
                tick,
                ..
            } if is_evadable_source(sim, source) => (source, false, tick),
            CombatEvent::Restart { .. } => {
                t.attempts_with_evade += u32::from(evaded_this_attempt);
                t.segments += 1;
                evaded_this_attempt = false;
                first_of_segment = true;
                opened = [false; 2];
                continue;
            }
            _ => continue,
        };
        if first_of_segment {
            first_of_segment = false;
            t.first_tick_min = Some(t.first_tick_min.map_or(tick, |m| m.min(tick)));
            t.first_tick_max = Some(t.first_tick_max.map_or(tick, |m| m.max(tick)));
        }
        let wave = wave_of(source);
        t.attempts += 1;
        t.wave_attempts[wave] += 1;
        if evaded {
            t.evades += 1;
            t.wave_evades[wave] += 1;
            evaded_this_attempt = true;
        }
        if !opened[wave] {
            opened[wave] = true;
            t.openers += 1;
            t.opener_evades += u32::from(evaded);
        }
    }
    t.attempts_with_evade += u32::from(evaded_this_attempt);
    assert_eq!(
        t.attempts,
        sim.combat.reflex().attempts_for_test(),
        "the event tally and the counter at the check agree"
    );
    t
}

/// Spec section 3.3: the share of evadable attempts the reflex evades, at
/// rearm 600, 720 and 900, over every `cf02_*` party fixture with the biscuit,
/// plus the arena pilots at seeds 0..8. **No target:** this asserts only that
/// the table is well formed and prints it (`--nocapture`); `DM_B_SHARE_OUT`
/// writes it to a file for the PR body and the evidence README.
#[test]
fn the_evade_share_is_measured_not_tuned() {
    const SEEDS: u64 = 48;
    use crate::cf02_fixtures::{CF02_FIXTURES, ENG_SOLO_ASSIST};
    use crate::companion_fixtures::COMPANION_FIXTURES;
    let pilots: [&'static PartyFixture; 3] = [
        crate::cf02_fixtures::find_party_fixture("cf02_arena_direct").unwrap(),
        crate::cf02_fixtures::find_party_fixture("cf02_arena_swap").unwrap(),
        &ENG_SOLO_ASSIST,
    ];
    let mut out = String::new();
    out.push_str("DM-B reflex share (spec 3.3, extended by DM-C F6): evades / attempts\n");
    out.push_str(
        "attempt = an evadable instance that reached the check on Donut (evaded or hurt)\n",
    );
    out.push_str(
        "A = biscuit from tick 0, as the spec's population (the reflex first arms at the rearm)\n",
    );
    out.push_str("B = biscuit held and 12 s or more hurt-free before the room (armed at tick 0)\n");
    out.push_str(
        "population = every cf02_* and companion party fixture; AI-Donut pilots = three solo pilots, 48 seeds each\n",
    );
    out.push_str(
        "human-style Donut (DM-C F6) = cf02_arena_coop only (P2 played as a human would). The three solo pilots with target_ai_bias_px 0 are still AI-Donut data (bias 0 changes nothing in these rooms: the rows equal the AI-Donut rows), so the combined total below is mostly AI-Donut data; read the coop row as the human-style number\n",
    );
    out.push_str(
        "the AI-Donut rows measure the AI Donut, which the shipped rooms almost never attack; with a 12 s rearm the reflex is either not yet ready or swallows the opener of the next wave\n",
    );
    out.push_str("regenerate: DM_B_SHARE_OUT=<file> cargo test --locked --lib the_evade_share\n\n");
    let mut shares = Vec::new();
    for (label, prearmed) in [("A", false), ("B", true)] {
        for rearm in [600_u32, 720, 900] {
            let mut population = Tally::default();
            for fixture in CF02_FIXTURES.iter().chain(COMPANION_FIXTURES.iter()) {
                population.add(&tally(&play(overlay(fixture, rearm), None, prearmed)));
            }
            let mut pilot_rows = Vec::new();
            let mut pilots_all = Tally::default();
            for pilot in pilots {
                let mut row = Tally::default();
                for seed in 0..SEEDS {
                    row.add(&tally(&play(overlay(pilot, rearm), Some(seed), prearmed)));
                }
                pilots_all.add(&row);
                pilot_rows.push((pilot.name(), row));
            }
            // DM-C F6: a human-driven Donut. `cf02_arena_coop` is the two-player
            // room with P2 Donut played as a human would (a step behind Carl,
            // swatting tells); the three solo pilots again with `target_ai_bias_px`
            // 0, so the AI Donut draws attacks as a human Donut does.
            let mut human_rows = Vec::new();
            let mut human_all = Tally::default();
            let coop = crate::cf02_fixtures::find_party_fixture("cf02_arena_coop").unwrap();
            let mut row = Tally::default();
            for seed in 0..SEEDS {
                row.add(&tally(&play(overlay(coop, rearm), Some(seed), prearmed)));
            }
            human_all.add(&row);
            human_rows.push(("cf02_arena_coop (P2 human-style Donut)".to_owned(), row));
            for pilot in pilots {
                let mut row = Tally::default();
                for seed in 0..SEEDS {
                    let fixture = overlay_with_bias(pilot, rearm, 0);
                    row.add(&tally(&play(fixture, Some(seed), prearmed)));
                }
                human_all.add(&row);
                human_rows.push((format!("{} (unbiased targeting)", pilot.name()), row));
            }
            let share = |t: &Tally| {
                if t.attempts == 0 {
                    0.0
                } else {
                    100.0 * f64::from(t.evades) / f64::from(t.attempts)
                }
            };
            out.push_str(&format!("== {label}: rearm {rearm} ticks\n"));
            out.push_str(&format!(
                "population ({} cf02 and companion fixtures): attempts {} evades {} share {:.1}%\n",
                CF02_FIXTURES.len() + COMPANION_FIXTURES.len(),
                population.attempts,
                population.evades,
                share(&population)
            ));
            let row_line = |kind: &str, name: &str, row: &Tally| {
                format!(
                    "{kind} {name} ({SEEDS} seeds): attempts {} evades {} share {:.1}%{} | wave1 {}/{} wave2 {}/{} | openers {}/{} evaded | attempts-with-evade {} of {} attempts | first attempt on her at ticks {}..{}\n",
                    row.attempts,
                    row.evades,
                    share(row),
                    if row.attempts < 40 {
                        " (insufficient sample)"
                    } else {
                        ""
                    },
                    row.wave_evades[0],
                    row.wave_attempts[0],
                    row.wave_evades[1],
                    row.wave_attempts[1],
                    row.opener_evades,
                    row.openers,
                    row.attempts_with_evade,
                    row.segments,
                    row.first_tick_min.map_or("-".to_owned(), |t| t.to_string()),
                    row.first_tick_max.map_or("-".to_owned(), |t| t.to_string())
                )
            };
            for (name, row) in &pilot_rows {
                out.push_str(&row_line("AI-Donut pilot", name, row));
            }
            out.push_str(&format!(
                "AI-Donut pilots all: attempts {} evades {} share {:.1}% | openers {}/{} evaded\n",
                pilots_all.attempts,
                pilots_all.evades,
                share(&pilots_all),
                pilots_all.opener_evades,
                pilots_all.openers
            ));
            for (name, row) in &human_rows {
                let kind = if name.starts_with("cf02_arena_coop") {
                    "human-style Donut"
                } else {
                    "AI-Donut pilot, bias 0 (equals the AI rows)"
                };
                out.push_str(&row_line(kind, name, row));
            }
            out.push_str(&format!(
                "coop plus bias-0 pilots (mostly AI-Donut data, not a human-style total): attempts {} evades {} share {:.1}% | openers {}/{} evaded\n\n",
                human_all.attempts,
                human_all.evades,
                share(&human_all),
                human_all.opener_evades,
                human_all.openers
            ));
            assert!(human_all.evades <= human_all.attempts);
            assert!(population.evades <= population.attempts);
            assert!(pilots_all.evades <= pilots_all.attempts);
            shares.push((label, rearm, population, pilots_all, human_all));
        }
    }
    println!("{out}");
    if let Ok(path) = std::env::var("DM_B_SHARE_OUT") {
        std::fs::write(path, &out).unwrap();
    }
    // The sample rule of spec 3.3: at least 40 attempts per row, or the PR says
    // "insufficient sample" and adds seeds.
    for (label, rearm, population, pilots_all, human_all) in &shares {
        println!(
            "{label} rearm {rearm}: population attempts {} AI pilots attempts {} human-style attempts {}",
            population.attempts, pilots_all.attempts, human_all.attempts
        );
    }
}
