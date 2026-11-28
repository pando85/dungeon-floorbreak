# F02-FOUNDATIONS-01 — The Wrong Color, Customer Complaints, The Cleanup Department

**Status:** authored combat/level greybox proposals only, 2026-11-22. This specification fills the three remaining encounters S00/S01/S04 in the [Floor 02 bible](floor-02-last-lesson-design-bible.md), and must be read with the [enemy acting bible](floor-02-enemy-acting-bible.md). No newly playable room, approved geometry, asset or owner game-feel test is claimed.

## Creative goal

Floor 02 must introduce a **new place with familiar actions**. Avoid a 10-minute sequence of walls and text explaining new mechanics. These three beats must create two kinds of gameplay contrast:
- S00/S01: clinical silence and an unfair-looking *mind attack* that turns out to have a simple, readable counter based on movement and interrupting.
- S04: the floor is alive with a cleanup economy. Defeating things creates ecology and opportunities, not an endless punishment loop; the player can turn industrial recycling into a weapon.

**Explicit anti-filler:** S00 may merge into S01 if the Floor 01 owner tests show players already understand all movement/cover interactions. S04 may become the *first section of the Meadow Lark chapter* with no extra stage-clear. These are encounter IDs, **not separate loading-screen levels**.

## S00 — THE WRONG COLOR

**World-width starting hypothesis:** ~832–1024px. Short controlled arrival with one enemy and one novel visual effect, not two tutorial rooms. Cold white cinderblock walls, orange cracks, concrete with an unusually crisp reflection, an arrow resembling a public utility wayfinding sign. First screen must look unlike all of Floor 01.

**Composition:**
- Left 0–240: dark exit frame from Floor 01 -> one hard architectural transition to pale floor. Background lichen enters the seams; no "health-draining orange sludge" inference.
- Center 240–680: lower continuous combat line, a small upper 32px ledge that Donut can exploit but does not require.
- Right 680–1024: metal safe-room recess / one regular hero regroup point.

**Beat by beat:**
1. Brief normal, player-controlled entry (no automatic walk-forward on retry). Fluorescent ballast hum and color shift are ambient, never an enemy telegraph.
2. A visible **legless janitor grub** investigates a corpse; a crawler can ignore it. It gives a visual glimpse of the cleanup ecology without a creature-stat tutorial.
3. One previously known weak combatant in an authored position pressures both heroes for a few seconds. The new Mind Horror silhouette is seen *farther right behind a wall*, initially noninteractive and non-damaging. Do not grant the Mind Horror projectile before its first warning has been introduced.
4. Carl's kick can crack a brittle facade tile at the side but **cannot dig out through a cinderblock corridor**. Cracks in inert wall art do not produce a collision hole; if a block can actually break it must have a separately authored unimportant prop ID and state.
5. Exit to S01 without a fanfare/panels. One checkpoint if stage loader requires it; regular retry is brief.

**Fail cases to prevent:** Floor 02 begins with a mandatory interact hint, five new monsters appearing simultaneously, a false destructible wall that softlocks players, an ultra-bright background hiding Carl, permanently safe Donut sniper shelf or arbitrary offscreen Mind Horror attack.

## S01 — CUSTOMER COMPLAINTS

**World-width hypothesis:** ~1600–1856px, three contiguous short pockets. This is the first new-mechanic combat level of Floor 02, but should teach by **consequence and anticipation**, not pop-up controls.

| Pocket | Approximate position | Landmark | Pressure/answer |
| --- | --- | --- | --- |
| C0 — The Queue | 0–35% | bent waist-high queue rails and ticket dispenser | 1 Mind Horror alone, obvious psychic telegraph and vulnerable recovery |
| C1 — The Window | 35–73% | broken customer-service counter and giant number display | Mind Horror + one common rushing/short-ranged foe, staggered tells; flank under pressure |
| C2 — No Refunds | 73–100% | falling waiting-number sign and damaged office furniture | two low-health existing opponents; contact-driven optional sign drop, straightforward direct combat |

### Mind Horror 1: visual teach in 3 seconds

Before Mind Horror commits its first psychic lane:
- It slows its hover and pulls tentacles inward.
- Floor or air projection shows **one clear directional danger wedge/beam**, visually distinct from a grenade, falling weight and llama projectile.
- The hero can use existing run/jump or the counter's reach/cover, depending on authored collision.
- During genuine recovery the main body droops to an attackable height. Carl's physical must be able to reach at least one such window; Donut can pounce or cast without a new input.

