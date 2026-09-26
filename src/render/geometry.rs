use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::terrain::material::{Material, ShadowPolicy, WIND_AMPLITUDE};
use crate::terrain::mesh::Vertex;

/// One uploaded draw, consumed by both visible and shadow passes. Vertices are
/// world-space today; future object transforms/deformation belong in the shared
/// vertex function, with the same conservative world bounds supplied here.
pub(super) struct Mesh {
    pub buffer: wgpu::Buffer,
    pub vertex_count: u32,
    pub bounds: Bounds,
    pub casts_shadow: bool,
    pub visible_distance: f32,
}

impl Mesh {
    pub fn upload(device: &wgpu::Device, vertices: &[Vertex], visible_distance: f32) -> Self {
        let mut casts_shadow = false;
        let mut margin: f32 = 0.0;
        for vertex in vertices {
            let material = Material::ALL[vertex.material as usize];
            casts_shadow |= material.shadow_policy() != ShadowPolicy::None;
            if material.wind() {
                margin = WIND_AMPLITUDE[0].max(WIND_AMPLITUDE[1]);
            }
        }
        Self {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("renderable geometry"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            vertex_count: vertices.len() as u32,
            bounds: Bounds::from_vertices(vertices, margin),
            casts_shadow,
            visible_distance,
        }
    }

    pub fn in_visible_range(&self, camera: Vec3) -> bool {
        camera.distance_squared(camera.clamp(self.bounds.min, self.bounds.max))
            < self.visible_distance.powi(2)
    }
}

/// World-space renderable bounds, shared by camera and light culling.
#[derive(Clone, Copy, Debug)]
pub(super) struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    pub fn from_vertices(vertices: &[Vertex], deformation_margin: f32) -> Self {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for vertex in vertices {
            let position = Vec3::from_array(vertex.position);
            min = min.min(position);
            max = max.max(position);
        }
        Self {
            min: min - Vec3::splat(deformation_margin),
            max: max + Vec3::splat(deformation_margin),
        }
    }

    pub fn corners(self) -> impl Iterator<Item = Vec3> {
        (0..8).map(move |i| {
            Vec3::new(
                if i & 1 == 0 { self.min.x } else { self.max.x },
                if i & 2 == 0 { self.min.y } else { self.max.y },
                if i & 4 == 0 { self.min.z } else { self.max.z },
            )
        })
    }

    pub fn visible(self, view_projection: Mat4) -> bool {
        let mut outside = 0b11_1111_u8;
        for point in self.corners() {
            let p = view_projection * point.extend(1.0);
            for (plane, inside) in [
                p.x >= -p.w,
                p.x <= p.w,
                p.y >= -p.w,
                p.y <= p.w,
                p.z >= 0.0,
                p.z <= p.w,
            ]
            .into_iter()
            .enumerate()
            {
                if inside {
                    outside &= !(1 << plane);
                }
            }
        }
        outside == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_clip_plane_is_zero_not_minus_w() {
        let behind_near = Bounds {
            min: Vec3::new(-0.1, -0.1, -0.5),
            max: Vec3::new(0.1, 0.1, -0.1),
        };
        assert!(!behind_near.visible(Mat4::IDENTITY));
        let crossing_near = Bounds {
            max: Vec3::new(0.1, 0.1, 0.1),
            ..behind_near
        };
        assert!(crossing_near.visible(Mat4::IDENTITY));
    }
}
