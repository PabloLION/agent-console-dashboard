# Named Agent Definition Format

Research date: 2026-02-12

## File Location

- Project agents: `.claude/agents/<name>.md` (git-tracked, shared)
- User agents: `~/.claude/agents/<name>.md` (private, all projects)

## YAML Frontmatter Schema

```yaml
---
name: agent-name # required, lowercase + hyphens
description: when to use # required, triggers auto-delegation
tools: Read, Edit, Bash # optional, inherits all if omitted
disallowedTools: Write # optional, removed from inherited set
model: sonnet # optional: sonnet, opus, haiku, inherit
permissionMode: default # optional: default, acceptEdits, plan, etc.
maxTurns: 20 # optional, max API round-trips
skills: # optional, injected into context
  - skill-name
mcpServers: # optional, MCP servers available
  server-name: {}
hooks: # optional, scoped to this agent
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "./validate.sh"
memory: project # optional: user, project, local
---
Agent system prompt in markdown here.
```

## Memory Persistence

| Scope   | Storage Location                     | Auto-loaded               |
| ------- | ------------------------------------ | ------------------------- |
| user    | `~/.claude/agent-memory/<name>/`     | MEMORY.md first 200 lines |
| project | `.claude/agent-memory/<name>/`       | MEMORY.md first 200 lines |
| local   | `.claude/agent-memory-local/<name>/` | MEMORY.md first 200 lines |

## Invocation

- CLI: `claude --agent <name>`
- Within conversation: Claude auto-delegates based on `description` field
- Agent spawning subagents: `tools: Task(reviewer, debugger)` in frontmatter

## Applied

12 domain-expert agents created in `.claude/agents/` with `memory: project`. See
individual files for each agent's scope and conventions.
