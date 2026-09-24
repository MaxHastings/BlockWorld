//! Render a top-down surface map for comparing terrain rules at a fixed seed.
//! Usage: cargo run --release --example material_map -- SEED X Z SIZE OUTPUT.png

#[allow(dead_code)]
#[path = "../src/terrain.rs"]
mod terrain;

use image::{Rgb, RgbImage};
use terrain::map::{Ground, TerrainMap};
use terrain::{Terrain, CHUNK_SIZE};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 {
        return Err("usage: material_map SEED X Z SIZE OUTPUT.png".into());
    }
    let seed: u32 = args[1].parse()?;
    let center_x: i32 = args[2].parse()?;
    let center_z: i32 = args[3].parse()?;
    let size: u32 = args[4].parse()?;
    if size == 0 || size > 2048 {
        return Err("SIZE must be between 1 and 2048".into());
    }
    let terrain = Terrain::with_seed(seed);
    let mut image = RgbImage::new(size, size);
    let min_x = center_x - size as i32 / 2;
    let min_z = center_z - size as i32 / 2;
    let max_x = min_x + size as i32 - 1;
    let max_z = min_z + size as i32 - 1;
    for chunk_x in min_x.div_euclid(CHUNK_SIZE)..=max_x.div_euclid(CHUNK_SIZE) {
        for chunk_z in min_z.div_euclid(CHUNK_SIZE)..=max_z.div_euclid(CHUNK_SIZE) {
            let map = TerrainMap::build(&terrain, chunk_x, chunk_z);
            for x in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let world_x = chunk_x * CHUNK_SIZE + x;
                    let world_z = chunk_z * CHUNK_SIZE + z;
                    if world_x < min_x || world_x > max_x || world_z < min_z || world_z > max_z {
                        continue;
                    }
                    let column = map.get(x, z);
                    let base: [u8; 3] = match column.ground {
                        Ground::Grass { .. } => [83, 132, 48],
                        Ground::Rock { andesite: false } => [113, 116, 116],
                        Ground::Rock { andesite: true } => [143, 139, 132],
                        Ground::Snow => [238, 241, 238],
                    };
                    let shade = (0.82 + column.elevation * 0.0015).clamp(0.65, 1.1);
                    let color = base.map(|channel| (channel as f32 * shade).min(255.0) as u8);
                    image.put_pixel(
                        (world_x - min_x) as u32,
                        (world_z - min_z) as u32,
                        Rgb(color),
                    );
                }
            }
        }
    }
    image.save(&args[5])?;
    Ok(())
}
