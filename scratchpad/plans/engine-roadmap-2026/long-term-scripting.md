# Scripting System for Rust4D

**Created:** 2026-01-27
**Status:** FUTURE/DRAFT - Not for immediate implementation
**Effort:** 6-8 sessions (MEDIUM)
**Priority:** P6 (Long-term feature)

---

## Executive Summary

A scripting system would allow game logic to be written in a high-level language and hot-reloaded without recompilation. This dramatically improves iteration speed for gameplay development and enables modding support. However, scripting adds significant complexity and is not needed during the engine's foundation phase.

**Recommendation:** Defer until Phase 5+ is complete and core engine is stable. Scripting should be an additive feature, not a replacement for Rust-based game logic.

---

## Rationale

### Current State
- All game logic is written in Rust
- Changes require full recompilation (slow iteration)
- No runtime modification of behavior
- High barrier to entry for gameplay programmers

### Benefits of Scripting
1. **Hot-reload** - Modify gameplay without restart
2. **Rapid iteration** - Test changes in seconds, not minutes
3. **Lower barrier** - Designers can write logic without Rust knowledge
4. **Modding support** - Users can extend the game
5. **Safer experimentation** - Scripts can't crash the engine (with sandboxing)
6. **Data-driven design** - Behavior defined in scene files

### Why Not Now?
- Core engine architecture still evolving
- API surface not yet stable (exposing unstable API to scripts = pain)
- Physics, rendering, and scene management need to be solid first
- Premature optimization - we don't know what gameplay patterns we need yet

---

## Scripting Language Options

### Option 1: Lua (via `mlua` or `rlua`)

**Description:** Industry-standard embeddable scripting language. Used in WoW, Roblox, Love2D, and countless game engines.

**Pros:**
- Battle-tested in game development
- Excellent performance (LuaJIT is very fast)
- Large ecosystem and community
- Familiar to many game developers
- `mlua` crate has great Rust integration
- Small runtime footprint (~200KB)

**Cons:**
- 1-indexed arrays (Rust programmers will trip over this)
- Weak typing (no compile-time safety)
- Garbage collection pauses (though small)
- Separate type system from Rust

**Best for:** General-purpose scripting, gameplay logic, AI behaviors

**Example:**
```lua
-- scripts/player_controller.lua
function on_update(entity, dt)
    if input.is_key_down("W") then
        entity:apply_force(Vec4.new(0, 0, 10, 0))
    end

    if input.is_key_pressed("Space") then
        entity:jump()
    end
end
```

---

### Option 2: Rhai (Rust-native scripting)

**Description:** A scripting language written in pure Rust, designed for embedding in Rust applications.

**Pros:**
- Rust-like syntax (familiar to Rust developers)
- Type-safe integration with Rust
- No garbage collection (reference counting)
- Pure Rust (no C dependencies, easy to build)
- Good error messages
- Sandboxed by default

**Cons:**
- Smaller community than Lua
- Less tooling (no LSP, debugger)
- Slower than Lua/LuaJIT
- Not well-known (learning curve for non-Rust devs)
- More verbose than Lua

**Best for:** Configuration scripts, simple behaviors, Rust-centric projects

**Example:**
```rust
// scripts/enemy_ai.rhai
fn on_update(enemy, dt) {
    let player = world.find_by_tag("player");
    let direction = (player.position - enemy.position).normalized();

    if enemy.can_see(player) {
        enemy.move_towards(direction, 5.0);
    }
}
```

---

### Option 3: WebAssembly (WASM)

**Description:** Compile scripts from any language (Rust, C++, AssemblyScript) to WASM and run in a sandboxed VM.

**Pros:**
- Near-native performance
- Strong security sandbox
- Multi-language support (Rust, C++, AssemblyScript, etc.)
- Growing ecosystem (`wasmtime`, `wasmer`)
- Deterministic (useful for replays, networked games)

**Cons:**
- Complex integration (API boundaries, memory management)
- No hot-reload out of the box (requires infrastructure)
- Larger runtime overhead than Lua
- Harder debugging experience
- Compile step required (not as "scripty")

**Best for:** Performance-critical logic, multi-language modding, sandboxed user content

