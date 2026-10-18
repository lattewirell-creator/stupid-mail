// The Stupid Mail for Windows: the same interface as the Mac app (dist/, built into this program) in Windows'
// own web engine (WebView2), with the Mac app's native side ported to Rust: mail (mail.rs), Outlook / Buddy /
// workspaces (net.rs), local storage and secrets (store.rs), calendar (calendar.rs), dialogs (dialog.rs),
// the bridge actions (bridge.rs) and updates (update.rs).
#![cfg_attr(windows, windows_subsystem = "windows")]
mod bridge;
mod calendar;
mod dialog;
mod mail;
mod net;
mod store;
mod update;
#[cfg(test)]
mod tests;

use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget};
use tao::window::{Window, WindowBuilder, WindowId};
use wry::{http::Response, WebView, WebViewBuilder};

static WEB: include_dir::Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/../dist");
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Work handed back to the main thread (from IPC, dialogs, worker threads).
pub enum Ev {
    Ipc(String),
    Dialog(u32, i64, Map<String, Value>),
    Run(Box<dyn FnOnce(&mut App) + Send>),
    Menu(muda::MenuId),
}
pub type DialogDone = Box<dyn FnOnce(&mut App, i64, Map<String, Value>)>;
/// A dialog: its window (created by the event loop) and what to do with the answer.
pub struct DialogEntry { win: Option<(Window, WebView)>, done: Option<DialogDone> }
enum Open { Dialog(u32, dialog::Spec), Workspace(String) }

pub struct App {
    pub proxy: EventLoopProxy<Ev>,
    window: Window,
    pub view: WebView,
    dialogs: HashMap<u32, DialogEntry>,
    next_dialog: u32,
    queue: Vec<Open>,
    pub workspaces: HashMap<String, (Window, WebView)>,
    pub mail_busy: Arc<AtomicBool>,
    pub buddy_busy: Arc<AtomicBool>,
    menu_ids: HashMap<muda::MenuId, &'static str>,
    pub workspace_menu: HashMap<muda::MenuId, (String, &'static str)>,
    pub workspace_menus: Vec<muda::Menu>,
    exit: bool,
}

impl App {
    /// window.nativeResponse({id, ok, data, error}) in the interface.
    pub fn respond(&self, id: &str, result: Result<Value, String>) {
        let payload = match result {
            Ok(data) => serde_json::json!({"id": id, "ok": true, "data": data}),
            Err(e) => serde_json::json!({"id": id, "ok": false, "data": {}, "error": e}),
        };
        let _ = self.view.evaluate_script(&format!("window.nativeResponse({payload})"));
    }
    pub fn js(&self, code: &str) { let _ = self.view.evaluate_script(code); }
    pub fn focus_main(&self) { self.window.set_focus(); }
    /// Opens a dialog window; `done(app, button, values)` runs when the user picks a button (closing = last button).
    pub fn dialog(&mut self, spec: dialog::Spec, done: impl FnOnce(&mut App, i64, Map<String, Value>) + 'static) -> u32 {
        self.next_dialog += 1;
        let id = self.next_dialog;
        self.dialogs.insert(id, DialogEntry { win: None, done: Some(Box::new(done)) });
        // Development self-test: STUPID_MAIL_TEST_ANSWERS='[{"button":0,"values":{...}},…]' answers dialogs in order.
        if let Ok(list) = std::env::var("STUPID_MAIL_TEST_ANSWERS") {
            let mut answers: Vec<Value> = serde_json::from_str(&list).unwrap_or_default();
            if let Some(a) = ANSWERED.with(|n| { let i = n.get(); n.set(i + 1); if i < answers.len() { Some(answers.remove(i)) } else { None } }) {
                println!("[test] dialog: {} → button {}", spec.title, a["button"]);
                let _ = self.proxy.send_event(Ev::Dialog(id, a["button"].as_i64().unwrap_or(-1), a["values"].as_object().cloned().unwrap_or_default()));
                return id;
            }
        }
        self.queue.push(Open::Dialog(id, spec));
        id
    }
    pub fn notice(&mut self, text: &str) { self.dialog(dialog::Spec::new("The Stupid Mail", text, &["OK"]), |_, _, _| {}); }
    pub fn close_dialog(&mut self, id: u32) { self.dialogs.remove(&id); }
    pub fn dialog_open(&self) -> bool { !self.dialogs.is_empty() }
    pub fn open_workspace(&mut self, provider: &str) { self.queue.push(Open::Workspace(provider.to_string())); }
    /// Runs `work` on a worker thread, then `then` on the main thread with its result.
    pub fn background<T: Send + 'static>(&self, work: impl FnOnce() -> T + Send + 'static, then: impl FnOnce(&mut App, T) + Send + 'static) {
        let proxy = self.proxy.clone();
        std::thread::spawn(move || { let r = work(); let _ = proxy.send_event(Ev::Run(Box::new(move |app| then(app, r)))); });
    }
}
thread_local! { static ANSWERED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html", "js" | "mjs" => "text/javascript", "css" => "text/css", "png" => "image/png", "svg" => "image/svg+xml",
        "ttf" => "font/ttf", "woff2" => "font/woff2", "json" => "application/json", "txt" => "text/plain", _ => "application/octet-stream",
    }
}
/// The interface's address: wry serves custom protocols as http://mail.localhost on Windows, mail://localhost elsewhere.
pub fn origin() -> &'static str { if cfg!(windows) { "http://mail.localhost" } else { "mail://localhost" } }

