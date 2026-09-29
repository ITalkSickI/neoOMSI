use super::*;

const IBIS: &str = include_str!("../../../../docs/examples/htmltexture/ibis.html");

fn px(frame: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * w + x) * 4) as usize;
    [frame[i], frame[i + 1], frame[i + 2], frame[i + 3]]
}

#[test]
fn the_ibis_example_shows_the_speed() {
    let mut r = EngineRenderer::new(800, 480, IBIS);
    r.set_vars(&[("velocity_kmh".into(), 42.4)], &[("next_stop".into(), "Hauptbahnhof".into())]);
    assert_eq!(r.text_of("speed").as_deref(), Some("42"));
    assert_eq!(r.text_of("stop").as_deref(), Some("Hauptbahnhof"));
    let a = r.poll_frame().unwrap();
    assert_eq!(a.len(), 800 * 480 * 4);
    assert_eq!(px(&a, 800, 2, 2), [0x10, 0x18, 0x20, 255]);
    assert!(a.chunks(4).any(|p| p[0] > 200 && p[1] > 120 && p[2] < 80), "amber text is drawn");
    r.set_vars(&[("velocity_kmh".into(), 7.0)], &[]);
    assert_eq!(r.text_of("speed").as_deref(), Some("7"));
    let b = r.poll_frame().unwrap();
    assert_ne!(a, b);
    assert!(r.poll_frame().is_none(), "no new frame without a change");
}

#[test]
fn css_colours_and_sizes_are_painted() {
    let html = "<style>#a{width:10px;height:4px;background:#ff0000;margin:1px}.b{background:rgb(0,0,255);height:2px}</style><body style='margin:0'><div id=a></div><div class=b></div></body>";
    let mut r = EngineRenderer::new(20, 20, html);
    let f = r.poll_frame().unwrap();
    assert_eq!(px(&f, 20, 5, 2), [255, 0, 0, 255]);
    assert_eq!(px(&f, 20, 15, 2), [0, 0, 0, 0]);
    assert_eq!(px(&f, 20, 3, 7), [0, 0, 255, 255]);
}

#[test]
fn scripts_write_variables_back() {
    let html = "<body><script>window.omsi = window.omsi || {}; window.omsi.update = function (d) { if (d.num.door > 0) omsi.setVar('lamp', 1); else window.omsi.setVar('lamp', 0); };</script></body>";
    let mut r = EngineRenderer::new(8, 8, html);
    r.set_vars(&[("door".into(), 1.0)], &[]);
    assert_eq!(r.take_events(), vec![("lamp".to_string(), 1.0)]);
    r.set_vars(&[("door".into(), 0.0)], &[]);
    assert_eq!(r.take_events(), vec![("lamp".to_string(), 0.0)]);
}

#[test]
fn javascript_basics() {
    let html = "<body><p id=o></p><script>var t=0; for (var i=1;i<=4;i++){ if (i==3) continue; t+=i*2; } var f=(a,b)=>a>b?a:b; var o={x:[1,2,3]}; document.getElementById('o').textContent = t + ':' + f(3,9) + ':' + o.x.length + ':' + (7.126).toFixed(2) + ':' + 'ab'.padStart(4,'-') + ':' + Math.max(1,5,2) + ':' + typeof o;</script></body>";
    let r = EngineRenderer::new(8, 8, html);
    assert_eq!(r.text_of("o").as_deref(), Some("14:9:3:7.13:--ab:5:object"));
}

#[test]
fn a_broken_script_does_not_stop_the_page() {
    let html = "<body style='background:#00ff00'><script>this is not javascript (</script></body>";
    let mut r = EngineRenderer::new(4, 4, html);
    let f = r.poll_frame().unwrap();
    assert_eq!(px(&f, 4, 1, 1), [0, 255, 0, 255]);
}

#[test]
fn an_endless_loop_is_cut_off() {
    let html = "<body><script>while (true) {}</script></body>";
    let mut r = EngineRenderer::new(4, 4, html);
    assert!(r.poll_frame().is_some());
}

// ─────────────── operating the page ───────────────

fn click(r: &mut EngineRenderer, x: f32, y: f32) {
    r.pointer(x, y, PointerKind::Down);
    r.pointer(x, y, PointerKind::Up);
}

