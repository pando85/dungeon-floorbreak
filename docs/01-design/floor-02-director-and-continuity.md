# F02-DIRECTOR-01 — Five-chapter staging, continuity, story and UX director's bible

**Status:** proposed authored presentation and progression design, 2026-11-22. **NOT implemented; no owner gameplay acceptance.** Paired references: [game bible](floor-02-last-lesson-design-bible.md), [enemy acting](floor-02-enemy-acting-bible.md), [S00/S01/S04](floor-02-arrival-cleanup-encounters.md), [Rev-Up and Krakaren](floor-02-revup-krakaren-setpieces.md), [Meadow and Rage](floor-02-meadow-rage-setpieces.md), [kennel and Ralph](floor-02-ralph-kennel-finale.md), the project's approved individual-viewer and no-inventory decisions.

## 1. Player-facing structure: one floor, five uninterrupted chapters

S00–S09 are source authoring beats, **not** ten separately advertised levels. On a first run the playable floor is experienced as a single linear campaign with five musical/lighting chapters A–E. Optional 5–15 minute checkpoint sessions are for replay, not a mandatory "Stage Clear" screen every 90 seconds.

| Chapter | Beats | Opening composition | Fight identity | Emotion at exit |
| --- | --- | --- | --- | --- |
| A — THE NEW RULES | S00–S01 | hard warm-to-cold seam, orange lichen, service counter | mobility vs committed psychic cone | player regains control in a disturbing facility |
| B — REV-UP GOES WRONG | S02–S03 | cheap golden sales expo opening to dark tendrils | real physical jug chain + clone boss | an almost comic corporate catastrophe |
| C — PEOPLE, NOT POINTS | S04–S05 | refuse conveyor resolving into warm wheelchair lanterns | body/environment routing, then protective defense | compassion and exposed vulnerability |
| D — THE IMPOSSIBLE ENEMY | S06–S07 | blinking restroom signage -> impossible black-violet mass | flee, cover and exploit a stairwell | exhaustion, real earned survival |
| E — TINY TERROR | S08–S09 | black-metal kennel/fightclub -> huge empty arena, tiny gerbil | rider dismount + tiny committed boss attacks | dark joke becomes quiet Mongo tenderness |

**Suggested first-clear pacing:** 25–40 min total, explicitly NOT validated. The priority is **impact per minute**, not reaching a quota. Slowdowns to save if needed: merge S00 into S01, tighten final part of S02, flow S04 straight into S05, merge Rage refuge/chase in one camera-driven sequence. Never shorten by deleting direct-combat agency or removing warning tells.

## 2. Transitions: no stop-and-start

- **A -> B:** broken customer-service number display clicks to 0, a ridiculous golden sales arrow lights on the wall. Player can move while audio perspective changes to a cheap commercial jingle. No mandatory dialogue/choice.
- **B -> C:** Krakaren dies; a small finite group of actual simulation grub clusters converges. The victorious business façade is overtaken by the cleaning machinery. Camera returns to stable combat coverage before S04 begins.
- **C -> D:** warm lanterns and convoy in a safe pocket; audio slowly empties, fluorescent ballast becomes dominant. A single warning beacon near a restroom uses recognizable institutional graphic language. The protected refuge is introduced *before* the high threat acts.
- **D -> E:** Rage is forcibly expelled into an **early stairwell**. **Crucial continuity rule: the heroes DO NOT descend to Floor 3 yet.** Their safe path is a Floor 02 side corridor connected to the kennel district; the Rage-only exile volume ends its threat and a separate locked/unselected player descent marker remains unavailable until S09 Ralph/Mongo. Playable route returns to Floor 02 after the protected pause. Do NOT advance the floor story ledger or show class selection here.
- **E -> Floor 3:** a completed Ralph boss grant -> small Mongo room -> true final descent. Only THIS transition may set floor_02_exited and queue future Floor 03 race/class presentation when supported.

