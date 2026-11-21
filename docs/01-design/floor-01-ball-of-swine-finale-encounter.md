# FLOOR01-BSW-FINALE-01 — Ball of Swine: The Ring That Breaks

**Status:** original game encounter design / implementation contract for local agents. **No playable Ball of Swine boss is claimed on this branch**. The preferred arena is approved **as concept direction only**; the native boss/Tuskling art has independent review gates, while the boss's gameplay AI, authored room, moving gates, full-floor integration and end sequence are new engineering work.

**Narrative placement:** **after Juicer**, after the already specified quiet ability-regroup passage, as the **narrative climax and exit of Floor 01**. Do **not** silently replace the existing `f01_late_mastery` debug/wave-test room: the merged [F01 integration plan](../07-agents/f01-playtest-integration-plan.md) currently ends the playable testing sequence in a mastery arena, which is **not** the Ball of Swine. A new final encounter and its story-ledger/floor-exit seam require the integration owner's review. Its own stage may be available in isolation before the complete campaign is connected.

**Experience promise:** "I survived a rolling avalanche of screaming aristocratic pigs, used its momentum to smash the ancient arena, fought off the creatures it threw out, and finished it with our own attacks." The finale tests **observational movement, committed attacks, physical positioning, Carl's kick/dynamite/gauntlet, Donut's Magic Missile/agility/earned Puddle Jumper, cooperative threat division and a player-caused environmental advantage**. No new input, spell, gear, inventory, mandatory double-player puzzle or midfight skill reward.

## 0. Reconcile actual project authorities before writing gameplay

- [SWINE-01 original 18-frame set](../02-art/swine-01-spec.md) decides a **128px diameter**, 176×144 frame, pivot (88,143), roll drawn as **six native raster frames**; **never runtime-rotate or rescale its pixel grid**. The concept arena's earlier **100–110px diameter suggestion is superseded by this accepted art size** for baseline greybox geometry. Runtime collider hypotheses from the art doc still require collision/playtest validation. The visual includes 30+ compressed Tusklings; it does **not** authorize 30 simultaneous physics enemies.
- [TUSK-01 original 16-frame set](../02-art/tusk-01-spec.md) has **30px idle height**, 56×40 frame, pivot (22,39), tux/gown variants. Its **28-tick** visual tell comprises an 8-tick onset + 20-tick hold; 3 ticks of launch make its first contact approximately **31 ticks** after warning starts. Animations are source/reference and **do not prove enemy runtime behavior exists**.
- [Approved standalone arena layout](../02-art/floor-01-ball-of-swine-arena-concept.md): `assets/concept/floor-01/ball-of-swine-arena/04-gameplay-layout-preferred.png` is the owner's preferred **concept image**, not scene geometry. 1200px provisional width, broad protected floor, short side flanks, central suspended impact gate, limited side gates and selective supports. No new top-down ring-maze movement.
- [Arena gameplay and camera probes](floor-01-ball-of-swine-arena-gameplay.md) are hypotheses for 1×/1.25×/1.5× view coverage, not an approved runtime zoom amendment. Follow [ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md) and current camera controls. 1.5× coverage must **not** ship without measured legibility and explicit rendering/camera approval if it changes source sampling.
- Juicer already has an independently authored `boss_f01` fight; Gauntlet and Puddle Jumper are earned at its defeat. **Do not change Juicer or move those unlocks** to this scene. Likewise `f01_late_mastery` is still a testing harness/wave arena, **not this boss**.
- The story's Ball consists of **30+ Tusklings** visually/fictionally. Design a finite, bounded **spawn budget of actual independent Tusklings** and a separate cosmetic fragmentation layer; don't imply every painted face becomes a full AI entity.
- Source/canon accuracy is not implied for the exact moving gates, damage values, arena apparatus, dialogue or victory staging. These are explicit **gameplay adaptations**.

## 1. Floor 01 ending emotional arc

