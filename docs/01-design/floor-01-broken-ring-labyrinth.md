# FLOOR01-RINGS-01 — The Broken Ring (the concentric labyrinth and Meadow Lark approach)

**Status:** owner-requested **design proposal**, draft for review; NOT an implemented stage, approved final geometry or verified novel sequence. Original game-design adaptation using existing Floor 01 art direction, abilities, enemies and local 2P contracts. It is **a 2D side-scrolling combat corridor with an architectural illusion of circularity**, not a top-down maze, a procedural dungeon, a metroidvania, a second Ball of Swine ring boss or a new physics engine.

**Placement:** late Floor 01 after the Slime / Rot Sticker pockets and before the Meadow Lark survivor camp, and therefore before troglodyte territory / the Juicer's gym. The first brief camp view and transition are part of this document; the **inside of the camp** is owned by the [Quiet Setpieces PR #187](https://github.com/pando85/dungeon-floorbreak/pull/187), which should not be duplicated.

## 1. Creative promise

**Memory: "We were fighting across the inside of an enormous broken stone ring; I baited a goblin into the closing gate, then we climbed over the fallen section and saw actual people alive on the other side."**

This stage has to provide three feelings in order:
1. **Awe:** enormous geometrically recognizable masonry concentric rings, old gates and damaged arches dwarf the heroes at native pixel scale.
2. **Clever aggression:** the player spots a real environmental opportunity in the combat while moving mostly rightward and using only already available combat verbs.
3. **Relief/curiosity:** in the far distance, warm lanterns and human voices beyond a broken ring reveal the Meadow Lark camp; this is a reward for advancing, not a new 90-second quest.

**This is not a frustrating maze.** A first-time human may pause to read a gate but never wander in circles because two corridors look indistinguishable. The challenge comes from fights and changing lines, not path memorization.

**Pacing hypothesis:** 4.5–7 minutes for a first human passage incl. fights, substantially less when replayed. No timer or movement slowdown enforces that duration. Quiet camp occupancy follows separately in PR #187.

## 2. How concentric architecture can work in a true side-scroller

The atlas image is a **spatial montage**, not actual topology. In a side-view engine, show a huge **partially visible circular stone relief/gateway** in the parallax/background, behind two or three adjacent horizontal traversal bands. As the party ascends from the lower line to the upper shelf and returns, it sees *the same ring landmark from a different height*, so it **feels** like traversing a circular structure without a depth-axis switch or camera teleport.

- **Background layer:** a recognizable broken 250–400px apparent-diameter concentric arch study, with thick concentric borders, fractured carving, weathered chains, an **off-axis missing segment** and at most two stone color values beyond the baseline catacomb kit. Large-scale reference only, no mandatory collider.
- **Walkable foreground:** protected dry floor; one short 64px mid-level step; a distinct **128px high shelf only for Donut after Light on Your Feet** (optional combat vantage; must not be the only forward path). Each has real and visibly consistent collision.
- **Midground:** a single unambiguous **slow stone shutter** with authored collision, an open/closed state and a visible sweep footprint. No full 360-degree rotation of level geometry or new player camera axis.
- **Wayfinding:** three visual landmarks in the world, each unique: (i) a broken crowned keystone at the entry, (ii) a three-notch carved shield adjacent to shutter, (iii) a torn canvas + warm lantern from the survivor direction. No mandatory minimap, compass or arrows. Short leftward tactical movement is permitted, but story progress remains intuitively rightward.

**Production distinction vs the Ball of Swine:** this labyrinth has **one slow local shutter**, mostly stone/gold torchlight, no ultraviolet spectacle, rolling composite boss, concentric floor combat grooves or arena-wide moving gates. The Ball arena remains the first truly enormous moving-ring boss spectacle.

## 3. Three playable regions + camp reveal (initial authoring sketch)

