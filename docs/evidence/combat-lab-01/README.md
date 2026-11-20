# COMBAT-LAB-01 LAB-01 / LAB-00-wire evidence

**Status: headless asserts pass and the live entry runs; not game-feel accepted.** The stills below are 640x360 virtual-view captures of the R00 to R02 greybox rooms on the **legacy rules** (the lab refuses `cf02_rules`), made by a scripted stand-in player (`--lab-autopilot`), not by a human. A human clear of R00 to R02 in solo and co-op (hard gate 1) is **not tested yet**, and the captured runs are scripted, so nothing was recorded (the badge reads `REC off (scripted run)`). A live run records a PLAYTRACE session with source `combat_lab`; the badge shows `REC`, `REC ERROR: <reason>` or `REC off (<reason>)`, and each relaunch closes and saves the session first.

## How to play it (the owner's command)

```bash
cargo run --bin floor01_scene -- --scene combat_lab
```

The Setup Cage (R00) opens with its menu. Rows: **Profile** (fixed, read-only), **Mode** (`solo_carl`, `solo_donut`, `coop_local`), **Rules** (`legacy`; `cf02_rules` is greyed out), **Variant** (`v1` to `v3`, R01 only, effective seed 1 to 3), **Start room** (`R01`, `R02`), **Start**.

| Input | Keyboard | Pad |
| --- | --- | --- |
| Menu row / value | `W` `S` / `A` `D` (or arrows) | d-pad up, down / left, right |
| Menu confirm | `Enter` | `A` (South) or `Start` (never quits while a panel is open) |
| Move, jump | `A` `D`, `Space` | stick or d-pad, `A` |
| Punch, kick | `J`, `K` | `X`, `Y` |
| Solo swap Carl and Donut | `Q` | `LB` |
| Respawn at the checkpoint (tap) / restart the room (hold 1 s) | `R` | `Select` |
| Quit | `Esc` | `Start` on the P1 pad |

**Two players (`coop_local`).** The keyboard is P1 (Carl) and merges with the first pad. P2 (Donut) joins with a **second gamepad: press `Start` on it**. Until it joins, the strip says `P2: Start on pad 2 (again each room)` and Donut is the follower bot. **Every room is a fresh launch, so P2 presses Start again in R00, in R01 and in R02** (a Start pressed while the menu is open is lost). Auto-joining a connected second pad is a follow-up. P1 on a pad plus P2 on a second pad works the same way (the first pad owns P1 unless the keyboard already drove it).

One room runs per launch. The Setup Cage confirming, an exit, and the end panel start the scene again with the next selection (`LAB_RELAUNCH` is printed). The exit opens when the room is clear and fires when the controlled hero (solo) or both heroes (co-op) stand in it. R00 -> chosen start room; R01 -> R02; R02 -> a factual end panel, then the Setup Cage.

Direct room entry (flags, any order; `--deck` selects the 640x400 view, `--scale N` the window):

```bash
cargo run --bin floor01_scene -- --scene combat_lab --lab-room R02 --lab-mode coop_local
cargo run --bin floor01_scene -- --scene combat_lab --lab-room R01 --cf02-seed 3     # seed 3 = variant v3
cargo run --bin floor01_scene -- --scene combat_lab --lab-room R01 --lab-variant v2
```

`--cf02-seed N` is the CF02-Q58 seed override and is allowed with `--scene combat_lab` only (plain `--cf02-seed` still needs `--cf02`). On R01 the seed **is** the variant (`1` to `3`; any other value is refused with the valid list); R00 and R02 have one layout and only record the seed. `sandbox` probes and the held-out R01H are CLI-only in the headless runner, never in the menu.

## What is here

| Path | What |
| --- | --- |
| `sheets/` | One contact sheet per run (frames left to right, top to bottom, two per row) |
| `frames/` | The 640x360 frames and the `COMBAT_STATE` lines at the first and last frame |
| `../../../scripts/combat_lab_evidence.sh` | The regeneration script |

Runs: `r00_solo_carl`, `r00_coop`, `r00_menu` (the Setup Cage menu open), `r01_solo_carl`, `r01_coop`, `r01_solo_donut` (spawn only), `r02_solo_carl`, `r02_coop`, `r02_solo_donut` (spawn only).

Regenerate (needs `xvfb-run`, ImageMagick and an exported `generated/art`):

```bash
cargo build --bin floor01_scene
xvfb-run -a -s '-screen 0 1280x800x24' bash scripts/combat_lab_evidence.sh
```

The binary renders at `--scale 2` (a scale-1 window renders black under xvfb on the capture machine, also for `--demo`), and each frame is reduced 2:1 with a point filter, which is exactly the 640x360 view.

## What the captures show, and what they do not

- The auto camera follows the pair; both heroes stay in frame in every co-op frame, the exit portal appears at the end of each room, and the strip (bottom 26 px) and the HUD do not overlap. The REC badge sits clear of the HUD corner emblem (it overlapped it in the first capture; fixed).
- The `--lab-autopilot` stand-in jabs goblins and kicks the llama; in R02 it dies once to the llama in the yard and respawns at the checkpoint (see the `deaths` counter in the strip). That is the policy, not a balance statement. In co-op the second slot follows 48 px behind Carl so the 448 px party wall never holds the pair; with an idle second human the wall holds Carl at x about 590 in R02, as designed (spec 3.5).
- A scripted run never leaves its room (the exit state is shown, the transition only happens in a live player run).
- 640x400 and 1080p stills, GIF clips with the kick-into-wall, the bait-drop-punish loop, the Donut ledge pounce and the llama glob cases (spec 9.2) need a human or a dedicated probe and are **not** produced here.
- Observation, not a defect of this PR: the rooms are 660 px tall, so the vertical camera follows jumps (R00 frames at ticks 85 and 125).
