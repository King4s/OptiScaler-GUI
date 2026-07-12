// Animated backgrounds behind the UI, selected by u.style. All styles are
// cheap by design: single fullscreen triangle, no textures, at most three
// noise taps.
//   0 = Orbits    — slow-drifting radial glows (the original)
//   1 = Aurora    — sine-wave light curtains, green/purple
//   2 = Synthwave — perspective grid + horizon glow, magenta/cyan
//   3 = Nebula    — layered value-noise clouds, purple/blue

struct Uniforms {
    time: f32,
    aspect: f32,
    intensity: f32,
    dark: f32,
    style: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    // Fullscreen triangle
    var out: VsOut;
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi >> 1u) * 4 - 1);
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 0.5 - y * 0.5);
    return out;
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, s.x), mix(c, d, s.x), s.y);
}

// uv has x pre-scaled by aspect; raw = original 0..1 uv (for y bands).
fn style_orbits(uv: vec2<f32>) -> vec3<f32> {
    let t = u.time * 0.05;
    let c1 = vec2<f32>(0.5 * u.aspect + 0.45 * u.aspect * cos(t), 0.30 + 0.25 * sin(t * 1.3));
    let c2 = vec2<f32>(0.5 * u.aspect + 0.40 * u.aspect * cos(t * 0.7 + 2.6), 0.75 + 0.20 * sin(t * 0.9 + 1.2));
    let g1 = exp(-3.5 * distance(uv, c1));
    let g2 = exp(-4.0 * distance(uv, c2));
    let accent1 = vec3<f32>(0.13, 0.32, 0.55); // deep blue
    let accent2 = vec3<f32>(0.16, 0.42, 0.62); // teal-blue
    return (accent1 * g1 + accent2 * g2) * 0.35;
}

fn style_aurora(raw: vec2<f32>) -> vec3<f32> {
    let t = u.time * 0.08;
    let x = raw.x * u.aspect;
    let w1 = 0.12 * sin(x * 2.0 + t * 2.0) + 0.06 * sin(x * 4.3 - t * 1.4);
    let w2 = 0.10 * sin(x * 1.6 - t * 1.7 + 2.1) + 0.05 * sin(x * 5.1 + t);
    let band1 = exp(-14.0 * abs(raw.y - 0.32 - w1));
    let band2 = exp(-16.0 * abs(raw.y - 0.55 - w2));
    let green = vec3<f32>(0.08, 0.42, 0.30);
    let purple = vec3<f32>(0.26, 0.14, 0.46);
    // Curtains brighten toward the top of the window
    let sky = mix(1.0, 0.4, raw.y);
    return (green * band1 * 0.55 + purple * band2 * 0.50) * sky;
}

fn style_synthwave(raw: vec2<f32>) -> vec3<f32> {
    let horizon = 0.60;
    let magenta = vec3<f32>(0.48, 0.09, 0.38);
    let cyan = vec3<f32>(0.05, 0.38, 0.46);
    // Horizon glow on both sides
    var color = magenta * exp(-16.0 * abs(raw.y - horizon)) * 0.6;
    // Low sun above the horizon
    let sun = vec2<f32>((raw.x - 0.5) * u.aspect, (raw.y - (horizon - 0.14)) * 1.4);
    color += magenta * exp(-9.0 * length(sun)) * 0.45;
    // Perspective grid scrolling toward the viewer below the horizon
    let d = raw.y - horizon;
    if d > 0.0 {
        let pz = 1.0 / (d + 0.06);
        let gx = abs(fract((raw.x - 0.5) * u.aspect * pz * 0.55) - 0.5);
        let gy = abs(fract(pz * 0.45 - u.time * 0.35) - 0.5);
        let line = max(smoothstep(0.44, 0.5, gx), smoothstep(0.42, 0.5, gy));
        // Lines fade out right at the horizon where they'd alias
        color += cyan * line * 0.5 * (1.0 - exp(-d * 9.0));
    }
    return color;
}

fn style_nebula(raw: vec2<f32>) -> vec3<f32> {
    let t = u.time * 0.03;
    let p = vec2<f32>(raw.x * u.aspect, raw.y) * 3.0 + vec2<f32>(t, -t * 0.7);
    var n = value_noise(p) * 0.5;
    n += value_noise(p * 2.1 + vec2<f32>(5.2, 1.3)) * 0.25;
    n += value_noise(p * 4.3 + vec2<f32>(1.7, 9.2)) * 0.125;
    n = smoothstep(0.30, 0.80, n);
    let purple = vec3<f32>(0.33, 0.16, 0.52);
    let blue = vec3<f32>(0.10, 0.28, 0.56);
    let tint = mix(blue, purple, value_noise(raw * 1.5 + vec2<f32>(-t * 0.5, t * 0.3)));
    return tint * n * 0.55;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var uv = in.uv;
    uv.x = uv.x * u.aspect;

    // Base tones follow the theme
    let base_dark = vec3<f32>(0.055, 0.066, 0.086);
    let base_light = vec3<f32>(0.949, 0.957, 0.969);
    let base = mix(base_light, base_dark, u.dark);

    let style = u32(u.style + 0.5);
    var glow: vec3<f32>;
    if style == 1u {
        glow = style_aurora(in.uv);
    } else if style == 2u {
        glow = style_synthwave(in.uv);
    } else if style == 3u {
        glow = style_nebula(in.uv);
    } else {
        glow = style_orbits(uv);
    }

    // Light theme keeps the effects subtle
    var color = base + glow * u.intensity * mix(0.35, 1.0, u.dark);

    // Subtle grain so gradients don't band
    let grain = (hash(in.uv * 1024.0 + vec2<f32>(u.time, 0.0)) - 0.5) * 0.015;
    color += vec3<f32>(grain);

    return vec4<f32>(color, 1.0);
}
