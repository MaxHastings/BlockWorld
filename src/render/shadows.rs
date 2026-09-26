use glam::{Mat4, Vec3};

use super::geometry::Bounds;
use super::scene::SUN_ORBIT_TILT;

pub(super) const CASCADE_COUNT: usize = 2;
pub(super) const SHADOW_MAP_SIZE: u32 = 2048;
pub(super) const NEAR_SPLIT: f32 = 40.0;
pub(super) const BLEND_HALF_WIDTH: f32 = 8.0;
pub(super) const SHADOW_DISTANCE: f32 = 160.0;
pub(super) const DISTANCE_FADE: f32 = 32.0;
/// Reserve resident geometry upstream of every fully shadowed receiver.
pub(super) const CASTER_RESERVE: f32 = 128.0;
const FILTER_RADIUS_TEXELS: f32 = 2.0;

pub(super) struct Frame {
    pub view_projection: [Mat4; CASCADE_COUNT],
    pub world_view_projection: [Mat4; CASCADE_COUNT],
    /// World units per texel, inverse depth span, inverse map size, filter radius in texels.
    pub scale: [[f32; 4]; CASCADE_COUNT],
    pub shadow_strength: f32,
    pub distance: f32,
}

pub(super) struct Camera {
    pub position: Vec3,
    pub forward: Vec3,
    pub aspect: f32,
    pub fov_y: f32,
}

impl Frame {
    pub fn new(camera: Camera, sun: Vec3, casters: &[Bounds], resident_radius: f32) -> Self {
        let (right, up) = light_basis(sun);
        let mut result = Self {
            view_projection: [Mat4::IDENTITY; CASCADE_COUNT],
            world_view_projection: [Mat4::IDENTITY; CASCADE_COUNT],
            scale: [[0.0; 4]; CASCADE_COUNT],
            // Deliberate horizon policy: no detailed shadows below 6 degrees;
            // full strength at 12 degrees. The light itself is never quantized.
            shadow_strength: smoothstep(
                6.0_f32.to_radians().sin(),
                12.0_f32.to_radians().sin(),
                sun.y,
            ),
            distance: SHADOW_DISTANCE.min((resident_radius - CASTER_RESERVE).max(0.0)),
        };
        for (cascade, reach) in [NEAR_SPLIT + BLEND_HALF_WIDTH, SHADOW_DISTANCE]
            .into_iter()
            .enumerate()
        {
            // Sphere enclosing the camera cone clipped by radial receiver distance.
            // Its radius is invariant under both camera and light rotation.
            let corner_cos = (1.0
                + (camera.fov_y * 0.5).tan().powi(2) * (1.0 + camera.aspect * camera.aspect))
                .sqrt()
                .recip();
            let offset = reach * corner_cos.min(0.5 / corner_cos);
            let radius = offset
                .max((reach * reach + offset * offset - 2.0 * reach * offset * corner_cos).sqrt());
            // Guard the filter footprint and half-texel snapping error.
            let extent =
                radius / (1.0 - 2.0 * (FILTER_RADIUS_TEXELS + 0.5) / SHADOW_MAP_SIZE as f32);
            let texel = 2.0 * extent / SHADOW_MAP_SIZE as f32;
            let center = camera.position + camera.forward * offset;
            let snap =
                |axis: Vec3| (center.dot(axis) / texel).round() * texel - camera.position.dot(axis);
            let relative_center =
                right * snap(right) + up * snap(up) + sun * (camera.forward * offset).dot(sun);
            let view = Mat4::look_to_rh(relative_center, -sun, up);

            // Fit receivers and every resident mesh overlapping the light plane.
            // This never uses camera visibility or a terrain-height proxy.
            let mut min_z = -extent;
            let mut max_z = extent;
            for bounds in casters {
                let mut min = Vec3::splat(f32::INFINITY);
                let mut max = Vec3::splat(f32::NEG_INFINITY);
                for p in bounds.corners() {
                    let p = view.transform_point3(p - camera.position);
                    min = min.min(p);
                    max = max.max(p);
                }
                if min.x <= extent && max.x >= -extent && min.y <= extent && max.y >= -extent {
                    min_z = min_z.min(min.z);
                    max_z = max_z.max(max.z);
                }
            }
            // RH orthographic depth is [0,1], including a negative near distance.
            // A world texel of padding keeps boundary triangles inside the clip.
            let near = -max_z - texel;
            let far = -min_z + texel;
            let matrix = Mat4::orthographic_rh(-extent, extent, -extent, extent, near, far) * view;
            result.view_projection[cascade] = matrix;
            result.world_view_projection[cascade] =
                matrix * Mat4::from_translation(-camera.position);
            result.scale[cascade] = [
                texel,
                (far - near).recip(),
                1.0 / SHADOW_MAP_SIZE as f32,
                FILTER_RADIUS_TEXELS,
            ];
        }
        result
    }
}

