//! BOIL-B wire and BOIL-C wire: the env step of the combat scene (the Goblin
//! Boiler's valve and steam, the hanging weight and its remnant).
//!
//! [`EnvRuntime`] is the Bevy-free domain half: the prop machines
//! ([`Valve`], [`FallingMass`]) and the hazard set built from the encounter
//! sidecar, the stimulus intake, one [`EnvRuntime::step`] per tick and the
//! state hash. The `impl CombatState` half wires it to the scene: Carl's melee
//! and Donut's Magic Missile as stimuli, the victim bodies (heroes, goblins,
//! llamas, slot enemies), the hits they take, the **runtime env solids sync**
//! and the slot enemies' read access ([`EnvView`]).
//!
//! **Tick slot** (integration plan section 3): the env step runs after the
//! charges and before the morale. Order inside it: the valve and the hazard
//! windows advance, the falling mass advances (its one landing evaluation),
//! the steam resolves against the bodies, then the lifts, the hits and the
//! solid sync apply. Stimuli land earlier in the tick (Carl's strike and Donut's
//! bolts run before the scene step), so a hit on tick `T` already counts one
//! warning tick on `T`: a brace released on `R` lands on `R + 31`.
//!
//! **Damage authority.** The falling mass' landing evaluation is the only
//! damage source of a weight. The sidecar's `weight_fall` volume is a data
//! source only (it is in the hazard set and never triggered), see
//! [`FallingMass`]. The valve and its steam share one timeline: the steam
//! volume is triggered by the valve's trigger and its windows equal the valve's
//! warning and vent (asserted by a test).
//!
//! **Runtime env solids (KR6, "sync, not a provider").** The first env step
//! appends the whole fixed set of the sidecar's solids (the weight's remnant,
//! the intact drum) to `scene.world.solids` as `Material::Timber` blocks, once
//! and idempotently, and rewrites each block's `intact` from the prop state on
//! every step. The index range is [`EnvRuntime::solid_range`]; every consumer
//! reads the solids unchanged (party collision, the goblin `wall_stop`, globs,
//! the missile, `NavDriver::sync`). Timber is the one destructible material, so
//! the nav graph updates incrementally after the single rebuild on the first
//! tick. The range is exposed to CP-C's structure hook and to the slot enemies
//! through [`EnvView`] (see AD8 below for how the hook stays off it).
//!
//! Adopted defaults of this step (question ids `BOILBC-AD*`, listed in the PR):
//!
//! * AD1 the hanging mass is as tall as its remnant and as wide as its
//!   footprint, its bottom `hang_px` above the floor (the sidecar has no mass
//!   height);
//! * AD2 a counted melee hit freezes Carl for the move's own hit-stop and kicks
//!   the camera, as a hit on an enemy does; a stimulus the prop ignores does
//!   not;
//! * AD3 only a prop that can still react is a target: the valve until it is
//!   set off, the support until it let go (a used prop is passed through);
//! * AD4 a scalded goblin takes the steam's damage and staggers (the existing
//!   contact stagger) pushed `push_x`, a lethal hit crushes it in place;
//! * AD5 a body that moved more than [`TELEPORT_PX`] in one tick is treated as
//!   having teleported (`prev == cur`, PR #200 review NB-1);
//! * AD6 the steam's blow on a hero carries a small hop of [`STEAM_HOP_PX_S`]
//!   so the data's `push_x` can drift the hero (the hurt path applies the
//!   horizontal knock only with a vertical one);
//! * AD7 Donut's melee (swat, pounce) is not a prop stimulus in this step: her
//!   Magic Missile is (DV-5). Launched bodies and blasts are BOIL-D's, through
//!   [`CombatState::env_stimulate`].
//! * AD8 the drum's solid is part of the synced set from the first step (intact
//!   until BOIL-D adds its detonation), because the validator's route graphs and
//!   the K8 physics check already model it; the Boiler smoke script jumps it.
//! * CP-Q35 / KC17: CP-C's `breaks_structures` blast reaches only the shore and
//!   the rack strut (each a `BlastTarget`), never the env solids; a test pins it.
//!   A prop is its own target: BOIL-D adds the `BlastTarget` impl for props.

use std::ops::Range;

use super::{Attacker, CombatState, Confirm, HeroId, MoveTick, Phase, SceneSim, hit_box};
use crate::blast::{BLAST_RADIUS, BlastBreak, BlastCharge, BlastProfile, BlastTarget, blast_break};
use crate::boss::HeroBlow;
use crate::camera_lab::room::EnemyKind;
use crate::camera_lab::room::IRect;
use crate::combat::{CombatEvent, DefeatCause, MoveId, PropPhase};
use crate::donut_moves::DONUT_HERO;
use crate::encounter::LinkKind;
use crate::encounter::{Encounter, Goal, LockEvent, PropKind, RosterKind, StickerLayout};
use crate::env_hazard::{HazardClass, HazardEdge, HazardSet, NavPhase, NavRect, Team, Victim};
use crate::env_prop::{
    Drum, DrumConfig, DrumState, FallingMass, FallingMassConfig, LinkEmit, LinkGraph, MassBody,
    MassEdge, MassState, Offer, STEP_OVER_PX, Stimulus, Valve, ValveEdge, ValveState,
};
use crate::goblin::{GoblinMode, WalkBarrier};
use crate::hero_damage::HeroDamagePolicy;
use crate::llama::LlamaMode;
use crate::prototype::{Block, HERO_HALF_HEIGHT, Material, Rect, TICK_SECONDS};
use crate::rot_sticker::{self, RotSticker, StickerCtx, StickerState};
use crate::scene_enemy::{EnvSource, EnvTarget, EnvView, SlotEnvHit};
use crate::slime::{self, AcidSet};

/// The `HeroBlow.source` of every environmental blow (a falling mass, steam):
/// `"EN"`. One constant for the whole module, distinct from the others (the
/// `every_hero_hurt_source_id_is_distinct` test lists it).
pub const ENV_SOURCE_ID: u32 = 0x454e;
/// `MoveRunner::record_hit` ids of the props: far above every goblin, llama
/// and slot id, plus the prop index.
const ENV_HIT_ID_BASE: u32 = 0x0100_0000;
/// BOILBC-AD5: a body that moved further than this in one tick teleported.
pub const TELEPORT_PX: i32 = 48;
/// BOILBC-AD6: the hop a steam blow gives a hero (px/s up).
pub const STEAM_HOP_PX_S: f32 = 80.0;

// ------------------------------------------------------------------ domain

#[derive(Debug, Clone)]
enum Machine {
    /// A valve and the id of the steam volume it sets off.
    Valve { valve: Valve, steam: u16 },
    /// A hanging weight (boxed: it is the large one).
    Mass(Box<FallingMass>),
    /// A fuel drum (BOIL-D): its acceptance machine and the blast it plans.
    Drum { drum: Drum, plan: DrumPlan },
    /// HAZ-A2: a Rot Sticker; its state lives in [`EnvRuntime::stickers`]
    /// (absent while `hazq02_rot_enabled` is false).
    Rot,
}

/// What a drum's blast is, from the sidecar (BOIL-D). The blast itself is the
/// drum's [`BlastCharge`] (one explosion engine, no second owner field); the
/// `drum_blast` hazard entry of the sidecar is a **data source only**, like the
/// weight's `weight_fall`: it is never triggered as a damage volume, or a hero
/// would be hurt twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrumPlan {
    /// Layout px of the drum's column (the blast column).
    pub x: i32,
    /// The outer radius, from the prop's `blast_radius_px`.
    pub radius: i32,
    /// Damage to a hurt hero (the hazard's `damage`).
    pub hero_hp: u8,
    /// Who the blast hurts (the hazard's `hero_damage`).
    pub hero_damage: HeroDamagePolicy,
    /// Ticks from the trigger to the blast (the hazard's telegraph).
    pub telegraph_ticks: u16,
    /// The area the tell shows (the hazard's telegraph rectangle).
    pub telegraph_rect: IRect,
}

impl DrumPlan {
    /// BOILD-AD4: the drum's blast profile. The core is the DEATH-D ring
    /// ([`BLAST_RADIUS`]), the outer band the sidecar's radius; the outer
    /// damage, launch and hero knock are the dynamite's own numbers
    /// (`carl_prog_v0.json` CP-Q21, CP-Q23), so a goblin thrown by a drum
    /// flies like one thrown by dynamite. Heroes take the hazard's damage under
    /// its policy (`AllHeroes`: the drum hurts everyone), logged with
    /// [`ENV_SOURCE_ID`]. Line of sight is on (CP-Q22). It does **not** break
    /// structures: the drum reaches props only along its authored `blast` links.
    pub fn profile(&self) -> BlastProfile {
        BlastProfile {
            core_radius: BLAST_RADIUS,
            outer_radius: self.radius as f32,
            outer_damage: 1,
            llama_outer_damage: 1,
            boss_damage: 0,
            launch: (200.0, 220.0),
            los: true,
            hero_damage: self.hero_damage,
            hero_hp: self.hero_hp,
            hero_knock: (120.0, 160.0),
            hero_source: ENV_SOURCE_ID,
            breaks_structures: false,
        }
    }
}

#[derive(Debug, Clone)]
struct EnvProp {
    /// The index in the encounter sidecar.
    index: u16,
    /// Where a stimulus hits the prop (layout px, y down).
    target: IRect,
    machine: Machine,
}

/// What a runtime env solid is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvSolidKind {
    /// The landed weight's remnant: a solid from the landing step on.
    Remnant,
    /// A drum that has not blown.
    Drum,
    /// HRD-A2: a sidecar lock's solid ([`crate::encounter::Lock::solid`]): it
    /// stands while the slot enemy `target` (the roster entry the lock waits on)
    /// is alive and is gone from the tick of its defeat. `EnvSolid::prop` is the
    /// lock's index in the sidecar. The state is read from the slot enemy, so a
    /// respawn (the defeated stay defeated) and a restart (all back) need no
    /// latch of their own.
    Lock { target: u32 },
    /// HRD-A2: a slot enemy's entry gate (HRD-Q41, boss data): it stands while
    /// the slot enemy `target` reports [`crate::scene_enemy::SlotEnemy::gate_solid`].
    /// `EnvSolid::prop` is 0.
    Gate { target: u32 },
    /// HAZ-A2: a full-height soffit a Rot Sticker clings to (no standable top).
    /// Always intact.
    Soffit,
    /// HAZ-A2: the exit of a `defeat_roster` room: it stands until every roster
    /// enemy is defeated (the room goal opens it at runtime, HAZ-Q39).
    ExitGate,
}

/// The slot target id of the roster entry a lock waits on. Appended to by the
/// enemy sheets: today only the Hoarder (her roster kind is `hoarder`).
fn roster_target_id(kind: RosterKind) -> Option<u32> {
    match kind {
        RosterKind::Room(EnemyKind::Hoarder) => Some(crate::pile_boss::PILE_BOSS_ID),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvSolid {
    /// Layout px, y down.
    pub rect: IRect,
    /// The prop index in the sidecar.
    pub prop: u16,
    pub kind: EnvSolidKind,
}

/// One victim of the env step (heroes and enemies, one per victim and source).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvHit {
    pub victim: Victim,
    /// The owning prop's index.
    pub prop: u16,
    pub class: HazardClass,
    /// The damage asked (255 from a landing: lethal to an ordinary body, the
    /// enemy's own profile maps it).
    pub damage: u8,
    /// Signed horizontal push, px/s.
    pub push_x: i16,
    /// Upward knock, px/s (a landing's hop; 0 for steam: [`STEAM_HOP_PX_S`]).
    pub knock_up: i16,
    /// The x the blow comes from (layout px: the centre of the volume).
    pub from_x: i32,
    pub initiator: Option<u32>,
    /// 1-based activation token of the volume (1 for a landing).
    pub activation: u8,
}

/// A hero lifted onto a remnant that appeared under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvLift {
    pub hero: u32,
    pub up_px: i32,
}

/// What one env tick produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvTick {
    pub events: Vec<CombatEvent>,
    pub hits: Vec<EnvHit>,
    pub lifts: Vec<EnvLift>,
    /// Non-hero bodies inside a remnant they cannot step over (NB-3).
    pub embedded: Vec<Victim>,
    /// The link contacts the edges of this tick hand on (BOIL-D), in prop
    /// order.
    pub fires: Vec<LinkFire>,
}

/// A prop edge an authored link carries to the props it reaches (BOIL-D).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkFire {
    /// The prop the edge belongs to.
    pub from: u16,
    pub emit: LinkEmit,
    /// The player credited with what the source did.
    pub initiator: Option<u32>,
    /// The chain depth the source was set off at (the contact is one deeper).
    pub depth: u8,
}

/// HAZ-A2: one Rot Sticker of the room.
#[derive(Debug, Clone)]
struct StickerRt {
    /// The prop index in the sidecar.
    prop: u16,
    layout: StickerLayout,
    /// Built by [`EnvRuntime::attach`] (it needs the scene's walking line).
    sticker: Option<RotSticker>,
    /// The sticker's authored charge, in `CombatState.charges`.
    charge: usize,
    /// Stimuli of this tick that reached it: Carl's melee (with the move) and
    /// Donut's bolts. Drained by the env step.
    pending: Vec<(Stimulus, Option<MoveId>)>,
}

/// The props, hazards and solids of one run of an encounter room.
#[derive(Debug, Clone)]
pub struct EnvRuntime {
    props: Vec<EnvProp>,
    hazards: HazardSet,
    pristine_hazards: HazardSet,
    /// `(hazard id, owning prop)`.
    owners: Vec<(u16, u16)>,
    solids: Vec<EnvSolid>,
    /// The authored prop-to-prop links (BOIL-D).
    links: LinkGraph,
    /// `(drum prop, index of its BlastCharge)` once the scene pushed the
    /// charges. Not hashed: the index is a function of the setup.
    drum_charges: Vec<(u16, usize)>,
    /// Where the solids start in `scene.world.solids` once synced.
    solid_start: Option<usize>,
    /// The bodies of the last tick (the swept test's start). Not hashed: it is
    /// a function of the hashed positions.
    prev: Vec<(Victim, IRect)>,
    /// HAZ-A2: the Rot Stickers, in sidecar order (empty in every other room).
    stickers: Vec<StickerRt>,
    /// HAZ-A2: the acid patches and the slime each one drains with.
    acid: AcidSet,
    /// `(patch id, linked slime's slot target id)`.
    acid_links: Vec<(u16, u32)>,
    /// HAZ-A2: the room's walking line in layout px (tracks and barriers are
    /// converted with it); set by [`Self::in_room`].
    floor_layout: Option<i32>,
    /// HAZ-A2: the exit rectangle of a `defeat_roster` room.
    defeat_roster: bool,
}

