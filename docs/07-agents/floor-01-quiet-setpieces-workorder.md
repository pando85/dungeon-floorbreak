# F01-REST-01 — Agent handoff: the quiet setpieces, design to greybox to art

This handoff carries the owner draft [#187](https://github.com/pando85/dungeon-floorbreak/pull/187) onto main, adapted to the code on main and to the owner decision of 2026-11-25 ("go with all": previously gated items proceed with adopted defaults recorded by ID; everything must reach `--scene f01_late --f01-run` in story order for the playtest; large unfinished items merge as a labelled playable greybox). The design and every number are in [Floor 01 Quiet Setpieces and Transitions](../01-design/floor-01-quiet-setpieces-and-transitions.md), section G. **No new ability, button or camera mechanic is authorized.**

## Work orders and ownership

| Order | Tier | Scope | Depends on |
| --- | --- | --- | --- |
| **REST-WIRE** (implementation) | medium | The three greybox rooms `rest_refuge`, `rest_camp`, `rest_aftermath` in the manifest and the run; the `f01_safe_room_reward_found` row armed in the refuge (G.5); the `hero_on_surface` lock and the staff door; the solo swap hint; fixtures, pins and evidence (G.9, G.10). Two PRs in sequence (REST-AD11): **PR 1** the refuge listed in the manifest after the corral, the grant move with the DP fixture retarget, the lock, the hint, the refuge fixtures and evidence; **PR 2** the camp, the aftermath, their decor and the twelve-room run walk | main (the corral, the Ring rooms and the Juicer's ledger grant are merged) |
| **AREA-FASTFOOD-01** (art) | top-tier design, medium execution, independent top-tier review | The original blue/white 1990s restaurant kit for REST-A, on the greybox rects | REST-WIRE merged (the rects are its contract) |
| **AREA-CAMP-01** (art) | same pipeline | The camp interior for REST-B, extending AREA-RINGS-01's `floor01_ring_camp` family; anonymous figures until REST-Q2 is answered | REST-WIRE merged; the `ring_marker_a` sprite wiring |
| REST-C dressing | same pipeline | Reuses AREA-GYM-01 and AREA-ARENA-01 pieces when those kits exist | those kits |

The model that designed, executed and reviewed each art order is recorded in that PR's `Rights/Provenance` section.

## Gates kept from the owner draft

- **The Infernal Corral (#181)** owns the llama fight and its threshold. REST-A begins after the corral exit; do not retime the corral.
- **DONUT-PROG-01** owns Light on Your Feet; **CP-A** owns the ledger. REST-A arms the existing row; it adds no second unlock system and grants nothing at the corral exit.
- **Option A1** is adopted (REST-AD5) with standing on the shelf as the act, because Interact R1 is not implemented. Option A2 (no lock) is the recorded game-feel fallback (REST-Q1), a one-line sidecar change.
- **The integration plan** ([f01-playtest-integration-plan](f01-playtest-integration-plan.md), section 16) owns run routing, the hot-file queue and the K1 chronology (dynamite after the Hoarder: never in REST-A).
- **The Juicer and the Ball of Swine** keep their own scenes. REST-C shows the Juicer's grants and grants nothing; it exits to `mastery` until #192 is integrated (REST-Q7).
- **PLAYTRACE** records and verifies each REST room like every run room; an unsupported transition is a blocker to report, not a result to invent.
- No item economy, inventory, shop, paid buff, checkpoint menu, dialogue system, NPC AI framework, player movement verb or floor rebuild.
- If an upstream contract blocks a part, hand off a **playable greybox subset and a precise blocker report**, not a false green status.

## Acceptance and evidence

Follow REST-A-01 to REST-ALL-02 of the design document and the test and evidence lists of its sections G.9 and G.10. Report the actual commands and outputs, the deterministic reachability results, the before/after 128 px proof, the grant at the box and its persistence through retry and restart, and correct solo and 2P state. **Machine tests cannot decide the pacing of a pause:** after merge every REST room is "merged, game-feel review pending" until the owner plays it.

All new copy and assets must be original and follow the project's rights and provenance rules (no brand, logo, menu text, music or book quote).
