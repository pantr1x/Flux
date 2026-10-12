// flux-preview --parent <X11 okno> --url <adresa>
// Príkazy zo stdin (JSON po riadkoch): {"bounds":[x,y,w,h]} {"visible":b} {"url":u} {"reload":true} {"zoom":f}
// Udalosti na stdout: {"nav":u} (stránka prešla na adresu) {"tab":u} (odkaz do novej karty) {"ready":true}
// Skončí, keď sa zavrie stdin (Flux skončil alebo spadol).
#[cfg(target_os = "linux")]
fn main() {
    linux::run();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("flux-preview is only for Linux");
    std::process::exit(2);
}

#[cfg(target_os = "linux")]
mod linux {
    use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle, WindowHandle, XlibDisplayHandle, XlibWindowHandle};
    use serde_json::{json, Value};
    use std::io::{BufRead, Write};
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::Duration;

    // cudzie okno (Flux) podľa čísla X11 okna
    struct Parent(u64);
    impl HasWindowHandle for Parent {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            let raw = RawWindowHandle::Xlib(XlibWindowHandle::new(self.0 as _));
            Ok(unsafe { WindowHandle::borrow_raw(raw) })
        }
    }
    impl HasDisplayHandle for Parent {
        fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
            let raw = RawDisplayHandle::Xlib(XlibDisplayHandle::new(None, 0));
            Ok(unsafe { DisplayHandle::borrow_raw(raw) })
        }
    }

    fn send(v: Value) {
        let mut o = std::io::stdout().lock();
        let _ = writeln!(o, "{v}");
        let _ = o.flush();
    }

    fn arg(name: &str) -> Option<String> {
        let a: Vec<String> = std::env::args().collect();
        a.iter().position(|x| x == name).and_then(|i| a.get(i + 1).cloned())
    }

    fn rect(v: &Value) -> Option<wry::Rect> {
        let a = v.as_array()?;
        let f = |i: usize| a.get(i).and_then(|x| x.as_f64()).unwrap_or(0.0);
        Some(wry::Rect { position: wry::dpi::LogicalPosition::new(f(0), f(1)).into(), size: wry::dpi::LogicalSize::new(f(2).max(1.0), f(3).max(1.0)).into() })
    }

    pub fn run() {
        let (Some(parent), Some(url)) = (arg("--parent").and_then(|p| p.parse::<u64>().ok()), arg("--url")) else {
            eprintln!("usage: flux-preview --parent <xid> --url <url>");
            std::process::exit(2);
        };
        // vkladáme do X11 okna Fluxu – GTK musí ísť cez X11 aj pod Waylandom (inak GDK zvolí Wayland a spadne)
        std::env::set_var("GDK_BACKEND", "x11");
        // chyba = čisté ukončenie s hlásením na stderr (Flux ho zapíše do flux-preview.log), nie „pád aplikácie“
        std::panic::set_hook(Box::new(|i| {
            eprintln!("flux-preview panic: {i}");
            std::process::exit(6);
        }));
        // DMA-BUF prenos obrazu WebKitu nefunguje v cudzom X11 podokne (XWayland, virtuálky) – zostane čierne
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        if gtk::init().is_err() {
            eprintln!("gtk init failed");
            std::process::exit(3);
        }
        // príkazy zo stdin v samostatnom vlákne; koniec stdin = koniec
        let (tx, rx) = mpsc::channel::<Value>();
        std::thread::spawn(move || {
            for line in std::io::stdin().lock().lines() {
                let Ok(l) = line else { break };
                if let Ok(v) = serde_json::from_str::<Value>(&l) {
                    if tx.send(v).is_err() {
                        break;
                    }
                }
            }
            std::process::exit(0);
        });
        // ako na Windows: vlastná doména ostane v karte, iný web a target=_blank → nová karta
        let origin = url.split('/').take(3).collect::<Vec<_>>().join("/");
        let cur = Arc::new(Mutex::new(String::new()));
        let cur2 = cur.clone();
        let nav = move |u: String| {
            let here = cur2.lock().map(|c| c.clone()).unwrap_or_default();
            let base = if here.is_empty() { origin.clone() } else { here.split('/').take(3).collect::<Vec<_>>().join("/") };
            if here.is_empty() || !u.starts_with("http") || u.starts_with(&base) {
                if let Ok(mut c) = cur2.lock() {
                    *c = u.clone();
                }
                send(json!({ "nav": u }));
                true
            } else {
                send(json!({ "tab": u }));
                false
            }
        };
        let neww = |u: String, _f: wry::NewWindowFeatures| {
            if u.starts_with("http") {
                send(json!({ "tab": u }));
            }
            wry::NewWindowResponse::Deny
        };
        let first = rect(&serde_json::from_str::<Value>(&arg("--bounds").unwrap_or_default()).unwrap_or(Value::Null)).unwrap_or(wry::Rect {
            position: wry::dpi::LogicalPosition::new(0.0, 0.0).into(),
            size: wry::dpi::LogicalSize::new(400.0, 300.0).into(),
        });
        let parent = Parent(parent);
        let view = match wry::WebViewBuilder::new().with_url(&url).with_bounds(first).with_focused(false).with_navigation_handler(nav).with_new_window_req_handler(neww).build_as_child(&parent) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("webview: {e}");
                std::process::exit(4);
            }
        };
        send(json!({ "ready": true }));
        loop {
            let mut busy = false;
            while gtk::events_pending() {
                gtk::main_iteration_do(false);
                busy = true;
            }
            while let Ok(c) = rx.try_recv() {
                busy = true;
                if let Some(r) = c.get("bounds").and_then(rect) {
                    let _ = view.set_bounds(r);
                }
                if let Some(v) = c.get("visible").and_then(|v| v.as_bool()) {
                    let _ = view.set_visible(v);
                }
                if let Some(u) = c.get("url").and_then(|v| v.as_str()) {
                    if let Ok(mut cu) = cur.lock() {
                        cu.clear();
                    }
                    let _ = view.load_url(u);
                }
                if c.get("reload").and_then(|v| v.as_bool()) == Some(true) {
                    let _ = view.reload();
                }
                if let Some(z) = c.get("zoom").and_then(|v| v.as_f64()) {
                    let _ = view.zoom(z);
                }
            }
            if !busy {
                std::thread::sleep(Duration::from_millis(8));
            }
        }
    }
}
