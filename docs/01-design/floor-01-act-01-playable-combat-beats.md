# Floor 01 — Act I playable combat beats (design proposal v0.1)

**Status:** DRAFT FOR OWNER REVIEW, **not approved** level topology, enemy AI, timing, magic balance, co-op camera, implementation, or literal novel events. Prepared 2026-11-18 as the first detailed pass on the three-act combat-first structure.

**Design promise:** **"Kill, kill, kill"** means frequent *satisfying and varied* combat, not compulsory kill quotas or never allowing the player to recover. The first act goes **panic -> retaliation -> newly earned magic -> positional combat confidence**. Its purpose is to prove the core action feels amazing without making the game a lock-and-key platform puzzler. The ordinary punch/kick kill must be fun before environmental tricks.

**Source of truth / boundaries:**
- [Combat-first encounter pillar](combat-first-encounter-design.md) and [hybrid combat feel](combat-feel-hybrid-physics.md) are approved creative direction; frame values remain test hypotheses.
- [MVP controller contract](mvp-controller-contract.md) is authoritative: Cross/A jump, Square/X primary, Triangle/Y physical, Circle/B **story-earned** special, R1/RB interact, L1/LB solo swap. Carl **kicks, never grabs**. Donut has no magic before the pet biscuit.
- [Floor 01 progression bible](floor-01-gameplay-progression-v0.md) and [living character horizon](character-progression-horizon.md) remain applicable; this document does not approve later skills.
- [Floor 01 environment atlas](../02-art/floor-01-art-atlas-v1.md) controls visual identity and regional mood. Do not infer a literal continuous map from the ten-area collage.
- [ENV-02 opening route specification](../02-art/env-02-opening-spec.md) is the current opening-space baseline, **not a new geometry authorization**. Preserve its accepted option R, six segments, chase alcove and barricade, slumped masonry block, protected Guild portal and floor/character scales.
- Original-IP-neutral Bevy domain schemas and rights/provenance rules apply. Narrative sequence is a *playable adaptation outline*, not independently reverified canonical staging.

## Three-act orientation — framing only, no approved complete map

- **Act I / survive and retaliate:** entrance, Murder Dozer chase, first goblin kills, Guild / biscuit, first Magic Missile fight, Bad Llamas and a short refuge.
- **Act II / make the world kill for you:** Scatterers, Hoarder, goblin machinery/workshop, distinct slimes and Rot Stickers areas, labyrinth/survivor beat.
- **Act III / spectacle under pressure:** troglodyte encounters, Juicer gym, short regroup, Ball of Swine and floor exit.

The acts are **dramatic rhythms**, not literal dungeon divisions, book chapter quotas, or a promise that each zone is a disconnected linear level. Detailed Acts II/III need separate design passes. The opening / Tutorial Guild portion is the **vertical-slice boundary**, not the whole finished Act I.

## Act I beat graph

```mermaid
flowchart LR
    A["I-01 Entry / orientation"] --> B["I-02 Dozer chase / hide"]
    B --> C["I-03 Goblin revenge + shore kill"]
    C --> D["I-04 Tutorial Guild / biscuit"]
    D --> E["I-05 First Magic Missile encounter"]
    E --> F["I-06 First Bad Llama"]
    F --> G["I-07 Mixed llama pressure"]
    G --> H["I-08 Fast-food refuge"]
```

**Separate authored boundaries:** I-01 through I-04 reuse the six ENV-02 segments and existing Floor 01 opening. I-05 through I-08 are **new design spaces**, not already built extensions, production assets or verified level adjacency. No scripted kill cinematic should substitute for an interactive player-authored result.

### Reference encounter matrix

