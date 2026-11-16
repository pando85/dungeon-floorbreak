# Floor 01 opening — catacomb corridor and goblin encounter

**Status:** owner-approved **concept direction** on 2026-11-16; not approved production pixels, mechanics, canon-accurate character sprites, level geometry, or Bevy gameplay.
**Project:** THE CRAWL — *NO PANTS. NO MERCY.* (working fan-concept branding)
**Relevant work:** WORK-007/008 art pipeline, Floor 1 combat greybox PR #21 and isolated Bevy Character Playground PR #22.
**Engine constraint:** **Rust + Bevy only**. No Godot workstream.

## Creative decision: gradual immersion

Floor 1 should steadily expand the player's understanding, visual vocabulary, and mechanical demands. Do **not** make its opening corridors look like a toxic sewer, a decayed factory, or a television studio. Drama primarily comes from the emerging threat and the player's actions, not constant visual clutter.

Approved initial sequence:

1. **Discovery:** monumental entrance is an exceptional landmark, not the template for every corridor.
2. **Calm traversal:** dry, relatively plain brick catacombs with occasional torches; the player has space to learn movement and read the level.
3. **Unease:** narrower junctions, small environmental clues, some green-glowing lichen as a local accent, restrained industrial servicing.
4. **Chase:** the Goblin Murder Dozer charges into a wider corridor; escaping and exploiting its poor turn radius matter more than attacking it.
5. **First confrontation:** two ordinary goblins can dismount and approach on foot; an introductory melee encounter follows.
6. **Reprieve and progression:** Tutorial Guild / Mordecai is the narrative destination; subsequent systems enter progressively, not all at once.

These are **adapted gameplay beats**, not a frame-by-frame assertion about the novel or a mandate to reproduce franchise dialogue.

## Environment grammar — three reusable intensities

| Module | Meaning | Visual elements | Where to use |
| --- | --- | --- | --- |
| **A — plain brick** | The ordinary fabric of the floor | Warm orange torch-lit brick falling into cold blue shadow (ADR-0007), dry flat masonry floor, occasional simple doors/torches | Connecting passages, readable action spaces |
| **B — structural arches** | Reassuring, monumental architecture in moderation | A few broad stone/masonry arches, pillars, a deep layered parallax gallery visible through the arches with cold blue haze (ADR-0007), warm torchlight | Intersections, important routes, orientation landmarks |
| **C — restrained industrial** | Localized infrastructure and activity | One or two pipes/valves, occasional iron rail/catwalk, mechanical wear | Maintenance transitions, goblin presence, special encounters |

Treat A/B/C as **one coherent material family**, not three isolated biomes. Their distribution is authored; do not hardcode a 70/20/10 percentage. Long uninterrupted corridors can be A, B provides rhythm, C marks a change in function. **The owner particularly liked C**, but does not want it everywhere.

### Environment locks

- Flat, solid, dry floor; no permanent sewer channels, toxic water, aggressive overgrowth, dripping slime, or ubiquitous ruins.
- Warm orange torch-lit brick that falls off into cold blue shadow, gray/neutral stone edge treatment, thicker copper/brass pipework where industrial detail appears, and a deep layered gallery visible through arches. Limited green lichen **only** in relevant secondary passages. *(Updated 2026-11-16 per [ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md); this replaces the earlier "muted red-brown brick ... restrained orange torch highlights" wording. All other locks are unchanged.)*
- Use repetition and negative space deliberately; avoid decorative detail behind the player, collision edges, or telegraphs.
- Keep at least one coherent architectural line through the corridor; arches are **art-direction proposals**, not proven universal first-floor architecture.
- The fish-demon relief, giant wooden doors, ornate wrought-iron stairs, and marble landing belong to the **special entrance**, not the modular default wall kit.
- Industrial devices, barrels, ladders, crates, and scaffolds are situational props; save workshop density for an actual goblin workshop.
- Early TV-show aesthetics, magenta broadcasting machinery, audience silhouettes, and intrusive HUD staging are deliberately **de-emphasized**, not denied across the whole game.
- Action tone comes from surprise, clumsy improvisation, physical effort, violent machinery, absurdity and danger. Do not overuse VHS effects, superhero postures, or glossy sci-fi panels.