**Example:**
```rust
// scripts/physics_modifier.rs (compiled to WASM)
#[no_mangle]
pub extern "C" fn on_collision(entity_a: u32, entity_b: u32) {
    if is_player(entity_a) && is_enemy(entity_b) {
        apply_damage(entity_a, 10.0);
    }
}
```

---

### Option 4: Custom DSL (Domain-Specific Language)

**Description:** Create a custom language tailored to Rust4D's specific needs.

**Pros:**
- Perfect fit for domain (4D-specific constructs)
- Complete control over features
- Can be very simple and focused
- Educational value

**Cons:**
- Huge time investment (language design, parser, interpreter)
- No ecosystem or tooling
- Maintenance burden
- Reinventing the wheel

**Best for:** Very specific needs that existing languages can't handle (unlikely)

**Verdict:** Not recommended unless there's a compelling reason Lua/Rhai can't work.

---

## Recommended Approach: Lua + `mlua`

**Primary choice:** Lua via the `mlua` crate

**Reasoning:**
1. **Proven track record** - Lua is the industry standard for game scripting
2. **Performance** - LuaJIT is extremely fast (critical for per-frame logic)
3. **Community** - Huge ecosystem, tutorials, and developers familiar with it
4. **Tooling** - Language servers, debuggers, and editors exist
5. **Ergonomics** - Simple, expressive syntax for gameplay code
6. **Integration** - `mlua` provides excellent Rust bindings

**Fallback:** If Lua's weak typing becomes painful, consider Rhai as an alternative.

---

## API Design: What to Expose to Scripts

### Core Principles
1. **Minimal API surface** - Only expose what's necessary
2. **Read-only where possible** - Reduce risk of scripts breaking engine state
3. **Use handles, not raw pointers** - Scripts get `EntityHandle`, not `&mut Entity`
4. **Event-driven** - Scripts respond to hooks, don't drive the main loop

### Proposed Script API

```lua
-- Entity Management
entity = world:get_entity(handle)
entity:set_position(Vec4.new(x, y, z, w))
entity:apply_force(force)
entity:set_material(color)

-- Queries
player = world:find_by_name("player")
enemies = world:find_by_tag("enemy")

-- Input (read-only)
if input:is_key_down("W") then ... end
delta_x, delta_y = input:mouse_delta()

-- Physics
entity:apply_impulse(impulse)
entity:set_velocity(velocity)

-- Spatial
distance = entity:distance_to(other)
direction = (target.position - entity.position):normalized()

-- Lifecycle Hooks
function on_spawn(entity) ... end
function on_update(entity, dt) ... end
function on_collision(entity, other, normal) ... end
function on_destroy(entity) ... end
```

### What NOT to Expose
- Direct access to GPU resources
- Mutable references to physics world internals
- File system access (unless explicitly sandboxed)
- Network sockets
- Arbitrary memory access

---

## Script Lifecycle

### 1. Load Phase
```rust
// Load script from file
let script = ScriptSystem::load("scripts/player.lua")?;

// Attach to entity
entity.attach_script(script);
```

### 2. Initialization
```lua
-- Called once when entity spawns
function on_spawn(entity)
    entity.health = 100
    entity.speed = 5.0
end
```

### 3. Update Loop
```lua
-- Called every frame
function on_update(entity, dt)
    -- Game logic here
end
```

### 4. Event Handling
```lua
-- Called on collision
function on_collision(entity, other, normal)
    if other:has_tag("enemy") then
        entity.health = entity.health - 10
    end
end
```

### 5. Cleanup
```lua
-- Called before entity is destroyed
function on_destroy(entity)
    -- Cleanup logic
end
```

---

## Hot-Reload Implementation

### Approach: File Watcher + Script Reload

```rust
// Pseudo-code for hot-reload system
struct ScriptSystem {
    lua: mlua::Lua,
    scripts: HashMap<ScriptId, LoadedScript>,
    watcher: notify::RecommendedWatcher,
}

impl ScriptSystem {
    fn watch_directory(&mut self, path: &Path) {
        self.watcher.watch(path, RecursiveMode::Recursive);
    }

    fn on_file_changed(&mut self, path: &Path) {
        // Reload the script
        if let Some(script) = self.scripts.get_by_path(path) {
            match script.reload() {
                Ok(_) => log::info!("Reloaded: {:?}", path),
                Err(e) => log::error!("Reload failed: {}", e),
            }
        }
    }
}
```

