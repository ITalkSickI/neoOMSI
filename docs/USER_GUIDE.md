# User guide

How to run openOMSI, drive, use the launcher, install mods and play over LAN. For building
from source see [BUILDING.md](BUILDING.md).

> openOMSI runs on the content of an **original OMSI 2 installation**. Without one the game does not start.

## Checking an installation

```bash
cargo run --release -p omsi-check -- "/path/to/OMSI 2"
```

This loads every content file in the install with the new loaders and reports what failed.

## Running

```bash
openomsi --root "/path/to/OMSI 2"
```

`--root` is only needed once: the path is remembered, so afterwards the program can be
started with no arguments at all. Without it the installation is looked for in `$OMSI_ROOT`,
next to the program (an `OMSI 2 Original` or `OMSI 2` folder beside openOMSI) and in the
usual Steam locations.

Started without arguments the program opens the launcher (see below). `--menu` shows the
in-game start menu instead, which asks for map, vehicle, time, traffic, passengers, the
timetable, the weather and the date (arrow keys change values, Enter starts, Esc quits).
Everything can also be given on the command line, which then skips both:

| Flag | Meaning |
| --- | --- |
| `--map maps/Grundorf/global.cfg` | map to load |
| `--weather Weather/Schmuddelwetter.owt --date 1989-01-15` | weather, and the date that decides the season |
| `--bus Vehicles/MAN_SD200/MAN_SD80.bus` | player vehicle (`--paint name`, `--hof name`) |
| `--entry N` / `--spawn x,y,heading` | where the vehicle starts |
| `--time HH:MM --date YYYY-MM-DD --weather Weather/x.owt` | time, date, weather |
| `--traffic N --schedule --line 76 --tour 1 --passengers` | AI cars, timetable buses, the player's tour, people at the stops |
| `--radius N` / `--all` | tiles around the start / the whole map |
| `--view-distance M` | how far around the camera the window keeps tiles loaded (m, 1200 by default) |
| `--content-zip mod.zip` | read an archive in place as a content root (repeatable; `OMSI_CONTENT_ZIP` does the same) |
| `--offscreen out.png --cam x,y,z,yaw,pitch --drive secs` | render one frame to a file |
| `--snapshots 7,9,11` / `--follow auto\|bus\|moving\|type:X` | more pictures during a `--drive` run, camera behind an AI vehicle |
| `--riders N --refuel --wash --repair --dirt 0..1` | passengers already aboard, the depot services, how dirty the bus starts |
| `--driver Drivers/OMSI-Fan.odr` | the driver's personnel file; the run is added to it |
| `--drive-keys wasd` | let W/A/S/D drive instead of the arrow keys |
| `--autostart` | put the bus into service before the run (as Shift+U does) |
| `--click x,y[,dx,dy]` | press (and drag) the cockpit switch at that pixel, offscreen |
| `--season winter` / `--situation x.osn` / `--physics simple` | season override, a saved situation, the kinematic dynamics instead of the rigid body |
| `--enhanced` / `--export-glb bus.glb` | the physically based renderer; write the bus as glTF (the launcher's preview) and quit |
| `--launcher` / `--menu` / `--no-menu` | open the launcher (the default without arguments), the in-game menu, or neither |

Keys in the window: **W** throttle, **S** brake, **A**/**D** steering - the arrow keys do the
same - and every vehicle key of `Inputs/keyboard.cfg` works as it does in OMSI: throttle
Shift+Num 8, brake Shift+Num 2, steering Shift+Num 4/6, **E** battery and ignition, **M**
starter, **N**/**R** the automatic, **.** the parking brake. W, S and D are OMSI's wiper,
viewpoint and **D of the automatic gearbox**, so hold shift for those: **Shift+D** selects D.
`--drive-keys arrows` leaves W/A/S/D to OMSI entirely.

To start a stock SD200: **E** (battery - it puts the ignition key in as well), **M** held for a
second (starter), **Shift+D** (drive), **.** (parking brake off), then throttle. **Shift+U** does the
whole start-up by itself (main switch, ignition, starter, gearbox to neutral); `--autostart`
is the same thing for an offscreen run.

Left-click a cockpit switch to operate it, hold the button and move the mouse to turn a knob,
or roll the mouse wheel over it (that is the `<event>_drag` OMSI fires); the name of the switch
under the cursor is shown in the HUD.
Right-drag the mouse to look around in any view (the head turns inside, the camera swings
around the bus outside), I/J/K/L does the same from the keyboard, the wheel zooms, Home
recentres. F1-F4 driver / passenger / outside / map (free) camera, F5-F8 the destination
sign and roller blind keys as in OMSI, Alt+S quick save, F9 write the run into the personnel
file, WASD+QE in the free camera, left click on cockpit elements, **V** the chat line in a
LAN session. Esc opens the game menu: drive the next placed vehicle, place any vehicle of
the installation in front of the camera (or beside the bus), couple what stands close behind
the bus and uncouple it again, save the situation or load the quicksave, the next weather, the clock an hour on or back, refuel and wash (only at a
petrol station, as in OMSI), repair (the team needs the map's travel time when the bus stands
in no depot yard), screenshot, timetable, the object editor (below), quit. Shift+Home is the ticket desk camera, and the
change keys of keyboard.cfg hand out or take back the change. The HUD
shows time, speed, line, next stop, delay and what the workshop just did (and why the bus
stands: the parking brake, low air pressure, a line the date's chrono takes off), and the
controls for the first seconds.

## The launcher

The launcher is the game's own window (`crates/omsi-app/src/launcher`): `omsi` started
without arguments (or with `--launcher`) opens it. It is drawn with wgpu - no web engine -
flat and dark (neutral greys, one amber accent), every control custom (sliders, switches,
dropdowns, a calendar, a time picker, text fields, segmented buttons), Material Symbols
icons and Roboto (`crates/omsi-ui`). The Drive page shows the chosen bus in a card, as a
picture **drawn by the game's renderer** - its model, paint, materials, reflections and
shadows exactly as in the game, under the light of the chosen time and weather - drawn
again only when something changes; drag on it to turn the bus, scroll to zoom. Its pages:

* **Drive** - four steps: the bus (search, liveries, depot file), the route (map, start
  point, line and tour - the lines that run on the chosen date), time and weather (time,
  date, season, traffic, passengers, timetable buses, autostart, *LAN play: host / join*,
  the weather presets that suit the season), and the roadbook with the IBIS codes; the
  summary and **Start the duty** bottom right.
* **Profile** - hours, experience and level, from OMSI's own `.odr` personnel files plus
  the session summaries the game writes to `~/.openomsi/sessions`.
* **Settings** - everything in `settings.cfg` below, saved as it changes; keys the page
  does not manage are kept as they are.
* **Controls** - `Inputs/keyboard.cfg`: click a key, press the new one; clashes are red.
* **Sessions** - every game started from the launcher, with its log, a **Stop** that lets
  it save its run (SIGTERM, up to 8 s, and only a stuck game is killed) and, for a LAN
  session, the code to copy, who is playing and the chat.
* **Mods** - installing mods and archives (see *Mods and the content folder*); a folder or
  .zip dropped on the window is installed.
* **Setup** - where the original installation and the game binary are.

```bash
scripts/build-macos.sh   # or build-windows.cmd / build-linux.sh: the game opens the launcher
```

Without a person at it: `OMSI_LAUNCHER_PAGE=drive:2` opens a page (and a Drive step),
`OMSI_LAUNCHER_SHOT=secs:file.png` writes a picture, `OMSI_LAUNCHER_EXIT=secs` closes it,
`OMSI_LAUNCHER_INPUT="t=2 click 412,60; t=3 type 76; t=4 key Enter; t=5 shot a.png"` works
it (logical pixels). The data side is `crates/omsi-launcher-core`:
`openomsi-launcher --cli lines '{"map":"maps/Grundorf/global.cfg"}'` runs any of its commands
from a terminal - `config`, `maps`, `vehicles`, `weather`, `lines`, `ibis`, `profiles`,
`profile`, `mods`, `modinfo`, `install`, `instances`, `stop`, `log`, `join`, `settings`,
`save_settings`, `keybindings`, `save_keybindings`, `preview`, `args`, `launch`. `lines`
takes a `"date":"YYYY-MM-DD"` as well: the chrono folders active that day add and remove
lines, as in the game, whose default date is 1989-05-30.

## Settings, enhanced graphics, the navigator

`~/.openomsi/settings.cfg` (written by the launcher's settings page, or by hand) holds
`msaa` (1/2/4; a count the GPU cannot do falls back to the next lower one), `anisotropy`
(1..16), `ssao`, `shadows`, `shadow_size`, `navigator`, `navigator_opacity`,
`navigator_corner` (`bottom-left` default, `bottom-right`, `top-left`, `top-right`),
`boarding`, `detail_textures`, `exact_fare`, `enhanced`, `fullscreen`, `vsync`, `volume`
and `drive_keys`, plus `render_scale` (`auto` or a fraction: the picture is drawn smaller
and upscaled), `post_aa` (`fxaa`, the enhanced renderer's, or `off`), `view_distance` (m,
how far the tiles are kept loaded), `texture_memory` (MB - OMSI's `texmemlimit` is read
under that name too; an eighth of the machine's memory when unset), `texture_compression`
(BC1-BC3 on the GPU, on by default) and `language` (`ENG`, `DEU`, `FRA`: the language the
HUD names cockpit switches in). The file also carries a `version`; older files that say
`boarding=pay` because that was the launcher's old default are read as `auto`.

`drive_keys` is a control preset: `simple` (W/S/A/D and the arrow keys drive; the default),
`wasd`, `arrows`, or `omsi` - only the original layout of `Inputs/keyboard.cfg` (Shift +
numpad), nothing added. **T** sells the ticket a passenger asks for on a bus without a
ticket printer (the original's `ticket_give` key).

`boarding` is how passengers board: `auto` (default) - they walk to the standing place the
cabin's `[ticket_sale]` names, turn to the driver, put the money down, take their ticket by
themselves after a moment and walk on; `pay` - they wait for the driver to sell the ticket
(the bus's printer, or **T**) and give up after 25 s; `walk` - straight into the saloon,
no cash desk (flat fare / ticket machines). People keep a body's width apart outside.
`exact_fare=0` makes them overpay so that change is due. Rain and snow stay outside the
player's bus (its `[boundingbox]`), and heavy rain darkens the day enough for the saloon
lights to matter.

`detail_textures` lays procedural (fractal) grain over the ground and the roads up close,
in vanilla and enhanced alike. `enhanced=1` (or `--enhanced`) switches to its own
physically based renderer: high-range lighting with energy-conserving diffuse and GGX
reflections (roughness from `[matl_envmap]`), a computed sky (Rayleigh/Mie scattering,
lit cumulus) that also lights the scene, contact-hardening sun shadows, aerial perspective
and height fog, automatic exposure, a glow only real highlights produce and the PBR
Neutral tone curve with FXAA (`post_aa`); no light shafts, vignette or grading. The
vanilla look stays the default. The navigator (`crates/omsi-app/src/navigator.rs`) sits in a corner of the screen (lower
left by default), after the Route Advisor of Euro Truck Simulator 2: small, dark and half
transparent, a tilted 3D map that turns with the bus and zooms out with speed - the roads
of the lane network, the trip's route with arrows along it, coloured stretch by stretch by
how busy the road is (blue empty, green light, yellow busy, red heavy, dark red jammed; it
changes as the traffic does), the stops ahead, the other vehicles as blue dots, the next
turn with its distance and the street it turns into, and the street the bus is on. It
routes on the whole map's road network, read from the tile files in the background at the
start, so the way shows however far the route or the next stop is from the loaded tiles.
From the depot it leads at once to the next stop (never past it onto a later
part of the route); leave the route and it is recalculated after two seconds (Dijkstra
over the lanes, back onto the route ahead). A duty begins with the first trip of the tour
whose first stop the bus can still reach before it leaves - not with the trip under way
at the start time, which put the driver late and halfway along the line. Above the map the speed with the limit, the line and the time;
below it the next stop, its distance, the time to it, its planned time and whether the bus
is early or late - in English, German, French or Russian (`language`). A click on the navigator (or **Shift+M**) opens the city map: a large window over the game
with the whole map from above - every road with its street name (read from the map's
street name signs), the trip's route by traffic with arrows, its stops with their times,
the bus and the traffic; drag to move, the wheel zooms, the buttons centre on the bus and
zoom, Escape or a click outside closes it. **Shift+N** cycles
map → map with the schedule of the next stops → off (N alone is the gearbox's neutral);
`OMSI_DEBUG_NAV=1` logs it.
**Z / X / C** are the indicators. **Shift + 1**, **Shift + 2**, … open or close a door, front
to back: a bus like the SD200/SD202/EN92 with one two-leaf front door and a combined
aft/stop-brake-release door answers to Shift+1/2/3, a low-floor mod with three or four
independent doors (the O530 Facelift) to Shift+1 through Shift+4/5 - whatever
`bus_doorfront<n>` triggers the bus's own script defines, `bus_dooraft` last (the HUD's
control reminder says how many).

## Mods and the content folder

The folder of the game binary (`dist/<platform>` in a build; beside `openOMSI.app` on macOS) is laid out like an OMSI 2
installation - `Vehicles`, `maps`, `Sceneryobjects`, `Splines`, `Texture`, `Fonts`,
`Plugins`, `TicketPacks`, `Drivers`, `Weather`, … - and is searched *before* the original
folder (`omsi_cfg::content_roots`): whatever a mod puts there is found exactly as if it had
been copied into OMSI 2, and a file of the same name replaces the stock one. The original
installation is never written to. `OMSI_CONTENT=/some/dir` moves the content folder.

Installing a mod: the launcher's **Mods** page opens the system's folder / file picker
(Finder, Explorer, GTK) for a mod folder or a `.zip` and sorts it
into place (OMSI-style folders anywhere inside are merged; a lone bus, map, object or
spline folder is recognised by its `.bus` / `global.cfg` / `.sco` / `.sli` files and put
under the right folder), or drop it into `Mods/` next to the binary and open the page.
`openomsi-launcher --cli install '{"path":"/path/to/mod.zip"}'` and `--cli mods` do the same
from a shell. An installation is a background job: the archive's table of contents becomes
a plan, the disk is checked for room, everything is unpacked into a staging folder on the
content volume and moved into place in one step, and it can be cancelled and cleaned up at
any point. A repaint for a bus that is not installed is kept aside and installed when the
bus arrives.

Archives can also be **used in place**: a `.zip` laid out like OMSI 2 is put into the
content folder's `Archives/` (hard-linked when it is on the same disk, moved from the
`Mods/` inbox, else copied after a free-space check) and read by the game without
unpacking (`omsi_cfg::vfs` mounts every archive there, as well as `--content-zip` and
`OMSI_CONTENT_ZIP`). The Mods page offers it ("use the archive in place"), and its default
unpacks what fits on the disk and uses an archive in place when its unpacked size does not;
`--cli install '{"path":…,"mode":"inplace"}'` (or `extract` / `auto`) and
`--cli modinfo '{"path":…}'` do the same from a shell. The launcher's lists see the maps
and buses inside the archives.

## Season and weather

The launcher's Departure card has a **Season** choice (spring / summer / autumn / winter,
or by the date as in the original). Choosing one moves the date into that season, so the
timetable and holidays follow, passes `--season` to the game (which picks the map's
seasonal texture folder), and the weather list only offers what fits: snowfall and frost
only in winter, no cold presets in summer.

Weather presets (`Weather/*.owt`) change the light: overcast takes the sun away, rain and
fog thicken the air, a snow preset puts any map into its winter textures with snow cover.

## Performance

`OMSI_PROFILE=1 … --exit-after N` prints the frame split (render, mirrors, traffic, people,
scripted objects, LAN), counts frames over 50 ms and logs the GPU and CPU memory by kind
every ten seconds; the per-draw buffers are updated in contiguous runs and appended to as
cars and people spawn (no full rebuild), everything behind the fog is culled, culling runs
on all cores, the main pass is recorded as render bundles on helper threads, and the AI
scripts run in parallel. Spandau with traffic, passengers and a storm: 14 → 69 fps on an
M4, no frame over 50 ms after start-up.

Big maps are kept within memory by compressed textures, a texture budget
(`texture_memory`), a timetable fleet read ahead and trimmed again, and tiles that give
everything back when they unload: Ahlheim V5 at its main station with traffic, passengers
and the timetable peaks at 1.93 GB instead of 8.75 GB (see `docs/ARCHITECTURE.md`,
*Memory*). The game's log is `~/.openomsi/game.log` (the launcher's `launcher.log`
beside it), and the first line of both is the build they were made from.

## Object editor

A small part of what OMSI's map editor does, inside the game: **Ctrl+Shift+E** (or *Object
editor* in the game menu) turns it on. **Enter** picks the scenery object nearest the middle
of the view (a magenta glow marks it), **Tab** the next nearest; **I/K/J/L** move it forward,
back, left and right as the camera faces, **U/O** lower and raise it, **N/M** turn it (half a
metre and five degrees a press, a tenth with Shift); **Delete** deletes it (again: back),
**Backspace** undoes everything done to it, **Ctrl+S** saves and **Esc** leaves the editor.
Saving writes each changed tile as a copy into the content folder's map folder
(`<content>/maps/<map>/tile_x_y.map`), which the game reads before the installation - the
original map is never written; delete the copy to have the original back. Only a tile's own
`[object]` records can be edited: splines, the ground, spline rows, new objects and the
timetable are not part of it.

## Debug and test switches

Environment variables, all off unless set. The useful ones:

| Variable | What it does |
| --- | --- |
| `OMSI_PROFILE=1`, `OMSI_GPU_TIMERS`, `OMSI_DEBUG_DRAWS` | frame split and memory, per-pass GPU times, draw and changed-instance counts |
| `OMSI_SEED=n` | repeat a session: the scripts' `random` is seeded per session (the log says which seed) |
| `OMSI_INPUT="t=3 move x,y; t=3.2 press; t=4 key F3; …"` | drive the real window handlers (mouse, keys, `look`/`turn`) from a script |
| `OMSI_CHURN=x,y` | offscreen check of tile streaming: load the tiles around that far point, unload the start area, unload the far tiles and load the start area again, so the picture is drawn from recycled GPU slots |
| `OMSI_HIDE_WINDOW=from,to` | pretend the window is hidden for those seconds |
| `OMSI_TEXTURE_MEMORY=MB`, `OMSI_BUDGET_FROM=x,y[,MB]` | the texture budget, and meeting it from somewhere else first |
| `OMSI_FLEET_IDLE=s`, `OMSI_FLEET_AHEAD=min` | how long an unused vehicle set is kept, how far ahead the fleet is read |
| `OMSI_NO_BC=1`, `OMSI_NO_TEXCOMPRESS=1`, `OMSI_KEEP_ALLOCATOR=1` | textures as RGBA, no compression of loose pictures, no allocator restart |
| `OMSI_NO_SHADOWS`, `OMSI_NO_CORONAS`, `OMSI_NO_ENVMAP`, `OMSI_NO_BUMP`, `OMSI_NO_CULL`, `OMSI_ENV_PHOTO=0` | leave one part of the picture out for an A/B |
| `OMSI_DEBUG_ENHANCED`, `OMSI_DEBUG_SKY`, `OMSI_DEBUG_EXPOSURE`, `OMSI_METER=…` | the enhanced renderer's lamps, sky, adaptation and metering |
| `OMSI_DEBUG_TRAFFIC`, `OMSI_DEBUG_PAX`, `OMSI_DEBUG_PHYSICS`, `OMSI_DEBUG_LAN`, `OMSI_DEBUG_IBIS`, `OMSI_DEBUG_VARS=a,b` | why a car, a passenger, a wheel, a peer, an IBIS or a script variable does what it does |
| `OMSI_CHECK_ROADS=1`, `OMSI_ROAD_PHOTO=1`, `OMSI_CHECK_ENTRIES=1` | walk the lanes as a bus wheel, photograph the carriageway from above, check every entry point |
| `OMSI_CONTENT=/dir`, `OMSI_CONTENT_ZIP=a.zip:b.zip`, `OMSI_ROOT=/dir` | where the content folder, the archives and the original installation are |
| `OMSI_DEBUG_CONES`, `OMSI_NO_LIGHT_MAP`, `OMSI_DEBUG_LIGHT_GRID` | the lamps' fog cones and halos, the tiles' night light maps left out, lights a full grid cell leaves out |
| `OMSI_PARKED_PULL_OUT=p` | the chance per population pass (about 2 s) that a parked car drives off (0.035 by default), with a log of why one does not |
| `OMSI_NO_BRIDGE=1` | a LAN host leaves the internet alone (no UPnP port forward, no address posting) - for tests |

`OMSI_MUTE`, `OMSI_LAN_AUDIO` and `OMSI_LAN_SAY` are the LAN tests' switches; the rest are
listed where they are read (`grep -r OMSI_ crates`).

## LAN play

`--lan-host [port]` hosts a session (UDP, port 27015 by default), `--lan-join <where>`
joins one and `--lan-name` is your name. The launcher's Drive page has the same as *LAN
play: host / join*.

A host prints a **session code** - `OMSI-7Q4K-2M9X-HD3P-R8TZ-KC5W-NB6E`, a base-32
alphabet without look-alike characters, the scrambled session id first and the host's
addresses hidden under a mask drawn from it, so two codes of one host share nothing. The
code carries up to three addresses of the host, a VPN's first: Hamachi (25.x.x.x), Radmin
VPN (26.x.x.x), ZeroTier, Tailscale (100.64.0.0/10), then the LAN's - never the loopback,
a 169.254 address or a bridge of virtual machines. The joining game says hello to all of
them at once and takes the one that answers; with no answer within 10 s it gives up and
says why that may be (not the same network, a firewall). The launcher's Sessions page and
the HUD list the same addresses with the network they belong to, for joining by hand.
`--lan-join` takes that code, an `ip`, `ip:port`, a host name, a bare port (a host on this
machine) or `auto` (find a host on the local network by broadcast). Over the internet both
players need the same VPN network; the host's firewall must let the game receive UDP on
port 27015 (Windows counts a Hamachi network as public).

Everybody sends the state of their own bus up to twenty times a second (five while nothing
changes, about 60 bytes: a bit-packed datagram with the pose, pedals, lights, indicators,
doors, wheel travel, the rear sections of an articulated bus and the vehicle's own lamp,
switch and sound variables); everything else is text. The host relays and owns the world:
a joining player takes its date, time, weather and season, and its clock keeps everybody
in step. The other players' buses run their own AI scripts with the sender's inputs, are
drawn and heard where they stand, and are obstacles for the AI traffic like your own bus -
as long as that bus type is installed locally, otherwise your own type stands in for it.
**V** opens the chat line (Enter sends, Esc drops it); joining and leaving are announced
there. The host checks everything it takes in and limits how much a player may send.

**One world.** The host simulates the AI traffic, the timetable buses, the people on the
pavements and at the stops, the riders of the timetable buses and the traffic lights for
everybody, around every player (it loads the ground and fills the streets around the
others too). A client simulates none of that: it draws the host's world, 160 ms in the
past so that it glides between the host's frames (`crates/omsi-app/src/lan_world.rs`,
`crates/omsi-net/src/world.rs`). Only its own bus and the passengers who board it are its
own: when its bus stands at a stop with a door open, it asks the host for the people
waiting there; the host hands over those still waiting and keeps those who meanwhile went
for another bus, so nobody is ever on two buses. The people walking up to a client's bus
and those getting off are sent back up, so the host sees them too. Money and the timetable
of your own duty stay local - but the host's timetable leaves the tour each player drives to
that player (its AI bus goes, and comes back from the next departure when the player leaves),
and everybody sees the passengers in everybody's bus: the riders of a player's bus travel
with the world frames and sit in that bus in every other game (protocol 5). `crates/omsi-net` is the transport, the same on every
platform.

Windows and Linux: nothing in the code is macOS-specific (wgpu, winit, cpal, std UDP);
paths are resolved case-insensitively so Windows-style `\` references in mods work
everywhere; settings live under `$HOME` / `%USERPROFILE%`. The same `cargo build --release`
produces `openomsi.exe` / `openomsi`, and the scripts in `scripts/` build the launcher tools alongside it. Stopping a game from the launcher uses `WM_CLOSE` on Windows where it
sends SIGTERM elsewhere.