**Threat discipline:** no full-screen pupil distortion, inverted input, forced confusion state or simultaneous lethal beam that covers both ground and short ledge. Attack direction is **committed after warning**, not homing through a subsequent jump. The effect must be readable without HUD text. Use different AI target seeds and test Donut-leading solo.

### Mind Horror 2: combined threat, not twice the same thing

In C1, a single rushing foe closes the safe window while the Mind Horror signals a psychic lane. The player can respond by:
- moving into a reachable flank and shooting the Horror (Donut mobility route),
- interrupting/launching the rusher into a counter before going for the Horror (Carl impact route),
- punishing the Horror with direct physical attacks during its clear recovery.

There is **no mandatory Carl-vs-Donut keyed switch**. A good player may finish the wave quickly with ordinary abilities; positional timing makes standing still/spamming missiles less safe. No static invulnerable elevated mage or perma-stun psychic.

### Optional spectacle

A launched enemy may strike the huge waiting-number display, which **falls only on a real support collision**, damaging whoever it actually lands on. One impact, short red hazard envelope, proper source attribution and no auto chain to the next room. All kills can be done without breaking it. Avoid a redundant explosive prop here: save the complex domino escalation for S02 Rev-Up.

**Quiet transition:** a customer-service audio chime plays after the fight as the display goes dark. A compact, optional narrative cue can seed Donut's Second Chance without equipping it or using invented source dialogue. Never interrupt gameplay to explain menu/quests.

## S04 — THE CLEANUP DEPARTMENT

**World-width hypothesis:** ~1792–2048px, after defeated Krakaren and finite grub flood. The room should feel different from the goblin Boiler: **it has a moving direction**. A single main recycling conveyor carries debris and bodies from left toward a large, illuminated extractor at the right. This is not a new physic-based push-the-cart puzzle.

| Pocket | Role | Visual / player action |
| --- | --- | --- |
| D0 — The Aftermath | half-screen | trace of corpses and animated cluster silhouettes; player learns the cleanup hazard has limits |
| D1 — The Production Line | ~one viewport | real slow conveyor belt moves discrete props/corpses; a hanging bin responds to actual struck support |
| D2 — The Breeding Vent | ~one viewport | one pupa shell splits (presentation cue), at most one Brindled Vespa attacks from a visible aerial lane; player can retaliate |
| D3 — The Exit | half-screen | conveyor breaks/ceases into a safe area and warm lamps of Meadow Lark ahead, seamless transition to S05 |

### Bounded ecology algorithm (design contract, not actual code)

- Every defeat already emits an authoritative defeat/corpse event. Derive a **budgeted deterministic simulation of grub cluster actors**, not a fresh independent actor per corpse pixel.
- Proposed first pilot cap: **max 4 active simulation clusters** and **max 1 new cluster per 90 fixed ticks** (provisional; reconcile engine tick length and actual perf) with stable actor IDs based on corpse/room + spawn sequence. Visible background grubs may be particle/tiles, but do not interact with hitboxes.
- Stage-local finite budget; after reached, no new gameplay clusters. A death does not guarantee a spawn or add uncapped difficulty. A run with several stylish kills must never be paradoxically harder only because the player is effective.
- Grubs move toward actual accessible corpses or the conveyor; their collision contacts are at most weak positional pressure, not high-DPS hidden damage. Corpses are not a secret "burn now or die" timer. No corpse burning or special spell required.
- A pupa/vespa moment is authored and **finite**, not an emergent evolutionary simulation requiring dozens of grub-to-vespa upgrades. If lore/game science later justifies a systemic version, evaluate separately from Floor 02 completion.

### Conveyor and hanging bin

- Belt is the only new major world motion. It moves discrete short props and corpses through existing authored collision resolution. Living players may be moved by a modest **readable** belt impulse; jump always provides a reliable answer and no platform is a permanent trap.
- The belt may feed a hazard extractor at the far right; keep a separate protected walking route across it with a reachable jump that both heroes and solo AI can handle. Do **not** put the critical exit behind a device that must be shattered.
- A hanging bin is visibly heavy. A true Carl strike, launched enemy or earned bomb can break its support; it swings/falls with tell, collides and settles, offering **one** gratifying optional defeat. Donut may use a magic projectile on the support if it genuinely deals the needed damage; no physical-route exclusivity. The enemy is also fully killable by attacks.
- Collision of enemy into conveyor and bin must be actual, replay-hashed and attributed; no scripted kill when the prop is merely hit. Prevent resting overlap repeated damage.

