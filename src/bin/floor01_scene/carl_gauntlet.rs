//! CARL-GAUNTLET-01 (CP-Q02, CP-Q43): the Bevy half of Carl's reinforced fist.
//! The gameplay capability `fist_reinforced` (CP-D) is merged and grants on the
//! Juicer's defeat; this file draws what it looks like. Each of Carl's four body
//! sprites (the locomotion sprite, `CarlMelee`, `CarlThrowSprite` and
//! `CarlCf02Sprite`) gets one overlay sprite from the matching `*_gauntlet`
//! atlas (`generated/art/carl_book1_h52_<sheet>_gauntlet.png`, the same 48 x 56
//! cells and the same `TextureAtlasLayout::from_grid` parameters as the body
//! sheet). After the body sprites are synced, the overlay copies the body
//! sprite's visibility, atlas index, `flip_x`, colour and translation (z +
//! 0.01), so the same cell index and flip give the same pivot with no
//! per-frame anchor table. It is shown only while Carl's combat state holds
//! `fist_reinforced`; there is no fade, flash or glow (the `HEAVY FIST` notice is
//! the cue). Donut and every other actor never get one.
//!
//! **Presentation only and hash-neutral**: the system takes `Res<Combat>`
//! (never `ResMut`), writes only the overlay sprites, adds no `CombatEvent`, and
//! nothing here is part of a hash, a replay, a playtrace pin or a golden. The
//! art is `prototype_fixture_unapproved`; nothing here approves it. Spec:
//! `docs/02-art/carl-gauntlet-01-spec.md`.

use super::*;
use dungeon_floorbreak::story_unlock::FIST_REINFORCED;

/// z above the body sprite the overlay copies.
const ABOVE_BODY: f32 = 0.01;
/// The overlay atlas of a body atlas: `x.png` -> `x_gauntlet.png` (the export
/// writes both next to each other).
const ATLAS_SUFFIX: &str = "_gauntlet.png";

/// Which of Carl's body sprites an overlay follows.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CarlGauntlet {
    /// The locomotion sprite (`ActorVisual` with `is_hero`).
    Locomotion,
    /// The melee sprite (`CarlMelee`).
    Melee,
    /// The dynamite throw sprite (`CarlThrowSprite`).
    Throw,
    /// The CF02 acting sprite (`CarlCf02Sprite`).
    Cf02,
}

/// The overlay atlas asset that goes with a body atlas asset.
pub(super) fn overlay_asset(body_asset: &str) -> Option<String> {
    body_asset
        .strip_suffix(".png")
        .map(|stem| format!("{stem}{ATLAS_SUFFIX}"))
}

/// Whether Carl's combat state holds the reinforced fist (the only gate).
pub(super) fn held(combat: &Combat) -> bool {
    combat
        .0
        .as_ref()
        .is_some_and(|state| state.story_holds(FIST_REINFORCED))
}

/// What the overlay of one body sprite shows: the body's values, or hidden.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Shown {
    pub index: usize,
    pub flip_x: bool,
    pub color: Color,
    pub translation: Vec3,
}

/// The overlay's state for a body sprite: `None` (hidden) unless the fist is
/// held and the body sprite itself is drawn. Pure, so it is unit-tested.
pub(super) fn shown(
    held: bool,
    body_visible: bool,
    index: Option<usize>,
    flip_x: bool,
    color: Color,
    translation: Vec3,
) -> Option<Shown> {
    if !held || !body_visible {
        return None;
    }
    Some(Shown {
        index: index?,
        flip_x,
        color,
        translation: translation + Vec3::new(0.0, 0.0, ABOVE_BODY),
    })
}

