package com.emil.stupidmail;

import static org.junit.Assert.*;

import com.emil.stupidmail.MailCore.Credentials;
import com.emil.stupidmail.MailCore.Provider;
import com.emil.stupidmail.MailCore.Server;
import com.emil.stupidmail.MailCore.Validation;

import org.json.JSONObject;
import org.junit.Test;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;

/** The Mac app's tests/CoreTests.swift and windows/src/mail_tests.rs, against the Android port. */
public class MailCoreTest {
    private static byte[] b(String s) { return s.getBytes(StandardCharsets.UTF_8); }

    @Test public void validationRules() throws Exception {
        assertTrue(Validation.icloudAddress("emil@icloud.com"));
        assertFalse(Validation.icloudAddress("emil@example.com"));
        assertFalse(Validation.address("a@b.com\r\nBcc: bad@x.com"));
        assertTrue(Validation.appPassword("abcd-efgh-ijkl-mnop"));
        assertFalse(Validation.appPassword("regularPassword!"));
        try { Validation.quote("secret\nurl=evil"); fail(); } catch (MailCore.MailFailure expected) { }
        assertEquals("\"a\\\"b\\\\c\"", Validation.quote("a\"b\\c"));
    }

    @Test public void headersAndEncodings() {
        assertEquals("Hello 👋", MailCore.header("=?UTF-8?B?SGVsbG8g8J+Riw==?="));
        assertEquals("Hej Emil", MailCore.header("=?UTF-8?Q?Hej_Emil?="));
        assertEquals("Hej då!", new String(MailCore.quotedPrintable(b("Hej=20d=C3=A5=\r\n!"), false), StandardCharsets.UTF_8));
        assertEquals("Hi & 👋", MailCore.plainHtml("<style>hidden</style><p>Hi &amp; &#x1F44B;</p><img src='https://tracker'>"));
    }

    @Test public void multipartRecord() throws Exception {
        String raw = "From: =?UTF-8?Q?Jamie_Chen?= <jamie@example.com>\r\nSubject: Hello\r\nMessage-ID: <abc@example.com>\r\nIn-Reply-To: <old@example.com>\r\nContent-Type: multipart/mixed; boundary=\"outside\"\r\n\r\n--outside\r\nContent-Type: multipart/alternative; boundary=inside\r\n\r\n--inside\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n\r\nSGVsbG8gRW1pbA==\r\n--inside\r\nContent-Type: text/html\r\n\r\n<p>Hello <b>Emil</b></p><script>evil()</script>\r\n--inside--\r\n--outside\r\nContent-Type: application/pdf; name=\"file.pdf\"\r\nContent-Disposition: attachment\r\n\r\n123\r\n--outside--\r\n";
        JSONObject r = MailCore.record(b(raw), "42", "FLAGS (\\Seen \\Flagged)", false);
        assertEquals("Hello Emil", r.getString("body"));
        assertEquals("Jamie Chen", r.getString("from"));
        assertEquals("jamie@example.com", r.getString("email"));
        assertEquals(1, r.getInt("attachmentCount"));
        assertFalse(r.getBoolean("unread")); assertTrue(r.getBoolean("star"));
        assertEquals("<old@example.com>", r.getString("inReplyTo"));
        assertTrue(MailCore.record(b("Subject: big\r\n\r\n"), "big", "", true).getString("body").contains("too large"));
    }

    @Test public void charsets() throws Exception {
        byte[] raw = b("Subject: x\r\nContent-Type: text/plain; charset=iso-8859-1\r\nContent-Transfer-Encoding: quoted-printable\r\n\r\nH=E4lsningar");
        assertEquals("Hälsningar", MailCore.record(raw, "c", "", false).getString("body"));
    }

    @Test public void literalsAndSearch() throws Exception {
        byte[] bytes = b("Subject: emoji\r\n\r\n🌻 hello");
        ByteArrayOutputStream fetch = new ByteArrayOutputStream();
        fetch.write(b("* 1 FETCH (UID 42 FLAGS () BODY[] {" + bytes.length + "}\r\n")); fetch.write(bytes); fetch.write(b("\r\n)\r\n"));
        List<MailCore.Literal> l = MailCore.literals(fetch.toByteArray());
        assertEquals(1, l.size()); assertArrayEquals(bytes, l.get(0).data);
        try { MailCore.literals(b("* 1 FETCH (BODY[] {800}\r\nshort")); fail(); } catch (MailCore.MailFailure expected) { }
        assertEquals(Arrays.asList(1L, 4L, 22L), MailCore.search(b("* SEARCH 4 1 22\r\n")));
    }

    @Test public void literalCannotSpoofProtocol() throws Exception {
        String poison = "Subject: literal\r\n\r\nM1 OK not-a-command\r\n* BYE not-a-command";
        String wire = "* OK ready\r\n* 1 FETCH (BODY[] {" + poison.length() + "}\r\n" + poison + " UID 9)\r\nM1 OK complete\r\n";
        ByteArrayOutputStream sent = new ByteArrayOutputStream();
        MailCore.Imap client = MailCore.Imap.open(new ByteArrayInputStream(b(wire)), sent);
        List<MailCore.Literal> parsed = MailCore.literals(client.command("UID FETCH 9 (UID BODY.PEEK[])"));
        assertEquals(1, parsed.size()); assertArrayEquals(b(poison), parsed.get(0).data); assertTrue(parsed.get(0).prefix.contains("UID 9"));
        assertEquals("M1 UID FETCH 9 (UID BODY.PEEK[])\r\n", sent.toString("UTF-8"));
        MailCore.Imap rejected = MailCore.Imap.open(new ByteArrayInputStream(b("* OK ready\r\nM1 NO authentication failed\r\n")), new ByteArrayOutputStream());
        try { rejected.login(new Credentials("test@icloud.com", "abcd-efgh-ijkl-mnop", Provider.ICLOUD)); fail(); } catch (MailCore.MailFailure expected) { }
        try { MailCore.Imap.open(new ByteArrayInputStream(b("garbage\r\n")), new ByteArrayOutputStream()); fail(); } catch (MailCore.MailFailure expected) { }
    }

