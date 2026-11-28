# GUILD-VERB-01: the Interact verb, the guild box and the guide

**Status:** implemented in the `floor01_scene` probe, on by default in `--f01-run` and `--f01-room act1_guild`; merged status and game feel are the owner's call ("merged, game-feel review pending"). This is probe behavior, not the approved MVP control contract. Rights basis: existing approved art only (ANIM-INTERACT-01 and AREA-ACT1-01), no new art.

Read with: [MVP controller contract](../01-design/mvp-controller-contract.md) (Interact row, R1/RB), [input and camera](input-camera.md), [ANIM-INTERACT-01 spec](../02-art/interact-anim-01-spec.md), [AREA-ACT1-01 spec](../02-art/area-act1-01-spec.md), [ACT1 integration](../01-design/floor-01-act-01-integration.md) section 4.3, [playtest packet](../04-production/f01-playtest-ready-01.md) section 3.

Living-horizon review: Interact is the contract's required verb (devices, props, doors, pickups; never an enemy grab) and adds no move, unlock or terrain interaction. It still fits [character-progression-horizon.md](../01-design/character-progression-horizon.md): Donut keeps only plausible cat verbs and has none above her low class.

## 1. Behavior

| Piece | Rule | Code |
| --- | --- | --- |
| Input | `RawInput::interact`, a **press edge** (a held button never repeats). Keyboard **E**, pad **R1/RB** (`GamepadButton::RightTrigger`). Recorded as bit 128 of the PLAYTRACE action bits (`rawinput-bits/3`; `/1` and `/2` recordings still decode and verify, with Interact released). | `src/input.rs`, `src/pad_slots.rs`, `src/playtrace/codec.rs`, `src/key_bindings.rs` (`INTERACT_KEY`) |
| Core | A pure, Bevy-free fixed-tick state machine: verb tables from the ANIM-INTERACT-01 spec (frame windows, contact ticks), stand-off and reach windows, one verb per target. Tick 0 is the press tick; **the effect lands on the contact tick, never on the press**. A hurt or defeated hero cancels the verb (before the contact tick the effect never happens); hit-stop holds the clock. | `src/interact.rs` |
| Lock | During a verb the hero's input is stripped (no move, jump or attack), so the feet are planted (the art has no root motion). The verb ends on its last tick; the lock is released on the tick after. | `src/combat_scene/interact.rs` |
| Rooms | Only `act1_guild` installs a site (`f01_act1::GUILD_INTERACT`). Every other room has none: a press does nothing and nothing is hashed. Even in the guild the state mixes **nothing** into the state hash until the first accepted press, so every golden stays byte-identical unless the verb is pressed on a target. | `src/f01_act1.rs`, `CombatState::interact` |
| Strongbox | `act1_grant_box` (24 x 28 px at x 340, hand-height latch): Carl facing it inside the reach window plays `interact_hand`; the box opens on the contact tick (clock 5) and stays open. It opens once. It was already drawn open once the guild door's lock is open (`donut_unlocked`); either source shows `act1_grant_box_open`. It grants nothing. | `src/bin/floor01_scene/act1_art.rs` |
| Guide | Mordecai (column x 380): a press within 64 px of his column (either hero) holds his existing `talk` key (frame 4) for 150 ticks. No hero animation and no lock. There is no guild text, by design ("Text: None", ACT1 section 4.3). | `sync_mordecai` in `src/bin/floor01_scene.rs` |
| Target priority | Strongbox first (while it is closed, free, in reach and the hero has a verb for it), then the guide, else nothing. | `Interact::pick` |
| Animation | `src/bin/floor01_scene/interact_art.rs` draws `interact_<verb>` frame `Verb::frame_at(clock)` over the hero's locomotion sprite (hidden meanwhile), facing the target. Presentation only: the window hash equals the headless hash. | `src/bin/floor01_scene/interact_art.rs`, `SpriteClips::load_{carl,donut}_interact` |

