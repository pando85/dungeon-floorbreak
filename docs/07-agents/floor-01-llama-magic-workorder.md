# FLOOR01-LLAMA-MAGIC-01 — Local agent implementation work order

**Task:** Build the actual **post-Guild first Magic Missile + Bad Llama showcase** as a memorable, playable Floor 01 encounter chain, not merely a lab document. The detailed design and 14 scenario criteria are in [the encounter contract](../01-design/floor-01-llama-magic-baptism.md). **This PR begins with design/work-order docs only**; agent completion requires in-engine implementation, tests, evidence and owner playtest verdict.

## Code ownership and preflight

1. Follow `AGENTS.md`, `docs/07-agents/agent-orchestration.md`, current CI/backlog and any actively assigned shared-file slots. Inspect merged implementation and open PRs immediately before working.
2. Audit live `floor01_scene` story/scene entry, `src/combat_scene.rs`, `src/combat_scene/donut.rs`, `src/llama.rs`, the real `LlamaSpawn` placement/target logic, `src/party.rs`, camera operators and existing combat-lab R02. The lab is a geometry/interaction reference, **not an alternate physics engine for story**.
3. Reuse approved `assets/source/characters/llama/` native art, `docs/02-art/llama-01-spec.md`, Donut's existing magic animation/VFX, combat/hurt rules, current Floor 01 catacomb kit and existing sound candidates subject to review.
4. Coordinate with the contemporaneous Donut mobility/reflex, companion autonomy, character-progression and Playtrace work. **Do not implement or rebalance their mechanics here.** In particular, confirm how a llama selects a target in solo Donut and 2P before claiming those modes are supported. If targeting is a separate team's responsibility, create a narrow follow-up issue/PR and keep this scenario's blocked cases explicit.
5. Respect the story sequence: only **after** the Guild unlock (8 mana; Magic Missile on Primary), **before** Carl's dynamite, and before Donut's late special; do not leak lab-only power overrides into story/demo.

## Deliverables

| Phase | What to implement | Must demonstrate |
| --- | --- | --- |
| **MAG-A** | Level/scene data: 3 connected combat pockets M0/M1/M2 + protected restaurant approach M3; clear dry ground and optional 16/32px rise | Real Bevy level entry; deterministic geometry; reachable exit for solo & 2P |
| **MAG-B** | First cast in first seconds after Guild; two normal goblins with distinct activation windows, Donut mana and Carl normal melee intact | MAG-01..03, positive magic and direct Carl clear, native footage |
| **MAG-C** | One Bad Llama with actual windup/projectile, masonry refuge and no hidden targeting/height hacks | MAG-04..07; both Carl and Donut can fight; ground/+32/+64 collision matrix |
| **MAG-D** | Two-llama "Throat Choir" with staggered *individual* attack states and player-driven counterplay | MAG-08; telegraphs/active glob always visible; no forced synchronized script |
| **MAG-E** | Optional one vulnerable screen using **already supported** material/impact interface; restaurant threshold/audio/lighting clarity | MAG-09/14; direct-win baseline unchanged, no new physics subsystem |
| **MAG-F** | Retry, real 1P/2P runs, Playtrace replay/hashes, independent review, no-regression and perf comparison | MAG-10..14, actual at-speed evidence and owner playtest decision explicitly pending until performed |

If story bridge is not yet built, a **properly flagged post-Guild fixture/test entry** is acceptable to validate M0..M3; **do not claim Stage 03 is integrated into a campaign** until the route, unlock persistence and restaurant transition are actually wired.

## Specific engineering constraints

