# Destructible terrain specification v0.1

## Goal

Terrain destruction must be a **player decision**: open routes, uncover risks, change cover, cause isolated controlled collapses. It must remain readable in a dark pixel-art game and performant on Linux/Steam Deck. There is **no Noita-style per-pixel fluid/material simulation** planned. Start with **Level 2** terrain damage, add authored structural collapse once safe.

## Three scopes

- **L1 — props:** barrels, crates, doors, glass; each is a standard entity with health and bespoke break behavior.
- **L2 — terrain:** bounded 16×16 material cells with health/resistance; a cell can become empty, update graphics and collision. Not every cell is a full 16×16 collider: since the Floor 1 kit v2 ([ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md), [kit contract](../02-art/floor-01-catacomb-kit.md#consumer-contract-comp-01-and-any-other-loader)) a deep-floor `deck_front` cell has the collider `[0, 4, 16, 12]` (its top is the walking line) and `deck_back` is visual only; derive solids from each tile's `collider`, not from the cell bounds.
- **L3 — structural:** tagged supports or small local structural graphs trigger predetermined falling chunks; a later milestone, not an arbitrary physics solver.

## Domain model (illustrative, not final Rust API)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material { Air, Wood, Stone, Metal, Glass, Reinforced }

pub struct TerrainCell {
    pub material: Material,
    pub hp: u16,
    pub flags: u16, // e.g. critical / structural / hazard
}

pub struct DamageEvent {
    pub center_px: (i32, i32),
    pub radius_px: u16,
    pub strength: u16,
    pub kind: DamageKind,
    pub source_entity: u64,
}
```

**Required APIs (shape, not exact signatures):** `material_at`, `is_solid`, `apply_damage(event) -> TerrainDelta`, `consume_dirty_chunks`, `validate_connectivity`, `serialize_grid`, `rebuild_collision_region`. Use integers for material damage and explicit deterministic ordering; collisions can use floats behind adapters if required.

## Material profile — qualitative initial values

| Material | Explosions | Magic control | Charged impact | Notes |
| --- | --- | --- | --- | --- |
| Air | None | None | None | No collision |
| Wood | Vulnerable | Flammable/freeze possible later | Vulnerable | Routes and props |
| Glass | Very vulnerable | Brittle | Vulnerable | Shatter FX |
| Stone | Moderate | Limited | Brittle variants only | Weight/cover |
| Metal | Resistant | Limited | No, except tagged weak support | Visual distinction |
| Reinforced | Immune in v1 | Immune | Immune | Protects critical geometry |

Numbers must live in versioned data and be tuned in tests. Do not hardcode material resistance into particle shaders. **No per-pixel fragments with gameplay collision**.

## Chunk strategy

- Tile size proposed 16 px; group source terrain into e.g. **32×32 tile chunks** (subject to profiling).
- `apply_damage` iterates the minimal bounding rectangle of affected cells and uses a circle/falloff predicate; damage event ordering stable.
- Output a delta of changed cells and affected chunks; atomically apply terrain state changes.
- Update edge/autotile variants in a one-tile neighborhood; deterministic seeded cosmetic debris from event ID.
- For collision, build **merged rectangles/contours** from surviving solids per dirty chunk and update colliders at a controlled point in the fixed schedule. Avoid one collider/body per tile; reconcile actors standing inside newly invalid collision surfaces.
- Adjacent chunk edges need consistent seam updates; include 1-cell overlap in dirty marking for autotiling, and collision seams must not leak.

## Explosion semantics

1. Create one `DamageEvent` with stable ID and impact location; deduplicate chained triggers.
2. Determine affected grid cells by integer math/deterministically ordered traversal.
3. Apply material-specific mitigation and damage once per explosion event.
4. Collect cell deltas, dirty regions and secondary prop triggers with capped chain depth/total events.
5. Recompute gameplay collisions; propagate `TerrainChanged` and path navigation invalidation.
6. Emit optional VFX/SFX/debris requests; cap counts and lifetime; do not gate game state on visual effects.

**Implemented in the combat scene (CARL-PROG-01 CP-C, CP-Q34):** an authored charge reached by a dynamite blast (or by a blast a chain reached) detonates 8 ticks later, to a chain depth of 4, deduplicated per charge. The cap is **engine-wide**: at most 8 charge detonations per tick, any charge (QA, world or chained), with the excess deferred one tick in stable charge-index order, keeping its depth and credit. A fixture or world author who lights 9 or more charges for one tick will see the ninth blow on the next. The first initiator keeps the credit (`BlastCharge.owner` is written only when empty).

## Structural collapse (later / authored)

Level author marks a `support_id`, `dependent_chunk_ids`, `break_threshold`, safe fall region and post-collapse routing constraint. When a tagged support reaches threshold, schedule the authored collapse group with telegraph and bounded dynamic objects. If not safe in the current room, use fixed animated breakup while setting the correct final terrain grid.

## Playability invariants

- At least one reachability path remains or an explicit reset/recovery option is available.
- Checkpoints, mission-critical exits and spawn pads use invulnerable backing or recoverable platforms.
- Players cannot become permanently enclosed in an unescapable solid cage due to collision reconstruction.
- Dangerous chain reactions have readable anticipation and event caps.
- Explosions may change enemy paths, but cannot silently strand an essential objective with no alternate resolution.

## Tests (required)

- Same seed + same action/event sequence → same material grid hash and damage result.
- Bomb on wood destroys expected cells; same bomb on reinforced destroys none.
- Two blasts in one tick produce correct ordered damage (no double application from visual processing).
- Radius on chunk boundary updates both chunks and neighboring autotiles.
- Rebuild removes collision from emptied cells; leaves it on remaining solids and no micro-gaps at chunk seams.
- Collapsed platform does not clip an active player into forbidden tiles.
- Path reachability holds for the authored inspection corridor after scripted destruction patterns; if false, test fails with a map diff.
- Stress: e.g. 50 concurrent visual blast effects, bounded debris and timed collision rebuild; **baseline only after profiling**.

## Instrumentation and performance target

Log `cells_changed`, `chunks_dirty`, `collision_rebuild_ms`, `chain_depth`, `dynamic_debris_count`, `p99_frame_ms`; define explicit budgets from first Steam Deck-class benchmark, not fabricated targets. Use profiler traces and deterministic stress scenarios to tune the chunk strategy.
