# FLOOR01-BOS: local agent work order (BOS-A, then BOS-B, BOS-ART, BOS-C)

**Task.** Make the Ball of Swine playable as room 18 (`bos`) of the floor run, and end the floor through its exit and the floor-end panel. The numbers, contracts, pins, fixtures and gate are in [the BOS integration plan](../01-design/floor-01-bos-integration.md). This order only sequences them, and cites the plan's sections as "plan 6.4" and so on. The result is a labelled greybox: "merged, game-feel review pending".

**Provenance.** This order comes from the owner draft PR #192 (`docs/07-agents/floor-01-ball-of-swine-finale-workorder.md` on branch `design/f01-ball-of-swine-finale`). #192 stays open and unmerged as the record of the original text.

- **Kept from #192:**
  - the core beat (tell, committed roll, impact, recovery, punish);
  - the fairness rules (every hit telegraphed, no mandatory phase, no immunity, no camping);
  - "the exit opens only on a real defeat";
  - the duty to test autonomously.
- **Changed:**
  - stages BSW-A..G collapse into one Sonnet PR (BOS-A) plus three named follow-ups;
  - the wide arena, the central gate, the Tusklings and the cinematics are deferred by ID (plan 12).

## BOS-A (Sonnet, one PR)

### Read and audit first

Follow `AGENTS.md` and [the orchestration guide](agent-orchestration.md) (speed mode, local-validation merge). Audit before you edit:

- `src/boss.rs`:
  - `BossData` and `validate`;
  - `Boss::new`, `neutral`, `reachable`, `charge`, `crash_pivot`, `clamp_walk`, `start_roar` (never reached: `queue_roar` is false);
  - the rack code, `BossSetup`, `builtin()`, `boss()`;
  - where boss state is mixed into `COMBAT_STATE`.
- `src/boss_scene.rs`: `arena(root, dressed)`, `load_arena_undressed`, `gate_block`, `wall_block`, the exit rect, and the `BossSetup` consts.
- `src/f01_late.rs`: `RoomScene`, `check_rules`, TROG-R1, `MANIFEST_HASH`, `SCOPED_PIN_KEYS`.
- `src/f01_run.rs`: `next_room`, `floor_end_panel`, and the tests that treat `mastery` as last (search for `"mastery"` and `FloorEnd`).
- `src/bin/floor01_scene.rs`: the per-room dispatch (`RoomScene::Boss => ...`) and how the ledger `SceneKind` is chosen for a boss run.
- `src/story_unlock.rs` and `assets/data/gameplay/story_unlocks_v0.json`: how `brute_boss_f01_defeated` is armed.
- The `trog_eval` and `corral_eval` harnesses, which `bos_eval` copies.
- **Room-18 plumbing (plan BOS-AD33):**
  - `scripts/f01_run_smoke.sh`: the room-count default (17) and the `F01_BOS_SMOKE_OK` marker rule;
  - `src/room_pilots.rs`: the per-room pilots (`juicer`, `mastery`);
  - `src/playtrace/replay.rs`: the hard-coded "boss" source (`load_arena`, `BOSS01_SCENE`) and the label check `("boss", Some("juicer"))`;
  - `story_setup` in `src/bin/floor01_scene.rs` (the `fixture == BOSS01_SCENE` test) and its replay twin.
- Presentation: `src/hud_adapter.rs` (`EMBLEM_ASSET`), `src/boss_art.rs` (the sprite and the defeat presenter on `BOSS_ENEMY_ID`), `death.rs` (the species fallback).

### Coordination rules

- **The Juicer is frozen.**
  - Do not edit `assets/data/bosses/brute_boss_f01.json`.
  - Do not move a Juicer fixture, golden or `COMBAT_STATE` hash.
  - Every new `BossData` field is optional with a default that keeps the Juicer's behaviour.
  - New hashed state is mixed in only when `data.siege` is Some (plan 6.4).
- **No forks:**
  - extend `src/boss.rs` and `src/boss_scene.rs`; do not copy them into a second boss module;
  - select the data through `BossSetup` (a `Copy` enum, for example `BossKind::{Juicer, Ball}`) and a `ball()` `OnceLock` loader next to `builtin()`.
- Engine identifiers stay IP-neutral (`rolling_boss_f01`, `RoomScene::Ball`, `siege`). "Ball of Swine" appears only in data strings and docs.
- Do not edit `docs/04-production/backlog.md`. Put the status in the PR body.
- **Stop and report**, and do not re-pin, if any item in plan 10 "must not move" moves.

### Deliverables