| Beat | What the player feels | Actual gameplay choice |
| --- | --- | --- |
| **E0 — The wrong silence** | Empty stone, enormous concentric carving, something moving behind the far gate | Safely look/read before combat; regroup without input lock |
| **E1 — The rolling mass** | "That thing fills the corridor." A 128px ball with hats/tusks/pig heads starts a committed approach | Read the roll tell; get out of **the lane** using real floor relief, structural lips or a lateral refuge—not an impossible 128px jump |
| **E2 — Make it hit** | Architecture becomes a weapon when Ball hits a true solid/gate | Bait a committed rolling charge into a gate/wall or ignore apparatus and fight directly during recovery |
| **E3 — They come apart** | 1–2 genuine Tusklings detach on qualified collision; now their frantic 30px bodies are individual threats | Carl kicks, Donut intercepts/finishes, or both keep pressure on the Ball. No compulsory cleanup gate |
| **E4 — The floor remembers** | One outer support visibly breaks; sightlines/rebound angle change and the ring appears fractured | Reposition around the changed geometry; use upper flanks or timed gate to force a stronger impact |
| **E5 — Earn the finish** | Last HP is removed by a real attack/collision, the Ball slumps and a **bounded** split of flesh/costume scraps occurs | Finish via sustained direct damage, optional environmental damage, or combined team strategy |
| **E6 — The exit** | Mechanical strain stops, one ridiculous leftover accessory tumbles; clear view of the next-floor passage | Short player-controlled regroup and checkpoint/exit activation; no new ability grant or unskippable cinematic |

**Signature image:** the committed mass strikes the central vertical gate, rebounds, loses two tuxedo/gown Tusklings and reels while one player uses a normal attack. Later, a deliberate strike against the weaker outer buttress changes the bounce route. These are **honest physics events**, not guaranteed cinematic kills. A player can also win by direct attacks and opportunistic recovery windows without engineering an environmental hit.

**Tone:** violent, grotesque, slightly absurd because the pigs are dressed like nobles, not a comedy cutscene. Lights, UV, sparking gates and detached pearls are secondary to readable silhouettes.

## 2. Room topology and safe geometry (first numbers are hypotheses)

Start with **1200 world px**, a **mostly continuous ~70% of width floor run-up**, and native 640×360 or 640×400 gameplay view. These are **authoring bounds**, not loaded coordinates. Integrate through the existing actual room/LDtk/story format, not a new loader. A proposed room-local coordinate layout:

```text
x = 0          160            400           600          800          1040        1200
    ENTRY      LEFT RELIEF     CHARGE LANE   GATE         CHARGE LANE  RIGHT RELIEF EXIT
    ┌──────┐   ┌─────┐      ┌─────────────┐  ║  ┌──────────────┐       ┌──────┐   ┌─────┐
    │SAFE  │═══│ alcove│══════│  FLOOR       │  ║  │  FLOOR       │══════│ alcove│═══│SAFE │
    └──────┘   └─┬───┘      └─────────────┘  ║  └──────────────┘       └──┬───┘   └─────┘
                 └── optional short raised flank ──┘  └── optional short raised flank ┘

    ~128px Ball: cannot be jumped cleanly with Carl's normal ~92px apex.
    Ground escape/safety = *real protected lateral relief*, not a fake 128px vertical hop.
```

