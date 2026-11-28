//! FA-Q23 (HRD-P) and HOARD-02: the Bevy half of the Hoarder room's presentation. The pure half is
//! `dungeon_floorbreak::hoarder_art`; this file only draws what it returns: the HOARD-02 atlas
//! frames of her, the `floor01_hoarder_*` room kit (door, gate, bank A, the J-B post and stack, the
//! lair mound and the dressing heaps), a few cosmetic dust motes and the J-B footprint's floor
//! shadow while the stack warns and falls (HRD-C).
//! **Presentation only**: every system reads the combat state through a shared reference and
//! writes only its own sprites, so nothing here reaches the simulation, the replay or the
//! state hash (the pure half proves it with a state-hash test).

use super::*;
use dungeon_floorbreak::encounter::Encounter;
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;
use dungeon_floorbreak::hoarder_art::{
    ATLAS, FRAME_SIZE, FRAMES, HoarderLook, HoarderMemo, Layer, MAX_MOTES, MAX_PROPS,
    POST_BURST_ABOVE_FOOT, POST_FLASH_TICKS, Prop, Shade, draw, lead_x, post_burst_frame,
    post_hit_age, stack_released, stir_level_held,
};
use dungeon_floorbreak::party::HeroId;

/// Her pooled sprite.
#[derive(Component, Clone, Copy)]
pub(super) struct HoarderSprite;

/// One pooled room-kit sprite (door, gate, bank, post, stack, mound, heaps).
#[derive(Component, Clone, Copy)]
pub(super) struct HoarderProp(usize);

/// One pooled dust mote.
#[derive(Component, Clone, Copy)]
pub(super) struct HoarderMote(usize);

/// SMALL-02 (HC-OQ3): the ENV-02 splinter burst on a counted hit on the J-B post (one sprite).
#[derive(Component, Clone, Copy)]
pub(super) struct HoarderPostBurst;

/// The J-B footprint's floor shadow (one sprite, scaled to the band).
#[derive(Component, Clone, Copy)]
pub(super) struct HoarderShadow;

/// R7: her sprite in the Reveal, about one ramp step darker (the mound's shadow value).
const REVEAL_SHADOW_TINT: Color = Color::srgb(0.62, 0.6, 0.68);
/// Dust: the `floor` ramp's rag fibre, lit.
const DUST_COLOUR: Color = Color::srgba(0.95, 0.82, 0.62, 0.95);
const DUST_PX: f32 = 3.0;
/// HRD-C: the stack's footprint warning, a translucent dark band on the floor.
const SHADOW_COLOUR: Color = Color::srgba(0.0, 0.0, 0.0, 0.44);

