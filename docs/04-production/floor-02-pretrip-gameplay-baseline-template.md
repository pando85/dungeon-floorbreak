# GAMEPLAY-BASELINE-V1 — Before-vacation owner playtest and safe change contract

**Status:** owner fill-in template ONLY, not a completed playtest. Written 2026-11-22. Target: before 2026-11-25. The owner will conduct real gamepad tests before travel; an agent must not fill PASS fields from bot scripts or claim these observations are measured. Read [F01 PT-GATE-B owner packet](../evidence/f01-playtest/README.md) when present on current main, [F02 execution work queue](../07-agents/floor-02-vacation-execution-packet.md) and [Floor01 production plan (PR #194)](https://github.com/pando85/dungeon-floorbreak/pull/194) while still in draft.

## 1. Record identity and version

- Date/time of session: **PENDING**
- Git commit tested: **PENDING** (full SHA, not "latest")
- Build profile / game launch command: **PENDING**
- Gamepads P1/P2 actual hardware and local resolution: **PENDING**
- Scene/room fixtures played: **PENDING**
- Playtrace local session IDs and strict replay verdicts: **PENDING**. Keep personal raw recordings local and do not commit.
- Baseline owner-accepted? **NO** until explicitly accepted after test.

**Available known launch candidates from F01 PT packet (verify they still work on the tested SHA):**

~~~sh
cargo run --release --bin floor01_scene -- --scene f01_late --scale 2 --f01-room boiler --f01-loadout story
cargo run --release --bin floor01_scene -- --scene f01_late --scale 2 --f01-room hazards --f01-loadout story
cargo run --release --bin floor01_scene -- --scene f01_late --scale 2 --f01-room juicer --f01-loadout story
cargo run --release --bin floor01_scene -- --scene f01_late --scale 2 --f01-room mastery --f01-loadout story
cargo run --release --bin floor01_scene -- --cf02 --cf02-engineer --carl-dynamite --companion cf02_free
cargo run --release --bin floor01_scene -- --scene boss_f01 --carl-prog --donut-prog
~~~

**Important:** The F01 late-room launcher previously rejected `--companion` combined with room select; do not combine the options or write "tested F01 late autonomous companion" until the actual tested commit supports it. Two-pad behavior and companion solo pilot must use their actual supported scenes.

## 2. What the owner should actually check (minimum practical route)

| ID | Scene | Player observation to record | Decision field |
| --- | --- | --- | --- |
| BASE-01 | ordinary goblin / late mastery | Does Carl's grounded jab/kick kill satisfy? Is body-to-body collision obvious and real? | PASS / TUNE / BLOCKED / NOT RUN |
| BASE-02 | hazard / Boiler | Can the player deliberately create one env consequence with a kick or dynamite? Understand why an enemy dies? | PASS / TUNE / BLOCKED / NOT RUN |
| BASE-03 | solo companion scene | Does AI Donut **actually land** useful hits, evade and follow, rather than only animate? | PASS / TUNE / BLOCKED / NOT RUN |
| BASE-04 | solo Donut leading | Does AI Carl act and does switching preserve safety and room? | PASS / TUNE / BLOCKED / NOT RUN |
| BASE-05 | Juicer | Can both heroes contribute damage and recover from boss tells? | PASS / TUNE / BLOCKED / NOT RUN |
| BASE-06 | 2 real pads | Both humans retain agency, attack and recovery while together and separated; camera legible | PASS / TUNE / BLOCKED / NOT RUN |
| BASE-07 | Donut ability and danger | Is mobility/reflex/short blink responsive and not an invincible shortcut? Does Magic Missile trivialize stationary play? | PASS / TUNE / BLOCKED / NOT RUN |
| BASE-08 | visual clarity 1x and intended display | Carl 52px and Donut 40px retain silhouette; hazards and hit effects visible | PASS / TUNE / BLOCKED / NOT RUN |
| BASE-09 | story grants versus all-test loadout | Does true story route avoid future abilities and regrant on retry? | PASS / TUNE / BLOCKED / NOT RUN |
| BASE-10 | recording/replay | Real session recorded, strict verify where supported, expected source data pins | PASS / TUNE / BLOCKED / NOT RUN |

For each FAIL/TUNE record **one reproducible room + control sequence + expected behavior + actual behavior + optional Playtrace ID**. Distinguish subjective fun from a mechanically reproducible regression. A replay that runs does not establish enjoyable combat.

## 3. Tiny owner decision sheet (copy and fill after session)

- **Keep unchanged**: [basic kick/attack responsiveness], [input mapping], [hero physical scale], [jump reach], [companion baseline], [camera policy], [HUD/viewer policy]. Replace brackets with exact observed agreements; leave UNSPECIFIED if not tested.
- **Highest impact tuning needed** (top 1–3): **PENDING**.
- **Release blockers before shared F02 core editing**: **PENDING**.
- **Known acceptable deviations in new rooms** (new enemy windups, geometry, prop tuning, unique boss tells): **PENDING**.
- **Human game-feel rating of current Floor01 representative rooms**: **PENDING** (optional short 1–5 with one sentence why; no false precision).
- **2P hardware tested?** **NO** until real two pads are played.
- **Approval/acceptance by owner**: **PENDING**. The local orchestrator does not fill this field on its own.

## 4. How the local orchestrator should consume the baseline

1. Pin exact tested F01 SHA and actual owner-accepted behavioral rules in F02-W00 read-only audit and return report.
2. Any shared change to input, combat impulses, hit-stop, player movement, companion decisions, camera or story slot requires a before/after replay on the owner-approved F01 representative scenes. If a difference exists, disclose it and open a focused, independently reviewed regression PR; do not silently "fix" F01 to satisfy F02.
3. Room-local new enemy timing and prop data can be tuned autonomously within existing mechanics; stage/boss art has its own review. Never use owner approval of F01 as blanket approval of new F02 feel.
4. If baseline incomplete, continue isolated pure logic, original assets and authored level source, mark integration HOLD where needed; do not ask the traveling owner repeated permission questions.
5. Report what the owner actually tested, not implied or automated results. The first F02 review after return still requires human play.

**Template intentionally remains PENDING and cannot be treated as user sign-off simply because this file is merged.**
