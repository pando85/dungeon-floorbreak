# F02-MR-01 — Meadow Lark Express, Rage Elemental and Bomb Chicken Run

**Status:** detailed S05–S07 design proposal only. No playable escort, chase, camera behavior, hazard or art is claimed. It complements the [Floor 02 bible](floor-02-last-lesson-design-bible.md) and is a local-agent engineering brief. Respect Floor 01 owner-tested movement, party and shared-camera contracts, game-feel baseline and existing story ledger. All world lengths/tick values below are **starting hypotheses**, not final tune.

## 0. Why this block is one dramatic act

S05 makes the player protect *people*, not loot. S06 shows that the dungeon can kill those people for an absurd procedural infraction. S07 allows Carl and Donut to reclaim agency by using the dungeon's own floor transition against an enemy too strong to defeat. This is not a three-room escort puzzle, nor a long generic keep-running-right platformer.

**Primary emotional sequence:** solidarity -> sudden unease -> awe and fear -> desperate intelligent action -> exhausted relief. Give the screen enough silence and meaningful sound contrast to make the huge chase impactful, rather than constant noisy exposure.

**Novel reference boundary:** floor 2's Rage Elemental is level 93 and emerges following a rule violation; Carl uses a bomb-launcher and stairwell exploit, while no normal XP is awarded. We preserve that *idea* but choose short reproducible 2D arcade behaviors. References include https://progressiontime.com/primers/dungeon-crawler-carl/book-01/ , https://www.dccdatabase.com/wiki/rage_elemental/ and https://mapaverse.com/dungeon-crawler-carl/world/second-floor . No copied prose, soundtrack, visual or book dialogue.

## REQUIRED continuity guard — Rage stairwell is not the Floor 03 exit

The S07 pit is a **Rage-only expulsion volume**. Carl and Donut stay on Floor 02 after it vanishes, leaving via a protected adjacent corridor to the S08 kobold kennel. The normal Floor 03 descent trigger remains locked/absent here and activates only after S09 Ralph and Mongo. Any phrase below about "entering the protected stairwell" refers to reaching the **safe adjacent rim**, not the player descending. This is checked by DIR-03 in the [director bible](floor-02-director-and-continuity.md). Never set floor_02_exited in S07. Always distinguish Rage Exiled from a normal kill.

## 1. Stage S05 — MEADOW LARK EXPRESS

**Provisional width ~2048–2304 world px.** Three regions and two bounded convoy defense stops, no procedural cart AI.

| Pocket | Approximate x fraction | Visual/playing objective |
| --- | --- | --- |
| M0 Meet the convoy | 0–23% | 3–4 linked wheelchair silhouettes, warm lamps; brief quiet protection cue; party rejoins |
| M1 Ventilation gantry | 23–62% | first authorized stop, Brindled Vespa high projectile lane and one exposed side lever/roller |
| M2 Broken crossing | 62–100% | second stop, mounted kobold/dingo teaser and a breakable screen; one last pressure flank then safe checkpoint |

### What a human actually does

- The convoy advances automatically when its protected lane is free. It stops at **two visibly marked defensive anchors**. Never require manual pushing or continuously tethering an NPC vehicle.
- Enemies target visible wheel/convoy components, but can cause at most one **telegraphed temporary delay event per stop**, not hidden human deaths. The mandatory lane always has a safe completion fallback after clearing its roster.
- Carl can throw/punch/kick enemies into a suspended road sign or turn a protected side lever to redirect a physical obstacle (existing interact binding). Donut can reach an optional short upper flank to counter the Vespa and return safely. **Neither route requires the AI partner to execute a specific jump**.
- A convincing kill: Carl knocks the rider off the dingo, Donut hits the launched rider as it reaches a wire-caged bin; physical contact stuns another foe. Every event actually occurs and has root-source attribution. This is an optional high-skill outcome, never scripted merely because the player reached x=1200.
- The exit follows an escort checkpoint with a **short true reprieve**: lantern glow, breathing room, only one compact interaction or text beat. Do not force repeated dialogue/slow animation on retries.

### Convoy state machine and fairness

