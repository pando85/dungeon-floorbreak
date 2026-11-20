# CF02 fixture replays

One file per `cf02_*` fixture: `headless_<fixture>.txt`, the `COMBAT_EVENT`, `COMBAT_STATE` and `PARTY_STATE` lines of `floor01_scene --headless-floor01-smoke --party-fixture <fixture>`.

This folder is **shared and append-only**: every COMBAT-FUN-02 PR adds the replays of its own fixtures here (and only changes one it owns), so the frozen `scripts/ci_floor01_combat.sh headless` loop over `--list-fixtures cf02_` never needs editing. Per-PR captures and notes stay in `docs/evidence/cf02/<pr-id>/`. COMPANION-AI-01 replays live in `docs/evidence/companion-ai/`.

CF02-A adds the thirteen `cf02_*` fixtures of the Carl branches, the lying goblin and Donut's hold (`cf02_kick_then_stomp`, `cf02_stomp_whiff`, `cf02_stomp_standing`, `cf02_stomp_no_stand_up`, `cf02_confirm_kick`, `cf02_whiff_no_confirm`, `cf02_down_physical_beats_confirm`, `cf02_jab_over_lying`, `cf02_lethal_kick_far`, `cf02_pounce_keeps_down`, `cf02_pounce_hold`, `cf02_stomp_boss`, `cf02_2p_same_tick`).
