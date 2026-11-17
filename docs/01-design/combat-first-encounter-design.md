# THE CRAWL — Combat-first encounter design (mandatory gameplay pillar)

**Status:** owner-approved product/level-design priority, 2026-11-17. This is design direction, **not** an implemented scene, balance approval or book-canon claim.

**One-line rule:** **The main player experience is killing enemies in spectacular, inventive and satisfying ways.** Movement, switching, asymmetric traversal, level architecture, selective destruction, enemy behaviors and eventually Shoulder Mode should *feed the combat fantasy*, not turn THE CRAWL into a primarily puzzle-platforming game.

**Precedence:** retain the [approved asymmetric-traversal mechanic](asymmetric-traversal-and-gates.md): Donut jumps higher/farther and Carl selectively destroys/opens routes; solo switching is a legitimate way to pass character-specific gates, and equivalent parallel routes are not required. **But character gates are seasoning, not the dominant encounter format.** Preserve [80s–90s action antihero tone](antihero-tone.md): Carl is an ordinary, scrappy survivor, not an invincible superhero.

## Design hierarchy

1. **Combat enjoyment and cathartic, memorable kills:** each action has convincing impact, clear response and lethal payoff when appropriate. Spectacle is the reward for good play and improvisation.
2. **Player-authored mayhem:** enemies, materials, props, hazards and geometry interact in ways players can anticipate, exploit and combine. A creative kill should often be more entertaining than a basic kill.
3. **Asymmetric protagonists and cooperation:** Carl applies physical force and terrain changes; Donut exploits superior traversal and later legitimately earned magic. In solo, rapid switching and companion commands provide the same strategic options; in 2P, both can contribute from the opening, even before magic.
4. **Readable, fair action:** responsive combat, visible telegraphs, recoverable mistakes, short restart time, legible screens at 640×360/640×400 and protected critical routes. Spectacle must not hide threats or undermine agency.
5. **Exploration, platforming and story beats:** teach tools, adjust pacing, create combat positions and set up surprising encounters; do not make mandatory levers/jumps/puzzles the primary activity.
6. **Narrative pacing and emotional contrast:** safe rooms, quiet corridors, helpless escape sequences and story moments are valuable exceptions. No requirement to kill an enemy in every room or to attach arbitrary enemy-count quotas.

### Core player loop — combat-first

**Spot a threat -> read its tell/terrain -> choose hero, position or terrain action -> strike / cast / detonate / manipulate -> witness expressive, mechanically meaningful consequences -> pursue the next threat or escape -> short recovery -> next encounter.**

This is an **action game with selective traversal puzzles**, not a puzzle-platformer with occasional enemies.

## Approved feel and physical-interaction direction (2026-11-17)

The owner approved **fast, heavy, low-HP-sponge combat; straightforward attacks plus spectacular environmental kills; emergent combos over long input memorization; and a hybrid deterministic simulation/presentation physics approach** for enemy knockback, impacts and potential chain reactions. Exact contact-damage rules, attack counts and collision responses remain test hypotheses. See [combat feel and hybrid enemy physics](combat-feel-hybrid-physics.md) and run the COMBAT-PHYS-01 staged tests before authoring a full combat catalogue.

## Three independent satisfaction tests

Every playable combat iteration should establish:

1. **Basic kill quality** — defeating one ordinary enemy with a straightforward Carl attack is fun even without a special gimmick. Evaluate hit timing, impact frames, knockback, hit-stop, sound, recovery, readable damage and death animation. Never compensate for weak hits by simply adding visual noise.
2. **Environmental kill quality** — breaking/dislodging one authored prop or material can kill, interrupt, pin or reposition an enemy, while preserving bounded and deterministic collision/destruction.
3. **Partner-created opportunity** — swapping or cooperating opens a *combat* advantage, not only a passage. Examples include Donut gaining a high firing vantage **after story-unlocked magic**, luring an enemy past Carl's improvised hazard, or Carl opening a high-risk flank. A pre-magic Donut may exploit superior jumping, positional distraction or a cat-credible physical interaction; do not grant spells early.

**A special kill should be readable as a causal chain.** Players should understand that *their decision* produced it: break beam -> beam falls -> goblin is crushed; bait a machine -> it overshoots -> enemies caught in a hazard. Avoid non-interactive spectacle or kills caused by invisible systems.

## Encounter variety: fight first, terrain matters

An action room can use any of the following; these are **non-exclusive patterns** and not fixed frequencies.

| Pattern | Primary player pleasure | Role of traversal / destruction |
| --- | --- | --- |
| **BRAWL** | Fast, satisfying direct takedowns of a few ordinary enemies | Positioning and simple cover |
| **ENVIRONMENTAL_EXECUTION** | Author a spectacular, legible chain reaction or collapsing hazard | Terrain is a weapon |
| **VERTICAL_AMBUSH** | Attack from above, avoid ground pressure or displace enemies | Donut's higher/farther jump creates attack angles |
| **BREACH_AND_CLEAR** | Break a weak wall/prop to expose, flank or surprise opponents | Carl transforms geometry to win a fight |
| **DUO_COMBO** | Two character actions create a result neither easily achieves alone | Switch/2P collaboration is an offensive choice |
| **PRESSURE_ESCAPE** | Survive an overmatched hazard and gain a later payoff | Chase, improvisation and fleeing are deliberate pacing exceptions |
| **BOSS_ARENA** | Learn attack tells and win through risk, timing and environment | Mechanic combinations without hard-to-read clutter |
| **SHORT_REFUGE** | Relief, character acting and preparation | Resets tension before more action |

