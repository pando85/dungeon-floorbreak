# COMBAT-FUN-02 evidence

Index of the COMBAT-FUN-02 evidence. CF02-00 creates it; each later PR writes only `docs/evidence/cf02/<pr-id>/` and CF02-REVIEW owns this index afterwards.

| Item | What it is |
| --- | --- |
| [`gap-report.md`](gap-report.md) | CF02-00: the Q23 audit of B2 contact, B3 decapitation, B5 blast, B4 llama finishes and the camera traces (measured, with hypotheses marked) |
| [`audit/audit_events.md`](audit/audit_events.md) | Event log joined with the presentation camera: for each audited event, where the bodies were and whether they were inside the view |
| [`audit/camera_trace.md`](audit/camera_trace.md) | Pan, impact kick and zoom per beat and camera; the legacy solo-swap pan trace |
| [`captures/`](captures/) | Eleven real 640x360 frames of the audited spectacles, their `COMBAT_STATE` lines and `SHA256SUMS` |
| [`scorecard-template.md`](scorecard-template.md) | The playtest scorecard the next session must fill in: build SHA, flags (including `--cf02-seed`), device and pad, resolution, gore preset, CINE config, deaths and restarts per attempt, cinematic count |

## Reproduce

```sh
cargo build --locked --bin floor01_scene
bash scripts/export_assets.sh
python3 scripts/cf02_baseline_audit.py docs/evidence/cf02/audit
xvfb-run -a -s '-screen 0 1280x720x24' bash scripts/cf02_baseline_captures.sh
```

The audit tables are deterministic (headless replays). The captures are a software-renderer screenshot at a fixed tick: they are evidence of what was drawn, not reproduced bit for bit by CI. `bash scripts/ci_floor01_combat.sh headless` replays the `cf02_*` and `companion_*` fixture registries (both empty after CF02-00) and checks that `--cf02` replays DEMO-01 like `--demo`.

Rights: the frames show only the committed original Carl, Donut, goblin and llama art under the recorded scope in the [rights register](../../04-production/rights-and-release.md). No new art in this PR. Model attribution: Claude Sonnet (this PR); design by the COMBAT-FUN-02 design PR.
