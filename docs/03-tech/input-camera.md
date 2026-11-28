# Input, controller UX, camera and accessibility

## Owner-approved MVP action contract (2026-11-18)

The [MVP controller contract](../01-design/mvp-controller-contract.md) defines the **approved design** for DualSense and Steam Controller / Steam Deck. It supersedes the provisional action layout previously listed in [gameplay](../01-design/gameplay.md). This is **not** an implementation completion claim.

Normalize device-specific inputs to player-scoped logical actions such as `Move`, `Jump`, `PrimaryAttack`, `PhysicalAttack`, `EquippedSpecial`, `Interact`, `SwapCharacter`, `Pause` and optional `PrecisionAim` / `CycleSpecial`. Preserve `tick`, player ownership, deterministic edge/held semantics and headless replay. Keep character abilities and narrative unlocks separate from the physical device binding; never hardcode PS5 button identifiers in the gameplay domain.

**Approved default mappings:** Cross/A jump; Square/X primary; Triangle/Y physical; Circle/B special; R1/RB interact (implemented for the `act1_guild` strongbox and guide only, keyboard E: [interact-verb.md](interact-verb.md)); L1/LB solo switch. Carl's Triangle/Y is **a kick, not a grab**. Donut's pre-biscuit primary is weak feline contact, changing to Magic Missile only after the story unlock. Optional L2/right-stick precision aim and R2 cycling are not MVP combat dependencies; D-pad locomotion must remain possible.

### Down (COMBAT-FUN-02, `--cf02` only)

CF02-A adds a **Down** direction to the logical input (`RawInput::down`, a held direction with no press edge, sampled on the press tick). It is not a fifth button: **Down + Physical** (Triangle/Y) is Carl's stomp on the ground and, in the air, his dive (CF02-H, Q52); Physical alone stays the kick. Donut ignores it (her Physical is the pounce). It reads only under `cf02_rules` (`--cf02`), so with the switch off the input changes nothing.

| Device | Down |
| --- | --- |
| Keyboard (P1) | `S` or the Down arrow |
| Pad (any slot) | D-pad down, or the left stick down to at least 0.5 with the vertical axis dominant (`abs(y) >= abs(x)`, so a diagonal run is not a stomp) |

The adapter resolves the device to the single `down` bool; a buffered press keeps the Down sample of its press tick (CF02-Q01, Q46).

### Existing fixture migration (pending implementation)

The current COMBAT-INT-01 prototype intentionally reuses `special_pressed` for Carl's push kick (keyboard K / East gamepad action; **superseded in `floor01_scene` by PR A: the kick is `physical`, K / North = Triangle/Y, and the WORK-021 greybox in `main.rs` keeps K / East as its bomb `ability`**) while `attack_pressed` (keyboard J / West) is his jab; WORK-021 separately uses that special action for bombs. Existing fixtures use press edges, 6-tick buffering and an initial facing correction. **Keep these facts as implementation history, not the approved final default bindings.**

The implementation PR must add an independent physical-attack action and migrate the integrated scene's kick without breaking fixed-step/replay regression tests. Only learned, equipped specials use the dedicated Circle/B action and cannot appear before story acquisition. Auto-repeat for held primary, aerial/down variations and exact timing remain unresolved, requiring real controller playtests.

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

DEMO-01 PR C puts the camera-lab automatic camera into `floor01_scene` as a presentation-only layer (`--camera auto`, to become the `--demo` default with DEMO-01 PR A): eased zoom and pan, rest widths only at exact integer scales of the output (never a fractional scale at rest), cap 960 px wide, the scroll camera and every hash untouched. See [Floor 01 scene > Camera](floor01-scene.md#camera-demo-01-pr-c).

Keep both active characters within a safe framing region. Use biased tracking toward the midpoint with a speed cap and optional modest zoom-out; large split-distance triggers regroup hints or a teleport safeguard only after warning. Camera shake must be toggleable and applied in presentation; it cannot affect aiming or collision tests. Strong FX should not occlude critical threats.

## Accessibility and usability acceptance

- Entire boot/start/pause/restart/exit loop usable from gamepad.
- At least 2-player local input works simultaneously with stable slot assignment.
- UI readable at Steam Deck's 1280×800 output viewed at handheld distance.
- Reduce screen shake, flashes, rumble and particles separately.
- Danger cues use shape/sound/animation in addition to color.
- Pauses do not discard controller states; no forced launcher/keyboards.
- Implement gameplay-friendly aim assist instead of assuming all players can use precise sticks.
