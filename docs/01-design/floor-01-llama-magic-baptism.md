# FLOOR01-LLAMA-MAGIC-01 — The Burning Throats (Donut's first magical combat showcase)

**Status:** owner-selected direction; this document is a **design + agent implementation specification**, not a claim that a new scene is playable, that a new attack is approved or that any game-feel score is final.  
**Placement:** Act I, directly **after the Tutorial Guild biscuit** and **before the short fast-food-restaurant refuge** (I-05 → I-06 → I-07 → I-08). This is new content beyond the protected ENV-02 entrance/guild route. Proposed Stage 03 is approximately 4–6 minutes for this passage, to measure from real play, not a timed gameplay condition.  
**Player promise:** "A minute ago Donut could only swat and pounce. Now she shoots magic into an oncoming wall of glowing throats while Carl advances to hit the monsters." This is her first feeling of **genuine magical strength**, but not an abrupt conversion into a stationary ranged turret.

## 1. Key creative decision: a three-part musical phrase, not a tutorial

1. **Power (I-05):** Immediately after the Guild, Donut can cast Magic Missile into a plain goblin. No new button or menu. Carl is simultaneously useful and dangerous at close range.
2. **Counterplay (I-06):** One Bad Llama creates a clear, physically honest horizontal spit lane. The player learns that facing/spacing, short cover and repositioning affect the fight. Donut can shoot back; Carl can close.
3. **Mastery (I-07):** Two Bad Llamas, initially staggered in approach and attack phases, produce distinct **alternating orange throat glows** while players choose which to interrupt/attack and where to stand. One fragile cover piece changes the lines if damaged. This is the centerpiece, **"the Throat Choir."**
4. **Punchline (I-08):** The danger abruptly gives way to the already-established absurd blue/white fast-food restaurant. A few seconds of relief and quiet. No mandatory extra battle, shopping screen or major unlock.

**The memorable scene is the dual llama volley, not a giant scripted explosion:** two huge llamas at different forward positions; each throat lights up, an orange spit passes, Donut's magenta bolt interrupts or finishes an enemy if hit timings actually allow, Carl rushes through the gap and kicks or punches a llama. It is an emergent **player-authored rhythm of pressure/counterpressure**. The crossfire must be fair, telegraphed and mechanically sound with **no forced synchronized co-op**.

## 2. Existing contracts and hard boundaries

- **Story and abilities:** The biscuit permanently unlocks Donut's primary Magic Missile in place of Swat. Do not allow magic in the approach to the Guild; do not invent a second spell, healing item, dodge input, grenade, damage buff, loot or early Carl dynamite. Donut retains Pounce on Physical. Carl retains basic punches/kicks.
- **Mana (approved):** 8 charges, cost 1 per missile, damage 1; regen begins 30 fixed ticks after the last bolt release, then 40 ticks per restored charge. A 0-mana primary is a dry press, **not Swat**; Pounce remains available. Donut/AI mana and cooldown persist across solo switching and 2P ownership. No mana pickups.
- **Missile current starting behavior:** one straight horizontal projectile, approximately **6px/fixed tick**, **240px maximum range**, release on local cast tick 4; direction is the caster's facing. Do not add diagonal/manual aiming, homing, extra range or vertical tracking to make the encounter easier.
- **Llama:** approved **64px** native-scale figure, **96×72 atlas frame**, visibly glowing throat, distinct lava spit with real travel and recovery. The animation design has a readable **30-tick throat tell**; the actual combat implementation must be audited for its live tell, cooldown and glob trajectories before geometry is fixed. It has **3 HP** in existing fixtures. Reuse approved native art, not generated or borrowed sprites.
- **Llama glob in legacy combat:** approximately **150px/s**, about **42–48px above its own ground feet**, **up to 320px range**, projectile hitbox about **8×6**. Crucially, that lane can **pass over grounded Donut's 20×24 hurtbox**, while threatening standing Carl, but **a Donut standing on a 32px ledge may be in its path**. This is an existing, unusual tactical property to **verify at real height/collision**, not "fix" by inventing vertical homing projectiles. It makes high ground an interesting tradeoff instead of always being safe.
- **Limits:** the existing llama may currently aim at **Carl's x only**, and integrated cat jump / companion movement are being changed in **another workstream**. First audit actual active-player vs AI targeting, collider and projectile ownership; do not silently claim solo Donut gets meaningful ranged pressure until proven. Any broad llama AI retargeting needs coordinated narrow tests/ownership and a separate review if it changes old fixtures.
- **Character identity:** Carl physically closes ground distance and uses kick/interrupt; Donut chooses moving ranged attacks and height. Both playable at all times, including solo player switching, with the same capability state. Don't turn this into a required human-P2 shooting gallery.
- **Visual setting:** dry brick catacombs with **localized scorched masonry**, a short run of pipes/arch openings, burnt timber or leaded glass, and glimpses of the restaurant's unnatural blue/white light near the exit. Not a lava cavern, generic burning city, sewer, rainbow neon arena or whole new biome.
- **Native screen:** existing 640×360 and 640×400 game spaces with approved 52px Carl and 40px Donut masters and full 64px llama silhouette. Camera legibility at 1x outranks any extra decorative platform.

