# FLOOR01-BOILER-01 — Local agent implementation work order

**Owner request:** turn [the Goblin Boiler encounter design](../01-design/floor-01-goblin-boiler-encounter.md) into an actual fun, deterministic Floor 01 mid-floor Bevy encounter. **This pull request initially contains documentation only.** The local coding agent should extend this PR/branch with implementation, tests and evidence, subject to normal repo review and CI. Do not mark this PR "done" merely because the specification exists.

## Priority / authority

Read `AGENTS.md` and the repository's current orchestrator/workorder rules first. The linked design spec is the **what/why** and initial numeric hypotheses; approved controller grammar, art baselines, actual merged code and existing feature ownership supersede implementation assumptions. **Inspect current main/open PRs before touching shared Rust or asset paths**. Coordination matters because character progression, companion behavior, terrain and replay are under active development. Keep all new source and PR text in English.

No need to create an independent "boiler engine" or change the game's movement, balance, rewards or companion navigation rules. Use existing `floor01_scene` and current level-format/scene registration conventions. If the actual scene format differs from the proposed coordinates or entities, adapt *the authored room*, retaining functional reachability and events. Do not misrepresent a lab-only fixture as the story level.

## Deliverables and order

| Step | Implementation deliverable | Evidence and definition of done | Gate |
| --- | --- | --- | --- |
| BOIL-A | Add a stable, versioned **Goblin Boiler room** level/scene ID, geometry and scene entry. Continuous ground route; two optional short elevation choices; proper checkpoint, protected exit, ~1152px scrolling test | Headless load/schema and path-reachability tests; visible Bevy capture with both heroes at native virtual resolution | Existing LDtk/material/room loader; must not invent a second incompatible parser |
| BOIL-B | Implement data-driven valve `intact→warning→venting→spent`, qualifying hit/contact, safe hazard area and one-shot per-actor contact | Deterministic 60Hz unit tests, single activation/token, restart test, visual/sonic tell captured at speed | Reuse hazard/impact machinery; only minimal domain primitive if truly missing |
| BOIL-C | Implement weak brace/weight `intact→weakened→released→falling→landed`, gravity/swept collision, low traversable remnant | Fixture proving actual landing overlap and damage/kill attribution; unaffected adjacent actors; both-hero exit reachable after landing | Existing CP-03 support and physics interfaces wherever viable |
| BOIL-D | Implement real launched-enemy→valve/brace contact and bounded P1→P2 causal sequence. Optional additional drum after direct and 2-link routes pass | At least three deterministic action sequences with exact event order; no script-injected kill, no contact spam; demo/legacy regressions unchanged | Actual relevant physics/terrain interfaces; do not force merge of another agent's work |
| BOIL-E | Integrate authored goblin composition, readable enemy staging, optional engineering enemy behind verified feature gate, approved Floor 01 workshop art, small FX/audio | Three at-speed in-engine playtraces/clips: direct, valve, full chain. Real 640×360/400 captures. Reviewable art provenance and editables if making new pixels | Only production sprites and behaviors which already have required approvals |
| BOIL-F | QA/acceptance & tuning on real game, solo Carl/Donut and local 2P, replays, camera, checkpoint, performance | Linked test commands/results; state-hash/replay checks; stage no-softlock assertions; quick human gamepad evaluation documented as **pending owner review** until actually played | PLAYTRACE-01 recorder; AUTOPLAY integration where available |

Aim for a **shippable base loop in BOIL-A/B/C** before the optional drum, chain cosmetics or new enemy family. A mid-floor test entry can expose the room with correctly declared **post-Hoarder** capabilities, but must never unlock dynamite or Donut spells early in the actual story. A real story transition may be deferred if Floor 01 global route wiring is still incomplete: the room itself must be executable end-to-end.

## Engineering contracts / safeguards

- Use explicit typed prop states and data knobs; same config + seed + tick inputs ⇒ same combat events and state hashes. Separate fixed-step domain simulation from Bevy draw/audio. No renderer or video frame chooses impact timing.
- Material/collider ownership remains in the existing level/terrain system. No new infinite physics cascade; explosions and environmental interactions resolve in stable order. Respect CP-02's baseline 0 enemy-to-enemy collision damage unless a separate tested feature changes it.
- Damage source must retain the initiating hero **even when** the final cause is steam, falling machinery or a secondary explosion. If the existing kill-attribution/event schema cannot represent it, propose the narrowest compatible extension with tests, not a silent incompatible change.
- One prop activation per attempt; one weight landing; one damage application per victim/token. Reject unknown prop IDs, invalid link graphs/cycles, out-of-bounds footprints and hazardous protected spawn/exit zones at load time.
- Include data-authoring / visualization of trigger volumes and contact boxes in debug mode. Warn before steam and weight damage; the warning may not be obscured by camera, foreground geometry or existing HUD.
- Do not change physical controls. Dynamite is an already-earned equipped special; Donut Magic Missile uses the existing 8-charge pool. Her automatic 12s dodge/reflect is owned by a **separate agent**: do **not** implement or retune it here; ensure physical hazards and dodgeable tags can interoperate when merged.
- Solo with character swap and 2P use identical critical stage path. Do not alter companion AI in this PR. Capture failures/lag in report for the companion team.
- Manual audio and visual spectacle are essential but **not** authoritative: no hidden radius extensions, auto-gather of targets, teleporting enemies into weight footprint, or scripted boss-style cutscene clear.
- Preserve untouched demo rules, live input recording defaults, current CI goldens, rights/provenance and isolated test-only capability profiles.

## Playtest checklist

1. Direct clear without touching any prop; no performance of "wrong play" and no progression gate requiring scenery.
2. Kick goblin into valve; visible rupture, warning, vent, stagger and subsequent normal combat.
3. Punch/throw at brace; visible release warning; weight genuinely lands on an enemy in an authored window.
4. Combine two mechanisms in either available valid order; optional drum only after independent tests pass.
5. Attempt bad situations: player under weight, two players separated, enemy dies before impact, valve triggered at last moment, explosion mid-fall, repeat strike after spent, checkpoint retry during every phase, all rubble states.
6. Verify logs/events and `playtrace verify` with true recorded fixed inputs when available; record unsupported integration as a blocker rather than asserting success.
7. At least one player-run evidence session per **direct**, **tactical**, **chain** style. Human fun assessment focuses on legibility, agency, distinctive spectacle, no forced puzzle, fast restart. No AI score alone substitutes for human signoff.

## PR completion template

- **Implemented:** exact files, runtime entry, feature gates and level loadout
- **Reused vs introduced:** which existing systems/props were reused; any small shared-domain primitives and justification
- **Tuning:** final data values and differences from hypotheses
- **QA:** actual commands and outcomes, scenario IDs BOIL-01..12, legacy hashes, no-softlock proof, 1P/2P status, performance
- **Evidence:** native screenshots, short at-speed real game capture and session replay IDs; do not use generated concept images as gameplay proof
- **Outstanding:** dependencies, unsupported enemy types, real-pad gaps, visual polish and owner signoff
- **Rights/provenance:** new assets if any
- **Review:** independent reviewer and green final CI head

**Done means playable and validated, not only documented.** Mark every missing gate and keep follow-up tasks separated by dependency; do not silently expand this PR into the other two future memorable scenarios (Dozer and Hoarder).
