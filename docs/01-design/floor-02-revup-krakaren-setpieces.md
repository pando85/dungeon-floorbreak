# F02-RK-01 — Rev-Up and Krakaren: playable two-stage combat setpiece

**Status:** detailed design proposal only. No implementation, tested balance or owner game-feel acceptance. This refines S02 and S03 of the [Floor 02 bible](floor-02-last-lesson-design-bible.md). Follow AGENTS.md, existing combat-first/physical contracts, accepted MVP controller and existing Floor 01 boss/prop work orders.

## Casting and arts continuity

The **foreman/pitchman is a combat role**, not the proposed name of a new canon species. First visual/acting choice is the lore-supported **Laminak Rev-Up Consultant Elite** (a fairy superior to the Clurichaun reps) with the existing alarm/interruption mechanic. Preserve one supervisory slot and one reinforcement budget. [Enemy acting bible](floor-02-enemy-acting-bible.md) specifies silhouette and attack poses. If the real primary-book depiction conflicts, update its visual, not the functional level contract.

## 1. The emotional and mechanical pitch

The wrong version is "Donut stands still, fires Magic Missile at a barrel, the entire room explodes for free." The right version is a lying sales expo that turns into a physical domino machine **because of a player decision**, then a grotesque tentacle boss that tears the facade down. Combat is satisfying before any clever prop trick.

**The player should say:** "I kicked a sales rep into a rack, the jugs rolled under the billboard, and the monster ripped the stage to pieces."

The clone is rooted to the arena: an immobile threat that commands huge *attack zones*, not a jumping, teleporting animal. Combat and story adapt the book's use of a revived clurichaun and explosives; our critical-path victory remains possible with earned physical and magic attacks. Any Second Chance summoned-actor feature is separate and opt-in, not a prerequisite.

## 2. Spatial blueprint (initial greybox hypotheses, not constants)

**S02 Rev-Up:** approximately **2240 world px, 3.5 native viewports**, a continuous flat primary walking line, two optional elevated flank shelves, three camera-safe sequential pockets. Use the current real scene walking line instead of inventing a global y coordinate. Preserve accepted Carl/Donut native sprite size and jump envelopes.

| Pocket | Provisional x range | Visual silhouette | Fight |
| --- | --- | --- | --- |
| A — The Pitch | 0–544 | broken podium, crooked sales arrow, receptionist slingshot | 1 slingshot rep + 1 low-health runner, staggered activation |
| B — Demonstration | 544–1504 | enormous triangular jug rack, crooked golden Rev-Up sign | rep + runner + foreman; optional environmental chain |
| C — The Warranty | 1504–2240 | ruined booth, tentacle shadow behind concrete | 2-foe direct brawl + exit/checkpoint |

**S03 Krakaren Clone:** approximately **1536 world px**, arrival (0–320), centered boss fight (320–1216), checkpoint and finite grub aftermath (1216–1536). At most **two simultaneously relevant height bands**. Carl has a continuous ground route to the core during recovery, Donut a short accessible alternate firing angle. The balcony is never a permanent invulnerable turret.

Do not change shared camera zoom policies to solve poor layout. Keep both heroes, attack lanes and core readable at 640x360 and 640x400.

## 3. S02 choreography

### Beat A — The Pitch

The player sees the first slingshot rep before it fires. Provisional **24–36 ticks** of distinct arm windup before a committed aimed projectile; no post-fire magical homing. The runner approaches on a different beat, never instant simultaneous offscreen attacks. Carl can kick him into a queue post, Donut can pounce, shoot or flank from the same main route. **Direct victory without any explosive prop** should already feel good.

### Beat B — The Demonstration

The main enemy roster initially caps at **three simultaneous attackers**. The foreman starts a very visible alarm: provisional **48–72 ticks** to respond, interruptible by *landed* physical contact, pounce, real magic hit or blast. Completion can bring at most **one finite reinforcement, once per attempt**. No endless waves or large HP.

