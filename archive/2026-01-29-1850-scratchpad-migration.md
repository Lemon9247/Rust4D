# Session Report: Multi-Swarm Skill & Scratchpad Migration

**Date**: 2026-01-29 18:50
**Focus**: Designing the multi-swarm Queen protocol and migrating the scratchpad to an orphan branch

---

## Summary

This session had two major pieces of work. First, we designed and built the `/multi-swarm` skill -- a coordination protocol for running multiple parallel swarms across git worktrees, with a "Queen" (the main Claude instance) orchestrating everything. Second, we migrated the existing `scratchpad/` directory from `main` to an orphan git branch mounted as an in-repo worktree, setting up the infrastructure the multi-swarm system relies on.

## What Was Done

### Multi-Swarm Skill Design
- **What**: Reviewed Willow's brainstorm and a Gemini-generated draft, then refined the design through iterative discussion and built `.claude/skills/multi-swarm/SKILL.md`
- **Why**: Running multiple swarms in parallel across git branches needs shared infrastructure (scratchpad) and coordination (who talks to whom, who commits where)
- **Files**: `.claude/skills/multi-swarm/SKILL.md`, `CLAUDE.md`

### Scratchpad Migration
- **What**: Moved all 157 scratchpad files from `main` to an orphan `scratchpad` branch, mounted as a worktree at `scratchpad/` inside the repo
- **Why**: The multi-swarm system needs a shared scratchpad accessible from all worktrees. An orphan branch mounted as a worktree gives version control without branch entanglement
- **Files**: `.gitignore`, `CLAUDE.md`, all four skill files, `docs/developer-guide.md`

### Skill Fixes
- **What**: Removed `disable-model-invocation: true` from `/plan` and `/report` skills
- **Why**: This flag was preventing the Skill tool from invoking them

## Decisions Made

| Decision | Rationale | Alternatives Considered |
|----------|-----------|------------------------|
| No symlinks for scratchpad sharing | Fragile, platform-dependent, needs per-worktree setup | Gemini's approach: symlink scratchpad/ to .hive-anchor/ |
| Discovery via `git worktree list` | Zero memory required across context resets, git is source of truth | Convention-based paths, config file |
| Orchestrator-only commits to scratchpad | Avoids concurrent git operations in shared worktree | Individual agents/swarms commit independently |
| Compose with `/swarm`, don't replace | Avoids duplicating hive-mind/agent/synthesis templates | Standalone skill with own templates |
| Swarm leader hierarchy | Clean boundary: leaders handle cross-swarm, agents stay focused | Flat agent pool managed by orchestrator |
| "Queen" naming | Fun, fits the swarm metaphor, Willow's suggestion | "Orchestrator", "Coordinator" |
| Mount worktree inside repo at `scratchpad/` | Same relative path as before, skills barely need updating | Sibling directory `../Rust4D-scratchpad/` (original plan) |

## Challenges / Gotchas

- **`git rm -rf .` on orphan branch**: This removes files from the working tree too, not just the index. Had to copy scratchpad to `/tmp` first before creating the orphan branch.
- **Leftover gitignored dirs on orphan branch**: `config/` and `target/` survived the `git rm -rf .` because they were gitignored. Had to be selective with `git add` rather than adding everything.
- **In-repo worktree was better than sibling**: Original plan was `../Rust4D-scratchpad/`. Willow pointed out we could mount inside the repo since it's gitignored -- this kept all relative paths working and simplified everything.

## Open Questions

- [ ] Should the `/swarm` skill be updated to auto-discover the shared scratchpad path, or is the current relative path (`scratchpad/reports/...`) sufficient?
- [ ] How will this work when a new contributor clones the repo? They'd need to `git worktree add scratchpad scratchpad` manually. Should this be in the README or a setup script?
- [ ] The multi-swarm system hasn't been tested end-to-end yet. First real use will reveal rough edges.

## Next Steps

- [ ] Test the `/multi-swarm` skill on a real parallel task
- [ ] Consider adding scratchpad worktree setup to contributor onboarding docs
- [ ] The multi-swarm skill still references `../$(basename $(pwd))-<name>` for feature worktrees -- consider whether those should also be inside the repo

## Technical Notes

The full hierarchy for multi-swarm:
```
Queen (main Claude session)
├── Swarm A Leader (Task agent or separate session, in worktree A)
│   └── Agents (spawned by leader, work in worktree A)
├── Swarm B Leader (in worktree B)
│   └── Agents
└── Shared scratchpad (orphan branch worktree at scratchpad/)
```

Committing to the scratchpad branch:
```bash
cd scratchpad && git add . && git commit -m "message" && cd ..
```

The scratchpad branch is completely independent of main -- no shared history, no merge conflicts possible between the two.

---

*Session duration: ~45 turns*
