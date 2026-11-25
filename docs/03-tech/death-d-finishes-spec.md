# DEATH-D: decapitation and explosion finishes, design spec

**Status:** DESIGN SPEC, implemented by the DEATH-D executor PR (draft; deviations in its "Design questions" and the notes at the end of this page). Independent gate review: PASS WITH EDITS; all 15 edits applied. Written for the medium-tier executor; a separate top-tier reviewer checks the implementation against section 7. Gameplay acceptance (game feel) stays an owner decision after the merge. Design: Claude Opus. Execution: medium tier. Review: a separate top-tier agent.

**Builds on (merged):** DEATH-A (#79, defeat modes and the universal death language draft), DEATH-B (#94, [species death profiles](species-death-profiles.md), `CombatEvent::EnemyDefeated` with `cause`, `force`, `instigator`), DEATH-C (#97, the cosmetic presenter and gore pool, [floor 01 scene doc](floor01-scene.md#death-c-cosmetic-death-presenter-and-gore-pool)), the [death-system proposal](monster-death-system-proposal.md) (section F, "PR D"), the art specs [GOB-02-DEATH](../02-art/gob-02-death-spec.md) (sections 2, 5, 7, 8), [GOB-02B-DEATH](../02-art/gob-02b-death-spec.md) and [LLAMA-02-DEATH](../02-art/llama-02-death-spec.md), and [ANIM-CARL-03](../02-art/carl-dynamite-anim-03-spec.md) (the dynamite throw, art only).

**Owner rules this spec must keep:**

1. Decapitation and explosion are **rare, clearly caused, high-impact kills, never every jab** (backlog DEATH-D). A blunt hit never severs a head (proposal section C).
2. The gore preset is **presentation only**. Default 4 (owner, 2026-11-18), and the owner wants even more intensity. No preset value enters simulation state, an event or a replay hash.
3. Gameplay is deterministic under the fixed 60 Hz update and the seeded RNG. Exactly one `EnemyDefeated` per alive -> dead transition.
4. There are no explosives before the story unlock (controller contract: the EquippedSpecial does nothing before it). The explosive here is a **QA fixture charge**, not given to Carl and not placed on any route (section 1.3).
5. The data contracts stay IP-neutral (`shore_block_edge`, `explosion`, `charge`, `shear`, `blast`; never "dynamite" or a character name).

## 0. Summary

| Finish | Authoritative trigger (today) | Force | Finish id (goblin) | Producer in code |
| --- | --- | --- | --- | --- |
| Decapitation | The CP-03 falling block lands with its **vertical edge across an upright goblin's neck**: the head is under the block, the torso is not | Catastrophic | `goblin_decapitated` | **Exists** after this PR: `step_shore` (new cause `ShoreBlockEdge`) |
| Explosive breakup | A blast charge detonates **point blank**: its centre is inside the target's body box | Catastrophic | `goblin_exploded` | **Exists** after this PR as an authored QA charge (`src/blast.rs`); the dynamite special is a **future hook** that reuses it |
| (blast ring kill) | The same blast reaches the body within 20 px but not point blank | Heavy | `goblin_knockback_down` (thrown dead) | as above |

Nothing else selects either finish. A jab, haymaker or push kick (any `Melee` defeat, at any force), a block landing squarely on a body, a goblin lying down under the edge, and a wall impact all keep their DEATH-B/C finishes.

## 1. Authoritative trigger rules

### 1.1 What exists in code today (before this PR)

| Cause | Where | Force | Finish today |
| --- | --- | --- | --- |
| `DefeatCause::Melee` (jab, haymaker, push kick) | `resolve_strike` (`src/combat.rs`) | `Routine`, or `Heavy` for the launching push kick | `goblin_knockback_down` |
| `DefeatCause::ShoreBlock { player }` (CP-03 block, footprint 80 px, one evaluation on the landing tick) | `step_shore` (`src/combat_scene.rs`) | `Catastrophic` | `goblin_crushed` |

No blade hazard, explosive, wall-impact damage or other lethal cause exists. A lethal push kick defeats the goblin **on the hit tick** (it becomes `Airborne`, then `Down`); its later wall stop is not an event.

### 1.2 Decapitation: the block's edge across the neck (`ShoreBlockEdge`)

The CP-03 rule is unchanged: on the landing tick every living goblin whose body overlaps the footprint is defeated, once, in index order. DEATH-D only **classifies** each of those defeats before `Goblin::crush()` mutates the goblin:

* The goblin is **upright**: mode `Neutral`, `Attack` or `Hurt`. (`Launched`, `Downed` and the dead modes are never upright: a lying goblin under the edge is a crush; see open question 4.)
* Two x probes on its body, from the goblin's `x` (its pivot column, where the neck sits on GOB-01 frame 0: the head part's `neck` anchor lands on frame (28, 31), the pivot column, 24 px above the feet) and its `facing` `f` (+1 / -1; the head leads in the facing direction):
  * head probe `hx = x + f * NECK_PROBE_FORWARD`, with `NECK_PROBE_FORWARD = 4.0`;
  * torso probe `tx = x - f * NECK_PROBE_BACK`, with `NECK_PROBE_BACK = 4.0`.
* With the footprint's x range taken half-open, `[left, left + width)`: **shear** when `hx` is inside and `tx` is outside. Otherwise it is the ordinary crush.

That is an 8 px window per side, and only for a goblin that faces into the block: for the CP-03 footprint `[strike + 25, strike + 105)` a goblin facing -1 shears for `x` in `[R - 4, R + 4)` at the right edge `R`, a goblin facing +1 for `x` in `[L - 4, L + 4)` at the left edge `L`. A goblin facing away from the block at an edge has its head outside and its back under the stone: crush. Over the 104 px band in which the block kills a goblin, the shear window is 8 px per facing side, about 8 % of block kills: rare, but a player can learn it ("drop it just as he leans in").

On a shear the event is `EnemyDefeated { cause: ShoreBlockEdge { player }, force: Catastrophic, instigator: goblin.hit_by }`, and the domain state change is exactly `Goblin::crush()` (same mode, position and timers as a crush; only the cause differs). The constants live in `src/goblin.rs` (`NECK_PROBE_FORWARD`, `NECK_PROBE_BACK`, `Goblin::neck_probes(&self) -> Option<(f32, f32)>`, `None` when not upright); the edge test lives in `src/shore.rs` (`Shore::edge_shears(&self, probes: (f32, f32)) -> bool`). Both are Bevy-free and unit-tested.