- Keep the exact **existing** 8-charge Donut mana and regen contract, missile release on local tick 4, straight horizontal range ~240px, 1 damage, dry press at 0, Pounce unchanged. Do not add auto aim, aim assist, free ammo or a second magic skill.
- Llama native art is a **64px** enemy with a bright throat glow. Existing active runtime may differ from concept's 30-tick tell and 42–48px-high glob; inspect/test live values instead of silently replacing old tests. Actual projectile/cover collision decides outcomes.
- **Height is real:** tests must prove whether ground Donut's 20×24 hurtbox evades high spit, whether +32px ledge exposes her, and whether Magic Missile fired from +32/+64 can really hit a ground llama. If it misses, design a reposition/drop instead of faking line of fire.
- Actual llama AI may aim at **Carl's x only** in the legacy scene. Audit all party modes; if active Donut does not attract hostility, do not report solo Donut balance as passed. Coordinate a tightly scoped target-selection fix and regression with whoever owns `src/llama.rs` / companion.
- The player can pass or kill with **ordinary attacks** without shattering the optional fragile screen. The screen must genuinely block both enemy glob and player missiles until its actual collision material changes; if not supported, leave it out of initial implementation and flag MAG-E gated.
- Default baseline uses **2 goblins + 3 llamas across the entire chain**, not all at once; first two llama firing patterns should be **separated by sightline, distance and individual enemy state**. No invisible projectile spam, no new enemy archetypes or unapproved enemy alliance.
- Avoid mandatory story progress via a Donut-only damage gate; Carl can reach and defeat llamas. Preserve original solo swap, 2P input ownership, actual companion behavior and camera policy.
- Donut reflex/dodge mechanic is authored by the movement agent; do **not** reimplement its 12s timer, hit tags or VFX here. Test with evasion isolated/off to verify fair baseline and then with merged rules on.
- No late-story abilities, no body-to-body bonus damage invented to facilitate a magic combo, no new full-screen flames/persistent fire puddles, no new HUD complexity and no automatic cinematic slow motion for ordinary hits.
- Gameplay deterministic 60Hz fixed ticks; events include projectile spawn/hit, source attribution, cast/mana, damage, optional screen state and checkpoint. Presentation (orange throat / magenta VFX / audio / light) cannot affect hashes/physics.
- Source art pixels, animation manifests and sprites must stay in approved editable source/atlas pipeline with provenance and native 1x validation. No AI-generated substitute masters or third-party game screenshots as assets.

## Acceptance evidence checklist

- **MAG-01..14** all checked with true actual commands and concrete pass/fail/blocked status.
- A real recorded **first Magic Missile kill**, **first solo llama exchange**, **two-llama spell+melee battle** and **restaurant exit**. Capture **at speed**, not only frozen concept art.
- One readable comparison of Donut on ground vs 32px perch vs 64px perch: missile/glob trajectories with optional hitbox debug overlay, showing **actual results** even if they disprove original staging idea.
- Scripted deterministic/headless fixtures that reproduce each meaningful state and hash twice. A test that broken/unused cover cannot block the critical route.
- Start with human Carl active, human Donut active and actual local P1+P2. No invisible aggression exclusively aimed at Carl in Donut-controlled solo run; document any remaining targeting issue honestly.
- Retry and exit handling: mana flags persist as story specifies; combat state resets exactly and promptly without recreating biscuit cutscene.
- Native 640×360, 640×400 and 1080p output captures; screenshots at the **dual glow** and initial magic hit, plus short footage demonstrating correct camera framing, telegraphs and reduced-VFX viability.
- Perf delta vs current demo; no regression to other goblin/llama fixtures and mana/camera replay goldens; reviewed PR head and green CI.
- Owner playtest/feel review explicit. Automated win rate, ML training traces or example GIF alone do **not** certify that the first magic feels good.

## Definition of done

A human can play **Guild exit → first magic goblin(s) → one Bad Llama → two-llama encounter → restaurant threshold**, in both player modes when available. The player sees readable orange vs magenta attacks; ordinary melee and magical solutions both work; Donut's first cast and the alternating dual-llama fight are actually memorable; full attempts are recorded and faithfully replayable. Every remaining unsupported piece is called out instead of being claimed implemented.

**Do not expand into Floor 01 Stage 04–06, Goblin Boiler #176, Dozer opening #179, Hoarder/hazards #178 or companion nav.** Those are separate review and implementation tracks.
