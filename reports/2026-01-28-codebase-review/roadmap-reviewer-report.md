# Roadmap Reviewer Report

## Summary

This report verifies the roadmap status claims against actual implementation. Overall, the phase tracking from the wave planning report is **mostly accurate** with minor discrepancies. The biggest finding is that Phase 3B (Comprehensive Guides) appears **COMPLETE** rather than "NOT STARTED" as stated in the context. Additionally, the "Prefab System" in Phase 2B was documented as having an "ALTERNATIVE (EntityTemplate)" but this is actually just the implementation approach - the prefab-like functionality is complete via EntityTemplate.

## Phase Status Verification

| Phase | Claimed Status | Actual Status | Notes |
|-------|----------------|---------------|-------|
| 1A: Scene Serialization | COMPLETE | COMPLETE | Scene.load/save works, RON format, EntityTemplate serialization all functional |
| 1B: Configuration System | COMPLETE | COMPLETE | AppConfig, figment integration, TOML loading, env var overrides all working |
| 2A: Scene Manager | COMPLETE | COMPLETE | SceneManager with push/pop/switch, scene stack, template instantiation |
| 2B: Prefab System | ALTERNATIVE (EntityTemplate) | COMPLETE (via EntityTemplate) | No dedicated "Prefab" type, but EntityTemplate provides equivalent functionality |
| 3A: Examples + ARCHITECTURE | COMPLETE | COMPLETE | 4 examples + README, ARCHITECTURE.md with Mermaid diagrams |
| 3B: Comprehensive Guides | NOT STARTED | **COMPLETE** | docs/ folder exists with all 4 guides |
| 4: Architecture Refactoring | NOT STARTED | NOT STARTED | main.rs still 500+ lines, no systems extraction |
| 5: Advanced Features | NOT STARTED | NOT STARTED | No asset management, scene transitions, or entity hierarchy |

## Documentation vs Reality

| Document | Claim | Reality | Issue |
|----------|-------|---------|-------|
| README.md | "Scene serialization" in "What's in progress" | Scene serialization is complete | README needs update - should be in "What works" |
| README.md | Lists 4 examples | 4 examples exist and work | Accurate |
| README.md | "Audio integration" in progress | No audio code exists | Accurate (still in progress) |
| ARCHITECTURE.md | Future: "Scene Graph" for hierarchical transforms | Not implemented, no entity hierarchy | Accurate |
| ARCHITECTURE.md | Future: "Async Asset Loading" | Not implemented | Accurate |
| Phase 1 Plan | "Prefab files with override support" in Phase 2B goals | Not implemented - EntityTemplate doesn't support overrides | Phase 2B plan overpromises |
| Phase 2 Plan | Full Prefab struct with instantiate_with() | Not implemented - only EntityTemplate exists | Plan describes features not built |
| Phase 3 Plan | Phase 3B "NOT STARTED" | docs/ directory has all 4 guides | Plan needs status update |
| Phase 5 Plan | Asset Management, Entity Hierarchy | Not started | Accurate |

## Outdated Roadmap Items

| Item | Location | Issue |
|------|----------|-------|
| "Scene serialization" in progress | README.md line 21 | Should be moved to "What works" - fully functional |
| Phase 3B status | Wave planning context | Listed as "NOT STARTED" but docs exist |
| Prefab System description | phase-2-scene-management.md | Describes full Prefab struct with overrides that doesn't exist |
| ColliderTemplate | phase-1-foundation.md line 249-252 | Mentions BoundedFloor collider - this IS implemented in physics but not in template |

## Missing from Roadmap

| Feature | Location | Description |
|---------|----------|-------------|
| ActiveScene struct | `crates/rust4d_core/src/scene.rs` | Runtime scene wrapper not in any plan |
| SceneLoadError/SceneSaveError | `crates/rust4d_core/src/scene.rs` | Error types not mentioned in plans |
| Scene gravity from template | `scene.rs` line 228-240 | Template gravity overrides documented but implementation details missing |
| Default physics config | SceneManager | `with_physics()` builder not in Phase 2 plan |
| Player radius config | SceneManager | `with_player_radius()` not in any plan |
| SceneConfig.path | `src/config.rs` | Scene file path in config not in Phase 1B |
| DirtyFlags enum | `entity.rs` | Bitflags for dirty tracking not in roadmap |

## Key Discrepancies Found

### 1. Phase 3B Documentation is Complete

The context from wave planning states Phase 3B is "NOT STARTED", but the `docs/` directory exists with:
- `docs/README.md` - Documentation index
- `docs/getting-started.md` - New user onboarding
- `docs/user-guide.md` - Comprehensive manual
- `docs/developer-guide.md` - Contributor guide

