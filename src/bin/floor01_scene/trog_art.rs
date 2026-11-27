//! TROG-A: the Bevy half of the troglodyte rooms' presentation
//! (`docs/01-design/floor-01-troglodyte-integration.md` section 12.1, TROG-AD20, AD39..AD41). The pure
//! half is `dungeon_floorbreak::trog_art`; this file only draws what it returns: the TROG-01/02/03
//! atlas frames (the committed `prototype_fixture_unapproved` candidates, **art review pending**).
//! The **labelled hurtbox rectangle** (the box the simulation hits, species colour, state tint)
//! stays as the fallback where an atlas is missing (`scripts/export_assets.sh` was not run) and
//! behind `--debug-combat`. The attack telegraph is the approved Goblin Workshop weight-shadow art cropped
//! to the committed hitbox (TROG-FX-01, `dungeon_floorbreak::trog_art::tell_draw`); the translucent
//! code-drawn box of the committed hitbox is the fallback where that atlas is missing. The room's decor is the AREA-TROG-01 dressing, composed into the
//! kit (`dungeon_floorbreak::trog_kit`); this file draws none of it.
//! **Presentation only**: every system reads the combat state through a shared reference and
//! writes only its own sprites, so nothing here reaches the simulation, the replay or the state
//! hash.

use super::*;
use dungeon_floorbreak::combat_scene::trog::TrogEnemy;
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;
use dungeon_floorbreak::scene_enemy::SlotEnemy;
use dungeon_floorbreak::trog::{Species, State, TrogTuning};
use dungeon_floorbreak::trog_art::{self, MOVE_HOLD_TICKS, TrogLook};

/// One pooled stand-in body, for slot index `.0`.
#[derive(Component, Clone, Copy)]
pub(super) struct TrogBody(usize);

/// One pooled atlas sprite, for slot index `.0`.
#[derive(Component, Clone, Copy)]
pub(super) struct TrogAtlasSprite(usize);

/// What the presenter remembers of a troglodyte between frames (render-local).
#[derive(Default, Clone, Copy)]
pub(super) struct TrogMemo {
    x: Option<f32>,
    last_move: Option<u64>,
    down_since: Option<u64>,
}

/// One pooled stand-in telegraph (the committed hitbox), for slot index `.0`.
#[derive(Component, Clone, Copy)]
pub(super) struct TrogTelegraph(usize);

/// One pooled tile of the telegraph art (TROG-FX-01): tile `.1` of slot index `.0`.
#[derive(Component, Clone, Copy)]
pub(super) struct TrogTellTile(usize, usize);

#[derive(Resource)]
pub(super) struct TrogArt {
    body_z: f32,
    tell_z: f32,
    /// Which species have an atlas loaded (Pygmy, Basher, Virtuoso).
    atlas: [bool; 3],
    /// `--debug-combat`: the hurtbox rectangles draw over the atlas too.
    rects_always: bool,
    /// The telegraph art (the approved weight-shadow sheet) is loaded; the stand-in box draws without it.
    tell_art: bool,
}

fn species_index(species: Species) -> usize {
    match species {
        Species::Pygmy => 0,
        Species::Basher => 1,
        Species::Virtuoso => 2,
    }
}

fn atlas_present(path: &str) -> bool {
    std::env::current_dir().is_ok_and(|root| root.join(path).is_file())
}

fn species_colour(species: Species) -> Color {
    match species {
        Species::Pygmy => Color::srgb(0.66, 0.50, 0.30),
        Species::Basher => Color::srgb(0.58, 0.34, 0.28),
        Species::Virtuoso => Color::srgb(0.40, 0.52, 0.34),
    }
}

/// The decor colour by label (flat, no art): only a **plain** (undressed) room draws any.
fn decor_colour(label: &str) -> Color {
    match label {
        "shelf" | "bench" => Color::srgb(0.36, 0.28, 0.24),
        "threshold" => Color::srgb(0.40, 0.46, 0.52),
        _ => Color::srgb(0.58, 0.72, 0.80),
    }
}

