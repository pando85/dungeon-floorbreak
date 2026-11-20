//! CF02-H move buffer, same-tick priority, landing grace and the closed cancel
//! table (Q46..Q49).
//!
//! Pure, Bevy-free and deterministic: every function takes the knob table
//! ([`Cf02Tuning`]) and plain values, and returns a decision. **Nothing is wired
//! here**: CF02-H calls these from the Carl press handling. Every rule is
//! effective only under `cf02_rules` (a boolean switch is read through
//! [`Cf02Tuning::on`]; a numeric window falls back to the legacy value when the
//! master switch is off).
//!
//! With `cf02_rules` off only [`window_ticks`] and [`runner_buffer_ticks`] (the
//! legacy values) are meaningful. [`MoveBuffer`] and [`cancel_for`] must not be
//! used then: C1 is `None` with the master switch off because the legacy
//! `MoveRunner` keeps its own jab-to-haymaker chain in that case.
//!
//! Conventions fixed by this module (see the CF02-CORES PR for the reading of the
//! design text):
//!
//! * Tick 0 is the tick a press is accepted. A buffered press is live while its
//!   age is `<=` the window, so a window of `w` ticks offers a press on `w + 1`
//!   consecutive ticks. This is the legacy convention (`expires = clock + 6`
//!   with `CARL_BUFFER_TICKS` 6), carried over to the cf02 value 8.
//! * Per fixed tick the caller (1) classifies and stores this tick's press,
//!   (2) offers the buffer to a runner that is idle, (3) calls
//!   [`MoveBuffer::advance`] once, passing whether the tick was a hit-stop
//!   tick. The age does not advance in hit-stop.
//! * A cancel is decided **on the press tick only** ([`classify_press`]). A press
//!   made before a cancel window opens is buffered and can only start a fresh
//!   move later, it never chains (Q46, legacy C1).

use crate::cf02::Cf02Tuning;
use crate::combat::CARL_BUFFER_TICKS;

/// The three buttons the move buffer knows. Only [`Button::Primary`] and
/// [`Button::Physical`] are ever stored; a Jump press is read only to decide a
/// C3 or C6 cancel and the same-tick rule (Q47). Ordinary jump and swap keep
/// their own buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Primary,
    Physical,
    Jump,
}

/// The press buffer window in ticks: the cf02 value under `cf02_rules`, the
/// legacy [`CARL_BUFFER_TICKS`] otherwise (Q46).
pub fn window_ticks(t: &Cf02Tuning) -> u8 {
    if t.cf02_rules {
        t.q46_move_buffer_ticks
    } else {
        CARL_BUFFER_TICKS
    }
}

/// The `MoveRunner`'s own buffer: 0 under `cf02_rules` (the move buffer in front
/// of it offers a press only when it can start), the legacy value otherwise.
pub fn runner_buffer_ticks(t: &Cf02Tuning) -> u8 {
    if t.cf02_rules {
        t.q46_runner_buffer_ticks
    } else {
        CARL_BUFFER_TICKS
    }
}

/// A press remembered with the direction it was made with (Q46, Q01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferedPress {
    pub button: Button,
    /// Sign of `move_x` on the press tick: -1, 0 or 1.
    pub move_x_sign: i8,
    /// The Down sample of the press tick.
    pub down: bool,
    age: u8,
}

/// How a remembered press ended up starting something.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartKind {
    /// A fresh move from an idle runner.
    Fresh,
    /// A string (C1, C2).
    Chain,
    /// Any other cancel (C3..C6).
    Cancel,
}

impl BufferedPress {
    /// A press accepted on this tick (age 0).
    pub fn new(button: Button, move_x_sign: i8, down: bool) -> Self {
        Self {
            button,
            move_x_sign: move_x_sign.signum(),
            down,
            age: 0,
        }
    }

    /// Ticks (not counting hit-stop) since the press.
    pub fn age(&self) -> u8 {
        self.age
    }

    /// Q46: a fresh move started from a held direction turns Carl toward it;
    /// chains and cancels never turn. `None` keeps the facing.
    pub fn turn_on_start(&self, kind: StartKind) -> Option<i8> {
        (kind == StartKind::Fresh && self.move_x_sign != 0).then_some(self.move_x_sign)
    }
}

/// One slot, newest press wins (the legacy `MoveRunner` keeps one, too).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MoveBuffer {
    slot: Option<BufferedPress>,
}

impl MoveBuffer {
    /// Remember a press, replacing an older one. A Jump press is not stored
    /// (returns `false`).
    pub fn store(&mut self, press: BufferedPress) -> bool {
        if press.button == Button::Jump {
            return false;
        }
        self.slot = Some(press);
        true
    }

    /// The remembered press, if any.
    pub fn peek(&self) -> Option<&BufferedPress> {
        self.slot.as_ref()
    }

    /// Offer the remembered press to the runner. It leaves the buffer only when
    /// the runner is idle (it can start a fresh move) and the actor can act; a
    /// hurt or defeated actor forgets it (`can_act == false`, as the legacy
    /// runner does).
    pub fn take_fresh(&mut self, runner_idle: bool, can_act: bool) -> Option<BufferedPress> {
        if !can_act {
            self.slot = None;
            return None;
        }
        if runner_idle { self.slot.take() } else { None }
    }

    /// Forget the press (a move started, the actor was hit).
    pub fn clear(&mut self) {
        self.slot = None;
    }

