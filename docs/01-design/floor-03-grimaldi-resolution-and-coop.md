# F03-GRIM-01 — Grimaldi's nonlethal boss resolution, party authority and honest combat

**Status: design proposal, NOT implemented or human-accepted, 2026-11-22.** Companion to [F03-CIRCUS-01](floor-03-circus-encounters-and-stagecraft.md) and [F03 design bible](floor-03-over-city-design-bible.md). The intention is to turn the narrative heart of Book 2 into **player-earned action** without making its abusive producer-written plot into a boring unkillable health bar or a three-lever puzzle. Exact dialogue requires original writing; no book excerpts, soundtrack, copied cover art or production rights are implied.

## 0. Canon boundary: fact, adaptation, and unknowable details

**Based on secondary chapter-by-chapter summaries, not a primary-text edition audit:**
1. Tsarina Signet's Water Lily spell temporarily leaves Donut unable to participate; Carl must defeat Heather, a Level19 tortured bear boss infected with white worms.
2. Heather dies after the parasites are removed; her death is tragic, not a happy nonlethal boss solution.
3. Signet's paper-flat ink army cuts through outer circus defenses, while the Big Top has a protective ward; Carl enters the tent and destroys its **three structural support poles**.
4. Grimaldi is a Level85 City Boss/Elite, a Pestiferous Vine. Carl first tries a poison strategy, then bargains with the show's producers and uses parasite-mediated communication and healing to persuade Grimaldi to un-banish Signet.
5. Grimaldi's conflict ends without an ordinary death; the circus story completes, with Mongo defending cursed Donut, and an early staircase to Floor04 appears.

Supporting recaps (not canonical original text):
- https://spoilerfreecarl.com/book-2/recap/
- https://github.com/cdkolb/dcc-compendium/blob/main/docs/chapter-summaries/book-02.md
- https://www.dccdatabase.com/wiki/grimaldi%27s_traveling_circus/
- https://progressiontime.com/primers/dungeon-crawler-carl/book-02/

**Game adaptations that require user review:** Carl and Donut remain controllable in Heather and Grimaldi combat; support poles become **optional gameplay efficiency tools rather than three compulsory quest-item triggers**; producer bargain and parasite/wisp/poison are authored story events (not newly equipped spells or forced inventory). Grimaldi cannot be depleted by conventional damage, but **his defenders are** and every physical attack result is authoritative.

## 1. Most important creative risk: a boss the player can't punch feels insulting

A nonlethal elite in a beat-em-up must still respond to attacks. We differentiate three sources of force:

- **Hero combat damage**: acts on actual defenders, exposed vine limb tips / infected growths and legitimate props; visible hit-stop/impulse and enemy/contact events. These cannot kill the final Grimaldi core or invent XP. No fake HP bar.
- **Progressive stage pressure**: each time a defending wave is cleared, a huge limb fails its attack, or a real support strike lands, the tent **visibly loses coherence** (lighting, canopy slack, vine silhouette changes). This is a deterministic setpiece state, not a hidden automatic "punch the scenery 10 times" stamina bar.
- **Narrative communication**: after the player has survived combat, the root physically opens enough to receive the party at a protected point. A single compact original prompt/action or fully automatic close-range contact starts the resolution. This is a readable act of choosing connection over indiscriminate destruction, but not a branching dialogue tree or a new gameplay input.

**Do not create:** boss core HP, scripted auto-kill from prop physics, random scripted health restore, three identical pole lever tasks, compulsory player spell ownership, no-hit requirement, guaranteed free sponsor reward or rare cinematic killing slow-mo.

## 2. Boss flow: six authored states, three player-controlled rings

The root remains within the background/foreground boundary, huge and alive; only defined attack volumes can hurt.

| State | Trigger | Visual truth | Player interaction | System truth |
| --- | --- | --- | --- | --- |
| GR0 WardOutside | party at Big Top entry | ink army hits barrier; huge dark center silhouette | move into tent under story entry flag | no damage from curtain VFX |
| GR1 DefendersActive | ring 1 entrance | tent mouth twists, blade lemur + low guard | real attacks and dodge, optional curtain support | defeated count / actual grounded contact |
| GR2 RiggingActive | ring 2 clears | three tall poles, clown and lion stagger | direct fight, real supported pole collisions | single chosen pole can tilt, no mandatory three count |
| GR3 RootAwake | ring 3 reachable | giant vine eye/knot opens; one limb telegraphs | real defender fighting, block/evade and punish open tendril | cannot cause regular boss HP death |
| GR4 ListeningWindow | last defending pressure resolved, one final limb overshoot | terrible vine now withdrawn, roots open, original cue | hero approaches protected contact ring or existing mapped context action | one authoritative `communication_open` flag and scene safety |
| GR5 Reunion | contact resolved once | Signet's two-dimensional ink silhouette enters; enormous vine holds still | short skippable visual/narrative beat | `grimaldi_resolved` exactly once, no kill XP, quest outcome |
| GR6 Exit | reunion settled | tent lights down, survivors audible, space open | both players move to quiet rescue section | story route S06, no Floor04 transition |

