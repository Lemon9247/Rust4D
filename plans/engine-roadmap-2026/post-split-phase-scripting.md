# Post-Split Phase: Lua Scripting Engine

**Date**: 2026-01-31
**Status**: Planning Document (Implementation-Ready)
**Depends On**: Engine/Game Split (ECS migration done, `rust4d_game` crate exists), Post-Split Phases P1-P4 (APIs to bind)
**Total Engine Estimate**: 8-11 sessions
**Crate**: `rust4d_scripting` (NEW)

---

## 1. Overview

### Why Lua

Lua is the right scripting language for Rust4D:

- **Fast**: LuaJIT achieves 2-5x slower than native C in numeric code; standard Lua 5.4 is 10-50x slower but sufficient for game logic at our entity counts
- **Embeddable**: Designed from the ground up as an embedded language -- ~200KB runtime footprint, zero external dependencies
- **Proven in games**: Used in World of Warcraft, Roblox, Garry's Mod, Love2D, PICO-8, Factorio, and dozens of other shipped titles
- **Hot-reloadable**: Scripts can be loaded, unloaded, and re-executed at runtime without restarting the engine
- **Small runtime**: The entire Lua VM fits in a few hundred KB, no JVM-style startup cost
- **Simple**: A non-Rust developer (level designer, gameplay programmer) can write Lua in hours, not weeks

### Why mlua

`mlua` is the recommended Rust binding crate:

- **Lua 5.4 support**: Full Lua 5.4 implementation with integers, bitwise operators, and generational GC
- **LuaJIT option**: Can be compiled with LuaJIT backend for performance-critical deployments via feature flag
- **Active maintenance**: Regular releases, responsive maintainer, good issue tracker
- **Userdata/metatable support**: First-class support for wrapping Rust types as Lua userdata with metatables for operator overloading (critical for `Vec4 + Vec4`, etc.)
- **Async support**: Optional async Lua coroutines (useful for cutscenes, sequenced events)
- **Serialize feature**: Integration with serde for automatic table-to-struct conversion
- **Safety**: All Lua operations return `Result`, preventing panics from script errors

### What This Phase Creates

This phase creates the `rust4d_scripting` crate, which:

1. Wraps a Lua 5.4 VM (via `mlua`) with engine-specific configuration
2. Loads and manages game scripts from a `scripts/` directory
3. Integrates with the engine loop via lifecycle callbacks (`on_init`, `on_update`, `on_fixed_update`, `on_shutdown`)
4. Exposes all engine subsystem APIs to Lua (math, physics, input, audio, rendering, scene, ECS)
5. Supports hot-reload of `.lua` files during development
6. Provides the `rust4d_game` framework types to Lua (CharacterController4D, FSM, events, tweens)

After this phase, the game repository contains Lua scripts + RON scenes + assets -- no compiled Rust game code.

---

## 2. Architecture

### 2.1 Crate Structure

```
crates/rust4d_scripting/
  Cargo.toml
  src/
    lib.rs                  # Public API, ScriptEngine, re-exports
    vm.rs                   # Lua VM initialization, configuration, sandboxing
    loader.rs               # Script loading, require() resolution, module caching
    lifecycle.rs            # Game loop integration (on_init, on_update, etc.)
    hot_reload.rs           # File watcher, module reload, state preservation
    error.rs                # ScriptError type, error formatting, stack traces
    bindings/
      mod.rs                # Binding registration orchestrator
      math.rs               # Vec4, Rotor4, Transform4D bindings
      physics.rs            # PhysicsWorld, raycasting, collision queries
      input.rs              # InputAction, InputMap bindings
      audio.rs              # AudioEngine4D bindings
      render.rs             # ParticleSystem, SpriteBatch, HUD bindings
      scene.rs              # Scene loading, entity templates
      ecs.rs                # hecs World bindings (spawn, query, get/set components)
      assets.rs             # Asset loading, cache queries
      game.rs               # CharacterController4D, events, FSM, tweens, screen shake
```

### 2.2 Cargo.toml

```toml
[package]
name = "rust4d_scripting"
version = "0.1.0"
edition = "2021"

[dependencies]
rust4d_math = { path = "../rust4d_math" }
rust4d_core = { path = "../rust4d_core" }
rust4d_physics = { path = "../rust4d_physics" }
rust4d_input = { path = "../rust4d_input" }
rust4d_render = { path = "../rust4d_render" }
rust4d_audio = { path = "../rust4d_audio" }
rust4d_game = { path = "../rust4d_game" }

mlua = { version = "0.10", features = ["lua54", "serialize", "send"] }
notify = "7.0"
log = "0.4"
serde = { version = "1.0", features = ["derive"] }

[features]
default = ["lua54"]
lua54 = []
luajit = ["mlua/luajit"]
hot-reload = ["notify"]
```

### 2.3 How the Lua VM Integrates with the Engine Loop

The engine binary (`rust4d` or a launcher) creates a `ScriptEngine` and calls its lifecycle methods at the appropriate points in the game loop:

```
Engine Startup
  |
  +--> ScriptEngine::new(config)           -- Create Lua VM, register all bindings
  +--> ScriptEngine::load_game("scripts/") -- Load main.lua, resolve requires
  +--> ScriptEngine::call_init()           -- Call Lua on_init()
  |
Main Loop (each frame):
  |
  +--> ScriptEngine::call_update(dt)        -- Call Lua on_update(dt)
  +--> ScriptEngine::call_fixed_update(dt)  -- Call Lua on_fixed_update(dt) [fixed timestep]
  +--> ScriptEngine::dispatch_events(events) -- Forward engine events to Lua handlers
  +--> ScriptEngine::check_hot_reload()     -- Poll for changed files, reload if needed
  |
Engine Shutdown:
  |
  +--> ScriptEngine::call_shutdown()        -- Call Lua on_shutdown()
  +--> ScriptEngine drops                   -- Lua VM cleaned up
```

### 2.4 Script Lifecycle

1. **Loading**: `main.lua` is the entry point. It uses `require()` to load other modules. The engine configures the Lua `package.path` to resolve from the game's `scripts/` directory.

2. **Initialization**: After all scripts are loaded, `on_init()` is called. Scripts register their systems, spawn initial entities, set up event handlers.

3. **Per-frame callbacks**: Each frame, `on_update(dt)` is called for variable-timestep logic (rendering prep, input polling, UI). `on_fixed_update(dt)` is called at the fixed physics rate for deterministic game logic (movement, AI, physics queries).

4. **Event dispatch**: Engine collision events, trigger events, and game events are forwarded to Lua event handlers registered via `events.on("event_name", callback)`.

5. **Hot-reload**: During development, the file watcher detects `.lua` file changes. Changed modules are re-executed, `on_reload()` callbacks fire, and the game continues with updated logic.

### 2.5 Memory Model

- **Lua owns game state**: Health, ammo, weapon inventories, AI state, scores, timers -- all live in Lua tables. This is the natural model for a scripted game.
- **Rust owns engine state**: PhysicsWorld, render pipelines, audio engine, GPU resources, ECS World -- all live in Rust. Lua accesses these through userdata handles and function bindings.
- **Bridge via userdata**: Rust types exposed to Lua are wrapped as mlua `UserData` with metatable methods. Lua code calls methods on these handles which dispatch to Rust implementations. The Lua GC manages the userdata lifetime; Rust resources use reference counting or are owned by the engine.
- **ECS bridge**: Entity handles are lightweight `u64` IDs (from `hecs::Entity`). Components are represented as Lua tables when read and converted back to Rust structs when written. Frequently-accessed components (Transform4D, Velocity) have optimized get/set paths that avoid full table conversion.

### 2.6 Error Handling

- **All Lua calls are wrapped in pcall**: Script errors never crash the engine. Errors are caught, formatted with file/line information, and logged.
- **Error display**: In development mode, script errors are shown on-screen via the egui overlay (red text with stack trace). In release mode, errors are logged to a file.
- **Error recovery**: When a script error occurs in `on_update`, the frame continues with the remaining systems. The error is reported once, then suppressed until the script is reloaded (to avoid log spam).
- **Hot-reload recovery**: If a reloaded script has errors, the old version continues running. The error is displayed and the engine keeps watching for a fixed version.

---

## 3. Sub-Phase A: Core Runtime (~2 sessions)

### 3.1 Scope

Build the foundational Lua VM infrastructure: initialization, script loading, game loop integration, and basic error handling. After this sub-phase, a `main.lua` script can be loaded and its lifecycle callbacks are invoked each frame.

### 3.2 Lua VM Initialization and Configuration

