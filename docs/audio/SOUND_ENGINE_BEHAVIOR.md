# Sound Engine Behavior

This document is the agreed, testable description of what the OMSI sound runtime does
*before* the Phase 1 refactor and what it must keep doing afterwards. It is the ground
truth for the offline fixtures in `crates/omsi-audio/tests/` and for the Block B rule
corrections.

It is not a design document: it records observable behavior (which sound plays, with which
parameters and for how long) for the cases the rework touches.

## How to read this

- **Status `CONFIRMED`** - the rule is encoded in the current code and backed by an OMSI
  assembly/decompile observation, or is a mechanical consequence of the mixer.
- **Status `PARTIAL`** - part of the behavior is implemented; the rest is missing or only
  applies on one path. The gap is named.
- **Status `OPEN`** - the exact OMSI behavior is not established from the static reference.
  A small live OMSI comparison is required before the rule is frozen. The intended
  comparison is listed with the case.

Evidence abbreviations:

- `TSound update` / `0x…` - offsets in `omsi.exe` 2.2.032, as recorded in the code
  comments and in the reference analysis under `H:\marcel_omsi`.
- `soundset.rs` / `mixer.rs` / `runtime/*` - the current neoOMSI implementation.

The behavior is exercised without an audio device by
`cargo run -p omsi-audio --example offline_render` and by
`cargo test -p omsi-audio` (fixtures under `crates/omsi-audio/tests/fixtures/soundcfg/`).

## 1. Entry kinds and how they play

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| `[sound]` without trigger | entry with conditions that hold | loops for as long as the conditions hold and it is audible | CONFIRMED | `SoundSet::update_fired`: `!triggered && !no_loop` -> looping voice; TSound update |
| `[loopsound]` | loop with `pitch_variable`/`pitch_ref` | loops; playback rate `|pitch| * sample_rate / ref`, silent below 100 Hz | CONFIRMED | `runtime::conditions::pitch_of`; buffer frequency set by TSound |
| `[noloop]` without trigger | one-shot entry | plays once, when its conditions **start** to hold | CONFIRMED | `SoundSet::update_fired`: `rising` starts a non-looping voice |
| `[trigger]` entry | trigger fires | plays once from the start; **conditions are not evaluated**; a triggered `[loopsound]` is not looped | CONFIRMED | `volume` skips conditions when `triggers` is non-empty; TSound update |
| `(T.F.trigger)` file trigger | script file trigger | the entry listening for `trigger` plays the named file | CONFIRMED | `SoundSet::play_file_trigger`; `omsi-sim` `fired_file_triggers` |
| trigger volume read at fire | `[volcurve]` on a variable the script changes after firing | volume is read at the moment the trigger fires, not at frame end; file triggers also use their own snapshot and initialize peak hold | CONFIRMED (implementation) | `SoundSet::update_events`; `runtime::files`; fixtures `trigger.cfg`; `quality_runtime.rs` (included in the 81 passing audio tests) |
| triggered peak hold | triggered entry whose curve falls while it plays | volume never drops below its value since the trigger fired | CONFIRMED | `runtime::conditions::peak_hold`; TSound +0x2c / 0x7507bc |