State: Parked -> Advancing -> WarningStop -> Defending -> Resuming -> SafeArrival. Position follows a fixed, authored path with integer fixed-tick progression, not Bevy presentation time. Rendered wheelchairs follow via offsets; allow minor cosmetic wheel jitter, but no authoritative independent pedestrian pathfinding.

Provisional **95–120 px/s** advance, below both protagonist base run speeds. It waits only at the two authored stop boundaries. Characters may run ahead and the convoy should catch up to the next stop; it must not block both heroes with physical collision or turn into an escort that takes 8 minutes. Do not automatically teleport it through damage hazards where players cannot safely follow.

The convoy has a small **bounded disruption counter** per anchor, not an exposed HP pool, third-party babysitting damage loop, or economy. If a wheel is attacked, play a clear tell and make a one-time local delay; repeated damage before a player can respond does not destroy progress. No offscreen permanent NPC deaths; if the story calls for loss, that is separate authored narrative content and not a bonus-score source.

**Stop exit rule:** all required local threats defeated OR the surviving threat retreats through an explicit authored exit. A direct attacks-only path always works; special attacks/props merely affect clearance time and spectacle.

### Two-human and solo camera tests

- Both humans can roam within the active combat pocket without the camera shrinking below legibility. If they diverge, mark a safe camera leash or a soft join prompt at pocket boundary, **not** a damaging forced teleport.
- A solo partner stays near the fight and never attaches to the wheelchair as an unattackable passenger. Require actual offensive actions and survival decisions; the Juicer's passive-partner failure must not repeat.
- The convoy is a visual landmark, not always centered: camera prioritizes player combat and imminent warnings, then a short authored composition at each convoy stop.

## 2. Stage S06 — TERMS AND CONDITIONS APPLY

**Provisional width ~1280–1500 px.** Opening is deliberately calmer. A recognizably official restroom/violation sign, a normal small piece of dungeon admin bureaucracy, then warning lamps turn purple. Far behind a deep concrete service slot, an impossibly large Rage silhouette moves. A protected safe-room door is visible **before** danger becomes active.

**No health bar, no "almost vulnerable" hit reaction, no fake boss fight.** When a player attacks, the contact sparks against an invulnerable hazard and a single concise noncombat warning appears. This is the *one intentional exception* to direct-combat boss viability, justified by narrative and gameplay. Avoid repeating the warning on every punch.

The human consequence is **implied through voice, sound, lantern extinction and the surviving party's animation**, not a voyeuristic or punitive cutscene of vulnerable people being massacred. The player should still feel an immediate duty to save those who remain. The warning/activation is not triggered by a player's input that they could accidentally spam or miss.

### Beat progression (all states observable)

1. **Whisper / Watch** — safe opening: enormous shadow in background, distant structure tremor and brief warning, no damage. Player sees the protected refuge.
2. **Rule Broken** — one authored event has a unique ID. World flickers purple; the party receives a clear visual goal and optional single-line system text, not a new controls tutorial.
3. **First Blow** — Rage commits to one marked strike, with visually safe counter-position. This is a *demonstration*, not a lethal hit before the player can move.
4. **Refuge** — players cross a known safe-room threshold; Rage violently reaches the door but **cannot damage inside**. Camera remains on the safe side and preserves player ownership.
5. **Resolve** — a brief checkpoint after this discovery. Retry need not replay the entire 60-second story lead-in.

**State machine:** Dormant -> Warned -> Enraged -> CommittedStrike -> Roaming -> RefugeBlocked -> ChaseReady. Strict actor-specific safe-room exclusion; test restart at every phase. Reused attack patterns must still telegraph after changing camera.

## 3. Stage S07 — THE BOMB CHICKEN RUN

**Provisional length ~2560–3072px**: three cinematic combat-escape pockets separated by protected short pauses, not 60 seconds of uninterrupted sprint. Primary ground route remains continuously navigable by Carl and by Donut. An alternate upper shelf may reward Donut's agility but cannot become a compulsory safe escape.

