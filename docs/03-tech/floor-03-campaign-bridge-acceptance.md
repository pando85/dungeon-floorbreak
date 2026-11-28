# CAMPAIGN-BRIDGE-01 — Cross-floor progression and acceptance tests

**Status: design and test contract only; no runtime implementation or owner playtest. 2026-11-22.** Extends the [Floor03 bible](../01-design/floor-03-over-city-design-bible.md), [Floor03 agent roadmap](../07-agents/floor-03-design-to-production-roadmap.md), and the [Floor02 director contract in PR #239](https://github.com/pando85/dungeon-floorbreak/blob/design/f02-last-lesson-bible/docs/01-design/floor-02-director-and-continuity.md). It does not imply that the design PRs are merged or the story boundary code exists.

## 1. Verified engine reuse boundary

Existing main has deterministic IP-neutral `StoryRuntime` / `UnlockLedger`, once-only capability receipts, `CombatState::story_snapshot` / `carry_story` between room relaunches, a versioned capability table (`assets/data/gameplay/story_unlocks_v0.json`), two-slot simulation (`src/party_scene.rs`), and source-pinned `src/playtrace`. The `src/f01_late.rs` loader is a **static late-room launcher**, not a complete beginning-to-end campaign.

**Do not conflate capability grants with narrative flags.** Current `UnlockLedger` handles abilities; the implementing agent must audit whether the current API can store plot progress. If not, add a narrow tested story-event ledger/adapter, rather than storing arbitrary boss/plot events as fictional equipped abilities. No parallel simulation, physics or reward engine.

## 2. Four types of state, four owners

| State | Authority | On retry | Safety requirement |
| --- | --- | --- | --- |
| Earned hero actions | current capability ledger | retain legitimate run grant | no future spell/class/stat auto-grants |
| Boss/plot events | narrow authoritative run-event extension (proposal) | idempotent run receipt or legitimate checkpoint carry | no duplicate narrative/viewer rewards |
| Local enemies/props/hazards | deterministic scene sim | restore safe declared checkpoint state | no mid-attack spawn, cross-scene hazard ghost or exit softlock |
| Camera, art, sound, score presentation | render and derived factual event credits | restore harmless presentational state | cannot cause real damage or floor transition |

No new inventory, class builder, special button, extra controller character or inflated monster HP is approved by this contract. Solo and two-human input ownership must survive scene transfer.

## 3. The actual progression path to enforce (candidate events, NOT compiled enums)

| From | Authoritative requirement | On resolution | Illegal shortcut |
| --- | --- | --- | --- |
| F01 exit -> F02 S00 | actual F01 story run ends; earned capability snapshot | transfer party to **Floor02** | spoof by all-abilities debug loadout |
| F02 S07 Rage-only pit | Rage actor enters authored exile volume | Rage Exiled, Carl/Donut **remain Floor02** and reach S08 | transfer party / award Rage normal XP |
| F02 S09 Ralph -> Mongo | Ralph genuinely defeated, Mongo met in epilogue | unlock final party stair | Mongo as early combat follower |
| F02 final party stair -> F03 S00 | Ralph defeated AND Mongo met and exit reached | enter Floor03 Guild carrying F01/F02 grants | treat S07 stair as this exit |
| F03 S00 Guild | entered from true F02 exit, or clearly isolated story fixture | once-only Carl Primal / Compensated Anarchist; Donut Cat / Former Child Actor; Mordecai manager identity | grant unapproved spells, alternate class builder or repeat grant |
| F03 S05 Grimaldi | physical defenders cleared, rooted listening/contact state reached | nonlethal story resolution, not boss kill XP | kill Grimaldi by reducing fake HP |
| F03 S06 first Floor04 stair | only seen/foreshadowed | remain **Floor03**, continue S07 | early party Floor04 transfer |
| F03 S08 GumGum | reachable alley aftermath | record finding **already dead** GumGum, advance investigation | make her escort/killable score target |
| F03 S10 Quill | legitimate magistrate office breach and safe story reveal | Quill dies before Remex reveal; Featherfall already dead | kill Quill from VFX or reveal Remex before |
| F03 S11 crystal | actual Remex contact, plot reveal and safe first-blast case closure | soul crystal contained, unique Doomsday story device | assume Remex crisis resolved |
| F03 S12 magic 1–4 | first crystal case closed, second Remex countdown triggered | **four** ordered, distinct world-state hazards | skip/reorder pulse, remove hero learned controls |
| F03 final party exit | all four pulse results, finale safe route and both heroes ready | true Floor04 transfer / Iron Tangle hook | use earlier circus stair, duplicate transition, strand P2 |

**Book/game boundary:** fixed story class choices, 2P Donut remaining active during Heather, early circus stairs being merely scenic and no gear stripping are **game adaptations**, not verbatim book claims. Source chronology itself remains subject to a primary-text editorial pass.

## 4. Concrete tests to implement with the real current Rust APIs

| ID | Stimulus | Expected acceptance result |
| --- | --- | --- |
| BR-01 | F01 normal accepted run snapshot -> first F02 room | legitimate earned per-hero capability bits preserved; no later abilities present |
| BR-02 | Rage enters S07 pit | only Rage exiled, no ordinary kill XP or party floor change |
| BR-03 | Carl or Donut enters same S07 pit | no Floor03 transition, protected safe path valid |
| BR-04 | F02 final stairs before Ralph defeat or Mongo meeting | DENY with explicit missing requirement |
| BR-05 | correct Ralph -> Mongo -> true stairs | exactly one Floor02->03 transition with both active heroes |
| BR-06 | repeat F03 Guild on retry | exactly one story identity/manager receipt, no ability duplication |
| BR-07 | clear Grimaldi defenders, contact root | nonlethal plot event, no normal boss XP/kill |
| BR-08 | enter early F03 circus stair | stay Floor03, continue to skyfowl |
| BR-09 | Miss Quill office special effect without actual game event | no story death grant from cosmetic VFX |
| BR-10 | actual office breach, then Remex reveal | Quill death precedes Remex, Featherfall never alive boss |
| BR-11 | first case closed | first disaster contained, Remex countdown still active |
| BR-12 | pulse 4 requested before 2 | REJECT; no state/loot/skill corruption |
| BR-13 | duplicate pulse, story case or class receipt | idempotent, zero bonus reward |
| BR-14 | heroes split across co-op camera during pulse | safe reachable route, no offscreen unavoidable hit, no silent partner teleport |
| BR-15 | final Floor04 door without contained crystal and ordered four pulses | DENY, no floor transition |
| BR-16 | correct true final door with both hero slots | exactly one Floor04 entry, story snapshot persists |
| BR-17 | retry during Rage/root/pulse attack | deterministic checkpoint; never resume inside lethal active volume |
| BR-18 | corrupt recorded PLAYTRACE event/data pin | strict replay rejects, no old F01 golden rewriting |
| BR-19 | isolated F02/F03 room loaded in test-all mode | reports ISOLATED; **not** full connected campaign |
| BR-20 | all bots succeed but real owner has not played | simulation validity only; 1P/2P enjoyment = NOT RUN |

**Mandatory negative controls:** BR-02/03/04/08/09/12/13/15/18 must deliberately attempt illegal progression and assert the real transition/replay API fails. A happy-path scripted clear is not sufficient evidence.

## 5. Implementer sequence and hot-file ownership

**BRIDGE-W00** audit latest main state, actual narrative flag storage, F01 route issue, F02/F03 draft branch differences, room input and Playtrace reset signatures; record source and compiler types rather than inventing CLI options.

**BRIDGE-W01** introduce the minimal floor-neutral transition validation over the existing room/story system and one-time plot receipts. One integrator owns shared hot files (`src/story_unlock.rs`, `src/combat_scene.rs`, `src/party_scene.rs`, `src/playtrace/replay.rs` and scene router). Preserve F01 golden tests and current launch flags.

**BRIDGE-W02** integrate F02 S07 Rage-exile-vs-party-stair and S09 Ralph/Mongo actual exit, with direct isolated negative fixtures. Floor01 E2E still draft PR #194: full campaign carry is **BLOCKED** until actual playable route exists, while local isolated F02 work may proceed.

**BRIDGE-W03** implement Floor03 Guild/Grimaldi/early stairs/Quill/Remex/pulse ordered run events with pure Rust fixtures on their own files, *without* requiring a playable F03 scene or cross-shared changes by several agents.

**BRIDGE-W04** once F02 and F03 real scenes exist, validate full normal story carry, 1P Carl-leading, 1P Donut-leading and two-slot scripted routes plus actual Bevy 640×360/400 clips, exact manifest pins and strict Playtrace. Physical two-pad and human game-feel remain owner's own gate.

## 6. Truthful completion states

`DESIGNED` docs only; `PURE` tested simulation; `SCENE` actual Bevy room playable; `REPLAY` strict source-pinned capture and reset; `CONNECTED` true story route from preceding floor, no all-debug grants or teleport shortcuts; `OWNER` real two-controller/game-feel/visual approval. **Do not mark CONNECTED from PURE + SCENE alone.**

Every agent PR reports exact tested source SHA, commands actually executed, first unimplemented story transition, source manifest hash, regression/rights/art status, one real game-size scene render where applicable, and all human-only tests as NOT RUN. Never claim tests ran because this markdown describes them.

**Next playable priority remains F02 S00–S03** and the owner Floor01 real-pad baseline. This cross-floor plan exists to prevent architectural redo, not to delay productive isolated stage/data/art work.
