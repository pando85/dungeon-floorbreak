# F02 — CONCRETE HELL: visual direction and asset production bible (v0.1)

**Status:** design proposal for ORIGINAL pixel-art production. No art masters, production sprites or approved illustrations are claimed here. 2026-11-22.

**Read:** [game bible](../01-design/floor-02-last-lesson-design-bible.md), [global art direction](art-direction.md), [antihero tone](../01-design/antihero-tone.md), [pixel-art pipeline](../07-agents/pixel-art-pipeline.md), [agent production plan](../07-agents/floor-02-autonomous-workorder.md). Approved Carl/Donut identity sizes, silhouettes and palettes win over this new environment guide. No copying game screenshots, book cover illustrations, logos, logos of real bands, music or recognizable third-party artwork.

## Casting/readability revision (2026-11-22)

The Mind Horror must read as a **floating cranial/brain mass with tentacles**, per secondary Floor 02 creature descriptions, not a generic elongated monster or humanoid mage. Update the A-signature wording below accordingly. The Rev-Up combat supervisor is visually a **Laminak elite fairy executive**, separate from the ground-level Clurichaun salesman; "foreman" is a role alias. See [enemy acting and animation bible](../01-design/floor-02-enemy-acting-bible.md) for expressive keys and the actual source-art gate. No produced concept sheet automatically replaces owner-approved protagonists or becomes a production sprite.

## 1. Art thesis: clean concrete, filthy intentions

The First Floor feels warm, organic, medieval-industrial and claustrophobic. The Second Floor begins **sterile and pale**: great expanses of pale floor and white cinderblocks, orange glowing lichen in wall seams, fluorescent light, emergency signage, abandoned desks, ugly pipes and modular security doors. The horror comes from things that have appropriated this clinical infrastructure. Keep it a darker, 1980s–1990s action-film-industrial aesthetic; avoid generic medieval tavern/castle.

Every setpiece must read as one instantly recognizable silhouette:
- **S00 threshold:** a vertical seam of warmth into huge sterile white slabs, a very large arrow.
- **S01 complaints:** broken queue barriers and a giant numbered-service display.
- **S02 Rev-Up:** pyramid of crooked sales booths and suspended bottles.
- **S03 Krakaren:** a corporate stage where a huge *wet tentacle shape* destroys signage.
- **S04 cleanup:** a giant diagonal refuse conveyor and low extractor.
- **S05 Meadow Lark:** long linked wheelchair silhouettes, warm human lanterns.
- **S06 Rage:** the sterile bathroom sign and an *oversized violet-black shadow*.
- **S07 Bomb Chicken:** emergency-red lighting and monumental descending stairs.
- **S08 Kennels:** hanging dog cages + invented underground black-metal venue branding.
- **S09 Ralph:** huge empty arena and a tiny cage in a literal oversized spotlight; relief comes from tiny Mongo in quiet warm light.

Scene composition never depends on adding a thick visual haze or high-frequency noise to every background tile. Silhouettes, move tells, dangerous lanes, foreground cover and HUD contrast come first.

## 2. Palette family (provisional art notes, not game constants)

| Purpose | Suggested sRGB HEX | Usage |
| --- | --- | --- |
| deep charcoal concrete | #30383D | low background, sharp shadows and girders |
| bone-white walking line | #AFB7B3 | floor and nonhazard structure; never white-out native protagonists |
| cold highlight | #D8E1DE | fluorescent seams only; not whole-screen glare |
| lichen orange | #DB7130 | growing through cinderblock joints, **not** an attack warning by default |
| utility teal | #78B7AE | refuge, navigational calm |
| warning red | #D4473A | attack tells, active dangerous machines, emergency runway |
| unreliable violet | #6E4D92 | Rage presence, distinctly NOT an ordinary boss HP meter |
| warm human amber | #F0B75A | Meadow Lark, Mongo epilogue, protection and brief relief |

The art review must verify in-scene color against **approved hero palettes** and actual 1x sprites; HEX values are a launch direction, not a mandate to recolor characters.

## 3. Three density layers at native virtual resolution

1. **Background:** 8/16/32-pixel rhythm, quiet wall joints, large silhouettes and sparse lichen. Parallax has low contrast; no flickering detail on attack telegraphs.
2. **Interactive world:** visually separable props, enemies, ledges and attack volumes. Interactive red signs, physical barrel anchors and collision-shape outlines in debug mode. A destructible sales sign must not resemble an inert sign.
3. **Foreground/action:** clear attacks and physical contact events; particles short-lived, capped, with unambiguous direction. Rare earned death-slowdown changes time/presentation only, not authoritative damage.

At 640x360 a player must see **which object can be hit, where danger will land, and which entity caused the collision**. At 640x400 no extra clipping from UI. No 0.5x shrinking to fit multiplayer; camera follow rules must preserve the duo.

## 4. Visual signatures and animation keys

### A. Mind Horror — psychological threat without visual obstruction
Silhouette: unsettling elongated mass, clearly unlike goblins. Keys: idle -> recognizable attack inhale -> directional psychic cone cue -> recover exposed -> stagger/defeat. Communicate Carl's pressure via small character-side stagger cues, not 50% full-screen distortion. Donut stays visible above/behind it.

