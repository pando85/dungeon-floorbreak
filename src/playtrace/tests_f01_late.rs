//! PT-SCENE proofs: the descriptor's `f01_room` and `f01_loadout` round-trip
//! and stay out of every older descriptor, a recorded session of each listed
//! room (the Boiler, the Hoarder, the hazard room and the Juicer, under both
//! loadouts) verifies strict from
//! its reset descriptor, a relabelled room or loadout is rejected, and the room
//! pins are scoped to a recording that names a room (U5 of the #207 review).
//!
//! The rig mirrors the order of operations of `fixed_step` in `floor01_scene`
//! (see `live_tests`) and builds each room the way the binary does.

use std::path::{Path, PathBuf};

use super::live::{self, LiveRecorder};
use super::live_tests::{DemoRig, TempDir, real_kit_hash, root};
use super::tests_verify::tamper_manifest;
use super::*;
use crate::combat::CombatEvent;
use crate::combat_scene::CombatState;
use crate::companion::profile::ProfileSetId;
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::f01_late::{self, Loadout};
use crate::input::RawInput;
use crate::party::{PROVISIONAL_TUNING, PartyState, PartyTuning, UncontrolledHero};

fn verify(session: &Path) -> Result<VerifyReport, VerifyFailure> {
    verify_dir(session, &root())
}

struct RoomRig {
    scene: SceneSim,
    combat: CombatState,
    party: PartyState,
    rec: LiveRecorder,
}

impl RoomRig {
    fn start(base: &Path, room: &str, loadout: Loadout) -> Self {
        Self::start_seeded(base, room, loadout, None)
    }

    /// `seed` is `--cf02-seed` (the hazard room and the Boiler take one).
    fn start_seeded(base: &Path, room: &str, loadout: Loadout, seed: Option<u64>) -> Self {
        Self::build(base, room, Some(loadout), seed, None)
    }

    /// F01-RUN: the room of a floor run, started from the ledger the run carried in.
    fn start_carried(base: &Path, room: &str, carry: &crate::story_unlock::LedgerSnapshot) -> Self {
        Self::build(base, room, Some(Loadout::Story), None, Some(carry))
    }

    /// The Hoarder's own scene (`--scene f01_hoarder`): no loadout label and no story programs.
    fn start_direct(base: &Path, room: &str) -> Self {
        Self::build(base, room, None, None, None)
    }

    fn build(
        base: &Path,
        room: &str,
        loadout: Option<Loadout>,
        seed: Option<u64>,
        carry: Option<&crate::story_unlock::LedgerSnapshot>,
    ) -> Self {
        let (carl, donut) = loadout.map_or_else(Default::default, Loadout::flags);
        // The live party of a CF02-rules room: `cf02_assist` and the 12-tick swap.
        let room_party = || {
            PartyState::new(
                PartyTuning {
                    uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
                    ..PROVISIONAL_TUNING
                }
                .with_cf02_swap(),
            )
        };
        let (source, name, cf02, scene, mut combat, party) = if room == "boiler" {
            // BOIL-E: the story launcher's label means the live roster, the
            // rule the binary and the verifier share.
            let (run, scene, combat) = f01_late::boiler_start_with(
                &root(),
                360,
                carl,
                donut,
                seed,
                crate::f01_boiler::Roster::for_loadout(loadout.map(Loadout::name)),
            )
            .expect("the Boiler launches");
            (
                f01_late::SESSION_SOURCE,
                run.fixture.name,
                true,
                scene,
                combat,
                room_party(),
            )
        } else if room == "hoarder" {
            // HRD-A2b: the one constructor the binary and the verifier share.
            let (run, scene, combat) = f01_late::hoarder_start(&root(), 360, carl, donut)
                .expect("the Hoarder's room launches");
            (
                f01_late::SESSION_SOURCE,
                run.fixture.name,
                true,
                scene,
                combat,
                room_party(),
            )
        } else if room == "hazards" {
            // HAZ-A2b: the same for the hazard room.
            let (run, scene, combat) = f01_late::hazards_start(&root(), 360, carl, donut, seed)
                .expect("the hazard room launches");
            (
                f01_late::SESSION_SOURCE,
                run.fixture.name,
                true,
                scene,
                combat,
                room_party(),
            )
        } else if room == "mastery" {
            // PT-MASTERY: the closing arena, the same way.
            let (run, scene, combat) = f01_late::mastery_start(&root(), 360, carl, donut, seed)
                .expect("the mastery arena launches");
            (
                f01_late::SESSION_SOURCE,
                run.fixture.name,
                true,
                scene,
                combat,
                room_party(),
            )
        } else if room == crate::f01_corral::ROOM_ID {
            // CORRAL-WIRE/CORRAL-F: room 0 (listed first since CORRAL-F), the same way.
            let (run, scene, combat) = f01_late::corral_start(&root(), 360, carl, donut, seed)
                .expect("the corral launches");
            (
                f01_late::SESSION_SOURCE,
                run.fixture.name,
                true,
                scene,
                combat,
                room_party(),
            )
        } else if let Some(ring) = crate::f01_ring::RingRoom::from_id(room) {
            // RING-A/B wire: a Ring room, the same way.
            let (run, scene, combat) = f01_late::ring_start(&root(), ring, 360, carl, donut, seed)
                .expect("the Ring room launches");
            (
                f01_late::SESSION_SOURCE,
                run.fixture.name,
                true,
                scene,
                combat,
                room_party(),
            )
        } else {
            let kit = crate::boss_scene::load_arena(&root()).expect("the boss arena");
            let fixture = crate::boss_scene::BOSS01_SCENE;
            let scene = SceneSim::new_at(&kit, 360, fixture.carl_x).unwrap();
            let combat = CombatState::new(fixture, &kit, &scene)
                .unwrap()
                .with_donut(DonutSetup::unlocked(8))
                .try_with_story(crate::donut_prog::story_setup(
                    carl,
                    donut,
                    crate::story_unlock::SceneKind::Boss,
                ))
                .unwrap();
            (
                "boss",
                fixture.name,
                false,
                scene,
                combat,
                PartyState::new(PROVISIONAL_TUNING),
            )
        };
        // F01-RUN: the binary carries the ledger after the room's constructor.
        if let Some(snapshot) = carry {
            combat.carry_story(snapshot).expect("the carry applies");
        }
        let reset = ResetDescriptor {
            source: source.to_owned(),
            name: name.to_owned(),
            demo: false,
            demo_beat: 0,
            cf02_rules: cf02,
            cf02_seed: seed,
            view_height: 360,
            donut_biscuit: room != "juicer" || donut.implies_biscuit(),
            carl_prog: carl.prog,
            carl_dynamite: carl.dynamite,
            carl_gauntlet: carl.gauntlet,
            donut_motion: party.tuning().donut_motion,
            donut_prog: donut.prog,
            donut_light_feet: donut.light_feet,
            donut_puddle: donut.puddle,
            f01_room: Some(room.to_owned()),
            f01_loadout: loadout.map(|l| l.name().to_owned()),
            f01_roster: (room == "boiler").then(|| {
                crate::f01_boiler::Roster::for_loadout(loadout.map(Loadout::name))
                    .name()
                    .to_owned()
            }),
            f01_carry: carry.cloned(),
            kit_hash: real_kit_hash(),
            start_hash: combat.state_hash(&scene),
        };
        let mut manifest = SessionManifest::new(generate_session_id(), reset);
        live::stamp_manifest(&mut manifest, real_kit_hash());
        let rec = LiveRecorder::start(base, manifest, &scene, Some(&combat), &party, false)
            .expect("the recorder starts");
        Self {
            scene,
            combat,
            party,
            rec,
        }
    }

