# Floor 01 boss encounter: the Juicer (HUD-DEBT-03), design spec

**Status:** design adopted by recommendation under the owner's design-defaults directive (2026-11-19). Not implemented. Game feel needs the owner's playtest. The keys in section 8 are drawn and frozen pending the independent review. All numbers are data knobs (section 6), not approved balance.

**Scope:** the boss, arena, moveset, HUD wiring, death, data, tests and asset routing for one encounter. The production pixels, Bevy code, validators and evidence go to execution work orders. Sources: [late enemy and boss concepts](floor-01-late-enemy-boss-concepts.md), [JUICE-01](juice-01-spec.md), [Juicer gym concept](floor-01-juicer-gym-standalone-concept.md), [HUD native spec](hybrid-hud-a-native-spec.md) 2.3 and 6, [progression bible](../01-design/floor-01-gameplay-progression-v0.md), [controller contract](../01-design/mvp-controller-contract.md), [death proposal](../03-tech/monster-death-system-proposal.md), [Death Lab](../../tools/art-references/death-lab/README.md), and the party design (`docs/01-design/party-2p-open-questions.md` on `feat/party-2p-full`).

## 1. The boss

**Choice: the Juicer.** The owner approved three Floor 01 boss concepts. The Hoarder needs the Scatterer swarm, and her tragedy as a transformed woman sits badly with a level-4 gore finish. The Ball of Swine needs rolling physics and moving ring walls, and the horizon calls it the mastery test, not the first boss. The Juicer is one heavy melee body. It fits the current per-hero melee model, and its readability set (JUICE-01: six frozen keys, 16 frames) already exists. The horizon calls it the cooperative climax.

| Item | Decision |
| --- | --- |
| Data id (IP-neutral) | `brute_boss_f01`. Species profile `brute_boss`. Engine code never names the Juicer. |
| Display name (fan layer) | `THE JUICER`, from data. It is replaceable. Never "Warlord Grizzog". |
| Originality | The pixels are original rule-drawn art from the owner-approved concept, not traced. The moves (pound, wall-crash charge, trap racks) are generic action grammar and a game adaptation, not book events. No boss design, dialogue or signage is taken from another game, film or franchise. |
| Size | 100 px idle height (JUICE-01, about 1.9 times Carl). Frame 160 x 136, pivot (72, 135). The book says "a little taller than Carl"; the mismatch is recorded in JUICE-01. |
| Silhouette | A hunched, wide block with a small sunk head, rope dreads, a pointed ear, two huge fists, wrist chains and a waist chain. |
| Palette | The 19 JUICE-01 colours (`trog-family-v0+juicer`). No new colour. No steroid, "juice" or aura effect. |
| Tone | The late-80s action heavy who stayed in the weight room too long: lumpy, diseased, drooling and absurd up close, deadly at range. The heroes win scrappily, with bait, timing and a dropped weight stack, not by out-muscling him. |

## 2. The arena

