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

pub(super) struct HazInput<'a> {
    pub raw: &'a RawEncounter,
    pub ctx: &'a Context,
    pub goal: Goal,
    pub props: &'a [Prop],
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
    if !pockets.is_empty() {
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
        for e in inp.roster {
            if !pockets.iter().any(|pk| e.x >= pk.x0 && e.x < pk.x1) {
                errs.push(haz(5, &[&e.id], "an enemy spawns in no pocket"));
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
    let suppliers =
        distinct_suppliers(inp.roster.iter().map(|e| e.kind), RosterKind::supplies_boss);
    if suppliers > 1 {
        errs.push(haz(
            9,
            &[],
            format!("{suppliers} kinds supply the companion `boss` input, at most 1"),
        ));
    }

    // -------------------------------------------- rule 10: direct route
    if inp.goal == Goal::DefeatRoster && inp.roster.is_empty() {
        errs.push(haz(10, &[], "defeat_roster needs a roster"));
    }

    (specs, pockets)
}

#[cfg(test)]
mod tests;
