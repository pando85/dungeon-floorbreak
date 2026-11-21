//! DM-C: the Bevy half of Donut's reflex cues (spec section 3.4, DM-Q21).
//!
//! The pure state is `dungeon_floorbreak::donut_cues::DonutCues`; this file only
//! draws what it reports. **Presentation only and no new art**: the cues are
//! code-drawn overlays on her existing approved locomotion frames (the jump-rise
//! frame for the evade arc and its afterimages, copies of her own tail-tip pixels
//! for the flick) plus two code-drawn rectangles (the glint, in the brightest
//! colour of her own atlas, and the white streak). Nothing here writes to the
//! simulation, the animators or a resource other than its own.
//!
//! The smug that follows an evade is the existing CF02-I overlay, started by
//! `ActingAnimators` (`combat_anim/cf02.rs`); it needs `--cf02` like every other
//! acting overlay.

use std::collections::VecDeque;

use super::*;
use dungeon_floorbreak::{
    anim::Tag,
    donut_cues::{
        AtlasCues, DonutCues, GLINT_SIZE_PX, STREAK_HALF_LENGTH_PX, STREAK_THICKNESS_PX,
        analyze_atlas, arc_lift_px,
    },
};

/// The cue sprites, pooled once at startup.
#[derive(Component, Clone, Copy)]
pub(super) enum CueSprite {
    /// The 2 x 2 tail glint.
    Glint,
    /// One of the (at most nine) 1 x 1 pixels of the shifted tail tip.
    Flick(usize),
    /// Her sprite on the evade arc.
    Arc,
    /// The `n`th afterimage.
    After(usize),
    /// The white streak.
    Streak,
}

/// Opaque pixels of the tail-tip cluster (the tip and its 8-neighbourhood).
const FLICK_PIXELS: usize = 9;

/// Frames of her own history kept for the afterimages (the longest lag + 1).
const HISTORY: usize = 8;

/// The cue state of the run, owned by the presenter.
#[derive(Resource)]
pub(super) struct DonutCueRes {
    cues: DonutCues,
    texture: Handle<Image>,
    /// Per-frame tail tips and the glint colour, read once from the loaded atlas.
    atlas: Option<AtlasCues>,
    /// The atlas had pixels that this presenter cannot read (no glint or flick).
    unreadable: bool,
    /// Her drawn centre (x, y) on the last [`HISTORY`] ticks, newest last.
    history: VecDeque<(f32, f32)>,
    last_tick: Option<u64>,
}

/// Spawn the pool of cue sprites and the resource. Called once from `setup`,
/// only in a combat scene (the cues need the combat log).
pub(super) fn spawn(
    commands: &mut Commands,
    actors: &Actors,
    texture: Handle<Image>,
    layouts: &mut Assets<TextureAtlasLayout>,
    layers: RenderLayers,
    z: f32,
) {
    let layout = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(
            actors.donut.spec.width as u32,
            actors.donut.spec.height as u32,
        ),
        actors.donut.clips.frame_count as u32,
        1,
        None,
        None,
    ));
    let mut rect = |kind: CueSprite, colour: Color, size: Vec2| {
        commands.spawn((
            kind,
            Sprite::from_color(colour, size),
            Transform::from_xyz(0.0, 0.0, z),
            Visibility::Hidden,
            layers.clone(),
        ));
    };
    rect(
        CueSprite::Glint,
        Color::WHITE,
        Vec2::splat(GLINT_SIZE_PX as f32),
    );
    rect(CueSprite::Streak, Color::WHITE, Vec2::ONE);
    for i in 0..FLICK_PIXELS {
        rect(CueSprite::Flick(i), Color::WHITE, Vec2::ONE);
    }
    let mut body = |kind: CueSprite, dz: f32| {
        commands.spawn((
            kind,
            Sprite::from_atlas_image(
                texture.clone(),
                TextureAtlas {
                    layout: layout.clone(),
                    index: 0,
                },
            ),
            Transform::from_xyz(0.0, 0.0, z + dz),
            Visibility::Hidden,
            layers.clone(),
        ));
    };
    body(CueSprite::Arc, 0.0);
    body(CueSprite::After(0), -0.001);
    body(CueSprite::After(1), -0.002);
    commands.insert_resource(DonutCueRes {
        cues: DonutCues::default(),
        texture,
        atlas: None,
        unreadable: false,
        history: VecDeque::new(),
        last_tick: None,
    });
}

