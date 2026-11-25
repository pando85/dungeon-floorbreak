//! DM-C and DONUT-MOVES-ART-01: the Bevy half of Donut's reflex cues (spec section
//! 3.4, DM-Q21; art spec `docs/02-art/donut-moves-art-01-spec.md`).
//!
//! The pure state is `dungeon_floorbreak::donut_cues::DonutCues`; this file only
//! draws what it reports, from the production atlases:
//!
//! - the ready tell: the tail glint is the Puddle Jumper strip's 3 x 3 plus (the
//!   small twinkle on the 4th on-tick) on the tail tip, and the tail flick is the
//!   tail-swap atlas (`flick_up`, `flick_snap`) drawn over her locomotion frame;
//! - the evade response: the five moves-atlas frames (coil, arch, the twist key,
//!   untwist, land) with the cosmetic `arc_lift_px` offset, and the strip's swish
//!   pair on the side the attack came from. The DM-C afterimages are retired
//!   (DMA-Q06).
//!
//! **Presentation only**: nothing here writes to the simulation, the animators or
//! a resource other than its own. A missing atlas leaves its cue undrawn (her
//! locomotion sprite stays). The smug that follows an evade is the existing
//! CF02-I overlay, started by `ActingAnimators` (`combat_anim/cf02.rs`); it needs
//! `--cf02` like every other acting overlay.

use super::*;
use dungeon_floorbreak::{
    actor_sprite::FOOT_SINK_PX,
    donut_art::{self, PuddleVfx},
    donut_cues::{AtlasCues, DonutCues, analyze_atlas},
};

/// The three DONUT-MOVES-ART-01 atlases (moves, tail swap, Puddle Jumper strip), each with its
/// layout; `None` when the atlas is not on disk.
#[derive(Clone)]
pub(super) struct MovesArt {
    pub moves: Option<(Handle<Image>, Handle<TextureAtlasLayout>)>,
    pub tail: Option<(Handle<Image>, Handle<TextureAtlasLayout>)>,
    pub vfx: Option<(Handle<Image>, Handle<TextureAtlasLayout>)>,
}

