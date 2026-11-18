# Species death profiles and the defeat event (DEATH-B)

**Status:** implemented as headless domain code and data, **not** wired into the Bevy renderer (DEATH-C draws the finishes). Not game-feel reviewed. Design source: the monster death-system proposal (`monster-death-system-proposal.md`, merged to `main` with PR #83), sections A to E and "PR B".

## The authoritative defeat event

`CombatEvent::EnemyDefeated` stays the one defeat event (there is no `MonsterDefeated`). It is emitted exactly once per alive -> dead transition by the simulation (a lethal melee hit in `resolve_strike`, the block landing in `step_shore`); never by particles or audio.

| Field | Meaning |
| --- | --- |
| `tick`, `target` | Attempt-local tick and the domain enemy index (not a Bevy `Entity`). |
| `cause` | `DefeatCause::Melee` or `ShoreBlock { player }` (the player who broke the shore). |
| `force` | `ForceTier`: `Routine`, `Heavy` or `Catastrophic`. A melee defeat is `Heavy` when the lethal move launches (the push kick), else `Routine`; the block is `Catastrophic`. |
| `instigator` | The player whose action is behind the defeat. Melee: the attacker. Block: the player who last hit the target while it lived (a kick that put it under the block), `None` if nobody did. The lethal event stays the block; the earlier actor is kept. |

There is **no gore, intensity or finish field** and none can reach the simulation: the gore preset (default 4 = full; 1 to 3 reduced and accessibility, owner decision 2026-11-18) is presentation-only. `species` is not in the event either: the caller knows it from the target id, and keeping it out leaves the hash alone.

### Replay hash and log format

No golden hash changed. The event hash is unchanged for every committed fixture:

* `force` is **not mixed**: it is a pure function of already-hashed data (the move id of the `Hit` on the same tick, or the constant block tier). A test checks that derivation over every fixture.
* `instigator` is mixed only when the cause does not imply it (implied: Carl's slot for melee, nobody for a block), so a kick followed by a block crush hashes differently from a block alone, and every earlier replay is untouched.
* The `COMBAT_EVENT` line adds ` force=heavy` and ` instigator=N` only when not implied. The one committed log that changed text is `cp01_kick` (its lethal second kick is now `force=heavy`); its `COMBAT_STATE` hash is the same (`docs/evidence/cp01-feel/headless_cp01_kick.txt` and `SHA256SUMS` updated).

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
* `recipes`: tried **in file order, first match wins**, so a species lists rarer, harder finishes first. A recipe matches when its `causes` contain the event's cause class (`blunt` for melee, `crush` for the block) and the event's `force` is at least `min_force`. `kind` is `knockback_down` or `crushed`; `finish` is the id the presentation layer draws.
* `fallback_finish`: played when no recipe matches. An unknown species plays `generic_collapse`. Selection is total: it never fails and never invents a head or other part a species lacks.

Shipped profiles: `goblin` (real, GOB-01), and two mock fixtures with no artwork, `skeleton_mock` (heavy blunt scatters, crush is a flat pile) and `slime_mock` (crush spreads; no knockback recipe, so a blunt defeat uses its fallback `slime_puddle`).

`DeathProfiles::select(species, cause, force)` and `select_for_event(species, &event)` return a `FinishSelection { finish, kind, fallback }`. They are pure: derived from the event, stored nowhere, hashed nowhere. Finish ids are not asserted to have art yet; DEATH-C maps them to poses and parts.

## Tests

`src/death.rs` (selection table over species, cause and force; no heavy finish for a routine hit; fallbacks; validation rejects bad version, duplicate ids, empty fallback, unknown tier and extra fields) and `src/combat_scene.rs` (kick then block credits the block and keeps the kicker; defeat once per target in every fixture; force derivation; hash words; same replay gives the same finish and hash).
