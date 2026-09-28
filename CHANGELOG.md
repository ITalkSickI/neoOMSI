# Changelog

Every push to `main` is released as `MAJOR.MINOR.COMMIT` (see
[docs/VERSIONING.md](docs/VERSIONING.md)); the downloads are on the
[Releases](https://github.com/turbo-devv/openOMSI/releases) page.

## 0.1.8 - 2026-09-28

### Controls
- Changing a key on the Controls page now takes effect: the page says which driving keys are
  in use, and changing any key switches *Driving keys* to *Custom controls* by itself (before,
  with W A S D the edited keys were silently ignored).
- Keys you bind yourself now beat the ready-made layouts: with W A S D chosen, a D you gave
  to the gearbox is the gearbox, not "steer right". The layouts' extra keys (Z/X/C for the
  indicators, I for the saloon lights) no longer apply with Custom controls (C is OMSI's
  "look ahead" there again).
- **Space** looks ahead again (OMSI's `view_reset_all_directions`); it was swallowed by the
  W A S D layout. Every view keeps its own direction: turning the outside camera (F3) no
  longer turns the driver's head (F1).
- Zoom inside the bus: the mouse wheel, **=** / **-** and a pinch narrow the view in the
  driver's and passenger views, as in OMSI.
- Mouse steering follows Omsi.exe exactly, now including its pedals (throttle from the middle
  of the window to the top edge, brake to the bottom edge, no dead zone). Settings → *Mouse
  steering* makes it more or less sensitive (100 % = OMSI).

### Wheels, pedals, joysticks
- On Windows the game controllers are read through DirectInput, as OMSI reads them: every
  device Windows lists as a game controller (wheels with their makers' drivers included), up
  to 128 buttons, and force feedback on wheels - the centring that grows with the speed, the
  heavy steering of a bus standing still, and the scripts' shaking.
- Buttons are numbered as DirectInput numbers them (they were counted in the order they were
  first pressed), and every button of a device can be given a key: the list stopped at the
  ten that fitted on the page (T16000M).
- *Set up step by step*: turn the wheel to the left, press each pedal - the axes, their
  direction and combined pedals are found by themselves. A connected device nobody has set up
  yet steers with its X axis and says where to set it up.

### Multiplayer
- Hosting no longer stops working after a while: the router's port forwarding was asked for
  two hours and never renewed, and the rendezvous relay was asked every second and refused the
  host after an hour or two. The forwarding is renewed every 20 minutes, the relay is asked
  every few seconds with a growing pause after a refusal, and a Cloudflare tunnel that ends is
  started again.

### Launcher
- Phones: the launcher is drawn at least at the system's text size, the settings stand in one
  column, a finger on a list at its end scrolls the page on, page titles no longer run under
  the tabs.
- The OMSI 2 folder is found when openOMSI was unpacked into it (openOMSI keeps its own content
  in an `openOMSI` folder there), when the path is pasted with quotes, or when `Omsi.exe` or a
  folder inside the game is chosen; a folder that is not a complete OMSI 2 is reported with
  what it lacks.
- Timetable: changes stay while you move between lines and are saved together (*Save all*);
  *New line*; *Repeat* makes a whole day of tours (every *n* minutes up to a last departure).
- Settings → Graphics → *Reflection maps* switches the materials' reflections
  (`[matl_envmap]`) off.

### Sound
- Distance as in OMSI: full volume up to the `[3d]` reference distance, then falling as 1/d
  (DirectSound's law). It fell much faster, so most sounds were far too quiet.
- The bus's own sounds are no longer muffled in the cab unless they are other vehicles':
  interior sounds such as the indicator relay were cut to a quarter and dulled, and only came
  through with a door or window open.
- Footsteps outside are heard through the bodywork from the cab (and the saloon's from the
  street); people in the street sounded as if they walked inside the bus.

### Maps and vehicles
- Matrix displays drawn by scripts (script textures as the LED mask, `\S:n`, e.g. churaPixel/
  Krüger++ matrices) show their dots instead of a fully lit panel: a `[matl_change]` ahead of
  the slot's `[matl]` made it opaque.
- Objects put on a road spline (`[splineAttachement]`), such as an entry point or a stop, are
  found by their id: a Novi Sad start point was "not in the map".
- An entry point whose object comes out on another level than the map recorded (under a
  bridge) starts the bus at the recorded height.

### Game
- The depot file (HOF) follows the date as the map's chrono says: Berlin in 1994 has line 137
  where 1986 had 92 - on every map with chrono depot changes.
- Phone: the pause menu scrolls with the finger; a finger put down to scroll no longer picks
  the line under it.
- With V-sync one frame waits for the screen instead of two: less input delay.

### Builds
- New downloads: Windows ARM64, macOS Intel, Linux ARM64, and the dedicated server for Windows
  (x64, ARM64) and Linux ARM64. The launcher updates itself on all of them.
- The release notes on GitHub list what changed (this changelog) instead of a link to the
  code changes.

## 0.1.7 - 2026-09-28

### Driving physics as the .bus file makes it
- The bus now follows its steering the way OMSI's own physics does: it turns exactly as far
  as its wheels point and only slides when a bend asks more grip than the road has. Before,
  every bus turned at 70 % of what its steering asked, 0.8 s late and drifting sideways -
  the "boat" feeling, and the same for every bus.
- Springs, dampers and their limits act where OMSI takes them (`achse_feder`,
  `achse_daempfer`, `achse_maxforce`, `achse_minwidth`/`achse_maxwidth`), every axle steers
  towards `[rot_pnt_long]`, and body pitch and roll are damped as in OMSI. Each bus feels as
  its author made it: stiffer springs, stronger dampers, a higher centre of gravity all show.
- `cargo run --release -p omsi-sim --example handling -- <file.bus>` prints how a bus
  handles (yaw response, side slip, body roll and how fast it settles).

### Steering
- Mouse steering (O) as in OMSI: the whole window width is the full lock, and above 10 km/h
  the same hand movement turns the wheel less and less (at 50 km/h a fifth as far). No more
  jumps when the cursor passes the middle.
- Phone: the on-screen wheel turns with the finger round it (a third of a turn is the full
  lock). It used to stop at about half a turn and spring back.

### Graphics and world
- Mirrors follow the bus's pitch and roll, use the camera distance from the `.bus` file, and
  only the mirrors in view are redrawn (the radius of `[add_camera_reflexion_2]`).
- The bus's own screens (IBIS, matrix displays, dashboard LCDs) are sharp again in Enhanced
  graphics: FXAA and the glow no longer blur them.
- Trees have the width the map gives them: slim trees such as firs were drawn up to six
  times too wide.
- Map tiles of older editor versions are read correctly (object tilt and strings).

### Updates
- The launcher updates itself from the GitHub releases: when a newer version is out it asks
  at the start, downloads it (checked against GitHub's SHA-256), replaces the program and
  starts again. Mods, content and settings stay as they are.
- On Android the system's installer is used: Update replaces the app and starts it again,
  Cancel leaves it as it was.
- Settings → Updates: look for updates at the start, install without asking, Check now.

### Fixes
- Android: vibration of the on-screen buttons works (the calls never reached the app).
- Android: `openOMSI/env.txt` takes the `OMSI_*` switches a computer takes from its
  environment (for looking into problems).

## 0.1.6 - 2026-09-27
- Phone: the on-screen wheel is drawn cleanly; calmer steering, tilt steering reaches the
  full lock at 45°.
- A version built again updates its release instead of failing.

## 0.1.5 - 2026-09-27
- Lua plugins (`plugins/<name>.lua` or `plugins/<name>/main.lua`) on every platform, next to
  the original DLL plugins: bus variables and triggers, events, timers, saved data, hot
  reload, sandboxed. See [docs/PLUGINS.md](docs/PLUGINS.md).

## 0.1.4 - 2026-09-27
- Android: the launcher and the game on phones and tablets, with on-screen driving controls
  (wheel or tilt, pedals, gearbox, doors, indicators, cab panel, cameras). See
  [docs/ANDROID.md](docs/ANDROID.md).

## 0.1.3 - 2026-09-27
- Small changes.

## 0.1.2 - 2026-09-27
- Website and repository improvements and fixes.

## 0.1.1 - 2026-09-27
- Website and repository improvements and fixes.

## 0.1.0 - 2026-09-27
- First public release as openOMSI: a from-scratch recreation of OMSI 2 in Rust that runs
  every map and mod (an original OMSI 2 is needed). Builds for Windows, macOS and Linux and
  a dedicated server, released automatically on every push.
