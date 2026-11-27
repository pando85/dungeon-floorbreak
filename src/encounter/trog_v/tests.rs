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

// ---------------------------------------------------------------- TROG-B

const STRENGTH: &str = crate::f01_trog::STRENGTH_ENCOUNTER_PATH;

fn strength() -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(STRENGTH)).unwrap()).unwrap()
}

fn strength_ctx() -> Context {
    let l = load(root(), STRENGTH).unwrap_or_else(|e| panic!("{}", text(&e)));
    Context {
        room: l.room,
        classes: l.classes,
        ceiling_px: l.ceiling_px,
    }
}

/// TROG-B's first action (work order, plan 5.4): the 80 px `bench_stack` partition goes through
/// the sidecar validator. **It is refused**: a `solid_intact` prop is at most
/// `MAX_LANE_OBSTACLE_PX` (48) px high. That is the plan's stop condition, and the declared
/// fallback is taken (TROG-AD12, TROG-AD43, TROG-AD46): the shipped room carries an 80 px static
/// bench and no partition, and the prop rule is not widened. This test is the recorded evidence;
/// if the validator's cap ever moves for a partition, it fails and the 80 px bench stack
/// (TROG-Q14) is the follow-up.
#[test]
fn the_80_px_bench_stack_partition_is_refused_so_the_static_fallback_is_taken() {
    let mut v = strength();
    v["props"] = json!([{
        "id": "bench_stack",
        "kind": "partition",
        "target": { "x": 640, "y": 500, "w": 32, "h": 80 },
        "solid_intact": true,
        "remnant": { "x": 640, "y": 564, "w": 32, "h": 16 },
        "partition": { "break_hits": 3, "shove_min_impulse_px_s": 160 }
    }]);
    v["statics"]
        .as_array_mut()
        .unwrap()
        .retain(|st| st["id"] != "bench");
    let errs = parse_sidecar(&v.to_string(), &strength_ctx()).expect_err("an 80 px solid prop");
    assert!(
        errs.iter()
            .any(|e| matches!(e, EncounterError::ObstacleTooTall { prop, px: 80 } if prop == "bench_stack")),
        "{}",
        text(&errs)
    );
    // The same stack at the cap (48 px, the corral's pen partition) is accepted.
    let mut ok = strength();
    ok["props"] = json!([{
        "id": "bench_stack",
        "kind": "partition",
        "target": { "x": 640, "y": 532, "w": 32, "h": 48 },
        "solid_intact": true,
        "remnant": { "x": 640, "y": 564, "w": 32, "h": 16 },
        "partition": { "break_hits": 3, "shove_min_impulse_px_s": 160 }
    }]);
    ok["statics"]
        .as_array_mut()
        .unwrap()
        .retain(|st| st["id"] != "bench");
    parse_sidecar(&ok.to_string(), &strength_ctx())
        .unwrap_or_else(|e| panic!("a 48 px partition passes: {}", text(&e)));
}

#[test]
fn the_shipped_strength_sidecar_passes_every_rule() {
    parse_sidecar(&strength().to_string(), &strength_ctx())
        .unwrap_or_else(|e| panic!("{}", text(&e)));
}

/// The fallback's geometry (plan 5.4, TROG-AD43, TROG-AD46): the bench is an 80 px static solid, no prop
/// stands in the room, and the Basher's leash (spawn 800, half-width 96) starts at 704, so it can
/// never stand at the bench's face (672).
#[test]
fn the_static_fallback_bench_and_the_basher_leash() {
    let loaded = load(root(), STRENGTH).unwrap_or_else(|e| panic!("{}", text(&e)));
    let enc = &loaded.encounter;
    assert!(enc.props.is_empty(), "the fallback has no partition prop");
    let bench = enc.statics.iter().find(|s| s.id == "bench").expect("bench");
    assert_eq!(bench.kind, StaticKind::Solid);
    assert_eq!(
        (bench.rect.x, bench.rect.y, bench.rect.w, bench.rect.h),
        (640, 500, 32, 80)
    );
    // `trog_eval`'s surfaces are the sidecar's (TROG-AD47): they cannot drift.
    assert_eq!(bench.rect.h, crate::trog_eval::BENCH_RISE_PX);
    let shelf = enc.statics.iter().find(|s| s.id == "shelf").expect("shelf");
    assert_eq!(shelf.rect.h, crate::trog_eval::SHELF_RISE_PX);
    let basher = enc
        .roster
        .iter()
        .find(|e| e.kind.trog() == Some(crate::trog::Species::Basher))
        .expect("basher");
    let leash = i32::from(crate::trog::TrogTuning::active().basher.leash_px);
    assert_eq!(basher.x - leash, 704);
    assert!(basher.x - leash > bench.rect.right() + 24);
}

/// V2 holds for both statics: neither lies inside a leash band (the Virtuoso's is 368..624, the
/// Basher's 704..896), and a 48 px solid dropped inside the Basher's band is refused.
#[test]
fn v2_the_shelf_and_the_bench_stay_outside_both_leash_bands() {
    let mut v = strength();
    v["statics"].as_array_mut().unwrap().push(json!(
        { "id": "wall", "kind": "solid", "rect": { "x": 800, "y": 532, "w": 32, "h": 48 } }
    ));
    let errs = parse_sidecar(&v.to_string(), &strength_ctx()).expect_err("rejected");
    assert!(
        errs.iter()
            .any(|e| matches!(e, EncounterError::Trog { rule: "V2", .. })),
        "{}",
        text(&errs)
    );
}

/// TROG-AD46: a sidecar static of kind `solid` may be 80 px high (Carl's class limit,
/// `MAX_STATIC_SOLID_PX`), not 81; a solid prop and a remnant stay at 48.
#[test]
fn a_static_solid_is_at_most_80_px() {
    let with_h = |h: i32| {
        let mut v = strength();
        let bench = v["statics"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|s| s["id"] == "bench")
            .unwrap();
        bench["rect"]["y"] = json!(580 - h);
        bench["rect"]["h"] = json!(h);
        parse_sidecar(&v.to_string(), &strength_ctx())
    };
    with_h(80).unwrap_or_else(|e| panic!("80 px passes: {}", text(&e)));
    let errs = with_h(81).expect_err("81 px is refused");
    assert!(
        errs.iter()
            .any(|e| matches!(e, EncounterError::StaticInvalid { id, reason }
            if id == "bench" && reason.contains("at most 80 px"))),
        "{}",
        text(&errs)
    );
    assert_eq!(MAX_STATIC_SOLID_PX, 80);
    assert_eq!(MAX_LANE_OBSTACLE_PX, 48, "a prop and a remnant stay at 48");
}