| Section | Visual silhouette | Player choice | System proof |
| --- | --- | --- | --- |
| R0 Bomb Chicken setup | rolling launcher/cart under enormous red hazard stencil | ignite/activate decoy, choose which lane to attract Rage into | true decoy cause, committed chase direction |
| R1 The Run | shutter corridor and a single asymmetric lifting/reverse-gravity hazard | cross during the committed Rage recovery, use existing movement or optional blink | bounded hazard and fair safe lane |
| R2 Stairwell | impossible downward void with protected approach, large overhead warning sign | lure Rage into authored stair pit, coordinate exit, watch deterministic expulsion | Rage crosses pit region; NO XP, no ordinary kill |

### Beat R0 — A choice made under pressure

Carl's pre-existing explosive-related skills and a compact single-use **authored launch cart** can create a loud decoy impact. The cart is a scene prop with an existing context-interact action, not a new weapon inventory. Triggering it is an *easier / more spectacular* line, not a unique success condition. A missed decoy leaves a bounded, more dangerous but fair direct retreat route along the lower passage.

The Rage must visibly **commit** to a direction before lunging, giving the player an actual positioning puzzle lasting seconds, not a frame-perfect timing test. It cannot recalculate target position every tick during the lunging phase. Intentional hit-stop and slight presentational shake must not alter authoritative physics or break replay.

### Beat R1 — A single memorable hazard, not a platforming gauntlet

A **one-way rising / reverse-gravity volume** is signposted by moving scraps and lit floor arrows, with one safe ground cover pocket and an alternate elevated run. Its active window is generous under baseline jump/coyote/input buffering. Heroes may bypass with already-unlocked movement or use environmental protection; **Puddle Jumper short blink** is an optional stylish save, never the required escape verb. The AI partner must be able to navigate using already accepted hazard-feed behavior; if not, do NOT add a global teleport/invincibility cheat: adjust authored cover or split the camera-safe pockets.

Rage's attacks are still threatening, but at most one large committed attack envelope matters at a time. No damaging invisible body aura spanning the entire viewport. Screen-edge pursuit can use huge background silhouette and shake while real collisions remain authored with grounded rects.

### Beat R2 — Victory through the dungeon's own rules

The stairwell trigger has **two separate volumes**: protected protagonist exit and the Rage-only expulsion crossing. Explosive launcher or bait changes Rage's committed pursuit lane, but does not subtract boss HP. A valid expulsion is when the Rage's authoritative position/charge enters the pit crossing volume after warning; it then goes to terminal Exiled. The outcome is not a normal Defeated mob and emits **no XP, no ordinary kill reward or generic finisher slow-mo**. A short authored silence and camera settle follow before the survivors approach the next stage.

An optional high-skill route may have the player manipulate a rolling cart to send Rage into the region earlier, but a **slower no-cart route must still be possible** using safe timing and clear geometry. This answers the danger of turning the scene into a rigid bomb-chicken-scripted puzzle.

### Rage threat states (candidate implementation)

Hidden -> Revealing -> Tracking -> Tell(direction, safe_lanes) -> CommittedLunge(direction, limited time) -> OvershootRecover -> PursuitReset -> Exiled. The lunge direction freezes on Tell completion. Safe rooms are excluded from attack volumes. Recovery and restart use full deterministic state reset. There is no ordinary boss HP, no damage/stagger acceptance from magic, no reusable Rage enemy in next stages.

Do not tune the chase by unbounded acceleration until the enemy outruns all characters. Cap committed speed relative to a **verifiable reaction budget** from the actual Floor 01 movement baseline. The *sense* of impossibly fast Rage should come from staging, parallax, animation and giant committed leaps, not undodgeable chasing velocity.

## 4. Story and reward continuity

