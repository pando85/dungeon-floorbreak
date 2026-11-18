# Floor 01 — Goblin defeat language and finishing reactions (design audit v0.1)

> **Direction update — 2026-11-18, owner feedback:** The original narrow four-finish audit below is an *implementation inventory*, not the intended overall violence level. The owner wants **body explosions, decapitations, dismemberment and outrageous monster-specific deaths**. A [universal monster death language](universal-monster-death-language-v0.md) is now proposed for review. It explicitly permits violent finishes with shared art/physics semantics while preserving each monster's identity. **Do not freeze GOB-02-CRUSH as the complete death-art plan or claim the universal design is already approved.**

**Status: proposed gameplay/art extension, NOT owner-approved final death catalogue or a playable implementation.** Prepared 2026-11-18 for the first I-03 post-Dozer goblin encounter. The intention is to identify **what can be tested with existing independently reviewed native art (owner gameplay/visual acceptance still pending)**, and isolate the **small genuinely missing art** rather than drawing a whole new goblin death spritesheet before the game feel is tested.

**Design aim:** "Kill, kill, kill" through satisfying, expressive *cause-and-effect*. A simple punch kill must feel good; the surprising deaths result from player-controlled kick/position/terrain chains. Keep the 1980s–1990s scrappy-action / nasty dark-comedy tone; readable, characterful impact, not a wall of particles, noisy gore, automatic fatalities or compulsory gimmicks.

