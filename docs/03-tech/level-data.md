# Level data / LDtk integration

## Authoring principle

LDtk files are **source-of-truth authored geometry and entity placements**, while runtime terrain is a mutable copy. The runtime must never save destruction back into canonical `.ldtk` projects. Data is generic and independent from franchise references.

## Mandatory design constraint: character-gated traversal

The owner has approved [asymmetric traversal and character-gated progression](../01-design/asymmetric-traversal-and-gates.md). **A room does NOT need two parallel or individually passable routes.** A critical gate may require Donut's higher/farther traversal, Carl's ability to selectively destroy/open a passage, or a staged handoff between them. Solo play achieves progress by switching actors and/or using reliable AI/context actions; 2P players perform the same distinct-character tasks.

Future LDtk/gameplay contracts **should** expose generic, versioned gate capability tags and story-unlock prerequisites (e.g. `high_traversal`, `break_weak_wall`), without coupling engine rules to franchise character names. This is a **proposed schema requirement**, not a claim that such metadata already exists or approval to rewrite the current data format.

Validation must check **solo party reachability with switching** and **2P party reachability**, not demand that Carl and Donut are each individually capable of clearing every room. Include safe regrouping, actor separation, checkpoints and alternate/destructible terrain states in the test plan. Gate prerequisites must be available at that story point; no impossible future-unlock requirements.

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
3. Ensure all spawn/exits are within map bounds and collide with intended platforms. The standing surface is the **walking line** inside the deep floor tile (collider rect offset), not the tile top, and markers sit one pixel above it (ADR-0007; `env-kit-v2` in [floor-01-catacomb-kit.md](../02-art/floor-01-catacomb-kit.md)).
4. Perform static reachability on base map. When actor/capability-aware gate data is introduced, test one-human **switchable-party** reachability, two-player party reachability, regrouping, and ability unlock phases; do **not** require individually complete routes. Generate a bounded set of scripted destruction deltas and repeat reachability.
5. Flag unprotected critical edges, inaccessible objectives and invalid weak-support dependencies.
6. Produce preview thumbnail and structural/debug overlay for agent PR review.

## Editor and AI workflow

Agents may edit JSON via a typed intermediate model and serialize valid LDtk data, but never make arbitrary changes to undocumented editor fields. Prefer generating runtime-side content manifests and verifying by opening the source project in LDtk. If LDtk schema complexity becomes a drag, keep authored levels in LDtk and spawn runtime procedural props separately.

## Testing hooks

Provide a level CLI: `validate-level`, `render-level-debug`, `simulate-destruction-scenario`, `check-reachability`. Commands are **planned**, not currently implemented. Sample `level-rules` contracts establish desired fields for first agent implementation.
