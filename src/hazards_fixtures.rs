//! HAZ-A2: the isolated pocket fixtures of the slime, the acid patch and the
//! Rot Sticker, run on the real combat scene (`docs/01-design/enemy-gameplay-slime-rot.md`
//! section 18, "HAZ-A2 fixtures").
//!
//! **The hazard test kit** is the hazard room's own geometry (floor 580, the
//! pocket B slab: underside 68, top 80), an edited copy of its encounter
//! (the fixture places its own acid patches, slimes and stickers, so no pocket
//! of the shipped room is a prerequisite) and extra full solids (a pillar). It
//! is built by [`Layout::sim`]; nothing here runs in the game.
//!
//! Each fixture is a unit test in this file: it scripts the inputs, asserts
//! the outcome the sheet names and, where the sheet says "pinned boundary",
//! finds the boundary and asserts that the boundary passes and the next value
//! fails. [`run_twice`] replays a script twice and compares the state hashes.
//!
//! Deviations from the sheet's kit (HAZA2-AD9): the end walls are the room's
//! (x 16 and 1136, not 1184); K1 is the room's slab at 496..624 (not 560..680);
//! K2 is an extra full solid at 960..984 (height 160) added to the env
//! runtime's solids.

use std::path::PathBuf;

use crate::camera_lab::room::IRect;
use crate::combat::{CombatEvent, DefeatCause, EnemyAi};
use crate::combat_scene::{CombatState, EnemySpawn, Fixture};
use crate::corridor::SceneSim;
use crate::donut_mana::DonutSetup;
use crate::encounter::{Encounter, Prop, PropKind, RosterEntry, RosterKind, StickerLayout};
use crate::f01_hazards::{self, HazardsRun};
use crate::input::RawInput;
use crate::party::HeroId;
use crate::party_scene::{PartyFixture, PartyFixtureSim};
use crate::rot_sticker::Orientation;
use crate::scene_enemy::{SlotRoom, spawn_with};
use crate::slime::AcidPatchSpec;

/// The repository root.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// One sticker of a fixture (layout px, y down: the sidecar's convention).
#[derive(Debug, Clone, Copy)]
pub struct StickerAt {
    pub orientation: Orientation,
    pub surface: IRect,
    pub a: (i32, i32),
    pub b: (i32, i32),
    pub start: (i32, i32),
}

/// One slime of a fixture.
#[derive(Debug, Clone, Copy)]
pub struct SlimeAt {
    pub x: i32,
    pub facing: i8,
    pub home: i32,
}

/// What a fixture places in the kit.
#[derive(Debug, Clone, Default)]
pub struct Layout {
    /// `(x0, x1, the roster index of the slime it drains with)`; ids follow the
    /// order, from 1.
    pub acid: Vec<(i32, i32, Option<u8>)>,
    /// The slimes, in roster order (their roster index is their position).
    pub slimes: Vec<SlimeAt>,
    pub stickers: Vec<StickerAt>,
    pub goblins: Vec<i32>,
    /// Extra full solids (layout px) such as a pillar.
    pub solids: Vec<IRect>,
    pub carl_x: i32,
    /// Carl holds the thrown charge from tick 0 (the story ledger holds it).
    pub dynamite: bool,
    /// The CF02 morale seed (a fixture that needs a Coward draws one).
    pub seed: Option<u64>,
    /// The party tuning of the live party (CA2-C: the autonomous partner); the
    /// provisional one when `None`.
    pub tuning: Option<crate::party::PartyTuning>,
}

/// The floor line of the kit in layout px.
pub const FLOOR: i32 = 580;

/// Carl's start column of the kit unless a fixture says otherwise.
pub const CARL_X: i32 = 112;

/// A running fixture: one scene, one combat state, Carl alone (the legacy
/// single-hero drive) or the live party.
pub struct Rig {
    solo: Option<(SceneSim, CombatState)>,
    party: Option<PartyFixtureSim>,
}