fn light_basis(sun: Vec3) -> (Vec3, Vec3) {
    // Normal of the game's entire sun orbit, not a switchable up vector.
    // Perpendicular to the sun at noon, midnight and wrap.
    let orbit_normal = Vec3::new(SUN_ORBIT_TILT, 0.0, 1.0).normalize();
    let right = orbit_normal.cross(sun).normalize();
    (right, sun.cross(right))
}

fn smoothstep(a: f32, b: f32, value: f32) -> f32 {
    let t = ((value - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sun(phase: f32) -> Vec3 {
        Vec3::new(phase.cos(), phase.sin(), -SUN_ORBIT_TILT * phase.cos()).normalize()
    }

    fn camera(position: Vec3) -> Camera {
        Camera {
            position,
            forward: -Vec3::Z,
            aspect: 1.6,
            fov_y: 70_f32.to_radians(),
        }
    }

    #[test]
    fn orbit_basis_is_continuous_and_orthonormal_including_noon_and_wrap() {
        let mut previous = light_basis(sun(-0.001));
        for step in 0..=6284 {
            let s = sun(step as f32 * 0.001);
            let (r, u) = light_basis(s);
            assert!(r.is_finite() && u.is_finite());
            assert!((r.length() - 1.0).abs() < 1e-5 && (u.length() - 1.0).abs() < 1e-5);
            assert!(r.dot(s).abs() < 1e-5 && u.dot(s).abs() < 1e-5);
            assert!(r.dot(previous.0) > 0.999 && u.dot(previous.1) > 0.999);
            previous = (r, u);
        }
    }

    #[test]
    fn receiver_cone_and_offscreen_caster_fit_wgpu_depth() {
        let caster = Bounds {
            min: Vec3::new(-3.0, 120.0, -30.0),
            max: Vec3::new(3.0, 130.0, -20.0),
        };
        for phase in [0.11, 0.8, std::f32::consts::FRAC_PI_2, 3.0] {
            let frame = Frame::new(camera(Vec3::ZERO), sun(phase), &[caster], 300.0);
            assert!(caster.visible(frame.world_view_projection[1]));
            for (i, distance) in [NEAR_SPLIT + BLEND_HALF_WIDTH, SHADOW_DISTANCE]
                .into_iter()
                .enumerate()
            {
                for x in [-1.6, 0.0, 1.6] {
                    for y in [-1.0, 0.0, 1.0] {
                        let ray = Vec3::new(
                            x * 35_f32.to_radians().tan(),
                            y * 35_f32.to_radians().tan(),
                            -1.0,
                        )
                        .normalize();
                        for d in [0.0, distance * 0.5, distance] {
                            let p = frame.world_view_projection[i].transform_point3(ray * d);
                            assert!(
                                p.x.abs() < 1.0 && p.y.abs() < 1.0 && p.z >= 0.0 && p.z <= 1.0,
                                "{p:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn translation_preserves_world_texel_grid_and_receiver_plane_depth() {
        let s = sun(0.8);
        let (right, up) = light_basis(s);
        let a = Frame::new(camera(Vec3::ZERO), s, &[], 300.0);
        let b = Frame::new(camera(right * 0.00001), s, &[], 300.0);
        let point = Vec3::new(3.0, 0.0, -12.0);
        for i in 0..CASCADE_COUNT {
            let pa = a.world_view_projection[i].transform_point3(point);
            let pb = b.world_view_projection[i].transform_point3(point);
            assert!((pa - pb).length() < 1e-5);
            for n in [Vec3::Y, Vec3::X, Vec3::new(0.4, 0.8, 0.3).normalize()] {
                let delta = right * a.scale[i][0] + up * a.scale[i][0];
                let on_plane = point + delta - s * (delta.dot(n) / s.dot(n));
                let depth_delta = a.world_view_projection[i].transform_point3(on_plane).z - pa.z;
                let predicted = delta.dot(n) / s.dot(n) * a.scale[i][1];
                assert!((depth_delta - predicted).abs() < 1e-6);
            }
        }
    }
}
