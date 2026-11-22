# F03-CITY-01 — Skyfowl city, GumGum, the Krasue and Miss Quill (S07–S10)

**Status: DESIGN PROPOSAL ONLY**, 2026-11-22. No engine implementation, owner gameplay acceptance or primary-book chapter verification. Companion to the [Floor03 bible](floor-03-over-city-design-bible.md), [art bible](../02-art/floor-03-over-city-art-direction.md), [Doomsday final-act spec](floor-03-doomsday-scenario-final-act.md) and [Floor03 agent roadmap](../07-agents/floor-03-design-to-production-roadmap.md). Six arcade chapters and thirteen internal beats are **game adaptation**, not exact novel route.

## 0. Design thesis

**From a carnival that openly exploits pain to a city that hides exploitation behind polished order.** Chapter D is a "false safe zone": grand walls, warm official lamps, disciplined bird guards and busy markets. The player sees a city with ordinary residents, then a boundary beyond which victims have been abandoned. Chapter E takes the same official architecture and turns it into a physically interactive machine that **Carl and Donut can break, but only through real combat events**.

**Plot truth for pacing (secondary book-2 summaries, check primary):**
1. GumGum introduces the murders of exploited women in the Medium Skyfowl Settlement; **she is already dead when the protagonists find her in an alley** after the Pustule/Desperado episode. Do **not** design a mandatory rescue-GumGum mission or treat her as a living AI escort. The protagonists encounter **three Krasue** in the original scene; our live roster may be smaller for readability.
2. A compromised letter and testimony lead from city-elf militia/Apito Education Center to Miss Quill, apparent assistant to Magistrate Featherfall, the true secret ruler.
3. The Apito encounter provokes town guard involvement in the source; avoid punishing players with opaque or politically arbitrary guard aggro. Provide a clear authored combat zone at the temple and a safe regroup route.
4. Carl's planted dynamite and scrying-letter decoy destroy part of the magistrate's quarters, **killing Miss Quill before the later Remex reveal**. Featherfall is a previously dead/mummified prop, not a live boss with conventional HP.
5. The Krasue nest and dying Burgundy provide the revelation; Remex is found **later** at the warehouse as a Soul Leech Capacitor, not a lich or ordinary boss.
Sources: https://spoilerfreecarl.com/book-2/recap/ ; https://github.com/cdkolb/dcc-compendium/blob/main/docs/chapter-summaries/book-02.md ; https://mapaverse.com/dungeon-crawler-carl/character/miss-quill . These are fan recaps, not the book text.

**Arcade rule:** this is *not a mystery adventure quest UI*. The player investigates by surviving **authored encounters**; information is conveyed via environment, small original story beats and four clear narrative receipts, not searching 12 drawers or interrogating five dialog trees.

## 1. S07 — SAFE UNTIL THE GUARDS NOTICE: two faces of one city

**Provisional layout:** ~3–4 camera widths of public/gate architecture then one normal combat pocket outside the safe jurisdiction. Three horizontal foreground bands: low public walk, elevated storefront ledge, fortified guarded upper parapet. Only the first two are playable for ordinary combat; guards are huge background / clearly protected actors until a separately identified story event.

**Landmark art:** monumental white winged stone gateway and over-ordered lamplights, a large bird silhouette above the street; the far skyline is a fake night canopy full of rigging. The right side darkens into cramped migrant worker alleys and lower-contrast maintenance ducts. No actual eagle-flag source art copied from fandom.

**Playable sequence:**
- B0: in a relatively peaceful public street, the party can walk and see the disparity: elite skyfowl above, tired service workers below. Short authored original citizen/GumGum introduction or memory vignette before the later night aftermath; no forced long conversation.
- B1: one city guard blocks a forbidden **upper** doorway and signals danger by clear visual pose. Player should not be expected to beat a level-overpowered official; a clearly painted legal lower route avoids the patrol. **No invisible instant kill for pressing normal Attack in a peaceful region.** If a trespass mechanic exists, warn and allow safe retreat.
- B2: outside protected territory, 1 low-health prowler or cult scout appears, then one ordinary ground enemy. Carl/Donut regain normal direct combat. A hanging fabric market sign is a purely optional kick-to-support falling hazard; it cannot fall on innocent residents outside the actual hit volume. A straightforward no-prop clear proves the stage is still an arcade fighter.
- Exit: a warm street lantern flickers as a distant scream/moan pulls the party toward S08. End with an **emotional pressure** shift, not 30 seconds of walking.

