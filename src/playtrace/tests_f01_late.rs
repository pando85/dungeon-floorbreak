//! PT-SCENE proofs: the descriptor's `f01_room` and `f01_loadout` round-trip
//! and stay out of every older descriptor, a recorded session of each listed
//! room (the Boiler and the Juicer, under both loadouts) verifies strict from
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
        let (carl, donut) = loadout.flags();
        let (source, name, cf02, scene, combat, party) = if room == "boiler" {
            let (run, scene, combat) = f01_late::boiler_start(&root(), 360, carl, donut, None)
                .expect("the Boiler launches");
            // The live Boiler party: `cf02_assist` and the 12-tick swap.
            let party = PartyState::new(
                PartyTuning {
                    uncontrolled_hero: UncontrolledHero::Companion(ProfileSetId::Cf02Assist),
                    ..PROVISIONAL_TUNING
                }
                .with_cf02_swap(),
            );
            (
                f01_late::SESSION_SOURCE,
                run.fixture.name,
                true,
                scene,
                combat,
                party,
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
        let reset = ResetDescriptor {
            source: source.to_owned(),
            name: name.to_owned(),
            demo: false,
            demo_beat: 0,
            cf02_rules: cf02,
            cf02_seed: None,
            view_height: 360,
            donut_biscuit: room == "boiler" || donut.implies_biscuit(),
            carl_prog: carl.prog,
            carl_dynamite: carl.dynamite,
            carl_gauntlet: carl.gauntlet,
            donut_motion: party.tuning().donut_motion,
            donut_prog: donut.prog,
            donut_light_feet: donut.light_feet,
            donut_puddle: donut.puddle,
            f01_room: Some(room.to_owned()),
            f01_loadout: Some(loadout.name().to_owned()),
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
    for key in f01_late::SCOPED_PIN_KEYS {
        assert!(
            loaded.manifest.content_hashes.contains_key(key),
            "{key} is stamped"
        );
    }
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
    // Another listed room, an unlisted one, and no room at all.
    for room in [Some("juicer"), Some("corral"), None] {
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
    for key in f01_late::SCOPED_PIN_KEYS {
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
    // A wrong value is rejected whether or not a room is named.
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

/// U5: a recording of a run that names no room (every recording made before the
/// rooms existed) verifies without the room pins, and one that records a
/// wrong value for any of them is still rejected.
#[test]
fn a_recording_without_a_room_verifies_without_the_room_pins() {
    let dir = TempDir::new("f01-scoped");
    let mut rig = DemoRig::new(&dir.0, false, 0);
    for tick in 0..300_u64 {
        rig.step(RawInput {
            move_x: if tick % 100 < 80 { 1.0 } else { 0.0 },
            ..RawInput::NONE
        });
    }
    let manifest = rig.rec.finish("test").unwrap();
    let session = dir.0.join(&manifest.session_id);
    verify(&session).expect("with the pins");
    tamper_manifest(&session, |m| {
        for key in f01_late::SCOPED_PIN_KEYS {
            m.content_hashes.remove(key);
        }
    });
    verify(&session).expect("an older recording has none of the room pins");
    // Another pin is never optional.
    tamper_manifest(&session, |m| {
        m.content_hashes.remove("cf02_tuning");
    });
    assert!(verify(&session).is_err());
}
