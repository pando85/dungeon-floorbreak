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
| Restart reshuffle (Q58) | `cf02_morale_restart` | see the table |

## Reactions per attempt over three restarts

`cf02_morale_restart` (three chasing goblins, Carl jabs, hits and kicks on the same ticks every attempt; seed `0xCF020D00`). The draw is `mix(seed, restarts, goblin, draw)`; the offset is the nerve offset from the temperament baseline. Test `q58_restarts_reshuffle_and_each_attempt_replays_identically` runs the three attempts twice and compares events and hashes.

| Attempt (`restarts`) | Temperaments (goblin 0, 1, 2) | Reactions (attempt-local tick, nerve after events) |
| --- | --- | --- |
| 0 | Brawler -4, Coward +1, Brawler -5 | g1 hesitate @109 (26) |
| 1 | Coward -1, Show-off +5, Coward -4 | g1 hesitate @109 (45) |
| 2 | Brawler -3, Brawler +3, Coward -1 | none |

## Not run

- The 1x GIF of a routine kill and a wall slam in front of a coward, a show-off and a brawler is **NOT RUN** (the pose and clip wiring for reactions is CF02-I; the goblins show their existing frames).
- "The room still ends within 60 s in the arena autopilot" needs the CF02-F microarena; **NOT RUN** here. The unit-level bounds (a panic ends within 90 ticks or when cornered, no tell is shortened) are tested.
