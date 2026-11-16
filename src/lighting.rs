//! Presentation-only lighting data for the Floor 1 corridor (COMP-02).
//!
//! Pure and headless: no Bevy types. It reads the lights the kit contract puts
//! on pieces (`pieces[].lights`) and the layout `lighting` profile, derives a
//! static emitter list once at load, and per frame turns that list plus a view
//! rectangle and the presentation tick into plain light samples in view pixels.
//! The Bevy adapter (`light_pass`) packs the samples for the GPU.
//!
//! **Lighting never feeds the simulation.** Nothing here takes a mutable
//! reference to simulation state or to collision data; the walking line is read
//! once at load to place floor pools. Lights, ambient, haze, bloom and shadows
//! are never read by collision, AI or damage (contract: "Presentation-only
//! data"). The model follows `scripts/env_kit_light.py` (Pillow), the owner
//! approved A1 reference.

use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

use crate::env_kit::{Contract, EnvKit, Layout, animation_frame, parallax_copies, parallax_start};

/// Most lights the GPU pass accepts in one frame (uniform array length).
pub const MAX_LIGHTS: usize = 64;
/// Most texture slots (draw-order runs that share a light group) the pass binds.
pub const MAX_SLOTS: usize = 6;
/// Most contact shadows (actors) per frame.
pub const MAX_SHADOWS: usize = 8;
/// Most emissive atlas colours the bloom mask accepts.
pub const MAX_EMISSIVE_COLOURS: usize = 16;

#[derive(Debug)]
pub enum LightingError {
    Invalid(String),
}

impl fmt::Display for LightingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(m) => write!(f, "lighting: {m}"),
        }
    }
}

impl std::error::Error for LightingError {}

fn invalid<T>(message: impl Into<String>) -> Result<T, LightingError> {
    Err(LightingError::Invalid(message.into()))
}

// ------------------------------------------------------------------ groups --

/// Light groups of the contract, in draw order (far to near).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LightGroup {
    ParallaxFar,
    ParallaxMid,
    Scene,
    Actors,
    Foreground,
}

impl LightGroup {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "parallax_far" => Self::ParallaxFar,
            "parallax_mid" => Self::ParallaxMid,
            "scene" => Self::Scene,
            "actors" => Self::Actors,
            "foreground" => Self::Foreground,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::ParallaxFar => "parallax_far",
            Self::ParallaxMid => "parallax_mid",
            Self::Scene => "scene",
            Self::Actors => "actors",
            Self::Foreground => "foreground",
        }
    }

    pub fn is_parallax(self) -> bool {
        matches!(self, Self::ParallaxFar | Self::ParallaxMid)
    }

    /// Which emitter list lights this group: parallax groups use their own
    /// emitters; scene, actors and foreground use the scene emitters (actors
    /// and foreground at their group's `light_scale`).
    pub fn emitter_group(self) -> Self {
        match self {
            Self::ParallaxFar => Self::ParallaxFar,
            Self::ParallaxMid => Self::ParallaxMid,
            _ => Self::Scene,
        }
    }
}

// ------------------------------------------------------------ contract data --

#[derive(Debug, Clone, Deserialize)]
pub struct GlowDef {
    pub radius: f32,
    pub additive: f32,
}

/// One emitter as the contract writes it on a piece (`pieces[].lights`).
#[derive(Debug, Clone, Deserialize)]
pub struct LightDef {
    pub id: String,
    /// Piece-local px from the piece top-left (strip-local for a parallax strip).
    pub offset: [f32; 2],
    pub colour: String,
    pub radius: f32,
    pub intensity: f32,
    pub group: String,
    #[serde(default)]
    pub glow: Option<GlowDef>,
    #[serde(default)]
    pub flicker: Option<Vec<f32>>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub project_dy: Option<f32>,
    #[serde(default)]
    pub squash_y: Option<f32>,
}

// ----------------------------------------------------------------- profile --

