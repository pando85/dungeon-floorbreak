# DEMO-01: integrated demo scene plan

**Status:** design plan (top tier), 2026-11-19. Nothing here is implemented yet. The owner wants to judge game feel as soon as possible in **one playable demo scene on `main`**, then answer the [DEMO-01 questionnaire](demo-01-questionnaire.md). This plan picks the fastest honest path: reuse what is merged, wire it into one continuous run of `floor01_scene`, add only the minimum new gameplay (a Bad Llama with the approved slam rule), and say plainly what the demo does not show.

**Owner decisions applied (2026-11-19):**

- **Llama slam rule approved.** A lethal heavy hit (`ForceTier::Heavy`, today Carl's lethal push kick) plays the LLAMA-02 slam pair (`slam_contact`, then `broken_heap`); a routine lethal hit (jab, haymaker) keeps the LLAMA-01 `down`. Recorded in the LLAMA-02-DEATH backlog row and in [the LLAMA-02 spec](../02-art/llama-02-death-spec.md).
- PRs #79, #94, #97, #106 and the camera lab (PR #96) are merged or merging; game feel is judged in this demo.
- Gore preset 4 stays the default. Tone: black humour plus 1980s to 1990s action-movie attitude ([antihero tone](../01-design/antihero-tone.md)).
- Camera lab owner verdict "yes to everything": automatic zoom with eased transits, 2P leash 416 px, Steam Deck options D + C (recorded in `docs/03-tech/camera-lab.md` on the camera-lab branch, PR #96).

Read with: [Floor 01 scene](../03-tech/floor01-scene.md), [DEATH-D spec](../03-tech/death-d-finishes-spec.md), [species death profiles](../03-tech/species-death-profiles.md), [gameplay](../01-design/gameplay.md), [combat feel and hybrid physics](../01-design/combat-feel-hybrid-physics.md), the open Act I beat proposal (PR #78) and the [backlog](backlog.md).

## (a) Inventory: what a player can reach today

Columns: **normal run** = `cargo run --bin floor01_scene` with no flags (the story corridor: walk and jump, no enemies); **fixture** = `floor01_scene --fixture <name> --manual` (interactive, but one small setup per run); **lab** = a separate probe binary or a lab-only flag; **not in Bevy** = art, data or a design that no binary draws. Status on `main` at `eae0b74`, plus the in-flight PRs where noted.

| Feature or art set | Reachable today | Where, and the gap the demo closes |
| --- | --- | --- |
| Carl locomotion (ANIM-CARL-01, ANIM-INT-01) | yes | normal run and fixtures |
| ENV-01 catacomb kit, A1 lighting (COMP-01/02) | yes | normal run; 2048 x 400 sample corridor, flat with two low platforms |
| Audio (AUDIO-01): music, ambience, swing and hit cues | yes / fixture | music and vent ambience everywhere; only `MoveStarted` and `Hit` map to cues. No cue for contacts, shore break, block landing, defeats or blasts although `EnemyBodyCollision`, `WallBreak` and `HeavyPropCrash` exist |
| Carl melee: jab, haymaker, push kick (ANIM-CARL-02, CP-01) | fixture only | hit-stop, stagger, knockback, camera kick, hurt, knockdown, fast retry |
| Goblins A and B (GOB-01): chase, tell, swing, token rule | fixture only | `cp01_*`, `cp02_*` |
| DEATH-A defeat modes D-01 to D-04 (#79): kick launch, downed state, kick contact, shore crush | fixture only | `cp01_kick`, `cp02_kick_contact`, `cp03_shore_kill`, `cp03_kick_block` |
| CP-03 rotten shore and falling block, ENV-02 shore / block / dust art | fixture only | the three `cp03_*` fixtures (one shore per fixture) |
| DEATH-B cause, force and species profiles (#94) | fixture only | data path, invisible except through the finish chosen |
| DEATH-C presenter and gore pool, preset 4 default (#97), GOB-02 and GOB-02B death art | fixture only | crush poses, knockback blood, stains; `--gore-preset 1..4` |
| DEATH-D decapitation (block edge) and explosion (QA charge) | lab flag only / in PR | on `main` only `--death-lab-finish decapitated`; real causes in draft PR #108 (`cp03_block_edge_decap`, `qa_blast_breakup`) |
| DEATH-E1 budgets, corpse cap 8, stains persist (#106) | fixture only | `cp03_goblin_pileup` (ten stationary goblins) |
| Hybrid-A HUD (plates, HP, System notices) | not on `main` | draft PR #103 (`floor01_scene`, normal run and fixtures) |
| Camera: deterministic scroll camera | yes | normal run and fixtures; its position is simulation state (in `FLOOR1_SCENE_STATE`) |
| Camera lab automatic shared camera, zoom 640 / 768 / 960, eased transits | lab only | `camera_lab` binary (PR #96); not in `floor01_scene` |
| 2P ownership, 416 px leash, solo swap, idle-partner catch-up | lab only | `camera_lab` |
| Donut | yes (passive) / lab | `floor01_scene`: passive trail follower, no collision or combat. `camera_lab`: crude follower AI with swat and pounce (ANIM-DONUT-02 atlas). Magic Missile (ANIM-DONUT-03): not in Bevy |
| Bad Llama LLAMA-01 | lab only | `camera_lab` ROOM-02 generic foe state machine with spit; not in `src/` gameplay |
| LLAMA-02 death poses and parts | not in Bevy | art and manifests only; the slam rule was waiting for the owner (now approved) |
| Mongo (76 px idle; ANIM-MONGO-01 locomotion) | lab only / not in Bevy | idle in `character_playground`; locomotion not drawn anywhere |
| Engineer (ENG-01), Scatterer (SCAT-01), Hoarder (HOARD-01), Troglodyte Basher (TROG-02), Juicer (JUICE-01), Ball of Swine (SWINE-01) | lab only | `camera_lab` rooms 2 to 6, lab hypotheses only |
| Rat (RAT-01), Green Slime (SLIME-01), Rot Sticker (ROT-01), Bomb Bard (BARD-01), Copper Chopper (CHOP-01), Brood Guardian (BROOD-01), Troglodyte Pygmy (TROG-01), Virtuoso (TROG-03), Tusklings (TUSK-01), Murder Dozer (DOZ-01) | not in Bevy | readability sets only |
| VFX-01 burst, ANIM-CARL-03 dynamite fizzle | in PR / lab | drawn by PR #108 for the QA charge only; hit spark in `camera_lab` |
| ENV-02 opening route (barricades, guild portal, hatch, soffit ceiling, 3584 px route proposal) | not in Bevy | only the shore, block and dust pieces are drawn (fixtures); the lab recomposes kit pieces |
| ANIM-INTERACT-01, Donut gear, Odette, Mordecai and other concepts | not in Bevy | art or concept only |
| WORK-021 greybox (bombs, timber, steel climb) | yes, other binary | `cargo run --bin dungeon-floorbreak`, placeholder visuals; not part of the demo |

**Conclusion.** Almost every combat feature is real but locked inside one-setup fixtures. The demo's main job is to put them in **one continuous run** with a respawn, add the llama, and run the approved camera on top.

## (b) The demo route

**Launch (after the three PRs below land):**

```sh
bash scripts/export_assets.sh                                   # once, as for any fixture
cargo run --release --locked --bin floor01_scene -- --demo --scale 3   # 1920x1080: the auto camera can zoom
cargo run --release --locked --bin floor01_scene -- --demo --deck      # 1280x800: 640 rest only, no zoom
cargo run --release --locked --bin floor01_scene -- --demo --scale 3 --demo-beat 4   # start at a beat
```

`--demo` is interactive (keyboard or pad, like `--manual`), lit, with audio, gore preset 4 and (with PR C) the automatic camera. Controls are the fixture controls: move, Cross/A or Space jump, Square/X or J jab (again inside the window: haymaker), Triangle/Y or K push kick. **Select / R tap: respawn at the last checkpoint now. Select / R held 1 s: restart the whole demo.** Circle/B, R1 and L1 do nothing (story-gated). Start or Esc quits.

**The hall.** The 2048 x 400 corridor cannot show any zoom: a 960 px view at 16:9 is 540 px tall and the camera never shows void. The demo therefore runs on a **new demo hall**, about 3200 x 640, composed at load time from the approved Floor 01 kit by the camera lab's `camera_lab::layout` composer (the same code that built the lab rooms: brick backwall bands, stacked arch bays, torches, deep floor on one walking line). It is a greybox composition of approved pieces, not new art and not an authored level. Floor flat except two low ledges (2 tiles); at least 470 px of drawn wall above the walking line and 80 px below.

**Respawn, not restart.** Carl has 4 HP. When he is knocked down, he gets up at the last checkpoint after the existing 50-tick delay with full HP; dead enemies stay dead and their gore stays (the aftermath is part of the joke). A checkpoint is the start column of each beat. A full restart (hold Select) is the existing fixture `Restart`, which also clears the gore.

| Beat | x range (proposal) | Setup | What it shows |
| --- | --- | --- | --- |
| B0 Wake-up | 0 to 500 | Carl at 160, Donut trailing; a low ledge to jump on and off | movement, jump and landing, lighting, music; nothing attacks |
| B1 First rematch | 500 to 1000 | two chasing goblins (A at 820, B at 900) | jab, haymaker, hit-stop and stagger, the goblin tell and swing, Carl's hurt; preset-4 knockback deaths, blood, stains that stay |
| B2 Kick alley | 1000 to 1500 | two chasing goblins that bunch up, a 2-tile ledge face about 60 px behind them | push-kick distance, a launched goblin knocking down the other (D-03 contact push), a kicked goblin stopped by the ledge face, hitting downed goblins, the heavy lethal-kick spray and the HUD `LAUNCHED!` notice (with #103) |
| B3 Rotten shore | 1500 to 2050 | the CP-03 shore (strike column about 1640); a passive goblin in the middle of the footprint, a passive goblin standing exactly on the footprint's right edge facing left, one chaser from the right | three hits break the shore, the block falls: one **crush** and one **decapitation** (DEATH-D, deterministic because of the edge placement), the chaser can be baited or kicked under it (the `cp03_kick_block` chain), `FLATTENED!`; the block never hurts Carl (open question) |
| B4 Bad Llama | 2050 to 2600 | one llama (home about 2420) and one chasing goblin in front of it | spit tell and dodge-by-jump, the fair-fire rule, the **slam rule** (lethal kick: slam pair; jab or haymaker kill: plain `down`), the auto camera widening when the llama takes aim (1080p and 4K) |
| B5 Lit charge | 2600 to 2950 | a QA blast charge with two passive goblins huddled on it and one chaser; the charge arms when Carl passes 2640 and blows 75 ticks later | **explosion**: one point-blank burst, one ring kill thrown intact, one survivor; the black-humour beat (goblins warming their hands on their own fuse). A labelled test prop, never Carl's ability |
| B6 Pileup | 2950 to 3200 | six chasing goblins (A and B alternating) in two clusters | crowd pressure with the two-attacker token rule; the **corpse cap of 8** retiring the oldest bodies while every stain stays; walk back left to see the whole mess. The console prints `DEMO01_COMPLETE` when the last enemy falls |

Expected run: 3 to 5 minutes, about 18 goblins and one llama. Every beat maps onto the PR #78 order (I-03 punch, kick, stone; I-06 llama) without the Dozer, the Guild or Donut's magic; the llama appears pre-biscuit here only to test its feel.

**Dependencies per beat.** B0 to B3 crush and B6 need only PR A. The B3 decapitation and B5 need PR #108 merged first (if it slips, B3 shows two crushes and B5 is cut; nothing else waits). B4 needs PR B. The zoom in B4 needs PR C. The HUD notices need PR #103.

**Stretch beats (only if the core is in and green):**

- S1: a second llama on a ledge in B4, staggered tells (PR #78 I-07 escalation, vertical framing for the auto camera).
- S2: a second shore in B6 to crush part of the pileup (needs `Fixture` to hold more than one shore).
- S3: HUD notice texts for decapitation, explosion and slam (HUD owner picks the words, tone rule applies).

## (c) Integration work: three parallel PRs (medium tier)

**Common rules for all three.**

- Executors: medium tier (Claude Sonnet). Review: one independent top-tier review per PR, then the owner approval route of AGENTS.md for gameplay and Bevy PRs. Design problems go back to the top tier; executors do not re-decide them.
- Start from `main` **after PR #108 (DEATH-D) and, if it is ready, PR #103 (HUD) have merged**; both edit `floor01_scene.rs` and `combat_scene.rs`. If #108 is not merged within a day, start anyway and leave its beats out.
- Fixed 60 Hz tick, no wall clock, no unseeded randomness. Presentation (gore preset, lighting, audio, camera view, HUD) never feeds the simulation: the same replay gives the same hash with any presentation flag (tested per PR).
- **No existing fixture hash, committed `headless_*.txt` log or story-route hash may change** (`bash scripts/ci_floor01_combat.sh headless` byte-identical for the 13 fixtures on `main` plus #108's two). A new field mixes nothing into the hash when it is empty. A PR that must change one migrates it explicitly in its description with old and new values.
- New fixtures add committed hashes and logs. Real Bevy evidence by xvfb as in DEATH-C/D, labelled honestly; no game-feel claim.
- `floor01_scene.rs` is shared: each PR keeps its edits in its own named systems and its own argument lines, and the PR merged second rebases. `docs/03-tech/floor01-scene.md` gets one new section per PR.

### PR A: demo hall, route and respawn (`feat/demo-01-route`)

**Scope.** The `--demo` run of section (b): the composed hall, beats B0 to B3, B5 and B6, checkpoint respawn, full restart, `--demo-beat N`, audio cue mapping, per-beat probes and evidence.

**Files.**

- `src/demo.rs` (new, Bevy-free): the `Demo` description (hall room, checkpoints, the demo `Fixture`, the beat table, `DEMO01_LLAMAS` spawn data left empty until PR B is on `main`), the autopilot scripts of the beat probes.
- `assets/levels/floor01/demo01_hall.room.json` (new): the hall in the camera-lab room schema (geometry only: size, floor, ledges, empty props and enemies), composed at load time by `camera_lab::layout::build_kit`.
- `src/combat_scene.rs`: an optional checkpoint list on `CombatState` (`None` for every existing fixture: behaviour and hashes unchanged), the respawn path (a new `CombatEvent::Respawn { tick, checkpoint }`, mixed only when emitted), the hold-to-restart timer.
- `src/blast.rs`, `src/blast_art.rs` (from #108): an arming rule `Arm::CarlPast { x, fuse_ticks }` next to the existing tick rule (existing QA fixture unchanged); before arming the charge draws a still frame, the fizzle loop only after arming.
- `src/audio.rs`: map `GoblinContact` to `EnemyBodyCollision`, `ShoreBroken` to `WallBreak`, `ShoreLanded` and `Blast` to `HeavyPropCrash` (existing cues and files only; mixer caps unchanged).
- `src/bin/floor01_scene.rs`: `--demo`, `--demo-beat N`, layout selection (the hall instead of `LAYOUT_PATH`), Select tap and hold.
- `scripts/ci_floor01_combat.sh` (the demo probes), `docs/evidence/demo-01/`, `docs/03-tech/floor01-scene.md` (Demo section), `docs/04-production/backlog.md` (DEMO-01 row status).

**Acceptance.**

1. The launch commands of section (b) start the demo at B0 (or at beat N) on the hall, lit, with audio and preset 4, controllable with keyboard and pad.
2. The hall composes through the existing `EnvKit` and A1 lighting budget (test as in the lab's `every_room_composes...`); a test checks clean flat walking line under every spawn, checkpoint, the shore strike column and footprint and the charge; the end walls stop Carl.
3. Beat probes `demo01_b1` to `demo01_b6` (start at the beat's checkpoint, scripted input) each have a committed hash and log and assert: B1 two melee defeats; B2 at least one `GoblinContact` and one launch; B3 one `ShoreBlock` crush and (with #108) one `ShoreBlockEdge` decapitation from the same landing; B5 one point-blank and one ring defeat and one survivor; B6 six defeats and, at the end, at most 8 settled corpses drawn with every stain kept.
4. Respawn: knocked down, Carl stands at the last checkpoint after `RESTART_DELAY_TICKS` with full HP, defeated enemies stay defeated, the presenter keeps its gore; tap Select respawns at once; held 60 ticks it restarts the demo (the existing `Restart`). Tested headlessly.
5. Every probe gives the same hash at presets 1 and 4, with and without lighting and audio. All existing hashes unchanged.
6. Evidence: a real 1920x1080 capture per beat (GIF or strip), the probe logs, `--perf-from-tick` style frame and memory figures for B5 and B6 (development machine only), SHA256SUMS.

### PR B: minimal Bad Llama with the slam rule (`feat/demo-01-llama`)

**Scope.** A small, deterministic llama enemy in the combat domain, its two finishes, and its drawing. Every number is a playtest proposal kept in `src/feel.rs`.

**Gameplay (proposal).**

- Health 3 (every Carl move does 1). Walks toward Carl at 40 px/s until 200 px away, stays within 160 px of its home column, never retreats.
- Spit: tell 36 ticks (LLAMA-01 `attack_tell`, the glow), release on the `attack_swing` frame spawns the glob (LLAMA-01 spawn contract), recover 24 ticks, cooldown 90 ticks. The glob flies straight at mouth height, 150 px/s, at most 320 px, stops on a solid, hits Carl once for 1 HP through the existing hurt path (invulnerability respected). Carl can jump it.
- **Fair-fire (owner option D):** a tell starts only while the llama's body is inside the simulation's 640 px scroll-camera view and Carl is within 320 px in its facing direction. This uses simulation state, so it is the same on every output.
- Reactions: jab or haymaker hurt 18 ticks (a hit in the tell cancels it); a kick pushes it back 30 px and cancels the tell. No launch, no contact with goblins (disclosed).
- **Slam rule (owner decision 2026-11-19):** the lethal hit emits the usual `EnemyDefeated` with the move's force. Heavy blunt (the lethal kick) selects `llama_slammed`: `slam_contact` 6 ticks, then `broken_heap` held, with the LLAMA-02 section 8 slam timing (tooth, 8 to 12 drops, one molten drop, three wool tufts, pool stages 1 and 2) scaled by the preset data. Routine blunt (jab, haymaker) selects `llama_down`: LLAMA-01 `down` then its last frame held, with the routine goblin blood amounts. Other llama finishes (crush, decapitation, explosion) are not wired: the presenter degrades them to `llama_down`, and the demo keeps the llama away from the block and the charge.

**Files.**

- `src/llama.rs` (new, Bevy-free): state machine, spit projectile, hit handling, hash mixing.
- `src/combat_scene.rs`: `Fixture.llamas: &'static [LlamaSpawn]` (`&[]` on every existing fixture, nothing mixed when empty), llamas stepped after the goblins in index order, llama targets in the strike list (defeat `target` indices after the goblins), new fixtures.
- `src/death.rs`, `assets/data/death/species_death_profiles.json`: recipe kind `slammed`; the llama profile gains `{ "kind": "slammed", "finish": "llama_slammed", "causes": ["blunt"], "min_force": "heavy" }` ahead of `knockback_down`; the selection table test pins llama blunt Heavy to `llama_slammed` and Routine to `llama_down`; a species per target in the observation.
- `src/gore.rs`: load the LLAMA-02 poses and parts with the `molten` layer shown at every preset.
- `src/death_present.rs`: the two llama finishes; llama corpses count toward `MAX_SETTLED_CORPSES` with the goblins.
- `src/combat_anim.rs`: a pure `llama_pose` (tag and frame from the state).
- `src/feel.rs`: the llama block. `src/bin/floor01_scene.rs`: llama and glob sprites (pre-spawned, one new sync system).
- `scripts/ci_floor01_combat.sh`, `docs/evidence/llama-int-01/`, `docs/03-tech/floor01-scene.md` (Llama section), `docs/04-production/backlog.md` (LLAMA-02-DEATH and DEMO-01 status).

**Acceptance.**

1. New fixtures with committed hashes and logs: `cp05_llama_slam` (kicks to death: one `llama_slammed`), `cp05_llama_down` (jab, haymaker, jab: one `llama_down`), `cp05_llama_spit` (Carl idles inside the frame: two spit hits, the second after invulnerability), `cp05_llama_offscreen` (llama outside the scroll view: no tell).
2. Unit tests: state machine timings, fair-fire, a jump clears the glob, the glob stops on a solid, the leash to home, the slam and down selection, preset 1 shows the molten layer and no gore-layer pixel, the corpse cap counts llamas.
3. Same hash at presets 1 and 4; all existing hashes unchanged.
4. Evidence: real captures of the tell and spit, the slam and the plain down at presets 4 and 1 (GIF and stills at 1920x1080 and 640x360).
5. Whichever of PR A and PR B merges second fills `DEMO01_LLAMAS` (B4 data only) and adds the B4 probe `demo01_b4` (one slam from a lethal kick).

### PR C: the camera-lab automatic camera in the real scene (`feat/demo-01-auto-camera`)

**Feasible as a presentation layer:** `camera_lab::camera_op` is pure and tested, the lab binary already drives the existing A1 pass at coverages above 1.0, and the scene's scroll camera can stay in the simulation untouched. Precondition: PR #96 merged.

**Scope.** `--camera auto|scroll|1.0|1.25|1.5` for `floor01_scene`; **`auto` is the default with `--demo`, `scroll` everywhere else** (so every existing capture and CI image is unchanged).

**Files.**

- `src/scene_camera.rs` (new, Bevy-free): after each tick, builds the `CameraInput` from `SceneSim` and `CombatState` (subjects Carl and Donut; threats: a llama that has acquired Carl, its tell and a glob in flight, by the lab's priorities), owns the `CameraOperator` for the output size, adds the CP-01 camera kick in whole pixels, returns the view; a `--view-trace` line per tick for tests.
- `src/bin/floor01_scene.rs`: the flag, lighting slot targets allocated at the largest coverage of the output, per-frame slot camera viewport and projection (as `sync_visuals` in `camera_lab.rs`), parallax and torch gathering from the presentation view; the HUD camera untouched.
- `src/camera_lab/camera_op.rs`: read only (making an item `pub` is the only allowed edit).
- `docs/03-tech/floor01-scene.md` (Camera section), `docs/evidence/demo-01-camera/`.

**Acceptance.**

1. The scroll camera stays in the simulation and in `FLOOR1_SCENE_STATE`; the presentation view is never read by the simulation. The demo probes give the same hash with `--camera auto`, `scroll` and `1.5`; all existing hashes unchanged.
2. Lab behaviour preserved in the scene: rests only on exact integer scales of the output (1080p 640 and 960, 4K 640, 768, 960, Deck and 720p 640 only), cap 960, eased Hermite transits, at most 14 px of width and 12.5 px of pan per tick, no transit during hit-stop, never outside the hall bounds (tests over a recorded demo trace at the three outputs).
3. At 1920x1080 the B4 llama makes the camera widen to 960 and return; captured as a real video or GIF with the view trace.
4. The camera kick still reads on the jab, haymaker and kick at 1.0 and 1.5.
5. Frame time and memory at 1080p and 4K, release, B4 and B6, against `--camera scroll` (development machine; no Steam Deck claim).
6. **Fallback, if retargeting the slots needs a lighting rewrite beyond about a day:** ship `auto` limited to 640 (framing, threat bias, eased pan), record the blocker in the PR, and never fake a zoom.

**Recommended merge order.** #108 and #103 (in flight) first; then PR A and PR B in any order (the second adds the B4 data); PR C last, because its zoom acceptance needs the tall hall of PR A and the llama of PR B. The owner plays once all three are on `main`, or earlier on whatever has landed, with the questionnaire's skip rules.

**Living-horizon review.** The only new gameplay is the Floor 01 Bad Llama already planned in PR #78 (I-06) with the owner-approved fair-fire rule and slam rule; Carl gains nothing; Donut is unchanged; the QA charge is a test prop. This still fits [character-progression-horizon.md](../01-design/character-progression-horizon.md).

## (d) Known gaps: what the demo will not show

- **Donut is a passive trail follower**: no control, no AI, no combat, cannot be hurt, passes through Carl. No 2P, no solo swap, no leash (the lab's 416 px leash is approved but not integrated). No Magic Missile, biscuit or mana.
- **No Carl explosives.** The B5 charge is a QA test prop that does not hurt Carl or terrain; the dynamite special stays story-gated.
- **QA charge on the route (orchestrator decision 2026-11-19, DEMO-01 only).** The [DEATH-D spec](../03-tech/death-d-finishes-spec.md) says the QA charge has no route. For the demo only, B5 may place it on the route, clearly labelled as a test prop; the rule stays in force everywhere else (no route, ability or input creates a charge outside DEMO-01).
- **PR A as built differs from the plan in four small ways** ([Demo](../03-tech/floor01-scene.md#demo-demo-01-pr-a-hall-route-and-respawn)): goblins need no wake rule, because `GOBLIN_SIGHT` (220 px) already keeps each beat's goblins asleep until Carl walks in; the beat probes run on `--demo --demo-beat N --autopilot` with a pure autopilot function instead of scripted `Hold` lists (hold lists would break at every tuning change); a Select tap respawns on release (so a hold can still restart); a full restart returns to the beat the demo was started at. B4 and B5 are marked hooks until PR B and PR #108 land; the probes are `demo01_b1`, `_b2`, `_b3`, `_b6` and the whole route `demo01` (`demo01_b4` and `demo01_b5` come with their beats).
- **Not the Act I route**: no Murder Dozer chase, no Guild, no ENV-02 opening route (barricades, portal, hatch, soffit ceiling). The hall is a greybox recomposition of approved kit pieces.
- **Two enemy types only** (goblins A/B and one llama). Rat, slime, rot sticker, bard, chopper, engineer, scatterer, brood guardian, hoarder, troglodytes, juicer, swine, tusklings and the Dozer remain art-only or lab-only. No Mongo.
- **Llama is minimal**: no crush, decapitation or explosion finishes for it, no contact with goblins, no spit or llama sounds, numbers unreviewed.
- **Audio is thin**: the kick whoosh for every swing, one hit-confirm, reused crash cues for the shore, block and blast; no punch, death, decapitation, explosion or voice sounds.
- **Camera**: no zoom at 1280x800 or 720p (integer rule); Steam Deck option C (opt-in wider view) is not implemented; option D exists only as the llama fair-fire rule. The block falling and the crowd do not widen the view (the lab found they fit at 1.0x).
- **Damage rules left open**: the falling block never hurts Carl; downed goblins can still be hit; corpses never collide (CP-04 off); a wall impact is not a finish.
- **Known presentation defects** stay visible: GOB-STACK-01 (stacked goblins, deep left-facing blows), the wall-spatter contrast on lit brick, the preset-1 spark colour; no screen-shake comfort toggle (DEATH-COMFORT).
- **No menus, pause, settings, save or controller remapping**; Select is respawn and restart. Steam Deck and Steam Controller hardware untested; glyphs follow the pad only with the HUD (#103).
- **Performance** is measured on one development machine only; nothing is claimed for the Steam Deck.
- **No acceptance is implied**: the demo exists to collect the owner's feel verdict; every tuning value is a proposal until the questionnaire and a follow-up decide it.
