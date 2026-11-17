use super::*;
use crate::actor_sprite::actor_center;
use crate::combat::CombatEvent;
use crate::combat_scene::{FIXTURES, Fixture, FixtureSim};
use crate::env_kit::EnvKit;
use crate::input::RawInput;
use crate::prototype::HERO_HALF_HEIGHT;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn kit() -> EnvKit {
    EnvKit::load(root(), "assets/levels/floor01/corridor_abc_sample.json").unwrap()
}

fn carl_clips() -> SpriteClips {
    SpriteClips::load_carl_melee(root(), FrameSource::GridSource).unwrap()
}

fn goblin_clips(variant: usize) -> SpriteClips {
    SpriteClips::load_goblin(root(), variant, FrameSource::GridSource).unwrap()
}

fn animators(sim: &FixtureSim) -> CombatAnimators {
    CombatAnimators::new(
        carl_clips(),
        [goblin_clips(0), goblin_clips(1)],
        &sim.combat,
    )
}

/// One recorded tick of a fixture run.
struct Row {
    tick: u64,
    carl: Option<ArtPose>,
    carl_frozen_move: bool,
    goblins: Vec<(GoblinMode, ArtPose, bool)>,
    foot_y: Vec<f32>,
    hero_foot_y: f32,
}

/// Run a fixture with `input` and record what is drawn after every tick.
fn record(
    fixture: &'static Fixture,
    ticks: u64,
    input: impl Fn(u64) -> RawInput,
) -> (Vec<Row>, FixtureSim) {
    let kit = kit();
    let mut sim = FixtureSim::new(&kit, 360, fixture).unwrap();
    let mut anim = animators(&sim);
    let mut rows = Vec::new();
    for _ in 0..ticks {
        let before = sim.scene.world.tick;
        sim.step(input(before + 1));
        if sim.scene.world.tick == before + 1 {
            anim.step(&sim.scene, &sim.combat);
        } else {
            anim.reset(&sim.combat);
        }
        rows.push(Row {
            tick: sim.scene.world.tick,
            carl: anim.carl_pose(),
            carl_frozen_move: sim
                .scene
                .world
                .hero
                .melee
                .current()
                .is_some_and(|m| m.frozen),
            goblins: (0..anim.goblin_count())
                .map(|i| {
                    (
                        sim.combat.enemies[i].mode,
                        anim.goblin_pose(i),
                        anim.goblin_input(i).frozen,
                    )
                })
                .collect(),
            foot_y: sim
                .combat
                .enemies
                .iter()
                .map(|g| g.y - ENEMY_HALF_HEIGHT)
                .collect(),
            hero_foot_y: sim.scene.world.hero.y - HERO_HALF_HEIGHT,
        });
    }
    (rows, sim)
}

fn scripted(fixture: &'static Fixture) -> (Vec<Row>, FixtureSim) {
    record(fixture, fixture.ticks, |tick| fixture.input(tick))
}

fn fixture(name: &str) -> &'static Fixture {
    Fixture::by_name(name).unwrap()
}

/// A kick against the stationary goblin of `cp01_two_hits`, pressed on tick 5.
fn kick_run() -> (Vec<Row>, FixtureSim) {
    record(fixture("cp01_two_hits"), 70, |tick| RawInput {
        ability: tick == 5,
        ..RawInput::default()
    })
}

/// Consecutive runs of the same value: `(value, length)`.
fn runs<T: PartialEq + Copy>(values: impl Iterator<Item = T>) -> Vec<(T, usize)> {
    let mut out: Vec<(T, usize)> = Vec::new();
    for v in values {
        match out.last_mut() {
            Some((last, n)) if *last == v => *n += 1,
            _ => out.push((v, 1)),
        }
    }
    out
}

fn pose(tag: &'static str, frame: usize) -> ArtPose {
    ArtPose { tag, frame }
}

fn goblin_input() -> GoblinInput {
    GoblinInput {
        mode: GoblinMode::Neutral,
        attack_tick: 0,
        frozen: false,
        down_ticks: 0,
        vy: 0.0,
        walking: false,
        walk_milli_px: 0,
        idle_tick: 0,
    }
}

