# CF02-G evidence

Replays, not captures: the eleven morale fixtures are in [`../replays/`](../replays/) (run twice by `scripts/ci_floor01_combat.sh headless`, pinned by `GOLDENS` in `src/cf02_fixtures.rs`). Seeds were chosen with the stateless Q58 draw so each fixture's goblins get the temperament its scenario needs (`q58_every_g_fixture_draws_the_temperaments_its_scenario_needs` pins them). A `dummy` goblin (`Passive`) is a QA victim without morale.

| Beat (design question) | Fixture | What the replay shows |
| --- | --- | --- |
| Witness, hesitate (Q60, Q61) | `cf02_morale_witness_hesitate` | haymaker kill on tick 20 (routine, -20): the Show-off 90 px away (nerve 43 after the event) hesitates 20 ticks and loses its turn; the goblin 380 px away and the dummy do nothing |
| Spectacle, panic (Q60, Q61) | `cf02_morale_spectacle_panic` | lethal kick (heavy, -35) on 43: the Coward behind Carl panics (nerve 11), its tell is ended (`goblin_interrupted`), it flees away from the scare for 60..90 ticks and attacks nothing |
| Panic once (Q57, Q61) | `cf02_morale_panic_once` | panic on 43; a second routine death on 165 only makes the same Coward hesitate (nerve 0) |
| Last goblin (Q62) | `cf02_morale_last_goblin` | the same routine kill and Show-off as the hesitate fixture, but nobody else lives: panic at nerve 43 (< 50) |
| Cornered (Q63) | `cf02_morale_cornered` | panic on 20, blocked by the raised block (x 1392), `cornered` on 64 with nerve 40, then it tells and swings |
| Enraged, tell unchanged (Q61, Q68) | `cf02_morale_enraged_tell_unchanged` | the swing that hurts Carl on 30 enrages the Brawler witness (nerve 86): walk 81, cooldown 24 (the hitter, in its swing, gets no reaction and rests 40); every tell is still tell + 26 = swing |
| Coward back-off (Q64) | `cf02_coward_backoff_kick`, `cf02_coward_jab_still_hits` | the kick (startup 13) starts on 5: the Coward hops back 28 px over 10 ticks and the kick finds nothing; the jab (startup 3) hits it on 8 |
| Show-off taunt (Q65) | `cf02_showoff_taunt_punish` | Carl's jab whiffs on 10: taunt (36 ticks, nerve unchanged); the jab on 39 staggers it 20 ticks instead of the jab's 10 |
| Pincer token (Q66) | `cf02_pincer_token` | goblins 0 and 1 close in from the left, goblin 2 from the right: goblin 1 (nearest) tells on 1 and the second token goes to goblin 2 on the other side, not to goblin 0 (the legacy rule, tested with morale off, gives it to goblin 0) |
| Restart reshuffle (Q58) | `cf02_restart_reshuffles` | see the table |

## Adopted defaults G-A1..G-A9

The implementation (PR #149) adopted nine defaults that the design table does not state. They are recorded as amendments under Q61, Q65, Q66 and Q68 in [`combat-fun-02-post-demo01-proposals.md`](../../../01-design/combat-fun-02-post-demo01-proposals.md#amendments-adopted-by-the-cf02-g-implementation-pr-149-g-a1g-a9).

| ID | Question | Default |
| --- | --- | --- |
| G-A1 | Q61 (Q60 events) | A living flier's contact, a body contact and a wall slam without a defeat are spectacles (-15) for witnesses; a slam that kills is the spectacular defeat only |
| G-A2 | Q61 | A reaction starts only from Neutral, Hurt or a tell, never while lying, flying, in a swing or in recovery. Narrowed: the reaction ends the tell; admitting `Hurt` is an open question |
| G-A3 | Q65 | A "whiff" is Carl's move ending its active phase without a hit (Donut's are not tracked); the taunt punish takes any hero's non-launching hit, damage or not |
| G-A4 | Q65 (Q64) | The Coward's back-off triggers on any hero move with startup >= 9 that starts in front within 48 px (Donut's pounce included) |
| G-A5 | Q61 | A hesitate did not abort a tell, only a panic did. **Superseded** by `q68_hesitate_aborts_tell` (below) |
| G-A6 | Q68 | A goblin in its first-approach delay takes no token; `q58_delay_from_first_sight` (shipped off) counts the delay only while a hero is in sight |
| G-A7 | Q66 (fixtures) | A wall-slammed goblin gets the flier hit-stop (4 ticks) |
| G-A8 | Q66 (fixtures) | Two contact fixtures start with a QA launch, because Carl's own kick would also hit a goblin that close |
| G-A9 | Q61 (Q60 events) | A `ShoreLanded` spectacle uses the block footprint centre |

**G-A2 and G-A5 after the re-review fix.** G-A5 is superseded: with `q68_hesitate_aborts_tell` on (the shipped value), a hesitate that starts in a tell ends it like a panic does and the token returns on the first tick (Q68). The knob off is the old behaviour and reproduces the old hashes. No committed G fixture starts a hesitate inside a tell, so no fixture golden moved; the new tests `q68_a_hesitate_started_in_a_tell_returns_the_token_on_its_first_tick`, `q68_the_hesitate_knob_off_keeps_the_legacy_tell_and_token` and `q68_the_hesitate_knob_moves_no_committed_fixture_hash` pin all three facts. G-A2 is narrowed, not replaced: the tell part is now consistent with Q61 and Q68, and `Hurt` stays admitted (a hurt goblin holds no token). Excluding it moves the hazard-room and Donut-progression pilot goldens and is left to the owner.

## Reactions per attempt over three restarts

`cf02_restart_reshuffles` (three chasing goblins, Carl jabs, hits and kicks on the same ticks every attempt; seed `0xCF020D00`). The draw is `mix(seed, restarts, goblin, draw)`; the offset is the nerve offset from the temperament baseline. Test `q58_restarts_reshuffle_and_each_attempt_replays_identically` runs the three attempts twice and compares events and hashes.

| Attempt (`restarts`) | Temperaments (goblin 0, 1, 2) | Reactions (attempt-local tick, nerve after events) |
| --- | --- | --- |
| 0 | Brawler -4, Coward +1, Brawler -5 | g1 hesitate @109 (26) |
| 1 | Coward -1, Show-off +5, Coward -4 | g1 hesitate @109 (45) |
| 2 | Brawler -3, Brawler +3, Coward -1 | none |

## Not run

- The 1x GIF of a routine kill and a wall slam in front of a coward, a show-off and a brawler is **NOT RUN** (the pose and clip wiring for reactions is CF02-I; the goblins show their existing frames).
- "The room still ends within 60 s in the arena autopilot" needs the CF02-F microarena; **NOT RUN** here. The unit-level bounds (a panic ends within 90 ticks or when cornered, no tell is shortened) are tested.
