use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

use crate::game::DAY_LENGTH_SECONDS;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Uniforms {
    view_projection: [[f32; 4]; 4],
    inverse_sky_view_projection: [[f32; 4]; 4],
    sun_direction: [f32; 4],
    moon_direction: [f32; 4],
    sky_horizon: [f32; 4],
    sky_zenith: [f32; 4],
    sun_light: [f32; 4],
    moon_light: [f32; 4],
    ambient: [f32; 4],
    camera_position: [f32; 4],
    pub shadow_origin: [i32; 4],
    pub shadow_params: [f32; 4],
}

pub(super) struct SceneFrame {
    pub uniforms: Uniforms,
    pub view_projection: Mat4,
}

impl SceneFrame {
    pub fn new(
        position: Vec3,
        forward: Vec3,
        day_seconds: f32,
        sky_seconds: f32,
        aspect: f32,
    ) -> Self {
        let view_rotation = Mat4::look_to_rh(Vec3::ZERO, forward, Vec3::Y);
        let view = view_rotation * Mat4::from_translation(-position);
        let projection = Mat4::perspective_rh(70.0_f32.to_radians(), aspect, 0.05, 1000.0);
        let view_projection = projection * view;
        // The sky is infinitely distant. Its ray must depend on orientation and
        // projection only; embedding translation loses precision as the player moves.
        let inverse_sky_view_projection = (projection * view_rotation).inverse();
        let sun_direction = sun_direction(day_seconds);
        let moon_direction = -sun_direction;
        let sun_height = sun_direction.y;
        let daylight = smoothstep(-0.10, 0.13, sun_height);
        let sun_strength = smoothstep(0.0, 0.16, sun_height);
        let moon_strength = smoothstep(0.04, 0.28, -sun_height);
        let warm = 1.0 - smoothstep(0.05, 0.5, sun_height);
        let sky_horizon = Vec3::new(0.035, 0.065, 0.13).lerp(Vec3::new(0.64, 0.81, 0.94), daylight);
        let sky_zenith = Vec3::new(0.008, 0.024, 0.07).lerp(Vec3::new(0.25, 0.58, 0.85), daylight);
        let sun_color = Vec3::new(1.0, 0.96, 0.88).lerp(Vec3::new(1.0, 0.52, 0.28), warm);
        let ambient = Vec3::new(0.11, 0.14, 0.23).lerp(Vec3::new(0.43, 0.45, 0.46), daylight);
        let uniforms = Uniforms {
            view_projection: view_projection.to_cols_array_2d(),
            inverse_sky_view_projection: inverse_sky_view_projection.to_cols_array_2d(),
            sun_direction: sun_direction.extend(0.0).to_array(),
            moon_direction: moon_direction.extend(0.0).to_array(),
            sky_horizon: sky_horizon.extend(0.0).to_array(),
            sky_zenith: sky_zenith.extend(0.0).to_array(),
            sun_light: sun_color.extend(sun_strength).to_array(),
            moon_light: Vec3::new(0.43, 0.57, 0.82)
                .extend(moon_strength * 0.4)
                .to_array(),
            ambient: ambient.extend(0.0).to_array(),
            camera_position: position.extend(sky_seconds).to_array(),
            shadow_origin: [0; 4],
            shadow_params: [0.0; 4],
        };
        Self {
            uniforms,
            view_projection,
        }
    }
}

fn sun_direction(day_seconds: f32) -> Vec3 {
    let phase = day_seconds * std::f32::consts::TAU / DAY_LENGTH_SECONDS;
    Vec3::new(phase.cos(), phase.sin(), -0.28 * phase.cos()).normalize()
}

fn smoothstep(a: f32, b: f32, value: f32) -> f32 {
    let t = ((value - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shadow_length(height: f32, sun: Vec3) -> f32 {
        height * Vec3::new(sun.x, 0.0, sun.z).length() / sun.y
    }

    #[test]
    fn sunrise_shortens_and_sunset_lengthens_geometric_shadows() {
        let mut previous = f32::INFINITY;
        for step in 1..1500 {
            let length = shadow_length(10.0, sun_direction(step as f32 * 0.1));
            assert!(
                length <= previous + 0.0001,
                "sunrise at {step}: {length} > {previous}"
            );
            previous = length;
        }
        previous = 0.0;
        for step in 1501..3000 {
            let length = shadow_length(10.0, sun_direction(step as f32 * 0.1));
            assert!(
                length + 0.0001 >= previous,
                "sunset at {step}: {length} < {previous}"
            );
            previous = length;
        }
    }

    #[test]
    fn sun_direction_is_continuous_through_day_wrap() {
        assert!((sun_direction(0.0) - sun_direction(DAY_LENGTH_SECONDS)).length() < 0.000001);
    }

    #[test]
    fn sky_rays_are_independent_of_camera_translation() {
        let forward = Vec3::new(0.3, 0.2, -0.9).normalize();
        let origin = SceneFrame::new(Vec3::ZERO, forward, 400.0, 10.0, 16.0 / 9.0);
        let moved = SceneFrame::new(
            Vec3::new(8192.0, 120.0, -4096.0),
            forward,
            400.0,
            10.0,
            16.0 / 9.0,
        );
        assert_eq!(
            origin.uniforms.inverse_sky_view_projection,
            moved.uniforms.inverse_sky_view_projection
        );
        assert_ne!(origin.view_projection, moved.view_projection);
    }
}
