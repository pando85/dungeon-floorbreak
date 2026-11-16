# Mongo — approved visual character bible (fan-content layer)

**Decision:** CREATIVE-2026-11-15-MONGO-01.  
**Approval:** Owner approved the preferred first exploration sheet, baby → juvenile → adult evolution, and the adult turnaround/acting board (2026-11-15).  
**Source status:** Three approved original PNGs **committed unchanged and verified** against original Git blob identities; `SHA256SUMS` manifest accompanies them. This is a complete **reference/concept-art package**, not editable gameplay sprites.  
**Runtime mapping:** Mongo → IP-neutral `charger` (WORK-017, after the initial Carl/Donut slice).

## Creative north star

Mongo is a **feathered, expressive, athletic, reckless and affectionate raptor** who matures from a deceptively adorable hatchling into Donut's powerful mount. Humor comes from contrast: dangerous teeth and sickle claws, yet a curious chick-like manner, impulsive food fixation and conspicuous devotion to Donut. His relaxed face must be charming, not permanently furious.

**Selected combination:** The owner's favorite **first** four-option sheet. Use **D's heroic/readable mount shape + B's athletic legs and speed**, just a trace of A's softness; avoid C's oversized bodybuilder bulk. A later four-column refinement was rejected as insufficient progress and is not an alternate canonical master.

## Identity anchors — art-approved

| Anchor | Production direction | Avoid |
| --- | --- | --- |
| Shape | Feathered biped, long sweeping fan-tail, springy hind legs, readable rear sickle claw | Generic unfeathered movie reptile, armored dragon or huge tank |
| Face | Golden/yellow expressive eye, distinctive blue mask, pink-red crest, tooth-lined beak/snout | Angry eyes in every neutral frame |
| Feathers | Saturated pink-red dominant, blue flight/wing/tail accents, warm cream throat and chest | Random recolors or all-blue body |
| Proportions | Compact, unstable fluff as baby; long juvenile legs; tall athletic adult capable of carrying Donut | Distinct unrelated silhouettes at each stage |
| Performance | Fast neck/head snaps, plume/crest acting, tail balance, whole-body surprise and tenderness | Generic monster roars with no soft acting |
| Bond | Donut is a proud small rider; Mongo is affectionate, impulsive, defensive and energetic | Mechanical mount with no personality |

Palette swatches in the illustrations are *concept references*, not locked HEX values. Preserve a fixed face-mask/crest design and blue feather map across views and animations.

## Three growth stages — visual evolution, not confirmed book timeline

1. **Hatchling:** round, messy pink plumage; comically oversized eyes and small claws; a cute chick silhouette hiding surprising teeth. Acting: wobble, chirp, curious peck, unexpected bite.
2. **Juvenile:** same eyes, face-mask and crest, but longer legs/snout/tail, less baby fluff, stronger blue feathers; slightly awkward and overeager. Acting: accelerate, skid, stumble, pounce.
3. **Adult / mount:** D+B athletic proportions, long dynamic tail, strong legs, expressive crest and readable rider-safe back, pink/red feathers with blue accents. Acting: sudden calm devotion → explosive aggression → guilty or pleased recovery.

The generated art contains numbers such as **18 cm, 60–90 cm and 1.2–1.5 m**. They are *illustrative AI captions*, not sourced canon or locked in-game scale. Exact narrative growth schedule and physical dimensions require primary-literature verification; see [lore tracker](mongo-lore-research.md).

## Adult model sheet and consistency

The final owner-approved board illustrates side, front, rear and 3/4 views, head-expression studies, close-up eye/beak/claw/feathers, actions and Donut-mounted poses. It is a **visual north star, not a mathematically consistent turnaround**; agents must establish a single editable adult master:
- Same head width, muzzle length, eye mask, crest direction and feather overlap from every angle.
- Same hind-leg joint structure, count of toes/claws, ground contact and tail origin.
- Tail stays expressive without hiding hazards, Donut or Carl.
- Distinct throat/cream, crest/pink and wings/blue from all directions.
- Define a rider/seat/socket pivot for Donut; avoid overlap or clipping during all mount states.
- No sprite may be generated independently and silently restyled away from the adult master.

