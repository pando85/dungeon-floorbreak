//! HUD-HYBRID-A-01: the Bevy-free draw list of the hybrid A HUD.
//!
//! [`build_draw_list`] turns a [`HudSnapshot`] into flat items (solid rects,
//! atlas frames, text lines) at whole-pixel virtual coordinates, top-left
//! origin. The Bevy adapter (`hud_adapter`) only places these items. Frame
//! names, sizes and the palette-coloured placeholders used while the art atlas
//! is missing live here too, so the whole HUD layout is unit-testable
//! headlessly. Spec: `docs/02-art/hybrid-hud-a-native-spec.md` section 2.

use crate::env_kit::IRect;
use crate::hud::{
    AbilityRow, BOSS_BAR_INNER_W, Chip, Control, FaceFrame, Glyph, HP_BAR_INNER_W,
    HP_FLASH_STEP_TICKS, HP_FLASH_TICKS, HpFlash, HudFont, HudLayout, HudSnapshot,
    MANA_BAR_INNER_W, MANA_FLASH_TICKS, ManaView, PORTRAIT_CARL, PlateView, Rgb, SpecialSlot,
    notice_dx, palette as p, text_width_px,
};

// -------------------------------------------------------------- frame names

/// One named rectangle of the `hud_hybrid_a` atlas. The names and sizes are
/// the contract with the art atlas: they equal the `hud.regions[].name` keys of
/// `assets/manifests/hud_hybrid_a_v1.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameId {
    PlateFrame,
    StripCapLeft,
    StripMid,
    StripCapRight,
    BossCapLeft,
    BossMid,
    BossCapRight,
    NoticeFrame,
    PortraitCarl,
    PortraitDonut,
    CrtIdle,
    CrtBlink,
    CrtNotice,
    GlyphA,
    GlyphB,
    GlyphX,
    GlyphY,
    GlyphCross,
    GlyphCircle,
    GlyphSquare,
    GlyphTriangle,
    SocketLocked,
    IconHeart,
    IconHeartCracked,
    IconSpark,
    IconSparkDepleted,
    IconWarning,
    IconBossEmblem,
    TagP1,
    TagP2,
    TagAi,
}

impl FrameId {
    pub const ALL: [Self; 31] = [
        Self::PlateFrame,
        Self::StripCapLeft,
        Self::StripMid,
        Self::StripCapRight,
        Self::BossCapLeft,
        Self::BossMid,
        Self::BossCapRight,
        Self::NoticeFrame,
        Self::PortraitCarl,
        Self::PortraitDonut,
        Self::CrtIdle,
        Self::CrtBlink,
        Self::CrtNotice,
        Self::GlyphA,
        Self::GlyphB,
        Self::GlyphX,
        Self::GlyphY,
        Self::GlyphCross,
        Self::GlyphCircle,
        Self::GlyphSquare,
        Self::GlyphTriangle,
        Self::SocketLocked,
        Self::IconHeart,
        Self::IconHeartCracked,
        Self::IconSpark,
        Self::IconSparkDepleted,
        Self::IconWarning,
        Self::IconBossEmblem,
        Self::TagP1,
        Self::TagP2,
        Self::TagAi,
    ];

    /// The frame name in the atlas metadata.
    pub const fn name(self) -> &'static str {
        match self {
            Self::PlateFrame => "plate_frame",
            Self::StripCapLeft => "strip_loc_left",
            Self::StripMid => "strip_loc_mid",
            Self::StripCapRight => "strip_loc_right",
            Self::BossCapLeft => "strip_boss_left",
            Self::BossMid => "strip_boss_mid",
            Self::BossCapRight => "strip_boss_right",
            Self::NoticeFrame => "notice_frame",
            Self::PortraitCarl => "portrait_carl",
            Self::PortraitDonut => "portrait_donut",
            Self::CrtIdle => "crt_idle",
            Self::CrtBlink => "crt_blink",
            Self::CrtNotice => "crt_notice",
            Self::GlyphA => "glyph_a",
            Self::GlyphB => "glyph_b",
            Self::GlyphX => "glyph_x",
            Self::GlyphY => "glyph_y",
            Self::GlyphCross => "glyph_cross",
            Self::GlyphCircle => "glyph_circle",
            Self::GlyphSquare => "glyph_square",
            Self::GlyphTriangle => "glyph_triangle",
            Self::SocketLocked => "socket_locked",
            Self::IconHeart => "heart",
            Self::IconHeartCracked => "heart_cracked",
            Self::IconSpark => "spark",
            Self::IconSparkDepleted => "spark_depleted",
            Self::IconWarning => "warning",
            Self::IconBossEmblem => "boss_emblem",
            Self::TagP1 => "tag_p1",
            Self::TagP2 => "tag_p2",
            Self::TagAi => "tag_ai",
        }
    }

    /// Native size in virtual pixels `(w, h)`.
    pub const fn size(self) -> (i32, i32) {
        match self {
            Self::PlateFrame => (164, 48),
            Self::StripCapLeft | Self::StripCapRight => (8, 16),
            Self::StripMid => (1, 16),
            Self::BossCapLeft | Self::BossCapRight => (8, 26),
            Self::BossMid => (1, 26),
            Self::NoticeFrame => (204, 32),
            Self::PortraitCarl | Self::PortraitDonut => (28, 28),
            Self::CrtIdle | Self::CrtBlink | Self::CrtNotice => (20, 18),
            Self::GlyphA
            | Self::GlyphB
            | Self::GlyphX
            | Self::GlyphY
            | Self::GlyphCross
            | Self::GlyphCircle
            | Self::GlyphSquare
            | Self::GlyphTriangle
            | Self::SocketLocked => (11, 11),
            Self::IconHeart | Self::IconHeartCracked => (7, 7),
            Self::IconSpark | Self::IconSparkDepleted => (5, 5),
            Self::IconWarning => (11, 10),
            Self::IconBossEmblem => (20, 20),
            Self::TagP1 | Self::TagP2 | Self::TagAi => (32, 9),
        }
    }

    pub const fn of_glyph(glyph: Glyph) -> Self {
        match glyph {
            Glyph::A => Self::GlyphA,
            Glyph::B => Self::GlyphB,
            Glyph::X => Self::GlyphX,
            Glyph::Y => Self::GlyphY,
            Glyph::Cross => Self::GlyphCross,
            Glyph::Circle => Self::GlyphCircle,
            Glyph::Square => Self::GlyphSquare,
            Glyph::Triangle => Self::GlyphTriangle,
        }
    }

    pub const fn is_glyph(self) -> bool {
        matches!(
            self,
            Self::GlyphA
                | Self::GlyphB
                | Self::GlyphX
                | Self::GlyphY
                | Self::GlyphCross
                | Self::GlyphCircle
                | Self::GlyphSquare
                | Self::GlyphTriangle
        )
    }
}

// ----------------------------------------------------------------- draw list

/// Z bands (spec 0): frames 0, wells and bars 1, portraits and icons 2, text 3,
/// flash overlays 4, PREVIEW tag 9. Items within a band keep emission order.
pub const Z_FRAME: f32 = 0.0;
pub const Z_BAR: f32 = 1.0;
pub const Z_ICON: f32 = 2.0;
pub const Z_TEXT: f32 = 3.0;
pub const Z_FLASH: f32 = 4.0;
pub const Z_PREVIEW: f32 = 9.0;
/// Emission-order step inside a band (the list has well under 1000 items).
const Z_STEP: f32 = 0.0005;