**Guard authority contract:** public guards either noncombat/scene-bound clearly signposted, or real limited authored combat once separated; they cannot fire from behind the camera or pursue across story-safe zones unless explicit danger/exclusion rules allow it. Do not implement open-world faction reputation, stealth meter or "attack one NPC: entire floor softlocked" system.

## 2. S08 — THE NIGHT HAS FACES: GumGum's alley

**Provisional layout:** ~3 compact camera widths, one narrowing alley, an overhead rope-and-lantern junction, a small dead-end revealing GumGum, one exit door to the magistrate street. Distinctly different from Floor02's Mind Horror: Krasue are **human-faced floating heads with trailing viscera**, with a violent *short committed dive*, rather than cranial floating brain with psychic cone. Keep horror restrained at game 1x: explicit art clarity > visual gore.

**Encounter choreography:**
1. Street light reveals **GumGum's body** in a safe authored victim space; no enemy spawns directly on it. The party gets one very brief, replay-safe reaction; body is never a collectible, kickable prop or hit-score target.
2. A single Krasue emerges across a **visible upper lane**. It fixes eyes and lifts its appendages before a straight committed dive; it has a **reachable lower recovery** for Carl's kick and Donut's physical/magic attacks. No permanent artillery hanging above Carl.
3. A second Krasue approaches from a different visible position **only after** the first warning has been demonstrated. An elf cult scout moves along the lower ground lane in a separate pocket; do not simultaneously create two high dives plus an offscreen slingshot shot.
4. The overhead lamppost swings if a genuine launched body, kick, or authorized projectile strikes a support. Once struck, it **commits to a swept collision** and drops real light/impact on targets; a miss is not automatically a kill. If no prop is touched, ordinary attacks fully clear.
5. Behind a dislodged poster or small clue board is a document/insignia linking the Krasue to the 201st Security Group, not a mandatory item search. Story flag advances on the actual stage clear/investigation region. The player cannot fail by missing one unhighlighted paper.

**Light mechanic boundary:** bright lamplight may cause a Krasue to retreat/reveal its attack position, reflecting the source where Krasue flee Donut's light. Do not require a new torch item, stealth puzzle, spell swap or absolute vulnerability only under magic light. Donut still has strong mobile magic; Carl can knock down the same foe during exposed recovery. Two optional conditions: unlit direct kill and lamp-assisted short stuns.

**Dignity:** GumGum is not a gore comedy beat, a loot chest, a kill credit or a sponsor/viewer exploit. The contrast with the previous grotesque circus is deliberate. A small compassionate story reaction does more than a long voiceover.

**A plausible local setpiece:** a wildly flailing Krasue hits a hanging lamp, lights its trailing shadow, and is knocked by Carl into another charged opponent. This is an *actual* chain only if collision/authority proves it, not a pre-scripted death animation.

## 3. S09 — THE APITO EDUCATION CENTER: order as propaganda

**Provisional stage:** ~4 camera widths, two action pockets and a brief safe corridor, one primary ground lane plus short **reachable** gallery platforms. Huge monumental wing statuary, overly polite lecture scrolls and banners, suspended utility lifting stages and one conspicuous structural joint.

**Main enemies:** actual **201st militia city elves/cultists**, one staff-bearing melee and one limited one-lane magical projection, no infinite reinforcement or high-level city guards in this combat pocket. A bounded Shambling Berserker can be introduced late if it replaces, rather than stacks on, the existing roster. Each new role is shown alone before combined pressure.

**Fight 1 — a public lesson:** a single melee city elf approaches under a severe statue, revealing how a combatant acts. A second enemy from a real window announces a directional spell cue (not a full-screen ranged barrage). Donut can interrupt from a ledge; Carl can launch an enemy into the shallow gallery support.

**Fight 2 — the fabricated miracle:** the militia tries to cross a suspended teaching platform. The platform can tilt under real body mass/impact, dropping the actual attacker into a reachable ground position. Carl may knock a guard into its support; Donut can move underneath and finish. If the player ignores the platform, the direct combat lane remains open and completable. This is a physical setpiece, not a four-switch classroom puzzle.

