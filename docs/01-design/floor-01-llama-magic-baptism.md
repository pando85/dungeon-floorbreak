# FLOOR01-LLAMA-MAGIC-01 — The Infernal Corral (revised design v0.2)

**Status:** owner-approved *design direction* (2026-11-21), **not implemented or game-feel-accepted**. This document **supersedes the former "Throat Choir" / alternating-volley centerpiece** in PR #181; do not build the old two-stationary-llama encounter. It is an original encounter adaptation, not a claim about an exact scene in the source novel.

**Placement:** Floor 01 Act I, immediately after the Tutorial Guild biscuit (I-05 / I-06 / I-07), before the blue-and-white fast-food refuge (I-08). First Magic Missile demonstration is intentionally easy; the main scene tests the **limits of staying still and spamming magic**. It remains a linear arcade encounter, not a puzzle arena, a mini-boss or a metroidvania ability gate.

## 0. Why the previous proposal was rejected

The old design expected challenge from staggering two llama spit attacks and moving between a couple of platforms. Actual current code makes stationary Donut spam a likely dominant strategy:

- A llama has **3 HP**; Donut's standard missile deals **1 damage**, 8-charge magazine and regeneration, with **~240px horizontal range**.
- A llama normally **stands off ~200px**, never deliberately retreats, and currently **cancels a spit tell when struck**. It can be interrupted and killed without closing distance.
- The current lava glob is fired near **42–48px above the llama's feet**, potentially **passing over grounded Donut's ~20×24 hurtbox** without requiring any move.
- Existing targeting may be tied to **Carl** and legacy scenes may lack active Donut→llama damage until a CF02-specific switch is active. The active project team must inspect/resolve these blockers before claiming fair 1P/2P play.

**Rejected fix:** adding HP, magic resistance, invulnerable attack phases, undodgeable homing spits, random additional enemies, a second Donut spell, extra controls or forced platforming. Those would hide the root cause or undermine Donut's earned magical power.

**Revised pillar:** the first llama may be efficiently beaten with Donut's magic. In the *main corral encounter*, the player should need to **reposition, avoid a readable close-range rush, interrupt at the right time, or use the environment**. Magic is strong and reliable but cannot be a universal safe stationary solution. Carl is an effective offensive closer, not a required key for Donut to survive.

## 1. Emotional progression and memorable payoff

1. **M0 — "I can do this now" (first spark):** Guild door opens, Donut has full mana; one/two ordinary goblins in a short unobstructed corridor. Donut sees and lands a true magenta Magic Missile. Carl can finish with punch or kick. No training dummy, spell-choice UI or second button.
2. **M1 — "This thing shoots back" (first llama):** one 64px Bad Llama in a straightforward ground pocket, visible orange throat glow and straight glob. Donut may shoot it down quickly. That **power fantasy is intentional**; teach the threat safely and keep melee a valid alternative.
3. **M2 — "The corral fights back" (signature set-piece):** a broad scorched masonry stock pen, low real cover and one obvious vulnerable timber partition, with **two llamas using the same enemy family** but different initial *positions*, not fantasy elemental variants. One occupies a readable ranged approach; the other can **rush** through an exposed corridor when the player stays planted. Their own decisions and the player's positions produce openings; the optional charge/partition collision changes line of sight and creates a memorable stagger/knockdown opportunity.
4. **M3 — "What is this place?" (rest):** sharp transition into the existing absurd blue/white fast-food restaurant. No required post-corral ambush; give the previous fight a moment to resonate.

**Signature moment:** a llama charges with a distinctive braced-neck/hoof-scrape warning; Donut gets out of its path and Carl punishes the exposed recovery. If baited toward the corral's vulnerable wooden screen, **a genuine collision** destroys it, sends nonlethal splinters flying and opens a new attack lane. A follow-up Magic Missile or kick ends the fight. No scripted "player entered x, so wall explodes" event or teleport into an environmental kill footprint.

## 2. Keep existing combat/visual/controller contracts

