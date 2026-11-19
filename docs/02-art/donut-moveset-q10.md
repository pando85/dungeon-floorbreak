# Q10: Donut's swat and pounce, gameplay and animation design

**Status:** design proposed under the owner's design-defaults directive (AGENTS.md, 2026-11-19). Every value is the recommended default and lives as data with a `Q10-##` ID. It is adopted after an independent review. Nothing is implemented yet, and game feel waits on the owner's 2P and solo play. This unblocks HUD-DEBT-02 (mana) and Donut's HUD chips.

**Read with:** [2P open questions Q10](../01-design/party-2p-open-questions.md#q10-donut-ability-chips-and-moves-as-p2-provisional-none), [controller contract](../01-design/mvp-controller-contract.md#character-actions-and-progression), [mana v0](../01-design/donut-mana-v0-proposal.md), [ANIM-DONUT-02](donut-melee-anim-02-spec.md), [ANIM-DONUT-03](donut-magic-missile-anim-03-spec.md), [hybrid HUD §2.4](hybrid-hud-a-native-spec.md#24-ability-chips-what-shows-per-state), `src/combat.rs`, `src/goblin.rs`, `src/feel.rs`, `src/party.rs`. Diagram: [q10_hitboxes_timing_x4.png](../evidence/donut-moveset-q10/q10_hitboxes_timing_x4.png), made by `scripts/make_donut_moveset_q10_diagram.py`.

**Key finding:** the Q10 analysis assumed Donut's atlas had locomotion only. It does not. ANIM-DONUT-02 already holds a drawn swat (4 frames) and pounce (4 frames). They come from eight frozen keys that passed a silhouette gate and a blind review. The frame data below follows that art's tick contract (manifest `melee.moves`), not the analysis' 2/2/8 placeholder, so **no frame is redrawn or retimed**.

## 1. Moves

All timings are 60 Hz ticks, and tick 0 is the tick a press is accepted. Boxes are pivot-relative (x forward, y up from the feet) and half-open. Donut's hurtbox is 20×24, `[-10, 10) x [0, 24)` (Q4). A goblin's body is 24×38 and its swing box is `[8, 30) x [0, 30)` ahead of it.

**Swat (Primary, pre-biscuit).** It breaks a tell and buys space, but never kills.

- **Frames:** startup 4, active 3 (t4-6, hit tick 4), recovery 7. Total **14**, the same as Carl's jab.
- **Box:** `[13, 24) x [5, 11)` (shins). It reaches a goblin centre closer than **36 px**. The goblin's swing reaches Donut from **40 px**, so the swat never out-ranges what it interrupts.
- **Hit:** damage **0**, hit-stop 2, stagger 10, push 60 px/s (about 1.5 px), no camera kick. Like any hit, it cancels a goblin's tell or recovery: the token goes back and `HURT_COOLDOWN` (20) runs. It staggers a neutral goblin.
- **Light guard (anti-lock):** a swat stagger makes the goblin wary for **60 ticks**. Further swats during that window **glance**: no stagger, no hit-stop, no state change, a 1-tick flash and a `Glance` event. A swat also glances on:
  - a committed swing (smear or impact),
  - a launched, downed or airborne goblin, so it never shortens Carl's knockdown,
  - every boss state.

  Carl's hits and the pounce still interrupt during the guard. Why: stagger 10, plus the 20-tick cooldown, plus the 24-tick tell would let unguarded zero-damage swats cancel every attack forever. With the guard, one swat cancels one attack. The next one (tell at about +33, impact at about +59) needs Carl or a pounce.
- **Movement:**
  - Grounded only.
  - `vx` is capped at 60 px/s, not zeroed, so the swat stays movement-compatible (the ANIM-DONUT-02 gate decision).
  - Facing is fixed and jump is off on t0-6.
  - A jump on t7-13 cancels the recovery. This is her agility; Carl has no such cancel.
  - A second press on t4-13 re-swats at t10.

**Pounce (Physical, before and after the biscuit).** It relocates her about 40 px and finishes weak goblins. Its risk is the whiff.

- **Frames:**
  - Crouch t0-9 (the tell).
  - Launch at t10 with vx 180 and vy 130 px/s. Under the sim's semi-implicit gravity (1050) the flight lasts 14 ticks, covers about 42 px and peaks at 7 px. It touches down at t24 on flat floor.
  - Active from launch to touchdown + 1 (pin t24-25).
  - Recovery 18 (t26-43). Total **44**, the art's nominal value.
- **Box:** `[11, 25) x [0, 17)` (flight and pin paws). It reaches a goblin centre closer than 37 px, so a launch from about 80 px or closer connects.
- **Hit:** damage **1**, hit-stop 4, stagger 14, no push. At most one hit per instance across flight and pin. On a hit `vx` goes to 0, and she drops into the pin at the goblin's feet. A kill is `DefeatCause::Melee` credited to hero index 1. Only that event's hash grows (non-implied instigator), and the force is Routine.
- **Bounds (no ledge exploits):**
  - Grounded start only.
  - No jump cancel; a pounce never refunds or spends a jump.
  - Walls and body contact (centre distance 24 px with a living goblin) stop it, so she never passes through or over a goblin.
  - The 7 px apex mounts nothing a walk cannot, so the boss rack ledge (104 px) stays a cat-jump gate.
  - **Air cap 20 ticks:** still airborne 20 ticks after launch (off a ledge), the pounce lapses. The box turns off, and she falls on the locomotion `jump_fall` and lands with `land`, no pin.
- **Commitment:** input and facing are locked until touchdown + 8. After that, a move or jump cancels the rest of the recovery. A whiff exposes her for 32 ticks, more than a goblin's 26 ticks from tell to impact.

**Shared rules.**

- Any hit on Donut cancels either move. Neither move has armour.
- A press while airborne or busy is buffered for 6 ticks (Carl's value), then dropped. A swap clears the buffer.

**Boss (PR #115, unmerged).** The moves fit #115 unchanged.

- A swat deals 0 HP and 1 poise, as #115 lists, and never breaks super armour.
- A pounce deals 1 HP. It is safe in recovers, stuns and pins, and gets punished in a pound tell.
- The charge is still answered by a jump.

## 2. Beside Carl, 2P and solo swap

- **Separate hit heights:** Carl's jab and haymaker hit 27-38 px up and his kick 18-28. Donut's swat hits 5-11 and her pounce 0-17. Where the flash appears shows who hit.
- **Roles:** Carl deals damage and launches. Donut interrupts and finishes. Typical plays:
  - A goblin tells at Carl, Donut swats, and Carl's jab and haymaker kill.
  - Carl jabs, and Donut's pounce finishes.

  Playing solo as Donut works (two pounces per goblin) but is slow on purpose: swap to Carl.
- **Same tick:** strikes resolve in the order Carl, Donut, goblins. A goblin frozen by Carl's hit-stop cannot be hit, so Donut's box retries on its later active ticks. The outcome is deterministic.
- **Swap (Q8):** a swap never cancels a move. The AI finishes the swat or pounce and starts none. **The follower never attacks** (Q9 B). Option C stays a later experiment.

## 3. Primary mapping and the 0-mana rule

| Donut state | Primary (Square/X) | Physical (Triangle/Y) |
| --- | --- | --- |
| Pre-biscuit | Swat | Pounce |
| Post-biscuit, mana ≥ 1 | Magic Missile (1 charge, 1 damage, interrupts like any hit) | Pounce |
| Post-biscuit, mana 0 | **Dry press (final)** | Pounce |

**Decision: dry press.** The mana v0 interim rule becomes final:

- no cast and no swat,
- no action lock and no buffer,
- a `CastRejected{Dry}` event, a muted `MISSILE` chip and an empty click.

It adds a presentation-only **dry cue**: 8 ticks of the ANIM-DONUT-03 gather head on the current locomotion frame, with no bolt. P2 watches the cat, not the plate. The cue is never hashed and never blocks input.

Why not the swat fallback:

- The last press of a missile burst would become a committed swat with capped `vx`, rooting a kiting caster.
- The muted chip would lie about what the button does.
- The pounce already prevents a soft-lock.

After the biscuit the swat is unreachable, and the missile takes over its interrupt role. `empty_press: "swat"` becomes a valid playtest-only value. With it, the 0-mana chip shows an unmuted `SWAT`, so the chip stays honest.

## 4. HUD chips (§2.4, no layout change)

A player-controlled Donut plate shows:

| Donut state | Chips |
| --- | --- |
| Pre-biscuit | `SWAT` / `POUNCE` |
| Post-biscuit | `MISSILE` / `POUNCE` |
| Mana 0 | muted `MISSILE` / `POUNCE` |

The plate uses its own pad's glyphs (Q11). The AI plate shows `COMPANION` and never shows glyphs.

**Gating:** chips show only when `donut_moves` (Q10-01) is on and `melee_live` is true (the scene's real melee gate). `PartyTuning::donut_ability_chips` is derived from Q10-01, and a test pins the two equal. `POUNCE` is never muted. Chips show no cooldown or guard state.

## 5. Data knobs

| ID | Knob | Value | Home |
| --- | --- | --- | --- |
| Q10-01 | `PartyTuning::donut_moves` | `true` (`false` = today's attack-less Donut) | `party.rs` |
| Q10-02 | `DONUT_SWAT` startup / active / recovery | 4 / 3 / 7 | `combat.rs` (`MoveId::Swat`, appended) |
| Q10-03 | swat box | `[13, 24) x [5, 11)` | `combat.rs` |
| Q10-04 | swat damage | 0 | `combat.rs` |
| Q10-05 | `SWAT_FEEL` | hit-stop 2, stagger 10, knock 60 / 0, kick 0, `light: true` | `feel.rs` |
| Q10-06 | `LIGHT_GUARD_TICKS` | 60 | `feel.rs` |
| Q10-07 | swat chain | press t4..=13, re-swat at t10 | `combat.rs` (`Chain`) |
| Q10-08 | `swat_vx_cap_px_s` | 60 | `party.rs` |
| Q10-09 | `swat_jump_cancel_from` | t7 | `party.rs` |
| Q10-10 | `DONUT_POUNCE` startup; `pounce_launch_vx` / `_vy` | 10; 180 / 130 | `combat.rs`, `party.rs` |
| Q10-11 | pounce box; active window | `[11, 25) x [0, 17)`; launch to touchdown + 1 | `combat.rs` |
| Q10-12 | `pounce_air_max_ticks` | 20 | `party.rs` |
| Q10-13 | pounce recovery; `pounce_cancel_after_touchdown` | 18; 8 | `combat.rs`, `party.rs` |
| Q10-14 | pounce damage; `POUNCE_FEEL` | 1; hit-stop 4, stagger 14, knock 0 | `combat.rs`, `feel.rs` |
| Q10-15 | `donut_buffer_ticks` | 6 | `party.rs` |
| Q10-16 | `empty_press` | `"dry"` (`"swat"` for playtests only) | `companion_magic_v0.json` |
| Q10-17 | `dry_cue_ticks` | 8 (presentation only) | `anim` / `actor_sprite` |

New `MoveId`s are appended after `PushKick`, so existing discriminants stay put. The pounce's fixed `MoveSpec` (10 / 16 / 18) is the flat-floor nominal. The scene ends the flight on touchdown, under the Q10-12 cap.

## 6. Determinism and tests

**Legacy goldens:** they must stay bit-identical with Q10-01 both on and off. That covers the `combat_scene` hash table (including `cp01_interrupt`), the corridor and party goldens, and every fixture's final hash.

- The goblin guard is mixed into the state hash only when it is non-zero.
- `Glance` events and Donut `Hit` events come only from Donut moves.
- A fixture that scripts Donut Primary or Physical presses pins `donut_moves: false` and says so.

**Unit tests (Bevy-free):**

- **Swat range and damage:** it hits only on t4-6 and its damage never defeats. It hits a goblin at centre distance 35 and misses at 36; the goblin swing hits Donut at 39 and misses at 40.
- **Swat interrupts and glances:** it interrupts a tell or a recovery and returns the token. It glances on a smear or impact, on a second swat within 60 ticks, and on a downed goblin, whose down timer stays unchanged. A jab during the guard still interrupts, and the guard ends on tick 60.
- **Swat movement:** the re-swat fires at t10. A jump cancels from t7 but not on t6. `vx` stays at or below 60 px/s.
- **Pounce flight:** launch t10, flat touchdown t24, 42 ± 1 px of travel, apex at most 7 px.
- **Pounce hit:** `vx` is 0 after a hit. It lands one hit per instance, deals 1 damage, and credits a kill to index 1.
- **Pounce bounds:** it stops at walls and at 24 px body contact. Off a ledge it lapses at 20 air ticks (box off, `land`, no pin). It never starts airborne and has no jump cancel.
- **Shared rules:** hurt cancels both moves. The buffer holds for 6 ticks and the press drops on tick 7. A swap clears the buffer, and the AI finishes the running move and starts none.
- **Mapping:** pre-biscuit Primary swats, post-biscuit it casts, and at 0 mana it is a dry press with no lock and no buffer. Physical is always the pounce.
- **Chips:** they follow the section 4 table.

**New fixtures** (headless and in the render loop, final hash committed, replayed twice):

- `q10_swat_interrupt`: 2P; a goblin tells at Carl, P2 swats, Carl kills.
- `q10_swat_guard`: a second swat within 60 ticks glances, and Carl gets hit.
- `q10_pounce_finish`: Carl jabs, then Donut's pounce kills.
- `q10_pounce_whiff`: a short pounce into a tell gets punished.
- `q10_pounce_ledge`: a pounce off the corridor step lapses.
- `q10_solo_swap_donut`: a swap mid-pounce; the AI finishes the move.
- `q10_dry_press`: lands with HUD-DEBT-02.

**Evidence:** real Bevy captures at 640×360, 1× and 3×:

- the swat and the pounce on a goblin, next to Carl's jab;
- `--debug-combat` boxes compared with the diagram;
- 2P and solo-swap GIFs;
- the three chip states;
- a frame-time note.

## 7. Animation plan and routing

| Clip | Frames | Ticks | Source |
| --- | --- | --- | --- |
| `swat` (re-swat jumps from s2 back to s0 at t10) | s0 key, s1 key, s2, s3 (melee 0-3) | 4 / 3 / 4 / 3 | ANIM-DONUT-02 |
| `pounce`, `pounce_air`, `pounce_land` | p0 key, p1 key (held to touchdown), p2 key (2 active), p3 (cancellable) (melee 4-7) | 10 / 14 / 8 / 12 | ANIM-DONUT-02 |
| pounce lapse | `jump_fall`, `land` (loco 13-15) | state-driven | ANIM-DONUT-01 |
| hurt, knockback, knockdown, getup | melee 8-14 | as drawn | ANIM-DONUT-02 |
| dry cue | `cast_gather` head on loco frame n | 8 | ANIM-DONUT-03 rule |

**Routing:** score 1 (quadruped anatomy), so the class is Easy and route C asks for one design-stage hero key. The set already has eight frozen design-stage keys, and they cover every pose Q10 uses. **No new key is required or produced, and no silhouette gate applies:** this is not a new design, and the keys passed their own gate.

**Execution chores:**

- add the chain, the air cap and the p3 cancel to `melee.moves`;
- point ANIM-DONUT-02 §8 questions 2-4 to this spec;
- wire the frames into the Floor 1 actor.

## 8. Art acceptance targets (review of the integration evidence)

1. At 1× on lit Floor 1, the swat paw meets the goblin's shins on the hit tick. Its flash sits clearly below Carl's jab flash.
2. **Pounce:** the 10-tick crouch reads as a tell. The flight has the paws forward, with no float and no ledge climb. The pin lands at the goblin's feet. A whiffed pin reads as exposed.
3. A glance looks weaker than a hit: a 1-tick flash and no flinch. The goblin keeps its tell pose.
4. A running swat shows no foot-skate beyond what the s2 hind shuffle hides.
5. No Donut pixel is new or edited. Her identity is unchanged: cute in neutral, never a brawler. Carl is untouched.
6. In 2P at the midpoint camera, both heroes' attacks read at once. P2's plate shows `SWAT`/`POUNCE` in P2's glyphs.
7. The dry cue reads as "tried, nothing happened", never as a cast.
8. The `--debug-combat` boxes match the diagram.

## 9. Open questions (for the playtest; the defaults apply until then)

1. Does a 0-damage swat make P2 feel useful? The fallback is Q10-04 = 1, a weak jab.
2. Does the 60-tick guard read as a rule ("it won't fall for that twice") or as a random failure? The alternative is a longer swat `HURT_COOLDOWN`.
3. Boss #115: swats alone can trigger a poise flinch every 40 or more ticks without dealing HP. #115 owns whether light hits count toward poise.
4. Pounce flanking and the contract's pounce-bounce remain candidates.
5. Donut's faster scamper and higher jump are separate work, and the boss rack depends on that jump.

## Horizon fit

Consulted the [character progression horizon](../01-design/character-progression-horizon.md). The design fits it:

- Before the biscuit, Donut gets only low-damage feline contact and no magic.
- The pounce stays after the biscuit.
- There are no new buttons, and Carl is unchanged.
- P2 can act from the first scene.
- Later pounce upgrades can be data changes.

The guide needs no update.

## Rights/Provenance

Claude Opus designed the moves and their frame data and wrote this spec and the diagram script, as the design step. No new pixels were drawn. The diagram composites the frozen ANIM-DONUT-02 keys and the GOB-01 tell frame. It is published under the owner-reported `ART-PUBLICATION-ALL-V1` scope ([rights register](../04-production/rights-and-release.md)). No generated illustration or third-party asset was used, and no game-distribution right is implied.