// -------------------------------------------------------------- data ------

#[test]
fn atlas_geometry_comes_from_the_manifests() {
    let carl = carl_clips();
    let spec = carl.spec;
    assert_eq!(
        (spec.width, spec.height, spec.pivot_x, spec.pivot_y),
        (48, 56, 25, 55)
    );
    assert_eq!(carl.frame_count, 24);
    assert_eq!(carl.atlas_asset, "carl_book1_h52_melee.png");
    for (variant, file) in ["goblin_a_h36.png", "goblin_b_h36.png"].iter().enumerate() {
        let g = goblin_clips(variant);
        let spec = g.spec;
        assert_eq!(
            (spec.width, spec.height, spec.pivot_x, spec.pivot_y),
            (64, 56, 28, 55)
        );
        assert_eq!(g.frame_count, 27);
        assert_eq!(&g.atlas_asset, file);
    }
}

#[test]
fn variant_b_has_the_same_frames_tags_and_timing_as_a() {
    let (a, b) = (goblin_clips(0), goblin_clips(1));
    assert_eq!(a.tags, b.tags);
    assert_eq!(a.ticks, b.ticks);
    assert_eq!(a.airborne_tags, b.airborne_tags);
    assert_eq!(a.boxes.len(), b.boxes.len());
    assert_eq!(
        [goblin_variant(0), goblin_variant(1), goblin_variant(2)],
        [0, 1, 0]
    );
}

#[test]
fn the_art_timings_fit_the_sim_windows() {
    // A disagreement is reported here, not patched in the sim.
    for variant in 0..GOBLIN_VARIANTS {
        assert_eq!(
            goblin_clips(variant).sim_mismatches("goblin"),
            Vec::<String>::new()
        );
    }
    assert_eq!(carl_clips().sim_mismatches("carl"), Vec::<String>::new());
}

#[test]
fn tick_tables_match_the_manifests() {
    let carl = carl_clips();
    let expect = |tag: &str, ticks: &[Option<u32>]| {
        let (first, len) = carl.tag(tag);
        let got: Vec<_> = (first..first + len).map(|f| carl.frame_ticks(f)).collect();
        assert_eq!(got, ticks, "{tag}");
    };
    expect("punch_a", &[Some(3), Some(4), Some(4), Some(3)]);
    expect("punch_b", &[Some(5), Some(5), Some(6), Some(6)]);
    expect(
        "kick",
        &[
            Some(4),
            Some(7),
            Some(2),
            Some(6),
            Some(5),
            Some(7),
            Some(5),
        ],
    );
    expect("hurt", &[Some(4), Some(8), Some(6)]);
    expect("knockback", &[None]);
    expect("knockdown", &[Some(6), Some(30)]);
    expect("getup", &[Some(8), Some(8), Some(6)]);
    let goblin = goblin_clips(0);
    assert_eq!(goblin.tag_ticks("idle"), Some(40));
    assert_eq!(goblin.tag_ticks("walk"), Some(30));
    assert_eq!(goblin.tag_ticks("attack_tell"), Some(24));
    assert_eq!(goblin.tag_ticks("attack_swing"), Some(6));
    assert_eq!(goblin.tag_ticks("attack_recover"), Some(20));
    assert_eq!(goblin.tag_ticks("hurt"), Some(12));
    // Hold frames: the airborne frame, the lying frame and the corpse.
    assert_eq!(goblin.frame_ticks(19), None);
    assert_eq!(goblin.frame_ticks(22), None);
    assert_eq!(goblin.frame_ticks(25), None);
    assert_eq!(goblin.tag_ticks("knockback"), None);
}