## 2. Conditions, curves and variables

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| conditions | `[conditionSingle/Int/Bool]` | entry plays only while all conditions hold; a triggered entry ignores them | CONFIRMED | `SoundSet::volume`; `runtime::conditions::conditions_hold` |
| volume curves | `[volcurve] name` + `[pnt]` | piecewise-linear factor; clamped at both ends | CONFIRMED | `runtime::conditions::curve` |
| negative curve variable `-1` | time-since-active input | seconds since the entry's conditions started to hold | CONFIRMED | `runtime::conditions::curve_input`; TSound update 0x750584 |
| negative curve variable `-2` | facing input | how much a `[3d]` entry with `[dir]` faces the listener (1 without one) | CONFIRMED | `runtime::conditions::curve_input` |
| other negative index | e.g. `-3` | the entry is skipped ("Volume Variable not valid!") | CONFIRMED | `runtime::conditions::curve_input` |
| unknown variable | name not a script variable | read as 0 | CONFIRMED | `curve_input` -> `var(...).unwrap_or(0.0)` |
| gain clamp | volume factor over 1 | plays at 1 (DirectSound has no gain over 0 dB) | CONFIRMED | the product is clamped once, after the global masters, in `voice::Voice::render_into`; `runtime::level`; MB 412D case |
| volume conversion | linear level | converted to DirectSound hundredths of dB (`20·log10`), clamped to at most 0 dB and at least -100 dB; `SetVolume` | CONFIRMED | `runtime::level::{hundredths_db, from_hundredths_db}`; `00750444` `00750bb0` / vtable +0x3c |
| loop pitch silence | pitch variable near 0 | below DirectSound's 100 Hz minimum the buffer is silent | CONFIRMED | `runtime::conditions::pitch_of` |

## 3. Viewpoint, inside/outside and `Snd_OutsideVol`

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| `[viewpoint]` gating | entry tagged inside/outside, listener on the other side | **open question**: in the current update path the entry is *not* cut by the listener's side (only `report` uses the listener view mask). The intended behavior is described in the `view_mask` comment. Behavior is unchanged by the Schritt-4 refactor and locked by a regression test. | OPEN | `RuntimeSound::update` -> `volume_side` builds the view from the entry's own bits; `report` uses `placement::view_mask`; test `viewpoint_gating_is_taken_from_the_entry_not_the_listener` |
| AI-only entry | `[viewpoint]` bit 4 on a non-AI vehicle | not heard | CONFIRMED | `volume_side`: entry view `(viewpoint & 3) | ai<<2`; `runtime::conditions::volume` |
| `Snd_OutsideVol` through-path | own bus, outside-only entry (`[viewpoint] 5`), listener in cab, variable set | played at the variable's value when over 0.01, else silent | CONFIRMED (when the listener view is supplied) | `runtime::conditions::volume`; TSound update 0x750340; `set_outside_open` |
| `Snd_OutsideVol` scaling | doors/window open | "shut" keeps a quarter; open lets it through (`sound_volume.osc`) | CONFIRMED (exterior sets) | `outside_gain`/`lowpass_of` are wired into an `exterior` set's transmission/timbre while the listener is muffled (`runtime::sound`); the own bus's outside-only entries use the `conditions::split` through-path instead, so the two never double-apply. The own-bus through-path is still gated by the entry's own bits, see `[viewpoint]` below (OPEN). |

## 4. Ordering and repeated events

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| repeat in one frame | a trigger fired twice in one script frame | the entry restarts once per frame; both firings survive as separate events (`runtime::event::SoundEvent`) | PARTIAL | `omsi-sim` `fired_sounds` is one ordered `Vec` and keeps duplicates; `SoundSet::update_events` still restarts once (the old `find` rule). Fixtures: `trigger.cfg`; tests `a_repeated_trigger_restarts_once_but_both_events_survive`, `two_identical_triggers_in_one_frame_start_one_voice` |
| trigger order across subsystems | player + AI + scenery triggers in one frame | one ordered stream; each event carries `EventSource` and a within-source `seq`, so `(source, seq)` is the explicit redraw order traffic -> player -> LAN -> scenery | PARTIAL | `runtime::event::{EventSource, SoundEvent, ordered}`; subsystems feed `SoundSet::update_events`. The exact cross-subsystem rule stays OPEN |
| normal vs file trigger ordering | `(T.…)` and `(T.F.…)` in one frame | normal and `(T.F.)` triggers are one ordered list (`omsi-sim` `fired_sounds`); `update_events` applies normal triggers then file triggers within the frame (as before) | PARTIAL | `omsi-sim` `FiredSound`; `SoundSet::update_events`. Relative application order stays OPEN |

