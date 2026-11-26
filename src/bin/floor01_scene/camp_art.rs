//! AREA-CAMP-01: the Bevy half of the aftermath's heavy bag.
//!
//! The dressing (`dungeon_floorbreak::camp_kit`) places the room's wall, structure and prop
//! pieces through the kit; the bag is **not** placed. It is one presenter sprite (40 x 122 at
//! (620, 450), just under the actors on the prop layer) whose atlas frame is a pure function of
//! the room tick and the heroes' active hit boxes (`camp_kit::BagSway`):
//!
//! * the entry swing plays the 38-tick table once from the first tick the presenter observes;
//! * the rising edge of an overlap between the bag's rect {624, 516, 32, 64} and Carl's active
//!   melee hit box (`combat::hit_box`, the call the strike sparks use) or Donut's
//!   `donut_hitbox()` restarts it, pushed away from the attacker.
//!
//! **Presentation only**: it reads the simulation and the combat state through shared references
//! and writes only its own sprite and its own sway memory. The sim's `hit_target` stays inert (no
//! knockback, no score, no event), so nothing here reaches the replay or the state hash.

use super::*;
use dungeon_floorbreak::camp_kit as camp;

#[derive(Component)]
pub(super) struct BagSprite;

/// The atlas rect of every bag frame and the presenter's own memory.
#[derive(Resource)]
pub(super) struct BagArt {
    rects: [[i32; 4]; 5],
    height: i32,
    sway: camp::BagSway,
}

fn atlas_rect(r: [i32; 4]) -> Rect {
    Rect::new(
        r[0] as f32,
        r[1] as f32,
        (r[0] + r[2]) as f32,
        (r[1] + r[3]) as f32,
    )
}

/// Spawn the bag sprite (hidden until the first sync). `atlas` is the `floor01_aft_bag` image and
/// `z` sits just under the actors. Returns whether the presenter is live (a missing contract piece
/// leaves the bag undrawn).
pub(super) fn spawn(
    commands: &mut Commands,
    atlas: &Handle<Image>,
    height: i32,
    z: f32,
    layers: RenderLayers,
) -> bool {
    let root = std::env::current_dir().unwrap_or_default();
    let rects = match camp::bag_rects(&root) {
        Ok(rects) => rects,
        Err(e) => {
            println!("CAMP_ART not drawn: {e}");
            return false;
        }
    };
    let at = camp::BAG_AT;
    commands.spawn((
        BagSprite,
        Sprite {
            image: atlas.clone(),
            rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
            ..default()
        },
        Anchor::TOP_LEFT,
        Transform::from_xyz(at.0 as f32, to_world_y(height, at.1), z),
        Visibility::Hidden,
        layers,
    ));
    commands.insert_resource(BagArt {
        rects,
        height,
        sway: camp::BagSway::default(),
    });
    println!(
        "CAMP_ART drawn: the heavy bag (5 frames, entry swing and hit sway) from the floor01_aft_bag \
         atlas (prototype art, owner review pending)"
    );
    true
}

/// Show the bag's frame. Presentation only.
pub(super) fn sync_bag_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<ResMut<BagArt>>,
    mut sprites: Query<(&mut Sprite, &mut Visibility), With<BagSprite>>,
) {
    let (Some(mut art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let scene = &sim.0;
    let bag = camp::bag_rect_world(art.height);
    // Carl: his active melee hit box, the call the strike sparks use.
    let hero = &scene.world.hero;
    let attacker = attacker_of(hero);
    let carl = hero
        .melee
        .current()
        .filter(|m| m.phase == Phase::Active)
        .and_then(|m| hit_box(&m.spec.shape, &attacker))
        .filter(|r| r.overlaps(bag))
        .map(|_| (hero.x.round() as i32, i32::from(hero.facing)));
    // Donut: her hit box, while she is in the party.
    let donut = state
        .donut_hitbox()
        .filter(|_| scene.party_live())
        .filter(|r| r.overlaps(bag))
        .map(|_| {
            let d = &scene.world.donut;
            (d.x.round() as i32, i32::from(d.facing))
        });
    let frame = art.sway.observe(scene.world.tick, carl, donut);
    let rect = art.rects[camp::bag_piece_index(frame)];
    for (mut sprite, mut visibility) in &mut sprites {
        sprite.rect = Some(atlas_rect(rect));
        *visibility = Visibility::Inherited;
    }
}
