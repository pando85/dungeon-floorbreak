# HUD-HYBRID-A-01: native 640x360 / 640x400 HUD design spec

**Status (2026-11-19):** design spec and keys for PR #103. Code side implemented in the same PR (HUD-HYBRID-A-01); the exported art atlas is integrated and drawn in the real captures. Not owner-accepted, game feel pending. It turns the contract in [hybrid-hud-a-design.md](hybrid-hud-a-design.md) into exact virtual-pixel rects, components, palette, type, timing, a state model and an asset list. Contract rules (control truth, resource policy, preview labelling) still apply. Where this spec and the contract disagree, the contract wins, except for the owner update below.

**Owner update (2026-11-19, replaces the "Donut plate: no HP bar" bullet in the contract):** Donut's plate **shows an HP bar in normal play**. The implementation adds a real, authoritative Donut HP value to the domain, with the same max as Carl (`HERO_HEALTH = 4`). It stays full until the Donut damage rules land (HUD-DEBT-01). This is real data, not a fixture. The contract's "Owner decisions" bullet was edited to match when this spec was committed.

Coordinates: virtual pixels, origin at the **top-left** of the virtual canvas, x to the right, y down. `rect(x, y, w, h)` covers x..x+w-1 and y..y+h-1. "360" means the 640x360 canvas and "400" means the 640x400 (`--deck`) canvas.

## 0. Rendering approach (decided)

- `Cargo.toml` enables bevy `=0.20.1` with `default-features = false` and `["2d", "bevy_audio", "vorbis"]`. In 0.20.1, `2d` = `default_app + default_platform + 2d_bevy_render + scene + picking`. That includes `bevy_sprite`, `bevy_sprite_render`, `bevy_text` (through `common_api`) and `default_font`. It does **not** include `bevy_ui` or `bevy_ui_render`, which come only with the `ui` collection. **UI `Node` is not available** unless a feature is added.
- **Use: a dedicated HUD `Camera2d` with sprites plus `Text2d`. Do not add `bevy_ui`.** Reasons: no new feature or dependency surface, the same integer `ScalingMode::Fixed` path the scene already uses (so it is integer-scaled by construction), and no UI-layout rounding. `Text2d` with `TextFont { font_smoothing: FontSmoothing::None, .. }` rasterises at the target scale factor (1.0 here, set by `scale_factor_override(1.0)`). The font atlas then switches to `ImageSampler::nearest()` (`bevy_text-0.20.1/src/font_atlas.rs:59`), and the fixed projection magnifies it with nearest filtering, so text stays crisp at 2x and 3x. `character_playground.rs:190` already uses `Text2d`.
- HUD camera: `Camera { order: <last scene camera order> + 1, clear_color: ClearColorConfig::None }`. When lit, that is `first_post + 5`, one after the debug overlay at `first_post + 4` (`src/light_pass.rs:523`). With `--no-lighting` it is the plain camera's order + 1. It also gets `Msaa::Off`, the same fixed projection 640 x `view_height(deck)`, and `RenderLayers::layer(LAYER_HUD = 15)`. Its `Transform` is constant at (320, view_h/2). It must **not** carry `SceneCamera`, so scroll, camera kick and shake never move it. Lighting slot cameras never see layer 15, so the HUD is unlit.
- Every HUD entity sits on whole-pixel positions. Use a helper `hud_to_world(x, y, w, h, view_h)` that returns the centre for sprites, and a top-left anchor for `Text2d`. Positions are computed from integers only. No `Transform` scale other than 1.
- Draw order inside the HUD: z bands. Frames 0, wells and bars 1, portraits and icons 2, text 3, flash overlays 4, PREVIEW tag 9.

## 1. Layout

Safe margin **8 px** on all sides. The **action lane** is the HUD-free band `y in [60, walk_y + 12]` across the full width. `walk_y` is the screen-space walk line. Derive it from layout data (`walk_line` layout y minus the view top of the layout `views` entry), never hard-code it. Measured from evidence: **288 at 360** (`walk_line_design_y = 288`), **about 306 to 308 at 400** (`docs/evidence/floor01-scene/floor01-scene-advanced-640x400.png`). The camera is horizontal-only (`src/scroll_camera.rs`: fixed `view_top`), so `walk_y` is constant per canvas.

Top-anchored elements have the same y at 360 and 400. Bottom-anchored elements are placed from the bottom edge, so they move down 40 px at 400.

| Element | 360 rect | 400 rect | Visible when |
| --- | --- | --- | --- |
| Carl plate (left slot) | (8, 8, 164, 48) | same | always in play |
| Donut plate (right slot) | (468, 8, 164, 48) | same | always in play |
| Location strip | centred: (320 - w/2, 8, w, 16), w = max(96, text + 28), w <= 280 | same | always, except while the boss strip shows |
| Boss strip | (180, 8, 280, 26) | same | boss encounter only (fixture today) |
| System idle face | (612, 334, 20, 18) | (612, 374, 20, 18) | always in play |
| System notice panel | (404, 320, 204, 32) | (404, 360, 204, 32) | finite: section 5 |
| PREVIEW tag | (8, 341, w, 11), w = text + 10 | (8, 381, w, 11) | only with `--hud-preview` |