| Area (hypothesis) | What is authored | Safety invariant |
| --- | --- | --- |
| **L0 entry x0–144** | Two-player waiting room, checkpoint, visible arch/window revealing the Ball's heavy movement but **no wheel-speed charge before both players can see** | Boss engagement starts only when all living heroes are in legal reachable arena positions; existing party/single camera contract, no offscreen spawn instant death |
| **L1 left x144–320** | One **recessed alcove** or low protected lip with true solid bounding; optional short 64–80px elevated escape flank behind a solid ball-stopping wall | A hero can escape the incoming ball **without needing to leap above 128px**, while the Ball's radius cannot pass through hero-sized gaps |
| **L2 central lane x320–880** | Long dry run-up, readable floor travel direction, **central suspended gate near x~600**, and one clearly marked sweep shadow | Fully traversable when the gate is up; when down, at least one **real reachable bypass** / later reopen route exists for either hero; no mandatory simultaneous switch |
| **L3 right x880–1056** | Another physically valid side relief and short elevated flank, optional one weak outer support on a **noncritical** structure | Do not seal ground, trap P2, delete protected floor or permit unlimited safe ranged turret play |
| **L4 exit x1056–1200** | Protected gate/elevator/stair well for after defeat with one visible lower-floor/next-stage architectural marker | Exit never opens due a VFX timer; it opens from real boss defeated flag once; remaining minions cannot spawn into protected checkpoint |

**Reality check on refuge collision:** a 128px sphere with ~60px gameplay collision radius overlapping the ground cannot be evaded reliably by Carl just jumping over it (current basic vertical apex ~92px). A 64px open shelf **without an intervening ball-stopping solid lip** is not a safe shelter; its floor is still lower than the Ball's top. An elevated flank counts as safe **only if an actual wall/lip stops the Ball before it can overlap the player**, or the route reaches a verified position outside its swept collision. Model all swept volumes and test real movement before painting the final geometry. Don't write inaccurate platform safety into the scene.

**No permanent camping:** refuges may remain safe from direct Ball impact but have limited firing LoS and brief tactical usefulness; detached Tusklings can reach their open approaches when the player stays too long. Never force an instant offscreen charge to punish hiding. Give the player a readable way back into the fight.

### Central moving gate: only one in MVP

- `Raised` → `Warning` → `Descending` → `Down` → `Ascending` → `Raised`, authored and validated, not a new generalized kinematic engine. Provisional warning **36–48 ticks**, movement **24–36 ticks**, with visible impact footprint/vertical lane. Actual numbers are knobs, not chosen balance.
- Use **existing Interact / environmental hit verbs** only if the authored prop API supports it; otherwise initial gate behavior may have a deterministic authored cycle as long as players still choose **how to lure the Ball**, and no guaranteed impact occurs without genuine contact. Neither hero must press a separate dedicated gate button, and no synchronised 2P switch.
- A gate **hits the Ball only on actual swept collision**, not because its timer expired. The gate never follows the Ball or snaps into an impact in the renderer. Contact with a live hero must have a predictable warning and a safe escape corridor; MVP can treat the gate as nonlethal to heroes / pause safely on player block rather than cause a crush-softlock.
- If the gate is down, do not leave both heroes separated permanently: provide a verified hero-sized bypass or timed reopen accessible from either side. The gate is a **tactical opportunity**, not the mandatory answer to win.
- Only one optional side support in v1: `Intact` → `Warning/Damaged` → `Broken`. Breakage removes a selected wall/partial barrier, not the main lane. Ball direction, damage/stagger, path/LoS recompute from actual new collision.

## 3. Ball boss: a rolling composite with honest committed momentum

### Minimal authoritative state model

`Dormant` → `Roam` → `RollTell` → `CommittedRoll` → `Impact/Rebound` → `Recovery` → `Roam`.

Optional `Stagger` state follows a genuine gate/support collision or a threshold of correctly recorded normal hits; `Fracture` is a **bounded ejection event** and can coexist with a recovery but is **not an invulnerable cinematic phase**. `Defeated` stops boss attacks and transitions to end-of-stage logic.

