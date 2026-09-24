# Visual direction

BlockWorld should read as a block-built alpine landscape from both walking and
flight views. Large landforms and material zones should be clear before small
texture details become visible.

## Surface rules

- Grass covers lower, gentler ground in connected areas. Near cliff edges the
  soil layer becomes thin, so dirt does not form speckles across rock faces.
- Rock follows broad steep landforms and remains visible on the steepest parts
  of high mountains. Its two existing textures add regional variation without
  changing the silhouette.
- Snow forms coherent high-altitude caps. The line varies slowly across the
  landscape and rises on steep faces.

## Next visual passes

1. Inspect saved flight and walking views at fixed seeds. The source screenshot
   uses seed `342688237` near `(-157, 33, -264)`; its camera angle was not saved.
2. Inspect the new world-aligned rock textures and height-varying rock patches
   on the broad stone faces. Add scree only if those faces still lack structure.
3. Add scale cues in the lower zones: tree groups, sparse alpine vegetation,
   and a few recognizable valley features.
4. Tune light, shadows, and distance haze after the surface regions read well.

Run `cargo run --release --example material_map -- SEED X Z SIZE OUTPUT.png`
to inspect top-down surface decisions without changing the game camera.
