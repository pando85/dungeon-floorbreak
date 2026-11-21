# Product vision and design pillars

**Status:** concept v0.1 (2026-11-15). Working title: Dungeon Floorbreak.

## One-sentence pitch

A darkly comic, detailed-pixel-art, 2D action platformer about two contrasting companions surviving a lethal, absurd dungeon by combining improvised explosives, magic, and **selective environmental destruction**, with a third powerful companion joining later.

## Audience and experience

Controller-first single-player and same-screen two-player local co-op for Linux desktop and Steam Deck. Fast, readable, laugh-out-loud improvisation rather than a precision-platforming simulator. Sessions of 5–15 minutes per floor/section; an initial slice of ~5 minutes is enough to validate the core fun.

## Mandatory primary gameplay goal (owner decision, 2026-11-17)

**THE CRAWL is combat-first: the central pleasure is killing enemies in spectacular, creative and satisfying ways.** Traversal, switching, asymmetric character gates and selective destruction should continually create more exciting fighting options, not transform the game into a mainly puzzle-driven platformer. Quiet exploration and the opening Dozer escape are intentional pacing exceptions; action should be the dominant reason players keep playing. See [combat-first encounter design](combat-first-encounter-design.md). No kill quota or promise of constant action is implied.

## Pillars (priority order)

1. **Distinct character fantasy.** Carl is a resourceful, close-range, explosive trapmaker; Donut is expressive, mobile, ranged and magical; Mongo is an impactful kinetic disruption tool later. Switching/coordination must have a tactical payoff.
2. **Meaningful destruction.** Breakable material and props create paths, eliminate cover, change line of sight and make enemies react. Some load-bearing structures can collapse with authored rules. Destruction is frequent, not arbitrary or universal.
3. **Responsive controller feel.** Clear acceleration, coyote time, input buffering, feedback and forgiving recovery; manual feel beats unnecessary simulation realism.
4. **Readable dark pixel art.** Medium-to-large character silhouettes, strong readable outlines, expressive facial cues, controlled palettes, foreground/background separation. The dark mood should not obscure game-critical objects.
5. **Surprising but controllable chaos.** Explosions and combinations create emergent comedy while preventing unwinnable rooms and unavoidable friendly-fire loops.
6. **AI-buildable production.** Every asset/mechanic is versionable, reproducible and testable through CLI. Human judgment remains final on entertainment value.

## Mandatory cross-cutting tone: 1980s–1990s action-film antihero

**Owner decision (2026-11-15):** The entire game must combine the scrappy **everyman antihero** energy of 1980s–1990s action cinema with the dungeon's vicious absurdity, dark comedy, explosive improvisation and sincere emotional stakes. Carl begins as an ordinary, slightly awkward man, not a handsome or heavily muscled superhero; his capability, physical toughness and charisma build gradually and visibly through the story. The tone governs character art, animation, enemy design, spaces, encounter staging, interface, sound and narrative, not just promotional illustrations. Read and enforce [`antihero-tone.md`](antihero-tone.md) before generating new content. Film references are influences only, never reusable copyrighted art/dialogue/music.

## Explicit non-goals for the first slice

No pixel-perfect granular fluid/temperature simulation; no full environmental procedural generation; no simultaneous online netcode; no Steam backend requirements; no complete RPG economy; no voice acting; no franchise dialogue or cutscenes; no licensed franchise assets in the public repository.

## Fun over canon (owner decision, 2026-11-21)

"We have to make concessions and deviate from the canon to make the game fun." The books are a source of characters, tone and story beats, not a rulebook. When book canon and good gameplay disagree, **gameplay wins**.

- Story anchors for unlocks follow the books where that works (check chronology first, as before), but timing, numbers, ranges, cooldowns and how an ability behaves are tuned for fun, readability and co-op, not for fidelity.
- Book abilities may be simplified, split, merged, moved or invented as game adaptations (for example Puddle Jumper as a short personal blink with a 9 s cooldown instead of the book's long-cooldown group teleport).
- Every deliberate deviation is recorded in the design doc as a **game adaptation** with a one-line reason, so canon claims stay honest. Never present an adaptation as canon.
- This does not relax the rights guardrails, the antihero tone or the replaceable fan-concept layer.

## Inspiration, not cloning

- **Broforce**: frenetic side-scrolling action, rapid combat readability, player-authored destruction paths.
- **Mewgenics**: high character expressiveness and reusable layered animation ideas (not copied visuals/rigs).
- **Dead Cells**: impact frames, visual effect contrast, movement responsiveness.
- **Noita**: inspiration for material-driven interactions, but *not* the per-pixel simulation scope.

## Living long-term progression horizon

See [character progression horizon](character-progression-horizon.md) for a deliberately non-binding, iteratively reviewed vision of how Carl, Donut and eventually Mongo could grow from Floor 01 through later floors. Consult it when planning major skills, levels or new floor mechanics; concrete unlocks require their own validation and owner approval. The [MVP controller contract](mvp-controller-contract.md) remains authoritative for accepted input semantics.

## Scope choices still open

- **Approved target (2026-11-16):** hybrid solo switching + AI companion/context commands, with two-player local co-op from the start. Mechanic UX, camera and controller ownership still require real testing; see [Floor 01 design bible](floor-01-gameplay-progression-v0.md). Shoulder Mode and mounted attacks remain unapproved experimental ideas.
- Checkpoint/death model: short replayable stages vs limited lives.
- Progression scale: authored level campaigns first, then optional run modifiers/loot.
- How far dark comedy can go while remaining legible and accessible.

## Original-IP escape hatch

The engine must use generic IDs (`demolitionist`, `caster`, `charger`) distinct from any franchise names. The fan-concept character mapping is a disposable content package, not a dependency of the physics, maps, UI or combat core. Commercial release requires a rights decision before external distribution of character-specific builds.
