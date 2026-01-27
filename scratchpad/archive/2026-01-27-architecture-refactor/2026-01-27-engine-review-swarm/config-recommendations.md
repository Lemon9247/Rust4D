# Configuration System Recommendations for Rust4D

**Agent**: Config Agent
**Date**: 2026-01-27
**Mission**: Research configuration systems and recommend an approach for Rust4D

## Executive Summary

After analyzing the Rust4D codebase and researching Rust game engine configuration patterns, I recommend implementing a **hierarchical configuration system using Figment with TOML as the primary format**. This will replace the 40+ hardcoded constants scattered throughout the codebase with a flexible, user-friendly configuration system.

## Current State: Hardcoded Values Analysis

### Critical Hardcoded Values in Main Application

**File**: `/home/lemoneater/Projects/Personal/Rust4D/src/main.rs`
- `GRAVITY: f32 = -20.0` (line 46)
- `FLOOR_Y: f32 = -2.0` (line 49)
- Window title: "Rust4D - 4D Rendering Engine" (line 145)
- Window size: `1280x720` (line 146)
- Scene parameters:
  - Player start position: `Vec4::new(0.0, 0.0, 5.0, 0.0)` (line 54)
  - Player radius: `0.5` (line 58)
  - Floor size: `10.0` (line 57)
  - Tesseract position: `Vec4::ZERO` (line 59)
  - Tesseract size: `2.0` (line 59)

### Camera and Input Controller Hardcoded Values

**File**: `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_input/src/camera_controller.rs`
- `move_speed: 3.0` (line 77)
- `w_move_speed: 2.0` (line 78)
- `mouse_sensitivity: 0.002` (line 79)
- `w_rotation_sensitivity: 0.005` (line 80)
- `smoothing_half_life: 0.05` (line 81)
- `smoothing_enabled: false` (line 82)

**File**: `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_render/src/camera4d.rs`
- `PITCH_LIMIT: 1.553` (89 degrees) (line 49)
- Default camera position: `Vec4::new(0.0, 0.0, 5.0, 0.0)` (line 54)

### Physics System Hardcoded Values

**File**: `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_physics/src/world.rs`
- `DEFAULT_JUMP_VELOCITY: 8.0` (line 32)
- Default gravity: `-20.0` (line 19)
- `GROUND_NORMAL_THRESHOLD: 0.7` (line 272)

**File**: `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_physics/src/player.rs`
- `DEFAULT_PLAYER_RADIUS: 0.5` (line 10)
- `DEFAULT_JUMP_VELOCITY: 8.0` (line 13)
- `GROUND_MARGIN: 0.01` (line 101)

**File**: `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_physics/src/material.rs`
- Material presets (ICE, RUBBER, METAL, WOOD, CONCRETE) with friction and restitution values (lines 26-56)

### Rendering Pipeline Hardcoded Values

**File**: `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_render/src/pipeline/types.rs`
- `MAX_OUTPUT_TRIANGLES: 100_000` (line 208)
- Default light direction: `[0.5, 1.0, 0.3]` (line 190)
- `ambient_strength: 0.3` (line 192)
- `diffuse_strength: 0.7` (line 193)
- `w_color_strength: 0.5` (line 194)
- `w_range: 2.0` (line 195)

**File**: `/home/lemoneater/Projects/Personal/Rust4D/src/main.rs` (render uniforms)
- FOV: `std::f32::consts::FRAC_PI_4` (45 degrees) (line 391)
- Near plane: `0.1` (line 393)
- Far plane: `100.0` (line 394)
- Background color: `r: 0.02, g: 0.02, b: 0.08, a: 1.0` (lines 465-468)

### Scene Builder Hardcoded Values

**File**: `/home/lemoneater/Projects/Personal/Rust4D/src/scene/scene_builder.rs`
- Floor subdivisions: `10` (line 63)
- Floor cell size: `size / 2.0` (line 63)
- Floor thickness: `0.001` (line 63)
- Tesseract mass: `10.0` (line 112)

## Configuration Crate Comparison

### Option 1: `toml` + `serde` (Manual)

**Pros:**
- Lightweight (only 2 dependencies)
- Full control over loading logic
- Simple and transparent
- No learning curve

