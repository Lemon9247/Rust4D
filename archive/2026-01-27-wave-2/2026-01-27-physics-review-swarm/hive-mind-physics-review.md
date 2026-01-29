# Physics & Entity Review Swarm - Hive Mind

## Overview

**Objective:** Comprehensive review of physics, entity, and scene loading systems to identify the root cause of the tesseract not falling correctly to the floor in `default.ron`.

**Key Issue:** The tesseract entity in `scenes/default.ron` does not correctly fall to and rest on the static hyperplane floor. This suggests either:
1. Physics collision/detection issues
2. Entity-physics synchronization problems
3. Rendering vs physics collider mismatches
4. Scene loading incorrectly setting up physics bodies

## File Ownership

| Agent | Focus Files |
|-------|-------------|
| Physics Reviewer | `crates/rust4d_physics/src/*.rs` |
| Scene Loading Reviewer | `crates/rust4d_core/src/scene.rs`, `crates/rust4d_core/src/shapes.rs`, `scenes/*.ron` |
| Architecture Reviewer | `src/main.rs`, crate structure, module dependencies |
| Testing Reviewer | All `#[cfg(test)]` sections, test coverage gaps |
| Existing Roadmap Reviewer | `scratchpad/plans/**/*.md`, `scratchpad/reports/**/*.md` |
| Entity-Physics Sync Reviewer | `crates/rust4d_core/src/world.rs`, `crates/rust4d_core/src/entity.rs` |

## Key Data Points

### Scene Configuration (default.ron)
- Floor: Hyperplane at y=-2, size=10, cell_size=5, thickness=0.001
- Tesseract: position (0,0,0,0), size=2.0, tagged "dynamic"
- Gravity: -20.0
- Player spawn: (0,0,5,0)

### Physics Setup (from scene.rs:from_template)
- Static "hyperplane" entities get `StaticCollider::floor_bounded()`
- Dynamic entities get `RigidBody4D::new_aabb()` with half_extent from shape size
- Floor uses bounded AABB collider (minimum thickness 5.0 units)

### Known Previous Issues (from session reports)
- Player floating was fixed by proper kinematic collision handling
- Camera W position reset was fixed by selective XYZ sync
- Bounded floor tunneling was fixed with minimum thickness

## Questions to Investigate

1. **Is the tesseract getting a physics body at all?** Check if `scene.rs:from_template` correctly creates body for "dynamic" tagged entities.

2. **Is the floor collider positioned correctly?** The bounded floor should have top at y=-2, but verify the AABB math.

3. **Is gravity being applied to the tesseract?** Dynamic bodies should get gravity unless marked otherwise.

4. **Is entity-physics sync working?** Check if `world.rs:update()` syncs physics body positions to entity transforms.

5. **Is the collision detection correct for AABB vs bounded floor AABB?** Check the `aabb_vs_aabb` function.

## Coordination Notes

- Each agent should focus on their area but note cross-cutting concerns
- If you find something that affects another agent's area, note it here
- DO NOT modify code - this is a review swarm

## Status

| Agent | Status | Finding |
|-------|--------|---------|
| Physics Reviewer | COMPLETE | Code CORRECT - collision, gravity, resolution all work |
| Scene Loading Reviewer | COMPLETE | Code CORRECT - bodies created correctly for "dynamic" entities |
| Architecture Reviewer | COMPLETE | Code CORRECT - game loop order is correct |
| Testing Reviewer | COMPLETE | CRITICAL GAP - No integration tests exist |
| Existing Roadmap Reviewer | COMPLETE | MINOR GAP - Bounded floor not in ColliderTemplate |
| Entity-Physics Sync Reviewer | COMPLETE | Code CORRECT - sync properly updates transforms |

## Cross-Cutting Findings

### Unanimous Conclusion
All 6 agents independently concluded that the code is **correctly implemented** in their respective areas. The tesseract falling bug was NOT reproduced through static code analysis.

### Critical Gap Identified
The codebase has ~180+ unit tests but **ZERO integration tests**. No test exists that verifies the complete pipeline:
1. Load scene → 2. Step physics → 3. Sync transforms → 4. Rebuild geometry

### Rendering Pipeline Verified
Additional verification confirmed that `add_entity_with_color()` correctly uses `entity.transform` which is updated by physics sync.

### Root Cause Hypothesis
Since all code paths appear correct, the bug may be:
- Race condition on first frame
- Delta time issue
- Scene not loading correctly at runtime
- Camera position not showing tesseract movement

### Recommended Action
Add integration tests (see synthesis-report.md). These will either:
1. Pass, proving code works and bug is elsewhere
2. Fail, revealing exactly where pipeline breaks

## Final Report

See `synthesis-report.md` for complete analysis and fix plan.

