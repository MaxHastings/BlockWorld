@group(0) @binding(1) var atlas: texture_2d_array<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;
@group(0) @binding(3) var shadow_maps: texture_depth_2d_array;
@group(0) @binding(4) var shadow_sampler: sampler_comparison;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) material: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) relative_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) material: u32,
};

struct ShadowVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) material: u32,
};

// Shared deformation and world-to-camera translation for both passes.
fn deformed_position(input: VertexInput) -> vec3<f32> {
    var position = input.position;
    if MATERIAL_PROPERTIES[input.material].z != 0u && input.uv.y < 0.5 {
        let wind = uniforms.camera_position.w * 0.55 + position.x * 0.23 + position.z * 0.31;
        position.x += sin(wind) * WIND_AMPLITUDE.x;
        position.z += cos(wind * 0.8) * WIND_AMPLITUDE.y;
    }
    return position - uniforms.camera_position.xyz;
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    let position = deformed_position(input);
    output.clip_position = uniforms.view_projection * vec4<f32>(position, 1.0);
    output.relative_position = position;
    output.normal = input.normal;
    output.uv = input.uv;
    output.material = input.material;
    return output;
}

@vertex
fn vs_shadow(input: VertexInput, @builtin(instance_index) cascade: u32) -> ShadowVertexOutput {
    var output: ShadowVertexOutput;
    let position = deformed_position(input);
    output.clip_position = uniforms.shadow_view_projection[cascade]
        * vec4<f32>(position, 1.0);
    output.uv = input.uv;
    output.material = input.material;
    return output;
}

fn shadow_map_visibility(relative: vec3<f32>, normal: vec3<f32>, cascade: i32) -> f32 {
    let clip = uniforms.shadow_view_projection[cascade]
        * vec4<f32>(relative, 1.0);
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if clip.w <= 0.0 || any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))
        || ndc.z < 0.0 || ndc.z > 1.0 {
        return 1.0;
    }

    let scale = uniforms.shadow_scale[cascade];
    let matrix = uniforms.shadow_view_projection[cascade];
    let right = normalize(vec3<f32>(matrix[0].x, matrix[1].x, matrix[2].x));
    let up = normalize(vec3<f32>(matrix[0].y, matrix[1].y, matrix[2].y));
    let n_dot_l = dot(normal, uniforms.sun_direction.xyz);
    let incidence = abs(n_dot_l);
    // Receiver-plane depth slope becomes ill-conditioned when a surface is
    // edge-on to the light. Clamp its denominator and fade the correction and
    // resulting shadow smoothly; direct diffuse is also vanishing in this case.
    let grazing_fade = smoothstep(0.025, 0.10, incidence);
    let safe_incidence = max(incidence, 0.10);
    let signed_incidence = select(-safe_incidence, safe_incidence, n_dot_l >= 0.0);

    // Exact depth change on the receiver's geometric plane, per map texel.
    // UV y points down; light-space y points up. Both passes use [0,1] depth.
    let gradient = vec2<f32>(dot(normal, right), -dot(normal, up))
        * (scale.x * scale.y / signed_incidence);
    // Only numerical tolerance remains: 1% of a world texel converted by the
    // actual depth span, or eight float32 epsilons in normalized depth.
    let bias = max(0.01 * scale.x * scale.y, 8.0 * 1.1920929e-7);
    let pixel = uv / scale.z - vec2<f32>(0.5);
    let base = floor(pixel);
    var visibility = 0.0;
    // Separable radius-2 tent, evaluated at exact texel centers. Nearest
    // comparison is deliberate: each tap has its own plane-corrected depth.
    // The weights vary continuously, including when base crosses a texel.
    for (var y = -1; y <= 2; y = y + 1) {
        for (var x = -1; x <= 2; x = x + 1) {
            let tap = base + vec2<f32>(f32(x), f32(y));
            let delta = tap - pixel;
            let weight = max(vec2<f32>(0.0), vec2<f32>(scale.w) - abs(delta));
            let depth = ndc.z + dot(gradient, delta) * grazing_fade - bias;
            visibility += weight.x * weight.y * textureSampleCompareLevel(
                shadow_maps, shadow_sampler, (tap + vec2<f32>(0.5)) * scale.z,
                cascade, depth,
            );
        }
    }
    return mix(1.0, visibility / 16.0, grazing_fade);
}

