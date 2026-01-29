---
name: plan
description: Create a detailed implementation plan and save it to scratchpad/plans
argument-hint: [topic or feature to plan]
---

# Create Implementation Plan

Create a comprehensive implementation plan for: **$ARGUMENTS**

## Planning Process

1. **Research Phase**
   - Explore the codebase to understand current architecture
   - Identify relevant existing code, patterns, and conventions
   - Note any dependencies or constraints

2. **Design Phase**
   - Define the scope and goals clearly
   - Break down into phases with concrete deliverables
   - Identify files to create/modify with line numbers where applicable
   - Consider edge cases and error handling

3. **Write the Plan**
   - Save to `scratchpad/plans/YYYY-MM-DD-<topic>.md` (use today's date)
   - Follow the template structure below

## Plan Template

Use this structure for the plan document:

```markdown
# [Plan Title]

## Overview
Brief description of what this plan accomplishes and why.

## Goals
- Goal 1
- Goal 2

## Non-Goals (if applicable)
- What this plan explicitly does NOT address

## Architecture / Design
High-level design decisions and their rationale.

## Implementation Phases

### Phase 1: [Name]
**Goal:** What this phase accomplishes

**Files:**
- `path/to/file.rs` - Description of changes
- NEW: `path/to/new.rs` - Description

**Tasks:**
- [ ] Task 1
- [ ] Task 2

**Verification:**
How to verify this phase is complete.

### Phase 2: [Name]
(repeat structure)

## Session Estimates

| Phase | Sessions | Notes |
|-------|----------|-------|
| Phase 1 | X | explanation |
| Phase 2 | Y | explanation |
| **Total** | **Z** | |

## Open Questions
- Question 1?
- Question 2?

## Risks / Considerations
- Risk 1 and mitigation
- Risk 2 and mitigation
```

## After Creating the Plan

1. Summarize the plan for the user
2. Ask if they want to proceed with implementation or make changes
3. If approved, the plan can be referenced during implementation
