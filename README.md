# Shared Scratchpad

This is an orphan branch containing Claude Code's documentary memory for the Rust4D project. It has no shared history with `main`.

## What's here

- **reports/** -- Session reports documenting what was done, why, and what's left unresolved
- **plans/** -- Implementation plans and architecture documents
- **ideas/** -- Feature proposals and design explorations
- **archive/** -- Historical documents from earlier phases of the project

## How it works

This branch is mounted as a git worktree inside the main repo at `scratchpad/`. It's gitignored on `main` so the two branches don't interfere with each other.

```bash
# Mount the scratchpad (from the repo root)
git worktree add scratchpad scratchpad

# Commit scratchpad changes
cd scratchpad && git add . && git commit -m "message" && cd ..
```

When running multiple swarms across worktrees, all agents share this scratchpad via absolute paths discovered with `git worktree list`. See the `/multi-swarm` skill for details.
