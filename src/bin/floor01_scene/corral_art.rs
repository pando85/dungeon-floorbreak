//! CORRAL-WIRE and CORRAL-E: the Bevy half of room 0's greybox stand-ins.
//!
//! * The M3 gate (CRL-Q26, CW-AD5): a code-drawn iron grate on the gate's solid
//!   (2048..2064, floor to the lane ceiling), drawn exactly while the simulation
//!   keeps the block standing (`!exit_gate_open()`). No art exists for it.
//! * The rush stand-in (CRL-Q38): the brace lean, hoof scrape, charge trail, stop
//!   dust and bonk burst of `dungeon_floorbreak::combat_anim::rush_stand_in`, drawn
//!   with a pool of code-drawn rectangles over the reused LLAMA-01 idle frames
//!   (the lean is applied by `sync_llama_visuals`). LLAMA-03 draws the body from the rush atlas instead of
//!   the idle frames and this keeps only the dust and dirt (the stand-in body stays as the missing-atlas
//!   fallback).
//!
//! * The goblin partition (CORRAL-D, `HEAT-AD1`): the three frames of the AREA-HEAT-01
//!   atlas `floor01_heat_partition` (intact, weakened, broken), the frame of the
//!   prop's state, drawn with the art's solid core on the collider.
//!
//! **Presentation only and no new art**: it reads the combat state through a
//! shared reference and writes only its own sprites, so nothing here reaches the
//! simulation, the replay or the state hash.

use super::*;
use dungeon_floorbreak::camera_lab::room::IRect as RoomRect;
use dungeon_floorbreak::combat_anim::rush_stand_in::{
    MAX_PIECES, PALETTE, Piece, rush_stand_in_art,
};

/// The grate's iron: the backing and the bars (greybox, no art).
const GATE_BACKING: Color = Color::srgba(0.09, 0.08, 0.10, 0.82);
const GATE_BAR: Color = Color::srgb(0.36, 0.34, 0.38);
/// Bar columns inside the 16 px gate and the crossbar pitch.
const GATE_BAR_COLUMNS: [i32; 3] = [1, 7, 13];
const GATE_BAR_W: i32 = 2;
const GATE_CROSS_PITCH: i32 = 24;
/// The grate's pool: the backing, the bars and the crossbars of a 240 px gate.
const GATE_RECTS: usize = 1 + GATE_BAR_COLUMNS.len() + 10;

#[derive(Component, Clone, Copy)]
pub(super) enum CorralSprite {
    Gate(usize),
    Rush(usize),
}

/// The gate rect (layout px, y down) and the rush pool size.
#[derive(Resource)]
pub(super) struct CorralArt {
    gate: Option<RoomRect>,
    rush_slots: usize,
}

/// Spawn the grate (when the room has an M3 gate) and a rush pool sized for
/// `rush_llamas` corral llamas.
pub(super) fn spawn(
    commands: &mut Commands,
    layers: RenderLayers,
    z: f32,
    gate: Option<RoomRect>,
    rush_llamas: usize,
) {
    if gate.is_none() && rush_llamas == 0 {
        return;
    }
    let mut rect = |kind: CorralSprite, colour: Color, dz: f32| {
        commands.spawn((
            kind,
            Sprite::from_color(colour, Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, z + dz),
            Visibility::Hidden,
            layers.clone(),
        ));
    };
    if let Some(g) = gate {
        for i in 0..GATE_RECTS {
            let colour = if i == 0 { GATE_BACKING } else { GATE_BAR };
            rect(
                CorralSprite::Gate(i),
                colour,
                if i == 0 { 0.0 } else { 0.001 },
            );
        }
        println!(
            "CORRAL_GATE drawn at x {}..{} (greybox stand-in, no art)",
            g.x,
            g.x + g.w
        );
    }
    let rush_slots = rush_llamas * MAX_PIECES;
    for i in 0..rush_slots {
        rect(CorralSprite::Rush(i), Color::WHITE, 0.03);
    }
    if rush_llamas > 0 {
        println!(
            "CORRAL_RUSH_STAND_IN pool {rush_llamas} llamas (code-drawn stand-in, LLAMA-03 pending)"
        );
    }
    commands.insert_resource(CorralArt { gate, rush_slots });
}

/// The grate's rectangles (world px, y up, bottom-left, size) for a gate whose
/// bottom-left is `(x, y)` and size `w` x `h`.
fn gate_rects(x: i32, y: i32, w: i32, h: i32) -> Vec<(i32, i32, i32, i32)> {
    let mut out = vec![(x, y, w, h)];
    for c in GATE_BAR_COLUMNS {
        out.push((x + c, y, GATE_BAR_W, h));
    }
    let mut cy = y + GATE_CROSS_PITCH / 2;
    while cy < y + h && out.len() < GATE_RECTS {
        out.push((x, cy, w, GATE_BAR_W));
        cy += GATE_CROSS_PITCH;
    }
    out
}

