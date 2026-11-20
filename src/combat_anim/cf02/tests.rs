use super::*;
use crate::actor_sprite::{FrameSource, load_carl, load_donut};
use crate::anim::Animator;
use crate::cf02::Cf02Tuning;
use crate::cf02_fixtures::{find_party_fixture, list_fixtures};
use crate::combat::{CombatEvent, carl_move, cf02_moves};
use crate::combat_anim::{
    CARL_CF02_MANIFEST, DONUT_CF02_MANIFEST, FOLLOW_KICK_FRAMES, GOBLIN_CF02_MANIFESTS,
    GOBLIN_SLAMMED_MANIFESTS,
};
use crate::env_kit::EnvKit;
use crate::party_scene::PartyFixtureSim;
use crate::scene_anim::SceneAnimators;
use std::collections::BTreeSet;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn art() -> ActingArt {
    ActingArt::load(root(), FrameSource::GridSource).unwrap()
}

fn on() -> &'static Cf02Tuning {
    crate::cf02::active()
}

/// The pinned tuning with the rules on and the overlay switch off.
fn overlays_off() -> &'static Cf02Tuning {
    let mut t = Cf02Tuning::pinned().unwrap();
    t.cf02_rules = true;
    t.q69_acting_overlays = false;
    Box::leak(Box::new(t))
}

fn manifest(file: &str) -> serde_json::Value {
    serde_json::from_str(
        &std::fs::read_to_string(root().join("assets/manifests").join(file)).unwrap(),
    )
    .unwrap()
}

