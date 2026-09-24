use bytemuck::{Pod, Zeroable};

use super::map::{Column, Ground, TerrainMap};
use super::material::Material;
use super::{Terrain, Tree, TreePart, CHUNK_SIZE};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub material: u32,
}

pub struct ChunkMesh {
    pub terrain: Vec<Vertex>,
    pub plants: Vec<Vertex>,
}

#[derive(Clone, Copy)]
struct WallFace {
    normal: [f32; 3],
    a: [f32; 2],
    b: [f32; 2],
    x: i32,
    z: i32,
}

pub fn build_chunk(terrain: &Terrain, chunk_x: i32, chunk_z: i32) -> ChunkMesh {
    let map = TerrainMap::build(terrain, chunk_x, chunk_z);
    let mut mesh = build_chunk_with(
        chunk_x,
        chunk_z,
        |x, z| map.height(x - chunk_x * CHUNK_SIZE, z - chunk_z * CHUNK_SIZE),
        |x, z| map.get(x, z),
        |x, z| terrain.detail_hash(x, z),
        |x, y, z| {
            if terrain.andesite_at(x, y, z) {
                Material::Andesite
            } else {
                Material::Stone
            }
        },
    );
    let x = chunk_x * CHUNK_SIZE;
    let z = chunk_z * CHUNK_SIZE;
    for tree in terrain.trees_in_region(x, z, x + CHUNK_SIZE, z + CHUNK_SIZE) {
        tree_mesh(&mut mesh.terrain, tree);
    }
    mesh
}

#[cfg(test)]
fn build_chunk_from_heights(
    chunk_x: i32,
    chunk_z: i32,
    height_at: impl Fn(i32, i32) -> i32,
) -> Vec<Vertex> {
    build_chunk_with(
        chunk_x,
        chunk_z,
        &height_at,
        |x, z| Column {
            elevation: height_at(chunk_x * CHUNK_SIZE + x, chunk_z * CHUNK_SIZE + z) as f32,
            height: height_at(chunk_x * CHUNK_SIZE + x, chunk_z * CHUNK_SIZE + z),
            region: 0.0,
            slope: 0.0,
            moisture: 0.0,
            ground: Ground::Grass { soil_depth: 3 },
        },
        |_, _| u32::MAX,
        |_, _, _| Material::Stone,
    )
    .terrain
}

fn build_chunk_with(
    chunk_x: i32,
    chunk_z: i32,
    height_at: impl Fn(i32, i32) -> i32,
    column_at: impl Fn(i32, i32) -> Column,
    hash_at: impl Fn(i32, i32) -> u32,
    rock_at: impl Fn(i32, i32, i32) -> Material,
) -> ChunkMesh {
    let origin_x = chunk_x * CHUNK_SIZE;
    let origin_z = chunk_z * CHUNK_SIZE;
    let width = (CHUNK_SIZE + 2) as usize;
    let mut heights = vec![0_i32; width * width];
    let index = |x: i32, z: i32| ((x + 1) as usize) * width + (z + 1) as usize;
    for x in -1..=CHUNK_SIZE {
        for z in -1..=CHUNK_SIZE {
            heights[index(x, z)] = height_at(origin_x + x, origin_z + z);
        }
    }

    let mut vertices = Vec::with_capacity((CHUNK_SIZE * CHUNK_SIZE * 12) as usize);
    let mut plants = Vec::new();
    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let wx = origin_x + x;
            let wz = origin_z + z;
            let column = column_at(x, z);
            let height = column.height;
            let east = heights[index(x + 1, z)];
            let west = heights[index(x - 1, z)];
            let south = heights[index(x, z + 1)];
            let north = heights[index(x, z - 1)];
            let hash = hash_at(wx, wz);
            let top = height as f32 + 0.5;
            let fx = wx as f32;
            let fz = wz as f32;
            quad(
                &mut vertices,
                [0.0, 1.0, 0.0],
                [
                    [fx + 0.5, top, fz + 0.5],
                    [fx - 0.5, top, fz + 0.5],
                    [fx - 0.5, top, fz - 0.5],
                    [fx + 0.5, top, fz - 0.5],
                ],
                [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                top_material(column.ground),
            );
            let faces = [
                (
                    [1.0, 0.0, 0.0],
                    [fx + 0.5, fz + 0.5],
                    [fx + 0.5, fz - 0.5],
                    east,
                ),
                (
                    [-1.0, 0.0, 0.0],
                    [fx - 0.5, fz - 0.5],
                    [fx - 0.5, fz + 0.5],
                    west,
                ),
                (
                    [0.0, 0.0, 1.0],
                    [fx - 0.5, fz + 0.5],
                    [fx + 0.5, fz + 0.5],
                    south,
                ),
                (
                    [0.0, 0.0, -1.0],
                    [fx + 0.5, fz - 0.5],
                    [fx - 0.5, fz - 0.5],
                    north,
                ),
            ];
            for (normal, a, b, neighbor) in faces {
                let bottom = neighbor as f32 + 0.5;
                if top > bottom {
                    let face = WallFace {
                        normal,
                        a,
                        b,
                        x: wx,
                        z: wz,
                    };
                    wall(&mut vertices, face, top, bottom, column.ground, &rock_at);
                }
            }
            if column.slope < 0.5 {
                if let Some(plant) = plant_at(column, hash) {
                    plant_quads(&mut plants, fx, top, fz, hash, plant);
                }
            }
        }
    }
    ChunkMesh {
        terrain: vertices,
        plants,
    }
}

