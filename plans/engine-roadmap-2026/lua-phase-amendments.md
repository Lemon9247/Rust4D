# Lua Migration: Phase Amendments (P1-P5)

**Date**: 2026-01-31
**Author**: Agent Amendments (Lua Migration Swarm)
**Purpose**: Document how each post-split engine phase changes with the shift from "game in Rust" to "game in Lua scripts"

---

## Context

The architectural shift: the game is no longer a compiled Rust binary depending on engine crates. Instead, the engine provides a Lua-scriptable runtime (via `rust4d_scripting` crate using mlua). The game repo contains Lua scripts + RON scenes + TOML config + assets. Hot-reload enables edit-save-test workflow without recompilation.

**Core principle**: The engine's internal Rust implementation does not change. What changes is the **surface area the engine exposes** -- everything that was "game implements in Rust" now needs Lua bindings so scripts can call it. Some game-framework types in `rust4d_game` may become unnecessary if Lua provides equivalent capability natively (tables, coroutines).

---

## Phase 1: Combat Core -- Amendments

**Original engine estimate**: 1.75 sessions
**Amended engine estimate**: 2.25-2.5 sessions (+0.5-0.75 for Lua bindings)

### What stays the same

All internal Rust implementation is unchanged:

- `Ray4D` geometric primitive in `rust4d_math`
- Ray-shape intersection functions (`ray_vs_sphere`, `ray_vs_aabb`, `ray_vs_plane`, `ray_vs_collider`)
- `PhysicsWorld::raycast()` and `raycast_nearest()` with layer mask filtering
- `CollisionEvent` and `CollisionEventKind` data structs
- `PhysicsWorld::drain_collision_events()` poll API
- Asymmetric trigger overlap detection (bug fix)
- Trigger enter/stay/exit tracking with `active_triggers: HashSet`
- `Vec4` utility additions (`distance()`, `distance_squared()`, `f32 * Vec4`)
- `sphere_vs_sphere` visibility fix
- `WorldRayHit`, `RayTarget`, `RayHit` types

### Engine vs Game boundary shift

**Previously game-side Rust, now needs Lua bindings:**

| Was (Rust game code) | Becomes (Engine Lua binding) |
|---------------------|------------------------------|
| Game calls `physics_world.raycast()` in Rust | `world:raycast(origin, direction, max_dist, layer_mask)` returns Lua table of hits |
| Game calls `physics_world.raycast_nearest()` in Rust | `world:raycast_nearest(origin, dir, max_dist, mask)` returns hit or nil |
| Game calls `physics_world.drain_collision_events()` in Rust | Engine dispatches events to registered Lua callbacks (see below) |
| Game reads `WorldRayHit.hit.distance`, `.hit.point`, `.hit.normal`, `.target` | Lua hit table: `{ distance=N, point=vec4, normal=vec4, target={type="body", key=K} }` |
| Game reads `CollisionEvent.kind` variants | Lua event tables: `{ type="body_vs_body", body_a=K, body_b=K, contact={...} }` |
| Game iterates `CollisionLayer` bitflags for filtering | `CollisionLayer` exposed as Lua constants: `LAYER.PLAYER`, `LAYER.ENEMY`, `LAYER.STATIC`, etc. |
| `EventBus` in `rust4d_game::events` dispatches typed Rust events | Engine provides Lua callback registration: `on_collision(function(event) ... end)` |

### New engine work

1. **Lua wrappers for raycasting** (~0.25 session):
   - Bind `PhysicsWorld::raycast()` as `world:raycast(origin, direction, max_dist, layer_mask)` returning a Lua table array of hit results
   - Bind `PhysicsWorld::raycast_nearest()` as `world:raycast_nearest(...)` returning a single hit table or `nil`
   - `RayHit` and `WorldRayHit` converted to Lua tables on return (not userdata -- scripts need to read fields freely)
   - `CollisionLayer` constants exposed to Lua scope

2. **Lua collision event dispatch** (~0.25 session):
   - Engine calls `drain_collision_events()` each frame internally
   - Engine dispatches to registered Lua callbacks: `on_collision(callback)`, `on_trigger_enter(callback)`, `on_trigger_stay(callback)`, `on_trigger_exit(callback)`
   - Each callback receives an event table with relevant fields
   - Alternative design: single `on_physics_event(callback)` with event type discrimination in Lua

3. **Lua trigger callbacks** (~0.25 session):
   - Triggers can invoke Lua functions directly instead of `GameEvent(String)`
   - `TriggerAction::LuaCallback(String)` -- calls a named Lua function when trigger fires
   - This is far more powerful than the string escape hatch -- any trigger action can be arbitrary Lua code

### What gets simpler

- **Trigger system**: `GameEvent(String)` was a workaround for not having a scripting language. With Lua, triggers call Lua functions directly. The string-to-game-event translation layer is unnecessary. Triggers become: detect overlap -> call Lua function. Much cleaner.
- **EventBus in rust4d_game**: The general-purpose `EventBus` with typed `GameEvent` enum becomes less important. Lua scripts can register callbacks directly with the engine. A Lua-side event system (a simple pub/sub table) replaces the Rust `EventBus` for game events.

