//! HAZ-B fixtures (tells, contact, classification): the slime's lunge in the
//! evadable allow-list (DM-Q17, HAZ-Q15) and the sources that stay unlisted
//! (DM-Q18), the hazard feed rows, the companion's `boss` row, and the
//! presentation timing (`docs/01-design/enemy-gameplay-slime-rot.md` section 18,
//! "HAZ-B fixtures").
use super::*;
use crate::agility::EvadeSource;
use crate::combat::CombatEvent;
use crate::combat_scene::slime::SlimeEnemy;
use crate::party::HeroId;
use crate::slime::slime_source;

const SLIME_A: SlimeAt = SlimeAt {
    x: 400,
    facing: 1,
    home: 400,
};

fn slime_of(rig: &Rig, i: usize) -> &SlimeEnemy {
    rig.combat().slot_enemies()[i]
        .as_any()
        .expect("a slime")
        .downcast_ref::<SlimeEnemy>()
        .expect("a SlimeEnemy")
}

fn attack_tick(rig: &Rig) -> Option<u16> {
    slime_of(rig, 0).slime().attack_tick()
}

fn evades(rig: &Rig) -> Vec<(u64, u32, EvadeSource)> {
    rig.events()
        .iter()
        .filter_map(|e| match *e {
            CombatEvent::HeroEvaded {
                tick,
                hero: HeroId::Donut,
                source,
                kind,
            } => Some((tick, source, kind)),
            _ => None,
        })
        .collect()
}

/// Step with idle inputs until the slime's tell starts; returns that tick.
fn until_tell(rig: &mut Rig, limit: u64) -> u64 {
    for _ in 0..limit {
        rig.step(RawInput::NONE, RawInput::NONE);
        if attack_tick(rig) == Some(0) {
            return rig.tick();
        }
    }
    panic!("the tell never started");
}

/// A live party (Carl and Donut) with the slime A at 400, the heroes placed by
/// the caller. Donut's reflex is armed.
fn armed_party(carl_x: f32, donut_x: f32) -> Rig {
    let mut rig = Layout {
        slimes: vec![SLIME_A],
        carl_x: carl_x as i32,
        ..Layout::default()
    }
    .sim(Some(true));
    rig.place(HeroId::Carl, carl_x);
    rig.place(HeroId::Donut, donut_x);
    rig.combat_mut().arm_reflex_for_test();
    rig
}

#[test]
fn haz_slime_lunge_evaded() {
    // Donut is nearer, so the lunge locks on her; Carl stands behind her on the
    // same impact tick. Her armed reflex evades (one `HeroEvaded`, no
    // `HeroHurt`); Carl is hit by the same lunge.
    let mut rig = armed_party(438.0, 424.0);
    let tell = until_tell(&mut rig, 20);
    for _ in 0..60 {
        rig.step(RawInput::NONE, RawInput::NONE);
    }
    let impact = tell + 32;
    let id = crate::combat_scene::slime::target_id(0);
    assert_eq!(
        evades(&rig),
        vec![(impact, id, EvadeSource::SlimeLunge)],
        "{:?}",
        rig.events()
    );
    // No hurt of Donut on any impact tick (33 to 38): the lunge is evaded and the
    // evading hero is exempt from the body for the rest of this lunge.
    let donut = rig.hurts(HeroId::Donut);
    assert!(donut.iter().all(|h| h.0 > impact + 5), "{donut:?}");
    let carl = rig.hurts(HeroId::Carl);
    assert_eq!(carl.len(), 1, "{carl:?}");
    assert_eq!(carl[0].0, impact);
    assert_eq!(carl[0].1, slime_source(0));
    // The evade started her wait again.
    assert!(!rig.combat().reflex().armed());
}

