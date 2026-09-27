# Contributing to openOMSI

Thanks for helping! A few rules keep the project healthy:

* **No original code or assets.** Never copy anything from an OMSI 2 installation into the
  repository — no textures, models, sounds, maps, scripts or disassembled code. Tests that need
  content read it from a local installation (`OMSI_ROOT`) and skip themselves without one.
  Reverse-engineering results are written down in `docs/` in our own words.
* **Compatibility first.** A change must not break a stock map or a mod that worked. Run
  `cargo run --release -p omsi-check -- "/path/to/OMSI 2"` before and after larger changes.
* **One change per pull request**, with a message that says what the player notices.
* `cargo test --workspace` and `cargo build --release` must pass (CI checks all platforms).
* Code style: `rustfmt` defaults, comments explain *why*; cite the `Omsi.exe` address
  (`sub_xxxxxx`) when following the original's behaviour.

## Where things are

See the layout in the [README](README.md#repository-layout) and
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). File formats: [docs/FORMATS.md](docs/FORMATS.md).

## Releases

Maintainers bump `MAJOR.MINOR` in the `VERSION` file; everything else is automatic — see
[docs/VERSIONING.md](docs/VERSIONING.md).
