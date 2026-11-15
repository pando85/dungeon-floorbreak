# Princess Donut — book/floor equipment and visual-state audit

**Status:** **Research tracker; NOT verified canon.** This is the artifact-to-gameplay bridge for [Donut's approved visual identity](donut-character-bible.md). The evolution infographic is an **art reference**, not evidence for individual equipment drops or exact novel events. Spoilers for the book series.

## Evidence and acceptance rubric

Mark each item as one of:

- `needs-verification`: mentioned in conversation or shown by the image; primary book evidence has not been recorded.
- `canon-verified`: direct primary-text book/chapter/floor evidence recorded, including name, effect, equipment and whether still owned.
- `art-approved`: human approval of a specific visual treatment; can coexist with either evidentiary status.
- `noncanonical-proposal`: added only for gameplay clarity or artistic legibility.

Do **not** treat a wiki, search snippet, secondary summary or an AI-generated image as primary verification. Record **book and chapter, event floor, first acquisition, what it replaces (if anything), item slot, and a short paraphrase**—not long quotations. Avoid treating `book N` as `floor N`. A visual phase can span several floors.

## Candidate equipment audit queue

The following are candidates, **not canon assertions**. Floor groupings came from the approved artistic infographic and earlier unverified research; item names and effects need confirmation before implementation.

| Concept-board period | Candidate / motif | Visual intention | Evidence status | Book / chapter / exact floor | Replacement / coexistence |
| --- | --- | --- | --- | --- | --- |
| Floors 1–2 | Crown / tiara | Petite purple-gem silhouette anchor | `needs-verification` | Unknown | Unknown |
| Floors 1–2 | Butterfly collar charm | Tiny, delicate signature pendant | `needs-verification` | Unknown | Unknown |
| Floors 1–2 | Scale-like back covering | Suppressed shoulder/back detail; no full armor | `needs-verification` | Unknown | Unknown |
| Floor 1 illustrated | `+2 DEX Bracelet` | Narrow foreleg band | `needs-verification` | Unknown | Unknown |
| Floor 3 illustrated | `Anklet of the Fallen Oak` | Organic green/gold ankle motif | `needs-verification` | Unknown | Unknown |
| Floor 4 illustrated | `Prism Glasses` | Optional spectacle/analysis overlay | `needs-verification` | Unknown | Unknown |
| Floor 6 illustrated | `Obsidian Bracelet of the Raggle Rouser` | Dark faceted bracelet; short amber activation flare | `needs-verification` | Unknown | Unknown |
| Floor 6 illustrated | `Silver Bracelet of the Ab-solar` | Thin silver contrasting bracelet | `needs-verification` | Unknown | Unknown |
| Floor 9 illustrated | `Tiara of a Thousand Lights` | Golden/bright tiara accent, not a larger head | `needs-verification` | Unknown | Unknown |
| Floors 10–11 illustrated | Assassin-themed veil / adornment | Optional overlay, must not obscure eyes | `needs-verification` | Unknown | Unknown |

Do not silently correct candidate names or stats to make them appear canonical. If source evidence conflicts with the illustration, **revise the tracker and annotate the image discrepancy**; never distort the source.

## Narrative and class-label audit queue

The illustration includes the following **visual themes**: princess, star/performer, tactician, spellcaster, diva, summoner, champion and veteran. These are not fixed level/class unlocks.

Candidate printed class labels requiring direct verification: `Former Child Actor`, `Glass Cannon`, `Legendary Diva`, `Viper Queen`, `Master Telephone Psychic`, `Assassin of Sekhmet`, and `Elite Gurkha Warrior`. Record **whether it is a class, skill, title, achievement, spell, race/character attribute or fan interpretation** before using it as a gameplay data field.

## Visual design constraints to preserve

- **Donut's face is the main event**: large golden eyes, chubby Persian muzzle, tiny nose, tortoiseshell pattern and huge tail never change across gear stages.
- A bracelet should typically read as a **1–2 px material cue**, where legible; never build oversized permanent cuffs that change body anatomy. This is an art-direction hypothesis subject to native-resolution testing.
- Store equipment UI and visual presentation separately; identical inventory entries need not all appear in one sprite.
- Stage labels `F1–2 / F3 / F4 / F5 / F6–7 / F8 / F9 / F10–11` are artistic milestones, **not a claim that those are the first acquisition floors**.
- Auras, cards, musical notes and light effects are **controlled VFX layers**, not permanent outfit changes.
- Only commit new runtime states after native-size Aseprite + Bevy screenshot tests and verified lore decisions.

## Next evidence pass

1. Verify early book 1 accessories and when Donut first receives or equips each one.
2. Build a **book-by-book event table** with chapter and floor boundaries.
3. Track every accessory as `obtained`, `equipped`, `replaced`, `lost` or `unknown` per event, not just a picture of the end state.
4. Confirm late-book classes and gear individually, especially any illustrated objects introduced in the last stages.
5. Separate `fan-canon` story adapter data from IP-neutral `caster` equipment slots/FX in code.

See also [rights register](../04-production/rights-and-release.md) before any external distribution.