| Beat | Location / existing constraints | Dominant feeling | Main combat / opportunity | New learning | Solo / 2P acceptance |
| --- | --- | --- | --- | --- | --- |
| I-01 First steps | ENV-02 S1 (0–640), S2 (640–944) | Uneasy recognition | No mandatory fight; player may try available move inputs | Face, jump, 2P positions; ominous threat | Both heroes start playable, no required character gate |
| I-02 Murder Dozer | S3 (944–1712), S4 (1712–2608) | Panic | Survive a committed pursuer, avoid humps, refuge niche, Dozer destroys barricade | Terrain can break; a threat can be too strong to kill *yet* | Both players can reach safe niche; same solution in 1P |
| I-03 First goblin takedowns | S5 (2608–3296), S6 approach | Vengeance | Two goblins first; punch/ **kick**; optional 3rd only after readability; rotten shore + fallen stone as a *player-triggered* kill | Direct kill -> launch/contact -> environment kill | Pre-magic Donut contributes through movement, bait or weak feline contact |
| I-04 Guild / biscuit | Protected Guild door in S6 (3296–3584), interior separate | Contrast + reward | A **brief noncombat** story beat; not a long inventory lecture or mandatory puzzle | Donut earns Magic Missile on Square/X; Carl stays the same | Both players stay interactive; exits and camera recover |
| I-05 Spell immediately pays off | New short Guild-exit passage | Surprise / empowerment | Goblins ground + modest elevated attack lane; Donut can now attack at range, Carl closes in | Same button, new action; cooperative angles | Carl can still reach and defeat all critical enemies without a magic-only damage gate |
| I-06 First Bad Llama | New localized scorched-catacomb lane | New risk | One visible spit tell, cover/positioning and a genuine attack window | Projectile hazard; attack then reposition | Donut can shoot from movement/height; Carl flanks and strikes |
| I-07 Llama escalation | New second small combat lane | Mastery | Two *staggered*, distinguishable ranged threats + one optional authored piece of vulnerable cover/terrain | Read more than one tell; environmental opportunities return | No mandatory two-person timing, offscreen projectile or indestructible safe turret spot |
| I-08 Fast-food refuge | Approved art-identity area (new geometry) | Absurd relief | No mandatory fight. Short environmental storytelling, optional resource/gear choices | Pacing and character contrast | No forced combat or loss of player agency; easy resume |

**Estimated testing rhythm only, not approved level duration:** opening through Guild ~4–6 minutes for a vertical slice; post-Guild encounter sequence + refuge ~4–7 additional minutes depending on player skill, dialogue and retries. These are *review estimates*, not runtime benchmarks or timed mission requirements.

## Beat design, authored behaviors and failure modes

### I-01 — Entry: a dangerous place, no tutorial corridor

**Player goal:** move forward, learn what is physically controllable, notice the huge architectural scale and suspicious pursuit cues.

- Use existing S1/S2 catacomb masonry and the approved 52 px Carl / 40 px Donut actors. Do not add giant UI arrows or a shooting range.
- Teach through safe input and environment; one short contextual action prompt is enough if the controller mapping is not self-evident.
- Camera establishes left-to-right direction while keeping both heroes readable, but does not force a fixed actor distance.
- **Avoid:** a slow mandatory dialogue, early magic, starter grenades or an unexplained default firearm.

**Check:** does each hero feel distinct just by moving/jumping for 10–15 seconds? Is Donut already fun to position in two-player mode?

### I-02 — Dozer: panic, readable threat, physical foreshadowing

**Existing material:** S3 lowered maintenance passage; two weak timber sets and small humps; S4 widening, committed Dozer charge, **104 px clear** niche on a ledge, Dozer-smashed full-height barricade, S5 wedge/outcome. Use ENV-02's **option R** clearance (212 px) and marker positions, rather than inventing a competing chase topology.