fn place(
    transform: &mut Transform,
    sprite: &mut Sprite,
    visibility: &mut Visibility,
    (x, y, w, h): (i32, i32, i32, i32),
) {
    sprite.custom_size = Some(Vec2::new(w as f32, h as f32));
    transform.translation.x = x as f32 + w as f32 / 2.0;
    transform.translation.y = y as f32 + h as f32 / 2.0;
    *visibility = Visibility::Inherited;
}

#[allow(clippy::type_complexity)]
pub(super) fn sync_corral_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<CorralArt>>,
    rush_look: Option<Res<LlamaRushLook>>,
    mut sprites: Query<(&CorralSprite, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let gate = match art.gate {
        Some(g) if !state.exit_gate_open() => {
            let layout_h = sim.0.world.layout_height();
            gate_rects(g.x, layout_h - (g.y + g.h), g.w, g.h)
        }
        _ => Vec::new(),
    };
    let mut pieces: Vec<Piece> = Vec::new();
    for llama in &state.llamas {
        // LLAMA-03: with the rush art drawing the body, the stand-in keeps only the dust and dirt.
        if let Some(s) = rush_stand_in_art(
            llama,
            CombatAnimators::llama_foot_y(llama),
            rush_look.is_some(),
        ) {
            pieces.extend(s.pieces);
        }
    }
    pieces.truncate(art.rush_slots);
    for (kind, mut transform, mut sprite, mut visibility) in &mut sprites {
        match *kind {
            CorralSprite::Gate(i) => match gate.get(i) {
                Some(&r) => place(&mut transform, &mut sprite, &mut visibility, r),
                None => *visibility = Visibility::Hidden,
            },
            CorralSprite::Rush(i) => match pieces.get(i) {
                Some(p) => {
                    let [r, g, b, a] = PALETTE[p.tone.index()];
                    sprite.color = Color::srgba_u8(r, g, b, a);
                    place(
                        &mut transform,
                        &mut sprite,
                        &mut visibility,
                        (p.x, p.y, p.w, p.h),
                    );
                }
                None => *visibility = Visibility::Hidden,
            },
        }
    }
}

/// CORRAL-D (`HEAT-AD1`): the partition's presenter state: the prop it reads, the
/// atlas rect of each state frame and the canvas's top-left (layout px, y down).
#[derive(Resource)]
pub(super) struct PartitionArt {
    prop: usize,
    rects: [[i32; 4]; 3],
    origin: (i32, i32),
}

#[derive(Component)]
pub(super) struct PartitionSprite;

/// Spawn the partition's sprite (hidden until the first sync) when the room has a
/// partition prop and the heat contract holds its three frames. Returns whether
/// the presenter is live. `z` is just above the dressing's props and below the
/// actors.
pub(super) fn spawn_partition(
    commands: &mut Commands,
    texture: &Handle<Image>,
    encounter: &dungeon_floorbreak::encounter::Encounter,
    z: f32,
    layers: RenderLayers,
) -> bool {
    let Some((prop, target)) = encounter
        .props
        .iter()
        .enumerate()
        .find(|(_, p)| p.kind == dungeon_floorbreak::encounter::PropKind::Partition)
        .map(|(i, p)| (i, p.target))
    else {
        return false;
    };
    let root = std::env::current_dir().unwrap_or_default();
    let rects = match dungeon_floorbreak::heat_kit::partition_atlas_rects(&root) {
        Ok(rects) => rects,
        Err(e) => {
            println!("CORRAL_PARTITION not drawn: {e}");
            return false;
        }
    };
    let origin = dungeon_floorbreak::heat_kit::partition_origin(target);
    commands.spawn((
        PartitionSprite,
        Sprite {
            image: texture.clone(),
            rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
            ..default()
        },
        Anchor::TOP_LEFT,
        Transform::from_xyz(origin.0 as f32, 0.0, z),
        Visibility::Hidden,
        layers,
    ));
    commands.insert_resource(PartitionArt {
        prop,
        rects,
        origin,
    });
    println!(
        "CORRAL_PARTITION drawn at ({}, {}) from the heat atlas, three states (prototype art, owner review pending)",
        origin.0, origin.1
    );
    true
}

/// Show the frame of the partition's state. Presentation only: it reads the prop
/// through a shared reference and writes only its own sprite.
pub(super) fn sync_partition_art(
    sim: Res<Sim>,
    combat: Res<Combat>,
    art: Option<Res<PartitionArt>>,
    mut sprite_q: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<PartitionSprite>>,
) {
    let (Some(art), Some(state)) = (art, combat.0.as_ref()) else {
        return;
    };
    let Some(part) = state.env().and_then(|env| env.partition(art.prop)) else {
        return;
    };
    let r = art.rects[part.state() as usize];
    let layout_h = sim.0.world.layout_height();
    for (mut transform, mut sprite, mut visibility) in &mut sprite_q {
        sprite.rect = Some(Rect::new(
            r[0] as f32,
            r[1] as f32,
            (r[0] + r[2]) as f32,
            (r[1] + r[3]) as f32,
        ));
        transform.translation.x = art.origin.0 as f32;
        transform.translation.y = (layout_h - art.origin.1) as f32;
        *visibility = Visibility::Inherited;
    }
}
