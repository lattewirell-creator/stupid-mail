// The Mac app's ProviderTests, WorkspaceTests, CalendarTests and UpdateSignatureTests, against this port.
use crate::mail::{Credentials, Provider};
use crate::{calendar, net, update};
use base64::Engine;
use serde_json::json;

#[test]
fn providers_and_oauth() {
    let old: Credentials = serde_json::from_str(r#"{"email":"old@icloud.com","password":"fixture"}"#).unwrap();
    assert_eq!(old.kind(), Provider::Icloud);
    assert_eq!((Provider::Gmail.imap_host(), Provider::Gmail.smtp_host()), ("imap.gmail.com", "smtp.gmail.com"));
    assert_eq!(Provider::Icloud.imap_host(), "imap.mail.me.com");
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
