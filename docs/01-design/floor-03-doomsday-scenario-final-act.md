# F03-DOOMSDAY-01 — Remex, the two explosions, four precursor bursts and the last stair (S11–S12)

**Status:** design proposal; not implemented, tested, rights-cleared as a game or approved in owner human playtest. 2026-11-22. Companion to [Floor03 bible](floor-03-over-city-design-bible.md), [Skyfowl story encounters](floor-03-skyfowl-city-and-quill-encounters.md), [art key brief](../02-art/floor-03-skyfowl-and-doomsday-art-direction.md) and project controller/no-inventory ADR. **This is the payoff of the Over City, not a copy of Floor02 Rage's one-pursuer escape.**

## 0. Crucial story architecture: TWO disasters, one player-made outcome

Book 2's climax unfolds as **two crises** in sequence:
1. **Quill's unfinished Final War / soul-crystal explosion:** Carl reaches Remex (former crawler, a **Soul Leech Capacitor**, not the expected lich); Remex reveals Miss Quill's plan. The unstable soul crystal is captured in a **Sheol Glass Reaper Case**, making the enormously dangerous contained **Carl's Doomsday Scenario**.
2. **The separate Remex/wild-magic detonation:** containing the crystal does **not** cancel Remex's own countdown. The escape is threatened by **four precursor bursts** that disrupt/remove/activate magic equipment before the full blast. Carl, Donut, Katia and survivors reach Floor04; Katia triggers a separate delayed depot detonation near the end. In the source Carl stores the device as inventory and strips off gear while fleeing.
Secondary references, verify exact primary details editorially: https://spoilerfreecarl.com/book-2/recap/ ; https://spoilerfreecarl.com/book-2/chapter-25/ ; https://github.com/cdkolb/dcc-compendium/blob/main/docs/chapter-summaries/book-02.md .

**Our adaptation with no inventory and arcade controls:** first the player earns containment through real combat/defense plus a bounded contextual event. Then the *world rules* transform around Carl and Donut as they escape through a variety of short physical/cover challenges. The unstable device is **one unique sealed story prop/flag**, not a new RPG inventory slot or reusable bomb weapon. Pulses affect **visible stage props, hazards and presentation**, not surprise-delete earned hero moves, approved Donut visual identity, or enforce a lengthy equipment management UI.

**Emotional beat:** "We stopped it." -> a new much larger countdown begins -> "That wasn't the explosion." -> four worsening world distortions -> a desperate compassionate escape to the last door.

## 1. What happens before the countdown: S11 THE SOUL LEECH CAPACITOR

**Stage length hypothesis:** 2–3 screen widths inside an industrial warehouse and upper containment chamber. Physically narrower than broad Over City; visual vocabulary reuses gigantic loading chains, tall shelves and enormous floating cyan/violet conduits, not yet a sky-destroying explosion.

### Pocket W0 — The depot fight

- One ordinary Shambling Berserker in reachable ground lane (already introduced or taught in an earlier stage), then **one city-elf defender** under a separate visible telegraph. No sudden mass boss add-wave at the final hour.
- A hanging lift assembly can be struck by **legitimate hero attack or launched body** and drops one weight with a clear warning and actual contact. Direct kills all work without that prop.
- A physical banner announces the separate **Swordsmen depot**; a scripted narrative party ally Katia can be seen receiving a single **fictional stage detonator** for a later story event. Do not simulate real explosive construction, electronic wiring or tutorial diagrams.
- This encounter matters mechanically: one final payoff for the game's physical combat before the most narrative-heavy setpiece.

### Pocket W1 — Remex's confession

