# HUD-ART-02: keyboard glyphs, special chips, Donut plate (T07), notice backing plate

Status: **keys drawn, execution done, `chip_puddle` redrawn after review (F1 redraw applied and re-reviewed); owner art review pending.** Extends the hybrid HUD A ([native spec](hybrid-hud-a-native-spec.md)); everything not named here keeps that spec. Presentation only: no gameplay state, no hash, no input semantics change. Review status of every new region: `prototype_fixture_unapproved` until the independent art review and the owner's batch review.

Scope (art audit, section 10, row 7): a keyboard key-cap glyph set (HUD-DEBT-05 leftover, audit H5), special chips for the dynamite and the blink (audit H3), a Donut plate layout that shows `BLINK` (T07, F01-FU-B10), a notice backing plate for dark scenes (CP-B2-FU-NOTICE, audit H4). AUDIENCE-01-A1 is out of scope (HA2-AD5).

**Difficulty (pixel-art pipeline, routing by difficulty):** new design 1, height under 40 px 0, simple construction 0, no VFX states 0: **score 1, Easy, route C** (spec plus hero keys; the executor draws the rest). The keys are new designs, so the silhouette gate applies to them (section 8, AT-10).

## 1. Adopted defaults

| ID | Decision | Why |
| --- | --- | --- |
| **HA2-AD1** (T07) | **Donut's mana track is 72 px outer on every plate state** (x +50..+121, rows +26..+30; inner 70 px at +51..+120, rows +27..+29): 8 segments of 7 px with 2 px gaps (pitch 9), same `mana_v0..v7`. The freed span +124..+157 on the mana row holds the special label, **right-aligned to +157 at text y +28, exactly where Carl's `BOOM` sits**. A socket label must be at most **33 px** wide (`BLINK` is 27). The rule that hid a label which would reach the track is removed: every socket label is drawn on both plates | The plate has no other free row; the ability row is full (two 50 px slots plus the socket). One fixed track length on every state means nothing moves at a grant or a swap. The same label anchor on both plates keeps one reading rule: the word above the socket names the special. Eight 7 px segments still count at 1x (gate, section 7) |
| **HA2-AD2** (chips) | **The socket keeps the button glyph.** The item chips (11x11: `chip_dynamite`, `chip_puddle`) are drawn in a new **item well on the grant notice** (15x15 at notice +182,+8; chip at +184,+10). A well is shown only for a grant that equips the special socket: `GOBLIN BOOM` gets the dynamite on `well_carl`, `PUDDLE JUMPER` gets the puddle on `well_donut`. No fist chip: `HEAVY FIST` and `LIGHT FEET` are passives and their notices keep the full-width headline | **A recorded deviation from "chips in the 11 px socket".** An 11 px cell cannot hold both the item and the button, and there is no free column beside it (slot B ends at +143, the socket starts at +147). The socket is controller truth (which button fires the special) and the label above names the item, so the plate stays complete without the icon. The icon teaches the item where the player first meets it: the grant moment, next to its name, in its owner's well colour. The fist is not useful as a marker: the gauntlet overlay (CARL-GAUNTLET-01) already shows it on Carl's sprite every frame |
| **HA2-AD3** (notice plate) | **The notice frame becomes a plate inside the same 204x32 rect**: ring 0 `hud_ink`, ring 1 a worn bevel (`metal_hi` top and left, `metal_mid` bottom and right, like the hero plates), ring 2 `magenta` (the System's colour), ring 3 `hud_ink`, fill `notice_bg`. The dotted `magenta_dk` rule moves to row +26 (2-on/2-off, x +4..+199). Text positions are unchanged (+7,+6 and +7,+15) | The notice always sits over the near-black foundation band (every f01_late room). Its fill and its outer ink row have the band's value (1.1:1), so only the 1 px magenta line separated it from the world. A lit metal rim makes it an object, in the same family as the hero plates and the CRT, at 4.6 to 4.7:1 against the band in the darkest and the brightest room. The rect, the safe margin (bottom at y 351 / 391) and the slide are unchanged |
| **HA2-AD4** (keyboard glyphs) | **Square key caps, 11x11**, one per keyboard key the HUD or a prompt can name: `key_j` (Primary), `key_k` (Physical), `key_l` (Special), `key_q` (Swap), `key_space` (Jump, 19x11 wide cap), `key_left`, `key_right`, `key_down` (movement). A new `GlyphSet::Keyboard` maps Primary to J, Physical to K, Special to L. The binding map is one Bevy-free table that the keyboard adapter also reads (section 3) | The keyboard set mirrors the pad set (four face buttons plus the swap and the stick directions). The cap shape (square, cut corners, lit bevel, a two-row skirt) differs from the round pad disks in silhouette, not only in the letter, so a player sees at once which device the HUD is talking about |
| **HA2-AD4b** (presenter) | **Last-used device on P1 only.** P1's glyph set becomes `Keyboard` on the first tick with a non-neutral keyboard input and returns to the pad family of P1's pad (vendor rule, unchanged) on the first tick with a non-neutral input from that pad. A tick with both keeps the current set. With no input yet the set stays the default (`Xbox`), so every scripted capture keeps its glyphs. P2 never shows keyboard glyphs. `--glyphs=keyboard` forces P1's set (P2 keeps its pad family) | The keyboard drives P1 only (`PadSlots::take_inputs`). Controller-first: the pad family is the default and the fallback. The state lives in the HUD runtime and `PartyView` only (hash-neutral) |
| **HA2-AD5** | **AUDIENCE-01-A1 arrows: not in this order.** AUDIENCE stays locked by default; no owner verdict exists on the arrows | Out of the slice's readability need |
| **HA2-AD6** (AI plate) | **The AI companion plate keeps the 72 px track (outline ends at +121) and leaves the span from +122 to +145 beside it empty (a human plate's label would start at +124 or later).** No socket label, no glyph | Review F5. HA2-AD1's one track length on every state is what keeps the plate still when Donut changes between human and AI control. The AI plate has no socket and no glyphs ([native spec](hybrid-hud-a-native-spec.md), "AI plates show no glyphs"), so a label there would name a button nobody presses. The empty span is the visible difference between "you drive this" and "the companion drives this". If a later order adds `LB SWAP` to the AI plate, it takes this span |

## 2. Glyph list

All glyphs are 11 px tall, pivot bottom-left `[0, 10]`, drawn at integer positions.

| Region | Size | Symbol (5 px tall, `text_white`) | Used for | Key drawn |
| --- | --- | --- | --- | --- |
| `key_j` | 11x11 | J | Primary (Carl punch, Donut missile) | **yes** (hero key) |
| `key_k` | 11x11 | K | Physical (kick, pounce) | no |
| `key_l` | 11x11 | L | Special socket (dynamite, blink) | no |
| `key_q` | 11x11 | Q | Swap prompt | no |
| `key_space` | 19x11 | an open bracket `W.......W / WWWWWWWWW` (9x2) at cols 5..13, rows 5..6 | Jump prompt | no |
| `key_left` | 11x11 | arrow, mirror of `key_right` | Move prompt | no |
| `key_right` | 11x11 | arrow | Move prompt | **yes** (hero key) |
| `key_down` | 11x11 | arrow pointing down | Drop / down prompt | no |
| `chip_dynamite` | 11x11 | n/a | `GOBLIN BOOM` notice well | **yes** |
| `chip_puddle` | 11x11 | n/a | `PUDDLE JUMPER` notice well | **yes** |

The 5x5 symbols for the caps the executor draws (cols 3..7, rows 2..6 of the cap; `W` is `text_white`):

```text
K        L        Q        down
W...W    W....    .WWW.    ..W..
W..W.    W....    W...W    ..W..
WWW..    W....    W.W.W    W.W.W
W..W.    W....    W..W.    .WWW.
W...W    WWWWW    .WW.W    ..W..
```

`key_left` is `key_right` mirrored horizontally (symbol only; the bevel stays lit on the top and left). The caps carry no vendor art and no vendor colours: they are the HUD metal ramp plus `text_white`.

### 2.1 Key-cap construction (hero key `key_j`)

```text
.KKKKKKKKK.   K hud_ink (outline, 1 px cut corners)
KHHHHHHHHMK   H metal_hi (lit bevel, top and left)
KHLLLLLLLMK   L metal_light (face)
KHL.....LMK   M metal_mid (right edge, bottom lip)
KHL.....LMK   r metal_dark (skirt: the key's side, the depth cue)
KHL.....LMK   . = symbol area (5x5, face colour where no symbol)
KHLLLLLLLMK
KMMMMMMMMMK
KrrrrrrrrrK
KrrrrrrrrrK
.KKKKKKKKK.
```

The wide `key_space` cap keeps every row and stretches the face, the bevel and the skirt to 19 px.

### 2.2 Chips

- `chip_dynamite`: one upright stick (ink outline, `hp_red_hi` lit left column, `hp_red` body, `hp_red_lo` shade column), a `brass_dark` paper band across its middle, a 2 px `brass` fuse and an `amber` spark cross with a `text_white` core (accent). Red is allowed: the dynamite is danger.
- `chip_puddle`: one rising drop over a flat puddle. The puddle is a filled lens the full 11 px wide and 4 rows tall (ink outline, `magenta` body, a `magenta_hi` sheen on the left of the far edge, `magenta_dk` on the right of the near edge), with no dark interior and no ring. The drop is a single 3 px teardrop (`magenta_hi`, a `crt_face_hi` accent core, `magenta` shade on the right), off-centre right at x 7..9, rows 1..4. **No paired features and no closed ring with a dark centre:** the first key (two twinkles over a lozenge) read as a second System face next to the CRT (review F1), so the chip must never offer two eye-like marks on one axis or a mouth-like opening. The magenta family matches the in-world puddle VFX without using its keys.
- Both use HUD tokens only. No new palette entry.

### 2.3 The item well (code-drawn)

At notice +182,+8: a 15x15 `hud_ink` square, a 13x13 `brass` ring inside it, an 11x11 fill in the owner's well colour (`well_carl` or `well_donut`), and the chip on top at +184,+10. With a well the headline must end at or before +179 (172 px from +7); without one the existing limit holds (190 px). Measured: `PUDDLE JUMPER` ends at +103, `GOBLIN BOOM` at +90.

## 3. Binding map and glyph selection

One Bevy-free table, the single source for both the adapter and the HUD:

| Logical action | Key (primary) | Alternates (no glyph) | Cap |
| --- | --- | --- | --- |
| Primary | `J` | | `key_j` |
| Physical | `K` | | `key_k` |
| Special | `L` | | `key_l` |
| Swap | `Q` | | `key_q` |
| Jump | `Space` | | `key_space` |
| Move left / right | `ArrowLeft` / `ArrowRight` | `A` / `D` | `key_left` / `key_right` |
| Down | `ArrowDown` | `S` | `key_down` |

Restart (`R`) and quit (`Esc`) are not HUD actions and get no cap. The table is a `const` of `(KeyAction, KeyCap, &[KeyName])` in a Bevy-free module; the `floor01_scene` keyboard adapter maps `KeyName` to Bevy `KeyCode` and reads its bindings from that table, so a rebinding changes the HUD glyph with it. A unit test asserts every HUD `LogicalAction` has exactly one cap and the adapter's key set equals the table's.

Selection: `GlyphSet::glyph(action)` gains the `Keyboard` arm (Primary `KeyJ`, Physical `KeyK`, Special `KeyL`). `FrameId::of_glyph` maps the new glyphs to the new regions. Nothing else in the draw list changes: a chip is still "glyph, 3 px gap, label".

## 4. Sizes and the socket grid (plate offsets, both plates)

| Row | Rows | Carl | Donut (new) |
| --- | --- | --- | --- |
| Name | +4.. | Jersey 10 at +40 | same |
| HP | +16..+23 | heart +40, bar +50..+145 | same |
| Mana | +26..+30 | none | spark +41; **track +50..+121 (outer 72, inner 70, 8 x 7 px, pitch 9)** |
| Special label | text y +28, right edge +157, max 33 px (left at or after +124) | `BOOM` (advance box +133..+156) | **`BLINK` (advance box +130..+156)** |
| Abilities | +34..+44 | slot A +40, slot B +93, socket +147 (11x11) | same |
| Socket flash | +146,+33, 13x13 | same | same |

The track's right end (+121) and the label's left limit (+124) leave a 2 px clear gap at the longest legal label.

Mana fill: `ManaView::fill_px` is re-expressed against the 70 px inner track (`MANA_BAR_INNER_W = 70`, `mana_fill_px = round(mana / mana_max * 70)`: 0, 9, 18, 26, 35, 44, 53, 61, 70 for `mana_max = 8`, each a whole segment). Segment `k` is drawn at `ix + 9k`, filled `clamp(fill - 9k, 0, 7)` px, the rest `mana_empty`. Fixtures keep their fractions: full 70, boss 45 (was 60/94), co-op 23 (was 31/94, a partial segment). Depleted outline, the outline flash and high contrast keep their colours on the 72x5 rect.

## 5. Palette

No new token. Caps: `hud_ink`, `metal_dark`, `metal_mid`, `metal_light`, `metal_hi`, `text_white`. Chips: `hud_ink`, `amber`, `text_white`, `brass`, `brass_dark`, `hp_red_hi`, `hp_red`, `hp_red_lo`, `magenta_dk`, `magenta`, `magenta_hi`, `crt_face_hi`. Notice plate: `hud_ink`, `metal_hi`, `metal_mid`, `magenta`, `magenta_dk`, `notice_bg`. Well: `hud_ink`, `brass`, `well_carl`, `well_donut`. The HUD swatch budget is unchanged.

## 6. States and layering

States are device-agnostic: a key cap takes exactly the treatment a pad glyph takes.

| State | Socket | Label |
| --- | --- | --- |
| Locked (disabled) | `socket_locked` padlock, no glyph | none |
| Equipped, ready (idle) | the glyph or cap, untinted | `amber` |
| Muted (too few charges) | glyph or cap tinted `amber_dim`, 1 px zig-zag at +44 | `amber_dim` (high contrast: `amber`) |
| Cooling | muted, plus the 1 px underline `ceil(remaining * 11 / 540)` | `amber_dim` |
| Flash ready / refused | 13x13 outline `brass_hi` / `magenta_hi`, 6 ticks, skipped under reduced motion | unchanged |

Layering (existing z bands): notice plate `Z_FRAME`; well rects `Z_BAR`; chip and all glyphs and caps `Z_ICON`; text `Z_TEXT`; muted zig-zag, underline and flashes `Z_FLASH`. The well and the chip slide with the notice (`notice_dx`); the reconnect pause message uses the new plate and never a well.

## 7. Readability gate (design keys at 1x)

Room captures from this branch's build (`--scene f01_late --f01-loadout all`, tick 240, 1x) were measured for mean world luminance (rows 52 to h-56): **darkest `hazards` (45.1 at 640x360, 43.5 at 640x400), brightest `dozer` (75.5 / 70.4)**; `boiler` 49.5, `mastery` 47.3, `corral` 73.1. The keys were composited at 1x onto both rooms at both sizes: Carl's plate with `key_j`/`key_k`/`key_l`, Donut's human plate (from a `hoarder` `direct:2p` capture) with the 72 px track and `BLINK`, and the notice plate with its well (puddle in `hazards`, dynamite in `dozer`; the headline pixels are the game's own 1x notice renders).

Result: **pass at 1x on all four frames.** `BLINK` reads, the 8 segments count, the caps read as keyboard keys and differ from the disks by shape, both chips read (stick and fuse; puddle and drop), and the notice reads as a plate. Notice edge contrast against the band under it: new `metal_hi` rim 4.6 to 4.7:1, old outer `hud_ink` row 1.1:1, old `magenta` ring 4.2 to 4.4:1. Images: [design-gate evidence](../evidence/hud-art-02/design-gate/README.md).

After review F1 the redrawn `chip_puddle` was re-gated at 1x in the notice well, next to the System face, on `hazards` (darkest) and `dozer` (brightest) at both sizes: one drop over a flat puddle, no face reading.

## 8. Acceptance targets (independent art review)

| ID | Target |
| --- | --- |
| AT-1 | At 1x, 640x360 and 640x400, in `hazards` and `dozer`, a window capture shows: `BLINK` fully drawn on Donut's human plate, the 8 mana segments countable, the special labels on both plates aligned to +157 |
| AT-2 | The notice plate's `metal_hi` rim measures at least 3:1 against the median background under the notice edge in every f01_late room (design measurement 4.6:1 or better) |
| AT-3 | Every socket label is at most 33 px (`text_width_px(SilkRegular)`), a unit test over `LabelId::for_socket_text` labels |
| AT-4 | Every new grid passes `pixel_grid_to_ase.py lint --strict`; palettes are HUD tokens only (manifest validator) |
| AT-5 | The caps differ from the pad disks by silhouette: cut 1 px corners and two `metal_dark` skirt rows on every cap (validator on the exported regions) |
| AT-6 | Hash-neutral: every room capture with `--glyphs=keyboard` ends with the window hash equal to the headless hash; committed replays and goldens byte-identical |
| AT-7 | With no keyboard input the glyphs are unchanged (default `Xbox`); the only expected pixel changes in existing captures are Donut's human plate (track and label) and the notice plate |
| AT-8 | P2 never shows keyboard glyphs; P1 switches on a non-neutral input only, and a tick with both inputs keeps the current set (unit tests) |
| AT-9 | The well is drawn only for `GOBLIN BOOM` and `PUDDLE JUMPER`; a headline with a well ends at or before +179 (unit test) |
| AT-10 | Silhouette gate: `key_j`, `chip_dynamite` and `chip_puddle` at 1x on the gate frames read as a keyboard key, a stick of dynamite and a magic puddle |

## 9. Keys and files

| File | Content |
| --- | --- |
| `assets/source/hud/keys/hud_art_02_keycaps.grid.json` | `key_j`, `key_arrow_right` (the cap construction and two symbol styles) |
| `assets/source/hud/keys/hud_art_02_chips.grid.json` | `chip_dynamite`, `chip_puddle` |
| `assets/source/hud/keys/hud_art_02_notice_plate.grid.json` | the 204x32 notice plate (no well; the well is code-drawn) |

The keys are frozen design inputs. The production sources are written by `scripts/author_hud_hybrid_a.py` (caps, chips and the new `make_notice_frame`) and must match the keys pixel for pixel where they overlap (a test). The concept PNGs were not sampled or traced.

## 10. Open questions

- HA2-Q1: whether the item chips should also appear on a future pause or loadout screen (no such screen exists).
- HA2-Q2: the owner's verdict on the deviation in HA2-AD2 (chip on the notice, glyph in the socket). If the owner wants the chip in the socket, the fallback is the chip in the socket with the button glyph moved into the label line (`[L] BLINK`), which needs a 47 px label slot and therefore a 58 px track. **Recommendation for the owner (designer and independent art review agree): keep HA2-AD2 and do not adopt the fallback.** The socket stays controller truth, a 58 px track would shrink the segments until they no longer count at 1x, and the chip is learned at the grant beside its own headline. This holds only with the redrawn `chip_puddle` (review F1), which no longer reads as a face beside the System's CRT.
- HA2-Q3: whether the movement caps should show `A`/`D` instead of the arrows on keyboards where the player uses WASD (the table already lists both; the HUD shows the primary key).
