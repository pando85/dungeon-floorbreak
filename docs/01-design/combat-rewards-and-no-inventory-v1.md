# REWARDS-01 — Combat-first progression, spectacle rewards and no inventory RPG

**Product direction: owner-decided, 2026-11-20. Implementation: NOT STARTED.** The absence of a conventional inventory/gear economy is the product decision; the exact reward values, cadence, catalogue and controller flow below are **reversible design defaults pending real-pad playtesting**. This document is not a claim of shipped gameplay, canonical franchise items, or release rights.

Read alongside [the roadmap](../04-production/rewards-roadmap.md), [implementation work order](../07-agents/rewards-01-workorder.md), [ADR-0010](../05-decisions/ADR-0010-combat-first-progression.md), the [MVP controller contract](mvp-controller-contract.md), [character progression horizon](character-progression-horizon.md), [combat-first encounter rules](combat-first-encounter-design.md), [AUDIENCE-01](../02-art/individual-viewers-hud-v1.md), [CINE-01](../../src/cine.rs) and [AUTOPLAY-01](../03-tech/autoplay-01-automated-gameplay-evaluation.md).

## 1. Product decision, priorities, boundaries

**THE CRAWL is an authored, combat-first action campaign, not a loot-management RPG or a full run-reset roguelite.** Think Hades-like in its brevity of between-fight upgrade choices, but simpler in equipment and more dependent on authored story capability unlocks. The primary rewards are the visceral actions themselves, terrain interactions and cooperation. Rewards amplify those strengths; they cannot rescue an unfun ordinary hit.

**Do not build:** bag/grid inventory; random weapon/armor replacement treadmill; common/rare/epic stat gear; a vendor, dismantling, crafting, encumbrance, item durability, loot currency, general equipment comparison screen, consumable hotbar, permanent stat farm, duplicate weapons with +x% damage, or compulsory grind. Existing 'Pause / menu / inventory' wording describes the menu button, **not authorization for an inventory screen**. One story-earned equipped special may remain active per hero as the approved controller grammar specifies; a possible R2 cycle is a separate later UX review, not a loadout menu commitment.

**Keep:** (a) permanent, authored narrative capability upgrades; (b) immediate short combat momentum; (c) a *small* selection of floor-limited modifiers at safe stage boundaries; (d) rare narrative sponsor grants after the broadcast unlock. A story-significant fictional object may be visible on a character and improve an existing move automatically; it does **not** generate a collectible inventory slot. All cross-floor permanence requires an explicit authored unlock or curated exceptional grant, never a random generic drop.

The default early Floor 01 demo remains **no reward popups, no viewers, no sponsor crates**, unless a clearly labelled test fixture / opt-in probe is used. Donut has no magic pre-biscuit; Carl has no explosives before earning them. Preserve small-HP enemy feel, selective destruction, four face-button verbs, Carl's kick (never grab), existing story beats, single-camera 1P/2P and native 640x360 / 640x400 HUD.

## 2. Four distinct persistence layers

| Layer / domain name | Acquisition | Active lifetime | Default constraint | Visibility |
| --- | --- | --- | --- | --- |
| `CombatMomentum` | Actual skilful event sequence | 6–10 s or limited successful contacts | At most one per **hero**; candidate once per encounter (provisional M1 scope in section 9) and at most once per 30 s anti-spam | Tiny transient notice/icon, not a HUD bar |
| `FloorModifier` | One offered choice after a qualifying stage | Until floor exit or explicitly consumed | At most two concurrent **team-owned** modifiers; one choice per qualifying stage | Safe stage-end card and small status chip when relevant |
| `SponsorGrant` | Broadcast-live eligibility AND authored sponsorship opportunity | Immediate use, floor-limited or rare authored persistent | No generic random permanent stat loot; capped opportunities | System/CRT message at safe moment |
| `CampaignCapability` | Canon-verified/author-approved story progression | Permanent in campaign, survives floor transition | Small fixed list per hero, expands existing verbs; no skill tree | Character move description / story beat |

The first two are the **playable proof-of-fun priority**. Sponsors are **future-gated**, not a blocker for combat momentum. A global stage reset never removes permanent story capabilities; a floor reset removes floor modifiers and unused momentum according to the snapshot policy below.

## 3. Event semantics — one source, multiple consumers

