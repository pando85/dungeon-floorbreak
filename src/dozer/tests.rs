use super::*;
use crate::prototype::Material;

const FLOOR: f32 = 92.0;

fn site() -> DozerSite {
    DozerSite {
        trigger_x: 256,
        idle_trigger_ticks: 300,
        emerge_front_x: 32,
        stop_x: 1000,
        lintel: IRect::new(1040, 352, 304, 116),
    }
}

/// A machine in the chase with its wheel front at `front`.
fn chasing(front: f32) -> Dozer {
    let mut d = Dozer::new(site(), 1264.0, FLOOR);
    d.o = front - d.tuning.wheel_front();
    d.state = DozerState::CommittedChase;
    d.band = Some(Band::Cruise);
    d.speed = d.profile.cruise;
    d
}

fn view(id: HeroId, x: f32, bot: bool) -> HeroView {
    HeroView {
        id,
        present: true,
        x,
        foot_y: FLOOR,
        grounded: true,
        hurt: body_rect(x, FLOOR),
        vulnerable: true,
        bot,
    }
}

fn absent(id: HeroId) -> HeroView {
    HeroView {
        present: false,
        ..view(id, 2000.0, false)
    }
}

fn ctx<'a>(tick: u64, heroes: [HeroView; 2], solids: &'a [Block], solo: bool) -> DozerCtx<'a> {
    DozerCtx {
        tick,
        heroes,
        solids,
        solo,
        view_x: (0.0, 640.0),
    }
}

fn wall(left: f32, width: f32) -> Block {
    Block {
        rect: Rect {
            left,
            bottom: FLOOR,
            width,
            height: 200.0,
        },
        material: Material::Stone,
        intact: true,
    }
}

fn field_of(err: DozerError) -> &'static str {
    match err {
        DozerError::Field { field, .. } => field,
        other => panic!("not a field error: {other:?}"),
    }
}

// ---------------------------------------------------------------- the data --

#[test]
fn the_embedded_data_matches_its_pin_and_validates() {
    let t = DozerTuning::pinned().expect("pinned");
    assert_eq!(&t, DozerTuning::active());
    let drifted = format!("{TUNING_JSON} ");
    assert!(matches!(
        DozerTuning::from_json_pinned(&drifted, TUNING_HASH),
        Err(DozerError::HashMismatch { .. })
    ));
}

#[test]
fn k5_rejects_a_speed_above_the_heroes_run_speed_and_names_the_field() {
    let ceiling = k5_speed_ceiling();
    assert!(ceiling >= 185.0, "K5 ceiling {ceiling}");
    let mut t = DozerTuning::active().clone();
    t.revenge.dzq07_catch_up_px_s = ceiling as u16 + 1;
    assert_eq!(
        field_of(t.validate().unwrap_err()),
        "revenge.dzq07_catch_up_px_s"
    );
    let mut t = DozerTuning::active().clone();
    t.revenge.dzq06_cruise_px_s = REVENGE_CRUISE_MAX_PX_S + 1;
    assert_eq!(
        field_of(t.validate().unwrap_err()),
        "revenge.dzq06_cruise_px_s"
    );
}

#[test]
fn a_charge_block_is_refused_in_room_3() {
    let mut t = DozerTuning::active().clone();
    t.revenge.dzq10_charge = Some(serde_json::json!({ "speed": 300 }));
    assert_eq!(field_of(t.validate().unwrap_err()), "revenge.dzq10_charge");
}

#[test]
fn the_hull_is_taller_than_the_highest_jump_by_the_margin() {
    let hull_top = DozerTuning::active().dzq02_push_box.y1 as f32;
    // Donut's boosted launch (DPQ-14) is the highest jump of the slice.
    let apex = crate::agility::discrete_apex_px(547.0);
    assert!(
        hull_top >= apex + HULL_APEX_MARGIN_PX,
        "hull {hull_top} vs apex {apex}"
    );
}

// ---------------------------------------------------------------- the band --

#[test]
fn the_band_edges_are_cruise() {
    let p = DozerTuning::active().revenge();
    assert_eq!(p.band(Some(96.0)), Band::Cruise);
    assert_eq!(p.band(Some(260.0)), Band::Cruise);
    assert_eq!(p.band(Some(95.5)), Band::Mercy);
    assert_eq!(p.band(Some(260.5)), Band::CatchUp);
    assert_eq!(p.band(None), Band::Cruise);
    assert_eq!(p.approach(160.0, 185.0), 164.0);
    assert_eq!(p.approach(152.0, 150.0), 150.0);
}