/// Spawn the four overlay sprites (hidden) for the body sheets this run loads.
/// `bodies` is `(which, body atlas asset, cell w, cell h, frame count, body z)`
/// per sheet; an overlay atlas that is not exported is skipped (the body sprite
/// is then drawn bare). Returns how many sprites were spawned.
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    handles: &mut Vec<(String, Handle<Image>)>,
    layers: &RenderLayers,
    bodies: &[(CarlGauntlet, &str, UVec2, usize, f32)],
) -> usize {
    let mut spawned = 0;
    for (kind, body_asset, cell, count, z) in bodies {
        let Some(asset) = overlay_asset(body_asset) else {
            continue;
        };
        // The windowed scene reads exported atlases from the repository root.
        if !Path::new("generated/art").join(&asset).is_file() {
            eprintln!(
                "CARL_GAUNTLET: {asset} is not exported; Carl is drawn bare (run bash scripts/export_assets.sh)"
            );
            continue;
        }
        let texture: Handle<Image> = asset_server.load(asset.clone());
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            *cell,
            *count as u32,
            1,
            None,
            None,
        ));
        commands.spawn((
            *kind,
            Sprite::from_atlas_image(texture.clone(), TextureAtlas { layout, index: 0 }),
            Transform::from_xyz(0.0, 0.0, *z + ABOVE_BODY),
            Visibility::Hidden,
            layers.clone(),
        ));
        handles.push((asset, texture));
        spawned += 1;
    }
    spawned
}

