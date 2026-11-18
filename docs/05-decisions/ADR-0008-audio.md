# ADR-0008: Audio through Bevy's built-in audio, with event-driven cues

- **Status:** Accepted (integration only; the sound family is still provisional, see Consequences)
- **Date:** 2026-11-18
- **Deciders:** project owner (asked for the Floor 01 music and sounds to be integrated and merged)
- **Evidence:** work order AUDIO-01; the Floor 01 audio pack under `assets/audio/floor01/` and its docs (`docs/README.md`, `audio_events.json`, `asset_manifest.json`, `QA_REPORT.md`, `REVIEW_SCORECARD.md`); [audio.md](../03-tech/audio.md).

## Context

- The pack ships 38 runtime Ogg Vorbis files (music loops, ambience beds, one-shot SFX, and three style banks of hit impacts) plus a proposed event table (`audio_events.json`) and a mixer section.
- The game had no audio at all. ADR-0001 fixes Rust + Bevy; ADR-0004 requires engine and data contracts to be IP-neutral; the architecture keeps gameplay logic headless and deterministic, with rendering and effects unable to change outcomes.
- The pack's event ids use fan-layer names (`carl.kick.*`, `goblin.hit`). Its own review notes say the true test is the integrated, interactive fight, so the integration must allow A/B/C comparison of the style banks.

## Decision

1. **Backend: Bevy's built-in audio.** Enable the `bevy_audio` and `vorbis` features of the `bevy` dependency (`=0.20.1`). No new direct dependency. The features add `rodio` and its Ogg decoder (`lewton`, `ogg`) to `Cargo.lock`; the system libraries (`libasound2-dev` on Linux) were already installed by every CI job that builds the game. `kira` and `bevy_kira_audio` were not needed: the required behaviour (one-shots, loops, per-sink volume) is covered by the built-in API.
2. **Three buses: music, ambience, sfx**, plus a master. Gains come from defaults in code (music 0.8, ambience 0.5, sfx 1.0, master 1.0), overridable with `FLOORBREAK_VOLUME_{MASTER,MUSIC,AMBIENCE,SFX}`. The pack's `mixer` section supplies the default style bank (`hybrid`), the music duck (`gain_fade_on_big_hit_db`, `duck_ms` clamped to 100-180 ms) and the cap of 2 overlapping major contact transients. The pack has no per-bus gain numbers, so those defaults are ours and are a listening decision for the owner.
3. **Event-driven cues in a pure domain layer.** `src/audio.rs` has no Bevy audio types. Gameplay events become IP-neutral cue ids (`PlayerKickSwing`, `PlayerKickHitConfirmed`, `EnemyHitReaction`, `EnemyBodyCollision`, `WallBreak`, `HeavyPropCrash`, `Warning`) and music/ambience states (`Tension`, `Combat`, `Chase`, `Gym`; `Ventilation`, `Gym`). The mapping from those ids to the pack's event ids and files is data (`assets/audio/floor01/cue_map.json`), so the fan-layer names stay in data and asset docs, never in code identifiers. The Bevy adapter (`src/audio_adapter.rs`) only plays what the domain resolves.
4. **Presentation never affects gameplay.** The simulation pushes commands into a queue and never reads from audio. Variant choice uses a separate seeded presentation RNG, never the gameplay RNG, and nothing audio-related is hashed, logged to replays or fed back. With audio disabled (`--no-audio`, `FLOORBREAK_AUDIO=off`, screenshot/reference/perf runs) the audio device is never opened and the queue is discarded, so headless runs and CI need no sound hardware. If a device is missing at runtime Bevy logs "No audio device found" and never attaches a sink, so despawn-on-finish never fires; the adapter tags every one-shot and despawns the ones still without a sink after about 1 s, so effects do not leak.
5. **Playback rules.** Random take per cue without immediate repeats; at most 2 overlapping major contact transients (extra ones are dropped); a short music duck (about 3.2 dB, 150 ms, 40 ms release) on a verified hit-confirm; looped music and ambience; style bank switch by `FLOORBREAK_AUDIO_STYLE`, `--audio-style` or the F9 debug key (F10 mutes).

## Consequences

- The sound family stays provisional and unapproved: the pack is procedural DSP placeholder material and the owner must judge game feel (ADR-0006 and AGENTS.md require human review before a mechanic is accepted).
- Only the cues with a gameplay hook are emitted today (kick swing, hit-confirm, enemy reaction, bomb wall break, scene music and ambience). `EnemyBodyCollision`, `HeavyPropCrash`, `Warning`, and the chase and gym states are defined, mapped and tested as contracts but have no emitter until enemy/prop physics, the gym and the chase exist.
- The Ogg files are read at startup into Bevy `Assets<AudioSource>` directly (about 1.2 MB), so playback does not depend on the scene's asset root.
- The DSP generator source named in the pack's manifest is not in this repository; reproducibility is a follow-up (see [rights-and-release.md](../04-production/rights-and-release.md)).
- No crossfade between music states yet (a state change cuts to the new loop).