    fn step(&mut self, carl: RawInput) {
        let slots = [carl, RawInput::NONE];
        self.rec.observe_party(&self.party);
        let hurt = self.combat.hurt_ticks_of(self.party.controlled_hero()) > 0;
        self.party.request_swap(slots[0].swap, hurt);
        let hero = self.party.hero_inputs(slots);
        let drive = self.party.drive(true);
        self.scene.set_drive(drive);
        self.rec.begin_tick(
            &self.scene,
            Some(&self.combat),
            &self.party,
            slots,
            hero,
            drive,
        );
        let restarts = self.combat.restarts;
        self.combat.step_party(&mut self.scene, hero);
        let respawned = self.combat.take_respawned();
        if self.combat.restarts != restarts || respawned {
            self.party.restart();
        }
        self.rec
            .end_tick(&self.scene, Some(&self.combat), &self.party, respawned);
    }

    fn walk(&mut self, ticks: u64) {
        for tick in 0..ticks {
            self.step(RawInput {
                move_x: if tick % 300 < 240 { 1.0 } else { 0.0 },
                attack: tick % 11 == 0,
                ..RawInput::NONE
            });
        }
    }
}

fn record(name: &str, room: &str, loadout: Loadout, ticks: u64) -> (TempDir, PathBuf) {
    let dir = TempDir::new(name);
    let mut rig = RoomRig::start(&dir.0, room, loadout);
    rig.walk(ticks);
    let manifest = rig.rec.finish("test").expect("finished");
    let session = dir.0.join(manifest.session_id);
    (dir, session)
}

#[test]
fn the_descriptor_fields_round_trip_and_stay_out_of_older_descriptors() {
    let mut reset = super::tests::reset();
    let plain = serde_json::to_string(&reset).unwrap();
    assert!(!plain.contains("f01_room") && !plain.contains("f01_loadout"));
    // An older descriptor (no fields) loads with both absent and re-serializes
    // byte-identically.
    let loaded: ResetDescriptor = serde_json::from_str(&plain).unwrap();
    assert_eq!((&loaded.f01_room, &loaded.f01_loadout), (&None, &None));
    assert_eq!(serde_json::to_string(&loaded).unwrap(), plain);
    // With the fields set they round-trip.
    reset.f01_room = Some("boiler".into());
    reset.f01_loadout = Some("story".into());
    let text = serde_json::to_string(&reset).unwrap();
    assert!(text.contains("\"f01_room\":\"boiler\"") && text.contains("\"f01_loadout\":\"story\""));
    let back: ResetDescriptor = serde_json::from_str(&text).unwrap();
    assert_eq!(back, reset);
}

#[test]
fn a_recorded_boiler_session_verifies_strict() {
    let (_dir, session) = record("f01-boiler", "boiler", Loadout::Story, 900);
    let report = verify(&session).expect("the Boiler session verifies");
    assert_eq!(report.ticks, 900);
    assert_eq!(report.scope, VerifyScope::Full);
    assert_eq!(report.build, "identical");
    let loaded = read_session(&session, false).unwrap();
    assert_eq!(loaded.manifest.reset.f01_room.as_deref(), Some("boiler"));
    assert_scoped_stamps(&loaded, "boiler");
}

/// A recording of `room` stamps exactly the room pins that cover it (and the
/// manifest pin), and none of another room's.
fn assert_scoped_stamps(loaded: &LoadedSession, room: &str) {
    for key in f01_late::SCOPED_PIN_KEYS {
        assert_eq!(
            loaded.manifest.content_hashes.contains_key(key),
            f01_late::pin_applies(key, Some(room)),
            "{room}: {key}"
        );
    }
}

