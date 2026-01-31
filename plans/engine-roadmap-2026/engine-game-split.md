# Engine/Game Split + Lua Scripting Architecture

## Overview

Split the Rust4D project into two repositories with a **Lua scripting** approach:

1. **Rust4D Engine** -- a 4D game engine (this repo, refactored) that ships as both a library and a binary/launcher. The engine provides Lua scripting via `rust4d_scripting`, exposing all engine APIs to Lua. The binary loads a game directory containing Lua scripts, RON scenes, and assets.
2. **Rust4D Shooter** -- a game project consisting entirely of Lua scripts, RON scene files, TOML configuration, and assets. No Rust compilation required. No `Cargo.toml`. The game IS data + scripts.

The engine also migrates to full ECS (hecs) since every downstream feature depends on a stable entity API.

### The Architectural Shift

**BEFORE** (original plan): Game repo is a compiled Rust binary (`Cargo.toml` + `src/`) depending on engine crates via git URL. All game logic (weapons, enemies, AI, health) is compiled Rust.

**AFTER** (this plan): Engine provides a `rust4d_scripting` crate with Lua 5.4 runtime (via `mlua`). Engine ships a binary/launcher that loads game scripts from a game directory. Game repo contains Lua scripts + RON scenes + TOML config + assets. Hot-reload enables edit-save-test workflow without recompilation.

## Goals

- Clean separation between engine and game code
- Engine is reusable for any 4D game, not just a boomer shooter
- Full hecs-based ECS replaces monolithic Entity
- New `rust4d_scripting` crate provides Lua runtime integration with all engine APIs exposed as Lua bindings
- New `rust4d_game` crate provides Rust implementations of common game patterns (CharacterController4D, events, FSM) that get Lua wrappers
- Game-specific code lives as Lua scripts in a game directory loaded by the engine binary
- Hot-reload of Lua scripts during development (edit-save-test without recompilation)
- Engine exposes clean Lua APIs for ECS, physics, rendering, input, and scene management

## Non-Goals

- Building gameplay systems (weapons, enemies, etc.) in Rust -- that is Lua game script territory
- Changing the rendering pipeline architecture
- Publishing engine crates to crates.io (future concern)
- Lua-based rendering or physics -- performance-critical systems stay in Rust
- Full visual scripting / node-based editor (Lua text scripts are the scripting layer)
- LuaJIT support (targeting Lua 5.4 for consistency; LuaJIT can be evaluated later)

## Decisions

### Game directory model (engine loads game from a path)

The engine binary takes a path argument (or defaults to `./game/`) pointing to a game directory containing `main.lua`, scripts, scenes, config, and assets. The game repo IS that directory.

```
# Running a game
rust4d --game ./path/to/shooter/

# Or with default path
cd my-project/
rust4d              # looks for ./game/ or ./main.lua
```

For development, the game directory can be a subdirectory of the engine workspace (gitignored), or a separate repo cloned alongside the engine. The engine binary is either installed system-wide, added to PATH, or run via `cargo run -- --game ../Rust4D-Shooter/`.

```
# Development layout option A: game as subdirectory of engine workspace
Rust4D/
  crates/
  game/               # gitignored, symlink or clone of game repo
  Cargo.toml

# Development layout option B: sibling directories
Projects/
  Rust4D/             # engine repo
  Rust4D-Shooter/     # game repo (the game directory itself)
```

### mlua for Lua bindings (Lua 5.4)

- **mlua** is actively maintained, supports Lua 5.4, has excellent `UserData` support for exposing Rust types
- Lua 5.4 chosen over LuaJIT: integers, better standard library, active development. LuaJIT performance can be evaluated later if scripting becomes a bottleneck.
- `UserData` trait on engine types (Vec4, Transform4D, PhysicsBody, etc.) provides type-safe Lua access
- Lua tables for component bundles, RON for scene data, TOML for config

### New `rust4d_scripting` crate for Lua runtime integration

