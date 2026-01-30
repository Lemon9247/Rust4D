# Hive Mind: Engine Roadmap Planning

## Task Overview
Build a new engine roadmap for Rust4D that accounts for the engine/game split decision. The cross-swarm synthesis (from the multi-engine review) outlined 6 phases of development assuming a single-repo approach. Now that the project will be split into:

1. **Rust4D Engine** -- generic 4D game engine library
2. **Rust4D Shooter** -- separate game repo for the 4D boomer shooter

...each phase needs to be re-evaluated. For each phase, an agent determines:
- What belongs in the **engine** (generic, reusable)
- What belongs in the **game** (boomer-shooter-specific)
- What the engine must expose as API for the game to build upon
- Detailed implementation plan for the engine side
- Dependencies on other phases and the engine/game split plan

## Key Context
- **Engine/Game Split Plan**: `scratchpad/plans/2026-01-30-engine-game-split.md`
  - Full ECS migration with hecs (decided, not partial)
  - New rust4d_game crate for CharacterController4D, events, FSM
  - Input refactored to action/axis abstraction
  - Git URL hybrid dependency approach
- **Cross-Swarm Synthesis**: `scratchpad/reports/2026-01-30-multi-engine-review/cross-swarm-synthesis.md`
  - 12 P0 gaps identified
  - 5 features requiring 4D adaptation
  - Foundation + 5 phases + deferred phase
- **Original Agent Reports**: `scratchpad/reports/2026-01-30-multi-engine-review/`

## Important Constraint
The engine/game split plan covers ECS migration and the split itself (9.5-14 sessions). That work is ALREADY PLANNED. Each agent here should:
1. ASSUME the split plan is complete (ECS done, rust4d_game exists, game repo exists)
2. Plan what the ENGINE needs AFTER the split to support their phase's features
3. Be clear about what APIs the engine exposes vs what the game implements

## Agents
1. **Agent F** (Foundation) - Fixed timestep + serialization + quick fixes. Reviews what's already in the split plan vs what's additional.
2. **Agent P1** (Combat Core) - Raycasting, event system, health/damage, trigger callbacks. Engine-side APIs.
3. **Agent P2** (Weapons & Feedback) - Audio system, HUD framework, particle system. Engine-side rendering/audio.
4. **Agent P3** (Enemies & AI) - Sprite/billboard rendering, AI framework, area damage. Engine-side support.
5. **Agent P4** (Level Design Pipeline) - RON preview tool, shape types, door/elevator mechanics. Engine-side tooling.
6. **Agent P5** (Editor & Polish) - egui editor, lighting, textures, input rebinding. Engine-side editor/rendering.

## Coordination Notes
- Each agent writes their plan to `agent-[f|p1|p2|p3|p4|p5]-report.md` in this folder
- The Foundation agent (F) identifies prerequisites that other agents should note as dependencies
- All agents should reference the engine/game split plan when deciding engine vs game boundaries
- Agents should check hive-mind for cross-phase dependencies

## Cross-Phase Dependencies to Watch
- Foundation items (serialization, fixed timestep) block almost everything
- Raycasting (P1) is needed by weapons (P2) and enemy AI (P3)
- Event system (P1) is needed by weapons (P2), pickups (P4), and triggers (P4)
- Sprite rendering (P3) informs particle system (P2/P3 overlap)
- Audio system (P2) used by weapons (P2), enemies (P3), and doors (P4)
- Editor (P5) needs all shape types from P4

## Questions for Discussion
(Agents add questions here for other agents to answer)

### From Agent F:
1. **For P1**: Raycasting should use fixed timestep. Does your plan assume fixed timestep is done?
2. **For P5**: Editor needs full physics type serialization (8+ types). Recommend deferring to split plan Phase 2. Compatible with your timeline?
3. **For ALL**: Rotor4 serialization fix changes RON format from `[f32; 8]` arrays to struct fields `{ s: 1.0, b_xy: 0.0, ... }`. Existing scene RON files will need re-export. Be aware.

## Status
- [x] Agent F (Foundation): COMPLETE
- [x] Agent P1 (Combat Core): COMPLETE
- [ ] Agent P2 (Weapons & Feedback): Pending
- [ ] Agent P3 (Enemies & AI): Pending
- [ ] Agent P4 (Level Design Pipeline): Pending
- [ ] Agent P5 (Editor & Polish): Pending
- [ ] Final synthesis: Pending

## Reports Generated
- `agent-f-report.md` - Foundation phase implementation plan (Agent F, 2026-01-30)
- `agent-p1-report.md` - Combat Core engine implementation plan (Agent P1, 2026-01-30)

## Key Findings

### Agent F (Foundation):
- **Foundation is ~1-1.5 sessions, not 2.** The synthesis overestimated because it included Partial ECS (now superseded by full ECS in split plan).
- **Rotor4 serialization is the only blocking item.** It's a prerequisite for ECS component serialization. The fix is trivial (add derives) but has a RON format breaking change.
- **Transform4D has a manual serialization workaround** (rotor4_serde module in transform.rs) that should be removed after the Rotor4 fix.
- **Physics type serialization is a cascade of ~8 types** (not just 2 as the synthesis said), but it can be deferred to Phase 2 of the split plan.
- **Fixed timestep is completely absent.** Physics is frame-rate dependent. Accumulator pattern needed in PhysicsWorld.
- **Diagonal movement is 41-73% faster** due to un-normalized movement direction. In 4D this is worse than 3D (3 movement axes = sqrt(3) speed multiplier).
- **Back-face culling was disabled for debugging** and never re-enabled. May reveal winding order issues in the compute shader.
- **All foundation items should be done BEFORE ECS migration** -- they clean up the codebase the ECS work will touch.
