@group(0) @binding(1) var atlas: texture_2d_array<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;
@group(0) @binding(3) var terrain_heights: texture_2d<i32>;
@group(0) @binding(6) var tree_spans: texture_2d<u32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) material: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) @interpolate(flat) material: u32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    var position = input.position;
    let is_plant = input.material == MAT_TALL_GRASS
                || input.material == MAT_DANDELION
                || input.material == MAT_OXEYE_DAISY
                || input.material == MAT_CORNFLOWER;
    if is_plant && input.uv.y < 0.5 {
        let wind = uniforms.camera_position.w * 0.55 + position.x * 0.23 + position.z * 0.31;
        position.x += sin(wind) * 0.045;
        position.z += cos(wind * 0.8) * 0.025;
    }
    output.clip_position = uniforms.view_projection * vec4<f32>(position, 1.0);
    output.world_position = position;
    output.normal = input.normal;
    output.uv = input.uv;
    output.material = input.material;
    return output;
}

// A sun ray walks through the exact one-block columns used to build the mesh.
// Its height rises continuously with distance, so an edge can only move when
// the sun or the actual terrain geometry moves. No light-space texel or cascade
// participates in the shadow length.
fn sun_visibility(world: vec3<f32>, starts_on_tree: bool) -> f32 {
    let sun = uniforms.sun_direction.xyz;
    if sun.y <= 0.0 || uniforms.shadow_params.x <= 0.0 {
        return 1.0;
    }
    let horizontal = length(sun.xz);
    if horizontal < 0.000001 {
        return 1.0;
    }
    let direction = sun.xz / horizontal;
    let rise = sun.y / horizontal;
    let dimensions = vec2<i32>(textureDimensions(terrain_heights));
    let origin = uniforms.shadow_origin.xy;
    let local = world.xz - vec2<f32>(origin);
    var cell = vec2<i32>(floor(local + vec2<f32>(0.5) + direction * 0.0001));
    let stride = select(vec2<i32>(-1), vec2<i32>(1), direction >= vec2<f32>(0.0));
    var t = 0.0;
    let max_t = min(max((uniforms.shadow_params.y - world.y) / rise, 0.0), 320.0);
    for (var step = 0; step < 1024; step = step + 1) {
        if any(cell < vec2<i32>(0)) || any(cell >= dimensions) || t > max_t {
            break;
        }
        // Mip 3 stores the highest block in each 8x8 tile. If even that top
        // lies below this rising ray, the whole tile is provably clear.
        let tile = cell / 8;
        let tile_top = f32(textureLoad(terrain_heights, tile, 3).x) + 0.5;
        if tile_top <= world.y + rise * t + 0.0005 {
            let tile_min = vec2<f32>(tile * 8) - vec2<f32>(0.5);
            let tile_edge = tile_min + select(vec2<f32>(0.0), vec2<f32>(8.0),
                                              direction >= vec2<f32>(0.0));
            let tile_exit = select((tile_edge - local) / direction,
                                    vec2<f32>(1e30), abs(direction) < vec2<f32>(0.000001));
            t = min(tile_exit.x, tile_exit.y) + 0.001;
            cell = vec2<i32>(floor(local + direction * t + vec2<f32>(0.5)));
            continue;
        }
        let top = f32(textureLoad(terrain_heights, cell, 0).x) + 0.5;
        if top > world.y + rise * t + 0.0005 {
            let edge = min(min(cell.x, cell.y),
                           min(dimensions.x - 1 - cell.x, dimensions.y - 1 - cell.y));
            return 1.0 - smoothstep(0.0, 32.0, f32(edge));
        }
        // The height mip includes tree canopies, but mip 0 contains only the
        // ground. The separate span preserves empty space below the leaves.
        // Ignore the first few metres for a ray starting on a tree so its own
        // canopy does not darken every leaf and trunk face.
        let packed_tree = textureLoad(tree_spans, cell, 0).x;
        if packed_tree != 0u && (!starts_on_tree || t > 3.0) {
            let tree_bottom = f32(i32(packed_tree & 0xffffu) - 32768) + 0.5;
            let tree_top = f32(i32(packed_tree >> 16u) - 32768) + 0.5;
            let ray_height = world.y + rise * t;
            if ray_height >= tree_bottom && ray_height <= tree_top {
                let edge = min(min(cell.x, cell.y),
                               min(dimensions.x - 1 - cell.x, dimensions.y - 1 - cell.y));
                return 1.0 - smoothstep(0.0, 32.0, f32(edge));
            }
        }
        let next_boundary = vec2<f32>(cell) + vec2<f32>(stride) * 0.5;
        let next_t = select((next_boundary - local) / direction,
                            vec2<f32>(1e30), abs(direction) < vec2<f32>(0.000001));
        if next_t.x < next_t.y {
            t = next_t.x + 0.001;
        } else {
            t = next_t.y + 0.001;
        }
        cell = vec2<i32>(floor(local + direction * t + vec2<f32>(0.5)));
    }
    return 1.0;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(atlas, atlas_sampler, input.uv, i32(input.material));
    if texel.a < 0.5 {
        discard;
    }
    let normal = normalize(input.normal);
    let distance = length(input.world_position - uniforms.camera_position.xyz);
    let sun_diffuse = max(dot(normal, uniforms.sun_direction.xyz), 0.0);
    let moon_diffuse = max(dot(normal, uniforms.moon_direction.xyz), 0.0);
    var sun_visible = 1.0;
    if sun_diffuse > 0.0 && uniforms.sun_light.a > 0.0 && distance < 320.0 {
        let starts_on_tree = input.material == MAT_SPRUCE_LOG
                          || input.material == MAT_SPRUCE_LOG_TOP
                          || input.material == MAT_SPRUCE_LEAVES;
        let shadow = sun_visibility(input.world_position, starts_on_tree);
        sun_visible = mix(shadow, 1.0, smoothstep(220.0, 320.0, distance));
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
    let view_direction = normalize(input.world_position - uniforms.camera_position.xyz);
    let fog = sky_color(view_direction);
    let visibility = 1.0 - smoothstep(260.0, 430.0, distance);
    return vec4<f32>(mix(fog, lit, visibility), texel.a);
}
