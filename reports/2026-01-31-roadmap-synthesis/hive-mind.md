# Hive Mind: Roadmap Synthesis

## Task Overview
Combine the engine roadmap agent reports (6 agents), the engine/game split plan, and the cross-swarm synthesis into a new comprehensive set of roadmap planning documents in `scratchpad/plans/engine-roadmap-2026/`.

## Source Documents
- `scratchpad/plans/2026-01-30-engine-game-split.md` - Engine/game split + ECS migration (5 phases, DECIDED)
- `scratchpad/reports/2026-01-30-engine-roadmap/hive-mind.md` - Cross-phase coordination notes
- `scratchpad/reports/2026-01-30-engine-roadmap/agent-f-report.md` - Foundation phase
- `scratchpad/reports/2026-01-30-engine-roadmap/agent-p1-report.md` - Combat Core
- `scratchpad/reports/2026-01-30-engine-roadmap/agent-p2-report.md` - Weapons & Feedback
- `scratchpad/reports/2026-01-30-engine-roadmap/agent-p3-report.md` - Enemies & AI
- `scratchpad/reports/2026-01-30-engine-roadmap/agent-p4-report.md` - Level Design Pipeline
- `scratchpad/reports/2026-01-30-engine-roadmap/agent-p5-report.md` - Editor & Polish
- `scratchpad/reports/2026-01-30-multi-engine-review/cross-swarm-synthesis.md` - Original integrated roadmap

## Key Principles
1. The split plan (ECS migration + repo split) is DECIDED and comes FIRST
2. Agent F's foundation items happen BEFORE the split plan's ECS migration
3. Post-split phases (P1-P5) happen AFTER the split is complete
4. Each document should preserve full detail from agent reports (file lists, APIs, task lists)
5. Clear engine vs game boundary in every document

## Agents
1. **Agent Index** - Creates the updated `00-index.md` master index with full timeline, dependency graph, and parallelization strategy
2. **Agent Pre-Split** - Creates `split-phase-0-foundation.md` (from Agent F) and `split-phases-1-5.md` (summary of split plan)
3. **Agent Combat** - Creates `post-split-phase-1-combat-core.md` from Agent P1 report
4. **Agent Media** - Creates `post-split-phase-2-weapons-feedback.md` from Agent P2 report
5. **Agent Enemies** - Creates `post-split-phase-3-enemies-ai.md` from Agent P3 report
6. **Agent Levels** - Creates `post-split-phase-4-level-design.md` from Agent P4 report
7. **Agent Editor** - Creates `post-split-phase-5-editor-polish.md` from Agent P5 report
8. **Agent Game** - Creates `game-roadmap-summary.md` consolidating all game-side work from all agent reports

## Coordination Notes
- Each agent writes their planning document(s) directly to `scratchpad/plans/engine-roadmap-2026/`
- Each agent also writes a brief completion report to this swarm folder
- Agents should preserve full implementation detail from source reports
- The Index agent should wait for others or work from the source reports directly

## Questions for Discussion
(Agents add cross-cutting questions here)

## Status
- [ ] Agent Index: Pending
- [ ] Agent Pre-Split: Pending
- [ ] Agent Combat: Pending
- [ ] Agent Media: Pending
- [ ] Agent Enemies: Pending
- [ ] Agent Levels: Pending
- [ ] Agent Editor: Pending
- [ ] Agent Game: Pending
- [ ] Final synthesis: Pending

## Reports Generated
(Update as reports are written)

## Key Findings
(Summarize major discoveries as they emerge)