```rust
// crates/rust4d_scripting/src/vm.rs

use mlua::prelude::*;

/// Configuration for the scripting engine
pub struct ScriptConfig {
    /// Root directory for game scripts
    pub scripts_dir: String,
    /// Whether to enable hot-reload file watching
    pub hot_reload: bool,
    /// Memory limit for the Lua VM in bytes (0 = unlimited)
    pub memory_limit: usize,
    /// Instruction count limit per call (0 = unlimited, for sandboxing)
    pub instruction_limit: u32,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            scripts_dir: "scripts".to_string(),
            hot_reload: cfg!(debug_assertions),
            memory_limit: 64 * 1024 * 1024, // 64MB
            instruction_limit: 0,
        }
    }
}

/// Initialize a Lua VM with engine configuration
pub fn create_lua_vm(config: &ScriptConfig) -> LuaResult<Lua> {
    let lua = Lua::new();

    // Configure package.path for require() resolution
    lua.load(&format!(
        r#"package.path = "{}/?.lua;{}/lib/?.lua;{}/lib/?/init.lua""#,
        config.scripts_dir, config.scripts_dir, config.scripts_dir
    )).exec()?;

    // Remove dangerous standard library modules for sandboxing
    let globals = lua.globals();
    globals.set("os", LuaNil)?;       // No OS access
    globals.set("io", LuaNil)?;       // No file I/O
    globals.set("loadfile", LuaNil)?;  // No arbitrary file loading
    globals.set("dofile", LuaNil)?;    // No arbitrary file execution
    // Keep: math, string, table, coroutine, debug (limited)

    // Add engine print function that routes to log::info
    let print_fn = lua.create_function(|_, args: LuaMultiValue| {
        let msg: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
        log::info!("[lua] {}", msg.join("\t"));
        Ok(())
    })?;
    globals.set("print", print_fn)?;

    Ok(lua)
}
```

### 3.3 Script Loading

```rust
// crates/rust4d_scripting/src/loader.rs

/// Load main.lua and execute it, establishing the global game module
pub fn load_game_scripts(lua: &Lua, scripts_dir: &str) -> Result<(), ScriptError> {
    let main_path = format!("{}/main.lua", scripts_dir);

    if !std::path::Path::new(&main_path).exists() {
        return Err(ScriptError::FileNotFound(main_path));
    }

    let source = std::fs::read_to_string(&main_path)
        .map_err(|e| ScriptError::IoError(main_path.clone(), e))?;

    lua.load(&source)
        .set_name(&main_path)
        .exec()
        .map_err(|e| ScriptError::LuaError(e))?;

    Ok(())
}
```

### 3.4 main.lua as Entry Point

The game's `main.lua` is the single entry point. It uses `require()` to load other modules:

```lua
-- scripts/main.lua
local player = require("player")
local weapons = require("weapons")
local enemies = require("enemies")
local hud = require("hud")

function on_init()
    player.init()
    weapons.init()
    enemies.init()
    hud.init()
end

function on_update(dt)
    player.update(dt)
    weapons.update(dt)
    enemies.update(dt)
    hud.update(dt)
end

function on_fixed_update(dt)
    player.fixed_update(dt)
    enemies.fixed_update(dt)
end

function on_shutdown()
    -- Cleanup
end
```

### 3.5 Game Loop Integration

```rust
// crates/rust4d_scripting/src/lifecycle.rs

use mlua::prelude::*;

/// Call a global Lua function if it exists, ignoring missing function errors
pub fn call_lifecycle(lua: &Lua, name: &str, args: impl IntoLuaMulti) -> Result<(), ScriptError> {
    let globals = lua.globals();
    match globals.get::<LuaFunction>(name) {
        Ok(func) => {
            func.call::<()>(args)
                .map_err(|e| ScriptError::RuntimeError {
                    callback: name.to_string(),
                    error: e,
                })?;
            Ok(())
        }
        Err(LuaError::FromLuaConversionError { .. }) => {
            // Function doesn't exist -- that's fine, it's optional
            Ok(())
        }
        Err(e) => Err(ScriptError::LuaError(e)),
    }
}
```

### 3.6 ScriptEngine Public API

```rust
// crates/rust4d_scripting/src/lib.rs

/// The main scripting engine handle
pub struct ScriptEngine {
    lua: Lua,
    config: ScriptConfig,
    error_state: Option<ScriptError>,
    // References to engine systems set via set_context()
}

impl ScriptEngine {
    /// Create a new scripting engine with the given configuration
    pub fn new(config: ScriptConfig) -> Result<Self, ScriptError>;

    /// Register all engine API bindings (math, physics, input, etc.)
    pub fn register_bindings(&mut self, ctx: &mut EngineContext) -> Result<(), ScriptError>;

    /// Load the game's main.lua and all required modules
    pub fn load_game(&mut self) -> Result<(), ScriptError>;

    /// Call on_init() in the loaded scripts
    pub fn call_init(&self) -> Result<(), ScriptError>;

    /// Call on_update(dt) each frame
    pub fn call_update(&self, dt: f32) -> Result<(), ScriptError>;

    /// Call on_fixed_update(dt) at fixed timestep rate
    pub fn call_fixed_update(&self, dt: f32) -> Result<(), ScriptError>;

    /// Dispatch engine events to Lua event handlers
    pub fn dispatch_events(&self, events: &[EngineEvent]) -> Result<(), ScriptError>;

    /// Check for file changes and hot-reload modified scripts
    pub fn check_hot_reload(&mut self) -> Result<bool, ScriptError>;

    /// Call on_shutdown() before engine exit
    pub fn call_shutdown(&self) -> Result<(), ScriptError>;

    /// Get the last error (for display in egui overlay)
    pub fn last_error(&self) -> Option<&ScriptError>;
}
```

### 3.7 Error Types

```rust
// crates/rust4d_scripting/src/error.rs

#[derive(Debug)]
pub enum ScriptError {
    /// Script file not found
    FileNotFound(String),
    /// IO error reading script file
    IoError(String, std::io::Error),
    /// Lua execution error (syntax error, runtime error)
    LuaError(mlua::Error),
    /// Error in a lifecycle callback
    RuntimeError {
        callback: String,
        error: mlua::Error,
    },
    /// Hot-reload error (old script continues running)
    ReloadError {
        path: String,
        error: mlua::Error,
    },
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound(path) => write!(f, "Script not found: {}", path),
            Self::IoError(path, e) => write!(f, "Failed to read {}: {}", path, e),
            Self::LuaError(e) => write!(f, "Lua error: {}", e),
            Self::RuntimeError { callback, error } => {
                write!(f, "Error in {}(): {}", callback, error)
            }
            Self::ReloadError { path, error } => {
                write!(f, "Reload failed for {}: {}", path, error)
            }
        }
    }
}
```

### 3.8 File List

| File | Action | Description |
|------|--------|-------------|
| `crates/rust4d_scripting/Cargo.toml` | NEW | Crate manifest with mlua, notify dependencies |
| `crates/rust4d_scripting/src/lib.rs` | NEW | ScriptEngine public API, re-exports |
| `crates/rust4d_scripting/src/vm.rs` | NEW | Lua VM creation, configuration, sandboxing |
| `crates/rust4d_scripting/src/loader.rs` | NEW | Script loading, require() resolution |
| `crates/rust4d_scripting/src/lifecycle.rs` | NEW | Game loop callback dispatch |
| `crates/rust4d_scripting/src/error.rs` | NEW | ScriptError types and formatting |
| `Cargo.toml` (workspace) | EDIT | Add `rust4d_scripting` to workspace members |

### 3.9 Tests Required

- Lua VM creates successfully with default config
- `print()` routes to `log::info`
- Sandboxed VM has no `os`, `io`, `loadfile`, `dofile` globals
- `main.lua` loads and executes
- `require()` resolves modules from scripts directory
- `on_init()` callback fires when present
- `on_update(dt)` receives correct delta time
- Missing callbacks are silently ignored (no error)
- Syntax errors in scripts produce `ScriptError::LuaError` with file/line info
- Runtime errors in `on_update` produce `ScriptError::RuntimeError` and don't crash
- Memory limit triggers error (not crash) when exceeded

### 3.10 Session Estimate

**2 sessions.**
- Session 1: VM initialization, script loading, require() setup, basic error handling, tests
- Session 2: Lifecycle callback dispatch, ScriptEngine API, game loop integration, error display, tests

---

## 4. Sub-Phase B: ECS Bindings (~2-3 sessions)

### 4.1 Scope

This is the critical binding that makes scripts interact with the game world. Lua code must be able to:
- Spawn entities with component bundles
- Query entities by component type
- Get and set component values
- Despawn entities
- Register custom component types as Lua tables

### 4.2 Design: Component Model

The ECS uses `hecs`. Since Lua is dynamically typed, we cannot register arbitrary Rust types as hecs components from Lua. Instead, we use a **dynamic component** approach:

- **Engine-defined components** (Transform4D, RigidBody4D, Health, etc.) are registered as known types with optimized get/set paths
- **Script-defined components** use a `LuaComponent` wrapper that stores a Lua table reference as the component data

```rust
// crates/rust4d_scripting/src/bindings/ecs.rs

/// A script-defined component stored as a serialized Lua table
#[derive(Clone)]
pub struct LuaComponent {
    /// Component type name (e.g., "Health", "Weapon", "EnemyAI")
    pub type_name: String,
    /// Serialized component data (Lua table -> serde_json::Value)
    pub data: serde_json::Value,
}
```

### 4.3 Entity Handle

Entities are exposed to Lua as lightweight userdata wrapping `hecs::Entity`:

```rust
/// Lua-side entity handle
pub struct LuaEntity(pub hecs::Entity);

impl LuaUserData for LuaEntity {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("id", |_, this, ()| {
            Ok(this.0.id() as u64)
        });

        methods.add_method("is_alive", |lua, this, ()| {
            let world = get_ecs_world(lua)?;
            Ok(world.contains(this.0))
        });

        methods.add_method("get", |lua, this, component_name: String| {
            get_component(lua, this.0, &component_name)
        });

        methods.add_method("set", |lua, this, (component_name, value): (String, LuaTable)| {
            set_component(lua, this.0, &component_name, value)
        });

        methods.add_method("despawn", |lua, this, ()| {
            let world = get_ecs_world_mut(lua)?;
            world.despawn(this.0).map_err(LuaError::external)?;
            Ok(())
        });
    }
}
```

