# ENEMY-GAMEPLAY-ENGINEER — the Goblin Engineer as a real enemy (LAB-04 design sheet)

> **Status: design adopted under the owner's design-defaults directive (AGENTS.md, 2026-11-19); revised after independent review round 1 (2026-11-21, [section 18](#18-review-round-1-resolutions)); implemented behind `cf02_rules` + `--cf02-engineer` (section 19), NOT game-feel accepted.**
> Every choice below has one adopted value with an `ENG-G-Q##` ID ([section 12](#12-adopted-defaults-eng-g-q01eng-g-q27)). Each value is a tuning hypothesis stored as data. The owner reviews game feel after the implementation merges and may change any value in a follow-up PR. Only [section 13](#13-genuinely-open-questions) waits for the owner.
> Design: Claude Opus (top tier). Execution: medium tier, see the [implementation brief](#14-implementation-brief-medium-tier-executor). Review: a separate top-tier agent.

**Why:** GitHub issue #41 (goblin trio and Murder Dozer) is done as art: GOB-01 goblins A/B (#44, in the Bevy scene), ENG-01 Engineer readability set (#86, loaded only by the camera lab, no gameplay; the point tell and pot ram are placeholders) and DOZ-01 (#47). The owner wants more enemy variety in the next playtest. This sheet turns the Engineer into an enemy under the [LAB-04 new-enemy rule](../07-agents/combat-lab-01-workorder.md#lab-04-new-enemy-rule) and LAB-Q14 (a separate `ENEMY-GAMEPLAY-<name>` item that reuses a merged readability set, not new art).

## 1. Authority and locks

- [ENG-01 spec](../02-art/eng-01-spec.md): 40 px, frame 64 x 56, pivot (28, 55), 18 frames; tell = the DOZ-01 rider's **point**, attack = a head-down **pot ram**, recover = the pot slid over the eyes. Section 2 lists what is not approved.
- [Floor 01 opening direction](../02-art/floor-01-opening-catacomb-goblins.md#first-goblins--selected-concept-direction): the wrench is non-canonical shorthand; **ranged bombs, buffs, in-combat repairs and class perks are not invented here**. This design uses only the ram the readability set already shows. No wrench use, no projectile, no mine or turret, no buff to other goblins, no repair, no drop.
- [COMBAT-FUN-02](combat-fun-02-post-demo01-proposals.md): the owner found fights repetitive, impacts inconsequential and goblins identical. Its boundaries apply: no new button, deterministic 60 Hz, seeded variety only, everything off outside `--cf02`, no health sponges.
- [COMBAT-LAB-01](combat-lab-01-advanced-playtest.md#enemy-roster-and-gating): a new archetype stays only if it **visibly changes a tactical choice**; it is added one at a time with an A/B comparison; at most 4 enemies active at once (LAB-Q04).
- [Antihero tone](antihero-tone.md): enemies are threatening yet eccentric, with motivated behaviour and readable tells.

## 2. Tactical role

**The Engineer is the goblins' mid-range enforcer. From up to 72 px away he points at a grounded hero, then charges up to 80 px along the floor head-first behind his saucepan, which reaches a standing hero up to 115 px from where he started. The ram punishes a hero who is busy at middle distance. A real jump beats it, and so do getting out of range early or hitting him during the point if you are already close (rushing in from further out makes him launch early). It can be baited into a wall, the parapet or his own crew, which leaves him dazed with the pot over his eyes.**

| | Goblin A/B | Bad Llama | Engineer |
| --- | --- | --- | --- |
| Range | Close: tell at 30 px, swing reach 8..30 px, in place | Long: spit at 42..48 px height over the yard | **Middle: tell at 72 px or less, ram travels up to 80 px, a standing hero is hit out to 115 px** |
| What it punishes | Standing next to it; a whiffed move in front of it | Standing in the open at height 42..48 | **Being committed (in a move, in hurt, busy with a goblin) at middle distance on the floor; walking straight at him while he points** |
| Main answers | Jab and kick first, bail-out hop, swat the tell | Jump, duck behind the parapet, a perch, Donut stays low | **Jump over it (the bail-out hop is too low and only escapes by distance), get out of range early, hit him during the point if you were already inside 48 px when he began, or stand in front of an obstacle and jump (matador)** |
| What it adds to the room | Pressure, morale, pincer | Area denial, cover choice | **A lane: where you stand relative to walls and goblins matters; a punish window you earn by dodging; hug him during his cooldown, never charge his point** |
| Body | 24 x 38, HP 2 | 46 x 39, HP 3, leashed to its yard | 24 x 38 (40 px art), **HP 3** |

Why it is not a duplicate: goblins threaten the space **next to** the hero and the llama the space **above the floor**; the Engineer threatens a **horizontal lane on the floor at middle distance**. The bail-out hop (CF02-Q54: apex 30 px at 60 Hz, below the ram box top of 33 px) never clears the ram by height. Within 40 px, or when it hops toward him, it is hit; it escapes only by distance, away from him and early enough ([reach table](#reach-table-carl-standing-or-jumping-in-place)). A busy Carl whose jump press becomes the bail-out (the CF02-Q49 C3 cancel) therefore has to choose: bail away early, or finish the move and jump for real. That is the one-at-a-time A/B claim the playtest must confirm ([section 11](#11-acceptance-criteria)).

## 3. Player-readable tell

1. **Approach:** he walks at 40 px/s, slower than goblins (62), pot rattling in idle (ENG-01 frames 0 to 2). Slow and stubborn: a foreman, not a runner. He stops 72 px from his target (the standoff) and never walks backward.
2. **The point (32 ticks):** rear (frame 9, 8 ticks), then a level arm pointing straight at the hero with the head and pot pulled back (frame 10, held 24 ticks). This silhouette is unique in the roster: no goblin raises a straight arm at shoulder height, and ENG-01's silhouette gate requires frame 10 to differ from every other frame. **He points at the hero he targets**; in 2P that tells each player who is targeted.
3. **The snap (ENG-G-Q27):** if the hero he points at comes within **48 px** of him during the point, having been farther than 48 px when the tell began, he cuts the point short and launches on the next tick. "Don't you dare." Charging his point gets you the pot. A hero who was already inside 48 px when he began is not snapped: that hero may hit him.
4. **Launch (3 ticks, frame 11):** arm swings back, head drops. **Committed** (like the goblin smear). The last instant to start a clearing jump depends on the distance ([reach table](#reach-table-carl-standing-or-jumping-in-place)): at the standoff it is ram active tick 5; point-blank it is attack tick 30, two ticks before the launch.
5. **Ram (frame 12):** the lowest standing frame and the only one with the pot ahead of the snout.

Audio: the existing goblin tell cue at the start of the point (no new sound asset in v1, ENG-G-Q23). No ground decal or other new VFX: the pose is the tell.

## 4. Moves and frame data (60 Hz, all data knobs)

Attack tick 0 = the tell starts. Pixels from the pivot (x forward, y up from the feet). Speeds are whole pixels per tick (240 px/s = 4 px per tick), so every number below is exact.

| Phase | Attack ticks | Frames | Motion | Boxes | Interrupt |
| --- | --- | --- | --- | --- | --- |
| Tell: rear | 0..8 | 9 | none, facing and target locked at tick 0 | hurtbox = body | Any hero hit staggers him and cancels the attack (token returned), like a goblin tell; Donut's swat too (Q10 light guard rules) |
| Tell: point | 8..32, or up to the snap | 10 | none | body | as above; the snap ends the point early |
| Launch | 32..35 (or snap tick + 1, 3 ticks) | 11 | none | body | **Committed**, as the goblin smear: damage and hit-stop only, no stagger, no kick launch, no dive knockdown; a lethal hit kills him |
| Ram (active) | 20 ticks at most after the launch (35..55 without a snap) | 12 held | **240 px/s** forward (4 px per tick, 80 px at most) | Ram box `[14, 24) x [14, 33)` every active tick; hurtbox = body | **Committed**: damage and hit-stop only (as a goblin swing); a lethal hit or a CF02-B contact knockdown (Q35) ends it |
| Recover (after a whiff, a hero hit or a bowl) | 40 | 13 held | stops dead | body | Any hit staggers him (as a goblin recovery) |
| Bonk (the pot met a solid) | 12 + 48 = 60 | 14, then 13 held | stops at the solid | body | Hits deal damage and hit-stop only; the bonk timer is unchanged. A launch (push kick, follow-through kick, relaunch), a knockdown (dive, stomp, contact) or a lethal hit overrides it |
| Cooldown before the next tell | 60, from the end of the recover or the bonk | idle / walk | approach or hold | | |

An interrupted tell (a hit in the rear or the point) leaves through the goblin's `Hurt` path and its `HURT_COOLDOWN` (20) instead of the 60-tick cooldown.

**One ram active tick, in this order (step first, then test):**

1. **Step:** he moves 4 px forward. If the ram box front (pivot + 24) would pass the face of an intact solid (end wall, parapet face), he is clamped so the box front rests on the face, and the tick is marked `met_solid`.
2. **Hero:** the ram box at the new position overlaps a hero hurtbox (the heroes stepped earlier in this tick) → that hero is hit ([section 8](#8-1p-2p-and-the-ai-companion): the first hero the box overlaps, whoever was pointed at; if both overlap on the same tick, the CRL-Q14 ruling applies: the hero the ram box front passed first, that is the one nearest the Engineer with the largest penetration `dir × (front − near edge)`, exact ties to Carl; see [the llama rush sheet](enemy-gameplay-llama-rush.md)). An invulnerable hero, or a hero protected by the solo off-screen rule (D1 M4), is not hit and does not stop the ram. On a hit: the ordinary hero hurt path `hurt_hero` with `HeroBlow { damage 1, from_x = his x, knock_vx = 120 in the ram direction, knock_vy = 160 }`. The hero gets that path's own values: Carl's `HERO_INVULNERABLE_TICKS` (55) and `HERO_HURT_TICKS` (18), Donut's `donut_invulnerable_ticks` and `donut_hurt_ticks`, the hero hit-stop `HERO_HURT_HIT_STOP` (4) and the solo last-HP guard. He takes hit-stop 4, or `HERO_DEFEAT_HIT_STOP` when the blow defeats the hero, as in `swing_on`. One hero per ram; the ram ends and he enters Recover.
3. **Solid:** (the clamp never moves him backward: when the pot's front already meets the face, as with a hero flush against a wall and him point-blank, there is no motion and he bonks on active tick 1) `met_solid` → **bonk**: he takes **1** self-damage (the Q11 wall-slam damage value), the hero he pointed at is credited, the appended `CombatEvent::RamBonk` is logged and he enters Bonk. At 0 health he is defeated in place by the existing wall-slam defeat path.
4. **Crew:** the ram box overlaps the body of a common goblin for which `can_be_contacted()` is true (`Neutral`, `Attack` or `Hurt`; never a lying, launched, airborne or defeated goblin and never another Engineer) → the goblin gets the existing CF02-B `contact_knockdown` (Q09 mode `knockdown`, Q12 damage 0) pushed in the ram direction; a goblin mid-swing loses the swing (Q35); a `BodyContact` event is logged ([section 14](#rules-the-executor-must-not-re-decide)). He enters Recover.
5. **Distance:** 20 active ticks elapsed → Recover (a whiff).

The ram passes through the llama, under the perches (his 38 px body fits under the 68 px slab underside) and over lying bodies. A hero who lands on top of him causes nothing by landing (the stomp is Down + Physical, not automatic).

### Reach maths

Let `d` be the pivot-to-pivot distance from him to a hero at the tell start and `h` the hero's hurtbox half-width (Carl 12, Donut 10). After active tick `k` the ram box spans `[4k + 14, 4k + 24)` ahead of his start pivot, at heights 14..33. Both heroes' hurtboxes start at the feet (Carl 24 x 52, Donut 20 x 24), so neither can duck under it, and a hero is clear only while the feet are at least 33 px up.

- **Contact on a standing hero:** the first `k` with `4k + 24 > d - h`, so `k = max(1, floor((d - 24 - h) / 4) + 1)`, if that is at most 20. Carl is hit when `d < 116`, Donut when `d < 114`.
- **Jump clearance:** a jump pressed on attack tick `J` has the feet at 33 px or more on jump ticks 6..44 (tick 1 is the press tick; 440 px/s, gravity 1050, apex 88.5 px, 50 air ticks). It clears if the feet are up on every active tick on which the box overlaps the hero's column.
- **Bonk:** with a solid face `F` px ahead of his start pivot, he bonks on the first `k` with `4k + 24 > F`, so `k = max(1, floor((F - 24) / 4) + 1)`, if that is at most 20, and stops with his pivot at `F - 24`. So he bonks only when `F < 104`. A hero standing flush in front of the face at distance `d` gives `F = d + 12 + gap`.
- **Run speed** 185 px/s (3.08 px per tick); **bail-out** 150 px/s back (or toward the stick), apex 30 px, 29 air ticks, landing recovery 4.

#### Reach table (Carl standing or jumping in place)

No snap in these rows: the hero stands still, jumps in place or moves away. Donut has the same contact ticks at every listed distance and jump windows within one tick of Carl's; she is out of reach from 114 px.

| Start `d` | Standing: hit on active tick (attack tick) | Jump in place clears if pressed on attack ticks | After a clean jump he stops at | Running away clears if started by attack tick | Bail-out away clears if started by attack tick |
| --- | --- | --- | --- | --- | --- |
| 12 (point-blank) | 1 (35) | up to 30 | 68 px past the hero | 21 | never |
| 36 | 1 (35) | up to 30 | 44 px past | 29 | never |
| 40 | 2 (36) | 0..31 | 40 px past | 30 | never |
| 48 (snap line) | 4 (38) | 2..33 | 32 px past | 32 | 27 |
| 60 | 7 (41) | 5..36 | 20 px past | 36 | 32 |
| **72 (standoff)** | **10 (44)** | **8..39** | **8 px past: you land behind him** | **40** | **37** |
| 88 | 14 (48) | 11..43 | 8 px short: you land in front of him | 45 | 43 |
| 100 | 17 (51) | 11..46 | 20 px short | 49 | 48 |
| 112 | 20 (54) | 11..49 | 32 px short | 53 | 53 |
| 116 or more | never | any | 80 px from his start | always | always |

At the standoff a whiff ends on attack tick 54 and the recover runs over attack ticks 55..94. A jump pressed on attack tick 8..39 lands on attack tick 57..88, 8 px behind his back. Turning and jabbing (startup 3) lands by attack tick 92, inside the recover, for every clearing jump. This is why the recover is 40 ticks: with 30 ticks, a jump at the end of the window landed after he had recovered. A dive (CF02-Q52) from the descending half of the jump is the faster punish.

**The snap at work:** Carl at 72 px who runs at him from attack tick 8 crosses 48 px on attack tick 15 (47.3 px). The launch runs over attack ticks 16..18 and the ram hits him on active tick 1 (attack tick 19, at 35 px), before he is within jab reach (34 px). If he stops to press attack on attack tick 18, he is hit on active tick 2. A Carl who was at 40 px when the tell began is not snapped: two or three steps in, and his jab lands around attack tick 14.

#### Matador table (hero jumps; `F` = solid face ahead of his start pivot)

| `F` | Example | Bonk on active tick (attack tick) | He stops with his pivot at |
| --- | --- | --- | --- |
| 24 | point-blank, hero flush with the wall (`d` 12) | 1 (35); the jump must start by attack tick 30 | 0 (no motion) |
| 56 | after the parapet drop, hero flush (`d` 44) | 9 (43); jump on attack ticks 0..32 | 32 |
| 84 | standoff, hero flush (`d` 72) | 16 (50); jump on attack ticks 7..39 | 60 |
| 92 | standoff, 8 px gap | 18 (52) | 68 |
| 103 | standoff, 19 px gap | 20 (54) | 79 |
| 104 or more | standoff, 20 px gap or more | no bonk: a whiff | 80 |

If the hero stands flush and does not jump, the ram hits him first: on active tick 10 at the standoff, before the pot reaches the face.

**Real positions in `cf02_arena_eng`** (`assets/levels/floor01/cf02_microarena.room.json`, `src/cf02.rs` arena module):

- **Parapet right face x 768** (the parapet is 736..768, 48 px tall). Carl flush at 780 and the Engineer at the standoff 852 give `F` 84: a bonk on active tick 16 (attack tick 50), and he stops at 792. Carl anywhere from 780 to 799 gets the bonk; from 800 on it is a whiff.
- **Hero on the parapet top** (about column 752): he is not eligible, so the Engineer waits 72 px out on his side, at 824. A hero who walks off the right edge holding +x falls for about 18 ticks and lands around 835, past an Engineer waiting at 824; landing around 780, about 44 px from him and inside the snap line, needs the stick released at the edge. From there the hero may jab him during the point, or stand flush and jump on attack ticks 0..32 for a bonk on active tick 9.
- **Left end wall face x 16:** Carl flush at 28 and the Engineer at 100 give a bonk on active tick 16, and he stops at 40. His next tell is then point-blank (`d` 12, `F` 24): a jump by attack tick 30 gives a bonk on active tick 1.
- **Right end wall face x 1136** lies beyond the room's exit trigger (goal x 1104); the live room never offers it, and fixtures use the left wall.

## 5. Brain: targeting, lane check and tokens

- **Sight:** 220 px, the goblin sight, so he wakes by position like the CF02-F waves.
- **Eligible hero:** a living hero whose feet are within **8 px** of his floor line. A hero on a perch (80 px), on the parapet top (48 px) or in a jump is not a ram target.
- **Target (ENG-G-Q06):** the existing `update_targets` rule (`EnemyTargetRule::HysteresisControlledBias`, with the solo bias toward the controlled hero) applied to **eligible heroes only**. When exactly one hero is eligible, it is the target. When the held target is no longer eligible and the other hero is, he switches at once. When neither is eligible, the existing rule over both heroes picks whom he walks toward and waits for, and he takes no token. The target locks at the tell start (the existing "a goblin in an attack keeps its target" rule).
- **What his step receives:** `combat_scene` computes an `EngineerSense` for him each tick before the goblin step. It holds `target: HeroId`, `target_x`, `eligible: bool`, `lane_clear: bool` and `in_trigger: bool` (eligible and within 72 px). The Bevy-free `engineer.rs` decides only from this, so it never reads the scene.
- **Lane check (ENG-G-Q07):** `lane_clear` is true when no intact solid and no common goblin for which `can_be_contacted()` is true has its body inside the open interval between his front edge (pivot + 12) and the target's column, at heights 0..38. It is evaluated when he would be approved for a token and again on the tick his tell would start. If it fails at the tell start he returns the token without telling. During the point the lane is not re-checked: a goblin or hero that moves into it then is the bait.
- **Approach and wait:** he walks toward the point 72 px from the target's column on his **current side** (the side of the target he is on, or his facing when level). He stops there, and he never walks backward and never crosses under a hero. If he is already closer than 72 px he holds where he is. With no token he holds facing the target. There is no minimum range: point-blank he still tells, and a hero already inside the snap line can jab him during the point.
- **Perch and parapet:** the same rule, with the target being whoever the existing rule picks. A hero who walks off the perch or the parapet lands in his range; a hero who dives on him from height (Q52) knocks him down when he waits outside the slab's footprint. He never climbs.
- **Tokens (ENG-G-Q16, fixes the starvation note at `combat_scene.rs` `step_goblins`):** he shares the goblins' two attack tokens (`MAX_ATTACKERS` 2) and the 12-tick tell spacing. He is a token candidate **only** when `wants_attack()` holds, `in_trigger` is true and `lane_clear` is true, so he never holds a slot he cannot use. He holds a token from the tell start until the ram ends (Rear, Point, Launch, Ram). A fight never has more than two attacks in flight, Engineer included. The CF02-Q66 pincer preference applies only to goblins, but counts his side as a holder side.

## 6. Size, collider and hit reactions

- **Body and hurtbox:** the goblin's 24 x 38 body (`body_rect`). ENG-01's measured idle hurtbox (-9..14 x 0..37) is within 1 px of it, so v1 keeps one rect for physics and hurt, which keeps every existing contact, wall, shore and Donut rule valid. Per-frame hurtboxes from ENG-01 section 3 are deferred: during the ram the sprite reaches x +27 while the hurtbox ends at +12, so a hit aimed at the pot may visibly whiff in v1.
- **Ram box:** `[14, 24) x [14, 33)` from ENG-01 frame 12 (pot front and snout). Its front (x 24) is what meets a solid.
- **Health 3**, one more than a goblin (level 3), never a sponge: a jab-haymaker string plus one hit, one kick-wall-slam plus one hit, or two baited bonks plus one hit kills him.
- **Hit reactions:** exactly a goblin's under `cf02_rules`: jab and haymaker stagger, the push kick launches him (contacts and wall slams apply), CF02 lying, the Q34 keep-down rule, stomp, dive, air kick, the Q38 pounce hold, Donut's swat light guard. His own differences are the committed launch and ram ([section 4](#4-moves-and-frame-data-60-hz-all-data-knobs)) and the bonk rule.

## 7. Counterplay

Numbers from the [reach table](#reach-table-carl-standing-or-jumping-in-place).

| Who | Answer | Cost or risk |
| --- | --- | --- |
| Carl | **Jump** (apex 88 px; feet 33 px up on jump ticks 6..44). At the standoff, any jump started from the point frame (attack tick 8) to ram active tick 5 (attack tick 39) clears. He passes 8 px beyond you, so you land behind him with the 40-tick recover to turn and jab, or dive on him from the jump | Jumping during the rear lands in the ram; the llama's spit line is 42..48 px |
| Carl | **Get out of range**: from the standoff, running away clears if started by attack tick 40 | Gives up ground under llama fire; no punish |
| Carl | **Hit him during the point** (jab, startup 3) when you were already inside 48 px as it began: stagger, token back | Must already be close, for example after his cooldown or a parapet drop; rushing in from the standoff triggers the snap and you are rammed |
| Carl | **Hug him during his cooldown** (60 ticks after a recover or a bonk): he never backs off, and a tell started inside 48 px is a free jab | A goblin may be swinging at you |
| Carl | **Matador:** stand with a wall or the parapet behind you, at most 19 px from it at the standoff, and jump on the point → the pot hits the face, a bonk (1 self-damage, 60-tick daze whose timer jabs do not shorten) | Mistimed, you are hit on active tick 10 with your back to the wall |
| Carl | **Bowl the crew:** jump the ram so a goblin standing beyond you is in its path, or let a goblin walk into the lane during the point → the ram knocks his own goblin down (and a lying goblin is a stomp target) | Needs reading two enemies at once |
| Carl | From a perch or the parapet: **dive** onto him while he waits outside the slab's footprint | Leaving the perch exposes you to spit |
| Donut | **Her plain jump** (the same 440 px/s jump) is her main answer; windows within one tick of Carl's | as Carl |
| Donut | **Cat hop** (CF02-Q55, only from swat or pounce recovery): apex 40 px, feet 33 px up on hop ticks 10..23, 170 px/s away, so it **clears and escapes**. This asymmetry is intended: her hop answers him, Carl's bail-out does not | Only after her own move |
| Donut | **Swat the point** (startup 4) when she is the partner, or when she was inside 48 px as it began; a swat sets the Q10-06 light guard, so the next point cannot be swat-cancelled until his next ram reaches its impact phase. **Magic Missile** (post-biscuit) during the point from range; pounce or Magic Missile in the recover | If she is the target and lunges in from outside 48 px (walk or pounce), the snap fires and she is rammed; a pounce whose box lands during the committed launch trades (1 damage each) instead of losing cleanly. Her 20 x 24 hurtbox overlaps the box at heights 14..24, so she cannot duck under it |
| Pair | One hero is pointed at and baits; the other interrupts the point (the snap watches only the target) or punishes the recover or bonk from the side | The partner who stands in the lane is the one hit |

The bail-out hop (apex 30 px) never clears the ram by height: within 40 px or toward him it is hit, and it only escapes by distance (bail away by attack tick 37 at the standoff). That is an intentional difference from the goblins.

## 8. 1P, 2P and the AI companion

- **Solo, Carl controlled, CF02 assistant on:** the Engineer is a goblin target in the `CompanionView`. Its `ThreatView` carries `tell_tick` = his attack tick and `tell_len` = the attack tick on which his launch begins: 32, or the snap tick + 1 once the snap fired. The phase mapping is the kind-aware one of [section 14](#rules-the-executor-must-not-re-decide). The `cf02_assist` profile is unchanged and never kills (`ca19_kill_policy` never). v1 adds **no** ram-lane avoidance and no Engineer-specific AI.
- **Expected AI Donut interaction (review F12).** `cf02_assist` treats his point aimed at Carl as a swat candidate when she is within `cf02_q14_swat_range_px` 96 of him, from tell tick 4 while `tell_tick + 4 < tell_len`. She then runs in (185 px/s, `RUN_SPEED`; 205 is the camera-lab `DONUT_RUN`) to her 6..30 px swat reach, limited by the Q16 action cadence (60 ticks). Because the snap watches only his target, her approach does not trigger it. Three outcomes follow:
  1. **She arrives in time:** the point is cancelled. This is the co-op answer, but the swat sets the Q10-06 light guard, and the director skips guarded threats. She therefore cancels at most every second point of one Engineer until a ram reaches its impact phase (or a launch or knockdown clears the guard).
  2. **She arrives late:** her swat meets the committed launch (no interrupt) and she stands in the lane. The ram hits the first hero it overlaps, so she is rammed. The solo last-HP guard keeps her at 1 HP or more.
  3. **She is out of range** (she usually trails Carl about 32 px behind, which puts her beyond 96 px of an Engineer at Carl's standoff): nothing happens.
  This is accepted for v1. Both counts are reported ([section 11](#11-acceptance-criteria), item 7). If the solo routes show **more than 1 AI swat cancel or more than 1 AI Donut ram hit per clear**, a COMPANION-AI-01 follow-up item is opened (AI behaviour belongs there, LAB-Q12).
- **Solo, Donut controlled:** the same rules; AI Carl (CA-37 limits) is measured, not required to match.
- **2P:** he targets by the rule of [section 5](#5-brain-targeting-lane-check-and-tokens) and shows it by pointing. The ram hits **the first hero its box overlaps, whoever was pointed at**: one hero per ram, so a partner who steps into the lane takes the ram and the pointed-at hero is spared. No friendly fire. Both heroes can use every answer in [section 7](#7-counterplay); the snap watches only the hero he points at.
- **Swap:** the facing and target lock at the tell start; swapping heroes during the point does not retarget.

## 9. Death and impact type

- **Authoritative:** unchanged. His defeats use the existing causes and force tiers (melee, wall slam, shore block, blast), with no new death physics.
- **Presentation v1 (no new art):** a new species entry `goblin_engineer` whose recipes all map to `engineer_down` = ENG-01 `down` frames 15 to 17 (the fall beat, the floor impact, the pot landing rim-down over his face). The lethal knock arc is the goblin's. No goblin A finish is borrowed in v1, because GOB-02 and the slammed cels have no pot and the pot would vanish.
- **Presentation after ENG-02 (art order, [section 10](#10-frames-per-state-and-the-missing-art-order-eng-02)):** the same recipes as the goblin plus the CF02 `slammed` overlay (stomped, wall crumple, flung), crushed and decapitated, each a goblin A cel plus a `pot` accessory layer; the pot becomes a tumbling death part in the wall, flung and exploded finishes.
- **ENG-02 is executed (PR #300, owner art review and game-feel review pending):** the five recipes map to `engineer_slammed`, `engineer_exploded`, `engineer_decapitated`, `engineer_crushed` and `engineer_knockback_down` (the last is also the fallback and keeps ENG-01 `down` 15 to 17); `engineer_down` no longer exists. The set, the rules and the deviations are in [the ENG-02 spec](../02-art/eng-02-spec.md); the get-up (ENG-01 frames 18 and 19) and the ram run (20 and 21, cycled `[12, 20, 21, 20]` at 4 ticks a frame) replace the `attack_recover` stand-in and the held frame 12.
- **Every `BodyContact` consumer applies to the ram bowl:** the audio `EnemyBodyCollision` cue and the morale `Spectacle` (-15) for witnesses. That is intended ("impacts matter").
- **Morale and CINE:** his defeat counts as **spectacular** (-35) for every goblin witness, whatever the cause: the crew sees the foreman fall (the target-priority decision). A bonk emits the appended `RamBonk` event; morale treats it as a `WallSlam` (-15); it never makes a defeat CINE-01 eligible (CF02-Q25 unchanged).

## 10. Frames per state and the missing art order ENG-02

| Gameplay state | ENG-01 frames (tag) | Playback |
| --- | --- | --- |
| Idle, holding at the standoff | 0 to 2 (`idle`) | loop 38 |
| Walk | 3 to 8 (`walk`) | loop 36 (its contact plan is exactly 40 px/s) |
| Tell | 9 (8), 10 (24, or until the snap) (`attack_tell`) | as authored |
| Launch | 11 (`attack_ram`) | 3, as authored |
| Ram active | 12 (`attack_ram`) | **held** for the active ticks, up to 20 (authored 6) |
| Recover | 13 (`attack_recover`) | **held** 40 (authored 18) |
| Bonk | 14 (12 ticks), then 13 (48) | state driven |
| Hurt, stagger, contact pose | 14 (`hurt`) | state driven |
| Launched or airborne | 14 | state driven |
| Landing and lying alive (`Downed`), Q03 | 15, then **16 held** | state driven; frame 16 (pot still on his head) keeps the living body distinct from the defeated hold |
| Get-up | **13** (he stands up with the pot over his eyes) | for `q04_getup_ticks` |
| Defeat (all finishes, v1) | 15 to 17 (`down`) | 5 + 10, then hold 17 (pot rim-down over his face) |

The held frames are playback only; the ENG-01 sources, manifest and validator do not change.

**ENG-02 (separate art order, not blocking this item, recommended after the owner's game-feel review, ENG-G-Q24).** Route: Easy, variant of the GOB-01/GOB-02 cels plus the DOZ-01 pot, the same way GOB-02B derives B from A ([GOB-02B spec](../02-art/gob-02b-death-spec.md)). Top tier writes the spec and the pot placement per frame, medium tier executes, separate top-tier review.

| Set | Frames | Source |
| --- | --- | --- |
| Get-up | 2 (sit up with the pot over the face, shove the pot back up) | GOB-01 get-up cels plus the pot |
| Slammed | 6 (stomped, wall, flung: impact and rest) | [CF02 acting](../02-art/cf02-acting-spec.md) goblin A slammed cels plus a `pot` accessory layer, the pot knocked loose in wall and flung |
| Crush and decap | 4 (`crush_contact`, `crushed_remains`, `decap_contact`, `headless_collapse`) | [GOB-02 A](../02-art/gob-02-death-spec.md) cels plus the pot (flattened under the block; the head part keeps the pot) |
| Death part | 1 part (3 rotation cels): the saucepan | the DOZ-01 pot pixels |
| Optional | 2: a ram run cycle (legs alternate under frame 12's torso) | only if the held frame 12 reads as a slide at 1x |

No hesitate, panic or taunt frames are needed: he has no morale reactions (ENG-G-Q18).

## 11. Acceptance criteria

Machine (CI):

1. Every fixture in [section 14](#fixtures-to-add) passes and replays identically twice; its golden is pinned.
2. Every golden listed under [goldens that must stay unchanged](#goldens-that-must-stay-unchanged) is byte-identical.
3. `enemy_engineer_v0.json` loads, validates (a load error names the field) and matches its pinned FNV-1a hash; a completeness test proves every ENG-G-Q has a knob or a recorded no-knob reason.
4. The gate tests in [section 14](#flag-gating) pass: the Engineer can only appear under `cf02_rules` with the CLI flag and the data switch.
5. The reach and matador tables of [section 4](#reach-maths) are reproduced by a domain unit test in `engineer.rs` (contact tick per start distance, jump-window edges at 72 px, bonk tick per face distance).
6. Performance: frame time of `cf02_arena_eng` versus `cf02_arena` reported (before and after, same machine); no regression claimed without numbers.

Evidence (implementation PR):

7. Clips at 1x speed, 640 x 360: point → ram → whiff → punish; jump-over; a snap on a rushing hero; matador bonk against the parapet; bowling a goblin; a perch dive on a waiting Engineer; a 2P point that shows who is targeted. A contact sheet of the tell next to goblin A's tell at 1x.
8. A/B report (LAB-04 one-at-a-time proof): the same scripted routes and seeds on `cf02_arena` and `cf02_arena_eng`, reporting ticks in the yard, damage taken, jumps, bail-outs, perch time, `RamBonk` and bowl counts, snaps fired, point interrupts by hero (jab, swat, missile), the share of his defeats that came from a hit during the point, **AI Donut swat cancels on Engineer points** and **AI Donut ram hits**. The follow-up threshold for both AI counts is the one in [section 8](#8-1p-2p-and-the-ai-companion).

Human (owner, game feel; the item stays "merged, game-feel review pending" until then):

9. The owner can name, without being told, who the Engineer is about to ram and when to jump.
10. In at least one of three attempts the owner changes plan because of him (jumped instead of bailing out, baited a bonk or a bowl, dived from the perch, or killed him first to break the crew).
11. He is not annoying: no unreadable hits, no pinning loop, no stalling turret problem at the perch, and the snap reads as cause and effect.

## 12. Adopted defaults (ENG-G-Q01..ENG-G-Q27)

Knob names follow the CF02 convention: the knob name starts with the question ID (`engq02_health`). The file is `assets/data/gameplay/enemy_engineer_v0.json`.

| ID | Question | Adopted value | Rationale | Knob |
| --- | --- | --- | --- | --- |
| ENG-G-Q01 | Role | Middle-range floor-lane charger with a punish window, [section 2](#2-tactical-role) | Distinct from goblin (close) and llama (height); uses only the ENG-01 ram | none (design) |
| ENG-G-Q02 | Health | **3** | Level 3, one more than a goblin, not a sponge | `engq02_health` |
| ENG-G-Q03 | Body and boxes | Body 24 x 38 (the goblin's); ram box `[14, 24) x [14, 33)`; the box front meets solids | Within 1 px of ENG-01; reuses every goblin contact rule; the pot, not the body, hits the wall | `engq03_ram_box` |
| ENG-G-Q04 | Walk speed and sight | **40** px/s; sight **220** | ENG-01 contact plan; goblin sight wakes the wave by position | `engq04_walk_speed`, `engq04_sight` |
| ENG-G-Q05 | Trigger and standoff | Tell at **72** px or less (pivot to pivot); standoff **72**; no minimum; never walks backward | With the hero's half-width (12) included, a standing hero at 72 px is hit on active tick 10 of 20 (reach out to 115 px), so the ram from the standoff is a real threat and not decided by step order, and a hero up to 19 px in front of a wall gets the matador bonk ([section 4](#reach-maths)) | `engq05_trigger_px`, `engq05_standoff_px` |
| ENG-G-Q06 | Targeting | The existing hysteresis rule over **eligible** heroes (feet within **8** px of his floor line); locked at the tell start; no token while none is eligible | No ram on perched or jumping heroes; a readable, unchanging point; reuses `update_targets` | `engq06_floor_tolerance_px` |
| ENG-G-Q07 | Lane check | At approval and at the tell start: no solid and no contactable common goblin between his front and the target's column, else no token and no tell | Stops trivial wall-cheese and accidental crew bowling; bait needs player timing | `engq07_lane_check` (switch) |
| ENG-G-Q08 | Tell | **32** ticks: rear 8 + point 24; interruptible by any hero hit, swat included | ENG-01 ticks; the longest enemy tell matches the longest reach | `engq08_rear_ticks`, `engq08_point_ticks` |
| ENG-G-Q09 | Launch | **3** ticks, no motion, no box; **committed** like the goblin smear | ENG-01 frame 11; matches `take_hit_split` and the companion's committed-from-`tell_len` assumption | `engq09_launch_ticks` |
| ENG-G-Q10 | Ram | Active at most **20** ticks at **240** px/s (**80** px); committed except for a lethal hit or a contact knockdown | Faster than Carl's run (185) but short; 80 px lets the matador work from the standoff with a 19 px margin; armour matches the goblin swing rule | `engq10_active_ticks`, `engq10_speed_px_s` (a multiple of 60) |
| ENG-G-Q11 | Ram hit on a hero | **1** damage through the ordinary `hurt_hero` path (that path's invulnerability, hurt ticks, hit-stop 4 and solo guards), knock **120 / 160** px/s in the ram direction; the first hero the box overlaps, one per ram; the ram stops | Existing hurt path and the Q42 impulse; a heavier hit than a swing, no new frames | `engq11_damage`, `engq11_knock_x`, `engq11_knock_y` |
| ENG-G-Q12 | Recover | **40** ticks (frame 13 held); hits stagger him | Every clearing jump at the standoff lands with time to turn and jab inside it (the latest lands on attack tick 88, recover ends 94) | `engq12_recover_ticks` |
| ENG-G-Q13 | Bonk | When the box front meets a solid face: hurt **12** + daze **48**; self-damage **1** (Q11 value); non-launch, non-knockdown, non-lethal hits leave the timer unchanged; credit to the pointed-at hero; appended `RamBonk` event; existing wall-slam defeat path; not CINE eligible | The matador answer is a real kill path but needs well-timed dodges plus a hit; jabs cannot shorten the punish window | `engq13_hurt_ticks`, `engq13_daze_ticks`, `engq13_self_damage` (0 = off) |
| ENG-G-Q14 | Crew bowl | A contactable common goblin in the ram box gets the CF02-B contact knockdown in the ram direction (Q12 damage 0); `BodyContact` with a ram stamp credits the pointed-at hero; the Engineer recovers 40 | Player-authored causality with the CF02-B system, no new physics | `engq14_bowl` (switch) |
| ENG-G-Q15 | Cooldown | **60** ticks after each recover or bonk (goblin 40); an interrupted tell uses the goblin `HURT_COOLDOWN` path instead | One ram about every 2.6 s at most; he is a punisher, not a spammer; a jab interrupt costs him HP, so a point-blank jab sequence ends in his defeat, not a lock | `engq15_cooldown_ticks` |
| ENG-G-Q16 | Tokens | Shares `MAX_ATTACKERS` 2 and the 12-tick tell spacing; a candidate only when in trigger with a clear lane; holds a token from the tell start to the ram end | At most two attacks in flight, Engineer included; no slot starvation | none (code) |
| ENG-G-Q17 | Shore | CF02-Q31 applies to his ram box: one shore hit per ram on a shore at `damaged_1` or later (`ShoreTripped`); an intact shore ignores him; the shore column is not a solid, so the ram passes | One consistent self-own rule for the goblin family | none (reuses `q31_*`) |
| ENG-G-Q18 | Morale | No nerve, no reactions (never hesitates, panics, taunts or enrages) and never witnesses; he counts as a living goblin for the CF02-Q62 "last goblin" rule | The stubborn foreman contrasts with the cowardly crew; no acting frames needed | `engq18_morale` (switch, `false`) |
| ENG-G-Q19 | Crew reaction to him | His defeat is a spectacular witness event (**-35**) for goblins in range; `RamBonk` counts as `WallSlam` (**-15**); his hit on a hero is a hero-hurt event (+15) | Killing him first breaks the crew's nerve: a target-priority choice using only existing deltas | `engq19_defeat_spectacular` (switch) |
| ENG-G-Q20 | Hit reactions | The goblin's under `cf02_rules`, plus the committed launch and ram and the bonk rule | One rig, one reaction language | none |
| ENG-G-Q21 | Death presentation | Species `goblin_engineer`; v1 every recipe → `engineer_down` (ENG-01 15 to 17); **ENG-02 executed (PR #300): five own finishes, `engineer_down` removed** (the first column is history) | No pot vanishing; authoritative death unchanged | data (death profile) |
| ENG-G-Q22 | Companion | Seen as a goblin target with the kind-aware phase mapping and `tell_len` = launch start; no new AI behaviour; the expected AI Donut interaction of [section 8](#8-1p-2p-and-the-ai-companion) is measured, follow-up if more than 1 AI swat cancel or more than 1 AI Donut ram hit per clear | AI changes belong to COMPANION-AI-01; the Q10-06 light guard already caps her cancels at every second point | none |
| ENG-G-Q23 | Cue | The existing goblin tell cue at the point start; no new audio asset | No invented sound; the owner judges audio later | none |
| ENG-G-Q24 | Art | v1 uses ENG-01 only ([section 10](#10-frames-per-state-and-the-missing-art-order-eng-02)); ENG-02 is ordered after the owner's game-feel review | Do not spend art on a role the owner may reject | none (process) |
| ENG-G-Q25 | Gate and placement | Active iff `cf02_rules` **and** the CLI flag `--cf02-engineer` **and** `engq25_enabled` (a data kill switch, default `true`). Reached only by `floor01_scene --cf02 --cf02-engineer`; fixture `cf02_arena_eng`: the CF02-F room with the yard goblin at x 940 (index 4) replaced by the Engineer at x 940. With the switch off, the flag runs plain `cf02_arena` | Plain `--cf02` stays the regression room; one-for-one swap keeps six enemies and the LAB-Q04 cap; the kill switch needs no rebuild | `engq25_enabled` (kill switch) |
| ENG-G-Q26 | Temperaments in the variant | The four remaining goblins take slots 0 to 3 of the same CF02-Q58 shuffled bag; the fifth slot is unused, so the missing temperament varies per attempt | No change to Q58; still different on every restart | none |
| ENG-G-Q27 | Snap (rush guard) | If the target comes within **48** px during the point, having been farther than 48 px at the tell start, the point ends and the launch starts on the next tick. Only the target is watched | Without it a rush-in jab from the standoff (about 13 ticks of running plus jab startup 3) always beats the 32-tick point, and the jump would never be needed. 48 px is jab reach (34) plus the 4 ticks from snap to ram contact at Carl's run speed, so a charging hero is rammed before his jab can reach. Watching only the target keeps the partner's interrupt as the co-op answer. 0 turns it off | `engq27_snap_px` (0 = off) |

## 13. Genuinely open questions

1. **Feel of the ram on a real pad:** 240 px/s over 80 px, the 40-tick recover, whether the snap reads as "he saw me coming" rather than as an unfair early launch, and whether the held frame 12 reads as a charge or a slide at 1x (the trigger for the optional ENG-02 run cycle).
2. **Is the matador bonk fun or a cheese?** The review judged it not a cheese as designed: each bonk costs a jump timed at the right distance, a mistake costs 1 damage, and after a bonk he stands near the wall, so the next ram usually points away from it. One repeatable case remains. With the hero flush against a wall and the Engineer point-blank, every point gives a bonk on active tick 1 if the hero jumps by attack tick 30, which costs about 155 ticks per cycle under room pressure. A jab is faster there anyway. `engq13_self_damage` 0 turns the bonk into a daze-only punish.
3. **Story placement:** whether the Engineer fights on foot in the Floor 01 opening after the Dozer is wrecked (ENG-01 section 13), and whether he joins plain `--cf02` or a LAB room (R04 Pressure Chamber is the natural candidate after LAB-03). Both wait for the owner's feel review.
4. **ENG-02 timing:** commission the death and get-up frames after the feel review (adopted) or before.
5. **The wrench** stays a stowed prop; any tool use needs a new design decision.

## 14. Implementation brief (medium-tier executor)

One PR (speed mode allows grouping), or ENG-I1 (Bevy-free domain, data, fixtures) then ENG-I2 (Bevy wiring, flag, evidence). Do not edit `docs/04-production/backlog.md`; put the status in the PR body. No design decision is left to the executor; if a fixture's outcome contradicts this sheet, stop and report it as a design defect.

### Files to touch

| File | Change |
| --- | --- |
| `assets/data/gameplay/enemy_engineer_v0.json` (new) | The `engq*` knobs of section 12, embedded with `include_str!`, FNV-1a pinned like `cf02_combat_v0.json`. Validation: standoff ≤ trigger; snap < trigger (0 allowed); `engq10_speed_px_s` a positive multiple of 60; active, recover, rear, point and launch ticks > 0; ram box non-empty |
| `src/engineer.rs` (new, Bevy-free) | `EngineerTuning` (load, validate, pin, completeness list), `EngineerSense`, the attack timeline (`EngineerPhase`: Rear, Point, Launch, Ram, Recover, Bonk), the snap, the ram step and its ordered tests (section 4), the reach and matador unit tests (section 11 item 5) |
| `src/goblin.rs` | `kind: GoblinKind { Common, Engineer }` (default `Common`, mixed into the state hash **only** when `Engineer`); for the Engineer, `step_with` delegates the brain and attack to `engineer.rs` with the `EngineerSense`; `attack_phase()`, `target_state()`, `token_held()`, `take_hit_split` (committed phases, bonk rule) and `swing_box()` (returns the ram box) are kind-aware at the source; the legacy branch is untouched |
| `src/combat_scene.rs` | `Fixture::engineers: &'static [EnemySpawn]` (empty everywhere existing; the `llamas` field is the precedent), built as Engineer goblins after the fixture's goblins; `EngineerSense` per tick; targeting over eligible heroes in `update_targets`; token candidacy (section 5); the ram hit through `hurt_hero`; the bonk and appended `CombatEvent::RamBonk`; the ram counter; the Q31 shore check on the ram box |
| `src/combat_scene/contacts_cf02.rs` | A `ram_bowl` helper called from the ram step (not from the flier loop of `resolve_contacts_cf02`, because a ramming Engineer is not a `Launched` flier) |
| `src/morale.rs`, `src/combat_scene/morale_cf02.rs` | Skip Engineer nerve; his defeat → spectacular delta; `RamBonk` → the `WallSlam` delta; the Q58 draw index counts common goblins only; Q62 counts him as living |
| `src/cf02.rs` (`arena` module) | `GOBLINS_ENG` (520, 600, 680, 880), `ENGINEERS` (940), fixtures `cf02_arena_eng` and `cf02_arena_eng_cp2`, `fixture_uses_arena` entries |
| `src/cf02_fixtures.rs` | The new fixtures and their goldens |
| `src/bin/floor01_scene.rs` | The `--cf02-engineer` token, rejected without `--cf02` and by `--demo`, the story route and `--scene combat_lab`; the kill-switch fallback to `cf02_arena` |
| `assets/data/death/species_death_profiles_cf02.json` and the death presenter | Species `goblin_engineer` → `engineer_down`. The overlay schema only prepends `recipes_first` to an existing species and `goblin_engineer` is not in the base file, so extend the overlay schema explicitly to add a species and record the migration; the presenter gets an enemy-kind → species mapping. **`species_death_profiles.json` stays byte-identical** |
| Scene presentation (the goblin sprite adapter) | Engineer kind → the `eng_01` atlas (`assets/manifests/eng_01.json`) with the frame mapping of section 10 |
| `src/companion/` | The Engineer in `CompanionView` with the kind-aware phase and `tell_len` = launch start; no behaviour change |
| PLAYTRACE observation | If it enumerates goblins, add the kind only for non-common goblins, so existing traces stay byte-identical; otherwise record a follow-up |

Do **not** touch: `src/camera_lab/` (its `ENGINEER` FoeSpec is a lab hypothesis, not this design), `src/combat_lab/descriptor.rs` (`engineer` is not a lab roster entry yet), the ENG-01 art sources and manifest, `cf02_combat_v0.json`.

### Rules the executor must not re-decide

- **Order within a tick** (unchanged scene order): Carl's strike, Donut's moves, the scene step (heroes move), then every goblin in index order. The Engineer's ram, on each active tick, **steps first, then tests** hero, solid, crew, distance (section 4). A strike that lands on him this tick resolves before his step (Carl wins a same-tick trade).
- **Phase mapping (one function, used by every consumer):** Rear and Point → `AttackPhase::Tell`; Launch → `Smear`; Ram → `Impact`; Recover and Bonk → `Recover`. `target_state()` follows it (so the Q10-06 light guard sees his ram as `Impact`), `token_held()` is true for Tell, Smear and Impact, and `take_hit_split` treats Smear and Impact as committed. Audit every consumer with a search for `attack_phase(`, `token_held(`, `target_state(`, `swing_box(` and `AttackPhase::`; at the time of writing these are in `goblin.rs`, `combat_scene.rs` (Q31 shore Impact check in `step_goblins`, the debug line, the screen-bounds test), `combat_scene/contacts_cf02.rs` (`was_attacking`), `combat_scene/morale_cf02.rs` (two token readers), `companion/view.rs` and `director.rs` (`ThreatView`), `autoplay/observation.rs`, `combat_anim.rs`, `combat_anim/cf02.rs` and `bin/floor01_scene.rs`. Presentation consumers use the section 10 frame mapping for the Engineer instead of the goblin swing frames. The goblin `swing_on_hero` path is never called for him; his ram has its own path.
- **Ram stamp for `BodyContact`:** `launch = 0x8000_0000 | ram_seq`, where `ram_seq` is a per-`CombatState` counter that starts at 1 and increments at each launch start, and is mixed into the hash only when non-zero. Carl's move instances are the existing launch ids; a disjoint space avoids collisions without renumbering them, which would change goldens. `source` = his enemy index, `target` = the goblin, `instigator` = `Some(pointed-at hero index)`. The pair is not pushed into `contacts`: a ram ends on its first crew contact, so it bowls at most once.
- **The bonk clamp** uses the ram box front (`engq03_ram_box` x1 = 24): pivot = face ∓ 24.
- **Hits during Bonk:** damage and hit-stop only, the timer is unchanged. A launch, a knockdown or a lethal hit overrides it with that reaction.
- **Cooldown:** `engq15` starts when the Recover or the Bonk ends; an interrupted tell goes through `Hurt` and `HURT_COOLDOWN`.
- **Snap:** armed at the tell start if the target's pivot distance is above `engq27_snap_px`; checked after the heroes' step on every Point tick, at any height (a jump-in also snaps). It shortens only the point, and the 3-tick launch follows.

### Fixtures to add

All headless on the arena kit, under `cf02_rules` with the engineer flag and switch on, golden-pinned. Unless stated: Carl only (`DonutSetup::NONE`, party not live), no other enemies, the Engineer at **E = 200** (clean brawl-lane floor, out of reach of the shore and the perches), awake, facing his target. **Attack tick 0** = the Engineer's first logged `GoblinTell`; scripted input ticks below are attack ticks, converted to absolute fixture ticks from that logged tick and pinned. Expected ticks come from the [reach tables](#reach-maths). Where a row says "outcome", the outcome is the assertion and the tick is pinned by the golden.

| Fixture | Setup | Input | Expected |
| --- | --- | --- | --- |
| `eng_ram_hits_idle_carl` | Carl 272 (`d` 72) | none | Tell 0, launch 32..34, HeroHurt on active tick 10 (attack tick 44), Engineer pivot 240, 1 damage, Carl knocked +x at 120 / 160, Recover 40 then cooldown 60 |
| `eng_jump_over_ram` | Carl 272 | jump on attack tick 20; right + attack on attack tick 70 | No HeroHurt; whiff ending on active tick 20 (attack tick 54) at pivot 280, 8 px past Carl; Carl lands on attack tick 69; the jab hits in the recover (55..94) and staggers him |
| `eng_jump_window_edges` | Carl 272, four runs | jump on attack tick 7 / 8 / 39 / 40 | Hit on active tick 17 / clear / clear / hit on active tick 10 |
| `eng_bailout_does_not_clear` | Carl 240 (`d` 40), facing him (spawn facing, or one tick of -x before the press; `d` stays at 36 or more) | attack on attack tick 20 (the jab whiffs at 40 px); jump on the first following jab tick on which `cancel_for` returns C3, no stick (bail-out away) | Outcome: the bail-out hop starts (apex 30 px) and Carl takes the ram (HeroHurt) |
| `eng_bailout_escapes_at_range` | Carl 272 (`d` 72), facing him | as above, the attack on attack tick 20 | Outcome: bail-out away, whiff, no HeroHurt (escape by distance only) |
| `eng_run_out_of_range` | Carl 272 | hold +x from attack tick 8 | Whiff, no HeroHurt |
| `eng_snap_rush` | Carl 272 | hold -x from attack tick 8 | Snap on attack tick 15, launch 16..18, HeroHurt on active tick 1 (attack tick 19) |
| `eng_point_interrupt_jab` | Carl 240 (`d` 40, inside the snap line) | hold -x attack ticks 8..10; attack on attack tick 11 | Jab Hit during the point, stagger, no launch, token returned, `HURT_COOLDOWN` path |
| `eng_point_interrupt_swat` | 2P: Carl 272 (target, nearer), Donut 280 | Donut holds -x attack ticks 4..19, swat on attack tick 20 | Swat interrupts the point (no snap: Donut is not the target), light guard set on him |
| `eng_parapet_bonk` | Carl 780 (flush with the parapet's right face 768), Engineer 852 | jump on attack tick 20 | `RamBonk` on active tick 16 (attack tick 50), Engineer pivot 792, health 3 → 2, 60-tick bonk, then cooldown 60; no HeroHurt |
| `eng_parapet_gap_whiff` | Carl 804 (24 px gap), Engineer 876 | jump on attack tick 20 | No bonk, whiff at pivot 796 |
| `eng_end_wall_bonk_kill` | Carl 28 (flush with the left end wall, face 16), Engineer 100 | jump on attack tick 20 of each of his three tells | Bonk 1 on active tick 16 (stops at 40); bonks 2 and 3 point-blank (`d` 12) on active tick 1; health 3 → 2 → 1 → 0; the third bonk defeats him in place by the wall-slam defeat path, credited to Carl; three `RamBonk`; no CINE trigger; no HeroHurt |
| `eng_lane_blocked_goblin` | Carl 272, a `Passive` goblin at 236 | none | No Engineer tell while the goblin stands in the lane (240 ticks); he is not a token candidate |
| `eng_lane_blocked_parapet` | Carl 724 (flush with the parapet's left face), Engineer 796 | none | No tell (240 ticks) |
| `eng_perch_not_targeted` | Carl 820, Engineer 1100 (asleep: 280 px) | jump on fixture tick 1, hold +x fixture ticks 1..44 (Carl jumps onto the right perch, 896..992, and stops near 956) | He wakes while Carl is airborne, walks to about 1028 (72 px from Carl's column, his side, outside the slab) and holds facing Carl; no tell up to fixture tick 360 |
| `eng_perch_dive` | as `eng_perch_not_targeted` | then hold +x until Carl is at about 980 (he holds at 1028: his new standoff point would be behind him); jump + hold +x off the perch toward him; the CF02-Q52 dive input in the descent (the executor may move the dive press by up to 3 ticks so the dive box meets him) | Outcome: the dive hits and knocks him down (`Downed`) |
| `eng_bowls_goblin` | Carl 272, a `Passive` goblin at 290 (beyond Carl, outside the lane) | jump on attack tick 20 | The ram passes under Carl and bowls the goblin on active tick 14 (attack tick 48): `BodyContact` with the ram stamp, instigator Carl; goblin `Downed`, pushed +x; Engineer Recover; no HeroHurt |
| `eng_shore_trip` | Carl 402, Engineer 330 (the box path covers the strike column 384); the shore pre-damaged to `damaged_1` the way the CF02-Q31 fixtures do; a second run with the shore intact | jump on attack tick 20 | One `ShoreTripped` for the ram; none with the intact shore |
| `eng_defeat_scares_crew` | Carl 230 (`d` 30), a `Passive` goblin at 300 (within the CF02 witness range) | Carl kills him with jab, haymaker, then jab (the executor picks press ticks so each hit lands, inside the chain window for the haymaker) | His `EnemyDefeated`; the witness goblin's nerve drops by exactly 35 on that tick |
| `eng_kick_launch_contact` | Carl 228 (`d` 28), a `Passive` goblin at 160 (behind him, outside his lane) | physical (push kick) pressed together with -x on attack tick 0 (Carl turns to face him and kicks; active on attack ticks 13..16) | The kick lands during his point, launches him -x into the goblin: a CF02-B `GoblinContact`, the goblin knocked down |
| `eng_2p_targets_nearest` | 2P: Donut 260 (`d` 60), Carl 300 (`d` 100) | none | He points at Donut; HeroHurt on Donut on active tick 7 (attack tick 41); Carl untouched |
| `eng_2p_partner_in_lane` | 2P: Carl 272 (target), Donut 310 | Donut holds -x attack ticks 10..28 | Outcome: Donut (in the lane, about 51 px after 19 ticks at her `RUN_SPEED` 185) is hit on active tick 5, Carl is not; no snap |
| `eng_solo_assist` | `cf02_arena_eng` solo with the CF02 assistant, the `cf02_arena` scripted Carl route | that route | No softlock; AI swat cancels on his points and AI Donut ram hits logged |
| `cf02_arena_eng`, `cf02_arena_eng_cp2` | the live variants | the `cf02_arena` routes | Replay identical; a restart reshuffles (Q58) |

Domain unit tests in `engineer.rs` / `goblin.rs` (no scene): the reach and matador tables; a jab during the launch and during the ram deals damage and hit-stop only and the ram continues, while a lethal hit and a contact knockdown end it; a jab during the bonk leaves the bonk timer unchanged, while a push kick launches him out of it; the snap arms only from outside `engq27_snap_px` and ignores the non-target hero.

Gate tests: `--cf02-engineer` without `--cf02` errors; `--demo`, the story route and the lab entry reject it; plain `--cf02` hashes exactly as the pinned `cf02_arena`; with `engq25_enabled` false, `--cf02 --cf02-engineer` runs and hashes exactly as `cf02_arena`; the data-file pin and completeness tests.

### Goldens that must stay unchanged

Legacy DEMO-01 and cf02-off scene hashes; every entry of `GOLDENS` in `src/cf02_fixtures.rs` (including `cf02_arena` and `cf02_arena_cp2`); the companion, Donut, party, boss, CINE, death presenter, PLAYTRACE and combat-lab fixture goldens; the pinned AUTOPLAY `autoplay-episode/1` goldens; `cf02_combat_v0.json` and its `TUNING_HASH`; `species_death_profiles.json`; the ENG-01 tests (`tests/test_art_eng01.py`).

### Flag gating

The Engineer exists only when `cf02_rules` is on **and** the CLI flag `--cf02-engineer` is given **and** the data value `engq25_enabled` is true. The data value is a kill switch: with it off, the flag selects plain `cf02_arena`. Switches are read through one accessor (as `Cf02Tuning::on`), never raw. `--demo`, the story route, plain `--cf02` and the lab profile never load him.

## 15. Placement plan

**`floor01_scene --cf02 --cf02-engineer` runs `cf02_arena_eng`: the CF02-F microarena with the second wave's goblin at x 940 replaced by the Engineer. He guards the llama yard between the parapet (a matador wall: flush at 780, he bonks from his 852 standoff), the right perch (a dive spot when he waits outside the slab) and goblin 880 (a bowling pin), under llama fire.**

## 16. Horizon fit

Consulted the [character progression horizon](character-progression-horizon.md). Still consistent, no guide change: the Engineer adds no player verb, unlock or button. He makes existing Floor 01 verbs (the jump, the dive, the push kick, wall and body impacts, Donut's swat and hop) the right answer at the right time, and rewards positioning against terrain and enemies. The snap is enemy behaviour, not a player rule. Solo and 2P get the same answers. The engine contract stays IP-neutral (`GoblinKind::Engineer` is a generic data kind; the fan-concept layer is only the art).

## 17. Rights and provenance

No new art, audio or text from third parties. The ENG-01 sources and provenance are unchanged ([ENG-01 section 15](../02-art/eng-01-spec.md#15-rights-and-provenance)). Gameplay values are original design hypotheses. Design and the round 1 revision by Claude Opus (top tier); review round 1 by a separate Claude Opus agent; execution is not done yet.

## 18. Review round 1 resolutions

The independent top-tier review (CHANGES REQUESTED) found one blocking design defect (B1) and twelve FIX-NOW items. Every recommended value was adopted except where noted.

| Finding | Resolution |
| --- | --- |
| **B1** ram reach without the hero half-width | Redone with Carl's 12 px and Donut's 10 px half-widths ([reach maths](#reach-maths)). Trigger and standoff **96 → 72**, ram **16 → 20** active ticks (**64 → 80 px**), the bonk tested on the **pot front** (box x 24), recover **30 → 40**. A motionless Carl at the standoff is hit on active tick 10 of 20. Worked reach and matador tables, checked against the parapet face 768 and the end wall face 16. The step order is fixed (step, then hero, solid, crew, distance). Jump windows per distance, bail-out property (never clears by height; escapes only by distance, away, from 48 px or more, early enough), "land behind him" (true below 80 px), Donut's answers restated, and every fixture has explicit start positions and inputs. Not adopted as-is: the review's example "72 with the 64 px ram" leaves the flush matador at the standoff with only 3 px of margin when the pot front meets the wall, so the ram is 80 px (19 px margin). |
| F1 launch phase | Launch committed (smear); companion `tell_len` = launch start (32, or snap tick + 1) |
| F2 token starvation | Token candidate only when in trigger with a clear lane; lane checked at approval and at tell start |
| F3 targeting | Existing hysteresis rule over eligible heroes, locked at tell start; `EngineerSense` input; waits on his current side, never backward, never under the hero |
| F4 non-targeted hero in the lane | The first hero overlapped is hit, one per ram; section 8 reworded |
| F5 hits during a bonk | Damage and hit-stop only, timer unchanged; launch, knockdown or lethal hit overrides |
| F6 cooldown paths | Interrupt → `Hurt` + `HURT_COOLDOWN`; recover and bonk → full 60 from their end |
| F7 hero-hit path | `hurt_hero` with a `HeroBlow`; that path's invulnerability (Carl 55, Donut her tuning), hurt ticks and hit-stop 4; the Engineer's hit-stop as `swing_on` |
| F8 bowl bookkeeping | Ram stamp in a disjoint id space (`0x8000_0000 \| ram_seq`) instead of Carl's move-instance counter, so Carl's ids and existing goldens are untouched; `ram_bowl` outside the flier loop; `can_be_contacted()` already excludes lying goblins, plus common goblins only |
| F9 every `attack_phase` consumer | One kind-aware mapping at the source; consumer list and search terms in the brief |
| F10 gate semantics | Active iff `cf02_rules` && flag && `engq25_enabled` (kill switch); gate tests restated ("plain `--cf02` hashes as `cf02_arena`"; switch off → the flag runs `cf02_arena`) |
| F11 Donut wording | Hurtbox overlap corrected (heights 14..24); cat hop only from swat or pounce recovery, clears and escapes (intended asymmetry); her plain jump is her main answer |
| F12 reversed AI Donut risk | Expected interaction described (in time: cancel, capped at every second point by the existing Q10-06 light guard; late: rammed in the lane; out of range: nothing); both counts in the A/B report with the "more than 1 per clear" follow-up threshold |
| U3 rush-in jab always wins | **Addressed in design:** the snap (ENG-G-Q27, 48 px, target only). A target who charges from outside 48 px is rammed on active tick 1 or 2; a hero already close, or the partner, can still interrupt. Share of point-interrupt defeats added to the A/B report |
| U1 alive and dead read the same | Living `Downed` holds frame 16; the defeat holds 17 |
| U4 landing on him | Stated: landing causes nothing (stomp is Down + Physical) |
| U5 species overlay | Brief states the schema extension and the kind → species mapping |
| U2, U6, U7 | U2 is measured (perch time in the A/B report); U6 is recorded as deferred in section 6; U7 (an optional horizon change-log row) is left for the implementation PR |

## 19. Implementation status and mismatches found while implementing

Implemented by the medium tier from the section 14 brief (the PR body carries the evidence). Game feel is **not accepted**; every value is still data in `assets/data/gameplay/enemy_engineer_v0.json`.

- **Round 2 nits N1 to N6 and U7** are folded into the sections above; the horizon change-log row is in [character-progression-horizon.md](character-progression-horizon.md).
- **Registry names.** `eng_jump_window_edges` is four fixtures (`_7`, `_8`, `_39`, `_40`, one run each) and `eng_shore_trip` has a sibling `eng_shore_intact` for its second run. The `eng_*` fixtures are Carl only: Donut is not a live hero in them (`PartyFixture::party_live`), and the 2P ones place her with `donut_start_x`.
- **Mismatch (design defect, adopted workaround).** `eng_defeat_scares_crew` asks for the witness to be a `Passive` goblin whose nerve drops by exactly 35, but a `Passive` goblin is never stepped by the morale layer (its nerve cannot change). The fixture uses a chasing goblin at 300 instead; the -35 is asserted on that goblin on the tick of his defeat.
- **Adopted details the brief left open.** The bonk's self-damage path adds the CF02 flier hit-stop only when it defeats him (the wall-slam defeat path); a ram that overlaps several common goblins bowls the nearest one ahead of him; the follower dodge ignores the Engineer's attack in v1 (`Goblin::dodge_threat`); `RamBonk` is credited to the pointed-at hero through `hit_by` and plays the wall-slam cue.
- **Follow-ups recorded, not done.** The PLAYTRACE observation rows carry no Engineer column (adding one changes the `autoplay-episode/1` shape), so a recorded Engineer session replays correctly (the replay resolves `cf02_arena_eng*` by name) but a policy cannot tell him from a goblin; the A/B report of section 11 item 8 needs the owner's play routes and is not produced here.
- **Review fix round (post-#165 rebase).** (1) `cf02_arena_eng_cp2` ran the `cf02_arena` direct pilot, which jumps the parapet, lands inside goblin e3 at x 780 and then jabs and swings for 1650 ticks without a hit or a hurt (it also stalls the plain checkpoint room; the Engineer never moved, so it exercised no Engineer behaviour). The fixture now uses the environment plan and clears the room with one tell and one ram on Donut; the direct-pilot overlap is a follow-up for the arena autopilot, not an Engineer defect. (2) The no-softlock test now pins, per run, the AI swat cancels and AI Donut ram hits (`eng_solo_assist` 1 and 1, `cf02_arena_eng` 0 and 1, `cf02_arena_eng_cp2` 0 and 1 after the active-Donut director of #165) and asserts that the Engineer points and rams and that the room is cleared. (3) The Carl-only `eng_*` fixtures no longer draw Donut's trail sprite in the real render (`donut_start_x` only places the live body, so it never moved the sprite), which is what hid the parapet in the wall-bonk clip. (4) The Bevy replay viewer resolves a recorded Engineer session to `cf02_arena_eng*` through the same match as the headless replay (`cf02::arena::fixture_for_recorded`).
