# Level data / LDtk integration

## Authoring principle

LDtk files are **source-of-truth authored geometry and entity placements**, while runtime terrain is a mutable copy. The runtime must never save destruction back into canonical `.ldtk` projects. Data is generic and independent from franchise references.

## Proposed layers

| Layer | Content | Required |
| --- | --- | --- |
| `CollisionMaterial` | Integer grid material identifiers + HP profiles | Yes |
| `VisualTerrain` | Environment tiles and edge variants | Yes |
| `Entities` | Spawn points, enemies, props, pickups, exits | Yes |
| `Structure` | Weak supports + dependent regions | Later |
| `NavigationHints` | Routes/links for enemy recovery and QA reachability | Recommended |
| `DecorBackground` | Noncollision parallax/detail | Optional |
| `ForegroundFX` | Visual overlays, may not hide critical UI | Optional |

## Metadata contracts

Level IDs stable and unique, `width/height`, `seed`, checkpoints, exits, bounded camera regions, per-level alternate-route flags, material mapping table and explicit protected critical cells. Every entity has an `id` and typed fields. Schema migrations must be versioned. The `examples/level-rules.example.json` is a **proposed runtime rules manifest**, not a complete LDtk project.

## Validation pipeline

1. Parse LDtk JSON and check expected version/schema.
2. Validate all required layers, entity type names and referenced materials.
3. Ensure all spawn/exits are within map bounds and collide with intended platforms.
4. Perform static reachability on base map. Generate a bounded set of scripted destruction deltas and repeat reachability.
5. Flag unprotected critical edges, inaccessible objectives and invalid weak-support dependencies.
6. Produce preview thumbnail and structural/debug overlay for agent PR review.

## Editor and AI workflow

Agents may edit JSON via a typed intermediate model and serialize valid LDtk data, but never make arbitrary changes to undocumented editor fields. Prefer generating runtime-side content manifests and verifying by opening the source project in LDtk. If LDtk schema complexity becomes a drag, keep authored levels in LDtk and spawn runtime procedural props separately.

## Testing hooks

Provide a level CLI: `validate-level`, `render-level-debug`, `simulate-destruction-scenario`, `check-reachability`. Commands are **planned**, not currently implemented. Sample `level-rules` contracts establish desired fields for first agent implementation.