const KEYPAD: &str = "<style>body{margin:0;width:200px;height:100px;background:#000}\
    button{width:40px;height:20px;margin:0;padding:0}</style><body>\
    <button id=a onclick=\"omsi.setVar('key', 1)\">A</button><button id=b>B</button><button id=c>C</button>\
    <script>document.getElementById('b').addEventListener('click', function (e) { omsi.setVar('key', 2); });\
    document.getElementById('c').onclick = () => omsi.setVar('key', 3);</script></body>";

#[test]
fn buttons_sit_in_a_row_and_take_clicks() {
    let mut r = EngineRenderer::new(200, 100, KEYPAD);
    // three 40px buttons side by side: x 0..40, 40..80, 80..120, all in the first row
    click(&mut r, 20.0, 10.0);
    assert_eq!(r.take_events(), vec![("key".to_string(), 1.0)]);
    click(&mut r, 60.0, 10.0);
    assert_eq!(r.take_events(), vec![("key".to_string(), 2.0)]);
    click(&mut r, 100.0, 10.0);
    assert_eq!(r.take_events(), vec![("key".to_string(), 3.0)]);
    // beside the buttons and below them nothing happens
    click(&mut r, 150.0, 10.0);
    click(&mut r, 20.0, 60.0);
    assert!(r.take_events().is_empty());
}

#[test]
fn a_click_needs_a_press_and_a_release() {
    let mut r = EngineRenderer::new(200, 100, KEYPAD);
    r.pointer(20.0, 10.0, PointerKind::Up);
    assert!(r.take_events().is_empty(), "a release alone is no click");
    r.pointer(20.0, 10.0, PointerKind::Down);
    assert!(r.take_events().is_empty(), "a press alone is no click");
    r.pointer(20.0, 10.0, PointerKind::Up);
    assert_eq!(r.take_events().len(), 1);
}

#[test]
fn inline_blocks_wrap_and_shrink_to_fit() {
    let html = "<style>body{margin:0;width:100px;height:100px}.k{display:inline-block;width:30px;height:10px;margin:0}</style>\
        <body><div class=k onclick=\"omsi.setVar('k',1)\"></div><div class=k onclick=\"omsi.setVar('k',2)\"></div>\
        <div class=k onclick=\"omsi.setVar('k',3)\"></div><div class=k onclick=\"omsi.setVar('k',4)\"></div></body>";
    let mut r = EngineRenderer::new(100, 100, html);
    // 3 fit into 100px, the fourth wraps to the second row
    click(&mut r, 75.0, 5.0);
    assert_eq!(r.take_events(), vec![("k".to_string(), 3.0)]);
    click(&mut r, 5.0, 15.0);
    assert_eq!(r.take_events(), vec![("k".to_string(), 4.0)]);
    // a button without a width is as wide as its label
    let plain = "<body style='margin:0'><button id=x onclick=\"omsi.setVar('x',1)\" style='padding:0'>Hi</button></body>";
    let mut r = EngineRenderer::new(200, 50, plain);
    click(&mut r, 190.0, 5.0);
    assert!(r.take_events().is_empty(), "the button does not fill the row");
    click(&mut r, 3.0, 5.0);
    assert_eq!(r.take_events().len(), 1);
}

#[test]
fn clicks_bubble_and_can_be_stopped() {
    let html = "<body style='margin:0'><div id=outer onclick=\"omsi.setVar('outer',1)\" style='height:50px'>\
        <div id=inner style='height:20px'></div></div><script>\
        document.getElementById('inner').addEventListener('click', function (e) { omsi.setVar('inner', 1); if (e.x > 100) e.stopPropagation(); });</script></body>";
    let mut r = EngineRenderer::new(200, 100, html);
    click(&mut r, 10.0, 5.0);
    let ev = r.take_events();
    assert_eq!(ev, vec![("inner".to_string(), 1.0), ("outer".to_string(), 1.0)]);
    click(&mut r, 150.0, 5.0);
    assert_eq!(r.take_events(), vec![("inner".to_string(), 1.0)]);
    click(&mut r, 10.0, 40.0);
    assert_eq!(r.take_events(), vec![("outer".to_string(), 1.0)]);
}

