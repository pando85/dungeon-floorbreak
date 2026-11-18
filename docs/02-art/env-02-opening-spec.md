# ENV-02: Floor 1 opening route pieces (design spec, phase 1)

**Status:** full set drawn (Sonnet); independent Opus review (2026-11-18) passed with execution items, fixed in the review fix round (see the execution record). This follows the [AGENTS.md art pipeline](../../AGENTS.md#model-routing-for-art-work): design and keys by Claude Opus (2026-11-17), full-set execution by Claude Sonnet (2026-11-17), **independent review by a separate Claude Opus agent done (PASS WITH EXECUTION ITEMS, fixed); owner review at the end only.** Nothing here is wired into `floor01_scene`; the integrator owns that. The evidence images are Pillow previews; the Bevy captures come when the integrator wires the route in.
**Builds on:** the owner-approved ENV-01 kit ([kit doc](floor-01-catacomb-kit.md), `env-kit-v2`), [ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md) (A1 frame: 640×360 pixel layers plus a full-resolution lighting pass, deep floor, walking line at y 308).
**Drives:** CHASE-01 (the pursuit, segments C and D of the [opening handoff](../07-agents/floor-01-opening-encounter-handoff.md)) and CP-03 (the environmental kill in segment E, [combat probes](../07-agents/floor-01-integrated-combat-probes.md), [Fixture 3](../01-design/combat-feel-hybrid-physics.md)).
**Evidence:** [`docs/evidence/env-02/`](../evidence/env-02/README.md).

## Summary

| Need | Design answer |
| --- | --- |
| Chase passage (C to D) | **S3**, a 768 px maintenance passage under a lowered ceiling (two options, below), with one copper run, a valve on a drop, an iron hatch, two timber shoring sets that the pursuer smashes, and two 1-tile jump humps. It opens into **S4**, the full-height widened passage. |
| Dodge out of the lane | An **alcove niche** on a **1-tile ledge** in S4 (a `safe_pocket` marker): **104 px clear**, too low for the 155 px pursuer (DOZ-01 refuge range 64 to 116 px) and 80 px wide for Carl and Donut together. Its entrance is 176 px past the charge trigger. The pursuer charges past it and cannot brake. |
| Overshoot | A **timber barricade** closes S4. The players are stopped by it; the charging pursuer smashes through it, overshoots, cannot make the turn and noses into the S5 arch window, leaving the lane. Its riders bail out. |
| CP-03 breakable | A **rotten raking shore**: a diagonal timber propping a **slumped masonry block** that is sliding out of the wall. 3 hits break it; the block drops straight down into an 80 px fall region. The strike point is outside that region. |
| Guild exit | A **guild portal**: a clean dressed-stone arch with a heavy double timber door, a lit grille and a light leak (closed), then a warm open doorway with a lamp (open). A hanging sign with a lantern pictogram, no text. |
| Route | `assets/levels/floor01/opening_route_proposal.json` (ceiling option R) and `..._ceiling_t.json` (option T): 224×25 tiles, 3584×400 px, six segments, 23 markers. The ENV-01 sample layout is unchanged. |

## Why this prop for CP-03

The **raking shore** is the brief's "rotten timber shoring post under a loose masonry chunk", set at an angle:

- **It belongs here.** Timber shoring is how a dry brick catacomb is patched. No machine, no copper, no workshop look (the area-kit no-go list keeps boilers in the later workshop), and no story equipment: Carl breaks it with bare hands.
- **It teaches itself.** Seconds earlier the players watched the pursuer splinter two timber sets and a timber barricade. The shore uses the same timber, so "timber breaks" is already learned.
- **Cause and effect read in one glance.** It is flimsy, rotten timber holding a heavy pale stone, so the danger is obvious before anything happens. The diagonal puts the foot, where Carl hits it, **outside** the drop zone, so the player sees *I hit this, that falls over there*. A vertical post would put Carl under the load.
- **It is small.** One prop, one falling chunk and no support graph. It is not the later L3 collapse technology ([destruction.md](../03-tech/destruction.md)).

Rejected: the brass boiler cap (copper or brass machinery belongs in C and the workshop, not in E) and the hanging cauldron chain (a goblin camp prop, and a three-part chain of chain, pulley and weight that is harder to read at 1x).

## Grid, palette, layers (unchanged from ENV-01)

- 16 px tiles, virtual 640×360 / 640×400, floor back line y 288, walking line y 308 (4 px behind the 24 px deep-floor lip), actor pivot at walking line − 1 (0 px sink).
- Palette `floor01-catacomb-v2`: **no new colours**. Atlas use: `floor01_opening_props` 36, `floor01_opening_decal` 16, `floor01_guild` 32, `floor01_soffit` 12, `floor01_shore_dust` 4, `floor01_block_impact` 6, `floor01_splinter_burst` 6 (max 63 each; all colours are ENV-01 palette entries, tested).
- Timber uses `wood_00..05` plus `floor_07/08` for torch-lit edges. Fresh splinters use `stone_06..09` (pale, warm). Dry rot uses `mortar_01..03` and `iron_03..05` (grey-brown, crumbly, never black). Dressed stone uses `stone_*`; the guild glow uses `flame_*` and `gal_warm_*` (these bloom, by the existing `emissive_colours` rule).
- Layers and draw order are inherited: `backwall_decal` (3) for the alcove and the wall cavity, `prop` (6) for everything else, actors (7) in front of all new pieces. **All pieces are non-collidable** (contract rule). Any blocking is an authored gameplay rect (marker), never a piece.
- Light: the pieces carry baked light from their nearest torch (S5 torch to the right of the block; S4 torch to the right of the niche). Emitters exist only on the guild portal (presentation only).

### Contract: `extends`

`assets/source/terrain/floor01/floor01_opening_kit.json` is an `env-kit-v2` **extension contract**. It has `"extends": "<ENV-01 contract>"` and adds seven atlases, 29 pieces and the 10 soffit tiles. `scripts/env_kit.py` (`resolve_extends`) merges it onto the base. It inherits the palette, layers, coordinates, walking-line rule, roles and lighting schema, and must not redeclare them. Atlas names and ids must be new. The base file is never modified, and layouts that name the base keep validating against the base alone. **The Rust loader (`src/env_kit.rs`) does not support `extends` yet.** That is an integrator task when the route is wired in: merge the same way, or flatten at load.

## Pieces

All sizes are in px; anchor = piece-local px.

| Piece id | Atlas | Size | Layer | Anchor / placement | States | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `shore_post_intact`, `_damaged_1`, `_damaged_2`, `_broken` | opening_props | 136×144 | prop | foot (16, 138); placed at (2952, 160) | 4, one rect | 11 px timber from foot (2968, 298) to head (3020, 168), about 68°. **Rotten foot (design fix round, re-keyed):** the lower 30 % stays warm wood (`wood_02/03`, never metal grey, no banding). The rot shows in the outline: a neck about 2.5 px narrower at the strike band (local y 111 to 130), 1 to 2 px bites eaten in irregular 2 to 4 px runs, dark bite rims on the shadow edge and pale `stone_06` torn fibres on the lit edge. It ends in a soft, splayed base with crumbs on the sole plate. d1: notch at the strike point with pale fibres. d2: deep notch, a split up the grain, the shaft bows 1.6 px and the head slips 2 px. broken: a jagged stub, and the upper shaft lies on the deck |
| `loose_block_seated`, `_settling`, `_slipped` | opening_props | 64×56 | prop | bottom centre (32, 48); placed at (3008, 120) | 3, one rect | **v2 chunk:** a heavy 3D mass of three dressed stones with an angular broken outline. The front face is lit toward the torch; extruded 6 px up and right, it shows a dim top face and a torch-lit right face, over a shadowed underside. One big split has a lit lip and crumbling mortar, plus a hairline. It is still keyed into the wall at the upper right. settling (hit 1): tilts 1.5° left-down, drops 1 px, first crumbs. slipped (hit 2): tilts 4° and drops 3 px, with a crumb shower: the "next hit drops it" tell |
| `loose_block_falling` | opening_props | 72×64 | prop | (36, 52) | debris sprite | the same chunk tipped 12° left-down, with crumbs trailing above. Gameplay moves it |
| `loose_block_landed` | opening_props | 96×32 | prop | bottom centre (48, 30) on the walking line | | two big chunks, bricks and a dust skirt. **Non-collidable** |
| `wall_cavity` | opening_decal | 80×64 | backwall_decal | block xy − (8, 8) | | the socket, drawn as depth: a lit left inner reveal of broken brick ends, a shadowed top reveal, a dark recessed back with faint bricks, a lit sill, a ragged outline and cracks. With the chunk seated, only uneven slivers show (above and on the left); after the fall it reads as a hole into the wall |
| `hazard_grit` | opening_props | 96×16 | prop | (2992, 288) | | pale grit and chips on the deck under the block: an in-world "things fall here" tell |
| `timber_set_intact`, `_smashed` | opening_props | 80×104 | prop | bottom centre on the back line; (1104 / 1424, 184) | 2 | mine-style set: two 8 px posts and a 10 px header about 120 px above the walking line, knee braces. smashed: stubs and two header pieces on the floor |
| `timber_barricade_intact`, `_smashed` (**DEPRECATED, background only**) | opening_props | 96×176 | prop | bottom centre on the back line; (2304, 112) | 2 | two 12 px posts, nine nailed planks and a diagonal brace. smashed: a left stub, a leaning right post, planks on the floor, one plank hanging by a nail |
| `maint_hatch` | opening_props | 32×32 | prop | top-left (1200, 208) | | riveted iron hatch in a stone frame, strap hinges, a small brass wheel |
| `alcove_niche` | opening_decal | 96×96 | backwall_decal | bottom centre on the ledge back line; (2064, 176); opening top y 188, so 104 px clear above the ledge walking line (292) | | segmental stone head, jambs, a lit inner reveal, a cold deep interior (`brick_cold`). The ledge deck covers its foot |
| `guild_portal_closed`, `_open` | guild | 112×136 | prop | bottom centre on the back line; (3368, 152) | 2, one rect | pale dressed-stone arch (68 px opening) with a carved lantern on the keystone. closed: double plank leaves, iron straps, a ring knocker, a glowing grille and leaks at the meeting stile and under the door. open: leaf edges and a warm interior (lamp, bench, far doorway, bright threshold) |
| `guild_sign` | guild | 40×44 | prop | wall plate; (3480, 168) | | iron bracket, chains, a board with a brass lantern pictogram. **No letters, no logo** |

Portal lights (presentation only; `#ffc27a`, `scene` group):

- **closed:** `grille` (r 70, i 0.22, glow 14) and `door_leak` (walking-line pool, r 110, i 0.2, squash 0.3);
- **open:** `interior` (r 240, i 0.5, glow 52) and `spill` (walking-line pool, r 170, i 0.5, squash 0.34).

## Chase passage and the two ceiling options (interface with DOZ-01)

**DOZ-01 interface** (decided by a separate Opus agent, 2026-11-17): the h120 pursuer is **155 px tall including the stack and 259 px long**. It needs a chase ceiling at least 164 px above the walking line, and 200 px or more is recommended so smoke and riders read. A refuge it cannot follow is 64 to 116 px clear. Its charge closes about 82 px over 90 ticks and starts only when the gap is 140 px or more. These values are in `design_parameters.pursuer_interface` in both layouts.

The parameter is the **clearance**: the walking line minus the underside of the lowered cornice in S3 (columns 61 to 104, x 976 to 1680). The pursuer drives on the base walking line and crushes the 1-tile humps, which are a presentation event. Over a hump, the players' own clearance is 16 px less.

| Option | Lowered rows | Cornice underside | Clearance | Fits pursuer total height (16 px margin) | Feel |
| --- | --- | --- | --- | --- | --- |
| **R** (committed default, recommended) | 0 to 5 | y 96 | **212 px** | ≤ 196 px | still a low passage after the 276 px corridor; 57 px above the pursuer, so smoke and riders read |
| **T** | 0 to 7 | y 128 | **180 px** | ≤ 164 px | tight: 25 px above the pursuer. The smoke flattens against the soffit and the stack may scrape the cornice (spark VFX) |

**Rule:** both options meet the DOZ-01 minimum of 164 px (tested). **R** meets the 200 px recommendation and is the default; **T** is the tighter, more claustrophobic alternative for the owner to compare. The layouts differ only inside the lowered span (tested). DOZ-01 also needs the S2/S5 arch window, 192 px wide and 218 px tall (y 70 to 288), for its entry and its wedge. The 259 px body is longer than the window is wide, so the turn-in and the wedge are end-on (rear view) presentation poses, at most 192 px wide. Its riders need room to bail out to the right of x 2760.

The soffit mass above the low cornice uses the dark `fill_1/2` blocks with one `ceiling_fill` course, so it reads as heavy masonry rather than blue brick. The step at each end borrows `wall_end_*` and `ceiling_corner_*` tiles (see Sonnet task 3).

**Jump and duck lanes:** two 1-tile humps (columns 73 to 76 and 91 to 94) for hop-or-step pacing while fleeing. **No duck lane**: there is no crouch input, and the 120 px timber headers are presentation, not obstacles. A duck lane is deferred until a crouch or slide exists.

## Route proposal (layout summary)

`opening_route_proposal.json`: 224×25 tiles (3584×400). The ENV-01 schema is unchanged; segments add `beat` (the handoff letter) and `purpose`.

| Segment | x | Kit grammar | Handoff beat | Content |
| --- | --- | --- | --- | --- |
| S1_intro | 0 to 640 | A | A | left end wall, a pooled torch, a closed timber door, a pilaster; spawn at x 72 |
| S2_arch | 640 to 944 | B | B | arch bay with a chain; the pursuer's entry window |
| S3_chase | 944 to 1712 | C | C | low passage (R or T): copper run, valve, hatch, two timber sets, two humps, one pooled torch, one burnt-out sconce |
| S4_widened | 1712 to 2608 | A | D | full height; two pooled torches; charge trigger at x 1888; alcove ledge at x 2064 to 2160 (104 px clear); barricade at x 2304 to 2400 |
| S5_skirmish | 2608 to 3296 | B | E | arch bay (wedge); the raking shore set; a pooled torch at x 3112; goblin spawns |
| S6_guild | 3296 to 3584 | A | E | guild portal and sign; protected exit |

Markers (ids are IP-neutral):

- `player_spawn`;
- `checkpoint_chase` (x 1008) and `checkpoint_post_chase` (x 2640);
- `trigger_chase_start` (x 1072);
- `pursuer_spawn` (S2 window) and `pursuer_path` (5 points along the walking line);
- `clearance_gauge_s3`;
- `timber_set_a/b` (`pursuer_breakable`) and `jump_hump_a/b`;
- `pursuer_charge_zone` (x 1888 to 2400; the refuge entrance is 176 px ahead) and `safe_pocket_alcove` (`clear_height_px` 104);
- `barricade` (`scripted_blocker`) and `pursuer_wedge` (S5 window);
- `breakable_shore` (3 hits, strike band y 271 to 290);
- `falling_block` and `hazard_fall_region` [3000, 168, 80, 140];
- `enemy_spawn_a/b/c` (x 3136, 3176, 3216; `small_melee` ×2 and an optional `small_engineer`);
- `protected_exit_region` (x ≥ 3232) and `exit_guild`.

The full plan is in `env02_route_plan_unlit.png`.

## Gameplay notes (proposals for the integrator; not implemented)

### CHASE-01

1. When the leading player crosses `trigger_chase_start`, the pursuer appears in the S2 arch window behind them. Its turn-in from the gallery is presentation (about 0.8 s, drawn between `backwall` and `structure`). It then follows `pursuer_path` on the walking line.
2. S3 pressure: pursuer speed is a little under Carl's run (proposal 130 px/s against Carl's 145 px/s). Contact with an actor is a hit or a fail; respawn at `checkpoint_chase`. The timber sets swap to `_smashed` when the pursuer's front edge crosses their rect: an authored event with splinter VFX, no collision and no damage.
3. S4 charge (DOZ-01 numbers): when the leading player crosses x 1888, the pursuer charges if the gap is 140 px or more. The charge closes about 82 px over 90 ticks. The refuge entrance (the ledge at x 2064) is 176 px ahead: 57 ticks at Carl's 185 px/s run and 73 ticks at 145 px/s, inside the 90-tick charge with at least 17 ticks for the 16 px hop (tested). The players must be in `safe_pocket_alcove` (feet on the ledge, inside the rect). It is 104 px clear, so the pursuer cannot follow, and pursuer contact skips them; they draw behind it with a slight darkening. Its momentum carries it past the alcove into the barricade 240 px further on. When its front hits the barricade, its 259 px body still covers the alcove entrance, so nobody steps out into its path early (tested).
4. **Donut is never trapped:** the pocket fits Carl and Donut side by side, and the barricade blocker is full height (y 32 to the walking line), so her higher jumps cannot strand her alone beyond it. There is no one-way drop anywhere on the route, and every step is at most 1 tile. `barricade` blocks actors until the pursuer reaches it. It is then removed in the same tick and the art swaps to `_smashed`. **Softlock guard:** if the pursuer is not charging 8 s after the leading player enters S4, the scripted charge starts anyway, so the barricade always opens.
5. Overshoot: the pursuer runs on to `pursuer_wedge`, cannot make the turn and noses into the S5 arch window. It leaves the lane with no collision and plays a stuck wheel-spin. The riders are thrown forward to `enemy_spawn_*` (a presentation arc), and the next checkpoint is `checkpoint_post_chase`.

