# ENEMY-GAMEPLAY-HOARDER: the Hoarder as a boss (HRD-0 design sheet)

> **Status: design adopted under the owner's design-defaults directive (AGENTS.md, 2026-11-19); not implemented, not game-feel accepted.**
> Every choice below has one adopted value with an `HRD-Q##` ID ([section 12](#12-adopted-defaults-hrd-q01hrd-q40)). Each value is a tuning hypothesis stored as data. The owner reviews game feel after the implementation merges and may change any value in a follow-up PR. Only [section 13](#13-genuinely-open-questions) waits for the owner.
> Design: Claude Opus (top tier). Execution: medium tier, steps HRD-A to HRD-D ([section 14](#14-implementation-brief-hrd-ahrd-d)). Review: a separate top-tier agent.

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
| DV-3 | "Hoarder silhouette emerges from one pile" | She emerges from a third, **non-interactive lair mound** at the arena centre, not from bank A or B | Keeps both prop state machines free of a spawn rule; the mound is a static prop with no gameplay state |
| DV-4 | Bank B at x 736..864 | Footprint x 752..880, support post x 808..824 | Sixteen px right so a baited shove into the post leaves her under the footprint from either side ([section 6](#6-pile-b-j-b-the-hanging-stack)) |
| DV-5 | (unstated) Magic Missile and the support | A Magic Missile is a qualifying hit on the J-B support | "Existing hit rules" include the missile; it gives Donut a real role in the collapse route. Needs one stimulus variant ([section 9](#9-interfaces-to-the-shared-cores)) |

No other #178 rule is changed. The dynamite placement follows #178 through K1.

## 3. Tactical role and the room

**She is a slow, heavy floor boss who owns the space in front of her. Close up she rears and slams both hands down (the swipe); at middle range she crouches and heaves her whole body forward along the floor (the shove). Both are long, readable and committed, and both leave her open. The shove is the attack the player baits: into bank A to rearrange cover, or into the support of bank B to bring the stack down on her.**

| Range | What she does | Main answers |
| --- | --- | --- |
| Close (pivot distance 88 px or less, in front) | Swipe: 34-tick tell, slam box 42..81 px ahead, 0..44 px high | Jump in place, step back past 93 px, or be behind her; punish the 48-tick recovery |
| Middle (96..176 px, in front) | Shove: 40-tick tell, then a lunge of up to 144 px at 180 px/s, box 0..48 px high | Jump the lunge, put bank A or the support post between you and her, punish the whiff (48) or the jam (72) |
| Far, or a hero she cannot reach | Shuffle at 45 px/s; push bank A down under a percher ([section 5](#5-pile-a-j-a-the-shuddering-bank)) | Ranged pressure, reposition, bait |

### Room greybox `f01_hoarder` (HRD-Q29, data in the room file and its sidecar)

All x in world px from the room's left edge, heights above the walking line, 16 px grid.

| Part | x | Content |
| --- | --- | --- |
| Entry pocket (protected) | 0..208 | Checkpoint at x 64; no hazard, footprint or leash reaches it |
| Approach | 208..352 | Floor; the lair mound is visible from x 240 (stir cue, presentation only) |
| Bank A (J-A) | 352..416 | Solid 64 x 48; its top is the only raised standable surface (the H0 vantage) |
| Arena | 416..880 | Floor; lair mound and her home at x 608, facing -1; her leash 256..880 (pivot) |
| Bank B (J-B) | 752..880 | Hanging stack 96..160 px up (no collision); support post 808..824, 0..64 px (non-solid hit target) |
| Exit gap | 880..928 | Floor, never under a footprint |
| Door | 928..944 | Full-height solid until her defeat |
| Exit pocket (protected) | 944..1056 | Room goal zone |

Room width 1056 (#178 range 960..1152). Lane ceiling **228 px** declared through `ceiling_px` (DP-Q45: bank A's top is above 42 px). Engage line: a hero's x reaches **432** (the arena floor past bank A).

## 4. Moves and frame data (60 Hz, all data knobs)

Attack tick 0 = the tell's first tick. Boxes are pivot-relative: x forward (mirrored by the facing), y up from the walking line, half-open `[a, b)`. Facing and target lock at the tell start. The hero step runs before her step in a tick (the Juicer order), so a hero's same-tick hit lands first. Hand-computed tick edges below use the real hero integrator (`step_hero_driven`: launch on the press tick, then gravity, then position); the executor's unit test reproduces them, and any edge that differs by more than 1 tick is a design defect to report, not to re-decide.

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

**Reach (standing hero):** hurtbox overlap with `[42, 81)` means Carl (half-width 12) is hit at pivot distance 31..92 and Donut (10) at 33..90. Her body pushes heroes out to 40 px or more (Carl), so **every hero in front of her within 92 px is in the slam**, including one close enough to jab her (40..48).

| Answer | Carl | Donut (base / boosted) |
| --- | --- | --- |
| Jump in place (box top 44; feet at 44 px or more on jump ticks 8..42 for Carl, 7..50 base, 6..56 boosted) | Press on attack ticks **1..30**; lands on attack tick press + 48, at most 78: 12 ticks left of the recover | Press on attack ticks **0..31** / **0..32** |
| Step back from pivot distance 44 | Running away must start by attack tick about **22** (49 px to 93 at 3.08 px per tick) | About 23 (207 px/s) |
| Be behind her | Safe; she turns in 20 ticks before her next tell | Same |
| Hit her during the tell | Damage lands (armour), then still jump or leave | Same; a swat adds poise only |

### 4.2 Shove (the committed displacement attack, needs HOARD-02)

| Phase | Attack ticks | Stand-in frames (labelled STAND-IN until HOARD-02) | Boxes and motion | Hits on her |
| --- | --- | --- | --- | --- |
| Tell: crouched brace | 0..39 | 7 held 40 ticks, drawn 4 px back (wind-up) | hurtbox; no motion | Armour |
| Lunge | 40..87 at most (lunge tick k = 1..48 is attack tick 39 + k) | 9 held, translated | **3 px per tick forward (180 px/s), 144 px at most**; **shove box `[20, 48) x [0, 48)`**; step first, then test | Armour |
| Recover (whiff, max distance or leash edge) | 48 ticks | 11 held | stops | **Free damage** |
| Jammed (pile A face, J-B post, any solid) | 72 ticks | 12 for 12 ticks, then 11 | stops at the clamp | **Free damage** |
| Cooldown | 30, then the shove waits for its own 180-tick cooldown | idle or walk | | Poise counts |

**One lunge tick, in this order:** (1) **step** 3 px forward; if the box front would pass the face of bank A while it is `stable` or `disturbed`, the near face of the J-B post while it is `supported` or `weakened`, or any intact solid (the door, a room wall), clamp her so the box front rests on that face (never moving her backward) and mark `met_obstacle`; the leash edge clamps her as a whiff; (2) **heroes:** every hero whose hurtbox overlaps the box and who was not hit by this shove takes **1 damage** and a knock of **300 px/s in the shove direction, 180 px/s up** (the displacement); the shove **continues**; (3) **obstacle:** `met_obstacle` sends the prop stimulus of [section 9](#9-interfaces-to-the-shared-cores) to that prop (none for a plain solid), logs `PileBoss(Jammed)` and enters Jammed; (4) **distance:** after lunge tick 48 she enters Recover.

**Reach (hero standing still, no obstacle):** the box front reaches `48 + 3k`, the back `20 + 3k`.

| Start distance | Carl hit on lunge tick (attack tick) | Carl jump-in-place clears if pressed on attack ticks | Donut base clears | Donut boosted clears |
| --- | --- | --- | --- | --- |
| 96 (trigger minimum) | 13 (52) | 28..45 | 19..46 | 13..47 |
| 128 | 23 (62) | **38..55** | 30..57 | 24..58 |
| 160 | 34 (73) | 47..66 | 39..68 | 33..69 |
| 176 (trigger maximum) | 39 (78) | 47..71 | 39..73 | 33..74 |
| 204 or more (203 for Donut) | never | any | any | any |

Carl's window is about 18 ticks and starts around the lunge start: jumping *as she lunges* is the read. Running away only escapes when started early in the tell (she lunges at 180 px/s, Carl runs at 185). Bank A and the J-B post are the safe answers: a hero behind either is never reached.

### 4.3 Hit reactions, armour and poise (the Juicer rules)

- **Armour:** every tell, the swipe smear and impact, and the lunge: hits deal damage and hit-stop, never interrupt.
- **Free damage, no poise:** Recover, Jammed, Flinch, Exposed and Get-up.
- **Poise:** hits in Neutral, Walk, Turn and Cooldown add their damage (a swat adds 1 with 0 damage). **4 within 120 ticks** → **Flinch 12 ticks** (frame 12), then Cooldown 30. For **60 ticks** after a flinch no poise accumulates (no flinch lock from missiles).
- **Never:** launched, knocked down, wall-slammed, bowled, stomped into `Downed` or pierced. The push kick deals its damage without a launch, as on the Juicer.
- **Damage taken:** every merged hero hit at its merged damage (Carl's moves 1, Magic Missile 1, pounce its effect damage, swat 0). Under `--f01-loadout all` only: dynamite 4 (CP-Q25 rule), the gauntlet doubles poise and does not pierce (CP-Q40 rule).

### 4.4 Exposure (the J-B payoff)

On the J-B landing tick, if the footprint overlaps her body box, whatever her mode: **6 damage, never lethal** (her HP floors at 1), then **Exposed 150 ticks**, then **Get-up 24**, then Cooldown 30. Exposed and Get-up have no attack boxes. Her hurtbox while Exposed is the prone box `[-48, 72) x [0, 48)` (HOARD-01 frame 15's bbox, rounded), which every merged hero hit can reach. Stand-in: frame 11 held under the landed remnant (alive, eyes open); never frames 13..15, which mean defeat (the Engineer U1 lesson). The finishing blow is always a hero's.

## 5. Pile A (J-A): the shuddering bank

States (#178): `stable` → `disturbed` → `slid` → `settled`, once per attempt.

| State | Collision | Duration | Notes |
| --- | --- | --- | --- |
| `stable` | Solid `[352, 416) x [0, 48)`, standable top | until a stimulus | Real cover: blocks her walk and shove, Magic Missiles and any projectile |
| `disturbed` | same solid | **24 ticks** (#178 range 18..30) | Shudder, dust, a floor scuff along the slide path (presentation) |
| `slid` | the solid is removed on the first tick; a moving mass (hazard, not a solid) translates **-x 3 px per tick for 16 ticks**, from `[352, 416)` to `[304, 368)`, 0..48 px | 16 ticks | A hero standing on top falls |
| `settled` | Solid remnant `[296, 376) x [0, 16)` | rest of the attempt | Traversable by both heroes and by her (she steps over 16 px or less) |

- **Stimuli that disturb it:** her shove meeting its face (the bait), a launched body (BOIL-D) and a blast (CP-C hook, test loadout only). **Hero melee and missiles never disturb it**, so cover cannot be knocked over by accident.
- **Slide hazard:** on each slide tick, a hero whose hurtbox overlaps the moving mass takes one blow per motion token: **1 damage**, knock **180 px/s toward -x, 160 up**, never evadable, every hero (K3). She is never on the -x side of a stable bank, so it never touches her.
- **Attribution:** the disturbance is credited to the player of the hero she was targeting (the bait), like the Engineer's bonk.
- **Nest rule (the ledge, K8):** when bank A is `stable`, she stands at its face (pivot within 4 px of x 444) and her target is on its top or behind it (x below 416) for **60 consecutive ticks**, she shoves bank A. The shove jams at once (no motion) and disturbs it. A target behind the bank at 96..176 px triggers the ordinary shove sooner, which also jams on the face. So bank A's top is a vantage, never a firing nest: after one slide no raised surface above 16 px is left in the room.

## 6. Pile B (J-B): the hanging stack

States (#178): `supported` → `weakened` → `released` → `falling` → `landed`, once per attempt.

| State | Duration | Notes |
| --- | --- | --- |
| `supported` | until a qualifying hit or a heavy impact | Post `[808, 824) x [0, 64)` (non-solid hit target); stack `[752, 880) x [96, 160)` with no collision |
| `weakened` | until a second qualifying hit or a heavy impact | Cracked post (presentation) |
| `released` | **24 ticks** warning | Shudder, falling dust, the floor shadow of the footprint |
| `falling` | **8 ticks** | Landing on the 8th tick: **release tick + 32** |
| `landed` | rest of the attempt | Solid remnant `[752, 880) x [0, 16)`, traversable |

- **Qualifying hit:** a distinct move instance of a **human-controlled** hero whose active box overlaps the post (Carl's melee moves, Donut's swat and pounce, a Magic Missile), weight 1 each (the per-move support-hit weight of CP-D; a gauntlet haymaker's 2 only applies under the test loadout). **Two** → released. Hits before the engage line is crossed, and hits from a bot-driven hero, do nothing (the Juicer rule "the AI never triggers a rack").
- **Heavy impact:** her shove meeting the post → released at once. A blast with `breaks_structures` (test loadout) → released.
- **Landing (one swept evaluation on the landing tick, footprint `[752, 880) x [0, 48)`):** each hero overlapped takes **1 damage**, knock **120 px/s** away from the footprint centre and **140 up**, never evadable, every hero. Her body overlapped → [Exposure](#44-exposure-the-j-b-payoff). One outcome per overlap, no per-tick aura.
- **Why the bait works from both sides:** a shove clamped on the post's left face leaves her pivot at 760 (body 732..788, 36 px under the footprint); on its right face at 872 (body 844..900, 36 px under). Jammed lasts 72 ticks and the landing comes 32 ticks after the release, so a baited shove always ends with her under the stack. A hero standing beyond the post is never reached by that shove (the box stops on the post), but stands in the footprint and has 32 ticks to leave (from the footprint's centre Carl needs 25 ticks to clear its edge by his half-width).
- **Attribution:** the release is credited to the hitter of the second hit, or to the player of the hero she targeted for a baited shove.

## 7. Brain: states, targeting and choices

**States:** `Dormant` → `Reveal` → `Neutral` (with `Walk`, `Turn`, `Cooldown`) ⇄ swipe (`SwipeTell`, `SwipeStrike`, `SwipeRecover`) / shove (`ShoveTell`, `Lunge`, `ShoveRecover`, `Jammed`) / `Flinch` / `Exposed` → `GetUp`; `Defeated`.

- **Dormant:** no hurtbox, no body, no attack. **Reveal** starts when a hero reaches the engage line: **90 ticks** on the first entry, **30** after a retry or checkpoint restart; no hurtbox and no attack during it; she becomes solid on its last tick (heroes inside her body are pushed out).
- **Eligible hero:** present, and feet at most **44 px** above the walking line (her blows' height). A hero on bank A's top (48) or in the air is not a swipe or shove target.
- **Target:** the Juicer rule over eligible heroes (nearest by pivot distance, **24 px** hysteresis, solo bias **+40 px** on the bot hero). With no eligible hero, the same rule over all present heroes picks whom she walks toward. Locked at the tell start.
- **Choice each Neutral tick (after Cooldown), in this order:** (1) the nest rule; (2) target behind her by more than 8 px → **Turn** (20 ticks, walk frames); (3) eligible target in front at **88 px or less** → swipe; (4) eligible target in front at **96..176 px** and the shove cooldown done → shove; (5) walk toward the target at **45 px/s** and stop at **64 px**.
- **Leash:** her pivot stays in **256..880**; solids still block her (bank A while stable). If no hero is within 320 px of her for **240 ticks** she walks back to x 608. No HP regeneration, ever; poise clears.
- **Offscreen:** every tell starts within 176 px of its target, inside any 640 px view that frames that hero. No camera state is read.

## 8. Solo, 2P and the AI partner

- **Parity:** solo Carl, solo Donut and 2P use the same room, the same rules and the same goal (her defeat, then the exit). Each hero can trigger J-B and bait J-A when human-controlled.
- **Solo, Carl controlled, AI Donut:** M1 (the AI hero never drops below 1 HP) and M2 (the +40 px bias) apply. She feeds the existing follower boss input (`corridor.rs` `boss: Option<BossThreat>`) with `BossThreat::Pound { x, facing, target }` during the swipe tell and `BossThreat::Charge { tell_left, x, dir, running, front: 48, depth: 28, speed: 3, lead_ticks: 18 }` during the shove tell and lunge. No AI code changes; the Juicer's M3 dodges read them as they are. The J-A slide and the J-B footprint reach the AI through CA2-C's hazard feed (active and telegraphed rectangles), or are a reported gap until it lands (K13).
- **Measured, not required (as the Engineer's F12):** AI Donut swipe hits, shove hits and footprint hits per clear. More than **1 per clear** of any kind opens a COMPANION follow-up item.
- **Solo, Donut controlled, AI Carl:** the same rules; AI Carl's existing assist and follower; it never throws dynamite (CP-Q29), never triggers a pile.
- **2P:** no guards (M5). Each hero is hit at most once per swipe, per shove, per slide and per landing. No friendly fire. The swipe and the shove hit every overlapped hero, so a partner in the lane shares the blow.
- **Donut's roles:** missiles from range (poise in neutral, damage in punish windows), the swat for poise, a missile as the second J-B hit from a safe angle. With Light on Your Feet she can jump over her (feet above her 110 px head box on jump ticks 17..44, enough to cross her 56 px body at 207 px/s; the base jump cannot). That is intended asymmetric mobility, not a nest.
- **Reflex evasion (K6, DM-Q18):** the swipe, the shove, the J-A slide and the J-B landing are **never evadable**. Every blow carries the non-evadable class; nothing here opts in.

## 9. Interfaces to the shared cores

| Need | Owner (integration plan section 3) | Contract used here |
| --- | --- | --- |
| Prop state machines, stimulus intake, swept landing, remnant, reset | BOIL-B (stimulus enum), BOIL-C (falling mass) in `src/env_prop.rs` | J-A is a slide variant of the prop core (warning, bounded motion, remnant); J-B is the falling-mass variant (support, warning, fall, one footprint, remnant) |
| **Her shove as a stimulus** | Interface request to **BOIL-B core**: one appended variant `Stimulus::EnemyImpact { source, credit: Option<u32> }` (heavy). If BOIL-B ships without it, HRD-B appends it (append only) | Sent by her step on the jam tick; consumed in the env step of the same tick |
| **Magic Missile on a support** (DV-5) | Interface request to **BOIL-B core**: `Stimulus::Projectile { owner, weight }`. If absent, HRD-C appends it | Weight 1; HRD-Q26 can switch it off |
| Hazard volumes (slide mass, footprint) | BOIL-B `src/env_hazard.rs` | Activation token per victim, team policy all heroes, class never evadable, hero hurt through `HeroBlow` |
| Launched body into a prop | BOIL-D | J-A accepts it (no launched bodies exist in this room in the story loadout) |
| Blast on a prop | CP-C `BlastTarget` hook | Test loadout only (K1) |
| Encounter data, gate validation, reachability | BOIL-A `src/encounter.rs` and the `*.encounter.json` sidecar | Props, door, protected rects, story pin, goal, K8 classes |
| Story trigger | CP-A ledger | `pile_boss_f01_defeated` = `EnemyDefeated { target: PILE_BOSS_ID }` |
| Tick slot | BOIL-B wire (`combat_scene/env.rs`) | Order: … Engineer, **pile boss** (new enemies slot), contacts, shore, boss, charges, **env props and hazards**, morale |

Within one tick: hero strikes (on her hurtbox, and on the J-B post through the prop intake), her step (lunge clamp against the current prop colliders, which her context carries), then the env step (stimuli applied, slide and landing evaluated, landing on her calls her `crush`).

## 10. Defeat, death and the story event

- **Defeat:** HP 0 from a hero's hit (never from J-B). On that tick: one `EnemyDefeated { target: PILE_BOSS_ID, cause, player }` (the final blow's existing cause), the door opens, her body loses collision, no new hazard starts (a stack already falling still lands and still hurts heroes).
- **Story event (K1):** the CP-A trigger row `pile_boss_f01_defeated` matches that event and grants `special_thrown_charge` on the same tick (one `CapabilityGranted`). HRD-A swaps CP-A's placeholder zone for this event row.
- **Death presentation:** species `pile_boss`; every recipe, every cause (blast included), maps to HOARD-01 `down` 13..15 (buckle, floor, settled prone hold). No gibs, no dismemberment, no blood: a ruined person, not a target for spectacle. Not CINE-eligible. The corpse stays without collision; the piles keep their resolved states.
- **Aftermath (H5):** no further enemies; the grant notice follows the CP rules. A retry after defeat is not possible (the room is complete).

## 11. Acceptance criteria

Machine (CI): every fixture of [section 14](#fixtures-and-tests-per-step) passes and replays identically twice with a pinned golden; every existing golden named in the CP, DP and DM work orders stays byte-identical; `enemy_pile_boss_v0.json` loads, validates (an error names the field), matches its FNV-1a pin, and a completeness test proves every HRD-Q has a knob or a recorded no-knob reason; the reach tables of section 4 are reproduced by a domain unit test; the room passes the BOIL-A validator for Carl, base Donut, boosted Donut and 2P in all four pile orderings.

Evidence (HRD-D, real Bevy, 640 x 360 and 640 x 400, 1x): reveal; swipe → jump → punish; shove → jump → punish; shove baited into bank A and the slide; nest push; two hits on the post; a baited shove into the post, landing and exposure finisher; a direct clear with both piles intact; the defeat and the grant; a 2P run (or **NOT RUN**); frame time; the AI counts of section 8.

Human (owner; the item stays "merged, game-feel review pending"): the encounter reads as dread, not comedy; the owner can tell swipe from shove before it lands; at least two of the three wins happen in three runs; a direct clear is not slow or frustrating (target 45..120 s for a human direct clear, tuned by HP in HRD-D).

## 12. Adopted defaults (HRD-Q01..HRD-Q40)

Knob names start with the question ID (`hrdq02_hp`). File: `assets/data/gameplay/enemy_pile_boss_v0.json`; room geometry in the room file and its sidecar.

| ID | Question | Adopted value | Rationale | Knob |
| --- | --- | --- | --- | --- |
| HRD-Q01 | Vocabulary | Swipe, shove, punish windows; one phase; no extra move in v1 | #178: a very small vocabulary; one more action only after playtest | none (design) |
| HRD-Q02 | Health | **30**, single phase | Juicer 36 at ~85 % with phases; she comes at ~40 % with the base kit; difficulty from commitment and geometry, not HP | `hrdq02_hp` |
| HRD-Q03 | Body and hurtbox | Body and hurtbox `[-28, 28) x [0, 104)` plus head `[1, 21) x [88, 110)`; pushes heroes out at 6 px/tick (never into a solid); not standable; no contact damage | HOARD-01 section 3; Juicer body rule; no body-contact damage keeps the vocabulary small | `hrdq03_body`, `hrdq03_head`, `hrdq03_push_px` |
| HRD-Q04 | Walk, turn, standoff | **45** px/s; turn **20** ticks; walk stop **64** px | Art contact plan; a slow turn makes the flank a real opening | `hrdq04_walk_px_s`, `hrdq04_turn_ticks`, `hrdq04_standoff_px` |
| HRD-Q05 | Engage and reveal | Engage at hero x **432**; reveal **90** ticks, **30** on retry; no hurtbox or attack in Dormant and Reveal; stir cue at x **240** (presentation) | H0/H1: no auto-hit during the reveal; rapid retry skips staging | `hrdq05_engage_x`, `hrdq05_reveal_ticks`, `hrdq05_retry_reveal_ticks`, `hrdq05_stir_x` |
| HRD-Q06 | Targeting | Juicer rule over eligible heroes (feet ≤ **44** px), hysteresis **24**, solo bot bias **40**; locked at the tell | Reuses Q3 B; perched or airborne heroes are not swiped | `hrdq06_eligible_px`, `hrdq06_switch_px`, `hrdq06_ai_bias_px` |
| HRD-Q07 | Swipe trigger | Eligible target in front at **88** px or less | Art estimate 80 + the box reaches 92; no hero in the slam is ignored | `hrdq07_trigger_px` |
| HRD-Q08 | Swipe timing | Tell **34** (8 + 26), smear **3**, impact **6**, recover **48**, cooldown **30** | HOARD-01 ticks; recover 48 so every clearing jump (latest lands on attack tick 78) can punish before 91 | `hrdq08_*` |
| HRD-Q09 | Swipe hit | Box `[42, 81) x [0, 44)`; **2** damage; knock **160 / 200**; once per hero | HOARD-01 frame 10 box; Juicer pound damage; the most readable blow costs the most | `hrdq09_box`, `hrdq09_damage`, `hrdq09_knock` |
| HRD-Q10 | Shove trigger | Eligible target in front at **96..176** px; shove cooldown done | Middle range only, distinct from the swipe; the gap closer for a 45 px/s boss | `hrdq10_range_px` |
| HRD-Q11 | Shove timing | Tell **40**; lunge **3** px/tick (180 px/s) for at most **48** ticks (**144** px); step, then test | Never faster than Carl's 185; a readable lunge start; reach to 203 px | `hrdq11_tell_ticks`, `hrdq11_px_per_tick`, `hrdq11_max_ticks` |
| HRD-Q12 | Shove hit | Box `[20, 48) x [0, 48)`; **1** damage; knock **300 / 180** in the shove direction; once per hero; the lunge continues | The displacement is the threat; an 18-tick jump window for Carl | `hrdq12_box`, `hrdq12_damage`, `hrdq12_knock` |
| HRD-Q13 | Shove ends | Whiff or leash → Recover **48**; pile face, post or solid → Jammed **72**; clamp never moves her backward | A baited jam pays more than a dodge; 72 > 32 keeps her under J-B | `hrdq13_recover_ticks`, `hrdq13_jam_ticks` |
| HRD-Q14 | Shove cooldown | **180** ticks after the shove ends | At most one shove every ~4.5 s; she is not a lane spammer | `hrdq14_cooldown_ticks` |
| HRD-Q15 | Armour | Tells, smear, impact, lunge: damage and hit-stop, no interrupt; Recover, Jammed, Flinch, Exposed, Get-up: free damage | Juicer super armour; "interrupt only when a real vulnerability allows" | none (code) |
| HRD-Q16 | Poise | **4** damage in **120** ticks in Neutral, Walk, Turn, Cooldown → Flinch **12**; **60** ticks of no poise after it | Juicer values; no missile flinch lock | `hrdq16_poise`, `hrdq16_window`, `hrdq16_flinch_ticks`, `hrdq16_immune_ticks` |
| HRD-Q17 | Damage taken | Merged hero damage; never launched, knocked down, slammed or pierced; test loadout: dynamite 4, gauntlet poise x2 | Juicer plug rules; K1 base kit in the story | none (reuses CP-Q25, CP-Q40) |
| HRD-Q18 | Exposure | J-B landing on her body: **6** damage, HP floor **1**, Exposed **150**, Get-up **24**; overrides any mode; prone hurtbox `[-48, 72) x [0, 48)` | A big, earned payoff (about 40 % of her HP with the finisher hits) that is never an automatic kill | `hrdq18_damage`, `hrdq18_floor_hp`, `hrdq18_exposed_ticks`, `hrdq18_getup_ticks`, `hrdq18_hurtbox` |
| HRD-Q19 | Leash and reset | Pivot **256..880**; return home (x **608**) after **240** ticks with no hero within **320** px; no regeneration | No closed arena; no frustrating heal | `hrdq19_leash`, `hrdq19_home_x`, `hrdq19_return_ticks`, `hrdq19_return_px` |
| HRD-Q20 | Evasion class | **Never evadable**: swipe, shove, slide, landing; the loader rejects another value | K6, DM-Q18 boss blows and hazards | `hrdq20_evade_class` (`never` only) |
| HRD-Q21 | J-A geometry and timing | Stable `[352, 416) x [0, 48)`; warning **24**; slide **-x 3 px/tick for 16** ticks; settled remnant `[296, 376) x [0, 16)` | #178 18..30 warning; one authored direction away from her, toward the protected entry but never into it | sidecar `hrdq21_*` |
| HRD-Q22 | J-A stimuli | Her shove, a launched body, a blast; never hero melee or missiles | Cover is real and cannot be knocked over by accident | sidecar `hrdq22_stimuli` |
| HRD-Q23 | J-A slide hazard | **1** damage, knock **180 / 160** toward -x, once per hero per motion | "Push or stagger once per motion token"; the warning makes it fair | sidecar `hrdq23_*` |
| HRD-Q24 | Nest rule | Target on or behind stable bank A while she is at its face for **60** ticks → she shoves bank A | K8: the vantage is never an untouchable nest | `hrdq24_nest_ticks` (0 = off) |
| HRD-Q25 | J-B geometry | Post `[808, 824) x [0, 64)` non-solid; stack `[752, 880) x [96, 160)` no collision; footprint `[752, 880) x [0, 48)`; remnant 16 px | DV-4; the floor route stays open; no debris past 880 | sidecar `hrdq25_*` |
| HRD-Q26 | J-B stimuli | **2** distinct qualifying hits (human melee, swat, pounce, missile; weight 1) or **1** heavy (her shove, a blast); only after engage; bot hits ignored | #178 candidate numbers; Juicer "AI never triggers a rack"; no premature release | sidecar `hrdq26_hits`, `hrdq26_missile` (switch) |
| HRD-Q27 | J-B timing | Released **24**, falling **8**; landing on release tick + 32; one evaluation | #178 24-tick warning | sidecar `hrdq27_*` |
| HRD-Q28 | J-B hero blow | **1** damage, knock **120 / 140** away from the centre, every hero, credited to the initiator | Juicer rack hero damage; K3 world hazards hurt every hero | sidecar `hrdq28_*` |
| HRD-Q29 | Room | Section 3 table: 1056 px, ceiling 228, door 928..944 | #178 geometry; DP-Q45 | room and sidecar |
| HRD-Q30 | Ledges (K8) | Raised standable surfaces: bank A's top (48) while stable or disturbed; remnants 16; nothing at 49..128; boosted Donut may jump over her | Carl 80 and Donut 128 classes both satisfied; the only perch is threatened by the nest rule | validator rule |
| HRD-Q31 | Defeat event | `EnemyDefeated { target: PILE_BOSS_ID }`, `PILE_BOSS_ID = 0x504C` (also her `HeroHurt` source); once; door opens on that tick | K1 trigger; disjoint from `0x4255`, `0x5241` and the Engineer ram stamp space | none (constant) |
| HRD-Q32 | Death presentation | Species `pile_boss`, every recipe → HOARD-01 13..15; no gore for any cause; not CINE-eligible | Tone: a ruined person, horror without gratuitous gore | death profile data |
| HRD-Q33 | AI partner | `BossThreat` feed (Pound, Charge with lead **18**); no AI code change; M1, M2; follow-up above 1 AI hit per clear | Reuses the Juicer's M3 consumer; AI belongs to CA2 | `hrdq33_ai_jump_lead` |
| HRD-Q34 | Stand-in frames | Section 4 mapping, labelled STAND-IN; HOARD-02 replaces them | K10 | none (presentation) |
| HRD-Q35 | Audio | Existing cues only (boss tell, hit, impact, debris); no new asset | No invented sound in v1 | none |
| HRD-Q36 | HUD | **No boss bar** in v1 | Plan: setpieces add no HUD; #178 tone (no hit counter); avoids the HUD hot files | `hrdq36_boss_bar` (`false`) |
| HRD-Q37 | Gate | Active only in `f01_hoarder` under `cf02_rules`, reached by HRD-A's scene arm and later `--f01-room hoarder`; data kill switch `hrdq37_enabled` | No existing route or golden changes | `hrdq37_enabled` |
| HRD-Q38 | Fairness floors | Every tell ≥ **24**, every punish window ≥ **24**, every prop warning ≥ **18**; checked on load | Juicer floors; #178 warning range | `hrdq38_min_tell`, `hrdq38_min_punish`, `hrdq38_min_warning` |
| HRD-Q39 | Hits per hero | At most one per swipe, shove, slide motion and landing; in 2P every overlapped hero | Bounded hit tokens (slate invariants) | none (code) |
| HRD-Q40 | Events | One appended `CombatEvent::PileBoss { tick, event }` with `PileBossEvent::{Engaged, Tell { attack, target }, Strike { attack }, Hurt { source, damage, hp_left }, Flinch, Jammed { obstacle }, Exposed { credit }, GetUp, NestPush}`; reuses `HeroHurt` and `EnemyDefeated` | One variant keeps `combat.rs` churn minimal; `boss.rs` is never edited (plan section 7) | none (code) |

## 13. Genuinely open questions

1. **Feel:** the 34-tick swipe and 40-tick shove tells at 1x; whether the 18-tick shove jump window is fair for Carl at this point of the floor; whether 2 swipe damage is brutal or punishing.
2. **Collapse strength:** 6 damage plus 150 exposed ticks against the direct path; whether the collapse may ever kill her (adopted: never).
3. **Boss bar** (adopted: none) and the H0 sound (no asset exists).
4. **The door** (DV-2): owner may prefer an open exit with the grant moved to a later trigger.
5. **One more dangerous action** after the first playtest (#178 allows one; nothing is designed).
6. **HOARD-02 timing:** commission the shove, jammed, pinned, get-up and emerge keys, plus the pile props, now (parallel track, K10) or after the owner's feel review (adopted: parallel, not blocking).

## 14. Implementation brief (HRD-A..HRD-D)

Medium tier. Do not edit `docs/04-production/backlog.md`; put status in the PR body. Do not edit `src/boss.rs`, `src/shore.rs`, `src/camera_lab/` or the HOARD-01 sources. No design decision is left to the executor: a fixture that contradicts this sheet is a design defect to report.

| Step | Owns (plan 4.3) | Content |
| --- | --- | --- |
| HRD-A | `src/pile_boss.rs` (new, Bevy-free), `enemy_pile_boss_v0.json` (new), `f01_hoarder.room.json` and `.encounter.json`, `src/hoarder_fixtures.rs`, the trigger row swap in `story_unlocks_v0.json`, the scene arm (queued), the appended `CombatEvent::PileBoss` (queued on `combat.rs`) | Dormant, Reveal, Neutral, swipe, shove, Recover, Jammed (bank A as a plain solid, the post as a plain obstacle), Flinch, Defeated, door, leash, targeting, `BossThreat` feed, death profile, stand-in presentation |
| HRD-B | sidecar data, bait logic in `pile_boss.rs`, the `EnemyImpact` stimulus if BOIL-B lacks it | J-A states, slide hazard, nest rule, attribution |
| HRD-C | sidecar data, `pile_boss.rs` | J-B states, qualifying hits, missile stimulus if absent, landing, `crush`, Exposed, Get-up |
| HRD-D | `docs/evidence/f01-hoarder/` | Evidence, human runs, tuning notes, owner packet |

**Integration-plan notes for the orchestrator:** HRD-A needs the "new enemies" tick slot that the plan assigns to BOIL-B wire; if BOIL-B wire has not merged, HRD-A adds `src/combat_scene/pile_boss.rs` with one hook line in `combat_scene.rs`, queued like any other. HRD-A also joins the `combat.rs` queue for its one appended variant.

### Fixtures and tests per step

All headless, under `cf02_rules`, golden-pinned. Unless stated: Carl only, her pivot at **E = 760** awake in Neutral facing -x, so Carl at distance `d` stands at x `760 - d` (Reveal skipped by fixture setup), piles in their initial states, distances are pivot to pivot.

**HRD-A (direct fight):**

| Fixture | Setup | Input | Expected |
| --- | --- | --- | --- |
| `hrd_reveal_no_hits` | Dormant; Carl walks right from x 400 | jab her during the reveal | `Engaged` when Carl reaches 432; no `Hurt` and no tell for 90 ticks; then Neutral |
| `hrd_retry_short_reveal` | as above, after a retry | none | reveal 30 ticks |
| `hrd_swipe_hits_idle_carl` | Carl at d 60 | none | tell 0..33, `HeroHurt` 2 damage on attack tick 37, knock +160 / 200 away; Recover 43..90 |
| `hrd_swipe_jump_edges` | Carl at d 60, four runs | jump on attack tick 0 / 1 / 30 / 31 | hit / clear / clear / hit |
| `hrd_swipe_punish` | Carl at d 44 | jump on 20; jab on landing + 1 | jab lands inside Recover; no flinch (free damage) |
| `hrd_armour_tell` | Carl at d 44 | jab on attack tick 5 | `Hurt` 1, swipe continues |
| `hrd_poise_flinch` | Carl at d 300 (she walks); 2P Donut (biscuit) at x 420 | Donut casts four missiles within 120 ticks | `Flinch` on the 4th; a 5th within 60 ticks adds no poise |
| `hrd_shove_hits_idle_carl` | Carl at d 128 | none | `HeroHurt` 1 on lunge tick 23 (attack tick 62), knock 300 / 180 in the shove direction; lunge continues to 144 px; Recover 48 |
| `hrd_shove_jump_edges` | Carl at d 128, four runs | jump on attack tick 37 / 38 / 55 / 56 | hit / clear / clear / hit |
| `hrd_shove_out_of_reach` | Carl at d 204 | none | no `HeroHurt` |
| `hrd_shove_2p_both` | 2P: Carl at d 112, Donut at d 160 | none | both hit once each, one shove |
| `hrd_shove_jams_on_solid` | bank A as a solid; she at 470, Carl at 330 (behind the bank, d 140) | none | the box front would pass the face 416 on lunge tick 3: clamp at pivot 464, `Jammed` 72; no motion backward |
| `hrd_turn_flank` | Carl behind her at 40 px | none | `Turn` 20 ticks, then a swipe tell facing him |
| `hrd_leash_return` | Carl retreats to x 100 | none | after 240 ticks she walks to 608; HP unchanged |
| `hrd_not_evadable` | Donut, biscuit, reflex armed (DM-B) | stand at d 60 | swipe `HeroHurt`, no `HeroEvaded` |
| `hrd_defeat_grants_dynamite` | story pin; her HP 1 | jab | one `EnemyDefeated { PILE_BOSS_ID }`, one `CapabilityGranted { special_thrown_charge }` on the same tick; door open |
| `hrd_direct_clear_solo` | scripted Carl pilot, AI Donut | pilot | defeat and exit; bank A `stable` and J-B `supported` at the end; clear ticks reported |
| `hrd_direct_clear_donut` | P1 Donut, AI Carl | pilot | defeat and exit |
| `hrd_room_validator` | room data | load | reachability for Carl, base Donut, boosted Donut, 2P; no surface 49..128; ceiling 228; protected rects clear |

Unit tests in `pile_boss.rs`: data load, validation, pin, completeness, fairness floors; the swipe and shove reach tables of section 4; armour per mode; the exposure HP floor; target lock at the tell.

**HRD-B (J-A):** `hrd_ja_bait_slide` (Carl at x 330 behind the bank, she at 470: the shove jams on the face on lunge tick 3, disturbed the same tick credited to Carl, slide on jam tick + 25..+40, settled; exactly one slide); `hrd_ja_slide_hits_once` (she at 470, Carl standing at x 300: her shove disturbs the bank, the moving mass reaches him, one blow, 1 damage, knock -x); `hrd_ja_nest_push` (Carl on the bank top for 60 ticks: `NestPush`, immediate jam, slide; Carl falls); `hrd_ja_melee_no_effect` (ten jabs and three missiles: stays `stable`, missiles stopped); `hrd_ja_settled_walkable` (both heroes and she cross the remnant); `hrd_ja_restart_mid_slide` (retry: fresh state, hash equal to a fresh attempt).

**HRD-C (J-B):** `hrd_jb_hits_before_engage` (two hits in Dormant: `supported`); `hrd_jb_same_instance_once` (one move instance overlapping twice: `weakened` only); `hrd_jb_two_hits_release` (release, landing on release tick + 32, one evaluation); `hrd_jb_missile_counts`; `hrd_jb_bot_ignored`; `hrd_jb_bait_left` and `hrd_jb_bait_right` (shove into each face: pivot 760 or 872, landing overlaps her: 6 damage, Exposed 150, finisher hits are free damage); `hrd_jb_nonlethal` (HP 4 → 1, Exposed, not defeated); `hrd_jb_misses_her` (she outside: no effect on her); `hrd_jb_hero_in_footprint` (1 damage, credited, no `HeroEvaded`); `hrd_orders` (A then B, B then A, neither: exit reachable, validator passes); `hrd_restart_mid_fall`; `hrd_collapse_clear` (pilot using the bait).

**HRD-D:** the evidence list of section 11, three human runs with honest notes, PLAYTRACE strict verification of each recorded session, frame time, the AI counts of section 8.

**Goldens that must stay unchanged:** every golden listed in the CP, DP and DM work orders, every `cf02_*`, Engineer, boss, companion, party, CINE, death presenter, PLAYTRACE and AUTOPLAY golden, and `species_death_profiles.json` (the species comes through the overlay, as the Engineer's did).

## 15. Art follow-up HOARD-02 (parallel, not blocking, K10)

Through the [pixel-art pipeline](../07-agents/pixel-art-pipeline.md#routing-by-difficulty), on the HOARD-01 rig: **shove tell** (a low brace with one shoulder forward and both arms spread low; it must differ from the rear-up and from the gather at 1x), **lunge** (2-frame shoulder heave, dragged foot), **jammed** (impact on an obstacle, junk spilling), **pinned** (alive under junk, eyes open; must differ from defeat frame 15), **get-up** (2), **emerge** (2 to 3, rising from the mound). Props: bank A (stable, shudder, sliding, remnant), bank B (stack, post intact, cracked, broken, falling, remnant), the lair mound. No junk on her back (HOARD-01 guardrail). Until then every stand-in is labelled STAND-IN in captures.

## 16. Horizon fit, tone and rights

- **Horizon:** consulted the [character progression horizon](character-progression-horizon.md). Still consistent, no guide change here: she adds no player verb, unlock or button; she makes the base kit's jump, the punish, ranged pressure and terrain use the right answers. Reference path item 3 ("Early encounters / Hoarder: deepen hit reactions and combat-under-pressure; dynamite at an appropriate point") holds; K1 moves the dynamite grant to her defeat, and PT-DOC records that change-log row.
- **Tone:** a transformed human woman, frightening and tragic, never comedy; no gore on her death; hurt and pinned frames read as pain, not as a joke. No invented biography or book quote.
- **Rights:** no new art, audio or text; HOARD-01 sources and provenance unchanged. Gameplay values are original design hypotheses. Design by Claude Opus (top tier); review and execution not done yet.
