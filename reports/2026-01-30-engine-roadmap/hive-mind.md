# Hive Mind: Engine Roadmap Planning

## Task Overview
Build a new engine roadmap for Rust4D that accounts for the engine/game split decision. The cross-swarm synthesis (from the multi-engine review) outlined 6 phases of development assuming a single-repo approach. Now that the project will be split into:

1. **Rust4D Engine** -- generic 4D game engine library
2. **Rust4D Shooter** -- separate game repo for the 4D boomer shooter

...each phase needs to be re-evaluated. For each phase, an agent determines:
- What belongs in the **engine** (generic, reusable)
- What belongs in the **game** (boomer-shooter-specific)
- What the engine must expose as API for the game to build upon
- Detailed implementation plan for the engine side
- Dependencies on other phases and the engine/game split plan

## Key Context
- **Engine/Game Split Plan**: `scratchpad/plans/2026-01-30-engine-game-split.md`
  - Full ECS migration with hecs (decided, not partial)
  - New rust4d_game crate for CharacterController4D, events, FSM
  - Input refactored to action/axis abstraction
  - Git URL hybrid dependency approach
- **Cross-Swarm Synthesis**: `scratchpad/reports/2026-01-30-multi-engine-review/cross-swarm-synthesis.md`
  - 12 P0 gaps identified
  - 5 features requiring 4D adaptation
  - Foundation + 5 phases + deferred phase
- **Original Agent Reports**: `scratchpad/reports/2026-01-30-multi-engine-review/`

## Important Constraint
The engine/game split plan covers ECS migration and the split itself (9.5-14 sessions). That work is ALREADY PLANNED. Each agent here should:
1. ASSUME the split plan is complete (ECS done, rust4d_game exists, game repo exists)
2. Plan what the ENGINE needs AFTER the split to support their phase's features
3. Be clear about what APIs the engine exposes vs what the game implements

## Agents
1. **Agent F** (Foundation) - Fixed timestep + serialization + quick fixes. Reviews what's already in the split plan vs what's additional.
2. **Agent P1** (Combat Core) - Raycasting, event system, health/damage, trigger callbacks. Engine-side APIs.
3. **Agent P2** (Weapons & Feedback) - Audio system, HUD framework, particle system. Engine-side rendering/audio.
4. **Agent P3** (Enemies & AI) - Sprite/billboard rendering, AI framework, area damage. Engine-side support.
5. **Agent P4** (Level Design Pipeline) - RON preview tool, shape types, door/elevator mechanics. Engine-side tooling.
6. **Agent P5** (Editor & Polish) - egui editor, lighting, textures, input rebinding. Engine-side editor/rendering.

## Coordination Notes
- Each agent writes their plan to `agent-[f|p1|p2|p3|p4|p5]-report.md` in this folder
- The Foundation agent (F) identifies prerequisites that other agents should note as dependencies
- All agents should reference the engine/game split plan when deciding engine vs game boundaries
- Agents should check hive-mind for cross-phase dependencies

## Cross-Phase Dependencies to Watch
- Foundation items (serialization, fixed timestep) block almost everything
- Raycasting (P1) is needed by weapons (P2) and enemy AI (P3)
- Event system (P1) is needed by weapons (P2), pickups (P4), and triggers (P4)
- Sprite rendering (P3) informs particle system (P2/P3 overlap)
- Audio system (P2) used by weapons (P2), enemies (P3), and doors (P4)
- Editor (P5) needs all shape types from P4

## Questions for Discussion
(Agents add questions here for other agents to answer)

### From Agent P3:
1. **For P2**: The particle system is needed by BOTH weapons (muzzle flash, impact sparks) and enemies (blood, explosions). I've designed it as part of P3's scope since enemies are the more comprehensive consumer. Are you planning a separate particle system, or should we share my design? Let's coordinate so we don't build two.
2. **For P2**: My sprite billboard pipeline and your HUD pipeline both need additional render passes after the main 4D slice pass. Render pass ordering should be: (1) 4D slice geometry, (2) sprites/billboards, (3) particles, (4) HUD overlay. Does this match your HUD design?
3. **For P1**: Confirmed -- I will use `PhysicsWorld::raycast()` for LOS checks via a `line_of_sight()` wrapper. My design stubs LOS as `true` until your raycasting is ready. The `layer_mask` approach works perfectly for AI sight.
4. **Answer to P1.3**: Using `CollisionLayer::STATIC` for LOS makes sense. For enemies that use W-phasing, the AI will check LOS considering W-distance attenuation, not just geometric occlusion.