### CP-03

1. **Hits to break: 3.** Any melee hit whose box overlaps the shore's strike band (x around 2975, y 271 to 290: Carl's jab at +16 to 22 px and his kick at +15 to 21 px ahead of the pivot both reach) advances intact → damaged_1 → damaged_2 → broken. Damage is one per hit event, deduplicated per attack; held inputs never repeat it. Goblins do not damage it.
2. **Telegraph:** each hit moves the block too. damaged_1 swaps it to `_settling` (it tilts and the first crumbs fall), and damaged_2 swaps it to `_slipped` (it tilts further, drops 3 px and sheds a crumb shower). That is the "next hit drops it" tell. The causal chain reads at 1x: rotten shore foot → block slips → it falls into the region marked by the grit.
3. **Fall:** 6 ticks after the break, the block (`loose_block_falling`) drops straight down from bottom y 168 to the walking line, 140 px. That takes about 0.52 s at the hero's 1050 px/s² gravity, and the fall can be tuned. **One impact event, on the landing tick:** every actor whose hurtbox overlaps `hazard_fall_region` takes it. Goblins are defeated, with the kill credited to the player who landed the breaking hit. Proposal: players inside take one hit and knockback (it is a real risk), but the designed strike position is outside the region.
4. **Hazard footprint:** [3000, 168, 80, 140]. That is the block's 64 px plus 8 px each side. Carl at the strike position (pivot about 2956, right edge about 2968) is 32 px clear (tested).
5. **After:** `loose_block_landed` sits on the walking line and is **non-collidable**. Navigation and collision are unchanged, so the exit stays reachable in any event order: shore first, goblins first, or the shore never touched. The direct melee kill is always the alternative.
6. **Luring:** the riders land at x 3136 and later and walk left toward the players, through the fall region. Players arriving from the left reach the shore foot first.