- A frightening large silhouette is revealed as a **helpless being bound to a soul crystal**. He **does not use a lich attack loop, have HP or count as a boss target**. No "kill Remex for xp" route.
- The camera provides a protected, reachable interaction area; real combat fully ends before the story-contact event. The confession can be a brief original motion graphic / voice-less vignette in simple stages, <10s skippable on retry, no verbatim book dialogue.
- Story flags in sequence: \`remex_found\` -> \`final_war_explained\` -> \`crystal_unstable\`. A connected gameplay route must not skip these flags just by crossing the x-exit collider.

### Pocket W2 — The first disaster: real containment decision

- A growing **glowing soul crystal** builds a clearly telegraphed dangerous radius from the center of the room. The player has a safe physical route to a supported enclosure/console without new tools, inventory or multiple switch input.
- First player option **direct safe setup**: after one bounded defensive exchange, enter a marked protected region and activate existing context input (or equivalent single verified scene-contact) to move an authored **Sheol Glass case** over the crystal. The case is a **fixed world mechanism** in this adaptation, not carried in Carl's invented backpack.
- Second option **expressive setup**: Carl legitimately kicks an enemy into the overhead arm support / Donut hits a reachable brace with her existing attack, causing the case assembly to shift early and giving the duo a more dramatic but still physically causal containment. **Missing the support does not lose the story**; they can finish the short defender clearance and use the direct protection route.
- Successful closure yields one authoritative \`crystal_contained\` (not \`remex_exiled\`) and \`doomsday_device_sealed\`. Sound abruptly disappears while the visual blast is trapped behind the case; no ordinary XP for hitting the case or a fake enemy kill.
- One short false victory beat. Immediately present **a second distinct amber/violet warning from Remex's body** and announce that the true danger remains. Do not portray first containment as accidentally causing the second blast; Remex already represented the underlying unstable magic.

**No actual world explosion** at the moment of capture; a short in-world silent bloom inside the case and backed-off shock on the fixture can show contained energy, but no offscreen massacre caused by VFX. The party must be able to reach the S12 exit after case closure.

## 2. S12 THIS CITY IS A BOMB: escape versus changing WORLD CONDITIONS

**Stage length hypothesis:** 4–6 contiguous, high-pressure *screen-sized pockets* without a giant continuous sprint, with **FOUR precursor events** at authored progress fractions and a single true Floor04 exit. Each precursor adds a *different, clearly shown environmental constraint* and leaves the core hero input grammar intact.

The biggest differentiator from Floor02 Rage: here **no giant enemy directly hunts you**. The city becomes unstable around the heroes; each pulse changes a known rule and allows short tactical combat/terrain expression. Give the player agency between blasts (kick object, save route, block lane) rather than 90 seconds of holding right.

| Scene slice | Visual setpiece | Pulse effect (GAME ADAPTATION) | Player's meaningful response |
| --- | --- | --- | --- |
| E0 / Escape order | Remex warehouse backdoor, panicked crowd silhouettes | prelude + large clear map arrow to true stair | move with two controllable heroes; Katia's role explained visually |
| E1 / 1st precursor | narrow city street with glowing enchanted lamps | **ward/light fields flicker off** and several magic *stage* props become inert | choose safer ground under structure, fight one limited pursuer directly |
| E2 / 2nd precursor | partly collapsed viaduct and huge empty sky hole | **spent magical props overcharge** and announce one localized chain (not all props) | act physically to dislodge one cart into a cover brace or take a wider safe path |
| E3 / 3rd precursor | central market / shattered skyline | **one genuine authored falling façade** plus other non-damaging magic art failing | protect a clearly safe crowd corridor by intercepting an attacker or opening an alternate covered lane; no escort HP chores |
| E4 / 4th precursor | descending access bridge / final door visible | **short directional magical anomalies** warn at ground and overhead, one at a time | cross via committed safe lane and one actual physical obstacle; both heroes retain attacks |
| E5 / Final stairs | a heavy floor door under collapsing fake sky | Remex's imminent final white silhouette envelops background only | Katia's visible distant relay event; open story-authorized door / both heroes to Floor04 |

**Original-book vs game differences:** source precursor bursts remove enchantments, destroy equipped gear and trigger carried items; we use them to rewrite environmental interaction states. A no-inventory game must not suddenly show an "unequip everything" submenu. The bursts should still **feel** like the dungeon's rules breaking: in-world signs glitch, magic glass cracks, stage lamps lose color, and familiar safe-looking structures turn lethal only after readable warnings. Carl's learned punch/kick, Donut's earned magic, companion input, and their approved source sprite identities are **not** randomly deleted. Source canonical loss of Donut's Sepsis Crown can be represented by an optional brief contextual story animation after art owner consent, but **do not remove her current approved crown from native appearance in the middle of a fight** without a separate approved design/visual pipeline.

**Katia role:** In the original, Katia actually triggers the depot explosives near the stair. The game shows her distant position and a single **once-only relay story flag** with audible/visual acknowledgement; she is NOT required to pathfind as a third permanent follower, hold an invisible button or be controlled by a human. If ally actor support matures, a local deterministic support action is possible only under its existing tested authority.

**Daniel Bautista:** may be seen near a protected stair leading refugees but should not be an extra enemy/required player. A short safe shot shows he knows the escape route. The player is moving among survivors, not escorting dozens of fully simulated NPCs.

## 3. Pressure, pace and accessibility

### Attack overlap constraint

For each precursor, active fatal threat coverage must leave at least one **physically reachable and camera-visible** route for *Carl's baseline mobility* and Donut's base mobility. An alternate short ledge is allowed; no special blink spell required, no need to jump with pixel-perfect 1-frame window. If one player is ahead and one behind, the scene must not trigger a pulse that traps the other human offscreen, beyond any actual established co-op camera leash policy.

**Pacing hypothesis:** 20–45 seconds of meaningful action between pulses, total 3–6 min escape; not validated against controller feel, and must defer to authored scene footprint/movement speed. A pulse itself takes a few seconds from readable *foreshadow -> commit -> resolve -> safe breath*. Do not progress a precursor on a UI timer alone while the party is in a protected checkpoint or pause; rely on fixed sim ticks and stage states. The source's longer countdown is narrative inspiration only.

### First-clear and replay rules

- A player can fail an active hazard and retry from the safe checkpoint **before that pulse**; earlier resolved pulses remain consistent story state. Restart resets active visual and collision state, no double viewers.
- A short retry should not replay Remex's 10-second confession or make players re-solve the case if already protected by a legitimate checkpoint and story flag.
- S12 is **not** an arbitrary "no hits ever" gauntlet. Damage is allowed, recoverable, and still grounded in action. Recoil/hit-stop must not make the next pulse unavoidable.
- Captions/reduced shake and reduced flash settings must preserve every danger cue. White flash is **background only** during any active combat window. Never hide the player under excessive light, HDR bloom or 100% alpha overlay.
- Rather than forcing all crowd NPCs to render/animate at 60Hz, use low-cost silhouette parallax with 2–4 actually relevant protected individuals; stage constraints, not ragdoll mass casualties.

## 4. Narrative/staging separation and normal combat viability

**Ordinary defenders and pursuers still require real direct attacks.** If a prop is removed, the stage can be completed with normal combat + a fixed safe route. Special showpiece props shorten or dramatize escape; they do not substitute a required "bring the bomb" item hunt. The final true Floor04 door is an explicit *story-progress* gate; other doors/early stairs seen at S06 are NOT the real transfer.

**First containment is a story milestone, not a kill.** Remex remains alive but doomed during the first contained crystal blast, and his final dangerous wild-magic event is distinct. Neither his suffering nor the souls in the crystal should score as repeated kills. Exceptional combat slow motion requires a genuinely skilled physical kill of an ordinary enemy; do not award it to a huge mandatory narrative explosion, magical pulse or victim death.

**Three safe story transitions that must never be conflated:**
- F02 S07 Rage-only stairwell expulsion → party still Floor02.
- F03 S06 early circus Floor04 stair *revealed visually* → party still Floor03 (linear game adaptation).
- F03 S12 final heavy stair door → party actually enters Floor04 after \`crystal_contained\` and \`remex_escape_committed\`.

The game may later revisit whether book-accurate optional early Floor04 exit is desirable; it is **not** v1.

## 5. Candidate deterministic state/event contracts (not currently compiled APIs)

\`S11DepotDefenderCleared\`, \`RemexContacted\`, \`FinalWarRevealed\`, \`SoulCrystalUnstable\`, \`CrystalCaseContacted(cause)\`, \`SoulCrystalContained(root_action_source)\`, \`DoomsdaySealedOnce\`, \`RemexCountdownStarted\`, \`PrecursorCommitted(index, safe_lanes)\`, \`PrecursorResolved(index)\`, \`KatiaRelayArmed\`, \`KatiaRelayTriggeredOnce\`, \`FinalStairEntered(party)\`.

All stage event names and descriptions are candidates, not the real \`src/story_unlock.rs\` grammar. One central scene/story authority owns stage state and persistent flags, with SHA-pinned authored data. Presentation layer **cannot** kill, move actors, award viewer currency, change magic buttons or carry players between floors.

**Safety of one-time receipts:**
- \`doomsday_device_sealed\` cannot be received twice on retry.
- \`KatiaRelayTriggeredOnce\` cannot fire at initial S12 spawn or by pressing an invented fourth-character controller.
- \`FinalStairEntered\` must include both active hero IDs and require a safe complete stage state.
- Roster and scenic civilians must not automatically be part of a gigantic blast damage sweep if they live outside authored gameplay volumes.

## 6. Tests and negative fixtures

| ID | Proof |
| --- | --- |
| DOOM-T01 | Remex is a story/captive NPC, not a lich-style boss HP target; no lethal contact from idle art |
| DOOM-T02 | depot basic combat direct-only and one physical suspended-weight impact, genuine source attribution |
| DOOM-T03 | case containment can occur without prop combo and with legitimate multi-action prop cause, after required safe setup |
| DOOM-T04 | first crystal explosion marked **contained**, then distinct Remex countdown begins; one does not silently resolve the other |
| DOOM-T05 | no inventory, special switcher, new bomb-building interface or gear-stripping mandatory control |
| DOOM-T06 | four distinct precursor states 1–4 in strict order, never repeated/skip after checkpoint |
| DOOM-T07 | no pulse causes unavoidable full-view simultaneous lethal volumes; Carl/Donut each have legal reachable safe route |
| DOOM-T08 | E0–E5 each reachable even when optional stage props ignored/destroyed |
| DOOM-T09 | solo AI partner follows moving hazard cues/route with actual 1P authority; 2-slot scripted paths verified when supported |
| DOOM-T10 | Katia's once-only authored relay triggers after safe prepare and before final door, never an extra player slot |
| DOOM-T11 | F03 early Floor04 stair blocked / not a transfer; only post-crystal S12 final exit transfers both active heroes |
| DOOM-T12 | genuine 640×360/400 real Bevy capture, VFX not hiding input/tells, real game-speed pulse evidence |
| DOOM-T13 | PLAYTRACE hashes for pulse ordering, route, retry, companion and unique story/viewer receipts; corrupt source pin rejected |
| DOOM-T14 | artificially disabled input/removal of magic must be rejected by contractual regression tests rather than passed as thematic effect |
| DOOM-T15 | two real human gamepads, actual game feel/emotional clarity and video review **NOT RUN** until owner session |

**Core negative tests:** attempt S12 true exit before \`crystal_contained\` (must fail), attempt to call \`miss_quill_dead\` after Remex story reveal without prior demolition flag (must fail), inject pulse 4 before pulse 2 (reject), remove all stage props and show direct route still works, set both players on opposite edges and confirm no surprise pulse trap, duplicate a sealed-case receipt and ensure it does not score/grant twice.

## 7. Original art keys and engineering dependency

- **F03-DOOM-KEY-01 — The Caged Soul:** Remex small and wretched inside enormous ritual-industrial capacitor, not a conventional evil lich.
- **F03-DOOM-KEY-02 — The Silent Success:** a brilliant trapped soul crystal inside one glass case; two heroes are lit by a *contained* event. Protected foothold, not all-white unusable screen.
- **F03-DOOM-KEY-03 — The Second Countdown:** giant skyline false sky shimmers from violet to gold while a disembodied Remex warning sounds. The *same* city from the chapter A vista is being consumed.
- **F03-DOOM-KEY-04 — Four Precursor Pulse Sheet:** four compact states of one city street (ward darkening, machine overload, real falling fascia, directional anomaly), each with distinct **safe lane** and two readable hero silhouettes.
- **F03-DOOM-KEY-05 — The Final Door:** an original vertical wall of light behind the protected Floor04 stair, Katia distant silhouette doing a **fictional relay event**; return of darkness for credits/next-floor hook.

**First implementation task:** pure event graph and negative tests S11/S12 without art or new story API. **Second:** actual one-room Remex narrative + one distinct contained crystal. **Third:** stage pulse 1+2 using existing env-hazard and scene/camera primitives, real current party route and strict replay. **Fourth:** add pulse 3+4, full story + final exit. **Last:** art, true cinematic audio and owner review. If Floor02 scene runner isn't merged, do only isolated logic/data/art and avoid touching shared core.

**This is a creative level design specification, not instructions to build physical explosives or real hazardous devices.** All charges, relays, glass case and pulse physics are fictional authored game props and independent narrative state machines.