**No contradiction:** A Rage-only stairwell trap and the real party exit are **two distinct authored progression triggers**, even if both use downward void visuals. Agent must implement in stage data and verify with a trace that Rage is exiled while Carl and Donut remain on Floor 02. A protected side walkway can be adjacent to the stairwell but should never look like the party already went downstairs.

## 3. Checkpoints and director ownership

Each chapter has an initial safe entry checkpoint and one or more encounter-local retries. Prefer existing Floor 01 checkpoint and safe-refuge contracts; all cutscenes authored as render-only camera/audio/presentation over deterministic gameplay state.

Proposed checkpoint boundaries:
- A0 after entry seam, A1 after first Horror.
- B0 before Rev-Up showroom, B1 immediately before Krakaren; B2 post-boss cleanup.
- C0 at cleaning factory entry, C1 before convoy defense, C2 safe-room arrival.
- D0 immediately before Rage warning reveal; D1 after discovering refuge; D2 after Rage exile / player rejoin on Floor 02.
- E0 before kennels; E1 before Ralph cage; E2 in Mongo room; E3 actual floor exit.

**Rules:** no checkpoint in the middle of an exploding prop chain, committed boss attack, convoy warning stop, Rage lunge or Ralph feint. On retry no duplicated score, viewer grant, dead NPC, hero special or enemy spawn. Animation/sound timeline restarts from a safe visual key, not raw inherited fractional time. Never respawn both heroes inside active hazard volumes.

## 4. Presentational beat budget (skippable/retry-safe)

- A entry: **~1–2 seconds of camera settle** while normal player control is already available. The striking visual seam is the key, not a forced walk.
- B Rev-Up pitch: **0–3 seconds** of sales joke embedded in enemy gesturing, no dialog box that masks targeting.
- B Krakaren reveal: **<=3 seconds first entry**, shortened on retry; the first attack still requires its full tell.
- C Meadow Lark: **3–6 seconds quiet scene** in a safe pocket with movement; the human stakes are portrayed by gesture/lanterns, never a compulsory escort briefing.
- D Rage: **3–5 seconds carefully paced reveal**, *skippable on retry* and never with a surprise active damaging hit.
- E Ralph: **2–4 seconds original theatrical reveal** (big gate / small cage), <=1 second retry. No long non-interactive joke at every failed attempt.
- E Mongo: **3–6 seconds quiet optional recognition beat**, then normal exit path.

All times are provisional **presentation-duration budgets**, not fixed gameplay timing or canon. Total unskippable cutscene time on a retry should be effectively zero. In 2P neither character loses action ownership during combat; if a reveal requires a short pose stop, buffer input and synchronize cameras without relocating actors.

## 5. Sound identity, no copied music

| Region | Bed | State-driven SFX | Silence/pause |
| --- | --- | --- | --- |
| Pale corridors | flickering ballast, empty hospital-scale ventilation | Horror inward inhale/pulse, office rail strike | after first psychic clear |
| Rev-Up | horrible original sales rhythm, bottle chime, public address | sling release, real jug rattle, alarm buzz, collapsing sign | 0.5–1 beat when clone breaches |
| Clone | heavy intermittent machine percussion and wet chamber resonance | left vs center tell distinct, real core recoil, last crash | instantly at actual defeat, then grub skitter |
| Cleanup | conveyor servo rhythm, metal bin bounce | cluster chew, pupa crack, Vespa acid spatter | warm convoy lamps signal change |
| Meadow | restrained human-scale tonal warmth | wheels/straps, protective hit on a wheel, defensive stop | safe-room breath |
| Rage | sub-bass pressure plus deliberately sparse warning intervals | huge tell/commit/blocked-refuge, mechanical launch cart | almost complete absence after exile |
| Kennel | original raw industrial/black-metal-ish percussion **without imitating recognizable music** | spear scrape, rider fall, kennel latch, metal speaker strike | ring introduction |
| Ralph / Mongo | comic stadium roar + sudden fast ticking rhythm, drops to near silence at defeat | scratch directions, rail bonk, small squeal, tiny pet footsteps | quiet Mongo room |