/// Frame names of a CF02 sheet, by atlas index, from its manifest.
fn frame_names(file: &str, block: &str) -> Vec<String> {
    let m = manifest(file);
    if block == "cf02" {
        // Donut: a name to index map.
        let map = m["cf02"]["frames"].as_object().unwrap();
        let mut names = vec![String::new(); map.len()];
        for (name, v) in map {
            names[v["index"].as_u64().unwrap() as usize] = name.clone();
        }
        return names;
    }
    m[block]["frames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap().to_owned())
        .collect()
}

fn carl_name(_art: &ActingArt, pose: ArtPose) -> String {
    frame_names(CARL_CF02_MANIFEST, "cf02_acting")[pose.frame].clone()
}

#[test]
fn every_cf02_sheet_loads_with_timed_frames_and_no_airborne_anchor() {
    let a = art();
    assert_eq!(a.carl.frame_count, 13);
    assert_eq!(a.donut.frame_count, 3);
    for g in &a.goblins {
        assert_eq!(g.frame_count, 5);
    }
    for g in &a.slammed {
        assert_eq!(g.frame_count, 6);
    }
    // Pivot-anchored: the sheets' airborne lift is baked in.
    for clips in [&a.carl, &a.donut, &a.goblins[0], &a.slammed[1]] {
        for tag in ["stomp", "bailout", "startle", "hesitate", "slammed_wall"] {
            if clips.has_tag(tag) {
                assert!(!clips.is_airborne_tag(tag));
            }
        }
    }
    // The ticks of the manifest: stomp 9, 7, 10; the plunge and the scramble hold.
    let ticks = |c: &SpriteClips, tag: &str| {
        let (first, len) = c.tag(tag);
        (first..first + len)
            .map(|f| c.frame_ticks(f))
            .collect::<Vec<_>>()
    };
    assert_eq!(ticks(&a.carl, "stomp"), [Some(9), Some(7), Some(10)]);
    assert_eq!(ticks(&a.carl, "air_kick"), [Some(5), Some(10)]);
    assert_eq!(ticks(&a.carl, "dive_tuck"), [Some(6)]);
    assert_eq!(ticks(&a.carl, "dive_plunge"), [None]);
    assert_eq!(ticks(&a.carl, "dive_land"), [Some(10)]);
    assert_eq!(ticks(&a.carl, "bailout"), [Some(3), None]);
    assert_eq!(ticks(&a.carl, "flinch"), [Some(8)]);
    assert_eq!(ticks(&a.carl, "frustration"), [Some(6), Some(6)]);
    assert_eq!(ticks(&a.donut, "startle"), [Some(6), Some(6)]);
    assert_eq!(ticks(&a.donut, "smug"), [Some(20)]);
    assert_eq!(ticks(&a.goblins[0], "hesitate"), [Some(20)]);
    assert_eq!(ticks(&a.goblins[1], "panic"), [Some(6), Some(6)]);
    assert_eq!(ticks(&a.goblins[0], "taunt"), [Some(9), Some(9)]);
    assert_eq!(ticks(&a.slammed[0], "slammed_stomped"), [Some(8), None]);
}

#[test]
fn the_exported_atlas_agrees_with_the_grid_source_when_present() {
    let exported = root().join("generated/art/carl_book1_h52_cf02.json");
    if !exported.is_file() {
        assert!(
            std::env::var("REQUIRE_EXPORTED_ATLAS").is_err(),
            "the exported CF02 atlases are required"
        );
        return;
    }
    let grid = ActingArt::load(root(), FrameSource::GridSource).unwrap();
    let atlas = ActingArt::load(root(), FrameSource::ExportedAtlas).unwrap();
    assert_eq!(grid, atlas);
}

// --- frame per state: Carl -------------------------------------------------

fn carl_at(a: &ActingArt, input: CarlActInput) -> Option<String> {
    carl_move_pose(&a.carl, &input).map(|p| carl_name(a, p))
}

fn melee(id: MoveId, tick: u16) -> CarlActInput {
    CarlActInput {
        melee: Some((id, tick)),
        ..CarlActInput::default()
    }
}

#[test]
fn the_stomp_shows_knee_up_then_stamp_then_recover() {
    let a = art();
    let spec = &cf02_moves().stomp;
    assert_eq!(spec.total(), 26, "the art covers the whole move");
    for (tick, want) in [
        (0, "stomp_knee_up"),
        (8, "stomp_knee_up"),
        (9, "stomp_stamp"), // the hit tick
        (15, "stomp_stamp"),
        (16, "stomp_recover"),
        (25, "stomp_recover"),
    ] {
        assert_eq!(
            carl_at(&a, melee(MoveId::Stomp, tick)).as_deref(),
            Some(want),
            "tick {tick}"
        );
    }
    assert_eq!(spec.startup, 9, "the stamp is the first active tick");
}

#[test]
fn the_follow_kick_skips_the_chamber_and_hits_on_the_contact_frame() {
    let a = art();
    let kit = SpriteClips::load_carl_melee(root(), FrameSource::GridSource).unwrap();
    let spec = &cf02_moves().follow_kick;
    let total: u32 = FOLLOW_KICK_FRAMES.iter().map(|(_, t)| t).sum();
    assert_eq!(
        total,
        u32::from(spec.total()),
        "the table covers FollowKick"
    );
    let (first, _) = kit.tag("kick");
    let mut seen = BTreeSet::new();
    for tick in 0..spec.total() {
        seen.insert(super::super::follow_kick_frame(&kit, tick) - first);
    }
    assert_eq!(
        seen.into_iter().collect::<Vec<_>>(),
        [0, 2, 3, 4, 5, 6],
        "no k1"
    );
    // The first active tick is the contact frame the strike geometry reads.
    let hit = super::super::follow_kick_frame(&kit, spec.startup);
    assert_eq!(hit, first + 3);
    let pose = super::super::carl_pose(
        &kit,
        &CarlInput {
            melee: Some((MoveId::FollowKick, spec.startup)),
            grounded: true,
            ..CarlInput::default()
        },
    )
    .unwrap();
    assert!(super::super::is_contact_pose(&kit, &pose));
    // The 9 tick startup is k0 but its last two ticks (k2): no long reach early.
    assert_eq!(super::super::follow_kick_frame(&kit, 6) - first, 0);
    assert_eq!(super::super::follow_kick_frame(&kit, 7) - first, 2);
    assert_eq!(super::super::follow_kick_frame(&kit, 8) - first, 2);
    let _ = a;
}

#[test]
fn the_air_kick_shows_the_chamber_then_the_boot_then_the_locomotion_frame() {
    let a = art();
    let spec = &cf02_moves().air_kick;
    assert_eq!(spec.total(), 21);
    for (tick, want) in [
        (0, Some("air_kick_chamber")),
        (4, Some("air_kick_chamber")),
        (5, Some("air_kick_boot")), // first active tick
        (14, Some("air_kick_boot")),
        (15, None), // the existing jump_fall
        (20, None),
    ] {
        assert_eq!(
            carl_at(&a, melee(MoveId::AirKick, tick)).as_deref(),
            want,
            "tick {tick}"
        );
    }
    assert_eq!(spec.startup, 5);
}

#[test]
fn the_dive_shows_the_tuck_then_the_plunge_then_the_locomotion_frame() {
    let a = art();
    let spec = &cf02_moves().dive_stomp;
    assert_eq!(spec.startup, 6, "the tuck is the startup");
    for (tick, want) in [
        (0, Some("dive_tuck")),
        (5, Some("dive_tuck")),
        (6, Some("dive_plunge")),
        (35, Some("dive_plunge")),
        (36, None),
    ] {
        assert_eq!(
            carl_at(&a, melee(MoveId::DiveStomp, tick)).as_deref(),
            want,
            "tick {tick}"
        );
    }
    // The tuck against the plunge: the tell is a visible height difference.
    let (tuck, plunge) = (a.carl.tag("dive_tuck").0, a.carl.tag("dive_plunge").0);
    let (bt, bp) = (a.carl.frame_box(tuck), a.carl.frame_box(plunge));
    assert!((bt.lowest - bt.top) + 6 <= bp.lowest - bp.top);
}

#[test]
fn the_bail_out_shows_the_pushoff_then_the_scramble_until_landing() {
    let a = art();
    for (age, want) in [
        (0, "bailout_pushoff"),
        (2, "bailout_pushoff"),
        (3, "bailout_scramble"),
        (33, "bailout_scramble"),
        (200, "bailout_scramble"),
    ] {
        let input = CarlActInput {
            bail_age: Some(age),
            ..CarlActInput::default()
        };
        assert_eq!(carl_at(&a, input).as_deref(), Some(want), "age {age}");
    }
}

#[test]
fn a_dive_whiff_lands_heavily_for_ten_ticks() {
    let a = art();
    let ticks = a.carl.tag_ticks("dive_land").unwrap();
    assert_eq!(ticks, u32::from(on().q56_landing_dive_whiff));
    for age in [0, 9] {
        let input = CarlActInput {
            heavy_landing_age: Some(age),
            ..CarlActInput::default()
        };
        assert_eq!(carl_at(&a, input).as_deref(), Some("dive_heavy_landing"));
    }
    // A move that is running wins over the landing frame.
    let mut input = melee(MoveId::Stomp, 0);
    input.heavy_landing_age = Some(0);
    assert_eq!(carl_at(&a, input).as_deref(), Some("stomp_knee_up"));
}

#[test]
fn flinch_and_frustration_frames() {
    let a = art();
    let name = |p: ArtPose| carl_name(&a, p);
    assert_eq!(
        name(carl_overlay_pose(&a.carl, CarlOverlay::Flinch, 0)),
        "flinch"
    );
    assert_eq!(
        name(carl_overlay_pose(&a.carl, CarlOverlay::Flinch, 7)),
        "flinch"
    );
    for (age, want) in [
        (0, "frustration_shake"),
        (5, "frustration_shake"),
        (6, "frustration_flap"),
        (11, "frustration_flap"),
        (12, "frustration_shake"),
        (18, "frustration_flap"),
        (23, "frustration_flap"),
    ] {
        assert_eq!(
            name(carl_overlay_pose(&a.carl, CarlOverlay::Frustration, age)),
            want,
            "age {age}"
        );
    }
}

// --- frame per state: Donut -------------------------------------------------

#[test]
fn donut_startle_puffs_then_settles_and_smug_holds() {
    let a = art();
    let names = frame_names(DONUT_CF02_MANIFEST, "cf02");
    for (age, want) in [
        (0, "startle_puff"),
        (5, "startle_puff"),
        (6, "startle_settle"),
        (11, "startle_settle"),
    ] {
        let p = donut_overlay_pose(&a.donut, DonutOverlay::Startle, age);
        assert_eq!(names[p.frame], want, "age {age}");
    }
    for age in [0, 19] {
        let p = donut_overlay_pose(&a.donut, DonutOverlay::Smug, age);
        assert_eq!(names[p.frame], "smug");
    }
}

// --- frame per state: goblins ------------------------------------------------

fn gob(
    mode: GoblinMode,
    alive: bool,
    reaction: Option<(GoblinReactionArt, u32)>,
) -> GoblinActInput {
    GoblinActInput {
        mode,
        alive,
        reaction,
        taunt_hold: None,
    }
}

fn gob_name(_a: &ActingArt, p: ArtPose) -> String {
    frame_names(GOBLIN_CF02_MANIFESTS[0], "cf02_goblin")[p.frame].clone()
}

#[test]
fn goblin_hesitate_panic_and_taunt_frames() {
    let a = art();
    let clips = &a.goblins[0];
    let pose = |input: &GoblinActInput, overlays| goblin_acting_pose(clips, input, overlays);
    let hes = gob(
        GoblinMode::Neutral,
        true,
        Some((GoblinReactionArt::Hesitate, 7)),
    );
    assert_eq!(gob_name(&a, pose(&hes, true).unwrap().pose), "hesitate");
    assert_eq!(pose(&hes, true).unwrap().flip, None, "it keeps its facing");
    // Panic: a two frame cycle, 6 ticks each, facing the flee direction.
    for (age, want) in [
        (0, "panic_flee"),
        (5, "panic_flee"),
        (6, "panic_flee_b"),
        (12, "panic_flee"),
        (78, "panic_flee_b"),
    ] {
        let panic = gob(
            GoblinMode::Neutral,
            true,
            Some((GoblinReactionArt::Panic { flee_dir: -1 }, age)),
        );
        let p = pose(&panic, true).unwrap();
        assert_eq!(gob_name(&a, p.pose), want, "age {age}");
        assert_eq!(p.flip, Some(true), "fleeing left: the sheet is mirrored");
    }
    let right = gob(
        GoblinMode::Neutral,
        true,
        Some((GoblinReactionArt::Panic { flee_dir: 1 }, 0)),
    );
    assert_eq!(pose(&right, true).unwrap().flip, Some(false));
    // Taunt: thump and beckon, 9 ticks each, over the 36 tick taunt.
    for (age, want) in [
        (0, "taunt_thump"),
        (8, "taunt_thump"),
        (9, "taunt_beckon"),
        (17, "taunt_beckon"),
        (18, "taunt_thump"),
        (27, "taunt_beckon"),
        (35, "taunt_beckon"),
    ] {
        let taunt = gob(
            GoblinMode::Neutral,
            true,
            Some((GoblinReactionArt::Taunt, age)),
        );
        assert_eq!(
            gob_name(&a, pose(&taunt, true).unwrap().pose),
            want,
            "age {age}"
        );
    }
}

#[test]
fn the_goblin_layers_follow_q69() {
    let hes = Some((GoblinReactionArt::Hesitate, 0));
    let taunt = Some((GoblinReactionArt::Taunt, 0));
    // death > hurt > move > morale reaction > overlay > locomotion.
    assert_eq!(
        goblin_layer(&gob(GoblinMode::Corpse, false, hes)),
        Layer::Death
    );
    assert_eq!(
        goblin_layer(&gob(GoblinMode::Down, false, taunt)),
        Layer::Death
    );
    for mode in [GoblinMode::Hurt, GoblinMode::Launched, GoblinMode::Downed] {
        assert_eq!(goblin_layer(&gob(mode, true, hes)), Layer::Hurt, "{mode:?}");
    }
    assert_eq!(
        goblin_layer(&gob(GoblinMode::Attack, true, hes)),
        Layer::Move
    );
    assert_eq!(
        goblin_layer(&gob(GoblinMode::Neutral, true, hes)),
        Layer::MoraleReaction
    );
    assert_eq!(
        goblin_layer(&gob(
            GoblinMode::Neutral,
            true,
            Some((GoblinReactionArt::Panic { flee_dir: 1 }, 3))
        )),
        Layer::MoraleReaction
    );
    assert_eq!(
        goblin_layer(&gob(GoblinMode::Neutral, true, taunt)),
        Layer::Locomotion
    );
    // The art follows: only a neutral goblin shows a reaction frame.
    let a = art();
    let clips = &a.goblins[1];
    for mode in [
        GoblinMode::Hurt,
        GoblinMode::Attack,
        GoblinMode::Launched,
        GoblinMode::Downed,
    ] {
        assert!(
            goblin_acting_pose(clips, &gob(mode, true, hes), true).is_none(),
            "{mode:?}"
        );
        assert!(
            goblin_acting_pose(clips, &gob(mode, true, taunt), true).is_none(),
            "{mode:?}"
        );
    }
    assert!(goblin_acting_pose(clips, &gob(GoblinMode::Corpse, false, hes), true).is_none());
    // The taunt is an overlay: switched off with `q69_acting_overlays`; the
    // morale reactions are not.
    assert!(goblin_acting_pose(clips, &gob(GoblinMode::Neutral, true, taunt), false).is_none());
    assert!(goblin_acting_pose(clips, &gob(GoblinMode::Neutral, true, hes), false).is_some());
}

#[test]
fn a_non_launching_hit_holds_the_taunt_frame() {
    let a = art();
    let clips = &a.goblins[0];
    let mut input = gob(GoblinMode::Hurt, true, None);
    input.taunt_hold = Some(1);
    let p = goblin_acting_pose(clips, &input, true).unwrap();
    assert_eq!(gob_name(&a, p.pose), "taunt_beckon");
    // The hold shows over a stagger, a stomped body, but never a flying one.
    for mode in [GoblinMode::Hurt, GoblinMode::Down, GoblinMode::Corpse] {
        let mut i = gob(mode, mode == GoblinMode::Hurt, None);
        i.taunt_hold = Some(0);
        assert!(goblin_acting_pose(clips, &i, true).is_some(), "{mode:?}");
    }
    for mode in [
        GoblinMode::Launched,
        GoblinMode::Airborne,
        GoblinMode::Downed,
    ] {
        let mut i = gob(mode, true, None);
        i.taunt_hold = Some(0);
        assert!(goblin_acting_pose(clips, &i, true).is_none(), "{mode:?}");
    }
    // With the overlays off there is no hold.
    assert!(goblin_acting_pose(clips, &input, false).is_none());
}

fn slam_name(_a: &ActingArt, p: ArtPose) -> String {
    frame_names(GOBLIN_SLAMMED_MANIFESTS[0], "cf02_goblin")[p.frame].clone()
}

fn slam(pose: SlamPose, mode: GoblinMode, dead_age: u32, landed_age: Option<u32>) -> SlamArtInput {
    SlamArtInput {
        pose,
        mode,
        dead_age,
        landed_age,
        impact_hold: u32::from(on().q21_slammed_impact_hold_ticks),
    }
}

#[test]
fn the_three_slammed_finishes_hold_their_impact_then_rest() {
    let a = art();
    let clips = &a.slammed[0];
    let name = |i: SlamArtInput| slammed_pose(clips, &i).map(|p| slam_name(&a, p));
    assert_eq!(on().q21_slammed_impact_hold_ticks, 8);
    // Stomped: the fold in place for 8 ticks, then the dead bug.
    assert_eq!(
        name(slam(SlamPose::Stomped, GoblinMode::Down, 0, Some(0))).as_deref(),
        Some("stomped_impact")
    );
    assert_eq!(
        name(slam(SlamPose::Stomped, GoblinMode::Down, 7, Some(7))).as_deref(),
        Some("stomped_impact")
    );
    assert_eq!(
        name(slam(SlamPose::Stomped, GoblinMode::Down, 8, Some(8))).as_deref(),
        Some("stomped_rest")
    );
    assert_eq!(
        name(slam(SlamPose::Stomped, GoblinMode::Corpse, 300, Some(300))).as_deref(),
        Some("stomped_rest")
    );
    // Wall crumple: the squash while falling and for the hold, then the slump.
    assert_eq!(
        name(slam(SlamPose::WallCrumple, GoblinMode::Airborne, 0, None)).as_deref(),
        Some("wall_impact")
    );
    assert_eq!(
        name(slam(SlamPose::WallCrumple, GoblinMode::Airborne, 30, None)).as_deref(),
        Some("wall_impact"),
        "never a rest frame in the air"
    );
    assert_eq!(
        name(slam(SlamPose::WallCrumple, GoblinMode::Down, 3, Some(0))).as_deref(),
        Some("wall_impact")
    );
    assert_eq!(
        name(slam(SlamPose::WallCrumple, GoblinMode::Down, 8, Some(5))).as_deref(),
        Some("wall_rest")
    );
    // Flung: the GOB-01 knockback flight, then the face-plant, then the heap.
    assert_eq!(
        name(slam(SlamPose::Flung, GoblinMode::Airborne, 12, None)),
        None
    );
    assert_eq!(
        name(slam(SlamPose::Flung, GoblinMode::Down, 13, Some(0))).as_deref(),
        Some("flung_impact")
    );
    assert_eq!(
        name(slam(SlamPose::Flung, GoblinMode::Down, 20, Some(7))).as_deref(),
        Some("flung_impact")
    );
    assert_eq!(
        name(slam(SlamPose::Flung, GoblinMode::Down, 21, Some(8))).as_deref(),
        Some("flung_rest")
    );
    assert_eq!(
        name(slam(SlamPose::Flung, GoblinMode::Corpse, 90, Some(77))).as_deref(),
        Some("flung_rest")
    );
}

#[test]
fn the_wall_crumple_contact_column_lands_on_the_wall_face() {
    let a = art();
    for clips in &a.slammed {
        // The sim stops the body with its back edge on the wall face; the sprite's
        // x16 is the same 12 px behind the pivot, so nothing needs shifting.
        assert_eq!(wall_contact_offset_px(clips), 0);
    }
}

// --- the tracker over real runs -------------------------------------------------

/// Every `cf02_*` fixture of every registry (the CF02 one, Donut's and the
/// COMPANION ones), so a registry that grows later is covered without an edit.
fn all_cf02() -> Vec<&'static crate::party_scene::PartyFixture> {
    list_fixtures("cf02_")
        .into_iter()
        .filter_map(find_party_fixture)
        .collect()
}

