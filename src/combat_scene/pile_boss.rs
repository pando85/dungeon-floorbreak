//! HRD-A2: the Hoarder (the pile boss) as a slot enemy.
//!
//! The per-enemy scene module behind ENEMY-SLOT's one hook line
//! (`CombatState::step_slot_enemies` and the hit, blast and threat routes of
//! `combat_scene/slot.rs`): HRD-A2 adds no hook of its own anywhere. It wraps
//! the pure state machine [`crate::pile_boss::PileBoss`] in the
//! [`SlotEnemy`] contract:
//!
//! * the hurtboxes, the hits (Carl's melee, Donut's swat, pounce and Magic
//!   Missile through [`SlotEnemy::apply_hit`], a dynamite blast through
//!   [`SlotEnemy::apply_blast`]) and the Donut target state;
//! * her tick: the heroes' views, the sidecar's static obstacles (bank A, the
//!   J-B post) and the exit door while it is closed, in; her blows, the
//!   push-out shifts, the solo bot regroup and her events, out;
//! * her `boss: Option<BossThreat>` supply (the one supplier of her room);
//! * the defeat, which is the existing `CombatEvent::EnemyDefeated { target:
//!   PILE_BOSS_ID }` and nothing else: the story ledger's event trigger reads
//!   it (HRD-A2's generic `EventMatch::EnemyDefeated`), so the dynamite is
//!   granted by the ledger and never by code here. [`crate::pile_boss::PileOut::story_trigger`]
//!   is telemetry only and is dropped at this boundary.
//!
//! Her non-defeat events leave as `CombatEvent::PileBoss { tick, event }`
//! (HRD-Q40). The room's solids that she owns (the entry gate, the exit door)
//! reach hero collision through the runtime env solids sync (`combat_scene/env.rs`):
//! the gate from [`SlotEnemy::gate_solid`], the door from her `alive`.
//!
//! The registry row is `hoarder` ([`KIND`], the roster kind string).

use crate::boss::BossThreat;
use crate::combat::{CombatEvent, DefeatCause};
use crate::donut_moves::TargetState;
use crate::encounter::Encounter;
use crate::env_hazard::HazardClass;
use crate::env_prop::Stimulus;
use crate::pile_boss::{
    HitReaction, Obstacle, ObstacleKind, PILE_BOSS_ID, PileBoss, PileBossTuning, PileCtx, PileHit,
    PileMode, PileOut, PileSetup, obstacles_from_encounter,
};
use crate::prototype::Rect;
use crate::scene_enemy::{
    EnvFace, FaceKind, SlotBlast, SlotBuild, SlotDeathBody, SlotEnemy, SlotEnvHit, SlotError,
    SlotHit, SlotHitResult, SlotOut, SlotRoom, SlotStepCtx, SlotStimulus,
};

/// The registry name of the kind: the roster kind string.
pub const KIND: &str = "hoarder";
/// The death-profile species key (HRD sheet section 10). The cf02 overlay maps every
/// recipe to `pile_boss_down`, which the generic death presenter draws as nothing (no gibs,
/// no blood): her own presenter (`hoarder_art`, FA-Q23) draws HOARD-01's `down` frames.
pub const SPECIES: &str = "pile_boss";
/// The wall columns of the room kit (16 px, the kit's tile): her `walls` are the
/// scene's width less one column on each side, so her push-out never places a
/// hero inside a wall (a test checks it against the composed kit).
pub const WALL_COLUMN_PX: f32 = 16.0;

/// The Hoarder in the slot.
#[derive(Debug, Clone)]
pub struct PileBossEnemy {
    boss: PileBoss,
    /// Bank A, the post and the closed door, in world px.
    closed: Vec<Obstacle>,
    /// The same with the door open (her defeat).
    open: Vec<Obstacle>,
    /// Fixture hook: bank A removed ("the settled case"): its face is never an
    /// obstacle, whatever the env says.
    bank_a_removed: bool,
}

