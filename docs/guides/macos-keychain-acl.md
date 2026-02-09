# macOS Keychain ACL and Password Prompts

This document explains why accessing another application's Keychain items
triggers password prompts, and how to avoid them.

## The Core Architecture

macOS Keychain has a **two-layer authorization system**:

| Layer                         | What it controls                                     |
| ----------------------------- | ---------------------------------------------------- |
| **Trusted Applications List** | Which specific binaries can access without prompting |
| **Partition ID**              | Code signature/teamID matching requirement           |

## How ACL Works

When an application creates a Keychain item:

1. The creating application is automatically added to the item's ACL
2. The application can optionally add other trusted applications
3. Only applications in the ACL can access the item without prompting

### ACL Authorization Levels

Three main ACL authorizations govern access:

- **ACLAuthorizationExportClear**: Enables retrieval of unencrypted secrets
- **ACLAuthorizationExportWrapped**: Allows encrypted export with alternative
  password
- **ACLAuthorizationAny**: Grants blanket permissions

### Trusted Application Lists

Each ACL accompanies a trusted applications roster that can be:

- **Nil** (universal trust - "no authorization required, everyone is trusted")
- **Empty** (complete restriction - "nobody is trusted")
- **Specific apps** (selective trust to particular binaries)

## Why Password Prompts Happen

### Scenario: Accessing Another App's Keychain Item

When our Rust binary tries to read Claude Code's credentials:

```text
┌─────────────────────────────────────────────────────────────────┐
│                     Our Rust Binary                             │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  security-framework crate                                │   │
│  │  └── Calls SecItemCopyMatching() directly               │   │
│  │      └── macOS sees OUR BINARY as the requester         │   │
│  │          └── OUR BINARY is NOT in ACL                   │   │
│  │              └── PASSWORD PROMPT                        │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Why `/usr/bin/security` CLI Doesn't Prompt

```text
┌─────────────────────────────────────────────────────────────────┐
│                     Our Rust Binary                             │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  std::process::Command                                   │   │
│  │  └── Spawns /usr/bin/security                           │   │
│  │      └── macOS sees /usr/bin/security as requester      │   │
│  │          └── /usr/bin/security IS IN ACL                │   │
│  │              └── NO PROMPT                              │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Claude Code's Keychain Item ACL

When we inspect Claude Code's credential ACL:

```text
entry 1:
    authorizations: decrypt, export_clear, ...
    applications (1):
        0: /usr/bin/security (OK)  ← Explicitly trusted
```

Only `/usr/bin/security` is in the trusted list, not "any app using
Security.framework".

## The Purpose of security-framework Crate

The `security-framework` Rust crate (and Apple's Security.framework) is designed
for applications to manage **their own** keychain items:

| Use Case                     | ACL Status               | Result          |
| ---------------------------- | ------------------------ | --------------- |
| App creates keychain item    | App automatically in ACL | No prompt       |
| App reads its own item       | App is in ACL            | No prompt       |
| App reads another app's item | App NOT in ACL           | Password prompt |

**It's not a flaw** - the crate is working as designed. The API requires
authorization for items you didn't create.

## Solution for Third-Party Access

To access another application's Keychain items without prompting:

1. **Use an already-authorized binary**: Shell out to `/usr/bin/security` if
   it's in the item's ACL
2. **Request user grants "Always Allow"**: The user can add your binary to the
   ACL via the password dialog
3. **Use alternative storage**: If possible, use file-based credentials or
   environment variables

### Example: Using security CLI in Rust

```rust
use std::process::Command;

fn get_keychain_password(service: &str, account: &str) -> Result<String, Error> {
    let output = Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-s", service,
            "-a", account,
            "-w", // Print password only
        ])
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?.trim().to_string())
    } else {
        Err(Error::NotFound)
    }
}
```

## Partition IDs and Code Signing

The Keychain also uses **PartitionID** for additional security:

- When created through Keychain Access.app: PartitionID = `"apple"`
- When created by third-party apps: PartitionID = `"teamid:[teamID]"`

An app's code signature must align with the PartitionID to bypass prompts. This
is why Apple-signed tools like `/usr/bin/security` can access items with
`"apple"` partition.

## References

- [Apple Developer Forums - Keychain Access Control](https://developer.apple.com/forums/thread/116579)
- [HackTricks - macOS Keychain](https://book.hacktricks.wiki/macos-hardening/macos-red-teaming/macos-keychain.html)
- [SS64 - security command](https://ss64.com/mac/security.html)
- [GitHub Notes - macOS Keychain](https://github.com/eoinkelly/notes/blob/main/security/macos-keychain.md)
