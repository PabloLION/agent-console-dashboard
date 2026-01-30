# Sub-Agent Lifecycle in Claude Code

## How Sub-Agents Work

Sub-agents spawned via the Task tool are **one-shot**. They execute, return a
result, and their process ends. They don't persist in the background waiting.

However, their context can be **resumed** using the agent ID returned when they
finish. If the parent agent passes that ID back to a new Task call with the
`resume` parameter, the sub-agent continues with its full previous transcript
preserved.

## Lifecycle

1. Parent spawns a sub-agent → it runs → returns result + agent ID → process
   ends
2. If needed later, parent resumes it by ID → it picks up where it left off
3. When the session ends, all agent IDs become invalid — nothing persists across
   sessions

## Key Points

- Sub-agents are tied to the current session, not running independently
- No background threads or processes waiting around
- Context is preserved for resumption within the session only
- Each invocation without `resume` starts completely fresh
