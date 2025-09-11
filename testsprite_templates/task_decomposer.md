# Task Decomposer Template

## Task Brief (3-5 lines)
**Scope:** [Brief description of what needs to be done]
**Success Criteria:** [How we'll know it's complete]
**Blast Radius:** [Estimated files/changes affected]
**Model Selection:** [simple_models for basic edits, advanced_models for complex work]

## Workflow Selection
- **WF-DEV-CHANGE**: For new features, bug fixes, or safe modifications
- **WF-REFACTOR**: For code cleanup, SOLID/DRY improvements, or architectural changes

## Pre-Implementation Checklist
- [ ] Input files < 500KB (chunk if larger)
- [ ] Max changed files set (≤10 for refactors)
- [ ] Custom deviations identified and documented
- [ ] Appropriate model selected for complexity

## Implementation Plan
1. [Step 1]
2. [Step 2]
3. [Step 3]

## Output Requirements
- [ ] task_brief.md
- [ ] patch_summary.md
- [ ] next_steps.md
- [ ] deviation_notes.md (if applicable)
