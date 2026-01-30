# Hive-Mind: Swarm B - Roadmap Feasibility Review
**Date**: 2026-01-30

## Shared Context
The Rust4D roadmap has 5 long-term plans written when the engine was younger:
- **ECS Migration** (P6, 8-12 sessions) - hecs-based migration from monolithic Entity
- **Advanced Rendering** (P6, 8-10 sessions) - Multi-pass, PBR, post-processing, shadows
- **Scripting** (P6, 6-8 sessions) - Lua via mlua for gameplay scripting
- **Visual Editor** (P7, 10-15 sessions) - egui-based scene editor
- **Networking** (P8, 15-20 sessions) - Quinn-based client-server multiplayer

Since these plans were written, Phases 1-5 have been completed. The engine now has:
- Asset management with hot reload
- Entity hierarchy with parent-child
- Scene transitions, async loading, validation
- Architecture refactored (systems extracted from main.rs)

### Review Focus
For each plan, assess:
1. **Still relevant?** Has the engine outgrown the plan's assumptions?
2. **Feasibility**: What changed that makes it easier/harder?
3. **Priority for boomer shooter**: How critical is this for a Doom-like 4D FPS?
4. **Blockers**: What must happen first?
5. **Effort re-estimate**: Has the estimate changed?

## Agent Discoveries
(Agents: write key findings here for cross-pollination)

### B1 (ECS & Rendering) - Key Findings

**ECS:**
- Entity struct is still clean: 7 fields, 2 Optional. NOT bloated yet.
- Only 1 of 2 required ECS triggers is met (stability). Feature triggers will fire once boomer shooter dev starts.
- A boomer shooter needs 11+ entity archetypes and 15+ component types -- monolithic Entity WILL become painful.
- hecs remains the right choice. No better alternatives have emerged.
- Revised ECS effort: 11-13 sessions (up from 10-11 due to Phase 5 additions: hierarchy, scene infra).
- RECOMMENDATION: Do "Partial ECS" (2-3 sessions) -- add ComponentStore<EntityKey, T> for gameplay components while keeping Entity for rendering. Defer full migration.

**Rendering:**
- Pipeline is clean two-stage: compute (4D slice) -> render (3D with lighting). Single-pass only.
- NO post-processing, NO shadow mapping, NO PBR. Material is just RGBA color.
- Multi-pass is moderately easy to add -- render() already takes parameterized texture view.
- Shader system is 2 hardcoded WGSL files via include_str!(). No hot-reload, no variants.
- Plan mentions wireframe/normal shaders as existing -- they are NOT in the current code.
- W-depth coloring works but has only one hardcoded color scheme (blue-gray-red).

**Boomer Shooter Rendering Needs (6-7 sessions):**
1. Weapon viewmodel + HUD (2 sessions) -- HIGHEST priority
2. Particle system (2 sessions) -- needed for blood, sparks, explosions
3. Multiple lights (1 session) -- muzzle flash, explosions
4. Polish: FXAA, bloom, shadows (1-2 sessions)
- PBR, SSAO, advanced post-processing are NOT needed for boomer shooter style.

**Critical Cross-Plan Dependency:**
- RenderableGeometry depends directly on Entity/World API (from_world iterates entities).
- ECS migration changes this API. Do ECS before major rendering refactor to avoid double work.
- Recommended order: Partial ECS -> FPS rendering -> Gameplay -> Full ECS (if needed)

### B2 (Scripting, Editor & Networking) - Key Findings

**Visual Editor should be elevated to P5-P6.**
- Level design is the #1 differentiator for boomer shooters. 4D levels cannot be designed in text.
- Recommend minimal egui editor (entity list, property inspector, W-slice nav, 3D viewport) in 6-8 sessions.
- 4D editing simplifies for boomer shooters: mostly 3D-per-slice with W-portal connections between slices.
- Near-term: RON preview tool with hot-reload (2-3 sessions) gives fast iteration cheaply.

**Runtime state serialization is a cross-cutting gap.**
- Templates serialize (Scene, EntityTemplate) but runtime state does NOT (World, ActiveScene, PhysicsWorld, RigidBody4D).
- Rotor4 lacks Serialize/Deserialize (manual workaround in transform.rs).
- This blocks: editor save, networking snapshots, save games, replays.
- Recommend 1-2 sessions fixing this BEFORE editor/scripting/networking.

**Scripting is less urgent for boomer shooters.**
- Classic boomer AI is simple state machines (doable in Rust).
- Declarative RON trigger system covers 80% of level scripting needs (1-2 sessions).
- Full scripting (6-8 sessions) should wait for actual iteration pain. Rhai > Lua for solo Rust dev.

**Networking has significant gaps in codebase readiness.**
- No deterministic physics (variable dt, no fixed timestep).
- No state snapshot capability. No stable entity IDs for network boundaries.
- 4D bandwidth overhead (70%) manageable with compression (<50 KB/s for 50 entities).
- Must be post-single-player. 15-20 session estimate still accurate.

**ECS decision gates everything (agrees with B1).**
- All three plans assume stable entity API. ECS migration invalidates scripting API, editor property inspector, and networking replication.
- DECIDE on ECS before starting any of these three features.

**Priority for boomer shooter: Editor > Scripting > Networking.**
