//! Common world schema of the camera lab rooms, loaded from
//! `tools/camera-lab/scenarios.json`.
//!
//! The browser lab's numbers are hypotheses; this module turns them into one
//! schema with
//!
//! * **render bounds** ([`Room::render`]): the full drawn layout. The camera is
//!   clamped inside it, so a view never shows void. Always whole 16 px tiles;
//! * **collision / play bounds** ([`Room::play_x`], [`Room::floor_y`], the
//!   platforms, steps and props): what the simulation collides with. Render and
//!   collision data are separate: the art is drawn far beyond anything an actor
//!   can touch;
//! * tagged destructible props and the **protected** floor, steps, platforms
//!   and exit that no prop or enemy can remove.
//!
//! All coordinates here are layout pixels: origin top-left, y down (the
//! `env-kit` convention). The simulation converts to y-up at its boundary.
//!
//! Snapping (recorded, not hidden): the floor walking line is moved to the
//! nearest `16 n + 4` (the deep-floor contract: a `deck_front` tile's collider
//! top is 4 px below its tile row), every y of the room moves by the same
//! offset, and platform tops are snapped to whole tiles above the floor.

use std::path::Path;

pub const TILE: i32 = 16;
/// Wall columns at both ends of every room (drawn and collidable).
pub const WALL_PX: i32 = 16;
/// Authored access steps: width and rise (px). Rise stays below Carl's jump.
pub const STEP_W: i32 = 32;
pub const STEP_RISE: i32 = 32;
/// Platforms up to this many px above the floor get access steps for Carl;
/// higher ones are Donut-only perches reached through other platforms.
pub const STEP_ACCESS_MAX_RISE: i32 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl IRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    pub const fn right(self) -> i32 {
        self.x + self.w
    }
    pub const fn bottom(self) -> i32 {
        self.y + self.h
    }
    pub fn overlaps(self, o: Self) -> bool {
        self.x < o.right() && self.right() > o.x && self.y < o.bottom() && self.bottom() > o.y
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    DryCatacomb,
    HeatCatacomb,
    Workshop,
    Hoarder,
    Gym,
    Arena,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnemyKind {
    Goblin,
    Llama,
    Engineer,
    Scatterer,
    Hoarder,
    Trog,
    Juicer,
    Ball,
}

impl EnemyKind {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "goblin" => Self::Goblin,
            "llama" => Self::Llama,
            "engineer" => Self::Engineer,
            "scatterer" => Self::Scatterer,
            "hoarder" => Self::Hoarder,
            "trog" => Self::Trog,
            "juicer" => Self::Juicer,
            "ball" => Self::Ball,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    Crate,
    WeakWall,
    Support,
    HangingWeight,
    Junk,
    Weight,
    Gate,
}

impl PropKind {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "crate" => Self::Crate,
            "weak-wall" => Self::WeakWall,
            "support" => Self::Support,
            "hanging-weight" => Self::HangingWeight,
            "junk" => Self::Junk,
            "weight" => Self::Weight,
            "gate" => Self::Gate,
            _ => return None,
        })
    }

    /// Hit points of a destructible prop (hits of 1 damage).
    pub fn hp(self) -> u8 {
        match self {
            Self::Crate | Self::Junk | Self::HangingWeight => 2,
            Self::WeakWall | Self::Support | Self::Weight => 3,
            Self::Gate => 0,
        }
    }

    /// Blocks actors and enemies while it stands. A hanging weight is above the
    /// lane; the gate blocks only while closed (see the simulation).
    pub fn solid(self) -> bool {
        !matches!(self, Self::HangingWeight | Self::Gate)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Prop {
    pub id: usize,
    pub kind: PropKind,
    pub rect: IRect,
    pub breakable: bool,
    pub hp: u8,
    /// Index of the hanging weight a support lets fall (a tagged, predetermined
    /// structural link, docs/03-tech/destruction.md L3).
    pub link: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Platform {
    /// Walkable slab, y-down; `y` is the standing surface.
    pub rect: IRect,
    /// Carl can reach it (authored steps) without a jump above his own.
    pub carl_access: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnemyDef {
    pub kind: EnemyKind,
    pub x: i32,
    /// Standing on this platform instead of the floor.
    pub perch: Option<usize>,
    pub hp: Option<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Hazard {
    pub rect: IRect,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub act: u8,
    pub setting: Setting,
    /// Render bounds (the whole drawn layout) in px, multiples of [`TILE`].
    pub render: (i32, i32),
    /// Walking line (y of the standing surface) of the floor.
    pub floor_y: i32,
    /// Walkable x range (between the wall columns).
    pub play_x: (i32, i32),
    pub spawn_carl: i32,
    pub spawn_donut: i32,
    pub platforms: Vec<Platform>,
    /// Authored access steps (full-height masonry blocks from the floor).
    pub steps: Vec<IRect>,
    pub props: Vec<Prop>,
    pub enemies: Vec<EnemyDef>,
    pub hazards: Vec<Hazard>,
    /// x ranges (`[x0, x1]`) of plain masonry backwall: the composer places no
    /// ground arch bay over them (DEMO-01: behind the rotten shore). Empty for
    /// every lab room.
    pub solid_backwall: Vec<(i32, i32)>,
    pub goal_x: i32,
    /// DM-Q27: a collision-only lane ceiling whose underside is this many px
    /// above the walking line (the HUD clearance of the room). `None` (the
    /// field is absent) means no ceiling. Read by the room's loader, never by
    /// the composer: nothing is drawn.
    pub ceiling_px: Option<i32>,
    /// The protected exit: a walk-in region at the end of the floor.
    pub exit: IRect,
    /// How far everything of the scenario moved in y to meet the tile grid.
    pub snap_dy: i32,
    pub purpose: String,
    pub expected: String,
    pub coop: String,
}

fn int(v: &serde_json::Value, key: &str) -> Result<i32, String> {
    v[key]
        .as_i64()
        .map(|n| n as i32)
        .ok_or_else(|| format!("missing integer {key}"))
}

fn text(v: &serde_json::Value, key: &str) -> Result<String, String> {
    v[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("missing string {key}"))
}

fn setting_of(name: &str) -> Result<Setting, String> {
    Ok(match name {
        "dry-catacomb" => Setting::DryCatacomb,
        "heat-catacomb" => Setting::HeatCatacomb,
        "workshop" => Setting::Workshop,
        "hoarder" => Setting::Hoarder,
        "gym" => Setting::Gym,
        "arena" => Setting::Arena,
        other => return Err(format!("unknown setting {other}")),
    })
}

fn round_to(value: i32, quantum: i32) -> i32 {
    ((value as f32 / quantum as f32).round() as i32) * quantum
}

/// Parse the scenarios file (path or text) into rooms.
pub fn parse_rooms(json: &str) -> Result<Vec<Room>, String> {
    let root: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let list = root["scenarios"].as_array().ok_or("no scenarios")?;
    list.iter().map(parse_room).collect()
}

pub fn load_rooms(repo_root: &Path) -> Result<Vec<Room>, String> {
    let path = repo_root.join("tools/camera-lab/scenarios.json");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_rooms(&text)
}

fn parse_room(s: &serde_json::Value) -> Result<Room, String> {
    let id = text(s, "id")?;
    let ctx = |e: String| format!("{id}: {e}");
    let width = int(s, "width").map_err(ctx)?;
    let height = int(s, "height").map_err(ctx)?;
    let floor = int(s, "floor").map_err(ctx)?;
    // Whole tiles, and the floor on the 16 n + 4 line of the deep-floor tiles.
    let floor_y = round_to(floor - 4, TILE) + 4;
    let dy = floor_y - floor;
    let render_w = (width + TILE - 1) / TILE * TILE;
    let render_h = ((height + dy + TILE - 1) / TILE * TILE).max(floor_y + 4 * TILE);

    let mut platforms = Vec::new();
    for p in s["platforms"]
        .as_array()
        .ok_or_else(|| ctx("platforms".into()))?
    {
        let x = round_to(int(p, "x").map_err(ctx)?, TILE);
        let top = int(p, "y").map_err(ctx)? + dy;
        let w = round_to(int(p, "w").map_err(ctx)?, TILE).max(2 * TILE);
        // Whole tiles above the walking line.
        let rise = round_to(floor_y - top, TILE).max(TILE);
        let rect = IRect::new(x, floor_y - rise, w, 2 * TILE);
        platforms.push(Platform {
            rect,
            carl_access: rise <= STEP_ACCESS_MAX_RISE,
        });
    }

    let mut props: Vec<Prop> = Vec::new();
    for (index, p) in s["props"]
        .as_array()
        .ok_or_else(|| ctx("props".into()))?
        .iter()
        .enumerate()
    {
        let kind = PropKind::parse(p["kind"].as_str().unwrap_or(""))
            .ok_or_else(|| ctx("prop kind".into()))?;
        let breakable = p["breakable"].as_bool().unwrap_or(false);
        let rect = IRect::new(
            int(p, "x").map_err(ctx)?,
            int(p, "y").map_err(ctx)? + dy,
            int(p, "w").map_err(ctx)?,
            int(p, "h").map_err(ctx)?,
        );
        props.push(Prop {
            id: index,
            kind,
            rect,
            breakable,
            hp: if breakable { kind.hp() } else { 0 },
            link: None,
        });
    }
    // A support lets the nearest hanging weight within 160 px fall.
    let weights: Vec<(usize, i32)> = props
        .iter()
        .filter(|p| p.kind == PropKind::HangingWeight)
        .map(|p| (p.id, p.rect.x + p.rect.w / 2))
        .collect();
    for prop in props.iter_mut().filter(|p| p.kind == PropKind::Support) {
        let cx = prop.rect.x + prop.rect.w / 2;
        prop.link = weights
            .iter()
            .filter(|(_, wx)| (wx - cx).abs() <= 160)
            .min_by_key(|(_, wx)| ((wx - cx).abs(), *wx))
            .map(|(id, _)| *id);
    }

    let mut enemies = Vec::new();
    for e in s["enemies"]
        .as_array()
        .ok_or_else(|| ctx("enemies".into()))?
    {
        let kind = EnemyKind::parse(e["kind"].as_str().unwrap_or(""))
            .ok_or_else(|| ctx("enemy kind".into()))?;
        let x = int(e, "x").map_err(ctx)?;
        let perch = e["on_y"].as_i64().map(|on| {
            let top = on as i32 + dy;
            // The platform whose original surface this enemy stands on.
            platforms
                .iter()
                .enumerate()
                .filter(|(_, p)| x >= p.rect.x && x < p.rect.right())
                .min_by_key(|(_, p)| (p.rect.y - top).abs())
                .map(|(i, _)| i)
        });
        enemies.push(EnemyDef {
            kind,
            x,
            perch: perch.flatten(),
            hp: e["hp"].as_u64().map(|n| n as u8),
        });
    }

    let mut hazards = Vec::new();
    for h in s["hazards"].as_array().into_iter().flatten() {
        hazards.push(Hazard {
            rect: IRect::new(
                int(h, "x").map_err(ctx)?,
                floor_y - int(h, "h").map_err(ctx)?,
                int(h, "w").map_err(ctx)?,
                int(h, "h").map_err(ctx)?,
            ),
            kind: text(h, "kind").map_err(ctx)?,
        });
    }

    let mut solid_backwall = Vec::new();
    for range in s["solid_backwall"].as_array().into_iter().flatten() {
        let pair = range.as_array().filter(|a| a.len() == 2);
        let (Some(x0), Some(x1)) = (
            pair.and_then(|a| a[0].as_i64()),
            pair.and_then(|a| a[1].as_i64()),
        ) else {
            return Err(ctx("solid_backwall needs [x0, x1] pairs".into()));
        };
        solid_backwall.push((x0 as i32, x1 as i32));
    }

    let ceiling_px = match s.get("ceiling_px") {
        None => None,
        Some(v) => match v.as_i64() {
            Some(n) if n > 0 => Some(n as i32),
            _ => return Err(ctx("ceiling_px must be a positive integer".into())),
        },
    };
    let goal_x = int(s, "goal_x").map_err(ctx)?;
    let mut room = Room {
        id: id.clone(),
        name: text(s, "name").map_err(ctx)?,
        subtitle: text(s, "subtitle").map_err(ctx)?,
        act: int(s, "act").map_err(ctx)? as u8,
        setting: setting_of(&text(s, "setting").map_err(ctx)?).map_err(ctx)?,
        render: (render_w, render_h),
        floor_y,
        play_x: (WALL_PX, render_w - WALL_PX),
        spawn_carl: int(&s["spawn"], "carl").map_err(ctx)?,
        spawn_donut: int(&s["spawn"], "donut").map_err(ctx)?,
        platforms,
        steps: Vec::new(),
        props,
        enemies,
        hazards,
        solid_backwall,
        goal_x,
        ceiling_px,
        exit: IRect::new(goal_x - 24, floor_y - 96, 64, 96),
        snap_dy: dy,
        purpose: text(s, "purpose").map_err(ctx)?,
        expected: text(s, "expected").map_err(ctx)?,
        coop: text(s, "coop").map_err(ctx)?,
    };
    room.exit.x = room.exit.x.min(room.play_x.1 - room.exit.w);
    room.steps = generate_steps(&room);
    Ok(room)
}

/// Authored access steps beside every Carl-accessible platform: a staircase of
/// [`STEP_W`] wide, [`STEP_RISE`] high masonry blocks leading up to the
/// platform from the side with free floor (left first). Their top surfaces are
/// walking surfaces; they are protected (never destructible).
fn generate_steps(room: &Room) -> Vec<IRect> {
    let mut steps: Vec<IRect> = Vec::new();
    let blocked = |r: IRect, steps: &[IRect]| -> bool {
        let lane = IRect::new(r.x, room.floor_y - r.h, r.w, r.h);
        r.x < room.play_x.0
            || r.right() > room.play_x.1
            || room.props.iter().any(|p| p.rect.overlaps(lane))
            || steps.iter().any(|s| s.overlaps(r))
            || room
                .platforms
                .iter()
                .any(|p| p.rect.overlaps(IRect::new(r.x, r.y, r.w, r.h)))
    };
    for platform in room.platforms.iter().filter(|p| p.carl_access) {
        let rise = room.floor_y - platform.rect.y;
        let count = (rise - 1) / STEP_RISE; // steps below the platform's own top
        let build = |dir: i32, steps: &[IRect]| -> Option<Vec<IRect>> {
            let mut out = Vec::new();
            for k in 1..=count {
                let height = rise - STEP_RISE * k;
                if height <= 0 {
                    break;
                }
                let x = if dir < 0 {
                    platform.rect.x - STEP_W * k
                } else {
                    platform.rect.right() + STEP_W * (k - 1)
                };
                let r = IRect::new(x, room.floor_y - height, STEP_W, height);
                if blocked(r, steps) {
                    return None;
                }
                out.push(r);
            }
            Some(out)
        };
        let chosen = build(-1, &steps)
            .or_else(|| build(1, &steps))
            .unwrap_or_default();
        steps.extend(chosen);
    }
    steps
}

// --------------------------------------------------------------- reachability

/// What an actor can do to get around, for the route checks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mobility {
    /// Largest rise (px) of a standing jump.
    pub jump_h: i32,
    /// Largest horizontal gap (px) a jump can bridge.
    pub reach_x: i32,
    /// Breaks any destructible prop (Carl) or only small ones (Donut).
    pub breaks_all: bool,
}

pub const CARL_MOBILITY: Mobility = Mobility {
    jump_h: 92,
    reach_x: 120,
    breaks_all: true,
};
pub const DONUT_MOBILITY: Mobility = Mobility {
    jump_h: 171,
    reach_x: 200,
    breaks_all: false,
};

#[derive(Debug, Clone, Copy)]
struct Surface {
    x0: i32,
    x1: i32,
    top: i32,
}

/// A route state of the props: all intact (the worst case for a blocked lane) or
/// all destroyed (the worst case for a lost support).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropState {
    AllIntact,
    AllBroken,
}

impl Room {
    pub fn render_w(&self) -> i32 {
        self.render.0
    }
    pub fn render_h(&self) -> i32 {
        self.render.1
    }
    pub fn prop_by_kind(&self, kind: PropKind) -> impl Iterator<Item = &Prop> {
        self.props.iter().filter(move |p| p.kind == kind)
    }

    /// Floor surfaces split by the props an actor can neither jump nor break.
    fn floor_surfaces(&self, m: Mobility, state: PropState) -> Vec<Surface> {
        let mut cuts: Vec<(i32, i32)> = Vec::new();
        for p in &self.props {
            if !p.kind.solid() && p.kind != PropKind::Gate {
                continue;
            }
            let height = self.floor_y - p.rect.y;
            if height <= m.jump_h - 6 {
                continue;
            }
            let removable = match p.kind {
                // The gate opens on a cycle (a moving gate has an open phase).
                PropKind::Gate => true,
                _ => {
                    p.breakable
                        && (m.breaks_all || matches!(p.kind, PropKind::Crate | PropKind::Junk))
                }
            };
            if state == PropState::AllBroken && p.breakable {
                continue;
            }
            if !removable {
                cuts.push((p.rect.x, p.rect.right()));
            }
        }
        cuts.sort_unstable();
        let mut out = Vec::new();
        let mut x = self.play_x.0;
        for (c0, c1) in cuts {
            if c0 > x {
                out.push(Surface {
                    x0: x,
                    x1: c0,
                    top: self.floor_y,
                });
            }
            x = x.max(c1);
        }
        if x < self.play_x.1 {
            out.push(Surface {
                x0: x,
                x1: self.play_x.1,
                top: self.floor_y,
            });
        }
        out
    }

    /// Can an actor with mobility `m` get from `from_x` on the floor to the exit
    /// (the exit's floor surface), over floor, steps and platforms?
    pub fn exit_reachable(&self, m: Mobility, state: PropState, from_x: i32) -> bool {
        let mut surfaces = self.floor_surfaces(m, state);
        let floor_count = surfaces.len();
        for s in &self.steps {
            surfaces.push(Surface {
                x0: s.x,
                x1: s.right(),
                top: s.y,
            });
        }
        for p in &self.platforms {
            surfaces.push(Surface {
                x0: p.rect.x,
                x1: p.rect.right(),
                top: p.rect.y,
            });
        }
        let start = surfaces[..floor_count]
            .iter()
            .position(|s| from_x >= s.x0 && from_x < s.x1);
        let Some(start) = start else { return false };
        let goal = self.goal_x.min(self.exit.right() - 1);
        let target = surfaces[..floor_count]
            .iter()
            .position(|s| goal >= s.x0 && goal < s.x1);
        let Some(target) = target else { return false };
        let mut seen = vec![false; surfaces.len()];
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(i) = stack.pop() {
            if i == target {
                return true;
            }
            let a = surfaces[i];
            for (j, b) in surfaces.iter().enumerate() {
                if seen[j] {
                    continue;
                }
                let gap = (b.x0 - a.x1).max(a.x0 - b.x1).max(0);
                let rise = a.top - b.top; // positive: b is higher
                let ok = if rise > 0 {
                    rise <= m.jump_h - 6 && gap <= m.reach_x
                } else {
                    // Walk off and drop: a short gap is enough.
                    gap <= 48
                };
                if ok {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        false
    }

    /// Can an actor standing on any platform or step get back to the floor? (A
    /// drop is always possible when the surface is not enclosed; this checks the
    /// platform ends are not walled in by a wall column.)
    pub fn platforms_dropable(&self) -> bool {
        self.platforms.iter().all(|p| {
            let left_free = p.rect.x - 48 >= self.play_x.0;
            let right_free = p.rect.right() + 48 <= self.play_x.1;
            left_free || right_free
        })
    }

    /// Height of a prop above the floor.
    pub fn prop_height(&self, p: &Prop) -> i32 {
        self.floor_y - p.rect.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rooms() -> Vec<Room> {
        load_rooms(Path::new(env!("CARGO_MANIFEST_DIR"))).expect("scenarios.json loads")
    }

    #[test]
    fn six_rooms_load_with_english_text() {
        let rooms = rooms();
        assert_eq!(rooms.len(), 6);
        let ids: Vec<&str> = rooms.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "ROOM-01", "ROOM-02", "ROOM-03", "ROOM-04", "ROOM-05", "ROOM-06"
            ]
        );
        for r in &rooms {
            for t in [&r.name, &r.subtitle, &r.purpose, &r.expected, &r.coop] {
                assert!(
                    t.chars()
                        .all(|c| c.is_ascii() || c == '\u{b7}' || c == '\u{d7}'),
                    "{}: non-English text {t:?}",
                    r.id
                );
            }
        }
    }

    #[test]
    fn render_bounds_are_whole_tiles_and_larger_than_the_max_camera_view() {
        for r in rooms() {
            assert_eq!(r.render.0 % TILE, 0);
            assert_eq!(r.render.1 % TILE, 0);
            // Even the 16:10 1.75x view (1120x700) fits.
            assert!(
                r.render.0 >= 1120 && r.render.1 >= 700,
                "{}: {:?}",
                r.id,
                r.render
            );
            // The floor is on the deep-floor line.
            assert_eq!((r.floor_y - 4) % TILE, 0, "{}", r.id);
            // Collision floor lies inside the render bounds with art below it.
            assert!(r.floor_y + 4 * TILE <= r.render.1);
            // The upper architecture above the floor is taller than any view.
            assert!(r.floor_y >= 560, "{}", r.id);
        }
    }

    #[test]
    fn platforms_props_and_steps_are_inside_the_play_area_and_do_not_overlap() {
        for r in rooms() {
            for p in &r.platforms {
                assert!(
                    p.rect.x >= r.play_x.0 && p.rect.right() <= r.play_x.1,
                    "{} platform",
                    r.id
                );
                assert_eq!(
                    (r.floor_y - p.rect.y) % TILE,
                    0,
                    "{} platform not on the grid",
                    r.id
                );
            }
            for p in &r.props {
                assert!(
                    p.rect.x >= r.play_x.0 && p.rect.right() <= r.play_x.1,
                    "{} prop",
                    r.id
                );
            }
            for (i, a) in r.steps.iter().enumerate() {
                for b in &r.steps[i + 1..] {
                    assert!(!a.overlaps(*b), "{}: steps overlap", r.id);
                }
                for p in &r.props {
                    let lane = IRect::new(a.x, r.floor_y - a.h, a.w, a.h);
                    assert!(!lane.overlaps(p.rect), "{}: step on prop {}", r.id, p.id);
                }
            }
            // Every Carl-accessible platform got its steps: the highest step is
            // within Carl's jump of the platform.
            for p in r.platforms.iter().filter(|p| p.carl_access) {
                let rise = r.floor_y - p.rect.y;
                if rise > CARL_MOBILITY.jump_h - 6 {
                    assert!(!r.steps.is_empty(), "{}: platform needs steps", r.id);
                }
            }
        }
    }

    #[test]
    fn the_protected_exit_is_reachable_for_each_actor_in_every_prop_state() {
        for r in rooms() {
            for (name, m, from) in [
                ("carl", CARL_MOBILITY, r.spawn_carl),
                ("donut", DONUT_MOBILITY, r.spawn_donut),
            ] {
                for state in [PropState::AllIntact, PropState::AllBroken] {
                    assert!(
                        r.exit_reachable(m, state, from),
                        "{}: {name} cannot reach the exit with props {state:?}",
                        r.id
                    );
                }
            }
            assert!(r.platforms_dropable(), "{}", r.id);
            // The exit sits on the protected floor, not on a prop or inside a wall.
            assert!(r.exit.right() <= r.play_x.1);
            for p in &r.props {
                assert!(
                    !p.rect.overlaps(r.exit),
                    "{}: prop {} on the exit",
                    r.id,
                    p.id
                );
            }
        }
    }

    #[test]
    fn only_authored_props_are_destructible_and_the_gate_is_not() {
        for r in rooms() {
            for p in &r.props {
                assert_eq!(
                    p.breakable,
                    p.kind != PropKind::Gate,
                    "{} prop {}",
                    r.id,
                    p.id
                );
                assert_eq!(p.hp > 0, p.breakable);
            }
        }
        let r3 = &rooms()[2];
        let support = r3
            .props
            .iter()
            .find(|p| p.kind == PropKind::Support)
            .unwrap();
        let weight = r3
            .props
            .iter()
            .find(|p| p.kind == PropKind::HangingWeight)
            .unwrap();
        assert_eq!(
            support.link,
            Some(weight.id),
            "the support holds the weight"
        );
    }

    #[test]
    fn high_platforms_are_marked_donut_only() {
        let rooms = rooms();
        // ROOM-06: the two side platforms get steps, the high central one does not.
        let r6 = &rooms[5];
        assert_eq!(r6.platforms.iter().filter(|p| p.carl_access).count(), 2);
        assert!(!r6.platforms[2].carl_access);
        // A platform Carl cannot reach is still reachable for Donut.
        assert!(r6.exit_reachable(DONUT_MOBILITY, PropState::AllIntact, r6.spawn_donut));
    }
}
