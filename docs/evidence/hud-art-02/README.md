# HUD-ART-02 evidence (executed captures)

Real Bevy window captures of the HUD-ART-02 build (`floor01_scene`, debug, xvfb, `--no-audio --scale 1`), replacing the designer's mock composites in [`design-gate/`](design-gate/README.md). Spec: [hud-art-02-spec.md](../../02-art/hud-art-02-spec.md). Status: execution done, independent review pending, owner art review pending. Every new region is `prototype_fixture_unapproved`.

Every capture below ran with the window hash equal to the headless hash (`match=yes`, all lines in [`capture_lines.txt`](capture_lines.txt)); `:2p` plans also with the timed live-loop hash. The `--glyphs=keyboard` and the default (pad) run of the same room end on the same hash, so the glyph switch is hash-neutral.

## Commands

```sh
bash scripts/export_assets.sh                      # atlas incl. the ten new regions and the new notice plate
BIN=./target/debug/floor01_scene
# Carl's plate, darkest (hazards) and brightest (dozer) room, 640x360 and --deck (640x400), keyboard and pad path
scripts/room_capture.sh hazards hazards:carl_solo:0 240:1:1 OUT --f01-loadout all [--glyphs=keyboard] [--deck]
scripts/room_capture.sh dozer   dozer:run          240:1:1 OUT --f01-loadout all [--glyphs=keyboard] [--deck]
# Donut's human plate in a room (a :2p plan; with --glyphs=keyboard only P1's plate gets the caps)
scripts/room_capture.sh hazards direct:2p 240:1:1 OUT --f01-loadout all [--glyphs=keyboard] [--deck]   # the darkest room
scripts/room_capture.sh boiler  direct:2p 240:1:1 OUT --f01-loadout all [--glyphs=keyboard] [--deck]
# GOBLIN BOOM on the Hoarder's defeat (story loadout, grant on tick 1235, the frame is 40 ticks in)
scripts/room_capture.sh hoarder hoarder:carl_solo:0 1275:1:1 OUT [--deck]
# Donut's blink states and the PUDDLE JUMPER notice (party fixtures; window run and headless run compared by hand)
floor01_scene --no-audio --no-record --scale 1 [--deck] --party-fixture donut_prog_arena_blink_tour --capture-sequence TICK:1:1 --capture-dir OUT   # TICK 30 ready, 43 windup (muted), 200 cooling, 590 ready again
floor01_scene --no-audio --no-record --scale 1 [--deck] --party-fixture donut_prog_grants_2p --capture-sequence 430:1:1 --capture-dir OUT          # PUDDLE JUMPER
python3 scripts/make_hud_art_02_evidence.py sheet docs/evidence/hud-art-02   # contact sheets
python3 scripts/make_hud_art_02_evidence.py at2 OUT.txt FRAME:ROOM ...       # AT-2
```

## Files