- **Route:** the Juicer's gym, beat 8 of the progression table and horizon step 6, before the Ball of Swine. The beats from the Guild to the labyrinth do not exist yet, so it ships as its own entry (`floor01_scene --scene boss_f01` and the fixtures in section 7). The route links it later through the gym door.
- **Layout:** one locked screen, 640 px wide. In the 640 x 400 view the extra 40 px are ceiling. The walls are at x 0..32 and 608..640, and the floor between them is 576 px, flat, with no pits. The ceiling is a solid collider **224 px above the walk line** at both canvases. The tell fists reach 128 px. The ceiling stops every hero top at screen y 72 or lower at 360 (walk_y 296), which keeps it out of the HUD band y < 60. At 400 the extra 40 px are masonry above the ceiling. The walls are faced with cracked mirrors (cosmetic) and are the crash surfaces. The checkpoint is at the entrance.
- **Entrance gate (2P- and solo-safe):** the gate closes only when **every living hero** stands at x 40 or more and 8 ticks have passed since the first hero crossed the engage line (x 96). In 2P it waits for P2 indefinitely, and the boss stays dormant until then. In solo, if the AI hero is still outside 120 ticks after the controlled hero crossed, the follower regroup places it at x 48 before the gate closes. `BossEngaged` fires on the closing tick. The gate opens on defeat, and the exit on the right unlocks then.
- **Two plate racks**, one per hero (asymmetric gates, [traversal rule](../01-design/asymmetric-traversal-and-gates.md)):
  - **Left rack (Carl, on by default):** a loaded plate stack hanging 189 px up (its underside), held by a rotten timber strut at floor x 200. It falls after 3 hits from separate melee moves, the CP-03 shore rule. Hits persist through the attempt, so Carl can pre-weaken it and then bait the boss. The fall takes 36 ticks, which is free fall from 189 px at gravity 1050, the same as CP-03.
  - **Right rack (Donut, data-gated: `rack.right.enabled = false`):** this rack waits for the higher cat jump of Q10 (500 px/s, apex 119 px), which is deferred. Today Donut jumps like Carl (440 px/s, apex 92 px). When it lands: a release plate on a ledge 104 px up at x 432..480 (Carl's margin below it 12 px, Donut's over it 15 px). Standing on the plate for 12 ticks drops the stack. Donut's top on the ledge is 104 + 52 px (the shared collider), and a jump from there hits the ceiling, so nothing reaches the HUD band. In P3 she has to be on the ledge before the crash: 16 ticks of rise, 12 dwell and a 36-tick fall make 64, over the 60-tick stun. **The default fight is complete without it**: the left rack plus direct combat.
  - **Crash pivot and footprint:** the boss stops when the ram front (+82) meets the inner wall face, then rebounds 16 px: `crash_pivot = wall_x - facing * (82 + 16)`. That puts the pivot at x 130 on the left (wall 32) and x 510 on the right (wall 608). The stunned body spans -36..+50 from the pivot, so its centre sits at `crash_pivot - facing * 7`. Each footprint is 80 px wide and centred there: left x 83..163, right x 477..557. A crush happens when the footprint overlaps the boss hurtbox. Each rack works once per attempt and resets on restart.
  - **Bait position (left rack):** stand at x 170..196, between the footprint and the strut, with the boss to the right. Jump the charge in place. The boss crashes at the left wall, and Carl lands outside the footprint (x 164 or more) and next to the strut: his jab box +16..22 reaches x 200. The third strut hit plus the 36-tick fall is about 41 ticks, inside every stun (60 or more).
- **Camera:** locked to the arena centre in solo and 2P. The gate rule above keeps every hero inside, so every actor is always on screen and safeguard M4 never triggers. No zoom.
- **HUD clearance:** the tell's peak sits at walk_y - 128, inside the action lane (y 60 or lower). The boss strip is at y 8..34.

## 3. Moveset and phases

Ticks are at 60 Hz. The boss has **36 HP**. Hero hits deal 1 each: jab, haymaker and kick (the kick never launches the boss). The design is beatable with Carl's base kit and an AI Donut that never attacks (Q10 deferred). Later kit plugs in through data: Magic Missile 1, dynamite 4 (explosive cause), and Donut's swat counts 1 toward poise but deals 0 HP.

| Move | Tell | Active | Damage to a 4-HP hero | Recovery / punish | Notes |
| --- | --- | --- | --- | --- | --- |
| **Pound** (JUICE-01 frames 7..11) | 36 (rise 8 + roar hold 28) | smear 3, impact 6 (ticks 39..44) | **2**, launch 160 / 200 px/s | recover **24**: free hits | Box +32..67 px ahead, 0..22 px up. Triggers on a target 30..72 px ahead. |
| **Shockwave** (P2+, from the pound) | the pound's tell | 2 ripples, 150 px/s, 120 px each way, 10 px tall | **1**, small hop | jump it | VFX order. No body key. |
| **Charge** | 40 / 36 / 30 by phase (K-A `charge_tell`) | runs at 300 px/s in the direction locked at tell start, until a wall (K-B, K-B2 cycle) | **1**, launch 220 / 180 px/s | **crash stun 90 / 75 / 60** (K-C) | Ram box +36..82, **0..72 px up**, so it hits a standing Donut's 20 x 24 box too. The body behind it never hurts and is not solid while he runs. Needs the target 128 px or more away and 160 px or more of run room. Cooldown 240 / 180 / 150. |
| **Double pound** (P3) | 36, then a 12-tick step and a 24-tick second tell | as the pound | 2 | 24 after each | The first recover is still a punish window. Leave before the second impact. |
| **Phase roar** (K-D `phase_roar`) | none (60 ticks, no damage) | push wave at tick 0: heroes within 72 px are pushed 48 px back | 0 | none | Super armour, but he takes full damage: no invulnerability gate. |
| **Flinch** | none | none | 0 | 14 (JUICE-01 `hurt`), then a 24 px backstep | Poise: 4 damage in neutral within 120 ticks. |
| **Rack crush** | 36-tick fall: shudder, dust and a floor shadow | landing tick | heroes in the footprint take 1 | boss: **4 HP + pinned 90** (K-E) | Catastrophic force. Credited to the player who dropped it. |