fn top_material(ground: Ground) -> Material {
    match ground {
        Ground::Grass { .. } => Material::GrassTop,
        Ground::Rock { andesite: true } => Material::Andesite,
        Ground::Rock { andesite: false } => Material::Stone,
        Ground::Snow => Material::Snow,
    }
}

fn wall(
    vertices: &mut Vec<Vertex>,
    face: WallFace,
    top: f32,
    bottom: f32,
    ground: Ground,
    rock_at: &impl Fn(i32, i32, i32) -> Material,
) {
    match ground {
        Ground::Grass { .. } if top - bottom <= 1.0 => {
            // A one-block terrace is grassy ground, not an exposed soil cliff.
            wall_segment(vertices, face, top, bottom, Material::GrassTop, false);
        }
        Ground::Grass { soil_depth } => {
            let cap_bottom = (top - 1.0).max(bottom);
            wall_segment(vertices, face, top, cap_bottom, Material::GrassSide, false);
            let soil_bottom = (top - soil_depth as f32).max(bottom);
            wall_segment(
                vertices,
                face,
                cap_bottom,
                soil_bottom,
                Material::Dirt,
                false,
            );
            rock_wall(vertices, face, soil_bottom, bottom, rock_at);
        }
        Ground::Snow => {
            let cap_bottom = (top - 1.0).max(bottom);
            wall_segment(vertices, face, top, cap_bottom, Material::SnowSide, false);
            rock_wall(vertices, face, cap_bottom, bottom, rock_at);
        }
        Ground::Rock { .. } => rock_wall(vertices, face, top, bottom, rock_at),
    }
}

fn rock_wall(
    vertices: &mut Vec<Vertex>,
    face: WallFace,
    top: f32,
    bottom: f32,
    rock_at: &impl Fn(i32, i32, i32) -> Material,
) {
    const BAND_HEIGHT: i32 = 8;
    let mut upper = top;
    let mut run_top = top;
    let mut run_material = None;
    while upper > bottom {
        let band = ((upper - 0.5).round() as i32).div_euclid(BAND_HEIGHT);
        let lower = ((band * BAND_HEIGHT) as f32 - 0.5).max(bottom);
        let y = ((upper + lower) * 0.5).round() as i32;
        let material = rock_at(face.x, y, face.z);
        if let Some(previous) = run_material {
            if previous != material {
                wall_segment(vertices, face, run_top, upper, previous, true);
                run_top = upper;
            }
        }
        run_material = Some(material);
        upper = lower;
    }
    if let Some(material) = run_material {
        wall_segment(vertices, face, run_top, bottom, material, true);
    }
}