### Challenges
1. **State preservation** - How to keep entity state when script reloads?
   - Solution: Serialize script-local state to Lua table, reload script, restore state

2. **Function references** - Callbacks become invalid on reload
   - Solution: Re-register all callbacks after reload

3. **Error handling** - Script errors shouldn't crash the engine
   - Solution: Catch all Lua errors, log them, keep old script version running

---

## Security Considerations

### Sandboxing Strategy

Lua has a sandbox system - limit what scripts can access:

```lua
-- Safe sandbox: only allow whitelisted APIs
local safe_env = {
    -- Math functions
    math = math,
    -- String functions
    string = string,
    -- Whitelisted engine API
    world = world,
    entity = entity,
    input = input,
    Vec4 = Vec4,
    -- NO file I/O, no os.execute, no require
}

-- Run script in sandboxed environment
lua.load(script_code).set_environment(safe_env).exec()
```

### Threat Model

| Threat | Mitigation |
|--------|------------|
| Infinite loops | Execution time limits (kill after N instructions) |
| Memory exhaustion | Memory quotas per script |
| File system access | Remove `io` library from sandbox |
| Code injection | Validate script source, never eval user strings |
| Save game tampering | Sign save files, validate on load |

### User-Generated Content

For mods/UGC, additional precautions:
- Run in separate process (IPC communication)
- Stricter time/memory limits
- No access to other mods' data
- Code review for published mods

---

## Performance Considerations

### Lua Performance Profile
- **Interpreted mode:** ~10-50x slower than Rust
- **LuaJIT mode:** ~2-5x slower than Rust (sometimes faster for numeric code!)
- **Function calls:** Expensive (avoid calling Rust from Lua in tight loops)

### Optimization Strategies

1. **Batch API calls**
   ```lua
   -- BAD: Call Rust 100 times
   for i = 1, 100 do
       entity:apply_force(Vec4.new(0, 1, 0, 0))
   end

   -- GOOD: One call with batch data
   entity:apply_forces(force_list)
   ```

2. **Cache frequently accessed data**
   ```lua
   -- BAD: Query every frame
   function on_update(entity, dt)
       local player = world:find_by_name("player")
   end

   -- GOOD: Cache on spawn
   local player = nil
   function on_spawn(entity)
       player = world:find_by_name("player")
   end
   ```

3. **Limit script count**
   - Not every entity needs a script
   - Use scripts for complex behaviors, not simple physics

4. **Profile and measure**
   - Add timing instrumentation to script calls
   - Log slow scripts (> 1ms per frame)

---

## Phased Implementation Plan

### Phase 1: Foundation (2 sessions)
**Goal:** Get basic Lua integration working

- Add `mlua` dependency
- Create `ScriptSystem` struct
- Load and execute a simple script
- Expose one simple API function (`print_message`)
- Write integration test

**Success criteria:** Can load and run "Hello World" script

---

### Phase 2: Entity API (2 sessions)
**Goal:** Scripts can query and modify entities

- Expose `Entity` handles to Lua
- Implement `get_position`, `set_position`
- Implement `find_by_name`, `find_by_tag`
- Add `on_update(entity, dt)` hook
- Write example script that moves an entity

**Success criteria:** Script can find player and follow it

---

### Phase 3: Input and Physics (1 session)
**Goal:** Scripts can read input and apply forces

- Expose input state (keyboard, mouse)
- Expose physics API (`apply_force`, `apply_impulse`)
- Add collision callback: `on_collision(entity, other)`

**Success criteria:** Script-controlled entity responds to WASD keys

---

### Phase 4: Hot-Reload (1-2 sessions)
**Goal:** Scripts reload without restarting engine

- Integrate `notify` crate for file watching
- Implement script reload logic
- Handle reload errors gracefully
- Test state preservation across reloads

**Success criteria:** Edit script file, see changes in-game without restart

---

### Phase 5: Sandboxing and Safety (1 session)
**Goal:** Scripts can't crash the engine or access unsafe APIs

- Remove dangerous Lua libraries (`io`, `os`, `debug`)
- Add execution time limits
- Add memory limits
- Wrap all script calls in error handlers

