# F01 playtest owner packet (PT-GATE-B)

The entry point for the owner's playtest of the late Floor 1 slice, and the evidence for the "playtest ready" gate of [plan section 10](../../07-agents/f01-playtest-integration-plan.md). It collects launch commands, what to look for in each room, the adopted defaults, the open questions, the scripted scorecards, the CA2 metrics of the closing arena, the gate checklist and the known gaps.

**Game feel is pending everywhere.** Nothing in this packet was played by a human, on a pad or on real hardware, and nothing is a game-feel verdict. Every number is a scripted-pilot or fixture measurement. A metric that cannot be derived is written **NOT RUN**, never estimated. **Gate verdict (independent review of PT-GATE and of #238): NOT playtest ready.** Items 1, 2, 3b, 4b, 5 and 7 are UNMET (section 8). The closing work is two follow-up PRs: **PT-GATE-C** (code: the room partner, the room 1 listing, aligned room pilots, the room 2 and room 4 gate fixtures, one gauntlet test) and **PT-GATE-D** (final evidence: the render-loop replay, 640x400 GIFs, room 4 routes, the Boiler at 640x400). This packet is evidence of the current state, not a ready declaration.

Files: [`scorecard.md`](scorecard.md) (generated tables), [`GATE_A.md`](GATE_A.md) (the PT-GATE-A evidence: tooling, Boiler perf, grants, the room 2, 4 and 5 sessions), [`perf.txt`](perf.txt) and [`perf_mastery.txt`](perf_mastery.txt), [`owner_commands_check.txt`](owner_commands_check.txt), [`capture_lines_b.txt`](capture_lines_b.txt), [`pilot_hashes_mastery.txt`](pilot_hashes_mastery.txt), [`playtrace_session_mastery.txt`](playtrace_session_mastery.txt), the GIFs, and `SHA256SUMS` (verified by `scripts/check_docs.py`).

## 1. Launch commands (plan section 9)

Release build, from the repository root, after `bash scripts/export_assets.sh`:

```text
cargo run --release --bin floor01_scene -- --scene f01_late --scale 2
    [--f01-room hoarder|boiler|hazards|juicer|mastery]    # jump to a room with its story pin (default: boiler)
    [--f01-loadout story|all]                              # story implies --carl-prog --donut-prog; all = TEST LOADOUT
    [--debug-combat] [--no-record]                         # recording is on by default (PLAYTRACE)
tools/record-run.sh floor01_scene --scene f01_late --scale 2    # window video plus audio
```

**`--companion` does not work with `--scene f01_late` on this head.** Plan section 9 lists `[--companion cf02_free|cf02_assist]` for `f01_late`, but a room scene is a fixture internally and the launcher refuses the flag together with a fixture (`--companion cannot be combined with --fixture or --party-fixture`, a CA-22 guard). So the autonomous `cf02_free` partner cannot be selected in rooms 1 to 6. To judge the autonomous partner use the arena line `--cf02 --cf02-engineer --carl-dynamite --companion cf02_free` (table below). **Which AI partner you actually get today:** in rooms 2, 4 and 6 (cf02 rules) the other hero is the **`cf02_assist`** companion (the `--cf02` default, `floor01_scene.rs` `uncontrolled_hero()`), not the CA2 autonomous partner, so the CA2 hazard feed never reaches you there (the assist partner walks into a live steam lane in the feed's own control fixture). In **room 5** (`boss_f01`, cf02 rules off) the other hero is the **follow-and-jump bot**, not CA2 at all. Plan sections 8 and 9 make `cf02_free` the default in these rooms. This is a gate failure (item 2), not changed here; PT-GATE-C closes it (known gaps, section 9).

To play two players, connect a second pad and press Start: P1 plays Carl and P2 plays Donut. Select restarts the room. Start on P1 quits.

**Listed rooms.** The manifest `assets/levels/f01_late/manifest.json` lists `hoarder, boiler, hazards, juicer, mastery`, and the game prints that list. **Room 1 (the Hoarder) is currently listed.** A follow-up, **PT-GATE-C**, removes the `hoarder` row from the manifest and re-pins `MANIFEST_HASH` (`--scene f01_hoarder` stays); until it lands, gate item 3b is UNMET (section 8) and the placeholder dynamite fixture is not valid gate evidence:

| Room | Command suffix | Gate status |
| --- | --- | --- |
| 2, the Goblin Boiler | `--f01-room boiler` (also `--scene f01_boiler`) | listed |
| 4, slimes and Rot Stickers | `--f01-room hazards` | listed |
| 5, the Juicer | `--f01-room juicer` (also `--scene boss_f01 --carl-prog --donut-prog`) | listed |
| 6, the mastery arena | `--f01-room mastery` | listed |
| 1, the Hoarder | `--f01-room hoarder` | **listed today** (manifest); no presentation, no defeating pilot; to be unlisted by PT-GATE-C |
| 0 the corral, 3 the Dozer, 4a and 4b the Ring | `corral`, `dozer`, `ring_court`, `ring_split` | not landed in the manifest: the game refuses them (`unlisted room` or `unknown room`) |

**Interim and diagnostic builds** (plan section 9, still valid on this head):

| Command | What the owner can judge |
| --- | --- |
| `--cf02 --cf02-engineer --donut-biscuit` | the arena base with the Engineer |
| the same plus `--carl-dynamite` | dynamite feel, launches, self-damage |
| the same plus `--carl-dynamite --companion cf02_free` | the autonomous AI partner |
| `--cf02 --cf02-engineer --carl-dynamite --carl-gauntlet --donut-puddle` | the full kits in the arena (implies Light on Your Feet and the biscuit) |
| `--scene boss_f01 --carl-prog --donut-prog` | the Juicer with dynamite and the grants |
| `--scene f01_boiler` | the Boiler setpiece |
| `cargo run --release --locked --bin corral_eval -- --out target/corral-eval` | a machine report, not game feel |

**Verified on the final head.** [`owner_commands_check.txt`](owner_commands_check.txt) holds one line per command above (and the refused `--companion` lines), run by `scripts/check_f01_owner_commands.sh` in the real window under xvfb for 30 ticks (the debug binary, no audio, no recording; the release build takes the same arguments). `ok` means the process started and exited 0; `refused` is an expected refusal: an unlisted room, or `--companion` on `f01_late`. **NOT RUN:** the commands in a release build, `tools/record-run.sh` (needs a display, `xdotool`, ffmpeg), a second pad.

## 2. What to look for, per room

- **Room 2, the Goblin Boiler (setpiece).** Jab the steam valve twice (a steam lane telegraphs 24 ticks, then bursts 72), jab the brace to drop the weight on goblins under its footprint, kick a goblin into the valve for the chain, and use the drum. The direct route must stay safe with every prop untouched. Look for: whether the props read as threats you can use, whether the steam telegraph is fair, the Engineer, and the greybox look (stand-in props are labelled, K10). Known: B3 can wake inside the steam lane (section 9).
- **Room 4, slimes and Rot Stickers (greybox).** Three isolated pockets: a slime with an acid pool, a Rot Sticker overhead, a crossover. Look for: the slime tell and its punish window, that acid bars walkers, the Rot Sticker's 30-tick warning and its blast on both heroes, and that nothing here can be reflex-evaded (K6). The scripted pilot does not dodge hazards and is defeated (section 6); a human should clear it.
- **Room 5, the Juicer (boss).** Carl holds dynamite and Donut the boost (the story pin). After the defeat two notices grant the gauntlet (HEAVY FIST) and the blink (PUDDLE JUMPER). Look for: dynamite on the boss, whether the grants read, and the partner: on the live `--f01-room juicer` the other hero is the follow-and-jump bot, not an autonomous Donut (the passive-escort figures of section 9 come from the `JuicerWho::Companion` fixture, which uses `cf02_free`, not from the live launch).
- **Room 6, the mastery arena (greybox, no presentation of its own).** Everything is held (the `f01_late_mastery` pin, no grant, no trigger). Wave 1 is the brawl lane: goblins, a slime with its acid pool and a spitting llama. Wave 2 is the llama yard: goblins in the matador lane, the Engineer with a llama, a second llama, and a Rot Sticker under the right perch. The exit gate opens when the roster is defeated. Look for the "powerful enemies" feel (overlap, the slime beat, the Engineer with a llama, clear time) against the questions of section 4.
- **Room 1, the Hoarder (listed in the manifest today; PT-GATE-C unlists it).** Wired and launchable; nothing is drawn for her, her door or her gate until HRD-B, so game feel cannot be judged. Launch it only to check it starts.

## 3. Adopted defaults K1 to K19

All are adopted defaults of the integration plan, recorded in its section 2 ([table](../../07-agents/f01-playtest-integration-plan.md)); the owner may change any of them in a follow-up. "Owner-level" means the plan also asks the owner (section 4).

| ID | Adopted value | Where it is recorded |
| --- | --- | --- |
| K1 | Dynamite is granted on the Hoarder's defeat (`pile_boss_f01_defeated`, a placeholder zone until she exists); the Boiler is the first dynamite room. Recorded as a game adaptation (fun over canon). Owner informed | plan section 2 K1; `assets/levels/f01_late/manifest.json` `story_order` |
| K2 | CP-Q34 caps charge chains at depth 4 (+8 ticks, 8 per tick); an encounter validator enforces at most 3 authored links in an acyclic graph | plan K2; CP-Q34 |
| K3 | No friendly fire only for the hero-thrown dynamite profile. World blasts and hazards hurt every hero; one `hero_damage` policy enum (`OwnerReduced`, `AllHeroes`, `NoHeroes`) on `BlastCharge` and hazard volumes | plan K3; `src/hero_damage.rs` |
| K4 | Setpiece rooms and the mastery arena run under `cf02_rules`; no new enemy-on-enemy damage is added | plan K4 |
| K5 | Dozer revenge room only: cruise at most 170 px/s, charge and rubber band never above Carl's 185 px/s, no Donut invincibility. Room 3 is not listed | plan K5 |
| K6 | Hazards, falling masses, blasts, boss blows and body contacts are not reflex-evadable (DM-Q18) | plan K6; the K6 fixtures (section 8, item 6) |
| K7 | Story-progressive loadout by default; `--f01-loadout all` is a labelled TEST LOADOUT. Owner-level | plan K7; question 1 below |
| K8 | Every setpiece room is validated for Carl (80 px), base Donut (96 and 104 px) and boosted Donut (128 px); an unthreatened 81 to 128 px nest is an error; `ceiling_px` at most 228 | plan K8; `src/encounter.rs` (`k8_*`, `dp_q45_*`) |
| K9 | The setpiece pins carry Light on Your Feet provisionally until the Act I doc's owner signs off | plan K9; the story pins in the manifest |
| K10 | Greybox props and code-drawn stand-in VFX are allowed in the playtest, labelled; art orders run in parallel. Owner-level | plan K10; question 2 below |
| K11 | The combat lab's LAB-02 reuses the Boiler core; the playtest has its own scene | plan K11 |
| K12 | The Bomb Bard is cut; the Goblin Engineer is used instead | plan K12 |
| K13 | The companion's hazard feed (CA2-C) reads the Boiler's hazard rectangles; until then a reported gap | plan K13; merged as #220 |
| K14 | The slice starts at the Hoarder's lair; Scatterers and rats are deferred | plan K14 |
| K15 | #176 is split: BOIL-A and BOIL-B core on the #176 branch, the wire steps as follow-up PRs. Owner informed | plan K15 |
| K16 | The Engineer is allowed in encounter-sidecar rosters (BOIL-E owns it); the story route, `--demo`, the lab and the boss arena still never load him | plan K16 |
| K17 | Room 5 runs the existing `boss_f01` scene with its own rules and the CP and DP story pins; `cf02_rules` only for the setpiece rooms and the arena; room changes are a scene relaunch | plan K17 |
| K18 | `src/env_prop.rs` is the one falling-mass engine for new setpieces; the CP-03 shore, the boss rack strut and the lab weight stay frozen | plan K18 |
| K19 | CP-A goes before CA2-B2; the shared hot files are append-and-rebase for every PR | plan K19 |

PT-TUNE adopted its own defaults PTT-Q1 to PTT-Q21 for the arena, in [`docs/03-tech/pt-tune-design.md`](../../03-tech/pt-tune-design.md) sections 7 and "Execution notes".

## 4. Owner questions

From plan section 13 (the full text is there):

1. **K7:** should the first session use the story-progressive loadout (default) or `--f01-loadout all` from the start?
2. **K10:** may the first playtest ship with greybox setpiece props and stand-in VFX (default: yes, labelled; art orders run in parallel)? This also covers the Ring's code-drawn stand-ins (KR17).
3. **KC3, only if reached:** whether a guarded corral llama may take a missile's damage without the `Hurt` stagger. **Not reached:** room 0 is not listed and CORRAL-EVAL's gate was not run for it.

The owner is also informed of K1 and K15, and of KC2, KC4, KC8, KC9 (the corral) and the #181 C-A sequencing (plan section 13).

PT-TUNE game-feel questions for room 6 (none blocks anything):

- **OQ1.** Does "kill the slime, the pool drains, the goblins rush" read as a trap you sprang, or as unfair?
- **OQ2.** Is wave 1's spit over the pool pressure or nagging? The alternative is the llama in wave 2 with a third goblin in wave 1.
- **OQ3.** Is one Rot Sticker enough for the arena?
- **OQ4. The human clear-time band.** Is 15 to 50 s per clear (ticks 900 to 3000) the "powerful" feel, or too long for a closing room? The scripted base-kit Carl clears in a median of 667 ticks (11 s); the band is for the owner's playthrough, **blank in the scorecard below and NOT RUN here**.
- **OQ5.** With party targeting, Donut gets spat at too. Is that right for the solo AI partner?

## 5. Scorecards

**Scripted reference** (the committed pilots and fixtures at seeds 0..8, median / worst; generated in [`scorecard.md`](scorecard.md), which has every column and the per-seed tables). Deaths are hero defeats, restarts are Select restarts (the pilots never press Select), the time is the exit tick (60 ticks a second), pierces are **NOT RUN everywhere** (no scripted pilot swings the gauntlet and the combat log has no pierce event), and throws, chains, blinks and evades are 0 for every base-kit pilot **by construction**, not a measurement of play (only the room 6 kit pilots press the dynamite and the blink).

| Room and pilot | Clears | Deaths | Hurts | Time (ticks) | Throws / chains / blinks / evades | Max stuck ticks |
| --- | --- | --- | --- | --- | --- | --- |
| 2 Boiler **greybox, no roster** (`Roster::Inert`: no goblins, no Engineer; not the live room), CarlSolo, direct | 9 of 9 | 0 / 0 | 0 / 0 | 308 / 308 | base kit: 0 | 0 |
| 2 Boiler greybox, no roster, DonutBySwap, direct | 9 of 9 | 0 / 0 | 0 / 0 | 286 / 286 | base kit: 0 | 1 |
| 2 Boiler greybox, no roster, CarlSolo, valve then brace | 9 of 9 | 0 / 0 | 0 / 0 | 485 / 485 | base kit: 0 | 1 |
| 2 Boiler greybox, no roster, DonutBySwap, valve then brace | 9 of 9 | 0 / 0 | 0 / 0 | 496 / 496 | base kit: 0 | 7 |
| 2 Boiler **live roster** (`Roster::Live`), room pilots `direct`, `valve`, `chain`, solo Carl, one seed (committed `docs/evidence/f01-boiler/pilot_hashes.txt`) | 1 of 1 each | 0 | n/a | exit ticks 536, 641 and 949 | not counted | not counted |
| 2 Boiler live roster, Donut by swap and the companion (`cf02_free`), seeds 0..8 | **NOT RUN** (no such pilot exists; closing work PT-GATE-C) | | | | | |
| 4 Hazards, CarlSolo, DonutBySwap, Companion | **NOT RUN** per seed. One room-pilot run: see `scorecard.md` (the pilot is defeated and retries, it does not dodge) | | | | | |
| 5 Juicer, CarlSolo | 9 of 9 | 0 / 0 | 38 / 43 | 3470 / 3887 | base kit: 0 | 17 |
| 5 Juicer, DonutBySwap | 9 of 9 | 0 / 0 | 38 / 43 | 3445 / 3863 | base kit: 0 | 17 |
| 5 Juicer, Companion | 9 of 9 | 0 / 0 | 39 / 44 | 3470 / 3887 | base kit: 0 | 17 |
| 6 Arena, carl_solo | 9 of 9 | 0 / 0 | 4 / 4 | 737 / 771 | base kit: 0 | 91 |
| 6 Arena, donut_by_swap | 9 of 9 | 0 / 0 | 4 / 4 | 735 / 764 | base kit: 0 | 91 |
| 6 Arena, companion (`cf02_free`) | 9 of 9 | 0 / 1 | 3 / 5 | 835 / 1053 | base kit: 0 | 95 |
| 6 Arena, donut_fights (Donut leads, base kit) | **0 of 9** | 7 / 7 | 30 / 31 | no clear | base kit: 0 | 21 |
| 6 Arena, carl_kit (dynamite, report-only) | 9 of 9 | 1 / 1 | 4 / 6 | 874 / 1009 | throws 1 / 2, chains 0 | 95 |
| 6 Arena, donut_kit (blink, report-only) | **0 of 9** | 8 / 8 | 30 / 30 | no clear | blinks 1 | 21 |

The Juicer rows are library fixtures (the `Companion` row uses `cf02_free`; the live room's partner is the follow-and-jump bot, section 1). The backlog attributes nearly every hurt of the companion run to the passive Donut (section 9); the table does not split them by hero. Room 1 (not listed): **NOT RUN**.

**"Powerful enemies" rating line.**

- **Room 6 (PT-TUNE D1 to D6, `carl_solo`, seeds 0..8):**
  - D1 overlap share W1 37 to 38 % (target median 30 %, every seed 20 % or more), W2 45 to 65 % (median target 40 %): pass.
  - D2 peak distinct engaged kinds W1 2 (3 seen over the wave), W2 3: pass.
  - D3 bot clear median 667 ticks, range 667 to 684 (main 517, target at least 647, at most 4800): pass; the human band 900 to 3000 is the owner's (OQ4).
  - D4 hurts 4 (2 in each wave) and 0 deaths on every seed: pass.
  - D5 peak engaged enemies 3 (limit 4): pass.
  - D6 every mastery enemy keeps its base kind's HP: a test (`the_mastery_waves_meet_the_difficulty_targets` and its HP guard).
- **Rooms 2, 4, 5:** composition and enemy numbers unchanged by PT-TUNE; there is no "powerful enemies" measure for them. The only numbers are the hurts above: **NOT RUN** as a rating.
- **Owner rating (1 to 5, with a sentence):** blank below.

**Owner scorecard (blank, for the human playtest).** One row per run.

| Room | Loadout, partner | Deaths | Restarts | Time | Throws | Chains | Pierces | Blinks | Evades | "Powerful enemies" (1 to 5) | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2 Boiler | | | | | | | | | | | |
| 4 Hazards | | | | | | | | | | | |
| 5 Juicer | | | | | | | | | | | |
| 6 Arena | | | | | | | | | | | |

## 6. Room 6: why the room pilot is defeated and the fixture pilot is not

The live room pilot (`--room-pilot mastery`, used for the perf, the wave GIF and the PLAYTRACE session) is defeated once, on tick 908, and exits on tick 1190. The PT-TUNE `carl_solo` fixture at seeds 0..8 has 0 deaths and clears on tick 667 to 684. **Cause, reproduced: a different partner, not a different seed or plan.** A scripted window run drives the other hero with the follower bot (`companion_inactive`), while the fixture run's partner is the active `cf02_assist` companion, which attacks and kills the slime on tick 101 where the follower-bot fight takes it on tick 180. With the same Carl pilot the first seven defeats of the follower fight are ticks 180, 239, 311, 529, 683, 683 and 786 (equal to the committed window capture) and Carl is then defeated once; with the assist partner he is never defeated. A pinned test, `f01_gate_scorecard::room_pilot_discrepancy`, holds both facts. It is not a seed effect (the room pilot is defeated at every seed 0 to 8 through `--cf02-seed`, once, on ticks 900 to 915, and twice at seed 4), and not a plan effect (the plan is the same function). A **6-tick residual** is not explained: the replica defeats on tick 914 where the window run does on 908. Nothing was changed to hide it. It is in the known gaps.

## 7. CA2 metrics M1 to M16 for the mastery arena

The autonomous partner (`cf02_free`) in the arena, Carl on the room's pilot, 1800 ticks, seeds 0..8, from the CA2 harness (`f01_mastery_fixtures::metrics`) on the final head; the per-seed table is in [`scorecard.md`](scorecard.md). Targets are those of `docs/03-tech/companion-ai-02-donut-autonomy.md` section 7.1. "Bound" is what the committed test `the_ca2_metrics_run_in_the_closing_arena` asserts.

| Metric | Measured (seeds 0..8) | Spec target | Bound in the test | Result |
| --- | --- | --- | --- | --- |
| M1 spread, median distance | 79 to 80 %, median 136 to 249 px | at least 40 % and 96 px | the target | pass, 9 of 9 |
| M2 shadowing | 31 to 60 % | at most 20 % | at most 65 % | **MISS**, 9 of 9 (bounded, a design note) |
| M3 distinct targets, same-target ticks | 76 to 85 %, 3 to 20 ticks | at least 70 %, at most 15 % | the target | pass, 9 of 9 |
| M4 kill steals | 0 on 7 seeds, 1 on 2 (seeds 0 and 5) | 0 | at most 1 | **MISS** on 2 of 9 (bounded) |
| M5 self-caused exits from view | 0 | 0 | the target | pass |
| M6 outside ticks, ahead-side exits | 0, 0 | 0, 0 | the target | pass |
| M7 camera width series | **NOT RUN in the arena** | pinned by CA2-B tests | n/a | the arena harness does not measure it |
| M8 body inside the view | **NOT RUN in the arena** | every tick of the `cf02_free_*` fixtures | n/a | not measured here |
| M9 hazard hurts, entries, unplanned falls | per-cause count **NOT RUN** (`HeroHurt` carries no hazard class); hurts to Donut in the companion run to the exit: 0 on every seed | 0 | n/a | no hurt to her; the cause split is not measured |
| M10 retreat distance, hits in a retreat | no retreat triggered on any seed (0 retreats) | at least 96 px, at most 1 hit | at least 90 px, at most 1 hit | vacuous: **not exercised** |
| M11 show-off | **NOT RUN in the arena** (its fixture is `cf02_free_perch_and_show_off`) | at least 1, 0 | n/a | n/a |
| M12 regroup teleports | 0 | 0 | the target | pass |
| M13 her landed hits against the assist partner's, hit share | 4 to 7 hits (23 to 41 %) against 7 to 8 | at least the assist value, at most 50 % | share at most 50 % | share passes; the comparison **MISSES** on 6 of 9 seeds (reported) |
| M14 twice-identical, planning time | identical on 9 of 9; planning time per tick **NOT RUN** | identical; reported | identical | pass; the cost figure is NOT RUN |
| M15 retreat share | 0 % | at most 25 % | the target | pass |
| M16 interrupts skipped, goblins that targeted her, drift-rejected Hunt candidates | 0 skipped, 1 goblin, 11 to 31 drift-rejected | reported, no target | n/a | reported |

M2, M4 and M13's comparison are the PT-TUNE design notes (PTT-Q16); the misses are deferred to CA2-C and CA2-D (`CA2-FU-M13`), not retuned.

**Setpiece rooms (companion fixtures' own counts; everything else NOT RUN).** These are staged fixtures asserted by tests (no per-seed table), in `src/cf02_free_fixtures/hazard.rs` and `src/hazards_fixtures/tests_b.rs`:

| Room | Fixture | Count |
| --- | --- | --- |
| 2 Boiler | `steam_she_holds_short_of_the_lane_...` | hazard hits 0, never inside the live lane, the assist partner without the feed does enter it (the control) |
| 2 Boiler | footprint fixtures | she leaves the floor shadow before the weight lands; hits 0 |
| 4 Hazards | `haz_free_partner_is_not_hurt_by_the_hazards_the_feed_names` | hurts 0, stickers woken 0 (the assist partner wakes it, 1) |
| 4 Hazards | the acid fixtures | acid hurts 0; against a persistent patch she holds short and the stall is **pinned as a known limit** (no safe way over a live patch); a draining patch costs 7 graph rebuilds and she reaches Carl in 300 ticks or fewer |
| 5 Juicer | `JuicerWho::Companion`, seeds 0..8 | deaths 0, max stuck 17 ticks, hurts 39 median / 44 worst, she never attacks or blinks (section 9) |
| 1 Hoarder (listed in the manifest today), per-seed stuck ticks, AI deaths, hazard hits in every other fixture | **NOT RUN** | |

## 8. Gate checklist (plan section 10)

PR numbers are merged PRs on main. "Final head" evidence is in the PR body of this PR. Status words: **MET** (evidence cited), **UNMET** (the review found the item not satisfied; the closing PR is named), **NOT RUN** (allowed by the item or owner-only), **owner**. The statuses follow the independent gate review.

| Item | Status | Evidence |
| --- | --- | --- |
| **1. Carl complete** | **UNMET: no gauntlet use in `f01_late`** (closing work: PT-GATE-C, one test) | CP-A #183, CP-B1 #203, CP-B2 and CP-C #210, CP-D #228, CP-E #233 (evidence in `docs/evidence/carl-prog-01/cp-e`) are merged. Dynamite in `f01_late`: the `carl_kit` pilot throws 1 to 2 times per run in room 6 (`scorecard.md`). **No pilot, test or capture swings the gauntlet in any `f01_late` room**; stagger, pierce and poise are proven only in the CP-D fixtures outside `f01_late` |
| **2. Donut complete** | **UNMET: rooms default to `cf02_assist` (room 5: the follow-and-jump bot), `cf02_free` unreachable** (closing work: PT-GATE-C) | The merges are all there: DM-A #186, DM-B #199, DM-C #222, DP-A #197, DP-B #213, DP-C #223, DP-D #225, DP-E #224 (merged, not cut), DP-F #226; CA2-A #198, CA2-B1 #205, CA2-B2 #213, CA2-C #220. But the partner a human P1 gets in rooms 2, 4 and 6 is `cf02_assist` and the launcher refuses `--companion` there (section 1), so the CA2 partner and its hazard feed never reach the owner in a room. Human P1 and P2 on pads: **owner** |
| **3a. `f01_late` launches every listed room** | MET | `owner_commands_check.txt` (every manifest room launches) |
| **3b. One `CapabilityGranted` per grant in the PLAYTRACE journal** | **UNMET: room 1 is listed** (closing work: PT-GATE-C unlists it) | Juicer: MET, [`GATE_A.md`](GATE_A.md) sections 3 and 4 (two lines on tick 3175; the Boiler, hazard and room 6 sessions hold none). **Dynamite: room 1 (the Hoarder) is listed in the manifest today**, and the placeholder-zone fixture `carl_prog_story_event_grants_dynamite` is allowed only if room 1 is **not** listed. Until PT-GATE-C removes the `hoarder` row and re-pins `MANIFEST_HASH`, the placeholder is not valid gate evidence, and a live `CapabilityGranted` for dynamite at her defeat is NOT RUN (no pilot defeats her) |
| **3c. Minimum room set, honest milestones** | MET (room 4 keeps its place only if item 5 closes for it) | Rooms 2 (BOIL-E #227 content, BOIL-F evidence #233), 5 (PT-SCENE #211, PT-SCENE-B #234), 6 (PT-MASTERY #232, PT-TUNE #236) and room 4 at HAZ-B (#231, after HAZ-A2 #214 and #229). Milestones: room 2 setpiece, room 4 greybox, room 5 boss, room 6 greybox. Not listed: room 0 (the corral: no CORRAL-F evidence, no CORRAL-AUTO gate run), room 3 (the Dozer track is deferred), rooms 4a and 4b (the Ring, optional) |
| **4a. Goldens byte-identical** | MET | This PR changes no simulation code and no golden (`git diff` touches a test-only module, scripts and evidence); the full suite and `scripts/ci_floor01_combat.sh headless` pass on the head (PR body) |
| **4b. New fixtures: committed hash, replay twice, headless and render loop** | **UNMET** (closing work: PT-GATE-C aligns the room pilots with the fixtures, PT-GATE-D replays all 63 in the render loop) | The 27 Juicer and 36 arena fixtures are library-only; no route replays them in the render loop. The Juicer has one render-loop match (`--room-pilot juicer` equals `juicer_pilot(CarlSolo, 0)`). **The arena has none**: `pilot_hashes_mastery.txt` and `capture_lines_b.txt` pin the window room pilot, which drives the follower bot, not any arena fixture (section 6) |
| **4c. One recorded live session per listed room, strict** | MET (scripted bot); human NOT RUN | Rooms 2, 4, 5: `playtrace_sessions.txt`. Room 6: `playtrace_session_mastery.txt` (1300 ticks, `PASS scope=full`, `run_kind ScriptedBot`). Human sessions: **owner**. Session files are not committed (the store is ignored; the scripts reproduce them) |
| **5a. Direct routes with props untouched** | **UNMET** (HAZ-06; closing work: PT-GATE-C) | BOIL-01 met: `the_direct_route_reaches_the_exit_with_every_prop_untouched` and the live `direct` room pilot. **HAZ-06 `haz_room_direct_route` does not exist** (specified under the HAZ-C fixtures; room 4 was listed at HAZ-B without them), and the `direct` room pilot is defeated 3 times. HOARD-02: `hrd_direct_clear_solo` and `hrd_direct_clear_donut` exist (room 1). DOZR-01: room 3 not listed |
| **5b. Per-room pilots at seeds 0..8, no hero stuck over 300** | **UNMET** (closing work: PT-GATE-C) | Room 2: the per-seed pilots run the **empty greybox** (`Roster::Inert`), not the live room, and no companion pilot exists; the live room has only three solo-Carl runs at one seed. Room 4: none for any mode. Room 5: all three modes, max stuck 17 (met). Room 6: all three, max stuck 95 (met) |
| **5c. No-softlock and reachability validators** | **UNMET: Boiler only; no hazard-room reach matrix** (closing work: PT-GATE-C) | `the_reach_matrix_covers_every_class_and_state_and_all_pass` (`src/encounter.rs`) covers the Boiler; no reach or no-softlock assertion covers the hazards sidecar. The DP-Q45 sweep `dpq45_every_room_with_a_high_surface_declares_a_lane_ceiling` (`src/donut_prog_fixtures.rs`) exists |
| **5d. AUTOPLAY coverage of the setpiece rooms** | NOT RUN (allowed by the item) | PT-AUTOPLAY is deferred (plan section 11) |
| **6. Fairness** | MET | K6: `haz_acid_not_evadable` (`src/hazards_fixtures/tests_b.rs`), `hazards_are_data_and_never_evadable` (`src/encounter.rs`), `hrd_not_evadable`. K3: `carl_prog_dyn_no_friendly_fire`, `carl_prog_dyn_hero_damage_policies_share_one_vocabulary` (`src/combat_scene/thrown.rs`), `the_rot_blast_is_a_blast_profile_with_no_core_and_all_heroes`. K8: `k8_a_nest_nobody_threatens_is_an_error` and its siblings. K5: room 3 not listed |
| **7a. One at-speed GIF per ability** | **UNMET at 640x400** (closing work: PT-GATE-D) | All nine abilities are mapped at 640x360 (table below). The item requires 640x360 and 640x400; 640x400 exists only for the throw, the pierce and the arena wave |
| **7b. Direct, tactical and spectacular route per setpiece** | **UNMET** (closing work: PT-GATE-D) | Boiler: direct, valve and chain (`docs/evidence/f01-boiler`, #233). **Room 4: a first look only, no tactical or spectacular route.** Juicer: the grant clip only. The Hoarder: none |
| **7c. The two grant moments** | MET for the Juicer; dynamite follows 3b | [`GATE_A.md`](GATE_A.md) section 3 (`grant-juicer-gauntlet-blink.gif`, `grant-dynamite-placeholder-zone.gif`). The dynamite clip counts only once room 1 is unlisted (PT-GATE-C) |
| **7d. A mastery-arena wave** | MET | `wave-mastery-w2.gif` (640x360) and `wave-mastery-w2-640x400.gif`: W2, ticks 560 to 854, 6 ticks a frame (100 ms, game speed), real Bevy, window hash equal to the headless hash, two generations byte-identical. It shows the follower-bot fight (section 6) |
| **7e. Frame time, gore preset 4** | **UNMET: the Boiler at 640x400** (closing work: PT-GATE-D); hardware NOT RUN | Boiler: `perf.txt` (640x360 only). Arena: `perf_mastery.txt` (640x360 and 640x400), table below. xvfb under the item's letter; **real-hardware frame time: NOT RUN** |
| **7f. A real two-pad 2P run** | NOT RUN (allowed by the item; owner) | needs two pads |
| **8. Owner packet** | MET | this file |
| **9. Independent review, game feel pending** | MET by the gate review; game feel **pending** | nothing is accepted |

**Gate item 7a: one at-speed GIF per ability.** "At speed" means one frame per N ticks at N/60 s (`scripts/room_gif.py` rounds the delay to 10 ms). Several committed GIFs are deliberately slowed or sped up, so the ones marked "regenerated" are new at-speed captures from the same fixtures.

| Ability | GIF | Notes |
| --- | --- | --- |
| Dynamite throw | `docs/evidence/carl-prog-01/cp-b2-c/throw-standing-1x.gif`, `docs/evidence/carl-prog-01/cp-e/throw-standing-640x400-1x.gif` | 6 ticks a frame at 100 ms: game speed |
| Dynamite ring | `docs/evidence/carl-prog-01/cp-e/reach-ring-outer-launch-1x.gif` | 3 ticks at 50 ms; `--debug-combat` draws the ring |
| Dynamite chain | `ability-chain.gif` (regenerated) | the committed `cp-b2-c/chain-1x.gif` plays at half speed |
| Gauntlet pierce | `docs/evidence/carl-prog-01/cp-e/pierce-goblin-1x.gif` and `-640x400-1x.gif` | 2 ticks at 30 ms; the control GIF is next to it |
| Donut agile motion | `docs/evidence/donut-mobility-01/dm_a_run_race_1x.gif`, `dm_a_jump_race_1x.gif` | 2 ticks at 30 ms |
| Reflex | `ability-reflex.gif` (regenerated) | `cf02_dm_reflex_cues`, ticks 712 to 799; the committed `dm_c_gif3_reflex.gif` is half speed at 2x |
| Magic Missile | `ability-magic-missile.gif` (regenerated) | `cf02_assist_donut_missiles_goblin`, ticks 8 to 70; cast by the AI partner (a human cast: owner). The committed `anim-donut-03/vfx` GIFs are Pillow models, not Bevy captures |
| Light on Your Feet | `ability-light-on-your-feet.gif` (regenerated) | `donut_prog_arena_ledge_boost`, ticks 28 to 100; the committed side-by-side GIF is slowed |
| Puddle Jumper | `ability-puddle-jumper.gif` (regenerated) | `donut_prog_arena_blink_tour`, ticks 30 to 108; the committed cycle GIF plays at double speed |

All regenerated GIFs are 640x360. Their window and headless final hashes are equal (`capture_lines_b.txt`), and two generations were compared byte for byte. **640x400 versions: only the throw, the pierce and the arena wave exist; the rest are NOT RUN.**

**Mastery arena frame time** (`bash scripts/make_f01_gate_b_perf.sh`; `bash scripts/room_perf.sh mastery mastery 1191`, N = the room pilot's exit tick 1190 plus 1; gore preset 4; lighting on; 3 runs per view). **xvfb virtual display, not real hardware, and not a handheld: the frame interval is bounded by the X copy path.** The GPU column (summed timed render passes) and the pilot tick are a **lower bound** on frame cost: they leave out the main-thread CPU of the ECS, extraction and batching, and the HUD. The readings below were taken with the dev profile on an AMD Ryzen 9 5950X with a discrete GPU reported by wgpu.

| View | Run | median ms | p95 ms | p99 ms | max ms | mean ms | GPU ms mean (passes) | pilot tick us mean | load average after |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 640x360 | 1 | 16.63 | 18.54 | 20.99 | 46.12 | 16.67 | 1.310 (4) | 29.9 | 2.42 3.46 4.99 |
| 640x360 | 2 | 16.65 | 18.33 | 19.43 | 22.82 | 16.67 | 1.271 (4) | 31.1 | 1.88 3.25 4.87 |
| 640x360 | 3 | 16.62 | 18.64 | 21.10 | 36.01 | 16.67 | 1.304 (4) | 33.9 | 1.67 3.08 4.78 |
| 640x400 | 1 | 16.64 | 18.53 | 19.47 | 32.47 | 16.67 | 1.368 (4) | 29.5 | 2.39 3.13 4.74 |
| 640x400 | 2 | 16.64 | 18.35 | 20.32 | 43.52 | 16.67 | 1.415 (4) | 30.0 | 2.05 2.99 4.65 |
| 640x400 | 3 | 16.64 | 18.39 | 19.50 | 37.05 | 16.67 | 1.441 (4) | 30.6 | 1.77 2.85 4.56 |

Every run is 1191 frames (pilot ticks 0 to 1190) with 847 death sprites spawned and up to 110 visible at once. The median sits at the 16.6 ms of the xvfb copy path (a hypothesis, not measured), so the p95, p99 and max columns and the GPU and tick columns carry the information. Machine load was 1.7 to 2.4 during the runs. The Boiler's numbers are in `perf.txt` and [`GATE_A.md`](GATE_A.md); an earlier arena run on a loaded machine (load average 7 to 10, p99 up to 46 ms) is not committed.

## 9. Known gaps

From the backlog's owner game-feel notes, the PT-TUNE follow-ups and #237:

**The arena (PT-TUNE and PT-MASTERY)**
- **The D3 margin is thin:** the bot's median clear is 667 ticks against the 647 target, a margin of 20 ticks; the 900 to 3000 human band is unmeasured (OQ4).
- **The seeds give only 2 distinct arena outcomes for `carl_solo`** (clear ticks 667 and 684), 3 for the companion: 9 seeds cover about 2 fights, so "9 seeds" overstates the coverage.
- **The Donut kit pilot stands still under fire.** `donut_fights` and `donut_kit` clear 0 of 9 (7 and 8 defeats): she is idle from about tick 500, takes a hurt every 95 ticks, and the stuck rule counts only walking input without motion, so it does not see it (PTT-Q21). Follow-up: a stall measure. A Donut-led clear is **unproven**.
- **The companion Donut is idle against the Juicer** (the `cf02_free` fixture; the live room launches the follow-and-jump bot instead) and takes most of the run's hurts, never attacking, blinking or retreating (CA2-Q35, `CA2-FU-BOSS`, COMPANION-AI-01E).
- **CA2 misses:** M2 (shadowing), M4 (steals on 2 seeds) and M13's comparison with the assist partner; M7, M8, M9's cause split, M11 and the M14 planning time are not measured in the arena.
- **The room pilot differs from the fixture pilot** (section 6): the live scripted run fights with the follower-bot partner and is defeated once; a 6-tick residual is unexplained.
- The arena reuses the stand-in art and has no presentation of its own; no human has played it.

**Rooms and presentation**
- **Gate failure (item 2): the owner gets the wrong partner.** `--scene f01_late --companion ...` is refused, and cf02 rules default to `cf02_assist`: rooms 2, 4 and 6 give the `cf02_assist` companion and room 5 the follow-and-jump bot, never the CA2 partner or its hazard feed. Plan sections 8 and 9 make `cf02_free` the room default. Closing work: PT-GATE-C (track a user-supplied fixture separately, default `cf02_free` in room scenes).
- Boiler: B3 can wake inside the steam lane; the goblin wake hook is a follow-up (`BOIL-FU`).
- Hazards: the Pocket C relayout, the missile window of 7 press ticks on the rise and 8 on the fall, Donut's run-through of a still `H` 68 sticker is hit once; **no HAZ-06 direct-route fixture was found**; no per-seed pilots; no tactical or spectacular route GIFs.
- The Hoarder's room has no presentation (nothing drawn for her, her door or her gate until HRD-B). **It is listed in the manifest today**; PT-GATE-C unlists it (gate item 3b).
- Production art is stand-in (K10); the Ring, the corral and the Dozer are not in the slice.

**Mechanics notes**
- Gauntlet: the haymaker-only pierce (F-12); the pierce spark is faint at 1x.
- The scripted coop blink pilot had more deaths than its no-blink control (7 against 0 over seeds 0..8), likely the swat-hold rule; the boss-charge blink guard is dead in the shipped stack (#226).
- The 720-tick reflex is not ready for the opener (a human-style Donut evaded 0 of 23 at 720 from tick 0, #222).

**Evidence limits (#237 and this PR)**
- Frame time is under xvfb only, with a dev build; real hardware, a Steam Deck, a 1080p window and a release build are **NOT RUN**. The perf reading is a lower bound (section 8, item 7e).
- No human, no pad, no real two-pad 2P run; no human-recorded session; the sessions are scripted bots.
- The Hoarder dynamite grant in a live recording; the live `--companion cf02_free` Juicer launch is not verified end to end.
- AUTOPLAY coverage of the setpiece rooms, per-ability 640x400 GIFs, the tactical and spectacular routes, the render-loop replay of the 27 Juicer and 36 arena fixtures.
- Tools: `perf.txt` and `GATE_A.md` predate PT-GATE-B; the GIF generation is byte-stable under the 60-frame first settle, and the generators refuse a near-black first frame.

## Regenerate

```bash
cargo build --locked --bin floor01_scene --bin playtrace --bin corral_eval && bash scripts/export_assets.sh   # xvfb-run and Pillow
bash scripts/make_f01_gate_b_perf.sh        # perf_mastery.txt, on a quiet machine (wall-clock)
bash scripts/make_f01_gate_b_evidence.sh    # wave and ability GIFs (twice, compared), the room 6 session, scorecard.md, pilot_hashes_mastery.txt
bash scripts/check_f01_owner_commands.sh    # owner_commands_check.txt
python3 scripts/check_docs.py               # verifies SHA256SUMS
```