**Narrative/innocent protection:** the source includes bystander harm, including Eunice/Ricky Joe in a chaotic temple battle. **Do not casually depict that tragedy as a rewardable target.** The adaptation establishes a protected civilian strip clearly before explosions; an original aftermath beat can acknowledge the broader harm. Never silently kill innocent characters from unseen hitboxes or award bonus viewers for their injury. Human morality should matter without implementing a giant crime/guard AI system.

**Exit:** militia records/insignia and exposed office service tube connect to S10; actual street guards can show up **behind a protected barrier** and force a clearly marked passage, not spawn on top of player.

## 4. S10 — MAGISTRATE, PLEASE HOLD: the lie collapses

**Provisional stage:** two physically connected pockets, a receptionist waiting room that gives way to a crumbling magistrate office. The first half is *tense stillness* rather than another 3-foe wave; the second is a **single high-value controlled demolition setpiece**.

**Plot structure (important order):**
1. An official Miss Quill figure dismisses the investigation and blocks access to Magistrate Featherfall. Player cannot damage an invulnerable "desk boss" via normal attacks during the safe-town dialog. A tiny original bureaucratic beat highlights a suspicious locked room.
2. A short *authored planted deception* follows. The player fights a finite guard room with an **obvious destructible shelving/office support**, and can strike it to expose a legitimate demolition line. This uses the pre-existing explosive **once-only authored event**, not a tutorial to make real explosives or a new inventory/crafting system.
3. **Controlled collapse**: when a clearly marked retreat area is safe and the story defense is cleared, a fixed-tick source-qualified blast breaches the office wall. The game may allow a faster direct-battle + auto-trigger variant so no player needs to find one exact small object. The shell removes collision only on authored noncritical surfaces; the only exit survives. Miss Quill's death is an **authored plot consequence of the building strike**, not an ordinary killable boss whose HP is reduced offscreen.
4. In the ruined upper office, reveal Featherfall's **long-dead mummified body** and a finite Krasue nest. One reachable Krasue wakes and attacks with already taught behavior, while a dying survivor (Burgundy) provides a short original, compassionate clue: **Quill was the ruler and Remex is still bound at the Swordsmen depot**.
5. The players have an active combat reward for physically safe retreat and actual guard defeats; the Quill plot reveal is a one-time narrative receipt, **not** a spectacular death slow-mo or "city boss achievement" farmed by retrying.

**Direct-vs-creative reconciliation:** the authored building collapse is story-critical but **its complex optional physics chain is not**. Standard attacks can clear guards, and a visible supported single context interaction can advance the story demolition once legitimately unlocked. More elaborate launched-enemy->office support->breach brings spectacle only if its physical source chain truly occurs. No mandatory bespoke bomb ammo, material gathering, hidden letter inventory or two-character lever.

**Adaptation note:** in source Carl sets charges from a different rooftop and uses a planted scrying letter to lure Quill; our arcade room condenses setup and detonation into one direct visible **authoring controller** with the same story order. This must be labeled GAME ADAPTATION in the event ledger and not narrated as exact book choreography.

**Katia setup:** she is encountered later in this branch of the story as an ally and is important for the depot. A short one-screen safe refuge encounter can introduce her at the end of S10, but no third player slot or fully independent companion pathfinding in v1. Her later detonator action is a scheduled, visible story beat tied to the finale stage event, not arbitrary magic trigger.

## 5. Chapter signatures and role/pressure matrix

| Beat | Intro threat / active max | Environmental signature | User fantasy | Target pressure |
| --- | --- | --- | --- | --- |
| S07 Gate | guard as readable protected boundary, 1 low street foe + 1 scout later | huge bird gate, warm lamps, lower service route | "I see who is safe and who isn't" | LOW rising to MED |
| S08 Alley | 1 Krasue teaches, later 2 sequential; at most 1 cultist in distinct pocket | directional swing lamp, cold alley shadows | "These monsters are people, and the city is lying" | MED-HIGH |
| S09 Temple | 1 city elf, later one staff + one spell | enormous wing statue, one overhead platform | "Their machinery can be used against them" | HIGH |
| S10 Office | minimal physical danger at Quill desk, then finite 1–2 defenders | massive shell support, authored demolition | "We expose the criminal by breaking the institution" | HIGH then silence |

