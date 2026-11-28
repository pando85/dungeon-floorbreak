# BSW-FINALE-01 — Local implementation-agent work order

**Objective:** implement the **actual final Floor 01 boss encounter** for the approved Ball of Swine and Tusklings, with a big, physically believable rolling enemy, a guaranteed playable direct-fight baseline, optional player-authored gate impacts, bounded detachments, solo/local 2P and a real narrative floor exit. **The linked [full gameplay design](../01-design/floor-01-ball-of-swine-finale-encounter.md) and BSW-01..20 acceptance matrix are authoritative for this task.** This PR begins with **documentation only**; the first agent's work is implementation, not another speculative design pass.

## 0. Mandatory dependency audit (before touching hot files)

1. Read current `AGENTS.md`, agent orchestration, `docs/07-agents/f01-playtest-integration-plan.md` and its in-flight revision #190, active shared-file ownership, and the actual merged engine version.
2. Confirm source art statuses. Existing **SWINE-01** sprite is **128px diameter** with **native 176×144 atlas frame** and six pre-rasterized roll frames; **TUSK-01** is **30px tall**, 56×40 frame, tux/gown variants. Original editable art exists in `assets/source/characters/swine/` but its Bevy instantiation/runtime AI and production asset review are not yet claimed. Use native source/manifest pipeline, no runtime rotation/scale or concept PNG as collision/sprites.
3. Check whether `ENEMY-SLOT` (sidecar actor registration, one-shot hit/damage/defeat, correct playtrace and companion feed) and BOIL's shared env prop/kinematic collision service have **actually merged**. If not: implement isolated minimal physical core/test fixture in your owned module and leave level wiring gated; do **not** rewrite or duplicate these global services.
4. Verify late-floor story flags. **Juicer defeat** grants Carl's gauntlet and Donut's Puddle Jumper; this fight starts only after those flags in story mode. Use separate **test-only** `all` loadout if current harness supports it, never inject skills in story scene. **`f01_late_mastery` is currently a wave-test arena**, not the Ball. A new scene entry (candidate `f01_ball_final`) and Floor 01 completion pin need coordination with current integration owner, not a silent replacement.
5. Cross-check actual Carl jump height (~92px base), Donut jump/blink distance, 640×360/400 camera and co-op separation logic. **A 128px rolling boss is NOT normally jumpable by Carl**. No scripted safe route may rely on that impossible input.

## 1. Implementation work plan (stop at real playable evidence at each gate)

| Slice | Target implementation | Pass gate / proof |
| --- | --- | --- |
| **BSW-A — 128px greybox** | Data-authored 1200px provisional side-view floor, 2 true wall-protected side refuges, one broad central lane, optional bypasses, protected entry/exit. Spawn a correct-sized inert Ball shell from genuine source sprite or accurate collider proxy. | Real path-reachability graph + 1× Bevy shot (640×360/400). Human Carl and Donut, solo and P1+P2 can evade the hull without 128px jump; camera/scene loader coherent. |
| **BSW-B — direct boss first** | Fixed-step `tell→committed roll→swept collision→rebound→recovery`, honest boss hurtbox, damage from melee/spell and earned kit, one legitimate `BossDefeated` event. No gates, no Tusklings. | **Playable direct-only boss clear** at true speed; deterministic 1P/2P fixtures. Move directions lock after tell. No unbounded rolling, hidden invulnerability or original sprite runtime rotation. |
| **BSW-C — primary gate** | One visible central vertical gate with warning, bounded up/down, real collision/bounce/stagger on the 128px Ball; no compulsory timing button. Optionally one **noncritical** breakable side support (after gate is fun). | Actual gate hit and deliberate miss, no checkpoint/exit obstruction, all gate states safe for separated 2P. Reuse BOIL service; otherwise keep gated and report. |
| **BSW-D — Tuskling identity** | `TUSK-01` **30px** tux/gown actor using same enemy-slot sim and genuine 31-tick pre-contact gore; finite detach budget from *real hard impacts*. | Max 3 simultaneously live, 4–6 total initially, legal collision-safe spawns, 1–2 per qualifying impact. Separate active HP actors from 30+ decorative faces. Actual 1× readable kill. |
| **BSW-D2 — optional kick discovery** | Carl launched live Tusklings can stagger Ball **only** when a verified source-linked collision exists in shared physics. | Ball staggers exactly once per genuine contact; default enemy-to-enemy damage remains 0 until separately adopted. If unsupported, clear **blocked/deferred**; encounter still winnable. |
| **BSW-E — camera and spectacle** | Native pre-rasterized roll frames, debris and dust, impact sound, restrained special UV/laser accent and a **camera experiment**, not an assumed global ADR rewrite. | Real 1× baseline, 1.25× experiment, 1.5× research capture on 1080p and 1280×800. Motion readability and render-policy decision documented; sim hashes unaffected by effects. |
| **BSW-F — story conclusion** | Scene-adapter/ledger seam from Juicer → rest → Ball → Floor 01 exit, defeat exactly once, protected regroup, no extra unlock. Preserve existing `f01_late_mastery` fixture. | Story flags and checkpoint/restart persist, exit reachable after optional props break, local 2P joins, no dummy `all` loadout leaked. |
| **BSW-G — QA and feel** | All BSW-01..20 scenarios, headless exploit sweeps, Playtrace, actual gamepads, 3 genuinely distinct clear styles and post-boss transition evidence. | Machine and owner-human signoffs explicitly separate; no `PASS` claim for a fixture not actually executed. |