/// Read the tail tips and the highlight colour from the loaded atlas, once.
fn read_atlas(state: &mut DonutCueRes, actors: &Actors, images: &Assets<Image>) {
    if state.atlas.is_some() || state.unreadable {
        return;
    }
    let Some(image) = images.get(&state.texture) else {
        return; // still loading
    };
    let spec = actors.donut.spec;
    let frames = actors.donut.clips.frame_count;
    match image.data.as_deref() {
        Some(data) if data.len() == (image.width() * image.height() * 4) as usize => {
            state.atlas = Some(analyze_atlas(
                data,
                image.width(),
                spec.width as u32,
                spec.height as u32,
                frames,
            ));
        }
        _ => state.unreadable = true,
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
    let Some((center, flip, frame, tint, shown, z)) = base
        .iter()
        .find(|(actor, ..)| !actor.is_hero)
        .map(|(_, transform, sprite, visibility)| {
            (
                transform.translation.truncate(),
                sprite.flip_x,
                sprite.texture_atlas.as_ref().map_or(0, |a| a.index),
                sprite.color,
                *visibility != Visibility::Hidden,
                transform.translation.z,
            )
        })
    else {
        hide_all(&mut cues);
        return;
    };

    // Her drawn centre on each new tick, for the afterimages.
    if state.last_tick != Some(tick) {
        if state.last_tick.is_some_and(|last| tick < last) {
            state.history.clear();
        }
        state.last_tick = Some(tick);
        state.history.push_back((center.x, center.y));
        while state.history.len() > HISTORY {
            state.history.pop_front();
        }
    }

    let spec = actors.donut.spec;
    let (w, h) = (spec.width as f32, spec.height as f32);
    let evade = state.cues.evade(tick);
    let arc_on = evade.is_some() && shown;
    let jump_rise = actors.donut.clips.clip(Tag::JumpRise).first;
    let glint = state.cues.glint_on(tick) && shown && evade.is_none();
    let flick = state.cues.flick(tick).filter(|_| shown && evade.is_none());
    let tip = state
        .atlas
        .as_ref()
        .and_then(|a| a.tips.get(frame).and_then(Option::as_ref));
    let highlight = state.atlas.as_ref().map(|a| a.highlight);

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
        // Where the cue is drawn (`None` hides it) and its depth above her sprite.
        let (at, dz): (Option<(f32, f32)>, f32) = match *kind {
            CueSprite::Glint => {
                // A 2 x 2 glint on the tip pixel and the pixel inward and below.
                let at = match (tip.filter(|_| glint), highlight) {
                    (Some(t), Some([r, g, b])) => {
                        let inward = if flip { -1.0 } else { 1.0 };
                        let (x, y) = to_world(t.x as f32 + 0.5, t.y as f32 + 0.5);
                        sprite.color = Color::srgb_u8(r, g, b);
                        Some((x + inward * 0.5, y - 0.5))
                    }
                    _ => None,
                };
                (at, 0.02)
            }
            CueSprite::Flick(i) => {
                // The tail-tip pixels again, one pixel higher while shifted out.
                let at = tip
                    .filter(|_| flick == Some(true))
                    .and_then(|t| t.cluster.get(i))
                    .map(|&(px, py, c)| {
                        let (x, y) = to_world(px as f32 + 0.5, py as f32 + 0.5);
                        sprite.color = Color::srgba_u8(c[0], c[1], c[2], c[3]);
                        (x, y + 1.0)
                    });
                (at, 0.015)
            }
            CueSprite::Arc => {
                let at = evade.filter(|_| arc_on).map(|view| {
                    if let Some(atlas) = sprite.texture_atlas.as_mut() {
                        atlas.index = jump_rise;
                    }
                    sprite.flip_x = flip;
                    sprite.color = tint;
                    (center.x, center.y + view.lift_px as f32)
                });
                (at, 0.005)
            }
            CueSprite::After(n) => {
                let at = evade.filter(|_| arc_on).and_then(|view| {
                    let (lag, alpha) = *view.afterimages.get(n)?;
                    // Where she stood `lag` ticks ago (the oldest kept, early on).
                    let back = (lag as usize).min(state.history.len().saturating_sub(1));
                    let (x, y) = state.history[state.history.len() - 1 - back];
                    if let Some(atlas) = sprite.texture_atlas.as_mut() {
                        atlas.index = jump_rise;
                    }
                    sprite.flip_x = flip;
                    sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha);
                    Some((x, y + arc_lift_px(view.age.saturating_sub(lag)) as f32))
                });
                (at, 0.004 - 0.001 * n as f32)
            }
            CueSprite::Streak => {
                let at = evade.and_then(|view| {
                    let side = view.streak?;
                    sprite.custom_size = Some(Vec2::new(
                        (2 * STREAK_HALF_LENGTH_PX) as f32,
                        STREAK_THICKNESS_PX as f32,
                    ));
                    sprite.color = Color::WHITE;
                    // Across her body, on the side the attack came from.
                    Some((center.x + f32::from(side) * 6.0, center.y + 2.0))
                });
                (at, 0.03)
            }
        };
        match at {
            Some((x, y)) => {
                transform.translation = Vec3::new(x, y, z + dz);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    // The arc draws her: the locomotion sprite steps aside while it runs.
    if arc_on {
        for (actor, _, _, mut visibility) in &mut base {
            if !actor.is_hero {
                *visibility = Visibility::Hidden;
            }
        }
    }
}
