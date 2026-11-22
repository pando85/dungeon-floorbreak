# FLOOR 02 — THE LAST LESSON (design bible v0.1)

**Status: DESIGN PROPOSAL ONLY. Not implemented, not playtested, not owner-approved for gameplay.** 2026-11-22. The floor is a fan-concept game adaptation. Consult AGENTS.md, the MVP controller contract, antihero tone, character progression horizon, combat-first and no-inventory ADRs before implementing.

**Companion documents:** [visual bible](../02-art/floor-02-concrete-hell-visual-direction.md), [production roadmap](../04-production/floor-02-production-roadmap.md), [local-agent work order](../07-agents/floor-02-autonomous-workorder.md).


## IMPORTANT: five playable chapters, ten internal beats — not ten stop-and-start levels

A linear **Metal Slug-like** floor must not feel like a menu of ten unrelated micro-stages. The S00–S09 IDs below are *internal encounter/scene beats* and authoring checkpoints. Present them to the player as **five contiguous authored chapters** with seamless or short transitions; never show ten stage-clear result screens.

| Player-facing chapter | Internal beats | Arc and signature image | Reward boundary |
| --- | --- | --- | --- |
| A — The New Rules | S00 + S01 | cold concrete / complaints desk / Mind Horror | short checkpoint; no forced new mechanics |
| B — Rev-Up Goes Wrong | S02 + S03 | sales-pyramid physics -> tentacled Krakaren Clone | substantial boss clear and safe checkpoint |
| C — People, Not Points | S04 + S05 | cleanup factory -> Meadow Lark wheelchair convoy | brief refuge and narrative pause |
| D — The Impossible Enemy | S06 + S07 | Rage warning/refuge -> Bomb Chicken / stairwell expulsion | major earned end-of-chapter relief |
| E — Tiny Terror | S08 + S09 | black-metal kennel -> Ralph -> Mongo | Floor 02 clear and Floor 03 descent |

**Arcade pacing:** 5–15 minute chunks to play or restart; brief in-room transitions and no forced result panels in the middle of a dangerous setup. The overall first clear 25–40 minutes is only a design hypothesis. If playtesting shows padding, merge/cut *beats*, not combat agency. Floor modifiers and individual viewer score, if later implemented, may resolve at safe chapter boundaries only, never mid-Rage telegraph.

**Detailed production specs (read before implementing the respective chapters):**
- [Rev-Up and rooted Krakaren combat choreography, prop state machines and test matrix](floor-02-revup-krakaren-setpieces.md)
- [Meadow Lark convoy, Rage refuge and Bomb Chicken flight choreography and safety tests](floor-02-meadow-rage-setpieces.md)
- [Kobold kennel, tiny Ralph state machine and quiet Mongo epilogue](floor-02-ralph-kennel-finale.md)

## Additional required encounter and director briefs

- [F02-FOUNDATIONS-01 — Arrival, Mind Horror service desk and cleanup conveyor](floor-02-arrival-cleanup-encounters.md)
- [F02-ACT-01 — Enemy silhouette, personality, tells, hit reactions and original animation keys](floor-02-enemy-acting-bible.md)
- [F02-DIRECTOR-01 — Five-chapter continuity, staging, score and true floor exit](floor-02-director-and-continuity.md)

**Critical cross-stage continuity:** the Rage stairwell is a **Rage-only expulsion trigger** in S07, with a protected Floor 02 side path for Carl and Donut. It must **not** activate the true Floor 03 descent. The actual player exit is **after Ralph and Mongo in S09**. This takes precedence over ambiguous phrases such as "enter the stair region" elsewhere in the draft.

**Rev-Up canon-sensitive actor correction:** the "foreman / pitchman" mechanical role is preferably represented by a **Laminak Rev-Up Consultant Elite**, a fairy supervisor of the Clurichauns, rather than introducing an unrelated invented foreman species. Implement one supervisory role, not two independent waves. This is a proposed adaptation pending primary-text editorial verification.

