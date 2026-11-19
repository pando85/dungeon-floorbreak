# Floor 01 — Gameplay and progression bible (design v0.1)

**Status:** approved *controller grammar* (2026-11-18), solo/co-op target and traversal identity + otherwise **unverified design hypotheses**. The [MVP controller contract](mvp-controller-contract.md) is authoritative for buttons and action semantics; this floor progression document does not reassign inputs. This document is not a claim of implemented mechanics, character lore, combat balance, or player approval of the integrated scene.

**Creative director decisions (2026-11-16):**
1. **Hybrid solo controls (option C):** one directly controlled character, AI for the other, instant character switching and compact context actions are the intended design. Precisely how the AI, switching and commands work needs playtests.
2. **Local two-player cooperative mode from the very beginning:** Carl and Donut are both playable at the start of the game, not unlocked as playable characters after the Tutorial Guild. **Narrative power unlocks still occur in story order.**
3. **Shoulder Mode is promising brainstorming**, not approved behavior or a required ability. Its combat/aiming simplifications and mounted specials remain experiments, not implementations.
4. **Character-gated level progression is approved:** Donut's distinguishing traversal is jumping higher and reaching farther places; Carl's is breaking/selectively transforming obstacles and opening routes. A room **can require one of these characters for mandatory progression**, using solo switching or distinct 2P roles. Two parallel paths are **not** mandatory. Later earned abilities make solutions more layered and less immediately obvious; see [mandatory asymmetric-traversal rule](asymmetric-traversal-and-gates.md).

5. **Combat-first priority (2026-11-17):** killing enemies creatively and spectacularly is the **primary fun**. Traversal, switching, unique protagonist gates and selective destruction primarily create combat opportunities and variety, not a puzzle-platformer-first loop. Short quiet/story/escape beats remain valid; see [combat-first encounter design](combat-first-encounter-design.md).

**Source boundaries:** Existing [vision](vision.md), [core gameplay](gameplay.md), [character sheets](characters.md), [antihero identity](antihero-tone.md), [input/camera technical specification](../03-tech/input-camera.md), [Floor 01 concept atlas](../02-art/floor-01-art-atlas-v1.md), [production and rights rules](../04-production/rights-and-release.md). Story chronology and equipment unlocks must be checked against the applicable book chapter before being labelled canon. All mechanics below marked *proposal* are game adaptations, not canon claims.

**Long-term context (living, not an approval of future abilities):** check the [character progression horizon](character-progression-horizon.md) when changing protagonist moves, unlock pacing, encounter capabilities or floor-to-floor assumptions. The Floor 01 decisions documented here remain the local gameplay design authority, subject to explicit review and playtest.

**Act I combat-first playable route (draft, NOT owner-approved):** see [Act I encounter beats](floor-01-act-01-playable-combat-beats.md) for a room-by-room adaptation of the existing Dozer-to-Guild scene and proposed post-Guild Magic Missile / Bad Llamas encounters. Do not treat beat timing, enemy counts or new spaces as approved implementation targets.

## North star

**A scrappy, emotionally grounded 1980s–1990s action-film antihero survives alongside an increasingly capable cat.** The two roles are different, valuable, and complementary. Success comes from understanding threats and making smart use of physical impact, magic, selective destruction and the environment. A player should feel more competent by the end of Floor 1 even if stats change little.

Keep everything physically readable on the **approved native-scale sprites**: Carl 52 opaque virtual pixels, Donut 40 pixels; an eventual adult Mongo is 76 pixels, **not** a Floor 1 co-op player by default. Do not modify these sizes just to fit a two-player shot.

## 1. Co-op as a first-class mode

| Property | Approved target | Prototype / test question |
| --- | --- | --- |
| Solo | Direct control of either Carl or Donut; inactive actor assisted by AI | How often is switching useful vs annoying? How much contextual AI autonomy feels fair? |
| Local 2P | P1 = Carl, P2 = Donut, both from start | Can each solve interesting movement and attack problems before the tutorial, without spell unlocks? |
| Input ownership | Two independent controllers (gamepad + keyboard can be tested separately) | What happens when one controller disconnects, reconnects, or joins mid-room? |
| Shared camera | Shared 640×360 / 640×400 virtual view, no indiscriminate zoom shrinking actors | Can players disagree on directions without jitter, invisibility, or harsh forced teleport? |
| Rejoin/drop-out | Desired seamless transfer to companion AI | Can reassigning authority preserve player state, held inputs, collision and camera continuity? |
| Difficulty | Same authored content and core critical path in 1P and 2P | Does co-op add opportunities rather than trivialize or multiply health pools? |

