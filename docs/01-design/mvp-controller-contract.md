# MVP controller grammar — THE CRAWL

**Decision status: OWNER-APPROVED DESIGN (2026-11-18).** The control *grammar*, MVP default button layout, and character identities below are approved for implementation. Concrete frame timing, damage, aim assistance, move variants, progression balance, game-feel, camera behavior, and advanced controls are **not yet playtest-approved or implemented**.

**Scope:** first playable Floor 01 MVP, with stable action meanings across future floors and additional character abilities. Controller-first on Linux, with DualSense as the reference layout and Steam Controller / Steam Deck / other gamepads mapped through device-neutral actions. This is a fan-concept design, not a canon claim or a shipping/rights approval.

**Authority:** This document supersedes the *proposed* button tables in [gameplay](gameplay.md) and any *temporary fixture bindings* in [input/camera](../03-tech/input-camera.md); it does **not** retroactively claim the current Bevy scene has these controls. Read with [combat feel](combat-feel-hybrid-physics.md), [Floor 01 progression](floor-01-gameplay-progression-v0.md), [pre-magic Donut experiments](floor-01-pre-magic-donut-experiments.md), and [integrated-scene playtests](../07-agents/floor-01-integrated-scene-playtest.md).

## Design pillars (approved)

1. **Broforce-immediate start:** movement + jump + two meaningful offensive actions; special actions arrive only at story-appropriate unlocks. No mandatory twin-stick aim, dodge, sprint, grab, long memorized combos, or weapon-selection menu in the initial encounter.
2. **Four permanent face-button verbs:** jump, primary attack, physical attack, equipped special. A new skill should first expand what these existing verbs do, not introduce a new required button.
3. **Carl kicks; Carl does not grab enemies.** He punches, kicks, stomps and redirects opponents by impacts and physics. Never silently turn his physical attack into a grapple, carry or throw-by-hand mechanic. Interact is for devices/items/doors, not combat grabs.
4. **Asymmetric protagonists from frame one:** Carl has reliable close combat and selective physical/environmental disruption; pre-biscuit Donut is a fragile, fast, high-jumping cat with very limited damage. Post-biscuit, her primary attack becomes Magic Missile without removing her feline physical action or traversal identity.
5. **Emergent depth, not input complexity:** staggers, knockback, airborne hits, enemy collisions and selective terrain destruction create combinations. Ordinary enemies die after a few meaningful hits; deterministic authoritative collision/damage governs outcomes.
6. **Solo and local co-op use the same action semantics:** single player directly controls one hero and swaps to the other with AI assistance; two players control Carl and Donut independently from the beginning. Same critical route in both modes, including character-specific gates.
7. **Progression is data/capability driven:** the input binding does not encode the floor, character name, skill ID or a specific bomb/spell. Later floors may add variants/equipment while keeping these verbs stable.

## Approved default layout

| Action / semantic verb | DualSense | Xbox / Steam Controller / Steam Deck | MVP expectation |
| --- | --- | --- | --- |
| Move / face | Left stick or D-pad | Left stick or D-pad | Required; D-pad movement is supported |
| Jump | Cross (south) | A (south) | Required |
| Primary attack | Square (west) | X (west) | Required |
| Physical attack | Triangle (north) | Y (north) | Required |
| Equipped special | Circle (east) | B (east) | Action slot available but not necessarily unlocked at spawn |
| Interact / pick up / activate | R1 | RB | Required; never an enemy grab |
| Swap active character (solo) | L1 | LB | Required for hybrid solo target |
| Pause / menu / inventory | Options | Menu / Start | Required |
| Precision aim modifier | L2 | LT | Reserved optional extension; default play cannot depend on it |
| Cycle equipped special | R2 | RT | Reserved; enable only when more than one special is selectable |
| Optional precision aim direction | Right stick | Right stick or right trackpad | Optional; absent on original Steam Controller as a stick |

The game must not require touchpad gestures, gyro, rear paddles, L3/R3, adaptive triggers or a right stick. Steam Controller (2015) may use its right trackpad for *optional* precision aim; newer Steam controllers and Steam Deck can map the same action. Steam Input can provide platform mappings, but no specific Steam SDK integration is declared implemented.