    /// End of a tick: age the press and drop it once it is older than the
    /// window. The age is frozen on a hit-stop tick (Q46), so a press made during
    /// hit-stop keeps its full window.
    pub fn advance(&mut self, t: &Cf02Tuning, in_hit_stop: bool) {
        if let Some(press) = self.slot.as_mut() {
            if !in_hit_stop {
                press.age = press.age.saturating_add(1);
            }
            if press.age > window_ticks(t) {
                self.slot = None;
            }
        }
    }
}

/// What one tick's buttons resolve to (Q05, Q35, Q47).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameResolution {
    /// The jump starts this tick.
    pub jump: bool,
    /// The attack press that is handled this tick (Primary wins over Physical).
    pub attack: Option<Button>,
    /// The attack is stored for the first airborne tick instead of starting now
    /// (Q47). Only set together with `jump`.
    pub defer_attack: bool,
}

/// Resolve the buttons of one tick. Primary and Physical on one tick: Primary
/// wins (Q05, Q10-A08). On the ground with Jump and an attack on the same tick
/// and `q47_jump_wins_same_tick` on, Jump wins and the attack stays buffered for
/// its air version (Q47); off (legacy) the ground move wins and the jump is
/// stripped. In the air a same-tick Jump is not a jump.
///
/// `can_jump` is true only when a jump can start this tick: the runner is idle,
/// Carl is grounded and not hurt. When it is false (a committed jab in the C1
/// window, a grounded recovery that makes the Jump a C3 bail-out) Q47 does not
/// apply: this returns `jump: false, defer_attack: false`, the attack goes to
/// [`classify_press`] as it would alone (C1 chain, buffer) and the Jump is
/// classified on its own (C3/C6 or ignore). Nothing is deferred, so no attack
/// can leak into a bail-out jump.
pub fn resolve_frame(
    t: &Cf02Tuning,
    primary: bool,
    physical: bool,
    jump: bool,
    grounded: bool,
    can_jump: bool,
) -> FrameResolution {
    let attack = if primary {
        Some(Button::Primary)
    } else if physical {
        Some(Button::Physical)
    } else {
        None
    };
    let jump_now = jump && grounded && can_jump;
    if jump_now && attack.is_some() {
        if t.on(t.q47_jump_wins_same_tick) {
            return FrameResolution {
                jump: true,
                attack,
                defer_attack: true,
            };
        }
        return FrameResolution {
            jump: false,
            attack,
            defer_attack: false,
        };
    }
    FrameResolution {
        jump: jump_now,
        attack,
        defer_attack: false,
    }
}

/// What an air attack press does (Q47, Q48, Q50, Q52).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirRoute {
    /// Q48: falling and within the grace height of a solid top: wait and fire
    /// as the ground move on the landing tick.
    WaitForLanding,
    /// Q52: Down + Physical more than the minimum height above the ground.
    Dive,
    /// Q50: Physical without Down, an air kick still unused in this period.
    AirKick,
    /// Stay buffered: a dive that is still too low, or the period's air kick or
    /// dive already used.
    Hold,
    /// Primary has no air move (CF02 adds none); the press keeps the legacy
    /// handling.
    NoAirMove,
}

/// The airborne state an attack press is routed with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AirState {
    /// Vertical speed in px/s, positive up.
    pub vy_px_s: i32,
    /// Feet height above the solid top below, in px.
    pub height_px: i32,
    pub air_kicks_used: u8,
    pub dives_used: u8,
    /// The airborne period is a Q54/Q55 bail-out: no attack until landing, so a
    /// press buffered before the jump (a same-tick Physical, a press made in the
    /// recovery) is held, never fired as an air move.
    pub no_attack_until_landing: bool,
}

/// Route an attack press made while airborne.
///
/// Q48: **any** attack press while falling (`vy <= 0`) with the feet within
/// `q48_landing_grace_px` of a solid top waits for the landing. Otherwise
/// Physical with Down is the dive when the feet are more than
/// `q52_dive_min_height_px` up, and Physical without Down is the air kick; each
/// is once per airborne period. A rising Physical within the grace height is
/// still an air kick (Q47: the buffered press fires on the first airborne tick).
/// During a bail-out jump ([`AirState::no_attack_until_landing`], Q54) an attack
/// is held until the landing grace (Q48) applies; no air move starts.
pub fn route_air_attack(t: &Cf02Tuning, button: Button, down: bool, air: &AirState) -> AirRoute {
    if button == Button::Jump {
        return AirRoute::NoAirMove;
    }
    if air.vy_px_s <= 0 && air.height_px <= i32::from(t.q48_landing_grace_px) {
        return AirRoute::WaitForLanding;
    }
    if air.no_attack_until_landing {
        return AirRoute::Hold;
    }
    if button == Button::Primary {
        return AirRoute::NoAirMove;
    }
    if down {
        let high_enough = air.height_px > i32::from(t.q52_dive_min_height_px);
        if high_enough && air.dives_used < t.q52_dives_per_period {
            AirRoute::Dive
        } else {
            AirRoute::Hold
        }
    } else if air.air_kicks_used < t.q50_air_kicks_per_period {
        AirRoute::AirKick
    } else {
        AirRoute::Hold
    }
}

/// The closed cancel list of Q49. Nothing outside these six cancels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelId {
    /// Jab -> haymaker (Primary on jab ticks 3..=13, legacy).
    C1,
    /// Jab hit -> follow-through (Q05).
    C2,
    /// Any grounded Carl move's recovery -> bail-out (Jump, Q54).
    C3,
    /// Air kick that hit -> dive (Down + Physical in its recovery).
    C4,
    /// Dive bounce -> air kick (Q53), if no air kick was used this period.
    C5,
    /// Donut's swat or pounce landing recovery -> cat hop (Jump, Q55).
    C6,
}