## 3. Authorable topology: 3 connected combat pockets + refuge threshold

**Provisional** layout in 16px horizontal grid, length **2304 world px** (3.6 native view widths). Coordinates relative to the *new Guild exit passage*, not the existing opening route's world x positions. Do **not** append directly to S6 x3584 without checking scene transition, LDtk/story loader and current authored geometry. Floor vertical walking line uses the existing scene's coordinate convention; avoid mixing combat-lab floor y580 with story ENV-02 floor y308.

| Pocket | World x suggestion | Encounter and physical purpose | Initial threat cap |
| --- | --- | --- | --- |
| **M0 — First Spark** | **0–512** | Wide, plain ground. Two ordinary goblins, introduced with time/position separation. One optional **16px** low step, not in the only attack lane. First Magic Missile can hit in the first seconds after the unlock. | 2 goblins |
| **M1 — A Glowing Throat** | **512–1184** | One Bad Llama with a clear forward-facing lane, an **opaque masonry refuge** to break line of fire, a short safe approach to close distance, and a low **32px** optional ledge to test Donut's positioning tradeoff. | 1 llama |
| **M2 — The Throat Choir** | **1184–2080** | Two Bad Llamas at separated locations, one optional damaged **timber screen** that changes the line when broken; several broad dry ground pockets and a short optional upper shelf with safe drop. | 2 llamas |
| **M3 — Fast-food approach** | **2080–2304** | No required enemies, no damage hazards, protected transition/checkpoint. First cool blue/white glimpse of restaurant; resolve music into relief. | 0 |

**Provisional enemy positions:** M0 goblins near x~320 and x~416 (stagger activation so both do not wind up on spawn). M1 llama near x~976 with shelter near x~752–816, adjusting after actual LoS probe. M2 llamas near x~1512 and x~1824, 312px apart, staggered by authored engagement zones so the player reads the **first** threat before the **second**; no offscreen simultaneous spit. Never spawn llamas floating or on a ledge if the runtime only accepts a shared ground walking line.

**Hard geometry rules:**
- All required progress is **linear rightward on reachable dry ground**, without a required P2-only gap, forced jump gate or irreversible destructible wall. Short optional elevation changes are combat modifiers only.
- No "permanent sniper perch" for Donut. Her **horizontal** Magic Missile may fly **above** an enemy hurtbox from a tall ledge. At 32/64px rises, prove projectile-to-llama and projectile-to-goblin overlap *at actual runtime sprite/hurtbox positions*, not by eye from a concept image. If it misses, the design should require **a drop or same-height shot**, not magical aim assist.
- Solid cover genuinely stops llama spit **and** Donut's missile, with visible collision. A static refuge must not render the llama unable to approach or be hit for an infinite free kill. Any destructible cover may open tactical sightlines but is **never** a gate to the exit.
- Camera framing never leaves a live 30-tick glow, active glob or cornered player wholly offscreen. Two-player separation requires adjusted threat activation / camera placement, not player teleportation.
- If the existing loader cannot express the optional raised path safely, first ship **ground + one cover**, prove the combat and restore an optional elevation in a later data-only iteration; don't rewrite the movement engine.

## 4. Fight beats / pacing and enemy composition