**Initial hypothesis:** ~3200 world px total (200 × 16px tiles) connected in three short authored room sections or one level as existing loader conventions allow. Coordinates are local to this **new** labyrinth scene, not Floor 01's opening ENV-02 markers. The vertical footline and floor tiles must match actual current level-kit semantics. No claim that a continuous 3200px level already exists.

| Span | Name | Main route | Optional architecture/decision | Threat design |
| --- | --- | --- | --- | --- |
| **R0 x0–672** | The Crowned Breach | Straight ground across fractured arch and broken relief | A low ledge for a 2-second view into the huge ring, no special unlock | None at immediate spawn; 1 ordinary opportunist at x>400 optional if intro feels too long |
| **R1 x672–2048** | **The Shutter Court** | One wide ground lane plus an always-reachable short bypass that passes **around/above** shutter | A 64px reachable step and **optional Donut 128px upper shelf** (after actual boost); rejoin to ground without falling into a hazard | **Two ordinary goblins** in staggered entry; a third later **only if** first tests stay readable. One single-use/slow-gate environmental opportunity |
| **R2 x2048–2816** | The Split Passage | Short apparent fork: ground 0-rise route and 64px overhead route; **both rejoin within one native screen** | Carl can potentially break one existing material for a *shortcut*, not progression; Donut can scout from above; no long dead end | One short mixed encounter using **already shipped** enemy types (e.g. goblin + one earlier Slime/Rot Sticker when stable); prototype first with 2 common goblins only |
| **R3 x2816–3200** | Light Beyond Stone | Safe protected walk through the destroyed inner ring | A high **view-only** ledge shows camp lanterns; optional, not a hidden collectible | No required fight; checkpoint/clean handoff into PR #187 REST-B |

Main fight R1 must be easy to see and remember: the same enormous ring carving remains visible behind both the fighting floor and optional elevated path; shutter physically changes an attack angle and divides the enemy group. R2 is a brief variation, not a second prolonged gate puzzle. At R3 the enemy pressure disappears and the first **recognizable human-made** camp structures appear; a piece of cloth hangs over a carved ancient stone gate, announcing that real people live here.

### Ground route remains fully traversable

R1 shutter must not **simultaneously** close every forward route. In the first greybox it may block a **shortcut** or temporarily occlude a direct attack lane, but there is always a normal forward bypass reachable by **Carl's existing ~80px jump** without a magical P2-only trick. The optional Donut 128px ledge gives a positional advantage, not a mandatory gate. Neither hero must be able to traverse every optional high shelf, but both must reach the shared checkpoint.

Do not make characters walk through the background ring or an invisible "front/back" plane. No platform teleportation, ring-travel animation replacing actual movement, offscreen return to a duplicate corridor or looping scene graph.

## 4. The signature encounter: The Shutter Court

### Player-facing beats

| Beat | What the human sees | Meaningful choice | What must be honest |
| --- | --- | --- | --- |
| S0 — Read | Massive broken ring and one slowly sliding door. One goblin clearly visible beyond the door/at its edge | Choose direct floor attack or optional elevated line | Gate shadow, collision and protected route match drawn geometry |
| S1 — Contact | Two goblins approach from separated positions; ordinary combat begins | Punch/kick into each other or into the real gate, Donut shoots from meaningful same-height line | Every kick and source attribution remains from normal combat |
| S2 — Pressure | Shutter gives a **distinct warning** before closing part of the combat passage. One enemy may walk into its footprint | Move, jump onto 64px route, attack, bait / delay | No instant unseen crush, no offscreen triggered closing into unprepared P2 |
| S3 — "I made that happen" | A launched goblin collides with a moving shutter or its prepared flank; the enemy is briefly pinned/staggered as shutter reaches its committed position | Use recovery window to finish by Carl physical hit or Donut Magic Missile | The hit only occurs through a genuine swept collider impact. No global timed enemy death; no automatic kill |
| S4 — Changed line | Gate now blocks one projectile path while another is accessible | Flank, recover or ignore environmental route | Props alter real LoS; even direct melee route remains viable |
| S5 — Climb / reveal | After enemies, a short bypass reaches the next tier and warm light becomes visible | Move forward into the human-camp reveal | No extra fake wave, no mandatory reset/repeat gate puzzle |