fn kit() -> EnvKit {
    EnvKit::load(root(), "assets/levels/floor01/corridor_abc_sample.json").unwrap()
}

fn scene_animators(sim: &PartyFixtureSim, tuning: Option<&'static Cf02Tuning>) -> SceneAnimators {
    let carl = load_carl(root(), FrameSource::GridSource).unwrap();
    let donut = load_donut(root(), FrameSource::GridSource).unwrap();
    let mut anim = SceneAnimators::new(
        Animator::new(carl.clips, carl.pacing),
        Animator::new(donut.clips, donut.pacing),
        &sim.scene.world,
    );
    let combat_art = super::super::CombatAnimators::new(
        SpriteClips::load_carl_melee(root(), FrameSource::GridSource).unwrap(),
        [
            SpriteClips::load_goblin(root(), 0, FrameSource::GridSource).unwrap(),
            SpriteClips::load_goblin(root(), 1, FrameSource::GridSource).unwrap(),
        ],
        &sim.combat,
    );
    anim.art = Some(match tuning {
        Some(t) => combat_art.with_acting(art(), t, &sim.combat),
        None => combat_art,
    });
    anim
}

/// What the CF02 acting layer drew on one tick of a run.
#[derive(Debug, Clone, PartialEq)]
struct Drawn {
    carl: Option<String>,
    donut: Option<String>,
    goblins: Vec<Option<String>>,
}

