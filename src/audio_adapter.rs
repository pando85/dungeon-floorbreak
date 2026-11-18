//! Bevy adapter for the Floor 01 audio (AUDIO-01, ADR-0008).
//!
//! Plays what the pure `audio` domain resolves, with Bevy's built-in audio
//! (`bevy_audio`, Ogg Vorbis). The simulation pushes [`AudioCommand`]s into the
//! [`AudioQueue`] and never reads anything back, so sound cannot change an
//! outcome. With audio disabled (`--no-audio`, `FLOORBREAK_AUDIO=off`, a
//! screenshot or perf run) the queue is drained and dropped and no audio device
//! is opened; with no device present Bevy only logs a warning.
//!
//! The Ogg files are read from the pack directory at startup and added to
//! `Assets<AudioSource>` directly, so playback does not depend on the asset
//! root of the host binary.

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::audio::{
    AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, PlaybackSettings, Volume,
};
use bevy::prelude::*;

use crate::audio::{
    AmbienceState, AudioCommand, AudioSettings, Bus, CueBank, CueId, Ducker, MusicState,
    PresentationRng, VoiceLimiter,
};

/// Commands waiting for the next frame. Always present, so callers never
/// branch on whether audio is on.
#[derive(Resource, Default, Debug)]
pub struct AudioQueue(Vec<AudioCommand>);

impl AudioQueue {
    pub fn push(&mut self, command: AudioCommand) {
        self.0.push(command);
    }

    pub fn push_cues(&mut self, cues: impl IntoIterator<Item = CueId>) {
        self.0.extend(cues.into_iter().map(AudioCommand::Cue));
    }
}

/// Key that cycles the style bank (arcade, cinematic, hybrid).
pub const STYLE_KEY: KeyCode = KeyCode::F9;
/// Key that mutes and unmutes everything.
pub const MUTE_KEY: KeyCode = KeyCode::F10;

pub struct FloorAudioPlugin {
    pub settings: AudioSettings,
    /// The pack directory (`assets/audio/floor01` under the repository root).
    pub pack_dir: PathBuf,
    /// Commands queued before the first frame (the scene's music and bed).
    pub initial: Vec<AudioCommand>,
}

#[derive(Component)]
struct MusicVoice;

#[derive(Component)]
struct AmbienceVoice;

/// A one-shot sound effect. With no audio device Bevy 0.20 never attaches a
/// sink, so its despawn-on-finish never fires; [`reap_orphan_voices`] removes
/// such entities after [`ORPHAN_GRACE`] seconds.
#[derive(Component)]
struct OneShotVoice {
    spawned_at: f32,
}

/// Seconds a one-shot may wait for its sink before it counts as orphaned.
const ORPHAN_GRACE: f32 = 1.0;

/// The pack directory: `assets/audio/floor01` under the working directory when
/// it is there (a run from the repository root, like the scene binary), else
/// under the crate root (a dev build started elsewhere).
pub fn resolve_pack_dir() -> PathBuf {
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .join(crate::audio::PACK_DIR);
    if cwd.is_dir() {
        cwd
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(crate::audio::PACK_DIR)
    }
}

#[derive(Resource)]
struct AudioRuntime {
    settings: AudioSettings,
    bank: CueBank,
    pack_dir: PathBuf,
    sources: HashMap<String, Handle<AudioSource>>,
    rng: PresentationRng,
    limiter: VoiceLimiter,
    ducker: Ducker,
    clock: f32,
    last_take: HashMap<CueId, usize>,
    music: Option<(MusicState, Entity)>,
    ambience: Option<(AmbienceState, Entity)>,
    muted: bool,
}

impl AudioRuntime {
    fn gain(&self, bus: Bus) -> f32 {
        if self.muted {
            0.0
        } else {
            self.settings.bus_gain(bus)
        }
    }

    fn music_gain(&self) -> f32 {
        self.gain(Bus::Music) * self.ducker.gain()
    }
}

