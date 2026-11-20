# COMBAT-FUN-02 — make Floor 01 combat worth replaying

> **Status: design adopted under the owner's design-defaults directive (AGENTS.md, 2026-11-19); not implemented, NOT game-feel accepted.**
> Owner's first hands-on DEMO-01 review: 2026-11-20. Every open choice below has one adopted value with a `CF02-Q##` ID ([Adopted defaults](#7-adopted-defaults-cf02-q01cf02-q33)). Each value is a tuning hypothesis stored as data. The owner reviews game feel after the implementation PRs merge and may change any value in a follow-up PR. Only the [genuinely open questions](#8-genuinely-open-questions) wait for the owner. The [recorded playtest](../04-production/demo-01-playtest-2026-11-20.md) is the evidence; the [agent work order](../07-agents/combat-fun-02-workorder.md) sequences delivery.

## 1. Problem and target experience

The current demo proves that a deterministic, lit Bevy encounter can be completed; it does **not** yet prove that playing it is consistently fun. The owner's recurring complaints are: basic repetitive fighting, enemy-to-enemy impacts with little perceptible consequence, limited evasive options, Donut contributing too little, weak/undifferentiated visible deaths, and flat encounters. Tone/identity rated 2/5. This is a **game-feel failure**, not evidence that the code for contact, gore or finishers is absent.

**North-star test:** repeat the *same 40–60 second fight* three times voluntarily because the player can invent a different satisfying sequence each time. The fight should be exciting with only Carl and goblins, then become more interesting with Donut and with the terrain; killing an enemy by hitting it repeatedly must remain viable.

### What the code says about the complaints (main at 46210b1)

These are reading results, not runtime claims; CF02-00 confirms them with traces.

- **"Deaths are always the same."** The goblin species profile (`assets/data/death/species_death_profiles.json`) has one blunt recipe, `knockback_down`, for routine **and** heavy melee. Every jab, haymaker and lethal kick kill uses the same finish and the same lethal knock (160 / 210 px/s).
- **"The enemy falls but you can't use the advantage."** A kicked goblin lies down for only `DOWNED_TICKS` 18 before its 22-tick getup. Carl's kick total is 36 ticks plus 8 hit-stop ticks, and the goblin lands about 90 px away, so Carl usually arrives during the getup. A lying goblin keeps its standing 24 x 38 hurtbox, so a chest-high jab "hits" empty air above the body and stands it up (`Hurt`).
- **Goblin-on-goblin 1/5.** CP-02 fires only for a **living** kicked goblin. A kick that kills (the common jab-then-kick case) launches a dead body that passes through other goblins. A living contact gives the second goblin a 24-tick stagger and a 1.5 px push, a small result next to the flier. A launched goblin that meets a wall "stops dead" with no event.
- **Llama evasion.** The glob flies straight at 42..48 px above the llama's feet. Carl's only answers are a jump or distance; Donut's 24 px hurtbox is below the line. Donut's moves do not hit llamas (WORK-023-F4).
- **AI Donut 2/5.** The follower never attacks (party Q9 B). PR #123 gave the *controlled* Donut her swat, pounce and mana; the AI still starts no move.

### Authority and influences