impl EnvRuntime {
    /// Build the runtime of an encounter: one machine per prop, in sidecar
    /// order. A prop kind this step does not run (a shutter, a rot sticker:
    /// RING and HAZ wire add their producers) is an **error**, never silently
    /// inert. A room with a remnant taller than the step-over height must not
    /// hold a roster kind that a landing may leave alive (NB-3).
    pub fn new(enc: &Encounter) -> Result<Self, String> {
        let mut props = Vec::new();
        let mut solids = Vec::new();
        // BOIL-D: the authored links, then what each linked prop must accept.
        let prop_u16 = |i: usize| u16::try_from(i).map_err(|_| "too many props".to_owned());
        let mut link_list = Vec::new();
        for l in &enc.links {
            let emit = match l.kind {
                LinkKind::Steam => LinkEmit::Steam,
                LinkKind::Landing => LinkEmit::Landing,
                LinkKind::Blast => LinkEmit::Blast,
            };
            link_list.push((prop_u16(l.from)?, prop_u16(l.to)?, emit));
        }
        let links = LinkGraph::new(link_list)?;
        let mut stickers: Vec<StickerRt> = Vec::new();
        for (i, p) in enc.props.iter().enumerate() {
            let index = u16::try_from(i).map_err(|_| "too many props".to_owned())?;
            let incoming = links.incoming(index);
            let machine = match p.kind {
                PropKind::Valve => {
                    let mut cfg = p
                        .valve
                        .ok_or_else(|| format!("prop {}: a valve needs its config", p.id))?;
                    if cfg.contact_weight == 0 {
                        cfg.contact_weight = linked_weight(&incoming, cfg.trigger_weight);
                    }
                    let steam = enc
                        .owned_hazard(i, HazardClass::Steam)
                        .ok_or_else(|| format!("prop {}: a valve owns a steam hazard", p.id))?;
                    Machine::Valve {
                        valve: Valve::new(cfg)?,
                        steam: steam.spec.id,
                    }
                }
                PropKind::WeightRig => {
                    let fall = enc
                        .owned_hazard(i, HazardClass::FallingMass)
                        .ok_or_else(|| format!("prop {}: a weight owns a falling mass", p.id))?;
                    let remnant = p
                        .remnant
                        .ok_or_else(|| format!("prop {}: a weight needs a remnant", p.id))?;
                    let hang = p
                        .hang_px
                        .ok_or_else(|| format!("prop {}: a weight needs hang_px", p.id))?;
                    let f = fall.spec.rect;
                    // BOILBC-AD1: as wide as the footprint, as tall as the
                    // remnant, its bottom `hang_px` above the floor.
                    let hanging = IRect::new(f.x, f.bottom() - hang - remnant.h, f.w, remnant.h);
                    let mut cfg = FallingMassConfig::new(hanging, f, remnant);
                    // BOILD-AD1: a linked support accepts its link.
                    if cfg.contact_weight == 0 {
                        cfg.contact_weight = linked_weight(&incoming, cfg.release_weight);
                    }
                    // The volume is a data source (see the module docs).
                    cfg.warning_ticks = fall.spec.telegraph_ticks();
                    cfg.damage = fall.spec.damage;
                    cfg.hero_damage = fall.spec.hero_damage;
                    cfg.hurts_enemies = fall.spec.hurts_enemies;
                    cfg.knock_x_px_s = fall.spec.push_x;
                    solids.push(EnvSolid {
                        rect: remnant,
                        prop: index,
                        kind: EnvSolidKind::Remnant,
                    });
                    Machine::Mass(Box::new(FallingMass::new(cfg)?))
                }
                PropKind::Drum => {
                    let blast = enc
                        .owned_hazard(i, HazardClass::Blast)
                        .ok_or_else(|| format!("prop {}: a drum owns a blast hazard", p.id))?;
                    let radius = p
                        .blast_radius_px
                        .ok_or_else(|| format!("prop {}: a drum needs blast_radius_px", p.id))?;
                    // The blast profile has no switch to spare the enemies: refuse
                    // data that asks for it rather than ignore it.
                    if !blast.spec.hurts_enemies {
                        return Err(format!(
                            "prop {}: a drum's blast hurts enemies (BlastProfile has no way to spare them)",
                            p.id
                        ));
                    }
                    if p.solid_intact {
                        solids.push(EnvSolid {
                            rect: p.target,
                            prop: index,
                            kind: EnvSolidKind::Drum,
                        });
                    }
                    let plan = DrumPlan {
                        x: p.target.x + p.target.w / 2,
                        radius,
                        hero_hp: blast.spec.damage,
                        hero_damage: blast.spec.hero_damage,
                        telegraph_ticks: blast.spec.telegraph_ticks(),
                        telegraph_rect: blast.spec.telegraph_rect,
                    };
                    let cfg = DrumConfig {
                        telegraph_ticks: plan.telegraph_ticks,
                        ..DrumConfig::DEFAULT
                    };
                    Machine::Drum {
                        drum: Drum::new(cfg)?,
                        plan,
                    }
                }
                PropKind::RotSticker => {
                    // HAZ-Q02: the data kill switch removes the kind.
                    if rot_sticker::active().enabled() {
                        let layout = p
                            .track
                            .ok_or_else(|| format!("prop {}: a sticker needs its track", p.id))?;
                        stickers.push(StickerRt {
                            prop: index,
                            layout,
                            sticker: None,
                            charge: 0,
                            pending: Vec::new(),
                        });
                    }
                    Machine::Rot
                }
                PropKind::Shutter => {
                    return Err(format!(
                        "prop {}: the env step does not run a {:?} yet (RING wire adds it)",
                        p.id, p.kind
                    ));
                }
            };
            props.push(EnvProp {
                index,
                target: p.target,
                machine,
            });
        }
        // HRD-A2: the sidecar's locks are runtime solids too (the Hoarder's exit
        // door), after the props' solids, in lock order.
        for (i, lock) in enc.locks.iter().enumerate() {
            // A new `LockEvent` variant fails to compile here: this step must say
            // what it does with it.
            let LockEvent::EnemyDefeated { roster } = lock.opens_on;
            let target = enc
                .roster
                .get(roster)
                .and_then(|e| roster_target_id(e.kind))
                .ok_or_else(|| {
                    format!(
                        "lock {}: roster entry {roster} has no slot target id",
                        lock.id
                    )
                })?;
            solids.push(EnvSolid {
                rect: lock.solid,
                prop: u16::try_from(i).map_err(|_| "too many locks".to_owned())?,
                kind: EnvSolidKind::Lock { target },
            });
        }
        // NB-3: a survivor inside a remnant over 16 px would stand in a solid.
        let tall = enc
            .props
            .iter()
            .any(|p| p.remnant.is_some_and(|r| r.h > STEP_OVER_PX));
        if tall && let Some(e) = enc.roster.iter().find(|e| e.kind.supplies_boss()) {
            return Err(format!(
                "roster {}: a slot boss may survive a landing inside a remnant over {STEP_OVER_PX} px",
                e.id
            ));
        }
        let hazards = enc.hazard_set().map_err(|e| e.to_string())?;
        let owners = enc
            .hazards
            .iter()
            .filter_map(|h| {
                h.owner
                    .map(|o| (h.spec.id, u16::try_from(o).expect("prop index")))
            })
            .collect();
        // HAZ-A2: the acid patches (the sidecar's `acid_patches`, never in
        // `hazards`) and the slime each one drains with.
        let acid =
            AcidSet::new(slime::active(), enc.acid_patches.clone()).map_err(|e| e.to_string())?;
        let acid_links = enc
            .acid_patches
            .iter()
            .filter_map(|a| {
                a.linked_slime
                    .map(|r| (a.id, super::slime::target_id(usize::from(r))))
            })
            .collect();
        Ok(Self {
            props,
            pristine_hazards: hazards.clone(),
            hazards,
            owners,
            solids,
            links,
            drum_charges: Vec::new(),
            solid_start: None,
            prev: Vec::new(),
            stickers,
            acid,
            acid_links,
            floor_layout: None,
            defeat_roster: enc.goal == Goal::DefeatRoster,
        })
    }

    /// HRD-A2: add a slot enemy's entry gate as a runtime solid, after every
    /// other solid (`rect` in layout px, y down).
    #[must_use]
    pub fn with_slot_gate(mut self, target: u32, rect: IRect) -> Self {
        self.solids.push(EnvSolid {
            rect,
            prop: 0,
            kind: EnvSolidKind::Gate { target },
        });
        self
    }

    /// HAZ-A2: place this runtime in a room: its walking line (layout px, for
    /// the sticker tracks and the walk barriers), the soffit solids the
    /// stickers cling to (layout px, no standable top) and, for a
    /// `defeat_roster` room, the exit gate rectangle.
    #[must_use]
    pub fn in_room(mut self, floor_layout: i32, soffits: &[IRect], exit: Option<IRect>) -> Self {
        self.floor_layout = Some(floor_layout);
        for r in soffits {
            self.solids.push(EnvSolid {
                rect: *r,
                prop: 0,
                kind: EnvSolidKind::Soffit,
            });
        }
        if self.defeat_roster
            && let Some(rect) = exit
        {
            self.solids.push(EnvSolid {
                rect,
                prop: 0,
                kind: EnvSolidKind::ExitGate,
            });
        }
        self
    }

    /// HAZ-A2: build the stickers' domain tracks in the scene's world frame
    /// (`foot_y` is the world y of the walking line). Idempotent.
    ///
    /// # Panics
    ///
    /// On a track the encounter validator accepted but the domain refuses, or a
    /// sticker room that was never placed with [`Self::in_room`].
    pub(super) fn attach(&mut self, foot_y: f32) {
        if self.stickers.is_empty() {
            return;
        }
        let floor = self
            .floor_layout
            .expect("a room with Rot Stickers is placed with EnvRuntime::in_room");
        let origin = foot_y.round() as i32 + floor;
        for st in &mut self.stickers {
            if st.sticker.is_none() {
                let track = st
                    .layout
                    .world_track(origin)
                    .expect("the validated sticker track");
                st.sticker = Some(RotSticker::new(track));
            }
        }
    }

    /// HAZ-A2: the stickers' count (0 in every other room).
    pub fn sticker_count(&self) -> usize {
        self.stickers.len()
    }

    /// The sticker domain object of sidecar prop `prop`.
    pub fn sticker(&self, prop: u16) -> Option<&RotSticker> {
        self.stickers
            .iter()
            .find(|s| s.prop == prop)
            .and_then(|s| s.sticker.as_ref())
    }

    /// The acid patches.
    pub fn acid(&self) -> &AcidSet {
        &self.acid
    }

    /// HAZ-A2: the walk barriers of the acid patches (world px): one per live
    /// patch, on the walking line of its surface, **following the live bounds**
    /// (the drain shrinks them every tick and a removed pool has none), so a
    /// drained pool stops blocking (validator rule 7, HAZ-Q21). The scene gives
    /// the goblins this list before it steps them every tick
    /// ([`crate::goblin::Goblin::with_walk_barriers`]).
    pub fn walk_barriers(&self, foot_y: f32) -> Vec<WalkBarrier> {
        let Some(floor) = self.floor_layout else {
            return Vec::new();
        };
        self.acid
            .patches()
            .filter_map(|p| {
                let (x0, x1) = p.bounds()?;
                Some(WalkBarrier {
                    x0: x0 as f32,
                    x1: x1 as f32,
                    line: foot_y + (floor - p.spec().floor_y) as f32,
                })
            })
            .collect()
    }

    /// Whether the room has acid patches at all (a room without them never
    /// refreshes a goblin's barrier list).
    pub fn has_acid(&self) -> bool {
        self.acid.patches().next().is_some()
    }

    /// Back to the first state of the attempt (the solids stay appended).
    pub fn reset(&mut self) {
        for p in &mut self.props {
            match &mut p.machine {
                Machine::Valve { valve, .. } => valve.reset(),
                Machine::Mass(m) => m.reset(),
                Machine::Drum { drum, .. } => drum.reset(),
                Machine::Rot => {}
            }
        }
        self.hazards = self.pristine_hazards.clone();
        self.prev.clear();
    }

    /// HAZ-A2: a full restart of the hazards: every sticker latched at its
    /// start (spent included), every pool full.
    pub fn reset_hazards(&mut self) {
        for st in &mut self.stickers {
            if let Some(s) = st.sticker.as_mut() {
                s.reset();
            }
            st.pending.clear();
        }
        self.acid.reset(slime::active());
    }

    /// HAZ-A2: a defeat respawn (HAZ-Q49): a spent sticker stays spent, a
    /// sticker in warning or scuttling is latched again at its start. The pools
    /// follow their slimes (the scene applies that, it reads the slot).
    pub fn respawn_hazards(&mut self) {
        for st in &mut self.stickers {
            if let Some(s) = st.sticker.as_mut() {
                s.defeat_respawn();
            }
            st.pending.clear();
        }
    }

    pub fn props(&self) -> usize {
        self.props.len()
    }

    /// The valve's state, if prop `index` is a valve.
    pub fn valve_state(&self, index: usize) -> Option<ValveState> {
        match &self.props.get(index)?.machine {
            Machine::Valve { valve, .. } => Some(valve.state()),
            _ => None,
        }
    }

    /// The falling mass' state, if prop `index` is a weight.
    pub fn mass_state(&self, index: usize) -> Option<MassState> {
        match &self.props.get(index)?.machine {
            Machine::Mass(m) => Some(m.state()),
            _ => None,
        }
    }

    pub fn mass(&self, index: usize) -> Option<&FallingMass> {
        match &self.props.get(index)?.machine {
            Machine::Mass(m) => Some(m),
            _ => None,
        }
    }

    pub fn hazards(&self) -> &HazardSet {
        &self.hazards
    }

    /// The CA2-C feed: the telegraphed and active volumes of this tick, plus the
    /// footprint of every falling mass that has been released (the shadow, from
    /// the release to the drop: telegraph; the drop's last ticks: active). The
    /// weight's `weight_fall` volume is never triggered (the mass owns the
    /// damage, see the module docs), so its footprint reaches the feed here, from
    /// the mass' own state, as the same plain rectangle every other hazard is.
    pub fn nav_feed(&self) -> Vec<NavRect> {
        let mut feed = self.hazards.nav_feed();
        // HAZ-A2: the acid patches are persistent active volumes of class Acid.
        feed.extend(self.acid.nav_feed());
        for p in &self.props {
            let Machine::Mass(m) = &p.machine else {
                continue;
            };
            let phase = match m.state() {
                MassState::Released => NavPhase::Telegraph,
                MassState::Falling => NavPhase::Active,
                _ => continue,
            };
            let cfg = m.config();
            let hazard = self
                .owners
                .iter()
                .find(|(_, prop)| *prop == p.index)
                .map_or(p.index, |(h, _)| *h);
            feed.push(NavRect {
                hazard,
                class: HazardClass::FallingMass,
                rect: cfg.footprint,
                phase,
                ticks_left: None,
                hurts_heroes: cfg.hero_damage.hurts_any_hero(),
                hurts_enemies: cfg.hurts_enemies,
            });
        }
        feed
    }

    /// The runtime solids, in index order of the sync.
    pub fn solids(&self) -> &[EnvSolid] {
        &self.solids
    }

    /// Whether solid `i` stands now.
    pub fn solid_intact(&self, i: usize) -> bool {
        let Some(s) = self.solids.get(i) else {
            return false;
        };
        // A lock and a gate follow a slot enemy, which the combat state reads
        // (`env_sync_solids`); the props know nothing of them, and a room may have
        // none (the Hoarder's `props` is empty): the kind is checked before props
        // are indexed.
        let prop = match s.kind {
            EnvSolidKind::Remnant | EnvSolidKind::Drum => self.props.get(usize::from(s.prop)),
            EnvSolidKind::Lock { .. } | EnvSolidKind::Gate { .. } => return false,
            // HAZ-A2: a soffit always stands; the exit gate defaults to closed (the
            // scene opens it when the roster is clear). Neither has a prop.
            EnvSolidKind::Soffit | EnvSolidKind::ExitGate => return true,
        };
        match (s.kind, prop.map(|p| &p.machine)) {
            (EnvSolidKind::Remnant, Some(Machine::Mass(m))) => m.state() == MassState::Landed,
            (EnvSolidKind::Drum, Some(Machine::Drum { drum, .. })) => {
                drum.state() != DrumState::Detonated
            }
            _ => false,
        }
    }

    /// The index range of the solids in `scene.world.solids` (empty before the
    /// first step appended them).
    pub fn solid_range(&self) -> Range<usize> {
        self.solid_start.map_or(0..0, |s| s..s + self.solids.len())
    }

    /// The hit targets of the props that can still react (BOILBC-AD3), layout
    /// px, y down.
    pub fn live_targets(&self) -> Vec<(u16, IRect)> {
        self.props
            .iter()
            .filter(|p| match &p.machine {
                Machine::Valve { valve, .. } => valve.state() == ValveState::Intact,
                Machine::Mass(m) => m.is_supported(),
                // A drum is no hit target of melee or the missile (BOILD-AD3).
                Machine::Drum { .. } | Machine::Rot => false,
            })
            .map(|p| (p.index, p.target))
            .collect()
    }

    /// BOIL-D: the boxes a blast must reach to set a prop off, per prop that
    /// can still be set off (the `BlastTarget` footprints), layout px, y down.
    /// Unlike [`Self::live_targets`] this includes an armed drum.
    pub fn blast_targets(&self) -> Vec<(u16, IRect)> {
        self.props
            .iter()
            .filter(|p| match &p.machine {
                Machine::Valve { valve, .. } => valve.state() == ValveState::Intact,
                Machine::Mass(m) => m.is_supported(),
                Machine::Drum { drum, .. } => drum.is_armed(),
                // A blast never provokes a sticker (its charge is reached by the chain).
                Machine::Rot => false,
            })
            .map(|p| (p.index, p.target))
            .collect()
    }

    /// The drum's state, if prop `index` is a drum.
    pub fn drum_state(&self, index: usize) -> Option<DrumState> {
        match &self.props.get(index)?.machine {
            Machine::Drum { drum, .. } => Some(drum.state()),
            _ => None,
        }
    }

    /// The drums' blast plans, `(prop, plan)` in prop order.
    pub fn drum_plans(&self) -> Vec<(u16, DrumPlan)> {
        self.props
            .iter()
            .filter_map(|p| match &p.machine {
                Machine::Drum { plan, .. } => Some((p.index, *plan)),
                _ => None,
            })
            .collect()
    }

    /// The authored links.
    pub fn links(&self) -> &LinkGraph {
        &self.links
    }

    /// Record which `BlastCharge` is the drum `prop`'s (set by the scene when
    /// it pushes the charge).
    pub(super) fn set_drum_charge(&mut self, prop: u16, charge: usize) {
        match self.drum_charges.iter_mut().find(|(p, _)| *p == prop) {
            Some(slot) => slot.1 = charge,
            None => self.drum_charges.push((prop, charge)),
        }
    }

    /// The charge index of drum `prop`.
    pub fn drum_charge(&self, prop: u16) -> Option<usize> {
        self.drum_charges
            .iter()
            .find(|(p, _)| *p == prop)
            .map(|(_, c)| *c)
    }

    /// The drum prop a charge belongs to.
    pub fn drum_of_charge(&self, charge: usize) -> Option<u16> {
        self.drum_charges
            .iter()
            .find(|(_, c)| *c == charge)
            .map(|(p, _)| *p)
    }

    /// `(charge index, telegraph ticks)` of drum `prop`: what the scene needs
    /// to arm its blast.
    pub fn drum_clock(&self, prop: u16) -> Option<(usize, u16)> {
        let charge = self.drum_charge(prop)?;
        match &self.props.get(usize::from(prop))?.machine {
            Machine::Drum { plan, .. } => Some((charge, plan.telegraph_ticks)),
            _ => None,
        }
    }

    /// The drum's machine follows a charge a chain reaction lit (CP-Q34): the
    /// credit and depth the charge took. True when the drum changed.
    fn drum_adopt_chain(&mut self, prop: u16, initiator: Option<u32>, depth: u8) -> bool {
        match self
            .props
            .get_mut(usize::from(prop))
            .map(|p| &mut p.machine)
        {
            Some(Machine::Drum { drum, .. }) => drum.adopt_chain(initiator, depth),
            _ => false,
        }
    }

    /// The drum's charge blew: the machine is `Detonated` and its `Blast`
    /// links are returned for the scene to deliver.
    fn drum_blown(&mut self, prop: u16) -> Option<LinkFire> {
        match self
            .props
            .get_mut(usize::from(prop))
            .map(|p| &mut p.machine)
        {
            Some(Machine::Drum { drum, .. }) => {
                drum.detonated();
                Some(LinkFire {
                    from: prop,
                    emit: LinkEmit::Blast,
                    initiator: drum.initiator(),
                    depth: drum.trigger_depth(),
                })
            }
            _ => None,
        }
    }

    /// The area a drum's tell shows while it is set off (layout px, y down),
    /// the ticks left of the tell are the charge's business.
    pub fn drum_telegraph_rect(&self, prop: usize) -> Option<IRect> {
        match &self.props.get(prop)?.machine {
            Machine::Drum { drum, plan } if drum.state() == DrumState::Telegraph => {
                Some(plan.telegraph_rect)
            }
            _ => None,
        }
    }

