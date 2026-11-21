# CARL-PROG-01: Carl's Floor 01 combat progression, reconciled spec

**Status:** design adopted by recommendation under the owner's design-defaults directive (2026-11-21). Not implemented. Every value below is a data knob with a question ID (CP-Q01..CP-Q43), not approved balance. Game feel needs the owner's playtest. Delivery is split into small PRs in the [work order](../07-agents/carl-prog-01-workorder.md).

**Owner request (2026-11-21), summarized:** all existing physical moves from the start (0–20 %); enemies that demand positioning, interruptions and aerial play (20–40 %); dynamite, permanently, through a story event at the Hoarder (~40 %); explosive traps, launches, chain reactions and destruction taught progressively (40–80 %); after the Juicer (~85 %) the gauntlet, which gives punches stagger, armour interruption and selected material breaking; a final boss that tests mastery with no new unlock. Dynamite: Circle/B, a predictable arc, a 2.5 s fuse from release, a 64 px radius, a 7 s recharge after detonation, unlimited uses, at most one active, reduced self-damage, no friendly fire, enemy launches, structure breaking, chain reactions, no manual detonation, cooking, inventory or extra control, the existing moving-throw animation, data-driven numbers. Unlocks are narrative-event-driven, persistent and checkpoint-safe; the percentages are pacing targets only.

