use super::*;
use serde_json::{Value, json};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn text(errs: &[EncounterError]) -> String {
    errs.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

fn loaded() -> Loaded {
    load(root(), HAZARDS_ENCOUNTER_PATH).unwrap_or_else(|e| panic!("{}", text(&e)))
}

fn sidecar() -> Value {
    let t = std::fs::read_to_string(root().join(HAZARDS_ENCOUNTER_PATH)).unwrap();
    serde_json::from_str(&t).unwrap()
}

fn ctx() -> Context {
    let l = loaded();
    Context {
        room: l.room,
        classes: l.classes,
        ceiling_px: l.ceiling_px,
    }
}

fn check(v: &Value) -> Result<Encounter, Vec<EncounterError>> {
    parse_sidecar(&v.to_string(), &ctx())
}

fn errors(v: &Value) -> Vec<EncounterError> {
    check(v).expect_err("the edited sidecar must be rejected")
}

fn rule(errs: &[EncounterError], n: u8) -> Vec<&EncounterError> {
    errs.iter()
        .filter(|e| matches!(e, EncounterError::Haz { rule, .. } if *rule == n))
        .collect()
}

fn has_rule(errs: &[EncounterError], n: u8, ids: &[&str]) -> bool {
    rule(errs, n).into_iter().any(|e| match e {
        EncounterError::Haz { ids: got, .. } => ids.iter().all(|i| got.iter().any(|g| g == i)),
        _ => false,
    })
}

// FNV-1a 64 of the two files: a data edit must update them on purpose.
const ROOM_HASH: u64 = 4352694897781657650;
const SIDECAR_HASH: u64 = 8047922853202382699;

// ------------------------------------------------------------ the shipped room

#[test]
fn the_shipped_room_passes_every_rule() {
    let l = loaded();
    let e = &l.encounter;
    assert_eq!(l.room.id, "F01-HAZARDS");
    assert_eq!(e.goal, Goal::DefeatRoster);
    assert_eq!(e.story_pin, "f01_pin_post_pile_boss");
    assert_eq!(
        l.ceiling_px,
        Some(228),
        "DP-Q45: the room declares its ceiling"
    );
    assert_eq!(e.roster_size(), 4, "S1, the A goblin, S2, the C goblin");
    assert_eq!(e.roster.iter().filter(|r| r.kind.is_slime()).count(), 2);
    assert_eq!(e.acid_patches.len(), 2);
    assert_eq!(
        e.props
            .iter()
            .filter(|p| p.kind == PropKind::RotSticker)
            .count(),
        2
    );
    assert_eq!(e.pockets.len(), 3);
    assert_eq!(e.gates, [ReachHero::Carl, ReachHero::Carl]);
    // The greybox spans of the owner's encounter (#178).
    assert_eq!(l.room.render.0, 1152);
}

#[test]
fn acid_patches_are_not_hazards_so_the_roster_in_hazard_rule_never_sees_them() {
    let l = loaded();
    assert!(l.encounter.hazards.is_empty());
    assert!(l.encounter.hazard_set().unwrap().volumes().is_empty());
    // The room file keeps its geometry arrays empty.
    assert!(l.room.props.is_empty() && l.room.enemies.is_empty() && l.room.hazards.is_empty());
    // S1 spawns inside its pool without any exemption.
    let s1 = l.encounter.roster.iter().find(|r| r.id == "s1").unwrap();
    let (b0, b1) = body_x(s1);
    let patch = l.encounter.acid_patches[0].clone();
    assert!(b0 < patch.x1 && b1 > patch.x0, "S1 is in its pool");
    assert!(s1.allow_in_hazard.is_empty());
}

#[test]
fn the_file_hashes_are_pinned() {
    let l = loaded();
    let room_text = std::fs::read_to_string(root().join(HAZARDS_ROOM_PATH)).unwrap();
    assert_eq!(l.room_hash, fnv64(room_text.as_bytes()));
    assert_eq!(l.room_hash, ROOM_HASH, "update with the room file");
    assert_eq!(l.sidecar_hash, SIDECAR_HASH, "update with the sidecar");
}

#[test]
fn the_room_file_is_the_existing_schema_and_english() {
    let t = std::fs::read_to_string(root().join(HAZARDS_ROOM_PATH)).unwrap();
    let v: Value = serde_json::from_str(&t).unwrap();
    assert_eq!(v["schema"], "floor01-demo-hall-v1");
    let r = loaded().room;
    for s in [&r.name, &r.subtitle, &r.purpose, &r.expected, &r.coop] {
        assert!(s.is_ascii(), "{s:?}");
    }
    let sc = std::fs::read_to_string(root().join(HAZARDS_ENCOUNTER_PATH)).unwrap();
    assert!(sc.is_ascii());
}

#[test]
fn the_room_composes_through_the_existing_kit() {
    use crate::camera_lab::layout::build_kit;
    let l = loaded();
    let (kit, _, _) = build_kit(root(), &l.room).expect("composes");
    assert_eq!((kit.width, kit.height), l.room.render);
}

#[test]
fn the_geometry_is_the_recorded_greybox() {
    let l = loaded();
    let floor = l.room.floor_y;
    assert_eq!(floor, 580);
    // The foothold: 64 px class, free-standing, 40 px before the pool.
    let a = l.room.platforms[0].rect;
    assert_eq!((a.x, floor - a.y), (160, 64));
    assert!(l.encounter.acid_patches[0].x0 - a.right() >= 32);
    // The slab of pocket B: 80 px class (underside 68).
    let b = l.room.platforms[1].rect;
    assert_eq!((b.x, floor - b.y), (496, 80));
    // Pool widths: 64.
    for p in &l.encounter.acid_patches {
        assert_eq!(p.x1 - p.x0, 64);
        assert_eq!(p.floor_y, floor);
    }
    // Both pools drain with their slime.
    assert_eq!(l.encounter.acid_patches[0].linked_slime, Some(0));
    assert_eq!(l.encounter.acid_patches[1].linked_slime, Some(2));
}

#[test]
fn the_c_geometry_keeps_s2_inside_r2s_blast_from_both_track_ends() {
    // The 48 px floor of the Rot blast radius while room C ships: S2 at home
    // (800, facing -x, idle hurtbox 772..832 x 0..32) is 47.5 px from the dome
    // at the start point (760) and inside the circle at the far end (816).
    let l = loaded();
    let t = crate::rot_sticker::active();
    let r2 = l.encounter.props.iter().find(|p| p.id == "r2").unwrap();
    let track = r2.track.unwrap().world_track(l.room.floor_y).unwrap();
    assert_eq!((track.lo, track.hi, track.start), (760, 816, 760));
    let s2 = l.encounter.roster.iter().find(|r| r.id == "s2").unwrap();
    assert_eq!((s2.x, s2.home, s2.facing), (800, 800, -1));
    let hurt = crate::prototype::Rect {
        left: 772.0,
        bottom: 0.0,
        width: 60.0,
        height: 32.0,
    };
    let radius = f32::from(t.hazq30_outer_radius_px);
    assert!(radius >= 48.0);
    for s in [track.lo, track.start, track.hi] {
        let dome = track.dome(s, t.hazq24_dome_offset_px);
        assert!(
            crate::rot_sticker::circle_reaches(dome, radius, hurt, true, &[]),
            "dome at {s}"
        );
    }
    let dome = track.dome(760, 6);
    let d2 = crate::rot_sticker::dist2_to_rect(hurt, dome.0, dome.1);
    assert!((d2.sqrt() - 47.5).abs() < 0.05, "{}", d2.sqrt());
    // A 47 px radius would silently break the collateral.
    assert!(!crate::rot_sticker::circle_reaches(
        dome,
        47.0,
        hurt,
        true,
        &[]
    ));
}

#[test]
fn the_stickers_targets_are_their_shells_and_their_tracks_lie_on_their_surfaces() {
    let l = loaded();
    let floor = l.room.floor_y;
    for p in l
        .encounter
        .props
        .iter()
        .filter(|p| p.kind == PropKind::RotSticker)
    {
        assert!(p.optional, "{}: never needed to clear the room", p.id);
        let lay = p.track.unwrap();
        let tr = lay.world_track(floor).unwrap();
        assert!(tr.length() <= 160 && tr.length() > 0);
        assert_eq!(lay.orientation, Orientation::Ceiling);
        let shell = tr.shell(tr.start, &crate::rot_sticker::active().hazq24_hurtbox);
        assert_eq!(
            IRect::new(shell.left as i32, floor - shell.top() as i32, 20, 14),
            p.target
        );
    }
}

// ------------------------------------------------------------------ rule 0

#[test]
fn schema_appends_deny_unknown_fields_and_name_the_field() {
    let mut v = sidecar();
    v["acid_patches"][0]["depth"] = json!(3);
    let errs = check(&v).unwrap_err();
    assert!(text(&errs).contains("depth"), "{}", text(&errs));
    let mut v = sidecar();
    v["pockets"][0]["max_birds"] = json!(3);
    assert!(text(&check(&v).unwrap_err()).contains("max_birds"));
    let mut v = sidecar();
    v["props"][0]["track"]["bend"] = json!(3);
    assert!(text(&check(&v).unwrap_err()).contains("bend"));
    let mut v = sidecar();
    v["roster"][0]["glow"] = json!(1);
    assert!(text(&check(&v).unwrap_err()).contains("glow"));
    // A missing track field is a parse error too.
    let mut v = sidecar();
    v["props"][0]["track"]
        .as_object_mut()
        .unwrap()
        .remove("start");
    assert!(text(&check(&v).unwrap_err()).contains("start"));
}

#[test]
fn the_slime_roster_fields_are_validated() {
    // A slime needs a facing of 1 or -1.
    let mut v = sidecar();
    v["roster"][0].as_object_mut().unwrap().remove("facing");
    assert!(has_rule(&errors(&v), 0, &["s1"]));
    let mut v = sidecar();
    v["roster"][0]["facing"] = json!(2);
    assert!(has_rule(&errors(&v), 0, &["s1"]));
    // A goblin has none.
    let mut v = sidecar();
    v["roster"][1]["facing"] = json!(1);
    assert!(has_rule(&errors(&v), 0, &["g_a"]));
    let mut v = sidecar();
    v["roster"][1]["home"] = json!(380);
    assert!(has_rule(&errors(&v), 0, &["g_a"]));
    // A slime has no perch, gate or fallback.
    let mut v = sidecar();
    v["roster"][0]["perch"] = json!(0);
    assert!(has_rule(&errors(&v), 0, &["s1"]));
    // Home within the leash of the spawn.
    let mut v = sidecar();
    v["roster"][0]["home"] = json!(150);
    assert!(has_rule(&errors(&v), 0, &["s1"]));
    // linked_pool must name a patch that names the slime back.
    let mut v = sidecar();
    v["roster"][0]["linked_pool"] = json!("2");
    assert!(has_rule(&errors(&v), 0, &["s1"]));
    let mut v = sidecar();
    v["acid_patches"][0]["linked_slime"] = json!("g_a");
    assert!(has_rule(&errors(&v), 0, &["1", "g_a"]));
    let mut v = sidecar();
    v["acid_patches"][0]["linked_slime"] = json!("nobody");
    assert!(has_rule(&errors(&v), 0, &["1", "nobody"]));
    // Duplicate and reversed patches.
    let mut v = sidecar();
    v["acid_patches"][1]["id"] = json!(1);
    assert!(has_rule(&errors(&v), 0, &["1"]));
    let mut v = sidecar();
    v["acid_patches"][0]["x1"] = json!(200);
    assert!(has_rule(&errors(&v), 0, &["1"]));
}

#[test]
fn defeat_roster_needs_a_roster_and_the_old_goals_still_parse() {
    let mut v = sidecar();
    v["roster"] = json!([]);
    v["acid_patches"] = json!([]);
    v["pockets"] = json!([]);
    assert!(has_rule(&errors(&v), 10, &[]));
    let mut v = sidecar();
    v["goal"] = json!("reach_exit");
    assert!(check(&v).is_ok(), "reach_exit is still valid for the room");
}

// ------------------------------------------------------------------- rule 1

#[test]
fn rule_1_an_acid_patch_width_dryness_surface_and_clearance() {
    // Width 32..80.
    let mut v = sidecar();
    v["acid_patches"][0]["x1"] = json!(248 + 90);
    assert!(has_rule(&errors(&v), 1, &["1"]), "90 px is too wide");
    let mut v = sidecar();
    v["acid_patches"][0]["x1"] = json!(248 + 80);
    assert!(
        check(&v).is_ok(),
        "80 px is the widest allowed (and clearable)"
    );
    let mut v = sidecar();
    v["acid_patches"][0]["x1"] = json!(248 + 31);
    assert!(has_rule(&errors(&v), 1, &["1"]), "31 px is too narrow");
    // Dry floor of at least 48 px on both sides: a patch 8 px from the step.
    let mut v = sidecar();
    v["acid_patches"][0]["x0"] = json!(168);
    v["acid_patches"][0]["x1"] = json!(232);
    let errs = errors(&v);
    assert!(
        rule(&errs, 1)
            .iter()
            .any(|e| e.to_string().contains("dry floor on the left")),
        "{}",
        text(&errs)
    );
    // A second patch 20 px from the first: the shared gap is under 48.
    let mut v = sidecar();
    v["acid_patches"][1]["x0"] = json!(332);
    v["acid_patches"][1]["x1"] = json!(396);
    assert!(
        rule(&errors(&v), 1)
            .iter()
            .any(|e| e.to_string().contains("dry floor")),
    );
    // On the floor lane only.
    let mut v = sidecar();
    v["acid_patches"][0]["floor_y"] = json!(516);
    assert!(has_rule(&errors(&v), 1, &["1"]));
    // Never inside the protected spawn.
    let mut v = sidecar();
    v["acid_patches"][0]["x0"] = json!(40);
    v["acid_patches"][0]["x1"] = json!(104);
    let errs = errors(&v);
    assert!(
        rule(&errs, 1)
            .iter()
            .any(|e| e.to_string().contains("protected spawn")),
        "{}",
        text(&errs)
    );
    // Outside the play area.
    let mut v = sidecar();
    v["acid_patches"][1]["x0"] = json!(1100);
    v["acid_patches"][1]["x1"] = json!(1160);
    assert!(has_rule(&errors(&v), 1, &["2"]));
}

#[test]
fn rule_1_a_pool_under_the_soffit_cannot_be_jumped() {
    // The sheet's recommended C geometry (pool 768..832 under the 84 px soffit)
    // bonks every jump at feet 32: Carl cannot clear 64 + 24 px. The shipped
    // data moves the pool right of the soffit.
    let mut v = sidecar();
    v["acid_patches"][1]["x0"] = json!(768);
    v["acid_patches"][1]["x1"] = json!(832);
    let errs = errors(&v);
    assert!(has_rule(&errs, 1, &["2"]), "{}", text(&errs));
    assert!(
        rule(&errs, 1)
            .iter()
            .any(|e| e.to_string().contains("not clearable")),
        "{}",
        text(&errs)
    );
    // Rule 2 follows: no jump route for Carl and base Donut.
    assert!(has_rule(&errs, 2, &["carl"]) && has_rule(&errs, 2, &["donut_base"]));
    // A pool whose take-off zone is under the soffit's side is the same.
    let mut v = sidecar();
    v["acid_patches"][1]["x0"] = json!(840);
    v["acid_patches"][1]["x1"] = json!(904);
    assert!(
        has_rule(&errors(&v), 1, &["2"]),
        "the soffit (to 824) reaches into the zone 808..840"
    );
    // The shipped pool starts exactly where the zone clears the soffit.
    assert!(check(&sidecar()).is_ok());
}

#[test]
fn rule_1_a_solid_in_the_take_off_zone_is_rejected() {
    // A pool 8 px beyond the step at 432..496 puts the step in its zone.
    let mut v = sidecar();
    v["acid_patches"][0]["x0"] = json!(504);
    v["acid_patches"][0]["x1"] = json!(568);
    let errs = errors(&v);
    assert!(
        rule(&errs, 1)
            .iter()
            .any(|e| e.to_string().contains("take-off zone") || e.to_string().contains("dry floor")),
        "{}",
        text(&errs)
    );
}

#[test]
fn the_jump_check_uses_the_real_arc() {
    let classes = jump_classes();
    let volume_h = 6.0;
    let no_ceiling: [Ceiling; 0] = [];
    // Every class clears the widest allowed patch from every take-off gap.
    for c in classes {
        for gap in 0..=TAKEOFF_ZONE_PX {
            assert_eq!(
                jump_clears(c, gap, (200, 280), &[(200, 280)], &no_ceiling, volume_h),
                Ok(()),
                "{} gap {gap}",
                c.name
            );
        }
    }
    // A 130 px patch is too wide for Carl's jump.
    assert!(
        jump_clears(
            classes[0],
            0,
            (200, 330),
            &[(200, 330)],
            &no_ceiling,
            volume_h
        )
        .is_err()
    );
    // A low ceiling (underside 84) bonks the arc short.
    let low = [Ceiling {
        x0: f32::NEG_INFINITY,
        x1: f32::INFINITY,
        underside: 84.0,
    }];
    assert!(jump_clears(classes[0], 0, (200, 264), &[(200, 264)], &low, volume_h).is_err());
    // The lane ceiling of the room (228) does not touch any class's arc.
    let lane = [Ceiling {
        x0: f32::NEG_INFINITY,
        x1: f32::INFINITY,
        underside: 228.0,
    }];
    for c in classes {
        assert!(jump_clears(c, 0, (200, 280), &[(200, 280)], &lane, volume_h).is_ok());
    }
    // The classes come from the pinned tuning files, not copies.
    assert_eq!(classes[0].launch, crate::prototype::JUMP_SPEED);
    assert_eq!(classes[1].launch, 500.0);
    assert_eq!(classes[2].launch, 547.0);
}

// ------------------------------------------------------------------- rule 3

#[test]
fn rule_3_the_track_the_surface_the_target_and_the_pockets() {
    // The orientation matches the surface: a ceiling track off the underside.
    let mut v = sidecar();
    v["props"][0]["track"]["a"]["y"] = json!(500);
    v["props"][0]["track"]["b"]["y"] = json!(500);
    v["props"][0]["track"]["start"]["y"] = json!(500);
    v["props"][0]["target"]["y"] = json!(500);
    assert!(has_rule(&errors(&v), 3, &["r1"]));
    // A track beyond the surface's end.
    let mut v = sidecar();
    v["props"][0]["track"]["b"]["x"] = json!(640);
    assert!(has_rule(&errors(&v), 3, &["r1"]));
    // A track longer than 160 px.
    let mut v = sidecar();
    v["props"][1]["track"]["b"]["x"] = json!(930);
    assert!(has_rule(&errors(&v), 3, &["r2"]));
    // The target is exactly the shell at the start point.
    let mut v = sidecar();
    v["props"][0]["target"]["x"] = json!(500);
    let errs = errors(&v);
    assert!(
        rule(&errs, 3)
            .iter()
            .any(|e| e.to_string().contains("shell at the start point")),
        "{}",
        text(&errs)
    );
    // An unknown orientation.
    let mut v = sidecar();
    v["props"][0]["track"]["orientation"] = json!("floor");
    assert!(has_rule(&errors(&v), 3, &["r1"]));
    // An off-axis end point.
    let mut v = sidecar();
    v["props"][0]["track"]["b"]["y"] = json!(513);
    assert!(has_rule(&errors(&v), 3, &["r1"]));
    // Start off the track.
    let mut v = sidecar();
    v["props"][0]["track"]["start"]["x"] = json!(620);
    assert!(has_rule(&errors(&v), 3, &["r1"]));
}

#[test]
fn rule_3_a_blast_free_pocket_of_64_px_on_each_side() {
    // R1's track moved 24 px right: its sweep ends 56 px before R2's.
    let mut v = sidecar();
    v["props"][0]["track"]["a"]["x"] = json!(536);
    v["props"][0]["track"]["b"]["x"] = json!(624);
    v["props"][0]["track"]["start"]["x"] = json!(624);
    v["props"][0]["target"]["x"] = json!(614);
    let errs = errors(&v);
    assert!(
        rule(&errs, 3)
            .iter()
            .any(|e| e.to_string().contains("blast-free floor")),
        "{}",
        text(&errs)
    );
    // The shipped pockets are exactly 64 px (R1 right / R2 left): the limit.
    assert!(check(&sidecar()).is_ok());
}

#[test]
fn rule_3_the_sweep_keeps_off_the_protected_zones() {
    // A sticker whose sweep reaches the exit rectangle.
    let mut v = sidecar();
    v["props"][1]["track"]["surface"] = json!({ "x": 1000, "y": 484, "w": 70, "h": 12 });
    v["props"][1]["track"]["a"] = json!({ "x": 1010, "y": 496 });
    v["props"][1]["track"]["b"] = json!({ "x": 1050, "y": 496 });
    v["props"][1]["track"]["start"] = json!({ "x": 1010, "y": 496 });
    v["props"][1]["target"] = json!({ "x": 1000, "y": 496, "w": 20, "h": 14 });
    let errs = errors(&v);
    assert!(
        rule(&errs, 3)
            .iter()
            .any(|e| e.to_string().contains("protected")),
        "{}",
        text(&errs)
    );
}

#[test]
fn rule_3_the_dome_stays_in_the_camera_band_and_under_the_lane_ceiling() {
    // A wall sticker 210 px up (dome) is outside the 200 px band.
    let mut v = sidecar();
    v["props"][1] = json!({
        "id": "r2",
        "kind": "rot_sticker",
        "optional": true,
        "target": { "x": 800, "y": 360, "w": 14, "h": 20 },
        "track": {
            "orientation": "wall_left",
            "surface": { "x": 760, "y": 300, "w": 40, "h": 150 },
            "a": { "x": 800, "y": 370 },
            "b": { "x": 800, "y": 370 },
            "start": { "x": 800, "y": 370 }
        }
    });
    let errs = errors(&v);
    assert!(
        rule(&errs, 3)
            .iter()
            .any(|e| e.to_string().contains("at most 200")),
        "{}",
        text(&errs)
    );
}

/// A wall sticker prop `r2` on `surface`, its face at `face_x`, dome 32 px up.
fn wall_sticker(orientation: &str, surface: Value, face_x: i32) -> Value {
    let (tx, y) = if orientation == "wall_left" {
        (face_x, 548)
    } else {
        (face_x - 14, 548)
    };
    json!({
        "id": "r2",
        "kind": "rot_sticker",
        "optional": true,
        "target": { "x": tx, "y": y - 10, "w": 14, "h": 20 },
        "track": {
            "orientation": orientation,
            "surface": surface,
            "a": { "x": face_x, "y": y },
            "b": { "x": face_x, "y": y },
            "start": { "x": face_x, "y": y }
        }
    })
}

#[test]
fn a_wall_sticker_lies_on_the_face_of_a_room_solid() {
    let l = loaded();
    let step = *l.room.steps.first().expect("the room has access steps");
    let surf = json!({ "x": step.x, "y": step.y, "w": step.w, "h": step.h });
    // On a room step: wall_left on its right face, wall_right on its left face.
    // Only pockets and bounds of the edit may fail, never the surface rules.
    for (o, face) in [("wall_left", step.right()), ("wall_right", step.x)] {
        let mut v = sidecar();
        v["props"][1] = wall_sticker(o, surf.clone(), face);
        let errs = check(&v).err().unwrap_or_default();
        assert!(
            rule(&errs, 3)
                .iter()
                .all(|e| !e.to_string().contains("matches no room")
                    && !e.to_string().contains("face of its surface")),
            "{o}: {}",
            text(&errs)
        );
    }
    // The wrong orientation for the same face does not lie on it.
    let mut v = sidecar();
    v["props"][1] = wall_sticker("wall_right", surf, step.right());
    assert!(
        rule(&errors(&v), 3)
            .iter()
            .any(|e| e.to_string().contains("face of its surface")),
    );
}

#[test]
fn rule_3_an_invented_wall_is_rejected_left_and_right_re_review_fix_now_1() {
    // wall_left: a "boundary" surface of any width puts the face mid-air.
    let mut v = sidecar();
    v["props"][1] = wall_sticker(
        "wall_left",
        json!({ "x": 16, "y": 448, "w": 684, "h": 132 }),
        700,
    );
    let errs = errors(&v);
    assert!(
        rule(&errs, 3)
            .iter()
            .any(|e| e.to_string().contains("matches no room")),
        "{}",
        text(&errs)
    );
    // wall_right: the mirror, the surface reaching the right boundary 1136.
    let mut v = sidecar();
    v["props"][1] = wall_sticker(
        "wall_right",
        json!({ "x": 700, "y": 448, "w": 436, "h": 132 }),
        700,
    );
    let errs = errors(&v);
    assert!(
        rule(&errs, 3)
            .iter()
            .any(|e| e.to_string().contains("matches no room")),
        "{}",
        text(&errs)
    );
    // The true boundary wall is not a HAZ-A1 surface either (no boundary branch).
    let mut v = sidecar();
    v["props"][1] = wall_sticker(
        "wall_left",
        json!({ "x": 0, "y": 448, "w": 16, "h": 132 }),
        16,
    );
    assert!(
        rule(&errors(&v), 3)
            .iter()
            .any(|e| e.to_string().contains("matches no room")),
    );
}

#[test]
fn rule_3_the_surface_is_real_room_geometry_f2() {
    // R1 with its surface shifted 16 px: no platform slab is there.
    let mut v = sidecar();
    v["props"][0]["track"]["surface"]["x"] = json!(512);
    assert!(has_rule(&errors(&v), 3, &["r1"]));
    let errs = errors(&v);
    assert!(
        rule(&errs, 3)
            .iter()
            .any(|e| e.to_string().contains("matches no room")),
        "{}",
        text(&errs)
    );
    // A sticker floating in free air under an invented slab.
    let mut v = sidecar();
    v["props"][0]["track"]["surface"] = json!({ "x": 496, "y": 400, "w": 128, "h": 12 });
    for k in ["a", "b", "start"] {
        v["props"][0]["track"][k]["y"] = json!(412);
    }
    v["props"][0]["target"]["y"] = json!(412);
    let errs = errors(&v);
    assert!(
        rule(&errs, 3)
            .iter()
            .any(|e| e.to_string().contains("matches no room")),
        "{}",
        text(&errs)
    );
    // A soffit that stops short of the lane ceiling (top 220, not 228) is not
    // an authored full-height solid.
    let mut v = sidecar();
    v["props"][1]["track"]["surface"] = json!({ "x": 744, "y": 360, "w": 80, "h": 136 });
    assert!(has_rule(&errors(&v), 3, &["r2"]));
    // The shipped soffit: bottom at 496 (underside 84), top at the ceiling.
    let l = loaded();
    let r2 = l.encounter.props.iter().find(|p| p.id == "r2").unwrap();
    let sf = r2.track.unwrap().surface;
    assert_eq!(sf, IRect::new(744, 352, 80, 144));
    assert_eq!(l.room.floor_y - sf.y, 228, "the top reaches ceiling_px");
    assert_eq!(l.room.floor_y - sf.bottom(), 84);
    assert!(check(&sidecar()).is_ok());
}

// ------------------------------------------------------------------- rule 4

#[test]
fn rule_4_tracks_and_charges_keep_more_than_56_px_apart() {
    let mut v = sidecar();
    let mut r3 = v["props"][0].clone();
    r3["id"] = json!("r3");
    v["props"].as_array_mut().unwrap().push(r3);
    let errs = errors(&v);
    assert!(has_rule(&errs, 4, &["r1", "r3"]), "{}", text(&errs));
    // An authored charge (a drum) within 56 px of a track.
    let mut v = sidecar();
    v["props"].as_array_mut().unwrap().push(json!({
        "id": "drum",
        "kind": "drum",
        "optional": true,
        "target": { "x": 560, "y": 548, "w": 32, "h": 32 },
        "solid_intact": true,
        "blast_radius_px": 48
    }));
    v["hazards"] = json!([{
        "id": 1, "name": "drum_blast", "class": "blast", "owner": "drum",
        "rect": { "x": 600, "y": 500, "w": 96, "h": 80 },
        "telegraph_ticks": 24, "active_ticks": 1, "max_activations": 1,
        "damage": 1, "push_x": 0, "hero_damage": "all_heroes", "hurts_enemies": true
    }]);
    let errs = errors(&v);
    assert!(has_rule(&errs, 4, &["r1", "drum"]), "{}", text(&errs));
}

// ------------------------------------------------------------------- rule 5

#[test]
fn rule_5_counts_per_pocket_and_caps() {
    // A second sticker in pocket B (cap 1).
    let mut v = sidecar();
    v["props"][1] = json!({
        "id": "r2",
        "kind": "rot_sticker",
        "optional": true,
        "target": { "x": 500, "y": 512, "w": 20, "h": 14 },
        "track": {
            "orientation": "ceiling",
            "surface": { "x": 496, "y": 500, "w": 128, "h": 12 },
            "a": { "x": 510, "y": 512 },
            "b": { "x": 510, "y": 512 },
            "start": { "x": 510, "y": 512 }
        }
    });
    let errs = errors(&v);
    assert!(has_rule(&errs, 5, &["b"]), "{}", text(&errs));
    // A pocket limit over the global cap.
    let mut v = sidecar();
    v["pockets"][0]["max_slimes"] = json!(2);
    assert!(has_rule(&errors(&v), 5, &["a"]));
    // A second slime in pocket A.
    let mut v = sidecar();
    v["roster"][1] = json!({ "id": "s3", "kind": "slime", "x": 350, "facing": -1, "beat": "A", "delay_ticks": 60 });
    assert!(has_rule(&errors(&v), 5, &["a"]));
    // An enemy in no pocket.
    let mut v = sidecar();
    v["roster"][1]["x"] = json!(1050);
    assert!(has_rule(&errors(&v), 5, &["g_a"]));
    // A pocket outside the play area, a duplicate pocket.
    let mut v = sidecar();
    v["pockets"][0]["x0"] = json!(0);
    assert!(has_rule(&errors(&v), 5, &["a"]));
    let mut v = sidecar();
    v["pockets"][1]["id"] = json!("a");
    assert!(has_rule(&errors(&v), 5, &["a"]));
}

#[test]
fn the_roster_cap_counts_slimes_and_stays_at_four() {
    let mut v = sidecar();
    v["roster"].as_array_mut().unwrap().push(json!(
        { "id": "g_x", "kind": "goblin", "x": 1000, "beat": "C", "delay_ticks": 90 }
    ));
    let errs = errors(&v);
    assert!(
        errs.iter()
            .any(|e| matches!(e, EncounterError::TooManyLiving { .. })),
        "{}",
        text(&errs)
    );
}

// ------------------------------------------------------------------- rule 6

#[test]
fn rule_6_only_a_slime_spawns_in_acid_and_only_in_its_own() {
    // A goblin in the pool.
    let mut v = sidecar();
    v["roster"][1]["x"] = json!(280);
    assert!(has_rule(&errors(&v), 6, &["g_a", "1"]));
    // S2 in S1's pool.
    let mut v = sidecar();
    v["roster"][2]["x"] = json!(290);
    v["roster"][2]["home"] = json!(290);
    let errs = errors(&v);
    assert!(has_rule(&errs, 6, &["s2", "1"]), "{}", text(&errs));
    // A slime on dry floor (S2 as shipped) and in its own pool (S1) are fine.
    assert!(check(&sidecar()).is_ok());
}

// ------------------------------------------------------------------- rule 7

#[test]
fn rule_7_a_goblin_behind_a_patch_needs_a_drain_or_a_jump() {
    // Unlinked patches that are clearable by a jump are fine.
    let mut v = sidecar();
    v["acid_patches"][0]
        .as_object_mut()
        .unwrap()
        .remove("linked_slime");
    v["roster"][0]
        .as_object_mut()
        .unwrap()
        .remove("linked_pool");
    v["roster"][0]["x"] = json!(200);
    v["roster"][0]["home"] = json!(200);
    let r = check(&v);
    assert!(
        r.is_ok(),
        "an unlinked, jumpable pool is allowed: {}",
        r.err().map(|e| text(&e)).unwrap_or_default()
    );
    // An unlinked patch that cannot be jumped (under the soffit) with a goblin
    // behind it is a stand-off softlock.
    let mut v = sidecar();
    v["acid_patches"][1] = json!({ "id": 2, "x0": 768, "x1": 832, "floor_y": 580 });
    v["roster"][2]
        .as_object_mut()
        .unwrap()
        .remove("linked_pool");
    let errs = errors(&v);
    assert!(has_rule(&errs, 7, &["g_c", "2"]), "{}", text(&errs));
    // Linked (drains with its slime): rule 7 holds (rules 1 and 2 still fail).
    let mut v = sidecar();
    v["acid_patches"][1]["x0"] = json!(768);
    v["acid_patches"][1]["x1"] = json!(832);
    assert!(rule(&errors(&v), 7).is_empty());
}

// ------------------------------------------------------------------- rule 9

#[test]
fn rule_9_at_most_one_kind_supplies_the_boss_input() {
    let kinds = ["slime", "slime", "goblin"];
    assert_eq!(distinct_suppliers(kinds, |k| k == "slime"), 1);
    assert_eq!(
        distinct_suppliers(["slime", "pile", "slime"], |k| k != "goblin"),
        2
    );
    assert_eq!(distinct_suppliers::<&str>([], |_| true), 0);
    // Today only the slime supplies; the shipped roster has two slimes: one kind.
    let l = loaded();
    assert_eq!(
        distinct_suppliers(
            l.encounter.roster.iter().map(|e| e.kind),
            RosterKind::supplies_boss
        ),
        1
    );
    assert!(RosterKind::Slime.supplies_boss());
    assert!(!RosterKind::Room(EnemyKind::Goblin).supplies_boss());
    // HRD-A1: the Hoarder supplies the boss input in her own room.
    assert!(RosterKind::Room(EnemyKind::Hoarder).supplies_boss());
}

#[test]
fn rule_9_fails_for_a_slime_plus_the_hoarder_and_passes_for_the_hoarder_alone() {
    // A real roster of a slime and the Hoarder: two supplier kinds.
    let mut v = sidecar();
    v["roster"].as_array_mut().unwrap().push(json!({
        "id": "hoard", "kind": "hoarder", "x": 600, "beat": "H1",
        "trigger_x": 432, "delay_ticks": 7
    }));
    let errs = errors(&v);
    assert!(
        !rule(&errs, 9).is_empty(),
        "a slime plus the Hoarder must fail rule 9: {}",
        text(&errs)
    );
    let kinds = [RosterKind::Slime, RosterKind::Room(EnemyKind::Hoarder)];
    match supplier_conflict(kinds, RosterKind::supplies_boss) {
        Some(EncounterError::Haz { rule, .. }) => assert_eq!(rule, 9),
        other => panic!("{other:?}"),
    }
    // The Hoarder alone is one supplier kind (and her shipped room validates).
    let alone = [RosterKind::Room(EnemyKind::Hoarder)];
    assert!(supplier_conflict(alone, RosterKind::supplies_boss).is_none());
    let hoarder_room =
        load(root(), crate::pile_boss::ENCOUNTER_PATH).unwrap_or_else(|e| panic!("{}", text(&e)));
    assert_eq!(
        distinct_suppliers(
            hoarder_room.encounter.roster.iter().map(|e| e.kind),
            RosterKind::supplies_boss
        ),
        1
    );
}

// ------------------------------------------------------------------ rule 10

#[test]
fn rule_10_no_step_depends_on_a_sticker() {
    let mut v = sidecar();
    v["props"][0]["optional"] = json!(false);
    assert!(has_rule(&errors(&v), 10, &["r1"]));
    // A sticker owns no hazard volume (no double blast).
    let mut v = sidecar();
    v["hazards"] = json!([{
        "id": 7, "name": "r1_blast", "class": "blast", "owner": "r1",
        "rect": { "x": 540, "y": 480, "w": 96, "h": 100 },
        "telegraph_ticks": 24, "active_ticks": 1, "max_activations": 1,
        "damage": 1, "push_x": 0, "hero_damage": "all_heroes", "hurts_enemies": true
    }]);
    let errs = errors(&v);
    assert!(
        errs.iter()
            .any(|e| matches!(e, EncounterError::HazardOwnerMismatch { prop, .. } if prop == "r1")),
        "{}",
        text(&errs)
    );
    // Only a sticker has a track; a sticker needs one.
    let mut v = sidecar();
    v["props"][0].as_object_mut().unwrap().remove("track");
    assert!(
        errors(&v)
            .iter()
            .any(|e| matches!(e, EncounterError::PropFields { prop, .. } if prop == "r1"))
    );
}

// --------------------------------------------------------------- misc units

#[test]
fn segment_distance_is_exact_for_axis_aligned_tracks() {
    let d = segment_distance((0.0, 0.0), (10.0, 0.0), (0.0, 5.0), (10.0, 5.0));
    assert_eq!(d, 5.0);
    let d = segment_distance((0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (30.0, 0.0));
    assert_eq!(d, 10.0);
    assert_eq!(
        segment_distance((0.0, 0.0), (10.0, 10.0), (0.0, 10.0), (10.0, 0.0)),
        0.0
    );
    // A point (a charge) against a segment.
    assert_eq!(
        segment_distance((0.0, 0.0), (10.0, 0.0), (5.0, 3.0), (5.0, 3.0)),
        3.0
    );
    assert_eq!(
        segment_distance((0.0, 0.0), (10.0, 0.0), (13.0, 4.0), (13.0, 4.0)),
        5.0
    );
}

#[test]
fn the_validation_is_deterministic() {
    let a = loaded();
    let b = loaded();
    assert_eq!(a, b);
    assert_eq!(
        text(&errors(&{
            let mut v = sidecar();
            v["acid_patches"][0]["x1"] = json!(400);
            v
        })),
        text(&errors(&{
            let mut v = sidecar();
            v["acid_patches"][0]["x1"] = json!(400);
            v
        }))
    );
}

#[test]
fn the_boiler_room_is_unaffected_by_the_hazard_rules() {
    let l = load(root(), BOILER_ENCOUNTER_PATH).unwrap();
    assert!(l.encounter.acid_patches.is_empty() && l.encounter.pockets.is_empty());
    assert!(l.encounter.roster.iter().all(|r| !r.kind.is_slime()));
}

#[test]
fn rule_5_hazard_entities_require_declared_pockets_f3() {
    // The shipped sidecar with `pockets` removed.
    let mut v = sidecar();
    v.as_object_mut().unwrap().remove("pockets");
    let errs = errors(&v);
    assert!(
        rule(&errs, 5)
            .iter()
            .any(|e| e.to_string().contains("require declared pockets")),
        "{}",
        text(&errs)
    );
    let mut v = sidecar();
    v["pockets"] = json!([]);
    assert!(has_rule(&errors(&v), 5, &[]));
    // A slime whose home is outside every pocket.
    let mut v = sidecar();
    v["pockets"][0]["x0"] = json!(300);
    let errs = errors(&v);
    assert!(has_rule(&errs, 5, &["s1"]), "{}", text(&errs));
    assert!(
        rule(&errs, 5)
            .iter()
            .any(|e| e.to_string().contains("home lies in no pocket")),
        "{}",
        text(&errs)
    );
    // An acid patch outside every pocket.
    let mut v = sidecar();
    v["pockets"][0]["x0"] = json!(260);
    let errs = errors(&v);
    assert!(
        rule(&errs, 5)
            .iter()
            .any(|e| e.to_string().contains("acid patch lies in no pocket")),
        "{}",
        text(&errs)
    );
    // Overlapping pockets.
    let mut v = sidecar();
    v["pockets"][1]["x0"] = json!(380);
    assert!(has_rule(&errors(&v), 5, &["a", "b"]));
    // A room with no hazard entities needs no pockets.
    let mut v = sidecar();
    v["props"] = json!([]);
    v["acid_patches"] = json!([]);
    v["pockets"] = json!([]);
    v["roster"] =
        json!([{ "id": "g_a", "kind": "goblin", "x": 380, "beat": "A", "delay_ticks": 60 }]);
    let r = check(&v);
    assert!(
        r.is_ok(),
        "{}",
        r.err().map(|e| text(&e)).unwrap_or_default()
    );
}

#[test]
fn rule_9_fails_with_a_second_supplier_kind() {
    // Only the slime supplies today; a synthetic second supplier kind (a goblin
    // that also supplies the input) must be rejected.
    let l = loaded();
    let kinds: Vec<RosterKind> = l.encounter.roster.iter().map(|e| e.kind).collect();
    let both = |k: RosterKind| k.is_slime() || k == RosterKind::Room(EnemyKind::Goblin);
    match supplier_conflict(kinds.iter().copied(), both) {
        Some(EncounterError::Haz { rule, .. }) => assert_eq!(rule, 9),
        other => panic!("{other:?}"),
    }
    assert!(supplier_conflict(kinds.iter().copied(), RosterKind::supplies_boss).is_none());
}

// ------------------------------------------- PT-TUNE: the wave-room rule 5 (PTT-Q7)

const MASTERY_PATH: &str = "assets/levels/f01_late/mastery.encounter.json";

/// The mastery room's context (the CF02 microarena geometry).
fn wctx() -> Context {
    let l = load(root(), MASTERY_PATH).unwrap_or_else(|e| panic!("{}", text(&e)));
    Context {
        room: l.room,
        classes: l.classes,
        ceiling_px: l.ceiling_px,
    }
}

/// The shipped mastery sidecar with its composition FROZEN: the pockets, the
/// acid patch and the roster below are fixture values, so a composition retune of
/// the shipped data (PT-TUNE iterations) never moves these validator tests. Only
/// the room, the spawn, the checkpoint, the exit and the Rot Sticker prop come
/// from the file.
fn wsidecar() -> Value {
    let t = std::fs::read_to_string(root().join(MASTERY_PATH)).unwrap();
    let mut v: Value = serde_json::from_str(&t).unwrap();
    v["pockets"] = json!([
        { "id": "a", "x0": 176, "x1": 352, "max_stickers": 0, "max_slimes": 1, "max_goblins": 0 },
        { "id": "b", "x0": 880, "x1": 944, "max_stickers": 1, "max_slimes": 0, "max_goblins": 0 }
    ]);
    v["acid_patches"] =
        json!([{ "id": 1, "x0": 272, "x1": 336, "floor_y": 580, "linked_slime": "s1" }]);
    v["roster"] = json!([
        { "id": "s1", "kind": "slime", "x": 304, "facing": -1, "home": 304,
          "linked_pool": "1", "beat": "W1", "delay_ticks": 0 },
        { "id": "l2", "kind": "llama", "x": 616, "beat": "W1", "delay_ticks": 30 },
        { "id": "g1", "kind": "goblin", "x": 380, "beat": "W1", "delay_ticks": 60 },
        { "id": "g2", "kind": "goblin", "x": 460, "beat": "W1", "delay_ticks": 90 },
        { "id": "g3", "kind": "goblin", "x": 784, "beat": "W2", "delay_ticks": 120 },
        { "id": "g4", "kind": "goblin", "x": 832, "beat": "W2", "delay_ticks": 150 },
        { "id": "l1", "kind": "llama", "x": 1000, "beat": "W2", "delay_ticks": 180 },
        { "id": "e1", "kind": "engineer", "gate": "cf02_engineer", "fallback_kind": "goblin",
          "x": 1036, "beat": "W2", "delay_ticks": 210 }
    ]);
    v
}

fn wcheck(v: &Value) -> Result<Encounter, Vec<EncounterError>> {
    parse_sidecar(&v.to_string(), &wctx())
}

/// The minimal waves fixture: the shipped room's slime pocket (a, 176..352,
/// acid 272..336), its Rot pocket (b, 880..944, sweep 864..976), the slime and
/// two goblins on dry floor outside both.
fn minimal_waves() -> Value {
    let mut v = wsidecar();
    v["roster"] = json!([
        { "id": "s1", "kind": "slime", "x": 304, "facing": -1, "home": 304,
          "linked_pool": "1", "beat": "W1", "delay_ticks": 0 },
        { "id": "g1", "kind": "goblin", "x": 520, "beat": "W1", "delay_ticks": 60 },
        { "id": "g3", "kind": "goblin", "x": 784, "beat": "W2", "delay_ticks": 120 }
    ]);
    v
}

fn with_goblin(x: i64, kind: &str) -> Value {
    let mut v = minimal_waves();
    v["roster"][1] = json!({ "id": "g1", "kind": kind, "x": x, "beat": "W1", "delay_ticks": 60 });
    v
}

fn wave_rule5(errs: &[EncounterError]) -> Vec<String> {
    rule(errs, 5)
        .into_iter()
        .map(ToString::to_string)
        .filter(|t| t.contains("pocket or a blast sweep"))
        .collect()
}

#[test]
fn p1_the_minimal_waves_fixture_is_valid() {
    let r = wcheck(&minimal_waves());
    assert!(
        r.is_ok(),
        "{}",
        r.err().map(|e| text(&e)).unwrap_or_default()
    );
}

#[test]
fn n1_a_wave_goblin_in_a_pocket_is_refused() {
    let errs = wcheck(&with_goblin(900, "goblin")).expect_err("a goblin in pocket b");
    assert!(has_rule(&errs, 5, &["g1", "b"]), "{}", text(&errs));
    assert!(!wave_rule5(&errs).is_empty());
}

#[test]
fn n2_a_wave_goblin_in_a_blast_sweep_only_is_refused_naming_the_sticker() {
    // x 855: body 843..867 overlaps the sweep 864..976 but not pocket b (880..).
    let errs = wcheck(&with_goblin(855, "goblin")).expect_err("a goblin in the sweep");
    assert!(has_rule(&errs, 5, &["g1", "r1"]), "{}", text(&errs));
    assert!(
        !has_rule(&errs, 5, &["g1", "b"]),
        "it is not in the pocket: {}",
        text(&errs)
    );
    // The sweep is half-open: a body ending at 864 passes (x 852).
    assert!(wcheck(&with_goblin(852, "goblin")).is_ok());
}

#[test]
fn n3_the_pocket_edge_is_half_open_for_a_llama() {
    // Pocket a is 176..352; a body is 24 wide: x 363 covers 351..375.
    let errs = wcheck(&with_goblin(363, "llama")).expect_err("one pixel inside");
    assert!(has_rule(&errs, 5, &["g1", "a"]), "{}", text(&errs));
    // x 364 covers 352..376: it touches the edge and passes.
    let r = wcheck(&with_goblin(364, "llama"));
    assert!(
        r.is_ok(),
        "{}",
        r.err().map(|e| text(&e)).unwrap_or_default()
    );
    // The left edge: x 164 covers 152..176 (touching) passes; x 165 fails.
    assert!(wave_rule5(&wcheck(&with_goblin(164, "llama")).err().unwrap_or_default()).is_empty());
    let errs = wcheck(&with_goblin(165, "llama")).expect_err("one pixel inside");
    assert!(has_rule(&errs, 5, &["g1", "a"]), "{}", text(&errs));
}

#[test]
fn n4_a_slime_counts_toward_the_wave_cap() {
    // The shipped W1 holds four (the slime, l2, g1, g2): a second slime (its own
    // pocket and patch) is a fifth.
    let mut v = wsidecar();
    v["pockets"].as_array_mut().unwrap().push(
        json!({ "id": "c", "x0": 392, "x1": 456, "max_stickers": 0, "max_slimes": 1,
                      "max_goblins": 0 }),
    );
    v["acid_patches"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "id": 2, "x0": 400, "x1": 440, "floor_y": 580, "linked_slime": "s2" }));
    v["roster"].as_array_mut().unwrap().push(json!({
        "id": "s2", "kind": "slime", "x": 420, "facing": -1, "home": 420,
        "linked_pool": "2", "beat": "W1", "delay_ticks": 240 }));
    let errs = wcheck(&v).expect_err("a fifth W1 entry");
    assert!(
        errs.iter()
            .any(|e| matches!(e, EncounterError::TooManyLiving { roster: 5, .. })),
        "{}",
        text(&errs)
    );
}

