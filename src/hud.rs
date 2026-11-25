//! HUD-HYBRID-A-01: the Bevy-free state model of the hybrid A HUD.
//!
//! [`build_snapshot`] is a pure function from real game values (Carl's and
//! Donut's authoritative health, the notice queue fed by real
//! [`CombatEvent`]s) plus an optional preview fixture to a [`HudSnapshot`].
//! Rendering (`hud_draw`, `hud_adapter`) reads only the snapshot; gameplay never
//! reads anything from here. A fixture overrides HUD fields only: it never
//! touches the simulation, the combat state, input, RNG or a replay (tested).
//!
//! Spec: `docs/02-art/hybrid-hud-a-native-spec.md` (sections 2 to 6). Debt
//! that retires each fixture: `docs/04-production/backlog.md`, HUD-DEBT-01..07.

use crate::audience::{
    AnticipationCue, AudienceInput, AudienceState, AudienceView, ChainFeat, CueError, Trend,
};
use crate::boss::{BOSS_ENEMY_ID, BossState};
use crate::combat::{CombatEvent, ForceTier, HERO_HEALTH};
use crate::corridor::DONUT_HEALTH;
use crate::donut_hud;
use crate::donut_mana::EmptyPress;
use crate::env_kit::IRect;
use crate::hud_settings::HudSettings;
use crate::party::HeroId;

// ------------------------------------------------------------------- numbers

/// Virtual canvas width.
pub const VIEW_WIDTH: i32 = 640;
/// Safe margin on every side.
pub const SAFE_MARGIN: i32 = 8;
/// Top of the action lane: nothing of the HUD reaches below y = 60 at the top.
pub const ACTION_LANE_TOP: i32 = 60;

pub const PLATE_W: i32 = 164;
pub const PLATE_H: i32 = 48;
pub const HP_BAR_INNER_W: i32 = 94;
pub const MANA_BAR_INNER_W: i32 = 94;
pub const BOSS_BAR_INNER_W: i32 = 234;

/// Notice timeline (ticks, section 5). Slide in 8, hold 120, slide out 8.
pub const NOTICE_SLIDE_TICKS: u16 = 8;
pub const NOTICE_HOLD_TICKS: u16 = 120;
pub const NOTICE_TOTAL_TICKS: u16 = NOTICE_SLIDE_TICKS * 2 + NOTICE_HOLD_TICKS;
/// Reduced motion: no slides, so the notice shows for the hold only.
pub const NOTICE_REDUCED_TOTAL_TICKS: u16 = NOTICE_HOLD_TICKS;
pub const NOTICE_SLIDE_PX_PER_TICK: i32 = 3;
pub const NOTICE_MIN_GAP_TICKS: u16 = 30;
pub const NOTICE_REPEAT_TICKS: u64 = 300;
/// The System face blinks for 6 ticks at the end of every 240-tick period.
pub const BLINK_PERIOD_TICKS: u64 = 240;
pub const BLINK_TICKS: u64 = 6;
/// HP loss flash: 24 ticks, toggling every 4.
pub const HP_FLASH_TICKS: u8 = 24;
pub const HP_FLASH_STEP_TICKS: u8 = 4;
/// Mana-depleted outline flash.
pub const MANA_FLASH_TICKS: u8 = 6;
/// Boss warning icon toggles bright/dim every 8 ticks.
pub const WARNING_BLINK_TICKS: u64 = 8;
/// Boss strip slide: 3 px per tick from y = -26 to 8 (the last step clamps).
pub const BOSS_SLIDE_PX_PER_TICK: i32 = 3;
pub const BOSS_SLIDE_TICKS: u8 = 12;

/// The location text is the real floor number read from the layout id
/// (`floor01_...` -> `FLOOR 01`); no area name exists in the layout data
/// (spec 2.2, open question 2).
pub fn location_text(layout_id: &str) -> Option<String> {
    let digits = layout_id.strip_prefix("floor")?;
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    let number = &digits[..end];
    (!number.is_empty() && (end == digits.len() || digits[end..].starts_with('_')))
        .then(|| format!("FLOOR {number}"))
}

/// F01-HUD-ROOM: the widest text the location strip holds (its 280 px cap
/// minus the 28 px of caps and padding).
pub const LOCATION_TEXT_MAX_PX: i32 = 252;

/// F01-HUD-ROOM: the location text of a late-Floor-1 room: the manifest's room
/// title in capitals and the attempt counter (`THE HOARDER · TRY 2`). A title
/// too wide for the strip loses letters from its end, never the counter.
pub fn room_location_text(title: &str, attempt: u32) -> String {
    let suffix = format!(" \u{b7} TRY {attempt}");
    let mut upper: Vec<char> = title.to_uppercase().chars().collect();
    loop {
        let text: String = upper.iter().collect::<String>() + &suffix;
        if upper.is_empty() || text_width_px(HudFont::Jersey, &text) <= LOCATION_TEXT_MAX_PX {
            return text;
        }
        upper.pop();
    }
}

/// F01-HUD-ROOM (PT-SCENE U5): the tag under the top strip of a room that did
/// not start from the story: the `all` loadout, or a story room launched from its
/// static pin instead of a carried ledger.
pub const LOADOUT_TAG_TEXT: &str = "TEST LOADOUT";

// ------------------------------------------------------------------- palette

/// An sRGB colour, `0xRRGGBB`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const fn hex(value: u32) -> Self {
        Self(
            (value >> 16) as u8,
            (value >> 8 & 0xff) as u8,
            (value & 0xff) as u8,
        )
    }
}

/// The 34-swatch HUD palette of spec section 3.
pub mod palette {
    use super::Rgb;

    pub const HUD_INK: Rgb = Rgb::hex(0x120c18);
    pub const PLATE_FILL: Rgb = Rgb::hex(0x211b27);
    pub const METAL_DARK: Rgb = Rgb::hex(0x2a2430);
    pub const METAL_MID: Rgb = Rgb::hex(0x3d3542);
    pub const METAL_LIGHT: Rgb = Rgb::hex(0x5c5262);
    pub const METAL_HI: Rgb = Rgb::hex(0x8a7e8c);
    pub const RUST: Rgb = Rgb::hex(0x7a3f2a);
    pub const BRASS_DARK: Rgb = Rgb::hex(0x6a4a1c);
    pub const BRASS: Rgb = Rgb::hex(0xb58a3a);
    pub const BRASS_HI: Rgb = Rgb::hex(0xe8c063);
    pub const AMBER: Rgb = Rgb::hex(0xffc24a);
    pub const AMBER_DIM: Rgb = Rgb::hex(0xa8742a);
    pub const TEXT_WHITE: Rgb = Rgb::hex(0xf4e6d2);
    pub const MAGENTA_DK: Rgb = Rgb::hex(0x6e1a4c);
    pub const MAGENTA: Rgb = Rgb::hex(0xe0338f);
    pub const MAGENTA_HI: Rgb = Rgb::hex(0xff6fc0);
    pub const SCREEN_BG: Rgb = Rgb::hex(0x2a0d24);
    pub const CRT_FACE_HI: Rgb = Rgb::hex(0xffb3dc);
    pub const NOTICE_BG: Rgb = Rgb::hex(0x1e0f22);
    pub const HP_RED_LO: Rgb = Rgb::hex(0x9a1a2c);
    pub const HP_RED: Rgb = Rgb::hex(0xd8283c);
    pub const HP_RED_HI: Rgb = Rgb::hex(0xff5a5f);
    pub const HP_EMPTY: Rgb = Rgb::hex(0x3a1620);
    pub const HP_FLASH: Rgb = Rgb::hex(0xfff1e0);
    /// Violet to cyan, one flat colour per mana segment.
    pub const MANA: [Rgb; 8] = [
        Rgb::hex(0x7a3cf0),
        Rgb::hex(0x6a4ff0),
        Rgb::hex(0x5a63f2),
        Rgb::hex(0x4a7af4),
        Rgb::hex(0x3d92f4),
        Rgb::hex(0x33aaf0),
        Rgb::hex(0x33c2ec),
        Rgb::hex(0x45d8ea),
    ];
    pub const MANA_EMPTY: Rgb = Rgb::hex(0x1c1a33);
    pub const CYAN: Rgb = Rgb::hex(0x5fe0e8);
    pub const WELL_CARL: Rgb = Rgb::hex(0x2b2038);
    pub const WELL_DONUT: Rgb = Rgb::hex(0x301a2e);
}

// -------------------------------------------------------------------- labels

/// Text the HUD can show. Copy lives here, in one table (spec open question 1:
/// the notice headlines are placeholders pending an owner tone check).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelId {
    Punch,
    Kick,
    Swat,
    Pounce,
    Missile,
    Companion,
    Launched,
    Flattened,
    /// Fixture-only copy: the `boss` preview. The real encounter raises the
    /// notices below (HUD-DEBT-03; copy pending the owner's tone check).
    BossIncoming,
    /// The boss gate closed.
    MainEvent,
    /// The boss entered phase 2 / 3.
    RoundTwo,
    FinalRound,
    /// A rack landed on the boss.
    NoSpotter,
    /// The boss was defeated.
    Knockout,
    /// CP-Q32: the thrown charge's socket label (`cpq32_socket_label`, `BOOM`).
    Boom,
    /// CP-Q11: the grant notice of the thrown charge (the `notices` row of
    /// `story_unlocks_v0.json`).
    GoblinBoom,
    /// DP-Q33: Donut's blink socket label (`dpq33_socket_label`, `BLINK`).
    Blink,
    /// DP-Q12: the grant notice of the jump boost (the `notices` row).
    LightFeet,
    /// DP-Q12: the grant notice of the short blink (the `notices` row).
    PuddleJumper,
    /// CP-Q43: the grant notice of Carl's reinforced fist (the `notices` row).
    HeavyFist,
}

impl LabelId {
    pub const fn text(self) -> &'static str {
        match self {
            Self::Punch => "PUNCH",
            Self::Kick => "KICK",
            Self::Swat => "SWAT",
            Self::Pounce => "POUNCE",
            Self::Missile => "MISSILE",
            Self::Companion => "COMPANION",
            Self::Launched => "LAUNCHED!",
            Self::Flattened => "FLATTENED!",
            Self::BossIncoming => "BOSS INCOMING",
            Self::MainEvent => "MAIN EVENT!",
            Self::RoundTwo => "ROUND TWO",
            Self::FinalRound => "FINAL ROUND",
            Self::NoSpotter => "NO SPOTTER!",
            Self::Knockout => "KNOCKOUT!",
            Self::Boom => "BOOM",
            Self::GoblinBoom => "GOBLIN BOOM",
            Self::Blink => "BLINK",
            Self::LightFeet => "LIGHT FEET",
            Self::PuddleJumper => "PUDDLE JUMPER",
            Self::HeavyFist => "HEAVY FIST",
        }
    }

    /// CP-Q32: the socket label named by the data (`cpq32_socket_label`), when
    /// the HUD has a label for it. The label set is code (a `&'static str` per
    /// entry), the choice is data; a text the HUD has no entry for is `None`
    /// and the caller keeps the default.
    pub fn for_socket_text(text: &str) -> Option<Self> {
        [Self::Boom, Self::Blink]
            .into_iter()
            .find(|label| text == label.text())
    }

    /// A capability grant notice (the `notices` rows): the queue never lets a
    /// later headline replace one, and a second grant waits behind the first
    /// (DP-Q12: with both programs Carl's notice shows first, then Donut's).
    pub const fn is_grant(self) -> bool {
        matches!(
            self,
            Self::GoblinBoom | Self::HeavyFist | Self::LightFeet | Self::PuddleJumper
        )
    }

    /// CP-Q11, DP-Q12: the grant notice of a capability by the data's notice text
    /// (the `notices` rows of `story_unlocks_v0.json`). The thrown charge and
    /// Donut's two grants have a HUD label; the other grants add theirs with their
    /// stage.
    pub fn for_notice_text(text: &str) -> Option<Self> {
        [
            Self::GoblinBoom,
            Self::HeavyFist,
            Self::LightFeet,
            Self::PuddleJumper,
        ]
        .into_iter()
        .find(|label| text == label.text())
    }
}

/// Which real defeat forces raise a System notice, and the headline for each.
/// A routine kill is deliberately absent: too frequent, it would become a
/// permanent overlay (spec 2.5).
pub const NOTICE_TRIGGERS: [(ForceTier, LabelId); 2] = [
    (ForceTier::Heavy, LabelId::Launched),
    (ForceTier::Catastrophic, LabelId::Flattened),
];

/// The notice headline a combat event raises, if any. `GoblinTell` (a threat
/// telegraph), `HeroHurt`, `HeroDefeated` and routine kills never do.
pub fn notice_for_event(event: &CombatEvent) -> Option<LabelId> {
    match event {
        // The boss encounter (HUD-DEBT-03). Its defeat is a knockout whatever
        // the force; every other defeat keeps the table below.
        CombatEvent::EnemyDefeated { target, .. } if *target == BOSS_ENEMY_ID => {
            Some(LabelId::Knockout)
        }
        CombatEvent::BossEngaged { .. } => Some(LabelId::MainEvent),
        CombatEvent::BossPhase { phase: 2, .. } => Some(LabelId::RoundTwo),
        CombatEvent::BossPhase { phase: 3, .. } => Some(LabelId::FinalRound),
        CombatEvent::RackLanded { crushed: true, .. } => Some(LabelId::NoSpotter),
        // CP-Q11: Carl's thrown charge is granted (the grant notice; the socket
        // turns equipped on the same tick through `HudRuntime::set_special`).
        CombatEvent::CapabilityGranted {
            hero: 0,
            capability,
            ..
        } if *capability == crate::story_unlock::SPECIAL_THROWN_CHARGE.as_str() => {
            Some(LabelId::GoblinBoom)
        }
        // CP-Q43: Carl's reinforced fist, from the same event.
        CombatEvent::CapabilityGranted {
            hero: 0,
            capability,
            ..
        } if *capability == crate::story_unlock::FIST_REINFORCED.as_str() => {
            Some(LabelId::HeavyFist)
        }
        // DP-Q12: Donut's two grants, from the same event.
        CombatEvent::CapabilityGranted {
            hero: 1,
            capability,
            ..
        } if *capability == crate::donut_prog::JUMP_REACH_BOOST.as_str() => {
            Some(LabelId::LightFeet)
        }
        CombatEvent::CapabilityGranted {
            hero: 1,
            capability,
            ..
        } if *capability == crate::donut_prog::SPECIAL_SHORT_BLINK.as_str() => {
            Some(LabelId::PuddleJumper)
        }
        CombatEvent::EnemyDefeated { force, .. } => NOTICE_TRIGGERS
            .iter()
            .find(|(tier, _)| tier == force)
            .map(|(_, label)| *label),
        _ => None,
    }
}

// ------------------------------------------------------------ small domain

/// The logical actions the ability chips can show (control contract). Jump,
/// Interact, Swap and Pause are never shown as chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalAction {
    /// Square / X.
    Primary,
    /// Triangle / Y.
    Physical,
    /// Circle / B: the equipped special (CP-Q32). Shown only in the special
    /// socket, and only once a special is held.
    Special,
}

/// USB vendor id of Sony Interactive Entertainment (DualShock 4, DualSense,
/// DualSense Edge all report it).
pub const SONY_VENDOR_ID: u16 = 0x054C;

/// Controller glyph family. Xbox is the default and the fallback; the family
/// follows the last-used gamepad ([`GlyphSet::for_vendor_id`], HUD-DEBT-05),
/// unless `--glyphs=` forces one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlyphSet {
    #[default]
    Xbox,
    DualSense,
}

impl GlyphSet {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "xbox" => Some(Self::Xbox),
            "dualsense" => Some(Self::DualSense),
            _ => None,
        }
    }

    /// The glyph family for a gamepad's USB vendor id. Sony gets the
    /// PlayStation set. Everything else (Microsoft, Valve/Steam Deck, Nintendo,
    /// third-party pads, no id at all) gets the Xbox set.
    pub const fn for_vendor_id(vendor_id: Option<u16>) -> Self {
        match vendor_id {
            Some(SONY_VENDOR_ID) => Self::DualSense,
            _ => Self::Xbox,
        }
    }

    /// The glyph for a chip action. Never Jump (A/Cross). The special
    /// (B/Circle) only for an **equipped** special: a locked one is an empty
    /// padlock socket (spec 2.4).
    pub const fn glyph(self, action: LogicalAction) -> Glyph {
        match (self, action) {
            (Self::Xbox, LogicalAction::Primary) => Glyph::X,
            (Self::Xbox, LogicalAction::Physical) => Glyph::Y,
            (Self::DualSense, LogicalAction::Primary) => Glyph::Square,
            (Self::DualSense, LogicalAction::Physical) => Glyph::Triangle,
            (Self::Xbox, LogicalAction::Special) => Glyph::B,
            (Self::DualSense, LogicalAction::Special) => Glyph::Circle,
        }
    }
}

/// Every glyph the art atlas carries (the Xbox set and the DualSense set).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    A,
    B,
    X,
    Y,
    Cross,
    Circle,
    Square,
    Triangle,
}

/// Who drives a plate. Spec 6: `Player(1)`, `Player(2)` or `Ai`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Player(u8),
    Ai,
}

/// IP-neutral portrait id; the fan-concept layer maps it to art.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortraitId(pub &'static str);

pub const PORTRAIT_CARL: PortraitId = PortraitId("carl");
pub const PORTRAIT_DONUT: PortraitId = PortraitId("donut");

/// IP-neutral fallback names, drawn only when no character data supplied a
/// display name. The real names come from the actor manifests (fan-concept
/// layer) through [`RealHudInputs`].
pub const FALLBACK_NAME_LEFT: &str = "HERO";
pub const FALLBACK_NAME_RIGHT: &str = "ALLY";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meter {
    pub cur: u8,
    pub max: u8,
}