Why this cause: it is the only existing physical edge in the game, it needs no new hazard, and it rewards the existing verbs (three hits on the shore, timing, positioning). The read is clear: the stone slams down at the goblin's face, the head pops out of the edge and the headless body recoils away from the stone.

**Not a decapitation (explicit rejections):**

* Any `Melee` defeat at any force (blunt cannot sever).
* A lethal push kick that carries the corpse into a wall. A wall is not sharp; a wall slam is a separate kinetic finish (the LLAMA-02 `slam` pair is the model) and needs its own owner rule. Open question 2.
* A launched living goblin's contact with another (CP-02 contact is never lethal).
* Cosmetic fragments (CP-04 stays off).

### 1.3 Explosion: an authored blast charge (`Explosion`)

There is no explosive in the game. DEATH-D adds the **authoritative blast resolution** that the dynamite special will later reuse, with one producer: a charge placed by a **fixture** (QA only). It is not placed on the opening route, it is not an ability, and Carl cannot create one.

* `src/blast.rs` (new, Bevy-free): `BlastCharge { x: f32, foot_y: f32, detonate_tick: u64, owner: Option<u32>, detonated: bool }`. The fixture lists `charges: &'static [ChargeSpawn]` with `ChargeSpawn { dx: i32, tick: u64, owner: Option<u32> }` (`dx` relative to `carl_x`, like `shore_dx`; feet on the fixture's walking line). All existing fixtures get `charges: &[]`.
* Order within a tick (appended to the existing order): strike, scene, goblins, contacts, shore, **then charges in index order**. A charge detonates exactly once, on the attempt-local tick `detonate_tick` (exactly that tick: `BlastCharge::detonate` compares with `==`, ticks count from 1 and are never skipped, and a fixture with a charge on tick 0 is refused at load); a restart rebuilds the charges from the fixture.
* On detonation: log `CombatEvent::Blast { tick, charge }`, then evaluate the blast once. Blast centre `(x, foot_y + BLAST_CENTRE_DY)` with `BLAST_CENTRE_DY = 8.0` (the VFX-01 burst centre for a floor blast). `sq = body.squared_distance_to(cx, cy)`: the existing private `Rect::squared_distance_to` (`src/prototype.rs`), made `pub(crate)`; it returns the **squared** distance from the centre to the goblin's body box, 0 inside. Compare squares, never a square root, so the boundary tests are exact. Below, `d` means `sqrt(sq)` for the reader only.
  * `sq == 0.0`: **point blank**, defeated with `Explosion { charge, player: owner, point_blank: true }`, force `Catastrophic`.
  * `0.0 < sq <= BLAST_RADIUS * BLAST_RADIUS` with `BLAST_RADIUS = 20.0`: defeated with `point_blank: false`, force `Heavy`.
  * Beyond: untouched.
  * Living goblins only, index order; a goblin defeated by an earlier charge or by the block on the same tick is skipped (once per target).
* The defeated goblin is thrown: `Goblin::blast(away: i8, point_blank: bool) -> bool` returns false and changes nothing for a goblin that is not alive; otherwise it sets exactly `health = 0`, `mode = Airborne`, `attack_tick = 0`, `stagger_left = 0`, `swing_hit = false`, `launch = None`, `contact_ticks = 0`, `hit_stop = 0`, `vx = away * (point_blank ? 220 : 180)` (clamped to `KNOCK_VX_MAX`) and `vy = point_blank ? 320 : 260` (px/s). `x`, `y`, `facing`, `flight_ticks`, `down_ticks`, the cooldowns and `hit_by` stay unchanged (as in the lethal branch of `take_hit`). `away = sign(goblin.x - charge.x)`, or `-facing` when they are equal. These four numbers are **placeholders for feel review**, named constants in `src/feel.rs` (`BLAST_KNOCK_VX`, `BLAST_KNOCK_VX_POINT_BLANK`, `BLAST_KNOCK_VY`, `BLAST_KNOCK_VY_POINT_BLANK`).
* The QA charge does not hurt Carl or Donut, does not damage terrain, ignores solids (no line-of-sight test) and triggers nothing else. All four are disclosed limits for the dynamite PR (section 8 and [destruction semantics](destruction.md#explosion-semantics)).
* Instigator: `goblin.hit_by`, as for the block.

Why core vs ring: one charge under one goblin bursts it; the neighbour within 20 px dies thrown, intact. The gradient keeps the burst special even when a blast kills several.

### 1.4 Future hooks (documented, not implemented here)

| Hook | Contract it will use | What that PR must add |
| --- | --- | --- |
| Carl's dynamite (ANIM-CARL-03, story-earned EquippedSpecial) | The thrown stick becomes a `BlastCharge` at its landing point with `owner: Some(player)` and its fuse tick; same `Explosion` cause, same core and ring rule, same finishes | Story unlock gate, throw, flight and fuse rules, line of sight against solids, terrain damage (`destruction.md`), friendly-fire rule (ANIM-CARL-03 section 9, question 4), a radius decision if above 26 px (`burst_large`) |
| Enemy or environmental bombs (Bomb Bard, Rot Sticker, authored machinery) | Same `BlastCharge`, `owner: None` | Their own gameplay spec and owner approval |
| Blade hazard (environmental blade or trap) | A new cause `Blade { hazard }`, cause-class ladder `[shear, blunt]`, force `Catastrophic`; shear when the blade line crosses the neck height band of an upright target | A hazard spec and owner approval (none exists) |
| Other falling masses (authored collapse) | The same `neck_probes` edge test as the shore | Their own hazard spec |
| Lying target under an edge | Would need a `crushed_headless` pose | Art work order first (open question 4) |

## 2. Finish selection: extend DEATH-B

Selection stays a pure function of `(species, cause, force)`, derived, stored nowhere, hashed nowhere (`src/death.rs`).

### 2.1 Cause classes and ladders

Two new classes, `Shear` and `Blast`. A cause maps to an **ordered ladder** of classes (replaces `CauseClass::of`; an exhaustive match, so a new cause must decide):

| Cause | Ladder |
| --- | --- |
| `Melee` | `[blunt]` |
| `ShoreBlock` | `[crush]` |
| `ShoreBlockEdge` | `[shear, crush]` |
| `Explosion` | `[blast, blunt]` |

`select` walks the ladder: for each class in order, the first recipe in file order whose `causes` contain the class and whose `min_force <= force` wins. If no class matches, the species fallback, else the generic one. For the two existing causes the ladder has one class, so every DEATH-B result is unchanged (the DEATH-B table test must pass untouched).

`FinishSelection` gains `degraded: bool`: true when the match came from a class after the first (a shear played as a crush, a blast played as a throw). `fallback` keeps its meaning (no recipe matched).

### 2.2 Recipe kinds and validation (schema version 2)

`RecipeKind` gains `decapitated` and `exploded`. `SCHEMA_VERSION` becomes **2**; the file is migrated in the same PR and version 1 is rejected (never half-read). New validation rules, so no data file can make a punch decapitate:

* `decapitated` recipes list only `shear`; `exploded` only `blast`; `crushed` only `crush`; `knockback_down` only `blunt`. (Cross-class fallback is the ladder's job, not the data's.)
* `decapitated` and `exploded` need `min_force` of at least `heavy`.

### 2.3 The profiles (schema 2)

```json
{
  "schema_version": 2,
  "generic_fallback_finish": "generic_collapse",
  "species": [
    {
      "id": "goblin",
      "body_material": "organic",
      "fallback_finish": "goblin_knockback_down",
      "recipes": [
        { "kind": "exploded", "finish": "goblin_exploded", "causes": ["blast"], "min_force": "catastrophic" },
        { "kind": "decapitated", "finish": "goblin_decapitated", "causes": ["shear"], "min_force": "heavy" },
        { "kind": "crushed", "finish": "goblin_crushed", "causes": ["crush"], "min_force": "routine" },
        { "kind": "knockback_down", "finish": "goblin_knockback_down", "causes": ["blunt"], "min_force": "routine" }
      ]
    },
    {
      "id": "llama",
      "body_material": "organic",
      "fallback_finish": "llama_down",
      "recipes": [
        { "kind": "exploded", "finish": "llama_exploded", "causes": ["blast"], "min_force": "catastrophic" },
        { "kind": "decapitated", "finish": "llama_decapitated", "causes": ["shear"], "min_force": "heavy" },
        { "kind": "crushed", "finish": "llama_crushed", "causes": ["crush"], "min_force": "routine" },
        { "kind": "knockback_down", "finish": "llama_down", "causes": ["blunt"], "min_force": "routine" }
      ]
    },
    { "id": "skeleton_mock", "...": "recipes unchanged from schema 1" },
    { "id": "slime_mock", "...": "recipes unchanged from schema 1" }
  ]
}
```

The skeleton and slime mocks keep their schema 1 recipes byte for byte (only the version changes); they deliberately have no shear or blast recipe, so they exercise the ladders.

### 2.4 Selection table (the unit test must pin every row)

| Species | Cause | Force | Finish | kind | degraded | fallback |
| --- | --- | --- | --- | --- | --- | --- |
| goblin | Melee | Routine / Heavy | `goblin_knockback_down` | knockback_down | no | no |
| goblin | ShoreBlock | Catastrophic | `goblin_crushed` | crushed | no | no |
| goblin | ShoreBlockEdge | Catastrophic | `goblin_decapitated` | decapitated | no | no |
| goblin | Explosion, point blank | Catastrophic | `goblin_exploded` | exploded | no | no |
| goblin | Explosion, ring | Heavy | `goblin_knockback_down` | knockback_down | **yes** | no |
| llama | ShoreBlockEdge / Explosion pb / ShoreBlock / Melee | as above | `llama_decapitated` / `llama_exploded` / `llama_crushed` / `llama_down` | as goblin | no | no |
| skeleton_mock | ShoreBlockEdge | Catastrophic | `skeleton_flat_pile` | crushed | yes | no |
| skeleton_mock | Explosion, point blank / ring | Catastrophic / Heavy | `skeleton_scatter` | knockback_down | yes | no |
| slime_mock | ShoreBlockEdge | Catastrophic | `slime_spread` (no head: never a decapitation) | crushed | yes | no |
| slime_mock | Explosion, any | any | `slime_puddle` | fallback | no | yes |
| unknown | any | any | `generic_collapse` | fallback | no | yes |

### 2.5 Species art coverage and variants

* **Goblin A:** has art for all four goblin finish ids (DEATH-C draws three; DEATH-D adds `goblin_exploded`). A test asserts that every finish id the goblin profile can select (each recipe and the fallback) maps to a presenter finish with art, never `NoArt`.
* **Goblin B:** the domain has no variant; species `goblin` covers A and B. GOB-02B shares A's frames, pivots, anchors and part ids, so the same scripts will play B's sheets when B is wired (a separate integration item; DEATH-D does not wire B).
* **Llama:** the profile pins the selection for the first non-goblin real species. Historical note: when DEATH-D merged, no llama existed in the simulation and its ids mapped to `NoArt`. DEMO-01 PR B wired the crush, the slam (owner rule 2026-11-19, `llama_slammed`) and the plain down; **LLAMA-DEATH-WIRE** maps `llama_decapitated` and `llama_exploded` to the LLAMA-02 art (the goblin scripts with the llama's own section 8 numbers and its own explosion tables). Every finish the llama profile can select now has art (tested). No simulation cause shears a llama yet (the block-edge test needs neck probes, which only the goblin has), so a llama under the block is still crushed.
* **A species without decapitation or explosion art** never gets a mismatched head or a silent missing sprite: the ladder falls to the next class with art (shear -> crush, blast -> blunt), else the species fallback, else `generic_collapse`; a finish id with no presenter art draws nothing.

## 3. Event and data contract, replay-hash impact

### 3.1 Changes

| Item | Change | Hash words |
| --- | --- | --- |
| `DefeatCause::ShoreBlockEdge { player: u32 }` | new variant | `EnemyDefeated` mixes `3, tick, target`, then `0x4544_4745` ("EDGE"), `player`, then the existing instigator rule. `force` is not mixed: it is the constant `Catastrophic` (`default_force` and `derived_forces = [Catastrophic]`, as for the block). `implied_instigator` = `None` |
| `DefeatCause::Explosion { charge: u32, player: Option<u32>, point_blank: bool }` | new variant | mixes `3, tick, target`, then `0x424c_5354` ("BLST"), `charge`, `player.map_or(0, p + 1)`, `point_blank as u64`, then the instigator rule. `default_force(Explosion { point_blank, .. })` = `Catastrophic` if `point_blank`, else `Heavy`; `default_force(ShoreBlockEdge)` = `Catastrophic`. `CombatEvent::enemy_defeated` is unchanged: its `_ => cause.default_force()` arm derives both, so the one constructor still derives every tier and `force` is not mixed; `derived_forces` is `[Catastrophic]` or `[Heavy]` by `point_blank` and the `debug_assert` stays. `implied_instigator` = `None` |
| `CombatEvent::Blast { tick, charge }` | new event | `14, tick, charge` |
| `BlastCharge` state | new | mixed into `CombatState::state_hash` after the shore, per charge: `0x4348_5247` ("CHRG"), `x` bits, `foot_y` bits, `detonate_tick`, `owner.map_or(0, p + 1)`, `detonated`. Nothing is mixed when the fixture has no charges |
| `Fixture.charges` | new field, `&[]` for the 12 existing fixtures | none |
| `src/death.rs` | ladders, `Shear`, `Blast`, `Decapitated`, `Exploded`, `degraded`, schema 2 | none (selection is never hashed) |

**Log lines** (fields implied by the cause stay out, as in DEATH-B):

```
COMBAT_EVENT tick=139 kind=enemy_defeated target=0 cause=shore_block_edge credit_player=0
COMBAT_EVENT tick=60 kind=blast charge=0
COMBAT_EVENT tick=60 kind=enemy_defeated target=0 cause=explosion charge=0 point_blank
COMBAT_EVENT tick=60 kind=enemy_defeated target=1 cause=explosion charge=0
```

Exact field order of an explosion defeat: `COMBAT_EVENT tick={t} kind=enemy_defeated target={n} cause=explosion charge={c}[ credit_player={p}][ point_blank]`, then the existing ` force=` rule (never printed here, because `force` always equals `default_force`) and the existing ` instigator=` rule. `credit_player` appears only when the charge has an owner, `point_blank` only when true. The block edge: `... cause=shore_block_edge credit_player={p}`, then the same two rules.

No new `hit_direction` or `impact_position` field: the impact point is derivable from hashed data (the charge id indexes the fixture's charge; the edge is the shore footprint), so the presenter looks it up (section 4). `species` stays out of the event. There is no gore, intensity or finish field anywhere.

### 3.2 Replay-hash impact: none for existing fixtures

* The `Melee` and `ShoreBlock` mixing code is untouched; the new variants and the new event only mix new tags; the charge state is mixed only when a fixture has charges.
* No existing fixture produces a shear: `cp03_shore_kill` (goblin at 484, facing -1, footprint [444, 524)) has both probes inside (480, 488); `cp03_kick_block` (goblin standing at 493.667 at the landing on tick 179, facing -1) has both inside (489.7, 497.7); `cp03_shore_miss` kills nobody. No existing fixture has a charge.
* Therefore **all 12 committed hashes and all 12 committed `headless_*.txt` logs stay byte-identical** (`fixtures_match_the_committed_expected_hashes` and `ci_floor01_combat.sh headless` unchanged). If an implementation changes any of them, that is a defect, not a migration.
* New committed fixtures (section 5.2) add two pinned hashes and two logs.

## 4. Presentation (`src/death_present.rs`, `src/bin/floor01_scene.rs`)

### 4.1 Decapitation: remove the lab flag

* `goblin_decapitated` is now selected by data. Remove `DeathPresenter::lab_finish`, `set_lab_finish`, the `--death-lab-finish` flag and its argument parsing (it becomes an unknown argument). Rewrite the lab-flag tests to feed a real `EnemyDefeated { cause: ShoreBlockEdge { .. } }`.
* The DEATH-C `script_decap` (GOB-02 section 8 decapitation timing, poses 3 -> 4 -> 5, head with `head_dead` swap, jet, wall spatter, trickle, pool stages D+14 / D+40 / D+100) is reused unchanged.
* `away` for `ShoreBlockEdge` = away from the block centre (as `ShoreBlock`), so the body recoils and the head pops out on the side away from the stone. The head part draws on the debris layer, which is above the props layer, so it shows over the landed block.
* **Cause-specific addition (the block pinched the neck), on D:** 8 `drop_blood` from the pose 3 `neck` anchor, low angles (10 to 35 degrees above horizontal), 80 to 160 px/s, away from the block; and one `decal_splat_m` on the walking line on D+2 at the edge that cut the neck: `x = block_x + signum(goblin.x - block_x) * FOOTPRINT_WIDTH / 2` (`block_x` is already in `Observation`). Scaled by the preset rules (drops 8 / 4 / 2 / 0; `decal_splat_m` at presets 3 and 4 only).

### 4.2 Explosive breakup: `Finish::Exploded` (`goblin_exploded`)

`Finish::from_id("goblin_exploded") = Exploded`. Impact point `I` = the charge centre `(charge.x, foot + 8)`, read by `observe` from `CombatState` (add the charges to `Observation`). X = the presenter's first tick of the death (`start`, as for every finish).

**The goblin sprite:** hidden from X at presets 2, 3 and 4 (the pieces replace it). API: `DeathPresenter::hides(target: u32) -> bool`, true for a target whose death is `Exploded` at presets 2 to 4 once the presenter clock reaches the death's `start`, and kept true for the rest of the attempt (a restart clears it). The adapter skips that target's GOB-01 sprite while it is true. `poses()` yields nothing for `Exploded`. At preset 1 the sprite stays and plays the GOB-01 `knockback` -> `corpse` frames, driven by the simulation's own `Airborne` flight (no extra code path).

**Spawn rules (age 0 unless stated):**

* **Body parts:** each part of the preset's list starts at its `frame0_origin` (relative to the goblin pivot at the lethal tick, mirrored with the facing). Velocity: the unit vector from `I` to the part's start point (straight up if closer than 2 px) times 160 to 300 px/s (times 0.7 for `mass: heavy`), plus an up-bias of 60 to 120 px/s on `vy`. Spin from the manifest's per-part `spin` class (GOB-02 section 8, "spin by class"): `fast` 720 to 1080, `slow` 180 to 360, `none` 0 degrees/s, sign seeded. `mass` only scales the launch speed. `head` swaps to `head_dead` on first floor contact (manifest); its roll leaves `decal_smear` (preset 4).
* **Gibs** (`gib_flesh_a`, `gib_flesh_b`, `gib_bone`) from the torso point (`torso_upper`'s `frame0_origin`), elevation 20 to 160 degrees, 140 to 260 px/s. **Rags** (`gib_rag`, class clean) from the same point, elevation 40 to 140 degrees, 80 to 160 px/s.
* **Drops** (`drop_blood`) from the torso point, elevation 10 to 170 degrees, 120 to 280 px/s; landing drops become `decal_splat_s` up to the per-death cap.
* **Age 1:** `decal_splat_m` under `I` (presets 3 and 4).
* **Age 2:** both wall spatters, `decal_wall_spatter` on the left of `I` and `decal_wall_spatter_b` on the right, at `(I.x - 12, foot + 14)` and `(I.x + 12, foot + 14)`, mirrored so each fans away from `I` (preset 4, back wall in reach).
* **Pools:** `Death.pool: Option<(usize, u64)>` becomes `pools: [Option<(usize, u64)>; 2]` (the other finishes use index 0 only), plus `rest_ticks: [Option<u64>; 2]` for the two tracked pieces, `torso_upper` (index 0) and `torso_lower` (index 1). A rest tick is recorded the first time that piece enters `Resting`, and never again. At its rest tick `R`: `decal_pool_1` under the piece's x; at `R + 16`: stage 2; at `R + 40`: stage 3 (the 38 px pair, one entry; preset 4 only). A tracked piece that was not spawned (`torso_lower` below preset 4) or was recycled before resting gets no pool. The crush rule "earlier than the decapitation" applies: the body emptied at once.
* Settled pieces keep the DEATH-C settle (floor clamp on the real outline, lowest eighth turn); the seeded launch makes the pile irregular, never an evenly spaced row.
* `done_after` = 240.

**Counts and budgets per explosion (preset 4 is the owner's default and is pushed beyond the art spec's 8 gibs and 24 drops):**

| | 4 (default) | 3 | 2 | 1 |
| --- | --- | --- | --- | --- |
| Sprite | hidden | hidden | hidden | GOB-01 knockback -> corpse (sim flight) |
| Body parts | 10: `head`, `torso_upper`, `torso_lower`, `arm_near`, `arm_far`, `leg_near`, `leg_far`, `ear`, `club_stick`, `pineapple` (the club splits) | 5: `head`, `torso_upper`, `arm_near`, `leg_far`, `club` | 3: `head`, `torso_upper`, `club` (gore muted) | 0 (the corpse frames carry the weapon; no duplicate club, a ruling against the GOB-02 section 5 "+ club") |
| Gibs | 10: 4 `gib_flesh_a`, 4 `gib_flesh_b`, 2 `gib_bone` | 4: 2 + 2, no bone | 0 | 0 |
| Rags | 2 | 2 | 2 | 2 |
| Drops | 32 | 16 | 8 (muted) | 0 |
| Drop splat cap | 14 | 8 | 4 (`decal_splat_s` only) | 0 |
| Part stain cap | 10 | 6 | 4 (`decal_splat_s` only) | 0 |
| Impact `decal_splat_m` | 1 | 1 | 0 | 0 |
| Pools | 2, to stage 3 | 1 (`torso_upper`), to stage 2 | 0 | 0 |
| Wall spatters | 2 (a and b) | 0 | 0 | 0 |
| Head smear | 1 | 0 | 0 | 0 |
| **Peak launched pieces** | **54** | **27** | **13** | **2** |
| **Peak floor decals** | **28** | **16** | **8** | **0** |

The per-preset rows are **explicit constant tables** in `death_present.rs`, not derived: `EXPLOSION_PARTS` (one id list per preset, exactly the lists above), gib counts per id, and the rag, drop and cap counts. Drops use `scale_count(32, drop_quarters)`. Gibs are scaled per id: `scale_count(4, gib_quarters)` for each flesh id, and `gib_bone` 2 only where the manifest allows it (preset 4). Rags are class `clean` and stay 2 at every preset by table. A unit test asserts that every id listed for a preset is `allowed_at(preset)` in the parts manifest. The `presets` on `main`, checked: `head` 1-4, `torso_upper` 2-4, `club` 1-4, `arm_near` and `leg_far` 3-4; `torso_lower`, `arm_far`, `leg_near`, `ear`, `club_stick`, `pineapple` and `gib_bone` 4 only; `gib_flesh_a` and `gib_flesh_b` 3-4; `gib_rag` 1-4; `drop_blood` 2-4. Preset 3 keeps the art spec's "head, torso_upper, club, 2 limbs" (5 parts), deliberately fewer than the 6 the manifest would allow. Per-death caps `MAX_DROP_SPLATS` and `MAX_PART_STAINS` stay 8 and 6 for the other finishes; the explosion uses the caps of this table.

**Pools are unchanged:** `PART_POOL_CAPACITY` 160, `FLOOR_DECAL_POOL_CAPACITY` 96, `WALL_DECAL_POOL_CAPACITY` 12, `MAX_DEATHS` 24. One preset 4 explosion plus one ring kill (`qa_blast_breakup`) peaks at about 81 launched pieces (54 plus the heavy knockback's 24 drops and 3 gibs) and must recycle nothing. DEATH-E sets the real budgets after profiling. `--death-stress` rotates crush, decapitation and explosion in thirds; a synthetic `Exploded` has no charge and uses `I = (x, ground_y + BLAST_CENTRE_DY)`.

### 4.3 Blast, charge and draw order (adapter)

* **Charge** (QA fixtures only): drawn from attempt tick 0 until its detonation as the ANIM-CARL-03 `dynamite_fizzle` loop (`vfx_dynamite` frames 4 and 5, 4 ticks each), lowest row on the walking line. It is the readable telegraph. Presentation only.
* **Blast:** on `CombatEvent::Blast`, the VFX-01 `burst` tag with its manifest timing, centred on `I`, at **every** preset (it is not gore).
* **Layers:** charge and burst at `actor_z - 0.45`, between the floor decals (-0.5) and the debris (-0.4): under every actor and under the flying pieces, over the props and the landed block. Add the layer to `DrawLayer` / `draw_z` and to its draw-order test.
* **Pure frame selection** (Bevy-free, unit-tested): `charge_frame(tick: u64) -> usize` (frame 4 on ticks 0 to 3 mod 8, frame 5 on 4 to 7) and `blast_vfx_frame(tick: u64, blast_tick: u64) -> Option<usize>` (the `burst` frame from the manifest tick table; `None` before the blast and after its last tick).
* **Art status (open art item):** `vfx_dynamite` and `vfx-01` are both `review_status: prototype_fixture_unapproved`, and the ANIM-CARL-03 and VFX-01 independent reviews are pending. They are drawn in QA fixtures only, which approves nothing, and the evidence labels them "unreviewed fizzle and burst art". In the same PR, change only the two manifests' `vfx.status` text from "not wired into any Bevy binary" to "drawn in the floor01_scene QA fixtures only (DEATH-D); review pending" (no pixel, frame or `review_status` change), so the manifests stop making a false claim.
* Debris and decals keep the DEATH-C layers (debris 0.4 and floor decals 0.5 under the actor layer, wall spatters on `backwall_decal`); nothing is drawn over Carl or Donut. No camera shake is added (DEATH-COMFORT owns the toggle first). No audio: there are no assets, so the "accurate audio" of proposal section F is deferred.
* A ring-killed goblin plays the DEATH-C `goblin_knockback_down` heavy script with `away` from `I`.

### 4.4 Unchanged guarantees

The presenter reads, never writes. Its only randomness is `PresentationRng::for_event(attempt, tick, target)`. The same replay gives the same `DEATH_PRESENT` digest, and every preset gives the same replay hash.

## 5. Test plan

### 5.1 Unit tests (headless, `cargo test`)

* `src/goblin.rs` / `src/shore.rs`: `neck_probes` is `None` for `Launched`, `Downed`, `Airborne`, `Down`, `Corpse`; the edge table at the right edge `R = 524` (facing -1): `x` 519.9 -> crush, 520.0 -> shear, 524.0 -> shear, 527.9 -> shear, 528.0 -> crush; the mirror at the left edge `L = 444` (facing +1): 439.9 -> crush (no overlap or crush), 440.0 -> shear, 447.9 -> shear, 448.0 -> crush; facing away at either edge -> crush; `Hurt` and `Attack` shear like `Neutral`. **Rarity sweep:** a passive goblin facing -1 at every `x` from 420 to 560 in 0.5 px steps under a landing block: the share of block kills that are shears is between 5 % and 12 % (expected 8/104).
* `src/blast.rs`: detonates exactly once on its tick, not before, never again; core (`d == 0`) -> point blank, `0 < d <= 20` -> ring, `d > 20` -> untouched (boundary: a body box exactly 20.0 px away is a ring kill, 20.01 px is untouched; compared as squares); index order; dead targets skipped; two charges on one tick, a goblin in both radii is defeated once by the lower charge index; a block and a charge on one tick, the block wins (it is evaluated first); no effect on the hero's health; restart re-arms the charge.
* `src/combat.rs`: `default_force`, `derived_forces` and `implied_instigator` for the two new causes; `enemy_defeated` derives `Catastrophic` / `Heavy` from `point_blank` and `Catastrophic` for the edge; golden `mix_into` words for one event of each existing variant (pinned literally, proving them unchanged) and for each new variant and `Blast`; the exact log lines of section 3.1.
* `src/death.rs`: the section 2.4 table, every row; the DEATH-B table unchanged; property tests over the built-in profiles: no `Melee` event at any force selects a `decapitated` or `exploded` kind for any species; no species without a `decapitated` recipe ever gets one; two DEATH-B tests change on purpose and say so in a comment: (a) the ids assertion in `the_embedded_profiles_are_valid_and_every_species_has_a_fallback` becomes `["goblin", "llama", "skeleton_mock", "slime_mock"]`; (b) `invalid_profile_files_are_rejected` now rejects version 1 (its `bad_version` replacement turns `2` into `1`); schema 2 validation (version 1 rejected, unknown class or kind rejected, a `decapitated` recipe with `blunt` rejected, an `exploded` recipe with `min_force: routine` rejected, extra field rejected); `degraded` and `fallback` flags.
* `src/combat_scene.rs`: every existing fixture logs zero `ShoreBlockEdge` and zero `Explosion` causes (rare: never from the routine fixtures) and keeps its committed hash; once per target in all 14 fixtures; same replay, same finish and same hash.
* `src/death_present.rs`: the block-edge decapitation through a real event (poses 3 -> 4 -> 5, `head` then `head_dead`, the 8 squeeze drops, the edge splat); the explosion at each preset reproduces the section 4.2 table exactly (part id multiset, gib, rag and drop counts, caps, pools and stages, wall spatters), sprite hidden at 2 to 4 and shown at 1; preset 1 draws no blood sprite and no gore-layer pixel; floor clamp over 420 ticks at every preset; at preset 4, resting pieces are not evenly spaced (the gaps between resting x positions are not all equal) and at least two overlap; `qa_blast_breakup` at preset 4 evicts nothing; the stress mix with explosions stays within every pool capacity; RNG isolation (the presentation seed changes no event and no hash); the goblin selection-to-art coverage test of section 2.5.
* `src/bin/floor01_scene.rs` and `src/death_present.rs`: `--death-lab-finish` is rejected as unknown; the charge and burst layer lies between the floor decals and the debris and below every actor offset of the scene; `charge_frame` and `blast_vfx_frame` over their whole tick range; `hides` is false at preset 1 and for every other finish, and true from `start` at presets 2 to 4 for `Exploded`.

### 5.2 New deterministic fixtures (committed logs, pinned hashes, CI)

| Fixture | Setup | Expected |
| --- | --- | --- |
| `cp03_block_edge_decap` | `cp03_shore_kill` with the goblin moved to `x: 524` (Passive, facing -1: neck on the footprint's right edge, head under the stone); seed `0x0C03_0004`; jabs on 20, 60, 100; 200 ticks | `shore_landed` on 139, then `enemy_defeated target=0 cause=shore_block_edge credit_player=0`; finish `goblin_decapitated` |
| `qa_blast_breakup` | Carl 400 idle; Passive goblins at 520, 548 and 580; one charge `dx: 120` (x 520), tick 60, no owner; seed `0x0D0D_0001`; 240 ticks | `blast charge=0` on 60; goblin 0 point blank (`goblin_exploded`), goblin 1 ring at d = 16 (`goblin_knockback_down`, thrown right), goblin 2 at d = 48 untouched and alive at the end |

Both join `FIXTURES`, the pinned-hash table, `ci_floor01_combat.sh` (headless: twice, diffed against `docs/evidence/cp01-feel/headless_<name>.txt`, presets 1 to 4 replay the same events and hash, the `DEATH_PRESENT` line is deterministic; render: the real loop reproduces the headless events and hash), and `SHA256SUMS`.

### 5.3 Hash stability

* The 12 existing hashes and logs unchanged (CI diff; section 3.2).
* Presets 1, 2, 3 and 4 give identical events and hashes for `cp03_block_edge_decap` and `qa_blast_breakup` (unit test and CI).
* The rarity, selection and presenter tests above do not depend on the preset.

### 5.4 Evidence (real Bevy, xvfb, software Vulkan, as DEATH-C)

In `docs/evidence/death-d/` with a README and `SHA256SUMS`, written by `scripts/make_death_d_evidence.py`:

* Stills at **640x360 (scale 1) and 1920x1080 (scale 3)**, **preset 4 and preset 1**: `cp03_block_edge_decap` on L-2, L, L+4, L+10, L+40, L+120 (L = 139); `qa_blast_breakup` on X-10 (the fizzling charge), X, X+3, X+12, X+40, X+180 (X = 60).
* A side-by-side preset 1 vs 4 sheet at the same ticks for both fixtures.
* GIFs at 960x540 (downscaled from the 1920x1080 frames, one per tick) of both fixtures at preset 4: L-10 to L+120 and X-10 to X+180.
* A frame-interval probe of `--death-stress` (with explosions in the mix) at 1920x1080: p50, p95, max frame interval and peak live parts and decals, next to the DEATH-C probe numbers. A desktop GPU number, not a Steam Deck claim.

## 6. Files the executor changes

`src/combat.rs`, `src/goblin.rs`, `src/shore.rs`, `src/blast.rs` (new), `src/lib.rs`, `src/feel.rs`, `src/combat_scene.rs`, `src/death.rs`, `assets/data/death/species_death_profiles.json`, `src/death_present.rs`, `src/bin/floor01_scene.rs`, `scripts/ci_floor01_combat.sh`, `scripts/make_death_d_evidence.py` (new), `docs/evidence/cp01-feel/headless_cp03_block_edge_decap.txt`, `docs/evidence/cp01-feel/headless_qa_blast_breakup.txt`, `docs/evidence/cp01-feel/SHA256SUMS`, `docs/evidence/death-d/`, `docs/03-tech/species-death-profiles.md` (schema 2, ladders), `docs/03-tech/floor01-scene.md` (DEATH-C flags table: the lab flag removed; a DEATH-D section), `docs/04-production/backlog.md` (the DEATH-D row), `assets/manifests/vfx_dynamite.json` and `assets/manifests/vfx-01.json` (the `vfx.status` text only). No art source, pixel, frame or exported atlas changes.

## 7. Acceptance criteria (the reviewer checks each)

1. A goblin is decapitated by the block only when it is upright, overlaps the footprint and its head probe is inside and its torso probe outside the footprint's half-open x range (probes 4 px forward and back of `x`); every other block kill is a crush. Unit-tested at the section 5.1 boundaries, both edges and both facings.
2. No `Melee` defeat at any force, for any species, selects a decapitation or an explosion (property test over the built-in profiles) and no data file can declare one (schema 2 validation).
3. The rarity sweep puts block-edge decapitations at 5 % to 12 % of block kills; none of the 12 existing fixtures produces one.
4. A blast detonates exactly once on its fixture tick; point blank (`d == 0`) is `Catastrophic` and `goblin_exploded`, `0 < d <= 20` is `Heavy` and the thrown knockback, beyond 20 px nothing happens; each target is defeated at most once per tick across the shore and every charge.
5. The QA charge exists only in fixtures: no route, ability, input or Carl code creates one; the EquippedSpecial still does nothing.
6. `EnemyDefeated` is still the only defeat event, built only by `CombatEvent::enemy_defeated`, which derives every force tier; the `derived_forces` assertion holds for the new causes.
7. The 12 existing fixture hashes and committed logs are byte-identical to `main`.
8. `cp03_block_edge_decap` and `qa_blast_breakup` are committed with pinned hashes and logs, run twice deterministically in CI, and replay the same events and hash at presets 1, 2, 3 and 4 and in the real render loop.
9. The selection table of section 2.4 is pinned row by row, the DEATH-B table passes unchanged (only the two tests named in section 5.1 change), and `degraded` / `fallback` are reported as specified; profiles are schema 2 with the goblin and llama recipes of section 2.3 and the mocks unchanged.
10. Every finish id the goblin profile can select has presenter art.
11. The lab flag (`set_lab_finish`, `--death-lab-finish`) is gone; the decapitation plays from a real `ShoreBlockEdge` event with the DEATH-C script plus the cause-specific squeeze of section 4.1.
12. The explosion reproduces the section 4.2 explicit tables at each preset (every listed id `allowed_at` its preset) (parts, gibs, rags, drops, caps, pools and stages, wall spatters, smear, sprite hidden at 2 to 4 and the GOB-01 knockback frames at 1); preset 1 draws no blood sprite and no gore-layer pixel.
13. Pieces start at their `frame0_origin`, fly radially from the impact point with the stated speeds, manifest spin classes and up-bias, clamp to the floor and, at preset 4, settle irregularly (test); the head swaps to `head_dead`; the two pools grow from the first-rest ticks of `torso_upper` and `torso_lower`.
14. Pool capacities are unchanged; `qa_blast_breakup` at preset 4 evicts nothing; the stress mix with explosions stays bounded.
15. The charge fizzle and the VFX-01 burst are drawn on the `actor_z - 0.45` layer at every preset through the pure frame functions; the exploded goblin's sprite is skipped through `hides`; nothing is drawn over Carl or Donut; no camera shake or audio is added; the two VFX manifests' status text is corrected and their art stays unapproved.
16. No gore, preset, intensity or finish value appears in simulation state, an event, a log line or a hash; the presentation RNG stays event-seeded and isolated (test).
17. Evidence of section 5.4 exists (stills at 640x360 and 1920x1080, presets 4 and 1, the preset sheet, the GIFs, the stress probe) with `SHA256SUMS`, and is real Bevy output, labelled as such.
18. `cargo fmt`, `clippy` (no new warnings) and the full test suite pass; CI is green on the final head.
19. The docs listed in section 6 are updated; the backlog row records "merged, game-feel review pending", not accepted.
20. The data contracts are IP-neutral: no character, franchise or "dynamite" name in a cause, class, kind, event, field, profile or log key (the adapter may load the existing `vfx_dynamite` asset by its id).

## 8. Risks and disclosed limits

* **Feel:** the 8 px window, the 20 px radius and the blast impulses are first values. If the owner finds the decapitation too rare or too easy, only `NECK_PROBE_FORWARD` / `NECK_PROBE_BACK` change (and the rarity bounds with them).
* **Edge art approximation:** `decap_contact` was drawn for a horizontal blade line; the block's edge is vertical. At 1x the pose reads as "head off, body recoils from the stone"; judge it in the capture. A dedicated `edge_contact` key is an art follow-up if it does not.
* **Domain body box:** a lying goblin keeps the standing 24 x 38 box, so a head lying outside that box is never hit. Unchanged from CP-03; out of scope.
* **QA charge limits:** no line of sight, terrain damage, friendly fire or player damage. The dynamite PR must add them before any player-facing explosive.
* **Budgets** are first guesses until DEATH-E profiles the target machine.
* **Audio:** none; there are no assets, so the proposal's "accurate audio" for PR D is deferred.
* **Unreviewed VFX art (open art item):** the charge fizzle (ANIM-CARL-03) and the burst (VFX-01) are drawn before their independent reviews, in QA fixtures only.

## 9. Living horizon and tone

This adds no move, input, unlock or ability. The decapitation is a new outcome of the existing Floor 01 environmental kill (the shore) with the existing verbs, which fits the [character progression horizon](../01-design/character-progression-horizon.md) ("one rewarding environmental kill", "novelty comes from collisions and selective destruction"). The explosion is the death half of the planned story-earned explosive (horizon: early encounters / Hoarder); it is not given to Carl here, and the QA charge is never placed on a route. No horizon update is needed. Tone ([antihero tone](../01-design/antihero-tone.md)): the beheading is a grimy accident Carl causes with three clumsy jabs on a rotten post, darkly comic and not heroic; the burst is the spectacle, not Carl.

## 10. Open owner questions (defaults chosen; nothing blocks)

1. **QA charge as the explosion's only authoritative source until the dynamite special.** Default: yes, fixtures only, never on a route.
2. **Wall impact.** Default: never a decapitation; a wall slam becomes its own kinetic finish later (with the LLAMA-02 slam rule).
3. **Decapitation window.** Default: 8 px per edge, upright goblins facing into the stone (about 8 % of block kills).
4. **A goblin lying under the edge.** Default: crush; a headless-crush pose would be a later art work order.
5. **Audio.** No audio and no assets in this PR; the proposal's "accurate audio" for PR D is deferred to an audio work order.

## 11. Implementation notes (executor)

Choices made where the spec met the code that DEATH-E1 (#106) had changed, or where it was silent; none redesigns the spec. **Demo wiring (after #110 merged):** `blast::Arm` adds `Arm::CarlPast { x, fuse_ticks }` next to the fixed-tick rule (`ChargeSpawn.arm`, `Arm::Tick` for every QA fixture, whose mixed words and hashes are unchanged): the charge lies unlit (a still stick) until Carl's x passes the column, then burns and blows `fuse_ticks` later, exactly; a respawn rebuilds a charge that has not blown. Used by DEMO-01 beat B5 only.

1. **Fixture count.** DEATH-E1 added `cp03_goblin_pileup`, so there are 13 existing fixtures (not 12) and 15 after this PR (not 14). All 13 hashes and logs are byte-identical.
2. **Pool budgets.** Section 4.2 says the pool capacities are unchanged (160 / 96 / 12 / 24); #106 raised them (`DEATH_BUDGET`: 256 / 192 / 32 / 48). The explosion uses the current budget; one explosion plus one ring kill peaks at 81 pieces and recycles nothing at either budget.
3. **Seeds.** `cp03_block_edge_decap` uses the spec's seed `0x0C03_0004`, which `cp03_goblin_pileup` (#106) also uses. The fixtures differ in every other input, so their hashes differ; nothing requires unique seeds.
4. **Rest position.** The first-rest record also keeps the piece's x (the pool is placed under it); the spec named only the tick.
5. **Blast layer.** The charge and the burst sprites are pre-spawned per charge of the fixture (one pair each) and use the `actors` render layer like the debris; the atlas rects come from the exported atlas JSON (`src/blast_art.rs`).
6. **Review fixes (independent review of the first implementation head).** (a) A goblin whose body is hidden by an explosion or retired casts no contact shadow: one rule, `DeathPresenter::draws_goblin`, drives the sprite and the shadow. (b) The fizzling charge is drawn **above the goblins and below Carl** (`actor_z + 0.08`, `DrawLayer::ChargeLit`, fuse frames only; every preset), because under the actors the goblin standing on it hid it; this replaces the "under every actor" part of section 4.3 for the charge. The burst stays on the blast layer (`actor_z - 0.45`) at presets 1 to 3. (c) **Preset 4 only, presentation only (owner intent: more intensity):** the burst is drawn twice as large (integer scale), over the debris (`DrawLayer::BlastFront`, `actor_z - 0.30`, still under every actor) with the existing impact tint for its first 3 ticks; the explosion's two pools are drawn twice as large (integer scale) (their lowest row stays on the floor) and two extra wall spatters at the larger size are added (four in all). These are fields of `EXPLOSION_TABLES` (`pool_scale`, `extra_wall_spatters`, `burst_scale`, `flash_ticks`), stay inside `DEATH_BUDGET` (the peaks of section 4.2 are unchanged except wall spatters 2 -> 4), and change no state, event or hash. (d) Gibs and rags spin by their manifest spin class, like the body parts, instead of a fixed 720. (e) Detonation is exact (`==`), see section 1.3.
