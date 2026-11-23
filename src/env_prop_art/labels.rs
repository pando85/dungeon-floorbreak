//! K10: the visible "STAND-IN: <name>" labels of the Boiler's code-drawn props.
//!
//! The pieces of [`super::pieces`] carry a label string for the capture and the
//! tests, but a flat rectangle on screen read as a broken texture in the first
//! human playtest. [`tags`] turns the stand-in pieces into small on-screen text
//! labels (layout px), and [`text_pixels`] rasterises one in a 3 x 5 pixel font
//! with a 1 px shadow so it reads on any background. **Presentation only**: both
//! are pure functions of the already drawn pieces, so nothing here touches the
//! simulation, a random number or a state hash.

use super::Piece;

/// The most labels on screen at once (the pool the presenter keeps).
pub const MAX_TAGS: usize = 8;

/// Piece label to on-screen text. Only the code-drawn stand-ins appear here:
/// the secondary pieces (bands, chains, plumes, tells) are parts of these.
pub const STAND_IN_TEXTS: [(&str, &str); 8] = [
    ("boiler landmark (stand-in)", "STAND-IN: BOILER"),
    ("valve body (stand-in)", "STAND-IN: VALVE"),
    ("brace (stand-in)", "STAND-IN: BRACE"),
    ("brace stump", "STAND-IN: BRACE"),
    ("hanging weight (stand-in)", "STAND-IN: WEIGHT"),
    ("landed remnant (stand-in)", "STAND-IN: RUBBLE"),
    ("fuel drum (stand-in)", "STAND-IN: DRUM"),
    ("drum scorch (stand-in)", "STAND-IN: SCORCH"),
];

/// How far inside the big landmark the label sits (its bottom edge, from the
/// landmark's top): below the dome, above the band.
pub const LANDMARK_TAG_DROP_PX: i32 = 14;

/// A label's ink box is `GLYPH_ADVANCE_PX` wide per character and
/// `GLYPH_HEIGHT_PX + 1` tall (ink and shadow row); a label that would overlap an
/// earlier one is lifted by this much (the box height and one blank line).
pub const LIFT_PX: i32 = GLYPH_HEIGHT_PX as i32 + 2;

/// One label to draw: its text, the centre column and the bottom edge of its
/// ink, in layout px (y down), like the pieces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag {
    pub text: &'static str,
    pub centre_x: i32,
    pub bottom_y: i32,
}

fn text_of(label: &str) -> Option<&'static str> {
    STAND_IN_TEXTS
        .iter()
        .find(|(l, _)| *l == label)
        .map(|(_, t)| *t)
}

/// The labels of the stand-ins among `pieces`: one per labelled piece, centred
/// on it and sitting just above its top edge (the big boiler landmark carries
/// its label inside, below the dome, so it stays on screen). Two pieces with the
/// same text (a brace stump and its twin) share one label. **No two labels
/// overlap** (K10 review of #254): a label whose ink box would cross an earlier
/// one (in piece order, which is stable) is lifted by [`LIFT_PX`] until it is
/// clear, so the text is readable and nothing depends on a draw order.
pub fn tags(pieces: &[Piece]) -> Vec<Tag> {
    let mut out: Vec<Tag> = Vec::with_capacity(MAX_TAGS);
    for p in pieces {
        let Some(text) = text_of(p.label) else {
            continue;
        };
        if out.len() == MAX_TAGS || out.iter().any(|t| t.text == text) {
            continue;
        }
        let bottom_y = if p.label == "boiler landmark (stand-in)" {
            p.rect.y + LANDMARK_TAG_DROP_PX
        } else {
            p.rect.y - 1
        };
        let mut tag = Tag {
            text,
            centre_x: p.rect.x + p.rect.w / 2,
            bottom_y,
        };
        // At most one lift per earlier label can be needed.
        for _ in 0..MAX_TAGS {
            if !out.iter().any(|other| tag.overlaps(other)) {
                break;
            }
            tag.bottom_y -= LIFT_PX;
        }
        out.push(tag);
    }
    out
}

impl Tag {
    /// The ink box `[x0, x1) x [y0, y1)` in layout px (y down).
    pub fn ink_box(&self) -> (i32, i32, i32, i32) {
        let w = self.text.chars().count() as i32 * GLYPH_ADVANCE_PX as i32;
        let x0 = self.centre_x - w / 2;
        (
            x0,
            self.bottom_y - GLYPH_HEIGHT_PX as i32 - 1,
            x0 + w,
            self.bottom_y,
        )
    }

    /// Whether the two ink boxes share a pixel.
    pub fn overlaps(&self, other: &Tag) -> bool {
        let (ax0, ay0, ax1, ay1) = self.ink_box();
        let (bx0, by0, bx1, by1) = other.ink_box();
        ax0 < bx1 && bx0 < ax1 && ay0 < by1 && by0 < ay1
    }
}

// The font is shared with the Hoarder room's labels (`crate::label_font`).
pub use crate::label_font::{
    GLYPH_ADVANCE_PX, GLYPH_HEIGHT_PX, TAG_INK, TAG_SHADOW, TextImage, text_pixels,
};