## 1. Identity and pillars

Working floor title: **THE LAST LESSON**. Visual world: **CONCRETE HELL**.

A soulless white-cinderblock last tutorial floor has been occupied by fraudulent moonshine salespeople, a tentacled corporate boss, a hostile corpse-cleaning ecology, a wheelchair convoy, an unbeatable punishment monster and a kobold dog-pound/fight club. The characters first learn to weaponize their environment better, then learn that **not all fights should be fought**. The final boss is a tiny, furious gerbil, reversing the expectation of an ever-larger monster.

**Rules:** linear, rightward, arcade-style combat-first level progression; distinct authored rooms with landmark silhouettes; deliberately paced quiet beats; readable action at 640x360/640x400; real enemy collisions, environmental outcomes and causal spectacle; standard melee/magic remains effective. No random maze, inventory, gear grind, mandatory extra button, forced class specialization or mandatory 2-human gate. No future-floor skills granted early. No arbitrary camera zoom below legible native scale. No copied book dialogue, third-party art or music.

**Narrative order (book 1, roughly ch.30–47; verify primary text before calling exact details canon):** arrive -> clurichauns / Rev-Up -> Krakaren Clone -> grubs / Meadow Lark -> Rage Elemental -> staged broadcast interlude -> kobold kennels / Ralph -> Mongo -> third-floor descent. Floor 3 race/class choices happen *after* descending.

**Game rhythm:** curiosity -> con-art comedy -> tentacled pressure -> corpse ecology -> sincere rescue -> overwhelming horror -> clever escape -> black-metal arena absurdity -> unexpectedly small frantic boss -> tenderness with small Mongo. This is not ten identical rooms with progressively larger HP bars.

## 2. Stage map (S00 + nine combat/setpiece stages)

Every stage must ship as: uniquely identified room data + one memorable entrance view + encounter purpose + deterministic state machine + direct success condition + optional physical spectacle + restart/exit/party safety + 640x360/400 capture + headless tests + replay evidence. Times below are *provisional targets*, not measured human performance. Expected total first human traversal 25–40 min, with checkpoints allowing shorter sessions; revisit after Floor 01 tests.

### S00 — THE WRONG COLOR (45–70s)
**Look:** harsh transition from Floor 01 shadow to pale concrete, glossy white floor, orange lichen veins, flickering fluorescent rectangles and an absurdly large RIGHT arrow.
**Play:** a corpse hosts a visible Brindle Grub; a distant Mind Horror silhouette makes the new biome unsettling. One simple ambush; a crackable but unbreakable cinderblock tells Carl that this is not a destructible maze. Donut can use a short elevated route, but neither needs it.
**Exit:** regular checkpoint. No unlock.

### S01 — CUSTOMER COMPLAINTS (~2m)
**Look:** underground customer-service counter, bent queue barriers, broken waiting-number display, stale motivational signage.
**Threat:** Mind Horror telegraphs a clear directional psychic pressure lane and becomes physically vulnerable on recovery; a handful of low-pressure grubs gather on defeated bodies.
**Play:** Carl displaces a target into queue rails, Donut flanks via short upper shelf and casts on exposed targets. NO control inversion, opaque screen blur or random unavoidable psionic damage.
**Reward:** optional spectacle route knocks down customer-service furniture, but direct attacks work. Narrative hint: Donut hears of the Second Chance spell. Do not force a new playable special or minion AI.

### S02 — REV-UP: OPPORTUNITY OF A LIFETIME (~2.5m)
**Look:** garish pyramid-sales display jammed into gray walls; crooked gold sign; barrels, false smiling executives, jugs on rollers and torn infographics.
**Threat:** Clurichaun slingshot reps + a foreman whose alarm can be interrupted; limited simultaneous enemies.
**Play:** real kick/body-to-jug chain; 1–3 authored linked explosives and falling sign, warning before explosion. Donut projectile alone must not solve all encounter pressure; use enemy approach, cover and timed recovery.
**Direct alternative:** beat all enemies without touching props; victory is never gated by a magical projectile.
**Story/tool constraint:** the canonical Jug O' Boom is represented first as an interactable environmental explosive family. No new inventory or unapproved special cycling.

