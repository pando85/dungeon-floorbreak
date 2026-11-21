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
use crate::boss::HeroBlow;
use crate::camera_lab::room::IRect;
use crate::combat::{CombatEvent, DefeatCause, PropPhase};
use crate::donut_moves::DONUT_HERO;
use crate::encounter::{Encounter, PropKind};
use crate::env_hazard::{HazardClass, HazardEdge, HazardSet, NavRect, Team, Victim};
use crate::env_prop::{
    FallingMass, FallingMassConfig, MassBody, MassEdge, MassState, Offer, STEP_OVER_PX, Stimulus,
    Valve, ValveEdge, ValveState,
};
use crate::goblin::GoblinMode;
use crate::llama::LlamaMode;
use crate::prototype::{Block, HERO_HALF_HEIGHT, Material, Rect};
use crate::scene_enemy::{EnvSource, EnvTarget, EnvView, SlotEnvHit};

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
    /// A drum: only its solid in this step (BOIL-D adds the blast).
    Drum { intact: bool },
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
    /// An intact drum.
    Drum,
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
    /// Where the solids start in `scene.world.solids` once synced.
    solid_start: Option<usize>,
    /// The bodies of the last tick (the swept test's start). Not hashed: it is
    /// a function of the hashed positions.
    prev: Vec<(Victim, IRect)>,
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
        for (i, p) in enc.props.iter().enumerate() {
            let index = u16::try_from(i).map_err(|_| "too many props".to_owned())?;
            let machine = match p.kind {
                PropKind::Valve => {
                    let cfg = p
                        .valve
                        .ok_or_else(|| format!("prop {}: a valve needs its config", p.id))?;
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
                    if p.solid_intact {
                        solids.push(EnvSolid {
                            rect: p.target,
                            prop: index,
                            kind: EnvSolidKind::Drum,
                        });
                    }
                    Machine::Drum {
                        intact: p.solid_intact,
                    }
                }
                PropKind::Shutter | PropKind::RotSticker => {
                    return Err(format!(
                        "prop {}: the env step does not run a {:?} yet (RING and HAZ wire add it)",
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
        Ok(Self {
            props,
            pristine_hazards: hazards.clone(),
            hazards,
            owners,
            solids,
            solid_start: None,
            prev: Vec::new(),
        })
    }

    /// Back to the first state of the attempt (the solids stay appended).
    pub fn reset(&mut self) {
        for p in &mut self.props {
            match &mut p.machine {
                Machine::Valve { valve, .. } => valve.reset(),
                Machine::Mass(m) => m.reset(),
                Machine::Drum { intact } => *intact = true,
            }
        }
        self.hazards = self.pristine_hazards.clone();
        self.prev.clear();
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

    /// The CA2-C feed: the telegraphed and active volumes of this tick.
    pub fn nav_feed(&self) -> Vec<NavRect> {
        self.hazards.nav_feed()
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
        match (s.kind, &self.props[usize::from(s.prop)].machine) {
            (EnvSolidKind::Remnant, Machine::Mass(m)) => m.state() == MassState::Landed,
            (EnvSolidKind::Drum, Machine::Drum { intact }) => *intact,
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
                Machine::Drum { .. } => false,
            })
            .map(|p| (p.index, p.target))
            .collect()
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
            Machine::Drum { .. } => return None,
        };
        let mut events = Vec::new();
        let progress = match result {
            Offer::Ignored(_) => return Some((result, events)),
            Offer::Progress { progress } => progress,
            Offer::Triggered { .. } => match &prop.machine {
                Machine::Valve { valve, .. } => valve.progress(),
                Machine::Mass(m) => m.progress(),
                Machine::Drum { .. } => 0,
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
                _ => PropPhase::Released,
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
                    Some(ValveEdge::VentStarted) => out.events.push(CombatEvent::PropPhase {
                        tick,
                        prop,
                        phase: PropPhase::Venting,
                        player: valve.initiator(),
                    }),
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
                    }
                    None => {}
                },
                Machine::Drum { .. } => {}
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
                Machine::Drum { intact } => mix(u64::from(*intact)),
            }
        }
        self.hazards.mix_into(&mut *mix);
    }

    /// ` env=<prop>:<state>,...` for the state line.
    pub fn token(&self) -> String {
        let parts: Vec<String> = self
            .props
            .iter()
            .map(|p| match &p.machine {
                Machine::Valve { valve, .. } => format!("{}:{:?}", p.index, valve.state()),
                Machine::Mass(m) => format!("{}:{:?}", p.index, m.state()),
                Machine::Drum { intact } => format!("{}:drum_{}", p.index, intact),
            })
            .collect();
        format!(" env={}", parts.join(","))
    }
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

impl CombatState {
    /// Run this state with the env step of `env` (the Boiler room). Without
    /// it nothing is stepped, evented or hashed: every other fixture is the
    /// legacy game byte for byte.
    #[must_use]
    pub fn with_env(mut self, env: EnvRuntime) -> Self {
        self.env = Some(Box::new(env));
        self.reset_attempt();
        self
    }

    /// The env runtime, if the room has props.
    pub fn env(&self) -> Option<&EnvRuntime> {
        self.env.as_deref()
    }

    /// A new attempt: every prop back to its first state.
    pub(super) fn env_reset(&mut self) {
        if let Some(env) = self.env.as_mut() {
            env.reset();
        }
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
            let intact = env.solid_intact(i);
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
        let (offer, events) = self.env.as_mut()?.offer(prop, stimulus, tick)?;
        for event in events {
            self.log(event);
        }
        Some(offer)
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
        let Some(env) = self.env.as_ref() else {
            return false;
        };
        let layout_h = scene.world.layout_height();
        let rect = bolt.rect();
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

#[cfg(test)]
mod tests {
    use super::*;
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
    const BRACE_X: f32 = 740.0;
    const VALVE_X: f32 = 460.0;

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
}