### B. Clurichaun marketing army
Comedic class contrast: tiny overconfident seller with cheap promotional pennant, armored foreman with bureaucratic badge, slingshot projectile with visible flight. Distinct 2-stage attack tells. Never depict recognizable product trademarks or copy real MLM corporate iconography.

### C. Krakaren Clone
A very large *occluded* shape behind a ridiculous small corporate dais. Three thick readable limbs, distinct left/center attack silhouettes, a real core and vulnerable recovery. Attacks can span the arena but never disappear against gray-white stone. On defeat, the stage collapses with limited original art fragments and a separate visual swarm.

### D. Corpse ecology and Brindled Vespa
Grubs may appear by the hundreds in the background, but gameplay simulates a bounded number of **cluster actors** with legible feet/contact and very few independent collisions. Vespa should read as an airborne predator with a single high-contrast acid warning vector, never as a miniature generic bat.

### E. Meadow Lark Express
Production target: distinctive wheelchair/personal silhouettes and readable warm lanterns. Dignity first: non-combatants are not crash-test dummies, comic collectibles or splatter props. Use practical patched metal, straps, linked wheel profiles, co-op safe camera. The convoy is a memorable horizontal graphic element, not an NPC navigation problem.

### F. Rage Elemental
Scale through **partial views and negative space**: limbs/eye glow/warped light seen behind large doorframe, rather than a huge high-poly sprite always onscreen. A pulse darkens only environment layers while red/teal safety signs remain distinct. One warning-silhouette pose, one telegraphed strike, one locked pursuer state, one authored stairwell fall/expulsion sequence. No fake health bar.

### G. Kobolds, danger dingoes, Ralph
Metal helmets and rigid lance line contrast with animal movement. Rider and mount separation need unique readable silhouette and dismount VFX. Dog helper status must be visible using original motion/stance, not simply recoloring a red enemy green. Ralph stays extremely small: bright eye, dust streak, scratch line, spotlight track and recovery wobble make his movements readable. The game must never enlarge his physical hurtbox by huge unexplained dimensions.

### H. Mongo
The existing approved Mongo concept references are the identity starting point. This Floor 02 key is a **small wary hatchling**: idle/wary -> tentative approach -> bond response. No adult combat frames or mount animation required yet. A deliberately quiet shot may be more emotionally powerful than another explosion.

## 5. Mandatory environment kits

| Kit ID | Deliverables | First stage |
| --- | --- | --- |
| F02-ENV-01 | white cinderblock + joint cracks + orange lichen + walking-line collision samples + cold lights + doorway + huge directional signs | S00–S01 |
| F02-ENV-02 | queue barrier, service counter, number display, shattered office props | S01 |
| F02-ENV-03 | crooked marketing booth, jugs, rack, kickable stand, falling sign, safe telegraph | S02–S03 |
| F02-ENV-04 | refuse conveyor, incinerator/collector, ducts, bin/cage, hazard tape | S04 |
| F02-ENV-05 | wheelchair convoy segments, human lanterns, patched harness, safe room | S05–S06 |
| F02-ENV-06 | blackout walls, bathroom sign, emergency shutters, launcher/decoy, stairwell | S06–S07 |
| F02-ENV-07 | kennels, speaker rig, cage latch, stage spotlight, tiny arena, pet room | S08–S09 |

Art agents must NOT implement unapproved new collision resolution or floor rules while drawing. Content collision sizes come from dedicated level contracts. Reuse existing art kits or native editable sources wherever possible, and document original art provenance.

## 6. Production art workflow and review gates

**Design** (top tier): create original 1x key images or editable concept keys showing scene silhouettes, native-scale composition and attack readability, specify palette and pivots. **Execution** (medium tier): native editable .ase/.aseprite masters or approved equivalent, sprite atlases and manifests, validators, 1x+2x contact sheets, runtime integration and game-speed evidence. **Independent review** (fresh top-tier): inspect actual game scene + native 1x image + one motion/capture, compare to global art and protagonist bibles. **Owner**: art acceptance is separate from gameplay feel.

Suggested order: **environment F02-ENV-01**, **S02 sales kit**, **S09 tiny Ralph/arena key**, **S06 Rage silhouette**, **S05 convoy**, **S03 Clone**, remaining enemies/props. This is visual coverage ordering, not license to merge an incomplete room over a working branch.

For every key set, record original editable sources, asset provenance, SHA256, bounds, transparency, pivots, palette compliance, native and doubled previews. No image scraped from fan sites or generated existing-game artwork becomes a sprite.

## 7. Visual review questions

- Can the player instantly tell Floor 02 from Floor 01, even in a neutral 640x360 frame?
- Can a player tell the difference between orange **lichen** and red **attack warning**?
- Does the mid-boss have a coherent silhouette and clear vulnerable center despite its limbs?
- Can the convoy be seen without obscuring the fighting protagonists?
- Does Rage feel physically too large to fight without destroying camera readability?
- Is Ralph both funny at first glance and fair at real gameplay speed?
- Does Mongo's brief finale feel earned, rather than another feature to learn?

**Nothing in this document establishes a new approved character design or legal publishing right.**