## First goblins — selected concept direction

**Owner likes the expressive green goblins in the 2026-11-16 chase concept. Keep their general silhouettes, energy, and color contrast.** These are not approved Aseprite source files.

- **Common goblin A:** level 2, short mostly bald green humanoid, pointed ears, angular face, sharp teeth, leather/rag clothing. Smaller scrappy silhouette; loud, reactive acting.
- **Common goblin B:** also level 2. May have a slightly different stance/face or small visual accessory so the two riders do not look cloned. A separate heavyweight class, more HP, special stagger, or differing damage is **NOT** decided.
- **Goblin engineer:** level 3, a **cooking pot helmet**, more tool-focused silhouette, visibly operates the Dozer. A wrench may be a noncanonical visual shorthand; inventing ranged bombs, buffs, repairs in combat, or class perks requires a later gameplay decision.
- **On-foot weapons:** in the early encounter the ordinary goblins use wooden sticks with **pineapples** attached. Generated concept sheets displaying spiked metal maces are **illustrative errors**, not approved lore/equipment. Use comic but readable fruit-head shapes; a pineapple falling off can inspire an authored gag.
- Preserve different faces and readable motion but avoid giving the duo RPG roles derived solely from generated labels.
- No final goblin pixel heights, sprite bounding boxes, attack frame counts, or damage values are accepted. Size against the **approved Carl 52 px** and **Donut 40 px** masters in an actual Bevy composition.

## Goblin Murder Dozer — selected silhouette

**Keep** a tractor-scale, steam-powered, tank-tread contraption made of mismatched, battered, rusted metal; an oversized **spinning spiked front wheel**, chimney/steam pipes, exposed metal plates, and **three goblins riding on top**. Its awkward industrial geometry and black exhaust should feel mechanical, dangerous, and faintly ridiculous.

- First encounter is primarily a **moving pursuit/set-piece enemy**, not a traditional health-bar boss arena.
- Its large turning radius is important for staging the escape through junctions.
- Distinguish the spinning wheel's lethal contact zone from the readable main hull. Telegraph speed and turning behavior. No unavoidable wall-to-wall collision.
- Use smoke locally around the machine; the entire corridor must not become an opaque haze.
- Rear ladder, vehicle-top surfaces and boiler breach can be researched for later authored interactions, not automatically implemented.
- Keep the dozer and all three goblins as **separate visual/animation concepts**; the on-foot goblin actors must be reusable.
- Generated side-view art is a **shape reference only**. Do not extract it as actual sprites, physics colliders, VFX, or terrain masks.

## Canon provenance vs creative interpretation

**Primary source:** user-supplied *Dungeon Crawler Carl*, Book 1, Chapter 2, locally checked against the EPUB. Do **not** commit book text, pages, or substantial quotations.

| Canon-grounded reference | Our adaptation / unresolved |
| --- | --- |
| Monumental entrance with wrought iron stairs, marble landing and decorated double wooden door | Keep as unique opening landmark; side-view layout and exact proportions still designed |
| Wide torchlit dungeon passages, brick-lined secondary passages and some glowing lichen | A/B/C modular grammar, cold blue depth (layered parallax gallery, ADR-0007) and industrial accents are artistic choices |
| Three small green goblin riders; two level 2 common goblins and one level 3 engineer with a pot helmet | Two visually distinct common riders, not necessarily two enemy classes |
| Tracked improvised rusted steam contraption, spiked spinning wheel, smoky pipes | Chase staging, camera logic, colliders, weak points and animation are game design |
| Goblins' poor cornering and pursuit through junctions | Gradual discovery -> pursuit -> on-foot fight -> Guild |
| Wooden sticks with pineapples on the ends | Preserve in encounter equipment study; generated spiked maces are not canon |

