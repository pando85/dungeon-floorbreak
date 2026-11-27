# TROG-FX-01: troglodyte death profiles and attack telegraph art

**Status:** implemented as presentation only (execution tier: Claude Sonnet; independent review by Claude Opus pending). **Owner art and game-feel review pending**: nothing here is accepted. Every pixel is a reuse of an approved asset (the TROG-01/02/03 idle frame 0, the GOB-02 gib and blood cels, the Goblin Workshop weight-shadow sheet); no new character key, no new concept art. Rooms: `trog_run` (room 13, #310) and `trog_strength` (room 14, #312).

**Contract.** Presentation only. `MANIFEST_HASH`, every golden, `trog_eval`, `bos_eval` and the tuning pin are unchanged; the window hash equals the headless hash (evidence README). The profile rows, the parts and the telegraph read the defeat event and the committed attack through shared references and write only render state. The global and the cf02 death profile files are byte-identical; the troglodyte rows are a third overlay.

## 1. What was missing

TROG-AD19 left a troglodyte without a death profile: its defeat plays the approved `down` frames and nothing else, and the committed hitbox was drawn as a translucent code-drawn box (backlog art table, room 13 and 14). This work order adds the profiles and the art for both.

## 2. Part cuts (`scripts/author_trogfx01.py`)

Each kind has a one-frame **pose sheet** (its approved idle frame 0, same canvas and pivot as the sprite: the reference the part origins are measured in) and a **parts sheet**. A part is a set of pixels of idle frame 0 moved unchanged into a cell with its bounding-box centre on the cell centre; `frame0_origin` is that centre in frame 0 pixels, so the parts **re-tile the sprite to the pixel** (`tests/test_art_trogfx01.py`). Each opaque pixel goes to exactly one part by the ordered region rules of `CUTS` (pinned measurements on frame 0, x right, y down); scraps of one or two pixels join the neighbour they touch most. The gore layer adds a wound rim on every cut face (dark clot `B`, every third pixel `R`, one wet `P`), never on a silhouette outline; the preset filter hides it at gore preset 1 like every other set.

| Kind (sprite) | Cell | Cuts (frame 0 pixels) | Gibs and drop | Palette |
| --- | --- | --- | --- | --- |
| Pygmy (80x56, pivot 30,55) | 32 | head 25x21, torso_upper 20x13, torso_lower 18x14, arm_near 16x19, leg_near 13x8 | `gib_flesh_a`, `gib_flesh_b`, `gib_bone`, `gib_rag`, `drop_blood` | 16 keys |
| Basher (112x88, pivot 48,87) | 56 | head 31x24, torso_upper 46x30, torso_lower 30x14, arm_near 16x32, arm_far 12x30, leg_near 15x16, leg_far 15x26 | same | 19 keys |
| Virtuoso (128x88, pivot 44,87) | 40 | head (hair) 33x24, torso_upper 24x17, torso_lower 13x18, arm_near 25x17, leg_near 13x26, leg_far 12x8, **bell** 7x11 | same | 20 keys |

* **Palette rules.** The troglodyte family keys keep the approved sprite's own colours; the only added keys are the three GOB-02 blood keys `R`, `B`, `P`. Each sheet lists exactly the keys it uses and stays within 20.
* **Gibs and the drop** are the GOB-02 cels recoloured by key to the troglodyte skin (`a s m l` to `d s m l`, tooth `w`) and to the kind's cloth (`gib_rag`: `c C` for the Pygmy and Basher, the robe `u v V` for the Virtuoso). Decals (pools, splats, wall spatters) stay the goblin set's, like the Engineer's: blood is blood.
* **The Virtuoso's bell** (the gold bell it holds, `t T` keys) is the kind's signature gag piece: a clean brass part that clatters, like the Engineer's pot, with no wound.
* **The Pygmy has no far arm or far leg** in its crouched frame 0 (one silhouette); the Basher and Virtuoso tables list only parts their sheet has.
* The cuts are **chunks along straight region lines** (the sprites overlap heavily: hair, arms and robes), not literal anatomy; a torn edge is what a burst makes. Whether they read as bodies is the owner's art call (TROG-FX-Q1).

Files: `assets/source/characters/troglodyte/death/trog_{pygmy,basher,virtuoso}_death_{pose,parts}.{grid.json,ase,pixelart.json}` and `assets/manifests/trog_*_death_{pose,parts}.json` (the same manifest schema the presenter reads for the goblins, the llama and the Engineer; `variant` `TROG`). Scripts: `python3 scripts/author_trogfx01.py sheets [--check]`; `debug OUT_DIR` renders each cut at x8 for review.

## 3. Profile values (`assets/data/death/species_death_profiles_trog.json`)

A separate overlay (`DeathProfiles::with_trogs`), schema 2, `add_species` only, placed in front of whichever profiles the presenter has (global, or the cf02 overlay). One row set per kind, `trog_pygmy`, `trog_basher`, `trog_virtuoso` (the `Species::kind_name` the slot hook already returns), `body_material` organic:

| Cause (force) | Finish | Draws |
| --- | --- | --- |
| Point blank blast (`blast`, catastrophic) | `trog_<kind>_exploded` | the burst: the sprite is replaced by the kind's parts at gore presets 2 to 4 (preset 1 keeps the `down` sprite and throws two rags) |
| Jab or kick (`blunt`, routine or heavy), ring blast (degraded to `blunt`) | `trog_<kind>_down` | the approved `down` frames stay; blood and a few recoloured gibs (preset 4: spray, drops, pool, as for a goblin's kill) |
| Crush, shear, shore block | fallback `trog_<kind>_down` | no troglodyte room has such a cause; no head or block is invented |

**Explosion tables** (`TROG_EXPLOSION_TABLES`, the goblin rows with the kind's own parts; gibs, rags, blood drops, pools, wall spatters, burst scale and flash are the goblin preset-4 punch-up unchanged):

| Preset | Pygmy | Basher | Virtuoso |
| --- | --- | --- | --- |
| 1 | two rags | two rags | two rags |
| 2 | head, torso_upper | head, torso_upper | head, torso_upper, bell |
| 3 | head, torso_upper, arm_near, leg_near (4) | same (4) | the same plus bell (5) |
| 4 (default) | head, torso_upper, torso_lower, arm_near, leg_near (5) | the Pygmy's five plus arm_far, leg_far (7) | head, torso_upper, torso_lower, arm_near, leg_near, leg_far, bell (7) |

Preset 4 is the shipping default (owner 2026-11-18); the death-lab intensity-4 rules (more and larger fragments, thicker pools, two extra wall spatters, a 2x burst and a 3 tick flash) apply unchanged because the tables inherit the goblin row. The presenter spawns a death set per kind (`TROG_FIRST_VARIANT + index`, after the Engineer so no earlier variant moves); with no troglodyte art loaded the species is unknown and a defeat draws nothing, exactly as before.

**Evidence flag.** `--death-lab-trog exploded|down` plays one finish for every troglodyte defeat (presentation only, not a recording run), because the room pilots kill with jabs and dynamite is not in them (precedent: `--death-lab-engineer`).

## 4. Attack telegraph

The stand-in was a translucent box over the committed hitbox. It is now the approved **Goblin Workshop weight-shadow** sheet (`f01-gw-tell-shadow`, AREA-GOBLINWORKSHOP-01: a dark dithered floor patch that flashes through its tell and goes solid when the weight lands), cropped to the **exact committed rectangle**:

| Part | Rule |
| --- | --- |
| Source | the sheet's centre columns 28..35 (a flat 8x8 pattern in every frame: tell phase 0 and 1, solid patch 2), tiled left to right and clipped on the last piece, so the drawn width equals the rectangle's and never shows the weight shadow's rounded ends (a 64 px ellipse would overstate a 17 px smash) |
| Frame | the sim's own window: dither (flash phases, the approved 4 tick clock `FALL_SHADOW_TELL`) from the tell's first tick until the hitbox is live, the solid patch (`FALL_SHADOW`) while it is (`strike_start..=strike_end`) |
| Where | the Basher's smash and the Pygmy's whole leap path (`Trog::threat`, the take-off to the landing) on the floor under the rectangle; the Virtuoso's clipped tongue band centred on the band's own height |
| Width | the rectangle's x extent, capped at 224 px (28 tiles; the data's widest, the Pygmy's path, is under it) |
| Alpha | 0x70, the Workshop shadow's |
| Fallback | where the atlas is missing the code-drawn box stays |

The sprite's own wind-up frames (7 and 8) and strike frames (9 and 10) already draw the body half of the tell (TROG-AD39). **Timing is unchanged**: the patch exists exactly while `Trog::threat` does (a test steps all three species through a full attack), and the tuning file (`TUNING_HASH`) is not touched.

## 5. Adopted defaults

| ID | Default |
| --- | --- |
| TROG-FX-AD1 | Reuse approved pixels only: the parts are cuts of the committed idle frame 0, the gibs and drop recoloured GOB-02 cels, the telegraph the committed Workshop sheet. No new character key, so no top-tier design tier; execution tier only. |
| TROG-FX-AD2 | One pose sheet per kind (the idle frame 0) rather than none: it fixes the part origins to the sprite's own pivot and lets the existing manifest loader read the set unchanged. No pose finish exists for a troglodyte; the pose frames are never drawn. |
| TROG-FX-AD3 | Cut by straight region rules and keep flat cut faces with a gore wound rim, rather than redrawing limbs. The Pygmy has no far limbs; only the Virtuoso has the bell. |
| TROG-FX-AD4 | Two finishes per kind: exploded (point blank blast) and down (everything else); crush, shear and the shore block take the fallback because no troglodyte room has them. |
| TROG-FX-AD5 | The profile rows live in a third overlay file; the global and cf02 files stay byte-identical, and a run without the art reads the profiles it always did. |
| TROG-FX-AD6 | Decals stay the goblin set's. The troglodyte sheets carry no decal frames. |
| TROG-FX-AD7 | A slot enemy has no blast centre in the presenter's observation: the burst originates under the body (the synthetic impact the boss and the lab already use), not at the charge. |
| TROG-FX-AD8 | The sprite of an exploded troglodyte is hidden from gore preset 2 on (the goblins' rule); at preset 1 the `down` frames stay. |
| TROG-FX-AD9 | The telegraph is the Workshop shadow tile cropped to the committed rectangle (no stretch), floor level for the floor attacks, band level for the tongue; the box stays as the missing-atlas fallback. |
| TROG-FX-AD10 | The evidence lab flag `--death-lab-trog` exists because the pilots never blast; it is presentation only. |

**Horizon fit.** `character-progression-horizon.md` is unchanged: this adds no move, ability, unlock, terrain interaction or co-op rule, only the cosmetic finish and telegraph of enemies that exist.

## 6. Open questions

| ID | Question | Default until answered |
| --- | --- | --- |
| TROG-FX-Q1 | Do the straight-cut chunks read as torn bodies at gameplay scale, or should the Basher and Virtuoso get redrawn limbs (a top-tier design pass)? | Chunks as drawn. |
| TROG-FX-Q2 | The Virtuoso's tongue is drawn in the sprite at a fixed reach while the committed band is clipped by solids; is the mid-air patch enough, or should the tongue frame follow the clip? | The patch is exactly the committed band. |
| TROG-FX-Q3 | Should a heavy blunt kill (a kick) get a slammed or flung finish for the Basher as it has for the goblins? | It keeps the `down` frames and the blood. |
| TROG-FX-Q4 | Audio for the burst and the tells (no audio exists for the troglodytes). | None. |
| TROG-FX-Q5 | If a later room puts a crush or a shear cause on a troglodyte, which finish does it play? | The fallback `down`. |

## 7. Tests and evidence

* `tests/test_art_trogfx01.py`: generator equality, the parts re-tile frame 0 exactly, cells and margins, palette rules, wounds on cut faces only, the manifest contract, toolkit validators.
* `src/death.rs`, `src/death_present.rs`: a profile and a table per kind, every table part exists in its sheet and fits its cell, the blast throws only the kind's set and hides its slot at presets 2 to 4, a jab keeps the frames and adds blood, no art draws nothing, determinism, the goblin pieces unchanged by the sets.
* `src/trog_art.rs` tests: the telegraph sheet is the approved sheet, the tile columns are flat, the pieces cover the rectangle exactly, the frame follows the hitbox, a full attack of each species draws exactly while the sim's threat exists, drawing never changes the state hash.
* Evidence: [`docs/evidence/trog-fx-01/`](../evidence/trog-fx-01/README.md): window stills at 640x360, each captured twice byte-identical with the window hash equal to the headless hash, and the parts contact sheet at x4.
