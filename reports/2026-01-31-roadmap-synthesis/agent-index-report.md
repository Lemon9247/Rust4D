# Agent Index Report: Master Roadmap Index

**Agent**: Index Agent
**Date**: 2026-01-31
**Task**: Create the updated master index for the Rust4D engine roadmap

## What I Did

Rewrote `scratchpad/plans/engine-roadmap-2026/00-index.md` from scratch to serve as the definitive master index for the entire Rust4D engine roadmap. The new document consolidates all planning work from the engine review swarms, the engine/game split plan, and the roadmap planning swarm into a single navigable reference.

## Source Documents Read

I read all 11 source documents:
- The existing `00-index.md` (for format and completed phase details)
- `split-phase-0-foundation.md` (Foundation pre-split tasks)
- `split-phases-1-5.md` (Engine/game split summary)
- `post-split-phase-1-combat-core.md` through `post-split-phase-5-editor-polish.md` (all 5 post-split phases)
- `game-roadmap-summary.md` (game-side roadmap)
- `hive-mind.md` (cross-phase coordination notes from the planning swarm)
- `cross-swarm-synthesis.md` (original integrated roadmap for strategic context)

## Key Decisions in the Index

### Document Structure

Organized the index into three parts reflecting the project's actual progression:
1. **Part I: Completed Work** -- preserves all completion notes from the original index
2. **Part II: Engine/Game Split** -- Stage 1 (foundation) and Stage 2 (split phases)
3. **Part III: Post-Split Engine Features** -- all 5 post-split phases plus game repo roadmap

### Dependency Graph

Created an ASCII dependency graph showing the complete flow from completed phases through foundation, split, and all post-split phases. Includes the game repo phases running in parallel. Highlighted that P4 Wave 1 (shape types) has zero dependencies and can start immediately.

### Session Estimates

Computed totals from individual plan documents:
- Engine total remaining: 35.25-42.75 sessions (sequential)
- Game repo: 13-21 sessions (parallel with engine)
- Critical path with parallelism: ~22-29 sessions

### Decision Log

Preserved all 4 original decisions and added 10 new entries capturing the key architectural choices from the roadmap planning swarm:
- Full hecs over partial ECS
- Engine/game split
- kira over rodio
- Triplanar mapping before UV pipeline
- egui for both HUD and editor
- 3D particles (not 4D)
- Health/damage is game-side
- Collision layer presets stay in engine
- Declarative trigger system
- Git URL hybrid dependency model

### Legacy Plans

Explicitly marked the 5 original long-term plans as SUPERSEDED with clear pointers to their replacements. Networking is marked as "Deferred" since it has no current replacement plan.

## Observations

1. The roadmap is large but highly parallelizable. The post-split phases have extensive internal parallelism documented in each plan.

2. P5 (Editor & Polish) is both the largest phase (10-12.5 sessions) and has the deepest dependency chain. It defines the critical path tail.

3. P4 Wave 1 (shape types -- Hyperprism4D and Hypersphere4D) is uniquely valuable as a parallel task because it has zero dependencies on any other work. Any available agent can start it at any time.

4. The particle system is shared between P2 and P3. The plans coordinate on this (P3 designed the comprehensive system, P2 coordinates on API via `ParticleSystem::spawn_burst()`), but implementation should be done once, not twice.

5. The game repo phases are structured to start as soon as each engine phase delivers, enabling maximum overlap between engine and game work.