**Default principle:** no mandatory **two-human** gate; some gates do require a **specific character**. All story-critical encounters remain completable by one human **switching** characters, and in 2P by each player fulfilling their role. Never bind character authority to a hardcoded physical device ID. Avoid premature online support, three-player co-op, or split-screen.

### Character fantasy and starting availability (design proposals)

- **Carl:** physically committed, close-range, resourceful; some environmental manipulation/stronger objects, improvisation and later controlled explosives. Early attacks should show effort, not heroic mastery. No shotgun.
- **Donut, before her story unlocks:** playable, agile and comparatively fragile; basic jump, evasion, physical catlike attack and small-scale interactions are **proposals to prototype**, not asserted book abilities. Provide meaningful choices for P2 without granting magic early.
- **Donut, after the relevant story event:** ranged magic and later control/support become part of her available kit. Do not assign exact spell order, damage numbers, items or timings without source-backed chronology.
- **Mongo:** later milestone, not a third player slot during this experiment.

**Approved qualitative movement roles:** Donut can jump **higher and farther** and reach elevated/distant locations not accessible to Carl with his base traversal. Carl **destroys/changes obstacles** to open paths Donut cannot physically open. Exact numerical jump performance, speed, collisions, materials and stage-specific available break abilities remain tuning/design hypotheses. Some stages intentionally require one specific protagonist; a second equivalent path is not required.

## 2. Shoulder Mode — open experiment, not a settled feature

**Central question:** does mounting Donut on Carl unlock meaningful teamwork and better inputs while preserving a reason to dismount?

| Variant | Mounted behavior to simulate | Benefits to validate | Main risks |
| --- | --- | --- | --- |
| **S0 baseline** | Both independent; no mounting | Establish ordinary camera/coop costs | Not a shoulder test |
| **S1 transport** | Donut mounts and dismounts voluntarily; Carl drives movement, Donut retains meaningful interactions / choice of exit | Regrouping, fast traversal, accessibility, reduced camera friction | P2 loses agency and becomes a passenger |
| **S2 mobile attack platform** | S1 + Donut can aim/cast only abilities **already unlocked** while mounted; in solo, contextual aim/command candidate | Simple duo targeting, asymmetric roles, enjoyable co-op | Dominant strategy: never dismount, or too many buttons |
| **S3 exclusive combo move** | Later optional special with distinct trigger/cooldown, entirely hypothetical | A reward for timing and team play | Power creep, canon conflicts, animation/input complexity |

**Sequence:** prove S0 -> S1 -> S2, only prototype S3 if S2 already enriches choice. Mounting is not required to finish a critical path. Pre-Tutorial Donut has no magically unlocked shoulder attack.

**Open implementation decisions:**
- Who initiates mounting/dismounting in 2P? Is consent needed from both? Can either cancel safely?
- Do Carl's hits interrupt Donut's aim/cast? Can P2 still participate during basic traversal?
- Are aim vectors relative to Carl's facing or Donut's independent right-stick direction? Can a single-button contextual attack replace precision aiming for solo players without removing player choice?
- What mount anchor, hurtboxes and knockback applies at approved 52/40 px sizes? Does being mounted merge vulnerabilities or simply positions?
- What happens if Carl jumps, takes a hit, reaches a ladder, drops through platforms or loses input?
- Does mounted casting supersede movement puzzles, make enemies trivial, or produce inaccessible/hidden telegraphs?
- Is a modest visual emphasis (small mounted silhouette and clear state HUD) enough, without new final art too early?

**Keep options reversible**: data-driven mount rules, feature flag, independent actor simulation, separate authority/attachment state, headless deterministic tests. Do not implement before experiments or call it owner-approved.

## 3. Floor 01 progression: increasingly inventive combat and capabilities rather than stat inflation

The following is an **adaptation/design sequence** anchored to first-floor area concepts, **not** a fixed book map or exact canonical ability chronology.

| Beat | Player-learning purpose | Carl's likely role | Donut's likely role | Synergy to test |
| --- | --- | --- | --- | --- |
| Early corridors / Murder Dozer escape | Run, jump, learn when not to fight; coordinate 1P/2P | Physical traversal, improvised obstacles | Fast escape, avoiding danger without magic | Rejoin without forced waiting |
| Tutorial Guild | Establish rules, story-specific unlock(s), presentation of new choices | Understand survival tools | New agency and later magic **when earned** | First true asymmetrical interaction |
| Bad Llamas district | Read slow projectile tells and hazardous floor/space | Close approach vs safe positioning | Flanking and ranged opportunities after unlock | Threat diversion, careful regrouping |
| Scatterers / Hoarder | Crowd pressure and clutter | Physical control and selective breakage | Targeting or evasion support | Use chokepoints and altered cover |
| Goblin workshop | Mechanics, traps and unintended chain reactions | Environmental improvisation | Precision timing and safe positioning | Trigger vs exploit sequence |
| Slimes / Rot Stickers | Contact risk, wall/ceiling threats, explosive hazards | Avoid unsuitable melee; set up terrain | Sightline or safe ranged approach | Turn environment/enemy behavior into advantage |
| Circular labyrinth / survivors | Navigation, parallel roles, empathy under pressure | Open/hold routes, protect | Scout/assist within reachable shared-camera bounds | Distinct roles without mandatory 2P gate |
| Juicer's gym | Overwhelming physical opponent | Utilize props, timing and openings | Reposition/control and protection | Combine roles rather than overpower the boss |
| Ball of Swine | Synthesize learned spatial rules under moving geometry | Timing and commitment | Precision and survival support | Split/rejoin choices, no camera/terrain softlock |

