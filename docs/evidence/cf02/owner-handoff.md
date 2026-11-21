# COMBAT-FUN-02 owner handoff

The whole COMBAT-FUN-02 feature is merged on main and the agents re-verified it headless ([re-verification table](README.md#re-verification-on-the-final-main)). **No human has played it.** Every mechanic below is recorded as "merged, game-feel review pending"; none is accepted until you play it. This page is what to run, what to judge and what the agents did not do.

## 1. Build and launch

Run from the repository root of the build you want to test. One-time setup (needs Aseprite or LibreSprite):

```sh
bash scripts/export_assets.sh
cargo build --locked --release --bin floor01_scene
```

Then pick one of the launches below. If you skip `export_assets.sh` the game stops at start-up with `FLOOR1_ASSET_FAILURE ... run bash scripts/export_assets.sh`; that is the missing art, not a bug. `--cf02` is the whole feature: it loads the microarena and turns every CF02 rule on. Without it you get the unchanged DEMO-01 build.

| Session | Command | What you get |
| --- | --- | --- |
| Solo, old follow-and-jump bot (assistant off, Donut still in the party) | `cargo run --locked --release --bin floor01_scene -- --cf02 --companion off --scale 3` | `--companion off` does **not** remove Donut and there is no Carl-only mode in the party game: it swaps the assistant for the old follow-and-jump bot. You control Carl, or Donut after a swap; the other hero follows you and jumps but never attacks |
| One player with the assistant (the reference run) | `cargo run --locked --release --bin floor01_scene -- --cf02 --scale 3` | The same party, but the `cf02_assist` companion drives the other hero as a fighting partner (CF02-DUO, 2026-11-21): she engages goblins and the llama on her own, swats a tell aimed at Carl, pounces what stands, may land the killing blow on an enemy you are not engaging (she leaves you the last hit of the one you are fighting, CF02-DUO-A08), about one action per 20 ticks. The room starts with Donut's biscuit locked, so by default she has no Magic Missile (pounce and the interrupt swat only); `--donut-biscuit` unlocks it and she then casts at enemies in range within the mana rules (headless-tested only, not run windowed with `--cf02`) |
| Two players, two pads | `cargo run --locked --release --bin floor01_scene -- --cf02 --scale 3` and press **Start** on the second pad | P1 is Carl, P2 becomes Donut; L1/LB no longer swaps; the assistant is replaced by P2 |
| Keyboard fallback | `cargo run --locked --release --bin floor01_scene -- --cf02 --scale 2` | Keyboard is P1 only; there is no keyboard binding for P2 |

Useful flags (all optional; write the ones you use in the scorecard): `--cf02-seed N` (a repeatable temperament draw; without it every launch uses the same fixed draw per attempt, printed as `seed=fixture` on the `CF02_CONFIG` line), `--camera scroll` (the old camera, if the auto camera moves too much), `--gore-preset 1..4` (default 4), `--cinematic-finishes=off` (default on in live play), `--deck` (640x400), `--no-audio`.

The console prints `CF02_ARENA_COMPLETE tick=N` when all six enemies are down.

- **Room length (the 40 to 60 s target):** seconds = N / 60, counted from the last restart (the Select hold), respawns included; slow motion does not add ticks. Write N and N / 60 on the scorecard.
- **Defeats and respawns:** a defeat respawns you automatically at the last checkpoint after about 1 s (50 ticks); you press nothing. A **Select tap respawns at the checkpoint at any time, also while alive, and it resets every living goblin**, so an accidental tap changes the attempt. A Select **hold** (1 s) restarts the whole room, redraws the temperaments and starts a new attempt.
- **Counting deaths:** `DEMO01_RESPAWN` prints for both an automatic and a tapped respawn, so do not count deaths from it. Count the console lines `COMBAT_EVENT tick=N kind=hero_defeated` (Carl; a Donut defeat carries `hero=donut`) or keep a manual tally on the scorecard.

A session is recorded locally by default (`PLAYTRACE_REC` shows its id; nothing is uploaded); `--no-record` turns that off, `cargo run --locked --bin playtrace -- list` shows your sessions.

## 2. Controls

From [input-camera](../../03-tech/input-camera.md) and [floor01-scene](../../03-tech/floor01-scene.md#flags). There is no new mandatory button: Down is a direction.

| Action | Pad | Keyboard (P1) |
| --- | --- | --- |
| Move | left stick or D-pad | A / D or the arrows |
| Jump | Cross / A | Space |
| Jab (again inside the window: haymaker) | Square / X | J |
| Physical: Carl's kick, Donut's pounce | Triangle / Y | K |
| **Down** (held with Physical) | D-pad down, or left stick down to 0.5 with the vertical axis dominant | S or the Down arrow |
| Solo swap (one player only) | L1 / LB | Q |
| Respawn at the last checkpoint (tap, works also while alive and resets the goblins) / restart the room (hold 1 s) | Select | R |
| Join as P2 (second pad) / quit | Start (second pad joins; Start on P1 quits) | Esc quits |

Carl's moves in the room: **Down + Physical on the ground is the stomp**; **Physical right after a jab that hit is the follow-through kick**; **Physical alone is the push kick**; in the air **Physical is the air kick** and **Down + Physical is the dive**; **Jump in the recovery of a grounded move is the bail-out hop** (away from the facing, or toward a held direction). Donut: Square/X is a weak swat, Triangle/Y the pounce, and **Jump in the recovery of her swat or pounce is her cat hop**. There is no special or ability in the room.

## 3. Play the room three times, once per approach

The room is one lane, 1152 px wide: spawn at x 128, the rotten shore (block) at x 384, a left perch at x 544 to 672, five goblins in two waves, a stone parapet, the llama yard at x 1000 and a right perch. Take at least three attempts, one for each approach below, and fill one scorecard row each. Restart with a Select hold between attempts (each restart redraws the goblins' temperaments).

### Attempt 1: direct

Fight on the ground with jab, haymaker, kick, stomp and the follow-through. Judge:

- **Basic combat fun.** Do the three ground branches (stomp on the spot, follow-through kick, ground finish) read at 1x without a new mandatory button or stun-lock? Do you want to repeat the room to vary the outcome?
- **Enemy-to-enemy impacts.** Does a kicked goblin knock a second one down? Does a kicked body that meets the end wall slam and crumple? Is the first and second body response visible and satisfying?
- **Enemy behaviour.** Did you notice at least two goblin reactions per attempt (hesitate, panic, taunt, enraged)? Did attempt 3 play differently from attempt 1? Can you say why a goblin fled?
- **Evasion.** Did you use the bail-out hop voluntarily? Did it make the goblin tells irrelevant, or does it still cost you?
- **Aerial play.** Did you use the air kick or the dive without prompting, and does it stay a spice rather than the main kill source?
- **Death readability and gore.** Can you name three causes of death (stomp, wall slam, flung, shore) without a log? Is the gore at preset 4 about right (3 of 5)?
- **Tone.** Does Carl look effortful, with a flinch when hit and frustration after missing? Does the assistant Donut look smug or startled (see the NOT RUN list)?

### Attempt 2: environment (shore and block)

Use the room: three jabs break the rotten shore at x 384 and the block falls. Lead goblins under it, let a goblin swing trip it, use the parapet and the perches against the llama. Note: with the default `--cf02` assistant Donut may kill or stagger the goblins before one swings into the shore (measured on the scripted plan: 0 shore trips with her, 3 without), so to rehearse the shore plan on its own start with `--companion off`. Judge:

- **Could you name the shore hazard without coaching?** Is the damaged shore readable before it falls?
- **Is the block fair?** It hurts every hero whose body is under it (1 damage, a short knock, 55 ticks of invulnerability), the partner included. Does standing under it feel like your mistake?
- **Goblin self-own.** When a goblin's swing trips a damaged shore and the block drops on its friends, is it funny and legible, and is the slow motion absent (it must never trigger)?
- **Llama answers.** Did you use at least two (jump the glob, duck behind the 48 px parapet, stand on a perch above the spit line, rush it during its tell, or Donut staying under the line)?
- **Room length.** How long did it take you? Take N from `CF02_ARENA_COMPLETE tick=N` and divide by 60, counted from the last restart. (The target is 40 to 60 s, see the NOT RUN list.)

### Attempt 3: perch and coop

Climb a perch (a jump lands on the 80 px slab), dive onto the goblins below, swap heroes (one player) or play with a second pad (two players). Judge:

- **Perch and dive.** Does the one-tile slab read as a place to stand and dive from? Does the dive feel good without becoming the only plan?
- **Solo swap.** Is the 12-tick swap with the eased pan an improvement? Is there any abrupt camera motion, and in which moment (swap pan, impact kick, zoom)?
- **Companion.** Does Donut act as a fighting partner (she engages goblins on her own, swats a tell, pounces, can finish an enemy you are not engaging) without making you redundant? Since CF02-DUO she may land the killing blow; the hit-share ceiling is 50 %, measured on the scripted `cf02_arena_direct` run, and one action takes at least 20 ticks.
- **Real 2P.** If you have two pads: does P2 Donut contribute, does the block hurt the partner as designed, and is the room completable together?
- **Checkpoint respawn.** After a defeat (the automatic respawn after about 1 s) or a Select tap, do you resume at the last checkpoint (x 128 or x 704)? Remember that a tap while alive also resets the goblins.

## 4. Open items you decide (adopted values)

Every value below was adopted by the orchestrator under the design-defaults directive, is data or a named constant, and can be changed in a follow-up PR after you play. Nothing is accepted.

| ID | Adopted value | What to judge |
| --- | --- | --- |
| Q19 | Solo swap cooldown 12 ticks (the legacy value is 20) | Swap feels responsive but not spammy |
| Q20 | Eased swap pan; the 40 px deadzone is kept, so a small pan is intended (ruling in #158); U1: the eased pan does not end while the new hero keeps running | Any abrupt or never-ending pan |
| Q13, Q33, Q39 | `--cf02` defaults to the `cf02_assist` companion; at least 3 starts per solo run (the arena run has 6) | Does Donut feel useful and not a thief |
| CF02-DUO-A01 to A10 | Owner feedback 2026-11-21 ("I expected Donut attacking") overrides the restrained assistant: kill policy `factual` (was `never`), one action per 20 ticks (was 60), hit-share ceiling 50 % (was 25 %), pounce and (after the biscuit) Magic Missile on her own initiative, the missile still behind the mana reserve of 2; review fixes: no offensive swat (A07), she never takes the last hit of the enemy Carl is engaging (A08), same-level targets only (A09), arena activity floor 2 (A10) | Does she now fight with you, and does she take too many kills. Values are in [the companion spec](../../03-tech/companion-ai-command-system.md#e-cf02-duo-adopted-defaults-ai-donut-as-a-fighting-partner) and `companion_ai_v0.json` |
| Q45 | Perch underside 68 px (slab top 80 px; Carl's 52 px clears by 16 px) | Perch climb and walking under it |
| Q29 | End walls are the drawn 16 px wall; the room is 1152 px wide with checkpoints at 128 and 704 | Wall slams read against the wall |
| Q30, Q42 | The landed block hurts every hero in its footprint: 1 damage, 18 hurt ticks, knock 120 / 160, 55 invulnerable ticks | Fairness of terrain that hurts |
| Q31, Q41 | A goblin swing on a damaged shore trips it (`ShoreTripped`); kill credit goes to the last hero who hit the shore; no slow motion | Authored comedy, never a cinematic |
| Q54, Q55 | Carl's bail-out hop 150 / 260; Donut's cat hop 170 / 300; no invulnerability (`q54_bailout_iframes` 0) | Does it keep tells meaningful |
| Q46 | Move buffer 8 ticks (6 is the legacy input buffer; the latency table is in [cf02-h](cf02-h/README.md)) | Attacks whiff on the buffer? |
| Q58, Q32 | Temperaments drawn from a bag (two brawlers, two cowards, one show-off) per restart; default seed `fixture` | Attempt 3 plays differently from attempt 1 |
| Q60 to Q68 | Nerve thresholds: panic 25, hesitate 50, enrage 85; panic 60 to 90 ticks; enrage 120 ticks | Reactions noticed and explainable |
| Q70 | Carl's frustration after 3 whiffs inside 150 ticks, pending 45 ticks from the third whiff (`FRUSTRATION_PENDING_TICKS`, adopted in #159), cooldown 600 | Frustration reads without text |
| Q71 | Donut's smug pending 45 ticks (`SMUG_PENDING_TICKS`, adopted in #159), cooldown 300; her startle clear shot exists only in one clip | Smug and startle readable (NOT RUN) |
| Q72 | Goblin taunt hold 6 ticks before a slammed sprite shows | Taunt then punish reads |
| Q22, Q43, Q36 | Gore baselines at preset 4 (stomped 5 gibs, wall crumple 3 plus an arm, flung 5 plus an arm); crush is catastrophic (D-DEATH-2); five audio cues (D-AUDIO-1: new deterministic DSP, not the legacy pack) | Gore 3 of 5; each cause has a distinct cue |
| A-1 (companion) | The assistant's follow layer does not dodge a swing aimed at the other hero | Does it ignore danger to Carl in a bad way |
| Q41 | No room fixture or autopilot run emits a cinematic (measured, 0 per attempt) | Do you ever get one, and does it feel earned |

Known limitations, so you do not mistake them for new findings (the backlog holds the follow-up rows): U1 above; the environment autopilot plan dies twice and does not finish the room (U4); a solo swap press is not seen by the acting overlay (FN3 residual), so a solo swap does not end Carl's overlay; Donut's smug was never shown readable in a scene (FU1); several 1x GIFs are hard to read because the actors are small; the AI pounce at the walking pace usually lands after the goblin's down timer and stands it up instead of holding it; a staggered goblin cannot be pounced.

## NOT RUN by the agents

Written without euphemism: none of this has been done, so none of it is verified.

- **Human play.** Nobody has played the room, in any mode. Every game-feel question above is open.
- **The live party with the assistant in combat.** The windowed `--cf02` run with a live party and the `cf02_assist` companion was run only idle (Carl stood still, no goblin came within sight, so no combat happened). The assistant in combat is covered only by the headless party fixtures (`cf02_arena_direct`, `_swap`, `_coop`, `_three_attempts`), which use the same party routing but not the windowed loop. The `--autopilot` run is a single-hero path with no Donut and no assistant.
- **Real two-pad 2P.** The two-player room is verified headless (`cf02_arena_coop`), not with two physical pads.
- **Frame time on real hardware.** NOT RUN. The agents ran only software rendering under Xvfb and headless replays, which say nothing about the target hardware. Use `--perf-frames N` on your machine if you want numbers; no number is claimed here.
- **Owner audio listening.** The five CF02 cues were generated and checked by a script and a QA report, never listened to by a human.
- **Donut's smug never shown readable in a scene.** Her startle clears in one clip; her smug has only the sheet and state lines (FU1).
- **The 40 to 60 s room length.** Unmeasured for humans. The scripted autopilot clears the room in about 8 to 12 s (it reached `CF02_ARENA_COMPLETE` at tick 487, 8.1 s, in the re-verification), but that is Carl alone on the single-hero path, so it proves the room is completable and gives a lower bound, not that it lasts 40 to 60 s for a person.
- **The environment autopilot plan (U4).** The scripted environment plan (break the shore, let the block fall) dies twice and does not complete the room that way. The environment route is verified by its individual fixtures (shore states, block hurts, self-own) and a headless "completable in every shore state" test, not by an end-to-end scripted clear.
- Cinematic count for a human (Q41): only the scripted plans were measured (0 per attempt).
- Real-pad button feel (Down on the stick or D-pad, bail-out timing) and the checkpoint respawn on a pad.

## What to send back

Fill in [`playtest-scorecard.md`](playtest-scorecard.md) (one header, one row per attempt, the ratings and any stop signal) and attach the `CF02_CONFIG` line of each attempt. Change any adopted value above by naming its ID; a follow-up PR will edit the data row.
