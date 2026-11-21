# FLOOR01-TROG-RUNUP-01 — The House of Broken Weights (approach to Juicer's Gym)

**Status:** owner-invited **design proposal** and local-agent implementation brief, NOT implemented/owner-playtested. This is an original Floor 01 gameplay adaptation using the **already owner-selected troglodyte family art** and the **existing playable Juicer boss scene**. Do not represent invented fights, timings or staging as literal novel events.

**Position:** after the Meadow Lark survivor camp (#187), then a series of troglodyte fights and the gym reception/turnstile, ending at the **existing** `--scene boss_f01`. Afterwards the post-Juicer rest and Ball of Swine are other scopes (#187 and the Ball arena documents). This is a **lead-in**, not a rewrite of Juicer, a long new tutorial or a new combat engine.

## 1. Dramatic goal

**"I learned to read these creatures' bodies; then I walked into a gym where the biggest one knows every way to hurt me."**

Distinct emotion map:
- **Horror → recognition:** coming from the human camp, we first hear a deep repetitive slam through stone.
- **Confidence:** the small Pygmy teaches why staying still and mashing is unsafe; the first correct dodge-and-punish wins quickly.
- **Power:** Basher's great weight makes the environment move, but an overcommitted punch leaves him open.
- **Discomfort:** Virtuoso's long tongue controls an approach lane at a distance. You must recognize a *different* silhouette/tell.
- **Synthesis:** two different threat profiles overlap in an absurd 1990s gym threshold; a true environmental contact may change real cover.
- **Intimidation:** an empty reception/turnstile, cracked mirrors and an ominous *heavy* pounding sound from beyond the boss door. No forced additional pack just before Juicer.

**Signature moment:** in the mixed encounter, the Basher commits a ground smash and **actually hits a weak workout bench**, tipping it into a low barrier and opening a different firing/flank lane while the Virtuoso retracts its tongue. A skilled player can bait this to create an advantageous opening and finish the pair with Carl's melee / Donut's earned Magic Missile. But **fighting directly must be at least as viable**. This is a bounded localized physical consequence, not a chain explosion, a second falling rack, or a new full-blown rigid-body solver.

**No spoiling the boss:** players should see the Juicer's gym, but *not* be forced to solve its rack-crush mechanism before fighting him. The racks and wall-bait charge are the boss's distinctive big payoff.

**First-run time hypothesis:** ~6–8 min from camp exit to boss-room gate, experienced ~3–4 min. Three teaching encounters and one mixed fight are paced by natural movement, not invisible stage timers. Adjust from first actual playtrace; keep the lead-in significantly shorter than an entire separate floor.

## 2. Authoritative existing material and what is NOT yet present

| Scope | Verified existing state (2026-11-21) | Constraint |
| --- | --- | --- |
| Troglodyte art | `TROG-01` Pygmy **40px**, `TROG-02` Basher **66px**, `TROG-03` Virtuoso **66px**, with original native/editable art and read sets | **Art exists, not gameplay AI.** No assumption these three enemies can spawn/attack in current story scene. Use approved silhouettes; do not stretch the sprites or rip concept sheets. |
| Pygmy animation | Coiled leap **24-tick tell**, ~11-tick launch/strike and 14-tick spent landing in original readability set | Range, knockback, damage and hitboxes are **unapproved gameplay hypotheses**. Do not equate art playback tags to final physics. |
| Basher animation | Overhead fists **36-tick tell**, ~11-tick slam, 20-tick recovery; no airborne tag | Smash area, damage, poise and collision effects still need new gameplay tests. |
| Virtuoso animation | Tongue feint **30-tick tell**, ~8-tick lash, 18-tick retract; illustrative tongue hitbox can reach around **x+56..78 at y+34..48** | Reach/damage/poison are not approved; **no poison** in v1. High attack may literally pass above grounded Donut; verify real hurtboxes rather than invent homing tongue. |
| Juicer boss | Existing `src/boss.rs`, `src/boss_scene.rs` and `assets/data/bosses/brute_boss_f01.json`, scene `boss_f01`; **36HP**, 3 phases, 36-tick pound tell, committed charge at **300px/s**, wall-crash stun, optional rack crush | **Do not replace/regenerate this boss.** Reuse its data, rules, gate/scene and headless tests; propose tuning after measured game feel in separate PR. |
| Juicer arena | Locked **640px** wide single screen, ceiling **224px** above walk line, left rack enabled / right rack **disabled by default**, protected 2P entry gate | No new arena cutscene, mixed-room boss spawn or re-authored rack geometry in this lead-in. |
| Character powers | Carl kicks/punches and may have post-Hoarder **dynamite** under the integrated K1 chronology; Donut has Magic Missile and the pre-gym **Light on Your Feet** reach boost | **No Juicer gauntlet or Puddle Jumper yet**. They grant on boss defeat through the existing story ledger. |
| Setting | Owner-selected **1990s industrial catacomb gym** with turnstile, reception, bench, cracked mirrors, workout stations; base palette is red-brown brick with **localized** fluorescent accent | The standalone panorama is an art-direction reference, **not** native-scale terrain or interactable geometry. |

The source novel describes troglodytes with reptilian traits and Juicer only a little taller than Carl; the approved conceptual art exaggerates humanoid silhouettes and the current boss is 100px high. Preserve the documented creative tradeoff and avoid claiming the chosen sprites literally match book anatomy.

## 3. Topology — four action pockets, then reception

**Initial authoring sketch:** ~**3200 world px** in 16px units, with existing game's 640×360/400 virtual cameras and correct floor/ceiling physics. Coordinate spans below are local to this encounter, **not** the global Floor 01 ENV-02 x positions; adapt to actual loader/level sidecar once checked.

| Pocket | Local x target | Layout and cast | What changes in the player's skill |
| --- | --- | --- | --- |
| **T0 — Impacts through stone** | **0–320** | Flat clear approach from camp; damaged masonry, knuckle marks and muffled gym metal sound. No surprise hit at spawn. | Read the area identity before the first enemy. |
| **T1 — The low hunter** | **320–896** | Broad ground + one low side refuge. **1 Pygmy**, optional second only after proof of the first's readability. | Small coiled pose → airborne pounce → vulnerable landing. |
| **T2 — The fist-shaped hole** | **896–1696** | Wider flat heavy-fight lane, no mandatory shelf; one cosmetic crackable wall or safe low cover. **1 Basher**. | Big wind-up/smash; punish 20-tick recovery, use spacing not repeated point-blank spam. |
| **T3 — The bell and tongue** | **1696–2256** | Narrower **but fair** wide-enough lane, one 32–64px optional ledge, a clear ground advance. **1 Virtuoso**. | Range and elevation: a tongue can reach ahead while leaving another approach. |
| **T4 — The test of strength** | **2256–2880** | 1 short **mixed** combat pocket, 1 weak workout bench, 1 short optional retreat shelf; **Basher + Virtuoso** staggered at first; optional Pygmy reinforcement **only** if camera/threat cap and human tuning allow. | Simultaneously read two tells and exploit a player-authored environment change. |
| **T5 — The reception** | **2880–3200** | No required enemy. Broken gym entry door, receptionist counter, **separate turnstile**, weight-room light/sound. Protected checkpoint just before Juicer scene entrance. | Anticipation, relief, confident boss readiness. |

**Geometry:**
- Continuous protected rightward floor in every pocket; no lethal pits, required timed companion switch or blocking fallen equipment.
- A high Donut-specific ledge is **optional**. Any elevated landing region must respect her already granted boosted jump (~128px) and **DP-Q45's 228px maximum lane-ceiling clearance rule**. Do not create an unfair unreachable ranged-turret position.
- Carl must be able to reach/kill every mandatory enemy using available physical attacks, without dynamite. Donut can defeat them with normal attacks/mana/movement; no precision cooperation hardgate.
- Each new enemy activates **only after its tell can be seen** on the shared camera; avoid a Pygmy jumping into the rear player from offscreen.
- When a player leaves T4 the scene ends with a readable noncombat transition; **boss rules happen only in the existing `boss_f01` scene**, and that scene closes its gate only after both active heroes are inside. Do not overwrite its x96 engage marker/gate tests.
- The goal is roughly **two screens of distinctive gym approach**, not a labyrinth or a mandatory weightlifting minigame.

## 4. New enemy gameplay profiles (minimal, authoring proposals)

### Trog A: Pygmy — committed low predatory leap

**Visual:** hunched 40px body, coiled crouch, then a clear airborne arc and an ungainly landing. Readability set has 24-tick tell and 14-tick landing recovery. Actual leap height, horizontal distance, collision and damage must be **engine-side data**, not fabricated from pose pixels.

**First gameplay tuning hypotheses** (not balance signoff):
- HP **2** initial candidate; direct Carl/Donut attacks may interrupt the coiling state if existing rules and test evidence support it. Don't give invisible hyperarmour.
- Leap start range ~**64–160px**, no target tracking once launched, bounded real jump trajectory via the same hero/enemy physics conventions where possible.
- One actual low-body/claw contact per attack ID; **1HP** if grounded intersection, normal i-frame rules; a miss produces **14-tick** recover (may lengthen if actual human punish feels too tight).
- **Antistationary:** a player who never moves while an active pygmy is free to leap should be exposed to an honest, avoidable threat, rather than win solely by standing outside every enemy's functional range. Do not teleport the Pygmy to Donut or cheat projectiles.
- Direct kick hit and launch into scenery: only when true CF02 rules support it. Enemy-to-enemy collision HP remains 0 unless a separate approved contract explicitly changes it.

**First room should allow a single clear victory**. Pygmy is not a mini-boss.

### Trog B: Basher — slam and vulnerable weight

**Visual:** 66px heavily muscled body with wrapped forearms, a clear tall two-fist wind-up, then floor smash. Current readability tags support **36-tick tell → 11-tick smash → 20-tick spent recovery**.

**Initial gameplay hypotheses**:
- HP **4** candidate, subject to direct fight duration: more durable than Pygmy but **not** a mini Juicer HP bar.
- Real short smash box in front (author in the native frame pose); **1HP** candidate on collision, no automatic full-room shockwave. **No Juicer shockwave** or tank-like committed charge for this enemy.
- Player can move behind, jump or step outside the actual footprint and punish the spent recovery. Boss previews **timing and shape**, not exact move set.
- A real Basher smash may damage an authored **weak bench** if its hitbox overlaps, turning the obstacle into a low traversable remnant; this mechanic should be learned only in T4, not force a prop tutorial in T2.
- Avoid unconditional super armour: first test ordinary poise/hit interrupts. Do not turn 4HP into 12HP just because Donut can cast from a safe line; improve approach/reach/cover with honest physics.

### Trog C: Virtuoso — telegraphed mid-height tongue

**Visual:** 66px tall/lean, purple robe and bells, visible tongue bunched during a **30-tick feint**; an 8-tick straight extension; **18-tick retraction** opportunity. This is a **long-range melee contact**, not magic, poison, exploding slime or a ranged projectile.

**First gameplay hypotheses**:
- HP **3** candidate; long tongue collision starts with the art's **x+56..78px, y+34..48px** illustrative extent, then must be validated against actual hero hurtboxes and scale.
- Tongue travels in a committed **fixed horizontal line** set at tell, not a homing lash or 180° snap. Once it whiffs, a real recovery allows a melee punish.
- The proposed high hitbox can **pass over grounded Donut**, analogous to the high Bad Llama glob. This is **not automatically a bug** and must not be patched with invisible downward hitboxes. The scene uses **one additional Pygmy or a short cover/positioning challenge only if necessary and tested** to keep grounded Donut combat nontrivial; do not make the Virtuoso naturally hit an unhittable protagonist for the sake of test scores.
- If actual Art-to-hurtbox probes show standing Carl is outside the strike lane too, revise authentic tongue angle/scene elevation **with explicit animation/geometry review**, not unseen auto-hit.
- **No poison effect on Floor 01 v1.** It is not present in the original animation and does not improve the learning arc.

**Targeting / solo:** Attack an observable, reachable actual living hero; not always Carl's x. A target above or below the tongue lane requires **repositioning**, not an impossible attack script.

### Data and shared systems

All three new enemy types need a legitimate **new-enemy scene slot** (the merged #182 integration plan's `ENEMY-SLOT` owner, with proper hero hits, death, draw, audio and companion threat feed). **Art-ready != runtime-ready**. Implement each in narrow modules/fixtures and register it via the owned adapter; don't build a second combat AI core or large switch statement in a shared hot file. Reuse fixed 60Hz state, actual source attributions, reset and Playtrace/Autoplay as available.

## 5. The main mixed fight — The Test of Strength

**Room:** a small anteroom where the stone floor turns into gym tile, mirror fragments hang, a practice bench sits along one side, a Basher and Virtuoso can be seen at **different x**. The bench is not the boss's actual rack and not a third player weapon system.

**Enemy composition:** Basher enters with a clear smash tell and one grounded lane; Virtuoso enters only when its tongue and its whole body can be shown in view. **2 simultaneous enemies max at first**. A Pygmy is an **optional** delayed add-on only if human trials show clear threat separation and 2P camera is stable; **do not require** three different types concurrently for v1.

**Dynamic beats:**

1. **Open lane:** one enemy is already visible; the second is readable but out of combat. Players choose to rush, cast or hold position.
2. **Distinct two-tell problem:** Basher overhead fists means close-area slam; Virtuoso high mouth/tongue means midrange line. **No two simultaneous unavoidable overlap hits** at a natural initial spawn.
3. **Optional environmental break:** if a **real smash hitbox overlaps the weak bench**, its state changes (`intact` → `dislodged` → `settled_low`), with one audible wooden/metal impact. This can **shorten/re-route line-of-sight** to the Virtuoso or create small temporary protection from its tongue. Only actual physics/collision/prop interaction causes it; **never** a scripted `tick 60: break bench`.
4. **Player-created reversal:** bait Basher into hitting the bench, move around the now-low remnant, punish one enemy while the other is in recovery. The bench may be broken deliberately by existing physical attacks if the material rules actually support this. The final blow remains the player's.
5. **Direct alternative:** players ignore the bench, clear both via honest punch/kick/Magic Missile, and move right. Environmental play should be more *distinctive*, not mandatory or automatically overpowered.
6. **Transition:** the last opponent hits the floor; an empty turnstile beyond comes into view. There is a moment of silence before the boss.

**Not allowed:** a complete weight-rack crash that pre-solves Juicer, a physics chain with 10 objects, flying dumbbells generated from nowhere, forced bench destruction before opening exit, auto-scripted environmental kills, standard stomp shockwaves everywhere, a prolonged wave gate, or three enemies always spam-interrupting a solo player.

### Optional bench state and constraints

Only author bench breakage **after confirming a compatible existing BOIL/#176 `env_prop` / impact interface has merged**. If unavailable, use a **static, traversable bench** with real collision/cover and ship T4's direct-combat variant first. No separate custom physics engine to move a bench. The bench state must be deterministic, bounded, stage-resettable and always allow both heroes to pass after it settles. It cannot strike heroes without a clear physical collision; no damage just for being close to the furniture.

## 6. Don't accidentally teach a wrong lesson about Juicer

The existing boss **is not just a larger Basher**:
- Basher has one ground slam and a short recovery, whereas Juicer has multiple phases, a 36-tick pound with phase-two shockwave, a committed **300px/s** charge that crashes into the wall, **36HP** and independently authored rack-crush opportunities.
- Pygmy and Virtuoso collectively teach the player to **watch low/high threats and move through openings**; Juicer tests whether the same principles hold under a much larger, stronger opponent.
- **Juicer's default left rack** needs three distinct hits to its strut and a 36-tick fall to cause 4 boss damage and a 90-tick pin; the **right rack is disabled by default**. Do not enable it just because the new gym lead-in has Donut's later jump boost.
- **Boss's 36HP duration is a real game-feel risk.** Require an at-speed solo and 2P time-to-clear measurement with and without the left rack. If the fight becomes a repetitive sponge *after* good enemy education, propose a focused boss-balance PR after owner review—**do not quietly retune `brute_boss_f01.json` in this scene-design PR**.
- Only **after** `EnemyDefeated {target: brute_boss_f01}` (per existing ledger) should Carl receive his gauntlet and Donut receive Puddle Jumper. The post-boss relief/exercise beat is separate PR #187.

## 7. UI, camera, agency, failures and readability

- Source 640×360 and 640×400 native virtual size. In T1–T4 keep **one visible tell at a time on initial contact**, then allow bounded overlap later. Enemy art is native; silhouettes not obscured by heavy gym signage or opaque foreground props.
- No boss bar for normal troglodytes; use normal enemy hit reaction/hurt feedback, no elite armour icon unless a game mechanic truly supports it. The gym doorway/turnstile is a **landmark**, not a real shop/menu.
- Safe approach after Meadow Lark; enemies may not spawn/strike through the camp entrance. After T5, boss's existing gate/camera own the transition; **do not** let T4's moving bench or projectiles leak into `boss_f01`.
- Solo Carl active, solo Donut active (human swap), real 2P with two controllers, proper companion AI autonomy from its independently owned branch. No mandatory timed two-human switch.
- Respect **Donut's smaller ground hurtbox**: both her advantage and its tradeoffs must be physically truthful. Attacks should not target Carl alone; no globally recast AI targeting logic in scene fixtures.
- Escape and retry: fixed checkpoint before entering the gym boss and direct T2/T4 checkpoint probes. Unlocked magic, jump boost and dynamite persist; Juicer's special rewards do not appear until defeat. Reset enemies, bench state and authored room hazard through the real retry semantics.
- Do not inject extra death gore or camera slow motion just because every Basher kill is heavy. The rare cinematic kill reward remains skill-gated under existing policy.

## 8. Acceptance tests and autobot challenge

| ID | Headless or actual gameplay probe | Required result |
| --- | --- | --- |
| **TROG-01** | Native first Pygmy reveal | 40px silhouette, 24-tick readable coil, 11-tick attack / 14 landing display does not hide collision |
| **TROG-02** | Pygmy leap hit vs lateral escape | Honest committed arc; one impact, no horizontal tracking and visible recovery |
| **TROG-03** | Carl punch/kick and Donut magic/pounce against Pygmy | Both characters can kill quickly; ordinary fight fun without special mechanic |
| **TROG-04** | Basher first slam vs walk/jump flank | 36-tick visible tell, bounded actual hit, punishable 20-tick recovery |
| **TROG-05** | Basher damage/poise and spell counterplay | No artificial magic immunity, repeated stunlock or excessive 4HP grind |
| **TROG-06** | Virtuoso strike from +0/+32/+64px ground elevations | Tongue/hurtbox truth including possible ground-Donut avoidance; no unseen tracking |
| **TROG-07** | Solo Carl and solo Donut direct win against Virtuoso | Real closable distance, real punish recovery, neither hero lacks a legitimate damage path |
| **TROG-08** | T4 two-enemy target/telegraph schedule | At most 2 initially active, no initial double-hit from beyond camera and one real safe response |
| **TROG-09** | Basher actual smash→bench | State/LoS only change on true overlapping impact, at most once; author attribution |
| **TROG-10** | Direct T4 victory with bench untouched | Exit reachable, no mandatory gimmick, reasonable fight time |
| **TROG-11** | T1–T5 scene route, checkpoint and story flags | No camp damage leak, no softlock, correct ability chronology and legal boss handoff |
| **TROG-12** | Two-player separation and solo AI | Every threat visible, compatible shared camera, no companion-specific unbounded deaths |
| **TROG-13** | Damage/restart/readability at native resolutions | Exact fixed-step hash/event after retry, true 640×360/400 and 1080p captures |
| **TROG-14** | **Standing-spam exploit sweep** at T2/T3/T4 | Static Donut Magic Missile from natural positions doesn't reliably trivialize all fights with no reposition; test without turning first Pygmy into sponge |
| **TROG-15** | Active-player movement/boss preparation | Mobile Donut, melee Carl, 2P playtests succeed and have meaningful dodge/punish decisions |
| **TROG-16** | Juicer follow-through | In existing `boss_f01`, compare with/without rack, duration, damage taken and 2P fairness; no silent boss tuning |
| **TROG-17** | Replay and no-content illusion | Playtrace/fixed-sim replay, real gamepad footage, exact current asset provenance; mark unsupported AUTOPLAY scenes blocked |

**Anti-degeneracy checks are active tasks, not paperwork.** Extend the in-repo Rust headless party/scene fixture minimally for these enemy types. From representative natural T2/T3/T4 positions, issue valid **new Primary press edges** each legal cast and track actual mana/release/hit, enemy damage, companion contribution, seconds stationary and hits taken. Repeat a **mobile Donut**, **Carl melee** and **true two-input 2P** strategy. If stationary full clears are consistently easy from a natural point, fix AI approach/tells/room geometry first; not Donut's cost/damage. Do not claim existing party-AUTOPLAY supports these new rooms until the agent actually wires them.

**Evaluation outputs:** JSON/CSV run summary and deterministic seed/input/hash evidence where feasible, plus real in-engine at-speed 1P and two-pad video/screenshot evidence. Automated success does **not** certify game feel.

## 9. Local implementation path and dependencies

1. **TR-A — Actual code and authoring audit**: read existing TROG-01/02/03 native spec/atlas manifest, active scene enemy-slot implementation, current party/camera, Juicer boss scene and current merged #182 integration/ledger. Do not rewrite global AI.
2. **TR-B — Three isolated enemy profiles**: one **Pygmy**, one **Basher**, one **Virtuoso**, each using the existing ENEMY-SLOT integration contract and current true art frames/attack colliders, data-backed and tested. Prioritize Pygmy and Basher if resource-constrained, keep Virtuoso explicitly gated rather than invent a placeholder ranged wizard.
3. **TR-C — Authored corridor T0..T3**: no new generic level loader; preserve protected rightward route and readable enemy-first encounters.
4. **TR-D — T4 mixed fight + static gym bench**: direct-fight fun/solo/2P first. Optional dynamic bench and source-attributed collision **only** after BOIL prop/contact interface is merged; leave clear blocked gate if not.
5. **TR-E — Gym reception and boss entry**: visually link art mood and protected checkpoint to existing `boss_f01` relaunch, no duplicate boss code or override of its co-op gate.
6. **TR-F — At-speed quality loop**: run TROG-01..17, headless standing-spam exploit search, native screenshots, real solo Carl/Donut and two-pad co-op. Measure Juicer fight duration separately without changing it, and report honest unresolved feel issues.

This design does not authorize implementation by itself and must be split/queued under the merged #182 integrated playtest plan (hot files). Scope should be implemented in a **separate focused PR(s)** only when dependencies are ready.

## 10. References

- `docs/02-art/trog-01-spec.md`, `trog-02-spec.md`, `trog-03-spec.md`; native source in `assets/source/characters/troglodyte/`; `assets/concept/floor-01/enemies/floor01-troglodyte-family-approved.png`.
- `docs/02-art/floor-01-juicer-gym-standalone-concept.md` and `assets/concept/floor-01/juicer-gym/floor01-juicer-gym-standalone-panorama-approved.png`.
- `docs/02-art/floor-01-boss-design.md`, `docs/02-art/juice-01-spec.md`, `assets/data/bosses/brute_boss_f01.json`, `src/boss.rs`, `src/boss_scene.rs` (existing boss).
- `docs/01-design/combat-first-encounter-design.md`, `docs/01-design/floor-01-gameplay-progression-v0.md`, `docs/03-tech/donut-prog-01-floor01-ability-progression.md`, `docs/01-design/character-progression-horizon.md`.
- `docs/07-agents/f01-playtest-integration-plan.md` (#182 merged) for shared enemy/scene owners and existing stage test composition.
- [Meadow Lark quiet approach PR #187](https://github.com/pando85/dungeon-floorbreak/pull/187) and [Broken Ring PR #189](https://github.com/pando85/dungeon-floorbreak/pull/189) for the camp/lead-in seam; `docs/01-design/floor-01-ball-of-swine-arena-gameplay.md` for the separate final boss.
- [Goblin Boiler PR #176](https://github.com/pando85/dungeon-floorbreak/pull/176) for future shared bounded authored environmental interactions. Distinct from the gym bench and not to be reimplemented here.

**No claim of franchise-canonical layout, enemy mechanics or novel dialogue.** These are original, deterministic action-game design proposals on top of selected art and existing controller/gameplay contracts.
