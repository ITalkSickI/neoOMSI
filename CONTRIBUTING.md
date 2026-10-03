# Contributing to neoOMSI

Thanks for contributing. neoOMSI optimizes for correctness, simplicity, and long-term maintainability rather than raw merge volume.

Before starting substantial work, please review:
- [Development workflow](docs/DEVELOPMENT.md)
- [OMSI compatibility policy](docs/COMPATIBILITY.md)
- [Issue triage](docs/ISSUE_TRIAGE.md)

## Core rules

1. **Compatibility first.** If a change touches OMSI-visible simulation behavior, verify what OMSI 2.2.032 actually does before altering engine logic.
2. **Clean-room implementation.** Never copy proprietary OMSI 2 source code, decompiled binaries, or copyrighted assets (textures, meshes, audio, maps, scripts) into the repository.
3. **Keep changes focused.** One pull request should have one coherent purpose. Avoid combining bug fixes with unrelated formatting, cosmetic refactoring, or dependency bumps.
4. **Prefer simplicity.** Delete unnecessary code before introducing abstractions. Never add defensive branches, compatibility layers, or configuration switches for hypothetical future problems.
5. **Leave touched code cleaner.** Proportional local cleanup is encouraged within the scope of your changes, but avoid sprawling rewrites.
6. **Code ownership.** The PR author must understand every line submitted and be able to explain the engineering rationale during review.

## AI-assisted contributions

AI-assisted development is permitted, but generated code receives **no lower review standard**. The contributor remains fully responsible for every submitted line.

* Do not submit large, AI-generated rewrites that you have not personally reviewed, understood, and tested.
* Treat AI suggestions as drafts: verify assumptions, strip out unnecessary complexity or boilerplate, match existing crate conventions, and add reproducible tests.
* Unchecked "vibe coding" and speculative AI code dumps will be closed during triage.

## Branch and pull request workflow

Work from latest `main` using short-lived branches:

```text
feat/<name>       new engine capability
fix/<name>        bug fix or regression repair
parity/<name>     OMSI 2 compatibility work
refactor/<name>   behavior-preserving simplification
perf/<name>       performance optimization
docs/<name>       documentation update
chore/<name>      tooling or build maintenance
```

Pull requests target `main`. There is no permanent `develop` or `nightly` branch.

### Local development builds

For fast local iteration while developing, use the development scripts:

* **Windows:** `scripts\dev-windows.cmd` (or `scripts\dev-windows-release.cmd` for optimized local testing)
* **macOS:** `sh scripts/dev-macos.sh`

### Pull request expectations

Every PR should document:
- What was changed and why.
- Observable impact for players or content authors.
- Validation performed (unit tests, manual reproduction, maps/buses tested).
- For parity work: reference OMSI 2.2.032 behavior and verification evidence.

## Changelog fragments

Every user-visible PR must add a fragment in `.changes/<pr-number>.<category>.md`:

```text
.changes/412.parity.md
```

```markdown
Fixed keyboard steering return behavior to match OMSI 2.
```

Supported categories: `parity`, `fix`, `feature`, `performance`, `breaking`, `internal`.
If a change is purely internal, maintainers may apply the `skip-changelog` label. See [.changes/README.md](.changes/README.md).

## Verification

Before opening a pull request, run workspace checks:

```sh
cargo test --workspace
cargo build --release
```

Add deterministic regression tests whenever a bug or parity rule can be reproduced reliably. Tests must never bundle proprietary OMSI assets; read them from `OMSI_ROOT` if required.