fn main() {
    let event_loop = EventLoopBuilder::<Ev>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let window = WindowBuilder::new().with_title("The Stupid Mail").with_inner_size(LogicalSize::new(1180.0, 820.0)).with_min_inner_size(LogicalSize::new(420.0, 550.0))
        .build(&event_loop).expect("window");

    // Menus (the Mac app's menu bar).
    let menu = muda::Menu::new();
    let mut menu_ids = HashMap::new();
    let item = |label: &str, accel: Option<&str>, name: &'static str, ids: &mut HashMap<muda::MenuId, &'static str>| {
        let i = muda::MenuItem::new(label, true, accel.and_then(|a| a.parse().ok())); ids.insert(i.id().clone(), name); i
    };
    let file = muda::Submenu::with_items("&File", true, &[
        &item("Connect Mail Account…", Some("CmdOrCtrl+,"), "connect", &mut menu_ids), &item("New Message", Some("CmdOrCtrl+N"), "compose", &mut menu_ids),
        &muda::PredefinedMenuItem::separator(), &muda::PredefinedMenuItem::quit(Some("Exit")),
    ]).unwrap();
    let edit = muda::Submenu::with_items("&Edit", true, &[
        &muda::PredefinedMenuItem::undo(None), &muda::PredefinedMenuItem::redo(None), &muda::PredefinedMenuItem::separator(),
        &muda::PredefinedMenuItem::cut(None), &muda::PredefinedMenuItem::copy(None), &muda::PredefinedMenuItem::paste(None), &muda::PredefinedMenuItem::select_all(None),
        &muda::PredefinedMenuItem::separator(), &item("Search Mail", Some("CmdOrCtrl+F"), "search", &mut menu_ids),
    ]).unwrap();
    let help = muda::Submenu::with_items("&Help", true, &[
        &item("Check for Updates…", None, "update", &mut menu_ids), &item("About The Stupid Mail", None, "about", &mut menu_ids),
    ]).unwrap();
    menu.append_items(&[&file, &edit, &help]).unwrap();
    #[cfg(windows)] unsafe { use tao::platform::windows::WindowExtWindows; let _ = menu.init_for_hwnd(window.hwnd() as _); }
    #[cfg(target_os = "macos")] menu.init_for_nsapp();
    { let p = proxy.clone(); muda::MenuEvent::set_event_handler(Some(move |e: muda::MenuEvent| { let _ = p.send_event(Ev::Menu(e.id)); })); }

    let ipc_proxy = proxy.clone();
    let view = WebViewBuilder::new()
        .with_custom_protocol("mail".into(), |_id, request| {
            if std::env::var("STUPID_MAIL_DEBUG").is_ok() { eprintln!("[serve] {}", request.uri()); }
            let path = request.uri().path().trim_start_matches('/');
            let path = if path.is_empty() { "index.html" } else { path };
            match WEB.get_file(path) {
                Some(f) => Response::builder().header("Content-Type", mime(path)).header("Cache-Control", "no-store").body(Cow::Borrowed(f.contents())).unwrap(),
                None => Response::builder().status(404).body(Cow::Borrowed(&b"Not found"[..])).unwrap(),
            }
        })
        // The interface talks to the app exactly as on the Mac (window.webkit.messageHandlers.mail).
        .with_initialization_script("(()=>{const real=window.webkit&&window.webkit.messageHandlers;const h={mail:{postMessage:(m)=>window.ipc.postMessage(JSON.stringify(m))}};if(real&&real.ipc)h.ipc=real.ipc;window.webkit={messageHandlers:h}})();document.addEventListener('DOMContentLoaded',()=>{const s=document.createElement('style');s.textContent='.window{min-height:100vh}';document.head.appendChild(s)});")
        .with_ipc_handler(move |req| {
            if std::env::var("STUPID_MAIL_DEBUG").is_ok() { eprintln!("[ipc] {} {}", req.uri(), req.body()); }
            // Only the app's own page may call it (the Mac app checks the frame's URL the same way).
            if req.uri().to_string().starts_with(origin()) { let _ = ipc_proxy.send_event(Ev::Ipc(req.body().clone())); }
        })
        .with_navigation_handler(|url| url.starts_with(origin()) || url == "about:blank")
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .with_url(format!("{}/index.html", origin()))
        .build(&window).expect("webview");

    let mut app = App { proxy: proxy.clone(), window, view, dialogs: HashMap::new(), next_dialog: 0, queue: vec![], workspaces: HashMap::new(),
        mail_busy: Arc::new(AtomicBool::new(false)), buddy_busy: Arc::new(AtomicBool::new(false)), menu_ids,
        workspace_menu: HashMap::new(), workspace_menus: vec![], exit: false };
    update::check(&app, false);
    // Development self-test: STUPID_MAIL_TEST_JS=<file> runs in the interface after it loads; it can report
    // with window.webkit.messageHandlers.mail.postMessage({action:'__log', id:'t', text}).
    if let Ok(file) = std::env::var("STUPID_MAIL_TEST_JS") {
        let code = std::fs::read_to_string(file).unwrap_or_default();
        app.background(|| std::thread::sleep(std::time::Duration::from_secs(3)), move |app, _| app.js(&code));
    }

    event_loop.run(move |event, target, flow| {
        *flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(Ev::Ipc(body)) => bridge::handle(&mut app, &body),
            Event::UserEvent(Ev::Run(f)) => f(&mut app),
            Event::UserEvent(Ev::Dialog(id, button, values)) => {
                if let Some(DialogEntry { done: Some(done), .. }) = app.dialogs.remove(&id) { done(&mut app, button, values); }
            }
            Event::UserEvent(Ev::Menu(id)) => match app.menu_ids.get(&id).copied() {
                Some("connect") => app.js("document.getElementById('connect-account').click()"),
                Some("compose") => app.js("if(!document.querySelector('dialog[open]'))document.getElementById('compose').click()"),
                Some("search") => app.js("if(!document.querySelector('dialog[open]'))document.getElementById('search').focus()"),
                Some("update") => update::check(&app, true),
                Some("about") => app.notice(&format!("The Stupid Mail {VERSION}\n\nQuiet Inbox • Follow-up Radar • AI Buddy\n\niCloud, Gmail, and Outlook inbox and sending. Sorting, follow-up reminders, and drafts stay on this computer. Latest 40 inbox messages. Attachments are not supported yet.")),
                _ => if let Some((p, k)) = app.workspace_menu.get(&id).cloned() { bridge::workspace_menu(&mut app, &p, k); },
            },
            Event::WindowEvent { window_id, event: WindowEvent::CloseRequested, .. } => close_requested(&mut app, window_id),
            _ => {}
        }
        // Windows asked for while handling the event (they need the loop's window target).
        for open in std::mem::take(&mut app.queue) { open_window(&mut app, target, open); }
        if app.exit { *flow = ControlFlow::Exit; }
    });
}