    /// Offer one stimulus to prop `index`. Returns the offer and the events of
    /// a counted one (`PropHit`, and `PropPhase` when it set the prop off). A
    /// valve that is set off triggers its steam volume with the initiator.
    pub fn offer(
        &mut self,
        index: usize,
        stimulus: Stimulus,
        tick: u64,
    ) -> Option<(Offer, Vec<CombatEvent>)> {
        let prop = self.props.get_mut(index)?;
        let prop_id = prop.index;
        let result = match &mut prop.machine {
            Machine::Valve { valve, .. } => valve.offer(stimulus),
            Machine::Mass(m) => m.offer(stimulus),
            Machine::Drum { drum, .. } => drum.offer(stimulus),
            Machine::Rot => return None,
        };
        let mut events = Vec::new();
        let progress = match result {
            Offer::Ignored(_) => return Some((result, events)),
            Offer::Progress { progress } => progress,
            Offer::Triggered { .. } => match &prop.machine {
                Machine::Valve { valve, .. } => valve.progress(),
                Machine::Mass(m) => m.progress(),
                // A drum has one weight: the trigger itself.
                Machine::Drum { .. } => 1,
                Machine::Rot => 0,
            },
        };
        events.push(CombatEvent::PropHit {
            tick,
            prop: prop_id,
            kind: stimulus.kind(),
            progress,
            player: stimulus.initiator(),
        });
        if let Offer::Triggered { initiator, .. } = result {
            let phase = match &prop.machine {
                Machine::Valve { steam, .. } => {
                    let steam = *steam;
                    let started = self.hazards.trigger(steam, initiator);
                    assert!(started, "the steam starts with the valve, exactly once");
                    PropPhase::Warning
                }
                // The drum's tell is its warning; its charge keeps the clock.
                Machine::Drum { .. } => PropPhase::Warning,
                Machine::Mass(_) => PropPhase::Released,
                // A sticker takes its stimuli in its own step (`offer` returns above).
                Machine::Rot => return Some((result, events)),
            };
            events.push(CombatEvent::PropPhase {
                tick,
                prop: prop_id,
                phase,
                player: initiator,
            });
        }
        Some((result, events))
    }

    fn owner_of(&self, hazard: u16) -> u16 {
        self.owners
            .iter()
            .find(|(h, _)| *h == hazard)
            .map_or(hazard, |(_, p)| *p)
    }

    /// One fixed tick after the stimuli of the tick: the valve, the hazard
    /// windows, the falling mass (its landing evaluation) and the steam
    /// contacts. `bodies` are the victims in layout px.
    pub fn step(&mut self, tick: u64, bodies: &[(Victim, IRect)]) -> EnvTick {
        let mut out = EnvTick::default();
        // BOILBC-AD5 / NB-1: a body that jumped further than a tick of motion
        // teleported: its swept volume would cross everything between.
        let mass_bodies: Vec<MassBody> = bodies
            .iter()
            .map(|(victim, cur)| {
                let prev = self
                    .prev
                    .iter()
                    .find(|(v, _)| v == victim)
                    .map_or(*cur, |(_, p)| *p);
                let far =
                    (cur.x - prev.x).abs() > TELEPORT_PX || (cur.y - prev.y).abs() > TELEPORT_PX;
                MassBody {
                    victim: *victim,
                    prev: if far { *cur } else { prev },
                    cur: *cur,
                }
            })
            .collect();
        for p in &mut self.props {
            let prop = p.index;
            match &mut p.machine {
                Machine::Valve { valve, .. } => match valve.step() {
                    Some(ValveEdge::VentStarted) => {
                        out.events.push(CombatEvent::PropPhase {
                            tick,
                            prop,
                            phase: PropPhase::Venting,
                            player: valve.initiator(),
                        });
                        // BOIL-D: the steam reaches what the sidecar links.
                        out.fires.push(LinkFire {
                            from: prop,
                            emit: LinkEmit::Steam,
                            initiator: valve.initiator(),
                            depth: valve.trigger_depth(),
                        });
                    }
                    Some(ValveEdge::Spent) => out.events.push(CombatEvent::PropPhase {
                        tick,
                        prop,
                        phase: PropPhase::Spent,
                        player: valve.initiator(),
                    }),
                    None => {}
                },
                Machine::Mass(m) => match m.step(&mass_bodies) {
                    Some(MassEdge::FallStarted) => out.events.push(CombatEvent::PropPhase {
                        tick,
                        prop,
                        phase: PropPhase::Falling,
                        player: m.initiator(),
                    }),
                    Some(MassEdge::Landed(landing)) => {
                        out.events.push(CombatEvent::PropPhase {
                            tick,
                            prop,
                            phase: PropPhase::Landed,
                            player: landing.initiator,
                        });
                        let centre = m.config().footprint.x + m.config().footprint.w / 2;
                        for h in &landing.hits {
                            out.hits.push(EnvHit {
                                victim: h.victim,
                                prop,
                                class: HazardClass::FallingMass,
                                damage: h.damage,
                                push_x: h.knock_x_px_s,
                                knock_up: h.knock_up_px_s,
                                from_x: centre,
                                initiator: h.initiator,
                                activation: 1,
                            });
                        }
                        for l in &landing.lifts {
                            out.lifts.push(EnvLift {
                                hero: l.victim.id,
                                up_px: l.up_px,
                            });
                        }
                        out.embedded.extend(landing.embedded);
                        // BOIL-D: the landing reaches what the sidecar links.
                        out.fires.push(LinkFire {
                            from: prop,
                            emit: LinkEmit::Landing,
                            initiator: landing.initiator,
                            depth: m.trigger_depth(),
                        });
                    }
                    None => {}
                },
                // A drum's clock is its charge's (the scene steps it).
                Machine::Drum { .. } | Machine::Rot => {}
            }
        }
        // The hazard windows advance in lockstep with the valve; their edges
        // carry nothing the props did not already log.
        let edges = self.hazards.step();
        debug_assert!(
            edges
                .iter()
                .all(|(_, e)| matches!(e, HazardEdge::ActiveStarted | HazardEdge::Ended))
        );
        for hit in self.hazards.resolve(bodies) {
            let volume = self.hazards.get(hit.hazard).expect("a resolved volume");
            let r = volume.spec().rect;
            out.hits.push(EnvHit {
                victim: hit.victim,
                prop: self.owner_of(hit.hazard),
                class: hit.class,
                damage: hit.damage,
                push_x: hit.push_x,
                knock_up: 0,
                from_x: r.x + r.w / 2,
                initiator: hit.initiator,
                activation: hit.activation,
            });
        }
        self.prev = bodies.to_vec();
        out
    }

    /// Fold the full state into a state hash, behind a marker word.
    pub fn mix_into(&self, mix: &mut impl FnMut(u64)) {
        mix(0x454e_5652_0001); // "ENVR"
        mix(self.props.len() as u64);
        for p in &self.props {
            mix(u64::from(p.index));
            match &p.machine {
                Machine::Valve { valve, .. } => valve.mix_into(&mut *mix),
                Machine::Mass(m) => m.mix_into(&mut *mix),
                Machine::Drum { drum, .. } => drum.mix_into(&mut *mix),
                Machine::Rot => {}
            }
        }
        self.hazards.mix_into(&mut *mix);
        // HAZ-A2: nothing is mixed in a room without stickers or acid, so every
        // older hash is unchanged.
        if !self.stickers.is_empty() {
            mix(0x524f_5453_0001); // "ROTS"
            for st in &self.stickers {
                mix(u64::from(st.prop));
                if let Some(s) = &st.sticker {
                    s.mix_into(&mut *mix);
                }
            }
        }
        if self.acid.patches().next().is_some() {
            mix(0x4143_4944_0001); // "ACID"
            self.acid.mix_into(&mut *mix);
        }
    }

    /// ` env=<prop>:<state>,...` for the state line.
    pub fn token(&self) -> String {
        let parts: Vec<String> = self
            .props
            .iter()
            .map(|p| match &p.machine {
                Machine::Valve { valve, .. } => format!("{}:{:?}", p.index, valve.state()),
                Machine::Mass(m) => format!("{}:{:?}", p.index, m.state()),
                Machine::Drum { drum, .. } => format!("{}:drum_{:?}", p.index, drum.state()),
                Machine::Rot => {
                    let state = self
                        .stickers
                        .iter()
                        .find(|s| s.prop == p.index)
                        .and_then(|s| s.sticker.as_ref())
                        .map_or("off", |s| match s.state() {
                            StickerState::Latched => "latched",
                            StickerState::Scuttling => "scuttling",
                            StickerState::Warning => "warning",
                            StickerState::Spent => "spent",
                        });
                    format!("{}:rot_{}", p.index, state)
                }
            })
            .collect();
        format!(" env={}", parts.join(","))
    }
}

/// BOILD-AD1: the weight a prop gives the contacts of its incoming links. A
/// steam link carries a **light** contact (1: steam alone must not always drop
/// the weight, Boiler P1: it finishes a brace that is already weakened); a
/// landing or a blast link carries the **full** weight (`full`: the trigger or
/// release weight of the receiving machine). The strongest incoming link wins.
fn linked_weight(incoming: &[(u16, LinkEmit)], full: u8) -> u8 {
    incoming
        .iter()
        .map(|(_, emit)| match emit {
            LinkEmit::Steam => 1.min(full),
            LinkEmit::Landing | LinkEmit::Blast => full,
        })
        .max()
        .unwrap_or(0)
}

// ------------------------------------------------------------- conversions

fn world_rect(layout_h: i32, r: IRect) -> Rect {
    Rect {
        left: r.x as f32,
        bottom: (layout_h - r.bottom()) as f32,
        width: r.w as f32,
        height: r.h as f32,
    }
}

/// A world rectangle (y up) in layout px (y down), each edge rounded to the
/// nearest pixel.
fn layout_rect(layout_h: i32, r: Rect) -> IRect {
    let x0 = r.left.round() as i32;
    let x1 = r.right().round() as i32;
    let y_top = layout_h - r.top().round() as i32;
    let y_bottom = layout_h - r.bottom.round() as i32;
    IRect::new(x0, y_top, x1 - x0, y_bottom - y_top)
}

// ------------------------------------------------------------- scene half

/// What the slot enemies say about the locks and gates (HRD-A2), read once per
/// sync: `(target id, alive, gate closed)`.
struct SlotSolids(Vec<(u32, bool, bool)>);

impl SlotSolids {
    fn of(slot: &super::slot::SlotRuntime) -> Self {
        Self(
            slot.enemies
                .iter()
                .map(|e| (e.target_id(), e.alive(), e.gate_solid().is_some()))
                .collect(),
        )
    }

    /// A lock stands while the enemy it waits on is alive. An enemy that is not
    /// in the slot keeps the lock closed (nothing can open it).
    fn lock_closed(&self, target: u32) -> bool {
        self.0.iter().find(|e| e.0 == target).is_none_or(|e| e.1)
    }

    fn gate_closed(&self, target: u32) -> bool {
        self.0.iter().find(|e| e.0 == target).is_some_and(|e| e.2)
    }
}

impl CombatState {
    /// Run this state with the env step of `env` (the Boiler room). Without
    /// it nothing is stepped, evented or hashed: every other fixture is the
    /// legacy game byte for byte.
    #[must_use]
    pub fn with_env(mut self, mut env: EnvRuntime) -> Self {
        // HAZ-A2: the stickers' tracks need the scene's walking line.
        env.attach(self.enemy_foot_y);
        self.env = Some(Box::new(env));
        self.reset_attempt();
        self
    }

    /// HAZ-A2: every sticker's authored charge is appended **after the
    /// fixture's charges** (sidecar order) when an attempt is built, at the end
    /// of `reset_attempt`. The reserved dynamite slot is the index after them
    /// (CP-Q31), and `push_authored_charge` keeps it so. A charge starts unlit
    /// (`detonate_tick` 0, no owner, never fires: tick 0 is never stepped) and
    /// carries the Rot profile with the sticker's own `HeroHurt` source.
    pub(super) fn env_append_charges(&mut self) {
        let Some(env) = self.env.as_mut() else {
            return;
        };
        let t = rot_sticker::active();
        let mut sources = Vec::new();
        for (i, st) in env.stickers.iter_mut().enumerate() {
            st.pending.clear();
            sources.push((
                i,
                rot_sticker::rot_source(u8::try_from(i).unwrap_or(u8::MAX)),
            ));
        }
        // The charge index is read before each push (a drum's blast charge, BOIL-D,
        // may already stand among the authored charges).
        for (i, source) in sources {
            let index = self.charges.len();
            self.push_authored_charge(
                BlastCharge::new(0.0, 0.0, 0, None).with_profile(t.world_profile(source)),
            );
            if let Some(env) = self.env.as_mut() {
                env.stickers[i].charge = index;
            }
        }
        if let Some(env) = self.env.as_mut() {
            env.reset_hazards();
        }
    }

    /// HAZ-A2, HAZ-Q49: a defeat respawn. A spent sticker stays spent; one in
    /// warning or scuttling is latched again at its start and its charge is
    /// written back to unlit (`detonate_tick` 0, owner none, chain depth 0). The
    /// pools follow their slimes: a living slime was reset to its spawn by the
    /// slot's respawn (so its pool is restored and its drain cancelled), a
    /// defeated one stays defeated and its pool keeps draining.
    pub(super) fn env_respawn_hazards(&mut self) {
        let alive: Vec<(u16, bool)> = match self.env.as_ref() {
            Some(env) => env
                .acid_links
                .iter()
                .map(|(patch, target)| {
                    let up = self
                        .slot
                        .index_of(*target)
                        .is_some_and(|i| self.slot.enemies[i].alive());
                    (*patch, up)
                })
                .collect(),
            None => return,
        };
        let env = self.env.as_mut().expect("checked above");
        env.respawn_hazards();
        for (patch, up) in alive {
            if up && let Some(p) = env.acid.get_mut(patch) {
                p.on_slime_respawn(slime::active());
            }
        }
        let slots: Vec<(usize, bool)> = env
            .stickers
            .iter()
            .map(|st| {
                (
                    st.charge,
                    st.sticker.as_ref().is_some_and(RotSticker::is_spent),
                )
            })
            .collect();
        for (charge, spent) in slots {
            if !spent && let Some(c) = self.charges.get_mut(charge) {
                c.detonate_tick = 0;
                c.owner = None;
                c.chain_depth = 0;
            }
        }
    }

    /// HAZ-A2, HAZ-Q39: every roster enemy is defeated (a goblin, a llama, a
    /// slot enemy). Stickers are props and never count.
    pub fn roster_cleared(&self) -> bool {
        self.enemies.iter().all(|g| !g.alive())
            && self.llamas.iter().all(|l| !l.alive())
            && self.slot.enemies.iter().all(|e| !e.alive())
    }

    /// The env runtime, if the room has props.
    pub fn env(&self) -> Option<&EnvRuntime> {
        self.env.as_deref()
    }

    /// A new attempt: every prop back to its first state, and every drum's
    /// charge back to unlit (BOIL-D).
    pub(super) fn env_reset(&mut self) {
        if let Some(env) = self.env.as_mut() {
            env.reset();
        }
        self.env_drum_charges();
    }

    /// BOIL-D: each drum's blast is one authored `BlastCharge` after the
    /// fixture's own (no second explosion module, no owner field of its own).
    /// It stands **unlit** (`Arm::Tick` with tick 0 never fires) at the drum's
    /// column on the walking line; the scene lights it with
    /// [`BlastCharge::chain_set`] when the drum is set off. A rebuilt attempt
    /// pushes it; a checkpoint respawn finds it (same column and profile) and
    /// resets it in place, so its index never moves.
    fn env_drum_charges(&mut self) {
        let Some(plans) = self.env.as_ref().map(|e| e.drum_plans()) else {
            return;
        };
        for (prop, plan) in plans {
            let fresh = BlastCharge::new(plan.x as f32, self.enemy_foot_y, 0, None)
                .with_profile(plan.profile());
            let found = (0..self.authored_charges).find(|&i| {
                self.charges[i].x == fresh.x && self.charges[i].profile == fresh.profile
            });
            let index = match found {
                Some(i) => {
                    self.charges[i] = fresh;
                    i
                }
                None => {
                    self.push_authored_charge(fresh);
                    self.charges.len() - 1
                }
            };
            if let Some(env) = self.env.as_mut() {
                env.set_drum_charge(prop, index);
            }
        }
    }

    /// Whether `charge` is a prop's blast (the presentation hides its stick).
    pub fn env_owns_charge(&self, charge: usize) -> bool {
        self.env
            .as_ref()
            .is_some_and(|e| e.drum_of_charge(charge).is_some())
    }

    /// After a restart or a respawn: the solids follow the props at once, not
    /// on the next env step (a remnant of the last attempt must not stand for
    /// a tick).
    ///
    /// The slot enemies get the fresh view at once as well (the previous
    /// attempt's targets and solid flags are never seen after a respawn).
    pub(super) fn env_after_reset(&mut self, scene: &mut SceneSim) {
        if self.env.is_some() {
            self.env_sync_solids(scene);
            let view = self.env_view(scene.world.layout_height());
            self.set_slot_env(view);
        }
    }

    /// Append the env solids and fill the slot's env view **before the first
    /// tick**, so tick 1 of a first attempt equals tick 1 of a restart (after a
    /// restart the solids are already in the world). Call it once after
    /// [`Self::with_env`] when the scene is at hand; a state that skips it
    /// appends them inside the first env step instead (the drum then collides
    /// one tick late on that first tick only).
    pub fn sync_env(&mut self, scene: &mut SceneSim) {
        self.env_after_reset(scene);
    }

    /// The KR6 sync: append the whole set once (idempotent) and rewrite each
    /// block's `intact` from the prop state.
    fn env_sync_solids(&mut self, scene: &mut SceneSim) {
        let layout_h = scene.world.layout_height();
        let dynamic = SlotSolids::of(&self.slot);
        let cleared = self.roster_cleared();
        let Some(env) = self.env.as_mut() else {
            return;
        };
        if env.solids.is_empty() {
            return;
        }
        let want: Vec<Rect> = env
            .solids
            .iter()
            .map(|s| world_rect(layout_h, s.rect))
            .collect();
        let present = env.solid_start.is_some_and(|start| {
            scene
                .world
                .solids
                .get(start..start + want.len())
                .is_some_and(|have| have.iter().zip(&want).all(|(b, r)| b.rect == *r))
        });
        if !present {
            env.solid_start = Some(scene.world.solids.len());
            for (i, rect) in want.iter().enumerate() {
                scene.world.solids.push(Block {
                    rect: *rect,
                    material: Material::Timber,
                    intact: false,
                });
                debug_assert!(i < env.solids.len());
            }
        }
        let start = env.solid_start.expect("synced");
        for i in 0..env.solids.len() {
            let intact = match env.solids[i].kind {
                EnvSolidKind::Lock { target } => dynamic.lock_closed(target),
                EnvSolidKind::Gate { target } => dynamic.gate_closed(target),
                // HAZ-A2: the exit opens on the tick the roster is clear.
                EnvSolidKind::ExitGate => !cleared,
                _ => env.solid_intact(i),
            };
            scene.world.solids[start + i].intact = intact;
        }
    }