/// An HP loss flash: the segments `[lost_to, lost_from)` flash for
/// [`HP_FLASH_TICKS`] ticks (`lost_from` is the HP before the hit, `lost_to`
/// the HP after it, i.e. `health_left`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HpFlash {
    pub lost_from: u8,
    pub lost_to: u8,
    pub age_ticks: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaView {
    /// Filled pixels of the 94 px track, `0..=94`.
    pub fill_px: u8,
    /// Ticks since the mana reached 0 (drives the one-time outline flash).
    pub depleted_age: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chip {
    pub action: LogicalAction,
    pub label: LabelId,
    pub muted: bool,
}

/// The special socket: `Locked` (the padlock) until a special is held, then
/// `Equipped(Chip)` (CP-Q32): the Circle/B glyph and the data label, muted while
/// the special cannot be used (a throw or stick live, or recharging).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialSlot {
    Locked,
    Equipped(Chip),
    /// DP-Q33: an equipped special that is cooling down: the muted chip and the
    /// 1 px cooldown underline of `bar_px` pixels (`1..=SOCKET_W`) in place of
    /// the muted zig-zag.
    Cooling {
        chip: Chip,
        bar_px: u8,
    },
}

/// The socket cell's width in virtual pixels (the glyph and the padlock).
pub const SOCKET_W: i32 = 11;

/// DP-Q33: which cue flashes the socket once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketFlashKind {
    /// The special is ready again (`SpecialReady`).
    Ready,
    /// A press was refused or the blink fizzled (`NoMana`, `NoDestination`),
    /// the dry-press cue.
    Refused,
}

/// DP-Q33: a socket flash, aged in HUD ticks. Presentation only, like the mana
/// outline flash: `MANA_FLASH_TICKS` long.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketFlash {
    pub kind: SocketFlashKind,
    pub age_ticks: u8,
}

/// What Carl's special is doing, as the socket shows it (CP-Q32). Fed by the
/// scene from the ledger and the thrower; presentation only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpecialPhase {
    /// Nothing held: the padlock.
    #[default]
    Locked,
    /// Held and usable.
    Ready,
    /// A throw or its stick is live (from the press to the detonation).
    Live,
    /// Detonated; the recharge runs.
    Recharging,
}

impl SpecialPhase {
    /// What the socket shows for Carl's special in `combat` (CP-Q32): the padlock
    /// until the ledger holds the thrown charge, then ready, live (a throw or its
    /// stick, from the press to the detonation) or recharging.
    pub fn of(combat: &crate::combat_scene::CombatState) -> Self {
        if !combat.story_holds(crate::story_unlock::SPECIAL_THROWN_CHARGE) {
            return Self::Locked;
        }
        let thrower = combat.thrower();
        if thrower.recharging() {
            Self::Recharging
        } else if thrower.is_idle() {
            Self::Ready
        } else {
            Self::Live
        }
    }

    /// The socket for this phase with `label`. Live and recharging are both
    /// muted: the cue is the same (the special cannot be used), the glyph and
    /// label dim and the zig-zag underline appears.
    pub const fn slot(self, label: LabelId) -> SpecialSlot {
        match self {
            Self::Locked => SpecialSlot::Locked,
            Self::Ready | Self::Live | Self::Recharging => SpecialSlot::Equipped(Chip {
                action: LogicalAction::Special,
                label,
                muted: !matches!(self, Self::Ready),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityRow {
    Chips {
        a: Chip,
        b: Chip,
        special: SpecialSlot,
    },
    /// An AI-controlled plate: no button glyphs (pressing them would act on
    /// the other hero); the label `COMPANION` instead.
    AiCompanion,
    /// A player-controlled plate with no live Primary/Physical action right
    /// now (the story corridor ignores melee): no chips, the layout and the
    /// locked special socket stay (controller contract, spec 2.4).
    NoActions { special: SpecialSlot },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlateView {
    pub portrait: PortraitId,
    pub name: String,
    pub control: Control,
    /// The glyph family of the pad that owns this plate (provisional Q11);
    /// the `--glyphs=` override forces it on both plates.
    pub glyphs: GlyphSet,
    pub hp: Meter,
    pub hp_flash: Option<HpFlash>,
    /// `None` = no mana resource (Carl always; Donut before the biscuit).
    pub mana: Option<ManaView>,
    pub abilities: AbilityRow,
    /// DP-Q33: the one-shot flash of this plate's special socket (Donut's only).
    pub socket_flash: Option<SocketFlash>,
    /// AUDIENCE-01: this hero's live viewer count. `None` while the feature is
    /// locked (the Floor 01 default: no eye, no number, no placeholder), so a
    /// locked plate draws exactly what it drew before. Owned per hero (indexed
    /// by the hero, never by controller slot or P1/P2).
    pub audience: Option<AudienceView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BossView {
    pub name: String,
    /// Filled pixels of the 234 px bar, `0..=234`.
    pub fill_px: u8,
    /// Width of the damage chip that follows the fill.
    pub chip_px: u8,
    /// The boss telegraph flag: shows the warning icon.
    pub warning: bool,
    /// The warning icon is in its bright phase (steady bright when reduced).
    pub warning_bright: bool,
    /// Ticks since the strip started sliding in (`>= BOSS_SLIDE_TICKS` = in).
    pub slide_tick: u8,
    /// The boss' own 18x18 emblem key; `None` draws the generic skull plate
    /// (HUD-DEBT-03, `docs/02-art/floor-01-boss-design.md` section 4).
    pub emblem: Option<EmblemId>,
}

/// The key of a boss emblem (`brute_boss_f01_emblem`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmblemId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoticeView {
    pub headline: LabelId,
    /// Ticks since the notice started.
    pub age_ticks: u16,
}

/// Which System face frame is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceFrame {
    Idle,
    Blink,
    Notice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudPreview {
    PreUnlock,
    PostUnlock,
    Depleted,
    Boss,
    Coop,
    /// AUDIENCE-01 part C: viewer-count readouts on both plates (presentation
    /// only; the real Floor 01 has no unlock, see the spec's F15).
    Viewers,
    ViewersRising,
    ViewersFalling,
    ViewersMax,
}

impl HudPreview {
    pub const ALL: [Self; 9] = [
        Self::PreUnlock,
        Self::PostUnlock,
        Self::Depleted,
        Self::Boss,
        Self::Coop,
        Self::Viewers,
        Self::ViewersRising,
        Self::ViewersFalling,
        Self::ViewersMax,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::PreUnlock => "pre-unlock",
            Self::PostUnlock => "post-unlock",
            Self::Depleted => "depleted",
            Self::Boss => "boss",
            Self::Coop => "coop",
            Self::Viewers => "viewers",
            Self::ViewersRising => "viewers-rising",
            Self::ViewersFalling => "viewers-falling",
            Self::ViewersMax => "viewers-max",
        }
    }

    /// The upper-case text of the PREVIEW tag.
    pub const fn tag_name(self) -> &'static str {
        match self {
            Self::PreUnlock => "PRE-UNLOCK",
            Self::PostUnlock => "POST-UNLOCK",
            Self::Depleted => "DEPLETED",
            Self::Boss => "BOSS",
            Self::Coop => "COOP",
            Self::Viewers => "VIEWERS",
            Self::ViewersRising => "VIEWERS RISING",
            Self::ViewersFalling => "VIEWERS FALLING",
            Self::ViewersMax => "VIEWERS MAX",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == value)
    }

    pub fn names() -> String {
        Self::ALL.map(Self::name).join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudSnapshot {
    /// `[0]` the left slot (Carl), `[1]` the right slot (Donut).
    pub plates: [PlateView; 2],
    /// Real: `FLOOR 01` (plus an area name only if layout data had one); in a
    /// late-Floor-1 room the room title and the attempt (F01-HUD-ROOM).
    pub location: Option<String>,
    /// F01-HUD-ROOM: the TEST LOADOUT tag under the top strip.
    pub loadout_tag: bool,
    /// Fixture only today (HUD-DEBT-03).
    pub boss: Option<BossView>,
    /// Real, from the notice queue fed by [`CombatEvent`]s; or a fixture.
    pub notice: Option<NoticeView>,
    /// `Some(n)`: the game is paused until player `n`'s controller is back
    /// (owner decision D6). Drawn as a persistent notice-frame message.
    pub reconnect: Option<u8>,
    /// `Some` draws the PREVIEW tag.
    pub preview: Option<HudPreview>,
    pub reduced_motion: bool,
    /// HUD-DEBT-06: all HUD text at 7:1 or better, empty-state outlines 13:1,
    /// fills and track outlines at least 4.4:1 (floor01-scene.md, HUD accessibility settings).
    pub high_contrast: bool,
    /// The System face frame (idle, blink, or notice while a panel is up).
    pub face: FaceFrame,
}

// -------------------------------------------------------- real inputs and fn

/// What Donut's magic currently allows (HUD-DEBT-02): `PreBiscuit` until the
/// unlock fires, then the whole-segment view of the real mana pool
/// ([`crate::donut_hud::magic_of`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonutMagic {
    PreBiscuit,
    Unlocked(ManaView),
}

/// The real game values the HUD reads, one tick's worth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealHudInputs {
    pub carl_health: u8,
    pub donut_health: u8,
    /// Display names from the actor manifests (the fan-concept layer).
    pub carl_name: String,
    pub donut_name: String,
    pub carl_control: Control,
    pub donut_control: Control,
    /// Whether the scene currently acts on Primary/Physical (melee): true in
    /// a combat section, false in the story corridor. Chips show only for
    /// live actions.
    pub melee_live: bool,
    /// CP-Q32: Carl's special socket (`Locked` unless the scene feeds a phase).
    pub carl_special: SpecialSlot,
    /// DP-Q33: Donut's special socket (`Locked` unless the scene feeds the blink
    /// state) and its one-shot flash.
    pub donut_special: SpecialSlot,
    pub donut_socket_flash: Option<SocketFlash>,
    pub donut_magic: DonutMagic,
    /// What Primary does at 0 mana (the loaded mana tuning, Q10-16).
    pub empty_press: EmptyPress,
    pub hp_flash: Option<HpFlash>,
    /// Donut's HP flash, from her real hurt events (HUD-DEBT-01).
    pub donut_hp_flash: Option<HpFlash>,
    pub notice: Option<NoticeView>,
    pub location: Option<String>,
    /// F01-HUD-ROOM: draw the TEST LOADOUT tag.
    pub loadout_tag: bool,
    /// Per-plate glyph families (the pad that owns each hero's plate).
    pub carl_glyphs: GlyphSet,
    pub donut_glyphs: GlyphSet,
    /// The player slot (1 or 2) whose controller is missing, if any.
    pub reconnect: Option<u8>,
    /// Q10-01: chips for a human-driven Donut (`PartyTuning::donut_moves`).
    pub donut_chips: bool,
    pub reduced_motion: bool,
    pub high_contrast: bool,
    /// The boss strip, built from the simulation's boss state by
    /// [`HudRuntime`] (`None` outside an encounter).
    pub boss: Option<BossView>,
    /// AUDIENCE-01: each hero's viewer view, `[Carl, Donut]`, `None` while the
    /// feature is locked. Filled by [`HudRuntime`] from the audience observer.
    pub audience: [Option<AudienceView>; 2],
}

impl RealHudInputs {
    /// The name and location fields hold neutral fallbacks; the runtime
    /// fills the real values ([`HudRuntime::set_identity`]). Carl is player 1, Donut is the AI companion, her magic is locked, the
    /// location is `FLOOR 01`, melee is live (a combat section). The runtime
    /// overrides `melee_live` from the scene's real capability.
    pub fn normal(carl_health: u8, donut_health: u8) -> Self {
        Self {
            carl_health,
            donut_health,
            carl_name: FALLBACK_NAME_LEFT.to_string(),
            donut_name: FALLBACK_NAME_RIGHT.to_string(),
            carl_control: Control::Player(1),
            donut_control: Control::Ai,
            melee_live: true,
            carl_special: SpecialSlot::Locked,
            donut_special: SpecialSlot::Locked,
            donut_socket_flash: None,
            donut_magic: DonutMagic::PreBiscuit,
            empty_press: EmptyPress::Dry,
            hp_flash: None,
            donut_hp_flash: None,
            notice: None,
            location: None,
            loadout_tag: false,
            carl_glyphs: GlyphSet::default(),
            donut_glyphs: GlyphSet::default(),
            reconnect: None,
            donut_chips: false,
            reduced_motion: false,
            high_contrast: false,
            boss: None,
            audience: [None, None],
        }
    }
}

/// The chip rows of spec 2.4. `muted` is only ever set by a fixture today.
/// Chips show only while Primary/Physical are live (`melee_live`).
fn carl_abilities(control: Control, melee_live: bool, special: SpecialSlot) -> AbilityRow {
    match control {
        Control::Ai => AbilityRow::AiCompanion,
        Control::Player(_) if !melee_live => AbilityRow::NoActions { special },
        Control::Player(_) => AbilityRow::Chips {
            a: Chip {
                action: LogicalAction::Primary,
                label: LabelId::Punch,
                muted: false,
            },
            b: Chip {
                action: LogicalAction::Physical,
                label: LabelId::Kick,
                muted: false,
            },
            special,
        },
    }
}

/// The chip row of a **preview fixture** (`--hud-preview`): the pre- and
/// post-unlock, depleted and co-op states the HUD can show without a scene.
/// The real plate goes through `donut_hud::donut_ability_row`.
fn donut_abilities(control: Control, magic: DonutMagic) -> AbilityRow {
    match control {
        Control::Ai => AbilityRow::AiCompanion,
        Control::Player(_) => {
            let (label, muted) = match magic {
                DonutMagic::PreBiscuit => (LabelId::Swat, false),
                DonutMagic::Unlocked(mana) => (LabelId::Missile, mana.fill_px == 0),
            };
            AbilityRow::Chips {
                a: Chip {
                    action: LogicalAction::Primary,
                    label,
                    muted,
                },
                b: Chip {
                    action: LogicalAction::Physical,
                    label: LabelId::Pounce,
                    muted: false,
                },
                special: SpecialSlot::Locked,
            }
        }
    }
}

fn plate(
    portrait: PortraitId,
    name: &str,
    control: Control,
    glyphs: GlyphSet,
    hp: Meter,
    abilities: AbilityRow,
) -> PlateView {
    PlateView {
        portrait,
        name: name.to_string(),
        control,
        glyphs,
        hp,
        hp_flash: None,
        mana: None,
        abilities,
        socket_flash: None,
        audience: None,
    }
}

fn face_for(panel_up: bool, tick: u64, reduced_motion: bool) -> FaceFrame {
    if panel_up {
        FaceFrame::Notice
    } else if !reduced_motion && tick % BLINK_PERIOD_TICKS >= BLINK_PERIOD_TICKS - BLINK_TICKS {
        FaceFrame::Blink
    } else {
        FaceFrame::Idle
    }
}

/// The pure HUD state function. `tick` is the HUD clock (fixed ticks stepped).
pub fn build_snapshot(real: &RealHudInputs, preview: Option<HudPreview>, tick: u64) -> HudSnapshot {
    let mut carl = plate(
        PORTRAIT_CARL,
        &real.carl_name,
        real.carl_control,
        real.carl_glyphs,
        Meter {
            cur: real.carl_health.min(HERO_HEALTH),
            max: HERO_HEALTH,
        },
        carl_abilities(real.carl_control, real.melee_live, real.carl_special),
    );
    carl.hp_flash = real.hp_flash;
    let mut donut = plate(
        PORTRAIT_DONUT,
        &real.donut_name,
        real.donut_control,
        real.donut_glyphs,
        Meter {
            cur: real.donut_health.min(DONUT_HEALTH),
            max: DONUT_HEALTH,
        },
        // Q10: the chips, the muted Missile at 0 mana and the AI label come
        // from `donut_hud` (the pure mapping the Q10 spec section 4 pins).
        donut_hud::donut_ability_row(
            real.donut_control,
            real.donut_magic,
            real.empty_press,
            real.donut_chips,
            real.melee_live,
            real.donut_special,
        ),
    );
    donut.socket_flash = real.donut_socket_flash;
    // Mana exists only for Donut, and only once her magic is unlocked.
    if let DonutMagic::Unlocked(mana) = real.donut_magic {
        donut.mana = Some(mana);
    }
    donut.hp_flash = real.donut_hp_flash;
    carl.audience = real.audience[HeroId::Carl.index()];
    donut.audience = real.audience[HeroId::Donut.index()];
    let mut snapshot = HudSnapshot {
        plates: [carl, donut],
        // The location strip gives way to the boss strip.
        location: if real.boss.is_some() {
            None
        } else {
            real.location.clone()
        },
        loadout_tag: real.loadout_tag,
        boss: real.boss.clone(),
        notice: real.notice,
        reconnect: real.reconnect,
        preview: None,
        reduced_motion: real.reduced_motion,
        high_contrast: real.high_contrast,
        face: FaceFrame::Idle,
    };
    if let Some(preview) = preview {
        apply_preview(&mut snapshot, preview, tick);
    }
    snapshot.face = face_for(
        snapshot.notice.is_some() || snapshot.reconnect.is_some(),
        tick,
        snapshot.reduced_motion,
    );
    snapshot
}

/// Mana full / 60 / 31 of the 94 px track (fixture values, not balance).
const MANA_FULL: u8 = HP_BAR_INNER_W as u8;
const MANA_BOSS: u8 = 60;
const MANA_COOP: u8 = 31;

fn set_donut(
    snapshot: &mut HudSnapshot,
    control: Control,
    hp: u8,
    magic: DonutMagic,
    flash_age: Option<u8>,
) {
    let mut donut = plate(
        PORTRAIT_DONUT,
        &snapshot.plates[1].name,
        control,
        snapshot.plates[1].glyphs,
        Meter {
            cur: hp,
            max: DONUT_HEALTH,
        },
        donut_abilities(control, magic),
    );
    if let DonutMagic::Unlocked(mana) = magic {
        donut.mana = Some(ManaView {
            depleted_age: if mana.fill_px == 0 { flash_age } else { None },
            ..mana
        });
    }
    donut.audience = snapshot.plates[1].audience;
    snapshot.plates[1] = donut;
}

fn set_carl(snapshot: &mut HudSnapshot, hp: u8, flash: Option<HpFlash>) {
    let mut carl = plate(
        PORTRAIT_CARL,
        &snapshot.plates[0].name,
        Control::Player(1),
        snapshot.plates[0].glyphs,
        Meter {
            cur: hp,
            max: HERO_HEALTH,
        },
        carl_abilities(Control::Player(1), true, SpecialSlot::Locked),
    );
    carl.hp_flash = flash;
    carl.audience = snapshot.plates[0].audience;
    snapshot.plates[0] = carl;
}

fn mana(fill_px: u8) -> DonutMagic {
    DonutMagic::Unlocked(ManaView {
        fill_px,
        depleted_age: None,
    })
}

/// Overlay a preview fixture. Fixture ages are static so every capture is
/// deterministic whatever the tick. Only HUD fields are written.
fn apply_preview(snapshot: &mut HudSnapshot, preview: HudPreview, tick: u64) {
    snapshot.preview = Some(preview);
    snapshot.boss = None;
    snapshot.notice = None;
    match preview {
        HudPreview::PreUnlock => {
            set_carl(snapshot, 4, None);
            set_donut(
                snapshot,
                Control::Player(2),
                4,
                DonutMagic::PreBiscuit,
                None,
            );
        }
        HudPreview::PostUnlock => {
            set_carl(snapshot, 4, None);
            set_donut(snapshot, Control::Player(2), 4, mana(MANA_FULL), None);
        }
        HudPreview::Depleted => {
            // Carl lost segment 2 (3 -> 2 HP) this tick; Donut's mana just hit 0.
            set_carl(
                snapshot,
                2,
                Some(HpFlash {
                    lost_from: 3,
                    lost_to: 2,
                    age_ticks: 0,
                }),
            );
            set_donut(snapshot, Control::Player(2), 4, mana(0), Some(0));
            snapshot.notice = Some(NoticeView {
                headline: LabelId::Launched,
                age_ticks: 20,
            });
        }
        HudPreview::Boss => {
            set_carl(snapshot, 3, None);
            set_donut(snapshot, Control::Ai, 4, mana(MANA_BOSS), None);
            snapshot.location = None;
            snapshot.boss = Some(BossView {
                name: "TEST BRUTE".to_string(),
                fill_px: 164,
                chip_px: 14,
                warning: true,
                warning_bright: snapshot.reduced_motion
                    || (tick / WARNING_BLINK_TICKS).is_multiple_of(2),
                slide_tick: BOSS_SLIDE_TICKS,
                emblem: None,
            });
            snapshot.notice = Some(NoticeView {
                headline: LabelId::BossIncoming,
                age_ticks: 20,
            });
        }
        HudPreview::Coop => {
            set_carl(snapshot, 3, None);
            set_donut(snapshot, Control::Player(2), 2, mana(MANA_COOP), None);
        }
        // The viewer previews keep the real plates (HP, tags, chips, mana)
        // and only add the readouts, so they read against the real capture.
        HudPreview::Viewers => set_viewers(snapshot, (6_800, Trend::Flat), (12_400, Trend::Flat)),
        HudPreview::ViewersRising => {
            set_viewers(snapshot, (38_400, Trend::Up), (87_100, Trend::Up));
        }
        HudPreview::ViewersFalling => {
            set_viewers(snapshot, (6_800, Trend::Down), (12_400, Trend::Down));
        }
        // The worst-case widths: `999M+` (29 px) and `99.9K` with a fall.
        HudPreview::ViewersMax => {
            set_viewers(snapshot, (5_000_000_000, Trend::Up), (99_900, Trend::Down));
        }
    }
}

/// Fixture values of the viewer previews (not balance): the count, with the
/// trend glyph that is on at the moment the fixture freezes.
fn set_viewers(snapshot: &mut HudSnapshot, carl: (u64, Trend), donut: (u64, Trend)) {
    for (plate, (count, trend)) in snapshot.plates.iter_mut().zip([carl, donut]) {
        plate.audience = Some(AudienceView {
            shown: count,
            animated: count,
            peak: count,
            trend,
            trend_steps_left: if trend == Trend::Flat { 0 } else { 24 },
        });
    }
}

// ---------------------------------------------------------------- the notice

/// Horizontal offset (px, to the right) of the notice panel at `age` ticks:
/// slide in 24 -> 0, hold, slide out 0 -> 24, 3 px per tick. Reduced motion
/// has no slide at all.
pub fn notice_dx(age: u16, reduced_motion: bool) -> i32 {
    if reduced_motion {
        return 0;
    }
    let slide = i32::from(NOTICE_SLIDE_TICKS) * NOTICE_SLIDE_PX_PER_TICK;
    let out_start = NOTICE_SLIDE_TICKS + NOTICE_HOLD_TICKS;
    if age < NOTICE_SLIDE_TICKS {
        slide - i32::from(age) * NOTICE_SLIDE_PX_PER_TICK
    } else if age < out_start {
        0
    } else {
        ((i32::from(age - out_start) + 1) * NOTICE_SLIDE_PX_PER_TICK).min(slide)
    }
}

/// The System notice queue: at most one pending, a minimum gap, no repeats.
/// Grant notices ([`LabelId::is_grant`]) are never replaced: they wait in their
/// own FIFO (two deep, in the order raised) and start before any other pending
/// headline; a grant replaces a pending non-grant headline.
#[derive(Debug, Clone, Default)]
pub struct NoticeQueue {
    active: Option<NoticeView>,
    pending: Option<LabelId>,
    pending_grants: Vec<LabelId>,
    /// Ticks still to wait after the last notice ended before the next starts.
    gap_left: u16,
    tick: u64,
    /// The tick each headline last started on (repeat suppression).
    last_start: Vec<(LabelId, u64)>,
    reduced_motion: bool,
}

impl NoticeQueue {
    pub fn new(reduced_motion: bool) -> Self {
        Self {
            reduced_motion,
            ..Self::default()
        }
    }

    pub fn active(&self) -> Option<NoticeView> {
        self.active
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some() || !self.pending_grants.is_empty()
    }

    fn lifetime(&self) -> u16 {
        if self.reduced_motion {
            NOTICE_REDUCED_TOTAL_TICKS
        } else {
            NOTICE_TOTAL_TICKS
        }
    }

    /// `Restart` clears the notice, the queue and the repeat memory.
    pub fn clear(&mut self) {
        self.active = None;
        self.pending = None;
        self.pending_grants.clear();
        self.gap_left = 0;
        self.last_start.clear();
    }

    /// Offer a headline raised by a real event on this tick.
    pub fn offer(&mut self, headline: LabelId) {
        let repeated = self.last_start.iter().any(|(label, at)| {
            *label == headline && self.tick.saturating_sub(*at) < NOTICE_REPEAT_TICKS
        });
        if repeated || self.pending == Some(headline) || self.pending_grants.contains(&headline) {
            return;
        }
        if self.active.is_none() && self.gap_left == 0 {
            self.start(headline);
        } else if headline.is_grant() {
            // A grant never replaces a grant; it replaces a pending non-grant.
            if self.pending_grants.len() < 2 {
                self.pending_grants.push(headline);
                self.pending = None;
            }
        } else {
            // A newer event replaces the pending one.
            self.pending = Some(headline);
        }
    }

    fn start(&mut self, headline: LabelId) {
        self.active = Some(NoticeView {
            headline,
            age_ticks: 0,
        });
        match self
            .last_start
            .iter_mut()
            .find(|(label, _)| *label == headline)
        {
            Some(entry) => entry.1 = self.tick,
            None => self.last_start.push((headline, self.tick)),
        }
    }

    /// Advance one fixed tick.
    pub fn advance(&mut self) {
        self.tick += 1;
        if let Some(active) = self.active.as_mut() {
            active.age_ticks += 1;
            if active.age_ticks >= self.lifetime() {
                self.active = None;
                self.gap_left = NOTICE_MIN_GAP_TICKS;
            }
        } else if self.gap_left > 0 {
            self.gap_left -= 1;
        }
        if self.active.is_none()
            && self.gap_left == 0
            && let Some(headline) = (!self.pending_grants.is_empty())
                .then(|| self.pending_grants.remove(0))
                .or_else(|| self.pending.take())
        {
            self.start(headline);
        }
    }
}

// --------------------------------------------------------------- the runtime

/// The HUD options the CLI sets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HudConfig {
    pub preview: Option<HudPreview>,
    /// `Some` is the explicit `--glyphs=` override; `None` follows the
    /// last-used gamepad ([`HudRuntime::set_detected_glyphs`]).
    pub glyphs: Option<GlyphSet>,
    /// HUD-DEBT-06: reduced UI motion and high contrast.
    pub settings: HudSettings,
}

/// Reads a capped event log with a cursor, so each event is seen once.
/// `dropped` is the count of events the log already dropped from its front.
#[derive(Debug, Clone, Copy, Default)]
pub struct EventCursor {
    seen: usize,
}

impl EventCursor {
    pub fn next<'a>(&mut self, events: &'a [CombatEvent], dropped: usize) -> &'a [CombatEvent] {
        let from = self.seen.saturating_sub(dropped).min(events.len());
        self.seen = dropped + events.len();
        &events[from..]
    }
}

type VisualKey = (
    u8,
    u8,
    [Option<HpFlash>; 2],
    Option<NoticeView>,
    bool,
    bool,
    PartyView,
    Option<BossView>,
    DonutMagic,
    [Option<AudienceView>; 2],
    (SpecialSlot, SpecialSlot, Option<SocketFlash>),
);

/// The party facts the HUD draws (real ownership, never a fixture): who drives
/// each plate, the glyph family of each plate's own pad, and the controller a
/// disconnect pause is waiting for. Fed by the scene adapter from
/// `crate::pad_slots`; presentation only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartyView {
    pub carl: Control,
    pub donut: Control,
    /// Glyph family of the pad owning each plate (Xbox when none).
    pub carl_glyphs: GlyphSet,
    pub donut_glyphs: GlyphSet,
    /// The slot number (1 or 2) whose controller is missing.
    pub reconnect: Option<u8>,
    /// Q10-01 (`PartyTuning::donut_ability_chips`). When false a human-driven
    /// Donut shows no chips, only the locked special socket.
    pub donut_chips: bool,
}