**Success criteria:** Infinite loop script kills itself, not the engine

---

### Phase 6: Polish and Documentation (1 session)
**Goal:** System is usable by others

- Write scripting API documentation
- Create example scripts (enemy AI, collectibles, triggers)
- Add error messages with line numbers
- Performance profiling tools

**Success criteria:** New contributor can write a script without asking questions

---

## Dependencies

### New Crates
```toml
[dependencies]
mlua = { version = "0.9", features = ["lua54", "serialize"] }
notify = "6.0"  # File watching for hot-reload
```

### Architecture Dependencies
- **Required:** Phase 1-2 of engine roadmap (scene management stable)
- **Recommended:** Phase 4 (clean system architecture)
- **Blocks:** Nothing (optional feature)

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| API instability breaks scripts | High | High | Version script API, migrate tool |
| Performance bottleneck | Medium | Medium | Profile early, limit script usage |
| Security vulnerabilities | Medium | High | Thorough sandboxing, code review |
| Hot-reload bugs | High | Low | Graceful fallback to old version |
| Scope creep (adding too many features) | High | Medium | Stick to minimal API, defer extensions |
| Lua dependency issues | Low | Medium | Pin `mlua` version, test builds |

---

## Success Criteria

After full implementation:

1. **Load script from file** - `ScriptSystem::load("path/to/script.lua")` works
2. **Attach to entity** - Entities can have scripts
3. **Lifecycle hooks work** - `on_spawn`, `on_update`, `on_collision`, `on_destroy` all fire
4. **Entity API works** - Scripts can query and modify entities
5. **Input API works** - Scripts can read keyboard/mouse state
6. **Physics API works** - Scripts can apply forces/impulses
7. **Hot-reload works** - Editing script file updates behavior without restart
8. **Sandboxing works** - Malicious scripts can't crash engine or access file system
9. **Error handling works** - Script errors are logged but don't crash engine
10. **Documentation exists** - API reference and examples are available

---

## Trigger Conditions: When to Start This Work

Do NOT start scripting system until:

1. **Core engine is stable** - Physics, rendering, and scene management work reliably
2. **Phase 1-3 complete** - Scene serialization and management are done
3. **API is stable** - Entity/World API has settled and isn't changing often
4. **There's a need** - We have actual gameplay code that would benefit from scripting
5. **Willow approves** - This is a significant feature addition

Indicators that scripting is needed:
- "I wish I could test this gameplay idea without recompiling"
- "This behavior tree is getting unwieldy in Rust"
- "I want to add modding support"
- "Iteration time is slowing down development"

---

## Alternatives Considered

### Alternative 1: No Scripting
**Pros:** Simpler, everything in Rust, better performance
**Cons:** Slow iteration, high barrier to entry
**Verdict:** Valid for early engine, not scalable long-term

### Alternative 2: Rust Hot-Reload (dynamic library loading)
**Pros:** Native performance, same language
**Cons:** Extremely complex, platform-specific, limited API
**Verdict:** Interesting but fragile

### Alternative 3: Visual Scripting (node graphs)
**Pros:** Designer-friendly, no coding required
**Cons:** Massive implementation effort, limited expressiveness
**Verdict:** Defer to Phase 8+ (future visual editor)

---

## References

- [mlua documentation](https://docs.rs/mlua/)
- [Lua 5.4 Reference Manual](https://www.lua.org/manual/5.4/)
- [Rhai Book](https://rhai.rs/book/)
- [WebAssembly in Game Engines](https://surma.dev/things/js-to-asc/)
- [Scripting Best Practices - GDC Talk](https://www.gdcvault.com/play/1025380/)
- [Love2D](https://love2d.org/) - Example of Lua-based game framework

---

## Next Steps (When Ready to Implement)

1. Read the `mlua` documentation thoroughly
2. Prototype minimal integration (Phase 1)
3. Design the Entity API surface carefully
4. Write comprehensive tests for each phase
5. Get feedback from Willow on API ergonomics
6. Document as you go, not at the end

---

**Remember:** This is a FUTURE plan. Do not start work until core engine features are complete and stable. Scripting is powerful but adds significant complexity - make sure it's actually needed before investing 6-8 sessions into it.