### What gets removed from engine scope

- **`rust4d_game::events::EventBus`**: No longer needed as a Rust type for game-side consumption. The engine's collision event reporting remains (it is internal), but the game-side event dispatch is now Lua-native. If `EventBus` was only planned for game-side Rust, it can be removed entirely.
- **`GameEvent` enum**: This was the typed Rust enum the game would define (Damage, Pickup, TriggerEnter, etc.). With Lua, events are just strings or tables -- no Rust enum needed.
- **Health/damage types** in `rust4d_game`: These were always game-side, and with Lua they remain game-side but in Lua. No Rust type was ever planned, so no removal -- just confirmation that the engine still provides nothing here.

### Session estimate change

| Sub-Phase | Original | Amended | Delta |
|-----------|----------|---------|-------|
| A: Raycasting | 1.0 | 1.0 | 0 (no change to Rust impl) |
| B: Collision Events & Triggers | 0.75 | 0.75 | 0 (no change to Rust impl) |
| NEW: Lua bindings for P1 APIs | -- | 0.5-0.75 | +0.5-0.75 |
| **Total** | **1.75** | **2.25-2.5** | **+0.5-0.75** |

### Verification changes

**New Lua integration tests needed:**
- Lua script calls `world:raycast()` and receives correct hit table
- Lua script calls `world:raycast_nearest()` and gets nil on miss
- `on_trigger_enter` callback fires when entity enters trigger zone
- `on_trigger_exit` callback fires when entity leaves trigger zone
- `CollisionLayer` constants accessible from Lua (`LAYER.PLAYER`, etc.)
- Lua callback receives correct event table fields (body keys, contact data)
- Multiple Lua callbacks can be registered for the same event type
- Error in Lua callback does not crash engine (logged, execution continues)

---

## Phase 2: Weapons & Feedback -- Amendments

**Original engine estimate**: 4.5-5.5 sessions
**Amended engine estimate**: 5.5-7.0 sessions (+1.0-1.5 for Lua bindings + HUD API)

### What stays the same

All internal Rust implementation is unchanged:

- `rust4d_audio` crate wrapping kira with 4D spatial audio (AudioEngine4D, Listener4D, SpatialConfig, AudioBus)
- `OverlayRenderer` egui-wgpu integration in `rust4d_render`
- `ParticleSystem`, `ParticleEmitter`, `ParticleEmitterConfig` in `rust4d_render`
- Billboard shader and blending pipelines (additive, alpha)
- `ScreenShake` struct in `rust4d_game`
- `TimedEffect` struct in `rust4d_game`
- Depth texture getter on `RenderPipeline`
- Kira threading model, bus routing, spatial audio projection

### Engine vs Game boundary shift

**Previously game-side Rust, now needs Lua bindings:**

| Was (Rust game code) | Becomes (Engine Lua binding) |
|---------------------|------------------------------|
| `audio_engine.play(&sound, AudioBus::Sfx)` | `audio:play("shotgun_fire", "sfx")` |
| `audio_engine.play_spatial(&sound, pos, bus, spatial)` | `audio:play_spatial("explosion", position, "sfx", { min_dist=1, max_dist=50 })` |
| `audio_engine.play_oneshot(&sound, bus)` | `audio:play_oneshot("pickup", "sfx")` |
| `audio_engine.play_oneshot_spatial(&sound, pos, bus, spatial)` | `audio:play_oneshot_spatial("bullet_impact", pos, "sfx")` |
| `audio_engine.set_bus_volume(AudioBus::Music, 0.5)` | `audio:set_volume("music", 0.5)` |
| `audio_engine.stop_bus(AudioBus::Sfx)` | `audio:stop("sfx")` |
| `particle_system.spawn_burst(pos, &config)` | `particles:burst(position, "muzzle_flash")` with named presets |
| `particle_system.spawn_emitter(pos, &config)` | `particles:emit(position, config_table)` |
| `particle_system.stop_emitter(id)` | `particles:stop(emitter_id)` |
| `screen_shake.add_trauma(0.5)` | `screen_shake(0.5)` or `effects:screen_shake(intensity, duration)` |
| Game builds HUD via `overlay.ctx()` (egui Context) | **New HUD API needed** (see below) |
| `overlay.begin_frame() / render()` cycle | Engine manages internally, Lua submits draw commands |

### New engine work

1. **Lua audio API wrappers** (~0.25 session):
   - `audio:load(name, path)` -- load and register a sound by name
   - `audio:play(name, bus)` -- play non-spatial
   - `audio:play_spatial(name, position, bus, config?)` -- play spatial at 4D position
   - `audio:play_oneshot(name, bus)` / `audio:play_oneshot_spatial(name, pos, bus)`
   - `audio:set_volume(bus, volume)` -- set bus volume
   - `audio:stop(bus?)` -- stop sounds on a bus or all
   - Sound names as string keys (Lua-friendly), engine maintains name-to-handle mapping

