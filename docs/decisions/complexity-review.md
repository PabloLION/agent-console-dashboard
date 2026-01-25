# Complexity Review

**Date**: 2026-01-25 **Status**: Pending Review

## Summary

Analysis of whether the project is over-engineered for its purpose (CLI daemon
tool).

## Metrics

| File                               | Lines | Comments  | Tests |
| ---------------------------------- | ----- | --------- | ----- |
| src/daemon/store.rs                | 2,303 | 618 (27%) | 8     |
| tests/socket_server_integration.rs | 1,501 | -         | 20    |
| src/lib.rs                         | 882   | 118       | 46    |
| src/client/connection.rs           | 713   | -         | -     |
| src/daemon/server.rs               | 564   | 167       | 2     |
| **Total**                          | 6,438 | -         | 171   |

## Potential Issues

### Unused Types

Review whether these are needed:

- `SessionMetadata` - builder pattern defined but never used in protocol
- `ApiUsage` - defined but not used
- `StateTransition` - history tracking not exposed in current protocol
- `history_depth_limit` - field exists but feature not implemented

### Duplicate Methods

- `remove()` vs `remove_session()` - identical functionality
- `get_or_create_session()` - could be simplified

### Premature Abstractions

- Full serde serialization when protocol is simple text (SET/GET/LIST/RM)
- `StoreError` enum when `Option<Session>` might suffice
- Broadcast channels for <5 subscribers

## What's Justified

- `Arc<RwLock>` for concurrent access
- Signal handling (SIGTERM/SIGINT)
- Socket cleanup logic
- Basic error types for client module

## Recommendations

1. Review epic files to understand which types are actually needed
2. Consider removing unused types after epic review
3. Consolidate duplicate methods
4. Keep documentation (comments are good)
5. Target: large files should be split, not stripped of docs

## Next Steps

- [ ] Review all epic/story files
- [ ] Identify which types are planned vs speculative
- [ ] Decide: remove unused or implement planned features
- [ ] Split large files if needed (store.rs > 2000 lines)
