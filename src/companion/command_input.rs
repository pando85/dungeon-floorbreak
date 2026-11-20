//! COMPANION-AI-01 stage 01C: the command-input grammar spike.
//!
//! An isolated, pure, deterministic per-tick state machine that turns the pad
//! buttons of the controlled hero's player into exactly one semantic [`Action`]
//! per tick, plus the face/Down/interact state that normal gameplay may still
//! see ([`Forward`]). It decides how L1 (or R1, or a toggle) separates a swap
//! from a companion command. It is **not wired**: production bindings
//! (`input.rs`, `pad_slots.rs`, `party.rs`) are unchanged, L1 still swaps on the
//! press edge, and no default L1 hold/release binding is adopted without
//! real-pad evidence (D-CTRL, `docs/03-tech/companion-ai-command-system.md`).
//!
//! Variants ([`GrammarVariant`]):
//!
//! - `Baseline`: L1 swaps on the press edge, as on main today. No commands.
//! - `L1ReleaseHold` (CA-12): L1 is a modifier. A short **release** swaps; a
//!   hold of `hold_ticks` (10) opens command mode; the first L1+face chord
//!   acts instantly, before the threshold.
//! - `R1Modifier` (CA-14): L1 keeps its instant press swap; R1 is the modifier.
//!   Interact moves to the R1 release (delayed) as an [`Action::DelayedInteract`].
//! - `Toggle` (CA-14): L1 keeps its instant press swap; a spare "toggle" button
//!   (remappable, unassigned in production) opens and closes command mode with
//!   no hold. The next face press is one command and closes the mode.
//!
//! Chords (CA-13): Square Focus, Triangle Hold, Circle Regroup, Cross Assist.
//! Down never forms a chord and always passes through. Two face edges on one
//! tick resolve in that fixed order and the other press is consumed.
//!
//! CA-27: a face press consumed by the grammar is masked from [`Forward`] until
//! the button is released, so it never reaches gameplay or the move buffer and
//! never fabricates a press edge. Entering command mode sets
//! [`TickOutput::clear_buffers`]; the (future) wiring must clear the controlled
//! hero's move buffer and the swap buffer. Nothing here does that wiring.
//!
//! Context inputs: `commands_blocked` (no companion, controlled hero dead or
//! respawning) makes chords and holds [`Action::Refused`]; `hurt` is carried
//! only for the test gate model and the grammar never reads it. Link events
//! clear all modal state; after a reconnect (or leaving two-human play) every
//! button that is down is latched until it is released. After
//! [`LinkEvent::SecondHumanJoined`] the grammar is disabled and passes
//! everything through (no swap, no commands), per the two-human party mode.

/// Default hold threshold in ticks (CA-12 hypothesis, about 167 ms at 60 Hz).
pub const DEFAULT_HOLD_TICKS: u8 = 10;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GrammarVariant {
    /// (a) L1 swaps on the press edge, as production does today.
    #[default]
    Baseline,
    /// (b) CA-12: L1 short release swaps, hold opens command mode.
    L1ReleaseHold,
    /// (c) CA-14: R1 is the command modifier, L1 stays an instant swap.
    R1Modifier,
    /// (d) CA-14: a toggle button opens and closes command mode.
    Toggle,
}

impl GrammarVariant {
    pub const ALL: [GrammarVariant; 4] = [
        GrammarVariant::Baseline,
        GrammarVariant::L1ReleaseHold,
        GrammarVariant::R1Modifier,
        GrammarVariant::Toggle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            GrammarVariant::Baseline => "a Baseline (L1 press swap)",
            GrammarVariant::L1ReleaseHold => "b CA-12 (L1 release swap, hold opens)",
            GrammarVariant::R1Modifier => "c CA-14 (R1 modifier)",
            GrammarVariant::Toggle => "d CA-14 (toggle)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrammarConfig {
    pub variant: GrammarVariant,
    /// Ticks the modifier must stay down (press tick included) to open command mode.
    pub hold_ticks: u8,
}

impl GrammarConfig {
    pub const fn new(variant: GrammarVariant) -> Self {
        Self {
            variant,
            hold_ticks: DEFAULT_HOLD_TICKS,
        }
    }
}

impl Default for GrammarConfig {
    fn default() -> Self {
        Self::new(GrammarVariant::Baseline)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionCommand {
    Focus,
    Hold,
    Regroup,
    Assist,
}

/// The four face buttons in chord priority order (CA-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Square,
    Triangle,
    Circle,
    Cross,
}

impl Face {
    pub const ALL: [Face; 4] = [Face::Square, Face::Triangle, Face::Circle, Face::Cross];

    fn index(self) -> usize {
        self as usize
    }

    pub fn command(self) -> CompanionCommand {
        match self {
            Face::Square => CompanionCommand::Focus,
            Face::Triangle => CompanionCommand::Hold,
            Face::Circle => CompanionCommand::Regroup,
            Face::Cross => CompanionCommand::Assist,
        }
    }
}

/// A button the grammar reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    L1,
    R1,
    Face(Face),
    Down,
    /// Remappable spare button for the toggle variant (unassigned in production).
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkEvent {
    PadDisconnected,
    PadReconnected,
    /// A second human joined: the party leaves solo mode and the grammar disables itself.
    SecondHumanJoined,
    /// Back to one human: the grammar re-enables.
    SecondHumanLeft,
}

/// One tick of per-button states for the controlled hero's player.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TickInput {
    pub l1: bool,
    pub r1: bool,
    pub square: bool,
    pub triangle: bool,
    pub circle: bool,
    pub cross: bool,
    pub down: bool,
    pub toggle: bool,
    pub link: Option<LinkEvent>,
    /// No companion to command, or the controlled hero is dead or respawning.
    pub commands_blocked: bool,
    /// Controlled hero in hit reaction. Context for the swap gate model only.
    pub hurt: bool,
}

impl TickInput {
    pub const NONE: TickInput = TickInput {
        l1: false,
        r1: false,
        square: false,
        triangle: false,
        circle: false,
        cross: false,
        down: false,
        toggle: false,
        link: None,
        commands_blocked: false,
        hurt: false,
    };

