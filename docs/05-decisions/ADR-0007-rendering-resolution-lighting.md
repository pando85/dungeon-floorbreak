# ADR-0007: 640x360 virtual pixels with a full-resolution lighting pass

- **Status:** Accepted
- **Date:** 2026-11-16
- **Deciders:** project owner
- **Evidence:** [PR #27](https://github.com/pando85/dungeon-floorbreak/pull/27), the Floor 1 resolution and lighting spike (`docs/02-art/lookdev-floor01-resolution-spike.md` on that branch; look-dev frames A0 / A1 / B1, review renders at 1080p, 4K and Steam Deck). Related: [PR #26](https://github.com/pando85/dungeon-floorbreak/pull/26) (ENV-01 catacomb kit, rejected as drawn, to be reworked).

## Context

- The primary target display is now **1080p on big screens**. 4K and Steam Deck stay supported.
- The owner doubted that a 640x360 virtual canvas can carry the Floor 1 concept's mood on a 4K display, and rejected the ENV-01 art (PR #26): thin grey pipes, muted brick, a flat far arcade and a 5 px floor strip.
- Before more environment work, a look-dev spike (PR #27) rendered one composition three ways, so the owner could judge on a real 4K monitor:
  - **A0:** 640x360 pixel layers with lighting baked into the pixels only.
  - **A1:** the same 640x360 pixel layers plus a smooth lighting pass computed at output resolution.
  - **B1:** a 960x540 redraw (not a resample) with the look-dev Carl 78 px / Donut 60 px redraws, plus the same lighting pass.
- Key measurements from the spike:
  - Colours: A0 native 640x360 has 130 unique colours (environment palette union 85; Carl 23, Donut 22). A1 has 6,181 at native size and 13,777 at 3840x2160. B1 pixel layers use an 86-colour environment union.
  - The largest per-layer palette is 26 colours.
  - Art cost: B1 has 2.25x the pixels of a 640x360 frame. Per-asset pixel area is about 2.2x (Carl 837 vs 1,815 px, Donut 1,305 vs 2,843 px). The spike's judgement (not measured) is about 1.8-2.5x hand-placed sprite cost per frame, and more for animation. Every approved master (Carl 52, Donut 40, Mongo 76) would need a full redraw, not a resample.
  - Display fit: 960x540 is exact only at 4K (4x) and 1080p (2x). It is 1.33x (non-integer) on the Steam Deck's 1280x800 and on 1280x720, and 2.67x at 1440p. 640x360 / 640x400 is an exact integer at 4K (6x), 1440p (4x), 1080p (3x) and Deck (2x, with the 640x400 view).
  - Readability: the lighting pass improved actor-to-surround contrast (Carl contrast ratio 2.01 in A0, 2.35 in A1, 2.44 in B1; Donut 1.78, 2.00, 1.94).
  - Mood: at normal viewing distance B1 looked almost the same as A1. Most of the atmosphere came from the lighting pass, not from pixel density.

## Decision

Owner decisions, 2026-11-16:

1. **Primary target display:** 1080p on big screens. 4K and Steam Deck remain supported.
2. **Virtual resolution stays 640x360** (640x400 Steam Deck variant) with integer scaling: 1080p x3, 4K x6, Deck 1280x800 x2. Look-dev option B1 (960x540, Carl 78 / Donut 60) is rejected. The approved Carl 52 / Donut 40 / Mongo 76 px heights stay.
3. **Rendering model is look-dev frame A1:**
   - pixel-art layers authored at 640x360 virtual pixels, palette-limited per atlas and upscaled by an integer factor with nearest-neighbour sampling;
   - a **screen-resolution lighting pass**: cold ambient, warm torch light pools with smooth falloff, flame-only bloom and per-layer distance haze;
   - **parallax background layers**.

   Lighting, haze and bloom must not change gameplay outcomes. The exact Bevy 0.20 implementation is **not** decided. The mapping in the spike write-up is a proposal that COMP-01 must verify against Bevy documentation and a running prototype.
4. **Floor 1 colour direction:** warm orange torch-lit brick falling off into cold blue shadow, thicker copper/brass pipework and a deep layered gallery visible through arches. This supersedes the "muted red-brown brick" wording in the Floor 1 Environment locks; the lock text is updated with a dated note. All other locks stay (dry, clean, flat floor; no sewer, toxic, factory or TV look; industrial detail sparing and local).
5. **Deep floor:** production terrain uses a deep visible floor top surface (about 24 px at 640x360) with the walking line inside it, feet about 4 px behind the front edge. This replaces ENV-01's thin 5 px strip. Foot contact stays at 0 px sink relative to the walking line.
6. **Sequencing:** ENV-01 (PR #26) art is reworked against A1 as the approved target frame. COMP-01 (the engine compositor and lighting work) follows ENV-01.

The owner judged the A1 ambient mood "great".

## Alternatives considered

- **A0, baked lighting only (640x360):** cheapest and fully palette-limited, with an exact integer scale everywhere. Rejected: light falloff steps per block and reads as tiled bands, there is no haze, and the gallery reads flatter. A1 costs the same art effort and gets much closer to the concept.
- **B1, 960x540 redraw plus lighting:** finer bevels and richer faces, less chunky at 4K. Rejected: about 2.2x art cost per asset and more for animation, a full redraw of every approved master, and a non-integer scale on Steam Deck and 1280x720. The spike showed almost no mood gain over A1 at normal viewing distance.
- **1280x720 virtual resolution:** rejected as non-integer at 1080p (1.5x), which breaks the integer-scaling rule.

## Consequences

- **Engine work in COMP-01:** a light-accumulation and composite pass at output resolution, per-depth-group ambient, flame-only bloom, per-layer haze and parallax. All of it is render-only and must not feed gameplay state.
- **Contract changes for environment data:**
  - the terrain surface is a deep floor (about 24 px top surface) with a walking line inside it, not a 5 px strip;
  - parallax layers (at least a far and a mid gallery) are separate layers with a parallax factor;
  - light markers (torches, flames, gallery lights) are part of level or prop data. They are presentation data and must not affect collision, AI or damage.
- **Palette limits** apply to the pixel layers (per atlas), not to the lit final frame. A1 output has thousands of colours, so a palette validator can only check layers.
- **Baked art and the light pass must be tuned together.** Pools can over-brighten already hot baked pixels.
- **Art cost stays at the 640x360 level.** No redraw of approved characters. The h78 / h60 cast redraws are look-dev only and rejected.
- ENV-01 art is redone, so its current kit is not the target. The Floor 1 lock wording changes as described above.
- Rendering and lighting stay outside authoritative simulation, consistent with AGENTS.md. Screenshot and replay evidence must be captured with the lighting pass on.

## Reversible and irreversible aspects

- **Reversible:** the lighting parameters (pool strength, ambient tint, haze, bloom), the Bevy implementation approach in COMP-01, and the light-marker format. All of them can change without redrawing art.
- **Hard to reverse:** the 640x360 / 640x400 virtual resolution. Once layers, character masters, animations and tile kits are produced at this density, moving to 960x540 means redrawing everything (about 2.2x per asset) and gives up the integer scale on Steam Deck. Revisit it only with renewed owner approval and new evidence.
- **Reversible with cost:** the deep-floor contract. Changing it later affects terrain art, collision surface offsets and level data.

## Implementation note (COMP-02)

COMP-02 verified the "Proposed Bevy 0.20 mapping" against Bevy 0.20.1 and a running prototype. The decision above stands unchanged. What the implementation confirmed or changed (details in [floor01-scene.md](../03-tech/floor01-scene.md#lighting-comp-02)):

- **Confirmed:** pixel layers stay at virtual resolution and are lit at output resolution; all lighting is presentation data derived from the kit contract and never read by the simulation (state hashes are identical with lighting on and off).
- **Changed, light accumulation:** instead of additive light sprites into a full-resolution accumulation texture, each light is evaluated analytically per output pixel in one `Material2d` fragment shader (about 30 lights, uniform array). This avoids a full-resolution render target and sprite overdraw and reproduces the reference falloff curves exactly. Per-depth-group ambient and haze use one virtual-resolution target per contiguous light-group run (6 slots for the sample layout), each lit as a unit.
- **Changed, bloom:** the engine HDR bloom was not used. A small custom pass (flame-colour mask at virtual resolution, separable gaussian, additive composite) feeds bloom only from the contract's emissive colours, with nearer opaque pixels occluding it.
- **Exact integer scaling kept:** the composite pass reads the virtual-resolution targets with integer `textureLoad`, so there is no filtering and no seams at any window size.