### 4.4 World Bindings

```rust
/// Register the "world" global table in Lua
pub fn register_world_bindings(lua: &Lua) -> LuaResult<()> {
    let world_table = lua.create_table()?;

    // world.spawn(components_table) -> entity
    world_table.set("spawn", lua.create_function(|lua, components: LuaTable| {
        spawn_entity(lua, components)
    })?)?;

    // world.despawn(entity)
    world_table.set("despawn", lua.create_function(|lua, entity: LuaEntity| {
        despawn_entity(lua, entity.0)
    })?)?;

    // world.query(component_names...) -> iterator
    world_table.set("query", lua.create_function(|lua, names: LuaMultiValue| {
        query_entities(lua, names)
    })?)?;

    // world.find_by_name(name) -> entity or nil
    world_table.set("find_by_name", lua.create_function(|lua, name: String| {
        find_entity_by_name(lua, &name)
    })?)?;

    // world.find_by_tag(tag) -> {entities}
    world_table.set("find_by_tag", lua.create_function(|lua, tag: String| {
        find_entities_by_tag(lua, &tag)
    })?)?;

    lua.globals().set("world", world_table)?;
    Ok(())
}
```

### 4.5 Lua API Design with Examples

#### Spawning Entities

```lua
-- Spawn an entity with a bundle of components
local enemy = world.spawn({
    transform = Transform4D.new(10, 0, 5, 0),
    health = { current = 80, max = 80 },
    enemy_ai = { state = "idle", sight_range = 25.0 },
    sprite = { sheet = "enemy_gunner", frame = 0 },
    physics_body = {
        body_type = "dynamic",
        collider = { type = "sphere", radius = 0.5 },
        mass = 1.0,
    },
})
```

#### Querying Entities

```lua
-- Query all entities with health and transform
for entity, health, transform in world.query("health", "transform") do
    if health.current <= 0 then
        entity:despawn()
    end
end

-- Query enemies specifically
for entity, ai, transform in world.query("enemy_ai", "transform") do
    update_enemy_ai(entity, ai, transform, dt)
end
```

#### Getting and Setting Components

```lua
-- Get a component
local health = entity:get("health")
print(health.current, health.max)

-- Set/update a component
health.current = health.current - 25
entity:set("health", health)

-- Shorthand for Transform4D (optimized path)
local pos = entity:get_position()
pos.y = pos.y + 1.0
entity:set_position(pos)
```

#### Component Registration

```lua
-- Register a script-defined component type
-- (This tells the ECS what fields to expect)
world.register_component("health", {
    current = 100,
    max = 100,
})

world.register_component("weapon", {
    weapon_type = "shotgun",
    ammo_current = 8,
    ammo_max = 8,
    fire_rate = 1.5,
    damage = 15,
    fire_cooldown = 0,
})

world.register_component("enemy_ai", {
    state = "idle",
    target = nil,
    sight_range = 20.0,
    attack_range = 2.0,
    move_speed = 10.0,
    pain_chance = 0.5,
})
```

### 4.6 Rust Binding Code (Key Implementation Details)

#### Optimized Transform Path

Transform4D is the most frequently accessed component. It gets a dedicated fast path:

```rust
// In LuaEntity methods
methods.add_method("get_position", |lua, this, ()| {
    let world = get_ecs_world(lua)?;
    let transform = world.get::<&Transform4D>(this.0)
        .map_err(LuaError::external)?;
    Ok(LuaVec4(transform.position))
});

methods.add_method("set_position", |lua, this, pos: LuaVec4| {
    let world = get_ecs_world_mut(lua)?;
    let mut transform = world.get::<&mut Transform4D>(this.0)
        .map_err(LuaError::external)?;
    transform.position = pos.0;
    Ok(())
});
```

#### Query Implementation

```rust
/// Execute an ECS query and return results as a Lua iterator
fn query_entities(lua: &Lua, component_names: LuaMultiValue) -> LuaResult<LuaFunction> {
    let names: Vec<String> = component_names
        .iter()
        .map(|v| lua.from_value(v.clone()))
        .collect::<LuaResult<Vec<String>>>()?;

    // Return a Lua iterator function
    // Each call returns (entity, component1, component2, ...) or nil
    // Implementation uses hecs dynamic queries internally
    lua.create_function(move |lua, ()| {
        // ... iterate through matching entities
        // Return entity handle + component tables
        Ok(LuaMultiValue::new())
    })
}
```

### 4.7 File List

| File | Action | Description |
|------|--------|-------------|
| `crates/rust4d_scripting/src/bindings/mod.rs` | NEW | Binding registration orchestrator |
| `crates/rust4d_scripting/src/bindings/ecs.rs` | NEW | World, Entity, Component bindings |

### 4.8 Tests Required

- Spawn entity from Lua with multiple components
- Query returns correct entities matching component filter
- Get component returns correct values
- Set component updates the ECS world
- Despawn removes entity from world
- `find_by_name` returns correct entity
- `find_by_tag` returns all matching entities
- Script-defined components survive round-trip (set -> get)
- Transform4D optimized path matches general get/set
- Querying with no results returns empty iterator
- Despawning during iteration does not crash
- Entity handles become invalid after despawn (`is_alive()` returns false)
- Memory: spawning thousands of entities from Lua stays within memory limit

### 4.9 Session Estimate

**2-3 sessions.**
- Session 1: LuaEntity userdata, world.spawn, world.despawn, get/set component basics
- Session 2: Query iterator, find_by_name/tag, component registration, optimized Transform path
- Session 3 (if needed): Dynamic component serialization, edge cases, performance testing

---

## 5. Sub-Phase C: Engine API Bindings (~2-3 sessions)

Bindings for each engine subsystem. For each, we specify the Rust types/functions to wrap and the resulting Lua API.

### 5.1 Math Bindings

**Rust types**: `Vec4`, `Rotor4`, `Transform4D` (from `rust4d_math`)

**Lua API**:

```lua
-- Vec4: constructor, operators, methods
local v = Vec4.new(1, 2, 3, 0)
local v2 = Vec4.new(4, 5, 6, 0)

-- Operator overloading via metatables
local sum = v + v2            -- __add
local diff = v - v2           -- __sub
local scaled = v * 3.0        -- __mul (scalar)
local negated = -v             -- __unm
local dot = v:dot(v2)
local len = v:length()
local norm = v:normalized()
local dist = v:distance(v2)
local lerped = Vec4.lerp(v, v2, 0.5)

-- Component access
print(v.x, v.y, v.z, v.w)
v.x = 10.0

-- Common constants
local zero = Vec4.ZERO
local up = Vec4.new(0, 1, 0, 0)

-- Rotor4: rotation constructors
local rot = Rotor4.from_plane(RotationPlane.XY, math.rad(45))
local rotated = rot:rotate(v)
local combined = rot * rot2    -- __mul (rotor composition)
local slerped = Rotor4.slerp(rot, rot2, 0.5)

-- Transform4D: position + rotation
local t = Transform4D.new(Vec4.new(0, 2, 0, 0), Rotor4.identity())
local world_pos = t:transform_point(local_pos)
```

**Rust binding code (Vec4 example)**:

```rust
// crates/rust4d_scripting/src/bindings/math.rs

pub struct LuaVec4(pub Vec4);

impl LuaUserData for LuaVec4 {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.0.x));
        fields.add_field_method_set("x", |_, this, val: f32| { this.0.x = val; Ok(()) });
        fields.add_field_method_get("y", |_, this| Ok(this.0.y));
        fields.add_field_method_set("y", |_, this, val: f32| { this.0.y = val; Ok(()) });
        fields.add_field_method_get("z", |_, this| Ok(this.0.z));
        fields.add_field_method_set("z", |_, this, val: f32| { this.0.z = val; Ok(()) });
        fields.add_field_method_get("w", |_, this| Ok(this.0.w));
        fields.add_field_method_set("w", |_, this, val: f32| { this.0.w = val; Ok(()) });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("dot", |_, this, other: LuaVec4| Ok(this.0.dot(other.0)));
        methods.add_method("length", |_, this, ()| Ok(this.0.length()));
        methods.add_method("length_squared", |_, this, ()| Ok(this.0.length_squared()));
        methods.add_method("normalized", |_, this, ()| Ok(LuaVec4(this.0.normalized())));
        methods.add_method("distance", |_, this, other: LuaVec4| {
            Ok((this.0 - other.0).length())
        });

        // Operator overloads via metamethods
        methods.add_meta_method(LuaMetaMethod::Add, |_, this, other: LuaVec4| {
            Ok(LuaVec4(this.0 + other.0))
        });
        methods.add_meta_method(LuaMetaMethod::Sub, |_, this, other: LuaVec4| {
            Ok(LuaVec4(this.0 - other.0))
        });
        methods.add_meta_method(LuaMetaMethod::Mul, |_, this, scalar: f32| {
            Ok(LuaVec4(this.0 * scalar))
        });
        methods.add_meta_method(LuaMetaMethod::Unm, |_, this, ()| {
            Ok(LuaVec4(-this.0))
        });
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("Vec4({}, {}, {}, {})", this.0.x, this.0.y, this.0.z, this.0.w))
        });
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaVec4| {
            Ok(this.0 == other.0)
        });
    }
}
```

