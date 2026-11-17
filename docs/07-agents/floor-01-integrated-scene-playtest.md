# Floor 01 — Integrated-scene playtest and experiment plan (v0.1)

**Purpose:** produce decision-quality human evidence, not mistake green CI or attractive screenshots for gameplay approval.

**Dependencies:** [gameplay/progression bible](../01-design/floor-01-gameplay-progression-v0.md), [current Bevy scene](../03-tech/floor01-scene.md), [existing scene evidence](../evidence/floor01-scene/README.md), [input/camera design](../03-tech/input-camera.md), [work order protocol](workorders.md). This is a testing design document; no new mechanics are implemented by it.

## Audit of current scene: what can actually be tested?

| Feature | Current integrated COMP-01/02 scene | Review |
| --- | --- | --- |
| Carl movement/jump/collisions/coyote/buffer | **Implemented** | Human feeling and controller check due |
| Real Carl h52, Donut h40 idle atlases | **Implemented** | Scale, legibility, grounding and overlap due |
| Donut following Carl's foot trail | **Implemented, passive only** | Visual impression; not a second playable actor |
| A/B/C environment, scrolling camera, torches, parallax and A1 lighting | **Implemented** | Mood, pacing, readability and camera check due |
| Direct Donut play, two independent gamepads, solo switching/commands | **Not yet implemented** | Separate DUO-INPUT / DUO-AI probes |
| True shared co-op camera / hot join and leave | **Not yet implemented** | Two-player camera and ownership probe |
| Shoulder Mode, mounted attacks | **Not implemented** | Separate reversible experiments |
| Goblins, Dozer chase, bombs, magic and destructible fight arena | **Not in this integrated corridor** | Some mechanics exist in a separate greybox; integrated encounters require separate PRs |
| Full Tutorial Guild room | **Not implemented** | Area-refuge work |

**Rule:** score absent features as N/A — NOT IMPLEMENTED, not PASS/FAIL. In particular, passive Donut proves neither co-op responsiveness nor companion AI quality.

## Stage A — baseline the scene that exists today

Run from repository root:

    bash scripts/export_assets.sh
    cargo run --release --locked --bin floor01_scene -- --scale 3

The 640x360 virtual scene is shown at 1920x1080 output (3x). Repeat with --deck (virtual 640x400 / output 1280x800) on an appropriate screen. Use --no-lighting for a comparable COMP-01 unlit presentation. Record commit, display size, platform and input device.

### A1 — Input, movement and collision

- Begin with 30–60 seconds idle. Then walk, stop abruptly, turn left/right five times and jump five times.
- Run/walk the entire authored corridor. Check coyote time and buffered jumps, 1/2-tile platform ascent, ground contact, clipping, exit and restart.
- Try D-pad, left stick and keyboard where available; test safe disconnect/reconnect and held-input release.
- Describe the feel in plain language: responsive vs heavy, predictable vs slippery, natural vs tiring. Give specific reproduction steps.
- Observe the existing **passive** Donut on reversal, jumping and stops: the 32 px foot-trail follow and pass-through overlap may look odd; this is not a test of Donut playability.

### A2 — Visual, camera and rendering

- Compare output at 1920x1080 and 1280x800. Can you identify Carl h52, Donut h40, the safe floor edge and nearby hazards at a glance? Any hard-to-read dark brick/light overlap?
- Compare A1 lighting enabled vs --no-lighting on the **same view**: torch flicker and pools, shadows, parallax haze, bloom, any excessive contrast.
- Is the calm A corridor distinct from B arches and C localized pipes? Do we have a coherent first-floor identity rather than constant visual noise?
- Look for parallax jitter, pixel seams, camera snap when reversing, bad foot anchors and odd behavior near the corridor ends.
- Evaluate 1980s–1990s antihero mood **without demanding future story/show effects at the opening**.

### A3 — Flow and curiosity

- Record duration of one full normal traversal, pauses/hesitations, and the one most memorable visual cue.
- Replay at a different pace. Ask whether the scene invites moving into the next room or whether the corridor is visually monotonous.
- Separate missing future gameplay (combat, sound, dialog, interactions) from existing defects in control/rendering.

