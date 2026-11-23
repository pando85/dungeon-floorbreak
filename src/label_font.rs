//! The 3 x 5 pixel label font shared by the code-drawn stand-in labels (the Boiler's props, K10 /
//! #254, and the Hoarder room's, FA-Q23): glyph bitmaps, one rasteriser with a 1 px shadow, and
//! the ink and shadow colours. **Presentation only**: a pure function of its text, nothing here
//! touches the simulation, a random number or a state hash. Glyphs are added as labels need them.

/// The 3 x 5 glyphs of the labels: five rows of three columns, `#` set.
const GLYPHS: &[(char, [&str; 5])] = &[
    ('A', [".#.", "#.#", "###", "#.#", "#.#"]),
    ('B', ["##.", "#.#", "##.", "#.#", "##."]),
    ('C', [".##", "#..", "#..", "#..", ".##"]),
    ('D', ["##.", "#.#", "#.#", "#.#", "##."]),
    ('E', ["###", "#..", "##.", "#..", "###"]),
    ('F', ["###", "#..", "##.", "#..", "#.."]),
    ('G', [".##", "#..", "#.#", "#.#", ".##"]),
    ('H', ["#.#", "#.#", "###", "#.#", "#.#"]),
    ('I', ["###", ".#.", ".#.", ".#.", "###"]),
    ('K', ["#.#", "#.#", "##.", "#.#", "#.#"]),
    ('L', ["#..", "#..", "#..", "#..", "###"]),
    ('M', ["#.#", "###", "###", "#.#", "#.#"]),
    ('N', ["##.", "#.#", "#.#", "#.#", "#.#"]),
    ('O', [".#.", "#.#", "#.#", "#.#", ".#."]),
    ('P', ["##.", "#.#", "##.", "#..", "#.."]),
    ('R', ["##.", "#.#", "##.", "#.#", "#.#"]),
    ('S', [".##", "#..", ".#.", "..#", "##."]),
    ('T', ["###", ".#.", ".#.", ".#.", ".#."]),
    ('U', ["#.#", "#.#", "#.#", "#.#", "###"]),
    ('V', ["#.#", "#.#", "#.#", "#.#", ".#."]),
    ('W', ["#.#", "#.#", "#.#", "###", "#.#"]),
    ('Y', ["#.#", "#.#", ".#.", ".#.", ".#."]),
    ('0', ["###", "#.#", "#.#", "#.#", "###"]),
    ('2', ["##.", "..#", ".#.", "#..", "###"]),
    ('-', ["...", "...", "###", "...", "..."]),
    (':', ["...", ".#.", "...", ".#.", "..."]),
    (' ', ["...", "...", "...", "...", "..."]),
];

/// Glyph advance (3 columns and 1 blank) and height of the label font, px.
pub const GLYPH_ADVANCE_PX: u32 = 4;
pub const GLYPH_HEIGHT_PX: u32 = 5;
/// The label ink (amber) and its 1 px drop shadow, so a label reads on any background.
pub const TAG_INK: [u8; 4] = [0xff, 0xd0, 0x70, 0xff];
pub const TAG_SHADOW: [u8; 4] = [0x10, 0x0c, 0x0a, 0xe0];

/// A rasterised label: `w x h` RGBA8, row-major, top row first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextImage {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// Rasterise `text` in the 3 x 5 label font with a 1 px shadow (one more column
/// and row than the ink). `None` when a character has no glyph.
pub fn text_pixels(text: &str) -> Option<TextImage> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return None;
    }
    let w = chars.len() as u32 * GLYPH_ADVANCE_PX; // the advance includes the shadow column
    let h = GLYPH_HEIGHT_PX + 1;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let mut put = |x: u32, y: u32, c: [u8; 4], over: bool| {
        let i = ((y * w + x) * 4) as usize;
        if over || rgba[i + 3] == 0 {
            rgba[i..i + 4].copy_from_slice(&c);
        }
    };
    let glyphs: Option<Vec<&[&str; 5]>> = chars
        .iter()
        .map(|ch| GLYPHS.iter().find(|(c, _)| c == ch).map(|(_, g)| g))
        .collect();
    let glyphs = glyphs?;
    // The shadow first (it never overwrites ink), then the ink on top.
    for (ink, colour) in [(false, TAG_SHADOW), (true, TAG_INK)] {
        for (n, glyph) in glyphs.iter().enumerate() {
            let ox = n as u32 * GLYPH_ADVANCE_PX;
            for (row, line) in glyph.iter().enumerate() {
                for (col, cell) in line.bytes().enumerate() {
                    if cell == b'#' {
                        let (dx, dy) = if ink { (0, 0) } else { (1, 1) };
                        put(ox + col as u32 + dx, row as u32 + dy, colour, ink);
                    }
                }
            }
        }
    }
    Some(TextImage { w, h, rgba })
}