/// HRD-A2b: a recorded Hoarder session (her slot enemy in the run) verifies
/// strict under both loadouts, stamps her room pins and the manifest's and not
/// another room's, and a relabelled room, loadout or seed is another run.
#[test]
fn a_recorded_hoarder_session_verifies_strict() {
    for loadout in [Loadout::Story, Loadout::All] {
        let dir = TempDir::new("f01-hoarder");
        let mut rig = RoomRig::start(&dir.0, "hoarder", loadout);
        assert_eq!(
            rig.combat.slot_enemies().len(),
            1,
            "she spawns from the roster"
        );
        rig.walk(900);
        let manifest = rig.rec.finish("test").expect("finished");
        let session = dir.0.join(manifest.session_id);
        let report = verify(&session).unwrap_or_else(|e| panic!("{}: {e}", loadout.name()));
        assert_eq!(report.ticks, 900);
        assert_eq!(report.scope, VerifyScope::Full);
        let loaded = read_session(&session, false).unwrap();
        assert_eq!(loaded.manifest.reset.name, "f01_hoarder");
        assert_scoped_stamps(&loaded, "hoarder");
        // The loadout label is bound to the flags, and the room has no seed.
        let other = if loadout == Loadout::Story {
            "all"
        } else {
            "story"
        };
        tamper_manifest(&session, |m| m.reset.f01_loadout = Some(other.into()));
        assert_eq!(
            verify(&session).expect_err("relabelled").code,
            "rules_mismatch"
        );
        tamper_manifest(&session, |m| {
            m.reset.f01_loadout = Some(loadout.name().into())
        });
        verify(&session).expect("restored");
        tamper_manifest(&session, |m| m.reset.cf02_seed = Some(7));
        assert_eq!(verify(&session).expect_err("a seed").code, "rules_mismatch");
    }
}

/// FA-Q24: her own scene (`--scene f01_hoarder`, no loadout label, no story programs)
/// still records and verifies strict now that her room is listed.
#[test]
fn a_recorded_direct_hoarder_session_verifies_strict() {
    let dir = TempDir::new("f01-hoarder-direct");
    let mut rig = RoomRig::start_direct(&dir.0, "hoarder");
    rig.walk(900);
    let manifest = rig.rec.finish("test").expect("finished");
    let session = dir.0.join(manifest.session_id);
    let report = verify(&session).expect("the direct scene verifies");
    assert_eq!(report.ticks, 900);
    let loaded = read_session(&session, false).unwrap();
    assert_eq!(loaded.manifest.reset.f01_loadout, None);
    assert_scoped_stamps(&loaded, "hoarder");
}

/// FA-Q24: a strict session of room 1 played by the room pilot to her defeat and out of the
/// room: it verifies from its reset descriptor, holds her one defeat and **exactly one**
/// `CapabilityGranted` (the dynamite, `special_thrown_charge`) in the journal, and it is the run
/// the library fixture reports for the seed.
#[test]
fn a_piloted_hoarder_session_has_exactly_one_capability_granted_and_verifies_strict() {
    let dir = TempDir::new("f01-hoarder-clear");
    let mut rig = RoomRig::start(&dir.0, "hoarder", Loadout::Story);
    let report = crate::hoarder_fixtures::room_run(&root(), 0).unwrap();
    let mut pilot = crate::hoarder_fixtures::ClearPilot::new(0);
    for _ in 0..report.ticks {
        let carl = pilot.input(&rig.combat, &rig.scene);
        rig.step(carl);
    }
    let defeats = rig
        .combat
        .events
        .iter()
        .filter(|e| matches!(e, CombatEvent::EnemyDefeated { .. }))
        .count();
    assert_eq!(defeats, 1, "she is defeated once");
    assert_eq!(rig.combat.deaths, 0, "no hero defeat on the way");
    assert!(rig.scene.world.complete, "Carl reaches the exit");
    assert_eq!(
        crate::party_scene::party_hash(&rig.combat, &rig.scene, &rig.party),
        report.hash,
        "the recorded run is the library fixture's"
    );
    let manifest = rig.rec.finish("test").expect("finished");
    let session = dir.0.join(manifest.session_id);
    let verified = verify(&session).expect("the piloted session verifies");
    assert_eq!(verified.ticks, report.ticks);
    assert_eq!(verified.scope, VerifyScope::Full);
    // The journal's own lines: one CapabilityGranted, for the dynamite.
    let mut lines = Vec::new();
    for entry in std::fs::read_dir(&session).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with("ticks-") && name.ends_with(".jsonl") {
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                if line.contains("\"k\":\"CapabilityGranted\"") {
                    lines.push(line.to_owned());
                }
            }
        }
    }
    let granted: usize = lines
        .iter()
        .map(|l| l.matches("\"k\":\"CapabilityGranted\"").count())
        .sum();
    assert_eq!(granted, 1, "{lines:?}");
    assert!(lines[0].contains("special_thrown_charge"), "{lines:?}");
}

