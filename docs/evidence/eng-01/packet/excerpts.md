# Doc excerpts

## docs/02-art/gob-01-goblin-spec.md#1. Canon and tone constraints

## 1. Canon and tone constraints

From the [opening encounter handoff](../../../07-agents/floor-01-opening-encounter-handoff.md) and the [Floor 1 opening direction](../../../02-art/floor-01-opening-catacomb-goblins.md):

- Common goblins are green, about 4 ft tall, in leathery rag clothing, with sharp teeth and pointy ears.
- The early on-foot weapon is a **pineapple on a wooden stick**. It is not a studded or metal club.
- The two common goblins are a **visual** variation (A and B), not two combat classes. Same frames, same timing, same boxes.
- Tone ([antihero tone](../../../01-design/antihero-tone.md)): a scrappy, nasty, slightly ridiculous threat with a readable tell, never a heroic orc. Carl must stay believable when he knocks one away.


## docs/02-art/doz-01-dozer-spec.md#1. Canon checklist

## 1. Canon checklist

From the [opening encounter handoff](../../../07-agents/floor-01-opening-encounter-handoff.md) (segments C, D and E), the [Floor 1 opening direction](../../../02-art/floor-01-opening-catacomb-goblins.md) and the [combat probes](../../../07-agents/floor-01-integrated-combat-probes.md):

| Canon or brief item | Where it is in the design |
| --- | --- |
| Rusted, mismatched steam contraption | A cool boiler-iron hull, with dark rust cab plates, a copper patch, a rust plate bolted over a hole in the tub, soot bands and rust streaks |
| Tank treads | One near-side tread unit: a link belt with cleats, 4 road wheels, 2 end wheels and a riveted track frame |
| Huge spiked spinning front wheel | A 96 px riveted drum with 12 spikes on a push-arm fork. It spins on its own cycle (section 5) and is the brightest part of the machine (section 7.1) |
| Several smoking pipes | The main stack with a bucket spark cap, a rear copper exhaust with a tin-can cap, and a kettle soldered on as the steam whistle (3 emitters) |
| Three riders: two level-2 goblins and a level-3 engineer in a kitchen-pot helmet | The engineer sits at the rear controls in an upside-down saucepan, its handle sticking out back. Rider A (GOB-01 variant A) and rider B (variant B, red bandana) sit in the front tub |
| The engineer visibly operates the Dozer | One hand on the throttle lever (the knob is on the hull, under his hand), the other arm pointing at Carl. He has a lever-pull key for the charge |
| Grimy, menacing and a little ridiculous | A painted shark-tooth grin on the tub, the kettle whistle, the bucket on the stack, the saucepan helmet (it hops off his head on a lurch and slides over his eyes on a brake), and a pressure-gauge needle past the red |
| Escape threat, not a boss | No health bar or damage frames; knockback immunity; tells for the charge and the overshoot (section 9) |
| Riders are separate, reusable concepts | Riders are their own 64 x 56 sprites with GOB-01 pixels and palette, seated by anchors. The on-foot goblins stay GOB-01 |
| Wheel lethal zone distinct from the hull | Bright spike ring (mean L* 0.58) over a dark drum (L* 0.26), in front of a cool iron hull |
| Smoke stays local | Small, short-lived, non-blocking puffs from 3 emitters (section 5) |
| 1980s to 1990s action tone, original design | A scrap-built road roller with a grin and a soot-faced crew: B-movie menace, no copied vehicle |


## docs/02-art/doz-01-dozer-spec.md#7.1 Keys (Opus, as authored)

### 7.1 Keys (Opus, as authored)

**Hull `body`**, from the back:

- a copper rear exhaust running up the back, with an elbow and a dented tin-can cap;
- a cab box of dark rust plates (two riveted seams, rust streaks) with a brass firebox door and three glowing slots;
- a deck plate, and the throttle lever with a brass knob at about (-36, 106);
- a vertical iron boiler with three riveted hoops, a copper patch, a brass gauge (needle past the red), soot bands and one dent; its dome top is at 120;
- a brass kettle on the dome as the whistle;
- a crooked rust stack with a brass collar and an upturned iron bucket as the spark cap (top 156);
- the dark tub back wall (hidden by `front`) and a dark chassis strip behind the tread.