#[test]
fn haz_slime_evade_no_body_burn() {
    // An armed Donut close to the slime's pivot: inside its lunge box and beside
    // its gathered body. She evades the lunge; she takes no body-contact
    // `HeroHurt` on any impact tick of that lunge (33 to 38 from the tell).
    //
    // HAZB-D1 (a deviation from the sheet's `d` 12): her 20 px hurtbox reaches the
    // gathered body (to +4 px) at `d` 13 and nearer, so from there the body burns
    // her on the first tick, before any lunge (pinned below). `d` 14 is the
    // nearest column where the lunge is the first thing that reaches her.
    let mut near = armed_party(112.0, 413.0);
    near.step(RawInput::NONE, RawInput::NONE);
    assert_eq!(near.hurts(HeroId::Donut).len(), 1, "d 13 is burned at once");
    let mut rig = armed_party(112.0, 414.0);
    let tell = until_tell(&mut rig, 20);
    for _ in 0..70 {
        rig.step(RawInput::NONE, RawInput::NONE);
    }
    let impact = tell + 32;
    assert_eq!(evades(&rig).len(), 1, "{:?}", rig.events());
    assert_eq!(evades(&rig)[0].0, impact);
    for (tick, source, _) in rig.hurts(HeroId::Donut) {
        assert_eq!(source, slime_source(0));
        assert!(
            !(impact..=impact + 5).contains(&tick),
            "a body burn on impact tick {tick}"
        );
    }
}

/// DM-Q18: an armed Donut takes this hurt: no `HeroEvaded`, no arm spent, and
/// the hurt restarts her wait (DM-Q13).
fn assert_not_evadable(rig: &Rig, source: u32) {
    assert!(evades(rig).is_empty(), "{:?}", rig.events());
    let hurts = rig.hurts(HeroId::Donut);
    assert_eq!(hurts.len(), 1, "{hurts:?}");
    assert_eq!(hurts[0].1, source, "{hurts:?}");
    let reflex = rig.combat().reflex();
    assert!(reflex.evaded().is_none(), "no evade was spent");
    assert!(!reflex.armed(), "the hurt reset the wait");
    assert_eq!(reflex.last_reset_tick(), hurts[0].0);
}

#[test]
fn haz_acid_not_evadable() {
    let mut rig = Layout {
        acid: vec![(700, 764, None)],
        carl_x: 600,
        ..Layout::default()
    }
    .sim(Some(true));
    rig.place(HeroId::Donut, 680.0);
    rig.combat_mut().arm_reflex_for_test();
    for _ in 0..60 {
        rig.step(RawInput::NONE, walk(1.0));
    }
    assert_not_evadable(&rig, crate::slime::acid_source(1));
}

#[test]
fn haz_slime_body_not_evadable() {
    // The gathered body at the first tick (the lunge is 32 ticks away): the
    // body contact is a hazard, never the reflex's (DM-Q18).
    let mut rig = armed_party(112.0, 413.0);
    for _ in 0..3 {
        rig.step(RawInput::NONE, RawInput::NONE);
    }
    assert_not_evadable(&rig, slime_source(0));
}

#[test]
fn haz_rot_not_evadable() {
    let mut rig = Layout {
        stickers: vec![Layout::under_slab(600, 600, 600)],
        carl_x: 112,
        ..Layout::default()
    }
    .sim(Some(true));
    rig.place(HeroId::Donut, 590.0);
    rig.combat_mut().arm_reflex_for_test();
    for _ in 0..50 {
        rig.step(RawInput::NONE, RawInput::NONE);
    }
    assert_not_evadable(&rig, crate::rot_sticker::rot_source(0));
}

fn missile_rig(height: i32, extra: Vec<IRect>) -> Rig {
    let mut solids = vec![Layout::pillar()];
    solids.extend(extra);
    let mut rig = Layout {
        stickers: vec![Layout::on_pillar(height)],
        solids,
        carl_x: 800,
        ..Layout::default()
    }
    .sim(Some(true));
    rig.place(HeroId::Donut, 880.0);
    rig.scene_mut().world.donut_body.facing = 1;
    rig
}