### S03 — KRAKAREN HAS OPEN POSITIONS (~3m)
**Look:** gigantic stage podium and projector; behind the cheap corporate facade, real wet organic tentacles emerge from lower service grates.
**Boss:** KRAKAREN CLONE, not Krakaren Prime. Three distinct telegraphed attacks: left/right tentacle sweep, committed center slam, short pull-zone. The vulnerable body is available to Carl and Donut during recovery.
**Routes:** direct damage at openings, or author a capped large payoff by triggering the jug rack/support fall. No one-shot boss exploit, no tentacle immortality puzzles, no inflated HP just to force props.
**Payoff:** defeated clone triggers a finite swarm of grubs. They are a transition pressure beat, not thousands of physical entities.

### S04 — THE CLEANUP DEPARTMENT (~2m)
**Look:** giant extraction ducts and conveyor platforms, garbage cages full of implied writhing grubs, luminous green maintenance signs.
**Threat:** Brindle Grub clusters, one Brindled Vespa with conspicuous aerial acid lane, a normal melee threat.
**Play:** a bounded conveyor/extractor shifts corpses and small foes; punch/kick into a hanging recycling bin, optional deliberate prop kill. Donut can attack Vespa directly. Critical platforms and exit never get destroyed.
**Fairness:** corpse-linked population has finite cap and cannot make killing creatively feel punished. No per-sprite simulated massive swarm. Direct route with no corpse burning works.

### S05 — THE MEADOW LARK EXPRESS (~2.5m)
**Look:** wheelchair train of 3–4 linked silhouettes, human lanterns against white institutional architecture, hanging emergency road signs.
**Play:** an authored automatic convoy with two short defensive stop points, not a physics push/cart escort. Mounted kobold/dingo teaser and aerial pressure; protect while doing satisfying attacks and environmental knockbacks.
**Party:** train follows a deterministic progress script; no NPC routefinding, no unexplained offscreen deaths, no mandatory Donut perfect jump or Carl precision throw. A quiet safe room follows.
**Tone:** Meadow Lark survivors are emotionally significant, not expendable score targets.

### S06 — TERMS AND CONDITIONS APPLY (~1.5m)
**Look:** institutional restroom sign -> absurd system warning -> light tubes turn violet; an enormous off-scale silhouette appears beyond the geometry.
**Threat:** the Rage Elemental; **not a regular boss and not killable**. No deceptive boss HP meter. Initial invulnerability response explicitly communicates retreat.
**Play:** witness, evade a telegraphed impossible blow, enter protected refuge; it attacks the entrance without causing offscreen unavoidable death. A short checkpoint avoids replaying the entire story setup on death.
**Story:** a rule violation and human consequences are conveyed in restrained offscreen staging; avoid a long, repetitive forced-death cinematic.

### S07 — THE BOMB CHICKEN RUN (~3m)
**Look:** emergency-red white corridor; rolling shutters; warning strobes; a vast downward stairwell, initially in the distance and finally filling the scene.
**Three beats:** (1) create a visible lure / decoy using existing interact, melee or explosive authority; this buys time, never damages Elemental HP; (2) cross a corridor with one telegraphed reverse-gravity hazard and safe alcove; (3) entice Elemental to overcommit into the protected stair region, then escape with both heroes. Treat as an authored hazard expulsion, NOT enemy kill or XP grant.
**Fail-safe:** missing the decoy opens a slower direct escape; no mandatory 1-frame spell, Puddle Jumper or 2P simultaneity. AI companion avoids the pursuer, camera preserves player and exit visibility, gate reopens on retry.
**Presentation:** one carefully timed cinematic beat can punctuate the escape but does not become routine earned-death slow motion.