A **character-only gate** can still exist, but an ordinary stretch should not repeatedly stop the player for a mandatory lock-and-key interaction. The majority of memorable gameplay moments should emerge from fighting and enemies reacting to terrain.

## Character combat identities (proposal, not yet tuned)

**Carl — kill by force and by turning the place against you**
- Core melee must be visceral and responsive, with distinct kick/strike, hit reaction and visible recovery; no superhero body language or default shotgun.
- Small destructive interactions should have offensive value: interrupt a chaser, drop a weak support, expose a target or open an attack lane.
- Larger explosions and traps expand this later, only according to the verified narrative unlock state.
- Strength is improvised action under pressure, not universally destructible materials.

**Donut — kill/control from angles Carl cannot reach**
- From the first frame in local 2P she can move and jump distinctly, get above/behind enemies and avoid direct contact, with modest physical/catlike actions before magic.
- After the proper story unlocks, magic converts altitude, precision and positional control into actual offensive power and combo potential.
- Her mobility is **not only** for reaching ceiling switches; it should offer frequent tactical combat advantages.
- The second player must not be reduced to a passenger or an AI-only spell turret.

**Shoulder Mode** remains an unapproved experiment: compare optional mounted positioning and attacks **only with abilities already earned** against independent positioning. Retain mount/dismount tradeoffs, 2P agency, attack readability and a reason to separate. No mandatory shoulder-only encounters before a human verdict.

## Floor 01 combat-learning arc (original gameplay design; not canonical event attribution)

- **Opening Murder Dozer:** demonstrate overwhelming danger and a dramatic chase. Not every enemy is defeatable on sight. This exception increases later combat satisfaction.
- **Tutorial Guild:** a brief safe reset and story-grounded progression; do not turn it into a long mandatory puzzle hub.
- **Early goblin skirmishes:** nail Carl's first direct kill, then an accessible environmental elimination; Donut can reposition and contribute physically before earned magic.
- **Bad Llamas:** combat against telegraphed ranged/area threats; jumping, cover and breaking geometry change how players fight.
- **Scatterers / Hoarder:** crowd control, localized clutter and group interactions; repeated basic attacks alone should be a weak but viable tactic.
- **Goblin workshop:** prime environment-combat set-piece with props, machinery, enemy reactions and bounded chain detonations.
- **Slimes / Rot Stickers:** different hazards; safe attack choices and deliberate triggering from terrain/other foes.
- **Juicer / Ball of Swine:** physical spectacle and culmination of authored combat systems; moving arena features must remain comprehensible without show lighting.

Do not require the specific novel enemy to use unverified attacks or grant an ability early. Validate every source-specific fact before marking it canonical.

## Vertical slice and prototyping priority

**Don't allocate the next large milestone solely to standalone cat-only platforming and noncombat gates.** Once two-actor ownership and camera are testable, prioritize a **combat micro-arena** integrating the existing greybox systems with real character scales and lighting. A short exploration/gate beat is enough to demonstrate each role; the center of the playable slice must be a genuine fight.

Minimum reusable **COMBAT-FEEL-01** fixture (specification only):
1. Carl + Donut, on a readable catacomb floor. In solo, instant switch/companion support; in co-op, two independently controllable heroes.
2. **A single ordinary enemy** whose basic melee defeat is tested for feel and VFX clarity.
3. **A second enemy adjacent to a single authored breakable prop/support**, allowing Carl to cause a spectacular but **understandable** terrain kill, tested with a safe protected exit.
4. **A reachable upper platform** giving Donut a positional benefit (pre-magic movement/survival first, later story-unlocked ranged attack). Do not require a parallel route just for parity.
5. **Fast restart** and identical initial encounter state for solo/2P, with gameplay replay determinism and an observable choice between direct and environmental outcomes.
6. Optional: one tiny sequential Donut-only and Carl-only gate **outside** the central fighting space, to validate switching without defining the whole game as a puzzle.

Human review asks: Is the first hit good? Is an environmental kill noticeably better? Does Donut have a meaningful offensive or tactical role in 2P from the beginning? Does switching change combat opportunities rather than force repeated chores? Is the extraordinary kill earned and legible rather than a cutscene?

### Prioritization / acceptance rule for local agents

For an authored area PR or gameplay task, record:
- **Combat role:** enemy pressure, encounter type, an interaction that can generate an interesting kill, or a deliberate explicit quiet/chase exception.
- **Character roles:** what Carl and Donut can do with the **currently unlocked** tools, and why solo switching or 2P teamwork matters to the combat.
- **Spectacle opportunity:** at least one memorable combat payoff where the level has an encounter; avoid forcing scripted kills in every room.
- **Fairness / recovery:** telegraphs, no friendly-fire death loops, retry, and destructible path protection.
- **Evidence:** direct-kill test, environmental-kill test and duo test as applicable, deterministic traces and real in-game video/screenshots; human verdict on fun.

Do not expand a full enemy catalogue, skill tree, procgen physics system or ten area kits before this core combat feel is proven. Keep Rust + Bevy authoritative, original editable pixel art for production, and rights/novel chronology safeguards intact.

**This document describes intended play; it does not certify that the integrated scene currently has these features.**
