//! Shared, world-coordinate terrain decisions for meshing and vegetation.
//! The halo lets every chunk compute the same slope at its borders.

use super::{Shape, Terrain, CHUNK_SIZE};

// A wider slope describes the landform, while the nearby slope still controls
// plants and shallow soil. Using only nearby heights made cliff cover flicker
// between grass and rock from block to block.
const HALO: i32 = 12;
const NEAR_SPAN: i32 = 4;
const WIDTH: usize = (CHUNK_SIZE + HALO * 2) as usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ground {
    Grass { soil_depth: u8 },
    Rock { andesite: bool },
    Snow,
}

impl Ground {
    pub fn is_grass(self) -> bool {
        matches!(self, Self::Grass { .. })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Column {
    pub elevation: f32,
    pub height: i32,
    pub region: f32,
    pub slope: f32,
    pub moisture: f32,
    pub ground: Ground,
}

pub struct TerrainMap {
    shapes: Vec<Shape>,
    columns: Vec<Column>,
}

impl TerrainMap {
    pub fn build(terrain: &Terrain, chunk_x: i32, chunk_z: i32) -> Self {
        let origin_x = chunk_x * CHUNK_SIZE;
        let origin_z = chunk_z * CHUNK_SIZE;
        let mut shapes = vec![
            Shape {
                elevation: 0.0,
                region: 0.0
            };
            WIDTH * WIDTH
        ];
        for x in -HALO..CHUNK_SIZE + HALO {
            for z in -HALO..CHUNK_SIZE + HALO {
                shapes[index(x, z)] = terrain.shape_at(origin_x + x, origin_z + z);
            }
        }
        let mut columns = Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE) as usize);
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let wx = origin_x + x;
                let wz = origin_z + z;
                let shape = shapes[index(x, z)];
                let elevation = |dx, dz| shapes[index(x + dx, z + dz)].elevation;
                let slope = slope_from(NEAR_SPAN, elevation);
                let landform_slope = slope_from(HALO, elevation);
                columns.push(classify(terrain, wx, wz, shape, slope, landform_slope));
            }
        }
        Self { shapes, columns }
    }

    pub fn get(&self, x: i32, z: i32) -> Column {
        self.columns[(x * CHUNK_SIZE + z) as usize]
    }

    pub fn height(&self, x: i32, z: i32) -> i32 {
        self.shapes[index(x, z)].elevation.round() as i32
    }
}

pub fn column_at(terrain: &Terrain, x: i32, z: i32) -> Column {
    let shape = terrain.shape_at(x, z);
    let elevation = |dx, dz| terrain.elevation_at(x + dx, z + dz);
    let slope = slope_from(NEAR_SPAN, elevation);
    let landform_slope = slope_from(HALO, elevation);
    classify(terrain, x, z, shape, slope, landform_slope)
}

fn index(x: i32, z: i32) -> usize {
    ((x + HALO) as usize) * WIDTH + (z + HALO) as usize
}

fn slope_from(span: i32, mut elevation: impl FnMut(i32, i32) -> f32) -> f32 {
    let width = (span * 2) as f32;
    let dx = (elevation(span, 0) - elevation(-span, 0)) / width;
    let dz = (elevation(0, span) - elevation(0, -span)) / width;
    (dx * dx + dz * dz).sqrt()
}

fn classify(
    terrain: &Terrain,
    x: i32,
    z: i32,
    shape: Shape,
    slope: f32,
    landform_slope: f32,
) -> Column {
    let Shape { elevation, region } = shape;
    let (moisture, temperature) = terrain.surface_variation(x, z);
    let snowline = 89.0 + temperature * 11.0 - region * 6.0;
    let rockline = 81.0 + moisture * 13.0;
    let steepness = landform_slope * 0.8 + slope * 0.2;
    let exposed = steepness > 0.95 + moisture * 0.10 || elevation > rockline;
    // Snow collects across the summit but leaves the steepest faces exposed.
    let snow_limit = snowline + (landform_slope - 0.8).max(0.0) * 18.0;
    let ground = if elevation > snow_limit {
        Ground::Snow
    } else if exposed {
        Ground::Rock {
            andesite: terrain.andesite_at(x, elevation.round() as i32, z),
        }
    } else {
        let depth = (3.5 + moisture - slope * 1.8 - landform_slope * 0.7)
            .round()
            .clamp(1.0, 4.0) as u8;
        Ground::Grass { soil_depth: depth }
    };
    Column {
        elevation,
        height: elevation.round() as i32,
        region,
        slope,
        moisture,
        ground,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_cover_follows_landform_and_altitude() {
        let terrain = Terrain::with_seed(42);
        let shape = |elevation| Shape {
            elevation,
            region: 0.5,
        };
        assert!(classify(&terrain, 0, 0, shape(20.0), 0.1, 0.1)
            .ground
            .is_grass());
        assert!(matches!(
            classify(&terrain, 0, 0, shape(20.0), 1.5, 1.5).ground,
            Ground::Rock { .. }
        ));
        assert_eq!(
            classify(&terrain, 0, 0, shape(150.0), 0.5, 0.5).ground,
            Ground::Snow
        );
    }

    #[test]
    fn chunk_edges_match_point_samples() {
        let terrain = Terrain::with_seed(42);
        for chunk_x in [-2, 0, 1] {
            let map = TerrainMap::build(&terrain, chunk_x, -1);
            for x in [0, CHUNK_SIZE - 1] {
                for z in [0, CHUNK_SIZE - 1] {
                    let from_map = map.get(x, z);
                    let from_point = column_at(&terrain, chunk_x * CHUNK_SIZE + x, -CHUNK_SIZE + z);
                    assert_eq!(from_map.height, from_point.height);
                    assert_eq!(from_map.ground, from_point.ground);
                    assert!((from_map.slope - from_point.slope).abs() < 0.00001);
                }
            }
        }
    }
}
