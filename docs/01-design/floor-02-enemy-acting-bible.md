# F02-ACT-01 — Enemy identity, gameplay acting and animation bible

**Status:** original enemy design candidates for THE CRAWL, 2026-11-22; not implemented, artist-approved or human-tested. Read with the [F02 game bible](floor-02-last-lesson-design-bible.md), [Floor 01 combat-first direction](combat-first-encounter-design.md), [global art direction](../02-art/art-direction.md), [native pixel animation schema](../02-art/animation.md), [asset toolkit](../02-art/asset-pipeline.md) and [pixel art routing gates](../07-agents/pixel-art-pipeline.md).

## North star

**Before an attack, silhouette communicates intention. On a hit, the enemy responds to actual force. On defeat, the result reflects what truly happened.**

An enemy's personality is *how it enters, commits, fails, recovers and reacts*, not a speech bubble or longer HP bar. Gameplay readability wins over lore costume details. Dark industrial 1980s/1990s humor, expressive large pixel art, approved protagonist identities. Use 6–8 keyframed run frames as a starting hypothesis only when the family needs it, 3–8 key combat frames, and 1–3 expressive recovery/defeat keys, following the native animation JSON event markers and fixed-tick authoritative attack contracts. No sprite frame can independently cause damage unless its matching authoritative attack state says so.

**Shared state graph, adapt the current canonical modules:** Enter -> Approach/Reposition -> Anticipate (distinct tell) -> AttackCommitted (direction/target chosen) -> Active (bounded contact) -> Recovery -> Reposition; interrupts only before commitment unless encounter explicitly owns a stagger window. Separate HitReact / Launched / Collided / Defeated (with causal owner) / Reset. A stunned enemy should visibly lose its composure; avoid a universal red flash as its sole reaction.

## Family A — Mind Horror: the hovering mind that loses its nerve

**Narrative silhouette:** an unsettling *floating brain with tentacles* (secondary lore references support floating brain form); unlike a common humanoid, its danger is spatial and psychological. The pale, wrinkled cranial mass hangs behind slender flexible arms. Do not design generic armored mage. A low hovering shadow communicates that the main body can be hit with normal attacks.

**Personality through acting:** curious forward tilt -> tentacles pull tightly inward during its inhaling directional psychic aim -> an abrupt pulsing cone -> exhausted sag and tremor. When struck during the long windup, the brain briefly squashes against a queue-rail and loses focus; this is physical comedy *within* a fair stagger.

**Attack:** one slow directed lane with a distinct crown/eye contraction. No input inversion, full-screen blur or UI obscuration. Normal body recovery allows Carl's kick and Donut's missile to hurt it. Attack target must be selected fairly for either human/AI-controlled protagonist. Initial visual concept height candidate 40–60 native pixels including tendrils (not a global collision bound).

**Animations:** float idle (bob only visual), anticipate/inward coil, projected pulse, exposed droop, interrupt recoil, launched/body impact, defeat collapse. The telegraph direction must be apparent with VFX disabled except essential hitbox debug.

## Family B — Clurichaun sales rep: combat while pitching

**Narrative silhouette:** an aggressively cheerful tiny fairy moonshiner wearing a cheap fake corporate sales sash with a slingshot. Wide flaring gesture, a jug sample strapped to the side. Avoid a real company mark or recognizable national stereotype. Secondary lore places them in Rev-Up business.

**Personality:** steps into combat still gesturing at a sale; slingshot recoil flings its own arm back; after being kicked into the product rack it briefly reaches for an intact bottle, then loses balance. A fleeing salesman turns to speak for half a beat and exposes itself to a tactical punish, **without** an invulnerable comic pause.

**Attack:** committed horizontal/slanted slingshot, 24–36 fixed-tick candidate warning; a short reposition. Its defining weakness is being launched physically into nearby props. Never an impossible airborne sniper.

**Animations:** salesman idle/gesticulate, run, aim/charge with exposed sling, release, recoil, stagger/body-flight, futile reach, hit wall, defeat. A short in-world label or bobbing pennant could explain its affiliation without text-heavy banter.

## Family C — Laminak elite: the boss who believes the pitch

**Canon-sensitive correction:** secondary sources place **Laminak Rev-Up Consultant Elites** as superiors to the Clurichauns. Earlier F02 designs used a generic foreman/pitchman. **Prefer replacing that invented species with a Laminak elite visual/body** while *retaining the foreman gameplay role*. Do not add both and inflate the roster. Lore sources describe a small fairy and conservative business attire; verify details against the primary book before declaring canon.

**Silhouette:** a small, self-important flyer with a rigid angular business outfit, oversized badge and angular warning pointer, distinct from short ground sales reps. Comic attitude from posture and hovering stability, not parody of a real individual. Stage it close enough to ground and within Carl's reach or reachable by player-caused collisions; no untouchable permanent air turret.

