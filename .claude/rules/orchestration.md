# Orchestration Rules

How to manage multi-agent dispatch sessions. The main thread is the
**orchestrator** (maestro) — it conducts the symphony, never plays an
instrument.

## Main Thread Role

The main thread is the **orchestrator**. It does NOT write code (except 1-2 line
fixes). Responsibilities:

- Assess task readiness per issue
- Present issues to user before dispatching
- Discuss doubts with user — dispatch only after explicit user approval
- Create and manage git worktrees
- Dispatch agents with clear task descriptions
- Merge worktree branches back to main after each issue
- Sync main → worktree before assigning next issue
- Review agent output for spec drift
- Handle beads sync and git operations
- Context management (avoid drift, keep agents aligned)

## Agent Role

Agents do ALL code work. Each agent:

- Works in its own git worktree (`/tmp/acd-wt-<agent-name>`)
- Makes atomic commits per issue
- Runs quality checks (`cargo test`, `cargo clippy`) before each commit
- Checks its persistent memory before starting
- Updates its persistent memory after completing
- Does NOT push — orchestrator handles merges

## Named Agents as Domain Experts

Define agents in `.claude/agents/` with `memory: project`. Agents are
**permanent domain experts**, not single-use task workers. They can be consulted
on any issue in their field across multiple sessions.

Persistence:

- **Definition** (`.claude/agents/<name>.md`): git-tracked, permanent
- **Memory** (`.claude/agent-memory/<name>/MEMORY.md`): persists across
  sessions, first 200 lines auto-loaded on each invocation
- **Conversation**: starts fresh each invocation, but MEMORY.md provides
  continuity

## Sequential Batching

One agent handles multiple issues sequentially. Group issues by:

- File scope (same files touched)
- Domain (same area of the codebase)
- Dependency (issue B needs context from issue A)

## Batch Dispatch Workflow

Dispatch happens in batches, not all at once. The orchestrator never dispatches
without explicit user approval.

### Readiness

An issue is **ready** when the orchestrator has 95% confidence that the spec is
clear and unambiguous. Otherwise, it is **not ready**.

- Ready issues: proceed to pre-dispatch protocol
- Not-ready issues: label as `needs-design`, discuss with user

### Pre-Dispatch Protocol (per issue)

For each ready issue, before dispatching:

1. Show the full issue (`bd show <id>`) including description, dependencies,
   etc.
2. State any remaining doubts, even minor ones
3. Wait for user's explicit go-ahead
4. If user wants changes: update the issue first, then re-present
5. Only dispatch after user says the issue is ready

### Discussion Priority

When choosing which issue to discuss next, apply these rules in order:

1. **Idle agent first**: prefer issues whose corresponding agent has no active
   work — this keeps agents utilized
2. **Difficulty ordering** (tiebreaker among idle agents):
   - If fewer than 6 agents are running: discuss hardest issues first (lowest
     confidence), so complex design work happens while agents are available
   - If 6 or more agents are running: discuss easiest issues first (highest
     confidence), so agents get assigned sooner

### Dispatch Cycle

1. Present confidence table for all pending issues (ready / not ready)
2. Follow pre-dispatch protocol for ready issues (show, doubts, user approval)
3. Launch approved issues to **background** agents (worktree + dispatch)
4. Continue discussing not-ready issues in **foreground**
5. When more issues become ready, repeat from step 2
6. Loop until all dispatched or deferred

### Labels for Readiness

- `needs-design`: issue spec is incomplete, needs user discussion
- Remove label after design is settled and spec is updated

## Worktree Sync Lifecycle

After an agent completes an issue:

1. Agent commits in its worktree branch
2. Orchestrator merges worktree branch → main
3. Orchestrator runs test suite on main
4. Orchestrator syncs main → worktree (`git merge main` in worktree)
5. Only then: dispatch next issue to that agent

This prevents agents from drifting when other agents' changes land on main.

## Overlap Management

If two agents touch the same file:

- Use git worktrees for isolation
- Merge non-overlapping agents first
- Run full test suite after each merge
- Overlapping agents merge last with conflict resolution

## Dispatch Capacity

Safe to run up to 12 parallel agents. More may work but 12 is tested and
efficient.

## Quality Assurance

Each agent must:

1. Check persistent memory for relevant patterns
2. Work in isolated worktree
3. Make atomic commits (one logical change per commit)
4. Run tests and linting before committing
5. Update persistent memory with new discoveries

## Research vs Code Agents

Some agents produce research reports, not code. Their output goes to
`docs/agents/<agent-name>/` (tracked, merges with branch). Each named agent gets
its own folder. Do NOT put research output in `.git-ignored/` inside a worktree
— ignored files are not merged and get lost when the worktree is deleted.
Orchestrator reviews research output and discusses with user.

## Post-Dispatch

After all agents complete:

1. Merge branches one by one (non-overlapping first)
2. Run full test suite after each merge
3. Final `cargo test && cargo clippy` on main
4. Close beads issues
5. `bd sync` + `git push`
