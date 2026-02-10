# Decision: Naming Rationale

Naming decisions for types, structs, and concepts in the codebase. Each section
documents why a specific name was chosen over alternatives.

## SessionSnapshot

**Decided:** 2026-02-10

### Context

The `Session` struct contains `Instant` fields (not serializable). A separate
struct is needed for the IPC wire format — a serializable, point-in-time view of
a session with computed fields (elapsed seconds, idle seconds). This struct
needed a clear, unambiguous name.

### Decision

`SessionSnapshot` — a point-in-time computed view of a session, sent over IPC.

### Alternatives Considered

| Name            | Meaning                        | Pro                                 | Con                                                                              |
| --------------- | ------------------------------ | ----------------------------------- | -------------------------------------------------------------------------------- |
| SessionSnapshot | State captured at this moment  | Intuitive, common pattern           | Slightly heavy word                                                              |
| SessionInfo     | General session information    | Short, familiar                     | Too generic — "info" says nothing                                                |
| SessionView     | Derived read-only projection   | Like a DB view — exactly what it is | Clashes with TUI `views` module                                                  |
| SessionWire     | The wire format representation | Explicit about purpose              | Conflicts with "wire format" (the encoding); leaks transport concern into domain |
| SessionFrame    | One frame of session state     | Clean, implies time-series          | Unusual in this domain                                                           |
| SessionDigest   | Compact summary                | Short, precise                      | Implies cryptographic hash                                                       |
| SessionRecord   | A record of current state      | Familiar                            | Implies persistence/storage                                                      |

### Rationale

- `SessionSnapshot` clearly communicates that this is a frozen, computed view —
  not the live internal state
- The "snapshot" metaphor is widely understood (database snapshots, filesystem
  snapshots, process snapshots)
- No naming collision with existing project concepts (`views`, `wire format`)
- Paired naturally with `StatusChange` for history entries
