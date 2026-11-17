//! CP-01 hit-feel and goblin-attack tuning: every number the owner may want to
//! change after a playtest lives in this one file (`docs/03-tech/floor01-scene.md`
//! explains each choice). Bevy-free; all values are ticks at 60 Hz, pixels, or
//! pixels per second. Nothing here is approved balance: they are the starting
//! values of CP-01 and are tunable.
//!
//! Changing a value changes the committed fixture hashes
//! (`docs/evidence/cp01-feel`); regenerate them with
//! `scripts/make_cp01_feel_evidence.sh`.

/// Per-move reaction of a goblin that Carl hits (starting values from the Carl
/// melee spec, section 4, adjusted where the doc says so).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveFeel {
    /// Ticks both fighters freeze on a confirmed hit (the attacker's move clock
    /// and the target's timers and motion).
    pub hit_stop: u8,
    /// Ticks the target is staggered (no attack, no walking, no tell).
    pub stagger: u8,
    /// Knockback impulse in px/s: horizontal away from the attacker and a small
    /// vertical hop. Applied once; gravity and ground friction do the rest.
    pub knock_vx: f32,
    pub knock_vy: f32,
    /// Camera kick on the confirming hit, in virtual pixels (presentation only).
    pub camera_kick_px: u8,
}

/// Jab (`punch_a`): quick, light. A 40 px/s impulse slides the goblin about
/// 1 px (40^2 / (2 x 1200), a little more with the per-tick steps): the melee art
/// asks for at most 2 px, so the haymaker still reaches a goblin the jab pushed
/// (a hit box reaches 34 px centre to centre).
pub const JAB_FEEL: MoveFeel = MoveFeel {
    hit_stop: 3,
    stagger: 10,
    knock_vx: 40.0,
    knock_vy: 0.0,
    camera_kick_px: 1,
};

/// Haymaker (`punch_b`): the heavier second blow. 80 px/s with a 60 px/s hop
/// carries the goblin about 12 px (7 ticks in the air, then a short slide).
pub const HAYMAKER_FEEL: MoveFeel = MoveFeel {
    hit_stop: 5,
    stagger: 18,
    knock_vx: 80.0,
    knock_vy: 60.0,
    camera_kick_px: 2,
};

/// Push kick: only its hit-stop (8 ticks, melee spec) is new; the reaction is
/// the jab's until the CP-02 kick launch.
pub const KICK_FEEL: MoveFeel = MoveFeel {
    hit_stop: 8,
    stagger: JAB_FEEL.stagger,
    knock_vx: JAB_FEEL.knock_vx,
    knock_vy: JAB_FEEL.knock_vy,
    camera_kick_px: 2,
};

/// A confirming (killing) hit freezes this many ticks longer than the move's
/// own hit-stop, so the last blow lands harder.
pub const LETHAL_HIT_STOP_BONUS: u8 = 3;
/// The killing hit's launch (replaces the move's knockback): about 21 ticks in
/// the air, 45 px, then about 7 px of sliding.
pub const LETHAL_KNOCK_VX: f32 = 130.0;
pub const LETHAL_KNOCK_VY: f32 = 180.0;
/// Extra camera kick (px) on the killing hit.
pub const LETHAL_CAMERA_KICK_BONUS: u8 = 1;
/// Largest horizontal speed any knockback may have (bounded impulse).
pub const KNOCK_VX_MAX: f32 = 300.0;
/// Gravity of a launched goblin (px/s^2, the WORK-021 value) and the ground
/// friction that slows a slide (px/s^2).
pub const KNOCK_GRAVITY: f32 = 1050.0;
pub const GROUND_FRICTION: f32 = 1200.0;

/// Presentation only: ticks the target flashes white after a hit and ticks the
/// camera kick takes to fade out (it decays linearly).
pub const IMPACT_FLASH_TICKS: u8 = 3;
pub const CAMERA_KICK_TICKS: u8 = 6;

// ---------------------------------------------------------- goblin attack --

/// Walk speed of an approaching goblin (px/s, the WORK-021 chase speed).
pub const GOBLIN_WALK_SPEED: f32 = 62.0;
/// A goblin notices Carl closer than this (px, centre to centre).
pub const GOBLIN_SIGHT: f32 = 220.0;
/// A goblin with an attack token starts its tell at this distance or closer.
pub const GOBLIN_TRIGGER: f32 = 30.0;
/// A goblin without a token stops this far away (the spec's 34..60 px stand-off).
pub const GOBLIN_STANDOFF: f32 = 60.0;
/// Attack timeline, tick 0 = the tick the tell starts (GOB-01 spec section 4):
/// tell 0..24 (6 lead-in + 18 wind-up), smear 24..26, impact (hit active)
/// 26..30, recover 30..50.
pub const TELL_TICKS: u16 = 24;
pub const SMEAR_END: u16 = 26;
pub const IMPACT_END: u16 = 30;
pub const ATTACK_TICKS: u16 = 50;
/// Idle or walking ticks before the next tell, and the pause after a stagger
/// before any new tell (walking is allowed in both).
pub const ATTACK_COOLDOWN: u8 = 40;
pub const HURT_COOLDOWN: u8 = 20;
/// Swing hit box, pivot-relative and half-open: 8..30 px ahead of the goblin's
/// feet centre, 0..30 px above its feet.
pub const SWING_X_MIN: f32 = 8.0;
pub const SWING_X_MAX: f32 = 30.0;
pub const SWING_Y_MIN: f32 = 0.0;
pub const SWING_Y_MAX: f32 = 30.0;
/// At most this many goblins may be in a tell or a swing at once, and two tell
/// starts are at least `TELL_SPACING` ticks apart (the attack token rule).
pub const MAX_ATTACKERS: usize = 2;
pub const TELL_SPACING: u64 = 8;
/// A defeated goblin lies down this many ticks, then stays as a corpse that
/// neither collides nor damages.
pub const DOWN_TICKS: u16 = 30;

// ------------------------------------------------------------------- Carl --

/// Ticks Carl cannot act after a hit (the melee art's `hurt` flinch).
pub const HERO_HURT_TICKS: u8 = 18;
/// Ticks Carl and the goblin that hit him freeze on a hit that does not and
/// does defeat Carl.
pub const HERO_HURT_HIT_STOP: u8 = 4;
pub const HERO_DEFEAT_HIT_STOP: u8 = 7;
/// Carl's knockdown on defeat: launched away from the goblin (px/s).
pub const HERO_KNOCKDOWN_VX: f32 = 120.0;
pub const HERO_KNOCKDOWN_VY: f32 = 160.0;
/// Ticks from the defeating hit until the fixture restarts by itself (knockdown
/// plus a short sprawl; R / North restarts at once). At most 45.
pub const RESTART_DELAY_TICKS: u16 = 40;
