# Lua Migration Analysis: Impact on Rust4D Engine Roadmap

**Date**: 2026-01-31
**Author**: Analysis Agent (Lua Migration Swarm)
**Status**: Complete analysis

---

## Executive Summary

The current roadmap assumes the 4D boomer shooter is a **compiled Rust binary** in a separate repository (`Rust4D-Shooter`), depending on engine crates via Cargo git URLs. The decision to shift to **Lua scripting** fundamentally changes the architecture: the engine becomes a runtime that loads and executes Lua scripts, and the game is authored as Lua scripts + data files (RON scenes, textures, sounds).

**Total impact**: Of the ~44-60 sessions estimated across engine + game roadmaps, approximately 13-21 sessions of game-side Rust work are **eliminated** (replaced by Lua scripting that is faster to iterate on but requires engine bindings). Approximately 4-8 sessions of **new engine work** are introduced for the Lua runtime, binding layer, and scripting infrastructure. The net effect is roughly neutral in total effort but shifts complexity from the game repository to the engine.

**Key architectural change**: Instead of two compiled Rust repositories connected by Cargo dependencies, there is one Rust engine binary/library that embeds a Lua runtime (mlua/rlua) and exposes engine APIs. The "game" is a directory of `.lua` files, `.ron` scenes, and asset files.

---

## Per-Document Analysis

---

### 1. Engine/Game Split Plan (`engine-game-split.md`)

#### What Stays the Same
- ECS migration to hecs (Phase 1) -- still needed, internal engine work
- The `rust4d_math`, `rust4d_physics`, `rust4d_core`, `rust4d_render`, `rust4d_input` crate structure
- The dependency chain between engine crates
- Generic body manipulation methods on `PhysicsWorld`
- Collision layer presets staying in engine

#### What Changes
- **The entire "two-repo" architecture is replaced.** There is no `Rust4D-Shooter` repo with `Cargo.toml` git URL dependencies. The game is Lua scripts loaded by the engine at runtime.
- **Phase 4 (Create Game Repository) is eliminated.** No new GitHub repo, no moving files, no adapting Rust imports.
- **Phase 5 (Engine Cleanup) changes scope.** Instead of removing `src/` and becoming "library-only," the engine keeps a binary (`src/main.rs`) that IS the runtime. The binary initializes wgpu, winit, physics, audio, then loads a Lua entry script.
- **The `rust4d_game` crate's role changes.** `CharacterController4D`, events, FSM, scene helpers, tweens -- these still exist in Rust, but they need Lua bindings so scripts can use them. The crate shifts from "things the game Rust binary calls" to "things the Lua runtime exposes."
- **Game-defined ECS components** (Health, Weapon, AIState, Pickup) were going to be Rust structs in the game repo. Now they must either:
  - (a) Be defined in Lua as tables/userdata and stored via a generic `ScriptComponent` ECS component, or
  - (b) Be predefined in the engine as common game components with Lua-accessible fields.
  - Recommendation: Hybrid -- provide common ones in engine, allow Lua-defined ones via a `LuaComponent` wrapper.

#### What Gets Simpler
- **No two-repo maintenance friction.** No coordinating Cargo.toml versions across repos. No git URL dependency issues.
- **Faster game iteration.** Lua hot-reload means changing game logic does not require recompilation. Modify a `.lua` file, reload, and the change is live.
- **Game-defined components are dynamic.** No need to recompile the engine to add a new component type.
- **.cargo/config.toml local path override hack is eliminated.** No need for the "git URL in CI, local path for dev" pattern.

#### What Gets Harder
- **The engine must embed a Lua runtime** (mlua or rlua). New dependency, new API surface, new error handling patterns.
- **All engine APIs need Lua bindings.** Every method the game calls (raycasting, physics queries, audio playback, particle spawning, etc.) needs a Lua-callable wrapper.
- **Type safety boundary.** Rust's compile-time guarantees stop at the Lua boundary. Runtime type errors replace compile-time errors for game logic.
- **Debugging Lua scripts** is harder than debugging Rust: no borrow checker, no compile-time type checking, stack traces are less informative.
- **Performance boundary management.** Must carefully decide what runs in Rust (hot paths) vs Lua (game logic).

#### New Engine Work Required
- **New `rust4d_scripting` crate** (or module): Lua runtime initialization, script loading, hot-reload, error handling
- **Lua binding layer for all engine types**: Vec4, Transform4D, PhysicsWorld, AudioEngine4D, ParticleSystem, etc.
- **Script-driven game loop**: The engine's main loop calls Lua hooks (on_init, on_update, on_fixed_update, on_render_ui, on_event)
- **Script-managed ECS components**: A way for Lua to define and query custom components

#### Session Estimate Impact
- Phase 1 (ECS Migration): **Unchanged** (4-6 sessions)
- Phase 2 (Game Logic Extraction): **Reduced to 2-3 sessions** -- still need generic physics APIs and `rust4d_game` crate, but input refactoring is less urgent (Lua scripts handle input mapping)
- Phase 3 (Pluggable Scene Instantiation): **Unchanged** (1 session) -- scene setup still needs to be decoupled, but Lua scripts replace the "game calls helpers" pattern with "Lua scripts call helpers"
- Phase 4 (Create Game Repo): **Eliminated** (saves 1-2 sessions)
- Phase 5 (Engine Cleanup): **Replaced** by scripting integration work (2-4 sessions for Lua runtime + bindings)
- **New**: Lua runtime + binding layer: **4-8 sessions** (see binding surface area below)

---

### 2. Phase 0 Foundation (`split-phase-0-foundation.md`)

#### What Stays the Same
- **All five tasks are unchanged.** These are engine-internal fixes:
  - Task 1: Rotor4 Serialization -- still needed for scene serialization
  - Task 2: Physics Type Serialization Audit -- still needed (deferred)
  - Task 3: Fixed Timestep -- still needed for deterministic physics
  - Task 4: Diagonal Movement Normalization -- still needed, but moves from `src/systems/simulation.rs` to engine's CharacterController4D
  - Task 5: Back-Face Culling -- still needed