**First essential shippable milestone: BSW-A+B**, even when the physical gate interface, production Tuskling AI or final story transition are blocked. A direct-only boss fight that is fair and fun is a dependency, not a fallback embarrassment. Never spend multiple days polishing a gate animation before the rolling body can be defeated through honest damage.

## 2. Data/state contract for the actual Rust code

Implement with current engine conventions; names can adapt but semantics must survive.

### Authoritative boss

- Fixed tick: **60Hz**. Game sim states `Dormant`, `Roam`, `RollTell`, `CommittedRoll`, `Impact`, `Recover`, `Stagger`, `Defeated`; no automatic 3-phase HP lock.
- **128px art diameter**, collider initial nominal **radius 60px**, from existing SWINE-01 placeholder (collider/hurtbox tuned only with real visual fit). The visual's rim lumps are mostly non-contact silhouettes; **never** derive damage by per-frame opaque pixels or sprite rotation.
- SWINE-01 authored **36-tick roll tell**, and six native roll frames (3..8). Its 3/6/24 frame action sequence is **not** a 33-tick complete travel timer; actual travel duration is an independent fixed-step physical state. Candidate roll 220–280px/s, commit 320–480px maximum, normal 45–75px/s; choose data knobs and document.
- Collisions are **swept** with stable ordering. Same tick wall/gate hit must stop/rebound, apply one response, optionally emit bounded detach and change authorized structure state. **No secondary hit or detach spam when the Ball remains touching the wall over several frames.**
- Maintain real boss HP and direct damage using existing combat events. Initial **20–24HP hypothesis**, gate impact **3–4HP** if included; adjust for measured direct-only and mixed clear duration, not assumed challenge. Standard spells are not disabled to make scenery necessary.
- Gameplay event source chain for kick/bomb/gate impacts and Ball defeat; independent of VFX/sound/camera.
- Deterministic load/room-state hashing, replay, retry and event IDs. No one-shot tokens reused after restart.

### Detached Tusklings

- Data slots for costume (tux/gown) and shared **TUSK-01** attack behavior; no unknown knight/courtesan classes or poison.
- At most **3 living simultaneous** and initially **4–6** independently detached over the fight, **1–2 per true impact**; different variants are cosmetic, not new damage classes. Test active-cap hit and deferred spawn/no-op clearly.
- Spawn from a qualifying **actual impact location** using validated safe authored offsets; no hero overlap, outside colliders, no free offscreen contact.
- A **30px sprite can become tiny under 1.5× camera coverage**; don't remove readability by zooming for spectacle. Test native and handheld output before permitting medium zoom.
- The remaining 30+ count is **fictional visual composite**, not a requirement to simulate a 30-body swarm.
- Optional kicked-body→Ball contact is a **stagger if physically supported**, not an unapproved game-wide enemy-to-enemy 1HP bonus.

### Props and exit

- Central `gate_raise / warn / descend / down / ascend` has an honest hazard footprint, reversible timing and actual collider. Start with **one** gate, add one optional support only when the direct fight + gate loop is satisfying.
- Every safe alcove has real room for heroes, **solid geometry stopping the Ball** and a tested independent exit/backtrack route. A 64px floating platform with no ball-stopping wall does **not** count as safety.
- Gate/outer-support state cannot destroy the only safe ground, checkpoint, party regroup or post-defeat exit. No two-human simultaneous gate ritual.
- A true boss defeat sets **one Floor 01 completion event** and opens a protected right exit; remaining Tusklings can survive without a forced 30-pig cleanup and never spawn attack-overlapping the exit.
- Floor completion flag is a **story integrator change requiring review**. Never reuse `f01_late_mastery` debug pin as narrative boss result.

## 3. Agent-run adversarial AUTOPLAY tests (mandatory)

**The agent must implement AND execute this evaluation, inspect failures, adjust within its owned scope, rerun, and attach before/after evidence.** A test-only spec with no execution is not done. Start with real Rust `PartyFixtureSim` or current actual scene domain, rather than inventing Python fake ball physics or silently assuming party `autoplay evaluate` exists.

Implement at least:
- `carl_direct`: legal punch/kick/gauntlet, attacks during actual recovery, no gate or dynamite required.
- `carl_gate`: legal positioning/inputs cause real Ball→gate impacts; show a **miss** also.
- `donut_mobile`: legal Magic Missile press edges, jump/short blink where valid, reposition to avoid committed roll; respects actual mana.
- `donut_stationary_cast`: search **many natural reachable x positions** and legal direct-shoot lines; stationary clears with low risk are a **failure** even if other positions lose.
- `duo_split`: Carl and Donut real per-tick dual inputs, try one on detachment control and one on Ball, with up to 448px leader separation test.
- `no_gate`, `gate_missed`, `early_gate`, `late_gate`: all must produce coherent simulation and eventual legal progress/retry.

