# LLAMA-DEATH-WIRE: the Bad Llama's decapitation and explosion in Bevy

Real captures of the two llama finishes that were art without wiring (`llama_decapitated`, `llama_exploded`, which mapped to `Finish::NoArt`), now played from the existing [LLAMA-02-DEATH](../../02-art/llama-02-death-spec.md) poses and parts by `src/death_present.rs`. Written by `python3 scripts/make_llama_death_wire_evidence.py` (debug binary, exported atlases, `xvfb-run`, Pillow): the real `floor01_scene` render loop under Xvfb, lighting on, one frame per tick, at gore preset 4 (the default) and preset 1, at 640x360 (1x, gameplay scale) and 1920x1080 (3x). The README is hand-written.

**What this is not.** Not a game-feel review, not a frame-time or memory measurement and not a Steam Deck claim. No art changed: the poses and parts are the LLAMA-02-DEATH set (independent top-tier review of that set: see its spec), drawn as they are. The owner has not played it.

## Scenarios and hashes

Every capture's final `COMBAT_STATE` line equals the headless replay's, at both presets and both resolutions ([`capture_lines.txt`](capture_lines.txt)): the presentation never reaches the simulation.

| Name | Run | Event | What it shows |
| --- | --- | --- | --- |
| `blast` | `--party-fixture carl_prog_dyn_llama_core` (new QA replay, `PARTY_STATE` `a00ca20c6dcfece8`) | X = 195: Carl's thrown dynamite, landed under a passive llama, kills it point blank (`cause=explosion ... point_blank`, a real simulation kill, CP-Q26) | the explosive breakup: at preset 4 all 11 body parts, 10 gibs and 2 ribs, a tooth, 2 wool tufts, 28 blood and 8 lava drops, pools under the two torso halves, both wall spatters, the LLAMA-01 sprite and its shadow hidden; at preset 1 the LLAMA-01 `down` stays with the tufts, the tooth and the lava |
| `decap` | `--fixture cp05_llama_crush --death-lab-edge` | L = 139: the shore block lands on the llama | the decapitation pair: `decap_contact` (4 ticks), the `headless_collapse` buckle (6 ticks) and hold, the head thrown muzzle up and swapped to `head_dead` on the floor, the jet, the duct lava, the trickle and the pool stages |

**The decapitation is captured through an evidence lab.** No simulation cause shears a llama today: the block-edge test (`ShoreBlockEdge`) needs neck probes, which only the goblin has, so a llama under the block is crushed (`cp05_llama_crush`, committed replay unchanged). `--death-lab-edge` is a presentation-only flag: the cosmetic presenter reads every block kill as a block-edge cut by the same player; the simulation, its event log and its hash are untouched (the window hash equals the headless hash without the flag). So the llama stands under the stone's centre, not at its edge; judge the poses, the head and the gore, not the placement. A real shear cause for the llama is a separate gameplay item.

The HUD's system notice reads `FLATTENED!` in both runs: that is the HUD's existing kill notice (its wording is not part of this PR).

## Files

* `still_<name>_p<preset>_<tick>_640x360.png`: native 1x frames at preset 4 and preset 1 (decap L-2, L+0, L+4, L+10, L+40, L+120; blast X-5, X+0, X+3, X+12, X+40, X+120).
* `still_<name>_p4_<tick>_1920x1080.png`: 3x frames at three key ticks, preset 4.
* `sheet_<name>_p4_vs_p1.png`: 1920x1080 crops at the same ticks, preset 4 left and preset 1 right (half size).
* `gif_<name>_p4_crop_960x540.gif`: every tick from 10 before the event (Pillow merges identical held frames), a 960x540 crop of the 3x frames (gameplay scale, 60 ticks per second as 20/20/10 ms).
* [`capture_lines.txt`](capture_lines.txt): the headless and window `COMBAT_STATE` lines; `SHA256SUMS`.

## Byte identity (local checks on the PR head)

* `scripts/ci_floor01_combat.sh headless`: every committed fixture, party, boss and room replay is unchanged (the new `carl_prog_dyn_llama_core` replay is added, nothing else moves).
* The `DEATH_PRESENT` summary (presenter digest, piece and decal counts) of all 20 legacy fixtures at presets 1 to 4 (80 runs, goblin and llama finishes, the goblin decapitation and explosion included) is byte-identical to `main`'s: the goblin scripts and tables are unchanged.

## Known limits

* Timings and counts are the LLAMA-02 spec's (section 8 and `spawner_hints`); where the spec gave no number the PR body records an adopted default (LDW-AD1 to LDW-AD6).
* The decapitation pose was drawn for a blade; the only block-edge cut seen here is the lab's.
* The molten crust swap after 240 ticks (`swap_on_ttl`), llama sound and a llama shear cause are not wired.
* Rights and provenance: the art is the existing LLAMA-01 and LLAMA-02-DEATH sets under the recorded `ART-PUBLICATION-ALL-V1` scope; this PR adds no art. Wiring, tests and captures: Claude Opus (implementer; the owner directed Opus for this session); independent top-tier review passed with fixes.