### S08 — NO DOGS ALLOWED (~2.5m)
**Look:** a kobold dog pound built around a black-metal gig venue; original fictional band insignia, cages, scratched speakers, spotlight on an absurdly tiny boss entrance.
**Threat:** Kobold Rider mounted on Danger Dingo, spear telegraph, dismount, kennel keeper. Small roster, no endless dog waves.
**Optional:** physically reachable kennel latch context interaction releases some dingoes as bounded neutral/helpers. No inventory/pet-biscuit menu; if necessary present a scripted single-use story contextual action. Their help is optional, never a critical-path damage requirement.
**Spectacle:** knock rider from mount, launch him into a speaker, collapse the speaker into the fence. Every effect follows real authoritative hit/prop events.

### S09 — RALPH IS FURIOUS (~3m + 1m quiet epilogue)
**Look:** a huge steel pit for an absurdly tiny gerbil; stadium searchlights track Ralph's scurrying dust trails. He must be legible without changing him to an enormous armored boss.
**Ralph:** Frenzied Gerbil, not kobold king. Provisional small 16–24 native-pixel sprite; larger *visual tell* envelope rather than unearned physics hurtbox.
**Three rhythms:**
- The Box: committed horizontal rush, collision recovery. Either hero can punish with existing attacks.
- Tiny Terror: precisely two-beat visible feints between authored anchors; short vulnerability after failure. Not random teleport or input-guessing.
- Second Entrance: if dingoes were freed, an optional dog temporarily swallows Ralph and he bursts out (nonlethal); otherwise an alternative direct-combat vulnerability occurs. In both cases player finishes the fight with either Carl or Donut.
**Exit:** a quiet pet reward chamber presents small, wary **Mongo**. Only narrative appearance, short bond animation and story flag; NO third local player, combat AI companion or mount on Floor 2. Take stairs; class/race selection remains a Floor 3 concern.
**Important:** a flashy Carl-specific finishing pose can be optional cosmetic; 2P Donut is never barred from the win.

## 3. Enemy roster and combat identities

| ID (IP-neutral runtime role) | Concept enemy | Mechanic; answer |
| --- | --- | --- |
| f02_psionic_sentry | Mind Horror | a slow directional psychic lane with interruptible recovery; flank/stagger |
| f02_sales_ranged | Clurichaun consultant | telegraphed slingshot and retreat to stock; direct hits/prop kick |
| f02_sales_foreman | Rev-Up pitchman | limited reinforcement alarm; interrupt + environmental opportunity |
| f02_clone | Krakaren Clone | 3 readable tentacle patterns, body recovery windows; direct or prop |
| f02_grub_cluster | Brindle Grubs | capped ecology near deaths; maintenance hazard, not kill tax |
| f02_vespa | Brindled Vespa | readable air lane; normal air attacks, Magic Missile or bin |
| f02_kobold_rider | Mounted kobold | mounted committed charge, dismount; body collision |
| f02_dingo | Danger Dingo | physical charge; optional local allegiance change |
| f02_ralph | Ralph | tiny, fast, predictable burst and long enough punish window |
| f02_rage | Rage Elemental | invulnerable authored pursuer, safe-room and stairs counter |

Each family needs explicit hurtboxes, body-contact event semantics, target policies, spawn caps, reset state, 2P policy and visual telegraph at native scale. Prefer the existing Floor 01 enemy slot / env prop / hazard / companion infrastructure; do not introduce a second simulation.

## 4. Progression and controller boundaries

**Four face buttons remain Jump, Primary, Physical, Equipped Special.** Carl's physical is kick, never a grab; Donut still pounces and casts Magic Missile when earned; RB interact, LB solo swap, same-screen 2P. Any added authored context action uses already-approved interactions.

