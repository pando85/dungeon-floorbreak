# Donut biscuit unlock and mana — v0 design (HUD-DEBT-02)

**Status:** adopted (orchestrator, owner directive 2026-11-19); implementation pending HUD-DEBT-04/01 and Q10; game-feel review pending. Design only: nothing here is implemented, and the numbers are first tuning values for a playtest, not balance.

**Adopted options:**

- Unlock at Tutorial Guild entry, persisting across retry and checkpoint restart.
- 8 charges. Magic Missile costs 1 and does 1 damage.
- Regen per the fixed rule in section 2 (30-tick delay after a release, then 40 ticks per charge).
- HUD shows whole segments only. Regen progress is not drawn.
- Mana is owned by Donut. The companion AI never spends below 2 charges. A hit cancels an unfired cast at no cost.
- Tuning lives in a hash-pinned JSON data file (section 5).

**Deferred, not adopted:** Alternative B (swat on an empty press) and the regen-progress row, both left for the playtest. The 0-mana fallback and the pre-unlock swat depend on the Donut moveset (Q10 of the 2P open-questions analysis), which is deferred.

**Inputs:** [control contract](mvp-controller-contract.md) (Donut primary = swat before the biscuit, Magic Missile after), [Floor 01 progression](floor-01-gameplay-progression-v0.md), [characters](characters.md), [living horizon](character-progression-horizon.md), [Magic Missile animation spec](../02-art/donut-magic-missile-anim-03-spec.md) (cast timeline, placeholder projectile numbers), the hybrid-A HUD native spec §2.1/§2.4/§5/§6 (`docs/02-art/hybrid-hud-a-native-spec.md`, merged in PR #103: 94 px mana track, 8 segments of 10 px, depleted cues, muted chip), `src/combat.rs` (60 Hz fixed tick, `HERO_HEALTH = 4`, `ENEMY_HEALTH = 2`, Carl jab 14 ticks for 1 damage).

**Intent in one line:** the biscuit turns a fragile cat into a self-important ranged threat, but her magic runs out fast enough that she still has to pick her shots and lean on Carl. Running dry is a beat of exposure, never a soft-lock once Q10 gives her a physical move (see section 2).

## 1. Unlock (domain flag)

- **What:** one IP-neutral progression flag per hero, `unlocks.primary_ranged`, on the hero entity whose profile declares a ranged-primary capability (the fan layer: Donut). The fan layer names it "biscuit". Default `false`. Carl has no mana and no flag.
- **Where:** an authored, once-only `UnlockTrigger` entity in Floor 01 level data at the Tutorial Guild beat. It fires on the first tick Donut's pivot is inside its zone, whoever controls her (P2, the solo player or the companion AI), so solo play cannot miss it. The zone is a half-open AABB: the minimum bounds are inclusive and the maximum bounds exclusive. No Interact press is needed, because Interact does not exist yet. Until the Guild room exists, the greybox places a placeholder trigger in a data-authored x range, labelled as a placeholder.
- **No grant while defeated:** the trigger does not fire for a hero during the party-defeat delay (D1, 50 ticks). It fires on the first tick after the retry restores her, if she is inside the zone.
- **Deterministic and replay-safe:** the trigger is evaluated in the fixed tick and emits `UnlockGranted { tick, hero, unlock }` into the capped event log. The flag is part of the state hash. Fixtures declare their starting unlocks (`unlocks: []` or `["primary_ranged"]`), so post-unlock scenarios do not have to replay the Guild.
- **Persistence:** the flag survives a defeat retry and a checkpoint restart once granted. It is run progression, not attempt state. A fixture restart (`R` / Select) restores the fixture's declared starting state. There is no disk save yet. When one exists, the flag goes into it.
- **At unlock:** mana starts full. The HUD plays its 24-tick reveal, which is presentation only and never blocks a cast.

## 2. Mana resource (adopted numbers)

| Knob | Value | Rationale |
| --- | --- | --- |
| `mana_max` | **8 charges** (integer) | One charge per HUD segment, so the bar shows exactly what she has |
| `missile_cost` | **1 charge** | A full bar is 8 bolts. Goblins have 2 HP and a bolt does 1, so a full bar kills 4 goblins |
| `missile_damage` | 1 | Same as Carl's jab. Her edge is range, not damage |
| cast cycle | 16 ticks (spec re-fire placeholder) | A full bar empties in about 2.1 s of mashing |
| `regen_delay_ticks` | **30** (0.5 s) after a bolt's release | Rewards committing to a burst, then repositioning. Trickle-firing is the slowest use |
| `regen_period_ticks` | **40** (0.67 s) per charge | Full refill from 0 takes 30 + 8 x 40 = **350 ticks (5.8 s)** |
| sustained rate | **0.86 to 1.04 bolts/s** (burst 3.75 /s) | See the cost formula below. Carl stays the sustained damage source |
| regen in combat | **yes, always** | There is no reliable "in combat" state to gate on. A hidden gate invites kiting exploits, and the delay already does the pacing work |

**Regen rule (exact):**

- The regen timer resets **only on a release tick** (cast tick 4, when a bolt spawns). Partial progress toward the next charge is discarded on a release.
- Dry presses, gathers cancelled by a hurt, and cast starts do not touch the timer. Mashing at 0 therefore cannot block regen.
- The timer runs on every tick, including the gather and cast ticks 0-3 of a cast that has not yet released.
- While `mana < mana_max`, charge k (k = 1..=8, counting from the last release) returns at `release + 30 + 40 * k` ticks. At `mana == mana_max` the timer is idle.

**Cost of a burst:** because each release restarts the delay, a burst of n bolts costs 16(n-1) + 30 + 40n ticks, which is 56 + 14/n ticks per bolt. Firing a bolt as soon as each charge returns is the worst case (70 ticks per bolt, 0.86 /s). Dumping all 8 and waiting is the best case (about 57.75 ticks per bolt, 1.04 /s). The delay rewards burst-then-wait, not trickle-firing.

- **Spend rule:** the primary is press-edge only. Holding the button does not repeat the cast, because the contract leaves auto-repeat and hold unresolved. The animation spec allows only a buffered press during the follow-through. The mana check (`mana >= cost`) runs on the tick a cast **starts**, also for a buffered press. The charge is spent on the **release tick (cast tick 4)**. If a hurt interrupts the gather (ticks 0-3), nothing is spent. A buffered press at the last charge is checked at its own cast start, not when it was buffered.
- **At 0, interim rule (adopted):** no cast, no projectile, no fallback move, no action lock. The press is dropped, not buffered until a charge comes back, so it cannot fire by surprise later. Feedback: the HUD's muted `MISSILE` chip, plus a placeholder empty-click sound. No new art. The HUD must not show a `POUNCE` chip, because chips follow live actions (HUD spec §2.4) and no pounce exists.
- **Dependency on Q10:** Donut has no swat or pounce in code or in an approved design. Q10 of the 2P open-questions analysis lists both as candidates that still need art and gameplay signoff. The intent "running dry is an exposure beat, never a soft-lock" holds once Q10 lands and gives her a physical action, expected to be the pounce on Triangle/Y. Until then a dry Donut has no attack, which is the interim rule above. Likewise, the pre-unlock primary is a swat only once Q10 delivers one. Before that, the pre-unlock primary is simply not a cast.
- **Alternative B (deferred):** at 0 mana, primary does the pre-biscuit swat. Available only once the swat exists (`empty_press: "swat"` is rejected by validation until then). The button is never dead, but the muted `MISSILE` chip would then lie about what the button does, so the HUD chip rule would need changing.
- No mana pickups, potions or loot in v0.

## 3. Swap, 2P and damage

- **Mana belongs to Donut, not to a player slot.** A solo swap neither refills nor resets it, and keeps the regen timer running. In 2P, P2's Donut uses it. Regen runs whoever controls her.
- **Companion AI** (WORK-006) pays the same cost and never spends below `ai_mana_reserve = 2` charges. A player can still drain her to 0 and swap back quickly, so a swap-in may find her empty. Until the AI attacks, the reserve is unused.
- **Donut damage (HUD-DEBT-01):** a hurt cancels an unreleased cast at no cost. A hurt does not drain mana or reset the regen delay. Mana is never an HP shield.
- **Restart:** defeat retry and any other restart that restores HP also restore full mana. A fixture restart restores the fixture's declared mana.

## 4. HUD mapping without fake precision

The HUD rule `fill_px = round(mana / mana_max * 94)` with `mana_max = 8` gives 0, 12, 24, 35, 47, 59, 71, 82, 94 px. Implement it in integers as `(mana * 94 + mana_max / 2) / mana_max`, which avoids float ties at m = 2 and m = 6. The test pins the table. Each value ends inside the inter-segment gap or on the end of a segment, and gap pixels stay ink, so the bar always shows **whole segments**, one per charge. The HUD rule does not change. Depleted (fill 0) triggers the three existing cues plus the 6-tick flash. The charge being refilled is **not** drawn as a partial segment.
**Deferred to the playtest, not adopted:** show regen progress as a 1 px row inside the next empty segment. That is honest, because it shows a real timer, but it is extra HUD work and more noise.

## 5. Data, not hardcoded

The knobs live in `assets/data/gameplay/companion_magic_v0.json`: `mana_max`, `missile_cost`, `missile_damage`, `regen_delay_ticks`, `regen_period_ticks`, `ai_mana_reserve`, `empty_press` (`"dry"`, or `"swat"` once the swat exists), plus the projectile placeholders from the animation spec (6 px/tick, 240 px range, re-fire tick 16). They are loaded and validated at startup. A load error stops the scene and names the field. While HUD-A has 8 segments, validation requires `mana_max == 8` (changing it needs a HUD change). It also requires `1 <= missile_cost <= mana_max` and positive periods. Engine names stay IP-neutral. Only the fan layer says "biscuit" or "Magic Missile".

**Replay pinning:** scenarios and replays load the committed file and record its content hash in the replay header or fixture. A mismatch is a clear test failure, not silent drift. Alternatively a fixture declares its own knob values. A tuning edit therefore never silently changes a recorded outcome.

**Determinism and events:** the state hash covers the flag, `mana`, `regen_timer` and the cast phase. Capped events: `UnlockGranted{tick, hero, unlock}`, `ManaSpent{tick, hero, left}`, `ManaRegained{tick, hero, now}` and `CastRejected{tick, hero, reason: Dry}`. The drain scenario must fit within the event-log cap.

## 6. Tests and evidence

- **Unit (Bevy-free):** the flag is `false` by default and the pre-unlock primary does not cast (a swat once Q10 lands). The trigger fires once and is idempotent, and does not fire while the hero is defeated. The flag survives a retry and a checkpoint restart, and a fixture restart restores the declared state. The charge is spent on tick 4 and not on a gather interrupted by a hurt. A press at 0 spawns nothing and buffers nothing. Charge k returns exactly at release + 30 + 40·k ticks for k = 1..=8, a refill from empty takes 350 ticks, and mana never goes above max. Regen rule: (a) a release mid-period discards partial progress, (b) a dry press and a cancelled gather do not reset the delay, (c) the delay runs during cast ticks 0-3. A swap mid-regen keeps the timer and leaves mana unchanged. A buffered press during the follow-through with the last charge is checked at its own cast start. The P2 Donut unlock fires in 2P. The state hash includes mana. The `fill_px` table for 0..=8 is checked. Bad configs are rejected, including `empty_press: "swat"` while no swat exists. A config-hash mismatch fails the replay. No `POUNCE` chip shows while the move is not live.
- **Scenarios (state hash plus event log, replayed twice):** `donut_unlock` (walk through the trigger, `UnlockGranted`, one bolt). `donut_mana_drain` (8 bolts, a dry press, regen to full, with `ManaSpent`, `CastRejected` and `ManaRegained` events checked over the 350 ticks).
- **Real captures** of pre-unlock, post-unlock and depleted, to retire the HUD fixture previews.

## 7. Left for the owner's game-feel playtest

Whether 8 bolts feel generous or stingy. Whether the delay and period feel like pacing or nagging. Dry press versus swat fallback (Alternative B, once Q10 lands). Whether P2 Donut is fun after the biscuit and still needs Carl. Whether regen progress needs to be visible. The AI reserve, once the AI attacks. The owner may change any adopted value in a follow-up PR, since the knobs are data.

## Living-horizon fit

This fits [the horizon](character-progression-horizon.md). Magic Missile becomes Donut's primary after the biscuit on the same button. She has no magic before it. Her feline contact stays. No new input. Carl gets no counterpart resource. Later "core action mastery" upgrades such as faster regen can be data changes to these knobs. Cheaper bolts cannot: `missile_cost` is already at its validated minimum of 1 and `mana_max` is locked to 8, so that would need a cost model or HUD change. The horizon text does not need an update.
