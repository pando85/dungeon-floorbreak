# Character progression horizon — living design guide

**Status: LIVING REFERENCE / DIRECTIONAL, NOT AN APPROVED SKILL TREE.** Established creative principles and the existing MVP control contract remain approved **in their own source documents**. Everything in this guide about future skills, floor-specific loadouts, combat effects, companions, difficulty curves, or precise unlock timing is a **hypothesis for repeated review**, not a production commitment, a book-canon declaration, or a request to implement features now.

**Last considered:** 2026-11-20. **Scope:** Floor 01 MVP through a possible long-term THE CRAWL campaign. **Owner intent:** provide an evolving horizon to consult whenever gameplay, abilities, enemies, levels, progression or protagonist interactions are designed. Update it as playable evidence and later narrative research overturn assumptions.

This is a *navigation aid*, not an authority override. In conflicts, follow [AGENTS.md](../../AGENTS.md), rights/accepted ADRs, [approved MVP controls](mvp-controller-contract.md), the [combat feel contract](combat-feel-hybrid-physics.md), and specific owner-accepted floor designs. **Do not use this guide to justify unrequested scope expansion.** It does not imply commercial game/character rights.

## North star

**Start with Broforce-like immediacy; grow into a deep physical/magical/terrain combat sandbox without multiplying mandatory inputs.**

- **Increasing expressive power:** existing actions gain meaningful properties and interactions (launch, stagger, disruption, selective destruction, tactical protection), not merely larger damage numbers.
- **Increasing tactical depth:** new enemy behaviors, materials, geometry and temporary floor rules make the *same* abilities useful in new ways.
- **Increasing distinctiveness:** Carl stays a scrappy punch/kick/explosive improviser; Donut stays mobile, fragile and cat-shaped while becoming a capable caster and tactical partner.
- **Increasing cooperation:** solo switching + AI companion and local 2P both offer valuable choices. Asymmetric actor-gated routes are fine; never require two humans for a critical gate.
- **Increasing readability:** enemies should usually die in a few satisfying hits; preserve physically legible outcomes and deterministic gameplay. A larger arsenal must not bury the player in UI or special-case controls.

**Invariants inherited from accepted decisions:** four face-button action verbs (jump, primary, physical, equipped special); Carl **kicks rather than grabbing enemies**; Donut has no magic before the biscuit; no required twin-stick aim; character unlocks respect narrative order; Mongo is introduced later and is not automatically a third local-player slot. Consult [the control contract](mvp-controller-contract.md) rather than reproducing or overriding bindings here.

## Four progression layers (design framework, not implemented architecture)

| Layer | How it grows | Persistence question | Example hypothesis |
| --- | --- | --- | --- |
| Core action mastery | Upgrades to existing punch, kick, pounce, jump, Magic Missile and their contact/terrain properties | Usually persistent | A stunned enemy flies farther when kicked; not a mandatory combo string |
| Equippable tools / spells | Acquired explosive or magical special selected for the same action slot | Inventory/equipment dependent | Carl prepares a bomb; Donut selects a later unlocked tactical spell |
| Floor-specific rules | Enemies, hazards, props, machinery, one-off systems and temporary abilities | Usually local to that floor | Different terrain and telegraphs reward familiar skills in new ways |
| Allies / synergy | Mongo and other allied behaviors, duo interactions, contextual requests | Story-and-availability dependent | A coordinated setup/finisher rather than extra compulsory buttons |

This is a **conceptual taxonomy**. It does not approve a skill tree, inventory UI, unlock currencies, a number of loadout slots, companion command wheel, or per-floor reset rules. Progression/state implementation stays IP-neutral and separate from physical controller bindings.

## Character identity over time

### Carl — impact first, sabotage later

- **Floor 01:** accessible punches and **kicks** that displace enemies, with later story-earned explosives and an equipment-enhanced fist. Novelty comes from collisions and selective destruction.
- **Early floors:** deepen reliability of physical attacks and improvised traps rather than replacing melee with a remote detonation simulator.
- **Mid campaign:** let new materials, hostile machinery, explosives and preparation opportunities make Carl more of a saboteur; still lethal and satisfying with his basic attacks.
- **Late campaign:** scale his influence from one opponent to complex encounters and coordinated operations, **not** to superhero anatomy, generic firearm mastery or grappling.
- **Never lose:** a satisfying direct goblin takedown, environmental attribution, vulnerability/risk, and the approved 1980s–1990s everyday antihero character arc.