#[test]
fn the_release_window_of_the_room_3_frame_is_at_least_the_minimum() {
    let t = DozerTuning::active();
    let w = release_window(
        &t.revenge(),
        &StopFrame {
            footprint_x0: 640.0,
            footprint_x1: 720.0,
            land_delay_ticks: 37,
            emerge_front_x: 32.0,
        },
        832.0,
        u32::from(t.dzq23_melee_recovery_ticks),
    );
    assert!(
        w.ticks >= u32::from(t.dzq23_min_release_window_ticks),
        "{w:?}"
    );
}

/// DR-B: the hit set of the window holds the useful set (the useful set adds
/// the hero's recovery), and in room 3 the hit window opens with the wheel
/// front near the section 6 estimate (about 537 at the release).
#[test]
fn the_hit_window_holds_the_useful_window() {
    let t = DozerTuning::active();
    let w = release_window(
        &t.revenge(),
        &StopFrame {
            footprint_x0: 640.0,
            footprint_x1: 720.0,
            land_delay_ticks: 37,
            emerge_front_x: 32.0,
        },
        832.0,
        u32::from(t.dzq23_melee_recovery_ticks),
    );
    assert!(w.hit_ticks >= w.ticks, "{w:?}");
    assert_eq!(w.hit_first_tick, w.first_tick, "{w:?}");
    assert!(w.hit_last_tick >= w.last_tick, "{w:?}");
    assert!((500.0..580.0).contains(&w.first_front), "{w:?}");
}

/// DR-B (DOZR-04): the stun carries the release's credit, once, only from the
/// chase.
#[test]
fn a_falling_mass_stuns_the_chase_once_with_its_credit() {
    let mut d = chasing(600.0);
    let mut out = DozerOut::default();
    assert!(d.apply_falling_mass(Some(1), &mut out));
    assert_eq!(
        out.events,
        vec![DozerEvent::Stunned {
            front_x: 600,
            by: Some(1)
        }]
    );
    assert!(!d.apply_falling_mass(Some(0), &mut out), "stunned once");
    assert_eq!(out.events.len(), 1);
}

/// DR-B (DZ-AD7): a footprint over the hull behind the front collider is a
/// glance; one ahead of the machine, or a parked machine, is nothing.
#[test]
fn a_landing_on_the_rear_hull_is_a_glance() {
    let footprint = |x0: f32| Rect {
        left: x0,
        bottom: 0.0,
        width: 80.0,
        height: 96.0,
    };
    let d = chasing(900.0);
    let floor = d.floor_y;
    let on = |x0: f32| Rect {
        bottom: floor,
        ..footprint(x0)
    };
    assert_eq!(
        d.glance(on(640.0)),
        Some(DozerEvent::Glance { front_x: 900 }),
        "the rear deck"
    );
    assert_eq!(d.glance(on(1000.0)), None, "ahead of the machine");
    let mut parked = d.clone();
    parked.state = DozerState::Escaped(EscapePhase::Idle);
    assert_eq!(parked.glance(on(640.0)), None, "the parked machine");
}

// ---------------------------------------------------------------- the step --

#[test]
fn the_chase_brakes_and_rests_with_the_wheel_front_at_stop_x() {
    let mut d = chasing(900.0);
    let heroes = [absent(HeroId::Carl), absent(HeroId::Donut)];
    let mut braked = false;
    for tick in 0..400 {
        let mut out = DozerOut::default();
        d.step(&ctx(tick, heroes, &[], false), &mut out);
        braked |= out
            .events
            .iter()
            .any(|e| matches!(e, DozerEvent::BrakeStarted { .. }));
    }
    assert!(braked);
    assert_eq!(d.state, DozerState::Escaped(EscapePhase::Idle));
    assert_eq!(d.wheel_front(), 1000.0);
    assert!(!d.wheel_lethal());
}

#[test]
fn a_push_out_into_a_solid_holds_the_machine_instead_of_crushing() {
    let mut d = chasing(400.0);
    let o = d.o;
    // Carl just inside the hull's rear, a wall where the push-out would put him.
    let x = d.push_box().left + 4.0;
    let solids = [wall(d.push_box().left - 40.0, 30.0)];
    let heroes = [view(HeroId::Carl, x, false), absent(HeroId::Donut)];
    let mut out = DozerOut::default();
    d.step(&ctx(1, heroes, &solids, false), &mut out);
    assert_eq!(d.o, o, "the machine moved");
    assert_eq!(d.stats.pinned_holds, 1);
    assert!(out.shifts.is_empty());
    assert!(
        out.events
            .contains(&DozerEvent::PinnedHold { hero: HeroId::Carl })
    );
}