#### What Changes
- Task 4 (diagonal normalization) was in game code (`src/systems/simulation.rs`) to be moved to the game repo. Now it should be fixed in the engine's `CharacterController4D` directly, since Lua scripts will call `controller:apply_movement(dx, dy, dz, dw)` and the controller handles normalization.

#### What Gets Simpler
- Nothing simpler -- these are all engine-internal.

#### What Gets Harder
- Nothing harder -- these are all engine-internal.

#### Session Estimate Impact
- **Unchanged**: ~1 session total.

---

### 3. Split Phases 1-5 Summary (`split-phases-1-5.md`)

#### What Stays the Same
- Phase 1 (ECS Migration): Fully unchanged
- Phase 2 (Game Logic Extraction + rust4d_game): Mostly unchanged -- still need CharacterController4D, event system, generic physics APIs
- Phase 3 (Pluggable Scene Instantiation): Mostly unchanged

#### What Changes
- Phase 4 (Create Game Repository): **Eliminated entirely**
- Phase 5 (Engine Cleanup): **Repurposed** -- instead of removing the binary and becoming library-only, the engine retains a binary that serves as the Lua runtime host
- The "Post-Split Phases" table changes: game work is no longer "game Rust code" but "Lua scripts"
- The dependency model (git URL hybrid) is **eliminated**

#### Session Estimate Impact
- Phases 1-3: **Unchanged** (8.5-11 sessions)
- Phase 4: **Eliminated** (saves 1-2 sessions)
- Phase 5: **Repurposed** to scripting integration (net change: +1-3 sessions vs original 0.5-1 session)
- **New total for split work**: ~10-15 sessions (was 9.5-14, but Phase 5 grows to include scripting infrastructure)

---

### 4. Post-Split Phase 1: Combat Core (`post-split-phase-1-combat-core.md`)

#### What Stays the Same
- **Sub-Phase A (Raycasting)**: 100% engine work, completely unchanged
  - `Ray4D` in `rust4d_math`
  - Ray-shape intersections in `rust4d_physics`
  - `PhysicsWorld::raycast()` and `raycast_nearest()`
  - `Vec4` utility additions
  - `sphere_vs_sphere` visibility fix
- **Sub-Phase B (Collision Events & Triggers)**: 100% engine work, completely unchanged
  - `CollisionEvent` data structs
  - `drain_collision_events()` API
  - Trigger detection bug fix
  - Enter/stay/exit tracking

#### What Changes
- **The "Game Implements" section transforms entirely.** Instead of a Rust game binary doing:
  ```rust
  let events = physics_world.drain_collision_events();
  for event in events { /* game logic */ }
  ```
  The engine calls a Lua callback:
  ```lua
  function on_collision(event)
    if event.kind == "TriggerEnter" then
      -- game logic in Lua
    end
  end
  ```
- **Health, Damage, EventBus, GameEvent enum** -- all described as "game Rust code" -- now become Lua implementations.
- The engine must **expose** `CollisionEvent`, `CollisionEventKind`, `RayHit`, `WorldRayHit`, `RayTarget` to Lua.

#### What Gets Simpler
- Iterating on combat game logic is faster (Lua hot-reload vs Rust recompile).
- Health/damage models can be changed without any Rust compilation.

#### What Gets Harder
- Performance of per-frame collision event processing in Lua. If there are hundreds of collision events per frame, iterating them in Lua adds overhead. Likely acceptable for a boomer shooter (~50-100 entities).
- Raycasting results must be marshalled from Rust to Lua (Vec4 positions, normals, distances). Each raycast result crossing the boundary has a small cost.

#### New Engine Work Required
- Lua bindings for: `Ray4D`, `RayHit`, `WorldRayHit`, `RayTarget`, `CollisionEvent`, `CollisionEventKind`
- Lua bindings for: `PhysicsWorld::raycast()`, `PhysicsWorld::raycast_nearest()`, `PhysicsWorld::drain_collision_events()`
- Lua callback registration for collision/trigger events

#### Session Estimate Impact
- Engine Sub-Phase A: **Unchanged** (1 session)
- Engine Sub-Phase B: **Unchanged** (0.75 sessions)
- Game-side work (2 sessions in original): **Replaced by Lua scripting** (no Rust compilation needed, but binding work adds ~0.5 sessions to engine)
- **New estimate**: 2.25 sessions engine (was 1.75 engine + 2 game = 3.75 total; now 2.25 engine + Lua scripting time)

---

### 5. Post-Split Phase 2: Weapons & Feedback (`post-split-phase-2-weapons-feedback.md`)

#### What Stays the Same
- **Sub-Phase A (Audio System)**: `rust4d_audio` crate wrapping kira -- 100% engine work, unchanged
  - AudioEngine4D, 4D spatial audio, bus/mixer, sound loading
- **Sub-Phase B (HUD/egui Overlay)**: OverlayRenderer -- 100% engine work, unchanged
  - egui-wgpu integration, event forwarding, begin/end frame pattern
- **Sub-Phase C (Particle System)**: ParticleSystem -- 100% engine work, unchanged
  - CPU-simulated billboards, additive/alpha blending, depth integration
- **Sub-Phase D (Screen Effects)**: ScreenShake, TimedEffect, depth texture getter -- engine work, unchanged

#### What Changes
- **Weapon system**: Was "100% game-side Rust" (hitscan shotgun, projectile rocket launcher, ammo, weapon switching). Now becomes Lua scripting. The engine must expose:
  - `PhysicsWorld::raycast()` to Lua for hitscan
  - Entity spawning to Lua for projectiles
  - `ParticleSystem::spawn_burst()` to Lua for effects
  - `AudioEngine4D::play_spatial()` to Lua for weapon sounds
