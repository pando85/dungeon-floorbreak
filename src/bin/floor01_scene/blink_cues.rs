//! DP-D (DP-Q34): the Bevy half of the Puddle Jumper stand-in cues. The pure
//! state is `dungeon_floorbreak::blink_cues::BlinkCues`; this file only draws what
//! it reports with a pool of code-drawn rectangles in the Magic Missile palette:
//! the ripple rings, the shrinking fizzle ring and the dotted streak. **Presentation
//! only and no new art**: nothing here writes to the simulation, the animators or
//! a resource other than its own. The crouch is drawn by `sync_donut_visuals`
//! from the blink state (`donut_art::blink_crouch_frame`).

use super::*;
use dungeon_floorbreak::blink_cues::{
    BlinkCues, PALETTE, STREAK_DOT_W_PX, ellipse_spans, max_ring_spans,
};

/// How many rings can be alive at once: two 3-ring ripples (the start of a press
/// and the arrival can overlap) and a fizzle ring.
const MAX_RINGS: usize = 7;
/// The most streak dots: 112 px at 6 px a dot, with margin.
const MAX_DOTS: usize = 24;

#[derive(Component, Clone, Copy)]
pub(super) enum BlinkSprite {
    Ring(usize),
    Dot(usize),
}

#[derive(Resource, Default)]
pub(super) struct BlinkCueRes {
    cues: BlinkCues,
}

pub(super) fn spawn(commands: &mut Commands, layers: RenderLayers, z: f32) {
    let mut rect = |kind: BlinkSprite| {
        commands.spawn((
            kind,
            Sprite::from_color(Color::WHITE, Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, z),
            Visibility::Hidden,
            layers.clone(),
        ));
    };
    // A span is at most two rects a row; one slot per rect.
    for i in 0..MAX_RINGS * max_ring_spans() {
        rect(BlinkSprite::Ring(i));
    }
    for i in 0..MAX_DOTS {
        rect(BlinkSprite::Dot(i));
    }
    commands.insert_resource(BlinkCueRes::default());
}

/// One rectangle to draw: left edge, bottom row (world px, y up), size, palette.
struct Piece {
    x: i32,
    y: i32,
    w: i32,
    tone: usize,
}

#[allow(clippy::type_complexity)]
pub(super) fn sync_blink_cues(
    sim: Res<Sim>,
    combat: Res<Combat>,
    state: Option<ResMut<BlinkCueRes>>,
    mut sprites: Query<(&BlinkSprite, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let (Some(mut state), Some(combat)) = (state, combat.0.as_ref()) else {
        return;
    };
    let sim = &sim.0;
    let tick = sim.world.tick;
    let mut rings: Vec<Piece> = Vec::new();
    let mut dots: Vec<Piece> = Vec::new();
    if sim.party_live() {
        let donut = &sim.world.donut;
        state.cues.step(
            tick,
            combat,
            donut.x.round() as i32,
            donut.foot_y.round() as i32,
        );
        let view = state.cues.view(tick);
        for ring in &view.rings {
            for (dy, from, to) in ellipse_spans(ring.rx, ring.ry) {
                let y = ring.foot_y + dy;
                // The right half, then the left half mirrored; the middle row's
                // two halves meet at the centre column.
                rings.push(Piece {
                    x: ring.x + from,
                    y,
                    w: to - from + 1,
                    tone: ring.tone,
                });
                if to > 0 || from > 0 {
                    rings.push(Piece {
                        x: ring.x - to,
                        y,
                        w: to - from + 1,
                        tone: ring.tone,
                    });
                }
            }
        }
        if let Some(streak) = view.streak {
            for x in streak.dots() {
                dots.push(Piece {
                    x,
                    y: streak.y(),
                    w: STREAK_DOT_W_PX,
                    tone: 3,
                });
            }
        }
    } else {
        state.cues.reset();
    }
    for (kind, mut transform, mut sprite, mut visibility) in &mut sprites {
        let piece = match *kind {
            BlinkSprite::Ring(i) => rings.get(i),
            BlinkSprite::Dot(i) => dots.get(i),
        };
        match piece {
            Some(p) => {
                let [r, g, b] = PALETTE[p.tone];
                sprite.color = Color::srgb_u8(r, g, b);
                sprite.custom_size = Some(Vec2::new(p.w as f32, 1.0));
                // The sprite is centred: shift by half its width, and by half a pixel.
                transform.translation.x = p.x as f32 + p.w as f32 / 2.0;
                transform.translation.y = p.y as f32 + 0.5;
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
