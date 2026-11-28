# F03-PROD-01 — Over City production roadmap and autonomous preproduction tasks

**Status:** Floor03 concept design / agent workorder, NOT implemented gameplay, not approval to rework F01/F02, 2026-11-22. Intent: seed **parallel preproduction** while local agents continue the much higher-priority Floor02 PR #239 and owner Floor01 game-feel tests. Do not compete for shared hot files or trip resources that should be spent on the first connected playable Floor02.

Read [F03 game bible](../01-design/floor-03-over-city-design-bible.md), [F03 art](../02-art/floor-03-over-city-art-direction.md), [F02 architecture audit (PR #239, unmerged)](https://github.com/pando85/dungeon-floorbreak/blob/design/f02-last-lesson-bible/docs/03-tech/floor-02-engine-reuse-and-route-contract.md) (currently draft PR #239), [F02 autonomous workorder (PR #239, unmerged)](https://github.com/pando85/dungeon-floorbreak/blob/design/f02-last-lesson-bible/docs/07-agents/floor-02-vacation-execution-packet.md) (on another branch until merged), AGENTS.md and current repo ADRs. If F02 docs are unmerged, read PR files directly; do not assume the references already exist on main.

## Circus implementation addendum — workstreams that can proceed separately

Read [F03-CIRCUS-01](../01-design/floor-03-circus-encounters-and-stagecraft.md), [F03-GRIM-01](../01-design/floor-03-grimaldi-resolution-and-coop.md), [F03-CIRCUS-ART-01](../02-art/floor-03-circus-visual-storytelling.md) before delegating the S02–S06 work. These are **proposals, not existing source code**.

| Workstream ID | Deliverable | Parallelization | Hard stop |
| --- | --- | --- | --- |
| F03-CIR-A | pure lemur/clown/lion attack tells + truthful hit/collision tests | can run isolated from Floor02 runtime with independent new modules | must not alter shared combat/party/hero kit before owner-approved F01 baseline |
| F03-CIR-B | real S02 awning support + original greybox geometry | authored stage data and existing `env_prop` adaptation | direct/no-prop route works, no fake hit or destroyed main path |
| F03-CIR-C | Heather rooted/rolling AI and 2-hero reachability | pure boss before Bevy hookup | somber mercy ending, no human Donut loss of controls |
| F03-CIR-D | Grimaldi 3-ring defense and nonlethal `communication_open`/`grimaldi_resolved` states | independent story director and scene data; integrator alone touches critical router | no boss HP kill; all supports optional; no 3-pole fixed-order puzzle |
| F03-CIR-E | four native pixel art master keys + original acting frames | may run immediately after independent silhouette gate | not a downscaled AI concept; rooted vine silhouette, approved heroes |
| F03-CIR-F | actual party/renderer/replay and source-level evidence | depends on Floor02 generic route + owner baseline | strict real game evidence, no false 2P or owner-fun claim |

**Current priority stays Floor02 S00–S03 gameplay**, not rushing a second floor before the first is playable. The first coherent Floor03 slice remains S00/S01/S02; art keys and pure isolated bosses for later scenes may proceed without touching shared F01/F02 files. Sourcebook chronology verification is the responsibility of F03-00 SOURCE, not a blanket approval from secondary recaps.

## Skyfowl/Remex production addendum — second half of the campaign

Read [F03-CITY-01](../01-design/floor-03-skyfowl-city-and-quill-encounters.md), [F03-DOOMSDAY-01](../01-design/floor-03-doomsday-scenario-final-act.md) and [F03-CITY-ART-01](../02-art/floor-03-skyfowl-and-doomsday-art-direction.md) before implementing S07–S12.

| Work ID | Safe independently-progressing task | Exact stop condition |
| --- | --- | --- |
| F03-CITY-A | original skyfowl facade, alley, Apito, Quill and Remex concept keys + 1x readability masks | no actor art/rights approval inferred from concept PNG; original source provenance required |
| F03-CITY-B | pure Krasue + city elf tells/real hitbox and recovery state tests | no floating unreachable enemy / cosmetic self-damage |
| F03-CITY-C | S07 public guard / civilian safe-lane scenes and S08 GumGum death aftermath | no secret guard instant-death, no living GumGum escort or viewer victim farm |
| F03-CITY-D | S09 two-role Apito battle + optional real platform fall | direct combat and all optional supports destroyed keep exit viable |
| F03-CITY-E | S10 Quill office/story state + legitimate blast/source event and Burgundy reveal | **Miss Quill dead before Remex**; Featherfall never a live boss; no actual physical explosive tutorial |
| F03-DOOM-A | pure S11 Remex story/crystal case states and separate second countdown | Remex no lich HP, first blast contained, later Remex hazard remains |
| F03-DOOM-B | four ordered precursor states and actual stage-source hazard data | no inventory/skill removal, no early/duplicated pulse or inescapable hit |
| F03-DOOM-C | S11/S12 real 1P/2-slot party/story run + Katia relay and final Floor04 event | Floor02 Rage exit / Floor03 early revealed stairs cannot transport players; strict replay + real render evidence |
| F03-CITY-Q | per-scene AUTOPLAY/PLAYTRACE probes and negative fixtures on final integration head | actual output evidence; real two-gamepad enjoyment remains NOT RUN |

**Source gate:** source-book events here are established by secondary fan summaries, not a primary licensed read. GumGum's murder precedes the Krasue investigation; Miss Quill dies from Carl's earlier office demolition and is **not** a second final physical boss; Remex's body is a separate wild-magic danger **after** the case contains the crystal. Escalating four magical precursor pulses precede Floor04 descent. Story and game adaptation deviations must remain documented.

**Priority:** Floor02 connected first, then Floor03 opening/circus playable slice; City/Remex preproduction art and isolated state tests may run in parallel without touching shared F01/F02 hot files. No agent can mark Floor03 complete from a coherent art storyboard alone.

## Cross-floor continuity: one integrator, not two campaign routers

[CAMPAIGN-BRIDGE-01](../03-tech/floor-03-campaign-bridge-acceptance.md) is the authoritative **proposal and acceptance test matrix** for Floor01→02→03→04 story/ability carry, nonlethal Grimaldi, false early stairs, true party exits, Quill-before-Remex and two separate explosive crises. Read it before implementing scene route, checkpoint, plot flags or Floor03 class identity. Existing Rust `UnlockLedger` owns abilities but does not automatically constitute a generic narrative flag system; the bridge contract explicitly calls for an API audit and minimal tested extension. **Do not build a second, incompatible F03-only route manager while Floor02 integrator is working.**

## 1. Hierarchy of work

**P0:** Floor01 owner game-feel/accepted core, Floor02 S00–S03 actual connected combat, camera/input and replay.
**P1:** Floor02 remainder and complete E2E, autonomous QA.
**P2:** Floor03 original visual exploration, narrative continuity / canon-source checks, pure scene geometry proposals and isolated enemy key designs; does not require shared runtime changes.
**P3:** Floor03 playable implementation **after** the engine's floor-neutral scene route and a successful Floor02 vertical slice, otherwise stop or deliver pure testable actor modules only if no overlap risk.

**No imaginary deadline:** one-week agent absence is not proof 3 floors can be built. Schedule by stable gates and reversible tasks; return packet must report what exists, not "almost done".

## 2. Six chapter build order (thirteen internal beats)

| Internal | Stage | First-pass gameplay proof | Primary art proof | Dependency |
| --- | --- | --- | --- | --- |
| F03-S00 | Guild / class identity | idempotent short story grants and no RPG inventory/menu overload | guild arch / class cards at 1x | F02 story/ledger route |
| F03-S01 | Ruins / knife lemur | direct solo Carl/Donut, 1 ranged tell + one authentic hanging-sign collision | huge false-sky ruin, first hero-scale camera | scene loader / validated enemy slot |
| F03-S02 | Circus road | one clown + 1 knife role overlap, moving awning support | billboard / truck, melee target | physical property support |
| F03-S03 | Ink and Mold | lion physical state plus optional story-only ink silhouette | lion+flat ink visual | F03-S02 plus distinct role exposure |
| F03-S04 | Heather | 2–3 readable large boss attacks, direct both-hero path | real bear grief/parasite keys | F03-S03 + 2P adapted story |
| F03-S05 | Big Top / Grimaldi | finite defender arena + authentic partial tent failure + nonlethal story resolution | giant vine center, physical ring | F03-S04 + story event contract |
| F03-S06 | Mongo vigil | short small cleanup wave, narrative guard visible | approved Mongo growth source | F03-S05 |
| F03-S07 | Skyfowl gate | distinguish safe guards vs attack zone, no instant unwanted city punishment | monumental wings and city gate | chapter crossing |
| F03-S08 | Krasue alley | flying attack reachable + genuine swinging-lamp collision | Krasue pose and wounded city | F03-S07 |
| F03-S09 | Cult/education center | physical lift / two role combinations, protected civilians | stage-set cult facade | F03-S08 |
| F03-S10 | Magistrate tower | controlled demolition plus direct clearing, Katia safe cameo | filing office / broken tower | F03-S09 |
| F03-S11 | Remex capacitor | limited last defense + one once-only story crisis event | glass-case / captive soul rig | F03-S10 |
| F03-S12 | Blast / Floor04 exit | 3 short pulse-escape pockets, real team camera, valid floor04 transition | false sky overwhelmed by light | F03-S11 and full story ledger |

The source book includes an earlier circus-site Floor04 stair. For this linear adaptation it is **a visible but narratively deferred exit**, with only the final emergency stair transitioning the player. The F02 Rage-only exile, F03 early revealed stair and F03 final stair must remain separate IDs and semantics.

## 3. Important owner decisions — do not treat as approved

- **F03-D1 canon-fixed race/class sequence:** proposed 60–120s skippable story sequence with existing actions, **not** alternate selectable builds. Owner may request real choice later. Don't implement broad class selection now.
- **F03-D2 Donut playable in Heather:** this deviates from book's Carl-solo scene but retains local 2P and AI companion value. Owner should approve or ask for a carefully scoped alternate.
- **F03-D3 Grimaldi resolved nonlethally:** defend and stage-breaking action followed by an earned story beat; no ordinary boss HP defeat and no repetitive Rage-esque forced flight.
- **F03-D4 Mongo and Katia roles:** both present as authored story helpers, but not a third or fourth permanent AI/camera/command architecture until separately tested. The owner may approve localized behavior as companion roadmap matures.
- **F03-D5 class-era power:** don't automatically teach Donut Clockwork Triplicate or Carl Fear to all players; these are canon inspirations requiring a separate gameplay proposal and existing equipped-special policy. No new button.
- **F03-D6 length:** 40–55 min first clear as a hypothesis; prefer 6 tight chapters with 13 internal beats, not 13 checkpoint-result panels.

**Hard constraints already accepted:** four face verbs, Carl kicks not grabs, asymmetric heroes, no inventory/gear treadmill, individual viewers, rare earned cinematic kill, physically authoritative interactions, long game art-readable sprites, normal attacks stay fun, one linear campaign.

## 4. Initial parallel agent deliverables

Each task should have its own worktree/PR, no broad shared repo edits.

| ID | Priority | Scope / read | Output | Done condition |
| --- | --- | --- | --- | --- |
| F03-00 SOURCE | P2 | secondary book2 timeline, verified primary notes if available, IP rights | a compact canon ledger: true / adapted / deferred / unknown, no copied long text | every scene major actor/order grounded, disputes flagged |
| F03-01 MOOD | P2 | F03 art bible + approved F01 hero art | 3 original keys: fake-sky vista, circus road, skyfowl/volcano; 1x camera crops | top-tier review of 1× read and original editable source, rights manifests |
| F03-02 ENEMY | P2 | first ranged and circus roles + art routing | lemur/clown/Mold Lion silhouette candidates and 1 tell/1 hit key each | original 1× contact sheets beside Carl/Donut, separate art review |
| F03-03 ROUTE | P2 | current room parser and F02 route proposal | versioned chapter stage/connection table (design-only until F02 integration) | no future ability leak, protected path, no Floor04 early trigger |
| F03-04 CLASS | P2 | approved controller + ADR-0010 + narrative race/class | short storyboard / event proposal for fixed canon identities | accepts skip and retry; no unapproved class RPG feature |
| F03-05 PHYSICS | P2 | real F01 env_prop + F02 jug design | one pure/testable circus awning/support source hypothesis (no duplicate simulation) | direct no-prop route and actual body-contact alternative |
| F03-06 FIRSTPLAY | P3 | F03-00–05 and F02 route actually live | S00/S01/S02 playable first slice in Bevy | full 1x render, one direct/propreaction strict replay, 1P and scripted 2-slot |
| F03-07 CIRCUS | P3 | firstplay, bear art, story gates | S03–S06 + Grimaldi resolution | genuine direct combat, 2P kept active, actual source event timeline |
| F03-08 SKYFOWL | P3 | circus, attack/guard boundaries | S07–S10 | no arbitrary safe-zone deaths, real collision, camera and companion tests |
| F03-09 DOOMSDAY | P3 | skyfowl, linked campaign, Playtrace | S11/S12 and true Floor04 exit | no stale floor triggers, separate from F02 Rage escape, short play trace |
| F03-10 REVIEW | ongoing | each asset/scene | independent code-art-rights review | explicit pass/fail/NOT RUN evidence |

**No mass asset creation** until F03-01 establishes and the owner is comfortable with the overall floor visual look. Similar circus materials can reuse palette/texture ideas but do not copy copyrighted video-game or other DCC artwork.

## 5. First-story controller and companion mechanics

**Guild event** is not a free-choice class builder. Each established hero label and one constrained gameplay implication can be shown on compact in-world panels; only an existing installed ability or separately reviewed action grants any gameplay effect. Carl's Primal identity is not an excuse to change hero model shape or base attacks. Donut remains a CAT after Former Child Actor; no human stylization. Mordecai's manager effect changes narrative availability but does not create a guaranteed third party AI.

**Mongo:** approved growth art is pink/red feathered raptor with blue crest accents. A small story-only room-local assist is allowed after pure simulation and owner review; never add Mongo as a third player or pet leveling stats UI without a prior decision. The stage author can show ordinary animation, rest, watch and protect without simulating another autonomous combat agent.

**Katia:** cameo/one room-local scripted blocking assist, not a second companion mode. An NPC who can be hurt must have honest source ownership and replay, and not an invulnerable helper that auto-wins encounters.

## 6. Acceptance and honest report format

One per-PR evidence card: owner design status; code commit; exact commands actually run; F01/F02 goldens preserved; scene/story/data pin IDs; direct and optional prop replay; 1x in-engine still + one game-speed clip; party mode support; performance/caps; known inaccessibility; art source/provenance; human 2P/game-feel **PENDING**.

**Mandatory negative tests:**
- replay never grants class/capability before the real F03 Guild; retry never grants twice;
- no huge silent boss hurtboxes, Grimaldi core not ordinary kill XP;
- without awning/sign/lamp prop, normal encounters still completable;
- in Heather co-op adaptation both humans keep active hero input and can contribute;
- at S06 the early Floor04 stair remains nontransitioning and cannot be exploited;
- finale true exit fires once after story event and carries both heroes, never in F03-S07 or F02-S07;
- training bot never claims human fun; supported AUTOPLAY policy uses observation not hidden future states.

**Return packet:** human time should be spent testing **Floor02 first** (the playable gate), then reviewing 3 original Floor03 art keys and the short fixed-class sequence. If Floor03 has real playable scenes, record their exact launch/replay commands; if only art has been completed, do not report stage progress.

## 7. Implementation caution

Existing `assets/levels/f01_late/manifest.json` is a late-floor launcher, not an end-to-end multi-floor campaign. Existing `src/env_prop.rs` accepts authoritative projectile/melee/body/blast stimuli, but a scripted carnival art event is not damage by default. Existing autoplay and playtrace may need F03 reset/observation additions. **Never create a toy second game loop in Python or script stage success by flipping a flag when a particle appears.**

Conformance to repo quality rules: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `python3 scripts/check_docs.py`, and art/asset validators when appropriate. These commands are an implementing-agent checklist, **not tests executed by this design PR**.

## 8. Autonomy-oriented first agent prompt

> Treat this PR as Floor 03 **concept design only**. Prioritize the owner-tested Floor 01 and Floor 02 playable vertical slice. Start F03-00/F03-01/F03-02 as separate, non-conflicting preproduction tasks: source ledger, original Over City visual keys and silhouette/attack-readable first enemies. Preserve approved Carl/Donut/Mongo source identities and all existing controller/rights rules. Do not refactor shared Rust, modify Floor 01 goldens, add new class/inventory UX or promise playable Floor 03 until the Floor 02 stage router and first physical setpiece are real and tested. Record exact file/source provenance and give one concise status with gaps for owner review.