**D-pad conflict policy:** if the D-pad is used to move, it cannot also be required for consumable/special cycling. R2 is the reserved cycling control; menus or device remapping may provide other alternatives. Show platform-appropriate glyphs and allow future remapping without changing action identities.

## Character actions and progression

| Verb | Carl at entrance | Donut pre-biscuit | Donut after biscuit / later Floor 01 |
| --- | --- | --- | --- |
| Move | Grounded, controlled momentum | Faster acceleration, quicker reversal; agile traversal | Retains agility |
| Jump | Human jump, grounded aerial control | Higher/farther cat jump | Retains higher/farther movement |
| Primary (Square/X) | Quick punch | Low-damage defensive swat | Magic Missile after verified unlock; remains movement-compatible |
| Physical (Triangle/Y) | **Kick**: damage + directional push/knockback | Short feline pounce/swat with low damage and positional value | Keeps feline contact action; meaningful risk due to fragility |
| Special (Circle/B) | Not available before the story grants a usable tool | Not available before story unlock | An earned, equipped ability (e.g. Puddle Jumper only after Juicer) |
| Interact (R1/RB) | Doors, props, switches, pickups and appropriate objects | Small plausible cat-accessible interactions only | Same fiction-aware rule |

**Proposed, NOT locked movement variants:** aerial Carl kick/stomp, Donut pounce-bounce, stronger knockback against stunned enemies, downward directional attack. These are playtest candidates, not literal book abilities. Physical interactions should not invent cat-operated human machinery. No shotgun or pre-unlock bombs/magic.

**Equipable specials:** the contract guarantees one active special slot and an optional future way to cycle available specials. Carl may eventually equip dynamite/placed explosives or tools; Donut may eventually equip earned spells. Tool selection must not bind a specific explosive/spell to the permanent button. No default dynamite at entrance. Magic Missile is **Donut's primary attack** after the biscuit, not a second special button.

### Story-aligned Floor 01 teaching sequence (design, not asserted canon timings)

| Beat | Teaching / extension | Input change? |
| --- | --- | --- |
| Entrance / Goblin Murder Dozer | Run, jump, Carl punch and kick; Donut evade, jump and low-impact feline action | No |
| Tutorial Guild / enhanced pet biscuit | Donut gains Magic Missile on primary attack; Carl only modest equipment/passive growth | Same buttons |
| First encounters / Hoarder | Stronger hit outcomes and enemy-to-enemy interaction; Carl starts using story-available dynamite as his equipped special once earned | Unlock Circle, not an extra verb |
| Goblin workshop / Rot Stickers | More useful destructible materials, traps and explosive configurations; Donut keeps evolving mobility/aim | Same buttons |
| Labyrinth / survivors | Encounters demand better positioning and cooperation | Same buttons |
| Juicer gym | Carl's gauntlet enhances fist strikes; Donut earns Puddle Jumper *after* Juicer, available as an equipped special | Same buttons |
| Ball of Swine / exit | Mastery test combining established abilities with geometry/environment | Same buttons |

No Floor 2+ ability is approved here. New floors should define capabilities for these same verbs and add input slots only after a separately reviewed reason and migration plan.

## Input behavior: approved intent vs tuning hypotheses

- **Jump:** responsive with variable height and air control. Exact coyote time, jump buffer, acceleration and height remain tunable.
- **Primary:** reliable, repeatable combat action. Whether holding Square auto-repeats, the repeat cadence and attack buffering are **unresolved**; do not mark auto-repeat as approved.
- **Physical:** a distinct deliberate press for Carl's kick or Donut's feline physical action. Attack launch angle, priority, damage and stagger thresholds remain test hypotheses.
- **Context:** facing and locomotion determine sensible attack direction. Exact air/down modifiers are optional experiments. Avoid unexplained auto-switches of move type at borderline ranges.
- **Special:** disabled/grayed out with a clear story-appropriate cue until earned; resources, cooldown, press-vs-hold, trajectory/placement and cycling behavior need their own ability spec.
- **Aim:** horizontal/facing attacks work without the right stick. Target assistance and optional L2/right-stick precision are future tests; they must not gate critical progression.
- **Interact:** R1 has deterministic target priority and a clear prompt; avoid accidental pickup during combat and do not repurpose Triangle for generic interact.
- **Move / dodge:** no mandatory dodge or sprint button for the initial MVP. Avoidance comes from positioning, jumps, and Donut's agility; later mobility upgrades require their own review.
- **Remapping and accessibility:** controller glyphs, gamepad-only menus, device reconnect/release, reduced shake/rumble and color-independent cues are retained requirements.