#[test]
fn n5_the_per_pocket_counts_still_fire_in_a_wave_room() {
    // Two goblins homed (by x) inside pocket a, which allows none.
    let mut v = minimal_waves();
    v["roster"][1] =
        json!({ "id": "g1", "kind": "goblin", "x": 200, "beat": "W1", "delay_ticks": 60 });
    v["roster"][2] =
        json!({ "id": "g3", "kind": "goblin", "x": 230, "beat": "W1", "delay_ticks": 120 });
    let errs = wcheck(&v).expect_err("goblins in the slime pocket");
    assert!(
        rule(&errs, 5)
            .iter()
            .any(|e| e.to_string().contains("2 goblins, at most 0")),
        "{}",
        text(&errs)
    );
}

#[test]
fn n6_without_waves_a_goblin_outside_every_pocket_is_still_refused() {
    let mut v = sidecar();
    v["roster"][1]["x"] = json!(1040); // g_a: dry floor right of every pocket
    let errs = errors(&v);
    assert!(has_rule(&errs, 5, &["g_a"]), "{}", text(&errs));
    assert!(
        rule(&errs, 5)
            .iter()
            .any(|e| e.to_string().contains("an enemy spawns in no pocket")),
        "{}",
        text(&errs)
    );
    assert!(wave_rule5(&errs).is_empty());
}

