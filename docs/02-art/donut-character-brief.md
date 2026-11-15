# Princess Donut — lore-grounded visual direction and acting bible

**Status:** Owner-selected **F-based personality direction** (2026-11-15) with lore audit; **NOT** a finalized canonical sprite or asset reference. New concept images are intentionally paused in favor of a single coherent sprite master and expression studies.
**Decision scope:** Proposed look and silhouette of `caster`'s replaceable *Dungeon Crawler Carl* fan-content character mapping. Artwork approved for public concept publication only when the owner explicitly approves the particular image. Do not infer game or standalone content distribution rights.

## Cross-checked physical traits (secondary book references; primary-text audit still required)

- **Species/breed:** domestic Persian **tortoiseshell** show cat; quadrupedal, not a humanoid mage or anthropomorphic sorceress.
- **Shape:** small and plump/compact, especially plush long coat, very flat short-snouted Persian face, small wide-set ears, long fluffy tail. Silhouette should remain recognizably feline even with accessories.
- **Color:** motley **black, beige and white tortoiseshell** coat, not an all-white, all-purple, uniform gray, or striped/tabby cat; avoid generic tabby pattern. The highly specific coat placement can be an art-direction choice and must remain stable across frames.
- **Eyes:** large **yellow** eyes, important for expression and readability.
- **Personality:** pre-dungeon champion show cat, vain, imperious and theatrical; subsequently intensely loyal, clever, dangerous and increasingly capable of leadership, still with aristocratic vanity.
- **Distinctive adornments (story-point dependent):** small purple-centered jeweled crown/tiara, butterfly charm at collar, silver fae-scale crupper over hindquarters. Treat accessories as *equipable narrative states*, not a permanent fused body part. Additional hats and magical items require separate evidence for their timeline.