**Adult expressions:** `neutral`, `alert`, `curious`, `content`, `startled`, `hungry`, `affectionate`, `protective`, `rage_roar`, `tired`. Keep the default adorable; use neck, head and crest movements in addition to eye shape.

## Gameplay proposals — separate from visual approval

The existing game specification maps Mongo to a later-stage **companion/triggered charge** under `charger`, not a promised third local player. Candidate attacks: bite, claw, short forward lunge, protective roar and controlled charge. Candidate environment effect: break *selected brittle props* / scatter groups, never disable mandatory pathways. **These are original game proposals, not confirmed book abilities.**

The bond with Donut matters mechanically and visually: mount start/stop, run, jump and safe dismount require a fixed attachment anchor and independent review. It is fine to defer playable mounting after the first AI companion/charge prototype.

The collision/hit windows are authoritative gameplay data; animation playback must never determine physics nondeterministically. Charge requires anticipation, direction cue, impact and recovery; no clipping through solid terrain, impossible stop distance or post-destruction softlocks. Runtime identifiers remain IP-neutral.

## Animation backlog

| Scope | Initial tags | Critical acceptance |
| --- | --- | --- |
| Baseline first | `idle`, `run` | Single editable adult sprite master; stable feet; readable tail/crest at 1× |
| Air movement | `rise`, `fall`, `land` | Springy weight and impact; fixed collision anchors |
| Charge | `anticipation`, `charge`, `impact`, `recover` | Distinct warning/recovery; deterministic and non-softlocking |
| Combat | `bite`, `claw`, `roar`, `hurt` | Visible arcs; no silent damage windows |
| Donut interaction | `mount_idle`, `mount_run`, `mount_jump`, `dismount`, `affectionate` | Stable seat/socket, rider visible, coherent bond |
| Future phases | `hatchling_idle`, `juvenile_run`, stage emotes | Facial/crest/color continuity across growth |

Do **not** crop the concept sheet's tiny illustrative mock sprites into an atlas. Build hand-curated/AI-assisted **editable ASE/Aseprite/LibreSprite source** and export through [the verified pipeline](asset-pipeline.md). Lock master palette, pivot, frame size, animation tags and metadata before high-volume production.

## Native-resolution acceptance

The virtual canvas remains a **640×360 / 640×400 hypothesis**. The owner **approved adult Mongo at 76 px opaque height** on 2026-11-16, alongside Carl 52 px and Donut 40 px; see the [scale approval](character-scale-approval.md). The 64/88 px Mongo sprites remain historic comparison candidates, not the default master. Bevy rendering, running/charge and Donut mount overlap are not yet validated.

First experiment: a lateral adult idle and 6-frame run in the actual dungeon camera, beside Carl and Donut, at original virtual size and integer 2×. Test mounted and unmounted silhouettes; no overlap of attack tells; contrast against blue-violet/magenta/amber world; stable foot and seat anchors; readable crest and long tail; no palette flicker or independent-frame redesign. A human must approve the genuine sprite master before mass animation.

## Canon, provenance and publication rules

The franchise-derived character design is a **fan/community concept**, not a licensed/production-ready playable character. The owner reports permission (`ART-PUBLICATION-ALL-V1`) to publish all fan-concept character art, concept images and production art, in this public repository; this is not independently verified and does **not** establish a public reuse licence, game/Steam/trailer/merchandise/standalone asset-pack distribution, sublicensing or official endorsement. Production art still needs provenance/manifest/checksum records. See [rights policy](../04-production/rights-and-release.md).

**Never use generated annotations as book evidence.** [Lore verification tracker](mongo-lore-research.md) explicitly separates primary-source claims from artistic choices. [Approved source register](../../assets/concept/mongo/approved/README.md) and [provenance](../../assets/concept/mongo/approved/provenance.md) record original exact PNG identities. The original PNGs are now committed, Git blob identity-verified and covered by `SHA256SUMS`. This is a completed **approved concept reference package**; native-resolution editable Aseprite sprites, primary-book lore verification and game distribution remain separate gates.

**Do:** preserve the approved pink/red feathered D+B adult; baby/juvenile/adult continuity; affection, impulse and danger; clean native-scale pixel art; evidence-driven lore.  
**Do not:** add unapproved shotgun/weapon props, make a permanent angry beast, inflate the adult into a tank, invent chapter/floor acquisition timing, or treat concept-board sprites as actual assets.
