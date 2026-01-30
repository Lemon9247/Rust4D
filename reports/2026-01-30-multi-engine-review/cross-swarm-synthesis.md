# Cross-Swarm Synthesis: Engine Review for 4D Boomer Shooter
**Date**: 2026-01-30

## Summary

Seven agents across three swarms reviewed the Rust4D engine from three angles: current codebase state, roadmap feasibility, and feature/genre analysis. Their findings converge on a clear picture: **Rust4D has a strong, well-tested foundation for 4D rendering and physics, but needs an entire gameplay layer built from scratch to become a boomer shooter.** The path to a playable demo is estimated at 12-18 sessions, with a full MVBS (Minimum Viable Boomer Shooter) at 15-25 sessions.

---

## Per-Swarm Results

### Swarm A: Codebase State (`swarm-a-codebase`)

**3 agents reviewed 5 crates (358 tests, ~10,000+ lines of source)**

The engine is well-architected with consistent 4/5 code quality across all crates. The 4D math (Rotor4 geometric algebra) and GPU hyperplane slicing are production-quality and represent the hardest technical problems already solved. The collision layer system was designed with an FPS in mind. Scene infrastructure (async loading, transitions, overlays, hot reload) is mature.

The codebase has zero gameplay systems. No raycasting, no audio, no HUD, no particles, no textures, no events, no health, no weapons, no enemies, no AI. The Entity struct is clean (7 fields) but not extensible -- adding gameplay features will require either modifying Entity or adopting ECS. Single-pass rendering with no shadows, textures, or post-processing limits visual capability.

**Overall FPS readiness: 2/5 -- strong platform, absent game layer.**

### Swarm B: Roadmap Feasibility (`swarm-b-roadmap`)

**2 agents assessed 5 long-term plans against the boomer shooter goal**

The most impactful finding: **elevate the Visual Editor from P7 to P5-P6**. Level design is the #1 differentiator for boomer shooters, and 4D levels cannot be designed in text. A two-phase approach (RON preview tool, then minimal egui editor) reduces the initial investment.

The ECS migration is not yet triggered (1 of 2 conditions met) but will be triggered once boomer shooter development begins. **Partial ECS (2-3 sessions)** -- adding ComponentStore<EntityKey, T> while keeping Entity -- is the pragmatic compromise. Full ECS (11-13 sessions) can wait.

Scripting is less urgent than expected -- classic boomer shooter AI uses simple state machines doable in Rust. A declarative RON trigger system (1-2 sessions) covers 80% of level scripting needs. Networking is firmly post-single-player (15-20 sessions, significant codebase gaps).

**Runtime state serialization** is a cross-cutting gap blocking editor, networking, and save systems. Rotor4 lacks Serialize/Deserialize.

### Swarm C: Feature & Genre Analysis (`swarm-c-features`)

**2 agents analyzed engine features and boomer shooter requirements**

Identified **12 critical P0 gaps** and **5 features requiring 4D adaptation**. Compared Rust4D against Unity and Godot across 11 feature categories, confirming that Rust4D's unique advantages (4D math, GPU slicing, 4D physics) are the hardest features to add and are not available in any commercial engine.

The genre analysis revealed that the W-axis transforms every boomer shooter system in meaningful ways:
- Movement gains W-strafing as a new dodge technique
- Explosions become hyperspheres (more valuable in 4D -- cover W-range)
- Hitscan requires W-alignment (creates a natural weapon hierarchy)
- Level design gains W-layered architecture
- Enemies gain W-flanking (ambush from invisible slices)

**Biggest risk: cognitive overload** (4D is hard to think about). **Biggest opportunity: genuine novelty** (no 4D boomer shooter exists).

---

## Cross-Swarm Convergence

All three swarms independently identified the same critical gaps, confirming the analysis:

### Unanimous P0 Gaps (all 3 swarms flagged)
1. **Raycasting** -- every swarm called this the single biggest gap
2. **Event/trigger system** -- collision layers exist but no callback mechanism
3. **Health/damage** -- no combat possible
4. **Weapon system** -- no shooting
5. **Enemy AI** -- nothing to fight
6. **HUD** -- no information display
7. **Audio** -- game is silent

### Unanimous Architecture Concerns
1. **Entity extensibility** -- monolithic struct won't scale for 10+ gameplay components
2. **No fixed timestep** -- physics varies with frame rate
3. **Serialization gap** -- runtime state can't be saved/loaded

### Converging Recommendations
- Both B1 and B2 agreed: **ECS decision gates all other features** (scripting, editor, networking)
- Both B2 and C1 agreed: **editor is more critical than scripting** for a boomer shooter
- Both A1 and C1 agreed: **raycasting math generalizes cleanly** to 4D (not technically hard)
- Both C1 and C2 agreed: **the W-axis is the key differentiator**, not just a gimmick

---

## Integrated Roadmap

Based on all seven agent reports, here is the recommended development sequence:

### Foundation (3-5 sessions)
**Goal: Unblock gameplay development**

1. Fix serialization gap -- add Serialize/Deserialize to Rotor4, RigidBody4D (1 session)
2. Partial ECS -- ComponentStore<EntityKey, T> for gameplay components (2 sessions)
3. Fixed timestep for physics (0.5 session)
4. Quick fixes: diagonal normalization, re-enable back-face culling (0.5 session)

