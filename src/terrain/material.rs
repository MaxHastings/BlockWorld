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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShadowPolicy {
    Opaque,
    AlphaTested,
    None,
}

// Shared by shader generation and conservative CPU deformation bounds.
pub const WIND_AMPLITUDE: [f32; 2] = [0.045, 0.025];

impl Material {
    pub const ALL: [Self; 14] = [
        Self::GrassTop,
        Self::GrassSide,
        Self::Dirt,
        Self::Stone,
        Self::Andesite,
        Self::Snow,
        Self::SnowSide,
        Self::TallGrass,
        Self::Dandelion,
        Self::OxeyeDaisy,
        Self::Cornflower,
        Self::SpruceLog,
        Self::SpruceLogTop,
        Self::SpruceLeaves,
    ];

    pub fn wind(self) -> bool {
        matches!(
            self,
            Self::TallGrass | Self::Dandelion | Self::OxeyeDaisy | Self::Cornflower
        )
    }

    pub fn alpha_test(self) -> bool {
        self.wind() || self == Self::SpruceLeaves
    }

    pub fn shadow_policy(self) -> ShadowPolicy {
        match self {
            Self::TallGrass | Self::Dandelion | Self::OxeyeDaisy | Self::Cornflower => {
                ShadowPolicy::None
            }
            Self::SpruceLeaves => ShadowPolicy::AlphaTested,
            _ => ShadowPolicy::Opaque,
        }
    }
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
    include_bytes!("../../res/blocks/leaves_spruce_cutout.png"),
];