The **real gameplay gate** for GR4 is: the last ring's **finite combat defenders** have been neutralized, the immediately committed vine attack has resolved, and the safe contact area becomes reachable. Collapsing poles can reduce incoming defender pressure or widen safe lanes, but never substitute for an unproved fake defeat. A player who destroys zero poles can still finish.

If the implementing engine only supports room-level clear state, represent GR0–GR6 with finite scene authoring controller and two actual enemy waves; don't invent a global quest engine. One central story/scene event owner persists the boundary, old checkpoint replays remain stable.

## 3. Grimaldi's three readable attacks and three ways of responding

**Attack A — Top-Down Theatre Slam.** A thick root appears above one authored lane, withdraws and swells visibly, then falls with a committed contact rectangle. The hero may move, jump or blink if already earned; on impact the root pauses and may receive real attacks to its exposed limb tip. A foot-level attack route for Carl must be reachable at least once after recovery. Repeated standing under a root should fail, but not through untelegraphed body aura.

**Attack B — Curtain Sweep.** One horizontal vine crosses the mid-level of the arena while the safe low/raised alternative remains viable for each hero. A committed direction is frozen before sweep, so Donut's side movement and Carl's short jump are useful without a new dodge button. Do not combine Sweep + Slam in the first prototype.

**Attack C — Root Pull.** A narrow marked ground strip tugs either hero toward the central root; the player gets a real reaction budget from current 60Hz movement, can flee to cover or interrupt an exposed limb. No forced control inversion, random stun, unavoidable double-hit or undeclared player disappearance.

**Answer 1 — normal combat**: finish live guardians, move/evade, punish real root exposed limb if useful; reach GR4, the meaningful story resolution. Neither hero needs a special consumable.
**Answer 2 — inventive physical**: launched enemy strikes a pole/rig in a single real contact and makes the next root attack overcommit; access GR4 earlier with less fighting pressure, but not for free.
**Answer 3 — botched trick**: player misses sign, support remains intact, one hazard activates but they survive and finish by honest attacks. No scripted reset nor trap that requires a precise retry.

The book's original poison->heal->parasite->communication strategy is retained as **story staging** inside the resolved GR4/GR5 sequence, so we communicate Carl's intelligence without creating a new parasite-spell minigame or inventory.

## 4. Two-player fairness and solo companion policy

**Canonical divergence to make explicit:** in the source, Heather and the Big Top are effectively solo Carl while Donut is cursed. Here removing Donut's controls for two entire chapters is unacceptable under the current game product direction. Proposed adaptation:
- Before S04, a short original water-lily threat tableau establishes the hostage risk; a fictional weaker/shortened effect can bind Donut's *story fate* without disabling her gameplay combat. Do not falsely label this book-accurate.
- Carl and Donut remain actionable in every required combat state; never render either as a corpse/inaccessible image just because the plot expects them sleeping.
- Solo with Carl player: AI Donut may attack defenders, evade the visible root, flee a triggered fall, and move to the final contact without receiving scripted invulnerability.
- Solo with Donut player: AI Carl can attack defenders, use optional support contact and follow through; no requirement for player to switch to Carl for the final narrative link.
- Two real humans: root attacks alternate meaningful targets / preserve fair line-of-sight, each can use the stage physics, no fixed simultaneous press to unlock reunion. The reunion can start from either hero approaching while the other is in a safe active area; a short regroup cue and input buffering may be used. No sudden teleport to the root while a player is in active combat.
- Companion's actual helpful damage, movement, hazard avoidance and event attribution must be measurable through supported PLAYTRACE / AUTOPLAY party simulation where real interfaces exist, not by animation counts.
- The narrative can still visually present Carl as the negotiator without rendering P2 inert: Donut reacts vocally/originally, communicates frustration and emotion, and both regain control promptly after the beat.

**Do not assert human co-op validated** from a two-slot scripted bot. That gate remains owner playtest pending.

## 5. The Signet relationship as a game mechanic WITHOUT a dialogue tree