#[test]
fn airborne_tags_have_no_pixel_on_the_pivot_row_and_grounded_ones_do() {
    for clips in [carl_clips(), goblin_clips(0), goblin_clips(1)] {
        for (name, (first, len)) in &clips.tags {
            for frame in *first..first + len {
                let lowest = clips.frame_box(frame).lowest;
                if clips.is_airborne_tag(name) {
                    assert!(lowest < clips.spec.pivot_y, "{name} frame {frame}");
                } else {
                    assert_eq!(lowest, clips.spec.pivot_y, "{name} frame {frame}");
                }
            }
        }
    }
    assert!(carl_clips().is_airborne_tag("knockback"));
    assert!(goblin_clips(0).is_airborne_tag("knockback"));
    assert!(goblin_clips(0).is_airborne_tag("knockback_contact"));
}

#[test]
fn bad_data_is_rejected() {
    assert!(SpriteClips::load_goblin(root(), 2, FrameSource::GridSource).is_err());
    let frames = serde_json::json!([{"index": 0, "ticks": 3}]);
    assert!(goblin_ticks(&serde_json::json!({"goblin": {"frames": frames}}), 2).is_err());
    assert!(range_ticks(&serde_json::json!([5, 2]), "x").is_err());
    assert_eq!(range_ticks(&serde_json::json!([3, 6]), "x"), Ok(Some(4)));
    assert_eq!(range_ticks(&serde_json::Value::Null, "x"), Ok(None));
}

// ----------------------------------------------------- frame mapping ---

#[test]
fn carl_move_frames_follow_the_move_tick() {
    let clips = carl_clips();
    let frame = |id: MoveId, tick: u16| {
        carl_pose(
            &clips,
            &CarlInput {
                melee: Some((id, tick)),
                grounded: true,
                ..CarlInput::default()
            },
        )
        .unwrap()
        .frame
    };
    // (move, first tick of each art frame, atlas frames in order)
    let table: [(MoveId, &[u16], &[usize]); 3] = [
        (MoveId::Jab, &[0, 3, 7, 11], &[0, 1, 2, 3]),
        (MoveId::Haymaker, &[0, 5, 10, 16], &[4, 5, 6, 7]),
        (
            MoveId::PushKick,
            &[0, 4, 11, 13, 19, 24, 31],
            &[8, 9, 10, 11, 12, 13, 14],
        ),
    ];
    for (id, starts, frames) in table {
        let total = crate::combat::carl_move(id).unwrap().total();
        for tick in 0..total {
            let index = starts.iter().rposition(|s| *s <= tick).unwrap();
            assert_eq!(frame(id, tick), frames[index], "{id:?} tick {tick}");
        }
    }
    // The greybox strike has no art and leaves Carl to the locomotion clip.
    let greybox = CarlInput {
        melee: Some((MoveId::GreyboxStrike, 0)),
        ..CarlInput::default()
    };
    assert!(carl_pose(&clips, &greybox).is_none());
    assert!(carl_pose(&clips, &CarlInput::default()).is_none());
}

#[test]
fn carl_hurt_and_knockdown_follow_the_sim_states() {
    let clips = carl_clips();
    let hurt = |hurt_ticks: u8| {
        carl_pose(
            &clips,
            &CarlInput {
                hurt_ticks,
                grounded: true,
                ..CarlInput::default()
            },
        )
        .unwrap()
    };
    // 18 ticks of flinch: h0 for ticks 0..3, h1 4..11, h2 12..17.
    for age in 0..18u8 {
        let want = match age {
            0..=3 => 15,
            4..=11 => 16,
            _ => 17,
        };
        assert_eq!(hurt(18 - age), pose("hurt", want), "flinch tick {age}");
    }
    let down = |frozen, grounded, landed_age| {
        carl_pose(
            &clips,
            &CarlInput {
                defeated: true,
                hurt_ticks: 5,
                frozen,
                grounded,
                landed_age,
                ..CarlInput::default()
            },
        )
        .unwrap()
    };
    // The blow's freeze shows the impact frame, the flight the launched frame,
    // touchdown d0 for 6 ticks, then the sprawl d1 until the restart.
    assert_eq!(down(true, false, None), pose("hurt", 15));
    assert_eq!(down(false, false, None), pose("knockback", 18));
    for age in 0..60 {
        let want = if age < 6 { 19 } else { 20 };
        assert_eq!(down(false, true, Some(age)), pose("knockdown", want));
    }
}

