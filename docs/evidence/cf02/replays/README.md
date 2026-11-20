# CF02 fixture replays

One file per `cf02_*` fixture: `headless_<fixture>.txt`, the `COMBAT_EVENT`, `COMBAT_STATE` and `PARTY_STATE` lines of `floor01_scene --headless-floor01-smoke --party-fixture <fixture>`.

This folder is **shared and append-only**: every COMBAT-FUN-02 PR adds the replays of its own fixtures here (and only changes one it owns), so the frozen `scripts/ci_floor01_combat.sh headless` loop over `--list-fixtures cf02_` never needs editing. Per-PR captures and notes stay in `docs/evidence/cf02/<pr-id>/`. COMPANION-AI-01 replays live in `docs/evidence/companion-ai/`.

CF02-A adds the thirteen `cf02_*` fixtures of the Carl branches, the lying goblin and Donut's hold (`cf02_kick_then_stomp`, `cf02_stomp_whiff`, `cf02_stomp_standing`, `cf02_stomp_no_stand_up`, `cf02_confirm_kick`, `cf02_whiff_no_confirm`, `cf02_down_physical_beats_confirm`, `cf02_jab_over_lying`, `cf02_lethal_kick_far`, `cf02_pounce_keeps_down`, `cf02_pounce_hold`, `cf02_stomp_boss`, `cf02_2p_same_tick`).

CF02-H adds fourteen more: `cf02_air_kick_jump_in`, `cf02_air_kick_relaunch_once`, `cf02_dive_lying`, `cf02_dive_standing_knockdown`, `cf02_dive_bounce_kick`, `cf02_dive_whiff_recovery`, `cf02_bailout_escapes_swing`, `cf02_bailout_no_attack_until_land`, `cf02_buffer_turns`, `cf02_buffer_no_early_chain`, `cf02_jump_attack_same_tick`, `cf02_landing_grace`, `cf02_cancel_table`, `cf02_dive_boss`.

CF02-B adds nine: `cf02_contact_knockdown`, `cf02_contact_below_threshold`, `cf02_contact_cancels_swing`, `cf02_no_chain`, `cf02_body_bowl`, `cf02_air_relaunch_contact`, `cf02_wall_slam_kill`, `cf02_wall_slam_body`, `cf02_layout_edge_no_slam`.

CF02-G adds eleven: `cf02_morale_witness_hesitate`, `cf02_morale_spectacle_panic`, `cf02_morale_panic_once`, `cf02_morale_last_goblin`, `cf02_morale_cornered`, `cf02_morale_enraged_tell_unchanged`, `cf02_coward_backoff_kick`, `cf02_coward_jab_still_hits`, `cf02_showoff_taunt_punish`, `cf02_pincer_token`, `cf02_restart_reshuffles`.
