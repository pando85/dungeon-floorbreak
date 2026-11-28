# BOS-B: the Ball of Swine's Tusklings, mini-spec and implementation notes

Status: **merged, game-feel review pending** once BOS-B lands. The mini-spec below (top tier, Claude Opus, orchestrator) is adopted under the owner rule "go with all"; nothing here is accepted until the owner plays it. This mini-spec fulfils the BOS-B prerequisite in `docs/07-agents/floor-01-bos-workorder.md` ("a short top-tier mini-spec for the Tuskling numbers from #192"). Sources:
- #192, sections 4, BSW-D, BSW-D2, BSW-11/12/17 and the open questions;
- `docs/02-art/tusk-01-spec.md`;
- `docs/01-design/floor-01-bos-integration.md`.

The numbers below are data, stored in the boss or room data file, each with its ID. They are not constants in code.

| ID | Rule | Value |
|---|---|---|
| BOSB-AD1 | Body | TUSK-01, 30 px idle height, 56x40 frame, pivot (22,39). The sprite is native and never scaled. Tux and gown alternate by spawn index (even = tux, odd = gown). |
| BOSB-AD2 | Qualifying impact | A committed roll that ends in a wall or arch crash (the existing crash/stun event), or the chain weight landing on the Ball (the existing pin event). Ordinary melee hits, bumps and rebounds without a crash never detach. |
| BOSB-AD3 | Detach count | 2 on the **first** qualifying impact (the #192 signature image), then 1 per later qualifying impact. |
| BOSB-AD4 | Caps | At most 3 live at once. The total budget is 5 per attempt (inside #192's 4..6). A detach that would exceed either cap is skipped and does **not** spend budget. The budget resets on room restart. |
| BOSB-AD5 | Legal spawn | Spawn at the Ball's flank on the side away from the nearest hero, on the floor, with at least 24 px clearance from every hero and solid. If no legal spot exists, skip the spawn without spending budget. A spawned Tuskling has a 20-tick scramble: it moves but has no attack and no contact damage. |
| BOSB-AD6 | Health and damage | HP = two of Carl's `punch_a` hits (one kick plus one punch also kills). Contact or attack damage = half the goblin jab damage, rounded up, with a minimum of 1. Uses the ordinary enemy hurt, knockback, defeat and death path (ENEMY-SLOT). No respawn, no reassembly. |
| BOSB-AD7 | Attack | Approach the nearest hero at 1.2x goblin walk speed. Tell = the art's 28 ticks (8 onset + 20 hold), then a 3-tick lunge with 20 px reach, so first contact comes at about 31 ticks (BSW-11). 60-tick cooldown. |
| BOSB-AD8 | Kick-stagger (BSW-D2) | A Tuskling launched by Carl's kick that contacts the Ball through the source-linked launch contact (`LaunchedBody.source`, RINGC-AD7) staggers the Ball for 20 ticks, once per launch. It deals **0** Ball HP damage: enemy-to-enemy HP damage stays 0. If the contact chain cannot be source-linked, it is blocked and recorded; the fight stays winnable. |
| BOSB-AD9 | Ball defeat (BSW-17) | Live Tusklings cower: they stop attacking, play idle/hurt, and are harmless and non-blocking. The exit opens as today. There is no forced cleanup, and the floor-clear pin fires once. |
| BOSB-AD10 | Bot alcove-flee (BOS-Q4 = yes) | The companion bot, when it is in the roll lane at the start of the roll tell, moves to the nearest hero alcove and leaves it after the roll ends (crash or stop). It never camps longer than the roll. |
| BOSB-AD11 | Ledger fact (BOS-Q7 = yes) | `f01_ball_defeated` is written once on the boss defeat event. |
| BOSB-AD12 | Crash self-damage (BOS-Q10 = yes) | An arch or wall crash costs the Ball 3 HP (#192 "gate impact 3..4", lower bound). If the BOS-A F11 clear time leaves its recorded band, lower it to 2 and record the change. |
| BOSB-AD13 | Policies | Add the `carl_environment` policy (Carl baits crashes and uses the weight) and the `duo_coop` policy (one hero on the Tusklings, one on the Ball) as playtest fixtures, the way the BOS-A policies are built. |
| BOSB-AD14 | Audience and death | Tuskling defeats go through the normal enemy-defeat audience path. A `tuskling` species death profile uses the existing TUSK-01 down/hurt frames as the fallback (no new keys). |

Open questions (for game feel): BOSB-Q1 two versus one Tuskling on the first impact (#192 Q); BOSB-Q2 whether the kick-stagger is satisfying or should stay cosmetic; BOSB-Q3 whether 5 is the right budget.

## Where the numbers live

Every number of the table is data in `assets/data/bosses/ball_boss_f01.json` (`BossData`, validated on load, unknown fields rejected). The Juicer's file has none of these keys and behaves, hashes and replays as before.

| ID | Data key |
|---|---|
| BOSB-AD1 | `tusklings.hurt_box`, `tusklings.body_half_width` (the art box); the sprite is `src/tusk_art.rs` (56 x 40, pivot (22, 39), native) |
| BOSB-AD2 | the qualifying events are `BossCrash` and a `RackLanded` that crushed the Ball and left it alive; no number |
| BOSB-AD3 | `tusklings.first_detach` 2, `tusklings.later_detach` 1 |
| BOSB-AD4 | `tusklings.max_live` 3, `tusklings.budget` 5 |
| BOSB-AD5 | `tusklings.clearance_px` 24, `tusklings.scramble_ticks` 20, `tusklings.flank_gap_px`, `spawn_step_px`, `spawn_tries` |
| BOSB-AD6 | `tusklings.hp_punches` 2 (times Carl's jab damage), `tusklings.damage_goblin_jab_divisor` 2 |
| BOSB-AD7 | `tusklings.walk_goblin_mult` 1.2, `tell_onset_ticks` 8, `tell_hold_ticks` 20, `lunge_ticks` 3, `lunge_px`, `reach_px` 20, `strike_y`, `active_ticks`, `recover_ticks`, `cooldown_ticks` 60, `tell_spacing_ticks`, `trigger_margin_px` |
| BOSB-AD8 | `tusklings.kick_stagger_ticks` 20, `tusklings.kick_flight_ticks` |
| BOSB-AD9 | no number (the cower) |
| BOSB-AD10 | `bot_flee.left_in` 110, `bot_flee.right_in` 530, `bot_flee.goal_inset_px` 10 |
| BOSB-AD11 | `ledger_fact` `"f01_ball_defeated"` |
| BOSB-AD12 | `crash.self_damage` 3 |
| BOSB-AD13 | `Pilot::CarlEnvironment` and `Pilot::DuoCoop` (`src/ball_fixtures.rs`), fixtures `ball01_env_clear_solo` and `ball01_duo_coop`, `bos_eval` rows G-0e and G-0d |
| BOSB-AD14 | `assets/data/death/species_death_profiles.json`, species `tuskling`, finish `tuskling_down` (no art of its own) |

## Implementation notes (adopted defaults of BOS-B)

The builder followed the table exactly. Where a rule needed a detail the table does not give, it adopted the smallest one and recorded it here. Each is a data value or a line of code a follow-up PR may change.

| ID | Adopted default | Why |
|---|---|---|
| BOSB-A1 | The Tuskling hurtbox is 32 px tall (`hurt_box.y` 0..32), not the art placeholder's 27. | Carl's jab (`punch_a`) lands 29..35 px above the feet; a 27 px box makes the HP rule ("two `punch_a` hits") impossible. The sprite is 30 px tall, so the box is two pixels generous. |
| BOSB-A2 | The only damage is the lunge strike (a 20 px box ahead of the body, 12..24 px above the feet, one blow per hero and strike). There is no passive contact damage. A jump clears the box. | "Contact or attack damage" is one value; the telegraphed strike is the one hit that can be read and dodged. |
| BOSB-A3 | Spawn candidates are on the flank away from the nearest hero, `flank_gap_px` past the Ball's body edge and then every `spawn_step_px`, `spawn_tries` in all. The clearance also applies against live Tusklings, walls and the hero wall. | Two bodies with 24 px between them are 48 px apart, so the second detach lands on the third candidate. |
| BOSB-A4 | A Tuskling walks through the Ball's body (no collision between enemies). It chases the nearest hero, keeps its target unless the other is 24 px nearer, and never starts a tell less than `tell_spacing_ticks` (12) after any enemy's last tell start. | The art note on staggered tells; the Ball is not a solid for enemies in this framework. |
| BOSB-A5 | The kick link is the launch itself: a push kick (`reaction.launch`) that does not kill puts the Tuskling in `Kicked` for `kick_flight_ticks` with the kick's move instance as the launch id, and the combat state lists the launch ids that already touched the Ball. A slot enemy has no goblin `LaunchedBody`, so RINGC-AD7's goblin contact path is not used. | The recorded fallback is "blocked and recorded". This keeps the rule alive without a new hook in the contacts file. |
| BOSB-A6 | The kick-stagger is a 20-tick freeze of the Ball's clock (its existing hit-stop): whatever it was doing, a walk, a tell, a roll or a stun, stands still and then resumes. Nothing is cancelled, and a longer freeze already running wins. A dormant or defeated Ball ignores it. The contact is consumed either way. | A stagger that cancelled a roll tell would delete the roll; a freeze delays it and keeps it readable. |
| BOSB-A7 | A crash never takes the Ball's last HP: `self_damage` is clamped to leave 1. The cost is the new event `BossSelfHurt`, not `BossHurt` (which stays the heroes' and the weight's damage, so the evaluator's `boss_damage` keeps its meaning). | A defeat always has a hero to credit; a self-kill would have none. |
| BOSB-A8 | The ledger fact is a combat-log event (`LedgerFact`, written once per attempt on the boss' defeat) and a hashed flag in the Tuskling yard. It is not a story-ledger row. | The story ledger holds capabilities; a new row would move `TABLE_HASH`, and the Ball grants nothing. See BOSB-Q4. |
| BOSB-A9 | The follower bot flees only while a roll is told or running, to the alcove on its own side of the ball (it never runs through the body), holds inside it, and is released by the end of the threat. The Juicer's threat carries no refuge, so its bot jumps as before. | AD10 as written. |
| BOSB-A10 | The Tusklings are not offered to the companion's targeting (`companion_target`), so the companion director does not dodge them; the solo bot has no Tuskling dodge. | Lean scope; see BOSB-Q5. |
| BOSB-A11 | The BOS-A fixture tests F4..F9 and F19 run on `ball_bos_a()` (the data without Tusklings, self-damage, flee and fact), so the siege, the bait and the recentre stay proven apart from them. F7b, the policies and the new tests run on the committed data. | Keeps BOS-A regressions meaningful. |

## Open questions

| ID | Question | Default |
|---|---|---|
| BOSB-Q1 | Two versus one Tuskling on the first impact (#192). | 2 |
| BOSB-Q2 | Is the kick-stagger satisfying, or should it stay cosmetic? In natural play it is rare: the Ball walks away from where Tusklings reach Carl. | on, rare |
| BOSB-Q3 | Is 5 the right budget? | 5 |
| BOSB-Q4 | Should the fact reach the story ledger (a capability-free row, which moves `TABLE_HASH`) once something reads it? | combat log only |
| BOSB-Q5 | Should the companion and the solo bot read the Tusklings' tells and dodge them? | no |
| BOSB-Q6 | In solo with the player hiding in the far alcove, the bot, after its flee, follows into the Ball's shove range and is shoved repeatedly (the M1 guard keeps it alive). Does the bot need a shove-keep-clear rule? | no |
| BOSB-Q7 | The crash cost (BOSB-AD12) lets a stationary caster win: `bos_eval` row G-7c (solo Donut casting from the right alcove at 540, the bot as the far-side decoy) now defeats the ball at tick 1350 at a party loss of 3 (the gate's rule is "not a defeat at a loss of 1 or less", so the gate passes). The mini-spec's trigger for lowering the cost to 2 (F11 leaving its 1200..6000 band) did not fire: F11 is 1407. Lower it anyway? | no, 3 |

## Living-horizon review

Compared with `docs/01-design/character-progression-horizon.md`: Tusklings that hunt the heroes while the Ball is baited, a split-role co-op policy (`duo_coop`) and an enemy kicked into the boss are terrain-and-cooperation interactions the guide already lists as directions. The design still fits; the guide is not changed. This is not an approval to implement anything the guide lists beyond the Ball's fight.

## Evidence

`docs/evidence/bos-b/` holds 640 x 360 window stills (each captured twice, byte-identical, with the window and headless state hashes equal) and `SHA256SUMS`. The numbers of the clear (F7, F11, the policies, `bos_eval`) are in the PR body.