| # | Deliverable | Plan |
|---|---|---|
| 1 | `assets/data/bosses/ball_boss_f01.json` with the values of plan 5. `BossData` gains optional `arena.hero_right_wall_x` and `siege { warn_ticks, after_ticks, debris_damage, debris_launch }` | 5 |
| 2 | Siege-gated alcove reachability, the `unreachable_ticks` counter, the warning notice, the siege flag on the charge, and debris on a siege crash | 6.1, 6.4 |
| 3 | `boss_scene` builds the arena from the selected data: `hero_right_wall_x` drives the hero wall, the exit and `wall_block`; greybox arch outlines (no collision); the post and weight through the rack presenter; a 120x124 greybox ball body with a label. The Juicer emblem, sprite and defeat presenter are **off** for `BossKind::Ball` (BOS-AD31) | 4, 6.5, 17 |
| 4 | `RoomScene::Ball` (`"f01_ball"`) mapped to `RoomRules::Boss` (one new `check_rules` arm); `--scene f01_ball` standalone; the run dispatch with its **own fixture constant**; `story_setup` maps that fixture to `SceneKind::F01Late` with the mastery loadout; the replay's scene mapping matches; a `bos` pilot in `src/room_pilots.rs` | 3, BOS-AD33 |
| 5 | Manifest: room 18 `bos`, the story_order row, the appended `status` step, and `MANIFEST_HASH` re-pinned; `scripts/f01_run_smoke.sh` default 18 and the `F01_BOS_SMOKE_OK` marker for `bos` (or a `juicer`-style exemption); the BOS-AD32 test data | 3, 10 |
| 6 | `floor_end_panel`: the "FLOOR 1 CLEARED" heading after `bos`, plus the fact line from `hud.display_name`. The panel has no boss data today, so it reads the name from `boss::ball()` inside the function (no signature change) | 9.5 |
| 7 | Fixtures F1..F18, including F8b, F17 (alcove edge) and F18 (PLAYTRACE `bos` round-trip, with `src/playtrace/replay.rs` gaining the Ball source and label; Juicer replay goldens unchanged) | 9.2 |
| 8 | `bos_eval` with rows G-0..G-7 (G-6 and G-7 are solo, with the live M1 bot) and the validity rule. If G-6 or G-7 fails, switch on BOS-Q11 and record it | 9.3 |
| 9 | Packet edits on lines 3, 7, 77, 83, 112 and 216, plus the new room-18 row and checklist item | 14 |
| 10 | Evidence: a contact sheet or GIF of one bait-and-punish cycle, one siege, the weight drop, the exit opening and the floor-end panel, plus the `bos_eval` table and the F11 clear time in the PR body | |

### Local check list (all of it must pass on the final head)

- `cargo fmt --check`;
- `cargo clippy --all-targets -- -D warnings`;
- `cargo test`;
- `bos_eval` (G-0 clears, and G-1..G-7 pass and are valid);
- the existing Juicer, `trog_eval` and `corral_eval` evals still pass;
- `python3 scripts/check_docs.py`;
- `scripts/f01_run_smoke.sh` with 18 rooms, all `status=ok`, plus the in-process `on_exit(bos)` panel test (F16).

### Cut order (if BOS-A runs long)

Cut from the top, record each cut in the PR body, and never cut the floor exit or the gate:

1. the evidence GIF (keep the stills);
2. the siege warning notice text (keep the turn-to-face and the timing);
3. F10, the chain weight: ship the weight disabled through `BossSetup` (as the Juicer's `FULL_NO_RACKS` does) and move it to BOS-B.

### PR body

The PR body must include:

- the behaviour;
- the evidence;
- the pins moved: `MANIFEST_HASH` old > new;
- the pins proven unchanged;
- the adopted defaults by BOS-AD and BOS-Q ID;
- the measured F7 hit count and F11 clear time;
- any stun change made under plan 6.2;
- the risks;
- the status line "merged, game-feel review pending".

Cite #192 and this plan. Under Rights/Provenance, record that no new art was added.

## Follow-ups (named, not scheduled)

| ID | Scope | Pre-requisite | Tier |
|---|---|---|---|
| **BOS-B** | TUSK-01 Tusklings on ENEMY-SLOT (at most 3 live, 1..2 per qualifying impact, a budget of 4..6, Carl's kick-stagger); bot alcove-flee (BOS-Q4); the `f01_ball_defeated` ledger fact (BOS-Q7); crash self-damage (BOS-Q10); policies `carl_environment` and `duo_coop` | BOS-A merged; a short top-tier mini-spec for the Tuskling numbers from #192 | Sonnet |
| **BOS-ART** | SWINE-01 atlas export from `swine-01.ase` and presenter wiring; arch and chain-weight props from the catacomb kit; the stairwell behind the exit wall | BOS-A merged | art pipeline (top-tier keys for the weight prop only, medium-tier execution, independent top-tier review) |
| **BOS-C** | the wide arena and camera pullback, the central suspended gate and outer support, the L0..L4 reliefs, the intro and exit beats (#192 E0, E6) | BOS-B; a top-tier design pass | top-tier design, then Sonnet |
