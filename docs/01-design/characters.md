# Character sheets — fan-concept layer only

**IMPORTANT:** Carl, Princess Donut and Mongo are franchise references for *unlicensed, noncommercial design exploration*. Mechanics are original proposals, not canonical claims. Runtime identifiers must remain IP-neutral to permit a future original-IP release.

## Character 1 — Carl → `demolitionist`

- **Fantasy:** Improvisation under pressure; physical confrontation; bombs, tricks, deliberate collateral damage.
- **Silhouette:** ordinary, visibly human proportions, unremarkable face and slightly awkward/irritated posture at the beginning; feet/stance still readable at gameplay scale. Preserve the canon-specific footwear/barefoot transitions after verification; **do not default to boots**. Muscle gains are modest and gradual across book phases, not superhero anatomy. Later charisma comes from composure, weary expressions and lived-in gear rather than a Hollywood face or power fantasy.
- **Mandatory art and narrative tone:** [`80s/90s everyman action antihero`](antihero-tone.md). A disaster survivor who improvises, swears and gets hurt; avoid heroic poses, instant mastery and polished tactical-hero outfits. Confirm appearance and loadout against book/floor/chapter before tagging as lore canon.
- **Baseline:** reliable ground movement, grounded close-combat/utility actions and a plantable timed explosive. Any ranged capability or prop requires a separate design and lore review; it must **not** be represented by a shotgun.
- **Special:** bomb with clear fuse, visible blast radius, material-specific damage, self-protection skill expression.
- **Weakness:** can't instantly reach high platforms; aim/placement risk; exposed while planting.
- **Required animation tags:** idle, run, rise, fall, land, attack, plant, hurt, death, celebrate.
- **Owner-approved negative equipment rule (2026-11-15): Carl never uses a shotgun.** The shotgun shown in a cinematic visual-target image was illustrative noise, not accepted character design, equipment, game mechanics or book canon. The runtime must not couple Carl to a default firearm loadout.
- **Do not define exact final equipment/art without human approval; art publication is covered by `ART-PUBLICATION-ALL-V1`, game/asset-pack distribution is not (see rights register).**

## Character 2 — Princess Donut → `caster`

- **Fantasy:** Self-important, charismatic, magical ranged specialist; high mobility and visual personality.
- **Silhouette:** **tortoiseshell Persian** show cat with long, fluffy black/beige/white fur, compact quadruped body, flat face, large yellow eyes, small ears and bushy tail. This corrects the earlier generic white/light-fur placeholder; retain fixed coat markings across frames and contrast via lighting/outline, not recoloring. See the [lore-grounded Donut visual brief](../02-art/donut-character-brief.md), pending owner approval of a specific concept.
- **Combat-role baseline after appropriate story unlocks (proposal):** quick traversal, optional double jump, ranged basic spell, short-cooldown control effect. **Important progression boundary:** Donut is playable from the first moment in 2P, but is **not** a spellcaster before the story grants magic. Pre-unlock claw/bite/evasion/small interactions are exploratory game-mechanic candidates, not verified canon. Exact unlock timing and spell identity require book-chapter evidence.
- **Special:** environmental modifier (e.g., freeze brittle props or shove foes), intentionally synergistic with bombs.
- **Weakness:** low structural damage and limited defense while casting.
- **Required animation tags:** idle, run, rise, fall, land, cast, hurt, death, emote; modular tail/ears/face where good.
- **Acting direction (owner selected 2026-11-15):** F is the base: adorable, self-important, clever and capable of manipulation. H = calculating, I = deliberately sweet, K = situational protective fury, J = regal/leadership, plus a separate quiet vulnerable pose for abandonment/loyalty scenes. All share **one** four-legged, fixed-coat sprite identity; never generate five physically different Donuts or treat anger as a villain transformation. Details: [Donut acting bible](../02-art/donut-character-brief.md).
- **Readability:** facial expression cannot carry essential telegraph alone; animation silhouette and FX must communicate it.

## Character 3 — Mongo → `charger` (later milestone)

**Owner-approved concept direction:** expressive pink/red-and-blue feathered raptor, gold eye, crest, long tail, adorable hatchling → lanky juvenile → athletic D+B adult who can carry Donut. The artistic progression is approved, not the book timeline or mechanics. See [Mongo's character bible](../02-art/mongo-character-bible.md) and [lore tracker](../02-art/mongo-lore-research.md); the three approved source images are committed and Git blob identity-verified, with a SHA-256 manifest.

- **Fantasy:** comedic, high-energy kinetic companion with violent momentum; third major character enters after the established duo.
- **Baseline:** AI companion or triggered allied charge first. Do not promise a third local player at launch.
- **Special:** break brittle columns, scatter clustered enemies, interrupt elite attacks.
- **Constraints:** charge needs obvious start/stop frames and safe recovery path; unbreakable critical geometry protected.
- **Required animation tags:** idle, run, anticipation, charge, impact, hurt, celebrate.

## Open questions

| ID | Question | Recommendation | Status |
| --- | --- | --- | --- |
| CHAR-Q-CELEBRATE | The "Required animation tags" above list `celebrate` for Carl, Donut and Mongo, while CF02-Q74 ([tone limits](combat-fun-02-post-demo01-proposals.md)) bans "poise, flips or victory poses" for Carl, [`cf02-acting-spec.md`](../02-art/cf02-acting-spec.md) lists victory poses and fists raised above the head (they read as cheering) under "Explicitly avoided", and the [Carl bible](../02-art/carl-character-bible.md) allows `victory` only as "disbelief/irony more than conquering-hero chest-puffing". The two texts conflict, and no `celebrate` animation exists in the shipped Carl sheets. | Keep the tag but redefine it for Carl: a disbelief or irony beat per the Carl bible, with no victory pose. Decide Donut's and Mongo's `celebrate` separately under their own bibles. | **Open**, recorded 2026-11-28 by the stale-docs audit; not resolved here, owner or top-tier design decision needed |

## Balance matrix (qualitative first)

| Ability | Demolitionist | Caster | Charger |
| --- | --- | --- | --- |
| Close-range burst | High | Low | Very high |
| Ranged pressure | Medium | High | None |
| Movement | Medium | High | Medium, momentum-driven |
| Terrain disruption | High | Indirect | High for brittle structures |
| Precision control | Medium | High | Low |

**Validation rule:** design a single room that becomes easier by combining abilities, but can be traversed solo without any permanently missing move. A later unlock may provide a shortcut; it may not create a retroactive softlock.