/// The labelled stand-in bodies and their telegraphs. A dressed room's decor is the AREA-TROG-01
/// dressing composed into the kit (nothing is drawn here for it); a **plain** room (the Test of
/// Strength until its stage 2 block lands, TROG-ART-AD19) draws its sidecar solids, the shelf and
/// the 80 px bench, as flat labelled rectangles so the greybox reads. A room without a troglodyte
/// spawns nothing.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
    handles: &mut Vec<(String, Handle<Image>)>,
    room: f01_trog::TrogRoom,
    state: &CombatState,
    kit: &EnvKit,
    height: i32,
    layers: RenderLayers,
    actor_layers: RenderLayers,
    rects_always: bool,
) {
    let mut sheets: [Option<(Handle<Image>, Handle<TextureAtlasLayout>)>; 3] = [None, None, None];
    for species in Species::ALL {
        let path = trog_art::atlas(species);
        if !atlas_present(path) {
            println!(
                "F01_TROG_ART missing {path} (run scripts/export_assets.sh): the labelled rectangle stays for {}",
                species.name()
            );
            continue;
        }
        let asset = path
            .strip_prefix(ATLAS_ASSET_PREFIX)
            .unwrap_or(path)
            .to_owned();
        let texture: Handle<Image> = asset_server.load(asset.clone());
        handles.push((asset, texture.clone()));
        let (w, h) = trog_art::frame_size(species);
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(w, h),
            trog_art::FRAMES,
            1,
            None,
            None,
        ));
        sheets[species_index(species)] = Some((texture, layout));
    }
    // `decor=dressed` when the trog contract has a dressing block for the room, `plain` otherwise
    // (a stage 2 room launches plain until its block lands, TROG-ART-AD19): a plain room draws the flat decor stand-ins below, a dressed room draws none.
    let dressed = std::env::current_dir()
        .is_ok_and(|root| dungeon_floorbreak::trog_kit::is_dressed(&root, room));
    println!(
        "F01_TROG_ART atlases={} rectangles={} decor={} art_review=pending (prototype_fixture_unapproved)",
        sheets.iter().filter(|s| s.is_some()).count(),
        if rects_always { "always" } else { "fallback" },
        if dressed { "dressed" } else { "plain" },
    );
    if !dressed {
        for decor in f01_trog::decor(room) {
            let rect = decor.rect;
            commands.spawn((
                Sprite::from_color(
                    decor_colour(decor.label),
                    Vec2::new(rect.w as f32, rect.h as f32),
                ),
                Anchor::TOP_LEFT,
                Transform::from_xyz(rect.x as f32, to_world_y(height, rect.y), kit.actor_z - 0.2),
                layers.clone(),
            ));
        }
    }
    let tell_sheet = if atlas_present(trog_art::TELL_SHEET) {
        let asset = trog_art::TELL_SHEET
            .strip_prefix(ATLAS_ASSET_PREFIX)
            .unwrap_or(trog_art::TELL_SHEET)
            .to_owned();
        let texture: Handle<Image> = asset_server.load(asset.clone());
        handles.push((asset, texture.clone()));
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(trog_art::TELL_FRAME.0, trog_art::TELL_FRAME.1),
            trog_art::TELL_FRAMES,
            1,
            None,
            None,
        ));
        Some((texture, layout))
    } else {
        println!(
            "F01_TROG_ART missing {} (run scripts/export_assets.sh): the translucent hitbox stays as the telegraph",
            trog_art::TELL_SHEET
        );
        None
    };
    let n = state.slot_enemies().len();
    commands.insert_resource(TrogArt {
        body_z: kit.actor_z + GOBLIN_Z_OFFSET - 32.0 * GOBLIN_Z_STEP,
        tell_z: kit.actor_z + GOBLIN_Z_OFFSET - 33.0 * GOBLIN_Z_STEP,
        atlas: [
            sheets[0].is_some(),
            sheets[1].is_some(),
            sheets[2].is_some(),
        ],
        rects_always,
        tell_art: tell_sheet.is_some(),
    });
    for index in 0..n {
        let species = state.slot_enemies()[index]
            .as_any()
            .and_then(|a| a.downcast_ref::<TrogEnemy>())
            .map(|e| e.trog().species);
        let Some(species) = species else { continue };
        commands.spawn((
            TrogBody(index),
            Sprite::from_color(species_colour(species), Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, kit.actor_z),
            Visibility::Hidden,
            actor_layers.clone(),
        ));
        if let Some((texture, layout)) = &sheets[species_index(species)] {
            commands.spawn((
                TrogAtlasSprite(index),
                Sprite::from_atlas_image(
                    texture.clone(),
                    TextureAtlas {
                        layout: layout.clone(),
                        index: 0,
                    },
                ),
                Transform::from_xyz(
                    0.0,
                    0.0,
                    kit.actor_z + GOBLIN_Z_OFFSET - 32.0 * GOBLIN_Z_STEP,
                ),
                Visibility::Hidden,
                actor_layers.clone(),
            ));
        }
        if let Some((texture, layout)) = &tell_sheet {
            for tile in 0..trog_art::TELL_MAX_PIECES {
                commands.spawn((
                    TrogTellTile(index, tile),
                    Sprite::from_atlas_image(
                        texture.clone(),
                        TextureAtlas {
                            layout: layout.clone(),
                            index: 0,
                        },
                    ),
                    Anchor::BOTTOM_LEFT,
                    Transform::from_xyz(0.0, 0.0, kit.actor_z),
                    Visibility::Hidden,
                    actor_layers.clone(),
                ));
            }
        }
        commands.spawn((
            TrogTelegraph(index),
            Sprite::from_color(Color::srgba(1.0, 0.25, 0.15, 0.3), Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, kit.actor_z),
            Visibility::Hidden,
            actor_layers.clone(),
        ));
    }
}