A new crate that owns the Lua VM, binding generation, script lifecycle, hot-reload, and error handling:
- Creates and manages the `mlua::Lua` instance
- Registers all engine type bindings (math, physics, ECS, input, scene)
- Manages script loading, module resolution, and the `require` system
- Provides the script lifecycle (on_init, on_update, on_fixed_update, on_event, on_reload)
- Implements file-watcher-based hot-reload
- Handles Lua errors gracefully (log + continue, don't crash the engine)

### `rust4d_game` crate provides Rust implementations exposed TO Lua

The role of `rust4d_game` changes from "Rust types the game imports" to "Rust implementations that get Lua wrappers":
- **CharacterController4D** -- Rust implementation, exposed to Lua via `UserData`. Lua scripts call `controller:apply_movement(dx, dy, dz, dw)`, `controller:jump()`, etc.
- **Event system** -- Rust-side event bus, Lua scripts register handlers: `events.on("damage", function(target, amount) ... end)`
- **State machine** -- Generic FSM exposed to Lua for AI, game states, animation states
- **Scene setup helpers** -- Rust helpers callable from Lua for common patterns (tag-based physics wiring)

### Collision layer presets stay in engine

`CollisionFilter::player()`, `::enemy()`, etc. are convenience presets -- just named bitflags. They are useful defaults for any game, not boomer-shooter-specific. Exposed to Lua as `CollisionFilter.player()`.

### Engine ships a binary (`rust4d` launcher)

The engine workspace produces a binary target that:
1. Parses CLI args (game directory path, debug flags, window config overrides)
2. Initializes the engine (window, GPU, physics, ECS world, input)
3. Creates the Lua VM and registers all bindings
4. Loads `main.lua` from the game directory
5. Calls script lifecycle hooks in the game loop (init, update, fixed_update, render)
6. Watches for Lua file changes and triggers hot-reload

## Architecture / Design

### Current State
```
Rust4D/
  crates/
    rust4d_math        # Pure 4D math (Vec4, Rotor4, shapes)          <- ENGINE
    rust4d_physics     # 4D physics (bodies, collision, world)        <- ENGINE (player-specific code mixed in)
    rust4d_core        # Entity, World, Scene, Assets                 <- ENGINE (game-specific scene logic)
    rust4d_render      # GPU rendering pipeline                       <- ENGINE
    rust4d_input       # Camera controller                            <- ENGINE
  src/                 # Tech demo binary + systems                   <- GAME-SPECIFIC
  examples/            # Example programs                             <- ENGINE (stays)
  scenes/              # RON scene files                              <- GAME-SPECIFIC
  config/              # TOML config files                            <- GAME-SPECIFIC
```

### Target State
```
Rust4D/                              # Engine repo (library + binary)
  crates/
    rust4d_math                      # Unchanged (pure math)
    rust4d_physics                   # Refactored: player logic extracted to generic API
    rust4d_core                      # Refactored: ECS via hecs, generic scene instantiation
    rust4d_game                      # NEW: CharacterController4D, events, FSM (Rust impls for Lua)
    rust4d_scripting                 # NEW: Lua VM, bindings, script lifecycle, hot-reload
    rust4d_render                    # Refactored: works with ECS components instead of Entity
    rust4d_input                     # Refactored: action/axis abstraction
  src/
    main.rs                          # Engine launcher binary (loads game dir, runs game loop)
    cli.rs                           # CLI argument parsing
    game_loop.rs                     # Main loop: input -> fixed_update -> update -> render
  examples/                          # ECS-based examples showcasing engine API
  Cargo.toml                         # Workspace with binary target

Rust4D-Shooter/                      # Game repo (Lua scripts + data, NO Cargo.toml)
  main.lua                           # Game entry point (registers systems, loads first scene)
  config.toml                        # Game configuration (window size, physics params, etc.)
  scripts/
    systems/
      simulation.lua                 # Physics step, character controller updates
      combat.lua                     # Damage, weapons, health
      enemy_ai.lua                   # Enemy state machines, pathfinding
      hud.lua                        # HUD rendering via engine overlay API
    entities/
      player.lua                     # Player setup, input handling, movement
      weapons.lua                    # Weapon definitions, fire logic, ammo
      enemies.lua                    # Enemy type definitions, spawn logic
      pickups.lua                    # Pickup definitions, collection logic
    events/
      damage.lua                     # Damage event handlers
      triggers.lua                   # Trigger zone responses (doors, elevators, portals)
    ui/
      menus.lua                      # Main menu, pause menu, settings
  scenes/
    level_01.ron                     # RON scene files (unchanged format)
    level_02.ron
  assets/
    sounds/                          # WAV/OGG sound effects
    textures/                        # PNG/JPG textures
    sprites/                         # Enemy sprite sheets
```

### Dependency Chain (Engine Crates)
```
rust4d_math
  ^
rust4d_physics (depends on math)
  ^
rust4d_core (depends on math, physics)
  ^
rust4d_game (depends on core, physics, math)          <- NEW
  ^
rust4d_scripting (depends on game, core, physics,     <- NEW
                  math, render, input; also mlua)
  ^
rust4d_render (depends on core, math, input)

Engine binary depends on: rust4d_scripting (which pulls in everything)
```

### What Stays in Rust (Performance-Critical)
- **rust4d_math**: All of it. Pure 4D math with no game assumptions. Vec4, Rotor4, Transform4D, shapes.
- **rust4d_physics**: Body simulation, collision detection, broadphase/narrowphase, collision layers/filters, materials, raycasting. The physics step runs entirely in Rust.
- **rust4d_core**: ECS world (hecs), component types, scene loading/saving (RON), scene manager, asset cache, hierarchy.
- **rust4d_render**: GPU slicing pipeline, Camera4D, RenderableGeometry, sprite batching, particle systems. All GPU work stays in Rust.
- **rust4d_input**: Input polling, action/axis abstraction, camera controller math.
- **rust4d_game**: CharacterController4D, event bus internals, FSM execution. These are Rust implementations exposed to Lua -- the heavy lifting stays in Rust.

### What is Scriptable in Lua (Game Logic)
- Game initialization (which scene to load, what entities to spawn)
- Player input handling (mapping input actions to character controller calls)
- Game systems that run each frame (AI tick, weapon cooldowns, pickup checks)
- Event handlers (on damage, on death, on trigger enter/exit, on pickup)
- UI/HUD layout and behavior (via engine overlay API)
- State machines (enemy AI states, game state transitions)
- Level scripting (door logic, elevator sequences, W-portal transitions, spawn waves)
- Game configuration and tuning (damage values, enemy stats, weapon params)

### Script Lifecycle

Scripts register callbacks. The engine calls these at appropriate points in the game loop:

```lua
-- main.lua
local player = require("scripts.entities.player")
local combat = require("scripts.systems.combat")
local hud = require("scripts.systems.hud")

function on_init(engine)
    -- Called once after Lua VM is created and bindings are registered
    -- engine provides access to World, PhysicsWorld, SceneManager, Input, etc.
    engine.scene:load("scenes/level_01.ron")
    player.init(engine)
    combat.init(engine)
end

function on_update(engine, dt)
    -- Called every frame (variable timestep)
    player.update(engine, dt)
    combat.update(engine, dt)
    hud.draw(engine)
end

function on_fixed_update(engine, dt)
    -- Called at fixed physics rate (e.g., 60Hz)
    player.fixed_update(engine, dt)
    combat.fixed_update(engine, dt)
end

function on_event(engine, event_type, event_data)
    -- Called when engine events fire (collision, trigger, input)
    combat.on_event(engine, event_type, event_data)
end

function on_reload()
    -- Called after hot-reload. Opportunity to re-register handlers.
    print("Scripts reloaded!")
end
```

### ECS from Lua

Scripts can interact with the ECS world through the engine bindings:

```lua
-- Spawn an entity with components
local entity = engine.world:spawn({
    Name = "health_pickup",
    Tags = {"pickup", "health"},
    Transform4D = Transform4D.new(10, 1, 5, 0),
    Shape = Shape.hypersphere(0.5),
    Material = Material.new(0.2, 0.8, 0.2, 1.0),  -- green
})

-- Query entities with specific components
for entity, transform, health in engine.world:query("Transform4D", "Health") do
    if health.current <= 0 then
        engine.world:despawn(entity)
    end
end

-- Add/remove components dynamically
engine.world:insert(entity, { Health = Health.new(100) })
engine.world:remove(entity, "Health")

-- Register a "script system" that runs each frame
engine.world:register_system("decay_system", {"Health", "Transform4D"}, function(entity, health, transform)
    -- Runs for each entity matching the query
end)
```

### Hot-Reload Design

1. **File watcher** (notify crate) monitors the game directory for `.lua` file changes
2. On change detection, the engine:
   a. Calls `on_pre_reload()` if defined (scripts can serialize state to a global table)
   b. Clears the Lua module cache for the changed module (and dependents)
   c. Re-requires the changed modules
   d. Calls `on_reload()` if defined (scripts can restore state)
3. Script state preservation via a special `persist` table that survives reload:
   ```lua
   -- State that survives hot-reload
   persist.score = persist.score or 0
   persist.player_health = persist.player_health or 100
   ```
4. Hot-reload scope: individual modules, not the entire VM. Unchanged modules keep their state.
5. Errors during reload: log the error, keep the previous version of the module loaded. Never crash.

### Lua Binding Examples

What the bindings look like from the Lua side:

```lua
-- Math types (UserData)
local v = Vec4.new(1, 2, 3, 4)
local w = Vec4.new(5, 6, 7, 8)
local sum = v + w                          -- operator overloading via metamethods
local dot = v:dot(w)
local len = v:magnitude()
local n = v:normalized()

-- Transform4D (UserData)
local t = Transform4D.new(0, 5, 0, 0)
t:translate(Vec4.new(1, 0, 0, 0))
t:rotate_xz(0.5)
local pos = t:position()

-- Physics (UserData wrapping BodyKey)
local body = engine.physics:create_body({
    body_type = "kinematic",
    shape = Shape.hypersphere(0.5),
    position = Vec4.new(0, 5, 0, 0),
    collision_filter = CollisionFilter.player(),
})
engine.physics:apply_impulse(body, Vec4.new(10, 0, 0, 0))
local grounded = engine.physics:body_is_grounded(body)

-- CharacterController4D (UserData, from rust4d_game)
local controller = CharacterController4D.new(engine.physics, body, {
    move_speed = 10.0,
    jump_velocity = 8.0,
})
controller:apply_movement(dx, dy, dz, dw)
controller:jump()
local pos = controller:position()
local on_ground = controller:is_grounded()

-- Scene loading
engine.scene:load("scenes/level_01.ron")
local spawn_point = engine.scene:player_spawn()

-- Input queries
if engine.input:action_pressed("jump") then
    controller:jump()
end
local move_x = engine.input:axis("move_right")
local move_z = engine.input:axis("move_forward")
local mouse_dx, mouse_dy = engine.input:mouse_delta()

-- Events
engine.events:on("collision", function(entity_a, entity_b, contact_point)
    -- handle collision
end)
engine.events:on("trigger_enter", function(trigger_entity, entering_entity)
    -- handle trigger
end)
engine.events:emit("game_event", { type = "door_open", door = door_entity })
```

### Code That Needs Refactoring (Currently Game-Specific, Embedded in Engine)

These are the same refactoring tasks from the original plan -- they are still needed regardless of whether the game is Rust or Lua:

1. **`PhysicsWorld` player methods** (`world.rs:54-57, plus ~200 lines of player-specific logic`):
   - `set_player_body()`, `player()`, `player_mut()`, `player_position()`, `apply_player_movement()`, `player_jump()`, `player_is_grounded()`
   - **Action**: Extract to `rust4d_game::CharacterController4D`. PhysicsWorld gets generic body manipulation methods. CharacterController4D is then exposed to Lua.

2. **`PhysicsConfig.jump_velocity`** (`world.rs:17`):
   - **Action**: Remove from PhysicsConfig. Moves to CharacterController4D config (which Lua scripts set).

3. **`ActiveScene::from_template()` tag-based physics setup** (`scene.rs:227-310`):
   - **Action**: Engine's `from_template()` spawns entities with core components only. `rust4d_game` provides scene setup helpers callable from Lua.

4. **`PhysicsWorld.step()` player-specific simulation** (`world.rs:~193-280`):
   - **Action**: Generalize gravity to work on any body. Edge-falling and grounded detection move to CharacterController4D.

### ECS Migration Design

Same as original plan -- replace the monolithic `Entity` struct and custom `World` with hecs:

**Current Entity struct** (7 fields):
```rust
struct Entity {
    name: Option<String>,
    tags: HashSet<String>,
    transform: Transform4D,
    shape: ShapeRef,
    material: Material,
    physics_body: Option<BodyKey>,
    dirty: DirtyFlags,
}
```

**New ECS components** (in `rust4d_core`):
```rust
struct Name(String);
struct Tags(HashSet<String>);
struct Transform4D { ... }      // Already exists
struct Material { ... }         // Already exists
struct ShapeRef { ... }         // Already exists
struct PhysicsBody(BodyKey);
struct DirtyFlags { ... }       // Already exists
struct Parent(hecs::Entity);
struct Children(Vec<hecs::Entity>);
```

All of these get Lua bindings in `rust4d_scripting`. Lua scripts can query, add, and remove components on entities.

**Game-defined components from Lua:**
```lua
-- Lua scripts define game components as tables
-- Stored in ECS via a generic ScriptComponent wrapper
engine.world:define_component("Health", { current = 100, max = 100 })
engine.world:define_component("Weapon", { type = "shotgun", ammo = 8, damage = 25 })
engine.world:define_component("EnemyAI", { state = "idle", sight_range = 20 })
```

## Implementation Phases

### Phase 1: ECS Migration in Engine
**Goal:** Replace monolithic Entity/World with hecs-based ECS while keeping everything compiling and tests passing.

**This phase is largely unchanged from the original plan.** The ECS migration is an internal engine change that must happen regardless of whether the game is Rust or Lua.

**Files (engine repo):**
- `crates/rust4d_core/Cargo.toml` - Add hecs dependency
- `crates/rust4d_core/src/components.rs` - NEW: Engine component types (Name, Tags, Transform4D, Material, ShapeRef, PhysicsBody, DirtyFlags)
- `crates/rust4d_core/src/world.rs` - Rewrite: Thin wrapper around hecs::World with physics integration, hierarchy, name index
- `crates/rust4d_core/src/entity.rs` - Refactor: EntityTemplate stays for serialization, Entity struct removed (replaced by component bundles)
- `crates/rust4d_core/src/scene.rs` - Refactor: ActiveScene::from_template spawns ECS entities with component bundles
- `crates/rust4d_core/src/lib.rs` - Update exports
- `crates/rust4d_render/src/renderable.rs` - Refactor: from_world() uses ECS queries instead of iterating Entity structs

**Tasks:**
- [ ] Add hecs to workspace dependencies
- [ ] Create engine component types as simple structs
- [ ] Create EntityBundle type for spawning (Transform4D, ShapeRef, Material + optional Name, Tags, PhysicsBody)
- [ ] Rewrite World as hecs::World wrapper with physics integration
- [ ] Port hierarchy system to Parent/Children components
- [ ] Port name index to side-table maintained on spawn/despawn
- [ ] Port dirty tracking to DirtyFlags component
- [ ] Refactor ActiveScene::from_template to spawn ECS entities
- [ ] Refactor RenderableGeometry::from_world to use ECS queries
- [ ] Update EntityTemplate serialization to produce component bundles
- [ ] Port all rust4d_core tests (202 tests)
- [ ] Port all integration tests
- [ ] Update examples to use new ECS API
- [ ] Update src/ binary to use new API (temporary, will be replaced by launcher in Phase 4)

**Verification:**
- `cargo test --workspace` passes (all 358+ tests)
- Examples run correctly
- Tech demo binary still works

### Phase 2: Extract Game Logic from Engine + Create rust4d_game
**Goal:** Remove game-specific assumptions from the physics and input crates. Create the `rust4d_game` framework crate with Rust implementations that will later get Lua wrappers.

**This phase is also largely unchanged from the original plan.** The game logic extraction is about making the engine generic. The `rust4d_game` crate provides the same types (CharacterController4D, events, FSM) -- the difference is they will be wrapped for Lua access in Phase 3 rather than imported by a Rust game binary.

**Files (engine repo):**
- `crates/rust4d_game/Cargo.toml` - NEW: depends on rust4d_core, rust4d_physics, rust4d_math
- `crates/rust4d_game/src/lib.rs` - NEW: crate root
- `crates/rust4d_game/src/character_controller.rs` - NEW: CharacterController4D
- `crates/rust4d_game/src/events.rs` - NEW: Event bus / channel system
- `crates/rust4d_game/src/fsm.rs` - NEW: Generic state machine
- `crates/rust4d_game/src/scene_helpers.rs` - NEW: Tag-based physics wiring, spawn point detection
- `crates/rust4d_physics/src/world.rs` - Remove player-specific methods, add generic body methods
- `crates/rust4d_input/src/camera_controller.rs` - Refactor: abstract input actions
- `crates/rust4d_input/src/input_actions.rs` - NEW: Action/axis abstraction

**Specific changes to PhysicsWorld:**
- Remove fields: `player_body: Option<BodyKey>`, `player_jump_velocity: f32`
- Remove methods: `set_player_body()`, `player()`, `player_mut()`, `player_position()`, `player_is_grounded()`, `apply_player_movement()`, `player_jump()`
- Remove from `PhysicsConfig`: `jump_velocity`
- Add generic body methods: `body_is_grounded(key) -> bool`, `apply_body_movement(key, movement)`, `set_body_velocity_xzw(key, velocity)`, etc.
- Generalize gravity: any body can opt in to gravity (not just "the player")
- Keep grounded detection and edge-falling as generic per-body logic

**CharacterController4D** (in rust4d_game):
- Wraps a `BodyKey` in a `PhysicsWorld`
- Owns: jump_velocity, move_speed, grounded state tracking
- Methods: `apply_movement()`, `jump()`, `is_grounded()`, `sync_position() -> Vec4`
- Internally calls generic PhysicsWorld body methods
- Will be wrapped as Lua `UserData` in Phase 3

**Scene helpers** (in rust4d_game):
- `setup_physics_from_tags(world, physics)` -- "static" -> collider, "dynamic" -> rigid body
- `find_player_spawn(world) -> Option<Vec4>` -- locate spawn point from scene data
- Callable from Rust and will be exposed to Lua in Phase 3

**Tasks:**
- [ ] Create rust4d_game crate with Cargo.toml
- [ ] Remove `jump_velocity` from PhysicsConfig
- [ ] Remove player-specific fields and methods from PhysicsWorld
- [ ] Add generic body manipulation methods to PhysicsWorld
- [ ] Generalize grounded detection and edge-falling to work on any specified body
- [ ] Implement CharacterController4D in rust4d_game
- [ ] Implement basic event system in rust4d_game
- [ ] Implement generic FSM in rust4d_game
- [ ] Move tag-based scene setup to rust4d_game::scene_helpers
- [ ] Create input action/axis abstraction in rust4d_input
- [ ] Refactor CameraController to use abstract actions
- [ ] Create InputMap with default bindings
- [ ] Update all physics tests (97 tests)
- [ ] Update all input tests (37 tests)
- [ ] Add CharacterController4D tests
- [ ] Add FSM tests
- [ ] Update src/ binary to use CharacterController4D, InputMap, and scene helpers

**Verification:**
- `cargo test --workspace` passes
- Physics demo example still works
- Tech demo binary still works with CharacterController4D and InputMap
- CharacterController4D produces identical behavior to old player methods
- Input behavior is identical with default InputMap
- Scene setup via helpers produces identical behavior

### Phase 3: Lua Scripting Integration (NEW)
**Goal:** Create the `rust4d_scripting` crate. Integrate `mlua`, implement Lua bindings for all engine types, build the script lifecycle and hot-reload system.

This is the **new phase** that replaces the old "create game repository" step. Instead of moving Rust code to a new repo, we create the Lua bridge that lets scripts drive game logic.

**Files (engine repo):**
- `crates/rust4d_scripting/Cargo.toml` - NEW: depends on mlua, rust4d_game, rust4d_core, rust4d_physics, rust4d_math, rust4d_render, rust4d_input, notify (file watcher)
- `crates/rust4d_scripting/src/lib.rs` - Crate root: `ScriptEngine` struct
- `crates/rust4d_scripting/src/vm.rs` - Lua VM creation, sandbox configuration, module resolution
- `crates/rust4d_scripting/src/lifecycle.rs` - Script lifecycle management (on_init, on_update, on_fixed_update, on_event, on_reload)
- `crates/rust4d_scripting/src/hot_reload.rs` - File watcher, module invalidation, state preservation
- `crates/rust4d_scripting/src/error.rs` - Lua error handling, error display, graceful recovery
- `crates/rust4d_scripting/src/bindings/mod.rs` - Binding registration entry point
- `crates/rust4d_scripting/src/bindings/math.rs` - Vec4, Rotor4, Transform4D bindings (UserData)
- `crates/rust4d_scripting/src/bindings/physics.rs` - PhysicsWorld, BodyKey, CollisionFilter bindings
- `crates/rust4d_scripting/src/bindings/ecs.rs` - World queries, entity spawn/despawn, component access, script-defined components
- `crates/rust4d_scripting/src/bindings/scene.rs` - Scene loading, player spawn, scene helpers
- `crates/rust4d_scripting/src/bindings/input.rs` - Input action queries, mouse delta, InputMap access
- `crates/rust4d_scripting/src/bindings/game.rs` - CharacterController4D, EventBus, FSM bindings
- `crates/rust4d_scripting/src/bindings/render.rs` - Camera4D access, overlay/egui context (for HUD)
- `crates/rust4d_scripting/src/script_component.rs` - Generic ScriptComponent for Lua-defined component data (stored as Lua table serialized to HashMap)
- `Cargo.toml` - Add rust4d_scripting to workspace members, add mlua + notify to workspace deps

**ScriptEngine API (Rust-side):**
```rust
pub struct ScriptEngine {
    lua: Lua,
    game_dir: PathBuf,
    watcher: Option<RecommendedWatcher>,
    pending_reloads: Vec<PathBuf>,
}

impl ScriptEngine {
    pub fn new(game_dir: &Path) -> Result<Self>;
    pub fn register_bindings(&mut self, world: &World, physics: &PhysicsWorld, ...) -> Result<()>;
    pub fn load_main(&mut self) -> Result<()>;              // Loads and executes main.lua
    pub fn call_init(&self) -> Result<()>;                   // Calls on_init()
    pub fn call_update(&self, dt: f32) -> Result<()>;        // Calls on_update(dt)
    pub fn call_fixed_update(&self, dt: f32) -> Result<()>;  // Calls on_fixed_update(dt)
    pub fn call_event(&self, event_type: &str, data: LuaValue) -> Result<()>;
    pub fn check_hot_reload(&mut self) -> Result<bool>;      // Returns true if reload happened
    pub fn enable_hot_reload(&mut self) -> Result<()>;
}
```

**Binding approach:**
- Engine types implement `mlua::UserData` trait, which defines methods and metamethods accessible from Lua
- Read-only access to engine state by default; mutations go through explicit method calls
- The `engine` table passed to Lua callbacks holds references (via `UserData`) to the live engine state
- Script-defined components are stored as `ScriptComponent(HashMap<String, LuaValue>)` in hecs, bridging Lua tables to the Rust ECS

**Module resolution:**
- `require("scripts.entities.player")` resolves to `<game_dir>/scripts/entities/player.lua`
- Custom Lua `package.path` set to game directory root
- Engine provides a `rust4d` module with built-in utilities (logging, profiling, etc.)

**Tasks:**
- [ ] Add mlua (with `lua54` and `vendored` features) and notify to workspace deps
- [ ] Create rust4d_scripting crate skeleton
- [ ] Implement Lua VM creation with sandboxed globals
- [ ] Implement Vec4 UserData bindings (constructor, arithmetic metamethods, methods)
- [ ] Implement Rotor4 UserData bindings
- [ ] Implement Transform4D UserData bindings
- [ ] Implement PhysicsWorld bindings (create_body, apply_impulse, raycast, body_is_grounded, etc.)
- [ ] Implement CollisionFilter bindings (player, enemy, projectile, trigger presets)
- [ ] Implement World/ECS bindings (spawn, despawn, query, insert component, remove component)
- [ ] Implement ScriptComponent for Lua-defined component data in hecs
- [ ] Implement Scene bindings (load, player_spawn, scene helpers)
- [ ] Implement Input bindings (action_pressed, action_held, axis, mouse_delta)
- [ ] Implement CharacterController4D Lua bindings
- [ ] Implement EventBus Lua bindings (on, emit, off)
- [ ] Implement FSM Lua bindings
- [ ] Implement Camera4D read access for HUD (viewport size, etc.)
- [ ] Implement script lifecycle (on_init, on_update, on_fixed_update, on_event)
- [ ] Implement module resolution (require with game_dir-relative paths)
- [ ] Implement hot-reload: file watcher, module cache invalidation, on_reload callback
- [ ] Implement persist table for state preservation across reloads
- [ ] Implement error handling: catch Lua panics, log with file/line, continue engine
- [ ] Write Lua binding tests (test each binding category from Rust using mlua)
- [ ] Write hot-reload integration test
- [ ] Write script lifecycle integration test
- [ ] Create example Lua scripts for testing (spawn entities, move character, handle events)

**Verification:**
- `cargo test --workspace` passes (including new scripting tests)
- A minimal Lua script can spawn entities, move a character controller, and handle events
- Hot-reload works: modify a .lua file, see changes take effect without restart
- Lua errors are caught and logged, engine continues running
- All engine types are accessible from Lua with correct behavior

### Phase 4: Engine Binary / Launcher (NEW)
**Goal:** Create the engine launcher binary that loads a game directory and runs the Lua-scripted game loop.

This phase replaces the old "create game repository" phase. Instead of moving Rust code to a new Cargo project, we build the engine binary that loads Lua scripts.

**Files (engine repo):**
- `src/main.rs` - REWRITE: Engine launcher (replaces tech demo binary)
- `src/cli.rs` - NEW: CLI argument parsing (game dir, debug flags, window overrides)
- `src/game_loop.rs` - NEW: Main game loop integrating engine systems + Lua script calls
- `src/config_loader.rs` - NEW: Load game config.toml from game directory

**Launcher architecture:**
```
main()
  -> parse CLI args (game dir path)
  -> load config.toml from game dir
  -> init window (winit)
  -> init GPU (wgpu)
  -> init PhysicsWorld
  -> init ECS World
  -> init InputManager
  -> init ScriptEngine(game_dir)
  -> register all bindings
  -> load main.lua
  -> call on_init()
  -> enter game loop:
       poll input
       while physics_accumulator >= fixed_dt:
           call on_fixed_update(fixed_dt)
           physics.step(fixed_dt)
           physics_accumulator -= fixed_dt
       call on_update(frame_dt)
       render
       check_hot_reload()
  -> on exit: call on_shutdown() if defined
```

**Config loading from game directory:**
```toml
# Rust4D-Shooter/config.toml
[window]
title = "Rust4D Shooter"
width = 1280
height = 720
fullscreen = false

[physics]
gravity = 20.0
fixed_timestep = 0.016667  # 60 Hz

[rendering]
ambient_strength = 0.15
slice_w = 0.0

[game]
start_scene = "scenes/level_01.ron"
```

**Tasks:**
- [ ] Implement CLI argument parsing (clap or manual): `--game <path>`, `--debug`, `--width`, `--height`, `--fullscreen`
- [ ] Implement config.toml loader from game directory
- [ ] Rewrite src/main.rs as engine launcher:
  - Initialize all engine subsystems
  - Create ScriptEngine and load main.lua
  - Run game loop with script lifecycle calls
- [ ] Implement game_loop.rs with fixed timestep + variable render
- [ ] Wire up input polling -> Lua input bindings
- [ ] Wire up physics step -> Lua fixed_update
- [ ] Wire up render pass -> Lua update (for HUD, etc.)
- [ ] Wire up hot-reload check in game loop
- [ ] Port the current tech demo behavior to a Lua game script as proof-of-concept:
  - Load scene, spawn player, handle input, move camera
- [ ] Remove old tech demo systems from src/ (simulation.rs, render.rs, window.rs, config.rs, input_mapper.rs)
- [ ] Update engine README to describe it as a launcher + library

**Verification:**
- `cargo run -- --game examples/lua_demo/` launches the engine with a Lua demo game
- The Lua demo game reproduces the behavior of the old tech demo binary
- Hot-reload works in the running game
- CLI flags work (--debug, --fullscreen, etc.)
- Config from game directory is correctly loaded and applied

### Phase 5: Game Repository Setup (Lua Scripts + Assets)
**Goal:** Create the Rust4D-Shooter game repository with Lua scripts, RON scenes, and assets.

This is **much simpler** than the original Phase 4 which involved creating a Rust Cargo project, adapting imports, and setting up git URL dependencies. The game repo is just a directory of scripts and data.

**New repo structure:**
```
Rust4D-Shooter/                      # Game repo (NO Cargo.toml)
  main.lua                           # Entry point
  config.toml                        # Game config
  scripts/
    systems/
      simulation.lua                 # Per-frame simulation (adapted from old src/systems/simulation.rs logic)
      combat.lua                     # Damage, weapons, health
      hud.lua                        # HUD rendering
    entities/
      player.lua                     # Player setup, CharacterController4D usage via Lua
      weapons.lua                    # Weapon definitions
    events/
      damage.lua                     # Damage event handlers
      triggers.lua                   # Trigger handlers (doors, elevators, W-portals)
    ui/
      menus.lua                      # Main menu, pause menu
  scenes/                            # From Rust4D/scenes/ (RON, unchanged format)
    level_01.ron
  assets/
    sounds/
    textures/
    sprites/
```

**Key differences from original Phase 4:**
- No `Cargo.toml` or `.cargo/config.toml`
- No compiled Rust code
- No git URL dependencies to manage
- Game logic is ported from Rust to Lua (not copy-pasted)
- Running the game: `rust4d --game ./Rust4D-Shooter/` (engine binary is the only compiled artifact)

**Tasks:**
- [ ] Create new GitHub repo (Rust4D-Shooter)
- [ ] Create main.lua game entry point
- [ ] Create config.toml with game settings
- [ ] Port simulation logic to Lua (scripts/systems/simulation.lua)
- [ ] Port player input handling to Lua (scripts/entities/player.lua) using CharacterController4D Lua API
- [ ] Move RON scene files to game repo
- [ ] Move config files to game repo (adapted to config.toml format)
- [ ] Set up asset directories
- [ ] Write a basic combat system in Lua as proof-of-concept
- [ ] Remove scenes/ and config/ from engine repo
- [ ] Keep examples/ in engine repo (including a minimal Lua example game)
- [ ] Document how to run: `rust4d --game ./Rust4D-Shooter/`

**Verification:**
- `cargo run -- --game ../Rust4D-Shooter/` from engine repo starts the game
- Game behavior reproduces the old tech demo
- Modifying a Lua script and saving triggers hot-reload
- Game runs independently when given the engine binary path

### Phase 6: Engine Cleanup
**Goal:** Clean up the engine repo now that game code is extracted and the launcher is in place.

**Tasks:**
- [ ] Ensure root Cargo.toml correctly defines the workspace with binary + all crates
- [ ] Remove any remaining game-specific code from engine crates
- [ ] Clean up re-exports in crate lib.rs files
- [ ] Have rust4d_game re-export commonly needed types from core/physics/math
- [ ] Update engine README to describe it as a Lua-scriptable 4D game engine
- [ ] Update CLAUDE.md for the new architecture
- [ ] Verify engine has no game-specific assumptions left
- [ ] Run full test suite
- [ ] Update/create examples:
  - `examples/minimal_lua/` -- simplest possible Lua game (spawn a cube, rotate it)
  - `examples/physics_lua/` -- Lua-driven physics demo
  - `examples/ecs_rust/` -- Rust-only ECS example (for engine developers)
- [ ] Write basic Lua API documentation (which functions are available, lifecycle, etc.)

**Verification:**
- Engine builds and passes all tests
- Engine binary runs example Lua games correctly
- Game repo runs against the engine
- No game-specific code remains in engine crates
- Examples demonstrate the Lua scripting API

## Session Estimates

| Phase | Sessions | Notes |
|-------|----------|-------|
| Phase 1: ECS Migration | 4-6 | Largest phase. 202 core tests + render/physics integration to port. Same as original plan. |
| Phase 2: Game Logic Extraction + rust4d_game | 3-4 | Create new crate, extract player logic, implement CharacterController4D + events + FSM. Also now includes scene helpers (merged from old Phase 3). |
| Phase 3: Lua Scripting Integration | 4-6 | NEW. mlua integration, all bindings, script lifecycle, hot-reload. Binding breadth is the main effort -- each engine type needs UserData impl + Lua tests. |
| Phase 4: Engine Binary / Launcher | 2-3 | NEW. Rewrite src/main.rs as launcher, game loop with script calls, config loading from game dir. |
| Phase 5: Game Repo Setup | 1-2 | Simpler than old Phase 4. Just Lua scripts + data, no Cargo project to set up. Porting logic from Rust to Lua is straightforward since it maps to binding calls. |
| Phase 6: Engine Cleanup | 0.5-1 | Removing dead code, updating docs/config. Same as original Phase 5. |
| **Total** | **14.5-22** | Up from 9.5-14. The Lua scripting phase adds ~6-9 sessions but simplifies the game repo phase. |

### Parallelism

Phases 1-2 are sequential (each depends on the previous). Phase 3 depends on Phase 2 (needs rust4d_game types to wrap). Phase 4 depends on Phase 3 (needs ScriptEngine). Phase 5 depends on Phase 4 (needs running launcher). Phase 6 can overlap with Phase 5.

Within Phase 3, bindings for different modules (math, physics, ECS, input, scene) are largely independent and could be done by parallel agents.

```
Phase 1 (ECS Migration)
  -> Phase 2 (Game Logic Extraction + rust4d_game + Scene Helpers)
    -> Phase 3 (Lua Scripting Integration)    [largest new phase]
       Internal parallelism:
         Agent A: math + physics bindings
         Agent B: ECS + scene bindings
         Agent C: input + game bindings
         Agent D: lifecycle + hot-reload
      -> Phase 4 (Engine Binary / Launcher)
        -> Phase 5 (Game Repo Setup) + Phase 6 (Engine Cleanup) [parallel]
```

## Resolved Decisions

1. **Lua 5.4 via mlua**: mlua is actively maintained, has excellent UserData support, and Lua 5.4 provides integers and a mature standard library. LuaJIT can be evaluated later if performance becomes an issue.

2. **Game directory model**: Engine binary loads a game directory (containing main.lua, scripts/, scenes/, assets/). No Cargo.toml in the game repo. Game IS the directory.

3. **Script lifecycle via callbacks**: Scripts register `on_init`, `on_update`, `on_fixed_update`, `on_event`, `on_reload`. Engine calls these at the right time. No magic -- explicit registration.

4. **Hot-reload via file watcher**: `notify` crate watches game directory. Changed modules are re-required. `persist` table survives reload. Errors during reload keep the old module.

5. **rust4d_game role**: Provides Rust implementations (CharacterController4D, EventBus, FSM) that get Lua UserData wrappers. The heavy lifting stays in Rust; Lua calls into it.

6. **Script-defined components**: Lua scripts define game components (Health, Weapon, EnemyAI) as tables. Stored in hecs via a generic `ScriptComponent` wrapper. Engine does not need to know about game-specific component types.

7. **Collision layer presets stay in engine**: `CollisionFilter::player()`, etc. are convenience presets. Exposed to Lua as `CollisionFilter.player()`.

8. **rust4d_input**: Stays in engine. Refactored to action/axis abstraction. Lua queries actions, not raw key codes. InputMap is loaded from game config.toml.

9. **Old Phase 3 (Pluggable Scenes) merged into Phase 2**: Scene helpers move to `rust4d_game` during the game logic extraction phase since it is a natural fit.

## Risks / Considerations

1. **hecs API differences from current World**: Same risk as original plan. hecs uses query iterators, not SlotMap get/set. The hierarchy system needs reimplementation as components. This is the riskiest part of the ECS migration.

2. **Test porting effort**: Same as original plan. 202 core tests + 97 physics tests + integration tests all assume the current Entity/World API.

3. **Lua binding surface area**: The biggest new risk. Every engine type that game code needs must have a UserData implementation with methods, metamethods, and documentation. The binding layer is significant engineering effort and must be maintained in sync with the Rust API. Estimate ~50-80 functions/methods to bind across all crates.

4. **Lua performance for game logic**: Game logic (AI tick, event dispatch, component queries) runs in Lua. For a boomer shooter with moderate enemy counts (<50 active enemies), this should be fine. Profile if frame time budget is exceeded. Mitigation: move hot inner loops to Rust if needed (e.g., spatial queries stay in Rust, only the "what to do with results" is Lua).

5. **Debugging Lua scripts**: Lua errors are harder to debug than Rust compiler errors. Mitigate with:
   - Detailed error messages including file, line, and stack trace
   - `rust4d.log()` / `rust4d.warn()` / `rust4d.error()` from Lua
   - Optional Lua debugger integration (mobdebug or similar) as a future enhancement
   - Hot-reload means fast iteration even without a debugger

6. **ECS bridge complexity**: Exposing hecs queries to Lua is non-trivial. Lua does not have Rust's type system, so component queries must be string-based (`world:query("Transform4D", "Health")`). The bridge code needs to dynamically look up component types and return Lua-friendly values. This is solvable but requires careful design.

7. **CharacterController4D fidelity**: Same risk as original plan. The extraction from PhysicsWorld to CharacterController4D must reproduce identical behavior. Now also needs to work correctly when called from Lua (no subtle differences from the Rust-side behavior).

8. **Hot-reload edge cases**: Module dependency tracking, global state management, and Lua closure captures can make hot-reload tricky. Start simple (reload entire changed file, preserve `persist` table) and add granularity later.

9. **Two-repo coordination is simpler**: A major upside of the Lua approach is that the game repo has no compile-time dependency on the engine. Breaking engine API changes require updating Lua scripts, but there is no Cargo version resolution, no git URL syncing, and no cross-repo compilation. The game just needs a compatible engine binary.

## What Post-Split Phases Become Possible

After the split is complete (Phases 1-6), post-split engine feature development continues as before. The difference is that game-side work for each phase is written in Lua instead of Rust:

| Post-Split Phase | Engine Work (Rust) | Game Work (Lua) | Depends On |
|------------------|-------------------|-----------------|------------|
| Combat Core (P1) | Raycasting in rust4d_physics, collision events, trigger fix + **Lua bindings for new APIs** | Health, damage, weapons **in Lua** | Split Phase 3 (Lua bindings) |
| Weapons & Feedback (P2) | rust4d_audio crate, particle system, egui overlay + **Lua bindings** | Weapon types, HUD widgets, screen shake **in Lua** | Split Phase 3 |
| Enemies & AI (P3) | Sprite billboard pipeline, spatial queries, FSM + **Lua bindings** | Enemy types, AI behaviors, spawn logic **in Lua** | Split Phase 3 |
| Level Design (P4) | Shape types, RON preview tool, tween system + **Lua bindings** | Door/elevator logic, pickups, level scripts **in Lua** | Split Phase 3 |
| Editor & Polish (P5) | rust4d_editor crate, point lights, textures + **Lua bindings** | Game-specific editor panels, menus **in Lua** | Split Phase 3 |

Note: Each post-split engine phase now includes a binding task -- when new Rust APIs are added, they need Lua wrappers. This is ongoing maintenance but typically 0.5-1 session per phase.

The engine/game split with Lua scripting is the critical path that gates all gameplay feature development. The Lua approach adds upfront effort (Phase 3) but dramatically accelerates game-side iteration by eliminating recompilation.
