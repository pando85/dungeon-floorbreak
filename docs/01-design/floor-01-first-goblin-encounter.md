# Floor 01 — First Goblin Encounter: "Carl's First Revenge"

**Status:** GAMEPLAY DESIGN HYPOTHESIS / OWNER REVIEW REQUIRED (2026-11-18). This is a detailed **proposal** for Act I beat **I-03**, not an approved level layout, accepted hit/damage tuning, asset-production specification or Bevy implementation. It extends the [Act I beat document](floor-01-act-01-playable-combat-beats.md) and **does not overwrite ENV-02**.

**North star:** One short, brutally satisfying combat following the Murder Dozer chase. The player should finish it and want to repeat it because *kicking a goblin, knocking enemies into one another, and arranging a falling-rock kill feel better than merely pressing attack*. The most important fun gate is still **the unassisted, direct first goblin kill**. No kill counter, compulsory spectacle, QTE, grapple, explosive, unearned magic, or puzzle lock.

**Controls / content boundary:** [Approved MVP verbs](mvp-controller-contract.md) remain: Carl Square/X punch, Triangle/Y **kick** (never grab), Cross/A jump; Donut pre-biscuit low-damage feline action and higher/farther jump. Circle/B special is locked. R1/RB interacts with authored objects, but **does not** perform the rock kill: **Carl physically hits the shore with normal attacks**. L1/LB solo swap; independent controllers for local 2P once that feature exists.

**Read with:** [ENV-02 opening route](../02-art/env-02-opening-spec.md), [integrated combat probes CP-01–04](../07-agents/floor-01-integrated-combat-probes.md), [combat-feel contract](combat-feel-hybrid-physics.md), [Floor 01 scene status](../03-tech/floor01-scene.md), [character horizon](character-progression-horizon.md). Do not re-create the scene as a standalone demo.

## 1. Locked ENV-02 geography versus proposed encounter timing

Existing **ENV-02 route proposal**, six segments at 224×25 16-px tiles, with option **R** chase ceiling. **No proposal here modifies these authored coordinates**:

| World x / region | Existing authored meaning | Encounter use / new unapproved hypothesis |
| --- | --- | --- |
| S4, x1712–2608 | Dozer overtakes lane after niche, smashes barricade, wedges in S5 arch | Pursuer is no longer an active combat collider in I-03 |
| x2608–3296 | S5 skirmish | **Single continuous** first melee encounter, not three new rooms |
| x2640 | `checkpoint_post_chase` | Restart here after fight failure, without replaying entire chase |
| approx. x2700–2930 | S5 clear left approach (no special marker; illustrative range) | Candidate first one-on-one contact / learn jab + kick |
| x~2975, y271–290 | `breakable_shore` strike band | Carl can hit rotten shore on normal attack; **3 hits** is current ENV-02 proposal |
| x3000–3080, y168–308 | `hazard_fall_region` (80px width), `falling_block` | Threat + optional authored environmental kill; single landing event |
| x3136 / 3176 / 3216 | `enemy_spawn_a/b/c` | **Two basic goblins** for initial gameplay baseline; third is a later optional variant, not an extra unavoidable wave |
| x≥3232 | `protected_exit_region` | Cannot be damaged, covered by a blocking corpse, or sealed by stone; do not place a live blocking spawn on this protected area |
| S6, x3296–3584 | Guild portal / exit | Reachability preserved regardless of whether shore is destroyed or enemies die |

The existing goblin spawn positions are **to the right of the falling-rock footprint**; enemies moving left naturally pass through it. Place them and stage their first actions using the existing markers; do not move the support under the first target to guarantee a scripted kill.

**Continuity:** riders tumbling out of the wrecked vehicle may visually explain why the goblins are present, but the exact landing arcs/AI recovery are hypotheses. Dozer background animation may continue but must not inflict unexplained combat damage. The player reaches this encounter still without bombs or Donut magic.

### Side-view gameplay sketch (not a collider/sprite specification)

```text
S4 Dozer wedge           S5 first fight (~688 px total)                         S6 Guild
    |                 x2640        first contact       shore |fall|   a  b  c     |exit
    v                    v             ~2800–2930      ~2975 3000–3080 3136...  ≥3232
[escape completes]  [checkpoint]   [Carl / Donut]       /|     [rock]   [goblins]   [safe]
============================ protected continuous ground ============================
            attack normally / kick -> contacts -> optional player-triggered collapse
```