fn close_requested(app: &mut App, id: WindowId) {
    if let Some(d) = app.dialogs.iter().find(|(_, e)| e.win.as_ref().map(|(w, _)| w.id() == id).unwrap_or(false)).map(|(k, _)| *k) {
        // Closing a dialog's window counts as cancelling it.
        if let Some(DialogEntry { done: Some(done), .. }) = app.dialogs.remove(&d) { done(app, -1, Map::new()); }
        return;
    }
    if let Some(p) = app.workspaces.iter().find(|(_, (w, _))| w.id() == id).map(|(k, _)| k.clone()) { app.workspaces.remove(&p); return; }
    if id != app.window.id() { return; }
    if app.mail_busy.load(Ordering::SeqCst) || app.buddy_busy.load(Ordering::SeqCst) {
        app.notice("Please wait for the current mail or Buddy request to finish before closing. This avoids interrupting an email that may be sending.");
        return;
    }
    // Save the draft first, like the Mac app.
    app.js("(async()=>{let ok=false;try{ok=window.flushDraft?await window.flushDraft():true}catch(e){}window.ipc.postMessage(JSON.stringify({action:'__flushed',id:'quit',ok:!!ok}))})()");
}
pub fn quit(app: &mut App, saved: bool) {
    if saved { app.exit = true; } else { app.notice("Your latest draft could not be saved. Keep the app open and check free disk space."); }
}