/// Load the atlases that exist in `generated/art` and return them with the handles the loading
/// watchdog waits for. The strip's layout carries two extra entries after its 16 frames: the
/// tileable streak dash and the streak head.
pub(super) fn load_art(
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> (MovesArt, Vec<(String, Handle<Image>)>) {
    let art_dir = std::env::current_dir()
        .ok()
        .map(|root| root.join("generated/art"));
    let on_disk = |name: &str| art_dir.as_ref().is_some_and(|dir| dir.join(name).is_file());
    let mut handles = Vec::new();
    let mut load = |name: &str,
                    w: u32,
                    h: u32,
                    count: usize,
                    extra: &[(u32, u32, u32, u32, usize)]|
     -> Option<(Handle<Image>, Handle<TextureAtlasLayout>)> {
        if !on_disk(name) {
            return None;
        }
        let texture: Handle<Image> = asset_server.load(name.to_owned());
        handles.push((name.to_owned(), texture.clone()));
        let mut layout =
            TextureAtlasLayout::from_grid(UVec2::new(w, h), count as u32, 1, None, None);
        for &(x, y, rw, rh, frame) in extra {
            let ox = frame as u32 * w + x;
            layout.add_texture(URect::new(ox, y, ox + rw, y + rh));
        }
        Some((texture, layouts.add(layout)))
    };
    let (dx, dy, dw, dh) = donut_art::PUDDLE_DASH_RECT;
    let (hx, hy, hw, hh) = donut_art::PUDDLE_HEAD_RECT;
    let art = MovesArt {
        moves: load(
            donut_art::MOVES_ATLAS,
            donut_art::FRAME_W,
            donut_art::FRAME_H,
            donut_art::MOVES_FRAMES,
            &[],
        ),
        tail: load(
            donut_art::TAIL_READY_ATLAS,
            donut_art::FRAME_W,
            donut_art::FRAME_H,
            donut_art::TAIL_READY_FRAMES,
            &[],
        ),
        vfx: load(
            donut_art::PUDDLE_VFX_ATLAS,
            donut_art::PUDDLE_VFX_W,
            donut_art::PUDDLE_VFX_H,
            donut_art::PUDDLE_VFX_FRAMES,
            &[
                (dx, dy, dw, dh, donut_art::PUDDLE_DASH_FRAME),
                (hx, hy, hw, hh, donut_art::PUDDLE_HEAD_FRAME),
            ],
        ),
    };
    (art, handles)
}

/// World centre of a strip sprite whose pivot pixel (the puddle surface row) sits on `foot_y` at
/// column `x`: the strip's pivot is the foot line, like her frames' pivot.
pub(super) fn vfx_center(x: f32, foot_y: f32) -> (f32, f32) {
    let h = donut_art::PUDDLE_VFX_H as f32;
    let pivot_y = donut_art::PUDDLE_VFX_PIVOT.1 as f32;
    let off_x = donut_art::PUDDLE_VFX_W as f32 / 2.0 - donut_art::PUDDLE_VFX_PIVOT.0 as f32;
    (
        (x + off_x).round(),
        (foot_y - FOOT_SINK_PX as f32 - (h - 1.0 - pivot_y) + h / 2.0).round(),
    )
}

/// The cue sprites, pooled once at startup.
#[derive(Component, Clone, Copy)]
pub(super) enum CueSprite {
    /// The glint (a strip frame) on the tail tip.
    Glint,
    /// Her tail-swap frame: the flick, drawn over her locomotion frame.
    Tail,
    /// Her moves-atlas frame on the evade arc.
    Arc,
    /// The swish (a strip frame).
    Swish,
}

/// The cue state of the run, owned by the presenter.
#[derive(Resource)]
pub(super) struct DonutCueRes {
    cues: DonutCues,
    texture: Handle<Image>,
    tail_texture: Option<Handle<Image>>,
    /// The moves atlas was loaded: the evade arc can draw her (else her locomotion sprite stays).
    has_moves: bool,
    /// Per-frame tail tips of the locomotion atlas, read once from the loaded atlas.
    atlas: Option<AtlasCues>,
    /// The same for the tail-swap atlas (the flick moves the tip, so the glint follows it).
    tail_atlas: Option<AtlasCues>,
    /// An atlas had pixels that this presenter cannot read (no glint on it).
    unreadable: bool,
}

/// Spawn the pool of cue sprites and the resource. Called once from `setup`,
/// only in a combat scene (the cues need the combat log).
pub(super) fn spawn(
    commands: &mut Commands,
    texture: Handle<Image>,
    layers: RenderLayers,
    z: f32,
    art: &MovesArt,
) {
    let mut sprite =
        |kind: CueSprite, atlas: &Option<(Handle<Image>, Handle<TextureAtlasLayout>)>, dz: f32| {
            let Some((texture, layout)) = atlas.clone() else {
                return;
            };
            commands.spawn((
                kind,
                Sprite::from_atlas_image(texture, TextureAtlas { layout, index: 0 }),
                Transform::from_xyz(0.0, 0.0, z + dz),
                Visibility::Hidden,
                layers.clone(),
            ));
        };
    sprite(CueSprite::Glint, &art.vfx, 0.02);
    sprite(CueSprite::Swish, &art.vfx, 0.03);
    sprite(CueSprite::Arc, &art.moves, 0.005);
    sprite(CueSprite::Tail, &art.tail, 0.004);
    commands.insert_resource(DonutCueRes {
        cues: DonutCues::default(),
        texture,
        tail_texture: art.tail.as_ref().map(|(t, _)| t.clone()),
        has_moves: art.moves.is_some(),
        atlas: None,
        tail_atlas: None,
        unreadable: false,
    });
}

/// Read the tail tips from the loaded atlases, once each (the highlight colour is no longer needed:
/// the glint is strip art).
fn read_atlas(state: &mut DonutCueRes, actors: &Actors, images: &Assets<Image>) {
    if state.unreadable {
        return;
    }
    let spec = actors.donut.spec;
    let analyse = |texture: &Handle<Image>, frames: usize| -> Option<Option<AtlasCues>> {
        let image = images.get(texture)?; // still loading
        Some(match image.data.as_deref() {
            Some(data) if data.len() == (image.width() * image.height() * 4) as usize => {
                Some(analyze_atlas(
                    data,
                    image.width(),
                    spec.width as u32,
                    spec.height as u32,
                    frames,
                ))
            }
            _ => None,
        })
    };
    if state.atlas.is_none() {
        match analyse(&state.texture, actors.donut.clips.frame_count) {
            Some(Some(cues)) => state.atlas = Some(cues),
            Some(None) => state.unreadable = true,
            None => {}
        }
    }
    if state.tail_atlas.is_none()
        && let Some(texture) = state.tail_texture.clone()
    {
        match analyse(&texture, donut_art::TAIL_READY_FRAMES) {
            Some(Some(cues)) => state.tail_atlas = Some(cues),
            Some(None) => state.unreadable = true,
            None => {}
        }
    }
}

/// Draw the cues for the tick on screen. Runs after the sprite systems that own
/// Donut's visibility (`sync_donut_visuals`, `sync_cf02_acting_visuals`): a cue
/// that sits on her locomotion frame is drawn only while that frame is.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn sync_donut_cues(
    sim: Res<Sim>,
    combat: Res<Combat>,
    actors: Res<Actors>,
    images: Res<Assets<Image>>,
    state: Option<ResMut<DonutCueRes>>,
    mut base: Query<(&ActorVisual, &Transform, &mut Sprite, &mut Visibility), Without<CueSprite>>,
    mut cues: Query<
        (&CueSprite, &mut Transform, &mut Sprite, &mut Visibility),
        Without<ActorVisual>,
    >,
) {
    let (Some(mut state), Some(combat)) = (state, combat.0.as_ref()) else {
        return;
    };
    read_atlas(&mut state, &actors, &images);
    let hide_all = |cues: &mut Query<
        (&CueSprite, &mut Transform, &mut Sprite, &mut Visibility),
        Without<ActorVisual>,
    >| {
        for (_, _, _, mut visibility) in cues.iter_mut() {
            *visibility = Visibility::Hidden;
        }
    };
    let sim = &sim.0;
    if !sim.party_live() {
        hide_all(&mut cues);
        return;
    }
    let tick = sim.world.tick;
    let donut_x = sim.world.donut_body.x;
    state.cues.step(tick, combat, donut_x);

    // Her locomotion sprite as the earlier systems left it this frame.
    let Some((center, flip, frame, tint, shown)) = base
        .iter()
        .find(|(actor, ..)| !actor.is_hero)
        .map(|(_, transform, sprite, visibility)| {
            (
                transform.translation.truncate(),
                sprite.flip_x,
                sprite.texture_atlas.as_ref().map_or(0, |a| a.index),
                sprite.color,
                *visibility != Visibility::Hidden,
            )
        })
    else {
        hide_all(&mut cues);
        return;
    };

    let spec = actors.donut.spec;
    let (w, h) = (spec.width as f32, spec.height as f32);
    let evade = state.cues.evade(tick);
    let arc_on = evade.is_some() && shown && state.has_moves;
    let glint = state
        .cues
        .glint_age(tick)
        .filter(|_| shown && evade.is_none());
    let flick = state
        .cues
        .flick(tick)
        .filter(|_| shown && evade.is_none() && state.tail_texture.is_some());
    // The tail-swap frame replaces the locomotion frame for the flick; the glint follows its tip.
    let tail_index = flick.map(|phase| donut_art::tail_ready_index(phase, frame));
    let tip = match tail_index {
        Some(index) => state
            .tail_atlas
            .as_ref()
            .and_then(|a| a.tips.get(index).and_then(Option::as_ref)),
        None => state
            .atlas
            .as_ref()
            .and_then(|a| a.tips.get(frame).and_then(Option::as_ref)),
    };

    // Pixel (x, y from the top) of her current frame to world coordinates.
    let to_world = |px: f32, py: f32| -> (f32, f32) {
        let x = if flip {
            center.x + w / 2.0 - px
        } else {
            center.x - w / 2.0 + px
        };
        (x, center.y + h / 2.0 - py)
    };

    for (kind, mut transform, mut sprite, mut visibility) in &mut cues {
        // Where the cue is drawn (`None` hides it), its strip or atlas frame and its depth.
        let (at, index): (Option<(f32, f32)>, usize) = match *kind {
            CueSprite::Glint => {
                // The strip's plus is centred on its pivot pixel: put that pixel on the tail tip.
                let frame = glint
                    .and_then(|age| donut_art::puddle_vfx_frame(PuddleVfx::Glint, age))
                    .unwrap_or(0);
                let at = match (tip.filter(|_| glint.is_some()), donut_art::PUDDLE_VFX_PIVOT) {
                    (Some(t), (pivot_x, pivot_y)) => {
                        let (x, y) = to_world(t.x as f32 + 0.5, t.y as f32 + 0.5);
                        let off_x = donut_art::PUDDLE_VFX_W as f32 / 2.0 - (pivot_x as f32 + 0.5);
                        let off_y = (pivot_y as f32 + 0.5) - donut_art::PUDDLE_VFX_H as f32 / 2.0;
                        Some((x + off_x, y + off_y))
                    }
                    _ => None,
                };
                (at, frame)
            }
            CueSprite::Tail => {
                let at = tail_index.map(|_| (center.x, center.y));
                if at.is_some() {
                    sprite.flip_x = flip;
                    sprite.color = tint;
                }
                (at, tail_index.unwrap_or(0))
            }
            CueSprite::Arc => {
                let view = evade.filter(|_| arc_on);
                let at = view.map(|v| (center.x, center.y + v.lift_px as f32));
                if at.is_some() {
                    sprite.flip_x = flip;
                    sprite.color = tint;
                }
                (
                    at,
                    view.and_then(|v| donut_art::evade_frame(v.age))
                        .unwrap_or(0),
                )
            }
            CueSprite::Swish => {
                // The visible path of the attack she slipped, on the side it came from: drawn
                // travelling toward her, so an attacker on her right flips it.
                let view = evade.and_then(|view| {
                    let side = view.streak?;
                    let frame = donut_art::puddle_vfx_frame(PuddleVfx::Swish, view.age)?;
                    Some((side, frame))
                });
                match view {
                    Some((side, frame)) => {
                        sprite.flip_x = side > 0;
                        sprite.color = Color::WHITE;
                        (
                            Some((center.x + f32::from(side) * 6.0, center.y + 2.0)),
                            frame,
                        )
                    }
                    None => (None, 14),
                }
            }
        };
        match at {
            Some((x, y)) => {
                transform.translation.x = x;
                transform.translation.y = y;
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = index;
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    // The arc or the tail swap draws her: the locomotion sprite steps aside while it runs.
    if arc_on || tail_index.is_some() {
        for (actor, _, _, mut visibility) in &mut base {
            if !actor.is_hero {
                *visibility = Visibility::Hidden;
            }
        }
    }
}