2. **Lua particle API wrappers** (~0.25 session):
   - `particles:define(name, config_table)` -- define a named particle preset from a Lua table
   - `particles:burst(position, preset_name)` -- one-shot burst effect
   - `particles:emit(position, preset_name)` -- start continuous emitter, returns emitter ID
   - `particles:stop(emitter_id)` -- stop an emitter
   - `particles:kill(emitter_id)` -- immediately remove
   - `particles:move(emitter_id, position)` -- update emitter position
   - `ParticleEmitterConfig` constructable from Lua table: `{ max_particles=20, lifetime=0.1, ... }`

3. **Lua screen effects API** (~0.1 session):
   - `effects:screen_shake(intensity, duration?)` -- add screen shake trauma
   - `effects:flash(r, g, b, a, duration)` -- full-screen color flash (replaces egui damage flash)

4. **HUD rendering API for Lua** (~0.5-1.0 session) -- **this is new work**:
   - egui's immediate-mode Rust API does not translate directly to Lua. Exposing the full egui `Context` to Lua would be extremely complex and fragile.
   - **Recommended approach**: Engine provides a simplified HUD drawing API:
     ```
     hud:draw_text(x, y, text, options?)       -- options: { color, size, font, anchor }
     hud:draw_bar(x, y, w, h, fill, options?)   -- health/ammo bar, options: { color, bg_color }
     hud:draw_rect(x, y, w, h, options?)         -- filled rectangle (for damage flash, etc.)
     hud:draw_image(x, y, w, h, image_name)      -- sprite/image on screen
     hud:draw_crosshair(style?)                   -- built-in crosshair styles
     ```
   - Engine translates these Lua draw commands into egui calls internally each frame
   - This is a new abstraction layer that did not exist in the pure-Rust approach
   - **Open question**: Should there be a retained-mode HUD (define layout once, update values) or immediate-mode (redraw every frame from Lua)? Immediate-mode is simpler and matches egui's model. Performance should be fine for HUD-level complexity.
   - **Alternative**: Expose egui directly to Lua via mlua userdata. This is more powerful but much more work to bind and maintain. Recommend starting with the simplified API and adding egui passthrough later if needed.

### What gets simpler

- **Sound triggering**: In the Rust approach, the game had to maintain a `GameAudio` struct with loaded sound handles and match on event types. In Lua, it is just `audio:play_oneshot("shotgun_fire", "sfx")` -- one line. No struct, no handle management.
- **Particle effect presets**: In Rust, these were `ParticleEmitterConfig` structs defined in compiled code. In Lua, they are data tables that can be defined in config files and hot-reloaded. Easier to tweak without recompilation.
- **Screen shake**: `effects:screen_shake(0.5)` is simpler than creating a `ScreenShake` struct and calling methods on it.

### What gets removed from engine scope

- **Game-side `GameAudio` struct pattern**: The Rust example showed a struct holding `AudioEngine4D` + named `SoundHandle` fields. With Lua, the engine manages handles by string name internally. No game-side Rust struct needed.
- **Game-side `GameHud` struct**: The egui-based HUD drawing shown in the P2 plan was Rust code. This is now Lua code using the HUD API. The Rust `GameHud` struct is gone.
- **`ScreenShake` and `TimedEffect` in `rust4d_game`**: These might still exist as Rust structs internally (engine uses them), but they need Lua-facing wrappers. The Rust types remain; the game-side Rust usage pattern is replaced by Lua calls.

### Session estimate change

| Sub-Phase | Original | Amended | Delta |
|-----------|----------|---------|-------|
| A: Audio System | 1.5-2 | 1.5-2 | 0 (Rust impl unchanged) |
| B: HUD/egui Overlay | 1 | 1 | 0 (Rust impl unchanged) |
| C: Particle System | 1.5-2 | 1.5-2 | 0 (Rust impl unchanged) |
| D: Screen Effects | 0.5 | 0.5 | 0 (Rust impl unchanged) |
| NEW: Lua audio bindings | -- | 0.25 | +0.25 |
| NEW: Lua particle bindings | -- | 0.25 | +0.25 |
| NEW: Lua screen effects bindings | -- | 0.1 | +0.1 |
| NEW: HUD drawing API for Lua | -- | 0.5-1.0 | +0.5-1.0 |
| **Total** | **4.5-5.5** | **5.6-7.1** | **+1.1-1.6** |

The HUD API is the significant new item. The other bindings are thin wrappers.

### Verification changes

**New Lua integration tests needed:**
- Lua script loads and plays a sound by name
- Lua script plays spatial audio at a 4D position
- Lua script spawns particle burst at a position
- Lua script creates and controls a continuous emitter
- Lua script triggers screen shake
- Lua `hud:draw_text()` renders visible text at correct position
- Lua `hud:draw_bar()` renders a bar with correct fill percentage
- HUD draws correctly on top of 3D scene
- Invalid sound/preset names produce Lua errors (not crashes)
- Hot-reload of Lua script updates HUD layout without restart