#[test]
fn a_span_in_text_is_hit_on_its_own() {
    let html = "<body style='margin:0;font-size:20px'>Go <span id=s onclick=\"omsi.setVar('s',1)\">here</span> now</body>";
    let mut r = EngineRenderer::new(300, 40, html);
    click(&mut r, 2.0, 10.0);
    assert!(r.take_events().is_empty());
    click(&mut r, 42.0, 10.0);
    assert_eq!(r.take_events(), vec![("s".to_string(), 1.0)]);
}

#[test]
fn a_page_reacts_to_its_own_clicks() {
    // a route picker: the list is built by script, a click selects and writes the choice back
    let html = "<style>body{margin:0}.row{height:20px}.sel{background:#00ff00}</style><body><div id=list></div><script>\
        var routes = ['A', 'B', 'C']; var chosen = -1;\
        function draw() { var l = document.getElementById('list'); l.innerHTML = '';\
          routes.forEach(function (n, i) { var d = document.createElement('div'); d.className = 'row' + (i == chosen ? ' sel' : '');\
            d.textContent = n; d.addEventListener('click', function () { chosen = i; omsi.setVar('route', i); draw(); }); l.appendChild(d); }); }\
        draw();</script></body>";
    let mut r = EngineRenderer::new(50, 80, html);
    let f = r.poll_frame().unwrap();
    assert_eq!(px(&f, 50, 45, 30), [0, 0, 0, 0]);
    click(&mut r, 10.0, 30.0); // the second row
    assert_eq!(r.take_events(), vec![("route".to_string(), 1.0)]);
    let f = r.poll_frame().expect("the click changed the page");
    assert_eq!(px(&f, 50, 45, 30), [0, 255, 0, 255]);
    assert_eq!(px(&f, 50, 45, 10), [0, 0, 0, 0]);
}

#[test]
fn class_list_and_inner_html() {
    let html = "<style>.on{background:#ff0000}</style><body style='margin:0'><div id=a style='height:4px'></div><div id=b></div><script>\
        var a = document.getElementById('a'); a.classList.add('x'); a.classList.toggle('on'); a.classList.toggle('x');\
        var t = a.classList.contains('on') + ',' + a.classList.contains('x');\
        document.getElementById('b').innerHTML = '<p id=p>hi <b>there</b></p>';\
        document.getElementById('b').setAttribute('id', 'b2'); window.t = t;</script></body>";
    let mut r = EngineRenderer::new(20, 20, html);
    let f = r.poll_frame().unwrap();
    assert_eq!(px(&f, 20, 5, 2), [255, 0, 0, 255]);
    assert_eq!(r.text_of("p").as_deref(), Some("hi there"));
}

#[test]
fn timers_fire_when_due_and_can_be_cleared() {
    let html = "<body><p id=o></p><script>var n = 0; var once = 0; var id = setInterval(function () { n++; document.getElementById('o').textContent = 'n' + n;\
        if (n == 3) clearInterval(id); }, 100); setTimeout(function () { once++; omsi.setVar('once', once); }, 250);</script></body>";
    let mut r = EngineRenderer::new(8, 8, html);
    r.js.now = 0.05;
    assert!(!r.run_timers());
    r.js.now = 0.11;
    assert!(r.run_timers());
    assert_eq!(r.text_of("o").as_deref(), Some("n1"));
    r.js.now = 0.26;
    assert!(r.run_timers());
    assert_eq!(r.text_of("o").as_deref(), Some("n2"));
    assert_eq!(r.take_events(), vec![("once".to_string(), 1.0)]);
    r.js.now = 0.40;
    r.run_timers();
    assert_eq!(r.text_of("o").as_deref(), Some("n3"));
    r.js.now = 5.0;
    assert!(!r.run_timers(), "cleared, and the timeout only ran once");
}

#[test]
fn arrays_objects_and_strings() {
    let html = "<body><p id=o></p><script>var a = [3, 1, 2]; var b = a.map(function (x) { return x * 2; }).filter(x => x > 2);\
        var o = {z: 1, a: 2}; var s = 'a-b-c'.split('-'); var sum = 0; a.forEach(function (x, i) { sum += x * i; });\
        document.getElementById('o').textContent = b.join('') + '|' + Object.keys(o).join('') + '|' + s.length + s[2] + '|' + sum + '|'\
          + a.indexOf(2) + a.includes(9) + '|' + a.slice(1).join('') + '|' + 'x.y'.replace('.', '+') + '|' + a.pop() + a.length;</script></body>";
    let r = EngineRenderer::new(8, 8, html);
    assert_eq!(r.text_of("o").as_deref(), Some("64|az|3c|5|2false|12|x+y|22"));
}