Three equally authored routes:
- **Direct:** attack everyone with familiar verbs; finish without touching the sales apparatus.
- **Opportunistic:** kick an enemy into one jug, causing real displacement, then ignite with a legitimate attack / source. The blast adds space but does not cleanly auto-win.
- **Showpiece:** a launched enemy loosens a rack; two jugs roll and one is ignited by a second committed action; a bounded chain damages the billboard support, producing a telegraphed falling mass that may crush a surviving enemy. No fake body-collision damage; source and event graph are authoritative.

**Donut anti-camping:** fixed-seed policies stationary_magic, mobile_magic, carl_direct and solo_companion should be evaluated. A permanently stationary Donut cannot safely defeat every wave just by firing, but achieve this with moving flank pressure and readable slingshots, **not** spell resistance, added HP, blind homing or mandatory Carl switch. Keep her mobile play strong.

**Party safety:** each armed jug has a visible danger envelope that gives both heroes time to move, including when caused by the other human. The live blast follows existing hero damage policy, not new friendly-fire rules. Broken props cannot eliminate the sole ground route.

### Beat C — The Warranty

A short, familiar two-foe fight; a huge tentacle silhouette moves behind the concrete wall but deals zero damage here. It is a story tease, not an untelegraphed new hazard. The actual boss is revealed in S03, and the player can restart there without replaying S02.

## 4. Three prop contracts

Prefer extending actual Floor 01 env_prop, BlastCharge and encounter state, **not** creating another physics or damage engine.

### P1 — Sales jug

State machine:
- Idle -> Dislodged: real strike, sufficiently energetic launched-body impact, authorized blast or rack release.
- Dislodged -> Armed: **valid ignition only**, such as story-earned explosive or an active world fire effect. A normal Magic Missile can dislodge it, but must **not** count as ignition unless its actual authoritative damage profile explicitly includes ignition.
- Armed -> Detonated: candidate **36–60 tick** visible fuse; single real blast and one authoritative causality event.
- Dislodged -> Settled: no detonation until legitimately ignited; restorable by checkpoint.

Reuse the existing maximum chain depth (**4** documented for Floor 01) and deduplication. Provisional **two concurrent active warning envelopes** in the first pilot; ensure no overlap creates unavoidable damage.

### P2 — Rack

Intact -> Loose on real source damage -> Fallen with **at most two** authored jugs, using stable IDs (not dynamic hidden unbounded spawns). Destructible but never a protected critical path. Magic Missile can damage it but cannot ignite every bottle automatically.

### P3 — Falling sales sign

Hanging -> Cracking -> Falling -> Landed. It needs real support impact, a swept fall volume, an early tell, one landing and real collision-only damage. The sign can strike a foe once on landing and never repeatedly on resting overlap. Check its footprint against both heroes and the exit.

The spectacular chain must be optional, not an enemy-HP workaround or a scripted cutscene. Score and viewers reward attributable *outcomes* rather than simply breaking props.

## 5. Clurichaun combat contracts

**Sales rep (ranged):** Approach -> Aim -> CommittedShot -> Recover -> Reposition. Aim captures the target before firing; real hit-stop/knockback can interrupt before commitment. Shots obey camera-safe arrival and cover. No unreachable sniper spot.

**Foreman:** Approach -> SignalWindup -> AlarmActive -> Recover. Real landed attack interrupts, never merely a button input. One backup enemy maximum, cannot create a permanent player-ambush loop. Not immune to Carl kicks or Donut attacks.

**Autonomous companion:** may attack, flank and move away from fuse; never decides to trigger an environmental chain independently merely because the visual prop exists. Track companion-caused shots and actual hits in replay. No Carl-only enemy targeting.

## 6. S03 Krakaren Clone: three attacks, one vulnerable core

A **rooted core** is visible early and remains reachable by both protagonists during recovery. Ambient art tentacles cannot deal incidental contact damage; only explicit authored attack volumes are authoritative.

Boss state flow: Presentation (skippable on retry) -> SelectCommittedAttack -> Windup -> Active -> Recover (core exposed) -> Select... -> Staggered on legitimate strong hit/prop impact -> Defeated -> finite grub transition.