impl Default for PartyView {
    /// Solo, Carl controlled by P1, Donut the AI companion, Xbox glyphs.
    fn default() -> Self {
        Self {
            carl: Control::Player(1),
            donut: Control::Ai,
            carl_glyphs: GlyphSet::default(),
            donut_glyphs: GlyphSet::default(),
            reconnect: None,
            donut_chips: false,
        }
    }
}

/// The HUD's own clock and cosmetic state: the notice queue, the HP flash and
/// the last health seen. Stepped once per fixed tick by the scene, after the
/// simulation tick; read-only on the simulation.
#[derive(Debug, Clone)]
pub struct HudRuntime {
    config: HudConfig,
    tick: u64,
    revision: u64,
    /// Real party ownership, glyph families and the reconnect request.
    party: PartyView,
    carl_health: u8,
    donut_health: u8,
    /// The HP flash of each hero's plate, `[Carl, Donut]`.
    flash: [Option<HpFlash>; 2],
    notices: NoticeQueue,
    cursor: EventCursor,
    /// Primary/Physical are acted on by the scene (a combat section).
    melee_live: bool,
    /// CP-Q32: what Carl's special is doing, and the label its socket shows.
    special: SpecialPhase,
    special_label: LabelId,
    /// DP-Q33: Donut's special socket and its one-shot flash (presentation only,
    /// fed by the scene from the blink state and the event log).
    donut_special: SpecialSlot,
    donut_flash: Option<SocketFlash>,
    carl_name: String,
    donut_name: String,
    location: Option<String>,
    /// F01-HUD-ROOM: the late-Floor-1 room this run is in (title, TEST LOADOUT),
    /// and the attempt the location text counts.
    room: Option<(String, bool)>,
    attempt: u32,
    /// The real boss encounter (HUD-DEBT-03), `None` outside one.
    boss: Option<BossHud>,
    /// HUD-DEBT-02: Donut's unlocked mana pool as `(mana, mana_max)`, `None`
    /// while the biscuit is locked. Fed by the scene from the simulation's
    /// `DonutMana`; the HUD never invents it.
    donut_mana: Option<(u8, u8)>,
    /// The loaded `empty_press` rule (Q10-16), fed by the scene.
    empty_press: EmptyPress,
    /// The one-time depleted outline flash (presentation only).
    depleted: donut_hud::DepletedFlash,
    /// AUDIENCE-01: the per-hero viewer model. An **observer** like CINE: it
    /// reads this tick's combat events (through this runtime's own cursor, so
    /// no second drain exists), never feeds anything back and is never mixed
    /// into `COMBAT_STATE` or a replay hash. Locked in every Floor 01 path: no
    /// state exists and no step runs until [`Self::unlock_audience`].
    audience: AudienceState,
    /// This tick's plain-data inputs (reused, filled only while unlocked).
    audience_inputs: Vec<AudienceInput>,
    /// CINE feats waiting for the next step.
    pending_feats: Vec<ChainFeat>,
    /// The heroes' x positions (scene units) for authored cues, `[Carl, Donut]`.
    hero_x: [i32; 2],
}

/// The cosmetic state of the boss strip: the slide, the damage chip and the
/// exit delay. Presentation only, never read by the simulation.
#[derive(Debug, Clone)]
struct BossHud {
    name: String,
    emblem: String,
    exit_delay: u16,
    state: Option<BossState>,
    /// The gate closed: the strip slides in.
    shown: bool,
    slide: u8,
    fill: u8,
    chip: u8,
    chip_hold: u8,
    /// Ticks since the boss was defeated.
    defeated: Option<u16>,
}

/// Ticks the damage chip holds before it drains, and its drain per tick.
pub const BOSS_CHIP_HOLD_TICKS: u8 = 30;
pub const BOSS_CHIP_DRAIN_PX: u8 = 2;

impl HudRuntime {
    pub fn new(config: HudConfig) -> Self {
        Self {
            config,
            tick: 0,
            revision: 0,
            party: PartyView::default(),
            carl_health: HERO_HEALTH,
            donut_health: DONUT_HEALTH,
            flash: [None, None],
            notices: NoticeQueue::new(config.settings.reduced_motion),
            cursor: EventCursor::default(),
            melee_live: false,
            special: SpecialPhase::Locked,
            special_label: LabelId::Boom,
            donut_special: SpecialSlot::Locked,
            donut_flash: None,
            carl_name: FALLBACK_NAME_LEFT.to_string(),
            donut_name: FALLBACK_NAME_RIGHT.to_string(),
            location: None,
            room: None,
            attempt: 1,
            boss: None,
            donut_mana: None,
            empty_press: EmptyPress::Dry,
            depleted: donut_hud::DepletedFlash::default(),
            audience: AudienceState::locked(),
            audience_inputs: Vec::new(),
            pending_feats: Vec::new(),
            hero_x: [0; 2],
        }
    }

    /// AUDIENCE-01: the one explicit `locked -> unlocked` flag
    /// (`broadcast_live`), with the authored cue table of this run. v1 has no
    /// in-game trigger (F15): only the QA probe (`--audience-probe`) calls it.
    /// Both heroes start at the baseline.
    pub fn unlock_audience(&mut self, cues: Vec<AnticipationCue>) -> Result<(), CueError> {
        self.audience.set_cues(cues)?;
        self.audience.unlock();
        self.revision += 1;
        Ok(())
    }

    /// The audience observer (read-only; its digest is the `AUDIENCE_STATE`).
    pub fn audience(&self) -> &AudienceState {
        &self.audience
    }

    /// Queue a CINE feat (a mapped `CinematicMoment`) for the next step.
    /// Ignored while locked.
    pub fn push_chain_feat(&mut self, feat: ChainFeat) {
        if self.audience.is_unlocked() {
            self.pending_feats.push(feat);
        }
    }

    /// The heroes' x positions, `[Carl, Donut]`, read by authored anticipation
    /// cues. Call once per tick before [`Self::step`].
    pub fn set_hero_x(&mut self, carl: i32, donut: i32) {
        self.hero_x = [carl, donut];
    }

    /// Each hero's viewer view, `[Carl, Donut]` (`None` while locked).
    fn audience_views(&self) -> [Option<AudienceView>; 2] {
        [HeroId::Carl, HeroId::Donut].map(|hero| self.audience.view(hero))
    }

    /// HUD-DEBT-02: the real mana of Donut's biscuit magic (`None` while it is
    /// locked). Call once per tick before [`Self::step`]; the depleted flash
    /// ages inside `step`.
    pub fn set_donut_mana(&mut self, mana: Option<(u8, u8)>) {
        let before = self.donut_magic();
        self.donut_mana = mana;
        // The renderer rebuilds only on a new revision, and `step` compares
        // against the state it starts from: a visible change is counted here.
        if self.donut_magic() != before {
            self.revision += 1;
        }
    }

    /// The loaded `empty_press` rule, so a depleted plate shows what the press
    /// really does. Call before [`Self::step`]; constant for a run.
    pub fn set_empty_press(&mut self, empty_press: EmptyPress) {
        if self.empty_press != empty_press {
            self.empty_press = empty_press;
            self.revision += 1;
        }
    }

    /// What Donut's magic currently allows, from the real mana.
    fn donut_magic(&self) -> DonutMagic {
        donut_hud::magic_of(self.donut_mana, self.depleted.age())
    }

    /// HUD-DEBT-03: tell the HUD about the encounter (name and emblem key from
    /// the boss data, the exit delay, the state read from the simulation).
    /// Call once per tick before [`Self::step`]; `None` outside an encounter.
    pub fn set_boss(
        &mut self,
        state: Option<BossState>,
        name: &str,
        emblem: &str,
        exit_delay: u16,
    ) {
        let Some(state) = state else {
            self.boss = None;
            return;
        };
        let boss = self.boss.get_or_insert_with(|| BossHud {
            name: name.to_owned(),
            emblem: emblem.to_owned(),
            exit_delay,
            state: None,
            shown: false,
            slide: 0,
            fill: state.fill_px(),
            chip: 0,
            chip_hold: 0,
            defeated: None,
        });
        boss.state = Some(state);
    }

