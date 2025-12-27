# AI Agent Instructions

This document contains instructions and guidelines for AI agents working on this project.

## Issue Tracking with bd (beads) via MCP

**IMPORTANT**: This project uses **beads MCP server** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Why beads MCP?

- Dependency-aware: Track blockers and relationships between issues
- Git-friendly: Auto-syncs to JSONL for version control
- Agent-optimized: Native MCP integration, no CLI parsing needed
- Prevents duplicate tracking systems and confusion

### MCP Tools (Primary Method)

**Always use MCP tools when available:**

| Action | MCP Tool |
|--------|----------|
| List ready work | `mcp__beads__list_issues` with `ready_only=true` |
| Create epic | `mcp__beads__create_issue` with `issue_type="epic"` |
| Create subtask | `mcp__beads__create_issue` with `parent="<epic-id>"` |
| Update status | `mcp__beads__update_issue` with `status="in_progress"` |
| Close issue | `mcp__beads__close_issue` |
| Show issue | `mcp__beads__get_issue` |
| Epic status | `mcp__beads__epic_status` |

### CLI Fallback

If MCP is unavailable, use CLI commands:

**Check for ready work:**
```bash
bd ready --json
```

**Create new issues:**
```bash
bd create "Issue title" -t bug|feature|task -p 0-4 --json
bd create "Subtask" --parent <epic-id> --json
```

**Claim and update:**
```bash
bd update bd-42 --status in_progress --json
```

**Complete work:**
```bash
bd close bd-42 --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `mcp__beads__list_issues(ready_only=true)`
2. **Claim your task**: `mcp__beads__update_issue(id, status="in_progress")`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue with `mcp__beads__create_issue`
5. **Complete**: `mcp__beads__close_issue(id)`
6. **Commit together**: Always commit the `.beads/issues.jsonl` file together with the code changes so issue state stays in sync with code state

### Auto-Sync

bd automatically syncs with git:
- Exports to `.beads/issues.jsonl` after changes (5s debounce)
- Imports from JSONL when newer (e.g., after `git pull`)
- No manual export/import needed!

### MCP Server Setup

The beads MCP server is installed and configured for this project.

### Managing AI-Generated Planning Documents

AI assistants often create planning and design documents during development:
- PLAN.md, IMPLEMENTATION.md, ARCHITECTURE.md
- DESIGN.md, CODEBASE_SUMMARY.md, INTEGRATION_PLAN.md
- TESTING_GUIDE.md, TECHNICAL_DESIGN.md, and similar files

**Best Practice: Use a dedicated directory for these ephemeral files**

**Recommended approach:**
- Create a `history/` directory in the project root
- Store ALL AI-generated planning/design docs in `history/`
- Keep the repository root clean and focused on permanent project files
- Only access `history/` when explicitly asked to review past planning

**Example .gitignore entry (optional):**
```
# AI planning documents (ephemeral)
history/
```

**Benefits:**
- ✅ Clean repository root
- ✅ Clear separation between ephemeral and permanent documentation
- ✅ Easy to exclude from version control if desired
- ✅ Preserves planning history for archeological research
- ✅ Reduces noise when browsing the project

### CLI Help

Run `bd <command> --help` to see all available flags for any command.
For example: `bd create --help` shows `--parent`, `--deps`, `--assignee`, etc.

### Important Rules

- ✅ Use bd for ALL task tracking
- ✅ Always use `--json` flag for programmatic use
- ✅ Link discovered work with `discovered-from` dependencies
- ✅ Check `bd ready` before asking "what should I work on?"
- ✅ Store AI planning docs in `history/` directory
- ✅ Run `bd <cmd> --help` to discover available flags
- ❌ Do NOT create markdown TODO lists
- ❌ Do NOT use external issue trackers
- ❌ Do NOT duplicate tracking systems
- ❌ Do NOT clutter repo root with planning documents

For more details, see README.md and QUICKSTART.md.

# Project Constitution
## Core Principles

### I. Conventional Commit Format

All commits MUST follow the Conventional Commits specification:
- Format: `<type>(<scope>): <subject> [<ISSUE-KEY>]`
- Supported types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `chore`, `revert`
- Breaking changes: Use `!` after type/scope (e.g., `feat(api)!: ...`) AND include `BREAKING CHANGE:` in footer
- Example: `feat(graph): add second-degree relationships [MHG-123]`

**Rationale**: Standardized commit messages enable automated changelog generation, semantic versioning, and clear project history. The format enforces clarity about what changed and why.

### II. Mandatory Issue Tracking

Every commit MUST reference exactly one task/issue:
- In subject: `feat(api): add endpoint [PROJ-123]`
- OR in footer: `Refs: PROJ-123`
- When closing: `Closes: PROJ-123` (Jira/Linear) or `Fixes #123` (GitHub)
- For maintenance work without issues: Use `Refs: OPS-000` (technical debt/housekeeping)