Enforce current live-enemy/hazard budgets (do not raise F01 max_living just because of urban crowd decoration), unique source IDs and only one active high-magnitude falling mass. Show dramatic multitudes through parallax/background crowd with **no hitboxes**. Never count scenery NPCs as real full-AI actors.

## 6. Event/continuity policy (proposed data names)

No new Rust API is asserted. Map candidate facts to the actual story and scene ledger when implemented:
- \`city_gate_entered\`, \`safe_guard_zone_seen\` — not combat boss kill flags.
- \`gumgum_death_discovered\` — once-only narrative; never spawn GumGum as an escort after this.
- \`krasue_first_defeated\`, \`militia_insignia_found\` — actual encounter cause and clue region.
- \`apito_defenders_cleared\`, \`apito_demolition_occurred\` — real cause and event (never an automatic entity kill on entering).
- \`miss_quill_dead\`, \`featherfall_long_dead_revealed\`, \`burgundy_testimony_heard\` — story results, not normal enemy XP.
- \`katia_joined_story\` — ally narrative only; separate from player slot / permanent AI.
- \`remex_location_known\` — authorizes the final chapter S11, not a midfloor Floor04 exit.
All once-only narrative grants and viewers follow safe checkpoint, replay and run-reset policy.

## 7. Automated acceptance and negative controls

| ID | Pass condition |
| --- | --- |
| CITY-T01 | S07 in clear public zone: innocuous inputs do not provoke invisible instant death or floor-wide endless guard chase |
| CITY-T02 | player can traverse from S07 legal ground lane to S08 with Carl and Donut, solo companion follows |
| CITY-T03 | GumGum is found dead, not made an NPC escort or physical kill target, clue receipt once |
| CITY-T04 | Krasue one-role teaching before mixed attack; committed dive tell and reachable direct hit by both heroes |
| CITY-T05 | lamp active only after actual qualified contact; no auto-Krasue death without collision; direct unlit path viable |
| CITY-T06 | S09 militia first teaches one staff role then adds second role; no triple unavoidable hitbox overlap |
| CITY-T07 | all S09 lifts/platforms remain safe and the only critical path remains after all optional props break |
| CITY-T08 | Quill remains a narrative culprit, not an invulnerable combat boss; she dies as a building-strike result before Remex is found |
| CITY-T09 | revealed Magistrate Featherfall is already dead; not a living extra boss or an awarded kill |
| CITY-T10 | demolition source event can be traced, retreat warning reaches both heroes, no critical exit softlock; no-prop-combo alternate |
| CITY-T11 | Burgundy/Krasue nesting stage event only after true reachable ruins sequence, not after opening desk dialog |
| CITY-T12 | plot/character flags idempotent across retry, no dual viewer credit, replay strict hashes when supported |
| CITY-T13 | camera/game-size output at real 640×360 and 640×400; civilian and enemy silhouettes separable |
| CITY-T14 | owner human 1P/2P and difficulty/feel **NOT RUN**; tests cannot mark enjoyment approved |

**Stop condition:** if this block starts requiring a crime reputation engine, class-specific mystery dialogs, permanent third-party escorts, giant scripted input lock or actual bystander damage mechanics, reduce it to finite authored stage events and revisit owner product scope.

## 8. Required art keys and implementation order

- **CITY-KEY-01** immense warm city gate above impoverished alleys, calm guard silhouette and a clear protected walking route (safe/unsafe contrast).
- **CITY-KEY-02** GumGum's quiet alley, swinging original utility lamp and *one visible Krasue tell*. Keep tragedy dignified, no corpse-comedy pose.
- **CITY-KEY-03** Apito temple winged facade, actual one platform and readable Carl kick impact against staff cultist.
- **CITY-KEY-04** Miss Quill's office before/after one real wall breach, same geometry two conditions; Featherfall absent from live enemy roster.
- **CITY-KEY-05** ruined magistrate room with small Burgundy survivor reveal and route toward Swordsmen depot.

**Work priority:** pure Krasue and city-elf attack/validator tests -> isolated legal guard boundary and true event ledger -> two first S07/S08 rooms and 1x capture -> S09 two-roles/one lift -> S10 source-authored blast and plot event -> one linked story trace S07-S10 -> original pixel masters and review. Can produce art and stage data in parallel while Floor02 engine bridge unimplemented, but no share-core refactor without a single owner.
