# F01-SETPIECE-HOARDER — "The Mountain Breathes"

**Status:** owner-favored memorable encounter concept; **level/boss mechanics and exact timings are hypotheses** for local implementation and human review. This is a standalone proposal, not a claim that the existing boss, environment or sound is playable.  
**Placement:** Floor 01 / proposed Stage 04, **after** Scatterer introduction and **before** Carl's dynamite narrative unlock. Donut has Magic Missile and her existing pounce/mana; post-biscuit auto-dodge is being implemented independently. Do not use future special abilities, including Puddle Jumper, as encounter prerequisites.  
**Creative premise:** a rubbish mound that appears inert starts moving. An uncomfortable encounter about a transformed **human woman**, not a generic goblin, toy explosion or comedy enemy.

## 1. One-minute player fantasy

You leave relatively open corridors for a quiet, junk-choked pocket. A wall of discarded objects seems to block the far lane. The pile shifts before the foe is seen; a partly exposed arm or silhouette warns that something lives inside. The Hoarder emerges and **her own attacks reshape the junk into navigable and dangerous cover**. The player can survive through direct physical/magical combat or deliberately use the unstable environment to create an opening. A final collapse feels earned because the player baited, damaged or positioned correctly, not because a cutscene auto-wins.

Tone: dread and uncertainty; reduced bright VFX, no comedic hit counter or repeated scripted ragdolls. Moment of aftermath is quiet, brief, skippable on retries. Do not dehumanize the transformed-human identity or assert an invented biography as source canon.

## 2. Geometry (initial greybox hypothesis; not authoritative pixels)

An approximately **960–1152 px** scrolling room, with an unbroken left-to-right protected **floor route**, no pits under movable junk and a protected far-side exit. Coordinates/height below are approximate; derive native placement from the existing 16px floor/LDtk grammar, actual camera, hero physics and approved 52/40px masters. No room-scale camera zoom invention.

- **Entry/read pocket (x≈0–208):** one safe lane with a partial view of a shifting mound ahead. Checkpoint before the first danger.
- **Junk bank A (x≈288–464):** lower unstable pile: reads as cover and a movable/collapsible object; can narrow but never seal the floor lane. A short safe ledge above/behind it helps Donut reposition but is not an untouchable firing nest.
- **Central confrontation (x≈464–752):** floor combat zone sufficiently wide for committed Hoarder attacks, recovery and close-combat spacing; no mandatory precision platforming.
- **Junk bank B (x≈736–864):** upper hanging/stacked pile with a clearly vulnerable support; optional player-caused drop, with visible impact footprint and escape direction.
- **Exit/regroup (right ~96–160px):** protected, always reachable across all piles' states. No debris colliders lingering over the final corridor.

The room should feel claustrophobic through architecture, lighting and set dressing, **not** by shrinking actual dodge lanes until the encounter becomes unfair. No loose-physics simulation of hundreds of props; use two authored piles and bounded nonauthoritative fragments.

## 3. Encounter beats

| Beat | What happens | Player opportunity | Hard constraint |
| --- | --- | --- | --- |
| H0 — Wrong quiet | Visible junk, one small settling motion, environmental sound before hostile contact | Observe and move; optional elevated vantage | No surprise offscreen damage |
| H1 — Reveal | Hoarder silhouette emerges from one pile after the player approaches the readable arena | Read size, physical attack style and the clear space behind cover | Never auto-hit during a forced camera/dialog |
| H2 — Brutal contact | Simple committed swipe/shove and a clear recovery; moving pile A changes part of the lane | Hit Hoarder directly; interrupt only when a real vulnerability allows | No endless immunity or HP sponge as sole difficulty |
| H3 — Controlled disarray | Baited charge/push against bank A or authored support weakens bank B | Punish whiff, open flank, reposition via ledge | No mandatory precise swap or scripted enemy teleport |
| H4 — Chosen climax | Player can land direct finishing blows or cause bank B to collapse into a true footprint, creating an exposure/finisher window | A player-authored decisive moment | Victory never requires the pile hazard |
| H5 — Unsettling aftermath | Foe defeated; piles remain in their resolved states; no further combat pressure | Short breathing room and forward momentum | Rapid retry skips staging, no long unskippable scene |

The narrative acquisition of Carl's dynamite occurs **after** this beat if chronology supports it; this encounter is not a dynamite tutorial.

## 4. Narrow prop/combat mechanics to prototype

### J-A: shuddering lower pile

`stable` → `disturbed` → `slid` → `settled`. A genuine Hoarder shove/charge contact or supported physical impact changes state; `disturbed` has a clearly readable **18–30 tick** warning before a bounded **horizontal slide** in one authored direction. Collision may push/stagger actors once per motion token, but cannot trap players behind a full-height impassable wall. The pile is reusable as temporary cover until it slides; ordinary cover occlusion must be real (sight/projectiles), not a cosmetic fake.