### 5.2 Physics Bindings

**Rust types**: `PhysicsWorld`, `RigidBody4D`, `BodyKey`, `CollisionLayer`, `Ray4D`, `RayHit`, `WorldRayHit` (from `rust4d_physics`)

**Lua API**:

```lua
-- Raycasting
local hits = physics.raycast(origin, direction, max_distance, layer_mask)
for _, hit in ipairs(hits) do
    print(hit.entity, hit.distance, hit.point, hit.normal)
end

local nearest = physics.raycast_nearest(origin, direction, max_distance, layer_mask)
if nearest then
    apply_damage(nearest.entity, 50)
end

-- Spatial queries
local nearby = physics.query_sphere(center, radius, { "ENEMY", "PLAYER" })
for _, result in ipairs(nearby) do
    print(result.entity, result.distance)
end

-- Area effects (explosions)
local area_hits = physics.query_area_effect(center, radius, { "ENEMY" }, true)
for _, hit in ipairs(area_hits) do
    local damage = base_damage * hit.falloff
    apply_damage(hit.entity, damage)
    physics.apply_impulse(hit.entity, hit.direction * knockback_force)
end

-- Line of sight
local can_see = physics.line_of_sight(from_pos, to_pos, { "STATIC" })

-- Body manipulation
physics.set_velocity(entity, Vec4.new(0, 10, 0, 0))
local vel = physics.get_velocity(entity)
physics.apply_impulse(entity, impulse_vec)

-- Collision events (received via event system)
events.on("collision", function(event)
    local a, b = event.body_a, event.body_b
    -- Handle collision
end)

events.on("trigger_enter", function(event)
    local body, trigger = event.body, event.trigger
    -- Handle trigger enter
end)
```

**Rust types to wrap**: `PhysicsWorld::raycast()`, `raycast_nearest()`, `query_sphere()`, `query_area_effect()`, `line_of_sight()`, `apply_impulse()`, `set_velocity()`, `get_velocity()`, body position getters.

### 5.3 Input Bindings

**Rust types**: `InputAction`, `InputMap` (from `rust4d_input`)

**Lua API**:

```lua
-- Poll input actions
if input.is_action_pressed("fire") then
    weapons.fire()
end

if input.is_action_just_pressed("jump") then
    player.jump()
end

local move_x = input.get_axis("move_right", "move_left")
local move_z = input.get_axis("move_forward", "move_backward")
local move_w = input.get_axis("move_ana", "move_kata")

-- Mouse input
local dx, dy = input.mouse_delta()

-- Input configuration
input.bind("fire", "MouseLeft")
input.bind("jump", "Space")
input.bind("move_forward", "W")
```

**Rust types to wrap**: `InputMap::is_action_pressed()`, `is_action_just_pressed()`, `get_axis()`, mouse delta, `rebind()`.

### 5.4 Audio Bindings

**Rust types**: `AudioEngine4D`, `SoundHandle`, `AudioBus`, `SpatialConfig` (from `rust4d_audio`)

**Lua API**:

```lua
-- Load sounds
local shotgun_fire = audio.load_sound("sounds/shotgun_fire.ogg")
local music_level1 = audio.load_sound("sounds/music_level1.ogg")

-- Play non-spatial (UI, music)
audio.play(music_level1, "music")

-- Play spatial (positioned in 4D)
audio.play_spatial(shotgun_fire, player_position, "sfx", {
    min_distance = 1.0,
    max_distance = 50.0,
})

-- One-shot (fire and forget)
audio.play_oneshot(shotgun_fire, "sfx")
audio.play_oneshot_spatial(shotgun_fire, explosion_pos, "sfx")

-- Volume control
audio.set_volume("master", 0.8)
audio.set_volume("sfx", 1.0)
audio.set_volume("music", 0.5)

-- Stop sounds
audio.stop_all()
audio.stop_bus("music")

-- Update listener (typically called each frame from player controller)
audio.update_listener(camera_position, camera_forward, camera_up)
```

**Rust types to wrap**: `AudioEngine4D::load_sound()`, `play()`, `play_spatial()`, `play_oneshot()`, `play_oneshot_spatial()`, `set_bus_volume()`, `set_master_volume()`, `update_listener()`, `stop_all()`, `stop_bus()`.

### 5.5 Rendering Bindings

**Rust types**: `ParticleSystem`, `ParticleEmitterConfig`, `SpriteBatch`, `OverlayRenderer` (from `rust4d_render`)

**Lua API**:

```lua
-- Particles
local muzzle_flash = particles.spawn_burst(position, {
    count = 15,
    lifetime = 0.1,
    initial_color = { 1.0, 0.9, 0.5, 1.0 },
    end_color = { 1.0, 0.3, 0.0, 0.0 },
    initial_size = 0.3,
    end_size = 0.05,
    speed = 2.0,
    spread = 0.8,
    gravity = 0,
    drag = 5.0,
    blend_mode = "additive",
})

local emitter = particles.spawn_emitter(position, config)
particles.update_position(emitter, new_position)
particles.stop(emitter)
particles.kill(emitter)

-- Sprites (enemy rendering)
sprites.add_4d(position_4d, slice_w, {
    sheet = "enemy_rusher",
    frame = current_frame,
    size = { 1.5, 2.0 },
    w_fade_range = 3.0,
})

-- HUD (via egui context exposed as Lua drawing commands)
hud.text("HP: " .. health, 20, screen_height - 60, { color = "red" })
hud.text("AMMO: " .. ammo, screen_width - 120, screen_height - 60)
hud.text("+", screen_width / 2 - 4, screen_height / 2 - 8)  -- Crosshair
hud.text("W: " .. string.format("%.1f", player_w), screen_width - 100, 20)
hud.rect(0, 0, screen_width, screen_height, { color = { 1, 0, 0, damage_flash_alpha } })
```

**Note**: Full egui exposure is complex. The HUD bindings provide a simplified drawing API (text, rect, image) that internally uses egui. Advanced egui usage requires Rust code.

### 5.6 Scene Bindings

**Rust types**: `Scene`, `ActiveScene`, `SceneManager` (from `rust4d_core`)

**Lua API**:

```lua
-- Load a scene
scene.load("scenes/level1.ron")

-- Instantiate an entity template
local door = scene.instantiate("door_template", {
    position = Vec4.new(5, 0, 0, 0),
})

-- Scene transitions
scene.transition_to("scenes/level2.ron", { effect = "fade", duration = 1.0 })
```

### 5.7 Assets Bindings

**Rust types**: `AssetCache`, `AssetHandle` (from `rust4d_core`)

**Lua API**:

```lua
-- Load assets (cached -- second call returns same handle)
local texture = assets.load_texture("textures/wall_brick.png")
local sound = assets.load_sound("sounds/explosion.ogg")
local scene = assets.load_scene("scenes/enemy_template.ron")

-- Check if asset is loaded
if assets.is_loaded(texture) then ... end
```

### 5.8 File List

| File | Action | Description |
|------|--------|-------------|
| `crates/rust4d_scripting/src/bindings/math.rs` | NEW | Vec4, Rotor4, Transform4D with metatables |
| `crates/rust4d_scripting/src/bindings/physics.rs` | NEW | Raycast, spatial queries, body manipulation |
| `crates/rust4d_scripting/src/bindings/input.rs` | NEW | Input polling, axis queries, rebinding |
| `crates/rust4d_scripting/src/bindings/audio.rs` | NEW | Sound loading, playback, spatial audio |
| `crates/rust4d_scripting/src/bindings/render.rs` | NEW | Particles, sprites, HUD drawing |
| `crates/rust4d_scripting/src/bindings/scene.rs` | NEW | Scene loading, entity templates |
| `crates/rust4d_scripting/src/bindings/assets.rs` | NEW | Asset cache queries |

### 5.9 Tests Required

- Vec4 arithmetic operators produce correct results from Lua
- Vec4 methods (dot, length, normalized, distance) match Rust results
- Rotor4 rotation applied from Lua matches Rust rotation
- Transform4D point transformation from Lua matches Rust
- Physics raycast from Lua returns correct hits
- Spatial query from Lua returns entities within radius
- Input polling returns current input state
- Audio sound loading and playback calls reach the engine
- Particle spawning from Lua creates particles in the render system
- HUD text drawing from Lua produces visible output
- Scene loading from Lua triggers engine scene load

### 5.10 Session Estimate

**2-3 sessions.**
- Session 1: Math bindings (Vec4, Rotor4, Transform4D with full metatable support) + Physics bindings (raycast, spatial queries)
- Session 2: Input bindings + Audio bindings + Scene/Asset bindings
- Session 3 (if needed): Render bindings (particles, sprites, HUD), integration testing

---

## 6. Sub-Phase D: Hot-Reload (~1 session)

### 6.1 Scope

File watcher for `.lua` files, module reload strategy, error recovery, and a dev console for runtime script evaluation.

### 6.2 File Watcher

