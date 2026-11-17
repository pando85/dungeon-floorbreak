# Floor 01 — Late enemy and boss concept approvals

**Status:** owner-approved **visual concept direction** as of 2026-11-17. This supplement extends the [previously approved Floor 01 enemy roster](floor-01-enemy-concept-roster.md) and does **not** amend its existing approvals. Production sprites, exact pixel scale, balance, collision, animation timing, in-engine appearance and gameplay remain separate gates for the local art/integration agents.

**Creative premise:** Keep the established 1980s–1990s gritty, darkly comic action tone, expressive readable silhouettes, cold brick-catacomb surroundings and warm local light. Avoid turning the entire floor into an arena of purple neon, a modern sewer or a generic fantasy environment.

## This batch

| Subject | Owner approval and visual features | Additional decisions still needed |
| --- | --- | --- |
| **The Hoarder — dedicated study** | The Hoarder's overall visual identity was already approved in the first roster, and the follow-on three-variant study is retained as an approved *reference/exploration sheet*. Massive transformed **human woman** with long dark disheveled hair, severe facial damage, everyday torn clothing and a cluttered refuge. Fear, loss of humanity and horror should be evident alongside grotesque action-comedy. | **No single A/B/C Hoarder variant has been specifically selected as the final production master.** Do not present the composite as a finished sprite or turn her into a goblin. Remove brand-like/real-world shirt logos from original production pixel art, without altering the original reference file. |
| **Troglodyte Pygmy** | Owner-approved family sheet: small, aggressive, fast-looking silhouette with readable leap preparation. | Final visual native-scale and attack timings pending. |
| **Troglodyte Basher** | Owner-approved family sheet: bulkier melee-oriented member of the family; heavy wind-up and impact poses. | Separate physical class stats, damage and hitboxes not approved here. |
| **Troglodyte Virtuoso** | Owner-approved family sheet: leaner, unnerving silhouette and prominent tongue-extension attack cue. | Tongue reach and poison mechanics require gameplay design, not inference from the concept sheet. |
| **The Juicer** | **Owner-approved** exaggerated-muscle grotesque family boss, maintaining the same troglodyte species cues. The concept covers lunge/pound/roar and facial poses, **not** engine-ready moves. | Book-vs-concept size difference must be documented during production: the first novel describes the Juicer as only a little taller than Carl rather than several times his size. The approved concept exaggerates its scale for presentation. Preserve the approved reference; independently decide a gameplay-readable scale. |
| **The Ball of Swine** | **Owner-approved** grotesque rolling spherical composite of multiple Tusklings: dense pink flesh, multiple eyes, snouts, tusks, hair, fragments of aristocratic dress and violent rotational silhouette. The standalone sheet includes impact/disassembly and individual Tusklings in contrasting aristocratic dress. | Exact sphere size, speed, separate Tuskling hitboxes, damage, rotation, arena-driven state changes and render scale are **not** approved engineering measurements. |
| **Tusklings (constituents)** | The Ball of Swine sheet approves the visual family: tusked pig-like short-legged aristocratic creatures distinguishable both within the sphere and separately. Knight/lady attire is visual direction; one later named Dominatrix is described in Chapter 27. | Individual combat archetypes and separated post-breakout abilities not locked by captions. |

### Source accuracy and creative adaptation

These notes were checked against the owner-supplied **Book 1 EPUB**, specifically the Floor 1 material: The Hoarder (Chapter 11), troglodytes and gym encounter (Chapters 23–24), and Ball of Swine/Tusklings (Chapters 26–27). No book text, page scans, or third-party imagery is committed.

