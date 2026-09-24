use crate::terrain::material::TEXTURES;

pub(super) fn load(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(wgpu::TextureView, wgpu::Sampler), String> {
    const TILE_SIZE: u32 = 16;
    const ROW_PITCH: usize = 256;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("block and plant texture layers"),
        size: wgpu::Extent3d {
            width: TILE_SIZE,
            height: TILE_SIZE,
            depth_or_array_layers: TEXTURES.len() as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (layer, bytes) in TEXTURES.iter().enumerate() {
        let image = image::load_from_memory(bytes)
            .map_err(|error| format!("Could not decode block texture {layer}: {error}"))?
            .to_rgba8();
        if image.width() != TILE_SIZE || image.height() != TILE_SIZE {
            return Err(format!("Block texture {layer} is not 16 by 16 pixels"));
        }
        let mut padded = vec![0_u8; ROW_PITCH * TILE_SIZE as usize];
        for row in 0..TILE_SIZE as usize {
            padded[row * ROW_PITCH..row * ROW_PITCH + 64]
                .copy_from_slice(&image.as_raw()[row * 64..row * 64 + 64]);
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer as u32,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &padded,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(ROW_PITCH as u32),
                rows_per_image: Some(TILE_SIZE),
            },
            wgpu::Extent3d {
                width: TILE_SIZE,
                height: TILE_SIZE,
                depth_or_array_layers: 1,
            },
        );
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("block texture array"),
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("nearest repeating block sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    Ok((view, sampler))
}
