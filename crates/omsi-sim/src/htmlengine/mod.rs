//! The built-in backend of `[htmltexture]`: a small HTML/CSS/JavaScript engine that needs no
//! browser and no extra dependency, so it runs the same on every desktop and on Android.
//!
//! What a page may use:
//! * HTML: nested elements, `<style>`, `<script>`, inline `style=""`, `id`, `class`, entities.
//! * CSS: selectors `tag`, `#id`, `.class`, `*`, compounds (`div.a#b`), descendant chains
//!   and `,` lists; `color`, `background(-color)`, `font-size`, `font-weight`, `text-align`,
//!   `line-height`, `margin*`, `padding*`, `width`, `height`, `display` (`none`, `inline`),
//!   `visibility`, `border-radius`. Units: `px`, `%`, `em`, `rem`, `pt`, `vw`, `vh`.
//!   Layout is block flow with wrapped inline text (no floats, no flexbox).
//! * JavaScript (ES5 plus arrow functions): `var/let/const`, functions, `if/for/while`,
//!   objects, arrays, `Math.*`, `parseInt/parseFloat/String/Number`,
//!   `document.getElementById/querySelector/body`, `element.textContent/innerText/className/id`,
//!   `element.style.*`, `element.setAttribute`.
//!
//! The page talks to the vehicle through `window.omsi`:
//! * the host calls `window.omsi.update({ num: {name: value}, str: {name: "text"} })` with the
//!   script variables that changed (all of them on the first call);
//! * the page calls `window.omsi.setVar(name, value)` to write a script variable back;
//! * the page calls `window.omsi.trigger(name)` to press a trigger of the vehicle's scripts
//!   (what a button in the cab does).
//! * `window.omsi.vehicle` is a normalised snapshot of the vehicle with fixed names for every
//!   bus: `engine.running/rpm`, `battery.on`, `doors.list[i].isOpen`, `lights.*`, `brakes.*` and
//!   more (see [`crate::vehicle_api`] for the full list). A signal the bus lacks is `null` or
//!   `false`. It is current whenever `update` runs and can be read from timers too.
//! * `window.omsi.vars.num` / `.str` hold the latest value of every variable of the bus's own
//!   variable list under its lower-case name, and `window.omsi.getVar(name)` reads one in any
//!   letter case (`undefined` when the bus has none). `update`'s argument carries `vehicle`
//!   and `vars` as well as the changed `num` / `str`.
//!
//! A page can be operated: the app passes presses, releases and moves of the pointer on the
//! texture ([`HtmlRenderer::pointer`]). They become `pointerdown`/`mousedown`, `pointerup`/
//! `mouseup`, `click` and `mousemove` on the element under the pointer, which bubble up through
//! its parents. Listeners: the `onclick="..."` attribute (`event` is the event object), the
//! `element.onclick = f` property and `element.addEventListener("click", f)`; the event has
//! `type`, `x`, `y` (texture pixels), `target`, `stopPropagation()` and `preventDefault()`.
//! * `display:inline-block` (and `<button>`, which has a default look) lays boxes out in rows that
//!   wrap; a box without a `width` is as wide as its content. Use it for key pads and lists.
//! * `window.omsi.vehicle.route` holds line, destination sign, the stops with their planned
//!   times, the stop the bus is at and the last stop (see [`crate::vehicle_api`]).
//!   `window.omsi.depot` lists the depot file's `lines[]` (each with its `routes[]`), `routes[]`
//!   and `destinations[]`; the page sets the IBIS with `omsi.setRoute(index)` (line, route and
//!   destination of `depot.routes[index]`), `omsi.setLine(text)` (the first route of that
//!   line) and `omsi.setDestination(index)` (only the destination sign, `depot.destinations`).
//!   `omsi.setNextStop(index)` moves the duty on to stop `index` of its trip (`route.stops[index]`;
//!   the stops before it are skipped, backwards is ignored).
//! * More JavaScript for such pages: `setTimeout`/`setInterval`/`clear*`, `classList`,
//!   `createElement`/`appendChild`/`removeChild`/`remove`, `innerHTML` with markup, `getAttribute`,
//!   `parentNode`, `Object.keys`, `Array.forEach/map/filter/indexOf/includes/pop/shift/slice`,
//!   `String.split/replace`. Timers run when the page is drawn and use real time.
//!
//! # Layout of this module
//! * `dom`: HTML parser and CSS rule parser. `style`: computed style. `layout`: block flow and
//!   inline text. `canvas`: pixels and painter.
//! * `js`: the JavaScript subset (`lexer`, `parser`, `interp`).
//! * `renderer`: [`EngineRenderer`], the [`HtmlRenderer`] backend that ties them together.

use crate::htmltex::{HtmlRenderer, PointerKind};
use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont, VariableFont};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod api;
mod canvas;
mod dom;
mod js;
mod layout;
mod renderer;
mod style;

use api::*;
use canvas::*;
use dom::*;
use js::*;
use layout::*;
use style::*;

pub use renderer::EngineRenderer;

pub(crate) const ROBOTO: &[u8] = include_bytes!("../../../../assets/fonts/Roboto-VariableFont_wdth,wght.ttf");
pub(crate) const STEP_LIMIT: u32 = 400_000;
pub(crate) const DEPTH_LIMIT: u32 = 48;

#[cfg(test)]
mod tests;