Clearances (all must hold, and are tested in section 8):
- Plates end at x=171 and start at x=468. The centre gap is 172..467. The boss strip (180..459) and the widest location strip (180..459) keep **at least 8 px** from both plates.
- All top elements end by y=55 (plates: 8+48-1). The action lane starts at 60.
- The notice panel top (320 / 360) is **32 px / about 52 px** below `walk_y`. Rule: `notice_top >= walk_y + 24`. It sits over the foundation band below the floor and never over the action lane.
- The notice panel (x 404..607) and the face (612..631) keep a 4 px gap. The PREVIEW tag (x <= 200) never reaches the notice (x >= 404).

Diagrams: [zones_640x360_2x.png](../evidence/hud-hybrid-a/zones_640x360_2x.png) and [zones_640x400_2x.png](../evidence/hud-hybrid-a/zones_640x400_2x.png). Green is the action lane and walk line, yellow the top HUD rects, cyan the bottom rects, magenta the safe margin. Composited mocks (design scratch, not committed): `mock_normal_640x360.png`, `mock_combat_depleted_640x360.png` and `mock_boss_coop_640x400.png` (each with `_3x`), plus `mock_details_zoom.png`. The mocks use the real candidate fonts and the keyed Carl and CRT art. Frames, chips and icons in them are spec-level placeholders the executor redraws. The Donut portrait in the mocks is a **raw 1x crop stub**, not a key.

## 2. Components

### 2.1 Hero plate (164 x 48, identical for both slots; content is left to right in both)

Offsets are relative to the plate's top-left corner.

- **Frame (bitmap):** 3 px ring. Row and column 0 are `hud_ink`. Ring 1 is `metal_hi` on the top and left and `metal_mid` on the bottom and right (a worn bevel). Ring 2 is `hud_ink`. The fill is `plate_fill`. Wear: at most 6 rust flecks (`rust`, 1 to 3 px clusters) on ring 1, only on the bottom and right edges, never inside the text column. No gradients and no translucency.
- **Portrait well:** rect (+4, +4, 32, 32). 1 px `hud_ink`, 1 px `brass`, then the 28x28 interior at (+6, +6) filled with the per-character well colour (`well_carl`, `well_donut`). **Rivets:** 2x2 at the well's four corners (+4,+4), (+34,+4), (+4,+34), (+34,+34): top-left pixel `brass_hi`, top-right and bottom-left `brass`, bottom-right `brass_dark`.
- **Portrait:** 28x28 at (+6, +6), transparent background, composited over the well. Carl faces right, toward the playfield. Donut uses her frontal sprite read. See section 7 for sources.
- **Ownership tag:** (+4, +36, 32, 9), directly under the well. Text in Silkscreen Bold, centred, caps top at +38. The three tags differ in **shape and text**, not only colour:
  - `P1`: solid `amber` with `hud_ink` text.
  - `P2`: solid `magenta_hi` with `hud_ink` text and notched corners (the 4 corner pixels are `plate_fill`).
  - `AI`: hollow, a 1 px `cyan` outline on `hud_ink` with `cyan` text.
