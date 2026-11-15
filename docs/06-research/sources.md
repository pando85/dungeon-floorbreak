# Verified technical source register (2026-11-15)

This file records **primary technical sources** checked while designing the project. Links are not a guarantee that every runtime plugin combination already compiles in this repo. Re-check upstream compatibility immediately before dependency pinning and before Steam release.

| Topic | Current checked information | Primary source |
| --- | --- | --- |
| Bevy | 0.20 released October 8, 2026; latest observed patch 0.20.1 on Oct 28 | https://bevy.org/news/bevy-0-20/ and https://github.com/bevyengine/bevy/releases |
| Bevy tilemap | `bevy_ecs_tilemap 0.20.0` provides entity-per-tile, chunk-based rendering | https://docs.rs/bevy_ecs_tilemap/latest/bevy_ecs_tilemap/ |
| LDtk integration | `bevy_ecs_ldtk 0.16.0` released Oct 25, 2026; targets Bevy 0.20 | https://github.com/Trouv/bevy_ecs_ldtk/releases and https://github.com/Trouv/bevy_ecs_ldtk |
| Avian physics | `avian2d 0.8` compatible with Bevy 0.20 per upstream version table | https://github.com/avianphysics/avian |
| LDtk | JSON format 1.5.3 documents maps/layers/entities | https://ldtk.io/json/ |
| Aseprite | CLI supports batch sprite-sheet/JSON export | https://aseprite.org/docs/cli/ |
| Aseprite scripting | Lua scripting API supported | https://www.aseprite.org/api/ |
| Steamworks | Store, SDK, upload, Deck and Remote Play docs | https://partner.steamgames.com/doc/ |

## Sources that should be rechecked before publishing

- Steamworks current AI-generated-content reporting and developer questionnaires.
- Steam Deck compatibility and performance/certification process.
- Bevy `0.20.x` minor/patch API behavior; platform Linux GPU/backend specifics.
- Licenses for every third-party code crate, game asset and font; correct commercial use rights.
- Official rights/permission for the Dungeon Crawler Carl franchise (no assumption or implicit permission).

## Unverified project hypotheses (not sourced facts)

- 640×360 + 640×400 virtual resolution is a good final camera design.
- 16 px tile material cells and ~32×32 cell dirty chunks are optimal.
- Bevy + Aseprite will outperform Godot in an AI-only workflow.
- Solo swap + companion AI is more fun than command-only AI.
- The future Steam build will run at 60 fps on Steam Deck.

Each is a **testable hypothesis**; record evidence and update ADRs.