### From Agent F:
1. **For P1**: Raycasting should use fixed timestep. Does your plan assume fixed timestep is done?
2. **For P5**: Editor needs full physics type serialization (8+ types). Recommend deferring to split plan Phase 2. Compatible with your timeline?
3. **For ALL**: Rotor4 serialization fix changes RON format from `[f32; 8]` arrays to struct fields `{ s: 1.0, b_xy: 0.0, ... }`. Existing scene RON files will need re-export. Be aware.

### From Agent P1:
1. **Answer to F.1**: Raycasting itself does NOT need fixed timestep -- it's an instantaneous query, not a simulation step. However, trigger enter/exit detection benefits from fixed timestep because variable dt can cause missed overlaps. Plan does not REQUIRE fixed timestep but works better with it.
2. **For P2**: The engine-side event system is collision event DATA (`CollisionEvent` structs from `drain_collision_events()`), not a general event bus. Weapons should consume these events directly from physics, or through the game-level EventBus in `rust4d_game`. Audio triggers would come from game events, not engine events.
3. **For P3**: `PhysicsWorld::raycast()` is the API for enemy line-of-sight. The `layer_mask` parameter lets you filter what the ray hits (e.g., `CollisionLayer::STATIC | CollisionLayer::PLAYER` for LOS checks that see through enemies).
4. **For P4**: Trigger zones need `CollisionFilter::trigger(detects)` on static colliders. Pickup triggers should use `CollisionFilter::trigger(CollisionLayer::PLAYER)`. The game reads `TriggerEnter` events from `drain_collision_events()`.
5. **BUG FOUND**: The current trigger system is non-functional -- `CollisionFilter::player()` excludes TRIGGER from its mask, and the symmetric `collides_with()` means triggers never detect players. My plan fixes this with an asymmetric trigger detection pass.

## Status
- [x] Agent F (Foundation): COMPLETE
- [x] Agent P1 (Combat Core): COMPLETE
- [x] Agent P2 (Weapons & Feedback): COMPLETE
- [x] Agent P3 (Enemies & AI): COMPLETE
- [x] Agent P4 (Level Design Pipeline): COMPLETE
- [x] Agent P5 (Editor & Polish): COMPLETE
- [ ] Final synthesis: Pending

## Reports Generated
- `agent-f-report.md` - Foundation phase implementation plan (Agent F, 2026-01-30)
- `agent-p1-report.md` - Combat Core engine implementation plan (Agent P1, 2026-01-30)
- `agent-p3-report.md` - Enemies & AI engine implementation plan (Agent P3, 2026-01-30)
- `agent-p5-report.md` - Editor & Polish engine implementation plan (Agent P5, 2026-01-30)
- `agent-p2-report.md` - Weapons & Feedback engine implementation plan (Agent P2, 2026-01-30)
- `agent-p4-report.md` - Level Design Pipeline engine implementation plan (Agent P4, 2026-01-30)

## Key Findings

### Agent F (Foundation):
- **Foundation is ~1-1.5 sessions, not 2.** The synthesis overestimated because it included Partial ECS (now superseded by full ECS in split plan).
- **Rotor4 serialization is the only blocking item.** It's a prerequisite for ECS component serialization. The fix is trivial (add derives) but has a RON format breaking change.
- **Transform4D has a manual serialization workaround** (rotor4_serde module in transform.rs) that should be removed after the Rotor4 fix.
- **Physics type serialization is a cascade of ~8 types** (not just 2 as the synthesis said), but it can be deferred to Phase 2 of the split plan.
- **Fixed timestep is completely absent.** Physics is frame-rate dependent. Accumulator pattern needed in PhysicsWorld.
- **Diagonal movement is 41-73% faster** due to un-normalized movement direction. In 4D this is worse than 3D (3 movement axes = sqrt(3) speed multiplier).
- **Back-face culling was disabled for debugging** and never re-enabled. May reveal winding order issues in the compute shader.
- **All foundation items should be done BEFORE ECS migration** -- they clean up the codebase the ECS work will touch.