### Phase 1: Combat Core (3-4 sessions)
**Goal: The player can shoot things**

5. 4D raycasting (Ray4D, ray-sphere, ray-AABB, world raycast with layer filtering) (1 session)
6. Event system (simple event bus for damage, pickup, trigger events) (1 session)
7. Health/damage system (HP component, damage, death) (1 session)
8. Trigger zone callbacks (extend collision system) (0.5 session)

### Phase 2: Weapons & Feedback (3-5 sessions)
**Goal: Combat feels good**

9. Weapon system (hitscan shotgun + projectile rocket, ammo, switching) (2 sessions)
10. Basic HUD (health, ammo, crosshair, W-position indicator) (1 session)
11. Audio system (rodio/kira, spatial with 4D distance attenuation) (1-2 sessions)
12. Muzzle flash, screen shake, damage flash (0.5 session)

### Phase 3: Enemies (3-4 sessions)
**Goal: Something fights back**

13. Enemy AI state machine (idle/chase/attack/pain/dead) (1 session)
14. Sprite billboard rendering (camera-facing quads) (1 session)
15. Explosion/area damage (hyperspherical volumes) (0.5 session)
16. 3 enemy types (melee rusher, projectile, W-phaser) (1 session)
17. Particle system for blood, sparks, explosions (1 session)

### Phase 4: Level Design Pipeline (4-6 sessions)
**Goal: Can build real levels**

18. RON preview tool with hot-reload (2-3 sessions)
19. Additional shape types for level geometry (1 session)
20. Door/elevator mechanics, key/door system (1-2 sessions)
21. Pickup system (health, ammo, weapons) (0.5 session)

### Phase 5: Editor & Polish (6-10 sessions)
**Goal: Efficient content creation and game polish**

22. Minimal egui editor (entity list, properties, W-slice nav, viewport) (6-8 sessions)
23. Point lights, basic shadows (1-2 sessions)
24. Texture support (1 session)
25. Input rebinding, pause menu (1 session)

### Phase 6: Advanced (deferred)
- Full ECS migration if partial ECS proves insufficient (11-13 sessions)
- Full scripting system (6-8 sessions)
- Post-processing pipeline (1-2 sessions)
- Networking (15-20 sessions)

---

## Effort Summary

| Milestone | Sessions | Cumulative |
|-----------|----------|------------|
| Foundation (unblock gameplay) | 3-5 | 3-5 |
| Can shoot things | 6-9 | 9-14 |
| Combat feels good | 9-14 | 15-23 |
| Something fights back | 12-18 | 21-32 |
| Can build real levels | 16-24 | 25-38 |
| Playable demo (Phases 1-3) | 12-18 | -- |
| MVBS (through Phase 4) | 16-24 | -- |

---

## Strategic Observations

### What to do NOW
1. Decide on Partial ECS vs Full ECS. Every agent agrees this gates further development.
2. Fix Rotor4 serialization -- it's a tiny fix that unblocks multiple features.
3. Start combat foundation (raycasting + events + health). This is the core loop.

### What to DEFER
1. Full ECS migration -- partial ECS buys time at 1/5th the cost.
2. Scripting -- declarative triggers cover most needs.
3. Networking -- post-single-player. Lay groundwork (fixed timestep, serialization) organically.
4. PBR/SSAO/advanced rendering -- wrong aesthetic for a boomer shooter.

### The Unique Opportunity
No 4D boomer shooter exists. The engine's strongest assets (Rotor4 math, GPU slicing, 4D collision layers) are precisely the features that are hardest to add and that no competitor has. The gameplay layer is standard game engine work -- well-understood, if substantial. The 4D-specific adaptations (4D raycasting, hyperspherical explosions, W-layered levels) are novel but tractable because the mathematical foundations are solid.

The W-axis is not a gimmick -- it fundamentally transforms every boomer shooter system in ways that create genuine new gameplay. W-strafing, hyperspherical explosions, W-layered levels, W-flanking enemies -- these are mechanics that have never existed in a game.

---

## Sources
- [Swarm A Synthesis: Codebase State](./swarm-a-codebase/synthesis.md)
- [Swarm B Synthesis: Roadmap Feasibility](./swarm-b-roadmap/synthesis.md)
- [Swarm C Synthesis: Feature & Genre Analysis](./swarm-c-features/synthesis.md)

### Agent Reports
- [A1: Math & Physics](./swarm-a-codebase/a1-math-physics.md)
- [A2: Core & Scene](./swarm-a-codebase/a2-core-scene.md)
- [A3: Render & Input](./swarm-a-codebase/a3-render-input.md)
- [B1: ECS & Rendering Plans](./swarm-b-roadmap/b1-ecs-rendering.md)
- [B2: Scripting/Editor/Networking Plans](./swarm-b-roadmap/b2-scripting-editor-networking.md)
- [C1: Engine Feature Comparison](./swarm-c-features/c1-engine-features.md)
- [C2: Boomer Shooter Genre Analysis](./swarm-c-features/c2-boomer-shooter.md)
