//! The `ap2a_*` party roster, the companion registry and the `PartyTuning`
//! encoding hash (AP-Q24, AP-Q25).
//!
//! No existing party fixture is a roster scenario (AP-Q25): they are feature
//! probes or need two humans, so the parity test uses them through a
//! crate-private constructor only. The roster is new `static PartyFixture`s on
//! the corridor kit with the v1 roster columns, `two_player: false`, no script
//! and no pilot. Each layout is instantiated per available `uncontrolled_hero`
//! value by [`roster_for!`], so the tuning comes from statics and
//! `PartyFixtureSim::new` stays the only construction path.

use crate::combat::EnemyAi;
use crate::combat_scene::{EnemySpawn, FIXTURES, Fixture};
use crate::donut_mana::DonutSetup;
use crate::input::RawInput;
use crate::party::{
    CameraFollow, DonutHurtbox, EnemyTargetRule, PROVISIONAL_TUNING, PartyTuning, PlateGlyphs,
    SwapPanMode, UncontrolledHero,
};
use crate::party_scene::{PartyFixture, base};

use super::super::error::AutoplayError;
use super::super::scenario::{self as v1, Split};
use crate::boss_scene::Pilot;

/// The companion id available on this build (AP-Q24).
pub const FOLLOW_AND_JUMP_BOT: &str = "follow_and_jump_bot";

/// Companion ids COMPANION-AI-01 adds with 01A-wire (and the kill-policy
/// selector for the last one). Known, but not backed by a variant yet.
const PLANNED_COMPANIONS: [&str; 3] = ["cf02_assist", "companion_v0_never", "companion_v0_factual"];

/// Registry ids selectable on this build.
pub const COMPANION_IDS: [&str; 1] = [FOLLOW_AND_JUMP_BOT];

/// Number of roster layouts per companion.
pub const ROSTER_LEN: usize = 6;

/// One roster entry: the layout's party fixture and the pre-entry
/// `PartyFixtureSim::state_hash` of a fresh sim (AP-Q11, AP-Q23).
#[derive(Debug, Clone, Copy)]
pub struct PartyRosterEntry {
    pub id: &'static str,
    pub split: Split,
    pub fixture: &'static PartyFixture,
    pub initial_hash: u64,
    /// The companion registry id this instantiation belongs to.
    pub companion: &'static str,
    /// `UncontrolledHero` debug name recorded in the header (AP-Q24).
    pub uncontrolled_hero: &'static str,
}