### Agent P2 (Weapons & Feedback):
- **Engine-side estimate: 4.5-5.5 sessions** (audio 1.5-2, HUD/egui 1, particles 1.5-2, screen effects 0.5).
- **New crate: `rust4d_audio`** wrapping kira (not rodio). Kira wins on spatial audio, tweens, mixer/tracks, and game-focused design.
- **4D spatial audio** projects 4D listener/emitter positions onto kira's 3D spatial system, using 4D Euclidean distance for attenuation. W-distance filtering provides "hearing through dimensions" effect.
- **HUD via egui-wgpu overlay** in `rust4d_render`. Adds an `OverlayRenderer` that draws egui after the 3D scene. Game builds all specific HUD widgets. This front-loads the egui dependency that Phase 5 editor will need anyway.
- **Particles are 3D, not 4D.** Particles exist in the sliced output space, not in pre-slice 4D. CPU-simulated (hundreds of particles, not millions) with GPU-rendered billboards.
- **Render pass ordering confirmed**: (1) 4D slice compute, (2) 3D cross-section render, (3) sprites/billboards (P3), (4) particles, (5) egui overlay.
- **Screen shake is game-side** (camera offset, not post-processing). Lives in `rust4d_game` as `ScreenShake` struct.
- **Damage flash via egui overlay** -- no post-processing pipeline needed for Phase 2.
- **Weapon system is 100% game-side.** Engine provides no weapon abstractions.
- **Risk**: egui-winit version must be compatible with workspace `winit = "0.30"`. Needs verification.
- **Answer to P3.1**: Particle system is in `rust4d_render`. Single shared system for both weapons and enemies. Use `ParticleSystem::spawn_burst()`.
- **Answer to P3.2**: Confirmed render order matches P3's proposal. Sprites go between cross-section and particles.
- **Answer to P1.2**: Agreed. Audio triggers come from game events, not engine collision events.

### Agent P1 (Combat Core):
- **Engine-side estimate: 1.75 sessions** (down from original 3.5 because health/damage is purely game work).
- **Trigger system design bug found**: `CollisionFilter::trigger()` and `CollisionFilter::player()` are incompatible -- the symmetric `collides_with()` check means triggers NEVER detect players. Fix: asymmetric trigger overlap detection pass in `step()`.
- **Health/Damage is 100% game-side.** The engine provides collision events and raycasting; the game defines Health components and damage logic.
- **Engine needs collision event DATA, not an event BUS.** `PhysicsWorld` accumulates `CollisionEvent` structs during `step()` and exposes `drain_collision_events()`. The event bus belongs in `rust4d_game`.
- **Raycasting split**: `Ray4D` struct in `rust4d_math` (geometric primitive), ray-shape intersections + world raycast in `rust4d_physics`.
- **Vec4 gaps**: missing `distance()`, `distance_squared()`, and `f32 * Vec4` operator. Should fix alongside raycasting.
- **`sphere_vs_sphere` is private** on PhysicsWorld but should be a public standalone function like the other collision tests.
- **Parallelism**: Raycasting (ray math + world queries) and collision events (trigger detection + enter/exit tracking) can be implemented in parallel by different agents.

### Agent P3 (Enemies & AI):
- **Engine estimate: 4 sessions** (down from original 4.5 because enemy types and specific AI behaviors are 100% game work).
- **Sprites are NOT 4D geometry.** They are 3D billboard quads rendered at the 3D projection of a 4D position, with W-distance fade. They bypass the compute-shader slicing pipeline entirely and use a separate render pass sharing the depth buffer.
- **Two-pass rendering expansion needed.** The sprite pipeline is a NEW wgpu render pipeline alongside the existing slice pipeline. Render order: geometry -> sprites -> particles -> HUD.
- **Particle system overlaps with P2.** Both weapons (muzzle flash) and enemies (blood, explosions) need particles. P3 designed the comprehensive particle system; P2 should coordinate on shared API.
- **Spatial queries are simple iteration.** `query_sphere` iterates all bodies checking 4D distance. O(n) is fine for boomer shooter enemy counts (20-50).
- **FSM is intentionally minimal.** StateMachine<S> is ~30 lines of code. All AI logic is game-side.
- **4D explosions cover MORE volume.** Hypersphere volume scales as R^4 -- deliberate gameplay advantage for explosive weapons countering W-phasing enemies.
- **All three Wave 2 items (sprites, particles, physics queries) can run in parallel.** Critical path is 1.5 sessions.
- **Depth buffer sharing is the key integration point.** `RenderPipeline::ensure_depth_texture()` needs to expose the depth buffer for sprites and particles.