One gameplay camera may pan within this route; no zoom-out is required merely because a rock and three goblins exist.

## 2. Three micro-beats in one fight

These are **teaching affordances**, **not scripted waves or a fixed compulsory sequence**.

### G1 — The first goblin: "My punch actually hurts"

- **Threat:** one ordinary light-infantry goblin commits to a readable short-range swing. Avoid an immediate unseen strike on exiting the chase.
- **Natural setup candidate:** the first goblin (spawn A at x3136) notices the player and approaches left, crossing the stone footprint into the clearer left space; the second rider (spawn B at x3176) initially has a readable delayed recovery/entry rather than attacking simultaneously. Both characters can see where the enemies came from. Exact perception/arrival times need actual Bevy tuning.
- **Basic answer:** Carl closes distance and lands a simple punch or kick. A few meaningful hits defeat the goblin, with contact hit-stop, hurt frame, stagger/knockback and unmistakable death confirmation. The existing **two-hit** goblin probe is a comparison baseline, **not final HP**.
- **Alternative:** Donut lures/interrupts briefly using *cat-credible* motion or a very weak swat/pounce if implemented; no powerful stun-lock and no new control.
- **Agency:** the player is allowed to ignore this ideal beat order and experiment with the shore early. Do **not** hard-lock the shore until the first goblin dies.
- **Failure signature:** holding a button or spamming attacks causes unclear repeated contact; the goblin dies with no visible reaction; shared camera cuts off the approach; the first kill occurs automatically without player agency.

### G2 — The kick: "I can use enemies as weapons"

- **Threat:** the second goblin enters the conflict with visible intent. The encounter should now permit two nearby active targets, but not two synchronized unseen attacks.
- **Answer:** Triangle/Y produces a deliberate **physical kick**, not the earlier fixture's special input. The first goblin may be staggered/recovering when the other is near; target positioning allows a launched **living** goblin to strike its companion.
- **CP-02 baseline:** *stagger / interrupt / short displacement on contact, **zero enemy-on-enemy contact damage*** until a separate A/B playtest demonstrates that contact damage is fun and understandable. Deduplicate the pair-contact per launch token; never apply damage every overlapping physics frame.
- **No forced combo:** punching both separately is valid; so is kicking one away and pursuing it. Do not require jump+kick or memorized launch inputs.
- **Failure signature:** a long stun-lock trivializes both foes; kick feels like a grab/throw; a single untelegraphed contact kills the whole group; corpses become authoritative bouncing mines.

### G3 — The stone: "I made the dungeon kill it"

- **Opportunity:** a **rotten diagonal timber support** visibly holds up a loose pale masonry block; there is grit on the floor directly underneath (visible landing-zone warning).
- **Player action:** strike the *foot of the support from the safe left side* with Carl's ordinary melee. Existing ENV-02 candidate: **three distinct hit events**. Each hit visibly worsens the timber and shifts the block; after final strike, the block drops after a short authored beat, then falls onto the footprint.
- **Positioning:** enemies advancing left from x3136+ can be baited into x3000–3080; an appropriately timed kick to the right may also reposition a live goblin into that area. **Do not teleport enemies, magnetize the stone, extend its hidden collider, or grant Donut magic to force the outcome.**
- **Existing ENV-02 candidate numbers:** break-to-fall delay ~6 simulation ticks; drop distance ~140 world px and fall time ~0.52 s, to be tested at real game speed. `hazard_fall_region` x3000–3080; strike location outside footprint. These are **authored proposal values, not human-approved balance**.
- **Outcome:** one authoritative landing impact resolves victims *currently overlapping* the authored region. If a goblin is underneath, it dies with credit assigned to the player who caused the support to break. If not, the rock falls harmlessly and the player finishes the fight directly. Allow a player hurt/knockback if inside the clearly warned danger area, subject to a fair/visible experiment.
- **Aftermath:** rock is non-collidable cosmetic rubble as specified by ENV-02; the intact protected floor and Guild exit stay reachable. Donut cannot accidentally become trapped on the wrong side.
- **Failure signature:** dead enemy appears before block landing, false kill attribution, impact through a wall or outside the footprint, repeated damage after landing, or permanent progression block.

