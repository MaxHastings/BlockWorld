#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    GrassTop,
    GrassSide,
    Dirt,
    Stone,
    Andesite,
    Snow,
    SnowSide,
    TallGrass,
    Dandelion,
    OxeyeDaisy,
    Cornflower,
    SpruceLog,
    SpruceLogTop,
    SpruceLeaves,
}

// Layer order matches Material. Keep the supplied images intact: the GPU
// texture array selects a face without baking a new atlas or tinting dirt.
pub const TEXTURES: [&[u8]; 14] = [
    include_bytes!("../../res/blocks/grass_carried.png"),
    include_bytes!("../../res/blocks/grass_side_carried.png"),
    include_bytes!("../../res/blocks/dirt.png"),
    include_bytes!("../../res/blocks/stone.png"),
    include_bytes!("../../res/blocks/stone_andesite.png"),
    include_bytes!("../../res/blocks/snow.png"),
    include_bytes!("../../res/blocks/grass_side_snowed.png"),
    include_bytes!("../../res/blocks/double_plant_grass_carried.png"),
    include_bytes!("../../res/blocks/flower_dandelion.png"),
    include_bytes!("../../res/blocks/flower_oxeye_daisy.png"),
    include_bytes!("../../res/blocks/flower_cornflower.png"),
    include_bytes!("../../res/blocks/log_spruce.png"),
    include_bytes!("../../res/blocks/log_spruce_top.png"),
    include_bytes!("../../res/blocks/leaves_spruce_opaque.png"),
];
