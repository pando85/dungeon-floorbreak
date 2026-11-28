//! RING-A core: headless topology fixtures of the Broken Ring (FLOOR01-RINGS-01,
//! plan section 15). Bevy-free and **not wired into any scene**; nothing here
//! runs in the game.
//!
//! Two rooms (KR1) load through the same encounter core as the Goblin Boiler
//! ([`crate::encounter::load`]): `ring_court` (the crowned breach and the
//! Shutter Court, about 2048 px) and `ring_split` (the split passage and the
//! light beyond stone, about 1152 px in local coordinates). Both name the
//! post-Hoarder story pin `f01_pin_post_pile_boss` (KR12), grant nothing and
//! declare the DP-Q45 `ceiling_px` 228 (KR9).
//!
//! What the tests prove, from the data and the validator's route graphs:
//!
//! * the 64 px step, the 80 px walkway over the arch and the 128 px shelf have
//!   the rises, gate classes and clearances of KR2 and KR8, and the shelf lies
//!   farther from the step and the walkway than base Donut's reach, so only
//!   boosted Donut gets there;
//! * the closed shutter cuts the lane for every class and the exit is still
//!   reachable for Carl, base Donut and boosted Donut **through the walkway**
//!   (the per-side proof lives in the validator and rejects a room without it);
//! * with a Carl whose reliable rise is below the 80 px class (the 8.5 px apex
//!   margin of KR2), the step is what puts the walkway in his reach;
//! * the KR8 headroom check (at least Carl's height between any two surfaces
//!   that overlap in x), the KR10 view ledge (at most 80 px) and the R2 fork
//!   (two routes that rejoin well inside 640 px).
//!
//! The route graphs use the height classes of the data file (an optimistic
//! bound for a rising jump); RING-A wire replaces that bound with the
//! **physics-exact run** ([`crate::encounter_physics::check`] on each room's
//! launched kit, the lane ceiling slab included): every edge the validator
//! accepts is replayed with the real Carl, base Donut and boosted Donut
//! integrators, and the exit is joined over the witnessed edges in every state.
//! The scene runs, fights and traversals are `crate::f01_ring_fixtures`'s; no
//! shutter motion is exercised (RING-C wire).

use std::path::Path;

use crate::camera_lab::room::Room;
use crate::encounter::{EncounterError, Loaded, load};

pub const COURT_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_ring_court.encounter.json";
pub const COURT_ROOM_PATH: &str = "assets/levels/floor01/f01_ring_court.room.json";
pub const SPLIT_ENCOUNTER_PATH: &str = "assets/levels/floor01/f01_ring_split.encounter.json";
pub const SPLIT_ROOM_PATH: &str = "assets/levels/floor01/f01_ring_split.room.json";

