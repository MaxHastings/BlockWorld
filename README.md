# BlockWorld

A small first-person procedural terrain explorer, now built with Rust and wgpu. It generates a block-shaped landscape from seeded elevation and terrain-region maps and lets you explore in flight or on foot. The Rust renderer uses Metal on macOS and DirectX 12 or Vulkan on Windows through wgpu.

## Run

Install the [Rust toolchain](https://rustup.rs/), then from this folder run:

```sh
cargo run --release
```

The game opens fullscreen on the primary display. For a regular window:

```sh
cargo run --release -- --windowed
```

Pass `--seed 42` to revisit a landscape. F5 saves the camera, seed, and sun position to `viewpoint.bwv`; F9 loads it. Use `--viewpoint FILE` to load and save a named viewpoint file. An explicit `--seed` overrides the seed in that file, so you can compare different landscapes from the same camera coordinates.

Use `--profile` to print chunk and shadow-field generation and upload times. It samples GPU frame time every 30 frames when timestamp queries are supported.

The same commands work on macOS and Windows. The first build downloads dependencies and takes longer. Press Escape to exit.

## Controls

| Key or input | Action |
| --- | --- |
| Mouse | Look around |
| W, A, S, D | Move |
| Left Shift | Fly 10 times faster |
| G | Toggle between flight and walking |
| Space | Jump while walking |
| R | Generate a new landscape |
| F5 / F9 | Save / load the viewpoint file |
| P | Pause or resume the sun and shadow movement |
| Escape | Close the game |

Flight is the default. Walking adds gravity, jumping, and collision with the heightmapped terrain. There are no caves or block editing; viewpoint files store camera and scene settings.

## Project layout

- `src/` contains the Rust game, terrain generator, wgpu renderer, and WGSL shader.
- `res/blocks/` holds the block and plant textures embedded by the Rust game.
- `res/textures.png` is retained for the earlier Java implementation.
- `Cargo.toml` declares the Rust dependencies.
- `REFACTORING.md` records the architecture review and completed refactoring plan.
- `src/Blockworld/`, `pom.xml`, and `run-blockworld.command` retain the earlier Java implementation for reference. The Rust executable is the current game.

The Rust renderer has a 600-second sunrise-to-sunrise cycle. The sun stays above the horizon for 300 seconds, with warm sunrise and sunset, a clear daytime sky, and a moonlit blue night with stars. Sun shadows trace rays through the terrain heights and tree canopies, so their edges follow the sun continuously. A background worker keeps a 1024-by-1024 height field around the player for these rays. Moonlight lights terrain without casting shadows.
