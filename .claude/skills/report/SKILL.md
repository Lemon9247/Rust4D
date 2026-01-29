---
name: report
description: Write a session report documenting work completed and save to scratchpad/reports
argument-hint: [topic or leave blank to auto-generate]
---

# Write Session Report

Document this session's work and save to scratchpad/reports.

Topic/focus: **$ARGUMENTS** (if blank, infer from conversation)

## Report Process

1. **Review the Session**
   - What was the original request/goal?
   - What was accomplished?
   - What decisions were made and why?
   - What remains unfinished or uncertain?

2. **Generate Report**
   - Filename: `scratchpad/reports/YYYY-MM-DD-HHMM-<topic>.md`
   - Use current date and time
   - Topic should be 2-4 words in kebab-case

3. **Key Sections**
   - Don't just document "what" - document "why"
   - Include reasoning behind decisions
   - Note open questions for future sessions
   - Record any gotchas or surprises discovered

## Report Template

```markdown
# Session Report: [Topic]

**Date**: YYYY-MM-DD HH:MM
**Focus**: [Brief description of main work]

---

## Summary

[2-3 sentence summary of what was accomplished]

## What Was Done

### [Task/Feature 1]
- What: [Description]
- Why: [Reasoning]
- Files touched: [list]

### [Task/Feature 2]
(repeat as needed)

## Decisions Made

| Decision | Rationale | Alternatives Considered |
|----------|-----------|------------------------|
| [Choice] | [Why] | [What else was considered] |

## Challenges / Gotchas

- [Challenge 1 and how it was resolved]
- [Gotcha discovered for future reference]

## Open Questions

- [ ] [Question that needs future investigation]
- [ ] [Uncertainty that should be resolved]

## Next Steps

- [ ] [Suggested follow-up work]
- [ ] [Things left to implement]

## Technical Notes

[Any technical details, code snippets, or architecture notes worth preserving]

---

*Session duration: ~X turns*
```

## Writing Tips

- **Be specific**: "Changed X to Y because Z" not "Updated code"
- **Capture uncertainty**: Future you will want to know what you weren't sure about
- **Include context**: Someone reading this later should understand without reading the full conversation
- **Note surprises**: If something didn't work as expected, document it
- **Link to code**: Reference specific files and line numbers when relevant

## After Writing

1. Show the user the report location
2. Offer to add anything they think is missing
3. Commit the report to the scratchpad branch: `cd scratchpad && git add . && git commit -m "Add session report: <topic>" && cd ..`
