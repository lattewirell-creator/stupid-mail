// The Mac app's tests/CoreTests.swift checks, against this port.
use super::*;
use std::io::Cursor;

/// Recorded server replies; remembers what the client sent.
struct Fixture { input: Cursor<Vec<u8>>, sent: Vec<u8> }
impl Fixture { fn new(text: &[u8]) -> Fixture { Fixture { input: Cursor::new(text.to_vec()), sent: vec![] } } }
impl Read for Fixture { fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> { self.input.read(buf) } }
impl Write for Fixture { fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> { self.sent.extend_from_slice(buf); Ok(buf.len()) } fn flush(&mut self) -> std::io::Result<()> { Ok(()) } }

#[test]
fn validation_rules() {
    assert!(validation::icloud_address("emil@icloud.com"));
    assert!(!validation::icloud_address("emil@example.com"));
    assert!(!validation::address("a@b.com\r\nBcc: bad@x.com"));
    assert!(validation::app_password("abcd-efgh-ijkl-mnop"));
    assert!(!validation::app_password("regularPassword!"));
    assert!(validation::quote("secret\nurl=evil").is_err());
    assert_eq!(validation::quote("a\"b\\c").unwrap(), "\"a\\\"b\\\\c\"");
}
#[test]
fn headers_and_encodings() {
    assert_eq!(header("=?UTF-8?B?SGVsbG8g8J+Riw==?="), "Hello 👋");
    assert_eq!(header("=?UTF-8?Q?Hej_Emil?="), "Hej Emil");
    assert_eq!(String::from_utf8(quoted_printable(b"Hej=20d=C3=A5=\r\n!", false)).unwrap(), "Hej då!");
    assert_eq!(plain_html("<style>hidden</style><p>Hi &amp; &#x1F44B;</p><img src='https://tracker'>"), "Hi & 👋");
}
#[test]
fn multipart_record() {
    let raw = "From: =?UTF-8?Q?Jamie_Chen?= <jamie@example.com>\r\nSubject: Hello\r\nMessage-ID: <abc@example.com>\r\nIn-Reply-To: <old@example.com>\r\nContent-Type: multipart/mixed; boundary=\"outside\"\r\n\r\n--outside\r\nContent-Type: multipart/alternative; boundary=inside\r\n\r\n--inside\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\nSGVsbG8gRW1pbA==\r\n--inside\r\nContent-Type: text/html\r\n\r\n<p>Hello <b>Emil</b></p><script>evil()</script>\r\n--inside--\r\n--outside\r\nContent-Type: application/pdf; name=\"file.pdf\"\r\nContent-Disposition: attachment\r\n\r\n123\r\n--outside--\r\n";
    let r = record(raw.as_bytes(), "42", "FLAGS (\\Seen \\Flagged)", false);
    assert_eq!(r.body, "Hello Emil");
    assert_eq!((r.from.as_str(), r.email.as_str()), ("Jamie Chen", "jamie@example.com"));
    assert_eq!(r.attachment_count, 1);
    assert!(!r.unread && r.star);
    assert_eq!(r.in_reply_to, "<old@example.com>");
    assert!(record(b"Subject: big\r\n\r\n", "big", "", true).body.contains("too large"));
}
#[test]
fn literals_and_search() {
    let bytes = "Subject: emoji\r\n\r\n🌻 hello".as_bytes().to_vec();
    let mut fetch = format!("* 1 FETCH (UID 42 FLAGS () BODY[] {{{}}}\r\n", bytes.len()).into_bytes();
    fetch.extend(&bytes); fetch.extend(b"\r\n)\r\n");
    let l = literals(&fetch).unwrap();
    assert_eq!(l.len(), 1); assert_eq!(l[0].data, bytes);
    assert!(literals(b"* 1 FETCH (BODY[] {800}\r\nshort").is_err());
    assert_eq!(search(b"* SEARCH 4 1 22\r\n"), vec![1, 4, 22]);
}
#[test]
fn outgoing_round_trip() {
    let out = outgoing("me@icloud.com", "you@example.com", "Hej 🌻", "hello\nworld", Some("<orig@example.com>")).unwrap();
    let sent = record(&out, "sent", "", false);
    assert_eq!(sent.subject, "Hej 🌻"); assert_eq!(sent.body, "hello\r\nworld"); assert_eq!(sent.in_reply_to, "<orig@example.com>");
    assert!(outgoing("me@icloud.com", "you@example.com", "Hi\r\nBcc: bad@example.com", "Hi", None).is_err());
    let long = outgoing("me@icloud.com", "you@example.com", &"Lång rubrik med åäö ".repeat(6), "x", None).unwrap();
    assert_eq!(record(&long, "l", "", false).subject, "Lång rubrik med åäö ".repeat(6));
}
#[test]
fn literal_cannot_spoof_protocol() {
    let poison = "Subject: literal\r\n\r\nM1 OK not-a-command\r\n* BYE not-a-command";
    let wire = format!("* OK ready\r\n* 1 FETCH (BODY[] {{{}}}\r\n{poison} UID 9)\r\nM1 OK complete\r\n", poison.len());
    let mut client = Imap::new(Fixture::new(wire.as_bytes())).unwrap();
    let response = client.command("UID FETCH 9 (UID BODY.PEEK[])").unwrap();
    let parsed = literals(&response).unwrap();
    assert_eq!(parsed.len(), 1); assert_eq!(parsed[0].data, poison.as_bytes()); assert!(parsed[0].prefix.contains("UID 9"));
    assert_eq!(client.channel.stream.sent, b"M1 UID FETCH 9 (UID BODY.PEEK[])\r\n");
    let mut rejected = Imap::new(Fixture::new(b"* OK ready\r\nM1 NO authentication failed\r\n")).unwrap();
    let c = Credentials { email: "test@icloud.com".into(), password: "abcd-efgh-ijkl-mnop".into(), provider: None, client_id: None, refresh_token: None, expires_at: None };
    assert!(rejected.authenticate(&c).is_err());
}
#[test]
fn charsets() {
    let raw = b"Subject: x\r\nContent-Type: text/plain; charset=iso-8859-1\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\nH=E4lsningar";
    assert_eq!(record(raw, "c", "", false).body, "Hälsningar");
}