#[derive(Debug, Clone, PartialEq)]
pub enum DrawKind {
    /// A solid rect.
    Rect { w: i32, h: i32, color: Rgb },
    /// An atlas frame, stretched to `w` x `h` (3-slice middles only).
    /// `tint` multiplies the frame colour (muted glyph, dim warning).
    Frame {
        frame: FrameId,
        w: i32,
        h: i32,
        tint: Option<Rgb>,
    },
    /// The boss strip's own 18 x 18 emblem (`boss-f01-emblem`): the adapter
    /// draws the exported art, or [`emblem_placeholder_rects`] without it.
    Emblem { w: i32, h: i32 },
    /// One line of text; `y` is the **caps top** and `x` the left edge.
    Text {
        font: HudFont,
        text: String,
        color: Rgb,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrawItem {
    pub x: i32,
    pub y: i32,
    pub z: f32,
    pub kind: DrawKind,
}

struct Out<'a>(&'a mut Vec<DrawItem>);

impl Out<'_> {
    fn z(&self, band: f32) -> f32 {
        band + self.0.len() as f32 * Z_STEP
    }

    fn rect(&mut self, band: f32, x: i32, y: i32, w: i32, h: i32, color: Rgb) {
        if w > 0 && h > 0 {
            let z = self.z(band);
            self.0.push(DrawItem {
                x,
                y,
                z,
                kind: DrawKind::Rect { w, h, color },
            });
        }
    }

    fn emblem(&mut self, band: f32, x: i32, y: i32) {
        let z = self.z(band);
        self.0.push(DrawItem {
            x,
            y,
            z,
            kind: DrawKind::Emblem { w: 18, h: 18 },
        });
    }

    fn frame(&mut self, band: f32, frame: FrameId, x: i32, y: i32) {
        let (w, h) = frame.size();
        self.frame_sized(band, frame, x, y, w, h, None);
    }

    #[allow(clippy::too_many_arguments)]
    fn frame_sized(
        &mut self,
        band: f32,
        frame: FrameId,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        tint: Option<Rgb>,
    ) {
        let z = self.z(band);
        self.0.push(DrawItem {
            x,
            y,
            z,
            kind: DrawKind::Frame { frame, w, h, tint },
        });
    }

    fn text(&mut self, band: f32, font: HudFont, text: &str, x: i32, caps_y: i32, color: Rgb) {
        let z = self.z(band);
        self.0.push(DrawItem {
            x,
            y: caps_y,
            z,
            kind: DrawKind::Text {
                font,
                text: text.to_string(),
                color,
            },
        });
    }
}

/// The multiplier that turns amber into `amber_dim` (the dim boss warning).
const AMBER_TO_DIM: Rgb = Rgb(168, 152, 145);

/// Build the full draw list of one snapshot into `out` (cleared first).
pub fn build_draw_list(snapshot: &HudSnapshot, layout: &HudLayout, out: &mut Vec<DrawItem>) {
    out.clear();
    let mut o = Out(out);
    plate(&mut o, snapshot, 0, layout.carl_plate);
    plate(&mut o, snapshot, 1, layout.donut_plate);
    if let Some(boss) = snapshot.boss.as_ref() {
        boss_strip(&mut o, snapshot, boss, layout);
    } else if let Some(location) = snapshot.location.as_deref() {
        location_strip(&mut o, location, layout);
    }
    system(&mut o, snapshot, layout);
    if let Some(preview) = snapshot.preview {
        preview_tag(&mut o, preview.tag_name(), layout);
    }
}

fn plate(o: &mut Out, s: &HudSnapshot, slot: usize, rect: IRect) {
    let view = &s.plates[slot];
    let (x, y) = (rect.x, rect.y);
    o.frame(Z_FRAME, FrameId::PlateFrame, x, y);
    // Portrait well interior (the per-character colour) and the portrait.
    let carl = view.portrait == PORTRAIT_CARL;
    o.rect(
        Z_BAR,
        x + 6,
        y + 6,
        28,
        28,
        if carl { p::WELL_CARL } else { p::WELL_DONUT },
    );
    o.frame(
        Z_ICON,
        if carl {
            FrameId::PortraitCarl
        } else {
            FrameId::PortraitDonut
        },
        x + 6,
        y + 6,
    );
    // Ownership tag: shape and text, not only colour.
    let (tag_frame, tag_text, tag_color) = match view.control {
        Control::Player(1) => (FrameId::TagP1, "P1", p::HUD_INK),
        Control::Player(_) => (FrameId::TagP2, "P2", p::HUD_INK),
        Control::Ai => (FrameId::TagAi, "AI", p::CYAN),
    };
    o.frame(Z_BAR, tag_frame, x + 4, y + 36);
    let tw = text_width_px(HudFont::SilkBold, tag_text);
    o.text(
        Z_TEXT,
        HudFont::SilkBold,
        tag_text,
        x + 4 + (32 - tw) / 2,
        y + 38,
        tag_color,
    );
    o.text(Z_TEXT, HudFont::Jersey, &view.name, x + 40, y + 4, p::AMBER);
    hp_row(o, s, view, x, y);
    if let Some(mana) = view.mana {
        mana_row(o, s, mana, x, y);
    }
    abilities_row(o, s, view, x, y);
}

fn hp_row(o: &mut Out, s: &HudSnapshot, view: &PlateView, x: i32, y: i32) {
    let cracked = view.hp.cur <= 1;
    o.frame(
        Z_ICON,
        if cracked {
            FrameId::IconHeartCracked
        } else {
            FrameId::IconHeart
        },
        x + 40,
        y + 16,
    );
    o.rect(Z_BAR, x + 50, y + 16, 96, 8, p::HUD_INK);
    let n = i32::from(view.hp.max.clamp(1, 8));
    let sw = (HP_BAR_INNER_W - (n - 1) * 2) / n;
    let rem = (HP_BAR_INNER_W - (n - 1) * 2) % n;
    let (ix, iy) = (x + 51, y + 17);
    for i in 0..n {
        let sx = ix + i * (sw + 2);
        let w = sw + if i == n - 1 { rem } else { 0 };
        let flash = view.hp_flash.filter(|f| segment_flashing(f, i));
        if let Some(f) = flash {
            if s.reduced_motion {
                empty_segment(o, s, sx, iy, w);
                hatch(o, sx + 1, iy + 1, w - 2, 4);
            } else if (f.age_ticks / HP_FLASH_STEP_TICKS).is_multiple_of(2) {
                o.rect(Z_BAR, sx, iy, w, 6, p::HP_FLASH);
            } else {
                empty_segment(o, s, sx, iy, w);
            }
        } else if i < i32::from(view.hp.cur) {
            // High contrast lifts the fill from 3.4:1 to 5.5:1 (and the top
            // row to 15:1); the ink border and segment gaps stay.
            let (body, top, bottom) = if s.high_contrast {
                (p::HP_RED_HI, p::HP_FLASH, p::HP_RED)
            } else {
                (p::HP_RED, p::HP_RED_HI, p::HP_RED_LO)
            };
            o.rect(Z_BAR, sx, iy, w, 6, body);
            o.rect(Z_BAR, sx, iy, w, 1, top);
            o.rect(Z_BAR, sx, iy + 5, w, 1, bottom);
        } else {
            empty_segment(o, s, sx, iy, w);
        }
    }
}

fn segment_flashing(flash: &HpFlash, index: i32) -> bool {
    flash.age_ticks < HP_FLASH_TICKS
        && index >= i32::from(flash.lost_to)
        && index < i32::from(flash.lost_from)
}

/// An empty HP segment: `hp_empty` with a 1 px `metal_light` outline (the
/// outline keeps max HP readable by shape). High contrast draws the outline
/// `text_white` (2.3:1 becomes 13.7:1).
fn empty_segment(o: &mut Out, s: &HudSnapshot, sx: i32, iy: i32, w: i32) {
    let outline = if s.high_contrast {
        p::TEXT_WHITE
    } else {
        p::METAL_LIGHT
    };
    o.rect(Z_BAR, sx, iy, w, 6, outline);
    o.rect(Z_BAR, sx + 1, iy + 1, w - 2, 4, p::HP_EMPTY);
}

/// A static diagonal hatch: `hp_flash` on one pixel in three (reduced motion).
fn hatch(o: &mut Out, x: i32, y: i32, w: i32, h: i32) {
    for dy in 0..h {
        for dx in 0..w {
            if (dx + dy) % 3 == 0 {
                o.rect(Z_FLASH, x + dx, y + dy, 1, 1, p::HP_FLASH);
            }
        }
    }
}

