# HTML textures: the page API

An `[htmltexture]` in a `model.cfg` draws a small HTML page onto a mesh (IBIS displays, cab
screens, touch panels). openOMSI runs the page with its own engine, no browser needed; what it
supports is listed at the top of `../crates/omsi-sim/src/htmlengine/mod.rs`. Examples:
`docs/examples/htmltexture/` (`ibis.html`, `controls.html`, `dashboard.html`).

A page talks to the vehicle through `window.omsi`.

## Reading the vehicle

```js
window.omsi = window.omsi || {};
window.omsi.update = function (d) {
    d.num, d.str      // script variables that changed since the last call (all on the first)
    d.vehicle         // the normalised snapshot, same object as omsi.vehicle
    d.vars            // every variable so far, same object as omsi.vars
};
```

| | |
| --- | --- |
| `omsi.vehicle` | normalised state with **fixed names on every bus** (below). Also readable from timers. |
| `omsi.vars.num`, `omsi.vars.str` | latest value of every variable of the bus's own variable list, keyed by lower-case name |
| `omsi.getVar(name)` | one variable in any letter case: a number, else text, else `undefined` |
| `omsi.apiVersion` | `1` |

OMSI buses name their variables as they like, so `omsi.vehicle` is built from the conventions
the game itself uses. What a bus does not have is `null` (numbers) or `false` (switches), so
`omsi.vehicle.engine.rpm` never throws. Numbers are rounded (speed to 0.1, rpm to 1 ...) so a
value that only jitters in the last digit does not redraw the page.

Bus-specific things (a mod's own switch, a battery voltage) are not standardised by OMSI: read
them with `omsi.getVar("their_variable_name")`.

## Time, date, locale

| | |
| --- | --- |
| `omsi.time.hour`, `.minute`, `.second` | simulation clock (numbers) |
| `omsi.time.asString` | `HH:MM:SS` |
| `omsi.date.day`, `.month`, `.year` | simulation date (numbers) |
| `omsi.date.asString` | `DD.MM.YYYY`, `MM/DD/YYYY` when `locale` is `en` |
| `omsi.locale` | interface language, ISO 639-1 (`en`, `de`, ...) |

They are set before `omsi.update` runs and change with the simulation clock (once per second).

## `omsi.vehicle` (version 1)

| path | meaning |
| --- | --- |
| `info.number`, `.ident`, `.yard`, `.route`, `.nextStop` | text variables of the bus |
| `motion.speedKmh`, `.heading`, `.pitch`, `.bank`, `.steeringDeg`, `.x`, `.y`, `.z`, `.odometerKm` | movement and place |
| `engine.running`, `.rpm`, `.throttle`, `.brake`, `.clutch`, `.gear`, `.tankContent` | drive train and pedals |
| `electrics.on`, `.busbarMain`, `.busbarAvailable`, `.failure` | on-board network |
| `battery.on` | battery switch, `null` when the bus has none |
| `doors.count`, `.anyOpen` | number of door leaves (`door_0`, `door_1` ...) and whether any is open |
| `doors.list[i].number`, `.open` (0..1), `.isOpen` | `list[0]` is door 1 |
| `passengers.onboard` | people aboard |
| `passengers.entries[i]`, `.exits[i]` | `number`, `open`, `requested` (`PAX_Entry/Exit<n>_Open/_Req`) |
| `lights.headlights` | 0 off, 1 parking, 2 dipped, 3 main beam |
| `lights.brake`, `.reverse`, `.fog`, `.interior` | lamps (`interior` 0..1) |
| `lights.indicator` | 0 off, 1 left, 2 right, 3 hazard; `.indicatorLeft`, `.indicatorRight` are the lamps, `.hazard` |
| `brakes.parking`, `.stop`, `.kneeling` | switches |
| `wipers.running` | windscreen wipers |
| `cabin.temperature` | cabin air, °C |
| `condition.dirt`, `.crashes`, `.lastImpactKJ`, `.streetCondition` | wear and road |
| `train.trailers` | coupled vehicles behind this one |

`doors.anyOpen` follows what the passengers are told is open (`PAX_*_Open`) where the bus has
those variables, and the door leaves (`door_<n>`) otherwise; some mod buses use `door_<n>` for
other things.

## Acting on the vehicle

| | |
| --- | --- |
| `omsi.setVar(name, value)` | write a script variable |
| `omsi.trigger(name)` | press a trigger of the vehicle's scripts, as a button in the cab does |

## For developers

The snapshot is built by `omsi_sim::vehicle_api` (`snapshot`, unit-tested without any game
content) and handed to the backend through `HtmlRenderer::set_vehicle`, which a backend may
ignore. To add a signal: add it to `vehicle_api::snapshot`, add a test next to the others, and
list it in the table above and in the module docs. Existing names stay stable within an API
version.
