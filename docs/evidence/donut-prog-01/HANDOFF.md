# DONUT-PROG-01 owner handoff (DP-F)

Everything below is merged behind flags (`--donut-prog`, `--donut-light-feet`, `--donut-puddle`); a run without them is unchanged. Gameplay and feel are **merged, game-feel review pending**: nothing here is accepted until the owner plays it.

## What to play

| Command (debug build, repository root) | Shows |
| --- | --- |
| `floor01_scene --cf02 --donut-light-feet` | Light on Your Feet held from tick 0 (microarena, no 128 px surface: feel only) |
| `floor01_scene --cf02 --donut-puddle` (P2 on a pad) | the blink: Circle/B with the stick away from a threat |
| `floor01_scene --donut-prog` | the grants: the blink on the Juicer's defeat; the boost's zone is armed in the `rest_refuge` room since REST-WIRE (the legacy corridor arms nothing; `--donut-light-feet` still holds the boost there) |
| `floor01_scene --party-fixture donut_prog_arena_blink_cornered` (headless, or `--capture-sequence`) | the AI Donut's escape |

## Every GIF (real Bevy renders, `SHA256SUMS`)

DP-D: `dp_d_gif1_grants.gif` (the two notices), `dp_d_gif2_cycle_1x.gif` (the 9 s socket cycle), `dp_d_gif3_flat_3x.gif` (flat blink), `dp_d_gif4_wall_3x.gif` (stopped by a wall), `dp_d_gif5_fizzle_1x.gif` (arrival fizzle), `dp_d_gif6_refusals_3x.gif` (no mana, no destination), `dp_d_cues_sheet.png`. DP-F: `dp_f_gif1_ledge_128_side_by_side.gif` (128 px ledge, without and with the boost), `dp_f_gif2_ai_escape_3x.gif` and `dp_f_escape_sheet.png` (the AI blinks out of a swing).

Not captured: the 2P separation limit as a GIF (covered by the DP-C fixtures), the Juicer recordings (the render loop cannot run the boss pilot; the headless replays `donut_prog_boss_pins` and `donut_prog_boss_both_programs` show the grant tick 311), a real two-pad check (**NOT RUN**).

## Diagrams

Jump arc (Donut, standing jump; DP-Q14, DP-Q18):

```
 px above the floor
 138 |            ___ boosted apex 137.95
 128 |  - - - - -/ - \- - - - -[ledge top, 128]      margin 9.95 px
 115 |        __/ base apex 114.92 \__
     |      _/                       \_              the base jump falls 13 px short
   0 |_____/_________________________\______  launch 500 px/s base, 547 px/s boosted
```

Blink timeline (ticks from the press; DP-Q24, DP-Q25):

```
 t0 press   t1..t5 windup   t6 arrival   t7..t11 recovery   t12 free   ... t546 ready
 |crouch    |ripple at feet |pay 2        |no presses        |           cooldown 540 from the arrival
 a primary / physical / jump press on t0..t5 cancels it (no cost); a hurt in t0..t6 cancels
```

Destination search (DP-Q27, DP-Q28): candidates 112, 108, ..., 16 px along the stick, the farthest wins; a candidate is dropped when it crosses a solid or `blink_barriers`, an unfired Donut zone, a hazard, leaves the view by more than the 24 px inset, or leaves the 2P separation limit; the AI also requires it inside Carl +/- 63 px (the hull) and the free zone. The search runs again on t6 from where she is.

## Scorecard

| Field | Value |
| --- | --- |
| Build | the head of the DP-F PR (`SHA256SUMS` lists the capture checksums; the PR body carries the SHA the local checks ran on) |
| Flags | `--donut-prog`, `--donut-light-feet`, `--donut-puddle`; AI blink only under `cf02_free` |
| Pad | none (headless pilots); the real pad check is **NOT RUN** |
| Resolution | 640 x 360 captures; the 640 x 400 view is covered by the drawn-view tests |
| Deaths and restarts | arena boost pilots 0 / 0; blink pilot 7 / 7 (same-stance control without the blink 0 / 0; the shipped trailing pilot 11 / 11, reference only); boss pilots 0 / 0 |
| Blinks and fizzles | blink pilot 15 resolved, 0 fizzled, 0 cancelled |
| Mana spent | blink pilot: 15 blink payments (2 charges each) and 81 missile spends |
| Dry presses | blink pilot 16 (same-stance control 14): missiles refused at 0 mana or blinks rejected |
| Open questions | the boss charge blink is kept as a guard behind the jump dodge (owner may want a boss window); the blink pilot's 32 px lead; owner game-feel note: blinking out of swings instead of swatting cost deaths in the scripted coop pilot (the swat and pounce are held back whenever a swing aims at her) |