fn encounter_of(run: &HazardsRun, l: &Layout) -> Encounter {
    let mut enc = run.loaded.encounter.clone();
    enc.props.clear();
    enc.roster.clear();
    enc.acid_patches.clear();
    for (i, s) in l.slimes.iter().enumerate() {
        enc.roster.push(RosterEntry {
            id: format!("s{i}"),
            kind: RosterKind::Slime,
            facing: s.facing,
            home: s.home,
            linked_pool: None,
            gate: None,
            fallback: None,
            x: s.x,
            perch: None,
            beat: "A".into(),
            trigger_x: None,
            delay_ticks: 0,
            threatens: Vec::new(),
            allow_in_hazard: Vec::new(),
            optional: false,
        });
    }
    for (i, (x0, x1, link)) in l.acid.iter().enumerate() {
        enc.acid_patches.push(AcidPatchSpec {
            id: (i + 1) as u16,
            x0: *x0,
            x1: *x1,
            floor_y: FLOOR,
            linked_slime: *link,
        });
    }
    for (i, st) in l.stickers.iter().enumerate() {
        enc.props.push(Prop {
            id: format!("r{i}"),
            kind: PropKind::RotSticker,
            target: IRect::new(0, 0, 1, 1),
            optional: true,
            solid_intact: false,
            hang_px: None,
            remnant: None,
            blast_radius_px: None,
            valve: None,
            track: Some(StickerLayout {
                orientation: st.orientation,
                surface: st.surface,
                a: st.a,
                b: st.b,
                start: st.start,
            }),
            shutter: None,
        });
    }
    enc
}

impl Layout {
    /// A ceiling sticker under the pocket B slab (underside 68) with its track.
    pub fn under_slab(x0: i32, x1: i32, start: i32) -> StickerAt {
        StickerAt {
            orientation: Orientation::Ceiling,
            surface: IRect::new(496, 500, 128, 12),
            a: (x0, FLOOR - 68),
            b: (x1, FLOOR - 68),
            start: (start, FLOOR - 68),
        }
    }

    /// The K2 pillar (960..984, height 160).
    pub fn pillar() -> IRect {
        IRect::new(960, FLOOR - 160, 24, 160)
    }

    /// A fixed wall sticker on the left face of K2, its dome at `height` above
    /// the floor.
    pub fn on_pillar(height: i32) -> StickerAt {
        StickerAt {
            // The sticker sits on the pillar's LEFT face, so the wall is on its
            // right: its face looks -x.
            orientation: Orientation::WallRight,
            surface: Self::pillar(),
            a: (960, FLOOR - height),
            b: (960, FLOOR - height),
            start: (960, FLOOR - height),
        }
    }

    /// Build the rig. `party`: `None` is Carl alone on the legacy drive; `Some(
    /// two_player)` is the live party (Donut live, her setup unlocked).
    pub fn sim(&self, party: Option<bool>) -> Rig {
        let mut run = f01_hazards::launch(&root()).expect("the hazard room launches");
        let goblins: Vec<EnemySpawn> = self
            .goblins
            .iter()
            .map(|x| EnemySpawn {
                x: *x,
                ai: EnemyAi::Chase,
            })
            .collect();
        let carl_x = if self.carl_x == 0 {
            CARL_X
        } else {
            self.carl_x
        };
        let fixture: &'static Fixture = Box::leak(Box::new(Fixture {
            name: "haz_fixture",
            summary: "HAZ-A2 isolated pocket",
            seed: 7,
            carl_x,
            enemies: Box::leak(goblins.into_boxed_slice()),
            shore_dx: None,
            boss: None,
            llamas: &[],
            engineers: &[],
            charges: &[],
            script: &[],
            ticks: 900,
        }));
        run.fixture = fixture;
        let enc = encounter_of(&run, self);
        let mut scene = SceneSim::new_at(&run.kit, 360, carl_x).expect("the kit scene");
        let room = SlotRoom {
            floor_y: scene.world.hero_foot_y(),
            walls: (0.0, run.kit.width as f32),
        };
        let spawn = spawn_with(crate::scene_enemy::SLOT_KINDS, &enc, room, false)
            .expect("the slimes spawn");
        let mut soffits = f01_hazards::soffits(&run.loaded);
        soffits.retain(|_| false);
        for st in &self.stickers {
            if !run.loaded.room.platforms.iter().any(|p| {
                IRect::new(p.rect.x, p.rect.y, p.rect.w, enc.platform_slab_px) == st.surface
            }) && !soffits.contains(&st.surface)
            {
                soffits.push(st.surface);
            }
        }
        for s in &self.solids {
            soffits.push(*s);
        }
        let env = crate::combat_scene::EnvRuntime::new(&enc)
            .expect("the env runtime")
            .in_room(FLOOR, &soffits, None);
        let donut = if party.is_some() {
            DonutSetup::unlocked(8)
        } else {
            DonutSetup::NONE
        };
        let story = if self.dynamite {
            crate::story_unlock::StorySetup::NONE.with_hold("special_thrown_charge")
        } else {
            crate::story_unlock::StorySetup::NONE
        };
        let mut combat = CombatState::new(fixture, &run.kit, &scene)
            .expect("the combat state")
            .with_donut(donut)
            .with_story(story)
            .with_demo(run.rules)
            .with_cf02()
            .with_cf02_seed(self.seed)
            .with_env(env)
            .with_slot_enemies(spawn.slot)
            .expect("the slot");
        combat.sync_env(&mut scene);
        match party {
            None => Rig {
                solo: Some((scene, combat)),
                party: None,
            },
            Some(two_player) => {
                let pf: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
                    base: fixture,
                    two_player,
                    tuning: self.tuning.unwrap_or(crate::party::PROVISIONAL_TUNING),
                    script: &[],
                    pilot: crate::boss_scene::Pilot::None,
                    donut,
                }));
                Rig {
                    solo: None,
                    party: Some(PartyFixtureSim::from_parts(scene, combat, pf)),
                }
            }
        }
    }
}

