# FLOOR 03 — THE OVER CITY (design bible v0.1)

**Status:** CONCEPT DESIGN ONLY, not implemented, playtested, canon-verified from primary text, or owner approved. Date 2026-11-22. Separate from Floor 02 PR #239. Fan-concept adaptation, not a claim of game distribution rights. The six-chapter structure below is an ORIGINAL arcade design, **not the map or room sequence as described in the book**.

**References and hierarchy:** AGENTS.md; [gameplay/combat first](combat-first-encounter-design.md); [owner-approved control contract](mvp-controller-contract.md); [character progression horizon](character-progression-horizon.md); [no-inventory ADR](../05-decisions/ADR-0010-combat-first-progression.md); [visual bible](../02-art/floor-03-over-city-art-direction.md); [agent work order](../07-agents/floor-03-design-to-production-roadmap.md). Floor 02 has its own [design PR #239](https://github.com/pando85/dungeon-floorbreak/pull/239), currently a separate unmerged design branch. Do not import new hypothetical Floor02 APIs as if they already existed.

## 0. Artistic thesis — welcome to a city built as a stage and a prison

The player leaves the pale enclosed Floor02 industrial structure and steps into a huge, false outdoor sky. The Over City is not a generic medieval town. It is a **decaying city-sized television set**: a night sky that never quite looks natural, crooked timber villages and impossible masonry ruins sitting above a volcanic scar, high-hanging propaganda and gleaming administrative wards, a carnival whose monstrous performances will not end. Fantasy elements are dressed with **1980s/90s action-film staging, grotesque game-show production and industrial backstage mechanics**.

The tonal contradiction is the identity: the Guild sells the fantasy of infinite RPG options; outside, terrified citizens live in a place that the organizers periodically destroy; the circus wants to entertain but is a real tragedy; the skyfowl settlement claims moral authority while hiding atrocities. Carl is an improvised, battered antihero in increasingly elaborate systems. Donut remains a cute, expressive, mobile cat, not a floating spellcasting human. Mongo adds emotional stakes but not an unapproved third controller.

**North star:** "The city is lying to you. The way you kick things into one another is still honest."

## 1. Six player-facing arcade chapters / thirteen internal beats

The book devotes almost its entire second volume to Floor03; the game must **not** copy every shopping scene, NPC, or branching quest. One linear Metal Slug-like route, six memorable visual movements with internal checkpoints, no compulsory map screen or multi-quest log. **Initial first clear target: 40–55 min, unvalidated**, with 5–10 min chapter segments and replayable checkpoints; cut filler based on owner feel instead of extending HP.

| Chapter | Internal beats | Player-facing title | Hero image | Gameplay escalation |
| --- | --- | --- | --- | --- |
| A | S00–S01 | **Welcome to the Over City** | Guild under a synthetic starfield -> colossal collapsed urban valley | class/race story beat then mobile combat |
| B | S02–S03 | **The Show Must Go On** | ruined fairground, knife lemurs on rooftops, skeletal circus caravans | committed throws, hostile vertical lanes, ink lion collision |
| C | S04–S06 | **The Big Top Has Teeth** | Heather in a bear pit -> impossible canvas tent full of violet vines -> Mongo vigil | physical neighborhood boss, high-risk setpiece with a nonlethal narrative resolution |
| D | S07–S08 | **The Angels Are Watching** | monumental skyfowl settlement gate -> lantern alleys and hovering Krasue | guarded safe zone contrast, night street ambush, authored social stakes |
| E | S09–S10 | **City Hall Is Closed** | institutional cult theater -> booby-trapped magistrate's tower | ranged/short threats, real wall breach and demolition |
| F | S11–S12 | **Doomsday Is Not a Plan** | trapped soul-capacitor / glass-case room -> a whole city running from light | last tactical fight + singular huge authored escape, party stair to Floor04 |

**Structure caution:** The circus is the *first* massive dramatic climax, not the last boss of the entire floor. The skyfowl conspiracy and approaching blast are the second half. This avoids endless circus-only content and creates a satisfying sense of arrival/change. S00–S12 are internal scene/encounter IDs, not thirteen loading screens or thirteen stage-clear panels.

### A — Welcome to the Over City

#### S00: CHOOSE YOUR OWN DISASTER

**Canon story anchors:** reach the Tutorial Guild, race/class are chosen; Donut remains Cat and chooses Former Child Actor; Carl chooses Primal and Compensated Anarchist. Donut's decision binds Mordecai as manager. Source summaries: https://www.dungeoncrawlercarl.nl/en/news/how-race-and-class-work/ and https://spoilerfreecarl.com/book-2/recap/ .

**Gameplay adaptation:** 60–120 seconds of **skippable, stylized story drafting**, not a complete hundreds-of-races RPG UI. Three parody comparison cards fly past each protagonist; Carl/Donut make their established canon choices as a short animated character beat. Optional interaction may fast-forward and inspect two tooltips, but **no alternate selectable class builds** in the first game. Such a system would multiply art, balance, co-op and UI scope and violate ADR-0010's small-input/no-inventory philosophy.

**Grant semantics:** clear, persistent story/class identity flags for Carl/Donut/Mordecai; only capability modifiers that *actually already exist* or receive separate approval become playable. Neither Primal nor Former Child Actor grants a new required button. Mongo remains a narrative/presentation ally. On retry, skip animation; don't regrant powers or stat points.

#### S01: THE SKY IS MADE OF BOARDS

A broad open city ruin with an **enormous faked sky backdrop**, a deep architectural valley behind hero plane, hanging stairways and boarded windows. Introduce one new *Former Circus Lemur* knife thrower alone, then add a rushing melee scavenger from a distinct visible lane. The knife is a committed ranged attack with a **clear pose before release and no tracking after release**, not another high-HP goblin.

**Signature encounter:** Carl kicks an approaching attacker into a hanging sign; it swings or falls by real collision and can shield him from a subsequent visible knife. Donut jumps to the familiar short rooftop to force a missed throw; the companion stays relevant and returns. The chapter ends at the silhouette of a crooked striped marquee far on the right. The city looks enormous, the actual walk remains linear.

### B — The Show Must Go On

#### S02: KNIFE THROWERS DON'T NEED AN AUDIENCE

A fairground road between **collapsed apartment balconies, rotting colorful tents and a huge dead giraffe-shaped parade puppet** (the giraffe is an ORIGINAL prop/puppet adaptation, not an asserted book scene). Clown faces are hand-painted onto emergency barricades; a few real clowns appear. One unusually tall **Terror the Clown** creates an enormous *legitimate* downward shadow on attack, but has a tangible hittable body and short recovery. Cannons in the distant backdrop prepare one slow mortar warning, not arbitrary offscreen damage.

**Combos:** kick lemur into a knife-throwing platform support; Donut intercepts launched target or takes a reachable flank; physical awning crash reveals one safe new route. Direct kills and a no-prop exit always work. End with a subtle view of Signet's crossed-out poster. No class/race dialog overlays combat.

#### S03: INK AND MOLD

A narrow ruin courtyard threatened by several **Mold Lions** whose bodies are partly infected; two types max in the first pilot (one straight charger and one heavy swiping blocker). An enormous moving **flat, tattoo-like ink silhouette** is framed behind the ruins, visually communicating Signet's action; it is NOT a third uncontrolled allied AI.

**Gameplay:** two authored attack rhythms (charge vs heavy slam) create kick-to-wall opportunities and close-range Donut flanks. A cornered lion can be launched into another *on actual authoritative contact*, letting the player learn the language of impact before the next boss. One single visible ink curtain temporarily blocks one lane in a telegraphed way but does not invert inputs, obscure the screen or demand a spell. The threat resolves to a short unsettling appearance by Tsarina Signet. No forced romance, gore cutscene or six unlimited spawns.

**Canon/2P caveat:** in the book Signet's Water Lily curse sidelines Donut while Carl fights Heather alone. A 2P game that removes one human's hero for ten minutes would violate the core co-op promise. First adaptation: curse is presented as threat and high stakes in cutscene, but **both heroes remain playable** in the actual boss encounter; the narrative difference is clearly recorded. An alternative short split-screen-only Carl solo sequence is deferred unless owner specifically approves a multi-character-disabling encounter.

### C — The Big Top Has Teeth

#### S04: HEATHER IS NOT THE MONSTER

**Heather the Bear** is a readable physical neighborhood boss: a heavy animal silhouette with a parasite-like second motion rhythm, not a grotesque anonymous HP sponge. Three clear attacks: committed paw swipe, short charging shove, and an *authoritatively separate*, telegraphed worm/parasite wave along the ground. The player can win with normal Carl/Donut damage and well-timed strikes; two tempting environmental support beams create optional safer windows through real body collisions.

**Emotional direction:** the bear's weakness/recovery should communicate pain and manipulation rather than triumphant execution for sport. A brief silent post-boss beat sets up the warped circus family. Do not introduce a carry/antidote inventory minigame, endless parasite adds or a forced surprise instant kill.

#### S05: THE BIG TOP IS A MACHINE

A colossal tent viewed as a **multi-layer silhouette**: audience stands, a central living vine, three gigantic support ribs and a menacing horizontal stage ring. It is a spectacle of physical collisions and backstage machinery. The ring is held by a *ward*; the protagonists fight circus defenders and use ground routes plus Donut's elevated route. A thrown body may strike a support, dropping an authored section of canopy in a fully telegraphed collision; support breaking is optional **combat advantage**, not a mandatory "hit three poles in order" puzzle.

**Grimaldi narrative rule:** the book's Ringmaster Grimaldi is a Level85 City Boss/Elite whose story resolves **without killing him**, through Carl's negotiation/connection and Signet's reunion. We do NOT convert this into an ordinary killable tentacle HP boss, nor repeat Floor02 Rage's "just flee" structure. The actual win is **defeat real hittable circus defenders, hold positional pressure through three distinct stage states, survive fair committed vine swipes and unlock an authored nonlethal reunion beat**. Optional collisions shorten/favor segments, but basic attacks must remain useful against defenders and exposed supporting tissue. Boss visuals may recoil on damage without fake HP loss. Avoid inventing poisoning mechanics or requiring special consumables/plot dialogue button sequences.

**Interactive climax:** when both protagonists clear the last defending lane, the original circus show music collapses to an empty note; the enormous vine opens, Signet arrives and the scene turns from a fight to a tragic recognition. No player-triggered scripted kill or unearned rare slow-mo for this narrative resolution.

#### S06: MONGO KEPT WATCH

A rescue/quiet passage with physical traces of **Street Urchins** and Mongo guarding the resting Donut in the book. For co-op, the scene opens with an extremely short non-playable sleep/recovery tableau, then **both heroes resume control** for a direct, small cleanup fight against visible spiny little scavengers; Mongo guards the safe room by authored *local protection behavior* without introducing third simultaneous player-slot AI. The now-empty fairground behind them reveals a **Floor04 stairwell** that is available earlier in the story but **NOT necessarily the route the game chooses yet**; we will use a different path to the remaining skyfowl chapter and only truly descend after the final city crisis.

No new Mongo mount commands or equipment UI; adult/tall Mongo art respects existing approved growth and proportions, not a generic dragon/raptor redesign. Pet ally behavior will be revisited when party AI and owner game feel allow it.

### D — The Angels Are Watching

#### S07: SAFE UNTIL THE GUARDS NOTICE

The safe skyfowl settlement is not a level where the player is punished for accidentally tapping attack at any time. High-level city guards are strong non-hostile architecture, signposted before the player sees them; the playable combat pocket is **beyond** guarded public space. The border between warm street lanterns, impossible polished towers and wretched alleyways tells its own story. One brief optional Mordecai/Desperado Club background show beat establishes the black-market politics but there is NO shopping simulator or full club minigame.

**Gameplay:** a run of low-pressure foes outside the gate and a strong two-role action beat against mobile darkened enemies within an unauthorized service alley. Carl's bomb-building subclass can create *world interactions with existing earned charges*; Donut remains a mobile caster. The citizen rescue theme belongs here rather than generic faction standing bars.

#### S08: THE NIGHT HAS FACES

A night alley lit by swinging overpass lamps. **Krasue** appear as disturbing floating heads with visible hanging entrails; sharply distinguish them from Mind Horror/Vespa by their round human face, short pulsing flight and directional diving attack. Author one compact **GumGum aftermath** clue/care scene with dignity; victims are not score items or destructible props. A cult elf threatens from a solid range and can be launched into a sign with a genuine physical hit.

**Combat grammar:** 1–2 Krasue plus one grounded guard/cultist under max roster; a lamp can be swung toward one enemy to expose it, but there is no mandatory portable light, spell switching or stealth meter. An optional hidden route is a one-screen bonus, not a branching quest campaign.

### E — City Hall Is Closed

#### S09: THE APITO EDUCATION CENTER

A cult administration temple that masquerades as a civic education complex. The skyfowl's elite architecture is designed like a theatre of compliance: huge projecting wings, propaganda banners and **suspended mechanical lifts**. A few visible **city elf cultists** control up/down lines with heavy melee and one directional magical denial; an optional launcher or existing bomb can collapse an unprotected support wall.

**Moral stakes:** protect real civilians and avoid "mass casualties = spectacle views" reward. If a destroyed section threatens civilians, telegraph and route the combat to save them; nobody outside the visible playfield can be silently killed by a player's unreadable explosion. All required enemies can be fought directly. No mandatory stealth, dialogue puzzle, faction-reputation grind or invisible guard patrol insta-death.

#### S10: MAGISTRATE, PLEASE HOLD

Two sequential rooms with visual continuity: an absurd front-office desk with dozens of waiting numbers -> a cramped, corrupted office becoming an exploitable demolition machine. **Miss Quill** is presented as an administrative puppet-master, NOT a giant standard physical boss with a thousand HP. The grounded fight is against visible cultist/undead guards and Krasue nests; physical charge into a desk rack may expose a structural weak support; the optional showpiece is a properly caused **controlled building breach** that damages only physically contacted enemies.

**Katia Grim:** introduced as a shy, shape-shifting fellow crawler via a very short authored safe-room beat; for v1 she can be a visible story ally and a contextual one-off defensive assist (if fair under current simulation), NOT a mandatory permanent third controllable character. A real Katia AI and player commands require a separate party contract.

### F — Doomsday Is Not a Plan

#### S11: THE SOUL LEECH CAPACITOR

The living engine of the conspiracy is not a conventional killable lich. Reveal **Remex** as a soul-leech/captive capacitor with a frightening silhouette, but the emotional twist is vulnerability. The playable section is a **narrow defensive brawl** against the last readable Krasue/cult defenders, with real overhead breaker/electrical charges the player can kick enemies into. A single short scripted revelation turns the room into a dangerously growing energy vessel; staging must not depend on copying a substantial book dialogue passage.

**No new object-carry inventory.** Carl's Doomsday Scenario can be represented by an original unique *sealed event prop* with persistent narrative story ID. A charged case and one interaction/event start the blast sequence. Narrative consequence is not an endlessly reusable grenade special.

#### S12: THIS CITY IS A BOMB

Floor climax: a **very wide false-sky night city** suddenly washes from violet to impossible white, with escalating warned shock-fronts. In a giant side-scrolling sequence, Carl, Donut and narrative allies cross three carefully authored *short combat escape segments*; the player may kick obstacles, interrupt pursuers, or trigger an existing real blast to open an **optional** shortcut. The mission does **not** become 90 seconds of constant full-speed platforming.

A first authored "precursor wave" teaches the pulse rule while all heroes see cover. Subsequent waves pressure decisions but **never strip controller controls, skills, equipment or assets from the party**: book's stripping of stats and items is translated into light/audio and a short temporary limitation if approved later. No permanent inventory exists in this game. The final protected stairwell is the **only** true party Floor04 transition. **Rage stair from Floor02, circus stair reveal from Floor03 and final escape stair are all distinct story events** — test their ordering. No fake global camera shake/white flash that hides active attack warnings.

**Story outcome:** escape is earned through combat decisions, not an "extra 1 HP survival runner"; the growing glass-case threat is contained in the story, and the heroes reach the stairs alongside visible survivors. Transition promises the Iron Tangle in Floor04, it does NOT render a playable train network as Floor03's epilogue.

## 2. Combat composition and identity

### New enemy families, priority
- **Former Circus Lemur** — committed knife; low HP, physical launch, visible line.
- **Terror Clown** — stilted long step, visible down-strike, high knockback, not HP wall.
- **Mold Lion** — one direct committed charge and recovery; heavy variant a distinct swipe.
- **Heather / parasite** — true physical boss with two layered tells; direct Carl and Donut viable.
- **Street Urchin** — spiny janitor, quick low approach, no entire screen obscuring.
- **Krasue** — floating head with hanging body parts, vulnerable hover/dive.
- **City elf cultist** — foot soldier with commitment, can be launched into civic props.
- **Shambling Berserker** — strong but slow undead, decisive contact, body launch into cover.
- **Grimaldi** — NARRATIVE elite setpiece, no ordinary HP kill route.
- **Miss Quill** — manipulator/presentation plus visible guards; not a generic physical boss.
- **Remex** — victim and story trigger, not a boss enemy.
- **Signet, Katia, Mongo, Mordecai** — story actor roles with separate **authoritative gameplay ownership** if and only if an explicit, tested small subsystem is approved.

Only a few combat families should operate in any room; reuse body/prop/hazard collision and current warning budgets. Before adding a new family, test distinctive silhouette, hit body, fair tell, and each hero's actual contact path.

### Three floor-specific combat lessons

1. **Kinetic stage geometry:** lamps, banners, awnings, rigging, circus supports. No invisible physics chain shortcuts. Direct close fighting is always good.
2. **Script versus agency:** the city "show" has scripted production elements (trailer, plot-armor elites) but actual combat damage is deterministic and entirely under the legitimate simulation. Don't use "plot armor" to invalidate player attacks without a clear narrative/UI boundary.
3. **Protecting people without escort chores:** civilians, Mongo and Katia create meaningful stage context, not a third pet HUD, NPC health babysitting or unbounded pathfinding.

## 3. Progression and class/race UX contract

The novel's race/class screen is a **story node**, not permission to build a full LitRPG inventory UI, hundreds of classes or stat-point allocator. The v1 characters keep their identity and previously earned moves. Their new narrative class labels appear once with concise modifiers:

| Character | Canon label | What gameplay v1 does | Not yet approved |
| --- | --- | --- | --- |
| Carl | Primal + Compensated Anarchist | story identity + existing kick/punch/bomb skill synergy, room-local sabotaging physics | new RPG stats, class branch tree, independent crafting |
| Donut | Cat + Former Child Actor | identity + manager/Mordecai story state, mobile spellcaster + cat physical | alternative classes / performative social skill UI |
| Mongo | growing pink/red feathered raptor | original approved source pose, room-local scripted protection or cinematic support | autonomous third follower, ridable mount controls |
| Katia | ally crawler / Doppelganger | authored safe NPC scene and perhaps one contextual fixed-tick assist | permanent additional player/companion slot |

A Floor03 *one-character-special* (e.g. Donut's Clockwork Triplicate of Mongo) is a **candidate for separate design, not a free automatic grant**; it would use the established equipped-special grammar and require correct ally cap and actual no-magic-before-biscuit continuity. Avoid inventing dozens of loot drops from the book.

Individual viewers and spectacle rewards follow existing rules; the City Boss/Grimaldi reconciliation, soul rescue and narrative death are not regular kill XP or effortless cinematic finisher score. All new story grants are once-only and replay-hashed; no retries farming views or class unlocks.

## 4. Performance, camera and story guards

- 640×360 and 640×400 at native hero scale, especially in a huge apparent outdoor city. Create depth with **background silhouette parallax**, never shrinking Carl/Donut to 5 px for a panoramic camera.
- Two real players remain active in all critical-path combat. Solo partner not guaranteed to use expensive new pathfinding. The game may show scripted solo Carl acts as very short narrative portraits, not ten-minute P2 loss of controls.
- All newly destructible city props must preserve story exit, protected respawn and safe route in every destructive state.
- Large scenes need strict active enemy + hazard + particle budgets; crowd/circus effects mostly presentation. No hero-hurting purely cosmetic fireworks.
- Circuses/lions/animals must remain readable amid VFX. If a major danger tell is hidden behind static crowd art, reject the concept.
- Story-gated citizens and sanctuary zones: guard rules announced before contact, no hidden offense on innocuous inputs.
- First clear 40–55 min and six chapters are exploratory, not a production commitment until owner plays. This is a larger floor by design but must still feel like one coherent arcade campaign.
- Standalone room fixtures and full story route are separate deliverables. F02 engine reuse/route contract is currently proposed in PR #239, not already on main.

## 5. Source status and rights

**High-level narrative source checks (secondary, not primary-book verification):**
1. https://spoilerfreecarl.com/book-2/recap/
2. https://github.com/cdkolb/dcc-compendium/blob/main/docs/chapter-summaries/book-02.md
3. https://mapaverse.com/dungeon-crawler-carl/world/third-floor
4. https://progressiontime.com/primers/dungeon-crawler-carl/book-02/
5. https://dungeoncrawlercarl.wikioasis.org/wiki/Third_Floor

**Canon anchors:** Over City (Floor03), race/class choices, Former Child Actor manager, Primal Compensated Anarchist, circus, Former Circus Lemurs, Mold Lions, Heather, Signet/Grimaldi narrative, Mongo defense, Skyfowl settlement, GumGum, city elves/Krasue/Miss Quill/Remex, Katia, final dangerous blast and Floor04 stairs.

**Explicit original game adaptations:** six-chapter linear order, all stage geometry, giraffe parade puppet, level routes, every attack move/timing/AI, physical awnings and demolition toys, optional no-prop paths, shortened class-selection UI, Donut playable in Heather even though original sequence is Carl solo, scripted Mongo/Katia role caps and alternate escape pacing. Check primary book before describing any exact dialogue/equipment chronology as canon. No copyrighted quote, movie asset, gameplay music or recognizable other-game sprite is authorized. Franchise fan concept art publishing rights are user-reported and do not automatically permit a distributed playable game or commercial use.

## 6. First build slice and human questions

**Preferred first Floor03 production candidate (after Floor02 integration is real):** S00 short class story beat -> S01 big city ruin with lemur ranged combat -> S02 circus perimeter physical chaos. Prove player-relevant class grant persistence, two distinct enemy body attacks, a real hit-to-awning interaction and native-scale camera readability before producing the whole Over City.

**Human questions:** Does the open-air environment feel substantially different from Floor02? Does a fixed story class-choice beat feel elegant or remove agency? Does Mongo's presence make the scene more personal without adding control complexity? Is Grimaldi's resolution satisfying in a combat-first game, or should the Big Top fight receive another direct physical mini-boss? Is Donut's continued control during the Heather arc the right co-op adaptation? Does the final city-blast chase feel unlike the prior Rage escape? Owner verdict PENDING.
