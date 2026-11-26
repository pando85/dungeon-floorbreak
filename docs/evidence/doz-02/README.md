# DOZ-02 evidence: Dozer damage states and the stop frame

Review renders and Bevy captures for the [DOZ-02 design spec](../../02-art/doz-02-dozer-damage-spec.md): the design keys, the execution frames and their wiring into room 3. The `doz02_bevy_*` files and the reversal GIFs are captures of the real window (`floor01_scene`, `--scene f01_late --f01-room dozer`, the `dozer:brace`, `dozer:retry` and `dozer:crew` pilots); the contact sheets, the lit scenes, the diagram and the glance GIFs are composed (the lit scenes use the A1 compositor, `scripts/env_kit_light.py`, on the Floor 1 kit).

Status:

- **Design keys:** drawn (spec section 6). Routing score 4 of 4, Hard (spec section 1).
- **Silhouette gate:** **PASS with notes** (an independent Claude Opus agent who did not draw the keys; verdict below and in spec section 11).
- **Execution:** done (Claude Sonnet): every frame of spec section 4, the export, the tests, this evidence and the wiring in `src/dozer_art.rs`; room 3 is at milestone `setpiece`. Questions for the designer: spec section 13 (DZ2-Q1 to DZ2-Q4).
- **Review:** pending, independent. **Owner art and game-feel review:** pending.

## Silhouette gate

Gate pack (design step): the lit before and after scenes at 1x, the key sheet at x4 and the spec.

| Question | Verdict |
| --- | --- |
| 1. Does the disabled machine read as the DOZ-01 Dozer, broken and harmless? | **PASS.** The silhouette below the stack is the DOZ-01 machine; at 1x the knocked stack, the steam puff, the dark yoke lamp and the barred wheel say "broken". The boiler crack, the cab dent and the 3 px tub crumple are 1 px detail that reads at 2x to 4x but only faintly at 1x (note G-1). |
| 2. Does the jammed wheel read as stopped and not lethal? | **PASS.** The spike ring drops from the brightest object on screen to a dull slate star, and the rusty tie bar through the spokes explains the stop. The three missing spikes register at 1x only at the top (note G-2). |
| 3. Does the remnant read as a fallen press frame? | **PASS.** Same rust beam, rivet row and end stencils as the raised key, now bowed and lying in the lane. The upward bow gives a slight speed-bump hump at 1x (note G-3). |

**Overall: PASS with notes** (independent Claude Opus agent, 1x first, then 4x and 8x crops). Notes G-1 to G-5 and how the execution step handled them:

- **G-1 (hull damage at 1x).** Quiet at gameplay scale; the "broken" read leans on the stack, the steam and the wheel. Unchanged: no brightness or new colour added. The Bevy stills (`doz02_bevy_*`) show it in the room.
- **G-2 (wheel).** The brass hub stays the brightest spot on the wheel and sits on the tie bar. Kept.
- **G-3 (remnant).** The remnant lies flat with a tight contact; the landing dust sits at both beam ends, 6 px up.
- **G-4 (composition artifacts).** The lit scenes are restaged (`MACHINE_O_X` and the staging comment in `scripts/make_dozer_doz02_review.py`): no wall torch or distant lamp lands on the cab, the stack, the yoke or the bar (measured: zero lamp-coloured pixels in the machine's lamp band). In the room the lighting is the room's own.
- **G-5 (stand-in driver).** The slump frames exist (`engineer_slump_0/1`): the pot over his eyes, no eye pixel, a 1 px breath; the lit after scene uses them.

## Regenerate

```sh
python3 scripts/author_dozer_doz02.py              # rewrites the grids, validator configs, manifests and the damage contract (--check verifies them)
bash scripts/export_assets.sh                      # builds the .ase, the atlases and the Floor 1 kit atlases the compositor needs
cargo build --locked --bin floor01_scene
bash scripts/make_dozer_doz02_evidence.sh          # the Bevy stills (2 passes, hash-matched) and the reversal frames under artifacts/doz02/
python3 scripts/make_dozer_doz02_review.py --reversal-frames artifacts/doz02/reversal_frames   # every other file here and SHA256SUMS
```

`SHA256SUMS` lists the hash of every file here except itself and this README.

## Files

| File | What it shows |
| --- | --- |
| `doz02_contact_x1.png`, `doz02_contact_x4.png` | Every DOZ-02 frame by atlas and tag, at 1x and 4x |
| `doz02_frames_x6.png` | Every execution frame at x6 with its pivot (yellow cross) and its tick count |
| `doz02_key_sheet_x4.png` | The design keys by gate item: live against jammed wheel, live against bent yoke (x4), the machine intact against `disabled` and the impact pitch (x2), the stop frame, the remnants and the brace (x4) |
| `doz02_state_diagram.png` | The frame and state diagram: the prop states with their frames and ticks, the machine's `committed_chase`, `impact_stunned` (48 ticks) and `disabled`, the miss branches (early remnant, rear glance), the room pieces, the haul route, the brace states and the crew |
| `doz02_lit_before_640x360.png`, `doz02_lit_after_640x360.png`, `doz02_lit_pair_x2.png` | Composed lit scenes at gameplay scale: the machine under the raised frame with the haul line to the brace and Carl at the brace; then the disabled machine (jammed wheel, bent yoke, slumped driver, thin steam and a black wisp) under the landed remnant, rider A on his back and rider B getting up in front of the wheel |
| `doz02_bevy_<state>_640x360.png`, `doz02_bevy_<state>_640x400.png` | Native captures of room 3 in the real window at 640 x 360 and at 640 x 400 (`--deck`), two passes byte-identical: `gate_closed` (tick 10), `gate_shake` (60), `brace_weakened` (306), `warning` (325), `fall` (348), `impact_pitch0` (353), `impact_pitch1` (359), `impact_tumble` (370), `impact_lying` (392), `disabled` (410) of `dozer:brace`; `gate_burst` (438) of `dozer:retry`; `crew_fight` (452) of `dozer:crew` |
| `doz02_bevy_contact.png`, `doz02_bevy_crops_x2.png` | The 640 x 360 stills at half size, labelled; and x2 crops of the machine and brace region for eight of the states |
| `doz02_capture_lines.txt` | One `DOZ02_STILL` line per still and size and one `DOZ02_GIF` line: the window's final COMBAT_STATE hash against the headless run's (`hash_match=yes`: the presentation changes no simulation hash), the two passes' byte identity, the partner (`cf02_free`) and the PNG's SHA-256 |
| `doz02_reversal_x1.gif`, `doz02_reversal_x3.gif` | The reversal of `dozer:brace` from just before the release (tick 312) to 60 ticks of `disabled` (461): one GIF frame per 60 Hz tick at 2 cs (so 0.83x real time), the release, the 30 tick warning with the shadow growing, the 8 tick fall, the 48 tick `impact_stunned` (pitch, spark, steam, tumbles, slump) and `disabled`. x1 is the captured frame cropped to x 0..420, y 70..310 (48 colours); x3 is a 230 x 122 crop at 3x (32 colours) to stay under the 3.5 MiB GIF budget |
| `doz02_glance_x1.gif`, `doz02_glance_x3.gif` | The rear glance, composed tick by tick (not a Bevy capture: the room has no late-release pilot): the frame falls, strikes the rear deck (spark not drawn), tips off behind the exhaust and the remnant lands behind while the machine keeps chasing with live parts. 88 ticks at 2 cs; x3 is a 270 x 190 crop at 3x. The falling frame descends at the footprint, as the presenter draws it (spec section 13, DZ2-Q2) |

## Weakest points (for the reviewer)

- The pitch shows 1 px stair steps along the deck bottom (design note N-1; integer column translation).
- The two riders overlap for about 15 ticks of the tumble (A overtakes B); `impact_tumble` shows one goblin cluster (DZ2-Q1 and the notes).
- The rear glance has no Bevy capture; its composed GIF shows the strike and the remnant, and the continuity question DZ2-Q2.
- The reversal GIFs are cropped and palette-limited (48 and 32 colours) for the size budget; the stills are the full-quality evidence.
- Hull damage and the cracks read faintly at 1x (gate note G-1); the quick levers (a second steam puff, a darker crack outline) were not used.
- The tunnel lintel tiles are drawn and tested but not wired (DZ2-Q4).
