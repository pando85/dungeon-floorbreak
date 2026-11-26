# Floor 01 — The Quiet Setpieces: Fast-Food Refuge, Survivor Camp and Post-Juicer Regroup

**Status:** REST plan. This is the integrated design of the three quiet rooms of the Floor 01 run: REST-A (the fast-food refuge), REST-B (the survivor camp) and REST-C (the quiet after the Juicer). It adds no code. The playable greybox is the REST-WIRE work order ([workorder](../07-agents/floor-01-quiet-setpieces-workorder.md)). Sections A to F carry the **owner draft [#187](https://github.com/pando85/dungeon-floorbreak/pull/187)** ("memorable refuges and progression-safe transition beats", checked 2026-11-21) onto main, adapted to the code on main at `ff6c9d00`. The adaptations are marked *(adapted)*. Section G is the new integration plan: room ids, run order, geometry, grant data, pins, tests, evidence, adopted defaults and open questions. Original adaptation beats must not be presented as literal book scenes.

**Owner decision (2026-11-25):** "go with all". Items that were owner-gated proceed with adopted defaults recorded by ID (REST-AD1 to REST-AD18). Everything must end on main for next week's playtest, reachable in `--scene f01_late --f01-run` in story order. New behaviour is on by default where the spec allows. Large unfinished items merge as a labelled playable greybox. Game feel stays the owner's call: once merged, every REST room is **"merged, game-feel review pending"**, never accepted.

## Intent: contrast IS part of a memorable action floor

Floor 01 risks becoming an uninterrupted series of loud encounters: Dozer, goblins, Llamas (#181), Hoarder (#184), Boiler (#176), Dozer revenge (#178), Slimes/Rot (#185), Juicer, Ball of Swine. **Three quiet, playable transitions** should make the violence more memorable by creating space and tonal surprise:

1. **F01-REST-A: "The Restaurant That Should Not Be Here"** — dissonant 1990s fast food, after the Infernal Corral, **before the Hoarder**. Carries Donut's Light on Your Feet story reward.
2. **F01-REST-B: "Meadow Lark's Temporary Order"** — an exhausted, organized human survivor camp after the labyrinth, before troglodyte/gym escalation.
3. **F01-REST-C: "The Quiet After Juicer"** — unmistakable mechanical destruction and a tiny *noncombat* test of newly earned physical/teleport skills after the Juicer, before the Ball of Swine. It grants no ability: the boss's single existing defeat trigger does that.

No traditional shop, arbitrary inventory, resource farm, scripted dialogue wall, forced NPC escort, lengthy cutscene, collectible hunt, new control or mandatory camera split. Short narrative observations and physical play carry the personality. All three locations reuse common brick/stone Floor 01 art with localized identity, not global biome swaps.

## A. Fast-food refuge — "The Restaurant That Should Not Be Here"

### Emotional thesis

Last room: burnt orange llama throats, fire-glob sounds, aggressive Carl and Donut. One arch later: **clean blue/white tiles, fluorescent hum, molded plastic benches, empty booths and a child play structure** in a place where there should be solid rock. A cold musical jingle is briefly cheerful but distantly unsettling. Then the dungeon stone is visible through outward-facing restaurant windows. This is an outrageous **juxtaposition**, not a horror boss room in disguise.

**Owner-approved art baseline:** [art atlas](../02-art/floor-01-art-atlas-v1.md) area 04 and [`atlas-v1/04-fast-food-refuge.png`](../../assets/concept/floor-01/atlas-v1/04-fast-food-refuge.png) (mood only). No real-world restaurant name/logo, commercial music, source-novel quote, unapproved character sprite or fabricated franchise brand.

### Functional story / level position

After the first Bad Llama and the two-llama Infernal Corral M2, **before the Hoarder**. DONUT-PROG-01 DP-Q02 places a **golden magic item box** reward in the first safe room. It grants `jump_reach_boost` through `f01_safe_room_reward_found`. This one passive raises Donut's existing jump reach (apex 114.92 to 137.95 px, DP-Q14). There is no double jump, no extra input and no equippable item. **Do not implement a second unlock system or grant the boost at the corral exit.** Use the CP-A story ledger and the DP-A ability plumbing.

The main path **passes right by the reward**. The trigger fires once, at the box. The act of finding must visually match the event. No invisible trigger far past the box, no reward for merely entering the room, no duplicate grant on checkpoint retry, no inventory UI.

### Player passage: ~60–100 seconds first run, ~15–30 seconds practiced

| Beat | Spatial function | Player experience | Authoring contract |
| --- | --- | --- | --- |
| **F-A0: hot→cold threshold** | Left entrance; ~1/4 screen of scorched masonry shifts to white tile | Combat audio cuts; abrupt clean ambience. Both heroes keep control. | Safe; no pursuing lava glob, enemy spawn or delayed kill through the door |
| **F-A1: the empty restaurant** | Counter, booth, fake window opening into rock; continuous floor lane | Discover the absurd setting and an untouched, strangely arranged children's play area | Environmental storytelling *without required interaction*, no copyrighted menu text |
| **F-A2: the found treasure** | Gold-box focal point on the normal route | The weirdly plausible reward changes Donut, with one restrained upgrade notice | The story grant `f01_safe_room_reward_found` exactly once; it persists on retry and in 1P/2P |
| **F-A3: the first higher leap** *(adapted)* | 128 px shelf **at the exit**, next to a staff door | Donut reaches a shelf she could not reach before. Standing on it opens the staff door for Carl. The player FEELS the upgrade. | Option A1 without an Interact button (section G.4, REST-AD5) |
| **F-A4: re-enter the dungeon** | Protected exit to less cluttered, ominous masonry | Cheerful ambience stops, distant insect movement begins | Short exit; no repeated dialogue or checkpoint lock |

**Layout:** the owner draft's hypothesis was ~832–1024 px of walkable sequence on a 16 px grid. The greybox is **1600 px** wide (REST-AD4: the room is built around the table's existing reward column at x 1200..1240). It has open central ground, a counter behind the collision plane, noninteractive booths, and an optional children's play structure of two low decks **without a new movement rule**. The art should read as a real establishment in a 640 px camera, not 20 furniture obstacles to jump.

**The memorable noncombat moment:** the party passes a lit plastic children's slide. Through a restaurant window one sees only thick, raw underground stone. One yellow playground ball gently rolls into the otherwise pristine dining area (a bounded, non-collidable visual animation). After the violence, the absence of danger is funny and oddly disquieting. This is an **original visual gag candidate** for the art pass (REST-Q5), not canon or a scripted player cue.

### The 128 px jump question *(adapted)*

DONUT-PROG-01 expects a **first 128 px boost-only ledge at the refuge exit** (DP-Q18: "at I-08's exit or later"). The owner draft offered two options:

- **A1 (faithful to the progression spec):** after the reward, Donut uses the 128 px reach to a short shelf, and one Interact R1 affordance opens a **ground-level passage for Carl**. The door then stays open. Solo play swaps to Donut; 2P regroups on the floor with both heroes in the camera. A quick 10–15 s payoff, not a multi-room puzzle, and no autonomous AI precision.
- **A2 (fallback):** the 128 px shelf is optional (an overlook), and the ordinary exit stays at ground height.

**Adopted: A1, without a new button (REST-AD5).** Interact R1 does not exist in the scene ([controller contract](mvp-controller-contract.md): not implemented). So **standing on the shelf** is the act: a new lock event `hero_on_surface` opens the staff door on the first tick Donut is grounded on the shelf, and it stays open. A2 stays available as a one-line sidecar change (drop the lock), recorded as the game-feel fallback (REST-Q1).

Do not create two long paths, lock Carl permanently on the wrong side, consume a mana charge, require Puddle Jumper or add a double jump. The boost is real through the story ledger and the mobility engine, not a scene-local gravity multiplier.

**No shop or healing invention:** the REST checkpoints use the existing checkpoint policy. There is no full restore because the room is "safe" (REST-AD8). No equipment picker, purchases, currency, new boss or patron rewards.

### What the player can do (without an objective screen)

- Look at the counter and the booths. They are decorative: there is no inspection mechanism.
- Step onto the children's play structure as expressive navigation (two low decks, Carl-reachable). Soft toy balls never alter contact rules.
- Reach the box, see Donut's restrained `LIGHT FEET` notice, then use the new reach at the exit.

No forced talk, cutscene, dialogue choice or stat purchase. The safe-room budget is **architectural surprise and one new mobility affordance**.

## B. Survivor camp — "Order, in a place with no order"

**Position:** after the Broken Ring (`ring_split`, which shows the first distant sight of the camp), before the troglodyte territory and the Juicer. The atlas shows temporary human organization: crates, improvised cloth and canvas, an old stone enclosure and a practical camp layout. It is **a human refuge**, not a second restaurant or a "save room shop".

**Emotional thesis:** the fast-food room felt absurdly *normal* because its architecture was wrong; the camp feels painfully *human* because the people try to organize themselves against impossible surroundings. It should acknowledge stakes without a speech.

**First visit ~75–120 seconds, replay 15–25 seconds**, subject to gamepad pacing; no inventory, recruitment or quest list. Beats:

1. **Arrival** *(adapted, KR14)*: the Ring owns the distant view (R3 silhouettes). REST-B opens on the shared marker (torn cloth strip and warm lantern, AREA-RINGS-01 `ring_marker_a`) at its entrance. A sentry faces the *opposite* direction. In the greybox the sentry is a static marker.
2. **Cross a living camp:** several nonhostile human figures near supplies, gear repair and hand-drawn maps, as **clearly labelled greybox markers** until native art is approved. One childlike prop or improvised aid shows practical ingenuity, with no exploitative misery.
3. **One physical/story note:** a survivor quietly repositions a pile of crates after the party passes, restoring a safe corridor. This is restrained in-world animation for the art pass, **not a wall puzzle** and not in the greybox.
4. **Checkpoint / emotional anchor:** a protected central light and quiet ambience. The player continues at will.
5. **Leave toward gym territory:** stone stained with chalk and odd hand-painted strength motifs builds anticipation. On main the next room is the Juicer itself; the troglodyte lead-in (#191) gets a slot (section G.2).

**Co-op:** neither player is forced into conversation or waiting. Shared camera bounds; no role swap. NPCs have no combat AI. No enemy or hazard system runs inside the camp. Any dynamic NPC face or animation must be native authored, not a concept sprite.

**Narrative gate:** no survivor names, dialogue, abilities or interactions beyond the atlas without script/source review (REST-Q2). The owner-selected concept boards (`docs/02-art/book-01-odette-meadow-lark-concept.md`: Imani, Elle McGibbons, Brandon and Chris Andrews) are mood references, not production sprites; Odette is not a physical Floor 01 survivor and is not placed in the camp. The camp **grants no ability**.

## C. Post-Juicer regroup — "Power, then silence"

**Position:** after the `boss_f01` Juicer and before the floor's final room. On main the final room is the mastery arena; the Ball of Swine (#192) gets a slot (section G.2). The boss stays a **separate live scene with story pins**, not an actor dragged into a mixed level loader.

**Emotional thesis:** after the absurd, violent gym fight, the machinery stops. A hanging punching bag swings once; an echoing weight clang trails off. The new abilities came from the **single existing Juicer defeat event**: Carl `fist_reinforced` and Donut `special_short_blink`. This room grants nothing again and has no reward.

**First visit 45–70 seconds, replay 10–20 seconds**, no cutscene lock. Three parts:

- **C1 / broken gym egress:** cracked mirrors and wrecked equipment; fully protected. The last fight's consequences are visible, but no mutable boss prop persists across the scene change.
- **C2 / tiny positive-control playground:** optional **test affordances** for the *already granted* powers: a heavy bag that the reinforced fist hits (REST-AD12), and a **112 px clear blink lane** for Donut's collision-safe Puddle Jumper. Both are short callouts with no "use both together" gate. The normal route is base movement.
- **C3 / arena entrance:** the stone corridor curves into broad circular grooves; a distant heavy gate clang announces the next arena. A clean checkpoint with no persistent hazard.

**Do not:** teleport both heroes against P2's will, consume Puddle Jumper mana outside the normal rules, build a blink-through-wall gate, require two simultaneous buttons, or add a skill tree or another ability before the final boss.

## D. Connected Floor 01 pacing map *(adapted)*

The owner draft's map, with what exists on main and the open slots (section G.2):

```text
Opening / Murder Dozer (#179, slot S0) → first goblins → Guild / biscuit
     → Infernal Corral (room `corral`, #181)
     → [REST-A: FAST-FOOD / DONUT JUMP REWARD]       room `rest_refuge`
     → Scatterers (slot S1) / Hoarder                room `hoarder`
     → Goblin Boiler → Dozer revenge                 rooms `boiler`, `dozer`
     → Slime / Rot Sticker pockets                   room `hazards`
     → The Broken Ring (#189, more regions: slot S2) rooms `ring_court`, `ring_split`
     → [REST-B: SURVIVOR CAMP]                       room `rest_camp`
     → Troglodytes / gym approach (#191, slot S3)
     → Juicer                                        room `juicer`
     → [REST-C: NEW POWER REGROUP]                   room `rest_aftermath`
     → Ball of Swine (#192, slot S4) / mastery arena room `mastery`
```

**Story chronology:** the integration plan's K1 grants Carl's dynamite on the Hoarder's defeat. Never place dynamite in REST-A or require it there. The Boiler comes after the grant.

**Ownership:** #181 owns the llama gameplay and its M3 threshold; REST-A begins **after** that threshold. DONUT-PROG-01 owns the jump capability; CP-A owns the ledger. The [integration plan](../07-agents/f01-playtest-integration-plan.md) (#182) owns run routing and the hot-file queue (its section 16 points here). #176 owns the shared prop physics; the Juicer's state belongs to its boss scene.

## E. Acceptance

| ID | Required test | Evidence | Automated by (section G.9) |
| --- | --- | --- | --- |
| REST-A-01 | From the corral exit into the refuge | Real in-engine visual contrast; no stray enemies or projectiles follow | `the_corral_exit_relaunches_into_the_refuge`, refuge captures |
| REST-A-02 | The room can be crossed with no optional interaction | Safe floor; no purchase, quest or menu | `rest_refuge_solo_carl_swap` and the other crossings |
| REST-A-03 | Reward `f01_safe_room_reward_found` | Exactly once at the box; one notice; persists through a retry and a checkpoint respawn and in solo/2P (a fixture restart returns to the room start and the box grants again, once) | `rest_refuge_grant_once_at_the_box`, `rest_refuge_retry_keeps_the_grant` |
| REST-A-04 | Donut's jump before and after the reward | The 128 px shelf only after the boost, no double jump; Carl's route stays valid | `rest_refuge_base_donut_cannot_open`, `rest_refuge_shelf_is_boosted_only` |
| REST-A-05 | A1 character gate (or the A2 fallback) | Solo swap, solo Donut-active and local 2P all reach the exit; no AI precision gate | `rest_refuge_solo_carl_swap`, `rest_refuge_solo_donut`, `rest_refuge_2p` |
| REST-B-01 | Labyrinth transition and camp safety | No hostile damage or spawn; recognizable human occupancy; two-player camera | `rest_camp_crossing_*`, camp captures |
| REST-B-02 | Revisit / fast advance | Nothing traps the player or imposes an unskippable wait | `rest_camp_crossing_*` (exit tick bound) |
| REST-C-01 | Juicer defeat → rest without duplicate unlock | The ledger grants only once; restart-safe | `rest_aftermath_no_duplicate_unlock` |
| REST-C-02 | Optional gauntlet/blink affordances | Usable, never mandatory; the blink destination is collision-safe | `rest_aftermath_blink_lane`, `rest_aftermath_crossing_base_moves` |
| REST-C-03 | Transition to the final room / checkpoint | No softlock, no carried hazard, no premature boss | `the_aftermath_exit_relaunches_into_mastery` |
| REST-ALL-01 | Both solo heroes and real 2P, 640×360/400 and 1080p | Native footage and path continuity; silhouettes readable | captures; the real two-pad run is the owner's |
| REST-ALL-02 | Live PLAYTRACE and same-input verification | Reward flags, checkpoint and scene transition events/hashes | `f01_rest_*_recording_verifies` |

**Mandatory experience question:** after each refuge, do players remember the previous fight more clearly and feel excited about the next part? Quiet rooms are not a 3-minute forced puzzle or an exposition dump. **A human at-speed playtest decides whether the durations are right.**

## F. Implementation and ownership strategy *(adapted)*

1. **REST-WIRE** (medium tier, two PRs in sequence, REST-AD11, section G.10): **PR 1 (REST-A)** lands the refuge room listed in the manifest after the corral, the grant move with the DP fixture retarget, the staff-door lock event and the swap hint, with the refuge fixtures and evidence; **PR 2 (REST-B and REST-C)** lands the camp and the aftermath, their decor and the full twelve-room run walk. The order keeps the riskiest work first; under "go with all" both land for the playtest, since the Ring rooms and the Juicer's ledger grant are already on main. PR 1 must list the refuge, or the boost would be unobtainable in `--f01-run` between the two PRs.
2. **AREA-FASTFOOD-01** (art, separate): the original 1990s restaurant kit (section G.7).
3. **AREA-CAMP-01** (art, separate): the camp interior, reusing AREA-RINGS-01's camp prop family; survivor figures only after narrative review (REST-Q2).
4. **Integration QA** stays inside REST-WIRE: solo Carl-active, solo Donut-active and 2P fixtures; checkpoint retry; story-flag idempotence; replay; Bevy captures at 640×360/400 and 1080p.

No new geometry system, camera rewrite, companion mobility, dialogue system, NPC AI framework or reward framework belongs in these rooms.

## G. Integration into the floor run (REST plan)

### G.1 What main holds today (checked at `ff6c9d00`)

- **The run** (`--scene f01_late --f01-run`, K17) relaunches the binary per room. `f01_run::first_room`, `next_room`, `relaunch_argv` and `carry_for` read the rows of `assets/levels/f01_late/manifest.json` in order, so **a listed row joins the run with no run code**. The ledger travels in `--f01-carry`; `carry_for` returns the room's `story_snapshot()` under the story loadout (the corral, which has no story runtime, carries the empty ledger).
- **Rooms listed:** `corral`, `hoarder`, `boiler`, `dozer`, `hazards`, `ring_court`, `ring_split`, `juicer`, `mastery`; `unlisted_rooms` is empty. `MANIFEST_HASH = 0x5a29_871b_5b47_bb85` (`src/f01_late.rs`).
- **The story order** has a *point* `refuge_light_feet` between `corral` and `hoarder` that places the `jump_reach_boost` grant (trigger `f01_safe_room_reward_found`). The KR12 test `f01_late::check_story_order` proves that no room pin holds a capability granted later.
- **The grant row** (`assets/data/gameplay/story_unlocks_v0.json`, `TABLE_HASH = 0x00f9_0eb3_7c41_fd8a` in `src/story_unlock.rs`, also stamped as the `story_unlocks` content pin of every PLAYTRACE recording): a zone trigger for hero 1 over x 1200..1240 at any height, granting `jump_reach_boost`, flag `donut_prog`, **`armed_in: ["corridor"]`**: DP-A's placeholder in the legacy story corridor (`--donut-prog`). No `--f01-run` room grants the boost in play today; the Hoarder's entry pin `f01_pin_pre_pile_boss` holds it (K9, R4).
- **Ledger hashing** mixes the held `(hero, id)` pairs and each receipt's hero, capability and tick; it does not mix the receipt's source or the armed trigger list. A zone fires once, on the first tick the hero's pivot is inside, never while the party is defeated. `rearm` marks a trigger spent when its grants are already held, so a carried or retried ledger never grants twice. A defeat retry and a checkpoint respawn keep the grants (CP-Q07). A fixture restart (R / Select hold) restores the room's declared start state: the carried ledger united with the room's pin (`carry_in`), with the receipts cleared and the triggers re-armed. In the refuge that start is the empty ledger carried from the corral, so a restart drops the boost and the box grants it again on the next pass, once.
- **Room pins** are `f01_late` rows chosen by id and arm nothing. A room that grants arms its trigger through its own `SceneKind` (precedent: the Hoarder's `PileBoss`). A room with no ledger state uses a sidecar `story_pin` that names no table row (precedent: the corral's `f01_pin_corral_entry`, `SceneKind::Corral`). A pin row must hold at least one capability.
- **Encounter sidecars** (`floor01-encounter-v1`, `src/encounter.rs`) reject `grants` ("a room never grants"). Locks open only on `enemy_defeated` today (`LockEvent` is `#[non_exhaustive]`, "appended to by later steps"; `combat_scene/env.rs` matches it exhaustively). The validator checks reachability for Carl 80, base Donut 104 and boosted Donut 128 px and 2P, the gate classes, the DP-Q45 ceiling rule (a boosted-standable surface above 42 px needs `ceiling_px` ≤ 228, which is a real collision slab) and the K8 nest rule (an 81..128 px surface needs a threatening enemy or a vantage such as `lane_ceiling`).
- **The 128 px tests:** `donut_prog_fixtures::donut_prog_ledge_128` (boosted lands, base Donut and Carl do not), the DP-F ledge evidence, and the Ring's boosted-only shelf (`ring_court` platform 2, RING-07).
- **HUD:** the location strip shows the manifest title in capitals and `TRY n` (F01-HUD-ROOM). The grant notice of `jump_reach_boost` is `LIGHT FEET` (`LabelId::LightFeet`). The room-clear panel states facts only. AUDIENCE-RUN is live in every room after the corral.
- **Controls:** Interact R1 is not implemented. The solo swap (L1/LB, keyboard Q) is.
- **Ring 4b camp hints (AREA-RINGS-01, KR14):** `ring_breach_r3` and `ring_camp_far` show the far camp with anonymous silhouettes; the shared marker `ring_marker_a` (24 × 56, pole, torn canvas strip, the camp lantern) stands before the `ring_split` exit, and "REST-B places the same piece at its entrance".
- **Art:** `assets/concept/floor-01/atlas-v1/04-fast-food-refuge.png` is a mood reference. `AREA-FASTFOOD-01` is already a named row of the [area kits plan](../07-agents/floor-01-area-kits-plan.md). `AREA-REFUGE-01` is Mordecai's guild room, a different room.

### G.2 Run order (REST-AD2)

| Run index | Room id | Scene | Title (REST-AD1) | Milestone | Story pin at entry | Grants in play |
| --- | --- | --- | --- | --- | --- | --- |
| *slot S0* | (#179 Dozer opening) | — | — | not scheduled | — | — |
| 0 | `corral` | `f01_corral` | The Infernal Corral | greybox | none (biscuit read through) | none |
| **1** | **`rest_refuge`** | **`f01_rest_refuge`** | **The Wrong Restaurant** | **greybox** | **none (`f01_pin_refuge_entry`, no table row)** | **`jump_reach_boost` at the box** |
| *slot S1* | (Scatterers) | — | — | deferred (K14) | — | — |
| 2 | `hoarder` | `f01_hoarder` | The Hoarder | boss | `f01_pin_pre_pile_boss` | dynamite |
| 3 | `boiler` | `f01_boiler` | Goblin Boiler | setpiece | `f01_pin_post_pile_boss` | none |
| 4 | `dozer` | `f01_dozer` | The Machine Remembers | setpiece | same | none |
| 5 | `hazards` | `f01_hazards` | Slimes and Rot Stickers | greybox | same | none |
| 6 | `ring_court` | `f01_ring_court` | The Shutter Court | greybox | same | none |
| 7 | `ring_split` | `f01_ring_split` | The Split Passage | greybox | same | none |
| *slot S2* | (#189 further Ring regions) | — | — | not scheduled | — | — |
| **8** | **`rest_camp`** | **`f01_rest_camp`** | **The Survivors' Camp** | **greybox** | **`f01_pin_post_pile_boss`** | **none** |
| *slot S3* | (#191 troglodytes, gym approach) | — | — | not scheduled | — | — |
| 9 | `juicer` | `boss_f01` | The Juicer | boss | `f01_dynamite_held`, `f01_donut_boss_held` | gauntlet, Puddle Jumper |
| **10** | **`rest_aftermath`** | **`f01_rest_aftermath`** | **After the Juicer** | **greybox** | **`f01_late_mastery`** | **none** |
| *slot S4* | (#192 Ball of Swine) | — | — | not scheduled | — | — |
| 11 | `mastery` | `f01_mastery` | The Mastery Arena | greybox | `f01_late_mastery` | none |

The slots are **documentation only**: no manifest row, no code. When a slotted room lands, its own step inserts its row at the slot and re-pins `MANIFEST_HASH`, the same way KR15 placed REST-B. #192 itself says not to replace `mastery` silently; until that integration decision, REST-C's exit leads to `mastery`.

**Manifest change** (REST-WIRE): three `rooms` rows at the positions above, each `"rules": "cf02_rules"`, `"milestone": "greybox"`. In `story_order`, the point `refuge_light_feet` becomes the room row `{ "id": "rest_refuge", "kind": "room", "pins": [], "grants": [{ "capability": "jump_reach_boost", "trigger": "f01_safe_room_reward_found" }] }`; `{ "id": "rest_camp", "kind": "room", "pins": ["f01_pin_post_pile_boss"], "grants": [] }` follows `ring_split`; `{ "id": "rest_aftermath", "kind": "room", "pins": ["f01_late_mastery"], "grants": [] }` follows `juicer`. KR12 still holds: the refuge's grant precedes the Hoarder's pin, the camp's pin holds nothing granted after `hoarder`, and the aftermath's pin holds nothing granted after `juicer`.

### G.3 Room geometry (greybox, layout px, y down, 16 px grid)

All three rooms use the floor-room conventions of the Ring rooms: a `floor01-demo-hall-v1` room file composed from the existing Floor 01 kit (no new art), height 660, walking line `floor` 580, spawns Carl 112 and Donut 80, `camera_intro` 112, `protected_spawn` {16, 484, 208, 96}, empty `props`, `enemies` and `hazards` arrays in the room file, the gameplay in the `*.encounter.json` sidecar (the *(decor)* items live in `f01_rest.rs`, G.6) with `goal: reach_exit` and an **empty roster** (REST-AD13).

**REST-A `rest_refuge`** (width 1600, `ceiling_px` 228: the slab underside is y 352):

| Item | Rect / value | Purpose |
| --- | --- | --- |
| platform 0: play deck | {464, 548, 96, 16} (32 px) | gate `carl`; expressive, optional |
| platform 1: slide top | {560, 532, 64, 16} (48 px) | gate `carl`; optional; 48 px above the free height, so the ceiling rule applies (met) |
| platform 2: staff shelf | {1296, 452, 96, 16} (128 px) | gate `donut_boosted`; vantage `lane_ceiling` (K8); the lock's surface |
| checkpoint | {1104, 484, 48, 96} | just before the box |
| reward box (decor) | x 1200..1240, 40 × 32 on the floor, non-collidable | the visual of the grant column |
| grant zone (table) | hero 1, x 1200..1240, any y | unchanged numbers, now armed in `rest_refuge` |
| lock `staff_door` | solid {1440, 352, 16, 228}; `opens_on` `hero_on_surface` donut, surface 2 | seals floor to slab until Donut stands on the shelf |
| exit | {1504, 484, 64, 96} | after the door |

```text
x:   0        224     464 560 624              1104 1200 1240 1296   1392 1440  1504 1568 1600
y352 ==========================================================================|=========  slab (228 px)
                                                                 [==shelf==]   |D
                                                                   128 px      |O
                            [slide 48]                                         |O
                       [deck 32]                                               |R
y580 SSSSSSSSSSSSSS ............................. CC .. [BOX] ....................... EEEE
     protected spawn  F-A0 threshold | F-A1 dining room, play area | F-A2 box | F-A3 shelf+door | F-A4 exit
```

The door spans the walking line to the slab, so nobody jumps over it (boosted apex from the shelf would be 266 px, above the 228 px slab). The shelf's left edge is 56 px past the grant column, so Donut always holds the boost when she can first reach it. Nothing within reach of the shelf is higher than the floor, so base Donut (104) and Carl (80) cannot chain onto it (the validator's bypass check proves it).

**REST-B `rest_camp`** (width 1536, no `ceiling_px`, no platform: no surface at all):

| Item | Rect / value | Purpose |
| --- | --- | --- |
| crate stack (decor) | {704, 548, 64, 32}, non-collidable | no platform: the validator's lane rule rejects a 32 px deck (REST-IMP-8) |
| entrance marker (decor) | 24 × 56 at x 232, on the floor | `ring_marker_a` once its sprite is wired; a greybox pole-and-lantern rect until then (KR14) |
| survivor markers (decor) | four flat grey figures: sentry 16 × 48 at x 520 facing left; repair 16 × 44 at x 880; seated 16 × 28 at x 1040; map table figure 16 × 46 at x 1200 | labelled greybox, no faces, non-collidable, static |
| central light (decor) | lantern at x 760 | the anchor of beat 4 |
| checkpoint | {744, 484, 48, 96} | under the central light |
| exit | {1456, 484, 64, 96} | chalk marks on the last 128 px (decor) |

```text
x:   0      232     520      704 768    880   1040  1200          1456 1536
y580 SSSSSSS M ...... s ....... [crates:decor] L/CC . r ..... t ..... m ......... EEEE
     spawn  marker  sentry     step     light  repair seated map         exit (chalk)
```

**REST-C `rest_aftermath`** (width 1280, no `ceiling_px`):

| Item | Rect / value | Purpose |
| --- | --- | --- |
| C1 egress decor | x 224..512: cracked mirrors, wrecked bench and weights (decor; the bench is decor, not a 24 px platform, REST-IMP-8) | the fight's consequence |
| heavy bag | `statics` entry `hit_target` {624, 516, 32, 64} (bottom on the walking line; the plan's y 452 floats) if a standalone hit target needs no machine; otherwise decor (REST-AD12). Landed as an inert static, REST-IMP-9 | optional gauntlet test, non-solid, never blocks |
| blink lane | x 768..880 (112 px), floor chalk marks at both ends (decor) | optional Puddle Jumper test; no solid in or after it |
| checkpoint | {1056, 484, 48, 96} | C3, before the exit |
| exit | {1200, 484, 64, 96} | circular grooves on the back wall (decor) |

```text
x:   0      224          512  624 656   768      880   1056  1200 1280
y580 SSSSSSS  C1 mirrors, wreck  [bag]   |<-112->|      CC     EEEE
     spawn    broken gym egress   C2 playground          C3 arena entrance
```

REST-B and REST-C have no lock and no gated surface: the route is base movement for every class.

### G.4 Entry, exit, checkpoint and the staff door

- **Entry:** each room starts like every run room: the relaunch, the carried ledger, the protected spawn, the location strip (`THE WRONG RESTAURANT · TRY 1`). No enemy, hazard or projectile exists in a REST room, so nothing crosses the threshold (REST-A-01).
- **Exit:** reaching the exit zone ends the room; the room-clear panel (facts only: time, tries, no kills) confirms the relaunch. A REST room has `goal: reach_exit` and an empty roster, so the panel needs no roster facts (REST-AD13).
- **Checkpoint:** one per room (positions above), with the existing policy: a defeat respawns there and keeps state. In practice a REST room has no damage source. No heal or mana refill beyond what the checkpoint policy already does (REST-AD8).
- **The staff door (REST-AD5, REST-AD6):** a sidecar lock whose `opens_on` is the new event `{"event": "hero_on_surface", "target": "donut", "surface": 2}`. It opens on the first fixed tick on which hero 1 is **grounded on platform 2** and stays open for the rest of the attempt; a checkpoint respawn keeps it open, a fixture restart (R / Select hold) closes it again with the rest of the room start, which also drops the boost (G.1), so the box re-grants it on the way to the shelf. Its opened state is sim state: it is hashed when the room has such a lock and mixes nothing otherwise, so every other room's hash is unchanged.
- **Loadouts:** the refuge is only completable under the Story or All loadout. Under Story the trigger is armed and the box grants the boost; under All the boost is held and the trigger is spent. A flagless launch (no story flags: `combat_of`, a bare module smoke, ad-hoc tools) arms nothing, so the door never opens and the room is **sealed by design**; the entry line says so (`boost=none`). `--scene f01_late` always resolves to Story or All. All three rooms start with Donut's biscuit unlocked (`DonutSetup::unlocked(8)`, as the Ring rooms and the corral): the boost requires `primary_ranged`, and a trigger whose guard fails is spent without granting, which would seal the refuge.
- **Solo hint (REST-AD7):** in solo, if Carl is the active hero, the staff door is closed and Carl has stood within 96 px left of it for 90 ticks, the HUD shows the headline `SWAP TO DONUT` (a new presentation-only `LabelId`, never hashed). It clears when the door opens or the swap happens. 2P shows no hint. The hint may never show when the autonomous companion (`cf02_free`) jumps to the shelf and opens the door first; that is intended (the door is latched and the beat still lands).

### G.5 The grant: data change and pins

**Data change (REST-AD3, REST-AD4).** In `story_unlocks_v0.json` the row `f01_safe_room_reward_found` keeps its id, kind, zone (hero 1, x 1200..1240, y ±100000), grants, flag and `cp_q`. Only `armed_in` changes, `["corridor"]` → `["rest_refuge"]`, and the table file's top-level `provenance` string gains one sentence recording the move (trigger rows have no provenance field of their own). `SceneKind` gains `RestRefuge` (serde `rest_refuge`): no pin row carries it; the refuge's story setup is `for_scene(RestRefuge)` under the program flags, which holds nothing and arms this trigger. This is the KC9 "swap": a single row moves, so no duplicate trigger id is added and no stub trigger is needed (the room is authored around the existing column).

**Why the zone numbers stay.** DP-A's fixtures (`donut_prog_story_unlocks`, `donut_prog_grant_survives_retry`, `donut_prog_primary_mapping`, `donut_prog_grants_2p`, the closure test in `donut_prog_fixtures`, and the blink fixture story `PuddleAndZone`) relied on the corridor arming. They are **rebuilt on `SceneKind::RestRefuge`** in place of `SceneKind::Corridor`: under the new table that scene arms exactly the row the corridor armed, with the same flag filtering, so each setup is semantically identical (a `--carl-prog`-only setup stays inert, which the closure test asserts). `StorySetup::with_trigger` is **not** used: it pushes the row whatever the hero flags are, which would give the closure test's carl-only setup a runtime and break its assertion. **Why the hashes stay byte-identical:** `UnlockLedger::hash_into` mixes only the held `(hero, id)` pairs and the receipts' hero, capability and tick, never the scene, the source string or the armed trigger list; same row, same zone, same ticks. The receipt-source assertions stay too. The blink stories `Puddle` and `PuddleAndDynamite` stay on `Corridor` (they arm nothing that matters), and the assertions that the corridor is inert under `--carl-prog` (`carl_prog`, `story_unlock`) pass untouched. Only the structural assertions about where the row is armed change: the `story_unlock` test (`safe.armed_in == [Corridor]`) and the `donut_prog` test (the corridor setup's `trigger_ids()`). If any DP fixture hash or committed DP replay line moves, the implementation stops and reports.

**Behaviour change, recorded.** The legacy story corridor (`--donut-prog` outside `--scene f01_late`) **stops granting the boost**; `--donut-light-feet` still holds it there. The Hoarder's direct launch still pins it (R4). `f01_pin_pre_pile_boss` is unchanged; it is no longer the only source of the boost in a run, which settles the K9 "provisional" note in play without a data change (REST-AD3).

**Pins that move (each with its reason):**

| Pin | Change | Reason |
| --- | --- | --- |
| `story_unlock::TABLE_HASH` | re-pinned | the `armed_in` and `provenance` text of one row. Every PLAYTRACE recording stamps the table as its `story_unlocks` content pin, so **any local recording made before REST-WIRE fails strict verification with a content mismatch**; this is expected and stated in the PR |
| `f01_late::MANIFEST_HASH` | re-pinned (from `0x5a29_871b_5b47_bb85`) | three `rooms` rows, three `story_order` rows and one REST-WIRE sentence in the manifest's `status` history (re-pinned once per REST-WIRE PR); every late-floor recording made before it fails strict verification the same way |
| `f01_rest` room and sidecar hashes | new, six | the three `*.room.json` and three `*.encounter.json` files, pinned in the module like `f01_ring` |
| `f01_late::SCOPED_PIN_KEYS` | 19 → 25 entries | six scoped keys `f01_rest_{refuge,camp,aftermath}_{room,encounter}`; `pin_applies` maps each to its room id |
| `playtrace::live::data_pins()` | +6 entries | the six `f01_rest` pins are stamped; the length and count assertions on the list move with them |
| height classes, tuning, goldens | **unchanged** | no physics value moves; the DP fixture hashes above are asserted unchanged |

### G.6 New code surfaces (for REST-WIRE)

- `src/f01_rest.rs` (new), modelled on `src/f01_ring.rs`: `RestRoom { Refuge, Camp, Aftermath }` with `scene_name`, `room_id`, paths, pinned hashes and pin keys; `launch` (load, re-pin check, story pin check: the refuge's pin must **not** be a table row, the camp's and aftermath's must be room pins), `combat_for`, `story_setup` (refuge: `donut_prog::story_setup(carl, donut, SceneKind::RestRefuge)`, the corral's pattern; camp and aftermath: `f01_boiler::story_setup(pin, carl, donut)`, the Ring rooms' pattern, which arms nothing). It also holds the **decor tables**: the greybox decor rects marked *(decor)* in G.3 (the box, the marker, the survivor figures, the bag when it is not a static, the chalk marks), as presentation-only data the binary draws as flat labelled rectangles; they are never solids, never in the sim and never hashed (the sidecar has no decor kind).
- `src/f01_late.rs`: `RoomScene::{RestRefuge, RestCamp, RestAftermath}` (serde `f01_rest_*`), `rest_start` (the shared constructor of the binary and the verifier), the `cf02_rules` check, the scoped pin keys.
- `src/encounter.rs`: `LockEvent::HeroOnSurface { hero: u8, surface: usize }`; the raw `opens_on` gains an optional `surface`; validation: `target` is `carl` or `donut`, the surface exists, the target's strongest class reaches it with every lock closed, the surface is not inside a protected zone; the lock's exit check stays "every lock open".
- `src/combat_scene/env.rs`: a latched lock solid for `HeroOnSurface` (opened on the grounded tick, kept on checkpoint respawn, reset on restart, hashed only when present).
- `src/story_unlock.rs`: `SceneKind::RestRefuge`. `src/hud.rs`: `LabelId::SwapToDonut` (presentation). `src/bin/floor01_scene.rs` and the PLAYTRACE replay/verify dispatch: the three scene arms, following every `RingSplit` site.
- `src/f01_run.rs`: **no logic change**. `carry_for` already returns each REST room's snapshot (each has a story runtime: the refuge arms a trigger, the others hold a pin).

### G.7 Greybox now, art later

The greybox uses the existing Floor 01 kit and flat labelled rectangles. Every REST room is listed as milestone `greybox` and its HUD title is the manifest title. Art lands in separate work orders, through the art pipeline (top-tier design, medium execution, independent top-tier review), and never changes collision:

- **AREA-FASTFOOD-01** (already a row of the [area kits plan](../07-agents/floor-01-area-kits-plan.md)). Design inputs: atlas area 04 and its concept PNG (mood only); the REST-A rects above (counter band behind x 224..1104, booths, the play structure over platforms 0 and 1, the box at x 1200..1240, the staff shelf as a pass-through counter or service hatch at 128 px, the staff door 16 × 228 with a closed and an open state); the hot→cold threshold over x 0..224 (pair with AREA-HEAT-01's scorched edge); windows showing raw rock; the rolling-ball gag as an optional review item (REST-Q5); a palette that reads blue/white and clean against the brown catacomb. No logo, menu text or brand. Donut's paw-switch frames from ANIM-INTERACT-01 are a candidate for the shelf moment once wired (presentation only).
- **AREA-CAMP-01** (new art row, REST-AD15): the camp interior. Design inputs: atlas row 08 (`08-circular-maze-survivor-camp.png`, mood only); AREA-RINGS-01's `floor01_ring_camp` family (canvas, crates, the lantern, `ring_marker_a` at the entrance, pixel-identical lantern); the REST-B rects above; anonymous figures first, native survivor sprites only after REST-Q2; the crate-repositioning beat as an animation candidate; chalk and strength motifs toward the exit.
- **REST-C dressing** reuses AREA-GYM-01 (mirrors, weights, the bag) and AREA-ARENA-01 (the grooves) once those kits exist; no separate order.
- **Audio:** no REST audio in the greybox. The refuge jingle, hum and camp ambience are a later original-audio order (REST-Q3).

### G.8 Adopted defaults (REST-AD) and open questions (REST-Q)

| ID | Adopted default | Why |
| --- | --- | --- |
| REST-AD1 | Room ids `rest_refuge`, `rest_camp`, `rest_aftermath`; scenes `f01_rest_*`; titles "The Wrong Restaurant", "The Survivors' Camp", "After the Juicer". "Meadow Lark" stays in design text, not in data | IP-neutral data; short titles fit the location strip |
| REST-AD2 | Run order of section G.2; slots S0 to S4 are documentation only | #187's pacing map; KR15; #192 keeps `mastery` until its own integration |
| REST-AD3 | REST-A arms the existing row through a new `SceneKind::RestRefuge`; the sidecar pin `f01_pin_refuge_entry` names no table row; `f01_pin_pre_pile_boss` unchanged | KC9 swap; the corral and Hoarder precedents; a pin row cannot be empty |
| REST-AD4 | The reward zone keeps x 1200..1240 and the room is built around it; DP fixtures arm the row by id | byte-identical DP hashes, no stub trigger |
| REST-AD5 | Option A1 without Interact: standing on the 128 px shelf opens the staff door (`hero_on_surface`), latched | DP-Q18 exit gate; no new button (horizon) |
| REST-AD6 | The door is a 16 px solid from the walking line to the 228 px slab | no jump-over from the shelf |
| REST-AD7 | Solo hint `SWAP TO DONUT` after 90 ticks within 96 px of the closed door, Carl active; none in 2P | "no AI precision gate"; 2P players talk |
| REST-AD8 | Checkpoints keep the existing policy; no heal or refill | #187 "no healing invention" |
| REST-AD9 | REST-B reuses `f01_pin_post_pile_boss`; REST-C reuses `f01_late_mastery` | no new pin rows; KR12 holds |
| REST-AD10 | Survivors and sentry are static labelled markers; no NPC AI, no dialogue | #187 F step 3 |
| REST-AD11 | REST-WIRE lands as two PRs at milestone `greybox`: PR 1 the refuge (listed in the manifest), the lock, the grant move and the DP retarget; PR 2 the camp, the aftermath and the twelve-room run walk | the riskiest work first; the review sized one PR as too large for the playtest deadline |
| REST-AD12 | The heavy bag is a non-solid `hit_target` static if one works without a machine; otherwise decor | no new combat system |
| REST-AD13 | A REST room has an empty roster under `reach_exit`; if the validator rejects that, it is allowed for `reach_exit` only, with a test | quiet rooms have no enemies |
| REST-AD14 | AUDIENCE-RUN runs unchanged in REST rooms: no kills, so no awards; decay runs as tuned (after 480 steps without an award it removes 23‰ of the excess over the 250 baseline every 60 steps, so a 75 to 120 s crossing keeps roughly 21 % down to 7 % of the excess); the floor stays the baseline and the peak counter is kept. The refuge is the first live room and starts at the baseline, so it loses nothing; the aftermath drains the Juicer peak before the mastery room | no special case; REST-Q10 asks the owner |
| REST-AD15 | The camp interior art is a new row AREA-CAMP-01, reusing the AREA-RINGS-01 camp family | separate art order |
| REST-AD16 | The box is non-collidable decor; the grant column covers any height at the box | "finding matches the event" |
| REST-AD17 | No door notice; the door's disappearance and the existing `LIGHT FEET` notice carry the beat | restrained HUD |
| REST-AD18 | The legacy `--donut-prog` corridor stops granting the boost | one row, one place (KC9, R3) |

| ID | Open question | Owner of the answer | Default until answered |
| --- | --- | --- | --- |
| REST-Q1 | Does A1 feel like a 10-second upgrade reveal or a roadblock? | owner, game feel | A1 on; A2 is a one-line sidecar edit (remove the lock) |
| REST-Q2 | Which survivors appear, with which names, lines and sprites? | owner and narrative review | anonymous markers |
| REST-Q3 | Refuge jingle, fluorescent hum, camp ambience | later original-audio order | silence beyond the existing ambience |
| REST-Q4 | Should a REST checkpoint restore health or mana? | owner, game feel | no (REST-AD8) |
| REST-Q5 | The rolling yellow ball gag | AREA-FASTFOOD-01 review | not in the greybox |
| REST-Q6 | What "the gauntlet interrupts more strongly" means on the bag | combat design | REST-AD12 |
| REST-Q7 | Does the Ball of Swine replace or follow `mastery`, and where does REST-C exit then? | #192 integration | REST-C → `mastery` |
| REST-Q8 | Slot order once #179, #189 R-regions and #191 land | their integration steps | slots S0 to S4 |
| REST-Q9 | Should 2P also get a door hint? | owner, after the playtest | no |
| REST-Q10 | Should audience decay freeze in rooms with an empty roster (the REST rooms), so the aftermath keeps the Juicer peak? | owner, game feel | decay runs (REST-AD14); data-ready, not implemented |

### G.9 Tests and fixtures (names and expected results)

Library and data tests:

| Test | Expected result |
| --- | --- |
| `story_unlock::tests::the_safe_room_trigger_is_armed_in_the_refuge_only` (replaces the `armed_in == [Corridor]` assertion) | `armed_in == [RestRefuge]`; zone and grants unchanged |
| `story_unlock::tests::the_data_file_is_pinned_and_valid` (existing) | passes with the new `TABLE_HASH` |
| `donut_prog::tests::no_flag_is_no_story_and_each_flag_maps_to_its_setup` (updated) | the corridor setup arms nothing; the `RestRefuge` setup holds nothing and arms `f01_safe_room_reward_found` |
| the DP fixture hash and assertion tests (`donut_prog_story_unlocks`, `donut_prog_grant_survives_retry`, `donut_prog_primary_mapping`, `donut_prog_grants_2p`, the closure test, the `PuddleAndZone` blink fixtures), rebuilt on `SceneKind::RestRefuge` | **unchanged state hashes and events** (grant at tick 246 for `donut_prog_story_unlocks`, receipt source `f01_safe_room_reward_found`) |
| `f01_late::tests::*manifest*` and `check_story_order` tests (updated indices) | new `MANIFEST_HASH`; KR12 passes; `rest_refuge` is a room row carrying the boost grant |
| `f01_late::tests::the_rest_rooms_are_listed_in_story_order` | the twelve ids of G.2 in order |
| `encounter::tests::a_hero_on_surface_lock_parses_and_validates` | parses; rejects an unknown hero, a bad surface index, a surface the hero cannot reach, a lock on a protected zone |
| `f01_rest::tests::rest_refuge_starts_with_the_biscuit` | Donut's mana is unlocked at tick 0 in all three rooms (the boost's `primary_ranged` guard) |
| `f01_rest::tests::rest_refuge_entry_line_reports_the_boost` | `boost=armed` under Story, `boost=held` under All, `boost=none` flagless |
| `f01_rest::tests::rest_refuge_flagless_smoke_is_sealed` | the flagless smoke walks right and ends with the exit **not** reached and the door closed |
| `f01_rest::tests::rest_refuge_story_smoke_reaches_the_exit` | under the Story flags a scripted crossing with the shelf jump reaches the exit |
| `f01_rest::tests::rest_refuge_sidecar_validates` | all gates hold; platform 2's effective class is `donut_boosted` (no bypass); vantage `lane_ceiling` accepted; `ceiling_px` 228 |
| `f01_rest::tests::rest_refuge_shelf_is_boosted_only` | the physics replay (`encounter_physics`) lands boosted Donut on platform 2; base Donut and Carl do not |
| `f01_rest::tests::rest_refuge_staff_door_seals_the_room` | with the lock closed the exit is unreachable for every class and 2P; with it open, reachable for all |
| `f01_rest::tests::rest_refuge_reward_zone_sits_at_the_box` | the table zone's x range equals the box decor's x range, lies after the checkpoint and before platform 2 |
| `f01_rest::tests::rest_camp_and_aftermath_sidecars_validate` | valid; pins are room pins; no gated surface above 42 px; no lock |
| `f01_run::tests::the_corral_exit_relaunches_into_the_refuge` (retargets the corral→Hoarder test) | the corral's next room is `rest_refuge`, carrying the empty ledger |
| `f01_run::tests::the_refuge_exit_carries_light_on_your_feet_into_the_hoarder` | the snapshot holds `(1, jump_reach_boost)` with one receipt |
| `f01_run::tests::the_camp_and_aftermath_carry_their_ledger_through` | ring_split → camp → juicer and juicer → aftermath → mastery carry the snapshot unchanged |
| `f01_run::tests::the_aftermath_exit_relaunches_into_mastery` | as named; mastery ends the run |
| `hud::tests::the_swap_hint_shows_only_for_solo_carl_at_the_closed_door` | shown only under REST-AD7's conditions; cleared on open or swap |

Scenario fixtures (`src/rest_fixtures.rs`, deterministic, seeded, each also run twice for an identical hash):

| Fixture | Script | Expected result |
| --- | --- | --- |
| `rest_refuge_solo_carl_swap` | companion pinned off (`--companion off`, the follow-and-jump bot); Carl active runs right; Donut follows; Carl stops at the door; swap; Donut jumps onto the shelf; both walk out | one `CapabilityGranted { hero 1, jump_reach_boost }` when Donut's pivot enters x 1200..1240; the door opens on the shelf tick; exit reached; no damage event |
| `rest_refuge_solo_donut` | companion pinned off; Donut active from the start | the same grant; door open; AI Carl follows through; exit |
| `rest_refuge_2p` | P1 Carl, P2 Donut | the same; both heroes in the camera at the door |
| `rest_refuge_solo_companion_crossing` | the default autonomous companion (`cf02_free`); Carl active walks right to the exit | the exit is reached; exactly one grant; Donut is never left on the shelf when Carl exits (the goal or the follow AI brings her down); the hint may or may not show |
| `rest_refuge_grant_once_at_the_box` | Donut walks back and forth across the column | exactly one grant, no second receipt |
| `rest_refuge_base_donut_cannot_open` | a story setup with the trigger **not** armed; Donut jumps at the shelf repeatedly | no grant, the door stays closed, the exit is not reached |
| `rest_refuge_retry_keeps_the_grant` | grant, a defeat retry or checkpoint respawn, then a fixture restart and a second crossing | after the respawn: the boost is held, no second receipt (`rearm`), the door stays open; after the restart: no boost, the door closed, the trigger armed; the second crossing grants exactly once again and opens the door |
| `rest_camp_crossing_solo`, `rest_camp_crossing_2p` | run right with the post-Hoarder pin | exit within 1200 ticks; no grant; no damage; ledger equals the pin |
| `rest_aftermath_no_duplicate_unlock` | start from a carried post-Juicer snapshot (all four capabilities, receipts present) and cross | zero `CapabilityGranted`; the snapshot at exit equals the carry |
| `rest_aftermath_blink_lane` | Donut blinks across the lane | the blink lands (collision-safe), mana as the normal rules |
| `rest_aftermath_crossing_base_moves` | Carl and Donut walk only | exit reached; the bag and the lane are optional |

PLAYTRACE: `f01_rest_refuge_recording_verifies`, `f01_rest_camp_recording_verifies`, `f01_rest_aftermath_recording_verifies` (record a live-shaped session per room and verify it strictly, the `tests_f01_late.rs` pattern).

### G.10 Evidence list (REST-WIRE, `docs/evidence/rest-wire/`)

1. Captures at 640×360, 640×400 and 1920×1080 of: the refuge threshold (F-A0), the `LIGHT FEET` notice at the box, Donut on the shelf with the door open, the camp entrance with the marker and the markers, the aftermath blink lane.
2. A before/after strip of the 128 px shelf: base Donut failing (`rest_refuge_base_donut_cannot_open`), boosted Donut landing.
3. The run walk: the `entry_line` of each room of a full `--f01-run` from the corral to mastery (scripted or by the walk test output), showing the rooms in order (PR 1: ten rooms with the refuge; PR 2: all twelve), with the AUDIENCE-RUN viewer count at each REST room's entry and exit (REST-AD14, REST-Q10).
4. The PLAYTRACE verify lines of the three new rooms.
5. The full local check list results on the final head.
6. A real two-pad run: **NOT RUN** unless a human does it. The game-feel verdict is the owner's.

### G.11 Status

- **REST-WIRE status (PR 1, REST-A).** Landed as a **labelled playable greybox**: `rest_refuge` is listed right after the corral (run of ten rooms), with the `hero_on_surface` lock and its latched env solid, `SceneKind::RestRefuge` and the retargeted story row, the refuge room and sidecar, `src/f01_rest.rs`, the binary and PLAYTRACE arms, the `SWAP TO DONUT` hint and seven refuge fixtures. The decor (counter, booths, windows, reward box, shelf hatch) is flat rectangles; the door is a rectangle; the AREA-FASTFOOD-01 art is not started. The play deck and the slide top are 64 px and 80 px (not the 32 and 48 px of G.3): the validator's lane rule needs 52 px of clearance under a deck (REST-IMP-1). REST-IMP-7: the `cf02_free` companion did not open the door by herself in the fixture run, so `rest_refuge_solo_companion_crossing` scripts a human swap at the closed door and a swap back. The A2 fallback (REST-Q1: drop the lock in a one-line sidecar change) stays available if the swap beat plays badly. Camp and aftermath: PR 2.
- **REST-WIRE status (PR 2, REST-B and REST-C).** Landed as **labelled playable greybox**: `rest_camp` (after `ring_split`) and `rest_aftermath` (after the Juicer, before `mastery`) are listed, the run is **twelve rooms** (corral, rest_refuge, hoarder, boiler, dozer, hazards, ring_court, ring_split, rest_camp, juicer, rest_aftermath, mastery), on by default. Each has its room file, sidecar, story-order room row (pins `f01_pin_post_pile_boss` and `f01_late_mastery`, no grant), decor table (flat rectangles), binary and PLAYTRACE arms and fixtures; the camp and the aftermath grant nothing, the aftermath enters with the Juicer's ledger and grants nothing twice, and the audience counts and the story ledger carry through both. The AREA-CAMP-01 art, the gym and arena dressing and any REST audio are not started. Adopted defaults of the implementation: **REST-IMP-8:** neither room has a platform. The plan's 32 px crate stack (camp) and 24 px bench (aftermath) fail the validator's lane rule (a deck needs 52 px of clearance under it, i.e. 64 px of rise) and a 64 px deck would need a lane ceiling the rooms do not have (DP-Q45: no surface above 42 px without one), so crates and bench are decor rectangles. **REST-IMP-9:** the heavy bag is the sidecar's inert `hit_target` static (validated, no env behaviour, never solid) at {624, 516, 32, 64}: its bottom stands on the walking line as the validator requires (the plan's y 452 floats), mirrored by a `heavy_bag` decor rectangle. **REST-IMP-10:** `goal_x` is 1480 (camp) and 1224 (aftermath) so each room's derived exit is the planned {1456, ...} and {1200, ...} (IMP-2); the entry line of the camp and the aftermath reports `lock=none` and no `boost` field. **REST-Q6 stays open:** the heavy bag is inert (no solid, no reaction, no feedback) until a presentation-only hit response exists; the "usable" half of REST-C-02 is unmet and the bag is only a placed target. **REST-IMP-12:** a fixture restart in a room entered with a carried ledger restores the carried held set and clears the receipts (the ledger's restart rule for every room, not new here); a checkpoint retry keeps the whole ledger. Neither can grant anything in the quiet rooms. **REST-IMP-11:** `rest_aftermath_no_duplicate_unlock` unites the rooms' snapshots by hand (a carry replaces the ledger it enters) in run order: refuge boost, Hoarder dynamite, Juicer fist and blink, four receipts.
- This design PR: design only.
- After the two REST-WIRE PRs merge: REST-A, REST-B and REST-C are **"merged, game-feel review pending"**, milestone `greybox`. Their art (AREA-FASTFOOD-01, AREA-CAMP-01) is not started.

### G.12 Horizon, tone and rights

- **Horizon fit:** consulted [character progression horizon](character-progression-horizon.md). Light on Your Feet is Donut's first mobility growth between the biscuit (step 2) and the Juicer (step 6). The refuge lets the player *feel* it once, with no new button, and the solo swap makes the duo the point. REST-B consolidates known moves (step 5) and REST-C shows the Juicer's grants without granting again or adding a power before the final room (step 7). Solo and 2P take the same route. The guide still fits; it is not edited.
- **Tone:** [antihero tone](antihero-tone.md). The restaurant is a joke on the world, not on the heroes; the camp shows tired, practical people; nothing is heroic display. No speeches.
- **Rights:** no real brand, logo, menu text, music or book quote; the concept PNGs are mood references only; "Meadow Lark" and character names are the replaceable fan layer and stay out of data ids.