The authoritative Rust 60 Hz simulation already has combat/death cause and instigator signals. Add a **pure, Bevy-free** `SpectacleEvaluator` that takes immutable, ordered, authoritative combat facts, not particles, rendered gore intensity, viewer numbers, cinematic slow motion or input device ownership. It emits deterministic `SpectacleFeat` and `RewardEligibility` records; the separate reward state decides grants. AUDIENCE-01 remains a HUD **observer** and CINE-01 a cosmetic event recipient; do not read either system's presentation state to influence combat.

Proposed minimal facts (reuse existing events/IDs; do **not** create a parallel defeat emitter):
- `event_id`, `tick`, `encounter_id`, `floor_id`, `stage_id`, `target_id`, `cause`, `force_tier` and deterministic `instigator_hero_id` (a `party::HeroId`: Carl, Donut, or None). `encounter_id` in M1 is the provisional scope of [section 9](#9-checkpoint-failure-and-fairness-contract); `floor_id` and `stage_id` stay unset until STAGE-BOUNDARY-01.
- **No `event_id` exists in `CombatEvent` today.** REW-01 derives it as (attempt, attempt-local tick, per-tick ordinal): ticks are attempt-local (they restart at 1 after each `CombatEvent::Restart`) and several events can share a tick.
- **Attribution is normalised by REW-01 into `party::HeroId`.** `CombatEvent::EnemyDefeated.instigator` is an `Option<u32>` documented as a "player" but in practice the hero index whichever pad drives that hero, and `CombatEvent::Hit` names no attacker, so a hit is attributed through `MoveId::hero()` (also a `u32`). Neither is a controller slot. A swap / 2P pad-permutation fixture must prove the hero stays stable.
- Optional credited assistant hero id only with a real preceding causal setup/contact: no proximity-only credit. Human control vs companion ownership is *not* a new hero identity.
- `feat_kind` categories: meaningful chained impact, body-to-body interaction, wall/environmental contact, terrain-caused defeat, mixed-method sequence, hero-to-hero setup/finisher, qualifying precision/risk. Ensure a direct routine kill alone cannot repeatedly trigger rewards.
- A single target defeat must be credited at most once for reward purposes, regardless of overlapping explosion/terrain/CINE presentations.

**Important decoupling:** audience viewership also increases from embarrassment or authored anticipation; it is not a mastery score. A ridiculous failure may get viewers but never grants a combat-mastery bonus by itself. Body-part count, blood/gore preset, presentation-only time scale, screen shake and death visuals have zero authority over style or rewards. A companion kill retains factual hero attribution, but autonomous actions alone must not farm endless rewards, cause CINE slow-motion or override user-controlled hero choices. AI eligibility and contribution are tested separately.

## 4. Proposed quantitative style rules — all tunable, not accepted balance

Implement integer/fixed-point scores and explicit event dedupe in the fixed-tick domain. The numbers below seed experiments, not production thresholds.

| Distinct qualifying fact | Candidate style points | Notes |
| --- | ---: | --- |
| Plain enemy defeat | 20 | Progress feedback only; cannot solo-trigger momentum |
| Two distinct offensive actions causally chained | +70 | Once per unique two-action sequence |
| Enemy-to-enemy impact with real gameplay consequence | +100 | Never reward repeated harmless overlap |
| Wall / prop impact with knockdown or damage | +100 | Outcome-driven, not a cosmetic wall touch |
| Environmental kill with attributable setup | +170 | One reward per authoritative defeat |
| Meaningful Carl + Donut setup/finisher | +140 | Both receive factual contribution; no 2x team payout |
| Additional fresh category / risky execution | +40 to +80 | Must be grounded in observable simulator facts |

A chain window ends after **180 fixed ticks (3 s)** without a meaningful action or when the encounter scope ends (provisional M1 scope: section 9). Repeating the same event category within a chain yields 100%, 50%, then 20% of its candidate points; minimum thereafter 20%, with integer truncation; repeat damping affects style points only, never damage or viewer rules. Chain contribution per target is capped and each stage/encounter has a bounded event budget. Proposed **350 points plus at least two distinct meaningful categories** to unlock momentum; no single kill, pre-scripted spectacle, respawn or AI-only event may satisfy it. A plain defeat (20 points) never counts as one of the two meaningful categories; it only adds progress points. Maintain a per-hero progress accumulator independent of viewer totals, with event-id dedupe and one momentum grant per hero per encounter as the initial baseline. If testing finds that waiting for 350 is too restrictive, tune data rather than hard-coding a second evaluator.

**Do not design for score farming:** no replayed fixed spawns, revived entity IDs, recurring hazard ticks, immortal targets, repeated friendly hits, self-damage, deliberately failed attacks, or checkpoint reload duplication. No negative player punishment for playing simply: basic attacks retain their full damage and feel. Do not show a new full-time style meter; capture scores in replay/diagnostics and use brief notices only when qualifying.

## 5. Concrete momentum v0 — two prototypes, not a new equipment system

| ID | Eligibility / unlock guard | Proposal | End / non-stack rule |
| --- | --- | --- | --- |
| `RM-CARL-01 / IMPACT DRIVE` | Carl available; no tool unlock needed; qualifying manual or factual mixed chain | For 8 s, the **next two** landed Carl kicks increase knockback impulse by **12%**, clamped by a **launch-impulse maximum that REW-02 introduces as a data knob with tests** (no such maximum exists in `src/` today; it may bind to a CF02 knob if the owner prefers) and by terrain/material rules; **no direct damage increase**, no extra spawned bodies and no new attack input. Indirect outcomes under CF02-B (a non-slam becoming a wall slam, or a body reaching a second body) are expected, are the point of a push decision, and are counted in REW-09A (extra slams and extra contacts per buffed kick) | Ends on 8 s, two landed kick contacts, hero death or encounter boundary |
| `RM-DONUT-01 / CAT'S OPENING` | Donut available (including pre-biscuit); qualifying mixed or self-owned chain | For 8 s, the **next** landed feline pounce adds at most **6 ticks** to an eligible non-lethal hold (the CF02 Q38 pounce hold), clamped by a **maximum total hold length that REW-02 introduces as a data knob with tests**, optionally bound to the Q38 hold length (no enemy control-immunity cap exists in `src/` today); **no magic**, no damage increase | Ends on 8 s, one valid pounce, hero death or encounter boundary |

Eligible Carl kicks are a **data list**, not prose: default `PushKick` only; the CF02 `FollowKick` and `AirKick` join the list only once those moves exist and only under `cf02_rules`. The Donut hold exists only under `cf02_rules` (CF02-A, Q38); legacy `--demo` has no hold, so `RM-DONUT-01` is not offered there. **The M1 probe therefore runs under `cf02_rules` (`--cf02`)** with CF02-A (hold) and CF02-B (`CombatEvent::WallSlam` / `BodyContact`) merged; legacy `GoblinContact` is explicitly "no damage" and does not qualify as a meaningful category. Without them 350 points with two meaningful categories is barely reachable (legacy arithmetic: jab to haymaker 70, damped repeat 35, shore-block kill 170, three plain defeats 60 = 335). REW-01 itself can start immediately on synthetic facts.

Both examples are **behind an opt-in flag** until gamepad-feel and enemy fairness have been tested. The reward has no effect if the recipient lacks the required move; a feature-gated catalogue validator must reject such impossible offers. For solo, switching control does not freeze timers and the AI cannot claim another hero's buff. For 2P, each hero has their own momentum eligibility; total floor modifiers remain team-capped. Boss/elite immunity and protection of critical traversal materials always win.

## 6. Stage choices — tiny Hades-like decision space

Only **authored qualifying stage clear** events open a choice, at a safe checkpoint / inter-stage screen with combat paused; never during an active enemy telegraph. After proof-of-fun, target **three mutually exclusive offers, one shared team selection**; skip offers if no eligible distinct choices exist. At most **two floor modifiers active for the entire team**, with replacement choice once full. A floor exit resets the floor modifiers. A stage is not an infinite loop: repeated checkpoint clears cannot mint more offers.

Candidate floor modifier catalogue (meaningful sidegrades; exact values pending experiment):
- `RF-01 WALL DANCER`: first Carl-caused enemy-to-wall crash per encounter produces a modest additional *non-lethal* stagger, bounded by a stagger cap that REW-04 introduces as a data knob with tests (no enemy resilience stat exists in `src/` today). Guard: Carl wall impacts implemented (CF02-B `WallSlam`).
- `RF-02 AIR CONTROL`: first valid aerial recovery per encounter improves a following already-unlocked air attack's recovery by 2 ticks. Guard: CF02 aerial moves stable.
- `RF-03 FELINE FOLLOW-UP`: once per encounter, Donut's pounce extends a real setup window by 6 ticks, only if the target is susceptible. Guard: pounce/hold integrated; remains valid pre-biscuit.
- `RF-04 MANA CONSERVATION`: first earned Magic Missile finisher in an encounter restores 1 mana, limited by existing max; *never offered pre-biscuit*. Guard: real story unlock, Donut mana implemented.
- `RF-05 LAST SECOND PATCH`: once per stage, restore 1 HP after surviving a genuine damaging hit while below a threshold; cannot prevent death by retroactive healing. It triggers only on a **human-controlled** hero: the controlled hero in solo, either human hero in 2P. In solo the uncontrolled hero is already kept at 1 HP or more by `PartyTuning::solo_last_hp_guard`, so solo must not get a second safety net. Guard: actual hurt/HP pipeline.
- `RF-06 TEAM RHYTHM`: first verified *two-hero* causal combo per encounter improves subsequent team positioning/recovery, not raw double damage. Guard: companion factual attribution or tested two-human combo event. Precise effect only after an interaction playtest; keep disabled until then.

Propose deterministic candidate ordering with seeded shuffling **inside the authoritative domain**, family/eligibility filters and no duplicate offers. The first test can pin a fixed three-card deck to remove randomness. No gold, rarity, item inventory or passive +X% global damage. Controller navigation reuses menu controls; never add required gameplay inputs. In 2P, test **alternating human chooser by qualifying stage** (single shared result, AI never votes) against P1 chooser, and record which preserves both players' agency; do not silently ship a permanent binding or a dual-confirmation deadlock.

## 7. Sponsors and audience — future-gated, no feedback loop through HUD

Existing AUDIENCE-01: per-character viewers, stage-end peak candidate, observer-only HUD with `broadcast_live` **locked through Floor 01 opening** and no approved real unlock beat. Keep that contract unchanged in v0. The REWARDS MVP can ship without any sponsor feature.

When the narrative later unlocks broadcast and sponsors, sponsorship eligibility needs **three independent gates**:
1. `broadcast_live` + a story-authorized sponsor opportunity/cooldown (not assumed unlocked by starting Floor 01).
2. A **deterministic, domain-owned notoriety/spectacle milestone**, potentially corresponding to a large viewer increase; use documented event provenance and uniqueness, not visible HUD numbers.
3. An authored reward-table entry allowed for the character/story phase, resolved at the next safe moment.

The viewer observer remains unable to affect gameplay. If a future design strictly requires the **numeric audience threshold**, first extract a shared pure audience scoring rule / domain-owned canonical audience projection with explicit AUDIENCE-01 parity, migration, replay hashes and owner review; only then can sponsorship depend on that authoritative projection. **Never inspect `AudienceView` / `HudRuntime` / rendered count or fire sponsorship from `CineAdapter`.** An authored anticipation spike or comedic accident can attract viewers but gives **no automatic combat reward**; sponsor eligibility may independently include it when curated and authorized. Sponsors are opportunities, not one crate per spike.

Sponsor resolution categories: `immediate_supply` (healing or resource restoration at a safe moment), `floor_boon` (follows floor-modifier cap), and exceptional `story_capability` (fixed authored permanent change with canon/time validation and explicit save semantics). Boxes may have darkly comic flavor and bounded drawbacks, but must never softlock the route, reverse a critical gate or circumvent a power unlock. Sponsor content is an adaptation proposal, **not a verified book-canon reward list**.

## 8. Narrative equipment becomes capabilities

Objects that define a character (e.g. earned gauntlet enhancing Carl's fist, later learned spell) may change the sprite and existing action attributes when narratively awarded. They are **auto-integrated**, not bag slots. `CampaignCapability` is an ID-neutral unlocked ability flag / data record with authored provenance; no generic permanent random roll or stat currency. Exactly one active equipped special per hero stays compatible with the controller contract; a later R2 cycling decision requires a separate usability proof. Retain project-wide IP-neutral runtime names (`demolitionist`, `caster`, `charger`). Verify any chapter-specific item, tool or spell before claiming book canon.

## 9. Checkpoint, failure and fairness contract

- Distinguish **stage** (authored encounter segment) from **floor** (larger chapter/persistence boundary), **attempt** (restart epoch) and **encounter** (combat lifecycle). Stage boundaries must be represented as explicit domain events before real rewards are offered.
- **Provisional M1 encounter scope.** `src/` has no encounter, stage or floor identity today (only `Restart` = attempt, DEMO-01 `Respawn { checkpoint }` and `BossEngaged`). Until STAGE-BOUNDARY-01 exists, REW-01 defines the "encounter" of this document as: the attempt (reset by `Restart`), the checkpoint segment between DEMO-01 `Respawn { checkpoint }` boundaries, and the `BossEngaged` boundary, **plus an anti-spam cooldown of 1800 ticks (30 s)** kept as a tunable knob. Chain windows, "once per encounter" limits and the per-hero momentum baseline use this scope. STAGE-BOUNDARY-01 later replaces it with a real encounter id through an explicit migration (versioned reward rules, regenerated reward digests, legacy goldens untouched).
- An authoritative snapshot contains unlocked campaign abilities (or reference to campaign save), active floor modifiers and spent charges, momentum expiry/charges, reward offer/selection IDs, grant history, encounter epoch, deterministic RNG state and event-id dedupe. Restore atomically on restart; no duplicated rewards through death or stage re-entry.
- A normal respawn restores the checkpoint-defined state, not an extra offer. Full floor replay / new attempt follows its explicitly defined campaign-vs-floor reset mode. No wall-clock timers; fixed ticks.
- Gameplay score/grants and state changes are included in the **reward-enabled replay digest**. Keep old baseline golden hashes byte-identical while opt-in is off; explicitly version/hash changes when turning rewards on. Presentation (HUD, slow motion, gore intensity) cannot influence outcomes.
- Unknown instigator fails closed for style rewards. AI may *contribute factually* but must not be able to autonomously trigger infinite reward cycling; log human-origin vs AI-only chain distinction without falsifying viewer credit. Never steal a human finisher or make solo stronger merely because the bot is active.
- Any team bonus is awarded **once** per causal action, not multiplied by each human participant; per-hero momentum remains hero-owned independent of control slot. Co-op and solo must have comparable reward budgets.
- Preserve existing boss immunity, terrain safety and character-specific progression gates under every modifier.

## 10. HUD / UX contract

No extra always-visible meter, viewer card or inventory tab. Reuse the existing hybrid hero plates, System CRT and stage-end presentation: one small icon/notice when momentum starts, an optional tiny active-state glyph, three clear cards only at safe stage transition, and one curated sponsor notice later. Clear impact on the next valid hit must be audible/visible but never obscure enemy tells, actor silhouettes, boss strip, health or mana. At 640x360 and 640x400 validate with regular / high-contrast / reduced-motion settings and two-controller input. The sponsor's *narrative crate opening* may be animated, but grants never stop a lethal attack mid-screen.

## 11. Acceptance criteria and evidence

**Policy (this PR):** clear no-inventory decision, explicit layered reset semantics, anti-farm / solo-2P model, compatibility notes, roadmap and implementation gates. **Not** a claim that new mechanics are implemented.

**Runtime (future PRs):**
1. Domain-only unit fixtures for qualifying and non-qualifying chain, repetition damping, event dedupe, unknown credit, real duo contribution, multi-target explosion, terrain kill, A/B config and scripted noncombat cues.
2. Replay golden parity with rewards flag off; independent deterministic reward digest with it on; seeded reset equivalence and no respawn duplication.
3. Two actual opt-in momentum effects land in a real 640x360 scene; demonstrate effect vs baseline with fixed-seed screenshots/replays and two gamepads (or explicitly report hardware not tested).
4. A 2-stage test path offers three eligible distinct cards once, honors cap, persists across stage but resets at floor exit, and handles both characters and 1P/2P ownership.
5. No offer is impossible pre-unlock, no reward breaks boss/critical route, no forbidden inventory screen appears; optional and inaccessible content stays disabled.
6. AUTOPLAY scripted/held-out comparisons across 1P and 2P (when supported): win rates, damage taken, attack/finish variety, award frequency, duration, dominant pick rate, exploit detection, stage clear time, and seed-invariant fairness. Automated agents do **not** judge fun or spectacle.
7. Manual owner playtest must answer: does the reward make me attempt a different combo; is the reward perceptible without clutter; does it trivialize danger; does the between-stage choice feel worthwhile; does co-op remain fair? Failure means tune/remove reward, not add more UI.
8. Up-to-date narrative/canon, art provenance and distribution-rights review for any franchise-derived sponsor/object. No new copyrighted asset implied by this design.

## 12. Revisit triggers / explicit exclusions

Reopen this decision only if real playtesting demonstrates that meaningful action choices *require* an item-management mechanic. Even then prefer one explicit limited tool slot over an RPG inventory. Reassess after the first momentum probe, first stage-choice 2P session, broadcast/story unlock definition and new floor design kickoff.

**Deferred (not part of REWARDS-01):** subscriptions, paid/fan monetization, random loot tiers, a persistent sponsor economy, rotating shops, a level-up skill tree, weapon crafting, numeric viewership-to-power conversion, full run-reset roguelite, multiple equip screens, and speculative sponsor drops before the narrative unlock.
