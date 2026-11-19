# Party, local 2P and solo swap: open design questions

**Status: decisions recorded (owner directive, 2026-11-19); game feel pending.** This is the analysis behind HUD-DEBT-04 (WORK-005 and WORK-006) and HUD-DEBT-01, which ship together in one PR after #103. The owner decided D1, D5 and D6 and directed the orchestrator to adopt the analysis' recommendations for Q2, Q3, Q4, Q7, Q8, Q9 and Q11 and the D1 solo safeguards; Q10 swat, pounce and chips are designed in [Q10 Donut moveset](../02-art/donut-moveset-q10.md) (WORK-023, proposed); its scamper and cat-jump rows stay open. See [Decisions](#decisions). Every adopted number is a data knob in `PartyTuning` (`src/party.rs`), labelled with its question, and a tuning hypothesis at the fixed 60 Hz tick: **no human has played it yet, so the game feel is not accepted.** Each question below still lists the options, the recommendation and the test that would settle it.

**Sources:** [MVP controller contract](mvp-controller-contract.md), [gameplay](gameplay.md), [characters](characters.md), [pre-magic Donut experiments](floor-01-pre-magic-donut-experiments.md), [ADR-0005](../05-decisions/ADR-0005-coop.md), [input and camera](../03-tech/input-camera.md), `src/combat.rs`, `src/goblin.rs`, `src/feel.rs`, `src/corridor.rs`, `src/scroll_camera.rs`.

**Current baseline in code:** `HERO_HEALTH` 4. `HERO_INVULNERABLE_TICKS` 55. `HERO_HURT_TICKS` 18. `RESTART_DELAY_TICKS` 50. The hero box is 24×52 px (`HERO_HALF_WIDTH` 12, `HERO_HALF_HEIGHT` 26). A goblin tell lasts 24 ticks and its impact runs on ticks 26..30. The swing box is 8..30 px ahead of the goblin and 0..30 px above its feet. At most 2 goblins attack at once (`MAX_ATTACKERS`). Goblins notice a hero at 220 px. Carl runs at 185 px/s. His jump (440 px/s against 1050 px/s² gravity) peaks at about 92 px. The camera follows horizontally only: lead 24 px, deadzone ±40 px, at most 4 px/tick (240 px/s), with a fixed top edge. Donut currently replays Carl's path 32 px behind him (`DONUT_FOLLOW_PX`). Her sprite canvas is 46×40 px idle and 72×44 px running. Carl's sprite is 52 px tall.

## Decisions

Status of every adopted row: **adopted (orchestrator, owner directive 2026-11-19), game-feel pending.** Values live in `PartyTuning` (`src/party.rs`, `PROVISIONAL_TUNING`) under the label in the first column; change them there. The owner plays the result with two real pads and a solo session before any of it is marked accepted.