#[test]
fn n7_without_waves_a_goblin_in_a_sweep_adds_no_new_error() {
    // g_c homed inside pocket c and inside r2's blast sweep: the new clause does
    // not run for a non-wave sidecar.
    let mut v = sidecar();
    v["roster"][3]["x"] = json!(780);
    let r = check(&v);
    let errs = r.err().unwrap_or_default();
    assert!(wave_rule5(&errs).is_empty(), "{}", text(&errs));
}

/// A wave room must still declare its pockets for its hazard entities (the
/// lifted loop was the only clause removed).
#[test]
fn a_wave_room_with_a_slime_and_no_pocket_is_still_refused() {
    let mut v = minimal_waves();
    v["pockets"] = json!([]);
    let errs = wcheck(&v).expect_err("hazard entities need pockets");
    assert!(has_rule(&errs, 5, &[]), "{}", text(&errs));
}

/// Every committed sidecar other than the mastery arena takes the unchanged
/// (non-wave) branch of rule 5: it loads with no error, `waves` is false and its
/// hash is its pin. The existing negative tests above pass unedited.
#[test]
fn non_wave_sidecars_validate_identically() {
    let pins: [(&str, u64); 6] = [
        (BOILER_ENCOUNTER_PATH, crate::f01_boiler::ENCOUNTER_HASH),
        (HAZARDS_ENCOUNTER_PATH, crate::f01_hazards::ENCOUNTER_HASH),
        (
            "assets/levels/floor01/f01_hoarder.encounter.json",
            crate::f01_hoarder::ENCOUNTER_HASH,
        ),
        // RING-A/B wire: the ring rooms carry their module pins now.
        (
            crate::f01_ring::COURT_ENCOUNTER_PATH,
            crate::f01_ring::COURT_ENCOUNTER_HASH,
        ),
        (
            crate::f01_ring::SPLIT_ENCOUNTER_PATH,
            crate::f01_ring::SPLIT_ENCOUNTER_HASH,
        ),
        // DOZER-CORE: room 3.
        (
            crate::f01_dozer::ENCOUNTER_PATH,
            crate::f01_dozer::ENCOUNTER_HASH,
        ),
    ];
    let mut seen = 0;
    for entry in walk(&root().join("assets/levels")) {
        let rel = entry
            .strip_prefix(root())
            .unwrap()
            .to_string_lossy()
            .to_string();
        // CORRAL-WIRE: room 0's roster (two M0 goblins, three llamas) exceeds the
        // living cap of four, so it is the second wave room (PTM-AD6, one beat
        // per milestone); its pin is checked by `f01_corral`.
        if !rel.ends_with(".encounter.json")
            || rel == MASTERY_PATH
            || rel == crate::f01_corral::ENCOUNTER_PATH
        {
            continue;
        }
        let raw: Value = serde_json::from_str(&std::fs::read_to_string(&entry).unwrap()).unwrap();
        assert!(
            raw.get("waves").is_none_or(|w| w == &json!(false)),
            "{rel}: only the mastery arena and the corral set waves"
        );
        let hash = pins
            .iter()
            .find(|(p, _)| *p == rel)
            .unwrap_or_else(|| panic!("{rel}: add its pin to this test"))
            .1;
        let loaded = load(root(), &rel).unwrap_or_else(|e| panic!("{rel}: {}", text(&e)));
        assert_eq!(loaded.sidecar_hash, hash, "{rel}");
        seen += 1;
    }
    assert_eq!(seen, pins.len());
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out.sort();
    out
}