    /// The slot enemies' env read access: the solid range and the live hit
    /// targets (world rectangles, y up).
    fn env_view(&self, layout_h: i32) -> EnvView {
        let Some(env) = self.env.as_ref() else {
            return EnvView::default();
        };
        EnvView {
            solid_range: env.solid_range(),
            targets: env
                .live_targets()
                .into_iter()
                .map(|(prop, rect)| EnvTarget {
                    prop,
                    rect: world_rect(layout_h, rect),
                    solid: false,
                })
                .collect(),
        }
    }

    /// Offer a stimulus to a prop and log its events. The one entry point for
    /// a stimulus that does not come from Carl's melee or Donut's missile
    /// (BOIL-D's launched bodies and blast hook, the Hoarder's shove).
    pub fn env_stimulate(&mut self, prop: usize, stimulus: Stimulus, tick: u64) -> Option<Offer> {
        // HAZ-A2: a Rot Sticker takes the stimulus in its own step (the intake
        // of the sheet's section 10: melee, projectile and launched body count;
        // a blast, a prop contact and a shove are ignored by the sticker).
        if let Some(st) = self
            .env
            .as_mut()?
            .stickers
            .iter_mut()
            .find(|s| usize::from(s.prop) == prop)
        {
            st.pending.push((stimulus, None));
            return Some(Offer::Progress { progress: 1 });
        }
        let (offer, events) = self.env.as_mut()?.offer(prop, stimulus, tick)?;
        for event in events {
            self.log(event);
        }
        if let Offer::Triggered { initiator, depth } = offer {
            self.env_arm_drum(prop, tick, initiator, depth);
        }
        Some(offer)
    }

    /// A drum that was just set off lights its charge: the blast comes
    /// `telegraph_ticks` after `tick`. [`BlastCharge::chain_set`] keeps the
    /// first writer's credit and depth, and leaves a charge a chain already
    /// lit for an earlier tick alone (CP-Q34), so one clock serves the prop
    /// link, a launched body, a blast and a chain reaction. A prop that is not
    /// a drum does nothing here.
    fn env_arm_drum(&mut self, prop: usize, tick: u64, initiator: Option<u32>, depth: u8) {
        let Some((charge, telegraph)) = u16::try_from(prop)
            .ok()
            .and_then(|p| self.env.as_ref()?.drum_clock(p))
        else {
            return;
        };
        self.charges[charge].chain_set(tick, u64::from(telegraph), initiator, depth);
    }

    /// Deliver what a prop edge carries along its links: one `PropContact` per
    /// linked prop, one link deeper, credited to the initiator (BOIL-D). The
    /// machines enforce the depth cap and the one activation per prop.
    fn env_fire(&mut self, fire: LinkFire, tick: u64) {
        let targets = self
            .env
            .as_ref()
            .map(|e| e.links().targets(fire.from, fire.emit))
            .unwrap_or_default();
        for to in targets {
            let contact = LinkGraph::contact(fire.from, fire.initiator, fire.depth);
            self.env_stimulate(usize::from(to), contact, tick);
        }
    }

    /// A drum whose charge a chain reaction lit (no offer reached the machine):
    /// the machine follows the charge into its telegraph, with the credit and
    /// depth the charge took, and logs the tell (a blast hit, then the warning).
    fn env_follow_lit_drums(&mut self, tick: u64) {
        let Some(env) = self.env.as_ref() else {
            return;
        };
        let lit: Vec<(u16, Option<u32>, u8)> = env
            .drum_plans()
            .into_iter()
            .filter(|(prop, _)| env.drum_state(usize::from(*prop)) == Some(DrumState::Armed))
            .filter_map(|(prop, _)| {
                let charge = &self.charges[env.drum_charge(prop)?];
                (!charge.detonated && charge.detonate_tick > 0).then_some((
                    prop,
                    charge.owner,
                    charge.chain_depth,
                ))
            })
            .collect();
        for (prop, owner, depth) in lit {
            let followed = self
                .env
                .as_mut()
                .is_some_and(|e| e.drum_adopt_chain(prop, owner, depth));
            if followed {
                self.log(CombatEvent::PropHit {
                    tick,
                    prop,
                    kind: crate::env_prop::StimulusKind::Blast,
                    progress: 1,
                    player: owner,
                });
                self.log(CombatEvent::PropPhase {
                    tick,
                    prop,
                    phase: PropPhase::Warning,
                    player: owner,
                });
            }
        }
    }

    /// The hook of `step_charges`: right after a charge's `Blast` event, before
    /// anything of that blast resolves. A drum's charge makes the drum
    /// `Detonated`, so its solid is gone for this blast's line of sight and
    /// every later collision, logs the phase and delivers its `blast` links.
    pub(super) fn env_charge_blown(&mut self, scene: &mut SceneSim, charge: usize, tick: u64) {
        let Some(prop) = self.env.as_ref().and_then(|e| e.drum_of_charge(charge)) else {
            return;
        };
        // A charge that was lit and blew without the machine ever seeing it
        // (it cannot happen with the 8-tick chain delay) still follows.
        self.env_follow_lit_drums(tick);
        let Some(fire) = self.env.as_mut().and_then(|e| e.drum_blown(prop)) else {
            return;
        };
        self.log(CombatEvent::PropPhase {
            tick,
            prop,
            phase: PropPhase::Detonated,
            player: fire.initiator,
        });
        self.env_sync_solids(scene);
        self.env_fire(fire, tick);
    }

    /// Carl's melee on the props: the active box on a live target, one hit per
    /// move instance and prop. Runs after the slot strike, before the scene
    /// step. A counted hit freezes Carl and kicks the camera (BOILBC-AD2).
    pub(super) fn strike_env(
        &mut self,
        scene: &mut SceneSim,
        move_tick: &MoveTick,
        attacker: Attacker,
        tick: u64,
    ) {
        if self.env.is_none() || move_tick.phase != Phase::Active || move_tick.frozen {
            return;
        }
        let Some(boxed) = hit_box(&move_tick.spec.shape, &attacker) else {
            return;
        };
        let layout_h = scene.world.layout_height();
        let targets = self
            .env
            .as_deref()
            .map(EnvRuntime::live_targets)
            .unwrap_or_default();
        for (prop, target) in targets {
            if !world_rect(layout_h, target).overlaps(boxed) {
                continue;
            }
            if !scene
                .world
                .hero
                .melee
                .record_hit(ENV_HIT_ID_BASE + u32::from(prop))
            {
                continue;
            }
            let stimulus = Stimulus::MeleeHit {
                token: move_tick.instance,
                player: attacker.player,
                weight: 1,
            };
            let counted = matches!(
                self.env_stimulate(usize::from(prop), stimulus, tick),
                Some(Offer::Progress { .. } | Offer::Triggered { .. })
            );
            if counted {
                let spec = move_tick.spec;
                scene
                    .world
                    .hero
                    .melee
                    .begin_hit_stop(spec.reaction.hit_stop);
                self.confirm = Some(Confirm {
                    tick,
                    kick_px: spec.reaction.camera_kick_px,
                    direction: attacker.facing,
                });
            }
        }
        self.strike_stickers(scene, move_tick, boxed, attacker, tick);
    }

    /// HAZ-A2: Carl's melee on a Rot Sticker's shell (the drawn hurtbox): one
    /// hit per move instance and sticker, any per-move weight provokes it, and
    /// **the attacker gets no hit-stop and no camera kick** from a sticker (the
    /// prop declares zero attacker hit-stop), so a hero who chose to hit it can
    /// run at once.
    fn strike_stickers(
        &mut self,
        scene: &mut SceneSim,
        move_tick: &MoveTick,
        boxed: crate::prototype::Rect,
        attacker: Attacker,
        _tick: u64,
    ) {
        let t = rot_sticker::active();
        let Some(env) = self.env.as_mut() else {
            return;
        };
        for st in &mut env.stickers {
            let Some(s) = st.sticker.as_ref() else {
                continue;
            };
            if !matches!(s.state(), StickerState::Latched | StickerState::Scuttling) {
                continue;
            }
            let Some(shell) = s.hurtbox(t) else {
                continue;
            };
            if !shell.overlaps(boxed) {
                continue;
            }
            if !scene
                .world
                .hero
                .melee
                .record_hit(ENV_HIT_ID_BASE + u32::from(st.prop))
            {
                continue;
            }
            st.pending.push((
                Stimulus::MeleeHit {
                    token: move_tick.instance,
                    player: attacker.player,
                    weight: 1,
                },
                Some(move_tick.spec.id),
            ));
        }
    }

    /// A Magic Missile that overlaps a live prop target ends there (DV-5),
    /// after the goblins, llamas, boss and slot enemies. Returns whether the
    /// bolt was consumed. The projectile is credited to Donut.
    pub(super) fn env_missile_hit(
        &mut self,
        bolt: &super::Bolt,
        scene: &SceneSim,
        tick: u64,
    ) -> bool {
        if self.env.is_none() {
            return false;
        }
        let layout_h = scene.world.layout_height();
        let rect = bolt.rect();
        if self.sticker_missile(bolt, scene) {
            return true;
        }
        let env = self.env.as_ref().expect("checked above");
        let Some((prop, _)) = env
            .live_targets()
            .into_iter()
            .find(|(_, t)| world_rect(layout_h, *t).overlaps(rect))
        else {
            return false;
        };
        self.env_stimulate(
            usize::from(prop),
            Stimulus::Projectile {
                projectile_id: bolt.instance,
                owner: Some(DONUT_HERO),
                weight: 1,
            },
            tick,
        );
        true
    }

    /// HAZ-A2, HAZ-Q29 (rule B): a Magic Missile bolt provokes a sticker when it
    /// overlaps the shell **or its box comes within the sticker's 32 px trigger
    /// circle** with line of sight, the sticker fully in view. The bolt is
    /// consumed and the provoke is credited to Donut (cause `Projectile`).
    /// Returns whether a sticker took the bolt.
    fn sticker_missile(&mut self, bolt: &super::Bolt, scene: &SceneSim) -> bool {
        let t = rot_sticker::active();
        if !t.enabled() || !t.on(t.hazq29_hit_provokes) {
            return false;
        }
        let solids = Self::intact_rects(scene);
        let view = scene.view_x();
        let rect = bolt.rect();
        let trigger = f32::from(t.hazq27_trigger_px);
        let Some(env) = self.env.as_mut() else {
            return false;
        };
        for st in &mut env.stickers {
            let Some(s) = st.sticker.as_ref() else {
                continue;
            };
            if !matches!(s.state(), StickerState::Latched | StickerState::Scuttling) {
                continue;
            }
            let frame = s.frame(t);
            let in_view = frame.left >= view.0 && frame.right() <= view.1;
            let touches = s.hurtbox(t).is_some_and(|shell| shell.overlaps(rect));
            let near =
                in_view && rot_sticker::circle_reaches(s.dome(t), trigger, rect, true, &solids);
            if touches || near {
                st.pending.push((
                    Stimulus::Projectile {
                        projectile_id: bolt.instance,
                        owner: Some(DONUT_HERO),
                        weight: 1,
                    },
                    None,
                ));
                return true;
            }
        }
        false
    }

    /// The intact solids of the scene as plain rectangles (line of sight).
    fn intact_rects(scene: &SceneSim) -> Vec<crate::prototype::Rect> {
        scene
            .world
            .solids
            .iter()
            .filter(|b| b.intact)
            .map(|b| b.rect)
            .collect()
    }

    /// HAZ-A2: the Rot Stickers' step, in the env slot after the charges step
    /// (the sheet's tick order): the charge is the authority for the detonation
    /// and for a dynamite chain (read from the charge, not from
    /// `ChargeChained`, which is not emitted for an owner fill or a cap
    /// deferral); then the stimuli, the proximity, the provoke and the charge
    /// position sync.
    fn step_stickers(&mut self, scene: &SceneSim, tick: u64) {
        let t = rot_sticker::active();
        let n = self.env.as_ref().map_or(0, |e| e.stickers.len());
        if n == 0 {
            return;
        }
        let heroes = self.slot_hero_views(scene);
        let solo = !scene.drive().two_player;
        let players = [Some(0), if solo { Some(0) } else { Some(1) }];
        let solids = Self::intact_rects(scene);
        let view = scene.view_x();
        for i in 0..n {
            let (charge_ix, prop) = {
                let st = &self.env.as_ref().expect("stickers").stickers[i];
                (st.charge, st.prop)
            };
            let Some(charge) = self.charges.get(charge_ix).copied() else {
                continue;
            };
            let env = self.env.as_mut().expect("stickers");
            let st = &mut env.stickers[i];
            let Some(sticker) = st.sticker.as_mut() else {
                continue;
            };
            if sticker.is_spent() {
                st.pending.clear();
                continue;
            }
            if charge.detonated {
                sticker.spend();
                st.pending.clear();
                continue;
            }
            let mut events = Vec::new();
            // A chain lit it, or pulled it forward (the charge is the truth).
            if charge.detonate_tick != 0
                && let Some(p) = sticker.sync_chain(tick, charge.detonate_tick, charge.owner)
            {
                events.push(CombatEvent::RotProvoked {
                    tick,
                    sticker: prop,
                    cause: p.cause,
                    hit: None,
                    player: p.player,
                });
            }
            let mut write_back = None;
            if sticker.state() == StickerState::Warning {
                sticker.hold_warning();
                st.pending.clear();
            } else {
                let frame = sticker.frame(t);
                let in_view = frame.left >= view.0 && frame.right() <= view.1;
                let pending = std::mem::take(&mut st.pending);
                let stimuli: Vec<Stimulus> = pending.iter().map(|(s, _)| *s).collect();
                let out = sticker.step(
                    t,
                    &StickerCtx {
                        tick,
                        heroes: &heroes,
                        players,
                        in_view,
                        solids: &solids,
                        stimuli: &stimuli,
                        missiles: &[],
                    },
                );
                if let Some(p) = out.provoked {
                    let hit = (p.cause == rot_sticker::RotCause::Hit)
                        .then(|| pending.iter().find_map(|(_, m)| *m))
                        .flatten();
                    events.push(CombatEvent::RotProvoked {
                        tick,
                        sticker: prop,
                        cause: p.cause,
                        hit,
                        player: p.player,
                    });
                    write_back = Some((sticker.detonate_tick(), sticker.owner()));
                }
            }
            let (x, foot) = sticker.charge_xy(t);
            let c = &mut self.charges[charge_ix];
            c.x = x;
            c.foot_y = foot;
            if let Some((due, owner)) = write_back {
                c.detonate_tick = due;
                if c.owner.is_none() {
                    c.owner = owner;
                }
            }
            for e in events {
                self.log(e);
            }
        }
    }

    /// HAZ-A2: the acid patches: the drain of a patch whose slime fell, the
    /// hero contacts (one pop per entry) and the launched goblins that landed
    /// in one (once per launch). Slimes, walking goblins and llamas never take
    /// acid (HAZ-Q20, HAZ-Q48).
    fn step_acid(&mut self, scene: &mut SceneSim, tick: u64) {
        let links: Vec<(u16, bool)> = match self.env.as_ref() {
            Some(env) if env.acid.patches().next().is_some() => env
                .acid_links
                .iter()
                .map(|(patch, target)| {
                    let up = self
                        .slot
                        .index_of(*target)
                        .is_none_or(|i| self.slot.enemies[i].alive());
                    (*patch, up)
                })
                .collect(),
            _ => return,
        };
        let layout_h = scene.world.layout_height();
        let env = self.env.as_mut().expect("checked above");
        for (patch, up) in links {
            if !up && let Some(p) = env.acid.get_mut(patch) {
                p.on_slime_defeated();
            }
        }
        env.acid.step();
        // Heroes: the contact is the hero's hurtbox against the live bounds.
        let drive = *scene.drive();
        let mut blows = Vec::new();
        let ids: Vec<u16> = env.acid.patches().map(slime::AcidPatch::id).collect();
        if !self.hero_defeated {
            for id in HeroId::ALL {
                if id != HeroId::Carl && !scene.party_live() {
                    continue;
                }
                let body = layout_rect(layout_h, Self::hurtbox(scene, id));
                let hero = scene.world.body(id);
                let unseen = drive.is_guarded(id)
                    && drive.tuning.solo_offscreen_immunity
                    && scene.is_offscreen(hero.x, f32::from(drive.tuning.offscreen_margin_px));
                let vulnerable = hero.invulnerable_ticks == 0 && !unseen;
                let x = hero.x;
                for patch in &ids {
                    let p = env.acid.get_mut(*patch).expect("a listed patch");
                    if p.contact(id, body, vulnerable) {
                        blows.push(p.pop_blow(id, x));
                    }
                }
            }
        }
        // Launched goblins that came to rest in a patch.
        let mut crushed = Vec::new();
        for (index, g) in self.enemies.iter().enumerate() {
            let Some(launch) = g.launch else { continue };
            if !g.alive() || g.y > g.floor_y + 0.5 {
                continue;
            }
            let body = layout_rect(layout_h, g.body());
            for patch in &ids {
                let p = env.acid.get_mut(*patch).expect("a listed patch");
                if p.launched_landing(launch, body) {
                    crushed.push((index, *patch, g.hit_by));
                }
            }
        }
        for blow in blows {
            self.hurt_hero(scene, blow, tick);
        }
        for (index, patch, credit) in crushed {
            self.acid_hit_goblin(index, patch, credit, tick);
        }
    }

    /// One damage of acid on a goblin (or Engineer) that landed in a patch:
    /// the landing's credit goes to whoever launched it, the cause is
    /// `Environment` (`prop` is the acid patch id).
    fn acid_hit_goblin(&mut self, index: usize, patch: u16, credit: Option<u32>, tick: u64) {
        let hit = EnvHit {
            victim: Victim::enemy(index as u32),
            prop: patch,
            class: HazardClass::Acid,
            damage: 1,
            push_x: 0,
            knock_up: 0,
            from_x: 0,
            initiator: credit,
            activation: 1,
        };
        let g = &mut self.enemies[index];
        if !g.alive() {
            return;
        }
        let hit_by = g.hit_by;
        if hit.damage >= g.health {
            if g.crush() {
                self.env_defeat(index as u32, &hit, hit_by, tick);
            }
            return;
        }
        g.health -= hit.damage;
    }

