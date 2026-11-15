# Visual target VT-001 — Inspection Corridor

**Status:** **overall artistic direction APPROVED** by the project owner on 2026-11-15; **character rendering, native-resolution pixel art, and production assets NOT approved**.
**Workstream:** WORK-007 (visual direction), feeding WORK-008 (Aseprite sources),
WORK-011 (LDtk room) and the Inspection Corridor vertical slice.
**Date:** 2026-11-15.

## Human creative decision (2026-11-15)

The owner explicitly liked the **overall visual impression** of VT-001: its dark, detailed pixel-art dungeon, vivid cold-violet versus hot-amber lighting, theatrical danger, grimy stage machinery, readable explosive spectacle, and irreverent everyman action tone. Preserve this direction as our first **approved mood/composition target**.

The owner also noted that **the illustrated Carl is not exactly the previously approved Carl**. This is acceptable **only** for evaluating a room's atmosphere and shot composition. **Do not use this rendition as a Carl reference, new canonical sprite, body/face correction, or permission to redesign him.** All future Carl-specific work must follow `carl-character-bible.md` and its privately held canonical design. When assets are licensed/authorized, replace the illustrative figure with a faithful portrayal of the previously approved Carl. Until then, use an IP-neutral actor in public assets.

This approval does **not** signify acceptance of sprite fidelity at 1x, runtime readability, Bevy rendering, level geometry, camera behavior, animations, VFX performance or legal permissions. These remain separate gates.

## Subsequent reference correction (2026-11-15)

The owner clarified that the **two project-generated images in the Art Slice v2
conversation** (gothic-industrial magenta arena matte and cinematic action scene)
are the agreed **visual quality/mood references**. Generic web-search imagery
shown later did **not** match the target; do not use or cite it as a substitute.
See [art-direction.md](art-direction.md#approved-visual-references--fixed-no-substitutes)
for the source filenames and approved artistic traits.

**Scope of approval:** room atmosphere, lighting, scenery, visual density,
explosions and overall tone **only**. The depicted characters are not approved
or canonical. **Carl's shotgun is explicitly forbidden**; treat the gun as an
error in a useful environment reference, not as a proposed ability or loadout.
Refer to the previously approved Carl master and the current character sheet
for any future portrait, sprite or equipment work.

## Purpose

Establish one understandable *visual-quality target* for a side-scrolling dark
fantasy action room before scaling the asset backlog. This is not a playable
build, a production sprite sheet, an animation test or proof that Bevy rendering
matches the concept.

The image reference was generated in the owner's private project conversation
on 2026-11-15 (candidate name: `VT-001 Inspection Corridor`). **It is not
included in the public repository**: the fan-character reference and its
distribution rights must pass the explicit gate in
[`carl-character-bible.md`](carl-character-bible.md) and
[`rights-and-release.md`](../04-production/rights-and-release.md).
Do not infer a right to publish from generation alone.

## Creative intent — already agreed

- An **ordinary, disheveled early survivor**, with the approved Carl silhouette:
  brown short messy hair, normal physical proportions, irritated / overwhelmed
  rather than heroic; no redesign of the approved face or four-phase evolution.
- A **dark, detailed, expressive pixel-art** direction; cold violet-blue stone,
  rusty utilitarian materials, vivid amber explosions and controlled magenta
  accents. These are compositional targets, not locked RGB values.
- **80s/90s everyman antihero action:** terrifying spectacle with physical
  effort, a slightly ridiculous predicament and black comedy; not heroic power
  fantasy, clean superhero armor or nostalgic VHS blur.
- A distinct gameplay foreground, readable enemies and bomb hazards, and
  background crowd / theatrical machinery that remains low in gameplay contrast.
- Environmental destruction **changes a route**. The damaged barrier should
  show what was solid, what is now traversable and what remains indestructible.
- Two understandable choices: a *lower breach* and an *upper traversal route*.
  The final geometry must be tested in LDtk/Bevy, not inferred from illustration.
- Character and objects must remain readable at typical handheld viewing size
  even when an explosion occupies the center of the scene.

## What the concept image does and does not decide

| Review topic | Present judgment | Not yet established |
| --- | --- | --- |
| Dungeon atmosphere, tonal contrast, stage spectacle | **Approved mood direction**, final art pending | Final palette IDs, lighting shader, perf |
| Antihero attitude and absurd danger | **Approved tone**, character rendering rejected as canonical | Sprite likeness or canon-accurate gear |
| Explosion color hierarchy and destructible materials | **Approved visual intent**, effect behavior pending | Damage model, falloff, collision rebuild |
| Room depth and foreground/background separation | **Approved composition intent**, runtime scene pending | True parallax and camera readability |
| Upper and lower route composition | **Approved route concept**, runtime geometry pending | Traversal distances and connectivity |
| Detailed character illustration | **Not** approved gameplay sprite | 48–56 px authentic Aseprite design, pivots, anims |
| Dramatic snapshot | **Not** gameplay proof | Hitstop, enemy AI, input latency, 60 fps |

Any small generated details (signs, equipment, skull iconography, enemy outfits,
facial expressions, joke props) are **illustrative, not canonical**. Verify
story-specific footwear, heart-print boxers and gear against the relevant
book/floor/chapter before claiming canon.

## Translation to an implementation-safe art package

1. Build an **IP-neutral** 640×360, 16:9 composition test (also 640×400 for
   16:10) with 16×16 authored foreground tiles; preserve 2 independent routes.
2. Use an early-survivor character *placeholder* only until rights clearance.
   Do not regenerate the approved Carl character master; use the privately
   approved reference for art review when permitted.
3. Author a small editable **Aseprite source set**, not a flattened concept
   crop: terrain stone/wood/glass/reinforced, one barrel, one enemy,
   character placeholder and impact FX, each with metadata and provenance.
4. Export nearest-neighbor atlases and inspect at **1×**, 1280×800 (2× 640×400)
   and 1920×1080 (3× 640×360). Confirm legible hurt, anticipation and explosion
   frames without relying on blur or a high-resolution promotional illustration.
5. Implement the same room in LDtk/Bevy only when the technical work orders
   support it. Record screenshot and replay evidence and compare against the
   *mood and readability* of this target, not pixel-perfect similarity.

## Human review questions — choose no more than three edits per pass

1. Does the scene feel like **our** everyman 80s/90s action antihero in a cruel
   dungeon show, rather than a generic fantasy platformer?
2. Can you immediately distinguish the playable route, dangerous enemies,
   breakable barrier and upper alternative while a large blast is visible?
3. Which **one** visual feature should be made stronger, and which **one**
   should be toned down? (e.g. explosion intensity, room texture, crowd/show
   layer, color contrast, protagonist scale).

### Review status and evidence

- [x] Static AI concept image created for conversation-based review.
- [x] Approved Carl character bible consulted; no new face/body redesign requested.
- [x] Human approval of **overall atmosphere and broad shot composition** (2026-11-15); this is not a line-by-line scene or character approval.
- [x] Noncanonical Carl depiction explicitly identified; **approved Carl reference remains unchanged**.
- [ ] Human approval of gameplay-scale pixel-art sample.
- [ ] Editable Aseprite sources and reproducible atlas export.
- [ ] Gameplay screenshot/video from Bevy.
- [ ] 1280×800 / 1920×1080 readability checks.
- [ ] Franchise rights / original-IP packaging review before publishing art.

**Decision:** VT-001 is **approved as a visual mood/composition target only**.
WORK-007, WORK-008 and the vertical slice remain incomplete until native-resolution
source assets, visual tests, human gameplay-scale approval and applicable rights
gates are satisfied.
