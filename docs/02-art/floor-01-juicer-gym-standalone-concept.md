# Floor 01 — The Juicer's Gym, standalone environment concept

**Status (2026-11-17):** The **gym's visual identity and mood** were owner-approved in the project conversation. A revised **standalone** concept board is now available **for owner review**, not yet approved as the final room composition. Do not imply that its interaction icons, poses, signs, room distances or native sprite measurements are locked.

**Scope:** environment concept only. No gameplay code, combat physics, Bevy atlas, encounter layout implementation, final pixel-grid environmental kit or release permission.

## Existing approval

The owner approved the gritty, comically exaggerated **1990s gym inside the recognizable Floor 01 stone catacombs**: brick arches, industrial piping, warm suspended lamps, occasional magenta fluorescent accents, cracked mirrors, rusty squat/bench equipment, weights, lockers and an expressive bodybuilder mural.

This is distinct from the later **Ball of Swine arena**. The first pass mixed both spaces in a single image; the approved gym mood was preserved in the **gym-only crop** and is now being developed into its own complete concept.

### Source and art compatibility

The [Floor 01 art atlas](floor-01-art-atlas-v1.md) identifies the Juicer gym as a special room containing **reception, turnstile, exercise machines, mirrors and heavy weights**, rather than a generic factory or neon coliseum. Maintain existing A/B/C catacomb material grammar: ordinary torchlit brick/stone foreground, architectural recesses and *locally concentrated* gym/industrial props. The area is a deliberate late-floor visual surprise, not a mandate to re-theme every corridor.

Character/engine references remain the already-approved Carl **52 opaque pixels**, Donut **40 opaque pixels**, virtual 640×360/640×400, original Rust + Bevy integration. The tiny illustrative characters in AI concept boards do not replace their canonical source art.

## Proposed single-room side-scrolling composition

An intuitive left-to-right panorama of **three connected zones**, with one visually continuous walking line and a camera-friendly action corridor.

1. **Entrance / reception (left):** brick threshold and clear gym identity, compact wooden/metal service counter, old computer or ledger, small display shelves and lockers; a *separate clearly recognizable turnstile* that establishes access flow without blocking gameplay permanently.
2. **Main gym / central fighting space:** largest uninterrupted foreground floor, mirrors across the back wall, several weight stations, bench press and loosely clustered dumbbells in the midground; keep scenery legible during launches and impacts. The Juicer's mural is atmosphere, **not a character sprite**. Avoid giant permanent pillars within the primary player-versus-enemy sightline.
3. **Training machines (right):** weight machine/cable station, punching bag, tires and secondary gym props; a final architectural threshold for continued movement. Preserve the natural catacomb structure beneath these props.

**One room / one theme.** Do not include the Ball of Swine arena, concentric ring mechanics, pig emblems or crowd-stands in this environment study.

## Art-direction and readability tests before final approval

- **Framing:** the wide composition must be recognizable as one connected 2D level, with a clearly illustrated transition between reception, playable main gym and machines. Do not treat a cropped overview as executable topology.
- **Reception/turnstile:** both visible in the actual main panorama, with a focused inset that explains their different silhouettes.
- **Combat-first visual hierarchy:** bright player/enemy silhouettes and the walking plane read in front of darker background gym gear, at 1× native virtual resolution. Show room clearance for physical knockback and multi-enemy combat without turning every object into a collision obstacle.
- **Prop material taxonomy:** separate candidate *breakable* glass, loose weights, benches and hanging bags from **structural, load-bearing walls, floors and arches**. Generated labels such as “explode”, “throw”, “crush” or “hazard” are *visual ideas*, **not approved gameplay mechanics**. Detailed deterministic collision, destruction and no-softlock rules belong to local agents after their own design approval.
- **Lighting:** retained soft warm floor pools and warm spotlights; magenta fluorescent accents only in and around the gym. Avoid continuous neon fog, too much reflective bloom or light that hides attacks.
- **Rights and continuity:** no unauthorized real-world logos, brand marks, ripped character art or direct-book quotes. Any Carl/Donut stand-ins in the candidate concept are illustrative and not the approved sprites.

## Reference image tracking

| Filename | Scope |
| --- | --- |
| `assets/concept/floor-01/juicer-gym/floor01-juicer-gym-approved-mood-reference.png` | **Owner-approved visual identity**: the cropped original gym concept, historic baseline; not a complete gym room layout |
| `assets/concept/floor-01/juicer-gym/floor01-juicer-gym-standalone-revision-candidate.png` | **New concept candidate for owner review**: gym-only wide panorama, reception, turnstile, three zones and prop insets; room-geometry and interaction text not approved |

Both are original OpenAI-generated images in the current project conversation. The original bytes, SHA-256, dimensions and generated source names are recorded in the image [manifest](../../assets/concept/floor-01/juicer-gym/manifest.json) and the user handoff archive. The owner must upload the two source PNGs to this **separate PR** branch before the integrity tests can pass. Preserve images unchanged rather than pretending the source drawings are native pixel art.

The owner's reported public art permission is recorded in the [rights and release policy](../04-production/rights-and-release.md); it is not a licence to distribute a game or source-derived standalone asset pack.

## Remaining work before closing this independent concept PR

- [x] Owner-approved gym *mood* and materials recorded separately from enemies.
- [x] New **standalone** gym composition drafted with reception, turnstile and combat space.
- [x] Original reference/candidate hashes and dimensions pinned.
- [ ] Owner reviews the revised room **composition**; revise if necessary and pin final accepted reference.
- [ ] Both original PNG files uploaded in `assets/concept/floor-01/juicer-gym/`.
- [ ] New art-manifest test and docs CI green on the *final* head.
- [ ] Review approved visual scope, mark PR ready, and merge.

**Outside this PR:** finalized art tiles, editable `.ase` masters, combat mechanics, destructible prop implementation, routing, scripted Juicer behavior, Ball of Swine arena and playable level.
