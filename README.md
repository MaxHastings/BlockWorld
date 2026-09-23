# BlockWorld

A small first-person procedural terrain experiment from a college project. It builds a block-shaped surface from layered noise and lets you explore it in flight or on foot. The focus is the generated landscape and moving through it; terrain editing and world saving are not part of the project.

## Run on Mac

The included launcher targets Apple silicon Macs and requires Java 17 or newer, Maven, and an OpenGL-compatible macOS installation.

1. Open Terminal in this folder.
2. Run:

   ```sh
   ./run-blockworld.command
   ```

The first run downloads the Java dependencies through Maven, compiles the source, and opens the game window. To start in fullscreen, run `./run-blockworld.command --fullscreen`.

## Controls

| Key or input | Action |
| --- | --- |
| Mouse | Look around |
| W, A, S, D | Move |
| Left Shift | Fly 10 times faster |
| G | Toggle between flight and walking |
| Space | Jump while walking |
| R | Generate a new landscape |
| Escape | Close the game |

Flight is the default. Walking adds gravity, jumping, and collision with the heightmapped terrain. The world is a surface height map, so there are no caves, block editing, or save files.

## Project layout

- `src/Blockworld/` contains the Java game and GLSL shaders.
- `res/textures.png` is the terrain texture atlas; the other terrain images are retained source assets.
- `pom.xml` declares the Java and LWJGL dependencies used by the Mac launcher.
