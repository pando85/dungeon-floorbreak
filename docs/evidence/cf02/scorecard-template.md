# COMBAT-FUN-02 playtest scorecard (template)

Copy this block into the owner's answer for each playtest session. **Nothing here is filled in by an agent:** the DEMO-01 report did not record build, flags, device, resolution, duration, deaths or gore and CINE settings ([playtest results](../../04-production/demo-01-playtest-2026-11-20.md)), which is why this template makes them mandatory. A missing field is written `NOT RECORDED`, never guessed.

## Session header (mandatory)

```text
Date:
Build SHA (git rev-parse HEAD of the tested build):
Command line (all flags, including --cf02, --cf02-seed N, --camera, --scale, --gore-preset, --demo/--cf02):
cf02_rules on (--cf02):                 yes / no
Effective seed (the CF02_CONFIG line):  <u64> or `fixture`
Device and pad (keyboard, model of each pad, one or two pads):
Output resolution (e.g. 1920x1080 at --scale 3, 640x400):
Gore preset (1-4):
CINE-01 config (cinematic finishes on/off; the env var or flag used):
Camera mode (auto / scroll / fixed):
Session length:
```

## Per attempt (one row per attempt of the room, at least three for `--cf02`)

| Attempt | Deaths | Restarts (Select hold) | Cinematics seen (CINE moments) | Goblin reactions noticed (hesitate, panic, taunt, enraged) | Kills by kind (jab/haymaker, kick, stomp, air, dive, wall slam, shore, Donut) | Time to clear (s) |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | | | | | | |
| 2 | | | | | | |
| 3 | | | | | | |

The effective seed and `restarts` give the attempt's temperament draw (Q58); the run logs both, so record the `CF02_CONFIG` line and the `restarts` value of the `COMBAT_STATE` line at the start of each attempt.

## Ratings (1 to 5; targets from the [design](../../01-design/combat-fun-02-post-demo01-proposals.md#5-acceptance-measurement-and-stop-criteria))

| Item | DEMO-01 baseline | Target | Rating | One line |
| --- | --- | --- | --- | --- |
| Basic combat fun | 3 | >= 4, and the owner repeats the room to vary the outcome | | |
| Enemy-to-enemy impacts (first and second body) | 1 | >= 4 | | |
| Enemy behaviour: did attempt 3 play differently from attempt 1? At least two goblin reactions noticed per attempt? | not rated | yes / yes | | |
| Evasion, goblins and llama (bail-out used voluntarily; at least two llama answers used) | 3 | >= 4 | | |
| Aerial play (air kick or dive used without prompting; not the main kill source) | n/a | yes | | |
| Gore intensity (1 too tame, 3 right, 5 too much) | 1 | 3 | | |
| Death readability (name three causal classes without logs) | 3 | >= 4 | | |
| Solo swap | 2 | >= 4 | | |
| Companion (assistant) | 2 | >= 4 | | |
| Identity and tone | 2 | >= 4 | | |
| HUD | 3 | >= 4 | | |
| Camera: any abrupt motion? Which moment (swap pan, impact kick, zoom)? | "movimientos bruscos" | none; a measured trace | | |
| Checkpoint respawn | not tested | must be tested | | |
| Real two-pad 2P | not tested | must be tested | | |

## Stop or simplify signals (from section 5 of the design)

Tick any that happened and say where: attack variants mostly whiff on the buffer; Down + Physical fires by accident; the companion lands more than a quarter of the hits; chain contacts repeat damage; a goblin reaction makes a room last over 60 s or the owner cannot say why a goblin fled; air moves account for more than half the kills; the bail-out makes tells irrelevant; the camera hides a threat; effects conceal the attacker; the room needs an unapproved ability.

## What the next playtest must still NOT claim

Anything not run: write `NOT RUN` for real two-pad 2P, the checkpoint respawn, the new sounds (the owner listens on his own speakers) and frame time on the target hardware.