```rust
// crates/rust4d_scripting/src/hot_reload.rs

use notify::{Watcher, RecursiveMode, RecommendedWatcher, Event, EventKind};
use std::sync::mpsc;
use std::path::PathBuf;

pub struct ScriptWatcher {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<notify::Result<Event>>,
    scripts_dir: PathBuf,
}

impl ScriptWatcher {
    pub fn new(scripts_dir: &str) -> Result<Self, ScriptError> {
        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(
            move |res| { let _ = tx.send(res); },
            notify::Config::default(),
        ).map_err(|e| ScriptError::WatcherError(e.to_string()))?;

        watcher.watch(
            scripts_dir.as_ref(),
            RecursiveMode::Recursive,
        ).map_err(|e| ScriptError::WatcherError(e.to_string()))?;

        Ok(Self {
            _watcher: watcher,
            rx,
            scripts_dir: PathBuf::from(scripts_dir),
        })
    }

    /// Check for file changes. Returns list of changed .lua file paths.
    pub fn poll_changes(&self) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        while let Ok(Ok(event)) = self.rx.try_recv() {
            match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) => {
                    for path in event.paths {
                        if path.extension().map_or(false, |ext| ext == "lua") {
                            if !changed.contains(&path) {
                                changed.push(path);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        changed
    }
}
```

### 6.3 Module Reload Strategy

When a `.lua` file changes:

1. **Identify the module**: Map file path back to Lua module name (e.g., `scripts/weapons.lua` -> `"weapons"`)
2. **Clear the module from `package.loaded`**: This forces the next `require()` to re-execute the file
3. **Re-execute the module**: Call `require(module_name)` to load the new version
4. **Call `on_reload()`**: If the module defines an `on_reload()` function, call it so the script can re-register event handlers, update references, etc.
5. **Update references**: The global callback functions (`on_update`, etc.) in `main.lua` will naturally call the new module functions if `main.lua` holds module references via `require()`

```rust
/// Reload a changed Lua module
fn reload_module(lua: &Lua, module_name: &str, file_path: &str) -> Result<(), ScriptError> {
    // Read new source
    let source = std::fs::read_to_string(file_path)
        .map_err(|e| ScriptError::IoError(file_path.to_string(), e))?;

    // Clear from package.loaded
    lua.load(&format!(r#"package.loaded["{}"] = nil"#, module_name))
        .exec()
        .map_err(ScriptError::LuaError)?;

    // Re-execute the module
    lua.load(&source)
        .set_name(file_path)
        .exec()
        .map_err(|e| ScriptError::ReloadError {
            path: file_path.to_string(),
            error: e,
        })?;

    // Call on_reload() if it exists
    let _ = call_lifecycle(lua, "on_reload", ());

    log::info!("[scripting] Reloaded: {}", file_path);
    Ok(())
}
```

### 6.4 Error Recovery on Reload Failure

If a reloaded script has errors:

1. The error is logged with file/line information
2. The `package.loaded` entry is restored to the old version (it was cleared but re-execute failed, so it stays `nil` -- the old module functions are still referenced by `main.lua`)
3. The error is displayed on-screen via the egui overlay
4. The file watcher continues watching -- the developer fixes the script and saves again
5. No game state is corrupted

### 6.5 on_reload() Callback

Scripts can define `on_reload()` to handle re-registration:

```lua
-- scripts/enemies.lua
local enemies_module = {}

function enemies_module.init()
    events.on("enemy_damaged", enemies_module.on_enemy_damaged)
end

function enemies_module.on_enemy_damaged(event)
    -- Handle damage
end

function on_reload()
    -- Re-register event handlers after reload
    enemies_module.init()
    print("Enemies module reloaded!")
end

return enemies_module
```

### 6.6 Dev Console

A simple runtime Lua evaluation console, rendered via egui:

```rust
/// Dev console state
pub struct DevConsole {
    input_buffer: String,
    history: Vec<(String, String)>,  // (input, output)
    visible: bool,
}

impl DevConsole {
    /// Toggle visibility (bound to backtick/tilde key)
    pub fn toggle(&mut self) { self.visible = !self.visible; }

    /// Execute a Lua string and capture output
    pub fn execute(&mut self, lua: &Lua, code: &str) {
        match lua.load(code).eval::<mlua::MultiValue>() {
            Ok(values) => {
                let output: Vec<String> = values.iter()
                    .map(|v| format!("{:?}", v))
                    .collect();
                self.history.push((code.to_string(), output.join(", ")));
            }
            Err(e) => {
                self.history.push((code.to_string(), format!("Error: {}", e)));
            }
        }
    }
}
```

This allows developers to inspect and modify game state at runtime:

```
> player_health = 100
> print(world.find_by_name("player"):get("health").current)
85
> physics.set_velocity(world.find_by_name("player"), Vec4.new(0, 50, 0, 0))
```

### 6.7 File List

| File | Action | Description |
|------|--------|-------------|
| `crates/rust4d_scripting/src/hot_reload.rs` | NEW | File watcher, module reload, error recovery |
| `crates/rust4d_scripting/src/dev_console.rs` | NEW | Runtime Lua evaluation console |

### 6.8 Tests Required

- File watcher detects `.lua` file modifications
- Module reload clears `package.loaded` and re-executes
- `on_reload()` callback fires after successful reload
- Reload error preserves old module (no crash, no state corruption)
- Reload error is reported with file/line info
- Dev console evaluates Lua expressions and captures output
- Dev console handles errors gracefully

### 6.9 Session Estimate

**1 session.**
- File watcher setup, module reload logic, error recovery, dev console, tests

---

## 7. Sub-Phase E: Game Framework Bindings (~1-2 sessions)

Bindings for `rust4d_game` types that provide higher-level game patterns.

### 7.1 CharacterController4D

```lua
-- Character controller (wraps rust4d_game::CharacterController4D)
local controller = CharacterController4D.new(player_entity, {
    move_speed = 10.0,
    jump_force = 15.0,
    gravity = -20.0,
    ground_friction = 8.0,
})

function on_fixed_update(dt)
    -- Read input and apply movement
    local move_input = Vec4.new(
        input.get_axis("move_right", "move_left"),
        0,
        input.get_axis("move_forward", "move_backward"),
        input.get_axis("move_ana", "move_kata")
    )

    controller:move(move_input, dt)

    if input.is_action_just_pressed("jump") and controller:is_grounded() then
        controller:jump()
    end
end
```

### 7.2 Event System

```lua
-- Subscribe to events
events.on("damage", function(event)
    local target = event.target
    local amount = event.amount
    local health = target:get("health")
    health.current = health.current - amount
    target:set("health", health)

    if health.current <= 0 then
        events.emit("death", { entity = target })
    end
end)

events.on("death", function(event)
    -- Play death animation, spawn particles, despawn after delay
    particles.spawn_burst(event.entity:get_position(), blood_config)
    audio.play_oneshot_spatial(death_sound, event.entity:get_position(), "sfx")
    -- Schedule despawn after 2 seconds
    timers.after(2.0, function() event.entity:despawn() end)
end)

-- Emit events from game logic
events.emit("weapon_fired", {
    weapon_type = "shotgun",
    position = player_position,
})

-- Collision events are automatically forwarded
events.on("collision", function(event)
    -- event.body_a, event.body_b, event.contact
end)

events.on("trigger_enter", function(event)
    -- event.body, event.trigger_index
end)
```

### 7.3 StateMachine

```lua
-- Create an FSM (wraps rust4d_game::StateMachine)
local fsm = StateMachine.new("idle")

-- Update each frame
fsm:update(dt)

-- Transition
fsm:transition("chase")

-- Query state
local state = fsm:current()
local prev = fsm:previous()
local time = fsm:time_in_state()
local just_entered = fsm:just_entered()

-- Typical usage in enemy AI
function update_enemy(entity, dt)
    local ai = entity:get("enemy_ai")
    ai.fsm:update(dt)

    local state = ai.fsm:current()
    if state == "idle" then
        update_idle(entity, ai, dt)
    elseif state == "chase" then
        update_chase(entity, ai, dt)
    elseif state == "attack" then
        update_attack(entity, ai, dt)
    elseif state == "pain" then
        update_pain(entity, ai, dt)
    elseif state == "dead" then
        update_dead(entity, ai, dt)
    end
end
```

### 7.4 Tween System

```lua
-- Create tweens (wraps rust4d_game::TweenManager)
tweens.position(entity, target_pos, 1.5, "ease_in_out_quad")
tweens.position_w(entity, 5.0, 0.5, "ease_in_out_quad")  -- Tween only W component

-- Tween callbacks
tweens.position(door_entity, open_position, 1.5, "ease_in_out_quad", {
    on_complete = function()
        -- Door finished opening
        timers.after(3.0, function()
            tweens.position(door_entity, closed_position, 1.5, "ease_in_out_quad")
        end)
    end
})

-- Available easing functions:
-- "linear", "ease_in_quad", "ease_out_quad", "ease_in_out_quad",
-- "ease_in_cubic", "ease_out_cubic", "ease_in_out_cubic"
```

### 7.5 ScreenShake and TimedEffect

```lua
-- Screen shake
screen_shake.add_trauma(0.3)  -- Weapon fire
screen_shake.add_trauma(0.5)  -- Explosion nearby
screen_shake.add_trauma(0.1)  -- Taking damage

-- Timed effects
local damage_flash = TimedEffect.new(0.3)  -- 0.3 second duration
damage_flash:trigger()

-- Check in HUD rendering
if damage_flash:is_active() then
    hud.rect(0, 0, screen_width, screen_height, {
        color = { 1, 0, 0, damage_flash:intensity() * 0.5 }
    })
end
```

