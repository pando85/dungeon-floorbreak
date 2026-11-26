# HRD-CUES-01: the Hoarder's stir and the J-B post's "not yet" at runtime

Window evidence for [the HRD-CUES-01 spec](../../02-art/hrd-cues-01-spec.md): the pre-wake stir of the lair mound (HRD-Q05: S1 from a lead hero x of 240, S2 after the engage crossing at x 432, with HC-AD11's hold) and the J-B post's cue (HRDC-AD4: `shored` while she is not out, then the 8-tick `give` beat). Both are **presentation only**: no simulation field, no pin and no replay moves. The art and the cue are not owner-approved; the independent Opus review is pending, owner art review and game-feel review are pending.

**How the stills were captured.** The cues' ages are render-local (`HoarderMemo` anchors a level on the first tick the presenter sees it), so a single-tick capture shows every age at 0. Each cue is therefore a **one-tick-per-frame sequence that starts just before the cue is active**, made by `scripts/make_hrd_cues_evidence.sh` with `scripts/room_capture.sh`, and the stills are picked from it by tick. The pilot is `--f01-room hoarder --room-pilot hoarder:carl_solo:0` (the `direct` plan: seed 0 reveals her on ticks 116..205); the floor run is the same plan with `--scene f01_late --f01-run` (the floor run's timeline is the same). Every sequence is captured twice per plan at 640x360 (`--scale 1`) and 1920x1080 (`--scale 3`): the two passes are byte-identical frame by frame, the sequence's final window hash equals the headless hash, and each kept still's own tick is also checked (the window's `COMBAT_STATE` hash of that tick equals a headless run to that tick).

| Sequence | Ticks | Why this range |
| --- | --- | --- |
| `s1` | 41..70 | Carl is at x 238.4 on tick 41 and x 241.5 on tick 42: S1 age 0 is tick 42 |
| `s2` | 107..116 | Carl crosses x 432 on tick 108 (S2 age 0); the Reveal starts on 116, so solo S2 lasts exactly 8 ticks (108..115) |
| `give` | 205..218 | tick 205 is the last Reveal tick, she is out on 206 (give out age 0..7 = ticks 206..213), the machine's frame from 214 |
| `shored` | 159 | a Reveal tick (the shored post has no age) |

## Stills

Expected phase, frame and age of each chosen still (`floor_*` is the floor run of the same tick). The S1 breath is 72 ticks (frame 3 for phase 0..5, frame 5 for 6..9, frame 0 after), the S2 breath 36 (frame 4 for 0..7, 5 for 8..11); stir motes start at phase 2 + 3k.

| Still | Tick | Expected |
| --- | --- | --- |
| `stir_s1_heave`, `floor_stir_s1_heave` | 42 | S1 age 0: mound frame 3 (`stir_heave`), no motes yet |
| `stir_s1_slump` | 48 | S1 age 6: frame 5 (`stir_slump`), one mote |
| `stir_s1_motes` | 54 | S1 age 12: resting frame 0, three motes sliding down and out |
| `stir_s1_rest` | 66 | S1 age 24: frame 0, two motes left |
| `stir_s2_heave`, `floor_stir_s2_heave` | 111 | S2 age 3: frame 4 (`stir_heave_hi`), one mote |
| `stir_s2_late` | 115 | S2 age 7, the last Dormant tick: frame 4, two motes |
| `shored`, `floor_shored` | 159 | a Reveal tick: post frame 3 (`shored`), Reveal dust |
| `shored_last` | 205 | the last Reveal tick: post still `shored` |
| `give_0`, `floor_give_0` | 206 | out age 0: post frame 4 (`give`), the R7 burst (12 motes) plus the 4 give motes |
| `give_2` | 208 | out age 2: `give`, burst and give motes |
| `give_7` | 213 | out age 7, the last give tick: `give`, 4 give motes rising |
| `intact_after`, `floor_intact_after` | 216 | out age 10: the machine's `intact` frame, no motes |

Strips and GIFs (x3 crops of the 1920x1080 frames, nearest-neighbour only; the window already renders at 3x; the GIFs play every tick of the sequence at 100 ms, 6x slower than the game):

| File | Content |
| --- | --- |
| `strip_stir_s1.png`, `stir_s1.gif` | one S1 breath on the visible part of the mound at the right screen edge: ticks 42, 45, 48, 51, 54, 60, 66 |
| `strip_stir_s2.png`, `stir_s2.gif` | the solo S2 beat: ticks 107 (rest), 108, 110, 112, 114, 115 (the crown held up by `stir_heave_hi`) |
| `strip_give.png`, `give.gif` | the post: ticks 205 (shored), 206, 208, 210, 213 (give), 214, 216 (intact) |

Readability: the stir reads in motion (the strips and GIFs); one still shows it only against the rest frame. The shored post is darker and banded; the give beat reads as the straps springing, and the straps vanish when it ends on tick 214 (an owner note, gate note N2).

[`capture_lines.txt`](capture_lines.txt) has one `HRDCUE_SEQUENCE` line per sequence (plan, size) and one `HRDCUE_STILL` line per still and size, pasted below. [`SHA256SUMS`](SHA256SUMS) is verified by `python3 scripts/check_docs.py`. Regenerate with `bash scripts/make_hrd_cues_evidence.sh` (needs the debug `floor01_scene` binary, the exported atlases, xvfb-run, ImageMagick and Pillow).

```text
HRDCUE_SEQUENCE plan=direct size=640x360 seq=s1 first_tick=41 count=30 passes_byte_identical=yes window_hash=e43fe2218019024a headless_hash=e43fe2218019024a match=yes
HRDCUE_SEQUENCE plan=direct size=640x360 seq=s2 first_tick=107 count=10 passes_byte_identical=yes window_hash=737046a087f4933e headless_hash=737046a087f4933e match=yes
HRDCUE_SEQUENCE plan=direct size=640x360 seq=give first_tick=205 count=14 passes_byte_identical=yes window_hash=cdc87eb93d17240a headless_hash=cdc87eb93d17240a match=yes
HRDCUE_SEQUENCE plan=direct size=640x360 seq=shored first_tick=159 count=1 passes_byte_identical=yes window_hash=0e6a3cf0a260dc80 headless_hash=0e6a3cf0a260dc80 match=yes
HRDCUE_SEQUENCE plan=floor size=640x360 seq=s1 first_tick=41 count=30 passes_byte_identical=yes window_hash=e43fe2218019024a headless_hash=e43fe2218019024a match=yes
HRDCUE_SEQUENCE plan=floor size=640x360 seq=s2 first_tick=107 count=10 passes_byte_identical=yes window_hash=737046a087f4933e headless_hash=737046a087f4933e match=yes
HRDCUE_SEQUENCE plan=floor size=640x360 seq=give first_tick=205 count=14 passes_byte_identical=yes window_hash=cdc87eb93d17240a headless_hash=cdc87eb93d17240a match=yes
HRDCUE_SEQUENCE plan=floor size=640x360 seq=shored first_tick=159 count=1 passes_byte_identical=yes window_hash=0e6a3cf0a260dc80 headless_hash=0e6a3cf0a260dc80 match=yes
HRDCUE_SEQUENCE plan=direct size=1920x1080 seq=s1 first_tick=41 count=30 passes_byte_identical=yes window_hash=e43fe2218019024a headless_hash=e43fe2218019024a match=yes
HRDCUE_SEQUENCE plan=direct size=1920x1080 seq=s2 first_tick=107 count=10 passes_byte_identical=yes window_hash=737046a087f4933e headless_hash=737046a087f4933e match=yes
HRDCUE_SEQUENCE plan=direct size=1920x1080 seq=give first_tick=205 count=14 passes_byte_identical=yes window_hash=cdc87eb93d17240a headless_hash=cdc87eb93d17240a match=yes
HRDCUE_SEQUENCE plan=direct size=1920x1080 seq=shored first_tick=159 count=1 passes_byte_identical=yes window_hash=0e6a3cf0a260dc80 headless_hash=0e6a3cf0a260dc80 match=yes
HRDCUE_SEQUENCE plan=floor size=1920x1080 seq=s1 first_tick=41 count=30 passes_byte_identical=yes window_hash=e43fe2218019024a headless_hash=e43fe2218019024a match=yes
HRDCUE_SEQUENCE plan=floor size=1920x1080 seq=s2 first_tick=107 count=10 passes_byte_identical=yes window_hash=737046a087f4933e headless_hash=737046a087f4933e match=yes
HRDCUE_SEQUENCE plan=floor size=1920x1080 seq=give first_tick=205 count=14 passes_byte_identical=yes window_hash=cdc87eb93d17240a headless_hash=cdc87eb93d17240a match=yes
HRDCUE_SEQUENCE plan=floor size=1920x1080 seq=shored first_tick=159 count=1 passes_byte_identical=yes window_hash=0e6a3cf0a260dc80 headless_hash=0e6a3cf0a260dc80 match=yes
HRDCUE_STILL name=stir_s1_heave plan=direct tick=42 size=640x360 window_hash=d0cc784121712d92 headless_hash=d0cc784121712d92 hash_match=yes passes_byte_identical=yes sha256=8bd24b3e0c87ad9cd46659183a089af7447e4405f3960b6ad12d02a68e724a37 note="S1 age 0: heave frame 3, no motes yet"
HRDCUE_STILL name=stir_s1_heave plan=direct tick=42 size=1920x1080 window_hash=d0cc784121712d92 headless_hash=d0cc784121712d92 hash_match=yes passes_byte_identical=yes sha256=121ffd727bfba3ecc69e4fe0e5eb4ee537fdd713c879cf22c445bfe732cd0057 note="S1 age 0: heave frame 3, no motes yet"
HRDCUE_STILL name=floor_stir_s1_heave plan=floor tick=42 size=640x360 window_hash=d0cc784121712d92 headless_hash=d0cc784121712d92 hash_match=yes passes_byte_identical=yes sha256=8bd24b3e0c87ad9cd46659183a089af7447e4405f3960b6ad12d02a68e724a37 note="S1 age 0: heave frame 3, no motes yet"
HRDCUE_STILL name=floor_stir_s1_heave plan=floor tick=42 size=1920x1080 window_hash=d0cc784121712d92 headless_hash=d0cc784121712d92 hash_match=yes passes_byte_identical=yes sha256=121ffd727bfba3ecc69e4fe0e5eb4ee537fdd713c879cf22c445bfe732cd0057 note="S1 age 0: heave frame 3, no motes yet"
HRDCUE_STILL name=stir_s1_slump plan=direct tick=48 size=640x360 window_hash=2e57cd7e7ecfd5e0 headless_hash=2e57cd7e7ecfd5e0 hash_match=yes passes_byte_identical=yes sha256=9aec43a242cd6b951c542a74e45fc5955cb69385fdbc718b73731f972f87761e note="S1 age 6: slump frame 5, first mote"
HRDCUE_STILL name=stir_s1_slump plan=direct tick=48 size=1920x1080 window_hash=2e57cd7e7ecfd5e0 headless_hash=2e57cd7e7ecfd5e0 hash_match=yes passes_byte_identical=yes sha256=0aaf85aa5f4c672d7ff73d904c8a5f732b38553b297de0b154f25f861f6cc99c note="S1 age 6: slump frame 5, first mote"
HRDCUE_STILL name=stir_s1_motes plan=direct tick=54 size=640x360 window_hash=39e96100cbddfe82 headless_hash=39e96100cbddfe82 hash_match=yes passes_byte_identical=yes sha256=def7d322b9c20313caf398dbb6cf4bb5bf12f535734f3f60f59f5d3e809751dd note="S1 age 12: resting frame 0, three motes sliding"
HRDCUE_STILL name=stir_s1_motes plan=direct tick=54 size=1920x1080 window_hash=39e96100cbddfe82 headless_hash=39e96100cbddfe82 hash_match=yes passes_byte_identical=yes sha256=c640dcb016974ac6074c3c130dd2cfdf5bd5b735c5339972019dd6f4196b87c9 note="S1 age 12: resting frame 0, three motes sliding"
HRDCUE_STILL name=stir_s1_rest plan=direct tick=66 size=640x360 window_hash=ea63cad659809353 headless_hash=ea63cad659809353 hash_match=yes passes_byte_identical=yes sha256=47344a572814eba5f127bd4b3eaecc86a15f71ebe924ed21adb22cca555d3854 note="S1 age 24: resting frame 0, two motes left"
HRDCUE_STILL name=stir_s1_rest plan=direct tick=66 size=1920x1080 window_hash=ea63cad659809353 headless_hash=ea63cad659809353 hash_match=yes passes_byte_identical=yes sha256=8d24b34bb32df2899707d834a32c96d1fb5c2ea2a98d73abffcece091e06ecfc note="S1 age 24: resting frame 0, two motes left"
HRDCUE_STILL name=stir_s2_heave plan=direct tick=111 size=640x360 window_hash=7067ec5dbcbe6fe3 headless_hash=7067ec5dbcbe6fe3 hash_match=yes passes_byte_identical=yes sha256=04e245f81b5096c13fa76565e41ca0171cd5c77f7067ad5e5a47abd206a70352 note="S2 age 3: heave_hi frame 4, one mote"
HRDCUE_STILL name=stir_s2_heave plan=direct tick=111 size=1920x1080 window_hash=7067ec5dbcbe6fe3 headless_hash=7067ec5dbcbe6fe3 hash_match=yes passes_byte_identical=yes sha256=0ad614430708a86f2a5cd873b05d2d98171380985f8b54505c964649dc60cf8d note="S2 age 3: heave_hi frame 4, one mote"
HRDCUE_STILL name=floor_stir_s2_heave plan=floor tick=111 size=640x360 window_hash=7067ec5dbcbe6fe3 headless_hash=7067ec5dbcbe6fe3 hash_match=yes passes_byte_identical=yes sha256=04e245f81b5096c13fa76565e41ca0171cd5c77f7067ad5e5a47abd206a70352 note="S2 age 3: heave_hi frame 4, one mote"
HRDCUE_STILL name=floor_stir_s2_heave plan=floor tick=111 size=1920x1080 window_hash=7067ec5dbcbe6fe3 headless_hash=7067ec5dbcbe6fe3 hash_match=yes passes_byte_identical=yes sha256=0ad614430708a86f2a5cd873b05d2d98171380985f8b54505c964649dc60cf8d note="S2 age 3: heave_hi frame 4, one mote"
HRDCUE_STILL name=stir_s2_late plan=direct tick=115 size=640x360 window_hash=f075edaadeedac16 headless_hash=f075edaadeedac16 hash_match=yes passes_byte_identical=yes sha256=f8c6f5db3052903accbb9d2b3ddf993512e73d49eb8598c8d499084027947ee0 note="S2 age 7 (last Dormant tick): heave_hi frame 4, two motes"
HRDCUE_STILL name=stir_s2_late plan=direct tick=115 size=1920x1080 window_hash=f075edaadeedac16 headless_hash=f075edaadeedac16 hash_match=yes passes_byte_identical=yes sha256=130780c3ce1e10aff1af83134c5b6f46083c14ddc24974e0a20047123543a5fd note="S2 age 7 (last Dormant tick): heave_hi frame 4, two motes"
HRDCUE_STILL name=shored plan=direct tick=159 size=640x360 window_hash=0e6a3cf0a260dc80 headless_hash=0e6a3cf0a260dc80 hash_match=yes passes_byte_identical=yes sha256=06982786c98edf60e38b38e5ff73628d5d94dee30f0b2744d941cebdbdf2e081 note="a Reveal tick: post frame 3 (shored), no age"
HRDCUE_STILL name=shored plan=direct tick=159 size=1920x1080 window_hash=0e6a3cf0a260dc80 headless_hash=0e6a3cf0a260dc80 hash_match=yes passes_byte_identical=yes sha256=9860fbe717319319f9af45a46f56319acf9b4ea3225cfacf1506a6fa7cb6ed9a note="a Reveal tick: post frame 3 (shored), no age"
HRDCUE_STILL name=floor_shored plan=floor tick=159 size=640x360 window_hash=0e6a3cf0a260dc80 headless_hash=0e6a3cf0a260dc80 hash_match=yes passes_byte_identical=yes sha256=06982786c98edf60e38b38e5ff73628d5d94dee30f0b2744d941cebdbdf2e081 note="a Reveal tick: post frame 3 (shored), no age"
HRDCUE_STILL name=floor_shored plan=floor tick=159 size=1920x1080 window_hash=0e6a3cf0a260dc80 headless_hash=0e6a3cf0a260dc80 hash_match=yes passes_byte_identical=yes sha256=9860fbe717319319f9af45a46f56319acf9b4ea3225cfacf1506a6fa7cb6ed9a note="a Reveal tick: post frame 3 (shored), no age"
HRDCUE_STILL name=shored_last plan=direct tick=205 size=640x360 window_hash=3573cf65b4e14d0c headless_hash=3573cf65b4e14d0c hash_match=yes passes_byte_identical=yes sha256=013227f9d862a2a5d1f5ca42596f0ac9ec18ffbaec0731cbe48eea214702b9c8 note="the last Reveal tick: shored"
HRDCUE_STILL name=shored_last plan=direct tick=205 size=1920x1080 window_hash=3573cf65b4e14d0c headless_hash=3573cf65b4e14d0c hash_match=yes passes_byte_identical=yes sha256=45ec224d1b15121c6c29356813915b34cbfa3c849276e3315e9b45673c01cf82 note="the last Reveal tick: shored"
HRDCUE_STILL name=give_0 plan=direct tick=206 size=640x360 window_hash=c95c7790ddecc61e headless_hash=c95c7790ddecc61e hash_match=yes passes_byte_identical=yes sha256=22a0015690585178ab2360a9f3f7a2d8fc85872776081e97a2123a2775d467fd note="out age 0: post frame 4 (give), 4 give motes with the 12-mote burst"
HRDCUE_STILL name=give_0 plan=direct tick=206 size=1920x1080 window_hash=c95c7790ddecc61e headless_hash=c95c7790ddecc61e hash_match=yes passes_byte_identical=yes sha256=462da89061b1174b84a250ce69bd5e7e0ccdad02d7d1c615d0f4e6e70b834252 note="out age 0: post frame 4 (give), 4 give motes with the 12-mote burst"
HRDCUE_STILL name=floor_give_0 plan=floor tick=206 size=640x360 window_hash=c95c7790ddecc61e headless_hash=c95c7790ddecc61e hash_match=yes passes_byte_identical=yes sha256=22a0015690585178ab2360a9f3f7a2d8fc85872776081e97a2123a2775d467fd note="out age 0: post frame 4 (give), 4 give motes with the 12-mote burst"
HRDCUE_STILL name=floor_give_0 plan=floor tick=206 size=1920x1080 window_hash=c95c7790ddecc61e headless_hash=c95c7790ddecc61e hash_match=yes passes_byte_identical=yes sha256=462da89061b1174b84a250ce69bd5e7e0ccdad02d7d1c615d0f4e6e70b834252 note="out age 0: post frame 4 (give), 4 give motes with the 12-mote burst"
HRDCUE_STILL name=give_2 plan=direct tick=208 size=640x360 window_hash=6cb3a6ea57ed00e5 headless_hash=6cb3a6ea57ed00e5 hash_match=yes passes_byte_identical=yes sha256=3894c27395200d192cfd1d6c0b1b8793fa28623639d815ab1be3c5120d66c038 note="out age 2: give, burst and give motes"
HRDCUE_STILL name=give_2 plan=direct tick=208 size=1920x1080 window_hash=6cb3a6ea57ed00e5 headless_hash=6cb3a6ea57ed00e5 hash_match=yes passes_byte_identical=yes sha256=0e1b4a2667bbfdeada586382fbfd648a768f80adc174d0c4e942957471d6998f note="out age 2: give, burst and give motes"
HRDCUE_STILL name=give_7 plan=direct tick=213 size=640x360 window_hash=56f8cb49d9d0534f headless_hash=56f8cb49d9d0534f hash_match=yes passes_byte_identical=yes sha256=8848331c9c656ae554d69f914d0669ed0b6804625bb2ef0a82aea5230368a1bb note="out age 7: the last give tick, 4 give motes"
HRDCUE_STILL name=give_7 plan=direct tick=213 size=1920x1080 window_hash=56f8cb49d9d0534f headless_hash=56f8cb49d9d0534f hash_match=yes passes_byte_identical=yes sha256=5d13342e9416c98116933ff3b71af8f6bdc754331ce445bc1abff3e501a1ccaf note="out age 7: the last give tick, 4 give motes"
HRDCUE_STILL name=intact_after plan=direct tick=216 size=640x360 window_hash=2e9947bec9d21678 headless_hash=2e9947bec9d21678 hash_match=yes passes_byte_identical=yes sha256=0c6b0ce07dee361973af113fb11dd98c9834f387a1785a10b5877e52bb9af8a8 note="out age 10: the machine's intact frame, no motes"
HRDCUE_STILL name=intact_after plan=direct tick=216 size=1920x1080 window_hash=2e9947bec9d21678 headless_hash=2e9947bec9d21678 hash_match=yes passes_byte_identical=yes sha256=72256f3a8a982f7633d89f03f7fadbdd3715375d81dd90e9c484c6c24fe23aed note="out age 10: the machine's intact frame, no motes"
HRDCUE_STILL name=floor_intact_after plan=floor tick=216 size=640x360 window_hash=2e9947bec9d21678 headless_hash=2e9947bec9d21678 hash_match=yes passes_byte_identical=yes sha256=0c6b0ce07dee361973af113fb11dd98c9834f387a1785a10b5877e52bb9af8a8 note="out age 10: the machine's intact frame, no motes"
HRDCUE_STILL name=floor_intact_after plan=floor tick=216 size=1920x1080 window_hash=2e9947bec9d21678 headless_hash=2e9947bec9d21678 hash_match=yes passes_byte_identical=yes sha256=72256f3a8a982f7633d89f03f7fadbdd3715375d81dd90e9c484c6c24fe23aed note="out age 10: the machine's intact frame, no motes"
```

The state hash is also proven by `hoarder_art::tests::the_presenter_does_not_change_the_state_hash` (extended: the stir level and memo run on every tick and reach S1 and S2) and by the render-loop replay of the room pilot runs (`../f01-playtest/render_replay_lines.txt`).

**Not shown or not run:** the S2 slump and the S2 motes after phase 7 (solo S2 lasts 8 ticks; a 2P gate wait shows them, covered by `the_two_player_gate_wait_keeps_the_hard_stir`), a human run, the release build, real hardware.
