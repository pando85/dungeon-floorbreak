# Evidence: local 2P, solo swap and Donut damage

Evidence for WORK-005, WORK-006, HUD-DEBT-01 and HUD-DEBT-04 (one grouped PR). Checksums: [SHA256SUMS](SHA256SUMS). Design and decisions: [party-2p-open-questions](../../01-design/party-2p-open-questions.md). Implementation notes: [floor01-scene.md, Live party](../../03-tech/floor01-scene.md#live-party-work-005-work-006-hud-debt-01).

**Game feel is pending.** Every value shown here is adopted by the orchestrator on the owner directive of 2026-11-19 and no human has played it. Real controllers are not verified (no pads on CI or on the capture machine).

## Real captures

Real window captures of `floor01_scene` with the real exported atlases and the real HUD, taken at the default scale 2 (1280x720, the 640x360 virtual canvas doubled), by `scripts/ci_floor01_render_smoke.sh hud` under `xvfb-run` (each passed the HUD pixel-integrity check and the rendered-state equals headless-replay check). Frozen captures of a scripted **party fixture** (`--party-fixture NAME --capture-tick N`): the slot inputs go through the same `PartyState`, swap, drive and `CombatState::step_party` path as interactive play.

| File | Fixture and tick | What it shows |
| --- | --- | --- |
| `party-REAL-two-players-640x360-2x-1280x720.png` | `duo_independent_moves`, tick 70 | P1 tag on Carl, P2 tag on Donut; Donut stands where she was while Carl runs ahead; the camera follows the midpoint |
| `party-REAL-two-players-separation-wall-640x360-2x-1280x720.png` | `duo_separation_wall`, tick 240 | Carl stopped at the 448 px wall, Donut still at her start; both on screen |
| `party-REAL-solo-swap-640x360-2x-1280x720.png` | `solo_swap_continuity`, tick 120 | After the swap: `P1` on Donut's plate, `AI` and `COMPANION` on Carl's; Carl follows as the bot |
| `party-REAL-donut-hurt-640x360-2x-1280x720.png` | `donut_hurt_iframes`, tick 68 | Donut's HP bar is real (3 of 4) after a real goblin swing; her sprite shows the hit tint; Carl untouched |

## Headless replays

`headless_<fixture>.txt` holds every `COMBAT_EVENT`, the final `COMBAT_STATE` and the final `PARTY_STATE` of each party fixture, from `floor01_scene --headless-floor01-smoke --party-fixture <name>`. `scripts/ci_floor01_combat.sh headless` runs each twice and diffs them against these files; the `PARTY_STATE` hash is the same value that the Rust goldens in `src/party_scene.rs` pin (`party_scene::tests::the_party_fixtures_match_their_committed_goldens_and_replay_identically`).

| Fixture | Ticks | Final `PARTY_STATE` hash | Result |
| --- | --- | --- | --- |
| `duo_independent_moves` | 160 | `81ca65ec029b6e63` | Carl runs on P1, Donut jumps twice and runs left on P2 |
| `duo_separation_wall` | 420 | `1f40a48b874e1dc9` | the separation never exceeds 448 px |
| `solo_swap_continuity` | 320 | `0a3bcbeffb97ab5b` | swaps at ticks 90 and 200 |
| `donut_hurt_iframes` | 140 | `6b13a15ae7fc186c` | one `hero_hurt hero=donut` at tick 64, the second swing at 76 lands in her i-frames |
| `donut_defeat_party` | 520 | `d4815dd69ec48fff` | `hero_defeated hero=donut` at tick 349, restart at 398 (both at full HP at the restart tick; the final state shows Donut at 3 HP because a goblin hit her again afterwards) |
| `solo_guard_floors_at_one` | 520 | `78d4244fd6216569` | `health_left` 3, 2, 1, 1, 1: no defeat |
| `solo_follower_dodge` | 200 | `8f8b7dcc2cc5dc20` | no hit; the bot is airborne at each swing |

## Unchanged

The 13 combat fixture hashes (`combat_scene::tests::EXPECTED_HASHES`), the corridor `GOLDEN_WALK_RIGHT_400` and the story route (`--scenario walk_right --capture-tick 250` ends in `hash=931868d6afa56498`, tick 613 in `82b8f87fa644de48`) are byte-identical: scripted runs keep the legacy drive, and every live-party value is hashed only when the party is live.

## Not shown

No video or GIF, no capture with a real pad, no capture of the Q7 edge arrow (not drawn), no Donut hurt art (she flashes and blinks; she has locomotion frames only).
