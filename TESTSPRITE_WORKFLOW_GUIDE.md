# TestSprite Workflow Guide for IPPAN

## Quick Start

### 1. Setup
The TestSprite guardrails are now configured in your project:
- `testsprite.prd.yaml` - Main guardrails configuration
- `testsprite_templates/` - Template files for outputs

### 2. Starting a Task

#### Option A: Use TestSprite MCP Tools
```yaml
use_prd: "./testsprite.prd.yaml"
workflow: "WF-DEV-CHANGE"   # or "WF-REFACTOR"
```

#### Option B: Manual Workflow
1. **Start with Task Decomposer**: Copy the template from `testsprite_templates/task_decomposer.md`
2. **Fill out the brief**: 3-5 line summary of scope and success criteria
3. **Select workflow**: Choose between `WF-DEV-CHANGE` or `WF-REFACTOR`
4. **Check constraints**: Ensure files < 500KB, max 10 changed files for refactors

### 3. Workflow Types

#### WF-DEV-CHANGE (Safe Dev Change)
Use for:
- New features
- Bug fixes
- Safe modifications
- Configuration changes

Steps:
1. Create 3–5 line task brief
2. Estimate blast radius; set max_changed_files
3. If input >500 KB, chunk appropriately
4. Check for custom deviations; add rationale
5. Choose appropriate model
6. Implement smallest vertical slice; run tests
7. Generate patch summary & next steps

#### WF-REFACTOR (SOLID/DRY Refactor)
Use for:
- Code cleanup
- SOLID principle improvements
- DRY violations
- Architectural improvements

Steps:
1. Identify duplication & responsibilities
2. Create helper/module; keep hunks ≤120 lines
3. Update imports & call sites incrementally
4. Run unit tests; summarize changes

### 4. Required Outputs

After each task, generate these files:

#### task_brief.md
- 3-5 line summary
- Scope and success criteria
- Files modified
- Model used and rationale

#### patch_summary.md
- List of changed files
- Key changes with rationale
- Testing status
- Follow-up actions

#### next_steps.md
- Immediate actions with priorities
- Future considerations
- Dependencies
- Additional notes

#### deviation_notes.md (conditional)
- Only when diverging from framework norms
- Custom requirements and rationale
- Impact assessment
- Alternative approaches considered

### 5. Model Selection Guidelines

#### Simple Models (gpt-4o-mini, o4-mini)
Use for:
- Static content edits
- Formatting changes
- Simple bug fixes
- Documentation updates

#### Advanced Models (gpt-5, gpt-5-thinking)
Use for:
- Design decisions
- Architecture changes
- Complex debugging
- Multi-file refactoring

### 6. File Processing Limits

- **File Size**: Max 500KB per file (chunk if larger)
- **Changed Files**: Max 10 files for refactors
- **Hunk Size**: Max 120 lines per change
- **Context**: Chunk large rule sets to ≤200 lines

### 7. Acceptance Criteria

Every task must meet:
- [ ] Changes compile/build with zero errors
- [ ] Unit tests added/updated for new logic
- [ ] Deviation notes present when diverging from norms
- [ ] Patch summary includes changed files, rationale, and follow-ups

### 8. Fail-Fast Rules

- If file/context exceeds limits after chunking → stop and request confirmation
- If acceptance criteria cannot be met in one pass → produce partial patch and TODOs

## Example Usage

### Example 1: Adding a New Feature
```markdown
# Task Decomposer

## Task Brief
**Scope:** Add quantum-resistant key generation to the crypto module
**Success Criteria:** New PQC keys can be generated and validated
**Blast Radius:** 3-4 files (crypto module, tests, config)
**Model Selection:** advanced_models (complex crypto implementation)

## Workflow Selection
WF-DEV-CHANGE - New feature implementation
```

### Example 2: Refactoring Duplicate Code
```markdown
# Task Decomposer

## Task Brief
**Scope:** Extract common validation logic from 5 different modules
**Success Criteria:** Single validation helper used across all modules
**Blast Radius:** 6-8 files (5 modules + new helper + tests)
**Model Selection:** simple_models (straightforward refactoring)

## Workflow Selection
WF-REFACTOR - DRY principle improvement
```

## Integration with Existing TestSprite Tests

Your existing TestSprite configuration in `testsprite_tests/` can be enhanced with these guardrails:

1. Update test plans to reference the PRD
2. Use the output templates for consistent reporting
3. Apply the workflow selection criteria to test generation

## Troubleshooting

### Common Issues
1. **File too large**: Use chunking strategy from FILE-1 policy
2. **Too many changes**: Limit to 10 files max for refactors
3. **Model mismatch**: Switch between simple/advanced based on complexity
4. **Missing outputs**: Ensure all required files are generated

### Getting Help
- Check the guardrails in `testsprite.prd.yaml`
- Use the templates in `testsprite_templates/`
- Follow the fail-fast rules for guidance
