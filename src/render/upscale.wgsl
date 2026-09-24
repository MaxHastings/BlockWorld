@group(0) @binding(0) var low_resolution_frame: texture_2d<f32>;
@group(0) @binding(1) var frame_sampler: sampler;

struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_upscale(@builtin(vertex_index) index: u32) -> Output {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var output: Output;
    output.position = vec4<f32>(positions[index], 0.0, 1.0);
    output.uv = vec2<f32>(positions[index].x * 0.5 + 0.5, 0.5 - positions[index].y * 0.5);
    return output;
}

@fragment
fn fs_upscale(input: Output) -> @location(0) vec4<f32> {
    return textureSample(low_resolution_frame, frame_sampler, input.uv);
}