#[test]
fn the_nav_rows_lie_on_the_grid() {
    let d = chasing(500.0);
    let grid = i32::from(d.tuning.dzq16_nav_grid_px);
    let rows = d.nav_rows(704);
    assert!(!rows.is_empty());
    for r in rows {
        let IRect { x, y, w, h } = r.rect;
        assert!(
            [x, y, w, h].iter().all(|v| v.rem_euclid(grid) == 0),
            "{r:?}"
        );
        assert!(r.hurts_heroes && !r.hurts_enemies);
    }
}

fn rescue_of(out: &DozerOut) -> (f32, RescueSpot, RescueCause) {
    let spot = out
        .events
        .iter()
        .find_map(|e| match *e {
            DozerEvent::AiRescue { spot, cause, .. } => Some((spot, cause)),
            _ => None,
        })
        .expect("a rescue");
    let (_, x, _) = out.place[0];
    (x, spot.0, spot.1)
}

#[test]
fn a_bot_at_the_wheel_goes_to_the_lane_ahead_of_its_leader_in_view() {
    let mut d = chasing(300.0);
    let heroes = [
        view(HeroId::Carl, 600.0, false),
        view(HeroId::Donut, 330.0, true),
    ];
    let mut out = DozerOut::default();
    // The camera centred on the leader.
    let c = DozerCtx {
        view_x: (280.0, 920.0),
        ..ctx(1, heroes, &[], true)
    };
    d.step(&c, &mut out);
    let (x, spot, cause) = rescue_of(&out);
    assert_eq!((spot, cause), (RescueSpot::Lane, RescueCause::Near));
    assert_eq!(x, 648.0);
    assert!(out.blows.is_empty(), "the rescued bot was hit");
    // The same spot outside the view margin falls back to the tunnel.
    let mut d = chasing(300.0);
    let mut out = DozerOut::default();
    d.step(&ctx(1, heroes, &[], true), &mut out);
    assert_eq!(rescue_of(&out).1, RescueSpot::Tunnel);
}

#[test]
fn a_bot_whose_leader_waits_in_the_tunnel_goes_past_stop_x() {
    let mut d = chasing(700.0);
    let heroes = [
        view(HeroId::Carl, 1100.0, false),
        view(HeroId::Donut, 722.0, true),
    ];
    let mut out = DozerOut::default();
    d.step(&ctx(1, heroes, &[], true), &mut out);
    let (x, spot, _) = rescue_of(&out);
    assert_eq!(spot, RescueSpot::Tunnel);
    assert_eq!(x, 1148.0);
    assert!((1048.0..=1264.0 - 16.0).contains(&x));
}

#[test]
fn a_second_rescue_within_the_spacing_never_returns_to_the_lane() {
    let mut d = chasing(300.0);
    let heroes = [
        view(HeroId::Carl, 600.0, false),
        view(HeroId::Donut, 330.0, true),
    ];
    let mut out = DozerOut::default();
    d.step(&ctx(1, heroes, &[], true), &mut out);
    let front = d.wheel_front();
    let heroes = [
        view(HeroId::Carl, 600.0, false),
        view(HeroId::Donut, front + 18.0, true),
    ];
    let mut out = DozerOut::default();
    d.step(&ctx(30, heroes, &[], true), &mut out);
    assert_eq!(rescue_of(&out).1, RescueSpot::Tunnel);
    assert_eq!(d.stats.rescues[HeroId::Donut.index()], 2);
}

#[test]
fn no_rescue_in_two_player_or_for_a_body_past_stop_x() {
    let mut d = chasing(300.0);
    let heroes = [
        view(HeroId::Carl, 600.0, false),
        view(HeroId::Donut, 330.0, true),
    ];
    let mut out = DozerOut::default();
    d.step(&ctx(1, heroes, &[], false), &mut out);
    assert!(out.place.is_empty());
    let mut d = chasing(990.0);
    let heroes = [
        view(HeroId::Carl, 1100.0, false),
        view(HeroId::Donut, 1013.0, true),
    ];
    let mut out = DozerOut::default();
    d.step(&ctx(1, heroes, &[], true), &mut out);
    assert!(out.place.is_empty());
}