**Read with:** [MVP controller contract](../01-design/mvp-controller-contract.md) (authoritative for buttons), [character progression horizon](../01-design/character-progression-horizon.md), [Floor 01 progression bible](../01-design/floor-01-gameplay-progression-v0.md#carls-combat-progression-carl-prog-01), [Act I beats](../01-design/floor-01-act-01-playable-combat-beats.md), [combat-first encounters](../01-design/combat-first-encounter-design.md), [REWARDS-01](../01-design/combat-rewards-and-no-inventory-v1.md) and [ADR-0010](../05-decisions/ADR-0010-combat-first-progression.md), [COMBAT-FUN-02](../01-design/combat-fun-02-post-demo01-proposals.md), [Juicer boss](../02-art/floor-01-boss-design.md), [ANIM-CARL-03](../02-art/carl-dynamite-anim-03-spec.md), [DEATH-D](death-d-finishes-spec.md), [destruction](destruction.md), [antihero tone](../01-design/antihero-tone.md).

## 1. Existing inventory (reuse, do not rebuild)

| Milestone need | Already on `main` | Reused as | New in CARL-PROG-01 |
| --- | --- | --- | --- |
| Carl's base kit from the start | Jab, haymaker, push kick (legacy); stomp, follow kick, air kick, dive stomp, bail-out hop, wall slams and body bowling under `--cf02` (CF02-A/B/H, merged, game feel pending) | Unchanged. The 0–20 % band adds **no code** | A parity test that Circle/B stays inert before the dynamite grant |
| Teaching encounters 0–40 % | Opening route ENV-02 S1–S6, CP-01/02/03 fixtures and the shore (`src/shore.rs`), DEMO-01 hall, CF02 microarena (perch, parapet, shore), Bad Llama, Goblin Engineer (`--cf02 --cf02-engineer`) | Pacing map in section 6 | Encounter content is listed as later work, not v1 code |
| Story unlock state | Donut's biscuit only: `DonutMana` + `UnlockTrigger` zone + `DonutSetup` start state, `RestartKind` (retry and checkpoint keep it, fixture restart restores), `--donut-biscuit`, `Unlock::PrimaryRanged` (`src/donut_mana.rs`) | The **pattern** (start state, once-only trigger, restart kinds, nothing hashed while locked) | A generic, IP-neutral ledger for Carl (section 3); the biscuit keeps its storage in v1 |
| Checkpoints and retries | `RestartKind`, DEMO-01 `respawn` / `Respawn { checkpoint }`, boss restart; no save file exists | Same restart kinds | Ledger restart rules; a serializable snapshot for the future campaign save |
| Equipped-special input | `RawInput.ability` / `ability_pressed` (Circle/B, East; keyboard binding in the scene adapter), inert in `combat_scene.rs`; COMPANION L1+Circle is the `Regroup` chord | Same input, press edge only | The capability-resolved special (section 4) |
| Blast resolution | `src/blast.rs` (DEATH-D): `BlastCharge`, `Arm::{Tick, CarlPast}`, `BLAST_RADIUS` 20, point blank vs ring, `Goblin::blast`, `CombatEvent::Blast`, `Explosion` cause, `goblin_exploded` finish, burst and fizzle drawing, audio cue | The core and its finishes unchanged; the dynamite is a new charge **profile** | Outer ring, line of sight, self-damage, chain, support breaking |
| Throw art | ANIM-CARL-03: `carl_book1_h52_dynamite` (9 standing + 36 moving frames, tick contract, spawn tick t41 and offset), `vfx_dynamite` flight and fizzle loops; VFX-01 `burst`. All `prototype_fixture_unapproved`, independent review pending | The throw timeline, the moving variant, flight, fizzle and burst | Runtime wiring only (no new art in v1) |
| Structures | CP-03 shore (3 melee hits, block fall, crush), boss rack strut (3 hits, drop, crush), CF02-F shore rules (Q41/Q42); `Block.intact` and `Material::{Stone, Steel, Catwalk, Timber}` exist, nav has `destroy_solid`, but no route solid is breakable and no broken-solid art exists | Supports are the v1 "vulnerable structures" | A blast-break and a gauntlet hit weight for supports |
| Armour | Goblin committed swing (smear and impact: damage and hit-stop only); Engineer launch and ram (committed, same rule); Juicer super armour and poise (`boss.rs`); llama and goblin light guard (Donut swat only) | The existing committed phases are the "armour" the gauntlet interrupts | One hit property (section 5) |
| HUD | `SpecialSlot::Locked` padlock socket; the enum reserves `Equipped(Chip)`; `vfx_hit_spark_metal` exists | The reserved variant and spark | The equipped socket and its recharge state |
| Juicer fight | `--scene boss_f01`, `brute_boss_f01` data, `EnemyDefeated { target: BOSS_ENEMY_ID }`; the boss spec plugs "dynamite 4 (explosive cause)" through data | The defeat event is the gauntlet's story event | Dynamite damage on the boss; the grant |
| Combat lab | Profile `floor01-postbiscuit-lab-v1` pins dynamite and gauntlet **off** (LAB-Q01); `ability.rs` lists both as unimplemented | Unchanged in v1 | Carl flags are refused in the lab |

## 2. Chronology check and adopted anchors

**Repo sources** (the art atlas and the late-enemy approval notes record chapters as checked against the owner-supplied Book 1 EPUB; this PR did not re-read the book and quotes no book text): Tutorial Guild Ch. 3–7; rats Ch. 8 onward; Scatterers and the Hoarder's lair Ch. 10–11 (the Hoarder Ch. 11); Goblin Bomb Bard Ch. 15; Copper Chopper Ch. 16; troglodytes and the Juicer's gym Ch. 23–24; the Ball of Swine and floor exit Ch. 26–27.

**Order:** biscuit (Ch. 3–7) < Hoarder (Ch. 11) < Juicer (Ch. 23–24) < Ball of Swine (Ch. 26–27). This matches the owner's order and the progression table (Hoarder is beat 4 of 9, the Juicer beat 8 of 9). As a chapter-position heuristic with the exit at Ch. 27, the Hoarder sits near 41 % and the Juicer near 89 %, close to the owner's ~40 % and ~85 %. No code reads a percentage.

**Unverified in repo sources:** *how and exactly when* Carl obtains dynamite and the gauntlet in Book 1. The contract, horizon and ANIM-CARL-03 place them at "early encounters / Hoarder" and "Juicer gym" without a chapter citation for the acquisition. They are game adaptations until the owner confirms them against the EPUB. Because the anchors are data event IDs, moving one later changes no code.

| ID | Adopted default | Conflict it resolves |
| --- | --- | --- |
| CP-Q01 | Dynamite is granted on the story event `swarm_boss_f01_defeated` (the Hoarder's defeat). | The contract says "First encounters / Hoarder ... once earned" and the horizon "at an appropriate point": ambiguous between the early encounters and the Hoarder. The owner names the Hoarder, so the defeat event is the anchor. |
| CP-Q02 | The gauntlet is granted on `brute_boss_f01_defeated` (the Juicer's defeat, `EnemyDefeated { target: BOSS_ENEMY_ID }`), on the defeat tick. | The [Carl bible](../02-art/carl-character-bible.md) lists the gauntlet under art milestone II (Books 2–3, "only when acquired in canon"), but its own line 38 says milestones are not acquisition claims. Gameplay unlock at the Juicer; the gauntlet on Carl's sprite waits for an art order with a canon check. |
| CP-Q03 | The final boss is the Ball of Swine. Its scene may hold no unlock trigger (load-time validation). | The [boss spec](../02-art/floor-01-boss-design.md) titles the Juicer "Floor 01 boss" and the ids are `boss_f01` / `brute_boss_f01`. It is the first implemented boss, not the final one; ids stay as they are. |
| CP-Q04 | Percentages are pacing targets from chapter positions; milestones are events. | Owner rule. |
| CP-Q05 | The combat lab profile stays unchanged (dynamite and gauntlet off; Carl flags refused). A `floor01-posthoarder-lab-v1` profile is later work. | LAB-Q01 pins "after the biscuit, before the Juicer", which spans the Hoarder; under CARL-PROG that pin really means "before the Hoarder". |

Donut's Puddle Jumper is granted after the Juicer in the contract and the horizon. It is out of scope here and is not fabricated; the ledger can carry it later as its own capability.

## 3. Unlock model: one generic story-unlock ledger

**Recommendation: one generic, IP-neutral mechanism** for every story capability. Carl's two capabilities use it now. Donut's biscuit is registered in it as a **read-through view** (`DonutMana::unlocked()` stays the storage) and migrates later in its own byte-identical PR. Reason: `src/donut_mana.rs` is pinned by hashes and is touched by DONUT-MOBILITY-01 (#173); moving its storage now risks both. `--donut-biscuit` is unchanged.

- **Module:** `src/story_unlock.rs` (new, Bevy-free). Engine types never name a franchise item or character.
  - `CapabilityId` (data ids): `special_thrown_charge` (dynamite), `fist_reinforced` (gauntlet), `primary_ranged` (biscuit, read-through). The fan layer maps display names in data.
  - `UnlockLedger`: the held set plus grant receipts `{ capability, tick, source }`. `grant` is idempotent (a second grant returns `None` and logs nothing).
  - `StoryTrigger`: `Zone { rect }` (the `UnlockTrigger` shape) or `Event { id }`, matched against combat events through a data table (`brute_boss_f01_defeated` = `EnemyDefeated { target: BOSS_ENEMY_ID }`; `swarm_boss_f01_defeated` waits for that encounter). Once only.
  - `StoryPin`: the declared start state of a scene (the capabilities held at tick 0).
- **Data:** `assets/data/gameplay/story_unlocks_v0.json` (capabilities, triggers, pins, notice text, provenance and the CP-Q ID of each row), loaded, validated and FNV-1a pinned like `companion_magic_v0.json`. A load error names the field.
- **Hash:** the ledger mixes **nothing while empty**, so every scene without a grant hashes as before. Once held, it mixes a tag, the sorted capability ids and the receipt ticks.
- **Event:** `CombatEvent::CapabilityGranted { tick, hero, capability }`, appended, emitted only on a grant.

| ID | Adopted default |
| --- | --- |
| CP-Q06 | Capability ids as above; `primary_ranged` is a read-through in v1. |
| CP-Q07 | Lifetime "run": a grant survives a defeat retry, a checkpoint respawn, a solo swap and a 2P join or drop. A fixture restart (R / Select) restores the declared start state. There is no save file today; the ledger serializes to a sorted, versioned snapshot so the future campaign save ([ADR-0010](../05-decisions/ADR-0010-combat-first-progression.md) point 8) can store it. |
| CP-Q08 | Story pins under `--carl-prog`: opening route, DEMO-01 hall and the CF02 microarena have none (nothing held); `--scene boss_f01` is pinned `f01_after_swarm_boss` (dynamite held from tick 0); the final-boss scene, when it exists, holds both and has no trigger. |
| CP-Q09 | Flags: `--carl-prog` turns on pins and triggers; `--carl-dynamite` and `--carl-gauntlet` grant at start (the `--donut-biscuit` style, debug and test). All three are refused with `--scene combat_lab`, are recorded in the PLAYTRACE reset descriptor (new fields default to false, so older recordings still load) and do nothing to a scripted fixture, which declares its own pin. |
| CP-Q10 | The biscuit keeps `DonutMana`, `UnlockTrigger` and `DonutSetup`; unification is an optional later PR that must prove every `q10_*`, party and story golden byte-identical. |
| CP-Q11 | Grant presentation: a HUD System notice (data text, 13 characters at most, owner tone check pending) and the special socket turns from padlock to equipped. No cutscene and no inventory screen. |

## 4. Dynamite (`special_thrown_charge`)

All knobs live in `assets/data/gameplay/carl_prog_v0.json` (knob names start with their question ID, for example `cpq18_fuse_ticks`), loaded, validated and FNV-1a pinned. Ticks are 60 Hz.

| ID | Rule | Value |
| --- | --- | --- |
| CP-Q12 | Input: the Circle/B **press edge** of the forwarded `ability` action. No hold, no cook, no manual detonation, no cycling, no new binding. The COMPANION `L1 + Circle` Regroup chord never starts a throw. | press only |
| CP-Q13 | A throw starts only when the capability is held, Carl is grounded, not hurt, not defeated, runs no melee move, has no live stick and the recharge is done. Otherwise the press is rejected (`SpecialRejected { reason }`), not buffered. | — |
| CP-Q14 | Timeline = ANIM-CARL-03 section 3: draw t0–5, fuse lit t6, hold t16–29, wind t30–37, release t38–41, **stick spawns on t41**, follow t42–49, brace t50–61, settle t62–69 (70 ticks). From t50 any move or jump cancels the recovery; nothing cancels earlier. | 70 / 41 / 50 |
| CP-Q15 | Walking and running continue during the throw (the moving variant). Jump presses are ignored from t0 to t49. The stick flies the way Carl **faces on t41**. | — |
| CP-Q16 | Spawn point: pivot + (facing × 16, 46 up), always the standing offset (the 1 px run-frame offset is art only). | 16 / 46 |
| CP-Q17 | Arc: `vx = facing × 150 + 0.5 × carl_vx` (never backwards), `vy = 260` up, gravity 1050 (`GRAVITY`). The stick ignores actors, stops dead on a walking surface (no bounce or slide), loses `vx` on a wall and `vy` on a ceiling, and never detonates on contact. Standing it lands about 111 px ahead of Carl's pivot on flat ground, running about 170 px. | 150 / 50 % / 260 |
| CP-Q18 | Fuse: detonation exactly 150 ticks (2.5 s) after the release tick (`==` compare, like DEATH-D). | 150 |
| CP-Q19 | Recharge: 420 ticks (7 s) counted from the detonation tick; at most one stick live from t0 to its detonation; unlimited uses. | 420 / 1 |
| CP-Q20 | Reach: squared distance from the blast centre (stick foot + 8 px, `BLAST_CENTRE_DY`) to a body box, compared with squared radii. **Core** 20 px keeps the DEATH-D rules exactly (0 = point blank, catastrophic, `goblin_exploded`; up to 20 = heavy, thrown dead). **Outer** 64 px. | 20 / 64 |
| CP-Q21 | Outer ring (20 < d ≤ 64): 1 damage and a living launch away from the centre, `vx` 200 and `vy` 220 (clamped to `KNOCK_VX_MAX`), a new launch instance credited to the thrower. A lethal outer hit is `Explosion { point_blank: false }`, force heavy. | 1 / 200 / 220 |
| CP-Q22 | Line of sight: the segment from the centre to the closest point of a target's box is blocked by the interior of any intact solid (touching edges do not block). Applies to enemies, Carl, supports and charges. Evaluated before any break on that tick. | — |
| CP-Q23 | Self-damage: Carl inside 64 px with line of sight takes 1 damage (reduced: the core kills an enemy), a knock of 120 / 160 px/s away, `HERO_HURT_TICKS` and 55 invulnerable ticks (the CF02-Q42 block values). It can be lethal at 1 HP. | 1 |
| CP-Q24 | Friendly fire off: Donut is never damaged, launched or logged by Carl's blast, in solo and 2P. | — |
| CP-Q25 | Juicer: 4 damage (the boss spec's plug), counted toward poise as damage; super armour holds; no launch. | 4 |
| CP-Q26 | Llama: the core kills as for goblins; the outer ring deals 1 damage with the llama's existing kick reaction. The Engineer is a goblin kind and follows the goblin rules. | 1 |
| CP-Q27 | Hit while throwing: t0–5 cancels (nothing spent); t6–40 drops the lit stick at Carl's feet, its fuse restarts at 150 from the drop tick and it is the live stick. | t6 |
| CP-Q28 | A defeat retry, a checkpoint respawn or a fixture restart removes a live stick without a blast and makes the special ready; the capability stays (CP-Q07). | — |
| CP-Q29 | The stick and the recharge belong to Carl, not to a player slot: a solo swap mid-fuse changes nothing. The AI never throws in v1. | — |
| CP-Q30 | Credit: `owner = Some(player controlling Carl on t41)`. Self-damage uses a new `HeroHurt` source id for the thrown charge. | — |
| CP-Q31 | Events (appended, emitted only while the capability is held): `SpecialStarted`, `ChargeThrown`, `ChargeLanded`, `SpecialRejected { reason }`, `SpecialReady`; the existing `Blast { tick, charge }`; the dynamite is a `BlastCharge` with its own profile, appended after the fixture charges. | — |
| CP-Q32 | HUD: `SpecialSlot::Equipped(Chip)` with the Circle glyph and a label from data (default `BOOM`), muted while a stick is live or recharging. | — |

**Tick order** (appended to the existing order): Carl strike and special press, scene physics, then the **stick** step (flight, landing), goblins, llamas, contacts, shore, boss, **charges in index order** (fixture charges first, then the dynamite), morale. Self-damage applies in the charges step.

**Generalizing the blast without moving a golden:** `BlastCharge` gains a profile `{ core_radius, outer_radius, outer_damage, launch, los, hurts_owner_hero }`. The QA profile is `{ 20, 20, -, -, off, off }`, which reproduces DEATH-D exactly (its ring is lethal, there is no outer band), so the two DEATH-D fixtures, the DEMO-01 B5 charge and every hash stay byte-identical. The dynamite profile is the table above.

**Moving-throw animation (reuse):** ANIM-CARL-03 section 5's runtime rule: while a throw phase in {light, hold, hold_b, wind, release, follow} is active and the locomotion state is `run`, draw `dynamite_move_<phase>` frame `run_frame - 4`; standing, draw the standing frame; draw, brace and settle on the run show the plain run frame. The throw clock is the same in both. The stick draws `dynamite_flight` in the air and `dynamite_fizzle` on the floor, then the VFX-01 `burst`.

| ID | Visual stand-ins in v1 |
| --- | --- |
| CP-Q33 | The Carl throw and stick art stays `prototype_fixture_unapproved` until its independent review. The VFX-01 burst is drawn unchanged (about a 20 px fireball; 2x at preset 4), so the 64 px reach is **wider than its art**: v1 adds a radius ring under `--debug-combat` only. A natively drawn `burst_large` (VFX-01 section 8) is a later art order (top-tier design). Audio reuses the existing detonation cue. |

### 4.1 Interactions: traps, launches, chains, structures

These rules make the 40–80 % band teachable with content that already exists. They apply only to a blast with the dynamite profile; QA charges keep DEATH-D behaviour.

| ID | Rule | Value |
| --- | --- | --- |
| CP-Q34 | Chain reaction: an authored charge (`BlastCharge`, any arm) whose centre is within 64 px of a detonating dynamite, with line of sight, is set to detonate 8 ticks later, unless it is already due earlier. An unarmed `CarlPast` charge counts as armed. A chained charge uses its own profile and may chain further. Depth cap 4, at most 8 detonations per tick, each charge detonates once (dedupe), matching [destruction](destruction.md) explosion semantics. | 8 / 4 / 8 |
| CP-Q35 | Structures: a blast whose outer circle intersects the CP-03 shore strike zone or the boss rack strut, with line of sight, breaks it outright (all remaining hits). The existing fall, crush and credit rules follow, credited to the thrower's player. | — |
| CP-Q36 | Launches: an outer-ring launch is a normal launched body, so the existing CP-02 body contacts and the CF02-B wall slams and bowling apply wherever those rules are on. Nothing new is added to them. | — |
| CP-Q37 | Terrain: no route solid breaks in v1. The data file declares `wood` and `brittle_stone` blast-break rows **disabled**: no route has breakable solids and no broken-state art exists. They turn on with TERRAIN-L1, which owns collision rebuild, navigation update and debris. | off |

## 5. Gauntlet (`fist_reinforced`)

The gauntlet changes what the existing punches do. No input, no new move, no damage increase (horizon: properties, not bigger numbers).

| ID | Rule | Value |
| --- | --- | --- |
| CP-Q38 | It modifies **Jab and Haymaker only**. Push kick, follow kick, stomp, air kick and dive stomp are unchanged. Damage is unchanged. Applied as a `MoveSpec` mapping in Carl's strike path (the same place as the glance hook), only while the capability is held. | — |
| CP-Q39 | Stagger: jab 10 → 16 ticks, haymaker 18 → 28 ticks. | 16 / 28 |
| CP-Q40 | Armour interruption, mapped to existing states: a gauntlet punch **pierces a committed attack** (goblin smear and impact, Engineer launch and ram: the phases `take_hit` treats as committed). The attack is cancelled, the punch's stagger and knock apply and the swing can no longer hit. Carl's strikes resolve before the goblins on a tick, so a same-tick impact never lands. The smallest addition is one `MoveSpec` field, `pierces_commit` (default false, appended). The Juicer's super armour is **not** pierced; a gauntlet punch adds 2× poise there. Llamas have no committed phase today. | 2 |
| CP-Q41 | Material breaking: on authored supports (CP-03 shore, boss rack strut) a gauntlet haymaker counts as **2 hits**, a jab as 1, so jab + haymaker breaks a fresh support in one combo. Terrain solids are out of scope (CP-Q37). | 2 |
| CP-Q42 | Feel: haymaker hit-stop +1 tick. Presentation of a pierce: the existing `vfx_hit_spark_metal` and a 2 px camera kick (presentation only, never hashed). | +1 / 2 |
| CP-Q43 | Grant notice from data (13 characters at most). **No change to Carl's sprite in v1**: the visible gauntlet waits for an art order with a canon check (CP-Q02). Tone: a battered, practical piece of gear; heavier punches, the same clumsy everyman, no glow or aura. | — |

## 6. Pacing plan per milestone

Existing content teaches through encounters; new rooms are **later work**, not v1 code.

| Band (pacing target) | Story anchor (repo chapters) | Teaches | Existing content that teaches it now | New content needed (later work) |
| --- | --- | --- | --- | --- |
| **M1** 0–20 % | Entrance, Dozer, first goblins, Guild and biscuit (Ch. 1–7) | Direct kill, kick launch and body contact, environmental kill, stomp and dive, wall slam, bail-out | Opening route S1–S6 with CP-01/02/03; CF02 microarena (shore, parapet, perch, temperaments); DEMO-01 hall | I-05 post-Guild passage (Act I beats) |
| **M2** 20–40 % | Bad Llamas, rats, Scatterers (Ch. 8–10) | Positioning against a ranged tell, interrupting tells, fighting from and into the air | Bad Llama (DEMO B4, CF02-C1), Goblin Engineer (middle-lane ram: jump, dive, push kick), CF02 perch and dive | I-06/I-07 llama lanes; Scatterer swarm and rat enemies (art exists: SCAT-01, RAT-01; gameplay specs needed) |
| **M3** ~40 % | The Hoarder's defeat (Ch. 11) | Dynamite granted (CP-Q01): the socket lights up, one safe first use | None (HOARD-01 is art only) | Hoarder encounter spec and arena; a short post-fight beat with a first-throw target |
| **M4** 40–80 % | Goblin workshop, Bomb Bard (Ch. 15), Copper Chopper (Ch. 16), slimes and Rot Stickers, labyrinth, troglodytes (Ch. 23) | Explosive traps (authored charges, trip charges), launches, chain reactions, support collapses; in that order, one new idea per room | DEMO-01 B5 trip charge pattern (`Arm::CarlPast`), CP-03 shore, CF02 bowling and wall slams | Workshop rooms with authored charges; Bomb Bard and Rot Sticker gameplay specs (enemy charges, `owner: None`); TERRAIN-L1 breakable timber and brittle stone with broken-state art |
| **M5** ~85 % | The Juicer's defeat (Ch. 23–24) | Use dynamite in a boss fight (4 damage, the rack strut); gauntlet granted on defeat (CP-Q02) | `--scene boss_f01` (pinned with dynamite under `--carl-prog`) | Gym approach rooms; the route link through the gym door |
| **M6** 85–100 % | Regroup, Ball of Swine, exit (Ch. 26–27) | Gauntlet pierce against committed swings, one-combo support breaks; then mastery of everything | CF02 microarena with `--carl-gauntlet` (playtest only) | Short gauntlet teaching room; Ball of Swine arena (section 7) |

## 7. Final boss: mastery without unlocks

The Ball of Swine ([arena proposal](../01-design/floor-01-ball-of-swine-arena-gameplay.md)) is the mastery test. When it is designed it must:

1. Hold every Floor 01 capability from its pin and contain **no** trigger (CP-Q03, validated at load).
2. Give each Carl tool at least one authored moment where it is clearly the best answer: a kick or air kick punts a detached Tuskling into the composite; dynamite breaks a side support or stuns the ball after a wall crash; a gauntlet punch pierces a detached Tuskling's committed charge; the stomp or dive finishes a downed one.
3. Never gate damage on one tool (no dynamite-only or gauntlet-only damage window), and stay winnable in solo with an AI Donut, as the Juicer is.
4. Keep every outcome deterministic and readable without show lighting.

## 8. Gating and goldens

- **No flag, no change.** Every new rule runs only while a capability is held, and capabilities are held only through `--carl-prog` pins and triggers, `--carl-dynamite`, `--carl-gauntlet`, or a `carl_prog_*` fixture. The ledger, thrower and gauntlet mix nothing into the state hash while empty or idle; new `CombatEvent` and `MoveSpec` fields are appended.
- **Byte-identical, every PR:** the 20 `EXPECTED_HASHES` (13 legacy, 2 DEATH-D, 5 llama), `GOLDEN_WALK_RIGHT_400`, the story hash, the party goldens, the 13 boss fixtures, the `q10_*` goldens, `demo01_*`, the CINE goldens, the whole-route autopilot hash, every `cf02_*`, `companion_*` and Engineer fixture, the ART-INT-02 anim traces and the PLAYTRACE goldens. Each PR lists the current counts from `main` in its body.
- **`--carl-prog` alone** on the opening route, the DEMO-01 hall and the CF02 microarena must reproduce the unflagged hashes (no pin, no trigger reachable there).
- **No golden moves on purpose.** New behaviour gets new `carl_prog_*` fixtures with committed final hashes, replayed twice, headless and in the render loop.

## 9. Acceptance tests per milestone

Fixture names are in the `carl_prog_*` namespace. Every fixture asserts exact ticks and a committed final hash.

**M0 unlock model**
- Unit: a grant is idempotent; a defeat retry and a checkpoint respawn keep it; a fixture restart restores the pin; the ledger hashes nothing while empty; the data file is pinned; no engine identifier names a franchise item or character.
- `carl_prog_flag_parity`: `--carl-prog` on the opening route, DEMO-01 and the CF02 arena reproduces the unflagged hashes.
- `carl_prog_grant_survives_respawn`: a zone stub grants `special_thrown_charge`; Carl dies and respawns at the checkpoint and still holds it; a fixture restart removes it.
- `carl_prog_juicer_grants_fist`: under the boss pin, the boss defeat emits exactly one `CapabilityGranted { fist_reinforced }` on the defeat tick.
- CLI: Carl flags with `--scene combat_lab` are refused; the reset descriptor records them; an older recording without the fields loads as false.

**M1 and M2 base kit (no new code)**
- `carl_prog_special_locked`: 80 ticks of Circle/B before any grant emit no event and hash like the same run without the flag; the socket stays a padlock.
- The existing `cp01_*`, `cp02_*`, `cp03_*`, `cf02_*`, llama and Engineer fixtures stay byte-identical (they are the teaching evidence of M1 and M2).

**M3 dynamite grant**
- `carl_prog_story_event_grants_dynamite`: a stub `swarm_boss_f01_defeated` event grants the capability once; the socket turns equipped on that tick and the notice appears.

**M4a dynamite core**
- `carl_prog_dyn_arc_standing` and `_moving`: spawn on t41, landing x and tick exact, detonation on release + 150.
- `carl_prog_dyn_one_active`: presses during flight, fuse and recharge are rejected; a press on detonation + 419 is rejected and on + 420 accepted.
- `carl_prog_dyn_core_kill`, `_outer_launch` (a 2 HP goblin at 40 px is launched alive with 1 HP), `_edge_64` (exactly 64 px is reached, beyond is not).
- `carl_prog_dyn_self_damage` (1 HP and a knock inside 64 px, nothing outside), `_no_friendly_fire` (Donut on the stick, solo and 2P: no damage, launch or event).
- `carl_prog_dyn_los_wall`, `_hit_drops_stick` (t10 drops it, t3 cancels), `_airborne_rejected`, `_swap_keeps_stick`, `_retry_clears`, `_regroup_chord` (L1 + Circle never throws), `_boss` (4 damage, super armour holds).

**M4b interactions**
- `carl_prog_dyn_chain_charge` (an authored charge in reach blows exactly 8 ticks later; depth cap 4), `_arms_trip_charge`, `_breaks_shore` (the block falls and its kill is credited to Carl's player), `_bowling` and `_wall_slam` (under `--cf02` rules), `_rack_strut` (boss arena drop credited).

**M5 Juicer and gauntlet**
- `carl_prog_juicer_dynamite_clear`: a scripted solo clear under the pin lands inside the boss spec's 2400..7200 ticks; `boss01_full_clear_solo` without flags is unchanged.
- `carl_prog_fist_stagger` (16 and 28 ticks), `_pierce_goblin` (a haymaker on attack tick 25 cancels the swing; without the gauntlet the same input lets the impact land), `_pierce_engineer`, `_boss_poise` (super armour holds, poise doubles), `_shore_combo` (jab + haymaker break a fresh shore), `_kick_unchanged`.

**M6 final boss (later work):** a load-time test that the final-boss scene holds no trigger; full-clear fixtures in solo and 2P with no `CapabilityGranted` event.

**GIF evidence** (real Bevy, 640x360 and 640x400, 1x and 3x): standing throw; moving throw; outer launch into a wall slam; self-damage; chain into an authored charge; shore broken by a blast; gauntlet pierce of a swing; one-combo shore break; the grant notice and socket. Plus one solo and one 2P recording of the Juicer with dynamite, a frame-time note with gore preset 4, and a real two-pad check or **NOT RUN**.

## 10. Horizon fit and tone

Consulted [the character progression horizon](../01-design/character-progression-horizon.md). The design follows its Floor 01 path: punch, then kick, then a story-earned explosive, then an enhanced fist, with the Ball of Swine as a mastery test with no new ability. It adds no button (Circle/B was reserved for this), no inventory (ADR-0010) and no Floor 2+ ability. It gives the player new choices rather than bigger numbers: where to place a delayed blast, when a committed swing becomes an opening. It works the same in solo and 2P; the AI does not throw in v1. Pinning the two acquisitions to defeat events makes the horizon's step 3 and step 6 concrete without changing the direction, so the guide is **not edited**. Tone: Carl lights the stick like a man who has burned his fingers before, and a heavier fist does not turn him into an action star.

## 11. Risks and open items

- **Readability:** a 64 px reach with a 20 px fireball can read as unfair. Owner game feel; `burst_large` is the art fix.
- **Self-damage lethality** at 1 HP (CP-Q23) may feel harsh; it is one knob.
- **No save file:** "persistent" means run-persistent until the campaign save exists (CP-Q07).
- **Book chronology:** the acquisition chapters are unverified in repo sources (section 2); the owner can confirm them against the EPUB without blocking any PR.
- **Shared hot files** with #172 and #173 (`src/combat_scene.rs`, `src/combat.rs`, `src/bin/floor01_scene.rs`, `src/lib.rs`, PLAYTRACE): serialize merges, see the work order.