/// Cast once (press on step 0) and fly for 60 ticks. Returns the lowest and the
/// highest the bolt's box reached `(bottom, top)` (world px, `None` if it never
/// flew), whether the bolt's box ever overlapped the sticker's shell, and the
/// nearest the bolt box came to the dome.
struct Flight {
    box_y: Option<(f32, f32)>,
    touched_shell: bool,
    nearest_to_dome: f32,
}

fn fly(rig: &mut Rig) -> Flight {
    let t = crate::rot_sticker::active();
    let mut out = Flight {
        box_y: None,
        touched_shell: false,
        nearest_to_dome: f32::MAX,
    };
    for k in 0..60 {
        rig.step(
            RawInput::NONE,
            RawInput {
                attack: k == 0,
                ..RawInput::NONE
            },
        );
        let sticker = rig.combat().env().unwrap().sticker(0).unwrap();
        for bolt in rig.combat().donut_bolts() {
            let r = bolt.rect();
            out.box_y = Some(match out.box_y {
                Some((b, t)) => (b.min(r.bottom), t.max(r.bottom + r.height)),
                None => (r.bottom, r.bottom + r.height),
            });
            if sticker.hurtbox(t).is_some_and(|s| s.overlaps(r)) {
                out.touched_shell = true;
            }
            let (dx, dy) = sticker.dome(t);
            out.nearest_to_dome = out
                .nearest_to_dome
                .min(crate::rot_sticker::dist2_to_rect(r, dx, dy).sqrt());
        }
    }
    out
}

fn dome_y(rig: &Rig) -> f32 {
    let t = crate::rot_sticker::active();
    rig.combat().env().unwrap().sticker(0).unwrap().dome(t).1
}

fn projectile_provokes(rig: &Rig) -> usize {
    rig.provokes()
        .iter()
        .filter(|e| {
            matches!(
                e,
                CombatEvent::RotProvoked {
                    cause: crate::rot_sticker::RotCause::Projectile,
                    player: Some(1),
                    ..
                }
            )
        })
        .count()
}

#[test]
fn haz_rot_missile_provoke() {
    // Donut, controlled at 880, casts +x at a fixed wall sticker on K2's left
    // face with its dome at height 26: `RotProvoked { cause: Projectile, player:
    // Donut's }`, the bolt is consumed and she is untouched.
    let mut rig = missile_rig(26, vec![]);
    let _ = fly(&mut rig);
    assert_eq!(projectile_provokes(&rig), 1, "{:?}", rig.provokes());
    assert_eq!(rig.provokes().len(), 1);
    assert!(
        rig.combat().donut_bolts().is_empty(),
        "the bolt is consumed"
    );
    assert!(rig.hurts(HeroId::Donut).is_empty());
}

#[test]
fn haz_rot_missile_miss() {
    // A grounded bolt more than 32 px below the dome does not provoke it: R1 at
    // `H` 68 (40 px) and R2 at `H` 84 (56 px).
    for height in [68, 84] {
        let mut rig = missile_rig(height, vec![]);
        let flight = fly(&mut rig);
        let (_, top) = flight.box_y.expect("the bolt flew");
        let gap = dome_y(&rig) - top;
        assert!(gap > 32.0, "H {height}: {gap} px below the dome");
        assert_eq!(rig.provokes().len(), 0, "H {height}: {:?}", rig.provokes());
        assert!(!flight.touched_shell);
    }
}

