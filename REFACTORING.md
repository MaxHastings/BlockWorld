# Refactoring plan

## Review

The active game is the Rust implementation. The Java sources are retained as a reference and are outside this plan. The Rust game was small, but three concerns were tangled:

1. `Terrain` combined noise sampling, mesh construction, GPU buffers, and chunk residency. That made terrain logic dependent on a graphics device and hid regeneration invalidation inside game state.
2. Building a chunk ran during a redraw. A large mesh could delay input and camera updates.
3. `render.rs` mixed resource setup, scene lighting calculations, streaming, and render passes. The math was difficult to test without reading through graphics setup.

## Principles

- Keep generation deterministic and independent of wgpu. A seed and world coordinates should determine heights and mesh geometry.
- Keep GPU resources with the renderer that creates and uses them.
- Keep expensive CPU work off the redraw thread, while limiting queued work so movement does not generate obsolete chunks.
- Extract a module when it owns a distinct responsibility. Preserve the existing rendering behavior while moving code.
- Test boundaries where errors are easy to miss: negative coordinates, chunk seams, regeneration, and stale worker results.

## Execution

1. **Separate terrain data and GPU residency — complete.** `terrain.rs` now samples heights, `terrain/mesh.rs` builds vertices, and `render/chunks.rs` owns GPU chunk buffers and load radius decisions. The terrain generation number invalidates loaded chunks after regeneration.
2. **Move meshing off the redraw thread — complete.** A single worker builds one chunk at a time. The renderer uploads completed meshes and discards results from an older world or a position outside the active radius.
3. **Isolate scene calculations — complete.** `render/scene.rs` calculates camera, sky, lighting, and cascade matrices. `render.rs` retains graphics resource creation and render passes.
4. **Protect the boundaries — complete.** Tests cover deterministic heights, flat and adjacent chunk meshes, negative chunk coordinates, nearest chunk selection, stale results, and the sun path at sunrise, sunset, and day wrap. CI checks formatting, compilation, and tests on macOS and Windows.

## Future change guide

- Add a terrain feature by changing the height sampler or pure mesh builder first; keep GPU allocation in `render/chunks.rs`.
- Add a visual feature by changing `render/scene.rs` for frame values, shaders for pixel behavior, and `render.rs` for resources or passes. `render/heightfield.rs` supplies terrain heights for sun-ray shadows.
- If profiling shows GPU upload or draw calls dominate frame time, measure them before changing the one worker design or adding mesh batching.

## Terrain foundation (2026-09-24)

The active Rust path now follows a single direction of data flow:

`seed + world coordinates` → continuous elevation and region → chunk map with nearby and twelve-block landform slopes → ground and vegetation decisions → CPU mesh → GPU buffers.

- `Terrain::elevation_at` owns the continuous shape. `height_at` rounds it for collision and the shadow field. The chunk map rounds the same sampled elevation for visible geometry.
- `terrain/map.rs` owns per-column region, slope, moisture, and ground layers. Its halo gives matching classifications at chunk seams. The mesh consumes its ground decisions for tops and exposed walls; trees and plants use the same column classification.
- `Game` owns simulation and camera state. `viewpoint.rs` owns file parsing and writing; applying a validated snapshot restores game state in one operation. F5/F9 files preserve seed, position, orientation, and both lighting and wind clocks; `--seed` and `--viewpoint` make comparisons repeatable.
- The renderer continues to own workers, uploads, and draw calls. `--profile` reports CPU generation and upload times separately from GPU pass timestamps. GPU readback is asynchronous so measurement does not stall every sample. Both chunk meshes and the shadow field discard completed work for an old world or camera position.
- Tree cuboids come from one terrain description for the visible mesh and the sun shadow spans. The terrain shader samples the tree spans along the same rays it uses for ground shadows. Shader material constants are generated from the Rust `Material` enum.

On a local release run, chunks generated in about 1–2 ms and uploaded in about 0.4–0.8 ms. The 1024×1024 shadow field generated in about 85–102 ms and uploaded in about 2 ms. GPU frames were generally about 6–8 ms after streaming settled, with longer frames while many chunks loaded. These are observations on one machine, not budgets or cross-platform guarantees.