- **Teach destruction without handing out bombs:** the pursuer splinters timber while players move, making the later damaged timber shore recognizable.
- Favor a committed charge with legible warning/overshoot; never an offscreen, instantly tracking death.
- The alcove is designed for **both** characters to survive. Shared camera must not abandon a slower teammate or make the 2P solution inherently harder than solo.
- A quick checkpoint retry is the intended feel. The chase is the purposeful exception to the "kill often" promise; its payoff is the first revenge fight immediately afterward.
- Dozer itself is not an early boss kill. Goblin riders' exact fallout/spawn remains a gameplay hypothesis, not a required cutscene.

**Check:** is the escape understood without an instruction wall, and can both players see the threat before danger?

### I-03 — First blood: learn a punch, then a kick, then use a stone

**Existing geometry:** S5 (2608–3296) has the **rotten raking shore** around x2952–3080, a separate **fall region [3000, 168, 80, 140]**, planned common-goblin spawn markers near x3136 / x3176 / x3216, and a protected Guild approach to the right. These numbers are authored ENV-02 starting points, not playtest-accepted attack timing or collision values.

1. **The first enemy is intentionally plain.** In a readable, minimally cluttered pocket, one goblin telegraphs an attack and is vulnerable to Carl's basic Square/X punch. This is a real direct-kill *feel* test: if it is boring, do not hide it under more props.
2. **The second rewards Triangle/Y.** Kick causes meaningful displacement; a stagger/contact with another goblin is an optional delight, not a mandatory combo.
3. **The rotten timber shore provides the environmental kill (CP-03).** Goblins may advance from the existing right-side spawn markers into the block's fall zone as players bait/position. Carl can strike the vulnerable shore from outside the fall zone; a heavy block drops onto a valid target when the position/physics align. Never silently move the block, goblin or trigger into a scripted death.
4. **Alternate valid solutions:** defeat goblins with regular hits, or trigger the block after enemies have moved away. Do not require the environmental kill to unlock the Guild.
5. **Optional third engineer:** only add once two-goblin readability, stagger and direct/prop deaths are satisfying; no pre-Guild Bomb Bard, unexplained explosive attack or premature advanced enemy behavior.

**Donut before the biscuit:** low damage, cat-credible pounce/swat and superior mobility. She can draw an enemy away, survive between threats, or reposition, subject to an AI/targeting test. **Do not grant Magic Missile, cast an unearned spell, or make her an obligatory spectator.** Baiting/aggro focus is an experiment, not an approved mechanic.

**Check:** one plain goblin kill feels punchy; a kicked enemy travels predictably; dropped stone can kill one without softlocking the gate; enemies don't become ragdoll damage pinballs. Use deterministic CP-01, CP-02 and CP-03 probes.

### I-04 — Tutorial Guild: short contrast, meaningful transformation

- Reuse the protected ENV-02 Guild portal in S6. Interior is the approved modest, warm old-guide room art direction; gameplay layout not yet defined.
- **Narrative beat:** introduction, relief, and the appropriate pet biscuit. Don't convert the Guild into a lengthy item-management tutorial or permit character swapping to undermine a brief essential story exchange.
- **Major mechanical payoff:** after the relevant event, Donut's **Square/X primary changes from weak swat to Magic Missile**. Her Triangle/Y feline physical identity and agility remain. Carl doesn't gain a new button merely to match her.
- Confirm the story moment and exact unlock chronology against the book before claiming literal canon. Keep an optional skip/fast advance for repeat play once the beat has been seen.

**Check:** gamepad navigation and dialogue are smooth; both players remain able to act; after the cutscene, no rebind or ambiguous ability icon.

### I-05 — The first magical kill: show, don't explain

**New required content (not part of ENV-02):** a short clean passage immediately after Guild. Two ground enemies plus one elevated threat/angle is a *candidate*, subject to visibility and solo balance.

- First target positioned so Donut can instantly understand the purpose of ranged attacks. Magic Missile should function while moving, with a clear direction/impact cue.
- Carl remains viable and satisfying: punch/kick and an ordinary access route; no mandatory magic-only gate or unwinnable solo swap scenario.
- An upper balcony/ledge is an **offensive vantage point** Donut reaches quickly. Carl can access required lanes through a slower ground route or stairs; no forced vertical switch puzzle.
- The environment provides a light opportunity to make the new spell interact with a goblin already destabilized by Carl. Don't add a new destructive material or spell upgrade at the same moment.

