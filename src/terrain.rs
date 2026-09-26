use std::time::{SystemTime, UNIX_EPOCH};

use noise::{NoiseFn, Perlin};

pub const CHUNK_SIZE: i32 = 64;
pub const VIEW_RADIUS: i32 = 6;
#[path = "terrain/map.rs"]
pub mod map;
#[path = "terrain/material.rs"]
pub mod material;
#[path = "terrain/mesh.rs"]
pub mod mesh;

#[derive(Clone)]
pub struct Terrain {
    seed: u32,
    noise: Perlin,
    generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tree {
    pub x: i32,
    pub z: i32,
    pub ground: i32,
    pub trunk_height: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreePart {
    Trunk,
    Leaves,
}

#[derive(Clone, Copy, Debug)]
pub struct TreeBox {
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub part: TreePart,
}

impl Tree {
    pub fn boxes(self) -> [TreeBox; 4] {
        let x = self.x as f32;
        let z = self.z as f32;
        let ground = self.ground as f32 + 0.5;
        let trunk_top = ground + self.trunk_height as f32;
        [
            TreeBox {
                min: [x - 0.5, ground, z - 0.5],
                max: [x + 0.5, trunk_top, z + 0.5],
                part: TreePart::Trunk,
            },
            TreeBox {
                min: [x - 2.5, trunk_top - 2.0, z - 2.5],
                max: [x + 2.5, trunk_top - 1.0, z + 2.5],
                part: TreePart::Leaves,
            },
            TreeBox {
                min: [x - 1.5, trunk_top - 1.0, z - 1.5],
                max: [x + 1.5, trunk_top + 1.0, z + 1.5],
                part: TreePart::Leaves,
            },
            TreeBox {
                min: [x - 0.5, trunk_top + 1.0, z - 0.5],
                max: [x + 0.5, trunk_top + 2.0, z + 0.5],
                part: TreePart::Leaves,
            },
        ]
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Shape {
    pub elevation: f32,
    pub region: f32,
}

impl Terrain {
    pub fn new() -> Self {
        Self::with_seed(random_seed())
    }

    pub fn with_seed(seed: u32) -> Self {
        Self {
            seed,
            noise: Perlin::new(seed),
            generation: 0,
        }
    }

    pub fn regenerate(&mut self) {
        let next = random_seed();
        let seed = if next == self.seed {
            next.wrapping_add(1)
        } else {
            next
        };
        self.set_seed(seed);
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    pub fn set_seed(&mut self, seed: u32) {
        if self.seed != seed {
            self.seed = seed;
            self.noise = Perlin::new(seed);
            self.generation = self.generation.wrapping_add(1);
        }
    }

    pub fn elevation_at(&self, x: i32, z: i32) -> f32 {
        self.shape_at(x, z).elevation
    }

    pub(crate) fn shape_at(&self, x: i32, z: i32) -> Shape {
        let sample = |scale: f64, dx: f64, dz: f64| {
            self.noise
                .get([x as f64 / scale + dx, z as f64 / scale + dz]) as f32
        };
        let region = self.region_at(x, z);
        let broad = sample(256.0, 0.0, 0.0) * 105.0;
        let ridges = (1.0 - sample(115.0, 31.0, -17.0).abs()) * 38.0 * region;
        let elevation = broad
            + ridges
            + sample(64.0, 0.0, 0.0) * 12.0
            + sample(18.0, 0.0, 0.0) * (2.0 + region * 3.0)
            + sample(5.0, 0.0, 0.0) * (0.7 + region);
        Shape { elevation, region }
    }

    fn region_at(&self, x: i32, z: i32) -> f32 {
        let value = self
            .noise
            .get([x as f64 / 360.0 + 91.0, z as f64 / 360.0 - 53.0]) as f32;
        smoothstep(-0.3, 0.4, value)
    }

    pub fn height_at(&self, x: i32, z: i32) -> i32 {
        self.elevation_at(x, z).round() as i32
    }

    pub fn ground_height(&self, x: f32, z: f32) -> f32 {
        self.height_at((x + 0.5).floor() as i32, (z + 0.5).floor() as i32) as f32 + 0.5
    }

    pub fn surface_variation(&self, x: i32, z: i32) -> (f32, f32) {
        let sample = |scale: f64, dx: f64, dz: f64| {
            self.noise
                .get([x as f64 / scale + dx, z as f64 / scale + dz]) as f32
        };
        (sample(110.0, 43.0, -77.0), sample(170.0, -131.0, 67.0))
    }

    pub fn andesite_at(&self, x: i32, y: i32, z: i32) -> bool {
        // The slow horizontal warp bends broad rock patches around the cliff.
        // Including height keeps a tall wall from inheriting one column color.
        let warp = self
            .noise
            .get([x as f64 / 88.0 + 211.0, z as f64 / 88.0 + 389.0])
            * 7.0;
        self.noise.get([
            x as f64 / 34.0 + 503.0,
            (y as f64 + warp) / 20.0 - 207.0,
            z as f64 / 34.0 + 139.0,
        ]) > 0.15
    }

    pub fn detail_hash(&self, x: i32, z: i32) -> u32 {
        let mut value =
            self.seed ^ (x as u32).wrapping_mul(0x9e3779b9) ^ (z as u32).wrapping_mul(0x85ebca6b);
        value ^= value >> 16;
        value = value.wrapping_mul(0x7feb352d);
        value ^= value >> 15;
        value = value.wrapping_mul(0x846ca68b);
        value ^ (value >> 16)
    }

    pub fn trees_in_region(&self, min_x: i32, min_z: i32, max_x: i32, max_z: i32) -> Vec<Tree> {
        const SPACING: i32 = 20;
        let mut trees = Vec::new();
        for cell_z in (min_z.div_euclid(SPACING))..=(max_z.div_euclid(SPACING)) {
            for cell_x in (min_x.div_euclid(SPACING))..=(max_x.div_euclid(SPACING)) {
                let hash = self.detail_hash(cell_x, cell_z);
                let x = cell_x * SPACING + 4 + ((hash >> 4) % 12) as i32;
                let z = cell_z * SPACING + 4 + ((hash >> 12) % 12) as i32;
                if x < min_x || x >= max_x || z < min_z || z >= max_z || hash.is_multiple_of(3) {
                    continue;
                }
                let column = map::column_at(self, x, z);
                let grove = self.surface_variation(x, z).0;
                if !column.ground.is_grass()
                    || column.slope > 0.35
                    || column.elevation > 65.0
                    || column.elevation < -55.0
                    || grove < 0.02
                    || hash.is_multiple_of(5)
                {
                    continue;
                }
                trees.push(Tree {
                    x,
                    z,
                    ground: column.height,
                    trunk_height: 4 + ((hash >> 24) % 3) as i32,
                });
            }
        }
        trees
    }
}

fn smoothstep(a: f32, b: f32, value: f32) -> f32 {
    let t = ((value - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn random_seed() -> u32 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (nanos as u32) ^ ((nanos >> 32) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_produces_same_heights() {
        let a = Terrain::with_seed(42);
        let b = Terrain::with_seed(42);
        for (x, z) in [(-65, -1), (0, 0), (63, 64), (512, -345)] {
            assert_eq!(a.height_at(x, z), b.height_at(x, z));
        }
    }

    #[test]
    fn regeneration_invalidates_chunk_generation() {
        let mut terrain = Terrain::with_seed(42);
        assert_eq!(terrain.generation(), 0);
        terrain.regenerate();
        assert_eq!(terrain.generation(), 1);
        assert_ne!(terrain.seed, 42);
    }
}