fn wall_segment(
    vertices: &mut Vec<Vertex>,
    face: WallFace,
    upper: f32,
    lower: f32,
    material: Material,
    world_uv: bool,
) {
    if upper <= lower {
        return;
    }
    let (u0, u1, v0, v1) = if world_uv {
        let along_x = (face.a[0] - face.b[0]).abs() > 0.5;
        let u0 = if along_x { face.a[0] } else { face.a[1] };
        let u1 = if along_x { face.b[0] } else { face.b[1] };
        (u0, u1, -upper, -lower)
    } else {
        (0.0, 1.0, 0.0, upper - lower)
    };
    quad(
        vertices,
        face.normal,
        [
            [face.a[0], upper, face.a[1]],
            [face.b[0], upper, face.b[1]],
            [face.b[0], lower, face.b[1]],
            [face.a[0], lower, face.a[1]],
        ],
        [[u0, v0], [u1, v0], [u1, v1], [u0, v1]],
        material,
    );
}

fn plant_at(column: Column, hash: u32) -> Option<Material> {
    let roll = hash % 1000;
    let meadow = column.moisture > -0.1 && column.region < 0.55;
    match column.ground {
        Ground::Grass { .. } if meadow => match roll {
            0..=13 => Some(Material::Dandelion),
            14..=27 => Some(Material::OxeyeDaisy),
            28..=41 => Some(Material::Cornflower),
            42..=290 => Some(Material::TallGrass),
            _ => None,
        },
        Ground::Grass { .. } if column.moisture > -0.35 => match roll {
            0..=5 => Some(Material::Dandelion),
            6..=11 => Some(Material::OxeyeDaisy),
            12..=17 => Some(Material::Cornflower),
            18..=125 => Some(Material::TallGrass),
            _ => None,
        },
        _ => None,
    }
}

fn plant_quads(
    vertices: &mut Vec<Vertex>,
    x: f32,
    ground: f32,
    z: f32,
    hash: u32,
    material: Material,
) {
    let offset_x = ((hash >> 8 & 255) as f32 / 255.0 - 0.5) * 0.30;
    let offset_z = ((hash >> 16 & 255) as f32 / 255.0 - 0.5) * 0.30;
    let center_x = x + offset_x;
    let center_z = z + offset_z;
    let size = if material == Material::TallGrass {
        0.85
    } else {
        0.72
    };
    let half = size * 0.45;
    for (dx, dz) in [(half, half), (half, -half)] {
        quad(
            vertices,
            [0.0, 1.0, 0.0],
            [
                [center_x - dx, ground + size, center_z - dz],
                [center_x + dx, ground + size, center_z + dz],
                [center_x + dx, ground, center_z + dz],
                [center_x - dx, ground, center_z - dz],
            ],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            material,
        );
    }
}

fn tree_mesh(vertices: &mut Vec<Vertex>, tree: Tree) {
    for part in tree.boxes() {
        let (side, top) = match part.part {
            TreePart::Trunk => (Material::SpruceLog, Material::SpruceLogTop),
            TreePart::Leaves => (Material::SpruceLeaves, Material::SpruceLeaves),
        };
        cuboid(vertices, part.min, part.max, side, top);
    }
}

fn cuboid(vertices: &mut Vec<Vertex>, min: [f32; 3], max: [f32; 3], side: Material, top: Material) {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = max;
    let top_uv = [
        [0.0, 0.0],
        [x1 - x0, 0.0],
        [x1 - x0, z1 - z0],
        [0.0, z1 - z0],
    ];
    quad(
        vertices,
        [0.0, 1.0, 0.0],
        [[x1, y1, z1], [x0, y1, z1], [x0, y1, z0], [x1, y1, z0]],
        top_uv,
        top,
    );
    quad(
        vertices,
        [0.0, -1.0, 0.0],
        [[x0, y0, z1], [x1, y0, z1], [x1, y0, z0], [x0, y0, z0]],
        top_uv,
        side,
    );
    for (normal, a, b, length) in [
        ([1.0, 0.0, 0.0], [x1, z1], [x1, z0], z1 - z0),
        ([-1.0, 0.0, 0.0], [x0, z0], [x0, z1], z1 - z0),
        ([0.0, 0.0, 1.0], [x0, z1], [x1, z1], x1 - x0),
        ([0.0, 0.0, -1.0], [x1, z0], [x0, z0], x1 - x0),
    ] {
        quad(
            vertices,
            normal,
            [
                [a[0], y1, a[1]],
                [b[0], y1, b[1]],
                [b[0], y0, b[1]],
                [a[0], y0, a[1]],
            ],
            [[0.0, 0.0], [length, 0.0], [length, y1 - y0], [0.0, y1 - y0]],
            side,
        );
    }
}

