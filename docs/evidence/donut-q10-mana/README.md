# Donut Q10 moves and mana: evidence (WORK-023, HUD-DEBT-02)

**Game feel is pending.** Every value is adopted by the orchestrator under the owner's design-defaults directive of 2026-11-19 and no human has played it. Real controllers are not verified. The cast head, the bolt and the swat and pounce frames are the ANIM-DONUT-02 and ANIM-DONUT-03 candidate art (not owner-approved); no Donut pixel was created or edited here.

Design: [Q10 Donut moveset](../../02-art/donut-moveset-q10.md), [Donut mana v0](../../01-design/donut-mana-v0-proposal.md). Integration notes: [floor01-scene.md](../../03-tech/floor01-scene.md#donut-moves-and-mana-work-023-hud-debt-02).

## Real captures

Frozen window captures of `floor01_scene` (real atlases, real HUD, real simulation) from `scripts/ci_floor01_render_smoke.sh hud` (1280x720, the 640x360 canvas at scale 2) and `hud-deck` (1280x800, the 640x400 Deck canvas), under `xvfb-run`. Each passed the HUD pixel-integrity check and the check that the rendered state equals the headless replay. The Donut plate in each is the real pool and the real chips, never a `--hud-preview` fixture.

| State | Fixture and tick | What it shows |
| --- | --- | --- |
| `pre-unlock` | `q10_swat_interrupt`, tick 60 | P2 Donut before the biscuit: `SWAT` and `POUNCE` chips, no mana bar |
| `swat` | `q10_swat_interrupt`, tick 98 | The swat paw at the goblin's shins on the last tick before the hit (tick 99) |
| `pounce` | `q10_pounce_hit`, tick 112 | The pounce in flight, low and forward, toward the goblin Carl weakened |
| `post-unlock` | `q10_mana_regen`, tick 60 | After the biscuit: `MISSILE` and `POUNCE`, 5 of 8 charges as whole segments (47 px), two bolts in flight |
| `depleted` | `q10_mana_regen`, tick 140 | 0 charges: empty bar, the depleted cues and the muted `MISSILE` chip; `POUNCE` stays lit |

Files: `donut-q10-<state>-640x360-2x-1280x720.png` and `donut-q10-<state>-640x400-2x-1280x800.png`, checksums in `SHA256SUMS`.

## Headless replays

`headless_<fixture>.txt` holds every `COMBAT_EVENT`, the final `COMBAT_STATE` and the final `PARTY_STATE` of each fixture, from `floor01_scene --headless-floor01-smoke --party-fixture <name>`. `scripts/ci_floor01_combat.sh headless` runs each twice (and at gore presets 1 and 4 and with `--hud-preview`) and diffs them against these files; the `PARTY_STATE` hash is the value the goldens in `src/donut_fixtures.rs` pin.

| Fixture | Scenario | Key events |
| --- | --- | --- |
| `q10_swat_interrupt` | 2P: a goblin tells at Carl, P2 swats the tell, Carl jabs and hauls it down | swat hit (damage 0) and `goblin_interrupted` on tick 99, jab hit on 103 |
| `q10_swat_guard` | Solo as Donut: the light guard | swat hits on 49 and 184; a `glance reason=guarded` on 89, and that swing lands |
| `q10_pounce_hit` | 2P: Carl jabs, Donut pounces | pounce hit and defeat on 116, `instigator=1`, routine force |
| `q10_pounce_miss` | Solo as Donut: a pounce from about 100 px | no hit; she lands 42 px further and the swing finds her on 144 |
| `q10_dry_press` | 2P: biscuit, 0 mana | `cast_rejected` on 10, 20 and 30; first charge on 70; cast on 100, spent on 104 |
| `q10_mana_regen` | 2P: 8 bolts, a dry press, the refill | `mana_spent` every 16 ticks from 14; `cast_rejected` on 140; charge k on `126 + 30 + 40 k`, full on 476 |
| `q10_unlock_persists` | 2P: the placeholder Guild zone, a bolt, one-hit defeat, retry | `unlock_granted` once on 55; after the retry she casts again with a full pool and no second grant |
| `q10_pounce_boss` | 2P in the boss arena | `boss_hurt source=1 damage=1` on 34; the pound that follows is not interrupted |

## Hash gating

The legacy goldens (the 13 combat `EXPECTED_HASHES`, `GOLDEN_WALK_RIGHT_400`, the story hash `931868d6afa56498`, the party and boss goldens) are unchanged. A unit test replays every party and boss fixture with `donut_moves` on and off and asserts equal hashes and events.

## Not covered

No frame-time measurement of its own (the per-tick work is one small state machine and a few boxes). The glance flash, the dry-press click, the 24 tick unlock reveal and Donut's hurt art are follow-ups.
