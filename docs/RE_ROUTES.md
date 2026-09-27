# How Omsi.exe runs routes (reverse-engineered)

What the original does with timetables, chrono scenarios, `car_use`, hof files and IBIS,
read from Omsi.exe with `tools/omsi-re` (addresses are Omsi.exe 2.3.004 virtual addresses;
`omsi-re func <addr>` shows each). Our code cites these addresses where it follows them.

## Chrono scenarios

Loader `sub_5a6d8c` (called by the map loader `sub_785f98` after `global.cfg`):

* Finds every `Chrono.cfg` under `<map>/Chrono` with `FindFilesRecursive` (`sub_7f4d24`):
  depth first, a folder's subfolders before its own files, in the order Windows lists names
  (case-insensitive). The index in that list is the scenario's number; later ones override.
* `sub_5a6ed0` parses one file into a 52-byte record (`chrono.f_10[i]`):
  `[name]`, `[description]` … `[end]`, `[startdate]` (flag bit 1, day at +0x1c),
  `[enddate]` (flag bit 2, day at +0x20), `[ticketpack]`, `[moneysystem]`,
  `[deactivate_lines]` = **a count, then that many line names** (`StrToInt`, then `ReadLn`
  count times).
* Dates are `YYYYMMDD`, converted to day numbers (`sub_7f4620`).
* `sub_5a6b3c(i, day)`: scenario *i* is in force when it has at least one date **and**
  `day >= start` (if given) **and** `day <= end` (if given) — both ends inclusive; a
  scenario without dates is never in force.
* `sub_5a7db8` builds the "Time Line" (every start/end change point) and `sub_5a6c28(day)`
  sets each record's active byte (+0x29) for the current date (also called when the date
  changes: `sub_678d84`, `sub_707ab8`).

## Timetable loading (`sub_72d330`)

For k = last active scenario … 0, then the map's own `TTData` (k = −1):

1. `*.ttr` (tracks) and `*.ttp` (trips) of the folder (`sub_72882c`, `sub_729050`).
2. `*.ttl`: the line's name is the file name. It is **skipped if a line of that name is
   already loaded** (a later scenario's file wins) **or if an active scenario with a higher
   index lists it in `[deactivate_lines]`**. So a scenario takes a line off only for the
   folders before it; a later scenario can bring the line back.
3. Then `car_use/*.ocu` of the map (`sub_72c834`, see below).

## Lines and tours (`.ttl`, reader `sub_72bb38`)

* `[userallowed]` sets the line record's byte +4; the player's line list in the
  "select timetable" dialog (`Tform_settt.FormShow` → `sub_67f120`) shows **only** lines
  with it. (Berlin-Spandau: 5 of 23 lines; the rest are AI-only.) `[priority]` → +5.
* `[newtour]`: number, AI group (depot), day mask (empty → 1023). The mask is split into ten
  bytes at tour +0x20…+0x29.
* Validity (`sub_73bc00`): on a public holiday (`[holiday]` date) only bit 7 counts;
  otherwise the weekday bit (0 Monday … 5 Saturday, 6 Sunday). In addition, in the school
  holidays (a `[holidays]` range) bit **8** must be set, otherwise bit **9**. The printed
  timetable (`sub_7430b4`) confirms it: bit 8 clear → "% : Not on school holidays",
  bit 9 clear → "~ : Only on school holidays".
* A tour started the previous day and running past midnight is checked against that day.

## `car_use/*.ocu` (`sub_72c834`)

`[valid]`, `[line]`, `[number_tour]`, `[type_tour]`, `[onlytypes]`, `[types_prefered]`. A
file whose `[line]` names no loaded line (`sub_73c7f4`: index in `TTimeTableMan.f_18`) is
ignored with "has no valid [line] entry for the current chrono scenario".

## Hof files and IBIS

* `TRoadVehicle.LoadFromFile` (`sub_7cb3f4`) loads **every** `*.hof` of the vehicle folder
  into its list (field 0x5e4) with `THof.LoadFromFile` (`sub_7e9414`).
* `TRoadVehicleInst.virtual_10` (`sub_7e6af0`) — `SetLineTo` / `ai_scheduled_settarget`;
  `TRoadVehicle.virtual_00` (`sub_7ea834`) — `AI_target_index`.
* A hof's `[name]` is THof +4; its termini (`[addterminus]`, `[addterminus_allexit]`, 24-byte
  records: +0 code, +8 name, +0x10 strings, +0x14 all-exit flag) are THof +0x14.
* `ailists.cfg` (`sub_780a58`): `[aigroup_depot]` = group name, then the **hof name** (a
  hof's `[name]`, not a file name) at group record +8; `[aigroup_depot_typgroup(_2)]` must
  come before it.
* An AI timetable bus (`sub_70a174`) takes the hof of its vehicle whose `[name]` equals the
  depot's hof name exactly (case-sensitive) into its selected-hof index (+0x7c8). Without
  a match the index stays 0: the vehicle folder's **first** hof (they are loaded in the
  order the folder lists them). The player's hof is the one chosen in the vehicle dialog
  (`Tform_selectVeh.ComboBox5Change`).
* Starting a trip (`sub_7e6c9c`) calls `TRoadVehicleInst.virtual_10` (`sub_7e6af0`) with the
  trip's line and **terminus name** (the `.ttp`'s `[trip]` fields): the selected hof's
  terminus of that exact name gives `AI_target_index` (float, +0x674; unchanged when there
  is none), the line goes into the string variable `SetLineTo`, and the bus script's
  `ai_scheduled_settarget` trigger runs. The script does the rest (IBIS_TerminusIndex,
  codes, the displays). Coupled parts get the same call.
* So a bus without the map's hof shows the first hof's first terminus in the original. We
  go further (an addition, not a change): the depot's hof is also looked up by name in the
  other vehicle folders (`omsi_vehicle::hof::depot_anywhere`), so its termini and stops are
  the map's.

## What we changed to match

* `[deactivate_lines]` read as count + names (the count had been taken for a line name, so
  "1", "2", "17" … were taken off).
* Chrono validity inclusive at both ends; no dates → never; folders in the game's order.
* `TimetableData::load_with_chrono`: lines from the last folder back, taken off only by
  later scenarios.
* Tour masks: bit 8 = school holidays, bit 9 = school days (they were swapped, so tours
  marked for one ran on the other and showed as "not on this date" in the launcher).