| Mechanic | Current contract and implementation constraint |
| --- | --- |
| Donut biscuit | Guild unlock only, persistent across checkpoint; **Magic Missile on Primary**, Pounce on Physical, no unearned special |
| Mana | 8 charges, 1 cost/1 damage per bolt; regen delay **30 fixed ticks** then **40 ticks per charge**; no additional pickups, no free refresh between corral pockets |
| Cast/bolt | 16-tick cast cycle, release on local tick 4; **straight horizontal** ~6px/tick, **240px** max. No vertical aim assist or homing |
| Llama silhouette | Use the approved **64px** LLAMA-01 art and glowing throat; base behavior has a committed tell before actual glob release |
| Llama ranged glob | Fixed physical trajectory and collision, currently travels high relative to grounded Donut; **do not silently re-aim it at her feet** |
| New proposed llama rush | A **single, clearly differentiated short horizontal charge**; only add after reviewing actual llama AI, collision and animation/asset ownership. It must not be present unapproved in every other preexisting llama fixture |
| Carl | Punch, kick, aerial attacks only if already earned/implemented; no dynamite (that comes later) |
| Donut movement/reflex | Independently owned mobility and 12-second biscuit-gated reflex. **Do not implement or rebalance either here.** The charge is a *targeted physical attack candidate*, but whether reflex can evade it requires a **specific review of the canonical classification list** rather than automatic default inclusion |
| Level presentation | Floor 01 scorched stone/brick corral with goblin-built rotten timber, **not** a lava biome/medieval ranch; maintain native 1× 640×360/400 readability |
| Other PRs | Companion movement/autonomy, Carl/Donut progression, Dozer opening, Goblin Boiler, Hoarder/hazards are separately owned |

**Important functional gate:** In current main, Donut's llama targeting may only be wired with CF02 rules and the llama's `step` receives a single hero position associated with Carl. An agent must demonstrate **both** damage and fair threat targeting in actual campaign-scope rules, coordinating the minimal upstream behavior change with owners instead of adding a scenario-specific cheat or silently saying this works.

## 3. Revised geometry — simple forward progress, meaningful combat lanes

**Initial world-pixel authoring hypothesis:** ~**2176–2304px** (about 3.4–3.6 native screen widths), three connected combat pockets and a refuge transition. x grows right, y grows down; choose actual floor line and collider geometry from the *live* level/LDtk/scene contract. Do **not** attach combat-lab `floor=580` numbers directly to the approved opening ENV-02 `floor≈308` without proper transition.

| Zone | Rough span | Layout | Combat/design role |
| --- | --- | --- | --- |
| **M0 First Spark** | x0–448 | Flat, spacious, short partial arch; one low optional step | Plain direct shot immediately after biscuit; 1–2 goblins, staged |
| **M1 One Burning Throat** | x448–1056 | Single visible lava-spitting llama, side refuge/short opaque cover, generous approach on ground | Teach heat warning and fair projectile recovery; **easy Donut victory allowed** |
| **M2 Infernal Corral** | x1056–2048 | **Wide low-ceiling pen**, one **ground-level clear lane**, two optional **32px rise shelves** that are *not* permanent sniper points, one near-ground **vulnerable partition**, two ordinary llamas at different x with a clear camera-intro moment | Long/short range decisions, side-step/jump vs charge, melee/magic punish, optional physical cover destruction |
| **M3 Fast-food Threshold** | x2048–2304 | Protected dry corridor, unmistakable blue/white light | Quiet 10–20s transition to already approved restaurant identity |

**Initial spawn hypotheses (data-driven):** M0 goblins x≈272 and x≈368 with separate activation windows; M1 llama x≈848. M2 llamas x≈1488 and x≈1824, activated by **actual player proximity / camera-safe sight**, never simultaneous offscreen projectiles. These are not production coordinates; verify against exact 240px Magic Missile, 320px glob, charge start range and true camera opening.

### Architectural rules

