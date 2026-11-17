//! Locomotion animation (ANIM-INT-01): a deterministic, presentation-only state
//! machine plus the clip data it plays. Pure Rust, no Bevy types.
//!
//! The simulation never reads anything from here. Each fixed 60 Hz tick the
//! caller derives an [`AnimInput`] (grounded, vx, vy) from the finished
//! simulation state and calls [`Animator::step`]; the adapter then maps
//! [`Animator::atlas_index`] to a sprite atlas cell.
//!
//! # Clock
//!
//! Animation time is an integer count of 1/60000 s units. One fixed tick adds
//! [`TICK_UNITS`] (1000) and one millisecond is [`UNITS_PER_MS`] (60), so
//! 60 Hz ticks and whole-millisecond frame durations meet without rounding and
//! no wall clock is involved.
//!
//! # States
//!
//! `idle`, `run`, `jump_start`, `jump_rise`, `jump_fall` and `land`, with the
//! frame ranges, durations and playback modes (loop, once, hold) read from the
//! exported atlas (or its pixel-grid source) and the manifest's
//! `locomotion.playback` block. Run frame times are not the authored ones: they
//! follow the speed-linked [`RunPacing`] rule of each actor.
//!
//! ```text
//!   idle <-- moving / stopped --> run          (instant, run restarts at frame 0)
//!
//!   grounded -> airborne, vy > 0   : jump_start (once, on the impulse tick)
//!                                      --> jump_rise (hold, while vy > 0)
//!                                      <-> jump_fall (hold, vy <= 0; the apex)
//!   grounded -> airborne, vy <= 0  : jump_fall   (walked off a ledge)
//!   airborne -> grounded           : land (once)
//!                                      --> idle, or run when moving
//!                                      run may cut land short once the impact
//!                                      frame has played; a jump interrupts it
//! ```

use std::collections::BTreeMap;

use serde::Deserialize;

/// Animation clock units per fixed 60 Hz tick (1 unit = 1/60000 s).
pub const TICK_UNITS: u64 = 1000;
/// Animation clock units per millisecond.
pub const UNITS_PER_MS: u64 = 60;
/// Horizontal speed (px/s) above which a grounded actor is "moving". Same
/// threshold the hero uses to pick a facing.
pub const MOVE_THRESHOLD_PX_S: f32 = 0.1;
/// `land` may be cut short by `run` once this many of its frames have played
/// (the impact frame): a running actor does not stand through the recovery.
pub const LAND_CUT_AFTER_FRAMES: usize = 1;

/// Carl: ground covered per run frame, in tenths of a pixel (8.0 px, the
/// authored stride at 60 ms per frame, see the run spec).
pub const CARL_RUN_GROUND_TENTHS_PX: u32 = 80;
/// Carl's shortest run frame. The orchestrator's choice of 43 ms is the
/// reviewer's recommendation: it makes the run slip-free at the 185 px/s
/// greybox `RUN_SPEED` (8000 / 185 = 43.2). Adjustable after playtest (the
/// earlier 50 ms floor left about 1.3 px of foot slip per frame at 185 px/s).
pub const CARL_RUN_FLOOR_MS: u32 = 43;
/// Carl's slowest run frame (slow starts look slower, not frantic).
pub const CARL_RUN_CEIL_MS: u32 = 90;
/// Donut: ground covered per run frame, tenths of a pixel (14.8 px).
pub const DONUT_RUN_GROUND_TENTHS_PX: u32 = 148;
pub const DONUT_RUN_FLOOR_MS: u32 = 50;
pub const DONUT_RUN_CEIL_MS: u32 = 130;
/// Donut's run frame weights `(numerator, denominator)`: run frame 4 (atlas
/// frame 8, the gallop suspension) is held 1.125 times as long.
pub const DONUT_RUN_WEIGHTS: [(u32, u32); 6] = [(1, 1), (1, 1), (1, 1), (1, 1), (9, 8), (1, 1)];

// ---------------------------------------------------------------- vocabulary --

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tag {
    Idle,
    Run,
    JumpStart,
    JumpRise,
    JumpFall,
    Land,
}

