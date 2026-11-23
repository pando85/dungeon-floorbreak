//! CF02-I: the final CF02 frames and the Q69 to Q72 acting overlays.
//!
//! Presentation only. Everything here reads the finished simulation state (and
//! the events of the tick) and nothing flows back, so no overlay can change an
//! event, a hash or an input timing. Like the ART-INT-02 selection of
//! [`super`], the frame is a pure function of simulation state: a move shows the
//! frame of its own clock (held through hit-stop for free, the clock does not
//! move), and the few things that need a memory (an overlay's remaining ticks,
//! the whiffs that make Carl frustrated, the taunt hold of a punished goblin, the
//! age of a goblin's slammed finish) are derived from the same finished states
//! and the tick's events, so they replay.
//!
//! # Layers (Q69)
//!
//! `death > hurt > move > morale reaction > acting overlay > locomotion`, chosen
//! by [`crate::acting`]. The legacy selection ([`super::carl_pose`],
//! [`super::goblin_pose`]) keeps death, hurt and the legacy moves; this module
//! adds the CF02 moves (the stomp, the air kick, the dive, the bail-out), the
//! heavy dive landing, Carl's flinch and frustration, Donut's startle and smug,
//! and the goblin hesitate, panic, taunt (and the taunt hold of Q72) and the
//! three slammed finishes (Q21).
//!
//! * **Moves** are shown under `cf02_rules` (they exist only there).
//! * **Overlays** (flinch, frustration, startle, smug, taunt, the Q72 hold) need
//!   `q69_acting_overlays` as well; hesitate and panic are morale reactions of
//!   the simulation and show under `cf02_rules`.
//! * The planted-feet overlays are **idle only** (the acting spec, section 3):
//!   they are never shown while the actor runs or does anything else.
//!
//! The sheets are pivot-anchored like the locomotion atlases (the Carl sheet's
//! airborne frames are already lifted 2 px), so a frame placed at the sim foot
//! never pops against the neighbouring `jump_fall` or `land` frames.

use std::path::Path;

use super::{ArtPose, CarlInput, SpriteClips};
use crate::acting::{Acting, ActingInput, Layer, Locomotion, base_layer};
use crate::anim::Tag;
use crate::cf02::Cf02Tuning;
use crate::combat::{CombatEvent, MoveId};
use crate::combat_scene::CombatState;
use crate::corridor::SceneSim;
use crate::death::SlamPose;
use crate::goblin::GoblinMode;
use crate::morale::Reaction;
use crate::party::HeroId;
use crate::prototype::Rect;

/// A hero overlay's planted feet: shown in idle only, never in run (acting spec
/// section 3, recorded under Q69, Q70 and Q71). The selector of
/// [`crate::acting`] allows idle or run; this narrows it for the sheets drawn.
pub const PLANTED_FEET_OVERLAYS_IDLE_ONLY: bool = true;

/// The CF02 sheets of the three characters.
#[derive(Debug, Clone, PartialEq)]
pub struct ActingArt {
    pub carl: SpriteClips,
    pub donut: SpriteClips,
    /// The goblin acting sheets (hesitate, panic, taunt) by variant.
    pub goblins: [SpriteClips; super::GOBLIN_VARIANTS],
    /// The goblin slammed sheets (stomped, wall crumple, flung) by variant.
    pub slammed: [SpriteClips; super::GOBLIN_VARIANTS],
}

impl ActingArt {
    pub fn load(root: &Path, source: super::FrameSource) -> Result<Self, String> {
        Ok(Self {
            carl: SpriteClips::load_carl_cf02(root, source)?,
            donut: SpriteClips::load_donut_cf02(root, source)?,
            goblins: [
                SpriteClips::load_goblin_cf02(root, 0, source)?,
                SpriteClips::load_goblin_cf02(root, 1, source)?,
            ],
            slammed: [
                SpriteClips::load_goblin_slammed(root, 0, source)?,
                SpriteClips::load_goblin_slammed(root, 1, source)?,
            ],
        })
    }
}

// ----------------------------------------------------------------- Carl -----

/// Carl's overlays (Q70).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarlOverlay {
    Flinch,
    Frustration,
}

/// What Carl's CF02 frame depends on, from one finished tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CarlActInput {
    /// The running move and its clock (held while the move is frozen).
    pub melee: Option<(MoveId, u16)>,
    /// Ticks since the bail-out hop started (`Some` while it is in flight).
    pub bail_age: Option<u32>,
    /// Ticks since a dive whiff touched down (`Some` for the 10 recovery ticks).
    pub heavy_landing_age: Option<u32>,
    /// The dive's active ticks (`q52_dive_max_active_ticks`): the plunge frame
    /// holds for them, then the locomotion frame.
    pub dive_plunge_ticks: u32,
}

/// Ticks of the stamp-less part of the dive: the tuck (the readable tell).
fn move_tag(clips: &SpriteClips, name: &'static str, age: u32) -> ArtPose {
    ArtPose {
        tag: name,
        frame: clips.frame_at(name, age),
    }
}

