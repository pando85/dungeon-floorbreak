# WORK-007/008 follow-up — Donut Floor 1 sprite scale study

**Status:** Agent work order; requires human artistic approval.  
**Progress:** the three studies, the comparison scenes and a 40 px idle prototype exist and await human artistic review. The scale is **not** decided. See the evaluation in [`docs/02-art/donut-scale-study.md`](../02-art/donut-scale-study.md) and the evidence in [`docs/evidence/donut-floor1-scale-study/`](../evidence/donut-floor1-scale-study/README.md).  
**Branch:** `art/donut-floor1-scale-study`  
**Goal:** Choose a practical native pixel-art scale for Princess Donut before committing to a definitive sprite master or large animation set.

## Design authority

Read `AGENTS.md`, `docs/02-art/donut-character-bible.md`, `docs/02-art/donut-gear-timeline.md`, `docs/02-art/art-direction.md`, `docs/02-art/animation.md`, `docs/02-art/asset-pipeline.md`, and `docs/04-production/rights-and-release.md`.

**Single primary visual reference:**
`assets/concept/donut/approved/Princess_Donut_Evolution_Gear_Bible_v1.png`

This reference's **adorable identity** is approved; its tiny mock sprites, exact items, floor/class labels and dimensions are **not** approved as literal production specifications. The visual master is an extremely cute, round, fluffy tortoiseshell Persian: oversized golden eyes, short muzzle, tiny pink nose, rounded cheeks, compact paws, immense plume tail and a **small** purple-gem tiara. The baseline should appear lovable, curious and lightly self-important rather than permanently angry. Dramatic rage is a separate expression.

## Deliverable A: three truly pixel-authored scale candidates

Create **three original native-resolution static designs** for a consistent early-dungeon side/three-quarter-side pose at **32, 40 and 48 virtual pixels high**. Each size includes the feet-to-highest permanent crown/ear silhouette (do not artificially increase height with temporary FX). Use the same facing, stance, coat markings and accessory treatment across variants. Pixel clusters must be deliberately drawn **at each target resolution**; do not just nearest-neighbor resize the same 48px master, filter a generated illustration or copy the tiny mock sprites from the concept board.

- Keep coat patches stable (dark ginger/cream tortoiseshell), the face round, golden eyes dominant, tail full and distinct from the body, and paws recognizably feline.
- Use a modest palette with explicit transparent pixels. No antialiased edges, inconsistent subpixel clusters, palette spray, or oversized metal cuffs.
- Keep early equipment visually understated and provisional: tiny tiara, collar/charm and a restrained bracelet if visible. Do not invent or canonize new gear.
- Author and preserve **editable source** in Aseprite-compatible `.ase` / `.aseprite` with named layers and traceable palette. Save a common pixel-art style sheet/notes describing exact differences and why they matter.
- Use original pixels, not extracted images, game sprite rips, or standalone AI text-to-image outputs as final raster sources.

## Deliverable B: controlled visual comparison

Create a **single side-by-side comparison** from the genuine pixel masters:

1. All three variants at true **1× scale** in a native **640×360** scene using the existing gothic/violet/orange art-fixture context or its restrained derivative.
2. The same scene at nearest-neighbor integer **2×**; optionally provide an additional 640×400/1280×800 Steam Deck framing.
3. Separate 6× nearest-neighbor close-ups (without smoothing) for intentional human pixel review.
4. A short evaluation matrix: eyes/muzzle readability, silhouette, tail, tiara, contact/foot pivot, apparent relative height versus Carl's provisional 48–56 px, clutter/lighting visibility, and authoring complexity.

The three variants must be placed under the **same camera, lighting and environment**, with no marketing-sized character crops passed off as gameplay proof. Present a recommended option **but do not silently lock it**: human artistic review decides the winning scale.

## Deliverable C: minimal animation prototype (only after comparison)

If time permits, implement a **four-frame `idle`** for the most promising candidate, derived from the same editable source: restrained breathing, slow blink and gentle plume-tail motion. Preserve pivot/feet and coat patch continuity. Don't attempt all three complete animation sets or a full `run`/combat suite.

Reuse the existing exporter `scripts/export_assets.sh`, `scripts/validate_assets.py`, `scripts/make_contact_sheet.py`, `scripts/make_animation_previews.py`, and `.github/workflows/art.yml`. Extend the pipeline **minimally and compatibly** if needed; never create a competing CI workflow or exporter.

## Acceptance and safety gates

- Editable Aseprite-compatible sources provided and reproducibly exportable; exact source paths and manifest provenance listed.
- The native and enlarged comparisons are **actual pixel art**, not generated illustration thumbnails.
- GIF preview and tag/duration/alpha/atlas checks run through the existing toolchain if an idle animation is included.
- CI green for tests relevant to committed changes; do not claim a CI run unless it actually completed.
- No changes to Carl, Mongo, approved Donut reference art, Bevy gameplay controls, or game mechanics.
- **Rights/publication:** cleared. Publication of all fan-concept character art, including production art such as Donut pixel sources, exports, GIF previews and gameplay-scale renders, in this public repository is covered by the owner-reported `ART-PUBLICATION-ALL-V1` record in `docs/04-production/rights-and-release.md` (2026-11-15). Donut pixel sources and renders are committed publicly under `assets/source/characters/donut/`, with provenance/manifest/checksum records (generator/tool and input references; no ripped or third-party assets). The scope is the owner's user-reported representation, not verified rightsholder consent, grants no public reuse licence, and does not permit distributing a playable game, Steam build, trailer, merchandise or standalone asset pack; do not assert otherwise. Keep tooling and tests IP-neutral where practical.
- Any unavoidable deviation or blocker is documented accurately. Do not self-approve a final production sprite or merge without human review.

## Agent procedure

1. Read repo instructions, report a short plan and locate actual existing commands/CI. Check `git status` and branch identity.
2. Implement the three real pixel-size studies and compare at gameplay scale.
3. Run exporter/validator, Python unit tests and documentation checks; collect real PNG/GIF/artifact evidence. Run other relevant checks if code is changed.
4. Review the comparison for silhouette, eyes, face, tail, stable pivot, and equipment restraint; iterate until defensible.
5. Create a **draft PR** to `main` with commands/results, image evidence links, limitations and one proposed winning scale. Stop for human artistic signoff; no auto-merge.
