# Rust4D Engine Development Roadmap

**Synthesized by:** Roadmap Agent
**Date:** 2026-01-27
**Status:** Draft for Review

---

## Executive Summary

Based on comprehensive reviews of documentation, architecture, configuration, and scene handling, the Rust4D engine is fundamentally sound but needs work in several areas to evolve from "working prototype" to "production-ready engine."

**Overall Assessment:**
- **Core Engine:** Excellent (well-structured crates, clean physics, solid math)
- **Documentation:** Good internal docs, minimal user-facing docs
- **Architecture:** Sound at library level, main.rs needs decomposition
- **Configuration:** Non-existent (40+ hardcoded values)
- **Scene System:** Minimal but functional (needs persistence and management)

**Top Priority Recommendations:**
1. Scene persistence (RON-based serialization)
2. Configuration system (Figment + TOML)
3. Documentation improvements (examples, guides)
4. main.rs decomposition (extract systems)

---

## Key Findings by Area

### Documentation (Grade: B- internal, D+ external)

**Strengths:**
- 867 item-level doc comments across codebase
- 176 module-level doc comment lines
- Consistent documentation of public APIs
- Good explanations of complex algorithms (4D rotations)

**Gaps:**
- No examples directory (critical gap)
- No getting-started guide
- No architecture documentation
- No tutorials or user guides
- No API examples in doc comments
- Minimal README (41 lines)

**Impact:** High - Current state blocks adoption and contribution

### Architecture (Grade: A- libraries, C+ application)

**Strengths:**
- Excellent crate organization with clean boundaries
- 5,700 lines of well-structured Rust code
- Good test coverage (260 tests)
- Modern patterns (slotmap, builders, dirty tracking)

**Weaknesses:**
- main.rs is 511 lines with mixed concerns
- RedrawRequested handler is 215 lines (42% of main.rs)
- No system-based architecture
- Limited error handling (relies on expect/unwrap)

**Impact:** Medium - Maintainable now, will become painful as features grow

### Configuration (Grade: F - does not exist)

**Current State:**
- 40+ hardcoded constants scattered throughout codebase
- No user-configurable settings
- All changes require recompilation

**Critical Hardcoded Values:**
- Physics: gravity (-20.0), jump velocity (8.0), player radius (0.5)
- Camera: FOV (45°), sensitivity (0.002), speeds (3.0, 2.0)
- Rendering: light direction, ambient/diffuse strength, background color
- Scene: player start position, floor size, tesseract parameters

**Impact:** High - Blocks content creation and experimentation

### Scene Handling (Grade: C+ - functional but minimal)

**Strengths:**
- SceneBuilder provides clean declarative API
- Recently added, reducing boilerplate significantly
- Good test coverage for builder

