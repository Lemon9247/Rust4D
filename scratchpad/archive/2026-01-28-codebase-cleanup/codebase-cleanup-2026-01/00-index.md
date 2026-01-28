# Codebase Cleanup Plans - January 2026

**Created**: 2026-01-28
**Based on**: Comprehensive Codebase Review (8-agent swarm)
**Total Effort**: 4-6 sessions

---

## Overview

These plans address issues identified in the codebase review swarm. The work is organized into 4 waves that can be executed sequentially or with some parallelism.

## Wave Summary

| Wave | Focus | Sessions | Priority | Dependencies |
|------|-------|----------|----------|--------------|
| 1 | Config Connections & Test Fix | 1 | HIGH | None |
| 2 | Dead Code Removal | 1-2 | HIGH | None (parallel with Wave 1) |
| 3 | Bug Fixes & Testing | 1-2 | MEDIUM | Wave 1 (for test patterns) |
| 4 | Documentation Updates | 0.5 | LOW | Waves 1-3 complete |

## Dependency Graph

```
Wave 1 (Config/Tests) ─────────────────┐
                                       ├──► Wave 4 (Docs)
Wave 2 (Dead Code) ────────────────────┤
                                       │
Wave 3 (Bugs/Tests) ───────────────────┘
```

Waves 1 and 2 can run in parallel.
Wave 3 can start after Wave 1 (needs serial_test pattern).
Wave 4 should wait for all others to document final state.

## Issue Inventory

### Config Issues (Wave 1)
- [ ] `camera.pitch_limit` not connected
- [ ] `window.fullscreen` not applied on startup
- [ ] `window.vsync` not connected
- [ ] `input.w_rotation_sensitivity` missing builder method
- [ ] `test_env_override` flaky test

### Dead Code (Wave 2)
- [ ] `player.rs` module in rust4d_physics
- [ ] Legacy Simplex4D pipeline in rust4d_render
- [ ] `slice.wgsl` shader file
- [ ] `thickness` field in Hyperplane4D

### Bugs (Wave 3)
- [ ] Orphaned physics bodies on entity removal
- [ ] Zero tests for rust4d_input crate

### Documentation (Wave 4)
- [ ] Phase 3B status wrong in roadmap
- [ ] README "scene serialization" status wrong
- [ ] ARCHITECTURE.md missing render->input dependency

## Files

- [Wave 1: Config Connections & Test Fix](./wave-1-config-and-tests.md)
- [Wave 2: Dead Code Removal](./wave-2-dead-code-removal.md)
- [Wave 3: Bug Fixes & Testing](./wave-3-bugs-and-testing.md)
- [Wave 4: Documentation Updates](./wave-4-documentation.md)

## Success Criteria

1. All compiler warnings eliminated
2. All tests pass (including previously flaky test)
3. Config values either connected or removed with explanation
4. No dead code modules remaining
5. Documentation accurately reflects codebase state

---

**Source**: `scratchpad/reports/2026-01-28-codebase-review/final-synthesis-report.md`