**Rationale**: Ensures full traceability from code changes back to requirements. Every line of code must have a documented business justification. Prevents "mystery commits" that become archaeological puzzles months later.

### III. Branch Naming Convention

All feature branches MUST follow the structured naming pattern:
- Format: `<type>/<ISSUE-KEY>-<slug>`
- Example: `feat/PROJ-123-import-pipeline`
- Types align with commit types: `feat`, `fix`, `refactor`, `chore`, etc.

**Rationale**: Enables automated workflows, improves repository organization, and makes branch purpose immediately clear. The issue key ensures traceability before a single commit is made.

### IV. Pull Request Traceability

Pull requests MUST link to the story level (not just tasks):
- In PR description: `Relates-to Story: PROJ-100`
- Enables traceability chain: User Story → Task → PR → Commits
- Issue tracker auto-linking will create bidirectional traceability

**Rationale**: Tasks are implementation details; stories represent user value. Linking PRs to stories ensures code review focuses on business goals, not just technical correctness.

### V. Mono-repo Scope Organization

For mono-repository structures, scopes MUST reflect component boundaries:
- Examples: `feat(airbyte)`, `fix(dbt)`, `refactor(dagster)`, `chore(infra/helm)`
- Scopes must be registered in project documentation (this constitution or dedicated scope registry)
- Nested scopes allowed with `/` separator for infrastructure or shared concerns

**Rationale**: In mono-repos, scopes are critical for understanding blast radius, triggering correct CI/CD pipelines, and generating component-specific changelogs. Without enforced scopes, mono-repo history becomes unusable.

### VI. Single Responsibility Principle

Commits and branches MUST maintain single responsibility:
- One commit addresses one task only
- If fixing multiple unrelated issues, create separate commits and PRs
- Documentation changes for a feature stay with that feature's commits
- Cross-cutting refactors may touch multiple files but serve a single purpose

**Rationale**: Simplifies code review, enables clean reverts, improves bisect accuracy, and makes cherry-picking safe. Violating this principle creates technical debt that compounds over time.

## Development Workflow

### Epic-Based Iteration Model

**Core principles**:
- One task = one commit
- One epic = one PR
- Preserve commit history (no squash)

1. **Feature = Epic (2-Level Architecture)**
   - Every feature request becomes an epic in beads
   - Epic contains all subtasks needed to complete the feature
   - Use `bd create "Feature name" -t epic` then add subtasks with `--parent`
   - **No orphan tasks** — all tasks MUST belong to an epic
   - Immediately create feature branch after epic creation
   - No epic exists without a branch — they are created together

2. **Task Requirements**
   - Every task MUST have a description explaining what needs to be done
   - Every task MUST have Definition of Done (DoD) criteria:
     - All tests green
     - Linters clean
     - Migrations applicable/reversible (if applicable)
     - Documentation/CHANGELOG updated
     - Feature flag added (if applicable)
   - Use: `bd create "Task" --parent <epic> --description="What and why" `

3. **One Task = One Commit**
   - Each subtask gets its own commit
   - Commit message references the task: `feat(scope): description [task-id]`
   - Do NOT squash commits — preserve granular history
   - Each commit should be atomic and self-contained
   - Epic/task creation is a separate commit: `chore(beads): add epic for <feature>`

4. **Complete Epic Per Iteration**
   - An iteration is complete only when the entire epic is closed
   - All subtasks must be done before closing the epic
   - No partial epic delivery — ship complete features

5. **Feature Branch Per Epic**
   - Create branch: `feat/<EPIC-ID>-<slug>` (e.g., `feat/test-cc-pm-8sz-calculator`)
   - All work for the epic happens in this branch
   - One epic = one branch = atomic feature delivery

6. **Draft PR at Branch Creation**
   - Create draft PR immediately after creating feature branch
   - **All PRs target `dev` branch** — never create PRs directly to master/main
   - This enables continuous review as commits are pushed
   - Draft PR description includes epic link and high-level plan
   - Reviewers can comment on implementation as it progresses
   - Convert to ready-for-review when epic is complete

7. **No Direct Commits to Protected Branches**
   - NEVER commit directly to master/main or dev
   - All changes go through feature branches → PR to dev
   - dev is merged to master via release process

8. **PR for Review**
   - When epic is complete, convert draft PR to ready-for-review
   - PR links to epic: `Relates-to Epic: <EPIC-ID>`
   - One PR = one epic (multiple commits preserved)
   - Use merge commit, NOT squash — keep commit history
   - AI agent performs self-review before requesting merge
   - **AI agent MUST NOT merge PRs** — only human can approve and merge
   - After human approval, human merges PR and deletes feature branch

### AI Self-Review Process

Before requesting PR merge, the AI agent MUST perform a self-review:

1. **Review Criteria (Weighted)**
   - **Requirements Compliance (60%)**: Does the implementation match the task requirements and DoD?
   - **Tests & Quality (20%)**: Are tests green? Is code quality acceptable?
   - **Style & Architecture (15%)**: Is code consistent with codebase style and architecture?
   - **DX & Documentation (5%)**: Is documentation updated? Is the change easy to understand?

2. **Scoring System (0-100)**
   - **90-100**: Excellent — fully aligned, clean implementation, ready to merge
   - **70-89**: Good — minor issues, acceptable with notes
   - **50-69**: Needs Work — significant gaps, should be addressed before merge
   - **0-49**: Reject — does not meet requirements, rewrite needed

3. **Review Output Format**
   ```
   ## PR Self-Review: [PR Title]

   ### Assessment
   | Criteria | Weight | Score | Weighted |
   |----------|--------|-------|----------|
   | Requirements Compliance | 60% | X/100 | X.X |
   | Tests & Quality | 20% | X/100 | X.X |
   | Style & Architecture | 15% | X/100 | X.X |
   | DX & Documentation | 5% | X/100 | X.X |
   | **TOTAL** | **100%** | | **XX.X/100** |

   ### Details
   - Requirements: [explanation]
   - Tests: [explanation]
   - Style: [explanation]
   - DX: [explanation]

   ### Verdict
   [APPROVE/NEEDS_WORK/REJECT] — [summary]
   ```

4. **Workflow**
   - Create PR
   - Perform self-review
   - If score >= 70: Add review as PR comment, request merge
   - If score < 70: Fix issues, amend commit, re-review

**Workflow summary**:
```
bd create "New feature" -t epic           # Create epic
git checkout -b feat/<epic-id>-slug       # Create feature branch IMMEDIATELY
git add .beads/ && git commit             # Commit epic creation
gh pr create --draft --base dev           # Create DRAFT PR targeting dev
# ... work on all subtasks (one commit per task) ...
# Reviewers can comment on commits as they are pushed
bd close <epic-id>                        # Close epic when done
gh pr ready                               # Convert draft to ready-for-review
# Perform self-review, add score as PR comment
# If score >= 70: merge PR (no squash!)
# Delete feature branch
```

### Commit Creation Process

1. **Before committing**:
   - Verify you have an assigned issue/task with clear acceptance criteria
   - Ensure issue key is in your branch name
   - Stage only files relevant to the single task

2. **Commit message construction**:
   - Choose correct type: `feat` (new capability), `fix` (bug), `refactor` (no behavior change), `docs` (documentation only), etc.
   - Choose correct scope: component/module affected (see Principle V)
   - Write clear subject: imperative mood, present tense, no period, <72 chars
   - Add issue reference: `[PROJ-123]` in subject or `Refs: PROJ-123` in footer
   - For breaking changes: Add `!` and `BREAKING CHANGE:` footer with migration notes
   - **NO AI signatures** — do NOT add "Generated with Claude Code", "Co-Authored-By: Claude", or similar footers

3. **Edge cases**:
   - **Multiple tasks**: Split into separate commits, each with its own issue reference
   - **No issue**: For housekeeping, use `chore(repo): description Refs: OPS-000`
   - **Documentation for feature**: Include in feature commit, use same issue reference
   - **Emergency hotfix**: Still requires issue (create urgent issue first)

### Pull Request Process

1. **Before creating PR**:
   - Ensure all commits follow Conventional Commits format
   - Verify each commit references an issue
   - Identify the parent User Story for all tasks in this PR

2. **PR description must include**:
   - `Relates-to Story: PROJ-XXX` (the user story, not individual tasks)
   - Summary of changes (auto-generated from commits acceptable if clear)
   - Test plan or verification steps
   - Screenshots/demos for UI changes

3. **PR review checklist**:
   - All commits follow constitution principles
   - Issue references are valid and accessible
   - Story link is correct
   - Scope usage is consistent
   - Breaking changes are clearly documented

## Governance

### Authority

This constitution supersedes all informal practices and tribal knowledge. When documentation conflicts with this constitution, the constitution prevails until formally amended.

### Amendment Process

1. Propose amendment via issue with `constitution-amendment` label
2. Document rationale and impact analysis
3. Requires approval from technical lead or team consensus (as per project governance)
4. Amendment merged updates `LAST_AMENDED_DATE` and increments `CONSTITUTION_VERSION`
5. Migration plan required for breaking changes (e.g., new mandatory scopes)

### Versioning Policy

Constitution follows semantic versioning:
- **MAJOR**: Backward-incompatible governance changes (e.g., removing an allowed commit type)
- **MINOR**: New principle added or existing principle materially expanded
- **PATCH**: Clarifications, wording improvements, typo fixes

### Compliance

- All PRs MUST be reviewed for constitutional compliance before merge
- Automated tools (commit linters, PR checks) SHOULD enforce machine-checkable rules
- Constitution violations MAY be fixed via amendment if the violation reveals a better practice
- Complexity that violates principles MUST be justified in plan.md Complexity Tracking section

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