Story ledger candidate rows: floor_02_entered, f02_revup_cleared, f02_clone_defeated, f02_meadow_arrived, f02_rage_expulsed, f02_kennel_freed (optional), f02_ralph_defeated, f02_mongo_met, floor_02_exited. These are **design-level names**, not committed schema.

**Second Chance:** canon-relevant Donut necromancy. Narrative mention and capability-data hypothesis only until equipped-special switching and summoned-actor authority are separately reviewed. Do not auto-replace Magic Missile or Puddle Jumper; no spell is mandatory to clear Floor 2.

**Jug O' Boom:** initially a distinctive authored firebomb prop; optional later story-enabled special only after actual controller UX and balance are approved. Avoid world-chemical simulation or crafting UI.

**Mongo:** arrives after Ralph; a small story ally, not a third play-controlled character or an excuse to expand to 3P. Make art and ledger extendible for Floor 3.

**Viewers/momentum:** continue per-character factual attribution; rare earned exceptional death cinema. No repeatable prop-farming loops, no loot economy, no viewer score gate.

## 5. Fairness, safety, performance and test constraints

- F02-A: standard direct combat wins S02/S03/S08/S09 without optional props, free dingoes or new spells.
- F02-B: each room can be cleared with solo Carl lead, solo Donut lead/swap, and local 2P; never requires a perfect AI-assisted jump.
- F02-C: checkpoint and retry restore all enemies, environment, convoy, grubs, boss, allegiance and story gates deterministically.
- F02-D: all critical traversal/camera regions stay navigable after destruction, including tall Donut 128px cases and regrouping.
- F02-E: all attacks and hazardous surfaces telegraph; no offscreen unavoidable wounds; 640x360 and 640x400 captures.
- F02-F: direct Rage HP is nonexistent; direct damage never kills it; expulsion produces no kill XP or ordinary kill spectacle.
- F02-G: missed Rage lure still has a valid path; any player may finish Ralph; no permanent run softlock from dead party member.
- F02-H: grubs and allied dingoes have hard per-room active counts and deterministic fixed-tick behavior, no broadphase O(N²) sim.
- F02-I: meaningful punches/kicks, wall slams, prop chains and duo assists use authoritative events for viewer and reward attribution.
- F02-J: record verified Playtrace for each stage and on retry; a human game-feel verdict and real two-controller hardware are **NOT RUN** until explicitly tested.

Targets for timing, telemetry, risk and controller focus live in [production roadmap](../04-production/floor-02-production-roadmap.md). Do not silently tune Floor 01 mechanics to make Floor 02 pass; isolate any core change and compare with the approved Floor 01 playtest baseline.

## 6. Lore citations, adaptation labels and outstanding owner decisions

Secondary references only: https://www.dccfansite.com/entry/floor-two/ ; https://progressiontime.com/primers/dungeon-crawler-carl/book-01/ ; https://www.dungeoncrawlercarl.nl/en/news/what-happens-on-the-second-floor/ ; https://manatoastmedia.com/dcc-floor-2/ . Primary-book chronology is to be verified by an editorial check. The stage layout, titles, combat tricks, setpieces, industrial art, attack patterns and shortcut alternatives are **original gameplay adaptations**, not literal canon or approved commercial assets.

Open questions after the upcoming Floor 01 owner test:
- F02-OQ01: Can present solo companion/swap and shared camera support the convoy and Rage without redesign?
- F02-OQ02: What normal-vs-spectacular environmental-kill benefit feels earned without trivializing direct combat?
- F02-OQ03: Is a 25–40m initial full-floor experience too long? Cut/merge stages instead of adding filler.
- F02-OQ04: Should Rage's refuge and run feel like one continuous chapter or two checkpoints?
- F02-OQ05: Does Ralph read at native speed with the small silhouette + visual tell envelope?
- F02-OQ06: Can second-chance summons be a separate experiment without delaying any critical story work?

**Never mark any of these accepted based on authored documents or scripted bot pilots.**
