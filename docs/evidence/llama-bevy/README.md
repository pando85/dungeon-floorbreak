# Bad Llama in Bevy: evidence (DEMO-01 PR B, draft)

Real captures of the Bad Llama (LLAMA-01 art, LLAMA-02-DEATH poses and parts) in the `floor01_scene` render loop, at **1920x1080** (`--scale 3`, lighting on, one frame per tick). Written by `bash scripts/make_llama_bevy_evidence.sh` (debug binary, exported atlases, `xvfb-run`, ImageMagick); the README is hand-written.

**What this is not.** The captures are the real Bevy render loop under Xvfb with software rendering. They show that the pieces are drawn where and when the simulation says; they are **not** a game-feel review, not a frame-time or memory measurement and not a Steam Deck claim. Every llama number is a playtest proposal (`src/feel.rs`), the art is `prototype_fixture_unapproved`, and the owner has not played it. The script also checks that the state each capture ends on equals the headless replay's (`COMBAT_STATE` hash), so the pictures are of the committed replays.

## Fixtures and hashes

The same final `COMBAT_STATE` hash in the headless replay, in the live render run and at gore presets 1, 2, 3 and 4 (`bash scripts/ci_floor01_combat.sh headless`, then `render` under Xvfb). The logs are `headless_*.txt` here (copies of `docs/evidence/cp01-feel/`).

| Fixture | What it shows | Hash | Captured ticks |
| --- | --- | --- | --- |
| `cp05_llama_spit` | tell, glow peak, release on tick 37, glob in flight, hit on Carl (76); second spit 187, hit 226 after his invulnerability | `5984ba11b72b53dc` | 30 to 90 |
| `cp05_llama_slam` | three push kicks; the first lands in the tell (33) and cancels it, the lethal third (163) is `force=heavy`: the slam pair | `611c0e44dbcef788` | 158 to 207 |
| `cp05_llama_down` | jab, haymaker, jab; the lethal jab (63) is routine: the plain LLAMA-01 `down` | `502be52b0c0ce4a4` | 58 to 107 |
| `cp05_llama_crush` | the shore block lands on a passive llama (139): the stone is drawn over it on crush frames 0 and 1 | `e151c497b2dd7506` | 136 to 165 |
| `cp05_llama_offscreen` | the llama is outside the simulation's 640 px view: it walks to its leash, never tells (no capture; the log shows no `llama_tell`) | `9e657acd442f059b` | none |

## Files

* **Stills, 1920x1080** (`still_*.png`): the tell glow (tick 30), the release (37), the glob in flight (55), the hit on Carl (77); the slam contact (164) at preset 4, the broken heap (190) at presets 4 and 1; the plain down at presets 4 and 1 (tick 100); the crush with the stone over the llama (139), the flattened frame (142) and the remains (150).
* **Sheets** (`sheet_*.png`, 640x360 cells): the spit, the slam at presets 4 and 1 side by side, the plain down at presets 4 and 1, the crush.
* **GIFs, 960x540, 30 fps** (`*_960x540.gif`): the spit, the slam at presets 4 and 1, the plain down at preset 4, the crush at preset 4.
* `headless_*.txt` (the committed replays) and `SHA256SUMS`.

## What to look at

* **The tell and the fair-fire rule.** The glow frame (`attack_tell` frame 8) holds for 28 ticks before the glob leaves; the `cp05_llama_offscreen` log has no tell at all.
* **The slam rule.** Kick kill: the llama is thrown onto its back (`slam_contact` for 6 ticks), then the stiff-legged `broken_heap`; jab or haymaker kill: the LLAMA-01 `down` frames (knees fold, floor impact, flat). Presets 1 and 4 have the same silhouettes: preset 1 has no blood sprite but keeps the **lava** (the spit-gland drop and the pool of lava), the tooth and the wool tufts.
* **The stone over the llama.** On the landing tick and the two after it the block sits on the squashed llama (its anchor on the mirrored `hit_origin`, the flatten frame 3 px lower), is drawn over the llama and below Carl and Donut; from the remains on, the block is back at its simulated position on the floor.

## Known limits

* Not drawn or not wired: the llama's decapitation and explosion, the molten crust swap after 240 ticks, the slam's impact dust, any llama sound, a debug box for the llama.
* The llama never launches and never touches a goblin; Carl passes through its body (the same as a goblin).
* The slam and down finishes are lit by the same A1 pass as the goblin's; no lighting change.
* No 640x360 captures (the 1920x1080 frames are 3x of the 640x360 virtual frame, shown as 640x360 cells in the sheets).
* Rights and provenance: the art is the existing LLAMA-01 and LLAMA-02-DEATH sets under the recorded `ART-PUBLICATION-ALL-V1` scope; this PR adds no art. Code and captures: Claude Sonnet (medium tier), executor of the DEMO-01 plan written by the top tier; an independent top-tier review is pending.