struct Run {
    hash: u64,
    events: Vec<CombatEvent>,
    hashes: Vec<u64>,
    drawn: Vec<Drawn>,
}

fn run_fixture(
    kit: &EnvKit,
    fixture: &'static crate::party_scene::PartyFixture,
    tuning: Option<&'static Cf02Tuning>,
) -> Run {
    let mut sim = PartyFixtureSim::new(kit, 360, fixture).unwrap();
    let mut anim = scene_animators(&sim, tuning);
    let mut hashes = Vec::new();
    let mut drawn = Vec::new();
    for _ in 0..fixture.base.ticks {
        let before = sim.scene.world.tick;
        sim.step_scripted();
        if sim.scene.world.tick == before + 1 {
            anim.step(&sim.scene.world);
            anim.step_art(&sim.scene, &sim.combat);
        } else {
            anim.reset(&sim.scene.world);
            anim.reset_art(&sim.combat);
        }
        hashes.push(sim.state_hash());
        let art = anim.art.as_ref().unwrap();
        let label = |p: Option<ArtPose>| p.map(|p| p.label());
        drawn.push(Drawn {
            carl: label(art.carl_cf02_pose()),
            donut: label(art.donut_cf02_pose()),
            goblins: (0..art.goblin_count())
                .map(|i| art.goblin_cf02_pose(i).map(|p| p.pose.label()))
                .collect(),
        });
    }
    Run {
        hash: sim.state_hash(),
        events: sim.combat.events.clone(),
        hashes,
        drawn,
    }
}

/// The parity proof: with the overlays on, with them off and with no acting art
/// at all, every `cf02_*` fixture produces the identical event log, the identical
/// final hash and the identical hash on every tick. The acting layer reads the
/// finished state and nothing flows back.
#[test]
fn acting_overlays_on_and_off_change_no_event_hash_or_input_timing() {
    let kit = kit();
    let off = overlays_off();
    let mut checked = 0;
    for fixture in all_cf02() {
        let with = run_fixture(&kit, fixture, Some(on()));
        let without = run_fixture(&kit, fixture, Some(off));
        let plain = run_fixture(&kit, fixture, None);
        for (name, other) in [("overlays off", &without), ("no acting art", &plain)] {
            assert_eq!(
                with.events,
                other.events,
                "{} {name}: events",
                fixture.name()
            );
            assert_eq!(
                with.hash,
                other.hash,
                "{} {name}: final hash",
                fixture.name()
            );
            assert_eq!(
                with.hashes,
                other.hashes,
                "{} {name}: per tick hashes",
                fixture.name()
            );
        }
        checked += 1;
    }
    assert_eq!(
        checked,
        list_fixtures("cf02_").len(),
        "every named cf02 fixture is a party fixture"
    );
    assert!(checked >= 47);
}

/// No simulation module reads an acting knob (the overlay switch, flinch,
/// frustration, startle, smug or taunt hold): they are presentation inputs.
#[test]
fn no_simulation_module_reads_an_acting_knob() {
    for file in [
        "src/combat.rs",
        "src/combat_scene.rs",
        "src/combat_scene/carl_cf02.rs",
        "src/combat_scene/morale_cf02.rs",
        "src/combat_scene/donut.rs",
        "src/combat_scene/contacts_cf02.rs",
        "src/goblin.rs",
        "src/llama.rs",
        "src/morale.rs",
        "src/input_buffer.rs",
        "src/corridor.rs",
        "src/prototype.rs",
        "src/feel.rs",
        "src/party.rs",
        "src/party_scene.rs",
        "src/shore.rs",
    ] {
        let Ok(text) = std::fs::read_to_string(root().join(file)) else {
            continue;
        };
        for knob in [
            "q69_",
            "q70_",
            "q71_",
            "q72_",
            "acting_overlays",
            "crate::acting",
            "combat_anim",
        ] {
            assert!(!text.contains(knob), "{file} reads {knob}");
        }
    }
}

fn collect(kit: &EnvKit) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    for fixture in all_cf02() {
        let run = run_fixture(kit, fixture, Some(on()));
        for d in &run.drawn {
            if let Some(c) = &d.carl {
                seen.insert(format!("carl:{c}"));
            }
            if let Some(c) = &d.donut {
                seen.insert(format!("donut:{c}"));
            }
            for g in d.goblins.iter().flatten() {
                seen.insert(format!("goblin:{g}"));
            }
        }
    }
    seen
}

/// Every CF02 move reaches the screen in a real run: the stomp, the air kick, the
/// dive and the bail-out frames are each drawn by at least one `cf02_*` fixture.
#[test]
fn real_runs_draw_every_cf02_move_frame() {
    let seen = collect(&kit());
    for tag in ["stomp", "air_kick", "dive_tuck", "dive_plunge", "bailout"] {
        assert!(
            seen.iter().any(|s| s.starts_with(&format!("carl:{tag}:"))),
            "no run drew {tag}: {seen:?}"
        );
    }
    for tag in ["hesitate", "panic"] {
        assert!(
            seen.iter()
                .any(|s| s.starts_with(&format!("goblin:{tag}:"))),
            "no run drew {tag}: {seen:?}"
        );
    }
    println!("{seen:#?}");
}

