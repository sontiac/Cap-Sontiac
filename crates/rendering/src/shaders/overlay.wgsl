struct Overlay {
    rect: vec4<f32>,
    opacity: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
};

@group(0) @binding(0) var<uniform> overlay: Overlay;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var out: VertexOutput;
    let u = f32(idx & 1u);
    let v = f32((idx >> 1u) & 1u);
    let x = mix(overlay.rect.x, overlay.rect.z, u);
    let y = mix(overlay.rect.w, overlay.rect.y, v);
    out.uv = vec2<f32>(u, v);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, in.uv);
    return vec4<f32>(c.rgb, c.a * overlay.opacity);
}