**Never assume combat mechanics listed here are literal novel events.** During actual enemy design, cite book chapters for anatomy, behavior and unlocks, and label original gameplay adaptations. Bosses and character abilities are separate acceptance tracks.

### Ability-unlock rule

Story unlocks should create **new decisions** instead of only increasing damage. A formerly straightforward Donut-only jump or Carl-only barrier can later have less intuitive alternate solutions (earned spells, changed terrain, emergent interaction), but the first instances should teach each ability clearly. **Switching is a central traversal input and legitimate solution**, not a fallback for a flawed room; consult [asymmetric traversal/gate authoring rule](asymmetric-traversal-and-gates.md). Candidate categories (all subject to canon review and prototyping):
- Traversal/positioning;
- Physical contact/evasion;
- Ranged/assisted aim;
- Area/tempo control;
- Selective environmental destruction;
- Cooperative interaction / switching / commands.

Prefer two reliable contextually distinct uses to ten weak upgrades. Do not rush RPG class selection or later-book abilities into Floor 1.

## 4. Enemy-design contract for later bestiary

Each enemy entry should eventually document:
- **Source status:** book-grounded look/behavior with chapter reference vs game-only adaptation.
- **Readable role:** threat silhouette and telegraph, locomotion and movement axis.
- **Counterplay:** a Carl approach, a Donut approach, and a valid solo route.
- **Team interaction:** one potential meaningful timing/positioning choice, including optional mounted alternative *only if proven*.
- **Environment:** which materials/props/hazards matter, what persists and what cannot be destroyed.
- **Fairness:** damage cue independent of color, minimum recovery, difficulty pressure for 1P vs 2P.
- **Robustness:** reachable exit, deterministic RNG / state replay, no terrain-softlock.
- **Performance/art:** source pixels, animation budget, FX legibility at 640×360 and 640×400.

Candidate first evaluation enemies: ordinary goblin, Murder Dozer, Bad Llama. Bosses should not be implemented from this table alone.

## 5. Decisions required after experiments

**Approved:** hybrid solo architecture as target, simultaneous playable Carl + Donut from game start, local 2P as core product design, same essential content in solo and 2P, **explicit one-character gates without symmetric routes**, Donut's higher/farther movement vs Carl's physical/destructive access, story-consistent unlocks, cast native scale, shared camera intent.

**Unresolved:** action timing/feel and advanced binding variants (the base mapping is approved in [MVP controller contract](mvp-controller-contract.md)), whether shoulder exists, mounted attack behavior, co-op revival/death system, difficulty adjustment, solo AI autonomy/command UX, fine-grained character acceleration, targeting assist strength, camera split/regroup rules, friendly-fire consequences, and exact progression unlock order.

The test sequence and experiment evidence requirements live in [Floor 01 integrated-scene playtest plan](../07-agents/floor-01-integrated-scene-playtest.md). **Do not count automatic CI success as human game-feel approval.**

## Targeted pre-magic two-player experiments

The owner has playtested the integrated lit corridor and judged it visually good but not yet memorable. The **next game-feel hypothesis** is whether Donut can be rewarding to control before her first magic ability, and whether cooperating with Carl is better than simply following him. See [pre-magic Donut experimental plan](floor-01-pre-magic-donut-experiments.md) for a deliberately small catlike action set, three neutral greybox micro-scenarios, and a C0/C1 transport-only Shoulder Mode comparison (C2 mounted casting **only after** relevant story unlocks). None of these mechanics are currently implemented or approved as final.

## 6. Next design iteration

After the first human baseline review, use the evidence to pick one small Rust + Bevy prototype (likely second controllable Donut and camera/ownership) before designing detailed mounted combat or skill trees. The gameplay designer owns the test questions and the human creative director owns the final feel decision; local AI agents implement the selected experiments.

**No new gameplay behavior is implemented by this design document.**
