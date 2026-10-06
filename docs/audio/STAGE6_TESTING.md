# Stage 6 test handover

Implementation date: 2026-10-06. Phase 1 / step 6 only; no Steam Audio.

## Verified in this workspace

- `cargo check -p omsi-audio --all-targets --locked`: succeeds without compiler warnings,
  including all existing/new unit tests, integration fixtures and examples.
- `cargo check -p omsi-sim --lib --locked`: succeeds.
- Existing `VoiceParams` literals, `Playback` methods and façade aliases are preserved.
- No dependencies, unsafe or mutable globals were added. Audio Rust files remain below
  500 lines; quality and mixer tests are separate modules.

The original SDK/linker limitation was resolved on 2026-10-06 by installing Microsoft
C++ Build Tools and Windows SDK with explicit user approval. `cargo test -p omsi-audio
--locked` now executes successfully: **66 unit + 8 offline + 5 quality + 2 runtime tests =
81 passed**. The optimized `dev-release` application build succeeds with version
`0.2.0-audio-stage6`; its `--version`/`--help` startup and the offline render succeeded.
The complete workspace/all-target check and production release profile remain pending.

Test executable: `H:\neoOMSI\dist\windows-audio-stage6\neoomsi.exe`.
Its content directories link to the existing windows-dev content. A separate folder with
canonical `neoomsi.exe` naming is necessary: the integrated launcher prioritizes that name.
The previous windows-dev executable is retained for comparison. Two leftover stamper
producers in `redraw/world.rs` were migrated from the removed `fired_triggers` list.

### Measured synthetic workload (optimized dev-release)

| Vehicle sources (+5 auxiliary clip buses) | Mean ms | p95 ms | Max ms | Render budget utilization | Observed peak working set |
|---|---:|---:|---:|---:|---:|
| 32 | 0.531 | 0.547 | 0.882 | 10.0% | 7.70 MiB |
| 100 | 1.556 | 1.638 | 2.667 | 29.2% | 7.14 MiB |
| 200 | 2.984 | 3.104 | 4.833 | 56.0% | 7.14 MiB |
| 500 | 3.055 | 3.164 | 8.481 | 57.3% | 7.35 MiB |

Block budget is 5.333 ms at 48 kHz/256 frames. All cases report zero dropped commands
and zero underruns; the limiter peak remains at/below 0.9. Only 200 clip voices are mixed;
others remain virtual and advance. The 500-source case has an isolated over-budget peak,
so sustained real-time safety is not established by the short synthetic probe. The probe
shares one 88200-byte clip and uses synthetic radio/dialogue bus clips, not a live stream;
its memory figures are **not** full-map or fleet memory. Raw CSV/process CPU records are
under `out/audio-stage6/`; the hearing and device tests below remain necessary.

## Run the gate on your development machine

Use the normal Windows developer shell with the repository's build prerequisites:

```powershell
.\scripts\test-audio-stage6.ps1
# Optional actual station decode probe (uses the supplied URL):
.\scripts\test-audio-stage6.ps1 -RadioUrl 'https://your-station/stream'
```

The script checks audio/sim, runs nextest (or cargo test if nextest is absent), checks the
whole workspace, builds the release application/examples, renders `out/audio-stage6/offline.wav`
and runs synthetic 32/100/200/500-source probes. CSV records callback timing (mean/p95/max
against the 5.33 ms block budget), render-budget utilization, sample payload, limiter peak
and fault counters. `process-memory.csv` records Windows process peak working set and
process CPU time if available. These are synthetic measurements, not representative map
results. The initial dev-release measurements are recorded above; this script reruns the
complete gate and measurements under the production release profile.

For just the audio tests or load probe:

```powershell
cargo nextest run -p omsi-audio --locked
cargo run --release -p omsi-audio --example audio_load --locked -- 200
```

## In-game checks

1. Start with no output device; attach headphones. Loops/radio should become audible
   without restarting the game. Horns/steps fired while disconnected must not play late.
2. Switch default output, disconnect it while driving, then reconnect the same named
   device. Repeated failed attempts should retry at most once/second. No doubled radio,
   permanently silent loop, or crash. Reconnection restarts loop phase intentionally.
3. Try headphones/stereo, mono, integer-format devices and a surround endpoint. Front
   L/R carry stereo, mono averages the channels, surround rear/centre/LFE remain silent.
4. Sweep RPM from idle to high speed; open/close doors and windows, change inside/outside
   view, stop/start sounds, and listen for clicks or seams. Record whether 3 ms fades and
   the seam correction affect authentic engine/door transients acceptably.