Frame mapping (pinned to the manifests' tick contract by a test): Carl `interact_hand` atlas 0 to 3 (31 ticks, contact t5), `interact_floor` 4 to 7 (36, t8), `interact_push` 8 to 11 (34, t6); Donut `interact_paw` 0 to 3 (26, t6), `interact_nudge` 4 to 7 (34, t8). `verb_for(hero, class)` picks the verb by height class: Carl hand, floor or low, waist; Donut low (paw) and floor (nudge), none above.

## 2. Adopted defaults

| ID | Default | Reason |
| --- | --- | --- |
| GUILDV-AD1 | The strongbox only **opens** (state and presentation); the grant stays Donut's guild zone (GUILD-BEAT-01, ACT1 section 7: `primary_ranged` is a read-through grant, the zone is the trigger). Interact on the box does not grant, unlock or open the door. | The docs already define the grant as the zone; no doc gives the box a grant. See GUILDV-Q1 |
| GUILDV-AD2 | Keyboard Interact is **E**. The contract fixes only R1/RB; E is free (J, K, L, Q, Space, the arrows, A, D, S, R, Esc are taken). It has no row in the HUD key-cap table because no prompt is drawn (GUILDV-Q2). | Free key next to the combat cluster |
| GUILDV-AD3 | A press is accepted when the near face of the box is 14 to 32 px ahead of the hero's pivot (the 17 px hand stand-off, minus the 3 px art tolerance, plus 15 px), facing it (a direction pressed with Interact turns the hero first), grounded and able to act. The hero is **snapped** to the exact 17 px stand-off on the press tick. | The art assumes no slide and a 7 px window is 2 ticks at run speed; a snap of at most 15 px keeps the contact pixels on the latch |
| GUILDV-AD4 | The companion AI does not interact: a bot-driven hero's press is ignored. A human-driven Donut can interact (solo swap or P2). | The contract makes Interact a player verb; the director has no interact intent |
| GUILDV-AD5 | The guide's `talk` key is held for **150 ticks** (2.5 s) after an accepted press within 64 px of his column, on top of the existing 48 px proximity key. | SMALL02-AD1 already shows talk by proximity; Interact makes it an explicit, longer beat |
| GUILDV-AD6 | Donut has no verb for the strongbox: its latch is hand height, above her low class (ANIM-INTERACT-01 sections 1 and 4). She can talk to the guide. Her paw and nudge verbs are implemented and mapped but have no target in any room yet. | The art spec says a prop above Donut's low class has no Donut verb |
| GUILDV-AD7 | The `interact` gauntlet overlay (CG-AD5) stays deferred. | Brief |
| GUILDV-AD8 | A hit during the verb cancels it into hurt; before the contact tick the effect does not happen, after it the effect stays (spec section 3 proposal). The press edge is read from the player's raw button, so a button held through a verb does not retrigger. | Spec section 3 |

## 3. Open questions

| ID | Question | Recommendation |
| --- | --- | --- |
| GUILDV-Q1 | Should opening the strongbox grant the biscuit (moving the trigger from Donut's zone to the box)? The docs define the zone as the trigger, and Donut might be far from the box. | Keep the zone; revisit when the owner plays the guild |
| GUILDV-Q2 | A prompt chip (R1/RB glyph, an `E` key cap) when a target is in reach. The HUD has no key-E cap and no interact chip. | A HUD-ART follow-up; the contract asks for a clear prompt |
| GUILDV-Q3 | Getup: Carl's `getup` tag and Donut's melee frames 13 and 14 are still unused. A defeat ends in an instant respawn (demo) or a restart (fixture): no sim recovery phase exists to show them in. A presentation-only getup after a respawn would play while the hero already moves. | Skipped. It needs a short sim respawn lock, which moves every room with a defeat in its goldens |
| GUILDV-Q4 | Push, floor and Donut verbs have no target in any room (waist crates, floor levers, low switches). | Wire them with the first prop that uses them (TROG-Q14 bench smash is a candidate for push) |
| GUILDV-Q5 | `interact` has no `playtrace stats` column and no Python bit name (the BC feature vector is unchanged). | Add when a model consumes it |

## 4. Tests

`src/interact/tests.rs` (verb tables against the export manifests, the height-class matrix, the box opening once on the contact tick, from either side, the turn with the press, the guide's talk, Donut and a bot, no target, hit-cancel and hit-stop, respawn cancel); `src/f01_act1/tests.rs` (the guild site pins, Carl opening the box in the live combat state, the guide, a press changes nothing without a target in the guild, shore and chase, determinism and the hash entering once accepted); `src/combat_anim/tests.rs` (the frame mapping by tag from both frame sources); `src/playtrace/tests.rs` (bit 128, `/1` and `/2` decode); `src/key_bindings.rs` (E is free).

## 5. Evidence

`docs/evidence/guild-verb-01/`: 640 x 360 window stills (Carl mid-animation, the open box, Mordecai talking on Interact), each captured twice byte-identical with the window hash equal to the headless hash, and `SHA256SUMS` (see its `README.md`). Regenerate with `bash scripts/make_guildverb01_evidence.sh`.
