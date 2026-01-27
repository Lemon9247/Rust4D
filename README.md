# Rust4D

A 4D rendering engine in Rust. See real-time 3D cross-sections of 4D geometry.
Heavily inspired by [Engine4D](https://github.com/HackerPoet/Engine4D) by Code Parade

## What is this?

Rust4D slices four-dimensional objects with a 3D hyperplane, like slicing a 3D object reveals a 2D cross-section. Move through the W-axis and watch 4D shapes morph in ways impossible in 3D.

## Building & Running

```bash
cargo run --release
```

Requires Rust 1.70+ and a GPU with Vulkan/Metal support.

## Controls

| Input | Action |
|-------|--------|
| WASD | Move in XZ plane |
| Q/E | Move along W-axis |
| Space/Shift | Move up/down |
| Mouse | Look around |
| Right-click drag | Rotate through W |
| Scroll | Adjust slice offset |
| R | Reset camera |
| F | Fullscreen |
| ESC | Release cursor / Quit |

## Inspiration

- [4D Golf](https://store.steampowered.com/app/2147950/4D_Golf/) - Camera controls based on this
- [4D Toys](http://4dtoys.com/) - 4D physics visualization
- [Miegakure](https://miegakure.com/) - 4D puzzle platformer

## License

MIT
