# Markdown Linting Configuration Justification

This document explains the configuration choices made for markdown linting and
formatting in this project.

## Overview

Two configuration files control markdown quality in this project:

- `.markdownlint.json` - Rules for markdown linting (structure, formatting)
- `.prettierrc.json` - Automatic formatting settings

## Configuration Details

### 1. No Trailing Spaces (`no-trailing-spaces`)

**File:** `.markdownlint.json`

```jsonc
// Override: Disallow ALL trailing spaces (default br_spaces is 2)
"no-trailing-spaces": {
  "br_spaces": 0
}
```

**Decision:** Disallow ALL trailing spaces, including the two-space sequence
traditionally used for `<br>` line breaks in markdown.

**Rationale:**

- Trailing whitespace is invisible and causes diff noise in version control
- Modern markdown renderers support `<br>` tags as an explicit alternative
- Most editors have "trim trailing whitespace" enabled by default
- Prevents accidental whitespace from being committed

**Alternative for Line Breaks:** Use the explicit `<br>` HTML tag instead of two
trailing spaces when a hard line break is needed.

### 2. No Duplicate Heading (Global Enforcement)

**File:** `.markdownlint.json`

**Decision:** Enforce globally unique headings across the entire document (uses
markdownlint default).

**Note:** With `"default": true`, the `no-duplicate-heading` rule is enabled
with global enforcement by default (`siblings_only: false`). No explicit
configuration is needed.

**Rationale:**

- Ensures every heading can be uniquely referenced via markdown links
- Improves document navigation and linking
- Makes table of contents more useful
- Avoids confusion when searching for specific sections

**Example:** Instead of having multiple "Installation" headings, use specific
names like "Frontend Installation" and "Backend Installation".

### 3. No Multiple Blank Lines (Rule Removed)

**File:** `.markdownlint.json`

**Decision:** The `no-multiple-blanks` rule is omitted entirely.

**Effect:** Uses markdownlint's default behavior, which allows at most 1 blank
line between elements.

**Rationale:**

- Default behavior (maximum: 1) provides reasonable spacing
- Explicit rule configuration was unnecessary
- Keeps configuration minimal and focused

### 4. Line Length: 80 Characters

**Files:** `.markdownlint.json` and `.prettierrc.json`

```jsonc
// .markdownlint.json - Override exceptions (80 is default but we exclude certain elements)
"line-length": {
  "line_length": 80,
  "code_blocks": false,
  "tables": false,
  "headings": false
}

// .prettierrc.json
{
  "printWidth": 80,
  "proseWrap": "always"
}
```

**Decision:** Standardize on 80-character line length for prose content.

**Rationale:**

- **Universal Standard:** 80 columns is a widely recognized standard dating back
  to terminal limitations
- **Terminal Compatibility:** Works well in default terminal windows
- **Side-by-Side Viewing:** Allows comfortable diff viewing and split-pane
  editing
- **Readability:** Optimal line length for reading prose (typically 60-80
  characters)
- **Consistency:** Matches many projects' conventions

**Exceptions:** The following are exempt from the 80-character limit:

- **Code Blocks:** Technical content should not be arbitrarily wrapped
- **Tables:** Table formatting should be preserved for readability
- **Headings:** Long headings are occasionally necessary for clarity

## Formatting Workflow

### Automatic Formatting

Prettier handles automatic formatting with these settings:

```json
{
  "printWidth": 80,
  "proseWrap": "always"
}
```

The `proseWrap: "always"` setting ensures Prettier wraps markdown prose at the
configured line width.

### Command Usage

Use globally installed tools when available, with npx as fallback:

```bash
# Preferred: Use global installation
markdownlint 'docs/**/*.md'
prettier --write 'docs/**/*.md'

# Fallback: Use npx if global tools not installed
npx markdownlint 'docs/**/*.md'
npx prettier --write 'docs/**/*.md'
```

### Lint the docs/ Folder

Run linting checks on documentation:

```bash
markdownlint 'docs/**/*.md'
# or with npx fallback:
npx markdownlint 'docs/**/*.md'
```

### Format the docs/ Folder

Apply formatting to documentation files:

```bash
prettier --write 'docs/**/*.md'
# or with npx fallback:
npx prettier --write 'docs/**/*.md'
```

### Pre-commit Hook

Run prettier and markdownlint **separately** (not combined with `&&`) so that if
one fails, the other still runs:

```bash
#!/bin/sh
# Run prettier first (fixes formatting)
prettier --write 'docs/**/*.md' || npx prettier --write 'docs/**/*.md'

# Run markdownlint separately (checks rules)
markdownlint 'docs/**/*.md' || npx markdownlint 'docs/**/*.md'
```

This ensures both tools run even if one encounters issues.

### One-Time Fix for Existing Files

To fix all existing markdown files in `docs/`, run the one-time fix script:

```bash
./scripts/fix-markdown-files.sh
```

This script formats and lints all markdown files in `docs/`. Run it once when
setting up the project.

### Installing the Pre-commit Hook

A ready-to-use pre-commit hook is available at `scripts/pre-commit-markdown.sh`.

To install it:

```bash
# Copy the hook to your git hooks directory
cp scripts/pre-commit-markdown.sh .git/hooks/pre-commit

# Make sure it's executable
chmod +x .git/hooks/pre-commit
```

The hook automatically:

1. Checks only **staged markdown files** (not all files)
2. Runs prettier and markdownlint on changed files only
3. Uses global tools if available, falls back to npx
4. Exits early if no markdown files are staged

## Summary of Changes

| Rule                 | Previous Value          | New Value                   |
| -------------------- | ----------------------- | --------------------------- |
| `br_spaces`          | 2 (allow trailing)      | 0 (no trailing spaces)      |
| `no-duplicate-heading` | `siblings_only: true` | `true` (global enforcement) |
| `no-multiple-blanks` | `maximum: 2`            | (removed, uses default: 1)  |
| `line-length`        | 100 characters          | 80 characters               |

## Questions and Answers

**Q: What if I need a line break in the middle of a paragraph?**
A: Use the `<br>` HTML tag explicitly instead of two trailing spaces.

**Q: Why not 120 characters for line length?**
A: While 120 is also popular, 80 was chosen for better terminal compatibility
and side-by-side viewing. Most developers' terminals default to 80 columns.

**Q: Can I have duplicate headings in different files?**
A: Yes, this rule only applies within a single file. Different files can have
headings with the same text.

---

_Document created as part of markdown linting configuration updates._
