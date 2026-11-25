# CARL-GAUNTLET-01: Carl's reinforced fist, design spec

**Status:** design spec, three key frames frozen and the execution done: the four overlay sheets (98 frames, 95 of them stamped by `scripts/author_carl_gauntlet.py`, Sonnet), manifests, validators (V18, CG-L1 to CG-L9), tests, the Bevy presenter and the evidence are in the PR (section 13). The silhouette gate passed with notes; the notes are folded into the keys (section 10). Independent top-tier art review pending; owner art review pending; not owner-approved.

**Route:** difficulty score 1 (a small, new prop design on an approved character, no new pose), class Easy, route **C + silhouette gate on the keys** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)). Work order: CARL-GAUNTLET-01 (art audit 2026-11-24). It closes the art half of CP-Q02 and CP-Q43 of the [Carl progression spec](../03-tech/carl-prog-01-floor01-combat-progression.md): the gameplay capability `fist_reinforced` is merged (CP-D, #228) and grants on the Juicer's defeat, but Carl's sprite never shows it.

**Authorities:** [Carl character bible](carl-character-bible.md) (locked look, gear "only when acquired in canon", line 38 on milestones), [antihero tone](../01-design/antihero-tone.md) (review gate), CP-Q02 (grant point) and CP-Q43 (battered, practical gear; heavier punches, the same clumsy everyman; no glow or aura) in the progression spec, the ANIM-CARL-02 sources `assets/source/characters/carl/carl_h52_{locomotion,melee,cf02,dynamite}.grid.json` and their manifests, [STYLE_NOTES](../../assets/source/characters/carl/STYLE_NOTES.md), the [pixel-art pipeline](../07-agents/pixel-art-pipeline.md), the [asset pipeline](asset-pipeline.md) and the [character progression horizon](../01-design/character-progression-horizon.md) (item 6: "the gauntlet modifies Carl's fist attacks").

## 1. Canon and tone check

| Detail | Label | Note |
| --- | --- | --- |
| Carl gets a war gauntlet as loot from the Juicer (CP-Q02) | `canon verified` (two fan summaries, chapters approximate; see the CP-Q02 research note) | The grant point is already game data; this spec adds only the look |
| What the gauntlet looks like | `art-direction choice` | Neither source describes its look in a way this spec can cite. The look below is a design interpretation and is not presented as canon |
| Which hand wears it | `art-direction choice` | The game mirrors Carl's sprite (`flip_x`), so handedness cannot be canon at this scale. The gauntlet sits on the **camera-near hand**, the haymaker hand, the move the gauntlet makes pierce (CP-Q40, F-12) |
| Data and engine ids | IP-neutral | `fist_reinforced`, `gauntlet` layer, `*_gauntlet` atlases. The item's franchise name appears nowhere in data, code or file names |

**Tone review gate** ([antihero tone](../01-design/antihero-tone.md), review gate):

1. Normal person under pressure: yes. The gauntlet replaces only the hand and wrist pixels; the body, face, jacket, boxers and Crocs are untouched and no pose changes. No bulk is added to the arm or shoulder.
2. Earned and messy: yes. It is loot taken off a gym brute, strapped over a cheap wristwatch with a mismatched brown strap; the steel is dented and notched.
3. Danger and dark humour: the heavy glove next to the heart boxers is the joke; the heavier fist read sells the stakes.
4. Pixel readability and material rules: the silhouette gate passed at 1x and 3x (section 10).
5. Lore and provenance: the look is labelled an art-direction choice; provenance is recorded in the PR and the manifests.
6. Original expression: hand-placed pixels on the approved Carl sources; no concept image, film prop or third-party design is copied or traced.

## 2. Adopted defaults

| ID | Default | Why |
| --- | --- | --- |
| CG-AD1 | **Overlay, not redrawn frames.** The gauntlet is a separate, full-cell overlay atlas per body sheet (same 48 x 56 cells, same frame order, same tags, same pivot), drawn on top of the bare Carl sprite. The body atlases do not change | Body atlases, their manifests, keys, fixtures and every golden stay byte-identical. The gauntlet stays a replaceable fan-concept layer (one switch). It needs no second copy of 98 body frames that would drift from later body fixes |
| CG-AD2 | **Camera-near hand only** (`near_arm` layer). The jab hand (`far_arm`) stays bare | The near hand is the haymaker (the pierce move) and is the hand the player sees. Two gauntlets would double the pixel work for a hand that is mostly hidden behind the body |
| CG-AD3 | **The watch stays visible**, strapped over the gauntlet's cuff; the overlay never covers a `watch` or `pin` pixel, nor a visible `prop` pixel (CG-L2) | The watch is a Carl identity anchor of the approved master |
| CG-AD4 | **Palette:** existing `carl-book1-v0` keys only: `K J j T U V w H`. No new colour, no `Y`, no glow; `w` (steel glint) at most 3 px per frame | CP-Q43 (no glow or aura); the steel reads through value contrast, not new hues |
| CG-AD5 | **Sheets:** overlays for `locomotion` (16), `melee` (24), `cf02` (13) and `dynamite` (45): 98 frames. `interact` is deferred (not drawn by `floor01_scene`) | The four sheets cover every Carl pose the player sees in rooms 5 and 6 |
| CG-AD6 | **Presenter copies the visible body sprite.** One overlay entity per Carl body sprite entity (`ActorVisual`, `CarlMelee`, `CarlThrowSprite`, `CarlCf02Sprite`) copies that sprite's visibility, atlas index, `flip_x`, colour and translation (z + 0.01) every frame, and is shown only while the capability is held | Anchoring is implicit: the same cell index and the same flip give the same pivot, so no per-frame anchor table exists to drift |
| CG-AD7 | **Brown wrist strap** (`H`) between the jacket sleeve and the glove | Gate note: with plum leather only, the glove read as the sleeve running into a dark mitt. The strap separates the two and says "leather"; `H` is the approved hair brown, so no new colour |
| CG-AD8 | **Brightest steel on the striking edge** in punch frames (the leading knuckle plate) and a top-left lit plate elsewhere | Gate note: keep the hit point as punchy as the bare skin fist was |

## 3. Silhouette and materials

- **Shape:** a squarish, slightly oversized fist, about **1 px heavier** than the bare hand on its leading or bottom edge (one extra outline row or a 1 px knuckle-bar protrusion). Never a spiked, clawed, flared or "power glove" shape. The forearm and sleeve outline do not change.
- **Materials, outer to inner:** a steel back-of-hand plate (`w`/`V` lit, `U` base, `T` shadow and dents), a steel knuckle bar on the striking edge, dark plum leather fingers and palm (`j`/`J`), and a brown leather wrist strap (`H`) between glove and sleeve. The watch strap sits over the cuff.
- **Wear:** at least one dent or notch per visible plate (a darker pixel that breaks a highlight run). The steel is never a clean gradient.
- **Read at 1x:** a dark block with a cool grey highlight where the bare hand was a warm skin spot. That value swap is the whole read; detail below it is for the 3x view.

## 4. Palette

| Key | Name | Hex | Role on the gauntlet |
| --- | --- | --- | --- |
| `K` | outline_plum | `#1b1124` | outline, finger gaps |
| `J` | jacket_shadow | `#251a2e` | leather shadow |
| `j` | jacket_base | `#352540` | leather fingers and palm |
| `T` | jacket_sheen | `#5a4068` | steel shadow, dents, notches |
| `U` | boxer_deep | `#82689a` | steel base |
| `V` | boxer_shade | `#c3a9c4` | steel lit face, knuckle bar |
| `w` | watch_face | `#b8c0d8` | steel glint (accent, at most 3 px per frame) |
| `H` | hair_mid | `#42231d` | brown leather wrist strap |

Any other key in the overlay is a validator error. The skin keys `L M S D` never appear in the overlay.

## 5. Frame list

Every frame of each covered sheet gets an overlay frame at the same index; a frame where the near hand is hidden or off the sheet has an **empty** overlay frame (allowed, but listed by the validator).

| Sheet | Frames | Tags (frames) | Notes |
| --- | --- | --- | --- |
| `locomotion` | 16 | idle 0-3, run 4-9, jump_start 10-11, jump_rise 12, jump_fall 13, land 14-15 | key `idle_hand` = frame 0; frames 1-15 follow the hand wherever the body frame puts it |
| `melee` | 24 | punch_a 0-3, punch_b 4-7, kick 8-14, hurt 15-17, knockback 18, knockdown 19-20, getup 21-23 | keys `punch_b_windup` = 4 and `punch_b_contact` = 5. In punch_a (the far-arm jab) the near hand is not the striking hand: gauntlet it where the body frame holds it |
| `cf02` | 13 | stomp 0-2, air_kick 3-4, dive_tuck 5, dive_plunge 6, dive_land 7, bailout 8-9, flinch 10, frustration 11-12 | frustration shakes the near fist: keep the strap and plate readable |
| `dynamite` | 45 | the 7 throw tags | when the stick is in the near hand, the overlay never covers a **visible** `prop` pixel (a stick pixel that shows). In the body sheets the near arm is drawn over the `prop` layer, so the fingers already lie over the stick and a glove on those skin pixels hides no stick pixel (approved by independent Opus review, PR #284, CG-EX-AD2) |
| `interact` | 0 | deferred (CG-AD5) | not drawn by `floor01_scene` |

Total: **98 overlay frames**, 3 of them the frozen keys.

## 6. Anchor rule

- The overlay cell is the body cell: same canvas (48 x 56), same pivot (25, 55), same atlas grid, same frame index, same flip. Nothing is anchored per frame.
- Airborne lift, hit-stop shake, knockback slide and any other offset the presenter applies to the body sprite's transform are inherited by copying that transform (CG-AD6).
- The overlay never needs pixels in column 0 or column 47, so the flip cannot shift it by a pixel against the body.

## 7. Layering rules (validator rules)

For each overlay frame `O` and its body frame `B` (the body sheet's layers composed):

| Rule | Check |
| --- | --- |
| CG-L1 | Every opaque pixel of `O` lies in `dilate1(near_arm(B))`, the near-arm layer grown by 1 px (8-neighbourhood) |
| CG-L2 | `O` covers no pixel of `watch(B)` or `pin(B)`, and no **visible** prop pixel: `prop(B)` minus `near_arm(B)` (the near arm is drawn over the prop layer). Approved by independent Opus review (PR #284, CG-EX-AD2) |
| CG-L3 | Outside `near_arm(B)`, `O` covers no pixel of `head(B)` or `hair(B)` |
| CG-L4 | At most 12 pixels of `O` lie outside `near_arm(B)` (the keys use 12, 3 and 11) |
| CG-L5 | Keys are only `K J j T U V w H`; `w` at most 3 per frame |
| CG-L6 | Every **exposed** skin pixel (`L M S D`) of `near_arm(B)` (not drawn over by the `watch` or `pin` layer) is covered by `O`, except the frozen exception list in the overlay manifest. An exception is a near-arm skin pixel that is **not on the hand** (the jaw-line pixel of the head on the arm layer, or a chest-edge shading pixel the arm layer carries inside the sleeve): key frames one, `punch_b_contact` (35, 19); the execution step lists 9 more (section 13). Approved by independent Opus review (PR #284, CG-EX-AD3) |
| CG-L7 | Columns 0 and 47 of `O` are empty |
| CG-L8 | Lint: `B + O` composed as one frame adds **no new isolated or orphan pixel** over `B` alone (orphans inherited from the approved body frames are reported, not failed) |
| CG-L9 | An empty `O` is allowed only where `near_arm(B)` has no skin pixel; empty frames are listed in the validator output |

## 8. Presenter rule

- Module `src/bin/floor01_scene/carl_gauntlet.rs` (the `pierce_spark.rs` pattern). It loads the four `*_gauntlet` atlases with the same `TextureAtlasLayout::from_grid` as their body sheets.
- One overlay entity per Carl body sprite entity (CG-AD6). Each frame, after the body sprites are synced, the overlay copies the body sprite's `Visibility`, atlas index, `flip_x`, `color` and `Transform` (z + 0.01).
- **Capability gate:** the overlay is visible only while the Carl actor's combat state holds `fist_reinforced` (`CombatState::story_holds(FIST_REINFORCED)`; `--carl-gauntlet`, `--f01-loadout all`, the story grant on the Juicer's defeat, and every story pin that holds it). Before the grant it is hidden; on the grant tick it appears with the next presented frame. No fade, flash or glow on the grant (the existing `HEAVY FIST` notice is the cue).
- **Hash-neutral:** read-only. It adds no `CombatEvent`, writes no simulation state, is never part of a hash, replay, playtrace pin or golden. Running with and without the overlay gives byte-identical simulation output.
- Donut and every non-Carl actor never get an overlay.

## 9. Key frames

Source: `assets/source/characters/carl/gauntlet_keys/carl_h52_gauntlet_keys.grid.json` (pixel-grid-v1, the 11 body layers copied from the ANIM-CARL-02 frames plus a `gauntlet` layer on top). These keys are **frozen**: the execution step copies their `gauntlet` cels into the overlay grids unchanged.

| Tag | Body frame | Overlay box (x, y, w, h) | Overlay px | Outside `near_arm` | `w` | Read |
| --- | --- | --- | --- | --- | --- | --- |
| `idle_hand` | locomotion 0 | 15, 32, 9, 8 | 46 | 12 | 1 | hanging fist, knuckles low; brown strap above and below the watch; plate notch at (18, 35), dent at (20, 37); one extra bottom row |
| `punch_b_windup` | melee 4 | 12, 11, 7, 4 | 24 | 3 | 2 | fist cocked by the head; knuckle bar on top; strap row 14 separates fist and sleeve; the hair pixel (18, 14) stays uncovered |
| `punch_b_contact` | melee 5 | 40, 22, 7, 8 | 45 | 11 | 1 | extended fist; leading knuckle bar protrudes 1 px up, glint (45, 23) at the striking corner; leading plate notch at (45, 26); strap column x 41; one extra bottom row |

**Lint note:** `python3 scripts/pixel_grid_to_ase.py lint` on the key file reports 0 isolated pixels and orphans `[0, 3, 4]`. All seven orphans are inherited from the approved melee frames 4 and 5 (windup (18, 11), (18, 14), (24, 19); contact (27, 23), (27, 29), (30, 20), (33, 26)); the `gauntlet` layer adds none (measured as body + overlay against body alone). `lint --strict` therefore exits 1 on this file and on the melee overlay review grid; that is expected and CG-L8 is the rule.

Evidence: [`docs/evidence/carl-gauntlet-01/`](../evidence/carl-gauntlet-01/README.md).

## 10. Silhouette gate

A fresh reviewer that had not seen the design looked at the x8 pairs, the x20 hand zoom and the Floor 1 scene at 640 x 360 (1x) and 1920 x 1080 (3x) with six Carls: bare idle, gauntlet idle, gauntlet windup, gauntlet contact, bare contact and a mirrored gauntlet idle.

- **Result: PASS with notes.** Gauntlet and bare Carls are told apart at 1x (the warm skin spot becomes a dark block with a cool steel highlight); it reads as battered metal-and-leather gear with no glow or superhero bulk; Carl stays the approved character; the contact fist reads mildly heavier with the pose intact. No blocking defect.
- **Notes folded into the frozen keys:** a brown leather strap (`H`) separates the glove from the jacket sleeve in all three keys (CG-AD7); a darker notch breaks the steel highlight on the idle plate and the leading contact plate; the striking corner keeps the brightest steel (CG-AD8). The windup is the weakest read at 1x by nature (the fist sits against the hair); the strap row is its separator.
- The renders in the evidence folder show the final keys.

## 11. Acceptance targets

1. Four overlay atlases (98 frames) built from pixel-grid sources, exported deterministically by `scripts/export_assets.sh`, with manifests, checksums and provenance; body atlases and every existing golden, replay and pin byte-identical.
2. The three keys appear unchanged in the overlay grids (frozen-key fixture, SHA-256 per key).
3. CG-L1 to CG-L9 pass on all 98 frames in a toolkit validator and a unit test.
4. In-game: the gauntlet is visible on Carl in room 5 after the Juicer's defeat and in room 6, and invisible before the grant; it follows every pose, flip and offset with no 1 px slip in a capture sequence at 1x and 3x (640 x 360 and 640 x 400).
5. Rust tests: the overlay is hidden without the capability and shown with it; the simulation hash of a fixture run is identical with the presenter on and off.
6. Tone gate (section 1) still holds on the full set: no glow, no bulk, the watch visible, Carl unchanged.
7. Owner art review at the end, on contact sheets, an in-room GIF and the frame-layout and state-flow diagrams.

## 12. Risks and open questions

- **Handedness flips with facing.** Accepted (section 1). If the owner wants a fixed hand, that needs a second overlay set on `far_arm` keyed by facing; it is not planned.
- **Two hands in a frame.** In some melee and cf02 frames both hands are visible; only the near one is gloved. Accepted by CG-AD2.
- **`interact` sheet** deferred (CG-AD5); a later scene that draws it needs its 12 overlay frames.
- **Canon look** unknown; if a cited source describes it later, the overlay is redrawn without touching the body sheets (CG-AD1).

## 13. Execution notes (Sonnet) and adopted defaults

The design above is unchanged. Execution choices, each recorded as an adopted default:

| ID | Default | Why |
| --- | --- | --- |
| CG-EX-AD1 | **The overlay grids hold the `gauntlet` layer only** (`carl_h52_<sheet>_gauntlet.grid.json`, palette = the 8 gauntlet keys). The body layers are not copied; the toolkit config's `overlay.body_grid` names the body sheet and the rules compose body + overlay on the fly. The combined review view is derived by `scripts/make_carl_gauntlet_evidence.py` | The export of the overlay atlas is then the gauntlet layer alone with no export option, and there is no second 4 MB copy of the body frames to drift (the point of CG-AD1). The keys file keeps its combined layout |
| CG-EX-AD2 | **CG-L2 judges the visible prop pixels** (approved by independent Opus review, PR #284). The near arm is drawn over the prop layer, so a prop pixel under a near-arm pixel does not show; the overlay may cover the skin pixels that lie on the stick (dynamite 1, 4, 9 to 14 and 27 to 32: 2 or 6 pixels each) and still never covers a stick pixel that shows | Otherwise CG-L2 and CG-L6 cannot both hold on those frames (bare skin would be left on a gloved hand), and no visible stick pixel is hidden |
| CG-EX-AD3 | **CG-L6 counts the exposed skin** (approved by independent Opus review, PR #284, with the 10 exceptions below): near-arm skin pixels that the watch or pin layer does not draw over. **Exceptions** (manifest `overlay.skin_exceptions`): melee 5 (35, 19) as designed, plus 2 pixels in melee 22 and 1 in dynamite 6 and each of 39 to 44: an isolated skin pixel the near-arm layer carries inside the sleeve, not on the hand | Not covered by the glove: it would be a patch on the chest, not a gauntlet |
| CG-EX-AD4 | **Stamping:** the three keys are copied byte for byte; a hand that equals a key's skin shape with the same cuff side (the idle hand in 13 frames, the windup fist in dynamite 4 and 27 to 32) reuses the key by translation; every other hand is built by the key's construction (strap at the cuff, steel plate with a lit knuckle bar and a glint, one dent, a 1 px heavier far edge along runs of two or more pixels), with an orphan repair so CG-L8 holds | The same look as the keys with no per-frame anchor table; the few 1 to 3 px fists of the run frames read as a mitt with a lit knuckle |
| CG-EX-AD5 | **Toolkit tick table in ms** (`tick_hz` 1000) for the overlay configs: the durations are copied from the body sheet, and some (260, 420, 560 ms) are not whole 60 Hz ticks | V03 then checks the copy exactly |
| CG-EX-AD6 | **The presenter skips an overlay atlas that is not exported** (stderr note, Carl drawn bare) and spawns overlays only in combat runs | A stale local export must not stop a run |

No overlay frame is empty (every body frame shows near-hand skin), so CG-L9 never fires.
