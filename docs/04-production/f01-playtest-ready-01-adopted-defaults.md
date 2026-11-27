# Floor 1 playtest, adopted defaults and open questions (PLAYTEST-READY-01 appendix)

Every adopted-default ID that entered the repository after `bebd5948` (the `docs(f01)` refresh of #269), one line each: the ID, the adopted value in one line, the merged PR that adopted it and where to change it. The owner's entry point is [f01-playtest-ready-01.md](f01-playtest-ready-01.md); its section 6 lists the defaults most likely to be felt. The value is the first cell of the spec row (shortened, never reworded); the spec row carries the reason. **Every value here is an adopted default, not an accepted decision**: the owner reviews game feel and art after merge and may change any value in a follow-up PR. Generated from the merged specs at head of main when the packet was written; if a spec row changes later, the spec is authoritative.

Families that are art specs change in their kit JSON (placement) or authoring script (pixels); gameplay families change in the data file named in the last column, or in the named constants where no data file exists yet.

## Adopted defaults (AD and IMP IDs)

### F01HUD (1)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| F01HUD-AD1 | The location strip shows the manifest room title and the attempt; the TEST LOADOUT tag shows only under `--f01-loadout all` | #270 | `src/combat_scene.rs` (`CombatState::hud_try`) and the HUD location strip |

### F01RUN (2)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| F01RUN-AD2 | Under `--f01-loadout all` `carry_for` returns nothing: the test loadout carries nothing | #270 | `src/f01_run.rs`; `assets/levels/f01_late/manifest.json` (room order) |
| F01RUN-AD3 | `--f01-run` takes no `--cf02-seed`; `--f01-carry` and `--f01-run-tally` need `--f01-run`; `--f01-carry` is refused under `--f01-loadout all` | #270 | `src/f01_run.rs`; `assets/levels/f01_late/manifest.json` (room order) |

### RINGW (2)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| RINGW-AD1 | Both Ring rooms are listed in the manifest at milestone greybox, between hazards and the Juicer | #271 | `assets/levels/f01_late/manifest.json`; `src/f01_ring.rs`; `src/combat_scene/env.rs` |
| RINGW-AD8 | The static shutter syncs as one block (its footprint), not 17 slices of 4 px | #277 | `assets/levels/f01_late/manifest.json`; `src/f01_ring.rs`; `src/combat_scene/env.rs` |

### LDW (5)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| LDW-AD1 | Parts without a frame-0 origin (the llama gland) start at the throat; the lava drops leave from there too | #272 | constants in `src/death_present.rs` |
| LDW-AD2 | Two wool tufts replace the goblin rags at every preset | #272 | constants in `src/death_present.rs` |
| LDW-AD3 | One knocked-out tooth flies at every preset of the explosion | #272 | constants in `src/death_present.rs` |
| LDW-AD5 | The decap jet and lava leave pose 4's jet base while the buckle shows and pose 5's neck from D+10, never above the lying body | #272 | constants in `src/death_present.rs` |
| LDW-AD6 | The duct lava dribble is slow (vx +-15, vy 40-90 px/s); explosion lava fan 30-150 deg, 140-260 px/s | #272 | constants in `src/death_present.rs` |

### DZ (25)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| DZ-AD1 | Placement: room 3 of the slice, between the Boiler and the Floor Lies (the Ring rooms 4a/4b follow the Floor Lies), with pin `f01_pin_post_pile_boss` (dynamite held) and no grant. | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD2 | K5 amendment: no `BossThreat` supplier in a Dozer room; the AI rule is the three layers of section 7, with CA2-DZ pre-registered as the named CA2 append and triggered by the counts of section 7.5 | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD3 | No charge in room 3; the charge sub-machine is CHASE-01's append | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD4 | The owner's R2 elevated bypass becomes the low-lintel escape tunnel, behind a buffer stop that is drawn but not a solid (DV-1) | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD5 | Push box 160 px tall, horizontal push-out only; the pinned rule is kept | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD6 | Wheel contact: circle first, then the token of a Dozer-owned persistent `Wheel` volume (not in the env set, `hurts_enemies` false); damage 255 via `hurt_hero`; never evadable | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD7 | Front impact collider (x +19..+172, y 0..120): only a landing on the tub or the wheel disables; a rear-hull landing is a drawn glance and a miss. | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD8 | A missed frame's 16 px remnant is overrun, never a barrier | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD9 | The brace takes Carl's melee, missiles, launched bodies and blasts (DV-2) | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD10 | No HP to lose (a fixed `hp()`, `alive()` always true), no health bar, no `EnemyDefeated`; the disable is a `CombatEvent::Dozer` sub-event; the jammed wheel is no longer lethal | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD11 | Crew: two carried roster goblins dismount on the disable; the driver stays in the cab | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD12 | PR split amends plan section 4.5 file ownership: the room and sidecar files move from DR-A into DOZER-CORE, because plan acceptance 3 asks for K5 fixtures *in the revenge room*. | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD13 | DR-A wires the approved DOZ-01 rig as the presentation (no box stand-in for the machine); only the frame, the damage states and the environment pieces are labelled stand-ins until DOZ-02; | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD14 | No camera rule change; fairness is the `dz_visible_lead` metric | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD15 | The gap is measured to the rearmost living hero ahead of the axle, with the cruise fallback of DZ-Q25 | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD16 | The wheel never hurts a hero outside the sim view | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD17 | Room id `dozer`, scene `f01_dozer` (not the plan's `f01_dozer_revenge`, DV-6), title "The Machine Remembers" (the doc's own title, IP-neutral); milestone `greybox` at DR-A and `setpiece` at DR-D. | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD18 | Display and data call it "a Dozer of the same family" (the doc's label); engine names stay generic (`dozer`, a tracked pursuit machine), with no franchise name in types or data keys | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD19 | One checkpoint (the entry); any party defeat retries from R0. The room is short, and the doc asks for fast retry | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD20 | The escaped wheel is lethal only during its 24-tick brake, and the machine feeds only its Active row once it leaves `committed_chase`. | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-AD21 | The rescue lands beyond the machine's reach, standing on the floor through `SlotOut.place` (section 7.3): an in-view lane spot ahead of the lead zone when it is safe, else the tunnel beyond `stop_x`. | #273 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-IMPL-AD8 | The wheel is lethal from the brake-start tick (`dzq14_lethal_in_brake`): 25 ticks in all (the last chase move plus 24 slide steps) | #279 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-IMPL-AD12 | Hull hurt rects are the push box plus the impact collider; Donut's swat and pounce hit only the first, so hits on the wheel or front collider do not clang yet (cosmetic in room 3) | #279 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-IMPL-AD14 | DV-8 'every hero reaches the exit' is read as room completion, frozen when the first hero overlaps the exit with every hero alive and ahead of the wheel | #279 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |
| DZ-IMPL-AD15 | 'Cruise <= 170' is checked on the cruise target and on the actual speed once it has been in the cruise band for 4 ticks | #279 | `assets/data/gameplay/enemy_dozer_v0.json` (`dzq*` keys); `assets/levels/floor01/f01_dozer.encounter.json` |

### EXIT (7)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| EXIT-AD1 | One new doorway sheet instead of stretching the 28 px room 5 grate or the 16 px Hoarder door | #274 | `scripts/author_danger_fx.py`; `src/room_exit_art.rs` |
| EXIT-AD2 | The Boiler and the Ring rooms (no exit gate) show the open frame | #274 | `scripts/author_danger_fx.py`; `src/room_exit_art.rs` |
| EXIT-AD3 | The grate snaps from closed to open on the clear tick (no lift animation) | #274 | `scripts/author_danger_fx.py`; `src/room_exit_art.rs` |
| EXIT-AD4 | The Guild door's two light emitters stay (its grille light now falls on the passage lamp, its floor spill on the threshold) | #274 | `scripts/author_danger_fx.py`; `src/room_exit_art.rs` |
| EXIT-AD5 | Only rooms 2, 4, 4a, 4b and 6 swap the door; room 1 (its own door), room 5 and the gym (their own gates), the CF02 microarena (`--cf02`) and the other composed scenes keep theirs | #274 | `scripts/author_danger_fx.py`; `src/room_exit_art.rs` |
| EXIT-AD6 | Design and execution in one pass; the silhouette gate is folded into the independent review | #274 | `scripts/author_danger_fx.py`; `src/room_exit_art.rs` |
| EXIT-AD7 | The composer's end torch behind the doorway is not drawn in the replaced rooms; its light emitter stays | #274 | `scripts/author_danger_fx.py`; `src/room_exit_art.rs` |

### GYM (8)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| GYM-AD1 | On the back wall at both ends, reaching each crash wall, because the `mirror_crack` VFX is anchored at the wall x and 40 px up; | #275 | `assets/source/terrain/floor01/floor01_gym_kit.json`; pixels `scripts/author_gym01.py` |
| GYM-AD2 | The reused `guild_portal_closed` exit marker | #275 | `assets/source/terrain/floor01/floor01_gym_kit.json`; pixels `scripts/author_gym01.py` |
| GYM-AD3 | Magenta without a palette colour | #275 | `assets/source/terrain/floor01/floor01_gym_kit.json`; pixels `scripts/author_gym01.py` |
| GYM-AD4 | The ceiling | #275 | `assets/source/terrain/floor01/floor01_gym_kit.json`; pixels `scripts/author_gym01.py` |
| GYM-AD5 | Torches in a gym | #275 | `assets/source/terrain/floor01/floor01_gym_kit.json`; pixels `scripts/author_gym01.py` |
| GYM-AD6 | Banner, posters, punching bag, lockers of the concept | #275 | `assets/source/terrain/floor01/floor01_gym_kit.json`; pixels `scripts/author_gym01.py` |
| GYM-AD7 | The door state | #275 | `assets/source/terrain/floor01/floor01_gym_kit.json`; pixels `scripts/author_gym01.py` |
| GYM-AD8 | The exit lights | #275 | `assets/source/terrain/floor01/floor01_gym_kit.json`; pixels `scripts/author_gym01.py` |

### RFA (1)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| RFA-AD1 | Keep RINGW-AD8's single-block static shutter (reverting it would move the solid count and nav) | #277 | `src/combat_scene/env.rs` |

### RWF1 (3)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| RWF1-AD1 | Escort when she is cut off from Carl by a barrier on his level (knob `fix_all.rwf1_cut_off_escort`, on in `cf02_free` only) | #278 | `assets/data/gameplay/companion_ai_v0.json` (`fix_all.rwf1_cut_off_escort`) |
| RWF1-AD2 | Escort when she is up on such a barrier above Carl: she keeps heading for his surface, drift check waived | #278 | `assets/data/gameplay/companion_ai_v0.json` (`fix_all.rwf1_cut_off_escort`) |
| RWF1-AD3 | Scope of the 'from above' case narrowed to a climb over a barrier on his level, not every ledge above him (review F1 of #278) | #278 | `assets/data/gameplay/companion_ai_v0.json` (`fix_all.rwf1_cut_off_escort`) |

### CW (5)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| CW-AD1 | Room 0 launchable but unlisted, and the floor run refuses it (until CORRAL-F) | #280 | `assets/levels/f01_late/manifest.json`; `assets/levels/floor01/f01_corral.encounter.json`; `src/f01_corral.rs` |
| CW-AD2 | The per-llama rush opt-ins, surface, facing, wake and leash stay in code (`corral_fixtures::room`), cross-checked against the sidecar | #280 | `assets/levels/f01_late/manifest.json`; `assets/levels/floor01/f01_corral.encounter.json`; `src/f01_corral.rs` |
| CW-AD3 | The story pin `f01_pin_corral_entry` is a label only: room 0 holds the Guild biscuit and grants nothing (KC9) | #280 | `assets/levels/f01_late/manifest.json`; `assets/levels/floor01/f01_corral.encounter.json`; `src/f01_corral.rs` |
| CW-AD4 | The CORRAL-C room file is unchanged | #280 | `assets/levels/f01_late/manifest.json`; `assets/levels/floor01/f01_corral.encounter.json`; `src/f01_corral.rs` |
| CW-AD5 | The M3 gate is an ExitGate solid at 2048..2064 that stands until both M2 llamas are down; drawn as a labelled greybox grate | #280 | `assets/levels/f01_late/manifest.json`; `assets/levels/floor01/f01_corral.encounter.json`; `src/f01_corral.rs` |

### DRA (5)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| DRA-AD1 | The dozer tuning pin applies to room `dozer` only (`pin_applies`, `SCOPED_PIN_KEYS`), so no other room's start hash moves | #282 | `src/f01_dozer.rs`, `src/room_pilots.rs`; `assets/levels/floor01/f01_dozer.encounter.json` |
| DRA-AD2 | Carl's input in the three Dozer escape plans is stateless; the brace plan keeps the first chase tick | #282 | `src/f01_dozer.rs`, `src/room_pilots.rs`; `assets/levels/floor01/f01_dozer.encounter.json` |
| DRA-AD3 | A parked machine puffs every 24 ticks | #282 | `src/f01_dozer.rs`, `src/room_pilots.rs`; `assets/levels/floor01/f01_dozer.encounter.json` |
| DRA-AD4 | The buffer stop stand-in is drawn 24 px high | #282 | `src/f01_dozer.rs`, `src/room_pilots.rs`; `assets/levels/floor01/f01_dozer.encounter.json` |
| DRA-AD5 | The Dozer pilot plans are aligned: the party is the `cf02_free` companion setup (review F1 of #282) | #282 | `src/f01_dozer.rs`, `src/room_pilots.rs`; `assets/levels/floor01/f01_dozer.encounter.json` |

### CF (3)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| CF-AD3 | An inert story room (the corral) carries the empty story ledger so the next room starts from its own pin | #283 | `assets/levels/f01_late/manifest.json`; `src/f01_run.rs` |
| CF-AD4 | The corral gets no render replay lines (the 153-line count is pinned); window = headless captures stand in | #283 | `assets/levels/f01_late/manifest.json`; `src/f01_run.rs` |
| CF-AD5 | The seed sweep of the gate probe is reported as is (seeds 0..8 identical); no seed-sensitive pilot added | #283 | `assets/levels/f01_late/manifest.json`; `src/f01_run.rs` |

### CG (8)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| CG-AD1 | Overlay, not redrawn frames. The gauntlet is a separate, full-cell overlay atlas per body sheet (same 48 x 56 cells, same frame order, same tags, same pivot), drawn on top of the bare Carl sprite. | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-AD2 | Camera-near hand only (`near_arm` layer). The jab hand (`far_arm`) stays bare | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-AD3 | The watch stays visible, strapped over the gauntlet's cuff; the overlay never covers a `watch` or `pin` pixel, nor a visible `prop` pixel (CG-L2) | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-AD4 | Palette: existing `carl-book1-v0` keys only: `K J j T U V w H`. No new colour, no `Y`, no glow; `w` (steel glint) at most 3 px per frame | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-AD5 | Sheets: overlays for `locomotion` (16), `melee` (24), `cf02` (13) and `dynamite` (45): 98 frames. `interact` is deferred (not drawn by `floor01_scene`) | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-AD6 | Presenter copies the visible body sprite. One overlay entity per Carl body sprite entity (`ActorVisual`, `CarlMelee`, `CarlThrowSprite`, `CarlCf02Sprite`) copies that sprite's visibility, atlas index, ... | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-AD7 | Brown wrist strap (`H`) between the jacket sleeve and the glove | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-AD8 | Brightest steel on the striking edge in punch frames (the leading knuckle plate) and a top-left lit plate elsewhere | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |

### CG-EX (6)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| CG-EX-AD1 | The overlay grids hold the `gauntlet` layer only (`carl_h52_<sheet>_gauntlet.grid.json`, palette = the 8 gauntlet keys). The body layers are not copied; | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-EX-AD2 | CG-L2 judges the visible prop pixels (approved by independent Opus review, PR #284). The near arm is drawn over the prop layer, so a prop pixel under a near-arm pixel does not show; | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-EX-AD3 | CG-L6 counts the exposed skin (approved by independent Opus review, PR #284, with the 10 exceptions below): near-arm skin pixels that the watch or pin layer does not draw over. | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-EX-AD4 | Stamping: the three keys are copied byte for byte; a hand that equals a key's skin shape with the same cuff side (the idle hand in 13 frames, the windup fist in dynamite 4 and 27 to 32) reuses the key by ... | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-EX-AD5 | Toolkit tick table in ms (`tick_hz` 1000) for the overlay configs: the durations are copied from the body sheet, and some (260, 420, 560 ms) are not whole 60 Hz ticks | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |
| CG-EX-AD6 | The presenter skips an overlay atlas that is not exported (stderr note, Carl drawn bare) and spawns overlays only in combat runs | #284 | `scripts/author_carl_gauntlet.py` and the overlay grids; spec only, no runtime knob |

### HZB (18)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| HZB-AD1 | The acid becomes the teal ramp of section 5.1, a 1:1 remap of the ten `dfx_acid` entries. The slime keeps its green. This supersedes the acid half of DFX-AD1 | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD2 | The ring's outer band row becomes `flame_00` in `ring_march` (both dash colours) and `ring_hot`. Nothing else changes | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD3 | The splash presents existing hurt events only: no sim entry tick, 4 slots, facing by the pop sign, the rim cropped to the live rect | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD4 | Corrosion uses the dry palette subset. Stains follow the authored patch extents, plus one const entry stain in room 4 at x 150 | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD5 | The Rot marks use ROT-01 entries only, with no glow, live red or spike colours | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD6 | Ceiling nests only; wall nests are deferred. Both drip orientations are drawn | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD7 | Hand-drawn keys live in `hz01b_keys/*.grid.json`, are loaded verbatim by the author script and are pinned by frozen-key fixtures | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD8 | Corrosion and nest marks are permanent from room load. They are dressing, not state | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD9 | The Rot nest and its track scuff are two grids, `dfx_rot_nest` (32 x 12) and `dfx_rot_trail` (16 x 3), not two tags of one grid | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD10 | The acid entry splash (`dfx_acid_splash`, 5 frames, 16 ticks) is a new layer above the actors and below the rings | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD11 | The splash and the decals have their own sprite pools (8 for the splash: a crown and a cropped lip per slot; one per static decal and one per sticker for the decals), not the 40-sprite danger FX pool | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD12 | An acid `EnvironmentKill` (the lethal landing) triggers the splash like `EnvironmentHurt` | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD13 | The scuff skips a position when it is 16 px or less from the nest (the nest is 26 px wide, so a scuff at exactly 16 px would sit under its edge). Room 6's 16 px track has a nest and no scuff | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD14 | The const entry stain is keyed by the combat fixture's name (`f01_hazards`), on the walking line of the room's first acid patch | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD15 | In frames 1 and 3 the sheet tips are one row taller than the table's "near" rows (right tip row 9 and left row 12 in frame 1; | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD16 | The wall drip sheet is authored with the ceiling pivot (4, 0) in the grid; the manifest records the wall pivot (0, 0) on its `_w` frames and the presenter applies it | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD17 | The wall trail is anchored on the backwall base (`floor_y - DECK_TOP_PX - 1`, with `DECK_TOP_PX` = 20 from the catacomb kit), not on the walking line | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |
| HZB-AD18 | The corrosion key is redrawn with three bites of different width, depth and spacing and offset trails. The stamping rule is unchanged (alternately mirrored tiles) | #285 | `scripts/author_hazard_decals.py`, `scripts/author_danger_fx.py`; `src/hazard_decals.rs` |

### HA2 (7)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| HA2-AD1 | Donut's mana track is 72 px outer on every plate state (8 segments of 7 px, 2 px gaps); the freed span holds the special label, right-aligned like Carl's BOOM | #286 | `assets/manifests/hud_hybrid_a_v1.json`; `src/hud_draw.rs`; `scripts/author_hud_hybrid_a.py` |
| HA2-AD2 | The socket keeps the button glyph; the item chips (dynamite, puddle) sit in a new item well on the grant notice (deviation, fallback HA2-Q2) | #286 | `assets/manifests/hud_hybrid_a_v1.json`; `src/hud_draw.rs`; `scripts/author_hud_hybrid_a.py` |
| HA2-AD3 | The notice frame becomes a plate inside the same 204x32 rect: worn bevel, magenta ring, dark fill, dotted rule moved to row +26 | #286 | `assets/manifests/hud_hybrid_a_v1.json`; `src/hud_draw.rs`; `scripts/author_hud_hybrid_a.py` |
| HA2-AD4 | Square key caps (11x11) for every keyboard key the HUD or a prompt can name; a new keyboard glyph set (J primary, K physical, L special) | #286 | `assets/manifests/hud_hybrid_a_v1.json`; `src/hud_draw.rs`; `scripts/author_hud_hybrid_a.py` |
| HA2-AD4b | Last-used device on P1 only: the keyboard set shows on the first keyboard input and returns to the pad family on the first pad input | #286 | `assets/manifests/hud_hybrid_a_v1.json`; `src/hud_draw.rs`; `scripts/author_hud_hybrid_a.py` |
| HA2-AD5 | AUDIENCE-01-A1 arrows: not in this order. AUDIENCE stays locked by default; no owner verdict exists on the arrows | #286 | `assets/manifests/hud_hybrid_a_v1.json`; `src/hud_draw.rs`; `scripts/author_hud_hybrid_a.py` |
| HA2-AD6 | The AI companion plate keeps the 72 px track and leaves the span beside it empty; no socket label, no glyph | #286 | `assets/manifests/hud_hybrid_a_v1.json`; `src/hud_draw.rs`; `scripts/author_hud_hybrid_a.py` |

### ST (9)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| ST-AD1 | The two ticks are unhashed observer fields (`landed_tick`, `gate_closed_tick`), excluded from the state hash like `Boss::charge_started` | #288 | `docs/02-art/f01-sim-ticks-spec.md`; observer fields in `src/` (unhashed) |
| ST-AD2 | The gate collider stays solid from the close tick; only the picture drops, over 8 ticks (four frames of 2 ticks, an accelerating fall) | #288 | `docs/02-art/f01-sim-ticks-spec.md`; observer fields in `src/` (unhashed) |
| ST-AD3 | The jamb channel is drawn from the first drop frame | #288 | `docs/02-art/f01-sim-ticks-spec.md`; observer fields in `src/` (unhashed) |
| ST-AD4 | The dust draws on its own layer, over the props and under the actors (`actor_z - 0.03`) | #288 | `docs/02-art/f01-sim-ticks-spec.md`; observer fields in `src/` (unhashed) |
| ST-AD5 | The Boiler landing throws two mirrored plumes, one from each edge of the remnant | #288 | `docs/02-art/f01-sim-ticks-spec.md`; observer fields in `src/` (unhashed) |
| ST-AD6 | The gate's dust starts on the impact (close + 8), from the gate's arena face, rolling into the arena only | #288 | `docs/02-art/f01-sim-ticks-spec.md`; observer fields in `src/` (unhashed) |
| ST-AD7 | The slam dust plays once over 20 ticks (2, 3, 4, 5, 6); frame 1 is the frozen key | #288 | `docs/02-art/f01-sim-ticks-spec.md`; observer fields in `src/` (unhashed) |
| ST-AD8 | Kit stone colours, no glow | #288 | `docs/02-art/f01-sim-ticks-spec.md`; observer fields in `src/` (unhashed) |
| ST-AD9 | DFX-AD7 (the gate has no drop animation) is superseded | #288 | `docs/02-art/f01-sim-ticks-spec.md`; observer fields in `src/` (unhashed) |

### RNG (24)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| RNG-AD1 | The kit is a contract dressing pass (`floor01_rings_kit.json`, per-room `room_dressing`), applied by a pure `ring_kit.rs` from `f01_ring::launch`. No room or encounter JSON edit. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD2 | Parallax shows only through the composer's existing openings. The R3 camp view is an opaque breach decal. No backwall tile is cleared (amended by RNG-AD20/AD21: two 4a openings are bricked up again with the ... | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD3 | Every ring arc and relief is `backwall_decal`. The arcs' bottoms sit at `W - 16`. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD4 | The soffit is drawn as `structure` tiles at `W - 244 .. W - 228` across each room's width, exactly over the existing ceiling slab solid. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD5 | Moving shutter frames are bottom-row crops of the closed key keyed by `edge_px`, under a 6-row housing. No drawn intermediate frames. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD6 | In-scene, the court's shutter is static closed. The moving states are drawn and unit-tested now, and their in-scene capture follows RING-C wire. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD7 | No stone value beyond the kit palette. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD8 | The crowned keystone is three blunt carved points with one snapped, as the apex stone of 4a's entry door (amended by RNG-AD21: 4a only). | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD9 | The three-notch shield is a plain relief shield with three V notches on its top edge, under the walkway right of the shutter. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD10 | The camp proper sits in 4b x 744..888 (between the route and R3). R3 shows the far camp through the breach; REST-B owns every actor. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD11 | Lanterns and the far campfire are baked glow, with no new light emitter. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD12 | Ring stone uses the warm `stone` ramp. The shutter slab is darker (`stone` 2..5 plus iron). The tiers are cold `gal` with `gal_warm` lamp pixels. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD13 | Ledge fronts in ring stone (32 px middle tile and 8 px end caps) replace the thin brick ledge look of every platform in both rooms. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD14 | Distant survivors are `survivor_silhouette_a..d`: flat `gal` 0..1, at most 24 px, no faces, at most two per piece, only in `ring_breach_r3` and `ring_camp_far`. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD15 | The concept's red hanging banners are dropped. Torn canvas in stone tones carries the cloth motif. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD16 | `door_timber` (composer, x 216) stays in both rooms; in 4a the crowned keystone is its apex stone (RNG-AD21). The composer's exit marker stays suppressed by `room_exit_art` (`dfx_exit`). | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD17 | The full ring's outer diameter is 204 px (r 102; the piece is 208 x 208 and its last four rows are empty), under #189's 250..400 px apparent diameter. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD18 | The Ring rooms take a `--ring-pilot` flag for evidence: the scripted run plays the library pilot (`f01_ring_fixtures::pilot`) instead of the walk-right script. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD19 | The ledge end caps are opaque (a darker return face and dark corner pixels), not transparent-cornered; the soffit, ledge, jamb, housing and shield pieces carry one nicked pixel so each holds the transparent ... | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD20 | 4a's ground bay at x 1680 is suppressed by the dressing (`remove_at`: both piers, the arch ring, the threshold and the chain) and its opening bricked up (`backwall_fill` x 1712..1920, y 352..560, the ... | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD21 | The crowned keystone is a 24 x 30 prop at (228, 466), the apex stone of 4a's entry door: its bottom 6 rows lie on the door's apex voussoir. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD22 | Torch bake: ring stone overlapping a torch pool rises +2 within 48 px of the pool's warm core and +1 within 100 px (dithered), capped at stone 5, never lowered (`Arc.torch_lit`). | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD23 | The shutter loses its two vertical iron straps; the courses are cut into blocks in running bond. The horizontal band and the iron shoe stay. | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |
| RNG-AD24 | Final 4a torch spots: the free torch hangs at (1360, 368) (pool `torch_pool_b_m` 1256/284), high over the great arc's broken right leg, 16 px under the soffit and 36 px clear of the arc's shoulder; | #290 | `assets/source/terrain/floor01/floor01_rings_kit.json`; pixels `scripts/author_rings01.py` |

### DZ2 (11)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| DZ2-AD1 | Separate damage atlases per part, replacing the live part one-for-one at the same anchor | #291 | `scripts/author_dozer_doz02.py`; `src/dozer_art.rs` (presentation only) |
| DZ2-AD2 | The pitch rule of section 3, 3 hull frames (`pitch_0`, `pitch_1`, `disabled`) at 6/6/held ticks | #291 | `scripts/author_dozer_doz02.py`; `src/dozer_art.rs` (presentation only) |
| DZ2-AD3 | No range tags; one tag per frame | #291 | `scripts/author_dozer_doz02.py`; `src/dozer_art.rs` (presentation only) |
| DZ2-AD4 | Hull validator limit 30 colours | #291 | `scripts/author_dozer_doz02.py`; `src/dozer_art.rs` (presentation only) |
| DZ2-AD5 | The frame hangs on a V sling from a hook; the chains are unoutlined | #291 | `scripts/author_dozer_doz02.py`; `src/dozer_art.rs` (presentation only) |
| DZ2-AD6 | The haul line runs from the pulley along the ceiling to an idler above the brace, then down to the cleat | #291 | `scripts/author_dozer_doz02.py`; `src/dozer_art.rs` (presentation only) |
| DZ2-AD7 | The 48-tick `impact_stunned` schedule and the `disabled` emitter rates of section 4.1 | #291 | `scripts/author_dozer_doz02.py`; `src/dozer_art.rs` (presentation only) |
| DZ2-AD8 | The tie bar of the frame ends up in the wheel (`landed_hit` remnant has no bar) | #291 | `scripts/author_dozer_doz02.py`; `src/dozer_art.rs` (presentation only) |
| DZ2-AD9 | The riders' tumble arc ends at the column where the simulation's goblin lands on the disable tick (the wheel front plus `dzq20_land_dx`: A 32, B 72), not at the design's 28 / 36 px of forward travel; | #291 | `scripts/author_dozer_doz02.py`; `src/dozer_art.rs` (presentation only) |
| DZ2-AD10 | The haul tiles share a 12 x 12 atlas cell (8 x 8 of chain content, the idler fills it) and the validator allows 12 orphans per frame for them only | #291 | `scripts/author_dozer_doz02.py`; `src/dozer_art.rs` (presentation only) |
| DZ2-AD11 | The gate, lintel mouth and fill tiles carry a few transparent pixels (chipped jambs, a chipped stone, a feathered shadow edge); the glance beam is redrawn column by column; | #291 | `scripts/author_dozer_doz02.py`; `src/dozer_art.rs` (presentation only) |

### DRD (2)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| DRD-AD1 | A non-blast clang sparks on the hull face nearest the hero nearest the hull, 24 px above the floor | #292 | `src/dozer_art.rs` (presentation only) |
| DRD-AD2 | The gate burst is two splinter bursts, at 40 px and 88 px, 4 ticks apart, on the emerge tick | #292 | `src/dozer_art.rs` (presentation only) |

### HT (1)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| HT-AD1 | TRY n counts every attempt start: room entry, each checkpoint respawn, each restart (one expression, `CombatState::hud_try`) | #293 | `CombatState::hud_try` in `src/combat_scene.rs` (one expression) |

### HEAT (21)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| HEAT-AD1 | All three partition states are drawn and exported now, but the dressing does not place the partition in room 0 until CORRAL-D lands. CORRAL-D is scheduled immediately after `heatart`; | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD2 | The kit is a contract dressing pass (`floor01_heat_kit.json`, `room_dressing.corral`) applied in `f01_corral::launch` through `compose_kit_hooked`. No room or encounter JSON edit. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD3 | The partition remnant is the canvas rows 32..47 (16 px, flat pale top). | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD4 | No new hue or value; embers are `flame` 0..1 only. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD5 | No emitter and no animation in the kit. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD6 | Floor decals use only the deck's back half (y 561..572) and never a step's top band; the dock gets wall soot and charred fronts instead of floor decals. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD7 | The step fronts and the bumper are scorched dressed stone and stacked timber capped with a stone slab; every walkable top's first row is `stone` 8. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD8 | Key pins (pixel-grid SHA-256, `json.dumps(px)`), after K2, K3 and the R-redraw: `heat_partition_intact` `0baf3cacb6ddb59e714f9608db946f880ccfdd3a6971e1792b0bee6bc1d33b70`, `heat_partition_weakened` ... | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD9 | The pen rails stand on the back line behind llama B's leash, not across it. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD10 | The hero wall scorch sits at the pen's entry (1088, 464), not behind the partition. Its rows 80..95 lie on the stone base course (y 544..559), not on brick, and only re-shade that stone. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD11 | First gate (independent top-tier review of the keys; its editor is the design authority for these edits): partition FREEZE with K2 and K3, `heat_scorch_floor_a` FREEZE, `heat_scorch_wall_a` REDRAW (its streaks ... | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD12 | No wall decal inside a torch pool's visible re-shade, and every wall decal on the world bond (section 5). A placement that cannot meet both moves along x by 32 px steps; | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD13 | Execution default: the shelf end caps keep row 0 `stone` 8 except the outer pixel (`stone` 6, the rounded corner of section 11.4); the "outer two columns one step darker" applies to rows 1..31. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD14 | Execution default: the rope lashings of the upper rail's two crossings start at row 7 (rows 7..9), so the rail's rows 0..6 stay no lighter than `wood` 4. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD15 | Execution default: `heat_scorch_wall_c`'s base-course dome uses `heat_scorch_wall_a`'s values (`base_s` 1.0, `base_rx` 14, `base_ry` 18, over local x 0..31), no crazing, its two embers at (13, 46) and (17, 45). | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD16 | `heat_scorch_wall_c` and `heat_trace_wall_a` are drawn, exported and in the contract, but not placed. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD17 | Execution default: the lit measurements use the kit's own lit model (`env_kit_light.py`, the Broken Ring precedent) for the actors' dL* (the library pilot cannot stand a goblin at x 1078, 1110, 1488 or 1968) ... | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD18 | Execution default: a still is defined by its capture sequence (first, step, count) and is its last frame. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD19 | Execution default: the before stills come from a `floor01_scene` built from origin/main (`BASE_BIN`); `scene_before_pen_entry_640x360.png` is byte-identical to the gate's real still. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD20 | `heat_scorch_wall_b`'s `foot_x` is 16, not 22 (the independent review's fix F2, applied by the executor): the core now sits inside the bottom-row brick 8..23. | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |
| HEAT-AD21 | Rule 10.1 and gate C2 exempt the torch-pool brick from the brightest-line comparison (section 10). | #294 | `assets/source/terrain/floor01/floor01_heat_kit.json`; pixels `scripts/author_heat01.py` |

### LL3 (10)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| LL3-AD1 | A bonk shows the LLAMA-01 `hurt` frame (11) for its first 12 ticks, then `recover_blown` | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |
| LL3-AD2 | `lean_px` is 0 in tell and committed when the art is on; the bonk rock stays | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |
| LL3-AD3 | The scrape gouge and flecks move to the pawing fore hoof (`x + dir * 6`); the 3 x 3 puff stays behind the hind hoof | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |
| LL3-AD4 | The tell is brace_in, three brace/scrape beats on the stand-in's 8-tick rhythm, then coil | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |
| LL3-AD5 | Run cycle `push, reach, contact, gather`, 3 ticks a frame, from active tick 0 | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |
| LL3-AD6 | Llama aggression cue: ears pinned plus tail raised on brace, scrape and run | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |
| LL3-AD7 | No in-game lane band | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |
| LL3-AD8 | The recover head pitches down, never up | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |
| LL3-AD9 | The rush art is on by default in `--scene f01_late --f01-run` and in the corral room | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |
| LL3-AD10 | A separate extension atlas (`llama_rush_h64`) instead of appending to LLAMA-01 | #295 | `assets/data/gameplay/enemy_llama_rush_v0.json`; `src/combat_anim/rush_stand_in.rs`; art `docs/02-art/llama-03-rush-spec.md` |

### HC (11)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| HC-AD1 | The stir frames are a rigid column lift of `sealed_back` with an unchanged front lip | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |
| HC-AD2 | Two levels: S1 period 72 (heave 6, slump 4), S2 period 36 (heave_hi 8, slump 4) | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |
| HC-AD3 | Lead x = Carl, plus Donut while the party is live, the companion bot included (`hoarder_art::lead_x`; the stir follows the same present set that crosses the gate, so S1, S2 and the gate never disagree about ... | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |
| HC-AD4 | Breath phase anchored to the first tick the presenter sees the level (render-local memo, cleared when the tick goes backwards) | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |
| HC-AD5 | Stir and give grit reuse the R7 dust motes (colour, size, `mote_z`); 3 (S1), 5 (S2), 4 (give) | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |
| HC-AD6 | `shored` on every tick she is not out, Dormant and Reveal, first attempt and retries | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |
| HC-AD7 | `give` for 8 ticks from her first out tick, only over `intact`; then the machine's frames | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |
| HC-AD8 | No reactive cue on a refused post hit; bolts keep passing the post | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |
| HC-AD9 | On by default wherever the Hoarder room is drawn, including `--scene f01_late --f01-run`; no flag | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |
| HC-AD10 | Cue frames appended: mound 3..5, post 3..4 | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |
| HC-AD11 | Hysteresis: once S1 is active it holds until the lead hero is under x 224 (16 px under `hrdq05_stir_x`); the hold lives in the render-local memo (`HoarderMemo::stir_held`, `stir_level_held`) | #296 | `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq41_*`); `src/hoarder_art.rs` |

### AR (25)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| AR-AD1 | The unlock beat is the corral exit (the refuge onward, room 4 of N, N = 15 since ACT1-B: the run length is read from the manifest). | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD2 | The corral's room-clear panel gets one fact line, only when the audience will be live next: `broadcast: live from the next room (viewers per hero)`. No number on any panel. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD3 | `--f01-audience C:P:D:Q` carries the current count and the peak per hero at the exit tick. The decay remainder is dropped. Validation is `peak >= current >= 250`. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD4 | A live room started without a carry starts at the baseline of 250 for both heroes. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD5 | Restart restores the room-start snapshot. Respawn keeps the counts. The per-room dedupe, chain, cue and trend state starts fresh. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD6 | `--audience=on/off`: default on iff the run default predicate. Explicit `on` is allowed only with `--f01-run`. `off` locks everywhere and persists across relaunches. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD7 | `--cine-showcase=on/off`: default on iff the run default predicate. Explicit `on` is allowed on any path. `off` gives main's rules. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD8 | The live adapter and the `AudienceFeatFeed` observer use the same `CineTuning`, including `showcase_feats` and the showcase-target bridge, so the chain feats of headless and fast-forward runs match the live ... | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD9 | `CINE-THROWN-MULTI`: score 5, base 3, `thrown_multi_min_kills = 3`, all of them on one blast's tick. Llama blast defeats count toward the 3 (any distinct non-boss target). | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD10 | Deliberate-throw proof: `ChargeThrown { charge: c }` on a tick without `HeroHurt` of the charge's owner. A drop never credits. A release on the same tick as a hurt of the owner counts as a drop. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD11 | Authored, chained, sticker, world and QA blasts, and `player: None`, stay uncredited | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD12 | Credited thrown-blast defeats are `DefeatKind::Blast`: they count as extra kills and never complete MIXED-CHAIN | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD13 | `CINE-SHOWCASE-KILL`: score 5, base 1, a credited llama defeat on the completing tick. The anchor is the lowest llama id of the batch. `showcase_kill_score = 0` disables it alone (data, no flag). | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD14 | `CINE-SHOWCASE-BONUS`: +1 when a llama is in the scored chain and the primary is not SHOWCASE-KILL | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD15 | Primary selection: largest base wins; ties go in the order THROWN-MULTI, MIXED-CHAIN, PRECISION-EDGE, SHOWCASE-KILL. All primaries score 5, so they never stack. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD16 | The boss never: not credited, not a showcase target, never one of the 3 | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD17 | Thresholds 5 and 8, windows 90 and 90, cooldown 900 and upgrade 8 are unchanged | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD18 | No new audience constants. Showcase moments pay through the existing chain formula. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD19 | Floor 01 authors no anticipation cue. The cue table is empty in the run. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD20 | No viewer number on the room-clear or floor-end panels | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD21 | `CINE01_MOMENT` appends ` thrown_multi=1`, ` showcase_kill=1` and ` showcase_bonus=1`, each only when set and before ` upgrade=1`. Legacy lines stay byte-identical. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD22 | `AUDIENCE_STATE` prints only under `--audience-probe` or an explicit `--audience=on`. An explicit `--cine-showcase=on` on a headless or capture path prints observer `CINE01_MOMENT` lines, so party fixtures can ... | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD23 | CA-29 is unchanged: a defeat credited to a companion-driven hero never triggers CINE, and audience credit stays factual. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD24 | Per-death-kind presentation: no new art or wiring, only evidence; the planner's `finish_active` rule is unchanged | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |
| AR-AD25 | The Hoarder (`PILE_BOSS_ID`) is excluded like the Juicer: never a showcase target, never one of the 3 blast kills. | #297 | code constants in `src/cine.rs` and `src/f01_run.rs` (`showcase_kill_score`, cooldown); no data file yet |

### CD (10)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| CD-AD1 | The sheet's collider (1392..1408 x 532..580) is authoritative and the art needs no change: the 40 x 48 canvas holds its solid core in columns 12..27 by spec, so it is placed at the collider's top-left minus ... | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |
| CD-AD2 | A counted hero hit is one hit whatever the move's support-hit weight (the reinforced haymaker, weight 2, still counts one): "three distinct hits" | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |
| CD-AD3 | The slowest shove that breaks it is 160 px/s (the other props' launch and shove threshold, BOILC-AD2); the rush is 240 px/s | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |
| CD-AD4 | A rush that meets the right face stops with its pivot at 1436 (box front 28 px), not the 1428 of the geometry table (the walking stop); the near-miss fixture's leash minimum is therefore 1440 | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |
| CD-AD5 | The break is the appended `PropPhase::Broken` (`partition_broken` of #181); the event order on the contact tick is the rush end, then `PropHit { Shove }`, then `PropPhase { broken }` | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |
| CD-AD6 | A launched body (a kicked goblin) and a blast do not break it | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |
| CD-AD7 | The CORRAL-AUTO `broken` variant is a tick-0 setup write (exempt like the other setup writes, F11); the partition-top and remnant-top starts are judged on the pivot column alone (her body overhangs the 16 px ... | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |
| CD-AD8 | A new evidence plan `corral_partition` (Carl jabs the screen) shows the three states in the run; the `corral_rush` plan is unchanged apart from its re-based tick map | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |
| CD-AD9 | The break plays the existing wall-break cue; no splinter or dust VFX of its own (the bonk dust of the rush stand-in plays) | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |
| CD-AD10 | A checkpoint respawn keeps the partition's state (a broken or weakened screen stays so); only a restart (retry) restores it. | #298 | `assets/levels/floor01/f01_corral.encounter.json`; `assets/data/gameplay/enemy_llama_rush_v0.json` |

### ENG2 (20)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| ENG2-AD1 | ENG-G-Q24 (the owner's game-feel verdict on the Engineer before any art) is waived. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD2 | Route Easy, score 1. Only the get-up pair are design keys; everything else is a rule-based derivation. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD3 | The get-up is appended to the ENG-01 atlas (18, 19; tag `getup`; 10 + 12 ticks = `q04_getup_ticks`). It replaces the `attack_recover` stand-in. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD4 | The ram run is taken: frames 20 and 21, tag `attack_ram_run`, cycle 12, 20, 21, 20 at 4 ticks. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD5 | Crush and decap use 6 frames on the 4 GOB-02 tags: 4 keyed poses plus GOB-02 A's 2 in-betweens. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD6 | No pot is drawn in any slammed cel. The pot and the wrench are launched as parts at the slam, in all three poses. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD7 | Crush: a 4-row flat pot (rows 16, 17, 23 and 24 of the pan, full width) is squashed into the head in frames 0 to 2. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD8 | The pot part is one cel rotated at runtime, not 3 rotation cels. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD9 | Palette `eng-death-floor01-v0`: 20 keys (the 17 ENG-01 keys plus `R`, `B` and `P` at the GOB-02 hex). | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD10 | The wrench stays a stowed prop. The crush and the slam launch it from the back hip (`hip_back`). The explosion throws it at presets 2 to 4. It is never drawn in a hand. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD11 | Finish ids are `engineer_slammed`, `engineer_exploded`, `engineer_decapitated`, `engineer_crushed` and `engineer_knockback_down`. The last is also the fallback and keeps ENG-01 `down` 15 to 17. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD12 | The Engineer's death art is its own variant, `ENGINEER_VARIANT = DEATH_VARIANTS + 1`. Decals are shared with goblin set 0. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD13 | The parts sheet drops `club`, `club_stick` and `pineapple`, and appends `pot`, `head_bare`, `head_bare_dead` and `wrench`. `head` and `head_dead` carry the pot. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD14 | No yellow `e` in any slammed, death or part cel; the GOB `R` pixels stay. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD15 | ENG2-Q1: the death sheets use 20 keys (the 17 ENG-01 keys plus `R`, `B` and `P`). | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD16 | ENG2-Q2: one runtime-rotated pot cel. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD17 | ENG2-Q3: the ram run plays at 4 ticks a frame. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD18 | ENG2-Q4: the wrench reappears on the shove; the sit stays as frozen. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD19 | ENG2-Q5: the stomped pot pops off. | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |
| ENG2-AD20 | Flung slam: the pot and the wrench launch on landing, not at age 0 (section 7). | #300 | `scripts/author_eng02.py`; `src/death_present.rs` death profiles |

### REST (18)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| REST-AD1 | Room ids `rest_refuge`, `rest_camp`, `rest_aftermath`; scenes `f01_rest_*`; titles "The Wrong Restaurant", "The Survivors' Camp", "After the Juicer". "Meadow Lark" stays in design text, not in data | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD2 | Run order of section G.2; slots S0 to S4 are documentation only | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD3 | REST-A arms the existing row through a new `SceneKind::RestRefuge`; the sidecar pin `f01_pin_refuge_entry` names no table row; `f01_pin_pre_pile_boss` unchanged | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD4 | The reward zone keeps x 1200..1240 and the room is built around it; DP fixtures arm the row by id | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD5 | Option A1 without Interact: standing on the 128 px shelf opens the staff door (`hero_on_surface`), latched | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD6 | The door is a 16 px solid from the walking line to the 228 px slab | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD7 | Solo hint `SWAP TO DONUT` after 90 ticks within 96 px of the closed door, Carl active; none in 2P | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD8 | Checkpoints keep the existing policy; no heal or refill | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD9 | REST-B reuses `f01_pin_post_pile_boss`; REST-C reuses `f01_late_mastery` | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD10 | Survivors and sentry are static labelled markers; no NPC AI, no dialogue | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD11 | REST-WIRE lands as two PRs at milestone `greybox`: PR 1 the refuge (listed in the manifest), the lock, the grant move and the DP retarget; PR 2 the camp, the aftermath and the twelve-room run walk | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD12 | The heavy bag is a non-solid `hit_target` static if one works without a machine; otherwise decor | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD13 | A REST room has an empty roster under `reach_exit`; if the validator rejects that, it is allowed for `reach_exit` only, with a test | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD14 | AUDIENCE-RUN runs unchanged in REST rooms: no kills, so no awards; decay runs as tuned (after 480 steps without an award it removes 23‰ of the excess over the 250 baseline every 60 steps, so a 75 to 120 s ... | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD15 | The camp interior art is a new row AREA-CAMP-01, reusing the AREA-RINGS-01 camp family | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD16 | The box is non-collidable decor; the grant column covers any height at the box | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD17 | No door notice; the door's disappearance and the existing `LIGHT FEET` notice carry the beat | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-AD18 | The legacy `--donut-prog` corridor stops granting the boost | #301 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |

### REST-IMP (12)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| REST-IMP-1 | the plan's play deck (32 px) and slide top (48 px) are rejected by the validator (`PlatformBlocksLane`: a deck needs 52 px of clearance under it, i.e. at least 64 px of rise with the 12 px slab). | #302 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-IMP-2 | `goal_x` is 1528 (not 1536) so the room's derived exit is the brief's `{1504, 484, 64, 96}` (the room clamps `goal_x - 24`). | #302 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-IMP-3 | `max_living` is 0 (the smallest valid value). The validator already accepts an empty roster under `reach_exit`, so no `encounter.rs` rule and no `an_empty_roster_is_valid_only_for_reach_exit` test were needed. | #302 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-IMP-4 | PR 1 keeps `RestRoom` with `Refuge` only and `f01_rest::data_pins()` returns 2 entries (not 6); the camp and aftermath arrive with PR 2. `SCOPED_PIN_KEYS` 19 -> 21. | #302 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-IMP-5 | the decor labels are the decor table's `label` fields (printed as `F01_REST_DECOR` lines and drawn as coloured rectangles); no on-screen text was added. | #302 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-IMP-6 | a fixture restart resets the lock through `EnvRuntime::reset`; a respawn keeps it through `reset_keeping_partitions`. | #302 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-IMP-7 | the `cf02_free` companion did not open the door by herself in the fixture run, so `rest_refuge_solo_companion_crossing` scripts a human swap at the closed door (as the hint asks) and a swap back ... | #302 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-IMP-8 | neither room has a platform. The plan's 32 px crate stack and 24 px bench fail the validator's lane rule (a deck needs 52 px of clearance, so 64 px of rise), and a 64 px deck would need a lane ceiling (DP-Q45: ... | #304 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-IMP-9 | the heavy bag is the sidecar's `hit_target` static (validates, inert: the env builds machines only for statics with a slide or a stack, so it never blocks or reacts) at {624, 516, 32, 64}: the validator ... | #304 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-IMP-10 | `goal_x` 1480 (camp) and 1224 (aftermath) give the planned derived exits; the entry line of the camp and aftermath reports `lock=none` and no `boost` field; | #304 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-IMP-11 | a carried snapshot replaces the ledger it enters, so `rest_aftermath_no_duplicate_unlock` unites the rooms' real snapshots by hand in run order (refuge boost, Hoarder dynamite, Juicer fist and blink): four ... | #304 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |
| REST-IMP-12 | a fixture restart in a room entered with a carried ledger restores the carried held set and clears the receipts (the ledger's existing restart rule); a checkpoint retry keeps the whole ledger. | #304 | `assets/data/gameplay/story_unlocks_v0.json` (`armed_in`); `assets/levels/floor01/f01_rest_*.{room,encounter}.json`; `src/f01_rest.rs` |

### ACT1 (19)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| ACT1-AD1 | Three rooms `act1_chase`, `act1_shore`, `act1_guild` before the corral, milestone `greybox`, `cf02_rules`, story-less | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD2 | The chase/shore boundary is ENV-02's post-chase checkpoint (route 2640); the standard room-clear panel shows between them | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD3 | Titles "The First Chase", "The Rotten Shore", "The Guild Door" | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD4 | Spawns Carl 72 / Donut 48 in the chase (ENV-02 spawn), Carl 112 / Donut 80 elsewhere | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD5 | Commit rule of section 5.3 (all heroes past `arm_x`, gap ≥ 140 from the rearmost body, reach margin 20 ticks, the charge reaches the barricade), one charge per attempt, no fail-open timer | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD6 | The chase trigger is crossed by the rearmost living hero | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD7 | No idle wake in the opening (216000 ticks) | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD8 | The S3 soffit is decor, not a solid | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD9 | No riders on the opening machine in the greybox; the shore goblins stand in for the thrown riders | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD10 | The wedged machine in the shore room is decor | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD11 | The guide is decor at the `refuge_guild` mark; the `mordecai_h44` sprite if drawable, else a labelled rectangle; no text | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD12 | The guild panel lists the read-through grant; a `MAGIC MISSILE` notice on `UnlockGranted` | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD13 | Donut locked in the Act I rooms under Story and flagless; `unlocked(8)` under All or the light-feet / puddle flags | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD14 | `RescueSpot::Pocket`, `RescueSpot::Behind` and `RescueCause::Charge` appended; the solo bot is put in the pocket at the tell; no rescue cause in the pocket or fully behind the hull; | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD15 | The opening values live in a new tuning file with its own pin | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD16 | `stop_x` 2856 and the exit at 2928; the machine wedges at the S5 arch's right edge | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD17 | The humps and the alcove ledge are 16 px sidecar statics of kind `solid`, not room platforms; the pocket's `ledge` names the static | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD18 | After `BarrierImpact` the charge ignores `max_ticks` and runs to `stop_x`, then brake, strain, `Idle` and `Wedged`; `ChargeEnded` at `stop_x` | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-AD19 | The barricade is open iff `Dozer::barrier_hit()`; no env flag | #303 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |

### FF (31)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| FF-AD1 | Route B + silhouette gate (score 2), up from the audit's C | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD2 | The door is a full-height stainless sliding panel that rises into the slab. No header or drum hangs in the lane; the mechanism is implied by the slab | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD3 | Jamb rails stay in both states, the lamp turns red to green, and the rise is 12 ticks of key crops | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD4 | The pickup is a small gilded box on the floor that opens and stays open when spent | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD5 | Box state reads Donut's held boost, not a separate flag | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD6 | No emitter on the box | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD7 | Six steady cool strip lights replace the four dining-room torches; the threshold and vestibule torches stay | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD8 | Cool strips in the refuge do not breach atlas row 09's "fluorescent/accent light only here" (the gym) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD9 | No palette extension; whites are warm and reserved (rule 10.2); "white" tile is pale `gal` and cream `stone` | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD10 | Skins cover collidable plank tiles on the tile's grid row (16 opaque rows, stand line on row 1) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD11 | The counter is placed at x 704..831 (behind no platform), not across the greybox `counter` rect (320..768) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD12 | The hot→cold seam is at the timber door (x 264) with loose tiles on the hot side; bay A's stone pier stays | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD13 | One static stray ball, no rolling gag in this work order | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD14 | The service bell rings once on the open tick (presentation only) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD15 | The door atlas pre-bakes 14 frames (closed, the 12 `door_raised(RISEk-1)` crops, open), 24 x 228 each; no runtime crop (gate note G5) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD16 | `ff_rock_fill` is a `backwall_decal` over bay B's opening at (904, 352), and the dressing's `parallax` block is empty | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD17 | A tile piece with no transparent pixel is declared `"opaque": true` in the contract, and `env_kit.py` checks it fully opaque instead of requiring one transparent pixel. | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD18 | The skins' end caps (`brass` 3) cover rows 2..14, so row 1 is `flame` 5 across the platform's full width | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD19 | The box's glint timer (`t_present`) is the scene's tick counter | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD20 | Rewritten after the independent review (F4): the wall tiles run under the hatch, the canopy and the under-panel; the three decals are placed after every tile, so they draw over them. | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD21 | FF-Q1 answered: no palette extension in this work order; cream `stone` 9/10 and pale `gal` stay (the drift from "blue/white" is known and recorded) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD22 | FF-Q2 answered: cool steady strip light in the refuge is acceptable; FF-AD7 and FF-AD8 stand | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD23 | FF-Q3 answered: no glow on the box (FF-AD6 stands) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD24 | FF-Q4 answered: the stray ball stays static (FF-AD13 stands) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD25 | FF-Q5 answered: no paw-switch frames in this work order | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD26 | The checker floor, its broken edge and the spill are `structure` pieces, listed before `ff_play_posts` (review F1) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD27 | `remove_at` also drops bay B's `arch_chain` (912, 384); bay A keeps its chain (review F2) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD28 | `ff_play_posts` is 224 x 68 at (464, 512) (review F3) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD29 | Both right piers are closed (review F5): `backwall_fill` reaches x 319 in bay A, `ff_rock_fill` is 168 wide in bay B, and one more wall tile cell sits at x 1120 | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD30 | The box's glint period stays 24 presentation ticks (review item 6) | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |
| FF-AD31 | G7 closed as accepted (review item 1): panel 47.8 against wall 72.3 (difference 24.5, ratio 0.66) under the scene ambient, the rail outline unbroken on 160 of 160 rows; FF-AD16 and FF-AD17 accepted | #305 | `assets/source/terrain/floor01/floor01_fastfood_kit.json`; pixels `scripts/author_ff01.py` |

### ACT1-IMP (19)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| ACT1-IMP-1 | The sidecar rows carry `id`, `kind`, `x`, `beat` and `delay_ticks`. The module constant `f01_act1::SHORE_TEMPERAMENTS` (brawler, coward) is applied through the new `CombatState::with_goblin_temperaments`, ... | #306 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-2 | A checkpoint behind the start is the only respawn column of the room (a retry puts the party at 56 in the shore and at 88 in the guild). A checkpoint ahead keeps the usual start then checkpoint pair. | #306 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-3 | The room parser derives the exit as `goal_x - 24`, 64 px wide, so `goal_x` is 752 (shore) and 584 (guild): the exit rectangles are the design's `{728, 484, 64, 96}` and `{560, 484, 64, 96}`. | #306 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-4 | `RawOpensOn.target` defaults to the empty string. A `donut_unlocked` lock must leave it and `surface` empty; | #306 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-5 | A `CombatEvent::ManaLockOpened { tick, lock }` is appended and logged once per opening (first step records the state, so a room that starts unlocked logs none). | #306 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-6 | `SceneKind::Corral`'s setup (no pin row, no trigger), so `story_unlock.rs` and its table are untouched. | #306 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-7 | The existing `mordecai_h44` sprite, drawn exactly as the refuge draws him (the refuge's Mordecai mark, standing at the guide decor's column x 380, facing left, with his contact shadow); presentation only. | #306 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-8 | The fixture calls `CombatState::respawn` (the call a party defeat makes), the REST retry fixtures' precedent, and asserts the respawn column and the restored goblins. | #306 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-9 | `--act1-pilot walk | #306 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-10 | Every "front reaches x" test and rule 4 accept a front within 0.01 px short of x (`REACH_EPS_PX`). The boundary test (front 1855 against 1856 at 160 px/s) is unchanged. | #307 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-11 | F14 asserts that the `WheelHit` precedes any `BarrierImpact`, no `PinnedHold`, no tell, and that the respawn rebuilds the machine (dormant, `barrier_hit()` false, the barricade solid standing again, the hint ... | #307 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-12 | The site carries the room's walking line in layout px (`floor_px`, set by the loader); the ledge top is `floor_px - rect.bottom()` over the world walking line. The same conversion, taken from the line. | #307 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-13 | The script holds P2 at `min(lag, 440)`; the separation at the leader's arm is recorded in the evidence table (6, 99, 230 and 444 px for the four cases). | #307 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-14 | The opening's O1 to O10 are reported as DOZER rules 11 to 20 with an `O<n>:` prefix in the reason. O8's closed half (the exit is unreachable with the barricade closed) is the test ... | #307 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-15 | The rule keeps the integrated count. The fixtures put Carl at the barricade (F14), as the plan says; Donut there is the rule-3 boundary and is not asserted. | #307 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-16 | The scripted hero waits at the pocket's right end (x 2136), so the trailing bot stands inside the pocket before the tell and `AiRescue` is 0. | #307 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-17 | The console line `F01_ACT1_WEDGE label="GREYBOX WEDGE"` and the darkened, arch-clipped rig; no on-screen text. | #307 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-18 | `dozer_art::draw` skips the room pieces and the stop frame for a profile with a charge block; the emergence, timber and barricade bursts use ENV-02's splinter art at the window, the timber sets and the ... | #307 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |
| ACT1-IMP-19 | The scene raises the hint while the window is live and the notice runs its own lifetime; presentation only. | #307 | `assets/levels/floor01/f01_act1_*.encounter.json`; opening profile `assets/data/gameplay/enemy_dozer_opening_v0.json`; `src/f01_act1.rs` constants |

### CAMP (26)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| CAMP-AD1 | Route B + silhouette gate (score 2) | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD2 | Two contracts: the camp extends the rings kit, the aftermath extends the gym kit (single-chain `extends`) | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD3 | Survivors are four anonymous, faceless figures (sentry, repair, seated, map), at most Carl's height and darker than the heroes | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD4 | The hearth replaces the greybox lantern as the central light, on the solid stretch 672..855 over the checkpoint; four torches and their pools are removed | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD5 | The marker stands at x 156, not 232; the sentry at 500, not 520; the crates are `ring_crates_b` at 652, not the greybox stack at 704; the map trestle and figure move to 1112..1177, out of window B4 | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD6 | The camp keeps `grime_a` and the Ring parallax | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD7 | REST-Q6 (presentation half): the bag swings once on entry and on each hit's rising edge, with the 38-tick table and a rewind guard; the sim stays inert | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD8 | B2 of the aftermath is bricked up (`backwall_fill`) with five girders; its `arch_chain` is removed | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD9 | No fluorescent tube in the niche | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD10 | The wreck panel reuses the gym mirror glass, its reflections darkened to `gal` 2/3 | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD11 | The bag body is drawn over the sim's hit rect {624, 516, 32, 64}: at rest its cap and body cover x 626..654, y 510..569 | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD12 | No crate-repositioning animation and no idle animation of the figures in this order | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD13 | No audio | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD14 | The grooves are drawn here as concentric 2 px cuts (radii 82, 94, 106, broken in places, a 1 px `stone` lip on the upper inner side), centred on the portal's mid-height (1232, 512), instead of with the rings ... | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD15 | Gate note G1: a sixth `gym_girder` at (684, 440), listed before the one at (676, 440); the pixel model shows no brick between the run and either pier shadow column (a transparent top corner at each girder ... | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD16 | Gate note G3: `camp_figure_map` at (1152, 514), after the table, so its wrapped hand draws over the table's end | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD17 | Gate note G4: `camp_toy_cart` at (1048, 548), 7 empty columns from the seated figure | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD18 | Gate note G5: the repair figure's lamp is a shuttered `iron` 2..4 tin with one 2 x 2 `flame` 3 slit, no emitter | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD19 | Gate note G6: the bag presenter's entry swing starts on the first tick it observes, not at tick 0; same tick, same frame; a rewind resets to `rest` and treats a held overlap as held; | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD20 | The fire is its own atlas (`floor01_camp_fire`, one canvas per frame, tag `burn`) and the hearth atlas holds the hearth only | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD21 | The floor chalk, the sand spill and the lane chalk are `structure` pieces | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD22 | A new evidence pilot `--rest-pilot punch` (aftermath only) | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD23 | The window-rule and torch-clearance tests cover the dressing's own `prop` and `structure` placements; a wall decal sits behind a torch, and the composer's `torch_pool_*` decals are not floor decals | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD24 | The drawn view lefts of the design stage (237, 483, 607) are 25 px short; the measured ones are 262, 508 and 632 | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD25 | Review R1: the strength motifs are a 48 x 48 cluster at (1420, 364) on exposed brick, not the 80 x 48 piece at (1344, 384) that the arch and the capital covered; | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |
| CAMP-AD26 | Review R2: no chalk pixel lies on a pier plinth's face. The chalk marks sit on the visible floor between the plinths; | #308 | `assets/source/terrain/floor01/floor01_camp_kit.json` (placement); pixels `scripts/author_camp01.py` |

### TROG (47)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| TROG-AD1 | Two rooms `trog_run` and `trog_strength` between `rest_camp` and `juicer`; greybox, `cf02_rules`, story pin `f01_pin_post_pile_boss` | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD2 | Titles "The Troglodyte Run" and "The Test of Strength" | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD3 | The numbers of 5.1 (HP 2/4/3, damage 1, rests 40/30/36, knock values, leap 64..160, apex 16, standoff 66) | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD4 | Hitboxes, hurtboxes and windows copied from the TROG manifests, with a drift test | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD5 | The leash is a spawn-relative half-width in tuning (192 / 96 / 128); no sidecar schema change | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD6 | A proximity wake needs `trigger_x` crossed by a hero and the whole hurtbox in view; every tell needs the whole hurtbox in view, whatever the wake cause (amended by AD21) | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD7 | Commit at tell start: facing, landing x, smash rect and line; one blow per hero per instance; no tracking | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD8 | A 24-tick trog-to-trog tell gap via an appended `last_tell_start` ctx field; fallback to `TELL_SPACING` | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD9 | All three attacks are evadable by the DM-B reflex | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD10 | Interrupts keyed by `HitSource` (5.2 table): bolts never interrupt the Basher or the Virtuoso; the Basher has an 18-tick window and a 120-tick budget | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD11 | The bench is an 80 px partition prop (`break_hits` 3, shove 160) and the Basher's smash shoves at 200 with a 12 px overlap minimum; | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD12 | The declared static fallback of 5.4 | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD13 | The Test of Strength has Basher + Virtuoso only; no Pygmy reinforcement | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD14 | T5 is the Juicer arena's zone A (reception, turnstile); no duplicate room | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD15 | Dynamite damages troglodytes and interrupts them | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD16 | A per-room companion slot feed (targets and threats); no rescue, no teleport, no invincibility | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD17 | Target choice (amended, review D3): the nearest living hero in x, outside `protected_spawn`, whose hurtbox can meet the species' attack band in y; tie to Donut; | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD18 | The gate is the invariant (row 1) plus KC19 (row 2) on the Test of Strength; T1..T3 are reported | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD19 | No death profile, gore or slow motion added; the `down` frame holds | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD20 | The TROG atlases are wired at runtime with the label "art review pending" (`prototype_fixture_unapproved`); wired in the TROG-A review round (AD39..AD41) | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD21 | Wake causes `Proximity` and `Hit`, recorded in the `Wake` event and the state hash. A hit, blast or contact on a Dormant trog deals its damage and sends it to Rest (no tell). | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD22 | The T1 lesson is "punish the landing low": Carl's CF02 stomp (box y 0..10, startup 9, inside the 14-tick land) or Donut's pounce (y 0..17), then a jab in Rest against the 33 px idle box. | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD23 | `bas_a` in `trog_strength` has `trigger_x` 600 | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD24 | The walk body is the idle and walk hurtbox's x range, stepped against standing solids like a leash end; the Virtuoso's leash ends (368, 624) touch the shelf and bench faces exactly | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD25 | The Pygmy's airborne boxes are measured from its airborne pivot, on the integer parabola of 5.2; the presenter adds no other offset | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD26 | A bench-smash shove's `credit` is the baiting hero's player (the far-face smash: its target's player), `None` for the bot; `BenchSmash` names the baiting hero either way | #309 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD27 | Re-review R1. The solo bot bias applies to the target rule after the band and reach filter: `trogq04_ai_bias_px` 40 (the boss's `ai_bias_px` and the goblins' `target_ai_bias_px`), plus a switch hysteresis ... | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD28 | The `Launch` event (code 3) marks the start of the active window of every species (the leap, the downswing, the lash); `Land` (code 4) is the Pygmy's landing only | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD29 | Re-review R3. A lethal hit on a Dormant troglodyte emits no `Wake` event: Dormant goes straight to Down, and only the existing `EnemyDefeated` is logged | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD30 | Re-review R4. With no living hero outside `protected_spawn` a troglodyte has no target: it holds still and never tells | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD31 | Re-review R5 (the reviewer's recommendation, in part): reachability in x is part of the candidate filter by the leash reach (the leash band widened by the hitbox's furthest edge); | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD32 | The Basher holds at the open band's centre (31 px) and the Virtuoso at `standoff_px`; each backs off when a hero is inside; the Pygmy only closes in | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD33 | The knock values of 5.1 are y-down layout numbers (the plan's rooms are y down): the data file stores the world-frame (y up) values 120, 160 and 60 | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD34 | `trog_run`'s exit is x 2224..2288, the last 64 px of the play area, because the walls take 16 px at each end; the plan's 2256..2304 lies outside it | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD35 | The T3 platform (64 px, slab 12) is declared a Carl-class surface (`gates: surface 0, class carl`): the validator measures that base Donut cannot jump it, so it is a Carl vantage. Boosted Donut reaches it; | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD36 | `EvadeSource::TrogBlow` is appended to the DM-B allow-list (five sources), so AD9 is real: Donut's reflex may evade any troglodyte blow, key `(slot target id, tell start tick)` | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD37 | A troglodyte needs a `trigger_x` (rule V0), and the roster's `delay_ticks` is an extra wait after a wake. It has no perch, gate, fallback or mount | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD38 | `trog_run`'s sidecar sets `max_living` 3 (the plan said 2): the validator's cap counts the roster, and three single-enemy beats are fought one at a time by geometry, never alive together unless the player ... | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD39 | The atlas frame map (presenter only, `src/trog_art.rs`): idle (0..=2, looped on the manifests' ticks) for Dormant, Rest and a standing Approach; walk (3..=6) while its x changed in the last 6 ticks; | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD40 | The art faces right; a facing of -1 mirrors the sprite and its pivot. The sprite stands its pivot on the troglodyte's pivot, the Pygmy's at its airborne pivot (AD25), so no other offset exists | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD41 | The labelled hurtbox rectangle stays as the fallback where an atlas is missing (the export was not run) and, faint, behind `--debug-combat`; the stand-in telegraph of the committed hitbox stays in both. | #310 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD42 | `trog_strength`'s exit is x 944..1008 (`goal_x` 992), the last 64 px of the play area, because the walls take 16 px at each end; the plan's 976..1024 lies outside it (the AD34 rule). | #312 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD43 | The declared static fallback is taken (AD12): the validator rejects the 80 px `bench_stack` partition (a `solid_intact` prop is at most 48 px), so the bench is a static `solid` at 640..672 (48 px first, 80 px ... | #312 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD44 | `trog_eval` decisions: the stationary policies and the reflex variant are `corral_eval`'s (one generalisation, `LlamaObs::rect`, `None` for a llama, so the corral report is byte-identical); | #312 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD45 | The gate is not weakened. Under AD43's first 48 px bench G-1 and G-2 failed with a measured counterexample (7.3; kept as history). Under AD46 (80 px) every row passes: `docs/evidence/trog-b/eval/report.txt` | #312 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD46 | The bench is an 80 px static `solid` (640..672, y 500; the shelf stays 48 px at 288..352). The validator gains `MAX_STATIC_SOLID_PX` 80, used only for a plain (non-sliding) sidecar static of kind `solid`; | #312 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |
| TROG-AD47 | `trog_eval` ties its two surfaces to the sidecar: `SHELF_RISE_PX` 48 and `BENCH_RISE_PX` 80 (the natural starts, the shelf and bench tops of the grid); | #312 | `assets/data/gameplay/enemy_trog_v0.json`; `assets/levels/floor01/f01_trog_*.{room,encounter}.json` |

### TROG-ART (20)

| ID | Adopted value | PR | Change by |
| --- | --- | --- | --- |
| TROG-ART-AD1 | Route B + silhouette gate (score 3) | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD2 | One contract `floor01_trog_kit.json` for both rooms, extending the gym kit, applied with `ring_kit::dress_with` | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD3 | Bays 1 and 4 of the run and bay 3 of the strength room are bricked up (`backwall_fill`) and their `arch_chain` removed | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD4 | The fist hole sits at (1312, 440), not the greybox (1344, 500) | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD5 | The refuge, bell step and shelf faces are new executor pieces from the heat shelf body, one stone step darker | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD6 | The gym mirror is not reused whole: `trog_mirror_shards` replaces it, at (792, 470) | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD7 | Of each doubled composer torch pair (run 2240; strength 32/48 and 944/960), the y 432 torch and its pool are removed | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD8 | The bell is dull brass (one ramp step darker than raw brass) | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD9 | The debris burst flies away from the breaker: authored leftward (the Basher smash), mirrored for a breaker left of the bench | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD10 | The lit read of section 10.1 is the operational form of plan 16's "troglodytes the brightest, highest-contrast shapes": against the dressing, not against the heroes | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD11 | The authoring script is `scripts/author_trogarea01.py`, not the brief's `author_trog01.py` | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD12 | No audio; no Pygmy-specific dressing beyond the crater | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD13 | Stage 1 ships alone: no stage-2 piece, atlas or contract block is authored | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD14 | Two atlases for stage 1, `floor01_trog_run_wall` and `floor01_trog_run_props`, 8 px padding; the crater and the step pieces are `structure` inside the props atlas (per-piece layer override, the camp pattern) | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD15 | The binary draws none of `f01_trog::decor` and prints `decor=dressed`; the `F01_TROG_DECOR` lines are removed | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD16 | The troglodytes' pose and position on a still are measured by masked normalised cross-correlation of the atlas frames; a still is gated when an idle frame (0..2) is in view | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD17 | New pilot ticks for stills TROG-A did not take: 20 (`t0_spawn`), 440 (`basher_dormant`), 840 (`virtuoso_dormant`) | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD18 | A piece's lit mean in a window still counts only the pixels visible in the layout and explained by the lit model ( | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD19 | `trog_kit::dressing_of` returns `Option`: a troglodyte room whose contract has no `room_dressing` block composes plain (`launch` stays `launch_with(.., true)`), and the binary prints `decor=dressed` or ... | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |
| TROG-ART-AD20 | The composer's blue `arch_threshold` sills of the two bricked bays, (208, 544) and (1216, 544), are removed | #311 | `assets/source/terrain/floor01/floor01_trog_kit.json`; pixels `scripts/author_trogarea01.py` |

## Questions with an adopted recommendation (Q and OQ IDs)

Each question was raised in a spec or a PR with a recommended default; the orchestrator adopted the recommendation (the owner's design-defaults rule). The right-hand side of the arrow is the adopted answer. Questions that are still genuinely open for the owner are marked in the entry point's section 7.

### DZ questions (28)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| DZ-OQ1 | Narrative placement: does this non-canonical revenge beat belong in the definitive story route after the Boiler, or ... -> In the slice as room 3; it can be dropped from the run by unlisting the row | #273 |
| DZ-OQ2 | Same machine or same model? Is it the opening's very Dozer and crew, or another of the same family? -> "Another Dozer of the same family"; only display text and rider presentation would change | #273 |
| DZ-OQ3 | Moved to the game-feel items (the release window of about 1.35 s) | #273 |
| DZ-Q01 | `dzq01_wheel_axle`, `dzq01_wheel_r_px` -> (124, 49), 44 | #273 |
| DZ-Q02 | `dzq02_push_box` -> x −84..+80, y 0..160 | #273 |
| DZ-Q03 | `dzq03_impact_collider` -> x +19..+172, y 0..120 | #273 |
| DZ-Q04 | `dzq04_wheel_damage`, `dzq04_knock_x_px_s`, `dzq04_knock_up_px_s`, `dzq04_source_id`, `dzq04_hp_constant` -> 255; 180; 140; `0x445A`; 255 | #273 |
| DZ-Q05 | `dzq05_clang_hit_stop` -> the move's own hit-stop, metal hit-spark, no state change | #273 |
| DZ-Q06 | `revenge.dzq06_cruise_px_s` -> 160 | #273 |
| DZ-Q07 | `revenge.dzq07_catch_up_px_s`, `revenge.dzq07_catch_up_gap_px` -> 185, above 260 | #273 |
| DZ-Q08 | `revenge.dzq08_mercy_px_s`, `revenge.dzq08_mercy_gap_px` -> 150, below 96 | #273 |
| DZ-Q09 | `revenge.dzq09_accel_px_s_per_tick` -> 4 | #273 |
| DZ-Q10 | `revenge.dzq10_charge` -> `null` | #273 |
| DZ-Q11 | sidecar `dozer.dzq11_trigger_x`, `dozer.dzq11_idle_trigger_ticks` -> 256, 300 | #273 |
| DZ-Q12 | `dzq12_rumble_ticks` -> 90 | #273 |
| DZ-Q13 | sidecar `dozer.dzq13_emerge_front_x`; `dzq13_emerge_speed` -> 32 (the gate's right edge, O = −136); `"target"` (the band's target on the first tick) | #273 |
| DZ-Q14 | sidecar `dozer.dzq14_stop_x`; `dzq14_brake_ticks`, `dzq14_brake_slide_px`, `dzq14_strain_ticks`, `dzq14_lethal_in_brake` -> 1000; 24, 4, 90, true (off after the brake) | #273 |
| DZ-Q15 | `dzq15_stunned_ticks` -> 48 | #273 |
| DZ-Q16 | `dzq16_nav_lead_px`, `dzq16_nav_grid_px` -> 144, 16 | #273 |
| DZ-Q17 | `dzq17_rescue_band_px`, `dzq17_rescue_ahead_px`, `dzq17_rescue_zone_margin_px`, `dzq17_rescue_past_stop_px`, ... -> 24, 48, 24, 48, 16, 32, 24, 120 | #273 |
| DZ-Q18 | sidecar prop `stop_frame` (`kind: weight_rig`, `target`, `hang_px`, `remnant`) and hazard `frame_fall` (`rect`, ... -> footprint x 640..720 × 96, hang 168, remnant × 16, telegraph 30, fall 8 (engine default), hero damage 1, push 120, ... | #273 |
| DZ-Q19 | the prop's `target` -> x 800..816 × 64, back wall | #273 |
| DZ-Q20 | sidecar roster `rider_a` / `rider_b`: `mount`, `dzq20_land_dx` (32, 72), `dzq20_down_ticks` 45, `dzq20_nudge_px` 32, ... -> brawler, coward | #273 |
| DZ-Q21 | sidecar `max_living`, `locks` -> 3, none | #273 |
| DZ-Q22 | room file and sidecar geometry -> width 1344, `ceiling_px` 228, spawn Carl 192 / Donut 160, checkpoint x 48..96, lintel solid x 1040..1344 from 112 to ... | #273 |
| DZ-Q23 | `dzq23_melee_recovery_ticks`, `dzq23_min_release_window_ticks`, `dzq23_min_visible_lead_ticks` -> 24, 45, 45 | #273 |
| DZ-Q24 | `dzq24_ca2dz_autonomous_rescues_max`, `dzq24_ca2dz_assist_rescues_per_run_max`, `dzq24_ca2dz_toward_regroups_max`, ... -> 0, 1.0 (mean over the matrix), 0, 300 (the plan's gate value) | #273 |
| DZ-Q25 | `revenge.dzq25_no_hero_ahead_target` -> `"cruise"` | #273 |

### DMA questions (22)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| DMA-Q01 | Airborne evade frames are lowest-row anchored; the presenter applies the cosmetic arc as a draw offset | #281 |
| DMA-Q02 | The evade twist is facing-relative: the rear always rolls toward the camera | #281 |
| DMA-Q03 | The puddle mouth is drawn inside the Donut sprite so the front rim can cover her body | #281 |
| DMA-Q04 | A tail swap is baked per locomotion frame (the missile head-swap pattern) | #281 |
| DMA-Q05 | The glint is a 3x3 plus from the VFX strip instead of a 2x2 square | #281 |
| DMA-Q06 | Afterimages are retired (they read as a superhero speed effect) | #281 |
| DMA-Q07 | The swish is the VFX strip's two-frame arc on the attacker side (2 ticks) | #281 |
| DMA-Q08 | BlinkFizzled fires at the origin; she never left (8 ticks) | #281 |
| DMA-Q09 | Airborne arrival: the puddle mouth stays at her feet in the air; the splash drops still fall | #281 |
| DMA-Q10 | The tail swap is its own sheet (`donut_h40_tail_ready`), not tags in the moves sheet -> The atlas index is the missile head swap's `16 * phase + locomotion frame`, and the moves sheet keeps one tag table | #281 |
| DMA-Q11 | The size bound of acceptance 6 (-26 .. +25 for blink frames) is checked on the puddle frames b1, b2, a0 and a1. -> b0 is the melee `p0` body (its tail alone is 35 px behind the pivot) | #281 |
| DMA-Q12 | `twinkle_small` is two `w` pixels (one above the other), not a lone pixel -> A lone pixel is an isolated-pixel lint error, which always fails; two accent pixels are not orphans either | #281 |
| DMA-Q13 | The origin puddle plays `ring_open_0` and `ring_open_1` over ages 0-5 (3 ticks each), then `ring_close` over ages 6-14; -> The spec lists ring_open as 12 ticks at both ends but the origin must close at age 6 | #281 |
| DMA-Q14 | A missing atlas leaves its cue undrawn (her locomotion sprite stays); there is no code-drawn fallback -> The stand-ins are retired | #281 |
| DMA-Q15 | An evade head cel is the approved head translated; the key's strict-lint orphan cleanup may recolour at most 2 outline ... -> The cleanup is part of the frozen key's authoring record | #281 |
| DMA-Q16 | The streak is `streak_dash` tiles of 16 px from the origin column toward the destination with the 4 px `streak_head` on ... -> The brief's tiling, made concrete | #281 |
| DMA-Q17 | `flick_up`: the 8 tip pixels move (-1, -2) and the two cells opened under them take the stem's own colours so the tail ... -> Acceptance 7 | #281 |
| DMA-Q18 | The swish is drawn with its sprite centre at her sprite centre + (6 px toward the attacker, +2 px), travelling toward ... -> The old streak's place | #281 |
| DMA-Q19 | The glint is the strip plus centred on the tail-tip pixel of the frame actually drawn: the locomotion frame, or the ... -> `analyze_atlas` runs on both atlases | #281 |
| DMA-Q20 | The strip's puddle effects (rings and the fizzle ring) anchor their surface row 3 px above her foot row ... -> Review F1: on the foot row the ring's lower crescent showed as a second ellipse under the mouth and read as a saucer | #281 |
| DMA-Q21 | Review fixes F3-F8: `evade_untwist` has a 2 px belly seam, `evade_land` and the pinholes of e0, b1, a2 and tail-swap ... -> Independent review, PASS-with-fixes | #281 |
| DMA-Q22 | The fizzle heads are the CF02 `startle_puff` and `startle_settle` heads, whose ears are up, although section 5 says ... -> The brief names those heads; a flat-ear variant is a later design task (review note F7) | #281 |

### HA2 questions (3)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| HA2-Q1 | Whether the item chips should also appear on a future pause or loadout screen (none exists) | #286 |
| HA2-Q2 | Owner verdict on HA2-AD2 (chip on the notice, glyph in the socket): recommendation, keep it; fallback is the chip in the socket with a 58 px track | #286 |
| HA2-Q3 | Whether the movement caps should show A/D instead of the arrows for WASD players (the table lists both) | #286 |

### ST questions (1)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| ST-OQ1 | The right Boiler plume rolls over the drum base for 20 ticks; art unchanged, a follow-up could clip the plume at a flush prop (owner) | #288 |

### RNG questions (5)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| RNG-OQ1 | Should the camp lantern and the marker lantern be real A1 light emitters (one each) in 4b? -> Baked glow (RNG-AD11). | #290 |
| RNG-OQ2 | Who owns the shutter's stone-scrape and grit audio (#189 asks for "stone scraping" during the warning)? RING-D appends ... -> Silent art; RING-D reuses an existing cue. | #290 |
| RNG-OQ3 | Will the court's shutter become a moving machine (RING-C wire) in 4a, and in which room should the moving-state capture ... -> The moving frames are unit-tested only (RNG-AD6). | #290 |
| RNG-OQ4 | Where exactly does REST-B place `ring_marker_a` at its entrance? -> REST-B places it; this kit ships the piece. | #290 |
| RNG-OQ5 | Game feel: is one full ring per court plus two arcs enough "awe" for the RING beat, or does the owner want a larger arc ... -> The current layout. The owner reviews after merge. | #290 |

### DZ2 questions (4)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| DZ2-Q1 | The riders' tumble lands 32/72 px past the wheel front as simulated (AD9), not the design's 28/36 px; confirm or move the landing data | #291 |
| DZ2-Q2 | Glance continuity: a fall that will glance off the rear hull ends on the deck (closed by review fix F2) | #291 |
| DZ2-Q3 | Riders A and B wear no helmet; the lost helmet is skipped (the thrown pineapple stands in) | #291 |
| DZ2-Q4 | Tunnel lintel wired (closed by review fix F1); the Guild door arch overlaps it by about 24 px: confirm | #291 |

### HEAT questions (5)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| HEAT-OQ1 | Who draws the M3 gate grate (x 2048..2064, code-drawn greybox)? -> Out of this kit; the grate stays code-drawn. | #294 |
| HEAT-OQ2 | Should the ceiling slab (y 336..352) get a soffit like the Ring's? -> No soffit; the composer's brick shows. | #294 |
| HEAT-OQ3 | ~~The partition can only be seen in-scene after CORRAL-D.~~ Answered: it is in-scene since CORRAL-D (in-scene stills of ... -> Gate composites were the only view until then (`HEAT-AD1`). | #294 |
| HEAT-OQ4 | Soot on the bay piers? -> None; piers are composer structure and stay clean in this kit. | #294 |
| HEAT-OQ5 | Game feel: is the pen's density enough "the corral fights back", or does the owner want heavier burn? -> The caps of section 3; the owner reviews after merge. | #294 |

### LL3 questions (1)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| LL3-OQ1 | Game feel: with no lane band the rush's range end reads from the body alone; the muzzle pokes up to 17 px past the hitbox front (adopted: keep) | #295 |

### HC questions (3)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| HC-OQ1 | A reactive 'dead knock' on a refused post hit needs an unhashed observer field: not done | #296 |
| HC-OQ2 | Audio: out of scope until the room has an audio pass | #296 |
| HC-OQ3 | The white-wood flash and splinter spray on a qualifying post hit is not drawn: follow-up order | #296 |

### AR questions (9)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| AR-Q1 | Is the corral exit the canonical broadcast beat for the finished game's story, or only for the `f01_late` playtest run? ... -> Corral exit, `f01_late` run only | #297 |
| AR-Q2 | Is every clean llama kill too frequent for slow motion? (The corral can give two or three in a few minutes; -> Every clean, credited llama kill qualifies | #297 |
| AR-Q3 | Should a llama kill shortly after its rush ended `Miss`, `Blocked` or `Partition` earn a punish bonus? (`LlamaRushEnd` ... -> No bonus | #297 |
| AR-Q4 | Should chained authored charges and provoked rot stickers carry the thrower's credit into THROWN-MULTI? -> No: uncredited | #297 |
| AR-Q5 | Should a hero-credited hazard prop kill (`Environment { player }`) become a primary, or a MIXED-CHAIN partner? ... -> No | #297 |
| AR-Q6 | Should THROWN-MULTI need 2 kills instead of 3, or should a goblin-only stick reach legendary (raise the extra-kill ... -> 3 core kills on one tick; cap +2 | #297 |
| AR-Q7 | Does a party defeat cost viewers or become a humiliation spike? (AUD-Q3) -> No change | #297 |
| AR-Q8 | Should viewers persist past the floor end or across sessions? (A save contract is needed.) -> Process lifetime only | #297 |
| AR-Q9 | Should the reveal get a narrative card or System announcement beyond the panel line? (Written to the antihero tone, ... -> Panel line only | #297 |

### ENG2 questions (5)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| ENG2-Q1 | Should the death sheets keep strictly to the 17 ENG-01 keys, with blood mapped to `o`? -> 20 keys (ENG2-AD9, ENG2-AD15). | #300 |
| ENG2-Q2 | Is one runtime-rotated pot cel enough, or should there be 3 drawn rotation cels with spin off? -> One cel (ENG2-AD8, ENG2-AD16). | #300 |
| ENG2-Q3 | Ram run timing: 4 ticks a frame (5 frames per ram) or 3 (about 7)? At 240 px/s the feet still slide at either value. -> 4 ticks (ENG2-AD17). | #300 |
| ENG2-Q4 | Get-up continuity: the wrench is absent in ENG-01 `down` 15 to 17 and in the sit, and reappears in the shove. -> As frozen: it reappears on the shove (ENG2-AD18). | #300 |
| ENG2-Q5 | Stomped: should the pot stay on, flattened like the crush, instead of popping off? -> It pops off (ENG2-AD6, ENG2-AD19). | #300 |

### REST questions (10)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| REST-Q1 | Does A1 feel like a 10-second upgrade reveal or a roadblock? -> owner, game feel | #301 |
| REST-Q2 | Which survivors appear, with which names, lines and sprites? -> owner and narrative review | #301 |
| REST-Q3 | Refuge jingle, fluorescent hum, camp ambience -> later original-audio order | #301 |
| REST-Q4 | Should a REST checkpoint restore health or mana? -> owner, game feel | #301 |
| REST-Q5 | The rolling yellow ball gag -> AREA-FASTFOOD-01 review | #301 |
| REST-Q6 | What "the gauntlet interrupts more strongly" means on the bag -> combat design | #301 |
| REST-Q7 | Does the Ball of Swine replace or follow `mastery`, and where does REST-C exit then? -> #192 integration | #301 |
| REST-Q8 | Slot order once #179, #189 R-regions and #191 land -> their integration steps | #301 |
| REST-Q9 | Should 2P also get a door hint? -> owner, after the playtest | #301 |
| REST-Q10 | Should audience decay freeze in rooms with an empty roster (the REST rooms), so the aftermath keeps the Juicer peak? -> owner, game feel | #301 |

### ACT1 questions (10)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| ACT1-Q1 | Should the chase/shore and shore/guild boundaries skip the room-clear panel (a seamless relaunch)? -> The standard panel | #303 |
| ACT1-Q2 | Runtime audio for the chase (music on the trigger, engine, whistle, impact, sudden quiet) once the game has an audio ... -> No audio; the provisional files stay unused | #303 |
| ACT1-Q3 | End-on wedge art (≤ 192 px wide, DOZ-02 or ENV-02's wall-breach contingency) -> The greybox wedge draw | #303 |
| ACT1-Q4 | The optional third goblin at spawn C -> Two goblins | #303 |
| ACT1-Q5 | A rescue budget for the opening (CA2-style), after a human session -> Zero wheel contacts asserted; counts reported | #303 |
| ACT1-Q6 | Riders on the opening machine, thrown at the barricade impact into the shore room -> None in the greybox | #303 |
| ACT1-Q7 | ENV-02 dressing of the three rooms (the art kit on the greybox) -> Greybox kit, decor rectangles | #303 |
| ACT1-Q8 | Guild dialogue, the skip / fast advance for repeat play (I-04) and the book chronology -> No text, nothing to skip | #303 |
| ACT1-Q9 | The #179 first-run duration hypothesis (25–40 s) and the reach margin, after a human session -> Values above | #303 |
| ACT1-Q10 | After a hero's defeat in the chase the machine keeps running through the defeat delay and visibly breaks the barricade ... -> The break is shown; nothing in the sim changes | #307 |

### FF questions (6)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| FF-Q1 | Should a neutral white/grey ramp be added to the floor palette in a palette PR, for a brighter "unexpectedly clean" ... -> owner art review | #305 |
| FF-Q2 | Is cool steady strip light in the refuge acceptable next to atlas row 09's gym-only accent rule? Answered by FF-AD22 ... -> owner art review | #305 |
| FF-Q3 | Should the box gain a faint glow, if it gets lost at 1x in play? Answered by FF-AD23 (no, shorten the glint first) -> owner game-feel review | #305 |
| FF-Q4 | REST-Q5: animate the stray ball rolling in (a bounded, non-collidable loop)? Answered by FF-AD24 (static) -> owner game-feel review | #305 |
| FF-Q5 | Should Donut's paw-switch frames (ANIM-INTERACT-01) play on the shelf moment? Answered by FF-AD25 (not in this work ... -> owner, once wired | #305 |
| FF-Q6 | Owner note (gate note G9, non-blocking): the door's open lamp (dim olive `lichen`) is darker than its closed lamp ... -> owner art review | #305 |

### CAMP questions (6)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| CAMP-Q1 | REST-Q2: which survivors appear, with names, lines or native sprites? Until then the four figures stay anonymous ... -> owner and narrative review | #308 |
| CAMP-Q2 | Beat 3: should a survivor reposition the crates after the party passes (a bounded presentation-only animation of ... -> owner game-feel review | #308 |
| CAMP-Q3 | Should the figures breathe or shift (two-frame idles), and the sentry turn its head when the heroes pass? -> owner game-feel review | #308 |
| CAMP-Q4 | REST-Q6 (combat half): should the bag do anything in the sim (a hit counter, a stronger gauntlet reaction)? CAMP-AD7 ... -> combat design | #308 |
| CAMP-Q5 | The bag's clang, the trailing weight clang of section C, the fire's crackle and the camp's quiet ambience -> REST audio order | #308 |
| CAMP-Q6 | Should the aftermath niche show a fallen fluorescent tube on the floor instead of none (CAMP-AD9)? -> owner art review | #308 |

### TROG questions (15)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| TROG-Q1 | The optional second Pygmy in T1 -> Not authored | #309 |
| TROG-Q2 | A Pygmy reinforcement in the Test of Strength once human trials show separation -> None (TROG-AD13) | #309 |
| TROG-Q3 | Is a stationary clear from the bait zone (both enemies, bench smashed by rule b) an acceptable skill answer? -> Yes; it is reported, not gated (the player chose the risk spot in both reaches) | #309 |
| TROG-Q4 | Should dynamite (a `Blast`) break the bench? -> No (the partition's rule); revisit with the owner's play | #309 |
| TROG-Q5 | Is a 4-HP Basher with the 120-tick interrupt budget the right weight after play? -> The values of 5.1 | #309 |
| TROG-Q6 | Troglodyte death profiles (species rows, finishes) -> None; the `down` frame holds | #309 |
| TROG-Q7 | Runtime audio (the slam through stone, the bench impact, gym hum) once the game has an audio path -> None | #309 |
| TROG-Q8 | Audience reactions to troglodyte kills and the bench smash -> None authored | #309 |
| TROG-Q9 | The #191 time hypothesis (6–8 min first run, 3–4 experienced) -> Measured from a human session only | #309 |
| TROG-Q10 | The Juicer's 36 HP feel after this lead-in (TROG-16) -> Unchanged; a separate boss-balance PR if the owner asks | #309 |
| TROG-Q11 | A sidecar leash field if a later room needs a non-default leash -> Tuning leash only | #309 |
| TROG-Q12 | Does the Virtuoso's tongue dead zone (inside +56) read clearly at 1x, or does it need a body-contact shove? -> The dead zone, as the art draws it | #309 |
| TROG-Q13 | (Also the R5 remainder, TROG-AD31: a hero a solid cuts off, such as Carl behind the intact bench or on the T3 platform ... -> Accepted as Carl's problem; reported in `trog_eval` (7.3) and TROG-F06; | #309 |
| TROG-Q14 | Should the sidecar validator let a partition prop be as high as Carl's class limit (80 px) while a prop stays at 48? It ... -> Deferred; not in this PR | #312 |
| TROG-Q15 | (Closed by TROG-AD46.) Is a hopping stationary Donut's over-the-bench shot acceptable? -> Closed: the 80 px bench removes the shot, so it is neither accepted nor reported, and the gate is not narrowed | #312 |

### TROG-ART questions (6)

| ID | Question -> adopted answer | PR |
| --- | --- | --- |
| TROG-ART-Q1 | The lane ceiling slab (y 336..351) has no art in the plain composition. Should it get a beam or vault piece? -> No piece in this order; follow whatever the Boiler room shows for the same slab, in a kit-wide follow-up | #311 |
| TROG-ART-Q2 | Is the break tick's `BenchSmash` event reachable from the scene for the debris direction? -> If not, keep the authored leftward burst for every break and note it; a hand break from the left is the rarer case | #311 |
| TROG-ART-Q3 | The slam through stone, the bench impact and the gym hum (TROG-Q7) -> none until the game has an audio path | #311 |
| TROG-ART-Q4 | More Pygmy territory dressing (a nest, gnawed bones) in T1? -> no; keep the run readable and the Pygmy's lesson about its landing | #311 |
| TROG-ART-Q5 | The static fallback bench (TROG-AD12, a 48 px solid) has no art -> if the fallback fires, the executor draws `trog_bench_low` 32 x 48 from the intact key's lower rows plus the cap, as an ... | #311 |
| TROG-ART-Q6 | The troglodytes' mean value is low against their surround (`delta_L` 0.4..11.5 in the gate) and Carl is brighter than ... -> a rim or value pass on the TROG sprites in the troglodyte family's own review, not in this kit | #311 |

Totals: 442 adopted-default rows (AD and IMP), 139 question rows.