impl Tag {
    pub const ALL: [Tag; 6] = [
        Tag::Idle,
        Tag::Run,
        Tag::JumpStart,
        Tag::JumpRise,
        Tag::JumpFall,
        Tag::Land,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Tag::Idle => "idle",
            Tag::Run => "run",
            Tag::JumpStart => "jump_start",
            Tag::JumpRise => "jump_rise",
            Tag::JumpFall => "jump_fall",
            Tag::Land => "land",
        }
    }

    pub fn parse(name: &str) -> Option<Tag> {
        Tag::ALL.into_iter().find(|tag| tag.name() == name)
    }

    fn slot(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Playback {
    /// Wrap to the first frame.
    Loop,
    /// Play every frame once, then hand over to the next state.
    Once,
    /// Play every frame once, then stay on the last frame until the state
    /// changes (a single-frame hold simply shows that frame).
    Hold,
}

impl Playback {
    fn parse(text: &str) -> Option<Playback> {
        match text {
            "loop" => Some(Playback::Loop),
            "once" => Some(Playback::Once),
            "hold" => Some(Playback::Hold),
            _ => None,
        }
    }
}

/// One tagged frame range of the atlas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    /// Atlas index of the first frame.
    pub first: usize,
    /// Authored duration of every frame in milliseconds (all greater than 0).
    pub durations_ms: Vec<u32>,
    pub playback: Playback,
}

impl Clip {
    pub fn len(&self) -> usize {
        self.durations_ms.len()
    }

    pub fn is_empty(&self) -> bool {
        self.durations_ms.is_empty()
    }
}

/// The six locomotion clips of one actor atlas, read from data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipSet {
    pub frame_size: (i32, i32),
    pub pivot: (i32, i32),
    pub frame_count: usize,
    clips: [Clip; 6],
}

#[derive(Deserialize)]
struct ManifestData {
    frame_size: [i32; 2],
    pivot: [i32; 2],
    #[serde(default)]
    locomotion: Option<LocomotionBlock>,
}

#[derive(Deserialize)]
struct LocomotionBlock {
    #[serde(default)]
    playback: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct TagRange {
    name: String,
    from: usize,
    to: usize,
}

#[derive(Deserialize)]
struct AtlasFrame {
    duration: u32,
}

#[derive(Deserialize)]
struct AtlasMeta {
    #[serde(rename = "frameTags")]
    frame_tags: Vec<TagRange>,
}

#[derive(Deserialize)]
struct AtlasFile {
    frames: Vec<AtlasFrame>,
    meta: AtlasMeta,
}

#[derive(Deserialize)]
struct GridFrame {
    duration_ms: u32,
}

#[derive(Deserialize)]
struct GridFile {
    frames: Vec<GridFrame>,
    tags: Vec<TagRange>,
}

impl ClipSet {
    pub fn clip(&self, tag: Tag) -> &Clip {
        &self.clips[tag.slot()]
    }

    /// Clips from the manifest and the exported atlas JSON (Aseprite
    /// `json-array`: `frames[].duration`, `meta.frameTags`). The runtime path.
    pub fn from_atlas_json(manifest: &str, atlas: &str) -> Result<Self, String> {
        let atlas: AtlasFile =
            serde_json::from_str(atlas).map_err(|e| format!("atlas json: {e}"))?;
        let durations = atlas.frames.iter().map(|f| f.duration).collect();
        Self::build(manifest, durations, atlas.meta.frame_tags)
    }

    /// Clips from the manifest and the committed pixel-grid source, which holds
    /// the same durations and tags as the export. Used by tests and tools that
    /// must not need the (gitignored) exported atlas.
    pub fn from_grid_json(manifest: &str, grid: &str) -> Result<Self, String> {
        let grid: GridFile = serde_json::from_str(grid).map_err(|e| format!("grid json: {e}"))?;
        let durations = grid.frames.iter().map(|f| f.duration_ms).collect();
        Self::build(manifest, durations, grid.tags)
    }

