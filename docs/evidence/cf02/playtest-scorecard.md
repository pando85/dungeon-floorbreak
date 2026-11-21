# COMBAT-FUN-02 playtest scorecard (CF02-REVIEW)

Fill in one copy per session. It extends the CF02-00 [template](scorecard-template.md) (kept as written) with the build and flags already filled in for the final main. **Nothing below the pre-filled lines is filled in by an agent.** A missing field is written `NOT RECORDED`, never guessed. Instructions and commands: [owner handoff](owner-handoff.md).

## Pre-filled for this build

```text
Build SHA tested by the agents (main with CF02-I merged):  9dbf5f54ed36f399c91f00573438342873df2114
Build SHA you play (git rev-parse HEAD of YOUR checkout):   <fill in; the PR head or the merge commit of CF02-REVIEW supersedes the line above>
Pinned CF02 data hash (the CF02_CONFIG line):              tuning_hash=0x2da48577fa64315e  schema_version=1
Flags of the reference run (the launch the agents describe):  --cf02   (rules on; the room; the cf02_assist companion for Donut; 12-tick swap cooldown; eased swap pan)
What the agents verified of it:                              headless (the party fixtures) and one idle live run; the live windowed party with the assistant in combat was NOT run
Flags you may add:                                         --cf02-seed N   --companion off|cf02_assist|v0   --camera auto|scroll|1.0|1.25|1.5   --gore-preset 1..4 (default 4)   --cinematic-finishes[=on|off]   --scale 1..8 (3 = 1920x1080)   --deck (640x400)
Defaults when a flag is absent:                            seed=fixture (the CF02_CONFIG line prints it), camera auto, gore preset 4, cinematic finishes on in live play, recording on
```

## Session header (mandatory)

```text
Date:
Command line (paste it whole, with every flag):
cf02_rules on (--cf02):                 yes / no
Effective seed (the CF02_CONFIG line):  <u64> or `fixture`
Device and pad (keyboard, model of each pad, one or two pads, which pad owns P1):
Output resolution (e.g. 1920x1080 at --scale 3, 640x400 with --deck):
Gore preset (1-4):
CINE-01 config (cinematic finishes on/off; the flag or FLOORBREAK_CINEMATIC_FINISHES used):
Camera mode (auto / scroll / 1.0 / 1.25 / 1.5):
Companion (cf02_assist default / off / v0, or a second human as Donut):
Approach played this session (direct / environment / perch-coop):
Session length:
Playtrace session id (printed as PLAYTRACE_REC; optional, stays local):
Release or debug build:
```

## Per attempt (one row per attempt of the room; at least three, one per approach)

| Attempt | Approach | Deaths (hand tally, or count `kind=hero_defeated` lines; not `DEMO01_RESPAWN`) | Restarts (Select hold) | Cinematics seen (CINE moments, count) | Goblin reactions noticed (hesitate, panic, taunt, enraged) | Kills by kind (jab/haymaker, kick, stomp, air, dive, wall slam, shore, Donut) | Time to clear (s) = N / 60 from `CF02_ARENA_COMPLETE tick=N`, since the last restart | Reached `CF02_ARENA_COMPLETE`? |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | direct | | | | | | | |
| 2 | environment (shore and block) | | | | | | | |
| 3 | perch / coop | | | | | | | |

The effective seed and the number of restarts give the attempt's temperament draw (Q58). Record the `CF02_CONFIG` line and the attempt number. The agents' reference for the cinematic count (Q41) is **0 moments per attempt for every scripted plan**; a human count above 0 is a finding, not an error.

## Ratings (1 to 5; targets from the [design](../../01-design/combat-fun-02-post-demo01-proposals.md#5-acceptance-measurement-and-stop-criteria))

| Item | DEMO-01 baseline | Target | Rating | One line |
| --- | --- | --- | --- | --- |
| Basic combat fun | 3 | >= 4, and you repeat the room to vary the outcome | | |
| Enemy-to-enemy impacts (first and second body, wall slam) | 1 | >= 4 | | |
| Enemy behaviour: did attempt 3 play differently from attempt 1? At least two goblin reactions per attempt? | not rated | yes / yes | | |
| Evasion, goblins and llama (bail-out used voluntarily; at least two llama answers used) | 3 | >= 4 | | |
| Aerial play (air kick or dive used without prompting; not the main kill source) | n/a | yes | | |
| Environment: could you name the shore hazard without coaching? Did the block feel fair when it hurt a hero? | not rated | yes / fair | | |
| Gore intensity (1 too tame, 3 right, 5 too much) | 1 | 3 | | |
| Death readability (name three causal classes without logs) | 3 | >= 4 | | |
| Solo swap (12-tick cooldown, eased pan) | 2 | >= 4 | | |
| Companion (assistant): does Donut contribute, and not steal the kills? | 2 | >= 4 | | |
| Identity and tone (Carl effortful, Donut smug, goblins with moods) | 2 | >= 4 | | |
| HUD | 3 | >= 4 | | |
| Camera: any abrupt motion? Which moment (swap pan, impact kick, zoom)? | "movimientos bruscos" (abrupt movements) | none; a measured trace | | |
| Room length: was it about 40 to 60 s when you played it? | not rated | yes | | |
| Checkpoint respawn (automatic after a defeat, and a Select tap; a tap while alive also resets the goblins) | not tested | must be tested | | |
| Real two-pad 2P | not tested | must be tested | | |
| Audio (the five CF02 cues on your speakers) | not heard | each cause reads | | |

## Stop or simplify signals (section 5 of the design)

Tick any that happened and say where: attack variants mostly whiff on the buffer; Down + Physical fires by accident; the companion lands more than half of the hits (CF02-DUO-A03); chain contacts repeat damage; a goblin reaction makes a room last over 60 s or you cannot say why a goblin fled; air moves account for more than half the kills; the bail-out makes tells irrelevant; the camera hides a threat; effects conceal the attacker; the room needs an unapproved ability.

## Fields you may leave as NOT RUN

Write `NOT RUN` for anything you did not do this session: real two-pad 2P, the checkpoint respawn, the sounds, frame time on the target hardware, the 40 to 60 s room length. See the NOT RUN list in the [owner handoff](owner-handoff.md#not-run-by-the-agents).
