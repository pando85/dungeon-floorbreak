# IP, licensing, source provenance and public release policy

**Project status:** fan/community game concept in a public repository. **The owner reports permission to publish all project art of the fan-concept characters (concept images and production art) in this public GitHub repository (confirmed 2026-11-15, record `ART-PUBLICATION-ALL-V1`); the permission has not been independently verified here.** No blanket adaptation, playable-game distribution or commercial rights are asserted. **Not legal advice.** Consult rights counsel and the written grant for any external distribution.

## Third-party intellectual property

The Dungeon Crawler Carl books, franchise names, characters and recognizable expressive elements are third-party IP. The owner has explicitly confirmed **permission to publish all project art of the fan-concept characters (Carl, Princess Donut, Mongo and other franchise-derived characters) in this public GitHub community repository**, covering both concept images and production art (see `ART-PUBLICATION-ALL-V1` below); treat that as a **user-reported scope**, not proof of a general franchise license. Publication here lets visitors view and download repository files, but does not grant the public a reuse license. A publicly playable or distributed game, Steam build, trailer, merchandise, or standalone asset-pack distribution still needs **separate** express permission (or fully original IP).

## Public repository safety rules

- Do not upload pages or substantial protected quotes from books, recorded audiobooks, unapproved third-party fan art, extracted sprites/audio or proprietary game screenshots. **Independently created fan-concept character art (concept images and production art such as pixel sources, spritesheets, animation data, GIF previews, gameplay-scale renders/screenshots and CI art artifacts) may be uploaded** within the owner-confirmed `ART-PUBLICATION-ALL-V1` scope and with an asset-level provenance/manifest/checksum record; art outside that scope (other franchises, third-party or unapproved fan art) requires its own permissions check.
- Use generic mechanic IDs and placeholder art in source code: `demolitionist`, `caster`, `charger`.
- Keep franchise-specific character/lore mapping confined to the documented fan-content adapter and the fan-concept art; engine and data contracts stay IP-neutral. Do not ship a public playable build, distribute a standalone asset pack or sublicense assets on this basis alone.
- Do not claim affiliation, endorsement, official status, public-domain status or an existing franchise license.
- Explicitly track whether generated files may contain third-party source/training references, and review for near-copying. Asset provenance must record model/tool and known input references.

## Fan-concept art — user-reported permission register

| Record | Scope confirmed by owner | Exclusions / outstanding evidence |
| --- | --- | --- |
| `ART-PUBLICATION-ALL-V1` (2026-11-15) | Publication in this public GitHub repository of **all project art of the fan-concept characters** (Carl, Princess Donut, Mongo and other franchise-derived characters): concept images **and** production art, i.e. editable pixel sources (`.ase`/`.aseprite`, pixel-grid JSON), exported spritesheets/atlases, animation JSON, GIF previews, gameplay-scale renders/screenshots and CI art artifacts. Supersedes the concept-images-only wording of the records below; Donut pixel sources live under `assets/source/characters/donut/`. | Owner's user-reported representation, **not** independently verified rightsholder consent and no written grant on file. **Not** a reuse licence for the public (no LICENSE file; standard copyright applies). **Not** permission for a publicly playable or distributed game, Steam build, trailer, merchandise or standalone asset-pack distribution, each of which needs separate express permission. Provenance/manifest/checksum rules apply to every asset (no ripped sprites, no third-party assets, no copied book text; record generator/tool and input references). Engine/data contracts stay IP-neutral; Carl/Donut/Mongo remain a replaceable fan-concept layer. If the actual grant is narrower, follow the grant and amend this record. |
| `CARL-CONCEPT-APPROVED-V1` (2026-11-15) | Publish the approved Carl concept image to `assets/concept/carl/approved/` in this public GitHub community repo | Not independently verified against grant text; no game distribution, commercial use, independent asset pack or sublicensing implied. See [asset provenance](../../assets/concept/carl/approved/provenance.md) and [Carl bible](../02-art/carl-character-bible.md). |
| `MONGO-CONCEPT-V1` (2026-11-15) | Owner selected and approved three project-generated Mongo character concept boards for public fan-community reference, with exact original binary images committed and verified | User-reported scope only, not verified rightsholder grant or game-distribution permission; see [provenance](../../assets/concept/mongo/approved/provenance.md) and [Mongo bible](../02-art/mongo-character-bible.md). |
| `DONUT-CONCEPT-EVOLUTION-V1` (2026-11-15) | Owner approved the Donut visual identity and asked to add its generated evolution board to this public repo; the exact PNG has been committed and source-verified | Publication scope is user-reported and not independently verified; item names/floor timing in the picture are not primary-source verified. See [Donut provenance](../../assets/concept/donut/approved/provenance.md) and [Donut bible](../02-art/donut-character-bible.md). |

**Keep permission claims narrowly stated.** A verbal/chat confirmation is a record of the owner's representation, not proof of rightsholder consent. The three per-asset records below (Carl, Mongo, Donut) keep their original wording; `ART-PUBLICATION-ALL-V1` widens the publication scope to production art. Do not publicly upload art of franchises or characters outside that scope, or any asset without provenance, without comparable scope confirmation.

## Technical IP-neutral layer

```
Core systems:  physics / damage / terrain / inventory / input / save
Data contracts: class IDs, material IDs, narrative event IDs
Fan-concept adapter: Carl / Donut / Mongo *names in planning only*
Release adapter: original characters or licensed character pack
```

No game systems should match on a hardcoded copyrighted character name. Names, story, voice/text references, icons and images must be replaceable. This is practical architecture protection, not a legal loophole for distributing unlicensed adaptations.

## Code and document licensing

There is **no LICENSE file** intentionally. Public GitHub visibility does not mean contributions are open source: standard copyright applies. Before collaboration choose an explicit license for original code (e.g., a permissive license if desired), an appropriate license for docs, and separate content/art licensing policy. Attribution and asset manifest are mandatory.

## Steam and AI-generated content

Before submission, read Steamworks' current AI-generated content requirements and be accurate about whether artwork, audio, text, or live generation appears in the product. 'AI wrote the Rust code' is different from 'the game contains generated art'; don't assume either automatically grants or removes rights. Also review store brand/trademark rules, music/fonts, controller compatibility, customer support and VAT/business obligations.

## Release gates

- [ ] Decide on official rights license vs completely original IP.
- [ ] Document distribution, artwork, music, font and character rights.
- [ ] Review output from all generative tools for copyright/trademark conflicts.
- [ ] Choose code/doc/asset licenses intentionally.
- [ ] Perform Steamworks platform and generated-content policy review at release time.
- [ ] Ensure no public repo secrets, user data or unlicensed assets.

No build should be published while any legally necessary clearance is unresolved.