/// The frame of Carl's CF02 **move layer**, or `None` when no CF02 frame draws
/// him (the legacy moves are drawn by [`super::carl_pose`]; the air kick and
/// the dive past their own frames show the locomotion `jump_fall`).
///
/// * stomp: knee-up tell, stamp (held through the hit-stop), recover;
/// * air kick: chamber ticks 0 to 4, the boot to tick 14, then the locomotion
///   frame;
/// * dive: the tuck for ticks 0 to 5, then the plunge until the move ends (30
///   active ticks; a hit bounces and the move ends);
/// * bail-out: the push-off for 3 ticks, then the scramble until landing;
/// * a dive whiff's landing: the heavy landing for its 10 ticks.
pub fn carl_move_pose(clips: &SpriteClips, input: &CarlActInput) -> Option<ArtPose> {
    if let Some((id, tick)) = input.melee {
        let tick = u32::from(tick);
        match id {
            MoveId::Stomp => return Some(move_tag(clips, "stomp", tick)),
            MoveId::AirKick => {
                return (tick < clips.tag_ticks("air_kick")?)
                    .then(|| move_tag(clips, "air_kick", tick));
            }
            MoveId::DiveStomp => {
                let tuck = clips.tag_ticks("dive_tuck")?;
                if tick < tuck {
                    return Some(move_tag(clips, "dive_tuck", tick));
                }
                // Dive plunge: held until the move ends, `q52_dive_max_active_ticks`.
                return (tick < tuck + input.dive_plunge_ticks)
                    .then(|| move_tag(clips, "dive_plunge", tick - tuck));
            }
            _ => {}
        }
    }
    if let Some(age) = input.bail_age {
        return Some(move_tag(clips, "bailout", age));
    }
    input
        .heavy_landing_age
        .map(|age| move_tag(clips, "dive_land", age))
}

/// A Carl overlay frame `age` ticks into it: the flinch is one frame, the
/// frustration alternates its two frames every 6 ticks (24 ticks in all).
pub fn carl_overlay_pose(clips: &SpriteClips, kind: CarlOverlay, age: u32) -> ArtPose {
    match kind {
        CarlOverlay::Flinch => move_tag(clips, "flinch", age),
        CarlOverlay::Frustration => loop_pose(clips, "frustration", age),
    }
}

/// Frames of `tag` in order, each for its own ticks, repeating.
fn loop_pose(clips: &SpriteClips, tag: &'static str, age: u32) -> ArtPose {
    let cycle = clips
        .tag_ticks(tag)
        .expect("a looping tag has timed frames");
    move_tag(clips, tag, age % cycle)
}

// ---------------------------------------------------------------- Donut -----

/// Donut's overlays (Q71).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonutOverlay {
    Startle,
    Smug,
}

/// A Donut overlay frame `age` ticks into it: the startle is the puff and then
/// the settle (6 ticks each), the smug one held frame.
pub fn donut_overlay_pose(clips: &SpriteClips, kind: DonutOverlay, age: u32) -> ArtPose {
    match kind {
        DonutOverlay::Startle => move_tag(clips, "startle", age),
        DonutOverlay::Smug => move_tag(clips, "smug", age),
    }
}

// --------------------------------------------------------------- goblins -----

/// The goblin acting sheet's reactions, as the selector sees them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoblinReactionArt {
    Hesitate,
    /// The flee direction (+1 / -1): the sheet's right is the flee direction.
    Panic {
        flee_dir: i8,
    },
    Taunt,
}

/// What a goblin's CF02 frame depends on, from one finished tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoblinActInput {
    pub mode: GoblinMode,
    pub alive: bool,
    /// The running reaction with the ticks it has run, if it has art.
    pub reaction: Option<(GoblinReactionArt, u32)>,
    /// The Q72 taunt hold: the taunt frame (an index inside the `taunt` tag)
    /// still held after a non-launching hit.
    pub taunt_hold: Option<usize>,
}

/// A goblin frame of the CF02 acting sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoblinActPose {
    pub pose: ArtPose,
    /// `Some(flip)` overrides the facing: a panicking goblin faces the way it
    /// flees (the sheet's right), not the hero.
    pub flip: Option<bool>,
}

/// The layer of a goblin this tick (Q69): dead, hurt (every hit, down and
/// launched mode), a move (its attack), a morale reaction (hesitate, panic), or
/// locomotion; a taunt is an acting overlay.
pub fn goblin_layer(input: &GoblinActInput) -> Layer {
    base_layer(&ActingInput {
        dead: !input.alive,
        hurt: input.alive
            && matches!(
                input.mode,
                GoblinMode::Hurt | GoblinMode::Launched | GoblinMode::Downed
            ),
        in_move: input.mode == GoblinMode::Attack,
        morale_reaction: matches!(
            input.reaction,
            Some((
                GoblinReactionArt::Hesitate | GoblinReactionArt::Panic { .. },
                _
            ))
        ),
        locomotion: if input.mode == GoblinMode::Neutral {
            Locomotion::Idle
        } else {
            Locomotion::Other
        },
        press_this_tick: false,
    })
}