    /// The victim bodies of this tick in layout px, in a fixed order: the
    /// living heroes, the goblins (and Engineers), the llamas, the slot
    /// enemies.
    fn env_bodies(&self, scene: &SceneSim) -> Vec<(Victim, IRect)> {
        let layout_h = scene.world.layout_height();
        let mut bodies = Vec::new();
        if !self.hero_defeated {
            for id in HeroId::ALL {
                if id == HeroId::Carl || scene.party_live() {
                    bodies.push((
                        Victim::hero(id.index() as u32),
                        layout_rect(layout_h, Self::hurtbox(scene, id)),
                    ));
                }
            }
        }
        for (i, g) in self.enemies.iter().enumerate() {
            if g.alive() {
                bodies.push((Victim::enemy(i as u32), layout_rect(layout_h, g.body())));
            }
        }
        for (i, l) in self.llamas.iter().enumerate() {
            if l.alive() {
                bodies.push((
                    Victim::enemy((self.enemies.len() + i) as u32),
                    layout_rect(layout_h, l.body()),
                ));
            }
        }
        for (id, rect) in self.slot_victim_bodies() {
            bodies.push((Victim::enemy(id), layout_rect(layout_h, rect)));
        }
        bodies
    }

    /// The env step: see the module docs for the order.
    pub(super) fn step_env(&mut self, scene: &mut SceneSim, tick: u64) {
        if self.env.is_none() {
            return;
        }
        // The first step appends the solids (KR6); the sync rewrites `intact`
        // again after the prop step below.
        self.env_sync_solids(scene);
        let layout_h = scene.world.layout_height();
        let bodies = self.env_bodies(scene);
        let out = self
            .env
            .as_mut()
            .expect("checked above")
            .step(tick, &bodies);
        for event in out.events {
            self.log(event);
        }
        debug_assert!(
            out.embedded.iter().all(|v| v.team == Team::Enemy),
            "only enemies are embedded"
        );
        // The remnant-lift rule: the lift is a position write in the same step
        // that makes the remnant solid, so a lifted hero never spends a tick
        // on a remnant that is not yet solid (PR #200 review NB-2).
        for lift in &out.lifts {
            let id = HeroId::ALL[lift.hero as usize];
            let body = scene.world.body_mut(id);
            body.y += lift.up_px as f32;
            if body.vy < 0.0 {
                body.vy = 0.0;
            }
            body.grounded = true;
        }
        let mut slot_hits: Vec<SlotEnvHit> = Vec::new();
        for hit in out.hits {
            match hit.victim.team {
                Team::Hero => self.env_hurt_hero(scene, &hit, tick),
                Team::Enemy => {
                    let id = hit.victim.id as usize;
                    if id < self.enemies.len() {
                        self.env_hit_goblin(id, &hit, tick);
                    } else if id < self.enemies.len() + self.llamas.len() {
                        self.env_hit_llama(id - self.enemies.len(), &hit, tick);
                    } else if self.slot_owns(hit.victim.id) {
                        slot_hits.push(SlotEnvHit {
                            victim: hit.victim.id,
                            source: EnvSource::Prop(hit.prop),
                            class: hit.class,
                            damage: hit.damage,
                            push_x: hit.push_x,
                            initiator: hit.initiator,
                            activation: hit.activation,
                        });
                    }
                }
            }
        }
        if !slot_hits.is_empty() {
            self.slot_env_hits(scene, &slot_hits, tick);
        }
        // BOIL-D: the edges of the tick reach their linked props, after the
        // hits they caused (a landing kills, then it reaches the drum).
        for fire in out.fires {
            self.env_fire(fire, tick);
        }
        self.env_follow_lit_drums(tick);
        // HAZ-A2: the stickers, then the acid.
        self.step_stickers(scene, tick);
        self.step_acid(scene, tick);
        self.env_sync_solids(scene);
        let view = self.env_view(layout_h);
        self.set_slot_env(view);
    }

    /// A hero in a volume: the one hurt path, with the volume's damage and
    /// push. A guarded hero outside the view cannot be hurt in solo (M4, like
    /// the shore block).
    fn env_hurt_hero(&mut self, scene: &mut SceneSim, hit: &EnvHit, tick: u64) {
        let id = HeroId::ALL[hit.victim.id as usize];
        let drive = *scene.drive();
        let x = scene.world.body(id).x;
        if drive.is_guarded(id)
            && drive.tuning.solo_offscreen_immunity
            && scene.is_offscreen(x, f32::from(drive.tuning.offscreen_margin_px))
        {
            return;
        }
        let knock_up = if hit.knock_up > 0 {
            f32::from(hit.knock_up)
        } else {
            STEAM_HOP_PX_S
        };
        self.hurt_hero(
            scene,
            HeroBlow {
                hero: id,
                source: ENV_SOURCE_ID,
                damage: hit.damage,
                from_x: hit.from_x as f32,
                knock_vx: f32::from(hit.push_x),
                knock_vy: knock_up,
            },
            tick,
        );
    }

    fn env_cause(hit: &EnvHit) -> DefeatCause {
        DefeatCause::Environment {
            player: hit.initiator,
            prop: hit.prop,
            class: hit.class,
        }
    }

    fn env_defeat(&mut self, target: u32, hit: &EnvHit, hit_by: Option<u32>, tick: u64) {
        let cause = Self::env_cause(hit);
        self.log(CombatEvent::enemy_defeated(
            tick,
            target,
            cause,
            hit.initiator.or(hit_by),
            None,
        ));
        self.log(CombatEvent::EnvironmentKill {
            tick,
            target,
            prop: hit.prop,
            class: hit.class,
            player: hit.initiator,
        });
        if let Some(slot) = self.flash_tick.get_mut(target as usize) {
            *slot = Some(tick);
        }
    }

    /// A goblin (or Engineer) in a volume: a lethal hit (the landing's 255, or
    /// damage that reaches its health) crushes it in place; otherwise it takes
    /// the damage and staggers, pushed `push_x` (BOILBC-AD4).
    fn env_hit_goblin(&mut self, index: usize, hit: &EnvHit, tick: u64) {
        let g = &mut self.enemies[index];
        if !g.alive() {
            return;
        }
        let hit_by = g.hit_by;
        if hit.damage >= g.health {
            if g.crush() {
                self.env_defeat(index as u32, hit, hit_by, tick);
            }
            return;
        }
        let was_attacking = g.mode == GoblinMode::Attack;
        g.health -= hit.damage;
        let push: i8 = if hit.push_x >= 0 { 1 } else { -1 };
        g.contact_stagger(
            push,
            crate::feel::CONTACT_STAGGER,
            f32::from(hit.push_x).abs(),
            0,
        );
        g.facing = -push;
        if hit.initiator.is_some() {
            g.hit_by = hit.initiator;
        }
        if was_attacking {
            self.log(CombatEvent::GoblinInterrupted {
                tick,
                goblin: index as u32,
            });
        }
    }

    /// A llama in a volume: the same rule as a goblin's, with the llama's hurt
    /// stagger.
    fn env_hit_llama(&mut self, index: usize, hit: &EnvHit, tick: u64) {
        let target = (self.enemies.len() + index) as u32;
        let l = &mut self.llamas[index];
        if !l.alive() {
            return;
        }
        let hit_by = l.hit_by;
        if hit.damage >= l.health {
            if l.crush() {
                self.env_defeat(target, hit, hit_by, tick);
            }
            return;
        }
        l.health -= hit.damage;
        l.mode = LlamaMode::Hurt;
        l.stagger_left = crate::feel::LLAMA_HURT_TICKS;
        l.vx = f32::from(hit.push_x);
        if hit.initiator.is_some() {
            l.hit_by = hit.initiator;
        }
    }

    /// BOIL-D: a flier (a launched goblin, or the body of one a lethal kick
    /// killed) against the props, after the enemy contacts of this tick. The
    /// prop contact target (contacts_cf02.rs calls it for every flier):
    ///
    /// * a **live hit target** (the valve, the brace) the flier's box overlaps
    ///   is struck: the flier flies through it, nothing stops it;
    /// * an **env solid**, identified by the index range of the runtime sync
    ///   ([`EnvRuntime::solid_range`], KR6/KR7), is struck when the flier's box
    ///   overlaps it **or the flier was stopped by a solid this tick and the
    ///   solid lies within the distance that tick's flight covered** (a goblin
    ///   a solid stops stays outside it, `wall_stop`; BOILD-AD2). What a struck
    ///   solid does is its kind's: the drum takes the launch, the landed
    ///   remnant takes nothing (a settled mass is no trigger). The match over
    ///   [`EnvSolidKind`] is exhaustive: a later solid kind (the Ring's
    ///   shutter slices) must decide what a launched body does to it. A solid
    ///   outside the range (a wall) is never a prop.
    ///
    /// One stimulus per (launch, prop): the machine dedupes launch ids, and a
    /// stimulus that is ignored logs nothing, so a flier that rests against a
    /// prop for many ticks produces one event at most. The impulse is the
    /// flier's speed at the start of the tick, or the speed a solid stopped
    /// it at, whichever is higher, credited to `initiator` (the hero whose
    /// move launched it).
    pub(super) fn env_launched_contacts(
        &mut self,
        scene: &SceneSim,
        flier: &LaunchedBody,
        tick: u64,
    ) {
        let Some(env) = self.env.as_ref() else {
            return;
        };
        let layout_h = scene.world.layout_height();
        let impulse = flier.speed.max(flier.stop.unwrap_or(0.0)).round();
        let impulse_px_s = impulse.clamp(0.0, f32::from(u16::MAX)) as u16;
        let mut struck: Vec<u16> = Vec::new();
        for (prop, target) in env.live_targets() {
            if world_rect(layout_h, target).overlaps(flier.body) {
                struck.push(prop);
            }
        }
        let range = env.solid_range();
        // One tick of the stopped flight, rounded up, plus a pixel.
        let reach = flier
            .stop
            .map_or(0.0, |speed| (speed * TICK_SECONDS).ceil() + 1.0);
        let probe = Rect {
            left: flier.body.left - reach,
            width: flier.body.width + 2.0 * reach,
            ..flier.body
        };
        for i in range.clone() {
            let Some(block) = scene.world.solids.get(i) else {
                continue;
            };
            if !block.intact || !probe.overlaps(block.rect) {
                continue;
            }
            let solid = env.solids()[i - range.start];
            match solid.kind {
                EnvSolidKind::Drum => struck.push(solid.prop),
                // The landed weight is a wall, not a trigger; a lock, a gate, a soffit
                // and the exit gate are room solids, never a prop to strike.
                EnvSolidKind::Remnant
                | EnvSolidKind::Lock { .. }
                | EnvSolidKind::Gate { .. }
                | EnvSolidKind::Soffit
                | EnvSolidKind::ExitGate => {}
            }
        }
        struck.sort_unstable();
        struck.dedup();
        for prop in struck {
            self.env_stimulate(
                usize::from(prop),
                Stimulus::LaunchedBody {
                    launch_id: flier.launch,
                    impulse_px_s,
                    initiator: flier.initiator,
                },
                tick,
            );
        }
    }

    /// Whether prop `index` is a drum whose charge is lit and has not blown.
    fn env_drum_is_lit(&self, index: usize) -> bool {
        let Some(env) = self.env.as_ref() else {
            return false;
        };
        u16::try_from(index)
            .ok()
            .and_then(|p| env.drum_charge(p))
            .is_some_and(|c| !self.charges[c].detonated && self.charges[c].detonate_tick > 0)
    }

    /// BOIL-D: a blast that breaks structures reaches the props it can set off
    /// (CP-Q35): each is a [`BlastTarget`] (the `PropTarget` below), judged with
    /// the same outer circle and line of sight as the shore and the rack strut.
    /// The blast's owner is the initiator and its chain depth is the prop's
    /// trigger depth (PR #200 review NB-4). The env-solid range is never read
    /// as a target: a prop answers for itself (KC17).
    pub(super) fn blast_props(
        &mut self,
        scene: &SceneSim,
        charge: &BlastCharge,
        blast_id: u32,
        tick: u64,
    ) {
        let Some(props) = self.env.as_deref().map(EnvRuntime::props) else {
            return;
        };
        let layout_h = scene.world.layout_height();
        let solids = &scene.world.solids;
        let mut logged = Vec::new();
        let mut set_off = Vec::new();
        for index in 0..props {
            // A drum a chain reaction already lit is the chain's (CP-Q34 gave
            // its charge the depth and the credit): the machine follows the
            // charge in the env step, so the blast does not offer it again.
            if self.env_drum_is_lit(index) {
                continue;
            }
            let Some(env) = self.env.as_mut() else {
                return;
            };
            let mut target = PropTarget {
                env,
                index,
                layout_h,
                tick,
                offer: None,
                events: Vec::new(),
            };
            blast_break(charge, blast_id, solids, &mut target);
            if let Some(Offer::Triggered { initiator, depth }) = target.offer {
                set_off.push((index, initiator, depth));
            }
            logged.extend(target.events);
        }
        for event in logged {
            self.log(event);
        }
        for (index, initiator, depth) in set_off {
            self.env_arm_drum(index, tick, initiator, depth);
        }
    }

    /// ` env=...` while the room has props, else empty.
    pub(super) fn env_token(&self) -> String {
        self.env.as_ref().map_or_else(String::new, |e| e.token())
    }

    /// The hero's feet y in layout px (tests and fixtures).
    pub fn env_hero_foot_layout(&self, scene: &SceneSim, id: HeroId) -> i32 {
        let body = scene.world.body(id);
        scene.world.layout_height() - (body.y - HERO_HALF_HEIGHT).round() as i32
    }
}

/// A flier as the prop contact target sees it (BOIL-D): its box (world px, y
/// up), the launch it flies on, the hero credited with it and its speeds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LaunchedBody {
    pub body: Rect,
    pub launch: u32,
    pub initiator: Option<u32>,
    /// |vx| at the start of the tick's contacts.
    pub speed: f32,
    /// The speed a solid stopped the flier at this tick, if one did
    /// (`Goblin::wall_stop`).
    pub stop: Option<f32>,
}

/// A prop as a [`BlastTarget`] of one blast (BOIL-D, CP-Q35): the box a blast
/// must reach, and the break call, which offers a `Blast` stimulus carrying the
/// blast's id, owner and chain depth. It collects the events and the offer for
/// the caller, which logs them and lights a drum's charge.
struct PropTarget<'a> {
    env: &'a mut EnvRuntime,
    index: usize,
    layout_h: i32,
    tick: u64,
    offer: Option<Offer>,
    events: Vec<CombatEvent>,
}

