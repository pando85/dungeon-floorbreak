# Floor 01 — Pre-magic Donut: cooperative gameplay experiments (design v0.1)

**Status:** proposals for a *first* directly-playable Donut experiment; NOT a locked moveset, book-canon mechanics, an implemented Bevy feature, or proof of enjoyment.
**Owner-established constraints:** Carl and Donut are playable **from minute one** in 2P local co-op; solo uses hybrid direct control + companion AI + quick context commands. Shoulder Mode is **brainstorming**, including possible simplified mounted attacks **after** suitable story unlocks. Approved sprite heights: Carl 52 opaque px; Donut 40 opaque px. These identities may not be resized to accommodate camera/framing.

Read [Floor 1 gameplay progression](floor-01-gameplay-progression-v0.md), [core gameplay](gameplay.md), [antihero tone](antihero-tone.md), [input and camera](../03-tech/input-camera.md), [Floor 1 atlas](../02-art/floor-01-art-atlas-v1.md), [integrated-scene playtest](../07-agents/floor-01-integrated-scene-playtest.md) and [source/right guardrails](../04-production/rights-and-release.md).

## Design problem and core hypothesis

Pre-Tutorial-Guild Donut is **not yet a ranged caster**. Giving player two a magical kit early would flatten her story progression. Making her a passive companion would violate the first-minute co-op promise.

**Hypothesis:** Donut is satisfying as a fragile, agile, physically plausible cat whose *position*, *timing* and *distraction* change the tactical situation. She can survive a dangerous scene and create opportunities for Carl without dealing large damage, planning military maneuvers, or operating implausibly human machinery.

The prototype must distinguish:
- **Cat-shaped fiction:** movement, jumping, light swats, a startle/hiss/yowl, knocking **very small** unstable items, optional climb/perch where authored.
- **Game adaptation (unapproved):** predictable input timing, enemy response to noise, an optional short cat-sized shortcut, context interaction.
- **Not pre-Guild:** missile spells, controlled freeze/displacement, formal companion orders, invented inventory skills, combat-class role.

### Minimal candidate action set (not a final unlock table)

| Action | Proposed Donut input | Player choice | Counterweight / risk |
| --- | --- | --- | --- |
| **Scamper** | left stick / D-pad | Escape or reposition quickly, able to reverse promptly | Low tolerance for body contact |
| **Jump / perch** | South | Take a higher but narrow authored ledge; avoid low obstacles | Surface reach and ceiling/hitbox must be tested at approved 40px silhouette |
| **Swat** | West | Nudge a light prop or fend off a close nuisance briefly | Low damage and short range; does not replace Carl's strikes |
| **Hiss / yowl (test-only distraction)** | East | Make one nearby suitable foe hesitate/look or redirect attention | Must have a readable tell, limited frequency and **no guaranteed taunt of machines/bosses** |
| **Interact / mount request** | North while at an authored object / near Carl | Switch between two route/coordination options | No invisible lever-operating cat; show target and resolve competing proximity cases |
| **Dismount** | North or equivalent when mounted | Gain independent movement and agency again | Cannot trap player on or below unreachable geometry |

This is deliberately **fewer active combat actions** than post-Guild Donut. Dodge may be movement-derived at first; don't add a second dodge button unless tests show a real need. Avoid three-button combos.

**Carl comparison:** move/jump, a deliberate close-range strike or kick, and physical manipulation of a larger obstacle; he is ordinary, slightly awkward, and not able to solve all threats by direct fighting. Destructive explosives are not a default opening-spawn ability.

## Solo and 2P modes: agency invariants

- **2P:** P1 controls Carl and P2 controls Donut from first frame. P2 always has a useful positional choice; no unskippable spectator escort state while Carl drives the action.
- **1P:** control either hero and instantly switch. Untouched actor follows/responds to authored danger using a modest, deterministic AI. Hiss/knock interactions can be reached with a **single contextual command** when appropriate; never require both players to issue coordinated inputs at a precise tick for the only way forward.
- **When Donut is the active solo character:** Carl's AI can follow and protect an escape route but must not steal every interaction/kill; same critical path remains completable.
- **Co-op progression:** same room design for 1P/2P; pressure can be tuned after evidence but do not simply double enemy health.
- **No P2-only required gates** or "cat needs to push a heavy switch" fiction. Narrow passages are **optional alternatives**, not required story gates.
- **On camera separation:** both should remain meaningfully controllable and visible. Warning/regroup behavior is future test work, not implicit teleport approval.

## Three controlled micro-scenarios (authored *test fixtures*, not confirmed book maps)

Use the existing Floor 1 catacomb tiles, approved Carl/Donut sprite masters and Bevy A1 lighting; new interactive props should have simple original editable pixel sources. A neutral toy enemy is acceptable in a mechanical spike, but never present it as a canonical new monster.

### E1 — The narrow way (30–60 s target; movement / role distinction)

**Layout:** one common safe floor and one optional short cat-sized side opening above a low step. A small hanging object or fragile light prop is positioned so the cat can swat it; Carl can interact with a heavier loose barrier on the main path.

- **Unassisted solution:** both can take the main safe route; no mandatory co-op timing.
- **Co-op alternative:** Donut uses the cat-only route and a light interaction to create a shortcut/early warning; Carl moves or braces a physical object.
- **Probe:** Is Donut fun to navigate without fighting? Can she meaningfully contribute? Does the camera allow temporary separation without hiding collision?
- **Failure signature:** shortcut saves no noticeable effort, or P2 becomes invisible while helping; cat sprite clips through the whole wall because hitbox was guessed from fluff.

