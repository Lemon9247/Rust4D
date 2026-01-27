# Roadmap Reviewer Agent Report - Physics Gap Analysis

**Agent**: Existing Roadmap Reviewer
**Date**: 2026-01-27
**Focus**: Understanding planned vs. implemented physics features and identifying potential gaps

---

## Executive Summary

The Rust4D engine has undergone significant physics development, with most planned Phase 1-8 features completed. The bounded floor collider is a recent addition (commits 0d9bb65, f9e8e39, c7116de, c5d36ff) designed to allow objects to fall off platform edges. However, this implementation is physics-focused and may not be fully integrated with the scene serialization or tesseract/dynamic entity behavior in all scenarios.

### Key Findings

1. **Bounded floors were added AFTER the initial physics completion** - they represent a late enhancement, not part of the original architecture
2. **The player floating fix focused specifically on kinematic vs dynamic collisions** - changes may have unintended side effects on other entity types
3. **Scene serialization uses a mix of infinite floors and bounded floors** - potential inconsistency
4. **No explicit "tesseract collision" tests** - physics tests use sphere colliders, not tesseract-specific shapes

---

## Timeline of Physics-Related Changes

### Original Physics Architecture (Phase 1-8)

From `scratchpad/reports/2026-01-27-physics-completion.md`:

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Generational handles (slotmap) | COMPLETE |
| 2 | Entity identity (names, tags) | COMPLETE |
| 3 | Physics materials (friction) | COMPLETE |
| 4 | Static colliders | COMPLETE |
| 5 | Player integration | COMPLETE |
| 6 | Collision groups | COMPLETE |
| 7 | Rendering optimization (dirty tracking) | COMPLETE |
| 8 | Main decomposition | PARTIAL |

### Recent Bounded Floor Addition (Post-Phase 8)

| Commit | Description | Files Changed |
|--------|-------------|---------------|
| `0d9bb65` | Add bounded floor colliders for finite platforms | body.rs, scene.rs |
| `f9e8e39` | Fix tunneling in bounded floor colliders | body.rs |
| `c5d36ff` | Add test for tesseract vs bounded floor collision | tests |
| `c7116de` | Fix collision axis selection for bounded floor | world.rs |

The bounded floor feature was added to allow objects to "fall off the edges of floor platforms instead of hitting an invisible infinite plane."

---

## Planned vs. Implemented Analysis

### What Was Planned (Engine Roadmap 2026)

From `scratchpad/plans/engine-roadmap-2026/00-index.md`:

**Near-Term (Active Development)**:
- Phase 1: Foundation (Scene Serialization, Configuration) - **COMPLETE**
- Phase 2: Scene Management (SceneManager, Prefab System) - **COMPLETE**
- Phase 3: Documentation - **COMPLETE**
- Phase 4: Architecture - Ready but not started
- Phase 5: Advanced Features - Ready but not started

**Long-Term**:
- ECS Migration - Draft
- Visual Scene Editor - Draft
- Scripting System - Draft
- Networking - Draft
- Advanced Rendering - Draft

### Physics-Specific Items That Were Planned

From `phase-1-foundation.md`:

1. **PhysicsTemplate enum**:
   - Static (collider, material)
   - Dynamic (mass, material)
   - Kinematic (material)
   - Status: Implemented in entity_template.rs

2. **Scene physics config**:
   - `ScenePhysicsConfig { gravity: f32 }`
   - Status: Implemented

3. **Collider templates**:
   - `ColliderTemplate::Sphere { radius }`
   - `ColliderTemplate::Floor { y, material }`
   - Status: Implemented, but **bounded floor not in template**

### Gap Identified: Bounded Floor Not in Templates

The `ColliderTemplate` enum from Phase 1 plans includes:
```rust
pub enum ColliderTemplate {
    Sphere { radius: f32 },
    Floor { y: f32, material: String },
}
```

But the bounded floor (`StaticCollider::floor_bounded()`) was added later and is **not** represented in the template system. This means:
- Scenes loaded from RON files may not use bounded floors
- The serialization path (`scene.rs`) has been updated but the template enum may not reflect this

---

## Player Floating Fix Analysis

From `scratchpad/reports/2026-01-27-physics-player-floating-fix.md`:

