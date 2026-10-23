// The Mac app's ProviderTests, WorkspaceTests, CalendarTests and UpdateSignatureTests, against this port.
use crate::mail::{Credentials, Provider};
use crate::{calendar, net, update};
use base64::Engine;
use serde_json::json;

#[test]
fn providers_and_oauth() {
    let old: Credentials = serde_json::from_str(r#"{"email":"old@icloud.com","password":"fixture"}"#).unwrap();
    assert_eq!(old.kind(), Provider::Icloud);
    let g = crate::mail::Server::preset(Provider::Gmail, "a@gmail.com").unwrap();
    assert_eq!((g.imap_host.as_str(), g.smtp_host.as_str()), ("imap.gmail.com", "smtp.gmail.com"));
    assert_eq!(crate::mail::Server::preset(Provider::Icloud, "a@icloud.com").unwrap().imap_host, "imap.mail.me.com");
    let c = net::credentials(&json!({"access_token": "fixture-token", "refresh_token": "fixture-refresh", "expires_in": 3600}), "me@outlook.com", "fixture-client", None).unwrap();
    assert!(c.kind() == Provider::Outlook && c.refresh_token.as_deref() == Some("fixture-refresh"));
    assert!(crate::mail::sync(&c).is_err(), "Outlook token must never go to IMAP");
    assert!(c.expires_at.unwrap() - chrono::Utc::now().timestamp() > 3500);
    let back: Credentials = serde_json::from_value(serde_json::to_value(&c).unwrap()).unwrap();
    assert_eq!(back.kind(), Provider::Outlook);
    let renewed = net::credentials(&json!({"access_token": "new-token"}), &c.email, "fixture-client", c.refresh_token.as_deref()).unwrap();
    assert_eq!(renewed.refresh_token, c.refresh_token);
    let rotated = net::credentials(&json!({"access_token": "new-token", "refresh_token": "new-refresh"}), &c.email, "fixture-client", c.refresh_token.as_deref()).unwrap();
    assert_eq!(rotated.refresh_token.as_deref(), Some("new-refresh"));
    assert!(net::credentials(&json!({"error": "authorization_declined"}), "", "", None).is_err());
    let input = json!({"value": [{"id": "immutable", "from": {"emailAddress": {"name": "Jamie", "address": "jamie@example.com"}}, "subject": "A message",
        "body": {"contentType": "html", "content": "<p>Hello &amp; welcome</p><script>bad()</script>"}, "isRead": false, "hasAttachments": true, "flag": {"flagStatus": "flagged"},
        "internetMessageId": "<one@example.com>", "internetMessageHeaders": [{"name": "In-Reply-To", "value": "<original@example.com>"}, {"name": "List-Unsubscribe", "value": "<https://example.com>"}]}]});
    let m = &net::records(&input.to_string()).unwrap()[0];
    assert!(m.id == "outlook:immutable" && m.email == "jamie@example.com");
    assert_eq!(m.body, "Hello & welcome");
    assert!(m.unread && m.star && m.newsletter && m.attachment_count > 0);
    assert_eq!(m.in_reply_to, "<original@example.com>");
    assert!(net::records(r#"{"value":[{}]}"#).is_err());
}
#[test]
fn workspace_endpoints_and_payloads() {
    assert!(net::webhook_valid("https://hooks.slack.com/services/T/B/fixture", "slack"));
    for url in ["http://hooks.slack.com/services/T/B/fixture", "https://hooks.slack.com.evil.test/services/T/B/fixture", "https://user@hooks.slack.com/services/T/B/fixture", "https://hooks.slack.com:444/services/T/B/fixture", "https://example.com/services/T/B/fixture"] {
        assert!(!net::webhook_valid(url, "slack"), "{url}");
    }
    assert!(net::webhook_valid("https://prod-1.westeurope.logic.azure.com/workflows/fixture/triggers/manual/paths/invoke?sig=fixture", "teams"));
    assert!(net::webhook_valid("https://fixture.environment.api.powerplatform.com/powerautomate/automations/direct/workflows/fixture/triggers/manual/paths/invoke?sig=fixture", "teams"));
    assert!(!net::webhook_valid("https://evil.test/workflows/fixture?sig=fixture", "teams"));
    assert!(!net::webhook_valid("https://fixture.environment.api.powerplatform.com/workflows/fixture", "teams"));
    let text = "From: fixture@example.com\n<!channel> <https://example.com> & hello";
    let slack: serde_json::Value = serde_json::from_slice(&net::webhook_payload(text, "slack").unwrap()).unwrap();
    assert_eq!(slack["blocks"][0]["text"]["type"], "plain_text"); assert_eq!(slack["blocks"][0]["text"]["text"], text);
    assert_eq!(slack["unfurl_links"], false);
    let teams: serde_json::Value = serde_json::from_slice(&net::webhook_payload(text, "teams").unwrap()).unwrap();
    assert_eq!(teams["type"], "message");
    assert!(net::webhook_payload("", "slack").is_err() && net::webhook_payload(&"x".repeat(2501), "slack").is_err());
}
#[test]
fn meeting_detection() {
    use chrono::TimeZone;
    let now = chrono::Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap().with_timezone(&chrono::Local);
    let found = calendar::suggestions("Project meeting June 4, 2030 at 10:00 AM.", "Project review", now);
    assert!(!found.is_empty());
    assert_eq!(found[0]["title"], "Project review");
    assert!(found[0]["phrase"].as_str().unwrap().contains("2030"));
    let (s, e) = (calendar::parse_iso(found[0]["start"].as_str().unwrap()).unwrap(), calendar::parse_iso(found[0]["end"].as_str().unwrap()).unwrap());
    assert!(e > s);
    assert_eq!(s.with_timezone(&chrono::Local).format("%H:%M").to_string(), "10:00");
    assert!(calendar::suggestions("June 4, 2020 at 10:00 AM", "Old meeting", now).is_empty());
    assert!(calendar::suggestions("Thank you for the update.", "No meeting", now).is_empty());
    // Relative and Swedish phrases
    let sv = calendar::suggestions("Ses vi imorgon kl 15:30? Annars tisdag kl 9.", "Möte", now);
    assert_eq!(sv.len(), 2);
    assert_eq!(calendar::parse_iso(sv[0]["start"].as_str().unwrap()).unwrap().with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string(), "2030-01-02 15:30");
    assert!(!calendar::suggestions("Can we do the 20 oktober at 2pm?", "Möte", now).is_empty());
    assert!(calendar::suggestions("The meeting room is great", "x", now).is_empty(), "words that aren't dates");
    assert!(calendar::ics("Lunch, with Sam", s, e).contains("SUMMARY:Lunch\\, with Sam"));
}
#[test]
fn update_signatures() {
    use ed25519_dalek::{Signer, SigningKey};
    let key = SigningKey::from_bytes(&[7u8; 32]);
    let public = base64::engine::general_purpose::STANDARD.encode(key.verifying_key().to_bytes());
    let archive = b"pretend installer bytes".to_vec();
    let sig = base64::engine::general_purpose::STANDARD.encode(key.sign(&archive).to_bytes());
    assert!(update::verify_with(&public, &archive, &sig), "genuine accepted");
    let mut changed = archive.clone(); changed[0] ^= 1;
    assert!(!update::verify_with(&public, &changed, &sig), "tampered rejected");
    let other = base64::engine::general_purpose::STANDARD.encode(SigningKey::from_bytes(&[9u8; 32]).verifying_key().to_bytes());
    assert!(!update::verify_with(&other, &archive, &sig), "wrong key rejected");
    assert!(!update::verify(&archive, &sig), "not signed with the app's key");
    assert!(update::newer("2.6", "2.5") && update::newer("2.6.1", "2.6.0") && !update::newer("2.5", "2.6.0") && !update::newer("2.6", "2.6.0"));
}
#[test]
fn dialog_page_preview() {
    use crate::dialog::{Field, Spec};
    let spec = Spec::new("Connect iCloud Mail", "Use your primary iCloud address and an Apple app-specific password from account.apple.com.\n\nUse an app password, not your normal account password.", &["Connect", "Cancel", "Create app password"])
        .field(Field::text("email", "you@icloud.com", "")).field(Field::secret("password", "App-specific password (xxxx-xxxx-xxxx-xxxx)"));
    let html = crate::dialog::html(&spec);
    assert!(html.contains("Connect iCloud Mail"));
    let hostile = crate::dialog::html(&Spec::new("x", "</script><script>alert(1)</script>", &["OK"]));
    assert_eq!(hostile.matches("</script>").count(), 1, "text cannot close the page script");
    if let Ok(path) = std::env::var("STUPID_MAIL_DIALOG_PREVIEW") { std::fs::write(path, html).unwrap(); }
}

#[test]
fn more_providers() {
    use crate::mail::{Server, ALL};
    assert_eq!(Server::preset(Provider::Yahoo, "a@yahoo.com").unwrap(), Server { imap_host: "imap.mail.yahoo.com".into(), imap_port: 993, smtp_host: "smtp.mail.yahoo.com".into(), smtp_port: 465, smtp_implicit_tls: true });
    assert_eq!(Server::preset(Provider::Gmx, "a@gmx.de").unwrap().imap_host, "imap.gmx.net");
    assert_eq!(Server::preset(Provider::Gmx, "a@gmx.com").unwrap().imap_host, "imap.gmx.com");
    assert_eq!(Server::preset(Provider::Proton, "a@proton.me").unwrap().imap_host, "127.0.0.1");
    assert!(Server::preset(Provider::Custom, "a@b.com").is_none() && Server::preset(Provider::Outlook, "a@b.com").is_none());
    assert!(Server::valid("imap.example.com", 993));
    for (h, p) in [("imap.example.com\r\nx", 993), ("imap example.com", 993), ("localhost", 993), ("imap.example.com", 0), ("-bad.example.com", 993), ("imap.example.com/inbox", 993)] { assert!(!Server::valid(h, p), "{h}:{p}"); }
    assert!(Provider::Yahoo.valid_password("abcdefghijklmnop") && !Provider::Yahoo.valid_password("short"));
    assert!(Provider::Fastmail.valid_password("abcd1234efgh5678") && !Provider::Fastmail.valid_password("ABCD-1234"));
    assert!(Provider::Proton.valid_password("bridge-pw-Xy7") && !Provider::Proton.valid_password("pw\nevil"));
    let other = Credentials { email: "me@example.com".into(), password: "secret-pass".into(), provider: Some(Provider::Custom), client_id: None, refresh_token: None, expires_at: None,
        server: Some(Server { imap_host: "imap.example.com".into(), imap_port: 993, smtp_host: "smtp.example.com".into(), smtp_port: 587, smtp_implicit_tls: false }) };
    let back: Credentials = serde_json::from_value(serde_json::to_value(&other).unwrap()).unwrap();
    assert!(back.kind() == Provider::Custom && back.mail_server().unwrap().smtp_port == 587 && !back.mail_server().unwrap().smtp_implicit_tls);
    // The Mac app stores the same JSON (camelCase keys), so the formats stay compatible.
    assert!(serde_json::to_string(&other).unwrap().contains("\"smtpImplicitTLS\":false"));
    assert_eq!(ALL.len(), 10);
}
#[test]
fn mailto_links() {
    use crate::bridge::mailto;
    let d = mailto("mailto:sam%40example.com?subject=Lunch%20%26%20tea&body=Hi%2C%0D%0Asee%20you&cc=x@example.com").unwrap();
    assert_eq!((d["email"].as_str().unwrap(), d["subject"].as_str().unwrap(), d["body"].as_str().unwrap()), ("sam@example.com", "Lunch & tea", "Hi,\nsee you"));
    assert_eq!(mailto("mailto:?to=a@example.com,b@example.com").unwrap()["email"], "a@example.com");
    assert_eq!(mailto("mailto:not-an-address?subject=Line%0Abreak").unwrap(), json!({"email": "", "subject": "Line break", "body": ""}));
    assert!(mailto("https://example.com").is_none() && mailto("--flag").is_none());
}
#[cfg(target_os = "linux")]
#[test]
fn linux_storage_and_updates() {
    let dir = std::env::temp_dir().join(format!("stupid-mail-xdg-{}", std::process::id()));
    std::env::set_var("XDG_DATA_HOME", &dir);
    let base = if std::env::var_os("STUPID_MAIL_DATA").is_some() { None } else { Some(crate::store::base()) };
    if let Some(base) = base {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(base, dir.join("stupid-mail"));
        assert_eq!(std::fs::metadata(&base).unwrap().permissions().mode() & 0o777, 0o700, "only this user can open the mail folder");
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(update::MANAGED_BY_SYSTEM, "apt updates the Linux app, it never downloads installers");
    assert!(crate::store::SECRETS_NOTE.contains("keyring"));
}
/// Live check (run on demand: cargo test -- --ignored): each preset's real IMAP server answers over TLS and
/// rejects a made-up account, which proves host, port and encryption.
#[test]
#[ignore]
fn live_provider_servers() {
    for p in [Provider::Icloud, Provider::Gmail, Provider::Yahoo, Provider::Aol, Provider::Fastmail, Provider::Zoho, Provider::Gmx] {
        let c = Credentials { email: format!("stupidmail-test-nobody@{}", match p { Provider::Icloud => "icloud.com", Provider::Gmail => "gmail.com", Provider::Yahoo => "yahoo.com", Provider::Aol => "aol.com", Provider::Fastmail => "fastmail.com", Provider::Zoho => "zohomail.com", _ => "gmx.com" }),
            password: "abcdefghijklmnop".into(), provider: Some(p), client_id: None, refresh_token: None, expires_at: None, server: None };
        let r = crate::mail::sync(&c);
        println!("{:<9} {}", p.title(), match r { Ok(_) => "unexpectedly signed in".into(), Err(e) => e.0 });
    }
}