#[test]
fn getup_frames_exist_for_a_future_sim_state_but_nothing_drives_them_yet() {
    // The fixture restarts after the knockdown, so no sim state gets Carl up:
    // the art is mapped (8 + 8 + 6 ticks) and tested, not wired.
    let clips = carl_clips();
    let frames: Vec<usize> = (0..22).map(|t| clips.frame_at("getup", t)).collect();
    assert_eq!(runs(frames.into_iter()), [(21, 8), (22, 8), (23, 6)]);
    assert_eq!(clips.frame_at("getup", 99), 23);
}

#[test]
fn goblin_attack_frames_follow_the_attack_tick() {
    let clips = goblin_clips(0);
    let at = |tick: u16| {
        goblin_pose(
            &clips,
            &GoblinInput {
                mode: GoblinMode::Attack,
                attack_tick: tick,
                ..goblin_input()
            },
        )
    };
    let got = runs((0..50).map(|t| at(t).frame));
    assert_eq!(
        got,
        [(10, 6), (11, 18), (12, 2), (13, 4), (14, 12), (15, 8)]
    );
    // The impact frame is shown exactly on the swing's hit window.
    for tick in 0..50u16 {
        let window = (SMEAR_END..IMPACT_END).contains(&tick);
        assert_eq!(at(tick).frame == 13, window, "attack tick {tick}");
    }
}

#[test]
fn goblin_neutral_walk_idle_down_and_corpse_mapping() {
    let clips = goblin_clips(0);
    let draw = |input: GoblinInput| goblin_pose(&clips, &input);
    // Idle loops 12, 8, 12, 8 ticks.
    let idle = runs((0..80u64).map(|t| {
        draw(GoblinInput {
            idle_tick: t,
            ..goblin_input()
        })
        .frame
    }));
    assert_eq!(
        idle,
        [
            (0, 12),
            (1, 8),
            (2, 12),
            (3, 8),
            (0, 12),
            (1, 8),
            (2, 12),
            (3, 8)
        ]
    );
    // Walk: one frame per 5 px of ground, six frames, then it loops.
    let walk: Vec<usize> = (0..13u32)
        .map(|step| {
            draw(GoblinInput {
                walking: true,
                walk_milli_px: step * 2500,
                ..goblin_input()
            })
            .frame
        })
        .collect();
    assert_eq!(
        runs(walk.into_iter()),
        [(4, 2), (5, 2), (6, 2), (7, 2), (8, 2), (9, 2), (4, 1)]
    );
    // Down: floor impact 4 ticks, settle 8, then the dazed hold; the corpse.
    let down = |ticks: u16| {
        draw(GoblinInput {
            mode: GoblinMode::Down,
            down_ticks: ticks,
            ..goblin_input()
        })
        .frame
    };
    assert_eq!(runs((0..30).map(down)), [(20, 4), (21, 8), (22, 18)]);
    let corpse = draw(GoblinInput {
        mode: GoblinMode::Corpse,
        ..goblin_input()
    });
    assert_eq!(corpse, pose("corpse", 25));
}

#[test]
fn walk_is_paced_to_the_chase_speed() {
    // 62 px/s at 5 px per frame: 4.84 ticks per frame, 29 ticks per cycle.
    let (rows, _) = scripted(fixture("cp01_goblin_attacks"));
    let drawn: Vec<ArtPose> = rows.iter().map(|r| r.goblins[0].1).collect();
    let start = drawn.iter().position(|p| p.tag == "walk").unwrap();
    let end = start + drawn[start..].iter().position(|p| p.tag != "walk").unwrap();
    let cycle = runs(drawn[start..end].iter().map(|p| p.frame));
    assert_eq!(cycle[0].0, 4, "the walk starts on the contact frame");
    for (frame, len) in &cycle[..cycle.len() - 1] {
        assert!((4..=5).contains(len), "frame {frame} shown {len} ticks");
    }
    let order: Vec<usize> = cycle.iter().map(|c| c.0).collect();
    let expect: Vec<usize> = (0..order.len()).map(|i| 4 + i % 6).collect();
    assert_eq!(order, expect);
    // The gait covers 5 px per frame: a full cycle is 30 px of 62 px/s ground.
    let full: usize = cycle.iter().take(6).map(|c| c.1).sum();
    assert!((28..=30).contains(&full), "a cycle is {full} ticks");
}