#[test]
fn haz_rot_missile_near_miss() {
    // Rule B (HAZ-Q29): a bolt whose box passes within 32 px of the dome with
    // line of sight, without touching the shell, provokes the sticker and is
    // consumed; with the line of sight blocked by a solid it does not.
    let t = crate::rot_sticker::active();
    let mut rig = missile_rig(48, vec![]);
    let shell = rig
        .combat()
        .env()
        .unwrap()
        .sticker(0)
        .unwrap()
        .hurtbox(t)
        .unwrap();
    let dome = dome_y(&rig);
    let flight = fly(&mut rig);
    let (_, top) = flight.box_y.expect("the bolt flew");
    // The bolt's lane passes under the shell (no touch) and within 32 px of the
    // dome (a 26 px vertical gap, closing to the dome's column).
    assert!(shell.bottom > top, "the lane misses the shell");
    assert!(!flight.touched_shell);
    assert!(dome - top <= 32.0, "{} px below the dome", dome - top);
    assert_eq!(projectile_provokes(&rig), 1, "{:?}", rig.provokes());
    assert!(
        rig.combat().donut_bolts().is_empty(),
        "the bolt is consumed"
    );
    // A 4 px ledge between the bolt's lane and the dome (layout px, y down).
    let ledge = IRect::new(900, FLOOR - 32, 60, 4);
    let mut blocked = missile_rig(48, vec![ledge]);
    let flight = fly(&mut blocked);
    assert!(flight.box_y.is_some_and(|(_, t)| dome - t <= 32.0));
    assert_eq!(blocked.provokes().len(), 0, "{:?}", blocked.provokes());
}

#[test]
fn haz_rot_selective_radius() {
    // HAZ-05: goblins whose boxes are 40 and 60 px from the dome centre: only
    // the inner one is hurt and launched, and the provoke credits the initiator
    // (Carl, whose collider set it off).
    let mut rig = Layout {
        stickers: vec![Layout::on_pillar(32)],
        solids: vec![Layout::pillar()],
        // The pillar's left face is x 960 and the dome 6 px in front of it
        // (954); a goblin's body is 24 px wide.
        goblins: vec![902, 882],
        carl_x: 925,
        ..Layout::default()
    }
    .sim(None);
    let t = crate::rot_sticker::active();
    let dome = rig.combat().env().unwrap().sticker(0).unwrap().dome(t);
    for (i, want) in [40.0, 60.0].into_iter().enumerate() {
        let g = &rig.combat().enemies[i];
        let body = crate::prototype::Rect {
            left: g.x - 12.0,
            bottom: g.floor_y - 18.0,
            width: 24.0,
            height: 36.0,
        };
        let d = crate::rot_sticker::dist2_to_rect(body, dome.0, dome.1).sqrt();
        assert!((d - want).abs() < 0.01, "goblin {i}: {d} px, wanted {want}");
    }
    let before: Vec<u8> = rig.combat().enemies.iter().map(|g| g.health).collect();
    let mut launched = [false; 2];
    for _ in 0..60 {
        rig.carl(RawInput::NONE);
        for (i, l) in launched.iter_mut().enumerate() {
            *l |= rig.combat().enemies[i].launch.is_some();
        }
    }
    assert_eq!(rig.blasts().len(), 1, "{:?}", rig.events());
    let after: Vec<u8> = rig.combat().enemies.iter().map(|g| g.health).collect();
    assert!(after[0] < before[0], "the inner goblin is hurt");
    assert_eq!(after[1], before[1], "the outer goblin is not");
    assert_eq!(launched, [true, false]);
    assert!(rig.provokes().iter().any(|e| matches!(
        e,
        CombatEvent::RotProvoked {
            player: Some(0),
            ..
        }
    )));
}