    /// The boss strip as drawn now (`None` before the gate closes and after
    /// the exit slide).
    fn boss_view(&self) -> Option<BossView> {
        let boss = self.boss.as_ref().filter(|b| b.shown)?;
        let state = boss.state.as_ref()?;
        let reduced = self.config.settings.reduced_motion;
        Some(BossView {
            name: boss.name.clone(),
            fill_px: boss.fill,
            chip_px: if reduced { 0 } else { boss.chip },
            warning: state.warning(),
            warning_bright: reduced || (self.tick / WARNING_BLINK_TICKS).is_multiple_of(2),
            slide_tick: if reduced {
                BOSS_SLIDE_TICKS
            } else {
                boss.slide
            },
            emblem: Some(EmblemId(boss.emblem.clone())),
        })
    }

    /// One tick of the boss strip's own timers, after the events were read.
    fn step_boss_hud(&mut self, engaged: bool, defeated: bool, restart: bool) {
        let reduced = self.config.settings.reduced_motion;
        if restart {
            self.boss = None;
            return;
        }
        let Some(boss) = self.boss.as_mut() else {
            return;
        };
        if engaged {
            boss.shown = true;
            boss.slide = 0;
            boss.defeated = None;
        }
        if defeated && boss.defeated.is_none() {
            boss.defeated = Some(0);
        }
        let fill = boss.state.as_ref().map_or(boss.fill, BossState::fill_px);
        if fill < boss.fill {
            boss.chip = boss.chip.saturating_add(boss.fill - fill).min(234 - fill);
            boss.chip_hold = BOSS_CHIP_HOLD_TICKS;
        }
        boss.fill = fill;
        if boss.chip_hold > 0 {
            boss.chip_hold -= 1;
        } else {
            boss.chip = boss.chip.saturating_sub(BOSS_CHIP_DRAIN_PX);
        }
        if !boss.shown {
            return;
        }
        match boss.defeated {
            Some(ticks) if ticks >= boss.exit_delay => {
                // Slide out (12 ticks); gone at once under reduced motion.
                boss.slide = boss.slide.saturating_sub(1);
                if boss.slide == 0 || reduced {
                    boss.shown = false;
                }
            }
            Some(ticks) => boss.defeated = Some(ticks + 1),
            None => {
                if boss.slide < BOSS_SLIDE_TICKS {
                    boss.slide += 1;
                }
            }
        }
    }

    /// Set what the plates and the location strip say, from real data: the
    /// actor manifests' display names and the layout id's floor number.
    pub fn set_identity(&mut self, carl_name: &str, donut_name: &str, layout_id: &str) {
        self.carl_name = carl_name.to_string();
        self.donut_name = donut_name.to_string();
        self.location = location_text(layout_id);
        self.refresh_room_location();
        self.revision += 1;
    }

    /// F01-HUD-ROOM: the run is a late-Floor-1 room: the location strip shows
    /// its title (the manifest's) and the attempt, and `test_loadout` draws the
    /// TEST LOADOUT tag (PT-SCENE U5).
    pub fn set_room(&mut self, title: &str, test_loadout: bool) {
        self.room = Some((title.to_owned(), test_loadout));
        self.refresh_room_location();
        self.revision += 1;
    }

    /// F01-HUD-ROOM: the attempt the location text counts (1 + the restarts of
    /// the room). Bumps the revision only on a change, and only in a room.
    pub fn set_attempt(&mut self, attempt: u32) {
        if self.attempt != attempt {
            self.attempt = attempt;
            if self.room.is_some() {
                self.refresh_room_location();
                self.revision += 1;
            }
        }
    }

    fn refresh_room_location(&mut self) {
        if let Some((title, _)) = &self.room {
            self.location = Some(room_location_text(title, self.attempt));
        }
    }

    /// Record whether the scene acts on Primary/Physical right now (the
    /// story corridor does not). Bumps the revision only on a change.
    /// CP-Q32: record what Carl's special is doing (presentation only; read
    /// from the ledger and the thrower by the scene). Bumps the revision only
    /// on a change.
    pub fn set_special(&mut self, phase: SpecialPhase) {
        if self.special != phase {
            self.special = phase;
            self.revision += 1;
        }
    }

    /// CP-Q32: the socket label from the data text (`cpq32_socket_label`); a
    /// text with no HUD label keeps `BOOM`.
    pub fn set_special_label(&mut self, text: &str) {
        let label = LabelId::for_socket_text(text).unwrap_or(LabelId::Boom);
        if self.special_label != label {
            self.special_label = label;
            self.revision += 1;
        }
    }

    /// DP-Q33: record Donut's special socket (read from the blink state by the
    /// scene, presentation only). Bumps the revision only on a change.
    pub fn set_donut_special(&mut self, slot: SpecialSlot) {
        if self.donut_special != slot {
            self.donut_special = slot;
            self.revision += 1;
        }
    }

    pub fn set_melee_live(&mut self, live: bool) {
        if self.melee_live != live {
            self.melee_live = live;
            self.revision += 1;
        }
    }

    pub fn config(&self) -> HudConfig {
        self.config
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Increments whenever anything the HUD draws changed (health, flash,
    /// notice, a blink phase, a restart). Steady play leaves it alone, so the
    /// renderer rebuilds nothing and allocates nothing.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Everything a step can change on screen.
    fn visual_key(&self) -> VisualKey {
        let blink = !self.config.settings.reduced_motion
            && self.tick % BLINK_PERIOD_TICKS >= BLINK_PERIOD_TICKS - BLINK_TICKS;
        let warning_bright = self.config.settings.reduced_motion
            || (self.tick / WARNING_BLINK_TICKS).is_multiple_of(2)
            || self.config.preview != Some(HudPreview::Boss);
        (
            self.carl_health,
            self.donut_health,
            self.flash,
            self.notices.active(),
            blink,
            warning_bright,
            self.drawn_party(),
            self.boss_view(),
            self.donut_magic(),
            self.audience_views(),
            (
                self.special.slot(self.special_label),
                self.donut_special,
                self.donut_flash,
            ),
        )
    }

    /// The party as drawn now: the explicit `--glyphs=` override wins over the
    /// owning pads' families on both plates.
    fn drawn_party(&self) -> PartyView {
        let mut view = self.party;
        if let Some(forced) = self.config.glyphs {
            view.carl_glyphs = forced;
            view.donut_glyphs = forced;
        }
        view
    }

    /// The glyph family drawn on each plate, `[Carl, Donut]`.
    pub fn plate_glyphs(&self) -> [GlyphSet; 2] {
        let view = self.drawn_party();
        [view.carl_glyphs, view.donut_glyphs]
    }

    /// Record the real party state (presentation only; the simulation never
    /// reads it). Bumps the revision only when something drawn changes, so a
    /// steady party costs nothing per frame.
    pub fn set_party(&mut self, party: PartyView) {
        let before = self.drawn_party();
        self.party = party;
        if self.drawn_party() != before {
            self.revision += 1;
        }
    }

    /// Clear the notice, the queue and the flash (a restart).
    pub fn restart(&mut self) {
        self.boss = None;
        self.notices.clear();
        self.flash = [None, None];
        self.donut_flash = None;
        self.carl_health = HERO_HEALTH;
        self.donut_health = DONUT_HEALTH;
        self.revision += 1;
    }

    /// One fixed tick: age the cosmetic timers, then take the real values and
    /// the newest combat events (`events` is the capped log, `dropped` the
    /// count already dropped from its front).
    pub fn step(
        &mut self,
        carl_health: u8,
        donut_health: u8,
        events: &[CombatEvent],
        dropped: usize,
    ) {
        let before = self.visual_key();
        self.tick += 1;
        self.notices.advance();
        match self.donut_mana {
            Some((mana, _)) => {
                self.depleted.step(mana);
            }
            None => self.depleted = donut_hud::DepletedFlash::default(),
        }
        // DP-Q33: the socket flash ages with the mana outline flash.
        self.donut_flash = self
            .donut_flash
            .map(|f| SocketFlash {
                age_ticks: f.age_ticks.saturating_add(1),
                ..f
            })
            .filter(|f| f.age_ticks < MANA_FLASH_TICKS);
        for slot in &mut self.flash {
            if let Some(flash) = slot.as_mut() {
                flash.age_ticks += 1;
            }
            if slot.is_some_and(|f| f.age_ticks >= HP_FLASH_TICKS) {
                *slot = None;
            }
        }
        let mut previous = [self.carl_health, self.donut_health];
        let (mut engaged, mut boss_down, mut restarted) = (false, false, false);
        let audience_live = self.audience.is_unlocked();
        for event in self.cursor.next(events, dropped) {
            if audience_live && let Some(input) = AudienceInput::from_combat(event) {
                self.audience_inputs.push(input);
            }
            match *event {
                CombatEvent::BossEngaged { .. } => engaged = true,
                CombatEvent::EnemyDefeated { target, .. } if target == BOSS_ENEMY_ID => {
                    boss_down = true;
                }
                CombatEvent::Restart { .. } => restarted = true,
                _ => {}
            }
            match *event {
                CombatEvent::HeroHurt {
                    health_left, hero, ..
                } => {
                    let index = hero.index();
                    // A guarded hit that took nothing (the solo last-HP guard)
                    // shows no loss.
                    if health_left < previous[index] {
                        self.flash[index] = Some(HpFlash {
                            lost_from: previous[index].max(health_left.saturating_add(1)),
                            lost_to: health_left,
                            age_ticks: 0,
                        });
                        previous[index] = health_left;
                    }
                }
                CombatEvent::Restart { .. } => {
                    self.notices.clear();
                    self.flash = [None, None];
                    self.donut_flash = None;
                }
                // DP-Q33: the socket flashes once on ready, and on a refusal or
                // a fizzle for want of mana or a destination (the dry-press cue).
                CombatEvent::SpecialReady { hero: 1, .. } => {
                    self.donut_flash = Some(SocketFlash {
                        kind: SocketFlashKind::Ready,
                        age_ticks: 0,
                    });
                }
                CombatEvent::SpecialRejected {
                    hero: 1,
                    reason:
                        crate::thrown_charge::SpecialReject::NoMana
                        | crate::thrown_charge::SpecialReject::NoDestination,
                    ..
                }
                | CombatEvent::BlinkFizzled { hero: 1, .. } => {
                    self.donut_flash = Some(SocketFlash {
                        kind: SocketFlashKind::Refused,
                        age_ticks: 0,
                    });
                }
                other => {
                    if let Some(headline) = notice_for_event(&other) {
                        self.notices.offer(headline);
                    }
                }
            }
        }
        if carl_health > self.carl_health {
            // Healed or restarted without an event: no stale flash.
            self.flash[0] = None;
        }
        if donut_health > self.donut_health {
            self.flash[1] = None;
        }
        self.carl_health = carl_health;
        self.donut_health = donut_health;
        self.step_boss_hud(engaged, boss_down, restarted);
        self.step_audience();
        if self.visual_key() != before {
            self.revision += 1;
        }
    }

    /// One fixed step of the audience observer (a no-op while locked).
    fn step_audience(&mut self) {
        if !self.audience.is_unlocked() {
            return;
        }
        for feat in self.pending_feats.drain(..) {
            self.audience_inputs.push(AudienceInput::Chain(feat));
        }
        for (hero, x) in [HeroId::Carl, HeroId::Donut].into_iter().zip(self.hero_x) {
            self.audience_inputs.push(AudienceInput::HeroX { hero, x });
        }
        self.audience.step(&self.audience_inputs);
        self.audience_inputs.clear();
    }

    /// The real inputs for this tick.
    pub fn real_inputs(&self) -> RealHudInputs {
        let mut real = RealHudInputs::normal(self.carl_health, self.donut_health);
        real.melee_live = self.melee_live;
        real.carl_special = self.special.slot(self.special_label);
        real.donut_special = self.donut_special;
        real.donut_socket_flash = self.donut_flash;
        real.carl_name.clone_from(&self.carl_name);
        real.donut_name.clone_from(&self.donut_name);
        real.location.clone_from(&self.location);
        real.loadout_tag = self.room.as_ref().is_some_and(|(_, tag)| *tag);
        real.hp_flash = self.flash[0];
        real.donut_hp_flash = self.flash[1];
        real.notice = self.notices.active();
        let view = self.drawn_party();
        real.carl_control = view.carl;
        real.donut_control = view.donut;
        real.carl_glyphs = view.carl_glyphs;
        real.donut_glyphs = view.donut_glyphs;
        real.reconnect = view.reconnect;
        real.donut_chips = view.donut_chips;
        real.reduced_motion = self.config.settings.reduced_motion;
        real.high_contrast = self.config.settings.high_contrast;
        real.boss = self.boss_view();
        real.donut_magic = self.donut_magic();
        real.empty_press = self.empty_press;
        real.audience = self.audience_views();
        real
    }

    pub fn snapshot(&self) -> HudSnapshot {
        build_snapshot(&self.real_inputs(), self.config.preview, self.tick)
    }
}

// -------------------------------------------------------------------- layout

/// The HUD rects of spec section 1 for one canvas. Virtual pixels, origin top
/// left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudLayout {
    pub view_h: i32,
    /// Screen-space walk line (layout walk line minus the view top).
    pub walk_y: i32,
    pub carl_plate: IRect,
    pub donut_plate: IRect,
    pub face: IRect,
    pub notice: IRect,
}

impl HudLayout {
    pub fn new(view_h: i32, walk_y: i32) -> Self {
        Self {
            view_h,
            walk_y,
            carl_plate: IRect {
                x: SAFE_MARGIN,
                y: SAFE_MARGIN,
                w: PLATE_W,
                h: PLATE_H,
            },
            donut_plate: IRect {
                x: VIEW_WIDTH - SAFE_MARGIN - PLATE_W,
                y: SAFE_MARGIN,
                w: PLATE_W,
                h: PLATE_H,
            },
            face: IRect {
                x: 612,
                y: view_h - 26,
                w: 20,
                h: 18,
            },
            notice: IRect {
                x: 404,
                y: view_h - 40,
                w: 204,
                h: 32,
            },
        }
    }

    /// The location strip: centred, `max(96, text + 28)` wide, at most 280.
    pub fn location(&self, text_w: i32) -> IRect {
        let w = (text_w + 28).clamp(96, 280);
        IRect {
            x: VIEW_WIDTH / 2 - w / 2,
            y: SAFE_MARGIN,
            w,
            h: 16,
        }
    }

    /// F01-HUD-ROOM: the TEST LOADOUT tag, centred 2 px under the location
    /// strip, or under the boss strip's resting position when that replaces it.
    pub fn loadout_tag(&self, text_w: i32, below_boss: bool) -> IRect {
        let w = text_w + 10;
        IRect {
            x: VIEW_WIDTH / 2 - w / 2,
            y: SAFE_MARGIN + if below_boss { 26 } else { 16 } + 2,
            w,
            h: 11,
        }
    }

    /// The boss strip at its resting position.
    pub fn boss(&self) -> IRect {
        IRect {
            x: 180,
            y: SAFE_MARGIN,
            w: 280,
            h: 26,
        }
    }

    /// The PREVIEW tag for a text of `text_w` px.
    pub fn preview_tag(&self, text_w: i32) -> IRect {
        IRect {
            x: SAFE_MARGIN,
            y: self.view_h - 19,
            w: text_w + 10,
            h: 11,
        }
    }

    /// Top y of the boss strip `slide_tick` ticks after it started to slide in.
    pub fn boss_y(slide_tick: u8, reduced_motion: bool) -> i32 {
        if reduced_motion {
            return SAFE_MARGIN;
        }
        (-26 + i32::from(slide_tick) * BOSS_SLIDE_PX_PER_TICK).min(SAFE_MARGIN)
    }
}

pub fn rects_intersect(a: IRect, b: IRect) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

// ----------------------------------------------------------------- text math

/// The two HUD fonts, by role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudFont {
    /// Jersey 10 at 10 px caps: names, location, headline, boss name.
    Jersey,
    /// Silkscreen Regular at 5 px caps: chip labels, `COMPANION`.
    SilkRegular,
    /// Silkscreen Bold at 5 px caps: tags, `SYSTEM NOTICE`, PREVIEW.
    SilkBold,
}

/// Advance of one character in whole pixels at the HUD grid sizes (Jersey 10
/// at 1/75 em per px, Silkscreen at 1/125 em per px), read from the fonts'
/// `hmtx` tables (every value is an exact integer at these sizes). Characters
/// outside the HUD set take the width of `M`.
pub fn advance_px(font: HudFont, ch: char) -> i32 {
    // (Jersey 10, Silkscreen Regular, Silkscreen Bold)
    let (jersey, regular, bold) = match ch {
        'A' => (8, 6, 7),
        'B' => (8, 6, 7),
        'C' => (8, 6, 7),
        'D' => (8, 6, 7),
        'E' => (7, 5, 6),
        'F' => (7, 5, 6),
        'G' => (8, 6, 7),
        'H' => (8, 6, 7),
        'I' => (3, 3, 4),
        'J' => (7, 6, 7),
        'K' => (7, 6, 7),
        'L' => (6, 5, 6),
        'M' => (10, 7, 8),
        'N' => (9, 7, 8),
        'O' => (8, 6, 7),
        'P' => (8, 6, 7),
        'Q' => (8, 6, 7),
        'R' => (8, 6, 7),
        'S' => (8, 6, 7),
        'T' => (7, 5, 6),
        'U' => (8, 6, 7),
        'V' => (8, 7, 8),
        'W' => (11, 7, 8),
        'X' => (8, 7, 8),
        'Y' => (9, 7, 8),
        'Z' => (9, 5, 6),
        '0' => (7, 6, 7),
        '1' => (4, 5, 6),
        '2' => (8, 6, 7),
        '3' => (8, 6, 7),
        '4' => (7, 6, 7),
        '5' => (7, 6, 7),
        '6' => (8, 6, 7),
        '7' => (7, 6, 7),
        '8' => (8, 6, 7),
        '9' => (8, 6, 7),
        ' ' => (4, 4, 4),
        '!' => (3, 3, 4),
        ':' => (3, 3, 4),
        '.' => (3, 3, 4),
        '-' => (7, 5, 6),
        '/' => (6, 5, 6),
        '+' => (7, 7, 8),
        '\u{b7}' => (3, 3, 4),
        _ => (10, 7, 8),
    };
    match font {
        HudFont::Jersey => jersey,
        HudFont::SilkRegular => regular,
        HudFont::SilkBold => bold,
    }
}

