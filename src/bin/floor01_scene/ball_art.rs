//! BOS-ART: the Bevy half of the Ball of Swine's room 18 (the pure half is
//! `dungeon_floorbreak::ball_art`, the dressing `dungeon_floorbreak::ball_kit`). With the SWINE-01 atlas
//! exported (`bash scripts/export_assets.sh`) and the Ball's dressed kit loaded, one pooled sprite draws the
//! ball's body from `ball_art::body_frame` (the six roll frames are native cels, never rotated or scaled), a
//! pool of sprites draws the chain weight (the weight, the chain it pays out, the tie to the post) at the sim's
//! rack footprint, and one sprite lays the open exit door over the composer's closed one once the exit is
//! released (the stairwell behind the right wall). The greybox of BOS-A (the body box and its label, the arch
//! outlines, the rack and strut rects) is not drawn while this presenter is live
//! (`ball_art::greybox_shown`). A missing atlas prints one `BALL_ART not drawn` line and the greybox stays.
//!
//! **Presentation only**: it reads the combat state through a shared reference and writes only the
//! rect, position and visibility of its own sprites, so nothing here reaches the simulation, the replay or the
//! state hash.

use super::*;
use dungeon_floorbreak::ball_art as art;
use dungeon_floorbreak::ball_kit as kit_art;
use dungeon_floorbreak::env_kit::ATLAS_ASSET_PREFIX;

/// The sprites of the presenter.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(super) enum BallSprite {
    Body,
    Part(usize),
    Portal,
}

/// The weight's pool: the weight, the tie (5), the paid-out chain (up to 14).
const PARTS: usize = 24;

/// The presenter is live: the pieces' atlas rects, read from the contract at spawn.
#[derive(Resource)]
pub(super) struct BallArt {
    rects: kit_art::PresenterRects,
}

fn atlas_of(handles: &[(String, Handle<Image>)], name: &str) -> Option<Handle<Image>> {
    handles
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, h)| h.clone())
}

fn rect_of(r: [i32; 4], rows: f32) -> Rect {
    Rect::new(
        r[0] as f32,
        r[1] as f32,
        (r[0] + r[2]) as f32,
        r[1] as f32 + rows,
    )
}

/// Spawn the presenter's sprites for the Ball's room. `portal_z` is the composer's closed exit door's z.
/// Returns whether the presenter is live (a missing atlas or contract piece leaves the greybox).
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn(
    commands: &mut Commands,
    asset_server: &AssetServer,
    combat_handles: &mut Vec<(String, Handle<Image>)>,
    handles: &[(String, Handle<Image>)],
    actor_z: f32,
    portal_z: Option<f32>,
    height: i32,
    actor_layers: RenderLayers,
    prop_layers: RenderLayers,
) -> bool {
    let root = std::env::current_dir().unwrap_or_default();
    if !root.join(art::ATLAS).is_file() {
        println!(
            "BALL_ART not drawn: {} is missing (run scripts/export_assets.sh)",
            art::ATLAS
        );
        return false;
    }
    let rects = match kit_art::presenter_rects(&root) {
        Ok(rects) => rects,
        Err(e) => {
            println!("BALL_ART not drawn: {e}");
            return false;
        }
    };
    let (Some(props), Some(guild)) = (
        rects
            .piece(art::Piece::WeightHung.id())
            .and_then(|(a, _)| atlas_of(handles, a)),
        atlas_of(handles, &rects.portal_open.0),
    ) else {
        println!("BALL_ART not drawn: the Ball's kit atlases are not loaded");
        return false;
    };
    let asset = art::ATLAS
        .strip_prefix(ATLAS_ASSET_PREFIX)
        .unwrap_or(art::ATLAS);
    let swine: Handle<Image> = asset_server.load(asset.to_owned());
    combat_handles.push((asset.to_owned(), swine.clone()));
    commands.spawn((
        BallSprite::Body,
        Sprite {
            image: swine,
            rect: Some(Rect::new(
                0.0,
                0.0,
                art::FRAME.0 as f32,
                art::FRAME.1 as f32,
            )),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, actor_z + BOSS_Z_OFFSET),
        Visibility::Hidden,
        actor_layers.clone(),
    ));
    for i in 0..PARTS {
        commands.spawn((
            BallSprite::Part(i),
            Sprite {
                image: props.clone(),
                rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                ..default()
            },
            Anchor::BOTTOM_LEFT,
            Transform::from_xyz(0.0, 0.0, actor_z + BOSS_Z_BACK),
            Visibility::Hidden,
            actor_layers.clone(),
        ));
    }
    if let Some(z) = portal_z {
        let (_, rect) = &rects.portal_open;
        commands.spawn((
            BallSprite::Portal,
            Sprite {
                image: guild,
                rect: Some(rect_of(*rect, rect[3] as f32)),
                ..default()
            },
            Anchor::TOP_LEFT,
            Transform::from_xyz(
                kit_art::PORTAL_AT.0 as f32,
                to_world_y(height, kit_art::PORTAL_AT.1),
                z + 0.0005,
            ),
            Visibility::Hidden,
            prop_layers,
        ));
    }
    commands.insert_resource(BallArt { rects });
    println!(
        "BALL_ART drawn: SWINE-01 body, the chain weight and the open stairwell from the exported atlases \
         (prototype art, owner review pending)"
    );
    true
}

