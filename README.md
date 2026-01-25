# Rust4D

A 4D rendering engine written in Rust that displays real-time 3D cross-sections of 4D geometry.

## What is this?

Rust4D renders four-dimensional objects by slicing them with a 3D hyperplane, similar to how a 2D slice through a 3D object reveals a 2D cross-section. As you move through the fourth dimension (W-axis), you see the 4D shape morph and transform in ways impossible in 3D.

## Inspiration

This project draws inspiration from:

- **[4D Golf](https://store.steampowered.com/app/2147950/4D_Golf/)** by CodeParade - A brilliant golf game set in 4D space that pioneered intuitive 4D navigation controls. The camera system in Rust4D directly replicates 4D Golf's approach: standard mouse-look for 3D rotation, right-click drag for W-axis rotation, and Q/E keys for ana/kata movement.

- **[4D Toys](http://4dtoys.com/)** by Marc ten Bosch - Interactive 4D physics toys that demonstrate how 4D objects behave when projected into 3D.

- **[Miegakure](https://miegakure.com/)** - A 4D puzzle-platformer that explores 4D space through gameplay.

- **Geometric Algebra** - The rotation system uses rotors from geometric algebra rather than matrices, providing clean composition of rotations across all 6 rotation planes in 4D.

## Features

- Real-time cross-section rendering using GPU compute shaders
- 4D Golf-style camera controls
- W-depth coloring (blue for -W, red for +W)
- Tesseract (4D hypercube) visualization

## Controls

| Input | Action |
|-------|--------|
| **WASD** | Move in XZ plane |
| **Q/E** | Move along W-axis (ana/kata) |
| **Space/Shift** | Move up/down |
| **Mouse drag** | Rotate view (3D) |
| **Right-click drag** | Rotate through W-axis |
| **Scroll wheel** | Adjust slice offset |
| **R** | Reset camera |
| **F** | Toggle fullscreen |
| **ESC** | Quit |

## Building

```bash
cargo build --release
cargo run --release
```

Requires Rust 1.70+ and a GPU with Vulkan or Metal support.

## Architecture

```
rust4d/
├── crates/
│   ├── rust4d_math/     # Vec4, Rotor4 (4D rotation via geometric algebra)
│   ├── rust4d_render/   # wgpu rendering, compute shaders, geometry
│   └── rust4d_input/    # Camera controller
└── src/main.rs          # Application
```

### How it works

1. **Geometry**: A tesseract is decomposed into 24 five-cells (4D simplices) using Kuhn triangulation
2. **Slicing**: A GPU compute shader intersects each 5-cell with the W=slice_w hyperplane
3. **Rendering**: The resulting 3D triangles are rendered with W-depth coloring to show proximity to the slice plane

## The Fourth Dimension

In 4D space, there are 6 rotation planes instead of 3 rotation axes:
- **XY, XZ, YZ** - Standard 3D rotations
- **XW, YW, ZW** - Rotations involving the 4th dimension

When you rotate in the ZW plane (right-click drag), objects appear to turn "inside out" as parts hidden in the W dimension become visible.

## License

MIT

## References

- [Visualizing 4D Geometry](https://www.qfbox.info/4d/) - Comprehensive guide to 4D visualization
- [4D Euclidean Space](https://eusebeia.dyndns.org/4d/) - Mathematical foundations
- [Geometric Algebra for Computer Science](http://www.geometricalgebra.net/) - Rotors and multivectors
