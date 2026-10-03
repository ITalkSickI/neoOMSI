# Releasing neoOMSI

neoOMSI maintains a clear distinction between rapid development snapshots and stabilized community releases.

## Versioning scheme

neoOMSI follows Semantic Versioning (`MAJOR.MINOR.PATCH`) during pre-1.0 development:

| Type | Format | Example |
| --- | --- | --- |
| **Nightly** | `0.x.y-nightly.<run-id>` | `0.4.0-nightly.312` |
| **Release Candidate** | `v0.x.y-rc.<n>` | `v0.4.0-rc.1` |
| **Stable** | `v0.x.y` | `v0.4.0` |
| **Patch** | `v0.x.y+1` | `v0.4.1` |

`1.0.0` is reserved for achieving comprehensive behavioral parity across the OMSI 2.2.032 baseline, not simply for elapsed development time.

## Tags

Git tags are created strictly for official milestone releases:

```text
v0.4.0-rc.1
v0.4.0-rc.2
v0.4.0
v0.4.1
```

**Nightly builds are not tagged.** They represent reproducible build artifacts associated directly with commits on `main` and GitHub Actions run IDs.

## Release workflow

```text
main branch (trunk)
    │
    ├── Nightly builds (automated on merge)
    │
    └── Create release branch: release/0.4
            │
            ├── Tag: v0.4.0-rc.1 (testing)
            ├── Tag: v0.4.0-rc.2 (blocker fixes)
            │
            └── Tag: v0.4.0      (final release commit)
```

### 1. Nightly builds

Every merge to `main` triggers automated CI builds for Windows, macOS, Linux, and Android. Nightlies provide immediate visibility into recent changes but carry no guarantee against regressions.

### 2. Preparing a Stable release

When `main` reaches a stabilization milestone:

1. Create a dedicated branch: `release/x.y`.
2. Enter **feature freeze**: only critical bug fixes, documentation corrections, and packaging fixes may be committed to this branch.
3. Keep `main` open for ongoing feature and parity development. Ensure all fixes on the release branch are cherry-picked back into `main`.

### 3. Release Candidates (RCs)

1. Tag the first candidate from the release branch (e.g. `v0.4.0-rc.1`).
2. Conduct testing across supported operating systems, maps, buses, and hardware configurations.
3. If release blockers are discovered, apply the fix to the release branch and tag `v0.4.0-rc.2`.

### 4. Tagging the Stable release

Once an RC exhibits no known release blockers:
- Tag the **exact commit** of the final accepted RC as the Stable release (e.g. `v0.4.0`).
- Avoid pushing last-minute, unvalidated changes between the final RC and the release tag.

### 5. Patch releases

If critical issues are identified post-release:
- Fixes are applied directly to the corresponding `release/x.y` branch.
- Tag and publish a patch release (e.g. `v0.4.1`).
- Mirror the fix into `main`.

## Changelog management

To prevent merge conflicts across concurrent pull requests, contributors add small fragment files under `.changes/` instead of editing `CHANGELOG.md` directly:

```text
.changes/<pr-number>.<category>.md
```

- **Nightly CI:** Aggregates pending fragments into release summaries without deleting them.
- **Stable Releases:** A release automation script compiles all accumulated fragments into a new section in `CHANGELOG.md` and deletes the processed fragment files.