**This is ONE distinctive encounter**, not six puzzles. A player can fight everyone normally, ignore the shutter and continue. Skilled players can deliberately exploit the gate but must never be forced to wait for an exact cycle to damage a goblin. One camera-safe entry telegraph is enough to learn the rule.

### Shutter MVP contract: simple and consistent

**Minimum states**: `Open` → `Warning` → `Closing` → `Closed` → `Opening` → `Open`. Implement only if a merged compatible authored-prop and swept collision interface exists or can be extended **narrowly**, preferably reusing BOIL-B/C shared hazard/prop event and validator ownership rather than adding a new general moving-platform physics system.

**Initial numeric hypotheses (data knobs, not approved balance):**

- Warning **36–48 fixed ticks** (~0.6–0.8s), with strong visible shadow/stone scraping and a distinct lower-edge sweep cue.
- Moving closure **24–36 fixed ticks**, stops at **an authored fixed position**. No following actors, no mid-cycle world positions chosen by combat AI.
- Open/closed dwell **90–150 fixed ticks** before a repeat only if human tests find the cycle satisfying; first prototype should support a *single manually configured close* rather than an infinite periodic puzzle.
- Moving gate collision **cannot inflict lethal damage to heroes in v1**. It may safely resolve with a bounded shove/stagger or pause closure on blocked player motion, depending on actual solver; assert that **no actor is squeezed into protected solid terrain**. If that cannot be made fair and deterministic, implement static `Open/Closed` variants first and **defer the moving collision**. Never use hidden invulnerability or instant teleport to "solve" bad geometry.
- Goblin impact: a genuinely launched body may stagger/pin **once per actual impact token**, after gate movement/overlap is calculated. Any wall/body impact HP damage follows the existing CF02 rules; do not invent a free death solely for the spectacle.
- Gate cannot seal checkpoint/exit, break floor supports, hide a lethal hazard, or trap one hero on the far side after co-op regroup.
- All mechanical state ticks at fixed 60 Hz, hashed and replayable. Camera/VFX/audio can accentuate the slam but cannot decide damage or collision.

**Important distinction:** the Shutter Court should not reuse the Boiler's "huge weight falls and immediately kills everyone" payoff. This is **a positional pin, interrupt and combat advantage** with a visually immense object, not another exploding factory.

### What enemies?

Prefer a controlled existing roster first: **two common goblins**, one held behind cover then entering after the player sees it. The original atlas does not assert goblins literally live in this labyrinth; this is a **gameplay adaptation to validate**, not a canonical assertion.

The R2 short mixed room may later use **one previously introduced hazard creature** as a knowledge check (one Rot Sticker *or* one Slime, not both), plus one ordinary enemy, only after those gameplay implementations actually exist. No new ring guardians, supernatural mimic, complicated pathfinding AI, permanent invisible ranged sniper, new loot creature or novel item.

## 5. Character and story loadout contracts

- **Donut** already has Guild Magic Missile, mana, base agile movement and **Light on Your Feet** from the fast-food refuge. Respect **DP-Q45** for raised shelf clearance: in any room where she can stand above a 42px elevation with the boost, the authored overhead lane ceiling must not exceed the specified 228px height band, and camera/HUD visibility must be verified. Her boosted ~128px reachable platform is an **optional** vantage/shortcut, with same-height projectile validity checked (straight Magic Missile does not home downward). No Puddle Jumper until **after Juicer**.
- **Carl** can use punch, kick, normal jump and his **already story-earned dynamite** if the merged K1 post-Hoarder grant remains authoritative. His standard jump reaches the 64px route; he can break a supported optional material shortcut only with actual approved attack/dynamite interactions. No Juicer gauntlet yet.
- No new button, grapple, forced dual simultaneous actions, Shoulder Mode, inventory, subscriber or sponsor mechanic, major reward or special movement ability. An optional asymmetric tactic is encouraged but cannot block the only path.
- **Solo controlled Carl, solo controlled Donut and local 2P** must all reach the same safe transition. AI companion mobility is independently owned; no scenario-specific teleport/follower brain implemented here.