| Beat | What happens | Meaningful player decision | Signature moment |
| --- | --- | --- | --- |
| **M0-A, 0–10s** | After Guild transition, Donut's 8 mana segments are visible; first common goblin comes into readable missile distance | Fire Magic Missile while moving or let Carl attack | A magenta scratch missile lands before new long tutorial text could appear |
| **M0-B, short escalation** | Second goblin, separated in time/space | Carl kicks toward Donut's line, or Donut shoots to finish Carl's staggered target; alternatively both clear directly | Cooperation emerges from existing combat interactions, never scripted enemy positioning |
| **M1-A, new threat** | Llama walks into view, throat glows orange for its existing tell | Choose move toward it, stand behind real cover, or shoot if within 240px | First orange glob crosses where Carl stood |
| **M1-B, answer** | Llama committed shot/recovery | Carl closes during the recovery; Donut attacks at ground or deliberately changes elevation | First successful punish after dodging spit |
| **M2-A, alternating warnings** | First llama warns; second becomes active slightly later / after player advances, with a distinct sound/visual cue | Pick a target and a lane, avoid trying to stand perfectly still | Two giant throats heat up out of phase in the same camera scene |
| **M2-B, cover changes** | If hit by a real qualifying glob/melee impact, one weakened screen is removed after its existing materials/impact rule | Use the changed sightline or ignore it and fight directly | Enemy accidentally opens a new attack lane; player exploits it |
| **M2-C, flourish** | Player finishes the second llama, possibly by Magic Missile after Carl opened space | Own the last hit, conserve or spend mana, make a spectacular but **earned** physical finish | Combat stops suddenly; orange fades, only Donut's magenta VFX remains briefly |
| **M3, cooldown** | Warm scorched dungeon gives way to deliberately absurd cool restaurant colors / ambience | Advance without fighting or managing equipment | Comic relief, no new tutorial |

Pacing hypothesis: M0 ~30–50s, M1 ~50–75s, M2 ~75–105s, M3 ~10–20s, total ~2.75–4.2min excluding Guild dialogue and retries. If stage-level 4–6min goal needs longer, improve the encounter rather than filling it with extra waves. No wall timer drives these beats; they respond to real progression and enemy state.

**Spawn fairness:** first M0 projectile target should be near enough for Magic Missile's 240px range and unobstructed. Each llama spawn must allow the player to see its throat tell and travel before collision; use sequential engagement and cooldown offsets **only when state-driven and deterministic**. Never force a global alternating volley script that ignores enemy reactions, stagger, death or line of sight. A llama stopped mid-tell (if current mechanics support cancellation) must not nonetheless spawn a free glob.

## 5. Mechanic: two threats, one decision

The set-piece is deliberately simpler than the Boiler: **one core enemy projectile and one optional cover state**. Its depth must come from spacing, movement and cooperation.

### The Spit Lane

- The 30-tick throat windup must render as a distinct escalating orange heat pulse, then a separate moving 8×6 projectile. Glob stops on solids per runtime contract; must not pass unseen through wall/ledge.
- Mark sources/target ownership so solo Donut (with Carl AI) and P1+P2 are meaningfully threatened according to real AI; do not force aim to the player slot P1. **No global retargeting rewrite without regressions.**
- Test how ground-level Donut's small 20×24 hurtbox reacts: a high glob may **genuinely pass above her**. That is a deliberate feline advantage; the scene may use a 32px ledge to make her choose between a firing angle and exposure, but avoid making a first-time player jump into unavoidable fire.
- Projectile fires along its existing physical height/trajectory with actual collision and damage. Do not add tracking mid-flight, lingering lava carpet, heat damage to the entire screen or extra ground explosions as hidden encounter rules.
- For a first live variant, make sure Carl can always avoid spit with walking, actual floor cover or jumping, and reach the llama to attack with regular melee. Donut can always choose to attack from the same ground line and her mana regeneration eventually provides further bolts.

### Optional fragile screen

- Exactly **one** obvious scorched timber/lead-screen segment, **outside** the only walking path, blocks line of sight/shot trajectories while intact.
- If existing terrain rules support it, a **real incoming llama glob** or Carl melee can shatter it through ordinary impact collision/damage (not a "X coordinate reached" script). Changes the combat lane, never spawns invisible burn puddles or destroys the exit.
- No new global glass-shattering/heat-material system in v1. If current projectile-vs-prop interactions do not exist, use a static breakable already implemented in the engine, or **defer** this optional prop; the M2 duet must remain fun without it.
- Transparent/cracked visual states cannot lie about collision. Once broken, true LoS/projectiles can pass through. Player can still kill both llamas without ever touching it.

### No misleading height tactics

