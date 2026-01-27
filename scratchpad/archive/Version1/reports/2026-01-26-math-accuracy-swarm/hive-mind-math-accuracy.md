# Hive Mind: Mathematical Accuracy Comparison

**Task**: Deep comparison of Rust4D vs Engine4D rendering pipelines, focusing on mathematical accuracy
**Created**: 2026-01-26

## Agents

| Agent | Role | Status |
|-------|------|--------|
| Rust4D Analysis Agent | Deep dive into Rust4D's math and rendering | **Complete** |
| Engine4D Analysis Agent | Clone and analyze Engine4D implementation | **Complete** |
| Comparison Agent | Compare implementations for mathematical equivalence | **Complete** |

## Focus Areas

Each agent should document with mathematical rigor:

1. **Coordinate Systems**
   - Handedness (left/right)
   - Axis conventions (which is "up", which is "W")
   - Origin placement

2. **Hyperplane Slicing**
   - Plane equation used
   - Intersection point calculation
   - Edge parameterization formula

3. **Simplex Decomposition**
   - How 4D shapes are decomposed into simplices
   - Vertex ordering conventions
   - Face/edge orientation rules

4. **Normal Calculation**
   - How surface normals are derived
   - Orientation determination (inward vs outward)
   - Winding order conventions

5. **Lookup Tables**
   - Case numbering scheme
   - Edge indexing
   - Triangle winding in each case

## Questions to Answer

- [x] Is Rust4D's intersection formula mathematically equivalent to Engine4D's? **YES**
- [x] Do both use the same normal orientation convention? **NO - Engine4D precomputes in LUT**
- [x] Are the lookup table cases indexed the same way? **YES (bit i = vertex i)**
- [x] Is simplex centroid the right reference for orientation? **NO - geometric test is better**
- [x] What's the exact formula Engine4D uses for orientation? **Scalar triple product of signed edges**

## Decision

**Going with Option C**: Decompose 5-cells into tetrahedra to match Engine4D's architecture.

## Coordination Notes

(Agents: post updates and questions here)

---