| Item | Status | Decision as implemented | Where |
| --- | --- | --- | --- |
| D1 Donut at 0 HP | decided (owner) | Party defeat, the same flow as Carl: 50-tick delay, then both heroes restart at full HP at the checkpoint; control returns to Carl | `combat_scene` (`swing_on`, `defeated_hero`), `party::is_party_defeat` |
| D5 Start joins P2 (D2 below) | decided (owner) | Start on an unowned pad joins P2; on an owned pad it is pause/quit as before | `party::PartyState::pad_start_pressed` |
| D6 pad disconnect (D3 below) | decided (owner) | Any owning pad disconnect pauses until a pad reclaims the slot; no AI handoff. "Continue solo" stays an owner follow-up | `party::PartyState::pad_disconnected` |
| Q2 Donut damage | adopted | Option A as a per-hero data profile: 1 HP per hit, 55 invulnerability ticks, 18 hurt ticks (`donut_damage_per_hit`, `donut_invulnerable_ticks`, `donut_hurt_ticks`). Variant C (3 HP, 75 ticks) is not enabled | `PartyTuning` |
| Q3 Targeting | adopted | Option B: nearest with 24 px hysteresis (`target_switch_px`), the target locked through the whole attack, and in solo the uncontrolled hero counts 40 px farther (`target_ai_bias_px`). The swing then hits whichever hero is in the box, the target first | `combat_scene::update_targets`, `PartyTuning` |
| Q4 Donut hurtbox | adopted | Option C: 20x24 from the feet, centred on the pivot (`DonutHurtbox::Forgiving`); her terrain collider stays the shared 24x52 body. `--debug-combat` draws it | `CombatState::hurtbox`, `PartyTuning` |
| Q7 Camera | adopted | Option A: solo follows the controlled hero; two players follow the midpoint (no look-ahead), the leader is stopped at 448 px (`camera_edge_margin_px` 96, kept at 96 px from the frame edge because the 2P midpoint camera has no deadzone), no zoom. A solo swap pans at 8 px per tick (`swap_pan_px_per_tick`) until caught up. The edge arrow at 384 px (`camera_arrow_px`) is a knob only: **not drawn yet** | `corridor::SceneSim`, `scroll_camera::step_capped_deadzone` |
| Q8 Swap | adopted | Option B: any time except while hurt, a 20-tick cooldown, a 6-tick press buffer, and a swap never cancels the outgoing hero's move | `party::PartyState::request_swap` |
| Q9 Uncontrolled hero | adopted | Option B: the follow-and-jump bot (40 px behind, jumps steps and ledges its own jump reaches, waits at taller walls, dodges a tell within 40 px with a jump, never attacks, regroups after 120 ticks off-screen). The trail replay remains only for scripted scenario and fixture runs | `follower`, `corridor::SceneSim::regroup_bots` |
| Q10 Donut moves | **swat, pounce, chips designed (proposed); scamper and jump open** | Not in PR #114: Donut has no attacks and no ability chips (`donut_ability_chips` false). Design: [Q10 Donut moveset](../02-art/donut-moveset-q10.md) | WORK-023 implementation |
| Q11 Glyphs | adopted (before this PR) | Each plate uses its own pad (agent B adapter); in solo the one pad's set appears on the controlled hero's plate | `pad_slots`, `hud` |
| D1 safeguard M1 | adopted | Solo only: the uncontrolled hero takes hits (flinch, i-frames, HUD) but never drops below 1 HP; the guard follows the AI after a swap | `PartyDrive::is_guarded` |
| D1 safeguard M2 | adopted | The Q3 bias above | `PartyTuning` |
| D1 safeguard M3 | adopted | The Q9 tell dodge above | `follower` |
| D1 safeguard M4 | adopted | Solo only: a guarded hero more than 16 px outside the view cannot be hurt | `combat_scene::swing_on` |
| M5 (2P) | adopted | In two players none of M1 to M4 apply; Donut is human-driven | `PartyDrive::is_guarded` |

## Decided (owner, 2026-11-19)