- **Tell:** use SWINE-01 36-tick readability `attack_tell` as the initial minimum. The contact hazard does not become lethal before the readable warning/real movement. Distinguish ordinary slow roll from committed fast roll with sound, squash of mass and a clearly locked direction arrow in debug only (not mandatory HUD arrow).
- **Motion:** candidate slow roaming **45–75px/s**, committed roll **220–280px/s**, about **320–480px max committed travel**, stopping at real collision, timeout cap or already authored wall; tune after playtest. These are not canon nor approved balance. **Heading locks at the tell/commit**; no target prediction after commitment, no invisible acceleration teleport, no sudden backwards reversal.
- **Collision:** start from SWINE-01 **nominal 128px diameter and placeholder radius 60px**; align actual floor collision/visual mass and swept circular contact, not raster pixels per roll frame. Each charge/impact has a token, one qualified damage instance per hero in the active window, existing i-frames; camera/FX never determine collision.
- **Roll animation:** use the approved **six hand-rasterized rotation phases** at native scale with fixed screen-space lighting, independent of authoritative angular velocity/position. **Never rotate or scale** the source sprite runtime to match apparent speed. Dust, noise and floor markings sell momentum.
- **Impact resolution:** deterministic stable order: world/gate solid contact → stop/rebound vector → damage/stagger event → (eligible) bounded Tuskling detach → terrain state change → presentation events. Prevent double wall contacts in a single tick, chain explosions and re-triggered detaches.
- **Recovery:** a clear, player-reachable **punish window** after collision (candidate **48–72 ticks**, data-driven). Carl can approach with fist/gauntlet/kick, Donut can fire Magic Missile or use Puddle Jumper to change position if the **true destination is clear**.
- **Damage:** direct strikes and earned spells against an actual Ball hurtbox always register legitimate HP damage if they hit. During committed high-speed roll, **damage may not interrupt momentum**, but do not add blanket invulnerability. No magic immunity or hidden 3-step shield.
- **Starting boss health hypothesis:** **20–24 HP**, with gate-assisted impact initially **3–4HP** if a genuine Ball collision occurs. Final values require time-to-clear data and normal hit frequency. These are **not** authorization to change the current Juicer's 36HP or the shared damage system. Direct-only clear should be challenging but **not a grind**.
- **No mandatory three health phases:** visual damage, loose costume scraps and occasional detaches track *real* impacts/thresholds, but the boss can die in any authorized way at any time; no HP threshold locks, always-required gate strike, shield-turn puzzle or invulnerability until all Tusklings detach.

### Fair engagement, two-player and camera

- All charge tells and the imminent trajectory must be visible to the **targeted** player, including the rear player. A camera showing one player but hiding a lethal 128px boss behind them **fails**.
- Do not require the Ball to jump over the player; it is a giant front-to-front threat. Its body does not automatically force players across itself. The correct answer is a grounded safe pocket, wall-protected elevated flank or another demonstrated physically valid route.
- A party can split across the central gate momentarily; the final fight must still have a two-way safe regroup, no 2P-only timing and no AI companion solve requirement. Do not teleport stragglers except when a preexisting, accepted recovery rule explicitly applies.
- For a solo human controlling either hero, the companion may assist ordinary attacks using independently implemented AI; the Ball cannot require that AI to operate gates or win a precision timing puzzle. Don't rewrite companion AI here.

## 4. Tusklings: separate enemy actors without an unbounded swarm

### What the ball visually contains vs what the simulator spawns

**Fiction/art:** 30+ compressed aristocratic Tusklings in the composite.  
**MVP independent agents:** max **3 simultaneously active**, **1–2 detached on a qualifying major impact**, with an authored **cumulative ejection budget of 4–6 during a normal fight** (final numbers playtested). The visual rest of the ball stays one authored composite sprite and a small **noninteractive debris** layer; **do not** create 30 tracked rigid bodies or claim a fixed spawn budget proves canonical head count.

