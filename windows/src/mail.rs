// iCloud and Gmail: IMAP for the inbox, SMTP for sending (port of macos/MailCore.swift and
// macos/IMAPConnection.swift). Only fixed provider endpoints; TLS certificates and host names are
// validated by the system (SChannel on Windows). MIME is turned into inert text: email HTML is never
// loaded into the web view, so no remote images, scripts, links or tracking pixels are fetched.
use base64::Engine;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug)]
pub struct MailFailure(pub String);
impl std::fmt::Display for MailFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str(&self.0) }
}
impl std::error::Error for MailFailure {}
pub type Result<T> = std::result::Result<T, MailFailure>;
pub fn fail<T>(text: &str) -> Result<T> { Err(MailFailure(text.to_string())) }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider { Icloud, Gmail, Outlook, Yahoo, Aol, Fastmail, Zoho, Gmx, Proton, Custom }
pub const ALL: [Provider; 10] = [Provider::Icloud, Provider::Gmail, Provider::Outlook, Provider::Yahoo, Provider::Aol, Provider::Fastmail, Provider::Zoho, Provider::Gmx, Provider::Proton, Provider::Custom];
impl Provider {
    pub fn title(self) -> &'static str {
        match self { Provider::Icloud => "iCloud", Provider::Gmail => "Gmail", Provider::Outlook => "Outlook", Provider::Yahoo => "Yahoo", Provider::Aol => "AOL",
            Provider::Fastmail => "Fastmail", Provider::Zoho => "Zoho", Provider::Gmx => "GMX", Provider::Proton => "Proton Mail", Provider::Custom => "Other" }
    }
    pub fn raw(self) -> &'static str {
        match self { Provider::Icloud => "icloud", Provider::Gmail => "gmail", Provider::Outlook => "outlook", Provider::Yahoo => "yahoo", Provider::Aol => "aol",
            Provider::Fastmail => "fastmail", Provider::Zoho => "zoho", Provider::Gmx => "gmx", Provider::Proton => "proton", Provider::Custom => "custom" }
    }
    pub fn menu_title(self) -> &'static str {
        match self {
            Provider::Icloud => "iCloud — Apple app password", Provider::Gmail => "Gmail — Google app password", Provider::Outlook => "Outlook — Microsoft sign-in",
            Provider::Yahoo => "Yahoo — app password", Provider::Aol => "AOL — app password", Provider::Fastmail => "Fastmail — app password",
            Provider::Zoho => "Zoho — app-specific password", Provider::Gmx => "GMX — password or app password", Provider::Proton => "Proton Mail — through Proton Mail Bridge",
            Provider::Custom => "Other mail service — enter its servers",
        }
    }
    pub fn password_page(self) -> Option<&'static str> {
        match self {
            Provider::Icloud => Some("https://account.apple.com/"), Provider::Gmail => Some("https://myaccount.google.com/apppasswords"), Provider::Yahoo => Some("https://login.yahoo.com/account/security"),
            Provider::Aol => Some("https://login.aol.com/account/security"), Provider::Fastmail => Some("https://app.fastmail.com/settings/security/apps"), Provider::Zoho => Some("https://accounts.zoho.com/home#security/app_password"),
            Provider::Gmx => Some("https://www.gmx.com/"), Provider::Proton => Some("https://proton.me/mail/bridge"), _ => None,
        }
    }
    pub fn help(self) -> &'static str {
        match self {
            Provider::Icloud => "Use your primary iCloud address and an Apple app-specific password from account.apple.com.",
            Provider::Gmail => "Use your full Gmail or Google Workspace address and a 16-letter Google app password. Enable 2-Step Verification, then create one at myaccount.google.com/apppasswords. Some work/school or protected accounts do not allow app passwords.",
            Provider::Yahoo => "Use your Yahoo address and a 16-letter app password: Yahoo Account → Account security → Generate app password.",
            Provider::Aol => "Use your AOL address and a 16-letter app password: AOL Account → Account security → Generate app password.",
            Provider::Fastmail => "Use your Fastmail address and an app password with Mail (IMAP/SMTP) access: Settings → Privacy & Security → App passwords.",
            Provider::Zoho => "Use your Zoho Mail address and an app-specific password (Zoho Accounts → Security → App passwords). IMAP access must be on in Zoho Mail settings. For zoho.eu or other regions, choose Other mail service.",
            Provider::Gmx => "Turn on POP3 & IMAP access in GMX settings first. Use your GMX address and, if two-factor authentication is on, an app password.",
            Provider::Proton => "Proton only lets other apps in through Proton Mail Bridge (paid Proton plans). Install Bridge, sign in, set its IMAP and SMTP connection mode to SSL (Bridge → Settings → Advanced), and use the Bridge password it shows, not your Proton password. Keep Bridge running.",
            Provider::Custom => "Enter your provider's IMAP (incoming) and SMTP (outgoing) servers. Connections always use encryption (TLS); IMAP needs SSL/TLS, usually port 993.",
            Provider::Outlook => "",
        }
    }
    pub fn valid_password(self, v: &str) -> bool {
        match self {
            Provider::Icloud | Provider::Gmail | Provider::Yahoo | Provider::Aol => validation::app_password(v),
            Provider::Fastmail => v.len() == 16 && v.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            Provider::Zoho => (12..=16).contains(&v.len()) && v.chars().all(|c| c.is_ascii_alphanumeric()),
            _ => (8..=256).contains(&v.chars().count()) && !v.chars().any(|c| (c as u32) < 32 || c as u32 == 127),
        }
    }
}

