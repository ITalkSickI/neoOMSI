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
| trigger volume read at fire | `[volcurve]` on a variable the script changes after firing | volume is read at the moment the trigger fires, not at frame end | CONFIRMED | `SoundSet::update_fired` `at_fire`; fixture `trigger.cfg`; `runtime::conditions` test |
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
| gain clamp | volume factor over 1 | plays at 1 (DirectSound has no gain over 0 dB) | CONFIRMED | `runtime::conditions::volume` clamp; MB 412D case |
| loop pitch silence | pitch variable near 0 | below DirectSound's 100 Hz minimum the buffer is silent | CONFIRMED | `runtime::conditions::pitch_of` |

## 3. Viewpoint, inside/outside and `Snd_OutsideVol`

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| `[viewpoint]` gating | entry tagged inside/outside, listener on the other side | **open question**: in the current `update` path the entry is *not* cut by the listener's side (only `report` uses the listener view mask). The intended behavior is described in the `view_mask` comment. | OPEN | `SoundSet::update_fired` uses `volume_side`, which builds the view from the entry's own bits; `report` uses `placement::view_mask` |
| AI-only entry | `[viewpoint]` bit 4 on a non-AI vehicle | not heard | CONFIRMED | `volume_side`: entry view `(viewpoint & 3) | ai<<2`; `runtime::conditions::volume` |
| `Snd_OutsideVol` through-path | own bus, outside-only entry (`[viewpoint] 5`), listener in cab, variable set | played at the variable's value when over 0.01, else silent | CONFIRMED (when the listener view is supplied) | `runtime::conditions::volume`; TSound update 0x750340; `set_outside_open` |
| `Snd_OutsideVol` scaling | doors/window open | "shut" keeps a quarter; open lets it through (`sound_volume.osc`) | PARTIAL | helper exists (`outside_gain`/`lowpass_of`, test-only) but is not wired into the update path |

## 4. Ordering and repeated events

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| repeat in one frame | a trigger fired twice in one script frame | current code matches the trigger once per frame per entry; the second occurrence does not restart it again within the same update | PARTIAL | `SoundSet::update_fired` uses `find`; the per-host `fired_triggers` is a `Vec` and can hold duplicates |
| trigger order across subsystems | player + AI + scenery triggers in one frame | order is fixed only by the redraw phase order (traffic -> player -> LAN -> scenery); there is no merged, sequenced event stream | OPEN | `app_events/redraw/mod.rs`; plan section 4 |
| normal vs file trigger ordering | `(T.…)` and `(T.F.…)` in one frame | the two are kept in separate lists; relative order is not modeled | OPEN | `omsi-sim` `fired_triggers` vs `fired_file_triggers` |

## 5. Start phase, restart and end

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| random loop start | loop start position | **open question**: the reference can start a loop at a random buffer offset; the current mixer always starts voices at position 0 (except loop wrap) | OPEN | plan section 1; `AudioEngine::play` starts `pos = 0.0` |
| one-shot restart | trigger fires while the entry's one-shot still plays | stop the old voice and start the entry again, unless `[onlyone]` keeps the running one | CONFIRMED | `SoundSet::update_fired` one-shot branch |
| voice end | non-looping clip reaches its end | voice is removed; a looping clip wraps with no gap | CONFIRMED | `mixer` `Shared::render`; `a_loop_has_no_gap_where_it_turns` test |
| distance | `[3d]` range and distance | full up to `range`, then inverse-distance (1/d); applies once | CONFIRMED | `mixer::distance_gain` |
| Doppler | moving `[3d]` entry | closer = higher, away = lower; disabled for the listener's own vehicle and by `OMSI_DOPPLER` | CONFIRMED | `mixer::apply_params` |

## 6. Loading, `[checkloading]`, `[onlyone]`, missing files

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| missing file | file cannot be read | the entry is skipped and warned about once per file | CONFIRMED | `SoundSet::warn_missing_once`, `read_clip` |
| `[checkloading]` | flag on an entry | **not evaluated at runtime** | OPEN | parsed in `omsi-vehicle/src/sound.rs`; no consumer |
| `[onlyone]` | flag on a trigger entry | **scope open**: current code only checks the entry's own still-playing voice, not a file-wide or central registry | PARTIAL | `SoundSet::update_fired` `s.def.only_one && engine.is_playing(id)` |
| reload after unload | tile/vehicle unloaded then loaded again | voices are stopped on unload; clips are re-read in the background | PARTIAL | `SoundSet::stop_all`, `AudioEngine::clips_ready`; logical playback state and asset cache are not yet separated |

## 7. Packs and budgets

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| pack ordering | several sound packs in earshot | the reference sorts packs by normalized distance, then budgets | OPEN | plan section 1; current mixer ranks individual voices instead |
| pack budget | many vehicles | the current mixer keeps `[important]` voices first, then the loudest, up to `MAX_VOICES` (200); skipped voices still advance in time | PARTIAL | `mixer::Shared::render`; `MAX_VOICES` |
| later-audible voice | a voice that was skipped becomes the loudest | it is mixed again from its advanced position | CONFIRMED | `skip_clip` advances the position; ranking is recomputed each block |

## 8. Pause, time base and ownership

| Case | Input | Expected | Status | Evidence |
|---|---|---|---|---|
| pause | game paused | master is 0; the mixer keeps running; time base unchanged | PARTIAL | `redraw/player.rs` sets `master = 0` when paused; `Instant` is not replaced by simulation time |
| player vehicle | own bus | 3D sounds pan and fade, but frame timing must not create Doppler pitch shifts | CONFIRMED | `SoundSet.listener_vehicle`, `doppler = !listener_vehicle` |
| AI / other players | exterior sound set | non-3D entries are placed at the vehicle and fade with distance | CONFIRMED | `SoundSet::new_exterior`, `placement::place` |
| trailers / coupled sets | coupled parts | each part's set updates with the leading vehicle's variables and triggers, at the part's transform | CONFIRMED | `SoundSet::update_parts`, `add_part` |

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
   voice resume?
8. What is the time base under pause, and are loop/sound ages frozen or advanced?

Once answered, each status above is updated to `CONFIRMED` with the comparison that
settled it, and a fixture is added under `crates/omsi-audio/tests/fixtures/soundcfg/`.
