# Floor 1 playtest packet (PLAYTEST-READY-01): the owner's single entry point

**What this is.** The one page to read before you play Floor 1 next week: how to start the game, the 18 rooms in the order the run walks them, what is missing and why, what to look at in each room, every default that was adopted for you to review, the open questions and the art notes collected by the reviewers. Nothing here is accepted. Every mechanic below is "merged, game-feel review pending" and every drawn room is "owner art review pending"; you decide what stays.

**Scope of this head.** Written against `main` after #313 (AREA-TROG-01 stage 2, `e9a05b1c`), the last PR before this one. The older packet [`docs/evidence/f01-playtest/README.md`](../evidence/f01-playtest/README.md) (PT-GATE-D) still holds the K1 to K19 defaults, the scorecards and the gate checklist, but its launch section predates the 17-room run: use this page for launching. The smoke and check outputs behind this packet are in [`docs/evidence/playtest-ready-01/`](../evidence/playtest-ready-01/README.md).

**The floor ends at the Ball of Swine arena (greybox), whose exit leads to the floor-end panel** (BOS-A, section 3). After `bos` the game shows the floor-end panel ("FLOOR 1 CLEARED", a facts-only total of the run) and quits when you confirm.

## 1. How to run

Release build, from the repository root, after `bash scripts/export_assets.sh`:

```text
# the whole floor in one session, 18 rooms, starts in act1_chase
cargo run --release --bin floor01_scene -- --scene f01_late --f01-run --scale 2

# one room on its own (ids in section 2); the story pin of that room sets the loadout
cargo run --release --bin floor01_scene -- --scene f01_late --f01-room trog_strength --scale 2

# start the run in the middle: every later room still follows
cargo run --release --bin floor01_scene -- --scene f01_late --f01-run --f01-room rest_camp --scale 2
```

**Flags the owner can use** (every flag and pilot below is exercised by [`scripts/check_f01_owner_commands.sh`](../../scripts/check_f01_owner_commands.sh), 30 ticks each under xvfb, debug build; the release build takes the same arguments):

| Flag | What it does |
| --- | --- |
| `--f01-run` | The floor in one session. A facts-only room-clear panel (ticks, deaths, restarts, grants, the next room) shows at every exit; Enter, South on any pad or Start on P1's pad goes on; the game relaunches itself into the next room and carries the story ledger and the audience counts. |
| `--f01-room ID` | Start in a room (with `--f01-run`, the run continues from it). |
| `--f01-loadout story` (default) / `all` | `all` is a labelled TEST LOADOUT: every capability from tick 0, nothing carried. Use it to see a late room without earning the grants. |
| `--companion off`, `cf02_assist`, `cf02_free`, `v0` | The AI partner. Room scenes default to `cf02_free`; `off` is the old follow-and-jump bot. |
| `--audience=off` / `--cine-showcase=off` | Switch off the viewer counts or the llama and dynamite slow motion that go live after the corral (AUDIENCE-RUN). |
| `--debug-combat` | Hitboxes and states on screen. |
| `--no-record` / `--no-audio` / `--scale N` | No PLAYTRACE recording, no audio, window scale. Recording is on by default. |

