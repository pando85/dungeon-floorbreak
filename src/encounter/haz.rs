//! HAZ-A1: the hazard room's validator rules, appended to BOIL-A core's encounter
//! validator (`docs/01-design/enemy-gameplay-slime-rot.md`, section 15, rules 1
//! to 10). A failure names the rule and the ids ([`EncounterError::Haz`]).
//!
//! The rules read the sidecar's HAZ appends: the `acid_patches` array (never the
//! room's `hazards`, so BOIL-A core's `RosterInHazard` never sees a patch and
//! needs no exemption), the `rot_sticker` prop kind with its `track`, the slime
//! roster fields and the `pockets`. The checks run in layout px (y down); the
//! sticker geometry is converted to the world frame (heights above the walking
//! line, y up) of [`crate::rot_sticker`] with [`StickerLayout::world_track`].
//!
//! Disclosed limits: the jump check of rules 1 and 2 simulates the real
//! discrete arc (`velocity set, gravity, move`, the same constants the scene
//! uses) from a take-off zone of [`TAKEOFF_ZONE_PX`] px with a flat floor and
//! the ceilings of the room (platform undersides, a ceiling sticker's surface,
//! the lane `ceiling_px`); it does not trace floor solids in the air. Patches
//! must lie on the floor lane (a patch on a platform is not in v1). The
//! swept-blast checks use the bounding box of the sweep, which is conservative.

use super::*;

/// The three jump classes a patch must be clearable by (rule 1): Carl, base
/// Donut and boosted Donut.
#[derive(Debug, Clone, Copy)]
pub(super) struct JumpClass {
    pub name: &'static str,
    /// Horizontal speed in px per tick while the stick is held.
    pub run: f32,
    pub launch: f32,
    /// Half-width of the hurtbox that touches the acid.
    pub half_hurt: f32,
}

pub(super) fn jump_classes() -> [JumpClass; 3] {
    use crate::prototype::{JUMP_SPEED, RUN_SPEED, TICK_SECONDS};
    let a = crate::agility::tuning();
    let boost = f32::from(crate::donut_prog::DonutProgTuning::active().dpq14_boost_launch_px_s);
    [
        JumpClass {
            name: "carl",
            run: RUN_SPEED * TICK_SECONDS,
            launch: JUMP_SPEED,
            half_hurt: 12.0,
        },
        JumpClass {
            name: "donut_base",
            run: a.dm_q01_run_speed * TICK_SECONDS,
            launch: a.dm_q02_jump_speed,
            half_hurt: 10.0,
        },
        JumpClass {
            name: "donut_boosted",
            run: a.dm_q01_run_speed * TICK_SECONDS,
            launch: boost,
            half_hurt: 10.0,
        },
    ]
}

/// A ceiling over the lane: x range and the height of its underside above the
/// walking line.
#[derive(Debug, Clone, Copy)]
pub(super) struct Ceiling {
    pub x0: f32,
    pub x1: f32,
    pub underside: f32,
}

/// Simulate one standing-start jump with the hero's hurtbox front edge `gap` px
/// before `patch`, the stick held toward it. `Err` names the first overlap of
/// the hurtbox with any patch below the volume's top.
pub(super) fn jump_clears(
    class: JumpClass,
    gap: i32,
    patch: (i32, i32),
    all: &[(i32, i32)],
    ceilings: &[Ceiling],
    volume_h: f32,
) -> Result<(), String> {
    use crate::prototype::{GRAVITY, HERO_HALF_HEIGHT, HERO_HALF_WIDTH, TICK_SECONDS};
    let mut xc = patch.0 as f32 - class.half_hurt - gap as f32;
    let (mut y, mut vy) = (0.0_f32, class.launch);
    for tick in 1..=240 {
        xc += class.run;
        vy -= GRAVITY * TICK_SECONDS;
        let mut next = y + vy * TICK_SECONDS;
        if vy > 0.0 {
            let (bl, br) = (xc - HERO_HALF_WIDTH, xc + HERO_HALF_WIDTH);
            let top_now = y + 2.0 * HERO_HALF_HEIGHT;
            let low = ceilings
                .iter()
                .filter(|c| bl < c.x1 && br > c.x0 && c.underside >= top_now - 0.01)
                .map(|c| c.underside)
                .fold(f32::INFINITY, f32::min);
            if next + 2.0 * HERO_HALF_HEIGHT >= low {
                next = low - 2.0 * HERO_HALF_HEIGHT;
                vy = 0.0;
            }
        }
        y = next;
        let landed = y <= 0.0;
        if landed {
            y = 0.0;
        }
        for &(p0, p1) in all {
            if xc + class.half_hurt > p0 as f32 && xc - class.half_hurt < p1 as f32 && y < volume_h
            {
                return Err(format!(
                    "{} overlaps the patch {p0}..{p1} on jump tick {tick} (feet {y:.1}, gap {gap})",
                    class.name
                ));
            }
        }
        if landed {
            return Ok(());
        }
    }
    Err(format!("{} never lands", class.name))
}

/// Distance between two axis-aligned or general segments (f64).
pub(super) fn segment_distance(
    a0: (f64, f64),
    a1: (f64, f64),
    b0: (f64, f64),
    b1: (f64, f64),
) -> f64 {
    fn point_seg(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dy * dy;
        let t = if len2 == 0.0 {
            0.0
        } else {
            (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
        };
        ((p.0 - (a.0 + t * dx)).powi(2) + (p.1 - (a.1 + t * dy)).powi(2)).sqrt()
    }
    fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    }
    let (d1, d2) = (cross(b0, b1, a0), cross(b0, b1, a1));
    let (d3, d4) = (cross(a0, a1, b0), cross(a0, a1, b1));
    if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
        return 0.0;
    }
    [
        point_seg(a0, b0, b1),
        point_seg(a1, b0, b1),
        point_seg(b0, a0, a1),
        point_seg(b1, a0, a1),
    ]
    .into_iter()
    .fold(f64::INFINITY, f64::min)
}

/// Distinct supplier kinds among `kinds`: HAZ rule 9 allows at most one kind
/// that supplies `CompanionView.boss` in a roster.
pub(super) fn distinct_suppliers<K: PartialEq + Copy>(
    kinds: impl IntoIterator<Item = K>,
    supplies: impl Fn(K) -> bool,
) -> usize {
    let mut seen: Vec<K> = Vec::new();
    for k in kinds.into_iter().filter(|k| supplies(*k)) {
        if !seen.contains(&k) {
            seen.push(k);
        }
    }
    seen.len()
}