**Important staging tradeoff:** because approaching goblins cross the footprint before they reach the first duel, the player can *miss* the ideal stone-kill window and must either bait a living target back or replay. That is good player agency if the tells are clear, but the spatial/timing prototype must demonstrate the trick is discoverable without a forced prompt. If the geometry cannot provide enough time, propose a modest enemy pursuit/recovery-timing change *before* requesting an approved art/marker redesign.

## 3. Goblin AI and reaction contract — minimalist first pass

| State / event | Candidate behavior | Readability / safety gate |
| --- | --- | --- |
| Land / recover after Dozer | Short visible stumble before walking left | No surprise attack when camera first reveals riders |
| Notice / approach | Face and move toward active hero inside perception | Stable facing; avoid jittery target changes on solo swap |
| Telegraph swing | Clearly separate preparation pose, then committed attack | Only enemy in attack range attacks at first; expanding to two requires visible staggered tells |
| Strike / recovery | Short-range swing, then exploitable recovery | Carl/Donut can escape by ordinary movement and jumps; no mandatory dodge |
| Hurt / stagger | Cancel active threat when appropriate; push consistent with lightweight enemy | No infinite control loop; knockback bounded; source of impact clear |
| Airborne/contact | Single launch ID; contacts resolve in stable order | CP-02 initially interrupt-only; no per-tick collision damage |
| Defeated | One defeat event, deterministic credit, visual landing pose | Bodies stop authoritative collision/attacks; CP-04 corpse physics is optional, **not** required |
| Lost target / regroup | Return to reasonable pursuit/alert rather than freeze or teleport | No enemy can hide inside protected Guild exit or permanently stall combat |

**Candidate first-pass pacing:** one enemy arrives ahead of the second by a short, visually motivated recovery gap, while both remain on the same continuous route. An optional third goblin (spawn C) is reserved for a later difficulty/readability check. Do **not** lock wave timing, aggro radius or simultaneous attacker count without simulation and manual review.

## 4. Carl and Donut choices by mode

| Situation | Carl | Donut pre-biscuit | Solo and 2P requirement |
| --- | --- | --- | --- |
| One goblin | Punch/kick for direct kill | Avoid contact, maybe light swat/pounce | Both have purposeful movement; Donut can be controlled from spawn |
| Two goblins | Kick to stagger and launch one into the other | Reposition to distract or interrupt briefly | No precise simultaneous dual action required |
| Stone hazard | Hit the support with physical attack | Bait a target across footprint or safely change position | Player can achieve kill without active Donut intervention |
| Exit / reset | Return to checkpoint or continue Guild | Can follow safely and reach entrance | No lock based on two human players, unreachable ledge or rubble |

**Solo:** test the same encounter with one player swapping (L1) and the other character AI-assisted. When AI/swap is not implemented, mark case **pending**, never silently treat a passive Donut sprite as passed. Swapping while one actor is attacked must not cancel combat reality or teleport enemies.

**2P:** test independent device ownership, active P2 before biscuit, attack feedback visible to both, and no camera abandonment of a flanked character. When 2P is not implemented, mark pending.

**Fairness:** ensure no mandatory co-op puzzle, no side effects from a player quitting midfight, no permanently blocked route, and no hidden friendly-fire contact while the game is teaching basic melee.

## 5. Implementation order and reversible decisions

Keep **one authoritative integrated Floor 01 scene** and its deterministic scenario setups. Do not recreate new input/combat models per beat.

1. **Baseline:** verify normal Dozer -> checkpoint -> goblin fight with the intended S5 positions. CP-01 single enemy, normal punch + telegraph/hurt/death, accepted native art when available.
2. **Kick separation:** map new `PhysicalAttack` to Triangle/Y; preserve older temporary `special_pressed` fixture compatibility as needed. New `EquippedSpecial` remains locked before unlock. Validate repeatable knockback and edge-trigger semantics.
3. **CP-02 collision:** add a second live enemy and contact without damage; keep source/launch IDs and record deterministic ordered events. Enable contact damage only after playtest signoff in a separate, reversible variant.
4. **CP-03 rock:** consume existing ENV-02 shore and fall-region markers; three per-hit visual states, single impact, kill attribution and protected exit reachability. No freeform structural-collapse graph.
5. **Donut parity:** implement/verify direct P2 authority and solo swap/AI in separate ownership-aware work; then replay G1–G3 on the same scene. Don't claim this gate passes just because combat tests pass.
6. **Feel pass:** compare quiet direct kill, kick collision, stone kill and improvised kills in real Bevy video/audio. If G1 is bland, tune it before adding more enemies/rooms.