impl Plugin for FloorAudioPlugin {
    fn build(&self, app: &mut App) {
        let mut queue = AudioQueue::default();
        let bank = if self.settings.enabled {
            match CueBank::load(&self.pack_dir) {
                Ok(bank) => Some(bank),
                Err(error) => {
                    eprintln!("AUDIO_DISABLED cannot load the cue bank: {error}");
                    None
                }
            }
        } else {
            None
        };
        let Some(bank) = bank else {
            app.insert_resource(queue)
                .add_systems(Update, discard_queue);
            return;
        };
        for command in &self.initial {
            queue.push(*command);
        }
        let runtime = AudioRuntime {
            rng: PresentationRng::new(self.settings.seed),
            limiter: VoiceLimiter::new(bank.mixer.max_major_contacts),
            ducker: Ducker::new(bank.mixer.duck_gain()),
            settings: self.settings.clone(),
            pack_dir: self.pack_dir.clone(),
            bank,
            sources: HashMap::new(),
            clock: 0.0,
            last_take: HashMap::new(),
            music: None,
            ambience: None,
            muted: false,
        };
        println!("AUDIO_STYLE {}", runtime.settings.style.name());
        app.insert_resource(queue)
            .insert_resource(runtime)
            .add_systems(Startup, load_sources)
            .add_systems(
                Update,
                (debug_keys, play_queue, reap_orphan_voices, apply_bus_gains).chain(),
            );
    }
}

fn discard_queue(mut queue: ResMut<AudioQueue>) {
    queue.0.clear();
}

/// Read every file the bank can play into `Assets<AudioSource>` once.
fn load_sources(mut runtime: ResMut<AudioRuntime>, mut assets: ResMut<Assets<AudioSource>>) {
    let paths = runtime.bank.referenced_assets();
    for path in paths {
        match std::fs::read(runtime.pack_dir.join(&path)) {
            Ok(bytes) => {
                let handle = assets.add(AudioSource {
                    bytes: bytes.into(),
                });
                runtime.sources.insert(path, handle);
            }
            Err(error) => eprintln!("AUDIO_MISSING {path}: {error}"),
        }
    }
}

fn debug_keys(keys: Res<ButtonInput<KeyCode>>, mut runtime: ResMut<AudioRuntime>) {
    if keys.just_pressed(STYLE_KEY) {
        runtime.settings.style = runtime.settings.style.next();
        println!("AUDIO_STYLE {}", runtime.settings.style.name());
    }
    if keys.just_pressed(MUTE_KEY) {
        runtime.muted = !runtime.muted;
        println!("AUDIO_MUTE {}", runtime.muted);
    }
}

fn play_queue(
    mut commands: Commands,
    mut queue: ResMut<AudioQueue>,
    mut runtime: ResMut<AudioRuntime>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();
    let rt = &mut *runtime;
    rt.clock += dt;
    rt.ducker.tick(dt);
    for command in std::mem::take(&mut queue.0) {
        match command {
            AudioCommand::Cue(cue) => play_cue(&mut commands, rt, cue),
            AudioCommand::Music(state) => start_music(&mut commands, rt, state),
            AudioCommand::Ambience(state) => start_ambience(&mut commands, rt, state),
        }
    }
}

fn play_cue(commands: &mut Commands, rt: &mut AudioRuntime, cue: CueId) {
    if rt.muted {
        return;
    }
    let takes = rt.bank.cue_assets(cue, rt.settings.style);
    if takes.is_empty() {
        return;
    }
    // A dropped cue must not consume the presentation RNG.
    let mut rng = rt.rng.clone();
    let pick = rng.pick_variant(takes.len(), rt.last_take.get(&cue).copied());
    let path = takes[pick].clone();
    if cue.is_major_contact() && !rt.limiter.try_acquire(rt.clock, rt.bank.duration(&path)) {
        return;
    }
    rt.rng = rng;
    rt.last_take.insert(cue, pick);
    if cue.ducks_music() {
        rt.ducker.trigger(rt.bank.mixer.duck_seconds);
    }
    spawn_one_shot(commands, rt, &path, Bus::Sfx);
}

fn spawn_one_shot(commands: &mut Commands, rt: &AudioRuntime, path: &str, bus: Bus) {
    let Some(handle) = rt.sources.get(path) else {
        return;
    };
    commands.spawn((
        OneShotVoice {
            spawned_at: rt.clock,
        },
        AudioPlayer::<AudioSource>(handle.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(rt.gain(bus))),
    ));
}

fn start_music(commands: &mut Commands, rt: &mut AudioRuntime, state: MusicState) {
    if rt.music.map(|(s, _)| s) == Some(state) {
        return;
    }
    if let Some((_, entity)) = rt.music.take() {
        commands.entity(entity).despawn();
    }
    let Some(plan) = rt.bank.music_plan(state) else {
        return;
    };
    let Some(handle) = rt.sources.get(&plan.looped) else {
        return;
    };
    let entity = commands
        .spawn((
            MusicVoice,
            AudioPlayer::<AudioSource>(handle.clone()),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(rt.music_gain())),
        ))
        .id();
    rt.music = Some((state, entity));
    for one_shot in plan.one_shots {
        spawn_one_shot(commands, rt, &one_shot, Bus::Sfx);
    }
}