/// Copy each visible body sprite of Carl onto its overlay, after the body sprites
/// were synced this frame. Reads the combat state and the body sprites only.
#[allow(clippy::type_complexity)]
pub(super) fn sync_carl_gauntlet(
    combat: Res<Combat>,
    locomotion: Query<(&ActorVisual, &Transform, &Sprite, &Visibility), Without<CarlGauntlet>>,
    melee: Query<(&Transform, &Sprite, &Visibility), (With<CarlMelee>, Without<CarlGauntlet>)>,
    throw: Query<
        (&Transform, &Sprite, &Visibility),
        (With<CarlThrowSprite>, Without<CarlGauntlet>),
    >,
    cf02: Query<(&Transform, &Sprite, &Visibility), (With<CarlCf02Sprite>, Without<CarlGauntlet>)>,
    mut overlays: Query<(&CarlGauntlet, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let held = held(&combat);
    for (kind, mut transform, mut sprite, mut visibility) in &mut overlays {
        let body = match kind {
            CarlGauntlet::Locomotion => locomotion
                .iter()
                .find(|(actor, ..)| actor.is_hero)
                .map(|(_, t, s, v)| (t, s, v)),
            CarlGauntlet::Melee => melee.iter().next(),
            CarlGauntlet::Throw => throw.iter().next(),
            CarlGauntlet::Cf02 => cf02.iter().next(),
        };
        let state = body.and_then(|(body_transform, body_sprite, body_visibility)| {
            shown(
                held,
                *body_visibility != Visibility::Hidden,
                body_sprite.texture_atlas.as_ref().map(|atlas| atlas.index),
                body_sprite.flip_x,
                body_sprite.color,
                body_transform.translation,
            )
        });
        match state {
            Some(state) => {
                transform.translation = state.translation;
                sprite.flip_x = state.flip_x;
                sprite.color = state.color;
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = state.index;
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_overlay_atlas_sits_next_to_its_body_atlas() {
        assert_eq!(
            overlay_asset("carl_book1_h52_melee.png").as_deref(),
            Some("carl_book1_h52_melee_gauntlet.png")
        );
        assert_eq!(overlay_asset("not_an_atlas.json"), None);
    }

    #[test]
    fn hidden_without_the_capability_and_with_a_hidden_body() {
        let at = Vec3::new(10.0, 20.0, 5.1);
        assert_eq!(shown(false, true, Some(3), false, Color::WHITE, at), None);
        assert_eq!(shown(true, false, Some(3), false, Color::WHITE, at), None);
        assert_eq!(shown(true, true, None, false, Color::WHITE, at), None);
    }

    #[test]
    fn shown_copies_index_flip_colour_and_translation_with_z_plus_one_hundredth() {
        let tint = Color::srgba(1.0, 1.0, 1.0, 0.5);
        let at = Vec3::new(10.0, 20.0, 5.1);
        let got = shown(true, true, Some(7), true, tint, at).expect("shown");
        assert_eq!(got.index, 7);
        assert!(got.flip_x);
        assert_eq!(got.color, tint);
        assert_eq!(got.translation, Vec3::new(10.0, 20.0, 5.1 + ABOVE_BODY));
    }

    use bevy::ecs::schedule::Schedule;

    /// A scene of the DEMO-01 hall (a run that reads the story flags),
    /// with or without `--carl-gauntlet`.
    fn scene(gauntlet: bool) -> (Args, SceneSim, SceneAnimators, CombatState, DeathPresenter) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let kit = demo::load_hall(root).unwrap();
        let mut list = vec![
            "floor01_scene".to_owned(),
            "--headless-floor01-smoke".to_owned(),
            "--demo".to_owned(),
        ];
        if gauntlet {
            list.push("--carl-gauntlet".to_owned());
        }
        let args = parse_args_with(&list, HudSettings::default()).unwrap();
        let runs_cf02 =
            dungeon_floorbreak::cf02::run_uses_cf02(args.cf02.rules, args.fixture.map(|f| f.name));
        let actors =
            Actors::load(root, FrameSource::GridSource, true, false, runs_cf02, false).unwrap();
        let mut hud = new_hud(&args, &kit, &actors);
        let (sim, anim, combat, death, _) =
            build_scene(&kit, &args, &actors, root, &mut hud).unwrap();
        (args, sim, anim, combat.unwrap(), death.unwrap())
    }

    fn body_sprite(index: usize, flip_x: bool) -> Sprite {
        let mut sprite = Sprite::from_atlas_image(
            Handle::default(),
            TextureAtlas {
                layout: Handle::default(),
                index,
            },
        );
        sprite.flip_x = flip_x;
        sprite
    }

    fn overlay_sprite() -> Sprite {
        body_sprite(0, false)
    }

    /// The four Carl body sprites (indices 3, 5, 7, 9; the melee one mirrored),
    /// Donut's locomotion sprite, and the four hidden overlays.
    fn populate(world: &mut World) {
        let at = |x: f32| Transform::from_xyz(x, 40.0, 5.1);
        world.spawn((
            ActorVisual { is_hero: true },
            body_sprite(3, false),
            at(10.0),
            Visibility::Inherited,
        ));
        world.spawn((
            ActorVisual { is_hero: false },
            body_sprite(1, false),
            at(99.0),
            Visibility::Inherited,
        ));
        world.spawn((
            CarlMelee,
            body_sprite(5, true),
            at(20.0),
            Visibility::Inherited,
        ));
        world.spawn((
            CarlThrowSprite,
            body_sprite(7, false),
            at(30.0),
            Visibility::Inherited,
        ));
        world.spawn((
            CarlCf02Sprite,
            body_sprite(9, true),
            at(40.0),
            Visibility::Inherited,
        ));
        for kind in [
            CarlGauntlet::Locomotion,
            CarlGauntlet::Melee,
            CarlGauntlet::Throw,
            CarlGauntlet::Cf02,
        ] {
            world.spawn((
                kind,
                overlay_sprite(),
                Transform::default(),
                Visibility::Hidden,
            ));
        }
    }

    fn overlay_of(world: &mut World, which: CarlGauntlet) -> (usize, bool, Vec3, Visibility) {
        let mut query = world.query::<(&CarlGauntlet, &Sprite, &Transform, &Visibility)>();
        let (_, sprite, transform, visibility) = query
            .iter(world)
            .find(|(kind, ..)| **kind == which)
            .expect("overlay");
        (
            sprite.texture_atlas.as_ref().unwrap().index,
            sprite.flip_x,
            transform.translation,
            *visibility,
        )
    }

    /// CARL-GAUNTLET-01 acceptance 5: hidden without the capability, shown with
    /// `--carl-gauntlet`; index, flip and translation are copied for each of the
    /// four body sprites, and Donut never gets an overlay.
    #[test]
    fn the_overlays_are_hidden_without_the_capability_and_copy_each_body_sprite_with_it() {
        for (gauntlet, shown_expected) in [(false, false), (true, true)] {
            let (_, _, _, combat, _) = scene(gauntlet);
            assert_eq!(
                combat.story_holds(FIST_REINFORCED),
                gauntlet,
                "--carl-gauntlet grants the fist at start"
            );
            let mut world = World::new();
            world.insert_resource(Combat(Some(combat)));
            populate(&mut world);
            let mut schedule = Schedule::default();
            schedule.add_systems(sync_carl_gauntlet);
            schedule.run(&mut world);
            let expect = [
                (CarlGauntlet::Locomotion, 3, false, 10.0),
                (CarlGauntlet::Melee, 5, true, 20.0),
                (CarlGauntlet::Throw, 7, false, 30.0),
                (CarlGauntlet::Cf02, 9, true, 40.0),
            ];
            for (kind, index, flip, x) in expect {
                let (got_index, got_flip, at, visibility) = overlay_of(&mut world, kind);
                if shown_expected {
                    assert_eq!(visibility, Visibility::Inherited, "{kind:?}");
                    assert_eq!((got_index, got_flip), (index, flip), "{kind:?}");
                    assert_eq!(at, Vec3::new(x, 40.0, 5.1 + ABOVE_BODY), "{kind:?}");
                } else {
                    assert_eq!(visibility, Visibility::Hidden, "{kind:?}");
                }
            }
            // A body sprite that is hidden this frame hides its overlay.
            if shown_expected {
                let mut hide = world.query_filtered::<&mut Visibility, With<CarlMelee>>();
                for mut visibility in hide.iter_mut(&mut world) {
                    *visibility = Visibility::Hidden;
                }
                schedule.run(&mut world);
                assert_eq!(
                    overlay_of(&mut world, CarlGauntlet::Melee).3,
                    Visibility::Hidden
                );
                assert_eq!(
                    overlay_of(&mut world, CarlGauntlet::Locomotion).3,
                    Visibility::Inherited
                );
            }
            // Four overlays exist and none follows Donut (her sprite has no overlay kind).
            let mut count = world.query::<&CarlGauntlet>();
            assert_eq!(count.iter(&world).count(), 4);
        }
    }

    /// CARL-GAUNTLET-01 acceptance 5: the simulation hash of a fixture run is
    /// identical with the overlay system present and absent (the system reads
    /// `Res<Combat>` only and writes the overlay sprites only).
    #[test]
    fn the_simulation_hash_is_identical_with_the_overlay_system_present_and_absent() {
        const TICKS: u64 = 200;
        let run = |with_overlay: bool| -> (u64, u64) {
            let (args, mut sim, mut anim, combat, mut death) = scene(true);
            let mut world = World::new();
            world.insert_resource(Combat(Some(combat)));
            populate(&mut world);
            let mut schedule = Schedule::default();
            schedule.add_systems(sync_carl_gauntlet);
            for _ in 0..TICKS {
                let mut resource = world.resource_mut::<Combat>();
                let combat = resource.0.as_mut().unwrap();
                let input = args.fixture.map_or_else(RawInput::default, |f| {
                    scripted_input(&args, f, &sim, combat)
                });
                step_combat(
                    &mut sim,
                    combat,
                    &mut anim,
                    Some(&mut death),
                    [input, RawInput::default()],
                );
                if with_overlay {
                    schedule.run(&mut world);
                }
            }
            let resource = world.resource::<Combat>();
            let combat = resource.0.as_ref().unwrap();
            (combat.state_hash(&sim), sim.world.state_hash())
        };
        assert_eq!(run(false), run(true));
    }
}
