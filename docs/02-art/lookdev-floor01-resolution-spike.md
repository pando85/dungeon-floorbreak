# Floor 1 look-dev: resolution and lighting spike (A0 / A1 / B1)

**Status:** look-dev complete — owner chose A1 (640×360 + full-resolution lighting) and a deep floor on 2026-11-16; B1 and the h78/h60 cast redraws are rejected look-dev only; see ADR-0007. This is **not production art**, not a tile kit and not runtime data. Nothing here changes an approved character master, a manifest, Rust code or CI.
**Authored by:** Claude Opus (AI art agent), 2026-11-16. Review pending.
**Mood reference:** [`01-catacomb-environment-clean.png`](../../assets/concept/floor-01/opening/01-catacomb-environment-clean.png). It was used **for mood, lighting, colour temperature and volume only**. It was never cropped, traced, sampled or read by any script.
**Evidence:** [`docs/evidence/lookdev-floor01-resolution-spike/`](../evidence/lookdev-floor01-resolution-spike/README.md).

## Purpose

The owner rejected the ENV-01 art (PR #26) and asked for a spike before any more environment work. The spike answers two questions:

1. Can a 640×360 virtual canvas carry the concept's mood on a 4K display? It is tested with baked lighting only (A0) and with a smooth lighting pass at full resolution (A1).
2. What does a 960×540 redraw (B1) gain on a 4K display, and what does it cost?

All three frames show **one composition**: plain warm brick, a thick sculpted arch with a deep layered gallery behind it, and a short copper pipe run with a valve. Carl and Donut stand on a visible floor plane at the ENV-01 `actor_check` positions (Carl pivot x = 64 and Donut pivot x = 126 in 640×360 units, contact line y = 288).

## Owner feedback this spike answers (PR #26 review, 2026-11-16)

| Feedback | What the spike does |
| --- | --- |
| Liked: character readability, Carl/Donut positions and foot contact | Same x positions and contact line as ENV-01. Feet stand 4 px behind the front lip of a 24 px deep floor plane. Contact shadows are baked into the floor. |
| Rejected: thin grey iron pipes | Copper pipe 14 px thick (21 px at B), with brass flanges, a brass handwheel, a bend and iron wall clamps. It is cylinder-shaded and has a specular streak from the torch beside it. It appears only in the C section on the right. |
| Rejected: muted, desaturated brick | Hue-shifted 12-step brick ramp. Brick is warm orange inside the torch pools and falls through dusky red to cold blue-violet at the top of the wall and between pools. |
| Rejected: flat blue far arcade | Two parallax gallery layers. The **mid** layer has dark pillars, a balcony slab with a railing and arches under it. The **far** layer has a lower arcade with four distant torches, a cornice and tall upper windows with the lightest cold haze. Ground fog sits in both. |
| Doubt: is 640×360 enough at 4K? | Native frames, 4K/1080p/Deck renders and a same-region 1:1 crop at 4K are provided for comparison on a 4K monitor. |

**Contradiction flagged, not resolved here:** the Environment locks in [`floor-01-opening-catacomb-goblins.md`](floor-01-opening-catacomb-goblins.md) say "muted red-brown brick … restrained orange torch highlights". The owner's 2026-11-16 review asks for warm orange torch-lit brick. The spike follows the newer owner feedback but keeps the brick muted outside the pools. The owner should update the lock wording in the ADR if this direction is accepted. The other locks still hold: dry, clean, flat floor; no sewer, toxic, factory or TV look; and industrial detail (one pipe and valve) is sparing and local.

## The three frames

| Frame | Pixel density | Lighting | Native file |
| --- | --- | --- | --- |
| **A0** | 640×360 (and a 640×400 view for 16:10) | Baked into the pixels only, palette-limited | `native/a0_640x360.png` |
| **A1** | the same 640×360 pixel layers | + lighting pass computed at **output** resolution | `native/a1_640x360.png` |
| **B1** | 960×540 **redraw** (each pixel evaluated at 1.5× density with density-specific detailing; nothing resampled) | the same lighting pass | `native/b1_960x540.png` |

**B1 uses the real look-dev cast redraws:** Carl 78 px and Donut 60 px, frame 0, from branch `art/lookdev-cast-h78-h60` (`assets/source/lookdev/characters/`). This branch is stacked on that one. The redraws are look-dev only and the approved 52/40 px masters are unchanged. They have a single frame, so B-density animation cost is estimated below, not measured. `lighting.json` still contains a `b960_placeholder` actor set (approved sprites nearest-upscaled 1.5×). It is not rendered by default and exists only for debugging.

### How the pixels are made

`scripts/lookdev_floor01_author.py` is a deterministic "pixel shader". It writes one `pixel-grid-v1` source per layer and density to `assets/source/lookdev/floor01/` and builds a matching `.ase` with `scripts/pixel_grid_to_ase.py`. Every surface pixel has a design-space position, a depth and a normal. The normals come from 1 px bevels at A and 2-step bevels at B, plus the arch soffit, the pillar jambs, the pipe cylinder and the floor plane. Four torches and a cold bounce from the gallery are evaluated with a windowed inverse-square falloff, and the result is quantised onto hand-picked hue-shifted ramps. Flat faces are shaded **per block**, so the art keeps clean clusters instead of per-pixel noise. Designed details:

- per-brick tone variation;
- pits made of 2-pixel clusters;
- chipped corners;
- soot above the flames;
- lit top edges on floor slabs;
- cold rims on surfaces that face the gallery;
- a keystone;
- a soffit with courses receding into the wall;
- a slight top-down floor plane whose slab joints converge;
- a lit lip on the front ledge.

At B the extra density is spent on wider bevels, a gentle light gradient inside each brick near the torches, larger pits with lit lips, 3 px chipped corners, occasional hairline cracks, more flange bolts and a hollow chain link.

Layers, bottom to top, each exported as a separate PNG in `layers/`:

1. far gallery (parallax 0.35);
2. mid gallery (0.6);
3. backwall;
4. terrain (floor plane, threshold and front ledge);
5. structure (pillars, jambs, imposts, plinths, voussoir ring, soffit);
6. props (torches, copper pipework, valve, chain);
7. actors (composited from the approved character grids);
8. foreground (low rubble at the corners).

### Lighting pass (A1, B1)

`scripts/lookdev_compose.py` uses Pillow only and is deterministic: a re-run is byte-identical. It nearest-upscales every pixel layer by an integer factor. Then it computes smooth per-channel float light buffers **at the output resolution** from the light list in `assets/source/lookdev/floor01/lighting.json`:

- **cold ambient multiply** per depth group: scene, gallery far, gallery mid, actors, foreground;
- **warm torch pools**: radial falloff sprites. Flat elliptical pools on the floor under each torch are included.
- **cold gallery bounce** and **sky haze** light;
- **additive glow** halos around flames and distant torches;
- **distance haze** on the gallery layers: a constant mix toward cold blue plus a ground-fog ramp;
- **bloom**, whose source is restricted to the emissive layers (props and gallery). Bright actor colours such as Donut's white fur therefore do not bloom.

Actors get a high, nearly neutral ambient (0.9) and 30 % of the torch light, so they stay readable while picking up warmth. A **parallax demo** GIF (`a1_parallax_demo.gif`) shows relative parallax: the near layers stay pinned while the gallery layers slide by (1 − parallax) × the camera offset, and their lights move with them.

## Measured facts

All values are measured by `scripts/lookdev_compose.py` and stored in `measurements.json`, except where a row says otherwise.

### Colour counts

| Image | Unique colours |
| --- | --- |
| A0 native 640×360 (environment and actors) | **130** (environment palette union 85; Carl 23, Donut 22) |
| A0 at 3840×2160 | 130 (nearest upscale adds no colours) |
| A1 native 640×360 | 6,181 |
| A1 at 3840×2160 | 13,777 |
| B1 pixel layers (environment palette union) | 86 |
| B1 native 960×540 (Carl 78 / Donut 60 redraws) | 7,925 |
| B1 at 3840×2160 | 13,789 |

The largest per-layer palette is 26 colours (backwall and props at both densities). Per-layer counts are listed in `scene.json`. These spike palettes are deliberately **not** added to `assets/manifests/`, whose validator limits a palette to 64 colours.

### Readability: Carl and Donut against their surroundings

For each actor the script measures three values at native resolution. The surround is a ring 6 virtual px wide around the actor's bounding box, with actor pixels excluded. The edge value is the mean |ΔL*| between each silhouette pixel and its non-actor 4-neighbours.

| Frame | Carl ΔL* (actor − surround) | Carl contrast ratio | Carl silhouette edge mean \|ΔL*\| | Donut ΔL* | Donut contrast ratio | Donut silhouette edge |
| --- | --- | --- | --- | --- | --- | --- |
| A0 | +14.0 (41.6 vs 27.6) | 2.01 | 15.0 | +13.2 (49.8 vs 36.6) | 1.78 | 34.7 |
| A1 | **+18.2** (39.1 vs 20.9) | **2.35** | 13.1 | **+16.3** (47.4 vs 31.1) | **2.00** | 33.0 |
| B1 (Carl 78 / Donut 60 redraws) | **+19.3** (40.9 vs 21.6) | **2.44** | 13.3 | +15.5 (48.3 vs 32.8) | 1.94 | 33.7 |

The lighting pass **improves** actor-to-surround separation, because the cold ambient darkens the environment more than the actors. B1's Carl is slightly brighter than A1's because the redraw adds lit detail. Carl's silhouette edge contrast is the weakest value in every frame (13–15 ΔL*). His dark jacket sits against the dark lower wall and the stone base course. A COMP-01 option is a 1 px warm rim or a slight actor-only key light. Neither is applied here.

### Art pixel area per asset

| Asset | 640×360 density | 960×540 density | Ratio |
| --- | --- | --- | --- |
| Whole frame | 230,400 px | 518,400 px | 2.25× |
| Structure layer (arch, pillars) | 36,672 opaque px | 82,258 | 2.24× |
| Props layer (four torches, pipework, chain) | 6,391 | 14,512 | 2.27× |
| Carl, frame 0 | 837 (52 px tall, approved) | 1,815 (78 px look-dev redraw) | 2.17× |
| Donut, frame 0 | 1,305 (40 px tall, approved) | 2,843 (60 px look-dev redraw) | 2.18× |

**Estimated cost multiplier (judgement, not measured):**

- Generated or rule-based environment art costs roughly 1× to switch density, because only the density-specific rules change.
- Hand-placed sprite pixels cost about **1.8–2.5× per frame** at 960×540: 2.25× more pixels, partly offset by bigger clusters.
- Animation likely costs **more than 2.25×** overall. At higher density, steps of 1 px are smaller, so motion arcs show more and need more in-betweens or cleaner sub-pixel animation.
- Every approved master (Carl 52, Donut 40, Mongo 76) would need a full redraw, not a resample.

### Integer-scale table

| Display | 640×360 | 960×540 |
| --- | --- | --- |
| 3840×2160 (4K) | **6×**, exact | **4×**, exact |
| 2560×1440 | **4×**, exact | 2.67×: non-integer. Needs 2× letterboxed (1920×1080 inside 2560×1440) or sharp-bilinear. |
| 1920×1080 | **3×**, exact | **2×**, exact |
| 1280×720 | 2× | 1.33×: non-integer |
| 1280×800 (Steam Deck) | **2×** with the 640×400 view (16:10, exact) | 1.33×: non-integer. Either sharp-bilinear (`deck/*_sharp_bilinear_1.33x.png`) or 1× letterboxed, which uses about 51 % of the screen (`deck/*_1x_letterbox.png`). |

The 640×360 / 640×400 pair is integer at every target listed. 960×540 is integer only at 4K and 1080p.

## Pros and cons (agent assessment, for the owner to judge on a 4K display)

**A0: 640×360, baked only**

- Pros:
  - True palette-limited pixel art (130 colours).
  - Exact integer scale everywhere.
  - Cheapest to author and animate.
  - The approved character masters are used unchanged.
  - Warm-to-cold brick and copper read correctly even without a lighting pass.
- Cons:
  - Light falloff is stepped per block. Pools read as tiled bands instead of glow.
  - No haze, so the gallery is flatter.
  - At 6× on 4K each art pixel is a 6×6 block. That is very chunky up close, although clean.

**A1: 640×360 + full-resolution lighting pass**

- Pros:
  - Closest to the concept's mood for the cost: smooth pools, cold falloff, glowing flames, a hazy deep gallery.
  - Best actor separation of the three frames (measured above).
  - Same art cost as A0.
  - Pixel layers stay crisp because only the light is smooth.
- Cons:
  - Needs a real rendering feature in Bevy (light buffer and composite). See the next section.
  - Pools can over-brighten already hot baked pixels, so baked art and the light pass must be tuned together.
  - Lit images are no longer palette-limited, so a palette validator can only check the layers, not the final frame.

**B1: 960×540 redraw + lighting pass**

- Pros:
  - Finer bevels, in-brick gradients, cracks and chips.
  - Characters carry visibly more facial and costume detail: Carl's almond eyes and modelled nose; Donut's larger eyes, catchlights and charm. See the 4K crop.
  - At 4× on 4K the blocks are 4×4, which looks less chunky up close.
- Cons:
  - Measured: about 2.25× the art pixels per asset.
  - Non-integer on Steam Deck and 1440p.
  - Every approved master needs a redraw.
  - In this spike the mood at normal viewing distance looks almost the same as A1. Most of the atmosphere comes from the lighting pass, not from density.
  - The gallery and haze layers gain almost nothing from the extra density.

## Proposed Bevy 0.20 mapping, for COMP-01

These are **proposals, not verified Bevy capabilities.** API names and availability must be checked against the Bevy 0.20.1 documentation and a running prototype before anyone relies on them. The repository already uses `ImagePlugin::default_nearest()` and a fixed 640×360 / 640×400 logical camera.

| Compositor step | Proposed Bevy approach |
| --- | --- |
| Nearest integer upscale of pixel layers | Either render the pixel layers to an offscreen image at the virtual resolution and draw it scaled by an integer factor, or keep rendering at full resolution with an integer camera scale and snapped positions, as the current playground does. |
| Cold ambient multiply and warm pools (per depth group) | Draw light sprites (a radial falloff texture, tinted, flattened for floor pools) additively into a full-resolution light-accumulation texture with a separate camera or render layer. Then multiply the scene by it in a full-screen 2D material. Per-group ambient needs either one pass per depth group (gallery, scene, actors) or a group ID per sprite. |
| Additive glow halos | Sprites with an additive blend state, through a custom 2D material, drawn above the scene. |
| Bloom on flames only | The engine's HDR bloom post-process, if it is available for the 2D camera in 0.20, fed only by emissive flame sprites with values above 1.0. Otherwise a small custom downsample, blur and add pass. |
| Distance haze per layer | A per-layer tint or mix uniform on the parallax layer's sprite material, plus a vertical fog ramp texture. |
| Parallax | Per-layer offset = camera x × (1 − factor), snapped to virtual pixels. Gallery lights move with their layer. |

## Self-review log

The full frames and the 4K crops were reviewed at native size and at 4K 1:1 against the owner feedback.

1. **First pass.** The light falloff was too broad, so the wall was flatly orange everywhere and did not fall into cold shadow. The soffit had a glyph-like noisy band. The mid-gallery torch made a hard orange stripe. The copper looked dull and thin. **Fix:** windowed inverse-square falloff; hue-shifted brick shadows toward blue-violet; redrawn soffit with courses that follow the voussoir divisions; softened mid-gallery warm pool; thicker pipe with stronger cylinder shading and a specular streak.
2. **Second pass.** The arch read as a dark, unlit ring. Two of the three distant torches were hidden behind the mid pillars. The lighting pass blew out the pools to near-white. Bloom made Donut's white fur glow. B cracks read as scribbles. **Fix:** torches added on both pillars, as in the concept's rhythm; far arcade re-phased so all four distant torches show between the mid pillars; pool intensities lowered and floor pools added; bloom restricted to emissive layers; fewer and shorter cracks.

**Honest gap to the concept.**

- A0 gets the colour temperature, the warm-to-cold brick and the copper. It does not get the glow or the atmospheric depth.
- A1 gets closest. The pools, flame glow, cold top and hazy deep gallery read as the same world.
- None of the frames matches the concept's very strong arch volume. The concept shows big protruding pillars in strong perspective and a deep, brightly lit floor. The spike's side-on, parallax-safe volume (jambs, soffit, plinths, 24 px floor plane) is deliberately more restrained.
- The ledge front is plain.
- The gallery is cleaner and emptier than the concept's.

## Open questions for the owner

1. Which target: A0, A1 or B1? Or A1 now, with B kept as a later option for close-up scenes?
2. If A1 is chosen: are the pool strength and the cold ambient right, or should shadows be bluer or darker?
3. Floor plane: keep the 24 px top-down plane with the feet 4 px behind the lip? This changes the COMP-01 terrain contract compared with ENV-01's 5 px surface.
4. Pillar torches: are four torches in one screen acceptable for B sections, or too warm?
5. Should the Environment lock wording ("muted red-brown brick") be updated to "warm torch-lit brick that falls off into cold blue shadow"?

## Reproduce

```bash
python3 scripts/lookdev_floor01_author.py      # grids, scene.json and .ase files (deterministic)
python3 scripts/lookdev_compose.py all --out docs/evidence/lookdev-floor01-resolution-spike
```
