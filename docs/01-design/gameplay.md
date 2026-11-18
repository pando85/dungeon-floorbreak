# Core loop, movement, combat and cooperative play

## Highest-priority product rule: combat comes first

**Owner decision (2026-11-17): the goal is to kill enemies, repeatedly, in spectacular and inventive ways.** Moment-to-moment combat must be compelling with a *basic* takedown and even more satisfying when players exploit geometry, destruction and duo synergies. Traversal gates and switching are important but primarily serve combat opportunities, pacing and variety; do not make THE CRAWL into a primary puzzle platformer. Deliberate quiet zones and the early Dozer flight remain valid story beats. See [combat-first encounter design](combat-first-encounter-design.md).

## Target moment-to-moment loop

**Read threat → position / choose hero → strike, cast or alter terrain → cause a satisfying kill or meaningful knockback → chain the advantage → clear the threat → recover briefly → advance.** **Owner-approved rule:** there is **no requirement for two parallel routes or for every individual character to clear a room alone**. Some story-critical gates should explicitly require Donut's superior traversal, Carl's selective destruction, or a sequence of both. Solo switching makes this valid; co-op assigns distinct responsibilities. Additional routes are optional design opportunities, not a room-by-room quota. Later unlocks can recontextualize previously obvious solutions. See [asymmetric traversal and character-gated progression](asymmetric-traversal-and-gates.md).

## MVP controls — owner-approved design (2026-11-18)

The binding grammar below is **approved for the MVP**, not yet fully implemented or playtested. The authoritative mapping, story unlocks, technical migration, device coverage and acceptance gates are in the [MVP controller contract](mvp-controller-contract.md). That document supersedes earlier provisional mapping proposals.

| Action | DualSense | Xbox / Steam Controller | Meaning |
| --- | --- | --- | --- |
| Move / face | Left stick or D-pad | Left stick or D-pad | Basic locomotion and facing |
| Jump | Cross | A | Responsive variable-height jump |
| Primary attack | Square | X | Carl punch; pre-biscuit Donut weak swat; post-biscuit Magic Missile |
| Physical attack | Triangle | Y | **Carl kick, never an enemy grab**; Donut feline physical/pounce action |
| Equipped special | Circle | B | Earned special only; unavailable before unlock |
| Interact | R1 | RB | Doors, props, appropriate pickups, mechanisms |
| Swap controlled hero (solo) | L1 | LB | AI controls inactive hero; no authority stealing in 2P |
| Pause / inventory | Options | Menu / Start | Full controller support |
| Optional future precision aim | L2 + right stick | LT + right stick/trackpad | Not essential to complete combat |
| Optional future special cycle | R2 | RT | Only when more than one special can be equipped |

**Progressive complexity:** reuse these verbs as skills and equipment improve, instead of increasing button count or requiring memorized combo strings. Combat depth comes from physical contacts, enemy knockback and selective destruction. No mandatory sprint, grab, dodge, twin-stick aim, extra combat buttons or prematurely granted bombs/magic. The intended feel references Broforce's immediacy, Guacamelee! 2's physical contact depth **without grappling**, and Cyber Shadow's evolving action vocabulary; these are design inspirations, not source assets.

Input timing (auto-repeat, buffers, press/hold semantics), aerial variants, aim assistance, camera and per-character speed/damage are **playtest hypotheses**, not locked acceptance results.

## Baseline movement *tuning hypotheses*

- Fixed update: aim at 60 Hz for simulation; measure. Runtime target: consistent 60 fps on Steam Deck-class hardware, not an existing benchmark.
- Jump buffer ~100 ms; coyote time ~100 ms; adjustable after playtest.
- **Approved identity:** Donut jumps higher and reaches farther surfaces; Carl breaks and opens routes through selectively destructible obstacles. Tune actual numbers and input feel after prototype evidence. **Solo completion requires access to both protagonists through switching**, not independent completion by Carl alone or Donut alone. Same critical path works with 2P.
- Separate `DesiredAction` from device inputs; record/replay the former.

## Approved combat-feel rule (2026-11-17)

Normal enemies should generally go down in a small number of impactful hits; challenge comes from their patterns, pressure and the environment. **Combos emerge through movement, positioning, knockback, other enemies and selective destruction**, rather than long memorized button strings. Use hybrid deterministic gameplay physics and expressive Bevy presentation to enable predictable wall/prop/enemy impacts and eventual readable chains. **Exact attack inputs, collision contact damage, kill counts and launch values are experiment candidates**, not approved implementations. See [combat feel and hybrid enemy physics](combat-feel-hybrid-physics.md).

## Combat

- Carl: short-medium attacks, timed bomb/charge placement, traps; powerful against breakable surfaces, limited mobility.
- Donut: safer ranged harassment, displacement/slow/freeze type effects, expressive motion; limited direct structural damage.
- Mongo (later): physical charge with clear telegraph; disrupts tough enemies and brittle supports, not a permanent solution to all gates.
- Hit pause, small camera shake, knockback, particles and controller rumble are presentation events that must not alter core collision logic.
- Keep a small damage vocabulary: direct, explosion, fire, impact, environmental; only expand when tests justify it.

## Solo and co-op modes

**Owner-approved product model (2026-11-16):** **hybrid solo** combines direct control of either Carl or Donut, instant switching, companion AI, and compact contextual commands; **local two-player co-op is available from the very beginning**, with P1 Carl and P2 Donut. Drop-in/drop-out and safe device-to-AI handoff are product targets requiring their own tests, not existing engine functionality. Character abilities must unlock in appropriate story order: Donut is playable *before* she gains magic. **Shoulder Mode remains an open experiment**, initially comparing transport with optional mounted attacks and simplified commands; do not implement or label it accepted without human trial. Mongo joins later as ally or mount / commandable ability; three-player co-op is **out of scope** until proved useful. See [Floor 01 gameplay/progression design](floor-01-gameplay-progression-v0.md) and [integrated scene playtest plan](../07-agents/floor-01-integrated-scene-playtest.md).

Camera constraints: both active heroes remain visible; soften split/zoom and avoid arbitrary teleportation. If players separate too far, provide gentle regroup/elastic boundaries and UI direction; do not force a split-screen prototype yet.

## Example synergies

- Donut freezes a brittle barricade; Carl blasts it with reduced resource cost (a design candidate, not lore canon).
- Carl destabilizes a beam; Donut displaces enemies into the collapse zone.
- Carl breaks a mandatory weak barrier in one room; Donut is the only character capable of reaching a subsequent elevated objective in another. The game does **not** need a simultaneous equivalent path for each character. A combined room can use a sequential handoff after both primitives have been taught.
- Later, Mongo's charge can trigger predetermined weak supports; characters must not duplicate one another's role.

## Difficulty, damage, and restart model

Use readable tells and short recovery windows. First prototype: low-friction checkpoint before the encounter and fast retry (target <3 s to regain control, measured in build). Friendly fire off by default, but terrain effects can still inconvenience allies without producing softlocks. No unlock tree until the moment-to-moment loop is validated.

## Accessibility baseline

Full controller navigation, subtitles for any speech, color-independent danger indicators, reduced screen shake, clear contrast, optional auto-aim assistance, adjustable particle density, and readable Steam Deck text sizing. Document actual behavior and tests; do not claim accessibility certification.