/// The body, the weight and the stairwell for this tick. Presentation only.
#[allow(clippy::type_complexity)]
pub(super) fn sync_ball_art(
    combat: Res<Combat>,
    sim: Res<Sim>,
    setup: Res<Setup>,
    art_res: Option<Res<BallArt>>,
    mut walk: Local<WalkTrack>,
    mut sprites: Query<(&BallSprite, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(ball), Some(state)) = (art_res, combat.0.as_ref()) else {
        return;
    };
    let Some(boss) = state.boss.as_ref() else {
        return;
    };
    let tick = sim.0.world.tick;
    if walk.tick != tick {
        walk.walking = boss.mode == BossMode::Neutral && (boss.x - walk.x).abs() > 1e-3;
        walk.tick = tick;
        walk.x = boss.x;
    }
    let floor = boss.floor_y();
    let actor_z = setup.kit.actor_z;
    let draws = art::weight_draws(boss);
    let defeat_age = dungeon_floorbreak::boss_art::defeat_age(&state.events, tick);
    for (kind, mut transform, mut sprite, mut visibility) in &mut sprites {
        let shown = match *kind {
            BallSprite::Body => {
                let frame = art::body_frame(boss, tick, walk.walking, defeat_age);
                let spec = ActorSpec {
                    name: "rolling_boss_f01",
                    width: art::FRAME.0,
                    height: art::FRAME.1,
                    pivot_x: art::PIVOT.0,
                    pivot_y: art::PIVOT.1,
                    shadow_width: 0,
                    shadow_pivot_x: 0,
                };
                let flip = boss.facing < 0;
                let (cx, cy) = actor_center(spec, boss.x, floor, flip);
                transform.translation.x = cx;
                transform.translation.y = cy;
                sprite.flip_x = flip;
                let left = (frame as i32 * art::FRAME.0) as f32;
                sprite.rect = Some(Rect::new(
                    left,
                    0.0,
                    left + art::FRAME.0 as f32,
                    art::FRAME.1 as f32,
                ));
                true
            }
            BallSprite::Part(i) => match draws.get(i) {
                Some(draw) => {
                    if let Some((_, rect)) = ball.rects.piece(draw.piece.id()) {
                        sprite.rect = Some(rect_of(rect, draw.rows));
                        transform.translation.x = draw.left.round();
                        transform.translation.y = (floor + draw.bottom).round();
                        transform.translation.z = actor_z
                            + match draw.band {
                                art::Band::Back => BOSS_Z_BACK,
                                art::Band::Front => BOSS_Z_FRONT,
                            }
                            + i as f32 * 0.0001;
                        true
                    } else {
                        false
                    }
                }
                None => false,
            },
            BallSprite::Portal => kit_art::stairwell_open(boss.released()),
        };
        let want = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
}
