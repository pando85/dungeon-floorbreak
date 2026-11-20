# CF02-AUDIO structural QA (cf02 cue overlay)

**These checks cannot establish audio quality or subjective fitness.** Owner listening on real speakers stays open (COMBAT-FUN-02 design section 8, item 3). Nothing here is approved audio.

- 15 runtime Ogg Vorbis assets, five cues with 3 takes each, 44.1 kHz mono.
- Provenance: original deterministic DSP (`scripts/author_cf02_audio.py`, base seed 20261120); no recording, sample, third-party or generated-model audio, no voice.
- Result: all structural checks passed.
- Level band: peak <= 0.8, RMS 0.05 to 0.14 (the measured range of the Floor 01 impact and crash cues).
- Overlap limit (at most two major transients), the cue mapping and "no cue drives gameplay" are checked by Rust tests (`src/audio.rs`, `tests/audio_assets.rs`), not here.

| File | Duration s | Peak | RMS | Centroid Hz | Checks |
| --- | --- | --- | --- | --- | --- |
| `cf02/sfx/cf02_punch_hit_01.ogg` | 0.217 | 0.633 | 0.0626 | 3111 | pass |
| `cf02/sfx/cf02_punch_hit_02.ogg` | 0.217 | 0.493 | 0.0595 | 2947 | pass |
| `cf02/sfx/cf02_punch_hit_03.ogg` | 0.217 | 0.614 | 0.0677 | 3057 | pass |
| `cf02/sfx/cf02_stomp_01.ogg` | 0.417 | 0.675 | 0.1200 | 2480 | pass |
| `cf02/sfx/cf02_stomp_02.ogg` | 0.417 | 0.707 | 0.1368 | 2699 | pass |
| `cf02/sfx/cf02_stomp_03.ogg` | 0.417 | 0.708 | 0.1269 | 2621 | pass |
| `cf02/sfx/cf02_wall_slam_01.ogg` | 0.547 | 0.657 | 0.0888 | 5234 | pass |
| `cf02/sfx/cf02_wall_slam_02.ogg` | 0.547 | 0.667 | 0.1050 | 5089 | pass |
| `cf02/sfx/cf02_wall_slam_03.ogg` | 0.570 | 0.671 | 0.0973 | 5246 | pass |
| `cf02/sfx/cf02_blast_01.ogg` | 0.922 | 0.636 | 0.0926 | 4750 | pass |
| `cf02/sfx/cf02_blast_02.ogg` | 0.897 | 0.725 | 0.1234 | 4583 | pass |
| `cf02/sfx/cf02_blast_03.ogg` | 0.897 | 0.729 | 0.1051 | 4664 | pass |
| `cf02/sfx/cf02_defeat_heavy_01.ogg` | 0.497 | 0.650 | 0.1222 | 2277 | pass |
| `cf02/sfx/cf02_defeat_heavy_02.ogg` | 0.497 | 0.659 | 0.1153 | 2203 | pass |
| `cf02/sfx/cf02_defeat_heavy_03.ogg` | 0.497 | 0.664 | 0.1225 | 2312 | pass |

## Not tested

- How the cues sound, whether each cause is distinguishable by ear, fatigue under repetition.
- In-engine loudness against the music bed and the other effects, headphones, TVs, other devices.
- Human-expert or end-user approval.