- **HUD widgets**: Were game-side Rust using `egui::Context`. Now Lua scripts build the HUD. This requires either:
  - (a) Exposing egui's API to Lua (complex, egui is immediate-mode with closures)
  - (b) Providing a simplified Lua-friendly HUD API: `hud:text(x, y, "HP: 100")`, `hud:bar(x, y, w, h, value, max, color)`
  - **Recommendation**: Option (b) -- create an engine-side `LuaHud` abstraction that wraps egui calls. Exposing raw egui to Lua is impractical.
- **Sound triggering logic**: Was game-side Rust event handlers. Now Lua callbacks.
- **Particle effect presets**: Were `ParticleEmitterConfig` structs in Rust. Now defined in Lua tables or RON data files.
- **Damage flash, muzzle flash lighting**: Were game-side egui overlay code. Now Lua-driven via the HUD abstraction.

#### What Gets Simpler
- **Effect tuning is dramatically faster.** Changing a particle effect preset, a screen shake amount, or a sound trigger is a Lua file edit + reload, not a Rust recompile.
- **HUD iteration is faster.** Layout and styling changes are instant.

#### What Gets Harder
- **HUD API design.** Exposing egui's full power to Lua is impractical. Need to design a simplified HUD API that covers 80% of needs. This is new engine work.
- **Audio asset management from Lua.** Need a clean way for Lua scripts to reference sound assets by name/path.
- **Particle config from Lua.** Need to convert Lua tables to `ParticleEmitterConfig` structs efficiently.

#### New Engine Work Required
- `LuaHud` wrapper: simplified HUD API for Lua (text, bars, images, rectangles, custom painters)
- Lua bindings for: `AudioEngine4D` (load_sound, play, play_spatial, play_oneshot, set_bus_volume, update_listener)
- Lua bindings for: `ParticleSystem` (spawn_burst, spawn_emitter, update_emitter_position, stop_emitter, kill_emitter)
- Lua bindings for: `ScreenShake` (add_trauma, get_offset)
- Lua bindings for: `TimedEffect` (trigger, intensity, is_active)
- Lua table -> `ParticleEmitterConfig` conversion

#### Session Estimate Impact
- Engine Sub-Phases A-D: **Unchanged** (4.5-5.5 sessions)
- Game-side work (3.75 sessions in original): **Replaced by Lua + binding work** (~1 session for HUD API + audio/particle bindings)
- **New estimate**: 5.5-6.5 sessions engine (was 4.5-5.5 engine + 3.75 game = 8.25-9.25 total; now 5.5-6.5 engine + Lua scripting time)

---

### 6. Post-Split Phase 3: Enemies & AI (`post-split-phase-3-enemies-ai.md`)

#### What Stays the Same
- **Sub-Phase A (Sprite/Billboard Rendering)**: 100% engine, unchanged
  - SpriteBatch, SpritePipeline, W-distance fade, SpriteAnimation
- **Sub-Phase B (Spatial Queries)**: 100% engine, unchanged
  - query_sphere(), query_sphere_sorted(), line_of_sight()
- **Sub-Phase C (FSM Framework)**: StateMachine<S> in rust4d_game -- engine, unchanged
- **Sub-Phase D (Area Damage)**: query_area_effect(), apply_impulse() -- engine, unchanged
- **Particle System**: Already covered in P2, unchanged

#### What Changes
- **Enemy AI logic**: Was game-side Rust (`EnemyAI` struct, `EnemyState` enum, per-state update functions). Now becomes Lua. The FSM framework's `StateMachine<S>` either:
  - (a) Gets Lua bindings so Lua scripts use it directly
  - (b) Is reimplemented in pure Lua (trivial -- it is ~30 lines as noted in the plan)
  - **Recommendation**: Option (b). A Lua FSM is simpler than binding the generic `StateMachine<S>`. The engine FSM is generic over a Rust type parameter; Lua does not need that.
- **Enemy type definitions** (`EnemyDef`, `WBehavior`): Were Rust data structs. Now Lua tables or RON data files. Much more natural as data.
- **Enemy spawning and wave management**: Was game-side Rust. Now Lua scripts.
- **W-phasing behavior**: Was game-side Rust controlling W-coordinates. Now Lua scripts.
- **Sprite sheets per enemy**: Asset references now come from Lua/data rather than compiled Rust constants.

#### What Gets Simpler
- **Enemy type iteration is dramatically faster.** Changing enemy stats (health, speed, damage, AI parameters) requires only editing a Lua file.
- **New enemy types require zero Rust compilation.** Define a new enemy entirely in Lua + sprite assets.
- **W-behavior tuning** (phase cooldown, phase duration, preferred W-offset) is data-driven.
- **AI behavior prototyping** is much faster in Lua.

#### What Gets Harder
- **AI performance.** Per-enemy AI updates in Lua are slower than Rust. For 20-50 enemies at 60fps, each enemy's AI running in Lua should be fine (~microseconds per enemy). For 200+ enemies, could become a bottleneck.
- **Spatial query results marshalling.** `query_sphere()` returns `Vec<SpatialQueryResult>` -- each result must cross the Lua boundary.
- **Complex AI behaviors.** Lua lacks Rust's type system for enforcing valid state transitions. More runtime errors possible.

#### New Engine Work Required
- Lua bindings for: `PhysicsWorld::query_sphere()`, `query_sphere_sorted()`, `query_area_effect()`, `line_of_sight()`, `apply_impulse()`
- Lua bindings for: `SpriteBatch::add_sprite_4d()`, `add_sprite_3d()`
- Lua bindings for: `SpriteAnimation` (new, update, current_frame, reset)
- Lua bindings for: `SpatialQueryResult`, `AreaEffectHit` result types
- Lua entity system: ability to spawn entities with sprite components, physics bodies, and Lua-defined components from scripts
- **Optional**: Lua FSM utility module (pure Lua, shipped with engine as a standard library)

#### Session Estimate Impact
- Engine Sub-Phases A-D + Particles: **Unchanged** (4.0 sessions)
- Game-side work (3.0 sessions): **Replaced by Lua + binding work** (~0.75 sessions for spatial query/sprite/entity bindings)
- **New estimate**: 4.75 sessions engine (was 4.0 engine + 3.0 game = 7.0 total; now 4.75 engine + Lua scripting time)

