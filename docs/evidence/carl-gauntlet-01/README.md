# CARL-GAUNTLET-01 evidence: Carl's reinforced fist

Evidence for [the CARL-GAUNTLET-01 spec](../../02-art/carl-gauntlet-01-spec.md). **Status:** the 98 overlay frames, their sources, manifests, validators, the Bevy presenter and the captures below are done; **owner art review pending**, independent top-tier review pending; art is `prototype_fixture_unapproved`. Game feel is not judged here.

## Design phase (keys and silhouette gate)

| File | Content |
| --- | --- |
| `keys_bare_vs_gauntlet_x8.png` | Three pairs at x8 on a flat `#342c46` background, bare then gauntlet: `idle_hand` (locomotion 0), `punch_b_windup` (melee 4), `punch_b_contact` (melee 5) |
| `keys_hand_zoom_x20.png` | The same three pairs cropped to a 16 x 14 window around the hand, x20 |
| `gate_floor1_640x360_x1.png`, `gate_floor1_1920x1080_x3.png` | Lit Floor 1 composition at gameplay scale: bare idle, gauntlet idle, gauntlet windup, gauntlet contact, bare contact, gauntlet idle mirrored (the runtime `flip_x`) |

The silhouette gate passed with notes (spec section 10); the notes are folded into the keys.

## Execution phase (this PR)

Composed from the committed grids by `scripts/make_carl_gauntlet_evidence.py` (deterministic; nothing here changes a source). The captures come from `scripts/make_carl_gauntlet_evidence.sh` (real Bevy window under xvfb through `scripts/room_capture.sh`).

| File | Content |
| --- | --- |
| `contact_<sheet>_gauntlet_x3.png` | The overlay composed on the body, every frame of `locomotion` (16), `melee` (24), `cf02` (13) and `dynamite` (45) at x3, captioned with frame index and tag |
| `contact_<sheet>_overlay_only_x3.png` | The same frames, the overlay alone |
| `gif_idle_gauntlet_x3.gif`, `gif_run_gauntlet_x3.gif`, `gif_punch_a_gauntlet_x3.gif`, `gif_punch_b_gauntlet_x3.gif` | Timing GIFs at the body sheet's frame durations (punches rest on their last frame) |
| `diagram_frame_layout.png` | Owner handoff: the overlay cell is the body cell (48 x 56 grid, pivot (25, 55), same index and flip), shown on melee frames 4, 5 and 0 |
| `diagram_state_flow.png` | Owner handoff: the capability gate and the per-frame copy (one gate, read-only, nothing hashed) |
| `gif_room6_haymaker_3x.gif` | Owner handoff: the in-room GIF, room 6 (`--scene f01_late --f01-room mastery --room-pilot mastery:carl_solo:2`), ticks 498 to 542, 2 ticks a frame, scale 3 cropped to the fight; Carl's haymaker starts on tick 510 |
| `room6_haymaker_bare_vs_gauntlet_x12.png` | The same tick (516) with the overlay atlases removed (Carl bare) and with them (gauntlet), cropped to the changed pixels and enlarged x4 of the 3x capture: the only pixels that differ are the fist |
| `room5_pre_grant_<res>.png`, `room5_post_grant_<res>.png`, `room5_post_grant_exit_<res>.png` | Room 5 (Juicer pilot `juicer`): tick 3165 before the grant (Carl bare), ticks 3200 and 3230 after the Juicer's defeat and the grant of `fist_reinforced` on tick 3175 |
| `room6_haymaker_<res>.png`, `room6_run_<res>.png` | Room 6 at tick 516 (haymaker contact) and tick 150 (running): the room's pin holds every capability, so the overlay is on from tick 0 |
| `capture_lines.txt` | One `ROOM_CAPTURE` line per capture: the window run's final `COMBAT_STATE` hash equals the headless run's (`match=yes` on all 22), also with the overlay atlases removed |
| `atlas/` | The four exported overlay atlases (PNG + metadata JSON) as `bash scripts/export_assets.sh` writes them; same size and cell rectangles as the body atlases |
| `SHA256SUMS` | SHA-256 of every file here except this README and itself |

`<res>` is `640x360` (scale 1), `1920x1080` (scale 3), `640x400` (`--deck`, scale 1) and `1920x1200` (`--deck`, scale 3).

**Room 5 and 6 loadouts (as run):** room 5 uses the story grant (no flag): the gauntlet is bare at tick 3165 and drawn from the tick after the defeat. Room 6 needs no flag (`f01_late_mastery` holds `fist_reinforced`). `--f01-loadout all` was not needed.

**Checks run on the final head** (see the PR body for exact results): validators V01 to V04, V15, V18 on the four configs (CG-L1 to CG-L9 on 98 frames, 0 findings, 0 empty frames, 10 listed CG-L6 exceptions); body atlases and `.ase` sources byte-identical before and after `bash scripts/export_assets.sh`; the four overlay atlases byte-identical across two exports; Rust tests for hidden-without-capability, shown-with-it, index and flip copied for each of the four body sprites, and equal simulation hashes with the overlay system present and absent.

**Provenance:** the gauntlet pixels are hand-placed (three Opus keys) or stamped by `scripts/author_carl_gauntlet.py` (95 frames, Sonnet) on the approved ANIM-CARL-02 body frames, which are not changed; no concept image, third-party design or image-model raster output is used, copied, traced or resampled. Published under the owner-reported scope `ART-PUBLICATION-ALL-V1` ([rights register](../../04-production/rights-and-release.md)); no public reuse licence and no distribution rights for a game or asset pack.
