// Floor 1 lighting pass (COMP-02): presentation only.
//
// One shader module, three fragment entry points, all run on a full-screen
// quad (see `light_pass.rs`):
//   emissive   virtual resolution: flame-coloured pixels that nothing nearer
//              hides, lit, then bright-passed (the bloom source);
//   blur       virtual resolution: one separable gaussian direction (the
//              direction comes from `params.blur`);
//   composite  output resolution: per-slot ambient + warm pools + shadow +
//              haze, additive glow, plus the bloom, from nearest-sampled
//              virtual-resolution slot textures.
//
// Colour maths follows the Pillow reference (`scripts/env_kit_light.py`): it
// multiplies and mixes *sRGB-encoded* values, so every slot texel is converted
// from the sampler's linear value to the encoded value first, and the final
// colour is converted back before it reaches the (sRGB) render target.

struct Light {
    // x, y: view px (y down); z: radius; w: squash_y
    pos: vec4<f32>,
    // rgb: colour 0..1 (encoded); w: intensity with flicker applied
    colour: vec4<f32>,
    // x: glow radius; y: glow strength with flicker applied
    glow: vec4<f32>,
}

struct Slot {
    // rgb: ambient multiplier; w: light_scale
    ambient: vec4<f32>,
    // rgb: haze colour 0..1 (encoded); w: constant haze amount
    haze: vec4<f32>,
    // x: fog from_y; y: fog to_y (layout y); z: fog amount; w: 1 when fog is on
    fog: vec4<f32>,
    // x, y: [start, end) range in `lights`; z: flags (1 emissive, 2 shadow, 4 parallax)
    info: vec4<u32>,
}

struct Params {
    // w, h of the virtual view; z: view top in layout px; w: slot count
    view: vec4<f32>,
    // x, y: target size in px; z: shadow count; w: shadow strength
    target_px: vec4<f32>,
    // x: bright-pass threshold 0..1; y: gain; z: blur sigma (virtual px); w: emissive colour count
    bloom: vec4<f32>,
    // x, y: blur step in texels
    blur: vec4<f32>,
    // x: far end; y: mid end; z: scene end of the light list; w: light count
    ranges: vec4<u32>,
    // cx, cy, half width, half height (view px)
    shadows: array<vec4<f32>, 8>,
    // rgb 0..255 atlas colours that feed bloom
    emissive: array<vec4<f32>, 16>,
    slots: array<Slot, 6>,
    lights: array<Light, 64>,
}

@group(2) @binding(0) var<uniform> params: Params;
@group(2) @binding(1) var slot0: texture_2d<f32>;
@group(2) @binding(2) var slot1: texture_2d<f32>;
@group(2) @binding(3) var slot2: texture_2d<f32>;
@group(2) @binding(4) var slot3: texture_2d<f32>;
@group(2) @binding(5) var slot4: texture_2d<f32>;
@group(2) @binding(6) var slot5: texture_2d<f32>;
@group(2) @binding(7) var aux: texture_2d<f32>;

struct FragIn {
    @builtin(position) pos: vec4<f32>,
}

const FLAG_EMISSIVE: u32 = 1u;
const FLAG_SHADOW: u32 = 2u;
const FLAG_PARALLAX: u32 = 4u;

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
}