---

## Phase 3: Enemies & AI -- Amendments

**Original engine estimate**: 4.0 sessions
**Amended engine estimate**: 3.75-4.5 sessions (+0.5 for Lua bindings, -0.75 if FSM removed)

### What stays the same

All internal Rust rendering and physics implementation is unchanged:

- `SpriteBatch`, `SpritePipeline` in `rust4d_render` -- billboard rendering at 4D positions with W-fade
- `SpriteAnimation` frame-based animation ticker
- `ParticleSystem` / `ParticleEmitter` (shared with P2)
- `query_sphere()`, `query_sphere_sorted()` in `rust4d_physics`
- `query_area_effect()` and `AreaEffectHit` in `rust4d_physics`
- `line_of_sight()` in `rust4d_physics`
- `apply_impulse()` in `rust4d_physics`
- Billboard WGSL shader, depth buffer sharing
- W-distance fade calculation for sprites
- Render pass ordering (sprites after geometry, before particles)

### Engine vs Game boundary shift

**Previously game-side Rust, now needs Lua bindings:**

| Was (Rust game code) | Becomes (Engine Lua binding) |
|---------------------|------------------------------|
| `sprite_batch.add_sprite_4d(pos, slice_w, cam, size, frame, tint, fade)` | `sprites:add(pos4d, { size={w,h}, frame=N, tint={r,g,b,a} })` |
| `sprite_animation.update(dt)` / `.current_frame()` | `anim:update(dt)` / `anim:frame()` -- or engine auto-updates based on component |
| `SpriteAnimation::new(frames, fps, looping)` | `animation.new({ frames={0,1,2,3}, fps=8, loop=true })` |
| `world.query_sphere(center, radius, layer_filter)` | `world:query_sphere(center, radius, layer?)` returns Lua table |
| `world.query_sphere_sorted(center, radius, layer_filter)` | `world:query_sphere_sorted(center, radius, layer?)` |
| `world.query_area_effect(center, radius, layer, require_los)` | `world:area_effect(center, radius, layer?, los?)` returns hits with falloff |
| `world.line_of_sight(from, to, block_layers)` | `world:line_of_sight(from, to, block_layers?)` returns boolean |
| `world.apply_impulse(key, impulse)` | `world:impulse(entity, impulse_vec4)` |
| `StateMachine<S>` generic FSM | **May not be needed** (see below) |

### New engine work

1. **Lua sprite API wrappers** (~0.25 session):
   - `sprites:load_sheet(name, path, cols, rows)` -- load a sprite sheet
   - `sprites:add(position_4d, config_table)` -- add a sprite to the current frame's batch
   - `sprites:animate(entity, config_table)` -- attach animation to an entity
   - Engine auto-updates sprite animations for entities with animation components
   - W-fade parameters configurable per sprite: `{ w_fade_range=3.0 }`

2. **Lua spatial query wrappers** (~0.15 session):
   - `world:query_sphere(center, radius, layer_mask?)` -- returns array of `{ entity, distance, position }`
   - `world:query_sphere_sorted(center, radius, layer_mask?)` -- same, sorted by distance
   - `world:area_effect(center, radius, layer_mask?, require_los?)` -- returns array of `{ entity, distance, falloff, position, direction }`
   - `world:line_of_sight(from, to, block_layers?)` -- returns boolean
   - `world:impulse(entity, impulse_vec4)` -- apply velocity impulse

3. **Lua animation control** (~0.1 session):
   - `animation.new(config)` -- create animation state
   - `anim:update(dt)` / `anim:frame()` / `anim:reset()` / `anim:finished()`
   - Or: engine manages animations as components, Lua just sets which animation is active: `entity:set_animation("walk")`

### What gets simpler

- **FSM framework (`StateMachine<S>`)**: This was ~30 lines of Rust in `rust4d_game`. With Lua, FSMs are trivially implemented as tables and functions:
  ```lua
  local enemy = {
    state = "idle",
    time_in_state = 0,

    update = function(self, dt)
      self.time_in_state = self.time_in_state + dt
      if self.state == "idle" then self:update_idle(dt)
      elseif self.state == "chase" then self:update_chase(dt)
      -- ...
      end
    end,

    transition = function(self, new_state)
      self.state = new_state
      self.time_in_state = 0
    end,
  }
  ```
  Lua coroutines could also model AI states elegantly (each state is a coroutine that yields when transitioning). **The Rust `StateMachine<S>` becomes optional/unnecessary.**

- **AI state logic**: In Rust, AI required defining an `EnemyState` enum, implementing match arms, managing state transitions through the FSM API. In Lua, it is just tables and functions -- more flexible, hot-reloadable, and faster to iterate on.

- **Enemy definitions**: `EnemyDef` data (health, speed, sprite sheets) was a Rust struct. In Lua, it is a simple data table loaded from a file -- trivially hot-reloadable.

### What gets removed from engine scope

