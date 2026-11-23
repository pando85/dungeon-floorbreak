# Species death profiles and the defeat event (DEATH-B)

**Status:** implemented as headless domain code and data; **schema 2 since DEATH-D** (cause-class ladders, the `shear` and `blast` classes, the `decapitated` and `exploded` kinds, the llama profile; see [Schema 2 (DEATH-D)](#schema-2-death-d)). The Bevy renderer draws the finishes through the DEATH-C cosmetic presenter (draft PR stacked on this one; see [DEATH-C in the floor 01 scene doc](floor01-scene.md#death-c-cosmetic-death-presenter-and-gore-pool)), which only reads the selected finish id. Not game-feel reviewed. Design source: the monster death-system proposal (`monster-death-system-proposal.md`, merged to `main` with PR #83), sections A to E and "PR B".

## The authoritative defeat event

`CombatEvent::EnemyDefeated` stays the one defeat event (there is no `MonsterDefeated`). It is emitted exactly once per alive -> dead transition by the simulation (a lethal melee hit in `resolve_strike`, the block landing in `step_shore`); never by particles or audio.

| Field | Meaning |
| --- | --- |
| `tick`, `target` | Attempt-local tick and the domain enemy index (not a Bevy `Entity`). |
| `cause` | `DefeatCause::Melee`, `ShoreBlock { player }` (the player who broke the shore), and since DEATH-D `ShoreBlockEdge { player }` (the block's edge across an upright goblin's neck) and `Explosion { charge, player, point_blank }` (an authored QA blast charge). |
| `force` | `ForceTier`: `Routine`, `Heavy` or `Catastrophic`. A melee defeat is `Heavy` when the lethal move launches (the push kick), else `Routine`; the block and its edge are `Catastrophic`; a point blank blast is `Catastrophic`, a blast ring kill `Heavy`. |
| `instigator` | The player whose action is behind the defeat. Melee: the attacker. Block: the player who last hit the target while it lived (a kick that put it under the block), `None` if nobody did. The lethal event stays the block; the earlier actor is kept. |

There is **no gore, intensity or finish field** and none can reach the simulation: the gore preset (default 4 = full; 1 to 3 reduced and accessibility, owner decision 2026-11-18) is presentation-only. `species` is not in the event either: the caller knows it from the target id, and keeping it out leaves the hash alone.

### Replay hash and log format

No existing golden hash changed. The event hash is unchanged for every earlier fixture (the headless script diffs all of them):

* `force` is **not mixed**: it is a pure function of already-hashed data (the move id of the `Hit` on the same tick, or the constant block tier). Guard: the only constructor is `CombatEvent::enemy_defeated`, which derives the tier (a caller cannot pass one), `DefeatCause::derived_forces` lists, by exhaustive match, the tiers each cause may record, `mix_into` `debug_assert`s it, and a test checks every fixture. **Rule:** a new cause whose force is not derivable from hashed data must mix `force`, as a documented hash migration.
* `instigator` is mixed only when the cause does not imply it (implied: Carl's slot for melee, nobody for a block), so a kick followed by a block crush hashes differently from a block alone.
* `Goblin::hit_by` (the source of a block's instigator) is mixed into the goblin state hash only for a player other than slot 0, so a future second player's hit never hashes like Carl's, while every 1P hash is unchanged.
* The `COMBAT_EVENT` line adds ` force=heavy` and ` instigator=N` only when not implied. The one earlier committed log that changed text is `cp01_kick` (its lethal second kick is now `force=heavy`); its `COMBAT_STATE` hash is the same (`docs/evidence/cp01-feel/headless_cp01_kick.txt` and `SHA256SUMS` updated).
* New committed fixture `cp03_kick_block`: a real kick launches the goblin into the shore footprint and the block lands on it (event ends `cause=shore_block credit_player=0 instigator=0`), final hash `73e409161ce5434f`. It pins the instigator-mixed branch in CI.

### Rulings (independent review of PR #94, owner sign-off pending)

* **Instigator window.** `hit_by` is kept for the whole attempt (cleared only by a restart). That is accepted only while nothing reads it. **Before any consumer reads it** (score, XP, co-op credit, achievements, DEATH-C callouts) it must be bounded: clear it when the target returns to `Neutral` (end of `Hurt` / `Launched` / `Downed`), which is deterministic and needs no tuning constant. Changing the window changes the hashes of chain events and is then a documented migration. A goblin pushed into the footprint by a D-03 contact gets no instigator (contact is not a `Hit`); whether contact chains carry credit is an owner decision.
* **Deferred fields.** `species` stays out of the event for good (it is resolved from the target id, which keeps the event IP-neutral and the hash unchanged). There is no `event_id`: (attempt, tick, target) is unique by the once-per-transition rule, and DEATH-C seeds presentation from `hash(run_seed, attempt, tick, target)` outside the domain. `hit_direction` and `impact_position` are deferred to DEATH-C, to be added only if derived from hashed state (attacker facing, the target body at the defeat tick, "down" for the block) and left unmixed under the same rule as `force`.
* **Rebase.** After #79 merges, rebase this PR onto `main`, retarget the base, keep main's backlog rows and re-insert the DEATH-B row, and rerun CI.

## Death profile data

`assets/data/death/species_death_profiles.json`, embedded with `include_str!`, parsed by `death::DeathProfiles` (`src/death.rs`, Bevy-free). Schema version 1; unknown fields are rejected, so a `gore` field cannot be added by accident.

```json
{
  "schema_version": 1,
  "generic_fallback_finish": "generic_collapse",
  "species": [
    {
      "id": "goblin",
      "body_material": "organic",
      "fallback_finish": "goblin_knockback_down",
      "recipes": [
        { "kind": "crushed", "finish": "goblin_crushed", "causes": ["crush"], "min_force": "routine" }
      ]
    }
  ]
}
```

* `id`: IP-neutral species key; fan-concept names stay replaceable data.
* `body_material`: `organic`, `skeletal`, `gelatinous` or `constructed`.
* `recipes`: tried **in file order, first match wins**, so a species lists rarer, harder finishes first. A recipe matches when its `causes` contain the event's cause class (`blunt` for melee, `crush` for the block) and the event's `force` is at least `min_force`. `kind` is `knockback_down`, `crushed` or `slammed`; `finish` is the id the presentation layer draws.
* `fallback_finish`: played when no recipe matches. An unknown species plays `generic_collapse`. Selection is total: it never fails and never invents a head or other part a species lacks.

Shipped profiles: `goblin` (real, GOB-01), `llama` (real, LLAMA-01 and LLAMA-02-DEATH: `llama_crushed` for the falling block, `llama_slammed` for a lethal `heavy` blunt hit (recipe kind `slammed`, owner decision 2026-11-19: the slam rule) and `llama_down` for any other blunt hit and as the fallback, listed in that order so the first match wins), and two mock fixtures with no artwork, `skeleton_mock` (heavy blunt scatters, crush is a flat pile) and `slime_mock` (crush spreads; no knockback recipe, so a blunt defeat uses its fallback `slime_puddle`).

`DeathProfiles::select(species, cause, force)` and `select_for_event(species, &event)` return a `FinishSelection { finish, kind, fallback }`. They are pure: derived from the event, stored nowhere, hashed nowhere. Finish ids are not asserted to have art yet; DEATH-C maps them to poses and parts.

## Tests

`src/death.rs` (selection table over species, cause and force; no heavy finish for a routine hit; fallbacks; validation rejects bad version, duplicate ids, empty fallback, unknown tier and extra fields) and `src/combat_scene.rs` (kick then block credits the block and keeps the kicker (a unit test and the committed `cp03_kick_block` fixture); a second player's hit hashes differently; the force constructor and allowed set; defeat once per target in every fixture; force derivation; hash words; same replay gives the same finish and hash).

## Schema 2 (DEATH-D)

The profile file is **schema version 2**; version 1 is rejected (never half-read). The rest of this page describes schema 1 and still holds, with these changes. Design: [DEATH-D finishes spec](death-d-finishes-spec.md) sections 2 and 3.

* **Cause classes and ladders.** `CauseClass` gains `shear` and `blast`. A cause maps to an **ordered ladder** of classes (`CauseClass::ladder`, an exhaustive match, so a new cause must decide): `Melee` is `[blunt]`, `ShoreBlock` is `[crush]`, `ShoreBlockEdge` is `[shear, crush]`, `Explosion` is `[blast, blunt]`. `select` walks the ladder: for each class in order, the first recipe in file order whose `causes` contain the class and whose `min_force` is not above the event's force wins; no match falls to the species fallback, then the generic one. For `Melee` and `ShoreBlock` the ladder has one class, so every DEATH-B result is unchanged.
* **`FinishSelection.degraded`** is true when the match came from a class after the first (a shear played as a crush, a blast as a thrown knockback); `fallback` keeps its meaning (no recipe matched).
* **Recipe kinds** gain `decapitated` (only `shear`) and `exploded` (only `blast`). Validation: each kind lists only its own class (`crushed` only `crush`, `knockback_down` only `blunt`), and `decapitated` and `exploded` need `min_force` of at least `heavy`. No data file can make a punch (any `Melee` defeat, at any force) decapitate or explode; the cross-class fallback is the ladder's job.
* **Shipped profiles:** `goblin` and `llama` (a decapitation, an explosion, a crush and a knockback recipe each; the llama is data only until a LLAMA-02 integration item, so its finish ids have no presenter art and draw nothing), and the unchanged mocks `skeleton_mock` and `slime_mock`, which have no shear or blast recipe and so exercise the ladders.

| Species | Cause | Force | Finish | degraded | fallback |
| --- | --- | --- | --- | --- | --- |
| goblin | `Melee` | routine / heavy | `goblin_knockback_down` | no | no |
| goblin | `ShoreBlock` | catastrophic | `goblin_crushed` | no | no |
| goblin | `ShoreBlockEdge` | catastrophic | `goblin_decapitated` | no | no |
| goblin | `Explosion` point blank | catastrophic | `goblin_exploded` | no | no |
| goblin | `Explosion` ring | heavy | `goblin_knockback_down` | yes | no |
| skeleton_mock | `ShoreBlockEdge` | catastrophic | `skeleton_flat_pile` | yes | no |
| skeleton_mock | `Explosion` (either) | catastrophic / heavy | `skeleton_scatter` | yes | no |
| slime_mock | `ShoreBlockEdge` | catastrophic | `slime_spread` (no head: never a decapitation) | yes | no |
| slime_mock | `Explosion` | any | `slime_puddle` | no | yes |
| unknown | any | any | `generic_collapse` | no | yes |

The llama rows equal the goblin rows with `llama_*` ids. `src/death.rs` pins every row of the spec's table, a property test over the built-in profiles (no `Melee` defeat at any force selects a decapitation or an explosion for any species; a species without such a recipe never gets one), and the schema 2 validation rules.

### Hash words of the new causes

`EnemyDefeated` mixes `3, tick, target` and then, per cause: `ShoreBlockEdge` `0x4544_4745` ("EDGE") and the player; `Explosion` `0x424c_5354` ("BLST"), the charge, the owner (0, or player + 1) and `point_blank`. `force` is still not mixed (it is derived: `Catastrophic` for the edge and the point blank blast, `Heavy` for the ring; `derived_forces` and the `debug_assert` cover both). The new event `Blast { tick, charge }` mixes `14, tick, charge`. Charge state is mixed into the state hash only for a fixture that has charges, so all 13 older fixture hashes and logs are unchanged. Log lines: `cause=shore_block_edge credit_player=0` and `cause=explosion charge=0[ credit_player=0][ point_blank]`.

## cf02 overlay (CF02-D, Q21, Q36)

`assets/data/death/species_death_profiles_cf02.json` (embedded, schema version 2, `deny_unknown_fields`) is the **cf02-only** profile overlay of COMBAT-FUN-02. Each entry names a species of the global file and lists `recipes_first`: recipes placed **in front of** that species' own (first match wins), then the merged profiles are validated again with the same rules as a global file. An overlay of another schema version, of an unknown species, with no recipe or with an unknown field is rejected, never half-read. `DeathProfiles::cf02()` is the merged set; the global `DeathProfiles::builtin()` and the pinned tests `selection_table_by_species_cause_and_force` and `selection_table_death_d_every_row` are unchanged.

| Species | Overlay row | Effect |
| --- | --- | --- |
| `goblin` | `{ "kind": "slammed", "finish": "goblin_slammed", "causes": ["blunt"], "min_force": "heavy" }` | a heavy (or catastrophic) blunt defeat is the slammed finish; a routine blunt kill keeps `goblin_knockback_down`; crush, shear and blast rows are unchanged |

**`pile_boss` (FA-Q23, HRD sheet section 10).** The overlay's `add_species` also carries the Hoarder (the global file stays byte-identical): the five recipes and the fallback all select `pile_boss_down`, which `Finish::from_id` maps to `NoArt`. The generic presenter therefore draws nothing for her (no gibs, no dismemberment, no blood: "a ruined person, not a target for spectacle"); her own presenter (`hoarder_art`) draws HOARD-01's `down` frames 13 to 15. She is not CINE-eligible. A test pins every cause and force.

A heavy ring blast (`Explosion { point_blank: false }`, force heavy) has no blast recipe and degrades along its ladder to the blunt class, so under the overlay it also selects `goblin_slammed` (degraded, played as the flung pose). That is the one non-melee row the overlay changes; a test pins it.

**Pose from events (presentation only).** The profile says "slammed"; `death::slam_pose_for(events, target)` picks one of three poses from the events of the defeat's own tick: a `WallSlam` of that goblin gives `WallCrumple`, else a `Hit` of `Stomp` or `DiveStomp` on it gives `Stomped`, else `Flung` (the lethal kick). The pose is stored nowhere authoritative and mixed into no hash.

**Gore baselines (Q22, Q43; `gore::Cf02GoreBudget`, read only when `cf02_rules` and `q36_gore_budget` are on).** At preset 4 a stomped goblin throws 6 gib quarters (5 gibs from the 3-gib set) and never separates a part; a wall crumple 4 quarters (3 gibs) and one arm on the wall side at once; a flung body 6 quarters and one arm as it lands. Presets 1 to 3 scale by the existing preset ratio (`quarters * gib_quarters / 4`, parts rounded up while the preset keeps anything; the manifest's preset filter still decides which part ids fly): preset 3 draws 2 gibs for a stomp (the bone is preset 4 only), presets 1 and 2 draw none and are identical to the legacy kick. Q22 (preset 4 only, any heavy or catastrophic finish): the crush's three gibs and the explosion's ten become five and fifteen (6 quarters, the set repeating), and the decapitation separates one arm. A routine blunt kill (`knockback_down`) at preset 4 adds one **transient** blood spray: six drops that leave no decal or stain and are removed 24 presenter ticks later (`q43_routine_spray_ticks`). Drop quarters and the corpse, decal and pool caps are unchanged.

**Impact hold (Q21).** `DeathPresenter::slammed_poses()` yields a `SlamView { pose, rest, age, .. }`; `rest` is false for the first `q21_slammed_impact_hold_ticks` (8) presenter ticks. CF02-I draws the goblin slammed sheets ([CF02-I acting integration](floor01-scene.md#cf02-i-acting-integration)): the sprite of a slammed finish is the impact frame for the hold and then the rest frame, chosen from `slam_pose_of` and the goblin's own state (a flung body keeps the GOB-01 knockback flight until it lands); without the `cf02_rules` overlay the GOB-01 sprites and only the legacy gore remain.

Wiring: `DeathPresenter::with_cf02(&Cf02Tuning)` in `build_scene` of `floor01_scene` when `CombatState::cf02()` is set (`--cf02` and every `cf02_*` fixture). Without it the presenter reads the global profiles and legacy counts exactly.