5. Dense traffic at a large stop: player, AI, LAN vehicle, articulated rear section,
   scenery, rain, passenger steps/dialogue, radio and announcements together. Test tile
   unload/reload and vehicle swaps; all voices should end or resume by the defined rules.
6. Trigger announcements and HTML `omsi.playSound`, including on coupled sections.
   Announcements keep their fire-time peak and bus on later frame updates.
7. Pause/unpause and virtualize >200 sounds; phase advancement is unchanged. Compare
   retained legacy behavior to the Stage 5 fixtures, not inferred OMSI rules.
8. Compare loudness and limiting with Stage 5. Stage 6 reserves 6 dB of master headroom;
   ambience defaults to 0.8 and radio to 0.7. Lower output level is intentional, but the
   balance/limiter and audible quality still need acceptance on actual content.

Record map/vehicle, source counts, output device/format, p95/max render time, fault counters,
CPU/process peak working set and hearing observations for quiet cab, busy terminus and
heavy rain plus radio. Use the same release build/machine for comparisons. The synthetic
probe intentionally shares one clip; real fleet cache consumption will be larger.

## Gate decision

**Ready for local testing; acceptance pending.** The 81 audio tests, optimized test-EXE
build, CLI startup, offline rendering and synthetic timing/process-memory probe succeed.
The full workspace/all-target check, production release profile, representative map
hearing/performance and physical device reconnection remain pending. The 500-source probe
includes a peak above its callback deadline; do not infer guaranteed real-time behavior
from its average. The nine live OMSI questions in `SOUND_ENGINE_BEHAVIOR.md` remain open.
Phase 2 remains gated.

## Stage 6.1 retest

After the C2 EN 6/BVG listening report, 90 audio tests pass (75 unit, 8 offline, 5 quality,
2 runtime). `0.2.0-audio-stage6.1` is built and its version start succeeds. New executable:
`H:\neoOMSI\dist\windows-audio-stage6-1\neoomsi.exe`. Prior test builds remain available.

Synthetic dev-release retest with phase interpolation and channel smoothing:

| Sources (+5 auxiliary) | Mean ms | p95 ms | Max ms |
|---|---:|---:|---:|
| 32 | 0.614 | 0.682 | 0.976 |
| 100 | 1.706 | 1.786 | 2.258 |
| 200 | 3.256 | 3.387 | 5.462 |
| 500 | 3.394 | 3.505 | 4.196 |

All cases have zero dropped commands and zero underruns; ceiling <=0.9. The 200-source
sample includes an over-budget peak, so representative-game scheduling/hearing still
requires testing. These short probes are not an uninterrupted-playback guarantee.
Raw retest CSV: `out/audio-stage6-1/`. Focus hearing QA on both-ear engine balance,
closed/open doors, fresh vehicle entries and control-noise/faint squeak reports; see
`STAGE6_SPIELTEST.md` and behavior section 11 for the exact corrections.

## Listening test 6.3

95 audio tests pass. Interior Announcement-bus entries (including untagged C2 file triggers) transmit through the source bus's body when heard outside, with source-script opening control, distance falloff and no restart on camera/door changes. See SOUND_ENGINE_BEHAVIOR.md. Dev-release app build and a separate package are provided; in-game acoustic acceptance remains pending. The package includes the optional C2 click recordings from 6.2.

## Listening test 6.4

99 audio tests pass. Outside PA transmission is reduced from 0.35/2200 Hz closed
and 1.0/9000 Hz open to 0.08/900 Hz and 0.30/4000 Hz. Interior PA receives a dedicated
25% wet, 0.45 s Schroeder return with 4500 Hz wet damping, while its direct share is
reduced. Tail/decay, bypass, outside dry output, bus mute, pause, callback partitions
and the previous runtime regressions are verified. The effect buffers are prepared
outside callbacks, idle effect processing is skipped after decay, and raw VoiceParams
remains unchanged. Hall is not automatically detected in recordings; a dry launcher
and SoundSet setter allow bypass. Matching OMSI or real-bus listening remains open.

Short 6.4 dev-release probe, 48 kHz / 256 frames, active PA and world hall:
100 voices plus 5 auxiliary: mean 1.7333 ms, p95 1.8406 ms, max 3.0128 ms.
200 plus 5 auxiliary: mean 3.3163 ms, p95 3.5647 ms, max 4.3145 ms.
Both are below the 5.3333 ms block duration in this short run; zero reported command
drops and radio underruns. This synthetic probe is not a real-map deadline guarantee.
CSV: out/audio-stage6-4/load-100.csv and load-200.csv.
