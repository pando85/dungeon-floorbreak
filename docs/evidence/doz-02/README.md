# DOZ-02 evidence: Dozer damage states and the stop frame, design keys

These are review renders for the [DOZ-02 design spec](../../02-art/doz-02-dozer-damage-spec.md). They are composed PNGs, not Bevy captures. The lit scenes use the A1 compositor (`scripts/env_kit_light.py`) on the Floor 1 kit. Nothing here is wired into a binary.

Status:

- **Design keys:** drawn (spec section 6). Routing score 4 of 4, Hard (spec section 1).
- **Silhouette gate:** **PASS with notes** (an independent Claude Opus agent who did not draw the keys; verdict below and in spec section 11). Execution may start; the notes are carried into it.
- **Execution:** pending (spec section 7): the remaining frames, the export, the wiring.
- **Review:** pending, independent.

## Silhouette gate

Gate pack: `doz02_lit_before_640x360.png` and `doz02_lit_after_640x360.png` (judge at 1x first), `doz02_key_sheet_x4.png`, and the spec.

| Question | Verdict |
| --- | --- |
| 1. Does the disabled machine read as the DOZ-01 Dozer, broken and harmless? | **PASS.** The silhouette below the stack is the DOZ-01 machine; at 1x the knocked stack, the steam puff, the dark yoke lamp and the barred wheel say "broken". The boiler crack, the cab dent and the 3 px tub crumple are 1 px detail that reads at 2x to 4x but only faintly at 1x (note G-1). |
| 2. Does the jammed wheel read as stopped and not lethal? | **PASS.** The clearest reversal of the set: the spike ring drops from the brightest object on screen to a dull slate star that sinks into the arch shadow, and the rusty tie bar through the spokes explains the stop in one glance. The three missing spikes register at 1x only at the top (note G-2). |
| 3. Does the remnant read as a fallen press frame? | **PASS.** Same rust beam, rivet row and end stencils as the raised key, now bowed and lying in the lane; it reads as the thing that fell, not as a step or a platform. The upward bow gives a slight speed-bump hump at 1x (note G-3). |

Any "no" sends the keys back to design before execution starts.

**Overall: PASS with notes.** Reviewer: an independent Claude Opus agent (did not draw the keys), judged at 1x first, then at 4x and 8x crops. No blocking design defect. Non-blocking notes for the execution step and the final review:

- **G-1 (hull damage at 1x).** The hull damage is quiet at gameplay scale; the "broken" read leans on the stack, the steam and the wheel. Acceptable because the spec keeps the silhouette, and the `impact_stunned` pitch and the emitters add motion. Do not add brightness or new colours to compensate; if the owner wants it louder, a second steam puff or a darker crack outline is the cheaper lever.
- **G-2 (wheel).** The brass hub stays the brightest spot on the wheel, but it sits on the tie bar and reinforces the jam point rather than the threat. Keep it.
- **G-3 (remnant).** In the execution `landed` dust and any runtime capture, keep the contact shadow tight under the bow so the arch reads as a bent beam, not a ramp.
- **G-4 (composition artifacts, not key defects).** In `doz02_lit_after_640x360.png` a Floor 1 wall-torch flame is composited over the cab deck in front of the driver, and a background orange point light lands on the tie bar's upper end; at 1x both read as fire on a disabled machine, against rule 2. The keys carry no fire pixels. The execution evidence and the Bevy wiring must draw wall-torch flames behind the machine (or frame the capture so no torch sits behind the cab) and confirm no light coincides with the bar.
- **G-5 (stand-in driver).** The driver in the after scene is the alert `engineer_hold` stand-in (N-5); the slump frames must land before the owner review, since the alert face undercuts "harmless".

## Regenerate

```sh
python3 scripts/author_dozer_doz02.py              # rewrites the grids, validator configs, manifests and the damage contract (--check verifies them)
bash scripts/export_assets.sh                      # builds the .ase and the Floor 1 kit atlases the compositor needs
python3 scripts/make_dozer_doz02_review.py         # writes every PNG here
```

`SHA256SUMS` lists the hash of every file here except itself and this README.

## Files

| File | What it shows |
| --- | --- |
| `doz02_contact_x1.png`, `doz02_contact_x4.png` | Every DOZ-02 key frame by atlas and tag, at 1x and 4x: the jammed wheel, the hull, cab and tub in `pitch_0`, `pitch_1` and `disabled`, the bent yoke, the stop frame (`raised`, `pulley`), the remnant (`landed`, `landed_hit`) and the brace (`intact`) |
| `doz02_key_sheet_x4.png` | The keys by gate item: the DOZ-01 live wheel beside the jammed wheel and the live yoke beside the bent yoke (x4); the assembled machine intact (DOZ-01) beside `disabled` (x2); `pitch_0` and `pitch_1` with the riders lurching on their pitched seats (x2); the stop frame raised with its pulley, the two remnants and the brace (x4) |
| `doz02_lit_before_640x360.png` | Room 3 lit at 1x before the drop: the machine chasing, the frame raised on its sling, the haul line to the brace, Carl at the brace |
| `doz02_lit_after_640x360.png` | The same after a hit: the disabled machine (jammed wheel, bent yoke, damage, thin steam and a black wisp), the `landed_hit` remnant under the front, the driver left in the cab (stand-in: `engineer_hold` until the slump exists) |
| `doz02_lit_pair_x2.png` | The two lit scenes stacked at 2x |

## Weakest points (for the gate and the reviewer)

- The pitch shows 1 px stair steps along the deck bottom (design note N-1; integer column translation).
- The haul line is drawn by the review script, not from a tile set (N-3).
- The driver in the `disabled` scene is a stand-in (N-5); the riders are absent after the hit, because their tumbles are execution frames.
- The jammed wheel keeps the DOZ-01 spoke and hub pixels; only the ring, the spikes and the bar change. Judge at 1x whether the hub's brass still draws the eye.