fn mana_row(o: &mut Out, s: &HudSnapshot, mana: ManaView, x: i32, y: i32) {
    let depleted = mana.fill_px == 0;
    o.frame(
        Z_ICON,
        if depleted {
            FrameId::IconSparkDepleted
        } else {
            FrameId::IconSpark
        },
        x + 41,
        y + 26,
    );
    // The track outline: ink normally; a visible metal outline when empty, and
    // `magenta_hi` for the first 6 ticks after reaching 0.
    let flashing = !s.reduced_motion && mana.depleted_age.is_some_and(|age| age < MANA_FLASH_TICKS);
    // High contrast: a `metal_hi` outline always (4.4:1) and `text_white` when
    // empty, so the track reads by shape.
    let outline = if !depleted {
        if s.high_contrast {
            p::METAL_HI
        } else {
            p::HUD_INK
        }
    } else if flashing {
        p::MAGENTA_HI
    } else if s.high_contrast {
        p::TEXT_WHITE
    } else {
        p::METAL_LIGHT
    };
    o.rect(Z_BAR, x + 50, y + 26, 96, 5, outline);
    let (ix, iy) = (x + 51, y + 27);
    o.rect(Z_BAR, ix, iy, MANA_BAR_INNER_W, 3, p::HUD_INK);
    let fill = i32::from(mana.fill_px).min(MANA_BAR_INNER_W);
    for k in 0..8 {
        let seg_x = k * 12;
        let filled = (fill - seg_x).clamp(0, 10);
        if filled > 0 {
            o.rect(Z_BAR, ix + seg_x, iy, filled, 3, p::MANA[k as usize]);
        }
        o.rect(
            Z_BAR,
            ix + seg_x + filled,
            iy,
            10 - filled,
            3,
            p::MANA_EMPTY,
        );
    }
}

fn abilities_row(o: &mut Out, s: &HudSnapshot, view: &PlateView, x: i32, y: i32) {
    match view.abilities {
        AbilityRow::AiCompanion => {
            o.text(
                Z_TEXT,
                HudFont::SilkRegular,
                crate::hud::LabelId::Companion.text(),
                x + 40,
                y + 37,
                // `amber_dim` is 4.2:1; high contrast uses `amber` (10.4:1).
                if s.high_contrast {
                    p::AMBER
                } else {
                    p::AMBER_DIM
                },
            );
        }
        AbilityRow::Chips { a, b, special } => {
            chip(o, s, view.glyphs, a, x + 40, y);
            chip(o, s, view.glyphs, b, x + 93, y);
            special_socket(o, special, x, y);
        }
        AbilityRow::NoActions { special } => special_socket(o, special, x, y),
    }
}

fn special_socket(o: &mut Out, special: SpecialSlot, x: i32, y: i32) {
    match special {
        SpecialSlot::Locked => o.frame(Z_ICON, FrameId::SocketLocked, x + 147, y + 34),
    }
}

fn chip(o: &mut Out, s: &HudSnapshot, glyphs: crate::hud::GlyphSet, chip: Chip, x0: i32, y: i32) {
    let glyph = FrameId::of_glyph(glyphs.glyph(chip.action));
    let (w, h) = glyph.size();
    // A muted chip is dimmed (`amber_dim`, 4.2:1) with a zig-zag underline.
    // High contrast keeps the zig-zag (the non-hue cue) and the dimmed glyph
    // (a graphical object only needs 3:1) but raises the label and the
    // underline to `amber` (10.4:1), so muted never reads brighter than live.
    let tint = chip.muted.then_some(p::AMBER_DIM);
    o.frame_sized(Z_ICON, glyph, x0, y + 34, w, h, tint);
    let color = match (chip.muted, s.high_contrast) {
        (true, false) => p::AMBER_DIM,
        _ => p::AMBER,
    };
    o.text(
        Z_TEXT,
        HudFont::SilkRegular,
        chip.label.text(),
        x0 + 14,
        y + 37,
        color,
    );
    if chip.muted {
        // A 1 px zig-zag along the chip's bottom row: not hue alone.
        for i in 0..25 {
            o.rect(Z_FLASH, x0 + i * 2, y + 44 - (i % 2), 2, 1, color);
        }
    }
}

fn location_strip(o: &mut Out, text: &str, layout: &HudLayout) {
    let tw = text_width_px(HudFont::Jersey, text);
    let r = layout.location(tw);
    three_slice(
        o,
        r,
        FrameId::StripCapLeft,
        FrameId::StripMid,
        FrameId::StripCapRight,
    );
    o.text(
        Z_TEXT,
        HudFont::Jersey,
        text,
        r.x + (r.w - tw) / 2,
        r.y + 3,
        p::AMBER,
    );
}

fn three_slice(o: &mut Out, r: IRect, left: FrameId, mid: FrameId, right: FrameId) {
    o.frame(Z_FRAME, left, r.x, r.y);
    o.frame_sized(Z_FRAME, mid, r.x + 8, r.y, r.w - 16, r.h, None);
    o.frame(Z_FRAME, right, r.x + r.w - 8, r.y);
}

fn boss_strip(o: &mut Out, s: &HudSnapshot, boss: &crate::hud::BossView, layout: &HudLayout) {
    let rest = layout.boss();
    let by = HudLayout::boss_y(boss.slide_tick, s.reduced_motion);
    let r = IRect { y: by, ..rest };
    three_slice(
        o,
        r,
        FrameId::BossCapLeft,
        FrameId::BossMid,
        FrameId::BossCapRight,
    );
    match boss.emblem.as_ref() {
        None => o.frame(Z_ICON, FrameId::IconBossEmblem, r.x + 3, r.y + 3),
        Some(_) => {
            // The 20x20 well; the 18x18 emblem (art or placeholder) sits in
            // it at offset (1, 1).
            o.rect(Z_ICON, r.x + 3, r.y + 3, 20, 20, p::HUD_INK);
            o.rect(Z_ICON, r.x + 4, r.y + 4, 18, 18, p::MAGENTA_DK);
            o.emblem(Z_ICON, r.x + 4, r.y + 4);
        }
    }
    o.text(
        Z_TEXT,
        HudFont::Jersey,
        &boss.name,
        r.x + 27,
        r.y + 4,
        p::TEXT_WHITE,
    );
    o.rect(Z_BAR, r.x + 27, r.y + 16, 236, 7, p::HUD_INK);
    let (ix, iy) = (r.x + 28, r.y + 17);
    let fill = i32::from(boss.fill_px).min(BOSS_BAR_INNER_W);
    o.rect(Z_BAR, ix, iy, fill, 5, p::HP_RED);
    o.rect(Z_BAR, ix, iy, fill, 1, p::HP_RED_HI);
    let chip = i32::from(boss.chip_px).min(BOSS_BAR_INNER_W - fill);
    o.rect(Z_BAR, ix + fill, iy, chip, 5, p::HP_FLASH);
    // Reading aids at 25, 50 and 75 percent (not phases).
    for quarter in 1..4 {
        o.rect(
            Z_FLASH,
            ix + BOSS_BAR_INNER_W * quarter / 4,
            iy,
            1,
            5,
            p::HUD_INK,
        );
    }
    if boss.warning {
        let (w, h) = FrameId::IconWarning.size();
        let tint = (!boss.warning_bright).then_some(AMBER_TO_DIM);
        o.frame_sized(Z_ICON, FrameId::IconWarning, r.x + 266, r.y + 5, w, h, tint);
    }
}

/// Stand-ins of the 18x18 emblem, relative to its top-left corner, used while
/// `boss-f01-emblem` is not exported: a head in profile (mop, red-rimmed eye,
/// open mouth) on the well's magenta, from palette colours.
pub fn emblem_placeholder_rects() -> Vec<PlaceholderRect> {
    vec![
        (2, 2, 9, 5, p::TEXT_WHITE),
        (3, 7, 11, 7, p::TEXT_WHITE),
        (12, 5, 3, 7, p::TEXT_WHITE),
        (10, 6, 2, 2, p::HP_RED),
        (7, 11, 7, 2, p::HUD_INK),
    ]
}

