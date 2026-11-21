# Floor 01 — The Quiet Setpieces: Fast-Food Refuge, Survivor Camp and Post-Juicer Regroup

**Status:** DESIGN PROPOSAL for owner review, **not approved level geometry, story dialogue or implemented gameplay**. Based on the owner-selected Floor 01 environment atlas, character progression and the active encounter PR slate, checked 2026-11-21. This PR adds no code and does not change any current unlock trigger or gameplay schema. Original adaptation beats must not be presented as literal book scenes.

## Intent: contrast IS part of a memorable action floor

Floor 01 risks becoming an uninterrupted series of loud encounters: Dozer, goblins, Llamas (#181), Hoarder (#184), Boiler (#176), Dozer revenge (#178), Slimes/Rot (#185), Juicer, Ball of Swine. **Three quiet, playable transitions** should make the violence more memorable by creating space and tonal surprise:

1. **F01-REST-A: "The Restaurant That Should Not Be Here"** — dissonant 1990s fast food, after Infernal Corral, **before Hoarder**. Carries Donut's *already approved* Light on Your Feet story reward.
2. **F01-REST-B: "Meadow Lark's Temporary Order"** — exhausted, organized human survivor camp after the labyrinth, before troglodyte/gym escalation.
3. **F01-REST-C: "The Quiet After Juicer"** — unmistakable mechanical destruction and a tiny *noncombat* test of newly earned physical/teleport skills after the Juicer, before Ball of Swine. Do not grant more abilities here; the boss's single existing defeat trigger does that.

No traditional shop, arbitrary inventory, resource farm, scripted dialogue wall, forced NPC escort, lengthy cutscene, collectible hunt, new control or mandatory camera split. Short narrative observations and physical play carry the personality. All three locations reuse common brick/stone Floor 01 art with localized identity, not global biome swaps.

## A. Fast-food refuge — "The Restaurant That Should Not Be Here"

### Emotional thesis

Last room: burnt orange llama throats, fire-glob sounds, aggressive Carl and Donut. One arch later: **clean blue/white tiles, fluorescent hum, molded plastic benches, empty booths and a child play structure** in a place where there should be solid rock. A cold musical jingle is briefly cheerful but distantly unsettling. Then the dungeon stone is visible through outward-facing restaurant windows. This is an outrageous **juxtaposition**, not a horror boss room in disguise.

**Owner-approved art baseline:** `docs/02-art/floor-01-art-atlas-v1.md`, area 04 and `assets/concept/floor-01/atlas-v1/04-fast-food-refuge.png`. No real-world restaurant name/logo, commercial music, source-novel quote, unapproved character sprite or fabricated franchise brand.

### Functional story / level position

After first Bad Llama and the two-llama Infernal Corral M2, **before Hoarder**. DONUT-PROG-01 DP-Q02 locates a **golden magic item box / Talisman of the Slate Butterfly** reward in the first safe room and grants `jump_reach_boost` through `f01_safe_room_reward_found`. This one passive increases Donut's existing jump reach ~20% (no double jump, no additional input or equippable item). **Do not implement a second unlock system or grant the boost at the corral exit**. Respect the already-owned CP-A story ledger and DP-A ability plumbing.

The proposed visual item and box have to be verified against current art/story asset policy. The main path intentionally **passes right by the reward**, and the associated trigger activates once as authored; the act of finding must visually match the event. No invisible trigger far past the box, no reward for merely entering the room before actually reaching it, no duplicate grant on checkpoint retry, no inventory-management UI.

### Player passage: ~60–100 seconds first run, ~15–30 seconds practiced

| Beat | Spatial function | Player experience | Authoring contract |
| --- | --- | --- | --- |
| **F-A0: hot→cold threshold** | Left entrance; ~1/4 screen of scorched masonry shifts to white tile | Combat audio cuts; abrupt clean ambience. Both heroes retain control. | Safe; no pursuing lava glob, enemy spawn or delayed kill through the door |
| **F-A1: the empty restaurant** | Counter, booth, fake window opening into rock; continuous floor lane | Discover the absurd setting and an untouched, strangely arranged children's play area | Environmental storytelling *without required interaction*, no copyrighted menu text |
| **F-A2: the found treasure** | Gold-box focal point adjacent to normal traversable route | The weirdly plausible reward changes Donut, with one restrained distinct upgrade notice | Actual story grant `f01_safe_room_reward_found` exactly once, persists on retry and in 1P/2P |
| **F-A3: the first higher leap** | 128px vertical short branch **at/near exit**, next to a lower landing/regroup area | Donut can now reach a shelf she could not reach before. Player FEELS the upgrade; Carl still matters. | Must reconcile DP-Q14 and Floor 01's character-gated passage with co-op parity. Details below |
| **F-A4: re-enter the dungeon** | Protected exit to less cluttered, ominous masonry | Cheerful ambience stops, distant insect movement begins | Short exit; accessible no repeated dialogue/checkpoint lock |

**Layout hypothesis:** ~**832–1024px** of horizontal walkable sequence, 16px tile grid; open central ground, counter set behind the collision plane, noninteractive dining booths, optional playable children's plastic tunnel/ball pit **without a new movement physics rule**. The art should afford a 640px camera composition where the broad room reads as a real establishment, not 20 furniture obstacles the player must jump.

**The memorable noncombat moment:** the party crosses past a lit plastic children's slide. Through a restaurant window one sees only thick, raw underground stone. One yellow playground ball gently rolls into the otherwise pristine empty dining area (bounded non-collidable visual animation). After the violence, the absence of danger is funny and oddly disquieting. This is an **original visual gag candidate**, not forced canon or a scripted player cue.

### The 128px jump question — do not silently override an existing design contract

Current DONUT-PROG-01 expects a **first 128px boost-specific ledge at the refuge exit**. The game's earlier design authorizes a short character-specific traversal gate via solo swap. Two viable room-authored options need a real controller test and explicit design signoff:

- **Preferred A1 (faithful to existing progression spec):** after the reward, Donut uses the 128px reach to a short shelf and one **existing Interact R1** affordance opens a **ground-level passage for Carl**. The door then remains open. Single player swaps to Donut; 2P can regroup on floor, with both actors in the camera. A quick 10–15s payoff, not a multi-room puzzle. Avoid requiring autonomous AI to solve this.
- **Fallback A2 (if A1 kills arcade pacing):** 128px shelf is **optional**, containing a short overlook / joke animation, while ordinary rightward exit stays at ground height. **This deviates from DP's "first exit gate" requirement and therefore requires owner/relevant progression PR approval**; never quietly adopt it in code.

Do not create two long paths, lock Carl permanently on the wrong side, consume a mana charge, require Donut's Puddle Jumper or add a double-jump. The new boost must be real via the existing story ledger and mobility engine, not a scene-local gravity multiplier.

**No shop or healing invention:** if existing game resource systems already award checkpoint recovery, use that policy; otherwise leave HP/mana semantics untouched. No full restore just because the scene is "safe" unless adopted separately. No equipment/gear picker, restaurant purchases, currency, new boss or patron rewards here.

### What the player can do (without giving them an objective screen)

- Approach counter/booth to inspect one small, optional environmental interaction through the game's existing interact action (if inspection mechanism is already present; otherwise leave the prop decorative).
- Step into/past the children's ball pit/slide as pure expressive navigation. Do not let soft toy balls alter contact rules in v1.
- Reach a clear, visually central reward and observe Donut's restrained jump upgrade notice; then test her new reach near the exit.

No forced talk, cutscene, multiple-choice dialogue or secret stat purchase. The safe-room entertainment budget should be **architectural surprise and one new mobility affordance**.

## B. Meadow Lark survivor camp — "Order, in a place with no order"

**Position:** after the compact circular labyrinth / Slime + Rot Sticker region, before the troglodyte territory. The approved atlas identifies temporary human organization: crates, improvised cloth/canvases, an old stone enclosure and practical camp layout. It is **a human refuge**, not an identical second restaurant or a "save room shop".

**Emotional thesis:** the fast-food room felt absurdly *normal* because its architecture was wrong; the camp feels painfully *human* because the people try to organize themselves against impossible surroundings. It should acknowledge stakes without a speech.

**Suggested first-visit rhythm ~75–120 seconds, replay 15–25 seconds**, subject to actual gamepad pacing; no required inventory, recruitment or quest list. Candidate beats:

1. **First distant evidence:** cloth partitions and lanterns become visible beyond the final maze gateway. A sentry is watching the *opposite* direction; then notices the party.
2. **Cross a living camp:** several distinct nonhostile human figures (or **clearly labelled placeholders** until native art approved) near supplies, gear repair and hand-drawn maps. One childlike prop or improvised aid evokes practical ingenuity but no exploitative misery.
3. **One original physical/story note:** a survivor quietly repositions a pile of crates after the player passes, trying to restore a safe corridor; this plays as restrained in-world animation, **not a new wall puzzle**.
4. **Checkpoint / emotional anchor:** a protected central light and quiet ambience. The player can continue at will and skip optional talk on repeat.
5. **Leave toward gym territory:** normal stone becomes stained with chalk and odd hand-painted strength motifs, then finally the fluorescent colors of the gym. The transition builds anticipation.

**Co-op:** neither player forced into conversation or long noninteractive waiting. Short shared camera bounds; no role swap requirement. NPCs need no combat AI until specifically planned; never let random enemy/hazard systems trigger inside the safe camp. Any dynamic NPC faces/animations must be native authored, not concept-only sprites.

**Narrative gate:** don't claim specific survivor names, dialogue, abilities or interactions beyond the atlas without script/source review. The camp may be designed first as **pure environment and pacing** and later receive verified character writing. It grants **no ability** by default.

## C. Post-Juicer regroup — "Power, then silence"

**Position:** after the existing `boss_f01` Juicer event and before the Ball of Swine entry. Respect the implementation plan's Juicer scene boundary: the boss is a **separate live scene with story pins**, not a casual extra actor dragged into an incompatible mixed level loader.

**Emotional thesis:** after the absurd, violent gym fight, the machinery stops. A hanging punching bag swings once; an echoing weight clang trails off. The party obtains new ability feedback via the **single existing Juicer defeat story event**: Carl `fist_reinforced` and Donut `special_short_blink`. This beat doesn't grant the powers again and contains no new reward.

**Suggested first-visit 45–70 seconds, replay 10–20 seconds**, without cutscene lock. It can be a narrow two-part corridor:

- **C1 / broken gym egress:** cracked mirrors and wrecked equipment; walking area fully protected. The last fight's consequences are visible but do not imply all mutable boss props persist across incompatible scene transitions unless actually implemented.
- **C2 / tiny positive-control playground:** optional 1–2 **test affordances** using the *already granted* powers: a clearly authored weak dummy/support that the gauntlet now interrupts more strongly (not a new gauntlet tutorial boss), and an **112px horizontal clear travel bay** for Donut's collision-safe Puddle Jumper. Both visual callouts are short, no mandatory "use both together" gate. The normal route remains traversable by base movement.
- **C3 / arena entrance:** the stone corridor curves visually into broad circular grooves; a distant heavy gate clang announces the very different Ball of Swine arena. Preserve a clean checkpoint with no persistent hazard that kills newly respawned characters.

**Do not:** teleport both heroes against P2 consent, consume Puddle Jumper mana outside ordinary rules, create a 112px blink-through-wall gate, require two simultaneous buttons, add a full skill tree or another new ability before the final boss.

## D. Connected Floor 01 pacing and integration map

```text
Opening / Murder Dozer (#179) → first goblins → Guild / biscuit
     → First Spark / Infernal Corral (#181)
     → [REST-A: FAST-FOOD / DONUT JUMP REWARD]
     → Scatterers / Hoarder (#178, #184)
     → Goblin Workshop / Boiler (#176) → optional Dozer revenge (#178)
     → Slime / Rot Sticker pockets (#178, #185)
     → Short concentric labyrinth
     → [REST-B: MEADOW LARK CAMP]
     → Troglodytes / Juicer Gym and boss
     → [REST-C: NEW POWER REGROUP]
     → Ball of Swine arena → Floor 01 exit
```

**Story chronology conflict that is NOT resolved here:** the current floor progression text previously placed Carl's dynamite **before** Hoarder, while the active #182 integration plan **K1** selects grant on Hoarder's defeat (and separately documents it as a game adaptation). Follow the **final merged story-ledger/integration authority**; never silently place dynamite in REST-A or require it for Hoarder's encounter within this noncombat design. The Boiler is positioned only after the grant is available.

**Do not duplicate another team's work:** #181 already owns llama gameplay and its M3 restaurant **threshold**; this new REST-A begins **after** that threshold. #175/DONUT-PROG owns the jump capability and ledger plumbing, #183 CARL-PROG owns the unlock ledger. #182 owns full-playtest integration and hot-file merge queues. #176 owns shared hazard/prop physics; #178 owns Hoarder/Dozer revenge/Rot encounter designs; Juicer boss state belongs to its existing boss scene.

## E. Acceptance (future implementation PRs)

| ID | Required test | Evidence |
| --- | --- | --- |
| REST-A-01 | From M3 llama exit into fast food | Real in-engine visual/audio contrast; no stray enemies/projectiles following invisibly |
| REST-A-02 | Room can be crossed with no optional interaction | Safe traversable floor; no forced purchase/quest/menu |
| REST-A-03 | Reward `f01_safe_room_reward_found` | Exactly once after reaching box; one notice; persists through retry and solo/2P ownership |
| REST-A-04 | Test true Donut jump before/after reward | Actual 128px ledge reach only after boost, no double jump; Carl route remains valid |
| REST-A-05 | A1 character-gate or explicitly approved A2 | 1P swap, solo Donut-active, local 2P all reach protected exit; no AI precision-only gate |
| REST-B-01 | Labyrinth transition and survivor-camp safety | No unintended hostile damage/spawn; recognizable human occupancy, two-player camera |
| REST-B-02 | Revisit/fast advance | No dialogue/animation that traps or imposes unskippable 60s |
| REST-C-01 | Juicer defeat -> rest without duplicate unlock | Story ledger grants only once, restart-safe |
| REST-C-02 | Optional gauntlet/blink affordances | Actually usable but never mandatory; Puddle Jumper destination fully collision-safe |
| REST-C-03 | Transition to Ball of Swine / checkpoint | No stage-state softlock or carried hazard, no premature boss activation |
| REST-ALL-01 | Both solo hero controls + real 2P, 640×360/400, 1080p | Native footage and path continuity; all silhouette-readable |
| REST-ALL-02 | Default live Playtrace and same-input verification | Correct reward flags, interactions, checkpoint and scene transition events/hashes |

**Mandatory experience question:** after each refuge, do players remember the previous fight more clearly and feel excited about the next part? Quiet scenes are not a 3-minute forced puzzle or a mandatory exposition dump. **Human at-speed playtest determines whether the duration targets are appropriate.**

## F. Implementation / ownership strategy (design PR only)

1. Have an agent review proposed **REST-A** against #181 threshold and the merged DP-Q02 jump grant, then build a **short native-size greybox** and prove a safe transition and valid 128px test/route.
2. Add the approved 1990s fast-food environment as an original native pixel kit, validate 1x/2x export and asset provenance; small optional jingle is original or correctly licensed. Do not use concept PNG as gameplay art.
3. Stage REST-B as a **static people/prop layout** before expensive NPC/dialogue work; test co-op progress and camp ambience. Script any original survivor dialogue separately with narrative review.
4. REST-C only after the Juicer story ledger/abilities are merged and the Ball of Swine transition contract is known; never set fake flags to make a campaign build pass. Use fixtures to test known post-boss states without shipping an unintended debug power loadout.
5. Integrate through #182's queue and validate actual full Floor 01 checkpoint order. **No new geometry system, camera rewrite, companion mobility or reward framework** belongs in these quiet scenes.

**Creative decision requested from owner before implementation:** Does A1's 128px Donut-only moment feel like a fun 10-second upgrade reveal or an intrusive roadblock in the arcade flow? A2 is the available fallback, but it explicitly needs the corresponding progression-contract approval. The visual gag and human-camp props remain original proposals to assess after native 1x tests.
