// The interface's requests (port of macos/NativeBridge.swift, AccountSetup.swift, CalendarBridge.swift and
// Workspace.swift): same actions, same checks, same wording, Windows dialogs instead of macOS sheets.
use crate::dialog::{Field, Spec};
use crate::mail::{self, validation, Credentials, Provider};
use crate::{calendar, net, store, App};
use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn s<'a>(r: &'a Value, k: &str) -> &'a str { r[k].as_str().unwrap_or("") }
fn val(m: &Map<String, Value>, k: &str) -> String { m.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string() }
fn records(list: Vec<mail::MailRecord>) -> Value { serde_json::to_value(list).unwrap_or(Value::Array(vec![])) }
fn load_credentials() -> Option<Credentials> { store::load_secret("mail").and_then(|v| serde_json::from_value(v).ok()) }
pub fn open_external(url: &str) {
    if !url.starts_with("https://") { return; }
    #[cfg(windows)] { let _ = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn(); }
    #[cfg(not(windows))] { let _ = std::process::Command::new("open").arg(url).spawn(); }
}

pub fn handle(app: &mut App, body: &str) {
    let Ok(r) = serde_json::from_str::<Value>(body) else { return };
    let (id, action) = (s(&r, "id").to_string(), s(&r, "action").to_string());
    if id.is_empty() { return; }
    match action.as_str() {
        "__flushed" => crate::quit(app, r["ok"].as_bool().unwrap_or(false)),
        "__log" if std::env::var("STUPID_MAIL_TEST_JS").is_ok() => println!("[test] {}", s(&r, "text")),
        "status" => {
            let email = store::setting("icloudEmail");
            let provider = { let p = store::setting("mailProvider"); if p.is_empty() { "icloud".to_string() } else { p } };
            app.respond(&id, Ok(json!({"email": email, "connected": !email.is_empty(), "provider": provider})));
        }
        "readCache" => { let account = s(&r, "account").to_string(); app.background(move || store::read_cache(&account), move |app, res| app.respond(&id, res.map(|v| json!({"messages": v})).map_err(|_| "Could not read the local mailbox. Your saved files have not been changed.".into()))); }
        "writeCache" => {
            let (account, Some(contents)) = (s(&r, "account").to_string(), r["contents"].as_str().map(String::from)) else { app.respond(&id, Err("Missing mailbox data.".into())); return };
            app.background(move || store::write_cache(&account, &contents), move |app, res| app.respond(&id, res.map(|_| json!({})).map_err(|e| e.0)));
        }
        "connect" => connect(app, id),
        "disconnect" => disconnect(app, id),
        "sync" => run_mail(app, id, |c| {
            let list = if c.kind() == Provider::Outlook { net::outlook_sync(&c)? } else { mail::sync(&c)? };
            Ok(json!({"email": c.email, "provider": c.kind().raw(), "messages": records(list)}))
        }),
        "send" => confirm_send(app, id, r),
        "calendarSuggest" => {
            let mut out = vec![];
            'outer: for item in r["items"].as_array().cloned().unwrap_or_default().iter().take(40) {
                for mut sug in calendar::suggestions(s(item, "text"), item["title"].as_str().unwrap_or("Meeting"), chrono::Local::now()) {
                    sug["source"] = Value::String(s(item, "source").chars().take(100).collect());
                    out.push(sug);
                    if out.len() == 30 { break 'outer; }
                }
            }
            app.respond(&id, Ok(json!({"suggestions": out})));
        }
        // No shared calendar store on Windows: events go to your calendar app as an .ics file.
        "calendarConnect" | "calendarList" => app.respond(&id, Ok(json!({"calendars": [{"id": "ics", "title": "Your calendar app (Outlook, Windows Calendar…)"}], "events": []}))),
        "calendarCreate" => calendar_create(app, id, r),
        "workspaceOpen" | "workspaceSettings" | "workspaceShare" => workspace(app, id, &action, r),
        "buddyStatus" => app.respond(&id, Ok(buddy_status())),
        "buddySettings" => buddy_settings(app, id),
        "buddyAsk" => buddy_ask(app, id, r),
        _ => app.respond(&id, Err("Unknown action.".into())),
    }
}