**Metrics per run**: actual tick count, boss HP/damage breakdown, attack tells, roll impact velocity/position, gate contact, detached/active Tusklings, heroes' HP and movement distance, missed vs valid hits, source owner, boss invisible-at-warning count, camera range, number of consecutive idle ticks/waits, post-defeat exit reachability, seed/config/input/hash, companion damage breakdown.

**Pass vs fail gates**:
- **Direct-only** player can clear the boss without gate damage, no boss HP lock.
- **Static exploit** from a naturally reachable safe spot must not beat the boss reliably while taking negligible risk; solve via safe-nook **LoS, approach options and tactical attack commitment**, not by nerfing Magic Missile or giving invisible armour.
- Every committed Ball charge has at least **one physically attainable hero escape route at actual movement speed**; jump-over-128px is never the sole answer.
- **Same-input replay hash** and gameplay event sequence consistent across two runs, 1× and effects-off render.
- **No softlocks** for every gate/outer-support state, two separated human players, and all detachment budgets.
- **At-speed solo** Carl, solo Donut and actual two-controller-human run. A bot pass cannot claim fun/hero skill balance.

**Output:** actual runnable Rust commands, JSON/CSV row data or current standardized artifact, short human-readable report, recorded fixture/run IDs, real Bevy screenshots/video and honest PASS/FAIL/BLOCKED/NOT RUN status for BSW-01..20. Document unknown/unimplemented parts explicitly.

## 4. Real-playtest and art gates

- **Three at-speed success routes:** (1) direct fighter, (2) physically timed gate impact, (3) 2P split threat, plus one meaningful failed/learned dodge. Capture actual live input/playtrace automatically per existing project contract.
- Evaluate first-time visibility of the rolling threat, fear vs fairness, recover length, minion crowd clarity, direct-only vs gate-clear durations, camera whip during long roll and 30px costume visibility.
- Compare real output **640×360, 640×400, 1080p and 1280×800**; inspect **1×** baseline and **1.25×** camera experiment. **1.5× never assumed approved**, and source sprite stays fixed 128/30px.
- Native sprites from `assets/source/characters/swine/swine-01.*`, `.../tusklings/tusk_01.*`, reviewed manifests/export scripts. Respect approved `assets/concept/floor-01/ball-of-swine-arena/04-gameplay-layout-preferred.png` as **visual reference only**.
- Drama/cinematic: no routine slow-motion at ordinary kills; exceptional player-caused finish may invoke existing CINE-01 policy if compatible with two-human gameplay. No show-laser damage absent real authored hazard contract. No final progress UI not yet present in narrative.
- Human owner playtest is an independent acceptance gate; no ML, scripted fixture or screenshot can label the encounter "fun" automatically.

## 5. Scope boundaries and coordination

**Do not include** a rewrite of the Juicer boss, generic inventory/gear, new player input, a full moving ring-physics maze, 30 individually simulated Tusklings, camera architecture replacement, new local companion controller, new spell, auto viewers/patron economy, bespoke collision-damage exception for all enemies, production publishing/license assumptions or next-floor gameplay.

**Integration conflict to resolve explicitly:** the merged #182 plan currently exposes **Juicer + `f01_late_mastery` waves** as its supported final testing sequence. This planned Ball boss is a **new narrative scene after Juicer**, not the wave arena. Keep both; add an integration amendment in the appropriate PR/owner once BSW-A/B is stable. In tests the Ball scene may temporarily launch with an isolated **post-Juicer capability fixture** but must be labelled test-only.

**Ticket for local agent**: begin with **BSW-A+B (direct-only playable rolling boss)**. Split gates, Tusklings, camera and story wiring into independent implementation PRs if current hot-file ownership demands it; do not assume an in-flight dependency is already merged. Provide a short, concrete integration-blockers table and prioritize useful partial runnable deliverables over sprawling unverified changes.

## 6. Final completion summary (required)

- Commit(s), exact runtime entry, props/enemy IDs, generated/reused source art and provenance.
- BSW-01..20 test states and commands, meaningful simulation evidence, versioned tuning values.
- **Direct-only vs gate-assisted vs co-op** duration/damage/state comparison from real gameplay.
- Detached counts & per-player credit; finite budget and kill/transition behavior.
- True solo Carl, solo Donut and local 2P camera and controls; physical escape correctness without jumping the Ball.
- Final Floor 01 exit/state continuity, while independent mastery fixture remains unaffected.
- Frame times, 1080p/handheld readability, effects-off gameplay equivalence.
- Open limitations, automation unsupported features, code reviewer result, green final CI and **explicit owner game-feel approval pending until played**.

**Definition of done:** A human can play **post-Juicer lead-in → learn/bait/defeat actual 128px Ball → face a bounded number of true 30px Tusklings → exit Floor 01 safely**, in solo and local co-op; real machine and human tests confirm meaningful decisions, replayability, legibility and no-cheat results. This document does **not** claim that has already happened.
