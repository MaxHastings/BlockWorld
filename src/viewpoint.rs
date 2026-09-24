//! Human-readable scene snapshots for repeatable terrain comparisons.

use std::fs;
use std::path::Path;

use glam::Vec3;

use crate::game::{Game, DAY_LENGTH_SECONDS};

pub struct Viewpoint {
    seed: u32,
    position: Vec3,
    yaw: f32,
    pitch: f32,
    day_seconds: f32,
    sky_seconds: f32,
}

impl Viewpoint {
    pub fn from_game(game: &Game) -> Self {
        Self {
            seed: game.terrain.seed(),
            position: game.position,
            yaw: game.yaw,
            pitch: game.pitch,
            day_seconds: game.day_seconds,
            sky_seconds: game.sky_seconds,
        }
    }

    pub fn apply(self, game: &mut Game, seed_override: Option<u32>) {
        game.restore_viewpoint(
            seed_override.unwrap_or(self.seed),
            self.position,
            self.yaw,
            self.pitch,
            self.day_seconds,
            self.sky_seconds,
        );
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create {}: {e}", parent.display()))?;
        }
        let data = format!(
            "BlockWorld viewpoint 2\n{}\n{} {} {}\n{} {}\n{} {}\n",
            self.seed,
            self.position.x,
            self.position.y,
            self.position.z,
            self.yaw,
            self.pitch,
            self.day_seconds,
            self.sky_seconds
        );
        fs::write(path, data).map_err(|e| format!("Cannot save {}: {e}", path.display()))
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let data =
            fs::read_to_string(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
        Self::parse(&data)
    }

    fn parse(data: &str) -> Result<Self, String> {
        let mut lines = data.lines();
        let version = lines.next();
        if !matches!(
            version,
            Some("BlockWorld viewpoint 1" | "BlockWorld viewpoint 2")
        ) {
            return Err("Unsupported viewpoint format".into());
        }
        let parse = |line: Option<&str>, count: usize| -> Result<Vec<f32>, String> {
            let values: Vec<f32> = line
                .ok_or("Incomplete viewpoint")?
                .split_whitespace()
                .map(str::parse::<f32>)
                .collect::<Result<_, _>>()
                .map_err(|_| "Invalid viewpoint number")?;
            if values.len() != count || values.iter().any(|value| !value.is_finite()) {
                return Err("Invalid viewpoint values".into());
            }
            Ok(values)
        };
        let seed = lines
            .next()
            .ok_or("Missing viewpoint seed")?
            .parse::<u32>()
            .map_err(|_| "Invalid viewpoint seed")?;
        let position = parse(lines.next(), 3)?;
        let orientation = parse(lines.next(), 2)?;
        let time = parse(
            lines.next(),
            if version == Some("BlockWorld viewpoint 2") {
                2
            } else {
                1
            },
        )?;
        if !(-89.0..=89.0).contains(&orientation[1])
            || !(0.0..DAY_LENGTH_SECONDS).contains(&time[0])
        {
            return Err("Viewpoint angle or time is out of range".into());
        }
        Ok(Self {
            seed,
            position: Vec3::new(position[0], position[1], position[2]),
            yaw: orientation[0],
            pitch: orientation[1],
            day_seconds: time[0],
            sky_seconds: *time.get(1).unwrap_or(&time[0]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewpoint_restores_scene_and_seed() {
        let mut game = Game::with_seed(42);
        game.position = Vec3::new(-12.5, 80.0, 31.25);
        game.yaw = 123.0;
        game.pitch = -23.0;
        game.day_seconds = 150.0;
        game.sky_seconds = 943.5;
        let path =
            std::env::temp_dir().join(format!("blockworld-viewpoint-{}.bwv", std::process::id()));
        Viewpoint::from_game(&game).save(&path).unwrap();
        let mut restored = Game::with_seed(1);
        Viewpoint::load(&path).unwrap().apply(&mut restored, None);
        std::fs::remove_file(path).unwrap();
        assert_eq!(restored.terrain.seed(), 42);
        assert_eq!(restored.position, game.position);
        assert_eq!(
            (restored.yaw, restored.pitch, restored.day_seconds),
            (123.0, -23.0, 150.0)
        );
        assert_eq!(restored.sky_seconds, 943.5);
        assert!(restored.time_paused);
    }

    #[test]
    fn malformed_viewpoint_does_not_change_game() {
        let mut game = Game::with_seed(42);
        assert!(Viewpoint::parse("BlockWorld viewpoint 2\n42\n1 nan 3\n0 0\n0 0").is_err());
        assert!(Viewpoint::parse("BlockWorld viewpoint 2\n42\n1 2 3\n0 90\n0 0").is_err());
        assert_eq!(game.terrain.seed(), 42);
        Viewpoint::parse("BlockWorld viewpoint 1\n9\n1 2 3\n0 0\n10")
            .unwrap()
            .apply(&mut game, None);
        assert_eq!(game.terrain.seed(), 9);
        assert_eq!(game.sky_seconds, 10.0);
    }
}