**Jump-over check (charge, both heroes):** the ram box (44 px wide) crosses a 24 px hero in 68 px / 300 px/s, about 14 ticks. With today's shared jump (440 px/s, gravity 1050), the feet stay above 72 px for about 23.5 ticks, so **Carl and Donut each have about 9 ticks of slack**. Takeoff has to come 14..22 ticks before the ram box arrives. With the Q10 cat jump (500 px/s), Donut's window grows to about 31 ticks.

**Body collision and hurtboxes (pivot-relative, x forward, y up):** the boss hurtbox and the solid body box are the same box per mode. Standing, walk, tell and roar: x -24..44, y 0..100. Charge tell, pound strike and recover: -20..60, 0..76. Crash stun: -36..50, 0..64. Pinned: -60..64, 0..30. While he runs the body is not solid (the ram box decides). After hero physics, every tick, a hero whose box overlaps the solid body box and whose feet are below the box top is pushed out to the nearer edge plus 1 px, at most 6 px per tick. If that side has less than 28 px of room before a wall, he goes to the other side. No hero stands inside him, and nobody walks through him: the front edge (+44) plus Carl's half-width keeps Carl 56 px or more from the pivot, inside pound range. Getting behind him takes a jump over a low pose (stun, pin) or the charge.

**Super armour:** in tells, strikes, the charge and the roar, hits deal damage and hit-stop but never interrupt. Neutral hits add to poise. Recovers, stuns and pins take free damage.

**Phases (by HP, knobs; a crossed threshold queues the roar until the current mode ends, and never shortens a pin, stun, charge or attack):** P1 *warm-up*, 36..25: pound and charge. Roar at 24 or less. P2 *use the room*, 24..13: the pound adds shockwaves, charges come faster and stuns shorten. Roar at 12 or less. P3 *no rest*, 12..0: the double pound and a 60-tick stun. Racks persist across phases. Fairness floors: **no tell under 24 ticks** (the goblin baseline) and **no punish window under 24 ticks**.

**Action choice:** rule-based, with no RNG: a target 30..72 px ahead means pound, a charge-eligible target means charge, otherwise walk at 72 px/s. The walk tag plays at 16 ticks per frame, so the feet keep JUICE-01's 19..20 px per step. He turns toward a target behind him in 12 ticks.

**Targeting (party design Q3 B):** he targets the nearest *reachable* hero, with 24 px hysteresis. The target is locked from tell start to the end of recovery, and the tell faces it. In solo the AI hero counts 40 px farther. Reachability is a surface test: a hero is unreachable only while standing on a surface other than the arena floor, or airborne above one (the ledge). A hero who is airborne over the floor stays reachable, so a jump never changes his target and never turns him. With no reachable target he walks to the centre and growls.

**Solo fairness (AI companion):** M1 applies: the AI hero takes hits but never drops below 1 HP. The bias (M2) applies. The follower adds boss dodges to M3: it steps out of a pound tell aimed at it within 72 px, and it jumps when the ram box will reach it in 18 ticks (`ai_charge_jump_lead`, inside the 14..22 takeoff window). The AI never triggers a rack. The player swaps to use the right rack once it is enabled. **2P:** no guards (M5). Donut at 0 HP is party defeat (D1). **Restart:** after the 50-tick delay both heroes are back at full HP at the checkpoint, the boss is at 36 HP, and the racks and strut reset.

## 4. HUD integration (`BossView`, spec 2.3 and 6)