// --------------------------------------------------- against the sim ------

#[test]
fn the_contact_frame_is_shown_on_the_hit_tick() {
    // Jab hits on move tick 3 (a1), haymaker on 5 (b1), kick on 13 (k3).
    let (rows, sim) = scripted(fixture("cp01_haymaker_chain"));
    let hits: Vec<u64> = sim
        .combat
        .events
        .iter()
        .filter_map(|e| match e {
            CombatEvent::Hit { tick, .. } => Some(*tick),
            _ => None,
        })
        .collect();
    assert_eq!(hits, [140, 152]);
    let at = |rows: &[Row], tick: u64| rows.iter().find(|r| r.tick == tick).unwrap().carl;
    assert_eq!(at(&rows, 140), Some(pose("punch_a", 1)));
    assert_eq!(at(&rows, 152), Some(pose("punch_b", 5)));
    // The tick before each hit is still the startup frame.
    assert_eq!(at(&rows, 139), Some(pose("punch_a", 0)));
    assert_eq!(at(&rows, 151), Some(pose("punch_b", 4)));
    // The kick against a stationary goblin: hit on move tick 13 (k3).
    let (rows, sim) = kick_run();
    let hit = sim
        .combat
        .events
        .iter()
        .find_map(|e| match e {
            CombatEvent::Hit { tick, move_id, .. } => Some((*tick, *move_id)),
            _ => None,
        })
        .expect("the kick hits the stationary goblin");
    assert_eq!(hit.1, MoveId::PushKick);
    assert_eq!(at(&rows, hit.0), Some(pose("kick", 11)));
    assert_eq!(at(&rows, hit.0 - 1), Some(pose("kick", 10)));
}

#[test]
fn the_frame_is_held_during_hit_stop() {
    for fixture in FIXTURES {
        let (rows, _) = scripted(fixture);
        for pair in rows.windows(2) {
            let (prev, cur) = (&pair[0], &pair[1]);
            if cur.tick != prev.tick + 1 {
                continue; // a restart
            }
            if cur.carl_frozen_move {
                assert_eq!(
                    cur.carl, prev.carl,
                    "{} tick {} Carl",
                    fixture.name, cur.tick
                );
            }
            for (i, (now, before)) in cur.goblins.iter().zip(&prev.goblins).enumerate() {
                // A frozen goblin keeps its state: its frame must not move. The
                // tick the freeze starts is not covered: that tick still ran (a
                // hit on the goblin, or its swing landing on Carl).
                if now.2 && before.2 && now.0 == before.0 {
                    assert_eq!(
                        now.1, before.1,
                        "{} tick {} goblin {i}",
                        fixture.name, cur.tick
                    );
                }
            }
        }
    }
}

#[test]
fn f1_the_kill_shows_recoil_during_hit_stop_then_launch_then_air_then_down() {
    let (rows, _) = scripted(fixture("cp01_haymaker_chain"));
    let kill = rows.iter().position(|r| r.tick == 152).unwrap();
    let drawn = rows[kill..].iter().map(|r| {
        let p = r.goblins[0].1;
        (p.tag, p.frame)
    });
    let sequence = runs(drawn);
    // Freeze: the hit tick and 8 more (haymaker 5 + lethal bonus 3) show the
    // recoil; the launch frame for 4 ticks from the release; the airborne frame
    // until touchdown; down 4 + 8 + hold; then the corpse.
    assert_eq!(sequence[0], (("hurt", 16), 9), "{sequence:?}");
    assert_eq!(sequence[1], (("knockback", 18), 4), "{sequence:?}");
    assert_eq!(sequence[2].0, ("knockback", 19));
    assert_eq!(sequence[3], (("down", 20), 4));
    assert_eq!(sequence[4], (("down", 21), 8));
    assert_eq!(sequence[5].0, ("down", 22));
    assert_eq!(sequence[6].0, ("corpse", 25));
    assert_eq!(sequence.len(), 7, "{sequence:?}");
    // The down frames run for the sim's 30 lying ticks (4 + 8 + 18).
    assert_eq!(sequence[3].1 + sequence[4].1 + sequence[5].1, 30);
}