**Stage A output:** at least three actionable observations; one thing to **preserve**; screenshots/recording when helpful. CI screenshots alone cannot substitute for game-feel judgement.

## Human baseline observation — first hands-on run (2026-11-16)

**Source:** creative director reported that they ran the previously suggested commands for the existing COMP-01/COMP-02 scene. Their actual assessment in Spanish was: **"no veo nada especial, tiene buena pinta y ya"** ("nothing especially notable; it looks good, that's all").

**What this supports:** the art/scene gives a positive basic impression, but the present demonstration does not yet provide a memorable gameplay hook. This is **not** a measured score or an acceptance of movement tuning, jump buffering, companion AI, 2P camera, or combat, and the tester did not supply device, frame timing, specific pass/fail records, or a recording. Retain the available visual baseline without polishing it endlessly in isolation.

**Action:** move the next design validation toward **playing Donut directly before she has magic**, independent control/co-op camera and the first meaningful jointly solvable encounter. Do not treat the observation as an instruction to implement the shoulder feature or a new skill tree. See [pre-magic Donut experimental design](../01-design/floor-01-pre-magic-donut-experiments.md).

**Still untested from Stage A:** specific coyote/buffer impressions, clipping, control feel per device, controller hotplug, A1 versus no-lighting, camera separation (requires B), frame pacing and 16:10 usability on hardware.

## Stage B — independent protagonists (requires a dedicated Rust + Bevy probe)

**Preconditions:** implement a separately owned, deterministic second actor/controller. Keep final character animation out of this probe if it would slow testing. The original 52/40px source masters remain mandatory.

| Scenario | Authority / participants | Key observation |
| --- | --- | --- |
| B1 | Solo Carl + AI Donut | Does she help rather than obstruct or require endless commands? |
| B2 | Solo Donut + AI Carl | Is controlling Donut fun even before magic? Can AI Carl keep up? |
| B3 | Solo with instant switch | Does camera/velocity/position stay understandable? Is swapping a strategic choice, not a chore? |
| B4 | P1 Carl, P2 Donut from frame one | Are both players active? Do physical controls remain fully independent? |
| B5 | Solo -> second player joins -> second player leaves | Stable ownership, no stuck held action, correct AI handoff |
| B6 | Two players move apart, then regroup | Shared camera doesn't excessively shrink pixels, hide threats or jump |

**Evidence:** deterministic action traces with player slots and disconnect/reconnect transitions; same replay state hashes; visible 16:9 and 16:10 Bevy captures, stable foot contacts and controller mapping; two human testers for B4/B6 when available. Test 1P and 2P on the same authored route. No two-human-only critical path.

### B7 — Character-gated traversal (first required two-role test)

Follow [the approved traversal/gating rule](../01-design/asymmetric-traversal-and-gates.md): **solo switching, not single-character sufficiency**, is the acceptance target. Build a tiny authored sequence with (1) a Donut-only high/far access opening a shared route and (2) a Carl-only weak barrier opening the next shared passage. These are **sequential gates**, not two equivalent paths. Donut's jump advantage and Carl's selective breaking are qualitatively approved; actual tuning values/material interactions are not.

Compare in three identical-start-state runs:
- **B7-1 solo:** use explicit switching and (where needed) a reliable companion command. Record number of switches, whether the inactive actor rejoins, and any unnecessary AI micromanagement.
- **B7-2 local 2P:** each player resolves their distinct gate, with useful participation/repositioning for the other and no prolonged off-screen waiting.
- **B7-3 safety/retry:** after one gate is opened, reverse direction, fall, restart, reconnect controller and apply supported selective destruction deltas. Both must reach the next common safe zone; test at each story-unlock state for which the gate is authored.

**Reject** an implementation that validates only Carl-alone and Donut-alone reachability, invents a mandatory second route, creates a non-recoverable stranded companion, or uses post-Guild abilities before they are unlocked. A successful gate should teach switching early and allow more creative answers later, not force a fixed character lane forever.

## Stage C — Shoulder Mode experiments (only after B)

**Core question:** does mounting make cooperation more interesting, or does it just conceal poor controls/camera?