**Specific risk from existing geometry:** Donut's missile is horizontal at her mouth height (~19px above her feet). If she stands on a 64px shelf above a ground llama, the bolt may pass above its entire gameplay hurtbox, even though the silhouette looks tall. A 32px rise can likewise overshoot a 39px body hurtbox depending on whether the head/neck collider is currently integrated. Therefore: **no authored requirement that Donut fire downward or score a high-perch shot until tested**. Instead, the intended high-ground tactic can be **reposition, dodge the ground lane, jump down for a clean shot, or pounce**. Avoid claiming height guarantees magical advantage.

## 6. Difficulty, parity and feel contracts

- The standard starting enemies remain **two goblins (2HP each) and three llamas total (3HP each)** across the passage: one isolated, two in final combat. No Bomb Bard, Copper Chopper, unfamiliar enemy species, extra llama volleys or early spells.
- Magic Missile is 1 damage; three hits per llama if existing 3HP gameplay stays. This is a short burst with meaningful mana costs: the player may run short after the goblins, so keep **Pounce and Carl's melee fully useful**. Do not refill mana after every beat, silently lower llama HP or turn the cast into auto-repeat.
- Never design a required hit that only Donut can do. Carl has a direct route to every llama and can damage them; Donut is an enhancement, not a magic-key lock.
- **Companion mobility/autonomy is independently owned.** Encounter may expose simple target/positioning and safe routes for the AI, but may not implement nav or commands. A valid solo run must not rely on the AI making a precise interrupt; report deficits to the companion track.
- Local 2P can split attack roles but must see both character silhouettes, projectile tells and retreat lanes. Avoid activation zones that let the leader drag an unseen glob into the follower's spawn.
- Donut's passive post-biscuit reflex/dodge is also independently owned; do not duplicate, randomize or retune it here. Under current design llama globs may be explicitly evadable when they actually collide with Donut, but geometry and damage baseline must be valid **with reflex temporarily off** in a test fixture so it cannot camouflage unfairness.
- No boss bar, no mandatory locked combat arena, no platforming puzzle or viewer count at the Guild. A short invisible gate while actual threats are active may be justified only if actual stage rules require clearing them for progression; prefer maintaining arcade forward momentum.

## 7. Visual and audio direction

- Distinguish **hot orange throat tell/spit** from **cool magenta Magic Missile**; both visible at 1×, with reduced VFX, against torch-lit brick. Keep existing LLAMA-01 64px shape and Donut approved 40px art.
- Character acting: Donut does **not** wave a wand or raise a human-like paw; Magic Missile is a feline head/face animation, movement-compatible. Keep already authored cast timeline.
- M1 starts with a brief, very readable first glow; M2 makes dual glows the visual signature, with **asynchronous warning phases** from real enemy timers. The glowing throat is the warning, not merely decorative bloom.
- A short audio "inhale → wet lava spit → cooling hiss" can punctuate each shot, but no constant loud fire ambience; the restaurant transition should be starkly **quiet/cheerful and absurd**.
- Environmental scorch/timber is local to these two llama pockets; reuse A/B/C dry catacomb art and only author new source assets when approved. Fast-food palette is the already-chosen 1990s blue/white visual identity, not a new red neon diner.
- Rare cinematic slow-motion only when a real exceptional skill/finish meets existing CINE-01 criteria; **never every normal Magic Missile kill**.

## 8. Deterministic QA and acceptance

