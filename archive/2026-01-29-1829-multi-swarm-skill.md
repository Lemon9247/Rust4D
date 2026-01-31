# Session Report: Multi-Swarm Skill Design

**Date**: 2026-01-29 18:29
**Focus**: Designing and implementing the `/multi-swarm` Queen protocol skill

---

## Summary

Reviewed Willow's brainstorm and a Gemini-generated draft for multi-swarm coordination, then refined the design through discussion and built the `/multi-swarm` skill. The core idea: multiple swarms work in parallel across git worktrees, sharing a scratchpad on an orphan git branch, coordinated by a "Queen" (the main Claude instance).

## What Was Done

### Reviewed existing ideas
- Read `scratchpad/ideas/multi-swarm/multi-swarm.md` (Willow's brainstorm on the problem)
- Read `scratchpad/ideas/multi-swarm/gemini-output` (Gemini's formalized draft)
- Identified what to keep and what to throw out

### Design refinement through discussion
- **Removed symlinks** -- unnecessary complexity. The orchestrator passes absolute paths to agents instead.
- **Chose `git worktree list` for discovery** -- no hardcoded paths, no config files. Git already knows where the worktree is. Works across context window resets.
- **Orchestrator-only commits** -- only the Queen commits to the scratchpad branch, avoiding concurrent git operations.
- **Local `.scratchpad/` for throwaway files** -- gitignored, per-worktree, for temp junk that doesn't need to survive.
- **Composition over duplication** -- the multi-swarm skill wraps `/swarm`, doesn't redefine it. Each swarm leader runs the existing swarm protocol internally.
- **Swarm leaders** -- each swarm has a leader (a background Task agent or a separate Claude session) that runs `/swarm` in its worktree. Only leaders handle cross-swarm awareness; agents within a swarm are unaware of other swarms.
- **Queen naming** -- Willow suggested calling the multi-swarm orchestrator "the Queen." It fits the swarm metaphor perfectly.

### Built the skill
- Created `.claude/skills/multi-swarm/SKILL.md` with six phases: infrastructure, task decomposition, coordination file, worktree setup, launch leaders, monitor/synthesize, cleanup
- Updated `CLAUDE.md` with Multi-Swarm Operations section and updated skills list

## Decisions Made

| Decision | Rationale | Alternatives Considered |
|----------|-----------|------------------------|
| No symlinks | Fragile, platform-dependent, needs per-worktree setup | Symlink `scratchpad/` to `.hive-anchor/scratchpad/` (Gemini's approach) |
| Discovery via `git worktree list` | Zero memory required, works across context resets | Convention-based paths, config file |
| Orchestrator-only commits | Avoids concurrent git operations in shared worktree | Agents commit individually |
| Compose with `/swarm` | Avoids duplicating hive-mind/agent/synthesis templates | Standalone skill with its own templates |
| Swarm leader hierarchy | Clean separation of cross-swarm vs intra-swarm coordination | Flat agent pool, orchestrator manages all agents directly |
| "Queen" naming | Fun, fits the swarm metaphor | "Orchestrator", "Coordinator" |

## Open Questions

- [ ] Scratchpad migration: need to move existing `scratchpad/` contents from `main` to the orphan branch and update all skill references
- [ ] Should the `/swarm` skill be updated to auto-discover the shared scratchpad via `git worktree list`? Or keep it simple and have the Queen pass the path?
- [ ] The `/report` and `/plan` skills also reference `scratchpad/` -- these need updating after migration
- [ ] Should scratchpad setup be part of repo initialization guidance in CLAUDE.md?

## Next Steps

- [ ] Migrate existing scratchpad to orphan branch
- [ ] Mount the orphan branch as a worktree
- [ ] Update `/swarm`, `/report`, `/plan` skills to find the shared scratchpad
- [ ] Update CLAUDE.md references to scratchpad paths
- [ ] Test the full flow end-to-end

## Technical Notes

The orphan branch setup sequence:
```bash
git checkout --orphan scratchpad
git rm -rf .
mkdir -p reports plans ideas
git add .
git commit -m "Initialize shared scratchpad"
git checkout main
git worktree add ../Repo-scratchpad scratchpad
```

The worktree can be discovered with:
```bash
git worktree list
# Look for the line ending in [scratchpad]
```

The hierarchy:
```
Queen (main Claude session)
├── Swarm A Leader (Task agent or separate session, in worktree A)
│   └── Agents (spawned by leader, work in worktree A)
├── Swarm B Leader (in worktree B)
│   └── Agents
└── Shared scratchpad (orphan branch worktree)
```

---

*Session duration: ~30 turns*