Instead of a 25-line quest-choice menu, stage three visually understandable transitions:
1. **Threat**: Signet stands outside the shimmering ward while an exhausted tent defender is still attacking. Her flat ink army is striking the boundary. Tiny panel/broadcast motif implies producers are watching.
2. **Question**: after real physical combat, Grimaldi withdraws. The music/pulse ends, a recognizable original two-tone "listen" cue plays and his central root uncurls. The hero sees **one reachable protected contact location**. Nearby repeated attacks still strike harmlessly/recoil, but there are no active damaging root tells during contact.
3. **Answer**: at reach, an original symbolic parasite-thread effect passes between Carl's and Grimaldi's silhouettes, the signet/ink figure reaches the threshold and a living ring of roots opens. It is **not** a normal boss death. Keep original original text short, no substantial quotations. Any optional two-way "strike versus talk" branch is **DEFERRED**, not a promise of two finished endings.

This honors the intent of the novel's surprising creative solution without asking the player to memorize producer contract clauses or perform an unrelated token-fetch puzzle.

## 6. What the score and HUD must say

- **No boss health bar** for Grimaldi. Instead at most a compact "Bigtop Ward — DEFENDERS" count and explicit active root tell/phase, grounded in actual combat state; don't add an unrelated HUD widget until owner approves layout.
- Hero hits to defenders yield ordinary actual damage and individually attributed viewers; real launched-prop collateral may qualify for a modest spectacle feat.
- One-off `grimaldi_resolved` yields a story/chapter completion and optional once-only result-panel context, **not** enemy-kill XP, no generic death gore and no high-skill kill slow-mo.
- Signet's flat paper army is a background narrator outside the ward, not an infinite enemy feed/AI partner army.
- Heather's result deserves a respectful pause, and her end is not a repeatable jackpot to farm. Save character progression receipts on true storyline milestones without repeating on retry.
- If either hero is defeated during GR1–GR3, retry from safe intro to appropriate ring, with all props/defender states restored. GR5 must never play twice after a resolved quest.

## 7. Technical event model and tests

**All names below are proposed IP-neutral source IDs, NOT claims of implemented enums.** One authoritative scene-state update owns them; presentation listens without generating damage.

- `tent_ward_entry_open` (party / story entry flag).
- `ring_cleared(ring_id, cause_tick)` from actual enemy clearance.
- `support_impacted(prop_id, root_source_id, kind)` from real env_prop stimulus / swept collision.
- `root_attack_committed(lane, tick)` and `root_attack_recovered(lane, tick)` from actual sim.
- `contact_window_exposed(tick)` after final pressure and recovery.
- `communication_started(hero_id, tick)` with a single causal party authorization event.
- `grimaldi_resolved(outcome=nonlethal, tick)` idempotent.
- `circus_vacated` and `early_floor04_stair_revealed` separate from `party_descend_to_floor04` (S12 only).

| Test | Required evidence |
| --- | --- |
| GRIM-T01 | S05 with all 3 supports untouched: defenders cleared, contact window available, successful nonlethal resolution |
| GRIM-T02 | true launched enemy support impact changes one rooted attack/recovery state and credits root input |
| GRIM-T03 | missed support setup: direct finish, no softlock, no mandatory "hit all three poles" |
| GRIM-T04 | repeated physical hits / spell projectiles cannot reduce fake Grimaldi HP to zero or earn boss kill reward |
| GRIM-T05 | root ambient art cannot damage; only committed lane with warning and one actual contact |
| GRIM-T06 | Carl-only lead + AI Donut and Donut-only lead + AI Carl can achieve resolution with normal controls |
| GRIM-T07 | both scripted input slots remain owned, no permanent Donut freeze / hidden solo switch in boss |
| GRIM-T08 | mandatory first attempt and retry checkpoint, exactly-once quest/story receipt and individual viewer attribution |
| GRIM-T09 | S06 early stair never performs Floor04 party transition; story final S12 true exit works after proper flag |
| GRIM-T10 | 640×360 and 640×400 in-game render still and game-speed clip show telegraph, protected contact and readable duo |
| GRIM-T11 | source chronology and dialogue checked by human editorial reader against actual licensed primary text, otherwise labeled SECONDARY |
| GRIM-T12 | actual real 2 gamepads, owner emotional/verdict, F01/F02 regression and game feel **PENDING** |

**Stop before implementation** if the desired story-contact mechanism requires a fifth button or rewriting global input/companion/loot. Use a verified existing contextual interaction or story-contact-on-reachable-zone after earned combat, with a visible, skippable presentation; keep that decision local to this setpiece.

## 8. Design gate for the owner

The owner must judge three questions with a *real playable greybox*, not concept drawings alone:
1. **Is a resolved nonlethal giant satisfying after a full physical fight against defenders?**
2. **Does the forced story resolution feel earned, or does the player want a genuine limited choice?**
3. **Would a short solo-Carl-only **optional** challenge mode improve book fidelity enough to justify losing the P2 hero for that mode?** It may be offered as a separate challenge, never replace the normal 2P playable campaign without approval.

No release of sourcebook-expressive elements/playable game beyond existing rights policy is authorized by a docs PR.
