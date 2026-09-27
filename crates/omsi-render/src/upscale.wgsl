// Render scale: the 3D picture, drawn smaller than the window, scaled up to it. A
// Catmull-Rom filter (nine bilinear taps) keeps edges and texture detail far better than a
// plain bilinear stretch; its overshoot is clamped to the neighbouring texels so that
// edges do not ring, and a contrast-adaptive sharpening pass (after AMD's CAS) brings back
// what the smaller picture lost, most where the picture is flat and least on hard edges.
struct Params {
    // xy: size of the source picture, z: sharpening 0..1, w unused
    src: vec4<f32>,
};
@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var t_src: texture_2d<f32>;
@group(0) @binding(2) var s_src: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    let x = f32(i32(i & 1u) * 4 - 1);
    let y = f32(i32(i >> 1u) * 4 - 1);
    var out: VsOut;
    out.clip = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, (1.0 - y) * 0.5);
    return out;
}

fn tap(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(t_src, s_src, uv, 0.0).rgb;
}

fn catmull_rom(uv: vec2<f32>, size: vec2<f32>) -> vec3<f32> {
    let pos = uv * size;
    let t1 = floor(pos - 0.5) + 0.5;
    let f = pos - t1;
    let w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    let w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    let w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    let w3 = f * f * (-0.5 + 0.5 * f);
    // the two middle taps become one bilinear tap between them
    let w12 = w1 + w2;
    let p0 = (t1 - 1.0) / size;
    let p3 = (t1 + 2.0) / size;
    let p12 = (t1 + w2 / w12) / size;
    var c = tap(vec2<f32>(p0.x, p0.y)) * w0.x * w0.y;
    c = c + tap(vec2<f32>(p12.x, p0.y)) * w12.x * w0.y;
    c = c + tap(vec2<f32>(p3.x, p0.y)) * w3.x * w0.y;
    c = c + tap(vec2<f32>(p0.x, p12.y)) * w0.x * w12.y;
    c = c + tap(vec2<f32>(p12.x, p12.y)) * w12.x * w12.y;
    c = c + tap(vec2<f32>(p3.x, p12.y)) * w3.x * w12.y;
    c = c + tap(vec2<f32>(p0.x, p3.y)) * w0.x * w3.y;
    c = c + tap(vec2<f32>(p12.x, p3.y)) * w12.x * w3.y;
    c = c + tap(vec2<f32>(p3.x, p3.y)) * w3.x * w3.y;
    return c;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let size = p.src.xy;
    let px = 1.0 / size;
    // the four texels around the point bound the filtered value (no ringing)
    let base = (floor(in.uv * size - 0.5) + 0.5) * px;
    let a = tap(base);
    let b = tap(base + vec2<f32>(px.x, 0.0));
    let c = tap(base + vec2<f32>(0.0, px.y));
    let d = tap(base + px);
    let lo = min(min(a, b), min(c, d));
    let hi = max(max(a, b), max(c, d));
    var col = clamp(catmull_rom(in.uv, size), lo, hi);
    // contrast-adaptive sharpening on the source's texel grid
    if (p.src.z > 0.0) {
        let n = tap(in.uv - vec2<f32>(0.0, px.y));
        let s = tap(in.uv + vec2<f32>(0.0, px.y));
        let w = tap(in.uv - vec2<f32>(px.x, 0.0));
        let e = tap(in.uv + vec2<f32>(px.x, 0.0));
        let mn = min(col, min(min(n, s), min(w, e)));
        let mx = max(col, max(max(n, s), max(w, e)));
        let amp = sqrt(clamp(min(mn, vec3<f32>(1.0) - mx) / max(mx, vec3<f32>(1e-4)), vec3<f32>(0.0), vec3<f32>(1.0)));
        let peak = -1.0 / mix(8.0, 5.0, p.src.z);
        let wgt = amp * peak;
        col = clamp((col + (n + s + w + e) * wgt) / (vec3<f32>(1.0) + 4.0 * wgt), vec3<f32>(0.0), vec3<f32>(1.0));
    }
    return vec4<f32>(col, 1.0);
}