// ------------------------------------------- DOZER-CORE: section 9 rules

fn dozer_sidecar() -> Value {
    let t = std::fs::read_to_string(root().join(crate::f01_dozer::ENCOUNTER_PATH)).unwrap();
    serde_json::from_str(&t).unwrap()
}

fn dozer_ctx() -> Context {
    let l =
        load(root(), crate::f01_dozer::ENCOUNTER_PATH).unwrap_or_else(|e| panic!("{}", text(&e)));
    Context {
        room: l.room,
        classes: l.classes,
        ceiling_px: l.ceiling_px,
    }
}

/// The edited Dozer sidecar's errors of `rule`.
fn dozer_rule(edit: impl FnOnce(&mut Value), n: u8) -> Vec<EncounterError> {
    let mut v = dozer_sidecar();
    edit(&mut v);
    let errs = parse_sidecar(&v.to_string(), &dozer_ctx())
        .expect_err("the edited sidecar must be rejected");
    let hits: Vec<EncounterError> = errs
        .iter()
        .filter(|e| matches!(e, EncounterError::Dozer { rule, .. } if *rule == n))
        .cloned()
        .collect();
    assert!(!hits.is_empty(), "no rule {n} error: {}", text(&errs));
    hits
}

#[test]
fn the_dozer_room_passes_every_rule() {
    parse_sidecar(&dozer_sidecar().to_string(), &dozer_ctx())
        .unwrap_or_else(|e| panic!("{}", text(&e)));
}