#[test]
fn legacy_fixtures_draw_no_acting_frame_and_keep_their_state_line() {
    // Without acting art the state line has no acting suffix: every committed
    // legacy line is byte-identical.
    let kit = kit();
    let fixture = all_cf02()[0];
    let sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
    let anim = scene_animators(&sim, None);
    let line = anim.art.as_ref().unwrap().state_line(0);
    assert!(!line.contains("acting_"), "{line}");
    let with = scene_animators(&sim, Some(on()));
    assert!(
        with.art
            .as_ref()
            .unwrap()
            .state_line(0)
            .contains(" acting_carl=-")
    );
}

#[test]
fn spec_move_totals_cover_the_art() {
    let a = art();
    assert_eq!(
        a.carl.tag_ticks("stomp").unwrap(),
        u32::from(cf02_moves().stomp.total())
    );
    assert_eq!(
        a.carl.tag_ticks("air_kick").unwrap() + 6,
        u32::from(cf02_moves().air_kick.total()),
        "the 6 recovery ticks show the locomotion fall frame"
    );
    let _ = carl_move(MoveId::Jab);
}

// --- the tracker on built states -------------------------------------------------

fn find(name: &str) -> &'static crate::party_scene::PartyFixture {
    crate::cf02_fixtures::find_party_fixture(name).unwrap()
}

/// A quiet run state to bend: the fixture's first tick, no input.
fn rig(name: &str) -> (PartyFixtureSim, ActingAnimators) {
    let mut sim = PartyFixtureSim::new(&kit(), 360, find(name)).unwrap();
    sim.step_scripted();
    let acting = ActingAnimators::new(art(), on(), &sim.combat);
    (sim, acting)
}

fn push(sim: &mut PartyFixtureSim, event: CombatEvent) {
    sim.combat.events.push(event);
}

fn quiet_step(sim: &mut PartyFixtureSim, acting: &mut ActingAnimators, carl: Tag, donut: Tag) {
    sim.scene.world.tick += 1;
    acting.step(
        &sim.scene,
        &sim.combat,
        &CarlInput::default(),
        false,
        carl,
        donut,
    );
}

fn carl_label(acting: &ActingAnimators) -> Option<String> {
    acting.carl_pose().map(|p| carl_name(&art(), p))
}

#[test]
fn a_near_miss_flinches_carl_for_eight_ticks_in_idle_only() {
    use crate::feel::{IMPACT_END, SMEAR_END};
    let near = |gap: f32, loco: Tag, hit: bool| {
        let (mut sim, mut acting) = rig("cf02_stomp_standing");
        let hurtbox = CombatState::hurtbox(&sim.scene, HeroId::Carl);
        let g = &mut sim.combat.enemies[0];
        g.facing = 1;
        g.mode = GoblinMode::Attack;
        g.attack_tick = SMEAR_END;
        g.swing_hit = false;
        g.x = hurtbox.left - crate::feel::SWING_X_MAX - gap;
        assert_eq!(g.attack_phase(), Some(crate::goblin::AttackPhase::Impact));
        let measured = rect_gap_px(&g.swing_box(), &hurtbox);
        assert!((measured - gap).abs() < 0.01, "{measured} vs {gap}");
        quiet_step(&mut sim, &mut acting, loco, Tag::Idle);
        assert_eq!(carl_label(&acting), None, "judged when the swing ends");
        // The swing ends (its recovery begins); the overlay runs on its own clock.
        let g = &mut sim.combat.enemies[0];
        g.attack_tick = IMPACT_END;
        g.swing_hit = hit;
        assert_eq!(g.attack_phase(), Some(crate::goblin::AttackPhase::Recover));
        let mut shown = 0;
        for _ in 0..14 {
            quiet_step(&mut sim, &mut acting, loco, Tag::Idle);
            shown += u32::from(carl_label(&acting).as_deref() == Some("flinch"));
        }
        shown
    };
    assert_eq!(
        near(8.0, Tag::Idle, false),
        8,
        "idle, 8 px away: eight ticks"
    );
    assert_eq!(near(12.0, Tag::Idle, false), 8, "exactly 12 px counts");
    assert_eq!(near(20.0, Tag::Idle, false), 0, "too far");
    assert_eq!(
        near(8.0, Tag::Idle, true),
        0,
        "a swing that hurt is not a miss"
    );
    assert_eq!(
        near(8.0, Tag::Run, false),
        0,
        "the planted feet never skate"
    );
    assert_eq!(near(8.0, Tag::JumpRise, false), 0, "not in the air");
}

#[test]
fn a_swing_that_overlaps_carl_is_a_hit_not_a_near_miss() {
    use crate::feel::{IMPACT_END, SMEAR_END};
    let (mut sim, mut acting) = rig("cf02_stomp_standing");
    let hurtbox = CombatState::hurtbox(&sim.scene, HeroId::Carl);
    let g = &mut sim.combat.enemies[0];
    g.facing = 1;
    g.mode = GoblinMode::Attack;
    g.attack_tick = SMEAR_END;
    g.x = hurtbox.left - crate::feel::SWING_X_MAX + 6.0;
    assert_eq!(rect_gap_px(&g.swing_box(), &hurtbox), 0.0);
    quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
    sim.combat.enemies[0].attack_tick = IMPACT_END;
    for _ in 0..12 {
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
        assert_eq!(carl_label(&acting), None);
    }
}

#[test]
fn a_glob_that_passes_close_flinches_carl_and_one_that_hits_does_not() {
    let glob_at = |sim: &PartyFixtureSim, dx: f32| {
        let hurtbox = CombatState::hurtbox(&sim.scene, HeroId::Carl);
        let (cx, cy) = hurtbox.center();
        crate::llama::Glob {
            x: cx + dx,
            y: cy,
            dir: -1,
            travelled: 0.0,
            owner: 0,
        }
    };
    let run = |hit: bool| {
        let (mut sim, mut acting) = rig("cf02_stomp_standing");
        sim.combat.enemies.clear();
        let hurtbox = CombatState::hurtbox(&sim.scene, HeroId::Carl);
        // 8 px clear of Carl's hurtbox on the right, flying left.
        let close = hurtbox.width * 0.5 + 8.0 + crate::feel::LLAMA_GLOB_HALF_W;
        sim.combat.globs = vec![glob_at(&sim, close)];
        let gap = rect_gap_px(&sim.combat.globs[0].rect(), &hurtbox);
        assert!(gap > 0.0 && gap <= 12.0, "{gap}");
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
        assert_eq!(carl_label(&acting), None, "judged when it has passed");
        if hit {
            let t = sim.scene.world.tick;
            sim.combat.globs.clear();
            push(
                &mut sim,
                CombatEvent::HeroHurt {
                    tick: t,
                    source: 0,
                    health_left: 3,
                    hero: HeroId::Carl,
                },
            );
        } else {
            // It flew on, 40 px clear on the other side.
            sim.combat.globs = vec![glob_at(&sim, -(hurtbox.width * 0.5 + 40.0))];
        }
        let mut shown = 0;
        for _ in 0..12 {
            quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
            shown += u32::from(carl_label(&acting).as_deref() == Some("flinch"));
        }
        shown
    };
    assert_eq!(run(false), 8);
    assert_eq!(run(true), 0);
}

