//! CF02 replay fixtures (CF02-00: an empty registry, plus the listing helper).
//!
//! The registry has the shape of [`crate::donut_fixtures::DONUT_FIXTURES`]
//! (`&PartyFixture`) so a `cf02_*` fixture replays through `--party-fixture`.
//! CF02-A, H, B, G and F append here; Donut and llama fixtures stay in
//! `donut_fixtures.rs` and the COMPANION ones in `companion_fixtures.rs`.
//! `CF02_FIXTURES` is deliberately not chained into `PartyFixture::all()` (so
//! `PartyFixture::by_name` and generic tests over `all()` do not see it);
//! `--party-fixture` and `--list-fixtures` use [`find_party_fixture`] and
//! [`list_fixtures`].

use crate::combat_scene::FIXTURES;
use crate::companion_fixtures::COMPANION_FIXTURES;
use crate::party_scene::PartyFixture;

pub const CF02_FIXTURES: [&PartyFixture; 0] = [];

/// Looks a party fixture up in `PartyFixture::all()` and then in the CF02
/// registry (which is not chained into `all()`).
pub fn find_party_fixture(name: &str) -> Option<&'static PartyFixture> {
    PartyFixture::by_name(name).or_else(|| CF02_FIXTURES.iter().copied().find(|f| f.name() == name))
}

/// Every fixture name starting with `prefix`, over every registry: the legacy
/// combat fixtures, every party fixture (party, boss, Donut and COMPANION) and
/// the CF02 registry, each name once, in registry order. An unmatched prefix
/// gives an empty list; an empty prefix lists everything.
pub fn list_fixtures(prefix: &str) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    let all = FIXTURES
        .iter()
        .map(|f| f.name)
        .chain(PartyFixture::all().map(PartyFixture::name))
        .chain(CF02_FIXTURES.iter().map(|f| f.name()))
        .chain(COMPANION_FIXTURES.iter().map(|f| f.name()));
    for name in all {
        if name.starts_with(prefix) && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_new_registries_are_empty_after_cf02_00() {
        assert!(CF02_FIXTURES.is_empty());
        assert!(COMPANION_FIXTURES.is_empty());
        // The CF02 llama fixtures (CF02-C1) live in `donut_fixtures.rs`, which
        // `PartyFixture::all()` chains, so the listing sees them.
        assert!(
            list_fixtures("cf02_")
                .iter()
                .all(|n| n.starts_with("cf02_llama_") || *n == "cf02_donut_solo_llama")
        );
        assert!(list_fixtures("companion_").is_empty());
        assert!(list_fixtures("no_such_prefix_").is_empty());
    }

    /// The CI loops select fixtures by prefix: a name outside its registry's
    /// prefix would never run (a silent skip).
    #[test]
    fn registry_names_carry_the_prefix_the_ci_loops_select() {
        for f in CF02_FIXTURES {
            let name = f.name();
            assert!(name.starts_with("cf02_"), "{name}");
            assert!(
                !name.starts_with("cf02_assist_"),
                "{name} belongs to COMPANION"
            );
        }
        for f in COMPANION_FIXTURES {
            let name = f.name();
            assert!(
                name.starts_with("companion_") || name.starts_with("cf02_assist_"),
                "{name}"
            );
        }
    }

    #[test]
    fn listing_covers_every_existing_registry_once() {
        let all = list_fixtures("");
        let mut unique = all.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), all.len(), "each name is listed once");
        for name in [
            "cp01_kick",
            "cp05_llama_crush",
            "solo_swap_continuity",
            "boss01_full_clear_2p",
            "q10_pounce_boss",
        ] {
            assert!(all.contains(&name), "{name}");
        }
        assert_eq!(list_fixtures("q10_").len(), 8);
        assert_eq!(list_fixtures("cp05_llama").len(), 5);
        assert!(find_party_fixture("q10_pounce_hit").is_some());
        assert!(find_party_fixture("cf02_nope").is_none());
    }

    /// CF02-00 baseline for CF02-Q20: the legacy solo-swap pan, measured on the
    /// same setup as the party test `q7_a_swap_pan_may_exceed_...` (Donut far
    /// behind, Carl runs 30 ticks, swap, 80 idle ticks). Run with
    /// `cargo test --locked cf02_baseline_swap_pan_trace -- --nocapture` to print
    /// the table recorded in `docs/evidence/cf02/audit/camera_trace.md`.
    #[test]
    fn cf02_baseline_swap_pan_trace() {
        use crate::env_kit::EnvKit;
        use crate::input::RawInput;
        use crate::party::HeroId;
        use crate::party_scene::{PartyFixtureSim, SOLO_SWAP_CONTINUITY};
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(&root, "assets/levels/floor01/corridor_abc_sample.json").unwrap();
        let mut sim = PartyFixtureSim::new(&kit, 360, &SOLO_SWAP_CONTINUITY).unwrap();
        sim.scene.world.donut_body.x = 100.0;
        sim.scene.world.sync_donut_for_tests();
        let idle = RawInput::NONE;
        for _ in 0..30 {
            sim.step_slots([
                RawInput {
                    move_x: 1.0,
                    ..RawInput::NONE
                },
                idle,
            ]);
        }
        let distance_before = (sim.scene.world.hero.x - sim.scene.world.donut_body.x).round();
        sim.step_slots([
            RawInput {
                swap: true,
                ..RawInput::NONE
            },
            idle,
        ]);
        assert_eq!(sim.party.controlled_hero(), HeroId::Donut);
        let mut last = sim.scene.camera.left();
        let mut steps = Vec::new();
        for n in 1..=80 {
            sim.step_slots([idle, idle]);
            let now = sim.scene.camera.left();
            steps.push(now - last);
            if n <= 21 {
                println!(
                    "CF02_BASELINE_SWAP_PAN tick_after_swap={n} left={now} dx={}",
                    now - last
                );
            }
            last = now;
        }
        println!("CF02_BASELINE_SWAP_PAN hero_gap_px={distance_before}");
        // Legacy pan (the baseline CF02-Q20 replaces): a hard start at the full
        // 8 px/tick cap on the first tick, 18 identical steps (the last one
        // shorter on arrival), then a hard stop.
        let mut expected = vec![-8; 18];
        expected.push(-5);
        expected.resize(80, 0);
        assert_eq!(steps, expected);
        assert!(!sim.scene.pan_boosted());
    }
}