#[test]
fn haz_companion_boss_row() {
    // While a slime is in tell, launch or impact its slot supplies the single
    // `boss` input: `Pound { x, facing, target: None }`; otherwise none. The
    // slime kind is the room's one supplier (N3), so a second supplier kind or a
    // fixture boss beside it is refused at setup, never merged.
    use crate::boss::BossThreat;
    use crate::scene_enemy::{SlotError, check_suppliers};
    let mut rig = Layout {
        slimes: vec![SLIME_A],
        carl_x: 444,
        ..Layout::default()
    }
    .sim(None);
    assert_eq!(rig.combat().boss_threat(), None, "idle: no row");
    let mut rows = Vec::new();
    for _ in 0..90 {
        rig.carl(RawInput::NONE);
        let attack = attack_tick(&rig);
        let (x, facing) = (
            slime_of(&rig, 0).slime().x,
            slime_of(&rig, 0).slime().facing,
        );
        rows.push((attack, rig.combat().boss_threat(), x, facing));
    }
    for (attack, row, x, facing) in rows {
        // Attack ticks 0..=37: tell (30), launch (2), impact (6); then recover and
        // cooldown supply nothing.
        match attack {
            Some(a) if a <= 37 => {
                let Some(BossThreat::Pound {
                    x: px,
                    facing: pf,
                    target,
                }) = row
                else {
                    panic!("attack tick {a}: {row:?}");
                };
                assert_eq!((px, pf, target), (x, facing, None), "attack tick {a}");
            }
            _ => assert_eq!(row, None, "attack tick {attack:?}"),
        }
    }
    // The slime is never a companion target or threat (its row is the boss row
    // alone): the slot offers no other row.
    assert!(rig.combat().slot_enemies()[0].supplies_boss());
    // One supplier kind per room; a fixture boss beside it is refused.
    assert!(check_suppliers(&[("slime", true)], false).is_ok());
    assert!(matches!(
        check_suppliers(&[("slime", true), ("pile_boss", true)], false),
        Err(SlotError::SupplierConflict { .. })
    ));
    assert!(matches!(
        check_suppliers(&[("slime", true)], true),
        Err(SlotError::FixtureBossConflict { .. })
    ));
}

fn feed_of(rig: &Rig) -> Vec<crate::env_hazard::NavRect> {
    rig.combat()
        .hazard_feed(rig.scene().world.layout_height(), rig.scene().world.tick)
}

#[test]
fn haz_hazard_feed_rows() {
    use crate::env_hazard::{HazardClass, NavPhase};
    // An acid patch, a slime and a sticker (under the B slab) in one layout.
    let mut rig = Layout {
        acid: vec![(700, 764, None)],
        slimes: vec![SLIME_A],
        stickers: vec![Layout::under_slab(600, 600, 600)],
        carl_x: 112,
        ..Layout::default()
    }
    .sim(None);
    let feed = feed_of(&rig);
    // Stable order: acid (the hazard core's rows), then slimes, then stickers.
    let kinds: Vec<_> = feed.iter().map(|n| (n.class, n.phase)).collect();
    assert_eq!(
        kinds,
        [
            (HazardClass::Acid, NavPhase::Active),
            (HazardClass::Acid, NavPhase::Active),
        ],
        "{feed:?}: the acid patch, the slime body; the latched sticker has no row"
    );
    assert_eq!(feed[0].ticks_left, None, "the acid patch is persistent");
    assert_eq!(feed[1].hazard, crate::combat_scene::slime::FEED_ID_SLIME);
    assert!(feed[1].hurts_heroes && !feed[1].hurts_enemies);
    assert_eq!(feed, feed_of(&rig), "stable between reads");
    // Wake the sticker: Carl walks within 144 px of the dome; its trigger box is a
    // telegraphed `Blast` row without a count.
    rig.place(HeroId::Carl, 500.0);
    let mut awake = None;
    for _ in 0..40 {
        rig.carl(RawInput::NONE);
        let f = feed_of(&rig);
        if let Some(row) = f.iter().find(|n| n.class == HazardClass::Blast) {
            awake = Some(*row);
            break;
        }
    }
    let row = awake.expect("an awake sticker has a trigger row");
    assert_eq!(row.phase, NavPhase::Telegraph);
    assert_eq!(row.ticks_left, None);
    assert_eq!(row.hazard, crate::combat_scene::slime::FEED_ID_STICKER);
    // 32 px trigger circle around the dome: a 64 px box (plus the rounding out).
    assert!((64..=66).contains(&row.rect.w), "{:?}", row.rect);
    // Provoke it: the warning row is the 48 px blast box with ticks left, one fewer
    // each tick, and gone once it detonated.
    rig.place(HeroId::Carl, 600.0);
    let mut counts = Vec::new();
    let mut wide = None;
    for _ in 0..40 {
        rig.carl(RawInput::NONE);
        let f = feed_of(&rig);
        match f.iter().find(|n| n.class == HazardClass::Blast) {
            Some(n) if n.ticks_left.is_some() => {
                counts.push(n.ticks_left.unwrap());
                wide = Some(n.rect);
            }
            Some(_) => {}
            None if !counts.is_empty() => break,
            None => {}
        }
    }
    assert!(counts.len() >= 20, "{counts:?}");
    for pair in counts.windows(2) {
        assert_eq!(pair[0], pair[1] + 1, "{counts:?}");
    }
    let wide = wide.unwrap();
    assert!((96..=98).contains(&wide.w), "{wide:?}");
    assert!(
        feed_of(&rig).iter().all(|n| n.class != HazardClass::Blast),
        "a spent sticker has no row"
    );
    // No sticker has a damage-dealing hazard volume: the env's own feed (the hazard
    // core's rows and the acid) never carries a Blast row.
    assert!(
        rig.combat()
            .env()
            .unwrap()
            .nav_feed()
            .iter()
            .all(|n| n.class != HazardClass::Blast)
    );
}