### 7.6 Timer System

A Lua-side convenience for delayed and repeating callbacks:

```lua
-- One-shot timer
timers.after(2.0, function()
    entity:despawn()
end)

-- Repeating timer
local spawn_timer = timers.every(5.0, function()
    spawn_enemy_wave()
end)

-- Cancel a timer
timers.cancel(spawn_timer)
```

The timer system is implemented in pure Lua (no Rust binding needed) and updated from `on_update(dt)`. It can be provided as a standard library script bundled with the engine.

### 7.7 File List

| File | Action | Description |
|------|--------|-------------|
| `crates/rust4d_scripting/src/bindings/game.rs` | NEW | CharacterController, events, FSM, tweens, screen shake |
| `crates/rust4d_scripting/lua_lib/timers.lua` | NEW | Pure Lua timer library (bundled with engine) |

### 7.8 Tests Required

- CharacterController4D move/jump from Lua matches Rust behavior
- Event subscription receives emitted events
- Collision events from physics reach Lua handlers
- StateMachine transitions correctly from Lua
- Tween creation from Lua starts position animation
- ScreenShake trauma accumulation from Lua works
- TimedEffect intensity decay works from Lua
- Timer callbacks fire at correct times

### 7.9 Session Estimate

**1-2 sessions.**
- Session 1: Event system bindings, CharacterController4D, FSM, ScreenShake/TimedEffect
- Session 2 (if needed): Tween system bindings, timer library, integration testing

---

## 8. Session Estimates

### Per Sub-Phase

| Sub-Phase | Scope | Sessions | Dependencies |
|-----------|-------|----------|-------------|
| A: Core Runtime | Lua VM, script loading, lifecycle, errors | 2 | Engine split complete |
| B: ECS Bindings | Entity spawn/query/despawn, components | 2-3 | Sub-Phase A |
| C: Engine API Bindings | Math, physics, input, audio, render, scene | 2-3 | Sub-Phase A, P1-P4 APIs exist |
| D: Hot-Reload | File watcher, module reload, dev console | 1 | Sub-Phase A |
| E: Game Framework Bindings | Controller, events, FSM, tweens, shake | 1-2 | Sub-Phase B, `rust4d_game` exists |
| **Total** | | **8-11** | |

### Dependencies on Other Phases

| Dependency | Required By | Blocking? |
|------------|-------------|-----------|
| Engine/Game Split complete (ECS, `rust4d_game`) | Sub-Phase A (crate structure) | **Yes** |
| P1 Combat Core (raycasting, collision events) | Sub-Phase C (physics bindings) | **Yes** for raycast/trigger bindings |
| P2 Weapons & Feedback (audio, particles, egui) | Sub-Phase C (audio/render bindings) | **Yes** for audio/particle bindings |
| P3 Enemies & AI (sprites, spatial queries, FSM) | Sub-Phase C (sprite/FSM bindings) | **Yes** for sprite/FSM bindings |
| P4 Level Design (triggers, tweens, shapes) | Sub-Phase C (trigger/tween bindings) | **Yes** for trigger/tween bindings |

**Important**: Sub-Phases A, B, and D can begin as soon as the engine split is complete, without waiting for P1-P4. Sub-Phase C bindings should be implemented incrementally as the APIs they wrap become available. Sub-Phase E requires `rust4d_game` types.

### Parallelization Opportunities

```
Wave 1 (Sequential -- Foundation)
  Sub-Phase A: Core Runtime (2 sessions)
    Must complete before any other sub-phase

Wave 2 (Parallel -- after A completes)
  +-- Sub-Phase B: ECS Bindings (2-3 sessions)
  +-- Sub-Phase D: Hot-Reload (1 session)
  +-- Sub-Phase C: Math bindings only (0.5 session)

Wave 3 (Incremental -- as P1-P4 APIs arrive)
  Sub-Phase C: Remaining engine API bindings (1.5-2.5 sessions)
    Physics bindings after P1
    Audio/particle bindings after P2
    Sprite/FSM bindings after P3
    Trigger/tween bindings after P4

Wave 4 (After Wave 2 + rust4d_game types)
  Sub-Phase E: Game Framework Bindings (1-2 sessions)
```

**Critical path**: 2 (A) + 3 (B) + 2.5 (C, incremental) + 2 (E) = ~9.5 sessions on the critical path. With parallelism in Wave 2, this can be reduced to ~7.5 sessions wall-clock.

---

## 9. Lua API Design Philosophy

### Lua-Idiomatic, Not 1:1 Rust Wrappers

The Lua API should feel natural to a Lua programmer, not like a thin wrapper over Rust functions. Specific guidelines:

1. **Metatables for operator overloading**: `Vec4 + Vec4`, `Vec4 * 3.0`, `-Vec4`, `Rotor4 * Rotor4` all work via `__add`, `__mul`, `__unm` metamethods. This makes math code readable.

2. **Method chaining where natural**: When methods return the modified object, support chaining. Example: `entity:set_position(pos):set_rotation(rot)` (though this is less idiomatic in Lua than in Rust/JS).

3. **Table-based configuration**: Pass tables instead of many positional arguments. This is the Lua convention for functions with many parameters:

   ```lua
   -- Good: table-based config
   particles.spawn_burst(position, {
       count = 15,
       lifetime = 0.1,
       color = { 1, 0.9, 0.5, 1 },
       blend_mode = "additive",
   })

   -- Bad: positional arguments
   particles.spawn_burst(position, 15, 0.1, 1, 0.9, 0.5, 1, "additive")
   ```

4. **Consistent naming**: All Lua APIs use `snake_case` to match Lua convention. Lua community convention is lowercase with underscores. Modules are lowercase (`world`, `physics`, `audio`, `input`). Methods are `snake_case` (`get_position`, `spawn_burst`, `is_action_pressed`).

5. **Nil-safe defaults**: Missing table fields use sensible defaults. If `config.gravity` is nil, use `0.0`. If `config.blend_mode` is nil, use `"alpha"`.

6. **String constants over enums**: Lua does not have enums. Use strings for enum-like values: `"additive"`, `"alpha"`, `"ease_in_quad"`, `"idle"`, `"chase"`. The Rust side validates and converts.

7. **Global modules for engine singletons**: Systems that have a single instance (`physics`, `audio`, `input`, `particles`, `world`) are exposed as global tables, not constructed objects. This matches how Lua game frameworks (Love2D, Defold) work.

8. **Iterators for queries**: `world.query()` returns a Lua iterator compatible with `for ... in` syntax, the idiomatic way to loop in Lua.

---

## 10. Example Scripts

### 10.1 Basic Entity Spawner

```lua
-- scripts/spawner.lua
-- Spawns a grid of tesseracts in 4D space

local spawner = {}

function spawner.init()
    -- Spawn a 3x3x3 grid of cubes at W=0
    for x = -2, 2, 2 do
        for y = 0, 4, 2 do
            for z = -2, 2, 2 do
                world.spawn({
                    transform = Transform4D.new(Vec4.new(x, y, z, 0)),
                    shape = { type = "hyperprism", x = 1, y = 1, z = 1, w = 1 },
                    material = {
                        base_color = {
                            (x + 2) / 4,
                            (y) / 4,
                            (z + 2) / 4,
                            1.0
                        }
                    },
                    physics_body = {
                        body_type = "static",
                        collider = { type = "aabb", half_extents = { 0.5, 0.5, 0.5, 0.5 } },
                    },
                })
            end
        end
    end

    -- Spawn the player
    world.spawn({
        name = "player",
        tags = { "player" },
        transform = Transform4D.new(Vec4.new(0, 2, 10, 0)),
        physics_body = {
            body_type = "dynamic",
            collider = { type = "sphere", radius = 0.5 },
            mass = 1.0,
            layer = "PLAYER",
        },
    })

    print("Spawner: created grid of cubes and player")
end

return spawner
```

### 10.2 Weapon Script (Hitscan Shotgun)