/// Mail work on a worker thread with the saved account (refreshing Outlook sign-in first).
fn run_mail(app: &mut App, id: String, work: impl FnOnce(Credentials) -> mail::Result<Value> + Send + 'static) {
    if app.mail_busy.swap(true, Ordering::SeqCst) { app.respond(&id, Err("A mail operation is already running. Please wait.".into())); return; }
    app.background(move || {
        let mut c = load_credentials().ok_or_else(|| mail::MailFailure("Connect your mail account first.".into()))?;
        if c.kind() == Provider::Outlook { c = net::refreshed(c)?; }
        work(c)
    }, move |app, res| { app.mail_busy.store(false, Ordering::SeqCst); app.respond(&id, res.map_err(|e| e.0)); });
}
fn busy(app: &App) -> bool { app.mail_busy.load(Ordering::SeqCst) || app.dialog_open() }

fn connect(app: &mut App, id: String) {
    if busy(app) { app.respond(&id, Err("Finish the current operation first.".into())); return; }
    let spec = Spec::new("Connect your mail", "Choose your provider. One account is active at a time; connecting another replaces the saved sign-in. Drafts and sorting stay separate for each email address.", &["Continue", "Cancel"])
        .field(Field::choice("provider", &["iCloud — Apple app password", "Gmail — Google app password", "Outlook — Microsoft sign-in"], 0));
    app.dialog(spec, move |app, button, v| {
        if button != 0 { app.respond(&id, Err("Cancelled.".into())); return; }
        match val(&v, "provider").as_str() { "2" => connect_outlook(app, id), "1" => connect_password(app, id, Provider::Gmail), _ => connect_password(app, id, Provider::Icloud) }
    });
}
fn connect_password(app: &mut App, id: String, provider: Provider) {
    let text = if provider == Provider::Icloud { "Use your primary iCloud address and an Apple app-specific password from account.apple.com. Your password is stored encrypted for your Windows account. Latest 40 inbox messages; local sorting and drafts." }
        else { "Use your full Gmail or Google Workspace address and a 16-letter Google app password. Enable 2-Step Verification, then create one at myaccount.google.com/apppasswords. Some work/school or protected accounts do not allow app passwords. Your password is stored encrypted for your Windows account." };
    let spec = Spec::new(&format!("Connect {} Mail", provider.title()), &format!("{text}\n\nUse an app password, not your normal account password."), &["Connect", "Cancel", "Create app password"])
        .field(Field::text("email", if provider == Provider::Icloud { "you@icloud.com" } else { "you@gmail.com" }, &store::setting("icloudEmail")))
        .field(Field::secret("password", "App-specific password (xxxx-xxxx-xxxx-xxxx)"));
    app.dialog(spec, move |app, button, v| {
        if button == 2 { open_external(if provider == Provider::Icloud { "https://account.apple.com/" } else { "https://myaccount.google.com/apppasswords" }); app.respond(&id, Err("Create an app-specific password, then connect again.".into())); return; }
        if button != 0 { app.respond(&id, Err("Cancelled.".into())); return; }
        let address = val(&v, "email").to_lowercase();
        let secret = val(&v, "password").replace(' ', "");
        let ok_address = if provider == Provider::Icloud { validation::icloud_address(&address) } else { validation::address(&address) };
        if !ok_address || !validation::app_password(&secret) { app.respond(&id, Err("Check your address and use a 16-letter app password from your provider.".into())); return; }
        app.mail_busy.store(true, Ordering::SeqCst);
        app.background(move || {
            let c = Credentials { email: address.clone(), password: secret, provider: Some(provider), client_id: None, refresh_token: None, expires_at: None };
            let list = mail::sync(&c)?;
            store::save_secret("mail", &serde_json::to_value(&c).unwrap())?;
            store::set_setting("icloudEmail", Some(&address)); store::set_setting("mailProvider", Some(provider.raw()));
            Ok::<_, mail::MailFailure>(json!({"email": address, "provider": provider.raw(), "messages": records(list)}))
        }, move |app, res| { app.mail_busy.store(false, Ordering::SeqCst); app.respond(&id, res.map_err(|e| e.0)); });
    });
}
fn connect_outlook(app: &mut App, id: String) {
    let spec = Spec::new("Connect Outlook", "Sign in with Microsoft in your browser. This build needs an application client ID from a Microsoft app registration. No client secret or Microsoft password goes here.\n\nRegistration: personal + work/school accounts, public client flows enabled, and delegated User.Read, Mail.Read, Mail.Send permissions. See the setup guide for steps.", &["Sign in", "Cancel", "Setup guide"])
        .field(Field::text("client", "Microsoft application (client) ID", &store::setting("microsoftClientID")));
    app.dialog(spec, move |app, button, v| {
        if button == 2 { open_external("https://github.com/lattewirell-creator/stupid-mail/blob/main/docs/accounts.md"); app.respond(&id, Err("Follow the Outlook setup guide, then connect again.".into())); return; }
        if button != 0 { app.respond(&id, Err("Cancelled.".into())); return; }
        let client = val(&v, "client");
        if uuid::Uuid::parse_str(&client).is_err() { app.respond(&id, Err("Enter the application client ID from your Microsoft app registration.".into())); return; }
        store::set_setting("microsoftClientID", Some(&client));
        app.mail_busy.store(true, Ordering::SeqCst);
        let c2 = client.clone();
        app.background(move || net::auth("devicecode", &[("client_id", &c2), ("scope", net::SCOPE)]), move |app, res| {
            let device = match res { Ok(d) => d, Err(e) => { app.mail_busy.store(false, Ordering::SeqCst); app.respond(&id, Err(e.0)); return; } };
            let (Some(code), Some(secret)) = (device["user_code"].as_str().map(String::from), device["device_code"].as_str().map(String::from)) else {
                app.mail_busy.store(false, Ordering::SeqCst); app.respond(&id, Err("Microsoft could not start sign-in. Check the client ID and enable public client flows in the registration.".into())); return;
            };
            let cancelled = Arc::new(AtomicBool::new(false));
            let c3 = cancelled.clone();
            let prompt = app.dialog(Spec::new("Finish Microsoft sign-in", "Open microsoft.com/devicelogin and enter this code. Approve access only for your The Stupid Mail registration. This window closes when sign-in finishes.", &["Cancel sign-in"]).field(Field::readonly("code", &code)),
                move |_, _, _| c3.store(true, Ordering::SeqCst));
            open_external("https://microsoft.com/devicelogin");
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(device["expires_in"].as_u64().unwrap_or(900).min(900));
            let mut interval = device["interval"].as_u64().unwrap_or(5).max(5);
            let c4 = cancelled.clone();
            app.background(move || -> mail::Result<(Credentials, Value)> {
                let mut provisional = None;
                while std::time::Instant::now() < deadline && !c4.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_secs(interval));
                    if c4.load(Ordering::SeqCst) { break; }
                    let token = net::auth("token", &[("client_id", &client), ("device_code", &secret), ("grant_type", "urn:ietf:params:oauth:grant-type:device_code")])?;
                    match token["error"].as_str() { Some("authorization_pending") => continue, Some("slow_down") => { interval += 5; continue } _ => {} }
                    provisional = Some(net::credentials(&token, "", &client, None)?); break;
                }
                let p = provisional.filter(|_| !c4.load(Ordering::SeqCst)).ok_or_else(|| mail::MailFailure("Microsoft sign-in was cancelled or timed out. Connect again when ready.".into()))?;
                let email = net::outlook_profile_email(&p.password)?;
                let c = Credentials { email, ..p };
                let list = net::outlook_sync(&c)?;
                Ok((c, records(list)))
            }, move |app, res| {
                app.close_dialog(prompt);
                app.mail_busy.store(false, Ordering::SeqCst);
                let result = res.and_then(|(c, messages)| {
                    if cancelled.load(Ordering::SeqCst) { return Err(mail::MailFailure("Cancelled.".into())); }
                    store::save_secret("mail", &serde_json::to_value(&c).unwrap())?;
                    store::set_setting("icloudEmail", Some(&c.email)); store::set_setting("mailProvider", Some("outlook"));
                    Ok(json!({"email": c.email, "provider": "outlook", "messages": messages}))
                });
                app.respond(&id, result.map_err(|e| e.0));
            });
        });
    });
}
fn disconnect(app: &mut App, id: String) {
    if busy(app) { app.respond(&id, Err("Wait for the current operation to finish.".into())); return; }
    app.dialog(Spec::new("Disconnect this account?", "This removes the saved sign-in from this computer. Your mail on the server is unchanged. Local drafts and sorting are kept for when you reconnect.", &["Disconnect", "Cancel"]), move |app, button, _| {
        if button != 0 { app.respond(&id, Err("Cancelled.".into())); return; }
        let r = store::remove_secret("mail").map(|_| { store::set_setting("icloudEmail", None); store::set_setting("mailProvider", None); json!({}) });
        app.respond(&id, r.map_err(|e| e.0));
    });
}
fn confirm_send(app: &mut App, id: String, r: Value) {
    if busy(app) { app.respond(&id, Err("Finish the current operation first.".into())); return; }
    let (to, subject, body) = (s(&r, "to").to_string(), s(&r, "subject").to_string(), s(&r, "body").to_string());
    if !validation::address(&to) || subject.len() > 900 || body.len() > 1024 * 1024 || r["subject"].as_str().is_none() || r["body"].as_str().is_none() { app.respond(&id, Err("Check the recipient and message length.".into())); return; }
    let reply_to = r["replyTo"].as_str().map(String::from);
    app.dialog(Spec::new("Send this email?", &format!("To: {to}\nSubject: {subject}\n\nThis will send a real email from your connected mail account."), &["Send", "Cancel"]), move |app, button, _| {
        if button != 0 { app.respond(&id, Err("Cancelled.".into())); return; }
        run_mail(app, id, move |c| {
            let message_id = if c.kind() == Provider::Outlook { net::outlook_send(&c, &to, &subject, &body, reply_to.as_deref())? } else { mail::send(&c, &to, &subject, &body, reply_to.as_deref())? };
            Ok(json!({"sent": true, "messageID": message_id}))
        });
    });
}