/// HAZ-A2b: the same for the hazard room (its goblins take `--cf02-seed`).
#[test]
fn a_recorded_hazard_session_verifies_strict() {
    for loadout in [Loadout::Story, Loadout::All] {
        let dir = TempDir::new("f01-hazards");
        let mut rig = RoomRig::start(&dir.0, "hazards", loadout);
        assert_eq!(
            rig.combat.slot_enemies().len(),
            2,
            "S1 and S2 from the roster"
        );
        rig.walk(900);
        let manifest = rig.rec.finish("test").expect("finished");
        let session = dir.0.join(manifest.session_id);
        let report = verify(&session).unwrap_or_else(|e| panic!("{}: {e}", loadout.name()));
        assert_eq!(report.ticks, 900);
        let loaded = read_session(&session, false).unwrap();
        assert_eq!(loaded.manifest.reset.name, "f01_hazards");
        assert_scoped_stamps(&loaded, "hazards");
        // The seed is part of the run: a recording made without one does not
        // verify as seed 7.
        tamper_manifest(&session, |m| m.reset.cf02_seed = Some(7));
        assert!(verify(&session).is_err(), "a seed is another run");
    }
}

/// Review of #229: `--cf02-seed` is an accepted input of the hazard room on both
/// sides, so a seeded recording verifies strict, and the seed is part of the run
/// (dropping or changing it fails).
#[test]
fn a_seeded_hazard_session_verifies_strict() {
    let dir = TempDir::new("f01-hazards-seeded");
    let mut rig = RoomRig::start_seeded(&dir.0, "hazards", Loadout::Story, Some(7));
    rig.walk(900);
    let manifest = rig.rec.finish("test").expect("finished");
    let session = dir.0.join(manifest.session_id);
    let report = verify(&session).expect("the seeded session verifies");
    assert_eq!((report.ticks, report.scope), (900, VerifyScope::Full));
    let loaded = read_session(&session, false).unwrap();
    assert_eq!(loaded.manifest.reset.cf02_seed, Some(7));
    assert_scoped_stamps(&loaded, "hazards");
    for seed in [None, Some(8)] {
        tamper_manifest(&session, |m| m.reset.cf02_seed = seed);
        assert!(verify(&session).is_err(), "seed {seed:?} is another run");
    }
    tamper_manifest(&session, |m| m.reset.cf02_seed = Some(7));
    verify(&session).expect("restored");
}

/// PT-MASTERY: a recorded session of the closing arena verifies strict under
/// both loadouts and a seed, stamps its room pins and the manifest's and not
/// another room's, and the roster, the loadout label and the seed are part of
/// the run. The recorded run is driven by the autopilot's fight (the arena's
/// Carl pilot), so it holds hits, hurts and deaths, not only a walk.
#[test]
fn a_recorded_mastery_session_verifies_strict() {
    for (loadout, seed) in [
        (Loadout::Story, None),
        (Loadout::Story, Some(5)),
        (Loadout::All, Some(7)),
    ] {
        let dir = TempDir::new("f01-mastery");
        let mut rig = RoomRig::start_seeded(&dir.0, "mastery", loadout, seed);
        assert_eq!(rig.combat.enemies.len(), 5, "four goblins and the Engineer");
        assert_eq!(rig.combat.llamas.len(), 2, "two llamas");
        for _ in 0..1500 {
            let input = crate::cf02::arena::autopilot(
                &rig.combat,
                &rig.scene,
                crate::cf02::arena::Plan::Direct,
            );
            rig.step(input);
        }
        assert!(
            rig.combat
                .events
                .iter()
                .any(|e| matches!(e, crate::combat::CombatEvent::EnemyDefeated { .. })),
            "the recorded run fights"
        );
        let manifest = rig.rec.finish("test").expect("finished");
        let session = dir.0.join(manifest.session_id);
        let report = verify(&session).unwrap_or_else(|e| panic!("{}: {e}", loadout.name()));
        assert_eq!((report.ticks, report.scope), (1500, VerifyScope::Full));
        let loaded = read_session(&session, false).unwrap();
        assert_eq!(loaded.manifest.reset.name, "f01_mastery");
        assert_eq!(loaded.manifest.reset.f01_room.as_deref(), Some("mastery"));
        assert_scoped_stamps(&loaded, "mastery");
        // The seed and the loadout label are part of the run.
        let other_seed = seed.map_or(Some(1), |s| Some(s + 1));
        tamper_manifest(&session, |m| m.reset.cf02_seed = other_seed);
        assert!(verify(&session).is_err(), "another seed is another run");
        tamper_manifest(&session, |m| m.reset.cf02_seed = seed);
        verify(&session).expect("restored");
        let other = if loadout == Loadout::Story {
            "all"
        } else {
            "story"
        };
        tamper_manifest(&session, |m| m.reset.f01_loadout = Some(other.into()));
        assert_eq!(
            verify(&session).expect_err("relabelled").code,
            "rules_mismatch"
        );
    }
}