**Important:** Chapter 15's larger goblin engineering/workshop facilities are a **different later area**. Do not move their clutter and machinery into this opening encounter by default. Later special areas get separate concept art and bespoke palettes/props.

## Character identity and pixel-production gates

- **Never use generated Carl/Donut figures** in the 2026-11-16 chase illustration as character designs. They are explicitly wrong. No Carl shotgun.
- Reuse exact editable masters and pivots: `assets/source/characters/carl/carl_h52.ase` (52 opaque px) and `assets/source/characters/donut/donut_h40.ase` (40 opaque px). Their existing idle prototypes are not finished movement sets.
- Work at virtual 640×360 and 640×400 first; tile grid 16×16 remains provisional. Favor nearest-neighbor/integer presentation.
- Design and test environment layers separately: distant matte, midground arches/rails, collidable gameplay terrain, actors, interactive props, restrained foreground.
- Author native editable `.ase`/pixel-grid material sources, deterministic generated atlases/metadata, manifests and checksum/provenance. **Do not crop rendered concept art into sprites.**
- Test at native 1×, 2× on 1280×800, and real Bevy camera. Check contrast, foot contact, silhouette overlap, readable hazards and long horizontal traversal.
- Keep runtime entity and mechanics IDs IP-neutral and rights/distribution restrictions unchanged.

## Selected visual references and boundaries

The owner explicitly accepted these **conceptual properties**, not arbitrary generated fine detail:

- The revised **single-door monumental entrance** with subtle industrial features (special landmark).
- A/B/C **normal-corridor** comparison: plain brick, restrained arches, slight localized pipes.
- The **chase illustration**: brick catacomb, green goblin group, spiked tracked Murder Dozer, expressive absurd action.
- The following **three concept reference PNGs are committed in this PR** under [`assets/concept/floor-01/opening/`](../../assets/concept/floor-01/opening/README.md), with image hashes, byte sizes and generation/crop provenance in [`manifest.json`](../../assets/concept/floor-01/opening/manifest.json):
  - [Clean environment-only brick catacomb](../../assets/concept/floor-01/opening/01-catacomb-environment-clean.png) — environmental mood only, not a tilemap or parallax source.
  - [Three goblins concept crop](../../assets/concept/floor-01/opening/02-goblin-trio-reference-crop.png) — green goblin shapes, expressions and engineer's pot helmet; generated weapons/classes/pixel sizes are not accepted.
  - [Murder Dozer concept crop](../../assets/concept/floor-01/opening/03-murder-dozer-reference-crop.png) — tracked steam machine, front spiked wheel and riders; not production sprites, collision or a boss spec.
- **Not included:** the full chase illustration with *incorrect Carl/Donut representations*. Approved Carl and Donut masters remain the only authoritative character references.

**Illustration caution:** Generated concept boards contain misleading labels, invented RPG subclasses, precise pixel sizes, mechanics and incorrect weapons. Their labels are not authoritative. The selected chase illustration's Carl/Donut are noncanonical.

## Human review gates — before calling any environment/enemy art production ready

- [x] Selected overall Floor 1 opening environment direction (A/B/C in one family).
- [x] Selected expressive goblins and tracked steam Dozer silhouette.
- [x] Selected gradual immersion and the pursuit-first encounter direction.
- [ ] Actual owner review/acceptance of isolated images as separate source-of-truth assets, not merely crop/reference files.
- [ ] Original editable wall, torch, arch, pipe, goblin and Dozer sprite masters with provenance.
- [ ] Real Carl/Donut sources composited without scaling deviations in Bevy.
- [ ] Real goblin + Dozer relative-scale review in 640×360 and 640×400.
- [ ] Chase/turning, on-foot combat, and reliable retry measured in Bevy.
- [ ] Pixel-contrast, occlusion, UI/FX, animation and controller game-feel acceptance.
- [ ] Rights/provenance check before any public executable distribution.

**This document is a direction lock for future art and agent collaboration, not an assertion that the scene has been built.**