- **Hoarder:** the source creature is a transformed human woman, not the goblin-with-backpack occasionally shown in earlier generated ensemble images. Retain her individuality and tragedy.
- **Troglodytes:** the source describes reptilian upper bodies reminiscent of Komodo dragons and long bent scaly legs. The approved generated sheet takes stronger humanoid/horror caricature liberties; **do not silently reinterpret the approved sheet as literal book-accurate anatomy.**
- **The Juicer:** the source describes an immensely muscular troglodyte and a gym setting with exercise equipment, reception, and mirrors; the book's relative height does not match the huge proportions on the exploratory concept sheet. Record that scale mismatch for the original pixel-art study.
- **The Ball:** the source describes a violently rolling formation of **30+ Tusklings**, with aristocratic costumes mixed into the mass. The related concentric/gated arena with changeable walls is a *separate environment design*, not proof that all stage lighting or dimensions from the sheet are canonical.
- Names, captions, production-class labels, exact sizes, invented attacks and gameplay mechanics printed in AI-generated image panels are **not self-authenticating canon or balance decisions**. Do not repaint the owner's approved source images to hide their exploratory nature.

## Required image references (owner upload to this PR branch)

Upload these **three original, unchanged PNGs** to `assets/concept/floor-01/enemies/`:

| File | Subject | Notes |
| --- | --- | --- |
| `floor01-hoarder-individual-study.png` | The Hoarder focused A/B/C designs | Reference approval; no final variant selected |
| `floor01-troglodyte-family-approved.png` | Pygmy, Basher, Virtuoso, Juicer | Entire visual concept family approved |
| `floor01-ball-of-swine-approved.png` | Rolling Ball + Tuskling variants | Visual concept approved |

The expected SHA-256 hashes, original conversation filenames, image dimensions and byte sizes are pinned in the **existing** `assets/concept/floor-01/enemies/manifest.json`, alongside all five files from the previous PR #50. The three new files are **absent** until the owner uploads them. No existing image or approval is replaced. The art-assets manifest validator must pass **after** upload and before merge.

**Provenance and publication:** All three are original OpenAI-generated concept images from the project's ChatGPT conversation. Source inspiration is the owner's previously supplied Book 1 EPUB and the project's approved A1 visual direction; no book excerpts, ripped sprites or externally sourced fan illustrations are included. Publication follows the owner-reported `ART-PUBLICATION-ALL-V1` record in the [rights policy](../04-production/rights-and-release.md), which is **not** a general licence to ship a playable adaptation or standalone asset pack.

## What remains after this batch

The **principal first-floor enemy/boss concepts** are covered. The remaining identified enemies/contraptions that merit **new** visual studies, based on the same first-book Floor 1 chapters, are:

1. **Level-2 dungeon rats:** repeated nuisance encounters in several corridors, Chapter 8 onward. Treat as a separate small enemy concept, not Mordecai's `Rat Hooligan` (an NPC).
2. **Goblin Bomb Bard (level 5):** a distinct later goblin type from Chapter 15; a new character silhouette, not permission to retrofit unexplained bombs into the initial goblin trio.
3. **Goblin Copper Chopper with sidecar:** small human-sized steam-powered goblin vehicle from Chapter 16; **not** the opening Murder Dozer.
4. **Goblin chieftain:** an off-screen/inferred neighborhood boss in Chapters 16–18; optional design only if the adaptation elects to show the character. Do not invent a canonical look.

Also still unfinished as **individual environment concept studies**, separate from enemy art: (a) the Juicer gym (reception, exercise machinery and mirrors), and (b) the Ball of Swine arena (readable circular/spiral channels, moving ring walls, floor exit staging). The atlas currently has only provisional overview crops for these two spaces.

**Scope boundary:** This PR is concept-direction documentation and pinned reference images only. Local agents handle native `.ase` source, animation, test renders, CI, and Bevy. No additional gameplay code, stage topology or detailed balance is authorized here.

## Final merge gates

- [x] All existing enemy approvals preserved
- [x] Hoarder study, troglodyte family, The Ball of Swine and Tusklings recorded with correct approval boundaries
- [x] Original image hashes and metadata computed from the three local originals
- [ ] Three unchanged source PNGs committed by owner to this PR branch
- [ ] All eight concept PNGs pass the automated hash/dimension/byte validator
- [ ] Required CI checks green on the final PR head
