# Backup File Naming

Put `.bak` at the end, timestamp in the middle:

- **Right**: `config.toml.20260223T112407Z.bak`
- **Wrong**: `config.toml.bak.20260223T112407Z`

## Rationale

- `ls *.bak` and `rm *.bak` work for cleanup
- File managers identify backups by final extension
- Timestamp in the middle provides uniqueness for multiple backups
- Follows the convention used by most Unix backup tools