### Donut — agile cat to mobile magical tactician

- **Pre-biscuit Floor 01:** high/far movement and rapid positioning, only low-damage feline physical actions; entertaining in P2 without fabricated magic.
- **Post-biscuit Floor 01:** Magic Missile becomes her primary attack; she *keeps* catlike physical movement/contact. Puddle Jumper comes only after its narrative event and requires a separate mechanical adaptation decision.
- **Early floors:** improve movement/ranged reliability, survivability through positioning, and teammate synergy. Mongo joins later; his precise abilities and join timing require source-grounded review.
- **Mid campaign:** introduce distinct control, support, protection or summoning decisions **only as verified/unlocked**, avoiding an all-purpose invulnerable turret.
- **Late campaign:** powerful and tactically flexible, potentially interacting with allies or temporary floor systems; still agile and fragile enough for positioning to matter.
- **Never lose:** four-legged cat silhouette, charming/theatrical personality, mobility-led play, ability to act without Carl's direct assistance.

### Mongo — companion horizon, not a premature controls commitment

- Joins after the initial Carl/Donut relationship is established, **not a Floor 01 default character**.
- Explore a progression from small companion to physically consequential ally and possible Donut mount, subject to source validation, pixel-scale tests and gameplay evidence.
- Start with optional AI/contextual assistance; do not precommit to full third-character direct control, three-player co-op or new compulsory button grammar.
- Reference [Mongo lore tracker](../02-art/mongo-lore-research.md) before claiming a species detail, growth timeline, attack or mount behavior is canon.

## Horizon by campaign phase (illustrative, not a release roadmap)

| Narrative phase | Player learning focus | Carl hypothesis | Donut / team hypothesis | Revisit before commitment |
| --- | --- | --- | --- | --- |
| **Floor 01 — MVP** | Movement; immediate hits; knockback; first earned specials; one rewarding environmental kill | Punch -> kick -> story-earned explosive -> enhanced fist | Fragile agile cat -> biscuit / Magic Missile -> advanced mobility only when earned | Exact first-floor chapter/event, feel and co-op hands-on playtest |
| **Floor 02** | Mastery and a few new enemy/prop interactions, not necessarily new inputs | Build on existing impact/explosive kit | Refine magic and survival; later companion arrival | Check canonical item/companion timing |
| **Floor 03** | More defined build identities and meaningful equipment choices | Potential sabotage emphasis | Potential floor-specific character-specialization decisions | Verify race/class lore and whether UI complexity is warranted |
| **Floors 04–05** | Complex environmental/machinery interactions with established verbs | Prepared traps, systems and terrain exploitation | New options in support/mobility/control, if earned | Avoid designing a separate controller or per-floor engine |
| **Floors 06–07** | Deliberate encounters, information, and different hazards | Planned encounters and control of terrain | Precision, cooperation and defensive utility | Some floors may be narrative or atypical; no mandatory new skill each floor |
| **Floor 08** | Evaluate temporary rule sets as an optional layer over core combat | Keep core punch/kick identity | Optional special floor mechanics and allied tactics | Verify actual narrative rule set; do not precommit to a separate card game |
| **Floor 09+** | Larger-scope encounters / teamwork without losing arcade feel | From individual fighter toward tactical sabotage/leadership | From caster toward tactical/support leadership | Scope, feasibility, story verification and player comprehension |
| **Later floors** | Unknown; preserve room for dramatic change | Do not invent abilities prematurely | Do not invent abilities prematurely | Review primary book events when actually designing them |

**No new mandatory skill each floor.** A floor can evolve the *world* instead of the protagonist. These bands are design sketches, not a canonical chronology, milestone list, shipping promise or approved class/loadout design. Do not treat any example as a ticket by itself.

## Floor 01 reference path (detail belongs in the Floor 01 bible)

1. **Opening / Dozer:** Carl uses physical melee and positioning, Donut fast traversal/low-damage contact; neither starts with bombs or spells.
2. **Tutorial / biscuit:** Donut gains Magic Missile, a **major** gameplay change; Carl primarily gets gear/passive improvements, not a corresponding new control.
3. **Early encounters / Hoarder:** deepen hit reactions and combat-under-pressure; Carl uses story-available dynamite as an earned special at an appropriate point.
4. **Workshop / hazards:** extend terrain/explosive interactions and make duo positioning more meaningful; no forced new buttons.
5. **Labyrinth / survivors:** consolidate known moves; challenge combinations and shared-camera/AI behavior.
6. **Juicer:** cooperative climax; the gauntlet modifies Carl's fist attacks and Donut gains Puddle Jumper **after** the encounter.
7. **Ball of Swine / exit:** mastery test combining known actions and geometry, **not** a final barrage of new abilities.

