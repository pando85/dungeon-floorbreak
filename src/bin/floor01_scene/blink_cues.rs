//! DP-D (DP-Q34) and DONUT-MOVES-ART-01: the Bevy half of the Puddle Jumper cues.
//! The pure state is `dungeon_floorbreak::blink_cues::BlinkCues`; this file only
//! draws what it reports, from the production atlases:
//!
//! - the puddle rings (`ring_open` at the origin and the destination, `ring_close`
//!   at the origin after she has left) and the drying fizzle ring, strip frames;
//! - the streak: `streak_dash` tiled from the origin and the `streak_head` at the
//!   arrival end;
//! - her own frames: `blink_crouch`, `blink_dip`, `blink_vanish` at the origin
//!   (the windup), `blink_pop`, `blink_arrive`, `blink_settle` at the destination
//!   (ages 6 to 12) and `fizzle_bonk`, `fizzle_huff` on a fizzle, moves-atlas
//!   frames drawn in place of her locomotion sprite.
//!
//! **Presentation only**: nothing here writes to the simulation, the animators or
//! a resource other than its own. A missing atlas leaves its cue undrawn. Her
//! frames are drawn only while her locomotion sprite is (a hurt reaction, a
//! knockback or a melee move that owns her sprite wins and hides them).

use super::donut_cues::{MovesArt, vfx_center};
use super::*;
use dungeon_floorbreak::blink_cues::{BlinkCues, DASH_W_PX, HEAD_W_PX};
use dungeon_floorbreak::donut_art;

/// How many strip puddles can be alive at once: the origin ring, the destination ring and the
/// fizzle ring (a press overlapping an old arrival's ring adds one spare).
const MAX_PUDDLES: usize = 4;
/// The most streak dashes: the farthest blink in the data (112 px) is seven tiles; room for more.
const MAX_DASHES: usize = 24;

#[derive(Component, Clone, Copy)]
pub(super) enum BlinkSprite {
    /// Her moves-atlas frame (windup, arrival or fizzle).
    Body,
    Puddle(usize),
    Dash(usize),
    Head,
}

#[derive(Resource, Default)]
pub(super) struct BlinkCueRes {
    cues: BlinkCues,
}

pub(super) fn spawn(commands: &mut Commands, layers: RenderLayers, z: f32, art: &MovesArt) {
    let mut sprite = |kind: BlinkSprite,
                      atlas: &Option<(Handle<Image>, Handle<TextureAtlasLayout>)>,
                      dz: f32,
                      index: usize| {
        let Some((texture, layout)) = atlas.clone() else {
            return;
        };
        commands.spawn((
            kind,
            Sprite::from_atlas_image(texture, TextureAtlas { layout, index }),
            Transform::from_xyz(0.0, 0.0, z + dz),
            Visibility::Hidden,
            layers.clone(),
        ));
    };
    // `z` is the actor depth plus a hair: her frames sit above the melee sprites, the puddle strip
    // just behind her (the in-sprite puddle mouth is part of her frame), the streak in front.
    sprite(BlinkSprite::Body, &art.moves, -0.013, 0);
    for i in 0..MAX_PUDDLES {
        sprite(BlinkSprite::Puddle(i), &art.vfx, -0.03, 0);
    }
    for i in 0..MAX_DASHES {
        sprite(
            BlinkSprite::Dash(i),
            &art.vfx,
            0.0,
            donut_art::PUDDLE_DASH_INDEX,
        );
    }
    sprite(
        BlinkSprite::Head,
        &art.vfx,
        0.0,
        donut_art::PUDDLE_HEAD_INDEX,
    );
    commands.insert_resource(BlinkCueRes::default());
}