---

### 7. Post-Split Phase 4: Level Design Pipeline (`post-split-phase-4-level-design.md`)

#### What Stays the Same
- **Sub-Phase A (Shape Type Expansion)**: 100% engine, unchanged
  - Hyperprism4D, Hypersphere4D, ShapeTemplate variants
- **Sub-Phase B (RON Preview Tool)**: 100% engine, unchanged
  - Hot-reload viewer, camera controls, W-slice navigation
- **Sub-Phase C (Tween/Interpolation System)**: 100% engine, unchanged
  - Interpolatable trait, EasingFunction, Tween<T>, TweenManager

#### What Changes
- **Sub-Phase D (Declarative Trigger System)**: The data model (`TriggerDef`, `TriggerZone`, `TriggerAction`, `TriggerRepeat` in RON) stays the same. But the `TriggerAction::GameEvent(String)` escape hatch now fires directly into Lua instead of into a Rust-side EventBus.
  - `TriggerAction::GameEvent("pickup_health_50")` -> engine calls Lua function `on_game_event("pickup_health_50")`
  - This is **simpler** than the original design which required a typed Rust EventBus with an `AnyEvent` variant.
- **Trigger runtime** (`TriggerRuntime` in `rust4d_game`): Still engine-side Rust, but instead of calling Rust game code for `GameEvent` actions, it calls Lua callbacks. The runtime stays in Rust for performance.
- **Door/elevator mechanics**: Were game-side Rust. Now Lua scripts using tween bindings.
- **Pickup system**: Was game-side Rust. Now Lua scripts.
- **Level scripting**: Was "80% declarative triggers + 20% custom Rust code." Now "80% declarative triggers + 20% Lua scripts." The Lua approach is **much more natural** for the 20% custom scripting than writing Rust systems.

#### What Gets Simpler
- **`GameEvent(String)` -> Lua is the ideal pattern.** The original plan struggled with how to bridge typed Rust events with string-named game events. Lua naturally handles string-dispatched events.
- **Level scripting is dramatically simpler.** The "20% custom code" that was going to be Rust systems becomes Lua scripts. Lua is a natural fit for game scripting -- this is exactly what Lua was designed for.
- **Iterating on trigger behavior** (door timing, elevator waypoints, pickup effects) is instant with Lua hot-reload.
- **No need for the "escape hatch" design pattern.** The trigger system can just call Lua directly for any action.

#### What Gets Harder
- **Debugging trigger -> Lua -> engine round-trips.** When a trigger fires a GameEvent that calls Lua that calls TweenManager, the debug trace crosses multiple boundaries.

#### New Engine Work Required
- `TriggerAction::GameEvent` implementation calls Lua callback instead of Rust EventBus
- Lua bindings for: `TweenManager::tween_position()`, `Tween` operations
- Lua bindings for: entity spawn/despawn from scripts
- **Optional**: `TriggerAction::RunLua(String)` -- execute arbitrary Lua code as a trigger action (powerful but potentially dangerous)

#### Session Estimate Impact
- Engine Sub-Phases A-C: **Unchanged** (3.5 sessions)
- Engine Sub-Phase D (triggers): Trigger data model unchanged (0.5 sessions). Trigger runtime simplified (0.5 sessions -- calling Lua is simpler than Rust EventBus bridging).
- Game-side work (2.5-4 sessions): **Replaced by Lua** + minor binding work (~0.5 sessions for tween/entity bindings)
- **New estimate**: 4.5 sessions engine (was 4.5 engine + 2.5-4 game = 7-8.5 total; now 4.5 engine + Lua scripting time)

---

### 8. Post-Split Phase 5: Editor & Polish (`post-split-phase-5-editor-polish.md`)

#### What Stays the Same
- **Sub-Phase A (Texture Support)**: 100% engine, unchanged
  - Triplanar mapping, TextureManager, shader changes
- **Sub-Phase B (Lighting System)**: 100% engine, unchanged
  - PointLight4D, W-distance attenuation, shadow mapping
- **Sub-Phase C (Input Rebinding)**: Mostly unchanged
  - `InputMap::rebind()`, `to_toml()`, `from_toml()` -- still engine-side
- **Sub-Phase D (Editor Framework)**: 100% engine, unchanged
  - `rust4d_editor` crate, EditorHost trait, entity list, property inspector, W-slice navigation, scene operations

#### What Changes
- **The `EditorHost` trait**: Was designed for a Rust game binary to implement. Now the engine runtime itself implements it. The editor becomes a built-in development tool rather than a game-opt-in overlay.
- **Game-specific editor panels** (weapon tuning, enemy config): Were game-side Rust extensions. Now either:
  - (a) Engine provides a generic "Lua component inspector" that can edit any Lua-defined component
  - (b) Lua scripts define custom editor panels (requires exposing egui-like API to Lua, which is impractical)
  - **Recommendation**: (a) -- the editor automatically inspects Lua-defined components as key-value tables. Custom game-specific panels are deferred.
- **Pause menu and settings screens**: Were game-side Rust using egui. Now need to be built via the `LuaHud` API or a Lua menu system.
- **Input rebinding UI**: Was game-side Rust. Now Lua-driven. The `InputMap` API still exists in engine, but the UI for rebinding is Lua.

#### What Gets Simpler
- **Editor is always available.** No need for games to opt-in via dev-dependencies. The engine runtime always has the editor, toggled with F12.
- **Game-specific editor panels are automatic.** A generic Lua component inspector displays any component defined in scripts.

#### What Gets Harder
- **Lua menu system.** Building a full pause menu with settings screens in Lua requires the `LuaHud` API to support interactive elements (buttons, sliders, text inputs). This is more complex than simple HUD display.
- **Input rebinding UI in Lua.** "Press a key..." capture flow is tricky to implement across the Lua boundary.

#### New Engine Work Required
- `EditorHost` implementation for the Lua runtime (instead of the game implementing it)
- Generic Lua component inspector in the editor
- `LuaHud` interactive elements (buttons, sliders) for Lua-driven menus
- Lua bindings for `InputMap::rebind()`, `unbind()`, `conflicts()`, `reset_defaults()`

