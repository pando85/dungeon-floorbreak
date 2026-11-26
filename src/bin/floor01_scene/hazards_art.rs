//! HAZ-B: the Bevy half of the hazard room's stand-in presentation. The pure half
//! is `dungeon_floorbreak::hazards_art`; this file only draws what it returns:
//! the SLIME-01 and ROT-01 atlas frames (the committed, unapproved candidates), the
//! acid strip, the ring at the Rot blast radius and the drip a spent sticker
//! leaves (code-drawn, labelled stand-ins), and, under `--debug-combat`, the
//! slimes' and stickers' boxes. The acid strip and the ring dots are skipped
//! where the F01 danger FX art draws them instead (`danger_fx_art`), and the drip
//! where the AREA-HAZARDS-01B drip art does (`hazard_decals`). **Presentation only**: every system reads the
//! combat state through a shared reference and writes only its own sprites, so
//! nothing here reaches the simulation, the replay or the state hash (the pure
//! half proves it with a state-hash test).

use super::*;
use dungeon_floorbreak::combat_scene::slime::SlimeEnemy;
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;
use dungeon_floorbreak::hazards_art::{
    ROT_ATLAS, ROT_FRAME_SIZE, ROT_FRAMES, SLIME_ATLAS, SLIME_FRAME_SIZE, SLIME_FRAMES, SlimeLook,
    Tone, max_pieces, pieces, slimes, stickers,
};
use dungeon_floorbreak::slime;

/// One pooled slime sprite, for roster index `.0`.
#[derive(Component, Clone, Copy)]
pub(super) struct SlimeSprite(usize);

/// One pooled sticker sprite, for the `.0`-th sticker of the room.
#[derive(Component, Clone, Copy)]
pub(super) struct StickerSprite(usize);

/// One pooled code-drawn piece (acid strip, ring dot, drip).
#[derive(Component, Clone, Copy)]
pub(super) struct HazardPiece(usize);

/// `--debug-combat`: a slime's hurtbox, contact box and lunge box, a sticker's
/// shell.
#[derive(Component, Clone, Copy)]
pub(super) enum HazardDebug {
    SlimeHurt(usize),
    SlimeContact(usize),
    SlimeLunge(usize),
    Shell(usize),
}

#[derive(Resource)]
pub(super) struct HazardArt {
    slime_z: f32,
    sticker_z: f32,
    piece_z: f32,
    tell_z: f32,
}

/// What the presenter remembers of a slime between frames (render-local).
#[derive(Default, Clone, Copy)]
pub(super) struct SlimeMemo {
    x: Option<f32>,
    last_move: Option<u64>,
    defeated_since: Option<u64>,
}

/// A slime stops "creeping" this many ticks after its last change of x.
const CREEP_HOLD_TICKS: u64 = 6;

fn atlas_present(path: &str) -> bool {
    std::env::current_dir().is_ok_and(|root| root.join(path).is_file())
}

