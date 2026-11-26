# Local-agent plan — Floor 01 area kits, after the first integrated corridor

**Do not execute all tasks concurrently.** The [Floor 1 art atlas](../02-art/floor-01-art-atlas-v1.md) records the current approved concept direction and its image boundaries. The [first Floor 1 encounter handoff](floor-01-opening-encounter-handoff.md) remains applicable.

## Dependencies

1. **First:** review [COMP-01 PR #29](https://github.com/pando85/dungeon-floorbreak/pull/29), the first genuine native-scale corridor, using actual render screenshots and collision/simulation traces. COMP-01 has pixel layers only; high-resolution lighting is a separate planned step.
2. **Then:** verify Carl 52 px and Donut 40 px, the scene atmosphere, selected lighting direction, camera and foot contact **in Bevy**. The user explicitly put human visual/controller acceptance at the integrated scene stage.
3. **Only then:** implement small area-extension kits, reusing existing editable ENV-01 source/contracts. **Rust + Bevy only. No Godot.** Do not derive runtime pixels from concept PNGs.
4. Keep book locations separate from authored game topology; the art inventory numbers are not a confirmed linear map.

## Independent work orders (proposed, not yet scheduled)

| Work order | Scope | Evidence | Explicit non-goals |
| --- | --- | --- | --- |
| **AREA-REFUGE-01** | Modest hearth, bed, shelves and functional door for Mordecai's room | Editable sources, deterministic exports, 1×/2× contacts, real Bevy capture | New combat systems or unapproved character animations |
| **AREA-HEAT-01** | Small scorched/burnt edge and lava-spit residue variants of the shared corridor | Unaltered vs scorched native scene comparison | Turning the neighborhood into a lava river biome |
| **AREA-FASTFOOD-01** | Blue/white 1990s restaurant furniture, counter and play props | Modular source layers, two virtual resolutions | Real-world logos, menu text, economy |
| **AREA-HOARDER-01** | Localized junk, fragile props, refuse and debris | Source/provenance, scene profile and occlusion review | Covering all passages with junk |
| **AREA-GOBLINWORKSHOP-01** | Boiler, tools, pipes, gears and parked vehicle-space props for later workshop | Editable layered sources and independent wheel-motion evidence | Replacing the earlier chase with a stationary factory boss |
| **AREA-HAZARDS-01** | Separate corrosive slime decals and ceiling/wall Rot Sticker tells | Native 1× hazard legibility; deterministic gameplay tests only when behavior is added | Merging slime/Rot Sticker geography or inventing monster abilities |
| **AREA-RINGS-01** | Ring structures and large gates; a separate improvised camp prop family | Native geometry study, map/camera checks | Full maze navigation inferred from a painting |
| **AREA-GYM-01** | First make a *dedicated standalone concept*; then turnstile, mirrors, weights, small props | Human-selected detailed study, native source kit, collisions when implemented | Treating the overview panel crop as final art |
| **AREA-TROG-01** | Troglodyte territory decals (knuckle marks, the fist-shaped hole, the bell), a stone-to-gym-tile threshold and the 3-state `gym_bench_stack` for `trog_run` and `trog_strength`, reusing ENV-01 and AREA-GYM-01 ([troglodyte plan section 16](../01-design/floor-01-troglodyte-integration.md#16-environment-art-companion-stub-for-the-art-queue-area-trog-01)) | Editable sources on `floor01-catacomb-v2`, 1×/2× contacts, real Bevy capture of both rooms | A new reception or turnstile (zone A of the arena has them); sprites that change collision |
| **AREA-ARENA-01** | First make a mechanical geometry concept with movable walls/grooves, then lighting phases | Readable unlit plan; deterministic reachability under wall changes; special ultraviolet VFX afterward | Magenta lights substituting for collision and arena design |

Each source file has one art-agent owner. No concurrent edits to the shared palette, collision schema, generated atlas manifest contract or canonical character masters without integrator coordination. Every PR follows [workorder protocol](workorders.md).

## Scene acceptance

Deliver both an **empty environment** contact sheet and a **real playable Bevy screenshot** with approved cast. Virtual 640×360 and 640×400 unless superseded by an ADR; source pixels stay integer-scaled, pivots stable. Show clear safe passage, terrain collision and source-to-export provenance. Human reviews the integrated gameplay/atmosphere rather than assuming a screenshot proves game feel.

## No-go list

- No generated Carl/Donut replacements or shotgun.
- No ripped assets, copyrighted novel passages, real-world fast-food brand logos, or licensed soundtrack.
- No broadcast aesthetics throughout the early floor; ultraviolet show lighting belongs to the late arena's special event.
- No dense boiler installations outside the separate goblin workshop.
- No universal garbage/sewer look.
- No unapproved exact enemy pixel heights, inferred sprite bounding-box colliders or player ability upgrades from the generated concept illustrations.
- No presumed public executable rights from published concept art.