### J-B: hanging unstable stack

`supported` → `weakened` → `released` → `falling` → `landed`. Damage the visibly vulnerable support through **existing hit/collision rules**, or bait a real Hoarder heavy attack into it. Start with 2 qualifying distinct hits or 1 heavy impact, a **24-tick** warning and exactly one swept landing footprint. These numbers are candidate knobs. If the supported pile is not actually breakable by current merged combat primitives, first deliver the simple heavy-attack-only collapse as a labelled prototype; do not invent a new essential control.

The falling stack may **stagger/expose** Hoarder and hurt ordinary enemies. The boss must not be instantly eliminated by a single unavoidable environmental scripted strike. Tune proportional effect against normal direct-damage path during actual playtest. One outcome per actual overlap at impact, never aura damage per frame.

### Hoarder minimum gameplay behaviors (separate boss-enemy implementation gate)

A very small readable vocabulary is preferable: **close swipe**, **committed displacement attack**, **clear punish recovery**. One optional more dangerous action may be introduced only after real playtest. Her hitboxes/collision and attack timing require independent review; owner approval of art concept is **not** gameplay approval. No fabricated summoning or generic sludge projectiles. Preserve source identity and final art selection review.

## 5. Three valid ways to win

1. **Direct fighter:** use melee, Magic Missile and the actual existing movement/combat system to defeat her; piles can remain intact.
2. **Terrain tactician:** deliberately bait an attack into bank A to rearrange cover and create an opening.
3. **Collapse finisher:** position her under J-B and release the real weight on time, exploiting the subsequent exposure. This is an optional skilled play, not a mandatory puzzle or free cinematic.

The encounter is unsatisfactory if every playthrough produces the collapse automatically, or if an ordinary direct defeat is too slow or frustrating.

## 6. Readability and two-character fairness

- Two native-resolution hero silhouettes must remain visible against clutter; cover never masks character hurtboxes or advance warnings.
- Donut has useful escape/vantage or Magic Missile angles; Carl can close and exploit stagger. No forced Donut-only elevated gate; no Puddle Jumper, pre-Hoarder dynamite or special command dependence.
- Solo Carl, solo Donut and genuine 2P have identical authored victory/exit rules. Do not rewrite AI companion movement in this encounter specification; treat currently unmerged companion changes as dependencies.
- Post-biscuit auto-dodge/reflect is owned by a different PR. Attacks/hazards must expose explicit classification metadata; do not implement, duplicate, or tune the dodge mechanic here.
- Screen brightness and atmospheric effects are presentation only; critical tells use silhouette, movement and sound rather than visibility suppression.

## 7. Replayable acceptance cases

| ID | Scenario | Required evidence |
| --- | --- | --- |
| HOARD-01 | Enter with effects muted | Reveal and attack tells readable in native in-engine view |
| HOARD-02 | Direct clear, no pile touched | Boss defeat and exit without environmental trick |
| HOARD-03 | Bait shove into bank A | Real contact causes one warning/slide, correct authority and attribution |
| HOARD-04 | Attempt unsupported hit and real supported hit on J-B | No premature release; exact once-only drop |
| HOARD-05 | Drop beside/onto boss | Collision and exposed state only on overlap at actual landing tick |
| HOARD-06 | Destruction in both available orderings | No trapped heroes, protected exit, bounded events |
| HOARD-07 | Restart during reveal, slide and fall | Deterministic seed/reset and replay state hashes |
| HOARD-08 | Solo both hero selections and local 2P | Combat and exit reachable, useful role for each, shared camera readable |
| HOARD-09 | Playtrace verification and three human runs | Repeatable direct/terrain/collapse approaches with honest "fun" notes |

Fixed seed/tick states, deterministic material/prop event order and cause attribution are mandatory. Record all real development playtests by the existing PLAYTRACE contract. Completion by automated bot never substitutes for a human verdict on dread, pacing and fair warning.

## 8. Dependencies / production boundary

**Prior art:** `docs/02-art/floor-01-late-enemy-boss-concepts.md` (Hoarder is transformed woman; final production master not selected), `docs/02-art/floor-01-art-atlas-v1.md`, `docs/01-design/floor-01-gameplay-progression-v0.md`, `docs/01-design/combat-first-encounter-design.md`. Validate the first-book source chronology in the actual project material before claiming book accuracy.

The local agent should: (1) audit actual production Hoarder art/AI availability, (2) implement a simple room and direct boss fight, (3) add authored pile states and one physical cause, (4) add optional collapse and evidence, (5) tune with humans. Art and boss animation production may require a **separate** work stream, and neither the open boiler PR nor companion mobility PR may be overwritten. Do not claim this design PR already implements the encounter.
