# F01 playtest owner packet (PT-GATE-D)

> **Start with [`docs/04-production/f01-playtest-ready-01.md`](../../04-production/f01-playtest-ready-01.md) (PLAYTEST-READY-01, 2026-11-27).** It is the owner's single entry point for the 17-room floor run: how to run, the room order, what is missing, the checklist, every adopted default and the art notes. The sections below are the PT-GATE-D record (K1 to K19, scorecards, the gate checklist); their launch tables and room lists predate the 17-room run, so where they differ the entry point is current.

> **Regenerated for ROOM-PORTALS-01 (the room exit door of rooms 2, 4, 4a, 4b and 6).** `wave-mastery-w2.gif` and `-640x400.gif` (`scripts/make_f01_cue_gifs_evidence.sh`), `route-room4-direct*.gif` and `route-room4-tactical*.gif` (`scripts/make_f01_gate_e_evidence.sh`) and `route-room4-spectacular*.gif` (`scripts/make_f01_gate_d_evidence.sh`) were regenerated, two generations byte-identical each, because their views reach the exit, now drawn as the room exit door instead of the Guild door. Their capture lines are byte-identical: `capture_lines_b.txt` and `_e.txt`, and the `GATE_D_ROUTE` lines of `capture_lines_d.txt` (same state hashes, window equal to headless). The four `GATE_D_CAPTURE` deck lines of `_d.txt` no longer reproduce on `main` (their fixtures' hashes moved since PT-GATE-D); they and the deck GIFs were left as they were, a separate refresh. The room 4 route GIFs had not been regenerated since the danger FX art (#263), so they also show it now, and the room title of the HUD (#270) shows in them too.

> **Regenerated for FA-Q38 (PR #259).** `wave-mastery-w2.gif`, `wave-mastery-w2-640x400.gif`, `route-room4-direct.gif` and `route-room4-direct-640x400.gif` were regenerated (`scripts/make_f01_cue_gifs_evidence.sh`, two passes each, byte-identical) so that they show the i-frame cue: a hero is hurt in each of them. Their window hashes in `capture_lines_b.txt` and `capture_lines_e.txt` differ from the PT-GATE-B and PT-GATE-E values because the simulation on `main` moved between those gates and this PR (PR-B to PR-D, #248 to #252), not because of the cue: the window hash equals the headless hash, and a headless run of `main` without this PR ends on the same four hashes. The tactical and spectacular route GIFs and every other clip here came out byte-identical or are not hurt-hero clips and were not touched.

The entry point for the owner's playtest of the late Floor 1 slice, and the evidence for the "playtest ready" gate of [plan section 10](../../07-agents/f01-playtest-integration-plan.md). It collects launch commands, what to look for in each room, the adopted defaults, the open questions, the scripted scorecards, the CA2 metrics of the closing arena, the gate checklist and the known gaps.

**Game feel is pending everywhere.** Nothing in this packet was played by a human, on a pad or on real hardware, and nothing is a game-feel verdict. Every number is a scripted-pilot or fixture measurement. A metric that cannot be derived is written **NOT RUN**, never estimated. **Gate status: playtest ready by the letter of plan section 10, on scripted evidence.** No item is UNMET. PT-GATE-D left two UNMET, 4b (the render-loop replay of #240's runs) and 7b (room 4's tactical and direct routes); PT-GATE-E (#244) closed both with evidence (section 8), and the independent top-tier reviews of #242 and #244 passed with no blocking finding and no logic or design defect. Of the 21 rows of section 8, 19 are MET and 2 are MET as NOT RUN where the item allows it (5d AUTOPLAY, 7f the two-pad run). What stays owner-only: real hardware, a real two-pad 2P run, and human play and game feel. This is a scripted-evidence verdict, not a game-feel verdict. The code the gate review asked for landed in #240 and #241 (the room partner, the room 1 listing, aligned room pilots, the room 2 and room 4 gate fixtures, the gauntlet test); PT-GATE-E was the evidence: the render-loop replay of all 63 aligned runs, the room 6 evidence regenerated on the aligned pilot, the 640x400 GIFs, room 4's spectacular route, the Boiler at 640x400, a room 6 session on the human default path and the live room 2 and room 4 scorecard rows. Nothing in this packet is a game-feel verdict.

Files: [`scorecard.md`](scorecard.md) (generated tables), [`GATE_A.md`](GATE_A.md) (the PT-GATE-A evidence: tooling, Boiler perf, grants, the room 2, 4 and 5 sessions), [`perf.txt`](perf.txt) and [`perf_mastery.txt`](perf_mastery.txt), [`owner_commands_check.txt`](owner_commands_check.txt), [`capture_lines_b.txt`](capture_lines_b.txt), [`pilot_hashes_mastery.txt`](pilot_hashes_mastery.txt), [`playtrace_session_mastery.txt`](playtrace_session_mastery.txt), [`playtrace_session_default_path.txt`](playtrace_session_default_path.txt), [`render_replay_lines.txt`](render_replay_lines.txt) (the render-loop replay of the 153 aligned runs), [`capture_lines_d.txt`](capture_lines_d.txt) (the 640x400 and room 4 spectacular route captures), [`capture_lines_e.txt`](capture_lines_e.txt) (room 4's tactical and direct route captures), the GIFs, and `SHA256SUMS` (verified by `scripts/check_docs.py`).

## 1. Launch commands (plan section 9)

Release build, from the repository root, after `bash scripts/export_assets.sh`:

```text
cargo run --release --bin floor01_scene -- --scene f01_late --f01-run --scale 2   # the whole floor in one session (see below)
cargo run --release --bin floor01_scene -- --scene f01_late --scale 2
    [--f01-room hoarder|boiler|hazards|juicer|mastery]    # jump to a room with its story pin (default: boiler); room 1 is the Hoarder (listed again, FA-Q24; also `--scene f01_hoarder`)
    [--f01-loadout story|all]                              # story implies --carl-prog --donut-prog; all = TEST LOADOUT
    [--companion off|cf02_assist|cf02_free|v0]             # the AI partner: rooms 1, 2, 4, 5 and 6 (and --scene f01_hoarder) default to cf02_free; `--companion off` restores the follow-and-jump bot
    [--debug-combat] [--no-record]                         # recording is on by default (PLAYTRACE)
tools/record-run.sh floor01_scene --scene f01_late --scale 2    # window video plus audio
```

**Play the full floor in one session (F01-RUN).** Add `--f01-run`: the game starts in the first listed room (room 0, the Infernal Corral, since CORRAL-F) and, each time you walk out of a room's exit, shows a facts-only panel (the room's ticks and clock, deaths, restarts, what was granted there, the run so far, the next room; no score, no rating). Press Enter, A (Cross, any pad) or Start (P1's pad) to go on (the panel ignores presses for its first half second, so a jump held through the door does not skip it): the game restarts itself in the next listed room (a short reload, one window at a time), carrying what the story granted (the Hoarder's dynamite is held in every later room; the Juicer's gauntlet and blink reach the mastery arena). After the last listed room a floor-end panel totals the run and lists what each hero holds; confirming quits. Esc quits at any time; Select (tap or hold) still respawns or restarts the current room. The run follows the manifest's room list, so rooms listed later (the Dozer) join it by themselves. The corral carries no story capability (room 0 grants nothing); it hands the Hoarder an empty story ledger, so her room starts from her own pin (CF-AD3). Under `--f01-run --f01-loadout all` the rooms chain the same way with the TEST LOADOUT and carry nothing. `--f01-run --f01-room juicer` starts the run further in, from that room's story pin. From the room after the corral on, the per-hero viewer counts are live and the best kills (a thrown dynamite multi-kill, a llama kill) get the slow-motion moment (AUDIENCE-RUN, game feel pending): `--audience=off` and `--cine-showcase=off` switch them off, and `--f01-run --f01-room hoarder` opens a later room directly, at the baseline of 250 viewers per hero. The restart count includes the Juicer room's defeat retries (a defeat there reads as a death and a restart). Each room is still its own PLAYTRACE session; a carried room's session stores the carried ledger (`f01_carry`) and verifies strict. `--f01-run` takes no `--cf02-seed`.

**The HUD names the room (F01-HUD-ROOM).** The top strip shows the room's title and your attempt (`THE HOARDER · TRY 2`; each restart adds one) instead of `FLOOR 01` (in the Juicer's room the boss bar replaces it). A **TEST LOADOUT** tag under it marks `--f01-loadout all` (every capability, no story grants); a story room launched on its own with `--f01-room` starts from its story pin and has no tag. The room stills and GIFs captured before PR #270 under `--scene f01_late --f01-room X` still show `FLOOR 01` in the top strip: they predate the room title, which is not a regression.

**The AI partner (PT-GATE-C1).** In the `f01_late` room scenes of rooms 1, 2, 4 and 6 (`--f01-room hoarder|boiler|hazards|mastery`) the other hero is the autonomous **`cf02_free`** partner by default (plan sections 8 and 9), and `--companion off|cf02_assist|cf02_free|v0` overrides it (the launcher now tells a fixture the user supplied from the placeholder fixture a room scene sets; `--fixture X --companion Y` is still refused). A scripted run (a room pilot, a capture, a headless smoke) never runs the live partner. **Room 5 (FA-Q08, PR #250):** the Juicer's room (`--f01-room juicer`, the boss scene) accepts `--companion` and defaults to `cf02_free` as well, because FA-Q07 holds on the final head (the autonomous partner fights the Juicer on 9 of 9 seeds, Carl takes the kill, her median hurts are 0, she retreats on 9 of 9); `--companion off` restores the follow-and-jump bot, and a fixture the user supplied is still refused. **Still open:** the arena line `--cf02 --cf02-engineer --carl-dynamite --companion cf02_free` keeps the `cf02_assist` default.

To play two players, connect a second pad and press Start: P1 plays Carl and P2 plays Donut. Select restarts the room. Start on P1 quits.

**Controls.** The keyboard drives P1 (Carl); every connected pad slot uses the same pad bindings (P1 on the first pad, P2 on the second pad after its Start). Pad bindings follow [the controller contract](../../01-design/mvp-controller-contract.md). The equipped special is **L on the keyboard and East (Circle on DualSense, B on Xbox) on a pad**; before the fix of this table it was unreachable from live input.

| Action | Keyboard (P1) | Pad (every slot) |
| --- | --- | --- |
| Move | A / D or Left / Right arrows | Left stick or D-pad |
| Jump | Space | South (Cross / A) |
| Primary attack (jab) | J | West (Square / X) |
| Physical attack (kick) | K | North (Triangle / Y) |
| Equipped special (Carl's dynamite, Donut's blink) | L | East (Circle / B) |
| Down (stomp or dive under the CF02 rules) | S or Down arrow | D-pad down, or left stick down |
| Swap hero (solo) | Q | L1 / LB |
| Restart the room (a tap respawns, a long hold restarts in a manual room scene) | R | Select |
| Quit | Esc | Start on P1's pad (Start on a second pad joins P2) |

The special does something only where the hero holds it: Carl's dynamite from room 2 on (the `f01_pin_post_pile_boss` pin; the HUD shows it as BOOM with the special button), Donut's blink where her grant or the loadout gives it (room 6, or `--f01-loadout all`). A press without the capability is inert. The lab menu and the end panel take the input first and block it.

**Listed rooms.** The manifest `assets/levels/f01_late/manifest.json` lists `corral, hoarder, boiler, dozer, hazards, ring_court, ring_split, juicer, mastery`, and the game prints that list: every room of the story order. **Room 0 (the Infernal Corral) is listed first since CORRAL-F** at milestone `greybox` (`MANIFEST_HASH` `0x727fc5cf5fd62109` to `0xbb11225ec03d7e0b`); see [`../floor01-corral/corral-f`](../floor01-corral/corral-f/README.md). **Room 3 (the Dozer revenge) is listed since DR-A** at milestone `greybox` (`MANIFEST_HASH` `0xbb11225ec03d7e0b` to `0xa33383ff73989d01`); evidence in [`../f01-dozer-revenge`](../f01-dozer-revenge/README.md). **Rooms 4a and 4b (the Broken Ring) are listed since RING-A/B wire** at milestone `greybox` (`MANIFEST_HASH` `0xb45c744a126ee87c` to `0xf50148114f3b5ed7`); they are launchable but not gate-listed; the RINGW-F1 companion finding is fixed (merged, game-feel review pending; see `docs/03-tech/floor01-scene.md`) and gate listing is a separate step. **Room 1 (the Hoarder) is listed again (FA-Q24, PR-E).** This reverses the earlier adopted default PTG-AD1 (PT-GATE-C1: unlisted, the smaller option): the owner's playtest packet now covers the room, because it has a defeating room pilot, a presentation and a strict session with exactly one `CapabilityGranted`. `--f01-room hoarder` launches it under the story or the TEST loadout (`--f01-loadout all`), `--scene f01_hoarder` still launches it with no loadout label, and `story_order` names it. `MANIFEST_HASH` is re-pinned (`0x24bc1268822f41f7` to `0xb45c744a126ee87c`, reason: the `hoarder` row is back in the manifest and its `status` text says so). Her drawing was **stand-in** in PR-E (K10: the HOARD-01 frames labelled "STAND-IN UNTIL HOARD-02" plus a labelled greybox door, entry gate, bank A, J-B post and lair mound). **Since PR #258 she and her room are drawn with prototype pixel art** (HOARD-02, 29 frames, and the AREA-HOARDER-01 kit: mound, bank A, J-B post and stack, door, gate, heaps; independent top-tier art review passed, owner art review pending) and no STAND-IN label is left; bank A is solid and the J-B stack can fall on her since PR #260 (HRD-B, HRD-C). See [`../f01-hoarder-presentation`](../f01-hoarder-presentation/README.md) and [`../hoard-02`](../hoard-02/README.md). The per-room art status (production art against stand-in) is a table in `docs/04-production/backlog.md`.

| Room | Command suffix | Gate status |
| --- | --- | --- |
| 2, the Goblin Boiler | `--f01-room boiler` (also `--scene f01_boiler`) | listed |
| 3, the Dozer revenge | `--f01-room dozer` (also `--scene f01_dozer`) | listed (greybox, DR-A): the approved DOZ-01 machine; the rear gate, the buffer stop and the damage marker are labelled stand-ins until DOZ-02. DR-B: the stop frame (jab the brace at x 832 twice, one stick of dynamite, or two Magic Missiles; it lands 37 ticks later and stuns the machine on its front), drawn with labelled frame, brace, shadow, remnant and glance stand-ins; room pilots `--room-pilot dozer:run`, `dozer:close_call`, `dozer:wait`, `dozer:brace` (the reversal) |
| 4, slimes and Rot Stickers | `--f01-room hazards` | listed |
| 5, the Juicer | `--f01-room juicer` (also `--scene boss_f01 --carl-prog --donut-prog`) | listed |
| 6, the mastery arena | `--f01-room mastery` | listed |
| 1, the Hoarder | `--f01-room hoarder` (also `--scene f01_hoarder`) | listed (FA-Q24); prototype pixel art since PR #258 (owner art review pending), defeating room pilot `--room-pilot hoarder:carl_solo:SEED` (9 of 9) |
| 4a and 4b, the Broken Ring | `--f01-room ring_court`, `--f01-room ring_split` | launchable (greybox geometry; Broken Ring prototype art since AREA-RINGS-01, owner art review pending), not gate-listed (RINGW-F1 fixed, game-feel review pending); the court's closed shutter is a dark stone slab under the walkway between two guide jambs: cross it over the step and the walkway. `--f01-run --f01-room ring_court` (or `ring_split`) starts a floor run inside the room |
| 0, the Infernal Corral | `--f01-room corral` | listed (CORRAL-F, greybox), first room of `--f01-run`; room pilots `--room-pilot corral` (M0 to M3, `corral:WHO:SEED`) and `--room-pilot corral_rush` (B's rush on a parked Carl, an evidence plan) |

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

**Verified on the final head.** [`owner_commands_check.txt`](owner_commands_check.txt) holds one line per command above (and the refused lines), run by `scripts/check_f01_owner_commands.sh` in the real window under xvfb for 30 ticks (the debug binary, no audio, no recording; the release build takes the same arguments). `ok` means the process started and exited 0; `refused` is an expected refusal: an unknown room (`cellar`: no committed room is unlisted any more), a scene the Ring rooms do not have (`--scene f01_ring_court`), or `--companion` next to a fixture the user supplied. **NOT RUN:** the commands in a release build, `tools/record-run.sh` (needs a display, `xdotool`, ffmpeg), a second pad.

## 2. What to look for, per room

- **Room 2, the Goblin Boiler (setpiece).** Jab the steam valve twice (a steam lane telegraphs 24 ticks, then bursts 72), jab the brace to drop the weight on goblins under its footprint, kick a goblin into the valve for the chain, and use the drum. The direct route must stay safe with every prop untouched. Look for: whether the props read as threats you can use, whether the steam telegraph is fair, the Engineer, and the props' look (the Goblin Workshop kit, prototype art, replaced the labelled stand-ins: PR 256). Retuned after the owner's first run ("tooooo easy": the AI Donut did the killing): the second goblin now wakes as you leave the spawn (so B1 is a staggered pair), the Engineer arrives with B3's goblin as a pair, Donut keeps out of the sight of sleeping enemies until you have woken them (FA-Q36), never shoots one that is not awake (FA-Q35), and leaves a one-hit-point enemy near you for you to finish (200 px) until it hurts someone or 150 ticks pass (FA-Q37). Look for: whether Donut visibly softens and staggers but leaves the finish nearby, and whether the first pass takes 90 to 180 seconds. Known: B3 wakes at x 560, before the steam lane, since PR-C (FA-Q20, section 9); B3 waits at the drum's face (FA-Q31, deferred: a non-solid drum needs engine code).
- **Room 4, slimes and Rot Stickers (greybox).** Three isolated pockets: a slime with an acid pool, a Rot Sticker overhead, a crossover. Look for: the slime tell and its punish window, that acid bars walkers, the Rot Sticker's 24-tick warning and its blast on both heroes, and that nothing here can be reflex-evaded (K6). The `direct` room pilot does not dodge hazards and is defeated; the PT-TUNE pilot clears the room (section 5, section 8 item 5), `--room-pilot haz_spectacular` plays the dynamite route, `--room-pilot haz_tactical` baits the second Rot Sticker over the slime, and `--room-pilot hazards:carl_solo:0` is the melee route that does not dodge (section 8, item 7b); a human should clear it.
- **Room 5, the Juicer (boss).** Carl holds dynamite and Donut the boost (the story pin). After the defeat two notices grant the gauntlet (HEAVY FIST) and the blink (PUDDLE JUMPER). Look for: dynamite on the boss, whether the grants read, and the partner: since PR #250 (FA-Q08) the live `--f01-room juicer` defaults to the autonomous `cf02_free` Donut, who fights the boss (FA-Q02..Q07) and leaves Carl the kill; `--companion off` restores the follow-and-jump bot. The boss gate is still a plain code-drawn rectangle (no art yet).
- **Room 6, the mastery arena (greybox, no presentation of its own).** Everything is held (the `f01_late_mastery` pin, no grant, no trigger). Wave 1 is the brawl lane: goblins, a slime with its acid pool and a spitting llama. Wave 2 is the llama yard: goblins in the matador lane, the Engineer with a llama, a second llama, and a Rot Sticker under the right perch. The exit gate opens when the roster is defeated. Look for the "powerful enemies" feel (overlap, the slime beat, the Engineer with a llama, clear time) against the questions of section 4.
- **Room 1, the Hoarder (boss, prototype pixel art since PR #258).** `--f01-room hoarder`: walk in past bank A (a solid since PR #260: jump onto it or over it), cross x 432 and the entry gate closes behind you (at x 208); she rises from the lair mound over a 90-tick reveal, then swipes (a 34-tick tell: the gather, then the rear-up) and shoves (a 40-tick crouch, then a lunge). Jump the swipe in place, jump the shove as she lunges, and punish her recovery. Her defeat opens the exit door and grants dynamite (one notice, K1). Look for: whether the swipe and the shove read before they land, whether the fight is dread and not comedy, whether the J-B stack (two hits on its post, or her shove into it, drops it on her for an Exposed punish) reads as a payoff, and how long a clear takes (`--room-pilot hoarder:carl_solo:0` clears in 1381 ticks on the PR-E head, about 23 s, and 4 ticks later since PR #260; a human is slower). Since PR #258 she is drawn from the HOARD-02 atlas (shove, lunge, jam, pinned, get-up and emerge included) and the room from the AREA-HOARDER-01 kit; the room pilot never draws a shove, a bank slide or a stack fall, so those states have offline composites only (`../hoard-02`). Known open design point: pile hits during her 90-tick Reveal can drop the stack while she has no body (follow-up in flight). Evidence: [`../f01-hoarder-presentation`](../f01-hoarder-presentation/README.md).

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
3. **KC3, only if reached:** whether a guarded corral llama may take a missile's damage without the `Hurt` stagger. Room 0 is listed since CORRAL-F and its CORRAL-AUTO gate runs (see [`../floor01-corral/corral-f`](../floor01-corral/corral-f/README.md)); this question is still open and is not decided there.

The owner is also informed of K1 and K15, and of KC2, KC4, KC8, KC9 (the corral) and the #181 C-A sequencing (plan section 13).

Room 2 retune questions (FA-Q31 to FA-Q35, `docs/01-design/floor-01-goblin-boiler-encounter.md` section 10; none blocks anything):

- **R2-OQ1.** Is Donut's "softens but leaves you the kill nearby" (finish reserve 200 px, FA-Q34) the right feel? Should FA-Q34 and FA-Q35 later apply to every room, not only the Boiler?
- **R2-OQ2.** The drum (FA-Q31, deferred): B3 still waits at the drum's face. Should an engine change make the drum non-solid (heroes and goblins pass through it) or hero-only solid, or is the drum-face fight fine? Would a non-solid drum read as wrong visually?
- **R2-OQ3.** Is a direct-route clear with up to 3 hurts (scripted pilots) the right difficulty, or should the room add a fifth enemy under `waves: true` (opt-in, PTM-AD6)?
- **R2-OQ4.** Is it right that nobody fires at a sleeping goblin (B0 "read the machine"), or should Donut be allowed a deliberate opening shot on the human's input only?
- **R2-OQ5.** With an idle human the room stays asleep until Carl moves (Donut keeps out of every sleeper's sight, FA-Q36). Is that right for B0?
- **R2-OQ6.** Should a reserved enemy be released after 1 hit on a hero (adopted, FA-Q37) or after 2? (After 150 ticks it is released either way.)

PT-TUNE game-feel questions for room 6 (none blocks anything):

- **OQ1.** Does "kill the slime, the pool drains, the goblins rush" read as a trap you sprang, or as unfair?
- **OQ2.** Is wave 1's spit over the pool pressure or nagging? The alternative is the llama in wave 2 with a third goblin in wave 1.
- **OQ3.** Is one Rot Sticker enough for the arena?
- **OQ4. The human clear-time band.** Is 15 to 50 s per clear (ticks 900 to 3000) the "powerful" feel, or too long for a closing room? The scripted base-kit Carl clears in a median of 672 ticks (11 s); the band is for the owner's playthrough, **blank in the scorecard below and NOT RUN here**.
- **OQ5.** With party targeting, Donut gets spat at too. Is that right for the solo AI partner?

## 5. Scorecards

**Scripted reference** (the committed pilots and fixtures at seeds 0..8, median / worst; generated in [`scorecard.md`](scorecard.md), which has every column and the per-seed tables). Deaths are hero defeats, restarts are Select restarts (the pilots never press Select), the time is the exit tick (60 ticks a second), pierces are **NOT RUN everywhere** (no scripted pilot swings the gauntlet and the combat log has no pierce event), and throws, chains, blinks and evades are 0 for every base-kit pilot **by construction**, not a measurement of play (only the room 6 kit pilots press the dynamite and the blink).

| Room and pilot | Clears | Deaths | Hurts | Time (ticks) | Throws / chains / blinks / evades | Max stuck ticks |
| --- | --- | --- | --- | --- | --- | --- |
| 2 Boiler **greybox, no roster** (`Roster::Inert`: no goblins, no Engineer; not the live room), CarlSolo, direct | 9 of 9 | 0 / 0 | 0 / 0 | 308 / 308 | base kit: 0 | 0 |
| 2 Boiler greybox, no roster, DonutBySwap, direct | 9 of 9 | 0 / 0 | 0 / 0 | 286 / 286 | base kit: 0 | 1 |
| 2 Boiler greybox, no roster, CarlSolo, valve then brace | 9 of 9 | 0 / 0 | 0 / 0 | 485 / 485 | base kit: 0 | 1 |
| 2 Boiler greybox, no roster, DonutBySwap, valve then brace | 9 of 9 | 0 / 0 | 0 / 0 | 496 / 496 | base kit: 0 | 7 |
| 2 Boiler **live roster** (`Roster::Live`: goblins and the Engineer), CarlSolo, direct | 9 of 9 | 0 / 0 | 3 / 3 | 862 / 862 | base kit: 0 | 28 |
| 2 Boiler live roster, DonutBySwap, direct | 9 of 9 | 0 / 0 | 3 / 3 | 842 / 842 | base kit: 0 | 28 |
| 2 Boiler live roster, Companion (`cf02_free`), direct | 9 of 9 | 0 / 0 | 1 / 3 | 611 / 882 | base kit: 0 | 120 |
| 2 Boiler live roster, CarlSolo, valve then brace | 9 of 9 | 0 / 0 | 0 / 0 | 667 / 667 | base kit: 0 | 40 |
| 2 Boiler live roster, DonutBySwap, valve then brace | 9 of 9 | 0 / 0 | 0 / 0 | 650 / 650 | base kit: 0 | 40 |
| 2 Boiler live roster, Companion (`cf02_free`), valve then brace | 9 of 9 | 0 / 0 | 1 / 1 | 804 / 804 | base kit: 0 | 82 |
| 4 Hazards, CarlSolo, the direct route (HAZ-06, HAZ-09; melee only) | 9 of 9 | 0 / 0 | 4 / 4 | 739 / 739 | base kit: 0 | 20 |
| 4 Hazards, the same route with the Rot kind removed (HAZ-06) | 9 of 9 | 0 / 0 | 2 / 2 | 636 / 636 | base kit: 0 | 17 |
| 4 Hazards, DonutBySwap | 9 of 9 | 0 / 0 | 4 / 4 | 749 / 749 | base kit: 0 | 17 |
| 4 Hazards, Companion (`cf02_free`) | 9 of 9 | 0 / 0 | 3 / 3 | 724 / 724 | base kit: 0 | 40 |
| 4 Hazards, `haz_spectacular` room pilot (R1 baited, then one dynamite chains R2; one run, no seeds) | 1 of 1 | 0 | 1 | 836 (the scene stops at its exit) | throws 1, chains 1 | not counted |
| 5 Juicer, CarlSolo | 9 of 9 | 0 / 0 | 38 / 43 | 3470 / 3887 | base kit: 0 | 17 |
| 5 Juicer, DonutBySwap | 9 of 9 | 0 / 0 | 38 / 43 | 3445 / 3863 | base kit: 0 | 17 |
| 5 Juicer, Companion | 9 of 9 | 0 / 0 | 39 / 44 | 3470 / 3887 | base kit: 0 | 17 |
| 6 Arena, carl_solo | 9 of 9 | 0 / 1 | 4 / 6 | 759 / 967 | base kit: 0 | 91 |
| 6 Arena, donut_by_swap | 9 of 9 | 0 / 1 | 4 / 6 | 756 / 970 | base kit: 0 | 91 |
| 6 Arena, companion (`cf02_free`) | 9 of 9 | 0 / 1 | 3 / 5 | 886 / 1050 | base kit: 0 | 91 |
| 6 Arena, donut_fights (Donut leads, base kit, FA-Q12 rules) | 9 of 9 | 1 / 1 | 12 / 12 | 1316 / 1343 | base kit: 0 | 28 |
| 6 Arena, carl_kit (dynamite, report-only) | 9 of 9 | 1 / 1 | 5 / 6 | 883 / 1009 | throws 1 / 2, chains 0 | 91 |
| 6 Arena, donut_kit (blink, report-only) | 9 of 9 | 1 / 1 | 9 / 12 | 1320 / 1338 | blinks 1 / 2 | 28 |

The Juicer rows are library fixtures (the `Companion` row uses `cf02_free`, which is also the live room's default partner since PR #250, FA-Q08, section 1). The room 2 live rows and the room 4 rows are the PT-GATE-C2 fixtures (#240: `scene_pilots::run_live`, `hazards_fixtures::pilots`), with their hashes pinned there; [`scorecard.md`](scorecard.md) has every column. Their hurts count **both heroes**: in the room 2 live `Direct` runs the `cf02_assist` partner hits the valve itself (a game-feel note, section 9), and in room 4 the assist partner takes the Rot blast on every seed of the Carl-solo and Donut-by-swap runs (9 of 9) while the `cf02_free` companion takes none (scorecard, AI hurts by source). The stuck figure covers the controlled hero and the `cf02_free` companion; the `cf02_assist` partner's stuck is not measured (section 9). The backlog attributes nearly every hurt of the companion run to the passive Donut (section 9); the table does not split them by hero. Room 1 (the Hoarder, listed again by FA-Q24) has its own "Room 1" section in [`scorecard.md`](scorecard.md): the `hoarder:carl_solo` room pilot, 9 of 9, and `hrd_direct_clear_companion` for the `cf02_free` partner.

**"Powerful enemies" rating line.**

- **Room 6 (PT-TUNE D1 to D6, `carl_solo`, seeds 0..8):**
  - D1 overlap share W1 37 to 38 % (target median 30 %, every seed 20 % or more), W2 45 to 65 % (median target 40 %): pass.
  - D2 peak distinct engaged kinds W1 2 (3 seen over the wave), W2 3: pass.
  - D3 bot clear median 672 ticks, range 646 to 918 (main 517, target at least 647, at most 4800): pass; the margin over 647 is 25 ticks, under the 30 FA-Q18 asks for (its one composition iteration, W2 entry delays +30 ticks cascaded, moved no clear tick; shipped as measured); the human band 900 to 3000 is the owner's (OQ4).
  - D4 hurts 4 to 6 (2 in W1, 2 to 4 in W2) and at most 1 death on every seed (one hero defeat on seeds 2 and 7, limit 2): pass. Until RING-FU-A the per-seed count read the restart counter, which a checkpoint respawn leaves unchanged, and printed 0 for those two seeds (review F1 of #271); the Deaths column of the table above already counted the defeats.
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

The room pilot is aligned with the library fixtures since PT-GATE-C1: `--room-pilot mastery` is `mastery:carl_solo:0`, and `mastery:<who>:<seed>` and `juicer:<who>:<seed>` (`who` = `carl_solo`, `donut_by_swap`, `companion`, and `donut_fights` in the arena) run the library fixture's party (`f01_mastery_fixtures::tuning`, the Juicer room parties), seed and inputs, through the live party; `ROOM_PILOT_STATE` prints the fixture hash (`party_hash`). A unit test asserts that the room-pilot hash equals the library fixture hash for every (who, seed): 63 of 63 (27 Juicer and 36 arena), and a binary test does the same through `build_scene`. **Why the old plain `mastery` pilot was defeated once (tick 908) while the fixture never was:** it ran the follower-bot partner (`companion_inactive`), whose fight takes the slime on tick 180, where the fixture's active `cf02_assist` partner takes it on tick 101. That unaligned run, and the 6-tick residual measured against a replica, no longer exist (`f01_gate_scorecard::room_pilot_discrepancy` is retired). `pilot_hashes_mastery.txt` is re-pinned (deaths 1 to 0, hash `a5b4af7b118f517e` to `9ce777b82eb734ef`) because the plain pilot now plays the `carl_solo` seed 0 fight. The perf, the wave GIF and the session recorded before PT-GATE-C1 used the old pilot; PT-GATE-D regenerated all of them on `--room-pilot mastery` (`mastery:carl_solo:0`, exit on tick 737): `wave-mastery-w2.gif` and `-640x400.gif` (W2 is now ticks 440 to 734), `perf_mastery.txt` (738 frames), `playtrace_session_mastery.txt` (760 ticks) and the `GATE_B_WAVE` lines of `capture_lines_b.txt`. **PT-GATE-E extends the alignment to the two #240 rooms:** `hazards:<who>:<seed>` (`who` = `carl_solo`, `donut_by_swap`, `companion`, seeds 0..8, 27 runs) plays `hazards_fixtures::pilots::run`'s route (the PT-TUNE pilot with the step back from a goblin that stands on the hero) and `boiler:<who>:<direct|both>:<seed>` (54 runs) plays `boiler_fixtures::scene_pilots::run_live`'s route, whose per-tick input is now the shared `LiveDriver` (a lift, no pin moved); both run the library fixture's party (`f01_mastery_fixtures::tuning`) and seed (`--cf02-seed`) through the live party, and `ROOM_PILOT_STATE` prints the fixture hash. `the_aligned_hazards_and_boiler_pilots_equal_their_library_fixtures` asserts that the room-pilot hash equals the library hash for all 81 (and `hazards:carl_solo:N` equals the pinned `haz_room_direct_route`), and a binary test does it through `build_scene`.

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
| 1 Hoarder | `hrd_direct_clear_companion` (`cf02_free`), seeds 0..8 | deaths 0, max stuck 120 ticks (Carl) and 97 (the companion), her hits on the boss 0 (she is passive against the boss by design, v1), no hazards in the room |

## 8. Gate checklist (plan section 10)

PR numbers are merged PRs on main. Status words: **MET** (evidence cited), **MET as NOT RUN** (the item allows it, or it is owner-only), **UNMET** (not satisfied), **pending**. Game feel is **pending** everywhere; nothing is accepted. The statuses follow the independent gate review (#238), the reviews of #240, #241, #242 and #244.

| Item | Status | Evidence |
| --- | --- | --- |
| **1. Carl complete** | **MET** | CP-A #183, CP-B1 #203, CP-B2 and CP-C #210, CP-D #228, CP-E #233 (evidence in `docs/evidence/carl-prog-01/cp-e`) are merged. Dynamite in `f01_late`: the `carl_kit` pilot throws 1 to 2 times per run in room 6 (`scorecard.md`), and room 4's `haz_spectacular` route throws it so that its blast chains R2 (item 7b). The gauntlet in `f01_late`: `the_gauntlet_staggers_and_pierces_in_the_mastery_room_under_the_story_pin` (`src/f01_mastery_fixtures.rs`, #241) holds `fist_reinforced` under the mastery room's story pin and lands Carl's haymaker with the CP-D stop (6), stagger (28), `goblin_interrupted` and one pierce |
| **2. Donut complete** | **MET** for rooms 2, 4, 5 and 6 (room 5 since FA-Q08, PR #250: boss play FA-Q02..Q07); human P1 and P2 on pads **owner** | The merges are all there: DM-A #186, DM-B #199, DM-C #222, DP-A #197, DP-B #213, DP-C #223, DP-D #225, DP-E #224 (merged, not cut), DP-F #226; CA2-A #198, CA2-B1 #205, CA2-B2 #213, CA2-C #220. The partner a human P1 gets in rooms 2, 4, 5 and 6 is the autonomous `cf02_free` one: [`playtrace_session_default_path.txt`](playtrace_session_default_path.txt) shows `COMPANION_CONFIG: set=cf02_free` on `--scene f01_late --f01-room mastery` with no pilot, and `--companion off\|cf02_assist\|cf02_free\|v0` overrides it ([`owner_commands_check.txt`](owner_commands_check.txt)). Room 5: an opt-in `--companion` is a follow-up (section 9) |
| **3a. `f01_late` launches every listed room** | **MET** | `owner_commands_check.txt` (31 lines: every manifest room launches, the refusals are the expected ones) |
| **3b. One `CapabilityGranted` per grant in the PLAYTRACE journal** | **MET** (the Juicer's two grants and the Hoarder's dynamite, each in a strict PLAYTRACE session) | Juicer: [`GATE_A.md`](GATE_A.md) sections 3 and 4 (two lines on tick 3175; the Boiler, hazard and room 6 sessions hold none). Dynamite: room 1 (the Hoarder) is **listed again** (FA-Q24, reverses PTG-AD1; `MANIFEST_HASH` re-pinned `0x24bc1268822f41f7` to `0xb45c744a126ee87c`) and its room pilot defeats her: `a_piloted_hoarder_session_has_exactly_one_capability_granted_and_verifies_strict` (`src/playtrace/tests_f01_late.rs`) records the `hoarder:carl_solo:0` run (1381 ticks, strict, `PASS scope=full`) and counts **exactly one** `CapabilityGranted` in the journal, hero 0 `special_thrown_charge`, on her defeat tick; the Hoarder room pilot defeats her on 9 of 9 seeds with one grant each (`the_room_pilot_defeats_her_nine_of_nine_with_one_grant_and_is_pinned`). The placeholder-zone fixture `carl_prog_story_event_grants_dynamite` stays as the second witness. A human recording of the grant is **NOT RUN** |
| **3c. Minimum room set, honest milestones** | **MET** | Rooms 2 (BOIL-E #227 content, BOIL-F evidence #233), 5 (PT-SCENE #211, PT-SCENE-B #234), 6 (PT-MASTERY #232, PT-TUNE #236) and room 4 at HAZ-B (#231, after HAZ-A2 #214 and #229), whose item 5 closed in #240. Milestones: room 2 setpiece, room 4 greybox, room 5 boss, room 6 greybox. Room 0 (the corral) is listed since CORRAL-F at greybox, after this gate was met. Not listed: room 3 (the Dozer track is deferred), rooms 4a and 4b (the Ring, optional) |
| **4a. Goldens byte-identical** | **MET** | PT-GATE-E moves no golden and no pin: it adds two room-pilot plans (`haz_tactical`, and the aligned `hazards:` and `boiler:` library pilots), lifts the Boiler live pilot's per-tick input into a shared `LiveDriver` (the 54 `LIVE_PINNED` hashes, `SOLO_CARL_PINNED` and `DIRECT_PINNED` pass unchanged), and regenerates evidence. The files that changed are listed with their reasons in the PR body |
| **4b. New fixtures: committed hash, replay twice, headless and render loop** | **MET** (153 of 153 aligned runs; the 9 Rot-off runs are exempt: no scene route) | [`render_replay_lines.txt`](render_replay_lines.txt): every aligned run was run through `scripts/room_capture.sh` twice with `scripts/make_f01_gate_d_replay.sh`: a real window under xvfb (the Bevy presentation) and the headless run, one after the other. The window's final `COMBAT_STATE` hash equals the headless one, equals the library fixture's combat hash, and the headless `ROOM_PILOT_STATE` hash equals the library fixture's final hash: **153 of 153 `match=yes`** = 27 Juicer (`juicer:WHO:SEED`) + 36 arena (`mastery:WHO:SEED`) + 27 live room 4 (`hazards:WHO:SEED`, the 27 Rot-on runs of #240) + 54 live room 2 (`boiler:WHO:direct|both:SEED`) + 9 room 1 (`hoarder:carl_solo:SEED`, FA-Q24). **Exempt, not run:** the 9 Rot-off runs of `haz_room_direct_route` build the real `hazq02_rot_enabled` switch since FIX-TOOL (`EnvRuntime::new_with_rot`, `src/hazards_fixtures.rs`), but the shipped scene reads that switch from the pinned data and has no flag or descriptor field to turn it off, so they still have no scene counterpart to replay; they are not counted in the 153. The 9 Rot-on `DIRECT_PINNED` hashes equal `SOLO_CARL_PINNED`, i.e. `hazards:carl_solo:N`. The room 2 `direct`, `valve` and `chain` room pilots and room 4's three route pilots are also window-equals-headless (`docs/evidence/f01-boiler/capture_lines.txt`, `capture_lines_d.txt`, `capture_lines_e.txt`). The two tactical and spectacular final hashes are pinned by unit tests. **Provenance of the committed lines:** the first full run (all 144) matched 90 of 144, because the Boiler's headless smoke did not print `ROOM_PILOT_STATE` (all 54 window, headless and library combat hashes already agreed); that smoke now prints it (output only), and the 54 `boiler:` runs were re-run with `ROOMS=boiler`, which keeps the other 90 committed lines (the 63 PT-GATE-D lines are byte-identical to main's). **Refresh after the batch of #256..#267:** the whole set (all 153, no `ROOMS=` filter) was re-run on main at #264; 153 of 153 `match=yes`, and in every line both window runs, both headless runs and the library combat hash are one value. The 144 non-Hoarder lines are byte-identical to the previous committed lines; only the 9 `room=hoarder` lines changed, because PR #260 moved the room 1 pilot by 4 ticks (each final tick is 4 later and its hashes are new). These are simulation-state hashes, so the art PRs of the batch (drawn output only) do not move them |
| **4c. One recorded live session per listed room, strict** | **MET** (scripted bots; room 6 also on the human default path, full scope, input-free); human NOT RUN | Rooms 2, 4, 5: `playtrace_sessions.txt`. Room 6: `playtrace_session_mastery.txt` (760 ticks on the aligned pilot, `PASS scope=full`, `run_kind ScriptedBot`). Room 6 on the human default path, no `--room-pilot`: `playtrace_session_default_path.txt`, `PASS scope=full` over 1200 ticks, `cf02_free`. The window ends itself with `--exit-after-ticks 1200` (FIX-TOOL: a scripted exit that needs no pilot and keeps the live input source; a SIGINT or SIGTERM and a window close also end the window cleanly now). **It is not a human session:** Carl got no input (no xdotool, no synthetic window input: the owner declined them), and `playtrace inspect` says so (`run_kind Human (input-free: no input in any tick, the partner AI only; not a human play session)`). A real human session is **owner**. Session files are not committed (the store is ignored; the scripts reproduce them) |
| **5a. Direct routes with props untouched** | **MET** | BOIL-01: `the_direct_route_reaches_the_exit_with_every_prop_untouched` and the live `direct` room pilot; the live `Direct` fixture runs clear 9 of 9 (the pilot touches no prop; the partner's hits are reported, section 9). HAZ-06: `haz_room_direct_route` (#240): a melee-only route clears every slime and goblin and reaches the exit at 9 of 9 seeds, with and without the Rot kind. Room 1 (HRD-A2, FA-Q24): `hrd_direct_clear_solo` and `hrd_direct_clear_donut` exist, and the `hoarder:carl_solo:SEED` room pilot clears the listed room at 9 of 9 seeds. DOZR-01: room 3 not listed |
| **5b. Per-room pilots at seeds 0..8, no hero stuck over 300** | **MET** (rooms 2, 4, 5 and 6; caveat in section 9) | Room 2 live (`Roster::Live`, solo Carl, Donut by swap, `cf02_free` companion; `direct` and valve-then-brace, 54 runs): 9 of 9 clear in every row, worst stuck 43 ticks. Room 4 (`haz_room_solo_carl`, `_solo_donut`, `_companion`, plus the Rot-off direct route, 36 runs): 9 of 9, worst stuck 40. Room 5: all three modes, max stuck 17. Room 6: all three, max stuck 95. See section 5 and `scorecard.md`. Not met in the letter: Donut who fights room 4 or room 6 alone (`haz_room_donut_fights` ignored, `donut_fights` 0 of 9) is an open known gap, not an item 5b mode (the item names Donut by swap) |
| **5c. No-softlock and reachability validators** | **MET** | Boiler: `the_reach_matrix_covers_every_class_and_state_and_all_pass` (`src/encounter.rs`). Hazards sidecar: `haz_room_reach_matrix_has_no_softlock_for_any_class` (`src/hazards_fixtures/tests_c.rs`, #240). The DP-Q45 sweep `dpq45_every_room_with_a_high_surface_declares_a_lane_ceiling` (`src/donut_prog_fixtures.rs`) exists |
| **5d. AUTOPLAY coverage of the setpiece rooms** | **MET as NOT RUN** (the item says so) | PT-AUTOPLAY is deferred (plan section 11) |
| **6. Fairness** | **MET** | K6: `haz_acid_not_evadable` (`src/hazards_fixtures/tests_b.rs`), `hazards_are_data_and_never_evadable` (`src/encounter.rs`), `hrd_not_evadable`. K3: `carl_prog_dyn_no_friendly_fire`, `carl_prog_dyn_hero_damage_policies_share_one_vocabulary` (`src/combat_scene/thrown.rs`), `the_rot_blast_is_a_blast_profile_with_no_core_and_all_heroes`. K8: `k8_a_nest_nobody_threatens_is_an_error` and its siblings. K5: room 3 not listed |
| **7a. One at-speed GIF per ability, 640x360 and 640x400** | **MET** | All nine abilities at both views (tables below): the 640x360 set (PT-GATE-B and earlier) and the 640x400 set: `deck-*.gif`, eight GIFs (the five regenerated abilities, both agile-motion races and the dynamite ring), each window equal to headless and generated twice byte-identical (`capture_lines_d.txt`); the throw and the pierce already had 640x400 GIFs |
| **7b. Direct, tactical and spectacular route per setpiece** | **MET** (room 4: all three routes have a clip at both views; Juicer: the grant clip only; the Hoarder: the direct room pilot as stills at 1x and 1080p, no clip) | Boiler: direct, valve and chain (`docs/evidence/f01-boiler`, #233). **Room 4 spectacular** (HAZ-07 pattern 3, the dynamite chain, onto R2; retargeted by FA-Q21 with `hazfu05_sticker_chain_reach_px = 76`): `route-room4-spectacular.gif` and `route-room4-spectacular-640x400.gif` (`--room-pilot haz_spectacular`, ticks 470 to 710, 6 ticks a frame): the PT-TUNE pilot clears pocket A, baits R1 (proximity provoke on tick 373, blast on 397, no hurt) and throws one dynamite from column 648 (throw tick 484, lands 759); the stick's blast on tick 634 defeats S2 (credited to Carl, it stands inside both blasts) and lights R2 (a `Chain` provoke of sticker 1 on the same tick), which goes off 8 ticks later (642); neither blast hurts Carl and the room is cleared to its exit (tick 836, final hash `80cfaca8118ce58e`, the one hurt of the run is the pocket A pool on tick 212). The unit test `the_haz_spectacular_plan_chains_the_dynamite_into_r2_over_the_slime` and the CI line `F01_HAZ_SPECTACULAR_OK` hold the chain. **Room 4 tactical (HAZ-07 pattern 2: R2 baited over the slime S2, sheet :333):** `route-room4-tactical.gif` and `route-room4-tactical-640x400.gif` (`--room-pilot haz_tactical`, ticks 340 to 700, 6 ticks a frame): the PT-TUNE pilot clears pocket A; Carl then baits R1 (proximity provoke on tick 373, back off, the blast on tick 397 lands on empty floor) and R2 (proximity provoke on tick 470, back off to x 693, the blast on tick 494 hits S2 at home for exactly 1: 3 hp to 2, S2 untouched before it); no Rot blast hurts Carl, the one hurt of the run is the pocket A pool on tick 212 (acid), and the room is cleared to its exit on tick 782 with 0 deaths (the sheet has no goblin collateral, Revision 4). The unit test `the_haz_tactical_plan_baits_r2_over_the_slime_and_the_blast_hits_s2` (`src/room_pilots.rs`) asserts the two proximity provokes, both blasts at provoke + 24, S2's hit point the tick before and after the blast, no Rot hurt, the exit with 0 deaths, the replay twice and the final hash `383a53070f66546a`; the CI line `F01_HAZ_TACTICAL_OK` holds it. **Room 4 direct:** `route-room4-direct.gif` and `route-room4-direct-640x400.gif` (`--room-pilot hazards:carl_solo:0`, ticks 320 to 650, 6 ticks a frame): the aligned HAZ-06 melee route, the PT-TUNE pilot of the library fixture `haz_room_solo_carl` at seed 0 (its hash equals `SOLO_CARL_PINNED[0]`, which equals `DIRECT_PINNED[0]`, asserted by `the_aligned_hazards_and_boiler_pilots_equal_their_library_fixtures`). It does not dodge the Rot blasts: Carl is hurt three times (the pocket A pool on tick 212, then R1 on tick 357 and R2 on tick 626) and Donut once (R1, tick 357), leaving Carl on 1 hp, never defeated; the room is cleared on tick 739. All three routes at both views: window equals headless equals the timed live loop, two generations byte-identical (`capture_lines_d.txt` for the spectacular route, `capture_lines_e.txt` for the other two; `scripts/make_f01_gate_e_evidence.sh`) |
| **7c. The two grant moments** | **MET** | [`GATE_A.md`](GATE_A.md) section 3 (`grant-juicer-gauntlet-blink.gif`, `grant-dynamite-placeholder-zone.gif`). Room 1 is listed again (FA-Q24) and the PLAYTRACE journal holds her grant (item 3b); the placeholder-zone dynamite clip is still the dynamite grant moment clip, and the Hoarder's own defeat is shown as stills (`../f01-hoarder-presentation`, `defeated_*.png`) |
| **7d. A mastery-arena wave** | **MET** (regenerated on the aligned pilot) | `wave-mastery-w2.gif` (640x360) and `wave-mastery-w2-640x400.gif`: W2, the llama yard, ticks 440 to 734, 6 ticks a frame (100 ms, game speed), real Bevy, `--room-pilot mastery` (= `mastery:carl_solo:0`, exit on tick 737), window hash equal to the headless hash at both views (`capture_lines_b.txt`, `GATE_B_WAVE`, `GATE_B_WAVE_640x400`), two generations byte-identical |
| **7e. Frame time, gore preset 4** | **MET** (xvfb, dev build; hardware **NOT RUN**) | Boiler chain: `perf.txt`, three runs at 640x360 (PT-GATE-A) and three at 640x400 (PT-GATE-D). Arena: `perf_mastery.txt`, three runs at each view, tables below. xvfb under the item's letter; **real-hardware frame time: NOT RUN** |
| **7f. A real two-pad 2P run** | **MET as NOT RUN** (the item allows it; owner) | needs two pads |
| **8. Owner packet** | **MET** | this file |
| **9. Independent review, game feel pending** | **MET** | the independent top-tier reviews of #242 and #244 passed with no blocking finding and no logic or design defect; game feel is **pending** and nothing is accepted |

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

The regenerated GIFs above are 640x360. Their window and headless final hashes are equal (`capture_lines_b.txt`), and two generations were compared byte for byte.

**The 640x400 set (`--deck`, PT-GATE-D, `scripts/make_f01_gate_d_evidence.sh`).** The same fixtures and tick windows, with `--deck`: the view height is an input of the simulation, so each ends on its own hash (a different run, not nondeterminism); each window hash equals its own headless hash and two generations are byte-identical (`capture_lines_d.txt`). The throw and the pierce already have 640x400 GIFs (CP-E, above).

| Ability | 640x400 GIF | Fixture, ticks |
| --- | --- | --- |
| Dynamite chain | `deck-ability-chain.gif` | `carl_prog_dyn_chain_depth_cap`, 188 to 245 |
| Dynamite ring | `deck-ring-outer-launch.gif` | `carl_prog_dyn_outer_launch`, 188 to 251 (the CP-E outer launch) |
| Donut agile motion | `deck-agile-run-race.gif`, `deck-agile-jump-race.gif` | `dm_run_race_2p`, 4 to 124; `dm_jump_race_2p`, 6 to 168 |
| Reflex | `deck-ability-reflex.gif` | `cf02_dm_reflex_cues`, 712 to 799 |
| Magic Missile | `deck-ability-magic-missile.gif` | `cf02_assist_donut_missiles_goblin`, 8 to 70 |
| Light on Your Feet | `deck-ability-light-on-your-feet.gif` | `donut_prog_arena_ledge_boost`, 28 to 100 |
| Puddle Jumper | `deck-ability-puddle-jumper.gif` | `donut_prog_arena_blink_tour`, 30 to 108 |

Room 4's routes (item 7b), each at 640x360 and 640x400: `route-room4-spectacular*.gif` (the dynamite chain onto R2), `route-room4-tactical*.gif` (R2 baited over the slime S2) and `route-room4-direct*.gif` (the aligned melee route).

**Mastery arena frame time** (`bash scripts/make_f01_gate_b_perf.sh`; `bash scripts/room_perf.sh mastery mastery 738`, N = the aligned room pilot's exit tick 737 plus 1; gore preset 4; lighting on; 3 runs per view). **xvfb virtual display, not real hardware, and not a handheld: the frame interval is bounded by the X copy path.** The GPU column (summed timed render passes) and the pilot tick are a **lower bound** on frame cost: they leave out the main-thread CPU of the ECS, extraction and batching, and the HUD. The readings were taken with the dev profile on an AMD Ryzen 9 5950X with a discrete GPU reported by wgpu (an NVIDIA GeForce RTX 4060 Ti).

| View | Run | median ms | p95 ms | p99 ms | max ms | mean ms | GPU ms mean (passes) | pilot tick us mean | load average after |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 640x360 | 1 | 16.95 | 19.65 | 20.46 | 22.16 | 16.67 | 0.293 (4) | 44.6 | 2.63 3.27 3.75 |
| 640x360 | 2 | 16.58 | 19.61 | 20.29 | 21.37 | 16.67 | 0.296 (4) | 45.8 | 3.29 3.37 3.77 |
| 640x360 | 3 | 16.45 | 19.64 | 20.78 | 22.03 | 16.66 | 0.293 (4) | 46.0 | 3.02 3.30 3.74 |
| 640x400 | 1 | 16.93 | 19.43 | 20.02 | 20.67 | 16.67 | 0.321 (4) | 44.6 | 2.87 3.25 3.71 |
| 640x400 | 2 | 16.52 | 19.74 | 20.16 | 22.26 | 16.66 | 0.302 (4) | 46.3 | 2.74 3.20 3.69 |
| 640x400 | 3 | 16.36 | 19.55 | 20.28 | 22.34 | 16.66 | 0.295 (4) | 46.2 | 2.82 3.20 3.68 |

Every run is 738 frames (pilot ticks 0 to 737) with 847 death sprites spawned and up to 109 visible at once. The aligned pilot's run is shorter than the pre-PT-GATE-C1 follower-bot run these numbers replace (1191 frames, from tick 0 to 1190). The median sits at the 16.6 ms of the xvfb copy path (a hypothesis, not measured), so the p95, p99 and max columns and the GPU and tick columns carry the information.

**The Boiler's full chain at 640x400** (`bash scripts/make_f01_gate_d_perf.sh`; `bash scripts/room_perf.sh boiler chain 950 --deck`; the live roster, gore preset 4, lighting on, 950 frames). The three 640x360 runs of PT-GATE-A are the first three blocks of `perf.txt`; they were taken on a loaded machine (their p95 and p99 reach 31 and 38 ms in run 1) and are not comparable with these.

| View | Run | median ms | p95 ms | p99 ms | max ms | mean ms | GPU ms mean (passes) | pilot tick us mean | load average after |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 640x400 | 1 | 16.64 | 19.44 | 20.29 | 22.55 | 16.67 | 0.364 (4) | 28.0 | 2.15 3.00 3.60 |
| 640x400 | 2 | 16.48 | 19.77 | 21.51 | 22.97 | 16.67 | 0.272 (4) | 33.1 | 2.51 3.02 3.59 |
| 640x400 | 3 | 16.33 | 19.85 | 21.05 | 23.83 | 16.66 | 0.276 (4) | 31.7 | 2.22 2.92 3.55 |

Every run: 844 death sprites spawned, up to 69 visible, 504 to 509 MiB resident. **Machine load was 2.2 to 3.3 during all of these runs** (other processes of the machine; the load average after each run is in the tables and in the files), so treat them as xvfb readings on a moderately busy machine, not a quiet-machine benchmark. No claim is made about a Steam Deck or any real display.

## 9. Known gaps

From the backlog's owner game-feel notes, the PT-TUNE follow-ups, #237, the reviews of #240, #241, #242 and #244:

**Gate items still open: none.** Items 4b and 7b, the two UNMET items of the PT-GATE-D review, are closed by the evidence in section 8, and item 9 is met by the independent reviews of #242 and #244. The owner-only items remain: real hardware (frame time included), a real two-pad 2P run, and human play and game feel. None of them is a gate item.
- **Done in PT-GATE-E (#244), with notes (none is a gate item):** room 4's tactical and direct routes (item 7b) and the render-loop replay of #240's fixtures (item 4b) are closed above. `haz_spectacular` was **not** retargeted onto R2 with S2 in the blast (the sheet's pattern 3, `docs/01-design/enemy-gameplay-slime-rot.md` :337): it still chains R1 in pocket B, which has no enemy. **PR-D (FA-Q21) retargeted it:** the 8 px sweep found that no column could chain R2 at the dynamite's reach of 64 (under the 84 px soffit R2's dome is 70.007 px from the nearest floor landing), so the adopted value `hazfu05_sticker_chain_reach_px = 76` (dynamite onto a Rot Sticker only) lets columns 648 to 672 chain it; `haz_spectacular` now baits R1 and throws once from column 648 (`haz_r2_chain_sweep_over_the_throw_columns`). Its hash, GIFs, scorecard row and CI line moved (see PR-D's re-pin table). HAZ-07's full `haz_room_c_orderings` fixture is not required by the gate and is not added. `haz_tactical` takes one acid hurt in pocket A (tick 212, the PT-TUNE pilot's pool crossing); the fallback of provoking R2 by a jump from x 712 was not needed. The 9 Rot-off runs have no scene route (see the FIX-TOOL bullet below).
- **The room 6 session on the human default path is input-free, not human** (`PASS scope=full`, 1200 ticks, FIX-TOOL): `--exit-after-ticks N` now ends a live room run without a pilot, SIGINT and SIGTERM and a window close end the window through the ordinary exit, and `playtrace inspect` labels a session whose slots carry no input in any tick as input-free. A session with a person at the pad is the owner's.
- **Fixed by FIX-TOOL (the tooling and harness known gaps), with notes:**
  - *Rot-off runs:* `src/hazards_fixtures.rs` now builds the room through the real switch (`EnvRuntime::new_with_rot`, the sticker props stay as `Machine::Rot` without a sticker state), not a room with the props removed. The 9 Rot-off hashes of `haz_room_direct_route` moved on purpose (old and new values in the PR that moved them); outcomes and the 9 Rot-on hashes did not move. The runs still have **no scene route** (the switch is read from the pinned `rot_sticker` data; no flag, descriptor field or recording can turn it off), so they are not in the render-loop replay (still 144 runs).
  - *Stuck metric of the uncontrolled partner:* measured by the companion's "left behind" rule for the hero nobody steers, in every mode (`LeftBehind`). Rooms 2 and 4, seeds 0..8: the `cf02_assist` partner (Carl solo, Donut by swap) is left behind at most 7 ticks in room 2 and 5 in room 4; the `cf02_free` companion at most 30 and 29. Nobody is stuck over 300. The partner's AI was not touched.
  - *`run_live` restarts:* the live driver restarts its plan on a checkpoint restart (a forced restart is tested). No committed run has a restart, so no hash moved.
  - *`chain` pilot gating:* the walk-on arm is no longer gated on a live one-player party; for the no-party and two-player runs at seeds 0..8 the 18 committed runs hash as before (the dead-band park never happened there), and a test now fails on a park in x 556 to 560.
  - *`user_fixture()`:* the invariant is in its doc comment and `every_room_scene_refuses_a_user_fixture_up_front` asserts the refusal text of every room scene, including `--fixture cp01_kick --companion cf02_free`.
  - *Tooling:* the Hoarder smoke in `ci_floor01_combat.sh` greps `slot_enemies=1` again; the filmed `haz_tactical` run is pinned in CI (`8e10bd0986d23ad9`); the `ROOMS=` filter of `make_f01_gate_d_replay.sh` finds the `boiler` lines (the `route=` field) and re-checks a kept line against the fresh library tick and hashes; `make_f01_gate_b_evidence.sh` prints the per-hero hurt split ("4 (Carl 3, Donut 1)").

**The arena (PT-TUNE and PT-MASTERY)**
- **The D3 margin is thin:** the bot's median clear is 672 ticks against the 647 target, a margin of 25 ticks (FA-Q18's one composition iteration changed no clear tick and was not kept); the 900 to 3000 human band is unmeasured (OQ4).
- **Seed coverage (FA-Q17):** the arena pilots offset the jab phase by the seed, so `carl_solo` has 7 distinct outcomes (clear tick, hurts, kill order) over seeds 0..8 (4 with the first form tried), `donut_fights` 9 and the companion 7; the seed still varies only the pilot's timing and the temperament draw. The Juicer's seed coverage (the seed shifts the jab phase only) is in the scorecard under room 5.
- **The Donut kit pilot (PTT-Q21, fixed by FA-Q12, FA-Q13):** the shared pilot now plays Donut's rules (stand-off 40 px, pounce at point-blank or dry, approach, the kit pilot's blink). `donut_fights` and `donut_kit` clear 9 of 9 and no pilot stalls more than 79 ticks (`max_stall_ticks`, asserted at most 120). A Donut-led clear is proven for the scripted pilot only; her hurts (9 to 12 a clear against Carl's 4 to 6) are the price of the stand-off rules.
- **The companion Donut under the Juicer (done in PR #250).** She was idle against the boss (CA2-Q35, `CA2-FU-BOSS`); FA-Q01..Q07 give her boss play and room 5 now launches her by default (FA-Q08). M13 (her hits against the assistant's) is still missed and shipped as measured (FA-Q28-AD1).
- **CA2 misses:** M2 (shadowing), M4 (steals on 2 seeds) and M13's comparison with the assist partner; M7, M8, the M9 cause split, M11 and M14 are measured in the arena since FA-Q (all pass, 0 hazard hurts to her; `the_ca2_metrics_run_in_the_closing_arena`).
- The arena reuses room 4's hazard presentation (danger-effect pixel art since PR #263; the spent drip is still stand-in) and has no presentation of its own; no human has played it.

**Room 4, the hazards room (#240 review follow-ups)**
- **`haz_room_donut_fights` is un-ignored (PR-D, FA-Q14): solo Donut who fights the whole room clears it at 9 of 9 seeds** (first pinned as one fight, cleared on tick 551 and exit on 633 at every seed, because the route reaches no random stream; the review follow-up gives Donut's press a `seed % 4` tick phase, FA-Q14 ruling 2, so the seeds now clear on 549 to 552 with 4 distinct outcomes; nine hashes in `DONUT_FIGHTS_PINNED`, six re-pinned, no death, nobody stuck). The missile sweeps from her front edge on its first tick (`missile_spawn_sweep`) and the shared pilot has the Donut stand-off (FA-Q12). **Owner-note correction (T35):** the earlier game-feel line "Donut has no point-blank answer once her casts are dry" is wrong as worded: at 0 mana the Physical button is the pounce, which is her point-blank answer (Q10); only the *Primary* press is dry, and that dry press (`empty_press: dry`) is Q10's final rule, so a dry cast not turning into a swat is by design, not a gap.
- **`haz_room_solo_carl` is the HAZ-06 route:** the first nine `DIRECT_PINNED` hashes equal `SOLO_CARL_PINNED`; both run the same route (HAZ-09 says "same route").
- **The assist partner walks into the Rot in room 4** (9 of 9 seeds of the Carl-solo and Donut-by-swap runs; the `cf02_free` companion never does): a game-feel note for the partner that rooms 2, 4 and 6 do not default to. **Fixed in PR #250 (FA-Q15):** the `cf02_assist` partner is fenced from an awake sticker's span and takes 0 Rot hurts on 9 of 9 seeds.

**Room 2, the Boiler**
- **The assist partner hits the valve in the live `Direct` runs.** `Direct` is defined and asserted as "the pilot touches no prop", and the partner's hits are reported, but the live runs no longer show that the live roster can be cleared with the props intact (only the greybox, `Roster::Inert`, tests do). Add a live variant with the partner held passive, or record it as a game-feel note: "the assist Donut vents the valve on her own in every direct run". PR-C adds the isolating variant: with a passive partner (`Who::PassivePartner`, the follow-and-jump bot) the live `Direct` route has 0 prop hits by any hero on 9 of 9 seeds, and a `partner_prop_hits` counter in the live report. The active-partner assertion is an `#[ignore = "needs PR-A FA-Q16"]` test that PR-A un-ignores.
- B3 no longer wakes inside the steam lane: since PR-C (FA-Q20) it wakes at x 560 (`trigger_x`), 128 px before the lane (`boil_b3_wakes_before_the_lane`). The drum's solid keeps it out of the lane until the drum blows. The owner has not played it.

**Rooms and presentation**
- **Room 5 partner (#241 follow-up 1, done in PR #250):** the Juicer's boss scene accepts `--companion` and defaults to `cf02_free` (FA-Q08; the fixture set by `--f01-room juicer` is a room placeholder; arg test and owner-commands `ok` lines added). `--companion off` is the follow-and-jump bot.
- The Hoarder's drawing is **prototype pixel art since PR #258** (HOARD-02 and AREA-HOARDER-01; independent top-tier art review passed, owner art review pending); since PR #266 (E6 polish, owner art review pending) window stills show the shove, lunge, jam, the stack's warning and fall, pinned and get-up (`docs/evidence/f01-hoarder-presentation/`); bank A's states after `stable` still have offline composites only. PR #260 moved the room 1 pilot by 4 ticks; the 9 `room=hoarder` lines of `render_replay_lines.txt` were refreshed by a full re-run of `scripts/make_f01_gate_d_replay.sh` after #267 (9 of 9 window equals headless equals library, twice). Not drawn: the stir cue before she wakes (sheet HRD-Q05). A design finding for the owner (HRD-FU, `hrd_leash_kite_probe`): with the entry gate **off** a ranged hero can kill her from x 150 with no hit taken (one draw cycle, 30 hit points, about 36 s); with the gate on (the default) heroes stand inside her reach. Her swipe tell after turning back on a follower starts 23 ticks after it enters 88 px (HRDF-D1, adopted; the sheet said 21).
- Hazards: the Pocket C relayout, the missile window of 7 press ticks on the rise and 8 on the fall, Donut's run-through of a still `H` 68 sticker is hit once.
- Some production art is still stand-in (K10): the spent drip (rooms 4 and 6). The Boiler's steam warning area and drum blast-area outline stay code-drawn footprints by design (the Goblin Workshop spec). Since PR #263 the acid pool, the blast reach rings, the shock waves and the room 5 gate are prototype pixel art (owner art review pending); the per-room table is in `docs/04-production/backlog.md`. The Dozer is not in the slice; the corral is listed since CORRAL-F at greybox.

**Mechanics notes**
- Gauntlet: the haymaker-only pierce (F-12); the pierce spark is faint at 1x.
- The scripted coop blink pilot had more deaths than its no-blink control (7 against 0 over seeds 0..8), likely the swat-hold rule; the boss-charge blink guard is dead in the shipped stack (#226).
- The 720-tick reflex was not ready for the opener (a human-style Donut evaded 0 of 23 at 720 from tick 0, #222). FA-Q19 (`dm_reflex_armed_at_attempt_start`) starts an attempt that begins with the biscuit armed; whether 720 and that rule feel right is the owner's measurement (DM spec U1).

**Evidence limits**
- Frame time is under xvfb only, with a dev build, on a machine with a load average of 2 to 3; real hardware, a Steam Deck, a 1080p window and a release build are **NOT RUN**. The perf reading is a lower bound (section 8, item 7e).
- No human, no pad, no real two-pad 2P run; no human-recorded session; the sessions are scripted bots (or, for the default-path session, an idle P1: input-free).
- The Hoarder dynamite grant in a human-played recording (the scripted pilot's strict session holds it); the live `--companion cf02_free` Juicer launch.
- AUTOPLAY coverage of the setpiece rooms.
- Tools: `perf.txt` and `GATE_A.md` predate PT-GATE-B; the GIF generation is byte-stable under the 60-frame first settle, and the generators refuse a near-black first frame.

## Regenerate

```bash
cargo build --locked --bin floor01_scene --bin playtrace --bin corral_eval && bash scripts/export_assets.sh   # xvfb-run and Pillow
bash scripts/make_f01_gate_b_perf.sh        # perf_mastery.txt, on a quiet machine (wall-clock)
bash scripts/make_f01_gate_d_perf.sh        # the Boiler at 640x400 into perf.txt, on a quiet machine
bash scripts/make_f01_gate_b_evidence.sh    # wave and ability GIFs (twice, compared), the room 6 session, scorecard.md, pilot_hashes_mastery.txt
bash scripts/make_f01_gate_d_evidence.sh    # the 640x400 GIFs and room 4's spectacular route (twice, compared)
bash scripts/make_f01_gate_e_evidence.sh    # room 4's tactical and direct routes at both views (twice, compared), capture_lines_e.txt
bash scripts/make_f01_gate_d_replay.sh      # render_replay_lines.txt: the 153 aligned runs, window versus headless versus the library, twice (about two hours, serial)
bash scripts/make_f01_gate_d_session.sh     # playtrace_session_default_path.txt (needs the playtrace binary)
bash scripts/make_hoarder_presentation_evidence.sh   # ../f01-hoarder-presentation: stills at 1x and 1080p, twice, compared
bash scripts/check_f01_owner_commands.sh    # owner_commands_check.txt
python3 scripts/check_docs.py               # verifies SHA256SUMS
```