    fn build(manifest: &str, durations: Vec<u32>, tags: Vec<TagRange>) -> Result<Self, String> {
        let manifest: ManifestData =
            serde_json::from_str(manifest).map_err(|e| format!("manifest json: {e}"))?;
        let playback = manifest
            .locomotion
            .map(|block| block.playback)
            .ok_or("manifest has no locomotion.playback block")?;
        if let Some(index) = durations.iter().position(|d| *d == 0) {
            return Err(format!("frame {index} has a zero duration"));
        }
        let mut slots: [Option<Clip>; 6] = Default::default();
        for range in tags {
            let Some(tag) = Tag::parse(&range.name) else {
                return Err(format!("unknown animation tag {:?}", range.name));
            };
            if range.from > range.to || range.to >= durations.len() {
                return Err(format!(
                    "tag {} range {}..={} is outside the {} frames",
                    range.name,
                    range.from,
                    range.to,
                    durations.len()
                ));
            }
            let mode = playback
                .get(tag.name())
                .ok_or_else(|| format!("manifest has no playback mode for {}", tag.name()))?;
            let mode = Playback::parse(mode)
                .ok_or_else(|| format!("bad playback mode {mode:?} for {}", tag.name()))?;
            let clip = Clip {
                first: range.from,
                durations_ms: durations[range.from..=range.to].to_vec(),
                playback: mode,
            };
            if slots[tag.slot()].replace(clip).is_some() {
                return Err(format!("tag {} appears twice", tag.name()));
            }
        }
        let mut clips = Vec::with_capacity(6);
        for tag in Tag::ALL {
            clips.push(
                slots[tag.slot()]
                    .take()
                    .ok_or_else(|| format!("atlas has no {} tag", tag.name()))?,
            );
        }
        Ok(Self {
            frame_size: (manifest.frame_size[0], manifest.frame_size[1]),
            pivot: (manifest.pivot[0], manifest.pivot[1]),
            frame_count: durations.len(),
            clips: clips.try_into().expect("six clips"),
        })
    }
}

// -------------------------------------------------------------------- pacing --

/// Speed-linked run frame time:
/// `frame_ms = clamp(round(ground_px * w[i] * 1000 / |vx|), min_ms, max_ms)`.
///
/// Integer maths only: ground distance in tenths of a pixel, the weight as a
/// fraction, the speed in milli-pixels per second, round half up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunPacing {
    pub ground_tenths_px: u32,
    pub min_ms: u32,
    pub max_ms: u32,
    pub weights: &'static [(u32, u32)],
}

pub const CARL_RUN_PACING: RunPacing = RunPacing {
    ground_tenths_px: CARL_RUN_GROUND_TENTHS_PX,
    min_ms: CARL_RUN_FLOOR_MS,
    max_ms: CARL_RUN_CEIL_MS,
    weights: &[],
};

pub const DONUT_RUN_PACING: RunPacing = RunPacing {
    ground_tenths_px: DONUT_RUN_GROUND_TENTHS_PX,
    min_ms: DONUT_RUN_FLOOR_MS,
    max_ms: DONUT_RUN_CEIL_MS,
    weights: &DONUT_RUN_WEIGHTS,
};

/// `|vx|` in milli-pixels per second, rounded to the nearest integer.
pub fn speed_milli(vx: f32) -> u64 {
    if vx.is_finite() {
        (vx.abs() * 1000.0).round() as u64
    } else {
        0
    }
}

impl RunPacing {
    /// Duration in ms of run frame `run_index` (0-based inside the run clip)
    /// at the given speed. A speed of 0 gives the slowest frame time.
    pub fn frame_ms(&self, run_index: usize, speed_milli: u64) -> u32 {
        if speed_milli == 0 {
            return self.max_ms;
        }
        let (weight_num, weight_den) = self.weights.get(run_index).copied().unwrap_or((1, 1));
        // ms = ground_px * w * 1000 / v_px_s
        //    = (tenths / 10) * (num / den) * 1000 * 1000 / v_milli
        let numerator = u64::from(self.ground_tenths_px) * u64::from(weight_num) * 100_000;
        let denominator = u64::from(weight_den) * speed_milli;
        let rounded = (2 * numerator + denominator) / (2 * denominator);
        rounded.clamp(u64::from(self.min_ms), u64::from(self.max_ms)) as u32
    }
}

// ------------------------------------------------------------------ animator --

/// What the animation needs from the simulation, once per fixed tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimInput {
    pub grounded: bool,
    /// Horizontal velocity in px/s (sign ignored).
    pub vx: f32,
    /// Vertical velocity in px/s, y up (positive rises).
    pub vy: f32,
}

#[derive(Debug, Clone)]
pub struct Animator {
    clips: ClipSet,
    pacing: RunPacing,
    tag: Tag,
    /// Frame index inside the current clip.
    frame: usize,
    /// Time spent on the current frame, in clock units.
    elapsed: u64,
    /// Grounded flag of the previous input (None before the first step).
    was_grounded: Option<bool>,
}

impl Animator {
    pub fn new(clips: ClipSet, pacing: RunPacing) -> Self {
        Self {
            clips,
            pacing,
            tag: Tag::Idle,
            frame: 0,
            elapsed: 0,
            was_grounded: None,
        }
    }

    pub fn reset(&mut self) {
        self.tag = Tag::Idle;
        self.frame = 0;
        self.elapsed = 0;
        self.was_grounded = None;
    }

    pub fn tag(&self) -> Tag {
        self.tag
    }

    /// Frame index inside the current clip.
    pub fn frame_in_clip(&self) -> usize {
        self.frame
    }

    /// Atlas cell to draw.
    pub fn atlas_index(&self) -> usize {
        self.clips.clip(self.tag).first + self.frame
    }

    pub fn clips(&self) -> &ClipSet {
        &self.clips
    }