### The Three Causes Identified

1. **Kinematic Body Collision Handling** - Fixed in `world.rs`
   - Kinematic vs Static: Kinematic gets pushed out
   - Kinematic vs Dynamic: Dynamic gets pushed, kinematic doesn't move

2. **Camera W Position Reset** - Fixed in `main.rs`
   - Only sync X, Y, Z from physics; preserve W for 4D navigation

3. **No Gravity for Kinematic Player** - Fixed in `world.rs`
   - Apply gravity to player body specifically (hybrid behavior)

### Collision Resolution Logic

```rust
// Position correction: kinematic is immovable only vs dynamic
let can_correct_a = !is_static_a && (!is_kinematic_a || is_static_b);
let can_correct_b = !is_static_b && (!is_kinematic_b || is_static_a);
```

This logic was designed for **player vs tesseract** interaction but affects all kinematic/dynamic collisions.

### Potential Issue: Non-Player Dynamic Entities

The fix ensures:
- Player (kinematic) doesn't float on tesseract (dynamic)
- Player pushes tesseract instead of being pushed

But the question is: **What about tesseract vs bounded floor?**

The commit `c5d36ff` mentions "Add test for tesseract vs bounded floor collision" which suggests this specific interaction was tested. However, the detailed behavior needs verification.

---

## Bounded Floor Implementation Details

From `crates/rust4d_physics/src/body.rs`:

```rust
pub fn floor_bounded(
    y: f32,
    half_size_xz: f32,
    half_size_w: f32,
    thickness: f32,
    material: PhysicsMaterial,
) -> Self {
    // Use reasonable thickness - enough to prevent tunneling but not so thick
    // that Y overlap equals X/Z overlap (which breaks collision axis selection)
    let actual_thickness = thickness.max(5.0);
    let half_thickness = actual_thickness / 2.0;

    // Position AABB so top surface is at y
    let center = Vec4::new(0.0, y - half_thickness, 0.0, 0.0);
    let half_extents = Vec4::new(half_size_xz, half_thickness, half_size_xz, half_size_w);

    Self {
        collider: Collider::AABB(AABB4D::from_center_half_extents(center, half_extents)),
        ...
    }
}
```

### Key Design Decisions

1. **Minimum thickness of 5.0 units** - Anti-tunneling measure
2. **Uses AABB collision** - Not infinite plane like regular floor
3. **Y overlap issue** - Comments mention "breaks collision axis selection" if thickness equals X/Z overlap

### Axis Selection Fix

Commit `c7116de` "Fix collision axis selection for bounded floor" addresses an issue where the collision system was selecting the wrong axis for resolution. This is critical for floors because:
- If Y axis is selected, objects are pushed up (correct)
- If X/Z axis is selected, objects are pushed sideways (incorrect)

---

## What The Plans Say About Tesseract Behavior

### Phase 5 (Advanced Features) - Entity Hierarchy

The plan mentions:
- Complex multi-part entities (robots, vehicles)
- Compound shapes that move together
- Relative positioning

But **no specific discussion of tesseract collision behavior** beyond basic physics.

### Scene Serialization Plan

From `phase-1-foundation.md`:

```ron
// tesseract example in scene file
EntityTemplate(
    name: Some("tesseract"),
    tags: ["dynamic", "interactable"],
    shape: Tesseract(size: 2.0),
    physics: Some(Dynamic(
        mass: 10.0,
        material: "wood",
        collider: Sphere(radius: 1.4), // ~sqrt(2) for tesseract bounding sphere
    )),
)
```

**Important observation**: The plan uses a **sphere collider** for the tesseract, not an AABB or tesseract-shaped collider. This is noted as `~sqrt(2) for tesseract bounding sphere`.

---

## Documented Issues / TODOs

### In Source Code

From grep results:
```
crates/rust4d_render/src/pipeline/render_pipeline.rs:210:
    // TODO: Add a small compute shader to multiply by 3, or do it on CPU
```

Only one TODO found in the entire codebase - physics system is clean.

### In Plans