impl Rig {
    /// The real hazard room (the scene arm's own build) with the live party,
    /// Carl at the room's spawn and Donut placed by the caller.
    pub fn room(two_player: bool) -> Rig {
        Self::room_at(
            two_player,
            f01_hazards::launch(&root()).unwrap().fixture.carl_x,
        )
    }

    /// [`Self::room`] with the scene (and so the camera) started at column
    /// `x`: a fixture that stages the heroes far from the spawn starts there.
    pub fn room_at(two_player: bool, x: i32) -> Rig {
        let run = f01_hazards::launch(&root()).expect("the hazard room launches");
        let mut scene = SceneSim::new_at(&run.kit, 360, x).expect("the room scene");
        let combat = f01_hazards::combat_of(&run, &mut scene).expect("the room combat state");
        let pf: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base: run.fixture,
            two_player,
            tuning: crate::party::PROVISIONAL_TUNING,
            script: &[],
            pilot: crate::boss_scene::Pilot::None,
            donut: DonutSetup::unlocked(8),
        }));
        Rig {
            solo: None,
            party: Some(PartyFixtureSim::from_parts(scene, combat, pf)),
        }
    }

    /// The scene and the combat state together (a test calls an API that needs
    /// both).
    pub fn parts(&mut self) -> (&mut SceneSim, &mut CombatState) {
        match (self.party.as_mut(), self.solo.as_mut()) {
            (Some(p), _) => (&mut p.scene, &mut p.combat),
            (None, Some((scene, combat))) => (scene, combat),
            (None, None) => unreachable!("a rig has a driver"),
        }
    }

    /// A checkpoint respawn of the combat state (DEMO-01 rules).
    pub fn respawn(&mut self) {
        match (self.party.as_mut(), self.solo.as_mut()) {
            (Some(p), _) => p.combat.respawn(&mut p.scene),
            (None, Some((scene, combat))) => combat.respawn(scene),
            (None, None) => unreachable!("a rig has a driver"),
        }
    }

    /// A fixture restart of the attempt.
    pub fn restart(&mut self) {
        match (self.party.as_mut(), self.solo.as_mut()) {
            (Some(p), _) => p.combat.restart(&mut p.scene),
            (None, Some((scene, combat))) => combat.restart(scene),
            (None, None) => unreachable!("a rig has a driver"),
        }
    }

    /// One tick. `donut` is read only with the live party.
    pub fn step(&mut self, carl: RawInput, donut: RawInput) {
        match (self.party.as_mut(), self.solo.as_mut()) {
            (Some(p), _) => p.step_slots([carl, donut]),
            (None, Some((scene, combat))) => combat.step(scene, carl),
            (None, None) => unreachable!("a rig has a driver"),
        }
    }

    pub fn carl(&mut self, raw: RawInput) {
        self.step(raw, RawInput::default());
    }

    pub fn idle(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.carl(RawInput::NONE);
        }
    }

    pub fn scene(&self) -> &SceneSim {
        match (&self.party, &self.solo) {
            (Some(p), _) => &p.scene,
            (None, Some((s, _))) => s,
            (None, None) => unreachable!("a rig has a driver"),
        }
    }

    pub fn scene_mut(&mut self) -> &mut SceneSim {
        match (self.party.as_mut(), self.solo.as_mut()) {
            (Some(p), _) => &mut p.scene,
            (None, Some((s, _))) => s,
            (None, None) => unreachable!("a rig has a driver"),
        }
    }

    pub fn combat(&self) -> &CombatState {
        match (&self.party, &self.solo) {
            (Some(p), _) => &p.combat,
            (None, Some((_, c))) => c,
            (None, None) => unreachable!("a rig has a driver"),
        }
    }

    pub fn combat_mut(&mut self) -> &mut CombatState {
        match (self.party.as_mut(), self.solo.as_mut()) {
            (Some(p), _) => &mut p.combat,
            (None, Some((_, c))) => c,
            (None, None) => unreachable!("a rig has a driver"),
        }
    }

    pub fn tick(&self) -> u64 {
        self.scene().world.tick
    }

    pub fn place(&mut self, hero: HeroId, x: f32) {
        let body = self.scene_mut().world.body_mut(hero);
        body.x = x;
    }

    /// Defeat the slime of roster index `roster` through the env-hit path (the
    /// real defeat: its linked pool starts to drain).
    pub fn kill_slime(&mut self, roster: usize) {
        use crate::scene_enemy::{EnvSource, SlotEnvHit};
        let hit = SlotEnvHit {
            victim: crate::combat_scene::slime::target_id(roster),
            source: EnvSource::Prop(0),
            class: crate::env_hazard::HazardClass::FallingMass,
            damage: 9,
            push_x: 0,
            initiator: Some(0),
            activation: 1,
        };
        let tick = self.tick();
        let (scene, combat) = self.parts();
        combat.slot_env_hits(scene, &[hit], tick);
    }

    pub fn x_of(&self, hero: HeroId) -> f32 {
        self.scene().world.body(hero).x
    }

    pub fn events(&self) -> Vec<CombatEvent> {
        self.combat().events.clone()
    }

    pub fn hash(&self) -> u64 {
        self.combat().state_hash(self.scene())
    }

    /// `(tick, source, health_left)` of every hurt of `hero`.
    pub fn hurts(&self, hero: HeroId) -> Vec<(u64, u32, u8)> {
        self.events()
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::HeroHurt {
                    tick,
                    source,
                    health_left,
                    hero: h,
                } if h == hero => Some((tick, source, health_left)),
                _ => None,
            })
            .collect()
    }

    pub fn defeats(&self) -> Vec<(u64, u32, DefeatCause)> {
        self.events()
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::EnemyDefeated {
                    tick,
                    target,
                    cause,
                    ..
                } => Some((tick, target, cause)),
                _ => None,
            })
            .collect()
    }

    pub fn provokes(&self) -> Vec<CombatEvent> {
        self.events()
            .into_iter()
            .filter(|e| matches!(e, CombatEvent::RotProvoked { .. }))
            .collect()
    }

    pub fn blasts(&self) -> Vec<(u64, u32)> {
        self.events()
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::Blast { tick, charge } => Some((tick, charge)),
                _ => None,
            })
            .collect()
    }
}

/// Run `script` (one input per tick) twice on fresh rigs of `layout` and return
/// the two final state hashes: they must be equal.
pub fn run_twice(layout: &Layout, party: Option<bool>, script: &[RawInput]) -> (u64, u64) {
    let run = || {
        let mut rig = layout.sim(party);
        for raw in script {
            rig.carl(*raw);
        }
        rig.hash()
    };
    (run(), run())
}

/// Hold `move_x` for a tick.
pub const fn walk(dir: f32) -> RawInput {
    RawInput {
        move_x: dir,
        ..RawInput::NONE
    }
}

pub const JAB: RawInput = RawInput {
    attack: true,
    ..RawInput::NONE
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_b;
