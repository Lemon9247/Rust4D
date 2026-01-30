# Swarm B Synthesis: Roadmap Feasibility Review
**Date**: 2026-01-30
**Agents**: B1 (ECS & Rendering Plans), B2 (Scripting, Editor & Networking Plans)

---

## Executive Summary

The five long-term roadmap plans were written when the engine was younger. Now that Phases 1-5 are complete and the goal is a 4D boomer shooter, the priority order has shifted significantly. The most impactful finding is that **the Visual Editor should be elevated from P7 to P5-P6** because level design is the #1 differentiator for boomer shooters, and 4D levels cannot be designed in text. A secondary finding is that **runtime state serialization is a cross-cutting gap** blocking the editor, networking, and save systems.

---

## Plan-by-Plan Assessment

### ECS Migration (originally P6, 8-12 sessions)

**Status**: 1 of 2 required triggers met (project maturity/stability). Feature triggers will fire rapidly once boomer shooter development begins (11+ archetypes, 15+ component types needed).

**Key findings**:
- Entity struct is still clean (7 fields, 2 Optional) -- NOT bloated yet
- A boomer shooter needs 11 entity archetypes and 15+ component types -- the monolithic Entity WILL become painful
- hecs remains the right ECS choice (no better alternatives have emerged)
- Revised effort: **11-13 sessions** (up from 10-11 due to Phase 5 additions: hierarchy, scene infrastructure)

**Recommendation: Partial ECS (2-3 sessions)** -- add `ComponentStore<EntityKey, T>` for gameplay components while keeping Entity for rendering data. This buys extensibility without the full migration cost. Defer full ECS until partial ECS proves insufficient.

### Advanced Rendering (originally P6, 8-10 sessions)

**Status**: Clean two-stage pipeline. Single-pass only. No shadows, no PBR, no post-processing, no textures.

**Key findings**:
- `render()` already takes a parameterized texture view -- moderate difficulty to add multi-pass
- Shader system is 2 hardcoded WGSL files via `include_str!()` -- no hot-reload or variants
- PBR/SSAO/advanced post-processing are NOT needed for boomer shooter aesthetics
- The wireframe/normal shaders mentioned in the plan do NOT exist in current code

**Boomer shooter rendering needs (6-7 sessions)**:
1. Weapon viewmodel + HUD (2 sessions) -- HIGHEST priority
2. Particle system (2 sessions) -- blood, sparks, explosions
3. Multiple lights (1 session) -- muzzle flash, explosion lighting
4. Polish: FXAA, bloom, shadows (1-2 sessions)

**Critical dependency**: RenderableGeometry depends on Entity/World API. Do ECS (partial or full) before major rendering refactor to avoid double work.

### Scripting (originally P6, 6-8 sessions)

**Status**: Only 1 of 5 triggers met. Entity API is not stable (ECS pending).

**Key findings**:
- Classic boomer shooter AI is simple state machines -- doable in Rust without scripting
- A declarative RON trigger system covers 80% of level scripting needs (1-2 sessions)
- If scripting is needed later, **Rhai > Lua** for a solo Rust developer
- Hot-reload of levels (already partially supported) is more valuable than hot-reload of scripts

**Recommendation**: Keep at P6. Build a declarative trigger system (1-2 sessions) before full scripting. Only pursue full scripting if iteration pain is genuinely felt.

### Visual Editor (originally P7, 10-15 sessions)

**Status**: 1-2 of 5 prerequisites met. But the demand condition is the most compelling -- boomer shooters live and die by level design.

**Key findings**:
- 4D levels cannot be designed in text editors
- The editing simplifies for boomer shooters: mostly 3D-per-slice with W-portal connections
- egui remains the right UI framework choice
- TrenchBroom integration is compelling but fundamentally outputs 3D geometry (not 4D)

**Recommendation: Elevate to P5-P6.** Build in two phases:
1. RON preview tool with hot-reload (2-3 sessions) -- cheap, high value
2. Minimal egui editor (6-8 sessions) -- entity list, property inspector, W-slice navigation, 3D viewport

### Networking (originally P8, 15-20 sessions)

**Status**: Significant gaps in codebase readiness.

**Key findings**:
- No deterministic physics (variable dt, no fixed timestep)
- No state snapshot capability (runtime World/PhysicsWorld cannot serialize)
- No stable entity IDs for network boundaries
- 4D bandwidth overhead (70%) is manageable (<50 KB/s for 50 entities with delta compression)
- Must be post-single-player

**Recommendation**: Keep at P8. Lay groundwork now: add Serialize/Deserialize to Rotor4, implement fixed timestep. These benefit the engine regardless.

---

## Cross-Plan Dependencies

### The ECS Decision Gates Everything

All three feature plans (scripting, editor, networking) assume a stable entity API. An ECS migration would invalidate:
- Scripting API design (entirely different entity access patterns)
- Editor property inspector (different component structure)
- Networking replication strategy (component-based vs monolithic entity)

**Decision must be made before starting any of these features.**

### Runtime State Serialization Is a Cross-Cutting Gap

The engine can serialize *templates* (Scene, EntityTemplate) but cannot serialize *runtime state* (World, ActiveScene, PhysicsWorld, RigidBody4D). Additionally, Rotor4 lacks `Serialize`/`Deserialize` (manual workaround exists in transform.rs). This blocks:
- Editor save (modified runtime state back to file)
- Networking snapshots
- Save game systems
- Replay systems

**Recommendation**: Invest 1-2 sessions fixing this BEFORE editor/scripting/networking.

### Recommended Sequencing

```
NOW:
  Fix serialization gap (1-2 sessions)
  Partial ECS (2-3 sessions)

NEXT:
  Build core gameplay in Rust (5-10 sessions)
  RON preview tool with hot-reload (2-3 sessions)

LATER:
  Minimal egui editor (6-8 sessions)
  Declarative trigger system (1-2 sessions)

MUCH LATER:
  Full scripting if needed (6-8 sessions)
  Networking (15-20 sessions)
```

---

## Revised Priority Ranking

| Rank | Feature | Original Priority | Revised Priority | Reasoning |
|------|---------|-------------------|------------------|-----------|
| 1 | Serialization fix | Not planned | P5 | Cross-cutting blocker |
| 2 | Partial ECS | Part of ECS plan | P5 | Unblocks gameplay components |
| 3 | Visual Editor | P7 | P5-P6 | Level design is #1 for boomer shooters |
| 4 | Scripting | P6 | P6 (unchanged) | Trigger system covers most needs |
| 5 | Full ECS | P6 | P6-P7 (deferred) | Partial ECS buys time |
| 6 | Full Rendering | P6 | Split (FPS subset now, rest later) | Different features for boomer shooter |
| 7 | Networking | P8 | P8 (unchanged) | Post-single-player |

---

## Source Reports
- [B1: ECS & Rendering Feasibility](./b1-ecs-rendering.md)
- [B2: Scripting, Editor & Networking Feasibility](./b2-scripting-editor-networking.md)