**Evidence starting points:**
- [Dungeon Crawler Carl Wiki — Princess Donut appearance](https://dungeoncrawlercarl.wikioasis.org/wiki/Donut)
- [DCC fan profile — first-book visual equipment](https://www.dungeoncrawlercarl.nl/en/characters/princess-donut/)
- [BookExplained — Princess Donut character timeline](https://bookexplained.com/dungeon-crawler-carl/characters/princess-donut/)
- [Donut profile from novel character references](https://bookworm.wiki/dungeon-crawler-carl/characters/princess-donut)

These are **secondary sources**, with occasional contradictions among fan wikis. They are useful for a concept brief; any lore-canon assertion in a final asset must be checked against the actual book and location before marking `canon verified`.

## Shared creative direction

- Match the approved gothic/vibrant world: cold blue and purple architectural depth, hot orange blasts, expressive magenta magic; natural black/beige/white coat should stand out through **edge light and shape**, not be bleached or recolored.
- Keep the mandatory [1980s/1990s everyman antihero action tone](../01-design/antihero-tone.md), *but adapt it to a proudly self-styled feline celebrity*: dry comedy, ridiculously aristocratic bearing in deadly environments, moments of fury, sincerity beneath narcissistic public performance.
- **No cat wielding a human-sized staff**, no humanoid hands, bipedal pose by default, human boots/robes, or oversized superhero armor. Design combat FX around paws, face and body direction as a separate readability/gameplay decision, not established canon.
- Donut is physically **much smaller than Carl**; avoid letting detail render her larger than the `demolitionist` when they share a game scene. Current virtual height hypothesis ~28–36px is provisional and must be playtested at 640×360; verify on Steam Deck and integer-scaled 4K.
- Shape language: squat four-legged body, rounded flat Persian face, small widely spaced rounded ears, expressive yellow eyes, plush collar, thick tail. No random coat recolors between animation frames.

## Selected direction: one body, five acting modes (owner direction)

The owner selected **F: the manipulative sweetheart** as Donut's default design because it reads **charismatic, intelligent, adorable, theatrical and capable of calculated charm**. **H, I, J and K are not separate models or physical transformations:** they are acting references to apply to the **same F-based cat**, with exactly the same tortoiseshell pattern, proportions, eye color and accessories appropriate to the floor. This is a direction decision; the final small pixel-scale master still requires approval.

| Reference | Acting mode | Face/body/tail performance | Narrative/game trigger | Prohibited shortcut |
| --- | --- | --- | --- | --- |
| **F** | Everyday diva, primary base | Slightly lifted chin, composed eyes, plush tail up, charming expression with subtle conceit | Idle, travel, social banter | Permanently sleepy or evil-looking |
| **I** | Calculated sweetness | Open eyes, minimal head tilt, gentle purring/nuzzle gesture, tail curved close | Persuasion, flattery, deliberately winning someone over | Magical pink heart particles as a universal or lore-canon effect |
| **H** | Smart strategist | Narrowed focused eyes, forward ears, attentive stationary tail, controlled posture | Planning, scanning an encounter, concentrating on a spell | Human-like schemer with hands, chess props or a literal human intelligence disguise |
| **K** | Protective fury | Fur poof, ears pinned, visible fangs, forward stance; tail expands dramatically; magenta VFX belongs to equipped spell | Rare, justified threat to loved ones, combat climax, fury/reaction | Default villain form, permanent dark aura, gratuitous cruelty |
| **Vulnerable (new)** | Fear, grief and abandonment | Ears lowered, eyes softened, minimal theatricality, tail down and still, curls close to trusted companion | Quiet emotional scenes with Carl/Mongo, scenes about abandonment | Making her continuously helpless or babyish |
| **J** | Earned regal leader **(optional sixth mode)** | Poised stance and self-command, economical elegant movement, unwavering gaze | Important victory, rally/command moment | Physically larger queen form or mandatory long regal cape |

**Important:** Her anger and protective behavior are not an independent evil personality or a magical form change. The same adorable cat should remain recognizable when she is furious. Darkness is a situational *reaction to real stakes*; vulnerability is what gives that contrast meaning.

### Three independent layers for animators and code

1. **Identity layer (fixed):** one exact color-patch map, face and eye shapes, four-legged anatomy, body/tail size, attachment points. A stable sprite master with constrained overlays or manually consistent frames.
2. **Performance layer (selectable):** facial cues, ear/tail posture, fur-ruffling, fangs and idles. Five primary acting modes plus an optional leader pose. VFX and text bubbles must not replace essential tells.
3. **Equipment/ability layer (story-driven):** tiara, silver butterfly charm, fae-scale hindquarter crupper, optional sunglasses or specialty equipment, spell FX. These obey the specific floor/book, not emotion. **Do not attach anger to an automatic gameplay damage bonus** unless explicitly designed and separately playtested.

The **Former Child Actor** class and its **Character Actor** skill offer a powerful thematic parallel: Donut excels at roles and can gain varying floor-based class specializations. We can reflect that with theatrical poses and staged presentation without inventing an emotion-based stat system.

### Narrative evidence and emotional depth

- **The adored performer who masks fear:** high-charisma show-cat background, concern with public image, performance and applause; her personality is not a nonstop comedy gag.
- **The calculating planner:** her story contains deliberate class and management choices, not merely cute accidental victories. See [Former Child Actor class index — Book 2, Chapter 1](https://www.dccfansite.com/entry/former-child-actor/).
- **The emotional center:** abandonment and loyalty are important, and her confrontation with Beatrice is a major anchor for the vulnerable state. See [Donut chronology and Beatrice — Book 5](https://manatoastmedia.com/dcc-donut/). **Contains spoilers.**
- **The formidable mage:** magical power can be frightening without making Donut visually evil. Cast gestures must remain compatible with real quadruped anatomy and [book-indexed skills](https://loremark.app/series/dcc/characters/princess-donut?book=8&chapter=88&tab=skills).

These linked indices are third-party summaries, **not** direct access to the original novels. Exact chapter references and ability timings need primary-text checks before labeling an item `canon verified`; the art bible must distinguish `source-backed secondary`, `artistic decision`, and `gameplay proposal`.

## What is frozen, what is not

- **Direction selected:** F's cute, smart, manipulative personality; K's forceful anger as a situational performance; H's calculation, I's charm and vulnerability all required.
- **Not frozen:** exact face/coat patch topology, hair/fur highlight simplification, crown/body-accessory geometries, palette hex IDs, animation frame counts, per-book outfit schedule, and any complete new image master.
- **No more broad concept variations.** The next art step is a *single F-based identity turnaround + consistent expression sheet* at practical size. Reject prompts that generate a different cat in each emotional mode.
- **Project scale:** provisionally 28–36px tall for Donut on 640×360. Test against Carl at 48–56px, 4K integer 6× scaling and Steam Deck view; adjust both for readable quadruped silhouette before committing to Aseprite source.
- **Repo/publication:** store concept references only when individually human-approved, accompanied by provenance and permission scope; no franchise-derived playable game distribution inferred.

## Long-term progression plan (do not misstate book canon)

Evolve mostly through **gear, demeanor, acting and class-specific magic**, not dramatic bodily growth:

1. Early: glamorous, vain, mildly alarmed show cat and tiara.
2. Developing: learns to fight and coordinate but remains extremely concerned with elegance and recognition.
3. Veteran: confident tactical caster, emotionally complicated, theatrically furious.
4. Leader: capable of commanding others while still unquestionably the same small, fluffy diva.

Do not map equipment or specific powers into individual books until their chronology has been checked. A future four-stage reference board should build from **one** approved tortoiseshell coat map, rather than independently generated cats.

## Production gate after owner selects style

- Approve front, profile, back and 3/4 pose turnaround, showing stable color patches and anatomy.
- Demonstrate small idle/run/cast expressions at the virtual game pixel size against the real castle tiles and next to the **approved Carl reference**.
- Create editable Aseprite `.aseprite` source, tagged animation frames, source palette, pivot, collision/FX metadata and reproducible exports. Do not use a concept render as a spritesheet.
- Record the selected approved image, hash, provenance and the permission scope before public upload.