fn whiff(sim: &mut PartyFixtureSim, instance: u32) {
    push(
        sim,
        CombatEvent::MoveStarted {
            tick: sim.scene.world.tick,
            move_id: MoveId::Jab,
            instance,
        },
    );
}

#[test]
fn three_whiffs_in_the_window_frustrate_carl_once_per_cooldown() {
    let (mut sim, mut acting) = rig("cf02_stomp_standing");
    sim.combat.enemies.clear();
    let shown = |sim: &mut PartyFixtureSim, acting: &mut ActingAnimators, ticks: u32| {
        (0..ticks)
            .filter_map(|_| {
                quiet_step(sim, acting, Tag::Idle, Tag::Idle);
                carl_label(acting)
            })
            .collect::<Vec<_>>()
    };
    // Two whiffs are not enough; the third ends the run of moves.
    for instance in 1..=2 {
        whiff(&mut sim, instance);
        shown(&mut sim, &mut acting, 20);
    }
    assert!(shown(&mut sim, &mut acting, 5).is_empty());
    whiff(&mut sim, 3);
    let run = shown(&mut sim, &mut acting, 30);
    assert_eq!(run.len(), 24, "{run:?}");
    let pattern: Vec<&str> = run.iter().map(String::as_str).collect();
    assert_eq!(pattern[0], "frustration_shake");
    assert_eq!(pattern[6], "frustration_flap");
    assert_eq!(pattern[12], "frustration_shake");
    assert_eq!(pattern[23], "frustration_flap");
    // The cooldown (600 ticks) swallows the next three whiffs.
    for instance in 4..=6 {
        whiff(&mut sim, instance);
        shown(&mut sim, &mut acting, 10);
    }
    assert!(
        shown(&mut sim, &mut acting, 100).is_empty(),
        "still cooling down"
    );
    for instance in 7..=9 {
        whiff(&mut sim, instance);
        shown(&mut sim, &mut acting, 10);
    }
    assert!(
        shown(&mut sim, &mut acting, 600).is_empty(),
        "the window was spent"
    );
    // After the cooldown a new run of three frustrates him again.
    let mut again = 0;
    for instance in 10..=12 {
        whiff(&mut sim, instance);
        again += shown(&mut sim, &mut acting, 10).len();
    }
    again += shown(&mut sim, &mut acting, 30).len();
    assert_eq!(again, 24);
}

#[test]
fn a_hit_resets_the_run_of_whiffs_and_a_slow_run_is_not_frustrating() {
    let (mut sim, mut acting) = rig("cf02_stomp_standing");
    sim.combat.enemies.clear();
    let hit = |sim: &mut PartyFixtureSim, instance: u32| {
        push(
            sim,
            CombatEvent::Hit {
                tick: sim.scene.world.tick,
                move_id: MoveId::Jab,
                instance,
                target: 0,
                damage: 1,
                health_left: 1,
                hit_stop: 4,
            },
        );
    };
    let any_overlay = |sim: &mut PartyFixtureSim, acting: &mut ActingAnimators, ticks: u32| {
        (0..ticks).any(|_| {
            quiet_step(sim, acting, Tag::Idle, Tag::Idle);
            acting.carl_pose().is_some()
        })
    };
    whiff(&mut sim, 1);
    any_overlay(&mut sim, &mut acting, 10);
    whiff(&mut sim, 2);
    any_overlay(&mut sim, &mut acting, 10);
    whiff(&mut sim, 3);
    hit(&mut sim, 3);
    assert!(
        !any_overlay(&mut sim, &mut acting, 40),
        "move 3 hit: no run of three"
    );
    // Three whiffs spread over more than 150 ticks do not count.
    for instance in 4..=6 {
        whiff(&mut sim, instance);
        any_overlay(&mut sim, &mut acting, 100);
    }
    assert!(!any_overlay(&mut sim, &mut acting, 60));
}

#[test]
fn donut_startles_near_a_crash_and_is_smug_when_her_hit_lands() {
    let tick = |sim: &PartyFixtureSim| sim.scene.world.tick;
    let (mut sim, mut acting) = rig("cf02_stomp_standing");
    sim.combat.enemies.clear();
    let donut_x = sim.scene.world.donut.x;
    let names = frame_names(DONUT_CF02_MANIFEST, "cf02");
    let name = |acting: &ActingAnimators| acting.donut_pose().map(|p| names[p.frame].clone());
    // A wall slam 100 px away: startle, 12 ticks (the puff then the settle).
    sim.combat.enemies.push(crate::goblin::Goblin::new(
        donut_x + 100.0,
        0.0,
        crate::combat::EnemyAi::Chase,
    ));
    let t = tick(&sim);
    push(
        &mut sim,
        CombatEvent::WallSlam {
            tick: t,
            launch: 1,
            goblin: 0,
            instigator: Some(0),
        },
    );
    let mut seen = Vec::new();
    for _ in 0..20 {
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
        seen.push(name(&acting));
    }
    let shown: Vec<&str> = seen.iter().flatten().map(String::as_str).collect();
    assert_eq!(shown.len(), 12, "{seen:?}");
    assert_eq!(shown[0], "startle_puff");
    assert_eq!(shown[5], "startle_puff");
    assert_eq!(shown[6], "startle_settle");
    assert_eq!(shown[11], "startle_settle");
    // 200 px away is too far; the cooldown (180) blocks a second one.
    sim.combat.enemies[0].x = donut_x + 200.0;
    let t = tick(&sim);
    push(
        &mut sim,
        CombatEvent::WallSlam {
            tick: t,
            launch: 2,
            goblin: 0,
            instigator: Some(0),
        },
    );
    quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
    assert_eq!(name(&acting), None, "outside 160 px");
    sim.combat.enemies[0].x = donut_x + 100.0;
    let t = tick(&sim);
    push(
        &mut sim,
        CombatEvent::WallSlam {
            tick: t,
            launch: 3,
            goblin: 0,
            instigator: Some(0),
        },
    );
    quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
    assert_eq!(name(&acting), None, "cooldown");
    for _ in 0..200 {
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
    }
    let t = tick(&sim);
    push(
        &mut sim,
        CombatEvent::WallSlam {
            tick: t,
            launch: 4,
            goblin: 0,
            instigator: Some(0),
        },
    );
    quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Run);
    assert_eq!(name(&acting), None, "never while she runs");
    // Smug: her own hit, 20 ticks of one frame, idle only.
    for _ in 0..200 {
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
    }
    let t = tick(&sim);
    push(
        &mut sim,
        CombatEvent::Hit {
            tick: t,
            move_id: MoveId::Swat,
            instance: 1,
            target: 0,
            damage: 0,
            health_left: 3,
            hit_stop: 0,
        },
    );
    let mut smug = 0;
    for _ in 0..30 {
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
        smug += u32::from(name(&acting).as_deref() == Some("smug"));
    }
    assert_eq!(smug, 20);
    // Carl's own hit does not make her smug.
    let (mut sim, mut acting) = rig("cf02_stomp_standing");
    sim.combat.enemies.clear();
    let t = tick(&sim);
    push(
        &mut sim,
        CombatEvent::Hit {
            tick: t,
            move_id: MoveId::Jab,
            instance: 1,
            target: 0,
            damage: 1,
            health_left: 2,
            hit_stop: 4,
        },
    );
    quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
    assert_eq!(acting.donut_pose(), None);
}

