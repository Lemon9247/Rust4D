---
name: swarm
description: Start a multi-agent task with hive-mind coordination for complex research or implementation
argument-hint: [task description]
---

# Multi-Agent Swarm Task

Initialize a swarm of agents to work on: **$ARGUMENTS**

## Setup Process

1. **Create Task Folder**
   - Create subfolder: `scratchpad/reports/YYYY-MM-DD-<task-name>/`
   - Use a short, descriptive name for the task (kebab-case)

2. **Create Hive-Mind Coordination File**
   - Create `hive-mind-<task>.md` in the task folder
   - Use the template below

3. **Identify Agents Needed**
   - Determine what specialized agents would help
   - Common agent types:
     - **Research Agent** - Investigates patterns, reads docs, gathers context
     - **Codebase Agent** - Reviews internal code structure and patterns
     - **Architecture Agent** - Designs high-level structure
     - **Implementation Agent** - Writes code
     - **Testing Agent** - Writes and runs tests
     - **Documentation Agent** - Updates docs and comments

4. **Launch Agents**
   - Use the Task tool to spawn subagents
   - Give each agent clear, specific instructions
   - Point them to the hive-mind file for coordination

5. **Synthesize Results**
   - When all agents complete, write `final-synthesis-report.md`
   - Update hive-mind file with completion status

## Hive-Mind File Template

```markdown
# Hive Mind: [Task Name]

## Task Overview
[Clear description of what we're trying to accomplish]

## Agents
1. **[Agent Name]** - [Brief role description]
2. **[Agent Name]** - [Brief role description]

## Coordination Notes
- Each agent should write findings to separate markdown files in this folder
- Focus areas: [list key areas to investigate]
- **Cross-agent coordination**: Agents can read and write to this file to share discoveries, ask questions, or flag dependencies

## Questions for Discussion
(Agents can add questions here - other agents should check this section and respond)

## Status
- [ ] [Agent 1 Name]: [Status]
- [ ] [Agent 2 Name]: [Status]
- [ ] Final synthesis: Pending

## Reports Generated
(Update as reports are written)

## Key Findings
(Summarize major discoveries as they emerge)
```

## Agent Instructions Template

When spawning an agent, provide instructions like:

```
You are the [Role] Agent working on [task].

Your mission:
1. [Specific goal 1]
2. [Specific goal 2]

Coordination:
- Read the hive-mind file at scratchpad/reports/YYYY-MM-DD-<task>/hive-mind-<task>.md
- Write your findings to scratchpad/reports/YYYY-MM-DD-<task>/<agent-name>-report.md
- **Cross-agent communication**: If you discover something relevant to other agents, or have questions, add them to the hive-mind file's "Questions for Discussion" section. Check this section periodically for questions from other agents that you can answer.

Focus on [specific aspects]. Do not [any constraints].

When complete, summarize your key findings at the top of your report.
```

## Final Synthesis Report Template

```markdown
# [Task Name] - Final Synthesis Report

**Date**: YYYY-MM-DD
**Task**: [Description]

---

## Executive Summary
[3-5 sentence summary of findings and recommendations]

## Part 1: [Topic from Agent 1]
[Synthesized findings]

## Part 2: [Topic from Agent 2]
[Synthesized findings]

## Recommendations
1. [Key recommendation]
2. [Key recommendation]

## Next Steps
- [ ] Action item 1
- [ ] Action item 2

## Sources
- [Agent 1 Report](./agent1-report.md)
- [Agent 2 Report](./agent2-report.md)
```

## After Setup

1. Show the user the hive-mind file location
2. Launch the agents in parallel using Task tool
3. **Display status table** showing all agents and their current state:
   ```
   | Agent | Focus | Status |
   |-------|-------|--------|
   | Documentation Agent | READMEs, docs | Running |
   | Architecture Agent | main.rs structure | Complete |
   ```
4. **Use TaskCreate/TaskUpdate** to track agent progress - create a task for each agent so progress is visible in the UI
5. **Don't interrupt running agents** - avoid starting new unrelated work while agents are running. Wait for notifications that agents have completed.
6. Update the status table as agents finish
7. Write the synthesis report when all agents complete
8. Commit swarm reports to the scratchpad branch: `cd scratchpad && git add . && git commit -m "Add swarm reports: <task>" && cd ..`