- S05 convoy and the human stakes **do not introduce permanent injured NPC inventory**. Use one narrative flag for successful arrival and a small transition story beat.
- S06's violation/Rage appearance are one-off stage events; camera and music cues are presentation-only and repeatable or skippable.
- S07 no XP for Rage expulsion. If per-character viewers register a feat, label it as a **unique authored stage accomplishment**, no generic enemy kill and no farming on retry. Never feed a sponsor power progression gate from this.
- The actual Floor 3 stairs remain the sole progression direction; after Rage is dealt with, S08/S09 must remain on Floor 2 before the final descent.
- Solo player swapping and 2P remain active during the chase except for a sub-second skippable presentational pause with real input safely buffered; no controlled hero becomes an inert NPC.
- Source recap details (launcher, grease, Rage, stair rule) are inspirations. Room geometry, timing, two-route escape, industrial corridor, convoy mechanism and all speech are our **original game adaptations**.

## 5. Fairness and test matrix

| ID | Required evidence |
| --- | --- |
| MR-01 | convoy deterministic track, stop at exactly two anchors and no hidden obstruction |
| MR-02 | S05 direct-only clear with one human controlling each character in separate scripted runs |
| MR-03 | 2P camera divergence: both heroes and pending danger visible, safe regrouping |
| MR-04 | companion makes actual useful attacks at each convoy stop; measure rather than claim |
| MR-05 | convoy damage disrupts at most bounded once per stop; no accidental NPC death softlock |
| MR-06 | staged warning, Rage cannot cause surprise harm before safe-room reveal |
| MR-07 | Rage safe-room volume completely prevents direct and indirect damage while occupied |
| MR-08 | repeated attacks never reduce Rage HP or mint viewer combat-farming points |
| MR-09 | missed/broken decoy still reaches the stairwell by safe direct route |
| MR-10 | optional Puddle Jumper changes efficiency, but no blink needed to clear |
| MR-11 | both solo control authorities and both humans can safely traverse rising hazard |
| MR-12 | real Rage lunge commitment and stair-exile event, independent of presentation timing |
| MR-13 | exile not a normal kill, no XP, no ordinary slow-mo reward |
| MR-14 | fixed seed / restart / strict Playtrace hashes where supported; no duplicate broadcast |
| MR-15 | 640x360 and 640x400 one-game-speed passage, no clipped warnings or unreadable heroes |
| MR-16 | human game-feel, two real pads, real hardware frame time — **NOT RUN** until owner tests |

## 6. Art, camera and sound keyframes

- **KEY-MR-01**: wheelchair lantern train amid huge empty cold cinderblocks; small human warmth against institution scale.
- **KEY-MR-02**: the first overhead warning changes blue-green fluorescent to violet-black. Giant *partial* Rage shape visible through the maintenance wall; the human sprites are still clear and full native size.
- **KEY-MR-03**: white-red emergency corridor; launcher is foreground-readable and huge spectral mass is behind two architectural bands. Clear covered lower route and one optional raised shelf.
- **KEY-MR-04**: vertical yawning stairwell, protagonist silhouettes on a protected rim, Rage committed across the pit. Extreme scale from negative space and lighting, not from zooming playable heroes to a few pixels.
- **KEY-MR-05**: after escape — near silence, warm lantern and an empty crimson sign. A very short recovery, no instant follow-on fight.

Audio philosophy: treat the refuge and aftermath as *real silence relative to battle*, with one low-frequency impact motif and a distinctive mechanized thrum for committed lunge. Avoid constant rumble causing fatigue, copycat recognizable film music or generic shouting over combat cues.

## 7. Implementation order

1. Pure deterministic convoy track and finite stop/defense state, with zero new shared camera modules.
2. S05 actual stage room, two stop patterns, solo/AI partner stats and first 640x360 capture; mark hardware 2P pending.
3. Pure Rage state machine, sanctuary exclusion and stairwell exile, tested against false HP/XP.
4. S06 actual refuge and story checkpoint, with skip-on-retry.
5. S07 pursuit and cart, first a **no-cart direct escape**, then optional launcher route. Measure normal and failed decoy path.
6. Art keys and editable native-source execution; separate independent art review.
7. Real Playtrace replay with retry and event trace; owner review after travel.

**Do not build a general-purpose NPC escort engine or invulnerable giant boss engine for this one dramatic act.** It needs two narrow authored stage controllers over the existing party, environment and event contracts. Compare any unavoidable shared-core changes to the owner-approved Floor 01 baseline.