| ID | Scripted / live scenario | Machine expectation | Actual-human evaluation |
| --- | --- | --- | --- |
| **MAG-01** | Enter M0 fresh after Guild | Donut has biscuit flag and exactly 8 mana; Carl no unearned special | Upgrade is immediately obvious |
| **MAG-02** | First goblin in valid 240px lane | Shot spawns at tick 4, collides once and inflicts exactly 1 damage | First cast has strong, clear impact |
| **MAG-03** | Two goblins, Carl first / Donut first | Both direct clear and alternating melee/magic clear work; no hardcoded combo | Does 2P feel more fun than solo? |
| **MAG-04** | Llama first reveal with 640×360 and 640×400 | Tell visible before release, glob only follows actual fired event | Player sees danger before hit |
| **MAG-05** | Carl walks/jumps/covers at M1 | At least one zero-damage movement route and a direct melee kill | Melee remains genuinely viable |
| **MAG-06** | Donut fires from ground / +32 / +64 shelf | Exact missile trajectory/hurtbox collision proves hits or misses as authored | No misleading perch / spell aim illusion |
| **MAG-07** | Llama glob across grounded and +32 raised Donut | Hurtbox-height result matches actual collision and no hidden re-aim | Small-cat advantage is intelligible |
| **MAG-08** | Two llama tells offset / first interrupted | Only each live llama's own firing event creates glob; no offscreen overlap spam | Alternation feels tactical rather than bullet hell |
| **MAG-09** | Screen intact / damaged / destroyed | Both enemy and hero projectiles obey real solids; exit reachable | Changing sightline is fun but optional |
| **MAG-10** | Spend 8 charges / dry press / regen / Pounce | No free Swat, no unwanted mana refill, no special lock | Empty-mana play still enjoyable |
| **MAG-11** | Solo Carl, solo Donut, real local 2P | Same stage, no P2-only gate, no offscreen unavoidable globs | Both heroes useful; camera fair |
| **MAG-12** | Retry mid-spit / after first llama / at checkpoint | Current encounter resets without illegal biscuit reset or duplicate enemies | Quick, frustration-free replay |
| **MAG-13** | Record real runs, replay twice | Event order, source attribution, projectile states, mana/AI/terrain hashes equal | Compare directly controlled vs assisted wins |
| **MAG-14** | M2 direct / cover-used / co-op finish | 2–3 visibly different legitimate outcomes, not guaranteed scripted deaths | At least one memorable "we did that" moment |

**Performance:** native scale captures in 640×360 and 640×400, screenshot output in 1080p, at-speed clips and actual real gamepad test (one and two pads) under current release/harness constraints. Note which cases are *unimplemented*, *blocked*, *scripted-only*, *measured* and *owner-playtest-pending*. ML/autoplay cannot sign off on delight or enemy readability.

## 9. Implementation dependencies and staged agent handoff

1. **M-A — Existing code audit + playable 3-pocket greybox.** Verify actual active Llama target choice, collision/hurtboxes and projectile heights; use post-Guild capability fixture if whole campaign route not yet wired. Preserve the Guild's actual unlock event and avoid leaking future skills.
2. **M-B — First 20 seconds are brilliant.** One first cast, one goblin, Donut movement/cast/VFX, Carl melee, with normal successful exit and no mandatory dialog. Record at speed.
3. **M-C — One llama is fun.** Add M1, shelter and 32px ledge only after true collision testing; make melee and magic both effective. Any targeting bug should be fixed through a separate, owner-coordinated small change.
4. **M-D — Two llama master beat.** Staggered fair 2-llama encounter, brief overlapping threat window, one optional fragile screen only if underlying material/impact pipeline exists.
5. **M-E — Restaurant transition and polish.** No forced combat, context sound/music, native approved art; avoid new shader/lighting engine. Capture minimal live footage.
6. **M-F — Acceptance:** MAG-01..14, full Playtrace, 1P/2P, camera tests, checkpoint restore, independent code review and explicit **owner human game-feel acceptance**. Do not label this implemented by merging design text.

**Ownership:** The repository already has concurrent character ability, Donut mobility/reflex, companion autonomy, opening Dozer, Goblin Boiler and future Hoarder/Dozer-revenge/hazard work. Coordinate any shared `src/combat_scene.rs`, llama logic, `src/bin/floor01_scene.rs`, camera, LDtk and Playtrace changes; avoid competing locomotion, AI and spell APIs.

## 10. References (precedence matters)

- [Act I beats](floor-01-act-01-playable-combat-beats.md) — I-05..I-08 primary topology and narrative order.
- [Donut biscuit and mana](donut-mana-v0-proposal.md), [MVP controller](mvp-controller-contract.md), [character horizon](character-progression-horizon.md).
- [LLAMA-01 art/moves](../02-art/llama-01-spec.md), [Donut Magic Missile animation](../02-art/donut-magic-missile-anim-03-spec.md), [approved Floor 01 art atlas](../02-art/floor-01-art-atlas-v1.md).
- [Combat-lab rooms and numeric constraints](combat-lab-01-rooms-r00-r02.md), [Donut mobility/reflex work in progress](../03-tech/donut-mobility-01.md), [Playtrace](../03-tech/playtrace-01-session-capture.md), [combat-first](combat-first-encounter-design.md), [rights](../04-production/rights-and-release.md).
- [Local agent work order](../07-agents/floor-01-llama-magic-workorder.md).

This is original **game-design adaptation** based on approved project material, not a claim that the exact three-pocket combat sequence is a literal event from any book. No borrowed Metal Slug art, dialogue or assets.
