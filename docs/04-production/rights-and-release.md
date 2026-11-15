# IP, licensing, source provenance and public release policy

**Project status:** fan/community game concept in a public repository. **The owner reports permission to publish approved concept images to this public GitHub repository (confirmed 2026-11-15); the permission has not been independently verified here.** No blanket adaptation, playable-game distribution or commercial rights are asserted. **Not legal advice.** Consult rights counsel and the written grant for any external distribution.

## Third-party intellectual property

The Dungeon Crawler Carl books, franchise names, characters and recognizable expressive elements are third-party IP. The owner has explicitly confirmed **permission to publish character concept images in this public GitHub community repository**; treat that as **limited, user-reported scope**, not proof of a general franchise license. Publication here lets visitors view and download repository files, but does not grant the public a reuse license. A publicly playable game, Steam version, trailer, merchandise, or independently distributed character asset pack may require **separate** express permission (or fully original IP).

## Public repository safety rules

- Do not upload pages or substantial protected quotes from books, recorded audiobooks, unapproved third-party fan art, extracted sprites/audio or proprietary game screenshots. **Approved, independently created fan/community concept illustrations may be uploaded** only within the owner-confirmed permission scope and with an asset-level provenance record; other recognizable character art requires its own permissions check.
- Use generic mechanic IDs and placeholder art in source code: `demolitionist`, `caster`, `charger`.
- Keep franchise-specific character/lore mapping confined to the documented fan-content adapter and approved concept references; do not ship a public playable build or sublicense assets on this basis alone.
- Do not claim affiliation, endorsement, official status, public-domain status or an existing franchise license.
- Explicitly track whether generated files may contain third-party source/training references, and review for near-copying. Asset provenance must record model/tool and known input references.

## Approved concept artwork — user-reported permission register

| Record | Scope confirmed by owner | Exclusions / outstanding evidence |
| --- | --- | --- |
| `CARL-CONCEPT-APPROVED-V1` (2026-11-15) | Publish the approved Carl concept image to `assets/concept/carl/approved/` in this public GitHub community repo | Not independently verified against grant text; no game distribution, commercial use, independent asset pack or sublicensing implied. See [asset provenance](../../assets/concept/carl/approved/provenance.md) and [Carl bible](../02-art/carl-character-bible.md). |
| `DONUT-CONCEPT-EVOLUTION-V1` (2026-11-15) | Owner approved the Donut visual identity and asked to add its generated evolution board to this public repo; **the exact PNG has not yet been uploaded** | Publication scope is user-reported and not independently verified; item names/floor timing in the picture are not primary-source verified. See [Donut provenance](../../assets/concept/donut/approved/provenance.md) and [Donut bible](../02-art/donut-character-bible.md). |

**Keep permission claims narrowly stated.** A verbal/chat confirmation is a record of the owner's representation, not proof of rightsholder consent. Do not publicly upload any other franchise-derived asset without comparable scope confirmation and provenance.

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