### Integrator notes from the review

- **Alcove:** the "drawn behind the pursuer with a slight darkening" treatment and Donut overlapping the niche's right jamb are only judged by eye in Pillow previews. Prove both in Bevy with the real A1 lighting and draw order.

### Guild exit

When a player enters the `exit_guild` rect, the portal swaps closed → open (a 1-frame `ajar` in-between is a Sonnet task), then the level completes. `protected_exit_region` holds no breakable, hazard, blocker or spawn (tested), and its floor is ordinary reinforced deck.

## Sonnet execution tasks (phase 2, final; done, see the execution record below)

Design decisions above are frozen, including the chunk redesign of the block (v2). Rects, anchors, the fall region and the hit count must not change. Send design problems back to Opus.

1. **Full-height barricade variant (required).** Add `timber_barricade_full_intact` and `timber_barricade_full_smashed` at 96×256. Use the same bottom anchor (48, 256) on the floor back line and place them at (2304, 32). Planks or a header must close up to the cornice, so the full-height `barricade` blocker (y 32 to the walking line) is visible. The layouts swap to this variant; the 96×176 pair stays as a lower option. Keep the same 2-state group and one rect per group.
2. **Soffit tiles for the chase-passage ceiling steps (required).** Add a small `collision` tileset for the two step columns of the lowered ceiling (columns 61 and 104 in both options), replacing the borrowed `wall_end_*` and `ceiling_corner_*` tiles:
   - `soffit_end_l` / `soffit_end_r`: the hanging face of the lowered soffit, in two quoin phases (`_b`);
   - `soffit_corner_l` / `soffit_corner_r`: the high cornice meeting the face, and the low cornice wrapping round its outer corner.

   The tiles must be opaque and collidable with the `[0, 0, 16, 16]` collider, and keep the validator's panel and edge rules. Use dark soffit masonry, so the step reads as a heavy lintel and not a corridor end wall. Both ceiling options must validate.
