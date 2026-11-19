# Donut biscuit unlock and mana — v0 proposal (HUD-DEBT-02)

**Status:** proposal, owner signoff required. Design only: nothing here is implemented, and the numbers are first tuning values for a playtest, not balance. Game feel stays the owner's call.

**Inputs:** [control contract](mvp-controller-contract.md) (Donut primary = swat before the biscuit, Magic Missile after), [Floor 01 progression](floor-01-gameplay-progression-v0.md), [characters](characters.md), [living horizon](character-progression-horizon.md), [Magic Missile animation spec](../02-art/donut-magic-missile-anim-03-spec.md) (cast timeline, placeholder projectile numbers), the hybrid-A HUD native spec §2.1/§2.4/§5/§6 (`docs/02-art/hybrid-hud-a-native-spec.md`, in draft PR #103: 94 px mana track, 8 segments of 10 px, depleted cues, muted chip), `src/combat.rs` (60 Hz fixed tick, `HERO_HEALTH = 4`, `ENEMY_HEALTH = 2`, Carl jab 14 ticks for 1 damage).

**Intent in one line:** the biscuit turns a fragile cat into a self-important ranged threat, but her magic runs out fast enough that she still has to pick her shots and lean on Carl. Running dry is a beat of exposure, never a soft-lock.

## 1. Unlock (domain flag)

- **What:** one IP-neutral progression flag per hero, `unlocks.primary_ranged` on the companion hero slot. The fan layer names it "biscuit". Default `false`. Carl has no mana and no flag.
- **Where:** an authored, once-only `UnlockTrigger` entity in Floor 01 level data at the Tutorial Guild beat. It fires on the first tick Donut's pivot enters its zone, whoever controls her (P2, the solo player or the companion AI), so solo play cannot miss it. No Interact press is needed, because Interact does not exist yet. Until the Guild room exists, the greybox places a placeholder trigger in a data-authored x range, labelled as a placeholder.
- **Deterministic and replay-safe:** the trigger is evaluated in the fixed tick and emits `UnlockGranted { tick, hero, unlock }` into the capped event log. The flag is part of the state hash. Fixtures declare their starting unlocks (`unlocks: []` or `["primary_ranged"]`), so post-unlock scenarios do not have to replay the Guild.
- **Persistence:** the flag survives a defeat retry and a checkpoint restart once granted. It is run progression, not attempt state. A fixture restart (`R` / Select) restores the fixture's declared starting state. There is no disk save yet. When one exists, the flag goes into it.
- **At unlock:** mana starts full. The HUD plays its 24-tick reveal, which is presentation only and never blocks a cast.

## 2. Mana resource (proposed numbers)

| Knob | Value | Rationale |
| --- | --- | --- |
| `mana_max` | **8 charges** (integer) | One charge per HUD segment, so the bar shows exactly what she has |
| `missile_cost` | **1 charge** | A full bar is 8 bolts. Goblins have 2 HP and a bolt does 1, so a full bar kills 4 goblins |
| `missile_damage` | 1 | Same as Carl's jab. Her edge is range, not damage |
| cast cycle | 16 ticks (spec re-fire placeholder) | A full bar empties in about 2.1 s of spamming |
| `regen_delay_ticks` | **30** (0.5 s) after the last bolt's release | Rewards pacing and punishes holding the button down |
| `regen_period_ticks` | **40** (0.67 s) per charge | Full refill from 0 takes 30 + 8 x 40 = **350 ticks (5.8 s)** |
| sustained rate | about 1 bolt per 70 ticks (0.86 /s) | Burst 3.75 /s. Carl stays the sustained damage source |
| regen in combat | **yes, always** | There is no reliable "in combat" state to gate on. A hidden gate invites kiting exploits, and the delay already does the pacing work |

- **Spend rule:** the press needs `mana >= cost`. The charge is spent on the **release tick (cast tick 4)**, when the bolt spawns. If a hurt interrupts the gather (ticks 0-3), nothing is spent.
- **At 0 (recommended):** no cast, no projectile, no action lock. The press is dropped, not buffered until a charge comes back, so it cannot fire by surprise later. Feedback: the HUD's muted `MISSILE` chip, plus a placeholder empty-click sound. No new art. Her fallback is the one she already has: the **pounce** on Triangle/Y.
- **Alternative B:** at 0 mana, primary does the pre-biscuit **swat**. The button is never dead, but the muted `MISSILE` chip would then lie about what the button does, so the HUD chip rule would need changing.
- No mana pickups, potions or loot in v0.

## 3. Swap, 2P and damage

- **Mana belongs to Donut, not to a player slot.** A solo swap neither refills nor resets it. In 2P, P2's Donut uses it. Regen runs whoever controls her.
- **Companion AI** (WORK-006) pays the same cost and keeps `ai_mana_reserve = 2` charges, so a solo swap-in never finds her empty. Until the AI attacks, the reserve is unused.
- **Donut damage (HUD-DEBT-01):** a hurt cancels an unreleased cast at no cost. A hurt does not drain mana or reset the regen delay. Mana is never an HP shield. Defeat and retry restore full mana.

## 4. HUD mapping without fake precision

The HUD rule `fill_px = round(mana / mana_max * 94)` with `mana_max = 8` gives 0, 12, 24, 35, 47, 59, 71, 82, 94 px. Each value ends inside the inter-segment gap or on the end of a segment, and gap pixels stay ink, so the bar always shows **whole segments**, one per charge. The HUD rule does not change. Depleted (fill 0) triggers the three existing cues plus the 6-tick flash. The charge being refilled is **not** drawn as a partial segment.
**Alternative (owner choice):** show regen progress as a 1 px row inside the next empty segment. That is honest, because it shows a real timer, but it is extra HUD work and more noise.

## 5. Data, not hardcoded

The knobs live in `assets/data/gameplay/companion_magic_v0.json`: `mana_max`, `missile_cost`, `missile_damage`, `regen_delay_ticks`, `regen_period_ticks`, `ai_mana_reserve`, `empty_press` (`"dry"` or `"swat"`), plus the projectile placeholders from the animation spec (6 px/tick, 240 px range, re-fire tick 16). They are loaded and validated at startup. A load error stops the scene and names the field. While HUD-A has 8 segments, validation requires `mana_max == 8` (changing it needs a HUD change). It also requires `1 <= missile_cost <= mana_max` and positive periods. Engine names stay IP-neutral. Only the fan layer says "biscuit" or "Magic Missile".

## 6. Tests and evidence

- **Unit (Bevy-free):** the flag is `false` by default and primary is the swat before unlock. The trigger fires once and is idempotent. The flag survives a retry and a checkpoint restart, and a fixture restart restores the declared state. The charge is spent on tick 4 and not on a gather interrupted by a hurt. A press at 0 spawns nothing and buffers nothing. A charge comes back exactly at release + 30 + 40·k ticks, a refill from empty takes 350 ticks, and mana never goes above max. A swap leaves mana unchanged. The table of `fill_px` values for 0..=8 is checked. Bad configs are rejected.
- **Scenarios (state hash plus event log, replayed twice):** `donut_unlock` (walk through the trigger, `UnlockGranted`, one bolt). `donut_mana_drain` (8 bolts, a dry press, regen to full, 350 ticks checked).
- **Real captures** of pre-unlock, post-unlock and depleted, to retire the HUD fixture previews.

## 7. Left for the owner's game-feel playtest

Whether 8 bolts feel generous or stingy. Whether the delay and period feel like pacing or nagging. Dry press versus swat fallback. Whether P2 Donut is fun after the biscuit and still needs Carl. Whether regen progress needs to be visible. The AI reserve, once the AI attacks.

## Living-horizon fit

This fits [the horizon](character-progression-horizon.md). Magic Missile becomes Donut's primary after the biscuit on the same button. She has no magic before it. Her feline contact stays. No new input. Carl gets no counterpart resource. Later "core action mastery" upgrades (cheaper bolts, faster regen) can be data changes to these knobs. The horizon text does not need an update.
