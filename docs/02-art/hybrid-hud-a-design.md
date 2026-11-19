# HUD-HYBRID-A-01 — THE CRAWL hybrid in-game HUD

**Status (2026-11-19):** owner-selected **visual direction A**, subsequently approved as a four-state concept family with a dedicated Donut mana bar. **Implementation not yet verified.** The concept images illustrate style and layout; they are NOT pixel-accurate production assets, native-scale Bevy screenshots, canon evidence, combat balance, or acceptance of the textual labels they depict.

## Goal and authority
Implement one cohesive, controller-first, combat-first, in-world-free **screen-space HUD** for Floor 01 in the existing Rust + Bevy scene, reusing authoritative player/combat data wherever actually available. Do not ship four separate permanent UI systems. Preserve approved action grammar in `docs/01-design/mvp-controller-contract.md`, graphics/size in `docs/02-art/art-direction.md`, camera in `docs/03-tech/input-camera.md`, and story unlocks in `docs/01-design/floor-01-gameplay-progression-v0.md`. Read `AGENTS.md` first.

### Approved aesthetic
- Rugged industrial hardware, dark metal, worn bevels, rivets, amber legible typography, magenta signal accents; atmospheric, irreverent 1980s/1990s action-show attitude rather than slick futuristic UI.
- **Persistent play HUD:** Carl **top-left**, Donut **top-right**; each with recognizable portrait, name / P1 or P2 ownership, red HP, accessible resource/action states. Floor or area identification near top center. Keep the scene action and critical telegraphs clear.
- **System personality:** happy/sinister tiny CRT-like face as an optional original UI motif, with deliberately theatrical magenta/gold notices. Small idle state; brief, high-impact event notice, not a permanent intrusive overlay.
- **Boss presentation:** temporary encounter-specific bar and attack warnings. **Local co-op:** differentiated P1/P2 conditions, usable with one shared camera. **Solo:** clarify which hero is controlled versus AI-assisted without lying about player 2.
- UI hierarchy: gameplay > threat tells > health/resource > contextual action > incidental System flavor. Excess particles/animation must not block hitboxes, route cues or telegraphs.

### Resource policy
- **Carl has HP only** for MVP. Do not create fake mana, stamina, rage or arbitrary charges.
- **Donut has HP + a thinner violet-to-cyan segmented mana bar** directly beneath HP and above the abilities row; icon AND color signal its meaning. Optional numbers are config/presentation, not approved balance. Display a genuine authoritative resource when available.
- **Before the story-backed magic unlock (biscuit): hide Donut's mana bar entirely.** Do not grant pre-biscuit spell, regeneration, or new input semantics. All four supplied mockups depict a *post-unlock UI preview*, even if their background resembles the early corridor.
- When mana is depleted, indicate via fill, muted affordance, and non-color cue. The exact mana cost/regeneration/magnitude is **not designed in this task**. Until game logic implements those values, use a clearly named non-story UI preview fixture; never quietly fabricate a resource in normal gameplay.

### Control truth: ignore incorrect concept-text
Concept art text (LT Punch, RT Dodge, LB Whisker Lash, RB Cat Step/Dash) is **wrong** for the approved MVP. Never copy it as UI behavior or documentation.
| Logical action | DualSense | Xbox / Steam |
| --- | --- | --- |
| Jump | Cross | A |
| Primary | Square (Carl punch; Donut pre-biscuit swat, post-biscuit Magic Missile) | X |
| Physical | Triangle (Carl kick; Donut feline action) | Y |
| Special | Circle (story-earned only) | B |
| Interact | R1 | RB |
| Solo swap | L1 | LB |
| Pause | Options | Menu/Start |
Use mapped controller glyphs only for actual supported actions and current input device. No mandatory dodge, grab, twin-stick aim or speculative spells. Never show a locked capability as a usable ability.

