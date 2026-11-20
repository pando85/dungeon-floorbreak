# CF02-B evidence

Replays, not captures: the nine `cf02_*` fixtures of CF02-B are in [`../replays/`](../replays/) (run twice by `scripts/ci_floor01_combat.sh headless` and pinned by `GOLDENS` in `src/cf02_fixtures.rs`). Tick numbers below are the attempt-local ticks of the committed replays.

| Beat (design question) | Fixture | What the replay shows |
| --- | --- | --- |
| Contact knockdown (Q09, Q12) | `cf02_contact_knockdown` | kick lands on 18, `goblin_contact launch=1 source=0 target=1` on 29; goblin 1 hops (vx 90, vy 90, no launch id), lies 40 ticks and gets up 22 ticks later; no damage; the flier keeps 25 % of 170 px/s |
| Below the threshold (Q09) | `cf02_contact_below_threshold` | a 100 px/s flier (QA launch) gives the legacy stagger 24 and push 60 px/s, never a knockdown |
| Attack ends, token returns (Q09, Q35) | `cf02_contact_cancels_swing` | goblin 1 starts its tell on 16, the flier (QA launch from behind Carl) meets it on 28: `goblin_interrupted`, knocked down, no hero hurt |
| No chain (Q09) | `cf02_no_chain` | three goblins in a row: one `goblin_contact`; the knocked goblin (no launch id) never touches the third |
| Bodies bowl (Q10) | `cf02_body_bowl` | jab then lethal kick (tick 43): the body keeps the kick instance 2 and `body_contact launch=2 source=0 target=1 instigator=0` follows on 65, not a `goblin_contact` |
| Air relaunch (Q09, Q51) | `cf02_air_relaunch_contact` | the air kick relaunches a flying goblin as launch 1 and its contact knocks goblin 1 down |
| Wall slam, living (Q11) | `cf02_wall_slam_kill` | kicked at the raised block (x 1392): `wall_slam launch=1 goblin=0 instigator=0` then `enemy_defeated ... force=heavy` on the same tick (34), credited to Carl |
| Wall slam, dead (Q11) | `cf02_wall_slam_body` | the lethal-kick body slams (59): the event only, no second defeat |
| Layout edge (Q11) | `cf02_layout_edge_no_slam` | the goblin stops at the right layout edge (2048): no `wall_slam`, only the kick's 1 damage |

`CombatEvent::BodyContact` and `CombatEvent::WallSlam` carry `instigator` (the hero index, `HeroId::index`, whose move made the launch) so the REWARDS work can attribute them. `CINE-CONTACT-SETUP` is never scored by a `BodyContact` (test `cine_a_body_contact_never_scores_the_contact_setup`).

**NOT RUN:** see the PR body for the 640x360 capture status.
