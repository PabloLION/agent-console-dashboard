# Orchestration Rules

How to manage multi-agent dispatch sessions.

## Main Thread Role

The main thread is the **orchestrator**. It does NOT write code (except 1-2 line
fixes). Responsibilities:

- Create and manage git worktrees
- Dispatch agents with clear task descriptions
- Review agent output for spec drift
- Merge branches back to main
- Handle beads sync and git operations
- Context management (avoid drift, keep agents aligned)

## Agent Role

Agents do ALL code work. Each agent:

- Works in its own git worktree (`/tmp/acd-wt-<agent-name>`)
- Makes atomic commits per issue
- Runs quality checks (`cargo test`, `cargo clippy`) before each commit
- Checks its persistent memory before starting
- Updates its persistent memory after completing
- Does NOT push — main thread handles merges

## Named Agents with Persistent Memory

Define agents in `.claude/agents/` with `memory: project`. Benefits:

- Accumulate knowledge across sessions
- Named and identifiable
- Memory auto-loads on each invocation (first 200 lines of MEMORY.md)
- Stored at `.claude/agent-memory/<agent-name>/`

## Sequential Batching

One agent can handle multiple related issues sequentially in a single prompt.
This reuses context and reduces overhead. Group issues by:

- File scope (same files touched)
- Domain (same area of the codebase)
- Dependency (issue B needs context from issue A)

Example: instead of 12 agents for 12 issues, use 12 agents for 26 issues
(~2 issues per agent average).

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
`docs/agents/<agent-name>/` (tracked, merges with branch). Each named agent
gets its own folder. Do NOT put research output in `.git-ignored/` inside a
worktree — ignored files are not merged and get lost when the worktree is
deleted. Main thread reviews research and discusses findings with user.

## Confidence Check

Before dispatching agents, confirm 95% confidence that:

- Issue descriptions are clear and unambiguous
- File scope is identified correctly
- No missing dependencies between issues
- Agent grouping minimizes overlap

## Post-Dispatch

After all agents complete:

1. Merge branches one by one (non-overlapping first)
2. Run full test suite after each merge
3. Final `cargo test && cargo clippy` on main
4. Close beads issues
5. `bd sync` + `git push`
