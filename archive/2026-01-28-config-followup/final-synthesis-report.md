# Config Consolidation Open Questions - Final Synthesis Report

**Date**: 2026-01-28
**Task**: Analyze three open questions from config consolidation to determine implementation priority

---

## Executive Summary

The swarm analyzed three open questions from the config consolidation report. Findings reveal one clear bug (max_triangles 10x mismatch), one redundant field (floor_y), and one low-priority enhancement (pitch_limit). Recommended action: implement max_triangles fix, remove floor_y, skip pitch_limit.

## Question 1: Camera pitch_limit

**Finding**: SKIP (values already match, low impact)

| Aspect | Details |
|--------|---------|
| Config value | `pitch_limit = 89.0` (degrees) |
| Hardcoded value | `PITCH_LIMIT = 1.553` (~89 degrees in radians) |
| Location | `crates/rust4d_render/src/camera4d.rs:49` |
| Usage | `Camera4D::rotate_3d()` clamps pitch |

**Analysis**:
- Values functionally match (89 degrees = ~1.553 radians)
- No actual mismatch exists - just not connected
- Low user impact - rarely adjusted in practice
- Would require API changes to Camera4D constructor

**Recommendation**: **SKIP** - Values match, low value for effort.

---

## Question 2: Physics floor_y

**Finding**: REMOVE (completely unused)

| Aspect | Details |
|--------|---------|
| Config value | `floor_y = -2.0` |
| Actual usage | **NONE** - never read in code |
| Scene handling | Scenes define floors via `y` in Hyperplane entities |

**Analysis**:
- `floor_y` exists in config but is **never used anywhere**
- `to_physics_config()` explicitly excludes it
- Scenes define their own floor positions in `.ron` files
- Examples hardcode floor values directly

**Evidence**:
- `scene.rs:251-260` reads floor Y from scene template, not config
- `examples/03_physics_demo.rs:54` hardcodes `floor_y = -3.0`
- `PhysicsConfig` struct has no floor_y field

**Recommendation**: **REMOVE** - Eliminates confusion, enforces correct pattern.

**Files to update**:
1. `config/default.toml` - Remove `floor_y` line
2. `src/config.rs` - Remove `floor_y` field and `default_floor_y()` function

---

## Question 3: Rendering max_triangles

**Finding**: IMPLEMENT (10x mismatch, silent corruption risk)

| Aspect | Details |
|--------|---------|
| Config value | `max_triangles = 1000000` (1M) |
| Hardcoded value | `MAX_OUTPUT_TRIANGLES = 100_000` (100K) |
| Location | `crates/rust4d_render/src/pipeline/types.rs:208` |
| Impact | **10x buffer size mismatch** |

**Analysis**:
- **Critical mismatch**: Config says 1M, code uses 100K
- Buffer exhaustion causes **silent data corruption** (no bounds checking in shader)
- Memory impact: 100K = 14.4 MB, 1M = 144 MB GPU memory
- Users editing config get no effect

**Shader behavior**:
```wgsl
let vertex_idx = atomicAdd(&triangle_count, 3u);
// No bounds check - overruns silently
triangles[output_idx].v0 = ...
```

**Recommendation**: **IMPLEMENT** - Fixes clear inconsistency, enables user customization.

**Files to update**:
1. `crates/rust4d_render/src/pipeline/slice_pipeline.rs` - Accept max_triangles parameter
2. `crates/rust4d_render/src/pipeline/types.rs` - Make constant a default
3. `src/main.rs` - Pass `config.rendering.max_triangles` to SlicePipeline

---

## Prioritized Recommendations

| Priority | Question | Action | Effort | Rationale |
|----------|----------|--------|--------|-----------|
| 1 | max_triangles | IMPLEMENT | 1-2 sessions | 10x mismatch, silent corruption |
| 2 | floor_y | REMOVE | 0.5 sessions | Unused, causes confusion |
| 3 | pitch_limit | SKIP | - | Values match, low impact |

## Implementation Plan

### Wave 1: Fix max_triangles (Parallel-ready)
1. Modify `SlicePipeline::new()` to accept `max_triangles: u32` parameter
2. Use parameter for buffer allocation instead of hardcoded constant
3. Update `main.rs` to pass `config.rendering.max_triangles`
4. Add reasonable bounds validation (min: 10K, max: 10M)

### Wave 2: Remove floor_y (Sequential)
1. Remove `floor_y` from `config/default.toml`
2. Remove `floor_y` field from `PhysicsConfigToml`
3. Remove `default_floor_y()` function
4. Update any documentation references

### Not Implemented
- `camera.pitch_limit` - Defer until there's user demand

---

## Sources
- Camera Agent analysis of `camera4d.rs`
- Physics Agent analysis of floor_y usage
- Rendering Agent analysis of SlicePipeline

## Open Questions Resolved

| Original Question | Resolution |
|-------------------|------------|
| Should camera.pitch_limit be connected? | No - values match, low impact |
| Is physics.floor_y still needed? | No - completely unused, remove it |
| Should rendering.max_triangles be connected? | Yes - 10x mismatch is a bug |
