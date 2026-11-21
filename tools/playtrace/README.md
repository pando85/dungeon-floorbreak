# PLAYTRACE-01 PT-D: exporter, dataset checks and behaviour-cloning smoke

Optional offline tooling over recorded sessions (`playtrace-session/1`). The game never
depends on it: the shipped binary and `cargo test --locked --all-targets` build no Python
and no NumPy. Contract: [PLAYTRACE-01](../../docs/03-tech/playtrace-01-session-capture.md)
(section 7.2 records the adopted PT-D defaults) and the
[work order](../../docs/07-agents/playtrace-01-workorder.md).

**Recordings and anything derived from them stay outside the repository.** Sessions live in
`$XDG_DATA_HOME/dungeon-floorbreak/playtrace`, datasets default to
`$XDG_DATA_HOME/dungeon-floorbreak/datasets`, and the exporter refuses an `--out` inside the
checkout. Nothing here is a human demonstration except what a human actually recorded; the
committed tests use only **synthetic** sessions (a pure-Python writer with a stub verifier,
and fixture-scripted sessions written by `playtrace synth-fixture` that really pass the
verifier). Never commit a session or a dataset.

## Commands

Rust binary (`cargo build --bin playtrace`): `list`, `stats [ID]` (read-only aggregate counts
and per-hero human/AI action histograms), `inspect ID`, `verify ID`, `recover ID`,
`delete ID --yes`, and `synth-fixture --dir DIR` (writes a synthetic session for tests; it
refuses a `--dir` that is or lies inside the default recording store). `stats` prints
`synthetic=true|false` per session, labels synthetic hero lines `class=synthetic_human` and keeps
synthetic sessions out of its totals (`synthetic_sessions`, `synthetic_ticks`).

Python (run from this directory, numpy only):

```bash
python -m playtrace_tools export-bc          --verifier PATH [--session FULL_ID ...] [--hero carl|donut|both] [--context N]
python -m playtrace_tools export-transitions --verifier PATH [--actor human|ai|both] [--hero ...] [--context N]
python -m playtrace_tools check-dataset DATASET_DIR
python -m playtrace_tools bc-smoke DATASET_DIR [--history K] [--epochs N] [--report FILE]
```

Common options: `--dir` (session base), `--out`, `--seed` (split), `--dev-frac`,
`--holdout-frac`, `--allow-synthetic`, `--export-consented` (needs `--session`), `--root`. `--session` takes full ids only: an
empty id, a prefix, an unknown or an ambiguous id is an error.

## What the exporter guarantees

- Only sessions that pass `playtrace verify` (exit 0, `result=PASS scope=full`) and the
  exporter's own structural re-check (chunk checksums, manifest table, contiguous ticks,
  control integrity) and whose files did not change during the export. Everything else is
  listed in `dataset.json` under `rejected` with a stable reason code (`invalid_journal`,
  `capture_incomplete`, `observation_partial`, `unsupported_schema`, `unsupported_obs_schema`,
  `unsupported_action_schema`, `verify_failed:<verifier code>`, `verify_not_full_scope`,
  `verifier_unavailable`, `modified_during_export`, `other_tester_not_consented`,
  `synthetic_not_allowed`, `synthetic_marker_mismatch`, `unsupported_label`, `label_mismatch`, `invalid_observation`, `illegal_action`).
- **The verifier must be built from the same source as the recording** (PTI-10: a different
  source is `build_mismatch`). Pass the matching binary with `--verifier` (or
  `PLAYTRACE_BIN`) and the checkout of that source with `--root`.
- Human and AI actions are separate streams (`human/`, `ai/`), decided by the recorded control
  source of the hero on that tick, and are never relabelled. A human hero's action is the
  routed input (the intent); an AI hero's action is the effective input.
- `includes_other_tester` sessions are excluded unless `--export-consented` **and** an explicit
  `--session`. A synthetic fixture needs `--allow-synthetic`, marks the dataset synthetic and is
  never mixed with real sessions.
- Policy inputs are ObservationV2 only (345 `float32`, exact bits, plus optional previous
  actions). `diagnostics.jsonl` (privileged) is never opened. No reward is exported; rows carry
  event ids and the event hash only (the event text is build-scoped and never parsed).
- Every row keeps `session_id`, `chunk`, `line`, `t`, `pre`/`post` hashes and the raw action
  bits; `check-dataset` re-reads the sources and proves each row maps to its tick, that
  `obs` is that tick's `o_t` and `next_obs` the following one or the episode-end observation,
  that `next_kind`/`terminal`/`truncated`/`outcome` follow from the source, and that the
  `mask`, `split`, `terminal`, `next_valid` and `context` arrays equal their rows.
- Splits are by session (or `participant_pseudonym` when a manifest has one), deterministic
  from `--seed`, with a leakage check (no group, session, episode or trajectory in two splits,
  no duplicate `(session, tick, hero)`). Datasets are content-addressed
  (`dataset-<id>/dataset.json` lists every file hash).

## What it does not claim

The BC smoke proves the loader, train and validation path. A few minutes of one player cannot
produce a usable policy, and a within-scenario session split is not a hold-out of new geometry
or enemies. The numbers it prints say how a tiny model fits that data, nothing more.
