# THE CRAWL — Universal Monster Death Language (living design v0.1)

**Status: DIRECTIONAL PROPOSAL / FOR OWNER VISUAL + GAMEPLAY REVIEW, NOT APPROVED FATALITY MECHANICS.** Developed 2026-11-18 in response to the owner's request for **substantially more savage, explosive, dismembering, blackly comic deaths** that retain the *actual* repository character art. This document proposes a reusable grammar for **all monster species**, while deliberately leaving the individual artists freedom to exaggerate each death. It supersedes the **overly restrained design aspiration** in the I-03 Goblin Death audit only when this direction is explicitly approved; current tests, sprites, accepted artwork, four-button controls and combat physics remain unchanged.

**Premise:** THE CRAWL is not a game about enemies politely staggering and lying down. It's about **the player causing memorable, sometimes outrageous destruction of monsters**, with the gritty/absurd 1980s–90s action antihero and dark humor from the [antihero tone](antihero-tone.md). A first punch should already feel substantial; a Croc kick, improvised falling stone, environmental blade, dynamite or later magic can produce ever-more shocking results. The brutality escalates as encounter tools expand, **not** by adding execution QTEs or extra controller buttons.

**Primary artistic reference:** actual committed [Carl approved concept](../02-art/carl-character-bible.md) and [52px native melee sprites](../02-art/carl-melee-anim-02-spec.md); [GOB-01 36px common goblin 27 frames A/B](../02-art/gob-01-goblin-spec.md); [ENV-02 original falling block / shore](../02-art/env-02-opening-spec.md); original [Bevy integration evidence](../evidence/art-int-02/README.md). **Do not redraw Carl as a muscular booted fantasy warrior or replace green pineapple-club goblins with knife-wielding generic orcs.** Existing character/floor designs remain authoritative. Generative concept images are *acting/mood suggestions*, not reusable production sprite masters.

## 1. Five families of outcomes — a common grammar, not five identical animations

| Family | Conditions / illustrative causes | Readable resulting silhouette | Floor 01 usage |
| --- | --- | --- | --- |
| **KINETIC / BRUTAL IMPACT** | Fist, Croc kick, airborne enemy collision, wall/floor impact | Clear recoil, launch, violent landing, discarded weapon | **Initial encounter / CP-01/02** |
| **DISMEMBERMENT / DECAPITATION** | Extremely powerful or suitably directed hit; *sharp* environment or later appropriate equipment | Readable separate head or selected limbs, delayed fall, separated weapon | **Candidate targeted encounter after physically justified cause; not automatic for every punch** |
| **EXPLOSIVE / GIB** | Earned dynamite, traps, authored machinery detonation, later magic with equivalent force | Brief silhouette breakup followed by a *small, authored set* of flying body pieces, smoke, gear | **Only after an earned or environmental explosive source; not at the pre-Guild opening** |
| **CRUSH / ENVIRONMENTAL** | ENV-02 masonry block, traps, heavy moving props | Compressed silhouette, ears/boots/pineapple club remaining visible under stone; dust; flattened aftermath | **First encounter / CP-03**, can be dramatic without mandatory copious blood |
| **ELEMENTAL / SPECIAL** | Fire, electricity, acid, other lore-earned spells/biome dangers | Anatomy/material-specific readable transformation or distinctive aftermath | **Later Floor 01/future floors only when available** |

**Semantic distinction:** a launched goblin that hits another goblin can first cause *stagger*, and only later a kill if an **authoritative damage rule** makes it lethal. Do not depict a head popping off as a kill when gameplay only registered a non-lethal contact. Explosion/decapitation must not be random VFX attached to a weak swat unless explicitly designed as a rare special condition with a convincing cause.

## 2. The death grammar for all monster types

A single elimination is built from **five independent elements**:

1. **CAUSE** — punch, kick, projectile, bomb, falling stone, blade, acid, fall, other justified sources; tracks a causal credit chain to actor/environment.
2. **INTENSITY** — ordinary / heavy / catastrophic, informed by authored attack, impact speed, target resistance and environment; never rendered by invisible random thresholds alone.
3. **ANATOMY / MATERIAL PROFILE** — what the monster is: fleshy goblin, bony undead, gelatinous slime, armored creature, construct, large boss, etc. Only a profile declares which detachable parts and splatter materials exist.
4. **FINISH RECIPE** — chosen eligible reaction: thrown intact, decapitated, split apart, crushed, burnt, dissolved, fragmented, etc., with small controlled variation. Designers must be able to force a particular recipe in a deterministic QA fixture.
5. **AFTERMATH** — nonblocking remains, gear, material chips, blood/slime/dust, transient marks and a clear kill confirmation. Persistence/cleanup follows arena budget and route constraints.

This structure provides consistency **without identical results**. A rock crush on a goblin shows compressed limbs, ears and a broken pineapple club; on a skeleton it scatters bones; on a slime it splashes; on an armored creature it dents armor and ejects pieces. The *cause remains legible*, the *material differs*.

### Reusable species profile — illustrative fields, NOT final Rust API

```yaml
monster_death_profile:
  id: goblin_common_a
  material: organic_green
  size_class: small
  skeleton_style: flexible_cartoon
  detachable_parts: [head, arm_l, arm_r, leg_l, leg_r]
  equipment_parts: [pineapple_club, vest_scrap]
  finish_families: [kinetic, dismemberment, explosive, crush, elemental]
  baseline_pose_refs: [hurt, knockback, down, corpse, knockback_contact]
  gore_palette: goblin_green_dark_crimson
  signature_gag: "pineapple club takes a separate ridiculous trajectory"
  forbidden: [invisible_weapon_swap, human_red_skin, generic_orc_boots]
```

Not every enemy needs every family. A slime doesn't have a head to decapitate. A machine shouldn't bleed organic blood. Small monsters can reuse fragments; bosses can justify bespoke set pieces after their combat design is accepted.

## 3. The art-direction contract: savage, absurd, specific, readable

**What we WANT**

- **Real consequences:** heads can detach, monsters can explode into pieces, a heavy object can flatten bodies. Make the outcome visibly different from a routine `down` animation.
- **Cartoon physics with credible force:** visible anticipation -> **contact** -> a powerful 1–3 beat breakup/launch -> aftermath. Strong silhouette and readable timing matter more than depicting realistic anatomy.
- **Black comedy anchored in character:** an eye-bulging goblin notices danger too late; the pineapple club flies in an absurd direction; a grotesquely flattened outline remains beneath a stone. The joke is the awful situation, *not* a separate meme overlay.
- **Material-specific effects:** greenish flesh, small dark-red blood accents when appropriate, bone dust, armor scrap, greasy smoke, green slime. Avoid one identical explosion sprite on every species.
- **Escalation and surprise:** common hits often kill conventionally; contextual strong impacts/traps trigger special finishes. Make player-created emergent kills worth seeking and replaying.
- **Pixel-art fidelity:** respect original silhouettes and indexed palettes; evaluate both 640×360 and 640×400 at **1× virtual scale**, then 3× integer upscale. Keep player/remaining threats legible under dust and impact flashes.
- **Sound/camera restraint that enhances brutality:** unique short punch thump, shoe crack, body-against-body impact, stone bass crash, explosive transient; silence/recovery and brief screen emphasis. No constant exaggerated screaming or screen-filling flashes.

**What we DON'T WANT**

- Polite, nearly identical enemies simply falling with stars for every lethal cause.
- Hyperrealistic surgical anatomy, realistic viscera or endless red particle confetti that obscures Carl/Donut, hazards and telegraphs.
- A generic fantasy goblin with sword/boots, or a muscular Carl without his battered purple jacket, heart-patterned boxers and **pink Crocs**. No weapon/skill he has not yet acquired.
- Four independent 30-frame full-body sprite atlases per enemy death type, every projectile rendered with authoritative 2D physics, unbounded gib interactions or procedural destruction that breaks collision.
- Mandatory finishers, cinematic pauses every kill, QTE executions or a gory RNG lottery unrelated to player actions.

**Artistic freedom permitted:** within the readably correct anatomy/material/scale/facing and source character identity, the artist can exaggerate expressions, poses, fragment shape, splash arcs, accessory motion, camera emphasis and humorous aftermath. An artist can propose a *signature kill* for a monster without adding a global mechanical requirement. Evaluate strong alternatives at 1× gameplay scale, not only enlarged concept boards.

