# HRD-CUES-01 design gate: the stir and "not yet" keys in the room

Design composites for [the HRD-CUES-01 spec](../../02-art/hrd-cues-01-spec.md). The cue frames (`hoarder_mound` 3..5, `hoarder_post_b` 3 and 4) and the code-drawn motes the spec defines are pasted onto committed window stills of the Hoarder room ([f01-hoarder-presentation](../f01-hoarder-presentation/README.md): `dormant`, `reveal2`, `out`, 640 x 360, 1:1 world px). The sprite positions were found by matching the committed frames against the stills; sprite colours are mapped to the colours the window drew for them. These are **not runtime captures**: the presenter does not draw the cues yet, and the runtime evidence (window stills with `window_hash == headless_hash`) belongs to the execution step.

Regenerate: `python3 scripts/author_hrd_cues.py --gate docs/evidence/hrd-cues-01-design`.

| File | Content |
| --- | --- |
| `gate_1x.png` | Four full 640 x 360 stills at 1x: today's dormant still (no stir), the S1 stir (`stir_heave`, breath phase 5, two grit motes), the `shored` post during the Reveal (`reveal2`), the `give` post on her out still (age 2, four grit motes) |
| `gate_x3.png` | Enlarged x3 crops. Row 1, the mound's visible part at the right screen edge: today, S1 heave (phase 5), S1 slump (phase 9), S2 heave_hi (phase 6), S2 slump (phase 12). Row 2, the post: today's `intact` in the Reveal, `shored` in the Reveal, `give` at ages 0 and 4, `intact` after the beat |

Readability notes at 1x: the heave lifts the crown, the pipe, the brass frame and the crate rigidly by 2 to 5 px and the grit motes read against the brick wall; the effect reads in motion, a single still shows it only next to the rest frame. The shored post is visibly darker and banded next to the lit stack; the give beat reads as the straps springing. The S2 heave is the strongest frame and is meant to be.
