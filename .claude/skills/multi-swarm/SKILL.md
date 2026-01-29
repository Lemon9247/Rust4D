---
name: multi-swarm
description: Orchestrate multiple swarms across git worktrees with shared scratchpad coordination
argument-hint: [task description]
---

# Multi-Swarm: Queen Protocol

You are the **Queen** -- the coordinating instance that manages multiple parallel swarms. Each swarm has a **leader** that runs the standard `/swarm` protocol in its own git worktree. You handle infrastructure, cross-swarm coordination, and final synthesis.

Coordinate swarms to work on: **$ARGUMENTS**

## Hierarchy

```
Queen (you)
├── Swarm A Leader (background agent in worktree A)
│   ├── Agent 1 (spawned by leader)
│   └── Agent 2
├── Swarm B Leader (background agent in worktree B)
│   ├── Agent 3
│   └── Agent 4
└── Shared scratchpad (orphan branch worktree, you commit here)
```

- **You (Queen)**: Infrastructure, task decomposition, cross-swarm coordination, synthesis
- **Leaders**: Run `/swarm` in their worktree, manage their own agents, check coordination file
- **Agents**: Do the work. They don't know other swarms exist.

---

## Phase 0: Shared Scratchpad Infrastructure

The shared scratchpad lives on an orphan git branch, mounted as a worktree.

### Discover

Run `git worktree list` and look for a worktree on the `scratchpad` branch.

- **If found**: Note the absolute path. This is `$SCRATCHPAD`.
- **If not found**: Set it up (see below).

### First-Time Setup

If the `scratchpad` branch doesn't exist yet:

```bash
# Create orphan branch with scratchpad structure
git checkout --orphan scratchpad
git rm -rf .
mkdir -p reports plans ideas
touch reports/.gitkeep plans/.gitkeep ideas/.gitkeep
git add .
git commit -m "Initialize shared scratchpad"

# Return to previous branch
git checkout main
```

Mount it as a worktree alongside the repo:

```bash
git worktree add ../$(basename $(pwd))-scratchpad scratchpad
```

The resulting path is `$SCRATCHPAD`. Verify it contains `reports/`, `plans/`, `ideas/`.

---

## Phase 1: Task Decomposition

1. Analyze the task and identify **independent streams** of work.
2. Each stream becomes a swarm with:
   - A **name** (short, kebab-case)
   - A **branch name** (e.g., `feature/physics-friction`)
   - A **task description** for the swarm leader
   - Any **dependencies** on other streams (ideally none)
3. Present the decomposition to the user for approval before proceeding.

---

## Phase 2: Coordination File

Create the multi-swarm task folder and coordination file:

```bash
mkdir -p $SCRATCHPAD/reports/YYYY-MM-DD-multi-<task>/
```

Write `$SCRATCHPAD/reports/YYYY-MM-DD-multi-<task>/coordination.md`:

```markdown
# Queen's Coordination: [Task Name]
**Date**: YYYY-MM-DD

## Objective
[What the overall task is trying to accomplish]

## Swarms
| Swarm | Branch | Worktree Path | Leader Status |
|-------|--------|---------------|---------------|
| [name] | `feature/...` | `../Repo-name/` | Pending |

## Cross-Swarm Notes
(Queen posts findings from one swarm that are relevant to another.
Leaders should check this section periodically.)

## Completion Checklist
- [ ] All swarm leaders complete
- [ ] Cross-swarm synthesis written
- [ ] Scratchpad committed
```

---

## Phase 3: Worktree Setup

For each swarm:

1. **Create the feature branch**:
   ```bash
   git branch feature/<name> main
   ```

2. **Add the worktree**:
   ```bash
   git worktree add ../$(basename $(pwd))-<name> feature/<name>
   ```

3. **Create the swarm's folder** in the shared scratchpad:
   ```bash
   mkdir -p $SCRATCHPAD/reports/YYYY-MM-DD-multi-<task>/<swarm-name>/
   ```

If a worktree or branch already exists, verify it's in the expected state rather than recreating it.

---

## Phase 4: Launch Swarm Leaders

### Automated Launch (Task Tool)

Spawn each leader as a **background Task agent**. Provide these instructions:

```
You are a Swarm Leader working in worktree: [absolute path to worktree]

IMPORTANT: All code operations use your worktree path. All reports use the
shared scratchpad path. Use absolute paths for everything.

Your task: [swarm's specific task description]

## Your Workspace
- Code worktree: [absolute worktree path]
- Shared scratchpad: [$SCRATCHPAD]
- Your report folder: [$SCRATCHPAD]/reports/[task-folder]/[swarm-name]/
- Coordination file: [$SCRATCHPAD]/reports/[task-folder]/coordination.md

## Instructions

Run the /swarm protocol for your task:
1. Create your hive-mind file at [report folder]/hive-mind.md
2. Identify and spawn agents for your task
3. Point agents at your hive-mind file and report folder
4. Monitor agents, write synthesis report when complete

## Cross-Swarm Awareness
- Check the coordination file for notes from the Queen
- If you discover something relevant to other swarms, add it to the
  "Cross-Swarm Notes" section of the coordination file
- When complete, update your status to "Complete" in the coordination file
```

Update the coordination file as leaders are launched (set status to Running).

Use **TaskCreate/TaskUpdate** to track each swarm leader in the UI.

### Manual Launch (Separate Sessions)

For larger tasks where automated agents may not have enough context, instruct the user to open separate Claude Code sessions:

```
cd ../Repo-<name> && claude
```

Provide the user with:
- Each session's task description
- The shared scratchpad path
- The coordination file path
- The swarm's report folder path

Each session runs `/swarm [task]` and writes to the shared scratchpad.

---

## Phase 5: Monitor & Synthesize

1. **Monitor**: Check the coordination file and swarm report folders for completed synthesis reports.
2. **Cross-pollinate**: If one swarm's findings are relevant to another, post notes in the coordination file's "Cross-Swarm Notes" section.
3. **Wait**: Don't start synthesis until all leaders report complete.
4. **Synthesize**: Write `$SCRATCHPAD/reports/YYYY-MM-DD-multi-<task>/cross-swarm-synthesis.md`:

```markdown
# Cross-Swarm Synthesis: [Task Name]
**Date**: YYYY-MM-DD

## Summary
[How the streams of work fit together]

## Per-Swarm Results

### [Swarm 1 Name] (`feature/...`)
[Key outcomes, referencing their synthesis report]

### [Swarm 2 Name] (`feature/...`)
[Key outcomes]

## Integration Notes
[Anything needed to merge the branches -- conflicts, shared types,
ordering of merges, etc.]

## Next Steps
- [ ] Merge branch `feature/...` into main
- [ ] Merge branch `feature/...` into main
- [ ] [Any follow-up work]

## Sources
- [Swarm 1 Synthesis](./swarm-1/final-synthesis-report.md)
- [Swarm 2 Synthesis](./swarm-2/final-synthesis-report.md)
```

---

## Phase 6: Cleanup

1. **Commit the shared scratchpad**:
   ```bash
   cd $SCRATCHPAD && git add . && git commit -m "Sync: [task] multi-swarm reports"
   ```

2. **Push if remote exists**:
   ```bash
   cd $SCRATCHPAD && git push origin scratchpad
   ```

3. **Branch merging**: Present the list of branches and integration notes to the user. Let them decide merge order and timing.

4. **Worktree removal** (ask user first):
   ```bash
   git worktree remove ../Repo-<name>
   ```
