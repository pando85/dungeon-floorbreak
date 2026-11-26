//! TROG-T05: the TROG-V rules V0 to V4 each reject a minimal broken sidecar, and the shipped
//! `trog_run` sidecar passes them all.

use super::super::*;
use serde_json::{Value, json};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

const PATH: &str = crate::f01_trog::RUN_ENCOUNTER_PATH;

fn text(errs: &[EncounterError]) -> String {
    errs.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

fn base() -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(PATH)).unwrap()).unwrap()
}

fn ctx() -> Context {
    let l = load(root(), PATH).unwrap_or_else(|e| panic!("{}", text(&e)));
    Context {
        room: l.room,
        classes: l.classes,
        ceiling_px: l.ceiling_px,
    }
}

/// The errors of the named TROG rule after `edit`.
fn rule(edit: impl FnOnce(&mut Value), name: &str) -> Vec<EncounterError> {
    let mut v = base();
    edit(&mut v);
    let errs = parse_sidecar(&v.to_string(), &ctx()).expect_err("the edited sidecar is rejected");
    let hits: Vec<EncounterError> = errs
        .iter()
        .filter(|e| matches!(e, EncounterError::Trog { rule, .. } if *rule == name))
        .cloned()
        .collect();
    assert!(
        !hits.is_empty(),
        "no TROG rule {name} error: {}",
        text(&errs)
    );
    hits
}

#[test]
fn the_shipped_run_sidecar_passes_every_rule() {
    parse_sidecar(&base().to_string(), &ctx()).unwrap_or_else(|e| panic!("{}", text(&e)));
}

#[test]
fn v0_a_troglodyte_needs_a_trigger_and_stands_on_the_floor() {
    rule(
        |v| {
            v["roster"][0].as_object_mut().unwrap().remove("trigger_x");
        },
        "V0",
    );
    rule(|v| v["roster"][0]["perch"] = json!(0), "V0");
}

#[test]
fn v1_the_leash_band_stays_inside_the_play_area() {
    // The Virtuoso's leash 128 on each side of 2250 leaves the room (play area ends at 2288).
    let hits = rule(|v| v["roster"][2]["x"] = json!(2250), "V1");
    assert!(hits[0].to_string().contains("vir_a"), "{}", hits[0]);
}

#[test]
fn v2_no_tall_static_solid_inside_a_leash_band() {
    // A 48 px solid inside the Pygmy's leash 448..832 (a 16 px one is allowed).
    let wall =
        json!({ "id": "wall", "kind": "solid", "rect": { "x": 700, "y": 532, "w": 32, "h": 48 } });
    rule(|v| v["statics"].as_array_mut().unwrap().push(wall), "V2");
    let kerb =
        json!({ "id": "kerb", "kind": "solid", "rect": { "x": 700, "y": 564, "w": 32, "h": 16 } });
    let mut v = base();
    v["statics"].as_array_mut().unwrap().push(kerb);
    let result = parse_sidecar(&v.to_string(), &ctx());
    let errs = result.err().unwrap_or_default();
    assert!(
        !errs
            .iter()
            .any(|e| matches!(e, EncounterError::Trog { rule: "V2", .. })),
        "a 16 px static is allowed: {}",
        text(&errs)
    );
}

#[test]
fn v3_the_protected_spawn_stays_out_of_reach() {
    // The Pygmy at 300 (leash 108..492) plus its leap and strike (192) reaches the spawn.
    rule(
        |v| {
            v["roster"][0]["x"] = json!(300);
            v["roster"][0]["trigger_x"] = json!(250);
        },
        "V3",
    );
}

#[test]
fn v4_no_partition_prop_inside_a_leash_band() {
    let bench = json!({
        "id": "bench", "kind": "partition",
        "target": { "x": 700, "y": 532, "w": 16, "h": 48 },
        "solid_intact": true,
        "remnant": { "x": 700, "y": 564, "w": 16, "h": 16 },
        "partition": { "break_hits": 3, "shove_min_impulse_px_s": 160 }
    });
    rule(|v| v["props"].as_array_mut().unwrap().push(bench), "V4");
}

#[test]
fn an_unknown_troglodyte_kind_is_refused_with_its_name() {
    let mut v = base();
    v["roster"][0]["kind"] = json!("trog_giant");
    let errs = parse_sidecar(&v.to_string(), &ctx()).expect_err("refused");
    assert!(text(&errs).contains("trog_giant"), "{}", text(&errs));
}
