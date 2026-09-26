use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Instant;

use glam::Vec3;

#[cfg(test)]
use super::geometry::Bounds;
use super::geometry::Mesh;

use crate::terrain::mesh::{build_chunk, ChunkMesh, Vertex};
use crate::terrain::{Terrain, CHUNK_SIZE, VIEW_RADIUS};

struct Chunk {
    surface: Mesh,
    plants: Option<Mesh>,
}

pub struct ChunkCache {
    generation: u64,
    chunks: HashMap<(i32, i32), Chunk>,
    requests: Sender<BuildRequest>,
    results: Receiver<BuildResult>,
    worker_busy: bool,
    profile: bool,
}

struct BuildRequest {
    terrain: Terrain,
    coordinate: (i32, i32),
}

struct BuildResult {
    generation: u64,
    coordinate: (i32, i32),
    mesh: ChunkMesh,
    build_ms: f64,
}

impl ChunkCache {
    pub fn new(profile: bool) -> std::io::Result<Self> {
        let (requests, request_receiver) = mpsc::channel::<BuildRequest>();
        let (result_sender, results) = mpsc::channel::<BuildResult>();
        thread::Builder::new()
            .name("terrain mesher".into())
            .spawn(move || {
                while let Ok(request) = request_receiver.recv() {
                    let started = Instant::now();
                    let mesh =
                        build_chunk(&request.terrain, request.coordinate.0, request.coordinate.1);
                    if result_sender
                        .send(BuildResult {
                            generation: request.terrain.generation(),
                            coordinate: request.coordinate,
                            mesh,
                            build_ms: started.elapsed().as_secs_f64() * 1000.0,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })?;
        Ok(Self {
            generation: 0,
            chunks: HashMap::new(),
            requests,
            results,
            worker_busy: false,
            profile,
        })
    }

    pub(super) fn meshes(&self) -> impl Iterator<Item = &Mesh> {
        self.chunks
            .values()
            .flat_map(|chunk| std::iter::once(&chunk.surface).chain(chunk.plants.iter()))
    }

    /// Largest conservative horizontal disk with no holes in chunk residency.
    /// Missing chunks reduce coverage before their absent casters can be seen.
    pub(super) fn resident_radius(&self, camera: Vec3) -> f32 {
        resident_radius(camera, |coordinate| self.chunks.contains_key(&coordinate))
    }

    pub fn update(&mut self, device: &wgpu::Device, terrain: &Terrain, x: f32, z: f32) {
        if self.generation != terrain.generation() {
            self.chunks.clear();
            self.generation = terrain.generation();
        }

        let center = (chunk_coordinate(x), chunk_coordinate(z));
        self.chunks
            .retain(|&coordinate, _| within_radius(coordinate, center, VIEW_RADIUS + 1));

        while let Ok(result) = self.results.try_recv() {
            self.worker_busy = false;
            if result_is_current(&result, self.generation, center) {
                self.upload(device, result);
            }
        }

        // One request at a time avoids a queue of chunks for positions left behind.
        if !self.worker_busy {
            if let Some(coordinate) = nearest_missing(center, VIEW_RADIUS, |coordinate| {
                self.chunks.contains_key(&coordinate)
            }) {
                let request = BuildRequest {
                    terrain: terrain.clone(),
                    coordinate,
                };
                self.requests.send(request).expect("terrain mesher stopped");
                self.worker_busy = true;
            }
        }
    }

    fn upload(&mut self, device: &wgpu::Device, result: BuildResult) {
        let started = Instant::now();
        let build_ms = result.build_ms;
        let coordinate = result.coordinate;
        let bytes =
            (result.mesh.terrain.len() + result.mesh.plants.len()) * std::mem::size_of::<Vertex>();
        let surface = Mesh::upload(device, &result.mesh.terrain, f32::INFINITY);
        let plants = (!result.mesh.plants.is_empty())
            .then(|| Mesh::upload(device, &result.mesh.plants, 160.0));
        self.chunks
            .insert(result.coordinate, Chunk { surface, plants });
        if self.profile {
            eprintln!("profile chunk {coordinate:?}: generate {build_ms:.2} ms, upload {:.2} ms, {bytes} bytes", started.elapsed().as_secs_f64() * 1000.0);
        }
    }
}

fn resident_radius(camera: Vec3, loaded: impl Fn((i32, i32)) -> bool) -> f32 {
    let center = (chunk_coordinate(camera.x), chunk_coordinate(camera.z));
    let mut radius = (VIEW_RADIUS as f32 - std::f32::consts::SQRT_2) * CHUNK_SIZE as f32 - 1.0;
    for dx in -VIEW_RADIUS..=VIEW_RADIUS {
        for dz in -VIEW_RADIUS..=VIEW_RADIUS {
            let coordinate = (center.0 + dx, center.1 + dz);
            if loaded(coordinate) {
                continue;
            }
            let min_x = (coordinate.0 * CHUNK_SIZE) as f32 - 0.5;
            let min_z = (coordinate.1 * CHUNK_SIZE) as f32 - 0.5;
            let x = camera.x.clamp(min_x, min_x + CHUNK_SIZE as f32);
            let z = camera.z.clamp(min_z, min_z + CHUNK_SIZE as f32);
            radius = radius.min(((camera.x - x).powi(2) + (camera.z - z).powi(2)).sqrt());
        }
    }
    radius
}

fn result_is_current(result: &BuildResult, generation: u64, center: (i32, i32)) -> bool {
    result.generation == generation && within_radius(result.coordinate, center, VIEW_RADIUS)
}

pub fn vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Uint32];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    }
}