- There must be a **continuous forward ground route**, and every necessary llama reachable/killable by Carl using ordinary moves. Donut must also be able to defeat the encounter in human-controlled solo mode. No mandatory simultaneous P1/P2 positioning.
- The **cover creates real line-of-sight occlusion** (for both Magic Missile and lava glob), but may not form an indestructible firing bunker. The second llama should be partially shielded by *real existing geometry* until the player moves, **not** auto-invulnerable by a magic flag.
- At ground level, high glob can pass over Donut; **the short rush must threaten that same stationary ground position with a low physical contact box and visible anticipation**. No untelegraphed vertical hitbox extension.
- Optional 32px shelves create routes around the low charge and expose different projectile heights; Donut's horizontal bolt can miss ground targets from a ledge. A shelf is useful for **repositioning/jumping down for a shot**, not a guaranteed height-based damage bonus. Real +0/+32/+64 hitbox probes are a prerequisite to final geometry.
- The vulnerable partition is **outside** the only progression path, clearly in/near a plausible charge lane, and changes actual LoS when destroyed. No scripted map-wide damage. If this material/collision interface is not available, **omit the breakable variant temporarily**, preserving the rest of M2's anti-spam movement pressure.
- Do not force P2 into offscreen attacks or split the 640px shared camera to show both distant enemies. Stage the threat with short camera-safe engagement bands.

## 4. Enemy behavior — add one genuinely readable action, not armor

### Existing baseline (retain)

- **Spit:** llama glowing throat -> committed straight glob -> recovery, real solid collision and actual 3 HP. Existing code uses a **36-tick** runtime windup; art shows **30 ticks**, so audit sync rather than assuming either duration. Hits in current baseline can interrupt this tell.
- Standard physical hit reactions, punch/kick stagger and Donut's actual missile hit must remain legitimate and satisfying. No blanket missile immunities, new giant HP pool, forced low-HP armor or magic suppression.

### Proposed single new behavior: short committed rush

Llama's low forward burst is a **proposed additive enemy behavior** needing animation acceptance and tests. It is not an existing finished animation. Preserve native llama silhouette and character: forelegs brace, neck dips or pushes forward, hooves scrape, chest/shoulder leads. Distinct from throat-glow spit.

**Data-tuning starting values, NOT owner-approved balance:**

| Parameter | Candidate | Gameplay intent |
| --- | --- | --- |
| Rush decision envelope | **~80–180px** forward horizontal gap and visible/valid lane | This isn't a long-distance teleport; player can read the commitment |
| Preparation | **30–36 fixed ticks (0.50–0.60s)** | Exaggerated body shift, hoof scrape and ground dust, clearly different from glowing throat |
| Movement | **210–250px/s for 20–28 ticks** (≈70–117px) | Short close-out/pressure, not a screen-spanning tracking projectile |
| Recovery | **30–42 ticks** | Honest and valuable punish window for Carl melee, Donut Magic Missile and Pounce |
| Contact outcome | **single 1 HP damage instance**, bounded knockback/stagger, existing i-frame rules | Real hazard, no per-frame touch damage spam |
| Cooldown | Longer than the attack's own active duration; initial **~2.0–3.0s** after resolution | Prevent perpetual rush chaining and keep spit relevant |

**Attack selection rules (design target):**

1. Choose from **real live heroes** in visible, attack-reachable positions, not always Carl's x. Do not privilege player slot; solo Donut must actually be threatened. Respect current AI/party safety rules.
2. Llama may **advance to acquire its desired threat range** instead of camping forever at a 200px standoff that is fully within Donut's 240px bolt range. Its own retreat/advance policy must be deterministic and limited to its authored leash, with no enemy-specific magic resistance.
3. A potential rush enters a **committed** state after its tell: aim/facing/lane **locks at commitment**. No tracking through the player's jump, no sudden 180° turn or target teleport during charge. If it misses, it stops and recovers visibly.
4. The spit and rush are **mutually exclusive per llama**. Each has an individual cooldown and actual state transitions; do not hardcode a screenplay of precisely alternating shots and charges regardless of player disruption.
5. No permanent stunlock exploit should be solved by invisible super-armor. First try **real geometry, separate activation sightlines and approach pressure**. If three ordinary bolts always kill the rush llama before it has a genuine opportunity to react, **measure why** and iterate timing/placement/approach logic, not a hidden damage immunity. Preserve the first M1 llama as the intentionally simple power demonstration.
6. Every approach, warning, rush, collision and recovery uses a fixed 60Hz simulation state and a unique attack-instance token; VFX/audio/camera do not affect hit results.