No vocal audio line should be required to know where an attack will land. Supports captions and reduced-shake/reduced-flash options where host settings allow. Source assets must have provenance/license just like sprites; there is no right to reuse a recognizable game/movie metal track.

## 6. Per-character viewers and arcade score

Existing game decision: individual viewer values, short combat rewards, floor-scoped modifiers and **no traditional inventory or gear grind**. Floor 02 should NOT invent a mandatory new HUD or different global currency. Player-facing viewer counters may be introduced at an **already-approved story milestone** only after HUD and progression integration owner review. Never silently award them from S00 if the real story flag says otherwise.

Recommended factual event hierarchy:
- ordinary direct hero kill: simple deterministic credit;
- legitimate knockback into another foe / prop collateral: attributable spectacle and short momentum credit;
- two-hero setup/finish: clearly attributable joint assist without inflating both player's individual kill counters;
- rare mechanically valid exceptional act: candidate brief cinematic death slow-motion, only when the existing rare-earned criteria are satisfied;
- chapter clear: safe optional results/individual viewer comparison, never mid-Rage chase or while a boss tell is active;
- Rage expulsion: unique narrative feat (optional one-off scoreboard event), **not** ordinary kill XP or farmable attack damage;
- Mongo: narrative outcome, **not** a combat/performance bonus or fake sponsor grant.

Retried checkpoint clears must not dupe persistent viewer milestones. Distinguish replay/testing resets from normal persistent progression.

## 7. Visual redirection budget (avoid camera being the boss)

Large figures may be placed in background and behind scene walls, but **only actual foreground hitboxes may hurt the characters**. Rare camera shake should last shorter than the next incoming attack warning, and must never hide a safety-critical Rage lane. Full native hero sprites remain readable even in 2P.

Director event states (suggested): reveal_started, threat_windup_visible, threat_committed, strike_resolved, stage_safe, boss_defeated, chapter_cleared, party_regrouped, true_floor_exit; authored IDs and event ordering must be replay-stable. The director schedules light, sound and camera cues; it does **not** resolve damage or move an enemy independent of simulation.

## 8. Continuity and arcade acceptance

| ID | What must be proven |
| --- | --- |
| DIR-01 | Five contiguous player-facing chapters but internal S00–S09 authored content loads correctly |
| DIR-02 | Any prior earned Floor 01 capabilities remain; no retroactive or early new skill grant |
| DIR-03 | Rage-only exile and hero-side Floor 02 continuation are **different transitions**; players do not leave for Floor 3 in S07 |
| DIR-04 | Mongo appears **after** real Ralph defeat, not from loading a boss fixture with all-grants |
| DIR-05 | True floor exit and Floor 3 hook occur after E, once, without checkpoint duplication |
| DIR-06 | Camera/audio director event ordering does not change deterministic physics/replay |
| DIR-07 | First-intro staging skippable; boss retry adds effectively no unskippable presentation |
| DIR-08 | All attack visibility valid with SFX disabled, all hazards audible with visual cues intact |
| DIR-09 | Viewer/assist/damage credit factual and non-duplicated on retry, valid solo and 2P |
| DIR-10 | At chapter boundaries both heroes regain free movement and camera exits constrained composition |
| DIR-11 | Caption/reduced-effects checks where supported; no flicker-based puzzle requirement |
| DIR-12 | Owner actual full-floor pacing + two-pad feel: NOT RUN; no auto-pilot claim of fun |

**Implementation:** prefer existing director/scene events, story ledger, progression flags, safe-room transitions and HUD. Write a versioned table mapping every trigger to actual current Rust and level source types **before** writing another global manager. Audio/lighting can be independent work packages; only one integrator writes critical scene transition files.

**Canon research:** secondary source references https://www.dccfansite.com/entry/floor-two/ and https://dungeoncrawlercarl.wikioasis.org/wiki/Second_Floor, with exact primary-text chronology pending editorial pass. Everything here beyond named high-level story beats is an original game staging and UX hypothesis.
