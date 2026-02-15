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

## Issue Sizing

Each issue should be handleable by one agent. If an issue is too large or spans
multiple domains, break it into smaller issues before dispatching. Each
sub-issue is assigned to one agent. The original issue becomes a parent or is
replaced by the sub-issues.

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

For every issue presented, report a **numerical confidence percentage** (0-100%)
that the issue is actionable as-written. If below 100%, list every doubt that
reduces confidence. This applies to the confidence table and to each issue in
the pre-dispatch protocol.

### Pre-Dispatch Protocol (per issue)

For **every** issue, before dispatching — even when the orchestrator has zero
doubts. The user may have doubts of their own. Never skip showing an issue.

1. Show the full issue: run `bd show <id>` and print doubts **immediately
   after** that single output. Do NOT batch multiple `bd show` calls — present
   one issue at a time, doubts directly following the output, before moving to
   the next issue. If a doubt references another issue, run `bd show` for that
   related issue inline (right where the doubt is stated). If no doubts, state
   "No doubts from orchestrator" explicitly.
2. Wait for user's explicit go-ahead
3. If user wants changes: update the issue first, then re-present
4. When a doubt is cleared or a design decision is made, update the issue
   description or notes (`bd update <id> --description/--notes`) so the agent
   gets the full context at dispatch time
5. Only dispatch after user says the issue is ready

### Lookahead

When showing an issue for review, also show the next issue that needs review in
the same message. For each issue, include: full issue details, all doubts (even
minor ones), and whether it reaches 95% confidence. Raise every doubt you can
identify — if it's negligible, the user will skip it.

### Discussion Priority

The goal is to keep every agent busy. When choosing which issue to discuss next,
apply these rules in order:

1. **Idle agent first**: prefer issues whose corresponding agent has no active
   work — this keeps agents utilized. Cycle through all idle agents before
   revisiting busy ones.
2. **Difficulty ordering** (tiebreaker among idle agents):
   - If fewer than 6 agents are running: discuss hardest issues first (lowest
     confidence), so complex design work happens while agents are available
   - If 6 or more agents are running: discuss easiest issues first (highest
     confidence), so agents get assigned sooner
3. **Pace**: move quickly through idle agents. Show the issue, state doubts, get
   approval, dispatch, immediately move to the next idle agent. Do not linger on
   one agent while others sit idle.

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

## Worktree Lifecycle

### Merge immediately, independently

When an agent completes, merge its worktree branch to main immediately. Do NOT
wait for other agents to finish — each agent's work is independent. Waiting for
unrelated agents wastes time and creates false dependencies.

### Merge and cleanup

After an agent completes an issue:

1. Agent commits in its worktree branch
2. Orchestrator merges worktree branch → main (`git merge --no-ff` for
   traceability)
3. Orchestrator runs test suite on main
4. Orchestrator removes worktree and branch (`git worktree remove`,
   `git branch -d`)
5. Close the beads issue with
   `bd close <id> --reason="<summary>. Commit: <hash>"` — the commit hash is the
   merge commit (or direct commit for non-worktree work). Never close an issue
   before its work is committed and merged.

### Fresh worktree per dispatch

Do NOT reuse worktrees. Each new dispatch gets a fresh worktree from current
main. This avoids sync problems and stale state. Creating a new worktree is
cheap; debugging a stale one is not.

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