**Animation gate:** LLAMA-01 art presently has idle/walk/glowing tell/spit/hurt/down, **not a dedicated rush pose sequence**. A readable greybox may use clearly provisional keyed states/physics with debug tells, but a production-ready charge needs a reviewed native sprite/animation extension; do **not** pretend the existing spit pose communicates a rush. Any new assets keep the approved editable pixel-art source and provenance.

## 5. Main set-piece — "Don't stand in the same place"

### Corral sequence

**Beat 1 — Sight/read:** entering M2 reveals a visibly scorched enclosure and the first llama. A second is partially shielded by authored solid cover; it cannot fire from offscreen. At least one clear lane remains for a normal advance.

**Beat 2 — Initial exchange:** Donut fires magenta bolts, Carl advances or prepares to punish. A throat glow and/or low-hoof rush warning is genuinely sourced from the live active enemy, not a pre-timed audio track. One threat initially dominates.

**Beat 3 — Positional pressure:** as the player holds an exposed position, a llama can close and enter a short charge with clear tell. Standing completely still is no longer a universally safe posture. Rising to a shelf or evading through normal movement can avoid the lane; the llama may miss.

**Beat 4 — Environmental opportunity:** if a rush **actually hits** the breakable partition, its support weakens/breaks and the obstacle's collision/LoS changes. Player may bait the charge to achieve this, but can also manually break the partition through existing physical attacks if that is an implemented, source-supported mechanic.

**Beat 5 — Punish/finish:** recovery after a missed rush leaves the llama exposed to Carl's kick or Donut's bolt/Pounce. Other surviving threat still requires positioning and attention. A genuinely skillful finishing strike (not routine kill) is eligible for the project's existing rare slow-motion policy, never automatic.

**Beat 6 — Exit:** remaining enemies die or the encounter's stage progression policy permits forward continuation. Scene stays reachable whether the partition is intact or broken. Sudden quiet and the strange cool restaurant light produce emotional contrast.

### Three credible winning approaches

- **Mobile Donut:** fires, changes lane, times shots around a rush, finishes with Magic Missile/Pounce; no Carl-only required gate.
- **Aggressive Carl:** closes distance during spit/rush recovery and uses physical interruptions; no mandatory magic hit.
- **Environmental/cooperative:** lure the charge into a physically vulnerable screen and follow up with real attacks from the newly opened line. This is faster/spectacular **when earned**, not a forced puzzle or free script kill.

**The first llama may die to 3 straightforward Magic Missiles.** That is not a failure; it is intended progression. **M2 must not be routinely solved by the same stationary spam strategy** without ever moving/repositioning, even if mana eventually regenerates.

## 6. Central anti-degeneracy experiment: STATIONARY-DONUT

The major acceptance gate added after owner feedback:

**Fixture:** post-Guild Donut human-controlled, full 8 mana, normal HP, Carl as companion *without requiring AI to make a precision save*. Place Donut at representative ground positions that a real player could naturally occupy when M2 activates (entry, shelter edge, optimal line to nearest llama and one safe-looking perch). Each run uses a fixed deterministic seed/input trace. If the companion deals meaningful damage, **report its contribution separately**; repeat in a diagnostic controlled setup that isolates Donut's actions without changing enemy rules.

**Policy:** choose left/right facing only as a player could, hold fixed x throughout, repeatedly press Primary at legal cooldown/on regen; **no jumps, pounce, reposition or movement**. Also test "press continuously" where edge-trigger means no repeated casts, but do not confuse this trivial no-op with a real spam exploit.