**Distinct attacks:**
1. **Center slam:** one tentacle commits to a center lane; readable pre-impact shadow; provisional 54–90 tick exposed core on recovery. Avoid a surprise full-view hit.
2. **Directed sweep:** left-to-right or right-to-left, visually distinct; a reachable low or aerial answer. Never every lane at once.
3. **Pull zone:** a bounded suction tell and spatial response with a real escape route; no forced QTE, no invisibly disabled controls.

Candidate escalating behavior: slightly tighter recovery or one additional environmental threat after a readable phase boundary. Do not simply add boss HP. Keep one telegraphed *active attack* at a time in first pilot.

**Direct win:** Carl physical melee, Donut Magic Missile or physical close-range can reduce core health through repeated recovery. Test both hero-leading policies without touching props.

**Environmental win:** a player-triggered legitimate jug/sign hazard reaches the boss core, provides a capped bonus damage/stagger and changes geometry. No free scripted kill after standing at one precise coordinate. The optional book-inspired revived-clurichaun route remains a separate feature flag requiring summoned actor/control design, not a hidden requirement.

The defeat triggers a **finite** visible cleanup-swarm burst, not thousands of simulated colliders, and a safe post-boss checkpoint.

## 7. Event and replay causality (candidate schema, reuse existing names)

Foreman alarm start / interruption; jug dislodged / armed / detonated with source ID; rack released with stable child IDs; sign landed with target/cause; boss attack committed with lane/tick; core recovery exposed until tick; clone defeated with cause; grub clusters spawned capped by budget.

For each: distinguish simulation-hashed state from presentation-only, keep a stable root-action attribution across chains, ensure retry resets without duplicate score/unlocks. Never claim spectacle based on a VFX animation alone.

## 8. Acceptance matrix for the implementing agent

| ID | Required proof |
| --- | --- |
| RK-01 | S02 direct clear with all props unused, Carl-leading |
| RK-02 | S02 direct clear with Donut-leading and legitimate AI Carl behavior |
| RK-03 | Carl launched body dislodges rack/jug only on actual contact |
| RK-04 | plain Magic Missile does not ignite jug without actual ignition damage type |
| RK-05 | two-stage authored physical chain: bounded warning, exactly-once effects |
| RK-06 | foreman can be interrupted; uninterrrupted alarm creates max one helper |
| RK-07 | stationary Donut diagnostic vs mobile Donut, same seeds, no HP/magic-immunity cheating |
| RK-08 | two heroes can flee both blast and falling-sign warnings; valid exit after destruction |
| RK-09 | Krakaren direct-only Carl and Donut clears independently; no exploit balcony |
| RK-10 | every boss attack has distinct tell, committed lane and genuine recovery window |
| RK-11 | props optional for boss, but real prop impact matters; no fake kill |
| RK-12 | retry bit-for-bit fixed-seed authoritative events, plus PLAYTRACE strict where supported |
| RK-13 | real game-speed Bevy capture 640x360 and 640x400 for direct and spectacle routes |
| RK-14 | genuine two-pad, hardware feel, human fun verdict: NOT RUN until owner tests |

**Implementation queue:** pure foreman/ranged logic -> existing env-prop adaptation -> S02 greybox and all three routes -> pure rooted boss -> S03 in engine -> original native pixel keys/masters -> strict Replay evidence and independent review. One shared-hot-file owner; never rewrite Floor 01 core to make S02 pass without an approved baseline comparison.

**Source boundary:** Rev-Up, the immobile Krakaren clone and the book's revived bomb-carrier are broadly grounded in secondary references including https://www.dccdatabase.com/wiki/krakaren_clone_%28second_floor%29/ and https://www.dungeoncrawlercarl.nl/en/news/who-is-krakaren/ . The physical jug pyramid, sales-bell foreman, scenario shapes, attack timings, two direct fight paths and room coordinates are gameplay adaptations, **not** claims about book text.