**Hull `front`:**

- The rider tub has a slanted front, a rolled iron lip (top y 104), a riveted seam, a bolted rust plate and streaks.
- Its oxblood lower plate carries the painted grin: staggered bone teeth around a dark mouth.
- The cab rail plate has a tube rail (top y 105).

**Wheel, phase 0 (phase 2 redesign).** The lethal part is the brightest thing on the machine.

- **Spikes:** 12, in iron `4` and `5`, with a `6` face on the side toward the upper-left light. The outer 2 px of every spike is `7`, and every spike keeps its `K` outline.
- **Rim band:** 3 px, `5` on the outer 2 px and `3` on the inner 1 px. It carries the 12 bolt glints (`7`) between the spikes.
- **Drum:** dark `Q` and `W`, with `E` only as an upper-left crescent.
- **Spokes:** 12, each 2 px of `2` with a 1 px `4` edge on the lit side. They run from the brass hub to a 2 px `3` inner ring at r 30.
- **Symmetry and light:** every feature is 12-fold, so phases 0, 7.5, 15 and 22.5 degrees loop exactly. The shading is in screen space, so the light does not turn with the wheel.
- **Measured:** the spike ring's mean L* (outline excluded) is 0.58 in all four phases, and the drum's is 0.26.

**Tread, phase 0:**

- The belt's bottom run lies on the contact row (row 33).
- Each road-wheel hub has 5 dark bolts. The one nearest the upper-left carries the `7` glint, so the glint keeps the light fixed while the bolts turn.

**`engineer_point` (phase 2 pot redesign, deepened in fix round D7).** The body is the GOB-01 idle body seated: rows shifted down 9, no pineapple. Over it go the saucepan, the pointing far arm and the near hand on the lever knob. The saucepan:

- jammed down to the brow;
- a **flat top** with a 1 px `6` light row (rider-canvas row 26; outline row 25);
- a 2 px `K` notch on the top-front corner;
- **a body 6 rows deep and 14 px wide** (rows 26 to 31, x 27 to 40), shaded as a cylinder (D7: the 4-row pot read as a brimmed hat at 1080p);
- **a 1 px lip** (row 32), only 1 px wider than the body on each side, just above the brow;
- a **2 px handle about 11 px long**, unchanged. It leaves the lip, rises 2 px toward the back and has a 1 px `K` hanging hole 2 px from its end.

**`engineer_lever_pull`.** The torso and head lean back 2 px. The head is the GOB-01 yelling head (frame 11 moved by (-2, -2)). Both forearms reach straight to the knob: the near arm lit, the far arm in shade above it. The pot tilts back, with its top 2 rows 1 px back.

**`engineer_lurch_back`.** He is thrown back 3 px, yelling, with white eyes. The pot **hops 3 px off his head** and stays tilted back. His hands stay on the knob.

**`engineer_lurch_forward`.** He is pitched 2 px forward and 1 px down. The pot **slides 3 px down over his eyes**.

**`rider_a_brandish`.** The GOB-01 attack-tell key (frame 11) seated, with its pixels unchanged.

**`rider_a_lurch_back`.** The GOB-01 hurt key (frame 16) seated: head thrown back, white eyes, an O mouth, the pineapple arm flung up and back.

**`rider_a_lurch_forward`.** The seated hold pose (GOB-01 frame 0) pitched 2 px forward and 1 px down, with white eyes.

**Rider B (`rider_b_hold`, `rider_b_lurch_back`, `rider_b_lurch_forward`).** The same construction from GOB-01 variant B (the bandana overlay). `rider_b_hold` is derived, not a new key.

**Smoke, frames 0 and 2.** Lumpy blobs with no outline.

- **Black (fix round D2):** frame 0 is the **8 px** spawn and frame 2 the **20 px** puff, both with broken edges (bites out of the silhouette). A light `:` cap covers the top third of the puff's height, the body is `-`, and `+` is used only on the underside row. The phase-2 puffs (mostly `=`, 6 and 16 px) read as dark debris over unlit brick.
- **Steam** frames 0 and 2 are unchanged: smaller and paler, with a body in the middle tone, a 1 px darker underside and an upper-left lit band.

(truncated: 2 more lines in docs/02-art/doz-01-dozer-spec.md)