impl PileBossEnemy {
    /// Build her from the room's sidecar (bank A, the post and the exit lock)
    /// and the room facts of the scene. She starts Dormant at her home with the
    /// entry gate open.
    pub fn from_encounter(
        encounter: &Encounter,
        room: SlotRoom,
        tuning: &'static PileBossTuning,
    ) -> Self {
        // The validator pins the protected spawn to the walking line, so its
        // bottom is the layout's floor (`Encounter` has no floor field).
        let layout_floor_y = encounter.protected_spawn.bottom();
        let setup = PileSetup {
            floor_y: room.floor_y,
            walls: (room.walls.0 + WALL_COLUMN_PX, room.walls.1 - WALL_COLUMN_PX),
        };
        // HRD-B / HRD-C: bank A and the post that carry a machine are faces the env
        // reports while they stand (`SlotStepCtx::env`), never a static obstacle.
        let bank_live = encounter.statics.iter().any(|s| s.slide.is_some());
        let post_live = encounter.statics.iter().any(|s| s.stack.is_some());
        let fixed = |door_open: bool| {
            let mut list =
                obstacles_from_encounter(encounter, layout_floor_y, room.floor_y, door_open);
            list.retain(|o| match o.kind {
                ObstacleKind::BankA => !bank_live,
                ObstacleKind::Post => !post_live,
                _ => true,
            });
            list
        };
        Self {
            boss: PileBoss::new(tuning, setup),
            closed: fixed(false),
            open: fixed(true),
            bank_a_removed: false,
        }
    }

    /// Fixture hook of the sheet: awake in Neutral at pivot `x`, facing
    /// `facing`, the entry gate closed ([`PileBoss::awake_at`]).
    #[must_use]
    pub fn awake_at(mut self, x: f32, facing: i8) -> Self {
        self.boss = PileBoss::awake_at(self.boss.tuning(), self.setup(), x, facing);
        self
    }

    /// Fixture hook: preset the shove's own cooldown.
    #[must_use]
    pub fn with_shove_cooldown(mut self, ticks: u32) -> Self {
        self.boss = self.boss.with_shove_cooldown(ticks);
        self
    }

    /// Fixture hook: preset her hit points (at least 1).
    #[must_use]
    pub fn with_hp(mut self, hp: u16) -> Self {
        self.boss = self.boss.with_hp(hp);
        self
    }

    /// Fixture hook: remove bank A from her obstacles (the settled case of the
    /// sheet's fixtures: "bank A removed by fixture setup").
    #[must_use]
    pub fn without_bank_a(mut self) -> Self {
        self.bank_a_removed = true;
        for list in [&mut self.closed, &mut self.open] {
            list.retain(|o| o.kind != crate::pile_boss::ObstacleKind::BankA);
        }
        self
    }

    /// Whether the fixture removed bank A (the scene removes its machine too).
    pub fn bank_a_removed(&self) -> bool {
        self.bank_a_removed
    }

    fn setup(&self) -> PileSetup {
        self.boss.setup()
    }

    /// The pure state machine (read only: fixtures and the scene read her state).
    pub fn boss(&self) -> &PileBoss {
        &self.boss
    }

    /// The exit door is open from her defeat tick on.
    pub fn door_open(&self) -> bool {
        self.boss.door_open()
    }

    /// Fold what the pure core asks of the scene into the slot's output. Her
    /// non-defeat events lead the existing ones: a lethal hit logs its `Hurt`
    /// before the `EnemyDefeated`, as the core produced them. The CP-A trigger
    /// name is dropped: the defeat event is the only grant path.
    fn absorb(&self, pile: PileOut, faces: &[EnvFace], out: &mut SlotOut) {
        // HRD-B / HRD-C: her shove meeting a pile is a heavy stimulus on its prop
        // (the sheet's `EnemyImpact`): the lunge speed, no credit when her target
        // was a bot hero.
        let impulse = u16::try_from(u32::from(self.boss.tuning().hrdq11_px_per_tick) * 60)
            .unwrap_or(u16::MAX);
        for jam in &pile.jams {
            let kind = match jam.obstacle {
                ObstacleKind::BankA => FaceKind::Bank,
                ObstacleKind::Post => FaceKind::Post,
                ObstacleKind::Solid | ObstacleKind::Remnant => continue,
            };
            if let Some(face) = faces.iter().find(|f| f.kind == kind) {
                out.env_stimuli.push(SlotStimulus {
                    prop: face.prop,
                    stimulus: Stimulus::Shove {
                        shove_id: u32::try_from(jam.tick).unwrap_or(u32::MAX),
                        impulse_px_s: impulse,
                        source: PILE_BOSS_ID,
                        credit: jam.credit,
                    },
                });
            }
        }
        out.events.extend(
            pile.pile_events
                .iter()
                .map(|&(tick, event)| CombatEvent::PileBoss { tick, event }),
        );
        out.events.extend(pile.events);
        out.blows.extend(pile.blows.iter().map(|b| b.blow));
        out.shifts.extend(pile.shifts);
        if let Some(hero) = pile.regroup {
            out.regroup
                .push((hero, f32::from(self.boss.tuning().hrdq41_regroup_x)));
        }
    }

