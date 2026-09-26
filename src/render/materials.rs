use crate::terrain::material::{Material, TEXTURES};

pub(super) fn load(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(wgpu::TextureView, wgpu::Sampler), String> {
    const TILE_SIZE: u32 = 16;
    const ROW_PITCH: usize = 256;
    const MIP_LEVELS: u32 = 5;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("block and plant texture layers"),
        size: wgpu::Extent3d {
            width: TILE_SIZE,
            height: TILE_SIZE,
            depth_or_array_layers: TEXTURES.len() as u32,
        },
        mip_level_count: MIP_LEVELS,
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
        let preserve_alpha_coverage = Material::ALL[layer].alpha_test();
        let base_coverage = alpha_coverage(image.as_raw());
        let mut mip = image.into_raw();
        let mut size = TILE_SIZE;
        for level in 0..MIP_LEVELS {
            let mut padded = vec![0_u8; ROW_PITCH * size as usize];
            for row in 0..size as usize {
                let source_start = row * size as usize * 4;
                let destination_start = row * ROW_PITCH;
                padded[destination_start..destination_start + size as usize * 4]
                    .copy_from_slice(&mip[source_start..source_start + size as usize * 4]);
            }
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level,
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
                    rows_per_image: Some(size),
                },
                wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
            );
            if size == 1 {
                break;
            }
            // At 1 by 1, binary alpha coverage cannot represent a partial
            // footprint. Keep its averaged alpha so sparse plants disappear
            // instead of turning into a solid pixel billboard.
            let preserve_coverage = (preserve_alpha_coverage && size > 2).then_some(base_coverage);
            mip = downsample(&mip, size, preserve_coverage);
            size /= 2;
        }
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("block texture array"),
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("mipmapped repeating block sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    Ok((view, sampler))
}

fn downsample(source: &[u8], size: u32, preserve_coverage: Option<f32>) -> Vec<u8> {
    let next_size = size / 2;
    let mut next = vec![0_u8; (next_size * next_size * 4) as usize];
    // A 4-tap binomial low-pass filter includes samples across the repeat
    // boundary. A local 2x2 box leaves discontinuous tile borders untouched,
    // which shows up as seams as soon as a minified mip is sampled.
    const FILTER: [f32; 4] = [1.0 / 8.0, 3.0 / 8.0, 3.0 / 8.0, 1.0 / 8.0];
    for y in 0..next_size {
        for x in 0..next_size {
            let mut alpha = 0.0;
            let mut color = [0.0; 3];
            for dy in 0..4 {
                for dx in 0..4 {
                    let source_x = (x as i32 * 2 + dx as i32 - 1).rem_euclid(size as i32) as u32;
                    let source_y = (y as i32 * 2 + dy as i32 - 1).rem_euclid(size as i32) as u32;
                    let weight = FILTER[dx] * FILTER[dy];
                    let index = ((source_y * size + source_x) * 4) as usize;
                    let a = source[index + 3] as f32 / 255.0;
                    alpha += a * weight;
                    for channel in 0..3 {
                        color[channel] +=
                            srgb_to_linear(source[index + channel] as f32 / 255.0) * a * weight;
                    }
                }
            }
            let index = ((y * next_size + x) * 4) as usize;
            for channel in 0..3 {
                let linear = if alpha > 0.0 {
                    color[channel] / alpha
                } else {
                    0.0
                };
                next[index + channel] = (linear_to_srgb(linear) * 255.0 + 0.5) as u8;
            }
            next[index + 3] = (alpha * 255.0 + 0.5) as u8;
        }
    }
    if let Some(target) = preserve_coverage {
        preserve_alpha_coverage(&mut next, target);
    }
    next
}

fn alpha_coverage(rgba: &[u8]) -> f32 {
    let covered = rgba.chunks_exact(4).filter(|pixel| pixel[3] >= 128).count();
    covered as f32 / (rgba.len() / 4) as f32
}

fn preserve_alpha_coverage(rgba: &mut [u8], target: f32) {
    let mut low = 0.0;
    let mut high = 8.0;
    for _ in 0..12 {
        let scale = (low + high) * 0.5;
        let coverage = rgba
            .chunks_exact(4)
            .filter(|pixel| (pixel[3] as f32 * scale).min(255.0) >= 128.0)
            .count() as f32
            / (rgba.len() / 4) as f32;
        if coverage < target {
            low = scale;
        } else {
            high = scale;
        }
    }
    for pixel in rgba.chunks_exact_mut(4) {
        pixel[3] = (pixel[3] as f32 * high).min(255.0) as u8;
    }
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> f32 {
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_colors_are_filtered_in_linear_light() {
        let source = [
            0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
        ];
        let mip = downsample(&source, 2, None);
        assert!((187..=189).contains(&mip[0]));
        assert_eq!(mip[3], 255);
    }

    #[test]
    fn sparse_cutouts_do_not_become_solid_at_the_smallest_mip() {
        let mut mip = vec![0_u8; 16 * 16 * 4];
        for y in 0..16 {
            for x in 0..4 {
                mip[(y * 16 + x) * 4 + 3] = 255;
            }
        }
        let coverage = alpha_coverage(&mip);
        let mut size = 16;
        while size > 1 {
            let preserve = (size > 2).then_some(coverage);
            mip = downsample(&mip, size, preserve);
            size /= 2;
        }
        assert!(mip[3] < 128);
    }

    #[test]
    fn wrapped_mips_reduce_tile_border_discontinuity() {
        let mut source = vec![0_u8; 16 * 16 * 4];
        for y in 0..16 {
            for x in 0..16 {
                let value = (x * 255 / 15) as u8;
                let index = (y * 16 + x) * 4;
                source[index..index + 4].copy_from_slice(&[value, value, value, 255]);
            }
        }

        let mip = downsample(&source, 16, None);
        let source_seam = (source[0] as f32 - source[15 * 4] as f32).abs();
        let source_interior = (source[4] as f32 - source[0] as f32).abs();
        let mip_seam = (mip[0] as f32 - mip[7 * 4] as f32).abs();
        let mip_interior = (mip[4] as f32 - mip[0] as f32).abs();

        assert!(mip_seam / mip_interior < source_seam / source_interior * 0.6);
    }
}