**Cons:**
- No hierarchical overrides (env vars, CLI args)
- Manual file handling code
- No hot-reloading support
- Must implement fallbacks manually

**Best for:** Simple projects with static config files

### Option 2: `config-rs`

**Pros:**
- Hierarchical overrides (files → env vars)
- Multiple format support (TOML, JSON, YAML, RON, etc.)
- Environment variable prefixing
- Established and stable (2.2M downloads/month)

**Cons:**
- More complex API
- Less flexible than Figment
- Harder to merge custom sources
- Somewhat dated design patterns

**Best for:** Traditional 12-factor web applications

### Option 3: `figment` (RECOMMENDED)

**Pros:**
- Semi-hierarchical configuration merging
- Extremely flexible provider system
- Built-in support for TOML, JSON, env vars
- Can merge from Serialize types (CLI args via clap)
- Excellent error reporting
- Clean, modern API
- Used by Rocket web framework

**Cons:**
- Slightly smaller community than config-rs
- More advanced features may have learning curve

**Best for:** Game engines needing flexible, layered configuration

### Verdict: Figment

Figment is the best choice for Rust4D because:
1. Games need to merge config from multiple sources (default → user config → CLI args)
2. Supports direct integration with CLI argument parsing
3. Modern, well-designed API that fits Rust idioms
4. Excellent for development (can override settings without editing config files)

## Recommended Configuration Format: TOML

**Why TOML?**
- Human-readable and editable
- Native Rust ecosystem standard (Cargo.toml)
- Good for hierarchical but not deeply nested data
- Better than JSON for config files (allows comments)
- More compact than YAML
- Simpler than RON for non-programmers

**When to use alternatives:**
- **JSON**: Machine-generated configs, web API integration
- **RON**: Complex nested data structures, Rust-native types
- **ENV**: Runtime overrides, containerized deployments

## Recommended Configuration File Structure

### `config/default.toml` (Checked into version control)

```toml
# Rust4D Engine Configuration
# This is the default configuration checked into version control.
# Override settings by creating config/user.toml (gitignored)

[window]
title = "Rust4D - 4D Rendering Engine"
width = 1280
height = 720
fullscreen = false
vsync = true

[camera]
# Starting position (x, y, z, w)
start_position = [0.0, 0.0, 5.0, 0.0]
# Field of view in degrees
fov = 45.0
near_plane = 0.1
far_plane = 100.0
# Pitch limit in degrees (prevents gimbal lock)
pitch_limit = 89.0

[input]
# Movement speed in units/second
move_speed = 3.0
# 4D (W-axis) movement speed in units/second
w_move_speed = 2.0
# Mouse look sensitivity
mouse_sensitivity = 0.002
# 4D rotation sensitivity (right-click drag)
w_rotation_sensitivity = 0.005
# Input smoothing (exponential, half-life in seconds)
smoothing_enabled = false
smoothing_half_life = 0.05

[physics]
# Gravity acceleration (negative = downward)
gravity = -20.0
# Player jump velocity
jump_velocity = 8.0
# Player collision radius
player_radius = 0.5
# Ground detection threshold (dot product of normal with up)
ground_normal_threshold = 0.7
# Time step for physics simulation (fixed timestep)
# Set to 0.0 for variable timestep
fixed_timestep = 0.0

# Physics materials presets
# These can be overridden per-material in scene files
[physics.materials.ice]
friction = 0.05
restitution = 0.1

[physics.materials.rubber]
friction = 0.9
restitution = 0.8

[physics.materials.metal]
friction = 0.3
restitution = 0.3

[physics.materials.wood]
friction = 0.5
restitution = 0.2

[physics.materials.concrete]
friction = 0.7
restitution = 0.1

[rendering]
# Maximum triangles in output buffer
max_output_triangles = 100_000
# Background color (RGB, 0.0-1.0)
background_color = [0.02, 0.02, 0.08]

# Lighting configuration
[rendering.lighting]
# Light direction (normalized in shader)
light_direction = [0.5, 1.0, 0.3]
ambient_strength = 0.3
diffuse_strength = 0.7
# W-depth color mixing strength
w_color_strength = 0.5
# W-depth color range
w_range = 2.0

# Scene configuration (loaded at startup)
[scene]
# Player starting position
player_start = [0.0, 0.0, 5.0, 0.0]
player_radius = 0.5

# Floor configuration
[scene.floor]
enabled = true
y_position = -2.0
size = 10.0
subdivisions = 10
material = "concrete"

# Objects in the scene
[[scene.objects]]
type = "tesseract"
name = "main_tesseract"
position = [0.0, 0.0, 0.0, 0.0]
size = 2.0
mass = 10.0
material = "wood"

[debug]
# Show debug overlay with FPS, position, etc.
overlay = false
# Log level: "error", "warn", "info", "debug", "trace"
log_level = "info"
# Show collision boxes
show_colliders = false
# Show 4D rotation axes visualization
show_4d_axes = false
```