    fn land(
        &mut self,
        hit: PileHit,
        spec: Option<&'static crate::combat::MoveSpec>,
        tick: u64,
        out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        let mut pile = PileOut::default();
        let result = self.boss.apply_hit(hit, spec, tick, &mut pile)?;
        self.absorb(pile, &[], out);
        Some(SlotHitResult {
            hp_left: result.hp_left,
            defeated: result.defeated,
        })
    }
}

impl SlotEnemy for PileBossEnemy {
    fn kind(&self) -> &'static str {
        KIND
    }

    fn target_id(&self) -> u32 {
        PILE_BOSS_ID
    }

    fn box_clone(&self) -> Box<dyn SlotEnemy> {
        Box::new(self.clone())
    }

    fn alive(&self) -> bool {
        self.boss.alive()
    }

    fn hp(&self) -> u16 {
        self.boss.hp()
    }

    fn hurt_rects(&self) -> Vec<Rect> {
        self.boss.hurt_rects()
    }

    /// Her swat sees: `Neutral` exactly in the modes that count poise, any other
    /// mode glances with none (the Juicer's rule).
    fn donut_target_state(&self) -> TargetState {
        if self.boss.mode().reaction() == HitReaction::Poise {
            TargetState::Neutral
        } else {
            TargetState::Recovery
        }
    }

    fn supplies_boss(&self) -> bool {
        true
    }

    fn threat(&self) -> Option<BossThreat> {
        self.boss.threat()
    }

    fn death_body(&self) -> Option<SlotDeathBody> {
        Some(SlotDeathBody {
            x: self.boss.x(),
            ground_y: self.setup().floor_y,
            facing: self.boss.facing(),
            species: SPECIES,
        })
    }

    fn step(&mut self, ctx: &SlotStepCtx<'_>, out: &mut SlotOut) {
        let pile = {
            let obstacles = if self.boss.door_open() {
                &self.open
            } else {
                &self.closed
            };
            let mut all: Vec<Obstacle> = obstacles.clone();
            for face in &ctx.env.faces {
                let kind = match face.kind {
                    FaceKind::Bank if self.bank_a_removed => continue,
                    FaceKind::Bank => ObstacleKind::BankA,
                    FaceKind::Post => ObstacleKind::Post,
                };
                all.push(Obstacle {
                    kind,
                    rect: face.rect,
                });
            }
            let pctx = PileCtx {
                heroes: ctx.heroes,
                obstacles: &all,
                solo: ctx.solo,
                tick: ctx.tick,
            };
            self.boss.step(&pctx)
        };
        self.absorb(pile, &ctx.env.faces, out);
    }

    fn apply_hit(&mut self, hit: &SlotHit, tick: u64, out: &mut SlotOut) -> Option<SlotHitResult> {
        self.land(
            PileHit {
                damage: hit.damage,
                poise: hit.poise,
                player: hit.player,
                hit_stop: hit.hit_stop,
                cause: DefeatCause::Melee,
            },
            Some(hit.spec),
            tick,
            out,
        )
    }

    /// A blast hurts her by the profile's boss damage anywhere in reach (CP-Q25,
    /// HRD sheet section 4.3: dynamite 4, test loadout only), counted toward poise
    /// as damage like the Juicer's, never launched. A zero `boss_damage` (the QA
    /// profile) does nothing.
    fn apply_blast(
        &mut self,
        blast: &SlotBlast,
        tick: u64,
        out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        let damage = u16::from(blast.charge.profile.boss_damage);
        if damage == 0 {
            return None;
        }
        self.land(
            PileHit {
                damage,
                poise: damage,
                player: blast.charge.owner.unwrap_or(crate::shore::PLAYER_ONE),
                hit_stop: 0,
                cause: blast.cause(),
            },
            None,
            tick,
            out,
        )
    }

    /// HRD-C: the J-B stack landing on her body (HRD-Q18): the exposure, never a
    /// kill. Any other env hit leaves her immune.
    fn apply_env(
        &mut self,
        hit: &SlotEnvHit,
        tick: u64,
        out: &mut SlotOut,
    ) -> Option<SlotHitResult> {
        if hit.class != HazardClass::FallingMass {
            return None;
        }
        let mut pile = PileOut::default();
        if !self.boss.expose(hit.initiator, tick, &mut pile) {
            return None;
        }
        self.absorb(pile, &[], out);
        Some(SlotHitResult {
            hp_left: self.boss.hp(),
            defeated: false,
        })
    }

    fn engaged(&self) -> bool {
        self.boss.mode() != PileMode::Dormant
    }

    fn gate_solid(&self) -> Option<Rect> {
        self.boss.gate_solid()
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn on_retry(&mut self) {
        self.boss.restart();
    }

    fn mix_into(&self, mix: &mut dyn FnMut(u64)) {
        self.boss.mix_into(&mut |v| mix(v));
    }
}