## 5. Start phase, restart and end

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| random loop start | loop start position | **open question**: the reference can start a loop at a random buffer offset; the current mixer always starts voices at position 0 (except loop wrap) | OPEN | plan section 1; `AudioEngine::play` starts `pos = 0.0` |
| one-shot restart | trigger fires while the entry's one-shot still plays | stop the old voice and start the entry again, unless `[onlyone]` keeps the running one | CONFIRMED | `SoundSet::update_fired` one-shot branch |
| voice end | non-looping clip reaches its end | voice is removed; a looping clip wraps with no gap | CONFIRMED | `mixer` `Shared::render`; `a_loop_has_no_gap_where_it_turns` test |
| distance | `[3d]` range and distance | full up to `range`, then inverse-distance (1/d); applies once | CONFIRMED | `mixer::distance_gain` |
| Doppler | moving `[3d]` entry | closer = higher, away = lower; disabled for the listener's own vehicle and by `OMSI_DOPPLER` | CONFIRMED | `mixer::apply_params` |
| resume rule (renderer limit) | a voice skipped because of `MAX_VOICES` becomes audible again | the voice kept running and its phase advanced; it is mixed again from that position | CONFIRMED | `mixer::Shared::render` `skip`; ranking recomputed each block |
| resume rule (OMSI) | a sound the runtime stopped/never admitted becomes audible again | it is admitted again; a one-shot starts from the start, a loop resumes only while its voice still runs | CONFIRMED | `runtime::level::SOUND_MAXCOUNT`; `RuntimeSound::update` admission branch |

## 6. Loading, `[checkloading]`, `[onlyone]`, missing files

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| missing file | file cannot be read | the entry is skipped and warned about once per file | CONFIRMED | `SoundSet::warn_missing_once`, `read_clip` |
| `[checkloading]` | flag on an entry | **not evaluated at runtime** | OPEN | parsed in `omsi-vehicle/src/sound.rs`; no consumer |
| `[onlyone]` | flag on a trigger entry | a central, file-keyed registry on the sound set: another entry of the same file reuses the running voice instead of restarting it. The exact scope (same file / same vehicle / global, and coupled parts) is OPEN. | PARTIAL | `RuntimeSound::update` consults `SoundSet`'s `OnlyOne` (file -> voice); fixture `onlyone.cfg`; tests `onlyone_is_central_per_file`, `onlyone_entries_share_a_single_voice` |
| reload after unload | tile/vehicle unloaded then loaded again | voices are stopped on unload; a fixed file is read lazily and retried from the cache, so a reload needs no contradictory state. An event accepted while the clip loads starts when it arrives. | CONFIRMED (asset separation) | `RuntimeSound::{Asset, resolve_asset, pending}`; `SoundSet::update_events`; fixture `trigger.cfg`; tests `an_accepted_event_survives_an_asynchronous_load`, `a_triggered_event_not_loaded_yet_keeps_its_start` |
| sound states | an entry's logical state | explicit `SoundState`: `NotLoaded`, `Loading`, `Ready`, `Running`, `Suppressed`, `Stopped`, `Ended`, apart from the asset cache. Renderer virtualization is not mirrored (a renderer swap cannot change runtime behavior). | CONFIRMED (model) | `runtime::sound::{Asset, SoundState}`; `SoundSet::state` |