/// RING-A/B wire: a recorded session of each Ring room verifies strict under
/// both loadouts and a seed, stamps its own room pins and the manifest's and no
/// other room's, and the seed and the loadout label are part of the run. The
/// recorded run is driven by the rooms' move-right pilot
/// (`f01_ring_fixtures::pilot`), so it holds the walkway crossing over the
/// static shutter, hits and defeats, not only a walk.
#[test]
fn a_recorded_ring_session_verifies_strict() {
    use crate::f01_ring::{RingRoom, launch};
    use crate::f01_ring_fixtures::{Route, pilot};
    for room in RingRoom::ALL {
        let route = Route::of(&launch(&root(), room).unwrap(), false);
        for (loadout, seed) in [
            (Loadout::Story, None),
            (Loadout::Story, Some(5)),
            (Loadout::All, Some(7)),
        ] {
            let dir = TempDir::new("f01-ring");
            let mut rig = RoomRig::start_seeded(&dir.0, room.id(), loadout, seed);
            assert_eq!(rig.combat.enemies.len(), 2, "two goblins");
            for _ in 0..900 {
                let hero = rig.party.controlled_hero();
                let mut input = pilot(&route, &rig.combat, &rig.scene, hero);
                // The TEST LOADOUT holds the dynamite: the pilot never throws it.
                input.ability = false;
                rig.step(input);
            }
            let defeated = rig
                .combat
                .events
                .iter()
                .filter(|e| matches!(e, crate::combat::CombatEvent::EnemyDefeated { .. }))
                .count();
            assert_eq!(defeated, 2, "{room:?}: the recorded run clears the room");
            let manifest = rig.rec.finish("test").expect("finished");
            let session = dir.0.join(manifest.session_id);
            let report = verify(&session).unwrap_or_else(|e| panic!("{room:?}: {e}"));
            assert_eq!(report.scope, VerifyScope::Full);
            let loaded = read_session(&session, false).unwrap();
            assert_eq!(loaded.manifest.reset.name, room.scene_name());
            assert_eq!(loaded.manifest.reset.f01_room.as_deref(), Some(room.id()));
            assert_scoped_stamps(&loaded, room.id());
            let other_seed = seed.map_or(Some(1), |s| Some(s + 1));
            tamper_manifest(&session, |m| m.reset.cf02_seed = other_seed);
            assert!(verify(&session).is_err(), "another seed is another run");
            tamper_manifest(&session, |m| m.reset.cf02_seed = seed);
            verify(&session).expect("restored");
            // The other Ring room's label is another run.
            let other = RingRoom::ALL.into_iter().find(|r| *r != room).unwrap();
            tamper_manifest(&session, |m| m.reset.f01_room = Some(other.id().into()));
            assert!(verify(&session).is_err(), "{room:?} relabelled {other:?}");
        }
    }
}

/// CORRAL-WIRE/CORRAL-F: a recorded room-0 session (launched by id) verifies
/// strict from its descriptor, under both loadouts and a seed; its llamas are
/// the CORRAL-C ones and another room's label or seed is another run.
#[test]
fn a_recorded_corral_session_verifies_strict() {
    use crate::f01_corral_fixtures::{Route, pilot};
    let route = Route::of(&crate::f01_corral::launch(&root()).unwrap());
    for (loadout, seed) in [
        (Loadout::Story, None),
        (Loadout::Story, Some(5)),
        (Loadout::All, Some(7)),
    ] {
        let dir = TempDir::new("f01-corral");
        let mut rig = RoomRig::start_seeded(&dir.0, "corral", loadout, seed);
        assert_eq!(rig.combat.enemies.len(), 2, "M0's two goblins");
        assert_eq!(rig.combat.llamas.len(), 3, "the M1 llama, B and A");
        for _ in 0..900 {
            let hero = rig.party.controlled_hero();
            let mut input = pilot(&route, &rig.combat, &rig.scene, hero);
            input.ability = false;
            rig.step(input);
        }
        let manifest = rig.rec.finish("test").expect("finished");
        let session = dir.0.join(manifest.session_id);
        let report = verify(&session).unwrap_or_else(|e| panic!("{loadout:?}: {e}"));
        assert_eq!(report.scope, VerifyScope::Full);
        let loaded = read_session(&session, false).unwrap();
        assert_eq!(loaded.manifest.reset.name, crate::f01_corral::SCENE_NAME);
        assert_eq!(loaded.manifest.reset.f01_room.as_deref(), Some("corral"));
        assert_scoped_stamps(&loaded, "corral");
        let other_seed = seed.map_or(Some(1), |s| Some(s + 1));
        tamper_manifest(&session, |m| m.reset.cf02_seed = other_seed);
        assert!(verify(&session).is_err(), "another seed is another run");
        tamper_manifest(&session, |m| m.reset.cf02_seed = seed);
        verify(&session).expect("restored");
        tamper_manifest(&session, |m| m.reset.f01_room = Some("ring_court".into()));
        assert!(verify(&session).is_err(), "relabelled");
    }
}

/// A room's own pin is required of its recordings: removing one of the room's
/// pins, or editing its value, fails, and another room's pin never does.
#[test]
fn a_hoarder_or_hazard_recording_needs_its_room_pins_and_no_other() {
    for room in [
        "hoarder",
        "hazards",
        "mastery",
        "ring_court",
        "ring_split",
        "corral",
    ] {
        let (_dir, session) = record("f01-room-pins", room, Loadout::Story, 200);
        for key in f01_late::SCOPED_PIN_KEYS {
            let covers = f01_late::pin_applies(key, Some(room));
            // A wrong value fails exactly the pins that cover the room.
            tamper_manifest(&session, |m| {
                m.content_hashes
                    .insert(key.to_owned(), "00000000000000ff".into());
            });
            let result = verify(&session);
            assert_eq!(result.is_err(), covers, "{room}: {key}");
            if let Err(failure) = result {
                assert_eq!(failure.code, "content_mismatch", "{room} {key}: {failure}");
                assert!(failure.message.contains(key), "{room} {key}: {failure}");
            }
            // Restore the right value.
            let (_dir2, other) = record("f01-room-pins-restore", room, Loadout::Story, 200);
            let stamped = read_session(&other, false)
                .unwrap()
                .manifest
                .content_hashes
                .get(key)
                .cloned();
            tamper_manifest(&session, |m| match &stamped {
                Some(value) => {
                    m.content_hashes.insert(key.to_owned(), value.clone());
                }
                None => {
                    m.content_hashes.remove(key);
                }
            });
        }
        verify(&session).expect("restored");
    }
}