- **`StateMachine<S>` in `rust4d_game`**: Can be removed entirely. Lua provides native FSM capability via tables, functions, and coroutines. No Rust FSM framework needed. If kept for non-Lua users, it becomes an optional utility, not a core dependency.
  - **Saves**: ~0.25 sessions (FSM was included in P3's Sub-Phase B/C estimate)
  - **Rationale**: The entire point of the FSM was to give Rust game code a state management pattern. With Lua, the language itself provides this.

- **`EnemyState` enum, `EnemyAI` struct, `EnemyDef` data, `WBehavior` enum**: These were always game-side, but they were designed as Rust types. Now they are Lua tables. No engine change, just confirmation.

### Session estimate change

| Sub-Phase | Original | Amended | Delta |
|-----------|----------|---------|-------|
| A: Sprite/Billboard rendering | 1.5 | 1.5 | 0 (Rust impl unchanged) |
| B: Spatial queries | 0.5 | 0.5 | 0 (Rust impl unchanged) |
| C: FSM framework | (in B) | 0 | -0.25 (removed -- Lua handles this natively) |
| D: Area damage / explosions | (in B) | (in B) | 0 |
| Particle system | 1.5 | 1.5 | 0 (Rust impl unchanged) |
| NEW: Lua sprite API bindings | -- | 0.25 | +0.25 |
| NEW: Lua spatial query bindings | -- | 0.15 | +0.15 |
| NEW: Lua animation control | -- | 0.1 | +0.1 |
| **Total** | **4.0** | **3.75-4.5** | **-0.25 to +0.5** |

Net effect is roughly neutral. The FSM removal saves a small amount, and the Lua bindings add a small amount. Sprite and spatial query bindings are thin wrappers over existing Rust APIs.

### Verification changes

**New Lua integration tests needed:**
- Lua script loads a sprite sheet and adds sprites to the batch
- Lua script queries `world:query_sphere()` and gets correct nearby entities
- Lua script checks `world:line_of_sight()` and gets correct boolean result
- Lua script calls `world:area_effect()` and receives falloff values
- Lua script calls `world:impulse()` and body velocity changes
- Lua script creates and drives a sprite animation
- Lua FSM pattern works (state transitions, time tracking -- this is pure Lua, but good to have example/test)
- W-fade correctly applied to Lua-spawned sprites

---

## Phase 4: Level Design Pipeline -- Amendments

**Original engine estimate**: 4.5 sessions
**Amended engine estimate**: 4.25-5.0 sessions (+0.25-0.5 for Lua trigger integration, -0.5 for simplified triggers)

### What stays the same

All internal Rust implementation is unchanged:

- `Hyperprism4D` and `Hypersphere4D` shape types in `rust4d_math`
- `ShapeTemplate` variants in `rust4d_core`
- Vertex generation, tetrahedra decomposition, `ConvexShape4D` trait implementations
- RON preview tool (`examples/ron_preview.rs`) with hot-reload
- `Interpolatable` trait in `rust4d_math` (lerp for `f32`, `Vec4`, `Transform4D`)
- `EasingFunction` enum and application logic
- `Tween<T>` struct and `TweenManager`
- `TriggerDef`, `TriggerZone`, `TriggerRepeat` data types in `rust4d_core`
- Scene integration (triggers field in `Scene`)
- Scene validation of trigger references

### Engine vs Game boundary shift

**Previously game-side Rust, now needs Lua bindings:**

| Was (Rust game code) | Becomes (Engine Lua binding) |
|---------------------|------------------------------|
| Game creates `TweenManager` and calls `tween_position()` | `tween:position(entity, target, duration, easing?)` |
| Game reads `GameEvent(String)` from trigger actions | Trigger calls Lua function directly |
| Game implements event handlers for trigger strings | Lua defines trigger callback functions |
| Door/elevator code calls `TweenManager` in Rust | Lua calls `tween:position()` with callbacks |

### New engine work

1. **Lua tween API** (~0.15 session):
   - `tween:position(entity, target_vec4, duration, easing?)` -- start a position tween
   - `tween:on_complete(tween_id, callback)` -- call Lua function when tween finishes
   - Easing functions as strings: `"linear"`, `"ease_in_quad"`, `"ease_out_cubic"`, etc.
   - `tween:pause(id)` / `tween:resume(id)` / `tween:cancel(id)`

2. **Lua trigger callback system** (~0.25-0.5 session) -- **this is the big change**:
   - `TriggerAction::GameEvent(String)` becomes `TriggerAction::LuaCallback(String)` -- calls a named Lua function
   - Or better: `TriggerAction::Callback(String)` where the string is a Lua function name or inline expression
   - Example RON:
     ```ron
     TriggerDef(
       name: "health_pickup",
       zone: Sphere(center: (10.0, 1.0, 3.0, 0.0), radius: 1.0),
       detects: [Player],
       actions: [
         Callback("on_health_pickup"),  // calls Lua function on_health_pickup(trigger, entity)
         DespawnSelf,
       ],
       repeat: Once,
     )
     ```
   - Engine's `TriggerRuntime` calls into Lua when processing `Callback` actions
   - This is **significantly more powerful** than `GameEvent(String)`: instead of the game needing to match on string event names and dispatch, the trigger directly invokes arbitrary game logic
   - `TriggerAction::PlaySound` still exists (engine-level action)
   - `TriggerAction::TweenPosition` still exists (engine-level action)
   - `TriggerAction::GameEvent(String)` can be kept for backward compatibility or removed in favor of `Callback`

3. **Lua trigger registration API** (~0.1 session):
   - `triggers:register(name, callback_fn)` -- register a Lua function as a trigger callback
   - `triggers:on_enter(trigger_name, callback_fn)` -- register callback for specific trigger
   - `triggers:on_exit(trigger_name, callback_fn)` -- register exit callback

### What gets simpler

- **Trigger system is dramatically simpler and more powerful**: The `GameEvent(String)` escape hatch was designed because Rust game code needed some way to handle arbitrary trigger actions without the engine knowing game-specific types. It was a string-based dispatch table -- awkward in Rust. With Lua, triggers just call Lua functions. The entire `GameEvent` string interpretation layer disappears. A trigger can do anything: heal the player, spawn enemies, play a cutscene, change the W-slice -- all in a few lines of Lua.

- **Door/elevator mechanics**: In the Rust approach, the game needed Rust structs (`Door`, `Elevator`, `DoorState`) with FSM logic. With Lua, a door is:
  ```lua
  function on_door_trigger(trigger, entity)
    tween:position(door_entity, open_position, 1.5, "ease_in_out_quad")
    audio:play_spatial("door_open", door_position, "sfx")
  end
  ```
  This is a few lines of Lua vs a significant Rust module.

- **Pickup system**: Similarly trivial in Lua -- a trigger callback that modifies player state.

### What gets removed from engine scope

- **`TriggerAction::GameEvent(String)` pattern**: Can be replaced entirely by `TriggerAction::Callback(String)`. The string-to-game-event translation layer in `rust4d_game` is no longer needed. If both are kept for flexibility, `GameEvent` becomes a legacy option.

- **Game-side event handler pattern**: The entire pattern of "receive `GameEvent(String)`, match on it, dispatch to handler function" is replaced by direct Lua callbacks. The `rust4d_game` code that was planned to interpret game event strings is unnecessary.

### Session estimate change

| Sub-Phase | Original | Amended | Delta |
|-----------|----------|---------|-------|
| A: Shape types (Hyperprism, Hypersphere) | 1.0 | 1.0 | 0 |
| B: RON preview tool | 2.0 | 2.0 | 0 |
| C: Tween/interpolation system | 0.5 | 0.5 | 0 (Rust impl unchanged) |
| D: Trigger data model (0.5) + runtime (0.5) | 1.0 | 0.75 | -0.25 (simpler: no GameEvent dispatch) |
| NEW: Lua tween bindings | -- | 0.15 | +0.15 |
| NEW: Lua trigger callback integration | -- | 0.25-0.5 | +0.25-0.5 |
| NEW: Lua trigger registration API | -- | 0.1 | +0.1 |
| **Total** | **4.5** | **4.75-5.0** | **+0.25-0.5** |

The trigger runtime is slightly simpler (no string event dispatch needed) but the Lua callback integration adds new work. Net is roughly neutral.

### Verification changes

**New Lua integration tests needed:**
- Lua script starts a position tween and verifies entity moves
- Lua script registers a trigger callback and it fires on trigger enter
- Lua trigger callback can access the triggering entity
- `TriggerAction::Callback("my_func")` correctly invokes `my_func` in Lua
- Tween completion callback fires in Lua
- Lua script can create tweens with all easing function types
- Error in Lua trigger callback does not crash engine
- RON scene with `Callback` trigger action loads and validates correctly

---

## Phase 5: Editor & Polish -- Amendments

**Original engine estimate**: 10-12.5 sessions
**Amended engine estimate**: 11.5-15 sessions (+1.5-2.5 for Lua script editing, console, input API)

### What stays the same

All internal Rust implementation is unchanged:

- Triplanar texture mapping and `TextureManager` in `rust4d_render`
- UV path through pipeline (if implemented)
- `PointLight4D` component and GPU light system
- W-distance attenuation for lights
- Directional shadow mapping (`ShadowPipeline`, shadow map, PCF)
- Light collection system from ECS
- `rust4d_editor` crate structure (EditorApp, EditorHost, panels)
- Entity list panel, property inspector, W-slice navigation
- Scene save/load to RON
- Undo/redo command pattern
- All shader code (render.wgsl point lights, triplanar, shadows)
- `egui_dock` for dockable panel layout
- Render pass ordering (editor overlay always last)

### Engine vs Game boundary shift

**Previously game-side Rust, now needs Lua bindings or new editor features:**

| Was (Rust game code) | Becomes (Engine Lua binding / Editor feature) |
|---------------------|----------------------------------------------|
| `InputMap::rebind()` called from Rust settings screen | `input:rebind(action, key)` from Lua settings script |
| `InputMap::to_toml()` / `from_toml()` called from Rust | `input:save(path)` / `input:load(path)` from Lua |
| `InputMap::conflicts()` checked in Rust | `input:conflicts()` returns table of conflicts to Lua |
| Game-specific editor panels in Rust | **Not needed** -- game config is Lua data files, editable as text |
| No script editing needed | **Editor needs script panel** (new feature) |
| No runtime console needed | **Editor needs Lua console** (new feature) |

### New engine work

1. **Lua input rebinding API** (~0.15 session):
   - `input:bind(action_name, key_name)` -- bind an action to a physical input
   - `input:unbind(action_name)` -- remove a binding
   - `input:conflicts()` -- returns table of conflicting bindings
   - `input:save(path?)` -- save input map to TOML file
   - `input:load(path?)` -- load input map from TOML file
   - `input:reset()` -- reset to defaults
   - `input:get_binding(action_name)` -- get current binding for an action
   - Lua can define custom input maps: `input:define_action("custom_action")` -- useful for game-specific actions

2. **Editor: Script editing panel** (~1.0-1.5 sessions) -- **new feature**:
   - Text editor panel in the editor for viewing and editing Lua scripts
   - Syntax highlighting for Lua (egui text editor with highlighting, or integrate a simple highlighter)
   - File tree showing the game's script directory
   - Save button (writes to disk)
   - Error display: when a Lua script has an error, show the error message with line number in the panel
   - **Hot-reload button**: Trigger script reload from the editor (or automatic on save)
   - This is the biggest new editor feature for the Lua approach. Without it, the developer must switch to an external text editor for scripts.
   - **Alternative**: Rely on external editors (VS Code) and just show error output in the editor. This reduces scope to ~0.25 sessions (just an error log panel). Recommend starting with the error log and adding the text editor later if needed.

3. **Editor: Lua console panel** (~0.5-1.0 session) -- **new feature**:
   - REPL-style console in the editor for runtime Lua evaluation
   - Type Lua expressions, see results immediately
   - Access to all engine APIs (inspect entities, modify properties, test functions)
   - Command history (up/down arrow)
   - Auto-complete for common API names (optional, adds complexity)
   - This is extremely valuable for debugging and iteration -- modify game state at runtime without editing files
   - **Minimal version** (~0.25 session): Text input + output log, `eval(lua_string)` execution, no auto-complete

4. **Lua texture/material API** (~0.1 session):
   - `textures:load(name, path)` -- load a texture
   - `entity:set_material({ texture="stone_wall", color={0.8, 0.8, 0.8, 1.0} })` -- set material on entity
   - Minimal -- mostly the TextureManager already handles this, just needs Lua surface

5. **Lua lighting API** (~0.1 session):
   - `entity:add_light({ color={1,0.9,0.7}, intensity=2.0, range=15, w_range=3 })` -- add point light component
   - `entity:set_light({ intensity=3.0 })` -- modify light properties
   - Light is already an ECS component; Lua just needs to add/modify it

### What gets simpler

- **Input rebinding UI**: In the Rust approach, the game needed to build a full settings screen in Rust (complex UI code). With Lua + the HUD API, a simple Lua script can build a rebinding screen. Even simpler: the game could use a TOML config file for bindings and not have an in-game UI at all (common for indie games).

- **Game-specific editor panels**: The Rust approach planned for games to extend the editor with custom panels (weapon tuning, AI config). With Lua, game configuration is data files (Lua tables or TOML). The editor's script editing panel + Lua console replaces the need for bespoke property editors. Tweaking enemy health is just editing a Lua file, not building a custom editor panel.

- **Pause menu**: Was planned as game-side Rust using egui. With Lua + HUD API, it is a simple Lua script.

### What gets removed from engine scope

- **Game-side editor extension API**: The `EditorHost` trait's purpose of letting games add custom panels becomes less important. Games customize through Lua scripts, not Rust editor extensions. The `EditorHost` trait still exists for engine-level editor integration, but game-specific panels are unnecessary.

- **Complex input rebinding UI infrastructure**: The engine just provides the API (`input:rebind`, `input:save`, etc.). The game builds whatever UI it wants in Lua. No need for the engine to provide a polished rebinding widget.

### Session estimate change

| Sub-Phase | Original | Amended | Delta |
|-----------|----------|---------|-------|
| A: Texture support | 1.5-2.5 | 1.5-2.5 | 0 |
| B: Lighting system | 2 | 2 | 0 |
| C: Input rebinding | 0.5 | 0.5 | 0 (Rust impl unchanged, just add Lua wrappers) |
| D: Editor framework | 6-8 | 6-8 | 0 (Rust impl unchanged) |
| NEW: Lua input bindings | -- | 0.15 | +0.15 |
| NEW: Editor script panel (minimal: error log) | -- | 0.25-1.5 | +0.25-1.5 |
| NEW: Editor Lua console (minimal) | -- | 0.25-1.0 | +0.25-1.0 |
| NEW: Lua texture/material/light bindings | -- | 0.2 | +0.2 |
| **Total** | **10-12.5** | **11.35-15.35** | **+1.35-2.85** |

The range is wide because the script editing panel and Lua console can be scoped from minimal (error log + basic eval) to full-featured (syntax highlighting editor + auto-completing REPL). **Recommended minimal approach: error log panel + basic Lua console = +0.5-0.75 sessions**. Full-featured versions are Phase 6 territory.

With the minimal approach: **10.85-13.75 sessions** (+0.85-1.25 over original).

### Verification changes

**New Lua integration tests needed:**
- Lua script calls `input:bind("jump", "space")` and binding takes effect
- Lua script calls `input:save()` and TOML file is written
- Lua script calls `input:load()` and bindings are restored
- Lua console evaluates expressions and returns results
- Lua console can inspect entity properties at runtime
- Script error in Lua displays in editor error panel with line number
- Lua script can add and modify `PointLight4D` via API
- Lua script can load textures and assign to materials
- Hot-reload of Lua scripts reflects in editor immediately

---

## Cross-Phase Summary

### Total Session Estimate Comparison

| Phase | Original Estimate | Amended Estimate | Delta |
|-------|------------------|------------------|-------|
| P1: Combat Core | 1.75 | 2.25-2.5 | +0.5-0.75 |
| P2: Weapons & Feedback | 4.5-5.5 | 5.6-7.1 | +1.1-1.6 |
| P3: Enemies & AI | 4.0 | 3.75-4.5 | -0.25 to +0.5 |
| P4: Level Design | 4.5 | 4.75-5.0 | +0.25-0.5 |
| P5: Editor & Polish | 10-12.5 | 10.85-15.35 | +0.85-2.85 |
| **Total (P1-P5)** | **24.75-28.25** | **27.2-34.45** | **+2.45-6.2** |

With minimal scoping of P5 editor script features: **Total: 27.2-30.75 sessions** (+2.45-2.5).

### Key Themes Across All Phases

1. **Rust implementations are unchanged**: Every phase's internal Rust code (physics, rendering, audio, math) is identical. The 4D engine core is unaffected.

2. **Lua bindings add ~0.25-0.5 sessions per phase**: Each phase needs thin wrapper functions to expose Rust APIs to Lua. These are generally straightforward (convert Rust types to Lua tables, register functions on a Lua table).

3. **HUD API is the biggest new item (P2)**: egui does not translate to Lua. The engine needs a simplified drawing API. This is ~0.5-1.0 sessions of new work.

4. **FSM becomes unnecessary (P3)**: Lua's tables, functions, and coroutines natively handle state machine patterns. The `StateMachine<S>` in `rust4d_game` can be removed.

5. **Triggers become dramatically more powerful (P4)**: `GameEvent(String)` escape hatch is replaced by direct Lua function calls. The entire string-dispatch pattern disappears.

6. **Editor gains script editing (P5)**: A new panel type (script editing/error display + Lua console) is needed. Scope varies from minimal (error log, 0.5 sessions) to full-featured (syntax-highlighted editor, 1.5+ sessions).

7. **Game-side complexity moves to Lua**: Everything that was "game implements in Rust" (health systems, weapon logic, AI, door mechanics, pickup handling) is now Lua scripts. The engine's job is to expose clean APIs; the game's job is to write Lua scripts that call them.

### New Binding Maintenance Cost

Each Lua binding is a maintenance surface: when the Rust API changes, the Lua wrapper must be updated. This is an ongoing cost not captured in session estimates. Mitigation:
- Keep bindings thin (direct pass-through to Rust functions)
- Use mlua's derive macros where possible for type conversion
- Automated tests catch binding/API mismatches
- Consider generating bindings from Rust type definitions (future tooling)

### Performance Considerations

The Lua scripting layer adds per-frame overhead:
- Lua function calls for event dispatch (collision events, trigger callbacks)
- Lua-to-Rust type conversion (Vec4 to Lua table and back) on every API call
- HUD drawing commands marshaled from Lua to egui each frame

For a boomer shooter (20-50 enemies, a few hundred particles, 16 lights), this overhead is negligible. LuaJIT would make it even more negligible, but standard Lua 5.4 via mlua should be sufficient.

Critical path operations (physics step, rendering, compute shader slicing) remain in Rust and are unaffected by the Lua layer.

---

## Dependency Notes for the Scripting Phase

The amendments above assume the `rust4d_scripting` crate (described in the separate scripting phase plan) already exists and provides:
- mlua integration with Lua 5.4 runtime
- Script loading and execution lifecycle
- Hot-reload support for Lua scripts
- Error handling and reporting
- The global Lua state with engine API tables registered

Each phase's Lua bindings are registered INTO the scripting crate's Lua state. The scripting phase should be completed before or in parallel with P1-P5 Lua binding work.

**Recommended order**: Scripting phase (foundation) -> P1 bindings -> P2 bindings -> P3 bindings -> P4 bindings -> P5 bindings. Or: scripting phase first, then all binding work in parallel with Rust implementation work per phase.
