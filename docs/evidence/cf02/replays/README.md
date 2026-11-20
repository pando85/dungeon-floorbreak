# CF02 fixture replays

One file per `cf02_*` fixture: `headless_<fixture>.txt`, the `COMBAT_EVENT`, `COMBAT_STATE` and `PARTY_STATE` lines of `floor01_scene --headless-floor01-smoke --party-fixture <fixture>`.

This folder is **shared and append-only**: every COMBAT-FUN-02 PR adds the replays of its own fixtures here (and only changes one it owns), so the frozen `scripts/ci_floor01_combat.sh headless` loop over `--list-fixtures cf02_` never needs editing. Per-PR captures and notes stay in `docs/evidence/cf02/<pr-id>/`. COMPANION-AI-01 replays live in `docs/evidence/companion-ai/`.

Empty after CF02-00 (no `cf02_*` fixture exists yet).