#### Session Estimate Impact
- Engine Sub-Phases A-D: **Unchanged** (10-12.5 sessions)
- Game-side work (2-4 sessions): **Replaced by Lua** + minor engine work for Lua component inspector (~0.5 sessions)
- **New estimate**: 10.5-13 sessions engine (was 10-12.5 engine + 2-4 game = 12-16.5 total; now 10.5-13 engine + Lua scripting time)

---

### 9. Game Roadmap Summary (`game-roadmap-summary.md`)

#### What Stays the Same
- **All gameplay design** is unchanged: health/damage system, weapon types, enemy types, AI behaviors, level design patterns, W-axis gameplay mechanics, pickup system, door/elevator mechanics, HUD widgets, pause menus.
- **The sequence of capabilities** is unchanged: combat core -> weapons & feedback -> enemies -> level design -> polish.

#### What Changes
- **The entire document shifts from "Rust game code" to "Lua scripts."** Every game-side implementation task that was `fn some_system(world: &mut World, ...)` becomes a Lua function.
- **Phase 0 (Game Repo Setup) is eliminated.** No repo to create, no Cargo.toml, no file moves. The "game" is a `game/` directory with Lua files next to the engine.
- **Cargo.toml dependencies, git URL setup, .cargo/config.toml** -- all eliminated.
- **Every "Game-side work" session estimate** needs recalibration. Lua is faster to write but slower to run. Net effect on development time is roughly neutral for logic, faster for iteration.

#### What Gets Simpler
- **No repository coordination.** One repo, one build system.
- **No Rust compilation for game changes.** Massive iteration speed improvement.
- **Adding new game systems** (new enemy type, new weapon, new pickup) requires only Lua files and assets.
- **Modding becomes natural.** The game is already Lua scripts -- mods are just additional/replacement Lua scripts.

#### What Gets Harder
- **Game code is untyped.** All the carefully typed Rust structs (`Health`, `Weapon`, `EnemyDef`, etc.) become Lua tables. Runtime errors replace compile-time errors.
- **Performance profiling.** When something is slow, determining whether it is the Lua logic, the binding overhead, or the engine itself is harder.
- **Distribution.** Instead of a single compiled binary, the game ships with Lua scripts that could be modified by end users (may or may not be desired -- for modding, it is a feature).

#### Session Estimate Impact
- Phase 0 (Game Repo Setup): **Eliminated** (saves 1-2 sessions)
- Game Phases 1-5 (13-21 sessions of Rust): **Replaced by Lua scripting** time. Lua coding is typically faster than Rust, so estimate ~8-14 sessions of Lua work for equivalent functionality.
- **New total game-side estimate**: 8-14 Lua sessions (was 13-21 Rust sessions)

---

## Summary of Total Shift

### Tasks Moving from Game-Side to Engine-Side

| Category | Original Location | New Location | Notes |
|----------|------------------|--------------|-------|
| Game entry point / main loop | Game repo `main.rs` | Engine `main.rs` (runtime) | Engine hosts the Lua VM and drives the loop |
| ECS component definitions | Game Rust structs | Lua tables + engine `LuaComponent` | Health, Weapon, etc. |
| Input mapping | Game `InputMap` config | Lua scripts calling engine `InputMap` API | Engine exposes bindings |
| Event bus / game events | Game Rust `EventBus` | Lua event dispatch (or engine-side `GameEvent` -> Lua callback) | Simplified |
| HUD rendering | Game Rust using egui | Lua via `LuaHud` API | New engine abstraction needed |
| AI/FSM logic | Game Rust `EnemyAI` | Lua scripts | Pure Lua reimplementation |
| Weapon logic | Game Rust weapon system | Lua scripts calling engine raycasting/physics | Bindings needed |
| Level scripting | Game Rust systems (20%) | Lua scripts (100% now) | Natural fit |
| Trigger response | Game Rust `EventBus` handlers | Lua callbacks from `GameEvent(String)` | Simpler |
| Particle presets | Game Rust `ParticleEmitterConfig` | Lua tables or RON data | Data-driven |
| Sound triggers | Game Rust event handlers | Lua callbacks | Bindings needed |
| Menu / settings UI | Game Rust egui code | Lua via `LuaHud` API | New engine abstraction needed |

**Summary**: Approximately **30-40 tasks** move from "game-side Rust implementation" to "Lua scripts calling engine APIs." The engine gains approximately **15-20 new binding tasks** to expose its APIs to Lua.

---

## The Performance Boundary

### MUST Stay in Rust (Performance-Critical Hot Paths)

| System | Why | Frequency |
|--------|-----|-----------|
| Physics step (`PhysicsWorld::step()`) | Collision detection, constraint solving, velocity integration | Every fixed timestep (60Hz) |
| Collision detection (narrow phase) | Shape-shape intersection math (sphere_vs_sphere, etc.) | O(n^2) body pairs per step |
| Raycasting (`ray_vs_sphere`, `ray_vs_aabb`, etc.) | Math-intensive, potentially many per frame | Per weapon fire, per AI LOS check |
| 4D slicing compute shader | GPU compute, already in WGSL | Every frame |
| Rendering pipeline (all render passes) | GPU work, wgpu API calls | Every frame |
| Particle simulation (CPU update loop) | Per-particle position/velocity/age update | Every frame, hundreds of particles |
| Spatial queries (`query_sphere`, `query_area_effect`) | Linear scan of all bodies | Per AI enemy per frame |
| Audio engine (kira internals) | Audio thread, real-time DSP | Continuous |
| Tween updates (`TweenManager::update()`) | Per-entity interpolation | Every frame |
| Trigger runtime (collision event processing) | Per-event trigger zone checks | Every physics step |
| Scene loading/parsing (RON deserialization) | I/O + parsing | On scene load |
| ECS queries (hecs iteration) | Component iteration | Every frame per system |