### Vespa danger and Donut versus Carl

The first Vespa emerges after a pupa cue and ascends to a reachable band; distinct acid spit projects a **single aimed arc** with slow tell. During the spit recovery the Vespa descends sufficiently for Carl's appropriately placed punch/kick or his existing explosive; Donut can use superior vertical access and her earned Magic Missile. No permanent floating artillery outside Carl reach, and no cheap room clear by standing permanently on an upper safety platform.

At most one active Vespa in the first pilot. Pairing with one ordinary enemy or a small cluster is enough. Only add a second after proving game feel and actual performance.

## Progression, retry, companion and camera requirements

- S00-S01 feed S02 without an isolated victory interstitial. S04 feeds S05 without an extra panel. Stage IDs are internal for source data, checkpoint / E2E tests.
- No new ability is required to finish any of the three. Do not equip Donut's Second Chance or add permanent summoned minions to avoid a low-pressure room.
- Solo AI companion follows existing hazard feed and never camps a protected shelf, chases a grub offscreen or blocks the exit. Distinct player-lead runs show real offensive participation.
- Camera sees committed Horror direction, falling number display, active conveyor and incoming Vespa acid; prioritize combat and real hazard over cartographic panorama. Keep 640x360 and 640x400 valid.
- Playtrace records defeat, corpse cluster spawn/disappear, conveyor interactions, emitted enemy attack and bin impact, without treating background grub animation as a simulated actor.
- After checkpoint retry, restore corpse/grub and conveyor/bin state deterministically. No duplicate story grants, viewers, rewards or endless resource farming from re-clearing.
- Owner's Floor 01 human gameplay baseline precedes any shared collision/camera changes.

## Acceptance and executable task breakdown

| ID | Verification |
| --- | --- |
| FOUND-01 | Floor 01 -> S00 visuals and story capabilities, no future grant or seam softlock |
| FOUND-02 | S00 first screen visually different; one easy direct fight, 1x native source capture |
| FOUND-03 | S01 Mind Horror distinctive precommit psychic warning with a visible evade/counter |
| FOUND-04 | Carl and Donut each can hit the Horror during real recovery, no permanent unreachable float |
| FOUND-05 | Combined C1 threat demands position in stationary-magic policy without magic resistance/HP padding |
| FOUND-06 | Optional number-display physical fall, single landed collision and normal direct victory |
| FOUND-07 | S04 no more than proposed finite clusters; CPU bounded, actual deterministic corpse attribution |
| FOUND-08 | No weird spawn inside hero, safe room, blocked exit or killed-cluster event that creates more limitless spawns |
| FOUND-09 | Conveyor pushes props predictably; player and inactive AI have a safe ground/jump escape |
| FOUND-10 | Hanging bin really hits or misses based on collision; one landing damage, exact restart |
| FOUND-11 | Vespa attack visible and reachable by both heroes after recovery; cannot camp offscreen |
| FOUND-12 | S04 -> S05 transition reveals Meadow Lark warmth with player control intact |
| FOUND-13 | Fixed-seed strict replay for one direct route and one optional physics route at each chapter |
| FOUND-14 | Actual real two-pad, handheld readability and owner fun: NOT RUN until tested |

**Implementation order:** (1) S00 environment kit and scene route; (2) pure Horror finite state; (3) S01 first/second pockets + tests; (4) simple finite cluster state + perf; (5) pure conveyor/bin collider; (6) S04 + one Vespa pilot; (7) S04->S05 seamless transition. Prefer existing level data and named collision prop contracts; do not implement three new mechanics in a single hot-file PR.

**Source boundary:** pale floor and orange lichen, Mind Horrors as floating tentacled brains, cleanup grubs and Vespa are canon-inspired via secondary sources (https://www.dccfansite.com/entry/floor-two/ ; https://www.supersummary.com/dungeon-crawler-carl/part-2-chapters-28-36-summary/ ; https://dungeoncrawlercarl.wikioasis.org/wiki/Second_Floor). The service counter, giant number display, conveyor/bin industrial room, exact spawns and move patterns are original scene adaptations. Do not reproduce third-party art or text.