3. **Polish without design change:**
   - `loose_block_landed`: two chunk halves that follow the v2 split (the right stone and the left pair), plus crumbs;
   - barricade: plank width and tone variety, with dark gaps between the planks;
   - `shore_post_damaged_1`: a slightly larger notch so it reads at 1x.
4. **Missing frames,** as pieces or animated pieces (`animation` with a manifest tag), on the same palette:
   - `shore_dust_trickle` (3 frames, 8×24, under the block from damaged_1 on);
   - `block_impact_dust` (4 frames, 112×40);
   - `timber_splinter_burst` (4 frames, 48×48);
   - `guild_portal_ajar` (1 frame, same rect as the portal).
5. **Optional:** `wall_breach` (192×176, backwall_decal), only if CHASE-01 rejects the arch-window wedge (see note 8).
6. **Validators and tests:** keep `tests/test_art_env02_opening.py` green, and add:
   - R1: each state group shares one rect;
   - R2: no new palette colours;
   - R3: the fall region lies fully below the block and above the walking line;
   - R4: the strike band is reachable by the jab and the kick;
   - R5: the full-height barricade art covers the blocker rect;
   - R6: soffit tiles appear only at the step columns.
7. **Evidence:** regenerate `docs/evidence/env-02` (`make_env02_evidence.py`, `SHA256SUMS`), and add GIFs of the CP-03 sequence and the portal opening.
8. **DOZ-01 interface note (Sonnet does not draw the Dozer here).** The S2 entry and the S5 wedge go through 192×218 arch windows, but the pursuer is 259 px long. The turn-in and the wedge therefore need an **end-on (rear three-quarter) or wedged pose at most 192 px wide and 155 px tall**, drawn in DOZ-01, with the wheel-spin loop visible from behind. ENV-02 only reserves the `pursuer_spawn` and `pursuer_wedge` rects. If DOZ-01 cannot supply that pose, ENV-02 falls back to `wall_breach` (task 5) at the S5 wedge.