/// The moves and states a cancel can come out of. The caller maps its own move
/// ids; `DiveBounce` is the airborne state after a dive's first hit (the dive
/// has ended).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Jab,
    Haymaker,
    PushKick,
    FollowKick,
    Stomp,
    AirKick,
    DiveStomp,
    DiveBounce,
    Swat,
    Pounce,
}

impl MoveKind {
    /// A grounded Carl move (the sources of C3).
    pub fn is_grounded_carl(self) -> bool {
        matches!(
            self,
            Self::Jab | Self::Haymaker | Self::PushKick | Self::FollowKick | Self::Stomp
        )
    }
}

/// Where a running move is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Startup,
    Active,
    /// After the last active tick. For a pounce, its landing recovery.
    Recovery,
}

/// The move that is running on the press tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunningMove {
    pub kind: MoveKind,
    pub phase: Phase,
    /// The move tick (frozen during hit-stop; 0 is the first tick).
    pub tick: u8,
    /// The instance already logged a `Hit`.
    pub hit: bool,
    pub in_hit_stop: bool,
    /// Air kicks used in this airborne period (C5).
    pub air_kicks_used: u8,
}

/// The actor on the press tick: a running move or idle (`None`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorCtx {
    pub running: Option<RunningMove>,
    /// Hurt or defeated: nothing is accepted.
    pub hurt: bool,
}

/// One press and the direction it was made with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressSample {
    pub button: Button,
    pub down: bool,
    pub move_x_sign: i8,
    /// Facing sign: -1 or 1 (decides "neutral or forward", Q05).
    pub facing: i8,
}

impl PressSample {
    /// Held toward the side behind the actor.
    fn backward(&self) -> bool {
        self.move_x_sign != 0 && self.move_x_sign.signum() != self.facing.signum()
    }
}

/// What a press does on its own tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// An open cancel takes it.
    Cancel(CancelId),
    /// The runner is idle: start a fresh move now.
    Fresh,
    /// Remember it (the move buffer); it can start a fresh move later, never a
    /// cancel (Q46).
    Buffer,
    /// Not for the move buffer: a Jump that cancels nothing, or an actor that
    /// cannot act.
    Ignore,
}

/// Q49: which cancel, if any, a press makes on its own tick.
///
/// C1 and C2 judge the press by the (hit-stop-frozen) move tick, so a press
/// during a jab's hit-stop is inside the window (legacy); the cancelled move
/// starts later (C1 at the haymaker's earliest tick, C2 at
/// [`c2_start_tick`]). C3..C6 are refused in hit-stop and need the recovery (C5:
/// the bounce state). A cancel is `None` when its switch is off or the master
/// switch is off.
pub fn cancel_for(t: &Cf02Tuning, run: &RunningMove, press: &PressSample) -> Option<CancelId> {
    let in_window = |from: u8, to: u8| (from..=to).contains(&run.tick);
    let settled = !run.in_hit_stop;
    match press.button {
        Button::Primary => (run.kind == MoveKind::Jab
            && t.on(t.q49_c1_enabled)
            && in_window(t.q49_c1_window_start, t.q49_c1_window_end))
        .then_some(CancelId::C1),
        Button::Physical => {
            if run.kind == MoveKind::Jab
                && run.hit
                && !press.down
                && !press.backward()
                && t.on(t.q49_c2_enabled)
                && in_window(t.q05_follow_window_start, t.q05_follow_window_end)
            {
                Some(CancelId::C2)
            } else if run.kind == MoveKind::AirKick
                && run.hit
                && press.down
                && settled
                && run.phase == Phase::Recovery
                && t.on(t.q49_c4_enabled)
            {
                Some(CancelId::C4)
            } else if run.kind == MoveKind::DiveBounce
                && !press.down
                && settled
                && run.air_kicks_used < t.q50_air_kicks_per_period
                && t.on(t.q49_c5_enabled)
            {
                Some(CancelId::C5)
            } else {
                None
            }
        }
        Button::Jump => {
            if run.kind.is_grounded_carl()
                && settled
                && run.phase == Phase::Recovery
                && t.on(t.q49_c3_enabled && t.q54_bailout_enabled)
            {
                Some(CancelId::C3)
            } else if matches!(run.kind, MoveKind::Swat | MoveKind::Pounce)
                && settled
                && run.phase == Phase::Recovery
                && t.on(t.q49_c6_enabled && t.q55_bailout_enabled)
            {
                Some(CancelId::C6)
            } else {
                None
            }
        }
    }
}

/// Q05: the follow-through starts on this jab tick or the press tick, whichever
/// is later.
pub fn c2_start_tick(t: &Cf02Tuning, press_jab_tick: u8) -> u8 {
    press_jab_tick.max(t.q05_follow_start_jab_tick)
}

/// Q46: classify one press on the tick it is accepted.
pub fn classify_press(t: &Cf02Tuning, actor: &ActorCtx, press: &PressSample) -> Outcome {
    if actor.hurt {
        return Outcome::Ignore;
    }
    let attack = press.button != Button::Jump;
    match actor.running {
        None if attack => Outcome::Fresh,
        None => Outcome::Ignore,
        Some(run) => match cancel_for(t, &run, press) {
            Some(id) => Outcome::Cancel(id),
            None if attack => Outcome::Buffer,
            None => Outcome::Ignore,
        },
    }
}

