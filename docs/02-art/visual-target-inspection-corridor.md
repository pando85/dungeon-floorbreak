# Visual target VT-001 — Inspection Corridor

**Status:** candidate for human artistic review, **not approved**.
**Workstream:** WORK-007 (visual direction), feeding WORK-008 (Aseprite sources),
WORK-011 (LDtk room) and the Inspection Corridor vertical slice.
**Date:** 2026-11-15.

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
| Dungeon atmosphere, tonal contrast, stage spectacle | Candidate aesthetic | Final palette IDs, lighting shader, perf |
| Antihero attitude and absurd danger | Candidate narrative staging | Sprite likeness or canon-accurate gear |
| Explosion color hierarchy and destructible materials | Candidate effect language | Damage model, falloff, collision rebuild |
| Room depth and foreground/background separation | Candidate art layout | True parallax and camera readability |
| Upper and lower route composition | Candidate level intention | Traversal distances and connectivity |
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
- [ ] Human approval of atmosphere and shot composition.
- [ ] Human approval of gameplay-scale pixel-art sample.
- [ ] Editable Aseprite sources and reproducible atlas export.
- [ ] Gameplay screenshot/video from Bevy.
- [ ] 1280×800 / 1920×1080 readability checks.
- [ ] Franchise rights / original-IP packaging review before publishing art.

**Decision:** pending. Do not relabel VT-001, WORK-007, WORK-008 or the vertical
slice as complete based on this illustration alone.