#[test]
fn overlays_off_shows_no_overlay_but_keeps_the_moves_and_reactions() {
    let (mut sim, _) = rig("cf02_stomp_standing");
    let mut acting = ActingAnimators::new(art(), overlays_off(), &sim.combat);
    assert!(!acting.overlays_on());
    let t = sim.scene.world.tick;
    push(
        &mut sim,
        CombatEvent::HeroHurt {
            tick: t,
            source: 0,
            health_left: 3,
            hero: HeroId::Carl,
        },
    );
    push(
        &mut sim,
        CombatEvent::Hit {
            tick: t,
            move_id: MoveId::Swat,
            instance: 1,
            target: 0,
            damage: 0,
            health_left: 3,
            hit_stop: 0,
        },
    );
    for _ in 0..10 {
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
        assert_eq!(acting.donut_pose(), None);
        assert_eq!(acting.carl_pose(), None);
    }
}

// --- taunt hold and slammed finishes on real runs --------------------------------

#[test]
fn a_punished_taunt_holds_its_frame_for_six_ticks_over_the_hit() {
    let kit = kit();
    let fixture = find("cf02_showoff_taunt_punish");
    let mut sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
    let mut anim = scene_animators(&sim, Some(on()));
    let mut taunting = false;
    let mut held = 0;
    let mut during_hold: Vec<GoblinMode> = Vec::new();
    for _ in 0..fixture.base.ticks {
        sim.step_scripted();
        anim.step(&sim.scene.world);
        anim.step_art(&sim.scene, &sim.combat);
        let art = anim.art.as_ref().unwrap();
        for (i, g) in sim
            .combat
            .enemies
            .iter()
            .enumerate()
            .take(art.goblin_count())
        {
            if let Some(p) = art.goblin_cf02_pose(i) {
                if p.pose.tag == "taunt" && g.mode == GoblinMode::Neutral {
                    taunting = true;
                }
                if p.pose.tag == "taunt" && matches!(g.mode, GoblinMode::Hurt | GoblinMode::Down) {
                    held += 1;
                    during_hold.push(g.mode);
                }
            }
        }
    }
    assert!(taunting, "the show-off taunts");
    assert_eq!(
        held,
        usize::from(on().q72_taunt_hold_ticks),
        "{during_hold:?}"
    );
}

#[test]
fn the_three_slammed_finishes_play_in_real_runs() {
    use crate::combat::ForceTier;
    let kit = kit();
    let mut by_kind: std::collections::BTreeMap<&str, Vec<String>> = Default::default();
    for fixture in all_cf02().into_iter().filter(|f| f.base.boss.is_none()) {
        let mut sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
        let mut anim = scene_animators(&sim, Some(on()));
        let mut slam: Vec<Option<SlamPose>> = vec![None; 16];
        let mut seq: Vec<Vec<String>> = vec![Vec::new(); 16];
        for _ in 0..fixture.base.ticks {
            let before = sim.combat.events.len();
            let restarts = sim.combat.restarts;
            sim.step_scripted();
            if sim.combat.restarts != restarts {
                slam = vec![None; 16];
            }
            let fresh: Vec<CombatEvent> =
                sim.combat.events[before.min(sim.combat.events.len())..].to_vec();
            for e in &fresh {
                if let CombatEvent::EnemyDefeated { target, force, .. } = *e
                    && force >= ForceTier::Heavy
                    && (target as usize) < slam.len()
                {
                    slam[target as usize] = Some(crate::death::slam_pose_for(&fresh, target));
                }
            }
            anim.step(&sim.scene.world);
            anim.step_art(&sim.scene, &sim.combat);
            let art = anim.art.as_ref().unwrap();
            for (i, (kind, seq_i)) in slam
                .iter()
                .zip(seq.iter_mut())
                .enumerate()
                .take(art.goblin_count().min(16))
            {
                if let Some(p) = art.goblin_slammed_pose(i, *kind) {
                    let names = frame_names(
                        GOBLIN_SLAMMED_MANIFESTS[crate::combat_anim::goblin_variant(i)],
                        "cf02_goblin",
                    );
                    seq_i.push(names[p.frame].clone());
                }
            }
        }
        for (i, mut s) in seq.into_iter().enumerate() {
            s.dedup();
            if let Some(kind) = slam[i] {
                by_kind.entry(kind.name()).or_insert(s);
            }
        }
    }
    let seq = |k: &str| by_kind[k].iter().map(String::as_str).collect::<Vec<_>>();
    assert_eq!(seq("stomped"), ["stomped_impact", "stomped_rest"]);
    assert_eq!(seq("wall_crumple"), ["wall_impact", "wall_rest"]);
    assert_eq!(seq("flung"), ["flung_impact", "flung_rest"]);
}

/// Not a check: lists, per CF02 fixture, the ticks each acting frame is drawn
/// (`cargo test --lib list_acting_frames_per_fixture -- --ignored --nocapture`).
#[test]
#[ignore]
fn list_acting_frames_per_fixture() {
    let kit = kit();
    for fixture in all_cf02() {
        let run = run_fixture(&kit, fixture, Some(on()));
        let mut spans: Vec<(String, usize, usize)> = Vec::new();
        for (t, d) in run.drawn.iter().enumerate() {
            let mut now: Vec<String> = Vec::new();
            if let Some(c) = &d.carl {
                now.push(format!("carl:{c}"));
            }
            if let Some(c) = &d.donut {
                now.push(format!("donut:{c}"));
            }
            for (i, g) in d.goblins.iter().enumerate() {
                if let Some(g) = g {
                    now.push(format!("g{i}:{g}"));
                }
            }
            for n in now {
                match spans
                    .iter_mut()
                    .find(|(k, _, last)| *k == n && *last + 1 == t)
                {
                    Some(s) => s.2 = t,
                    None => spans.push((n, t, t)),
                }
            }
        }
        if !spans.is_empty() {
            println!("{}:", fixture.name());
            for (n, a, b) in spans {
                println!("  {n} ticks {}..{}", a + 1, b + 1);
            }
        }
    }
}

#[test]
fn a_dive_whiff_draws_the_heavy_landing_for_exactly_its_ten_recovery_ticks() {
    let kit = kit();
    let run = run_fixture(&kit, find("cf02_dive_whiff_recovery"), Some(on()));
    let land: Vec<usize> = run
        .drawn
        .iter()
        .enumerate()
        .filter(|(_, d)| d.carl.as_deref() == Some("dive_land:7"))
        .map(|(t, _)| t)
        .collect();
    assert_eq!(
        land.len(),
        usize::from(on().q56_landing_dive_whiff),
        "{land:?}"
    );
    assert_eq!(
        land.last().unwrap() - land[0],
        land.len() - 1,
        "consecutive"
    );
    // A dive that hits bounces: it has no landing frame (Q56).
    for name in [
        "cf02_dive_standing_knockdown",
        "cf02_dive_bounce_kick",
        "cf02_dive_lying",
    ] {
        let run = run_fixture(&kit, find(name), Some(on()));
        assert!(
            run.drawn
                .iter()
                .all(|d| d.carl.as_deref() != Some("dive_land:7")),
            "{name}"
        );
    }
}