- **Name:** Jersey 10, `amber`, caps top at (+40, +4), 10 px caps. The name comes from character data (fan-concept layer), not from code.
- **HP row** (+16..+23):
  - Icon (+40, +16), a 7x7 heart: `hp_red` with two `hp_red_hi` pixels. At `hp == 1` it swaps to the **cracked heart**, an ink crack of 4 px. This is a non-hue low-HP cue.
  - Bar outer (+50, +16, 96, 8): 1 px `hud_ink` border, inner 94x6.
  - **Segment rule:** one segment per HP point while `hp_max <= 8`. Segment width `sw = (94 - (n-1)*2) / n` with 2 px `hud_ink` gaps. If the division leaves a remainder, add it to the last segment. For n=4: sw=22, gaps 2, which is exact.
  - Full segment: `hp_red` with a top row of `hp_red_hi` and a bottom row of `hp_red_lo`. Empty segment: `hp_empty` fill with a 1 px `metal_light` outline. The outline keeps max HP readable by **shape** (contrast `hp_empty` vs plate is only 1.1:1, so the outline is mandatory).
  - If a future hero has `hp_max > 8`: 8 segments, with fill proportional to the pixel. Not needed now.
  - **Why one per HP:** Carl has exactly 4 integer HP and takes 1 per hit (`HeroHurt{health_left}`). A segment per point makes "how many hits left" readable at a glance and by counting, with no colour needed. It invents no fake granularity (the concept's "88/100" is rejected) and needs no number. No numeric readout.
- **Mana row** (+26..+30), Donut only, and only when `mana` is `Some`. Hidden entirely pre-unlock: the space stays empty, so the layout never shifts at unlock.
  - Icon (+41, +26): a 5x5 diamond spark, `mana_v0` with a `cyan` centre pixel.
  - Bar outer (+50, +26, 96, 5): 1 px `hud_ink`, inner 94x3. That is thinner than HP (3 vs 6 fill rows).
  - 8 segments of 10 px with 2 px gaps (8x10 + 7x2 = 94). Each segment is one flat colour from `mana_v0` (violet) to `mana_v7` (cyan). No per-pixel gradient.
  - Fill is pixel-proportional (`round(frac * 94)` px, left to right, gap pixels stay ink). This is a presentation rule only. The resource and its granularity are HUD-DEBT-02.
- **Mana depleted** (fill == 0). Three non-hue cues, all required:
  1. Empty track: all segments `mana_empty` with a 1 px `metal_light` outline around the whole 94x3 inner track.
  2. The spark icon becomes a hollow `metal_hi` outline with an `amber_dim` diagonal slash, bottom-left to top-right.
  3. The primary chip that consumes mana is muted (2.4).
  A one-time flash on reaching 0 is in section 5.
- **Ability row** (+34..+44): chip slot A at (+40, +34), chip slot B at (+93, +34), special socket at (+147, +34, 11, 11). The row ends at +157 and the frame starts at +161. Each chip slot is at most 50 px wide: an 11x11 glyph, a 3 px gap, then a label of at most 36 px in Silkscreen `amber`, caps top at +37. AI-controlled plates show no chips (2.4).

### 2.2 Location strip (16 px tall, variable width)

- A 3-slice bitmap: left cap 8 px, a 1 px repeatable middle column, right cap 8 px. The same 3 px ring as the plate.
- Each cap holds a magenta "lamp": a 4x6 `magenta_dk` rect with a 2x4 `magenta_hi` core, at cap x+4, y+5.
- Text: Jersey 10, `amber`, centred, caps top at y+3.
- Content is real data only: `FLOOR 01` (the floor number is real). An area name is appended as `FLOOR 01 · <AREA>` only if the layout data carries one. The concept's "INSPECTION CORRIDOR" is **not** used. Maximum width 280.

### 2.3 Boss strip (280 x 26)

- 3-slice frame, same ring.
- Emblem well (+3, +3, 20, 20): `hud_ink` border with `magenta_dk` fill. It holds a **generic 10x8 skull-plate emblem** in `text_white`, not a boss portrait. A real boss gets a portrait in HUD-DEBT-03.
- Name: Jersey 10 `text_white`, caps top at (+27, +4). Fixture name **`TEST BRUTE`**. Never "Warlord Grizzog".
- Bar outer (+27, +16, 236, 7): 1 px ink, inner 234x5.
  - Continuous fill `hp_red`, with a top row of `hp_red_hi`.
  - Ink ticks at 25%, 50% and 75%. These are reading aids, **not** phases.
  - Damage "chip": the lost span shows `hp_flash` and drains toward the fill (section 5).
- Warning icon (+266, +5, 11, 10): `amber` triangle with an ink "!" in it. It shows only while the boss telegraph flag is set and is otherwise hidden. The shape is the cue, the blink is extra.
- No numbers. "2,400/2,400" is rejected.

### 2.4 Ability chips: what shows, per state

Glyphs are 11x11 monochrome disks: a `hud_ink` rim, the top half `metal_light`, the bottom half `metal_mid`, and a 5x5 symbol in `text_white`. Xbox set: letters A, B, X, Y. DualSense set (`--glyphs dualsense`, and also accept `--glyphs=dualsense`): cross, circle, square, triangle. Glyphs are **original pixel drawings**. Do not copy vendor glyph art or use vendor button colours (no green A or red B). Meaning comes from the letter or shape.

| Plate state | Slot A | Slot B | Special socket |
| --- | --- | --- | --- |
| Carl, player-controlled, Primary/Physical live (a combat section) | X/Square `PUNCH` | Y/Triangle `KICK` | locked |
| Carl, player-controlled, no live Primary/Physical (the story corridor ignores melee) | no chips (layout kept) | no chips | locked |
| Donut, AI-controlled (normal play now) | no chips. Instead the label `COMPANION` in `amber_dim` at (+40, caps +37) | (none) | (none) |
| Donut, player-controlled, pre-biscuit (fixture) | X/Square `SWAT` | Y/Triangle `POUNCE` | locked |
| Donut, player-controlled, post-biscuit (fixture) | X/Square `MISSILE` | Y/Triangle `POUNCE` | locked |
| Donut, post-biscuit, mana 0 (fixture) | `MISSILE` **muted** | `POUNCE` | locked |

- **Chips follow live actions (orchestrator ruling 2026-11-19, review item R1 of PR #103; the controller contract decides it):** a chip shows only for an action the scene acts on right now. The HUD reads this from `RealHudInputs::melee_live`, which the scene sets from the real melee gate (`combat.is_some()`, the same condition that gates melee input), never per mode in the HUD. Fixtures keep their table values.
- **Locked special socket:** an 11x11 `hud_ink` / `metal_dark` socket holding a 7x7 `metal_light` padlock. **No glyph and no label.** The empty socket tells the player a slot exists without showing B/Circle as a usable button (control contract: "disabled/grayed out with a clear cue"; HUD contract: "never show a locked capability as usable").
- **Equipped special socket (CARL-PROG-01 CP-Q32, CP-B2; adopted default CP-B2-AD1, owner art review pending):** once a special is held the socket shows the Circle (DualSense) or B (Xbox) glyph in its 11x11 cell, in place of the padlock, and the data label (`cpq32_socket_label`, default `BOOM`) in Silkscreen `amber`, right-aligned to the row end (+157) with its caps top at +28 on the free row above the chips (no label fits beside an 11 px socket; only Donut's plate has a mana bar on that row, and it ends at +146). Muted while a throw or stick is live and while it recharges: the glyph and label turn `amber_dim` (high contrast keeps the label `amber`) and a 1 px zig-zag runs under the glyph, so the cue is not hue alone. The padlock stays for every plate whose special is not held. **Superseded in part by [HUD-ART-02](hud-art-02-spec.md) (HA2-AD1):** Donut's mana track is now 72 px outer (ending at +121), and `BLINK` is drawn on her plate right-aligned to +157 like `BOOM`; the "label not drawn beside the track" reading above no longer applies.
- **Muted chip:** the glyph symbol and label change to `amber_dim`, plus a 1 px `amber_dim` zig-zag hatch along the chip's bottom row. Whether Magic Missile is actually blocked at 0 mana is HUD-DEBT-02. The muted chip appears only in the fixture.
- **Never shown:** A/Cross Jump (universal, taught by prompts). RB/R1 Interact and LB/L1 Swap until those actions exist. Once they exist, Interact is a contextual prompt and `LB SWAP` replaces `COMPANION` on the AI plate in solo. Pause, LT/RT, any dodge, Whisker Lash, Cat Step or Dash.
- AI-controlled plates never show button glyphs, because pressing them would act on the other hero.

### 2.5 System (original CRT motif; **keyed**, `keys/system_crt_face.grid.json`)

- A beat-up brass portable TV, 20x18: two bent metal antennas (the right one kinked), a dented top bevel, a 9x7 `screen_bg` screen, a side column with two knobs and a speaker grille, and stub feet.
- Frames: `idle` (happy ^ ^ eyes, small smile), `blink`, `notice` (V brows, wide toothy grin: theatrical and slightly sinister). No crown: the crown is Donut's motif and the concept's crowned box is not copied.
- **Idle state:** the face alone at the idle rect. No label, no panel.
- **Notice panel** (204x32):
  - Frame: a 1 px `hud_ink` ring, a 1 px `magenta` ring, a 1 px `hud_ink` ring, then fill `notice_bg`. A dotted `magenta_dk` rule 3 px above the bottom.
  - Line 1: Silkscreen Bold `magenta_hi`, caps top at (+7, +6). Fixed text `SYSTEM NOTICE`.
  - Line 2: Jersey 10 `amber`, caps top at (+7, +15). The headline, at most 180 px, ideally 16 characters or fewer.
  - While the panel is up, the face shows `notice`.
- **Real triggers** (from `CombatEvent`). Copy is placeholder, kept in a data table, and needs an owner tone check:

| Event | Notice | Headline |
| --- | --- | --- |
| `EnemyDefeated{force: Heavy}` (kick-launch kill) | yes | `LAUNCHED!` |
| `EnemyDefeated{force: Catastrophic}` (e.g. the shore block) | yes | `FLATTENED!` |
| `EnemyDefeated{force: Routine}` | **no** (too frequent; would become a permanent overlay) | |
| `HeroHurt` | no. The HP flash is enough; never compete with a threat. | |
| `GoblinTell` | **never**. It is a threat telegraph. The HUD stays out of its way. | |
| `HeroDefeated` | no | |
| `Restart` | clears the notice and the queue, face to `idle` | |

  Never use scores, multipliers, buff text ("+25%"), or claims of rewards.

### 2.6 PREVIEW tag

`hud_ink` box, 11 px tall, with a 2-on/2-off `amber` dash stripe on the top and bottom rows. Text in Silkscreen Bold `amber`: `PREVIEW <FIXTURE>`, for example `PREVIEW BOSS`. It is drawn above everything, cannot be hidden, and exists only when `--hud-preview` is set.

## 3. Palette (HUD swatch budget: 34 colours)

The HUD follows the art direction: cold blue-violet metal, warm amber/brass, magenta for "the System" and magic, red **only** for HP and danger, and a violet-to-cyan accent for Donut's magic. The darks are plum-shifted to sit with the character outlines (`#1b1124` and `#1e1229`).

| Token | Hex | Use |
| --- | --- | --- |
| hud_ink | `#120c18` | outlines, gaps, text on light chips |
| plate_fill | `#211b27` | plate interior |
| metal_dark | `#2a2430` | sockets |
| metal_mid | `#3d3542` | lower bevel, glyph lower half |
| metal_light | `#5c5262` | empty-segment outline, glyph upper half, padlock |
| metal_hi | `#8a7e8c` | upper bevel, hollow icons, knobs |
| rust | `#7a3f2a` | wear flecks only |
| brass_dark / brass / brass_hi | `#6a4a1c` / `#b58a3a` / `#e8c063` | well rim, rivets, CRT casing |
| amber | `#ffc24a` | names, labels, P1 tag, headlines |
| amber_dim | `#a8742a` | muted labels, `COMPANION`, slash |
| text_white | `#f4e6d2` | glyph symbols, boss name |
| magenta_dk / magenta / magenta_hi | `#6e1a4c` / `#e0338f` / `#ff6fc0` | System frame, lamps, P2 tag, notice label |
| screen_bg | `#2a0d24` | CRT screen |
| crt_face_hi | `#ffb3dc` | CRT eyes and teeth (notice) |
| notice_bg | `#1e0f22` | notice fill |
| hp_red_lo / hp_red / hp_red_hi | `#9a1a2c` / `#d8283c` / `#ff5a5f` | HP fill |
| hp_empty | `#3a1620` | empty HP segment |
| hp_flash | `#fff1e0` | hit flash, damage chip |
| mana_v0..v7 | `#7a3cf0 #6a4ff0 #5a63f2 #4a7af4 #3d92f4 #33aaf0 #33c2ec #45d8ea` | Donut mana segments |
| mana_empty | `#1c1a33` | empty mana track |
| cyan | `#5fe0e8` | AI tag, spark centre |
| well_carl / well_donut | `#2b2038` / `#301a2e` | portrait backgrounds |

Portraits use **their own sprite-master palettes** (Carl: the 23 keys of `carl_h52.grid.json`; Donut: `donut_h40.grid.json`). They are not counted in the 34.

**Contrast** (WCAG ratio, computed):

| Foreground | on plate_fill | on metal_dark | on notice_bg |
| --- | --- | --- | --- |
| amber | 10.4 | 9.4 | 11.4 |
| text_white | 13.7 | 12.3 | 14.9 |
| cyan | 10.6 | | |
| magenta_hi | 6.6 | | 7.2 |
| amber_dim | 4.2 | | |
| hp_red | 3.4 | | |
| hp_empty | 1.1 | | |
| mana_empty | 1.0 | | |

- All text is at least 4.2:1. Primary text (amber, white) is at least 9:1.
- `amber_dim` (4.2) is used only for de-emphasised or muted text, which is intended.
- HP red is a fill, not text. It is always bordered by ink and segmented.
- `hp_empty` and `mana_empty` are near-invisible against the plate, which is why empty state is carried by the outline shapes and not by colour.

## 4. Typography

The owner delegated the choice. Both fonts are OFL 1.1 with **no Reserved Font Name** (checked in each `OFL.txt` on 2026-11-19), so a bitmap atlas built from them raises no RFN issue. Commit each TTF with its `OFL.txt`, a provenance README and SHA-256 under `assets/fonts/`. Re-download the files from the primary source in the executor step. The hashes below are from the design probe.

| Role | Font | Size (exact pixel grid) | Measured | Licence / source |
| --- | --- | --- | --- | --- |
| Display: names, location, notice headline, boss name | **Jersey 10** (Sarah Cadigan-Fried, "Soft Type") | `FontSize::Px(1400/75 = 18.6667)` gives 1 font pixel = 75 units, so **10 px caps** with integer advances | `CARL` 30 px, `DONUT` 40, `FLOOR 01` 52, `TEST BRUTE` 71, `LAUNCHED!` and `FLATTENED!` within 70 | https://github.com/google/fonts/blob/main/ofl/jersey10/OFL.txt, upstream https://github.com/scfried/soft-type-jersey; sha256 `db9cbd091617048a145d249daa2b815fe7083be6ab66ac26626e21a4e01c3e82` |
| Labels: chips, tags, `SYSTEM NOTICE`, PREVIEW, `COMPANION` | **Silkscreen** Regular and Bold (Jason Kottke) | `FontSize::Px(8.0)`, **5 px caps** | `PUNCH` 31, `KICK` 21, `SWAT` 24, `POUNCE` 36, `MISSILE` 35, `COMPANION` 54, `SYSTEM NOTICE` (Bold) about 72 | https://github.com/google/fonts/blob/main/ofl/silkscreen/OFL.txt, upstream https://github.com/googlefonts/silkscreen; Regular sha256 `c845473330b94c2079ce9af01c51ac8ba2d99c24f4d14c039843bbb8e642ebd8`, Bold `768476aa712d4f5c3e18d3bce80f980a8bd3f72b7094d22ec5e768df3acfed61` |

- **Why Jersey 10:** condensed, heavy athletic caps. It matches the concept's chunky amber headers and the trashy sports-broadcast and game-show framing. The condensed width fits a 640 px canvas. It renders with no grey pixels at the grid size (probe: `font_probe_4x.png`).
- Silkscreen is the legible 5 px workhorse. Text is all caps throughout.
- The implementer must verify in Bevy: render `HAMBURGLE 0123` at both sizes at 1x and assert every pixel is fully on or off (no partial alpha) and the cap height is exactly 10 / 5. If the Jersey grid size gives partial pixels in Bevy's swash rasteriser, try the nearest integer size (18 or 19) and re-measure. Record the result.
- **Fallback if a licence check or the grid check fails:** an original in-repo bitmap font. 5x7 caps plus digits and `·!:+-/`, authored as a `.grid.json` glyph sheet through the existing pipeline, and drawn as atlas sprites by a tiny HUD text helper. This has no licence dependency. Display text then uses the same 5x7 at 1x (the plate needs no re-layout: the name row has room for 7 px caps).

## 5. Animation and timing (fixed 60 Hz tick; HUD timers are driven by the scene tick, so captures are deterministic)

| Cue | Normal | Reduced motion (`--hud-reduced-motion`) |
| --- | --- | --- |
| **Notice in** | ticks 0..7: the panel slides from x+24 to x+0, 3 px per tick. The face switches to `notice` at tick 0. Optional screen flash: CRT screen fill `crt_face_hi` on ticks 0 and 1. | appears at tick 0, no slide, no screen flash |
| **Notice hold** | 120 ticks | 120 ticks |
| **Notice out** | 8 ticks, sliding right 3 px per tick. Face back to `idle` at the end. Total 136 ticks (about 2.3 s). | disappears at once |
| **Notice queue** | at most 1 pending. A newer event replaces the pending one. At least 30 ticks between notices. The same headline cannot repeat within 300 ticks. `Restart` clears everything. | same |
| **Idle blink** | `blink` for 6 ticks every 240 ticks | no blink |
| **HP loss** (`HeroHurt`) | lost segment(s) `[health_left, previous)`: ticks 0..23 alternate `hp_flash` and `hp_empty` every 4 ticks (3 flashes), then the empty state. This sits inside the 55-tick invulnerability window. | lost segment shows a static diagonal hatch (`hp_flash` 1-in-3 pixels on `hp_empty`) for 24 ticks, then empty |
| **Low HP** | at `hp == 1`, cracked-heart icon (static) | same |
| **Mana reaches 0** | the empty-track outline is `magenta_hi` for 6 ticks, then `metal_light`. Icon and chip switch at once. | no flash |
| **Mana reveal** (future real unlock) | segments appear left to right, one every 3 ticks (24 ticks) | at once |
| **Boss strip in / out** | slides down from y = -26 to 8 at 3 px per tick (12 ticks, last step clamped). Reverse on end. The location strip hides on in-tick 0 and returns after out. | at once |
| **Boss damage chip** | the lost span is `hp_flash` for 30 ticks, then drains 2 px per tick | the chip is removed at once |
| **Boss warning icon** | while the telegraph flag is on, bright (`amber`) and dim (`amber_dim`) toggle every 8 ticks | steady `amber` |

Positions are integers on every tick. No easing curves, no sub-pixel motion, no alpha fades. No HUD element shakes. Camera shake and kick never touch the HUD camera.

## 6. State model (Bevy-free, `src/hud.rs` or similar; unit-testable headlessly)

```rust
pub struct HudSnapshot {
    pub plates: [PlateView; 2],          // [0] = left slot, [1] = right slot
    pub location: Option<String>,         // real: "FLOOR 01" (+ area name only if layout data has one)
    pub boss: Option<BossView>,           // FIXTURE ONLY today (HUD-DEBT-03)
    pub notice: Option<NoticeView>,       // real: from NoticeQueue fed by CombatEvent; or fixture
    pub glyphs: GlyphSet,                 // Xbox (default) | DualSense (--glyphs), HUD-DEBT-05
    pub preview: Option<HudPreview>,      // Some => PREVIEW tag drawn
    pub reduced_motion: bool,             // --hud-reduced-motion, HUD-DEBT-06
}
pub struct PlateView {
    pub portrait: PortraitId,             // data id, IP-neutral; fan layer maps it to art
    pub name: String,                     // from character data
    pub control: Control,                 // Player(1) | Player(2) | Ai
    pub hp: Meter,                        // REAL for both: Carl Hero.health / HERO_HEALTH;
                                          // Donut new authoritative value, max = HERO_HEALTH (HUD-DEBT-01 adds damage)
    pub hp_flash: Option<HpFlash>,        // { lost_from: u8, lost_to: u8, age_ticks: u8 }, from HeroHurt
    pub mana: Option<ManaView>,           // None = pre-unlock (REAL today); Some only from fixture (HUD-DEBT-02)
    pub abilities: AbilityRow,            // derived: control + capability/unlock state (2.4 table)
}
pub struct Meter { pub cur: u8, pub max: u8 }
pub struct ManaView { pub fill_px: u8 /* 0..=94 */, pub depleted_age: Option<u8> }
pub enum AbilityRow { Chips { a: Chip, b: Chip, special: SpecialSlot }, AiCompanion }
pub struct Chip { pub action: LogicalAction /* Primary | Physical */, pub label: LabelId, pub muted: bool }
pub enum SpecialSlot { Locked /* future: Equipped(Chip) */ }
pub struct BossView { pub name: String, pub fill_px: u8 /* 0..=234 */, pub chip_px: u8, pub warning: bool, pub slide_tick: u8 }
pub struct NoticeView { pub headline: LabelId, pub age_ticks: u16 }
pub enum HudPreview { PreUnlock, PostUnlock, Depleted, Boss, Coop }
```

**Amendment (Floor 01 boss, [floor-01-boss-design](floor-01-boss-design.md) section 4):** `BossView` gains `pub emblem: Option<EmblemId>`. `None` draws the generic skull plate, and `Some(id)` draws that boss's 18x18 emblem centred in the 20x20 well (see asset row 8).

- `build_snapshot(real: &RealHudInputs, preview: Option<HudPreview>, tick)` is a pure function. `RealHudInputs` reads `Hero.health`, the new Donut HP, control assignment (Carl P1, Donut AI today), unlock caps (none today) and the notice queue.
- **The fixture only overrides HUD fields.** It never writes to simulation, combat, input, RNG or replay. Headless replays and snapshots are byte-identical with and without `--hud-preview`.
- Rendering reads only `HudSnapshot`. Gameplay never reads HUD state.
- **Preview fixtures**, reachable only through `--hud-preview <name>` (also accept `=`; parse it like `--fixture`, error on unknown names). They combine with `--deck`, `--glyphs`, `--scale`, `--capture-tick`, `--no-lighting` and `--hud-reduced-motion`. The PREVIEW tag always shows.

| Name | Carl | Donut | Other | Retired by |
| --- | --- | --- | --- | --- |
| `pre-unlock` | P1, 4/4, PUNCH/KICK | **P2**, 4/4, no mana, SWAT/POUNCE | location strip | HUD-DEBT-04 (P2) |
| `post-unlock` | P1, 4/4 | P2, 4/4, mana 94/94, MISSILE/POUNCE | location strip | HUD-DEBT-02, -04 |
| `depleted` | P1, 2/4, HP flash active on segment 2 | P2, 4/4, mana 0, MISSILE muted, depleted flash at age 0 | notice `LAUNCHED!` at age 20 | HUD-DEBT-02, -04 |
| `boss` | P1, 3/4 | AI, 4/4, mana 60/94 | boss `TEST BRUTE` at 164/234 with a 14 px chip, warning on; notice `BOSS INCOMING` (fixture-only copy); location hidden | HUD-DEBT-03 |
| `coop` | P1, 3/4 | P2, 2/4, mana 31/94 (about 1/3), MISSILE/POUNCE | location strip | HUD-DEBT-04, -02 |

- Fixture Donut HP below max (`depleted`, `coop`) is fixture data, not balance.
- Normal play (no flag) = Carl P1 4/4 live from `Hero.health`, Donut AI 4/4 (real, full), `COMPANION`, no mana, location `FLOOR 01`, real notices only.

## 7. Asset list (for the executor)

Difficulty routing (pixel-art-pipeline "routing by difficulty"):
- Each portrait: new design 1, height 0, construction 0, VFX 0, score 1, **Easy, route C**.
- CRT face: score 1, **Easy, route C**.
- Frames, chips and icons: score 0 or 1, prop-like, **Easy, spec-only**.
- **The silhouette gate applies to both portraits and the CRT face** (new designs).

| # | Asset | Size | Source | Who |
| --- | --- | --- | --- | --- |
| 1 | `hud_portrait_carl` | 28x28, 1 frame | `assets/source/hud/hud_portrait_carl.grid.json` from the **key** `keys/carl_portrait.grid.json` (palette = `carl_h52` keys). The executor may split it into layers (`hair`, `head`, `jacket`, `pin`) without changing pixels. | **Keyed (design).** The executor only fixes lint-level issues. Any pixel change goes back to design or the reviewer. |
| 2 | `hud_portrait_donut` | 28x28, 1 frame | Derive from `donut_h40.grid.json` frame 0. Take the head region (crown to upper chest), keep **1:1 pixel density** (no scaling), and reframe so the crown is fully inside row 0 or below and the eyes sit on rows 11 to 15. Cut the cheek ruff at the well edges, repaint edge pixels where the crop leaves orphans, and keep her neutral cute expression and golden eyes. Palette = `donut_h40` keys only. | Spec-only; the executor builds it; silhouette gate |
| 3 | `hud_system_crt` | 20x18, 3 frames (`idle`, `blink`, `notice`) | the **key** `keys/system_crt_face.grid.json` (HUD palette subset) | **Keyed (design).** Same rule as #1. |
| 4 | `hud_plate_frame` | 164x48 (frame, well, rivets; tag areas transparent) | `.grid.json`, built from 2.1 | Spec-only |
| 5 | `hud_strip_3slice` | caps 8x16 plus a 1x16 middle; also a 26 px-tall variant for the boss strip | `.grid.json` | Spec-only |
| 6 | `hud_notice_frame` | 204x32 | `.grid.json` (or 3-slice) | Spec-only |
| 7 | `hud_glyphs` | 11x11 x 8 (A B X Y, cross circle square triangle) + `socket_locked` 11x11 | `.grid.json`, 9 frames | Spec-only |
| 8 | `hud_icons` | heart 7x7 (normal, cracked), spark 5x5 (normal, depleted), warning 11x10, boss emblem 20x20 (generic plate; per-boss emblems are 18x18 inside the well and are standalone atlases with the boss palette, like the portraits, e.g. `brute_boss_f01_emblem`, not merged into `hud_icons`) | `.grid.json` | Spec-only |
| 9 | `hud_tags` | 32x9 x 3 backgrounds (P1 solid, P2 notched, AI hollow); text from font | `.grid.json` | Spec-only |
| 10 | Bars, PREVIEW tag, hatches | none (native coloured sprites at integer rects) | code, palette constants | Spec-only (code) |
| 11 | Fonts | Jersey 10, Silkscreen Regular and Bold + `OFL.txt` | `assets/fonts/` + README + SHA-256 | Executor |

- Pipeline for #1 to #9: `.grid.json`, then `scripts/pixel_grid_to_ase.py lint --strict` and `build`, then `.ase`, then export through `scripts/export_assets.sh` into one `hud_hybrid_a` atlas PNG plus metadata.
- Add a manifest `assets/manifests/hud_hybrid_a_v1.json` with rects, SHA-256 and provenance, recording "derived from approved sprite masters; no AI concept pixels copied".
- Concept PNGs are **never** sampled, traced or cropped.
- Keys and mocks are in this design's scratch folder. Copy the two key grids into `assets/source/hud/`. The mocks are evidence of layout intent only and are not committed as assets.

## 8. Acceptance targets for the independent art review

1. **Rects:** an automated test asserts that every element rect in section 1 at 360 and 400 lies inside the 8 px safe margin and that no two top rects or bottom rects intersect. It also asserts that no HUD rect intersects the action lane `[60, walk_y + 12]`, using `walk_y` from layout data, and that `notice_top >= walk_y + 24`.
2. **Gameplay clearance:** over every existing CP or combat fixture's capture run, the highest screen-space top of any hero or enemy hitbox, launch arc or telegraph box is at y >= 60. It must never intersect a HUD rect. Report the measured minimum. (Measured 2026-11-19 by `combat_scene::tests::gameplay_stays_below_the_hud_plates`: minimum 147.5 at 360 and 167.5 at 400, see the [evidence README](../evidence/hud-hybrid-a/README.md).)
3. **Pixel integrity:** 1x captures at 640x360 and 640x400 contain no HUD pixel with partial alpha against the HUD layer, and text cap heights are exactly 10 and 5 px. A 3x capture equals the nearest-neighbour 3x upscale of the 1x capture for the HUD region (no shimmer, no fractional offsets). Check this across a camera scroll.
4. **Legibility at 1x:**
   - Every label is readable in the 1x capture viewed 1:1, and in the 2x (1280x800) capture at Steam Deck viewing distance.
   - No text under 5 px caps.
   - Contrast of every text colour on its fill is at least 4.2:1 (table in 3).
5. **Non-hue cues** (grayscale capture test): in a desaturated 1x capture, the reviewer can tell all of the following without colour:
   - HP count and empty segments (outline)
   - low HP (cracked heart)
   - mana present vs depleted (fill, outline, slashed icon, hatch)
   - P1 vs P2 vs AI (text and tag shape)
   - a locked socket (padlock, no glyph)
   - the boss warning (triangle shape)
6. **Control truth:**
   - The chips match table 2.4 exactly, with Xbox letters by default and DualSense shapes under `--glyphs dualsense`.
   - No LT/RT, no Dodge, Whisker Lash, Cat Step or Dash, and no B/Circle glyph anywhere.
   - AI plates show no glyphs.
7. **Truthful data:**
   - Normal play renders live `Hero.health` (check one `HeroHurt`) and Donut's real 4/4.
   - There is no mana bar without `--hud-preview`, and no boss strip or P2 tag without it.
   - Every preview capture shows the PREVIEW tag.
   - Replays and snapshots are unchanged with and without the flag.
8. **Notices:** fire only on the trigger table events. They are never on screen for more than 136 ticks, never over the action lane, and never triggered by `GoblinTell`. Reduced motion removes slides, blinks and flashes.
9. **Tone** (antihero-tone review gate):
   - Carl's portrait reads as a plain, irritated everyman: soft jaw, messy mop, open "huh" mouth, not a heroic face.
   - Donut's portrait keeps her cute neutral face and golden eyes.
   - The System face reads as a cheerful, sinister, cheap game-show box.
   - The frames read as worn industrial hardware, not slick sci-fi.
   - Nothing copied from the concept PNGs, vendor glyph art or third-party UI.
10. **Silhouette gate:** each portrait and the CRT face at 1x in the lit scene compared with its source (sprite master or this spec). Answer: "reads as the approved character / as the System motif?"
11. **Performance:** report the measured frame time with and without the HUD (`--perf-frames`). No claim of 60 fps without numbers.

## 9. Open questions (short)

1. **Notice copy:** `LAUNCHED!`, `FLATTENED!` and `BOSS INCOMING` are placeholder tone. The owner may replace them; they live in a data table.
2. **Area name:** the layout has no area-name field. Should Floor 01 areas get names (fan-layer data), or stay at `FLOOR 01` only?
3. **Keyboard players** see pad glyphs (no keyboard glyph set and no device detection; HUD-DEBT-05). Is a keyboard glyph set wanted in P1, or later? **Answered by [HUD-ART-02](hud-art-02-spec.md) (HA2-AD4, HA2-AD4b):** a keyboard key-cap glyph set and last-used-device detection for P1 exist.
4. **Plate occlusion:** if a future encounter puts actors in the top 56 rows, should plates dim to a 50% dither? Not designed now; test 8.2 guards it.
5. **Portrait reactions** (hurt or low-HP portrait frames) are out of scope here. Candidate follow-up.
6. **Keyboard glyphs (HUD-DEBT-05):** keyboard players see gamepad glyphs today. Recorded as a TODO in the backlog. **Done by [HUD-ART-02](hud-art-02-spec.md):** `GlyphSet::Keyboard` and the P1 last-used-device switch.
7. **Area names (implementation note):** the HUD shows `FLOOR 01` only until layout data carries an area name.
8. **Notice copy (implementation note):** the headlines live in a data table in `src/hud.rs` and are placeholders pending an owner tone check.
9. **Implementation deviations:** see the HUD section of [floor01-scene.md](../03-tech/floor01-scene.md#hud-hud-hybrid-a-01) (muted glyph tint, no CRT screen flash, static fixture ages).