#[derive(Debug, Clone, Deserialize)]
pub struct FogProfile {
    pub from_y: f32,
    pub to_y: f32,
    pub amount: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HazeProfile {
    pub colour: String,
    pub amount: f32,
    #[serde(default)]
    pub floor_fog: Option<FogProfile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroupProfile {
    pub ambient: [f32; 3],
    #[serde(default)]
    pub haze: Option<HazeProfile>,
    #[serde(default)]
    pub light_scale: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContactShadowProfile {
    pub width_scale: f32,
    pub half_height: f32,
    pub strength: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BloomProfile {
    pub emissive_layers: Vec<String>,
    #[serde(default)]
    pub emissive_colours: Vec<String>,
    /// Bright-pass threshold in 8-bit units (0..255).
    pub threshold: f32,
    pub intensity: f32,
    /// Gaussian sigma in virtual pixels.
    pub radius_virtual: f32,
}

/// The layout `lighting` profile. Carries ambient, haze, shadow and bloom only.
#[derive(Debug, Clone, Deserialize)]
pub struct LightingProfile {
    pub profile_id: String,
    pub presentation_only: bool,
    pub groups: BTreeMap<String, GroupProfile>,
    #[serde(default)]
    pub actor_contact_shadow: Option<ContactShadowProfile>,
    pub bloom: BloomProfile,
}

/// `#rrggbb` to 8-bit channels.
pub fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let t = text.strip_prefix('#')?;
    if t.len() != 6 || !t.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(t, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// `#rrggbb` to channels in 0..=1 (the sRGB-encoded value divided by 255, the
/// space the reference model multiplies in).
pub fn hex_unit(text: &str) -> Option<[f32; 3]> {
    parse_hex(text).map(|c| [c[0], c[1], c[2]].map(|v| f32::from(v) / 255.0))
}

fn in_range(name: &str, value: f32, lo: f32, hi: f32) -> Result<(), LightingError> {
    if value.is_finite() && (lo..=hi).contains(&value) {
        Ok(())
    } else {
        invalid(format!("{name} = {value} is outside {lo}..={hi}"))
    }
}

impl LightingProfile {
    /// Validate ranges and structure. `presentation_only` must be true.
    pub fn validate(&self) -> Result<(), LightingError> {
        if !self.presentation_only {
            return invalid("lighting profile must declare presentation_only: true");
        }
        if self.profile_id.is_empty() {
            return invalid("lighting profile needs a profile_id");
        }
        for name in [
            "parallax_far",
            "parallax_mid",
            "scene",
            "actors",
            "foreground",
        ] {
            if !self.groups.contains_key(name) {
                return invalid(format!("lighting profile has no group {name:?}"));
            }
        }
        for (name, group) in &self.groups {
            if LightGroup::parse(name).is_none() {
                return invalid(format!("unknown lighting group {name:?}"));
            }
            for v in group.ambient {
                in_range(&format!("{name} ambient"), v, 0.0, 2.0)?;
            }
            if let Some(scale) = group.light_scale {
                in_range(&format!("{name} light_scale"), scale, 0.0, 1.0)?;
            }
            if let Some(haze) = &group.haze {
                if hex_unit(&haze.colour).is_none() {
                    return invalid(format!(
                        "{name} haze colour {:?} is not #rrggbb",
                        haze.colour
                    ));
                }
                in_range(&format!("{name} haze amount"), haze.amount, 0.0, 1.0)?;
                if let Some(fog) = &haze.floor_fog {
                    in_range(&format!("{name} fog amount"), fog.amount, 0.0, 1.0)?;
                    if fog.to_y <= fog.from_y {
                        return invalid(format!("{name} floor_fog needs to_y > from_y"));
                    }
                }
            }
        }
        if let Some(shadow) = &self.actor_contact_shadow {
            in_range("contact shadow width_scale", shadow.width_scale, 0.1, 4.0)?;
            in_range("contact shadow half_height", shadow.half_height, 0.5, 64.0)?;
            in_range("contact shadow strength", shadow.strength, 0.0, 1.0)?;
        }
        in_range("bloom threshold", self.bloom.threshold, 0.0, 254.0)?;
        in_range("bloom intensity", self.bloom.intensity, 0.0, 4.0)?;
        in_range("bloom radius_virtual", self.bloom.radius_virtual, 0.1, 8.0)?;
        if self.bloom.emissive_colours.len() > MAX_EMISSIVE_COLOURS {
            return invalid(format!(
                "bloom lists {} emissive colours, the pass accepts {MAX_EMISSIVE_COLOURS}",
                self.bloom.emissive_colours.len()
            ));
        }
        for colour in &self.bloom.emissive_colours {
            if parse_hex(colour).is_none() {
                return invalid(format!("bloom emissive colour {colour:?} is not #rrggbb"));
            }
        }
        Ok(())
    }

    pub fn group(&self, group: LightGroup) -> &GroupProfile {
        // `validate` guarantees every group exists.
        &self.groups[group.name()]
    }
}

// ------------------------------------------------------------------- slots --

/// One texture slot of the lighting pass: a run of contiguous draw orders that
/// share a light group and an emissive flag. Each slot is rendered to its own
/// virtual-resolution target and lit as a unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub name: String,
    pub group: LightGroup,
    /// Flame-coloured pixels of this slot feed bloom; everything else occludes.
    pub emissive: bool,
}

/// Slots in draw order plus the slot index of every contract layer and of the
/// actors.
#[derive(Debug, Clone)]
pub struct SlotPlan {
    pub slots: Vec<Slot>,
    pub layer_slot: BTreeMap<String, usize>,
    pub actor_slot: usize,
}

fn plan_slots(contract: &Contract, profile: &LightingProfile) -> Result<SlotPlan, LightingError> {
    let actor_group = LightGroup::parse(&contract.actor_light_group)
        .ok_or_else(|| LightingError::Invalid("contract actor_light_group is unknown".into()))?;
    // (draw order, name, group, emissive)
    let mut entries: Vec<(u32, String, LightGroup, bool)> = Vec::new();
    for layer in &contract.layers {
        let Some(name) = layer.light_group.as_deref() else {
            return invalid(format!("layer {} has no light_group", layer.name));
        };
        let Some(group) = LightGroup::parse(name) else {
            return invalid(format!(
                "layer {} names unknown light group {name:?}",
                layer.name
            ));
        };
        let emissive = profile.bloom.emissive_layers.contains(&layer.name);
        entries.push((layer.draw_order, layer.name.clone(), group, emissive));
    }
    entries.push((
        contract.actor_draw_order,
        "actors".to_owned(),
        actor_group,
        false,
    ));
    entries.sort_by_key(|e| e.0);
    let mut slots: Vec<Slot> = Vec::new();
    let mut layer_slot = BTreeMap::new();
    let mut actor_slot = 0;
    for (_, name, group, emissive) in entries {
        match slots.last_mut() {
            Some(last) if last.group == group && last.emissive == emissive => {
                last.name.push('+');
                last.name.push_str(&name);
            }
            _ => slots.push(Slot {
                name: name.clone(),
                group,
                emissive,
            }),
        }
        let index = slots.len() - 1;
        if name == "actors" {
            actor_slot = index;
        }
        layer_slot.insert(name, index);
    }
    if slots.len() > MAX_SLOTS {
        return invalid(format!(
            "the layer plan needs {} texture slots, the pass binds {MAX_SLOTS}",
            slots.len()
        ));
    }
    Ok(SlotPlan {
        slots,
        layer_slot,
        actor_slot,
    })
}

// ---------------------------------------------------------------- emitters --

/// Frame source for a flickering emitter (the piece's animation).
#[derive(Debug, Clone)]
pub struct EmitterAnim {
    pub durations: Vec<u32>,
    pub offset: usize,
}

/// A derived emitter, ready to place. Pieces give an absolute layout position;
/// a parallax strip gives a strip-local x repeated per copy.
#[derive(Debug, Clone)]
pub struct Emitter {
    pub id: String,
    pub group: LightGroup,
    /// Layout px (y down). For strip emitters `x` is strip-local.
    pub x: f32,
    pub y: f32,
    pub colour: [f32; 3],
    pub radius: f32,
    pub intensity: f32,
    pub squash_y: f32,
    pub glow_radius: f32,
    pub glow_additive: f32,
    pub flicker: Vec<f32>,
    pub anim: Option<EmitterAnim>,
    /// Index into `EnvKit::strips` for a strip emitter.
    pub strip: Option<usize>,
}

impl Emitter {
    /// Flicker multiplier at a presentation tick (1.0 without a flicker list).
    /// Driven by the piece's animation frame, so it is a pure function of the
    /// tick and the contract durations.
    pub fn flicker_at(&self, tick: u64) -> f32 {
        if self.flicker.is_empty() {
            return 1.0;
        }
        let frame = self
            .anim
            .as_ref()
            .map_or(0, |a| animation_frame(&a.durations, a.offset, tick));
        self.flicker[frame % self.flicker.len()]
    }
}

/// One light in view pixels (origin at the view's top-left, y down), as sent to
/// the GPU after packing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewLight {
    pub group: LightGroup,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub squash_y: f32,
    pub colour: [f32; 3],
    /// Intensity with the flicker multiplier applied.
    pub intensity: f32,
    pub glow_radius: f32,
    /// Glow strength with the flicker multiplier applied.
    pub glow_additive: f32,
}

/// The view rectangle in layout pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewRect {
    pub left: i32,
    pub top: i32,
    pub w: i32,
    pub h: i32,
}

/// Lights that reach a view, sorted by group (far, mid, scene), with the end
/// index of each emitter group (`far_end <= mid_end <= scene_end`).
#[derive(Debug, Clone, Default)]
pub struct GatheredLights {
    pub lights: Vec<ViewLight>,
    pub far_end: u32,
    pub mid_end: u32,
    pub scene_end: u32,
}

impl GatheredLights {
    /// Slice of `lights` that lights a group.
    pub fn range_for(&self, group: LightGroup) -> (u32, u32) {
        match group.emitter_group() {
            LightGroup::ParallaxFar => (0, self.far_end),
            LightGroup::ParallaxMid => (self.far_end, self.mid_end),
            _ => (self.mid_end, self.scene_end),
        }
    }
}

/// An actor contact shadow in view pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowEllipse {
    pub cx: f32,
    pub cy: f32,
    pub half_w: f32,
    pub half_h: f32,
    pub strength: f32,
}

/// Contact shadow under an actor: centred on the sprite box centre at the
/// walking line (`y` = walking line + 0.5, the reference model's `y1 + 0.5`),
/// as wide as the sprite box times `width_scale`.
pub fn contact_shadow(
    profile: &ContactShadowProfile,
    box_centre_x: f32,
    walking_line_y: f32,
    box_width: f32,
    view: ViewRect,
) -> ShadowEllipse {
    ShadowEllipse {
        cx: box_centre_x - view.left as f32,
        cy: walking_line_y + 0.5 - view.top as f32,
        half_w: box_width * profile.width_scale / 2.0,
        half_h: profile.half_height,
        strength: profile.strength,
    }
}

/// Everything static the pass needs, derived once at load.
#[derive(Debug, Clone)]
pub struct LightingScene {
    pub profile: LightingProfile,
    pub plan: SlotPlan,
    /// Emitters sorted by group (far, mid, scene), then by placement order.
    pub emitters: Vec<Emitter>,
    /// Largest emitter reach in px; used to include strip copies just off view.
    pub max_radius: f32,
}

impl LightingScene {
    /// Derive slots and emitters. Floor pools use the walking line of
    /// `kit` (read once here; lighting never writes back).
    pub fn build(
        contract: &Contract,
        layout: &Layout,
        kit: &EnvKit,
    ) -> Result<Option<Self>, LightingError> {
        let Some(profile) = layout.lighting.clone() else {
            return Ok(None);
        };
        profile.validate()?;
        let plan = plan_slots(contract, &profile)?;

        let mut emitters: Vec<Emitter> = Vec::new();
        let piece_def = |id: &str| contract.pieces.iter().find(|p| p.id == id);
        for piece in &kit.pieces {
            let Some(def) = piece_def(&piece.piece) else {
                continue;
            };
            let anim = (piece.frames.len() > 1).then(|| EmitterAnim {
                durations: piece.frames.iter().map(|(_, d)| *d).collect(),
                offset: piece.frame_offset,
            });
            for light in &def.lights {
                let mut emitter = derive_emitter(light, &def.id)?;
                emitter.x = piece.x as f32 + light.offset[0];
                emitter.y = piece.y as f32 + light.offset[1];
                if light.project.as_deref() == Some("walk_line") {
                    let Some(line) = kit.walking_line_at(emitter.x.floor() as i32) else {
                        // Reference model: no walking line under the light, no pool.
                        continue;
                    };
                    emitter.y = line as f32 + light.project_dy.unwrap_or(0.0);
                } else if let Some(other) = &light.project {
                    return invalid(format!("light {} projects to unknown {other:?}", light.id));
                }
                emitter.anim = anim.clone();
                emitters.push(emitter);
            }
        }
        for (index, strip) in kit.strips.iter().enumerate() {
            let Some(def) = piece_def(&strip.piece) else {
                continue;
            };
            for light in &def.lights {
                let mut emitter = derive_emitter(light, &def.id)?;
                emitter.x = light.offset[0];
                emitter.y = strip.y as f32 + light.offset[1];
                emitter.strip = Some(index);
                emitters.push(emitter);
            }
        }
        // Stable sort: placement order inside a group is kept.
        emitters.sort_by_key(|e| e.group);
        let max_radius = emitters.iter().map(|e| e.radius).fold(0.0, f32::max);
        Ok(Some(Self {
            profile,
            plan,
            emitters,
            max_radius,
        }))
    }

