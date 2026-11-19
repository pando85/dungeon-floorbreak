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

/// Jab (`punch_a`): quick, light. A 75 px/s impulse slides the goblin about
/// 3 px (75^2 / (2 x 1200), a little more with the per-tick steps), a push the
/// player can see. The haymaker still reaches a goblin the jab pushed because
/// it steps forward ([`HAYMAKER_STEP_PX`]). Decided by the Opus feel review
/// (round 1), pending owner playtest. No camera kick: heavy blows only.
pub const JAB_FEEL: MoveFeel = MoveFeel {
    hit_stop: 3,
    stagger: 10,
    knock_vx: 75.0,
    knock_vy: 0.0,
    camera_kick_px: 0,
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

/// The haymaker steps Carl forward this many pixels in total, one pixel per
/// tick on haymaker ticks 1..=`HAYMAKER_STEP_TICKS`: it keeps the 34 px reach
/// on a goblin the jab pushed back about 3 px. The step stops at body contact
/// (centre distance 24 px with an alive goblin) and at walls; it is gameplay
/// motion (the art's own lunge is drawn on top of it).
pub const HAYMAKER_STEP_PX: f32 = 4.0;
pub const HAYMAKER_STEP_TICKS: u16 = 4;

/// Push kick (D-02): 8 ticks of hit-stop (melee spec) and a **launch**: a
/// goblin the kick does not defeat flies away (`KICK_LAUNCH_VX` / `_VY`), lands
/// and lies down (`Downed`), then gets up. The stagger value is unused by a
/// launch (the flight replaces it); a goblin in an armoured swing takes
/// neither stagger nor launch.
pub const KICK_FEEL: MoveFeel = MoveFeel {
    hit_stop: 8,
    stagger: JAB_FEEL.stagger,
    knock_vx: KICK_LAUNCH_VX,
    knock_vy: KICK_LAUNCH_VY,
    camera_kick_px: 2,
};

/// A living goblin's launch from the kick, px/s: about 17 ticks in the air
/// with an 11 px apex and 48 px of flight, then a short slide. Below the lethal
/// launch (160 / 210), so a kill still flies farther than a kick.
pub const KICK_LAUNCH_VX: f32 = 170.0;
pub const KICK_LAUNCH_VY: f32 = 150.0;
/// A launched living goblin lies down this many ticks (`down` frames), then
/// gets up for `GETUP_TICKS` (`getup` frames, 8 + 8 + 6) and is neutral again.
pub const DOWNED_TICKS: u16 = 18;
pub const GETUP_TICKS: u16 = 22;

// ------------------------------------------------- CP-02 goblin contact --

/// CP-02 (first pass: **no collision damage**). When a kicked living goblin
/// touches a second one, the flier shows `knockback_contact` for this many
/// ticks and loses most of its speed, the second goblin is interrupted
/// (stagger, attack cancelled, token back) and pushed a little, and both
/// freeze for the contact hit-stop. One reaction per (launch, target).
pub const CONTACT_POSE_TICKS: u8 = 6;
pub const CONTACT_HIT_STOP: u8 = 4;
pub const CONTACT_STAGGER: u8 = 24;
pub const CONTACT_PUSH_VX: f32 = 60.0;
/// Share of its horizontal speed the flier keeps after the contact.
pub const CONTACT_SOURCE_KEEP: f32 = 0.25;

/// A confirming (killing) hit freezes this many ticks longer than the move's
/// own hit-stop, so the last blow lands harder.
pub const LETHAL_HIT_STOP_BONUS: u8 = 3;
/// The killing hit's launch (replaces the move's knockback): 24 ticks in the
/// air with a 21 px apex, 64 px, then about 11 px of sliding: about 75 px in
/// all. The hit-stop comes before the flight, so a trace shows the whole
/// `Airborne` mode longer than the flight (freeze 9 ticks + 24 flight).
pub const LETHAL_KNOCK_VX: f32 = 160.0;
pub const LETHAL_KNOCK_VY: f32 = 210.0;
/// Extra camera kick (px) on the killing hit.
pub const LETHAL_CAMERA_KICK_BONUS: u8 = 1;
/// Largest horizontal speed any knockback may have (bounded impulse).
pub const KNOCK_VX_MAX: f32 = 300.0;
/// Gravity of a launched goblin (px/s^2, the WORK-021 value) and the ground
/// friction that slows a slide (px/s^2).
pub const KNOCK_GRAVITY: f32 = 1050.0;
pub const GROUND_FRICTION: f32 = 1200.0;

/// Presentation only: ticks the target flashes white after a hit and ticks the
/// camera kick takes to fade out (a linear decay rounded to whole pixels: the
/// view only moves by whole virtual pixels, so a 3 px kick passes 3, 3, 2, 2,
/// 2, 1, 1, 0). The kick shifts the view so the scene jolts the way the goblin
/// flies, never against the knockback.
pub const IMPACT_FLASH_TICKS: u8 = 3;
pub const CAMERA_KICK_TICKS: u8 = 8;

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
pub const TELL_SPACING: u64 = 12;
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
/// plus a short sprawl; R / Select restarts at once). At most 60.
pub const RESTART_DELAY_TICKS: u16 = 50;

// -------------------------------------------------------------- Bad Llama --

/// DEMO-01 PR B: the Floor 1 Bad Llama (LLAMA-01 art, `docs/02-art/llama-01-spec.md`).
/// Every number is a **playtest proposal** (not approved balance), kept here
/// like the goblin values. The attack timeline counts ticks from the tick the
/// tell starts: tell 0..36 (the glow; the art's 30 tick tell holds its peak
/// frame for the last 6), the spit releases on tick 36 (the first tick of the
/// art's `attack_swing` frame), recover 36..60.
pub const LLAMA_HEALTH: u8 = 3;
/// Walk speed toward Carl (px/s) while he is farther than `LLAMA_STAND_OFF`.
pub const LLAMA_WALK_SPEED: f32 = 40.0;
/// The llama walks toward Carl until it is this close (px, centre to centre)
/// and never walks away from him.
pub const LLAMA_STAND_OFF: f32 = 200.0;
/// It never strays farther than this from its home column (px).
pub const LLAMA_LEASH: f32 = 160.0;
pub const LLAMA_TELL_TICKS: u16 = 36;
pub const LLAMA_ATTACK_TICKS: u16 = 60;
/// Ticks after an attack (or a stagger) before the next tell.
pub const LLAMA_COOLDOWN: u16 = 90;
/// Stagger of a jab or haymaker hit; a kick staggers it the same and pushes it
/// back `LLAMA_KICK_PUSH_VX` (about 30 px with ground friction).
pub const LLAMA_HURT_TICKS: u8 = 18;
pub const LLAMA_KICK_PUSH_VX: f32 = 265.0;
/// A defeated llama lies this many ticks, then it is a corpse that neither
/// collides nor damages.
pub const LLAMA_DOWN_TICKS: u16 = 30;
/// Fair fire (owner option D): a tell starts only while the llama's whole body
/// is inside the simulation's scroll-camera view and Carl is at most this far
/// away (px, centre to centre) in the direction it faces.
pub const LLAMA_FIRE_RANGE: f32 = 320.0;
/// The spit glob: speed (px/s), furthest travel (px), mouth offset ahead of
/// the llama's centre and height above its feet (LLAMA-01 spawn contract:
/// pivot offset (+46, -45)), and the 8 x 6 px hit box.
pub const LLAMA_GLOB_SPEED: f32 = 150.0;
pub const LLAMA_GLOB_RANGE: f32 = 320.0;
pub const LLAMA_GLOB_MOUTH_DX: f32 = 46.0;
pub const LLAMA_GLOB_MOUTH_DY: f32 = 45.0;
pub const LLAMA_GLOB_HALF_W: f32 = 4.0;
pub const LLAMA_GLOB_HALF_H: f32 = 3.0;
/// Pause after a hit staggers it (the tell it was in is cancelled): the llama
/// does not start another tell for this many ticks.
pub const LLAMA_HURT_COOLDOWN: u16 = 30;
