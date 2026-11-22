# F03-CIRCUS-01 — Playable circus arc, S02–S06: authored combat and setpieces

**Status:** PROPOSAL / design-only, not implemented, tested, or owner-approved. 2026-11-22. Refines chapters B–C of [Floor03 Over City bible](floor-03-over-city-design-bible.md). Read [global combat contract](combat-first-encounter-design.md), [Floor03 art direction](../02-art/floor-03-over-city-art-direction.md), [no-inventory policy](../05-decisions/ADR-0010-combat-first-progression.md), and [Grimaldi nonlethal encounter](floor-03-grimaldi-resolution-and-coop.md). Prefer established room/encounter/camera/party and `env_prop` APIs, not a new circus-only physics engine.

## 0. Product target

"Metal Slug gone wrong in the world's cruelest carnival."

**Three signature memories** the player should be able to create:
1. "I kicked a knife-throwing lemur off its balcony, its body hit a rope, and a huge striped awning crushed the next two attackers." This is a **real causal physics chain**, not a scripted camera shot.
2. "Donut dodged the roller-skating bear's first charge, I launched a stunned circus performer into a smoke curtain, and we caught the bear as it skidded out of control."
3. "We fought our way into the tent, made its defenses collapse, and found out the thing inside wasn't simply evil. We didn't kill it."

**Five emotional movements:** excitement -> ridiculous escalating danger -> horror/grief (Heather) -> monumental tense confrontation (Grimaldi) -> warmth/silence (Mongo). Two short breaths: after street ambush and before entering the central tent; end with quiet room.

**Critical rules:**
- Carl kicks; he doesn't need a grab or item-carry button. Donut uses earned magic and agile physical.
- Direct combat against **ordinary enemies and Heather** works without a single optional prop, special, ally or perfect co-op setup.
- Grimaldi is a **nonlethal story elite** resolved by successful combat defense and one legible contextual contact, not a typical HP bar or a bullet sponge.
- No mandatory synchronized two-player levers, no 3-hit "pole puzzle", no extra RPG/inventory screen. Every control input stays approved.
- A physical setpiece can fail; the fight still completes. Every collision is authoritative, with root actor/cause attribution.
- Sourcebook has a solo Carl while Donut is cursed; in our 1P/2P adaptation **both remain controllable** during critical path. This divergence is explicit, not a canon claim.
- The story's protective ward and three support poles can inspire architecture and a controlled collapse, but not an arbitrary hardcoded three-pole lock that stops punches from mattering.

## 1. Global geography — one continuous circus, not five little level-select screens

The real in-game space is **two connected authored chapters**: Chapter B (S02 street fair + S03 lion ruin) and Chapter C (S04 Heather arena + S05 Big Top + S06 rescue), with short safe transitions. The internal SXX labels are source-data IDs and replay checkpoints.

Design widths are **unvalidated proportions** and intentionally expressed in camera-sized modules: the actual character collider height, jump envelope and virtual view (640×360 and 640×400) win. Avoid inventing a global Y coordinate. Final stage JSON comes from inspection of real parser and protected route checks.

### S02 — KNIFE THROWERS DON'T NEED AN AUDIENCE

| Pocket | Approx. camera lengths | Architecture | Combat beat | Optional spectacle |
| --- | --- | --- | --- | --- |
| B0 Entrance of Lies | 0.9 | cheap boardwalk/crowded fake ticket gate | *one* Former Circus Lemur with committed knife | launched body dents fake ticket window |
| B1 The Balcony | 1.3 | two-story tilted apartments, one **reachable** knife balcony and broad ground lane | lemur + low-health rusher; angle control | launch rusher into support/strapped knife kiosk |
| B2 Painted Smiles | 1.5 | tall terror clown over ripped red/gold awning | one tall clown + previously known lemur, staggered | authentic awning-support collapse, 1 heavy impact |
| B3 Lost Parade | 0.8 | empty giant giraffe-shaped **carnival puppet**, detached carts in silhouette | short known 2-actor direct finish | optional small roll-to-cart contact |

