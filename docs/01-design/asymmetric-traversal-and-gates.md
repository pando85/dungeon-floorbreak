# Asymmetric traversal and character-gated progression — mandatory level-design rule

**Status: owner-approved core mechanic (2026-11-17); design specification, NOT currently implemented in the integrated Bevy scene.**

Read alongside [Floor 01 gameplay/progression](floor-01-gameplay-progression-v0.md), [core gameplay](gameplay.md), [pre-magic Donut experiments](floor-01-pre-magic-donut-experiments.md), [level data](../03-tech/level-data.md), and [integrated playtest](../07-agents/floor-01-integrated-scene-playtest.md).

## Higher-order product priority: spectacular combat

The creative director explicitly approved [combat-first encounter design](combat-first-encounter-design.md) on 2026-11-17. **This asymmetric-traversal rule enriches encounters but must not dominate the playable experience.** A high Donut vantage should often support attacks or evasion; a destructible Carl gate should frequently create a breach, flanking opportunity, trap or memorable enemy elimination. Do not turn every room into a lock, switch, wait and regroup puzzle. Quiet traversal sections remain valid intentional pacing.

## Rule: switching is part of the solution

**Do not require every room to offer one route for Carl and one equivalent route for Donut.** A room, obstacle, encounter or progression gate **may require the distinct capability of one specific character**. In solo, switching to that character is an intended action, not a design defect or a failure of the other character. In 2P, the player controlling that character performs that role and the other contributes where appropriate.

**Approved asymmetry:**
- **Donut conquers space:** she can ultimately jump *higher and farther* and reach small, elevated or distant platforms Carl cannot traverse directly with his base moves. The specific accelerations, jump height, air control, and platform metrics remain playtest-tuned hypotheses.
- **Carl transforms space:** he has stronger physical interactions and *selectively breaks/removes/opens* obstacles to create a viable route that Donut cannot open the same way. **What Carl can break depends on the ability and tools already earned in the current story phase**; do not assign starting explosives early.
- Neither hero is a reskin of the other, but neither must have exclusive utility in every room. Their capabilities can change, overlap and combine as the floor progresses.

**Core progression direction:** begin with obvious character-specific gates (learn "switch to Donut for the jump", "switch to Carl for the wall"). Later present less intuitive, multi-step, reordered or hybrid solutions, including destruction that creates new landing surfaces, high access that triggers a physical change for Carl, and after-unlock magic or cooperation. A later ability can offer a **different** valid solution to a previously familiar gate. Progression should enrich choices, not merely inflate damage or jump distances.

### Non-negotiable constraints

1. **Solo solvability, not single-character solvability.** Every story-critical challenge must be solvable by *one human controlling both characters* through instant switching, contextual AI orders and/or reliable companion behavior. It does **not** need to be solvable by Carl alone or Donut alone. This corrects any earlier wording implying each individual hero must independently traverse all rooms.
2. **2P solvability without a second equivalent route.** Two humans can have distinct responsibilities; a critical action may be executable only by P1-Carl or only by P2-Donut. Do **not** require simultaneous impossible inputs, abandonment of one player behind an irreversible gate, or indefinite inactive waiting for either player. Place safe regroup positions, short actions, useful support, optional interaction, or danger avoidance near asymmetric gates.
3. **Respect progression.** Every mandatory gate must require an ability already unlocked at the point it is encountered, including a fresh run, after death/retry, and after reconnect. Level metadata must declare intended gate capabilities and story phase; no late-book unlocks as an emergency workaround.
4. **Avoid softlocks.** Destruction, falling, losing an item, camera separation, companion AI, checkpoint resets, mounting and controller changes must not eliminate the last reachable progression solution. A required one-character action must remain repeatable or have an authored recovery route.
5. **Teach, combine, subvert.** The first instance of a gate should communicate the relevant affordance clearly through geometry/material/silhouette and optional unobtrusive hints. Later gates may be less obvious but must remain *legible and learnable*; do not substitute arbitrary hidden interactions for genuine difficulty.
6. **Shoulder Mode is not a solution prerequisite.** Mounting, shoulder launch, mounted aiming and mounted specials remain experiments. Do not author mandatory shoulder-only gates until the mechanic is human-approved, supports both 1P and 2P, and is integrated/tested.
7. **Same game and narrative in solo/2P.** The authored critical path is invariant across human player count, even though the control method differs. Never create a human-P2-only lock; a Donut-required action is valid because the solo player can switch to Donut.

## Authoring grammar (choose based on the room, not a required quota)