/// One row of the Q49 table, as data (documentation and a closed-list test).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CancelRow {
    pub id: CancelId,
    pub from: &'static [MoveKind],
    pub button: Button,
}

/// The closed cancel list. [`cancel_for`] returns `Some(id)` only for a row's
/// `from` moves and `button`.
pub const CANCEL_TABLE: [CancelRow; 6] = [
    CancelRow {
        id: CancelId::C1,
        from: &[MoveKind::Jab],
        button: Button::Primary,
    },
    CancelRow {
        id: CancelId::C2,
        from: &[MoveKind::Jab],
        button: Button::Physical,
    },
    CancelRow {
        id: CancelId::C3,
        from: &[
            MoveKind::Jab,
            MoveKind::Haymaker,
            MoveKind::PushKick,
            MoveKind::FollowKick,
            MoveKind::Stomp,
        ],
        button: Button::Jump,
    },
    CancelRow {
        id: CancelId::C4,
        from: &[MoveKind::AirKick],
        button: Button::Physical,
    },
    CancelRow {
        id: CancelId::C5,
        from: &[MoveKind::DiveBounce],
        button: Button::Physical,
    },
    CancelRow {
        id: CancelId::C6,
        from: &[MoveKind::Swat, MoveKind::Pounce],
        button: Button::Jump,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn on() -> Cf02Tuning {
        let mut t = Cf02Tuning::pinned().expect("pinned tuning");
        t.cf02_rules = true;
        t
    }

    fn off() -> Cf02Tuning {
        let mut t = on();
        t.cf02_rules = false;
        t
    }

    fn press(button: Button) -> PressSample {
        PressSample {
            button,
            down: false,
            move_x_sign: 0,
            facing: 1,
        }
    }

    fn run(kind: MoveKind, phase: Phase, tick: u8) -> RunningMove {
        RunningMove {
            kind,
            phase,
            tick,
            hit: false,
            in_hit_stop: false,
            air_kicks_used: 0,
        }
    }

    fn actor(r: RunningMove) -> ActorCtx {
        ActorCtx {
            running: Some(r),
            hurt: false,
        }
    }

    /// Simulate: a press at tick `p` while the runner is busy until tick `free`;
    /// returns the tick it starts, if any.
    fn start_tick(t: &Cf02Tuning, p: u32, free: u32, hit_stop: &[u32]) -> Option<u32> {
        let mut buf = MoveBuffer::default();
        for tick in 0..40u32 {
            if tick == p {
                buf.store(BufferedPress::new(Button::Primary, 0, false));
            }
            if let Some(_p) = buf.take_fresh(tick >= free, true) {
                return Some(tick);
            }
            buf.advance(t, hit_stop.contains(&tick));
        }
        None
    }

    #[test]
    fn q46_window_is_8_under_cf02_and_the_runner_buffer_is_0() {
        assert_eq!(window_ticks(&on()), 8);
        assert_eq!(runner_buffer_ticks(&on()), 0);
        // Legacy values with the master switch off.
        assert_eq!(window_ticks(&off()), CARL_BUFFER_TICKS);
        assert_eq!(runner_buffer_ticks(&off()), CARL_BUFFER_TICKS);
    }

    #[test]
    fn q46_a_press_is_live_for_exactly_the_window() {
        let t = on();
        // Press on tick 10, runner free on tick 18: age 8 = the last live tick.
        assert_eq!(start_tick(&t, 10, 18, &[]), Some(18));
        // Free on tick 19: the press is 9 ticks old and forgotten.
        assert_eq!(start_tick(&t, 10, 19, &[]), None);
        // The legacy window of 6 ends two ticks sooner.
        let l = off();
        assert_eq!(start_tick(&l, 10, 16, &[]), Some(16));
        assert_eq!(start_tick(&l, 10, 17, &[]), None);
    }

    #[test]
    fn q46_a_press_does_not_expire_during_hit_stop() {
        let t = on();
        // Five hit-stop ticks (11..=15) freeze the age: free on tick 23 would
        // be 13 ticks old without the freeze, and is 8 with it.
        let hs = [11, 12, 13, 14, 15];
        assert_eq!(start_tick(&t, 10, 23, &hs), Some(23));
        assert_eq!(start_tick(&t, 10, 24, &hs), None);
        // Without hit-stop the same press is long gone.
        assert_eq!(start_tick(&t, 10, 23, &[]), None);
    }

    #[test]
    fn q46_a_press_during_hit_stop_keeps_its_full_window() {
        let t = on();
        let mut buf = MoveBuffer::default();
        buf.store(BufferedPress::new(Button::Physical, 1, true));
        for _ in 0..20 {
            buf.advance(&t, true);
        }
        assert_eq!(buf.peek().map(BufferedPress::age), Some(0));
    }

    #[test]
    fn q46_the_newest_press_replaces_the_older_one_and_keeps_its_direction() {
        let mut buf = MoveBuffer::default();
        buf.store(BufferedPress::new(Button::Primary, -1, false));
        buf.store(BufferedPress::new(Button::Physical, 1, true));
        let got = buf.take_fresh(true, true).expect("a press");
        assert_eq!(
            (got.button, got.move_x_sign, got.down),
            (Button::Physical, 1, true)
        );
        assert!(buf.peek().is_none());
    }

    #[test]
    fn q46_a_hurt_actor_forgets_the_press_and_a_busy_runner_keeps_it() {
        let mut buf = MoveBuffer::default();
        buf.store(BufferedPress::new(Button::Primary, 0, false));
        assert!(buf.take_fresh(false, true).is_none());
        assert!(buf.peek().is_some(), "busy runner keeps it");
        assert!(buf.take_fresh(true, false).is_none());
        assert!(buf.peek().is_none(), "hurt actor forgets it");
    }

    #[test]
    fn q46_a_jump_press_is_never_stored() {
        let mut buf = MoveBuffer::default();
        assert!(!buf.store(BufferedPress::new(Button::Jump, 0, false)));
        assert!(buf.peek().is_none());
    }

    #[test]
    fn q46_a_buffered_fresh_move_turns_and_chains_and_cancels_never_do() {
        let right = BufferedPress::new(Button::Primary, 1, false);
        let left = BufferedPress::new(Button::Primary, -1, false);
        let none = BufferedPress::new(Button::Primary, 0, false);
        assert_eq!(right.turn_on_start(StartKind::Fresh), Some(1));
        assert_eq!(left.turn_on_start(StartKind::Fresh), Some(-1));
        assert_eq!(none.turn_on_start(StartKind::Fresh), None);
        for kind in [StartKind::Chain, StartKind::Cancel] {
            assert_eq!(right.turn_on_start(kind), None);
            assert_eq!(left.turn_on_start(kind), None);
        }
        // A sign is normalised.
        assert_eq!(BufferedPress::new(Button::Primary, 5, false).move_x_sign, 1);
    }

    #[test]
    fn q46_a_press_on_jab_ticks_1_and_2_does_not_chain() {
        let t = on();
        for tick in [0, 1, 2] {
            let a = actor(run(MoveKind::Jab, Phase::Startup, tick));
            assert_eq!(
                classify_press(&t, &a, &press(Button::Primary)),
                Outcome::Buffer,
                "jab tick {tick}"
            );
        }
        // The buffered press never becomes a C1 on a later tick: a decision is
        // made on the press tick only, and the buffer offers a fresh start
        // only.
        let mut buf = MoveBuffer::default();
        buf.store(BufferedPress::new(Button::Primary, 0, false));
        buf.advance(&t, false);
        buf.advance(&t, false); // now jab tick 3 inside the C1 window
        assert!(
            buf.take_fresh(false, true).is_none(),
            "runner busy: no chain"
        );
        // It starts a fresh jab once the runner is idle, if still live.
        assert!(buf.take_fresh(true, true).is_some());
    }

    #[test]
    fn q49_c1_window_is_jab_ticks_3_to_13_in_hit_stop_too() {
        let t = on();
        let c1 = |tick: u8, hs: bool| {
            let mut r = run(MoveKind::Jab, Phase::Active, tick);
            r.in_hit_stop = hs;
            cancel_for(&t, &r, &press(Button::Primary))
        };
        assert_eq!(c1(2, false), None);
        assert_eq!(c1(3, false), Some(CancelId::C1));
        assert_eq!(c1(13, false), Some(CancelId::C1));
        assert_eq!(c1(14, false), None);
        // A press during the freeze is judged by the frozen tick (legacy).
        assert_eq!(c1(3, true), Some(CancelId::C1));
        // Off: no cancel at all.
        assert_eq!(
            cancel_for(
                &off(),
                &run(MoveKind::Jab, Phase::Active, 5),
                &press(Button::Primary)
            ),
            None
        );
        let mut s = on();
        s.q49_c1_enabled = false;
        assert_eq!(
            cancel_for(
                &s,
                &run(MoveKind::Jab, Phase::Active, 5),
                &press(Button::Primary)
            ),
            None
        );
    }

    #[test]
    fn q49_c2_needs_a_jab_hit_neutral_or_forward_and_no_down() {
        let t = on();
        let mut hit = run(MoveKind::Jab, Phase::Recovery, 8);
        hit.hit = true;
        let miss = run(MoveKind::Jab, Phase::Recovery, 8);
        let with = |move_x_sign: i8, down: bool| PressSample {
            button: Button::Physical,
            down,
            move_x_sign,
            facing: 1,
        };
        assert_eq!(cancel_for(&t, &hit, &with(0, false)), Some(CancelId::C2));
        assert_eq!(cancel_for(&t, &hit, &with(1, false)), Some(CancelId::C2));
        assert_eq!(cancel_for(&t, &hit, &with(-1, false)), None, "backward");
        assert_eq!(
            cancel_for(&t, &hit, &with(0, true)),
            None,
            "Down is the stomp"
        );
        assert_eq!(cancel_for(&t, &miss, &with(0, false)), None, "whiffed jab");
        // Facing left: forward is -1.
        let left = PressSample {
            facing: -1,
            ..with(-1, false)
        };
        assert_eq!(cancel_for(&t, &hit, &left), Some(CancelId::C2));
        // Window 3..=13.
        let mut early = hit;
        early.tick = 2;
        assert_eq!(cancel_for(&t, &early, &with(0, false)), None);
        early.tick = 13;
        assert_eq!(cancel_for(&t, &early, &with(0, false)), Some(CancelId::C2));
        early.tick = 14;
        assert_eq!(cancel_for(&t, &early, &with(0, false)), None);
    }

    #[test]
    fn q05_c2_starts_on_jab_tick_9_or_the_press_tick_whichever_is_later() {
        let t = on();
        assert_eq!(c2_start_tick(&t, 3), 9);
        assert_eq!(c2_start_tick(&t, 9), 9);
        assert_eq!(c2_start_tick(&t, 12), 12);
    }

    #[test]
    fn q49_c3_any_grounded_move_recovery_not_in_hit_stop() {
        let t = on();
        for kind in [
            MoveKind::Jab,
            MoveKind::Haymaker,
            MoveKind::PushKick,
            MoveKind::FollowKick,
            MoveKind::Stomp,
        ] {
            let r = run(kind, Phase::Recovery, 10);
            assert_eq!(cancel_for(&t, &r, &press(Button::Jump)), Some(CancelId::C3));
            for phase in [Phase::Startup, Phase::Active] {
                let r = run(kind, phase, 2);
                assert_eq!(cancel_for(&t, &r, &press(Button::Jump)), None, "{kind:?}");
            }
            let mut frozen = run(kind, Phase::Recovery, 10);
            frozen.in_hit_stop = true;
            assert_eq!(cancel_for(&t, &frozen, &press(Button::Jump)), None);
        }
        // The bail-out switch also gates it.
        let mut s = on();
        s.q54_bailout_enabled = false;
        assert_eq!(
            cancel_for(
                &s,
                &run(MoveKind::Stomp, Phase::Recovery, 5),
                &press(Button::Jump)
            ),
            None
        );
    }

    #[test]
    fn q49_c4_needs_an_air_kick_that_hit_in_recovery_with_down_and_physical() {
        let t = on();
        let dp = PressSample {
            down: true,
            ..press(Button::Physical)
        };
        let mut r = run(MoveKind::AirKick, Phase::Recovery, 12);
        r.hit = true;
        assert_eq!(cancel_for(&t, &r, &dp), Some(CancelId::C4));
        assert_eq!(
            cancel_for(&t, &r, &press(Button::Physical)),
            None,
            "no Down"
        );
        let mut miss = r;
        miss.hit = false;
        assert_eq!(cancel_for(&t, &miss, &dp), None, "air kick that missed");
        for phase in [Phase::Startup, Phase::Active] {
            let mut early = r;
            early.phase = phase;
            assert_eq!(cancel_for(&t, &early, &dp), None);
        }
        let mut frozen = r;
        frozen.in_hit_stop = true;
        assert_eq!(cancel_for(&t, &frozen, &dp), None);
    }

    #[test]
    fn q49_c5_dive_bounce_gives_the_one_unused_air_kick() {
        let t = on();
        let mut r = run(MoveKind::DiveBounce, Phase::Recovery, 0);
        assert_eq!(
            cancel_for(&t, &r, &press(Button::Physical)),
            Some(CancelId::C5)
        );
        let dp = PressSample {
            down: true,
            ..press(Button::Physical)
        };
        assert_eq!(cancel_for(&t, &r, &dp), None, "Down + Physical is a dive");
        r.air_kicks_used = 1;
        assert_eq!(
            cancel_for(&t, &r, &press(Button::Physical)),
            None,
            "kick used"
        );
    }

    #[test]
    fn q49_c6_donut_swat_or_pounce_recovery_takes_jump() {
        let t = on();
        for kind in [MoveKind::Swat, MoveKind::Pounce] {
            assert_eq!(
                cancel_for(&t, &run(kind, Phase::Recovery, 6), &press(Button::Jump)),
                Some(CancelId::C6)
            );
            assert_eq!(
                cancel_for(&t, &run(kind, Phase::Active, 2), &press(Button::Jump)),
                None
            );
            // Physical and Primary are not cancels for her.
            assert_eq!(
                cancel_for(&t, &run(kind, Phase::Recovery, 6), &press(Button::Physical)),
                None
            );
        }
        let mut s = on();
        s.q49_c6_enabled = false;
        assert_eq!(
            cancel_for(
                &s,
                &run(MoveKind::Swat, Phase::Recovery, 6),
                &press(Button::Jump)
            ),
            None
        );
    }

    #[test]
    fn q49_the_table_is_closed_nothing_outside_it_cancels() {
        let t = on();
        let kinds = [
            MoveKind::Jab,
            MoveKind::Haymaker,
            MoveKind::PushKick,
            MoveKind::FollowKick,
            MoveKind::Stomp,
            MoveKind::AirKick,
            MoveKind::DiveStomp,
            MoveKind::DiveBounce,
            MoveKind::Swat,
            MoveKind::Pounce,
        ];
        let mut seen = [false; 6];
        for kind in kinds {
            for phase in [Phase::Startup, Phase::Active, Phase::Recovery] {
                for tick in 0..40u8 {
                    for (hit, hs, used) in [
                        (false, false, 0),
                        (true, false, 0),
                        (true, true, 0),
                        (false, false, 1),
                    ] {
                        let r = RunningMove {
                            kind,
                            phase,
                            tick,
                            hit,
                            in_hit_stop: hs,
                            air_kicks_used: used,
                        };
                        for button in [Button::Primary, Button::Physical, Button::Jump] {
                            for down in [false, true] {
                                for dir in [-1i8, 0, 1] {
                                    let p = PressSample {
                                        button,
                                        down,
                                        move_x_sign: dir,
                                        facing: 1,
                                    };
                                    if let Some(id) = cancel_for(&t, &r, &p) {
                                        let row = CANCEL_TABLE
                                            .iter()
                                            .find(|row| row.id == id)
                                            .expect("row exists");
                                        assert!(row.from.contains(&kind), "{id:?} from {kind:?}");
                                        assert_eq!(row.button, button, "{id:?}");
                                        seen[CANCEL_TABLE
                                            .iter()
                                            .position(|row| row.id == id)
                                            .unwrap()] = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(seen, [true; 6], "every row is reachable");
        // The dive itself, the haymaker's own string and the stomp have no
        // cancel of their own for Primary or Physical.
        for kind in [MoveKind::DiveStomp, MoveKind::Haymaker, MoveKind::Stomp] {
            for button in [Button::Primary, Button::Physical] {
                let r = run(kind, Phase::Recovery, 10);
                assert_eq!(cancel_for(&t, &r, &press(button)), None, "{kind:?}");
            }
        }
        // No cancel at all with the master switch off.
        for row in CANCEL_TABLE {
            for kind in row.from {
                let r = RunningMove {
                    kind: *kind,
                    phase: Phase::Recovery,
                    tick: 5,
                    hit: true,
                    in_hit_stop: false,
                    air_kicks_used: 0,
                };
                assert_eq!(cancel_for(&off(), &r, &press(row.button)), None);
            }
        }
    }

    #[test]
    fn q49_cancels_are_exclusive_per_press() {
        // For any single press at most one cancel opens (an Option), and the
        // table pairs a button with disjoint sources except C1/C2 (different
        // buttons) and C3/C6 (different heroes).
        for a in CANCEL_TABLE {
            for b in CANCEL_TABLE {
                if a.id != b.id && a.button == b.button {
                    assert!(
                        a.from.iter().all(|k| !b.from.contains(k)),
                        "{:?} and {:?} overlap",
                        a.id,
                        b.id
                    );
                }
            }
        }
    }

    #[test]
    fn q46_classification_fresh_buffer_cancel_ignore() {
        let t = on();
        let idle = ActorCtx {
            running: None,
            hurt: false,
        };
        assert_eq!(
            classify_press(&t, &idle, &press(Button::Primary)),
            Outcome::Fresh
        );
        assert_eq!(
            classify_press(&t, &idle, &press(Button::Physical)),
            Outcome::Fresh
        );
        assert_eq!(
            classify_press(&t, &idle, &press(Button::Jump)),
            Outcome::Ignore
        );
        let busy = actor(run(MoveKind::Haymaker, Phase::Active, 3));
        assert_eq!(
            classify_press(&t, &busy, &press(Button::Primary)),
            Outcome::Buffer
        );
        assert_eq!(
            classify_press(&t, &busy, &press(Button::Jump)),
            Outcome::Ignore
        );
        let jab = actor(run(MoveKind::Jab, Phase::Active, 4));
        assert_eq!(
            classify_press(&t, &jab, &press(Button::Primary)),
            Outcome::Cancel(CancelId::C1)
        );
        let hurt = ActorCtx {
            running: None,
            hurt: true,
        };
        for b in [Button::Primary, Button::Physical, Button::Jump] {
            assert_eq!(classify_press(&t, &hurt, &press(b)), Outcome::Ignore);
        }
        // Down + Physical inside a jab's window is buffered, never C2 (Q35).
        let mut jab_hit = run(MoveKind::Jab, Phase::Active, 4);
        jab_hit.hit = true;
        let dp = PressSample {
            down: true,
            ..press(Button::Physical)
        };
        assert_eq!(classify_press(&t, &actor(jab_hit), &dp), Outcome::Buffer);
    }

    #[test]
    fn q05_q47_priority_primary_beats_physical_and_jump_beats_an_attack() {
        let t = on();
        let both = resolve_frame(&t, true, true, false, true, true);
        assert_eq!(both.attack, Some(Button::Primary));
        let phys = resolve_frame(&t, false, true, false, true, true);
        assert_eq!(phys.attack, Some(Button::Physical));
        assert_eq!(
            resolve_frame(&t, false, false, false, true, true).attack,
            None
        );
        // Q47: on the ground Jump wins; the attack is kept for the air.
        let same = resolve_frame(&t, false, true, true, true, true);
        assert_eq!(
            same,
            FrameResolution {
                jump: true,
                attack: Some(Button::Physical),
                defer_attack: true
            }
        );
        let same_primary = resolve_frame(&t, true, false, true, true, true);
        assert!(same_primary.jump && same_primary.defer_attack);
        // Jump alone, attack alone.
        let j = resolve_frame(&t, false, false, true, true, true);
        assert!(j.jump && !j.defer_attack && j.attack.is_none());
        let a = resolve_frame(&t, true, false, false, true, true);
        assert!(!a.jump && !a.defer_attack);
        // In the air a Jump press is not a jump and defers nothing.
        let air = resolve_frame(&t, false, true, true, false, true);
        assert!(!air.jump && !air.defer_attack && air.attack == Some(Button::Physical));
    }

    #[test]
    fn q47_jump_plus_primary_in_the_jab_window_keeps_the_c1_chain_when_jump_is_impossible() {
        let t = on();
        // Jab tick 5: committed, so the jump cannot start.
        let r = resolve_frame(&t, true, false, true, true, false);
        assert_eq!(
            r,
            FrameResolution {
                jump: false,
                attack: Some(Button::Primary),
                defer_attack: false
            }
        );
        let jab = actor(run(MoveKind::Jab, Phase::Active, 5));
        assert_eq!(
            classify_press(&t, &jab, &press(Button::Primary)),
            Outcome::Cancel(CancelId::C1),
            "the Primary reaches classify_press and chains the haymaker"
        );
        // The Jump alone is not a cancel inside the jab's active window.
        assert_eq!(
            classify_press(&t, &jab, &press(Button::Jump)),
            Outcome::Ignore
        );
        // Free to jump: Q47 still wins and defers the attack.
        assert!(resolve_frame(&t, true, false, true, true, true).defer_attack);
    }

    #[test]
    fn q54_jump_plus_physical_in_recovery_is_a_bailout_with_no_air_kick_until_landing() {
        let t = on();
        let r = resolve_frame(&t, false, true, true, true, false);
        assert!(!r.jump && !r.defer_attack, "nothing is deferred");
        let rec = actor(run(MoveKind::Jab, Phase::Recovery, 20));
        assert_eq!(
            classify_press(&t, &rec, &press(Button::Jump)),
            Outcome::Cancel(CancelId::C3)
        );
        // The Physical of that tick is only buffered, never a cancel.
        assert_eq!(
            classify_press(&t, &rec, &press(Button::Physical)),
            Outcome::Buffer
        );
        // On the bail-out's first airborne ticks the buffered Physical is held.
        let bail = |vy: i32, h: i32| AirState {
            vy_px_s: vy,
            height_px: h,
            air_kicks_used: 0,
            dives_used: 0,
            no_attack_until_landing: true,
        };
        for down in [false, true] {
            assert_eq!(
                route_air_attack(&t, Button::Physical, down, &bail(260, 2)),
                AirRoute::Hold
            );
            assert_eq!(
                route_air_attack(&t, Button::Physical, down, &bail(-50, 30)),
                AirRoute::Hold
            );
        }
        assert_eq!(
            route_air_attack(&t, Button::Primary, false, &bail(-50, 30)),
            AirRoute::Hold
        );
        // Landing grace (Q48) still releases it on the landing tick.
        assert_eq!(
            route_air_attack(&t, Button::Physical, false, &bail(-50, 4)),
            AirRoute::WaitForLanding
        );
        // A normal jump (flag off) still gets the air kick.
        let normal = AirState {
            no_attack_until_landing: false,
            ..bail(260, 2)
        };
        assert_eq!(
            route_air_attack(&t, Button::Physical, false, &normal),
            AirRoute::AirKick
        );
    }

    #[test]
    fn q47_legacy_the_ground_move_wins_and_the_jump_is_stripped() {
        for t in [off(), {
            let mut s = on();
            s.q47_jump_wins_same_tick = false;
            s
        }] {
            let r = resolve_frame(&t, false, true, true, true, true);
            assert_eq!(
                r,
                FrameResolution {
                    jump: false,
                    attack: Some(Button::Physical),
                    defer_attack: false
                }
            );
        }
    }

    #[test]
    fn q48_landing_grace_waits_only_when_falling_within_8_px() {
        let t = on();
        let air = |vy: i32, h: i32| AirState {
            vy_px_s: vy,
            height_px: h,
            air_kicks_used: 0,
            dives_used: 0,
            no_attack_until_landing: false,
        };
        for b in [Button::Primary, Button::Physical] {
            assert_eq!(
                route_air_attack(&t, b, false, &air(-100, 8)),
                AirRoute::WaitForLanding
            );
            assert_eq!(
                route_air_attack(&t, b, false, &air(0, 0)),
                AirRoute::WaitForLanding
            );
        }
        // Down + Physical within 8 px waits for the ground stomp.
        assert_eq!(
            route_air_attack(&t, Button::Physical, true, &air(-100, 5)),
            AirRoute::WaitForLanding
        );
        // 9 px or rising: no wait.
        assert_eq!(
            route_air_attack(&t, Button::Physical, false, &air(-100, 9)),
            AirRoute::AirKick
        );
        assert_eq!(
            route_air_attack(&t, Button::Primary, false, &air(-100, 9)),
            AirRoute::NoAirMove
        );
        // Q47: a rising Physical right after take-off is an air kick.
        assert_eq!(
            route_air_attack(&t, Button::Physical, false, &air(260, 2)),
            AirRoute::AirKick
        );
    }

    #[test]
    fn q52_q50_the_dive_needs_more_than_8_px_and_each_move_is_once_per_period() {
        let t = on();
        let air = |h: i32, kicks: u8, dives: u8| AirState {
            vy_px_s: -50,
            height_px: h,
            air_kicks_used: kicks,
            dives_used: dives,
            no_attack_until_landing: false,
        };
        assert_eq!(
            route_air_attack(&t, Button::Physical, true, &air(9, 0, 0)),
            AirRoute::Dive
        );
        // Rising within 8 px: the dive waits for height.
        let rising = AirState {
            vy_px_s: 200,
            height_px: 8,
            air_kicks_used: 0,
            dives_used: 0,
            no_attack_until_landing: false,
        };
        assert_eq!(
            route_air_attack(&t, Button::Physical, true, &rising),
            AirRoute::Hold
        );
        assert_eq!(
            route_air_attack(&t, Button::Physical, true, &air(30, 0, 1)),
            AirRoute::Hold,
            "one dive per period"
        );
        assert_eq!(
            route_air_attack(&t, Button::Physical, false, &air(30, 1, 0)),
            AirRoute::Hold,
            "one air kick per period"
        );
        assert_eq!(
            route_air_attack(&t, Button::Jump, false, &air(30, 0, 0)),
            AirRoute::NoAirMove
        );
    }
}