/// BOIL-E: the recorded story run holds the sidecar's four enemies (the Engineer
/// among them) and still verifies strict, so the live roster is part of the run
/// the verifier rebuilds.
#[test]
fn a_recorded_boiler_session_holds_the_live_roster_and_verifies() {
    let dir = TempDir::new("f01-boiler-roster");
    let mut rig = RoomRig::start(&dir.0, "boiler", Loadout::Story);
    assert_eq!(rig.combat.enemies.len(), 4);
    assert_eq!(
        rig.combat
            .enemies
            .iter()
            .filter(|g| g.is_engineer())
            .count(),
        1
    );
    rig.walk(600);
    let manifest = rig.rec.finish("test").expect("finished");
    let session = dir.0.join(manifest.session_id);
    assert_eq!(verify(&session).expect("verifies").ticks, 600);
}

#[test]
fn a_recorded_juicer_session_verifies_under_both_loadouts() {
    for loadout in [Loadout::Story, Loadout::All] {
        let (_dir, session) = record("f01-juicer", "juicer", loadout, 700);
        let report = verify(&session).unwrap_or_else(|e| panic!("{}: {e}", loadout.name()));
        assert_eq!(report.ticks, 700, "{}", loadout.name());
        // The loadout label is bound to the flags: the other label is rejected.
        let other = if loadout == Loadout::Story {
            "all"
        } else {
            "story"
        };
        tamper_manifest(&session, |m| m.reset.f01_loadout = Some(other.into()));
        let failure = verify(&session).expect_err("a relabelled loadout is another run");
        assert_eq!(failure.code, "rules_mismatch", "{failure}");
    }
}

#[test]
fn a_boiler_session_under_the_all_loadout_verifies() {
    let (_dir, session) = record("f01-boiler-all", "boiler", Loadout::All, 600);
    verify(&session).expect("the TEST LOADOUT session verifies");
}

#[test]
fn a_relabelled_room_is_rejected() {
    let (_dir, session) = record("f01-relabel", "boiler", Loadout::Story, 300);
    // Another listed room (the corral among them since CORRAL-F), an unlisted
    // room that does not launch, and no room at all.
    for room in [
        Some("juicer"),
        Some("hoarder"),
        Some("hazards"),
        Some("mastery"),
        Some("ring_court"),
        Some("ring_split"),
        Some("corral"),
        Some("dozer"),
        None,
    ] {
        tamper_manifest(&session, |m| m.reset.f01_room = room.map(str::to_owned));
        assert!(verify(&session).is_err(), "{room:?}");
    }
    tamper_manifest(&session, |m| m.reset.f01_room = Some("boiler".into()));
    verify(&session).expect("restored");
    // The room rules are part of the run.
    tamper_manifest(&session, |m| m.reset.cf02_rules = false);
    assert!(verify(&session).is_err());
}

#[test]
fn a_recording_that_names_a_room_needs_every_room_pin() {
    let (_dir, session) = record("f01-pins", "boiler", Loadout::Story, 300);
    for key in f01_late::SCOPED_PIN_KEYS
        .into_iter()
        .filter(|key| f01_late::pin_applies(key, Some("boiler")))
    {
        tamper_manifest(&session, |m| {
            m.content_hashes.remove(key);
        });
        let failure = verify(&session).expect_err(key);
        assert_eq!(failure.code, "content_mismatch", "{key}: {failure}");
        assert!(failure.message.contains(key), "{key}: {failure}");
        // Restore it.
        let (_dir2, other) = record("f01-pins-restore", "boiler", Loadout::Story, 300);
        let stamped = read_session(&other, false).unwrap().manifest.content_hashes[key].clone();
        tamper_manifest(&session, |m| {
            m.content_hashes.insert(key.to_owned(), stamped);
        });
    }
    // A wrong value is rejected in a recording the pin covers.
    tamper_manifest(&session, |m| {
        m.content_hashes.insert(
            f01_late::MANIFEST_PIN_KEY.to_owned(),
            "0000000000000001".into(),
        );
    });
    let failure = verify(&session).expect_err("a drifted manifest hash");
    assert!(
        failure.message.contains(f01_late::MANIFEST_PIN_KEY),
        "{failure}"
    );
}

/// A recording of a run that names no room (`--cf02`, the boss arena, every
/// recording before PT-SCENE) neither stamps nor needs the room pins, so a
/// manifest re-pin or a Boiler edit (simulated by a different recorded value)
/// never fails it (review F1).
fn demo_session(name: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new(name);
    let mut rig = DemoRig::new(&dir.0, false, 0);
    for tick in 0..300_u64 {
        rig.step(RawInput {
            move_x: if tick % 100 < 80 { 1.0 } else { 0.0 },
            ..RawInput::NONE
        });
    }
    let manifest = rig.rec.finish("test").unwrap();
    let session = dir.0.join(&manifest.session_id);
    (dir, session)
}

#[test]
fn a_recording_without_a_room_neither_stamps_nor_needs_the_room_pins() {
    let (_dir, session) = demo_session("f01-scoped");
    let loaded = read_session(&session, false).unwrap();
    for key in f01_late::SCOPED_PIN_KEYS {
        assert!(
            !loaded.manifest.content_hashes.contains_key(key),
            "{key} is not stamped"
        );
    }
    assert!(loaded.manifest.content_hashes.contains_key("cf02_tuning"));
    verify(&session).expect("with no room pin");
    // A simulated manifest re-pin and Boiler edit: the recording carries values
    // from another build for every room pin, and still verifies.
    tamper_manifest(&session, |m| {
        for key in f01_late::SCOPED_PIN_KEYS {
            m.content_hashes
                .insert(key.to_owned(), "00000000000000ff".into());
        }
    });
    verify(&session).expect("a re-pin of the room data cannot fail a run without a room");
    // Another pin is never optional.
    tamper_manifest(&session, |m| {
        m.content_hashes.remove("cf02_tuning");
    });
    assert!(verify(&session).is_err());
}