## 7. Packs and budgets

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| pack ordering | several sound packs in earshot | the reference sorts packs by normalized distance, then budgets | OPEN | plan section 1; current mixer ranks individual voices instead |
| admission vs renderer limit | many vehicles | two separate decisions: the runtime admits at most `[sound_maxcount]` (200) voices per set (an OMSI decision with the sound type's lifetime rules); the mixer keeps `[important]` first, then the loudest, up to `MAX_VOICES` as a performance-only virtualization that never changes what the runtime believes is playing | PARTIAL | `runtime::level::SOUND_MAXCOUNT`; `runtime::soundset::SoundSet::update_frame`; `mixer::Shared::render`; `MAX_VOICES` |
| later-audible voice | a voice that was skipped becomes the loudest | it is mixed again from its advanced position; a sound stopped by the runtime is re-admitted and starts per its type | CONFIRMED | `skip_clip` advances the position; ranking is recomputed each block; `RuntimeSound::update` |

## 8. Pause, time base and ownership

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| pause | game paused | master is 0; the mixer keeps running; time base unchanged | PARTIAL | `redraw/player.rs` sets `master = 0` when paused; `Instant` is not replaced by simulation time |
| player vehicle | own bus | 3D sounds pan and fade, but frame timing must not create Doppler pitch shifts | CONFIRMED | `SoundSet.listener_vehicle`, `doppler = !listener_vehicle` |
| AI / other players | exterior sound set | non-3D entries are placed at the vehicle and fade with distance | CONFIRMED | `SoundSet::new_exterior`, `placement::place` |
| trailers / coupled sets | coupled parts | each part's set updates with the leading vehicle's variables and complete event stream (normal/file triggers and fire-time snapshots), at the part's transform | CONFIRMED | `SoundSet::update_parts`, `add_part` |

## 9. Legacy parameter split and renderer order

Schritt 5 hands the renderer separate values instead of one pre-mixed gain and documents the
order in which they are applied. The reference's `TSound` update (`00750444`) computes the
parts separately and only then calls `SetVolume` (hundredths of dB, clamped to 0 dB),
`SetPan` and `SetFrequency`; distance is a separate 3D min-distance.

```text
final  = clamp01(record × script × transmission × set_master × listener_master)
voice  = sample × final × distance_gain × pan → low-pass
output = limiter(headroom × sum(bus_gain × bus_samples) → reverb)
```

- `record` is the `[sound]`/`[loopsound]` recording level, `script` the product of the
  `[volcurve]` factors, `transmission` the inside/outside share. They are carried by
  `voice::Level::Omsi` inside the renderer-only `voice::MixParams`; the public
  `voice::VoiceParams` and every application call site stay unchanged (the app's raw gain is
  `voice::Level::Raw` and is not clamped by the runtime's 0 dB rule).
- The clamp happens **once**, after the set master and the listener master, so a loud
  recording with a small master is not cut early (a level 4.0 with master 0.5 gives 1.0,
  not 0.5).
- Distance and inside/outside apply exactly once: distance in `spatial`, the own-bus
  `Snd_OutsideVol` share in `conditions::split`, the bodywork/`Snd_OutsideVol` of a foreign
  set in `outside::{outside_gain, lowpass_of}`. The paths are disjoint by `exterior`.
- Pan uses near/far channel damping. Stage 6.1 bounds the listening preset to 12 dB
  and narrows own-cabin sources after feedback about muted ears. The exact OMSI scale
  stays OPEN; these listening adjustments are not new OMSI comparison evidence.
- Frequency is `pitch_of` (loop rate, silent below 100 Hz), unchanged.

## Open items to resolve with live OMSI

These are the `OPEN` entries above, collected for the comparison runs the plan requires
before Block B freezes the rules:

1. Is `[viewpoint]` used to gate playback by the listener's actual inside/outside view, and
   how does it interact with `Snd_OutsideVol`?
2. How are two identical triggers in one frame handled (restart once or not)?
3. Is there a global ordering between normal and `(T.F.)` file triggers within a frame?
4. Do loop voices start at a random buffer offset, and how is it computed?
5. What is the exact scope of `[onlyone]` (same entry, same file, same vehicle, global)?
6. What does `[checkloading]` change at load/unload time?
7. What is the sound-pack sort key and budget rule, and how does a previously suppressed
   voice resume? Schritt 5 separates the runtime admission budget (`[sound_maxcount]`) from
   the mixer's `MAX_VOICES` virtualization and defines the resume rule per type, but the
   pack *sort key* is unchanged (the mixer still ranks individual voices) until the
   comparison settles it.
8. What is the time base under pause, and are loop/sound ages frozen or advanced?
9. What is the exact OMSI pan scale and curve handed to `SetPan`? Schritt 5 reproduces the
   documented DirectSound one-channel-damping behaviour with a fixture, but the reference
   only shows the direction calculation (`00750444` `00750a03`), not DirectSound's DSP.
   Likewise the exact dB floor for `SetVolume`.

Once answered, each status above is updated to `CONFIRMED` with the comparison that
settled it, and a fixture is added under `crates/omsi-audio/tests/fixtures/soundcfg/`.

## Note on the Schritt-4 event stream

The ordered stream (`runtime::event::SoundEvent`) and the explicit sound states
(`runtime::sound::SoundState`) are the representation Schritt 4 adds; they do not answer the
questions above by themselves. Each subsystem feeds one source-tagged stream
(`omsi-app::sound_events::events_from`), and an entry's accepted start now survives an
asynchronous asset load (`RuntimeSound::{Asset, resolve_asset, pending}`). The rule
*outcomes* stay as they were until the comparisons above settle them.

## 10. Stage 6 rendering and output decisions

These are neoOMSI quality/robustness decisions, not newly established OMSI rules.
The implementation and all test targets compile; all 81 audio tests now execute and pass
after the approved Windows SDK installation. Hearing/device QA is still pending. See `STAGE6_TESTING.md` for the handover.
All nine live-OMSI questions above remain OPEN/PARTIAL; no comparison was available here.

- **Resampling:** deterministic 32-tap Blackman-windowed sinc, 256 fractional phases,
  25 quarter-octave cutoff bands; choose the next narrower band at each smoothed step.
  Cutoff is 0.94 of source Nyquist at unity rate and reduced for decimation. Ratios are
  bounded to 0.0001..64. Tables (~0.78 MiB/engine) and loop endpoint data are prepared
  outside the callback. Finite kernels have limited stopband quality at extreme ratios;
  the alias fixture establishes attenuation at 2:1, not ideal brick-wall filtering.
  Radio retains fixed-rate linear stereo interpolation, with f32 decoded precision.
- **Transitions:** gain uses a 5 ms exponential time constant, pitch 15 ms, filter
  coefficient 120 ms (per sample), bus gain 20 ms. These are independent of callback
  partition and device rate. Start/stop fades are finite 3 ms ramps; one-shots fade their
  last 3 ms in output time. Logical `stop` is immediate to gameplay, but rendering retains
  its brief tail. Radio close cancels decoding and fades the last decoded frame.
- **Loops:** taps wrap without silent frames, including very short and fractional-pitch
  loops. A seam exceeding twice the adjacent sample slope and 2% of full scale is
  corrected over the last 1 ms (at most a quarter-loop); no samples are inserted/skipped.
  This does not settle the OPEN random start offset question.
- **Device formats/layout:** typed CPAL paths for f32/f64 and signed/unsigned 8/16/32/64
  integer output. Unsupported defaults try an advertised mono/stereo 48/44.1 kHz format;
  otherwise fail explicitly and retry. Hardware rates 8..192 kHz, 1..8 channels. Internal
  processing is stereo; mono receives `(L+R)/2`. Multichannel devices receive front L/R,
  with remaining channels silent, including LFE. There is no inferred surround/HRTF
  placement. Conversion sanitizes nonfinite samples and clamps to the device range.
- **Hardware lifecycle:** `enabled` means logical playback can run, even at startup without
  hardware. `device_state()` distinguishes deliberate offline mode (`None`) from
  `NoDevice/Opening/Open/Lost/Reopening`. The watcher also starts without a device;
  `follow_device` retries failures at most once/second, even if the default name is
  unchanged. Stream build **and play** must succeed before reporting Open. On switching,
  stop the old callback, drain retirements, clear stale commands and rebuild the core
  after selecting its format. Replay surviving voices once, ordered by id, restoring
  listener/bus controls. Loops restart at zero; radio retains its bounded buffer. One-shots
  started during an outage are discarded; those interrupted by loss are also discarded.
  Ordinary default-device switching restarts still-active one-shots. Lost loops retain
  their latest parameters and can be stopped before reconnection.
- **Radio ring:** 524288 stereo frame slots (8 MiB at the Windows slot layout), with
  per-frame sample rate and release/acquire indices. Producer writers serialize only
  among themselves; the audio thread owns one exclusive reader and never takes their
  lock or the status-string lock. Full buffers drop newest frames, counted by
  `dropped_frames()`. The decoder throttles at 8 seconds ahead or fewer than 8192 free
  slots. Initial prebuffer remains 1.5 seconds, increasing by 1 second/stall up to 6;
  for high-rate input it is capped at 75% of ring capacity so it can always refill.
  Underrun is silence, playback position held, followed by rebuffering. The engine
  counts 256-frame render fragments with missing input. Format changes are marked per
  frame, preventing samples of different rates from being interpolated together.
- **WAV:** RIFF/chunk bounds, odd padding, required/duplicate chunks, byte rate, block
  alignment, complete sample frames, channel count, sample rate and extensible GUID,
  valid bits/channel mask are validated. Sources are mono/stereo PCM 8/16/24/32 and
  IEEE float32, including their extensible forms. Nonfinite floats are rejected.
  Preserve public `Clip { samples: Vec<i16>, ... }` and fleet/cache memory. Higher-bit
  input rounds once through f64 instead of discarding low bits; the renderer and radio
  keep f32 processing. A blanket f32 cache doubles existing PCM16 memory with no new
  information and was deliberately avoided.
- **Buses/headroom:** Vehicle 1.0, Ambience 0.8, Passenger 1.0, Announcement 1.0,
  Radio 0.7, Interface 1.0; configurable with `set_bus_gain` (linear 0..2).
  Defaults reserve 6 dB (`0.5`) before the master. The stereo-linked limiter attacks
  immediately at 0.9, releases over 500 ms, and keeps channel ratios linked. Empty
  buffers are valid. These mix gains follow the single legacy clamp and never change
  runtime admission. Existing `VoiceParams` literals/API stay valid; raw parameter
  updates retain the selected bus. File-trigger bus selection survives later runtime
  updates (announcements by default, explicitly selectable); HTML playback uses Interface.
- **Real-time bounds:** 200 mixed clip voices; 512 total voice slots including streams,
  virtualized voices and fades. At capacity, starts are counted/rejected on the game side
  or returned through the bounded reaper. Reaper backpressure retains rejected assets
  and defers additional commands, without callback allocation or late playback. Complete
  finished voices move to the reaper: final clip/ring/reader frees happen on the game
  thread, including after stop/unload. Command drain/rejection/ranking/bus buffers and
  reverb delay lines are prepared before rendering; ranking uses an unstable in-place
  sort with an explicit index tie-break. No new unsafe, dependencies or mutable globals.

### Producer audit

| Producer | Routing / Stage 6 finding |
|---|---|
| Player / AI / LAN vehicles | Vehicle; existing ordered events and legacy split retained |
| Trailer / coupled parts | Vehicle; now receive file events and fire-time snapshots through `update_parts_events` |
| Scenery / lamps | Ambience for regular and file-trigger sounds; existing unloading still calls `stop_all` |
| Weather / wet-road sounds | Rain -> Ambience; vehicle-authored wet-road entries remain Vehicle |
| Footsteps / passenger dialogue | Passenger; raw `VoiceParams` shape preserved |
| Script file announcements | Announcement; snapshot/peak and bus persist on subsequent frames |
| Internet radio | Radio; bounded ring; raw updates retain routing; close has a short fade |
| HTML `omsi.playSound` | Interface through `play_file_direct` |
| Ticket stamper | Player/traffic stamps migrated to `FiredSound::Trigger` in `redraw/world.rs` |
| Offscreen / simulation audio tools | Same façade and player path; second engine has independent bus/device state |

### Phase gate

Stage 6 is ready for local testing: all 81 audio tests pass, the optimized dev-release
application builds and starts at its CLI, offline rendering succeeds and synthetic mixer
CPU/process-memory measurements are recorded in `STAGE6_TESTING.md`. SDK import libraries
are now available after explicit approval to install the Microsoft Build Tools/SDK.
Phase 1 is **not yet accepted**: complete workspace/all-target checking, the production
release profile, physical device switching, representative map/fleet hearing and performance,
and the live OMSI decisions remain pending. The 500-source synthetic probe has an isolated
8.48 ms peak against a 5.33 ms block deadline despite a 3.06 ms mean. Phase 2 / Steam Audio
has not been started. No synthetic measurement is presented as a live map or OMSI comparison.

## 11. Stage 6.1 listening corrections (2026-10-06)

User hearing report: C2 EN 6/BVG and general playback had nearly muted ears, a brief loud
entry before muffling, weak door-closing attenuation, and noisy/faint high-pitched tails
on controls. The following are neoOMSI quality choices, not newly CONFIRMED OMSI rules:

- **Panning:** retain near/far damping, but cap full broadband far-ear attenuation at
  12 dB (amplitude >= 0.251). Quarter-side is now 3 dB rather than 25 dB. Normalize the
  listener basis; interpolate channel gains with a 25 ms sample-time constant. Initialize
  a new/resumed voice to its actual direction so it does not start centred then jump.
- **Own cabin:** non-3D recordings smoothly become centred/local in the cabin. Explicit
  3D recordings retain their distance curve but use 35% stereo width there. A declared
  inside view overrides stale spawn-camera geometry. The geometry blend still applies
  when approaching/leaving a hull from an outside view. Distance blending is transported
  privately in MixParams and is also reflected in ranking/read-back; VoiceParams is stable.
- **Door/body transfer:** own outside-only recordings use the existing 0.25..1.0 transfer
  with Snd_OutsideVol and 450..7200 Hz low-pass shaping. Unknown opening defaults to shut
  for this own-body path. Own inside-only recordings retain full level in the cab and
  become quieter/duller outside (quarter level, 1200 Hz). Both-side/untagged recordings
  remain unchanged. An explicit Snd_OutsideVol curve keeps its authored gain, avoiding
  double attenuation. Transfer follows peak handling; established admission thresholds,
  event ordering and logical viewpoint gating remain unchanged/OPEN.
- **Initial state:** a newly loaded set starts at its current inside/muffled state;
  only subsequent changes use the existing blend. Startup queues the user's volume
  before any vehicle starts playing. These address the transient full-level entry.
- **Resampling:** interpolate the two neighbouring fractional-phase coefficient rows
  instead of rounding phase. This removes the sample-position staircase that can create
  periodic high-frequency modulation. FIR size, cutoff bands and offline determinism
  are retained. The listening report's exact squeak source still requires the user's test.
- **Hall:** a cabin/bodywork hint no longer forces a 22% wet mix and invented room time
  onto every button and loop. Explicit listener/world/trigger-box reverb remains supported.
  A short control fixture verifies zero unrequested tail after its one-shot ends.

All 90 audio tests pass (75 unit, 8 offline, 5 quality, 2 runtime). Door transfer, initial
state, centred cabin playback, bounded/smoothed pan, fractional phase and transient tail
have direct regressions. Legacy global-dependent door fixtures now use explicit inputs
instead of racing each other's OUTSIDE_OPEN changes; no new globals/locks were added.
The optimized 0.2.0-audio-stage6.1 EXE is handed to the user for confirmation of these
subjective fixes. Physical C2 hearing and the open reference comparisons remain pending.