### `config/user.toml` (User overrides, gitignored)

```toml
# User Configuration Overrides
# This file is not tracked by git - customize your settings here!

# Example: Faster movement for testing
[input]
move_speed = 10.0

# Example: Different starting position
[scene]
player_start = [5.0, 2.0, 0.0, 1.0]

# Example: Enable debug overlay
[debug]
overlay = true
log_level = "debug"
```

### `.gitignore` additions

```
# User configuration (don't commit personal settings)
config/user.toml
```

## Implementation Approach

### Phase 1: Add Configuration Infrastructure (0.5 sessions)

1. Add dependencies to `Cargo.toml`:
```toml
[dependencies]
figment = { version = "0.10", features = ["toml", "env"] }
serde = { version = "1.0", features = ["derive"] }
```

2. Create `src/config.rs` with config structs:
```rust
use serde::{Deserialize, Serialize};
use figment::{Figment, providers::{Format, Toml, Env}};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub window: WindowConfig,
    pub camera: CameraConfig,
    pub input: InputConfig,
    pub physics: PhysicsConfig,
    pub rendering: RenderingConfig,
    pub scene: SceneConfig,
    pub debug: DebugConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, figment::Error> {
        Figment::new()
            // 1. Default config (checked into git)
            .merge(Toml::file("config/default.toml"))
            // 2. User overrides (gitignored)
            .merge(Toml::file("config/user.toml"))
            // 3. Environment variables (R4D_ prefix)
            .merge(Env::prefixed("R4D_"))
            .extract()
    }
}
```

### Phase 2: Migrate Hardcoded Values (1-2 sessions)

**Priority 1 (Critical gameplay values):**
- Window configuration
- Camera settings
- Input controller settings
- Physics parameters
- Scene initial state

**Priority 2 (Rendering parameters):**
- Lighting configuration
- Pipeline buffer sizes
- Rendering quality settings

**Priority 3 (Debug/development):**
- Debug overlay settings
- Logging configuration

### Phase 3: Testing and Documentation (0.5 sessions)

1. Create example configs
2. Test environment variable overrides
3. Document configuration options in README.md
4. Add config validation and helpful error messages

### Phase 4: Advanced Features (Future)

- Hot-reload configuration during development
- In-game settings menu that writes to user.toml
- Scene file loader (separate from engine config)
- Profile system (e.g., `config/profiles/low-spec.toml`)

## Example Usage Patterns

### Developer Override via Environment Variables

```bash
# Test with low gravity
R4D_PHYSICS__GRAVITY=-5.0 cargo run

# Test with different window size
R4D_WINDOW__WIDTH=1920 R4D_WINDOW__HEIGHT=1080 cargo run

# Enable debug overlay
R4D_DEBUG__OVERLAY=true cargo run
```

### Scene-Specific Overrides

```bash
# Load a different scene config
cargo run -- --scene config/scenes/test_chamber.toml
```

### Build-Time Optimization

For release builds, embed default config:
```rust
const DEFAULT_CONFIG: &str = include_str!("../config/default.toml");
```

## Alternative: RON for Scene Files

While TOML is great for engine configuration, consider **RON (Rusty Object Notation)** for **scene definition files**:

**Advantages:**
- Native Rust types
- Better for complex nested structures
- Can represent Vec4, colors, etc. naturally
- Comments and cleaner syntax for game data

**Example**: `scenes/test_chamber.ron`
```ron
Scene(
    entities: [
        Entity(
            name: "player",
            transform: Transform(
                position: (0.0, 1.0, 5.0, 0.0),
                rotation: Identity,
            ),
            components: [
                Player(
                    radius: 0.5,
                    jump_velocity: 8.0,
                ),
            ],
        ),
        Entity(
            name: "floor",
            components: [
                Floor(
                    y: -2.0,
                    size: 20.0,
                    material: Concrete,
                ),
            ],
        ),
        // ... more entities
    ],
)
```

## Recommendations Summary

### Immediate Actions

1. **Add Figment dependency** with TOML support
2. **Create `config/` directory structure**:
   - `config/default.toml` (version controlled)
   - `config/user.toml` (gitignored)
   - `config/scenes/` (scene definitions)
3. **Implement `AppConfig` struct** with serde Deserialize
4. **Migrate critical hardcoded values** to config (window, camera, input, physics)

### Configuration Strategy

- **Engine settings**: TOML files (`config/default.toml`, `config/user.toml`)
- **Scene data**: RON files (`scenes/*.ron`) - Future enhancement
- **Runtime overrides**: Environment variables (`R4D_*`)
- **Build profiles**: Separate config files per platform/quality level

### Benefits

1. **User-friendly**: Players can tweak settings without rebuilding
2. **Developer-friendly**: Quick iteration via env vars and user.toml
3. **Modding-friendly**: Clear config structure for community
4. **Production-ready**: Hierarchical overrides for deployment
5. **Maintainable**: Centralized configuration, not scattered constants

## Additional Considerations

### Config Validation

Add validation for config values:
```rust
impl AppConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.physics.gravity > 0.0 {
            return Err(ConfigError::InvalidGravity);
        }
        if self.window.width < 640 || self.window.height < 480 {
            return Err(ConfigError::WindowTooSmall);
        }
        Ok(())
    }
}
```

### Config Migration

For version upgrades, provide migration helpers:
```rust
impl AppConfig {
    pub fn migrate_from_v1(old: ConfigV1) -> Self {
        // Handle config format changes between versions
    }
}
```

### Platform-Specific Defaults

```rust
#[cfg(target_os = "windows")]
const DEFAULT_CONFIG: &str = include_str!("../config/default-windows.toml");

#[cfg(target_os = "linux")]
const DEFAULT_CONFIG: &str = include_str!("../config/default-linux.toml");
```

## References and Research Sources

### Configuration Crate Documentation
- [toml - Rust](https://docs.rs/toml/latest/toml/)
- [figment - Rust](https://docs.rs/figment/latest/figment/)
- [config - Rust](https://docs.rs/config/latest/config/)
- [GitHub - Figment Repository](https://github.com/SergioBenitez/Figment)

### Rust Configuration Best Practices
- [How to Work With TOML Files in Rust](https://www.makeuseof.com/working-with-toml-files-in-rust/)
- [Configuration management in Rust web services - LogRocket Blog](https://blog.logrocket.com/configuration-management-in-rust-web-services/)
- [Rust hierarchical configuration with clap and figment](https://steezeburger.com/2023/03/rust-hierarchical-configuration/)

### Game Engine References
- [Bevy Game Engine Configuration](https://docs.rs/bevy)
- [Fyrox - Config Documentation](https://fyrox.rs/config/)
- [GitHub - FyroxEngine/Fyrox](https://github.com/FyroxEngine/Fyrox)

## Conclusion

Implementing a Figment-based configuration system will transform Rust4D from a hardcoded prototype into a flexible, user-configurable game engine. The hierarchical override system (default → user → env vars) provides excellent developer experience while maintaining simplicity for end users.

**Estimated Implementation Time**: 2-3 sessions total
- Phase 1 (Infrastructure): 0.5 sessions
- Phase 2 (Migration): 1-2 sessions
- Phase 3 (Testing/Docs): 0.5 sessions

**Priority**: Medium-High - Not blocking current development, but essential for moving beyond prototype stage and enabling user customization.