/// Load and validate one of the two Ring rooms with its sidecar.
pub fn load_ring(root: &Path, sidecar: &str) -> Result<Loaded, String> {
    load(root, sidecar).map_err(|errs| {
        errs.iter()
            .map(EncounterError::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    })
}

/// KR8 headroom: for every two surfaces that overlap in x (the floor, the
/// platforms and every standable prop solid), the lower surface's top is at
/// least `carl_height_px` below the upper platform's slab underside. A shutter
/// is excluded (it is never standable, KR3 clause 1). Returns the violating
/// `(lower, upper, clearance)` surface names.
pub fn headroom_violations(
    room: &Room,
    slab_px: i32,
    carl_height_px: i32,
    extra_lower: &[(String, crate::camera_lab::room::IRect)],
) -> Vec<(String, String, i32)> {
    struct Surface {
        name: String,
        x0: i32,
        x1: i32,
        top: i32,
    }
    let mut lower = vec![Surface {
        name: "floor".into(),
        x0: room.play_x.0,
        x1: room.play_x.1,
        top: room.floor_y,
    }];
    let mut upper: Vec<Surface> = Vec::new();
    for (i, p) in room.platforms.iter().enumerate() {
        let s = Surface {
            name: format!("platform {i}"),
            x0: p.rect.x,
            x1: p.rect.right(),
            top: p.rect.y,
        };
        upper.push(Surface {
            name: s.name.clone(),
            x0: s.x0,
            x1: s.x1,
            top: s.top,
        });
        lower.push(s);
    }
    for (name, r) in extra_lower {
        lower.push(Surface {
            name: name.clone(),
            x0: r.x,
            x1: r.right(),
            top: r.y,
        });
    }
    let mut bad = Vec::new();
    for l in &lower {
        for u in &upper {
            if u.top >= l.top || u.x0 >= l.x1 || u.x1 <= l.x0 {
                continue;
            }
            let clearance = l.top - (u.top + slab_px);
            if clearance < carl_height_px {
                bad.push((l.name.clone(), u.name.clone(), clearance));
            }
        }
    }
    bad
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_lab::room::IRect;
    use crate::encounter::{
        Context, HEIGHT_CLASSES_PATH, HeightClasses, PropKind, ReachHero, parse_sidecar,
    };
    use serde_json::{Value, json};

    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
    }

    fn court() -> Loaded {
        load_ring(root(), COURT_ENCOUNTER_PATH).expect("the court validates")
    }

    fn split() -> Loaded {
        load_ring(root(), SPLIT_ENCOUNTER_PATH).expect("the split validates")
    }

    fn sidecar(path: &str) -> Value {
        serde_json::from_str(&std::fs::read_to_string(root().join(path)).unwrap()).unwrap()
    }

    fn ctx(l: &Loaded) -> Context {
        Context {
            room: l.room.clone(),
            classes: l.classes,
            ceiling_px: l.ceiling_px,
        }
    }

    fn slab() -> i32 {
        court().encounter.platform_slab_px
    }

    fn rise(l: &Loaded, platform: usize) -> i32 {
        l.room.floor_y - l.room.platforms[platform].rect.y
    }

    /// The state bit of the only changeable prop of the court: the shutter.
    const CLOSED: u16 = 1;
    const OPEN: u16 = 0;

    fn row(l: &Loaded, hero: ReachHero, state: u16) -> &crate::encounter::ReachRow {
        l.encounter
            .reach
            .iter()
            .find(|r| r.hero == hero && r.state == state)
            .expect("the matrix covers every class and state")
    }

    // ---- loading, pins, shared declarations

    #[test]
    fn both_rooms_load_validate_and_share_the_declarations() {
        for (l, id, enc) in [
            (court(), "F01-RING-COURT", "F01-RING-COURT-ENC"),
            (split(), "F01-RING-SPLIT", "F01-RING-SPLIT-ENC"),
        ] {
            assert_eq!(l.room.id, id);
            assert_eq!(l.encounter.id, enc);
            // KR12: the existing post-Hoarder pin, and the room grants nothing.
            assert_eq!(l.encounter.story_pin, "f01_pin_post_pile_boss");
            // KR9 / DP-Q45.
            assert_eq!(l.ceiling_px, Some(228));
            assert_eq!(l.room.floor_y, 580);
            assert_eq!(l.room.snap_dy, 0, "the authored y values are on the grid");
            assert!(l.room.props.is_empty() && l.room.enemies.is_empty());
            assert!(l.room.hazards.is_empty());
            // Two staggered goblins in each room (KR20: RING-11 swaps them later).
            assert_eq!(l.encounter.roster.len(), 2);
            assert!(l.encounter.roster.iter().all(|e| e.kind
                == crate::encounter::RosterKind::Room(crate::camera_lab::room::EnemyKind::Goblin)));
            let delays: Vec<u32> = l.encounter.roster.iter().map(|e| e.delay_ticks).collect();
            assert!(delays[0] != delays[1], "staggered: {delays:?}");
            assert_eq!(l.encounter.locks.len(), 0);
            for t in [
                &l.room.name,
                &l.room.subtitle,
                &l.room.purpose,
                &l.room.coop,
            ] {
                assert!(t.is_ascii(), "{t:?}");
            }
        }
    }

    #[test]
    fn the_room_sizes_follow_kr1() {
        // About 2048 px for R0 and R1, about 1152 px for R2 and R3.
        assert_eq!(court().room.render.0, 2048);
        assert_eq!(split().room.render.0, 1152);
    }

    #[test]
    fn a_room_that_grants_is_rejected_here_too() {
        for path in [COURT_ENCOUNTER_PATH, SPLIT_ENCOUNTER_PATH] {
            let mut v = sidecar(path);
            v["grants"] = json!(["dynamite"]);
            let l = if path == COURT_ENCOUNTER_PATH {
                court()
            } else {
                split()
            };
            let errs = parse_sidecar(&v.to_string(), &ctx(&l)).unwrap_err();
            assert!(errs.contains(&EncounterError::GrantsNotEmpty));
        }
    }

    #[test]
    fn the_rooms_compose_through_the_existing_kit() {
        use crate::camera_lab::layout::build_kit;
        for l in [court(), split()] {
            let (kit, _, plan) = build_kit(root(), &l.room).expect("composes");
            assert_eq!((kit.width, kit.height), l.room.render);
            assert!(plan.tiers >= 2);
        }
    }

    #[test]
    fn the_file_hashes_are_pinned() {
        let c = court();
        let s = split();
        assert_eq!(
            (c.room_hash, c.sidecar_hash),
            COURT_HASHES,
            "update with the court files"
        );
        assert_eq!(
            (s.room_hash, s.sidecar_hash),
            SPLIT_HASHES,
            "update with the split files"
        );
    }

    // FNV-1a 64 of (room file, sidecar): a data edit must update them on purpose.
    // RING-A/B wire moved the pins to `crate::f01_ring` (the launcher refuses an
    // unpinned edit); the status texts of all four files changed with it.
    const COURT_HASHES: (u64, u64) = (
        crate::f01_ring::COURT_ROOM_HASH,
        crate::f01_ring::COURT_ENCOUNTER_HASH,
    );
    const SPLIT_HASHES: (u64, u64) = (
        crate::f01_ring::SPLIT_ROOM_HASH,
        crate::f01_ring::SPLIT_ENCOUNTER_HASH,
    );

    // ---- RING-A wire: the physics-exact reachability (K8, replaces the
    // optimistic class bound above)

    /// Both rooms on their launched kits: every accepted edge of every class
    /// and state lands with the real integrator **but one**, and the exit is
    /// joined over the witnessed edges alone in every state. The one is KR2's
    /// prediction, confirmed: Carl's direct 80 px jump from the left floor onto
    /// the walkway (the validator accepts it, 80 is his class) has no takeoff
    /// with the shutter closed (the step is in the way and the apex margin is
    /// about 8.5 px); the 64 px step is his real route, and it is witnessed. The
    /// lane walker (which hops what is ahead and never climbs a route) reaches
    /// the exit with the shutter open, and **never** with it closed: the closed
    /// shutter is a real cut, and the crossing is the step and the walkway (KR3).
    #[test]
    fn the_physics_exact_run_proves_both_rooms() {
        use crate::encounter_physics::check;
        use crate::f01_ring::{RingRoom, launch};
        for room in RingRoom::ALL {
            let run = launch(root(), room).unwrap();
            let report = check(&run.kit, &run.loaded).unwrap();
            println!("{} K8 {}", room.id(), report.summary());
            let bad = report.untraversable();
            let walkway = run.loaded.room.platforms.get(1).map(|p| p.rect);
            let kr2 = |e: &&crate::encounter_physics::EdgeReport| {
                room == RingRoom::Court
                    && e.hero == ReachHero::Carl
                    && e.state & CLOSED != 0
                    && e.from.floor
                    && e.from.x1 <= run.loaded.encounter.props[0].target.x
                    && walkway.is_some_and(|w| e.to.top == w.y && e.to.x0 == w.x)
            };
            let other: Vec<_> = bad.iter().filter(|e| !kr2(e)).collect();
            assert!(
                other.is_empty(),
                "{room:?}: validator false positives: {other:#?}"
            );
            assert_eq!(
                bad.len(),
                usize::from(room == RingRoom::Court),
                "{room:?}: {bad:#?}"
            );
            for exit in &report.exits {
                assert!(exit.by_edges, "{room:?}: {exit:?}");
                let closed = room == RingRoom::Court && exit.state & CLOSED != 0;
                assert_eq!(exit.walk_tick.is_none(), closed, "{room:?}: {exit:?}");
            }
            assert_eq!(
                report.exits.len(),
                if room == RingRoom::Court { 6 } else { 3 }
            );
            for hero in ReachHero::ALL {
                let edges: Vec<_> = report.edges.iter().filter(|e| e.hero == hero).collect();
                assert!(
                    edges.iter().any(|e| e.from.floor && !e.to.floor),
                    "{hero:?} climb"
                );
                assert!(
                    edges.iter().any(|e| !e.from.floor && e.to.floor),
                    "{hero:?} drop"
                );
            }
        }
    }

    /// KR2's step, physics-exact: Carl's real jump lands on the 64 px step from
    /// the floor, and from the step on the 80 px walkway; the 128 px shelf has a
    /// boosted-Donut witness and no Carl or base-Donut edge (KR8).
    #[test]
    fn the_physics_exact_run_lands_the_step_the_walkway_and_the_boosted_shelf() {
        use crate::encounter_physics::check;
        use crate::f01_ring::{RingRoom, launch};
        let run = launch(root(), RingRoom::Court).unwrap();
        let report = check(&run.kit, &run.loaded).unwrap();
        let top = |i: usize| run.loaded.room.platforms[i].rect.y;
        let lands = |hero: ReachHero, from: Option<i32>, to: i32| {
            report.edges.iter().any(|e| {
                e.hero == hero
                    && e.to.top == to
                    && !e.to.floor
                    && from.map_or(e.from.floor, |f| !e.from.floor && e.from.top == f)
                    && e.witness.is_some()
            })
        };
        assert!(lands(ReachHero::Carl, None, top(0)), "floor to step");
        assert!(
            lands(ReachHero::Carl, Some(top(0)), top(1)),
            "step to walkway"
        );
        assert!(
            lands(ReachHero::DonutBoosted, None, top(2)),
            "boosted Donut on the shelf"
        );
        for hero in [ReachHero::Carl, ReachHero::DonutBase] {
            assert!(
                !report
                    .edges
                    .iter()
                    .any(|e| e.hero == hero && e.to.top == top(2)),
                "{hero:?} has no shelf edge"
            );
        }
    }

    // ---- the court: step, walkway, shutter, shelf

    #[test]
    fn the_court_platforms_are_the_step_the_walkway_and_the_shelf() {
        let l = court();
        assert_eq!(l.room.platforms.len(), 3);
        // KR2: a 64 px step onto the 80 px walkway (a 16 px rise), KR8: 128 px.
        assert_eq!(
            (rise(&l, 0), rise(&l, 1), rise(&l, 2)),
            (64, 80, 128),
            "step, walkway, shelf"
        );
        let (step, walk) = (l.room.platforms[0].rect, l.room.platforms[1].rect);
        // The step abuts the walkway: no gap to cross at the top.
        assert_eq!(step.right(), walk.x);
        assert_eq!(step.y - walk.y, 16);
        // Walkway underside 68 (CF02-Q45): the slab leaves Carl's height.
        let clearance = rise(&l, 1) - slab();
        assert_eq!(clearance, 68);
        assert!(clearance >= l.classes.carl_height_px);
        assert_eq!(
            l.encounter.gates,
            [ReachHero::Carl, ReachHero::Carl, ReachHero::DonutBoosted]
        );
        // The step is well inside Carl's reliable range: 64 px against his 80.
        assert!(l.classes.max_rise(ReachHero::Carl) - rise(&l, 0) >= 16);
    }

    #[test]
    fn the_shutter_fills_the_arch_under_the_walkway() {
        let l = court();
        let e = &l.encounter;
        assert_eq!(e.props.len(), 1);
        let p = &e.props[0];
        assert_eq!(p.kind, PropKind::Shutter);
        let walk = l.room.platforms[1].rect;
        let t = p.target;
        // KR2: 68 px, from the slab underside to the walking line.
        assert_eq!(
            (t.y, t.h, t.bottom()),
            (walk.y + slab(), 68, l.room.floor_y)
        );
        assert!(t.x >= walk.x && t.right() <= walk.right());
        // RING-C wire: a moving shutter (open, one closure, armed 192 px before
        // the arch), at the authored KR2 numbers.
        let cfg = p.shutter.expect("a shutter has its config");
        assert!(!cfg.is_static());
        assert_eq!(cfg.arm_x, Some(1008));
        assert_eq!(cfg.cycles, 1);
        assert_eq!(cfg.warning_ticks, 40);
        assert_eq!(cfg.speed_px_per_tick, 2);
        assert_eq!(cfg.closing_ticks(t.h), 34, "inside #189's 24 to 36");
        assert_eq!(t.h / i32::from(cfg.slice_px), 17, "17 slices of 4 px (KR6)");
        assert_eq!(
            cfg.blocked_window_ticks(t.h),
            34 + 150 + 34,
            "KR19: about 218"
        );
    }

    #[test]
    fn the_closed_shutter_cuts_the_lane_for_every_class_and_the_walkway_crosses_it() {
        let l = court();
        for hero in ReachHero::ALL {
            let open = row(&l, hero, OPEN);
            assert!(open.exit && open.lane && open.regroup, "{hero:?} open");
            let closed = row(&l, hero, CLOSED);
            // The cut is for every class, a 68 px solid within every class's
            // rise: it is never stepped over (KR3 clause 1).
            assert!(!closed.lane, "{hero:?}: the closed shutter cuts the lane");
            // The crossing is the walkway (KR3 clause 4): exit, regroup, perches.
            assert!(closed.exit, "{hero:?} reaches the exit over the walkway");
            assert!(
                closed.regroup,
                "{hero:?}: both spawns are on the left floor"
            );
            // The step and the walkway are reached and left (Carl's class).
            assert_eq!(closed.perches[0], (true, true), "{hero:?} step");
            assert_eq!(closed.perches[1], (true, true), "{hero:?} walkway");
        }
    }

    #[test]
    fn carl_needs_the_step_when_the_80_px_class_is_only_barely_his() {
        // KR2: an 80 px jump from the floor has an apex margin of about 8.5 px.
        // Model a Carl whose reliable rise is 72 (80 minus that margin, rounded):
        // the walkway is then a base-Donut-class surface, and the validator sees
        // that the step puts it in Carl's reach (a bypassed gate), and that
        // without the step he cannot cross the closed shutter at all.
        let l = court();
        let text = std::fs::read_to_string(root().join(HEIGHT_CLASSES_PATH)).unwrap();
        let strict_text = text.replace(
            "\"carl\": { \"max_rise_px\": 80,",
            "\"carl\": { \"max_rise_px\": 72,",
        );
        assert_ne!(text, strict_text, "the data file moved its Carl line");
        let strict = HeightClasses::parse(&strict_text).expect("a valid class set");
        assert_eq!(strict.gate_class(80), Some(ReachHero::DonutBase));

        // The walkway declared at its strict class: Carl still gets there.
        let mut with_step = sidecar(COURT_ENCOUNTER_PATH);
        with_step["gates"][1]["class"] = json!("donut_base");
        let c = Context {
            classes: strict,
            ..ctx(&l)
        };
        let errs = parse_sidecar(&with_step.to_string(), &c).unwrap_err();
        assert!(
            errs.iter().any(|e| matches!(
                e,
                EncounterError::GateBypassed {
                    surface: 1,
                    by: "carl",
                    ..
                }
            )),
            "with the step Carl reaches the walkway: {errs:?}"
        );
        assert!(
            !errs
                .iter()
                .any(|e| matches!(e, EncounterError::ExitUnreachable { .. })),
            "with the step every class crosses: {errs:?}"
        );

        // Without the step platform: the walkway and the shelf shift down by one.
        let mut room = l.room.clone();
        room.platforms.remove(0);
        let mut no_step = sidecar(COURT_ENCOUNTER_PATH);
        no_step["gates"] = json!([
            { "surface": 0, "class": "donut_base" },
            { "surface": 1, "class": "donut_boosted" }
        ]);
        no_step["vantages"] = json!([{ "surface": 1, "counter": "lane_ceiling" }]);
        let c = Context {
            room,
            classes: strict,
            ..ctx(&l)
        };
        let errs = parse_sidecar(&no_step.to_string(), &c).unwrap_err();
        assert!(
            errs.iter().any(|e| matches!(
                e,
                EncounterError::ExitUnreachable {
                    hero: ReachHero::Carl,
                    state: CLOSED
                }
            )),
            "without the step this Carl cannot cross the closed shutter: {errs:?}"
        );
        // And the per-side proof names the side he starts from.
        assert!(
            errs.iter().any(|e| matches!(
                e,
                EncounterError::ShutterBypass {
                    hero: ReachHero::Carl,
                    side: crate::encounter::ShutterSide::Left,
                    ..
                }
            )),
            "{errs:?}"
        );
        // Donut's classes still cross without it (their rise is over 80).
        assert!(
            !errs.iter().any(|e| matches!(
                e,
                EncounterError::ExitUnreachable {
                    hero: ReachHero::DonutBase | ReachHero::DonutBoosted,
                    ..
                }
            )),
            "{errs:?}"
        );
    }

    // ---- KR8: the shelf

    #[test]
    fn the_shelf_is_a_boosted_only_flank_that_no_chain_reaches() {
        let l = court();
        let shelf = l.room.platforms[2].rect;
        let reach_base = l.classes.reach_x(ReachHero::DonutBase);
        assert_eq!(reach_base, 188, "base Donut's reach_x, the binding class");
        // The shelf overlaps neither the step nor the walkway in x, and its
        // horizontal gap to both is more than base Donut's reach (KR8).
        for i in [0usize, 1] {
            let o = l.room.platforms[i].rect;
            assert!(
                shelf.x >= o.right() || shelf.right() <= o.x,
                "no overlap in x"
            );
            let gap = (shelf.x - o.right()).max(o.x - shelf.right()).max(0);
            assert!(gap > reach_base, "platform {i}: gap {gap} <= {reach_base}");
        }
        // On the goblins' side of the court: right of the shutter.
        assert!(shelf.x >= l.encounter.props[0].target.right());
        // Only boosted Donut gets there, in every state; Carl and base Donut
        // never do (no chain), and the validator declares it a lane-ceiling
        // vantage (DP-Q45).
        for state in [OPEN, CLOSED] {
            assert!(!row(&l, ReachHero::Carl, state).perches[2].0);
            assert!(!row(&l, ReachHero::DonutBase, state).perches[2].0);
            assert_eq!(
                row(&l, ReachHero::DonutBoosted, state).perches[2],
                (true, true)
            );
        }
        assert_eq!(l.encounter.effective_gates[2], ReachHero::DonutBoosted);
        assert_eq!(
            l.encounter.vantages,
            [(2, crate::encounter::VantageCounter::LaneCeiling)]
        );
        // Under the 228 px soffit, with room to stand: shelf top 128 plus
        // Donut's boosted jump stays inside DP-Q45's band by construction of the
        // ceiling rule; the declared ceiling is the room's.
        assert!(l.ceiling_px.unwrap() <= l.classes.ceiling_rule.ceiling_max_px);
    }

    #[test]
    fn a_shelf_placed_next_to_the_step_is_rejected_as_a_bypassed_gate() {
        // Negative: the same shelf 112 px from the walkway (inside base Donut's
        // reach of 188) is no longer boosted-only: base Donut chains onto it.
        let l = court();
        let mut room = l.room.clone();
        room.platforms[2].rect.x = room.platforms[1].rect.right() + 112;
        let errs = parse_sidecar(
            &sidecar(COURT_ENCOUNTER_PATH).to_string(),
            &Context { room, ..ctx(&l) },
        )
        .unwrap_err();
        assert!(
            errs.iter().any(|e| matches!(
                e,
                EncounterError::GateBypassed {
                    surface: 2,
                    declared: "donut_boosted",
                    ..
                }
            )),
            "{errs:?}"
        );
    }

    // ---- KR8: headroom

    #[test]
    fn headroom_holds_in_both_rooms() {
        for l in [court(), split()] {
            let bad = headroom_violations(
                &l.room,
                l.encounter.platform_slab_px,
                l.classes.carl_height_px,
                &[],
            );
            assert!(bad.is_empty(), "{}: {bad:?}", l.room.id);
        }
    }

    #[test]
    fn headroom_rejects_a_platform_that_crowds_another() {
        // A platform 96 px up straight over the 80 px walkway leaves 4 px.
        let l = court();
        let mut room = l.room.clone();
        let over = IRect::new(1152, room.floor_y - 96, 64, 32);
        room.platforms.push(crate::camera_lab::room::Platform {
            rect: over,
            carl_access: true,
        });
        let bad = headroom_violations(
            &room,
            l.encounter.platform_slab_px,
            l.classes.carl_height_px,
            &[],
        );
        assert!(
            bad.iter()
                .any(|(lo, up, c)| lo == "platform 1" && up == "platform 3" && *c == 4),
            "{bad:?}"
        );
        // The exact limit passes: 52 px between the lower top and the slab.
        let mut room = l.room.clone();
        room.platforms.push(crate::camera_lab::room::Platform {
            rect: IRect::new(1152, 500 - 52 - 12, 64, 32),
            carl_access: true,
        });
        let bad = headroom_violations(
            &room,
            l.encounter.platform_slab_px,
            l.classes.carl_height_px,
            &[],
        );
        assert!(!bad.iter().any(|(lo, _, _)| lo == "platform 1"), "{bad:?}");
    }

    // ---- the split: R2 fork and the R3 view ledge

    #[test]
    fn the_split_fork_has_two_routes_that_rejoin_inside_one_screen() {
        let l = split();
        let overhead = l.room.platforms[0].rect;
        assert_eq!(rise(&l, 0), 64, "KR2 / #189: the 64 px overhead route");
        // The fork opens at the platform's start and the routes rejoin at its
        // end, inside one native screen (640 px), with the ground lane intact.
        assert!(overhead.w <= 640, "rejoin within 640 px: {}", overhead.w);
        for hero in ReachHero::ALL {
            let r = row(&l, hero, 0);
            assert!(r.lane && r.exit && r.regroup, "{hero:?} ground route");
            assert_eq!(r.perches[0], (true, true), "{hero:?} overhead route");
        }
        assert_eq!(l.encounter.gates[0], ReachHero::Carl);
        assert!(l.encounter.props.is_empty());
    }

    #[test]
    fn the_view_ledge_is_at_most_80_px_and_carl_stands_on_it() {
        // KR10: both heroes and both players see the camp together.
        let l = split();
        let ledge = 1usize;
        let up = rise(&l, ledge);
        assert!(up <= 80, "view ledge {up} px");
        assert_eq!(l.encounter.gates[ledge], ReachHero::Carl);
        for hero in ReachHero::ALL {
            assert_eq!(row(&l, hero, 0).perches[ledge], (true, true), "{hero:?}");
        }
        // It is right of the fork and close to the exit (R3 follows R2).
        let ledge_rect = l.room.platforms[ledge].rect;
        assert!(ledge_rect.x >= l.room.platforms[0].rect.right());
        assert!(ledge_rect.right() <= l.room.exit.x);
        // Unthreatened ledge above 80 px would be a nest (K8): this one is not.
        assert!(l.encounter.vantages.is_empty());
    }

    #[test]
    fn a_view_ledge_over_80_px_would_be_a_nest() {
        // Negative: raise it to 96 px and K8 rejects it (unthreatened, no
        // counter), so the reveal cannot quietly become a Donut-only peek.
        let l = split();
        let mut room = l.room.clone();
        room.platforms[1].rect.y = room.floor_y - 96;
        let mut v = sidecar(SPLIT_ENCOUNTER_PATH);
        v["gates"][1]["class"] = json!("donut_base");
        let errs = parse_sidecar(&v.to_string(), &Context { room, ..ctx(&l) }).unwrap_err();
        assert!(
            errs.iter().any(|e| matches!(
                e,
                EncounterError::UnthreatenedNest {
                    surface: 1,
                    rise: 96
                }
            )),
            "{errs:?}"
        );
    }
}