This is a **status tracking error** - Phase 3B appears complete.

### 2. Prefab System Mismatch

The Phase 2B plan (lines 565-1295 of phase-2-scene-management.md) describes:
- `Prefab` struct with `instantiate()` and `instantiate_with()` methods
- Override support with `EntityTemplateOverrides`
- RON prefab files in `prefabs/` directory
- Prefab registry in Scene

**Reality:** None of these exist. Instead, `EntityTemplate` provides basic serializable entity definitions that can be converted to entities via `to_entity()`. There is no:
- Separate `Prefab` struct
- Override merging functionality
- `prefabs/` directory
- Prefab loading from files

The roadmap should clarify that EntityTemplate IS the prefab solution, or acknowledge that the full Prefab system was descoped.

### 3. ColliderTemplate Missing BoundedFloor

Phase 1 plan (line 249-252) mentions:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColliderTemplate {
    Sphere { radius: f32 },
    Floor { y: f32, material: String },
}
```

In reality, `ShapeTemplate` exists (not `ColliderTemplate`) with:
- `Tesseract { size: f32 }`
- `Hyperplane { y, size, subdivisions, cell_size, thickness }`

The physics colliders are created in `ActiveScene::from_template()` based on tags ("static" or "dynamic"), not from a separate ColliderTemplate.

### 4. Configuration Connected But Not All Values Used

The config consolidation work connected many values, but:
- `camera.pitch_limit` - in config but hardcoded in Camera4D (noted in hive-mind)
- `debug.show_colliders` - in config but not implemented
- `debug.show_overlay` - in config but not implemented

## Recommendations

1. **Update Phase 3B status** - Mark as COMPLETE in roadmap index and any tracking documents

2. **Clarify Prefab System status** - Either:
   - Document that EntityTemplate IS the prefab solution (simplified approach)
   - Or create a new plan for implementing full Prefab system as Phase 5+

3. **Update README.md** - Move "Scene serialization" from "in progress" to "What works"

4. **Add missing features to roadmap** - Document the following in future phases:
   - Debug overlay implementation
   - Collider visualization
   - Camera pitch_limit connection

5. **Clean up Phase 1 plan** - Remove or update ColliderTemplate references since it doesn't exist

6. **Add Phase 2B completion notes** - Document that EntityTemplate was chosen over full Prefab system

7. **Update wave planning status table** - Reflect accurate completion states

## Files Reviewed

- `/home/lemoneater/Projects/Rust4D/README.md`
- `/home/lemoneater/Projects/Rust4D/ARCHITECTURE.md`
- `/home/lemoneater/Projects/Rust4D/scratchpad/plans/engine-roadmap-2026/00-index.md`
- `/home/lemoneater/Projects/Rust4D/scratchpad/plans/engine-roadmap-2026/phase-1-foundation.md`
- `/home/lemoneater/Projects/Rust4D/scratchpad/plans/engine-roadmap-2026/phase-2-scene-management.md`
- `/home/lemoneater/Projects/Rust4D/scratchpad/plans/engine-roadmap-2026/phase-3-documentation.md`
- `/home/lemoneater/Projects/Rust4D/scratchpad/plans/engine-roadmap-2026/phase-4-architecture.md`
- `/home/lemoneater/Projects/Rust4D/scratchpad/plans/engine-roadmap-2026/phase-5-advanced-features.md`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/scene.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/scene_manager.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/entity.rs`
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/shapes.rs`
- `/home/lemoneater/Projects/Rust4D/src/config.rs`
- `/home/lemoneater/Projects/Rust4D/docs/` (confirmed exists with 4 markdown files)
- `/home/lemoneater/Projects/Rust4D/examples/` (confirmed 4 examples + README)
- `/home/lemoneater/Projects/Rust4D/scenes/` (confirmed default.ron and test_chamber.ron)

## Updated Phase Status Summary

Based on this review, the accurate phase status is:

| Phase | Status |
|-------|--------|
| Phase 1A: Scene Serialization | COMPLETE |
| Phase 1B: Configuration System | COMPLETE |
| Phase 2A: Scene Manager | COMPLETE |
| Phase 2B: Prefab System | COMPLETE (simplified as EntityTemplate) |
| Phase 3A: Examples + ARCHITECTURE | COMPLETE |
| Phase 3B: Comprehensive Guides | COMPLETE |
| Phase 4: Architecture Refactoring | NOT STARTED |
| Phase 5: Advanced Features | NOT STARTED |

**Next recommended work:** Phase 4 (Architecture Refactoring) to reduce main.rs complexity.