Read: [Act I first encounter PR #78](https://github.com/pando85/dungeon-floorbreak/pull/78), [integrated combat probes](../07-agents/floor-01-integrated-combat-probes.md), [GOB-01 goblin spec](../02-art/gob-01-goblin-spec.md), [GOB-01 visual evidence](../evidence/gob-01/README.md), [Carl melee spec](../02-art/carl-melee-anim-02-spec.md), [ART-INT-02 real Bevy captures](../evidence/art-int-02/README.md), [ENV-02 existing prop](../02-art/env-02-opening-spec.md), [approved controller grammar](mvp-controller-contract.md).

## Inventory: what is already designed versus what is missing

GOB-01 **already has 27 native frames per variant (A/B), 64×56 canvas and 36px tall goblin**, with tags including `hurt` (16–17), `knockback` (18–19), `down` (20–22), `getup` (23–24), `corpse` (25), and `knockback_contact` (26). A/B production source, editable grids/.aseprite, tests, manifests and review GIFs are committed; ART-INT-02 already uses direct hit/flight/down/corpse in Bevy combat fixtures. **Important limitation:** `knockback_contact` and `getup` are mapped/tested in presentation but the CP-01 simulation does not yet generate those states; no GOB-01 frame specifically depicts a rock-crushed enemy.

| Finish/reaction | Proposed look and gameplay meaning | Art readiness | Bevy/gameplay readiness | First I-03? |
| --- | --- | --- | --- | --- |
| **D-01 Direct punch / haymaker** | Visible face/torso recoil, short impact freeze, collapse/flight if lethal, corpse | **Existing** `hurt`, `knockback`, `down`, `corpse`; Carl punch art exists | **Existing CP-01 / ART-INT-02 fixture**, needs owner game-feel validation in story route | **Yes, P0** |
| **D-02 Kick launch / ground impact** | Carl's shoe visibly connects, goblin flies with consistent direction, hits ground and stays down | **Existing** Carl `kick`; goblin `knockback`, `down`, `corpse` | Kick already pictured in CP-01 fixture; input currently temporarily `special_pressed` (East), **must move to PhysicalAttack (North)**; world/landing tuning unapproved | **Yes, P0** |
| **D-03 Flying goblin collides with another** | Crumpled impact pose on first goblin; second recoils/staggers; later direct kill may follow | **Existing** `knockback_contact` frame 26 and second goblin `hurt` | **CP-02 NOT yet implemented as a real gameplay contact**; begin with **stagger/position only, zero contact damage**. A collision here is not automatically a death | **Yes, P0 experiment** |
| **D-04 Stone crush / terrain kill** | Rock lands; brief collision compression with recognizable ears/limbs/weapon; dust and visible aftermath | **Missing dedicated goblin-crush pose/effect**, though ENV-02 rock, support states and dust VFX already exist | **CP-03 NOT integrated**; use existing `hazard_fall_region` for one deterministic landing impact, attribution and exit safety | **Yes, P1, art extension needed** |
| **D-05 Wall slam** | Launched goblin impacts masonry; short debris/contact beat; falls | Standard airborne/down can be reused; no unique wall-hit polish | Distinct wall/contact presentation/damage rules not proven | **Optional later** |
| **D-06 Explosion** | Blast recoil + smoke/fire cue; distinct from blunt force | No approved dedicated explosion-defeat art for opening goblins | **Not available before story-earned explosives**; do not add opening bombs | **Later, outside I-03** |
| **D-07 Magic Missile defeat** | Clear magical impact and enemy physical reaction | Basic hurt/down can be reused; unique spell finish VFX not designed here | Donut has **no magic before biscuit** | **Post-Guild only** |
| **D-08 Defeated-body projectile** | A corpse might collide once before going cosmetic | Current `corpse` exists; no extra art required for research | CP-04 explicitly **optional experiment**. Default: defeated bodies are noncolliding cosmetics | **Not MVP gate** |

**No need for eight separate full death animations.** Reuse grounded/airborne/down/corpse clips and select a distinct *cause-specific transition + impact effect* for meaningful reasons. Confirm input, character, attack and source causality in the domain; do not classify death solely from the final sprite/tag.

## First-pass art composition: D-04 stone crush only

**Proposed, not approved:** a minimal additive original-pixel extension named **GOB-02-CRUSH** for both existing GOB-01 variants, *without editing or invalidating their frozen 27-frame source/key fixtures*.

1. **One `crush_contact` key per goblin variant:** a brief squash/tuck reaction framed **at the moment of rock contact**, with startled expression and collapsing limbs; the upper part may be occluded naturally by the falling block. Distinct silhouette at 1×; do not make generic red gore particles the only visual identification.
2. **One `crush_remains` key per variant:** crushed, nonblocking silhouette partially visible beside/under the landed block (ear, boot, pineapple stick or similar small shape). Keep it readable but avoid cartoon-paper-flat distortion that breaks the approved body identity.
3. **Optional 1–2 transition in-betweens** only if reviewing the impact animation at 60 Hz reveals an obvious pop. Do not freeze a frame count from text alone; compare at real game speed.
4. **Existing ENV-02 VFX:** reuse authored block impact dust and rubble. A brief directional contact squash and one low-frequency impact accent are candidates; the true collision/kill is one sim event independent of visuals/audio.
5. **Technical compatibility:** preserve GOB-01 64×56 frame convention and original palettes, pivot/foot / no-pixel sink semantics, A/B variant distinction, existing hit/hurt tests; if the corpse artwork protrudes beyond that canvas, create a narrowly scoped **separate overlay** with explicit world anchor. No runtime scaling or image-model cutout.
6. **Temporal readability:** impact event `T` controls contact pose, dust and kill confirmation. The block must visibly touch the goblin before death animation/vanishing, and the corpse must not obscure the Guild exit or re-enter authoritative collision.

**Art pipeline:** top-tier design (1× key silhouettes + original pose design) → medium-tier editable grid/.ase source and export/validator/evidence → independent top-tier art review → owner native-scale/Bevy visual review. No need to solicit more conceptual images until the small native-scale extension proves unsatisfactory; production should be original authored pixel art, not a downscaled illustration. Sound should emphasize differentiated **thud** (punch), **shoe impact + brief rush** (kick), **two-body contact** (CP-02) and **stone/dust impact** (CP-03) without continuous shouting or overly metallic synthetic punches; sound design itself remains a separate review.

## Game rules that should determine animation selection

- **D-01 / D-02:** direct melee kills share ordinary `hurt -> launch/down -> corpse`, with their difference principally conveyed by attacker pose, velocity and timing. Do not duplicate full motion sets only to label the attack.
- **D-03:** the first launched living enemy reaches another living enemy and triggers `knockback_contact` and a clear second-goblin reaction **at most once per launch/contact pair**. Phase A has zero collision damage; assess optional damage only after playtesting.
- **D-04:** the authored block landing produces **one collision event for each overlapping victim**, after visible descent; goblins in the hazard footprint at contact may die, and the destroyer of the support gets credit. Outside footprint or with an unbroken support, **no kill**. Player inside footprint may suffer a readable hit as a separate tuning decision. If a basic corpse/down clip is temporarily reused pending GOB-02-CRUSH, it must be labeled **temporary** in playtest evidence.
- **Death attribution:** track direct attacker / launched enemy source / prop breaker as a stable causal chain. Replay with the same seed/input must produce identical contact order and defeat results. No presentation-driven death logic, no infinite chain damage or physics-based fatality outside authored semantics.
- **Progression:** the fight is winnable with ordinary attacks. Rock kill and kicked-body contact are bonus opportunities, never required puzzle solutions; Guild exit/camera/traversal remain safe.

## What needs human evaluation

1. Do D-01 and D-02 already feel good with existing 27-frame sprites, or is the problem audio/timing/camera rather than art?
2. Does D-03 produce a strong *apparent* impact when no collision damage is applied, or do we need a narrowly bounded alternative?
3. Does the rock D-04 event register clearly with existing dust and a generic down pose? Is GOB-02-CRUSH worth authoring before broad rollout?
4. Can a player cause all three outcomes by play choices in **the same integrated scene**, with no forced choreography?
5. Can two people see the results in 2P and can solo replay them once those actor-control systems exist? Mark missing modes **pending**, not passed.

**Review/approval:** owner approves or revises the small cause-specific death language after inspecting **real Bevy 640×360 and 640×400 captures**, a short full-speed clip (not only slow x3 GIFs), and an interactive run. Approving an art reference is not approval of collision feel, tuning or implementation. Do not label this catalogue `FINAL` until that test.