## 4. Efficient original-pixel authoring; *not* a new sprite atlas for each finish

For ordinary enemy species:
- Keep existing idle/move/attack/hurt/knockback/down/corpse sprites unchanged.
- Author **separable original-pixel pieces** only when needed: head key/silhouette, 2–4 limb/torso chunks, equipment fragment (goblin pineapple club), small stains/material splashes, optional single-frame impact squash.
- Author one or two **cause-specific transition keys** for distinctive finishes (e.g. decap-contact, crush-contact) and an aftermath silhouette (e.g. headless collapse, flat remains). Only add in-betweens if 60Hz playtest reveals jumps.
- Allow part motion to be parameterized with deterministic seeded **cosmetic** curves (flight, spin, bounce, fade); preserve pivot, palettes, native pixel grids and clipping. Small palette/style-preserving variation is fine; no interpolated/scaled-up fake sprites.
- GIB debris and detached body pieces are **presentation by default**. Real interactions with a flying live goblin or an exploding barrel are separate, authoritative, bounded events. A corpse chunk that visually hits an enemy **does not cause damage** unless later CP-04/other gameplay design is explicitly approved.
- Hard limits per instance for piece count, flight lifetime, screen occupancy, blood marks and sound simultaneity are set after GPU/CPU/camera playtest; **no numerical budgets are approved by this doc**.

**Optional accessibility/comfort direction:** investigate `full`, `reduced` and `off` gore presentation levels. These only change presentation — no altered hitboxes, damage, enemy AI, routes, timing or multiplayer replay hashes. In a reduced mode, a detached head can become a silhouette/cosmetic puff while the underlying defeat and knockback remain the same.

## 5. Gameplay-to-presentation pipeline (design, not code delivered)

```text
Deterministic simulation (Rust):
   Attack / Explosion / Terrain / Collision
     -> damage + resistance + state + actor/environment attribution
     -> EnemyDefeated {tick, target, cause} (the one authoritative defeat event of `src/combat.rs`;
        cause carries the credit; NO intensity, gore or recipe field: those are presentation-only; owner default: intensity preset 4 = full)
     -> eligible death recipe ID chosen from species data + deterministic event seed
Presentation (Bevy, read-only):
   original 1x poses + detached-parts sprites + VFX + camera/SFX -> cleanup
```

- **Authoritative side:** order of hits, kill causality, scripted terrain impact, limited material destruction, player/AI hitboxes, target defeat and any later damageable living-enemy launches. Reproduce exactly from input/seed.
- **Presentation side:** detached-part trajectories, blood/slime/smoke, screen flash/shake, temporary wall splashes, weapon fragments, corpse arrangement. Never feed presentation collision into damage or player traversal.
- An explosion must resolve damage **once** using the existing [destructible terrain](../03-tech/destruction.md) area/line-of-sight rules. Enemy defeat recipes can vary, but cannot invent new damage or manipulate the outcome of the sim.
- Preserve the [hybrid combat-physics contract](combat-feel-hybrid-physics.md): bounded cascades and once-per-launch contact events. No unbounded ragdoll simulation.
- **Priority when multiple causes coincide:** use actual ordered authoritative events; the recorded killing event owns the finish. Any cinematic compound effect (e.g. kicked goblin then rock crush) must follow the real event sequence; do not attach a decapitation to a non-lethal first hit.
- **Network/controller scope:** two controllers share the same deterministic domain state; any non-authoritative gore effect must not change controls or block input / checkpoint replays.

## 6. Floor 01 staged prototype plan, preserving narrative unlocks