- Standard independent Tuskling: **30px native** tux or gown variant, normal enemy hurtbox, movement, damage/death system and collision source attribution.
- One simple attack: its existing `tell→gore→recover` read (about **31 ticks to first active hitbox** from tell onset); actual small forward gouge must be simulated/collided from its committed physical position. No costume-specific attack variants, free projectiles, poison, invulnerable mini-bosses, hidden Dominion character or 6 new class kits.
- Spawn separated **from the actual impact point** with bounded no-overlap initial positions chosen from a deterministic safe list; **never** spawn directly overlapping a hero, inside gate, within protected exit or offscreen as an immediate active attack.
- Detachment trigger (first proposal): one or two on a **true hard wall/central-gate impact** with enough committed speed, or a reviewed boss cohesion breakpoint. Cap by active budget and remaining pool; **never every frame of collision**. If maximum active is reached, defer the ejection to a later qualifying hit or produce visual bits **only** (without false enemy hitboxes).
- A Tuskling that dies from normal attack remains dead. No automatic reassembly, no unlimited summoning/respawn, no orphan actor ownership after Ball death and no enemy death falsely attributed to the player because an FX particle touched it.
- On Ball defeat, remaining detached enemies **remain ordinary independently simulated creatures**, but the protected exit can open once the Ball itself dies; don't force a prolonged minion cleanup as a surprise extra boss phase. A true exit-safe corridor and spawn leash prevents ambush. Whether they flee rather than continue attacking is a **separate behavior decision** and must not be scripted as a free player kill.

### Carl's kick as an optional discovery

Carl may kick a **live detached Tuskling** into the main boss to cause a **bounded short stagger** only if the actual merged physics supports launched enemy→boss impact contact with a valid owner/source chain and no new blanket enemy-to-enemy damage. **Default CF02 enemy-to-enemy collision HP is zero**; do **not** silently turn every body collision into lethal damage or grant extra Ball HP loss without approval. First pass can make the collision a visible **stagger-only** interaction or defer the interaction until the supported contact interface exists. Either way, direct attacks and environment damage must complete the encounter.

## 5. How Carl and Donut each contribute

| Player style | Legitimate strategy | What must not be assumed |
| --- | --- | --- |
| **Carl aggressive** | Bait a committed roll from a valid safe route, step into recovery, punch/kick/gauntlet the mass, kick detached Tusklings out of his lane | No grab action, no jump over the whole Ball, no pre-Juicer equipment gate |
| **Carl tactical** | Time existing dynamite to damage the Ball or a genuinely breakable support; punish the real impact | Bombs aren't required, auto-homing or an unavoidable global gate-clear script |
| **Donut mobile** | Move along reachable flanks, shoot horizontally during genuine line-of-sight windows, use earned jump and **short blink** where its *actual* landing space is valid | Puddle Jumper is **not** an unconditional phase-through-128px-ball teleport; horizontal bolt cannot track downward or through a gate |
| **Two human players** | Split short-term crowd pressure: one manages one or two Tusklings while the other baits/staggers Ball, regroup for punish | No mandatory simultaneous button, camera split, AI precision intervention or fixed 2P choreography |
| **Solo with companion** | Human swaps/controls either hero, companion participates opportunistically, stage exits via same rules | No assumption companion handles moving gate, chooses a precise projectile or can survive lethal offscreen roll |

**Anti-degeneracy:** unlike a stationary 3-HP llama, the Ball can leave the limited bolt firing lane and approach under a real tell, while its minimum 128px radius forces relocation. But a safe nook with uninterrupted missile LoS can accidentally create a Donut turret; the agent must search for that and alter cover/approach, **not** nerf Magic Missile or make the Ball immune to spells.

## 6. Camera, art and HUD: spectacle without losing the fighters

**Camera prototype matrix:** **1× native 640×360 / 640×400** is baseline; **1.25× world coverage (~800×450)** is experimental and requires true Bevy screenshots/replay; **1.5× (~960×540)** is a *research-only* extreme and cannot be assumed production-safe. At 1.25×, Carl 52px appears ~42px, Donut 40px ~32px, Tusklings 30px ~24px. At 1.5× Donut becomes ~27px and Tusklings ~20px: potential unacceptable clarity on handheld. Test on a 1080p view and 1280×800 output, including motion, AA/shimmer, HUD and 2P separation.