    /// Lights that reach `view` at presentation tick `tick`. Pure: reads the
    /// kit and the emitter list, returns a new list. At most `MAX_LIGHTS` are
    /// returned; when more reach the view the weakest are dropped (the second
    /// return value counts them).
    pub fn gather(&self, kit: &EnvKit, view: ViewRect, tick: u64) -> (GatheredLights, usize) {
        let mut found: Vec<ViewLight> = Vec::new();
        let margin = self.max_radius.ceil() as i32;
        for emitter in &self.emitters {
            let flicker = emitter.flicker_at(tick);
            let mut place = |layout_x: f32| {
                let x = layout_x - view.left as f32;
                let y = emitter.y - view.top as f32;
                let r = emitter.radius;
                if x + r < 0.0 || x - r > view.w as f32 || y + r < 0.0 || y - r > view.h as f32 {
                    return;
                }
                found.push(ViewLight {
                    group: emitter.group,
                    x,
                    y,
                    radius: r,
                    squash_y: emitter.squash_y,
                    colour: emitter.colour,
                    intensity: emitter.intensity * flicker,
                    glow_radius: emitter.glow_radius,
                    glow_additive: emitter.glow_additive * flicker,
                });
            };
            match emitter.strip {
                None => place(emitter.x),
                Some(index) => {
                    let strip = &kit.strips[index];
                    let start = parallax_start(strip.x_origin, view.left, strip.factor);
                    // Copies slightly outside the view still light its edge.
                    for copy_x in parallax_copies(
                        start,
                        strip.rect.w,
                        view.left - margin,
                        view.w + 2 * margin,
                    ) {
                        place(copy_x as f32 + emitter.x);
                    }
                }
            }
        }
        let mut dropped = 0;
        if found.len() > MAX_LIGHTS {
            dropped = found.len() - MAX_LIGHTS;
            // Keep the strongest; a stable sort keeps placement order for ties.
            let mut order: Vec<usize> = (0..found.len()).collect();
            order.sort_by(|&a, &b| {
                let strength = |l: &ViewLight| l.intensity * l.radius + l.glow_additive;
                strength(&found[b]).total_cmp(&strength(&found[a]))
            });
            order.truncate(MAX_LIGHTS);
            order.sort_unstable();
            found = order.into_iter().map(|i| found[i]).collect();
        }
        let count = |g: LightGroup| found.iter().filter(|l| l.group == g).count() as u32;
        let far_end = count(LightGroup::ParallaxFar);
        let mid_end = far_end + count(LightGroup::ParallaxMid);
        let scene_end = found.len() as u32;
        (
            GatheredLights {
                lights: found,
                far_end,
                mid_end,
                scene_end,
            },
            dropped,
        )
    }
}

fn derive_emitter(light: &LightDef, piece: &str) -> Result<Emitter, LightingError> {
    let Some(group) = LightGroup::parse(&light.group) else {
        return invalid(format!(
            "light {piece}/{} has unknown group {:?}",
            light.id, light.group
        ));
    };
    if !matches!(
        group,
        LightGroup::ParallaxFar | LightGroup::ParallaxMid | LightGroup::Scene
    ) {
        return invalid(format!(
            "light {piece}/{} lights group {:?}; the pass supports parallax_far, parallax_mid and scene emitters",
            light.id, light.group
        ));
    }
    let Some(colour) = hex_unit(&light.colour) else {
        return invalid(format!(
            "light {piece}/{} colour {:?} is not #rrggbb",
            light.id, light.colour
        ));
    };
    let what = |field: &str| format!("light {piece}/{} {field}", light.id);
    in_range(&what("radius"), light.radius, 1.0, 1024.0)?;
    in_range(&what("intensity"), light.intensity, 0.0, 4.0)?;
    let squash_y = light.squash_y.unwrap_or(1.0);
    in_range(&what("squash_y"), squash_y, 0.05, 1.0)?;
    let (glow_radius, glow_additive) = match &light.glow {
        Some(glow) => {
            in_range(&what("glow radius"), glow.radius, 1.0, 1024.0)?;
            in_range(&what("glow additive"), glow.additive, 0.0, 1.0)?;
            (glow.radius, glow.additive)
        }
        None => (0.0, 0.0),
    };
    let flicker = light.flicker.clone().unwrap_or_default();
    for k in &flicker {
        in_range(&what("flicker"), *k, 0.0, 2.0)?;
    }
    Ok(Emitter {
        id: format!("{piece}/{}", light.id),
        group,
        x: 0.0,
        y: 0.0,
        colour,
        radius: light.radius,
        intensity: light.intensity,
        squash_y,
        glow_radius,
        glow_additive,
        flicker,
        anim: None,
        strip: None,
    })
}

// --------------------------------------------------------- reference maths --
//
// The falloff curves and haze ramp of the Pillow reference model, in Rust, so
// tests can pin them and the WGSL port can be checked against them.

/// Pool falloff over `t2 = (d / radius)^2` (squash applied by the caller).
pub fn pool_falloff(t2: f32) -> f32 {
    if t2 >= 1.0 {
        0.0
    } else {
        (1.0 - t2).powi(2) / (1.0 + 5.0 * t2)
    }
}

/// Glow (additive halo) falloff.
pub fn glow_falloff(t2: f32) -> f32 {
    if t2 >= 1.0 {
        0.0
    } else {
        (1.0 - t2).powi(3) / (1.0 + 30.0 * t2)
    }
}

/// Haze mix factor at layout `y`: constant plus an optional smoothstep ramp.
pub fn haze_amount(haze: &HazeProfile, layout_y: f32) -> f32 {
    let Some(fog) = &haze.floor_fog else {
        return haze.amount;
    };
    let t = ((layout_y - fog.from_y) / (fog.to_y - fog.from_y)).clamp(0.0, 1.0);
    haze.amount + fog.amount * t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corridor::{Scenario, SceneSim};
    use crate::env_kit::{Contract, EnvKit, Layout};
    use std::path::PathBuf;

    const LAYOUT: &str = "assets/levels/floor01/corridor_abc_sample.json";

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn files() -> (Contract, Layout) {
        let layout_text = std::fs::read_to_string(root().join(LAYOUT)).unwrap();
        let layout: Layout = serde_json::from_str(&layout_text).unwrap();
        let contract_text = std::fs::read_to_string(root().join(&layout.kit_contract)).unwrap();
        (serde_json::from_str(&contract_text).unwrap(), layout)
    }

    fn kit() -> EnvKit {
        EnvKit::load(&root(), LAYOUT).unwrap()
    }

    fn scene(kit: &EnvKit) -> &LightingScene {
        kit.lighting.as_ref().expect("sample layout has lighting")
    }

    const VIEW: ViewRect = ViewRect {
        left: 328,
        top: 20,
        w: 640,
        h: 360,
    };

    #[test]
    fn sample_profile_parses_and_validates() {
        let (_, layout) = files();
        let profile = layout.lighting.expect("lighting profile");
        profile.validate().unwrap();
        assert!(profile.presentation_only);
        assert_eq!(profile.bloom.emissive_colours.len(), 10);
        assert_eq!(profile.group(LightGroup::Actors).light_scale, Some(0.3));
        assert_eq!(profile.group(LightGroup::Scene).ambient, [0.6, 0.62, 0.82]);
    }

    #[test]
    fn profile_validation_rejects_bad_data() {
        let (_, layout) = files();
        let good = layout.lighting.unwrap();
        let mut p = good.clone();
        p.presentation_only = false;
        assert!(p.validate().is_err());
        let mut p = good.clone();
        p.groups.remove("scene");
        assert!(p.validate().is_err());
        let mut p = good.clone();
        p.groups.get_mut("scene").unwrap().ambient[1] = 2.5;
        assert!(p.validate().is_err());
        let mut p = good.clone();
        p.groups
            .get_mut("parallax_far")
            .unwrap()
            .haze
            .as_mut()
            .unwrap()
            .colour = "blue".into();
        assert!(p.validate().is_err());
        let mut p = good.clone();
        p.bloom.emissive_colours.push("#12".into());
        assert!(p.validate().is_err());
        let mut p = good.clone();
        p.bloom.threshold = 300.0;
        assert!(p.validate().is_err());
        let mut p = good;
        p.actor_contact_shadow.as_mut().unwrap().strength = 1.5;
        assert!(p.validate().is_err());
    }

    #[test]
    fn profile_json_without_presentation_only_is_rejected_by_serde() {
        // `presentation_only` is required data, not a default.
        let text = r##"{"profile_id":"x","groups":{},"bloom":{"emissive_layers":[],"threshold":200,"intensity":1,"radius_virtual":2}}"##;
        assert!(serde_json::from_str::<LightingProfile>(text).is_err());
    }

    #[test]
    fn slot_plan_has_six_slots_in_draw_order() {
        let kit = kit();
        let plan = &scene(&kit).plan;
        let names: Vec<&str> = plan.slots.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "parallax_far",
                "parallax_mid",
                "backwall+backwall_decal+collision+structure",
                "prop",
                "actors",
                "foreground"
            ]
        );
        let emissive: Vec<bool> = plan.slots.iter().map(|s| s.emissive).collect();
        assert_eq!(emissive, [true, true, false, true, false, false]);
        assert_eq!(plan.actor_slot, 4);
        assert_eq!(plan.layer_slot["collision"], 2);
        assert_eq!(plan.layer_slot["prop"], 3);
        assert_eq!(plan.layer_slot["foreground"], 5);
    }

    #[test]
    fn emitters_come_from_pieces_and_strips_in_contract_terms() {
        let kit = kit();
        let s = scene(&kit);
        // 7 wall torches x (flame + floor pool), 4 arch rings x ... counted by id.
        let count = |needle: &str| s.emitters.iter().filter(|e| e.id.ends_with(needle)).count();
        assert_eq!(count("torch_wall/flame"), 7);
        assert_eq!(count("torch_wall/floor_pool"), 7);
        assert_eq!(count("gallery_mid/mid_torch"), 1);
        assert_eq!(count("gallery_far/far_torch_0"), 1);
        // Unlit sconces carry no light.
        assert_eq!(count("torch_wall_unlit/flame"), 0);
        // Sorted by group: far, mid, scene.
        let groups: Vec<LightGroup> = s.emitters.iter().map(|e| e.group).collect();
        assert!(groups.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn piece_lights_use_piece_offset_and_the_y_down_layout() {
        let kit = kit();
        let s = scene(&kit);
        // torch_wall at (128, 160), flame offset [8, 18]: layout (136, 178).
        let flame = s
            .emitters
            .iter()
            .find(|e| e.id == "torch_wall/flame" && e.x == 136.0)
            .expect("flame of the first torch");
        assert_eq!(flame.y, 178.0);
        assert_eq!(flame.radius, 250.0);
        assert_eq!(flame.flicker, [1.0, 0.93, 1.07, 0.97]);
        assert_eq!(flame.group, LightGroup::Scene);
        // Gathered into view pixels: x - left, y - top (still y down).
        let (g, dropped) = s.gather(&kit, VIEW, 0);
        assert_eq!(dropped, 0);
        let found = g
            .lights
            .iter()
            .find(|l| l.group == LightGroup::Scene && l.x == 136.0 - 328.0 && l.y == 178.0 - 20.0);
        assert!(found.is_some(), "first torch flame sits at its view pixel");
    }

    #[test]
    fn floor_pools_project_onto_the_walking_line_of_their_column() {
        let kit = kit();
        let s = scene(&kit);
        for e in s
            .emitters
            .iter()
            .filter(|e| e.id == "torch_wall/floor_pool")
        {
            let line = kit
                .walking_line_at(e.x.floor() as i32)
                .expect("every torch stands over floor");
            // project_dy is -6 in the contract; the base corridor walks at 308.
            assert_eq!(e.y, line as f32 - 6.0, "pool of torch at x {}", e.x);
            assert_eq!(e.squash_y, 0.32);
        }
        // At least one pool sits over the base line and one over a platform.
        let ys: Vec<f32> = s
            .emitters
            .iter()
            .filter(|e| e.id == "torch_wall/floor_pool")
            .map(|e| e.y)
            .collect();
        assert!(ys.contains(&302.0));
    }

    #[test]
    fn flicker_is_a_pure_function_of_tick_and_contract_durations() {
        let kit = kit();
        let s = scene(&kit);
        let flame = s
            .emitters
            .iter()
            .find(|e| e.id == "torch_wall/flame")
            .unwrap();
        let again = flame.clone();
        // Durations 120/140/100/130 ms at 60 Hz, offset 0: frame boundaries at
        // ticks 8 (133 ms), 17, 23, 31.
        let table: Vec<f32> = (0..40).map(|t| flame.flicker_at(t)).collect();
        let repeat: Vec<f32> = (0..40).map(|t| again.flicker_at(t)).collect();
        assert_eq!(table, repeat);
        assert_eq!(flame.flicker_at(0), 1.0);
        assert!(table.contains(&0.93));
        assert!(table.contains(&1.07));
        assert!(table.contains(&0.97));
        // Period is the 490 ms cycle: tick t and t + 294 (490 ms * 60 / 1000 * 10) agree.
        assert_eq!(flame.flicker_at(7), flame.flicker_at(7 + 294));
        // Emitters without a flicker list never change.
        let pool = s
            .emitters
            .iter()
            .find(|e| e.id.ends_with("gallery_mid/mid_torch"))
            .unwrap();
        assert_eq!(pool.flicker_at(0), 1.0);
        assert_eq!(pool.flicker_at(123), 1.0);
    }

    #[test]
    fn gather_is_deterministic_and_bounded_for_every_camera_position() {
        let kit = kit();
        let s = scene(&kit);
        let mut worst = 0;
        for (view_h, top) in [(360, 20), (400, 0)] {
            for left in (0..=(kit.width - 640)).step_by(8) {
                let view = ViewRect {
                    left,
                    top,
                    w: 640,
                    h: view_h,
                };
                for tick in [0u64, 13, 200] {
                    let (a, dropped) = s.gather(&kit, view, tick);
                    let (b, _) = s.gather(&kit, view, tick);
                    assert_eq!(a.lights, b.lights);
                    assert_eq!(dropped, 0, "view left {left} drops lights");
                    assert!(a.far_end <= a.mid_end && a.mid_end <= a.scene_end);
                    assert_eq!(a.scene_end as usize, a.lights.len());
                    worst = worst.max(a.lights.len());
                }
            }
        }
        assert!(
            worst > 5 && worst <= MAX_LIGHTS,
            "worst case {worst} lights"
        );
    }

    fn far_light_xs(kit: &EnvKit, left: i32) -> Vec<f32> {
        let view = ViewRect {
            left,
            top: 20,
            w: 640,
            h: 360,
        };
        scene(kit)
            .gather(kit, view, 0)
            .0
            .lights
            .iter()
            .filter(|l| l.group == LightGroup::ParallaxFar)
            .map(|l| l.x)
            .collect()
    }

    #[test]
    fn parallax_strip_lights_repeat_with_the_strip_and_move_with_it() {
        let kit = kit();
        let a = far_light_xs(&kit, 328);
        let b = far_light_xs(&kit, 400);
        assert!(
            a.len() >= 3 && b.len() >= 3,
            "far torches repeat across copies"
        );
        // Strip lights keep the strip's spacing (the 44 px torch pitch) ...
        assert!(a.windows(2).any(|w| (w[1] - w[0] - 44.0).abs() < 0.5));
        // ... and a 72 px camera move shifts them by 72 * 0.35 = ~25 px
        // on screen (whole-pixel snapped), not by 72.
        let shifted = a
            .iter()
            .any(|x| b.iter().any(|y| (x - y - 25.2).abs() <= 1.5));
        assert!(shifted, "{a:?} vs {b:?}");
    }

    #[test]
    fn lights_far_outside_the_view_are_culled() {
        let kit = kit();
        let s = scene(&kit);
        let (g, _) = s.gather(
            &kit,
            ViewRect {
                left: 0,
                top: 20,
                w: 640,
                h: 360,
            },
            0,
        );
        // The last torch (x ~1904) is nowhere near a view at x 0..640.
        assert!(g.lights.iter().all(|l| l.x < 640.0 + 300.0));
    }

    #[test]
    fn contact_shadow_follows_the_reference_geometry() {
        let (_, layout) = files();
        let profile = layout.lighting.unwrap().actor_contact_shadow.unwrap();
        let shadow = contact_shadow(&profile, 392.5, 308.0, 36.0, VIEW);
        assert_eq!(shadow.cx, 392.5 - 328.0);
        assert_eq!(shadow.cy, 308.0 + 0.5 - 20.0);
        assert!((shadow.half_w - 36.0 * 1.3 / 2.0).abs() < 1e-5);
        assert_eq!((shadow.half_h, shadow.strength), (8.0, 0.7));
    }

    #[test]
    fn falloff_and_haze_match_the_reference_curves() {
        assert_eq!(pool_falloff(0.0), 1.0);
        assert_eq!(pool_falloff(1.0), 0.0);
        assert!((pool_falloff(0.25) - 0.5625 / 2.25).abs() < 1e-6);
        assert_eq!(glow_falloff(0.0), 1.0);
        assert!((glow_falloff(0.25) - 0.421875 / 8.5).abs() < 1e-6);
        let (_, layout) = files();
        let haze = layout.lighting.unwrap().groups["parallax_far"]
            .haze
            .clone()
            .unwrap();
        assert_eq!(haze_amount(&haze, 100.0), 0.26);
        assert!((haze_amount(&haze, 288.0) - 0.56).abs() < 1e-6);
        let mid = haze_amount(&haze, 267.0);
        assert!((mid - (0.26 + 0.3 * 0.5)).abs() < 1e-6);
    }

    #[test]
    fn hex_colours_parse_to_unit_floats() {
        assert_eq!(parse_hex("#ffbd7a"), Some([255, 189, 122]));
        assert_eq!(parse_hex("ffbd7a"), None);
        assert_eq!(parse_hex("#ffbd7"), None);
        assert_eq!(parse_hex("#gg0000"), None);
        let c = hex_unit("#ff8000").unwrap();
        assert_eq!(c[0], 1.0);
        assert!((c[1] - 128.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn lighting_never_changes_the_simulation_state() {
        // The same scripted run with lighting derived every tick (gather on the
        // camera view, as the renderer does) and with no lighting at all ends
        // in the same state hash at every checkpoint.
        let kit = kit();
        let s = scene(&kit);
        let mut plain = SceneSim::new(&kit, 360).unwrap();
        let mut lit = SceneSim::new(&kit, 360).unwrap();
        for tick in 0..420u64 {
            let raw = Scenario::WalkRight.input(&plain.world);
            plain.step(raw);
            let raw = Scenario::WalkRight.input(&lit.world);
            lit.step(raw);
            let view = ViewRect {
                left: lit.camera.left(),
                top: lit.camera.top(),
                w: 640,
                h: 360,
            };
            let (gathered, _) = s.gather(&kit, view, lit.world.tick);
            assert!(!gathered.lights.is_empty());
            if tick % 60 == 0 {
                assert_eq!(plain.state_hash(), lit.state_hash(), "tick {tick}");
                assert_eq!(plain.state_line(), lit.state_line());
            }
        }
        assert_eq!(plain.state_hash(), lit.state_hash());
    }

    #[test]
    fn missing_lighting_block_means_no_lighting_scene() {
        let (contract, mut layout) = files();
        layout.lighting = None;
        let kit = EnvKit::build(&contract, &layout).unwrap();
        assert!(kit.lighting.is_none());
    }

    #[test]
    fn a_non_presentation_profile_fails_the_load() {
        let (contract, mut layout) = files();
        layout.lighting.as_mut().unwrap().presentation_only = false;
        let error = EnvKit::build(&contract, &layout).unwrap_err().to_string();
        assert!(error.contains("presentation_only"), "{error}");
    }
}
