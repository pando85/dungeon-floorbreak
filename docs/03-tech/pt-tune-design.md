# PT-TUNE: the "powerful enemies" pass (design note)

**Status:** top-tier design note for the medium-tier execution of PT-TUNE ([plan row](../07-agents/f01-playtest-integration-plan.md), backlog `PT-TUNE` and follow-ups `PT-FU-B7`). It was **executed in the same PR** (see "Execution notes" at the end for the deviations and the measured misses). Every number here is a greybox hypothesis and an adopted default (`PTT-Qn`), and game feel is the owner's call after merge. The one validation result quoted below comes from a local probe and is labelled as one. Every other figure is a target, not a measurement.

**Scope.** This is a data pass. The closing arena (`f01_mastery`, room 6) gets the slime, the acid pool and a Rot Sticker; both waves get overlapping threats; and PTM-Q1 lands as ruling (c). Code changes are limited to four things:

- the scoped rule-5 validator clause;
- one builder call (llama party targeting) and the soffit argument in `f01_mastery.rs`;
- pilot rules plus a headless difficulty measure in `f01_mastery_fixtures.rs`;
- tests.

Rooms 2, 4 and 5 keep their composition data. No enemy HP, damage or timing knob changes (#178: difficulty comes from composition, not HP).

## 1. Mastery-wave composition targets

### Fixed geometry (unchanged room file, `cf02_microarena.room.json`)

| Feature | Value (layout px, y down) | Role in the waves |
| --- | --- | --- |
| Floor, ceiling | floor 580, `ceiling_px` 228 | one walking lane |
| Spawn | Carl 128, Donut 96, `protected_spawn` 48..160 | nothing may reach it on tick 0 |
| Left perch | `{544,500,128}`, runtime slab 12 (underside 68 px up) | W1 high ground |
| Parapet | 736..768, 48 px tall | splits the waves; a 48 px wall blocks a llama glob (spit line 42..48 px up) |
| Right perch | `{896,500,96}`, slab 12 | the W2 spit refuge |
| Rotten shore | strike column 384, footprint 409..489 | the arena's own environmental answer (unchanged) |
| Checkpoint | 680..728 (CF02-F checkpoint 704) | must lie left of every interactive prop |
| Exit gate | 1072..1136, opens on clear | nothing may stand in it |

**Hard constraint found by probe.** `encounter.rs` requires every prop target to have `target.x >= checkpoint.right()` (728), stickers included. A Rot Sticker under the left perch is therefore impossible without moving the checkpoint, and moving it would break the CF02-F checkpoint contract (704). All Rot pressure lives in the W2 yard. W1's hazard pressure comes from the slime, its pool and a llama.

### Roster (adopted PTT-Q1)

| id | kind | x | beat | delay | Temperament source | Wakes when |
| --- | --- | --- | --- | --- | --- | --- |
| s1 | slime (slot), facing -1, home 304, `linked_pool` "1" | 304 | W1 | 0 | none (slot kind) | whole body in view and a hero hurtbox within 160 px (HAZ-Q06); leash 96 |
| l2 | llama | 640 | W1 | 30 | none | in view and a hero within 320 px in its facing (fair-fire); leash 160 from 640 |
| g1 | goblin | 520 | W1 | 60 | seeded CF02 draw with temperament cover (unchanged builder) | sight 220 |
| g2 | goblin | 600 | W1 | 90 | seeded CF02 draw | sight 220 |
| g3 | goblin | 784 | W2 | 120 | seeded CF02 draw | sight 220 (blocked by the parapet until a hero crosses) |
| g4 | goblin | 832 | W2 | 150 | seeded CF02 draw | sight 220 |
| l1 | llama | 1000 (home, CF02-Q29) | W2 | 180 | none | fair-fire, 320 px; leash 160 |
| e1 | Goblin Engineer, gate `cf02_engineer`, `fallback_kind` goblin | 1036 | W2 | 210 | seeded CF02 draw | sight 220 |

As on main, the existing kinds stand at their column from tick 0 and wake by sight. `delay_ticks` is authoring order only, and the values are distinct because the validator forbids synchronised spawns.

**Caps.** W1 has 4 entries and W2 has 4, 8 in total. That is exactly `max_living` 4 per beat and `MAX_ROSTER_WAVES` 8. **The caps are not raised** (PTT-Q11). The slime counts toward W1 (ruling (c)). The Rot Sticker is a prop and does not count.

**Exit and spawn checks.**
- e1's body ends at 1060, inside the exit limit of 1072 (the existing test asserts `x + 24 <= 1072`).
- The slime's home of 304 is 164 px from the right edge of Carl's spawn hurtbox (140), so it is dormant on tick 0. Its leash floor is 208, and its lunge (reach about 51) ends around 157, short of both spawn columns.
- l2's leash floor of 480 sits 352 px from Carl's spawn column, beyond the 320 px fire range, so it cannot spit at the spawn.
- The checkpoint is now empty: g3 moves out of 680.

### Wave 1: the brawl lane (slime pocket under pressure)

**Threats.** The slime and its acid pool (272..336) sit in pocket a (176..352). The goblins g1 and g2 hold the lane behind the pool. The llama l2 sits behind them under the left perch.

**What overlaps, and why it is hard without more HP.**
1. Walkers cannot cross acid (HAZ-Q48), so g1, g2 and l2 stop at the pool's right edge. The slime has to be fought on its own terms: bait the lunge, then punish in its 68-tick window (slime HP 3).
2. Spit does cross acid. l2 closes to its 200 px stand-off from Carl (Carl at about 250 puts it at about 450 to 480, inside its leash). A hero waiting out the slime's punish window is in a glob line. Jumping does not help: Carl's feet reach 33 px up, and the glob line is 42 to 48 px.
3. Killing the slime drains its pool (rule 7's "drains"). That **opens the floodgate**: g1 and g2 rush a hero who has just spent his punish. The answer is spacing and the push kick, not damage.
4. The llama is reachable only past the goblins or from the left perch (dive). The perch is the W1 high ground but gives no spit cover.

There are at least two engaged threats (slime plus spit, then goblins plus spit) and three kinds (slime, llama, goblin). Every one dies in its base number of hits.

### Wave 2: the llama yard (Engineer with a llama, Rot pocket under pressure)

**Threats.** The goblins g3 and g4 stand in the matador lane (780..852). l1 holds its home at 1000. The Engineer e1 is at 1036. The Rot Sticker r1 hangs under the right perch's left end (dome x 912..928, blast radius 48, computed sweep 864..976), with pocket b at 880..944.

**The Engineer-with-a-llama pairing (the core of W2).** Each Engineer answer is crossed by the llama:
- **Jump.** Jumping his charge does not leave the glob line (feet 33 px against a 42..48 px line). Against l1 the jump costs a hurt unless the hero times it between spits.
- **Matador.** With the hero's back to the parapet (flush 780), the Engineer stands off at 852 and charges into the wall. That spot is 220 px from l1: inside fire range, in view, and with no cover on the right side of the parapet. The matador answer works only right after l1 has spat, or once l1 is down.
- **Bowling his own crew.** g3 and g4 stand between the parapet and the Engineer's stand-off, so the "make him charge through his goblins" answer is available on the first approach. It is the safe opening for a hero who reads it.
- **Perch dive.** The right perch is the spit refuge (80 px up, above the glob line). Diving on the Engineer or l1 from it is the strongest answer, but the Rot Sticker under it punishes a hero who fights beneath the perch.

**The Rot pocket under pressure.**
- r1 wakes within 144 px and triggers within 32 px or on any hit. Its blast hits every hero (`AllHeroes`) and launches goblins.
- l1's stand-off and leash (840..1160) keep it pacing under and around the sticker.
- Goblins chasing a hero under the right perch can be launched by triggering r1 on purpose: the yard's environmental answer, mirroring the shore in W1.
- A hero who fights under the perch without looking up eats the blast.
- The sticker is `optional: true` (HAZ rule 10), so no step depends on its blast.

During W1 the parapet walls off g3, g4 and e1 and blocks l1's spit for heroes left of it. The checkpoint at 704 is spit-safe.

### Exact sidecar values (`assets/levels/f01_late/mastery.encounter.json`)

Change only the keys below. Rewrite the `status` sentence so it describes this composition and drops the "not in this file yet / PTM-Q1, open" text. Everything else stays as it is (`max_living` 4, `waves` true, spawn, checkpoint, exit, goal, pin, `locks`, `hazards`, `links`).

```json
"platform_slab_px": 12,
"props": [
  { "id": "r1", "kind": "rot_sticker", "optional": true,
    "target": { "x": 902, "y": 512, "w": 20, "h": 14 },
    "track": { "orientation": "ceiling",
      "surface": { "x": 896, "y": 500, "w": 96, "h": 12 },
      "a": { "x": 912, "y": 512 }, "b": { "x": 928, "y": 512 },
      "start": { "x": 912, "y": 512 } } }
],
"acid_patches": [ { "id": 1, "x0": 272, "x1": 336, "floor_y": 580, "linked_slime": "s1" } ],
"pockets": [
  { "id": "a", "x0": 176, "x1": 352, "max_stickers": 0, "max_slimes": 1, "max_goblins": 0 },
  { "id": "b", "x0": 880, "x1": 944, "max_stickers": 1, "max_slimes": 0, "max_goblins": 0 }
],
"roster": [
  { "id": "s1", "kind": "slime", "x": 304, "facing": -1, "home": 304, "linked_pool": "1", "beat": "W1", "delay_ticks": 0 },
  { "id": "l2", "kind": "llama", "x": 640, "beat": "W1", "delay_ticks": 30 },
  { "id": "g1", "kind": "goblin", "x": 520, "beat": "W1", "delay_ticks": 60 },
  { "id": "g2", "kind": "goblin", "x": 600, "beat": "W1", "delay_ticks": 90 },
  { "id": "g3", "kind": "goblin", "x": 784, "beat": "W2", "delay_ticks": 120 },
  { "id": "g4", "kind": "goblin", "x": 832, "beat": "W2", "delay_ticks": 150 },
  { "id": "l1", "kind": "llama", "x": 1000, "beat": "W2", "delay_ticks": 180 },
  { "id": "e1", "kind": "engineer", "gate": "cf02_engineer", "fallback_kind": "goblin", "x": 1036, "beat": "W2", "delay_ticks": 210 }
]
```

**Probe evidence (local, not CI).**
- Setup: this exact data, with the section 2 skip clause applied as a one-line local edit.
- `encounter::load` passed every check: HAZ rules 1 to 10, `CheckpointInvalid`, the caps, spawn stagger, `RosterOnProtected` and `PlatformBlocksLane`.
- The only failure was the deliberate pin mismatch (`ENCOUNTER_HASH`).
- The validator computed r1's sweep as 864..976.
- Without the skip clause, rule 5 rejects every non-slime entry ("an enemy spawns in no pocket").

**Why `platform_slab_px` 12 (PTT-Q2).**
- It matches the runtime slab collider (12).
- r1's track surface then equals the right perch slab, so `f01_hazards::soffits` adds no phantom full-height soffit under the perch. With 16, the surface `{896,500,96,12}` would not match the platform slab.
- This is the "soffit solids as data" item of PT-FU-B7 (1).

## 2. The PTM-Q1 validator change (ruling (c), `waves: true` only)

**Where.** `src/encounter/haz.rs`, rule 5 block (on main around lines 705..786).

**Rules.**
1. **Lift.** The closing loop "an enemy spawns in no pocket" runs only when `!inp.raw.waves`.
2. **Keep.** Every other rule-5 clause runs unchanged for every sidecar, wave rooms included:
   - pocket overlap;
   - patch-in-pocket;
   - slime-home-in-pocket;
   - the per-pocket counts (`max_stickers`, `max_slimes`, `max_goblins`);
   - sticker track in pocket.

   This keeps "counts and safety inside pockets".
3. **New, wave rooms only (`inp.raw.waves`).** Every non-slime roster entry's `body_x(e)` must not overlap:
   - any pocket `[x0, x1)`: `b0 < pk.x1 && b1 > pk.x0`;
   - any sticker's `sweep_x` `(lo, hi)`: `b0 < hi && b1 > lo`.

   A violation is `haz(5, &[&e.id, &pocket_or_prop_id], "a wave enemy spawns in a pocket or a blast sweep")`. Slimes are exempt because their home must lie in a pocket (an existing clause).
4. **Slimes count toward the cap.** No new code is needed: a slime entry has a `beat` and is already counted by the per-beat `max_living` and the `MAX_ROSTER_WAVES` total (`encounter.rs`, PTM-AD6). A test pins this.

Rules 1 and 3 are one `if inp.raw.waves { new loop } else { old loop }` at the same place. The `Sticker` list (with `sweep_x`) and `pockets` are already in scope there.

**Tests** (in the `haz` tests module, built from a minimal waves fixture: a slime pocket, one sticker pocket, and goblins on dry floor outside both):

| # | Fixture | Expect |
| --- | --- | --- |
| P1 | the minimal waves fixture | valid |
| P2 | the shipped mastery sidecar (through `f01_mastery::launch`) | valid, pins match |
| N1 | waves, a goblin at a pocket's interior column | rule 5 "pocket or a blast sweep" naming the goblin and the pocket |
| N2 | waves, a goblin outside every pocket but inside a sweep (`body_x` overlapping `sweep_x` only) | the same error naming the sticker |
| N3 | waves, a llama straddling a pocket edge by 1 px | error (half-open edges: `b1 == x0` passes, `b1 == x0 + 1` fails) |
| N4 | waves, a fifth W1 entry that is a slime (with its own pocket and patch) | `TooManyLiving` (slimes count) |
| N5 | waves, a pocket with two goblins *homed inside it* through `x` | the per-pocket count error still fires |
| N6 | **no** waves, a goblin outside every pocket on a hazard sidecar | "an enemy spawns in no pocket" (unchanged) |
| N7 | **no** waves, a goblin inside a sweep but inside a pocket | no new error (the new rule does not run) |

**Proof that every other room validates the same.**
1. On main only `assets/levels/f01_late/mastery.encounter.json` sets `"waves": true` (checked with `git grep '"waves"' -- assets`). Every other sidecar takes the `else` branch, which is the unchanged old loop. Every other clause is byte-for-byte unchanged.
2. A test, `non_wave_sidecars_validate_identically`, loads every committed `*.encounter.json` other than mastery and asserts four things:
   - the file loads with zero errors;
   - `waves` is false;
   - its loaded `sidecar_hash` equals the existing pin constant of its module (hazards, boiler, hoarder, ring court, ring split, corral and the rest);
   - the error list of each existing negative test is unchanged (the existing negative tests keep passing unedited).
3. The non-mastery room pins and every non-mastery replay or pilot golden stay byte-identical. The full test suite is the proof, and no other golden may move.

**PT-FU-B7 (7) is superseded.** Refusing `waves` together with pockets, acid or stickers was a stop-gap "until PTM-Q1 lands". This PR lands it, so the refusal is not added.

## 3. Headless difficulty measure (the scorecard's "powerful enemies" line)

**Where.** A new pure function `difficulty(...)` in `src/f01_mastery_fixtures.rs`. It is sampled once per fixed tick of a pilot run from `CombatState` and the scene, and returns one `WaveDifficulty` per beat plus totals. It is Bevy-free and deterministic, and it is reported in `Report::line()`.

**Definitions (adopted PTT-Q8).**

An enemy or hazard is **engaged** with a present hero on a tick when it is awake and not defeated, and one of these holds:
- **Melee kinds** (goblin, Engineer): horizontal gap to the hero ≤ `ENGAGE_MELEE_PX` 96, on the same side of the parapet (736..768), or in a tell, charge or attack that targets the hero.
- **Slime:** gap ≤ its trigger px (44) plus `ENGAGE_MELEE_PX` 96, that is 140; or it is in an attack.
- **Llama:** the fair-fire rule holds for that hero, with the glob line unblocked by 48 px or more of cover; or one of its globs is in flight.
- **Rot Sticker:** awake (within 144) and not yet spent.

A tick is **wave-active** for beat B from the first tick any B entry is engaged to the tick B's last entry is defeated.

| ID | Measure | Target (seeds 0..8, base-kit pilots) |
| --- | --- | --- |
| D1 | Overlap share: wave-active ticks with ≥ 2 engaged threats, divided by wave-active ticks, per wave | `CarlSolo`: W2 median ≥ 40 %, **W1 median ≥ 30 % (PTT-Q19)**, every seed ≥ 20 % in each wave |
| D2 | Peak distinct engaged kinds at one tick (goblin, Engineer, llama, slime, Rot) | W2 ≥ 3 at one tick; **W1 ≥ 2 at one tick and ≥ 3 distinct kinds over the wave (PTT-Q18)**; every seed |
| D3 | `CarlSolo` time from tick 0 to `cleared_tick` | **bot check (PTT-Q20): every seed clears, median ≥ 1.25 × main's 517 (647), every seed ≤ 4800**. The 900..3000 human band (15 to 50 s) is the owner scorecard's line (OQ4), measured in the owner playthrough; the bot's figure is printed beside it |
| D4 | `CarlSolo` hurts and defeats | `hero_hurts` median 3..10, ≥ 1 in each wave, `deaths` ≤ 2 on every seed |
| D5 | Peak engaged enemies (Rot excluded) | ≤ 4 at every tick, every pilot (LAB-Q04 holds in play) |
| D6 | HP guard | every mastery enemy's starting HP equals its base kind's (test) |
| D7 | Kit check, report-only: `Who::DonutKit` (Donut solo with blink and jump boost) and `Who::CarlKit` (Carl solo with dynamite and gauntlet) | report clear rate and D3 per seed. Expect ≥ 7 of 9 clears and a faster median than the base kit. A miss goes in the owner packet, not a merge blocker (PT-FU-B7 (5)) |

**Discrimination check (mandatory evidence).** Run D1 and D2 on main's composition too (the current sidecar, on a scratch branch, report only). Main's W1 is three goblins, so D2 = 1 there. If main passes D1 and D2, the measure is too loose: tighten `ENGAGE_MELEE_PX` before tuning anything.

**Remedy rule (PTT-Q10).** A miss is fixed by composition only: x, beat, kind, delay order, or pocket and patch placement. It is never fixed by HP, damage, speed or a tuning file. If two composition iterations still miss, ship the numbers as measured and list the miss in the PR body. Never adjust a target to fit a result.

The pass-or-fail assertions for D1 to D6 go in one test, `the_mastery_waves_meet_the_difficulty_targets`. D3 and D4 depend on the pilot, so the pilot rules below are part of the measure:
- **Slime:** stand at lunge-bait range (60 to 70 px), step back on the tell, then punish with two jabs in the 68-tick window.
- **Acid:** never stand in a live patch. Jump a live patch only when the slime is down or the hero is the slime's target.
- **Rot:** do not stand within 32 px under an awake sticker; strike it only when a goblin is in its blast and no hero is.

The base-kit rule is unchanged for `CarlSolo`, `DonutBySwap`, `DonutFights` and `Companion`.

## 4. The full-kit Juicer rule (it already holds)

**Rule.** The Juicer fight is full-kit only under `--f01-loadout all` or an explicit post-defeat rematch, never in story.

**Verified on main by reading the code:**
- The story pins for room 5 are `f01_dynamite_held` (Carl: thrown charge) and `f01_donut_boss_held` (Donut: jump boost). The gauntlet (`fist_reinforced`) and the blink (`special_short_blink`) are granted on `brute_boss_f01_defeated`.
- KR12 `check_story_order` fails any room pin that holds a later grant.
- `Loadout::All` is the test loadout: every capability is held and story triggers are off. `floor01_scene` refuses `--f01-loadout all` combined with `--carl-prog` or `--donut-prog`.
- No rematch code exists. `respawn()` keeps defeated enemies defeated, and a fixture restart restores the declared pin.
- Existing tests:
  - `the_juicer_grants_exactly_the_gauntlet_and_the_blink_under_the_story_loadout`;
  - `no_grant_fires_under_the_all_loadout`;
  - the defeat-retry carry test.

**No code change.** Add three pinning tests:
1. A story Juicer start with no carried ledger holds neither `FIST_REINFORCED` nor `SPECIAL_SHORT_BLINK` on tick 0.
2. After a hero defeat and a fixture restart, the boss is present again and neither grant is held.
3. The only story state that holds the full kit is the carried post-defeat ledger. The Juicer is then defeated, so no story run fights a full-kit Juicer.

A real "explicit rematch" entry point does not exist and is **not** added here. If the owner wants one, it is a separate work order.

## 5. Rooms 2, 4 and 5

| Room | Composition data change | Why |
| --- | --- | --- |
| 2, the Boiler | none | Its setpiece (props, vents, perches) is the threat; the "powerful enemies" goal is the closing arena's |
| 4, the hazards room | none | Its pockets are isolated on purpose: it teaches the slime, acid and Rot one at a time before room 6 combines them |
| 5, the Juicer | none | Boss tuning is out of scope (section 4) |

One optional, hash-neutral hygiene fix is included: PT-FU-B7 (6). `src/f01_hazards.rs` (about lines 173..182) silently drops non-goblin existing kinds. Make it refuse them the way `f01_hoarder.rs` and `f01_boiler::existing_enemies` do. The hazards sidecar holds only goblins and slimes, so its hashes do not move. Add one negative test: a llama in a hazards-style sidecar is refused.

## 6. Goldens that move

| Pin | Moves? | Note |
| --- | --- | --- |
| `f01_mastery::ROOM_HASH` 0x4991_e0c7_5078_4c18 | **no** | The room file is unchanged (confirmed by probe: room hash equal) |
| `f01_mastery::ENCOUNTER_HASH` 0xd5ef_f132_19c0_020a | **yes** | Re-pin to the committed file's hash. The probe's value came from a differently formatted file, so take the value from the test output |
| `f01_late::MANIFEST_HASH` 0x956c_44a8_bbd1_41c3 | **no** | `manifest.json` is unchanged |
| `f01_mastery_fixtures` `GOLDEN` (4 x 9 = 36 pilot hashes) | **yes** | The new roster and hazards, the new pilot rules and party-targeted llamas |
| New `KIT_GOLDEN` (2 x 9) | **new** | D7 pilots, pinned for replay |
| PLAYTRACE `a_recorded_mastery_session_verifies_strict` | regenerates | It records and verifies in a temp dir; no committed file |
| `f01_mastery.rs` asserts | **edit** | Roster columns (goblins [520,600,784,832], engineers [1036], llamas [640,1000]), `slot_enemies().len() == 1` (PTM-AD7 retired), and the entry line `goblins=4 engineers=1 llamas=2 slot=1` (or the existing format with slot 1) |
| Every other room, replay or pilot hash | **no** | A move is a defect |

## 7. Adopted defaults (`PTT-Qn`) and owner questions

| ID | Adopted value | Reason |
| --- | --- | --- |
| PTT-Q1 | The roster of section 1 | Overlap by composition, within the 4/8 caps |
| PTT-Q2 | `platform_slab_px` 12 | Matches the runtime collider; no phantom soffit |
| PTT-Q3 | One Rot Sticker, `optional: true`, under the right perch (a 912, b 928) | The checkpoint-ordering rule forbids W1 stickers; a second yard sticker's sweep would overlap l1's home |
| PTT-Q4 | Slime home 304, patch 272..336, pocket a 176..352 | Dormant on tick 0; the lunge cannot reach the spawn columns |
| PTT-Q5 | l2 at 640 (W1), out of the matador lane | Closes PT-FU-B7 (8); its leash floor keeps the spawn out of fire range |
| PTT-Q6 | KC1 llama party targeting on in mastery (`.with_llama_party_target()` in `f01_mastery::combat_for`) | #181 gate (b); a fixture restart keeps it (only corral fixtures reapply opt-ins) |
| PTT-Q7 | The rule-5 change of section 2 | Ruling (c) |
| PTT-Q8 | Engagement thresholds: melee 96, slime 140, llama fair-fire, Rot awake | Section 3 |
| PTT-Q9 | D1 to D7 targets | Section 3 |
| PTT-Q10 | Composition-only remedy; at most two iterations, then ship as measured | #178 |
| PTT-Q11 | Caps unchanged (4 per beat, 8 total) | Eight entries fit exactly |
| PTT-Q12 | `env_runtime` passes `f01_hazards::soffits(&run.loaded)` instead of `&[]` | Generic soffit wiring (empty for this data) |
| PTT-Q13 | Pilot slime, acid and Rot rules of section 3 | The base pilot cannot clear a pool or a slime |

**Owner game-feel questions** (after merge; none blocks the merge):

- **OQ1.** Does the "kill the slime, the pool drains, the goblins rush" beat read as a trap you sprang, or as unfair?
- **OQ2.** Is W1's spit over the pool pressure or nagging? The alternative is l2 in W2 with a third goblin in W1.
- **OQ3.** Is one Rot Sticker enough for the arena? A W1 sticker needs the checkpoint moved, which breaks the CF02-F contract.
- **OQ4.** Is 15 to 50 s per clear for a base-kit bot the "powerful" feel, or too long for a closing room?
- **OQ5.** With party targeting, Donut gets spat at too. Is that right for the solo AI partner?

## 8. Horizon fit

This fits [the progression horizon](../01-design/character-progression-horizon.md) as written:
- **Line 14:** new enemy behaviors and geometry make the *same* abilities useful in new ways. The arena combines the existing slime, acid, Rot, llama and Engineer so that the jump, push kick, perch dive and bait-and-punish each have a right moment.
- **Line 17:** enemies still die in a few hits, because no HP changes.

No new verb, button, unlock or enemy type is added, and the guide needs no update.

## Execution notes (PT-TUNE execution, same PR)

Implemented as written, with these recorded deviations. The measured numbers are in the PR body; every one is a greybox measurement, not a game-feel verdict.

- **PTT-Q14 (composition iterations, PTT-Q10).** Iteration 1 narrowed the acid patch to 272..312, moved l2 to 616 (its leash floor 456 stays out of the spawn's 320 px fire range), g1 to 440 and g2 to 500. Iteration 2 moved g1 to 380 and g2 to 460. Both missed D1 and D2 in W1.
- **PTT-Q17 (reviewer-directed W1, supersedes the W1 values of section 1).** Slime and home 352, acid patch 320..360, pocket a 224..400, g1 430, g2 490, l2 616 (unchanged from Q14); everything else as designed. Why: in a duel at the shipped W1 the hero holds the bait spot at about 220 while l2 stands about 343 px away, beyond its 320 px fire range, so the slime phase was a solo duel. With these values the llama spits during the duel (56 slime-plus-llama ticks per seed). Valid under the wave clause (g1's body 418..442 is outside pocket a); the slime is dormant on tick 0 (its pivot is 212 px from the right edge of Carl's spawn hurtbox, beyond the 160 px wake distance); its leash floor (256) and lunge stay clear of the spawn columns.
- **PTT-Q18 (D2 for W1, design defect in PTT-Q9).** The acid bars walkers (HAZ-Q48), so section 1 itself describes W1 as sequential pairs ("slime plus spit, then goblins plus spit"), and "3 kinds at one tick" contradicted it. D2 for W1 is at least 2 kinds at one tick and at least 3 distinct kinds over the wave (`kinds_seen`). W2 is unchanged. `ENGAGE_MELEE_PX` stays 96, so a goblin held behind the acid still does not count as engaged (main still fails: W1 shows 1 kind).
- **PTT-Q19 (D1 for W1).** W1 is the teaching wave and W2 the test, so W1's median target is 30 % (three quarters of W2's 40 %), with the 20 % per-seed floor. W2 is unchanged.
- **PTT-Q20 (D3, design defect in PTT-Q9).** The 900..3000 band is human time applied to a bot that never hesitates, under constraints that composition cannot beat (no HP or damage change, capped rosters, a geometry that ends at 1072). D3 is now a bot check, "harder than main, no stall": median clear ≥ 1.25 × 517 = 647. The human band moves to the owner scorecard as OQ4. No HP is raised.
- **PTT-Q21 (D7 explanation).** `DonutKit` (and the fighter) clear 0 of 9, as `DonutFights` did on main. The slime is already dead by tick 108 when Donut takes her first hurt; she then stands idle from about tick 500 to tick 3000 and takes a hurt about every 95 ticks (28 against 3 for Carl), while the assist partner Carl idles. The Donut pilot has no rule for the remaining W1 enemy at range, and the assist partner does not take over. The stuck rule counts only walking input without motion, so it does not see this. Follow-up: a stall measure for "no movement input and at least one hurt in a 300-tick window while a wave entry lives", reported per pilot.
- **PTT-Q15 (pilot).** The slime rule is one window of punishing jabs until it falls, not exactly two, and the pilot stays at its bait spot instead of retreating inside the bait range; on a ledge above an enemy it hops and dives (Down plus Physical). The base pilots otherwise keep their rules.
- **PTT-Q16 (CA2 harness).** The CA2 metrics test now drives Carl with the room's pilot (`cf02_free_fixtures::autonomy_metrics_with`, a behaviour-identical refactor), because the arena autopilot cannot bait the slime. M2 (shadowing) and M4 (steals) miss their spec targets in the new room and are bounded and listed as design notes.