| Gate | What to prove | Required art scope | Validation |
| --- | --- | --- | --- |
| **P0 — First goblin** | First ordinary punch and kick feel outstanding | Reuse GOB-01 `hurt / knockback / down / corpse` and Carl ANIM-CARL-02 | Actual CP-01 Bevy at full speed, native 1× |
| **P0 — Two living goblins** | Kick launches goblin A into goblin B; readable reaction without premature automatic contact damage | Reuse GOB-01 `knockback_contact` + `hurt` | CP-02 deterministic replay; owner sees cinematic contact |
| **P1 — Rock kill** | Goblin visibly crushed by authored falling block; hit attributed correctly | One small `crush_contact`, `crush_remains` key family + existing block/dust; show multiple *brutality* concepts | CP-03 footprint/landing tests + art comparison |
| **P1 — Decapitation showcase** | One **causally justified** cutting/trap/especially heavy contextual encounter creates a recognizable detached head and headless collapse | `head_detached` plus 1 aftermath pose, optional material effect | New isolated, deterministic test; no pre-Guild fictional sharp weapon / no requirement to rewrite first fight |
| **P1/P2 — First explosion into parts** | After Carl has **actually acquired** explosives, a direct authored detonation can separate 2–5 major readable pieces and send pineapple weapon flying | Head/limb/torso equipment fragments, small material VFX | Explosion damage once, deterministic outcome + cosmetic parts footage |
| **Later** | Fire, acid, magic, unique monsters and bosses | Specific material/profile signature | Separate approved design and tests |

**Critical pacing:** the first goblin need not always be decapitated or exploded. Teach punch -> kick -> collision -> stone first, then introduce increasingly brutal ways of finishing as Carl earns explosive tools and reaches dangerous machinery. The global language applies already; the gameplay does not grant bombs or swords before story events.

## 7. Concept-art brief for the next visual pass

**Use the first prior 4-row/3-stage concept-board composition as the favored presentation, but replace the prior generic characters with the exact project masters.**

- Character sources: Carl **short messy brown hair, real-world imperfect body, battered purple-dark jacket, white/red-heart boxers, bright pink Crocs**, no big muscles, sword, boots or shotgun. Goblin A/B **36px green narrow figure, long ears and long crooked nose, brown vest, bizarre pineapple-on-a-stick weapon**, B variant keeps its bandana/tooth/accessory identity. Environment: ENV-02 warm brick and cool blue-violet dungeon, rotten timber shore and slate masonry.
- **Panel row A — Ordinary punch:** punch contact + expressive recoil + airborne/down corpse; essential baseline.
- **Panel row B — Decapitation:** pre-contact justified force (e.g. strong authored cutting trap) + clean readable detached head silhouette + body/gear falling separately; blood used as limited focal punctuation.
- **Panel row C — Explosion into pieces:** story-available dynamite / industrial explosive (not pre-Guild Carl) + shock silhouette + 3–5 visually distinguishable body/weapon pieces and smoke, controlled negative space.
- **Panel row D — Rock crush:** shore broken by Carl -> stone landing on goblin with a clear brief squash -> ears/boots/fruit club under/around rock.
- Optional fifth separate strip for **kick -> airborne goblin -> hit second goblin** (shows *non-lethal collision first* as the gameplay starting point, then alternative lethal variant).
- **Goal:** more outrageously violent and more faithful to the specific designs simultaneously. Favor *strong silhouettes and obscene black comedy* over realistic mutilation. Character fidelity is a hard acceptance gate; if the generated visual redraws the characters, treat it as a **mood failure**, not usable concept art. A true source-faithful sprite montage should be composed from existing grid/atlas assets (with new original-pixel keys where necessary) and not visually approximated by freehand generative painting.

## 8. Review questions and governance

- **Violence level:** when does a finishing blow earn decapitation or a gib explosion, and when would that cheapen surprise? Should wall impact be a separate recipe or variant of kinetic?
- **Freedom:** which parts are mandatory per species profile and which are optional signature deaths? One grotesque unique moment per new monster may be enough.
- **Scale:** which gore effects are still readable at 36px goblin height and full-speed Bevy gameplay?
- **Rules:** is anything from corpse fragments allowed to remain authoritative, or do all remains stay cosmetic in MVP?
- **Options:** do reduced gore settings materially improve clarity/comfort without changing game simulation?
- **Style:** judge by actual Carl, goblin, environment master imagery side by side at native size.

**Review triggers:** consult this language when a new monster species, boss, environmental hazard, explosive, magical kill, corpse physics effect or gore option is designed. When approved, link from the universal art-direction guide and `AGENTS.md`. Do not silently claim this draft is a final global rule; archive rejected experiments and update the living [character progression horizon](character-progression-horizon.md) only where a real long-term identity change occurs.