fn sun_visibility(relative: vec3<f32>, normal: vec3<f32>) -> f32 {
    let distance = length(relative);
    if uniforms.sun_direction.y <= 0.0 || uniforms.shadow_params.y <= 0.0
        || distance >= uniforms.shadow_params.z {
        return 1.0;
    }
    var visibility: f32;
    if distance < uniforms.shadow_params.x - uniforms.shadow_params.w {
        visibility = shadow_map_visibility(relative, normal, 0);
    } else if distance > uniforms.shadow_params.x + uniforms.shadow_params.w {
        visibility = shadow_map_visibility(relative, normal, 1);
    } else {
        let near_visibility = shadow_map_visibility(relative, normal, 0);
        let far_visibility = shadow_map_visibility(relative, normal, 1);
        let cascade_blend = smoothstep(uniforms.shadow_params.x - uniforms.shadow_params.w,
                                       uniforms.shadow_params.x + uniforms.shadow_params.w, distance);
        visibility = mix(near_visibility, far_visibility, cascade_blend);
    }
    let distance_fade = 1.0 - smoothstep(uniforms.shadow_params.z - uniforms.shadow_fade.x,
                                         uniforms.shadow_params.z, distance);
    return mix(1.0, visibility, uniforms.shadow_params.y * distance_fade);
}

fn surface_texel(uv: vec2<f32>, material: u32) -> vec4<f32> {
    let texel = textureSample(atlas, atlas_sampler, uv, i32(material));
    if MATERIAL_PROPERTIES[material].x != 0u && texel.a < 0.5 { discard; }
    return texel;
}

@fragment
fn fs_shadow(input: ShadowVertexOutput) {
    // Sample before the material-policy branch so implicit mip derivatives are
    // evaluated in uniform control flow.
    _ = surface_texel(input.uv, input.material);
    if MATERIAL_PROPERTIES[input.material].y == 0u { discard; }
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Evaluate derivatives before divergent alpha/lighting branches. This is
    // the actual deformed triangle plane, not a potentially smoothed normal.
    let receiver_normal = normalize(cross(dpdx(input.relative_position), dpdy(input.relative_position)));
    let texel = surface_texel(input.uv, input.material);
    let normal = normalize(input.normal);
    let distance = length(input.relative_position);
    let sun_diffuse = max(dot(normal, uniforms.sun_direction.xyz), 0.0);
    let moon_diffuse = max(dot(normal, uniforms.moon_direction.xyz), 0.0);
    var sun_visible = 1.0;
    if sun_diffuse > 0.0 && uniforms.sun_light.a > 0.0
        && distance < uniforms.shadow_params.z {
        sun_visible = sun_visibility(input.relative_position, receiver_normal);
    }
    let sun = uniforms.sun_light.rgb * uniforms.sun_light.a * sun_diffuse
            * sun_visible;
    let moon = uniforms.moon_light.rgb * uniforms.moon_light.a * moon_diffuse;
    var albedo = texel.rgb;
    if input.material == MAT_SPRUCE_LEAVES {
        // Spruce leaves are grayscale so they can take on foliage color.
        albedo *= vec3<f32>(0.43, 0.95, 0.42);
    }
    let lit = albedo * (uniforms.ambient.rgb + sun + moon);
    let view_direction = normalize(input.relative_position);
    let fog = sky_color(view_direction);
    let visibility = 1.0 - smoothstep(260.0, 430.0, distance);
    return vec4<f32>(mix(fog, lit, visibility), texel.a);
}