    pub fn with(mut self, button: Btn) -> Self {
        match button {
            Btn::L1 => self.l1 = true,
            Btn::R1 => self.r1 = true,
            Btn::Face(Face::Square) => self.square = true,
            Btn::Face(Face::Triangle) => self.triangle = true,
            Btn::Face(Face::Circle) => self.circle = true,
            Btn::Face(Face::Cross) => self.cross = true,
            Btn::Down => self.down = true,
            Btn::Toggle => self.toggle = true,
        }
        self
    }

    fn held(&self) -> Held {
        Held {
            l1: self.l1,
            r1: self.r1,
            toggle: self.toggle,
            down: self.down,
            faces: [self.square, self.triangle, self.circle, self.cross],
        }
    }
}

/// The semantic action of one tick. Exactly one per input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Nothing for the grammar; gameplay sees [`TickOutput::forward`] as normal.
    PassThrough,
    /// One swap request (the party still applies hurt, cooldown and buffer rules).
    Swap,
    /// One companion command.
    Command(CompanionCommand),
    OpenMode,
    /// Command mode closed with no command (unused hold released, toggle, link event).
    CancelMode,
    Refused(RefuseReason),
    /// Variant (c) only: a short R1 tap released, so interact fires now.
    DelayedInteract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefuseReason {
    /// `commands_blocked` was set when a chord or hold completed.
    CommandsUnavailable,
}

impl Action {
    /// One character per action, for compact traces.
    pub fn code(self) -> char {
        match self {
            Action::PassThrough => '.',
            Action::Swap => 'S',
            Action::Command(CompanionCommand::Focus) => 'F',
            Action::Command(CompanionCommand::Hold) => 'H',
            Action::Command(CompanionCommand::Regroup) => 'R',
            Action::Command(CompanionCommand::Assist) => 'A',
            Action::OpenMode => 'O',
            Action::CancelMode => 'C',
            Action::Refused(_) => 'X',
            Action::DelayedInteract => 'I',
        }
    }
}

/// What normal gameplay may still see this tick (held states, so a masked
/// button never produces a press edge downstream).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Forward {
    /// Cross.
    pub jump: bool,
    /// Square.
    pub attack: bool,
    /// Triangle.
    pub physical: bool,
    /// Circle.
    pub ability: bool,
    /// R1 held, passed through immediately (not in variant c).
    pub interact: bool,
    pub down: bool,
}

impl Forward {
    pub const NONE: Forward = Forward {
        jump: false,
        attack: false,
        physical: false,
        ability: false,
        interact: false,
        down: false,
    };