fn quad(
    vertices: &mut Vec<Vertex>,
    normal: [f32; 3],
    positions: [[f32; 3]; 4],
    uv: [[f32; 2]; 4],
    material: Material,
) {
    for i in [0, 1, 2, 0, 2, 3] {
        vertices.push(Vertex {
            position: positions[i],
            normal,
            uv: uv[i],
            material: material as u32,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_chunk_has_only_top_faces() {
        let mesh = build_chunk_from_heights(0, 0, |_, _| 5);
        assert_eq!(mesh.len(), (CHUNK_SIZE * CHUNK_SIZE * 6) as usize);
        assert!(mesh.iter().all(|vertex| vertex.normal == [0.0, 1.0, 0.0]));
        assert!(mesh.iter().all(|vertex| vertex.position[1] == 5.5));
        assert!(mesh
            .iter()
            .all(|vertex| vertex.material == Material::GrassTop as u32));
    }

    #[test]
    fn adjacent_chunks_emit_only_the_higher_boundary_wall() {
        let height = |x, _| if x < CHUNK_SIZE { 2 } else { 1 };
        let left = build_chunk_from_heights(0, 0, height);
        let right = build_chunk_from_heights(1, 0, height);
        let wall_vertices = (CHUNK_SIZE * 6) as usize;
        let top_vertices = (CHUNK_SIZE * CHUNK_SIZE * 6) as usize;
        assert_eq!(left.len(), top_vertices + wall_vertices);
        assert_eq!(right.len(), top_vertices);
        assert_eq!(
            left.iter()
                .filter(|vertex| vertex.normal == [1.0, 0.0, 0.0])
                .count(),
            wall_vertices
        );
        assert_eq!(left.last().unwrap().material, Material::GrassTop as u32);
    }

    #[test]
    fn cliffs_expose_stone_below_the_soil() {
        let mesh = build_chunk_from_heights(0, 0, |x, _| if x == 0 { 4 } else { 0 });
        assert!(mesh
            .iter()
            .any(|vertex| vertex.material == Material::Stone as u32));
    }

    #[test]
    fn ground_layers_use_the_column_decision() {
        assert_eq!(top_material(Ground::Snow), Material::Snow);
        assert_eq!(
            top_material(Ground::Grass { soil_depth: 4 }),
            Material::GrassTop
        );
        assert_eq!(
            top_material(Ground::Rock { andesite: true }),
            Material::Andesite
        );
    }

    #[test]
    fn tall_rock_wall_changes_material_with_height_and_aligns_texture() {
        let mut vertices = Vec::new();
        rock_wall(
            &mut vertices,
            WallFace {
                normal: [1.0, 0.0, 0.0],
                a: [4.5, 2.5],
                b: [4.5, 1.5],
                x: 4,
                z: 2,
            },
            16.5,
            -0.5,
            &|_, y, _| {
                if y >= 8 {
                    Material::Andesite
                } else {
                    Material::Stone
                }
            },
        );
        assert_eq!(vertices.len(), 12);
        assert!(vertices[..6]
            .iter()
            .all(|vertex| vertex.material == Material::Andesite as u32));
        assert!(vertices[6..]
            .iter()
            .all(|vertex| vertex.material == Material::Stone as u32));
        assert_eq!(vertices[0].uv, [2.5, -16.5]);
        assert_eq!(vertices[11].uv, [2.5, 0.5]);
    }
}