/* ---------------- Buddy ---------------- */
fn buddy() -> Option<(String, String, String)> {
    let v = store::load_secret("buddy")?;
    Some((v["provider"].as_str()?.to_string(), v["model"].as_str()?.to_string(), v["key"].as_str().unwrap_or("").to_string()))
}
fn buddy_status() -> Value { match buddy() { Some((p, m, _)) => json!({"ready": true, "provider": p, "model": m}), None => json!({"ready": false, "provider": "", "model": ""}) } }
fn buddy_settings(app: &mut App, id: String) {
    if app.dialog_open() { app.respond(&id, Err("Close the current dialog first.".into())); return; }
    let current = buddy();
    let names = ["ollama", "openai", "anthropic"];
    let selected = current.as_ref().and_then(|c| names.iter().position(|n| *n == c.0)).unwrap_or(0);
    let spec = Spec::new("Wake up Buddy", "Choose an AI and enter its model name. Cloud providers receive your questions, plus the selected email only when you enable ‘Include this email’. API usage may be billed by that provider. Ollama runs locally; use a locally installed model.\n\nKeys are saved encrypted for your Windows account. Existing browser keys are not imported.", &["Save", "Cancel"])
        .field(Field::choice("provider", &["Ollama — local", "OpenAI", "Claude — Anthropic"], selected))
        .field(Field::text("model", "Exact model name from your provider", current.as_ref().map(|c| c.1.as_str()).unwrap_or("")))
        .field(Field::secret("key", "API key (not needed for Ollama)"));
    app.dialog(spec, move |app, button, v| {
        if button != 0 { app.respond(&id, Err("Cancelled.".into())); return; }
        let p = names[val(&v, "provider").parse::<usize>().unwrap_or(0).min(2)];
        let m = val(&v, "model");
        let typed = val(&v, "key");
        let key = if typed.is_empty() && current.as_ref().map(|c| c.0 == p).unwrap_or(false) { current.map(|c| c.2).unwrap_or_default() } else { typed };
        let model_ok = !m.is_empty() && m.chars().count() <= 160 && m.chars().all(|c| c.is_ascii_alphanumeric() || "._:/-".contains(c));
        if !model_ok || !(p == "ollama" || (!key.is_empty() && key.len() <= 512 && !key.contains('\n'))) { app.respond(&id, Err("Enter a model name and, for cloud providers, an API key.".into())); return; }
        let r = store::save_secret("buddy", &json!({"provider": p, "model": m, "key": key})).map(|_| buddy_status());
        app.respond(&id, r.map_err(|e| e.0));
    });
}
fn buddy_ask(app: &mut App, id: String, r: Value) {
    if app.buddy_busy.load(Ordering::SeqCst) { app.respond(&id, Err("Buddy is already thinking.".into())); return; }
    let (Some((p, m, k)), question) = (buddy(), s(&r, "question").to_string()) else { app.respond(&id, Err("Open Buddy settings and choose an AI first.".into())); return };
    if question.is_empty() { app.respond(&id, Err("Open Buddy settings and choose an AI first.".into())); return; }
    let context = s(&r, "context").to_string();
    app.buddy_busy.store(true, Ordering::SeqCst);
    app.background(move || net::buddy_ask(&p, &m, &k, &question, &context), move |app, res| { app.buddy_busy.store(false, Ordering::SeqCst); app.respond(&id, res.map(|a| json!({"answer": a})).map_err(|e| e.0)); });
}

