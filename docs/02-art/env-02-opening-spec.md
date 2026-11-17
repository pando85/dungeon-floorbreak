# ENV-02: Floor 1 opening route pieces (design spec, phase 1)

**Status:** design phase 1 of the [AGENTS.md art pipeline](../../AGENTS.md#model-routing-for-art-work). Design and keys by Claude Opus (2026-11-17). **Execution pending (Claude Sonnet); independent review pending (a separate Claude Opus agent); owner review at the end only.** Nothing here is wired into `floor01_scene`; the integrator owns that.
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
- Palette `floor01-catacomb-v2`: **no new colours**. Atlas use: `floor01_opening_props` 33, `floor01_opening_decal` 14, `floor01_guild` 32 (max 63 each).
- Timber uses `wood_00..05` plus `floor_07/08` for torch-lit edges. Fresh splinters use `stone_06..09` (pale, warm). Dry rot uses `mortar_01..03` and `iron_03..05` (grey-brown, crumbly, never black). Dressed stone uses `stone_*`; the guild glow uses `flame_*` and `gal_warm_*` (these bloom, by the existing `emissive_colours` rule).
- Layers and draw order are inherited: `backwall_decal` (3) for the alcove and the wall cavity, `prop` (6) for everything else, actors (7) in front of all new pieces. **All pieces are non-collidable** (contract rule). Any blocking is an authored gameplay rect (marker), never a piece.
- Light: the pieces carry baked light from their nearest torch (S5 torch to the right of the block; S4 torch to the right of the niche). Emitters exist only on the guild portal (presentation only).

### Contract: `extends`

`assets/source/terrain/floor01/floor01_opening_kit.json` is an `env-kit-v2` **extension contract**. It has `"extends": "<ENV-01 contract>"` and adds three atlases and 19 pieces, with no tiles. `scripts/env_kit.py` (`resolve_extends`) merges it onto the base. It inherits the palette, layers, coordinates, walking-line rule, roles and lighting schema, and must not redeclare them. Atlas names and ids must be new. The base file is never modified, and layouts that name the base keep validating against the base alone. **The Rust loader (`src/env_kit.rs`) does not support `extends` yet.** That is an integrator task when the route is wired in: merge the same way, or flatten at load.

## Pieces

All sizes are in px; anchor = piece-local px.

| Piece id | Atlas | Size | Layer | Anchor / placement | States | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `shore_post_intact`, `_damaged_1`, `_damaged_2`, `_broken` | opening_props | 136×144 | prop | foot (16, 138); placed at (2952, 160) | 4, one rect | 11 px timber from foot (2968, 298) to head (3020, 168), about 68°. Lower 30 % is dry rot. d1: notch at the strike point with pale fibres. d2: deep notch, a split up the grain, the shaft bows 1.6 px and the head slips 2 px. broken: a jagged stub, and the upper shaft lies on the deck |
| `loose_block_seated`, `_slipped` | opening_props | 64×56 | prop | bottom centre (32, 48); placed at (3008, 120) | 2, one rect | 60×34 dressed corbel stone, torn brick stubs on top, a lip course and a shadowed underside, one big split with a lit lip and two hairlines. slipped: content 2 px lower plus a grit trickle |
| `loose_block_falling` | opening_props | 72×64 | prop | (36, 52) | debris sprite | the same block tipped 9°, with loose brick bits. Gameplay moves it |
| `loose_block_landed` | opening_props | 96×32 | prop | bottom centre (48, 30) on the walking line | | two big chunks, bricks and a dust skirt. **Non-collidable** |
| `wall_cavity` | opening_decal | 80×64 | backwall_decal | block xy − (8, 8) | | the socket: a wide ragged gap above the block, a narrow one on the left, a lit lip on the right, cracks. It frames the seated block (the "loose" tell) and shows whole after the fall |
| `hazard_grit` | opening_props | 96×16 | prop | (2992, 288) | | pale grit and chips on the deck under the block: an in-world "things fall here" tell |
| `timber_set_intact`, `_smashed` | opening_props | 80×104 | prop | bottom centre on the back line; (1104 / 1424, 184) | 2 | mine-style set: two 8 px posts and a 10 px header about 120 px above the walking line, knee braces. smashed: stubs and two header pieces on the floor |
| `timber_barricade_intact`, `_smashed` | opening_props | 96×176 | prop | bottom centre on the back line; (2304, 112) | 2 | two 12 px posts, nine nailed planks and a diagonal brace. smashed: a left stub, a leaning right post, planks on the floor, one plank hanging by a nail |
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
2. **Telegraph:** on damaged_2 the block swaps to `_slipped` and a dust trickle starts (VFX). That is the "next hit drops it" tell.
3. **Fall:** 6 ticks after the break, the block (`loose_block_falling`) drops straight down from bottom y 168 to the walking line, 140 px. That takes about 0.52 s at the hero's 1050 px/s² gravity, and the fall can be tuned. **One impact event, on the landing tick:** every actor whose hurtbox overlaps `hazard_fall_region` takes it. Goblins are defeated, with the kill credited to the player who landed the breaking hit. Proposal: players inside take one hit and knockback (it is a real risk), but the designed strike position is outside the region.
4. **Hazard footprint:** [3000, 168, 80, 140]. That is the block's 64 px plus 8 px each side. Carl at the strike position (pivot about 2956, right edge about 2968) is 32 px clear (tested).
5. **After:** `loose_block_landed` sits on the walking line and is **non-collidable**. Navigation and collision are unchanged, so the exit stays reachable in any event order: shore first, goblins first, or the shore never touched. The direct melee kill is always the alternative.
6. **Luring:** the riders land at x 3136 and later and walk left toward the players, through the fall region. Players arriving from the left reach the shore foot first.

### Guild exit

When a player enters the `exit_guild` rect, the portal swaps closed → open (a 1-frame `ajar` in-between is a Sonnet task), then the level completes. `protected_exit_region` holds no breakable, hazard, blocker or spawn (tested), and its floor is ordinary reinforced deck.

## Sonnet execution tasks (phase 2)

Design decisions above are frozen; send design problems back to Opus.

1. **Polish the keys** without changing their rects or anchors:
   - block: more volume (a 2 px top face catching light, a softer bevel), and the cavity gap must not read as a picture frame at 1x;
   - landed rubble: chunk shapes that clearly come from the block (same split);
   - barricade: plank width and tone variety, and dark gaps between the planks;
   - shore damaged_1: a slightly larger notch so it reads at 1x;
   - barricade: a full-height variant (96×256) that closes up to the cornice, so the full-height blocker is visible.
2. **Add the missing frames** as new pieces or animated pieces (`animation` with a manifest tag), keeping the palette:
   - `shore_dust_trickle` (3 frames, 8×24);
   - `block_impact_dust` (4 frames, 112×40);
   - `timber_splinter_burst` (4 frames, 48×48);
   - `guild_portal_ajar` (1 frame, same rect as the portal).
3. **Soffit edge tiles:** add a small tileset for the lowered-ceiling steps (`soffit_end_l/r`, `soffit_corner_l/r`, in two quoin phases), replacing the borrowed `wall_end_*` and `ceiling_corner_*` at columns 61 and 104. They must keep the validator's panel and edge rules.
4. **Optional:** `wall_breach` (192×176, backwall_decal), only if CHASE-01 rejects the arch-window wedge.
5. **Validators and tests:** keep `tests/test_art_env02_opening.py` green, and add:
   - R1: each state group shares one rect;
   - R2: no new palette colours;
   - R3: the fall region is fully below the block and fully above the walking line;
   - R4: the strike band is reachable by the jab and the kick;
   - R5: no text-like glyphs. This is a review item.
6. **Regenerate the evidence:** run `python3 scripts/make_env02_evidence.py --out docs/evidence/env-02`, update `SHA256SUMS`, then add GIFs of the CP-03 sequence and the portal opening.

## Acceptance targets (for the independent Opus review)

1. **Readability at 1x (640×360, lit):**
   - the shore reads as timber, the block as heavy stone, and the rotten foot as the weak point;
   - Carl, Donut and goblins stay distinct from the set (measured in `measurements.json`);
   - the alcove reads as a place to hide;
   - the portal reads as the safe, warm goal.
2. **State legibility:** each of the 4 shore states is distinguishable at 1x in `env02_cp03_state_sequence_lit_2x.png`.
3. **Grammar and locks:** a dry, clean brick catacomb; copper only in S3; no factory, sewer or workshop look; no text or logos; antihero tone (scrappy and improvised, not heroic or ornate).
4. **Data:** both layouts validate; the ids are IP-neutral; state groups share rects; the strike is outside the hazard; the protected exit is clear (all tested).
5. **Palette:** no new colours; each atlas has at most 63.

## Known weak spots

- **Barricade height:** the art reaches y 112 but the blocker is full height (y 32), so an 80 px band above the planks blocks without showing it. Sonnet should extend the barricade with planks or a header to the cornice (a 96×256 variant), keeping the bottom anchor.

- **Block (CP-03 load):** it reads as a cracked slab in a socket. At 1x it is flatter than ENV-01's arch stones, and the even gap can read as a frame.
- **Ceiling transitions in S3:** the borrowed end-wall quoins at the two step columns read as wall ends, not as a soffit (Sonnet task 3).
- **Goblin luminance** against the brick in S5 is low (mean ΔL* about −3 to +3). The silhouette edge contrast is fine (17 to 24). The fix belongs to the scene lighting or GOB-01, not to this kit.
- The **pursuer entry and wedge through arch windows** are a depth trick that only CHASE-01 can prove in Bevy. `wall_breach` is the fallback.
- No Bevy capture; all evidence is the Pillow A1 model.

## Rights and provenance

Original pixel art written by Claude Opus as deterministic rules in `scripts/author_floor01_opening.py`, reusing the ENV-01 shading helpers. There is no image-model raster output, and nothing is cropped, traced or resampled from concept images. The guild sign and keystone carry an invented lantern pictogram: no letters, no real-world mark. Actor previews composite the approved Carl 52 and Donut 40 atlases and the GOB-01 h36 candidates under the owner-reported `ART-PUBLICATION-ALL-V1` scope in [rights-and-release.md](../04-production/rights-and-release.md). That scope is not independently verified, is not a public reuse licence and grants no right to distribute a game or an asset pack. The manifests stay `prototype_fixture_unapproved`. Design by Claude Opus; execution pending (Claude Sonnet); review pending (a separate Claude Opus agent).
