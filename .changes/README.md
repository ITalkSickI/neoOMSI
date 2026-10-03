# Changelog fragments

User-visible changes are recorded as small per-PR fragments instead of editing
`CHANGELOG.md` directly in every pull request.

## File name

```text
<pr-number>.<category>.md
```

Example:

```text
412.parity.md
```

## Categories

- `parity` – OMSI 2 compatibility correction
- `fix` – bug or regression fix
- `feature` – intentional neoOMSI feature
- `performance` – meaningful performance improvement
- `breaking` – intentional compatibility/API/configuration break
- `internal` – relevant internal/tooling change worth mentioning

## Contents

Write one short user-facing statement describing the result, not the implementation process.

Good:

```markdown
Fixed keyboard steering return behaviour to match OMSI 2.
```

Avoid:

```markdown
Refactored three functions and changed a HashMap to a Vec.
```

unless that internal change is itself important to release consumers.

## When no fragment is needed

Purely internal changes that should not appear in release notes may use an explicitly
reviewer-approved `skip-changelog` label.

Nightly builds read unreleased fragments without deleting them. Stable release preparation
compiles the accumulated fragments into `CHANGELOG.md` and removes the processed fragment
files.