/* ---------------- calendar ---------------- */
fn calendar_create(app: &mut App, id: String, r: Value) {
    if busy(app) { app.respond(&id, Err("Finish the current operation first.".into())); return; }
    let title = s(&r, "title").trim().to_string();
    let (Some(start), Some(end)) = (calendar::parse_iso(s(&r, "start")), calendar::parse_iso(s(&r, "end"))) else { app.respond(&id, Err("Check the title, times, and destination calendar.".into())); return };
    if title.is_empty() || title.chars().count() > 200 || end <= start || (end - start).num_seconds() > 7 * 86400 { app.respond(&id, Err("Check the title, times, and destination calendar.".into())); return; }
    let local = |d: chrono::DateTime<chrono::Utc>| d.with_timezone(&chrono::Local).format("%a %d %b %Y, %H:%M").to_string();
    app.dialog(Spec::new("Add this event?", &format!("{title}\n{} – {}\n\nYour calendar app opens with this event for you to save. No invitation or email body is included.", local(start), local(end)), &["Add event", "Cancel"]), move |app, button, _| {
        if button != 0 { app.respond(&id, Err("Cancelled.".into())); return; }
        let path = std::env::temp_dir().join(format!("stupid-mail-{}.ics", uuid::Uuid::new_v4()));
        if std::fs::write(&path, calendar::ics(&title, start, end)).is_err() { app.respond(&id, Err("Could not prepare the event file.".into())); return; }
        #[cfg(windows)] let opened = std::process::Command::new("rundll32").arg("url.dll,FileProtocolHandler").arg(&path).spawn().is_ok();
        #[cfg(not(windows))] let opened = std::process::Command::new("open").arg(&path).spawn().is_ok();
        app.respond(&id, if opened { Ok(json!({"saved": true})) } else { Err("No calendar app opened the event. Check that a calendar app is installed.".into()) });
    });
}

