# Floor 01 — The Juicer's Gym: approved environment direction

**Status (2026-11-17):** The 1990s gym-in-catacombs mood was approved earlier. In the follow-up session, the owner explicitly chose to save a newly generated full-width **gym-only panorama** and the accompanying **proposed encounter direction**. This approves the image as an **art-direction reference**, not native level topology, exact boss behavior, balance, final sprites, collider dimensions, or book canon.

**Scope of this PR:** original standalone environment concept image, source-byte validation, and creative encounter notes. No Bevy gameplay, editable .ase sprites, damage/physics schemas, animation implementation, or completed boss fight is implied.

## Art direction and continuity

Keep the approved A/B/C Floor 01 catacomb language: red-brown brickwork, carved arches, industrial pipe retrofit, amber hanging lights and restrained magenta fluorescent accents concentrated inside the gym. A ridiculous 1990s bodybuilding mural, battered reception desk, turnstile, rusty workout equipment, dumbbells, cracked mirrors, lockers, squat rack, cable station, hanging punching bag and tires convey local personality and dark physical comedy.

The Juicer's Gym remains **separate from the Ball of Swine ring arena**. Do not add concentric rings, pig emblems, arena audience stands, or floor-wide saturated neon. The gym scene must read as a **side-scrolling combat location**, not a top-down floor plan.

## Owner-selected single-room panorama

The new reference is a coherent left-to-right view with these three contiguous landmarks:

1. **A / Reception and threshold:** service desk and old terminal, shelving/lockers, a **separate legible turnstile**, and a recognizable gym banner. The left threshold is a safe entry/readability beat, not an irreversible lock.
2. **B / Central fighting lane:** the largest continuous playable floor, mirrors, bodybuilder mural, bench press, dumbbells and squat station arranged in or behind the midground. Enemies and players must retain visual priority; avoid permanent tall foreground clutter or a narrow collision obstacle course.
3. **C / Machines and onward exit:** cable machine/pulleys, hanging bag, stacked weights/tires and stone arch transition. Opportunities for environmental combat must not close the exit.

The concept artwork is **illustrative**: its illustrated apparent pixels are not the game's actual source-pixel scale, and its prop placement is not literal collider geometry. The approved Carl 52 px and Donut 40 px masters are the scale authority at 640×360 and 640×400 virtual resolutions.

## Candidate gym combat interactions (NOT implemented or numerically approved)

These proposals implement the already-approved combat-first direction: quick, consequential strikes, readable physical reactions and occasional spectacular environmental kills. **Direct boss combat must remain a viable route**; these props create options, not a mandatory switch puzzle.

| Prop | Candidate effect | Safety / implementation boundary |
| --- | --- | --- |
| Loose dumbbells, weight plates | Light weights get launched or kicked; a heavy plate may fall when an authored support releases | Weight/impulse classes fixed by deterministic gameplay logic, not free physics chaos |
| Bench press / workout bench | Shove, topple or temporarily block a lane; impact briefly interrupts an enemy | Does not create an unpassable permanent wall |
| Hanging punching bag | Clear swinging pendulum; timed impact can interrupt a boss charge | Consistent tell, bounded swing, authored hit registration |
| Cracked mirrors | Visible localized shattering on hits | Cosmetic by default; no surprise offscreen glass damage |
| High pulley / weight stack | Donut can reach a high mechanism; released weights threaten a marked ground region | Requires an already-unlocked capability; solo switch and 2P both possible |

**Structural boundary:** stone arches, load-bearing floor, primary passage and required regroup locations are protected. Broken props may create short-term tactical changes but never strand an actor. L3 general structural collapse and simulated corpse projectiles are *not* granted by these concept images.

## Proposed Juicer boss arc (three encounter beats; design hypothesis)

1. **Warm-up / establish tells:** exaggerated melee strikes, a stomp and a committed charge are readable and punishable. Carl can use normal attacks during recoveries. A collision with a gym fixture creates feedback, not an automatic instant win.
2. **Use the room:** boss attacks displace selected benches or equipment; bag/weights/pulley interactions create optional interruptions. Donut gains positional utility through elevated access or an ability legitimately unlocked by this late-floor story beat.
3. **No rest / transformed battlefield:** pressure rises and selected props break. The fight remains readable at native resolution, with a direct-damage finish **or** a well-telegraphed environmental finish. No repeating invulnerability gates or mandatory memorize-the-pattern inputs.

**Boss scaling note:** preserve the owner-approved grotesquely muscular troglodyte art family, but verify the eventual runtime height against the novel and the approved character scales; the exploratory artwork deliberately exaggerates relative size. Move lists and apparent weaknesses are **gameplay adaptations, not verified book facts**.

## Solo / local co-op contract

- **Carl:** close physical force, strikes, heavy props, punish recovery windows and selective destruction.
- **Donut:** jump higher/farther, reach elevated fixtures and create positions or interruptions. She is playable in 2P from the beginning; **no early magic before story unlock**.
- **Solo:** the same encounter must work with instant switch and a robust companion; never require a second human, simultaneous mutually exclusive inputs or unsupported Shoulder Mode.
- **2P:** both humans retain purposeful actions; shared camera must keep sprites and cues legible without indiscriminate shrinking.

These are encounter hypotheses to test, not a declaration that WORK-005/006 or boss integration already exist.

## Visual and engineering acceptance boundaries

- Preserve a clear continuous main fighting floor and a reachable onward exit after every authored destruction sequence.
- Maintain warm/cool contrast with accent magenta, not constant neon bloom.
- Keep broken mirrors cosmetic until tested, and keep physics contacts deterministic (bounded impulses, deduped ordered events).
- Evaluate impact, bosses, cooperative control and environmental death on real Rust + Bevy playtests; CI alone cannot approve game feel.
- Keep the **Juicer** encounter separate from the **Ball of Swine** arena concept and the opening goblin CP-01/02/03 integration of issue #40.

## Art reference and provenance

| Reference | Availability and meaning |
| --- | --- |
| [Floor 01 provisional gym atlas excerpt](../../assets/concept/floor-01/atlas-v1/09-juicer-gym-atlas-excerpt.png) | Already on main; supplementary historical area reference, not the previously exported owner-approved mood crop |
| [New approved standalone panorama](../../assets/concept/floor-01/juicer-gym/floor01-juicer-gym-standalone-panorama-approved.png) | **Original generated PNG** selected in the 2026-11-17 gym follow-up; source pixels and SHA-256 pinned in the local [manifest](../../assets/concept/floor-01/juicer-gym/manifest.json) |

The previously prepared original cropped mood sheet and the earlier revised candidate had different source files and are **not available in this session**. Do not substitute new derivatives for those historical originals or keep their old hashes in the manifest. This PR requires **only the newly owner-selected PNG**, with earlier approved atmosphere preserved in text and the existing atlas.

Original OpenAI-generated art; no novel excerpts, fan art or ripped sprites. Publication and playable-adaptation permissions remain governed by [rights-and-release](../04-production/rights-and-release.md).

## Remaining PR gates

- [x] Gym identity and owner-selected new standalone panorama documented
- [x] Proposed environmental interactions and boss encounter recorded as concepts
- [x] Original image metadata pinned in the manifest
- [ ] Upload the exact original PNG to this PR's branch
- [ ] Image checksum and relevant CI checks pass on final PR head
- [ ] Mark PR ready and merge after image upload and checks; playable encounter still requires independent implementation/feel review