#[test]
fn a_failing_callback_or_handler_does_not_stop_the_page() {
    let html = "<body style='margin:0'><div id=d style='height:10px' onclick=\"nope.x()\"></div><div id=e style='height:10px' onclick=\"omsi.setVar('ok', 1)\"></div>\
        <script>[1].forEach(function () { missing.call(); }); document.getElementById('d').addEventListener('click', function () { throw_it(); });</script></body>";
    let mut r = EngineRenderer::new(40, 40, html);
    click(&mut r, 5.0, 5.0);
    click(&mut r, 5.0, 15.0);
    assert_eq!(r.take_events(), vec![("ok".to_string(), 1.0)]);
}

#[test]
fn a_click_can_press_a_trigger() {
    let html = "<body style='margin:0'><button style='padding:0;width:30px;height:10px' onclick=\"omsi.trigger('door_1'); omsi.setVar('seen', 1)\">Door</button></body>";
    let mut r = EngineRenderer::new(60, 20, html);
    click(&mut r, 5.0, 5.0);
    assert_eq!(r.take_triggers(), vec!["door_1".to_string()]);
    assert_eq!(r.take_events(), vec![("seen".to_string(), 1.0)]);
    assert!(r.take_triggers().is_empty());
}

#[test]
fn the_controls_example_can_be_operated() {
    const CONTROLS: &str = include_str!("../../../../docs/examples/htmltexture/controls.html");
    let mut r = EngineRenderer::new(800, 480, CONTROLS);
    r.set_vars(&[("velocity_kmh".into(), 34.2)], &[("next_stop".into(), "Hauptbahnhof".into())]);
    assert_eq!(r.text_of("speed").as_deref(), Some("34"));
    let idle = r.poll_frame().unwrap();
    click(&mut r, 55.0, 196.0); // 1
    click(&mut r, 55.0, 266.0); // 4
    assert_eq!(r.text_of("entry").as_deref(), Some("14"));
    assert!(r.take_events().is_empty(), "nothing is sent before OK");
    click(&mut r, 230.0, 406.0); // OK
    assert_eq!(r.take_events(), vec![("IBIS_Linie".to_string(), 14.0)]);
    click(&mut r, 500.0, 242.0); // Doors
    assert_eq!(r.take_triggers(), vec!["door_toggle".to_string()]);
    let typed = r.poll_frame().unwrap();
    assert_ne!(idle, typed, "the page shows what was pressed");
    // the flash of the pressed button ends by itself
    r.js.now += 1.0;
    assert!(r.run_timers());
    assert_eq!(px(&r.render(), 800, 420, 220), [0x2b, 0x3a, 0x4a, 255]);
}