/// The goblin's CF02 acting frame, or `None` when the GOB-01 selection draws it.
///
/// * hesitate and panic (morale reactions) win over locomotion and lose to a
///   hurt, a dead body and the attack;
/// * the taunt overlay needs `overlays` and shows only while the goblin stands;
/// * the Q72 taunt hold shows the taunt frame over the hit it just took (a
///   non-launching hit: the goblin stays `Hurt` or lies `Down`); the caller never
///   sets it for a launch.
pub fn goblin_acting_pose(
    clips: &SpriteClips,
    input: &GoblinActInput,
    overlays: bool,
) -> Option<GoblinActPose> {
    if overlays
        && let Some(frame) = input.taunt_hold
        && matches!(
            input.mode,
            GoblinMode::Hurt | GoblinMode::Down | GoblinMode::Corpse
        )
    {
        return Some(GoblinActPose {
            pose: ArtPose {
                tag: "taunt",
                frame: clips.frame_in("taunt", frame),
            },
            flip: None,
        });
    }
    let (reaction, age) = input.reaction?;
    match (goblin_layer(input), reaction) {
        (Layer::MoraleReaction, GoblinReactionArt::Hesitate) => Some(GoblinActPose {
            pose: move_tag(clips, "hesitate", age),
            flip: None,
        }),
        (Layer::MoraleReaction, GoblinReactionArt::Panic { flee_dir }) => Some(GoblinActPose {
            pose: loop_pose(clips, "panic", age),
            flip: Some(flee_dir < 0),
        }),
        (Layer::Locomotion, GoblinReactionArt::Taunt) if overlays => Some(GoblinActPose {
            pose: loop_pose(clips, "taunt", age),
            flip: None,
        }),
        _ => None,
    }
}

/// What a slammed goblin's frame depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlamArtInput {
    pub pose: SlamPose,
    pub mode: GoblinMode,
    /// Presentation ticks since the defeat (the Q72 taunt hold excluded: the
    /// impact hold starts after it).
    pub dead_age: u32,
    /// Ticks since the dead body stopped flying (0 on the first lying tick).
    pub landed_age: Option<u32>,
    /// `q21_slammed_impact_hold_ticks`.
    pub impact_hold: u32,
}

/// The slammed finish frame (Q21), or `None` while the GOB-01 knockback flight
/// is drawn (a flung body is thrown through the air first).
///
/// * stomped: the fold for the impact hold, then the dead bug;
/// * wall crumple: the squash on the wall for the impact hold (and while the body
///   is still falling), then the slump;
/// * flung: the face-plant from the landing for the impact hold, then the heap.
pub fn slammed_pose(clips: &SpriteClips, input: &SlamArtInput) -> Option<ArtPose> {
    let airborne = input.mode == GoblinMode::Airborne;
    let (tag, rest) = match input.pose {
        SlamPose::Stomped => ("slammed_stomped", input.dead_age >= input.impact_hold),
        SlamPose::WallCrumple => (
            "slammed_wall",
            input.dead_age >= input.impact_hold && !airborne,
        ),
        SlamPose::Flung => {
            if airborne {
                return None;
            }
            (
                "slammed_flung",
                input.landed_age.is_none_or(|age| age >= input.impact_hold),
            )
        }
    };
    Some(ArtPose {
        tag,
        frame: clips.frame_in(tag, usize::from(rest)),
    })
}

/// Draw offset (px along the facing) that puts the wall-contact column x16 of a
/// slammed-wall frame on the wall face. The sim stops a body at the wall with its
/// back edge on the face (`ENEMY_HALF_WIDTH` behind its centre); column x16 is
/// `pivot - 16` behind the pivot, which is the same 12 px, so the offset is 0 for
/// the shipped sheet. It is computed, not assumed, so a sheet change shows up in
/// the test.
pub fn wall_contact_offset_px(clips: &SpriteClips) -> i32 {
    crate::combat::ENEMY_HALF_WIDTH as i32 - (clips.spec.pivot_x - WALL_CONTACT_COLUMN)
}

/// The sprite column of a wall crumple's contact with the wall (acting spec,
/// section 5).
pub const WALL_CONTACT_COLUMN: i32 = 16;

// -------------------------------------------------------------- tracking -----

/// Ticks a smug waits for Donut to finish her move and stand idle: her hit
/// lands inside her own swat or pounce, the reaction follows it. Adopted default
/// under Q71 (the longest tail after a landing hit is the pounce's, at most about
/// 34 ticks plus the landing); the owner may retune it after playing.
pub const SMUG_PENDING_TICKS: u8 = 45;

/// Ticks a frustration waits for Carl to stand idle, counted from the third
/// whiff. Adopted default under Q70: `q70_frustration_window_ticks` bounds the run
/// of three whiffs only, so the shake never appears long after the miss.
pub const FRUSTRATION_PENDING_TICKS: u64 = 45;

/// The Carl moves a whiff or a hit is counted for (Q70).
fn is_carl_move(id: MoveId) -> bool {
    matches!(
        id,
        MoveId::Jab
            | MoveId::Haymaker
            | MoveId::PushKick
            | MoveId::FollowKick
            | MoveId::Stomp
            | MoveId::AirKick
            | MoveId::DiveStomp
    )
}

/// Donut's own moves: a hit by one of them makes her smug (Q71).
fn is_donut_move(id: MoveId) -> bool {
    matches!(id, MoveId::Swat | MoveId::Pounce | MoveId::MagicMissile)
}