### E2 — Do not fight the machine (45–90 s target; escape / pressure)

**Layout:** clearly telegraphed incoming heavy machine, one safe escape trajectory and a turn where vehicle tracking is imperfect. Optional tiny prop/noise interaction and a later on-foot nuisance may offer Donut a short tactical opportunity.

- **Baseline:** running and smart positioning suffice in both modes. The Dozer is **not** defeated by Donut's swat, hiss, or invented magic.
- **Carl alternative:** physical obstruction or route selection, not a mandatory explosive.
- **Donut alternative:** route scouting or a very modest distraction of a **suitable on-foot enemy**, never a guaranteed taunt affecting a machine.
- **Probe:** Does P2 feel actively involved while escaping? Can both players see where to go? Is switching fun or necessary only because the AI fails?
- **Failure signature:** insta-death hidden by camera, Dozer turning perfectly despite established design, escape relies on P2 landing an arbitrary forced taunt.

### E3 — Regroup and refuge (30–60 s; camera / optional shoulder)

**Layout:** corridor closes toward a safe room entrance. Both players are given an optional chance to travel together or separately. A tiny low obstruction may test mounted silhouette/hurtbox *only in C1 later*.

- **Baseline (B-stage):** both freely enter Guild; solo AI follows; no mandatory mounting.
- **C1 controlled variant:** Donut can **request** to mount Carl and dismount any time before safe zone. P2 retains look/interaction agency; avoid coercing the second player into spectator mode.
- **C2 future post-unlock variant:** in a separate fixture **after a justified story unlock**, test aiming/casting from the shoulder vs casting unmounted. **Not part of this opening run.**
- **Probe:** Is mounting a meaningful convenience? Is separation still worth choosing? Is the duo readable at 52/40px when stacked?
- **Failure signature:** mounted Donut too big/obscured, P2 has nothing to do, camera uses forced zoom-out, solo swapping produces a stuck actor.

The tests do **not** prescribe a novel-accurate physical map. They explore movement/roles while allowing the story scene to be authored separately from the greybox experiment.

## Shoulder Mode: practical rules to test, not adopt yet

**Comparison variants:**
- **C0:** normal independent actors;
- **C1:** mount/dismount and shared position, **transport only**, no early magic;
- **C2:** mounted projectile/aim simplification with powers already earned later in the floor;
- **C3:** special combo kept as a **future idea**, not part of the first test.

**Input proposal:**
- In 2P, **Donut initiates the mount request** while close to Carl; Carl may accept/refuse via simple context interaction, or prototype a short automatic consenting posture if two-button consent proves clumsy. Never steal authority from either player.
- While mounted, Carl controls locomotion. Donut retains independent facing/aim/cue/exit request; **do not assign her a simulated sword or magically unlocked spell before Guild**.
- In 1P, a single contextual command may request mount/dismount; AI should not spontaneously change mounting state during a dangerous platform jump unless a safe authored rule says so.
- Mounted position follows an explicit animation/socket and **new gameplay colliders**, not hair/coat pixel extent. Keep camera tracking predictable.
- A mount cannot become a single uncounterable superior combat mode. For the first experiment assess it as an **accessibility and coordination affordance**, then see whether C2 has real room in later gameplay.

**Open questions for user research:** Is the mount trigger too slow during chase? Is two-person consent fun or irritating? How will knockback/fall cause a dismount? Is cat facing independent of Carl? Will mounted aim simplify one-player controls without forcing constant micromanagement? How does a scene teach dismounting without a wall of text?

## Minimal implementation and test handoff

**Prerequisite:** Stage A human feedback, [WORK-005](../04-production/backlog.md) independent directly controlled Donut and two controller slots, a viable shared duo camera and modest solo follow/switch architecture. No Shoulder Mode coding before those baselines exist.

Small independent implementation steps:
1. **DUO-INPUT-01:** independent Donut controller + physics and stable device authority. One PR; no additional weapons, skills or mount.
2. **DUO-CAMERA-01:** both actors share authored constraints and camera framing at 640x360/640x400. Test opposite directions, reunions, hard-edged collision.
3. **DUO-FEEL-01:** E1 neutral fixture with cat jump/swat and Carl heavy interaction. Compare 1P and 2P; do not call it lore-canon content.
4. **DUO-ESCAPE-01:** E2 escape fixture; only then add a simple if-supported hiss/distract trial. If distraction adds nothing or is thematically implausible, remove it.
5. **SHOULDER-SPIKE-01:** E3, C0 vs C1 baseline; feature-flagged. C2 only after independently validated post-Guild combat.

**For each PR:** Rust + Bevy only, pure deterministic fixed-step gameplay/actions, headless replay and state hash, 2P physical input ownership test and disconnect, collider/camera bounds, actual Bevy screenshots with approved sprites at both virtual resolutions, controller playtest notes. AI/scene adapter must not infer authority from P1's hardcoded device ID. Do not copy AI concept images into native sprites.

**Human decision gate:** use [the playtest worksheet](../07-agents/floor-01-integrated-scene-playtest.md). In each test record:
- P2 agency (did the Donut player choose a genuinely useful action?);
- the moments where solo AI gave help vs needed rescuing;
- how often switching was *voluntary tactical play* vs forced micromanagement;
- whether split travel, mount travel or solo commands felt preferable;
- camera issues, irreversible softlocks, hitbox/visibility issues;
- at least three concrete observations plus one behavior to preserve.

This defines the **next experiments**, not a commitment to a final pre-magic powerset or shoulder-based special attack.