- **Source:** the simulation's `BossState { hp, hp_max, mode, phase }` is read through `RealHudInputs.boss`. `fill_px = ceil(234 * hp / hp_max)`, so it is at least 1 px while he is alive and 0 at defeat. The chip comes from `BossHurt` (30 ticks hold, then a 2 px/tick drain; HUD-only). The bar ticks at 25/50/75 % stay reading aids. The phases are at 2/3 and 1/3 on purpose.
- **Name:** `display_name` from the boss data. **Emblem:** the 18 x 18 key `brute_boss_f01_emblem` replaces the generic skull plate (`BossView.emblem: Option<EmblemId>`). This amends [HUD spec](hybrid-hud-a-native-spec.md) section 6 and asset row 8, where the emblem is 18 x 18 inside the 20 x 20 well.
- **Warning icon:** `warning = mode is PoundTell, SecondPoundTell or ChargeTell`. That is true on exactly the tell ticks: never during the roar, the strikes or the rack fall.
- **Slide:** in on `BossEngaged` (12 ticks). Out 90 ticks after the boss is defeated. Removed at once on `Restart`, then slides in again on re-engage.
- **Notices** (System panel, at most 13 chars, existing queue rules; copy pending the owner's tone check): engage `MAIN EVENT!`, P2 `ROUND TWO`, P3 `FINAL ROUND`, rack crush `NO SPOTTER!`, defeat `KNOCKOUT!`. `BOSS INCOMING` stays fixture-only.

## 5. Death and finish (death language, Death Lab intensity 4)

The defeat is the existing `EnemyDefeated` event with a cause and a force tier. Presentation is chosen from the `brute_boss` profile, never stored in the simulation:

| Lethal cause | Force | Finish id | Staging |
| --- | --- | --- | --- |
| Melee | routine / heavy | `brute_ko_timber` (fallback) | JUICE-01 `hurt`, then `down` 13..15 (falls back like a felled tree). On the floor-impact frame: camera kick x2, a 120 px dust burst, the racks rattle, and at preset 4 the boils burst in red spatters with a spreading pool. No slow motion or cinematic pause. Play continues. |
| Rack crush | catastrophic | `brute_crushed` | K-E `pinned` with the `dead` face under the landed stack. At preset 4: a pool from under the plates, boil spatter, and one fist twitch at +20 ticks. |
| Explosive (later dynamite) | catastrophic | `brute_blown` | Needs a body-parts sheet, which is a separate order. Until then it falls back to `brute_ko_timber`. |

Presets 1 to 3 reduce the presentation only. The replay hash is identical at presets 1 and 4. The corpse stays, without collision or damage.

## 6. Data and tuning (`assets/data/bosses/brute_boss_f01.json`, embedded like the death profiles)

| Knob | Value | Knob | Value |
| --- | --- | --- | --- |
| `hp_max` | 36 | `phase_thresholds` | [24, 12] |
| `walk_px_s` / `turn_ticks` | 72 / 12 | `poise_damage` / `poise_window` | 4 / 120 |
| `pound.tell` / `smear` / `impact` / `recover` | 36 / 3 / 6 / 24 | `pound.damage` / `range_px` | 2 / [30, 72] |
| `pound.box` | x 32..67, y 0..22 | `pound.launch_px_s` | [160, 200] |
| `shockwave.{speed, reach, height, damage}` | 150, 120, 10, 1 | `shockwave.from_phase` | 2 |
| `charge.tell_by_phase` | [40, 36, 30] | `charge.speed_px_s` | 300 |
| `charge.box` | x 36..82, y 0..72 | `charge.damage` / `launch` | 1 / [220, 180] |
| `charge.min_target_px` / `min_room_px` | 128 / 160 | `charge.cooldown_by_phase` | [240, 180, 150] |
| `crash.stun_by_phase` | [90, 75, 60] | `crash.rebound_px` / `ticks` | 16 / 6 |
| `double_pound.step_ticks` / `second_tell` | 12 / 24 | `roar.ticks` / `push_radius` / `push_px` | 60 / 72 / 48 |
| `rack.fall_ticks` / `boss_damage` / `pin_ticks` | 36 / 4 / 90 | `rack.hero_damage` | 1 |
| `rack.left.enabled` / `rack.right.enabled` | true / **false** (until Q10's jump lands) | `rack.footprint_centre` | `crash_pivot - facing * 7` |
| `rack.strut_hits` / `ledge_height_px` / `dwell_ticks` | 3 / 104 / 12 | `rack.footprint_px` | 80 |
| `target.switch_px` / `ai_bias_px` / `reachability` | 24 / 40 / surface test (section 3) | `fairness.min_tell` / `min_punish` | 24 / 24 (validated on load) |
| `hud.display_name` / `emblem` | `THE JUICER` / `brute_boss_f01_emblem` | `exit_slide_delay` | 90 |
| `body.push_px_per_tick` / `min_side_room_px` | 6 / 28 | `ai_charge_jump_lead` | 18 |
| `arena.ceiling_px` / `gate.inside_x` / `gate.regroup_ticks` | 224 / 40 / 120 | `phase.queue_roar` | true |

Load-time validation rejects a tell or punish under the fairness floors, thresholds out of order, or unknown fields.

## 7. Determinism and test plan

- **Domain:** a Bevy-free `src/boss.rs`. Rule-based, with no RNG draw. The boss state (mode, tick, hp, target, poise, rack and strut state) is mixed into `COMBAT_STATE`. VFX, gore, the HUD and notices are never mixed in. Boss steps run after the hero strikes in the scene tick order, so Carl wins a same-tick trade, as with the goblins.
- **New events (hashed):** `BossEngaged`, `BossTell{move, target}`, `BossStrike{move}`, `BossCrash{wall}`, `BossPhase{phase}`, `BossHurt{source, damage, hp_left}`, `RackArmed`/`RackDropped{rack, player}`, `RackLanded{rack}`. `HeroHurt` carries the boss id. `DefeatCause::DropHazard{player, rack}` generalises `ShoreBlock`, as a documented hash migration.
- **Unit tests:** every tell is at least 24 ticks and every punish at least 24; the target stays locked through an attack and the 24 px hysteresis works; a hero on the ledge is ignored, and Carl jumping in neutral never changes the target; a hero inside the body box is pushed out (and never into a wall); the ram box hits a standing Donut's 20 x 24 box; a threshold crossed during a pin, stun or charge queues the roar and never shortens it; the gate stays open while any living hero is at x < 40; each threshold gives exactly one roar; poise triggers on the 4th neutral damage; super armour never interrupts; the wall crash happens on both sides; the footprint is centred on the crash body and covers it on both walls; `fill_px` maps 36 to 234 px, is 7 px or more at 1 HP and 0 at defeat; the warning flag is set on exactly the tell ticks; data validation.
- **Fixtures (`--fixture`, headless and in the render loop, final hash committed):** `boss01_pound_punish`, `boss01_charge_jump`, `boss01_charge_hit`, `boss01_hug_guard` (Carl walks into the body: pushed out, then pounded), `boss01_rack_carl` (pre-hit twice, bait, third hit), `boss01_rack_donut` (P2 script; **pending**, with its tests, until `rack.right.enabled`), `boss01_phase_roar`, `boss01_roar_queued` (a rack crush crosses 24 during the pin), `boss01_2p_gate` (P2 lags at x 20; the gate waits), `boss01_solo_ai_guard`, `boss01_party_defeat_retry`, `boss01_crush_kill`, `boss01_full_clear_solo` (Carl's base kit only, no racks, AI Donut never attacks: proves the fight can be won alone), `boss01_full_clear_2p`. Each full clear asserts a clear tick inside **2400..7200 (40..120 s)**. The playtest target for a first clear is 60..120 s.
- **Goldens:** `COMBAT_EVENT` logs and `COMBAT_STATE` hashes per fixture. The hash is the same headless and rendered, the same at presets 1 and 4, and the same with and without `--hud-preview`.
- **Evidence:** real Bevy at 640 x 360 and 640 x 400, 1x and 3x. A GIF per move (tell, strike, recover), HUD strip captures (fill, chip, warning, emblem), one solo clear and one 2P clear recording, and a frame-time report with gore preset 4.

## 8. Assets and difficulty routes

**Keys produced in this design step** (one tree: the encounter set lives under `assets/source/enemies/brute_boss_f01/` and imports the JUICE-01 base set from `assets/source/characters/troglodyte/juicer/`; keys in `keys/`, generated by `scripts/author_boss_f01_keys.py` on the JUICE-01 rig; previews in [`docs/evidence/floor01-boss/`](../evidence/floor01-boss/)):

| Key | Tag | Opaque h | Pose (facing right) |
| --- | --- | --- | --- |
| K-A | `charge_tell` | 80 | Lineman crouch: far knuckles on the floor ahead, near fist cocked behind the back line, far foot scraping back, head dropped forward. |
| K-B | `charge_run` | 90 | Ram, contact A: trunk pitched about 40 degrees, hump and shoulder leading, far fist driven ahead, far knee up, near leg bent and pushing off on the ball of the foot, hair streaming. |
| K-B2 | `charge_run_b` | 92 | Ram, contact B: near knee up and forward, far leg pushing off, trunk 2 px higher, near fist low under the head, far fist pulled back. |
| K-C | `crash_stun` | 72 | Sat down hard: near leg straight with an upturned foot, far knee up and foot flat, trunk slumped back, head lolled with the new `dazed` face, fists on the floor. |
| K-D | `phase_roar` | 105 | "Most muscular" crab flex: elbows out front and back, fists together at the gut, full roar. The fists stay low, never the pound tell. |
| K-E | `pinned` | 56 | Face-down under the stack, with a level back for the plates; near arm splayed forward with an open hand, far hand clawing up, far leg kicking with the sole up. The lethal crush swaps in the `dead` face. |
| Emblem | `emblem` | 18 x 18 | Head in profile: mop, ear, red-rimmed eye, roaring mouth, a brow boil. |

| Asset | Score (1 new, 2 >40 px, 3 complex, 4 VFX) | Route |
| --- | --- | --- |
| Boss encounter set: in-betweens for K-A..K-E (tell 2, run passing 2, crash 2, roar 2, pin 1; about 9 frames) plus the face swap | boss | **A + silhouette gate** on K-A..K-E and K-B2 (all keys done; the run's two contact keys are top tier). The medium tier does the in-betweens, atlas, manifest and validators |
| HUD emblem | 1 | **C**: keyed. The medium tier wires it into the HUD atlas |
| Pound debris + shockwave ripple VFX | 2 (1, 4) | **B + gate** |
| Crash dust + mirror crack VFX, roar push ring | 2 | **B + gate** (one order) |
| Plate rack prop (hanging, shudder, falling, landed) | 2 (1, 3) | **B + gate** |
| Rotten strut (3 damage states), ledge and release plate | 1 | **C** (extends the CP-03 shore kit) |
| Gym arena kit (AREA-GYM-01) | env | Its own environment order. A greybox catacomb kit with mirrors is enough for the first playable |
| Finishes `brute_ko_timber`, `brute_crushed` (boil spatter, pool) | 2 | **B**, from the Death Lab grammar |

## 9. Art acceptance targets (independent review)

1. At 1x in the lit Floor 1 scene, each new key reads as the same JUICE-01 creature: head, mop and ropes, ear, boils, veins, cuffs and chains, two countable fists.
2. In the silhouette sheet, no new key can be mistaken for the pound tell or impact. K-A shows the cocked arm behind and the knuckles ahead. K-D keeps both fists below the chest line.
3. Opaque heights are within 4 px of the table in section 8. Every key sits on contact row 135. At least 1 px of margin to the canvas edge.
4. Only the 19 palette colours, no orphan pixels, closed outline. The family's open mouth may break the front outline (`o` on the edge), as the JUICE-01 roar does.
5. Every leg has JUICE-01 thigh and shin proportions and ends in a foot (heel and toe block, dirt-caked sole). No tapered spike legs.
6. Tone: lumpy, pitiful and grotesque, never a sculpted heroic physique. K-D reads as a parody flex.
7. The emblem reads as the Juicer in profile at 1x (brow step, snout, open jaw, mop and ear behind) in the 20 x 20 well against `magenta_dk`.

## 10. Open questions (decided by default, owner may override)

1. JUICE-01's 100 px height is still pending owner confirmation. This design assumes it.
2. Heroes hit by a rack take 1 damage (CP-03 left it off). Default: 1.
3. Notice copy and the boxing framing need a tone check.
4. The gauntlet and Puddle Jumper are awarded after the fight. That is a separate design, out of scope.
5. Book chronology check of the heroes' kit at the gym (dynamite, missile). The fight is kit-tolerant, so it is not blocking.
6. The right rack ships disabled until Q10's higher cat jump is adopted and tuned. Default: enable it in the same PR that lands the jump, with `boss01_rack_donut` un-pended. Until then Donut's role is to survive and draw aggro.

**Horizon fit:** fits [horizon](../01-design/character-progression-horizon.md) step 6. There are no new buttons or verbs: jump, primary and physical only. It uses Donut's higher reach and Carl's breaking as optional, asymmetric options. Direct combat stays a complete route. Rewards stay after the encounter. No guide update needed.

**Depends on:** #114, merged (per-hero combat, party targeting and lock, the follower bot with M1 to M3, D1); Q10's jump for the right rack only; the CP-03 hazard generalisation, DEATH-B/C finish selection, and JUICE-01 passing review.