See [Floor 01 gameplay/progression bible](floor-01-gameplay-progression-v0.md) and [MVP controller contract](mvp-controller-contract.md). This sequence is an *adaptation teaching proposal*: each unlock/event still needs chapter-grounded review, and its mechanical effect needs human playtest signoff.

## Questions to keep open rather than freeze

- Which upgrades enhance **player agency** versus simply inflate damage/health? What new choices do they provide?
- How many specials can be **equipped** versus merely known, and how are they switched on a controller? R2 is reserved, not a finished loadout UI.
- How should magical mobility, explosive placement, airborne impacts and defensive powers be bounded so they do not trivialize encounters?
- How do solo AI, co-op camera, character switching and possible Donut/Mongo riding scale without P2 spectatorship or mandatory multi-human puzzles?
- Which abilities belong to core progression, consumable equipment, specific-floor rules or one-off narrative events?
- How many new systems can a player learn in a stage **before** encounter comprehension deteriorates?
- What is truly book-grounded (chapter and item acquisition) versus a fan adaptation, prototype hypothesis, or approved art-only representation?

## Mandatory review habit for design work

**Consult this guide whenever a PR/issue meaningfully adds or changes:** playable character moves, enemy reaction rules, progression/unlock events, equipped abilities, level gates or terrain synergies, AI/co-op agency, a new floor's gameplay model, or a major combat/balance redesign.

For that change, the agent/designer must:

1. **Read the latest guide and controlling documents** rather than rely on earlier prompts or model memory; identify which parts are accepted constraints versus proposals.
2. **Describe the horizon fit in the design PR:** what existing action/character role it builds on, what new decision it gives the player, and whether it affects solo and 2P.
3. **Check narrative chronology** for franchise-specific items, spells, classes and companions before describing them as canon; mark unverified claims and separate game adaptations.
4. **Test or propose a focused test** for added mechanics: predictable hits, readable knockback, deterministic replay, safe routes and camera, real controller feel when implemented.
5. **Update this guide when the horizon changes** (milestone, role, layer, contradiction, evidence or rejected idea); otherwise add one line to the PR stating that it was consulted and remains consistent. Do not edit this guide for trivial tuning or cosmetic changes that do not alter the direction.
6. **Escalate conflicts** with approved controls, art, architecture or owner decisions for explicit review. This guide alone cannot approve a new button, production ability or story deviation.

**Periodic review:** explicitly revisit the guide at each *new floor design kickoff*, after significant progression/mechanics playtests, and before locking that floor's gameplay bible. Compare it with the actual build, not just documentation.

### Change record (keep concise and append when the horizon changes)

| Date | Change | Status / evidence |
| --- | --- | --- |
| 2026-11-18 | Initial long-term horizon: Floor 01 learning, permanent action identities, layered progression, future-floor design themes and review triggers | Living guide; owner asked for a revisitable reference, **not** approval of future skills |
| 2026-11-20 | COMBAT-FUN-02 after the first DEMO-01 playtest: Carl's Floor 01 impact set gains a Down + Physical stomp on downed foes and its aerial form (dive), a short air kick, a scrambling bail-out hop and wall/body impacts; ordinary goblins gain reactive morale (hesitate, panic, enraged, taunt) with seeded temperaments; the solo AI Donut gains a restrained, opt-in assist (interrupts and non-lethal holds, never in 2P, no magic before the biscuit). No new button, no new unlock, no new enemy type | Adopted defaults CF02-Q01..Q75, behind `--cf02`; game feel pending ([design](combat-fun-02-post-demo01-proposals.md)) |

### Update note template

When materially revising this file, capture in the PR:

- **What changed and why?** New playtest evidence, narrative research, floor concept or discovered conflict.
- **Status:** accepted invariant / new candidate / rejected hypothesis / needs book verification.
- **Impact:** Carl, Donut, Mongo; Floor 01, later floors; actions, equipment, terrain, co-op, camera.
- **Next validation:** actual test or owner decision required.

**Document status rule:** approving this living *guide* approves the **practice of revisiting a horizon**, not every example or hypothetical feature inside it.