fn from_srgb(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((max(c, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

fn load_slot(s: u32, p: vec2<i32>) -> vec4<f32> {
    switch s {
        case 0u: { return textureLoad(slot0, p, 0); }
        case 1u: { return textureLoad(slot1, p, 0); }
        case 2u: { return textureLoad(slot2, p, 0); }
        case 3u: { return textureLoad(slot3, p, 0); }
        case 4u: { return textureLoad(slot4, p, 0); }
        case 5u: { return textureLoad(slot5, p, 0); }
        default: { return vec4<f32>(0.0); }
    }
}

fn view_size() -> vec2<i32> {
    return vec2<i32>(i32(params.view.x), i32(params.view.y));
}

// Reference falloff curves (t2 is the squared normalised distance).
fn pool(t2: f32) -> f32 {
    if t2 >= 1.0 { return 0.0; }
    let a = 1.0 - t2;
    return a * a / (1.0 + 5.0 * t2);
}

fn glow_curve(t2: f32) -> f32 {
    if t2 >= 1.0 { return 0.0; }
    let a = 1.0 - t2;
    return a * a * a / (1.0 + 30.0 * t2);
}

fn shadow_amount(p: vec2<f32>) -> f32 {
    var s = 0.0;
    let n = u32(params.target_px.z);
    for (var i = 0u; i < n; i++) {
        let e = params.shadows[i];
        let d = (p - e.xy) / e.zw;
        s += pool(dot(d, d));
    }
    return clamp(s * params.target_px.w, 0.0, 1.0);
}

// Lit and hazed colour of one slot texel; `c` and the result are encoded sRGB 0..1.
fn shade_slot(s: u32, c: vec3<f32>, p: vec2<f32>) -> vec3<f32> {
    let slot = params.slots[s];
    var m = slot.ambient.rgb;
    let scale = slot.ambient.w;
    for (var i = slot.info.x; i < slot.info.y; i++) {
        let l = params.lights[i];
        let dx = (p.x - l.pos.x) / l.pos.z;
        let dy = (p.y - l.pos.y) / (l.pos.z * l.pos.w);
        let f = pool(dx * dx + dy * dy);
        m += scale * l.colour.w * f * l.colour.rgb;
    }
    if (slot.info.z & FLAG_SHADOW) != 0u {
        m *= 1.0 - shadow_amount(p);
    }
    var lit = c * m;
    var h = slot.haze.w;
    if slot.fog.w > 0.5 {
        let t = clamp((params.view.z + p.y - slot.fog.x) / (slot.fog.y - slot.fog.x), 0.0, 1.0);
        h += slot.fog.z * t * t * (3.0 - 2.0 * t);
    }
    if h > 0.0 {
        lit = lit * (1.0 - h) + slot.haze.rgb * h;
    }
    return clamp(lit, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn is_flame(c: vec3<f32>) -> bool {
    let v = round(c * 255.0);
    let n = u32(params.bloom.w);
    for (var i = 0u; i < n; i++) {
        let e = params.emissive[i].rgb;
        if all(abs(v - e) < vec3<f32>(0.5)) { return true; }
    }
    return false;
}

// Bloom source at one virtual pixel: lit flame colour of the nearest visible
// slot when that pixel is a flame of an emissive slot, black otherwise (every
// other opaque pixel occludes flames behind it, so nothing blooms through walls).
fn emissive_at(texel: vec2<i32>, p: vec2<f32>) -> vec3<f32> {
    var acc = vec3<f32>(0.0);
    var trans = 1.0;
    let count = u32(params.view.w);
    for (var i = count; i > 0u; i--) {
        let s = i - 1u;
        let t = load_slot(s, texel);
        if t.a > 0.0 {
            let c = to_srgb(t.rgb);
            var e = vec3<f32>(0.0);
            if (params.slots[s].info.z & FLAG_EMISSIVE) != 0u && is_flame(c) {
                e = shade_slot(s, c, p);
            }
            acc += trans * t.a * e;
            trans *= 1.0 - t.a;
            if trans < 0.001 { break; }
        }
    }
    return acc;
}

@fragment
fn emissive(in: FragIn) -> @location(0) vec4<f32> {
    let texel = clamp(vec2<i32>(floor(in.pos.xy)), vec2<i32>(0), view_size() - vec2<i32>(1));
    let p = vec2<f32>(texel) + 0.5;
    let e = emissive_at(texel, p);
    let b = clamp((e - params.bloom.x) * params.bloom.y / (1.0 - params.bloom.x), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(from_srgb(b), 1.0);
}

@fragment
fn blur(in: FragIn) -> @location(0) vec4<f32> {
    let c = vec2<i32>(floor(in.pos.xy));
    let size = view_size();
    let sigma = params.bloom.z;
    let radius = i32(ceil(sigma * 3.0));
    let dir = vec2<i32>(i32(params.blur.x), i32(params.blur.y));
    var acc = vec3<f32>(0.0);
    var wsum = 0.0;
    for (var o = -radius; o <= radius; o++) {
        let w = exp(-f32(o * o) / (2.0 * sigma * sigma));
        wsum += w;
        let q = c + dir * o;
        if q.x >= 0 && q.y >= 0 && q.x < size.x && q.y < size.y {
            acc += w * to_srgb(textureLoad(aux, q, 0).rgb);
        }
    }
    return vec4<f32>(from_srgb(acc / wsum), 1.0);
}

fn bloom_texel(q: vec2<i32>) -> vec3<f32> {
    let c = clamp(q, vec2<i32>(0), view_size() - vec2<i32>(1));
    return to_srgb(textureLoad(aux, c, 0).rgb);
}

// Bilinear sample of the (smooth) blurred bloom at virtual position p.
fn bloom_at(p: vec2<f32>) -> vec3<f32> {
    let q = p - 0.5;
    let i = vec2<i32>(floor(q));
    let f = q - floor(q);
    let a = mix(bloom_texel(i), bloom_texel(i + vec2<i32>(1, 0)), f.x);
    let b = mix(bloom_texel(i + vec2<i32>(0, 1)), bloom_texel(i + vec2<i32>(1, 1)), f.x);
    return mix(a, b, f.y);
}

@fragment
fn composite(in: FragIn) -> @location(0) vec4<f32> {
    // Output pixel centre in continuous virtual pixels. The slot textures are
    // read with integer coordinates (exact nearest-neighbour: no filtering, no
    // seams, at any window size); only the light maths is continuous.
    let k = params.target_px.xy / params.view.xy;
    let p = in.pos.xy / k;
    let texel = clamp(vec2<i32>(floor(p)), vec2<i32>(0), view_size() - vec2<i32>(1));

    var acc = vec3<f32>(0.0);
    var trans = 1.0;
    var covered = 0.0; // coverage by non-parallax slots (hides gallery halos)
    let count = u32(params.view.w);
    for (var i = count; i > 0u; i--) {
        let s = i - 1u;
        let t = load_slot(s, texel);
        if t.a > 0.0 {
            let lit = shade_slot(s, to_srgb(t.rgb), p);
            acc += trans * t.a * lit;
            if (params.slots[s].info.z & FLAG_PARALLAX) == 0u {
                covered = max(covered, t.a);
            }
            trans *= 1.0 - t.a;
            if trans < 0.001 { break; }
        }
    }

    // Additive halos: gallery torches only where the gallery shows, scene
    // flames everywhere.
    var halo = vec3<f32>(0.0);
    let far_mid_end = params.ranges.y;
    let scene_end = params.ranges.z;
    for (var i = 0u; i < scene_end; i++) {
        let l = params.lights[i];
        if l.glow.y <= 0.0 { continue; }
        let d = (p - l.pos.xy) / l.glow.x;
        var g = l.glow.y * glow_curve(dot(d, d));
        if i < far_mid_end { g *= 1.0 - covered; }
        halo += g * l.colour.rgb;
    }

    let out_c = clamp(acc + halo + bloom_at(p), vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(from_srgb(out_c), 1.0);
}