Use existing CP-01/CP-02/CP-03 fixtures rather than manufacturing a new combat-demo binary; a named I-03 scenario variant may reuse the same runtime seed/setup.

### Exact questions for human review

1. Did the **very first simple kill** already feel good, without props or magic?
2. Did Triangle/Y feel like a satisfying **kick**, never an accidental grab or special?
3. Could the player understand and anticipate the living-enemy collision, and was it worth attempting?
4. Could someone discover the rotten support and deliberately make the stone land on a goblin **without a forced glowing switch**?
5. Did Donut matter before magic, as a human-controlled P2 or meaningful solo swap, rather than a background animation?
6. Was the fight over before it became repetitive? Did it make the player want to replay to try a different kill?
7. On failing the fight, did respawning at x2640 and retrying feel fast and fair?
8. Did the unmodified story route and protected Guild exit remain available under **every** kill/rock order?

## 6. Acceptance scenarios (design tests, not passed checks)

| Scenario | Setup / action | Expected deterministic property | Human evidence |
| --- | --- | --- | --- |
| `I03-DIRECT` | Carl vs one basic goblin, no prop interaction | Single attributed defeat, correct input/attack window, no ghost strikes | First-hit / first-kill recording |
| `I03-KICK` | Two live goblins close; kick one into the other | Once-per-launch interrupt, **zero contact damage** in baseline | Both hit reactions are readable |
| `I03-SHORE-HIT` | Attack shore 1, 2, 3 times | One increment per attack instance; visual damage states; rock breaks once | Wood cracks/stone shifts legibly |
| `I03-ROCK-KILL` | Player causes fall while a goblin is inside footprint at landing | One attributed death at impact tick; never early | Cause -> fall -> kill legible in recording |
| `I03-ROCK-MISS` | Player breaks support while no goblin is inside | No phantom damage, Guild still reachable | Player can finish fight directly |
| `I03-ROCK-FIRST` | Player targets shore before first goblin dies | No hard-coded kill-order gate or invisible lock | Creative solution remains possible |
| `I03-GUILD-EXIT` | All enemy deaths / prop states, including untouched support | Guild portal reachable and operational; collision hashes stable | No softlock; regular scene remains intact |
| `I03-2P` | Two real pads, pre-biscuit Donut controlled by P2 | No cross-player input; no unearned spell; fair camera | Both players engaged |
| `I03-SOLO` | One player swaps midfight, inactive hero is AI-driven | No vanished actor, frozen AI, input duplication or offscreen trap | Switching creates optional tactics |
| `I03-RETRY` | Carl takes lethal hit / rapid checkpoint restart | Enemy, shore and debris return to their initial deterministic state | Retry is welcome, not laborious |

**Gate policy:** deterministic automated results and recorded Bevy game-feel evidence are both required for actual implementation signoff. A PR carrying *this* markdown is **not** a playable encounter. No music/sound, camera shake, kill count, SFX values or damage numbers are declared approved here.

## 7. Key decisions still intentionally open

- Does the stagger-only contact baseline feel substantial enough, or do we test one-shot impact damage as a variant?
- Is the support readable at real 1× scale when the first enemy and protagonist occupy S5?
- Do enemy approach/recovery timings reliably create a voluntary falling-rock setup without dragging out G1?
- Can pre-magic Donut create a credible distraction **without** inventing new cat powers or AI target exploits?
- Do the current authored S5 span and camera rules give the player time to identify *all* three opportunities?
- Does the entire encounter benefit from optional third goblin, or is two the better first-time amount?
- Will the first checkpoint restart reestablish both heroes and terrain consistently under solo and 2P?

**Approval status:** the owner has approved the *combat-first fantasy*, *MVP controls*, *character identity*, and *concept art direction*, **not** this specific encounter choreography. Update this document after playtests or owner review; record what changed rather than quietly converting candidate numbers into immutable contracts.