// ------------------------------------------------------------- goldens

/// The hazard fixtures' first pinned state hashes (#214 FOLLOW-UP 9, #229
/// FOLLOW-UP 1). Each case replays twice to the same hash and matches its golden
/// here, so a change to what the slime, the acid or the sticker do (or to the
/// state they hash) is a visible edit of this table, never a silent one. The
/// presentation (HAZ-B) is not in the hash (`hazards_art`'s state-hash test).
type Case = (&'static str, fn() -> u64);

fn idle_run(mut rig: Rig, ticks: u64) -> u64 {
    for _ in 0..ticks {
        rig.step(RawInput::NONE, RawInput::NONE);
    }
    rig.hash()
}

fn pool_rig(carl_x: i32, linked: Option<u8>) -> Rig {
    Layout {
        acid: vec![(700, 764, linked)],
        slimes: if linked.is_some() {
            vec![SLIME_A]
        } else {
            vec![]
        },
        carl_x,
        ..Layout::default()
    }
    .sim(None)
}

fn case_slime_lunge_hits_idle_carl() -> u64 {
    idle_run(
        Layout {
            slimes: vec![SLIME_A],
            carl_x: 444,
            ..Layout::default()
        }
        .sim(None),
        80,
    )
}

fn case_slime_lunge_evaded_by_donut() -> u64 {
    idle_run(armed_party(438.0, 424.0), 90)
}

fn case_acid_pop_out() -> u64 {
    let mut rig = pool_rig(650, None);
    for k in 0..120 {
        rig.carl(if k < 60 { walk(1.0) } else { RawInput::NONE });
    }
    rig.hash()
}

fn case_acid_drain_after_the_slime() -> u64 {
    let mut rig = pool_rig(112, Some(0));
    rig.idle(10);
    rig.kill_slime(0);
    rig.idle(110);
    rig.hash()
}

fn case_rot_ceiling_provoke_and_blast() -> u64 {
    idle_run(
        Layout {
            stickers: vec![Layout::under_slab(600, 600, 600)],
            carl_x: 590,
            ..Layout::default()
        }
        .sim(None),
        60,
    )
}

fn case_rot_scuttle_wake() -> u64 {
    idle_run(
        Layout {
            stickers: vec![Layout::under_slab(560, 640, 640)],
            carl_x: 520,
            ..Layout::default()
        }
        .sim(None),
        100,
    )
}

fn case_rot_missile_near_miss() -> u64 {
    let mut rig = missile_rig(48, vec![]);
    let _ = fly(&mut rig);
    rig.hash()
}

fn case_room_idle_600() -> u64 {
    idle_run(Rig::room(false), 600)
}

fn case_room_two_player_walk() -> u64 {
    let mut rig = Rig::room(true);
    for k in 0..600u64 {
        let carl = if k < 300 { walk(1.0) } else { RawInput::NONE };
        let donut = if k % 90 < 45 { walk(1.0) } else { walk(-1.0) };
        rig.step(carl, donut);
    }
    rig.hash()
}

const GOLDEN_CASES: [(Case, u64); 9] = [
    (
        (
            "slime_lunge_hits_idle_carl",
            case_slime_lunge_hits_idle_carl,
        ),
        0x283294e39c30b750,
    ),
    (
        (
            "slime_lunge_evaded_by_donut",
            case_slime_lunge_evaded_by_donut,
        ),
        0x2490047559f7fde4,
    ),
    (("acid_pop_out", case_acid_pop_out), 0xccf2072aab48794d),
    (
        (
            "acid_drain_after_the_slime",
            case_acid_drain_after_the_slime,
        ),
        0x89c8b59c6279e432,
    ),
    (
        (
            "rot_ceiling_provoke_and_blast",
            case_rot_ceiling_provoke_and_blast,
        ),
        0x3e5aff3ae5035f3e,
    ),
    (
        ("rot_scuttle_wake", case_rot_scuttle_wake),
        0x2f03608c9eb73abd,
    ),
    (
        ("rot_missile_near_miss", case_rot_missile_near_miss),
        0xa75bd68f2cab1b45,
    ),
    (("room_idle_600", case_room_idle_600), 0x27dec748f4116792),
    (
        ("room_two_player_walk", case_room_two_player_walk),
        0x0756195b4d40de0f,
    ),
];

#[test]
fn the_hazard_fixtures_replay_identically_and_match_their_goldens() {
    let mut actual = Vec::new();
    for ((name, run), golden) in GOLDEN_CASES {
        let (a, b) = (run(), run());
        assert_eq!(a, b, "{name} replays");
        actual.push((name, a, golden));
    }
    let wrong: Vec<String> = actual
        .iter()
        .filter(|(_, a, g)| a != g)
        .map(|(n, a, g)| format!("{n}: {a:#018x} (pinned {g:#018x})"))
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
    let names: std::collections::BTreeSet<_> = GOLDEN_CASES.iter().map(|((n, _), _)| *n).collect();
    assert_eq!(names.len(), GOLDEN_CASES.len(), "distinct names");
}

/// A free-autonomy run (CA2): Donut is the autonomous partner and the leader
/// Carl stands to the right of the hazard. Returns `(her hurts, provokes)` after
/// `ticks`.
fn partner_run(layout: Layout, tuning: crate::party::PartyTuning, ticks: u64) -> (usize, usize) {
    let mut rig = Layout {
        tuning: Some(tuning),
        ..layout
    }
    .sim(Some(false));
    rig.place(HeroId::Donut, 470.0);
    for _ in 0..ticks {
        rig.step(RawInput::NONE, RawInput::NONE);
    }
    (rig.hurts(HeroId::Donut).len(), rig.provokes().len())
}

#[test]
fn haz_free_partner_is_not_hurt_by_the_hazards_the_feed_names() {
    // The autonomous partner (cf02_free) reads the hazard feed: a scuttling
    // sticker's trigger box is a hazard rectangle to the nav, so she does not walk
    // into it and wake the blast. The comparison run (the assist set has no goal
    // layer and no feed) does, which guards the case (CA2-C's steam precedent).
    let layout = || Layout {
        stickers: vec![Layout::under_slab(560, 640, 600)],
        carl_x: 760,
        ..Layout::default()
    };
    let (hurts, provokes) = partner_run(layout(), crate::cf02_free_fixtures::FREE, 600);
    assert_eq!(
        (hurts, provokes),
        (0, 0),
        "the free partner is neither hurt nor wakes it"
    );
    let assist = crate::party::PartyTuning {
        uncontrolled_hero: crate::party::UncontrolledHero::Companion(
            crate::companion::profile::ProfileSetId::Cf02Assist,
        ),
        ..crate::party::PROVISIONAL_TUNING
    }
    .with_cf02_swap();
    let (a_hurts, a_provokes) = partner_run(layout(), assist, 600);
    assert_eq!(
        a_provokes, 1,
        "the assist partner wakes it (hurts {a_hurts})"
    );
}
