# Input, controller UX, camera and accessibility

## Device-neutral action layer

Normalize keyboard/gamepad/Steam Input actions into `PlayerAction { tick, player_slot, movement_axis, aim_axis, jump_pressed, attack_pressed, special_pressed, interact_pressed, swap_pressed }`. Input is a frontend adapter only. Recorded actions must replay headlessly without physical gamepads. Never couple character abilities to hardcoded gamepad device IDs.

## Melee and the secondary action before the Guild (COMBAT-INT-01)

Binding decision (orchestrator, 2026-11-17), adjustable after playtest: Carl has no bombs or explosives before the Guild, so the secondary/special action that WORK-021 used for bombs (`special_pressed`; K / East in the greybox) drives the **push kick** in the Floor 1 scene fixtures. `attack_pressed` (J / West) is the jab. Both act on press edges only: a held button never repeats a move, and a press made while a move runs is buffered for 6 ticks ([move table and buffering](floor01-scene.md#combat-fixtures-combat-int-01)). Pressing a direction with the attack turns Carl first. Bombs stay WORK-021-only and unreachable in the scene. The kick can move to hold-attack or a jab follow-up if playtest says so; the table in `src/combat.rs` does not change.

## Controller plan

See [gameplay](../01-design/gameplay.md) for action mapping. Use 2-player local co-op with explicit controller assignment and UI state. A player may use a gamepad while the second uses a different gamepad or keyboard (keyboard co-op is optional). Support hot unplug/pause rather than sending stuck movement input after disconnect.

- Prefer limited button demands; repeated action does not require combos of three buttons.
- Jump buffer and coyote time are adjustable gameplay tuning data.
- Donut magic aiming: choose assisted directional targeting over precision twin-stick aiming if playtesting finds friction.
- New player can learn all essential inputs via 1-screen prompts while playing.
- Keep input remapping as a pre-release gate; hardcoded layout is acceptable only in early spike.

## Pixel-perfect camera without jitter

Store simulation world transforms independently from screen pixel snapping. At render presentation, compute an integer scale for supported viewports, snap world-to-screen position (not simulation velocity), apply camera interpolation carefully, and validate against 1-pixel shimmer. For 640×360 → 1920×1080 scale 3; for 640×400 → 1280×800 scale 2. For non-integer display sizes use explicit letterboxing or carefully evaluated scale policy; never silently stretch sprites.

## Same-screen camera

Keep both active characters within a safe framing region. Use biased tracking toward the midpoint with a speed cap and optional modest zoom-out; large split-distance triggers regroup hints or a teleport safeguard only after warning. Camera shake must be toggleable and applied in presentation; it cannot affect aiming or collision tests. Strong FX should not occlude critical threats.

## Accessibility and usability acceptance

- Entire boot/start/pause/restart/exit loop usable from gamepad.
- At least 2-player local input works simultaneously with stable slot assignment.
- UI readable at Steam Deck's 1280×800 output viewed at handheld distance.
- Reduce screen shake, flashes, rumble and particles separately.
- Danger cues use shape/sound/animation in addition to color.
- Pauses do not discard controller states; no forced launcher/keyboards.
- Implement gameplay-friendly aim assist instead of assuming all players can use precise sticks.