#[test]
fn dozer_rule_1_one_machine_its_block_and_a_long_enough_lane() {
    dozer_rule(|v| v["dozer"] = Value::Null, 1);
    dozer_rule(
        |v| {
            let e = v["roster"][0].clone();
            let mut twin = e;
            twin["id"] = json!("dozer_b");
            v["roster"].as_array_mut().unwrap().push(twin);
        },
        1,
    );
    dozer_rule(|v| v["roster"][0]["x"] = json!(48), 1);
    dozer_rule(|v| v["dozer"]["dzq14_stop_x"] = json!(300), 1);
    dozer_rule(|v| v["dozer"]["dzq11_trigger_x"] = json!(1100), 1);
}

#[test]
fn dozer_rule_2_the_chase_lane_is_clear() {
    dozer_rule(
        |v| {
            v["statics"] = json!([{
                "id": "crate", "kind": "solid",
                "rect": { "x": 500, "y": 540, "w": 32, "h": 40 }
            }]);
        },
        2,
    );
    dozer_rule(|v| v["dozer"]["dzq22_lintel"]["x"] = json!(1010), 2);
}

/// The index of the stop frame prop and its falling mass in the sidecar.
fn frame_of(v: &Value) -> (usize, usize) {
    let p = v["props"]
        .as_array()
        .unwrap()
        .iter()
        .position(|p| p["kind"] == "weight_rig")
        .expect("room 3 has a stop frame");
    let h = v["hazards"]
        .as_array()
        .unwrap()
        .iter()
        .position(|h| h["class"] == "falling_mass")
        .expect("the frame owns a falling mass");
    (p, h)
}

