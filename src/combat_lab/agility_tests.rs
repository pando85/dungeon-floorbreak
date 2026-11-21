//! DONUT-MOBILITY-01 (DM-A) proofs on the combat lab rooms: Donut's HUD
//! clearance (DM-Q26, spec test 8) and the nav diff between the integrated and
//! the agile preset (spec test 5). The checks live with the other spaces in
//! `agility_geometry_tests`; this file only supplies the lab rooms, because the
//! story and demo code may not mention the lab (see `entry::tests`).

use super::{EntryPoint, LabCatalog, LabConfig, PlayerMode};
use crate::agility_geometry_tests::{check_hud, check_nav_diff, root};
use crate::env_kit::to_world_y;

const ROOMS: [(&str, &str); 5] = [
    ("R00", "v1"),
    ("R01", "v1"),
    ("R01", "v2"),
    ("R01", "v3"),
    ("R02", "v1"),
];

/// `(label, solids, walking line world y, max x)` of every lab play scenario.
fn rooms() -> Vec<(String, Vec<crate::prototype::Block>, f32, f32)> {
    let catalog = LabCatalog::load(&root()).unwrap();
    ROOMS
        .iter()
        .map(|&(room, variant)| {
            let mut cfg = LabConfig::new(room, PlayerMode::CoopLocal);
            cfg.variant = variant.to_owned();
            let run = catalog.launch(EntryPoint::CombatLab, &cfg).unwrap();
            (
                format!("lab {room} {variant}"),
                run.scene.world.solids.clone(),
                to_world_y(run.kit.height, run.kit.base_walk_line_y),
                run.kit.width as f32 - 12.0,
            )
        })
        .collect()
}

#[test]
fn donut_stays_under_the_hud_band_in_every_lab_room() {
    let report: Vec<String> = rooms()
        .into_iter()
        .map(|(label, blocks, walk, max_x)| check_hud(&label, &blocks, walk, max_x))
        .collect();
    println!("HUD {}", report.join(" | "));
}

#[test]
fn the_agile_preset_changes_no_carl_edge_in_the_lab_rooms() {
    for (label, blocks, walk, _) in rooms() {
        println!("{}", check_nav_diff(&label, blocks, walk));
    }
}