    /// True when `self` holds a face button (jump, primary, physical, special)
    /// that `prev` did not: a press edge gameplay would see.
    pub fn new_face_edge_over(&self, prev: &Forward) -> bool {
        (self.jump && !prev.jump)
            || (self.attack && !prev.attack)
            || (self.physical && !prev.physical)
            || (self.ability && !prev.ability)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickOutput {
    pub action: Action,
    pub forward: Forward,
    /// CA-27: the controlled hero's move buffer and the swap buffer must be cleared.
    pub clear_buffers: bool,
    /// Command mode is open and visible after this tick.
    pub mode_open: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Held {
    l1: bool,
    r1: bool,
    toggle: bool,
    down: bool,
    faces: [bool; 4],
}

impl Held {
    const NONE: Held = Held {
        l1: false,
        r1: false,
        toggle: false,
        down: false,
        faces: [false; 4],
    };

    fn zip(self, o: Held, f: impl Fn(bool, bool) -> bool) -> Held {
        Held {
            l1: f(self.l1, o.l1),
            r1: f(self.r1, o.r1),
            toggle: f(self.toggle, o.toggle),
            down: f(self.down, o.down),
            faces: [
                f(self.faces[0], o.faces[0]),
                f(self.faces[1], o.faces[1]),
                f(self.faces[2], o.faces[2]),
                f(self.faces[3], o.faces[3]),
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    /// Modifier down, nothing decided yet; `held` counts ticks down.
    Candidate {
        held: u8,
    },
    /// Command mode. `used`: a command (or refusal) happened, so releasing is
    /// silent. `visible`: the legend is shown.
    Open {
        used: bool,
        visible: bool,
    },
}

#[derive(Debug, Clone)]
pub struct CommandGrammar {
    cfg: GrammarConfig,
    enabled: bool,
    connected: bool,
    /// Buttons that were down across a reconnect or re-enable; ignored until released.
    latch: Held,
    prev: Held,
    phase: Phase,
    /// Face presses owned by the grammar until released.
    consumed: [bool; 4],
    /// Variant (c): a released R1 tap whose interact has not fired yet.
    pending_interact: bool,
}

impl CommandGrammar {
    pub fn new(cfg: GrammarConfig) -> Self {
        Self {
            cfg,
            enabled: true,
            connected: true,
            latch: Held::NONE,
            prev: Held::NONE,
            phase: Phase::Idle,
            consumed: [false; 4],
            pending_interact: false,
        }
    }

    pub fn config(&self) -> GrammarConfig {
        self.cfg
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn mode_visible(&self) -> bool {
        matches!(self.phase, Phase::Open { visible: true, .. })
    }

    fn reset_modal(&mut self) {
        // `consumed` is kept: a masked button stays masked until it is released.
        self.phase = Phase::Idle;
        self.pending_interact = false;
    }

    fn out(action: Action, forward: Forward, clear: bool, open: bool) -> TickOutput {
        TickOutput {
            action,
            forward,
            clear_buffers: clear,
            mode_open: open,
        }
    }

    /// Advance one 60 Hz tick.
    pub fn step(&mut self, input: TickInput) -> TickOutput {
        let raw = input.held();
        let mut event_cancel = false;
        let mut event_clear = false;
        match input.link {
            Some(LinkEvent::PadDisconnected) => {
                let visible = self.mode_visible();
                self.connected = false;
                self.reset_modal();
                self.prev = Held::NONE;
                let action = if visible {
                    Action::CancelMode
                } else {
                    Action::PassThrough
                };
                return Self::out(action, Forward::NONE, true, false);
            }
            Some(LinkEvent::PadReconnected) => {
                self.connected = true;
                self.latch = raw;
                self.reset_modal();
                self.prev = Held::NONE;
                return Self::out(Action::PassThrough, Forward::NONE, false, false);
            }
            Some(LinkEvent::SecondHumanJoined) => {
                event_cancel = self.mode_visible();
                event_clear = true;
                self.enabled = false;
                self.reset_modal();
            }
            Some(LinkEvent::SecondHumanLeft) => {
                self.enabled = true;
                self.reset_modal();
                self.latch = raw;
            }
            None => {}
        }
        if !self.connected {
            self.prev = Held::NONE;
            return Self::out(Action::PassThrough, Forward::NONE, false, false);
        }
        self.latch = self.latch.zip(raw, |l, r| l && r);
        let eff = raw.zip(self.latch, |r, l| r && !l);
        for (c, held) in self.consumed.iter_mut().zip(eff.faces) {
            *c = *c && held;
        }
        if !self.enabled {
            if event_cancel {
                // The press that arrives with the join never reaches gameplay.
                let pressed = self.pressed_faces(eff);
                self.consume(pressed);
            }
            self.prev = eff;
            let action = if event_cancel {
                Action::CancelMode
            } else {
                Action::PassThrough
            };
            return Self::out(action, self.forward(eff), event_clear, false);
        }
        let (action, clear) = match self.cfg.variant {
            GrammarVariant::Baseline => {
                let swap = eff.l1 && !self.prev.l1;
                (
                    if swap {
                        Action::Swap
                    } else {
                        Action::PassThrough
                    },
                    false,
                )
            }
            GrammarVariant::L1ReleaseHold | GrammarVariant::R1Modifier => {
                self.step_modifier(eff, input.commands_blocked)
            }
            GrammarVariant::Toggle => self.step_toggle(eff, input.commands_blocked),
        };
        self.prev = eff;
        Self::out(action, self.forward(eff), clear, self.mode_visible())
    }

    fn forward(&self, eff: Held) -> Forward {
        let f = |face: Face| eff.faces[face.index()] && !self.consumed[face.index()];
        Forward {
            jump: f(Face::Cross),
            attack: f(Face::Square),
            physical: f(Face::Triangle),
            ability: f(Face::Circle),
            interact: eff.r1 && self.cfg.variant != GrammarVariant::R1Modifier,
            down: eff.down,
        }
    }

    fn pressed_faces(&self, eff: Held) -> [bool; 4] {
        std::array::from_fn(|i| eff.faces[i] && !self.prev.faces[i])
    }

    fn first_edge(pressed: [bool; 4]) -> Option<Face> {
        Face::ALL.into_iter().find(|f| pressed[f.index()])
    }

    fn chord(face: Face, blocked: bool) -> Action {
        if blocked {
            Action::Refused(RefuseReason::CommandsUnavailable)
        } else {
            Action::Command(face.command())
        }
    }

    fn consume(&mut self, pressed: [bool; 4]) {
        for (c, p) in self.consumed.iter_mut().zip(pressed) {
            *c = *c || p;
        }
    }

    /// Variants (b) and (c): a modifier button with tap, hold and chord.
    fn step_modifier(&mut self, eff: Held, blocked: bool) -> (Action, bool) {
        let r1_variant = self.cfg.variant == GrammarVariant::R1Modifier;
        let (m_now, m_prev) = if r1_variant {
            (eff.r1, self.prev.r1)
        } else {
            (eff.l1, self.prev.l1)
        };
        let m_pressed = m_now && !m_prev;
        let pressed = self.pressed_faces(eff);
        let face_edge = Self::first_edge(pressed);
        let live = self.phase != Phase::Idle || m_pressed;
        if self.phase == Phase::Idle && m_pressed {
            self.phase = Phase::Candidate { held: 0 };
        }
        let mut action = Action::PassThrough;
        let mut clear = false;
        match self.phase {
            Phase::Idle => {}
            Phase::Candidate { held } => {
                if let Some(face) = face_edge {
                    action = Self::chord(face, blocked);
                    clear = true;
                    self.pending_interact = false;
                    self.phase = if m_now {
                        Phase::Open {
                            used: true,
                            visible: !blocked,
                        }
                    } else {
                        Phase::Idle
                    };
                } else if !m_now {
                    self.phase = Phase::Idle;
                    if r1_variant {
                        self.pending_interact = true;
                    } else {
                        action = Action::Swap;
                    }
                } else {
                    let held = held.saturating_add(1);
                    if held >= self.cfg.hold_ticks.max(1) {
                        clear = true;
                        self.pending_interact = false;
                        self.phase = Phase::Open {
                            used: blocked,
                            visible: !blocked,
                        };
                        action = if blocked {
                            Action::Refused(RefuseReason::CommandsUnavailable)
                        } else {
                            Action::OpenMode
                        };
                    } else {
                        self.phase = Phase::Candidate { held };
                    }
                }
            }
            Phase::Open { used, visible } => {
                let mut used = used;
                if let Some(face) = face_edge {
                    action = Self::chord(face, blocked);
                    used = true;
                }
                if m_now {
                    self.phase = Phase::Open { used, visible };
                } else {
                    if face_edge.is_none() && !used {
                        action = Action::CancelMode;
                    }
                    self.phase = Phase::Idle;
                }
            }
        }
        if live {
            self.consume(pressed);
        }
        if r1_variant {
            let open = matches!(self.phase, Phase::Open { .. });
            if eff.l1 && !self.prev.l1 && action == Action::PassThrough && !open {
                action = Action::Swap;
            }
            if action == Action::PassThrough && self.pending_interact && !pressed.iter().any(|p| *p)
            {
                self.pending_interact = false;
                action = Action::DelayedInteract;
            }
        }
        (action, clear)
    }

    /// Variant (d): L1 swaps on press; a toggle button opens and closes the mode.
    fn step_toggle(&mut self, eff: Held, blocked: bool) -> (Action, bool) {
        let toggle_pressed = eff.toggle && !self.prev.toggle;
        let l1_pressed = eff.l1 && !self.prev.l1;
        let pressed = self.pressed_faces(eff);
        let face_edge = Self::first_edge(pressed);
        let mut action = Action::PassThrough;
        let mut clear = false;
        if matches!(self.phase, Phase::Open { .. }) {
            if let Some(face) = face_edge {
                action = Self::chord(face, blocked);
                self.phase = Phase::Idle;
                self.consume(pressed);
            } else if toggle_pressed || l1_pressed {
                action = Action::CancelMode;
                self.phase = Phase::Idle;
            }
        } else if toggle_pressed {
            if let Some(face) = face_edge {
                action = Self::chord(face, blocked);
                clear = true;
                self.consume(pressed);
            } else if blocked {
                action = Action::Refused(RefuseReason::CommandsUnavailable);
            } else {
                action = Action::OpenMode;
                clear = true;
                self.phase = Phase::Open {
                    used: false,
                    visible: true,
                };
            }
        } else if l1_pressed {
            action = Action::Swap;
        }
        (action, clear)
    }
}

/// Run a whole script through a fresh grammar.
pub fn run(cfg: GrammarConfig, script: &[TickInput]) -> Vec<TickOutput> {
    let mut grammar = CommandGrammar::new(cfg);
    script.iter().map(|tick| grammar.step(*tick)).collect()
}

/// The action stream as one character per tick (see [`Action::code`]).
pub fn render(outs: &[TickOutput]) -> String {
    outs.iter().map(|o| o.action.code()).collect()
}

/// Index of the first tick whose action satisfies `pred`.
pub fn first_tick(
    cfg: GrammarConfig,
    script: &[TickInput],
    pred: impl Fn(&Action) -> bool,
) -> Option<usize> {
    run(cfg, script).iter().position(|o| pred(&o.action))
}

/// Parse a compact script: whitespace separated ticks, each `spec` or
/// `spec*N`. A spec is `-` (nothing) or `+`-joined, case-insensitive names:
/// `L1 R1 Sq Tri Cir Cro Dn Tog`, flags `Blocked Hurt`, link events
/// `Disc Reconn Join Leave`.
pub fn parse_script(text: &str) -> Result<Vec<TickInput>, String> {
    let mut ticks = Vec::new();
    for token in text.split_whitespace() {
        let (spec, count) = match token.rsplit_once('*') {
            Some((spec, n)) => (
                spec,
                n.parse::<usize>()
                    .map_err(|_| format!("bad repeat in `{token}`"))?,
            ),
            None => (token, 1),
        };
        let mut tick = TickInput::NONE;
        for name in spec.split('+') {
            match name.to_ascii_lowercase().as_str() {
                "-" => {}
                "l1" => tick.l1 = true,
                "r1" => tick.r1 = true,
                "sq" | "square" => tick.square = true,
                "tri" | "triangle" => tick.triangle = true,
                "cir" | "circle" => tick.circle = true,
                "cro" | "cross" => tick.cross = true,
                "dn" | "down" => tick.down = true,
                "tog" | "toggle" => tick.toggle = true,
                "blocked" => tick.commands_blocked = true,
                "hurt" => tick.hurt = true,
                "disc" => tick.link = Some(LinkEvent::PadDisconnected),
                "reconn" => tick.link = Some(LinkEvent::PadReconnected),
                "join" => tick.link = Some(LinkEvent::SecondHumanJoined),
                "leave" => tick.link = Some(LinkEvent::SecondHumanLeft),
                other => return Err(format!("unknown name `{other}` in `{token}`")),
            }
        }
        // A link event happens once, not on every repeated tick.
        let link = tick.link.take();
        for i in 0..count {
            let mut t = tick;
            if i == 0 {
                t.link = link;
            }
            ticks.push(t);
        }
    }
    Ok(ticks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{InputFrame, RawInput};
    use crate::party::{PartyState, PartyTuning};

    const B: GrammarVariant = GrammarVariant::Baseline;
    const L: GrammarVariant = GrammarVariant::L1ReleaseHold;
    const R: GrammarVariant = GrammarVariant::R1Modifier;
    const T: GrammarVariant = GrammarVariant::Toggle;

    fn script(text: &str) -> Vec<TickInput> {
        parse_script(text).expect("valid script")
    }

    fn cfg(variant: GrammarVariant) -> GrammarConfig {
        GrammarConfig::new(variant)
    }

    /// Structural invariants checked on every run of variants b, c and d:
    /// one action per tick (by type), a non-pass action never carries a new
    /// face press edge (jump, primary, physical, special) (a swap in c and d is the documented exception:
    /// L1 there is not a modifier, so L1 plus a face press still behaves like
    /// production today), and Down passes through whenever the grammar is live.
    fn check(variant: GrammarVariant, outs: &[TickOutput]) {
        let mut prev = Forward::NONE;
        for (i, out) in outs.iter().enumerate() {
            let exempt_swap = out.action == Action::Swap && variant != L;
            let acted = out.action != Action::PassThrough && !exempt_swap;
            if acted {
                assert!(
                    !out.forward.new_face_edge_over(&prev),
                    "tick {i}: {:?} also forwarded a new press {:?}",
                    out.action,
                    out.forward
                );
            }
            prev = out.forward;
        }
    }

    fn actions(variant: GrammarVariant, text: &str) -> String {
        let outs = run(cfg(variant), &script(text));
        if variant != B {
            check(variant, &outs);
        }
        render(&outs)
    }

    #[test]
    fn parser_expands_repeats_and_fires_link_events_once() {
        let ticks = script("L1*2 Sq+Dn disc*3 -");
        assert_eq!(ticks.len(), 7);
        assert!(ticks[0].l1 && ticks[1].l1 && !ticks[2].l1);
        assert!(ticks[2].square && ticks[2].down);
        assert_eq!(ticks[3].link, Some(LinkEvent::PadDisconnected));
        assert_eq!(ticks[4].link, None);
        assert!(parse_script("L1+Nope").is_err());
        assert!(parse_script("L1*x").is_err());
    }

    /// Table: (name, variant, script, expected action stream).
    #[test]
    fn action_streams_table() {
        let table: &[(&str, GrammarVariant, &str, &str)] = &[
            // Ordinary short swap.
            ("baseline swaps on press", B, "L1*3 -*2", "S...."),
            ("b short tap swaps on release", L, "L1*3 -*2", "...S."),
            ("b tap of 9 ticks still swaps", L, "L1*9 -", ".........S"),
            ("c swap is instant", R, "L1*3 -*2", "S...."),
            ("d swap is instant", T, "L1*3 -*2", "S...."),
            // Hold with no selection.
            (
                "b hold opens then cancels",
                L,
                "L1*12 -*2",
                ".........O..C.",
            ),
            ("b hold of exactly 10 opens", L, "L1*10 -", ".........OC"),
            ("b hold released never swaps", L, "L1*12 -", ".........O..C"),
            (
                "c R1 hold opens then cancels",
                R,
                "R1*12 -",
                ".........O..C",
            ),
            ("d toggle open then cancel", T, "Tog - Tog", "O.C"),
            ("d toggle held opens once", T, "Tog*3 -", "O..."),
            // Fast L1 plus face chord (before the hold threshold).
            ("b chord on tick 2", L, "L1 L1+Sq -", ".F."),
            ("b chord leaves release silent", L, "L1 L1+Sq L1 -", ".F.."),
            ("b chord same tick as L1", L, "L1+Tri -", "H."),
            ("b chord on the release tick", L, "L1 L1 Sq", "..F"),
            (
                "b four chords in one hold",
                L,
                "L1 L1+Sq L1 L1+Cir L1 L1+Cro L1 L1+Tri -",
                ".F.R.A.H.",
            ),
            (
                "b chord after the mode opened",
                L,
                "L1*10 L1+Cir -",
                ".........OR.",
            ),
            ("c chord", R, "R1 R1+Sq -", ".F."),
            ("c chord Assist", R, "R1 R1+Cro -", ".A."),
            ("d one-shot chord", T, "Tog Cir -", "OR."),
            ("d toggle plus face same tick", T, "Tog+Tri -", "H."),
            // Simultaneous buttons.
            (
                "b two faces one tick: priority",
                L,
                "L1 L1+Cro+Tri -",
                ".H.",
            ),
            ("b L1 and R1 and a face", L, "L1+R1+Sq -", "F."),
            (
                "c L1 and R1 and a face: command wins, swap dropped",
                R,
                "R1+L1+Sq -",
                "F.",
            ),
            (
                "c swap while holding R1 candidate",
                R,
                "R1 R1+L1 R1 -",
                ".S.I",
            ),
            (
                "c swap swallowed in open mode",
                R,
                "R1*10 R1+L1 R1 -",
                ".........O..C",
            ),
            (
                "c R1 release with L1 press: swap then interact",
                R,
                "R1 L1 L1",
                ".SI",
            ),
            ("c short R1 tap is a delayed interact", R, "R1*3 -", "...I"),
            ("d L1 while open cancels without swap", T, "Tog L1 -", "OC."),
            ("d face without toggle passes", T, "Sq", "."),
            ("faces without a modifier pass", L, "Sq+Cir Cro -", "..."),
            // Down never chords.
            ("b Down is not a chord", L, "L1 L1+Dn -", "..S"),
            ("b Down beside a chord", L, "L1 L1+Dn+Sq -", ".F."),
            ("d Down in open mode", T, "Tog Dn -", "O.."),
            // Pre-held face then L1 tap: not a chord.
            (
                "b pre-held Square then L1 tap",
                L,
                "Sq Sq+L1 Sq+L1 Sq -",
                "...S.",
            ),
            // Context flags.
            ("b blocked chord refused", L, "L1 L1+Sq+Blocked -", ".X."),
            (
                "b blocked hold refused, release silent",
                L,
                "L1+Blocked*12 -",
                ".........X...",
            ),
            ("b blocked tap still swaps", L, "L1+Blocked*2 -", "..S"),
            ("d blocked toggle refused", T, "Tog+Blocked -", "X."),
            ("c blocked chord refused", R, "R1 R1+Tri+Blocked -", ".X."),
        ];
        for (name, variant, text, expected) in table {
            assert_eq!(&actions(*variant, text), expected, "{name}");
        }
    }

    #[test]
    fn consumed_face_press_never_reaches_gameplay() {
        // Square stays down after the chord and after L1 is released.
        let outs = run(cfg(L), &script("L1 L1+Sq Sq Sq - Sq"));
        let attack: Vec<bool> = outs.iter().map(|o| o.forward.attack).collect();
        assert_eq!(attack, [false, false, false, false, false, true]);
        assert_eq!(render(&outs), ".F....");
        // A different pre-held button keeps flowing and makes no new edge.
        let outs = run(cfg(L), &script("Tri Tri+L1 Tri+L1+Sq Tri+L1 Tri -"));
        check(L, &outs);
        assert!(outs.iter().take(5).all(|o| o.forward.physical));
        assert!(outs.iter().all(|o| !o.forward.attack));
        // Both faces of a double chord are masked; only the first is a command.
        let outs = run(cfg(L), &script("L1 L1+Cir+Cro L1+Cir+Cro"));
        assert_eq!(render(&outs), ".R.");
        assert!(outs.iter().all(|o| !o.forward.ability && !o.forward.jump));
    }

    #[test]
    fn down_passes_through_and_never_chords() {
        for variant in [L, R, T] {
            let outs = run(cfg(variant), &script("Dn L1+Dn R1+Dn Tog+Dn Dn -"));
            check(variant, &outs);
            let down: Vec<bool> = outs.iter().map(|o| o.forward.down).collect();
            assert_eq!(down, [true, true, true, true, true, false], "{variant:?}");
            assert!(
                outs.iter()
                    .all(|o| !matches!(o.action, Action::Command(_) | Action::Refused(_))),
                "{variant:?}: Down is not a chord"
            );
        }
    }

    #[test]
    fn r1_passes_as_interact_except_in_variant_c() {
        for variant in [B, L, T] {
            let outs = run(cfg(variant), &script("R1 R1 -"));
            let held: Vec<bool> = outs.iter().map(|o| o.forward.interact).collect();
            assert_eq!(held, [true, true, false], "{variant:?}");
        }
        let outs = run(cfg(R), &script("R1*3 -"));
        assert!(outs.iter().all(|o| !o.forward.interact));
        assert_eq!(outs[3].action, Action::DelayedInteract);
    }

    #[test]
    fn clear_buffers_signals_entry_into_command_mode() {
        let clears = |variant, text: &str| -> Vec<bool> {
            run(cfg(variant), &script(text))
                .iter()
                .map(|o| o.clear_buffers)
                .collect()
        };
        assert_eq!(clears(L, "L1 L1+Sq L1+Cir -"), [false, true, false, false]);
        let mut ten = vec![false; 9];
        ten.extend([true, false]);
        assert_eq!(clears(L, "L1*10 -"), ten);
        assert_eq!(clears(L, "L1*3 -"), [false; 4]);
        assert_eq!(clears(R, "R1 R1+Sq"), [false, true]);
        assert_eq!(clears(T, "Tog Sq"), [true, false]);
        assert_eq!(clears(T, "L1 -"), [false, false]);
        assert_eq!(clears(B, "L1 L1+Sq"), [false, false]);
    }

    // ---- swap gate model: hurt, cooldown and buffer windows as inputs ----

    /// Mirror of `PartyState::request_swap` for solo, unpaused play with a
    /// `clear` hook that production does not have yet (01D adds it).
    struct SwapGate {
        cooldown: u8,
        buffer: u8,
        tuning: PartyTuning,
    }

    impl SwapGate {
        fn new() -> Self {
            Self {
                cooldown: 0,
                buffer: 0,
                tuning: PartyTuning::default(),
            }
        }

        fn request(&mut self, pressed: bool, hurt: bool) -> bool {
            self.cooldown = self.cooldown.saturating_sub(1);
            self.buffer = self.buffer.saturating_sub(1);
            if pressed {
                self.buffer = self.tuning.swap_buffer_ticks.max(1);
            }
            if self.buffer == 0 {
                return false;
            }
            if (hurt && self.tuning.swap_blocked_while_hurt) || self.cooldown > 0 {
                return false;
            }
            self.buffer = 0;
            self.cooldown = self.tuning.swap_cooldown_ticks;
            true
        }

        fn clear(&mut self) {
            self.buffer = 0;
        }
    }

    /// Ticks at which a swap landed, honoring `clear_buffers` when asked to.
    fn swaps_through_gate(variant: GrammarVariant, text: &str, honor_clear: bool) -> Vec<usize> {
        let ticks = script(text);
        let outs = run(cfg(variant), &ticks);
        check(variant, &outs);
        let mut gate = SwapGate::new();
        let mut landed = Vec::new();
        for (i, (tick, out)) in ticks.iter().zip(&outs).enumerate() {
            if honor_clear && out.clear_buffers {
                gate.clear();
            }
            if gate.request(out.action == Action::Swap, tick.hurt) {
                landed.push(i);
            }
        }
        landed
    }

    #[test]
    fn swap_gate_model_matches_party_state() {
        let mut state = 0x2545_f491_u32;
        let mut party = PartyState::default();
        let mut gate = SwapGate::new();
        for i in 0..4000 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let pressed = (state >> 24).is_multiple_of(9);
            let hurt = (state >> 16).is_multiple_of(5);
            assert_eq!(
                party.request_swap(pressed, hurt),
                gate.request(pressed, hurt),
                "tick {i}"
            );
        }
    }

    #[test]
    fn hurt_window_buffers_a_release_swap_up_to_six_ticks() {
        // Tap released on tick 2 while hurt; hurt ends on tick 4: lands there.
        let landed = swaps_through_gate(L, "L1+Hurt L1+Hurt Hurt Hurt - -", true);
        assert_eq!(landed, [4]);
        // Hurt outlasts the 6 tick buffer: the swap is dropped, never late.
        let landed = swaps_through_gate(L, "L1+Hurt L1+Hurt Hurt*8 - -", true);
        assert!(landed.is_empty());
    }

    #[test]
    fn cooldown_window_swallows_a_second_tap_and_allows_commands() {
        // First tap swaps on tick 2 and starts the 20 tick cooldown. A second
        // tap released on tick 8 is buffered for 6 ticks and expires.
        let landed = swaps_through_gate(L, "L1*2 -*3 L1*2 -*6", true);
        assert_eq!(landed, [2]);
        // A chord during that cooldown is still a command.
        assert_eq!(actions(L, "L1*2 -*3 L1 L1+Sq -"), "..S...F.");
    }

    #[test]
    fn a_chord_clears_a_buffered_swap_so_no_phantom_swap_follows() {
        // Tap released on tick 2 while hurt (buffered); chord on ticks 3-4;
        // hurt ends on tick 6.
        let text = "L1+Hurt L1+Hurt Hurt L1+Hurt L1+Sq+Hurt Hurt - - - -";
        assert_eq!(actions(L, text), "..S.F.....");
        assert!(swaps_through_gate(L, text, true).is_empty());
        // Negative control: without honoring clear_buffers the buffered swap
        // would fire after the command (the C-05 phantom swap).
        assert_eq!(swaps_through_gate(L, text, false), [6]);
    }

    // ---- latency (ticks from the physical press edge) ----

    fn ticks_to_swap(variant: GrammarVariant, down: usize) -> Option<usize> {
        let mut party = PartyState::default();
        let mut grammar = CommandGrammar::new(cfg(variant));
        for t in 0..down + 8 {
            let tick = if t < down {
                TickInput::NONE.with(Btn::L1)
            } else {
                TickInput::NONE
            };
            let out = grammar.step(tick);
            if party.request_swap(out.action == Action::Swap, false) {
                return Some(t);
            }
        }
        None
    }

    #[test]
    fn swap_latency_versus_baseline() {
        for down in 1..=9 {
            assert_eq!(ticks_to_swap(B, down), Some(0), "baseline");
            assert_eq!(
                ticks_to_swap(L, down),
                Some(down),
                "b, L1 down {down} ticks"
            );
            assert_eq!(ticks_to_swap(R, down), Some(0), "c");
            assert_eq!(ticks_to_swap(T, down), Some(0), "d");
        }
        // 10 ticks or more is a hold: no swap in b.
        assert_eq!(ticks_to_swap(L, 10), None);
        assert_eq!(ticks_to_swap(L, 30), None);
    }

    #[test]
    fn command_and_open_latency() {
        let first = |variant, text: &str, pred: fn(&Action) -> bool| {
            first_tick(cfg(variant), &script(text), pred)
        };
        let is_cmd = |a: &Action| matches!(a, Action::Command(_));
        // Fast chord: acts on the face press tick, zero ticks after the edge.
        assert_eq!(first(L, "L1*3 L1+Sq", is_cmd), Some(3));
        assert_eq!(first(L, "L1+Sq", is_cmd), Some(0));
        assert_eq!(first(R, "R1*3 R1+Sq", is_cmd), Some(3));
        assert_eq!(first(T, "Tog -*2 Sq", is_cmd), Some(3));
        // Hold: the mode opens on the 10th tick down (9 ticks after the press).
        let is_open = |a: &Action| *a == Action::OpenMode;
        assert_eq!(first(L, "L1*20", is_open), Some(9));
        assert_eq!(first(R, "R1*20", is_open), Some(9));
        assert_eq!(first(T, "Tog*20", is_open), Some(0));
        assert_eq!(first(B, "L1*20", is_open), None);
        // Variant c delays interact until the R1 release.
        let is_int = |a: &Action| *a == Action::DelayedInteract;
        assert_eq!(first(R, "R1*5 -", is_int), Some(5));
    }

    // ---- reconnect and two humans ----

    #[test]
    fn disconnect_clears_modal_state_and_reconnect_latches_held_buttons() {
        // Mode open, pad drops: one CancelMode, buffers cleared.
        let outs = run(cfg(L), &script("L1*10 disc L1 -"));
        assert_eq!(render(&outs), ".........OC..");
        assert!(outs[10].clear_buffers && !outs[10].mode_open);
        // While away, everything is ignored.
        assert_eq!(actions(L, "disc L1+Sq -"), "...");
        // L1 still held when the pad returns must not swap or open.
        let expected = format!("{}S", ".".repeat(27));
        assert_eq!(actions(L, "L1*2 disc - reconn+L1 L1*20 - L1 -"), expected);
        // A button held across the reconnect does not fabricate a press.
        let outs = run(cfg(L), &script("disc reconn+Sq Sq Sq - Sq"));
        let attack: Vec<bool> = outs.iter().map(|o| o.forward.attack).collect();
        assert_eq!(attack, [false, false, false, false, false, true]);
        // Disconnect during a hold with no selection: no stale swap afterwards.
        assert_eq!(actions(L, "L1*4 disc reconn - -"), "........");
        for variant in [B, L, R, T] {
            let outs = run(cfg(variant), &script("disc*3 reconn - L1 -"));
            let swaps = outs.iter().filter(|o| o.action == Action::Swap).count();
            assert_eq!(swaps, 1, "{variant:?}: one swap after reconnect");
        }
    }

    #[test]
    fn second_human_join_disables_the_grammar_and_leave_reenables_it() {
        // Mode open when P2 joins: one CancelMode, then pass-through only.
        let outs = run(cfg(L), &script("L1*10 join L1 L1+Sq L1*30 -"));
        assert_eq!(&render(&outs)[..11], ".........OC");
        assert!(outs[10].clear_buffers);
        assert!(outs[11..].iter().all(|o| o.action == Action::PassThrough));
        assert!(
            outs[12].forward.attack,
            "the face press is plain gameplay in 2P"
        );
        assert!(!outs[12].mode_open);
        for variant in [B, L, R, T] {
            let outs = run(
                cfg(variant),
                &script("join L1*3 -*3 L1*30 -*2 R1*20 Tog*20 Tog+Sq"),
            );
            assert!(
                outs.iter().all(|o| o.action == Action::PassThrough),
                "{variant:?}: no swap, command or mode in two-human play"
            );
        }
        // Leaving 2P re-enables; a button held at that moment is latched.
        assert_eq!(actions(L, "join - leave+L1 L1 - L1 -"), "......S");
        let mut grammar = CommandGrammar::new(cfg(L));
        grammar.step(TickInput::NONE.with(Btn::L1));
        let mut join = TickInput::NONE;
        join.link = Some(LinkEvent::SecondHumanJoined);
        grammar.step(join);
        assert!(!grammar.is_enabled());
    }

    #[test]
    fn hurt_is_context_only_the_grammar_never_reads_it() {
        for variant in GrammarVariant::ALL {
            let plain = script("L1 L1+Sq L1*3 -*2 R1 R1+Cro - Tog Tri L1*12 -");
            let mut hurt = plain.clone();
            for tick in &mut hurt {
                tick.hurt = true;
            }
            assert_eq!(run(cfg(variant), &plain), run(cfg(variant), &hurt));
        }
    }

    // ---- baseline parity and determinism ----

    fn lcg_script(seed: u32, len: usize, with_links: bool) -> Vec<TickInput> {
        let mut state = seed;
        let mut next = move || {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            state >> 16
        };
        // Buttons change rarely so holds, taps and chords all occur.
        let mut cur = TickInput::NONE;
        (0..len)
            .map(|_| {
                let r = next();
                if r.is_multiple_of(4) {
                    let which = (next() % 8) as usize;
                    match which {
                        0 => cur.l1 = !cur.l1,
                        1 => cur.r1 = !cur.r1,
                        2 => cur.square = !cur.square,
                        3 => cur.triangle = !cur.triangle,
                        4 => cur.circle = !cur.circle,
                        5 => cur.cross = !cur.cross,
                        6 => cur.down = !cur.down,
                        _ => cur.toggle = !cur.toggle,
                    }
                }
                let mut tick = cur;
                tick.commands_blocked = next().is_multiple_of(50);
                if with_links && next().is_multiple_of(60) {
                    tick.link = Some(match next() % 4 {
                        0 => LinkEvent::PadDisconnected,
                        1 => LinkEvent::PadReconnected,
                        2 => LinkEvent::SecondHumanJoined,
                        _ => LinkEvent::SecondHumanLeft,
                    });
                }
                tick
            })
            .collect()
    }

    #[test]
    fn baseline_matches_production_l1_press_edge_and_passes_everything() {
        let ticks = lcg_script(7, 5000, false);
        let outs = run(cfg(B), &ticks);
        let mut frame = InputFrame::default();
        for (i, (tick, out)) in ticks.iter().zip(&outs).enumerate() {
            frame.advance(RawInput {
                swap: tick.l1,
                jump: tick.cross,
                attack: tick.square,
                physical: tick.triangle,
                ability: tick.circle,
                ..RawInput::default()
            });
            assert_eq!(out.action == Action::Swap, frame.swap_pressed, "tick {i}");
            assert_eq!(out.forward.jump, tick.cross, "tick {i}");
            assert_eq!(out.forward.attack, tick.square, "tick {i}");
            assert_eq!(out.forward.physical, tick.triangle, "tick {i}");
            assert_eq!(out.forward.ability, tick.circle, "tick {i}");
            assert_eq!(out.forward.interact, tick.r1, "tick {i}");
            assert_eq!(out.forward.down, tick.down, "tick {i}");
            assert!(!out.clear_buffers && !out.mode_open);
        }
    }

    #[test]
    fn fuzzed_streams_are_deterministic_and_never_leak() {
        for variant in [L, R, T] {
            for seed in 1..=40_u32 {
                let ticks = lcg_script(seed, 600, seed.is_multiple_of(2));
                let first = run(cfg(variant), &ticks);
                assert_eq!(first, run(cfg(variant), &ticks), "{variant:?} seed {seed}");
                check(variant, &first);
                let mut enabled = true;
                let mut connected = true;
                for (tick, out) in ticks.iter().zip(&first) {
                    match tick.link {
                        Some(LinkEvent::SecondHumanJoined) => enabled = false,
                        Some(LinkEvent::SecondHumanLeft) => enabled = true,
                        Some(LinkEvent::PadDisconnected) => connected = false,
                        Some(LinkEvent::PadReconnected) => connected = true,
                        None => {}
                    }
                    if !enabled || !connected {
                        assert!(
                            matches!(out.action, Action::PassThrough | Action::CancelMode),
                            "{variant:?} seed {seed}: {:?} while disabled or away",
                            out.action
                        );
                    }
                    if !connected {
                        assert_eq!(out.forward, Forward::NONE);
                    }
                }
            }
        }
    }

    #[test]
    fn variant_b_swaps_only_after_a_clean_short_l1_tap() {
        for seed in 1..=60_u32 {
            let ticks = lcg_script(seed, 800, false);
            let outs = run(cfg(L), &ticks);
            let mut down_for = 0_u32;
            let mut dirty = false;
            let mut prev = TickInput::NONE;
            for (tick, out) in ticks.iter().zip(&outs) {
                let face_edge = (tick.square && !prev.square)
                    || (tick.triangle && !prev.triangle)
                    || (tick.circle && !prev.circle)
                    || (tick.cross && !prev.cross);
                if tick.l1 {
                    down_for += 1;
                    dirty |= face_edge;
                } else if prev.l1 {
                    dirty |= face_edge;
                    if out.action == Action::Swap {
                        assert!(!dirty, "seed {seed}: swap after a chord");
                        assert!(down_for < u32::from(DEFAULT_HOLD_TICKS), "seed {seed}");
                    }
                    down_for = 0;
                    dirty = false;
                } else {
                    assert_ne!(out.action, Action::Swap, "seed {seed}: swap without a tap");
                }
                prev = *tick;
            }
        }
    }
}