/// Width in whole pixels of `text` on one line.
pub fn text_width_px(font: HudFont, text: &str) -> i32 {
    text.chars().map(|c| advance_px(font, c)).sum()
}

// --------------------------------------------------------------------- tests

/// Test inputs with the fan-layer names and location filled in, as the
/// scene supplies them from the manifests and the layout id.
#[cfg(test)]
pub(crate) fn test_inputs(carl_health: u8, donut_health: u8) -> RealHudInputs {
    let mut real = RealHudInputs::normal(carl_health, donut_health);
    real.carl_name = "CARL".to_string();
    real.donut_name = "DONUT".to_string();
    real.location = location_text("floor01_test");
    real
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::DefeatCause;

    fn defeat(force: ForceTier) -> CombatEvent {
        CombatEvent::EnemyDefeated {
            tick: 1,
            target: 0,
            cause: DefeatCause::Melee,
            force,
            instigator: Some(0),
        }
    }

    fn hurt(health_left: u8) -> CombatEvent {
        CombatEvent::HeroHurt {
            tick: 1,
            source: 0,
            health_left,
            hero: crate::party::HeroId::Carl,
        }
    }

    fn snap(preview: HudPreview) -> HudSnapshot {
        build_snapshot(&test_inputs(4, 4), Some(preview), 0)
    }

    #[test]
    fn normal_play_is_carl_p1_donut_ai_full_health_no_mana() {
        let s = build_snapshot(&test_inputs(4, 4), None, 0);
        assert_eq!(s.preview, None);
        assert_eq!(s.plates[0].control, Control::Player(1));
        assert_eq!(s.plates[1].control, Control::Ai);
        assert_eq!(s.plates[0].hp, Meter { cur: 4, max: 4 });
        assert_eq!(s.plates[1].hp, Meter { cur: 4, max: 4 });
        assert!(s.plates.iter().all(|p| p.mana.is_none()));
        assert_eq!(s.plates[1].abilities, AbilityRow::AiCompanion);
        assert_eq!(s.location.as_deref(), Some("FLOOR 01"));
        assert!(s.boss.is_none() && s.notice.is_none());
        // Live Carl HP flows through.
        let s = build_snapshot(&test_inputs(1, 4), None, 0);
        assert_eq!(s.plates[0].hp.cur, 1);
    }

    #[test]
    fn fixture_table_values() {
        let s = snap(HudPreview::PreUnlock);
        assert_eq!(s.plates[0].control, Control::Player(1));
        assert_eq!(s.plates[1].control, Control::Player(2));
        assert_eq!(s.plates[1].hp.cur, 4);
        assert_eq!(s.plates[1].mana, None);
        assert_label_pair(&s.plates[1], LabelId::Swat, LabelId::Pounce);
        assert_label_pair(&s.plates[0], LabelId::Punch, LabelId::Kick);

        let s = snap(HudPreview::PostUnlock);
        assert_eq!(s.plates[1].mana.unwrap().fill_px, 94);
        assert_label_pair(&s.plates[1], LabelId::Missile, LabelId::Pounce);

        let s = snap(HudPreview::Depleted);
        assert_eq!(s.plates[0].hp.cur, 2);
        assert_eq!(
            s.plates[0].hp_flash,
            Some(HpFlash {
                lost_from: 3,
                lost_to: 2,
                age_ticks: 0
            })
        );
        let mana = s.plates[1].mana.unwrap();
        assert_eq!((mana.fill_px, mana.depleted_age), (0, Some(0)));
        assert_eq!(s.plates[1].hp.cur, 4);
        let AbilityRow::Chips { a, b, .. } = s.plates[1].abilities else {
            panic!("a player-controlled Donut has chips");
        };
        assert!(a.muted && a.label == LabelId::Missile);
        assert!(!b.muted);
        assert_eq!(
            s.notice,
            Some(NoticeView {
                headline: LabelId::Launched,
                age_ticks: 20
            })
        );

        let s = snap(HudPreview::Boss);
        assert_eq!(s.plates[0].hp.cur, 3);
        assert_eq!(s.plates[1].control, Control::Ai);
        assert_eq!(s.plates[1].mana.unwrap().fill_px, 60);
        assert_eq!(s.plates[1].abilities, AbilityRow::AiCompanion);
        let boss = s.boss.as_ref().unwrap();
        assert_eq!(boss.name, "TEST BRUTE");
        assert_eq!((boss.fill_px, boss.chip_px, boss.warning), (164, 14, true));
        assert_eq!(s.notice.unwrap().headline, LabelId::BossIncoming);
        assert_eq!(s.location, None);

        let s = snap(HudPreview::Coop);
        assert_eq!(s.plates[0].hp.cur, 3);
        assert_eq!(s.plates[1].control, Control::Player(2));
        assert_eq!(s.plates[1].hp.cur, 2);
        assert_eq!(s.plates[1].mana.unwrap().fill_px, 31);
        assert_label_pair(&s.plates[1], LabelId::Missile, LabelId::Pounce);
        assert_eq!(s.location.as_deref(), Some("FLOOR 01"));
        assert!(s.boss.is_none());
    }

    fn assert_label_pair(plate: &PlateView, a: LabelId, b: LabelId) {
        let AbilityRow::Chips {
            a: chip_a,
            b: chip_b,
            special,
        } = plate.abilities
        else {
            panic!("expected chips");
        };
        assert_eq!((chip_a.label, chip_b.label), (a, b));
        assert_eq!(special, SpecialSlot::Locked);
    }

    #[test]
    fn every_fixture_shows_the_preview_tag_and_no_fixture_exists_without_one() {
        for preview in HudPreview::ALL {
            assert_eq!(snap(preview).preview, Some(preview));
            assert_eq!(HudPreview::parse(preview.name()), Some(preview));
        }
        assert_eq!(HudPreview::parse("nope"), None);
        assert_eq!(build_snapshot(&test_inputs(4, 4), None, 0).preview, None);
    }

    #[test]
    fn pre_unlock_has_no_mana_and_carl_never_has_mana() {
        assert!(snap(HudPreview::PreUnlock).plates[1].mana.is_none());
        // Real, un-flagged play: no mana, whatever the tick.
        for tick in [0, 100, 239, 5000] {
            let s = build_snapshot(&test_inputs(4, 4), None, tick);
            assert!(s.plates[1].mana.is_none());
        }
        for preview in HudPreview::ALL {
            assert!(snap(preview).plates[0].mana.is_none(), "{preview:?}");
        }
        // Even a fully unlocked Donut gives Carl none.
        let mut real = test_inputs(4, 4);
        real.donut_magic = mana(94);
        let s = build_snapshot(&real, None, 0);
        assert!(s.plates[0].mana.is_none());
        assert!(s.plates[1].mana.is_some());
    }

    #[test]
    fn a_locked_special_is_never_usable_and_no_b_or_circle_is_offered() {
        for glyphs in [GlyphSet::Xbox, GlyphSet::DualSense] {
            for action in [LogicalAction::Primary, LogicalAction::Physical] {
                assert!(!matches!(glyphs.glyph(action), Glyph::B | Glyph::Circle));
                assert!(!matches!(glyphs.glyph(action), Glyph::A | Glyph::Cross));
            }
        }
        for preview in HudPreview::ALL {
            for plate in &snap(preview).plates {
                if let AbilityRow::Chips { special, .. } = plate.abilities {
                    assert_eq!(special, SpecialSlot::Locked);
                }
            }
        }
        let s = build_snapshot(&test_inputs(4, 4), None, 0);
        let AbilityRow::Chips { special, .. } = s.plates[0].abilities else {
            panic!("Carl is player controlled");
        };
        assert_eq!(special, SpecialSlot::Locked);
    }

    #[test]
    fn glyph_sets_map_the_primary_and_physical_actions() {
        assert_eq!(GlyphSet::Xbox.glyph(LogicalAction::Primary), Glyph::X);
        assert_eq!(GlyphSet::Xbox.glyph(LogicalAction::Physical), Glyph::Y);
        assert_eq!(
            GlyphSet::DualSense.glyph(LogicalAction::Primary),
            Glyph::Square
        );
        assert_eq!(
            GlyphSet::DualSense.glyph(LogicalAction::Physical),
            Glyph::Triangle
        );
        assert_eq!(GlyphSet::default(), GlyphSet::Xbox);
        assert_eq!(GlyphSet::parse("dualsense"), Some(GlyphSet::DualSense));
        assert_eq!(GlyphSet::parse("ps5"), None);
    }

    #[test]
    fn vendor_ids_pick_the_glyph_family() {
        assert_eq!(SONY_VENDOR_ID, 0x054C);
        // Sony (DualShock 4 / DualSense / Edge): PlayStation glyphs.
        assert_eq!(GlyphSet::for_vendor_id(Some(0x054C)), GlyphSet::DualSense);
        // Microsoft, Valve (Steam Deck), Nintendo, 8BitDo, Logitech, unknown
        // and missing ids: Xbox glyphs.
        for vendor in [0x045E, 0x28DE, 0x057E, 0x2DC8, 0x046D, 0x0000, 0xFFFF] {
            assert_eq!(GlyphSet::for_vendor_id(Some(vendor)), GlyphSet::Xbox);
        }
        assert_eq!(GlyphSet::for_vendor_id(None), GlyphSet::Xbox);
    }

    #[test]
    fn plate_glyphs_follow_each_plates_own_pad_unless_overridden() {
        let mut hud = HudRuntime::new(HudConfig::default());
        assert_eq!(hud.plate_glyphs(), [GlyphSet::Xbox; 2]);
        let start = hud.revision();
        let mut party = PartyView {
            donut_glyphs: GlyphSet::DualSense,
            ..PartyView::default()
        };
        hud.set_party(party);
        let snapshot = hud.snapshot();
        assert_eq!(snapshot.plates[0].glyphs, GlyphSet::Xbox);
        assert_eq!(snapshot.plates[1].glyphs, GlyphSet::DualSense);
        assert_eq!(hud.plate_glyphs(), [GlyphSet::Xbox, GlyphSet::DualSense]);
        assert_eq!(hud.revision(), start + 1);
        // The same party again: nothing to redraw.
        hud.set_party(party);
        assert_eq!(hud.revision(), start + 1);
        party.carl_glyphs = GlyphSet::DualSense;
        hud.set_party(party);
        assert_eq!(hud.revision(), start + 2);

        // An explicit override wins on both plates and never redraws for it.
        let mut forced = HudRuntime::new(HudConfig {
            glyphs: Some(GlyphSet::Xbox),
            ..HudConfig::default()
        });
        let start = forced.revision();
        forced.set_party(party);
        assert_eq!(forced.plate_glyphs(), [GlyphSet::Xbox; 2]);
        assert_eq!(forced.revision(), start);
    }

    #[test]
    fn real_ownership_is_drawn_not_faked() {
        let mut hud = HudRuntime::new(HudConfig::default());
        let s = hud.snapshot();
        assert_eq!(s.plates[0].control, Control::Player(1));
        assert_eq!(s.plates[1].control, Control::Ai);
        // Solo swap: the P1 tag moves to Donut, Carl becomes the AI.
        hud.set_party(PartyView {
            carl: Control::Ai,
            donut: Control::Player(1),
            ..PartyView::default()
        });
        let s = hud.snapshot();
        assert_eq!(s.plates[0].control, Control::Ai);
        assert_eq!(s.plates[1].control, Control::Player(1));
        assert_eq!(s.plates[0].abilities, AbilityRow::AiCompanion);
        // Two players.
        hud.set_party(PartyView {
            carl: Control::Player(1),
            donut: Control::Player(2),
            ..PartyView::default()
        });
        let s = hud.snapshot();
        assert_eq!(s.plates[0].control, Control::Player(1));
        assert_eq!(s.plates[1].control, Control::Player(2));
    }

    #[test]
    fn a_human_donut_shows_no_chips_until_q10_is_answered() {
        let mut hud = HudRuntime::new(HudConfig::default());
        hud.set_melee_live(true);
        let two = PartyView {
            carl: Control::Player(1),
            donut: Control::Player(2),
            ..PartyView::default()
        };
        hud.set_party(two);
        let s = hud.snapshot();
        assert!(matches!(s.plates[0].abilities, AbilityRow::Chips { .. }));
        assert!(matches!(
            s.plates[1].abilities,
            AbilityRow::NoActions { .. }
        ));
        hud.set_party(PartyView {
            donut_chips: true,
            ..two
        });
        assert!(matches!(
            hud.snapshot().plates[1].abilities,
            AbilityRow::Chips { .. }
        ));
    }

    #[test]
    fn a_reconnect_request_is_real_and_lights_the_system_face() {
        let mut hud = HudRuntime::new(HudConfig::default());
        assert_eq!(hud.snapshot().reconnect, None);
        let start = hud.revision();
        hud.set_party(PartyView {
            reconnect: Some(2),
            ..PartyView::default()
        });
        let s = hud.snapshot();
        assert_eq!(s.reconnect, Some(2));
        assert_eq!(s.face, FaceFrame::Notice);
        assert_eq!(hud.revision(), start + 1);
        hud.set_party(PartyView::default());
        assert_eq!(hud.snapshot().reconnect, None);
    }

    #[test]
    fn a_fixture_keeps_the_real_plate_glyphs() {
        let mut hud = HudRuntime::new(HudConfig {
            preview: Some(HudPreview::Coop),
            ..HudConfig::default()
        });
        hud.set_party(PartyView {
            donut_glyphs: GlyphSet::DualSense,
            ..PartyView::default()
        });
        let s = hud.snapshot();
        assert_eq!(s.plates[1].control, Control::Player(2));
        assert_eq!(s.plates[1].glyphs, GlyphSet::DualSense);
    }

    #[test]
    fn only_heavy_and_catastrophic_defeats_raise_a_notice() {
        assert_eq!(notice_for_event(&defeat(ForceTier::Routine)), None);
        assert_eq!(
            notice_for_event(&defeat(ForceTier::Heavy)),
            Some(LabelId::Launched)
        );
        assert_eq!(
            notice_for_event(&defeat(ForceTier::Catastrophic)),
            Some(LabelId::Flattened)
        );
        for event in [
            CombatEvent::GoblinTell { tick: 1, goblin: 0 },
            CombatEvent::GoblinSwing { tick: 1, goblin: 0 },
            CombatEvent::GoblinInterrupted { tick: 1, goblin: 0 },
            hurt(2),
            CombatEvent::HeroDefeated {
                tick: 1,
                hero: crate::party::HeroId::Carl,
            },
        ] {
            assert_eq!(notice_for_event(&event), None, "{event:?}");
        }
    }

    #[test]
    fn a_notice_has_a_finite_lifetime_of_136_ticks() {
        let mut queue = NoticeQueue::new(false);
        queue.offer(LabelId::Launched);
        assert_eq!(queue.active().unwrap().age_ticks, 0);
        let mut visible = 1;
        for _ in 0..400 {
            queue.advance();
            if queue.active().is_some() {
                visible += 1;
            }
        }
        assert_eq!(visible, 136);
        assert_eq!(NOTICE_TOTAL_TICKS, 136);
        assert!(queue.active().is_none());
    }

    #[test]
    fn reduced_motion_notices_last_the_hold_only_and_never_slide() {
        let mut queue = NoticeQueue::new(true);
        queue.offer(LabelId::Flattened);
        let mut visible = 1;
        for _ in 0..400 {
            queue.advance();
            if queue.active().is_some() {
                visible += 1;
            }
        }
        assert_eq!(visible, 120);
        assert!((0..200).all(|age| notice_dx(age, true) == 0));
    }

    #[test]
    fn notice_slide_offsets() {
        assert_eq!(notice_dx(0, false), 24);
        assert_eq!(notice_dx(7, false), 3);
        assert_eq!(notice_dx(8, false), 0);
        assert_eq!(notice_dx(127, false), 0);
        assert_eq!(notice_dx(128, false), 3);
        assert_eq!(notice_dx(135, false), 24);
    }

    #[test]
    fn queue_keeps_one_pending_a_gap_and_suppresses_repeats() {
        let mut queue = NoticeQueue::new(false);
        queue.offer(LabelId::Launched);
        // Newer events replace the single pending one.
        queue.offer(LabelId::Flattened);
        assert!(queue.has_pending());
        queue.offer(LabelId::BossIncoming);
        // The same headline cannot repeat within 300 ticks.
        queue.offer(LabelId::Launched);
        let mut ended_at = None;
        let mut next_start = None;
        for t in 1..600 {
            queue.advance();
            match queue.active() {
                None if ended_at.is_none() => ended_at = Some(t),
                Some(n) if ended_at.is_some() && next_start.is_none() => {
                    assert_eq!(n.headline, LabelId::BossIncoming);
                    next_start = Some(t);
                }
                _ => {}
            }
        }
        let (ended, started) = (ended_at.unwrap(), next_start.unwrap());
        assert!(started - ended >= i32::from(NOTICE_MIN_GAP_TICKS));
        // Launched started on tick 0; after 300 ticks it may show again.
        let mut queue = NoticeQueue::new(false);
        queue.offer(LabelId::Launched);
        for _ in 0..299 {
            queue.advance();
        }
        queue.offer(LabelId::Launched);
        assert!(!queue.has_pending());
        queue.advance();
        queue.offer(LabelId::Launched);
        assert!(queue.has_pending() || queue.active().is_some());
    }

    /// DP-Q12: a grant notice is never replaced by a later headline of the same
    /// tick, and a second grant waits behind the first in the order raised; a
    /// grant replaces a pending non-grant.
    #[test]
    fn a_grant_notice_is_never_replaced_and_a_second_grant_waits_its_turn() {
        let shown = |offers: &[LabelId]| {
            let mut queue = NoticeQueue::new(false);
            queue.offer(LabelId::MainEvent);
            for o in offers {
                queue.offer(*o);
            }
            let mut order = Vec::new();
            for _ in 0..900 {
                queue.advance();
                if let Some(n) = queue.active()
                    && n.age_ticks == 0
                {
                    order.push(n.headline);
                }
            }
            order
        };
        use LabelId::{Flattened, HeavyFist, Knockout, PuddleJumper};
        // Both grants after a busy tick: Carl's first, then Donut's.
        assert_eq!(
            shown(&[Flattened, Knockout, HeavyFist, PuddleJumper]),
            [HeavyFist, PuddleJumper]
        );
        // A later non-grant of the same tick never replaces a pending grant.
        assert_eq!(shown(&[HeavyFist, Knockout]), [HeavyFist, Knockout]);
        // Single grants are unchanged (the newest non-grant before it is replaced).
        assert_eq!(shown(&[Knockout, HeavyFist]), [HeavyFist]);
        assert_eq!(shown(&[PuddleJumper]), [PuddleJumper]);
    }

    #[test]
    fn restart_clears_the_notice_and_the_queue() {
        let mut queue = NoticeQueue::new(false);
        queue.offer(LabelId::Launched);
        queue.offer(LabelId::Flattened);
        queue.clear();
        assert!(queue.active().is_none() && !queue.has_pending());
        // And it can start right away again (no gap, no repeat memory).
        queue.offer(LabelId::Launched);
        assert!(queue.active().is_some());
    }

    #[test]
    fn real_events_feed_notices_and_the_hp_flash() {
        let mut hud = HudRuntime::new(HudConfig::default());
        let log = vec![hurt(3), defeat(ForceTier::Heavy)];
        hud.step(3, 4, &log, 0);
        let s = hud.snapshot();
        assert_eq!(s.plates[0].hp.cur, 3);
        assert_eq!(
            s.plates[0].hp_flash,
            Some(HpFlash {
                lost_from: 4,
                lost_to: 3,
                age_ticks: 0
            })
        );
        assert_eq!(s.notice.unwrap().headline, LabelId::Launched);
        assert_eq!(s.face, FaceFrame::Notice);
        // Events are read once: stepping again with the same log adds nothing.
        for _ in 0..30 {
            hud.step(3, 4, &log, 0);
        }
        let s = hud.snapshot();
        assert_eq!(s.plates[0].hp_flash, None, "the flash ends after 24 ticks");
        assert_eq!(s.notice.unwrap().age_ticks, 30);
        // The notice is gone after its lifetime.
        for _ in 0..200 {
            hud.step(3, 4, &log, 0);
        }
        assert!(hud.snapshot().notice.is_none());
        assert_eq!(hud.snapshot().face, FaceFrame::Idle);
    }

    #[test]
    fn donuts_real_hurt_event_flashes_her_plate_and_never_carls() {
        use crate::party::HeroId;
        let mut hud = HudRuntime::new(HudConfig::default());
        let donut_hurt = |health_left| CombatEvent::HeroHurt {
            tick: 3,
            source: 0,
            health_left,
            hero: HeroId::Donut,
        };
        let log = vec![donut_hurt(3)];
        hud.step(4, 3, &log, 0);
        let s = hud.snapshot();
        assert_eq!(s.plates[1].hp.cur, 3);
        assert_eq!(
            s.plates[1].hp_flash,
            Some(HpFlash {
                lost_from: 4,
                lost_to: 3,
                age_ticks: 0
            })
        );
        assert_eq!(s.plates[0].hp_flash, None, "Carl's plate stays calm");
        for _ in 0..HP_FLASH_TICKS {
            hud.step(4, 3, &log, 0);
        }
        assert_eq!(hud.snapshot().plates[1].hp_flash, None);
        // A hit the solo guard absorbed (health stays at 1) shows no new loss.
        let mut hud = HudRuntime::new(HudConfig::default());
        let log = vec![donut_hurt(1)];
        hud.step(4, 1, &log, 0);
        assert!(
            hud.snapshot().plates[1].hp_flash.is_some(),
            "4 -> 1 is a loss"
        );
        let mut log = log;
        log.push(donut_hurt(1));
        for _ in 0..HP_FLASH_TICKS {
            hud.step(4, 1, &log, 0);
        }
        assert_eq!(
            hud.snapshot().plates[1].hp_flash,
            None,
            "1 -> 1: nothing lost"
        );
    }

    #[test]
    fn a_restart_event_clears_everything() {
        let mut hud = HudRuntime::new(HudConfig::default());
        let mut log = vec![hurt(3), defeat(ForceTier::Catastrophic)];
        hud.step(3, 4, &log, 0);
        log.push(CombatEvent::Restart { tick: 5, deaths: 1 });
        hud.step(4, 4, &log, 0);
        let s = hud.snapshot();
        assert!(s.notice.is_none() && s.plates[0].hp_flash.is_none());
        assert_eq!(s.plates[0].hp.cur, 4);
        assert_eq!(s.face, FaceFrame::Idle);
    }

    #[test]
    fn the_event_cursor_survives_a_capped_log() {
        let mut cursor = EventCursor::default();
        let a = vec![hurt(3), hurt(2)];
        assert_eq!(cursor.next(&a, 0).len(), 2);
        assert_eq!(cursor.next(&a, 0).len(), 0);
        // The log dropped its first event and gained one.
        let b = vec![hurt(2), hurt(1)];
        assert_eq!(cursor.next(&b, 1), &b[1..]);
        // Many events were dropped unread: the surviving one is new, and the
        // dropped ones are lost, never replayed.
        let c = vec![hurt(1)];
        assert_eq!(cursor.next(&c, 10).len(), 1);
        assert_eq!(cursor.next(&c, 10).len(), 0);
    }

    #[test]
    fn the_face_blinks_for_six_ticks_every_240_unless_reduced() {
        let real = test_inputs(4, 4);
        let blinks = (0..480)
            .filter(|t| build_snapshot(&real, None, *t).face == FaceFrame::Blink)
            .count();
        assert_eq!(blinks, 12);
        assert_eq!(build_snapshot(&real, None, 0).face, FaceFrame::Idle);
        assert_eq!(build_snapshot(&real, None, 239).face, FaceFrame::Blink);
        let mut reduced = real.clone();
        reduced.reduced_motion = true;
        assert!((0..480).all(|t| build_snapshot(&reduced, None, t).face == FaceFrame::Idle));
    }

    #[test]
    fn steady_play_leaves_the_revision_alone() {
        let mut hud = HudRuntime::new(HudConfig::default());
        let start = hud.revision();
        for _ in 0..1000 {
            hud.step(4, 4, &[], 0);
        }
        // Only the blink edges (start and end of each 6-tick blink) change it.
        assert_eq!(hud.revision() - start, 8);
        let mut reduced = HudRuntime::new(HudConfig {
            settings: HudSettings {
                reduced_motion: true,
                ..HudSettings::default()
            },
            ..HudConfig::default()
        });
        for _ in 0..1000 {
            reduced.step(4, 4, &[], 0);
        }
        assert_eq!(reduced.revision(), 0);
    }

    #[test]
    fn fixtures_override_only_hud_fields() {
        // The runtime holds no simulation reference: stepping it with the same
        // real values yields the same real inputs with or without a fixture.
        let mut plain = HudRuntime::new(HudConfig::default());
        let mut fixture = HudRuntime::new(HudConfig {
            preview: Some(HudPreview::Boss),
            ..HudConfig::default()
        });
        for _ in 0..50 {
            plain.step(4, 4, &[], 0);
            fixture.step(4, 4, &[], 0);
        }
        assert_eq!(plain.real_inputs(), fixture.real_inputs());
        assert_eq!(plain.tick(), fixture.tick());
        assert!(fixture.snapshot().preview.is_some() && plain.snapshot().preview.is_none());
    }

    #[test]
    fn layout_rects_respect_the_margins_the_lane_and_each_other() {
        use crate::env_kit::EnvKit;
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let kit = EnvKit::load(&root, "assets/levels/floor01/corridor_abc_sample.json").unwrap();
        for (view_name, view_h, expected_walk) in
            [("abc_640x360", 360, 288), ("abc_640x400", 400, 308)]
        {
            let view = kit.view(view_name).expect("layout view");
            let walk_y = kit.base_walk_line_y - view.y;
            assert_eq!(walk_y, expected_walk);
            let layout = HudLayout::new(view_h, walk_y);
            let location = layout.location(text_width_px(HudFont::Jersey, "FLOOR 01"));
            let wide = layout.location(1000);
            let boss = layout.boss();
            let widest_preview = layout.preview_tag(
                HudPreview::ALL
                    .iter()
                    .map(|p| text_width_px(HudFont::SilkBold, &format!("PREVIEW {}", p.tag_name())))
                    .max()
                    .unwrap(),
            );
            let top = [layout.carl_plate, layout.donut_plate, location, boss];
            let bottom = [layout.face, layout.notice, widest_preview];
            let all = [top.as_slice(), bottom.as_slice(), &[wide]].concat();
            let inside = IRect {
                x: SAFE_MARGIN,
                y: SAFE_MARGIN,
                w: VIEW_WIDTH - 2 * SAFE_MARGIN,
                h: view_h - 2 * SAFE_MARGIN,
            };
            for rect in &all {
                assert!(
                    rect.x >= inside.x
                        && rect.y >= inside.y
                        && rect.right() <= inside.right()
                        && rect.bottom() <= inside.bottom(),
                    "{rect:?} leaves the safe margin at {view_h}"
                );
            }
            // No two top rects intersect, except the (mutually exclusive)
            // location and boss strips.
            for (i, a) in top.iter().enumerate() {
                for (j, b) in top.iter().enumerate().skip(i + 1) {
                    if (i, j) == (2, 3) {
                        continue;
                    }
                    assert!(!rects_intersect(*a, *b), "{a:?} vs {b:?}");
                }
            }
            for (i, a) in bottom.iter().enumerate() {
                for b in bottom.iter().skip(i + 1) {
                    assert!(!rects_intersect(*a, *b), "{a:?} vs {b:?}");
                }
            }
            // At least 8 px between the centre strips and the plates.
            for strip in [wide, boss] {
                assert!(strip.x - layout.carl_plate.right() >= 8);
                assert!(layout.donut_plate.x - strip.right() >= 8);
            }
            assert!(wide.w <= 280);
            // The action lane [60, walk_y + 12] stays free.
            let lane = IRect {
                x: 0,
                y: ACTION_LANE_TOP,
                w: VIEW_WIDTH,
                h: walk_y + 12 - ACTION_LANE_TOP + 1,
            };
            for rect in &all {
                assert!(!rects_intersect(*rect, lane), "{rect:?} in the lane");
            }
            assert!(layout.notice.y >= walk_y + 24);
            assert!(layout.carl_plate.bottom() <= ACTION_LANE_TOP);
            // The notice and the face keep a gap; the PREVIEW tag stays left.
            assert!(layout.face.x - layout.notice.right() >= 4);
            assert!(widest_preview.right() <= 200);
            // The sliding-out panel still ends inside the canvas.
            assert!(layout.notice.right() + 24 <= VIEW_WIDTH);
        }
    }

    #[test]
    fn bottom_anchored_rects_move_down_40_px_at_400() {
        let a = HudLayout::new(360, 288);
        let b = HudLayout::new(400, 308);
        assert_eq!(b.notice.y - a.notice.y, 40);
        assert_eq!(b.face.y - a.face.y, 40);
        assert_eq!(a.carl_plate, b.carl_plate);
        assert_eq!(a.location(0), b.location(0));
    }

    #[test]
    fn boss_slide_is_three_px_per_tick_and_clamps() {
        assert_eq!(HudLayout::boss_y(0, false), -26);
        assert_eq!(HudLayout::boss_y(11, false), 7);
        assert_eq!(HudLayout::boss_y(12, false), 8);
        assert_eq!(HudLayout::boss_y(0, true), 8);
    }

    #[test]
    fn text_widths_match_the_spec_measurements() {
        assert_eq!(text_width_px(HudFont::Jersey, "CARL"), 30);
        assert_eq!(text_width_px(HudFont::Jersey, "DONUT"), 40);
        assert_eq!(text_width_px(HudFont::Jersey, "FLOOR 01"), 52);
        assert_eq!(text_width_px(HudFont::Jersey, "TEST BRUTE"), 71);
        assert_eq!(text_width_px(HudFont::SilkRegular, "PUNCH"), 31);
        assert_eq!(text_width_px(HudFont::SilkRegular, "KICK"), 21);
        assert_eq!(text_width_px(HudFont::SilkRegular, "SWAT"), 24);
        assert_eq!(text_width_px(HudFont::SilkRegular, "POUNCE"), 36);
        assert_eq!(text_width_px(HudFont::SilkRegular, "MISSILE"), 35);
        assert_eq!(text_width_px(HudFont::SilkRegular, "COMPANION"), 54);
    }

    #[test]
    fn segments_per_hp_point_fit_the_bar_exactly() {
        let n = i32::from(HERO_HEALTH);
        assert_eq!((HP_BAR_INNER_W - (n - 1) * 2) % n, 0);
    }

    #[test]
    fn chips_show_only_for_live_actions_story_corridor_vs_combat() {
        // The runtime defaults to the story corridor: melee is ignored there.
        let mut hud = HudRuntime::new(HudConfig::default());
        let AbilityRow::NoActions { special } = hud.snapshot().plates[0].abilities else {
            panic!("story corridor must show no PUNCH/KICK chips");
        };
        assert_eq!(special, SpecialSlot::Locked);
        // A combat section acts on Primary/Physical: the chips appear.
        let before = hud.revision();
        hud.set_melee_live(true);
        assert!(hud.revision() > before);
        let AbilityRow::Chips { a, b, .. } = hud.snapshot().plates[0].abilities else {
            panic!("combat must show the chips");
        };
        assert_eq!((a.label, b.label), (LabelId::Punch, LabelId::Kick));
        // Back to the corridor (a restart into the story run) removes them.
        hud.set_melee_live(false);
        assert!(matches!(
            hud.snapshot().plates[0].abilities,
            AbilityRow::NoActions { .. }
        ));
        // An AI plate never has chips either way.
        assert_eq!(hud.snapshot().plates[1].abilities, AbilityRow::AiCompanion);
    }

    /// HUD-DEBT-02: the plate reads the real mana pool, never a fixture. Locked
    /// shows no bar; unlocked shows whole segments; at 0 the depleted flash
    /// ages once and the MISSILE chip is muted.
    #[test]
    fn the_real_donut_mana_drives_the_bar_the_chips_and_the_depleted_flash() {
        let mut hud = HudRuntime::new(HudConfig::default());
        hud.set_melee_live(true);
        // Solo swap: P1 drives Donut, so her plate shows chips.
        hud.set_party(PartyView {
            carl: Control::Ai,
            donut: Control::Player(1),
            donut_chips: true,
            ..PartyView::default()
        });
        let donut = |hud: &HudRuntime| hud.snapshot().plates[1].clone();
        // Locked: the pre-biscuit SWAT chip and no mana bar.
        let locked = donut(&hud);
        assert_eq!(locked.mana, None);
        let AbilityRow::Chips { a, b, .. } = locked.abilities else {
            panic!("chips")
        };
        assert_eq!((a.label, b.label), (LabelId::Swat, LabelId::Pounce));
        // Unlocked and full: 94 px, MISSILE unmuted.
        let before = hud.revision();
        hud.set_donut_mana(Some((8, 8)));
        hud.step(HERO_HEALTH, DONUT_HEALTH, &[], 0);
        assert!(hud.revision() > before, "the change is a visual change");
        let full = donut(&hud);
        assert_eq!(
            full.mana,
            Some(ManaView {
                fill_px: 94,
                depleted_age: None
            })
        );
        let AbilityRow::Chips { a, .. } = full.abilities else {
            panic!("chips")
        };
        assert_eq!((a.label, a.muted), (LabelId::Missile, false));
        // Whole segments: 3 charges is 35 px.
        hud.set_donut_mana(Some((3, 8)));
        hud.step(HERO_HEALTH, DONUT_HEALTH, &[], 0);
        assert_eq!(donut(&hud).mana.map(|m| m.fill_px), Some(35));
        // Depleted: the flash starts at 0 and ages while she stays at 0.
        hud.set_donut_mana(Some((0, 8)));
        hud.step(HERO_HEALTH, DONUT_HEALTH, &[], 0);
        let empty = donut(&hud);
        assert_eq!(
            empty.mana.map(|m| (m.fill_px, m.depleted_age)),
            Some((0, Some(0)))
        );
        let AbilityRow::Chips { a, b, .. } = empty.abilities else {
            panic!("chips")
        };
        assert_eq!((a.label, a.muted), (LabelId::Missile, true));
        assert!(!b.muted, "POUNCE is never muted");
        hud.step(HERO_HEALTH, DONUT_HEALTH, &[], 0);
        assert_eq!(donut(&hud).mana.and_then(|m| m.depleted_age), Some(1));
        // A charge back clears the flash; relocking removes the bar.
        hud.set_donut_mana(Some((1, 8)));
        hud.step(HERO_HEALTH, DONUT_HEALTH, &[], 0);
        assert_eq!(donut(&hud).mana.and_then(|m| m.depleted_age), None);
        hud.set_donut_mana(None);
        hud.step(HERO_HEALTH, DONUT_HEALTH, &[], 0);
        assert_eq!(donut(&hud).mana, None);
    }

    /// The Donut plate is an AI label, or no chips outside a combat section,
    /// or the chips: from the pure mapping of `donut_hud`.
    #[test]
    fn the_loaded_empty_press_reaches_the_depleted_plate() {
        let mut hud = HudRuntime::new(HudConfig::default());
        hud.set_melee_live(true);
        hud.set_party(PartyView {
            carl: Control::Ai,
            donut: Control::Player(1),
            donut_chips: true,
            ..PartyView::default()
        });
        hud.set_donut_mana(Some((0, 8)));
        let label = |hud: &HudRuntime| {
            let AbilityRow::Chips { a, .. } = hud.snapshot().plates[1].abilities else {
                panic!("chips")
            };
            (a.label, a.muted)
        };
        assert_eq!(label(&hud), (LabelId::Missile, true), "dry press: muted");
        hud.set_empty_press(EmptyPress::Swat);
        assert_eq!(
            label(&hud),
            (LabelId::Swat, false),
            "swat press: unmuted swat"
        );
    }

    #[test]
    fn the_donut_chips_follow_control_the_melee_gate_and_q10_01() {
        let mut real = RealHudInputs::normal(4, 4);
        real.donut_control = Control::Player(2);
        real.donut_chips = true;
        real.melee_live = true;
        let row = |real: &RealHudInputs| build_snapshot(real, None, 0).plates[1].abilities;
        assert!(matches!(row(&real), AbilityRow::Chips { .. }));
        real.melee_live = false;
        assert!(matches!(row(&real), AbilityRow::NoActions { .. }));
        real.melee_live = true;
        real.donut_chips = false;
        assert!(matches!(row(&real), AbilityRow::NoActions { .. }));
        real.donut_control = Control::Ai;
        assert_eq!(row(&real), AbilityRow::AiCompanion);
    }

    #[test]
    fn fixtures_keep_their_chips_whatever_the_capability() {
        let mut real = RealHudInputs::normal(4, 4);
        real.melee_live = false;
        let s = build_snapshot(&real, Some(HudPreview::PostUnlock), 0);
        assert!(matches!(s.plates[0].abilities, AbilityRow::Chips { .. }));
    }

    #[test]
    fn names_and_location_come_from_real_data() {
        assert_eq!(
            location_text("floor01_corridor_abc_sample").as_deref(),
            Some("FLOOR 01")
        );
        assert_eq!(location_text("floor12_x").as_deref(), Some("FLOOR 12"));
        assert_eq!(location_text("floor").as_deref(), None);
        assert_eq!(location_text("floorplan_a").as_deref(), None);
        assert_eq!(location_text("sample").as_deref(), None);
        let mut hud = HudRuntime::new(HudConfig::default());
        assert_eq!(hud.snapshot().location, None);
        assert_eq!(hud.snapshot().plates[0].name, FALLBACK_NAME_LEFT);
        hud.set_identity("NAME A", "NAME B", "floor03_x");
        let s = hud.snapshot();
        assert_eq!(s.plates[0].name, "NAME A");
        assert_eq!(s.plates[1].name, "NAME B");
        assert_eq!(s.location.as_deref(), Some("FLOOR 03"));
        // A fixture keeps the real names.
        let mut fixture = HudRuntime::new(HudConfig {
            preview: Some(HudPreview::Coop),
            ..HudConfig::default()
        });
        fixture.set_identity("NAME A", "NAME B", "floor03_x");
        assert_eq!(fixture.snapshot().plates[1].name, "NAME B");
    }
    // ------------------------------------------------ boss strip (HUD-DEBT-03)

    use crate::boss::{BossMode, BossState};

    fn boss_state(hp: u16, mode: BossMode) -> BossState {
        BossState {
            hp,
            hp_max: 36,
            mode,
            phase: 1,
        }
    }

    /// A HUD runtime and the cumulative event log it reads (its cursor wants
    /// the whole log, like the scene's).
    struct Rig {
        hud: HudRuntime,
        log: Vec<CombatEvent>,
    }

    impl Rig {
        fn new() -> Self {
            Self {
                hud: HudRuntime::new(HudConfig::default()),
                log: Vec::new(),
            }
        }

        fn tick(&mut self, state: BossState, events: &[CombatEvent]) {
            self.log.extend_from_slice(events);
            self.hud
                .set_boss(Some(state), "THE JUICER", "brute_boss_f01_emblem", 90);
            self.hud.step(4, 4, &self.log, 0);
        }

        fn snapshot(&self) -> HudSnapshot {
            self.hud.snapshot()
        }
    }

    #[test]
    fn the_boss_strip_slides_in_on_engage_and_reads_name_fill_and_emblem() {
        let mut hud = Rig::new();
        let state = boss_state(36, BossMode::Neutral);
        hud.tick(state.clone(), &[]);
        assert!(hud.snapshot().boss.is_none(), "no strip before the gate");
        assert!(hud.snapshot().location.is_none() || hud.snapshot().boss.is_none());
        let engaged = [CombatEvent::BossEngaged { tick: 1 }];
        hud.tick(state.clone(), &engaged);
        let view = hud.snapshot().boss.expect("the strip is up");
        assert_eq!(
            (view.name.as_str(), view.fill_px, view.chip_px),
            ("THE JUICER", 234, 0)
        );
        assert_eq!(view.emblem, Some(EmblemId("brute_boss_f01_emblem".into())));
        assert!(view.slide_tick < BOSS_SLIDE_TICKS);
        assert_eq!(
            hud.snapshot().location,
            None,
            "the strip replaces the location"
        );
        for _ in 0..BOSS_SLIDE_TICKS {
            hud.tick(state.clone(), &[]);
        }
        assert_eq!(hud.snapshot().boss.unwrap().slide_tick, BOSS_SLIDE_TICKS);
    }

    #[test]
    fn the_damage_chip_holds_30_ticks_then_drains_2_px_a_tick() {
        let mut hud = Rig::new();
        hud.tick(
            boss_state(36, BossMode::Neutral),
            &[CombatEvent::BossEngaged { tick: 1 }],
        );
        // 36 -> 35 HP: 234 -> 228 px, a 6 px chip.
        hud.tick(boss_state(35, BossMode::Neutral), &[]);
        let view = hud.snapshot().boss.unwrap();
        assert_eq!((view.fill_px, view.chip_px), (228, 6));
        for _ in 0..29 {
            hud.tick(boss_state(35, BossMode::Neutral), &[]);
        }
        assert_eq!(hud.snapshot().boss.unwrap().chip_px, 6, "held for 30 ticks");
        hud.tick(boss_state(35, BossMode::Neutral), &[]);
        assert_eq!(hud.snapshot().boss.unwrap().chip_px, 4);
        for _ in 0..3 {
            hud.tick(boss_state(35, BossMode::Neutral), &[]);
        }
        assert_eq!(hud.snapshot().boss.unwrap().chip_px, 0);
    }

    #[test]
    fn reduced_motion_removes_the_damage_chip_at_once() {
        let mut hud = Rig {
            hud: HudRuntime::new(HudConfig {
                settings: HudSettings {
                    reduced_motion: true,
                    ..HudSettings::default()
                },
                ..HudConfig::default()
            }),
            log: Vec::new(),
        };
        hud.tick(
            boss_state(36, BossMode::Neutral),
            &[CombatEvent::BossEngaged { tick: 1 }],
        );
        hud.tick(boss_state(35, BossMode::Neutral), &[]);
        let view = hud.snapshot().boss.unwrap();
        assert_eq!(view.fill_px, 228, "the bar still drops");
        assert_eq!(view.chip_px, 0, "HUD spec section 5: no chip");
        assert_eq!(view.slide_tick, BOSS_SLIDE_TICKS, "and no slide");
    }

    #[test]
    fn the_warning_icon_is_up_on_exactly_the_tell_modes() {
        let mut hud = Rig::new();
        hud.tick(
            boss_state(36, BossMode::Neutral),
            &[CombatEvent::BossEngaged { tick: 1 }],
        );
        for (mode, expected) in [
            (BossMode::PoundTell, true),
            (BossMode::SecondTell, true),
            (BossMode::ChargeTell, true),
            (BossMode::Roar, false),
            (BossMode::PoundStrike, false),
            (BossMode::Charge, false),
            (BossMode::Pinned, false),
        ] {
            hud.tick(boss_state(30, mode), &[]);
            assert_eq!(hud.snapshot().boss.unwrap().warning, expected, "{mode:?}");
        }
    }

    #[test]
    fn the_strip_leaves_90_ticks_after_defeat_and_a_restart_removes_it_at_once() {
        let mut hud = Rig::new();
        hud.tick(
            boss_state(36, BossMode::Neutral),
            &[CombatEvent::BossEngaged { tick: 1 }],
        );
        for _ in 0..20 {
            hud.tick(boss_state(36, BossMode::Neutral), &[]);
        }
        let down = [CombatEvent::EnemyDefeated {
            tick: 9,
            target: BOSS_ENEMY_ID,
            cause: crate::combat::DefeatCause::Melee,
            force: ForceTier::Routine,
            instigator: Some(0),
        }];
        hud.tick(boss_state(0, BossMode::Defeated), &down);
        for _ in 0..80 {
            hud.tick(boss_state(0, BossMode::Defeated), &[]);
        }
        let view = hud.snapshot().boss.expect("still up inside the 90 ticks");
        assert_eq!(view.fill_px, 0);
        for _ in 0..30 {
            hud.tick(boss_state(0, BossMode::Defeated), &[]);
        }
        assert!(hud.snapshot().boss.is_none(), "gone after the slide out");
        // A restart removes a live strip at once, and it slides in again.
        let mut hud = Rig::new();
        hud.tick(
            boss_state(20, BossMode::Neutral),
            &[CombatEvent::BossEngaged { tick: 1 }],
        );
        hud.tick(
            boss_state(20, BossMode::Neutral),
            &[CombatEvent::Restart { tick: 3, deaths: 1 }],
        );
        assert!(hud.snapshot().boss.is_none());
        hud.tick(
            boss_state(36, BossMode::Neutral),
            &[CombatEvent::BossEngaged { tick: 1 }],
        );
        assert!(hud.snapshot().boss.unwrap().slide_tick < BOSS_SLIDE_TICKS);
    }

    #[test]
    fn the_boss_notices_come_from_real_events_and_fit_13_characters() {
        let phase = |n| CombatEvent::BossPhase { tick: 1, phase: n };
        assert_eq!(
            notice_for_event(&CombatEvent::BossEngaged { tick: 1 }),
            Some(LabelId::MainEvent)
        );
        assert_eq!(notice_for_event(&phase(2)), Some(LabelId::RoundTwo));
        assert_eq!(notice_for_event(&phase(3)), Some(LabelId::FinalRound));
        let landed = |crushed| CombatEvent::RackLanded {
            tick: 1,
            rack: 0,
            crushed,
        };
        assert_eq!(notice_for_event(&landed(true)), Some(LabelId::NoSpotter));
        assert_eq!(notice_for_event(&landed(false)), None);
        let down = |force| CombatEvent::EnemyDefeated {
            tick: 1,
            target: BOSS_ENEMY_ID,
            cause: crate::combat::DefeatCause::Melee,
            force,
            instigator: Some(0),
        };
        for force in [
            ForceTier::Routine,
            ForceTier::Heavy,
            ForceTier::Catastrophic,
        ] {
            assert_eq!(notice_for_event(&down(force)), Some(LabelId::Knockout));
        }
        for label in [
            LabelId::MainEvent,
            LabelId::RoundTwo,
            LabelId::FinalRound,
            LabelId::NoSpotter,
            LabelId::Knockout,
        ] {
            assert!(label.text().len() <= 13, "{}", label.text());
        }
        // A tell, a hurt and a strike never raise a notice.
        assert_eq!(
            notice_for_event(&CombatEvent::BossTell {
                tick: 1,
                mv: crate::boss::BossMove::Pound,
                target: crate::party::HeroId::Carl
            }),
            None
        );
    }

    #[test]
    fn the_boss_preview_fixture_stays_available_and_unchanged() {
        let s = snap(HudPreview::Boss);
        let boss = s.boss.unwrap();
        assert_eq!(boss.name, "TEST BRUTE");
        assert_eq!((boss.fill_px, boss.chip_px, boss.warning), (164, 14, true));
        assert_eq!(boss.emblem, None, "the generic skull plate");
        assert_eq!(s.notice.unwrap().headline, LabelId::BossIncoming);
    }

    // ------------------------------------------------ AUDIENCE-01 part C --

    use crate::audience::{AnticipationTrigger, TUNING, format_count};
    use crate::combat::{CARL_PUSH_KICK, DONUT_POUNCE, DONUT_SWAT, MAGIC_MISSILE, MoveId};

    /// A runtime with the audience unlocked (the QA probe's state) and an
    /// optional cue at `cue_x` for both heroes.
    fn live_audience(cue_x: Option<i32>) -> HudRuntime {
        let mut hud = HudRuntime::new(HudConfig::default());
        let cues = cue_x
            .map(|x| {
                vec![AnticipationCue {
                    id: 1,
                    trigger: AnticipationTrigger::HeroCrossesX { x },
                    heroes: vec![HeroId::Carl, HeroId::Donut],
                    award: 4_000,
                }]
            })
            .unwrap_or_default();
        hud.unlock_audience(cues).unwrap();
        hud
    }

    /// Append `event` to the cumulative log and take one HUD step.
    fn push_step(hud: &mut HudRuntime, log: &mut Vec<CombatEvent>, event: CombatEvent) {
        log.push(event);
        hud.step(4, 4, log, 0);
    }

    fn counts(hud: &HudRuntime) -> (u64, u64) {
        let s = hud.snapshot();
        let n = |i: usize| s.plates[i].audience.expect("unlocked").shown;
        (n(0), n(1))
    }

    fn live_counts(hud: &HudRuntime) -> (u64, u64) {
        (
            hud.audience().count(HeroId::Carl).unwrap(),
            hud.audience().count(HeroId::Donut).unwrap(),
        )
    }

    #[test]
    fn the_viewer_previews_only_add_the_readouts() {
        let base = build_snapshot(&test_inputs(4, 4), None, 0);
        for preview in [
            HudPreview::Viewers,
            HudPreview::ViewersRising,
            HudPreview::ViewersFalling,
            HudPreview::ViewersMax,
        ] {
            let s = build_snapshot(&test_inputs(4, 4), Some(preview), 0);
            assert_eq!(s.preview, Some(preview));
            for i in 0..2 {
                assert!(s.plates[i].audience.is_some(), "{preview:?} plate {i}");
                let mut plate = s.plates[i].clone();
                plate.audience = None;
                assert_eq!(plate, base.plates[i], "{preview:?}: the real plate stays");
            }
        }
        let trend = |p, i: usize| snap(p).plates[i].audience.unwrap().trend;
        assert_eq!(trend(HudPreview::Viewers, 0), Trend::Flat);
        assert_eq!(trend(HudPreview::ViewersRising, 1), Trend::Up);
        assert_eq!(trend(HudPreview::ViewersFalling, 0), Trend::Down);
        let max = snap(HudPreview::ViewersMax);
        assert_eq!(
            format_count(max.plates[0].audience.unwrap().shown),
            "999M+",
            "the widest string"
        );
        assert_eq!(format_count(max.plates[1].audience.unwrap().shown), "99.9K");
    }

    #[test]
    fn a_locked_audience_never_shows_and_never_reads_the_events() {
        let mut hud = HudRuntime::new(HudConfig::default());
        let mut log = Vec::new();
        push_step(&mut hud, &mut log, defeat(ForceTier::Catastrophic));
        assert!(!hud.audience().is_unlocked());
        let s = hud.snapshot();
        assert!(s.plates[0].audience.is_none() && s.plates[1].audience.is_none());
        assert_eq!(hud.audience().steps(), 0, "no step runs while locked");
        assert_eq!(hud.audience().digest(), None);
        // Feats and positions are ignored too.
        hud.push_chain_feat(ChainFeat {
            credited: 0,
            target: 1,
            score: 9,
        });
        hud.set_hero_x(10_000, 10_000);
        hud.step(4, 4, &log, 0);
        assert!(hud.snapshot().plates[0].audience.is_none());
    }

    /// A locked audience costs nothing: the same events, feats and positions
    /// bump the revision exactly as often as a runtime that never heard of
    /// the audience (so the renderer never rebuilds the HUD sprites for it).
    #[test]
    fn a_locked_audience_never_bumps_the_revision() {
        let events = [
            defeat(ForceTier::Catastrophic),
            defeat(ForceTier::Routine),
            hurt(3),
        ];
        let run = |with_audience_inputs: bool| {
            let mut hud = HudRuntime::new(HudConfig::default());
            let mut log = Vec::new();
            let mut revisions = Vec::new();
            for event in &events {
                if with_audience_inputs {
                    hud.push_chain_feat(ChainFeat {
                        credited: 0,
                        target: 1,
                        score: 9,
                    });
                    hud.set_hero_x(10_000, 10_000);
                }
                push_step(&mut hud, &mut log, *event);
                revisions.push(hud.revision());
            }
            assert!(!hud.audience().is_unlocked());
            revisions
        };
        let plain = run(false);
        assert_eq!(run(true), plain, "a locked audience bumped the revision");
        // The baseline moves (the events draw notices), so the comparison is
        // not vacuous.
        assert!(plain.iter().any(|&r| r > 0));
    }

    #[test]
    fn unlock_shows_both_counts_at_the_baseline_and_bumps_the_revision() {
        let mut hud = HudRuntime::new(HudConfig::default());
        let before = hud.revision();
        hud.unlock_audience(Vec::new()).unwrap();
        assert!(hud.revision() > before, "the renderer must rebuild");
        assert_eq!(counts(&hud), (TUNING.baseline, TUNING.baseline));
    }

    #[test]
    fn a_donut_kill_credits_hero_one_and_never_hero_zero() {
        for (id, spec) in [
            (MoveId::Swat, &DONUT_SWAT),
            (MoveId::Pounce, &DONUT_POUNCE),
            (MoveId::MagicMissile, &MAGIC_MISSILE),
        ] {
            let mut hud = live_audience(None);
            let mut log = Vec::new();
            // The scene logs a Donut kill with `instigator = Some(DONUT_HERO)`
            // (`MoveId::hero()`), never the controller slot.
            push_step(
                &mut hud,
                &mut log,
                CombatEvent::enemy_defeated(3, 5, DefeatCause::Melee, Some(id.hero()), Some(spec)),
            );
            let (carl, donut) = live_counts(&hud);
            assert_eq!(carl, TUNING.baseline, "{id:?} must never credit hero 0");
            assert!(donut > TUNING.baseline, "{id:?} credits Donut");
        }
        // Carl's push kick (a launching, heavy finish) credits Carl only.
        let mut hud = live_audience(None);
        let mut log = Vec::new();
        push_step(
            &mut hud,
            &mut log,
            CombatEvent::enemy_defeated(
                3,
                5,
                DefeatCause::Melee,
                Some(MoveId::PushKick.hero()),
                Some(&CARL_PUSH_KICK),
            ),
        );
        let (carl, donut) = live_counts(&hud);
        assert_eq!(carl, TUNING.baseline + TUNING.award_heavy);
        assert_eq!(donut, TUNING.baseline);
    }

    #[test]
    fn the_death_causes_credit_the_hero_that_caused_them() {
        let cases = [
            (DefeatCause::ShoreBlockEdge { player: 1 }, None, (0, 1)),
            (DefeatCause::ShoreBlock { player: 0 }, None, (1, 0)),
            (
                DefeatCause::Explosion {
                    charge: 3,
                    player: Some(1),
                    point_blank: true,
                },
                None,
                (0, 1),
            ),
            (
                DefeatCause::Explosion {
                    charge: 3,
                    player: Some(0),
                    point_blank: false,
                },
                None,
                (1, 0),
            ),
            (DefeatCause::DropHazard { player: 0, rack: 1 }, None, (1, 0)),
            (DefeatCause::DropHazard { player: 1, rack: 0 }, None, (0, 1)),
        ];
        for (cause, instigator, (carl_up, donut_up)) in cases {
            let mut hud = live_audience(None);
            let mut log = Vec::new();
            let target = if matches!(cause, DefeatCause::DropHazard { .. }) {
                BOSS_ENEMY_ID
            } else {
                7
            };
            push_step(
                &mut hud,
                &mut log,
                CombatEvent::enemy_defeated(9, target, cause, instigator, None),
            );
            let (carl, donut) = live_counts(&hud);
            assert_eq!(
                (carl > TUNING.baseline, donut > TUNING.baseline),
                (carl_up == 1, donut_up == 1),
                "{cause:?}"
            );
        }
    }

    #[test]
    fn a_cine_feat_credits_the_named_hero_only() {
        let mut hud = live_audience(None);
        let log = Vec::new();
        hud.push_chain_feat(ChainFeat {
            credited: 1,
            target: 4,
            score: 8,
        });
        hud.step(4, 4, &log, 0);
        let (carl, donut) = live_counts(&hud);
        assert_eq!(carl, TUNING.baseline);
        assert_eq!(donut, TUNING.baseline + 250 * (8 - 4));
    }

    #[test]
    fn an_authored_cue_fires_once_when_a_hero_reaches_its_column() {
        let mut hud = live_audience(Some(500));
        let log = Vec::new();
        hud.set_hero_x(400, 380);
        hud.step(4, 4, &log, 0);
        assert_eq!(live_counts(&hud), (250, 250));
        hud.set_hero_x(501, 380);
        hud.step(4, 4, &log, 0);
        assert_eq!(live_counts(&hud), (4_250, 4_250), "both targeted heroes");
        hud.step(4, 4, &log, 0);
        assert_eq!(live_counts(&hud), (4_250, 4_250), "fires once");
        let a = hud.snapshot().plates[0].audience.unwrap();
        assert_eq!(a.trend, Trend::Up, "a cue reads like any spectacular gain");
    }

    #[test]
    fn a_restart_event_restores_the_attempt_start_counts() {
        let mut hud = live_audience(None);
        let mut log = Vec::new();
        push_step(&mut hud, &mut log, defeat(ForceTier::Catastrophic));
        assert!(live_counts(&hud).0 > TUNING.baseline);
        push_step(
            &mut hud,
            &mut log,
            CombatEvent::Restart { tick: 9, deaths: 1 },
        );
        assert_eq!(live_counts(&hud), (TUNING.baseline, TUNING.baseline));
        // The same target id in the new attempt pays again once (a new
        // attempt), but the dying costs nothing and farms nothing.
        push_step(&mut hud, &mut log, defeat(ForceTier::Routine));
        assert_eq!(live_counts(&hud).0, TUNING.baseline + TUNING.award_routine);
    }

    #[test]
    fn the_audience_runs_beside_the_hud_without_touching_the_other_plate_fields() {
        let mut quiet = HudRuntime::new(HudConfig::default());
        let mut live = live_audience(None);
        let mut log = Vec::new();
        for event in [hurt(3), defeat(ForceTier::Heavy)] {
            log.push(event);
            quiet.step(3, 4, &log, 0);
            live.step(3, 4, &log, 0);
        }
        let (a, b) = (quiet.snapshot(), live.snapshot());
        for i in 0..2 {
            let mut plate = b.plates[i].clone();
            assert!(plate.audience.take().is_some());
            assert_eq!(plate, a.plates[i], "plate {i}");
        }
        assert_eq!((a.notice, a.face), (b.notice, b.face));
    }

    // ---------------------------------------------- DONUT-PROG-01 DP-D: HUD --

    fn granted(hero: u8, capability: &'static str) -> CombatEvent {
        CombatEvent::CapabilityGranted {
            tick: 1,
            hero,
            capability,
        }
    }

    /// CP-Q43: Carl's reinforced-fist grant raises its notice from the same event,
    /// by the data row's text, at most 13 characters and inside the notice frame;
    /// Donut's grant of the same id is not Carl's notice.
    #[test]
    fn the_reinforced_fist_grant_raises_its_notice_from_the_data_row() {
        let row = crate::story_unlock::StoryTable::active()
            .notice("fist_reinforced")
            .unwrap();
        assert_eq!(
            notice_for_event(&granted(0, "fist_reinforced")),
            Some(LabelId::HeavyFist)
        );
        assert_eq!(
            LabelId::for_notice_text(&row.text),
            Some(LabelId::HeavyFist)
        );
        assert_eq!(LabelId::HeavyFist.text(), row.text);
        assert!(LabelId::HeavyFist.text().len() <= 13);
        let layout = HudLayout::new(360, 288);
        assert!(text_width_px(HudFont::Jersey, LabelId::HeavyFist.text()) <= layout.notice.w - 14);
        assert_eq!(notice_for_event(&granted(1, "fist_reinforced")), None);
    }

    /// DP-Q12: Donut's two grants raise their notices, from the same event, and the
    /// labels are the data rows' text (so the two cannot drift); at most 13
    /// characters, and wide enough for the notice frame.
    #[test]
    fn donut_grants_raise_their_notices_from_the_data_rows() {
        let table = crate::story_unlock::StoryTable::active();
        for (capability, label) in [
            ("jump_reach_boost", LabelId::LightFeet),
            ("special_short_blink", LabelId::PuddleJumper),
        ] {
            assert_eq!(notice_for_event(&granted(1, capability)), Some(label));
            let row = table.notice(capability).unwrap();
            assert_eq!(LabelId::for_notice_text(&row.text), Some(label));
            assert_eq!(label.text(), row.text);
            assert!(label.text().len() <= 13);
            let layout = HudLayout::new(360, 288);
            assert!(text_width_px(HudFont::Jersey, label.text()) <= layout.notice.w - 14);
            // Another hero's grant of the same id is not Donut's notice.
            assert_eq!(notice_for_event(&granted(0, capability)), None);
        }
        assert_eq!(
            LabelId::for_socket_text("BLINK"),
            Some(LabelId::Blink),
            "the socket label is by data text"
        );
        assert_eq!(LabelId::for_socket_text("BOOM"), Some(LabelId::Boom));
        assert_eq!(LabelId::for_socket_text("ZOOM"), None);
    }

    /// DP-Q12: on the Juicer tick with both programs on, the two notices queue in
    /// the grants-list order (Carl's first), each for its normal duration with the
    /// normal gap; the second never replaces the first.
    #[test]
    fn two_grants_on_one_tick_queue_in_grant_list_order() {
        let mut hud = HudRuntime::new(HudConfig::default());
        let log = vec![
            granted(0, "special_thrown_charge"),
            granted(1, "special_short_blink"),
        ];
        hud.step(4, 4, &log, 0);
        let first = hud.snapshot().notice.expect("Carl's notice starts at once");
        assert_eq!((first.headline, first.age_ticks), (LabelId::GoblinBoom, 0));
        assert!(hud.notices.has_pending());
        let mut start_of_second = None;
        for tick in 1..400u16 {
            hud.step(4, 4, &log, 0);
            match hud.snapshot().notice {
                Some(n) if n.headline == LabelId::PuddleJumper => {
                    start_of_second = Some((tick, n.age_ticks));
                    break;
                }
                Some(n) => assert_eq!(n.headline, LabelId::GoblinBoom, "tick {tick}"),
                None => {}
            }
        }
        // The first lives NOTICE_TOTAL_TICKS, then the gap, then the second starts.
        assert_eq!(
            start_of_second,
            Some((NOTICE_TOTAL_TICKS + NOTICE_MIN_GAP_TICKS, 0))
        );
    }

    /// DP-Q33: the socket flashes for `MANA_FLASH_TICKS` on ready, and on a refusal
    /// or fizzle for want of mana or a destination; a busy or cooling press and
    /// Carl's special never flash Donut's socket.
    #[test]
    fn the_donut_socket_flashes_once_on_ready_and_on_a_dry_press() {
        use crate::thrown_charge::SpecialReject;
        let flash = |hud: &HudRuntime| hud.snapshot().plates[1].socket_flash;
        let run = |event: CombatEvent| -> Vec<Option<SocketFlash>> {
            let mut hud = HudRuntime::new(HudConfig::default());
            let mut out = Vec::new();
            hud.step(4, 4, &[event], 0);
            out.push(flash(&hud));
            for _ in 0..8 {
                hud.step(4, 4, &[event], 0);
                out.push(flash(&hud));
            }
            out
        };
        let on = |kind, age_ticks| Some(SocketFlash { kind, age_ticks });
        let ready = run(CombatEvent::SpecialReady { tick: 1, hero: 1 });
        // The cursor shows the event once: age 0 on its tick, 5 on the last, then none.
        assert_eq!(ready[0], on(SocketFlashKind::Ready, 0));
        assert_eq!(ready[5], on(SocketFlashKind::Ready, 5));
        assert_eq!(ready[6], None, "gone after {MANA_FLASH_TICKS} ticks");
        assert_eq!(MANA_FLASH_TICKS, 6);
        for reason in [SpecialReject::NoMana, SpecialReject::NoDestination] {
            let seq = run(CombatEvent::SpecialRejected {
                tick: 1,
                hero: 1,
                reason,
            });
            assert_eq!(seq[0], on(SocketFlashKind::Refused, 0), "{reason:?}");
        }
        let fizzle = run(CombatEvent::BlinkFizzled {
            tick: 1,
            hero: 1,
            reason: crate::short_blink::FizzleReason::NoDestination,
        });
        assert_eq!(fizzle[0], on(SocketFlashKind::Refused, 0));
        for quiet in [
            CombatEvent::SpecialRejected {
                tick: 1,
                hero: 1,
                reason: SpecialReject::Busy,
            },
            CombatEvent::SpecialRejected {
                tick: 1,
                hero: 1,
                reason: SpecialReject::Cooldown,
            },
            CombatEvent::SpecialReady { tick: 1, hero: 0 },
            CombatEvent::SpecialRejected {
                tick: 1,
                hero: 0,
                reason: SpecialReject::NoMana,
            },
        ] {
            assert!(run(quiet).iter().all(Option::is_none), "{quiet:?}");
        }
        // A restart clears it; the flash is on Donut's plate only.
        let mut hud = HudRuntime::new(HudConfig::default());
        hud.step(4, 4, &[CombatEvent::SpecialReady { tick: 1, hero: 1 }], 0);
        assert_eq!(hud.snapshot().plates[0].socket_flash, None);
        hud.restart();
        assert_eq!(flash(&hud), None);
    }

    /// The socket is part of the visual key: a new cooldown width or a flash
    /// bumps the revision, a steady one does not.
    #[test]
    fn the_donut_socket_bumps_the_revision_only_when_it_changes() {
        let mut hud = HudRuntime::new(HudConfig::default());
        let chip = Chip {
            action: LogicalAction::Special,
            label: LabelId::Blink,
            muted: true,
        };
        let slot = SpecialSlot::Cooling { chip, bar_px: 6 };
        let r0 = hud.revision;
        hud.set_donut_special(slot);
        assert_eq!(hud.revision, r0 + 1);
        hud.set_donut_special(slot);
        assert_eq!(hud.revision, r0 + 1, "unchanged");
        hud.set_donut_special(SpecialSlot::Cooling { chip, bar_px: 5 });
        assert_eq!(hud.revision, r0 + 2);
        assert_eq!(
            hud.real_inputs().donut_special,
            SpecialSlot::Cooling { chip, bar_px: 5 }
        );
    }
}

