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

// ---------------------------------------------------------------------------
// PT-GATE-C2: the HAZ-C gate fixtures of the live room (HAZ-06, HAZ-09).
// ---------------------------------------------------------------------------

/// Scripted **room pilots** of the hazard room (`docs/01-design/enemy-gameplay-slime-rot.md`
/// section 18, "HAZ-C fixtures": `haz_room_direct_route` (HAZ-06) and
/// `haz_room_solo_carl`, `haz_room_solo_donut`, `haz_room_companion` (HAZ-09)).
///
/// They play the **shipped room** (`--scene f01_late --f01-room hazards`: the
/// story loadout, the sidecar's two slimes, two goblins and two Rot Stickers, its
/// exit gate) with the PT-TUNE pilot ([`crate::f01_mastery_fixtures::pilot`]),
/// which baits and punishes the slime, fights goblins with jabs and the dive,
/// hops solids, never walks into live acid and keeps out of an armed sticker's
/// column while a wave enemy is left. It is melee only: it never presses the
/// ability or the cast. The party is the library fixtures' (`cf02_assist` for
/// the solo runs, `cf02_free` for the companion run).
///
/// A pilot is **not** AUTOPLAY and not an evaluator of fun: it proves that a
/// reasonable scripted route clears the room (every slime and goblin defeated,
/// the exit reached), that no hero is stuck for more than 300 ticks, and that each
/// run replays to the same state hash. HAZ-Q45 is **report-only**: the AI hero's
/// hurts are counted by source ([`HurtSource`]) and its Rot provokes by cause.
pub mod pilots {
    use std::path::Path;

    use crate::combat::CombatEvent;
    use crate::combat_scene::{CombatState, EnvRuntime};
    use crate::corridor::SceneSim;
    use crate::donut_mana::DonutSetup;
    use crate::encounter::PropKind;
    use crate::f01_hazards;
    use crate::f01_late::{Loadout, hazards_start};
    use crate::f01_mastery_fixtures::{APART_PX, RUN_TICKS, SWAP_TICK, Who as MWho, pilot, tuning};
    use crate::input::RawInput;
    use crate::party::HeroId;
    use crate::party_scene::{PartyFixture, PartyFixtureSim};
    use crate::rot_sticker::{ROT_SOURCE_ID, RotCause};
    use crate::scene_enemy::{SlotRoom, spawn_from_roster};
    use crate::slime::{ACID_SOURCE_ID, SLIME_SOURCE_ID, SlimePhase};

    /// A walking hero that does not move for this many ticks is stuck: the gate
    /// allows at most 300 (plan section 10, item 5).
    pub const STUCK_LIMIT: u32 = 300;
    /// The seeds every fixture covers (`--cf02-seed`).
    pub const SEEDS: std::ops::RangeInclusive<u64> = 0..=8;

