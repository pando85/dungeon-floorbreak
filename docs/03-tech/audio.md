# Audio integration (AUDIO-01)

Decision record: [ADR-0008](../05-decisions/ADR-0008-audio.md). Asset pack and its own docs: `assets/audio/floor01/` (`docs/README.md`, `AUDIO_DIRECTION.md`, `REVIEW_SCORECARD.md`). The pack docs name a franchise title and fan-layer characters; that stays in the asset docs and in `cue_map.json` data only.

## Layers

| Layer | File | Notes |
| --- | --- | --- |
| Domain (headless, no Bevy audio types) | `src/audio.rs` | Cue ids, music/ambience states, gameplay-event to cue translation, cue bank, settings, variant RNG, voice cap, music duck. |
| Data | `assets/audio/floor01/cue_map.json`, `docs/audio_events.json`, `docs/asset_manifest.json` | `cue_map.json` maps the IP-neutral ids to the pack's event ids. Swap the right-hand side to re-skin. |
| Bevy adapter | `src/audio_adapter.rs` | `FloorAudioPlugin`, `AudioQueue`. Loads the Ogg files, plays one-shots and loops, applies bus gains and the duck. |

## What emits what

| Gameplay event | Cue |
| --- | --- |
| `CombatEvent::MoveStarted` (every swing, hit or miss; jab, haymaker and push kick all use the kick whoosh until punch audio exists) | `PlayerKickSwing` |
| `CombatEvent::Hit` (only a verified hit from `resolve_strike`) | `PlayerKickHitConfirmed` (ducks music) and `EnemyHitReaction` |
| Bomb destroys a timber barrier in the prototype world (`World::timbers_broken_last_step`) | `WallBreak` |
| Floor 1 scene start | music `Tension` (no fixture) or `Combat` (fixture), ambience `Ventilation` |

Future hooks (defined, mapped and tested, but nothing emits them yet because the game has no such systems): `EnemyBodyCollision` and `HeavyPropCrash` (via `ContactEvent` and `cues_from_contact`, for enemy/prop physics and gym weights), `Warning` (siren), music `Chase` (pursuit), music and ambience `Gym` (gym section). Wire them where those systems land; do not invent gameplay for them.

## Controls and switches

- `--no-audio` (both binaries) or `FLOORBREAK_AUDIO=off`: no sound, no audio device. Screenshot, reference-frame and perf runs (`--freeze`, `--capture-*`, `--reference-frame`, `--perf-frames`) are silent automatically. The headless smokes never create an app.
- Style bank (changes the hit-confirm impacts): `FLOORBREAK_AUDIO_STYLE=arcade|cinematic|hybrid` (default `hybrid`), `--audio-style <name>` on `floor01_scene`, or **F9** in the window to cycle arcade, cinematic, hybrid. The active bank is printed as `AUDIO_STYLE <name>`.
- **F10** mutes and unmutes everything (prints `AUDIO_MUTE`).
- Volumes (linear, 0 to 2): `FLOORBREAK_VOLUME_MASTER`, `_MUSIC`, `_AMBIENCE`, `_SFX`. Defaults: music 0.8, ambience 0.5, sfx 1.0, master 1.0. `FLOORBREAK_AUDIO_SEED` seeds the variant picker.

Try it from the repository root:

```bash
cargo run --bin floor01_scene -- --fixture cp01_kick --manual
FLOORBREAK_AUDIO_STYLE=arcade cargo run --bin floor01_scene -- --fixture cp01_two_hits
```

## Rules the code keeps

- Presentation only: the simulation only pushes commands; nothing is read back, hashed or replayed. Variant choice uses a presentation RNG, never the gameplay RNG.
- At most 2 overlapping major contact transients (hit-confirm, body collision, wall break, heavy crash); extra ones are dropped, not queued.
- Music duck on a verified hit: about 3.2 dB (from the pack mixer), 150 ms (clamped to 100-180 ms), 40 ms release.
- With no audio device, one-shots that never get a sink are despawned after about 1 s (Bevy 0.20 would otherwise keep them forever). F10 mute also silences effects already playing.
- Music and ambience loop; a music state change cuts to the new loop (no crossfade yet).

## Tests

- `src/audio.rs` unit tests: cue emission for swing, miss, verified hit, contacts and a real bomb wall break; every combat fixture checked for one swing cue per move and one confirm per real hit; settings, RNG, voice cap, duck.
- `tests/audio_assets.rs`: every cue, state and style bank resolves to an existing Ogg; all 38 manifest assets exist with matching sha256 and none is unlisted; mixer values; engine cue names are IP-neutral.
- Audio itself cannot be asserted headless. Listening is a human review; use `assets/audio/floor01/docs/REVIEW_SCORECARD.md`.
