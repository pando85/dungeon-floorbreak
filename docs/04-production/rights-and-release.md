# IP, licensing, source provenance and public release policy

**Project status:** unlicensed fan-concept planning, public documentation repository. **Not legal advice.** Consult relevant rights counsel before external distribution of a franchise-derived game.

## Third-party intellectual property

The Dungeon Crawler Carl books, franchise names, characters and recognizable expressive elements are third-party IP. This repository can discuss gameplay concepts and engineering choices, but it grants **no adaptation, distribution, merchandising or commercial rights**. Releasing a publicly downloadable game, paid Steam version, trailer or character asset based on that IP requires an appropriate rightsholder permission/license, or a conversion to fully original characters and storytelling.

## Public repository safety rules

- Do not upload pages or quotes from books, recorded audiobooks, fan art without permissions, extracted sprites/audio, proprietary game screenshots as downloadable assets, or recognizable third-party character illustrations.
- Use generic mechanic IDs and placeholder art in source code: `demolitionist`, `caster`, `charger`.
- Keep character/lore mapping confined to fan-concept documentation until clearance.
- Do not claim affiliation, endorsement, official status, public-domain status or an existing franchise license.
- Explicitly track whether generated files may contain third-party source/training references, and review for near-copying. Asset provenance must record model/tool and known input references.

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