## Solo, co-op and future companion controls

- **Solo:** L1 switches directly controlled Carl/Donut; the inactive hero is AI-driven. Preserve action authority and handle camera/velocity continuity. Later companion command wheel on *held* L1 is **reserved only**, not approved for implementation in this PR.
- **2P:** player 1 Carl, player 2 Donut; L1 must **not** steal the other human's character. A co-op ping on L1 is a candidate, not a required or approved first implementation. Hot-join, disconnect and AI handoff must be explicitly tested.
- **Shoulder Mode:** optional, unresolved experiment. If later accepted, R1 can be a contextually prompted mount/dismount interaction, but only if it does not disable meaningful P2 participation. No early mounted magic.
- **Shared camera:** preserve approved native sprite scale (Carl 52px / Donut 40px) and safe duo framing; no uncontrolled split/zoom that makes threats unreadable.
- **Steam input:** player slots bind logical devices/actions, never assume hardcoded physical IDs or a PS5 console target. Same game actions must work with DualSense and both original trackpad-based and newer Steam Controller layouts.

## Bevy implementation contract and migration

**Current state (2026-11-18):** the integrated scene/greybox have *temporary* input bindings. In the post-chase CP-01 fixture, `attack_pressed` is the punch and `special_pressed` is temporarily the kick because pre-Guild Carl has no explosive. This is **implemented probe behavior, not the approved MVP control contract**. Do not misreport a docs-only PR as gameplay implementation.

**Target logical actions** (names can be adapted to Rust style): `Move`, `AimOptional`, `Jump`, `PrimaryAttack`, `PhysicalAttack`, `EquippedSpecial`, `Interact`, `SwapCharacter`, `CycleSpecialOptional`, `PrecisionAimOptional`, `Pause`. Keep physical bindings in device adapters; emit ticked per-player input states/events to deterministic game-domain logic. Unlock state resolves the action in a generic capability/ability layer; game-domain logic must not depend on physical button codes or narrative character names.

**Migration order:**
1. Add a distinct physical-attack action without breaking WORK-021/CP fixtures; migrate old `special_pressed`-as-kick wiring in the integrated scene, with regression coverage for both punch and kick.
2. Bind Square/X -> primary, Triangle/Y -> physical, Circle/B -> equipped special (only post-unlock), R1/RB -> interact, L1/LB -> solo swap; preserve keyboard adapters and clearly document temporary defaults.
3. Test deterministic command recording/replay, one press causing one kick, action buffering during recovery, button-hold semantics, and no phantom action at startup/reconnect.
4. Verify the same encounters with Carl and Donut, independent controllers, safe camera, Steam Controller mappings, and both 640x360 and 640x400 presentation.
5. Only then experiment with repeat/hold attacks, aerial/down variants, optional precision aim and special cycling. Keep failed experiments reversible.

## Acceptance gates for MVP design implementation

- Carl can defeat ordinary goblins enjoyably with movement, jump, punch and **kick**, including understandable knockback/collisions; **never a grab**.
- Donut is useful and enjoyable in co-op from the first scene without magic, through mobility/positioning and modest catlike physical actions.
- The biscuit changes Donut's primary attack to Magic Missile without rebinding basic controls; Carl's grammar remains unchanged.
- Destructible props and enemy impacts are deliberate, readable, bounded and reproducible; no gameplay-authoritative ragdoll or infinite collision chains.
- Story-locked explosives / spells cannot be used prematurely. Specials use the same Circle/B slot when acquired.
- All primary actions are playable without a second stick, gyro, trackpad or shoulder paddles; D-pad movement is supported.
- Solo swapping / companion authority and true two-pad co-op are verified, including disconnect/rejoin and no cross-player input.
- Native 52/40px actors and camera remain readable at both 640x360 and 640x400; no terrain softlock.
- Review actual controller footage and hands-on game feel. Passing CI is **not** proof of a satisfying control scheme.

**Non-goals:** gameplay implementation, fully balanced ability trees, specific post-Floor-01 skills, final Puddle Jumper tuning, Steamworks integration, finalized Shoulder Mode, shipping/licensing approval.
