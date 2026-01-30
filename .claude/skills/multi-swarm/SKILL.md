---
name: multi-swarm
description: Orchestrate multiple swarms across git worktrees with shared scratchpad coordination
argument-hint: [task description]
---

# Multi-Swarm: Queen Protocol

You are the **Queen** -- the coordinating instance that manages multiple parallel swarms across git worktrees. You spawn all agents directly, handle cross-swarm coordination, and write the final synthesis.

Coordinate swarms to work on: **$ARGUMENTS**

## Architecture Constraint

**Sub-agents cannot spawn their own sub-agents.** The Task tool has a nesting depth of 1. This means:

- The Queen spawns ALL agents directly (flat fan-out)
- There are no "leader" agents -- the Queen IS the leader for every swarm
- Agents do their work and write reports; the Queen reads reports and synthesizes

```
Queen (you)
├── Swarm A agents (all spawned directly by Queen)
│   ├── Agent A1 (background task)
│   ├── Agent A2 (background task)
│   └── Agent A3 (background task)
├── Swarm B agents (all spawned directly by Queen)
│   ├── Agent B1 (background task)
│   └── Agent B2 (background task)
└── Shared scratchpad (orphan branch worktree, Queen commits here)
```

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

Mount it as a worktree inside the repo (it's gitignored on main):

```bash
git worktree add scratchpad scratchpad
```

The resulting path is `$SCRATCHPAD`. Verify it contains `reports/`, `plans/`, `ideas/`.

---

## Phase 1: Task Decomposition

1. Analyze the task and identify **independent streams** of work (swarms).
2. For each swarm, identify the **individual agents** needed.
3. Each agent gets:
   - A **name** (e.g., "Agent A1: Core Analysis")
   - A **specific task** with clear scope
   - A **report path** to write findings to
   - The **worktree path** to read code from
4. Present the full decomposition (swarms + agents) to the user for approval before proceeding.

---

## Phase 2: Coordination File & Hive-Mind Files

Create the multi-swarm task folder:

```bash
mkdir -p $SCRATCHPAD/reports/YYYY-MM-DD-multi-<task>/{swarm-a,swarm-b}/
```

Write the coordination file at `$SCRATCHPAD/reports/YYYY-MM-DD-multi-<task>/coordination.md`:

```markdown
# Queen's Coordination: [Task Name]
**Date**: YYYY-MM-DD

## Objective
[What the overall task is trying to accomplish]

## Swarms
| Swarm | Branch | Worktree Path | Status |
|-------|--------|---------------|--------|
| [name] | `feature/...` | `/path/to/worktree` | Pending |

## Agents
| Agent | Swarm | Focus | Status |
|-------|-------|-------|--------|
| A1 | swarm-a | [focus] | Pending |
| A2 | swarm-a | [focus] | Pending |
| B1 | swarm-b | [focus] | Pending |

## Cross-Swarm Notes
(Queen posts findings relevant across swarms as agents complete.)

## Completion Checklist
- [ ] All agents complete
- [ ] Per-swarm synthesis written (by Queen)
- [ ] Cross-swarm synthesis written (by Queen)
- [ ] Scratchpad committed
```

Write a hive-mind file for each swarm at `$SCRATCHPAD/reports/YYYY-MM-DD-multi-<task>/<swarm>/hive-mind.md`. This gives agents within a swarm shared context.

---

## Phase 3: Worktree Setup

For each swarm that needs its own worktree:

1. **Create the worktree**:
   ```bash
   git worktree add ../$(basename $(pwd))-<swarm-name> <branch-name>
   ```

2. If using an existing branch/worktree, verify it's in the expected state.

If a swarm operates on the main worktree, no setup is needed.

---

## Phase 4: Launch All Agents

**Spawn all agents in a single message** using multiple parallel Task tool calls with `run_in_background: true`. This is the key step -- launch everything at once.

Each agent prompt should include:
- Their specific task and scope
- The absolute worktree path to read code from
- The report file path to write to
- The hive-mind file path for shared context
- Clear instruction that this is read-only (for research tasks) or what files they own (for implementation tasks)

Example agent prompt:

```
You are Agent A1 analysing [specific area] of the [project] codebase.

This is a RESEARCH task -- do NOT modify any code files. Only read, search, and write your report.

## Your Workspace
- Code to analyse: [absolute worktree path]
- Write your report to: [absolute report path]
- Hive-mind file (shared context): [absolute hive-mind path]

## Your Task
[Detailed description of what to analyse]

Read every file in [specific directories]. Report on:
1. [Specific aspect]
2. [Specific aspect]
3. [Specific aspect]

Reference file paths and line numbers in your findings.
```

Update the coordination file agent statuses to "Running".

Use **TaskCreate/TaskUpdate** to track each agent in the UI.

---

## Phase 5: Monitor & Synthesize

1. **Wait** for all agents to complete (use TaskOutput with blocking).
2. **Read all agent reports** from the scratchpad.
3. **Write per-swarm synthesis** for each swarm at `$SCRATCHPAD/reports/YYYY-MM-DD-multi-<task>/<swarm>/synthesis.md`:
   - Combine findings from that swarm's agents
   - Identify the top issues within that swarm's scope
4. **Write cross-swarm synthesis** at `$SCRATCHPAD/reports/YYYY-MM-DD-multi-<task>/cross-swarm-synthesis.md`:

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
[Cross-cutting concerns, shared issues, conflicts between swarms]

## Next Steps
- [ ] Action item 1
- [ ] Action item 2

## Sources
- [Swarm 1 Synthesis](./swarm-1/synthesis.md)
- [Swarm 2 Synthesis](./swarm-2/synthesis.md)
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
