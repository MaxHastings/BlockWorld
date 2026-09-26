# Foundation review (2026-09-24)

The playable implementation is Rust. The Java project remains a reference and
its existing launcher change is included in this branch; visual and simulation
findings below concern the active Rust game.

## Invariants checked

| Principle | Review result |
| --- | --- |
| Seed and world coordinates determine terrain | Elevation, cover, rock variation, trees, and meshes remain pure CPU decisions. Negative coordinates and chunk boundaries have tests. |
| Visible geometry and shadows describe the same world | The shader now uses the existing tree span texture. Tree cuboids and shadow spans come from one `Tree::boxes` description. Terrain heights remain shared by mesh, collision, and shadows. |
| Async work cannot restore obsolete state | Both chunk and shadow workers now reject results from an old seed or camera position. |
| Cross-module values have one owner | The day length comes from `Game`. Rust `Material` values generate the shader's material constants, avoiding hard-coded layer numbers in WGSL. |
| Movement and performance remain predictable | Diagonal flight is normalized. Formatting, Clippy, tests, release build, and a profiled Metal launch pass. |

## Remaining design choices

- Walking collides with the heightmapped ground, as documented in the README.
  Tree trunks and leaves remain decorative in physics. If trees should be solid,
  collision should use the same `Tree::boxes` geometry rather than new dimensions.
- Directional shadows cover up to 160 world units from the camera, fade over
  the final 32 units, and weaken near the horizon. Chunk residency can shorten
  that reach. Small plants and moonlight do not cast shadows.
- Rendering targets the configured surface resolution directly, without a
  lower-resolution scaling pass. Higher display resolutions use proportionally
  more fill rate and depth-buffer memory.
- `run-blockworld.command` still starts the archived Java game. `cargo run`
  starts the active Rust game. The README distinguishes them; the launcher
  should be repointed or renamed if Rust becomes the only supported path.

For an exact before-and-after view, save a scene with F5. A screenshot's seed
and position alone do not preserve the camera angle or sun time.