/// Straight-line gap in px between two rectangles (0 when they touch or
/// overlap).
pub fn rect_gap_px(a: &Rect, b: &Rect) -> f32 {
    let dx = (a.left - b.right()).max(b.left - a.right()).max(0.0);
    let dy = (a.bottom - b.top()).max(b.bottom - a.top()).max(0.0);
    dx.hypot(dy)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReactionTag {
    None,
    Hesitate,
    Panic,
    Taunt,
    Other,
}

#[derive(Debug, Clone)]
struct GoblinActing {
    reaction: ReactionTag,
    reaction_age: u32,
    /// Ticks the taunt hold still shows (Q72) and the frame it shows.
    hold_left: u8,
    hold_frame: usize,
    /// Presentation ticks since the slammed frame may show: `None` while a Q72
    /// taunt hold runs (the taunt frame is drawn, never the finish).
    dead_age: Option<u32>,
    /// Ticks since the dead body stopped flying.
    landed_age: Option<u32>,
    /// A near miss was already counted for the swing running now.
    near_miss_counted: bool,
    input: GoblinActInput,
    pose: Option<GoblinActPose>,
}

impl GoblinActing {
    fn new(g: &crate::goblin::Goblin) -> Self {
        Self {
            reaction: ReactionTag::None,
            reaction_age: 0,
            hold_left: 0,
            hold_frame: 0,
            dead_age: (!g.alive()).then_some(0),
            landed_age: None,
            near_miss_counted: false,
            input: GoblinActInput {
                mode: g.mode,
                alive: g.alive(),
                reaction: None,
                taunt_hold: None,
            },
            pose: None,
        }
    }
}

/// The Carl move whose whiff is being judged.
#[derive(Debug, Clone, Copy)]
struct CarlMoveWatch {
    instance: u32,
    hit: bool,
}

/// The acting presentation of one run: the overlays of Carl and Donut, the
/// goblin reactions, the slammed ages. Stepped once per fixed tick from the
/// finished state; nothing in it reaches the simulation.
#[derive(Debug, Clone)]
pub struct ActingAnimators {
    pub art: ActingArt,
    tuning: &'static Cf02Tuning,
    /// Combat events already seen (`dropped_events + events.len()`).
    seen_events: usize,
    carl_overlay: Acting<CarlOverlay>,
    carl_overlay_total: u8,
    donut_overlay: Acting<DonutOverlay>,
    donut_overlay_total: u8,
    startle_cooldown: u16,
    smug_cooldown: u16,
    /// A smug waiting for Donut to be idle (ticks left), set by her hit.
    smug_pending: Option<u8>,
    /// DM-C (DM-Q21): the tick her evade response ends, after which the smug
    /// follows (the cat is cute first, smug as the payoff).
    evade_smug_at: Option<u64>,
    frustration_cooldown: u16,
    /// Ticks (scene tick) of the last whiffs in a row, oldest first.
    whiffs: Vec<u64>,
    watch: Option<CarlMoveWatch>,
    /// Globs within the flinch distance of Carl on the last tick (owner, dir).
    /// The globs near Carl last tick, by instance identity (owner and spawn tick,
    /// CF02-FU-I FU4: two globs of one llama in one direction are not one).
    near_globs: Vec<(u32, u32)>,
    // Carl move memory.
    prev_melee: Option<(MoveId, bool)>,
    bail_age: Option<u32>,
    heavy_left: u8,
    heavy_age: u32,
    carl_pose: Option<ArtPose>,
    carl_layer: Layer,
    donut_pose: Option<ArtPose>,
    goblins: Vec<GoblinActing>,
}

impl ActingAnimators {
    /// `tuning` is the run's pinned tuning (`combat.cf02()`); the overlays need
    /// its `q69_acting_overlays` switch as well.
    pub fn new(art: ActingArt, tuning: &'static Cf02Tuning, combat: &CombatState) -> Self {
        let mut this = Self {
            art,
            tuning,
            seen_events: 0,
            carl_overlay: Acting::default(),
            carl_overlay_total: 0,
            donut_overlay: Acting::default(),
            donut_overlay_total: 0,
            startle_cooldown: 0,
            smug_cooldown: 0,
            smug_pending: None,
            evade_smug_at: None,
            frustration_cooldown: 0,
            whiffs: Vec::new(),
            watch: None,
            near_globs: Vec::new(),
            prev_melee: None,
            bail_age: None,
            heavy_left: 0,
            heavy_age: 0,
            carl_pose: None,
            carl_layer: Layer::Locomotion,
            donut_pose: None,
            goblins: Vec::new(),
        };
        this.reset(combat);
        this
    }

    /// The run's tuning (the overlay switch included).
    pub fn tuning(&self) -> &'static Cf02Tuning {
        self.tuning
    }

    /// Whether the Q69 overlays are on (`cf02_rules` and `q69_acting_overlays`).
    pub fn overlays_on(&self) -> bool {
        crate::acting::overlays_enabled(self.tuning)
    }

    /// Back to the start of an attempt (the goblins were re-created, the
    /// overlays and memories are gone).
    pub fn reset(&mut self, combat: &CombatState) {
        self.seen_events = combat.dropped_events + combat.events.len();
        self.carl_overlay = Acting::default();
        self.donut_overlay = Acting::default();
        self.startle_cooldown = 0;
        self.smug_cooldown = 0;
        self.smug_pending = None;
        self.evade_smug_at = None;
        self.frustration_cooldown = 0;
        self.whiffs.clear();
        self.watch = None;
        self.near_globs.clear();
        self.prev_melee = None;
        self.bail_age = None;
        self.heavy_left = 0;
        self.carl_pose = None;
        self.carl_layer = Layer::Locomotion;
        self.donut_pose = None;
        self.goblins = combat.enemies.iter().map(GoblinActing::new).collect();
    }

    /// Carl's CF02 frame (a move, the heavy landing, or an overlay), or `None`
    /// when the legacy selection or the locomotion animation draws him.
    pub fn carl_pose(&self) -> Option<ArtPose> {
        self.carl_pose
    }

    /// The layer that won for Carl on the last tick (for tests and the trace).
    pub fn carl_layer(&self) -> Layer {
        self.carl_layer
    }

    /// Donut's overlay frame, if she is drawn one.
    pub fn donut_pose(&self) -> Option<ArtPose> {
        self.donut_pose
    }

    /// A goblin's acting frame (hesitate, panic, taunt, the taunt hold).
    pub fn goblin_pose(&self, index: usize) -> Option<GoblinActPose> {
        self.goblins.get(index)?.pose
    }

    /// A goblin's slammed finish frame when `slam` says it has one.
    pub fn slammed_pose(
        &self,
        index: usize,
        variant: usize,
        slam: Option<SlamPose>,
    ) -> Option<ArtPose> {
        let slam = slam?;
        let goblin = self.goblins.get(index)?;
        let dead_age = goblin.dead_age?;
        slammed_pose(
            &self.art.slammed[variant],
            &SlamArtInput {
                pose: slam,
                mode: goblin.input.mode,
                dead_age,
                landed_age: goblin.landed_age,
                impact_hold: u32::from(self.tuning.q21_slammed_impact_hold_ticks),
            },
        )
    }

    /// Advance by one fixed tick. `carl_legacy` is Carl's legacy combat frame
    /// (hurt, knockdown, jab, haymaker, kick), which outranks every CF02 frame
    /// but a move that has none. `carl_loco` and `donut_loco` are the
    /// locomotion tags of the tick; `carl_input` is the finished
    /// [`CarlInput`].
    pub fn step(
        &mut self,
        scene: &SceneSim,
        combat: &CombatState,
        carl_input: &CarlInput,
        carl_legacy: bool,
        carl_loco: Tag,
        donut_loco: Tag,
    ) {
        let t = self.tuning;
        let hero = &scene.world.hero;
        let tick = scene.world.tick;
        let total = combat.dropped_events + combat.events.len();
        let fresh = total
            .saturating_sub(self.seen_events)
            .min(combat.events.len());
        self.seen_events = total;
        let events = &combat.events[combat.events.len() - fresh..];
        self.startle_cooldown = self.startle_cooldown.saturating_sub(1);
        self.smug_cooldown = self.smug_cooldown.saturating_sub(1);
        self.frustration_cooldown = self.frustration_cooldown.saturating_sub(1);

        // A wave can add goblins (or a restart remove them) between resets.
        if self.goblins.len() != combat.enemies.len() {
            self.goblins.truncate(combat.enemies.len());
            let known = self.goblins.len();
            self.goblins
                .extend(combat.enemies[known..].iter().map(GoblinActing::new));
        }

        // ------------------------------------------------ events of the tick
        let mut want_startle = false;
        for event in events {
            match *event {
                CombatEvent::MoveStarted {
                    move_id, instance, ..
                } if is_carl_move(move_id) => {
                    if let Some(prev) = self.watch.replace(CarlMoveWatch {
                        instance,
                        hit: false,
                    }) && !prev.hit
                    {
                        self.note_whiff(tick);
                    }
                }
                CombatEvent::Hit {
                    move_id,
                    instance,
                    target,
                    ..
                } => {
                    if is_carl_move(move_id) {
                        if let Some(w) = self.watch.as_mut()
                            && w.instance == instance
                        {
                            w.hit = true;
                        }
                        self.whiffs.clear();
                    }
                    if is_donut_move(move_id) {
                        // Shown once she is idle (her move is still running).
                        self.smug_pending = Some(SMUG_PENDING_TICKS);
                    }
                    self.note_hit_on_goblin(target as usize, combat);
                }
                CombatEvent::HeroEvaded {
                    hero: HeroId::Donut,
                    tick: evaded_at,
                    ..
                } => {
                    // DM-C: the evade response runs `EVADE_TICKS`, then the smug.
                    self.evade_smug_at = Some(evaded_at + crate::donut_cues::EVADE_TICKS);
                }
                CombatEvent::HeroHurt {
                    hero: HeroId::Carl, ..
                } => want_startle |= self.within_startle(scene, hero.x),
                CombatEvent::WallSlam { goblin, .. } => {
                    if let Some(g) = combat.enemies.get(goblin as usize) {
                        want_startle |= self.within_startle(scene, g.x);
                    }
                }
                CombatEvent::Blast { charge, .. } => {
                    if let Some(c) = combat.charges.get(charge as usize) {
                        want_startle |= self.within_startle(scene, c.x);
                    }
                }
                CombatEvent::ShoreLanded { .. } => {
                    if let Some(shore) = combat.shore.as_ref() {
                        want_startle |= self.within_startle(scene, shore.strike_x);
                    }
                }
                _ => {}
            }
        }

        // -------------------------------------------------------------- Carl
        let view = combat.carl_acting_view();
        // A move that ended without a hit counts as a whiff when its clock is
        // gone (the next `MoveStarted` handles a chained move).
        if hero.melee.current().is_none()
            && let Some(w) = self.watch.take()
            && !w.hit
        {
            self.note_whiff(tick);
        }
        let melee = hero.melee.current();
        // The landing of a dive that hit nothing: the sim ends the dive on the
        // tick after the touchdown and starts the 10 tick landing recovery
        // (Q56); the heavy landing frame covers exactly those ticks.
        if melee.is_none()
            && hero.grounded
            && self.heavy_left == 0
            && view.landing_lock > 0
            && let Some((MoveId::DiveStomp, false)) = self.prev_melee
        {
            self.heavy_left = t.q56_landing_dive_whiff;
            self.heavy_age = 0;
        } else if self.heavy_left > 0 && (!hero.grounded || melee.is_some()) {
            self.heavy_left = 0;
        }
        let heavy = (self.heavy_left > 0).then_some(self.heavy_age);
        if self.heavy_left > 0 {
            self.heavy_left -= 1;
            self.heavy_age += 1;
        }
        self.bail_age = if view.bail_hop {
            Some(self.bail_age.map_or(0, |a| a + 1))
        } else {
            None
        };
        self.prev_melee = melee.map(|m| (m.spec.id, hero.melee.current_hit()));

        // Near miss (Q70 flinch): an enemy swing box or a glob came within the
        // distance of Carl's hurtbox and did not hit. A swing is judged when it
        // ends (its impact window over, `swing_hit` still false), a glob when it
        // leaves the zone or ends without a `HeroHurt` of Carl's: the flinch is
        // the reaction to the miss, never to a blow that lands.
        let hurtbox = CombatState::hurtbox(scene, HeroId::Carl);
        let near = f32::from(t.q70_flinch_px);
        let carl_hurt_now = events.iter().any(|e| {
            matches!(
                e,
                CombatEvent::HeroHurt {
                    hero: HeroId::Carl,
                    ..
                }
            )
        });
        let mut flinch = false;
        for (index, goblin) in combat.enemies.iter().enumerate() {
            let acting = &mut self.goblins[index];
            match goblin.attack_phase() {
                Some(crate::goblin::AttackPhase::Impact) => {
                    let gap = rect_gap_px(&goblin.swing_box(), &hurtbox);
                    acting.near_miss_counted |= gap > 0.0 && gap <= near;
                }
                Some(crate::goblin::AttackPhase::Recover) => {
                    flinch |= acting.near_miss_counted && !goblin.swing_hit && !carl_hurt_now;
                    acting.near_miss_counted = false;
                }
                _ => acting.near_miss_counted = false,
            }
        }
        let mut near_globs: Vec<(u32, u32)> = Vec::new();
        for glob in &combat.globs {
            let gap = rect_gap_px(&glob.rect(), &hurtbox);
            if gap > 0.0 && gap <= near {
                near_globs.push((glob.owner, glob.spawn_tick));
            }
        }
        for key in &self.near_globs {
            if near_globs.contains(key) {
                continue;
            }
            // It was near last tick and is not now: it passed (still in flight
            // and clear of the zone), or it ended without hurting Carl.
            let alive = combat
                .globs
                .iter()
                .find(|g| (g.owner, g.spawn_tick) == *key)
                .map(|g| rect_gap_px(&g.rect(), &hurtbox));
            flinch |= match alive {
                Some(gap) => gap > near,
                None => !carl_hurt_now,
            };
        }
        self.near_globs = near_globs;
        // The planted-feet overlays start only when they can be drawn (idle, no
        // move, not hurt, no press this tick), so a cooldown is never spent on a
        // frame that does not show.
        let carl_can_show = carl_loco == Tag::Idle
            && melee.is_none()
            && self.bail_age.is_none()
            && heavy.is_none()
            && carl_input.hurt_ticks == 0
            && !carl_input.defeated
            && !view.press_edge;
        if flinch
            && carl_can_show
            && self
                .carl_overlay
                .start(t, CarlOverlay::Flinch, t.q70_flinch_ticks)
        {
            self.carl_overlay_total = t.q70_flinch_ticks;
        }
        // Frustration (Q70): three moves in a row without a hit in the window.
        if self.whiffs.len() >= usize::from(t.q70_frustration_moves)
            && let (Some(first), Some(last)) = (
                self.whiffs
                    .get(self.whiffs.len() - usize::from(t.q70_frustration_moves)),
                self.whiffs.last(),
            )
            && last.saturating_sub(*first) <= u64::from(t.q70_frustration_window_ticks)
        {
            // Pending until Carl stands idle, for `FRUSTRATION_PENDING_TICKS` from
            // the third whiff.
            let fresh = tick.saturating_sub(*last) <= FRUSTRATION_PENDING_TICKS;
            if !fresh {
                self.whiffs.clear();
            } else if carl_can_show && self.frustration_cooldown == 0 {
                // CF02-FU-I FU3: the run of whiffs is spent only when the overlay
                // really starts: a running flinch refuses the start, and the
                // frustration stays pending (until its window passes) instead of
                // being lost.
                if self
                    .carl_overlay
                    .start(t, CarlOverlay::Frustration, t.q70_frustration_ticks)
                {
                    self.whiffs.clear();
                    self.carl_overlay_total = t.q70_frustration_ticks;
                    self.frustration_cooldown = t.q70_frustration_cooldown;
                }
            } else if self.frustration_cooldown > 0 {
                // Cooling down: the run of whiffs is spent, as if it had shown.
                self.whiffs.clear();
            }
        }
        let carl_in = CarlActInput {
            melee: melee.map(|m| (m.spec.id, m.tick)),
            bail_age: self.bail_age,
            heavy_landing_age: heavy,
            dive_plunge_ticks: u32::from(t.q52_dive_max_active_ticks),
        };
        let loco = |tag: Tag| match tag {
            Tag::Idle => Locomotion::Idle,
            Tag::Run if !PLANTED_FEET_OVERLAYS_IDLE_ONLY => Locomotion::Run,
            _ => Locomotion::Other,
        };
        let selection = self.carl_overlay.select(
            t,
            &ActingInput {
                dead: carl_input.defeated,
                hurt: carl_input.hurt_ticks > 0,
                in_move: melee.is_some() || self.bail_age.is_some() || heavy.is_some(),
                morale_reaction: false,
                locomotion: loco(carl_loco),
                press_this_tick: view.press_edge,
            },
        );
        self.carl_layer = selection.layer;
        self.carl_pose = if carl_legacy {
            None
        } else {
            match (selection.layer, selection.overlay) {
                (Layer::Overlay, Some(kind)) => {
                    let age = overlay_age(
                        self.carl_overlay_total,
                        self.carl_overlay.running().map(|(_, left)| left),
                    );
                    Some(carl_overlay_pose(&self.art.carl, kind, age))
                }
                (Layer::Move, _) => carl_move_pose(&self.art.carl, &carl_in),
                _ => None,
            }
        };

        // ------------------------------------------------------------- Donut
        let donut_busy = combat.donut_move_view().is_some() || combat.donut_cast().is_some();
        let donut_can_show = donut_loco == Tag::Idle
            && !donut_busy
            && combat.hurt_ticks_of(HeroId::Donut) == 0
            && !combat.hero_defeated;
        if want_startle && donut_can_show && self.startle_cooldown == 0 {
            self.start_donut(DonutOverlay::Startle);
        }
        if let Some(at) = self.evade_smug_at
            && tick >= at
        {
            self.evade_smug_at = None;
            self.smug_pending = Some(SMUG_PENDING_TICKS);
        }
        if let Some(left) = self.smug_pending {
            if donut_can_show && self.smug_cooldown == 0 && !self.donut_overlay.is_running() {
                self.smug_pending = None;
                self.start_donut(DonutOverlay::Smug);
            } else {
                self.smug_pending = left.checked_sub(1).filter(|l| *l > 0);
            }
        }
        let selection = self.donut_overlay.select(
            t,
            &ActingInput {
                dead: combat.hero_defeated,
                hurt: combat.hurt_ticks_of(HeroId::Donut) > 0,
                in_move: donut_busy,
                morale_reaction: false,
                locomotion: loco(donut_loco),
                press_this_tick: view.donut_press_edge,
            },
        );
        self.donut_pose = match (selection.layer, selection.overlay) {
            (Layer::Overlay, Some(kind)) => {
                let age = overlay_age(
                    self.donut_overlay_total,
                    self.donut_overlay.running().map(|(_, left)| left),
                );
                Some(donut_overlay_pose(&self.art.donut, kind, age))
            }
            _ => None,
        };

        // ----------------------------------------------------------- goblins
        let overlays = self.overlays_on();
        for index in 0..self.goblins.len() {
            let goblin = &combat.enemies[index];
            let variant = super::goblin_variant(index);
            let acting = &mut self.goblins[index];
            acting.hold_left = acting.hold_left.saturating_sub(1);
            let (tag, flee_dir) = match combat.goblin_reaction(index) {
                Reaction::Hesitate { .. } => (ReactionTag::Hesitate, 0),
                Reaction::Panic { flee_dir, .. } => (ReactionTag::Panic, flee_dir),
                Reaction::Taunt { .. } => (ReactionTag::Taunt, 0),
                Reaction::Neutral => (ReactionTag::None, 0),
                _ => (ReactionTag::Other, 0),
            };
            // The taunt hold is set by a hit event below (after the reaction
            // moved on), so the age it freezes is the previous tick's.
            if tag == acting.reaction {
                acting.reaction_age += 1;
            } else {
                acting.reaction = tag;
                acting.reaction_age = 0;
            }
            let reaction = match tag {
                ReactionTag::Hesitate => Some((GoblinReactionArt::Hesitate, acting.reaction_age)),
                ReactionTag::Panic => {
                    Some((GoblinReactionArt::Panic { flee_dir }, acting.reaction_age))
                }
                ReactionTag::Taunt => Some((GoblinReactionArt::Taunt, acting.reaction_age)),
                _ => None,
            };
            if goblin.alive() {
                acting.dead_age = None;
                acting.landed_age = None;
            } else {
                // Counted in presentation ticks from the first tick the slammed
                // frame may show: while a Q72 taunt hold runs the taunt frame is
                // drawn and the impact has not begun (D's hold starts after it).
                if acting.hold_left == 0 {
                    acting.dead_age = Some(acting.dead_age.map_or(0, |age| age + 1));
                }
                acting.landed_age = if goblin.mode == GoblinMode::Airborne {
                    None
                } else {
                    Some(acting.landed_age.map_or(0, |a| a + 1))
                };
            }
            acting.input = GoblinActInput {
                mode: goblin.mode,
                alive: goblin.alive(),
                reaction,
                taunt_hold: (acting.hold_left > 0).then_some(acting.hold_frame),
            };
            acting.pose = goblin_acting_pose(&self.art.goblins[variant], &acting.input, overlays);
        }
    }

    fn note_whiff(&mut self, tick: u64) {
        self.whiffs.push(tick);
        let keep = usize::from(self.tuning.q70_frustration_moves).max(1);
        if self.whiffs.len() > keep {
            self.whiffs.remove(0);
        }
    }

    /// Q71 startle: whether an event at `x` is within the distance of Donut.
    fn within_startle(&self, scene: &SceneSim, x: f32) -> bool {
        (scene.world.donut.x - x).abs() <= f32::from(self.tuning.q71_startle_px)
    }

    /// Start a Donut overlay and its cooldown (the caller checked she can show
    /// it, so the cooldown is only spent on an overlay that is drawn).
    fn start_donut(&mut self, kind: DonutOverlay) {
        let t = self.tuning;
        let (ticks, cooldown) = match kind {
            DonutOverlay::Startle => (t.q71_startle_ticks, t.q71_startle_cooldown),
            DonutOverlay::Smug => (t.q71_smug_ticks, t.q71_smug_cooldown),
        };
        if self.donut_overlay.start(t, kind, ticks) {
            self.donut_overlay_total = ticks;
            match kind {
                DonutOverlay::Startle => self.startle_cooldown = cooldown,
                DonutOverlay::Smug => self.smug_cooldown = cooldown,
            }
        }
    }

    /// Q72: a non-launching hit on a taunting goblin holds the taunt frame.
    fn note_hit_on_goblin(&mut self, target: usize, combat: &CombatState) {
        let Some(goblin) = combat.enemies.get(target) else {
            return;
        };
        let launched = matches!(goblin.mode, GoblinMode::Launched | GoblinMode::Airborne);
        let hold = self.tuning.q72_taunt_hold_ticks;
        let overlays = crate::acting::overlays_enabled(self.tuning);
        let acting = &mut self.goblins[target];
        if overlays && acting.reaction == ReactionTag::Taunt && !launched && hold > 0 {
            // The frame shown on the tick before the hit.
            let clips = &self.art.goblins[super::goblin_variant(target)];
            let cycle = clips.tag_ticks("taunt").unwrap_or(1);
            let frame = clips.frame_at("taunt", acting.reaction_age % cycle) - clips.tag("taunt").0;
            // `hold_left` is consumed at the start of the next step; +1 so this
            // tick (the hit tick) counts as the first shown tick.
            acting.hold_left = hold + 1;
            acting.hold_frame = frame;
        }
    }

    /// The text appended to the `COMBAT_ANIM_STATE` line (empty without the
    /// CF02 art, so legacy lines are byte-identical).
    pub fn state_suffix(&self) -> String {
        let label = |p: &Option<ArtPose>| p.map_or_else(|| "-".to_owned(), |p| p.label());
        let goblins: Vec<String> = (0..self.goblins.len())
            .map(|i| {
                self.goblin_pose(i)
                    .map_or_else(|| "-".to_owned(), |p| p.pose.label())
            })
            .collect();
        // CF02-FU-I FU5: the slammed clock of each goblin (ticks since its slammed
        // frame may show, and since it landed), `-` while it lives.
        let slammed: Vec<String> = self
            .goblins
            .iter()
            .map(|g| match (g.dead_age, g.landed_age) {
                (None, _) => "-".to_owned(),
                (Some(d), None) => format!("{d}/-"),
                (Some(d), Some(l)) => format!("{d}/{l}"),
            })
            .collect();
        format!(
            " acting_carl={} acting_donut={} acting_goblins={} acting_slammed={}",
            label(&self.carl_pose),
            label(&self.donut_pose),
            goblins.join(","),
            slammed.join(",")
        )
    }
}

/// The ticks an overlay has been shown, given its total and the ticks left after
/// this tick's selection.
fn overlay_age(total: u8, left_after: Option<u8>) -> u32 {
    u32::from(total).saturating_sub(1 + u32::from(left_after.unwrap_or(0)))
}

#[cfg(test)]
mod tests;