| File | Shows |
| --- | --- |
| `new-regions-sheet-1x.png`, `new-regions-sheet-8x.png` | the eight key caps and two chips at 1x and 8x, under the existing pad disks |
| `carl-{hazards,dozer}-{360,400}-{kb,pad}-{1x,crop3x}.png` | Carl's plate with caps (`kb`, `--glyphs=keyboard`) and with the default Xbox glyphs (`pad`): full frame at 1x and a 3x crop of the plate. Mean world luminance (rows 52 to h-56): `hazards` 43.7 / 42.2, `dozer` 75.4 / 70.4 (360 / 400) |
| `two-player-{hazards,boiler}-{360,400}-{kb,pad}-{1x,crop2x}.png` | both plates in a `:2p` room (`hazards direct:2p`, the darkest room, and `boiler direct:2p`): Donut's human plate with the 72 px track and `BLINK` (ready), and with `kb` only Carl's (P1's) plate has caps, Donut (P2) keeps her pad glyphs |
| `donut-blink-{ready,windup,cooling,ready2}-{360,400}-crop4x.png` | Donut's plate: ready (amber label, untinted socket), windup (muted: `amber_dim` label, tinted socket with the zig-zag), cooling (muted with the 1 px underline), ready again (with the brass ready flash). `donut-blink-{ready,cooling}-*-1x.png` are the full frames |
| `notice-goblin-boom-hoarder-{360,400}-{1x,crop3x}.png` | `GOBLIN BOOM` with the dynamite well, over the Hoarder's room |
| `notice-puddle-jumper-fixture-{360,400}-{1x,crop3x}.png` | `PUDDLE JUMPER` with the puddle well (fixture `donut_prog_grants_2p`, mean luminance 39.3 at 360: dark), with the redrawn `chip_puddle` (one drop over a flat puddle, review F1) |
| `at2_rim_contrast.txt` | AT-2 on all nine f01_late rooms |
| `capture_lines.txt` | every `ROOM_CAPTURE` and fixture line |

## Honest gaps (what the brief asked and what could be captured)

- **No room pilot presses the blink.** Headless events of `hoarder`, `boiler`, `hazards`, `mastery`, `juicer`, `dozer` and `corral` plans (`:2p` where valid) hold no `special_started`, so Donut's muted and cooling states in a room cannot come from a room pilot. They are captured from the party fixture `donut_prog_arena_blink_tour` (real HUD, press tick 40, cooldown 540 ticks); the room frames show the ready state, `BLINK` fully drawn on Donut's human plate in `hazards direct:2p` (the darkest room) and `boiler direct:2p`. `dozer` has no `:2p` plan (`dozer:*` are solo plans), so its frames show Carl's plate only.
- **`PUDDLE JUMPER` in a room does not show in a capture.** In the juicer room both grants happen on tick 3175 and `HEAVY FIST` shows first, then `PUDDLE JUMPER` 166 ticks later, but after the room is complete the scene tick stops advancing (`tick_03255` in every frame name) and the notice clock stops with it: frames requested up to tick 3478 all show `HEAVY FIST`. The puddle notice is therefore captured from the grant fixture. Whether the second grant notice is ever shown in live play after the boss falls is **not verified** here (a pre-existing behaviour, not changed by this PR); it should go to the owner's playtest.
- `GOBLIN BOOM` was captured in the Hoarder's room (the room that grants it, mean luminance 73.9), not in a near-black room. The notice is drawn over the near-black foundation band in every room, which AT-2 measures directly.

## Acceptance targets (HUD-ART-02 spec section 8)

| ID | Result |
| --- | --- |
| AT-1 | PASS (human check of the captures): at 1x in `hazards` and `dozer` at 640x360 and 640x400 Carl's plate shows the caps and the `BOOM` label right-aligned to +157; Donut's human plate (`hazards direct:2p` and `boiler direct:2p`, and the fixture) shows `BLINK` fully drawn, the eight mana segments countable and the label aligned to +157 like `BOOM` |
| AT-2 | PASS: the `metal_hi` rim measures 4.72 to 5.10 : 1 against the median colour of a 2 px band around the notice rect in all nine f01_late rooms at both sizes (`at2_rim_contrast.txt`; method: the 2 px band just outside the rect in a capture without a notice, so the world under the plate is what is measured) |
| AT-3 | PASS: unit test `every_socket_label_fits_beside_donuts_track` (`BOOM`, `BLINK` at most 33 px; `BLINK` is 27 and starts at +130) |
| AT-4 | PASS: `pixel_grid_to_ase.py lint --strict` on every HUD grid, palettes HUD tokens only (`tests/test_art_hud_hybrid_a.py`) |
| AT-5 | PASS: cut corners (exactly four transparent pixels), two `metal_dark` skirt rows and the `metal_hi` bevel row on all eight caps (`test_caps_differ_from_the_pad_disks_by_silhouette`); the disks have many more transparent corner pixels |
| AT-6 | PASS: every capture line says `match=yes`; the `kb` and `pad` runs of each room end on the same hash; the whole `cargo test --locked --all-targets` and `ci_floor01_combat.sh headless` are green with every committed replay and golden byte-identical |
| AT-7 | PASS by diff: between the `kb` and `pad` frames the only differing pixels are Carl's ability row (x 48..166, y 42..53), so with no keyboard input the glyphs are the unchanged default Xbox set; the expected presentation changes against earlier stills are Donut's human plate (72 px track, `BLINK`) and the notice plate. A pixel diff against a `main` build was NOT RUN |
| AT-8 | PASS: unit tests in `pad_slots` (`a_non_neutral_keyboard_tick_switches_...`, `a_tick_with_both_inputs_keeps_the_current_set`, `p2_never_shows_keyboard_glyphs_...`) and `hud` (`the_forced_keyboard_set_applies_to_p1_only`); the `two-player-boiler-*-kb` frames show P2 on pad glyphs |
| AT-9 | PASS: `the_item_well_is_drawn_only_for_the_two_special_grants` and `well_headlines_end_at_or_before_plus_179` |
| AT-10 | Design-stage gate was PASS on the mock composites; on the real captures `key_j` reads as a keyboard key, `chip_dynamite` as a stick of dynamite and `chip_puddle` as a magic puddle (human check; independent review to confirm) |

## Existing evidence stills that now differ (expected presentation change)

Any committed still or GIF that shows **Donut's plate, human or AI companion** (the AI plate's mana track changed too; her mana track was 96 px outer with 12 px segments and no `BLINK` label, now 72 px outer with 7 px segments and the label) or a **grant or other System notice** (the frame is now the lit-rim plate, and `GOBLIN BOOM` and `PUDDLE JUMPER` gain a well) predates this PR, notably under `docs/evidence/donut-prog-01`, `docs/evidence/f01-playtest` (the notice stills and the grant GIFs) and the HUD evidence of `hud-hybrid-a-01`, `hud-a11y` and `audience-01`. They are history, not goldens; none is regenerated here. No pixel hash, replay line or golden moved: the committed `COMBAT_STATE` hashes and replay lines are simulation hashes and the HUD does not feed them.

## Owner playtest checklist (items this PR leaves to a human)

- **Live device switch (keyboard, then pad, then keyboard).** The captures force the keyboard set with `--glyphs=keyboard`, which bypasses `PadSlots::take_inputs`, so the live switch is covered only by the `pad_slots` unit tests. Check on a real pad: P1's plate shows caps after a key press, the pad's glyphs after a pad press, caps again after a key press, and a tick with both keeps the current set; P2's plate never shows caps.
- **`PUDDLE JUMPER` not shown in the juicer room (likely a capture artifact).** In the juicer room captures `HEAVY FIST` stays up and the second grant notice never appears (the frame names freeze at `tick_03255` after the room completes). The HUD clock steps on every call, so live play most likely shows the second notice after the boss falls; this is not verified. Play the juicer room to the defeat and check that `PUDDLE JUMPER` follows `HEAVY FIST`.
