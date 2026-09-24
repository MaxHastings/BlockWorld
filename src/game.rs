use std::collections::HashSet;

use glam::Vec3;
use winit::keyboard::KeyCode;

use crate::terrain::Terrain;

pub const DAY_LENGTH_SECONDS: f32 = 600.0;
const EYE_HEIGHT: f32 = 1.7;
const PLAYER_RADIUS: f32 = 0.28;
const MAX_STEP_HEIGHT: f32 = 1.0;
const GRAVITY: f32 = 24.0;
const JUMP_VELOCITY: f32 = 8.5;

pub struct Game {
    pub terrain: Terrain,
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub walking: bool,
    pub day_seconds: f32,
    pub sky_seconds: f32,
    pub time_paused: bool,
    pub keys: HashSet<KeyCode>,
    vertical_velocity: f32,
    grounded: bool,
}

impl Game {
    pub fn new() -> Self {
        Self::with_terrain(Terrain::new())
    }

    pub fn with_seed(seed: u32) -> Self {
        Self::with_terrain(Terrain::with_seed(seed))
    }

    fn with_terrain(terrain: Terrain) -> Self {
        let position = Vec3::new(0.0, terrain.ground_height(0.0, 0.0) + 8.0, 0.0);
        Self {
            terrain,
            position,
            yaw: 0.0,
            pitch: 0.0,
            walking: false,
            day_seconds: 0.0,
            sky_seconds: 0.0,
            time_paused: false,
            keys: HashSet::new(),
            vertical_velocity: 0.0,
            grounded: false,
        }
    }

    pub fn restore_viewpoint(
        &mut self,
        seed: u32,
        position: Vec3,
        yaw: f32,
        pitch: f32,
        day_seconds: f32,
        sky_seconds: f32,
    ) {
        self.terrain.set_seed(seed);
        self.position = position;
        self.yaw = yaw;
        self.pitch = pitch;
        self.day_seconds = day_seconds;
        self.sky_seconds = sky_seconds;
        self.time_paused = true;
        self.walking = false;
        self.vertical_velocity = 0.0;
        self.grounded = false;
    }

    pub fn key_pressed(&mut self, key: KeyCode) {
        match key {
            KeyCode::KeyG => {
                self.walking = !self.walking;
                self.vertical_velocity = 0.0;
                self.grounded = false;
            }
            KeyCode::Space if self.walking && self.grounded => {
                self.vertical_velocity = JUMP_VELOCITY;
                self.grounded = false;
            }
            KeyCode::KeyR => {
                self.terrain.regenerate();
                self.position = Vec3::new(0.0, self.terrain.ground_height(0.0, 0.0) + 8.0, 0.0);
                self.vertical_velocity = 0.0;
                self.grounded = false;
            }
            KeyCode::KeyP => self.time_paused = !self.time_paused,
            _ => {}
        }
    }

    pub fn mouse_motion(&mut self, dx: f64, dy: f64) {
        const SENSITIVITY: f32 = 0.12;
        self.yaw += dx as f32 * SENSITIVITY;
        self.pitch = (self.pitch - dy as f32 * SENSITIVITY).clamp(-89.0, 89.0);
    }

    pub fn update(&mut self, seconds: f32) {
        if !self.time_paused {
            self.day_seconds = (self.day_seconds + seconds.max(0.0)) % DAY_LENGTH_SECONDS;
            self.sky_seconds += seconds.max(0.0);
        }
        let dt = seconds.min(0.05);
        let forward_input = self.axis(KeyCode::KeyW, KeyCode::KeyS);
        let strafe_input = self.axis(KeyCode::KeyD, KeyCode::KeyA);
        let yaw = self.yaw.to_radians();
        let pitch = self.pitch.to_radians();
        let forward = Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            -yaw.cos() * pitch.cos(),
        );
        let right = Vec3::new(yaw.cos(), 0.0, yaw.sin());

        if self.walking {
            let horizontal = (Vec3::new(forward.x, 0.0, forward.z) * forward_input
                + right * strafe_input)
                .normalize_or_zero();
            let movement = horizontal * (4.0 * dt);
            let feet_y = self.position.y - EYE_HEIGHT;
            if self.can_occupy(
                self.position.x + movement.x,
                self.position.z + movement.z,
                feet_y,
            ) {
                self.position.x += movement.x;
                self.position.z += movement.z;
            } else {
                if self.can_occupy(self.position.x + movement.x, self.position.z, feet_y) {
                    self.position.x += movement.x;
                }
                if self.can_occupy(self.position.x, self.position.z + movement.z, feet_y) {
                    self.position.z += movement.z;
                }
            }

            self.vertical_velocity -= GRAVITY * dt;
            self.position.y += self.vertical_velocity * dt;
            let floor = self.terrain.ground_height(self.position.x, self.position.z) + EYE_HEIGHT;
            if self.position.y <= floor {
                self.position.y = floor;
                self.vertical_velocity = 0.0;
                self.grounded = true;
            } else {
                self.grounded = false;
            }
        } else {
            let speed = if self.keys.contains(&KeyCode::ShiftLeft) {
                10.0
            } else {
                1.0
            };
            let movement = (forward * forward_input + right * strafe_input).normalize_or_zero();
            self.position += movement * speed * (20.0 * dt);
        }
    }

    fn axis(&self, positive: KeyCode, negative: KeyCode) -> f32 {
        (self.keys.contains(&positive) as i32 - self.keys.contains(&negative) as i32) as f32
    }

    fn can_occupy(&self, x: f32, z: f32, feet_y: f32) -> bool {
        for dx in [-PLAYER_RADIUS, 0.0, PLAYER_RADIUS] {
            for dz in [-PLAYER_RADIUS, 0.0, PLAYER_RADIUS] {
                if self.terrain.ground_height(x + dx, z + dz) > feet_y + MAX_STEP_HEIGHT {
                    return false;
                }
            }
        }
        true
    }

    pub fn forward(&self) -> Vec3 {
        let yaw = self.yaw.to_radians();
        let pitch = self.pitch.to_radians();
        Vec3::new(
            yaw.sin() * pitch.cos(),
            pitch.sin(),
            -yaw.cos() * pitch.cos(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_clock_wraps_after_ten_minutes() {
        let mut game = Game::new();
        game.update(300.0);
        assert_eq!(game.day_seconds, 300.0);
        game.update(300.0);
        assert_eq!(game.day_seconds, 0.0);
    }

    #[test]
    fn diagonal_flight_has_the_same_speed_as_straight_flight() {
        let mut game = Game::with_seed(42);
        let start = game.position;
        game.keys.insert(KeyCode::KeyW);
        game.update(0.05);
        let straight = game.position.distance(start);

        game.position = start;
        game.keys.insert(KeyCode::KeyD);
        game.update(0.05);
        let diagonal = game.position.distance(start);
        assert!((straight - diagonal).abs() < 0.00001);
    }
}
