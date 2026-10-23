// Updates for the Windows version: checks GitHub Releases at start and from Help → Check for Updates…, downloads
// StupidMail-<version>-windows-setup.exe and runs it only if its Ed25519 signature (the .sig next to it, made with
// the same key as the Mac app's Sparkle updates) checks out against the public key built into this program.
// On Linux the app comes from a .deb and the system's package manager updates it, so nothing is downloaded here.
use crate::dialog::Spec;
use crate::{store, App, VERSION};
use base64::Engine;
use serde_json::Value;
use std::io::Read;
use std::time::Duration;

const REPO: &str = "lattewirell-creator/stupid-mail";
/// updates/config.json → public_key (the Sparkle Ed25519 key).
const PUBLIC_KEY: &str = "hjzH4UlhdL7xKzZAAPIvZDq0srXbl8sg57VDv/SQ48Q=";

/// Is version a newer than b? ("2.6" vs "2.5.1")
pub fn newer(a: &str, b: &str) -> bool {
    let parse = |v: &str| v.trim_start_matches('v').split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    let (x, y) = (parse(a), parse(b));
    for i in 0..x.len().max(y.len()) { let (p, q) = (*x.get(i).unwrap_or(&0), *y.get(i).unwrap_or(&0)); if p != q { return p > q; } }
    false
}
pub fn verify(data: &[u8], signature_b64: &str) -> bool { verify_with(PUBLIC_KEY, data, signature_b64) }
pub fn verify_with(public_key: &str, data: &[u8], signature_b64: &str) -> bool {
    let b64 = base64::engine::general_purpose::STANDARD;
    let (Ok(key), Ok(sig)) = (b64.decode(public_key), b64.decode(signature_b64.trim())) else { return false };
    let (Ok(key), Ok(sig)) = (<[u8; 32]>::try_from(key.as_slice()), <[u8; 64]>::try_from(sig.as_slice())) else { return false };
    let Ok(key) = ed25519_dalek::VerifyingKey::from_bytes(&key) else { return false };
    key.verify_strict(data, &ed25519_dalek::Signature::from_bytes(&sig)).is_ok()
}

fn agent() -> ureq::Agent { ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(300))).build().into() }
struct Found { version: String, setup: String, sig: String }
fn latest() -> Option<Found> {
    let mut r = agent().get(&format!("https://api.github.com/repos/{REPO}/releases/latest")).header("User-Agent", "TheStupidMail-Updater").header("Accept", "application/vnd.github+json").call().ok()?;
    let rel: Value = serde_json::from_str(&r.body_mut().read_to_string().ok()?).ok()?;
    let version = rel["tag_name"].as_str()?.trim_start_matches('v').to_string();
    let assets = rel["assets"].as_array()?;
    let url = |suffix: &str| assets.iter().find(|a| a["name"].as_str().map(|n| n.ends_with(suffix)).unwrap_or(false)).and_then(|a| a["browser_download_url"].as_str()).map(String::from);
    Some(Found { version, setup: url("-windows-setup.exe")?, sig: url("-windows-setup.exe.sig")? })
}
fn download(url: &str, limit: u64) -> Option<Vec<u8>> {
    if !url.starts_with("https://github.com/") && !url.starts_with("https://objects.githubusercontent.com/") { return None; }
    let r = agent().get(url).header("User-Agent", "TheStupidMail-Updater").call().ok()?;
    let mut data = Vec::new();
    r.into_body().into_reader().take(limit).read_to_end(&mut data).ok()?;
    Some(data)
}

/// Linux: installed and updated by the system (apt, Software Updates), never by the app itself.
pub const MANAGED_BY_SYSTEM: bool = cfg!(target_os = "linux");

/// manual: from the menu (says so when there's nothing new). Automatic checks run once a day at start.
pub fn check(app: &mut App, manual: bool) {
    if MANAGED_BY_SYSTEM {
        if manual { app.notice(&format!("The Stupid Mail {VERSION}\n\nUpdates for this app come from your system: install them with Software Updates (or your package manager) like your other apps.")); }
        return;
    }
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    if !manual && (store::setting("lastUpdateCheck") == today || !cfg!(windows)) { return; }
    store::set_setting("lastUpdateCheck", Some(&today));
    app.background(latest, move |app, found| match found {
        Some(f) if newer(&f.version, VERSION) => {
            let text = format!("The Stupid Mail {} is available (you have {VERSION}). It downloads now and installs when you click Install. Your accounts, drafts and settings stay on this computer.", f.version);
            app.dialog(Spec::new("Update available", &text, &["Install", "Later"]), move |app, button, _| { if button == 0 { install(app, f); } });
        }
        Some(_) if manual => app.notice(&format!("You have the newest version ({VERSION}).")),
        None if manual => app.notice("Couldn’t check for updates. Check your connection and try again later."),
        _ => {}
    });
}
fn install(app: &mut App, f: Found) {
    app.background(move || {
        let setup = download(&f.setup, 200 * 1024 * 1024)?;
        let sig = String::from_utf8(download(&f.sig, 4096)?).ok()?;
        if !verify(&setup, &sig) { return Some(Err("The downloaded update failed its signature check, so it wasn’t installed.")); }
        let path = std::env::temp_dir().join(format!("StupidMail-{}-windows-setup.exe", f.version));
        std::fs::write(&path, &setup).ok()?;
        Some(Ok(path))
    }, |app, res| match res {
        Some(Ok(path)) => {
            if std::process::Command::new(&path).spawn().is_ok() { app.js("window.flushDraft&&window.flushDraft()"); crate::quit(app, true); }
            else { app.notice("The installer couldn’t start. Download the update from GitHub instead."); }
        }
        Some(Err(e)) => app.notice(e),
        None => app.notice("The update couldn’t be downloaded. Try again later."),
    });
}
