# OMSI plugins (`plugins/*.opl` + DLL)

What OMSI does with plugins, and how
openOMSI does the same (`crates/omsi-plugin`, driven from `crates/omsi-app/src/plugins.rs`).

## The original

* **Finding them**: every `*.opl` under `<OMSI>\plugins`, recursively
  (`FindFilesRecursive`). Tags: `[dll]` (a path relative to `plugins\`), `[varlist]`,
  `[stringvarlist]`, `[systemvarlist]`, `[triggers]` - each a count, then that many names.
* **Loading**: `LoadLibrary`, then `GetProcAddress` for
  `PluginStart` and `PluginFinalize` (required - "Could not load plugin …: procedure … not
  found!") and `AccessVariable`, `AccessTrigger`, `AccessSystemVariable`,
  `AccessStringVariable` (optional - "Loading plugin …: procedure … not found!"). Then
  `PluginStart(AOwner)`. `PluginFinalize` runs when the game ends.
* **Every frame**, plugin by plugin:
  1. each listed system variable: `AccessSystemVariable(index: Word; var value: Single;
     var write: Boolean)`; written back when `write` is true;
  2. with a player vehicle: each listed vehicle variable (`AccessVariable`, same shape);
  3. each string variable: `AccessStringVariable(index: Word; text: PWideChar;
     var write: Boolean)` - a buffer of length + 1 wide characters with the text and its
     terminating zero; read back when `write` is true;
  4. each trigger: `AccessTrigger(index: Word; var active: Boolean)`, `active` false before
     the call. A change from the last frame is a key event: down fires the
     trigger, up fires `<trigger>_off`.

  All `stdcall`; `index` is the position in the plugin's own list. Names the vehicle does
  not have are skipped.

## openOMSI

* `omsi_plugin::Plugins::load` reads the `plugins` folder of every content root (the
  first root's copy of an `.opl` wins) and loads each library:
  * **in-process** when the running program can load it (same system and architecture -
    a plugin built for openOMSI, or a 32-bit DLL in a 32-bit Windows build);
  * otherwise in **`omsi-plugin-host32.exe`**, `omsi-plugin-host` built for 32-bit Windows,
    which loads the DLL and answers over stdin/stdout (one round trip per frame). On
    Windows it runs directly; on macOS and Linux through Wine (`wine` on the `PATH`, or
    `OMSI_WINE`). The host is found next to the game (`OMSI_PLUGIN_HOST32` overrides).
* The system variables are the scripts' (`omsi_script::SysVar`); a plugin's writes to
  them are not applied (the clock, weather and input stay the game's).
* `OMSI_NO_PLUGINS=1` leaves every plugin out. A plugin whose host stops answering is
  left out for the rest of the session.
* `PluginStart` gets a nil owner: there is no Delphi application object. Plugins that
  open their own windows do so without a parent.

## Building the host

```bash
scripts/build-plugin-host.sh
```

needs `rustup target add i686-pc-windows-gnu` and MinGW (`brew install mingw-w64`);
Copy `dist/omsi-plugin-host32.exe` next to the game. The 32-bit build uses
`panic=abort` and a stand-in `_Unwind_Resume` (Homebrew's i686 MinGW links no unwinder the
prebuilt standard library can use).

## Tests

`cargo test -p omsi-plugin` builds `crates/omsi-plugin/demo` (a plugin with the OMSI
interface) and drives it in-process and through the host. With
`OMSI_TEST_WINE_DIR=target/i686-pc-windows-gnu/release` (after building the host and the
demo for `i686-pc-windows-gnu`) the test `windows_dll_under_wine` runs the real chain: a
32-bit Windows DLL with Delphi-style undecorated `stdcall` exports, in the 32-bit host,
under Wine.