**Expectations:**
- **M1 first llama:** stationary three-shot win is **allowed and desirable** as magical empowerment.
- **M2 corral:** static spam **must not be a reliable, low-risk full clear** across the naturally available positions. The encounter should induce at least one meaningful movement/positioning change or exposure to a fair, dodgeable threat; no guarantee of forced player damage.
- **Human Donut moving strategically** should win reliably with skill, without Carl's precise AI behavior, no skill unlocks beyond Guild and no need to manipulate destructibles.
- **Carl-only and 2P** outcomes must be viable and enjoyable, not dramatically slower/unfair due to magical targeting.
- **No exploit patch** that silently nerfs Donut missile, grants magical immunity, extends projectile beyond its visible line or sets global timers based on player inactivity. Solve using enemy movement/targeting, **real LoS and attack windows**.

**Report:** static runs by location, initial mana/final mana, time-to-clear, hits taken, number of rush tells/commits, interruption counts, actual llama contact/line-of-sight, human-vs-AI damage attribution. Report **distribution across representative locations and variants**, not a single binary "bot lost" claim. The captured Playtrace should show the exact reason for each outcome. Use AUTOPLAY for additional exploration if available; this fixture must work headlessly even when ML tooling is missing.

## 7. Solo/2P, progression, camera, feedback

- Guild biscuit is the only unlock; keep 8 charges persistent and deterministic, no refill between stages except normal regen/checkpoint contract. Fixture starts already-unlocked only if flagged as a test; story entrance may not grant magic before the Guild.
- Solo Carl/Donut and P1+P2 share the critical combat geometry. Do not build a choreography requiring one hero to hold a switch while another dodges. Companion autonomous movement is owned by another agent, not this PR; expose reasonable room approach surfaces.
- Donut's grounded smaller hurtbox **remains** her advantage against high spit. Charge is a readable close-range answer, not a stealth hitbox adjustment. Reflex classification for the new charge must be explicitly reviewed with the movement owner; baseline tests with reflex off and on.
- Two independent attack tells: **orange throat** = horizontal lava glob; **hooves/low posture + dust** = rush. Distinguishable in silhouette at 1×, not solely by color/audio.
- Keep camera framed to show both heroes, a committed charge's lane, and the enemy's tell. Trigger second llama only once fully in visible threat conditions; never spawn an immediate projectile from beyond the camera.
- Cover and shelf height can change where Donut's straight bolt intersects a llama; test and depict collision truth, **never add homing auto-aim**. One safe ground path and room exit survive partition breakage.
- VFX/audio may celebrate an exceptional physical finish but no routine slow motion. Do not turn M2 into projectile bullet hell or conventional HP sponge.

## 8. Revised acceptance and replay matrix

| Test | Setup | Required evidence |
| --- | --- | --- |
| **MAG-01** | Guild transition into M0 | Donut unlock full mana exactly once, same Primary; Carl no unearned special |
| **MAG-02** | First plain goblin | One visible 1-damage post-biscuit missile, release tick 4; Carl melee alternative |
| **MAG-03** | First isolated M1 llama | Three honest magenta bolts may quickly kill it; glowed spit visible with warning if attack occurs |
| **MAG-04** | Rush target at 80–180px | Distinct non-spit 30–36 tick warning, committed heading, no mid-rush tracking |
| **MAG-05** | Hero leaves rush lane vs stays | Real collider overlap only, one contact-damage token, clear post-miss recovery |
| **MAG-06** | Llama tries rush/spit while its tell is interrupted | No phantom glob/rush; state recovers and cannot be permanently locked by unverified exploit |
| **MAG-07** | Ground/+32/+64 Donut and Carl | True bolt/glob/hurtbox heights documented; no fake shots, auto-aim or hidden lunges |
| **MAG-08** | M2 with 2 llamas, staggered visibility | No offscreen projectile; each enemy chooses own action based on real state/target |
| **MAG-09** | Real rush→partition physical impact | Broken only on proper collision/contact; actual LoS and collision update; direct path intact |
| **MAG-10** | 8 shots→dry press→regen→Pounce | Exact existing mana/cast behavior; no mana cheat added to support corral |
| **MAG-11** | **Stationary spam exploit sweep** | Static Donut at representative safe spots does not routinely clear M2 without fair positional pressure; report all cases and failures |
| **MAG-12** | Mobile Donut, solo Carl, P1+P2 | Each mode can finish with existing tools and no mandatory teammate timing |
| **MAG-13** | Retry during rush/spit/breakage | Full deterministic restored states and legitimate retained Guild unlock |
| **MAG-14** | Captured real attempts, replay twice | Stable event order/phase/damage/attribution hash, native 640×360/400 capture |
| **MAG-15** | Three human styles: Donut mobile, Carl aggressive, environment/coop | At least two clearly different successful choices and memorable player-authored beat |
| **MAG-16** | Evasion off/on, audio off/reduced VFX | No hidden unavoidable attack; only intended classified evadable contacts consume reflex |

