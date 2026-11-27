# TROG-FX-01 evidence: troglodyte death profiles and attack telegraph art

Spec: [trog-fx-01-spec.md](../../02-art/trog-fx-01-spec.md). **Owner art and game-feel review pending**; the art is `prototype_fixture_unapproved` and reuses approved pixels only (TROG-01/02/03 idle frame 0, GOB-02 gib and blood cels, the Goblin Workshop weight-shadow sheet). Presentation only: every still's window hash equals the headless `COMBAT_STATE` hash of the same tick, and the hashes of the tell stills equal the TROG-A evidence of the same ticks (`docs/evidence/trog`), so the simulation did not move. Every still was captured twice and the two passes are byte-identical (`capture_lines.txt`).

| File | What is in frame | How |
| --- | --- | --- |
| `still_pygmy_tell_640x360.png` | The Pygmy's tell: the dithered patch under its committed leap path (floor level, the path's exact x extent) | window, `--scene f01_late --f01-room trog_run --trog-pilot carl --companion off`, tick 125 |
| `still_basher_tell_640x360.png` | The Basher's tell: a 17 px dithered patch on the committed smash rectangle in front of it | tick 520 |
| `still_basher_smash_live_640x360.png` | The Basher's hitbox live: the patch is solid | tick 543 |
| `still_virtuoso_tell_640x360.png` | The Virtuoso's feint: the patch at the clipped tongue band's height | tick 870 |
| `still_basher_burst_640x360.png` | Mid-burst: the Basher thrown into its own parts (head, torso, arms, legs) with blood and a wall spatter, preset 4 | tick 621 with the evidence flag `--death-lab-trog exploded` (the pilot's kill is a jab; the flag only chooses the finish) |
| `still_virtuoso_burst_640x360.png` | The Virtuoso's death mid-burst: the hair-covered head, the torso, the arm and the gold bell | tick 947, same flag |
| `still_pygmy_down_blood_640x360.png` | The default: the Pygmy's approved `down` frame with blood and gibs after the pilot's jab, no flag | tick 219 |
| `parts_contact_sheet_x4.png` | The parts sheets at x4 (body and gore layers), one row per kind; the idle frame 0 each is cut from leads the row | `python3 scripts/author_trogfx01.py contact OUT.png` |
| `capture_lines.txt`, `SHA256SUMS` | One line per still (the hash-equality fields, `passes_byte_identical`) and the checksums | `trogfx-evidence` capture loop of the TROG-A evidence, 640x360 only |

At 640x360 the patches are small by design (they are the committed rectangles, 17 to 21 px wide for the Basher and the Virtuoso). The stills are 1x captures of the real window; the telegraph is visible at 6x crops of the floor. Nothing here claims audio, a recording, or accepted game feel.