## Design fix round (Claude Opus, after the independent review)

The independent Opus review failed the Sonnet set on two MAJOR design items. Both are re-keyed by Opus here; Sonnet's other execution work is untouched.

1. **`shore_post_*` (all four states):** the old foot read as a metal sleeve or wrapped grip ("reinforced"), because it was about 95 % iron and mortar greys drawn in regular horizontal bands. The new foot is described in the piece table. All four states are re-cut on the same foot. The notch, the d2 split, the stub and the fallen shaft are unchanged, and so are rects, anchors, `STRIKE_T` and the hit count. All four are frozen Opus keys again (`tests/fixtures/env02_opus_keys.json`: 19 frozen, `loose_block_landed` stays polished). The player reads "hit here, it's weak" from the silhouette, which is thinnest and most chewed exactly at the fist and kick height.
2. **`timber_barricade_full_*` crosses the walking lane:**
   - **The problem:** the 96×256 art stood on the floor back line (bottom y 288), so actors walked visibly in front of it on the deck while the blocker stopped them. It read as a boarded doorway in the back wall.
   - **The redesign:** the piece is now **96×280, anchor (48, 280) on the deck's front lip (y 312)**, placed at (2304, 32). From the cornice down to the lip it covers the whole deck depth: two wide lane boards over the deck between the posts, the posts run down to the lip, a sill board lies on the front lip, and two kicker braces run from the posts down to the deck front.
   - **Layer:** it stays in `prop` (draw order 6, behind actors). The `barricade` blocker stops actors at its left face (x 2312), so nothing walks in front of it. A `foreground` layer was rejected because it would also draw over the pursuer as it charges through.
   - **Tests:** R5 now checks the 280-row cover and that at least 80 % of the lane band (y 288 to 312) is opaque.