#[test]
fn a_hurt_carl_loses_the_overlay_and_it_does_not_come_back() {
    let (mut sim, mut acting) = rig("cf02_stomp_standing");
    sim.combat.enemies.clear();
    let t = sim.scene.world.tick;
    for instance in 1..=3 {
        whiff(&mut sim, instance);
        let _ = t;
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
    }
    // Frustration is running; Carl is hurt: the hurt frame belongs to the legacy
    // selection and the overlay is over, not paused.
    assert!(carl_label(&acting).is_some());
    let hurt = CarlInput {
        hurt_ticks: 10,
        ..CarlInput::default()
    };
    sim.scene.world.tick += 1;
    acting.step(&sim.scene, &sim.combat, &hurt, true, Tag::Idle, Tag::Idle);
    assert_eq!(acting.carl_pose(), None);
    assert_eq!(acting.carl_layer(), Layer::Hurt);
    for _ in 0..30 {
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
        assert_eq!(acting.carl_pose(), None, "it does not resume");
    }
    // A run (not idle) ends it as well.
    let (mut sim, mut acting) = rig("cf02_stomp_standing");
    sim.combat.enemies.clear();
    for instance in 1..=3 {
        whiff(&mut sim, instance);
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
    }
    assert!(carl_label(&acting).is_some());
    quiet_step(&mut sim, &mut acting, Tag::Run, Tag::Idle);
    assert_eq!(acting.carl_pose(), None);
    quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
    assert_eq!(acting.carl_pose(), None);
}

/// Not a check: where each slammed finish plays
/// (`cargo test --lib list_slammed_finishes -- --ignored --nocapture`).
#[test]
#[ignore]
fn list_slammed_finishes() {
    use crate::combat::ForceTier;
    let kit = kit();
    for fixture in all_cf02() {
        let mut sim = PartyFixtureSim::new(&kit, 360, fixture).unwrap();
        let mut anim = scene_animators(&sim, Some(on()));
        let mut slam: Vec<Option<SlamPose>> = vec![None; 16];
        let mut spans: Vec<(usize, String, u64, u64)> = Vec::new();
        for _ in 0..fixture.base.ticks {
            let before = sim.combat.events.len();
            sim.step_scripted();
            let fresh: Vec<CombatEvent> =
                sim.combat.events[before.min(sim.combat.events.len())..].to_vec();
            for e in &fresh {
                if let CombatEvent::EnemyDefeated { target, force, .. } = *e
                    && force >= ForceTier::Heavy
                    && (target as usize) < 16
                {
                    slam[target as usize] = Some(crate::death::slam_pose_for(&fresh, target));
                }
            }
            anim.step(&sim.scene.world);
            anim.step_art(&sim.scene, &sim.combat);
            let art = anim.art.as_ref().unwrap();
            let t = sim.scene.world.tick;
            for (i, kind) in slam.iter().enumerate().take(art.goblin_count().min(16)) {
                if let Some(p) = art.goblin_slammed_pose(i, *kind) {
                    let key = p.label();
                    match spans
                        .iter_mut()
                        .find(|(g, k, _, last)| *g == i && *k == key && *last + 1 == t)
                    {
                        Some(s) => s.3 = t,
                        None => spans.push((i, key, t, t)),
                    }
                }
            }
        }
        for (i, k, a, b) in spans {
            println!("{} g{i} {k} {a}..{b}", fixture.name());
        }
    }
}

#[test]
fn an_overlay_waits_for_the_actor_to_be_idle_and_never_spends_a_cooldown_unseen() {
    let names = frame_names(DONUT_CF02_MANIFEST, "cf02");
    let donut = |acting: &ActingAnimators| acting.donut_pose().map(|p| names[p.frame].clone());
    // Smug: her hit lands inside her own move; it shows once she is idle.
    let (mut sim, mut acting) = rig("cf02_stomp_standing");
    sim.combat.enemies.clear();
    let t = sim.scene.world.tick;
    push(
        &mut sim,
        CombatEvent::Hit {
            tick: t,
            move_id: MoveId::Pounce,
            instance: 1,
            target: 0,
            damage: 1,
            health_left: 1,
            hit_stop: 0,
        },
    );
    for _ in 0..8 {
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Run);
        assert_eq!(donut(&acting), None, "she is running");
    }
    let mut smug = 0;
    for _ in 0..30 {
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
        smug += u32::from(donut(&acting).as_deref() == Some("smug"));
    }
    assert_eq!(smug, 20);
    // It is forgotten when she never stands still for 90 ticks.
    let (mut sim, mut acting) = rig("cf02_stomp_standing");
    sim.combat.enemies.clear();
    let t = sim.scene.world.tick;
    push(
        &mut sim,
        CombatEvent::Hit {
            tick: t,
            move_id: MoveId::Swat,
            instance: 1,
            target: 0,
            damage: 0,
            health_left: 3,
            hit_stop: 0,
        },
    );
    for _ in 0..100 {
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Run);
    }
    for _ in 0..10 {
        quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
        assert_eq!(donut(&acting), None, "too late");
    }
    // Startle: not shown while she runs, and that does not spend the cooldown.
    let (mut sim, mut acting) = rig("cf02_stomp_standing");
    sim.combat.enemies.clear();
    let donut_x = sim.scene.world.donut.x;
    sim.combat.enemies.push(crate::goblin::Goblin::new(
        donut_x + 50.0,
        0.0,
        crate::combat::EnemyAi::Chase,
    ));
    let t = sim.scene.world.tick;
    push(
        &mut sim,
        CombatEvent::WallSlam {
            tick: t,
            launch: 1,
            goblin: 0,
            instigator: Some(0),
        },
    );
    quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Run);
    assert_eq!(donut(&acting), None);
    let t = sim.scene.world.tick;
    push(
        &mut sim,
        CombatEvent::WallSlam {
            tick: t,
            launch: 2,
            goblin: 0,
            instigator: Some(0),
        },
    );
    quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
    assert_eq!(donut(&acting).as_deref(), Some("startle_puff"));
}

#[test]
fn frustration_waits_for_idle_inside_its_window() {
    let run = |idle_after: u32| {
        let (mut sim, mut acting) = rig("cf02_stomp_standing");
        sim.combat.enemies.clear();
        for instance in 1..=3 {
            whiff(&mut sim, instance);
            quiet_step(&mut sim, &mut acting, Tag::Run, Tag::Idle);
        }
        for _ in 0..idle_after {
            quiet_step(&mut sim, &mut acting, Tag::Run, Tag::Idle);
            assert_eq!(acting.carl_pose(), None, "running: no overlay");
        }
        (0..30)
            .filter(|_| {
                quiet_step(&mut sim, &mut acting, Tag::Idle, Tag::Idle);
                acting.carl_pose().is_some()
            })
            .count()
    };
    assert_eq!(run(20), 24, "idle again inside the window: shown, once");
    assert_eq!(run(200), 0, "the window has passed");
}