fn system(o: &mut Out, s: &HudSnapshot, layout: &HudLayout) {
    let face = match s.face {
        FaceFrame::Idle => FrameId::CrtIdle,
        FaceFrame::Blink => FrameId::CrtBlink,
        FaceFrame::Notice => FrameId::CrtNotice,
    };
    o.frame(Z_ICON, face, layout.face.x, layout.face.y);
    if let Some(slot) = s.reconnect {
        // D6: a persistent pause message in the notice frame, no slide.
        let (x, y) = (layout.notice.x, layout.notice.y);
        o.frame(Z_FRAME, FrameId::NoticeFrame, x, y);
        o.text(
            Z_TEXT,
            HudFont::SilkBold,
            "GAME PAUSED",
            x + 7,
            y + 6,
            p::MAGENTA_HI,
        );
        o.text(
            Z_TEXT,
            HudFont::Jersey,
            &reconnect_text(slot),
            x + 7,
            y + 15,
            p::AMBER,
        );
    } else if let Some(notice) = s.notice {
        let dx = notice_dx(notice.age_ticks, s.reduced_motion);
        let (x, y) = (layout.notice.x + dx, layout.notice.y);
        o.frame(Z_FRAME, FrameId::NoticeFrame, x, y);
        o.text(
            Z_TEXT,
            HudFont::SilkBold,
            "SYSTEM NOTICE",
            x + 7,
            y + 6,
            p::MAGENTA_HI,
        );
        o.text(
            Z_TEXT,
            HudFont::Jersey,
            notice.headline.text(),
            x + 7,
            y + 15,
            p::AMBER,
        );
    }
}

/// The pause message headline for player `slot` (1 or 2).
pub fn reconnect_text(slot: u8) -> String {
    format!("RECONNECT P{slot} CONTROLLER")
}

fn preview_tag(o: &mut Out, tag_name: &str, layout: &HudLayout) {
    let text = format!("PREVIEW {tag_name}");
    let tw = text_width_px(HudFont::SilkBold, &text);
    let r = layout.preview_tag(tw);
    o.rect(Z_PREVIEW, r.x, r.y, r.w, r.h, p::HUD_INK);
    // A 2-on / 2-off amber dash stripe on the top and bottom rows.
    let mut dx = 0;
    while dx < r.w {
        let w = 2.min(r.w - dx);
        o.rect(Z_PREVIEW, r.x + dx, r.y, w, 1, p::AMBER);
        o.rect(Z_PREVIEW, r.x + dx, r.y + r.h - 1, w, 1, p::AMBER);
        dx += 4;
    }
    o.text(
        Z_PREVIEW,
        HudFont::SilkBold,
        &text,
        r.x + 5,
        r.y + 3,
        p::AMBER,
    );
}

// ------------------------------------------------------------- placeholders

/// A placeholder rect relative to its frame origin.
pub type PlaceholderRect = (i32, i32, i32, i32, Rgb);

fn bitmap(rows: &[&str], x: i32, y: i32, color: Rgb, out: &mut Vec<PlaceholderRect>) {
    for (dy, row) in rows.iter().enumerate() {
        for (dx, c) in row.chars().enumerate() {
            if c == '#' {
                out.push((x + dx as i32, y + dy as i32, 1, 1, color));
            }
        }
    }
}

/// The 3 px worn-metal ring of plates and strips, filled with `fill`.
fn ring(w: i32, h: i32, fill: Rgb, out: &mut Vec<PlaceholderRect>) {
    out.push((0, 0, w, h, p::HUD_INK));
    out.push((1, 1, w - 2, h - 2, p::METAL_MID));
    out.push((1, 1, w - 2, 1, p::METAL_HI));
    out.push((1, 1, 1, h - 2, p::METAL_HI));
    out.push((2, 2, w - 4, h - 4, p::HUD_INK));
    out.push((3, 3, w - 6, h - 6, fill));
}

/// Palette-coloured stand-ins of the spec sizes, used while `hud_hybrid_a`
/// is not exported. They are layout placeholders, not art.
pub fn placeholder_rects(frame: FrameId, w: i32, h: i32) -> Vec<PlaceholderRect> {
    let mut v = Vec::new();
    match frame {
        FrameId::PlateFrame => {
            ring(w, h, p::PLATE_FILL, &mut v);
            v.push((4, 4, 32, 32, p::HUD_INK));
            v.push((5, 5, 30, 30, p::BRASS));
            for (cx, cy, c) in [
                (4, 4, p::BRASS_HI),
                (34, 4, p::BRASS),
                (4, 34, p::BRASS),
                (34, 34, p::BRASS_DARK),
            ] {
                v.push((cx, cy, 2, 2, c));
            }
        }
        FrameId::StripCapLeft
        | FrameId::StripCapRight
        | FrameId::BossCapLeft
        | FrameId::BossCapRight => {
            ring(w, h, p::PLATE_FILL, &mut v);
            v.push((4, h / 2 - 3, 4, 6, p::MAGENTA_DK));
            v.push((5, h / 2 - 2, 2, 4, p::MAGENTA_HI));
        }
        FrameId::StripMid | FrameId::BossMid => {
            v.push((0, 0, w, h, p::PLATE_FILL));
            v.push((0, 0, w, 1, p::HUD_INK));
            v.push((0, 1, w, 1, p::METAL_HI));
            v.push((0, 2, w, 1, p::HUD_INK));
            v.push((0, h - 3, w, 1, p::HUD_INK));
            v.push((0, h - 2, w, 1, p::METAL_MID));
            v.push((0, h - 1, w, 1, p::HUD_INK));
        }
        FrameId::NoticeFrame => {
            v.push((0, 0, w, h, p::HUD_INK));
            v.push((1, 1, w - 2, h - 2, p::MAGENTA));
            v.push((2, 2, w - 4, h - 4, p::HUD_INK));
            v.push((3, 3, w - 6, h - 6, p::NOTICE_BG));
        }
        FrameId::PortraitCarl => v.push((0, 0, w, h, p::BRASS)),
        FrameId::PortraitDonut => v.push((0, 0, w, h, p::MAGENTA)),
        FrameId::CrtIdle | FrameId::CrtBlink | FrameId::CrtNotice => {
            v.push((2, 4, 16, 14, p::BRASS));
            v.push((3, 6, 9, 7, p::SCREEN_BG));
            v.push((13, 6, 4, 10, p::BRASS_DARK));
            v.push((5, 0, 1, 4, p::METAL_HI));
            v.push((13, 0, 1, 4, p::METAL_HI));
            let eye = match frame {
                FrameId::CrtBlink => vec![(4, 9, 3, 1), (8, 9, 3, 1)],
                _ => vec![(4, 8, 2, 2), (9, 8, 2, 2)],
            };
            for (x, y, w, h) in eye {
                v.push((x, y, w, h, p::CRT_FACE_HI));
            }
            if frame == FrameId::CrtNotice {
                v.push((4, 11, 7, 1, p::CRT_FACE_HI));
            }
        }
        FrameId::GlyphA
        | FrameId::GlyphB
        | FrameId::GlyphX
        | FrameId::GlyphY
        | FrameId::GlyphCross
        | FrameId::GlyphCircle
        | FrameId::GlyphSquare
        | FrameId::GlyphTriangle => {
            v.push((0, 0, 11, 11, p::HUD_INK));
            v.push((1, 1, 9, 4, p::METAL_LIGHT));
            v.push((1, 5, 9, 5, p::METAL_MID));
            let rows: [&str; 5] = match frame {
                FrameId::GlyphA => [".###.", "#...#", "#####", "#...#", "#...#"],
                FrameId::GlyphB => ["####.", "#...#", "####.", "#...#", "####."],
                FrameId::GlyphX => ["#...#", ".#.#.", "..#..", ".#.#.", "#...#"],
                FrameId::GlyphY => ["#...#", ".#.#.", "..#..", "..#..", "..#.."],
                FrameId::GlyphCross => ["##.##", "#####", ".###.", "#####", "##.##"],
                FrameId::GlyphCircle => [".###.", "#...#", "#...#", "#...#", ".###."],
                FrameId::GlyphSquare => ["#####", "#...#", "#...#", "#...#", "#####"],
                _ => ["..#..", ".#.#.", ".#.#.", "#...#", "#####"],
            };
            bitmap(&rows, 3, 3, p::TEXT_WHITE, &mut v);
        }
        FrameId::SocketLocked => {
            v.push((0, 0, 11, 11, p::HUD_INK));
            v.push((1, 1, 9, 9, p::METAL_DARK));
            v.push((4, 2, 3, 1, p::METAL_LIGHT));
            v.push((3, 3, 1, 2, p::METAL_LIGHT));
            v.push((7, 3, 1, 2, p::METAL_LIGHT));
            v.push((3, 5, 5, 4, p::METAL_LIGHT));
        }
        FrameId::IconHeart | FrameId::IconHeartCracked => {
            bitmap(
                &[
                    ".##.##.", "#######", "#######", "#######", ".#####.", "..###..", "...#...",
                ],
                0,
                0,
                p::HP_RED,
                &mut v,
            );
            v.push((1, 1, 1, 1, p::HP_RED_HI));
            v.push((2, 1, 1, 1, p::HP_RED_HI));
            if frame == FrameId::IconHeartCracked {
                for (x, y) in [(3, 1), (4, 2), (3, 3), (4, 4)] {
                    v.push((x, y, 1, 1, p::HUD_INK));
                }
            }
        }
        FrameId::IconSpark => {
            bitmap(
                &["..#..", ".###.", "#####", ".###.", "..#.."],
                0,
                0,
                p::MANA[0],
                &mut v,
            );
            v.push((2, 2, 1, 1, p::CYAN));
        }
        FrameId::IconSparkDepleted => {
            for (x, y) in [
                (2, 0),
                (1, 1),
                (3, 1),
                (0, 2),
                (4, 2),
                (1, 3),
                (3, 3),
                (2, 4),
            ] {
                v.push((x, y, 1, 1, p::METAL_HI));
            }
            for i in 0..5 {
                v.push((i, 4 - i, 1, 1, p::AMBER_DIM));
            }
        }
        FrameId::IconWarning => {
            for r in 0..h {
                let half = (r * 5) / (h - 1);
                v.push((5 - half, r, 2 * half + 1, 1, p::AMBER));
            }
            v.push((5, 4, 1, 3, p::HUD_INK));
            v.push((5, 8, 1, 1, p::HUD_INK));
        }
        FrameId::IconBossEmblem => {
            v.push((0, 0, 20, 20, p::HUD_INK));
            v.push((1, 1, 18, 18, p::MAGENTA_DK));
            v.push((5, 6, 10, 6, p::TEXT_WHITE));
            v.push((6, 12, 8, 2, p::TEXT_WHITE));
            v.push((7, 8, 2, 2, p::HUD_INK));
            v.push((11, 8, 2, 2, p::HUD_INK));
        }
        FrameId::TagP1 => v.push((0, 0, w, h, p::AMBER)),
        FrameId::TagP2 => {
            v.push((0, 0, w, h, p::MAGENTA_HI));
            for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
                v.push((x, y, 1, 1, p::PLATE_FILL));
            }
        }
        FrameId::TagAi => {
            v.push((0, 0, w, h, p::CYAN));
            v.push((1, 1, w - 2, h - 2, p::HUD_INK));
        }
    }
    v
}