    /// Who plays.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Who {
        /// Solo Carl; Donut is the `cf02_assist` partner.
        CarlSolo,
        /// Solo Donut **by swap** (the mode of BOIL-09 and HAZ-09): Carl clears
        /// the roster, the pilot swaps on the tick after the clear, and Donut
        /// walks to the exit.
        DonutBySwap,
        /// The harder probe: the swap on [`SWAP_TICK`], then Donut plays the
        /// whole room (see the PR's finding).
        DonutFights,
        /// Carl is the pilot's; Donut is the autonomous `cf02_free` partner.
        Companion,
    }

    impl Who {
        /// The three HAZ-09 modes (`haz_room_solo_carl`, `haz_room_solo_donut`,
        /// `haz_room_companion`).
        pub const ALL: [Who; 3] = [Who::CarlSolo, Who::DonutBySwap, Who::Companion];

        pub const fn name(self) -> &'static str {
            match self {
                Self::CarlSolo => "haz_room_solo_carl",
                Self::DonutBySwap => "haz_room_solo_donut",
                Self::DonutFights => "haz_room_donut_fights",
                Self::Companion => "haz_room_companion",
            }
        }

        fn mastery(self) -> MWho {
            match self {
                Self::CarlSolo => MWho::CarlSolo,
                Self::DonutBySwap => MWho::DonutBySwap,
                Self::DonutFights => MWho::DonutFights,
                Self::Companion => MWho::Companion,
            }
        }
    }

    /// What hurt a hero (HAZ-Q45's four sources, plus anything else).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HurtSource {
        Acid,
        /// The slime's lunge: a hit on an impact tick of its attack.
        SlimeLunge,
        /// The slime's corrosive body outside the lunge's impact.
        SlimeBody,
        Rot,
        /// A goblin (every other source).
        Goblin,
    }

    /// One hero hurt, classified.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Hurt {
        pub tick: u64,
        pub hero: HeroId,
        pub source: HurtSource,
        /// The hurt hero was the AI's (not the controlled one) on that tick.
        pub ai: bool,
    }

    /// One Rot provoke, with the hero nearer to the sticker's dome on that tick
    /// (a solo run credits every provoke to player 0, so the event alone cannot
    /// tell the AI hero's from the pilot's).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Provoke {
        pub tick: u64,
        pub cause: RotCause,
        pub nearer: HeroId,
        /// The nearer hero was the AI's on that tick.
        pub ai: bool,
    }

    /// What a pilot run saw.
    #[derive(Debug, Clone)]
    pub struct Report {
        pub who: Who,
        pub seed: u64,
        pub rot_enabled: bool,
        /// The tick the last slime and goblin were defeated on (the exit opens).
        pub cleared_tick: Option<u64>,
        /// The tick the scene completed (a hero reached the exit) after the clear.
        pub exit_tick: Option<u64>,
        pub max_stuck: [u32; 2],
        /// Checkpoint restarts (a defeat of the controlled hero).
        pub deaths: u32,
        pub hash: u64,
        pub ticks: u64,
        pub hurts: Vec<Hurt>,
        pub provokes: Vec<Provoke>,
        pub events: Vec<CombatEvent>,
    }

    impl Report {
        /// Hurts of the AI hero (the one the pilot does not steer on that tick).
        pub fn ai_hurts(&self) -> Vec<Hurt> {
            self.hurts.iter().copied().filter(|h| h.ai).collect()
        }

        /// The AI hero's hurts by `source`.
        pub fn ai_hurts_by(&self, source: HurtSource) -> usize {
            self.ai_hurts()
                .iter()
                .filter(|h| h.source == source)
                .count()
        }

        /// HAZ-Q45's measure: the AI hero's acid, slime and Rot hurts (not a
        /// goblin's), per clear.
        pub fn ai_hazard_hurts(&self) -> usize {
            self.ai_hurts()
                .iter()
                .filter(|h| h.source != HurtSource::Goblin)
                .count()
        }

        /// The Rot provokes whose nearer hero is the AI hero.
        pub fn ai_provokes(&self) -> usize {
            self.provokes.iter().filter(|p| p.ai).count()
        }

        pub fn rot_blasts_hurting_anyone(&self) -> usize {
            self.hurts
                .iter()
                .filter(|h| h.source == HurtSource::Rot)
                .count()
        }

        /// The table line of the PR.
        pub fn line(&self) -> String {
            let by = |s| self.ai_hurts_by(s);
            format!(
                "{} rot={} seed={} cleared={:?} exit={:?} stuck={:?} deaths={} hurts={} \
                 ai_hurts(acid={} lunge={} body={} rot={} goblin={}) provokes={} ai_provokes={} hash={:#018x}",
                self.who.name(),
                self.rot_enabled,
                self.seed,
                self.cleared_tick,
                self.exit_tick,
                self.max_stuck,
                self.deaths,
                self.hurts.len(),
                by(HurtSource::Acid),
                by(HurtSource::SlimeLunge),
                by(HurtSource::SlimeBody),
                by(HurtSource::Rot),
                by(HurtSource::Goblin),
                self.provokes.len(),
                self.ai_provokes(),
                self.hash
            )
        }
    }

    /// The combat state of the room with the Rot kind **removed**, the way the
    /// kill switch `hazq02_rot_enabled` false builds it: no sticker exists, the
    /// soffits stay. (The switch is read from one process-wide static
    /// ([`crate::rot_sticker::active`]), so a fixture cannot flip it; this
    /// builds the same env: the sidecar without its sticker props, the soffit
    /// solids of the full sidecar.) Everything else is
    /// [`f01_hazards::combat_for`] under the story flags.
    fn combat_without_stickers(
        run: &f01_hazards::HazardsRun,
        scene: &mut SceneSim,
        seed: u64,
    ) -> Result<CombatState, String> {
        let (carl, donut) = Loadout::Story.flags();
        let room = SlotRoom {
            floor_y: scene.world.hero_foot_y(),
            walls: (0.0, run.kit.width as f32),
        };
        let spawn =
            spawn_from_roster(&run.loaded.encounter, room, false).map_err(|e| e.to_string())?;
        let setup = crate::f01_boiler::story_setup(&run.loaded.encounter.story_pin, carl, donut)?;
        let mut enc = run.loaded.encounter.clone();
        enc.props.retain(|p| p.kind != PropKind::RotSticker);
        let env = EnvRuntime::new(&enc)?.in_room(
            run.loaded.room.floor_y,
            &f01_hazards::soffits(&run.loaded),
            Some(enc.exit),
        );
        let mut combat = CombatState::new(run.fixture, &run.kit, scene)?
            .with_donut(DonutSetup::unlocked(8))
            .try_with_story(setup)
            .map_err(|e| e.to_string())?
            .with_demo(run.rules)
            .with_cf02()
            .with_cf02_seed(Some(seed))
            .with_env(env)
            .with_slot_enemies(spawn.slot)
            .map_err(|e| e.to_string())?;
        combat.sync_env(scene);
        Ok(combat)
    }

    /// A goblin whose centre is this close (px) to a hero's is inside the jab's
    /// dead zone (the jab starts 16 px ahead of the body): the hero steps back
    /// from it instead of swinging at nothing (the `room_pilots` rule).
    const TOO_CLOSE: f32 = 10.0;

    /// The PT-TUNE pilot plus the step back from a goblin that stands on the
    /// hero. The PT-TUNE pilot (written for the closing arena) swings at it from
    /// where the jab cannot reach, and Donut, whose approach ends closer, is hit
    /// until she is defeated; stepping back opens the jab's reach again. The step
    /// is never into a live acid patch.
    pub fn route_pilot(combat: &CombatState, scene: &SceneSim, hero: HeroId) -> RawInput {
        let body = scene.world.body(hero);
        let on_top = combat
            .enemies
            .iter()
            .filter(|g| {
                g.alive()
                    && g.ai == crate::combat::EnemyAi::Chase
                    && !matches!(
                        g.mode,
                        crate::goblin::GoblinMode::Down | crate::goblin::GoblinMode::Corpse
                    )
                    && (g.x - body.x).abs() < TOO_CLOSE
                    && (g.y - body.y).abs() < 30.0
            })
            .min_by(|a, b| (a.x - body.x).abs().total_cmp(&(b.x - body.x).abs()));
        if let Some(g) = on_top
            && body.grounded
            && !combat.hero_defeated
        {
            let away = if g.x >= body.x { -1.0 } else { 1.0 };
            let next = body.x + away * 2.0;
            let into_acid = combat.env().is_some_and(|env| {
                env.acid().patches().any(|p| {
                    p.bounds().is_some_and(|(x0, x1)| {
                        let inside = |x: f32| {
                            x > x0 as f32 - crate::prototype::HERO_HALF_WIDTH
                                && x < x1 as f32 + crate::prototype::HERO_HALF_WIDTH
                        };
                        inside(next) && !inside(body.x)
                    })
                })
            });
            if !into_acid {
                return RawInput {
                    move_x: away,
                    ..RawInput::NONE
                };
            }
        }
        pilot(combat, scene, hero)
    }

    /// The slimes' attack phases, by roster index.
    fn slime_phases(combat: &CombatState) -> Vec<(u8, SlimePhase)> {
        let t = crate::slime::active();
        combat
            .slot_enemies()
            .iter()
            .filter_map(|e| {
                e.as_any()
                    .and_then(|a| a.downcast_ref::<crate::combat_scene::slime::SlimeEnemy>())
            })
            .map(|s| (s.slime().index, s.slime().phase(t)))
            .collect()
    }

    /// The room built the way the scene builds it, under the party of `who`:
    /// the sim, and the number of Rot Stickers in its env.
    pub fn start(
        root: &Path,
        who: Who,
        seed: u64,
        rot_enabled: bool,
    ) -> Result<(PartyFixtureSim, usize), String> {
        let (carl, donut) = Loadout::Story.flags();
        let (run, scene, combat) = if rot_enabled {
            hazards_start(root, 360, carl, donut, Some(seed))?
        } else {
            let run = f01_hazards::launch(root)?;
            let mut scene = SceneSim::new_at(&run.kit, 360, run.fixture.carl_x)?;
            let combat = combat_without_stickers(&run, &mut scene, seed)?;
            (run, scene, combat)
        };
        let stickers = combat.env().map_or(0, |e| e.sticker_count());
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base: run.fixture,
            two_player: false,
            tuning: tuning(who.mastery()),
            script: &[],
            pilot: crate::boss_scene::Pilot::None,
            donut: DonutSetup::unlocked(8),
        }));
        Ok((
            PartyFixtureSim::from_parts(scene, combat, fixture),
            stickers,
        ))
    }

    /// Run one pilot for [`RUN_TICKS`] (or until the exit is reached).
    pub fn run(root: &Path, who: Who, seed: u64, rot_enabled: bool) -> Result<Report, String> {
        let (mut sim, _) = start(root, who, seed, rot_enabled)?;
        let rt = crate::rot_sticker::active();
        let mut last_x = [sim.scene.world.hero.x, sim.scene.world.donut_body.x];
        let mut stuck = [0u32; 2];
        let mut max_stuck = [0u32; 2];
        let mut cleared_tick = None;
        let mut exit_tick = None;
        let mut deaths = 0u32;
        let mut last_restarts = sim.combat.restarts;
        let mut swap_pressed = false;
        let mut seen = 0usize;
        let mut hurts = Vec::new();
        let mut provokes = Vec::new();
        for _ in 0..RUN_TICKS {
            let tick = sim.scene.world.tick + 1;
            let controlled = sim.party.controlled_hero();
            let (slots, walking) = match who {
                Who::CarlSolo | Who::Companion => {
                    let input = route_pilot(&sim.combat, &sim.scene, HeroId::Carl);
                    ([input, RawInput::NONE], [input.move_x != 0.0, false])
                }
                Who::DonutBySwap | Who::DonutFights => {
                    // The swap is pressed once: on `SWAP_TICK` for the fighter,
                    // the tick after the clear for the walker. The pilot then
                    // plays whoever is controlled (a restart returns control to
                    // Carl).
                    let due = if who == Who::DonutFights {
                        tick >= SWAP_TICK
                    } else {
                        sim.combat.roster_cleared()
                    };
                    let mut input = route_pilot(&sim.combat, &sim.scene, controlled);
                    if due && !swap_pressed && controlled == HeroId::Carl {
                        input.swap = true;
                        swap_pressed = true;
                    }
                    let mut walking = [false; 2];
                    let at = HeroId::ALL.iter().position(|h| *h == controlled);
                    walking[at.unwrap_or(0)] = input.move_x != 0.0;
                    ([input, RawInput::NONE], walking)
                }
            };
            let before = slime_phases(&sim.combat);
            sim.step_slots(slots);
            let after = slime_phases(&sim.combat);
            let controlled_after = sim.party.controlled_hero();
            let carl_x = sim.scene.world.hero.x;
            let donut_x = sim.scene.world.donut_body.x;
            for e in &sim.combat.events[seen..] {
                match *e {
                    CombatEvent::HeroHurt {
                        tick, source, hero, ..
                    } => {
                        let source = match source & 0xffff {
                            SLIME_SOURCE_ID => {
                                let index = (source >> 16) as u8;
                                let impact = |v: &[(u8, SlimePhase)]| {
                                    v.iter()
                                        .any(|(i, p)| *i == index && *p == SlimePhase::Impact)
                                };
                                if impact(&before) || impact(&after) {
                                    HurtSource::SlimeLunge
                                } else {
                                    HurtSource::SlimeBody
                                }
                            }
                            ACID_SOURCE_ID => HurtSource::Acid,
                            ROT_SOURCE_ID => HurtSource::Rot,
                            _ => HurtSource::Goblin,
                        };
                        hurts.push(Hurt {
                            tick,
                            hero,
                            source,
                            ai: hero != controlled_after,
                        });
                    }
                    CombatEvent::RotProvoked {
                        tick,
                        sticker,
                        cause,
                        ..
                    } => {
                        let dome = sim
                            .combat
                            .env()
                            .and_then(|env| env.sticker(sticker))
                            .map_or(carl_x, |s| s.dome(rt).0);
                        let nearer = if (donut_x - dome).abs() < (carl_x - dome).abs() {
                            HeroId::Donut
                        } else {
                            HeroId::Carl
                        };
                        provokes.push(Provoke {
                            tick,
                            cause,
                            nearer,
                            ai: nearer != controlled_after,
                        });
                    }
                    _ => {}
                }
            }
            seen = sim.combat.events.len();
            let carl_moved = (carl_x - last_x[0]).abs() >= 0.1;
            for (i, id) in HeroId::ALL.into_iter().enumerate() {
                let x = sim.scene.world.body(id).x;
                let still = (x - last_x[i]).abs() < 0.1;
                let counts = if who == Who::Companion && id == HeroId::Donut {
                    // Left behind: she stands still, apart from Carl, while he walks on.
                    still && (x - carl_x).abs() > APART_PX && carl_moved
                } else {
                    walking[i] && still
                };
                stuck[i] = if counts { stuck[i] + 1 } else { 0 };
                max_stuck[i] = max_stuck[i].max(stuck[i]);
                last_x[i] = x;
            }
            if sim.combat.restarts != last_restarts {
                deaths += 1;
                last_restarts = sim.combat.restarts;
            }
            if cleared_tick.is_none() && sim.combat.roster_cleared() {
                cleared_tick = Some(sim.scene.world.tick);
            }
            // The scene completes when a hero reaches the exit gate's goal column,
            // which freezes its tick: that is the exit, once the roster is clear.
            if cleared_tick.is_some() && exit_tick.is_none() && sim.scene.world.complete {
                exit_tick = Some(sim.scene.world.tick);
            }
            if exit_tick.is_some() {
                break;
            }
        }
        Ok(Report {
            who,
            seed,
            rot_enabled,
            cleared_tick,
            exit_tick,
            max_stuck,
            deaths,
            hash: sim.state_hash(),
            ticks: sim.scene.world.tick,
            hurts,
            provokes,
            events: sim.combat.events.clone(),
        })
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_b;
#[cfg(test)]
pub(crate) mod tests_c;