/// Spawn the pools a hazard room needs. A room without slimes, stickers or acid
/// spawns nothing. A missing atlas (`bash scripts/export_assets.sh` was not run)
/// skips that sprite pool with a one-line notice; the code-drawn pieces still draw.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    state: &CombatState,
    layers: RenderLayers,
    debug_layers: Option<RenderLayers>,
    kit: &EnvKit,
    handles: &mut Vec<(String, Handle<Image>)>,
    entities: &mut usize,
) {
    let n_slimes = slimes(state, 0, |_, _, _| SlimeLook::default()).len();
    let (n_stickers, n_pieces) = state
        .env()
        .map_or((0, 0), |e| (e.sticker_count(), max_pieces(e)));
    if n_slimes + n_stickers + n_pieces == 0 {
        return;
    }
    println!(
        "HAZ_B_ART slimes={n_slimes} stickers={n_stickers} pieces={n_pieces} (stand-in presentation)"
    );
    let actor_z = kit.actor_z;
    commands.insert_resource(HazardArt {
        // Among the goblins' band, behind Carl and Donut (the llama's convention).
        slime_z: actor_z + GOBLIN_Z_OFFSET - 32.0 * GOBLIN_Z_STEP,
        // On the slab or the wall, in front of the kit's props, behind the actors.
        sticker_z: actor_z - 0.04,
        piece_z: actor_z - 0.045,
        // The ring is a warning: above everything, never hidden.
        tell_z: kit.foreground_z + 0.1,
    });
    let mut sheet = |path: &str, frame: (u32, u32), frames: u32| {
        if !atlas_present(path) {
            println!(
                "HAZ_B_ART missing {path} (run scripts/export_assets.sh): that sprite pool is skipped"
            );
            return None;
        }
        // The asset server's root is the atlas folder (`ATLAS_ASSET_PREFIX`).
        let asset = path
            .strip_prefix(ATLAS_ASSET_PREFIX)
            .unwrap_or(path)
            .to_owned();
        let texture: Handle<Image> = asset_server.load(asset.clone());
        handles.push((asset, texture.clone()));
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(frame.0, frame.1),
            frames,
            1,
            None,
            None,
        ));
        Some((texture, layout))
    };
    if n_slimes > 0
        && let Some((texture, layout)) = sheet(SLIME_ATLAS, SLIME_FRAME_SIZE, SLIME_FRAMES)
    {
        for index in 0..n_slimes {
            commands.spawn((
                SlimeSprite(index),
                Sprite::from_atlas_image(
                    texture.clone(),
                    TextureAtlas {
                        layout: layout.clone(),
                        index: 0,
                    },
                ),
                Transform::from_xyz(0.0, 0.0, actor_z),
                Visibility::Hidden,
                layers.clone(),
            ));
            *entities += 1;
        }
    }
    if n_stickers > 0
        && let Some((texture, layout)) = sheet(ROT_ATLAS, ROT_FRAME_SIZE, ROT_FRAMES)
    {
        for index in 0..n_stickers {
            commands.spawn((
                StickerSprite(index),
                Sprite::from_atlas_image(
                    texture.clone(),
                    TextureAtlas {
                        layout: layout.clone(),
                        index: 0,
                    },
                ),
                Transform::from_xyz(0.0, 0.0, actor_z),
                Visibility::Hidden,
                layers.clone(),
            ));
            *entities += 1;
        }
    }
    for index in 0..n_pieces {
        commands.spawn((
            HazardPiece(index),
            Sprite::from_color(Color::WHITE, Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, actor_z),
            Visibility::Hidden,
            layers.clone(),
        ));
        *entities += 1;
    }
    if let Some(debug_layers) = debug_layers {
        let mut overlay = |kind: HazardDebug, colour: Color| {
            commands.spawn((
                kind,
                Sprite::from_color(colour, Vec2::ONE),
                Transform::from_xyz(0.0, 0.0, kit.foreground_z + 1.0),
                Visibility::Hidden,
                debug_layers.clone(),
            ));
            *entities += 1;
        };
        for index in 0..n_slimes {
            overlay(
                HazardDebug::SlimeHurt(index),
                Color::srgba(0.2, 1.0, 0.3, 0.3),
            );
            overlay(
                HazardDebug::SlimeContact(index),
                Color::srgba(0.8, 1.0, 0.1, 0.3),
            );
            overlay(
                HazardDebug::SlimeLunge(index),
                Color::srgba(1.0, 0.1, 0.1, 0.5),
            );
        }
        for index in 0..n_stickers {
            overlay(HazardDebug::Shell(index), Color::srgba(1.0, 0.4, 0.9, 0.4));
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn sync_hazard_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<HazardArt>>,
    danger: Option<Res<super::danger_fx_art::DangerFxArt>>,
    decals: Option<Res<super::hazard_decals::HazardDecalArt>>,
    mut memo: Local<Vec<SlimeMemo>>,
    mut slime_sprites: Query<
        (&SlimeSprite, &mut Transform, &mut Sprite, &mut Visibility),
        (Without<StickerSprite>, Without<HazardPiece>),
    >,
    mut sticker_sprites: Query<
        (&StickerSprite, &mut Transform, &mut Sprite, &mut Visibility),
        (Without<SlimeSprite>, Without<HazardPiece>),
    >,
    mut piece_sprites: Query<
        (&HazardPiece, &mut Transform, &mut Sprite, &mut Visibility),
        (Without<SlimeSprite>, Without<StickerSprite>),
    >,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let world = &sim.0.world;
    let tick = world.tick;
    let layout_h = world.layout_height();
    // Slimes.
    let draws = slimes(state, tick, |index, x, defeated| {
        if memo.len() <= index {
            memo.resize(index + 1, SlimeMemo::default());
        }
        let m = &mut memo[index];
        if m.x.is_some_and(|prev| (prev - x).abs() > f32::EPSILON) {
            m.last_move = Some(tick);
        }
        m.x = Some(x);
        if defeated {
            m.defeated_since.get_or_insert(tick);
        } else {
            m.defeated_since = None;
        }
        SlimeLook {
            moving: m
                .last_move
                .is_some_and(|t| tick.saturating_sub(t) <= CREEP_HOLD_TICKS),
            defeated_age: m.defeated_since.map(|since| tick.saturating_sub(since)),
        }
    });
    for (sprite_id, mut transform, mut sprite, mut visibility) in &mut slime_sprites {
        match draws.iter().find(|d| d.index == sprite_id.0) {
            Some(d) => {
                transform.translation = Vec3::new(d.centre.0, d.centre.1, art.slime_z);
                sprite.flip_x = d.flip_x;
                sprite.color = if d.flash { FLASH_TINT } else { Color::WHITE };
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = usize::from(d.frame);
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    // Stickers.
    let draws = stickers(state, tick);
    for (sprite_id, mut transform, mut sprite, mut visibility) in &mut sticker_sprites {
        match draws.get(sprite_id.0) {
            Some(d) => {
                transform.translation =
                    Vec3::new(d.centre.0.round(), d.centre.1.round(), art.sticker_z);
                transform.rotation = Quat::from_rotation_z(
                    -std::f32::consts::FRAC_PI_2 * f32::from(d.quarter_turns_cw),
                );
                sprite.flip_x = d.flip_x;
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = usize::from(d.frame);
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    // The code-drawn pieces. The F01 danger FX art replaces the acid strip and lip
    // and the ring dots where its atlas loaded (and the ring's radius has a canvas).
    let acid_art = danger.as_ref().is_some_and(|d| d.draws_acid());
    let ring_art = danger.as_ref().is_some_and(|d| d.draws_rot_ring());
    // AREA-HAZARDS-01B: the spent drip art replaces the code-drawn drip (the stand-in and its
    // label remain only where the drip atlas is missing).
    let drip_art = decals.as_ref().is_some_and(|d| d.draws_drip());
    let drawn: Vec<_> = state
        .env()
        .map(|env| pieces(env, layout_h, tick))
        .unwrap_or_default()
        .into_iter()
        .filter(|p| match p.tone {
            Tone::Acid | Tone::AcidLip => !acid_art,
            Tone::Ring => !ring_art,
            Tone::Drip => !drip_art,
        })
        .collect();
    for (piece_id, mut transform, mut sprite, mut visibility) in &mut piece_sprites {
        match drawn.get(piece_id.0) {
            Some(p) => {
                let [r, g, b, a] = p.tone.rgba();
                sprite.color = Color::srgba_u8(r, g, b, a);
                sprite.custom_size = Some(Vec2::new(p.w as f32, p.h as f32));
                transform.translation.x = p.x as f32 + p.w as f32 / 2.0;
                transform.translation.y = p.y as f32 + p.h as f32 / 2.0;
                transform.translation.z = match p.tone {
                    Tone::Ring => art.tell_z,
                    _ => art.piece_z,
                };
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// `--debug-combat`: the boxes of the slimes and the sticker shells.
pub(super) fn sync_hazard_debug(
    combat: Res<Combat>,
    mut boxes: Query<(&HazardDebug, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let Some(state) = combat.0.as_ref() else {
        return;
    };
    let t = slime::active();
    let shells: Vec<_> = state
        .env()
        .map(|e| {
            e.stickers()
                .map(|(_, s, _)| s.hurtbox(dungeon_floorbreak::rot_sticker::active()))
                .collect()
        })
        .unwrap_or_default();
    for (kind, mut transform, mut sprite, mut visibility) in &mut boxes {
        let rect = match *kind {
            HazardDebug::SlimeHurt(i)
            | HazardDebug::SlimeContact(i)
            | HazardDebug::SlimeLunge(i) => {
                let s = state
                    .slot_enemies()
                    .get(i)
                    .and_then(|e| e.as_any())
                    .and_then(|a| a.downcast_ref::<SlimeEnemy>())
                    .map(SlimeEnemy::slime);
                s.and_then(|s| match *kind {
                    HazardDebug::SlimeHurt(_) => s.hurtbox(t),
                    HazardDebug::SlimeContact(_) => s.contact_box(t),
                    _ => s.lunge_box(t),
                })
            }
            HazardDebug::Shell(i) => shells.get(i).copied().flatten(),
        };
        match rect {
            Some(r) => {
                sprite.custom_size = Some(Vec2::new(r.width, r.height));
                transform.translation.x = r.left + r.width / 2.0;
                transform.translation.y = r.bottom + r.height / 2.0;
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