| Variant | Experiment | Failure to watch for |
| --- | --- | --- |
| C0 baseline | Unmounted play, free two-character movement | Establish reference for agency/friction |
| C1 mount transport | Voluntary mount/dismount with shared movement and preserved P2 actions | P2 becomes a spectator; nobody ever chooses to dismount |
| C2 mounted attack | C1 plus Donut aims/casts only **already unlocked** abilities, with a solo contextual command candidate | Dominant permanent mounted strategy; too many buttons; player confusion |
| C3 advanced special | Optional later combination only if C1/C2 add real choices | Arbitrary power creep, lore/timing conflict |

Do **not** test or implement mounted magic in a **pre-Guild** fixture, where Donut does not yet have the corresponding story-granted capability.

**Questions:** who initiates mount in 1P/2P, can the other player refuse, what action remains for P2, can mounted Donut aim independently, does solo benefit from simplified commands, how do hitboxes/knockback/camera/interruptions work, can disconnection leave anyone stuck? Is each character more valuable free or mounted depending on the problem?

**Selection rule:** favor the *simplest* variant demonstrably increasing meaningful choice for both 1P and 2P. Reject a version that forces a second human to spectate, undermines unmounted play, hides hazards, or needs complicated input combos. No numeric pass thresholds before real measurements.

## Stage D0 — combat in the first playable integrated opening

The owner requested that [combat-feel and hybrid-physics tests](floor-01-integrated-combat-probes.md) be **embedded in the opening Floor 1 scene**. Reuse the planned **post-chase on-foot goblin encounter** rather than creating a separate permanent combat sandbox or requiring the Dozer to become a killable boss.

The same scene must support reproducible fixtures for:
- **CP-01 basic hit and ordinary goblin takedown** (first mandatory combat quality gate);
- **CP-02 knockback into another goblin** (initially stagger/position without automatic collision damage);
- **CP-03 one authored environment-driven defeat** (only when a story-available physical interaction and protected geometry are ready);
- **CP-04 defeated-body contact** as an optional A/B experiment, **not** required for scene acceptance.

Capture fixed-step traces, actual Bevy visual evidence, controller-play notes, restart and exit reachability. Record variants that do not exist as **N/A—not implemented**. Repeat CP fixtures with real 1P switching and 2P ownership when WORK-005/006 foundations exist; do not fake co-op validation with the current passive Donut follower. Normal story play should not surface debug fixture controls or a long forced training sequence.

## Stage D — first genuinely cooperative action loop

Only after Stage A/B foundations are understood:
- **D1 ordinary goblin:** readable tell, Carl alternative, Donut alternative, fair solo AI assistance; verify challenge in 1P and 2P.
- **D2 Murder Dozer pursuit:** large hazard, escape and geometry/turn-radius learning rather than early mandatory boss defeat; reachability and fail/retry.
- **D3 selective destruction:** demonstrate a **Carl-required gate** where breaking a designed weak obstacle opens the shared route (without insisting on a second bypass), then test an optional alternative or a late unlocked route on a separate fixture. Protected materials must remain clear; deterministic collision updates and solo party/2P reachability both required.
- **D4 later ranged synergy:** after a verified story unlock, give Donut a distinct ranged/control role to complement Carl, not an early spell.

These are **proposed fixture goals**. The first scene currently does not contain these enemies or gameplay mechanics even if the greybox separately demonstrates part of the technology.

## Human review worksheet — fill once per tested scenario

    Build/commit:
    Test date:
    Tester(s):
    Input: keyboard / controller name(s), number of players:
    View: 1920x1080 (3x), 1280x800 (2x), other:
    Platform/CPU/GPU, release/debug and vsync if profiling:
    Scenario ID (A1/A2/A3/B1..B6/C0..C3/D1..D4):
    Lighting mode: lit / --no-lighting:
    Character story ability state (pre-Guild / post-unlock / test-only):
    Result: PASS / FAIL / N/A NOT IMPLEMENTED / NEEDS DATA:
    Completion time, retries, stuck/clip events:
    Camera separation or off-screen moments:
    Controller ownership or control confusion:
    Feel (1–5) and why:
    Readability (1–5) and why:
    Enjoyment (1–5) and why:
    P2 agency (1–5 or N/A):
    Shoulder usefulness (1–5 or N/A):
    Thing worth preserving:
    Observation 1 + reproduction:
    Observation 2 + reproduction:
    Observation 3 + reproduction:
    Evidence link/screenshots/logs:
    Decision: keep / tune / separate spike / reject:
    Next workorder:

Single-tester 1–5 scores are reflection prompts, **not statistical results**. Objective measurements (e.g. frame interval median/p95, CPU/GPU, camera frames outside safe zone) need precise hardware/build/method notes. A Steam Deck-like 1280x800 viewport on desktop is not a Steam Deck hardware measurement.

### Triage

- **P0:** crash, impossible traversal, game softlock, player input cross-talk/stuck action, lost authority after hotplug, unusable camera, destruction removes required route.
- **P1:** repeated unreadable threat, serious camera jitter/snap, frequent companion obstruction, persistent painful controls, confusing ownership, misleading collision/silhouette.
- **P2:** subtle character staging, decorative palette, minor FX and polish.

Always classify reports as **existing-engine defect**, **unimplemented feature**, or **unresolved design choice**.

## Hypothesis ledger and decision gates

| ID | Question | First valid stage | Evidence needed |
| --- | --- | --- | --- |
| H01 | Does Carl movement/jumping actually feel responsive? | A1 | Human run + repeatable notes |
| H02 | Does lighting A1 improve atmosphere *and* keep hazards readable? | A2 | Lit/unlit same-state comparisons |
| H03 | Is 640x360/640x400 readable for true co-op at h52/h40? | A2 baseline, **B6 decisive** | Native-scale dual actor camera capture |
| H04 | Does Carl read as an ordinary antihero, rather than flawless action hero? | A2 + animation iteration | Human look and feel review |
| H05 | Is pre-magic Donut satisfying as P2? | B2/B4 | Co-op play session, player feedback |
| H06 | Does hybrid swap/command support beat manual micromanagement? | B1/B3 | Solo comparison, friction notes |
| H07 | Can camera support both players without forced miniature actors? | B6 | Diverse separation/join tests |
| H08 | Does transport-only shoulder mode preserve both players' agency? | C1 | Solo/coop side-by-side with C0 |
| H09 | Does mounted attack/assisted aim improve tactical choices? | C2 | 1P/2P vs unmounted baseline |
| H10 | Does destruction actually change player tactics? | D3 | Two-route setup, replay/choice evidence |
| H11 | Is the same authored encounter fair and enjoyable in both 1P and 2P? | D1/D2/D4 | Same-seed 1P/2P comparisons |
| H12 | Are **one-character-required gates** legible and satisfying, without forced parallel routes, for both a switchable solo player and a 2P duo? | B7 | Same-state gate scenario; solo switch/rejoin, 2P contribution/rejoin, post-destruction reachability |
| H13 | Can later earned abilities produce non-obvious alternate solutions without retroactive softlocks? | D3 + later unlock fixture | Unlock-state capability audit and seeded solution/replay proofs |

**No hypothesis is considered validated by a passing workflow.** Update with the observed result, evidence reference, designer verdict and follow-up action.

## Proposed independent workorders (do not start all at once)

1. **TEST-A:** owner plays current scene, logs issues; no new code unless a concrete bug is found.
2. **DUO-INPUT-01 / WORK-005:** independent Donut actor + two device slots; no abilities/Shoulder Mode yet.
3. **DUO-CAMERA-01:** shared camera that keeps characters readable; avoid coupling to input rewrite.
4. **DUO-AI-01 / WORK-006:** solo swap and context-assist test, deterministic.
5. **SHOULDER-SPIKE-01:** transport C0 vs C1 only; proceed to C2 if the human test warrants it.
6. **ENCOUNTER-01:** first meaningful goblin/Dozer/destruction slice after movement/2P foundations are usable.

Avoid overlapping agents in the player input ownership, camera contract, character master files or core collision schema. Follow [workorder process](workorders.md). New input/camera architecture needs an ADR if it materially changes the accepted existing contracts.

**Immediate playtest:** Stage A, about 10–15 minutes of intentional movement, visual review, lighting comparison and 3 concrete observations. This document does not make future work depend on completing Stage B/C right away.