/// Where a password account's mail lives. Presets are fixed per provider; "Other" uses what you entered.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Server { pub imap_host: String, pub imap_port: u16, pub smtp_host: String, pub smtp_port: u16, #[serde(rename = "smtpImplicitTLS")] pub smtp_implicit_tls: bool }
impl Server {
    fn new(imap: &str, ip: u16, smtp: &str, sp: u16, implicit: bool) -> Server { Server { imap_host: imap.into(), imap_port: ip, smtp_host: smtp.into(), smtp_port: sp, smtp_implicit_tls: implicit } }
    pub fn preset(p: Provider, email: &str) -> Option<Server> {
        let domain = email.rsplit('@').next().unwrap_or("").to_lowercase();
        Some(match p {
            Provider::Icloud => Server::new("imap.mail.me.com", 993, "smtp.mail.me.com", 587, false),
            Provider::Gmail => Server::new("imap.gmail.com", 993, "smtp.gmail.com", 587, false),
            Provider::Yahoo => Server::new("imap.mail.yahoo.com", 993, "smtp.mail.yahoo.com", 465, true),
            Provider::Aol => Server::new("imap.aol.com", 993, "smtp.aol.com", 465, true),
            Provider::Fastmail => Server::new("imap.fastmail.com", 993, "smtp.fastmail.com", 465, true),
            Provider::Zoho => Server::new("imap.zoho.com", 993, "smtp.zoho.com", 465, true),
            Provider::Gmx => { let de = ["gmx.net", "gmx.de", "gmx.at", "gmx.ch"].contains(&domain.as_str());
                Server::new(if de { "imap.gmx.net" } else { "imap.gmx.com" }, 993, if de { "mail.gmx.net" } else { "mail.gmx.com" }, 587, false) }
            Provider::Proton => Server::new("127.0.0.1", 1143, "127.0.0.1", 1025, true),
            Provider::Outlook | Provider::Custom => return None,
        })
    }
    /// A server you typed in: a plain host name and a normal port, nothing else.
    pub fn valid(host: &str, port: u16) -> bool {
        port > 0 && host.len() <= 253 && Regex::new(r"^[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?(\.[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?)+$").unwrap().is_match(host)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub email: String,
    pub password: String,
    pub provider: Option<Provider>,
    #[serde(rename = "clientID")]
    pub client_id: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>, // seconds since 1970
    #[serde(default)]
    pub server: Option<Server>, // only for "Other"
}
impl Credentials {
    pub fn kind(&self) -> Provider { self.provider.unwrap_or(Provider::Icloud) }
    pub fn mail_server(&self) -> Option<Server> { self.server.clone().or_else(|| Server::preset(self.kind(), &self.email)) }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MailRecord {
    pub id: String,
    pub from: String,
    pub email: String,
    pub subject: String,
    pub body: String,
    pub time: String,
    pub unread: bool,
    pub star: bool,
    pub newsletter: bool,
    pub attachment_count: i64,
    #[serde(rename = "messageID")]
    pub message_id: String,
    pub in_reply_to: String,
}

pub mod validation {
    use regex::Regex;
    use std::sync::OnceLock;
    pub fn address(value: &str) -> bool {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r"^[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]+@[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?\.[A-Za-z]{2,}$").unwrap());
        value.len() <= 254 && !value.chars().any(|c| (c as u32) < 33 || c as u32 == 127) && re.is_match(value)
    }
    pub fn icloud_address(value: &str) -> bool {
        address(value) && ["icloud.com", "me.com", "mac.com"].contains(&value.rsplit('@').next().unwrap_or("").to_lowercase().as_str())
    }
    pub fn app_password(value: &str) -> bool {
        static RE: OnceLock<Regex> = OnceLock::new();
        RE.get_or_init(|| Regex::new(r"^[a-z]{4}(-?[a-z]{4}){3}$").unwrap()).is_match(value)
    }
    pub fn quote(value: &str) -> super::Result<String> {
        if value.chars().any(|c| (c as u32) < 32 || c as u32 == 127) { return super::fail("Invalid control character in account or message settings."); }
        Ok(format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"")))
    }
}