/// One prop's atlas: its image and its frame layout.
struct PropAtlas {
    prop: Prop,
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

#[derive(Resource)]
pub(super) struct HoarderArt {
    encounter: Encounter,
    props: Vec<PropAtlas>,
    hoarder_z: f32,
    back_z: f32,
    prop_z: f32,
    front_z: f32,
    mote_z: f32,
    /// SMALL-02: the splinter burst's z, in front of the actors like the partition's.
    burst_z: f32,
}

fn atlas_present(path: &str) -> bool {
    std::env::current_dir().is_ok_and(|root| root.join(path).is_file())
}

fn asset_path(path: &str) -> String {
    path.strip_prefix(ATLAS_ASSET_PREFIX)
        .unwrap_or(path)
        .to_owned()
}

/// Spawn the pools of the Hoarder room. A missing atlas (`bash scripts/export_assets.sh` was
/// not run) skips that sprite with a one-line notice.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    encounter: Encounter,
    layers: RenderLayers,
    kit: &EnvKit,
    handles: &mut Vec<(String, Handle<Image>)>,
    entities: &mut usize,
) {
    println!("HRD_P_ART HOARD-02 presentation (boss atlas, room kit, no stand-ins)");
    let actor_z = kit.actor_z;
    let mut props = Vec::new();
    for prop in Prop::ALL {
        if !atlas_present(prop.atlas()) {
            println!(
                "HRD_P_ART missing {} (run scripts/export_assets.sh): that prop is skipped",
                prop.atlas()
            );
            continue;
        }
        let asset = asset_path(prop.atlas());
        let image: Handle<Image> = asset_server.load(asset.clone());
        handles.push((asset, image.clone()));
        let (w, h) = prop.frame_size();
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(w, h),
            prop.frames(),
            1,
            None,
            None,
        ));
        props.push(PropAtlas {
            prop,
            image,
            layout,
        });
    }
    commands.insert_resource(HoarderArt {
        encounter,
        props,
        // Among the goblins' band, behind Carl and Donut (the slime's convention).
        hoarder_z: actor_z + GOBLIN_Z_OFFSET - 32.0 * GOBLIN_Z_STEP,
        back_z: actor_z - 0.5,
        prop_z: actor_z - 0.045,
        // In front of every actor (Carl is +0.1, a glob +0.2): the mound's front lip.
        front_z: actor_z + 0.3,
        mote_z: actor_z + GOBLIN_Z_OFFSET - 32.0 * GOBLIN_Z_STEP + 0.0005,
        burst_z: actor_z + 0.305,
    });
    // SMALL-02 (HC-OQ3): the ENV-02 splinter burst of a counted post hit (a missing atlas draws none).
    {
        use dungeon_floorbreak::heat_kit::{BURST_ATLAS, BURST_FRAME_PX, BURST_FRAMES};
        if atlas_present(BURST_ATLAS) {
            let asset = asset_path(BURST_ATLAS);
            let image: Handle<Image> = asset_server.load(asset.clone());
            handles.push((asset, image.clone()));
            let layout = layouts.add(TextureAtlasLayout::from_grid(
                UVec2::splat(BURST_FRAME_PX),
                BURST_FRAMES as u32,
                1,
                None,
                None,
            ));
            commands.spawn((
                HoarderPostBurst,
                Sprite::from_atlas_image(image, TextureAtlas { layout, index: 0 }),
                Transform::from_xyz(0.0, 0.0, actor_z + 0.305),
                Visibility::Hidden,
                layers.clone(),
            ));
            *entities += 1;
        }
    }
    if atlas_present(ATLAS) {
        let asset = asset_path(ATLAS);
        let texture: Handle<Image> = asset_server.load(asset.clone());
        handles.push((asset, texture.clone()));
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(FRAME_SIZE.0, FRAME_SIZE.1),
            FRAMES,
            1,
            None,
            None,
        ));
        commands.spawn((
            HoarderSprite,
            Sprite::from_atlas_image(texture, TextureAtlas { layout, index: 0 }),
            Transform::from_xyz(0.0, 0.0, actor_z),
            Visibility::Hidden,
            layers.clone(),
        ));
        *entities += 1;
    } else {
        println!("HRD_P_ART missing {ATLAS} (run scripts/export_assets.sh): her sprite is skipped");
    }
    for index in 0..MAX_PROPS {
        commands.spawn((
            HoarderProp(index),
            Sprite::default(),
            Transform::from_xyz(0.0, 0.0, actor_z),
            Visibility::Hidden,
            layers.clone(),
        ));
        *entities += 1;
    }
    commands.spawn((
        HoarderShadow,
        Sprite::from_color(SHADOW_COLOUR, Vec2::ONE),
        Transform::from_xyz(0.0, 0.0, actor_z),
        Visibility::Hidden,
        layers.clone(),
    ));
    *entities += 1;
    for index in 0..MAX_MOTES {
        commands.spawn((
            HoarderMote(index),
            Sprite::from_color(DUST_COLOUR, Vec2::splat(DUST_PX)),
            Transform::from_xyz(0.0, 0.0, actor_z),
            Visibility::Hidden,
            layers.clone(),
        ));
        *entities += 1;
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn sync_hoarder_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<HoarderArt>>,
    mut memo: Local<HoarderMemo>,
    mut hoarder_sprite: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (
            With<HoarderSprite>,
            Without<HoarderProp>,
            Without<HoarderMote>,
            Without<HoarderShadow>,
            Without<HoarderPostBurst>,
        ),
    >,
    mut prop_sprites: Query<
        (&HoarderProp, &mut Transform, &mut Sprite, &mut Visibility),
        (
            Without<HoarderSprite>,
            Without<HoarderMote>,
            Without<HoarderShadow>,
            Without<HoarderPostBurst>,
        ),
    >,
    mut mote_sprites: Query<
        (&HoarderMote, &mut Transform, &mut Visibility),
        (
            Without<HoarderSprite>,
            Without<HoarderProp>,
            Without<HoarderShadow>,
            Without<HoarderPostBurst>,
        ),
    >,
    mut shadow_sprite: Query<
        (&mut Transform, &mut Visibility),
        (
            With<HoarderShadow>,
            Without<HoarderSprite>,
            Without<HoarderProp>,
            Without<HoarderMote>,
            Without<HoarderPostBurst>,
        ),
    >,
    mut burst_sprite: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (
            With<HoarderPostBurst>,
            Without<HoarderSprite>,
            Without<HoarderProp>,
            Without<HoarderMote>,
            Without<HoarderShadow>,
        ),
    >,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let world = &sim.0.world;
    let tick = world.tick;
    // Render-local looks (the hit flash, the age of her defeat, the age of her emergence, the age of
    // the J-B stack's warning), kept by the pure half.
    let look = dungeon_floorbreak::combat_scene::pile_boss::of(state).map_or_else(
        HoarderLook::default,
        |enemy| {
            let mut look = memo.observe(enemy.boss().hp(), enemy.boss().alive(), tick);
            let hidden = matches!(
                enemy.boss().mode(),
                dungeon_floorbreak::pile_boss::PileMode::Dormant
                    | dungeon_floorbreak::pile_boss::PileMode::Reveal
            );
            look.out_age = memo.observe_out(hidden, tick);
            look.released_age = memo.observe_released(stack_released(state, &art.encounter), tick);
            // HRD-CUES-01: the lead hero is Carl, plus Donut while the party is live (the sim's gate view).
            let lead = lead_x(
                world.body(HeroId::Carl).x,
                world.body(HeroId::Donut).x,
                sim.0.party_live(),
            );
            let level = stir_level_held(enemy.boss(), Some(lead), memo.stir_held());
            look.stir = memo.observe_stir(level, tick);
            look
        },
    );
    let Some(drawn) = draw(state, &art.encounter, world.layout_height(), tick, look) else {
        return;
    };
    for (mut transform, mut sprite, mut visibility) in &mut hoarder_sprite {
        match drawn.hoarder {
            Some(d) => {
                transform.translation = Vec3::new(d.centre.0, d.centre.1, art.hoarder_z);
                sprite.flip_x = d.flip_x;
                sprite.color = if d.flash {
                    FLASH_TINT
                } else if d.shade == Shade::Shadow {
                    REVEAL_SHADOW_TINT
                } else {
                    Color::WHITE
                };
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = usize::from(d.frame);
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    // SMALL-02 (HC-OQ3): the age of a counted hit on the post (flash and splinter burst).
    let post_hit = post_hit_age(state, &art.encounter, tick);
    let mut post_at = None;
    for (slot, mut transform, mut sprite, mut visibility) in &mut prop_sprites {
        let drawn_prop = drawn.props.get(slot.0);
        let atlas = drawn_prop.and_then(|p| art.props.iter().find(|a| a.prop == p.prop));
        match (drawn_prop, atlas) {
            (Some(p), Some(a)) => {
                let (w, h) = p.prop.frame_size();
                // The sprite is centred; its bottom-left corner sits on `(x, y)`. A bank in its
                // `disturbed` frames shudders 1 px each way (presentation).
                let shudder = if p.prop == Prop::Bank && matches!(p.frame, 1 | 2) {
                    if p.frame == 1 { 1.0 } else { -1.0 }
                } else {
                    0.0
                };
                transform.translation = Vec3::new(
                    p.x as f32 + w as f32 / 2.0 + shudder,
                    p.y as f32 + h as f32 / 2.0,
                    match p.layer {
                        Layer::Back => art.back_z,
                        Layer::Prop => art.prop_z,
                        Layer::Front => art.front_z,
                    },
                );
                if p.prop == Prop::Post {
                    post_at = Some((p.x as f32 + w as f32 / 2.0, p.y as f32));
                    sprite.color = if post_hit.is_some_and(|age| age < POST_FLASH_TICKS) {
                        FLASH_TINT
                    } else {
                        Color::WHITE
                    };
                }
                sprite.image = a.image.clone();
                sprite.texture_atlas = Some(TextureAtlas {
                    layout: a.layout.clone(),
                    index: usize::from(p.frame),
                });
                *visibility = Visibility::Inherited;
            }
            _ => *visibility = Visibility::Hidden,
        }
    }
    for (mut transform, mut sprite, mut visibility) in &mut burst_sprite {
        match (post_at, post_hit.and_then(post_burst_frame)) {
            (Some((cx, foot)), Some(frame)) => {
                transform.translation =
                    Vec3::new(cx, foot + POST_BURST_ABOVE_FOOT as f32, art.burst_z);
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = frame;
                }
                *visibility = Visibility::Inherited;
            }
            _ => *visibility = Visibility::Hidden,
        }
    }
    for (mut transform, mut visibility) in &mut shadow_sprite {
        match drawn.shadow {
            Some(s) => {
                *transform = Transform::from_xyz(
                    s.x as f32 + s.w as f32 / 2.0,
                    s.y as f32 + s.h as f32 / 2.0,
                    art.back_z + 0.001,
                )
                .with_scale(Vec3::new(s.w as f32, s.h as f32, 1.0));
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
    for (slot, mut transform, mut visibility) in &mut mote_sprites {
        match drawn.motes.get(slot.0) {
            Some(m) => {
                transform.translation = Vec3::new(m.x.round(), m.y.round(), art.mote_z);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