/* ---------------- Slack / Teams ---------------- */
fn workspace(app: &mut App, id: String, action: &str, r: Value) {
    let provider = s(&r, "provider").to_string();
    if provider != "slack" && provider != "teams" { app.respond(&id, Err("Choose Slack or Teams.".into())); return; }
    let name = if provider == "slack" { "Slack" } else { "Teams" };
    let key = format!("workspace-{provider}");
    match action {
        "workspaceOpen" => { app.open_workspace(&provider); app.respond(&id, Ok(json!({}))); }
        "workspaceSettings" => {
            if app.dialog_open() { app.respond(&id, Err("Close the current dialog first.".into())); return; }
            let text = if provider == "slack" { "Create a Slack Incoming Webhook for your chosen channel. Give it a recognizable destination label and paste the secret webhook URL below. The URL is stored encrypted. Chat sign-in is separate." }
                else { "Create a Teams Workflow using ‘When a Teams webhook request is received’ and a post-card action. Use a signed URL that accepts unauthenticated webhook calls. Tenant-authenticated workflows are not supported. The URL is stored encrypted. Chat sign-in is separate." };
            let label = store::load_secret(&key).and_then(|v| v["label"].as_str().map(String::from)).unwrap_or_default();
            let spec = Spec::new(&format!("Share emails to {name}"), text, &["Save", "Cancel", "Remove connection"]).field(Field::text("label", "Destination label, e.g. Design team / #inbox", &label)).field(Field::secret("url", "Secret webhook URL"));
            app.dialog(spec, move |app, button, v| {
                if button == 2 { let r = store::remove_secret(&key).map(|_| json!({})); app.respond(&id, r.map_err(|e| e.0)); return; }
                if button != 0 { app.respond(&id, Err("Cancelled.".into())); return; }
                let (label, url) = (val(&v, "label"), val(&v, "url"));
                if label.is_empty() || label.chars().count() > 100 || !net::webhook_valid(&url, &provider) { app.respond(&id, Err(format!("Enter a destination label and a valid {} webhook URL. See docs/workspaces.md for setup.", if provider == "slack" { "Slack" } else { "Teams" }))); return; }
                let r = store::save_secret(&key, &json!({"provider": provider, "label": label, "webhook": url})).map(|_| json!({}));
                app.respond(&id, r.map_err(|_| "Could not save the workspace connection.".into()));
            });
        }
        _ => {
            if busy(app) { app.respond(&id, Err("Finish the current operation first.".into())); return; }
            let text = s(&r, "text").to_string();
            let dest = store::load_secret(&key);
            let (Some(label), Some(hook)) = (dest.as_ref().and_then(|d| d["label"].as_str().map(String::from)), dest.as_ref().and_then(|d| d["webhook"].as_str().map(String::from)).filter(|u| net::webhook_valid(u, &provider))) else {
                app.respond(&id, Err("Set up this workspace’s sharing connection first.".into())); return;
            };
            let payload = match net::webhook_payload(&text, &provider) { Ok(p) => p, Err(e) => { app.respond(&id, Err(e.0)); return; } };
            let spec = Spec::new(&format!("Share to {name}?"), &format!("Destination label: {label}\nThe webhook determines the actual channel/chat. This text will be shared with everyone who can access it. No attachments are included."), &["Share now", "Cancel"]).field(Field::preview("text", &text));
            app.dialog(spec, move |app, button, _| {
                if button != 0 { app.respond(&id, Err("Cancelled.".into())); return; }
                app.mail_busy.store(true, Ordering::SeqCst);
                app.background(move || net::webhook_post(&hook, &payload), move |app, res| {
                    app.mail_busy.store(false, Ordering::SeqCst);
                    app.respond(&id, res.map(|_| json!({"accepted": true})).map_err(|_| "Sharing was not confirmed. Check the destination before retrying to avoid a duplicate. The connection may need updating.".into()));
                });
            });
        }
    }
}
/// A Slack or Teams window. Its menu: open in the browser, or send highlighted chat text to Mail's calendar.
pub fn workspace_view(app: &mut App, provider: &str, url: &str, window: &tao::window::Window) -> wry::WebView {
    let menu = muda::Menu::new();
    let external = muda::MenuItem::new("Open in browser if sign-in is unavailable here", true, None);
    let meeting = muda::MenuItem::new("Find meeting in selected chat text", true, None);
    let _ = menu.append_items(&[&muda::Submenu::with_items("&Workspace", true, &[&external, &meeting]).unwrap()]);
    #[cfg(windows)] unsafe { use tao::platform::windows::WindowExtWindows; let _ = menu.init_for_hwnd(window.hwnd() as _); }
    app.workspace_menu.insert(external.id().clone(), (provider.to_string(), "external"));
    app.workspace_menu.insert(meeting.id().clone(), (provider.to_string(), "meeting"));
    app.workspace_menus.push(menu);
    wry::WebViewBuilder::new().with_url(url)
        .with_navigation_handler(|u| u.starts_with("https://") || u == "about:blank")
        .with_new_window_req_handler(|u, _| { open_external(&u); wry::NewWindowResponse::Deny })
        .build(window).expect("workspace view")
}
/// Workspace menu clicks.
pub fn workspace_menu(app: &mut App, provider: &str, kind: &str) {
    if kind == "external" { open_external(if provider == "slack" { "https://app.slack.com/client/" } else { "https://teams.microsoft.com/v2/" }); return; }
    let Some((_, view)) = app.workspaces.get(provider) else { return };
    let proxy = app.proxy.clone();
    let _ = view.evaluate_script_with_callback("window.getSelection().toString().slice(0,6000)", move |result| {
        let text: String = serde_json::from_str(&result).unwrap_or_default();
        let _ = proxy.send_event(crate::Ev::Run(Box::new(move |app: &mut App| {
            if text.trim().is_empty() { app.notice("Select a message first\n\nHighlight the chat text containing a meeting time, then try again. You can also copy chat text into Mail’s calendar."); return; }
            app.focus_main();
            app.js(&format!("window.receiveMeetingText({})", serde_json::to_string(&text).unwrap()));
        })));
    });
}