/// One strip piece to draw: world centre and the atlas index, mirrored when `flip`.
struct Piece {
    at: (f32, f32),
    index: usize,
    flip: bool,
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(super) fn sync_blink_cues(
    sim: Res<Sim>,
    combat: Res<Combat>,
    actors: Res<Actors>,
    state: Option<ResMut<BlinkCueRes>>,
    mut locomotion: Query<(&ActorVisual, &mut Visibility), Without<BlinkSprite>>,
    mut sprites: Query<
        (&BlinkSprite, &mut Transform, &mut Sprite, &mut Visibility),
        Without<ActorVisual>,
    >,
) {
    let (Some(mut state), Some(combat)) = (state, combat.0.as_ref()) else {
        return;
    };
    let sim = &sim.0;
    let tick = sim.world.tick;
    let party = sim.party_live();
    // Her locomotion sprite is drawn this frame (nothing earlier took her over).
    let shown = locomotion
        .iter()
        .any(|(actor, visibility)| !actor.is_hero && *visibility != Visibility::Hidden);
    let mut puddles: Vec<Piece> = Vec::new();
    let mut dashes: Vec<Piece> = Vec::new();
    let mut head: Option<Piece> = None;
    let mut body: Option<(usize, (f32, f32), bool)> = None;
    if party {
        let donut = &sim.world.donut;
        state.cues.step(
            tick,
            combat,
            donut.x.round() as i32,
            donut.foot_y.round() as i32,
        );
        let view = state.cues.view(tick);
        for cue in &view.puddles {
            puddles.push(Piece {
                at: vfx_center(cue.x as f32, cue.foot_y as f32),
                index: cue.frame,
                flip: false,
            });
        }
        if let Some(streak) = view.streak {
            let dir = streak.dir();
            let y = streak.y() as f32;
            // A tile is centred 8 px along the streak from its near edge; the dash art faces right.
            for edge in streak.dash_edges() {
                let (cx, cy) = streak_center(edge as f32, dir, y, DASH_W_PX);
                dashes.push(Piece {
                    at: (cx, cy),
                    index: donut_art::PUDDLE_DASH_INDEX,
                    flip: dir < 0,
                });
            }
            let (cx, cy) = streak_center(streak.head_edge() as f32, dir, y, HEAD_W_PX);
            head = Some(Piece {
                at: (cx, cy),
                index: donut_art::PUDDLE_HEAD_INDEX,
                flip: dir < 0,
            });
        }
        // Her own frame: the windup at the origin, else the rise at the destination, else a fizzle.
        let frame = donut_art::blink_crouch_frame(
            combat.donut_blink().windup_age(tick),
            DonutProgTuning::active().dpq34_crouch_ticks,
        )
        .or(view.arrive)
        .or(view.fizzle);
        if let Some(frame) = frame.filter(|_| shown) {
            let flip = donut.facing < 0;
            let spec = actors.donut.spec;
            body = Some((frame, actor_center(spec, donut.x, donut.foot_y, flip), flip));
        }
    } else {
        state.cues.reset();
    }
    if body.is_some() {
        for (actor, mut visibility) in &mut locomotion {
            if !actor.is_hero {
                *visibility = Visibility::Hidden;
            }
        }
    }
    let tint = hero_cue(sim, Some(combat), HeroId::Donut);
    for (kind, mut transform, mut sprite, mut visibility) in &mut sprites {
        let piece: Option<Piece> = match *kind {
            BlinkSprite::Body => body.map(|(index, at, flip)| {
                sprite.color = tint;
                Piece { at, index, flip }
            }),
            BlinkSprite::Puddle(i) => puddles.get(i).map(|p| Piece {
                at: p.at,
                index: p.index,
                flip: false,
            }),
            BlinkSprite::Dash(i) => dashes.get(i).map(|p| Piece {
                at: p.at,
                index: p.index,
                flip: p.flip,
            }),
            BlinkSprite::Head => head.as_ref().map(|p| Piece {
                at: p.at,
                index: p.index,
                flip: p.flip,
            }),
        };
        match piece {
            Some(p) => {
                transform.translation.x = p.at.0;
                transform.translation.y = p.at.1;
                sprite.flip_x = p.flip;
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = p.index;
                }
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}

/// World centre of a streak piece `width` px wide whose near edge is column `edge` and which runs
/// toward `dir` (+1 right, -1 left), on the row `y` (world, y up). The pieces are 3 rows tall: the
/// middle row is the streak's own row.
fn streak_center(edge: f32, dir: i32, y: f32, width: i32) -> (f32, f32) {
    let x = edge + dir as f32 * width as f32 / 2.0;
    (x, y + 0.5)
}