/// The stamp is scoped by room on the recorder side too: a Juicer recording
/// carries the manifest hash and not the Boiler's, and survives a Boiler edit.
#[test]
fn a_juicer_recording_carries_the_manifest_pin_and_not_the_boilers() {
    let (_dir, session) = record("f01-juicer-pins", "juicer", Loadout::Story, 300);
    let loaded = read_session(&session, false).unwrap();
    let hashes = &loaded.manifest.content_hashes;
    assert!(hashes.contains_key(f01_late::MANIFEST_PIN_KEY));
    assert!(!hashes.contains_key(crate::f01_boiler::ROOM_PIN_KEY));
    assert!(!hashes.contains_key(crate::f01_boiler::ENCOUNTER_PIN_KEY));
    verify(&session).expect("verifies");
    // A Boiler edit does not touch it; a manifest re-pin does.
    tamper_manifest(&session, |m| {
        m.content_hashes.insert(
            crate::f01_boiler::ROOM_PIN_KEY.to_owned(),
            "00000000000000ff".into(),
        );
    });
    verify(&session).expect("a Boiler edit cannot fail a Juicer recording");
    tamper_manifest(&session, |m| {
        m.content_hashes.insert(
            f01_late::MANIFEST_PIN_KEY.to_owned(),
            "00000000000000ff".into(),
        );
    });
    assert!(
        verify(&session).is_err(),
        "a manifest re-pin fails a room recording"
    );
}

/// Review F2: the room label is bound to the session source. A `demo`, `fixture`
/// or any other session cannot claim a room, so a relabelled `--cf02` session
/// never stands for a room recording.
#[test]
fn a_session_of_another_source_cannot_claim_a_room() {
    let (_dir, session) = demo_session("f01-claim");
    for (room, loadout) in [
        (Some("boiler"), None),
        (Some("hoarder"), None),
        (Some("hazards"), None),
        (Some("mastery"), None),
        (Some("ring_court"), None),
        (Some("ring_split"), None),
        (Some("corral"), None),
        (Some("juicer"), None),
        (Some("boiler"), Some("story")),
        (Some("juicer"), Some("all")),
        (None, Some("story")),
    ] {
        tamper_manifest(&session, |m| {
            m.reset.f01_room = room.map(str::to_owned);
            m.reset.f01_loadout = loadout.map(str::to_owned);
        });
        let failure = verify(&session).expect_err("a relabelled demo session");
        assert_eq!(
            failure.code, "rules_mismatch",
            "{room:?} {loadout:?}: {failure}"
        );
    }
    tamper_manifest(&session, |m| {
        m.reset.f01_room = None;
        m.reset.f01_loadout = None;
    });
    verify(&session).expect("unlabelled again");
}

#[test]
fn only_the_listed_rooms_sessions_may_carry_their_own_room() {
    let reset = |source: &str, name: &str, room: Option<&str>, loadout: Option<&str>| {
        let mut r = super::tests::reset();
        r.source = source.to_owned();
        r.name = name.to_owned();
        r.f01_room = room.map(str::to_owned);
        r.f01_loadout = loadout.map(str::to_owned);
        r
    };
    let boss_name = crate::boss_scene::BOSS01_SCENE.name;
    let ok = |r: &ResetDescriptor| super::replay::check_f01_source(r).is_ok();
    // The two genuine shapes, and every unlabelled source.
    assert!(ok(&reset(
        f01_late::SESSION_SOURCE,
        "f01_boiler",
        Some("boiler"),
        Some("story")
    )));
    assert!(ok(&reset("boss", boss_name, Some("juicer"), None)));
    // HRD-A2b and HAZ-A2b: the two rooms on their own fixtures only.
    for (room, name) in [
        ("hoarder", "f01_hoarder"),
        ("hazards", "f01_hazards"),
        ("mastery", "f01_mastery"),
        ("ring_court", "f01_ring_court"),
        ("ring_split", "f01_ring_split"),
        ("corral", "f01_corral"),
    ] {
        assert!(ok(&reset(
            f01_late::SESSION_SOURCE,
            name,
            Some(room),
            Some("story")
        )));
        assert!(!ok(&reset(
            f01_late::SESSION_SOURCE,
            "f01_boiler",
            Some(room),
            None
        )));
        assert!(!ok(&reset(
            f01_late::SESSION_SOURCE,
            name,
            Some("boiler"),
            None
        )));
        assert!(!ok(&reset("demo", name, Some(room), None)));
        assert!(!ok(&reset("boss", boss_name, Some(room), None)));
    }
    for source in ["demo", "fixture", "boss", "story", "party_fixture"] {
        assert!(ok(&reset(source, "x", None, None)), "{source} unlabelled");
    }
    // `demo` and `fixture` (and the rest) claiming either room, or only a loadout.
    for source in ["demo", "fixture", "story", "party_fixture"] {
        for room in [
            "boiler",
            "hoarder",
            "hazards",
            "mastery",
            "ring_court",
            "ring_split",
            "corral",
            "juicer",
        ] {
            assert!(
                !ok(&reset(source, "x", Some(room), None)),
                "{source} {room}"
            );
            assert!(
                !ok(&reset(source, boss_name, Some(room), Some("story"))),
                "{source} {room}"
            );
        }
        assert!(
            !ok(&reset(source, "x", None, Some("all"))),
            "{source} loadout only"
        );
    }
    // The boss source: only `juicer` on the boss scene's fixture.
    assert!(!ok(&reset("boss", boss_name, Some("boiler"), None)));
    assert!(!ok(&reset(
        "boss",
        "other_boss_fixture",
        Some("juicer"),
        None
    )));
    assert!(!ok(&reset("boss", boss_name, None, Some("story"))));
    // The `f01_room` source: only `boiler` on its own fixture.
    assert!(!ok(&reset(
        f01_late::SESSION_SOURCE,
        "f01_boiler",
        Some("juicer"),
        None
    )));
    assert!(!ok(&reset(
        f01_late::SESSION_SOURCE,
        "f01_boiler",
        None,
        None
    )));
    assert!(!ok(&reset(
        f01_late::SESSION_SOURCE,
        "demo01",
        Some("boiler"),
        None
    )));
}