/// F01-HUD-ROOM: the room title, the attempt counter and the TEST LOADOUT tag.
#[cfg(test)]
mod room_tests {
    use super::*;
    use crate::hud_draw::{DrawKind, build_draw_list};

    fn manifest_titles() -> Vec<String> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        crate::f01_late::Manifest::load(&root)
            .unwrap()
            .rooms
            .into_iter()
            .map(|r| r.title)
            .collect()
    }

    #[test]
    fn every_listed_room_title_fits_the_strip_with_its_attempt_counter() {
        for title in manifest_titles() {
            let text = room_location_text(&title, 99);
            assert_eq!(text, format!("{} \u{b7} TRY 99", title.to_uppercase()));
            assert!(text_width_px(HudFont::Jersey, &text) <= LOCATION_TEXT_MAX_PX);
        }
        assert_eq!(
            room_location_text("The Hoarder", 1),
            "THE HOARDER \u{b7} TRY 1"
        );
        // A title too wide loses letters, never the counter.
        let long = room_location_text(&"W".repeat(40), 3);
        assert!(long.ends_with(" \u{b7} TRY 3"), "{long}");
        assert!(text_width_px(HudFont::Jersey, &long) <= LOCATION_TEXT_MAX_PX);
    }

    #[test]
    fn a_room_run_shows_its_title_attempt_and_tag() {
        let mut hud = HudRuntime::new(HudConfig::default());
        hud.set_identity("A", "B", "floor01_x");
        // Outside a room nothing changes: the floor number, no tag, and the
        // attempt never bumps the revision.
        let r0 = hud.revision;
        hud.set_attempt(4);
        assert_eq!(hud.revision, r0);
        assert_eq!(hud.snapshot().location.as_deref(), Some("FLOOR 01"));
        assert!(!hud.snapshot().loadout_tag);
        hud.set_room("The Hoarder", true);
        assert_eq!(
            hud.snapshot().location.as_deref(),
            Some("THE HOARDER \u{b7} TRY 4")
        );
        assert!(hud.snapshot().loadout_tag);
        let r1 = hud.revision;
        hud.set_attempt(4);
        assert_eq!(hud.revision, r1, "unchanged");
        hud.set_attempt(5);
        assert_eq!(hud.revision, r1 + 1);
        assert_eq!(
            hud.snapshot().location.as_deref(),
            Some("THE HOARDER \u{b7} TRY 5")
        );
        // A later identity keeps the room.
        hud.set_identity("A", "B", "floor01_x");
        assert_eq!(
            hud.snapshot().location.as_deref(),
            Some("THE HOARDER \u{b7} TRY 5")
        );
        hud.set_room("The Juicer", false);
        assert!(!hud.snapshot().loadout_tag);
    }

    fn texts(snapshot: &HudSnapshot, view_h: i32) -> Vec<(String, i32)> {
        let mut out = Vec::new();
        build_draw_list(snapshot, &HudLayout::new(view_h, view_h - 72), &mut out);
        out.iter()
            .filter_map(|i| match &i.kind {
                DrawKind::Text { text, .. } => Some((text.clone(), i.y)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_tag_draws_under_the_location_or_the_boss_strip_inside_the_margins() {
        for view_h in [360, 400] {
            let layout = HudLayout::new(view_h, view_h - 72);
            let tw = text_width_px(HudFont::SilkBold, LOADOUT_TAG_TEXT);
            let widest = layout.location(LOCATION_TEXT_MAX_PX);
            for (below_boss, strip) in [(false, widest), (true, layout.boss())] {
                let tag = layout.loadout_tag(tw, below_boss);
                assert!(tag.y >= strip.bottom() + 2, "{tag:?} under {strip:?}");
                for plate in [layout.carl_plate, layout.donut_plate] {
                    assert!(!rects_intersect(tag, plate), "{tag:?} vs {plate:?}");
                }
                assert!(tag.x >= SAFE_MARGIN && tag.right() <= VIEW_WIDTH - SAFE_MARGIN);
            }
            let mut real = RealHudInputs::normal(4, 4);
            real.location = Some(room_location_text("The Hoarder", 2));
            real.loadout_tag = true;
            let drawn = texts(&build_snapshot(&real, None, 0), view_h);
            assert!(drawn.iter().any(|(t, _)| t == "THE HOARDER \u{b7} TRY 2"));
            assert!(drawn.iter().any(|(t, _)| t == LOADOUT_TAG_TEXT));
            real.loadout_tag = false;
            let drawn = texts(&build_snapshot(&real, None, 0), view_h);
            assert!(!drawn.iter().any(|(t, _)| t == LOADOUT_TAG_TEXT));
        }
    }
}