pub fn capture(pattern: &str, value: &str) -> Option<String> {
    let re = Regex::new(&format!("(?i){pattern}")).ok()?;
    re.captures(value)?.get(1).map(|m| m.as_str().to_string())
}

/* ---------------- IMAP ---------------- */
/// The connection to the server (TLS in the app; recorded replies in tests).
pub struct Channel<S: Read + Write> { stream: S, buffer: Vec<u8> }
fn tls(host: &str, port: u16) -> Result<native_tls::TlsStream<TcpStream>> {
        let addr = (host, port).to_socket_addrs().ok().and_then(|mut a| a.next())
            .ok_or_else(|| MailFailure("The mail server did not respond. Check your internet connection and try again.".into()))?;
        let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(20))
            .map_err(|_| MailFailure("The mail server did not respond. Check your internet connection and try again.".into()))?;
        tcp.set_read_timeout(Some(Duration::from_secs(30))).ok();
        tcp.set_write_timeout(Some(Duration::from_secs(20))).ok();
        let tls = native_tls::TlsConnector::new().map_err(|_| MailFailure("Could not establish a secure connection to the mail server.".into()))?;
        tls.connect(host, tcp).map_err(|_| MailFailure("Could not establish a secure connection to the mail server.".into()))
}
impl<S: Read + Write> Channel<S> {
    fn receive(&mut self) -> Result<()> {
        let mut chunk = [0u8; 65536];
        match self.stream.read(&mut chunk) {
            Ok(0) => fail("The mail server closed the connection. Please refresh."),
            Ok(n) => { self.buffer.extend_from_slice(&chunk[..n]); Ok(()) }
            Err(_) => fail("The mail server took too long to reply. Refresh to try again."),
        }
    }
    fn line(&mut self) -> Result<Vec<u8>> {
        loop {
            if let Some(i) = self.buffer.windows(2).position(|w| w == b"\r\n") { return Ok(self.buffer.drain(..i + 2).collect()); }
            if self.buffer.len() >= 2 * 1024 * 1024 { return fail("The mail server returned an unusually long response."); }
            self.receive()?;
        }
    }
    fn bytes(&mut self, count: usize) -> Result<Vec<u8>> {
        if count > 2 * 1024 * 1024 { return fail("This message is too large to load."); }
        while self.buffer.len() < count { self.receive()?; }
        Ok(self.buffer.drain(..count).collect())
    }
    fn send(&mut self, data: &[u8]) -> Result<()> {
        self.stream.write_all(data).and_then(|_| self.stream.flush()).map_err(|_| MailFailure("The connection to the mail server was interrupted.".into()))
    }
}