**Hard review gates:** do not approve the main corral if (a) first player can park at an obviously safe shot line and win M2 nearly every time without moving, (b) a llama attacks only Carl regardless of actual human control, (c) the rush looks indistinguishable from a spit at 1×, (d) Donut loses her intended first magic power spike, (e) simple Carl melee cannot work, or (f) any actor sees offscreen rush damage.

**Game-feel acceptance:** actual gamepad sessions and Playtrace captures, not just deterministic scripted bot wins. Owner reviews whether normal combat remains punchy and whether the charge/partition collapse is actually fun.

## 9. Local-agent delivery plan and dependency gates

1. **C-A — Audit + honest baseline:** check active llama targeting, Donut-llama interaction in campaign vs CF02, current AI movement/leash/spit, actual jump/hurtbox/camera and approved native assets. Build the **M0→M1 direct playable baseline** with no new rush. Measure whether the old static strategy dominates; attach trace.
2. **C-B — Rush prototype in isolated fixture:** one **bounded** deterministic committed low rush, tell/collision/recovery, native-height body and realistic momentum, optional limited facing choice. Validate direct Carl/Donut punishes, test without companion.
3. **C-C — M2 corral greybox:** two staggered existing llamas and real cover/LoS; add rush selection within each enemy's state. Run STATIONARY-DONUT sweep; first adjust **geometry and attack selection**, not player damage or enemy HP.
4. **C-D — Destructible partition:** add only if current support/terrain/prop/contact machinery can do it honestly, preferably reusing BOILER-01 or CP-03; leave as documented gated follow-up if not. Never block getting to M3.
5. **C-E — Presentation & transition:** native charge sprite/animation extension requires art review; sound and tell, contrast from magenta and orange to blue/white restaurant. Don't assume art is ready because concept is approved.
6. **C-F — QA/evidence:** MAG-01..16, playable 1P+2P with real pads and camera, checkpoint, replay hashes, before/after idle-spam tests, CI, and **owner game-feel signoff**.

**No code changes in this specification-only PR yet.** A local coding agent should implement against the actual merged branch and coordinate with independent companion/mobility/progression work. Prefer a small standalone charge PR if enemy behavior/animation needs independent review. Do not mark overall feature implemented until the main corral genuinely passes the anti-spam test and real at-speed review.

## 10. Source of truth and related work

- [Act I beat map](floor-01-act-01-playable-combat-beats.md) — progression M0→M1→M2→restaurant.
- [Donut mana](donut-mana-v0-proposal.md), [controller contract](mvp-controller-contract.md), [character horizon](character-progression-horizon.md).
- [Llama art](../02-art/llama-01-spec.md), [Magic Missile animation](../02-art/donut-magic-missile-anim-03-spec.md), [Level and collision data](../03-tech/level-data.md).
- [Existing lab](combat-lab-01-rooms-r00-r02.md), [movement/reflex in progress](../03-tech/donut-mobility-01.md), [Playtrace recording](../03-tech/playtrace-01-session-capture.md), [Combat-first pillar](combat-first-encounter-design.md).
- [Local work order](../07-agents/floor-01-llama-magic-workorder.md).

This replaces the previous "two staggered spitters are sufficient" hypothesis. **Maintaining both proposals as alternative implementations is explicitly rejected** pending new human review.