- A moving 128px boss and a 1200px arena **cannot always be fully shown together** on 640px camera; choose **fair event activation zones, camera lead and unhidden upcoming warnings** before changing core pixel-sampling rules.
- A true charge may not begin where the threatened hero cannot see the Ball/tell; do not substitute a warning icon for unseen lethal physical contact.
- Only mild hysteretic zoom changes if technically compatible with render policy; no camera pumping on every rebound. Camera state is pure presentation and **never feeds the physics event order**.
- Preserve approved art: pink/flesh/ivory **128px** Ball, Tusklings in tux/gown **30px**, ringed masonry, brass/iron gates, restrained special UV/laser/show lights. Show lighting may peak late but never conceal travel cues or define collision.
- Boss HUD may reuse the existing boss bar view with display name and HP; **no second complex cohesion/resource meter** unless an approved UI schema exists. Any current active character viewers/score UI follows its narrative unlock timing—do not introduce a new score counter just for this fight.
- **Rare dramatic time dilation:** only for exceptional player skill according to the existing CINE-01 kill policy and local 2P fairness. Normal final boss defeat gets a weighty impact/hitstop and short presentation, not automatic several seconds of simulation slow-motion.
- No AI-generated concept PNG cut-and-paste as final sprites; use original editable .ase/pixel grids, atlas manifests and independent review. Ball's roll stays native raster frames, not runtime rotation or scaling.

## 7. Ending Floor 01: specific, short, player-controlled

1. `BallDefeated` (or an existing stable boss defeat event) fires **exactly once** from actual HP/collision outcomes, recording final damage source, owner hero, spawned/ejected state and current episode/room hash.
2. Physics attacks cease for the boss; the heavy shape **slumps or unravels** using its approved `down` frames; small cloth/pearl/stone particles settle as **non-collidable presentation**. If final Tuskling ejection is supported, cap the **live actors** separately; no forced attack during an unskippable victory scene.
3. Short audible contrast: the gate chain whines, then quiet. A single aristocratic detail falls into the dust (original VFX gag candidate). Do **not** kill player controls, force all players to stand at a point, add a new unlock or require minion mop-up.
4. An **existing-style exit** becomes interactable/traversable at the right protected zone. Every living hero can regroup and exit under current party/camera semantics; no mandatory two-human button press, boss not secretly rearmed by approaching the exit.
5. Persist one `f01_ball_defeated` (candidate name) **narrative completion pin** through checkpoint/scene transition. This is **distinct from `f01_late_mastery` debug state**, and needs explicit story-ledger/integration review before implementation.
6. End-of-floor presentation may reuse an existing results/stage-card mechanism if one has been approved. If viewers/rewards/score are not yet available at this narrative point, **record semantic playtrace events** and leave their UI hooks dormant rather than inventing a score economy.

**Target playtime first human encounter:** **3–5 minutes** boss fight after short reveal, practiced faster. This is hypothesis, not a speedrun lock; prioritize low repetitive downtime and short retry. If direct-only fight routinely takes >5 min without genuinely new decisions, reduce dead time / tune boss HP/recovery (not simply add more particle effects).

## 8. Automated fixture matrix and design-level proof obligations

All modes use authoritative fixed 60Hz scene/party inputs; a visual-only simulated spinner and a tiny scripted fake are not test evidence.