### Agent P5 (Editor & Polish):
- **Revised estimate: 10-12.5 sessions total** (8-10 critical path). Texture support is harder than the synthesis's 1-session estimate.
- **Texture support in 4D is the hardest sub-task.** The compute shader has no UV path. Recommended: triplanar mapping first (no compute shader changes), defer UV-through-pipeline to later.
- **Editor is a new `rust4d_editor` crate.** Overlay via `EditorHost` trait. Games opt in; editor never takes over the event loop. Toggle with F12.
- **Point lights need W-distance attenuation.** Lights "bleed through" nearby W-slices, dimming with W-distance. Beyond `w_range`, invisible.
- **Shadows work on already-sliced 3D geometry.** No 4D shadow math needed. Standard directional shadow mapping.
- **Input rebinding is 80% planned** by the split plan's action/axis abstraction. Engine adds `InputMap::rebind()` + TOML persistence.
- **Editor has the deepest dependency chain** of any feature -- needs ECS, serialization, all shape types, working renderer.
- **W-slice navigation thumbnails** are the killer editor feature for 4D level design. Render scene at multiple W values as small previews.
- **Phase 5 sub-features are internally parallel:** lights, textures, and input rebinding are independent. Editor framework starts after or alongside.
- **Answer to F.2**: Yes, deferring physics type serialization to split plan Phase 2 is compatible.
- **For P3/P2**: Render pass ordering with editor: (1) 4D slice geometry, (2) sprites/billboards, (3) particles, (4) HUD overlay, (5) egui editor overlay (last).
- **For P2**: Point lights add bind group 1 to the main render pipeline. HUD/sprite passes use separate pipelines, no conflict.

### Agent P4 (Level Design Pipeline):
- **Engine-side estimate: 4.5 sessions** (original synthesis was 4-6; door/elevator/pickup game logic moves to game repo).
- **Only 2 renderable shape types exist**: `Tesseract4D` and `Hyperplane4D`. The most impactful gap is that `Tesseract4D` only supports equal-sided hypercubes -- need `Hyperprism4D` with independent X/Y/Z/W dimensions for walls, platforms, corridors.
- **`Hypersphere4D` needed**: collision `Sphere4D` exists but has no renderable counterpart. Need icosphere-like tetrahedral decomposition for GPU slicing.
- **Hot-reload infrastructure already exists** in `AssetCache` (file modification time polling, reload detection, 35 tests). The RON preview tool extends this, not reinvents it. However, `Scene` does not implement the `Asset` trait.
- **RON preview tool should start as `examples/ron_preview.rs`**, promoted to `rust4d_tools` crate if/when egui overlay is added.
- **Declarative trigger system covers 80% of level scripting**: `TriggerDef` in RON with built-in actions (TweenPosition, GameEvent, DespawnSelf). `GameEvent(String)` is the escape hatch -- engine fires named event, game interprets it.
- **Tween/interpolation system belongs in `rust4d_game`**: `Tween<T>` with easing functions, `TweenManager` for entity property animation. Engine provides `Interpolatable` trait in `rust4d_math`.
- **Wave 1 (shape types) has zero dependencies** and can start immediately as a parallel task.
- **4D-specific insight**: W-layered rooms connected by W-portals are just trigger zones that tween the player's W-coordinate. No special engine support needed beyond triggers + tweens.

### From Agent P4:
1. **For P1**: Does the event system support string-named events? The declarative trigger system assumes `GameEvent(String)` which the game interprets. If P1's `CollisionEvent` is purely typed, we need an `AnyEvent` or `NamedEvent` variant in the `rust4d_game` EventBus.
2. **Answer to P1.4**: Confirmed. The declarative trigger data model defines zones + actions in RON. At runtime, the trigger system reads `TriggerEnter` from `drain_collision_events()` per your design.
3. **For P5**: The RON preview tool (my Wave 4) could serve as the foundation for the editor's viewport. Sharing camera/render code would avoid duplication.
4. **For ALL**: Shape type expansion (Wave 1) has no dependencies and could be done first by any available agent.
