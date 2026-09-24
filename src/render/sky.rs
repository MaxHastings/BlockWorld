const NOISE_SIZE: u32 = 256;

pub(super) fn cloud_noise(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> (wgpu::TextureView, wgpu::Sampler) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("tileable cloud noise"),
        size: wgpu::Extent3d {
            width: NOISE_SIZE,
            height: NOISE_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rg8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut pixels = Vec::with_capacity((NOISE_SIZE * NOISE_SIZE * 2) as usize);
    for z in 0..NOISE_SIZE {
        for x in 0..NOISE_SIZE {
            let u = x as f32 / NOISE_SIZE as f32;
            let v = z as f32 / NOISE_SIZE as f32;
            pixels.push((periodic_noise(u, v, 6, 0) * 255.0).round() as u8);
            pixels.push((periodic_noise(u, v, 17, 1) * 255.0).round() as u8);
        }
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(NOISE_SIZE * 2),
            rows_per_image: Some(NOISE_SIZE),
        },
        wgpu::Extent3d {
            width: NOISE_SIZE,
            height: NOISE_SIZE,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("cloud noise sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    (view, sampler)
}

fn periodic_noise(u: f32, v: f32, cells: i32, seed: u32) -> f32 {
    let x = u * cells as f32;
    let z = v * cells as f32;
    let ix = x.floor() as i32;
    let iz = z.floor() as i32;
    let fx = smooth(x.fract());
    let fz = smooth(z.fract());
    let a = grid_value(ix, iz, cells, seed);
    let b = grid_value(ix + 1, iz, cells, seed);
    let c = grid_value(ix, iz + 1, cells, seed);
    let d = grid_value(ix + 1, iz + 1, cells, seed);
    (a + (b - a) * fx) + ((c + (d - c) * fx) - (a + (b - a) * fx)) * fz
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn grid_value(x: i32, z: i32, cells: i32, seed: u32) -> f32 {
    let mut hash = (x.rem_euclid(cells) as u32).wrapping_mul(0x9e3779b9)
        ^ (z.rem_euclid(cells) as u32).wrapping_mul(0x85ebca6b)
        ^ seed.wrapping_mul(0xc2b2ae35);
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7feb352d);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846ca68b);
    hash ^= hash >> 16;
    (hash & 0xffff) as f32 / 65535.0
}