From `phase-1-foundation.md`:
```rust
ShapeTemplate::Sphere { radius } => {
    // TODO: Implement HyperSphere4D in rust4d_math
    // For now, use tesseract as placeholder
    ShapeRef::shared(Tesseract4D::new(*radius * 2.0))
}

ShapeTemplate::Wall { position, width, height, thickness } => {
    // TODO: Implement Wall4D in rust4d_math
    // For now, use tesseract as placeholder
    ShapeRef::shared(Tesseract4D::new(*width))
}
```

These TODOs affect shape variety but not core physics.

### From Physics Completion Report

From `2026-01-27-physics-completion.md`:
> "**Visual testing**: Run the application to verify physics behavior" listed as a next step

This suggests visual verification was planned but may not have been completed for all scenarios.

---

## Previous Swarm Findings

### Wave 2 Swarm (2026-01-27)

From `scratchpad/reports/2026-01-27-wave-2-swarm/`:

**SceneManager Agent**:
- Successfully integrated SceneManager
- Uses `active_world_mut().and_then(|w| w.physics_mut())` pattern
- All 90 rust4d_core tests pass

**Documentation Agent**:
- Created 04_camera_exploration example with tesseracts at different W positions
- Notes: "Placed tesseracts at various W positions to encourage 4D exploration"

### Engine Review Swarm

Referenced in `00-index.md`:
- `architecture-review.md`
- `config-recommendations.md`
- `scene-handling-review.md`
- `roadmap-draft.md`

These synthesized the engine roadmap but predate the bounded floor addition.

---

## Recommendations for Addressing Gaps

### High Priority

1. **Verify tesseract vs bounded floor collision behavior**
   - The test `c5d36ff` exists but should be reviewed for completeness
   - Ensure tesseract doesn't tunnel through bounded floors at high velocity
   - Ensure collision axis selection works correctly for AABB vs AABB

2. **Review scene.rs bounded floor integration**
   - Confirm RON-loaded scenes use bounded floors where appropriate
   - Consider adding `BoundedFloor` to `ColliderTemplate` enum

3. **Test non-player dynamic entities**
   - The player floating fix changed kinematic/dynamic collision handling
   - Verify other dynamic objects (tesseracts, enemies) behave correctly vs all collider types

### Medium Priority

4. **Document bounded floor parameters**
   - The 5.0 minimum thickness is an anti-tunneling measure
   - Document expected half_size_xz and half_size_w values for typical scenes
   - Explain axis selection issue in comments or docs

5. **Consider adding bounded floor to prefab templates**
   - Currently `ColliderTemplate` has `Floor` but not `BoundedFloor`
   - This limits scene serialization flexibility

### Low Priority

6. **Implement tesseract-shaped collider**
   - Current approach uses sphere approximation
   - Could improve collision accuracy but adds complexity

7. **Add visual debugging for colliders**
   - Would help diagnose physics issues
   - Listed in Phase 4 plans as `debug.show_colliders`

---

## Summary of Gaps

| Gap | Severity | Source |
|-----|----------|--------|
| Bounded floor not in ColliderTemplate | Medium | Plan vs Implementation |
| Tesseract uses sphere collider approximation | Low | Design decision |
| Limited axis selection testing | Medium | Recent fix |
| No visual collider debugging | Low | Phase 4 planned |
| Player fix may affect non-player entities | Medium | Side effect concern |

---

## Files Reviewed

- `scratchpad/plans/engine-roadmap-2026/00-index.md`
- `scratchpad/plans/engine-roadmap-2026/phase-1-foundation.md`
- `scratchpad/plans/engine-roadmap-2026/phase-2-scene-management.md`
- `scratchpad/plans/engine-roadmap-2026/phase-4-architecture.md`
- `scratchpad/plans/engine-roadmap-2026/phase-5-advanced-features.md`
- `scratchpad/reports/2026-01-27-physics-player-floating-fix.md`
- `scratchpad/reports/2026-01-27-physics-completion.md`
- `scratchpad/reports/2026-01-27-wave-2-swarm/hive-mind-wave-2.md`
- `scratchpad/reports/2026-01-27-wave-2-swarm/scene-manager-agent.md`
- `scratchpad/reports/2026-01-27-wave-2-swarm/documentation-agent.md`
- `crates/rust4d_physics/src/body.rs` (StaticCollider implementation)
- Git log for recent commits

---

**End of Roadmap Reviewer Report**