**2P and companion.** Plug a second pad and press its Start: P1 plays Carl and P2 plays Donut. Solo, L1/LB (keyboard Q) swaps heroes. With one pad the other hero is the autonomous partner. The controls table is in [`docs/evidence/f01-playtest/README.md`](../evidence/f01-playtest/README.md#1-launch-commands-plan-section-9) (keyboard: A/D, Space, J, K, L for the equipped special, S, Q, R, Esc). Real two-pad play has never been run.

**Evidence pilots** (scripted players for one room; refused together with `--f01-run`, because a floor run relaunches into the next room):

```text
... --scene f01_late --f01-room act1_chase --act1-pilot hide        # hide | skip | watch (chase), strike (shore), donut (guild), walk
... --scene f01_late --f01-room rest_refuge --rest-pilot donut      # donut | carl_door | base_jump (refuge), walk (any quiet room)
... --scene f01_late --f01-room rest_aftermath --rest-pilot punch   # punch | blink (aftermath): the heavy bag and the blink lane
... --scene f01_late --f01-room trog_run --trog-pilot carl          # carl | donut: plays the troglodyte run
... --scene f01_late --f01-room ring_court --ring-pilot             # plays the Ring room
... --scene f01_late --f01-room dozer --room-pilot dozer:run        # dozer:run | close_call | wait | brace
... --scene f01_late --f01-room hoarder --room-pilot hoarder:carl_solo:0
... --scene f01_late --f01-room corral --room-pilot corral_rush     # B's rush on a parked Carl
```

**Diagnostic binaries** (machine reports, not game feel): `cargo run --release --locked --bin corral_eval -- --out target/corral-eval` and `cargo run --release --locked --bin trog_eval -- --quick --out target/trog-eval`.

**Old recordings.** Several PRs moved the manifest hash. A local PLAYTRACE recording made before them fails strict verification (`content_mismatch`): re-record. No committed session is affected.

## 2. Run order: the 18 rooms

The HUD strip shows `ROOM n/17` in the panels and the room title with `TRY n`. The "legacy" column is the label older docs use ("room 4" for the hazards, and so on). Mechanic status is "merged, game-feel review pending" in every row; the art column says whether the room has a drawn kit (owner art review pending) or is still greybox (flat labelled shapes on the Floor 01 kit).

| # | Room id (legacy) | What it is | Art state |
| ---: | --- | --- | --- |
| 1 | `act1_chase` "The First Chase" (I-01/I-02) | The opening chase: a quiet lane, a rumble when the rearmost hero passes x 1072, the Murder Dozer emerges, cruises at 160 px/s, and charges only when it can be dodged (pocket, barricade lock, wedge at the end). ACT1-B #307. | dressed (AREA-ACT1-01 #317, owner art review pending): catacomb and ENV-02 kit, the gallery breach key, timbers, barricade, alcove niche; the DOZ-01 machine and the wedge are pixel art |
| 2 | `act1_shore` "The Rotten Shore" (I-03) | Carl's first revenge: two shore goblins (brawler, coward), the rotten shore, `defeat_roster`. Donut is locked: swat only. ACT1-A #306. | dressed (AREA-ACT1-01 #317, owner art review pending): ENV-02 shore, the wedged DOZ-01 rig |
| 3 | `act1_guild` "The Guild Door" (I-04) | Walk to the guide; Donut's pivot in the zone grants the biscuit (`MAGIC MISSILE` notice) and opens the door in the same tick. ACT1-A #306. | dressed (AREA-ACT1-01 #317, owner art review pending): the refuge kit hearth and furniture, the guild portal, the grant strongbox key; the guide is the existing `mordecai_h44` sprite (no talk animation, GUILD-BEAT-01) |
| 4 | `corral` "The Infernal Corral" (room 0) | Goblins, three llamas, the rush-breakable pen partition (CORRAL-D), the M3 grate that opens when both M2 llamas are down. CORRAL-C/WIRE/F/D #276, #280, #283, #298. | dressed: AREA-HEAT-01 #294, LLAMA-03 rush art #295; the M3 grate is a labelled greybox |
| 5 | `rest_refuge` "The Wrong Restaurant" | Quiet room. Donut on the 128 px shelf opens the staff door; the box grants Light on Your Feet (the boost). REST-A #302. | dressed: AREA-FASTFOOD-01 #305 |
| 6 | `hoarder` "The Hoarder" (room 1, boss) | Bank A, the entry gate, the lair mound, her 90-tick Reveal, swipe and shove, the J-B stack. Her defeat grants dynamite. | dressed: HOARD-02, AREA-HOARDER-01 (#258, #266), stir and "not yet" cues #296 |
| 7 | `boiler` "Goblin Boiler" (room 2) | Setpiece: steam valve, hanging weight, drum, the Engineer. | dressed: Goblin Workshop kit (#256, #264), landing dust #288, exit door #274 |
| 8 | `dozer` "The Machine Remembers" (room 3) | The revenge machine (DOZER-CORE #279, DR-A to DR-D #282, #287, #289, #292): escape the wheel, brace the frame at x 832, the crew. | dressed: DOZ-01 rig, DOZ-02 damage states #291; room shell greybox |
| 9 | `hazards` "Slimes and Rot Stickers" (room 4) | Three pockets: slime and acid, Rot Stickers, a crossover; nothing is reflex-evadable. | dressed: AREA-HAZARDS-01B #285 (teal acid, decals, splash), exit door #274 |
| 10 | `ring_court` "The Shutter Court" (4a) | Closed shutter over a step and a walkway; the AI partner follows over the walkway (RINGW-F1 #278). #271. | dressed: AREA-RINGS-01 #290, exit door #274 |
| 11 | `ring_split` "The Split Passage" (4b) | Branching passage, far camp visible through the breach. #271. | dressed: AREA-RINGS-01 #290, exit door #274 |
| 12 | `rest_camp` "The Survivors' Camp" | Quiet room: anonymous survivors, hearth, a heavy bag. REST-B #304. | dressed: AREA-CAMP-01 #308 |
| 13 | `trog_run` "The Troglodyte Run" | Pygmy, Basher and Virtuoso one after another; T1 lesson: punish the landing low; T3 platform. TROG-A #310. | dressed: AREA-TROG-01 stage 1 #311 |
| 14 | `trog_strength` "The Test of Strength" | An 80 px static bench between the Virtuoso and the Basher; the exit hands the ledger to the Juicer. TROG-B #312. | dressed: AREA-TROG-01 stage 2 #313 (bench, shelf, gym bay, shards, threshold, mats) |
| 15 | `juicer` "The Juicer" (room 5, boss) | The boss arena with the gym kit; dynamite, the gate, the grants (HEAVY FIST, PUDDLE JUMPER); an autonomous Donut fights too. | dressed: AREA-GYM-01 #275 (its own exit door, GYM-AD2), gate drop #288 |
| 16 | `rest_aftermath` "After the Juicer" | Quiet room: the wrecked gym, the heavy bag, a 112 px blink lane with no reward. REST-C #304. | dressed: AREA-CAMP-01 #308 |
| 17 | `mastery` "The Mastery Arena" (room 6) | Everything is held; wave 1 brawl lane, wave 2 llama yard; the exit gate opens when the roster is down. Leads into the Ball of Swine. | greybox; the exit gate is drawn (#274), the room itself is not dressed |
| 18 | `bos` "The Ball of Swine" (the finale, `--scene f01_ball`) | The locked 640 px arena with two abstract arches and a hero alcove behind each. Bait the roll into an arch from the alcove, then punish the crash; hit the chain-weight post in the left alcove three times to drop it on the stunned ball; a hero who hides in an alcove is sieged (the ball sniffs, then rolls at the arch); after a crash the ball walks back to the centre so the roll can be baited again (BOS-AD34); a hero who stays within its shove range gets the shove loop instead. The exit opens only on a real defeat. BOS-A (the plan: `docs/01-design/floor-01-bos-integration.md`, from the owner draft #192). | greybox: the ball is a labelled body, the arches are outlines, no new art |

## 3. Missing, by owner decision (budget order 2026-11-27)

The owner set the order on 2026-11-27 with 74 % of the weekly limit used: camp art, troglodytes, then this packet; **the rest comes after the weekly limit resets**:

1. **The rest of the Ball of Swine finale.** BOS-A is a labelled greybox: the Tusklings and the bot's alcove-flee (BOS-B), the SWINE-01 art, arch and chain-weight props and the stairwell behind the exit (BOS-ART), and the wide arena, central gate, reliefs and cinematics (BOS-C) are deferred (#192, the owner draft, stays unmerged as the record of the original text).
2. **Act I room art** (chase, shore, guild): done by AREA-ACT1-01 #317 (owner art review pending). Still missing: Mordecai's idle and talk animation (GUILD-BEAT-01) and the interact verbs (GUILD-VERB-01).
3. **Room polish:** corral (M3 grate, brace readability), dozer, hazards, mastery dressing (the arena is undressed).
4. **The V7 workshop wall** (backdrop of the Boiler) and **cleanup** (EVID-REFRESH-01, ART-HOLE-TRIAGE with the GOB-01 touch-up, LOS-UNIFY, ROOM-PILOT-RUN, DOZER-PERF-01, DOZER-NAV-01, HRD-D, HAZ-C/D).

Also not done, not on the budget list: the moving Ring shutter (RING-C wire; the court's shutter is static), audio for every new room (the game has no runtime audio path for them), named survivors and dialogue (REST-Q2), Interact (R1/RB is not implemented), the breakable bench (TROG-Q14), the optional Scatterers slot (K14).

**AREA-TROG-01 stage 2 (#313) is merged** (`e9a05b1c`, owner art review pending): the Test of Strength is dressed (the bench key as a static piece, shelf, gym bay, shards, threshold, mats). Review decisions: TROG-ART-AD-S7 removed the torch above the retreat shelf and AD-S8 accepted a 4 px seam. The weakened and broken bench keys and the debris are drawn but reserved for TROG-Q14, so the bench never breaks in play.

## 4. Playtest checklist

Two to four things per room. "Look" means judge with your eyes at 1x; "feel" means play it several times.

1. **`act1_chase`.** Feel: is the rumble at x 1072 enough warning and is the cruise fair (first run about 23 s at the pilot's pace)? Look: the alcove and its notice, and the barricade break after a defeat (kept on screen, ACT1-Q10). The room-clear panel between rooms 1 to 3 (ACT1-Q1): does it dull the beat?
2. **`act1_shore`.** Feel: the two shore goblins are now a fixed brawler and coward (not the seeded bag); a swat-only Donut with no missile yet. Look: the wedged machine is a labelled decor rectangle.
3. **`act1_guild`.** Feel: stepping into the zone grants the missile and opens the door at once; does the grant read? Look: the guide, a sprite on flat decor.
4. **`corral`.** Feel: the screen blocks Donut's missiles until B's rush or a hero breaks it (the scripted Donut-solo clear is about 340 ticks longer); the rush is never evadable by Donut's reflex. Look: the LLAMA-03 brace and rush at 1x (CORRAL-F OQ1). Answer the sheet's OQ3 (a level can be cleared by one stunlock) after a few runs.
5. **`rest_refuge`.** Feel: with Carl solo, does "swap to Donut and stand on the shelf" feel like a reveal or a roadblock (REST-Q1; the fallback is a one-line data edit)? Look: the box states (closed, grant, spent), the door lamp, `LIGHT FEET`.
6. **`hoarder`.** Feel: bank A needs a jump; the swipe and shove read before they land; the J-B stack drop is the payoff. Look: the mound stir before she wakes (subtle, 3 px straps) and the post "not yet" give during her Reveal.
7. **`boiler`.** Feel: valve, brace, drum; the Donut retune (she does not do the killing). Look: the landing dust, the plume over the drum base (ST-OQ1), the plume washing out Carl in the lane.
8. **`dozer`.** Feel: run, hold at x 480 until the wheel is near, brace at x 832 (two jabs, one stick of dynamite or two missiles; it lands 37 ticks later); the release window of about 1.35 s. Look: the stop frame, riders and the damaged hull at 1x; the TRY counter after a respawn.
9. **`hazards`.** Look: teal acid, the entry splash (small), the ring tape, the dark Rot nest. Feel: the Rot Sticker's 24-tick warning, the blast on both heroes, acid bars walkers.
10. **`ring_court`.** Feel: walking right on the floor meets the shutter's face and turns you back to the step; the AI partner crosses the walkway (about 4 s of jumps). Look: the great arc and the ring, the shutter's read.
11. **`ring_split`.** Look: the far camp through the breach (survivors hard to see), the marker before the exit. Feel: the branches.
12. **`rest_camp`.** Look: the hearth fire against the old four torches, the sentry and the repair figure in the dark, the heavy bag's swing on entry and on a hit. Feel: is a quiet room welcome here?
13. **`trog_run`.** Feel: the Pygmy's leap (punish the landing low with Carl's stomp or Donut's pounce), the Basher's smash window, the Virtuoso's tongue and its dead zone. Look: the knuckle marks, the crater, the bell.
14. **`trog_strength`.** Feel: cross the 80 px bench (jump over), the Virtuoso is down before the Basher wakes at x 600; a stationary Donut can only hit the enemy on her side. Look: the bench, the shards and the threshold (see the art notes).
15. **`juicer`.** Feel: dynamite on the boss, the autonomous Donut in the fight (she leaves Carl the kill). Look: the `HEAVY FIST` then `PUDDLE JUMPER` notices (the second notice never showed in one capture, check it in the Juicer), the gate drop, the narrow 20 px exit door.
16. **`rest_aftermath`.** Look: the wrecked gym, the grooves, the bag (`--rest-pilot punch`). Feel: the 112 px blink lane has no reward: is it worth having?
17. **`mastery`.** Feel: the "powerful enemies" waves with everything held; clear time. The exit leads into the Ball of Swine.
18. **`bos`.** Feel: the 36-tick roll tell and the run to the alcove (a roll is dodged only by the alcove; Carl's jump does not clear it), the punish of the crash (about 4 jab hits in the 66-tick stun) and the ball walking back to the centre afterwards (a pilot baits about 8 rolls in a clear), the shove's tell and recovery, the chain weight (hit the post twice early, bait the roll into the left arch, land the third hit during the crash), and the siege: stay in an alcove and the ball sniffs (`IT SNIFFS AT THE ARCH`), then rolls at the arch for 2 damage to heroes in it; stepping out cancels it. **BOS-Q13 (open):** after a left crash, a hero who stays at pivot 116..140 (in shove range, outside the alcove) keeps the ball shoving instead of recentring; stepping back during the 30-tick tell and punishing the recovery can be repeated indefinitely at 1 HP per mistake. Judge whether this shove loop feels like a cheap farm or a fair alternative to baiting rolls; the cheap fix is a shove cap before a forced recentre. Look: the greybox ball with its label, the arch outlines, the exit opening after the defeat and the `FLOOR 1 CLEARED` panel with its facts-only total.

## 5. Adopted defaults to review

You said "go with all" and to end everything on `main`, so the open design choices were adopted by the recommendation of a top-tier design analysis and recorded by ID. **None of them is accepted.** Change a value in a follow-up PR; the last column says where. This section lists the ones most likely to be felt (75 rows); the complete list of 449 adopted-default rows and 139 question rows since `bebd5948`, with the PR that adopted each one, is in [the appendix](f01-playtest-ready-01-adopted-defaults.md).

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| **Run and HUD** | | | |
| F01RUN-AD3 | `--f01-run` takes no `--cf02-seed`; `--f01-carry` and `--f01-run-tally` need `--f01-run`; `--f01-carry` is refused under `--f01-loadout all` | #270 | `src/f01_run.rs`; `assets/levels/f01_late/manifest.json` (room order) |
| F01HUD-AD1 | The location strip shows the manifest room title and the attempt; the TEST LOADOUT tag shows only under `--f01-loadout all` | #270 | `src/combat_scene.rs` (`CombatState::hud_try`) and the HUD location strip |
| HT-AD1 | TRY n counts every attempt start: room entry, each checkpoint respawn, each restart (one expression, `CombatState::hud_try`) | #293 | `CombatState::hud_try` in `src/combat_scene.rs` (one expression) |
| **Act I (rooms 1 to 3)** | | | |
| ACT1-AD1 | Three rooms `act1_chase`, `act1_shore`, `act1_guild` before the corral, milestone `greybox`, `cf02_rules`, story-less (owner-gated, authorised by go-with-all) | #303 | `assets/levels/f01_late/manifest.json` (room rows) |
| ACT1-AD2 | The chase/shore boundary is ENV-02's post-chase checkpoint (route 2640); the standard room-clear panel shows between them | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD5 | Commit rule of section 5.3 (all heroes past `arm_x`, gap ≥ 140 from the rearmost body, reach margin 20 ticks, the charge reaches the barricade), one charge per attempt, ... | #303 | `assets/data/gameplay/enemy_dozer_opening_v0.json` (commit rule values) |
| ACT1-AD7 | No idle wake in the opening (216000 ticks) | #303 | `dzq11_idle_trigger_ticks` in `assets/levels/floor01/f01_act1_chase.encounter.json` |
| ACT1-AD9 | No riders on the opening machine in the greybox; the shore goblins stand in for the thrown riders (owner-gated, authorised by go-with-all) | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD13 | Donut locked in the Act I rooms under Story and flagless; `unlocked(8)` under All or the light-feet / puddle flags (owner-gated, authorised by go-with-all) | #303 | Donut's setup per loadout in `src/f01_act1.rs` |
| ACT1-AD16 | `stop_x` 2856 and the exit at 2928; the machine wedges at the S5 arch's right edge | #303 | `dzq14_stop_x` in `assets/levels/floor01/f01_act1_chase.encounter.json` |
| ACT1-IMP-1 | The sidecar rows carry `id`, `kind`, `x`, `beat` and `delay_ticks`. The module constant `f01_act1::SHORE_TEMPERAMENTS` (brawler, coward) is applied through the new ... | #306 | `SHORE_TEMPERAMENTS` in `src/f01_act1.rs` |
| ACT1-IMP-11 | F14 asserts that the `WheelHit` precedes any `BarrierImpact`, no `PinnedHold`, no tell, and that the respawn rebuilds the machine (dormant, `barrier_hit()` false, the ... | #307 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| **Audience and showcase (from the corral on)** | | | |
| AR-AD1 | The unlock beat is the corral exit (the refuge onward, room 4 of N, N = 17 now (15 when #297 merged): the run length is read from the manifest). (owner-gated, authorised by go-with-all) | #297 | positional rule in `src/f01_run.rs` (the `corral` manifest row) |
| AR-AD7 | `--cine-showcase=on/off`: default on iff the run default predicate. Explicit `on` is allowed on any path. `off` gives main's rules. (owner-gated, authorised by go-with-all) | #297 | `--cine-showcase=off` flag; defaults in `src/f01_run.rs` |
| AR-AD9 | `CINE-THROWN-MULTI`: score 5, base 3, `thrown_multi_min_kills = 3`, all of them on one blast's tick. | #297 | `thrown_multi_min_kills` in `CineTuning` (`src/cine.rs`) |
| AR-AD13 | `CINE-SHOWCASE-KILL`: score 5, base 1, a credited llama defeat on the completing tick. The anchor is the lowest llama id of the batch. | #297 | `showcase_kill_score` in `CineTuning` (`src/cine.rs`); 0 switches it off |
| AR-AD16 | The boss never: not credited, not a showcase target, never one of the 3 | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD25 | The Hoarder (`PILE_BOSS_ID`) is excluded like the Juicer: never a showcase target, never one of the 3 blast kills. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| **Corral (room 4)** | | | |
| CW-AD5 | The M3 gate is an ExitGate solid at 2048..2064 that stands until both M2 llamas are down; drawn as a labelled greybox grate | #280 | `assets/levels/floor01/f01_corral.encounter.json`; `src/f01_corral.rs` |
| CF-AD3 | An inert story room (the corral) carries the empty story ledger so the next room starts from its own pin | #283 | `assets/levels/f01_late/manifest.json`; `src/f01_run.rs` |
| CD-AD2 | A counted hero hit is one hit whatever the move's support-hit weight (the reinforced haymaker, weight 2, still counts one): "three distinct hits" | #298 | the `pen_partition` prop in `assets/levels/floor01/f01_corral.encounter.json` |
| CD-AD3 | The slowest shove that breaks it is 160 px/s (the other props' launch and shove threshold, BOILC-AD2); the rush is 240 px/s | #298 | the `pen_partition` prop in `assets/levels/floor01/f01_corral.encounter.json` |
| CD-AD6 | A launched body (a kicked goblin) and a blast do not break it | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |
| CD-AD10 | A checkpoint respawn keeps the partition's state (a broken or weakened screen stays so); only a restart (retry) restores it. | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |
| LL3-AD7 | No in-game lane band | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |
| LL3-AD9 | The rush art is on by default in `--scene f01_late --f01-run` and in the corral room | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |
| **Refuge (room 5)** | | | |
| REST-AD5 | Option A1 without Interact: standing on the 128 px shelf opens the staff door (`hero_on_surface`), latched (owner-gated, authorised by go-with-all) | #301 | the lock in `assets/levels/floor01/f01_rest_refuge.encounter.json`; `armed_in` in `assets/data/gameplay/story_unlocks_v0.json` |
| REST-AD7 | Solo hint `SWAP TO DONUT` after 90 ticks within 96 px of the closed door, Carl active; none in 2P | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD10 | Survivors and sentry are static labelled markers; no NPC AI, no dialogue | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD12 | The heavy bag is a non-solid `hit_target` static if one works without a machine; otherwise decor | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD14 | AUDIENCE-RUN runs unchanged in REST rooms: no kills, so no awards; decay runs as tuned (after 480 steps without an award it removes 23‰ of the excess over the 250 ... | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD17 | No door notice; the door's disappearance and the existing `LIGHT FEET` notice carry the beat | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD18 | The legacy `--donut-prog` corridor stops granting the boost (owner-gated, authorised by go-with-all) | #301 | `assets/data/gameplay/story_unlocks_v0.json` |
| FF-AD7 | Six steady cool strip lights replace the four dining-room torches; the threshold and vestibule torches stay | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD13 | One static stray ball, no rolling gag in this work order | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| **Hoarder (room 6)** | | | |
| HC-AD2 | Two levels: S1 period 72 (heave 6, slump 4), S2 period 36 (heave_hi 8, slump 4) | #296 | constants in `src/hoarder_art.rs` (`MOUND_STIR_HEAVE`, `STIR_HOLD_PX`); `hrdq05_stir_x` in `assets/data/gameplay/enemy_pile_boss_v0.json` |
| HC-AD8 | No reactive cue on a refused post hit; bolts keep passing the post | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |
| HC-AD11 | Hysteresis: once S1 is active it holds until the lead hero is under x 224 (16 px under `hrdq05_stir_x`); | #296 | constants in `src/hoarder_art.rs` (`MOUND_STIR_HEAVE`, `STIR_HOLD_PX`); `hrdq05_stir_x` in `assets/data/gameplay/enemy_pile_boss_v0.json` |
| **Dozer (room 8)** | | | |
| DZ-AD1 | Placement: room 3 of the slice, between the Boiler and the Floor Lies (the Ring rooms 4a/4b follow the Floor Lies), with pin `f01_pin_post_pile_boss` (dynamite held) and ... (owner-gated, authorised by go-with-all) | #273 | `assets/levels/f01_late/manifest.json` (room order) |
| DZ-AD2 | K5 amendment: no `BossThreat` supplier in a Dozer room; the AI rule is the three layers of section 7, with CA2-DZ pre-registered as the named CA2 append and triggered by ... | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD3 | No charge in room 3; the charge sub-machine is CHASE-01's append | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD10 | No HP to lose (a fixed `hp()`, `alive()` always true), no health bar, no `EnemyDefeated`; the disable is a `CombatEvent::Dozer` sub-event; | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD19 | One checkpoint (the entry); any party defeat retries from R0. The room is short, and the doc asks for fast retry | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD21 | The rescue lands beyond the machine's reach, standing on the floor through `SlotOut.place` (section 7.3): an in-view lane spot ahead of the lead zone when it is safe, ... | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-IMPL-AD14 | DV-8 'every hero reaches the exit' is read as room completion, frozen when the first hero overlaps the exit with every hero alive and ahead of the wheel | #279 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DRD-AD1 | A non-blast clang sparks on the hull face nearest the hero nearest the hull, 24 px above the floor | #292 | `src/dozer_art.rs` (presentation only) |
| **Hazards (room 9)** | | | |
| HZB-AD1 | The acid becomes the teal ramp of section 5.1, a 1:1 remap of the ten `dfx_acid` entries. The slime keeps its green. This supersedes the acid half of DFX-AD1 | #285 | `scripts/author_danger_fx.py` (acid ramp) |
| HZB-AD10 | The acid entry splash (`dfx_acid_splash`, 5 frames, 16 ticks) is a new layer above the actors and below the rings | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| **Broken Ring (rooms 10 and 11)** | | | |
| RINGW-AD1 | Both Ring rooms are listed in the manifest at milestone greybox, between hazards and the Juicer | #271 | `assets/levels/f01_late/manifest.json`; `src/f01_ring.rs`; `src/combat_scene/env.rs` |
| RWF1-AD1 | Escort when she is cut off from Carl by a barrier on his level (knob `fix_all.rwf1_cut_off_escort`, on in `cf02_free` only) | #278 | `assets/data/gameplay/companion_ai_v0.json` (`fix_all.rwf1_cut_off_escort`) |
| RNG-AD6 | In-scene, the court's shutter is static closed. The moving states are drawn and unit-tested now, and their in-scene capture follows RING-C wire. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD17 | The full ring's outer diameter is 204 px (r 102; the piece is 208 x 208 and its last four rows are empty), under #189's 250..400 px apparent diameter. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| **Survivors' camp and aftermath (rooms 12 and 16)** | | | |
| CAMP-AD3 | Survivors are four anonymous, faceless figures (sentry, repair, seated, map), at most Carl's height and darker than the heroes (owner-gated, authorised by go-with-all) | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD4 | The hearth replaces the greybox lantern as the central light, on the solid stretch 672..855 over the checkpoint; four torches and their pools are removed | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD7 | REST-Q6 (presentation half): the bag swings once on entry and on each hit's rising edge, with the 38-tick table and a rewind guard; the sim stays inert | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD13 | No audio | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| **Troglodytes (rooms 13 and 14)** | | | |
| TROG-AD1 | Two rooms `trog_run` and `trog_strength` between `rest_camp` and `juicer`; greybox, `cf02_rules`, story pin `f01_pin_post_pile_boss` (owner-gated, authorised by go-with-all) | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD3 | The numbers of 5.1 (HP 2/4/3, damage 1, rests 40/30/36, knock values, leap 64..160, apex 16, standoff 66) | #309 | `assets/data/gameplay/enemy_trog_v0.json` |
| TROG-AD10 | Interrupts keyed by `HitSource` (5.2 table): bolts never interrupt the Basher or the Virtuoso; the Basher has an 18-tick window and a 120-tick budget | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD13 | The Test of Strength has Basher + Virtuoso only; no Pygmy reinforcement | #309 | `assets/levels/floor01/f01_trog_strength.encounter.json` (roster) |
| TROG-AD17 | Target choice (amended, review D3): the nearest living hero in x, outside `protected_spawn`, whose hurtbox can meet the species' attack band in y; tie to Donut; | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD21 | Wake causes `Proximity` and `Hit`, recorded in the `Wake` event and the state hash. A hit, blast or contact on a Dormant trog deals its damage and sends it to Rest (no ... | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD22 | The T1 lesson is "punish the landing low": Carl's CF02 stomp (box y 0..10, startup 9, inside the 14-tick land) or Donut's pounce (y 0..17), then a jab in Rest against ... | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD36 | `EvadeSource::TrogBlow` is appended to the DM-B allow-list (five sources), so AD9 is real: Donut's reflex may evade any troglodyte blow, key `(slot target id, tell start ... | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD43 | The declared static fallback is taken (AD12): the validator rejects the 80 px `bench_stack` partition (a `solid_intact` prop is at most 48 px), so the bench is a static ... | #312 | `MAX_STATIC_SOLID_PX` in `src/encounter.rs`; superseded by TROG-AD46 |
| TROG-AD46 | The bench is an 80 px static `solid` (640..672, y 500; the shelf stays 48 px at 288..352). | #312 | `MAX_STATIC_SOLID_PX` in `src/encounter.rs`; `assets/levels/floor01/f01_trog_strength.encounter.json` |
| **Engineer, death, HUD, gauntlet, sim ticks** | | | |
| ENG2-AD1 | ENG-G-Q24 (the owner's game-feel verdict on the Engineer before any art) is waived. (owner-gated, authorised by go-with-all) | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD4 | The ram run is taken: frames 20 and 21, tag `attack_ram_run`, cycle 12, 20, 21, 20 at 4 ticks. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD6 | No pot is drawn in any slammed cel. The pot and the wrench are launched as parts at the slam, in all three poses. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| HA2-AD1 | Donut's mana track is 72 px outer on every plate state (8 segments of 7 px, 2 px gaps); the freed span holds the special label, right-aligned like Carl's BOOM | #286 | `assets/manifests/hud_hybrid_a_v1.json`; `src/hud_draw.rs`; `scripts/author_hud_hybrid_a.py` |
| HA2-AD2 | The socket keeps the button glyph; the item chips (dynamite, puddle) sit in a new item well on the grant notice (deviation, fallback HA2-Q2) | #286 | `assets/manifests/hud_hybrid_a_v1.json`; `src/hud_draw.rs`; `scripts/author_hud_hybrid_a.py` |
| HA2-AD4b | Last-used device on P1 only: the keyboard set shows on the first keyboard input and returns to the pad family on the first pad input | #286 | `assets/manifests/hud_hybrid_a_v1.json`; `src/hud_draw.rs`; `scripts/author_hud_hybrid_a.py` |
| CG-AD1 | Overlay, not redrawn frames. The gauntlet is a separate, full-cell overlay atlas per body sheet (same 48 x 56 cells, same frame order, same tags, same pivot), drawn on ... | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| ST-AD1 | The two ticks are unhashed observer fields (`landed_tick`, `gate_closed_tick`), excluded from the state hash like `Boss::charge_started` | #288 | `docs/02-art/f01-sim-ticks-spec.md`; observer fields in `src/` (unhashed) |
| EXIT-AD1 | One new doorway sheet instead of stretching the 28 px room 5 grate or the 16 px Hoarder door | #274 | `scripts/author_danger_fx.py`; `src/room_exit_art.rs` |

**Owner drafts that are not merged.** #179, #187, #189, #191 and #192 stay open as drafts by the owner's rule; their content reached `main` through these PRs:

| Draft | Content now on `main` | Still missing |
| --- | --- | --- |
| #179 Dozer opening choreography | #303 (the ACT1 plan, which carries it) with #306 and #307 (rooms 1 to 3), and the DOZER-CORE chain #273, #279, #282, #287, #289, #291, #292 | riders on the opening machine (ACT1-Q6); audio (ACT1-Q2); the Act I room art is done by #317 |
| #187 Rest spaces and transition beats | #301 (REST plan), #302 (refuge), #304 (camp, aftermath), #305 (fast-food kit), #308 (camp kit) | named survivors (REST-Q2), Interact, audio, the Scatterers slot |
| #189 The Broken Ring | #271 (rooms wired), #277 (blast seam fix), #278 (partner crossing), #290 (kit; divergences recorded as RNG-AD1 to AD24) | the moving shutter (RING-C wire), further Ring regions |
| #191 Troglodytes and the Test of Strength | #309 (plan), #310 (TROG-A), #311 (art stage 1), #312 (TROG-B), #313 (art stage 2) | the breakable bench and its break art (TROG-Q14), audio, death profiles |
| #192 Ball of Swine | BOS-A greybox (room 18, the exit and the floor-end panel) | Tusklings, art, wide arena and cinematics deferred (section 3) |

## 6. Open questions and the adopted recommendation

Each question is non-blocking; the right-hand side is what is in the game now.

| ID | Question | Adopted |
| --- | --- | --- |
| ACT1-Q1 | Skip the room-clear panel between the Act I rooms? | Standard panel stays |
| ACT1-Q10 | Show the barricade break after a defeat, or hold on the downed hero? | The break is shown |
| ACT1-Q2, Q3, Q6, Q7 | Chase audio, end-on wedge art, riders, ENV-02 dressing | Q7 (ENV-02 dressing): done by #317; Q2, Q3, Q6: none |
| ACT1-Q9 | The #179 first-run duration hypothesis (25 to 40 s) | Values as built, judged after a human session |
| REST-Q1 | Does the shelf-opens-door beat read as a reveal or a roadblock? | Option A1 kept; A2 (drop the lock) is a one-line sidecar edit |
| REST-Q2 | Which survivors, names, lines? | Four anonymous, faceless figures |
| REST-Q6 | What does the heavy bag do? | Inert in the sim; a cosmetic swing only (CAMP-AD7) |
| REST-Q10 | Should audience decay freeze in empty quiet rooms? | No change: decay runs |
| CAMP-Q1 to Q6 | Survivors, crate animation, idles, bag combat half, audio, fallen tube | Anonymous, none, none, inert, none, no tube |
| FF-Q6 | Lamp pair of the refuge door (olive open lamp, colour-blind weak) | Keys frozen; the panel carries the state |
| FF-Q1 to Q5 | Palette ramp, cool strips, box glow, rolling ball, paw frames | No, yes, no, static, no |
| AR-Q1 to Q9 | Audience beat, llama frequency, punish bonus, chained credit, hazard credit, threshold, party defeat, persistence, reveal card | Corral exit in `f01_late` only; every clean llama kill; no bonus; uncredited; no; 3 kills cap +2; no cost; process lifetime; panel line only |
| CORRAL-F OQ1 to OQ5 | Brace read, evasion of the rush, per-level stunlock, and two more | Open: recorded in [the corral-f README](../evidence/floor01-corral/corral-f/README.md) |
| HEAT-OQ5 | Is the pen's density enough ("the corral fights back")? | The caps as built |
| LL3-OQ1 | Range-end readability of the rush without a lane band | No band; the muzzle pokes up to 17 px past the hitbox |
| DZ-OQ1, OQ2 | Narrative placement of the revenge beat; same machine or same model | In the slice as room 8; "another Dozer of the same family" |
| ST-OQ1 | The Boiler plume over the drum base | Art unchanged |
| RNG-OQ5 | Is one full ring and two arcs enough "awe"? | As built |
| HA2-Q2 | Item chip on the grant notice, glyph in the socket? | Kept (the fallback is not recommended) |
| TROG-Q3 | Is a stationary clear from the bait zone acceptable? | Yes, reported not gated |
| TROG-Q4 | Does dynamite break the bench? | No |
| TROG-Q13 | A grounded Donut cannot be hit by the Virtuoso: is that "Carl's problem"? | Yes: the Virtuoso is Carl's problem; a grounded Donut bolts it freely in co-op |
| TROG-Q14 | Let a partition be 80 px, restoring the breakable bench (rule b, BenchSmash)? | Deferred; the bench is an 80 px static (TROG-AD46), so it never breaks in play and the break art is reserved |
| TROG-ART-Q6 | The troglodytes (L* 25 to 33) are darker than Carl (about 41) | Not fixed in the kit: a value pass in the troglodyte family's own review |
| R2-OQ1 to OQ6, KC3 | Older questions from the Boiler retune and the corral (packet PT-GATE-D section 4) | Still open |

## 7. Owner art notes, by room

These come from the independent art-director reviews and the executors' risk notes. They are looks, not defects the reviewers could fix; each is yours to confirm or drop. Nothing is accepted.

- **Act I rooms (1 to 3), AREA-ACT1-01 #317.** The S3 timber set can read as a gallows. The S4 alcove niche reads by its shape and its cold colour, and it is not the brightest shape. The guild grant box is dark beside the desk, and the hearth is brighter than Mordecai's face (ACT1-ART-AD22 relaxed these lit-read targets). The open bays keep the blue sill (ACT1-ART-AD19). On the shore, a torch flame sits close to the leaning beam. Mordecai has no idle or talk animation yet (GUILD-BEAT-01).
- **Corral (4).** The scorch plume is subtle (HEAT-OQ5); the bumper reads as a crate; the llama's muzzle is drawn 17 px past its hitbox, the limp ear of the recover frames reads as a collar, and the rush silhouette reads as a hyena (accepted). The stand-in brace is faint at 1x (LLAMA-03 replaced it).
- **Refuge, fast food (5).** The brass reads olive and the gilded box drifts to olive under the scene ambient; the checker floor is on the back half only; the booth is blue on blue; the ball pit reads as a dark tray with olive specks; the hatch reads like a dark monitor; the rock blobs read a little like holes; the lamp pair is weak for colour-blind players (FF-Q6).
- **Hoarder (6).** The stir is subtle in a still; the shored straps are 3 px bands; in solo play only the strongest heave shows; heap b's rim light and the boxed-tray rubble cap are yours to judge; the warning may want a creak.
- **Boiler (7).** The denser plume washes out Carl's head and torso (alpha 0x80 is an easy fix); the torch stands under the weight; the plume merges with the drum band (ST-OQ1).
- **Dozer (8).** The Guild door arch overlaps the tunnel lintel by about 24 px (DZ2-Q4); a wall torch sits behind the disabled tub and reads as fire on the wreck; hull cracks and the tub crumple are faint at 1x; the landed-hit remnant is about 70 % hidden under the wheel; the 32 px gate tile repeats as a ladder; the splinter burst is weak over warm brick; riders A and B wear no helmet.
- **Hazards (9).** The splash crown is small; the Rot nest is a dark blotch, nearly invisible on dark brick; the tritan read of the new acid is the weakest (dE 10.4); the second sticker's scuff runs over open brick (the drawn pillar is narrower than its solid).
- **Broken Ring (10, 11).** The survivors in the breach are hard to see at 1x; the walkway's top edge is only just the brightest run; a torch hangs about 64 px above its neighbours; the ring's break ends hint at a spiral when enlarged; the shutter may still read door-like (lower the iron band to the shoe); the full ring risks reading as an arch.
- **Camp (12) and aftermath (16).** The bag's entry swing plays off-screen and the sway is subtle (plus or minus 4 px); the grooves read as cables; the wreck glass reads as mesh or barbed wire; the repair figure is the weakest read; the plate tree reads as a speaker stack; one fire against four torches (does it read warm enough?).
- **Troglodyte run (13).** The knuckle marks read as bats (a redraw is suggested); the crater reads as a hole rather than a dent; the Virtuoso is the weakest read because the troglodyte sprites (L* 25 to 33) are darker than Carl (about 41; Q6); the fist hole's loose brick reads as a second socket; the blue sill under the bricked bays is a kit-wide question.
- **Test of Strength (14), from the #313 review** (AD-S7 removed the shelf torch; AD-S8 accepted a 4 px seam).
  - **Bench:** it plays well (solid and dark with a lit top) but still reads as a cabinet or a doorway with a porthole and steps; the plate stack looks like walkable stairs.
  - **Mirror shards:** they read even more as a framed night-mountain painting.
  - **Threshold:** the stone-to-gym-tile story is weak; it shows as a small blue patch.
  - **Mats:** they read as a low dais under the Basher.
  - **Q6:** the Virtuoso still reads by its edge only (delta L* -0.3).
  - **Cold tube:** check its bloom in motion.
  - **Break art:** reserved for TROG-Q14, so the bench never breaks in play yet (TROG-Q14 is deferred, the bench is an 80 px static, TROG-AD46).
- **Juicer and gym (15).** The room reads busier; the exit door is narrow (20 px) and easy to miss after the defeat; the `mirror_crack` sprite reads as a spark more than a spiderweb; the open gym door's inner edge is a touch dark; check `PUDDLE JUMPER` in the Juicer (the notice never showed in one capture because the scene tick stops after the boss falls).
- **Characters and HUD.** Carl's reinforced fists are 1 to 3 px at run size and read as a dark mitt (the wind-up is the weakest read); Donut's `blink_dip` is built from capsule chains and the fizzle heads show up ears (DMA-Q22); the puddle chip is close to a plain pink bar at 1x; the Engineer's ram cycle reuses frame 20 for both passings and the forced `slammed_wall` clip plays in open floor; the llama blast throws its head behind Carl's head and the preset 4 wall spatter reads as a blood bush, and the HUD says `FLATTENED!` on a dynamite kill.

**Tone and rights.** All art is fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope, with the model that designed, executed and reviewed each piece recorded in its PR; none of it is owner-approved.

## 8. Known issues and follow-ups

None of these blocks the playtest.

- **R4 `exit_open` capture nondeterminism.** A 32x8 patch of an `exit_open` capture differs between runs (reported by the #312 review as R4); follow-up PR. Two wording nits (manifest "bench stack", room "exit apron at 944") ride with it.
- **EVID-REFRESH-01.** Old evidence is stale: the `deck-*` lines of `capture_lines_d.txt` no longer reproduce, several clips predate the room title and the exit door, the room 5 and `f01-danger-fx` stills predate the gate drop and the gym kit, older folders still show the green acid pool, and the `hoarder:carl_solo` scorecard row drifts. No gameplay depends on them.
- **HUD-TRY-01** is fixed (#293, HT-AD1, game-feel pending): `TRY n` counts every attempt start. The Juicer's boss strip still hides the location strip.
- **ART-HOLE-TRIAGE** (69 unallowed pockets on GOB-01 A/B, a top-tier call), **LOS-UNIFY** (one line-of-sight rule for stickers and blasts), **ROOM-PILOT-RUN** (a piloted `--f01-run` cannot cross a room boundary), **DOZER-PERF-01** (about +9 % tick mean from the stop frame, debug under xvfb), **DOZER-NAV-01**, **HRD-D**, **HAZ-C/D**, **DOZ-02-FU F4 to F7**, **HEAT-FU-1**, **LLAMA-03-FU** (the limp ear), **ENG-02** follow-ups (pot origin, ram cycle), and the owed re-reviews of #149, #154 and #150.
- **Edges.** A non-demo defeat retry would close a latched door while the story keeps the boost (unreachable in today's rooms); a companion's last hit drops a thrown-stick moment in co-op (#299 watch item); a human P2 who stays behind x 1072 in the chase never wakes the machine (ACT1-AD7, by choice); with the default companion the bot is rescued into the chase pocket (twice with Carl active).
- **Never run:** real two-pad play, real-display frame times and a release-build benchmark (all perf numbers are debug under xvfb), real-window recording with `tools/record-run.sh`, audio for any new room, and any human game-feel session. Nothing is claimed about Steam Deck, performance or legal clearance.