pub struct Imap<S: Read + Write> { channel: Channel<S>, sequence: u32 }
pub fn connect(host: &str, port: u16) -> Result<Imap<native_tls::TlsStream<TcpStream>>> { Imap::new(tls(host, port)?) }
impl<S: Read + Write> Imap<S> {
    pub fn new(stream: S) -> Result<Imap<S>> {
        let mut channel = Channel { stream, buffer: Vec::new() };
        let greeting = String::from_utf8_lossy(&channel.line()?).to_string();
        if !greeting.starts_with("* OK") { return fail("The mail server did not accept the connection."); }
        Ok(Imap { channel, sequence: 0 })
    }
    pub fn command(&mut self, command: &str) -> Result<Vec<u8>> {
        if command.chars().any(|c| (c as u32) < 32 || c as u32 == 127) { return fail("Invalid mail command."); }
        self.sequence += 1;
        let tag = format!("M{}", self.sequence);
        self.channel.send(format!("{tag} {command}\r\n").as_bytes())?;
        let literal = Regex::new(r"\{([0-9]+)\}\r\n$").unwrap();
        let mut response = Vec::new();
        loop {
            let line = self.channel.line()?;
            let text = String::from_utf8_lossy(&line).to_string();
            response.extend_from_slice(&line);
            if response.len() >= 16 * 1024 * 1024 { return fail("The inbox response is too large."); }
            if text.starts_with(&format!("{tag} ")) {
                if !text.starts_with(&format!("{tag} OK")) {
                    return fail(if command.starts_with("LOGIN") { "The mail server rejected the sign-in. Check your email address and app-specific password." } else { "The mail server could not complete this request. Your mailbox was not changed." });
                }
                return Ok(response);
            }
            if text.starts_with("* BYE") { return fail("The mail server ended the session. Please refresh."); }
            if let Some(n) = literal.captures(&text).and_then(|c| c[1].parse::<usize>().ok()) { response.extend(self.channel.bytes(n)?); }
        }
    }
    pub fn authenticate(&mut self, c: &Credentials) -> Result<()> {
        self.command(&format!("LOGIN {} {}", validation::quote(&c.email)?, validation::quote(&c.password)?)).map(|_| ())
    }
}

pub struct Literal { pub prefix: String, pub data: Vec<u8> }
/// The message literals of a FETCH response ({n} followed by n bytes), each with its surrounding text.
pub fn literals(data: &[u8]) -> Result<Vec<Literal>> {
    let size = Regex::new(r"\{([0-9]+)\}$").unwrap();
    let find = |from: usize| data[from..].windows(2).position(|w| w == b"\r\n").map(|i| i + from);
    let (mut cursor, mut out) = (0usize, Vec::new());
    while cursor < data.len() {
        let Some(end) = find(cursor) else { break };
        let line = String::from_utf8_lossy(&data[cursor..end]).to_string();
        cursor = end + 2;
        let Some(count) = size.captures(&line).and_then(|c| c[1].parse::<usize>().ok()) else { continue };
        if count > 2 * 1024 * 1024 || count > data.len() - cursor { return fail("The mail server returned an incomplete message. Please refresh."); }
        let after = cursor + count;
        let tail_end = find(after).unwrap_or(after);
        let tail = String::from_utf8_lossy(&data[after..tail_end]).to_string();
        out.push(Literal { prefix: format!("{line} {tail}"), data: data[cursor..after].to_vec() });
        cursor = after;
    }
    Ok(out)
}
pub fn search(data: &[u8]) -> Vec<u64> {
    let text = String::from_utf8_lossy(data);
    let Some(line) = text.lines().find(|l| l.starts_with("* SEARCH")) else { return vec![] };
    let mut ids: Vec<u64> = line.split(' ').skip(2).filter_map(|s| s.trim().parse().ok()).filter(|&n| n > 0).collect();
    ids.sort();
    ids
}

