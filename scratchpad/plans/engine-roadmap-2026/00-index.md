# Rust4D Engine Roadmap 2026

**Created:** 2026-01-27
**Status:** Active Planning

---

## Overview

This folder contains detailed implementation plans for the Rust4D engine roadmap. Plans are organized by phase and priority.

## Plan Index

### Near-Term (Active Development)

| Plan | Sessions | Priority | Status |
|------|----------|----------|--------|
| [Phase 1: Foundation](./phase-1-foundation.md) | 4 | P0 | Ready |
| [Phase 2: Scene Management](./phase-2-scene-management.md) | 3 | P1 | Ready |
| [Phase 3: Documentation](./phase-3-documentation.md) | 4-5 | P2 | Ready |
| [Phase 4: Architecture](./phase-4-architecture.md) | 4 | P3 | Ready |
| [Phase 5: Advanced Features](./phase-5-advanced-features.md) | 5-6 | P4 | Ready |

### Long-Term (Future Consideration)

| Plan | Sessions | Priority | Status |
|------|----------|----------|--------|
| [ECS Migration](./long-term-ecs.md) | 8-12 | P6 | Draft |
| [Visual Scene Editor](./long-term-visual-editor.md) | 10-15 | P7 | Draft |
| [Scripting System](./long-term-scripting.md) | 6-8 | P6 | Draft |
| [Networking](./long-term-networking.md) | 15-20 | P8 | Draft |
| [Advanced Rendering](./long-term-rendering.md) | 8-10 | P6 | Draft |

---

## Timeline Summary

```
Phase 1 ████████ (4 sessions)     <- START HERE
Phase 2 ██████ (3 sessions)       <- Depends on Phase 1A
Phase 3 ████████ (4-5 sessions)   <- Can run parallel
Phase 4 ████████ (4 sessions)     <- Depends on Phase 1B
Phase 5 ██████████ (5-6 sessions) <- Depends on Phase 2

Total Near-Term: 20-22 sessions
```

## Parallelization Strategy

```
Wave 1 (Sessions 1-2):
├── Agent A: Scene Serialization (Phase 1A)
└── Agent B: Configuration System (Phase 1B)

Wave 2 (Sessions 3-4):
├── Agent A: Scene Manager (Phase 2A)
└── Agent B: Examples + README (Phase 3A)

Wave 3 (Sessions 5-7):
├── Agent A: Prefab System (Phase 2B)
├── Agent B: User Guide (Phase 3B)
└── Agent C: Developer Guide (Phase 3B)

Wave 4 (Sessions 8-11):
├── Agent A: System Extraction (Phase 4A)
└── Agent B: Error Handling (Phase 4B)

Wave 5 (Sessions 12-16):
└── Phase 5 features as needed
```

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-27 | RON for scenes, TOML for config | RON handles Rust types well; TOML is human-friendly for settings |
| 2026-01-27 | Figment for config loading | Hierarchical overrides, env var support |
| 2026-01-27 | Defer ECS migration | Current Entity works; migrate when pain points emerge |
| 2026-01-27 | Architecture graphs required | ARCHITECTURE.md must include Mermaid diagrams |

---

## How to Use These Plans

1. **Starting a phase:** Read the phase document, understand dependencies
2. **Parallel work:** Check the parallelization notes in each plan
3. **Swarm execution:** Each phase can be executed by 1-3 agents
4. **Progress tracking:** Update status in plan headers as work completes
5. **Long-term plans:** Review when near-term phases complete

## Source Documents

These plans were synthesized from the engine review swarm:
- `scratchpad/reports/2026-01-27-engine-review-swarm/documentation-review.md`
- `scratchpad/reports/2026-01-27-engine-review-swarm/architecture-review.md`
- `scratchpad/reports/2026-01-27-engine-review-swarm/config-recommendations.md`
- `scratchpad/reports/2026-01-27-engine-review-swarm/scene-handling-review.md`
- `scratchpad/reports/2026-01-27-engine-review-swarm/roadmap-draft.md`
