# Contributing to neoOMSI

Thank you for contributing to neoOMSI. Please review our project guidelines before opening an issue or pull request:

## Core principles

* **Clean-room implementation.** Never import or commit proprietary OMSI 2 source code, binaries, or game assets (textures, meshes, audio, maps, scripts). Tests that require game assets read them dynamically from an existing installation via `OMSI_ROOT`.
* **Behavioral parity first.** neoOMSI targets behavioral compatibility with OMSI 2.2.032. Changes to engine logic must preserve or improve reference parity rather than invent custom behavior. See [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md).
* **Focused pull requests.** Keep PRs small, well-documented, and scoped to a single architectural or behavioral concern.
* **Trunk-based workflow.** Branch from `main` using standard prefixes (`feat/`, `fix/`, `parity/`, `refactor/`, `perf/`, `docs/`, `chore/`). See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).
* **Code standards.** Standard `rustfmt` formatting, idiomatic Rust, and comments that explain *intent and rationale* rather than obvious code.

## Local development

For fast iteration while developing, use the development scripts:

* **Windows:** `scripts\dev-windows.cmd`
* **macOS:** `sh scripts/dev-macos.sh`

On Windows, `scripts\dev-windows-release.cmd` compiles an optimized build into `dist\windows-dev` without slow final LTO passes, ideal for testing release-specific behavior.

Before submitting a pull request, run workspace tests:

```sh
cargo test --workspace
cargo build --release
```

## Documentation & policies

* **[Development workflow](docs/DEVELOPMENT.md)** — Branching, review requirements, and CI.
* **[Compatibility policy](docs/COMPATIBILITY.md)** — How OMSI 2 behavior is verified and documented.
* **[Building guide](docs/BUILDING.md)** — System requirements and cross-platform builds.
* **[Issue triage](docs/ISSUE_TRIAGE.md)** — Bug reporting and triage lifecycle.
* **[Release process](docs/RELEASING.md)** — Nightly builds, versioning, and changelog fragments.