/// Instantiate every roster layout for one `uncontrolled_hero` value. The six
/// literals are the pinned initial hashes in roster order. Two goblin layouts
/// repeat the v1 columns (`ap_*`), the biscuit variants add
/// `DonutSetup::unlocked(8)` so Magic Missile and mana use are reachable.
macro_rules! roster_for {
    ($module:ident, $id:expr, $uncontrolled:expr, $label:expr, [$h0:expr, $h1:expr, $h2:expr, $h3:expr, $h4:expr, $h5:expr]) => {
        mod $module {
            use super::*;

            const TUNING: PartyTuning = PartyTuning {
                uncontrolled_hero: $uncontrolled,
                ..PROVISIONAL_TUNING
            };

            const fn chase(x: i32) -> EnemySpawn {
                EnemySpawn {
                    x,
                    ai: EnemyAi::Chase,
                }
            }

            static SINGLE_ENEMIES: [EnemySpawn; 1] = [chase(560)];
            static WIDE_ENEMIES: [EnemySpawn; 2] = [chase(480), chase(704)];
            static PAIR_ENEMIES: [EnemySpawn; 2] = [chase(520), chase(552)];
            static FAR_ENEMIES: [EnemySpawn; 1] = [chase(776)];

            static SINGLE_BASE: Fixture = base(
                "ap2a_single_goblin",
                "autoplay party: one chasing goblin, Donut as the companion",
                0xA902_0001,
                &SINGLE_ENEMIES,
                600,
            );
            static WIDE_BASE: Fixture = base(
                "ap2a_two_goblins_wide",
                "autoplay party: two chasing goblins far apart",
                0xA902_0002,
                &WIDE_ENEMIES,
                600,
            );
            static WIDE_BISCUIT_BASE: Fixture = base(
                "ap2a_two_goblins_wide_biscuit",
                "autoplay party: two chasing goblins far apart, Donut has the biscuit",
                0xA902_0003,
                &WIDE_ENEMIES,
                600,
            );
            static PAIR_BASE: Fixture = base(
                "ap2a_two_goblins",
                "autoplay party: two chasing goblins side by side",
                0xA902_0004,
                &PAIR_ENEMIES,
                600,
            );
            static FAR_BASE: Fixture = base(
                "ap2a_single_goblin_far",
                "autoplay party: one chasing goblin that starts off-screen",
                0xA902_0005,
                &FAR_ENEMIES,
                600,
            );
            static PAIR_BISCUIT_BASE: Fixture = base(
                "ap2a_two_goblins_biscuit",
                "autoplay party: two goblins side by side, Donut has the biscuit",
                0xA902_0006,
                &PAIR_ENEMIES,
                600,
            );

            const fn fixture(base: &'static Fixture, donut: DonutSetup) -> PartyFixture {
                PartyFixture {
                    base,
                    two_player: false,
                    tuning: TUNING,
                    script: &[],
                    pilot: Pilot::None,
                    donut,
                }
            }

            static SINGLE: PartyFixture = fixture(&SINGLE_BASE, DonutSetup::NONE);
            static WIDE: PartyFixture = fixture(&WIDE_BASE, DonutSetup::NONE);
            static WIDE_BISCUIT: PartyFixture =
                fixture(&WIDE_BISCUIT_BASE, DonutSetup::unlocked(8));
            static PAIR: PartyFixture = fixture(&PAIR_BASE, DonutSetup::NONE);
            static FAR: PartyFixture = fixture(&FAR_BASE, DonutSetup::NONE);
            static PAIR_BISCUIT: PartyFixture =
                fixture(&PAIR_BISCUIT_BASE, DonutSetup::unlocked(8));

            const fn entry(
                split: Split,
                fixture: &'static PartyFixture,
                initial_hash: u64,
            ) -> PartyRosterEntry {
                PartyRosterEntry {
                    id: fixture.base.name,
                    split,
                    fixture,
                    initial_hash,
                    companion: $id,
                    uncontrolled_hero: $label,
                }
            }

            /// Development first, then hold-out (AP-Q25).
            pub(super) fn entries() -> [PartyRosterEntry; ROSTER_LEN] {
                [
                    entry(Split::Development, &SINGLE, $h0),
                    entry(Split::Development, &WIDE, $h1),
                    entry(Split::Development, &WIDE_BISCUIT, $h2),
                    entry(Split::Holdout, &PAIR, $h3),
                    entry(Split::Holdout, &FAR, $h4),
                    entry(Split::Holdout, &PAIR_BISCUIT, $h5),
                ]
            }
        }
    };
}

roster_for!(
    follow_and_jump,
    FOLLOW_AND_JUMP_BOT,
    UncontrolledHero::FollowAndJumpBot,
    "FollowAndJumpBot",
    [
        0xbde5_4184_e0c6_befb,
        0x89de_e33c_081a_bf93,
        0x5e88_a024_1da9_32d7,
        0xc9c6_b42f_bbb1_d698,
        0xbbf1_59d5_a5e5_267d,
        0xfc59_516e_a203_d277
    ]
);

/// Resolve a companion id to the roster instantiated for it (AP-Q24). An id
/// whose backing variant does not exist on this build is `UnsupportedCompanion`.
pub fn roster(companion: &str) -> Result<[PartyRosterEntry; ROSTER_LEN], AutoplayError> {
    match companion {
        FOLLOW_AND_JUMP_BOT => Ok(follow_and_jump::entries()),
        id if PLANNED_COMPANIONS.contains(&id) => {
            Err(AutoplayError::UnsupportedCompanion(format!(
                "{id} (needs COMPANION-AI-01 01A-wire; available: {})",
                COMPANION_IDS.join(", ")
            )))
        }
        id => Err(AutoplayError::UnsupportedCompanion(format!(
            "{id} (not in the registry; available: {})",
            COMPANION_IDS.join(", ")
        ))),
    }
}

/// Resolve a scenario id for a companion; any other id is `UnsupportedScenario`.
pub fn lookup(id: &str, companion: &str) -> Result<PartyRosterEntry, AutoplayError> {
    let roster = roster(companion)?;
    roster.iter().copied().find(|e| e.id == id).ok_or_else(|| {
        let supported: Vec<&str> = roster.iter().map(|e| e.id).collect();
        AutoplayError::UnsupportedScenario(format!("{id} (supported: {})", supported.join(", ")))
    })
}

