# THE CRAWL — Monster Death Lab (reference prototype)

**Status:** visual-design REFERENCE / EXPERIMENT; **not** production animation, owner-approved final death-spec, an authoritative combat simulator, or a Bevy rendering capture. The owner selected **intensity 4** as the target direction, and asked that differences between intensity settings be much clearer. **Visual approval of all four exact outcomes remains pending.**

## Open the lab

Open [index.html](index.html) locally in a browser. It has **no network requests, build step, external fonts, packages or runtime dependencies**. Controls:

- **Play/pause**, **timeline scrubber**, **reset**.
- **Intensity 1–4**, defaulting to **4 / MAXIMUM**. Levels 1, 2, 3 and 4 adjust effect density; 4 (owner-decided default, 2026-11-18) additionally enables more and larger fragments, heavier blood and wall spatter, a spreading pool with lingering floor debris, cause-specific aftermath, an impact ray burst and a short screen shake. Preset 1 draws no blood sprites. Every preset is presentation-only and floor-clamped.
- Four animated examples: **kinetic punch**, **environmentally justified decapitation**, **explosive body breakup** and **authored falling-stone crush**.

The previously weak difference between 3 and 4 was addressed in this version by scaling intensity **nonlinearly** and adding clearly distinguishable level-4 elements. This is an art reference / tuning comparison, **not a validated final gore preset**.

## Sprite provenance and fidelity

The lab embeds small self-contained RLE snapshots made directly from **the repo's original layered pixel grids**, respecting palette and original frame dimensions:

| Source | Exact authoritative asset | In the prototype |
| --- | --- | --- |
| Carl melee, 52 px tall | `assets/source/characters/carl/carl_h52_melee.grid.json` (48×56 frame) | Authentic punch and kick poses (`c1`, `c11`) |
| Goblin A, 36 px tall | `assets/source/characters/goblin/goblin_a_h36.grid.json` (64×56 frame) | Authentic idle, airborne and defeated frames (`g0`, `g19`, `g25`) |
| Catacombs/shore/block | `docs/02-art/env-02-opening-spec.md` and ENV-01/02 source art | **Simplified drawn HTML approximation, not exact source tiles or physics** |

**Only the base body poses are actual source-frame pixels.** The decapitation scene uses the `corpse` frame with its head columns masked (headless body) plus the `idle` head clip as the single flying head; the crush scene reuses the `corpse` frame under the slab and draws the aftermath. Extra particles, cutout body pieces, decapitation, blood pixels, stone, dust, scenery and timing are **concept sketches drawn in JavaScript Canvas**, not canon sprites, animation atlas changes or authoritative hitbox geometry. For the game, create **original layered pixel art assets** rather than extracting the lab's procedural cutouts.

Carl must remain the unglamorous antihero with purple/dark jacket, heart-pattern boxers and pink Crocs; GOB-01 remains a green, long-eared, pineapple-club-carrying 36px monster. Refer to their canonical specs for all production changes. Do **not** redesign the characters to match the decorative HTML background.

## Local agent workflow

1. Read `AGENTS.md`, [this README](README.md), [universal death-language proposal (draft PR #79)](https://github.com/pando85/dungeon-floorbreak/pull/79) for the tentative visual grammar and [character progression horizon](../../../docs/01-design/character-progression-horizon.md) for long-term context, the combat-feel specification, destruction contract, and GOB-01/Carl art specs.
2. Run the HTML and inspect **1/4 vs 4/4** at the same timeline position for each of the four scenarios. The selected target is **4**, but maintain a readable reduced-presentation option without altering combat rules.
3. Treat this as a **visual grammar for staging**: anticipation -> contact -> reaction / fragment emission -> physical-looking but cosmetic trajectory -> clear aftermath.
4. Implement reusable species profiles, validated damage cause, material-specific effects and detachable-part art **in the Rust+Bevy pipeline**, not by embedding Canvas code into Bevy or shipping the RLE sample as production sprites.
5. Capture **real Bevy** at 640×360 and 640×400 virtual pixels, 1× plus exact 3× scale. Compare them to this prototype and submit visual/gameplay evidence for human review.

## Related review and implementation work

- [First goblin fight / Act I draft PR #78](https://github.com/pando85/dungeon-floorbreak/pull/78).
- [Defeat-generic design / first implementation handoff draft PR #79](https://github.com/pando85/dungeon-floorbreak/pull/79): `docs/01-design/universal-monster-death-language-v0.md`, `docs/01-design/floor-01-goblin-defeat-language.md`, `docs/07-agents/i03-goblin-death-implementation-handoff.md`. **Those files are on a draft branch, not automatically on main.**
- [Generic implementation design](../../../docs/03-tech/monster-death-system-proposal.md) in this PR documents a viable Rust+Bevy split, species recipes, event contracts and validation milestones.

**Do not treat this repository reference as permission to grant Carl explosives before the narrative unlock, decapitate every weak contact or turn cosmetic fragments into gameplay-authoritative projectiles.**