**Pacing:** first new ranged family is learned alone before combined pressure. B0 can be completed by either hero in one honest normal combat exchange; no dialog box explaining the knife. B1 launches are predictable enough to learn "bodies are weapons". B2 forms the **first deliberate big prop-chain punchline**, then the level decompresses rather than throwing five more clown enemies.

**Lemur attack**: face/knife hand enters an identifiable windup, target sampled and locked at fire commitment, knife travels physical line; no postcommit homing. If knocked down before release, no ghost knife fires. Lemur can be struck/reached by both heroes without needing to reach a high invulnerable platform. Target elevated platforms must have safe exit and companion access under actual F01 height classes.

**Terror Clown**: towering stilt silhouette, but its damaging body isn't an enormous arbitrary aura. Telegraph one massive straight/diagonal downstrike by locking a stilt and making a visible floor mark. A properly-timed Carl kick can target a near-ground stilt and cause stumble; Donut has legitimate pounce or missile contact. One distinct recovery with genuine attack opportunity. Do not make an ordinary clown a new 3-stage boss.

### The B2 awning contraption

The awning is an original *physics setpiece* built from existing support/prop kinds if possible:

- `AwningIntact` -> `SupportCracked` -> `AboutToFall` -> `Collapsed`.
- `SupportCracked` requires actual melee contact, a sufficiently energetic launched enemy/body, a valid projectile token or authorized blast; a VFX-only attack is not enough. Add true root attacker credit.
- `AboutToFall` has enough authored warning before the swept rectangle becomes harmful; **both** heroes can leave the contact area. If the clown is underneath at real collapse, actual collision/hurt resolves once; if it moves out, the awning misses.
- The fallen awning remains **nonblocking for the single protected through-route**, with a low ramp over it or a clear side lane. No accidental corpse/awning/hero overlap softlock.
- Direct fight with awning never triggered must be viable and reasonably satisfying. A Donut Magic Missile may damage support but must not auto-summon a full scripted chain unless source cause / thresholds are met.
- Rare earned cinematic finishing FX only if user skill criteria qualify. Mere awning destruction isn't a spectacle score event.

**S02->S03 transition:** the party moves through an alley where the carnival tune becomes a muffled scratch. No 12-second results screen.

### S03 — INK AND MOLD

Three compact authored pockets around a real broken arcade plaza:

- **M0: First lion.** One Mold Lion, crouch warning then straight committed charge. Its vulnerable landing/recovery is intentionally obvious.
- **M1: Two different cats.** A second **heavy swiping** mold lion enters only after the normal charger has demonstrated its role. The player learns to avoid the swipe and use the charging lion as a launched object against the other. If actual mass/impulse makes that impossible under existing physics, tune the **local enemy parameters** or substitute a breakable pillar contact; do NOT weaken normal common collision globally.
- **M2: The Ink Wall.** A large *presentation-only* flat tattoo elemental passes behind the plaza as Signet's presence; one authored committed moving temporary lane denial may also exist as a real obstacle **if clearly telegraphed**. No third free autonomous ally, no hidden damage from decorative ink.

**One boss-like option, not a mandatory wizard puzzle:** a lion may crack a wide scenery column, creating a one-time dust screen **only** while leaving clear player/attack danger outlines. The core combat remains ordinary: sidestep a lion, punish its recovery, interact physically. In solo, AI partner's attempt to reach a lion must be observed; not all enemies aggro Carl exclusively.

**Story placement:** Signet appears after meaningful contact and the party is pressured toward Heather. Any charm, curse or theater script belongs to brief original authored storytelling, not sudden untelegraphed controller inversion.

### S04 — HEATHER IS NOT THE MONSTER

**Arena concept:** an enclosed animal rehearsal hall with a low oval skating track, two safe side ramps, one **tall foreground cage that provides contrast but is not part of collision**, and the shattered front of an "ice cream" cart as original storytelling. Two short elevated shelves only if their reachable angles can be tested with actual Carl/Donut movement. Heather is a **large bear with roller skates and worm-infected claws** (grounded in secondary chapter recaps), still physically vulnerable to normal combat.

**Opening sequence:** one slow art key of a lumbering circus bear trying to stand on skates; faint parasite movement on the claws; warning floor scratches; then control. The visual message must shift from "funny" to "someone is being hurt." The prelude is short/skippable on retry.