#[test]
fn f2_a_stagger_shows_recoil_for_4_ticks_then_holds_the_stagger_frame() {
    // Jab: hit-stop 3, so 4 frozen ticks; stagger 10.
    let (rows, _) = scripted(fixture("cp01_interrupt"));
    let hit = rows.iter().position(|r| r.tick == 140).unwrap();
    let drawn = rows[hit..]
        .iter()
        .map(|r| r.goblins[0].1)
        .take_while(|p| p.tag == "hurt")
        .map(|p| p.frame);
    let sequence = runs(drawn);
    assert_eq!(sequence[0], (16, 4), "{sequence:?}");
    assert_eq!(sequence[1].0, 17);
    assert_eq!(sequence.len(), 2);
    // The sim staggers it for 10 ticks, the last of which hands over.
    assert_eq!(sequence[1].1, 9, "{sequence:?}");
    // A longer freeze keeps the recoil for the whole freeze: the kick's 8 ticks
    // of hit-stop (9 frozen ticks) hold frame 16 throughout.
    let (rows, _) = kick_run();
    let hurt = runs(
        rows.iter()
            .map(|r| r.goblins[0].1)
            .filter(|p| p.tag == "hurt")
            .map(|p| p.frame),
    );
    assert_eq!(hurt[0], (16, 9), "{hurt:?}");
    assert_eq!(hurt[1].0, 17);
}

#[test]
fn no_frame_index_is_ever_out_of_range_and_every_frame_belongs_to_its_tag() {
    let carl = carl_clips();
    let goblins = [goblin_clips(0), goblin_clips(1)];
    for fixture in FIXTURES {
        let (rows, _) = scripted(fixture);
        for row in &rows {
            if let Some(pose) = row.carl {
                let (first, len) = carl.tag(pose.tag);
                assert!(pose.frame < carl.frame_count);
                assert!((first..first + len).contains(&pose.frame));
            }
            for (i, (_, pose, _)) in row.goblins.iter().enumerate() {
                let clips = &goblins[goblin_variant(i)];
                let (first, len) = clips.tag(pose.tag);
                assert!(pose.frame < clips.frame_count);
                assert!(
                    (first..first + len).contains(&pose.frame),
                    "{} tick {} goblin {i}: {pose:?}",
                    fixture.name,
                    row.tick
                );
            }
        }
    }
}

#[test]
fn every_state_the_fixtures_reach_is_drawn() {
    let mut seen = BTreeSet::new();
    for fixture in FIXTURES {
        let (rows, _) = scripted(fixture);
        for row in &rows {
            seen.extend(row.goblins.iter().map(|g| g.1.tag));
            seen.extend(row.carl.map(|c| c.tag));
        }
    }
    let (rows, _) = kick_run();
    seen.extend(rows.iter().filter_map(|r| r.carl.map(|c| c.tag)));
    for tag in [
        "idle",
        "walk",
        "attack_tell",
        "attack_swing",
        "attack_recover",
        "hurt",
        "knockback",
        "down",
        "corpse",
        "punch_a",
        "punch_b",
        "kick",
        "knockdown",
    ] {
        assert!(seen.contains(tag), "{tag} never drawn: {seen:?}");
    }
}

// ------------------------------------------------------- placement ------

/// World y of the bottom edge of frame row `row` of a sprite centred at `cy`.
fn row_bottom(clips: &SpriteClips, cy: f32, row: i32) -> f32 {
    cy + clips.spec.height as f32 / 2.0 - (row + 1) as f32
}