**Personality:** repeatedly straightens its jacket as the showroom burns; points dramatically toward a warning bell; furious at employees who fail to make the sales pitch. On impact it spirals through debris with very unexecutive body language.

**Attack/utility:** alarm/signal with long 48–72 tick candidate interrupt window; single finite reinforcement attempt. Optional small ranged flick, but **avoid stacking projectile spam with the Clurichaun in the same pocket**. Basic physical/magic damage and actual interruption. No huge HP or boss immunity.

**Animations:** strict idle, hover drift, bossy pointing tell, alarm active, interrupted finger wobble, knocked-flight, furious recovery, defeat. Show its current attack intent with pose even when the alarm UI has no tooltip.

## Family D — Krakaren Clone: a workplace of tentacles, not an HP sponge

**Silhouette:** an enormous *occluded* corporate-daïs monster; one readable attack body/core plus 2–3 thick tentacle shapes. The distant appendages are scenic until their own authoritative telegraph lane is armed. No giant invisible hurting aura. This is a clone, not Prime.

**Personality:** attempts to continue a staged presentation while tentacles involuntarily pulverize the set; pressure rises through increasingly frantic violent movement. First reveal should be absurd, second attack genuinely scary. The boss must still be expressive when a fully physical support/jug strike staggers the body.

**Attacks:** committed slam / sweeping limb / pull-zone. Unique key silhouettes and recovery body opening; direct attacks from Carl and Donut work, as does optional capped environmental damage. No mandatory necromancy summons or forced environmental finisher.

**Animation keys:** podium-screen intact -> reveal/breach -> slam anticipates (high limb), sweep anticipates (low extended limb), pull (radial curled limbs), body exposed/exhausted, violently interrupted by falling sign, actual defeat, capped grub surge. Boss gets mandatory top-tier fully authored key drawings and independent review.

## Family E — Brindle Grub cluster and Vespa: cleanup made threatening

**Brindle Grub silhouette:** small, visibly legless, cat-sized alien janitor grub (secondary lore description; pixel scale adapt). The local game ecology should simulate a **few clusters** around corpses, not one rigid-body for every visible grub. Movement is patient and focused on cleanup rather than intentionally seeking a hero until a clear escalation flag. On hit a soft compress/rebound reaction; launched cluster physically slides toward machinery. No corpse-farming snowball, endless reinforcements or arbitrary kill punishment.

**Brindled Vespa silhouette:** sharply defined large wing shape and dangling acid sac. Rework the natural life-cycle transition as a presentation illusion with a **bounded scripted pupa key** and at most one adult in the first S04 pilot. Its acid spit is a real telegraphed trajectory, distinct from a mind-horror psychic lane and existing Floor 01 llama projectile. The adult's recovery puts it within either protagonist's line of fire; a conveyor bin may be an optional physical finisher.

**Animations:** grub munch / tremble / collision squish / recover, cluster body recoil; pupa swelling and rupture as **inert authored cutaway**, not uncontrolled spawn; Vespa hover, wing-pause before spit, projectile drop, dive recovery, collision with bin, defeat. No gross overlay across the entire screen; use small material effects.

## Family F — Mounted Kobold / Danger Dingo: two things in one silhouette

**Silhouette:** rider with absurdly rigid spear on a very fast metallic-looking dingo profile; when separated they must become **two recognizable moving bodies** with distinct contact owners. Exact metallic nature/coat is a research detail to verify, not a license for a robot dog retrofit.

**Personality:** kobold salutes a cruel arena protocol, overbalances as dingo commits, then pinwheels when launched. The dingo continues momentum because the rider fell off; the dog's movement must make it feel like a living creature, not a disposable projectile.

**Attack:** rider commits visible horizontal spear line with mount charge; hit before commitment can dismount or delay; hit after commitment yields real mounted momentum response. The direct-only solution can fight rider and mount normally; optional kennel release switches a room-local allegiance state. The dingo is not a third playable hero or persistent follower.

**Animations:** mount paired idle/run, rider lean/point spear, charge, hit/dismount, separate tumble, mount free stand/reconsider, optional ally stance, combat defeat. QA must inspect source ownership/retargeting through dissociation, including solo companion.

## Family G — Ralph: a very small catastrophe

**Silhouette:** genuinely tiny gerbil close to floor; the attack warning is much bigger than its hurtbox. Tiny ears/eyes, scrabbling feet, long readable dust direction, a wildly disproportionate steel cage and twin searchlights supply scale. Prefer ~16–24 native px hypothesis and actual real-speed contact checks. Never put a 90px invisible hurtbox around him.

**Personality:** attacks with preposterous conviction, crashes into the ring rail, is dizzy for a split second, resumes being incandescently furious. After a miss he looks almost surprised. The player must laugh at the entrance and fear his **committed**, readable speed, not a randomly teleporting low target.