/// Layout ids in roster order (companion independent).
pub fn layout_ids() -> [&'static str; ROSTER_LEN] {
    follow_and_jump::entries().map(|e| e.id)
}

// ------------------------------------------------------ the tuning encoding --

/// Tag byte of an `UncontrolledHero` (exhaustive match, never `as`).
fn uncontrolled_tag(value: UncontrolledHero) -> u8 {
    match value {
        UncontrolledHero::TrailFollowNeverAttacks => 0,
        UncontrolledHero::FollowAndJumpBot => 1,
        // COMPANION-AI-01: the AI companion, per profile set (no autoplay
        // scenario uses it; the tags keep the encoding total and distinct).
        UncontrolledHero::Companion(crate::companion::profile::ProfileSetId::Cf02Assist) => 2,
        UncontrolledHero::Companion(crate::companion::profile::ProfileSetId::CompanionV0) => 3,
    }
}

fn target_tag(value: EnemyTargetRule) -> u8 {
    match value {
        EnemyTargetRule::NearestLivingTieSlot0 => 0,
        EnemyTargetRule::HysteresisControlledBias => 1,
    }
}

fn hurtbox_tag(value: DonutHurtbox) -> u8 {
    match value {
        DonutHurtbox::SameAsCarl => 0,
        DonutHurtbox::FitBody => 1,
        DonutHurtbox::Forgiving => 2,
    }
}

fn camera_tag(value: CameraFollow) -> u8 {
    match value {
        CameraFollow::ControlledSoloMidpointTwoPlayer => 0,
    }
}

fn glyph_tag(value: PlateGlyphs) -> u8 {
    match value {
        PlateGlyphs::OwnPad => 0,
    }
}

/// Fixed placeholder for the `uncontrolled_hero` slot of the layout
/// fingerprint, so one layout shares a fingerprint across companions.
const UNCONTROLLED_PLACEHOLDER: u8 = 0xFF;