**Boss attack contract (provisional):**
1. `StraightSkate`: compress -> front paw down -> fast **one-direction locked** charge along ground; genuine collision against side apparatus/rail -> stagger recovery. With no rail impact, a smaller recovery exists so direct fights remain viable.
2. `ClawSweep`: one heavy sweeping paw with **separate visible worm mass**; safe low/jump lane exists; 1 committed active volume. No untelegraphed long-range worm damage from cosmetic frames.
3. `RearingStomp`: she rears, raises both wormed claws, one clearly marked localized ground pulse. Body vulnerable on return; no screen-wide undodgeable damage.
4. Optional **infected reach** in later phase only if implemented as explicit, telegraphed *short* lance; never a hidden status/controls debuff or randomly reproducing parasite.

**Interaction opportunity:** Carl can kick a damaged small wheel cart into the arena rail, creating a rail impact and longer legitimate stagger; Donut can use her jump, close pounce or earned magic from the main floor. The cart is NOT a required stun device. A disrupted momentum charge must visibly slow the bear or send it toward the wall, not pass through as a ghost collision.

**Narrative outcome:** Heather is a tragic boss whose parasite has degraded her; a genuinely earned final hit gives **mercy and brief silence**, not a gore spectacle multiplier. The game's rare cinematic slow-mo for exceptional feats must **not** be automatically triggered by this emotionally sensitive mandatory boss defeat. The source's accidental curing/healing and mercy are summarized in optional original visual notes, not an unapproved RNG lottery-ticket ability.

**1P/2P handling:** book event is solo Carl because Donut is disabled by Water Lily, but our game-critical encounter keeps human Donut player and AI partner active. If depicting curse, show a *short severe warning* during before/after cutscene, NOT disabled controls, frozen NPC or zero-HP invulnerable partner for the entire fight. An optional separate faithful-Carl-only challenge could exist later ONLY by owner decision.

**Heather -> Big Top transition:** a safe one-screen dressing area; Signet's flat ink army crosses the distance behind a wall while the heroes still control movement. A genuine brief atmospheric reprieve before Grimaldi.

### S05 — THE BIG TOP IS A MACHINE

**Architecture:** the big tent is enormous in the background but gameplay consists of **three readable combat rings**, all on one primary walking lane with no mandatory switch puzzles.

| Ring | Visual silhouette | Present defending threat | Optional valid environmental advantage |
| --- | --- | --- | --- |
| R1 The Mouth | enormous animal-head curtains and a low split rail | one knife lemur + one low-health circus stage guard | hit curtain support, falling canvas obstructs a genuine enemy knife line |
| R2 The Mechanism | three towering canvas support poles and a rotating mirror fixture, one intermediate stage rib visible | heavy clown + charging lion introduced sequentially | impact against support pole tilts one rib; falling debris contacts actual actors |
| R3 The Root | Grimaldi's huge vine rooted *behind* the actor plane with visible central vulnerable contact zone | bounded infected circus defender and **one vine limb telegraph at a time** | a properly caused support fall creates breathing time and visible exposure |

**Ward:** a protective shimmer visibly stops Signet's tattoo army outside; Carl/Donut cross with the authored party story grant/ticket flag. Don't create a new mandatory equip or inventory step. The ward is a narrative/transition gate: if the ticket/entry flag is missing, an explicit **visible** encounter setup can supply the needed narrative entry, rather than forcing a hidden unsupported collectible search.

**Three supports are visually grounded in the novel**, but they do NOT need to be struck in a fixed order, nor each struck by an explosive. In the game, forward progress comes from regular battle plus safe distance/pressure; **every pole hit is an optional combat advantage** and one line of physical spectacle. If all poles are left intact, defenders can still be beaten and the nonlethal Grimaldi access opens.

**R1 direct fight:** visible lemur aiming and stage guard approach on staggered timing; no ceiling-filling invisible curtain hazard. In player-coop both heroes can see committed lanes.

**R2 pole spectacle:** Carl kicks a clown into a pole base, causing the rib to tilt; Donut attacks the stunned enemy or uses projectile to strike another exposed support, with **one actual contact** on a ring defender. Reward player-created chain only if real contact. A falling pole must not destroy the only main walking line and cannot auto-defeat Grimaldi.