- **D1:** Donut at 0 HP means party defeat, with the same restart as Carl: 50-tick delay, then both heroes come back at full HP at the checkpoint.
- **D2 (the owner's D5):** P2 joins by pressing Start. Start on a pad that is not yet assigned means join. Start on an assigned pad means pause. If P1 is controlling Donut when P2 joins, P1 goes back to Carl, because the contract gives P1 Carl and P2 Donut. The camera then retargets to the midpoint.
- **D3 (the owner's D6):** Any pad disconnect pauses the game until the pad reconnects. Follow-up for the owner: the pause screen should offer "continue solo" (Donut goes back to AI), or a P2 who never comes back blocks the game.

## How D1 interacts with Q2, Q3 and Q9

D1 makes Donut's survival as important as Carl's. Under the provisional values, solo play is unfair. Donut is AI-driven, replays a trail and never dodges (Q9). Goblins attack the nearest hero (Q3), and Donut trails 32 px behind Carl, so any goblin coming from behind reaches her first. She has the same 4 HP as Carl (Q2) and a box 12 px taller than her sprite (Q4). The player can lose a run to hits they could neither see coming nor prevent. Possible mitigations:

| # | Mitigation | Effect | Cost |
| --- | --- | --- | --- |
| M1 | **Last-HP guard:** a hero under AI control takes hits (flinch, HUD shows the loss) but cannot drop below 1 HP | Only the hero you control can end the run; swapping into a 1-HP Donut is real tension | Low: one check in the damage path |
| M2 | **Targeting bias:** goblins prefer the controlled hero (Q3 option B) | The AI hero is hit less often | Low |
| M3 | **Tell-dodge:** the AI hero jumps when a goblin starts a tell within 40 px of it (Q9 option B) | The AI looks competent; a 24-tick tell leaves enough time | Medium |
| M4 | **Off-screen immunity:** a hero more than 16 px outside the view cannot be damaged | Nobody loses to something they could not see | Low |
| M5 | 2P only: equal HP and a forgiving hurtbox (Q2 A, Q4 C) instead of protection | P2's mistakes stay fair and readable | None |

**Recommendation:** in solo, ship M1 + M2 + M4 with this PR. Add M3 with the follower bot. In 2P, apply none of them (Donut is human-controlled) and rely on M5. A downed/revive state would be a larger, later option and needs gameplay signoff.

## Q2. Donut damage profile (provisional: same as Carl)

| Option | Numbers | Solo / 2P feel | Cost and risk |
| --- | --- | --- | --- |
| **A. Same as Carl** | 4 HP, 55 i-frame ticks, 18 hurt ticks, 1 HP lost per swing | Fair for a new P2. Donut feels no more fragile than Carl | None: already in the code |
| B. Fragile | 3 HP, 55 i-frame ticks, 12 hurt ticks (she recovers faster: agile) | Matches the "fragile cat" identity. In 2P, P2 ends the run more often (D1) | Low, but it is the harshest choice for a couch partner |
| C. Fragile but slippery | 3 HP, 75 i-frame ticks (1.25 s), 12 hurt ticks | Fragility shows without frequent defeats; longer escape window | Low. 75 ticks is longer than the 50-tick goblin attack cycle, which risks turning into a free pass |

Controller readability: the HP bar inner width is 94 px. Four segments are 23.5 px each and three are about 31 px; both read clearly. **Recommendation:** ship A as a data-driven profile per hero. Keep C ready as a playtest variant. Show fragility through reach and damage (Q10) rather than HP, because D1 makes Donut's HP the party's HP. **How to decide:** a 2P session with a non-expert P2 on corridor fixtures A/B/C under A and C, counting defeats caused by each hero. Pick C only if Donut never feels at risk under A.

## Q3. Goblin targeting (provisional: nearest living hero)

| Option | Rule | Feel | Cost and risk |
| --- | --- | --- | --- |
| A. Nearest | Distance between centres, recomputed every tick | Simple. In solo it punishes the AI Donut (see D1) | Low. Risk of flip-flop when both heroes are about equally close |
| **B. Nearest with hysteresis, controlled-hero bias** | Each goblin keeps its target until its attack ends. It switches only if the other hero is at least 24 px closer. In solo, the AI hero counts as 40 px farther away than it is | Goblins visibly commit to one hero. In 2P they spread naturally | Low to medium. Needs a stored target id per goblin, included in the state hash |
| C. Split the tokens | Of the 2 attack tokens, at most 1 goes to each hero | Even pressure in 2P. Prevents two-goblin gangs on Donut | Medium; changes the token logic Carl's fixtures depend on |

**Recommendation:** B. Lock the target during the tell so the tell always points at the hero who will be hit (readable from the goblin's facing). **How to decide:** unit tests show no target change inside a tell and a hysteresis switch at 24 px. Replay the fixtures with the second hero placed 32 px behind Carl. Carl's existing fixture hashes must stay unchanged when Donut is out of range.

## Q4. Donut hurtbox (provisional: Carl's box)

| Option | Size (w×h, from the feet) | Effect | Cost and risk |
| --- | --- | --- | --- |
| A. Carl's box | 24×52 | Reaches 12 px above her 40 px sprite, so attacks that visibly pass over her still hit | None. Unfair as soon as anything hits high |
| B. Fit to the body | 32×24 | Matches the visible torso. The leading edge is 4 px wider than Carl's, so a swing reaches her at 46 px between centres (Carl: 42) | Low. She gets hit at a range where Carl is safe, which works against "agile" |
| **C. Forgiving** | 20×24 | Smaller than the silhouette, so a swing that only clips her tail misses. A swing reaches her at 40 px between centres | Low. The clearance tests and `--debug-combat` boxes must read the box from the hero's profile |

Note: the swing box spans 0..30 px above the feet, so height does not matter against a goblin swing on the ground. Jump clearance is decided by the feet, not by box height. Keep the hurtbox separate from Donut's terrain collider: the collider belongs to traversal and cat-sized gaps, and should be its own value per hero. **Recommendation:** C, centred on the pivot and mirrored with facing. **How to decide:** fixture tests at fixed distances (39/41/45/47 px) for both heroes, plus a `--debug-combat` capture at 640×360 that shows the box inside the sprite.

## Q7. Camera in solo, swap and 2P (provisional: follow the controlled hero, midpoint in 2P)

The 640×360 frame leaves an action lane of 300 px between y 60 and 360 (`ACTION_LANE_TOP`). There is no vertical scroll. Do **not** zoom out: the 640×360 → 1920×1080 ×3 integer scale and the approved 52/40 px sprite sizes rule out zoom. A hero standing at the edge of the frame sees almost none of the 220 px goblin sight range. A margin of 96 px between each hero and the frame edge is about 1.5 s of warning at goblin walk speed (62 px/s).

| Option | 2P separation rule | Feel | Cost and risk |
| --- | --- | --- | --- |
| **A. Midpoint + hard wall** | The camera tracks the midpoint with **no deadzone** (a 40 px deadzone would leave only 56 px at the wall). Maximum separation is 640 − 2×96 = **448 px**; at that point the leader hits an invisible wall at the frame edge. An arrow at the edge appears from 384 px | Classic and predictable. The trailing player decides when to move on | Low. Reuses the speed cap (the deadzone is 0 in 2P); the wall is a clamp in the movement step |
| B. Elastic | Same, but between 320 and 448 px the leader is slowed linearly to 50% of run speed | Softer, but "gummy". Bad for Donut's speed identity | Medium. Changes movement physics, which must stay deterministic |
| C. Follow the leader + regroup | The camera follows the hero furthest forward. A hero more than 32 px off-screen for 120 ticks gets a warning, then teleports next to the leader | Fast. Easy to miss what happens to the trailing player | Medium. The docs only allow teleport "after a warning" and ask to avoid it |

Solo: the camera follows the controlled hero, as now. On a swap, the target moves to the new hero at a temporary cap of 8 px/tick. A worst-case 448 px pan then takes about 56 ticks (0.9 s) instead of 112 ticks (1.9 s). With M4, a hero who is off-screen cannot be hurt. Constraints: Donut's run speed must stay below the 240 px/s camera cap. Her higher jump has to pass the existing `gameplay_stays_below_the_hud_plates` test (y ≥ 60) on both canvases. Authored ledges must sit inside the 300 px lane. **Recommendation:** A for 2P, with swap panning in solo. **How to decide:** DUO-CAMERA-01 fixtures (heroes running in opposite directions, reunion, the wall at 448 px, a swap at maximum separation) captured at 640×360 and 640×400, then a 2P hardware session asking each player whether they ever lost sight of their own hero or a tell.

## Q8. Swap rules (provisional: any time except while hurt)

| Option | Rule | Feel | Cost and risk |
| --- | --- | --- | --- |
| A. Any time except while hurt | Blocked only during the 18 hurt ticks | Most responsive. With M1, swapping away from an incoming hit makes the threatened hero AI-controlled (it still takes the hit but is guarded) | Lowest |
| **B. A + cooldown + buffer** | A, plus a 20-tick cooldown. A press during hurt or cooldown is buffered for 6 ticks (like `CARL_BUFFER_TICKS`). A swap never cancels the outgoing hero's move: the AI finishes it | Readable and hard to spam. A swap pressed during hurt is not lost | Low |
| C. Grounded only, no move active, 30-tick cooldown | Most conservative | Feels sluggish in combat. Fights "instant switching" in the product model | Low |

In 2P, L1 does nothing to the other player's hero (contract). **Recommendation:** B. **How to decide:** replay tests (one press = one swap, the buffered press fires on tick 19 after the hurt starts, no swap inside the cooldown), then a solo hardware session asking whether a swap ever felt eaten or accidental.

## Q9. Uncontrolled-hero behaviour (provisional: trail replay, never attacks)

| Option | Behaviour | Feel | Cost and risk |
| --- | --- | --- | --- |
| A. Trail replay | Copies the controlled hero's foot path 32 px behind, ignoring its own physics | Cheap and deterministic (already in `corridor.rs`). Walks into swings | **High risk:** as AI, Carl would replay Donut's jumps onto Donut-only ledges and break the character gates. Unusable once gates exist |
| **B. Follow-and-jump bot** | Steers toward a point 32–48 px behind the leader using its own run and jump. Jumps only when its own jump reaches the target. Waits at gaps it cannot cross. Dodges with a jump on a tell within 40 px (M3). Never attacks | Credible companion; the gates stay intact | Medium. Stuck states need a regroup rule (off-screen for 120 ticks → placed just off-screen behind the leader) |
| C. B + context attacks | Like B. It also strikes a goblin within reach that is telling at it or staggered, at most once per 60 ticks, and never lands the killing blow | Helpful. Risks stealing kills (the docs forbid stealing every kill) | High. Depends on Q10 moves and needs gameplay signoff |

**Recommendation:** B. Keep A only as a stopgap in the current corridor, which has no gates, and only with M1 + M4. C is a later experiment. **How to decide:** the WORK-006 recording of A versus B on the B7/E1 two-gate layout. A test that AI Carl can never stand on a Donut-only ledge. A count of hits taken by the AI hero per fixture.

## Q10. Donut ability chips and moves as P2 (provisional: none)

**Design (2026-11-19):** the swat, pounce and chips rows are resolved in [Q10 Donut moveset](../02-art/donut-moveset-q10.md) (swat 0 damage with a state-based light guard, pounce 1 damage, dry press at 0 mana). The Move (scamper) and Jump rows stay open. The table below is the original analysis.

Without moves, P2 can only watch, which contradicts the acceptance gate "Donut is useful and enjoyable in co-op from the first scene". Below is a minimal pre-biscuit set within the approved verbs. Enemy HP is 2 and damage is a whole number, so "low damage" in practice means 0 or 1.

| Verb | Candidate | Numbers | Signoff needed |
| --- | --- | --- | --- |
| Move | Faster scamper | 215 px/s (Carl 185), faster reversal; stays below the 240 px/s camera cap | Yes: speed and feel |
| Jump (Cross/A) | Higher cat jump | 500 px/s → about 119 px apex (Carl about 92) | Yes; must also pass the HUD clearance test |
| Primary (Square/X) | **Swat** | Startup 2, active 2, recovery 8; box 12..20 px ahead, 4..16 px up; **damage 0**, stagger 10, push 40 px/s. A swat during a tell interrupts the attack (`take_hit` already does this with damage 0) | Yes: damage 0 versus 1 |
| Physical (Triangle/Y) | **Pounce** | Startup 6, a 40 px leap with a 16 px hop that is active in flight, recovery 16; damage 1, push 60 px/s | Yes: whether it exists and its damage |
| Special (Circle/B) | Locked | Greyed-out chip until the biscuit (HUD-DEBT-02) | No |

The role this gives Donut: she breaks a tell on Carl with the swat, and finishes a goblin with a pounce. That is the synergy the design asks for, and Carl's rules stay untouched. **Flags:** the older "hiss on East" idea conflicts with the contract, because East is the locked special slot, so leave it out. Donut's approved atlas has locomotion only, so swat and pounce frames need an art work order (top-tier design first). Until those frames exist, a greybox flash on the locomotion frames would serve. **Recommendation:** ship swat and pounce behind a profile flag, with the HUD chips enabled only when the move exists. All of it is "merged, game-feel review pending". **How to decide:** a 2P session asking whether P2 had something useful to do in every encounter, plus unit tests for the interrupt and for 0/1 damage.

## Q11. Per-plate glyphs (provisional: each plate uses its own pad)

| Option | Rule | Effect | Cost and risk |
| --- | --- | --- | --- |
| **A. Each plate uses its own pad** | The glyph set is fixed when the pad joins (D2). It changes only on input from that same pad | Correct with mixed pads (DualSense + Xbox) | Low to medium. HUD-DEBT-05 detection becomes per player instead of global |
| B. Global last-used pad (HUD-DEBT-05 today) | One set for both plates | In mixed 2P the plates switch sets every time someone presses a button | None, but wrong |
| C. A, with only the active plate showing chips in solo | In solo the AI hero's plate shows only the L1 swap glyph | Less clutter in solo | Low |

**Recommendation:** A + C. In solo, the one pad's glyph set appears on the active plate. **How to decide:** unit tests with two fake vendor ids (Sony `0x054C` plus one other), then the owner's pending hardware check with a real DualSense and Xbox pad at the same time.

## Horizon fit

Consulted [character progression horizon](character-progression-horizon.md). Everything here is consistent with it: Donut stays a mobile, fragile, cat-shaped partner without magic before the biscuit. Solo swap plus AI and 2P from the first moment are kept, no new buttons are added, Carl is unchanged, and P2 gets real agency instead of watching. The guide needs no update. The swat and pounce numbers are candidates, not approval.

## Summary

| Q | Provisional value | Recommendation | How to decide |
| --- | --- | --- | --- |
| D1 interaction | No protection | Solo: last-HP guard, targeting bias, off-screen immunity; later tell-dodge | Count AI-hero defeats in solo replays |
| Q2 Donut damage | Same as Carl | Keep 4 HP / 55 / 18 as a data profile; test 3 HP / 75 i-frames | 2P session with a non-expert P2 |
| Q3 Targeting | Nearest living hero | Nearest + 24 px hysteresis, target locked through the tell, 40 px controlled-hero bias in solo | Unit tests + unchanged Carl hashes |
| Q4 Hurtbox | Carl's 24×52 | 20×24 at the pivot; terrain collider kept separate | Distance fixtures + debug capture |
| Q7 Camera | Follow / midpoint | Midpoint, wall at 448 px, edge arrow from 384 px, swap pan at 8 px/tick, no zoom | DUO-CAMERA-01 captures + 2P session |
| Q8 Swap | Any time except hurt | Plus a 20-tick cooldown and 6-tick buffer, no move cancel | Replay tests + solo session |
| Q9 Uncontrolled hero | Trail, never attacks | Follow-and-jump bot with tell-dodge; trail only as a gate-free stopgap | WORK-006 A/B recording on B7/E1 |
| Q10 Donut moves | None | Swat (0 dmg, interrupts tells) + pounce (1 dmg); needs art and gameplay signoff | 2P session + interrupt tests |
| Q11 Glyphs | Each plate uses its own pad | Same, fixed on join; solo shows chips on the active plate only | Fake-vendor tests + mixed-pad check |

**Risky enough to change before implementation:** Q9 (trail replay breaks character gates and cannot dodge), the D1 interaction (an unprotected AI Donut makes solo defeats unfair) and Q4 (Carl's box is taller than Donut's sprite).
