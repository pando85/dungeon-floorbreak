# ENV-FINISH-A: environmental death finishes from existing recipes

Status: implemented in `src/death.rs` (`CauseClass::ladder`, `CauseClass::excluded_kinds`), game-feel review pending.

## Problem

Before this change `CauseClass::ladder` mapped every `Environment` hazard except a falling mass and a blast to `[Blunt]`. A lethal heavy acid, steam, slide or wheel kill therefore played the flung `slammed` finish (boiler, hazards and dozer rooms), which reads as a body thrown by a punch from a giant.

## Adopted defaults (top-tier design, orchestrator Claude Opus)

| ID | Rule |
| --- | --- |
| ENVF-AD1 | `Wheel`: ladder `[Crush, Blunt]`. A vehicle wheel rolls over the body and plays the species' crushed finish where it has one, else the blunt finish. |
| ENVF-AD2 | `Slide`: ladder `[Crush, Blunt]`. A pile slide buries the body. |
| ENVF-AD3 | `Steam` and `Acid` stay `[Blunt]`, but the pure selection never picks a `Slammed` recipe for them (`CauseClass::excluded_kinds`). They play the ordinary knockback and down (a crumple in place), whatever the force. The `EnemyDefeated` event, its force and the audience award tiers are untouched. |
| ENVF-AD4 | No new keys. The `burned` and `dissolved` classes with their own art stay open (ENVF-Q1). |

`FallingMass` stays `[Crush, Blunt]` and `Blast` stays `[Blast, Blunt]`.

## Scope and safety

Finish selection is presentation only: it is read by the Bevy presenters (`death_present`, `combat_scene`, `scene_enemy`) and by tests, never stored in simulation state, mixed into a hash or carried by an event. No golden, eval log, replay hash or `MANIFEST_HASH` changes, and the window hash equals the headless hash.

## Open question

| ID | Question | Size |
| --- | --- | --- |
| ENVF-Q1 | ENV-FINISH-ELEMENTAL: give steam and acid kills their own `burned` and `dissolved` cause classes with dedicated art (needs top-tier keys). | L |

## Tests

`src/death.rs`: `env_finish_a_ladders_for_all_six_hazard_classes`, `heavy_acid_and_steam_never_select_a_slammed_finish` (goblin through the cf02 overlay, llama in the global file), `wheel_and_slide_play_the_crushed_finish_or_fall_back_to_blunt` (the brute boss crushes only at catastrophic force, so it falls to blunt).

## Evidence

No window stills: no existing deterministic pilot or fixture reaches a wheel, slide, acid or steam kill of an enemy, so the tests carry the rule.