/// HAZ rule 9: the failure when more than one kind supplies the companion
/// `boss` input.
pub(super) fn supplier_conflict<K: PartialEq + Copy>(
    kinds: impl IntoIterator<Item = K>,
    supplies: impl Fn(K) -> bool,
) -> Option<EncounterError> {
    let n = distinct_suppliers(kinds, supplies);
    (n > 1).then(|| {
        haz(
            9,
            &[],
            format!("{n} kinds supply the companion `boss` input, at most 1"),
        )
    })
}

pub(super) struct HazInput<'a> {
    pub raw: &'a RawEncounter,
    pub ctx: &'a Context,
    pub goal: Goal,
    pub props: &'a [Prop],
    /// The parsed hazards (DR-B: a frame's footprint is its owned falling mass).
    pub hazards: &'a [Hazard],
    pub roster: &'a [RosterEntry],
    pub protected: [(&'static str, IRect); 3],
}

fn haz(rule: u8, ids: &[&str], reason: impl Into<String>) -> EncounterError {
    EncounterError::Haz {
        rule,
        ids: ids.iter().map(|s| (*s).to_owned()).collect(),
        reason: reason.into(),
    }
}

/// The x range `[left, right)` of a roster entry's body on the floor lane.
fn body_x(e: &RosterEntry) -> (i32, i32) {
    match e.kind {
        RosterKind::Slime => {
            let b = &crate::slime::active().hazq05_hurtbox_idle;
            let (x0, x1) = (i32::from(b.x0), i32::from(b.x1));
            if e.facing > 0 {
                (e.x + x0, e.x + x1)
            } else {
                (e.x - x1, e.x - x0)
            }
        }
        RosterKind::Room(_) => (e.x - ENEMY_BODY.0 / 2, e.x + ENEMY_BODY.0 / 2),
        // TROG-A: a sleeper's idle hurtbox (it spawns facing left).
        RosterKind::Trog(species) => {
            let b = &crate::trog::TrogTuning::active().species(species).hurt_idle;
            (e.x - i32::from(b.x1), e.x - i32::from(b.x0))
        }
        // DOZER-CORE: the parked machine, push box rear to spike tips, with its
        // wheel front at the entry's x.
        RosterKind::Dozer => {
            let t = crate::dozer::DozerTuning::active();
            let o = e.x - (t.dzq01_wheel_axle.x + i32::from(t.dzq01_wheel_r_px));
            (o + t.dzq02_push_box.x0, o + t.dzq03_impact_collider.x1)
        }
    }
}

/// Run every HAZ rule; returns the acid specs and pockets for the encounter.
pub(super) fn check(
    inp: &HazInput<'_>,
    errs: &mut Vec<EncounterError>,
) -> (Vec<AcidPatchSpec>, Vec<Pocket>) {
    let room = &inp.ctx.room;
    let floor = room.floor_y;
    let slime_t = crate::slime::active();
    let rot_t = crate::rot_sticker::active();
    let raw = inp.raw;
    let slab = raw.platform_slab_px;

    // ---------------------------------------------------- schema: patches
    let mut specs: Vec<AcidPatchSpec> = Vec::new();
    let mut seen: BTreeSet<u16> = BTreeSet::new();
    for p in &raw.acid_patches {
        let id = p.id.to_string();
        if !seen.insert(p.id) {
            errs.push(haz(0, &[&id], "duplicate acid patch id"));
            continue;
        }
        if p.x0 >= p.x1 {
            errs.push(haz(0, &[&id], "x0 must be below x1"));
            continue;
        }
        let linked = match &p.linked_slime {
            None => None,
            Some(name) => match inp
                .roster
                .iter()
                .position(|e| e.id == *name && e.kind.is_slime())
            {
                Some(i) => Some(i as u8),
                None => {
                    errs.push(haz(
                        0,
                        &[&id, name],
                        "linked_slime is not a slime of the roster",
                    ));
                    continue;
                }
            },
        };
        specs.push(AcidPatchSpec {
            id: p.id,
            x0: p.x0,
            x1: p.x1,
            floor_y: p.floor_y,
            linked_slime: linked,
        });
    }
    // The slime's `linked_pool` and the patch's `linked_slime` name each other.
    for e in inp.roster.iter().filter(|e| e.kind.is_slime()) {
        if let Some(pool) = &e.linked_pool {
            let ok = specs.iter().any(|p| {
                p.id.to_string() == *pool
                    && inp
                        .roster
                        .get(usize::from(p.linked_slime.unwrap_or(u8::MAX)))
                        .is_some_and(|s| s.id == e.id)
            });
            if !ok {
                errs.push(haz(
                    0,
                    &[&e.id, pool],
                    "linked_pool must name an acid patch whose linked_slime names this slime",
                ));
            }
        }
        if (e.x - e.home).abs() > i32::from(slime_t.hazq06_leash_px) {
            errs.push(haz(
                0,
                &[&e.id],
                "home is farther than the leash from the spawn",
            ));
        }
    }

    // --------------------------------------- floor solids and ceilings
    let mut floor_solids: Vec<IRect> = room.steps.clone();
    let mut ceilings: Vec<Ceiling> = Vec::new();
    for p in &room.platforms {
        if p.rect.bottom() >= floor {
            floor_solids.push(p.rect);
        } else {
            ceilings.push(Ceiling {
                x0: p.rect.x as f32,
                x1: p.rect.right() as f32,
                underside: ((floor - p.rect.y) - slab) as f32,
            });
        }
    }
    for prop in inp.props.iter().filter(|p| p.kind == PropKind::RotSticker) {
        if let Some(t) = prop.track.filter(|t| t.orientation == Orientation::Ceiling) {
            ceilings.push(Ceiling {
                x0: t.surface.x as f32,
                x1: t.surface.right() as f32,
                underside: (floor - t.surface.bottom()) as f32,
            });
        }
    }
    if let Some(c) = inp.ctx.ceiling_px {
        ceilings.push(Ceiling {
            x0: f32::NEG_INFINITY,
            x1: f32::INFINITY,
            underside: c as f32,
        });
    }
    let all_patch_x: Vec<(i32, i32)> = specs.iter().map(|p| (p.x0, p.x1)).collect();
    let volume_h = f32::from(slime_t.hazq18_volume_height_px);
    let classes = jump_classes();

    // ------------------------------------------------ rule 1: the patches
    // Per class: whether every patch is clearable (rules 2 and 7).
    let mut clearable = [true; 3];
    for p in &specs {
        let id = p.id.to_string();
        let w = p.x1 - p.x0;
        let (min, max) = (
            i32::from(slime_t.hazq18_width_min_px),
            i32::from(slime_t.hazq18_width_max_px),
        );
        if w < min || w > max {
            errs.push(haz(1, &[&id], format!("width {w} is not in {min}..={max}")));
        }
        if p.floor_y != floor {
            errs.push(haz(
                1,
                &[&id],
                "a patch lies on the floor lane (a patch on a platform is not in v1)",
            ));
            continue;
        }
        if p.x0 < room.play_x.0 || p.x1 > room.play_x.1 {
            errs.push(haz(1, &[&id], "outside the play area"));
            continue;
        }
        let rect = IRect::new(
            p.x0,
            floor - i32::from(slime_t.hazq18_volume_height_px),
            w,
            i32::from(slime_t.hazq18_volume_height_px),
        );
        for (zone, r) in inp.protected {
            if overlaps(rect, r) {
                errs.push(haz(1, &[&id], format!("overlaps the protected {zone}")));
            }
        }
        // At least 48 px of dry floor on both sides.
        let mut left = room.play_x.0;
        let mut right = room.play_x.1;
        let others = specs
            .iter()
            .filter(|o| o.id != p.id)
            .map(|o| (o.x0, o.x1))
            .chain(floor_solids.iter().map(|s| (s.x, s.right())));
        for (a, b) in others {
            if b <= p.x0 {
                left = left.max(b);
            }
            if a >= p.x1 {
                right = right.min(a);
            }
        }
        for (side, free) in [("left", p.x0 - left), ("right", right - p.x1)] {
            if free < MIN_DRY_SIDE_PX {
                errs.push(haz(
                    1,
                    &[&id],
                    format!(
                        "only {free} px of dry floor on the {side} side, at least {MIN_DRY_SIDE_PX}"
                    ),
                ));
            }
        }
        // A take-off zone free of solids, then the arc of every class.
        let column = (p.x0 - 2 * 12 - TAKEOFF_ZONE_PX, p.x0);
        if let Some(s) = floor_solids
            .iter()
            .find(|s| s.x < column.1 && s.right() > column.0)
        {
            errs.push(haz(
                1,
                &[&id],
                format!(
                    "a solid at x {}..{} stands in the take-off zone",
                    s.x,
                    s.right()
                ),
            ));
            clearable = [false; 3];
        }
        for (i, class) in classes.iter().enumerate() {
            for gap in 0..=TAKEOFF_ZONE_PX {
                if let Err(why) =
                    jump_clears(*class, gap, (p.x0, p.x1), &all_patch_x, &ceilings, volume_h)
                {
                    errs.push(haz(
                        1,
                        &[&id],
                        format!("not clearable by a standing-start plain jump: {why}"),
                    ));
                    clearable[i] = false;
                    break;
                }
            }
        }
    }

    // ------------------------------------------- rule 2: alternate footing
    for (i, class) in classes.iter().enumerate().take(2) {
        if !clearable[i] {
            errs.push(haz(
                2,
                &[class.name],
                "no route from entry to exit touches no patch (walking and plain jumps only)",
            ));
        }
    }

    // ------------------------------------------------- pockets (rule 5)
    let mut pockets: Vec<Pocket> = Vec::new();
    for pk in &raw.pockets {
        if pk.x0 >= pk.x1 || pk.x0 < room.play_x.0 || pk.x1 > room.play_x.1 {
            errs.push(haz(
                5,
                &[&pk.id],
                "a pocket is a non-empty range inside the play area",
            ));
            continue;
        }
        if pockets.iter().any(|q| q.id == pk.id) {
            errs.push(haz(5, &[&pk.id], "duplicate pocket id"));
            continue;
        }
        for (what, v, cap) in [
            ("max_stickers", pk.max_stickers, POCKET_MAX_STICKERS),
            ("max_slimes", pk.max_slimes, POCKET_MAX_SLIMES),
            ("max_goblins", pk.max_goblins, POCKET_MAX_GOBLINS),
        ] {
            if v > cap {
                errs.push(haz(
                    5,
                    &[&pk.id],
                    format!("{what} {v} is over the cap {cap}"),
                ));
            }
        }
        pockets.push(Pocket {
            id: pk.id.clone(),
            x0: pk.x0,
            x1: pk.x1,
            max_stickers: pk.max_stickers,
            max_slimes: pk.max_slimes,
            max_goblins: pk.max_goblins,
        });
    }

    // ----------------------------------------------- stickers: rules 3, 4, 10
    struct Sticker<'a> {
        prop: &'a Prop,
        dome_lo: (f64, f64),
        dome_hi: (f64, f64),
        sweep_x: (i32, i32),
    }
    let mut stickers: Vec<Sticker<'_>> = Vec::new();
    let radius = i32::from(rot_t.hazq30_outer_radius_px);
    for prop in inp.props.iter().filter(|p| p.kind == PropKind::RotSticker) {
        let id = prop.id.as_str();
        if !prop.optional {
            errs.push(haz(
                10,
                &[id],
                "a rot_sticker is optional: no step may depend on its blast",
            ));
        }
        let Some(lay) = prop.track else { continue };
        let track = match lay.world_track(floor) {
            Ok(t) => t,
            Err(e) => {
                errs.push(haz(3, &[id], e.to_string()));
                continue;
            }
        };
        // The orientation matches the solid it lies on.
        let (lo, hi) = (track.lo, track.hi);
        let surface = lay.surface;
        let on_surface = match lay.orientation {
            Orientation::Ceiling => {
                surface.bottom() == lay.a.1 && lo >= surface.x && hi <= surface.right()
            }
            Orientation::WallLeft => {
                surface.right() == lay.a.0
                    && floor - hi >= surface.y
                    && floor - lo <= surface.bottom()
            }
            Orientation::WallRight => {
                surface.x == lay.a.0 && floor - hi >= surface.y && floor - lo <= surface.bottom()
            }
        };
        // F2 (PR #204 review): the declared surface must be real geometry. A
        // ceiling surface is a room platform's slab, or an authored soffit whose
        // top reaches the lane ceiling (a full-height solid, no standable top);
        // a wall surface must equal a room step or platform (HAZ-A1 has no
        // boundary-wall sticker: a boundary branch would accept an invented
        // wall of any width, re-review FIX-NOW 1; follow-up when a room needs
        // one).
        let solid_match = |r: &IRect| *r == surface;
        let real_surface = match lay.orientation {
            Orientation::Ceiling => {
                room.platforms
                    .iter()
                    .any(|pl| surface == IRect::new(pl.rect.x, pl.rect.y, pl.rect.w, slab))
                    || inp
                        .ctx
                        .ceiling_px
                        .is_some_and(|c| floor - surface.y >= c && surface.y >= 0)
            }
            Orientation::WallLeft | Orientation::WallRight => room
                .steps
                .iter()
                .chain(room.platforms.iter().map(|pl| &pl.rect))
                .any(solid_match),
        };
        if !real_surface {
            errs.push(haz(
                3,
                &[id],
                "the surface matches no room platform slab, step or boundary and is not a full-height soffit reaching ceiling_px",
            ));
        }
        if !on_surface {
            errs.push(haz(
                3,
                &[id],
                format!(
                    "the {} track does not lie on the face of its surface",
                    lay.orientation.name()
                ),
            ));
        }
        if !within_play(room, surface) {
            errs.push(haz(3, &[id], "the surface is outside the play area"));
        }
        // The target is exactly the drawn shell at the start point.
        let shell = track.shell(track.start, &rot_t.hazq24_hurtbox);
        let shell_layout = IRect::new(
            shell.left as i32,
            floor - shell.top() as i32,
            shell.width as i32,
            shell.height as i32,
        );
        if shell_layout != prop.target {
            errs.push(haz(
                3,
                &[id],
                format!(
                    "target must be the shell at the start point: {},{} {}x{}",
                    shell_layout.x, shell_layout.y, shell_layout.w, shell_layout.h
                ),
            ));
        }
        // The dome stays in the camera band and under the lane ceiling.
        let off = rot_t.hazq24_dome_offset_px;
        let (d_lo, d_hi) = (track.dome(lo, off), track.dome(hi, off));
        for (label, d) in [("start end", d_lo), ("far end", d_hi)] {
            let height = d.1 as i32;
            if height > STICKER_BAND_PX {
                errs.push(haz(
                    3,
                    &[id],
                    format!("the dome at the {label} is {height} px up, at most {STICKER_BAND_PX}"),
                ));
            }
            if let Some(c) = inp.ctx.ceiling_px
                && height >= c
            {
                errs.push(haz(
                    3,
                    &[id],
                    format!("the dome at the {label} is not under ceiling_px {c}"),
                ));
            }
            if height <= 0 {
                errs.push(haz(3, &[id], "the dome is not above the walking line"));
            }
        }
        // The swept blast circle stays off the protected rectangles.
        let sweep_x = (
            d_lo.0.min(d_hi.0) as i32 - radius,
            d_lo.0.max(d_hi.0) as i32 + radius,
        );
        let (y_lo, y_hi) = (
            d_lo.1.min(d_hi.1) as i32 - radius,
            d_lo.1.max(d_hi.1) as i32 + radius,
        );
        let sweep = IRect::new(sweep_x.0, floor - y_hi, sweep_x.1 - sweep_x.0, y_hi - y_lo);
        for (zone, r) in inp.protected {
            if overlaps(sweep, r) {
                errs.push(haz(
                    3,
                    &[id],
                    format!("the blast sweep reaches the protected {zone}"),
                ));
            }
        }
        stickers.push(Sticker {
            prop,
            dome_lo: (f64::from(d_lo.0), f64::from(d_lo.1)),
            dome_hi: (f64::from(d_hi.0), f64::from(d_hi.1)),
            sweep_x,
        });
    }
    // Pockets of dry, blast-free floor on each side of every sweep (rule 3).
    let mut blocked: Vec<(i32, i32)> = specs.iter().map(|p| (p.x0, p.x1)).collect();
    blocked.extend(stickers.iter().map(|s| s.sweep_x));
    blocked.sort_unstable();
    let mut merged: Vec<(i32, i32)> = Vec::new();
    for (a, b) in blocked {
        match merged.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    for s in &stickers {
        if let Some(i) = merged
            .iter()
            .position(|m| m.0 <= s.sweep_x.0 && m.1 >= s.sweep_x.1)
        {
            let before = if i == 0 {
                room.play_x.0
            } else {
                merged[i - 1].1
            };
            let after = if i + 1 == merged.len() {
                room.play_x.1
            } else {
                merged[i + 1].0
            };
            for (side, free) in [
                ("left", merged[i].0 - before),
                ("right", after - merged[i].1),
            ] {
                if free < MIN_POCKET_PX {
                    errs.push(haz(
                        3,
                        &[s.prop.id.as_str()],
                        format!("only {free} px of dry, blast-free floor on the {side} side of the sweep, at least {MIN_POCKET_PX}"),
                    ));
                }
            }
        }
    }

    // ------------------------------------------------------ rule 4
    let sep = f64::from(STICKER_SEPARATION_PX);
    for (i, a) in stickers.iter().enumerate() {
        for b in &stickers[i + 1..] {
            let d = segment_distance(a.dome_lo, a.dome_hi, b.dome_lo, b.dome_hi);
            if d <= sep {
                errs.push(haz(
                    4,
                    &[a.prop.id.as_str(), b.prop.id.as_str()],
                    format!(
                        "the tracks are {d:.1} px apart, more than {STICKER_SEPARATION_PX} required"
                    ),
                ));
            }
        }
        for charge in inp.props.iter().filter(|p| p.kind == PropKind::Drum) {
            let c = (
                f64::from(charge.target.x + charge.target.w / 2),
                f64::from(floor - (charge.target.y + charge.target.h / 2)),
            );
            let d = segment_distance(a.dome_lo, a.dome_hi, c, c);
            if d <= sep {
                errs.push(haz(
                    4,
                    &[a.prop.id.as_str(), charge.id.as_str()],
                    format!("the track is {d:.1} px from an authored charge, more than {STICKER_SEPARATION_PX} required"),
                ));
            }
        }
    }

    // --------------------------------------------------- rule 5: counts
    let has_hazard_entities =
        !stickers.is_empty() || !specs.is_empty() || inp.roster.iter().any(|e| e.kind.is_slime());
    if has_hazard_entities && pockets.is_empty() {
        errs.push(haz(
            5,
            &[],
            "hazard entities (stickers, slimes, acid patches) require declared pockets",
        ));
    }
    for (i, a) in pockets.iter().enumerate() {
        for b in &pockets[i + 1..] {
            if a.x0 < b.x1 && b.x0 < a.x1 {
                errs.push(haz(5, &[&a.id, &b.id], "pockets overlap"));
            }
        }
    }
    if !pockets.is_empty() {
        for p in &specs {
            if !pockets.iter().any(|pk| p.x0 >= pk.x0 && p.x1 <= pk.x1) {
                errs.push(haz(
                    5,
                    &[&p.id.to_string()],
                    "an acid patch lies in no pocket",
                ));
            }
        }
        for e in inp.roster.iter().filter(|e| e.kind.is_slime()) {
            if !pockets.iter().any(|pk| e.home >= pk.x0 && e.home < pk.x1) {
                errs.push(haz(5, &[&e.id], "a slime's home lies in no pocket"));
            }
        }
        for pk in &pockets {
            let stickers_in = stickers
                .iter()
                .filter(|s| {
                    let (a, b) = (
                        s.dome_lo.0.min(s.dome_hi.0) as i32,
                        s.dome_lo.0.max(s.dome_hi.0) as i32,
                    );
                    a >= pk.x0 && b <= pk.x1
                })
                .count();
            let slimes_in = inp
                .roster
                .iter()
                .filter(|e| e.kind.is_slime() && e.x >= pk.x0 && e.x < pk.x1)
                .count();
            let goblins_in = inp
                .roster
                .iter()
                .filter(|e| !e.kind.is_slime() && e.x >= pk.x0 && e.x < pk.x1)
                .count();
            for (what, n, max) in [
                ("stickers", stickers_in, pk.max_stickers),
                ("slimes", slimes_in, pk.max_slimes),
                ("goblins", goblins_in, pk.max_goblins),
            ] {
                if n > usize::from(max) {
                    errs.push(haz(5, &[&pk.id], format!("{n} {what}, at most {max}")));
                }
            }
        }
        for s in &stickers {
            let (a, b) = (
                s.dome_lo.0.min(s.dome_hi.0) as i32,
                s.dome_lo.0.max(s.dome_hi.0) as i32,
            );
            if !pockets.iter().any(|pk| a >= pk.x0 && b <= pk.x1) {
                errs.push(haz(
                    5,
                    &[s.prop.id.as_str()],
                    "a sticker's track lies in no pocket",
                ));
            }
        }
        if inp.raw.waves {
            // PT-TUNE, PTM-Q1 ruling (c) (PTT-Q7): a wave room is a crowd room, so
            // its enemies do not spawn in pockets. Every other rule-5 clause above
            // still holds. Instead, no non-slime body overlaps a pocket or a
            // sticker's blast sweep (half-open ranges). Slimes are exempt: their
            // home must lie in a pocket (clause above).
            for e in inp.roster.iter().filter(|e| !e.kind.is_slime()) {
                let (b0, b1) = body_x(e);
                for pk in &pockets {
                    if b0 < pk.x1 && b1 > pk.x0 {
                        errs.push(haz(
                            5,
                            &[&e.id, &pk.id],
                            "a wave enemy spawns in a pocket or a blast sweep",
                        ));
                    }
                }
                for s in &stickers {
                    if b0 < s.sweep_x.1 && b1 > s.sweep_x.0 {
                        errs.push(haz(
                            5,
                            &[&e.id, s.prop.id.as_str()],
                            "a wave enemy spawns in a pocket or a blast sweep",
                        ));
                    }
                }
            }
        } else {
            for e in inp.roster {
                if !pockets.iter().any(|pk| e.x >= pk.x0 && e.x < pk.x1) {
                    errs.push(haz(5, &[&e.id], "an enemy spawns in no pocket"));
                }
            }
        }
    }

    // --------------------------------------------- rule 6: spawn in acid
    for e in inp.roster {
        let (b0, b1) = body_x(e);
        for p in &specs {
            if p.floor_y != floor || !(b0 < p.x1 && b1 > p.x0) {
                continue;
            }
            let own =
                e.kind.is_slime() && e.linked_pool.as_deref() == Some(p.id.to_string().as_str());
            if !own {
                errs.push(haz(
                    6,
                    &[&e.id, &p.id.to_string()],
                    "only a slime may spawn in acid, and only inside its own linked patch",
                ));
            }
        }
    }

    // ------------------------------------------- rule 7: goblin reach
    for e in inp.roster.iter().filter(|e| !e.kind.is_slime()) {
        let (b0, _) = body_x(e);
        for (pi, p) in specs.iter().enumerate() {
            let between = p.x0 >= room.spawn_carl && p.x1 <= b0;
            if !between {
                continue;
            }
            let drains = p.linked_slime.is_some();
            let crossable = clearable[0] && clearable[1];
            let _ = pi;
            if !(drains || crossable) {
                errs.push(haz(
                    7,
                    &[&e.id, &p.id.to_string()],
                    "the goblin would hold at a patch that neither drains nor is crossed by a jump",
                ));
            }
        }
    }

    // -------------------------------------- rule 9: one boss supplier
    errs.extend(supplier_conflict(
        inp.roster.iter().map(|e| e.kind),
        RosterKind::supplies_boss,
    ));

    // -------------------------------------------- rule 10: direct route
    if inp.goal == Goal::DefeatRoster && inp.roster.is_empty() {
        errs.push(haz(10, &[], "defeat_roster needs a roster"));
    }

    (specs, pockets)
}

fn dozer_err(rule: u8, ids: &[&str], reason: impl Into<String>) -> EncounterError {
    EncounterError::Dozer {
        rule,
        ids: ids.iter().map(|s| (*s).to_owned()).collect(),
        reason: reason.into(),
    }
}

/// DR-C (section 8, DZ-Q20), rule 7: a mounted entry rides the room's one
/// machine (the roster holds a `dozer` entry), stands at the machine's column
/// (its roster `x` is the dozer entry's), and each seat is taken at most once.
fn check_crew(roster: &[RosterEntry], dozers: &[&RosterEntry], errs: &mut Vec<EncounterError>) {
    let riders: Vec<&RosterEntry> = roster.iter().filter(|e| e.mount.is_some()).collect();
    for (i, r) in riders.iter().enumerate() {
        let Some(m) = r.mount else { continue };
        match dozers.first() {
            None => errs.push(dozer_err(
                7,
                &[&r.id],
                "a mounted entry needs the room's dozer entry (its carrier)",
            )),
            Some(d) if d.x != r.x => errs.push(dozer_err(
                7,
                &[&r.id, &d.id],
                "a rider's x is its carrier's (the dozer entry's x)",
            )),
            Some(_) => {}
        }
        if let Some(other) = riders[..i]
            .iter()
            .find(|o| o.mount.is_some_and(|om| om.seat == m.seat))
        {
            errs.push(dozer_err(
                7,
                &[&other.id, &r.id],
                format!("seat {} is taken twice", m.seat.name()),
            ));
        }
    }
}

/// DOZER-CORE and DR-B: the rules of the Dozer note's section 9 (1 to 6; 3 and
/// 5 read the stop frames, the `weight_rig` props with their owned
/// `falling_mass` footprints). A no-op for a sidecar with neither a `dozer`
/// roster entry nor a `dozer` block. Layout px (y down).
/// ACT1-B: the opening rules O1..O10 report as DOZER rules 11..=20 (`opening_rule(n)`).
const fn opening_rule(n: u8) -> u8 {
    10 + n
}

pub(super) fn check_dozer(inp: &HazInput<'_>, errs: &mut Vec<EncounterError>) {
    use crate::dozer::{SiteProfile, rules};
    let dozers: Vec<&RosterEntry> = inp.roster.iter().filter(|e| e.kind.is_dozer()).collect();
    check_crew(inp.roster, &dozers, errs);
    let block = inp.raw.dozer.as_ref();
    let contact: Vec<&RawLock> = inp
        .raw
        .locks
        .iter()
        .filter(|l| l.opens_on.event == "dozer_contact")
        .collect();
    if dozers.is_empty() && block.is_none() {
        for l in &contact {
            errs.push(dozer_err(
                opening_rule(1),
                &[l.id.as_str()],
                "O1: a dozer_contact lock needs the room's dozer block with profile opening",
            ));
        }
        return;
    }
    let ids: Vec<&str> = dozers.iter().map(|e| e.id.as_str()).collect();
    // ---- rule 1: exactly one machine, its block, and a long enough lane
    if dozers.len() != 1 {
        errs.push(dozer_err(
            1,
            &ids,
            "a dozer room has exactly one dozer entry",
        ));
    }
    let Some(block) = block else {
        errs.push(dozer_err(
            1,
            &ids,
            "a dozer entry needs the sidecar's dozer block",
        ));
        return;
    };
    let Some(profile) = block.profile() else {
        errs.push(dozer_err(
            1,
            &ids,
            "the dozer block's profile is revenge or opening",
        ));
        return;
    };
    if block.trigger_by().is_none() {
        errs.push(dozer_err(1, &ids, "dzq11_trigger_by is lead or rearmost"));
        return;
    }
    let room = &inp.ctx.room;
    let site = block.site(room.floor_y, contact.first().map(|l| l.solid.x));
    if let Some(e) = dozers.first()
        && e.x != site.emerge_front_x
    {
        errs.push(dozer_err(
            1,
            &[&e.id],
            "the dozer entry's x is its emergence front (dzq13_emerge_front_x)",
        ));
    }
    if site.stop_x <= room.play_x.0 || site.stop_x >= room.play_x.1 {
        errs.push(dozer_err(1, &ids, "dzq14_stop_x lies outside the room"));
    }
    if site.stop_x < site.emerge_front_x + rules::MIN_RUN_PX {
        errs.push(dozer_err(
            1,
            &ids,
            format!(
                "dzq14_stop_x must be at least the gate x + {}",
                rules::MIN_RUN_PX
            ),
        ));
    }
    if site.trigger_x <= site.emerge_front_x || site.trigger_x >= site.stop_x {
        errs.push(dozer_err(
            1,
            &ids,
            "dzq11_trigger_x lies between the gate and dzq14_stop_x",
        ));
    }
    let solids = lane_solids(inp);
    match profile {
        SiteProfile::Revenge => {
            // ACT1-B: the opening's fields belong to an opening room.
            if !site.pockets.is_empty()
                || !site.timbers.is_empty()
                || site.wedge
                || site.trigger_by != crate::dozer::TriggerBy::Lead
                || !contact.is_empty()
            {
                errs.push(dozer_err(
                    1,
                    &ids,
                    "pockets, timbers, wedge, a rearmost trigger and a dozer_contact lock belong to profile opening",
                ));
            }
            match site.lintel {
                Some(lintel) => check_revenge_lane(inp, &site, lintel, &solids, &ids, errs),
                None => errs.push(dozer_err(
                    4,
                    &ids,
                    "a revenge dozer room needs dzq22_lintel (the tunnel)",
                )),
            }
        }
        SiteProfile::Opening => check_opening(inp, &site, &contact, &solids, &ids, errs),
    }
    // ---- rules 3 and 5: the stop frames
    let t = crate::dozer::DozerTuning::active();
    let wheel_front = t.wheel_front().round() as i32;
    let push = t.dzq02_push_box;
    let tips = t.tips().round() as i32;
    // The parked machine, push box rear to spike tips (stop_x - 252 .. stop_x + 4).
    let rest = (
        site.stop_x - wheel_front + push.x0,
        site.stop_x - wheel_front + tips,
    );
    for (i, p) in inp.props.iter().enumerate() {
        if p.kind != PropKind::WeightRig {
            continue;
        }
        let Some(fall) = inp
            .hazards
            .iter()
            .find(|h| h.owner == Some(i) && h.spec.class == HazardClass::FallingMass)
        else {
            // The prop rules refuse a weight without its falling mass.
            continue;
        };
        let f = fall.spec.rect;
        if f.x < rest.1 && f.right() > rest.0 {
            errs.push(dozer_err(
                3,
                &[&p.id],
                format!(
                    "a frame's footprint overlaps the resting envelope {}..{}",
                    rest.0, rest.1
                ),
            ));
        }
        let min_hang = push.y1 + rules::HANG_OVER_PUSH_BOX_PX;
        match p.hang_px {
            None => errs.push(dozer_err(
                3,
                &[&p.id],
                format!("a frame needs `hang_px` (at least {min_hang} px, the push box top + 8)"),
            )),
            Some(h) if h < min_hang => errs.push(dozer_err(
                3,
                &[&p.id],
                format!("a frame hangs at least {min_hang} px (the push box top + 8)"),
            )),
            Some(_) => {}
        }
        let hero_x =
            p.target.right() + rules::BRACE_STAND_OFF_PX + crate::prototype::HERO_HALF_WIDTH as i32;
        let land_delay = u32::from(fall.spec.telegraph_ticks())
            + u32::from(crate::env_prop::DEFAULT_FALL_TICKS)
            - 1;
        let w = crate::dozer::release_window(
            &t.revenge(),
            &crate::dozer::StopFrame {
                footprint_x0: f.x as f32,
                footprint_x1: f.right() as f32,
                land_delay_ticks: land_delay,
                emerge_front_x: site.emerge_front_x as f32,
            },
            hero_x as f32,
            u32::from(t.dzq23_melee_recovery_ticks),
        );
        if w.ticks < u32::from(t.dzq23_min_release_window_ticks) {
            errs.push(dozer_err(
                5,
                &[&p.id],
                format!(
                    "the useful release window is {} ticks, under the minimum {}",
                    w.ticks, t.dzq23_min_release_window_ticks
                ),
            ));
        }
    }
    // ---- rule 6: no supplier at all, and the K5 profile
    for e in inp.roster.iter().filter(|e| e.kind.supplies_boss()) {
        errs.push(dozer_err(
            6,
            &[&e.id],
            "a dozer room has no BossThreat supplier at all",
        ));
    }
    let tuned = site.tuning();
    if let Err(err) = if profile == SiteProfile::Opening {
        tuned.validate_opening()
    } else {
        t.validate()
    } {
        errs.push(dozer_err(6, &ids, err.to_string()));
    }
}

/// The solids of the room's lane, named (platform slabs, steps, props, solid statics, locks).
fn lane_solids(inp: &HazInput<'_>) -> Vec<(String, IRect)> {
    let room = &inp.ctx.room;
    let slab = inp.raw.platform_slab_px;
    let mut solids: Vec<(String, IRect)> = Vec::new();
    for (i, p) in room.platforms.iter().enumerate() {
        solids.push((
            format!("platform {i}"),
            IRect::new(p.rect.x, p.rect.y, p.rect.w, slab),
        ));
    }
    for (i, r) in room.steps.iter().enumerate() {
        solids.push((format!("step {i}"), *r));
    }
    for p in inp.props {
        for r in p
            .remnant
            .into_iter()
            .chain(p.solid_intact.then_some(p.target))
        {
            solids.push((format!("prop {}", p.id), r));
        }
    }
    for st in inp.raw.statics.iter().filter(|st| st.kind == "solid") {
        solids.push((format!("static {}", st.id), IRect::from(&st.rect)));
    }
    for l in &inp.raw.locks {
        solids.push((format!("lock {}", l.id), IRect::from(&l.solid)));
    }
    solids
}

/// Rules 2 and 4 of room 3 (`revenge`): the clear lane, the ceiling, the passage and the
/// lintel.
fn check_revenge_lane(
    inp: &HazInput<'_>,
    site: &crate::dozer::DozerSite,
    lintel: IRect,
    solids: &[(String, IRect)],
    ids: &[&str],
    errs: &mut Vec<EncounterError>,
) {
    use crate::dozer::rules;
    let floor = inp.ctx.room.floor_y;
    // ---- rule 2: the lane is clear
    let mouth = lintel.x;
    for (name, r) in solids {
        let in_lane = r.x < mouth && r.right() > site.emerge_front_x;
        let bottom_up = floor - r.bottom();
        if in_lane && r.h > rules::REMNANT_MAX_PX && bottom_up < rules::FRAME_HANG_PX {
            errs.push(dozer_err(
                2,
                &[name],
                "a solid on the chase lane is taller than a remnant and below the frame hang",
            ));
        }
        if r.x < mouth && r.right() > site.stop_x - rules::RUN_OUT_CLEAR_PX {
            errs.push(dozer_err(
                2,
                &[name],
                "no solid stands between dzq14_stop_x - 256 and the tunnel mouth",
            ));
        }
    }
    if mouth < site.stop_x + rules::MOUTH_MARGIN_PX {
        errs.push(dozer_err(
            2,
            ids,
            format!(
                "the tunnel mouth must be at least dzq14_stop_x + {}",
                rules::MOUTH_MARGIN_PX
            ),
        ));
    }
    // ---- rule 4: the ceiling, the passage and the lintel
    match inp.ctx.ceiling_px {
        Some(c) if c >= rules::MIN_CEILING_PX => {
            if lintel.y > floor - c {
                errs.push(dozer_err(
                    4,
                    ids,
                    "the tunnel lintel does not reach the lane ceiling",
                ));
            }
        }
        _ => errs.push(dozer_err(
            4,
            ids,
            format!(
                "the lane ceiling must be declared and at least {}",
                rules::MIN_CEILING_PX
            ),
        )),
    }
    let clear = floor - lintel.bottom();
    if !(rules::PASSAGE_CLEAR_PX.0..=rules::PASSAGE_CLEAR_PX.1).contains(&clear) {
        errs.push(dozer_err(
            4,
            ids,
            format!(
                "the escape passage's clear height {clear} is not in {}..={}",
                rules::PASSAGE_CLEAR_PX.0,
                rules::PASSAGE_CLEAR_PX.1
            ),
        ));
    }
    if lintel.x < site.stop_x {
        errs.push(dozer_err(
            4,
            ids,
            "the escape passage lies beyond dzq14_stop_x",
        ));
    }
    let exit = inp.protected[2].1;
    if exit.x < lintel.x || exit.right() > lintel.right() {
        errs.push(dozer_err(4, ids, "the tunnel lintel holds the exit"));
    }
}

/// ACT1-B: the opening's rules O1..O10 (`docs/01-design/floor-01-act-01-integration.md`
/// section 5.6), replacing room 3's rules 2 and 4 (rule 1 still applies, and rules 3, 5 and
/// 6 run after). They report as DOZER rules 11..=20.
fn check_opening(
    inp: &HazInput<'_>,
    site: &crate::dozer::DozerSite,
    contact: &[&RawLock],
    solids: &[(String, IRect)],
    ids: &[&str],
    errs: &mut Vec<EncounterError>,
) {
    use crate::dozer::rules;
    use crate::prototype::{HERO_HALF_HEIGHT, HERO_HALF_WIDTH};
    let room = &inp.ctx.room;
    let floor = room.floor_y;
    let t = site.tuning();
    let mut o = |n: u8, who: &[&str], reason: String| {
        errs.push(dozer_err(opening_rule(n), who, format!("O{n}: {reason}")));
    };
    // ---- O1: the charge block, a pocket and exactly one dozer_contact lock
    let Some(ch) = t.charge else {
        o(
            1,
            ids,
            "the opening tuning has no dzq10_charge block".into(),
        );
        return;
    };
    if site.pockets.is_empty() {
        o(1, ids, "an opening room has at least one pocket".into());
    }
    if contact.len() != 1 {
        o(
            1,
            ids,
            format!(
                "an opening room has exactly one dozer_contact lock (found {})",
                contact.len()
            ),
        );
    }
    let barricade = contact.first().map(|l| IRect::from(&l.solid));
    // ---- O2: the ordering of the emergence, the trigger, the arm, the timbers and the pocket
    if !(site.emerge_front_x < site.trigger_x && site.trigger_x < ch.arm_x) {
        o(2, ids, "emerge_front_x < trigger_x < arm_x".into());
    }
    for p in &site.pockets {
        if ch.arm_x >= p.rect.x {
            o(2, &[&p.id], "arm_x < the pocket's x".into());
        }
    }
    let mut seen_timbers: Vec<&str> = Vec::new();
    let mut prev_x = i32::MIN;
    for tm in &site.timbers {
        if seen_timbers.contains(&tm.id.as_str()) {
            o(2, &[&tm.id], "timber ids are unique".into());
        }
        seen_timbers.push(&tm.id);
        if !(site.emerge_front_x < tm.x && tm.x < ch.arm_x) {
            o(
                2,
                &[&tm.id],
                "a timber's x lies in (emerge_front_x, arm_x)".into(),
            );
        }
        if tm.x <= prev_x {
            o(2, &[&tm.id], "timber x is strictly increasing".into());
        }
        prev_x = tm.x;
    }
    // ---- O3: each pocket stands on a solid static, fits two bodies and clears a hero
    for p in &site.pockets {
        let ledge = inp
            .raw
            .statics
            .iter()
            .find(|st| st.id == p.ledge && st.kind == "solid");
        match ledge {
            None => o(
                3,
                &[&p.id],
                format!(
                    "the ledge {:?} is not a solid static of the sidecar",
                    p.ledge
                ),
            ),
            Some(st) if st.rect.y != p.rect.bottom() => o(
                3,
                &[&p.id],
                format!(
                    "the ledge's top {} is not the pocket rect's bottom {}",
                    st.rect.y,
                    p.rect.bottom()
                ),
            ),
            Some(st) => {
                let ledge = IRect::from(&st.rect);
                if ledge.x > p.rect.x || ledge.right() < p.rect.right() {
                    o(3, &[&p.id], "the ledge spans the pocket".into());
                }
            }
        }
        if p.rect.w < (4.0 * HERO_HALF_WIDTH) as i32 {
            o(
                3,
                &[&p.id],
                "the pocket fits two bodies (width >= 4 half widths)".into(),
            );
        }
        if p.rect.h < (2.0 * HERO_HALF_HEIGHT) as i32 + 8 {
            o(
                3,
                &[&p.id],
                "the pocket clears a hero (height >= body + 8)".into(),
            );
        }
        if let Some(b) = barricade
            && p.rect.right() > b.x
        {
            o(3, &[&p.id], "the pocket lies left of the barricade".into());
        }
    }
    // ---- O4: the barricade lies after the pocket, far enough before stop_x, floor to a wall
    if let Some(b) = barricade {
        if site.pockets.iter().any(|p| b.x < p.rect.right()) {
            o(4, ids, "the barricade lies after the pocket".into());
        }
        if b.right() + rules::RUN_OUT_CLEAR_PX > site.stop_x {
            o(
                4,
                ids,
                format!(
                    "the barricade ends at least {} px before dzq14_stop_x",
                    rules::RUN_OUT_CLEAR_PX
                ),
            );
        }
        if b.bottom() != floor {
            o(4, ids, "the barricade reaches the floor".into());
        }
        let apex = jump_classes()
            .iter()
            .map(|c| crate::agility::discrete_apex_px(c.launch))
            .fold(0.0_f32, f32::max);
        if (b.h as f32) <= apex + 2.0 * HERO_HALF_HEIGHT {
            o(
                4,
                ids,
                format!("the barricade is taller than any hero jump ({apex:.0} px apex)"),
            );
        }
    }
    // ---- O5: the exit lies past the past-stop rescue spot
    let exit = inp.protected[2].1;
    let past = i32::from(t.dzq17_rescue_past_stop_px) + i32::from(t.dzq17_rescue_exit_margin_px);
    if exit.x < site.stop_x + past {
        o(5, ids, format!("exit.x >= stop_x + {past}"));
    }
    // ---- O6: nothing else on the lane between the emergence front and stop_x stands over 16 px
    for (name, r) in solids {
        let is_barricade = barricade.is_some_and(|b| b == *r) && name.starts_with("lock ");
        if is_barricade {
            continue;
        }
        let in_lane = r.x < site.stop_x && r.right() > site.emerge_front_x;
        if in_lane && r.h > rules::REMNANT_MAX_PX {
            o(
                6,
                &[name],
                format!(
                    "a solid on the lane is at most {} px high (the barricade excepted)",
                    rules::REMNANT_MAX_PX
                ),
            );
        }
    }
    // ---- O7: no lintel and no crew
    if site.lintel.is_some() {
        o(7, ids, "an opening room has no dzq22_lintel".into());
    }
    if inp.roster.iter().any(|e| e.mount.is_some()) {
        o(7, ids, "an opening room has no mounted crew".into());
    }
    // ---- O8: the barricade closed cuts the exit (the room's closed-lock tests); open, the
    // existing reach checks hold. Nothing to add here: the lock is a solid of every graph.
    // ---- O9: the commit window is non-empty at the slowest band speed
    if let (Some(b), Some(p)) = (barricade, site.pockets.first()) {
        let reach = (f32::from(ch.tell_ticks) * f32::from(t.revenge.dzq08_mercy_px_s)
            + f32::from(ch.speed_px_s) * f32::from(ch.max_ticks))
            / 60.0;
        let earliest = b.x as f32 - reach;
        let latest = (p.rect.x - ch.min_gap_px) as f32;
        if earliest > latest {
            o(
                9,
                ids,
                format!("the commit window {earliest:.0}..{latest:.0} is empty"),
            );
        }
        // ---- O10: a pocket rescue never puts a body in the barricade
        let centre = p.rect.x as f32 + p.rect.w as f32 * 0.5;
        if centre >= b.x as f32 - HERO_HALF_WIDTH {
            o(
                10,
                ids,
                "the pocket centre lies left of the barricade by a body half width".into(),
            );
        }
    }
}

#[cfg(test)]
mod tests;