#[test]
fn airborne_frames_sit_on_the_sim_body_and_grounded_ones_on_the_foot() {
    let carl = carl_clips();
    let goblins = [goblin_clips(0), goblin_clips(1)];
    for name in ["cp01_haymaker_chain", "cp01_single_goblin", "cp01_retry"] {
        let (rows, _) = scripted(fixture(name));
        for row in &rows {
            for (i, (_, pose, _)) in row.goblins.iter().enumerate() {
                let clips = &goblins[goblin_variant(i)];
                let foot = row.foot_y[i];
                let anchored = clips.anchor_foot_y(pose, foot);
                let (_, cy) = actor_center(clips.spec, 400.0, anchored, false);
                let lowest = clips.frame_box(pose.frame).lowest;
                // The lowest opaque row of the drawn frame is on the bottom of
                // the sim body (within the rounding of the sprite centre):
                // no hover, no sink, grounded or airborne.
                let gap = row_bottom(clips, cy, lowest) - foot;
                assert!(
                    gap.abs() <= 0.5,
                    "{name} tick {} goblin {i} {pose:?}: gap {gap}",
                    row.tick
                );
            }
            if let Some(pose) = row.carl {
                let anchored = carl.anchor_foot_y(&pose, row.hero_foot_y);
                let (_, cy) = actor_center(carl.spec, 400.0, anchored, false);
                let lowest = carl.frame_box(pose.frame).lowest;
                let gap = row_bottom(&carl, cy, lowest) - row.hero_foot_y;
                assert!(gap.abs() <= 0.5, "{name} tick {} Carl: {gap}", row.tick);
            }
        }
    }
}

#[test]
fn the_pivot_rule_alone_would_hover_the_knockback_frames() {
    // Why airborne frames are anchored by their lowest row: with the pivot on
    // the body's bottom the launched frames float 2 to 17 px above it.
    let goblin = goblin_clips(0);
    for (frame, lift) in [(18usize, 2), (19, 9), (26, 11)] {
        assert_eq!(goblin.spec.pivot_y - goblin.frame_box(frame).lowest, lift);
    }
    let carl = carl_clips();
    assert_eq!(carl.spec.pivot_y - carl.frame_box(18).lowest, 17);
}

#[test]
fn launch_age_is_recovered_from_the_vertical_speed_of_a_real_flight() {
    let fixture = fixture("cp01_haymaker_chain");
    let kit = kit();
    let mut sim = FixtureSim::new(&kit, 360, fixture).unwrap();
    let mut ages = Vec::new();
    for _ in 0..fixture.ticks {
        sim.step_scripted();
        let g = &sim.combat.enemies[0];
        if g.mode == GoblinMode::Airborne {
            ages.push(launch_age(g.vy));
        }
    }
    // The frozen ticks still read the launch speed (age 0), then 1, 2, 3, ...
    assert!(ages.iter().take(9).all(|a| *a == 0), "{ages:?}");
    let flight: Vec<u32> = ages.iter().copied().skip_while(|a| *a == 0).collect();
    let expected: Vec<u32> = (1..=flight.len() as u32).collect();
    assert_eq!(flight, expected);
}

// --------------------------------------------- presentation boundary -----

#[test]
fn animation_does_not_change_the_simulation() {
    for fixture in FIXTURES {
        let kit = kit();
        let mut plain = FixtureSim::new(&kit, 360, fixture).unwrap();
        let mut shown = FixtureSim::new(&kit, 360, fixture).unwrap();
        let mut anim = animators(&shown);
        for _ in 0..fixture.ticks {
            let before = shown.scene.world.tick;
            plain.step_scripted();
            shown.step_scripted();
            if shown.scene.world.tick == before + 1 {
                anim.step(&shown.scene, &shown.combat);
            } else {
                anim.reset(&shown.combat);
            }
            let _ = anim.state_line(shown.scene.world.tick);
            let _ = anim.trace_line(&shown.scene, &shown.combat);
            assert_eq!(plain.state_hash(), shown.state_hash(), "{}", fixture.name);
        }
    }
}