**R3 root defense:** for a short bounded sequence, a big vine limb performs a limited slam/sweep or curved bind: visible committed lane and **no permanent damage** from its ambient giant decorative sprite. Keep at most one independent active giant attack envelope. Real defenders are hittable by normal attacks, with openings following limb recovery. Root is **not** a giant ordinary killable HP boss; attack connections may cause *stagger/sympathy responses* but not hidden attrition HP.

**Resolution gate:** clear the final defender role, survive a last committed limb attack and remain within reachable safe contact ring during exposed root recovery; the existing context interaction (or a verified no-extra-button equivalent) initiates an **authored, brief, nonlethal communication**. Two-player party routes must regroup safely; neither player is punished for not reading a tooltip. Full detail and alternate no-prop route are in [Grimaldi resolution spec](floor-03-grimaldi-resolution-and-coop.md).

### S06 — MONGO KEPT WATCH

**Aim:** emotionally reverse the carnival's cruel spectacle. The hero enters a quiet upstairs refuge where Mongo has defended Donut/others; one small finite Street Urchin scene remains at the doorway. No urgent escort babysitting; no extra required pet commands.

**Practical encounter:** introduce 1 low-health spiny Street Urchin near the wall, then one more from a visible vent with significant warning. Carl can kick it into a door; Donut can perform direct attack. Mongo's **original approved pink/red feathered raptor silhouette** is a visual protection beat or narrowly tested room-local one-off assist, not an unapproved third generic combat AI actor. Story healing is presentational unless a full supported capability has been implemented separately.

**End of circus act:** walk past the now-silent park with the shadow of a Floor04 stair revealed, but the linear game does **not** descend here. A separately marked side route leads to the skyfowl chapter. The final true Floor04 descent occurs after S12. This is an explicit game adaptation: in the novel the stair becomes available already here. The route must label the early stair as visual/not party transition, never let a player accidentally leave before story progression.

## 2. Five controlled high-value player tests

**CIRCUS-PLAY-1** S02 awning: no-prop direct clear vs one authentic launch->support->fall kill and failed swing; input logs and true event cause.
**CIRCUS-PLAY-2** S03 physical lions: two different cues, no undodgeable overlapping threat, Carl/Donut each can deal real damage; screen and replay.
**CIRCUS-PLAY-3** S04 Heather: direct Carl, direct Donut, optional prop, no hidden worm damage; genuinely readable committed skate charge and mercy beat.
**CIRCUS-PLAY-4** S05 Big Top: direct no-support-destruction route, optional 1–3 support hits, real defender attrition and Grimaldi story event; 2P no disabling.
**CIRCUS-PLAY-5** S06 post-quest: once-only story grant, bounded Street Urchins, Mongo recognizable source shape, early stair cannot change party floor.

**Every** test needs present-day source commit/manifest pin, actual supported gameplay policy/action input, fixed tick/event hashes, valid checkpoint reset, both character roles and real 1x game capture where supported. Human fun and real 2-pad test explicitly pending until owner plays.

## 3. Production and art order

- **Physical first:** S02 one real balcony and awning + two enemies -> one hero real collision test -> S03 two lions, genuine impulse.
- **Boss feel:** Heather's skate charge / stagger tested as a greybox before final source art; if it plays like another generic charging beast, change **timing/terrain** not merely add gore/HP.
- **Final narrative:** Grimaldi's three-ring fighting data + nonlethal interaction; proper chapter continuity and separated art/background/body hitboxes.
- **Look and sound:** one original full-size key each of B2 awning aftermath, S04 Heather, S05 Big Top R3, S06 quiet Mongo room; high-tier design, independent edit and native pixel-source execution.
- **QA before scaling:** 1P and 2-slot scripted, real Bevy 640×360/400 clips, in-sim cause/root IDs, replay negative fixtures, F01/F02 goldens. Human fun still PENDING.

**No new self-built escort engine, disease/parasite inventory, circus party system, broad class stat overhaul or extra magic button.** The visible stage should look much more complicated than its core deterministic combat rules.