fn chunk_coordinate(position: f32) -> i32 {
    (position.floor() as i32).div_euclid(CHUNK_SIZE)
}

fn within_radius(coordinate: (i32, i32), center: (i32, i32), radius: i32) -> bool {
    let dx = coordinate.0 - center.0;
    let dz = coordinate.1 - center.1;
    dx * dx + dz * dz <= radius * radius
}

fn nearest_missing(
    center: (i32, i32),
    radius: i32,
    loaded: impl Fn((i32, i32)) -> bool,
) -> Option<(i32, i32)> {
    let mut next = None;
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            let distance = dx * dx + dz * dz;
            if distance > radius * radius {
                continue;
            }
            let coordinate = (center.0 + dx, center.1 + dz);
            if !loaded(coordinate) && next.is_none_or(|(_, best_distance)| distance < best_distance)
            {
                next = Some((coordinate, distance));
            }
        }
    }
    next.map(|(coordinate, _)| coordinate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Mat4;

    #[test]
    fn shadow_residency_accounts_for_holes_and_negative_coordinates() {
        for camera in [Vec3::ZERO, Vec3::new(-128.25, 80.0, -0.75)] {
            assert_eq!(resident_radius(camera, |_| false), 0.0);
            let complete = resident_radius(camera, |_| true);
            assert!(
                complete
                    >= super::super::shadows::SHADOW_DISTANCE
                        + super::super::shadows::CASTER_RESERVE
            );
        }
        // A missing chunk starts at x=127.5, independent of camera altitude.
        let radius = resident_radius(Vec3::new(0.0, 500.0, 0.0), |key| key != (2, 0));
        assert_eq!(radius, 127.5);
    }

    #[test]
    fn negative_world_coordinates_use_euclidean_chunks() {
        assert_eq!(chunk_coordinate(-0.01), -1);
        assert_eq!(chunk_coordinate(-64.0), -1);
        assert_eq!(chunk_coordinate(-64.01), -2);
        assert_eq!(chunk_coordinate(63.99), 0);
        assert_eq!(chunk_coordinate(64.0), 1);
    }

    #[test]
    fn nearest_chunk_is_center_then_neighbor() {
        assert_eq!(nearest_missing((3, -2), 1, |_| false), Some((3, -2)));
        assert_eq!(
            nearest_missing((3, -2), 1, |key| key == (3, -2)),
            Some((2, -2))
        );
        assert_eq!(nearest_missing((3, -2), 0, |key| key == (3, -2)), None);
        assert!(within_radius((4, -2), (3, -2), 1));
        assert!(!within_radius((4, -1), (3, -2), 1));
    }

    #[test]
    fn completed_work_from_old_world_or_position_is_discarded() {
        let result = BuildResult {
            generation: 2,
            coordinate: (0, 0),
            mesh: ChunkMesh {
                terrain: Vec::new(),
                plants: Vec::new(),
            },
            build_ms: 0.0,
        };
        assert!(result_is_current(&result, 2, (0, 0)));
        assert!(!result_is_current(&result, 3, (0, 0)));
        assert!(!result_is_current(&result, 2, (VIEW_RADIUS + 1, 0)));
    }

    #[test]
    fn frustum_rejects_chunks_behind_camera() {
        let view_projection = Mat4::perspective_rh(70.0_f32.to_radians(), 1.0, 0.05, 1000.0)
            * Mat4::look_to_rh(Vec3::ZERO, -Vec3::Z, Vec3::Y);
        let ahead = Bounds {
            min: Vec3::new(-2.0, -2.0, -20.0),
            max: Vec3::new(2.0, 2.0, -16.0),
        };
        let behind = Bounds {
            min: Vec3::new(-2.0, -2.0, 16.0),
            max: Vec3::new(2.0, 2.0, 20.0),
        };
        assert!(ahead.visible(view_projection));
        assert!(!behind.visible(view_projection));
    }
}