#[test]
fn the_frame_sequence_is_deterministic() {
    for name in ["cp01_haymaker_chain", "cp02_two_goblins"] {
        let label = |rows: &[Row]| -> Vec<String> {
            rows.iter()
                .map(|r| {
                    let goblins: Vec<ArtPose> = r.goblins.iter().map(|g| g.1).collect();
                    format!("{:?} {goblins:?}", r.carl)
                })
                .collect()
        };
        let (a, _) = scripted(fixture(name));
        let (b, _) = scripted(fixture(name));
        assert_eq!(label(&a), label(&b), "{name}");
    }
}

#[test]
fn a_restart_resets_the_animation() {
    let (rows, _) = scripted(fixture("cp01_retry"));
    // The fixture restarts once: right after it Carl is back to the locomotion
    // animation and every goblin to a neutral frame.
    let restart = rows.windows(2).position(|p| p[1].tick < p[0].tick).unwrap();
    let after = &rows[restart + 1];
    assert!(after.carl.is_none());
    assert!(after.goblins.iter().all(|g| g.0 == GoblinMode::Neutral));
}

#[test]
fn contact_shadows_follow_the_frame_on_the_floor() {
    let goblin = goblin_clips(0);
    assert_eq!(
        goblin.shadow_footprint(&pose("idle", 0)),
        GOBLIN_SHADOW_FOOTPRINT
    );
    let (width, pivot) = goblin.shadow_footprint(&pose("down", 22));
    assert!(width > GOBLIN_SHADOW_FOOTPRINT.0 && width <= goblin.spec.width);
    assert!((0..=goblin.spec.width).contains(&pivot));
    assert_eq!(
        carl_clips().shadow_footprint(&pose("punch_a", 1)),
        CARL_SHADOW_FOOTPRINT
    );
}

#[test]
fn carl_is_above_the_goblin_unless_it_swings_and_he_is_not_active() {
    assert!(carl_above_goblin(false, false));
    assert!(carl_above_goblin(true, false));
    assert!(carl_above_goblin(true, true), "his fist must not be hidden");
    assert!(!carl_above_goblin(false, true));
}

// ----------------------------------------------------------- export ------

#[test]
fn exported_atlas_json_matches_the_grid_source_when_present() {
    // The export is gitignored: compare when it exists. With
    // REQUIRE_EXPORTED_ATLAS=1 (set in the floor01-scene workflow after the
    // export step) a missing export fails the test instead of skipping it.
    let required = std::env::var_os("REQUIRE_EXPORTED_ATLAS").is_some_and(|v| v == "1");
    let atlases = [
        ("carl_book1_h52_melee.json", Kind::CarlMelee),
        ("goblin_a_h36.json", Kind::Goblin),
        ("goblin_b_h36.json", Kind::Goblin),
    ];
    for (manifest_file, kind) in atlases {
        let manifest: serde_json::Value = serde_json::from_str(
            &read(root(), &format!("assets/manifests/{manifest_file}")).unwrap(),
        )
        .unwrap();
        let atlas_path = manifest["output_metadata"].as_str().unwrap();
        let png_path = manifest["output_atlas"].as_str().unwrap();
        if !root().join(atlas_path).exists() || !root().join(png_path).exists() {
            assert!(
                !required,
                "{atlas_path} or {png_path} is missing but REQUIRE_EXPORTED_ATLAS=1"
            );
            continue;
        }
        let load =
            |source| SpriteClips::load(root(), "T", manifest_file, (1, 1), source, kind).unwrap();
        assert_eq!(
            load(FrameSource::ExportedAtlas),
            load(FrameSource::GridSource),
            "{manifest_file}"
        );
        // The atlas durations are the ticks converted to ms (previews only).
        let atlas: AtlasFile = serde_json::from_str(&read(root(), atlas_path).unwrap()).unwrap();
        let clips = load(FrameSource::ExportedAtlas);
        for (index, frame) in atlas.frames.iter().enumerate() {
            if let Some(ticks) = clips.frame_ticks(index) {
                let ms = (f64::from(ticks) * 1000.0 / 60.0).round() as u32;
                assert_eq!(frame.duration, ms, "{manifest_file} frame {index}");
            }
        }
    }
}