| Pattern | Required action / solution | Intended learning | Failure to avoid |
| --- | --- | --- | --- |
| **DONUT_GATE** | Only Donut can reach a high or far landing at current ability state and activate a reachable progression interaction | Recognize mobility as a tool, choose/switch actor | Carl stranded or the next mandatory exit inaccessible to his follower/AI |
| **CARL_GATE** | Only Carl can remove a designated breakable obstruction/open a heavy physical route | Destruction is a deliberate movement choice | All masonry looks destructible, or required damage capability unavailable |
| **HANDOFF** | One character changes the geometry or objective so the other can advance; later they reunite | Switch and cooperate, each has meaningful agency | 2P spectator wait or solo AI cannot safely rejoin |
| **OPTIONAL_DUAL_PATH** | Both routes exist, with different risks, resources or time | Replayability and tactical preference | Every room reduced to two predictable character lanes |
| **MULTI_ABILITY_GATE** | A short sequence combines earned character skills, perhaps in a non-obvious order | Mastery and recontextualization | Unsignposted puzzle of unknown prerequisites |
| **EMERGENT_ROUTE** | Several previously learned abilities/destruction choices can transform an obstacle into a passable route | Creative play and non-intuitive discoveries | Critical path depends on a fragile physics glitch or unbounded blast |

**These are patterns, not a mandate for parallel branches.** Many rooms can be ordinary traversal, combat, exploration, safe zones, or visual beats.

## Floor 01 example beats (design adaptations, not book-canon map facts)

### Early: one obvious specialist at a time

**G1 — Donut's elevated ledge.** A gap/ledge is reachable through Donut's superior jump at her current movement tuning. She can perform a small, physically credible interaction that opens a **shared** onward route; solo player switches to Donut, then reunites with Carl. In 2P Carl has a safe position and useful situational context while waiting. An even simpler learning variant is a Donut-only optional collectible followed immediately by a shared path.

**G2 — Carl's weak barrier.** On the next beat, an authored fragile obstacle blocks the single forward route. Carl can break/remove it with an **already available** action; Donut cannot. Once open, *both* can progress through the same passage. No alternate cat-only lane is required.

These two short challenges may be separate rooms and need not have symmetrical geometry.

### Mid-floor: a sequence, not a mirror

**G3 — High access changes the lower route.** Donut reaches an upper fixture and releases a short bridge or makes a support accessible. Carl can then break a support or obstacle, opening the critical passage. In solo, use two intentional switches or one switch plus a reliable contextual command; in 2P, each has a distinct task. Both reunite at a safe checkpoint.

**G4 — Recontextualized destruction.** Carl's breakable terrain creates a landing platform, which makes a previously unhelpful Donut jump possible. Alternatively, a new ability obtained **at that story point** offers a different solution. The level should teach each constituent action earlier, then combine them rather than hiding a random trigger.

### Late-floor: discover instead of mechanically obey

**G5 — The apparent obvious route isn't the only route.** A high gap looks like a Donut gate, but Carl's new tool can convert a nearby object into a new path; or an opening that once required Carl's damage can be approached by Donut using a newly earned interaction. These are **proposals for unlocked skills**, not claims of actual canonical spells/items. Reward lateral thinking; avoid one exact sequence solved by unexplained trial and error.

## Implementation handoff / level-data requirements (future, not current schema)

Level/encounter authoring should **declare**, in versioned generic data (not character-brand IDs):
- Gate ID, story/unlock phase, required capability tags (e.g. `high_traversal`, `break_weak_wall`), optional solution sets and the intended actor(s).
- Spawn, checkpoint, protected critical cells, destructible deltas, exit and regroup/recovery locations.
- A solo plan: can one human switch and bring the companion through? Which contextual actions must AI perform reliably (if any)?
- A 2P plan: where does the non-leading actor wait, act, evade, or rejoin; how does the shared camera behave while separated?
- Reachability scenarios: each required unlock phase, solo Carl/Donut active at start, 2P, after destruction, after partial interaction, after death/restart, after device join/leave, after an optional mount/dismount if implemented.

Do not prematurely change LDtk/Bevy schemas in this documentation PR. Implement a narrow authored fixture first, extend the validator with **actor/capability-aware reachability** when the actual gameplay contract exists, and keep tests deterministic/seeded. Record game-specific narrative mapping outside generic runtime gate identifiers.

## Prototype recommendation: shift away from mandatory mirrored paths

Build a short **two-gate sequence** (not two parallel lanes): first Donut's elevated access opens a shared way, then Carl breaks a weak barrier that opens another shared way. Compare with an all-common route baseline, using human swap/2P control. Only after this works, add one compact HANDOFF gate. The experiment should answer whether switching feels purposeful or like busywork, and whether the inactive/other actor can rejoin without an irritating camera or pathfinding problem.

**Acceptance target:** observable difference in each actor's traversal utility; solo completes by switching; 2P completes with independent roles; no mandatory shoulder feature; no softlocks. Human game feel remains the gate. No exact jump values or barrier-HP numbers are approved by this document.

## Design review checklist

- Who has the currently available capability that solves this obstacle? Why that actor?
- Is a parallel route *actually* beneficial here, or are we adding one just to be symmetrical?
- Can solo switch and complete the entire critical path with both heroes reaching the next safe state?
- Can 2P contribute and regroup without a long spectator wait or forced zoom that ruins native pixels?
- Is the correct solution understandable at this stage of progression? Which learned rule does it use?
- What new solutions emerge when an ability is unlocked later?
- Can any destruction/physics event or checkpoint order remove the final viable solution?
- Have we distinguished verified novel traits from proposed game-specific mechanics?

**Rule owner:** creative director. Do not silently revert to a two-routes-per-room mandate.