/// HAZ-B evidence, not a check: record the scripted hazard-room route into the
/// directory named by `HAZB_SESSION_DIR` (so the replay viewer can draw it and
/// capture window frames), and print the ticks of the events worth a frame.
/// `cargo test -- --ignored write_the_hazard_evidence_session`.
#[test]
#[ignore = "evidence generator: needs HAZB_SESSION_DIR"]
fn write_the_hazard_evidence_session() {
    let base = std::env::var("HAZB_SESSION_DIR").expect("HAZB_SESSION_DIR");
    let mut rig = RoomRig::start(Path::new(&base), "hazards", Loadout::Story);
    // A steady walk right with a jab now and then and a hop over the pool: it
    // reaches the slime, both pockets' stickers and the acid.
    for tick in 0..1500u64 {
        rig.step(RawInput {
            move_x: 1.0,
            attack: tick % 23 == 0,
            jump: tick % 97 == 40,
            ..RawInput::NONE
        });
    }
    for e in &rig.combat.events {
        match e {
            CombatEvent::RotProvoked { tick, sticker, .. } => {
                println!("HAZB_EVENT rot_provoked tick={tick} sticker={sticker}");
            }
            CombatEvent::HeroHurt { tick, source, .. } => {
                println!("HAZB_EVENT hero_hurt tick={tick} source={source:#x}");
            }
            CombatEvent::EnemyDefeated { tick, target, .. } => {
                println!("HAZB_EVENT defeated tick={tick} target={target:#x}");
            }
            _ => {}
        }
    }
    let manifest = rig.rec.finish("test").expect("finished");
    println!("HAZB_SESSION {}", manifest.session_id);
}

/// F01-RUN: a room of a floor run starts from the ledger the run carried into it
/// (the Hoarder's, with her dynamite receipt). Its session writes the carry into the
/// descriptor and verifies strict from it, in the Boiler and in the Juicer's boss
/// scene; the same session without the carry (or with another ledger) is another
/// start state and fails, and a carry with no room is refused.
#[test]
fn a_carried_floor_run_room_verifies_strict_from_its_descriptor() {
    let snapshot = crate::f01_late_fixtures::hoarder_defeat(&root())
        .unwrap()
        .snapshot()
        .expect("a ledger");
    for room in ["boiler", "juicer"] {
        let dir = TempDir::new("f01-run-carry");
        let mut rig = RoomRig::start_carried(&dir.0, room, &snapshot);
        assert!(
            rig.combat
                .story_holds(crate::story_unlock::SPECIAL_THROWN_CHARGE)
        );
        rig.walk(300);
        let manifest = rig.rec.finish("test").expect("finished");
        let session = dir.0.join(manifest.session_id);
        let report = verify(&session).unwrap_or_else(|e| panic!("{room}: {e}"));
        assert_eq!((report.ticks, report.scope), (300, VerifyScope::Full));
        let loaded = read_session(&session, false).unwrap();
        assert_eq!(loaded.manifest.reset.f01_carry.as_ref(), Some(&snapshot));
        // Dropped: the pin-only start hashes differently (the receipt is hashed).
        tamper_manifest(&session, |m| m.reset.f01_carry = None);
        assert!(verify(&session).is_err(), "{room}: a dropped carry");
        // A carry the table does not know is refused as content.
        let mut bad = snapshot.clone();
        bad.held[0].capability = "no_such_capability".into();
        tamper_manifest(&session, |m| m.reset.f01_carry = Some(bad.clone()));
        assert_eq!(
            verify(&session).expect_err("unknown capability").code,
            "content_mismatch"
        );
        // A carry needs the story loadout (the binary refuses it under `all`).
        tamper_manifest(&session, |m| m.reset.f01_carry = Some(snapshot.clone()));
        verify(&session).unwrap_or_else(|e| panic!("{room} restored: {e}"));
        tamper_manifest(&session, |m| m.reset.f01_loadout = None);
        assert_eq!(
            verify(&session).expect_err("no loadout").code,
            "rules_mismatch"
        );
        // A carry needs a room.
        tamper_manifest(&session, |m| {
            m.reset.f01_carry = Some(snapshot.clone());
            m.reset.f01_room = None;
            m.reset.f01_loadout = None;
        });
        assert!(verify(&session).is_err(), "{room}: a carry without a room");
        tamper_manifest(&session, |m| {
            m.reset.f01_room = Some(room.into());
            m.reset.f01_loadout = Some("story".into());
        });
        verify(&session).unwrap_or_else(|e| panic!("{room} restored: {e}"));
    }
    // Absent from every older descriptor's text.
    let plain = serde_json::to_string(&super::tests::reset()).unwrap();
    assert!(!plain.contains("f01_carry"));
}
