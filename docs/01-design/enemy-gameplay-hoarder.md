# ENEMY-GAMEPLAY-HOARDER: the Hoarder as a boss (HRD-0 design sheet)

> **Status: design adopted under the owner's design-defaults directive (AGENTS.md, 2026-11-19); revised after independent review rounds 1 and 2 ([Revision 1](#17-revision-1-resolutions), [Revision 2](#18-revision-2-resolutions)); not implemented, not game-feel accepted.**
> Every choice below has one adopted value with an `HRD-Q##` ID ([section 12](#12-adopted-defaults-hrd-q01hrd-q41)). Each value is a tuning hypothesis stored as data. The owner reviews game feel after the implementation merges and may change any value in a follow-up PR. Only [section 13](#13-genuinely-open-questions) waits for the owner.
> Design: Claude Opus (top tier). Execution: medium tier, steps HRD-A1, HRD-A2 and HRD-B to HRD-D ([section 14](#14-implementation-brief-hrd-a1hrd-d)). Review: a separate top-tier agent.

**Why:** step HRD-0 of the Floor 01 playtest integration plan ([PR #182](https://github.com/pando85/dungeon-floorbreak/pull/182), `docs/07-agents/f01-playtest-integration-plan.md` section 4.3, conflicts K1, K6, K8, K10). It turns the owner's encounter concept [The Mountain Breathes](floor-01-hoarder-mountain-encounter.md) (#178) and the HOARD-01 readability set into exact rules a medium-tier agent can build. The precedent is the [Engineer sheet](enemy-gameplay-engineer.md) (#162); the Juicer's `src/boss.rs` is the code precedent for a boss.

## 1. Authority and locks

- **Owner intent (#178), kept:** a dread encounter about a transformed human woman, never comedy; a very small vocabulary (**close swipe, committed displacement attack, clear punish recovery**); her own attacks reshape the junk; three valid wins (direct, terrain, collapse); victory never requires a pile; no HP sponge, no endless immunity, no summons, no sludge projectiles, no surprise offscreen damage, no forced camera; rapid retry skips staging. Deviations are listed in [section 2](#2-deviations-from-178-flagged).
- **[HOARD-01 spec](../02-art/hoard-01-spec.md):** 112 px, frame 160 x 128, pivot (70, 127), 16 frames; one attack (`attack_tell` 8 + 26 = 34 ticks, `attack_swing` smear 3, impact 6, recovery 24); the impact hitbox; the hurtbox; the 80 px art estimate for the tell trigger. No shove art exists (K10).
- **Integration plan conflicts:** **K1** dynamite is granted on her defeat (`pile_boss_f01_defeated`), so she is fought with the base kit; **K6** her blows and her room's hazards are never evadable (DM-Q18); **K8** no unthreatened 81..128 px surface; **K10** greybox stand-ins are allowed, labelled.
- **[CARL-PROG-01](https://github.com/pando85/dungeon-floorbreak/blob/main/docs/03-tech/carl-prog-01-floor01-combat-progression.md)** (main): base kit and dynamite rules; its CP-Q01 text is changed by K1 in PT-DOC, not here. **[DONUT-PROG-01](https://github.com/pando85/dungeon-floorbreak/blob/main/docs/03-tech/donut-prog-01-floor01-ability-progression.md)** (main, #175): Light on Your Feet is held in this room (story pin: biscuit and Light on Your Feet); DP-Q18 gate classes; DP-Q45 lane ceiling. **[DONUT-MOBILITY-01](../03-tech/donut-mobility-01.md):** DM-Q18.
- **[Juicer boss design](../02-art/floor-01-boss-design.md):** super armour, poise, fairness floors (no tell and no punish window under 24 ticks), solo M1/M2/M3 rules, rack crush. Reused wherever it fits.
- **[Antihero tone](antihero-tone.md)** and **[Fun over canon](https://github.com/pando85/dungeon-floorbreak/blob/main/docs/01-design/vision.md#fun-over-canon-owner-decision-2026-11-21)** (#177).
- **IP-neutral engine names:** `pile_boss_f01`, `PILE_BOSS_ID`, `src/pile_boss.rs`. The fan-concept name lives only in art and display data.

## 2. Deviations from #178 (flagged)

| ID | #178 text | This sheet | Why |
| --- | --- | --- | --- |
| DV-1 | Bank A "can narrow but never seal the floor lane" | Bank A is a 64 x 48 px solid on the lane that both heroes clear with any ordinary jump (Carl apex 88.5, Donut 114.9 or 137.95) | In a 2D side view any floor obstacle blocks walking. "Never seal" is read as "never taller than a plain jump clears with a wide margin, never full height". Its slide later flattens it to a 16 px remnant |
| DV-2 | "Protected far-side exit, always reachable" | The exit is geometrically reachable in every pile state, but a **door is closed until her defeat** (as the Juicer's gate) | K1 ties the dynamite grant to her defeat; skipping her would skip the grant. The door is a solid that opens on the defeat tick |
| DV-3 | "Hoarder silhouette emerges from one pile" | She emerges from a third, **non-interactive lair mound** at the arena centre, not from bank A or B | Keeps both prop state machines free of a spawn rule; the mound is a static backdrop prop with **no collision** and no gameplay state |
| DV-4 | Bank B at x 736..864 | Footprint x 752..880, support post x 808..824 | Sixteen px right so a baited shove into the post leaves her under the footprint from either side ([section 6](#6-pile-b-j-b-the-hanging-stack)) |
| DV-5 | (unstated) Magic Missile and the support | A Magic Missile is a qualifying hit on the J-B support | "Existing hit rules" include the missile; it gives Donut a real role in the collapse route. Needs one stimulus variant ([section 9](#9-interfaces-to-the-shared-cores)) |

| DV-6 | (unstated) #178 has no entry gate; it asks for an unbroken left-to-right floor route and a checkpoint before the first danger | An **entry gate** at x 208..224 closes behind the heroes once all are inside (the Juicer rule) and opens on her defeat, a retry or a restart ([section 7](#7-brain-states-targeting-and-choices)) | Without it, a fast ranged hero can stand left of her leash, beyond her reach, and fire at no risk (the leash kite loop, review round 2 N2). The gate removes that band. It does not contradict #178: the forward route to the exit, the entry pocket and its checkpoint are unchanged, and nothing traps a hero against a full-height wall inside the arena. Lever: `hrdq41_gate_enabled` |

No other #178 rule is changed. The dynamite placement follows #178 through K1.

## 3. Tactical role and the room

**She is a slow, heavy floor boss who owns the space in front of her. Close up she rears and slams both hands down (the swipe); at middle range she crouches and heaves her whole body forward along the floor (the shove). Both are long, readable and committed, and both leave her open. The shove is the attack the player baits: into bank A to rearrange cover, or into the support of bank B to bring the stack down on her.**

| Range | What she does | Main answers |
| --- | --- | --- |
| Close (pivot distance 88 px or less, in front) | Swipe: 34-tick tell, slam box 42..81 px ahead, 0..44 px high | Jump in place, step back past 92 px (Carl) or 90 px (Donut), or be behind her; punish the 48-tick recovery |
| Middle (96..176 px, in front) | Shove: 40-tick tell, then a lunge of up to 144 px at 180 px/s, box 0..48 px high | Jump the lunge (from 96..128 px you land behind her), put bank A or the support post between you and her, punish the whiff (48) or the jam (72) |
| Far, or a hero she cannot reach | Shuffle at 45 px/s; walk to bank A's face and push it down under a percher ([section 5](#5-pile-a-j-a-the-shuddering-bank)); give up a target she cannot reach from her leash edge and go home ([section 7](#7-brain-states-targeting-and-choices)) | Ranged pressure, reposition, bait |

### Room greybox `f01_hoarder` (HRD-Q29, data in the room file and its sidecar)

All x in world px from the room's left edge, heights above the walking line, 16 px grid.

| Part | x | Content |
| --- | --- | --- |
| Entry pocket (protected) | 0..208 | Checkpoint at x 64; no hazard, footprint, leash or attack box reaches it (her swipe box from the leash minimum 290 ends at 209) |
| Entry gate | 208..224 | Full-height solid, open until every living hero is inside (x 236 or more) after the engage; then closed until her defeat, a retry or a restart (DV-6) |
| Approach | 208..352 | Floor; the lair mound is visible from x 240 (stir cue, presentation only) |
| Bank A (J-A) | 352..416 | Solid 64 x 48; its top is the only raised standable surface (the H0 vantage) |
| Arena | 416..880 | Floor; lair mound (no collision) and her home at x 608, facing -1; her leash **290..880** (pivot) |
| Bank B (J-B) | 752..880 | Hanging stack 96..160 px up (no collision); support post 808..824, 0..64 px (non-solid hit target) |
| Exit gap | 880..928 | Floor, never under a footprint |
| Door | 928..944 | Full-height solid until her defeat |
| Exit pocket (protected) | 944..1056 | Room goal zone; sealed by the closed door for as long as she can attack (her +x swipe box from pivot 880 reaches 961, but only through the door) |

Room width 1056 (#178 range 960..1152). Lane ceiling **228 px** declared through `ceiling_px` (DP-Q45: bank A's top is above 42 px). Engage line: a hero's x reaches **432** (the arena floor past bank A); the gate closes and the reveal starts 8 ticks later at the earliest ([section 7](#7-brain-states-targeting-and-choices)).

## 4. Moves and frame data (60 Hz, all data knobs)

Attack tick 0 = the tell's first tick. **Counting convention for states and props:** the tick a state is entered is its tick 1, so a state of N ticks entered on tick T occupies T..T+N-1 and the next state starts on T+N. Boxes are pivot-relative: x forward (mirrored by the facing), y up from the walking line, half-open `[a, b)`. Facing and target lock at the tell start. The hero step runs before her step in a tick (the Juicer order), so a hero's same-tick hit lands first. Hand-computed tick edges below use the real hero integrator (`step_hero_driven`: launch on the press tick, then gravity, then position); the executor's unit test reproduces them, and any edge that differs by more than 1 tick is a design defect to report, not to re-decide.

### 4.0 Her body box per mode (the Juicer rule)

| Modes | Body box (collision and push-out) | Hurtbox |
| --- | --- | --- |
| Dormant | none | none |
| Reveal | none until its last tick, then standing | none |
| Neutral, Walk, Turn, every tell, Smear, Impact, Recover, Jammed, Flinch, Cooldown | **standing** `[-28, 28) x [0, 104)` | standing body plus head `[1, 21) x [88, 110)` |
| Lunge | **none: not solid, pushes nobody**; the shove box alone decides | standing body plus head |
| Exposed, Get-up | **prone** `[-48, 72) x [0, 48)` (Carl's 88.5 px apex clears it) | prone box |
| Defeated | none | none |

**Push-out:** a hero overlapping the body box is moved out at most **6 px per tick**, toward the **nearer** edge of the box, unless fewer than **28 px** (`min_side_room`) are free before a solid on that side, in which case toward the other side; never into a solid. **Free room is measured in the hero's own height band, from its feet up:** a solid whose top is at or below the hero's feet does not count (bank A under a percher on its top is not an obstacle to that percher's push). Push-out resumes on the first tick after the lunge. The body is never standable and deals no contact damage. She ignores remnants of 16 px or less for body collision (she walks over them, and a remnant that appears under her never moves her).

### 4.1 Swipe (the HOARD-01 slam)

| Phase | Attack ticks | HOARD-01 frames | Boxes | Hits on her |
| --- | --- | --- | --- | --- |
| Tell: gather | 0..7 | 7 (8 ticks) | hurtbox | Armour: damage and hit-stop, never an interrupt |
| Tell: rear-up hold | 8..33 | 8 (26) | hurtbox | Armour |
| Smear | 34..36 | 9 (3) | hurtbox, no attack box | Armour |
| Impact | 37..42 | 10 (6) | **slam box `[42, 81) x [0, 44)`** on each of the 6 ticks; hurtbox | Armour |
| Recover | 43..90 | 11 **held 48** (authored 24; playback only) | hurtbox | **Free damage**, no poise |
| Cooldown | 91..120 | idle or walk | hurtbox | Poise counts |

Each hero is hit at most once per swipe: **2 damage**, knock **160 px/s** away from her pivot and **200 px/s** up, through the ordinary hero hurt path (that path's invulnerability, hurt ticks, hit-stop and solo guards).

**Reach (standing hero):** hurtbox overlap with `[42, 81)` means Carl (half-width 12) is hit at pivot distance 31..92 and Donut (10) at 33..90. Her standing body pushes heroes out to 40 px or more (Carl), so **every hero in front of her within 92 px is in the slam**, including one close enough to jab her (40..48).

| Answer | Carl | Donut (base / boosted) |
| --- | --- | --- |
| Jump in place (box top 44; feet at 44 px or more on jump ticks 8..42 for Carl, 7..50 base, 6..56 boosted) | Press on attack ticks **1..30**; lands on jump tick 50 = attack tick press + **49**, at most **79**: 12 Recover ticks left (79..90) | Press on attack ticks **0..31** / **0..32** |
| Step back from pivot distance 44 | Running away must start by attack tick **22** (49 px to 93 at 3.08 px per tick, 16 ticks) | By attack tick **24** (47 px to 91 at 3.45 px per tick, 14 ticks) |
| Be behind her | Safe; she turns in 20 ticks before her next tell | Same |
| Hit her during the tell | Damage lands (armour), then still jump or leave | Same; a swat adds poise only |

### 4.2 Shove (the committed displacement attack, needs HOARD-02)

| Phase | Attack ticks | Stand-in frames (labelled STAND-IN until HOARD-02) | Boxes and motion | Hits on her |
| --- | --- | --- | --- | --- |
| Tell: crouched brace | 0..39 | 7 held 40 ticks, drawn 4 px back (wind-up) | hurtbox; no motion | Armour |
| Lunge | 40..87 at most (lunge tick k = 1..48 is attack tick 39 + k) | 9 held, translated | **3 px per tick forward (180 px/s), 144 px at most**; **shove box `[20, 48) x [0, 48)`**; step first, then test; **her body is not solid** (section 4.0) | Armour |
| Recover (whiff, max distance or leash edge) | 48 ticks | 11 held | stops | **Free damage** |
| Jammed (pile A face, J-B post, any solid) | 72 ticks | 12 for 12 ticks, then 11 | stops at the clamp | **Free damage** |
| Cooldown | 30, then the shove waits for its own 180-tick cooldown | idle or walk | | Poise counts |

**One lunge tick, in this order** (her body is not solid on any lunge tick, so she never pushes a hero; she passes under a jumping hero): (1) **step** 3 px forward; if the box front would pass the face of bank A while it is `stable` or `disturbed`, the near face of the J-B post while it is `supported` or `weakened`, or any intact solid (the door, a room wall), clamp her so the box front rests on that face (never moving her backward) and mark `met_obstacle`; the leash edge clamps her as a whiff; (2) **heroes:** every hero whose hurtbox overlaps the box and who was not hit by this shove takes **1 damage** and a knock of **300 px/s in the shove direction, 180 px/s up** (the displacement); the shove **continues**; (3) **obstacle:** `met_obstacle` sends the prop stimulus of [section 9](#9-interfaces-to-the-shared-cores) to that prop (none for a plain solid), logs `PileBoss(Jammed)` and enters Jammed; (4) **distance:** after lunge tick 48 she enters Recover.

**Reach (hero standing still, no obstacle):** the box front reaches `48 + 3k`, the back `20 + 3k`.

| Start distance | Carl hit on lunge tick (attack tick) | Carl jump-in-place clears if pressed on attack ticks | Donut base clears | Donut boosted clears |
| --- | --- | --- | --- | --- |
| 96 (trigger minimum) | 13 (52) | 28..45 | 19..46 | 13..47 |
| 128 | 23 (62) | **38..55** | 30..57 | 24..58 |
| 160 | 34 (73) | 47..66 | 39..68 | 33..69 |
| 176 (trigger maximum) | 39 (78) | 47..71 | 39..73 | 33..74 |
| 204 or more for Carl, 202 or more for Donut | never | any | any | any |

Carl's window is about 18 ticks and starts around the lunge start: jumping *as she lunges* is the read. Running away only escapes when started early in the tell (she lunges at 180 px/s, Carl runs at 185). Bank A and the J-B post are the safe answers: a hero behind either is never reached.

**The payoff of the jump: the flank.** She travels the full 144 px under a hero who jumps the shove. From a start distance of 96 or 128 that hero comes down **behind** her (from 128: she ends at pivot E - 144, Carl at E - 128, 16 px behind her pivot, and the push-out that resumes after the lunge moves him to her back edge). Her Recover (48) plus her Turn (20) is the punish. This is how Carl gets behind her: he can never jump her 104 px standing body. From 160 or more the hero lands in front of her and is pushed out to the front.

### 4.3 Hit reactions, armour and poise (the Juicer rules)

- **Armour:** every tell, the swipe smear and impact, and the lunge: hits deal damage and hit-stop, never interrupt.
- **Free damage, no poise:** Recover, Jammed, Flinch, Exposed and Get-up.
- **Poise:** hits in Neutral, Walk, Turn and Cooldown add their damage (a swat adds 1 with 0 damage). **4 within 120 ticks** → **Flinch 12 ticks** (frame 12), then Cooldown 30. For **60 ticks** after a flinch no poise accumulates (no flinch lock from missiles).
- **Never:** launched, knocked down, wall-slammed, bowled, stomped into `Downed` or pierced. The push kick deals its damage without a launch, as on the Juicer.
- **Damage taken:** every merged hero hit at its merged damage (Carl's moves 1, Magic Missile 1, pounce its effect damage, swat 0). Under `--f01-loadout all` only: dynamite 4 (CP-Q25 rule), the gauntlet doubles poise and does not pierce (CP-Q40 rule).

### 4.4 Exposure (the J-B payoff)

On the J-B landing tick, if the footprint overlaps her body box, whatever her mode (in a mode without a body box, including the Lunge, the standing box is tested): **6 damage, never lethal** (her HP floors at 1), then **Exposed 150 ticks**, then **Get-up 24**, then Cooldown 30. Exposed and Get-up have no attack boxes. Her body and hurtbox while Exposed and in Get-up are the prone box `[-48, 72) x [0, 48)` (HOARD-01 frame 15's bbox, rounded; section 4.0), which every merged hero hit can reach. Stand-in: frame 11 held under the landed remnant (alive, eyes open); never frames 13..15, which mean defeat (the Engineer U1 lesson). The finishing blow is always a hero's.

## 5. Pile A (J-A): the shuddering bank

States (#178): `stable` → `disturbed` → `slid` → `settled`, once per attempt. With the disturbance on tick T (for a baited shove, the jam tick): `disturbed` T..T+23, `slid` **T+24..T+39**, `settled` from T+40.

| State | Collision | Duration | Notes |
| --- | --- | --- | --- |
| `stable` | Solid `[352, 416) x [0, 48)`, standable top | until a stimulus | Real cover: blocks her walk and shove, Magic Missiles and any projectile |
| `disturbed` | same solid | **24 ticks** (#178 range 18..30) | Shudder, dust, a floor scuff along the slide path (presentation) |
| `slid` | the solid is removed on the first tick; a moving mass (hazard, not a solid) translates **-x 3 px per tick for 16 ticks**, from `[352, 416)` to `[304, 368)`, 0..48 px | 16 ticks | A hero standing on top falls |
| `settled` | Solid remnant `[296, 376) x [0, 16)` | rest of the attempt | Traversable by both heroes and by her (she steps over 16 px or less). A hero overlapping the remnant on the tick it appears is lifted onto its top (a 16 px step), never snapped sideways |

- **Stimuli that disturb it:** her shove meeting its face (the bait), a launched body (BOIL-D) and a blast (CP-C hook, test loadout only). **Hero melee and missiles never disturb it**, so cover cannot be knocked over by accident.
- **Slide push (as #178: "push/stagger actors once per motion token"):** on each slide tick, a hero whose hurtbox overlaps the moving mass takes one blow per motion token: **0 damage**, knock **180 px/s toward -x, 160 up** (the push and the stagger of the ordinary hero hurt path; HP never changes), never evadable, every hero (K3). She is never on the -x side of a stable bank, so it never touches her. The slide costs a percher the vantage and costs a baiter position, never HP: the terrain route is not taxed.
- **Attribution:** the disturbance is credited to the player of the hero she was targeting (the bait), like the Engineer's bonk.
- **Nest rule (the ledge, K8):** while bank A is `stable` or `disturbed` and her walk target is on its top or at x below 416, her walk goal is **the face itself (pivot 444), with no standoff** ([section 7](#7-brain-states-targeting-and-choices), choice 5). When bank A is `stable`, she is at that goal (pivot within 4 px of x 444) and her target is still on its top or behind it for **60 consecutive ticks**, she shoves bank A (`NestPush`). **The nest timer** counts every tick, **in every mode** (Neutral, Walk, Turn, a tell, Recover, Jammed, Flinch, Cooldown), on which her pivot is within 4 px of 444 and at least one present hero's hurtbox left edge is below 416, **at any height**. It is keyed on hero x only: a jump, a hit on her, a Flinch, or a target switch (including between two heroes who both qualify) never resets it. It resets only when no present hero qualifies or her pivot leaves the face. When it reaches 60 she pushes at the first tick she is in Neutral or Cooldown (the push is a shove, so it waits for any attack or recovery in progress, never for the shove cooldown). The shove jams at once (no motion) and disturbs it. A target behind the bank at 96..176 px triggers the ordinary shove sooner, which also jams on the face. A percher anywhere on the top (pivot up to x 427 for Carl, 425 for Donut) is covered: no percher can hold her 64 px away, and a percher her standing body overlaps at the face is pushed back onto the top, not off it. So bank A's top is a vantage, never a firing nest: after one slide no raised surface above 16 px is left in the room.

## 6. Pile B (J-B): the hanging stack

States (#178): `supported` → `weakened` → `released` → `falling` → `landed`, once per attempt.

| State | Duration | Notes |
| --- | --- | --- |
| `supported` | until a qualifying hit or a heavy impact | Post `[808, 824) x [0, 64)` (non-solid hit target); stack `[752, 880) x [96, 160)` with no collision |
| `weakened` | until a second qualifying hit or a heavy impact | Cracked post (presentation) |
| `released` | **24 ticks** warning | Shudder, falling dust, the floor shadow of the footprint |
| `falling` | **8 ticks** | With the release on tick R: `released` R..R+23, `falling` R+24..R+31; the landing evaluation runs on the 8th falling tick, **R + 31**; `landed` from R+32 |
| `landed` | rest of the attempt | Solid remnant `[752, 880) x [0, 16)`, traversable; a hero overlapping it on the landing tick is lifted onto its top (a 16 px step); she ignores it for body collision |

- **Qualifying hit:** a distinct move instance of a **human-controlled** hero whose active box overlaps the post (Carl's melee moves, Donut's swat and pounce, a Magic Missile), weight 1 each (the per-move support-hit weight of CP-D; a gauntlet haymaker's 2 only applies under the test loadout). **Two** → released. Hits before the engage line is crossed, and hits from a bot-driven hero, do nothing (the Juicer rule "the AI never triggers a rack").
- **Heavy impact:** her shove meeting the post → released at once. A blast with `breaks_structures` (test loadout) → released.
- **Landing (one swept evaluation on the landing tick, footprint `[752, 880) x [0, 48)`):** each hero overlapped takes **1 damage**, knock **120 px/s** away from the footprint centre and **140 up**, never evadable, every hero. Her body overlapped → [Exposure](#44-exposure-the-j-b-payoff). One outcome per overlap, no per-tick aura.
- **Why the bait works from both sides:** a shove clamped on the post's left face leaves her pivot at 760 (body 732..788, 36 px under the footprint); on its right face at 872 (body 844..900, 36 px under). Jammed lasts 72 ticks (J..J+71) and the landing evaluation runs on J+31, so a baited shove always ends with her under the stack. A hero standing beyond the post is never reached by that shove (the box stops on the post), but stands in the footprint and has 31 ticks to leave (from the footprint's centre Carl needs 25 ticks to clear its edge by his half-width).
- **Attribution:** the release is credited to the hitter of the second hit, or to the player of the hero she targeted for a baited shove.
- **Her own shove at a bot hero:** "the AI never triggers a pile" covers the bot's own hits only. Her shove aimed at a bot-driven hero still jams on the J-B post (released at once) or on bank A's face (disturbed), as for any target. The credit is then **`None`** (no player baited it); the prop still resolves, and the grant and the defeat are unaffected.

## 7. Brain: states, targeting and choices

**States:** `Dormant` → `Reveal` → `Neutral` (with `Walk`, `Turn`, `Cooldown`) ⇄ swipe (`SwipeTell`, `SwipeStrike`, `SwipeRecover`) / shove (`ShoveTell`, `Lunge`, `ShoveRecover`, `Jammed`) / `Flinch` / `Exposed` → `GetUp`; `Defeated`.

- **Entry gate (the Juicer rule, DV-6, HRD-Q41):** a full-height gate solid at x **208..224** stands open at the start. It closes only when a hero has crossed the engage line (x 432), **8 ticks** have passed since, and **every living hero** stands at x **236** or more (Carl's left edge clears 224). In 2P it waits for P2 indefinitely, and she stays Dormant until then. In solo, if the bot hero is still outside **120 ticks** after the controlled hero crossed, it is placed at x **248** before the gate closes (the Juicer's `regroup` output). `PileBoss(Engaged)` fires on the closing tick. The gate opens on her defeat (with the door) and is open again in a fresh attempt after a retry or checkpoint restart. With the gate closed, every hero on the floor is inside her reach from the leash edge: a hero at x 236 has its hurtbox at 224..248, inside the swipe box `[209, 248)` from pivot 290. No unreachable band is left.
- **Dormant:** no hurtbox, no body, no attack. **Reveal** starts on the entry gate's closing tick: **90 ticks** on the first entry, **30** after a retry or checkpoint restart; no hurtbox and no attack during it; she becomes solid on its last tick (heroes inside her body are pushed out).
- **Eligible hero:** present, and feet at most **44 px** above the walking line (her blows' height). A hero on bank A's top (48) or in the air is not a swipe or shove target.
- **Target:** the Juicer rule over eligible heroes (nearest by pivot distance, **24 px** hysteresis, solo bias **+40 px** on the bot hero). With no eligible hero, the same rule over all present heroes picks whom she walks toward. Locked at the tell start.
- **Choice each Neutral tick (after Cooldown), in this order:** (1) the nest rule ([section 5](#5-pile-a-j-a-the-shuddering-bank); its timer counts in every mode); (2) the disengage rule (below); (3) target behind her by more than 8 px → **Turn** (20 ticks, walk frames); (4) eligible target in front at **88 px or less** → swipe; (5) eligible target in front at **96..176 px** and the shove cooldown done → shove, **unless the lunge would end at a leash edge before reaching the target**: she does not shove when the target's hurtbox lies wholly beyond the box front at the edge clamp (x **242** = leash minimum - 48 facing -x, x **928** = leash maximum + 48 facing +x); she walks instead; (6) walk toward the target at **45 px/s** and stop at **64 px**; **except** while bank A is `stable` or `disturbed` and the walk target is on its top or at x below 416: then her walk goal is the bank's face (pivot **444**) with **no standoff**, and the nest timer counts while she is at that goal.
- **Leash:** her pivot stays in **290..880**; solids still block her (bank A while stable). At the leash minimum her swipe box facing -x is `[209, 248)`, outside the entry pocket (0..208) and reaching the gate's inner face. If no hero is within 320 px of her for **240 ticks** she walks back to x 608. No HP regeneration, ever; poise clears.
- **Disengage (a safety net; with the entry gate closed no floor hero can be unreachable):** a target is **unreachable** on a tick when her pivot is at a leash edge, the target is beyond that edge, and its hurtbox does not overlap her swipe box from her current pivot horizontally (Carl beyond 92 px, Donut beyond 90 px). After **90 consecutive** unreachable ticks, counted in every mode, she finishes her current attack or recovery and **walks home to x 608** (`PileBoss(Disengage)`). During the disengage walk she still answers a hero who follows her. The walk ends, and her ordinary choices resume (Turn included), as soon as an eligible hero is inside her swipe trigger (88 px) on either side. It also ends when an eligible hero is inside her shove trigger (96..176 px) on either side and is not beyond the leash edge she left. Hits alone still do not end it; they deal damage and add poise (a Flinch plays, then the walk resumes).
- **Home hold:** after a disengage or a 240-tick return she stays at x 608 facing the nearest hero and makes no walk choice until a hero is within **320 px** of her pivot (`hrdq19_return_px`); the swipe and shove choices still apply. So a ranged hero must come to x 288 or more to draw her out again. With the gate disabled (HRD-Q41 lever off), a fast ranged hero can repeat draw, retreat and fire; that loop is an HRD-D measurement ([section 11](#11-acceptance-criteria)).
- **Offscreen:** every tell starts within 176 px of its target, inside any 640 px view that frames that hero. No camera state is read.

## 8. Solo, 2P and the AI partner

- **Parity:** solo Carl, solo Donut and 2P use the same room, the same rules and the same goal (her defeat, then the exit). Each hero can trigger J-B and bait J-A when human-controlled.
- **Solo, Carl controlled, AI Donut:** M1 (the AI hero never drops below 1 HP) and M2 (the +40 px bias) apply. **Feed path: two supply sites, both ENEMY-SLOT's.** She is the room's single supplier of the existing `boss: Option<BossThreat>` input at both:
  - the **plain bot** (no `--companion`): `PartyStepCtx.boss`, built in `src/combat_scene.rs` (today from `self.boss` and `Boss::threat`) and read by `corridor.rs::step_live`;
  - the **companion path** (`--companion`): the follow layer and `CompanionView.boss` (`src/companion/view.rs`) in `src/combat_scene/companion.rs`, the boss argument of the follower's `follow_input` and `dodge_input`.

  She reports:
  - during the swipe tell, smear and impact: `BossThreat::Pound { x, facing, target: None }` with **`x = pivot + facing * 9`** (`hrdq33_pound_anchor_px`). **`target` is `None`** because her swipe hits every hero in the box: the follower steps out only when the pound targets it or nobody, so `None` makes an untargeted bot (following Carl at about 84..100 px from her) step out too. This `x` is a **dodge anchor, not her pivot**: the follower steps out while the gap to the anchor is at most its `POUND_STEP_OUT_PX` (84), so it stops at 93 px or more from her pivot, beyond her 92 px reach (9 = 92 - 84 + 1). The anchor is never used for hits, targeting or presentation;
  - during the shove tell and the lunge: `BossThreat::Charge { tell_left, x, dir, running, front: 48, depth: 28, speed: 3, lead_ticks: 18 }`, `x` her real pivot. With the body not solid during the lunge, the bot's press at hit tick - 18 falls inside every clear window.
  No AI code changes. **She is not in `CompanionView.targets` in v1:** the AI partner follows and dodges her, as with the Juicer, and never attacks her (otherwise the director would treat a 30 HP boss like a goblin and pounce into her slam). "AI Donut contributes missiles against a slot boss" is a recorded COMPANION follow-up, not v1. The J-A slide and the J-B footprint reach the AI through CA2-C's hazard feed (active and telegraphed rectangles), or are a reported gap until it lands (K13).
- **Measured, not required (as the Engineer's F12):** AI Donut swipe hits, shove hits and footprint hits per clear. More than **1 per clear** of any kind opens a COMPANION follow-up item. The AI partner is passive against her by design in v1 (the Juicer precedent); HRD-D records how that feels in solo Carl.
- **Solo, Donut controlled, AI Carl:** the same rules; AI Carl's existing follower and the same `BossThreat` feed; she is not in his `CompanionView.targets` either; he never throws dynamite (CP-Q29), and his own hits never trigger a pile. Her shove aimed at him can still jam on a pile ([section 6](#6-pile-b-j-b-the-hanging-stack), credit `None`).
- **2P:** no guards (M5). Each hero is hit at most once per swipe, per shove, per slide and per landing. No friendly fire. The swipe and the shove hit every overlapped hero, so a partner in the lane shares the blow.
- **Donut's roles:** missiles from range (poise in neutral, damage in punish windows), the swat for poise, a missile as the second J-B hit from a safe angle. With Light on Your Feet she can jump over her (feet above her 110 px head box on jump ticks 17..44, enough to cross her 56 px body at 207 px/s; the base jump cannot). That is intended asymmetric mobility, not a nest.
- **Reflex evasion (K6, DM-Q18):** the swipe, the shove, the J-A slide and the J-B landing are **never evadable**. Every blow carries the non-evadable class; nothing here opts in.

## 9. Interfaces to the shared cores

| Need | Owner (integration plan section 3, Revision 2) | Contract used here |
| --- | --- | --- |
| Her tick slot, hurtboxes for hero melee, missiles, blasts, contacts and env victims, emit-hero-hurt, sidecar roster spawn | **ENEMY-SLOT** (`src/scene_enemy.rs` and **its one hook line** in `src/combat_scene.rs`) | Her scene module `src/combat_scene/pile_boss.rs` (HRD-A2) plugs into ENEMY-SLOT's per-enemy slot. HRD-A2 adds **no** hook of its own in `combat_scene.rs`, `companion/**` or `contacts_cf02.rs` |
| Read access to env prop solids and hit targets in the slot step context | **ENEMY-SLOT** (interface), filled by **BOIL-C wire** | Her lunge clamp reads bank A's face and the J-B post from it. Until HRD-B, bank A is a sidecar static solid (see the next row) |
| Bank A before HRD-B | **HRD-A1** (sidecar data) | Bank A is a **static solid in the encounter sidecar**, never in the room file's blocks (the room file keeps `props`, `enemies` and `hazards` empty, and the validator rejects non-empty arrays). HRD-B upgrades that same sidecar entry in place into the J-A prop |
| Prop state machines, stimulus intake, swept landing, remnant, reset | **BOIL-B core** (stimulus enum), **BOIL-C core** (falling mass), in `src/env_prop.rs` | J-B is the falling-mass variant (support, warning, fall, one footprint, remnant) |
| **J-A slide motion** | **BOIL-C wire** appends `Slide { dx_per_tick, ticks, remnant }` to `src/env_prop.rs` (the plan's slide variant, `stable -> disturbed -> slid -> settled`); **fallback: HRD-B appends it** (append only, after BOIL-C wire) | `dx_per_tick` -3, `ticks` 16, remnant `[296, 376) x [0, 16)`. The **moving mass is an `src/env_hazard.rs` volume** (the plan's "slide push") |
| **Remnant under an actor** | **BOIL-C core** (remnant rule; BOIL-C wire if the core lands without it) | On the tick a remnant appears, an overlapped hero is lifted onto its top (a 16 px step), never snapped sideways (an invulnerable hero is not knocked out of a footprint). Slot enemies ignore remnants of 16 px or less for body collision |
| **Her shove as a stimulus** | **BOIL-B core** appends `Stimulus::EnemyImpact { source, credit: Option<u32> }` (heavy); if it ships without it, HRD-B appends it (append only) | Sent by her step on the jam tick; consumed in the env step of the same tick. `credit` is `None` when her target was a bot hero |
| **Magic Missile on a support** (DV-5) | **BOIL-B core** appends `Stimulus::Projectile { owner, weight }`; the scene test is **BOIL-C wire**'s; fallback HRD-C | Weight 1; HRD-Q26 can switch it off |
| Hazard volumes (slide push, footprint) | **BOIL-B core** and **wire**, `src/env_hazard.rs` | Activation token per victim, team policy all heroes (the shared `HeroDamagePolicy`), class never evadable, hero hurt through `HeroBlow`; the slide push carries 0 damage |
| Launched body into a prop | BOIL-D | J-A accepts it (no launched bodies exist in this room in the story loadout) |
| Blast on a prop | CP-C `BlastTarget` hook | Test loadout only (K1) |
| Encounter data, gate validation, reachability, the closed door | **BOIL-A core** (validator, the "exit locked until an event" rule), BOIL-A wire (physics-exact check) | Props, door, **entry gate**, protected rects, story pin, goal, K8 classes. The protected-rect check covers her **attack boxes at both leash edges**, not only the leash. HRD-A2 toggles the door and the gate through the sidecar's "solid toggled by event" hook (gate: closed on `Engaged`, open on defeat or a fresh attempt; door: open on defeat) |
| AI partner threat feed | **ENEMY-SLOT**, at both supply sites: `PartyStepCtx.boss` in `src/combat_scene.rs` (plain bot) and the follow layer plus `CompanionView.boss` in `src/combat_scene/companion.rs` (companion path) | She fills `boss` at both ([section 8](#8-solo-2p-and-the-ai-partner)), Pound with `target: None`; she is not in `CompanionView.targets` in v1 |
| Solo bot regroup before the gate closes | **HRD-A2** in its own module, mirroring `BossOut::regroup`; if ENEMY-SLOT's slot output has no hero-placement path, that is a one-append ENEMY-SLOT interface request to report, not a design choice | Places the bot hero at x 248 after 120 ticks outside |
| Defeat event | existing `CombatEvent::EnemyDefeated { tick, target, cause, force, instigator }` (`src/combat.rs`); ENEMY-SLOT adds no defeat variant | `target: PILE_BOSS_ID` |
| Non-defeat events | **HRD-A2** appends one `CombatEvent::PileBoss { tick, event }` (joins the `combat.rs` queue, with its hash-mix and journal arms) | HRD-Q40 |
| Story trigger | **CP-A** ledger; the matcher code and the room's scene kind are **HRD-A2**'s ([section 10](#10-defeat-death-and-the-story-event)) | `pile_boss_f01_defeated` = `EnemyDefeated { target: PILE_BOSS_ID }` |
| Tick order | ENEMY-SLOT (slot enemies), **BOIL-B wire owns only the env step** (`combat_scene/env.rs`) | Order: … llamas, Engineer, **slot enemies (her)**, contacts, shore, boss, charges, **env props and hazards**, morale |

Within one tick: hero strikes (on her hurtbox, and on the J-B post through the prop intake), her slot step (lunge clamp against the current prop solids, read from ENEMY-SLOT's slot step context), then the env step (stimuli applied, slide and landing evaluated, landing on her calls her `crush`).

## 10. Defeat, death and the story event

- **Defeat:** HP 0 from a hero's hit (never from J-B). On that tick: one existing `CombatEvent::EnemyDefeated { tick, target: PILE_BOSS_ID, cause, force, instigator }` (the final blow's existing cause, force and instigator; no new defeat variant), the door opens, her body loses collision, no new hazard starts (a stack already falling still lands and still hurts heroes).
- **Story event (K1):** the CP-A trigger row `pile_boss_f01_defeated` matches that event and grants `special_thrown_charge` on the same tick (one `CapabilityGranted`). CP-A ships the row with a placeholder zone. The swap to the event is **code, not only a data row**, because CP-A's `EventMatch` has only `EnemyDefeatedBoss` (keyed on `BOSS_ENEMY_ID` in `combat_scene/story.rs::observe_story`), and `armed_in` and `PinRow.scene` are `SceneKind` values (`corridor`, `demo`, `cf_arena`, `boss`). HRD-A2 lists and owns all three parts, queued behind CP-A and DP-A on those files:
  1. **the matcher:** one generic `EventMatch::EnemyDefeated` with a `target` field on `EventRow` (adopted; it serves later slot bosses too), in `src/story_unlock.rs` and `src/combat_scene/story.rs`. The existing `EnemyDefeatedBoss` arm stays byte-identical;
  2. **the scene kind:** a `SceneKind` value for the Hoarder room, so the trigger is armed there under `--carl-prog`. The room's story pin (biscuit read through, Light on Your Feet as Donut's `jump_reach_boost` from DP-A's row) is **PT-SCENE's**, which owns room pins in the plan; HRD-A2 adds only the scene kind and its arming;
  3. **the re-pin** of `assets/data/gameplay/story_unlocks_v0.json` after the row swap.
  The fixture `hrd_defeat_grants_dynamite` arms the trigger by id, as CP-A's fixtures do, so it does not wait for PT-SCENE.
- **Death presentation:** species `pile_boss`; every recipe, every cause (blast included), maps to HOARD-01 `down` 13..15 (buckle, floor, settled prone hold). No gibs, no dismemberment, no blood: a ruined person, not a target for spectacle. Not CINE-eligible. The corpse stays without collision; the piles keep their resolved states.
- **Aftermath (H5):** no further enemies; the grant notice follows the CP rules. A retry after defeat is not possible (the room is complete).

## 11. Acceptance criteria

Machine (CI): every fixture of [section 14](#fixtures-and-tests-per-step) passes and replays identically twice with a pinned golden; every existing golden named in the CP, DP and DM work orders stays byte-identical; `enemy_pile_boss_v0.json` loads, validates (an error names the field), matches its FNV-1a pin, and a completeness test proves every HRD-Q has a knob or a recorded no-knob reason; the reach tables of section 4 are reproduced by a domain unit test; the room passes the BOIL-A validator for Carl, base Donut, boosted Donut and 2P in all **five pile orderings: A only, B only, A then B, B then A, neither**.

Evidence (HRD-D, real Bevy, 640 x 360 and 640 x 400, 1x): reveal; swipe → jump → punish; shove → jump → punish; shove baited into bank A and the slide; nest push; two hits on the post; a baited shove into the post, landing and exposure finisher; a direct clear with both piles intact; the defeat and the grant; a 2P run (or **NOT RUN**); HOARD-01's **"effects muted"** capture (tells and blows still readable with dust, shake and flashes off); frame time; the AI counts of section 8.

**HRD-D measurements (fun watch list from review round 1; measure before tuning anything else):** time to a human direct clear against the 45..120 s target (HP 30 may be short: tell hits deal damage under armour); ranged kiting (does Donut at 207 px/s with regenerating missiles out-kite a 45 px/s boss; if so, the first candidate is a shorter shove cooldown when the target sits at 176..240 px); incidental J-A slides per run (any leftward shove from a pivot under 608 can jam on bank A); how a passive AI partner feels in solo Carl; **the leash kite loop** (review round 2 N2): with the entry gate on (default) and off (`hrdq41_gate_enabled = false`, the comparison), solo Donut draws her to the leash edge and fires from x < 200; report damage per cycle and time to clear. **Levers** if a zero-risk clear comes in under 120 s: (a) the entry gate (adopted as the default, DV-6); (b) a shorter `hrdq19_unreachable_ticks`; (c) a faster disengage walk.

Human (owner; the item stays "merged, game-feel review pending"): the encounter reads as dread, not comedy; the owner can tell swipe from shove before it lands; at least two of the three wins happen in three runs; a direct clear is not slow or frustrating (target 45..120 s for a human direct clear, tuned by HP in HRD-D).

## 12. Adopted defaults (HRD-Q01..HRD-Q41)

Knob names start with the question ID (`hrdq02_hp`). File: `assets/data/gameplay/enemy_pile_boss_v0.json`; room geometry in the room file and its sidecar.

| ID | Question | Adopted value | Rationale | Knob |
| --- | --- | --- | --- | --- |
| HRD-Q01 | Vocabulary | Swipe, shove, punish windows; one phase; no extra move in v1 | #178: a very small vocabulary; one more action only after playtest | none (design) |
| HRD-Q02 | Health | **30**, single phase | Juicer 36 at ~85 % with phases; she comes at ~40 % with the base kit; difficulty from commitment and geometry, not HP | `hrdq02_hp` |
| HRD-Q03 | Body and hurtbox | Per mode (section 4.0): standing body and hurtbox `[-28, 28) x [0, 104)` plus head `[1, 21) x [88, 110)`; **no body during the Lunge** (`hrdq03_body_solid_in_lunge = false`); prone `[-48, 72) x [0, 48)` in Exposed and Get-up; push-out at 6 px/tick toward the nearer edge, the other side when fewer than 28 px are free before a solid in the hero's own height band (a solid whose top is at or below the hero's feet does not count; never into a solid); not standable; no contact damage; remnants of 16 px or less ignored | HOARD-01 section 3; Juicer body rule (`solid_body` is `None` during `Charge`); the jump-over windows of section 4.2 and the flank depend on the non-solid lunge; no body-contact damage keeps the vocabulary small | `hrdq03_body`, `hrdq03_head`, `hrdq03_prone`, `hrdq03_push_px`, `hrdq03_side_room_px`; `hrdq03_body_solid_in_lunge` is a recorded constant, no knob (reason: code) |
| HRD-Q04 | Walk, turn, standoff | **45** px/s; turn **20** ticks; walk stop **64** px | Art contact plan; a slow turn makes the flank a real opening | `hrdq04_walk_px_s`, `hrdq04_turn_ticks`, `hrdq04_standoff_px` |
| HRD-Q05 | Engage and reveal | Engage at hero x **432**; the reveal starts on the entry gate's closing tick (HRD-Q41); reveal **90** ticks, **30** on retry; no hurtbox or attack in Dormant and Reveal; stir cue at x **240** (presentation) | H0/H1: no auto-hit during the reveal; rapid retry skips staging | `hrdq05_engage_x`, `hrdq05_reveal_ticks`, `hrdq05_retry_reveal_ticks`, `hrdq05_stir_x` |
| HRD-Q06 | Targeting | Juicer rule over eligible heroes (feet ≤ **44** px), hysteresis **24**, solo bot bias **40**; locked at the tell | Reuses Q3 B; perched or airborne heroes are not swiped | `hrdq06_eligible_px`, `hrdq06_switch_px`, `hrdq06_ai_bias_px` |
| HRD-Q07 | Swipe trigger | Eligible target in front at **88** px or less | Art estimate 80 + the box reaches 92; no hero in the slam is ignored | `hrdq07_trigger_px` |
| HRD-Q08 | Swipe timing | Tell **34** (8 + 26), smear **3**, impact **6**, recover **48**, cooldown **30** | HOARD-01 ticks; recover 48 so every clearing jump (the latest, pressed on attack tick 30, lands on attack tick 79) can punish before 91 | `hrdq08_*` |
| HRD-Q09 | Swipe hit | Box `[42, 81) x [0, 44)`; **2** damage; knock **160 / 200**; once per hero | HOARD-01 frame 10 box; Juicer pound damage; the most readable blow costs the most | `hrdq09_box`, `hrdq09_damage`, `hrdq09_knock` |
| HRD-Q10 | Shove trigger | Eligible target in front at **96..176** px; shove cooldown done | Middle range only, distinct from the swipe; the gap closer for a 45 px/s boss | `hrdq10_range_px` |
| HRD-Q11 | Shove timing | Tell **40**; lunge **3** px/tick (180 px/s) for at most **48** ticks (**144** px); step, then test | Never faster than Carl's 185; a readable lunge start; reaches Carl at 203 px or less, Donut at 201 or less | `hrdq11_tell_ticks`, `hrdq11_px_per_tick`, `hrdq11_max_ticks` |
| HRD-Q12 | Shove hit | Box `[20, 48) x [0, 48)`; **1** damage; knock **300 / 180** in the shove direction; once per hero; the lunge continues | The displacement is the threat; an 18-tick jump window for Carl | `hrdq12_box`, `hrdq12_damage`, `hrdq12_knock` |
| HRD-Q13 | Shove ends | Whiff or leash → Recover **48**; pile face, post or solid → Jammed **72**; clamp never moves her backward | A baited jam pays more than a dodge; 72 > 32 keeps her under J-B | `hrdq13_recover_ticks`, `hrdq13_jam_ticks` |
| HRD-Q14 | Shove cooldown | **180** ticks after the shove ends | At most one shove every ~4.5 s; she is not a lane spammer | `hrdq14_cooldown_ticks` |
| HRD-Q15 | Armour | Tells, smear, impact, lunge: damage and hit-stop, no interrupt; Recover, Jammed, Flinch, Exposed, Get-up: free damage | Juicer super armour; "interrupt only when a real vulnerability allows" | none (code) |
| HRD-Q16 | Poise | **4** damage in **120** ticks in Neutral, Walk, Turn, Cooldown → Flinch **12**; **60** ticks of no poise after it | Juicer values; no missile flinch lock | `hrdq16_poise`, `hrdq16_window`, `hrdq16_flinch_ticks`, `hrdq16_immune_ticks` |
| HRD-Q17 | Damage taken | Merged hero damage; never launched, knocked down, slammed or pierced; test loadout: dynamite 4, gauntlet poise x2 | Juicer plug rules; K1 base kit in the story | none (reuses CP-Q25, CP-Q40) |
| HRD-Q18 | Exposure | J-B landing on her body: **6** damage, HP floor **1**, Exposed **150**, Get-up **24**; overrides any mode; prone hurtbox `[-48, 72) x [0, 48)` | A big, earned payoff (about 40 % of her HP with the finisher hits) that is never an automatic kill | `hrdq18_damage`, `hrdq18_floor_hp`, `hrdq18_exposed_ticks`, `hrdq18_getup_ticks`, `hrdq18_hurtbox` |
| HRD-Q19 | Leash, disengage and reset | Pivot **290..880**; disengage home after **90** consecutive ticks with the target unreachable beyond a leash edge (hits alone do not end the walk; it ends, and her ordinary choices resume, Turn included, as soon as an eligible hero is inside her swipe trigger on either side, or inside her shove trigger on either side and not beyond the leash edge she left); return home (x **608**) after **240** ticks with no hero within **320** px; home hold until a hero is within 320 px; no regeneration | No frustrating heal. **290** keeps her swipe box (`[209, 248)` facing -x) out of the protected entry pocket 0..208; 256 let it reach 175. The disengage rule removes the static snipe; see the HRD-D kite measurement. A hero who follows her home is answered, never given a free back. With the entry gate closed (HRD-Q41) the disengage is a safety net only | `hrdq19_leash`, `hrdq19_unreachable_ticks`, `hrdq19_home_x`, `hrdq19_return_ticks`, `hrdq19_return_px` |
| HRD-Q20 | Evasion class | **Never evadable**: swipe, shove, slide, landing; the loader rejects another value | K6, DM-Q18 boss blows and hazards; DM-Q18 is an allow-list, so `never` is already the default | `hrdq20_evade_class` (`never` only) |
| HRD-Q21 | J-A geometry and timing | Stable `[352, 416) x [0, 48)`; warning **24**; slide **-x 3 px/tick for 16** ticks (disturbed on T: slid T+24..T+39); settled remnant `[296, 376) x [0, 16)` | #178 18..30 warning; one authored direction away from her, toward the protected entry but never into it | sidecar `hrdq21_*` |
| HRD-Q22 | J-A stimuli | Her shove, a launched body, a blast; never hero melee or missiles | Cover is real and cannot be knocked over by accident | sidecar `hrdq22_stimuli` |
| HRD-Q23 | J-A slide push | **0** damage, knock **180 / 160** toward -x (push and stagger), once per hero per motion | #178 J-A: "push/stagger actors once per motion token". The slide mostly hits the hero who baited it, so damage would tax the terrain route; the push still costs a percher the vantage | sidecar `hrdq23_*` |
| HRD-Q24 | Nest rule | With her walk target on bank A's top or at x < 416 while the bank is `stable` or `disturbed`, her walk goal is the face (pivot **444**, no standoff); at that goal for **60** ticks with a hero still there → she shoves bank A; the timer counts in every mode, is keyed on hero x only (hurtbox left edge < 416, any height) and is never reset by a jump, a hit, a Flinch or a target switch | K8: the vantage is never an untouchable nest, wherever the percher stands on the top | `hrdq24_nest_ticks` (0 = off), `hrdq24_face_x` |
| HRD-Q25 | J-B geometry | Post `[808, 824) x [0, 64)` non-solid; stack `[752, 880) x [96, 160)` no collision; footprint `[752, 880) x [0, 48)`; remnant 16 px | DV-4; the floor route stays open; no debris past 880 | sidecar `hrdq25_*` |
| HRD-Q26 | J-B stimuli | **2** distinct qualifying hits (human melee, swat, pounce, missile; weight 1) or **1** heavy (her shove, a blast); only after engage; bot hits ignored | #178 candidate numbers; Juicer "AI never triggers a rack"; no premature release | sidecar `hrdq26_hits`, `hrdq26_missile` (switch) |
| HRD-Q27 | J-B timing | Released **24**, falling **8** (released on R: falling R+24..R+31); the landing evaluation runs on **R + 31**, once; `landed` from R+32 | #178 24-tick warning | sidecar `hrdq27_*` |
| HRD-Q28 | J-B hero blow | **1** damage, knock **120 / 140** away from the centre, every hero, credited to the initiator | Juicer rack hero damage; K3 world hazards hurt every hero | sidecar `hrdq28_*` |
| HRD-Q29 | Room | Section 3 table: 1056 px, ceiling 228, door 928..944 | #178 geometry; DP-Q45 | room and sidecar |
| HRD-Q30 | Ledges (K8) | Raised standable surfaces: bank A's top (48) while stable or disturbed; remnants 16; nothing at 49..128; boosted Donut may jump over her | Carl 80 and Donut 128 classes both satisfied; the only perch is threatened by the nest rule | validator rule |
| HRD-Q31 | Defeat event | Existing `EnemyDefeated { tick, target: PILE_BOSS_ID, cause, force, instigator }`, `PILE_BOSS_ID = 0x504C` (also her `HeroHurt` source); once; door opens on that tick | K1 trigger; disjoint from `0x4255`, `0x5241` and the Engineer ram stamp space | none (constant) |
| HRD-Q32 | Death presentation | Species `pile_boss`, every recipe → HOARD-01 13..15; no gore for any cause; not CINE-eligible | Tone: a ruined person, horror without gratuitous gore | death profile data |
| HRD-Q33 | AI partner | `BossThreat` feed at ENEMY-SLOT's two supply sites (`PartyStepCtx.boss` and `CompanionView.boss`): Pound with the dodge anchor **9** px ahead of her pivot and `target: None` (her swipe hits every hero in the box), Charge with lead **18**; not in `CompanionView.targets` in v1; no AI code change; M1, M2; follow-up above 1 AI hit per clear | Reuses the Juicer's M3 consumer; the anchor makes the follower's 84 px step-out clear her 92 px reach; AI belongs to CA2 | `hrdq33_pound_anchor_px`, `hrdq33_ai_jump_lead` |
| HRD-Q34 | Stand-in frames | Section 4 mapping, labelled STAND-IN; HOARD-02 replaces them | K10 | none (presentation) |
| HRD-Q35 | Audio | Existing cues only (boss tell, hit, impact, debris); no new asset | No invented sound in v1 | none |
| HRD-Q36 | HUD | **No boss bar** in v1 | Plan: setpieces add no HUD; #178 tone (no hit counter); avoids the HUD hot files | `hrdq36_boss_bar` (`false`) |
| HRD-Q37 | Gate | Active only in `f01_hoarder` under `cf02_rules`, reached by HRD-A2's scene arm and later `--f01-room hoarder`; data kill switch `hrdq37_enabled` | No existing route or golden changes | `hrdq37_enabled` |
| HRD-Q38 | Fairness floors | Every tell ≥ **24**, every punish window ≥ **24**, every prop warning ≥ **18**; checked on load | Juicer floors; #178 warning range | `hrdq38_min_tell`, `hrdq38_min_punish`, `hrdq38_min_warning` |
| HRD-Q39 | Hits per hero | At most one per swipe, shove, slide motion and landing; in 2P every overlapped hero | Bounded hit tokens (slate invariants) | none (code) |
| HRD-Q40 | Events | For the **non-defeat** events only: one appended `CombatEvent::PileBoss { tick, event }` with `PileBossEvent::{Engaged, Tell { attack, target }, Strike { attack }, Hurt { source, damage, hp_left }, Flinch, Jammed { obstacle }, Exposed { credit }, GetUp, NestPush, Disengage}`. Defeat uses the existing `EnemyDefeated`; hero hits reuse `HeroHurt`. **HRD-A2 joins the `combat.rs` queue** for this one variant, with its hash-mix and journal arms | One variant keeps `combat.rs` churn minimal; `boss.rs` is never edited (plan section 7) | none (code) |
| HRD-Q41 | Entry gate | Full-height gate solid at x **208..224**; closes when a hero has crossed x 432, **8** ticks have passed and every living hero is at x **236** or more; 2P waits for P2 indefinitely (she stays Dormant); solo bot regroup to x **248** after **120** ticks; open on her defeat, a retry or a restart | The Juicer's entrance gate (`GateData`); removes the band left of her leash where a ranged hero is beyond her reach (DV-6) | `hrdq41_gate_enabled`, `hrdq41_gate_x`, `hrdq41_inside_x`, `hrdq41_settle_ticks`, `hrdq41_regroup_ticks`, `hrdq41_regroup_x` |

## 13. Genuinely open questions

1. **Feel:** the 34-tick swipe and 40-tick shove tells at 1x; whether the 18-tick shove jump window is fair for Carl at this point of the floor; whether 2 swipe damage is brutal or punishing.
2. **Collapse strength:** 6 damage plus 150 exposed ticks against the direct path; whether the collapse may ever kill her (adopted: never).
3. **Boss bar** (adopted: none) and the H0 sound (no asset exists).
4. **The door** (DV-2): owner may prefer an open exit with the grant moved to a later trigger.
5. **One more dangerous action** after the first playtest (#178 allows one; nothing is designed).
6. **HOARD-02 timing:** commission the shove, jammed, pinned, get-up and emerge keys, plus the pile props, now (parallel track, K10) or after the owner's feel review (adopted: parallel, not blocking).
7. **The entry gate** (DV-6, HRD-Q41): the owner may prefer an open arena; the lever turns it off, and HRD-D's kite measurement shows the cost.

## 14. Implementation brief (HRD-A1..HRD-D)

Medium tier. Do not edit `docs/04-production/backlog.md`; put status in the PR body. Do not edit `src/boss.rs`, `src/shore.rs`, `src/camera_lab/` or the HOARD-01 sources. No design decision is left to the executor: a fixture that contradicts this sheet is a design defect to report. Step names and ownership follow the integration plan's section 4.3 (Revision 2).

| Step | Depends on | Owns | Content |
| --- | --- | --- | --- |
| **HRD-A1** pure core | HRD-0, BOIL-A core (schema) | `src/pile_boss.rs` (new, Bevy-free), `assets/data/gameplay/enemy_pile_boss_v0.json` (new), `assets/levels/floor01/f01_hoarder.room.json` and `.encounter.json` (new; bank A and the J-B post as sidecar static entries, room `props`, `enemies` and `hazards` empty), headless unit tests | State machine: Dormant, Reveal, Neutral, Walk, Turn, swipe, shove, Recover, Jammed (bank A as a sidecar static solid, the post as a plain obstacle), Flinch, Defeated; body box per mode; push-out; entry gate; leash, disengage (with the follow answer) and home hold; nest walk goal and nest timer; targeting; the `BossThreat` values (Pound dodge anchor, Charge); fairness floors; data load, pin and completeness |
| **HRD-A2** wire and direct fight | HRD-A1, **ENEMY-SLOT**, BOIL-A wire, CP-A, DP-A (story files) | `src/combat_scene/pile_boss.rs` (new, the per-enemy scene module behind **ENEMY-SLOT's hook line**; HRD-A2 adds no hook of its own), the slot registration, her `boss` supply at ENEMY-SLOT's two sites (`PartyStepCtx.boss` and `CompanionView.boss`), the closed door and its runtime opening on the defeat tick, the entry gate toggles and the solo bot regroup, the appended `CombatEvent::PileBoss` (queued on `combat.rs`, with its hash-mix and journal arms), the story matcher, the room's `SceneKind` and the trigger row swap with the re-pin of `story_unlocks_v0.json` ([section 10](#10-defeat-death-and-the-story-event)), the scene arm (queued), `src/hoarder_fixtures.rs` (new), death profile overlay, stand-in presentation | The direct fight in the real scene; defeat and the K1 grant |
| **HRD-B** J-A | HRD-A2, BOIL-C wire | sidecar data (upgrades bank A in place into the J-A prop), bait and nest logic in `pile_boss.rs`; **fallback** owner of the `Slide` motion variant in `env_prop.rs` and of `Stimulus::EnemyImpact` (append only) | J-A states, slide push, nest push, attribution |
| **HRD-C** J-B | HRD-B | sidecar data, `pile_boss.rs`; **fallback** owner of `Stimulus::Projectile` and of the missile-against-support scene test | J-B states, qualifying hits, landing, `crush`, Exposed, Get-up |
| **HRD-D** evidence | HRD-C (HRD-A2 for a direct-fight-only packet) | `docs/evidence/f01-hoarder/` | Evidence, human runs, the HRD-D measurements of section 11, owner packet |

**Integration-plan notes for the orchestrator:**
- ENEMY-SLOT owns her tick slot, the one hook line in `combat_scene.rs`, the `CompanionView` row in `combat_scene/companion.rs` and the env-victim and env-solid access; BOIL-B wire owns only the env step. There is no HRD fallback hook: HRD-A2 waits for ENEMY-SLOT.
- HRD-A2's story-matcher and `SceneKind` work (section 10) is code in CP-A's `story_unlock.rs` and `combat_scene/story.rs`; the plan's CP-A row lists HRD-A2 only for the data row swap. The room pin stays PT-SCENE's.

### Fixtures and tests per step

All headless, under `cf02_rules`, golden-pinned. Unless stated: Carl only, her pivot at **E = 760** awake in Neutral facing -x, so Carl at distance `d` stands at x `760 - d` (Reveal skipped by fixture setup), piles in their initial states, the entry gate closed (and disabled by fixture setup where a hero starts at x < 236), distances are pivot to pivot. Prop and state tick numbers use the counting convention of section 4 (entry tick = tick 1).

**HRD-A1 (unit tests in `pile_boss.rs`):** data load, validation (an error names the field), pin, completeness, fairness floors; the swipe and shove reach tables of section 4 (the shove rows with the body not solid during the lunge); armour per mode; body box per mode and push-out side rule; the exposure HP floor; target lock at the tell; the nest walk goal for targets at x 340..427 on the top; **push-out on the bank top: Carl at x 427 on the top when she arrives at 444 ends at x 404 or less, still on the top** (the bank under his feet does not count as a solid, N3); the nest timer is not reset by a percher's hop, a hit, a Flinch or a switch between two qualifying heroes; the entry gate (closes only with every living hero at x 236 or more, 8 ticks after the engage; 2P waits; solo regroup at 120 ticks; opens on defeat and in a fresh attempt); the disengage counter, the follow answer and home hold; the shove guard at the leash clamp (no shove when the target's hurtbox lies wholly beyond x 242 facing -x or x 928 facing +x); the Pound dodge anchor (`pivot + facing * 9`); `hrd_shove_out_of_reach` as a **forced shove** with no walk: Carl at 203 hit, 204 clear; Donut at 201 hit, 202 clear; `hrd_evade_class_never`: every blow and hazard of this sheet carries the `never` class (DM-Q18 is an allow-list, so `never` is already the default).

**HRD-A2 (direct fight, `hoarder_fixtures.rs`):**

| Fixture | Setup | Input | Expected |
| --- | --- | --- | --- |
| `hrd_reveal_no_hits` | Dormant, gate open; Carl alone walks right from x 400 | jab her during the reveal | the gate closes and `Engaged` fires 8 ticks after Carl reaches 432; no `Hurt` and no tell for 90 ticks; then Neutral |
| `hrd_gate_2p_waits` | 2P, gate open; Carl walks past 432, P2 Donut stays at x 150 | none | gate open and she stays Dormant while Donut is outside; when Donut walks to x 236 the gate closes 1 tick later at most and `Engaged` fires |
| `hrd_gate_solo_regroup` | solo Carl with the plain bot held at x 150 by fixture setup | Carl walks past 432 | after 120 ticks the bot is placed at x 248, the gate closes, `Engaged` |
| `hrd_gate_no_band` | gate closed; bank A removed by fixture setup; P1 Donut at x 236 | none | she reaches the leash edge 290 and her swipe hits Donut (hurtbox 226..246 inside `[209, 248)`); no `Disengage` |
| `hrd_retry_short_reveal` | as above, after a retry | none | reveal 30 ticks |
| `hrd_restart_mid_reveal` | restart on reveal tick 45 | none | fresh state; the retry reveal is 30 ticks; state hash equal to a fresh retry |
| `hrd_swipe_hits_idle_carl` | Carl at d 60 | none | tell 0..33, `HeroHurt` 2 damage on attack tick 37, knock +160 / 200 away; Recover 43..90 |
| `hrd_swipe_jump_edges` | Carl at d 60, four runs | jump on attack tick 0 / 1 / 30 / 31 | hit / clear / clear / hit |
| `hrd_swipe_punish` | Carl at d 44 | jump on 20; jab on landing + 1 | jab lands inside Recover; no flinch (free damage) |
| `hrd_armour_tell` | Carl at d 44 | jab on attack tick 5 | `Hurt` 1, swipe continues |
| `hrd_poise_flinch` | **P1 Donut alone** (biscuit) at d 150; her shove cooldown preset to 180 by fixture setup | Donut casts four Magic Missiles at the refire cadence **from tick 0** (all land while she walks in, before she reaches the 88 px swipe trigger at about tick 83) | `Flinch` on the 4th; a 5th within 60 ticks adds no poise |
| `hrd_shove_hits_idle_carl` | Carl at d 128 | none | `HeroHurt` 1 on lunge tick 23 (attack tick 62), knock 300 / 180 in the shove direction; lunge continues to 144 px; Recover 48 |
| `hrd_shove_jump_edges` | Carl at d 128, four runs | jump on attack tick 37 / 38 / 55 / 56 | hit / clear / clear / hit |
| `hrd_shove_jump_lands_behind` | Carl at d 128 | jump on attack tick 45 | no `HeroHurt`; after the lunge Carl is behind her (+x side); `Turn` before her next tell |
| `hrd_shove_2p_both` | 2P: Carl at d 112, Donut at d 160 | none | both hit once each, one shove |
| `hrd_shove_jams_on_solid` | bank A as the sidecar static solid; she at 470, Carl at 330 (behind the bank, d 140) | none | the box front would pass the face 416 on lunge tick 3: clamp at pivot 464, `Jammed` 72; no motion backward |
| `hrd_turn_flank` | Carl behind her at 40 px | none | `Turn` 20 ticks, then a swipe tell facing him |
| `hrd_leash_return` | gate disabled and bank A removed by fixture setup (the settled case); Carl retreats to x 100 | none | she stops at the leash edge 290; after 90 unreachable ticks `Disengage`, she walks to 608 and holds; HP unchanged |
| `hrd_no_safe_snipe` | gate disabled and bank A removed by fixture setup; P1 Donut at x 150 | Donut fires Magic Missiles whenever she can | she holds at 290 for 90 ticks, then walks home (missile hits deal damage and poise, never cancel the walk) and holds at 608; from x 150 Donut's missiles no longer reach her |
| `hrd_disengage_follow_punished` | gate disabled and bank A removed by fixture setup; Carl at x 150 | after `Disengage`, Carl follows to 44 px behind her and jabs | she starts `Turn` within 1 tick of Carl entering 88 px; a swipe tell aimed at Carl starts within 21 ticks; Carl lands at most 2 jabs before the tell |
| `hrd_leash_kite_probe` | gate disabled and bank A removed by fixture setup; scripted solo Donut draws her (to x 288 or more), retreats to x 150 and fires whenever she is in range | pilot, 3 cycles | **reporting only**, no pass threshold: damage per cycle and ticks per cycle, for the HRD-D kite measurement |
| `hrd_ai_steps_out_of_swipe` | P1 Carl idle at d 140; AI Donut at d 50, her target (50 + the 40 px M2 bias is still nearer than 140); **two runs: the plain bot and `--companion cf02_free`** (the companion run waits for CA2-B2, as plan companion fixtures do) | none | the follower steps out on the Pound anchor; no `HeroHurt` on Donut from the swipe |
| `hrd_ai_steps_out_untargeted` | P1 Carl at d 44, her target; AI Donut at d 84; the same two runs | none | the Pound carries `target: None`; the untargeted bot steps out; no `HeroHurt` on Donut |
| `hrd_not_evadable` | Donut, biscuit, reflex armed | stand at d 60 | swipe `HeroHurt`, no `HeroEvaded`. **Runs once DM-B has merged**; until then the HRD-A1 unit test `hrd_evade_class_never` stands in |
| `hrd_defeat_grants_dynamite` | trigger `pile_boss_f01_defeated` armed by id (as CP-A's fixtures do); her HP 1 | jab | one `EnemyDefeated { target: PILE_BOSS_ID, instigator }`, one `CapabilityGranted { special_thrown_charge }` on the same tick; door open |
| `hrd_direct_clear_solo` | scripted solo Carl pilot, seeds 0..8 | pilot | defeat and exit in every seed; bank A `stable` and J-B `supported` at the end; no hero stuck more than 300 ticks; clear ticks reported |
| `hrd_direct_clear_donut` | scripted solo Donut pilot by swap, seeds 0..8 | pilot | as above |
| `hrd_direct_clear_companion` | Carl pilot with the companion under `--companion cf02_free`, seeds 0..8; **waits for CA2-B2** (plan), may be re-pinned by CA2-C and DP-E | pilot | defeat and exit; no hero stuck more than 300 ticks; AI hit counts of section 8 reported |
| `hrd_room_validator` | room and sidecar data | load | reachability for Carl, base Donut, boosted Donut, 2P in the five pile orderings; no surface 49..128; ceiling 228; protected rects clear of hazards, footprints and **her attack boxes at both leash edges** (the exit pocket counts as sealed while the door is closed); the entry gate opens on a reachable event |

**HRD-B (J-A):** `hrd_ja_bait_slide` (Carl at x 330 behind the bank, she at 470: the shove jams on the face on lunge tick 3 (jam tick J), disturbed on J credited to Carl, slid **J+24..J+39**, settled from J+40; exactly one slide; Carl stands still and the mass reaches him: **one slide push, 0 damage, knock 180 / 160 toward -x**, HP unchanged); `hrd_ja_slide_hits_once` (she at 470, Carl at x 300: one push only across all 16 slide ticks); `hrd_ja_nest_push` (**Carl on the bank top at x 410**: she walks to the face 444, `NestPush` after 60 ticks there, immediate jam, slide on J+24; Carl falls into the moving mass and takes one slide push, 0 damage); `hrd_ja_nest_push_donut` (P1 Donut at x 425 on the top: same result); `hrd_ja_remnant_lift` (a hero overlapping `[296, 376)` on the settle tick is lifted onto the 16 px top, not snapped sideways); `hrd_ja_melee_no_effect` (ten jabs and three missiles: stays `stable`, missiles stopped); `hrd_ja_settled_walkable` (both heroes and she cross the remnant); `hrd_ja_bot_target_credit` (her shove aimed at AI Donut jams on bank A: disturbed, credit `None`); `hrd_ja_restart_mid_slide` (retry: fresh state, hash equal to a fresh attempt).

**HRD-C (J-B):** `hrd_jb_hits_before_engage` (two hits in Dormant: `supported`); `hrd_jb_same_instance_once` (one move instance overlapping twice: `weakened` only); `hrd_jb_two_hits_release` (released on R, falling R+24..R+31, **landing evaluation on R+31**, once; `landed` from R+32); `hrd_jb_missile_counts`; `hrd_jb_bot_ignored`; `hrd_jb_bait_left` and `hrd_jb_bait_right` (shove into each face: pivot 760 or 872, landing overlaps her: 6 damage, Exposed 150, prone body, finisher hits are free damage); `hrd_jb_nonlethal` (HP 4 → 1, Exposed, not defeated); `hrd_jb_misses_her` (she outside: no effect on her); `hrd_jb_hero_in_footprint` (1 damage, credited, no `HeroEvaded`); `hrd_jb_remnant_lift` (an invulnerable hero inside the footprint on the landing tick is lifted onto the remnant top); `hrd_orders` (**A only, B only, A then B, B then A, neither**: exit reachable after her defeat, validator passes); `hrd_restart_mid_fall`; `hrd_collapse_clear` (pilot using the bait).

**HRD-D:** the evidence list and the measurements of section 11, three human runs with honest notes, PLAYTRACE strict verification of each recorded session, frame time, the AI counts of section 8.

**Goldens that must stay unchanged:** every golden listed in the CP, DP and DM work orders, every `cf02_*`, Engineer, boss, companion, party, CINE, death presenter, PLAYTRACE and AUTOPLAY golden, and `species_death_profiles.json` (the species comes through the overlay, as the Engineer's did). The CP-A story goldens change only by HRD-A2's documented re-pin.

## 15. Art follow-up HOARD-02 (parallel, not blocking, K10)

Through the [pixel-art pipeline](../07-agents/pixel-art-pipeline.md#routing-by-difficulty), on the HOARD-01 rig: **shove tell** (a low brace with one shoulder forward and both arms spread low; it must differ from the rear-up and from the gather at 1x), **lunge** (2-frame shoulder heave, dragged foot), **jammed** (impact on an obstacle, junk spilling), **pinned** (alive under junk, eyes open; must differ from defeat frame 15), **get-up** (2), **emerge** (2 to 3, rising from the mound). Props: bank A (stable, shudder, sliding, remnant), bank B (stack, post intact, cracked, broken, falling, remnant), the lair mound. No junk on her back (HOARD-01 guardrail). Until then every stand-in is labelled STAND-IN in captures.

## 16. Horizon fit, tone and rights

- **Horizon:** consulted the [character progression horizon](character-progression-horizon.md). Still consistent, no guide change here: she adds no player verb, unlock or button; she makes the base kit's jump, the punish, ranged pressure and terrain use the right answers. Reference path item 3 ("Early encounters / Hoarder: deepen hit reactions and combat-under-pressure; dynamite at an appropriate point") holds; K1 moves the dynamite grant to her defeat, and PT-DOC records that change-log row.
- **Tone:** a transformed human woman, frightening and tragic, never comedy; no gore on her death; hurt and pinned frames read as pain, not as a joke. No invented biography or book quote.
- **Rights:** no new art, audio or text; HOARD-01 sources and provenance unchanged. Gameplay values are original design hypotheses. Design and Revision 1 by Claude Opus (top tier); independent review round 1 and a narrow re-review (round 2) by a separate top-tier agent (both FAIL, resolved in sections 17 and 18; a narrow top-tier look at the section 7 disengage rule, HRD-Q19 and `hrd_disengage_follow_punished` is next); Revision 2 also by Claude Opus; execution not done yet.

## 17. Revision 1 resolutions

Independent top-tier review round 1 returned FAIL (four logic blockers, fix-now items F1..F9). Every item is applied as the review recommends, with the orchestrator's rulings for F6 and B4. The integration plan reference is now Revision 2 (#182).

| Item | Resolution | Where |
| --- | --- | --- |
| B1 body during the lunge | Her body is not solid and pushes nobody during the Lunge (the Juicer's `Charge` rule); push-out resumes on the first tick after it with the nearer-edge and 28 px side-room rule. Body box per mode defined (standing, none, prone). A hero who jumps the shove from 96 or 128 px lands behind her: Carl's flank. New `hrd_shove_jump_lands_behind` | 4.0, 4.2, HRD-Q03, 14 |
| B2 nest walk goal | With the walk target on bank A's top or at x < 416 while the bank is stable or disturbed, her walk goal is the face (pivot 444) with no standoff; the nest timer counts there. `hrd_ja_nest_push` uses Carl at x 410, plus a Donut variant at x 425 | 5, 7, HRD-Q24, 14 |
| B3 leash edge | Leash minimum 290 (swipe box ends at 209, outside the entry pocket). Disengage after 90 unreachable ticks at a leash edge, walk home, hits do not cancel it; home hold until a hero is within 320 px. The validator checks her attack boxes at both leash edges. New `hrd_no_safe_snipe` | 3, 7, HRD-Q19, 9, 14 |
| B4 AI swipe dodge | `Pound.x` is a dodge anchor 9 px ahead of her pivot, so the follower's 84 px step-out clears her 92 px reach with no AI code change. Feed path: ENEMY-SLOT's `CompanionView` row in `src/combat_scene/companion.rs`. Not in `CompanionView.targets` in v1; AI missiles against her are a COMPANION follow-up. New `hrd_ai_steps_out_of_swipe` | 8, HRD-Q33, 9, 14 |
| F1 plan alignment | HRD-A1 (pure core, data, room and sidecar, unit tests) and HRD-A2 (scene module behind ENEMY-SLOT's hook, door, trigger, scene arm, fixtures). The HRD fallback hook is deleted. ENEMY-SLOT owns her tick slot, hook, `CompanionView` row and env access; BOIL-B wire owns only the env step | 9, 14 |
| F2 events | `EnemyDefeated { tick, target, cause, force, instigator }`; `PileBoss` carries the non-defeat events; HRD-A2 joins the `combat.rs` queue for it with its hash-mix and journal arms; the plan is asked to word its row "HRD-A2: one `PileBoss` variant" | 10, HRD-Q31, HRD-Q40, 14 |
| F3 K1 trigger code | HRD-A2 owns a generic `EventMatch::EnemyDefeated` with a `target` field, a `SceneKind` for the room and the re-pin of `story_unlocks_v0.json`; the room pin stays PT-SCENE's; the grant fixture arms the trigger by id | 10, 14 |
| F4 J-A slide owner | `Slide { dx_per_tick, ticks, remnant }` in `env_prop.rs` by BOIL-C wire (plan Revision 2), fallback HRD-B; the moving mass is an `env_hazard.rs` volume; bank A is a sidecar static solid in HRD-A1 that HRD-B upgrades in place | 9, 14 |
| F5 remnants under actors | An overlapped hero is lifted onto a new remnant's top; she ignores remnants of 16 px or less; rule placed on the BOIL-C row | 4.0, 5, 6, 9 |
| F6 slide damage | The J-A slide push does 0 damage, knock 180 / 160 (per #178); no new deviation | 5, HRD-Q23, 14 |
| F7 fixtures | `hrd_poise_flinch` (P1 Donut alone at d 150, shove cooldown preset); `hrd_shove_out_of_reach` as a forced-shove unit test (Carl 203/204, Donut 201/202); `hrd_ja_bait_slide` expects a 0-damage push; `hrd_ja_nest_push` gives x; `hrd_not_evadable` runs once DM-B has merged, with a class unit test until then; direct-clear pilots for solo Carl, solo Donut by swap and the companion at seeds 0..8 with no hero stuck more than 300 ticks; new `hrd_restart_mid_reveal` | 14 |
| F8 clarifications | The lair mound has no collision; her shove at a bot hero still resolves a pile with credit `None`; five pile orderings; the "effects muted" capture | 2, 3, 6, 8, 11, 14 |
| F9 off-by-one | Carl lands on press + 49, at most 79 (12 Recover ticks); Donut out of shove reach at 202; Donut's step-back by attack tick 24; counting convention (entry tick = tick 1): J-A slides on J+24..J+39, J-B lands on R+31 | 4, 4.1, 4.2, 5, 6, HRD-Q08, HRD-Q21, HRD-Q27, 14 |
| Fun notes (non-blocking) | HP 30 against the 45..120 s target, ranged kiting, incidental J-A slides and the passive AI partner are HRD-D measurements | 11 |

## 18. Revision 2 resolutions

The narrow re-review (round 2) confirmed B1, B2, B4 and F1..F9, and returned FAIL on one new logic blocker (N1) with fix-now items N2..N8. Its plan wording items P1..P3 are routed to the plan by the orchestrator and are not edited here.

| Item | Resolution | Where |
| --- | --- | --- |
| N1 free back-hit on the disengage walk | The reviewer's paragraph is adopted verbatim: during the walk she answers a hero who follows her; the walk ends and her ordinary choices resume (Turn included) when an eligible hero is inside her swipe trigger on either side, or inside her shove trigger on either side and not beyond the leash edge she left; hits alone do not end it. Mirrored in HRD-Q19. New `hrd_disengage_follow_punished` | 7, HRD-Q19, 14 |
| N2 leash kite loop | The false claims ("from there she can reach that hero", "No disengage loop") are deleted. Adopted default: an **entry gate** at x 208..224 that closes on the engage with the Juicer's rule (every living hero inside, 8 ticks, 2P waits, solo regroup at 120 ticks) and opens on defeat, a retry or a restart. #178 does not mention a gate and does not forbid one, so it is flagged as **DV-6**. The loop, the gate on/off comparison and the levers (b) shorter unreachable ticks and (c) a faster walk are HRD-D measurements, with a reporting-only `hrd_leash_kite_probe`. New HRD-Q41 and gate fixtures | 2, 3, 7, 9, 11, HRD-Q05, HRD-Q41, 14 |
| N3 push-out room | Free room is measured in the hero's own height band from the feet up; a solid whose top is at or below the hero's feet does not count. Unit test: Carl at 427 on the top ends at x 404 or less, still on the top | 4.0, HRD-Q03, 14 |
| N4 nest timer | Counts in every mode; keyed on hero x only (hurtbox left edge < 416, any height); never reset by a jump, a hit, a Flinch or a target switch; resets only when no hero qualifies or she leaves the face | 5, 7, HRD-Q24, 14 |
| N5 Pound target | `target: None`, so an untargeted bot steps out too. New `hrd_ai_steps_out_untargeted` | 8, HRD-Q33, 14 |
| N6 two feed sites | `PartyStepCtx.boss` in `src/combat_scene.rs` (plain bot) and the follow layer plus `CompanionView.boss` in `src/combat_scene/companion.rs` (companion path), both ENEMY-SLOT's supply. The AI step-out fixtures run in both modes | 8, 9, HRD-Q33, 14 |
| N7 shove whiff at the leash clamp | No shove when the target's hurtbox lies wholly beyond the box front at the edge clamp (x 242 facing -x, x 928 facing +x); she walks instead | 7, 14 |
| N8 small items | `hrd_poise_flinch` fires from tick 0 at the refire cadence; the stale plan request in section 14 is deleted; the HRD-Q19 rationale now reads "removes the static snipe; see the HRD-D kite measurement" | 12, 14 |