/// The Hoarder of a combat state, when its slot holds her (fixtures and tests).
pub fn of(combat: &crate::combat_scene::CombatState) -> Option<&PileBossEnemy> {
    combat
        .slot_enemies()
        .iter()
        .find_map(|e| e.as_any()?.downcast_ref::<PileBossEnemy>())
}

/// The registry builder of the `hoarder` roster kind ([`crate::scene_enemy::SLOT_KINDS`]).
///
/// Refuses a spawn without a sidecar, and a roster `x` that is not her home
/// (the pure core starts her at `hrdq19_home_x`; a divergent roster column would
/// be silently ignored otherwise).
pub fn build(build: &SlotBuild<'_>) -> Result<Box<dyn SlotEnemy>, SlotError> {
    let encounter = build.encounter.ok_or_else(|| SlotError::Build {
        kind: KIND,
        reason: "the Hoarder spawns from a sidecar roster".to_owned(),
    })?;
    let tuning = PileBossTuning::active();
    let home = f32::from(tuning.hrdq19_home_x);
    if (build.entry.x - home).abs() > f32::EPSILON {
        return Err(SlotError::Build {
            kind: KIND,
            reason: format!(
                "roster {}: x {} is not her home column {home} (hrdq19_home_x)",
                build.entry.id, build.entry.x
            ),
        });
    }
    Ok(Box::new(PileBossEnemy::from_encounter(
        encounter, build.room, tuning,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blast::{BlastCharge, BlastProfile, BlastReach};
    use crate::boss::HeroView;
    use crate::combat::{DONUT_SWAT, MoveId};
    use crate::party::HeroId;
    use crate::pile_boss::PileMode;
    use crate::scene_enemy::{HitSource, SLOT_KINDS, spawn_from_roster};
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn room() -> SlotRoom {
        SlotRoom {
            floor_y: 80.0,
            walls: (0.0, 1056.0),
        }
    }

    fn enemy() -> PileBossEnemy {
        let run = crate::f01_hoarder::launch(&root()).unwrap();
        PileBossEnemy::from_encounter(&run.loaded.encounter, room(), PileBossTuning::active())
    }

    fn hit(damage: u16, poise: u16) -> SlotHit {
        SlotHit {
            source: HitSource::CarlMelee(MoveId::Jab),
            instance: 1,
            damage,
            poise,
            player: 0,
            hit_stop: 0,
            facing: 1,
            spec: &crate::combat::CARL_JAB,
        }
    }

    fn view(id: HeroId, x: f32, bot: bool) -> HeroView {
        HeroView {
            id,
            present: true,
            x,
            foot_y: 80.0,
            grounded: true,
            hurt: Rect {
                left: x - 12.0,
                bottom: 80.0,
                width: 24.0,
                height: 52.0,
            },
            vulnerable: true,
            bot,
        }
    }

    fn step_ctx<'a>(
        heroes: [HeroView; 2],
        tick: u64,
        solo: bool,
        env: &'a crate::scene_enemy::EnvView,
    ) -> SlotStepCtx<'a> {
        SlotStepCtx {
            tick,
            heroes,
            solids: &[],
            env,
            solo,
            room: room(),
            view_x: (0.0, 1152.0),
            token_free: true,
            evade: None,
        }
    }

    #[test]
    fn the_registry_row_is_the_roster_kind_and_the_room_spawns_her() {
        assert!(SLOT_KINDS.iter().any(|d| d.name == KIND));
        let run = crate::f01_hoarder::launch(&root()).unwrap();
        let spawn = spawn_from_roster(&run.loaded.encounter, room(), false).unwrap();
        assert!(spawn.existing.is_empty());
        assert_eq!(spawn.slot.len(), 1);
        let (_, scene, combat) = {
            let (r, s, c) = crate::f01_hoarder::start(
                &root(),
                360,
                crate::carl_prog::CarlProgFlags::OFF,
                crate::donut_prog::DonutProgFlags::OFF,
            )
            .unwrap();
            (r, s, c)
        };
        let _ = scene;
        let her = of(&combat).expect("her slot enemy");
        assert_eq!(her.kind(), KIND);
        assert_eq!(her.target_id(), PILE_BOSS_ID);
        assert!(her.supplies_boss());
        assert_eq!(her.boss().mode(), PileMode::Dormant);
        assert!(her.hurt_rects().is_empty(), "no hurtbox while dormant");
    }

    #[test]
    fn the_builder_refuses_a_roster_column_that_is_not_her_home() {
        let run = crate::f01_hoarder::launch(&root()).unwrap();
        let mut encounter = run.loaded.encounter.clone();
        encounter.roster[0].x = 700;
        let err = spawn_from_roster(&encounter, room(), false).unwrap_err();
        assert!(err.to_string().contains("home column"), "{err}");
    }

    #[test]
    fn a_lethal_hit_logs_the_hurt_then_the_existing_defeat_and_nothing_else() {
        let mut her = enemy().awake_at(760.0, -1).with_hp(1);
        let mut out = SlotOut::default();
        let result = her.apply_hit(&hit(1, 1), 9, &mut out).unwrap();
        assert!(result.defeated && result.hp_left == 0);
        assert!(matches!(
            out.events[0],
            CombatEvent::PileBoss {
                event: crate::pile_boss::PileEvent::Hurt { hp_left: 0, .. },
                ..
            }
        ));
        assert!(matches!(
            out.events[1],
            CombatEvent::EnemyDefeated { target, instigator: Some(0), .. } if target == PILE_BOSS_ID
        ));
        assert_eq!(
            out.events.len(),
            2,
            "no grant, no trigger event: {:?}",
            out.events
        );
        assert!(!her.alive() && her.door_open());
        // A later hit is refused.
        let mut again = SlotOut::default();
        assert!(her.apply_hit(&hit(1, 1), 10, &mut again).is_none());
    }

    #[test]
    fn a_dynamite_blast_hurts_her_by_the_profile_and_the_qa_profile_does_nothing() {
        let blast = |profile: BlastProfile| SlotBlast {
            charge_index: 0,
            charge: BlastCharge::new(740.0, 80.0, 1, Some(0)).with_profile(profile),
            reach: BlastReach::Outer,
            away: -1,
            launch: 1,
        };
        let dynamite = BlastProfile {
            core_radius: 20.0,
            outer_radius: 64.0,
            boss_damage: 4,
            ..BlastProfile::QA
        };
        let mut her = enemy().awake_at(760.0, -1);
        let mut out = SlotOut::default();
        let result = her.apply_blast(&blast(dynamite), 5, &mut out).unwrap();
        assert_eq!(result.hp_left, 26, "30 - 4");
        let mut none = SlotOut::default();
        assert!(
            her.apply_blast(&blast(BlastProfile::QA), 6, &mut none)
                .is_none()
        );
        assert!(none.is_empty());
    }

    #[test]
    fn a_blast_kill_is_an_explosion_credited_to_the_owner() {
        let mut her = enemy().awake_at(760.0, -1).with_hp(1);
        let blast = SlotBlast {
            charge_index: 3,
            charge: BlastCharge::new(740.0, 80.0, 1, Some(0)).with_profile(BlastProfile {
                boss_damage: 4,
                core_radius: 20.0,
                outer_radius: 64.0,
                ..BlastProfile::QA
            }),
            reach: BlastReach::Ring,
            away: -1,
            launch: 1,
        };
        let mut out = SlotOut::default();
        her.apply_blast(&blast, 5, &mut out).unwrap();
        assert!(out.events.iter().any(|e| matches!(
            e,
            CombatEvent::EnemyDefeated {
                cause: DefeatCause::Explosion { charge: 3, .. },
                ..
            }
        )));
    }

    #[test]
    fn the_solo_bot_is_regrouped_to_the_data_column() {
        let mut her = enemy();
        let env = crate::scene_enemy::EnvView::default();
        let mut placed = None;
        // Carl crosses the engage line at tick 1; the bot stays outside.
        for tick in 1..=130 {
            let heroes = [
                view(HeroId::Carl, 440.0, false),
                view(HeroId::Donut, 100.0, true),
            ];
            let mut out = SlotOut::default();
            her.step(&step_ctx(heroes, tick, true, &env), &mut out);
            if let Some(entry) = out.regroup.first() {
                placed = Some((tick, *entry));
                break;
            }
        }
        let (tick, (hero, x)) = placed.expect("a regroup");
        assert_eq!((tick, hero, x), (121, HeroId::Donut, 248.0));
    }

    #[test]
    fn her_swipe_is_a_pound_with_no_target_and_the_donut_state_follows_the_poise_modes() {
        let mut her = enemy().awake_at(760.0, -1);
        let env = crate::scene_enemy::EnvView::default();
        assert_eq!(her.donut_target_state(), TargetState::Neutral);
        let heroes = [
            view(HeroId::Carl, 700.0, false),
            view(HeroId::Donut, 0.0, true),
        ];
        let mut out = SlotOut::default();
        her.step(&step_ctx(heroes, 1, false, &env), &mut out);
        assert_eq!(her.boss().mode(), PileMode::SwipeTell);
        assert_eq!(
            her.donut_target_state(),
            TargetState::Recovery,
            "a tell glances"
        );
        assert!(matches!(
            her.threat(),
            Some(BossThreat::Pound { target: None, .. })
        ));
        assert!(matches!(
            out.events[0],
            CombatEvent::PileBoss {
                event: crate::pile_boss::PileEvent::Tell { .. },
                ..
            }
        ));
    }

    #[test]
    fn a_retry_gives_the_short_reveal_and_the_state_of_a_restarted_core() {
        let mut her = enemy();
        let mut again = her.clone();
        again.on_retry();
        let hash = |e: &PileBossEnemy| {
            let mut h = 0u64;
            e.mix_into(&mut |v| h = h.rotate_left(5) ^ v);
            h
        };
        assert_ne!(
            hash(&her),
            hash(&again),
            "the retry flag is part of the state"
        );
        her.on_retry();
        assert_eq!(hash(&her), hash(&again));
    }

    #[test]
    fn the_swat_is_a_poise_only_glance_that_counts_in_neutral() {
        // The slot hands her the swat as poise 1 with 0 damage.
        let mut her = enemy().awake_at(760.0, -1);
        let swat = SlotHit {
            source: HitSource::DonutSwat,
            instance: 1,
            damage: 0,
            poise: 1,
            player: 1,
            hit_stop: 0,
            facing: 0,
            spec: &DONUT_SWAT,
        };
        let mut out = SlotOut::default();
        let result = her.apply_hit(&swat, 3, &mut out).unwrap();
        assert_eq!(result.hp_left, 30);
        assert!(out.events.is_empty(), "no Hurt for a 0-damage glance");
    }
}
