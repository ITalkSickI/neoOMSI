# Development workflow

This guide covers engineering standards, branching, code review, and development practices for neoOMSI.

## Engineering principles

When making trade-offs, prioritize:

1. **Correctness** — simulation behavior must accurately reflect verified reference behavior.
2. **Simplicity** — choose the simplest complete solution over complex abstractions.
3. **Maintainability** — clear data flow and explicit ownership beat clever or hidden mechanisms.
4. **Consistency** — follow established crate conventions and standard Rust idioms.
5. **Minimal churn** — keep diffs focused on the task at hand. Avoid formatting or refactoring untouched code.

## AI-assisted code standards

AI tools may assist with development, but generated code receives **no lower review standard**:

- Every line of submitted code must be understood, verified, and defended by the PR author.
- Speculative AI code dumps, unreviewed vibe-coded refactors, and phantom abstractions will be rejected during triage.
- Treat generated code as an untrusted draft: eliminate boilerplate, verify assumptions against OMSI 2.2.032 reference behavior, and write focused regression tests.

## Branching model

neoOMSI uses a **trunk-based workflow**. `main` is the sole permanent integration branch.

```text
main
├── feat/...         (new engine capabilities)
├── fix/...          (bug fixes and regressions)
├── parity/...       (verified compatibility improvements)
├── refactor/...     (simplifications preserving behavior)
├── perf/...         (performance optimizations)
├── docs/...         (documentation updates)
├── chore/...        (tooling and build infrastructure)
└── release/x.y      (temporary stabilization branches)
```

### Working with branches

- Create focused, short-lived branches directly from `main`.
- Keep changes scoped to a single purpose. Avoid combining refactors with behavioral fixes.
- Delete branches upon merge.

### Release branches

Temporary `release/x.y` branches are created only to stabilize Release Candidates. Normal development continues unhindered on `main`. See [Releasing](RELEASING.md).

## Local development builds

To streamline testing during day-to-day work, use the fast development scripts:

| Platform | Command | Description |
| --- | --- | --- |
| **Windows** | `scripts\dev-windows.cmd` | Fast development profile build; launches automatically |
| **Windows (Dev Release)** | `scripts\dev-windows-release.cmd` | Optimized local build without slow LTO passes (`dist\windows-dev`) |
| **macOS** | `sh scripts/dev-macos.sh` | Fast development build using Metal |

Game arguments can be passed directly after each command:

```sh
scripts\dev-windows.cmd --map maps/Grundorf/global.cfg --bus Vehicles/MAN_SD200/MAN_SD80.bus
```

Before opening a pull request, run workspace checks:

```sh
cargo test --workspace
cargo build --release
```

## Pull requests

### PR scope

- Keep pull requests small and focused on a single architectural or behavioral boundary.
- Do not mix parity fixes with unrelated cosmetic refactoring or dependencies updates.
- If a larger architectural defect is discovered, address only what is needed for the current fix and file a dedicated issue or follow-up for the remainder.

### Review standards

Pull requests merged into `main` require:
- At least **two approving reviews** from maintainers.
- Passing continuous integration checks.
- For `parity/` changes: clear OMSI 2 reference evidence that can be independently validated.
- A changelog fragment for any user-visible change.

## Testing guidelines

- Bug fixes should include a unit or integration test reproducing the original issue whenever practical.
- Parsers, format deserializers, and math routines must have direct unit test coverage.
- Tests must **never** bundle proprietary OMSI 2 game assets. Where test fixtures are required, use synthetic mock data or read optionally from `OMSI_ROOT`.

## Changelog fragments

Rather than editing `CHANGELOG.md` directly (which causes merge conflicts), each user-facing PR adds a small markdown fragment under `.changes/`:

```text
.changes/<pr-number>.<category>.md
```

Supported categories:
- `parity`
- `fix`
- `feature`
- `performance`
- `breaking`
- `internal`

Example (`.changes/412.parity.md`):
```markdown
Corrected pneumatic brake pressure drop curve when operating the handbrake lever.
```

Nightly CI aggregates unreleased fragments automatically. When preparing a Stable release, fragments are compiled into `CHANGELOG.md` and deleted. See [Releasing](RELEASING.md).
