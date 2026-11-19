# CAMERA-LAB evidence

Real Bevy captures, videos and measurements of the Floor 01 Camera Scenarios Lab probe ([doc and recommendation](../../03-tech/camera-lab.md), [review](../../../tools/camera-lab/REVIEW.md)). Probe only: not merged, not an approved camera, **no human game-feel verdict**.

Everything shows existing approved art only (Carl 52 px and Donut 40 px production sprites, the approved goblin, llama, engineer, scatterer, hoarder, troglodyte, juicer and Ball of Swine readability sets, and the Floor 01 environment kit pieces recomposed into six greybox rooms). No art was invented. The images are published under the same owner-reported `ART-PUBLICATION-ALL-V1` scope as the source art and are not a licence to distribute a game.

Provenance: frames come from `src/bin/camera_lab.rs` at the commit that carries this directory, rendered by Bevy 0.20.1 with the A1 lighting pass on, scripted 2P runs (the bot in `src/camera_lab/script.rs`), the Vulkan adapter recorded in [data/adapter.txt](data/adapter.txt). Models: the probe, the layout composition, the evidence and the documents were written by Claude Sonnet 5.5. This is a probe PR, not an art PR: nothing here is new art, so the AGENTS.md art pipeline (design, execution and independent art review) does not apply; the independent review of the code is separate.

## Files

| Path | What |
|---|---|
| `sheets/roomNN_*.jpg` | One contact sheet per room, 1920x1080 outputs scaled to 480x270 cells: top row solo scripted action, bottom row 2P separated, columns 1.00x / 1.25x / 1.50x / 1.75x. |
| `frames/room01_solo_1.0x_1920x1080.png`, `room01_solo_1.5x_1920x1080.png` | Full-resolution goblin brawl at 1.0x and 1.5x. |
| `frames/room04_solo_1.5x_1920x1080.png` | Hoarder lair, tiny scatterers, 1.5x. |
| `frames/room06_2p_separated_1.5x_1920x1080.png`, `room06_2p_separated_auto_1920x1080.png` | Ball of Swine, duo 680 px apart (736 px leash), fixed 1.5x and the automatic camera; the boss beside Donut is cropped by a few pixels at the cap. |
| `frames/room05_2p_separated_1.0x_1280x800_deck.png`, `..._1.5x_1280x800_deck.png` | Steam Deck sized 16:10 output, gym, duo 400 px apart (the Deck leash is 416 px). |
| `frames/room03_solo_1.5x_3840x2160_centre_crop.png` | A 4K frame (the file is the centre 1920x1080 of a 3840x2160 capture, so a 4K pixel is 1 pixel here). |
| `crops/spawn_crops_1080p.png` | Carl and Donut at 1080p, every coverage, enlarged 3x nearest: the texel width at 1.0x and 1.5x is exact, at 1.25x and 1.75x some columns are one pixel wider. |
| `videos/llama_tell_room02_auto.mp4`, `llama_tell_room02_1.0.mp4` | A llama tells and spits across the lane at the automatic camera and at fixed 1.0x (the tell is only partly on screen at 1.0x). |
| `videos/combat_hit_room01_1.0.mp4`, `..._1.5.mp4` | A goblin kill (hit-stop, knockback, launch) at 1.0x and 1.5x, 60 fps, scaled to 1280 wide. |
| `videos/destruction_room03_1.0.mp4`, `destruction_room03_1.5.mp4` | Cause and effect: Carl breaks the support, the weight falls, crushes a goblin and lands. The automatic camera stays at 1.0x in this scene, so its capture is the 1.0x one. |
| `videos/separation_room05_auto.mp4`, `separation_room05_1.0.mp4` | 2P separation and regroup with the **736 px wide-leash alternative** (`--leash wide`; the default leash is 416 px), one frame every 6 ticks (10 fps): the automatic camera widens and returns, fixed 1.0x loses the duo. |
| `videos/boss_charge_room06_auto.mp4`, `boss_charge_room06_1.5.mp4` | The swine boss tell, charge and gate stun. |
| `zoom_transition_trace_room06.png`, `data/view_trace_room06_auto_1080p_{before,after}.txt`, `videos/zoom_room06_auto_before_stepped.mp4`, `videos/zoom_room06_auto_after_eased.mp4` | The owner's "zoom transition too rough" report (ROOM-06, 2P, automatic camera, 1080p): the per-tick view trace (width, centre, floor anchor, pan velocity, urgent flag, hit-stop) before and after the fix, a plot of both, and the two videos (ticks 20 to 238, 30 fps). The before trace and video are the stepped camera at the previous commit. |
| `framing_metrics.md` | The headless framing metrics of the six scripted runs (fixed coverages and automatic, 16:9 and 16:10) and the automatic camera's width histogram per room and output (second revision). |
| `perf_frame_times.md`, `data/perf_raw.txt`, `data/adapter.txt` | Frame time and memory per room x coverage on the real GPU at 1920x1080, 3840x2160 and 1280x800 (and the dev profile), the raw `LAB_PERF` lines and the adapter. **Measured before the camera rework of the second revision and not re-measured** (the render path did not change). |
| `SHA256SUMS` | Checksums of every file here. |

## Not committed (bulk, regenerate)

The full matrix of 180 real screenshots (6 rooms x fixed 1.0 / 1.25 / 1.5 / 1.75 and automatic x 1920x1080, 3840x2160, 1280x800 x solo scripted action and 2P separated, about 150 MB) was captured and checked but kept out of the repository, with the frame sequences behind the videos and the xvfb frame-time logs. Regenerate with:

```bash
cargo build --locked --bin camera_lab && bash scripts/export_assets.sh && bash scripts/camera_lab_assets.sh
bash scripts/camera_lab_evidence.sh shots  artifacts/camera-lab/shots     # 180 PNG
bash scripts/camera_lab_evidence.sh videos artifacts/camera-lab/videos    # frames and mp4
bash scripts/camera_lab_evidence.sh crops  artifacts/camera-lab/crops
NO_XVFB=1 BIN=./target/release/camera_lab bash scripts/camera_lab_evidence.sh perf artifacts/camera-lab/perf   # real display
bash scripts/camera_lab_curate.sh artifacts/camera-lab/shots artifacts/camera-lab/videos artifacts/camera-lab/crops docs/evidence/camera-lab
./target/debug/camera_lab --metrics > docs/evidence/camera-lab/framing_metrics.md   # then re-add the heading
```

Capture tick choices (solo scripted action per room: a hit, a spit, a crush, a kill, a charge) and the separated positions are in `scripts/camera_lab_evidence.sh`.

## Not claimed

No Steam Deck run, no DualSense run, no handheld legibility check and no human review exist. Frame times are one machine on a desktop GPU. Passing CI is not evidence that any camera setting feels right.