**Co-op gate priority:** If one player is beyond the closing shutter and P2 is lagging, the shutter cannot permanently strand them; the always-accessible forward bypass and camera/spacing bounds must work under real two-pad input. Author threats so the second player sees the closing warning at the native resolution. Neither a late P2 nor a passive AI may be required to perform a timed rescue.

## 6. Intro and payoff: show people, not exposition

The authored ring architecture does the dramatic reveal: the wide broken center opens to **cloth, lanterns, salvage crates and an improvised camp** in the protected next scene. A faint human hammer knock (or other sound **originally authored** for the game) replaces the repeated metallic gate noise. Do not invent a new written quest, new NPC behavior system or heavy cutscene.

The next `REST-B` camp can use the already **owner-selected visual concept boards** for **Imani, Elle McGibbons, Brandon Andrews and Chris Andrews**, recorded in `docs/02-art/book-01-odette-meadow-lark-concept.md`. Those boards are **not production sprites**, and specific script/dialogue/equipment and interactions must be independently reviewed. Odette is **not** a physical Floor 01 survivor; do not place her in the camp.

The broken ring's exit and the camp entrance should share a visible marker (e.g. one torn cloth strip and warm practical lantern), preventing the emotional beat from feeling like a disconnected teleport. **Do not duplicate PR #187's internal camp gameplay**, only establish a clear handoff point and checkpoint.

## 7. Acceptance matrix: headless, replay and human gameplay

| ID | Scenario | Executable condition / human observation |
| --- | --- | --- |
| **RING-01** | First-time ground-only route | Always exits without backtracking, hidden puzzle or optional switch |
| **RING-02** | Large ring camera reveal | In real native 640×360/400 Bevy, repeated landmark visible at two heights; heroes and collision readable |
| **RING-03** | Shutter warning and closure | No damage/collider activation before true warning; actual swept contact, data-authored phase |
| **RING-04** | Actor caught at shutter | No softlock, no forced out-of-bounds movement, no unavoidable instant lethal hero crush |
| **RING-05** | Goblin launch into moving flank | Actual overlap creates one pin/stagger with correct source; no proximity-script death |
| **RING-06** | No props used / direct combat | Normal Carl/Donut combat succeeds; no mandatory environmental exploit |
| **RING-07** | Optional 64px Carl and 128px Donut shelves | True owned capabilities and hitbox/jump reach; optional Donut-only high line |
| **RING-08** | R2 apparent fork choices | Both routes rejoin within one screen, no dead end/hidden required pickup |
| **RING-09** | Shutter phase vs P2 separation | 1P either active hero and real 2P can reach protected exit, camera still shows threats |
| **RING-10** | Retry while gate closing / goblin pinned | Same authored reset, deterministic input/event/state hash, no damage-token duplicates |
| **RING-11** | Slime/Rot variant when available | Does not include unimplemented mechanics; verified enemy behavior or explicit deferred gate |
| **RING-12** | Camp transition | No enemy projectile/event leaks, quiet protected checkpoint, survivor identity consistent with #187 |
| **RING-13** | Real recorded human attempts | Playtrace captures normal, optional high route and gate-use victories; 1× screenshot and at-speed footage |
| **RING-14** | Anti-tedium scripted policy | Simple move-right/jump/attack player can reach camp without interacting with gate; detour/backtrack/time metrics reported |