**Check:** is casting fun within the first few seconds after the unlock? Does two-player combat improve rather than split into separate disconnected lanes?

### I-06 — Bad Llama reveal: force movement while fighting

**New content:** local scorch/heat marks on ordinary shared catacombs; a scorched **district**, not a complete volcanic remake.

- Introduce **one** ranged Bad Llama with visible windup, projectile travel, and a reasonable recovery interval. The player must be able to read the threat at 640×360 and 640×400.
- Carl can approach with jump/cover, close into contact and strike; Donut can move on or off a modest ledge and fire Magic Missile. Avoid designing a turret that exclusively yields to one character.
- Give the enemy an honest direct-damage solution. A terrain prop/cover interaction is an optional shortcut, not a mandatory switch.
- First hit should be survivable/recoverable under the intended damage model (to test); no unavoidable lava spit or offscreen lock-on.

**Check:** the two heroes have distinct but equally useful approaches; the player naturally learns to change height/position rather than just absorb projectile spam.

### I-07 — Llama escalation: deliberate, not bullet-hell chaos

- Two ranged threats can be staged in **sequence or different elevations**, not both firing on the same tick from offscreen. Do not introduce arbitrary goblin/llama alliances just to fill space.
- A single fragile cover/overhang can be kicked/broken or used for line-of-fire changes if it matches the authored local scenery. This introduces a *new use* of a known verb.
- Avoid a cramped maze, giant scripted collapse or camera framing that loses projectile travel.
- Provide a clear path to rejoin if one player uses a higher route. No invincible permanent firing perch for Donut.

**Check:** tactics and cooperation create faster or more entertaining kills than passive ranged attrition, without making basic melee obsolete.

### I-08 — First fast-food refuge: comedy and tension release

- Use the approved 1990s-style blue/white fast-food restaurant identity (counter, booths, child play area, windows into stone), not a new visual direction.
- **No required fight.** One surprising environmental/story gag and the contrast between absurd amenities and the dungeon can be enough.
- Short recovery and checkpoint, optional readable resources, and a clean setup/exit toward Act II. No major skill unlock here unless explicitly added after separate review.
- Keep a replay-friendly skip for nonessential lines and zero mandatory shopping/minigames.

**Check:** the break feels welcome *because* the preceding combat was strong; the player is motivated to step back into danger.

## Combat dramaturgy and pacing rules

- The overall act should feel like **escape -> personal first kill -> environmental satisfaction -> magic power spike -> new enemy counterplay -> brief relief**.
- A regular kill is already rewarding; spectacular kills arise through contact, positioning and authored consequences rather than mandatory buttons/switches or passive cinematic traps.
- Do **not** specify kill quotas or an enemy on every screen. Small encounter groups keep effects readable and retries short.
- Add no new mandatory input between beats. Circle/B stays story-locked; pre-Guild Carl does not use explosives and pre-biscuit Donut does not cast.
- Use a readable telegraph -> commitment -> consequence -> short opening structure for all hostile attacks.
- Keep visible character and hazard silhouettes at approved native scale; avoid indiscriminate camera zoom-out as a difficulty mechanic.
- Dismemberment/corpses, enemy contact chains, audio/particles and destruction spectacle must not change authoritative state outside deterministic rules.
- The more elaborate environmental death should feel **earned**; avoid "press R1 on glowing button to kill room" as the primary fun.
- If the first direct goblin kill does not feel great, fix timing/animation/sound/contact **before** adding more rooms.

## Two-player and solo parity (required for every beat)

