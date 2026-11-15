# Vertical slice: The Inspection Corridor

**Status:** proposed test level, NOT implemented. Primary goal: prove *destruction changes decisions* and duo powers feel different, without losing controller readability.

## Player-facing 5-minute experience

One room network, approximately 3–5 minutes for a practiced pair and up to 8 minutes for first-time players. Begin in a ruined inspection annex. Two corridors separated by a brittle wood wall. Carl can breach the wall; Donut slows or repositions approaching guards. Breakable glass reveals a hidden upper ledge. A beam can collapse over a guard if timed. Reinforced floor and exit must survive all attacks. End with an exit and a short victory animation.

## Content budget

- 2 playable character **placeholders first**; art pass only after mechanics work.
- 1 small authored LDtk map, 3 material types + reinforced safety tiles.
- 2 enemy archetypes: ground chaser and ranged guard.
- 1 destructible prop class, 1 bomb/explosion, 1 control spell.
- 1 local co-op mode with two controllers; solo switch mode may use simple follow AI.
- 1 checkpoint and fast retry; 1 exit trigger; lightweight debug HUD.
- 1 performance stress fixture; 1 deterministic seed and replay path.

## Player journey / proof of fun

| Beat | Decision | Proof |
| --- | --- | --- |
| Entrance | Walk, jump, basic attack | Controls feel immediate |
| First guard | Choose melee vs spell control | Distinct combat roles |
| Fork | Break wooden wall or climb alternate path | Destruction creates route |
| Beam | Risk collapse for strategic kill | Environment influences tactics |
| Exit | Recover from unintended destruction | No softlocks |
| Retry | Immediately replay with a second approach | Short sessions stay engaging |

## Acceptance criteria (must observe)

- Character controls independently; at least one tested Linux gamepad profile is functional.
- Two local controllers can play simultaneously without action cross-talk.
- One destructible passage creates a clearly shorter or safer route than the original.
- Destruction updates render and collision within a bounded number of physics ticks (exact limit set by measured pipeline).
- At least one material is immune and visually distinguishable from breakable materials.
- Donut's cast enables a cooperative maneuver not simply equivalent to Carl's attack.
- Guard navigates or recovers safely following wall destruction; player exit remains reachable.
- Reproducible action scenario + input seed returns the same grid/result hash.
- Art proof: readable dark pixel-art character at 1× and 2× display scale, two animated tags each.
- A human gamepad playtest answers 'Is destruction fun and useful?' and records three actionable observations.

## Gates / stop criteria

If the Bevy tooling makes sprite animation or debug captures significantly slower than a small Godot benchmark, run a one-week tech comparison before multiplying content. If explosions lead to frequent stuck states or high frame-time spikes, simplify material model, not add more simulation. If AI animation remains visually inconsistent after 2 iterations with a canonical master, pause asset expansion and revise the pipeline.

## Out of scope

Mongo, full campaign, procedural dungeon, expansive loot, cloud saves, achievements, network rollback, Steam release, official character likeness, music and dialogue.