### Can Be Lua (Game Logic, Low Frequency)

| System | Why Lua is Fine | Frequency |
|--------|-----------------|-----------|
| AI decision-making | Per-enemy logic, simple conditionals | Per enemy per frame (~50 enemies) |
| Weapon fire logic | Button press -> raycast call + event dispatch | Per player input (low frequency) |
| Health/damage application | Simple arithmetic on entity components | Per damage event |
| Game event dispatch | String matching, function calls | Per event |
| HUD rendering (via LuaHud API) | Builds draw commands, engine renders | Every frame (lightweight) |
| Menu/UI logic | Button clicks, state transitions | Per user input |
| Level scripting | Trigger responses, door/elevator logic | Per trigger activation |
| Enemy state transitions | FSM logic, simple conditionals | Per enemy per state change |
| Pickup effects | Apply health/ammo/weapon | Per pickup event |
| Spawn logic | Create entities with components | Per spawn event |
| W-phasing behavior | Modify W-coordinate periodically | Per phasing enemy per phase |

### The Boundary Rule
**Anything called O(n^2) or more per frame stays in Rust. Anything called O(n) or less per frame with n < 100 can be Lua.**

---

## The Binding Surface Area

### Tier 1: Core Types (Must Have -- Day 1)

| Type | Lua Representation | Methods to Expose |
|------|-------------------|-------------------|
| `Vec4` | Userdata with metamethods | new, x/y/z/w getters/setters, +, -, *, /, dot, cross, length, normalized, distance |
| `Transform4D` | Userdata | position (Vec4), rotation (read-only or simplified), translate, rotate |
| `Rotor4` | Userdata (read-mostly) | identity, from_angle, slerp |
| `hecs::Entity` | Lightweight handle (u64) | id(), is_alive() |

### Tier 2: Physics (Needed for Combat Core)

| Type/Function | Lua Exposure | Notes |
|--------------|-------------|-------|
| `PhysicsWorld::raycast()` | `physics:raycast(origin, direction, max_dist, layer_mask)` -> table of hits | Returns Lua table array |
| `PhysicsWorld::raycast_nearest()` | `physics:raycast_nearest(...)` -> hit or nil | Single result |
| `PhysicsWorld::drain_collision_events()` | Callback-based: `on_collision(event)` | Engine calls Lua per event |
| `PhysicsWorld::query_sphere()` | `physics:query_sphere(center, radius, layer)` -> table | Results as Lua table |
| `PhysicsWorld::query_area_effect()` | `physics:query_area_effect(center, radius, layer, los)` -> table | With falloff |
| `PhysicsWorld::line_of_sight()` | `physics:line_of_sight(from, to, layers)` -> bool | Simple boolean |
| `PhysicsWorld::apply_impulse()` | `physics:apply_impulse(body_key, impulse_vec4)` | Modifies velocity |
| `CollisionEvent` | Lua table with kind, body keys | Read-only |
| `RayHit` / `WorldRayHit` | Lua table with distance, point, normal, target | Read-only |
| `CollisionLayer` | Enum/constants: `PLAYER`, `ENEMY`, `STATIC`, `TRIGGER`, `PROJECTILE` | Bitflag operations |
| `CollisionFilter` | Constructor functions: `filter.player()`, `filter.enemy()`, `filter.trigger(layers)` | Presets |
| `BodyKey` | Opaque handle | Used in physics queries |

### Tier 3: Audio

| Type/Function | Lua Exposure | Notes |
|--------------|-------------|-------|
| `AudioEngine4D::load_sound()` | `audio:load("path/to/sound.ogg")` -> handle | Returns opaque handle |
| `AudioEngine4D::play()` | `audio:play(handle, bus)` | Non-spatial |
| `AudioEngine4D::play_spatial()` | `audio:play_spatial(handle, position, bus, config)` | 4D spatial |
| `AudioEngine4D::play_oneshot()` | `audio:play_oneshot(handle, bus)` | Fire and forget |
| `AudioEngine4D::update_listener()` | `audio:update_listener(position, forward, up)` | Once per frame |
| `AudioEngine4D::set_bus_volume()` | `audio:set_volume(bus, volume)` | Bus control |
| `AudioBus` | Constants: `SFX`, `MUSIC`, `AMBIENT` | Enum |

### Tier 4: Rendering / Particles

| Type/Function | Lua Exposure | Notes |
|--------------|-------------|-------|
| `ParticleSystem::spawn_burst()` | `particles:burst(position, config)` | Config as Lua table |
| `ParticleSystem::spawn_emitter()` | `particles:emitter(position, config)` -> id | Returns emitter ID |
| `ParticleSystem::stop_emitter()` | `particles:stop(id)` | Existing particles continue |
| `ParticleSystem::kill_emitter()` | `particles:kill(id)` | Immediate removal |
| `ParticleEmitterConfig` | Lua table with named fields | Converted to Rust struct |
| `SpriteBatch::add_sprite_4d()` | `sprites:add(position4d, slice_w, size, frame, tint, w_fade)` | Per-frame call |
| `SpriteAnimation` | Lua-side (pure Lua implementation preferred) | Trivial logic |
| `ScreenShake::add_trauma()` | `shake:add_trauma(amount)` | Simple setter |

### Tier 5: ECS / World

| Type/Function | Lua Exposure | Notes |
|--------------|-------------|-------|
| Entity spawn | `world:spawn(components_table)` -> entity | Components as Lua table |
| Entity despawn | `world:despawn(entity)` | By handle |
| Component get | `world:get(entity, "component_name")` -> value | Returns Lua value |
| Component set | `world:set(entity, "component_name", value)` | Sets component |
| Component has | `world:has(entity, "component_name")` -> bool | Query |
| Query | `world:query("Transform4D", "Health")` -> iterator | Returns entity + component tuples |
| Entity by name | `world:find("entity_name")` -> entity or nil | Name lookup |

### Tier 6: Game Framework (rust4d_game)