    pub fn pacing(&self) -> RunPacing {
        self.pacing
    }

    /// `tag:atlas_index`, for logs and state lines.
    pub fn label(&self) -> String {
        format!("{}:{}", self.tag.name(), self.atlas_index())
    }

    /// Advance one fixed tick. The state entered on a tick shows its first
    /// frame on that same tick; time only accumulates on later ticks.
    pub fn step(&mut self, input: AnimInput) {
        let moving = input.vx.abs() > MOVE_THRESHOLD_PX_S;
        let was_grounded = self.was_grounded.unwrap_or(input.grounded);
        self.was_grounded = Some(input.grounded);
        let target = match (was_grounded, input.grounded) {
            // Touchdown.
            (false, true) => Some(Tag::Land),
            // Left the ground: a rising start is a jump (impulse applied this
            // tick); a non-rising start is a walk-off and goes straight to fall.
            (true, false) => Some(if input.vy > 0.0 {
                Tag::JumpStart
            } else {
                Tag::JumpFall
            }),
            (_, true) => self.grounded_target(moving),
            (_, false) => self.airborne_target(input.vy),
        };
        match target {
            Some(tag) if tag != self.tag => self.enter(tag),
            _ => self.advance(input, moving),
        }
    }

    fn enter(&mut self, tag: Tag) {
        self.tag = tag;
        self.frame = 0;
        self.elapsed = 0;
    }

    fn grounded_target(&self, moving: bool) -> Option<Tag> {
        match self.tag {
            Tag::Idle if moving => Some(Tag::Run),
            Tag::Run if !moving => Some(Tag::Idle),
            // A running actor leaves the landing recovery once the impact
            // frame has played; a standing one plays it to the end.
            Tag::Land if moving && self.frame >= LAND_CUT_AFTER_FRAMES => Some(Tag::Run),
            // Stale airborne tag on the ground (only possible on the first tick).
            Tag::JumpStart | Tag::JumpRise | Tag::JumpFall => {
                Some(if moving { Tag::Run } else { Tag::Idle })
            }
            _ => None,
        }
    }

    fn airborne_target(&self, vy: f32) -> Option<Tag> {
        let by_velocity = if vy > 0.0 {
            Tag::JumpRise
        } else {
            Tag::JumpFall
        };
        match self.tag {
            // jump_start always plays to its end (then rise or fall).
            Tag::JumpStart => None,
            // Rise and fall are held frames chosen by the sign of vy. Idle,
            // run and land in the air are stale (first tick only).
            Tag::JumpRise | Tag::JumpFall | Tag::Idle | Tag::Run | Tag::Land => Some(by_velocity),
        }
    }

    fn frame_units(&self, speed: u64) -> u64 {
        let ms = if self.tag == Tag::Run {
            self.pacing.frame_ms(self.frame, speed)
        } else {
            self.clips.clip(self.tag).durations_ms[self.frame]
        };
        u64::from(ms) * UNITS_PER_MS
    }

    fn advance(&mut self, input: AnimInput, moving: bool) {
        let speed = speed_milli(input.vx);
        self.elapsed += TICK_UNITS;
        loop {
            let duration = self.frame_units(speed);
            if self.elapsed < duration {
                return;
            }
            let clip = self.clips.clip(self.tag);
            if self.frame + 1 < clip.len() {
                self.elapsed -= duration;
                self.frame += 1;
                continue;
            }
            match clip.playback {
                Playback::Loop => {
                    self.elapsed -= duration;
                    self.frame = 0;
                }
                Playback::Hold => {
                    self.elapsed = 0;
                    return;
                }
                Playback::Once => {
                    let next = self.finished_target(input, moving);
                    self.enter(next);
                    return;
                }
            }
        }
    }