fn open_window(app: &mut App, target: &EventLoopWindowTarget<Ev>, open: Open) {
    match open {
        Open::Dialog(id, spec) => {
            let Some(slot) = app.dialogs.get_mut(&id) else { return };
            let w = WindowBuilder::new().with_title(&spec.title).with_inner_size(LogicalSize::new(460.0, spec.height())).with_resizable(false).with_always_on_top(true).build(target).expect("dialog");
            let p = app.proxy.clone();
            let v = WebViewBuilder::new().with_html(dialog::html(&spec))
                .with_ipc_handler(move |req| {
                    let reply: Value = serde_json::from_str(req.body()).unwrap_or_default();
                    let values = reply["values"].as_object().cloned().unwrap_or_default();
                    let _ = p.send_event(Ev::Dialog(id, reply["button"].as_i64().unwrap_or(-1), values));
                })
                .with_navigation_handler(|url| url.starts_with("about:") || url.starts_with("data:"))
                .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
                .build(&w).expect("dialog view");
            slot.win = Some((w, v));
        }
        Open::Workspace(provider) => {
            if let Some((w, _)) = app.workspaces.get(&provider) { w.set_focus(); return; }
            let url = if provider == "slack" { "https://app.slack.com/client/" } else { "https://teams.microsoft.com/v2/" };
            let title = if provider == "slack" { "Slack — The Stupid Mail" } else { "Teams — The Stupid Mail" };
            let w = WindowBuilder::new().with_title(title).with_inner_size(LogicalSize::new(1100.0, 780.0)).build(target).expect("workspace");
            let v = bridge::workspace_view(app, &provider, url, &w);
            app.workspaces.insert(provider, (w, v));
        }
    }
}