### Sonnet task 9 (fix round): barricade plank rhythm and the smashed pose

Do not change the rect, the anchor, the lane footing, the posts or the sill. Redraw only the boards between the posts, to remove the ladder look:

- 7 to 9 boards of mixed width (6 to 14 px), with gaps of 2 to 7 px that never repeat twice in a row. Two or three boards are off-angle (6 to 12 degrees) and nailed over their neighbours, and one or two short vertical slats patch a gap. No even rhythm and no full-width diagonal running from post to post.
- Boards may overhang a post by 2 to 6 px on either side, but nothing may extend left of x 2304, so the art stays inside the blocker side. Keep the header under the cornice.
- **Smashed (same rect):**
  - the left post is snapped about 30 px above the back line with a splintered top;
  - the right post leans into the next bay, top about 18 px further right;
  - 4 to 6 boards are thrown across the lane band (y 288 to 312), lying low and flat so actors walk over them;
  - one board still hangs by a nail;
  - two header stubs hang from the cornice.

  Everything stays in `prop` (behind actors). Nothing may rise above 40 px over the walking line inside the lane once the blocker is removed.
- Keep R5 green, then regenerate `env02_key_sheet_*`, `env02_lit_s4_widened_*` and `SHA256SUMS`.

**Pieces Sonnet must redraw to match:** `timber_barricade_full_intact` and `timber_barricade_full_smashed` (task 9 only). The 96×176 `timber_barricade_*` pair still stands on the back line, so it must **not** be used as a lane blocker; leave it as a background-only option. Nothing else needs redrawing. The shore keys are final, and `shore_dust_trickle` keeps its position under the block.

## Execution record (Sonnet, phase 2)

All eight tasks above are executed; rects, anchors, the fall region and the hit count are unchanged.

- **Frozen keys:** `tests/fixtures/env02_opus_keys.json` freezes the pixel hash of 18 Opus keys, and `FrozenKeyTests` fails if one changes. The two keys that task 3 explicitly asks to polish (`shore_post_damaged_1`, `loose_block_landed`) keep rect, anchor and layer; their Opus-key hash is kept in the fixture and the polished hash is frozen too. `guild_sign` and every other key are byte-identical. New pieces are appended to each atlas, so no existing rect moved.
- **Task 1:** `timber_barricade_full_intact` / `_smashed` (96x256, anchor (48, 256), placed at (2304, 32)); **superseded** by the design fix round: 96×280, anchor (48, 280) on the deck front lip. Boards vary in width (7 to 12 px) and tone, dark gaps are filled between them, and a header and a sill close the top and the bottom: every row of the blocker rect is covered (R5, at least 85 % opaque). Both layouts use it; the 96x176 pair is unchanged.
- **Task 2:** new tileset atlas `floor01_soffit` with 8 collision tiles (**10 after the fix round**, see below) (opaque, `[0, 0, 16, 16]` collider). The spec names `soffit_end_l/_r` (+`_b`) and `soffit_corner_l/_r` but describes two corner functions, so the contract has **8 tiles**: `soffit_end_l`, `_l_b`, `_r`, `_r_b` (hanging face, columns 61 and 104), `soffit_corner_l/_r` (low cornice wrapping the outer corner, row low-1 of the step column) and `soffit_joint_l/_r` (high cornice meeting the face, row 1 one column outside the step: columns 60 and 105). **Side convention: `l` is the left step (column 61), `r` the right step (column 104).** R6 therefore reads: face and corner tiles only at the step columns, joint tiles only at the two cornice columns beside them. Face masonry is the dark fill (`brick_00`, `mortar_00`) with narrow lit quoins (3 and 5 px).
- **Task 3:** `loose_block_landed` is redrawn as the right stone (with the big crack), the left pair (two stones, bed joint) and crumbs; the barricade polish is in the full-height variant; `shore_post_damaged_1` has a deeper, wider notch.
- **Task 4:** `shore_dust_trickle` (3 frames, 8x24), `block_impact_dust` (4, 112x40), `timber_splinter_burst` (4, 48x48), each in its own `prop_animation` atlas (the exporter needs one canvas size per sprite) with a manifest tag, and `guild_portal_ajar` (112x136, same rect; a warm slit between the leaves). Ajar lights are interpolated between closed and open (`interior` r 170, i 0.32; `spill` r 130, i 0.3): a Sonnet choice for the reviewer to confirm.
- **Task 5** (`wall_breach`) is not drawn: the optional condition (CHASE-01 rejects the wedge) has not occurred. **Task 8** is a note for DOZ-01 and needs no art here.
- **Tests (task 6):** R1 to R6, 23 markers, hit point outside the fall region, exit reachable in any break order, alcove clear height 64 to 116 px, chase ceiling at least 164 px.
- **Evidence (task 7):** [`docs/evidence/env-02`](../evidence/env-02/README.md), including GIFs of the CP-03 sequence and the portal opening.