| ID | Fixture / player scenario | Required evidence |
| --- | --- | --- |
| **BSW-01** | Real 128px Ball native art in stage entry | Collider and rendered hull fit; six authored roll frames and 1× legibility |
| **BSW-02** | Boss warns at edge of 640×360/400 visibility | 36-tick readable direction before acceleration, no offscreen instant hits |
| **BSW-03** | Roll into solid wall vs clean lane | True swept circle/solid intersection; no tunneling or frame-count-dependent rebounds |
| **BSW-04** | Carl escape without jump-over | At least one *genuine* safe physical route/refuge when Ball crosses, across actual initial positions |
| **BSW-05** | Two active humans spread 0/96/224/448px | Safe threat visibility and camera coverage; both can regroup across gate, no unavoidable sealed path |
| **BSW-06** | Gate raised / warning / down / reopening | Correct real collision/LoS, safe ground bypass, no heroic crush-softlock |
| **BSW-07** | Ball hits gate and *near misses* it | Exactly one genuine impact/stagger/damage when overlapping, no scripted kill on timer |
| **BSW-08** | Break outer support early/late/not at all | Persistent bounded geometry changes, exit still reachable in every authored state |
| **BSW-09** | Ball direct-only win / environmental-assisted win | Both paths finish with valid events and measured time-to-clear; no required 3rd-party puzzle |
| **BSW-10** | Bounded detachment on qualified impact | 1–2 new actor IDs, max 3 live, finite 4–6 budget, legal spawn, costume variants distinct at 1× |
| **BSW-11** | Tuskling real gore/defeat | Actual 31-tick precontact window, ordinary hurt/damage, no unbounded chain spawning |
| **BSW-12** | Optional Carl kick detached Tuskling into Ball | No default enemy-to-enemy HP bonus; stagger only when source-linked real contact if supported, else explicitly blocked |
| **BSW-13** | Donut Magic Missile + Puddle Jumper from different heights | Horizontal bolt actually intersects target; blink destination safe and never crosses invalid 128px collision |
| **BSW-14** | Stationary Donut exploit search | Natural static positions cannot reliably clear Ball with low risk while NPC companion carries fight; attribute damage correctly |
| **BSW-15** | Solo human Carl, human Donut, local P1+P2 with actual pads | Each contributes, no AI precision/2P-gate requirement, camera legibility at true native size |
| **BSW-16** | Restart during charge, gate fall, detach, outer break and death | Complete deterministic reset, no duplicate actor/spawn/damage/grant; same input/events/state hashes twice |
| **BSW-17** | Boss defeat + remaining live Tusklings | Exit still accessible/safe, no forced tedious cleanup, only one floor-clear pin |
| **BSW-18** | 1× vs experimental 1.25×/1.5× + effects off | Same sim hashes, readable actors/tells, no shimmering/unclear hitboxes on 1080p and handheld |
| **BSW-19** | Recorded normal, environmental, aggressive and co-op runs | Real at-speed Bevy footage, PLAYTRACE, measurable earned spectacle and human fun/fairness notes |
| **BSW-20** | Juicer→regroup→Ball story continuity | Only already-unlocked story skills present, no hidden debug `all` loadout and no overwriting the mastery test room |

### Required headless strategy policies

- `carl_direct`: honest closing/striking after a roll misses, **no required gate**.
- `carl_environment`: try to bait committed roll into real gate, then punish, use dynamite if already unlocked.
- `donut_mobile`: dynamic safe flanks + legal horizontal missiles, jump/blink only where current collision permits.
- `donut_stationary_cast`: **legal press-edge** missile spam at every likely safe position; search the best no-risk full-clear exploit, not average over impossible spots.
- `duo_coop`: distinct two-player inputs handling Tusklings and boss without simultaneously-operating mandatory switches.
- `no_gate`, `gate_early`, `gate_timed` and `gate_missed` runs must all terminate legally (victory or honest failure with quick retry; not softlock).

**Metrics:** time-to-first-contact, charge count, misses vs true impacts, boss HP damage direct/environment/character, number of detach triggers and living/max Tusklings, kick→Ball impact source, player deaths/fair avoidable contact, damage received, movement distance, time spent in static safe pockets, camera unseen-threat incidents, lost frame budget, exit reachability, same-input hashes and 1P/2P coverage. No fabricated metrics for unsupported harnesses.