fn start_ambience(commands: &mut Commands, rt: &mut AudioRuntime, state: AmbienceState) {
    if rt.ambience.map(|(s, _)| s) == Some(state) {
        return;
    }
    if let Some((_, entity)) = rt.ambience.take() {
        commands.entity(entity).despawn();
    }
    let Some(path) = rt.bank.ambience_asset(state).map(str::to_owned) else {
        return;
    };
    let Some(handle) = rt.sources.get(&path) else {
        return;
    };
    let entity = commands
        .spawn((
            AmbienceVoice,
            AudioPlayer::<AudioSource>(handle.clone()),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(rt.gain(Bus::Ambience))),
        ))
        .id();
    rt.ambience = Some((state, entity));
}

type OneShotSinks<'w, 's> = Query<
    'w,
    's,
    &'static mut AudioSink,
    (
        With<OneShotVoice>,
        Without<MusicVoice>,
        Without<AmbienceVoice>,
    ),
>;

/// Despawn one-shots that never got a sink (no audio device).
fn reap_orphan_voices(
    mut commands: Commands,
    runtime: Res<AudioRuntime>,
    voices: Query<(Entity, &OneShotVoice), Without<AudioSink>>,
) {
    for (entity, voice) in &voices {
        if runtime.clock - voice.spawned_at > ORPHAN_GRACE {
            commands.entity(entity).despawn();
        }
    }
}

/// Keep the looping buses at their gain: bus volume, master, mute and the duck.
fn apply_bus_gains(
    runtime: Res<AudioRuntime>,
    mut music: Query<&mut AudioSink, (With<MusicVoice>, Without<AmbienceVoice>)>,
    mut ambience: Query<&mut AudioSink, (With<AmbienceVoice>, Without<MusicVoice>)>,
    mut one_shots: OneShotSinks,
) {
    // Mute also silences effects that are already playing.
    if runtime.muted {
        for mut sink in &mut one_shots {
            sink.set_volume(Volume::Linear(0.0));
        }
    }
    for mut sink in &mut music {
        sink.set_volume(Volume::Linear(runtime.music_gain()));
    }
    for mut sink in &mut ambience {
        sink.set_volume(Volume::Linear(runtime.gain(Bus::Ambience)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> AudioRuntime {
        let pack_dir = resolve_pack_dir();
        let bank = CueBank::load(&pack_dir).expect("bank");
        AudioRuntime {
            rng: PresentationRng::new(1),
            limiter: VoiceLimiter::new(2),
            ducker: Ducker::new(0.7),
            settings: AudioSettings::default(),
            pack_dir,
            bank,
            sources: HashMap::new(),
            clock: 0.0,
            last_take: HashMap::new(),
            music: None,
            ambience: None,
            muted: false,
        }
    }

    #[test]
    fn one_shots_that_never_get_a_sink_are_reaped() {
        let mut app = App::new();
        app.insert_resource(runtime());
        app.add_systems(Update, reap_orphan_voices);
        let fresh = app.world_mut().spawn(OneShotVoice { spawned_at: 0.0 }).id();
        let other = app.world_mut().spawn(OneShotVoice { spawned_at: 0.0 }).id();
        app.update();
        assert!(app.world().get_entity(fresh).is_ok(), "within the grace");
        app.world_mut().resource_mut::<AudioRuntime>().clock = ORPHAN_GRACE + 0.1;
        let later = app
            .world_mut()
            .spawn(OneShotVoice { spawned_at: 1.05 })
            .id();
        app.update();
        assert!(app.world().get_entity(fresh).is_err(), "orphan removed");
        assert!(app.world().get_entity(other).is_err(), "orphan removed");
        assert!(app.world().get_entity(later).is_ok(), "a young one stays");
    }

    #[test]
    fn a_dropped_cue_does_not_consume_the_variant_rng() {
        let mut rt = runtime();
        rt.sources.clear();
        let mut commands_world = World::new();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &commands_world);
        // Fill the major-contact cap, then ask for one more hit confirm.
        assert!(rt.limiter.try_acquire(0.0, 5.0));
        assert!(rt.limiter.try_acquire(0.0, 5.0));
        let before = rt.rng.clone().next_u64();
        play_cue(&mut commands, &mut rt, CueId::PlayerKickHitConfirmed);
        assert_eq!(rt.rng.clone().next_u64(), before);
        queue.apply(&mut commands_world);
    }
}