**Attacks:** committed ground rush with direction locked, an obvious two-step feint followed by second tell, and a recoverable bonk. Every meaningful recovery accepts either Carl kick or Donut pounce/missile; direct victory always available. Optional dog interruption cannot win on its own. Art must not cover collision uncertainty with enormous dust.

**Animations:** tiny intro/cage escape, braced paws, scratch direction tell, 2–3 frame horizontal velocity smear, wall bonk, wobble, second feint, open recovery, legitimate hero-caused defeat. Separate animated dust direction marker from gameplay state for testing without VFX.

## Family H — Rage Elemental: an unbeatable performance

Not a boss actor in the conventional HP template. The art lives partly *behind* the play field: violet-black negative-space mass, huge glimpsed limb, eye-light, deformed fluorescents, distant structural movement. This builds scale without shrinking the protagonists or inventing an impossible full-screen hitbox.

**Acting:** the reveal barely moves, then a dramatic directional weight shift makes its next charge predictable. After committed overshoot, the creature takes a moment to recenter; it never reacts to normal damage as if losing life. In refuge it scratches/strikes the barrier without harming the party; at the stairwell it visibly goes across an authored exile region.

**Animation keys:** impossible shadow silhouette, one attack tell, committed lunge, angry collision against refuge, step-over/stair expulsion, silence. It is a flight threat, NOT a source of XP or normal kill slow-mo. Stop at one attack lane at a time.

## Distinction tests: can users identify the threat before reading the label?

| Pair | Silhouette-level separation | Threat-level separation |
| --- | --- | --- |
| Mind Horror vs Vespa | floating cranial folds vs horizontal pointed wing | aimed psychic pressure vs descending acid spit |
| Clurichaun vs Laminak | hunched ground seller vs rigid flying executive | committed slingshot vs interruptible alarm |
| Grub vs Ralph | slow thick grub shape vs tiny straight-line streak | ecology/cleanup vs fast committed boss charge |
| Dingo vs Ralph | large four-legged runner with rider or free stance vs tiny foot-level gerbil | mounted spear/charge vs small fast feint |
| Krakaren vs Rage | visible core and grounded attack limbs vs huge negative-space shadow | defeatable boss recovery vs explicitly invulnerable escape |

At 1x native render, a player should distinguish the five most common warnings: red physical dash, directional psychic pulse, high acid arc, falling heavy prop and Rage pursuit. Do not assign orange lichen to mean damage; hazard red must be distinct.

## Animation pipeline and production cost control

For each enemy deliver:
1. A **32–96px-class native silhouette hypothesis** matching its role (Ralph smaller, clone composited; do not override game dimensions), two-color readability proof, at 640x360 and 640x400 with **approved** Carl and Donut sprites beside it.
2. A single expressive hero key and two *honest* attack/hit keys. Bosses use **top-tier full key-pose art** and independent review; novel medium/hard families follow current B + silhouette gate; simple variants follow C. No speculative third-party media asset reuse.
3. Tagged original source frames with pivot, scale, provenance, attack anticipation/active/recovery metadata and atlas export checks. Movement/attack state controls hits; animation can only visualize them.
4. **Black-box test** for telegraph visible before active damage and recovery at actual game speed; lossless 1x/2x contact sheets, not just an upscale marketing render.
5. Fixed-seed replay plus at least one forceful Carl physical collision and one Donut contact test; avoid turning a victim into a death animation that plays without real impact.

**Recommended priority:** Clurichaun + Laminak + jug props -> Krakaren key set -> Mind Horror -> Grub/Vespa -> Kobold/Dingo -> Ralph collision prototype then native keys -> Rage shadow keys. Ralph/Rage can receive top-tier **design** keys early in parallel, but should not block the S02+S03 playable gate.

## Acceptance IDs

ACT-01: five threat silhouettes identifiable when names and SFX hidden.
ACT-02: each live attack has one readable precommit key, actual committed direction and finite recovery.
ACT-03: all pose event markers correspond to real state; damage/score never generated by VFX alone.
ACT-04: real launched/collided reaction distinct from normal hurt and deterministic on retry.
ACT-05: Carl and Donut both can hit each ordinary family by accepted control verbs and story grants.
ACT-06: ranged/airborne roles never become permanently unreachable or cause repeated unavoidable offscreen attacks.
ACT-07: observer can tell a death caused by Carl, Donut, other enemy, prop or hazard in replay metadata.
ACT-08: AI companion participates meaningfully rather than standing idle, without independently detonating risky props.
ACT-09: screenshot/video art review at native size and expected handheld distance; human verdict PENDING.
ACT-10: all editable sources original or appropriately licensed, manifest + checksum, independent art review PENDING.