### Fix round execution record (Sonnet, after the Opus design fix round)

Done on top of `681cc57`; the shore keys are untouched (17 keys stay frozen; `shore_post_*` hashes verified in `FrozenKeyTests`). Three Opus keys that the review's execution fixes ask to change are now listed as polished in the fixture (`loose_block_landed`, `hazard_grit`, `guild_portal_open`); their Opus-key hashes are kept.

- **Task 9 (barricade, `timber_barricade_full_*`, 96x280, rect, anchor, posts, sill, lane boards, kickers and header unchanged):**
  - Intact: the boards between the posts are redrawn with an uneven rhythm (8 to 14 px thick, 2 to 7 px gaps that never repeat twice running), tone variety, 2 to 6 px overhang past the posts (nothing left of the piece), two wide gaps patched by short vertical slats and three off-angle boards (9, -7 and 11 degrees) nailed over their neighbours. No board runs post to post as a diagonal. The gaps are narrow `mortar_00` seams, not black voids (near-black rows are at most 15 % of the height, tested), and R5 stays green.
  - Smashed: the left post is snapped 30 px above the back line with a pale splintered top, the right post leans over (foot moved to x 73, top 18 px to the right), five low boards lie across the lane band, one board hangs by a nail from the leaning post and two header stubs hang from the cornice. Nothing in the middle of the lane rises above 40 px over the walking line (tested). **Deviations:** (a) it takes 18 front boards, not 7 to 9, to close the 240 px between header and lane boards with gaps of 7 px or less; (b) the spec's "top about 18 px further right" is applied inside the 96 px rect by moving the foot left, because the rect is frozen; (c) the post and hanging-board stubs above the snapped post are not drawn.
- **Review execution fixes:**
  - `loose_block_landed`: halves of about 44x28 and 40x26 plus loose bricks and crumbs, 2016 px opaque (the seated block is 2363).
  - `hazard_grit`: 18.8 % of the canvas (22.5 % of the fall-region width), 2x2 pale chips (`stone_08/09`), a scuff line, densest under the block, hard ends at piece x 8 and 88 (the fall region, 3000 to 3080).
  - `block_impact_dust`: frames 2 and 3 are 2x2 and 2x1 puff clusters that thin through darker tones (no isolated pixels in frame 3), and frame 3 is mirrored when its mass sits right of the anchor. `timber_splinter_burst` frame 3 is 2 px slivers.
  - `guild_portal_open`: the `gal_warm` share fell from 42 % to under 12 %: warm stone walls and bench, floor `floor_07` to `floor_09`, glow only on the lamp and the threshold strip; the interior light glow radius is 28 (was 52). `guild_portal_ajar` follows, as it composes the open interior.
  - Soffit: the lit quoins alternate 6 px and 4 px with a 1 px dark return; the blue ceiling course runs into a new cut-end face tile (`soffit_end_l_c` / `soffit_end_r_c`, used in the row above the low cornice; 10 tiles now, a 5x2 atlas); the face joints use `mortar_03` (about 8 dL* from the `brick_00` face, was about 1). The base kit's own fill panels (ENV-01) are not changed, so their joints keep the old contrast. Every soffit tile carries a `facing` note and is drawn per side: do not mirror.
  - CP-03 GIF: the block holds for 6 ticks (100 ms) and falls on the computed timeline (about 0.52 s under 1050 px/s^2). The loops follow 60 Hz ticks: trickle 117 ms, impact dust 83 ms, burst 67 ms.
- **Integrator note (not drawn, prove in Bevy):** the alcove hiding darkening (actors in the pocket draw behind the pursuer with a slight darkening) and Donut overlapping the right jamb of the niche need the real lighting and draw order.

### Review fix round (Sonnet, after the independent Opus review of 2026-11-18)

The independent review (a separate Claude Opus, verdict PASS WITH EXECUTION ITEMS) listed five execution items; all are done, and no frozen key changed.