    @Test public void outgoingRoundTrip() throws Exception {
        byte[] out = MailCore.outgoing("me@icloud.com", "you@example.com", "Hej 🌻", "hello\nworld", "<orig@example.com>");
        JSONObject sent = MailCore.record(out, "sent", "", false);
        assertEquals("Hej 🌻", sent.getString("subject")); assertEquals("hello\r\nworld", sent.getString("body")); assertEquals("<orig@example.com>", sent.getString("inReplyTo"));
        assertTrue(MailCore.messageId(out).endsWith("@stupidmail.local>"));
        try { MailCore.outgoing("me@icloud.com", "you@example.com", "Hi\r\nBcc: bad@example.com", "Hi", null); fail(); } catch (MailCore.MailFailure expected) { }
        StringBuilder subject = new StringBuilder(); for (int i = 0; i < 6; i++) subject.append("Lång rubrik med åäö ");
        assertEquals(subject.toString(), MailCore.record(MailCore.outgoing("me@icloud.com", "you@example.com", subject.toString(), "x", null), "l", "", false).getString("subject"));
    }

    @Test public void smtpSession() throws Exception {
        assertEquals("a\r\n..b\r\n", new String(Smtp.dotStuff(b("a\r\n.b")), StandardCharsets.US_ASCII));
        ByteArrayOutputStream sent = new ByteArrayOutputStream();
        Smtp.Session s = new Smtp.Session(new ByteArrayInputStream(b("250-smtp.example.com\r\n250-AUTH LOGIN PLAIN\r\n250 OK\r\n235 ok\r\n")), sent);
        String ehlo = s.command("EHLO stupidmail.local", 250);
        assertTrue(ehlo.contains("AUTH LOGIN PLAIN"));
        Smtp.authenticate(s, ehlo, "me@example.com", "pw");
        assertTrue(sent.toString("UTF-8").contains("AUTH PLAIN AG1lQGV4YW1wbGUuY29tAHB3\r\n"));
        Smtp.Session bad = new Smtp.Session(new ByteArrayInputStream(b("535 no\r\n")), new ByteArrayOutputStream());
        try { Smtp.authenticate(bad, "250 AUTH PLAIN", "me@example.com", "pw"); fail(); } catch (Smtp.Rejected r) { assertTrue(r.auth); }
        try { s.command("RCPT TO:<a@b.com>\r\nDATA"); fail(); } catch (Smtp.Rejected r) { assertFalse(r.auth); }
    }

    @Test public void providersAndServers() throws Exception {
        assertEquals(10, Provider.values().length);
        assertFalse(Provider.onPhone().contains(Provider.PROTON));
        Server y = Server.preset(Provider.YAHOO, "a@yahoo.com");
        assertEquals("imap.mail.yahoo.com", y.imapHost); assertEquals(993, y.imapPort); assertEquals("smtp.mail.yahoo.com", y.smtpHost); assertEquals(465, y.smtpPort); assertTrue(y.smtpImplicitTLS);
        assertEquals("imap.gmail.com", Server.preset(Provider.GMAIL, "a@gmail.com").imapHost);
        assertEquals("imap.mail.me.com", Server.preset(Provider.ICLOUD, "a@icloud.com").imapHost);
        assertEquals("imap.gmx.net", Server.preset(Provider.GMX, "a@gmx.de").imapHost);
        assertEquals("imap.gmx.com", Server.preset(Provider.GMX, "a@gmx.com").imapHost);
        assertNull(Server.preset(Provider.CUSTOM, "a@b.com")); assertNull(Server.preset(Provider.OUTLOOK, "a@b.com"));
        assertTrue(Server.valid("imap.example.com", 993));
        for (Object[] bad : new Object[][]{{"imap.example.com\r\nx", 993}, {"imap example.com", 993}, {"localhost", 993}, {"imap.example.com", 0}, {"-bad.example.com", 993}, {"imap.example.com/inbox", 993}})
            assertFalse(bad[0] + ":" + bad[1], Server.valid((String) bad[0], (Integer) bad[1]));
        assertTrue(Provider.YAHOO.validPassword("abcdefghijklmnop")); assertFalse(Provider.YAHOO.validPassword("short"));
        assertTrue(Provider.FASTMAIL.validPassword("abcd1234efgh5678")); assertFalse(Provider.FASTMAIL.validPassword("ABCD-1234"));
        // Same JSON as the Mac and Windows apps.
        Credentials old = Credentials.from(new JSONObject("{\"email\":\"old@icloud.com\",\"password\":\"fixture\"}"));
        assertEquals(Provider.ICLOUD, old.provider);
        Credentials other = new Credentials("me@example.com", "secret-pass", Provider.CUSTOM);
        other.server = new Server("imap.example.com", 993, "smtp.example.com", 587, false);
        String json = other.json().toString();
        assertTrue(json.contains("\"smtpImplicitTLS\":false"));
        Credentials back = Credentials.from(new JSONObject(json));
        assertEquals(Provider.CUSTOM, back.provider); assertEquals(587, back.mailServer().smtpPort); assertFalse(back.mailServer().smtpImplicitTLS);
    }
}
