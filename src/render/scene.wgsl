struct Uniforms {
    view_projection: mat4x4<f32>,
    inverse_sky_view_projection: mat4x4<f32>,
    sun_direction: vec4<f32>,
    moon_direction: vec4<f32>,
    sky_horizon: vec4<f32>,
    sky_zenith: vec4<f32>,
    sun_light: vec4<f32>,
    moon_light: vec4<f32>,
    ambient: vec4<f32>,
    camera_position: vec4<f32>,
    shadow_view_projection: array<mat4x4<f32>, 2>,
    shadow_scale: array<vec4<f32>, 2>,
    shadow_params: vec4<f32>,
    shadow_fade: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

fn horizon_color(direction: vec3<f32>) -> vec3<f32> {
    let sun_side = pow(max(dot(normalize(vec3<f32>(direction.x + 0.00001, 0.0, direction.z)),
                               normalize(vec3<f32>(uniforms.sun_direction.x, 0.0, uniforms.sun_direction.z))), 0.0), 5.0);
    let twilight = (1.0 - smoothstep(-0.02, 0.40, uniforms.sun_direction.y))
                 * smoothstep(-0.25, 0.02, uniforms.sun_direction.y);
    let glow = sun_side * twilight * (1.0 - smoothstep(0.0, 0.35, abs(direction.y)));
    return mix(uniforms.sky_horizon.rgb, vec3<f32>(1.0, 0.38, 0.17), glow * 0.78);
}

fn sky_color(direction: vec3<f32>) -> vec3<f32> {
    let height = smoothstep(-0.04, 0.8, max(direction.y, 0.0));
    let base = mix(horizon_color(direction), uniforms.sky_zenith.rgb, height);
    let toward_sun = max(dot(direction, uniforms.sun_direction.xyz), 0.0);
    let low_sun = 1.0 - smoothstep(0.08, 0.48, uniforms.sun_direction.y);
    let glow_color = mix(vec3<f32>(1.0, 0.86, 0.66), vec3<f32>(1.0, 0.53, 0.30), low_sun);
    let glow = (0.09 * pow(toward_sun, 12.0) + 0.22 * pow(toward_sun, 80.0))
             * smoothstep(-0.12, 0.10, uniforms.sun_direction.y);
    return mix(base, glow_color, glow);
}
