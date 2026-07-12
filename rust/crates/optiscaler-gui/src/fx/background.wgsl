// Animated background: slow-drifting radial glows in the accent hue over a
// dark base, with subtle film grain. Cheap by design (single fullscreen
// triangle, a handful of ALU ops — no textures, no loops).

struct Uniforms {
    time: f32,
    aspect: f32,
    intensity: f32,
    dark: f32,
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

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var uv = in.uv;
    uv.x = uv.x * u.aspect;
    let t = u.time * 0.05;

    // Base tones follow the theme
    let base_dark = vec3<f32>(0.055, 0.066, 0.086);
    let base_light = vec3<f32>(0.949, 0.957, 0.969);
    let base = mix(base_light, base_dark, u.dark);

    // Two slow-orbiting glow centers in accent hues
    let c1 = vec2<f32>(0.5 * u.aspect + 0.45 * u.aspect * cos(t),        0.30 + 0.25 * sin(t * 1.3));
    let c2 = vec2<f32>(0.5 * u.aspect + 0.40 * u.aspect * cos(t * 0.7 + 2.6), 0.75 + 0.20 * sin(t * 0.9 + 1.2));
    let g1 = exp(-3.5 * distance(uv, c1));
    let g2 = exp(-4.0 * distance(uv, c2));

    let accent1 = vec3<f32>(0.13, 0.32, 0.55); // deep blue
    let accent2 = vec3<f32>(0.16, 0.42, 0.62); // teal-blue
    var color = base + (accent1 * g1 + accent2 * g2) * 0.35 * u.intensity * mix(0.35, 1.0, u.dark);

    // Subtle grain so gradients don't band
    let grain = (hash(in.uv * 1024.0 + vec2<f32>(u.time, 0.0)) - 0.5) * 0.015;
    color += vec3<f32>(grain);

    return vec4<f32>(color, 1.0);
}
