@group(0) @binding(4) var cloud_noise: texture_2d<f32>;
@group(0) @binding(5) var cloud_sampler: sampler;

struct SkyOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) clip_xy: vec2<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) index: u32) -> SkyOutput {
    var output: SkyOutput;
    let xy = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    output.position = vec4<f32>(xy[index], 0.0, 1.0);
    output.clip_xy = xy[index];
    return output;
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn cloud_opacity(direction: vec3<f32>) -> f32 {
    if direction.y < 0.08 {
        return 0.0;
    }
    // The noise is sampled on a distant sky plane. Camera movement gives a
    // little parallax, while the independent clock keeps cloud drift continuous
    // when the 600-second day cycle wraps.
    let distance = 650.0 / max(direction.y, 0.12);
    let wind = vec2<f32>(0.000040, 0.000017) * uniforms.camera_position.w;
    let uv = (uniforms.camera_position.xz + direction.xz * distance) / 2048.0 + wind;
    let noise = textureSampleLevel(cloud_noise, cloud_sampler, uv, 0.0).rg;
    let shape = noise.r * 0.76 + noise.g * 0.24;
    return smoothstep(0.56, 0.70, shape)
         * smoothstep(0.08, 0.24, direction.y) * 0.58;
}

fn star_coverage(direction: vec3<f32>, pixel_width: f32) -> f32 {
    // A star is an angular disc on the celestial sphere. Keep its center away
    // from cell edges and filter its angular footprint by the screen pixel.
    // This gives small stars stable coverage as the view rotates.
    let spherical = vec2<f32>(atan2(direction.z, direction.x) * (384.0 / 6.2831853),
                              asin(direction.y) * 115.0);
    let cell = floor(spherical);
    let wrapped_cell = vec2<f32>(cell.x - floor(cell.x / 384.0) * 384.0, cell.y);
    if hash2(wrapped_cell) <= 0.92 {
        return 0.0;
    }
    let center = vec2<f32>(hash2(wrapped_cell + 19.1), hash2(wrapped_cell + 71.7)) * 0.5 + 0.25;
    let offset = spherical - cell - center;
    let angular_offset = offset * vec2<f32>(sqrt(max(1.0 - direction.y * direction.y, 0.0))
                                            * (6.2831853 / 384.0),
                                            1.0 / 115.0);
    let radius = 0.0012;
    let half_pixel = pixel_width * 0.5;
    return 1.0 - smoothstep(max(radius - half_pixel, 0.0), radius + half_pixel,
                            length(angular_offset));
}

@fragment
fn fs_sky(input: SkyOutput) -> @location(0) vec4<f32> {
    let point = uniforms.inverse_sky_view_projection * vec4<f32>(input.clip_xy, 1.0, 1.0);
    let direction = normalize(point.xyz);
    let pixel_width = length(fwidth(direction));
    var color = sky_color(direction);
    if direction.y > 0.0 {
        let cloud = cloud_opacity(direction);
        let daylight = smoothstep(-0.12, 0.12, uniforms.sun_direction.y);
        let toward_sun = max(dot(direction, uniforms.sun_direction.xyz), 0.0);
        let low_sun = 1.0 - smoothstep(0.08, 0.48, uniforms.sun_direction.y);
        let cloud_base = mix(vec3<f32>(0.18, 0.23, 0.32), vec3<f32>(0.96, 0.97, 0.98), daylight);
        let cloud_color = mix(cloud_base, vec3<f32>(1.0, 0.73, 0.56),
                              low_sun * pow(toward_sun, 4.0) * daylight);
        color = mix(color, cloud_color, cloud * (0.28 + 0.72 * daylight));
        let sun_angle = dot(direction, uniforms.sun_direction.xyz);
        let sun_disc = smoothstep(0.9996, 0.99985, sun_angle);
        let sun_halo = pow(max(sun_angle, 0.0), 128.0) * 0.08;
        color += vec3<f32>(1.0, 0.82, 0.61) * (sun_disc + sun_halo)
               * uniforms.sun_light.a * (1.0 - cloud * 0.75);
        let moon_angle = dot(direction, uniforms.moon_direction.xyz);
        let moon_disc = smoothstep(0.99945, 0.9997, moon_angle);
        color += vec3<f32>(0.77, 0.84, 1.0) * moon_disc * uniforms.moon_light.a * 1.7
               * (1.0 - cloud * 0.75);
        let night = 1.0 - smoothstep(-0.15, 0.05, uniforms.sun_direction.y);
        if night > 0.0 {
            color += vec3<f32>(0.75, 0.84, 1.0)
                   * star_coverage(direction, pixel_width) * night * (1.0 - cloud);
        }
    }
    return vec4<f32>(color, 1.0);
}
