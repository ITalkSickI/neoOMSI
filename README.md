<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logos/openomsi-wordmark-light.svg">
    <img alt="openOMSI" src="assets/logos/openomsi-wordmark-dark.svg" width="420">
  </picture>
</p>

<p align="center">
  <a href="https://github.com/turbo-devv/openOMSI/releases/latest"><img alt="Version" src="https://img.shields.io/github/v/release/turbo-devv/openOMSI?label=version&color=f47f30&style=for-the-badge"></a>
  <a href="https://github.com/turbo-devv/openOMSI/actions/workflows/release.yml"><img alt="Build" src="https://img.shields.io/github/actions/workflow/status/turbo-devv/openOMSI/release.yml?branch=main&style=for-the-badge&label=build"></a>
  <a href="https://turbo-devv.github.io/openOMSI/"><img alt="Docs" src="https://img.shields.io/badge/docs-website-2d3138?style=for-the-badge"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/turbo-devv/openOMSI?style=for-the-badge"></a>
</p>

> [!WARNING]
> **Early release. Expect bugs.** openOMSI is in an early stage of development: things may be
> missing, broken or change between versions. Please report problems in
> [Issues](https://github.com/turbo-devv/openOMSI/issues).

**openOMSI** is a from-scratch recreation of the bus simulator **OMSI 2**, written in Rust:
64-bit, multithreaded, with a modern renderer (Metal / Vulkan / DirectX 12 through wgpu),
and fully compatible with the existing maps, buses, scenery and mods.

> [!IMPORTANT]
> **openOMSI needs an original copy of OMSI 2.** It contains no game content of its own: it
> plays on the maps, vehicles and other files of an installed OMSI 2 and **will not start without one**.


## Download

Every commit to `main` is built by GitHub Actions and published on the
[**Releases**](https://github.com/turbo-devv/openOMSI/releases) page:

| Platform | File |
| --- | --- |
| Windows x64 | `openOMSI-<version>-windows-x64.zip` - run `openomsi.exe` |
| macOS (Apple silicon) | `openOMSI-<version>-macos-arm64.zip` - open `openOMSI.app` |
| Linux x64 | `openOMSI-<version>-linux-x64.zip` - run `openomsi` |
| Android (arm64, 8.0+) | `openOMSI-<version>-android-arm64.apk` - see [docs/ANDROID.md](docs/ANDROID.md) |
| Dedicated server (Linux x64) | `openOMSI-<version>-server-linux-x64.zip` - see [docs/SERVER.md](docs/SERVER.md) |

Start the game, point the launcher to your OMSI 2 folder once, pick a map, a bus and a duty,
and drive. Mods go into the folder next to the game (or through the launcher's **Mods**
page); the original installation is never written to.

## Goals

1. **1:1 behaviour.** Every content format of the original - maps, splines, scenery objects,
   vehicles, scripts, timetables, HOF files, fonts, weather, tickets, situations, plugins -
   loads and behaves exactly as in OMSI 2.2.032. Existing maps and mods work unchanged.
2. **No original code or assets.** Nothing from the original is copied; the formats are
   described in [docs/FORMATS.md](docs/FORMATS.md).
3. **A better engine.** 64-bit address space, streaming and texture loading on worker threads,
   no 2 GB limit, no single-thread stalls, LAN multiplayer and a dedicated server.

## Documentation

The full documentation is on the website: **https://turbo-devv.github.io/openOMSI/**. The same
pages live in [`docs/`](docs):

| Document | What is in it |
| --- | --- |
| [User guide](docs/USER_GUIDE.md) | running, controls, launcher, settings, mods, LAN play, debug switches |
| [Android](docs/ANDROID.md) | the mobile version: install, touch controls, building the APK |
| [PBR materials](docs/PBR.md) | normal, roughness, metalness and occlusion maps for mods |
| [Building](docs/BUILDING.md) | building from source on macOS, Windows and Linux |
| [Content formats](docs/FORMATS.md) | every OMSI 2 file format |
| [Architecture](docs/ARCHITECTURE.md) | crates, threading, renderer, roadmap |
| [Routes](docs/ROUTES.md) | how the original runs timetables, chrono, HOF, IBIS |
| [Plugins](docs/PLUGINS.md) | OMSI plugin DLLs and the 32-bit plugin host |
| [Dedicated server](docs/SERVER.md) | hosting a session without a window |
| [Versioning & releases](docs/VERSIONING.md) | the `MAJOR.MINOR.COMMIT` scheme and the CI |

## Building from source

```sh
git clone https://github.com/turbo-devv/openOMSI.git && cd openOMSI
scripts/build-macos.sh        # macOS   → dist/macos/openOMSI.app
scripts\build-windows.cmd     # Windows → dist\windows\openomsi.exe
scripts/build-linux.sh        # Linux   → dist/linux/openomsi
scripts/build-android.sh      # Android → dist/android/openOMSI-<version>.apk
scripts/build-server.sh       # server  → dist/server
```

Needs [Rust stable](https://rustup.rs) (1.85+) and the platform's C toolchain; details in
[docs/BUILDING.md](docs/BUILDING.md).

## Repository layout

```
openOMSI/
├── VERSION            MAJOR.MINOR of the next release (edited by hand)
├── crates/            the engine, one crate per subsystem of the original
│   ├── omsi-app/        the game binary `openomsi` (window, launcher, HUD, server mode)
│   ├── omsi-launcher-core/  launcher data side + `openomsi-launcher` terminal tool
│   ├── omsi-cfg/        text files, code pages, virtual file system, content roots
│   ├── omsi-script/     the OMSI script language (compiler + VM)
│   ├── omsi-o3d/ omsi-model/ omsi-texture/ omsi-geometry/   meshes, models, textures, splines
│   ├── omsi-map/ omsi-scenery/ omsi-timetable/ omsi-vehicle/ omsi-content/   content formats
│   ├── omsi-sim/        vehicles, AI traffic, people, physics
│   ├── omsi-render/     the wgpu renderer
│   ├── omsi-audio/ omsi-net/ omsi-plugin/ omsi-ui/   sound, multiplayer, plugins, UI toolkit
├── tools/             developer tools: omsi-check (format coverage)
├── scripts/           build scripts for every platform, version.sh, packaging files
├── assets/            fonts, Material icons, app icons (assets/icons/app), logos (assets/logos)
├── docs/              documentation (also published as the website)
├── site/              the GitHub Pages website
└── .github/workflows/ CI: release builds for every commit, the website
```

## Contributing

Issues and pull requests are welcome - see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

openOMSI is released under the [MIT License](LICENSE). OMSI and OMSI 2 are trademarks of their
respective owners. openOMSI is an independent project and is not affiliated with them.
