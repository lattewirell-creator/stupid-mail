// Local storage for the Windows version (macos/MailCache.swift, the Keychain stores and UserDefaults):
// %APPDATA%\The Stupid Mail\  settings.json, Accounts\<sha256 of address>.json (mail cache, drafts, sorting),
// and secrets\*.bin: passwords, tokens, API keys and webhook URLs encrypted with Windows' per-user data
// protection (DPAPI), so only this Windows account can read them. Passwords are never in the web view.
use crate::mail::{fail, validation, MailFailure, Result};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

pub fn base() -> PathBuf {
    let root = std::env::var_os("STUPID_MAIL_DATA").map(PathBuf::from).unwrap_or_else(|| {
        #[cfg(windows)] { PathBuf::from(std::env::var_os("APPDATA").unwrap_or_default()).join("The Stupid Mail") }
        #[cfg(not(windows))] { PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join("Library/Application Support/The Stupid Mail (Windows build)") }
    });
    fs::create_dir_all(&root).ok();
    root
}

/// Write via a temporary file and rename, so a crash never leaves half a file.
fn atomic_write(path: &PathBuf, data: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)
}

/* ---------------- settings (UserDefaults) ---------------- */
pub fn settings() -> Map<String, Value> {
    fs::read(base().join("settings.json")).ok().and_then(|d| serde_json::from_slice(&d).ok()).unwrap_or_default()
}
pub fn setting(key: &str) -> String { settings().get(key).and_then(Value::as_str).unwrap_or("").to_string() }
pub fn set_setting(key: &str, value: Option<&str>) {
    let mut s = settings();
    match value { Some(v) => { s.insert(key.into(), Value::String(v.into())); } None => { s.remove(key); } }
    let _ = atomic_write(&base().join("settings.json"), &serde_json::to_vec_pretty(&s).unwrap_or_default());
}

/* ---------------- mail cache ---------------- */
fn cache_path(account: &str) -> Result<PathBuf> {
    if !(account.is_empty() || validation::address(account)) { return fail("Invalid local account."); }
    let dir = base().join("Accounts");
    fs::create_dir_all(&dir).map_err(|_| MailFailure("Could not create the local mailbox folder.".into()))?;
    let name = if account.is_empty() { "demo".to_string() } else { account.to_lowercase() };
    let hash: String = Sha256::digest(name.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    Ok(dir.join(format!("{hash}.json")))
}
fn decode_cache(data: &[u8]) -> Result<Value> {
    if data.len() > 80 * 1024 * 1024 { return fail("Invalid local mailbox data."); }
    match serde_json::from_slice::<Value>(data) { Ok(v @ Value::Array(_)) => Ok(v), _ => fail("Invalid local mailbox data.") }
}
pub fn read_cache(account: &str) -> Result<Value> {
    let path = cache_path(account)?;
    match fs::read(&path) { Ok(d) => decode_cache(&d), Err(_) => Ok(Value::Null) }
}
pub fn write_cache(account: &str, contents: &str) -> Result<()> {
    decode_cache(contents.as_bytes())?;
    atomic_write(&cache_path(account)?, contents.as_bytes()).map_err(|_| MailFailure("Could not save your mailbox on this computer. Check free disk space before closing the app.".into()))
}

/* ---------------- secrets ---------------- */
fn secret_path(name: &str) -> PathBuf {
    let dir = base().join("secrets");
    fs::create_dir_all(&dir).ok();
    dir.join(format!("{name}.bin"))
}
pub fn save_secret(name: &str, value: &Value) -> Result<()> {
    let plain = serde_json::to_vec(value).map_err(|_| MailFailure("Could not save this securely.".into()))?;
    let sealed = protect(&plain).ok_or_else(|| MailFailure("Windows could not protect this secret. Nothing was saved.".into()))?;
    atomic_write(&secret_path(name), &sealed).map_err(|_| MailFailure("Could not save this securely. Check free disk space.".into()))
}
pub fn load_secret(name: &str) -> Option<Value> {
    let sealed = fs::read(secret_path(name)).ok()?;
    serde_json::from_slice(&unprotect(&sealed)?).ok()
}
pub fn remove_secret(name: &str) -> Result<()> {
    match fs::remove_file(secret_path(name)) { Ok(_) => Ok(()), Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()), Err(_) => fail("Could not remove the saved secret.") }
}

#[cfg(windows)]
fn protect(data: &[u8]) -> Option<Vec<u8>> { dpapi(data, true) }
#[cfg(windows)]
fn unprotect(data: &[u8]) -> Option<Vec<u8>> { dpapi(data, false) }
#[cfg(windows)]
fn dpapi(data: &[u8], seal: bool) -> Option<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};
    let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
    let ok = unsafe {
        if seal { CryptProtectData(&input, std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) }
        else { CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) }
    };
    if ok == 0 || output.pbData.is_null() { return None; }
    let result = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe { LocalFree(output.pbData as _) };
    Some(result)
}
// Development builds on a Mac (the Windows app's logic is tested there): files readable only by this user.
#[cfg(not(windows))]
fn protect(data: &[u8]) -> Option<Vec<u8>> { Some(data.to_vec()) }
#[cfg(not(windows))]
fn unprotect(data: &[u8]) -> Option<Vec<u8>> { Some(data.to_vec()) }