    /// Where a finished `once` clip hands over to.
    fn finished_target(&self, input: AnimInput, moving: bool) -> Tag {
        match self.tag {
            Tag::JumpStart if input.vy > 0.0 => Tag::JumpRise,
            Tag::JumpStart => Tag::JumpFall,
            _ if moving => Tag::Run,
            _ => Tag::Idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const RUN_SPEED: f32 = 185.0;

    fn read(path: &str) -> String {
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
    }

    /// Clips read from the committed manifests and pixel-grid sources.
    fn carl() -> ClipSet {
        ClipSet::from_grid_json(
            &read("assets/manifests/carl_book1_h52_locomotion.json"),
            &read("assets/source/characters/carl/carl_h52_locomotion.grid.json"),
        )
        .unwrap()
    }

    fn donut() -> ClipSet {
        ClipSet::from_grid_json(
            &read("assets/manifests/donut_h40_locomotion.json"),
            &read("assets/source/characters/donut/donut_h40_locomotion.grid.json"),
        )
        .unwrap()
    }

    fn carl_animator() -> Animator {
        Animator::new(carl(), CARL_RUN_PACING)
    }

    fn ground(vx: f32) -> AnimInput {
        AnimInput {
            grounded: true,
            vx,
            vy: 0.0,
        }
    }

    fn air(vx: f32, vy: f32) -> AnimInput {
        AnimInput {
            grounded: false,
            vx,
            vy,
        }
    }

    fn drive(animator: &mut Animator, input: AnimInput, ticks: usize) {
        for _ in 0..ticks {
            animator.step(input);
        }
    }

    #[test]
    fn clips_come_from_the_manifest_and_the_grid_source() {
        let carl = carl();
        assert_eq!((carl.frame_size, carl.pivot), ((48, 56), (25, 55)));
        assert_eq!(carl.frame_count, 16);
        let run = carl.clip(Tag::Run);
        assert_eq!((run.first, run.len(), run.playback), (4, 6, Playback::Loop));
        assert_eq!(carl.clip(Tag::Idle).durations_ms, [420, 260, 560, 260]);
        assert_eq!(carl.clip(Tag::JumpStart).durations_ms, [50, 70]);
        assert_eq!(carl.clip(Tag::Land).durations_ms, [80, 100]);
        assert_eq!(carl.clip(Tag::Idle).playback, Playback::Loop);
        assert_eq!(carl.clip(Tag::JumpStart).playback, Playback::Once);
        assert_eq!(carl.clip(Tag::JumpRise).playback, Playback::Hold);
        assert_eq!(carl.clip(Tag::JumpFall).playback, Playback::Hold);
        assert_eq!(carl.clip(Tag::Land).playback, Playback::Once);
        let donut = donut();
        assert_eq!((donut.frame_size, donut.pivot), ((72, 44), (46, 43)));
        assert_eq!(donut.frame_count, 16);
        assert_eq!(
            donut.clip(Tag::Run).durations_ms,
            [80, 80, 80, 80, 90, 80],
            "the atlas stores the 185 px/s values"
        );
        assert_eq!(donut.clip(Tag::Idle).durations_ms, [320, 260, 180, 260]);
        assert_eq!(donut.clip(Tag::JumpStart).durations_ms, [90, 70]);
        assert_eq!(donut.clip(Tag::JumpRise).first, 12);
        assert_eq!(donut.clip(Tag::JumpFall).first, 13);
        assert_eq!(donut.clip(Tag::Land).durations_ms, [80, 110]);
        assert_eq!(donut.clip(Tag::JumpRise).playback, Playback::Hold);
    }

    #[test]
    fn exported_atlas_json_matches_the_grid_source_when_present() {
        // The export is gitignored: only compare when it exists.
        for (manifest, atlas, set) in [
            (
                "assets/manifests/carl_book1_h52_locomotion.json",
                "generated/art/carl_book1_h52_locomotion.json",
                carl(),
            ),
            (
                "assets/manifests/donut_h40_locomotion.json",
                "generated/art/donut_h40_locomotion.json",
                donut(),
            ),
        ] {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(atlas);
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };
            assert_eq!(
                ClipSet::from_atlas_json(&read(manifest), &text).unwrap(),
                set
            );
        }
    }

    #[test]
    fn bad_clip_data_is_rejected() {
        let manifest = read("assets/manifests/carl_book1_h52_locomotion.json");
        let grid = read("assets/source/characters/carl/carl_h52_locomotion.grid.json");
        assert!(ClipSet::from_grid_json(&manifest, &grid).is_ok());
        let bad_mode = manifest.replace("\"jump_rise\": \"hold\"", "\"jump_rise\": \"spin\"");
        assert!(
            ClipSet::from_grid_json(&bad_mode, &grid)
                .unwrap_err()
                .contains("bad playback mode")
        );
        let no_block = r#"{"frame_size":[48,56],"pivot":[25,55]}"#;
        assert!(ClipSet::from_grid_json(no_block, &grid).is_err());
        let truncated = grid.replace("\"to\": 15", "\"to\": 99");
        assert!(
            ClipSet::from_grid_json(&manifest, &truncated)
                .unwrap_err()
                .contains("outside")
        );
        let zero = grid.replacen("\"duration_ms\": 420", "\"duration_ms\": 0", 1);
        assert!(
            ClipSet::from_grid_json(&manifest, &zero)
                .unwrap_err()
                .contains("zero duration")
        );
    }

    #[test]
    fn run_pacing_matches_the_specs_at_the_documented_speeds() {
        // Carl: clamp(round(8000 / |vx|), 43, 90).
        let carl = CARL_RUN_PACING;
        assert_eq!(carl.frame_ms(0, speed_milli(0.0)), 90, "standstill");
        assert_eq!(carl.frame_ms(0, speed_milli(145.0)), 55);
        assert_eq!(
            carl.frame_ms(0, speed_milli(185.0)),
            43,
            "slip-free at RUN_SPEED"
        );
        assert_eq!(carl.frame_ms(0, speed_milli(133.0)), 60, "authored speed");
        assert_eq!(carl.frame_ms(3, speed_milli(60.0)), 90, "ceiling");
        assert_eq!(carl.frame_ms(3, speed_milli(400.0)), 43, "floor");
        assert_eq!(carl.frame_ms(0, speed_milli(-185.0)), 43, "sign is ignored");
        // Donut: clamp(round(w * 1000 * 14.8 / |vx|), 50, 130), w[4] = 1.125.
        let donut = DONUT_RUN_PACING;
        let at = |v: f32| {
            (0..6)
                .map(|i| donut.frame_ms(i, speed_milli(v)))
                .collect::<Vec<_>>()
        };
        assert_eq!(at(185.0), [80, 80, 80, 80, 90, 80], "the atlas values");
        assert_eq!(at(145.0), [102, 102, 102, 102, 115, 102]);
        assert_eq!(at(100.0), [130; 6], "below about 114 px/s the cap applies");
        assert_eq!(at(0.0), [130; 6]);
        assert_eq!(at(400.0), [50; 6], "floor");
    }

    #[test]
    fn idle_loops_with_the_authored_durations() {
        let mut a = carl_animator();
        assert_eq!((a.tag(), a.atlas_index()), (Tag::Idle, 0));
        // Frame 0 lasts 420 ms = 25.2 ticks: frame 1 appears on the 26th step.
        drive(&mut a, ground(0.0), 25);
        assert_eq!(a.atlas_index(), 0);
        drive(&mut a, ground(0.0), 1);
        assert_eq!(a.atlas_index(), 1);
        // The full cycle is 420 + 260 + 560 + 260 = 1500 ms = 90 ticks.
        let mut b = carl_animator();
        drive(&mut b, ground(0.0), 90);
        assert_eq!((b.tag(), b.atlas_index()), (Tag::Idle, 0));
    }

    #[test]
    fn idle_and_run_switch_on_the_move_threshold() {
        let mut a = carl_animator();
        drive(&mut a, ground(0.0), 30);
        assert_eq!(a.tag(), Tag::Idle);
        a.step(ground(RUN_SPEED));
        assert_eq!(
            (a.tag(), a.atlas_index()),
            (Tag::Run, 4),
            "run starts on its first frame"
        );
        a.step(ground(-RUN_SPEED));
        assert_eq!(a.tag(), Tag::Run, "direction does not matter");
        a.step(ground(0.05));
        assert_eq!(a.tag(), Tag::Idle, "below the threshold is idle");
        assert_eq!(a.atlas_index(), 0, "idle restarts on its first frame");
    }

    #[test]
    fn run_frames_follow_the_speed_linked_time_and_loop() {
        let mut a = carl_animator();
        a.step(ground(RUN_SPEED));
        // 43 ms = 2.58 ticks per frame: the frame changes on the 3rd tick.
        let mut seen = vec![a.atlas_index()];
        for _ in 0..40 {
            a.step(ground(RUN_SPEED));
            seen.push(a.atlas_index());
        }
        assert_eq!(&seen[..8], [4, 4, 4, 5, 5, 5, 6, 6]);
        assert!(seen.iter().all(|i| (4..=9).contains(i)));
        assert!(seen.contains(&9) && seen.contains(&4));
        // One cycle is 6 x 43 ms = 15480 clock units: it wraps on the 16th tick.
        let mut c = carl_animator();
        c.step(ground(RUN_SPEED));
        let mut ticks = 0u64;
        let mut previous = c.atlas_index();
        loop {
            c.step(ground(RUN_SPEED));
            ticks += 1;
            if previous == 9 && c.atlas_index() == 4 {
                break;
            }
            previous = c.atlas_index();
        }
        assert_eq!(ticks, 16);
    }

    #[test]
    fn run_phase_is_kept_when_the_speed_changes() {
        let mut a = carl_animator();
        a.step(ground(RUN_SPEED));
        drive(&mut a, ground(RUN_SPEED), 8);
        let frame = a.frame_in_clip();
        assert!(frame >= 2);
        // Slowing down changes the frame time, not the frame.
        a.step(ground(60.0));
        assert!(a.frame_in_clip() >= frame && a.tag() == Tag::Run);
    }

    #[test]
    fn takeoff_starts_jump_start_on_the_same_tick_then_rises_then_falls() {
        let mut a = carl_animator();
        drive(&mut a, ground(RUN_SPEED), 20);
        // The impulse tick: left the ground with vy > 0.
        a.step(air(RUN_SPEED, 432.0));
        assert_eq!(
            (a.tag(), a.atlas_index()),
            (Tag::JumpStart, 10),
            "crouch on the takeoff tick"
        );
        // 50 ms = 3 ticks on frame 10, then frame 11 for 70 ms = 4.2 ticks.
        a.step(air(RUN_SPEED, 415.0));
        a.step(air(RUN_SPEED, 397.0));
        assert_eq!(a.atlas_index(), 10);
        a.step(air(RUN_SPEED, 380.0));
        assert_eq!(a.atlas_index(), 11);
        drive(&mut a, air(RUN_SPEED, 300.0), 4);
        assert_eq!((a.tag(), a.atlas_index()), (Tag::JumpStart, 11));
        a.step(air(RUN_SPEED, 280.0));
        assert_eq!(
            (a.tag(), a.atlas_index()),
            (Tag::JumpRise, 12),
            "rise follows jump_start"
        );
        // Held while rising, however long.
        drive(&mut a, air(RUN_SPEED, 100.0), 60);
        assert_eq!((a.tag(), a.atlas_index()), (Tag::JumpRise, 12));
        // Apex: vy <= 0 flips to fall on that tick.
        a.step(air(RUN_SPEED, 0.0));
        assert_eq!((a.tag(), a.atlas_index()), (Tag::JumpFall, 13));
        drive(&mut a, air(RUN_SPEED, -200.0), 60);
        assert_eq!((a.tag(), a.atlas_index()), (Tag::JumpFall, 13));
    }

    #[test]
    fn jump_start_is_not_cut_by_an_early_apex_but_hands_over_by_velocity() {
        let mut a = carl_animator();
        a.step(ground(0.0));
        a.step(air(0.0, 440.0));
        assert_eq!(a.tag(), Tag::JumpStart);
        // A ceiling bonk zeroes vy: jump_start keeps playing (120 ms = 7.2
        // ticks), then falls.
        drive(&mut a, air(0.0, -10.0), 7);
        assert_eq!(a.tag(), Tag::JumpStart);
        a.step(air(0.0, -10.0));
        assert_eq!((a.tag(), a.atlas_index()), (Tag::JumpFall, 13));
    }

    #[test]
    fn walking_off_a_ledge_goes_straight_to_fall() {
        let mut a = carl_animator();
        drive(&mut a, ground(RUN_SPEED), 10);
        a.step(air(RUN_SPEED, -17.5));
        assert_eq!((a.tag(), a.atlas_index()), (Tag::JumpFall, 13));
    }

    #[test]
    fn touchdown_plays_land_once_then_idles() {
        let mut a = carl_animator();
        a.step(ground(0.0));
        a.step(air(0.0, 440.0));
        drive(&mut a, air(0.0, -50.0), 30);
        assert_eq!(a.tag(), Tag::JumpFall);
        // Touchdown tick: impact frame at once.
        a.step(ground(0.0));
        assert_eq!((a.tag(), a.atlas_index()), (Tag::Land, 14));
        // 80 ms = 4.8 ticks -> recovery on the 5th tick after touchdown.
        drive(&mut a, ground(0.0), 4);
        assert_eq!(a.atlas_index(), 14);
        a.step(ground(0.0));
        assert_eq!(a.atlas_index(), 15);
        // Not moving: the recovery plays to the end (100 ms = 6 ticks).
        drive(&mut a, ground(0.0), 5);
        assert_eq!((a.tag(), a.atlas_index()), (Tag::Land, 15));
        a.step(ground(0.0));
        assert_eq!(
            (a.tag(), a.atlas_index()),
            (Tag::Idle, 0),
            "once, then idle"
        );
        drive(&mut a, ground(0.0), 20);
        assert_eq!(a.tag(), Tag::Idle, "land does not replay");
    }

    #[test]
    fn running_cuts_land_short_only_after_the_impact_frame() {
        let mut a = carl_animator();
        a.step(air(RUN_SPEED, -300.0));
        a.step(ground(RUN_SPEED));
        assert_eq!((a.tag(), a.atlas_index()), (Tag::Land, 14));
        // While the impact frame is still showing, run does not cut in.
        drive(&mut a, ground(RUN_SPEED), 4);
        assert_eq!((a.tag(), a.atlas_index()), (Tag::Land, 14));
        // The recovery frame appears on tick 5; the next tick hands over to run.
        a.step(ground(RUN_SPEED));
        assert_eq!((a.tag(), a.atlas_index()), (Tag::Land, 15));
        a.step(ground(RUN_SPEED));
        assert_eq!(
            (a.tag(), a.atlas_index()),
            (Tag::Run, 4),
            "run cut the recovery short"
        );
        // Standing still through the same landing does not cut.
        let mut b = carl_animator();
        b.step(air(0.0, -300.0));
        b.step(ground(0.0));
        drive(&mut b, ground(0.0), 5);
        assert_eq!((b.tag(), b.atlas_index()), (Tag::Land, 15));
        drive(&mut b, ground(0.0), 3);
        assert_eq!(b.tag(), Tag::Land);
    }

    #[test]
    fn a_new_jump_interrupts_land() {
        let mut a = carl_animator();
        a.step(air(0.0, -300.0));
        a.step(ground(0.0));
        assert_eq!(a.tag(), Tag::Land);
        a.step(air(0.0, 432.0));
        assert_eq!((a.tag(), a.atlas_index()), (Tag::JumpStart, 10));
    }

    #[test]
    fn donut_uses_her_own_clips_and_run_rule() {
        let mut a = Animator::new(donut(), DONUT_RUN_PACING);
        a.step(ground(RUN_SPEED));
        let mut frames = vec![a.atlas_index()];
        for _ in 0..40 {
            a.step(ground(RUN_SPEED));
            frames.push(a.atlas_index());
        }
        // 80 ms = 4.8 ticks per frame at 185 px/s.
        assert_eq!(&frames[..6], [4, 4, 4, 4, 4, 5]);
        assert!(frames.contains(&8) && frames.contains(&9));
        let mut b = Animator::new(donut(), DONUT_RUN_PACING);
        b.step(ground(0.0));
        b.step(air(0.0, 100.0));
        assert_eq!((b.tag(), b.atlas_index()), (Tag::JumpStart, 10));
        // jump_start: 90 ms = 5.4 ticks on the crouch, then 70 ms.
        drive(&mut b, air(0.0, 100.0), 5);
        assert_eq!(b.atlas_index(), 10);
        drive(&mut b, air(0.0, 100.0), 1);
        assert_eq!(b.atlas_index(), 11);
    }

    /// A fixed input trace covering every state.
    fn trace() -> Vec<AnimInput> {
        let mut t = Vec::new();
        t.extend(std::iter::repeat_n(ground(0.0), 40));
        t.extend(std::iter::repeat_n(ground(RUN_SPEED), 60));
        t.push(air(RUN_SPEED, 432.0));
        for k in 0..50 {
            t.push(air(RUN_SPEED, 432.0 - 17.5 * (k + 1) as f32));
        }
        t.extend(std::iter::repeat_n(ground(RUN_SPEED), 20));
        t.extend(std::iter::repeat_n(ground(0.0), 30));
        t
    }

    fn play(set: ClipSet, pacing: RunPacing) -> Vec<(Tag, usize)> {
        let mut a = Animator::new(set, pacing);
        trace()
            .into_iter()
            .map(|input| {
                a.step(input);
                (a.tag(), a.atlas_index())
            })
            .collect()
    }

    #[test]
    fn the_same_input_trace_gives_the_same_frame_sequence() {
        let first = play(carl(), CARL_RUN_PACING);
        assert_eq!(first, play(carl(), CARL_RUN_PACING));
        let donut_frames = play(donut(), DONUT_RUN_PACING);
        assert_eq!(donut_frames, play(donut(), DONUT_RUN_PACING));
        // Every state shows up in the trace.
        for tag in Tag::ALL {
            assert!(first.iter().any(|(t, _)| *t == tag), "{tag:?} missing");
        }
        // A reset replays the identical sequence.
        let mut a = carl_animator();
        let replay = |a: &mut Animator| -> Vec<(Tag, usize)> {
            trace()
                .into_iter()
                .map(|input| {
                    a.step(input);
                    (a.tag(), a.atlas_index())
                })
                .collect()
        };
        let run1 = replay(&mut a);
        a.reset();
        assert_eq!(run1, replay(&mut a));
        assert_eq!(run1, first);
    }

    #[test]
    fn nonfinite_velocity_never_panics() {
        let mut a = carl_animator();
        a.step(ground(f32::NAN));
        a.step(ground(f32::INFINITY));
        a.step(air(f32::NAN, f32::NAN));
        assert_eq!(speed_milli(f32::NAN), 0);
    }
}