| Contract | Design implication |
| --- | --- |
| True 2P from first moment | Both players control their own actor; pre-biscuit Donut is active, not a follower-only fake |
| Solo switching | L1/LB directly swaps Carl/Donut, inactive actor follows/supports via AI |
| No compulsory two-human gate | Any specific-character obstacle can be completed in solo with switching; no synchronized dual levers |
| Camera fairness | Both players see threats and telegraphs; speed/height asymmetry cannot produce unavoidable offscreen deaths |
| No invented character abilities | Cat swat/pounce before biscuit, no spell; Carl punch/kick, no enemy grab or early bomb |
| Protected progression | Fallen block, smashed timber and other authored damage cannot permanently block Guild or restaurant exits |

**Candidate duo combat beats:** (a) Donut distracts an oncoming goblin so Carl lines up a kick/shore impact; (b) Carl knocks a goblin into an advantageous line for Donut's *post-biscuit* Magic Missile. These must be **choices**, not frame-perfect co-op chores, and only count when demonstrated in a playable test.

## MVP implementation boundary (separate gameplay PRs)

**First integrated slice:** I-01 to I-04 only, using the existing six-segment ENV-02 proposal (S1–S6, 224×25 tiles; option R). Keep chase, CP-01, CP-02 and CP-03 in the **same** scene; don't spin off unrelated demos or replace approved 52/40 px protagonists. Integrate DONUT-INPUT, solo swap and shared-camera tests incrementally without conflicting changes to input ownership.

**Next content extension:** Guild-to-restaurant I-05–I-08 after the existing chase/fight/Guild is demonstrably fun. New encounters may be greyboxes first, then a production kit expansion following existing ENV-01 modular art language. Do not implement all Act I content just because this outline exists.

### Focused validation matrix

| Probe | What human playtesting must show | Automated/test evidence |
| --- | --- | --- |
| ACT1-FEEL-01 | One direct goblin takedown makes the player want another | Hit-stop, attack edge buffering, recovery, sound/VFX recordings, deterministic hit trace |
| ACT1-KICK-02 | Carl kick launches consistently; knockback into another enemy is legible | Collision replay + no repeated chain-damage event |
| ACT1-SHORE-03 | Player can choose to make the stone fall on an enemy | Correct damage/prop states, safe striker, hazard marker, preserved exit, deterministic replay |
| ACT1-DONUT-04 | Pre-magic Donut is fun in solo and 2P and contributes without fake magic | Two-device state ownership, swap, camera framing, no offscreen-only deaths |
| ACT1-MAGIC-05 | Donut's biscuit upgrade is instantly comprehensible | Ability unlocked only after Guild event, same binding, no Carl button change |
| ACT1-LLAMA-06 | Projectile attacks change positioning without making melee obsolete | Predictable trajectories/tells, bounded damage, solo/2P safe routes |
| ACT1-PACE-07 | The overall act alternates interesting fights with intentionally short relief | Scene recording; short retry loop; no forced stop every room |

### Approval checklist for this design document

- [ ] Owner reviews/approves Act I **dramatic arc**.
- [ ] Owner reviews/approves **per-beat combat promises** and pre-/post-biscuit Donut roles.
- [ ] Confirm adaptation of **Goblin -> Guild -> Bad Llamas -> restaurant** against the user-provided Book 1 chapters; flag artistic compression.
- [ ] Verify **I-03** enemy approach and stone fall position in the existing S5 layout with real actors.
- [ ] Run real 1P + 2P playtests before promoting any timings, enemy counts, abilities or geometry from hypothesis to approved production parameters.
- [ ] After Act I direction is agreed, write Act II as a **combat escalation matrix** and Act III as a **boss mechanics/space design**, then stitch acts together into a Floor 01 playable-design bible.

**Change hygiene:** review [living character progression horizon](character-progression-horizon.md) at each new significant character mechanic. Design approvals are not runtime acceptance. Keep a record of rejected room gimmicks as well as adopted ones; never silently turn a draft into a shipped requirement.