/// Explicit field-by-field encoding of a `PartyTuning`: each field's name and
/// its value in declaration order (integers little-endian at fixed width, bools
/// one byte, enums by an exhaustive tag). `PartyTuning` has no floats today; a
/// future float must be encoded with `to_bits`. The destructuring has no `..`,
/// so a new field is a compile error until it is encoded here (AP-Q24).
fn encode_tuning(tuning: &PartyTuning, uncontrolled: Option<u8>) -> Vec<u8> {
    let PartyTuning {
        donut_damage_per_hit,
        donut_invulnerable_ticks,
        donut_hurt_ticks,
        enemy_target,
        target_switch_px,
        target_ai_bias_px,
        donut_hurtbox,
        camera_follow,
        camera_edge_margin_px,
        camera_arrow_px,
        swap_pan_px_per_tick,
        swap_pan_mode,
        swap_blocked_while_hurt,
        swap_cooldown_ticks,
        swap_buffer_ticks,
        uncontrolled_hero,
        follow_distance_px,
        dodge_radius_px,
        regroup_ticks,
        solo_last_hp_guard,
        solo_offscreen_immunity,
        offscreen_margin_px,
        donut_moves,
        plate_glyphs,
    } = *tuning;
    let mut out = Vec::new();
    let mut put = |name: &str, bytes: &[u8]| {
        out.extend_from_slice(name.as_bytes());
        out.push(b'=');
        out.extend_from_slice(bytes);
        out.push(b';');
    };
    put("donut_damage_per_hit", &[donut_damage_per_hit]);
    put("donut_invulnerable_ticks", &[donut_invulnerable_ticks]);
    put("donut_hurt_ticks", &[donut_hurt_ticks]);
    put("enemy_target", &[target_tag(enemy_target)]);
    put("target_switch_px", &target_switch_px.to_le_bytes());
    put("target_ai_bias_px", &target_ai_bias_px.to_le_bytes());
    put("donut_hurtbox", &[hurtbox_tag(donut_hurtbox)]);
    put("camera_follow", &[camera_tag(camera_follow)]);
    put(
        "camera_edge_margin_px",
        &camera_edge_margin_px.to_le_bytes(),
    );
    put("camera_arrow_px", &camera_arrow_px.to_le_bytes());
    put("swap_pan_px_per_tick", &[swap_pan_px_per_tick]);
    // CF02-Q20: written only when it is not the legacy value, so every existing
    // layout fingerprint is unchanged.
    if swap_pan_mode != SwapPanMode::Constant {
        put("swap_pan_mode", &[1]);
    }
    put(
        "swap_blocked_while_hurt",
        &[u8::from(swap_blocked_while_hurt)],
    );
    put("swap_cooldown_ticks", &[swap_cooldown_ticks]);
    put("swap_buffer_ticks", &[swap_buffer_ticks]);
    put(
        "uncontrolled_hero",
        &[uncontrolled.unwrap_or_else(|| uncontrolled_tag(uncontrolled_hero))],
    );
    put("follow_distance_px", &follow_distance_px.to_le_bytes());
    put("dodge_radius_px", &dodge_radius_px.to_le_bytes());
    put("regroup_ticks", &regroup_ticks.to_le_bytes());
    put("solo_last_hp_guard", &[u8::from(solo_last_hp_guard)]);
    put(
        "solo_offscreen_immunity",
        &[u8::from(solo_offscreen_immunity)],
    );
    put("offscreen_margin_px", &offscreen_margin_px.to_le_bytes());
    put("donut_moves", &[u8::from(donut_moves)]);
    put("plate_glyphs", &[glyph_tag(plate_glyphs)]);
    out
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The `party_tuning_hash` of the replay header (AP-Q24): FNV-1a over the
/// explicit field encoding. Never goes through a `Debug` string.
pub fn party_tuning_hash(tuning: &PartyTuning) -> u64 {
    fnv1a(&encode_tuning(tuning, None))
}

/// Same hash with the `uncontrolled_hero` slot replaced by a fixed
/// placeholder: the companion-independent part of the layout fingerprint.
fn layout_tuning_hash(tuning: &PartyTuning) -> u64 {
    fnv1a(&encode_tuning(tuning, Some(UNCONTROLLED_PLACEHOLDER)))
}

/// The Donut setup as a stable string (`locked` or `unlocked:<mana>`), as
/// the /2 header will record it (AP-Q33).
pub fn donut_setup_label(setup: &DonutSetup) -> String {
    let base = if setup.start.unlocked {
        format!("unlocked:{}", setup.start.mana)
    } else {
        "locked".to_string()
    };
    match setup.guild_zone {
        None => base,
        Some((lo, hi)) => format!("{base}+zone:{:08x}:{:08x}", lo.to_bits(), hi.to_bits()),
    }
}

/// The party layout fingerprint (AP-Q25): the v1 fingerprint of the base
/// fixture extended with `two_player`, the companion-independent tuning hash
/// and the Donut setup. Not name, script, seed or ticks.
pub fn layout_fingerprint(fixture: &PartyFixture) -> String {
    format!(
        "party|{}|two_player={}|tuning={:016x}|donut={}",
        v1::layout_fingerprint(fixture.base),
        fixture.two_player,
        layout_tuning_hash(&fixture.tuning),
        donut_setup_label(&fixture.donut)
    )
}

/// Roster validation: the base fixture must fit the v1 rules, and the party
/// layer must be the plain solo, scriptless, pilotless one (AP-Q25).
pub fn validate_fixture(fixture: &PartyFixture) -> Result<(), AutoplayError> {
    v1::validate_fixture(fixture.base)?;
    let name = fixture.base.name;
    if fixture.two_player || !fixture.script.is_empty() || fixture.pilot != Pilot::None {
        return Err(AutoplayError::UnsupportedScenario(format!(
            "{name} is not a plain solo party fixture (two_player, script or pilot set)"
        )));
    }
    Ok(())
}

/// True when `name` collides with a game fixture (party, boss, Donut,
/// companion, combat or any `Fixture::by_name` one).
pub fn collides_with_game_fixtures(name: &str) -> bool {
    PartyFixture::by_name(name).is_some()
        || Fixture::by_name(name).is_some()
        || FIXTURES.iter().any(|f| f.name == name)
}

/// Slot 0 of the seat-entry tick: the seat's swap press for Donut, nothing
/// for Carl (AP-Q23). Slot 1 is always idle.
pub(crate) fn entry_slot(seat: crate::party::HeroId) -> RawInput {
    RawInput {
        swap: seat == crate::party::HeroId::Donut,
        ..RawInput::NONE
    }
}