- [Vision](vision.md) and [combat-first encounter direction](combat-first-encounter-design.md): basic takedowns must feel good; physical and environmental causality; player-authored mayhem; short recovery, not sponge enemies or puzzle-first rooms.
- [Combat feel / hybrid physics](combat-feel-hybrid-physics.md): fixed-step authoritative outcomes; authored, expressive cosmetic reactions; bounded contacts and one-hit identity; no uncontrolled ragdolls.
- [MVP controller contract](mvp-controller-contract.md): South jump, West primary, North physical, East story-unlocked special, R1 interact, L1 solo swap. No mandatory dodge/grab/sprint/twin-stick buttons. Carl never grabs enemies; the contract already names "Carl punches, kicks, **stomps**" and lists a downward directional attack as a candidate.
- [Antihero tone](antihero-tone.md): a scrappy, imperfect everyman winning by effort, improvisation and accidents, not flawless superhero martial arts. Original 1980s–1990s action atmosphere and vicious situational comedy rather than copyrighted scene/music/dialogue imitation.
- [Character progression horizon](character-progression-horizon.md): early Carl punches/kicks and environment; pre-biscuit Donut uses cat-like swat/pounce/mobility; Magic Missile only after the biscuit; earned explosives only when fiction allows; no early-floor Mongo or unapproved late powers.
- Merged work this design builds on and must not duplicate: [Q10 Donut moveset](../02-art/donut-moveset-q10.md) with its adopted defaults Q10-A01..A10 and [mana v0](donut-mana-v0-proposal.md) (#123); [CINE-01](../03-tech/cinematic-finishers-cine-01.md) (#122); DEATH-D finishes (#108); AUDIENCE-01 viewer counters (#118); THE JUICER boss (#120).
- Our reference vocabulary: **Broforce** for urgency, readable destruction and emergent surprises; **Dead Cells** for quick response and distinctive hit feedback; **Mewgenics** for character expressiveness and reusable authored animation; **Noita** for meaningful material interaction **without** reproducing its per-pixel simulation. These are *principles to evaluate*, not an instruction to copy assets, exact moves or rules.

### Non-negotiable boundaries

1. No extra required face-button verb and no redesign of the approved controller layout; context/direction/state may vary an existing move only when communicated clearly. Remapping/accessibility retained.
2. No early Magic Missile, dynamite, Puddle Jumper, shoulder mode or other story-locked skills; probe flags may explicitly grant post-unlock capabilities for tests only and must not leak into the story route. The DEMO-01 hall stays locked (no biscuit, `DonutSetup::NO` until the Guild room exists).
3. New combo depth = **branching consequences**, not memorized ten-hit strings or higher enemy HP. No arbitrary melee decapitations, blast kills, or slow-motion rewards from ordinary hits.
4. Preserve the visual 1x/Deck/1080p readability and approved Carl/Donut sprite identities, silhouettes, foot pivots and established industrial catacomb art. Never use concept PNGs as production sprites.
5. Same combat opportunity and solvable encounter in solo and local 2P. The solo AI must not steal the player's wins; P2 must not wait as a spectator.
6. Actor/terrain collision, damage, attribution, cooldowns and kill causes stay deterministic at 60 Hz. VFX, screen shake, camera zoom, audio, gore preset and cinematic slow motion never write back to gameplay or its replay hash.
7. Every CF02 rule is **off by default** outside the opt-in `--cf02` scenario until the owner accepts it, so `--demo`, the story route and every existing golden stay bit-identical.
8. The live-viewer HUD (AUDIENCE-01, merged #118 as preview and QA only) **stays absent at the start of Floor 01** and is outside this iteration. CINE-01 eligibility is unchanged.

## 2. What we reuse

| Existing system | Status on main (46210b1) | Gap identified by playtest | CF02 answer |
| --- | --- | --- | --- |
| Carl jab -> haymaker, push kick, hit-stop, goblin tell and stagger | Live in DEMO-01; CP-01 / ANIM-CARL-02 | Owner 3/5, few attacks, monotonous kills | CF02-A |
| CP-02 living goblin body contact | DEATH-A (#79), demo B2 | Owner **1/5** | CF02-B |
| CP-03 shore, falling block, B3 decap and B5 QA charge | DEMO-01 + DEATH-D (#108) | Environmental kill 3/5; decap and blast unseen | CF02-00 audit, CF02-D, CF02-F |
| Death cause/profile/presenter and preset 4 | DEATH-B/C/D/E1 (#94/#97/#108/#106) | Gore intensity **1/5 too mild**, clarity 3/5, aftermath 3/5 | CF02-D |
| CINE-01 exceptional slow motion | Merged #122; skill-gated | No owner verdict yet | Unchanged |
| Solo swap, 2P, follow-and-jump bot | Merged #114 | Swap 2/5, AI 2/5, real two-pad play **not tested** | CF02-C |
| Donut swat/pounce, mana, Magic Missile, dry press | **Merged #123**, game feel pending; AI follower starts no move (Q9 B); llamas not targets (WORK-023-F4) | Owner "Donut does not attack" (#123 may not have been in the tested build) | CF02-C builds on it |
| Viewer counters | Merged #118, preview/QA only | Not a prerequisite | Out of scope |
| THE JUICER boss | Merged #120 | Not in DEMO-01 | Stomp and assist rules only |
| HUD, camera and SFX | #103, #112, #80 | HUD 3/5; zoom unseen, sometimes abrupt motion; some cues not hooked up | CF02-00 trace, CF02-C pan, CF02-D audio |

Do not assert a player's exact build, device or graphics settings: none was recorded. Before making claims about defects, collect the play-session commit/output/flags or reproduce on a known commit.

## 3. Mechanics (adopted values in section 7)

### CF02-A — Short, branching Carl combos and meaningful downed targets [P0]

Keep the quick jab, chained haymaker and push kick. Add three short branches, each with a different consequence:

| Branch | Input | Consequence |
| --- | --- | --- |
| On-the-spot kill | Jab -> haymaker (legacy, connect not required) | Routine kill where he stands (`knockback_down`) |
| Follow-through kick | Jab **that connected**, then Physical in the jab's chain window (Q05) | A faster kick; on a 1 HP goblin a heavy kill whose body flies far (Q06) and can bowl over another goblin (Q10) |
| Ground finish | Kick a fresh goblin, it lies down longer (Q04), then **Down + Physical** stomp (Q01, Q02) | A stomp kill in place; or the kicked goblin slams a wall (Q11) or knocks a second goblin down (Q09) |

- **Stomp binding (Q01).** Variant A, **Down + Physical**, grounded only. Rejected: B, Physical turning into a stomp near a lying target, because the contract forbids "unexplained auto-switches of move type at borderline ranges", it would steal relaunch kicks, and a missed context read is not learnable. Down + Physical always stomps, even with no target, so it is predictable. Physical without Down is always the kick.
- **Lying targets (Q03, Q04).** In `--cf02` a lying goblin has a low hurtbox, so the chest-high jab and haymaker pass over the body (they no longer stand it up), while the kick (relaunch), the stomp and Donut's pounce reach it. Every effective hit on a lying goblin deals damage and never extends the down timer, so ground loops end within the goblin's HP.
- **Carl's effort.** The stomp is an ugly, effortful knee-up and stamp with a short lurch (Q02 root step), not a martial-arts axe kick. It needs new Carl frames (art PR, section 5 of the work order); until then the gameplay PR may use the kick frames as a labelled placeholder.
- **Donut** keeps her merged Q10 moves; her pounce already finishes a lying goblin (Q10-A03) and her swat glances on it.

Guardrails: per-instance hit dedup, bounded stun, no infinite ground loops, no guaranteed instant executions (a stomp deals 1 damage, so a full-HP goblin survives one), no stun-lock of bosses (Q08). Ordinary goblins remain quick to defeat.

### CF02-B — Impacts that create an *observable* second decision [P0]

- **Contact knockdown (Q09, variant B).** A kicked flier moving at least 120 px/s knocks the second goblin **down** (a short hop, then the CF02 lying time) instead of a 24-tick stagger. It still deals 0 damage (Q12). The knocked goblin carries no launch, so it never chains into a third. Below the threshold the legacy stagger applies. Rejected: variant A (stagger only), which is what the owner rated 1/5.
- **Bodies bowl (Q10).** A goblin killed by a kick is a heavy flier: it knocks a living goblin down by the same rule, logged as a new appended `BodyContact` event (not `GoblinContact`, so CINE-01's "living contact" bonus keeps its meaning). Jab and haymaker kills do not bowl.
- **Wall slam (Q11).** A kicked flier at 120 px/s or more that meets a solid emits an appended `WallSlam` event. A living flier takes 1 damage (a kick-launched goblin therefore dies against the wall, credited `Melee`, force Heavy, to the hero who kicked it); a dead flier only crumples. The layout edge without a solid is not a wall.
- The visual adapter reads the events and stays independent of collision. Cosmetic gibs never become damaging projectiles.

### CF02-C — Donut agency and solo-swap responsiveness, on top of #123 [P0]

#123 is merged: the controlled Donut swats, pounces and (after the biscuit) casts. This item does **not** reimplement or retune any Q10 move or mana value. It adds:

1. **Llamas as hero-hit targets (WORK-023-F4, folded in, Q18).** Swat, pounce and Magic Missile hit llamas. A swat on a llama tell interrupts it with 0 damage and sets the same light guard as on goblins (Q10-06), so Donut alone cannot lock a llama; the pounce and the bolt deal 1 damage and stop at the llama's body instead of passing through. The slam rule is unchanged (only Carl's lethal kick slams). This also gives CF02-E a cooperative answer.
2. **Restrained combat assistant (Q13–Q17).** The party Q9 option C, as an opt-in knob, on in `--cf02` only. The **AI Donut** may swat to interrupt a tell aimed at the controlled hero and may pounce a lying or staggered goblin that the pounce cannot kill. She never lands a killing blow, never acts off-screen, never spends mana in the locked demo, and starts at most one move per 60 ticks. The AI **Carl** stays passive (his moves deal damage and would steal kills). In 2P the assistant never runs: Donut is human. A swap still never cancels a move; the AI finishes it and, under the assistant, may start the next one only after the cooldown.
3. **Solo swap feel (Q19, Q20).** A 12-tick cooldown (from 20) and an eased camera pan (fast start, soft arrival) instead of a constant 8 px per tick. The 6-tick buffer, the hurt block, 2P ownership and the party-defeat rules are unchanged.

### CF02-D — Defeat language that really communicates force [P0–P1]

1. **Audit observation first (CF02-00):** on a known build, capture B3 shear, B5 charge, kick impact and llama heavy-vs-routine finishing at 640x360. Classify each miss as trigger not reached, off-screen, hidden by layering, or visible but unreadable.
2. **A heavy goblin finish (Q21).** The goblin profile gains a `slammed` recipe (blunt, heavy). Its presenter picks one of three poses from same-tick events (presentation only): `Hit{Stomp}` -> stomped flat in place; `WallSlam` -> crumpled against the wall; otherwise -> flung (lethal kick). Routine kills keep `knockback_down`. Together with crushed, decapitated and exploded this gives five distinct goblin finishes, each tied to a cause the player chose.
3. **Preset 4 gets more where force is high (Q22).** At preset 4 only, heavy and catastrophic finishes emit 1.5x gibs and may separate one more part where the species anatomy allows. Blood drops, retained corpses and decals stay at today's caps (the owner rated clutter 3/5 and asked for bodies that break, not more stains). Presets 1–3 are unchanged.
4. **Original SFX per cause (Q24)** for punch hits (today they reuse the kick sound), stomp, wall slam, block landing, blast and heavy defeat, mixed by the pack's hierarchy before any global volume change.
5. **CINE-01 is unchanged (Q25).** Stomp, wall slam and knockdowns add no new feat; a bowling body is not a "living contact". Slow motion stays rare and skill-gated.

### CF02-E — Fair defensive choices against the llama [P1]

Adopted: **A + B, no new input** (Q26). Option C (an optional evade command) is not built in this iteration and needs its own input-contract review if the next playtest still rates llama evasion 3/5 or lower.

The spit line (42..48 px above the llama's feet) and the 126 ticks between spits already allow five answers once the arena provides the terrain (Q27, Q28):

| Answer | Who | Needs |
| --- | --- | --- |
| Jump the glob | Carl, Donut | Nothing new |
| Duck behind cover | Both | A solid at least 48 px tall between them and the llama (the arena parapet) |
| Stand above the line | Both | A perch 64 px up (the arena perches) |
| Rush between spits and hit during the tell | Carl; Donut after F4 | Nothing new: 126 ticks between spits, a hit during the 36-tick tell cancels it |
| Stay under the line | Donut | Nothing new: her 24 px hurtbox is below 42 px; adopted as an intended cat-scale advantage |

The tell gets an original audio cue on its first tick (presentation only, Q27). Llama numbers are unchanged.

### CF02-F — A *real* combat microarena with authored choices [P1]

The DEMO-01 hall stays the regression route. One new 40–60 s room, `cf02_microarena`, composed from approved ENV-01/02 kit pieces (no new environment art), reached only through `--cf02` (Q33). Layout (Q29), x from left, floor at y 580, 16 px tiles:

| x range | Feature | Purpose |
| --- | --- | --- |
| 0..32 | Left end wall (solid, full height) | Wall-slam surface for kicks to the left |
| 128 | Spawn and checkpoint 1 | Carl 128, Donut 96 |
| 352..448 | Rotten shore: strike column x 384, block footprint 409..489 | Environmental kill; can hurt heroes (Q30); goblin self-own (Q31) |
| 544..672 | Left perch, one-way, 64 px up (y 516) | Donut vantage; above the spit line |
| 32..736 | Brawl lane: goblins at 520, 600, 680 | Direct fight, contacts, stomps |
| 704 | Checkpoint 2 | After the first wave |
| 736..768 | Stone parapet, solid, 48 px tall | Wall slam from the left; cover from spit from the right; jumpable by both heroes |
| 768..1120 | Llama yard: llama home 1000 (leash 160), goblins at 880 and 940 | Goblins under spit pressure |
| 896..992 | Right perch, one-way, 64 px up | Above the spit line |
| 1120..1152 | Right end wall, solid; exit opens when all enemies are defeated | Wall slam; end of the room |

Width 1152 and height 660 (the DEMO-01 hall height) leave no void at the widest auto-camera view or at 640x400. Goblins keep a fixed floor, so the goblin lanes are flat; the parapet separates the two waves without needing goblins to jump.

Three solutions to the same room: **direct** (jab/kick/stomp through both waves), **environmental** (kick goblins into walls, the parapet and the shore footprint), **cooperation** (Donut interrupts or pins, Carl finishes; in solo through a swap or the assistant).

- **Falling load hurts heroes (Q30, owner-requested).** The block's landing hurts any hero in the footprint through the ordinary hurt path: 1 damage and the hero knockdown, invulnerability respected, the solo guards M1/M4 unchanged. The tell is the existing `settling`/`slipped` states plus a creak and dust from the break; the fall takes 6 + about 31 ticks. Not an instant kill; no off-screen drop (the shore is inside the brawl lane).
- **Authored absurd beat (Q31).** A goblin swing whose impact overlaps the strike zone of a shore that a hero has already damaged counts as a shore hit. The break is credited to the last hero who hit the shore, so the goblin can bring the block down on its friends (or on a careless hero). An intact shore never breaks from goblin swings. No dialogue, no new monster, no viewer counter.

## 4. Release sequence

See the [work order](../07-agents/combat-fun-02-workorder.md#pr-sequence): small PRs with disjoint file ownership, two or three in parallel, every rule behind `--cf02` and data knobs, legacy goldens bit-identical.

## 5. Acceptance, measurement and stop criteria

**Human playtest targets (not automatic pass/fail gates):**

| Item | DEMO-01 | COMBAT-FUN-02 target |
| --- | --- | --- |
| Basic combat fun | 3/5 | >= 4/5 and owner voluntarily repeats the encounter to vary the outcome |
| Enemy-to-enemy impacts | 1/5 | >= 4/5; visible first and second body responses |
| Gore intensity | 1/5 (too soft) | 3/5 (right intensity on the original scale) |
| Death readability | 3/5 | >= 4/5; 3 causal classes named accurately without logs |
| Solo swap / companion | 2/5 / 2/5 | >= 4/5 / >= 4/5 |
| Llama evasion | 3/5 | >= 4/5; at least two different answers used |
| Identity | 2/5 | >= 4/5; remembered authored situation rather than generic combat |
| HUD / camera | 3/5 / zoom unseen, abrupt motion reported | HUD >=4/5; measured pan/zoom trace plus no disorienting snap |
| Checkpoint / real 2P | not tested | must be tested; **no invented numeric baseline** |

**Engineering gates:** original 60 Hz headless scenarios still replay; every existing hash unchanged (CF02 rules are off outside `--cf02`); new `cf02_*` fixtures with committed goldens; no presentation flag alters authoritative events; bounded contact/debris pools; no softlocks in any shore state; the same room solvable in 1P and 2P; original editable sprites; 1x/640x400 render evidence; clean format/clippy/unit/docs CI and per-PR independent review; frame-time measured on target hardware rather than assumed.

**Stop / simplify if:** attack variants mostly whiff due to input buffering, Down + Physical fires by accident in ordinary play, the companion lands more than a quarter of the hits in a solo run, chain contacts cause repeated damage or impossible recovery, camera framing hides threats, color/particles conceal the attacker, or the arena requires an unapproved ability. Narrow the experiment before adding content.

## 6. Decision ledger

**Owner observed or requested:** more combos and attacks; ground advantage via kicks/stomps and Donut claws; environment may injure Carl; more physically broken bodies without simply piling up stains; meaningful evasion; richer authored spaces; Donut needs to contribute.

**Adopted by the orchestrator (2026-11-20, design-defaults directive):** CF02-Q01..Q33 below. They are not owner-tested game feel. Each is a knob or data row the owner can change in a follow-up PR.

**Out of scope:** audience counters and their unlock, end-stage results, subscriber economy, full boss tuning, new enemy-roster production, Dozer/Guild full narrative route, full destructible pixels, procedural arenas, new monetization/loot, an evade button, aerial attacks, and commercial release rights.

## 7. Adopted defaults (CF02-Q01..CF02-Q33)

All ticks are 60 Hz fixed ticks; tick 0 is the tick the press is accepted. Boxes are pivot-relative (x forward, y up from the feet), half-open. Every knob lives in a new data file `assets/data/gameplay/cf02_combat_v0.json` (loaded, validated and FNV-1a pinned like `companion_magic_v0.json`), or in `PartyTuning` where the party already keeps its knobs, under the key named after the ID. A master switch `cf02_rules` (default `false`; `true` only in `--cf02`) gates them all, so every legacy golden stays byte-identical. **Reversibility:** *knob* = change one data value, no code; *switch* = turn a rule off by data; *code* = needs a code change.

### CF02-A — Carl combos and ground state

| ID | Question | Adopted value | Rationale | Reversibility |
| --- | --- | --- | --- | --- |
| CF02-Q01 | Stomp binding | **Down + Physical** (variant A), grounded only. "Down" = D-pad down, or left stick y <= -0.5 with abs(y) >= abs(x), sampled on the press tick; a buffered press keeps the direction it was pressed with. Airborne Down + Physical is buffered like any press. Physical without Down is the kick | No auto-switch at borderline ranges (contract), never steals a relaunch kick, always the same move so it is learnable; uses an existing verb with a direction, as the contract foresees | Knob `stomp_binding`: `down_physical` / `off` (a context variant would be code) |
| CF02-Q02 | Stomp frame data and hit | Startup **9**, active **3**, recovery **14** (total 26). Root step 6 px over move ticks 1..=6, stopped by body contact and walls like the haymaker step. Box `[6, 26) x [0, 10)`. Damage **1**. Hit-stop 6 (+3 lethal), camera kick 2 px. Standing target: stagger 10, knock 0. Lying target: stays down, timer not extended. Lethal: the body stays in place (lethal knock 0 / 0), force **Heavy**, cause `Melee` | Slower than the jab (3) so it is not the default attack; the 9-tick knee-up is a readable tell; 1 damage keeps it from being an instant execution on a full-HP goblin | Knob (each number) |
| CF02-Q03 | Lying hurtbox | While `Downed` and `down_ticks < DOWNED_TICKS`: `[-18, 18) x [0, 20)`. Getup and every other state: the standing 24 x 38. Jab (29..35) and haymaker (27..38) pass over; kick (18..28), stomp, pounce (0..17) reach | The box matches the lying sprite; the owner's "jab stands it up" oddity goes away; kicks still relaunch | Knob (box); switch via `cf02_rules` |
| CF02-Q04 | How long a kicked goblin lies | `DOWNED_TICKS` **40** under CF02 (legacy 18); `GETUP_TICKS` 22 unchanged | Kick hit to landing is about 26 ticks; Carl recovers about 31 ticks after the hit and needs about 15 ticks to walk about 46 px plus 9 ticks of stomp startup: the stomp lands about tick 68 after the kick press, inside the lying window (ticks 39..79), with about 10 ticks to spare | Knob |
| CF02-Q05 | Hit-confirm branch | A Physical press on jab ticks 3..=13 **after that jab instance logged a `Hit`** starts a follow-through kick on jab tick 9 or on the press tick, whichever is later. The follow-through kick is the push kick with startup **9** (legacy 13), same box, active 4, recovery 19, same reaction. A whiffed jab gives no follow-through (the press is buffered as today). Primary and Physical on one tick: Primary wins (same order as Q10-A08) | Rewards connecting with a 9-tick-faster heavy hit, without a new string; jab -> haymaker is unchanged | Knob (window, startup); switch |
| CF02-Q06 | Lethal kick knock | A kick that kills launches the body at **240 / 220 px/s** (about 25 air ticks, about 100 px). Jab and haymaker kills keep 160 / 210 | Makes the heavy kill visibly different from a routine one and feeds Q10 | Knob |
| CF02-Q07 | Aerial stomp / kick | Not in this iteration | Keeps the change small; the contract lists it as a later candidate | n/a |
| CF02-Q08 | Stomp on the boss and the llama | An ordinary 1-damage hero hit under #115's rules (super armour, poise and windows unchanged). On a llama an ordinary hit (stagger 18); the llama slam rule is unchanged (only a lethal kick slams) | No boss stun-lock; no new boss rule | Code |

### CF02-B — contacts and walls

| ID | Question | Adopted value | Rationale | Reversibility |
| --- | --- | --- | --- | --- |
| CF02-Q09 | Contact consequence | Variant **B, knockdown**. At contact, if the flier's abs(vx) >= **120 px/s**, the target becomes `Launched` with `launch: None` (it never chains), vx = push x **90**, vy **90** (about 10 air ticks), then `Downed` for Q04's 40 + 22 ticks. Below 120: the legacy stagger 24 / push 60. Flier side unchanged (pose 6, keeps 25 % of vx, hit-stop 4). One effect per (launch, target) | The second goblin's fall is the visible "second decision" (stomp it, pounce it, or kick it again); a kick launch is 170 px/s, so every kick contact qualifies | Knob (threshold, hop); switch |
| CF02-Q10 | Do killed bodies bowl? | Yes for **lethal kicks only**: the body keeps the kick's launch instance and knocks a living goblin down by Q09, logged as an appended `CombatEvent::BodyContact { tick, launch, source, target }`. Jab and haymaker kills do not bowl | Turns the common jab -> kick kill into a projectile without making every death a projectile; a separate event keeps CINE-CONTACT-SETUP's "living" meaning | Switch |
| CF02-Q11 | Wall slam | A kicked flier (living or a lethal-kick body) with abs(vx) >= 120 that is stopped by a solid emits an appended `CombatEvent::WallSlam { tick, launch, goblin }`. Living: **1 damage**; at 0 HP it is defeated in place (`Melee`, force Heavy, credited to the hero whose kick made the launch), otherwise it drops `Downed`. Dead: event only. The layout edge without a solid is not a wall | A kicked goblin within about 50 px of a wall dies against it: a deliberate environmental kill using only the kick | Knob (damage 0 turns it into a visual-only slam); switch |
| CF02-Q12 | Contact damage | **0** (unchanged) | Knockdown already gives the payoff; no damage from bodies or pieces | Knob |

### CF02-C — Donut, the assistant and the swap

| ID | Question | Adopted value | Rationale | Reversibility |
| --- | --- | --- | --- | --- |
| CF02-Q13 | Opt-in assistant | Party Q9 option **C** as `PartyTuning::follower_assist`: `true` in `--cf02`, `false` elsewhere (so `--demo` and every golden are unchanged). Applies to the **AI Donut only**; AI Carl stays the passive Q9 B bot. Never in 2P | The owner rated AI Donut 2/5 and said she does nothing; Carl's damaging moves would steal kills | Knob |
| CF02-Q14 | Assist swat | Trigger: a goblin (or, after Q18, a llama) in its tell **aimed at the controlled hero**, at tell tick >= **8**, within **96 px** of Donut, Donut and target both on-screen, target not guarded. Donut runs toward it and swats only if the swat's hit tick lands by the target's last tell tick (goblin tell 24, llama tell 36); otherwise she does not start | Leaves the player the first 8 ticks to act; only interrupts real threats to the player's hero | Knob (8, 96) |
| CF02-Q15 | Assist pounce | Only on a goblin that is lying or staggered **and has HP above the pounce damage** (never lethal), 30..80 px ahead, on-screen | "Donut pins, Carl finishes"; never takes the last hit | Knob (range) |
| CF02-Q16 | Assist cadence | At most **one** AI-started move per **60** ticks, none while a move is running; a swap mid-move lets the AI finish it as today | Matches Q9 C's "once per 60 ticks"; prevents automation | Knob |
| CF02-Q17 | Assist magic | Pre-biscuit: never. Post-biscuit (probe flags only; the demo stays locked): Magic Missile only to interrupt a tell aimed at the controlled hero, 97..160 px away, only if `mana - cost >= ai_mana_reserve` (2, Q10-A09), never lethal | Gives the existing reserve knob its first use without touching story progression | Knob |
| CF02-Q18 | Llamas as hero-hit targets (WORK-023-F4) | Swat on a llama: 0 damage; on a tell it interrupts (llama stagger 18, `LLAMA_HURT_COOLDOWN` 30) and sets the Q10 light guard (clears after 60 ticks **and** the llama's next spit release, or on defeat); otherwise a glance. Pounce: 1 damage, one target, stops at the llama body. Magic Missile: 1 damage, the bolt ends on the llama. A Donut kill is a plain `down`; the slam rule is unchanged | Fixes bolts flying through llamas; gives Donut a cooperative llama answer; the guard stops a Donut-only lock | Code (targets); knobs for numbers. **Not** behind `cf02_rules`: it is a bug-class follow-up of #123 and applies everywhere; CF02-C1 must prove with a parity test that no existing golden changes, and update the DEMO-01 plan and questionnaire Q12d |
| CF02-Q19 | Swap cooldown | **12** ticks (from 20) in `--cf02`; buffer 6 and the hurt block unchanged | Faster hand-over; the buffer still stops spam | Knob |
| CF02-Q20 | Swap pan | Eased: each tick the camera moves `clamp(ceil(remaining / 4), 2, 12)` px toward the new hero (legacy constant 8). About 44 ticks for 448 px (legacy 56), about 9 ticks for the 40 px follow distance (legacy 5) with no hard stop | Fast start, soft arrival: addresses "abrupt camera" without a slower pan over long distances | Knob (`swap_pan_mode`: `constant` / `eased`) |

### CF02-D — defeat language and audio

| ID | Question | Adopted value | Rationale | Reversibility |
| --- | --- | --- | --- | --- |
| CF02-Q21 | Heavy goblin finish | New goblin recipe `{ "kind": "slammed", "finish": "goblin_slammed", "causes": ["blunt"], "min_force": "heavy" }`. Presentation picks the pose from same-tick events: `Hit{move: Stomp}` -> **stomped**, `WallSlam` -> **wall crumple**, otherwise **flung**. Each pose holds its impact frame at least 8 ticks before the aftermath | Ends "deaths are always the same" with causes the player chose; reuses the merged recipe schema (the llama already has `slammed`) | Data (profile row) |
| CF02-Q22 | Preset 4 intensity | At preset 4 only: heavy and catastrophic finishes use **6 gib quarters** (1.5x) and may separate **one** extra part where the species anatomy allows. Drop quarters, corpse cap, decal cap and presets 1–3 unchanged | The owner said too soft at 4 and clutter fine: more breakup where force is high, not more stains | Knob |
| CF02-Q23 | Defeat audit before art | The CF02-00 audit classifies B3 decap, B5 blast, heavy kick and both llama finishes before any death art is drawn | Do not redraw what was merely off-screen | n/a |
| CF02-Q24 | SFX | New cue ids: `PlayerPunchHitConfirmed` (jab, haymaker), `CarlStomp`, `EnemyWallSlam`, `ShoreBlockLanding`, `BlastDetonation`, `EnemyDefeatHeavy`, `LlamaTell`; contacts and bowling reuse `EnemyBodyCollision`. At most two major transients overlap (pack rule) | Each cause sounds different; the punch no longer reuses the kick | Data (cue map) |
| CF02-Q25 | CINE-01 | **Unchanged.** No new feat; `BodyContact` does not count as a living contact; boss and routine kills still never trigger | Slow motion stays rare and skill-gated | n/a |

### CF02-E — llama defence

| ID | Question | Adopted value | Rationale | Reversibility |
| --- | --- | --- | --- | --- |
| CF02-Q26 | Which defensive options | **A + B**: jump and positioning plus terrain answers. **C (evade command) not built**; revisit only if the next playtest rates llama evasion 3/5 or lower, with its own input-contract review | No new verb; the five answers in the CF02-E table need only the arena and F4 | n/a |
| CF02-Q27 | Llama numbers and tell | Unchanged: tell 36, recover to 60, cooldown 90, glob 150 px/s at 42..48 px above its feet, range 320, fair fire. Add the `LlamaTell` audio cue on tell tick 0 (presentation) | The timings already allow jumping and rushing; the complaint was variety, not time | Knob |
| CF02-Q28 | Terrain counters and Donut | Cover >= 48 px tall stops a glob (existing glob/solid rule); a perch 64 px up is above the line; Donut's 24 px hurtbox under the line is an **intended** cat-scale advantage | Gives three non-input answers; Donut still cannot kill a llama fast (1 damage per pounce, 3 HP) | Data (room) |

### CF02-F — the microarena

| ID | Question | Adopted value | Rationale | Reversibility |
| --- | --- | --- | --- | --- |
| CF02-Q29 | Layout | The section 3 CF02-F table: 1152 x 660, floor 580, left and right end walls, shore at strike x 384, left perch 544..672 and right perch 896..992 at 64 px, parapet 736..768 at 48 px, goblins 520/600/680 and 880/940, llama home 1000, checkpoints 128 and 704, exit on clear | One feature serves several answers (the parapet is a slam wall and cover); flat goblin lanes fit the goblin's fixed floor | Data (`cf02_microarena.room.json`) |
| CF02-Q30 | Falling load vs heroes | The landing hurts every hero in the footprint once: **1 damage + hero knockdown** through the ordinary hurt path (invulnerability, M1 and M4 respected), evaluated on the landing tick with the goblins | Owner asked for it; fair because it has a 37-tick tell and an 80 px footprint ahead of the striker | Knob (damage 0 = off) |
| CF02-Q31 | Goblin self-own | A goblin swing impact overlapping the strike zone of a shore in `damaged_1` or later counts as one shore hit; the break is credited to the last hero who hit the shore; an intact shore ignores goblin swings | Authored comedy that still needs the player's setup; attribution stays with a real hero action | Switch |
| CF02-Q32 | Encounter rules | Two waves by position (no scripted triggers beyond the goblin sight 220 and llama fair fire); exit opens when all six enemies (five goblins, one llama) are defeated; Select hold restarts the room (existing 60-tick hold) | Short, repeatable, the same in 1P and 2P | Data |
| CF02-Q33 | How it ships | `floor01_scene --cf02` loads the room with `cf02_rules: true` and the assistant on; `--demo` and the story route unchanged until the owner accepts | Owner can A/B the old hall against the new room on one build | Knob |

## 8. Genuinely open questions

Only these need the owner's hands or ears; every other choice above is defaulted.

1. **Feel of the new verbs on a real pad.** Down + Physical accidental activation, the stomp's 9-tick startup and the follow-through kick window cannot be judged by fixtures. Owner plays `--cf02` solo and answers the CF02 scorecard.
2. **Real two-pad 2P and checkpoint respawn.** Never tested by a human. Needs two physical controllers; agents mark it NOT RUN.
3. **The new sounds.** Agents can synthesize the Q24 cues but cannot judge them; the owner listens on his own speakers (or decides to commission a sound designer).
4. **Which camera motion felt abrupt.** CF02-00 traces swap pan, impact kick and zoom; only the owner can say which moment he meant. A short owner capture of one occurrence would settle it.

## 9. Horizon fit

Consulted the [character progression horizon](character-progression-horizon.md). The design fits it:

- **Carl** stays "impact first" on Floor 01: punches, kicks and now a stomp on a downed foe, with walls and other bodies as the novelty. No explosives, no grab, no new button.
- **Donut** keeps low-damage feline contact before the biscuit; the assistant never uses magic before the unlock. Llama targets extend her existing moves.
- **Solo vs 2P:** the assistant answers the horizon's open question about solo AI "without P2 spectatorship": it never runs in 2P, never kills, never acts off-screen.
- **Mongo** is not affected.

The horizon's change record gains one line for this iteration (Carl's ground stomp and the restrained AI assist as Floor 01 candidates).
