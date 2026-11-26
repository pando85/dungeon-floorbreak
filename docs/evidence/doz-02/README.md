# DOZ-02 evidence: Dozer damage states and the stop frame, design keys

These are review renders for the [DOZ-02 design spec](../../02-art/doz-02-dozer-damage-spec.md). They are composed PNGs, not Bevy captures. The lit scenes use the A1 compositor (`scripts/env_kit_light.py`) on the Floor 1 kit. Nothing here is wired into a binary.

Status:

- **Design keys:** drawn (spec section 6). Routing score 4 of 4, Hard (spec section 1).
- **Silhouette gate:** **PENDING.** A fresh design-tier agent who did not draw the keys must answer the three questions below; the verdict is recorded here and in spec section 11.
- **Execution:** pending (spec section 7): the remaining frames, the export, the wiring.
- **Review:** pending, independent.

## Silhouette gate

Gate pack: `doz02_lit_before_640x360.png` and `doz02_lit_after_640x360.png` (judge at 1x first), `doz02_key_sheet_x4.png`, and the spec.

| Question | Verdict |
| --- | --- |
| 1. Does the disabled machine read as the DOZ-01 Dozer, broken and harmless? | PENDING |
| 2. Does the jammed wheel read as stopped and not lethal? | PENDING |
| 3. Does the remnant read as a fallen press frame? | PENDING |

Any "no" sends the keys back to design before execution starts.

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