fn map(items: Vec<(&str, crate::vehicle_api::ApiValue)>) -> crate::vehicle_api::ApiValue {
    crate::vehicle_api::ApiValue::Map(items.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn sample_vehicle() -> crate::vehicle_api::ApiValue {
    use crate::vehicle_api::ApiValue as A;
    let door = |n: f64, open: f64| map(vec![("number", A::Num(n)), ("open", A::Num(open)), ("isOpen", A::Bool(open > 0.05))]);
    map(vec![
        ("info", map(vec![("number", A::Str("4711".into()))])),
        ("engine", map(vec![("running", A::Bool(true)), ("rpm", A::Null)])),
        ("doors", map(vec![("count", A::Num(2.0)), ("list", A::List(vec![door(1.0, 0.0), door(2.0, 1.0)]))])),
    ])
}

#[test]
fn a_page_reads_the_vehicle_object() {
    let html = "<body><p id=e></p><p id=d></p><p id=n></p><p id=r></p><script>\
        window.omsi = window.omsi || {};\
        window.omsi.update = function (d) {\
          var v = omsi.vehicle;\
          document.getElementById('e').textContent = v.engine.running ? 'on' : 'off';\
          document.getElementById('d').textContent = v.doors.list[1].isOpen + ':' + v.doors.count + ':' + v.doors.list[0].number;\
          document.getElementById('n').textContent = d.vehicle.info.number;\
          document.getElementById('r').textContent = v.engine.rpm == null ? 'none' : 'rpm';\
        };</script></body>";
    let mut r = EngineRenderer::new(40, 40, html);
    r.set_vehicle(&sample_vehicle());
    r.set_vars(&[], &[]);
    assert_eq!(r.text_of("e").as_deref(), Some("on"));
    assert_eq!(r.text_of("d").as_deref(), Some("true:2:1"));
    assert_eq!(r.text_of("n").as_deref(), Some("4711"));
    assert_eq!(r.text_of("r").as_deref(), Some("none"), "a signal the bus lacks is null");
}

#[test]
fn every_variable_can_be_read_by_name_in_any_letter_case() {
    let html = "<body><p id=a></p><p id=b></p><p id=c></p><p id=t></p><script>\
        window.omsi = window.omsi || {};\
        window.omsi.update = function (d) {\
          document.getElementById('a').textContent = omsi.getVar('Engine_N');\
          document.getElementById('b').textContent = omsi.vars.num.engine_n;\
          document.getElementById('c').textContent = omsi.getVar('IDENT') + '/' + typeof omsi.getVar('nope');\
          document.getElementById('t').textContent = d.vars.num.throttle;\
        };</script></body>";
    let mut r = EngineRenderer::new(40, 40, html);
    r.set_vars(
        &[("Engine_N".into(), 812.0), ("Throttle".into(), 0.5)],
        &[("ident".into(), "OMS-1".into())],
    );
    assert_eq!(r.text_of("a").as_deref(), Some("812"));
    assert_eq!(r.text_of("b").as_deref(), Some("812"));
    assert_eq!(r.text_of("c").as_deref(), Some("OMS-1/undefined"));
    assert_eq!(r.text_of("t").as_deref(), Some("0.5"));
}

#[test]
fn a_timer_sees_the_latest_vehicle_without_an_update_function() {
    let html = "<body><p id=e></p><script>\
        setInterval(function () {\
          document.getElementById('e').textContent = omsi.vehicle.doors ? omsi.vehicle.doors.count : 'none';\
        }, 10);</script></body>";
    let mut r = EngineRenderer::new(40, 40, html);
    r.set_vehicle(&sample_vehicle());
    r.set_vars(&[], &[]);
    std::thread::sleep(std::time::Duration::from_millis(30));
    assert!(r.poll_frame().is_some());
    assert_eq!(r.text_of("e").as_deref(), Some("2"));
}

const DASHBOARD: &str = include_str!("../../../../docs/examples/htmltexture/dashboard.html");

#[test]
fn the_dashboard_example_shows_the_vehicle() {
    use crate::vehicle_api::{snapshot, Inputs};
    let var = |n: &str| match n.to_ascii_lowercase().as_str() {
        "door_0" => Some(0.0),
        "door_1" => Some(1.0),
        "engine_n" => Some(800.0),
        _ => None,
    };
    let text = |n: &str| if n == "number" { "4711".to_string() } else { String::new() };
    let api = snapshot(&Inputs {
        var: &var,
        text: &text,
        speed_kmh: 36.6,
        steer_deg: 0.0,
        heading: 0.0,
        pitch: 0.0,
        bank: 0.0,
        position: (0.0, 0.0, 0.0),
        engine_running: true,
        interior_light: 0.0,
        crashes: 0,
        last_impact_j: 0.0,
        dirt: 0.0,
        trailers: 0,
    });
    let mut r = EngineRenderer::new(800, 480, DASHBOARD);
    r.set_vehicle(&api);
    r.set_vars(&[], &[]);
    assert_eq!(r.text_of("kmh").as_deref(), Some("37"));
    assert_eq!(r.text_of("num").as_deref(), Some("Bus 4711"));
    let f = r.poll_frame().unwrap();
    assert!(f.chunks(4).any(|p| p[0] == 0xc6 && p[1] == 0x28 && p[2] == 0x28), "the open door is drawn red");
    assert!(f.chunks(4).any(|p| p[0] == 0x2e && p[1] == 0x7d && p[2] == 0x32), "the running engine is drawn green");
}