```lua
-- scripts/weapons/shotgun.lua
-- Hitscan shotgun with spread pattern

local shotgun = {}

local PELLET_COUNT = 8
local SPREAD_ANGLE = 0.1     -- radians
local MAX_RANGE = 30.0
local BASE_DAMAGE = 15
local FALLOFF_START = 10.0
local FIRE_RATE = 1.5         -- shots per second
local MAX_AMMO = 8

local ammo = MAX_AMMO
local fire_cooldown = 0

function shotgun.init()
    shotgun.fire_sound = audio.load_sound("sounds/shotgun_fire.ogg")
    shotgun.impact_sound = audio.load_sound("sounds/impact_metal.ogg")
end

function shotgun.update(dt)
    fire_cooldown = math.max(0, fire_cooldown - dt)
end

function shotgun.try_fire(player_pos, player_forward)
    if fire_cooldown > 0 or ammo <= 0 then
        return false
    end

    fire_cooldown = 1.0 / FIRE_RATE
    ammo = ammo - 1

    -- Audio and screen shake
    audio.play_oneshot_spatial(shotgun.fire_sound, player_pos, "sfx")
    screen_shake.add_trauma(0.3)

    -- Muzzle flash particles
    particles.spawn_burst(player_pos + player_forward * 0.5, {
        count = 15,
        lifetime = 0.1,
        initial_color = { 1, 0.9, 0.5, 1 },
        end_color = { 1, 0.3, 0, 0 },
        speed = 2.0,
        spread = 0.8,
        blend_mode = "additive",
    })

    -- Cast pellets with spread
    local total_damage = 0
    for i = 1, PELLET_COUNT do
        -- Random spread offset
        local spread_x = (math.random() - 0.5) * SPREAD_ANGLE * 2
        local spread_y = (math.random() - 0.5) * SPREAD_ANGLE * 2
        local spread_w = (math.random() - 0.5) * SPREAD_ANGLE * 2

        -- Apply spread to forward direction (simplified -- real 4D spread is more complex)
        local dir = (player_forward + Vec4.new(spread_x, spread_y, 0, spread_w)):normalized()

        local hit = physics.raycast_nearest(player_pos, dir, MAX_RANGE, { "ENEMY", "STATIC" })
        if hit then
            -- Distance falloff
            local falloff = 1.0
            if hit.distance > FALLOFF_START then
                falloff = 1.0 - (hit.distance - FALLOFF_START) / (MAX_RANGE - FALLOFF_START)
            end

            local damage = BASE_DAMAGE * falloff

            -- Impact particles
            particles.spawn_burst(hit.point, {
                count = 5,
                lifetime = 0.2,
                initial_color = { 1, 1, 0.5, 1 },
                speed = 3.0,
                spread = 1.0,
                blend_mode = "additive",
            })

            -- Apply damage if entity has health
            if hit.entity then
                events.emit("damage", {
                    target = hit.entity,
                    amount = damage,
                    source = "shotgun",
                    hit_point = hit.point,
                    hit_normal = hit.normal,
                })
                total_damage = total_damage + damage
            end

            audio.play_oneshot_spatial(shotgun.impact_sound, hit.point, "sfx")
        end
    end

    return true
end

function shotgun.get_ammo() return ammo end
function shotgun.get_max_ammo() return MAX_AMMO end
function shotgun.reload() ammo = MAX_AMMO end

return shotgun
```

### 10.3 Enemy AI Script (State Machine)

```lua
-- scripts/enemies/melee_rusher.lua
-- Fast melee enemy that charges the player

local rusher = {}

local MOVE_SPEED = 15.0
local ATTACK_DAMAGE = 20
local ATTACK_RANGE = 1.5
local SIGHT_RANGE = 25.0
local PAIN_CHANCE = 0.5
local ATTACK_COOLDOWN = 0.8

function rusher.create(entity, spawn_pos)
    -- Set up components
    entity:set("enemy_ai", {
        fsm = StateMachine.new("idle"),
        target = nil,
        attack_timer = 0,
        pain_timer = 0,
    })
    entity:set("health", { current = 50, max = 50 })
    entity:set("sprite_anim", {
        sheet = "enemy_rusher",
        anims = {
            idle = { frames = { 0, 1, 2, 3 }, fps = 4, looping = true },
            chase = { frames = { 4, 5, 6, 7, 8, 9 }, fps = 10, looping = true },
            attack = { frames = { 10, 11, 12 }, fps = 8, looping = false },
            pain = { frames = { 13 }, fps = 1, looping = false },
            death = { frames = { 14, 15, 16, 17, 18 }, fps = 8, looping = false },
        },
        current_anim = "idle",
    })

    -- Register damage handler for this enemy
    events.on("damage", function(event)
        if event.target == entity then
            rusher.on_damaged(entity, event.amount)
        end
    end)
end

function rusher.on_damaged(entity, amount)
    local health = entity:get("health")
    health.current = health.current - amount
    entity:set("health", health)

    local ai = entity:get("enemy_ai")

    if health.current <= 0 then
        ai.fsm:transition("dead")
        rusher.set_anim(entity, "death")
    elseif math.random() < PAIN_CHANCE then
        ai.fsm:transition("pain")
        ai.pain_timer = 0.3
        rusher.set_anim(entity, "pain")
    end

    entity:set("enemy_ai", ai)

    -- Blood particles
    particles.spawn_burst(entity:get_position(), {
        count = 20,
        lifetime = 0.5,
        initial_color = { 0.8, 0, 0, 1 },
        end_color = { 0.5, 0, 0, 0 },
        speed = 3.0,
        spread = 1.5,
        gravity = 15.0,
        blend_mode = "alpha",
    })
end

function rusher.update(entity, dt)
    local ai = entity:get("enemy_ai")
    ai.fsm:update(dt)

    local state = ai.fsm:current()
    if state == "idle" then
        rusher.update_idle(entity, ai, dt)
    elseif state == "chase" then
        rusher.update_chase(entity, ai, dt)
    elseif state == "attack" then
        rusher.update_attack(entity, ai, dt)
    elseif state == "pain" then
        rusher.update_pain(entity, ai, dt)
    elseif state == "dead" then
        rusher.update_dead(entity, ai, dt)
    end

    entity:set("enemy_ai", ai)
end

function rusher.update_idle(entity, ai, dt)
    -- Check for player within sight range
    local my_pos = entity:get_position()
    local player = world.find_by_name("player")
    if not player then return end

    local player_pos = player:get_position()
    local dist = my_pos:distance(player_pos)

    if dist < SIGHT_RANGE then
        -- Check line of sight
        if physics.line_of_sight(my_pos, player_pos, { "STATIC" }) then
            ai.target = player
            ai.fsm:transition("chase")
            rusher.set_anim(entity, "chase")
        end
    end
end

function rusher.update_chase(entity, ai, dt)
    if not ai.target or not ai.target:is_alive() then
        ai.fsm:transition("idle")
        rusher.set_anim(entity, "idle")
        return
    end

    local my_pos = entity:get_position()
    local target_pos = ai.target:get_position()
    local direction = (target_pos - my_pos):normalized()
    local dist = my_pos:distance(target_pos)

    -- Move toward player
    physics.set_velocity(entity, direction * MOVE_SPEED)

    -- In attack range?
    if dist < ATTACK_RANGE then
        ai.fsm:transition("attack")
        ai.attack_timer = ATTACK_COOLDOWN
        rusher.set_anim(entity, "attack")
    end

    -- Lost sight for too long? Return to idle
    if not physics.line_of_sight(my_pos, target_pos, { "STATIC" }) then
        if ai.fsm:time_in_state() > 3.0 then
            ai.fsm:transition("idle")
            rusher.set_anim(entity, "idle")
            physics.set_velocity(entity, Vec4.ZERO)
        end
    end
end

function rusher.update_attack(entity, ai, dt)
    ai.attack_timer = ai.attack_timer - dt
    physics.set_velocity(entity, Vec4.ZERO)  -- Stop during attack

    if ai.fsm:just_entered() then
        -- Deal damage
        events.emit("damage", {
            target = ai.target,
            amount = ATTACK_DAMAGE,
            source = entity,
        })
    end

    if ai.attack_timer <= 0 then
        ai.fsm:transition("chase")
        rusher.set_anim(entity, "chase")
    end
end

function rusher.update_pain(entity, ai, dt)
    ai.pain_timer = ai.pain_timer - dt
    physics.set_velocity(entity, Vec4.ZERO)  -- Stagger

    if ai.pain_timer <= 0 then
        ai.fsm:transition("chase")
        rusher.set_anim(entity, "chase")
    end
end

function rusher.update_dead(entity, ai, dt)
    physics.set_velocity(entity, Vec4.ZERO)

    -- Despawn after death animation
    if ai.fsm:time_in_state() > 1.0 then
        entity:despawn()
    end
end

function rusher.set_anim(entity, anim_name)
    local sprite = entity:get("sprite_anim")
    sprite.current_anim = anim_name
    entity:set("sprite_anim", sprite)
end

return rusher
```

### 10.4 Trigger Handler

```lua
-- scripts/triggers.lua
-- Handles declarative trigger events from the engine

local triggers = {}

function triggers.init()
    events.on("trigger_enter", function(event)
        triggers.handle_trigger(event.body, event.trigger_name, event.actions)
    end)

    events.on("game_event", function(event)
        triggers.handle_game_event(event.name, event.data)
    end)
end

-- Handle game events fired by trigger actions
function triggers.handle_game_event(name, data)
    if name == "pickup_health_large" then
        triggers.pickup_health(data.entity, 50)
    elseif name == "pickup_ammo_shotgun" then
        triggers.pickup_ammo(data.entity, "shotgun", 4)
    elseif name == "shift_player_w_to_5" then
        triggers.shift_player_w(5.0)
    elseif name == "spawn_enemy_wave_1" then
        triggers.spawn_enemy_wave(1)
    elseif name == "level_complete" then
        triggers.level_complete()
    end
end

function triggers.pickup_health(pickup_entity, amount)
    local player = world.find_by_name("player")
    if not player then return end

    local health = player:get("health")
    health.current = math.min(health.current + amount, health.max)
    player:set("health", health)

    audio.play_oneshot(audio.load_sound("sounds/pickup_health.ogg"), "sfx")
    particles.spawn_burst(pickup_entity:get_position(), {
        count = 20,
        lifetime = 0.5,
        initial_color = { 0, 1, 0, 1 },
        end_color = { 0, 1, 0, 0 },
        speed = 2.0,
        blend_mode = "additive",
    })

    pickup_entity:despawn()
    print("Picked up health: +" .. amount)
end

function triggers.pickup_ammo(pickup_entity, weapon_type, amount)
    local player = world.find_by_name("player")
    if not player then return end

    events.emit("ammo_pickup", {
        weapon_type = weapon_type,
        amount = amount,
    })

    audio.play_oneshot(audio.load_sound("sounds/pickup_ammo.ogg"), "sfx")
    pickup_entity:despawn()
    print("Picked up " .. amount .. " " .. weapon_type .. " ammo")
end

function triggers.shift_player_w(target_w)
    local player = world.find_by_name("player")
    if not player then return end

    tweens.position_w(player, target_w, 0.5, "ease_in_out_quad")
    print("Shifting player to W=" .. target_w)
end

function triggers.spawn_enemy_wave(wave_id)
    print("Spawning enemy wave " .. wave_id)
    -- Wave definitions are data tables
    local waves = require("data/enemy_waves")
    local wave = waves[wave_id]
    if not wave then return end

    for _, spawn in ipairs(wave) do
        local enemy = world.spawn({
            transform = Transform4D.new(spawn.position),
            health = { current = spawn.health, max = spawn.health },
            -- ... full enemy setup
        })
        local ai_module = require("enemies/" .. spawn.type)
        ai_module.create(enemy, spawn.position)
    end
end

function triggers.level_complete()
    print("Level complete!")
    events.emit("level_complete", {})
    -- Show completion screen, save progress, etc.
end

return triggers
```

