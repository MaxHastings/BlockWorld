use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use crate::terrain::Terrain;

pub(super) const SIZE: u32 = 1024;
const RECENTER_DISTANCE: i32 = 128;
const MAX_MIP_LEVEL: u32 = 3;

pub(super) struct HeightField {
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    tree_texture: wgpu::Texture,
    pub tree_view: wgpu::TextureView,
    pub origin: [i32; 2],
    pub max_height: f32,
    pub valid: bool,
    generation: u64,
    worker_busy: bool,
    requests: Sender<Request>,
    results: Receiver<ResultData>,
    profile: bool,
}

struct Request {
    terrain: Terrain,
    origin: [i32; 2],
}

struct ResultData {
    generation: u64,
    origin: [i32; 2],
    levels: Vec<Vec<i32>>,
    tree_spans: Vec<u32>,
    max_height: f32,
    build_ms: f64,
}

impl HeightField {
    pub fn new(device: &wgpu::Device, profile: bool) -> std::io::Result<Self> {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("terrain heights for exact sun rays"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: MAX_MIP_LEVEL + 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Sint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let tree_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("tree canopy and trunk height spans"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let tree_view = tree_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (requests, request_receiver) = mpsc::channel::<Request>();
        let (result_sender, results) = mpsc::channel::<ResultData>();
        thread::Builder::new()
            .name("sun ray height field".into())
            .spawn(move || {
                while let Ok(request) = request_receiver.recv() {
                    let result = build(&request.terrain, request.origin);
                    if result_sender.send(result).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            texture,
            view,
            tree_texture,
            tree_view,
            origin: [0, 0],
            max_height: 0.0,
            valid: false,
            generation: 0,
            worker_busy: false,
            requests,
            results,
            profile,
        })
    }

    pub fn update(&mut self, queue: &wgpu::Queue, terrain: &Terrain, x: f32, z: f32) {
        if terrain.generation() != self.generation {
            self.generation = terrain.generation();
            self.valid = false;
        }
        while let Ok(result) = self.results.try_recv() {
            self.worker_busy = false;
            if !result_is_current(&result, self.generation, x, z) {
                continue;
            }
            let upload_started = Instant::now();
            for (level, heights) in result.levels.iter().enumerate() {
                let size = SIZE >> level;
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.texture,
                        mip_level: level as u32,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    bytemuck::cast_slice(heights),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(size * 4),
                        rows_per_image: Some(size),
                    },
                    wgpu::Extent3d {
                        width: size,
                        height: size,
                        depth_or_array_layers: 1,
                    },
                );
            }
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.tree_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                bytemuck::cast_slice(&result.tree_spans),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(SIZE * 4),
                    rows_per_image: Some(SIZE),
                },
                wgpu::Extent3d {
                    width: SIZE,
                    height: SIZE,
                    depth_or_array_layers: 1,
                },
            );
            self.origin = result.origin;
            self.max_height = result.max_height;
            self.valid = true;
            if self.profile {
                eprintln!(
                    "profile shadow field: generate {:.2} ms, upload {:.2} ms, {} samples",
                    result.build_ms,
                    upload_started.elapsed().as_secs_f64() * 1000.0,
                    SIZE * SIZE
                );
            }
        }

        let center_x = self.origin[0] + SIZE as i32 / 2;
        let center_z = self.origin[1] + SIZE as i32 / 2;
        if !self.worker_busy
            && (!self.valid
                || (x.floor() as i32 - center_x).abs() > RECENTER_DISTANCE
                || (z.floor() as i32 - center_z).abs() > RECENTER_DISTANCE)
        {
            let origin = [
                x.floor() as i32 - SIZE as i32 / 2,
                z.floor() as i32 - SIZE as i32 / 2,
            ];
            self.requests
                .send(Request {
                    terrain: terrain.clone(),
                    origin,
                })
                .expect("sun ray height worker stopped");
            self.worker_busy = true;
        }
    }
}

fn result_is_current(result: &ResultData, generation: u64, x: f32, z: f32) -> bool {
    let center_x = i64::from(result.origin[0]) + i64::from(SIZE / 2);
    let center_z = i64::from(result.origin[1]) + i64::from(SIZE / 2);
    result.generation == generation
        && (i64::from(x.floor() as i32) - center_x).abs() <= i64::from(RECENTER_DISTANCE)
        && (i64::from(z.floor() as i32) - center_z).abs() <= i64::from(RECENTER_DISTANCE)
}

fn build(terrain: &Terrain, origin: [i32; 2]) -> ResultData {
    let started = Instant::now();
    let mut heights = Vec::with_capacity((SIZE * SIZE) as usize);
    let mut max_height = f32::NEG_INFINITY;
    for z in 0..SIZE as i32 {
        for x in 0..SIZE as i32 {
            let height = terrain.height_at(origin[0] + x, origin[1] + z);
            max_height = max_height.max(height as f32 + 0.5);
            heights.push(height);
        }
    }
    let mut tree_spans = vec![0_u32; (SIZE * SIZE) as usize];
    let mut caster_heights = heights.clone();
    for tree in terrain.trees_in_region(
        origin[0] - 2,
        origin[1] - 2,
        origin[0] + SIZE as i32 + 2,
        origin[1] + SIZE as i32 + 2,
    ) {
        for dz in -2..=2 {
            for dx in -2..=2 {
                let x = tree.x + dx - origin[0];
                let z = tree.z + dz - origin[1];
                if x < 0 || x >= SIZE as i32 || z < 0 || z >= SIZE as i32 {
                    continue;
                }
                let index = (z as u32 * SIZE + x as u32) as usize;
                if let Some((bottom, top)) = tree.vertical_span(dx, dz) {
                    tree_spans[index] = pack_span(bottom, top);
                    caster_heights[index] = caster_heights[index].max(top);
                    max_height = max_height.max(top as f32 + 0.5);
                }
            }
        }
    }
    let mut levels = vec![heights];
    let mut previous = caster_heights;
    for level in 1..=MAX_MIP_LEVEL {
        let width = (SIZE >> level) as usize;
        let previous_width = width * 2;
        let mut maxima = Vec::with_capacity(width * width);
        for z in 0..width {
            for x in 0..width {
                let index = (z * 2) * previous_width + x * 2;
                maxima.push(
                    previous[index]
                        .max(previous[index + 1])
                        .max(previous[index + previous_width])
                        .max(previous[index + previous_width + 1]),
                );
            }
        }
        previous = maxima.clone();
        levels.push(maxima);
    }
    ResultData {
        generation: terrain.generation(),
        origin,
        levels,
        tree_spans,
        max_height,
        build_ms: started.elapsed().as_secs_f64() * 1000.0,
    }
}

fn pack_span(bottom: i32, top: i32) -> u32 {
    ((bottom + 32768) as u32 & 0xffff) | (((top + 32768) as u32 & 0xffff) << 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_field_discards_results_for_an_old_position_or_world() {
        let result = ResultData {
            generation: 2,
            origin: [-512, -512],
            levels: Vec::new(),
            tree_spans: Vec::new(),
            max_height: 0.0,
            build_ms: 0.0,
        };
        assert!(result_is_current(&result, 2, 128.0, -128.0));
        assert!(!result_is_current(&result, 2, 129.0, 0.0));
        assert!(!result_is_current(&result, 2, 0.0, -129.0));
        assert!(!result_is_current(&result, 3, 0.0, 0.0));
    }
}