- **E2, barricade posts:** the posts of `timber_barricade_full_intact` show again above the lane boards. Post timber fills the board gaps at the two post columns, and every second board stops 3 px inside its post, so a 3 px edge of upright shows on both sides. Rect, anchor, board rhythm, lane boards, kickers and header are unchanged; R5 and the near-black-row limit stay green.
- **E3, state strips with no actor:** `env02_cp03_states_no_actor_lit_1x.png` and `..._3x.png` show the four shore states with the five block states (seated, settling, slipped, falling, landed) in the lit frame, without Carl or any actor in front.
- **E1, key sheet:** `soffit_end_l_c` and `soffit_end_r_c` are in `env02_key_sheet_1x/3x`, so all 10 soffit tiles are shown.
- **E4, `loose_block_landed`:** the two halves are redrawn as angular broken stone (straight sheared faces, hard corners, flat torch-lit top planes), with the right half's crack kept and the opaque area above 2000 px. The polished hash in `tests/fixtures/env02_opus_keys.json` is updated with a note; the Opus-key hash is kept.
- **E5, deprecated 96×176 barricade:** `timber_barricade_intact` / `_smashed` carry `deprecated: true` and `background_only: true` in the contract (a `deprecated_note` states why), the `timber_barricade` state group is marked deprecated, and the owner key sheet no longer shows the pair. The pieces are not deleted and stay frozen keys.
- **Ratified (review note D1):** the 18-board count in task 9 above (instead of 7 to 9 boards) is **ratified by the independent Opus review (2026-11-18)**. Seven to nine boards of 6 to 14 px with gaps of at most 7 px add up to at most 182 px and cannot close the 240 px span; 18 boards read well.
- **Open for the owner (not acted on, non-blocking design notes):** D2, the frozen shore keys (a long grain check on `shore_post_intact` pre-announces the damaged_2 split; the fallen upper shaft in `shore_post_broken` is shorter than the shaft above the break and the head plate is gone); D3, the seated block's softened corners (the spec asks for an angular broken outline; harden two corners only if the block is ever re-keyed).

## Acceptance targets (for the independent Opus review)

1. **Readability at 1x (640×360, lit):**
   - the shore reads as timber, the block as heavy stone, and the rotten foot as the weak point;
   - Carl, Donut and goblins stay distinct from the set (measured in `measurements.json`);
   - the alcove reads as a place to hide;
   - the portal reads as the safe, warm goal.
2. **State legibility:** each of the 4 shore states and the matching block state (seated, settling, slipped, falling) is distinguishable at 1x in `env02_cp03_state_sequence_lit_2x.png` and, with no actor in front, in `env02_cp03_states_no_actor_lit_1x.png`.
3. **Grammar and locks:** a dry, clean brick catacomb; copper only in S3; no factory, sewer or workshop look; no text or logos; antihero tone (scrappy and improvised, not heroic or ornate).
4. **Data:** both layouts validate; the ids are IP-neutral; state groups share rects; the strike is outside the hazard; the protected exit is clear (all tested).
5. **Palette:** no new colours; each atlas has at most 63.

## Owner decision (2026-11-18)

The owner accepted ENV-02 as drawn after the review fix round:

- The full-height barricade posts stay as they are (faint at 1x is accepted).
- The frozen shore keys are accepted unchanged (review note D2: the grain pre-announces the d2 split; the fallen upper shaft is short and has no head plate).
- The softened corners of the seated block are accepted (review note D3).
- The chase ceiling uses option **R** (212 px clearance); option T stays as an unused alternative layout.

## Known weak spots

- **Barricade:** the full-height variant crosses the walking lane (96×280, design fix round), has the task 9 plank rhythm, and shows its two posts through the board gaps (review fix round). The 96x176 pair is **deprecated** (background only, see below).
- **Block (CP-03 load), v2:** redesigned as a 3D chunk with a socket drawn as depth. Its top face is deliberately dim (it faces away from the torch), so the volume relies on the lit right face; the independent review should check it at 1x. `loose_block_landed` now follows the v2 split (Sonnet task 3); it is small at 1x, so the review should check that it reads as the block's remains.
- **Ceiling transitions in S3:** replaced by the dedicated soffit tiles (Sonnet task 2); the review should judge the steps in `env02_soffit_steps_lit_3x.png` and the S3 frames.
- **Goblin luminance** against the brick in S5 is low (mean ΔL* about −3 to +3). The silhouette edge contrast is fine (17 to 24). Left alone on purpose: it is judged in Bevy with the real A1 lighting.
- The **pursuer entry and wedge through arch windows** are a depth trick that only CHASE-01 can prove in Bevy. Sonnet task 8 records the end-on pose DOZ-01 needs; `wall_breach` is the fallback.
- No Bevy capture; all evidence is the Pillow A1 model.

## Rights and provenance

Original pixel art written by Claude Opus (keys) and Claude Sonnet (full set) as deterministic rules in `scripts/author_floor01_opening.py`, reusing the ENV-01 shading helpers. There is no image-model raster output, and nothing is cropped, traced or resampled from concept images. The guild sign and keystone carry an invented lantern pictogram: no letters, no real-world mark. Actor previews composite the approved Carl 52 and Donut 40 atlases and the GOB-01 h36 candidates under the owner-reported `ART-PUBLICATION-ALL-V1` scope in [rights-and-release.md](../04-production/rights-and-release.md). That scope is not independently verified, is not a public reuse licence and grants no right to distribute a game or an asset pack. The manifests stay `prototype_fixture_unapproved`. Design and keys by Claude Opus; full-set execution by Claude Sonnet; review by a separate Claude Opus agent (2026-11-18); review fixes by Claude Sonnet.
