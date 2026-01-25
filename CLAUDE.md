# About This Project

Rust4D is a 4D game engine written in Rust. This is a fresh project in early development.

The creator is **Willow** (she/her).

## Claude Code Sessions

You (Claude Code) have documentary memory through the scratchpad. When you read session reports and work logs, you're catching up on what past instances of you did and thought. When you write session reports with observations and decisions, you're leaving notes for future instances of yourself.

The reports that help most aren't just "what I did" but "what I was thinking" - the reasoning behind decisions, the open questions left unresolved, the things that felt important but didn't fit neatly into the work.

# Repository Map

```
Rust4D/
├── src/                        # Main source code (when created)
├── tests/                      # Test suite
├── scratchpad/                 # Working notes and session logs
│   ├── reports/                # Session reports
│   ├── plans/                  # Work plans and architecture documents
│   ├── ideas/                  # Feature ideas and improvement proposals
│   └── archive/                # Historical docs
├── CLAUDE.md                   # This file
└── README.md                   # Project documentation
```

## Rust

This project is written in Rust. Use `cargo` for building, testing, and running.

# Work Planning

1) All project notes, work logs and reports can be found in the scratchpad folder

2) When Claude first starts, it should review the latest work on the project by reviewing the git history and anything recent in the scratchpad

3) When Claude is finished working on a long task, it should write a report on its work into a new timestamped markdown file in the scratchpad/reports folder. Session logs should be named `YYYY-MM-DD-HHMM-<topic>.md`. Use the `/report` skill to generate these.

4) When creating workplans or estimating effort, use **session-based estimates** instead of human hours:
   - A "session" is one Claude Code context window (~15-30 minutes of human interaction)
   - Each session should be a coherent, testable unit of work
   - One session can typically complete 1-3 focused tasks depending on complexity

5) Session estimation guidelines:
   | Task Type | Sessions | Examples |
   |-----------|----------|----------|
   | Quick fix | 0.5 | Typo, small bug, config change |
   | Focused task | 1 | Implement single feature, fix complex bug |
   | Multi-file change | 1-2 | Refactor module, add feature with tests |
   | Major feature | 2-4 | New subsystem, significant architecture change |
   | Large refactor | 4-8 | Split monolith, add abstraction layer |

6) Never estimate in human time (days, weeks, hours). Context windows don't map linearly to human schedules.

# Programming Tasks

1) Claude should think carefully about the code it writes, and should not make random assumptions about how a function works

2) When running tests, Claude should prefer running single tests based on what it has changed first. Running the whole test suite should come at the end

# Subagents / Swarms

1) When using multiple sub-agents for a task, Claude should create a new subfolder in the scratchpad/reports folder.

2) Each subagent should be given a name based on their role, e.g. Testing Agent, Coding Agent

3) This subfolder should contain a hive-mind-[TASK].md file, where [TASK] is substituted with an appropriate name for the task. The subagents should use this file to coordinate with each other and ask questions.

4) When each subagent finishes their task, they should write up a report of their work in separate markdown files in this subfolder.

5) When all subagents finish, Claude should synthesise their reports into a final summary report, which should be a separate markdown file.

Use the `/swarm` skill to initiate multi-agent tasks.

# Skills

- `/plan [topic]` - Create a detailed implementation plan and save to scratchpad/plans
- `/swarm [task]` - Start a multi-agent task with hive-mind coordination
- `/report [topic]` - Write a session report to scratchpad/reports