| Type/Function | Lua Exposure | Notes |
|--------------|-------------|-------|
| `CharacterController4D` | `controller:apply_movement(dx, dy, dz, dw)`, `:jump()`, `:is_grounded()` | Core player API |
| `TweenManager::tween_position()` | `tweens:position(entity, target_vec4, duration, easing)` | For doors/elevators |
| `EasingFunction` | Constants: `LINEAR`, `EASE_IN_QUAD`, `EASE_OUT_QUAD`, etc. | Enum |
| `InputMap::rebind()` | `input:rebind(action, key)` | For settings |
| `InputMap::is_pressed()` | `input:pressed("MoveForward")` -> bool | Per-frame input query |
| `InputMap::axis()` | `input:axis("MoveForward")` -> f32 | Analog value |

### Tier 7: HUD (New -- does not exist in current plans)

| Type/Function | Lua Exposure | Notes |
|--------------|-------------|-------|
| `LuaHud::text()` | `hud:text(x, y, text, options)` | Font size, color, alignment |
| `LuaHud::bar()` | `hud:bar(x, y, w, h, value, max, color)` | Health/ammo bars |
| `LuaHud::rect()` | `hud:rect(x, y, w, h, color)` | Damage flash, backgrounds |
| `LuaHud::image()` | `hud:image(x, y, w, h, texture)` | Icons, crosshair |
| `LuaHud::button()` | `hud:button(x, y, text)` -> bool (clicked) | Menu buttons |
| `LuaHud::slider()` | `hud:slider(x, y, w, value, min, max)` -> new_value | Settings sliders |

**Total binding count**: Approximately **80-100 individual functions/methods** need Lua exposure.

---

## Risk Assessment

### 1. Performance Risk (MEDIUM)

**Risk**: Lua overhead on per-frame game logic (AI for 50 enemies, collision event processing, HUD rendering) could impact frame times.

**Mitigation**:
- Profile early with representative enemy counts
- Keep hot paths in Rust (physics, rendering, particles)
- Use LuaJIT or mlua with optimized table access
- Batch Lua calls where possible (e.g., pass all collision events as one table, not individual callbacks)
- Provide Rust-side "system" helpers that do bulk operations (e.g., `physics:query_sphere_and_apply` instead of query-in-Lua-then-apply-impulse-in-Lua-per-entity)

**Likelihood**: Low for a boomer shooter with <100 entities. Medium if scope grows.

### 2. Binding Complexity Risk (HIGH)

**Risk**: The binding surface area (~80-100 functions) is substantial. Each binding requires careful type conversion, error handling, and documentation. This is the single largest new work item.

**Mitigation**:
- Use mlua's derive macros for automatic UserData implementation
- Prioritize bindings by game phase (Tier 1-2 first, defer Tier 5-7)
- Generate binding documentation automatically
- Write binding tests alongside each binding
- Consider a code generation approach for repetitive binding patterns

**Likelihood**: High. This is unavoidable new work.

### 3. Debugging Difficulty Risk (MEDIUM)

**Risk**: When a Lua script causes incorrect behavior, the developer must trace through Lua code, the binding layer, and engine Rust code. Stack traces are fragmented across languages.

**Mitigation**:
- Invest in good error messages at the binding layer (include Lua file + line number in all errors)
- Provide a Lua debug console (print to overlay, inspect variables)
- Log all binding calls in debug mode
- Lua linting/type-checking tools (Teal, LuaLS) for static analysis before runtime

**Likelihood**: Medium. Standard for any scripted game engine.

### 4. HUD API Design Risk (MEDIUM)

**Risk**: The `LuaHud` abstraction must be powerful enough for game menus (settings, pause, level select) but simple enough for Lua scripts. Getting this wrong means either insufficient UI capabilities or an overly complex Lua API.

**Mitigation**:
- Start with minimal API (text, bars, rects) -- sufficient for a boomer shooter HUD
- Add interactive elements (buttons, sliders) only for the settings menu
- Consider using a Lua UI library (like SUIT or a custom immediate-mode Lua UI) that maps to egui
- Defer complex menus to a later phase if the minimal API is insufficient

**Likelihood**: Medium. UI is always harder than expected.

### 5. Hot-Reload Stability Risk (LOW-MEDIUM)

**Risk**: Lua hot-reload can leave the game in an inconsistent state if a script is reloaded mid-frame or if global state is not properly reset.

**Mitigation**:
- Reload scripts only at safe points (beginning of frame, after physics step)
- Provide a `reload_all()` function that cleanly reinitializes the Lua state
- Keep persistent state (entity positions, health values) in the ECS (Rust side), not in Lua globals
- Lua scripts should be stateless functions that read/write ECS components

**Likelihood**: Low if the reload boundary is well-defined.

### 6. Distribution/Modding Risk (LOW)

**Risk**: Shipping Lua source files means game logic is readable and modifiable by end users. This could be a feature (modding) or a problem (piracy, cheating).

**Mitigation**:
- This is actually a **feature** for a boomer shooter (Doom modding is the gold standard)
- If obfuscation is needed, compile Lua to bytecode (LuaJIT bytecode is not human-readable)
- Separate "core engine" (Rust binary, compiled) from "game content" (Lua + assets)
- Steam DRM applies to the engine binary, not the scripts

**Likelihood**: Low -- modding support is a positive for the target audience.

### 7. Lua Ecosystem/Dependency Risk (LOW)

**Risk**: mlua or rlua could have compatibility issues, security vulnerabilities, or maintenance abandonment.

**Mitigation**:
- mlua is actively maintained and widely used in the Rust game dev community
- Pin to a specific version
- The Lua VM itself is one of the most stable, well-tested pieces of software in existence
- If mlua has issues, rlua is an alternative with a similar API

**Likelihood**: Very low. Lua and mlua are mature.

---

## Revised Session Estimates

### Engine Work