/* ---------------- MIME ---------------- */
pub fn split(data: &[u8]) -> (std::collections::HashMap<String, String>, Vec<u8>) {
    let find = |pat: &[u8]| data.windows(pat.len()).position(|w| w == pat).map(|i| (i, i + pat.len()));
    let Some((head_end, body_start)) = find(b"\r\n\r\n").or_else(|| find(b"\n\n")) else { return (Default::default(), data.to_vec()) };
    let raw = String::from_utf8_lossy(&data[..head_end]).replace("\r\n", "\n");
    let raw = Regex::new(r"\n[ \t]+").unwrap().replace_all(&raw, " ").to_string();
    let mut headers = std::collections::HashMap::new();
    for line in raw.split('\n') {
        if let Some(colon) = line.find(':') { headers.insert(line[..colon].to_lowercase(), line[colon + 1..].trim().to_string()); }
    }
    (headers, data[body_start..].to_vec())
}
pub fn quoted_printable(data: &[u8], header: bool) -> Vec<u8> {
    let (mut out, mut i) = (Vec::with_capacity(data.len()), 0);
    while i < data.len() {
        if header && data[i] == b'_' { out.push(b' '); i += 1; continue; }
        if data[i] == b'=' {
            if i + 1 < data.len() && data[i + 1] == b'\n' { i += 2; continue; }
            if i + 2 < data.len() && data[i + 1] == b'\r' && data[i + 2] == b'\n' { i += 3; continue; }
            if i + 2 < data.len() {
                if let Ok(b) = u8::from_str_radix(std::str::from_utf8(&data[i + 1..i + 3]).unwrap_or("zz"), 16) { out.push(b); i += 3; continue; }
            }
        }
        out.push(data[i]); i += 1;
    }
    out
}
pub fn decode(data: &[u8], charset: &str) -> String {
    let enc = encoding_rs::Encoding::for_label(charset.trim().as_bytes()).unwrap_or(encoding_rs::UTF_8);
    enc.decode(data).0.into_owned()
}
pub fn header(value: &str) -> String {
    let value = Regex::new(r"\?=[ \t]+=\?").unwrap().replace_all(value, "?==?").to_string();
    let re = Regex::new(r"=\?([^?]+)\?([bBqQ])\?([^?]*)\?=").unwrap();
    re.replace_all(&value, |c: &regex::Captures| {
        let bytes = if c[2].eq_ignore_ascii_case("b") { base64::engine::general_purpose::STANDARD.decode(c[3].trim()).unwrap_or_default() } else { quoted_printable(c[3].as_bytes(), true) };
        decode(&bytes, &c[1])
    }).to_string()
}
pub fn parameter(name: &str, value: &str) -> Option<String> {
    capture(&format!(r#"(?:^|;)\s*{name}\s*=\s*"([^"]+)""#), value).or_else(|| capture(&format!(r"(?:^|;)\s*{name}\s*=\s*([^;\s]+)"), value))
}
pub fn plain_html(html: &str) -> String {
    let mut text = Regex::new(r"(?is)<script\b[^>]*>.*?</script\s*>|<style\b[^>]*>.*?</style\s*>|<head\b[^>]*>.*?</head\s*>").unwrap().replace_all(html, "").to_string();
    text = Regex::new(r"(?i)<(?:br\s*/?|/p|/div|/tr|/h[1-6])\s*>").unwrap().replace_all(&text, "\n").to_string();
    text = Regex::new(r"<[^>]*>").unwrap().replace_all(&text, "").to_string();
    for (entity, with) in [("&nbsp;", " "), ("&lt;", "<"), ("&gt;", ">"), ("&quot;", "\""), ("&#39;", "'"), ("&apos;", "'"), ("&amp;", "&")] { text = text.replace(entity, with); }
    text = Regex::new(r"&#(x[0-9a-fA-F]+|[0-9]+);").unwrap().replace_all(&text, |c: &regex::Captures| {
        let t = &c[1];
        let n = if let Some(h) = t.strip_prefix('x') { u32::from_str_radix(h, 16).ok() } else { t.parse().ok() };
        n.and_then(char::from_u32).map(String::from).unwrap_or_else(|| c[0].to_string())
    }).to_string();
    Regex::new(r"\n{3,}").unwrap().replace_all(&text, "\n\n").trim().to_string()
}
#[derive(Default)]
pub struct Content { pub plain: String, pub html: String, pub attachments: i64 }
pub fn content(data: &[u8], depth: u32) -> Content {
    if depth >= 12 { return Content::default(); }
    let (headers, body) = split(data);
    let ty = headers.get("content-type").cloned().unwrap_or_else(|| "text/plain; charset=utf-8".into());
    let lower = ty.to_lowercase();
    if headers.get("content-disposition").map(|d| d.to_lowercase().contains("attachment")).unwrap_or(false) || parameter("name", &ty).is_some() {
        return Content { attachments: 1, ..Default::default() };
    }
    if lower.starts_with("multipart/") {
        if let Some(boundary) = parameter("boundary", &ty).filter(|b| b.len() < 200) {
            // Latin-1 keeps every byte while splitting on the ASCII MIME delimiters.
            let raw: String = body.iter().map(|&b| b as char).collect();
            let mut merged = Content::default();
            for part in raw.split(&format!("--{boundary}")).skip(1) {
                if part.starts_with("--") { break; }
                let trimmed = part.trim_matches(|c| c == '\r' || c == '\n');
                if trimmed.is_empty() { continue; }
                let bytes: Vec<u8> = trimmed.chars().map(|c| c as u32 as u8).collect();
                let item = content(&bytes, depth + 1);
                if !item.plain.is_empty() { if !merged.plain.is_empty() { merged.plain += "\n\n"; } merged.plain += &item.plain; }
                if !item.html.is_empty() { if !merged.html.is_empty() { merged.html += "\n\n"; } merged.html += &item.html; }
                merged.attachments += item.attachments;
            }
            return merged;
        }
    }
    if !(lower.starts_with("text/plain") || lower.starts_with("text/html")) { return Content { attachments: if body.is_empty() { 0 } else { 1 }, ..Default::default() }; }
    let transfer = headers.get("content-transfer-encoding").map(|t| t.to_lowercase());
    let decoded = match transfer.as_deref() {
        Some("base64") => { let clean: Vec<u8> = body.iter().copied().filter(|b| !b.is_ascii_whitespace()).collect(); base64::engine::general_purpose::STANDARD.decode(clean).unwrap_or_default() }
        Some("quoted-printable") => quoted_printable(&body, false),
        _ => body,
    };
    let text = decode(&decoded, &parameter("charset", &ty).unwrap_or_else(|| "utf-8".into())).trim().to_string();
    if lower.starts_with("text/html") { Content { html: plain_html(&text), ..Default::default() } } else { Content { plain: text, ..Default::default() } }
}
pub fn record(data: &[u8], id: &str, flags: &str, limited: bool) -> MailRecord {
    let (headers, _) = split(data);
    let c = if limited { Content::default() } else { content(data, 0) };
    let from = header(headers.get("from").map(String::as_str).unwrap_or("Unknown sender"));
    let email = capture("<([^>]+)>", &from).unwrap_or_else(|| if validation::address(&from) { from.clone() } else { String::new() });
    let name = from.split('<').next().unwrap_or(&from).trim_matches(|ch| ch == ' ' || ch == '"').to_string();
    let body = if limited { "This message is too large for the current sync. Open it in your provider’s mail app to read the full message and attachments.".to_string() }
        else if !c.plain.is_empty() { c.plain } else if !c.html.is_empty() { c.html }
        else { "No readable text. This message may contain only attachments or encrypted content.".to_string() };
    let flags = flags.to_lowercase();
    MailRecord {
        id: id.into(), from: if name.is_empty() { email.clone() } else { name }, email,
        subject: header(headers.get("subject").map(String::as_str).unwrap_or("(No subject)")), body,
        time: headers.get("date").cloned().unwrap_or_default(), unread: !flags.contains("\\seen"), star: flags.contains("\\flagged"),
        newsletter: headers.contains_key("list-id") || headers.contains_key("list-unsubscribe"), attachment_count: c.attachments,
        message_id: headers.get("message-id").cloned().unwrap_or_default(), in_reply_to: headers.get("in-reply-to").cloned().unwrap_or_default(),
    }
}

/* ---------------- iCloud / Gmail ---------------- */
pub fn sync(c: &Credentials) -> Result<Vec<MailRecord>> {
    if c.kind() == Provider::Outlook { return fail("Outlook requires Microsoft sign-in."); }
    let server = c.mail_server().ok_or_else(|| MailFailure("Connect your mail account again.".into()))?;
    let mut imap = connect(&server.imap_host, server.imap_port)?;
    imap.authenticate(c)?;
    let status = imap.command("EXAMINE INBOX")?;
    let validity = capture(r"UIDVALIDITY\s+([0-9]+)", &String::from_utf8_lossy(&status))
        .ok_or_else(|| MailFailure("The mail server did not return a valid mailbox identifier. Please reconnect.".into()))?;
    let all = search(&imap.command("UID SEARCH ALL")?);
    let ids: Vec<u64> = all[all.len().saturating_sub(40)..].to_vec();
    if ids.is_empty() { return Ok(vec![]); }
    let set = ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
    let heads = imap.command(&format!("UID FETCH {set} (UID FLAGS RFC822.SIZE BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE MESSAGE-ID IN-REPLY-TO LIST-ID LIST-UNSUBSCRIBE)])"))?;
    let (mut records, mut body_ids, mut total) = (std::collections::HashMap::new(), Vec::new(), 0u64);
    let kind = c.kind().raw();
    for lit in literals(&heads)?.into_iter().rev() {
        let Some(uid) = capture(r"UID\s+([0-9]+)", &lit.prefix).and_then(|s| s.parse::<u64>().ok()) else { continue };
        let size = capture(r"RFC822.SIZE\s+([0-9]+)", &lit.prefix).and_then(|s| s.parse::<u64>().ok()).unwrap_or(u64::MAX);
        records.insert(uid, record(&lit.data, &format!("{kind}:{validity}:{uid}"), &lit.prefix, true));
        if size <= 1024 * 1024 && total + size <= 10 * 1024 * 1024 { body_ids.push(uid); total += size; }
    }
    if !body_ids.is_empty() {
        let set = body_ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        let full = imap.command(&format!("UID FETCH {set} (UID FLAGS BODY.PEEK[])"))?;
        for lit in literals(&full)? {
            let Some(uid) = capture(r"UID\s+([0-9]+)", &lit.prefix).and_then(|s| s.parse::<u64>().ok()) else { continue };
            records.insert(uid, record(&lit.data, &format!("{kind}:{validity}:{uid}"), &lit.prefix, false));
        }
    }
    if records.len() != ids.len() { return fail("The inbox changed while syncing. Refresh again to get a complete snapshot."); }
    Ok(ids.iter().rev().filter_map(|id| records.remove(id)).collect())
}

/// The message as sent: plain text, UTF-8, base64 body, RFC 2047 subject folded at word boundaries.
pub fn outgoing(from: &str, to: &str, subject: &str, body: &str, reply_to: Option<&str>) -> Result<Vec<u8>> {
    if !validation::address(from) || !validation::address(to) || subject.chars().any(|c| (c as u32) < 32 || c as u32 == 127) || subject.len() > 900 || body.len() > 1024 * 1024 {
        return fail("Use one valid recipient, a subject under 900 bytes, and a message under 1 MB.");
    }
    let b64 = base64::engine::general_purpose::STANDARD;
    let id = format!("<{}@stupidmail.local>", uuid::Uuid::new_v4().to_string().to_lowercase());
    let (mut words, mut segment) = (Vec::new(), String::new());
    for ch in subject.chars() {
        if segment.len() + ch.len_utf8() > 39 && !segment.is_empty() { words.push(format!("=?UTF-8?B?{}?=", b64.encode(&segment))); segment.clear(); }
        segment.push(ch);
    }
    if !segment.is_empty() { words.push(format!("=?UTF-8?B?{}?=", b64.encode(&segment))); }
    let date = chrono::Local::now().format("%a, %d %b %Y %H:%M:%S %z");
    let mut headers = format!("From: {from}\r\nTo: {to}\r\nSubject: {}\r\nDate: {date}\r\nMessage-ID: {id}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n", words.join("\r\n "));
    if let Some(r) = reply_to {
        if Regex::new(r"^<[^<>\s]+@[^<>\s]+>$").unwrap().is_match(r) && r.chars().count() < 250 { headers += &format!("In-Reply-To: {r}\r\nReferences: {r}\r\n"); }
    }
    let normalized = body.replace("\r\n", "\n").replace('\n', "\r\n");
    let encoded = b64.encode(normalized.as_bytes());
    let lines: Vec<&str> = encoded.as_bytes().chunks(76).map(|c| std::str::from_utf8(c).unwrap()).collect();
    Ok(format!("{headers}\r\n{}\r\n", lines.join("\r\n")).into_bytes())
}
pub fn message_id(data: &[u8]) -> String { split(data).0.get("message-id").cloned().unwrap_or_default() }

pub fn send(c: &Credentials, to: &str, subject: &str, body: &str, reply_to: Option<&str>) -> Result<String> {
    use lettre::transport::smtp::authentication::Credentials as Login;
    let data = outgoing(&c.email, to, subject, body, reply_to)?;
    let envelope = lettre::address::Envelope::new(Some(c.email.parse().map_err(|_| MailFailure("Invalid sender address.".into()))?), vec![to.parse().map_err(|_| MailFailure("Invalid recipient address.".into()))?])
        .map_err(|_| MailFailure("Check the recipient address.".into()))?;
    let server = c.mail_server().ok_or_else(|| MailFailure("Connect your mail account again.".into()))?;
    let builder = if server.smtp_implicit_tls { lettre::SmtpTransport::relay(&server.smtp_host) } else { lettre::SmtpTransport::starttls_relay(&server.smtp_host) };
    let transport = builder.map_err(|_| MailFailure("Could not reach your mail provider securely.".into()))?
        .port(server.smtp_port)
        .credentials(Login::new(c.email.clone(), c.password.clone()))
        .timeout(Some(Duration::from_secs(90)))
        .build();
    use lettre::Transport;
    transport.send_raw(&envelope, &data).map_err(|e| {
        if e.is_permanent() && e.to_string().to_lowercase().contains("auth") { MailFailure("The provider rejected sign-in. Check the email address and generate a new app password.".into()) }
        else { MailFailure("Sending was not confirmed. Your draft is safe. Check your provider’s Sent folder before retrying, because the server may have accepted it.".into()) }
    })?;
    Ok(message_id(&data))
}

#[cfg(test)]
#[path = "mail_tests.rs"]
mod tests;