**Autoplay truth:** current AUTOPLAY party roster may not cover these complex level scenes. Agent must implement **scenario-specific Rust fixed-input fixture probes** using the authoritative actual party sim where available, and explicitly label unresolved Playtrace/level transitions. Don't claim unsupported `autoplay evaluate` subcommands have tested this arena. Human playtest judges memory, clarity, pace, the surprising sight of survivors and whether the shutter adds fun.

**Human-review questions:** Is it immediately visually clear that this is a ring-based ancient structure? Does the player feel clever for exploiting the shutter? Did the optional fork feel like variation or like a delay? Did the first sight of camp genuinely change the emotion?

## 8. Phased implementation and shared-file ownership

1. **RING-A — Art/level topology greybox:** actual 2D ring background as a temporary authored geometry reference (not copied concept image), 3200px provisional route, safe 64px bypass and optional 128px Donut shelf; no moving gate yet. Run reachability/solo/2P checks.
2. **RING-B — Static gate + basic combat:** two goblins and one open/closed gate that genuinely changes collision and projectile LoS while the mandatory route remains connected; record direct-win and optional high-route plays. This is the **minimum viable encounter** if moving collider support is not available.
3. **RING-C — Real deterministic movement/impact** only after checking merged BOIL prop/hazard/terrain ownership and scene adapter. Gate warning, committed one-shot or periodic slow shutter movement, bounded physical pin; full reset and replay. No new general mechanical maze engine.
4. **RING-D — Feel/architecture:** reuse AREA-RINGS-01 art kit with approved Floor 01 brick tones, unique crown/gate/canvas landmarks and atmospheric audio. The ring must read at native scale, not just in a landscape painting.
5. **RING-E — REST-B boundary + QA:** align with the [quiet refuge PR #187](https://github.com/pando85/dungeon-floorbreak/pull/187) and #182 integration file queue, include safe protected transition, Playtrace, 2P real-pad evidence and independent design review.

**Do not start implementation against unmerged assumptions.** This PR is the *design concept and local-agent implementation handoff*, not an instruction to rewrite currently in-flight level, character, companion, BOIL or replay changes. If scene adapter/prop interfaces are blocked, make an honest **reachable static greybox** and report the blocker. The right-hand scene can remain independently testable until the integrated Floor 01 route is wired.

## 9. Related docs and precedence

- `docs/01-design/floor-01-gameplay-progression-v0.md` — characters, asymmetry, zone chronology.
- `docs/01-design/combat-first-encounter-design.md` — combat-first and authored interaction.
- `docs/01-design/asymmetric-traversal-and-gates.md` — no two-human mandatory gates; solo switching supported.
- `docs/02-art/floor-01-art-atlas-v1.md`, asset `08-circular-maze-survivor-camp.png` — visual direction **only**, not authored level geometry.
- `docs/02-art/book-01-odette-meadow-lark-concept.md` — approved **concept** boards for Imani, Elle, Brandon and Chris; no production sprite approval and Odette not in camp.
- `docs/03-tech/donut-prog-01-floor01-ability-progression.md` — Light on Your Feet before maze, no Puddle Jumper yet.
- `docs/07-agents/f01-playtest-integration-plan.md` (#182 merged) — scene/prop owners, story K1 dynamite-after-Hoarder, 1P/2P gates, and hot-file queue.
- [Quiet Setpieces design PR #187](https://github.com/pando85/dungeon-floorbreak/pull/187) — **camp interior** scope; this document only designs **the approach and reveal**.
- [Hazard gameplay PR #185](https://github.com/pando85/dungeon-floorbreak/pull/185) and [setpieces PR #178](https://github.com/pando85/dungeon-floorbreak/pull/178) — previous hazard species and enemy contracts.
- [Goblin Boiler PR #176](https://github.com/pando85/dungeon-floorbreak/pull/176) — future shared authored environmental collision; do not duplicate or claim it has already shipped.

**Not canon claim:** the exact shutter fight, ring geometry, survivor glimpse and enemy placement are original game-design adaptations. The atlas/source only grounds the presence of a ringed gated labyrinth and a nearby survivor camp in Floor 01.