// --------------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::{GlyphSet, HudPreview, build_snapshot};

    fn items(preview: Option<HudPreview>, deck: bool) -> Vec<DrawItem> {
        let layout = if deck {
            HudLayout::new(400, 308)
        } else {
            HudLayout::new(360, 288)
        };
        let mut out = Vec::new();
        build_draw_list(
            &build_snapshot(&crate::hud::test_inputs(4, 4), preview, 0),
            &layout,
            &mut out,
        );
        out
    }

    fn frames(items: &[DrawItem]) -> Vec<FrameId> {
        items
            .iter()
            .filter_map(|i| match i.kind {
                DrawKind::Frame { frame, .. } => Some(frame),
                _ => None,
            })
            .collect()
    }

    fn texts(items: &[DrawItem]) -> Vec<&str> {
        items
            .iter()
            .filter_map(|i| match &i.kind {
                DrawKind::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn frame_names_are_unique_and_sizes_match_the_spec() {
        let mut names: Vec<_> = FrameId::ALL.iter().map(|f| f.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), FrameId::ALL.len());
        assert!(names.contains(&"plate_frame") && names.contains(&"strip_loc_mid"));
        assert_eq!(FrameId::PlateFrame.size(), (164, 48));
        assert_eq!(FrameId::NoticeFrame.size(), (204, 32));
        assert_eq!(FrameId::CrtIdle.size(), (20, 18));
        assert_eq!(FrameId::GlyphX.size(), (11, 11));
        assert_eq!(FrameId::TagP2.size(), (32, 9));
    }

    #[test]
    fn placeholders_stay_inside_their_frame() {
        for frame in FrameId::ALL {
            let (w, h) = frame.size();
            for (x, y, rw, rh, _) in placeholder_rects(frame, w, h) {
                assert!(
                    x >= 0 && y >= 0 && x + rw <= w && y + rh <= h,
                    "{frame:?}: ({x},{y},{rw},{rh}) outside {w}x{h}"
                );
            }
        }
    }

    #[test]
    fn normal_play_draws_the_plain_hud_only() {
        for deck in [false, true] {
            let list = items(None, deck);
            let f = frames(&list);
            assert!(f.contains(&FrameId::TagP1) && f.contains(&FrameId::TagAi));
            assert!(!f.contains(&FrameId::TagP2));
            assert!(!f.contains(&FrameId::IconSpark) && !f.contains(&FrameId::IconSparkDepleted));
            assert!(!f.contains(&FrameId::IconBossEmblem) && !f.contains(&FrameId::NoticeFrame));
            let t = texts(&list);
            assert!(t.contains(&"FLOOR 01") && t.contains(&"COMPANION"));
            assert!(!t.iter().any(|s| s.starts_with("PREVIEW")));
            assert!(t.contains(&"PUNCH") && t.contains(&"KICK"));
        }
    }

    #[test]
    fn story_corridor_draws_no_chips_but_keeps_the_locked_socket() {
        let mut real = crate::hud::test_inputs(4, 4);
        real.melee_live = false;
        let mut list = Vec::new();
        build_draw_list(
            &build_snapshot(&real, None, 0),
            &HudLayout::new(360, 288),
            &mut list,
        );
        let f = frames(&list);
        assert!(!f.iter().any(|f| f.is_glyph()));
        assert_eq!(f.iter().filter(|f| **f == FrameId::SocketLocked).count(), 1);
        let t = texts(&list);
        assert!(!t.contains(&"PUNCH") && !t.contains(&"KICK"));
        assert!(t.contains(&"COMPANION") && t.contains(&"CARL"));
    }

    #[test]
    fn control_truth_no_b_circle_jump_or_ai_glyphs() {
        for glyphs in [GlyphSet::Xbox, GlyphSet::DualSense] {
            for preview in std::iter::once(None).chain(HudPreview::ALL.into_iter().map(Some)) {
                let mut real = crate::hud::test_inputs(4, 4);
                real.carl_glyphs = glyphs;
                real.donut_glyphs = glyphs;
                let snapshot = build_snapshot(&real, preview, 0);
                let mut list = Vec::new();
                build_draw_list(&snapshot, &HudLayout::new(360, 288), &mut list);
                let glyph_frames: Vec<_> =
                    frames(&list).into_iter().filter(|f| f.is_glyph()).collect();
                let allowed = match glyphs {
                    GlyphSet::Xbox => [FrameId::GlyphX, FrameId::GlyphY],
                    GlyphSet::DualSense => [FrameId::GlyphSquare, FrameId::GlyphTriangle],
                };
                assert!(glyph_frames.iter().all(|f| allowed.contains(f)));
                // Two chips per player-controlled plate, none on an AI plate.
                let chips: usize = snapshot
                    .plates
                    .iter()
                    .filter(|p| matches!(p.abilities, AbilityRow::Chips { .. }))
                    .count();
                assert_eq!(glyph_frames.len(), chips * 2, "{preview:?} {glyphs:?}");
                // The locked special is a socket, one per chips plate.
                let sockets = frames(&list)
                    .iter()
                    .filter(|f| **f == FrameId::SocketLocked)
                    .count();
                assert_eq!(sockets, chips);
            }
        }
    }

    #[test]
    fn dualsense_draws_square_and_triangle() {
        let mut real = crate::hud::test_inputs(4, 4);
        real.carl_glyphs = GlyphSet::DualSense;
        real.donut_glyphs = GlyphSet::DualSense;
        let mut list = Vec::new();
        build_draw_list(
            &build_snapshot(&real, None, 0),
            &HudLayout::new(360, 288),
            &mut list,
        );
        let f = frames(&list);
        assert!(f.contains(&FrameId::GlyphSquare) && f.contains(&FrameId::GlyphTriangle));
        assert!(!f.contains(&FrameId::GlyphX));
    }

    #[test]
    fn fixtures_draw_their_states_and_always_the_preview_tag() {
        for preview in HudPreview::ALL {
            let list = items(Some(preview), false);
            let t = texts(&list);
            assert!(
                t.iter()
                    .any(|s| *s == format!("PREVIEW {}", preview.tag_name()))
            );
            let f = frames(&list);
            match preview {
                HudPreview::PreUnlock => {
                    assert!(f.contains(&FrameId::TagP2));
                    assert!(!f.contains(&FrameId::IconSpark));
                    assert!(t.contains(&"SWAT") && t.contains(&"POUNCE"));
                }
                HudPreview::PostUnlock => {
                    assert!(f.contains(&FrameId::IconSpark));
                    assert!(t.contains(&"MISSILE"));
                }
                HudPreview::Depleted => {
                    assert!(f.contains(&FrameId::IconSparkDepleted));
                    assert!(f.contains(&FrameId::NoticeFrame) && t.contains(&"LAUNCHED!"));
                    assert!(f.contains(&FrameId::IconHeart));
                }
                HudPreview::Boss => {
                    assert!(f.contains(&FrameId::IconBossEmblem));
                    assert!(f.contains(&FrameId::IconWarning));
                    assert!(t.contains(&"TEST BRUTE") && !t.contains(&"FLOOR 01"));
                    assert!(t.contains(&"BOSS INCOMING"));
                    assert!(f.contains(&FrameId::TagAi));
                }
                HudPreview::Coop => {
                    assert!(f.contains(&FrameId::TagP1) && f.contains(&FrameId::TagP2));
                }
            }
        }
    }

    #[test]
    fn low_hp_swaps_to_the_cracked_heart() {
        let mut list = Vec::new();
        build_draw_list(
            &build_snapshot(&crate::hud::test_inputs(1, 4), None, 0),
            &HudLayout::new(360, 288),
            &mut list,
        );
        let f = frames(&list);
        assert_eq!(
            f.iter()
                .filter(|f| **f == FrameId::IconHeartCracked)
                .count(),
            1
        );
        assert_eq!(f.iter().filter(|f| **f == FrameId::IconHeart).count(), 1);
    }

    #[test]
    fn hp_segments_are_one_per_point_and_empty_ones_have_an_outline() {
        let count_color = |hp, color: Rgb| {
            let mut list = Vec::new();
            build_draw_list(
                &build_snapshot(&crate::hud::test_inputs(hp, 4), None, 0),
                &HudLayout::new(360, 288),
                &mut list,
            );
            // Carl's bar rows only (the plate is at y = 8, the bar at +17).
            list.iter()
                .filter(|i| {
                    i.x < 164
                        && matches!(
                            i.kind,
                            DrawKind::Rect { color: c, h: 6 | 4, .. } if c == color
                        )
                        && (25..=30).contains(&i.y)
                })
                .count()
        };
        assert_eq!(count_color(4, p::HP_RED), 4);
        assert_eq!(count_color(2, p::HP_RED), 2);
        assert_eq!(count_color(2, p::METAL_LIGHT), 2);
        assert_eq!(count_color(0, p::METAL_LIGHT), 4);
    }

    #[test]
    fn every_item_stays_on_the_canvas_inside_its_zone() {
        for deck in [false, true] {
            let view_h = if deck { 400 } else { 360 };
            for preview in std::iter::once(None).chain(HudPreview::ALL.into_iter().map(Some)) {
                for item in items(preview, deck) {
                    let (w, h) = match &item.kind {
                        DrawKind::Rect { w, h, .. }
                        | DrawKind::Frame { w, h, .. }
                        | DrawKind::Emblem { w, h } => (*w, *h),
                        DrawKind::Text { font, text, .. } => (
                            text_width_px(*font, text),
                            if *font == HudFont::Jersey { 10 } else { 5 },
                        ),
                    };
                    assert!(item.x >= 0 && item.y >= 0, "{item:?}");
                    assert!(item.x + w <= 640 && item.y + h <= view_h, "{item:?}");
                    // Nothing of the HUD enters the action lane [60, walk_y + 12].
                    let walk = if deck { 308 } else { 288 };
                    let bottom = item.y + h;
                    assert!(
                        bottom <= 60 || item.y > walk + 12,
                        "{item:?} in the action lane"
                    );
                }
            }
        }
    }

    #[test]
    fn z_bands_order_frames_below_text_and_the_preview_on_top() {
        let list = items(Some(HudPreview::Boss), false);
        let max_non_preview = list
            .iter()
            .filter(|i| i.z < Z_PREVIEW)
            .map(|i| i.z)
            .fold(0.0, f32::max);
        assert!(max_non_preview < Z_PREVIEW);
        assert!(list.iter().filter(|i| i.z >= Z_PREVIEW).count() > 3);
        for item in &list {
            match &item.kind {
                DrawKind::Text { .. } if item.z < Z_PREVIEW => assert!(item.z >= Z_TEXT),
                DrawKind::Frame {
                    frame: FrameId::PlateFrame | FrameId::NoticeFrame,
                    ..
                } => {
                    assert!(item.z < Z_BAR)
                }
                _ => {}
            }
        }
        // Distinct z inside the list so no two items z-fight.
        let mut zs: Vec<_> = list.iter().map(|i| i.z.to_bits()).collect();
        zs.sort_unstable();
        zs.dedup();
        assert_eq!(zs.len(), list.len());
    }

    #[test]
    fn the_reconnect_message_is_a_static_notice_frame_that_fits() {
        for slot in [1u8, 2] {
            let mut real = crate::hud::test_inputs(4, 4);
            real.reconnect = Some(slot);
            let snapshot = build_snapshot(&real, None, 0);
            let layout = HudLayout::new(360, 288);
            let mut list = Vec::new();
            build_draw_list(&snapshot, &layout, &mut list);
            let t = texts(&list);
            let wanted = format!("RECONNECT P{slot} CONTROLLER");
            assert!(t.contains(&wanted.as_str()) && t.contains(&"GAME PAUSED"));
            assert!(frames(&list).contains(&FrameId::NoticeFrame));
            assert!(frames(&list).contains(&FrameId::CrtNotice));
            // Inside the 204 px notice frame, with the 7 px inset on both sides.
            assert!(text_width_px(HudFont::Jersey, &wanted) <= layout.notice.w - 14);
            // The panel never slides and stays below the action lane.
            let panel = list
                .iter()
                .find(|i| {
                    matches!(
                        i.kind,
                        DrawKind::Frame {
                            frame: FrameId::NoticeFrame,
                            ..
                        }
                    )
                })
                .unwrap();
            assert_eq!(panel.x, layout.notice.x);
            assert!(panel.y >= crate::hud::ACTION_LANE_TOP);
        }
        // A reconnect request replaces a queued notice rather than stacking.
        let mut real = crate::hud::test_inputs(4, 4);
        real.reconnect = Some(1);
        real.notice = Some(crate::hud::NoticeView {
            headline: crate::hud::LabelId::Launched,
            age_ticks: 20,
        });
        let mut list = Vec::new();
        build_draw_list(
            &build_snapshot(&real, None, 0),
            &HudLayout::new(360, 288),
            &mut list,
        );
        assert!(!texts(&list).contains(&"LAUNCHED!"));
    }

    #[test]
    fn each_plate_draws_its_own_pads_glyphs() {
        let mut real = crate::hud::test_inputs(4, 4);
        real.carl_control = Control::Player(1);
        real.donut_control = Control::Player(2);
        real.donut_chips = true;
        real.carl_glyphs = GlyphSet::Xbox;
        real.donut_glyphs = GlyphSet::DualSense;
        let snapshot = build_snapshot(&real, None, 0);
        let layout = HudLayout::new(360, 288);
        let mut list = Vec::new();
        build_draw_list(&snapshot, &layout, &mut list);
        let mid_x = crate::hud::VIEW_WIDTH / 2;
        let glyph_at = |left: bool| -> Vec<FrameId> {
            list.iter()
                .filter_map(|i| match i.kind {
                    DrawKind::Frame { frame, .. } if frame.is_glyph() && (i.x < mid_x) == left => {
                        Some(frame)
                    }
                    _ => None,
                })
                .collect()
        };
        assert_eq!(glyph_at(true), [FrameId::GlyphX, FrameId::GlyphY]);
        assert_eq!(
            glyph_at(false),
            [FrameId::GlyphSquare, FrameId::GlyphTriangle]
        );
        // The tags are real, not fixture.
        let t = texts(&list);
        assert!(t.contains(&"P1") && t.contains(&"P2") && !t.contains(&"AI"));
    }

    #[test]
    fn a_human_donut_with_no_chips_shows_only_her_socket() {
        let mut real = crate::hud::test_inputs(4, 4);
        real.carl_control = Control::Player(1);
        real.donut_control = Control::Player(2);
        let snapshot = build_snapshot(&real, None, 0);
        let mut list = Vec::new();
        build_draw_list(&snapshot, &HudLayout::new(360, 288), &mut list);
        let glyphs = frames(&list).iter().filter(|f| f.is_glyph()).count();
        assert_eq!(glyphs, 2, "Carl's two chips only");
        let sockets = frames(&list)
            .iter()
            .filter(|f| **f == FrameId::SocketLocked)
            .count();
        assert_eq!(sockets, 2);
    }

    #[test]
    fn the_notice_slides_in_integer_pixels_and_never_overlaps_the_lane() {
        for age in 0..140 {
            let mut real = crate::hud::test_inputs(4, 4);
            real.notice = Some(crate::hud::NoticeView {
                headline: crate::hud::LabelId::Launched,
                age_ticks: age,
            });
            let mut list = Vec::new();
            build_draw_list(
                &build_snapshot(&real, None, 0),
                &HudLayout::new(360, 288),
                &mut list,
            );
            let panel = list
                .iter()
                .find(|i| {
                    matches!(
                        i.kind,
                        DrawKind::Frame {
                            frame: FrameId::NoticeFrame,
                            ..
                        }
                    )
                })
                .unwrap();
            assert!(panel.x >= 404 && panel.x <= 428 && panel.y == 320);
        }
    }

    #[test]
    fn reduced_motion_hatches_the_flash_and_never_blinks() {
        let mut real = crate::hud::test_inputs(3, 4);
        real.reduced_motion = true;
        real.hp_flash = Some(HpFlash {
            lost_from: 4,
            lost_to: 3,
            age_ticks: 0,
        });
        let mut list = Vec::new();
        build_draw_list(
            &build_snapshot(&real, None, 239),
            &HudLayout::new(360, 288),
            &mut list,
        );
        assert!(!frames(&list).contains(&FrameId::CrtBlink));
        assert!(list.iter().any(|i| i.z >= Z_FLASH
            && matches!(i.kind, DrawKind::Rect { w: 1, h: 1, color, .. } if color == p::HP_FLASH)));
        // The normal-motion flash fills the whole segment at age 0 instead.
        real.reduced_motion = false;
        build_draw_list(
            &build_snapshot(&real, None, 0),
            &HudLayout::new(360, 288),
            &mut list,
        );
        assert!(list.iter().any(
            |i| matches!(i.kind, DrawKind::Rect { w: 22, h: 6, color, .. } if color == p::HP_FLASH)
        ));
    }

    #[test]
    fn mana_rows_fill_left_to_right_and_flag_empty_by_shape() {
        let rects_of = |fixture| {
            items(Some(fixture), false)
                .into_iter()
                .filter(|i| i.x >= 468)
                .collect::<Vec<_>>()
        };
        let full = rects_of(HudPreview::PostUnlock);
        // 8 filled segments of 10 px, one per mana colour.
        for color in p::MANA {
            assert_eq!(
                full.iter()
                    .filter(|i| matches!(i.kind, DrawKind::Rect { w: 10, h: 3, color: c, .. } if c == color))
                    .count(),
                1
            );
        }
        let empty = rects_of(HudPreview::Depleted);
        // Depleted: a visible outline (not ink) around the track, and every
        // segment in the empty colour.
        assert!(empty.iter().any(|i| matches!(i.kind, DrawKind::Rect { w: 96, h: 5, color, .. } if color == p::METAL_LIGHT || color == p::MAGENTA_HI)));
        assert_eq!(
            empty
                .iter()
                .filter(|i| matches!(i.kind, DrawKind::Rect { w: 10, h: 3, color, .. } if color == p::MANA_EMPTY))
                .count(),
            8
        );
        // The muted chip carries the zig-zag hatch.
        assert!(
            empty
                .iter()
                .any(|i| matches!(i.kind, DrawKind::Frame { tint: Some(_), .. }))
        );
    }

    fn hc_list(preview: Option<HudPreview>, hp: u8, high_contrast: bool) -> Vec<DrawItem> {
        hc_list_motion(preview, hp, high_contrast, false)
    }

    fn hc_list_motion(
        preview: Option<HudPreview>,
        hp: u8,
        high_contrast: bool,
        reduced_motion: bool,
    ) -> Vec<DrawItem> {
        let mut real = crate::hud::test_inputs(hp, 4);
        real.high_contrast = high_contrast;
        real.reduced_motion = reduced_motion;
        real.hp_flash = None;
        let mut list = Vec::new();
        build_draw_list(
            &build_snapshot(&real, preview, 0),
            &HudLayout::new(360, 288),
            &mut list,
        );
        list
    }

    /// Contrast ratio of two colours (WCAG relative luminance).
    fn contrast(a: Rgb, b: Rgb) -> f64 {
        let lum = |c: Rgb| {
            let f = |v: u8| {
                let v = f64::from(v) / 255.0;
                if v <= 0.03928 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * f(c.0) + 0.7152 * f(c.1) + 0.0722 * f(c.2)
        };
        let (hi, lo) = (lum(a).max(lum(b)), lum(a).min(lum(b)));
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn high_contrast_only_recolours_and_never_moves_anything() {
        for preview in std::iter::once(None).chain(HudPreview::ALL.into_iter().map(Some)) {
            for hp in [1u8, 2, 4] {
                let (normal, high) = (hc_list(preview, hp, false), hc_list(preview, hp, true));
                assert_eq!(normal.len(), high.len(), "{preview:?} hp {hp}");
                for (a, b) in normal.iter().zip(&high) {
                    assert_eq!((a.x, a.y, a.z), (b.x, b.y, b.z), "{preview:?}");
                    let size = |k: &DrawKind| match k {
                        DrawKind::Rect { w, h, .. }
                        | DrawKind::Frame { w, h, .. }
                        | DrawKind::Emblem { w, h } => (*w, *h),
                        DrawKind::Text { text, font, .. } => (text_width_px(*font, text), 0),
                    };
                    assert_eq!(size(&a.kind), size(&b.kind), "{preview:?}");
                }
            }
        }
    }

    #[test]
    fn high_contrast_off_is_the_unchanged_default() {
        // The default list still carries the dim colours the spec defines.
        let normal = hc_list(Some(HudPreview::Depleted), 4, false);
        assert!(
            normal
                .iter()
                .any(|i| matches!(i.kind, DrawKind::Frame { tint: Some(_), .. }))
        );
        assert!(
            normal
                .iter()
                .any(|i| matches!(i.kind, DrawKind::Rect { color, .. } if color == p::METAL_LIGHT))
        );
    }

    #[test]
    fn high_contrast_text_is_at_least_seven_to_one_on_its_fill() {
        for preview in [
            None,
            Some(HudPreview::PreUnlock),
            Some(HudPreview::Depleted),
        ] {
            for hp in [1u8, 4] {
                for i in hc_list(preview, hp, true) {
                    let DrawKind::Text {
                        font,
                        ref text,
                        color,
                    } = i.kind
                    else {
                        continue;
                    };
                    // Tag texts sit on their own solid tag, PREVIEW on ink,
                    // and the notice label on the notice fill; the rest is
                    // on the plate.
                    if matches!(text.as_str(), "P1" | "P2" | "AI") || text.starts_with("PREVIEW") {
                        continue;
                    }
                    let _ = font;
                    // The notice panel (y >= 320) has its own fill.
                    let fill = if i.y >= 320 {
                        p::NOTICE_BG
                    } else {
                        p::PLATE_FILL
                    };
                    assert!(
                        contrast(color, fill) >= 7.0,
                        "{preview:?}: {text:?} is {:.1}:1",
                        contrast(color, fill)
                    );
                }
            }
        }
        // And the normal list still has the 4.2:1 dim text high contrast removes.
        assert!(
            hc_list(None, 4, false)
                .iter()
                .any(|i| matches!(&i.kind, DrawKind::Text { color, .. } if *color == p::AMBER_DIM))
        );
    }

    #[test]
    fn high_contrast_muted_chip_is_never_more_emphatic_than_a_live_chip() {
        let high = hc_list(Some(HudPreview::Depleted), 1, true);
        // No dim text in high contrast (4.2:1 is below 7:1).
        assert!(
            !high
                .iter()
                .any(|i| matches!(&i.kind, DrawKind::Text { color, .. } if *color == p::AMBER_DIM))
        );
        // The muted chip (MISSILE) has the dimmed glyph (luminance, not hue)
        // and exactly one tinted frame; the live chip (POUNCE) is untinted.
        let tinted = high
            .iter()
            .filter(
                |i| matches!(i.kind, DrawKind::Frame { tint: Some(t), .. } if t == p::AMBER_DIM),
            )
            .count();
        assert_eq!(tinted, 1);
        // Its label and the zig-zag underline are `amber`, never `text_white`
        // (13.7:1, brighter than the live chip's 10.4:1) and still >= 7:1.
        assert!(
            !high.iter().any(|i| i.z >= Z_FLASH
                && matches!(i.kind, DrawKind::Rect { w: 2, h: 1, color, .. } if color == p::TEXT_WHITE))
        );
        let strokes = |list: &[DrawItem], c: Rgb| {
            list.iter()
                .filter(|i| {
                    i.z >= Z_FLASH
                        && matches!(i.kind, DrawKind::Rect { w: 2, h: 1, color, .. } if color == c)
                })
                .count()
        };
        let normal = hc_list(Some(HudPreview::Depleted), 1, false);
        // 25 dim strokes in normal become 25 more `amber` strokes in high contrast.
        assert_eq!(strokes(&normal, p::AMBER_DIM), 25);
        assert_eq!(strokes(&high, p::AMBER), strokes(&normal, p::AMBER) + 25);
        assert_eq!(strokes(&high, p::AMBER_DIM), 0);
        assert!(contrast(p::AMBER, p::PLATE_FILL) >= 7.0);
        assert!(contrast(p::AMBER_DIM, p::PLATE_FILL) >= 3.0);
        // The zig-zag is the only extra mark: a live-only list has none.
        let live = hc_list(None, 4, true);
        assert!(
            !live
                .iter()
                .any(|i| i.z >= Z_FLASH && matches!(i.kind, DrawKind::Rect { w: 2, h: 1, .. }))
        );
    }

    #[test]
    fn high_contrast_keeps_outlines_white_and_fills_lighter() {
        let high = hc_list(Some(HudPreview::Depleted), 1, true);
        // Empty HP segments (hp 1 of 4) use white outlines. The empty mana
        // track does too once the (normal-motion) magenta flash is not shown.
        assert!(high.iter().any(|i| matches!(i.kind, DrawKind::Rect { w: 22, h: 6, color, .. } if color == p::TEXT_WHITE)));
        let calm = hc_list_motion(Some(HudPreview::Depleted), 1, true, true);
        assert!(calm.iter().any(|i| matches!(i.kind, DrawKind::Rect { w: 96, h: 5, color, .. } if color == p::TEXT_WHITE)));
        // A track that is not empty gets a metal_hi outline instead of ink.
        let post = hc_list(Some(HudPreview::PostUnlock), 4, true);
        assert!(post.iter().any(
            |i| matches!(i.kind, DrawKind::Rect { w: 96, h: 5, color, .. } if color == p::METAL_HI)
        ));
        // Full HP fill is the lighter red.
        let full = hc_list(None, 4, true);
        assert!(full.iter().any(
            |i| matches!(i.kind, DrawKind::Rect { w: 22, h: 6, color, .. } if color == p::HP_RED_HI)
        ));
        assert!(!full.iter().any(|i| matches!(i.kind, DrawKind::Rect { color, .. } if color == p::HP_RED && i.z < Z_ICON && i.y == 25)));
        assert!(contrast(p::HP_RED_HI, p::PLATE_FILL) >= 5.0);
        assert!(contrast(p::TEXT_WHITE, p::PLATE_FILL) >= 13.0);
    }

    #[test]
    fn a_real_boss_strip_draws_its_own_emblem_placeholder_instead_of_the_skull_plate() {
        use crate::hud::{BossView, EmblemId};
        let mut real = crate::hud::test_inputs(4, 4);
        real.boss = Some(BossView {
            name: "THE JUICER".to_owned(),
            fill_px: 234,
            chip_px: 0,
            warning: false,
            warning_bright: true,
            slide_tick: crate::hud::BOSS_SLIDE_TICKS,
            emblem: Some(EmblemId("brute_boss_f01_emblem".to_owned())),
        });
        let mut out = Vec::new();
        build_draw_list(
            &build_snapshot(&real, None, 0),
            &HudLayout::new(360, 288),
            &mut out,
        );
        let frames = |f: FrameId| {
            out.iter()
                .any(|i| matches!(&i.kind, DrawKind::Frame { frame, .. } if *frame == f))
        };
        assert!(!frames(FrameId::IconBossEmblem), "no generic skull plate");
        assert!(!frames(FrameId::IconWarning), "no warning outside a tell");
        // The 20x20 well and the 18x18 emblem inside it, at the strip's +3, +3.
        let rest = HudLayout::new(360, 288).boss();
        let well = out
            .iter()
            .any(|i| i.x == rest.x + 3 && matches!(i.kind, DrawKind::Rect { w: 20, h: 20, .. }));
        let emblem = out
            .iter()
            .any(|i| i.x == rest.x + 4 && matches!(i.kind, DrawKind::Rect { w: 18, h: 18, .. }));
        assert!(well && emblem);
        // The 18x18 emblem item itself, over the magenta, at +4, +4.
        assert!(
            out.iter()
                .any(|i| i.x == rest.x + 4 && matches!(i.kind, DrawKind::Emblem { w: 18, h: 18 }))
        );
    }
}