/// Place the stand-in bodies and telegraphs from the simulation each frame.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn sync_trog_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    death: Res<Death>,
    art: Option<Res<TrogArt>>,
    mut memo: Local<Vec<TrogMemo>>,
    mut bodies: Query<
        (&TrogBody, &mut Transform, &mut Sprite, &mut Visibility),
        (
            Without<TrogTelegraph>,
            Without<TrogAtlasSprite>,
            Without<TrogTellTile>,
        ),
    >,
    mut atlas_sprites: Query<
        (
            &TrogAtlasSprite,
            &mut Transform,
            &mut Sprite,
            &mut Visibility,
        ),
        (
            Without<TrogTelegraph>,
            Without<TrogBody>,
            Without<TrogTellTile>,
        ),
    >,
    mut telegraphs: Query<
        (&TrogTelegraph, &mut Transform, &mut Sprite, &mut Visibility),
        (
            Without<TrogBody>,
            Without<TrogAtlasSprite>,
            Without<TrogTellTile>,
        ),
    >,
    mut tiles: Query<
        (&TrogTellTile, &mut Transform, &mut Sprite, &mut Visibility),
        (
            Without<TrogBody>,
            Without<TrogAtlasSprite>,
            Without<TrogTelegraph>,
        ),
    >,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let t = TrogTuning::active();
    let solids = &sim.0.world.solids;
    let tick = sim.0.world.tick;
    // The atlas sprites: the frame the pure half picks, mirrored with the facing.
    for (id, mut transform, mut sprite, mut visibility) in &mut atlas_sprites {
        let enemy = state
            .slot_enemies()
            .get(id.0)
            .and_then(|e| e.as_any())
            .and_then(|a| a.downcast_ref::<TrogEnemy>());
        let Some(trog) = enemy.map(TrogEnemy::trog) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        // TROG-FX-01: a point blank blast replaced the body with its own parts (gore presets 2 to 4).
        if death
            .0
            .as_ref()
            .is_some_and(|presenter| enemy.is_some_and(|e| presenter.hides(e.target_id())))
        {
            *visibility = Visibility::Hidden;
            continue;
        }
        if memo.len() <= id.0 {
            memo.resize(id.0 + 1, TrogMemo::default());
        }
        let m = &mut memo[id.0];
        if m.x.is_some_and(|prev| (prev - trog.x).abs() > f32::EPSILON) {
            m.last_move = Some(tick);
        }
        m.x = Some(trog.x);
        if trog.state == State::Down {
            m.down_since.get_or_insert(tick);
        } else {
            m.down_since = None;
        }
        let look = TrogLook {
            moving: m
                .last_move
                .is_some_and(|t| tick.saturating_sub(t) <= MOVE_HOLD_TICKS),
            down_age: m.down_since.map(|s| tick.saturating_sub(s)),
        };
        let d = trog_art::draw(t, trog, tick, look);
        transform.translation = Vec3::new(d.centre.0.round(), d.centre.1.round(), art.body_z);
        sprite.flip_x = d.flip_x;
        sprite.color = if trog.state == State::Hurt {
            FLASH_TINT
        } else {
            Color::WHITE
        };
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = usize::from(d.frame);
        }
        *visibility = Visibility::Inherited;
    }
    for (id, mut transform, mut sprite, mut visibility) in &mut bodies {
        let enemy = state
            .slot_enemies()
            .get(id.0)
            .and_then(|e| e.as_any())
            .and_then(|a| a.downcast_ref::<TrogEnemy>());
        let Some(trog) = enemy.map(TrogEnemy::trog) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        if death
            .0
            .as_ref()
            .is_some_and(|presenter| enemy.is_some_and(|e| presenter.hides(e.target_id())))
        {
            *visibility = Visibility::Hidden;
            continue;
        }
        // Where the atlas draws this species the rectangle is hidden (shown, faint, under
        // `--debug-combat`); it is the visible body only where the atlas is missing.
        let atlas_drawn = art.atlas[species_index(trog.species)];
        if atlas_drawn && !art.rects_always {
            *visibility = Visibility::Hidden;
            continue;
        }
        // A downed body lies flat on the floor (the `down` frame holds, TROG-AD19).
        let rect = trog
            .hurtbox(t)
            .unwrap_or(dungeon_floorbreak::prototype::Rect {
                left: trog.x - 20.0,
                bottom: trog.foot_y,
                width: 40.0,
                height: 10.0,
            });
        sprite.custom_size = Some(Vec2::new(rect.width, rect.height));
        transform.translation = Vec3::new(
            (rect.left + rect.width / 2.0).round(),
            (rect.bottom + rect.height / 2.0).round(),
            art.body_z,
        );
        let base = species_colour(trog.species);
        sprite.color = match trog.state {
            State::Down => Color::srgba(0.3, 0.3, 0.3, 0.7),
            State::Hurt => FLASH_TINT,
            State::Tell => Color::srgb(1.0, 0.9, 0.35),
            State::Active => Color::srgb(1.0, 0.45, 0.25),
            State::Recover => Color::srgb(0.55, 0.62, 0.9),
            State::Dormant => base.darker(0.25),
            State::Approach | State::Rest => base,
        };
        if atlas_drawn {
            sprite.color = sprite.color.with_alpha(0.35);
        }
        *visibility = Visibility::Inherited;
    }
    // The telegraph art: the approved weight-shadow tile, cropped to the committed rectangle.
    let draws: Vec<Option<trog_art::TellDraw>> = state
        .slot_enemies()
        .iter()
        .map(|e| {
            e.as_any()
                .and_then(|a| a.downcast_ref::<TrogEnemy>())
                .and_then(|e| trog_art::tell_draw(t, e.trog(), solids, tick))
        })
        .collect();
    for (id, mut transform, mut sprite, mut visibility) in &mut tiles {
        let draw = draws.get(id.0).copied().flatten();
        let piece = draw.and_then(|d| trog_art::tell_pieces(d.width).get(id.1).map(|p| (d, *p)));
        let Some((d, piece)) = piece else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let (w, h) = (piece.width as f32, d.height);
        sprite.rect = Some(bevy::math::Rect::new(
            piece.src_x as f32,
            0.0,
            piece.src_x as f32 + w,
            h,
        ));
        sprite.custom_size = Some(Vec2::new(w, h));
        sprite.color = Color::srgba(1.0, 1.0, 1.0, f32::from(trog_art::TELL_ALPHA) / 255.0);
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = usize::from(d.frame);
        }
        transform.translation = Vec3::new(d.x.round() + piece.dx as f32, d.y.round(), art.tell_z);
        *visibility = Visibility::Inherited;
    }
    for (id, mut transform, mut sprite, mut visibility) in &mut telegraphs {
        if art.tell_art {
            *visibility = Visibility::Hidden;
            continue;
        }
        let threat = state
            .slot_enemies()
            .get(id.0)
            .and_then(|e| e.as_any())
            .and_then(|a| a.downcast_ref::<TrogEnemy>())
            .and_then(|e| e.trog().threat(t, solids));
        match threat {
            Some((rect, left)) => {
                sprite.custom_size = Some(Vec2::new(rect.width, rect.height));
                transform.translation = Vec3::new(
                    (rect.left + rect.width / 2.0).round(),
                    (rect.bottom + rect.height / 2.0).round(),
                    art.tell_z,
                );
                // Faint while it is only a tell, solid while the box is live.
                sprite.color = Color::srgba(1.0, 0.2, 0.1, if left == 0 { 0.65 } else { 0.22 });
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