**Hard fail:** any legally positioned human is forced to be hit by an **unseen** 128px charge, any required escape depends on jumping over 128px with base Carl, any gate closes both routes, any Tusklings are spawned overlapping the party, any winning path requires a gate puzzle, a third-party debug power bypass, or an unapproved camera/asset rule.

## 9. Implementation delivery plan (separate future implementation PRs)

1. **BSW-A: Audited greybox and native source check.** 1200px staged floor; both protected reliefs; Ball 128px using provisional data collider + read-only art; true solo/2P route. Must show no impossible jump assumptions.
2. **BSW-B: Boss direct loop.** 36-tick tell, committed roll, swept wall rebound, recover, direct HP win, no gate or Tusklings. **This is the first actual playable milestone**; record true direct-only fun before adding spectacle.
3. **BSW-C: Gate and optional outer support.** Reuse approved BOIL authored prop/contact interface once merged; otherwise separate data gate and block this phase. Only one primary central gate in MVP. Check all collapse states, no softlock.
4. **BSW-D: One Tuskling actor profile and detachment budget.** Reuse actual ENEMY-SLOT adapter; 30px tux/gown native, real gore hitboxes, max 3 active. Initially one guaranteed *qualifying* collision fixture then make the gameplay trigger from real contact. Optional kick-stagger interaction behind verified physical collision gate.
5. **BSW-E: Presentation / adaptive camera probes.** Native roll frames, audiovisual impact, limited UV show lighting; 1× baseline and measured medium zoom. Preserve render ADR and HUD behavior. Keep camera separate from sim.
6. **BSW-F: Story/exit seam.** Coordinate with #182/#190 owner to add `f01_ball_final` (candidate) and post-Juicer progression without replacing the distinct `f01_late_mastery` harness. Correct checkpoint, boss defeat pin, protected results/exit.
7. **BSW-G: Replay + autoplay + human review.** Implement/run BSW-01..20, multi-policy headless sweeps, true pads, deterministic recording/replay and at-speed 1080p/handheld inspection; iterate geometry and enemy action without silently rewriting input, generic health economy, camera ADR or companion AI.

**Parallel-friendly boundaries:** BSW-A/B and the first TUSK-01 domain profile may be separate workstreams if they do not edit the same hot shared files. Gate wiring follows BOIL/contact merge; native art FX/camera tests can run against the BSW-B direct fixture. Story integration is **last**, when scene owner and checkpoints are stable. The required implementation agent checks `AGENTS.md`, current merged playtest integration plan and active PR queue before editing files. No feature is "implemented" by merging this document.

## 10. Explicit design questions for owner after first real playtest

- Does a 1200px arena yield too much inactive chase/run time? Compare 960 / 1200 / 1344px world width with same 128px Ball and real native camera.
- Are the physical side niches intuitive, or do players incorrectly expect to jump over a giant ball? If unclear, fix silhouettes/placement, not jump physics.
- Should two detached Tusklings per impact stay as a distinctive surprise, or would one per major impact produce much better readability?
- Is the current 20–24HP boss target too repetitive or too brief? Compare direct-only and gate-assisted time and *number of meaningful decisions*, not raw HP average.
- Is an optional kicked Tuskling→Ball stagger intuitive and satisfying, or should it stay purely cosmetic until collision rules mature?
- Does a 1.25× camera truly help co-op without making 30px Tusklings disappear? If not, keep native 1× and tune activation/warnings.
- Does the final victory leave a stronger impression through quiet contrast than a long mandatory fireworks cutscene?

**Scope boundary:** do not attempt a new independent physics/render engine, true 30-entity swarm, permanent crowd-score HUD, bonus boss phase, new spells or core camera policy, Juicer retune, unlimited destructible cathedral, auto-spawned Tusklings on every collision or a full Floor 02 level. End the first floor on a strong player-authored victory and a safe, legible exit.