**Gaps:**
- No scene persistence (can't load/save scenes)
- No scene switching (single world only)
- No prefab/template system
- No entity hierarchy (flat structure)
- Purely programmatic (no file-based scenes)

**Impact:** Critical - Blocks multi-level games and content pipelines

---

## Dependency Analysis

### Critical Dependencies (Blocks Multiple Features)

```
Scene Serialization (Phase 1A)
├── Blocks: Scene Manager, Prefab System, Asset Management
├── Enables: Content creation without recompilation
└── Priority: CRITICAL PATH

Configuration System (Phase 1B)
├── Blocks: User settings, Scene configuration integration
├── Enables: Experimentation without rebuilding
└── Priority: CRITICAL PATH (Parallel with 1A)
```

### Feature Dependencies

```
Documentation Layer (Can run parallel to all phases)
└── No blockers, enhances all features

Architecture Refactoring
├── Depends on: Configuration system (for clean separation)
├── Blocks: Advanced features (UI system, debug overlay)
└── Can proceed incrementally alongside other work

Scene Management Stack
├── Phase 1: Scene persistence (Foundation)
├── Phase 2: Scene manager (Depends on Phase 1)
└── Phase 3: Prefab system (Depends on Phase 1, enhances Phase 2)
```

---

## Phased Roadmap

### Phase 1: Foundation (Sessions 1-4)

**Goal:** Establish core infrastructure for configuration and scene persistence

#### Wave 1A: Scene Serialization (2 sessions) - CRITICAL PATH

**Tasks:**
1. Add serde derives to Entity, World, Transform4D, Material
2. Create Scene struct with metadata (name, version, author)
3. Implement ShapeTemplate enum for serialization (avoid trait objects)
4. Add Scene::load() and Scene::save() using RON format
5. Create example scene file (test_level.ron)
6. Add tests for serialization roundtrip

**Dependencies:** None (can start immediately)
**Estimate:** 2 sessions
**Impact:** CRITICAL - Unlocks content creation

**Files affected:**
- `crates/rust4d_core/src/entity.rs` - Add serde derives
- `crates/rust4d_core/src/world.rs` - Add serde derives
- `crates/rust4d_core/src/scene.rs` (NEW) - Scene struct + serialization
- `scenes/test_level.ron` (NEW) - Example scene file

**Deliverables:**
- Can save current world to RON file
- Can load world from RON file
- Scene metadata tracked (name, version)
- Tests prove roundtrip works

---

#### Wave 1B: Configuration System (2 sessions) - CRITICAL PATH (Parallel)

**Tasks:**
1. Add Figment + serde dependencies
2. Create `src/config.rs` with config structs
3. Create `config/default.toml` with all engine settings
4. Add config loading with hierarchy (default → user → env)
5. Migrate hardcoded values to config
6. Add config validation and error handling

**Dependencies:** None (parallel with Wave 1A)
**Estimate:** 2 sessions
**Impact:** HIGH - Enables experimentation and user customization

**Files affected:**
- `Cargo.toml` - Add figment dependency
- `src/config.rs` (NEW) - AppConfig structs
- `config/default.toml` (NEW) - Default settings
- `src/main.rs` - Load config at startup
- `crates/rust4d_input/src/camera_controller.rs` - Use config values
- `crates/rust4d_physics/src/world.rs` - Use config values

**Deliverables:**
- All hardcoded values moved to config
- Users can edit config/user.toml to override
- Environment variables work (R4D_PHYSICS__GRAVITY=-5.0)
- Config validation with helpful errors

---

### Phase 2: Scene Management (Sessions 5-7)

**Goal:** Multi-scene support and prefab system

**Dependencies:** Requires Phase 1A (Scene serialization)

#### Wave 2A: Scene Manager (1 session)

**Tasks:**
1. Create SceneManager singleton
2. Implement scene stack (push/pop/switch)
3. Add scene loading/unloading
4. Support multiple active scenes (overlays)
5. Update main.rs to use SceneManager

**Estimate:** 1 session
**Impact:** HIGH - Enables multi-level games, menus

**Files affected:**
- `crates/rust4d_core/src/scene_manager.rs` (NEW)
- `src/main.rs` - Use SceneManager instead of single World

---

#### Wave 2B: Prefab System (2 sessions)

**Tasks:**
1. Create Prefab and EntityTemplate types
2. Implement prefab loading from RON files
3. Add prefab instantiation with overrides
4. Support nested prefabs (prefabs containing prefabs)
5. Add tests for prefab system

**Estimate:** 2 sessions
**Impact:** HIGH - Essential for runtime entity spawning

**Files affected:**
- `crates/rust4d_core/src/prefab.rs` (NEW)
- `crates/rust4d_core/src/scene.rs` - Add prefab instantiation
- `prefabs/` (NEW) - Example prefab files

---

### Phase 3: Documentation (Sessions 8-12)

**Goal:** Make Rust4D accessible to users and contributors

**Dependencies:** None (can run parallel to all phases)

This phase has high parallelization potential - multiple documentation tasks can run simultaneously.

#### Wave 3A: Quick Wins (2 sessions) - Can run parallel

**Agent 1: Examples (1 session)**
- Create `examples/` directory
- Add 01_hello_tesseract.rs (minimal example)
- Add 02_physics_demo.rs (physics showcase)
- Add 03_scene_loading.rs (load scene from file)
- Add examples/README.md index

**Agent 2: Core Docs (1 session)**
- Expand README.md (add architecture overview, screenshots)
- Create ARCHITECTURE.md with diagrams:
  - Crate dependency graph (Mermaid or ASCII)
  - Data flow diagram (input → physics → render)
  - Entity/Component relationships
  - 4D coordinate system visualization
- Add code examples to key doc comments (Vec4, Rotor4, Transform4D)

**Impact:** HIGH - Immediate improvement to usability

---

#### Wave 3B: Comprehensive Guides (2-3 sessions) - Can run parallel

**Agent 1: User Guide (1.5 sessions)**
- Create `docs/getting-started.md`
- Create `docs/user-guide.md` (comprehensive reference)
- Create `docs/4d-primer.md` (understanding 4D concepts)

**Agent 2: Developer Guide (1.5 sessions)**
- Create `docs/developer-guide.md` (architecture, internals)
- Enhance crate-level documentation (lib.rs files)
- Add inline examples to all public APIs

**Impact:** MEDIUM - Lowers barrier to contribution

---

### Phase 4: Architecture Refactoring (Sessions 13-16)

**Goal:** Decompose main.rs, improve maintainability

**Dependencies:** Configuration system (Phase 1B) should be complete

#### Wave 4A: System Extraction (3 sessions)

**Session 1: WindowSystem (0.5 sessions)**
- Extract window creation, cursor management, fullscreen
- Target: Reduce main.rs by ~60 lines
- Low risk, high clarity improvement

**Session 2: RenderSystem (1 session)**
- Extract RenderContext, pipelines, frame rendering
- Target: Reduce main.rs by ~120 lines
- Medium risk (touches GPU code)

**Session 3: SimulationSystem (1 session)**
- Extract game loop, physics integration, dirty tracking
- Target: Reduce main.rs by ~80 lines
- Medium risk (core game logic)

**Session 4: InputMapper (0.5 sessions)**
- Extract special key handling, action mapping
- Target: Reduce main.rs by ~40 lines
- Low risk, enables configurable controls

**Estimate:** 3 sessions total
**Impact:** MEDIUM - Prepares for future growth, improves testability

**Final main.rs:** ~150 lines (from 511)

---

#### Wave 4B: Error Handling (1 session) - Can run parallel

**Tasks:**
1. Replace panics with Results in key systems
2. Create error types (RenderError, SceneError, ConfigError)
3. Implement error propagation
4. Add helpful error messages

**Estimate:** 1 session
**Impact:** LOW - Nice to have, improves UX

---

### Phase 5: Advanced Features (Sessions 17-22)

**Goal:** Production-ready features

**Dependencies:** Scene Manager (Phase 2A), Prefab system (Phase 2B)

#### Wave 5A: Asset Management (1 session)

**Tasks:**
1. Create AssetCache for shared resources
2. Track asset dependencies
3. Add asset hot reloading
4. Reference counting for memory efficiency

**Estimate:** 1 session
**Impact:** MEDIUM - Improves loading times and memory

---

#### Wave 5B: Entity Hierarchy (2-3 sessions) - OPTIONAL

**Tasks:**
1. Add parent-child relationships to entities
2. Implement hierarchical transforms (children relative to parent)
3. Scene graph traversal and operations
4. Update serialization to support hierarchy

**Estimate:** 2-3 sessions
**Impact:** LOW - Nice to have, not urgent

**Note:** Defer unless needed for specific features (robot with joints, car with wheels)

---

#### Wave 5C: Advanced Scene Features (2 sessions)

**Tasks:**
1. Scene transitions with effects (fade, slide)
2. Scene overlays (HUD, pause menu)
3. Scene streaming (load in background)
4. Scene validation and diagnostics

**Estimate:** 2 sessions
**Impact:** MEDIUM - Polish for production games

---

## Session Estimates by Priority

### Critical Path (Must Do)

| Phase | Feature | Sessions | Priority |
|-------|---------|----------|----------|
| 1A | Scene Serialization | 2 | P0 |
| 1B | Configuration System | 2 | P0 |
| 2A | Scene Manager | 1 | P1 |
| 2B | Prefab System | 2 | P1 |
| **Total Critical** | | **7 sessions** | |

### High Value (Should Do)

| Phase | Feature | Sessions | Priority |
|-------|---------|----------|----------|
| 3A | Examples + Core Docs | 2 | P2 |
| 3B | Comprehensive Guides | 2-3 | P2 |
| 4A | System Extraction | 3 | P3 |
| **Total High Value** | | **7-8 sessions** | |

### Nice to Have (Can Defer)

| Phase | Feature | Sessions | Priority |
|-------|---------|----------|----------|
| 4B | Error Handling | 1 | P4 |
| 5A | Asset Management | 1 | P4 |
| 5B | Entity Hierarchy | 2-3 | P5 |
| 5C | Advanced Scenes | 2 | P5 |
| **Total Nice to Have** | | **6-7 sessions** | |

**Grand Total:** 20-22 sessions for complete roadmap

---

## Parallelization Opportunities

### Maximum Parallelism Strategy

**Wave 1 (Sessions 1-2):** 2 agents in parallel
- Agent A: Scene serialization (Phase 1A)
- Agent B: Configuration system (Phase 1B)

**Wave 2 (Sessions 3-4):** 2 agents in parallel
- Agent A: Scene manager (Phase 2A)
- Agent B: Examples + Core docs (Phase 3A-1)

**Wave 3 (Sessions 5-6):** 3 agents in parallel
- Agent A: Prefab system (Phase 2B)
- Agent B: User guide (Phase 3B-1)
- Agent C: Developer guide (Phase 3B-2)

**Wave 4 (Sessions 7-9):** 2 agents in parallel
- Agent A: System extraction (Phase 4A)
- Agent B: Error handling (Phase 4B)

**Optimistic Timeline with Parallelism:** 9 sessions to complete P0-P3
**Conservative Timeline:** 14-16 sessions (accounting for integration overhead)

---

## Quick Wins (Do First)

These provide maximum impact for minimum effort:

### 1. Scene Serialization (2 sessions) - HIGHEST ROI

**Why first:**
- Blocks all other scene features
- Biggest productivity unlock
- Enables non-programmers to create content

**What it enables:**
- Load scenes without recompiling
- Artists can edit scene files
- Rapid iteration on level design

---

### 2. Configuration System (2 sessions) - HIGHEST ROI

**Why second:**
- Independent from scene work (can run parallel)
- Quick experimentation without rebuilds
- Enables user customization

**What it enables:**
- Tweak physics without rebuilding
- Test different settings easily
- Users can customize controls

---

### 3. Examples Directory (1 session) - HIGH ROI

**Why third:**
- Dramatically improves documentation
- Shows how to use the engine
- Quick to implement

**What it enables:**
- New users can understand the engine
- Reference implementations for common tasks
- Living documentation that stays up to date

---

### 4. Enhanced README (0.5 sessions) - HIGH ROI

**Why fourth:**
- First impression for new users
- Minimal effort, high impact
- Sets expectations correctly

**What it enables:**
- Better GitHub discoverability
- Clear project status communication
- Links to other documentation

---

## Long-Term Vision (Beyond 22 Sessions)

### Future Considerations (Not in Current Roadmap)

**Component-Based Architecture (ECS)**
- Current monolithic Entity works for now
- Consider ECS when extensibility becomes limiting
- Estimated: 8-12 sessions for full ECS migration
- Priority: Defer until pain points emerge

**Visual Scene Editor**
- GUI tool for editing scenes (egui-based)
- Drag-and-drop entity placement
- Visual prefab editing
- Estimated: 10-15 sessions
- Priority: Low (manual RON editing works)

**Scripting System**
- Embed Lua or Rhai for gameplay logic
- Hot-reload scripts without rebuilding
- Estimated: 6-8 sessions
- Priority: Medium (future feature)

**Networking/Multiplayer**
- Client-server architecture
- Entity replication
- Estimated: 15-20 sessions
- Priority: Low (single-player focus for now)

**Advanced Rendering**
- Post-processing effects
- Custom shader support
- Multiple render passes
- Estimated: 8-10 sessions
- Priority: Medium (polish)

---

## Risk Assessment

### Low Risk (Safe to Start)

- Scene serialization (well-understood problem)
- Configuration system (proven libraries)
- Documentation (no code changes)
- WindowSystem extraction (isolated)

### Medium Risk (Needs Care)

- Scene manager (affects game loop structure)
- Prefab system (complex object instantiation)
- RenderSystem extraction (GPU code is sensitive)
- SimulationSystem extraction (core game logic)

### High Risk (Defer or Prototype First)

- Entity hierarchy (affects core architecture)
- Full ECS migration (massive refactoring)
- Visual editor (large scope, unclear ROI)

---

## Implementation Recommendations

### For Phase 1 (Foundation)

**Do:**
- Start both 1A and 1B in parallel (different agents)
- Write comprehensive tests for serialization
- Document config file format in ARCHITECTURE.md
- Create example files (test_level.ron, config/default.toml)

**Don't:**
- Try to serialize everything at once (start with basics)
- Optimize serialization performance yet (profile first)
- Implement advanced config features (keep it simple)

### For Phase 2 (Scene Management)

**Do:**
- Build SceneManager on top of Phase 1A foundation
- Test scene switching thoroughly (memory leaks, cleanup)
- Document prefab format and usage patterns
- Create example prefabs for common objects

**Don't:**
- Implement scene transitions yet (Phase 5C)
- Add nested prefab support initially (start simple)
- Worry about scene streaming (load everything for now)

### For Phase 3 (Documentation)

**Do:**
- Parallelize documentation work (3+ agents)
- Make examples runnable and testable
- **Include architecture graphs** (Mermaid diagrams for crate deps, data flow, etc.)
- Include screenshots/diagrams where helpful
- Keep examples minimal and focused

**Don't:**
- Write exhaustive documentation (cover 80% cases)
- Create video tutorials yet (text first)
- Over-document obvious things (focus on 4D concepts)

### For Phase 4 (Architecture)

**Do:**
- Extract one system at a time (test between each)
- Keep commits small and focused (commit after each system)
- Add integration tests for system interactions
- Document system interfaces before implementing

**Don't:**
- Extract everything at once (incremental approach)
- Break working functionality (test thoroughly)
- Optimize prematurely (profile first)

---

## Success Metrics

### Phase 1 Success Criteria

- [ ] Can load and save scenes from RON files
- [ ] All hardcoded values moved to config
- [ ] User can edit config/user.toml to customize
- [ ] Tests prove serialization roundtrip works
- [ ] No regressions in existing functionality

### Phase 2 Success Criteria

- [ ] Can load multiple scene files
- [ ] Can switch between scenes at runtime
- [ ] Can instantiate prefabs from RON files
- [ ] Memory is cleaned up on scene unload
- [ ] Scene transitions work smoothly

### Phase 3 Success Criteria

- [ ] Examples directory with 3+ working examples
- [ ] Enhanced README with architecture overview
- [ ] ARCHITECTURE.md explains crate relationships
- [ ] Getting started guide for new users
- [ ] Developer guide for contributors

### Phase 4 Success Criteria

- [ ] main.rs reduced to ~150 lines (from 511)
- [ ] Each system has clear responsibilities
- [ ] Integration tests prove systems work together
- [ ] No performance regressions
- [ ] Error handling improves user experience

---

## Conclusion

The Rust4D engine has excellent fundamentals but needs infrastructure work to support content creation and growth. The roadmap prioritizes two critical paths:

1. **Scene persistence** (enables content creation)
2. **Configuration system** (enables experimentation)

These two features unlock everything else and provide immediate value. Documentation and architecture refactoring support long-term maintainability but can proceed incrementally.

**Recommended Starting Point:**
- Session 1-2: Implement scene serialization (Agent A) and configuration system (Agent B) in parallel
- Session 3-4: Add scene manager and examples
- Session 5-7: Implement prefab system and core documentation
- Review progress and prioritize remaining phases

**Expected Outcome After 7 Sessions:**
- Scenes can be loaded from files
- Engine is fully configurable
- Multi-scene games are possible
- Prefabs enable runtime entity spawning
- Basic documentation exists

This transforms Rust4D from "working prototype" to "usable game engine."

---

**Next Steps:**
1. Review this roadmap with Willow
2. Prioritize phases based on immediate needs
3. Start with Phase 1 (Foundation)
4. Commit to incremental progress

**Report prepared by:** Roadmap Agent
**Based on:** 4 comprehensive agent reviews
**Total analysis:** ~3,200 lines of findings across all reports