impl BlastTarget for PropTarget<'_> {
    fn blast_footprint(&self) -> Option<Rect> {
        self.env
            .blast_targets()
            .into_iter()
            .find(|(prop, _)| usize::from(*prop) == self.index)
            .map(|(_, rect)| world_rect(self.layout_h, rect))
    }

    fn break_by_blast(&mut self, blast: BlastBreak) -> bool {
        let stimulus = Stimulus::Blast {
            blast_id: blast.blast_id,
            owner: blast.owner,
            depth: blast.depth,
        };
        let Some((offer, events)) = self.env.offer(self.index, stimulus, self.tick) else {
            return false;
        };
        self.events.extend(events);
        self.offer = Some(offer);
        matches!(offer, Offer::Triggered { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blast::BlastProfile;
    use crate::boss_scene::Pilot;
    use crate::combat::EnemyAi;
    use crate::combat_scene::{EnemySpawn, Fixture};
    use crate::donut_mana::DonutSetup;
    use crate::encounter::{BOILER_ENCOUNTER_PATH, RosterKind, load};
    use crate::env_hazard::Phase as HazardPhase;
    use crate::env_prop::StimulusKind;
    use crate::f01_boiler::{self, BoilerRun};
    use crate::input::RawInput;
    use crate::party::PROVISIONAL_TUNING;
    use crate::party_scene::{PartyFixture, PartyFixtureSim};
    use crate::scene_enemy::SlotSet;
    use crate::scene_enemy::testing::Dummy;
    use std::path::PathBuf;

    const VALVE: usize = 0;
    const VALVE_X: f32 = 460.0;
    const BRACE_X: f32 = 740.0;
    const WEIGHT: usize = 1;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn boiler() -> BoilerRun {
        f01_boiler::launch(&root()).expect("the Boiler launches")
    }

    fn base(goblins: &[i32]) -> &'static Fixture {
        let enemies: Vec<EnemySpawn> = goblins
            .iter()
            .map(|&x| EnemySpawn {
                x,
                ai: EnemyAi::Passive,
            })
            .collect();
        Box::leak(Box::new(Fixture {
            name: "test_boiler_env",
            summary: "test",
            seed: 7,
            carl_x: 112,
            enemies: Box::leak(enemies.into_boxed_slice()),
            shore_dx: None,
            boss: None,
            engineers: &[],
            llamas: &[],
            charges: &[],
            script: &[],
            ticks: 900,
        }))
    }

    fn sim_with(goblins: &[i32], two_player: bool, dummies: Vec<Dummy>) -> PartyFixtureSim {
        let run = boiler();
        let base = base(goblins);
        let mut scene = SceneSim::new_at(&run.kit, 360, base.carl_x).unwrap();
        let mut combat = CombatState::new(base, &run.kit, &scene)
            .unwrap()
            .with_donut(DonutSetup::unlocked(8))
            .with_demo(run.rules)
            .with_cf02()
            .with_env(f01_boiler::env_runtime(&run).unwrap());
        if !dummies.is_empty() {
            let set = SlotSet::new(dummies.into_iter().map(Dummy::boxed).collect(), false).unwrap();
            combat = combat.with_slot_enemies(set).unwrap();
        }
        combat.sync_env(&mut scene);
        let fixture: &'static PartyFixture = Box::leak(Box::new(PartyFixture {
            base,
            two_player,
            tuning: PROVISIONAL_TUNING,
            script: &[],
            pilot: Pilot::None,
            donut: DonutSetup::unlocked(8),
        }));
        PartyFixtureSim::from_parts(scene, combat, fixture)
    }

    fn sim(goblins: &[i32]) -> PartyFixtureSim {
        sim_with(goblins, false, Vec::new())
    }

    const ATTACK: RawInput = RawInput {
        attack: true,
        ..RawInput::NONE
    };

    fn step(s: &mut PartyFixtureSim, carl: RawInput) {
        s.step_slots([carl, RawInput::default()]);
    }

    fn tick(s: &PartyFixtureSim) -> u64 {
        s.scene.world.tick
    }

    fn put_carl(s: &mut PartyFixtureSim, x: f32) {
        s.scene.world.hero.x = x;
        s.scene.world.hero.facing = 1;
    }

    fn events(s: &PartyFixtureSim) -> Vec<CombatEvent> {
        s.combat.events.clone()
    }

    fn phase_tick(s: &PartyFixtureSim, prop: u16, want: PropPhase) -> Option<u64> {
        phase_tick_after(s, 0, prop, want)
    }

    /// The first such phase in the events after the first `from`.
    fn phase_tick_after(
        s: &PartyFixtureSim,
        from: usize,
        prop: u16,
        want: PropPhase,
    ) -> Option<u64> {
        events(s)
            .iter()
            .skip(from)
            .filter_map(|e| match *e {
                CombatEvent::PropPhase {
                    tick,
                    prop: p,
                    phase,
                    ..
                } if p == prop && phase == want => Some(tick),
                _ => None,
            })
            .next()
    }

    /// Two jabs 30 ticks apart on a prop target `reach` px ahead of Carl's
    /// column `x`, until the prop's `Warning`/`Released` phase. Returns the
    /// trigger tick `T` (the sim stands on tick `T`).
    fn trigger_by_jabs(s: &mut PartyFixtureSim, x: f32, prop: u16, phase: PropPhase) -> u64 {
        put_carl(s, x);
        let from = events(s).len();
        for t in 0..200 {
            let press = t % 30 == 0 && t / 30 < 2;
            step(s, if press { ATTACK } else { RawInput::NONE });
            if let Some(at) = phase_tick_after(s, from, prop, phase) {
                return at;
            }
        }
        panic!("two jabs never set prop {prop} off: {:?}", events(s));
    }

    fn hurts_of(s: &PartyFixtureSim, hero: HeroId) -> Vec<(u64, u32, u8)> {
        events(s)
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

    fn defeats(s: &PartyFixtureSim) -> Vec<(u64, u32, DefeatCause)> {
        events(s)
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

    // ---- the valve and the steam (BOIL-B wire)

    #[test]
    fn two_distinct_jabs_set_the_valve_off_and_a_third_does_nothing() {
        let mut s = sim(&[]);
        let t = trigger_by_jabs(&mut s, VALVE_X, VALVE as u16, PropPhase::Warning);
        let hits: Vec<_> = events(&s)
            .into_iter()
            .filter_map(|e| match e {
                CombatEvent::PropHit {
                    prop: 0,
                    progress,
                    kind,
                    player,
                    ..
                } => Some((progress, kind, player)),
                _ => None,
            })
            .collect();
        assert_eq!(
            hits,
            [
                (1, StimulusKind::MeleeHit, Some(0)),
                (2, StimulusKind::MeleeHit, Some(0))
            ],
            "one counted hit per move instance"
        );
        assert_eq!(
            s.combat.env().unwrap().valve_state(VALVE),
            Some(ValveState::Warning)
        );
        // A third and a fourth jab on a used valve count for nothing.
        for i in 0..60 {
            step(&mut s, if i % 30 == 0 { ATTACK } else { RawInput::NONE });
        }
        let counted = events(&s)
            .iter()
            .filter(|e| matches!(e, CombatEvent::PropHit { prop: 0, .. }))
            .count();
        assert_eq!(counted, 2, "T = {t}");
    }

    /// The steam's exact timeline in game order (stimulus, then the step): the
    /// warning logs on T, the vent opens on T + 23, the steam is active on
    /// T + 23 ..= T + 94 and the valve is spent on T + 95.
    #[test]
    fn the_steam_window_runs_t_plus_23_to_t_plus_94_and_agrees_with_the_valve() {
        let mut s = sim(&[]);
        let t = trigger_by_jabs(&mut s, VALVE_X, VALVE as u16, PropPhase::Warning);
        assert_eq!(tick(&s), t);
        let mut venting = None;
        let mut spent = None;
        for _ in 0..120 {
            step(&mut s, RawInput::NONE);
            let env = s.combat.env().unwrap();
            let steam = env.hazards().get(1).unwrap().phase();
            let valve = env.valve_state(VALVE).unwrap();
            // One timeline: warning = telegraph, venting = active.
            assert!(
                matches!(
                    (valve, steam),
                    (ValveState::Warning, HazardPhase::Telegraph)
                        | (ValveState::Venting, HazardPhase::Active)
                        | (ValveState::Spent, HazardPhase::Spent)
                ),
                "{valve:?} vs {steam:?} on tick {}",
                tick(&s)
            );
            if valve == ValveState::Venting && venting.is_none() {
                venting = Some(tick(&s));
            }
            if valve == ValveState::Spent && spent.is_none() {
                spent = Some(tick(&s));
            }
        }
        assert_eq!(venting, Some(t + 23));
        assert_eq!(spent, Some(t + 95));
        assert_eq!(phase_tick(&s, 0, PropPhase::Venting), Some(t + 23));
        assert_eq!(phase_tick(&s, 0, PropPhase::Spent), Some(t + 95));
    }

    /// Carl stands in the steam for the one tick `T + k` only (and outside on
    /// every other): the hurts he took.
    fn carl_in_steam_on(k: u64) -> Vec<(u64, u32, u8)> {
        let mut s = sim(&[]);
        let t = trigger_by_jabs(&mut s, VALVE_X, VALVE as u16, PropPhase::Warning);
        put_carl(&mut s, 600.0);
        while tick(&s) < t + k - 1 {
            step(&mut s, RawInput::NONE);
        }
        put_carl(&mut s, 720.0);
        step(&mut s, RawInput::NONE);
        put_carl(&mut s, 600.0);
        for _ in 0..3 {
            step(&mut s, RawInput::NONE);
        }
        hurts_of(&s, HeroId::Carl)
    }

    #[test]
    fn the_steam_hurts_a_hero_on_its_first_and_last_active_tick_and_not_outside() {
        // Both edges: T + 22 (still the telegraph) and T + 95 (spent) are free;
        // T + 23 (first active) and T + 94 (last active) hurt.
        assert!(carl_in_steam_on(22).is_empty(), "the warning is a tell");
        let first = carl_in_steam_on(23);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].1, ENV_SOURCE_ID);
        let last = carl_in_steam_on(94);
        assert_eq!(last.len(), 1, "the last active tick still hurts");
        assert!(carl_in_steam_on(95).is_empty(), "spent: nothing");
        // The hit is the volume's 1 damage.
        assert_eq!(first[0].2, crate::combat::HERO_HEALTH - 1);
    }

    #[test]
    fn steam_staggers_a_goblin_once_per_burst_and_never_credits_it_twice() {
        let mut s = sim(&[720]);
        let t = trigger_by_jabs(&mut s, VALVE_X, VALVE as u16, PropPhase::Warning);
        let health = s.combat.enemies[0].health;
        for _ in 0..110 {
            step(&mut s, RawInput::NONE);
        }
        // 2 health, 1 damage, one token per activation: it survives with 1.
        assert_eq!(s.combat.enemies[0].health, health - 1, "T = {t}");
        assert!(s.combat.enemies[0].alive());
        assert!(defeats(&s).is_empty());
        assert_eq!(s.combat.enemies[0].hit_by, Some(0), "the initiator is kept");
    }

    /// K3 / DM-Q18: the steam hurts every hero by its data, Donut too, once
    /// each, and nobody can dodge it.
    #[test]
    fn the_steam_hurts_both_heroes_once_each_in_two_player() {
        let mut s = sim_with(&[], true, Vec::new());
        let t = trigger_by_jabs(&mut s, VALVE_X, VALVE as u16, PropPhase::Warning);
        put_carl(&mut s, 700.0);
        s.scene.world.donut_body.x = 730.0;
        while tick(&s) < t + 40 {
            step(&mut s, RawInput::NONE);
        }
        let carl = hurts_of(&s, HeroId::Carl);
        let donut = hurts_of(&s, HeroId::Donut);
        assert_eq!(carl.len(), 1, "{carl:?}");
        assert_eq!(donut.len(), 1, "{donut:?}");
        assert_eq!((carl[0].0, carl[0].1), (t + 23, ENV_SOURCE_ID));
        assert_eq!((donut[0].0, donut[0].1), (t + 23, ENV_SOURCE_ID));
        assert!(
            !events(&s)
                .iter()
                .any(|e| matches!(e, CombatEvent::HeroEvaded { .. })),
            "never evadable"
        );
    }

    // ---- the hanging weight (BOIL-C wire)

    /// Carl's jab reaches 16..22 px ahead of his column, 29..35 px up: the column
    /// 20 px left of the target's left edge hits it.

    #[test]
    fn the_brace_releases_on_two_jabs_and_lands_on_r_plus_31() {
        let mut s = sim(&[]);
        let r = trigger_by_jabs(&mut s, BRACE_X, WEIGHT as u16, PropPhase::Released);
        let start = {
            // The solids were appended on the first env step.
            let range = s.combat.env().unwrap().solid_range();
            assert_eq!(range.len(), 2);
            range.start
        };
        let remnant = start; // the weight's remnant is solid 0
        let mut falling = None;
        let mut landed = None;
        for _ in 0..40 {
            step(&mut s, RawInput::NONE);
            let up = s.scene.world.solids[remnant].intact;
            if tick(&s) < r + 31 {
                assert!(!up, "no remnant before the landing (tick {})", tick(&s));
            }
            if falling.is_none() {
                falling = phase_tick(&s, 1, PropPhase::Falling);
            }
            if landed.is_none() {
                landed = phase_tick(&s, 1, PropPhase::Landed);
                if landed.is_some() {
                    assert!(up, "the remnant stands from the landing step on");
                    assert_eq!(tick(&s), r + 31);
                }
            }
        }
        assert_eq!(falling, Some(r + 23));
        assert_eq!(landed, Some(r + 31));
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Landed)
        );
    }

    #[test]
    fn the_landing_kills_only_the_goblin_under_the_footprint_and_credits_carl() {
        // g0 under the footprint (784..848), g1 just outside (body 868..892).
        let mut s = sim(&[816, 880]);
        let r = trigger_by_jabs(&mut s, BRACE_X, WEIGHT as u16, PropPhase::Released);
        for _ in 0..40 {
            step(&mut s, RawInput::NONE);
        }
        assert_eq!(
            defeats(&s),
            [(
                r + 31,
                0,
                DefeatCause::Environment {
                    player: Some(0),
                    prop: 1,
                    class: HazardClass::FallingMass
                }
            )],
            "one death, on the landing tick, credited to the initiator"
        );
        assert!(events(&s).contains(&CombatEvent::EnvironmentKill {
            tick: r + 31,
            target: 0,
            prop: 1,
            class: HazardClass::FallingMass,
            player: Some(0),
        }));
        assert!(
            s.combat.enemies[1].alive(),
            "the goblin outside is untouched"
        );
        assert!(hurts_of(&s, HeroId::Carl).is_empty());
        // Instigator and force follow the cause (catastrophic, like the block).
        assert!(events(&s).iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                force: crate::combat::ForceTier::Catastrophic,
                instigator: Some(0),
                ..
            }
        )));
    }

    #[test]
    fn a_hero_in_the_footprint_is_hurt_once_and_lifted_onto_the_remnant() {
        let mut s = sim(&[]);
        let r = trigger_by_jabs(&mut s, BRACE_X, WEIGHT as u16, PropPhase::Released);
        // Carl walks under the weight (the tell is honest: 31 ticks).
        while tick(&s) < r + 10 {
            step(&mut s, RawInput::NONE);
        }
        put_carl(&mut s, 800.0);
        while tick(&s) < r + 31 {
            step(&mut s, RawInput::NONE);
        }
        let hurts = hurts_of(&s, HeroId::Carl);
        assert_eq!(hurts.len(), 1, "{hurts:?}");
        assert_eq!((hurts[0].0, hurts[0].1), (r + 31, ENV_SOURCE_ID));
        // The remnant is 32 px: the feet end on its top, x untouched.
        assert_eq!(s.combat.env_hero_foot_layout(&s.scene, HeroId::Carl), 548);
        assert_eq!(s.scene.world.hero.x, 800.0, "never a sideways snap");
        // The hurt hops him (a landing's knock is 140 px/s up), never below
        // the remnant's top; he settles on it.
        for _ in 0..40 {
            step(&mut s, RawInput::NONE);
            assert!(
                s.combat.env_hero_foot_layout(&s.scene, HeroId::Carl) <= 548,
                "never sunk into the remnant"
            );
        }
        assert_eq!(s.combat.env_hero_foot_layout(&s.scene, HeroId::Carl), 548);
        assert_eq!(hurts_of(&s, HeroId::Carl).len(), 1, "one hit per landing");
    }

    #[test]
    fn a_remnant_never_lifts_an_enemy_and_a_body_outside_is_not_embedded() {
        let run = boiler();
        let mut env = f01_boiler::env_runtime(&run).unwrap();
        let bodies = [
            (Victim::enemy(0), IRect::new(800, 528, 24, 52)), // inside the footprint
            (Victim::enemy(1), IRect::new(900, 528, 24, 52)), // outside
        ];
        env.offer(
            WEIGHT,
            Stimulus::Blast {
                blast_id: 1,
                owner: Some(0),
                depth: 0,
            },
            1,
        )
        .unwrap();
        let mut landed = None;
        for t in 1..=32 {
            let out = env.step(t, &bodies);
            if out.events.iter().any(|e| {
                matches!(
                    e,
                    CombatEvent::PropPhase {
                        phase: PropPhase::Landed,
                        ..
                    }
                )
            }) {
                landed = Some(out);
            }
        }
        let out = landed.expect("landed on R + 31");
        assert!(out.lifts.is_empty(), "enemies are never lifted");
        assert_eq!(out.embedded, [Victim::enemy(0)], "32 px is no step-over");
        assert_eq!(out.hits.len(), 1);
    }

    #[test]
    fn a_teleport_is_not_a_sweep_and_continuous_motion_is() {
        let run = boiler();
        let go = |path: &dyn Fn(u64) -> IRect| {
            let mut env = f01_boiler::env_runtime(&run).unwrap();
            env.offer(
                WEIGHT,
                Stimulus::Blast {
                    blast_id: 1,
                    owner: None,
                    depth: 0,
                },
                1,
            )
            .unwrap();
            let mut hits = 0;
            for t in 1..=32 {
                hits += env.step(t, &[(Victim::hero(0), path(t))]).hits.len();
            }
            hits
        };
        // A warp across the footprint on the landing tick (x 600 -> 1000, past
        // 784..848): prev == cur, so nothing is swept.
        let warp = go(&|t| IRect::new(if t < 32 { 600 } else { 1000 }, 528, 24, 52));
        assert_eq!(warp, 0, "a jump of 400 px is a teleport");
        // A body that walks out of the footprint on the landing tick is still
        // hit (the swept volume, BOILC-AD7): 8 px per tick is continuous.
        let out = go(&|t| IRect::new(if t < 32 { 840 } else { 848 }, 528, 24, 52));
        assert_eq!(out, 1);
    }

    // ---- the runtime solids (KR6)

    #[test]
    fn the_solids_are_appended_once_as_timber_and_follow_the_props() {
        let mut s = sim(&[]);
        // Appended at setup (`sync_env`), before the first tick.
        let total = s.scene.world.solids.len();
        let range = s.combat.env().unwrap().solid_range();
        assert_eq!(range, total - 2..total);
        for _ in 0..6 {
            step(&mut s, RawInput::NONE);
        }
        assert_eq!(s.scene.world.solids.len(), total, "idempotent");
        assert_eq!(s.combat.env().unwrap().solid_range(), range);
        let blocks = &s.scene.world.solids[range.clone()];
        assert!(blocks.iter().all(|b| b.material == Material::Timber));
        assert!(!blocks[0].intact, "the remnant is not there yet");
        assert!(blocks[1].intact, "the drum stands (optional, solid_intact)");
        // Geometry: the remnant (784, 548, 64, 32) in world px (y up, 672 high).
        let r = blocks[0].rect;
        assert_eq!(
            (r.left, r.bottom, r.width, r.height),
            (784.0, 92.0, 64.0, 32.0)
        );
        // The slot enemies read the same range.
        assert_eq!(s.combat.slot_env().solid_range, range);
        assert_eq!(
            s.combat.slot_env().targets.len(),
            2,
            "the valve and the brace"
        );
    }

    #[test]
    fn a_restart_and_a_respawn_restore_the_props_and_the_solids() {
        let mut s = sim(&[]);
        let r = trigger_by_jabs(&mut s, BRACE_X, WEIGHT as u16, PropPhase::Released);
        while tick(&s) < r + 32 {
            step(&mut s, RawInput::NONE);
        }
        let range = s.combat.env().unwrap().solid_range();
        assert!(s.scene.world.solids[range.start].intact);
        assert_eq!(s.combat.slot_env().targets.len(), 1, "the brace is used up");
        s.combat.respawn(&mut s.scene);
        assert_eq!(
            s.combat.slot_env().targets.len(),
            2,
            "a slot enemy sees the fresh props at once, not the last attempt's"
        );
        assert_eq!(s.combat.slot_env().solid_range, range);
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Supported)
        );
        assert!(
            !s.scene.world.solids[range.start].intact,
            "the remnant is gone at once, not a tick later"
        );
        // The brace can be used again, with the same timeline.
        let again = trigger_by_jabs(&mut s, BRACE_X, WEIGHT as u16, PropPhase::Released);
        assert!(again > r);
        s.combat.restart(&mut s.scene);
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Supported)
        );
        assert!(!s.scene.world.solids[range.start].intact);
    }

    #[test]
    fn the_same_script_gives_the_same_hash_and_the_env_is_hashed() {
        let play = |jabs: bool| {
            let mut s = sim(&[816]);
            put_carl(&mut s, BRACE_X);
            for t in 0..120 {
                let press = jabs && t % 30 == 0 && t / 30 < 2;
                step(&mut s, if press { ATTACK } else { RawInput::NONE });
            }
            s.combat.state_hash(&s.scene)
        };
        assert_eq!(play(true), play(true), "deterministic");
        assert_ne!(play(true), play(false), "the prop state is in the hash");
    }

    /// CP-Q35 / KC17: a blast whose profile `breaks_structures` breaks the shore
    /// and the rack strut only (each a `BlastTarget`); the env solids are never
    /// targets of it. A prop is its own target (BOIL-D adds the `BlastTarget`
    /// impl). Pinned here: a structure-breaking blast on the drum and the
    /// remnant's columns leaves the env range, its `intact` flags and its
    /// geometry exactly as they were.
    /// A structure-breaking blast on the drum and the remnant's columns; the
    /// env solids stay as they were. With `landed` the weight has come down
    /// first, so the remnant is intact too.
    fn blast_never_touches_env_solids(landed: bool) {
        use crate::blast::{BlastCharge, BlastProfile};
        let mut s = sim(&[]);
        if landed {
            let r = trigger_by_jabs(&mut s, BRACE_X, WEIGHT as u16, PropPhase::Released);
            while tick(&s) < r + 33 {
                step(&mut s, RawInput::NONE);
            }
        }
        step(&mut s, RawInput::NONE);
        let range = s.combat.env().unwrap().solid_range();
        let state = |s: &PartyFixtureSim| -> Vec<_> {
            s.scene.world.solids[range.clone()]
                .iter()
                .map(|b| (b.rect, b.intact, b.material))
                .collect()
        };
        let before = state(&s);
        assert_eq!(
            before[0].1, landed,
            "the remnant stands only after the landing"
        );
        let foot = s.scene.world.hero_foot_y();
        let profile = BlastProfile {
            core_radius: 20.0,
            outer_radius: 64.0,
            los: true,
            breaks_structures: true,
            ..BlastProfile::QA
        };
        assert!(!profile.is_qa());
        let t = tick(&s) + 3;
        for x in [864.0, 816.0] {
            s.combat
                .push_authored_charge(BlastCharge::new(x, foot, t, Some(0)).with_profile(profile));
        }
        for _ in 0..8 {
            step(&mut s, RawInput::NONE);
        }
        let blasts = events(&s)
            .iter()
            .filter(|e| matches!(e, CombatEvent::Blast { .. }))
            .count();
        assert_eq!(blasts, 2, "both charges went off");
        assert_eq!(s.combat.env().unwrap().solid_range(), range);
        assert_eq!(
            before,
            state(&s),
            "the env solids are never a blast's structure"
        );
    }

    #[test]
    fn a_structure_breaking_blast_never_touches_the_env_solids() {
        blast_never_touches_env_solids(false);
    }

    #[test]
    fn a_structure_breaking_blast_never_touches_the_landed_remnant_either() {
        blast_never_touches_env_solids(true);
    }

    /// Tick 1 of a first attempt equals tick 1 of a restart: the solids are in
    /// the world at setup (`sync_env`), as they are after a restart.
    #[test]
    fn the_first_attempt_and_a_restart_start_from_the_same_solids() {
        let solids = |s: &PartyFixtureSim| -> Vec<_> {
            s.scene
                .world
                .solids
                .iter()
                .map(|b| (b.rect, b.intact, b.material))
                .collect()
        };
        let mut fresh = sim(&[]);
        let at_setup = solids(&fresh);
        assert_eq!(fresh.combat.env().unwrap().solid_range().len(), 2);
        step(&mut fresh, RawInput::NONE);
        assert_eq!(solids(&fresh), at_setup, "tick 1 appends nothing");
        let mut again = sim(&[]);
        let r = trigger_by_jabs(&mut again, BRACE_X, WEIGHT as u16, PropPhase::Released);
        while tick(&again) < r + 33 {
            step(&mut again, RawInput::NONE);
        }
        again.combat.restart(&mut again.scene);
        assert_eq!(solids(&again), at_setup, "a restart is the setup state");
        step(&mut fresh, RawInput::NONE);
        step(&mut again, RawInput::NONE);
        assert_eq!(solids(&again), solids(&fresh));
    }

    // ---- stimuli beyond melee

    #[test]
    fn a_magic_missile_on_the_support_counts_for_donut_and_a_used_prop_is_passed() {
        let mut s = sim(&[]);
        let bolt = super::super::Bolt {
            instance: 5,
            x: 750.0,
            y: 110.0,
            dir: 1,
            travelled: 0.0,
        };
        let t = tick(&s) + 1;
        assert!(
            s.combat.env_missile_hit(&bolt, &s.scene, t),
            "the first bolt ends"
        );
        let second = super::super::Bolt {
            instance: 6,
            ..bolt
        };
        assert!(s.combat.env_missile_hit(&second, &s.scene, t + 1));
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Released)
        );
        let credited: Vec<_> = events(&s)
            .into_iter()
            .filter_map(|e| match e {
                CombatEvent::PropHit {
                    kind: StimulusKind::Projectile,
                    player,
                    progress,
                    ..
                } => Some((progress, player)),
                _ => None,
            })
            .collect();
        assert_eq!(
            credited,
            [(1, Some(1)), (2, Some(1))],
            "Donut, two distinct strikes"
        );
        let third = super::super::Bolt {
            instance: 7,
            ..bolt
        };
        assert!(
            !s.combat.env_missile_hit(&third, &s.scene, t + 2),
            "a released brace is passed"
        );
        // A bolt that misses (above the 40 px target) does nothing.
        let high = super::super::Bolt {
            instance: 8,
            y: 200.0,
            ..bolt
        };
        assert!(!s.combat.env_missile_hit(&high, &s.scene, t + 3));
    }

    #[test]
    fn a_launched_body_and_the_env_stimulate_entry_point_reach_the_valve() {
        let mut s = sim(&[]);
        let slow = Stimulus::LaunchedBody {
            launch_id: 1,
            impulse_px_s: 150,
            initiator: Some(0),
        };
        assert!(matches!(
            s.combat.env_stimulate(VALVE, slow, 1),
            Some(Offer::Ignored(_))
        ));
        let fast = Stimulus::LaunchedBody {
            launch_id: 2,
            impulse_px_s: 170,
            initiator: Some(0),
        };
        assert!(matches!(
            s.combat.env_stimulate(VALVE, fast, 2),
            Some(Offer::Triggered {
                initiator: Some(0),
                ..
            })
        ));
        assert_eq!(s.combat.env_stimulate(9, fast, 3), None, "no such prop");
    }

    // ---- the slot hooks (ENEMY-SLOT)

    #[test]
    fn a_slot_enemy_reads_the_env_solids_and_takes_the_landing_and_the_steam() {
        let foot = {
            let run = boiler();
            crate::corridor::SceneSim::new_at(&run.kit, 360, 112)
                .unwrap()
                .world
                .hero_foot_y()
        };
        let under = Dummy::new(crate::scene_enemy::SLOT_ID_MIN, 816.0, foot);
        let log = under.log.clone();
        let mut s = sim_with(&[], false, vec![under]);
        let r = trigger_by_jabs(&mut s, BRACE_X, WEIGHT as u16, PropPhase::Released);
        for _ in 0..40 {
            step(&mut s, RawInput::NONE);
        }
        let log = log.lock().unwrap();
        // The step context carries the env solid range and the hit targets.
        let last = log.seen.last().unwrap();
        assert_eq!(last.2, 2, "two env solids in range");
        let env_hits: Vec<_> = log
            .env
            .iter()
            .map(|(t, h)| (*t, h.source, h.class, h.damage))
            .collect();
        assert_eq!(
            env_hits,
            [(r + 31, EnvSource::Prop(1), HazardClass::FallingMass, 255)],
            "one landing hit through slot_env_hits"
        );
        assert_eq!(log.env[0].1.initiator, Some(0));
        assert!(matches!(
            log.env[0].1.cause(),
            DefeatCause::Environment {
                player: Some(0),
                prop: 1,
                class: HazardClass::FallingMass
            }
        ));
    }

    // ---- the build rules

    #[test]
    fn an_unsupported_prop_kind_is_an_error_not_a_silent_skip() {
        use crate::ring_fixtures::COURT_ENCOUNTER_PATH;
        let court = load(&root(), COURT_ENCOUNTER_PATH).unwrap();
        let err = EnvRuntime::new(&court.encounter).expect_err("the court has a shutter");
        assert!(err.contains("RING"), "{err}");
    }

    #[test]
    fn a_roster_that_may_survive_the_landing_in_a_tall_remnant_is_refused() {
        let mut enc = load(&root(), BOILER_ENCOUNTER_PATH).unwrap().encounter;
        assert!(EnvRuntime::new(&enc).is_ok());
        let mut boss = enc.roster[0].clone();
        boss.kind = RosterKind::Slime;
        enc.roster.push(boss);
        let err = EnvRuntime::new(&enc).expect_err("a slot boss in a 32 px remnant room");
        assert!(err.contains("remnant"), "{err}");
    }

    #[test]
    fn the_environment_source_id_is_above_the_goblin_index_range() {
        const { assert!(ENV_SOURCE_ID > 0xff) };
    }

    const KICK: RawInput = RawInput {
        physical: true,
        ..RawInput::NONE
    };

    // ---- BOIL-D: launched bodies, links, drum, blast hook

    const DRUM: usize = 2;

    fn kick_goblin_into_the_valve(s: &mut PartyFixtureSim) {
        put_carl(s, 400.0);
        for t in 0..80 {
            step(s, if t == 2 { KICK } else { RawInput::NONE });
        }
    }

    fn prop_events(s: &PartyFixtureSim) -> Vec<(u64, String)> {
        events(s)
            .iter()
            .filter_map(|e| match *e {
                CombatEvent::PropHit {
                    tick,
                    prop,
                    kind,
                    progress,
                    player,
                } => Some((tick, format!("hit {prop} {kind:?} {progress} {player:?}"))),
                CombatEvent::PropPhase {
                    tick,
                    prop,
                    phase,
                    player,
                } => Some((tick, format!("phase {prop} {} {player:?}", phase.name()))),
                _ => None,
            })
            .collect()
    }

    /// BOIL-03: a kicked goblin (170 px/s, a real launch) flies through the
    /// valve's box: one counted launched-body hit credited to the kicker sets it
    /// off, the goblin is not stopped, and nothing repeats while it flies on.
    #[test]
    fn a_kicked_goblin_sets_the_valve_off_once_and_flies_on() {
        let mut s = sim(&[430]);
        kick_goblin_into_the_valve(&mut s);
        let log = prop_events(&s);
        let hit = log.iter().find(|(_, e)| e.starts_with("hit 0")).unwrap();
        assert_eq!(hit.1, "hit 0 LaunchedBody 2 Some(0)");
        assert_eq!(
            log.iter().filter(|(_, e)| e.starts_with("hit 0")).count(),
            1,
            "one launch counts once, however many ticks it overlaps"
        );
        let t = hit.0;
        // The warning, then the vent exactly 23 ticks on, then the steam
        // reaches the brace (its link, depth 1, one light contact).
        assert_eq!(log[1], (t, "phase 0 warning Some(0)".to_owned()));
        assert_eq!(log[2], (t + 23, "phase 0 venting Some(0)".to_owned()));
        assert_eq!(log[3], (t + 23, "hit 1 PropContact 1 Some(0)".to_owned()));
        assert_eq!(log.len(), 4, "{log:?}");
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Weakened),
            "steam alone only weakens the brace"
        );
        // The goblin flew through: it ends well past the valve's far edge.
        assert!(s.combat.enemies[0].x > 480.0);
    }

    /// The same, from the right, with Carl facing left (both facings).
    #[test]
    fn a_goblin_kicked_leftwards_into_the_valve_counts_too() {
        let mut s = sim(&[520]);
        put_carl(&mut s, 550.0);
        s.scene.world.hero.facing = -1;
        for t in 0..80 {
            step(&mut s, if t == 2 { KICK } else { RawInput::NONE });
        }
        assert!(
            prop_events(&s)
                .iter()
                .any(|(_, e)| e == "hit 0 LaunchedBody 2 Some(0)"),
            "{:?}",
            prop_events(&s)
        );
    }

    /// Put a goblin in flight by hand (no hero kick): `vx` px/s from `x`, on a
    /// launch id of its own.
    fn fling(s: &mut PartyFixtureSim, goblin: usize, x: f32, vx: f32, launch: u32) {
        let g = &mut s.combat.enemies[goblin];
        g.x = x;
        g.mode = GoblinMode::Launched;
        g.vx = vx;
        g.vy = 0.0;
        g.launch = Some(launch);
    }

    #[test]
    fn a_launch_below_the_threshold_touches_the_valve_without_a_count() {
        let mut s = sim(&[470]);
        fling(&mut s, 0, 470.0, 100.0, 9);
        for _ in 0..6 {
            step(&mut s, RawInput::NONE);
        }
        assert!(prop_events(&s).is_empty(), "{:?}", prop_events(&s));
        assert_eq!(
            s.combat.env().unwrap().valve_state(VALVE),
            Some(ValveState::Intact)
        );
    }

    /// A launch with nobody credited (an enemy-origin launch) still counts, and
    /// credits no one (BOIL-02 player- and enemy-origin variants).
    #[test]
    fn an_unattributed_launch_at_speed_counts_and_credits_nobody() {
        let mut s = sim(&[470]);
        fling(&mut s, 0, 470.0, 200.0, 9);
        step(&mut s, RawInput::NONE);
        let log = prop_events(&s);
        assert_eq!(log[0].1, "hit 0 LaunchedBody 2 None");
        assert_eq!(log[1].1, "phase 0 warning None");
    }

    /// Two launches at a brace each weigh one (BOILC-AD1): the first weakens,
    /// the second, a distinct launch, releases.
    #[test]
    fn two_distinct_launches_release_the_brace_and_one_launch_never_counts_twice() {
        let mut s = sim(&[760, 760]);
        fling(&mut s, 0, 760.0, 200.0, 21);
        step(&mut s, RawInput::NONE);
        step(&mut s, RawInput::NONE);
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Weakened),
            "the same launch overlapping on two ticks counted once"
        );
        fling(&mut s, 1, 760.0, 200.0, 22);
        step(&mut s, RawInput::NONE);
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Released)
        );
    }

    /// The drum is a solid: a goblin a solid stops at kick speed is stopped by
    /// the drum, which takes the launch (the contact target reads the stop).
    #[test]
    fn a_goblin_stopped_by_the_drum_sets_it_off_credited_to_the_kicker() {
        let mut s = sim(&[820]);
        put_carl(&mut s, 790.0);
        for t in 0..60 {
            step(&mut s, if t == 2 { KICK } else { RawInput::NONE });
        }
        let log = prop_events(&s);
        let hit = log.iter().find(|(_, e)| e.starts_with("hit 2")).unwrap();
        assert_eq!(hit.1, "hit 2 LaunchedBody 1 Some(0)");
        let t = hit.0;
        assert!(
            events(&s)
                .iter()
                .any(|e| matches!(e, CombatEvent::WallSlam { tick, .. } if *tick == t))
        );
        assert_eq!(
            log[log.iter().position(|e| e == hit).unwrap() + 1],
            (t, "phase 2 warning Some(0)".to_owned())
        );
        assert!(
            log.iter()
                .any(|(tick, e)| *tick == t + 24 && e == "phase 2 detonated Some(0)"),
            "blast 24 ticks after the trigger: {log:?}"
        );
        let charge = s.combat.charges[s.combat.env().unwrap().drum_charge(DRUM as u16).unwrap()];
        assert!(charge.detonated);
        assert_eq!((charge.owner, charge.chain_depth), (Some(0), 0));
    }

    /// The index-range rule is generic: only the env solids are props. A body
    /// stopped by the landed remnant triggers nothing, one beside the drum
    /// triggers it, and a solid outside the range (a wall) is never a prop.
    #[test]
    fn the_prop_contact_target_reads_the_env_solid_range_only() {
        let range = sim(&[]).combat.env().unwrap().solid_range();
        assert!(!range.is_empty() && range.start > 0);
        // `gap` px between a 24 px box's right edge and `solid`'s left edge.
        let beside = |solid: Rect, gap: f32| LaunchedBody {
            body: Rect {
                left: solid.left - gap - 24.0,
                bottom: solid.bottom,
                width: 24.0,
                height: 52.0,
            },
            launch: 70,
            initiator: Some(0),
            speed: 0.0,
            stop: Some(170.0),
        };
        let drum = sim(&[]).scene.world.solids[range.start + 1].rect;
        // 170 px/s: ceil(170 / 60) + 1 = 4 px of reach, and the box test is
        // strict: a 3 px gap is struck, a 4 px gap is not.
        let mut s = sim(&[]);
        s.combat
            .env_launched_contacts(&s.scene, &beside(drum, 4.0), 5);
        assert!(prop_events(&s).is_empty(), "4 px away");
        s.combat
            .env_launched_contacts(&s.scene, &beside(drum, 3.0), 6);
        assert_eq!(prop_events(&s)[0].1, "hit 2 LaunchedBody 1 Some(0)");
        // Not stopped by anything: a box beside the drum is not a contact.
        let mut free = sim(&[]);
        let mut loose = beside(drum, 3.0);
        loose.stop = None;
        free.combat.env_launched_contacts(&free.scene, &loose, 5);
        assert!(prop_events(&free).is_empty(), "no stop, no overlap");
        // A solid outside the range (a wall of the room) is never a prop.
        let mut wall = sim(&[]);
        let outside = (0..range.start)
            .map(|i| wall.scene.world.solids[i].clone())
            .find(|b| b.intact && (b.rect.right() < 700.0 || b.rect.left > 960.0))
            .expect("the room has a solid outside the env range")
            .rect;
        wall.combat
            .env_launched_contacts(&wall.scene, &beside(outside, 1.0), 5);
        assert!(prop_events(&wall).is_empty(), "{outside:?}");
    }

    #[test]
    fn a_body_stopped_by_the_landed_remnant_triggers_nothing() {
        let mut s = sim(&[]);
        trigger_by_jabs(&mut s, BRACE_X, WEIGHT as u16, PropPhase::Released);
        for _ in 0..40 {
            step(&mut s, RawInput::NONE);
        }
        let range = s.combat.env().unwrap().solid_range();
        let remnant = s.scene.world.solids[range.start].clone();
        assert!(remnant.intact);
        let before = events(&s).len();
        let body = LaunchedBody {
            body: Rect {
                left: remnant.rect.left - 25.0,
                bottom: remnant.rect.bottom,
                width: 24.0,
                height: 52.0,
            },
            launch: 71,
            initiator: Some(0),
            speed: 0.0,
            stop: Some(240.0),
        };
        let t = tick(&s) + 1;
        s.combat.env_launched_contacts(&s.scene, &body, t);
        assert_eq!(
            events(&s).len(),
            before,
            "a settled mass is a wall, not a trigger"
        );
    }

    fn jab_the_brace_once(s: &mut PartyFixtureSim) {
        put_carl(s, BRACE_X);
        for t in 0..40 {
            step(s, if t == 0 { ATTACK } else { RawInput::NONE });
        }
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Weakened)
        );
    }

    fn run_ticks(s: &mut PartyFixtureSim, n: u64) {
        for _ in 0..n {
            step(s, RawInput::NONE);
        }
    }

    fn drum_charge_of(s: &PartyFixtureSim) -> BlastCharge {
        let i = s.combat.env().unwrap().drum_charge(DRUM as u16).unwrap();
        s.combat.charges[i]
    }

    /// BOIL-06: the full P1 -> P2 -> P3 chain, in exact event order. A jab
    /// weakens the brace, a kicked goblin sets the valve off, the steam (depth 1)
    /// finishes the brace, the weight lands 32 ticks later (the link costs one
    /// tick), crushes the goblin under it, reaches the drum (depth 2) and the drum
    /// blows 24 ticks after that. Carl is credited all the way; nothing is
    /// scripted: every step is a real contact.
    #[test]
    fn the_full_chain_runs_in_exact_order_and_credits_carl_throughout() {
        let mut s = sim(&[430, 816]);
        jab_the_brace_once(&mut s);
        kick_goblin_into_the_valve(&mut s);
        let log = prop_events(&s);
        let t = log.iter().find(|(_, e)| e.starts_with("hit 0")).unwrap().0;
        let r = t + 23;
        let l = r + 32;
        // Run past the blast.
        while tick(&s) < l + 40 {
            step(&mut s, RawInput::NONE);
        }
        let log = prop_events(&s);
        let after_jab: Vec<_> = log.iter().filter(|(tick, _)| *tick >= t).cloned().collect();
        let want: Vec<(u64, String)> = vec![
            (t, "hit 0 LaunchedBody 2 Some(0)".into()),
            (t, "phase 0 warning Some(0)".into()),
            (r, "phase 0 venting Some(0)".into()),
            (r, "hit 1 PropContact 2 Some(0)".into()),
            (r, "phase 1 released Some(0)".into()),
            (r + 24, "phase 1 falling Some(0)".into()),
            (l, "phase 1 landed Some(0)".into()),
            (l, "hit 2 PropContact 1 Some(0)".into()),
            (l, "phase 2 warning Some(0)".into()),
            (l + 24, "phase 2 detonated Some(0)".into()),
        ];
        // The valve's spend is in the list too (t + 95): compare without it.
        let got: Vec<_> = after_jab
            .into_iter()
            .filter(|(_, e)| !e.starts_with("phase 0 spent"))
            .collect();
        assert_eq!(got, want, "{log:?}");
        // The steam was depth 1, so the brace's trigger depth is 1 and the
        // landing's contact is depth 2: the drum's charge carries it.
        let charge = drum_charge_of(&s);
        assert!(charge.detonated);
        assert_eq!((charge.owner, charge.chain_depth), (Some(0), 2));
        // The goblin under the weight died to the weight, credited to Carl.
        assert_eq!(
            defeats(&s)
                .iter()
                .filter(|(_, _, c)| matches!(c, DefeatCause::Environment { .. }))
                .count(),
            1
        );
        // The room ends walkable: the drum solid is gone, the remnant stands.
        let range = s.combat.env().unwrap().solid_range();
        assert!(s.scene.world.solids[range.start].intact, "remnant");
        assert!(!s.scene.world.solids[range.start + 1].intact, "drum");
    }

    /// The other valid order: the valve first (the steam only weakens the
    /// brace), then a jab releases it (depth 0 for the weight, so the drum's
    /// contact is depth 1).
    #[test]
    fn the_chain_in_the_other_order_carries_a_shallower_depth() {
        let mut s = sim(&[430]);
        kick_goblin_into_the_valve(&mut s);
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Weakened)
        );
        // The brace stands in the steam's lane (688..768): wait the vent out.
        while s.combat.env().unwrap().valve_state(VALVE) != Some(ValveState::Spent) {
            step(&mut s, RawInput::NONE);
        }
        jab_the_brace_to_release(&mut s);
        let r = phase_tick(&s, WEIGHT as u16, PropPhase::Released).unwrap();
        while tick(&s) < r + 32 + 30 {
            step(&mut s, RawInput::NONE);
        }
        assert_eq!(
            phase_tick(&s, WEIGHT as u16, PropPhase::Landed),
            Some(r + 31)
        );
        assert_eq!(
            phase_tick(&s, DRUM as u16, PropPhase::Warning),
            Some(r + 31)
        );
        assert_eq!(drum_charge_of(&s).chain_depth, 1);
    }

    fn jab_the_brace_to_release(s: &mut PartyFixtureSim) {
        put_carl(s, BRACE_X);
        for t in 0..20 {
            step(s, if t == 0 { ATTACK } else { RawInput::NONE });
        }
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Released)
        );
    }

    /// The drum's blast: a goblin in the core ring dies to an explosion
    /// credited to the initiator, one in the outer band is hurt and launched,
    /// both heroes in the radius are hurt exactly once with the environment
    /// source, and the drum's solid is gone for the line of sight.
    #[test]
    fn the_drum_blast_hurts_everyone_in_reach_once_and_credits_the_initiator() {
        let mut s = sim_with(&[890, 930], true, Vec::new());
        // Set the drum off by a launch of Carl's kick (a stopped flier), then
        // move both heroes into the outer band just before the blast.
        let drum = s.combat.env().unwrap().drum_charge(DRUM as u16).unwrap();
        let stimulus = |s: &mut PartyFixtureSim| {
            let t = tick(s) + 1;
            s.combat
                .env_stimulate(
                    DRUM,
                    Stimulus::LaunchedBody {
                        launch_id: 5,
                        impulse_px_s: 170,
                        initiator: Some(0),
                    },
                    t,
                )
                .unwrap()
        };
        assert!(matches!(stimulus(&mut s), Offer::Triggered { .. }));
        let due = s.combat.charges[drum].detonate_tick;
        assert_eq!(due, tick(&s) + 1 + 24);
        let health_before = s.combat.enemies[1].health;
        put_carl(&mut s, 820.0);
        s.scene.world.donut_body.x = 908.0;
        while tick(&s) + 1 < due {
            step(&mut s, RawInput::NONE);
            put_carl(&mut s, 820.0);
            s.scene.world.donut_body.x = 908.0;
        }
        step(&mut s, RawInput::NONE);
        assert_eq!(tick(&s), due);
        let blast = events(&s)
            .iter()
            .position(
                |e| matches!(e, CombatEvent::Blast { charge, .. } if *charge as usize == drum),
            )
            .expect("the drum blew");
        // Order: the blast, the drum's phase, then the victims.
        assert!(matches!(
            events(&s)[blast + 1],
            CombatEvent::PropPhase {
                phase: PropPhase::Detonated,
                ..
            }
        ));
        let kills: Vec<_> = defeats(&s);
        assert!(
            kills.iter().any(|(_, g, c)| *g == 0
                && matches!(
                    c,
                    DefeatCause::Explosion {
                        player: Some(0),
                        point_blank: false,
                        ..
                    }
                )),
            "the goblin in the core ring: {kills:?}"
        );
        let hurts_carl = hurts_of(&s, HeroId::Carl);
        let hurts_donut = hurts_of(&s, HeroId::Donut);
        assert_eq!(hurts_carl.len(), 1, "{hurts_carl:?}");
        assert_eq!(hurts_donut.len(), 1, "{hurts_donut:?}");
        assert_eq!(hurts_carl[0].1, ENV_SOURCE_ID);
        assert_eq!(hurts_donut[0].1, ENV_SOURCE_ID);
        let range = s.combat.env().unwrap().solid_range();
        assert!(!s.scene.world.solids[range.start + 1].intact);
        // The outer-band goblin (index 1, 66 px away) is outside 48 px: it keeps
        // the health it had before the blast and is not launched.
        assert!(s.combat.enemies[1].alive());
        assert_eq!(s.combat.enemies[1].health, health_before);
        assert_ne!(s.combat.enemies[1].mode, GoblinMode::Launched);
        assert!(s.combat.enemies[1].launch.is_none());
    }

    /// A drum is set off once: a second stimulus after the telegraph or after
    /// the blast counts nothing (no second activation, no second blast).
    #[test]
    fn a_drum_blows_once_and_later_stimuli_count_nothing() {
        let mut s = sim(&[]);
        let t = tick(&s) + 1;
        let launch = |id| Stimulus::LaunchedBody {
            launch_id: id,
            impulse_px_s: 300,
            initiator: Some(0),
        };
        assert!(matches!(
            s.combat.env_stimulate(DRUM, launch(1), t),
            Some(Offer::Triggered { .. })
        ));
        let due = drum_charge_of(&s).detonate_tick;
        assert!(matches!(
            s.combat.env_stimulate(DRUM, launch(2), t + 3),
            Some(Offer::Ignored(crate::env_prop::Ignored::AlreadyActivated))
        ));
        assert_eq!(
            drum_charge_of(&s).detonate_tick,
            due,
            "the clock never moves"
        );
        while tick(&s) < due + 30 {
            step(&mut s, RawInput::NONE);
        }
        let blasts = events(&s)
            .iter()
            .filter(|e| matches!(e, CombatEvent::Blast { .. }))
            .count();
        assert_eq!(blasts, 1);
    }

    /// A checkpoint respawn and a restart in every phase of the drum restore
    /// it: armed, its charge unlit with no owner or depth, its solid back.
    #[test]
    fn a_respawn_and_a_restart_in_every_drum_phase_restore_the_drum() {
        for phase in 0..3 {
            for restart in [false, true] {
                let mut s = sim(&[]);
                let t = tick(&s) + 1;
                if phase >= 1 {
                    s.combat.env_stimulate(
                        DRUM,
                        Stimulus::LaunchedBody {
                            launch_id: 1,
                            impulse_px_s: 300,
                            initiator: Some(1),
                        },
                        t,
                    );
                }
                if phase == 2 {
                    run_ticks(&mut s, 30);
                    assert_eq!(
                        s.combat.env().unwrap().drum_state(DRUM),
                        Some(DrumState::Detonated)
                    );
                }
                let index = s.combat.env().unwrap().drum_charge(DRUM as u16).unwrap();
                let authored = s.combat.charges.len();
                if restart {
                    s.combat.restart(&mut s.scene);
                } else {
                    s.combat.respawn(&mut s.scene);
                }
                assert_eq!(
                    s.combat.env().unwrap().drum_state(DRUM),
                    Some(DrumState::Armed),
                    "phase {phase} restart {restart}"
                );
                assert_eq!(s.combat.charges.len(), authored, "no second charge");
                assert_eq!(
                    s.combat.env().unwrap().drum_charge(DRUM as u16),
                    Some(index)
                );
                let c = s.combat.charges[index];
                assert_eq!(
                    (c.detonate_tick, c.owner, c.chain_depth, c.detonated),
                    (0, None, 0, false)
                );
                let range = s.combat.env().unwrap().solid_range();
                assert!(
                    s.scene.world.solids[range.start + 1].intact,
                    "the drum stands again"
                );
            }
        }
    }

    /// A dynamite-style blast (depth 1: a chain reached it) beside the drum sets it
    /// off through the blast hook, not the charge chain: the drum's own solid
    /// stands between the two charge centres, so the chain's line of sight is
    /// blocked (CP-Q22) while the hook's closest-point segment ends on the
    /// drum's edge. The drum takes the blast's credit and depth and its own 24
    /// tick tell; the brace in reach is released at the same time.
    #[test]
    fn a_dynamite_blast_beside_the_drum_sets_it_off_through_the_hook() {
        use crate::carl_prog::CarlProgTuning;
        let mut s = sim(&[]);
        let n = 20;
        let mut charge = BlastCharge::new(830.0, s.combat.enemy_foot_y, n, Some(0))
            .with_profile(crate::thrown_charge::blast_profile(CarlProgTuning::active()));
        charge.chain_depth = 1;
        s.combat.push_authored_charge(charge);
        run_ticks(&mut s, n + 30);
        let log: Vec<_> = prop_events(&s)
            .into_iter()
            .filter(|(_, e)| e.starts_with("hit 2") || e.starts_with("phase 2"))
            .collect();
        assert_eq!(
            log,
            [
                (n, "hit 2 Blast 1 Some(0)".to_owned()),
                (n, "phase 2 warning Some(0)".to_owned()),
                (n + 24, "phase 2 detonated Some(0)".to_owned()),
            ],
            "{log:?}"
        );
        assert!(
            !events(&s)
                .iter()
                .any(|e| matches!(e, CombatEvent::ChargeChained { .. })),
            "the solid blocks the chain's line of sight"
        );
        let c = drum_charge_of(&s);
        assert_eq!((c.owner, c.chain_depth), (Some(0), 1));
    }

    /// CP-Q34 on the drum's charge: when a chain reaction lights it (the charge
    /// is lit with its owner and depth and no offer reached the machine) the
    /// machine follows with its tell logged once, and the blast is the chain's
    /// tick.
    #[test]
    fn a_chain_lit_drum_charge_makes_the_machine_follow_it() {
        let mut s = sim(&[]);
        let drum = s.combat.env().unwrap().drum_charge(DRUM as u16).unwrap();
        run_ticks(&mut s, 5);
        let now = tick(&s) + 1;
        assert_eq!(
            s.combat.charges[drum].chain_set(now, 8, Some(0), 2),
            Some(now + 8)
        );
        run_ticks(&mut s, 12);
        assert_eq!(
            prop_events(&s),
            [
                (now, "hit 2 Blast 1 Some(0)".to_owned()),
                (now, "phase 2 warning Some(0)".to_owned()),
                (now + 8, "phase 2 detonated Some(0)".to_owned()),
            ]
        );
        assert_eq!(drum_charge_of(&s).chain_depth, 2);
    }

    /// CP-Q35: a dynamite-style blast (breaks_structures) reaches the props in
    /// its outer circle: the brace is released credited to the owner and carries
    /// the blast's depth; the valve too; a QA blast and one out of reach do not.
    #[test]
    fn a_structure_breaking_blast_sets_off_the_props_it_reaches_with_its_depth() {
        use crate::carl_prog::CarlProgTuning;
        let dyn_profile = crate::thrown_charge::blast_profile(CarlProgTuning::active());
        let blast_at = |x: f32, profile: BlastProfile, depth: u8| {
            let mut s = sim(&[]);
            let mut c =
                BlastCharge::new(x, s.combat.enemy_foot_y, 9, Some(0)).with_profile(profile);
            c.chain_depth = depth;
            let t = tick(&s) + 1;
            s.combat.blast_props(&s.scene, &c, 0x4000_0001, t);
            s
        };
        // At the brace (744..776): released, depth 2 carried, Carl credited.
        let s = blast_at(760.0, dyn_profile, 2);
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Released)
        );
        assert_eq!(
            s.combat
                .env()
                .unwrap()
                .mass(WEIGHT)
                .unwrap()
                .trigger_depth(),
            2
        );
        assert_eq!(
            s.combat.env().unwrap().mass(WEIGHT).unwrap().initiator(),
            Some(0)
        );
        // At the valve (464..496): set off.
        let s = blast_at(480.0, dyn_profile, 0);
        assert_eq!(
            s.combat.env().unwrap().valve_state(VALVE),
            Some(ValveState::Warning)
        );
        // Out of the 64 px circle of every prop at x = 600 (valve right edge
        // 496 is 104 px away, the brace left edge 744 is 144 px away).
        let s = blast_at(600.0, dyn_profile, 0);
        assert!(prop_events(&s).is_empty());
        // The same blast under a profile that breaks nothing (the QA ring).
        let s = blast_at(760.0, BlastProfile::QA, 0);
        assert!(prop_events(&s).is_empty());
        // At the armed drum: it is set off and lights its charge for 24 ticks.
        let s = blast_at(830.0, dyn_profile, 0);
        assert_eq!(
            s.combat.env().unwrap().drum_state(DRUM),
            Some(DrumState::Telegraph)
        );
        assert_eq!(drum_charge_of(&s).detonate_tick, 1 + 24);
    }

    /// BOILD-AD1: what a linked prop accepts comes from the links that reach it.
    #[test]
    fn a_linked_prop_takes_the_weight_its_incoming_link_carries() {
        // The Boiler: valve --steam--> weight --landing--> drum.
        let s = sim(&[]);
        let env = s.combat.env().unwrap();
        assert_eq!(
            env.links().links().len(),
            2,
            "both authored links are in the runtime graph"
        );
        assert_eq!(env.mass(WEIGHT).unwrap().config().contact_weight, 1);
        assert_eq!(env.mass(WEIGHT).unwrap().config().release_weight, 2);
        assert_eq!(env.drum_plans().len(), 1);
        // The pure rule.
        assert_eq!(linked_weight(&[], 2), 0, "no link, no contact");
        assert_eq!(linked_weight(&[(0, LinkEmit::Steam)], 2), 1);
        assert_eq!(linked_weight(&[(0, LinkEmit::Steam)], 1), 1);
        assert_eq!(linked_weight(&[(0, LinkEmit::Landing)], 2), 2);
        assert_eq!(linked_weight(&[(0, LinkEmit::Blast)], 2), 2);
        assert_eq!(
            linked_weight(&[(0, LinkEmit::Steam), (1, LinkEmit::Landing)], 2),
            2,
            "the strongest incoming link wins"
        );
    }

    /// Steam alone never drops the weight (Boiler P1): one steam contact on a
    /// fresh brace only weakens it.
    #[test]
    fn steam_alone_weakens_the_brace_and_never_releases_it() {
        let mut s = sim(&[430]);
        kick_goblin_into_the_valve(&mut s);
        run_ticks(&mut s, 200);
        assert_eq!(
            s.combat.env().unwrap().mass_state(WEIGHT),
            Some(MassState::Weakened)
        );
        assert_eq!(
            s.combat.env().unwrap().drum_state(DRUM),
            Some(DrumState::Armed)
        );
    }

    /// The chain replays: the same inputs give the same events and the same
    /// state hash, and the drum is part of that hash.
    #[test]
    fn the_chain_replays_to_the_same_hash_and_events() {
        let play = |full: bool| {
            let mut s = sim(&[430, 816]);
            if full {
                jab_the_brace_once(&mut s);
            }
            kick_goblin_into_the_valve(&mut s);
            run_ticks(&mut s, 120);
            (s.combat.state_hash(&s.scene), events(&s))
        };
        assert_eq!(play(true), play(true));
        let (a, _) = play(true);
        let (b, _) = play(false);
        assert_ne!(a, b, "the drum and the brace states are hashed");
    }

    /// Props and charges share one chain counter (K2, CP-Q34): the prop link
    /// cap and the data file's chain depth are the same number, so an edit of
    /// either cannot split them silently.
    #[test]
    fn the_prop_chain_depth_cap_equals_the_charge_chain_depth_cap() {
        assert_eq!(
            crate::env_prop::MAX_CHAIN_DEPTH,
            crate::carl_prog::CarlProgTuning::active().cpq34_chain_max_depth
        );
    }
}
