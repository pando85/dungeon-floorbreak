# Character native pixel-scale approval — 2026-11-16

**Decision:** ART-SCALE-2026-11-16-01  
**Status:** **OWNER APPROVED** after reviewing the actual native-scale sprite studies and cast comparison. This is a human decision on the **selected pixel-art scale and current study baseline**, not a claim of final production/gameplay readiness.

## Locked baseline sizes

| Character / stage | Approved opaque height | Selected existing editable master | Manifest and frame-tagged animation | Other study options |
| --- | --- | --- | --- | --- |
| Princess Donut — Floor 1 | **40 virtual pixels** | `assets/source/characters/donut/donut_h40.ase` | `assets/manifests/donut_scale_h40.json`, four-frame `idle` | 32 / 48 px retained for comparison |
| Carl — Book 1 | **52 virtual pixels** | `assets/source/characters/carl/carl_h52.ase` | `assets/manifests/carl_book1_h52.json`, four-frame `idle` | 48 / 56 px retained for comparison |
| Mongo — adult / mount | **76 virtual pixels** | `assets/source/characters/mongo/mongo_adult_h76.ase` | `assets/manifests/mongo_adult_h76.json`, four-frame `idle` | 64 / 88 px retained for comparison |

The `hNN` suffix denotes **opaque silhouette height**, not the full Aseprite canvas dimensions (which can contain transparency and room for animation). Preserve manifest pivots, palette mappings, source layers and deterministic `pixel-grid-v1` masters. Never mass-resize a different candidate to achieve the selected size.

**Rationale / knowingly accepted tradeoff:** the 40 px Donut version preserves her golden eyes, Persian muzzle, fluffy face and adorable personality better than the smaller 32 px version, although the plume makes her unusually wide alongside the 52 px Carl. The owner reviewed that visual tradeoff and approved **40 px**. Carl 52 retains the ordinary antihero silhouette while remaining expressive; Mongo 76 preserves crest/mask/feather and adult-mount readability without the footprint of the 88 px option.

## Approval boundaries

**Approved:** relative baseline sprite sizes and selection of these three pre-existing native-resolution studies as the direction from which the next art iterations should proceed. The selected `idle` prototypes exist; their existence does not certify final expressive or movement animation quality.

**Not yet approved or completed:** Bevy runtime camera/play-scale rendering of these specific sprites; multi-character collision/occlusion and combat visibility; Carl/Donut side-profile `run` or jump sets; Mongo charge/locomotion; Donut riding Mongo and seat socket; VFX contrast; actual Steam Deck hardware checks; final palette coherence; detailed book equipment chronology; playable game distribution rights.

The earlier agent-written recommendations inside the three comparative studies remain **historic analysis**; the human decision above takes precedence, including where Donut's agent preferred 32 px after a cast-width assessment. Do not silently switch to 32 px or modify any approved size without renewed owner review.

## Next work orders

1. In a **separate PR**, present the three approved sprites together at **native 640×360 and 640×400** on the same floor; inspect 1× and integer 2× scene views. If practical, render them in the actual Bevy runtime, explicitly distinguishing it from a composed PNG.
2. Produce genuine lateral locomotion and jumping frames from each approved master (start with Carl and Donut; Mongo can follow), keeping feet/pivots and face/coat/crest identity stable. Review silhouette overlap during action.
3. Test a Donut 40 px / Mongo 76 px mounted mockup, head/crest separation and magenta spell effects before choosing a gameplay mount socket or final VFX.
4. Keep the fan-character/source rights gate in `docs/04-production/rights-and-release.md` unchanged. Art approval is **not** legal release clearance.

**Evidence:** [Donut](../evidence/donut-floor1-scale-study/README.md), [Carl](../evidence/carl-book1-scale-study/README.md), [Mongo](../evidence/mongo-adult-scale-study/README.md). See their individual [Donut](donut-scale-study.md), [Carl](carl-scale-study.md), and [Mongo](mongo-scale-study.md) reports for rejected candidates and remaining visual weaknesses.
