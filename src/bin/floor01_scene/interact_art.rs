//! GUILD-VERB-01: the Bevy half of the hero interact animations (ANIM-INTERACT-01). Carl's hand,
//! floor and push verbs and Donut's paw and nudge are atlas sprites drawn over the hero's
//! locomotion sprite while the Interact verb runs: the frame is a pure function of the verb clock
//! (`dungeon_floorbreak::interact::Verb::frame_at`), read from the combat state.
//!
//! **Presentation only**: the sync system reads the combat state and the scene through shared
//! references and writes only its own sprites and the visibility of the hero sprites it replaces,
//! so nothing here reaches the simulation, the replay or the state hash.

use super::*;
use dungeon_floorbreak::interact::Verb;

/// One hero's interact sprite.
#[derive(Component, Clone, Copy)]
pub(super) struct InteractHeroSprite(HeroId);

/// The two loaded interact sheets.
#[derive(Resource)]
pub(super) struct InteractArt {
    carl: SpriteClips,
    donut: SpriteClips,
}

impl InteractArt {
    fn clips(&self, hero: HeroId) -> &SpriteClips {
        match hero {
            HeroId::Carl => &self.carl,
            HeroId::Donut => &self.donut,
        }
    }
}

/// Load both sheets and spawn the hidden hero sprites. A missing sheet leaves the verb undrawn
/// (one console line) and never fails the scene.
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    handles: &mut Vec<(String, Handle<Image>)>,
    source: FrameSource,
    z: f32,
    layers: RenderLayers,
) {
    let root = std::env::current_dir().unwrap_or_default();
    let (carl, donut) = match (
        SpriteClips::load_carl_interact(&root, source),
        SpriteClips::load_donut_interact(&root, source),
    ) {
        (Ok(carl), Ok(donut)) => (carl, donut),
        (Err(e), _) | (_, Err(e)) => {
            println!("INTERACT_ART not drawn: {e} (run bash scripts/export_assets.sh)");
            return;
        }
    };
    for (hero, clips, first) in [
        (HeroId::Carl, &carl, Verb::Hand),
        (HeroId::Donut, &donut, Verb::Paw),
    ] {
        let texture: Handle<Image> = asset_server.load(clips.atlas_asset.clone());
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(clips.spec.width as u32, clips.spec.height as u32),
            clips.frame_count as u32,
            1,
            None,
            None,
        ));
        commands.spawn((
            InteractHeroSprite(hero),
            Sprite::from_atlas_image(
                texture.clone(),
                TextureAtlas {
                    layout,
                    index: clips.tag(first.tag()).0,
                },
            ),
            Transform::from_xyz(0.0, 0.0, z),
            Visibility::Hidden,
            layers.clone(),
        ));
        handles.push((clips.atlas_asset.clone(), texture));
    }
    commands.insert_resource(InteractArt { carl, donut });
    println!(
        "INTERACT_ART drawn: Carl hand/floor/push and Donut paw/nudge from the interact atlases \
         (prototype art, owner art review pending)"
    );
}

/// Draw the verb frame of each hero in a verb and hide the sprite it replaces.
#[allow(clippy::type_complexity)]
pub(super) fn sync_interact_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<InteractArt>>,
    mut sprites: Query<
        (
            &InteractHeroSprite,
            &mut Transform,
            &mut Sprite,
            &mut Visibility,
        ),
        Without<ActorVisual>,
    >,
    mut actors: Query<(&ActorVisual, &mut Visibility), Without<InteractHeroSprite>>,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let world = &sim.0.world;
    let mut replaced = [false; 2];
    for (marker, mut transform, mut sprite, mut visibility) in &mut sprites {
        let hero = marker.0;
        let Some((verb, frame)) = state.interact_pose(hero) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let clips = art.clips(hero);
        let (x, foot_y, facing) = match hero {
            HeroId::Carl => (
                world.hero.x,
                world.hero.y - HERO_HALF_HEIGHT,
                world.hero.facing,
            ),
            HeroId::Donut => (world.donut.x, world.donut.foot_y, world.donut.facing),
        };
        let flip = facing < 0;
        let (cx, cy) = actor_center(clips.spec, x, foot_y, flip);
        transform.translation.x = cx;
        transform.translation.y = cy;
        sprite.flip_x = flip;
        sprite.color = hero_cue(&sim.0, Some(state), hero);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = clips.frame_in(verb.tag(), frame);
        }
        *visibility = Visibility::Inherited;
        replaced[hero.index()] = true;
    }
    for (actor, mut visibility) in &mut actors {
        let hero = if actor.is_hero {
            HeroId::Carl
        } else {
            HeroId::Donut
        };
        if replaced[hero.index()] {
            *visibility = Visibility::Hidden;
        }
    }
}