### Four presentation states
1. **Exploration/quiet:** HUD compact, no large System toast. Show the correct unlock phase; pre-biscuit has NO mana bar.
2. **Combat + System event:** HP and permitted abilities remain stable; a short bottom-right achievement/combo presentation may play on an **actual event**. In a UI demo/fixture it may use fixture data; never invent scoring rules or buffs in story mode.
3. **Boss/arena:** use a temporary boss name+HP strip with clear fill and phase-independent warning icons, avoiding the floor/action lane. The generated “Warlord Grizzog” is an **invented placeholder only**, NOT a canonical Floor 01 enemy or approved encounter; replace with an existing actual boss only when that feature exists.
4. **Co-op / per-player statuses:** Carl and Donut's health/mana/condition distinguished by portrait, text/icon, fill and animation, not color alone. Donut mana may be ~1/3 in preview. Red/green mockup labels like “PRESSURED +25%” or “TEAM COMBO +10%” are NON-APPROVED fictional tuning; don't add mechanics or tooltips claiming those buffs.

## UI implementation and technical contract
- One reusable Bevy UI plugin/component family with data adapters; use existing render/font/art infrastructure where practical. Model UI as snapshots or events from domain state; never couple gameplay damage/cooldowns/unlocks to rendered bars or the mascot.
- Work in the **integrated `floor01_scene`** and actual shared presentation code. Do not replace the scene with the large concept screenshots, hardcode health in the standard corridor, or build an unrelated mockup application. Keep headless replays and deterministic combat fixtures untouched.
- Concept images are **composition references**, not approved pixel masters. Produce editable original HUD source assets (`.ase` / `.aseprite` or documented native editable pixel-grid source), reproducible export + provenance + checksum, or native Bevy UI primitives where appropriate. Reuse approved character portraits/sprite masters where legally and technically possible; avoid copying crop/resizing AI concept portraits as canonical sprites.
- Use the virtual canvas 640×360 and 640×400 with integer output scaling (e.g. 1920×1080, 1280×800). HUD attaches to viewport, independent of camera position / world zoom; no fractional pixel shimmering. Preserve critical visibility and text readability on Deck. Safe margins, ability row and boss bar should not overlap at the small virtual resolutions.
- Differentiate **real states** vs **preview-only states** cleanly. Preview fixtures must be explicit and non-authoritative, make screenshots reproducible, and require no new gameplay physics or fabricated companion control.
- Do not require a new dependency unless needed; use the existing Rust/Bevy engine and repo toolchain.
- Expose configurable visibility and UX accessibility controls where the infrastructure permits (screen shake/reduced UI animation, contrast, text size); document any unavailable option rather than pretending it exists.

## Acceptance gates for single-agent integration
- Actual in-scene HUD renders, anchored to corners, with valid runtime values when present; missing gameplay fields do NOT render fabricated numbers.
- Donut has an independent, thinner magical resource bar in **post-unlock preview**; none for Carl; no Donut bar in **pre-unlock**.
- A device/ownership-aware ability row matches the authoritative button grammar, not the generated concepts.
- The 4 requested situations are demonstrated through normal flow where supported plus deterministic **explicit preview** screenshots for unsupported states. Label preview vs real evidence.
- No regressions in movement/combat, camera, replay snapshots, or existing Floor 01 normal story route.
- CI / local checks: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `python3 scripts/check_docs.py`; art validation if sources were added. Run Bevy captures for 640×360 and 640×400, show evidence at 1×, 2×, 3×.
- Human/controller review: legibility under busy combat, no obscured boss telegraphs, co-op identity understandable, depleted mana identifiable without relying on hue, brief notices never steal focus. Document performance measurements rather than assert 60 fps.
- Update `docs/04-production/backlog.md` and scene docs with actual status; submit implementation to this PR. No automatic merge from this handoff; independent review and owner verdict on the in-engine look.

## References and upload workflow
The ZIP `the-crawl-hud-hybrid-a-implementation-kit.zip` contains four **concept-only** PNGs, a SHA-256 manifest, an installer that stages them as **reference assets**, and an English single-agent prompt. The PR initially commits **this spec and the agent workorder only**; images are to be installed from the ZIP and added to the SAME branch by the local agent. See `docs/07-agents/hybrid-hud-a-single-agent-workorder.md` and `assets/references/hud/hybrid-a-v1/README.md`.

**Provenance:** generated in the project conversation with OpenAI image generation, 2026-11-19; human approved visual direction/layout. `ART-PUBLICATION-ALL-V1` is owner-reported permission to publish fan-concept images, not independently verified, not a commercial game distribution license. Keep generated mockup labels non-canonical and preserve hashes.