#[test]
fn dozer_rule_3_the_frame_misses_the_parked_machine_and_hangs_over_it() {
    // The footprint inside the resting envelope (stop_x - 252 .. stop_x + 4).
    dozer_rule(
        |v| {
            let (_, h) = frame_of(v);
            v["hazards"][h]["rect"]["x"] = json!(760);
            v["hazards"][h]["telegraph_rect"]["x"] = json!(760);
        },
        3,
    );
    // A hang under the push box top + 8 (168).
    let errs = dozer_rule(
        |v| {
            let (p, _) = frame_of(v);
            v["props"][p]["hang_px"] = json!(160);
        },
        3,
    );
    assert!(text(&errs).contains("168"), "{}", text(&errs));
    // A frame without `hang_px` is reported by the rule itself (#287 review F5).
    let errs = dozer_rule(
        |v| {
            let (p, _) = frame_of(v);
            v["props"][p].as_object_mut().unwrap().remove("hang_px");
        },
        3,
    );
    assert!(text(&errs).contains("needs `hang_px`"), "{}", text(&errs));
}

#[test]
fn dozer_rule_5_the_useful_release_window_is_long_enough() {
    // The brace so far left that the waiting hero stands on the footprint's
    // edge: the machine reaches her almost as soon as a landing could hit.
    let errs = dozer_rule(
        |v| {
            let (p, _) = frame_of(v);
            v["props"][p]["target"]["x"] = json!(620);
        },
        5,
    );
    assert!(
        text(&errs).contains("under the minimum 45"),
        "{}",
        text(&errs)
    );
}

#[test]
fn dozer_rule_4_the_passage_and_the_lintel() {
    // A passage too low for a hero.
    dozer_rule(|v| v["dozer"]["dzq22_lintel"]["h"] = json!(180), 4);
    // A lintel that no longer covers the exit.
    dozer_rule(|v| v["dozer"]["dzq22_lintel"]["w"] = json!(200), 4);
}

#[test]
fn dozer_rule_6_no_boss_supplier_in_a_dozer_room() {
    dozer_rule(
        |v| {
            v["roster"].as_array_mut().unwrap().push(json!({
                "id": "hoard", "kind": "hoarder", "x": 600, "beat": "R1"
            }));
        },
        6,
    );
}
