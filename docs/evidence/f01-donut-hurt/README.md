# Donut's hurt, knockback and knockdown frames in the Floor 1 scene

Screenshot evidence that Donut's damage reactions are drawn from the committed ANIM-DONUT-02 melee atlas (`donut_h40_melee.png`, frames 8 to 12) instead of a white flash and a 50% blink. **No sprite was drawn or changed.** The presentation is `donut_art::hurt_frame` (pure), stepped each tick in `CombatAnimators` (`src/combat_anim.rs`, the same shape as Carl's `CarlInput` and `carl_pose`) and drawn by `sync_donut_visuals` in `src/bin/floor01_scene.rs`. Game feel is pending.

`scripts/make_donut_hurt_evidence.sh` renders the real Bevy window under xvfb at 1080p (`--scale 3`) on the two committed party fixtures that hurt and defeat her, at six ticks, **twice**, and requires the two passes to be byte-identical. Each still is a pure crop of the scene pixels around her (no resampling); `strip_x6.png` lays the six crops out enlarged 2x with nearest-neighbour, which is 6x the 640x360 canvas.

| Still | Fixture, tick | Frame (melee atlas) | What it shows |
| --- | --- | --- | --- |
| `hurt_h0.png` | `donut_hurt_iframes`, 68 | 8 `h0 flinch` | a goblin's swing landed on tick 64; the 4 tick freeze and the first 5 flinch ticks, eyes shut |
| `hurt_h1.png` | `donut_hurt_iframes`, 80 | 9 `h1 recoil` | the other 13 ticks of the 18 tick flinch |
| `defeat_freeze_h0.png` | `donut_defeat_party`, 352 | 8 `h0 flinch` | the lethal blow (tick 349): the hit-stop holds `h0` (8 ticks) |
| `knockback_kb.png` | `donut_defeat_party`, 364 | 10 `kb thrown` | thrown away from the goblin, held while airborne, anchored by the pivot at the launch and by the lowest row from the apex (Carl's blend) |
| `knockdown_d0.png` | `donut_defeat_party`, 375 | 11 `d0 sploot impact` | the first 6 ticks on the floor |
| `knockdown_d1.png` | `donut_defeat_party`, 390 | 12 `d1 sploot sulk` | held until the retry (tick 398) |

In `defeat_freeze_h0.png`, `knockdown_d0.png` and `knockdown_d1.png` Carl stands over her: that is the fixture's geometry (she is a stationary target at his column), and she is drawn behind him as every other Donut frame is.

[`capture_lines.txt`](capture_lines.txt) has one `DONUT_HURT_STILL` line per still: the window's final `COMBAT_STATE` hash, the headless run's (equal on all six: the presentation changes no sim hash), whether the two passes were byte-identical, and the PNG's sha256. [`SHA256SUMS`](SHA256SUMS) is verified by `python3 scripts/check_docs.py`.

## What changed on screen

Before: a white flash for the first 3 ticks of a flinch and a 50% alpha blink through the 55 tick invulnerability, on her locomotion frame. Now: the reaction frames above. **The flash and the blink are gone** for both the flinch and the invulnerability: Carl's presentation layers neither on his hurt and knockdown frames (his sprite is never tinted), and this change matches Carl. If the owner wants an invulnerability cue for either hero, it is one shared presentation change.

## What the art does not cover

- **Getup (frames 13 and 14):** not wired. No simulation state gets a hero up: Donut's defeat ends the attempt and the retry restarts it, exactly as for Carl (whose getup is not wired either).
- **A non-lethal launch** (a boss or rack blow with `knock_vy`): the sim lifts her but her state is a flinch, so `h0` then `h1` play while she is in the air, as for Carl. The `kb` frame is drawn only for the defeat, as for Carl's.
- Her contact shadow keeps the idle footprint while she lies (Carl's follows his sprawl); not changed here.

## Sim unchanged

Presentation only: `CombatState::donut_defeated()` is a read-only accessor. Every unit test, golden and pin passes unchanged; `scripts/ci_floor01_combat.sh headless` replays every committed party and combat fixture against its committed text; no `docs/evidence/*/SHA256SUMS` file other than this folder's changed. New tests: `donut_art::tests` (frame table, manifest pin, anchor blend) and `combat_anim::tests` (the two fixtures' frame runs, the animated and plain hashes equal on every tick).

Regenerate: `cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh && bash scripts/make_donut_hurt_evidence.sh`.

## Rights and provenance

Integration only: the frames are the existing committed ANIM-DONUT-02 candidate (fan-concept art under the owner-reported scope `ART-PUBLICATION-ALL-V1`, see `docs/04-production/rights-and-release.md`; not owner-approved). **No new art, no sprite was drawn, resampled or altered.** Integration and evidence: Claude Sonnet (medium tier, Bevy integration of existing art); independent review pending. Nothing here is a claim of approved art, a Steam build, or a playable-game distribution right.