---

## 11. Performance Considerations

### 11.1 What Stays in Rust

These systems run at high frequency or require low-level hardware access. They must remain in Rust:

| System | Reason |
|--------|--------|
| Physics simulation (`PhysicsWorld::step()`) | Fixed timestep, collision detection, contact resolution -- performance-critical |
| Rendering pipeline (compute shader, render passes) | GPU interaction, shader compilation, buffer management |
| Audio engine (kira backend) | Separate audio thread, low-latency mixing |
| Collision detection (narrowphase) | Per-pair shape tests, contact manifold computation |
| 4D slicing compute shader | GPU-bound, ~100k vertices |

### 11.2 What Runs in Lua

These systems run at game-logic frequency (once per entity per frame or once per event):

| System | Frequency | Expected Cost |
|--------|-----------|---------------|
| Player input processing | 1/frame | Negligible |
| AI state machine updates | N enemies / frame | Low (N < 50) |
| Event dispatch | M events / frame | Low (M < 100) |
| HUD rendering commands | 1/frame | Negligible |
| Weapon fire logic | Occasional | Negligible |
| Trigger handlers | Occasional | Negligible |

### 11.3 Lua Call Overhead and Minimization

**The problem**: Each Lua-to-Rust and Rust-to-Lua boundary crossing has overhead (~100-500ns per call). For per-entity logic running on 50 enemies, this is fine. For per-particle logic on 500 particles, it is not.

**Strategies to minimize overhead**:

1. **Batch operations**: Instead of calling `physics.set_velocity(entity, vel)` 50 times in a loop, provide `physics.set_velocities(entity_vel_pairs)` that processes a batch in one Rust call.

2. **Avoid per-entity Lua calls in hot paths**: The physics step, particle simulation, and rendering are entirely Rust. Lua never touches individual particles or vertices.

3. **Cache frequently accessed data**: Encourage scripts to cache entity handles and component references at module load time rather than querying every frame:

   ```lua
   -- Cache on init, not every frame
   local player = nil
   function on_init()
       player = world.find_by_name("player")
   end
   ```

4. **Use fixed_update for game logic**: Run AI and gameplay logic in `on_fixed_update` (typically 60Hz) rather than `on_update` (which may be higher). This halves the Lua call frequency at 120fps.

5. **Profile and measure**: Add timing instrumentation around Lua callback execution. Log warnings when a single `on_update` call takes more than 2ms:

   ```rust
   let start = std::time::Instant::now();
   call_lifecycle(lua, "on_update", dt)?;
   let elapsed = start.elapsed();
   if elapsed > Duration::from_millis(2) {
       log::warn!("[scripting] on_update took {:?}", elapsed);
   }
   ```

### 11.4 Profiling Hooks

The scripting engine provides optional profiling:

```lua
-- Enable profiling (development only)
debug_profiler.enable()

-- After some gameplay, get report
local report = debug_profiler.report()
-- Reports per-module execution time, top 10 slowest functions,
-- Lua-to-Rust call counts per frame, memory usage
```

On the Rust side, this uses mlua's `HookTriggers` to instrument function calls:

```rust
lua.set_hook(
    HookTriggers::EVERY_NTH_INSTRUCTION { n: 1000 },
    |_lua, debug| {
        // Track instruction count for timeout detection
        // Track function call frequency for profiling
        Ok(())
    },
);
```

### 11.5 Memory Budget

- **Lua VM base**: ~500KB
- **Script code**: ~100KB for a typical game (all scripts loaded)
- **Game state tables**: ~1-5MB (entity data, AI state, inventories)
- **Total expected**: 2-10MB
- **Limit**: 64MB (configurable, hard fail if exceeded)

The GC is configured for game use: incremental collection, small step sizes to avoid frame hitches. `lua.gc_set_incremental(100, 200, 13)` provides a good default.

---

## 12. Verification Criteria

### Sub-Phase A: Core Runtime
- [ ] `ScriptEngine::new()` creates a Lua VM with sandboxed globals
- [ ] `load_game()` finds and executes `main.lua` from the scripts directory
- [ ] `require()` resolves modules relative to the scripts directory
- [ ] `on_init()` callback fires after loading
- [ ] `on_update(dt)` receives correct delta time value
- [ ] `on_fixed_update(dt)` fires at fixed timestep rate
- [ ] `on_shutdown()` fires on engine exit
- [ ] Missing callbacks do not produce errors
- [ ] Lua syntax errors produce `ScriptError` with file and line number
- [ ] Lua runtime errors in callbacks do not crash the engine
- [ ] `os`, `io`, `loadfile`, `dofile` are not accessible from scripts
- [ ] Memory limit triggers error (not crash) when exceeded
- [ ] `print()` routes to the Rust logger

### Sub-Phase B: ECS Bindings
- [ ] `world.spawn()` creates an entity with the specified components
- [ ] `entity:get("component_name")` returns correct component data as Lua table
- [ ] `entity:set("component_name", table)` updates component in ECS
- [ ] `entity:get_position()` / `entity:set_position()` fast path works correctly
- [ ] `entity:despawn()` removes entity from the world
- [ ] `entity:is_alive()` returns false after despawn
- [ ] `world.query("a", "b")` returns iterator over entities with both components
- [ ] `world.find_by_name(name)` returns correct entity or nil
- [ ] `world.find_by_tag(tag)` returns all matching entities
- [ ] Script-defined components survive round-trip through Lua tables
- [ ] Spawning 1000 entities from Lua completes without error or excessive time

### Sub-Phase C: Engine API Bindings
- [ ] `Vec4` arithmetic operators (`+`, `-`, `*`, unary `-`) produce correct results
- [ ] `Vec4:dot()`, `:length()`, `:normalized()`, `:distance()` match Rust implementations
- [ ] `Rotor4` rotation applied from Lua matches Rust rotation
- [ ] `physics.raycast()` returns hits sorted by distance
- [ ] `physics.raycast_nearest()` returns closest hit or nil
- [ ] `physics.query_sphere()` returns entities within radius
- [ ] `physics.line_of_sight()` returns correct boolean
- [ ] `input.is_action_pressed()` reflects current input state
- [ ] `audio.load_sound()` returns a valid handle
- [ ] `audio.play_spatial()` plays sound at 4D position
- [ ] `particles.spawn_burst()` creates visible particles
- [ ] HUD text drawing produces visible on-screen text
- [ ] `scene.load()` triggers scene loading in the engine

### Sub-Phase D: Hot-Reload
- [ ] File watcher detects `.lua` file modifications
- [ ] Modified module is re-executed with updated code
- [ ] `on_reload()` callback fires after successful reload
- [ ] Syntax errors in reloaded scripts keep old version running
- [ ] Runtime errors in reloaded scripts keep old version running
- [ ] Error messages include file path and line number
- [ ] Dev console evaluates arbitrary Lua expressions
- [ ] Dev console displays results or errors

### Sub-Phase E: Game Framework Bindings
- [ ] `CharacterController4D` move and jump work from Lua
- [ ] `events.on()` registers handlers that receive emitted events
- [ ] `events.emit()` dispatches to registered handlers
- [ ] Collision events from physics reach Lua handlers
- [ ] Trigger enter/exit events from physics reach Lua handlers
- [ ] `StateMachine` transitions and state queries work from Lua
- [ ] `tweens.position()` starts a tween that animates entity position
- [ ] `screen_shake.add_trauma()` produces visible camera shake
- [ ] `TimedEffect` intensity decay works from Lua
- [ ] Timer callbacks (after, every) fire at correct times

### Integration
- [ ] A complete game loop functions: Lua `on_init` spawns entities -> `on_update` processes input and updates AI -> entities move -> HUD displays state -> hot-reload updates logic without restart
- [ ] Memory usage stays below 64MB with a typical game scenario
- [ ] Frame time contribution from Lua stays below 2ms with 50 active scripted entities
- [ ] `cargo build --workspace` succeeds with the new crate
- [ ] `cargo test --workspace` passes all new and existing tests