| Phase | Original Estimate | Revised Estimate | Change | Notes |
|-------|------------------|-----------------|--------|-------|
| Foundation (Phase 0) | 1 session | 1 session | 0 | Unchanged |
| Split Phase 1 (ECS) | 4-6 sessions | 4-6 sessions | 0 | Unchanged |
| Split Phase 2 (Extract + rust4d_game) | 3-4 sessions | 2-3 sessions | -1 | Input refactor less urgent |
| Split Phase 3 (Pluggable Scenes) | 1 session | 1 session | 0 | Unchanged |
| Split Phase 4 (Create Game Repo) | 1-2 sessions | **0 sessions** | **-1 to -2** | **Eliminated** |
| Split Phase 5 (Engine Cleanup) | 0.5-1 session | 0.5 session | -0.5 | Simplified (no repo removal) |
| **NEW: Lua Runtime + Core Bindings** | -- | **3-4 sessions** | **+3 to +4** | VM init, script loading, hot-reload, Tier 1-2 bindings |
| **NEW: Extended Bindings (Audio, Particles, HUD)** | -- | **2-3 sessions** | **+2 to +3** | Tier 3-7 bindings, LuaHud API |
| Post-Split P1 (Combat Core) | 1.75 sessions | 2.25 sessions | +0.5 | Collision/ray bindings added |
| Post-Split P2 (Weapons & Feedback) | 4.5-5.5 sessions | 5.5-6.5 sessions | +1 | HUD API + audio/particle bindings |
| Post-Split P3 (Enemies & AI) | 4.0 sessions | 4.75 sessions | +0.75 | Spatial query/sprite bindings |
| Post-Split P4 (Level Design) | 4.5 sessions | 4.5 sessions | 0 | Trigger -> Lua is simpler |
| Post-Split P5 (Editor & Polish) | 10-12.5 sessions | 10.5-13 sessions | +0.5 | Lua component inspector |
| **Engine Total** | **~31-39 sessions** | **~34-42 sessions** | **+3 to +3** | |

### Game Work

| Phase | Original Estimate (Rust) | Revised Estimate (Lua) | Change | Notes |
|-------|-------------------------|----------------------|--------|-------|
| Phase 0 (Repo Setup) | 1-2 sessions | **0 sessions** | **-1 to -2** | No repo to create |
| Phase 1 (Combat) | 2-3 sessions | 1.5-2 sessions | -0.5 to -1 | Lua faster than Rust for game logic |
| Phase 2 (Weapons & Feedback) | 3-4 sessions | 2-3 sessions | -1 | Lua iteration speed |
| Phase 3 (Enemies) | 3-4 sessions | 2-3 sessions | -1 | AI in Lua, data-driven enemies |
| Phase 4 (Level Design) | 2-4 sessions | 1.5-3 sessions | -0.5 to -1 | Lua natural for scripting |
| Phase 5 (Polish) | 2-4 sessions | 1.5-3 sessions | -0.5 to -1 | Menus/settings in Lua |
| **Game Total** | **13-21 sessions** | **8.5-14 sessions** | **-4.5 to -7** | |

### Grand Total

| | Original | Revised | Change |
|--|---------|---------|--------|
| Engine | 31-39 sessions | 34-42 sessions | +3 |
| Game | 13-21 sessions | 8.5-14 sessions | -5 to -7 |
| **Total** | **44-60 sessions** | **42.5-56 sessions** | **-1.5 to -4** |

**Net effect**: The Lua migration is roughly **session-neutral to slightly faster** overall. It shifts ~5 sessions of work from game Rust coding to engine binding work, while the Lua game code itself is faster to write than equivalent Rust.

---

## Recommended Binding Implementation Order

### Wave 1: Core Runtime (2 sessions -- blocks everything)
1. Lua VM initialization (mlua)
2. Script loading from filesystem
3. Hot-reload infrastructure (file watcher + safe reload)
4. `Vec4` userdata with full operator overloading
5. `Transform4D` userdata
6. Entity handle type
7. Game loop hooks: `on_init()`, `on_update(dt)`, `on_fixed_update(dt)`
8. `world:spawn()`, `world:despawn()`, `world:get()`, `world:set()`

### Wave 2: Physics + Input (1 session -- enables combat)
1. `physics:raycast()`, `physics:raycast_nearest()`
2. `physics:drain_collision_events()` (or callback-based)
3. `physics:query_sphere()`, `physics:line_of_sight()`
4. `physics:apply_impulse()`
5. `CollisionLayer` constants
6. `input:pressed()`, `input:axis()` -- basic input query
7. `controller:apply_movement()`, `controller:jump()`, `controller:is_grounded()`

### Wave 3: Audio + Particles + HUD (2 sessions -- enables game feel)
1. `audio:load()`, `audio:play()`, `audio:play_spatial()`
2. `audio:update_listener()`, `audio:set_volume()`
3. `particles:burst()`, `particles:emitter()`, `particles:stop()`
4. `ParticleEmitterConfig` from Lua table
5. `shake:add_trauma()`
6. `hud:text()`, `hud:bar()`, `hud:rect()`
7. `hud:button()`, `hud:slider()` (for menus)

### Wave 4: Advanced (1 session -- enables level design + editor integration)
1. `sprites:add()` for enemy rendering
2. `tweens:position()`, easing constants
3. `physics:query_area_effect()`
4. `world:query()` iterator
5. `world:find()` name lookup
6. `input:rebind()`, `input:to_toml()`, `input:from_toml()`
7. Lua component inspector for editor

---

## Conclusion

The shift from "compiled Rust game binary" to "Lua scripts + assets" is a natural evolution for a game engine. The key insight is that the **engine's internal work is almost entirely unchanged** -- physics, rendering, math, audio, particles, editor are all still Rust. What changes is the **boundary between engine and game**: instead of a Cargo dependency, it becomes a Lua FFI boundary.

The biggest risks are binding complexity (mitigated by incremental delivery) and HUD API design (mitigated by starting minimal). The biggest gains are iteration speed (Lua hot-reload) and the elimination of two-repo maintenance friction.

For a 4D boomer shooter with <100 entities, the performance overhead of Lua is negligible. The binding surface area is substantial (~80-100 functions) but deliverable in ~5-7 sessions spread across the engine roadmap phases.
