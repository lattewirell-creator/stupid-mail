package com.emil.stupidmail;

import org.json.JSONException;
import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.Charset;
import java.nio.charset.StandardCharsets;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Base64;
import java.util.Collections;
import java.util.Date;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.UUID;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import javax.net.ssl.SSLParameters;
import javax.net.ssl.SSLSocket;
import javax.net.ssl.SSLSocketFactory;

/** Mail for The Stupid Mail on Android: providers, IMAP for the inbox, SMTP for sending, and MIME turned into
 *  inert text (the same rules as macos/MailCore.swift and windows/src/mail.rs). Only encrypted connections; the
 *  server's certificate and host name are checked. Email HTML is never loaded: no remote images or trackers. */
final class MailCore {
    private MailCore() {}

    static final class MailFailure extends Exception { MailFailure(String m) { super(m); } }

    enum Provider {
        ICLOUD("icloud", "iCloud", "iCloud — Apple app password"), GMAIL("gmail", "Gmail", "Gmail — Google app password"), OUTLOOK("outlook", "Outlook", "Outlook — Microsoft sign-in"),
        YAHOO("yahoo", "Yahoo", "Yahoo — app password"), AOL("aol", "AOL", "AOL — app password"), FASTMAIL("fastmail", "Fastmail", "Fastmail — app password"),
        ZOHO("zoho", "Zoho", "Zoho — app-specific password"), GMX("gmx", "GMX", "GMX — password or app password"),
        PROTON("proton", "Proton Mail", "Proton Mail — through Proton Mail Bridge"), CUSTOM("custom", "Other", "Other mail service — enter its servers");
        final String raw, title, menu;
        Provider(String raw, String title, String menu) { this.raw = raw; this.title = title; this.menu = menu; }
        static Provider of(String raw) { for (Provider p : values()) if (p.raw.equals(raw)) return p; return ICLOUD; }
        /** Proton Mail Bridge only runs on computers, so phones get the other providers. */
        static List<Provider> onPhone() { List<Provider> l = new ArrayList<>(); for (Provider p : values()) if (p != PROTON) l.add(p); return l; }
        String passwordPage() {
            switch (this) {
                case ICLOUD: return "https://account.apple.com/"; case GMAIL: return "https://myaccount.google.com/apppasswords"; case YAHOO: return "https://login.yahoo.com/account/security";
                case AOL: return "https://login.aol.com/account/security"; case FASTMAIL: return "https://app.fastmail.com/settings/security/apps"; case ZOHO: return "https://accounts.zoho.com/home#security/app_password";
                case GMX: return "https://www.gmx.com/"; case PROTON: return "https://proton.me/mail/bridge"; default: return null;
            }
        }
        String help() {
            switch (this) {
                case ICLOUD: return "Use your primary iCloud address and an Apple app-specific password from account.apple.com.";
                case GMAIL: return "Use your full Gmail or Google Workspace address and a 16-letter Google app password. Enable 2-Step Verification, then create one at myaccount.google.com/apppasswords. Some work/school or protected accounts do not allow app passwords.";
                case YAHOO: return "Use your Yahoo address and a 16-letter app password: Yahoo Account → Account security → Generate app password.";
                case AOL: return "Use your AOL address and a 16-letter app password: AOL Account → Account security → Generate app password.";
                case FASTMAIL: return "Use your Fastmail address and an app password with Mail (IMAP/SMTP) access: Settings → Privacy & Security → App passwords.";
                case ZOHO: return "Use your Zoho Mail address and an app-specific password (Zoho Accounts → Security → App passwords). IMAP access must be on in Zoho Mail settings. For zoho.eu or other regions, choose Other mail service.";
                case GMX: return "Turn on POP3 & IMAP access in GMX settings first. Use your GMX address and, if two-factor authentication is on, an app password.";
                case PROTON: return "Proton only lets other apps in through Proton Mail Bridge, which runs on computers.";
                case CUSTOM: return "Enter your provider's IMAP (incoming) and SMTP (outgoing) servers. Connections always use encryption (TLS); IMAP needs SSL/TLS, usually port 993.";
                default: return "";
            }
        }
        boolean validPassword(String v) {
            switch (this) {
                case ICLOUD: case GMAIL: case YAHOO: case AOL: return Validation.appPassword(v);
                case FASTMAIL: return v.matches("^[a-z0-9]{16}$");
                case ZOHO: return v.matches("^[A-Za-z0-9]{12,16}$");
                default: return v.codePointCount(0, v.length()) >= 8 && v.length() <= 256 && !hasControl(v, 32);
            }
        }
    }

    /** Where a password account's mail lives. Presets are fixed per provider; "Other" uses what you entered. */
    static final class Server {
        final String imapHost, smtpHost; final int imapPort, smtpPort; final boolean smtpImplicitTLS;
        Server(String imapHost, int imapPort, String smtpHost, int smtpPort, boolean smtpImplicitTLS) { this.imapHost = imapHost; this.imapPort = imapPort; this.smtpHost = smtpHost; this.smtpPort = smtpPort; this.smtpImplicitTLS = smtpImplicitTLS; }
        static Server preset(Provider p, String email) {
            String domain = email.substring(email.lastIndexOf('@') + 1).toLowerCase(Locale.ROOT);
            switch (p) {
                case ICLOUD: return new Server("imap.mail.me.com", 993, "smtp.mail.me.com", 587, false);
                case GMAIL: return new Server("imap.gmail.com", 993, "smtp.gmail.com", 587, false);
                case YAHOO: return new Server("imap.mail.yahoo.com", 993, "smtp.mail.yahoo.com", 465, true);
                case AOL: return new Server("imap.aol.com", 993, "smtp.aol.com", 465, true);
                case FASTMAIL: return new Server("imap.fastmail.com", 993, "smtp.fastmail.com", 465, true);
                case ZOHO: return new Server("imap.zoho.com", 993, "smtp.zoho.com", 465, true);
                case GMX: { boolean de = domain.equals("gmx.net") || domain.equals("gmx.de") || domain.equals("gmx.at") || domain.equals("gmx.ch");
                    return new Server(de ? "imap.gmx.net" : "imap.gmx.com", 993, de ? "mail.gmx.net" : "mail.gmx.com", 587, false); }
                case PROTON: return new Server("127.0.0.1", 1143, "127.0.0.1", 1025, true);
                default: return null;
            }
        }
        /** A server you typed in: a plain host name and a normal port, nothing else. */
        static boolean valid(String host, int port) {
            return port > 0 && port <= 65535 && host.length() <= 253 && host.matches("^[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?(\\.[A-Za-z0-9]([A-Za-z0-9-]*[A-Za-z0-9])?)+$");
        }
        JSONObject json() throws JSONException { return new JSONObject().put("imapHost", imapHost).put("imapPort", imapPort).put("smtpHost", smtpHost).put("smtpPort", smtpPort).put("smtpImplicitTLS", smtpImplicitTLS); }
        static Server from(JSONObject o) { return o == null ? null : new Server(o.optString("imapHost"), o.optInt("imapPort"), o.optString("smtpHost"), o.optInt("smtpPort"), o.optBoolean("smtpImplicitTLS")); }
    }

    /** The saved account, stored as the same JSON as on the Mac and Windows. */
    static final class Credentials {
        String email, password, clientID, refreshToken; Provider provider; long expiresAt; Server server;
        Credentials(String email, String password, Provider provider) { this.email = email; this.password = password; this.provider = provider; }
        Server mailServer() { return server != null ? server : Server.preset(provider, email); }
        JSONObject json() throws JSONException {
            JSONObject o = new JSONObject().put("email", email).put("password", password).put("provider", provider.raw);
            if (clientID != null) o.put("clientID", clientID);
            if (refreshToken != null) o.put("refreshToken", refreshToken);
            if (expiresAt > 0) o.put("expiresAt", expiresAt);
            if (server != null) o.put("server", server.json());
            return o;
        }
        static Credentials from(JSONObject o) {
            Credentials c = new Credentials(o.optString("email"), o.optString("password"), Provider.of(o.optString("provider", "icloud")));
            c.clientID = o.has("clientID") ? o.optString("clientID") : null; c.refreshToken = o.has("refreshToken") ? o.optString("refreshToken") : null;
            c.expiresAt = o.optLong("expiresAt"); c.server = Server.from(o.optJSONObject("server"));
            return c;
        }
    }

    static boolean hasControl(String s, int below) { for (int i = 0; i < s.length(); i++) { char c = s.charAt(i); if (c < below || c == 127) return true; } return false; }

    static final class Validation {
        static boolean address(String v) { return v.length() <= 254 && !hasControl(v, 33) && v.matches("^[A-Za-z0-9.!#$%&'*+/=?^_`{|}~-]+@[A-Za-z0-9](?:[A-Za-z0-9.-]*[A-Za-z0-9])?\\.[A-Za-z]{2,}$"); }
        static boolean icloudAddress(String v) { if (!address(v)) return false; String d = v.substring(v.lastIndexOf('@') + 1).toLowerCase(Locale.ROOT); return d.equals("icloud.com") || d.equals("me.com") || d.equals("mac.com"); }
        static boolean appPassword(String v) { return v.matches("^[a-z]{4}(-?[a-z]{4}){3}$"); }
        static String quote(String v) throws MailFailure {
            if (hasControl(v, 32)) throw new MailFailure("Invalid control character in account or message settings.");
            return "\"" + v.replace("\\", "\\\\").replace("\"", "\\\"") + "\"";
        }
    }

    static String capture(String pattern, String value) { Matcher m = Pattern.compile(pattern, Pattern.CASE_INSENSITIVE | Pattern.DOTALL).matcher(value); return m.find() ? m.group(1) : null; }
    static String latin1(byte[] b, int from, int to) { return new String(b, from, to - from, StandardCharsets.ISO_8859_1); }
    static int indexOf(byte[] data, byte[] pat, int from) {
        outer: for (int i = Math.max(0, from); i <= data.length - pat.length; i++) { for (int j = 0; j < pat.length; j++) if (data[i + j] != pat[j]) continue outer; return i; }
        return -1;
    }
    static final byte[] CRLF = {13, 10};

    /* ---------------- connections ---------------- */
    /** A TLS socket that checks the certificate and the host name. */
    static SSLSocket tls(String host, int port) throws MailFailure {
        try {
            Socket raw = new Socket(); raw.connect(new InetSocketAddress(host, port), 20000); raw.setSoTimeout(30000);
            return upgrade(raw, host, port);
        } catch (IOException e) { throw new MailFailure("The mail server did not respond. Check your internet connection and try again."); }
    }
    static SSLSocket upgrade(Socket raw, String host, int port) throws MailFailure {
        try {
            SSLSocket s = (SSLSocket) ((SSLSocketFactory) SSLSocketFactory.getDefault()).createSocket(raw, host, port, true);
            SSLParameters p = s.getSSLParameters(); p.setEndpointIdentificationAlgorithm("HTTPS"); s.setSSLParameters(p);
            s.startHandshake();
            return s;
        } catch (IOException e) { throw new MailFailure("Could not establish a secure connection to the mail server."); }
    }

    /** IMAP: tagged commands, literal-aware responses (a message body can never pose as a server reply). */
    static final class Imap {
        final InputStream in; final OutputStream out; byte[] buf = new byte[0]; int seq = 0;
        Imap(InputStream in, OutputStream out) throws MailFailure {
            this.in = in; this.out = out;
        }
        static Imap open(InputStream in, OutputStream out) throws MailFailure {
            Imap i = new Imap(in, out);
            byte[] greeting = i.line();
            if (!new String(greeting, StandardCharsets.UTF_8).startsWith("* OK")) throw new MailFailure("The mail server did not accept the connection.");
            return i;
        }
        private void receive() throws MailFailure {
            byte[] chunk = new byte[65536]; int n;
            try { n = in.read(chunk); } catch (IOException e) { throw new MailFailure("The mail server took too long to reply. Refresh to try again."); }
            if (n <= 0) throw new MailFailure("The mail server closed the connection. Please refresh.");
            byte[] next = new byte[buf.length + n]; System.arraycopy(buf, 0, next, 0, buf.length); System.arraycopy(chunk, 0, next, buf.length, n); buf = next;
        }
        byte[] line() throws MailFailure {
            while (true) {
                int i = indexOf(buf, CRLF, 0);
                if (i >= 0) return take(i + 2);
                if (buf.length >= 2 * 1024 * 1024) throw new MailFailure("The mail server returned an unusually long response.");
                receive();
            }
        }
        byte[] bytes(int count) throws MailFailure {
            if (count < 0 || count > 2 * 1024 * 1024) throw new MailFailure("This message is too large to load.");
            while (buf.length < count) receive();
            return take(count);
        }
        private byte[] take(int n) { byte[] head = new byte[n]; System.arraycopy(buf, 0, head, 0, n); byte[] rest = new byte[buf.length - n]; System.arraycopy(buf, n, rest, 0, rest.length); buf = rest; return head; }
        byte[] command(String command) throws MailFailure {
            if (hasControl(command, 32)) throw new MailFailure("Invalid mail command.");
            String tag = "M" + (++seq);
            try { out.write((tag + " " + command + "\r\n").getBytes(StandardCharsets.UTF_8)); out.flush(); }
            catch (IOException e) { throw new MailFailure("The connection to the mail server was interrupted."); }
            ByteArrayOutputStream response = new ByteArrayOutputStream();
            Pattern literal = Pattern.compile("\\{([0-9]+)\\}\r\n$");
            while (true) {
                byte[] l = line(); String text = new String(l, StandardCharsets.UTF_8);
                response.write(l, 0, l.length);
                if (response.size() >= 16 * 1024 * 1024) throw new MailFailure("The inbox response is too large.");
                if (text.startsWith(tag + " ")) {
                    if (!text.startsWith(tag + " OK")) throw new MailFailure(command.startsWith("LOGIN") ? "The mail server rejected the sign-in. Check your email address and app-specific password." : "The mail server could not complete this request. Your mailbox was not changed.");
                    return response.toByteArray();
                }
                if (text.startsWith("* BYE")) throw new MailFailure("The mail server ended the session. Please refresh.");
                Matcher m = literal.matcher(text);
                if (m.find()) { byte[] b = bytes(Integer.parseInt(m.group(1))); response.write(b, 0, b.length); }
            }
        }
        void login(Credentials c) throws MailFailure { command("LOGIN " + Validation.quote(c.email) + " " + Validation.quote(c.password)); }
    }

    static final class Literal { final String prefix; final byte[] data; Literal(String p, byte[] d) { prefix = p; data = d; } }
    /** The message literals of a FETCH response ({n} followed by n bytes), each with its surrounding text. */
    static List<Literal> literals(byte[] data) throws MailFailure {
        List<Literal> out = new ArrayList<>(); Pattern size = Pattern.compile("\\{([0-9]+)\\}$"); int cursor = 0;
        while (cursor < data.length) {
            int end = indexOf(data, CRLF, cursor); if (end < 0) break;
            String line = new String(data, cursor, end - cursor, StandardCharsets.UTF_8); cursor = end + 2;
            Matcher m = size.matcher(line); if (!m.find()) continue;
            int count = Integer.parseInt(m.group(1));
            if (count > 2 * 1024 * 1024 || count > data.length - cursor) throw new MailFailure("The mail server returned an incomplete message. Please refresh.");
            int after = cursor + count, tailEnd = indexOf(data, CRLF, after); if (tailEnd < 0) tailEnd = after;
            byte[] body = new byte[count]; System.arraycopy(data, cursor, body, 0, count);
            out.add(new Literal(line + " " + new String(data, after, tailEnd - after, StandardCharsets.UTF_8), body));
            cursor = after;
        }
        return out;
    }
    static List<Long> search(byte[] data) {
        List<Long> ids = new ArrayList<>();
        for (String line : new String(data, StandardCharsets.UTF_8).split("\r?\n")) {
            if (!line.startsWith("* SEARCH")) continue;
            String[] parts = line.split(" ");
            for (int i = 2; i < parts.length; i++) try { long n = Long.parseLong(parts[i].trim()); if (n > 0) ids.add(n); } catch (NumberFormatException ignored) { }
            break;
        }
        Collections.sort(ids);
        return ids;
    }

    /* ---------------- MIME ---------------- */
    static final class Parts { final Map<String, String> headers; final byte[] body; Parts(Map<String, String> h, byte[] b) { headers = h; body = b; } }
    static Parts split(byte[] data) {
        int i = indexOf(data, "\r\n\r\n".getBytes(StandardCharsets.US_ASCII), 0), skip = 4;
        if (i < 0) { i = indexOf(data, "\n\n".getBytes(StandardCharsets.US_ASCII), 0); skip = 2; }
        if (i < 0) return new Parts(new HashMap<>(), data);
        String raw = new String(data, 0, i, StandardCharsets.UTF_8).replace("\r\n", "\n").replaceAll("\n[ \t]+", " ");
        Map<String, String> headers = new HashMap<>();
        for (String line : raw.split("\n")) { int c = line.indexOf(':'); if (c > 0) headers.put(line.substring(0, c).toLowerCase(Locale.ROOT), line.substring(c + 1).trim()); }
        byte[] body = new byte[data.length - i - skip]; System.arraycopy(data, i + skip, body, 0, body.length);
        return new Parts(headers, body);
    }
    static byte[] quotedPrintable(byte[] d, boolean header) {
        ByteArrayOutputStream out = new ByteArrayOutputStream(); int i = 0;
        while (i < d.length) {
            if (header && d[i] == '_') { out.write(' '); i++; continue; }
            if (d[i] == '=') {
                if (i + 1 < d.length && d[i + 1] == '\n') { i += 2; continue; }
                if (i + 2 < d.length && d[i + 1] == '\r' && d[i + 2] == '\n') { i += 3; continue; }
                if (i + 2 < d.length) { try { out.write(Integer.parseInt(latin1(d, i + 1, i + 3), 16)); i += 3; continue; } catch (NumberFormatException ignored) { } }
            }
            out.write(d[i]); i++;
        }
        return out.toByteArray();
    }
    static String decode(byte[] data, String charset) {
        try { return new String(data, Charset.forName(charset.trim())); } catch (Exception e) { return new String(data, StandardCharsets.UTF_8); }
    }
    static String header(String value) {
        value = value.replaceAll("\\?=[ \t]+=\\?", "?==?");
        Matcher m = Pattern.compile("=\\?([^?]+)\\?([bBqQ])\\?([^?]*)\\?=").matcher(value);
        StringBuffer sb = new StringBuffer();
        while (m.find()) {
            byte[] bytes;
            try { bytes = m.group(2).equalsIgnoreCase("b") ? Base64.getMimeDecoder().decode(m.group(3)) : quotedPrintable(m.group(3).getBytes(StandardCharsets.US_ASCII), true); }
            catch (IllegalArgumentException e) { bytes = new byte[0]; }
            m.appendReplacement(sb, Matcher.quoteReplacement(decode(bytes, m.group(1))));
        }
        m.appendTail(sb);
        return sb.toString();
    }
    static String parameter(String name, String value) {
        String q = capture("(?:^|;)\\s*" + name + "\\s*=\\s*\"([^\"]+)\"", value);
        return q != null ? q : capture("(?:^|;)\\s*" + name + "\\s*=\\s*([^;\\s]+)", value);
    }
    static String plainHtml(String html) {
        String t = html.replaceAll("(?is)<script\\b[^>]*>.*?</script\\s*>|<style\\b[^>]*>.*?</style\\s*>|<head\\b[^>]*>.*?</head\\s*>", "");
        t = t.replaceAll("(?i)<(?:br\\s*/?|/p|/div|/tr|/h[1-6])\\s*>", "\n").replaceAll("<[^>]*>", "");
        t = t.replace("&nbsp;", " ").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#39;", "'").replace("&apos;", "'").replace("&amp;", "&");
        Matcher m = Pattern.compile("&#(x[0-9a-fA-F]+|[0-9]+);").matcher(t); StringBuffer sb = new StringBuffer();
        while (m.find()) {
            String g = m.group(1); String rep = m.group(0);
            try { int cp = g.startsWith("x") ? Integer.parseInt(g.substring(1), 16) : Integer.parseInt(g); rep = new String(Character.toChars(cp)); } catch (Exception ignored) { }
            m.appendReplacement(sb, Matcher.quoteReplacement(rep));
        }
        m.appendTail(sb);
        return sb.toString().replaceAll("\n{3,}", "\n\n").trim();
    }
    static final class Content { String plain = "", html = ""; int attachments; }
    static Content content(byte[] data, int depth) {
        Content c = new Content(); if (depth >= 12) return c;
        Parts p = split(data);
        String type = p.headers.getOrDefault("content-type", "text/plain; charset=utf-8"), lower = type.toLowerCase(Locale.ROOT);
        if (p.headers.getOrDefault("content-disposition", "").toLowerCase(Locale.ROOT).contains("attachment") || parameter("name", type) != null) { c.attachments = 1; return c; }
        String boundary = lower.startsWith("multipart/") ? parameter("boundary", type) : null;
        if (boundary != null && boundary.length() < 200) {
            String raw = new String(p.body, StandardCharsets.ISO_8859_1);   // keeps every byte while splitting on ASCII delimiters
            String[] parts = raw.split(Pattern.quote("--" + boundary), -1);
            for (int i = 1; i < parts.length; i++) {
                if (parts[i].startsWith("--")) break;
                String trimmed = parts[i].replaceAll("^[\r\n]+|[\r\n]+$", "");
                if (trimmed.isEmpty()) continue;
                Content item = content(trimmed.getBytes(StandardCharsets.ISO_8859_1), depth + 1);
                if (!item.plain.isEmpty()) c.plain += (c.plain.isEmpty() ? "" : "\n\n") + item.plain;
                if (!item.html.isEmpty()) c.html += (c.html.isEmpty() ? "" : "\n\n") + item.html;
                c.attachments += item.attachments;
            }
            return c;
        }
        if (!(lower.startsWith("text/plain") || lower.startsWith("text/html"))) { c.attachments = p.body.length == 0 ? 0 : 1; return c; }
        String transfer = p.headers.getOrDefault("content-transfer-encoding", "").toLowerCase(Locale.ROOT);
        byte[] decoded = p.body;
        if (transfer.equals("base64")) { try { decoded = Base64.getMimeDecoder().decode(p.body); } catch (IllegalArgumentException e) { decoded = new byte[0]; } }
        else if (transfer.equals("quoted-printable")) decoded = quotedPrintable(p.body, false);
        String cs = parameter("charset", type);
        String text = decode(decoded, cs == null ? "utf-8" : cs).trim();
        if (lower.startsWith("text/html")) c.html = plainHtml(text); else c.plain = text;
        return c;
    }
    static JSONObject record(byte[] data, String id, String flags, boolean limited) throws JSONException {
        Parts p = split(data); Content c = limited ? new Content() : content(data, 0);
        String from = header(p.headers.getOrDefault("from", "Unknown sender"));
        String email = capture("<([^>]+)>", from); if (email == null) email = Validation.address(from) ? from : "";
        String name = from.split("<", 2)[0].replaceAll("^[ \"]+|[ \"]+$", "");
        String body = limited ? "This message is too large for the current sync. Open it in your provider’s mail app to read the full message and attachments."
            : !c.plain.isEmpty() ? c.plain : !c.html.isEmpty() ? c.html : "No readable text. This message may contain only attachments or encrypted content.";
        String f = flags.toLowerCase(Locale.ROOT);
        return new JSONObject().put("id", id).put("from", name.isEmpty() ? email : name).put("email", email)
            .put("subject", header(p.headers.getOrDefault("subject", "(No subject)"))).put("body", body).put("time", p.headers.getOrDefault("date", ""))
            .put("unread", !f.contains("\\seen")).put("star", f.contains("\\flagged")).put("newsletter", p.headers.containsKey("list-id") || p.headers.containsKey("list-unsubscribe"))
            .put("attachmentCount", c.attachments).put("messageID", p.headers.getOrDefault("message-id", "")).put("inReplyTo", p.headers.getOrDefault("in-reply-to", ""));
    }

    /* ---------------- sync and send ---------------- */
    static List<JSONObject> sync(Credentials c) throws MailFailure, JSONException {
        if (c.provider == Provider.OUTLOOK) throw new MailFailure("Outlook requires Microsoft sign-in.");
        Server s = c.mailServer(); if (s == null) throw new MailFailure("Connect your mail account again.");
        try (SSLSocket socket = tls(s.imapHost, s.imapPort)) {
            Imap imap = Imap.open(socket.getInputStream(), socket.getOutputStream());
            imap.login(c);
            String validity = capture("UIDVALIDITY\\s+([0-9]+)", new String(imap.command("EXAMINE INBOX"), StandardCharsets.UTF_8));
            if (validity == null) throw new MailFailure("The mail server did not return a valid mailbox identifier. Please reconnect.");
            List<Long> all = search(imap.command("UID SEARCH ALL"));
            List<Long> ids = new ArrayList<>(all.subList(Math.max(0, all.size() - 40), all.size()));
            if (ids.isEmpty()) return new ArrayList<>();
            Map<Long, JSONObject> records = new HashMap<>(); List<Long> bodyIds = new ArrayList<>(); long total = 0;
            List<Literal> heads = literals(imap.command("UID FETCH " + join(ids) + " (UID FLAGS RFC822.SIZE BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE MESSAGE-ID IN-REPLY-TO LIST-ID LIST-UNSUBSCRIBE)])"));
            Collections.reverse(heads);
            for (Literal l : heads) {
                String u = capture("UID\\s+([0-9]+)", l.prefix); if (u == null) continue; long uid = Long.parseLong(u);
                String sz = capture("RFC822.SIZE\\s+([0-9]+)", l.prefix); long size = sz == null ? Long.MAX_VALUE : Long.parseLong(sz);
                records.put(uid, record(l.data, c.provider.raw + ":" + validity + ":" + uid, l.prefix, true));
                if (size <= 1024 * 1024 && total + size <= 10 * 1024 * 1024) { bodyIds.add(uid); total += size; }
            }
            if (!bodyIds.isEmpty()) {
                for (Literal l : literals(imap.command("UID FETCH " + join(bodyIds) + " (UID FLAGS BODY.PEEK[])"))) {
                    String u = capture("UID\\s+([0-9]+)", l.prefix); if (u == null) continue; long uid = Long.parseLong(u);
                    records.put(uid, record(l.data, c.provider.raw + ":" + validity + ":" + uid, l.prefix, false));
                }
            }
            if (records.size() != ids.size()) throw new MailFailure("The inbox changed while syncing. Refresh again to get a complete snapshot.");
            List<JSONObject> out = new ArrayList<>(); for (int i = ids.size() - 1; i >= 0; i--) out.add(records.get(ids.get(i)));
            return out;
        } catch (IOException e) { throw new MailFailure("The connection to the mail server was interrupted."); }
    }
    private static String join(List<Long> ids) { StringBuilder b = new StringBuilder(); for (Long id : ids) { if (b.length() > 0) b.append(','); b.append(id); } return b.toString(); }

    /** The message as sent: plain text, UTF-8, base64 body, RFC 2047 subject folded at word boundaries. */
    static byte[] outgoing(String from, String to, String subject, String body, String replyTo) throws MailFailure {
        if (!Validation.address(from) || !Validation.address(to) || hasControl(subject, 32) || subject.getBytes(StandardCharsets.UTF_8).length > 900 || body.getBytes(StandardCharsets.UTF_8).length > 1024 * 1024)
            throw new MailFailure("Use one valid recipient, a subject under 900 bytes, and a message under 1 MB.");
        Base64.Encoder b64 = Base64.getEncoder();
        List<String> words = new ArrayList<>(); StringBuilder seg = new StringBuilder();
        for (int i = 0; i < subject.length(); ) {
            int cp = subject.codePointAt(i); String ch = new String(Character.toChars(cp));
            if ((seg + ch).getBytes(StandardCharsets.UTF_8).length > 39 && seg.length() > 0) { words.add("=?UTF-8?B?" + b64.encodeToString(seg.toString().getBytes(StandardCharsets.UTF_8)) + "?="); seg.setLength(0); }
            seg.append(ch); i += Character.charCount(cp);
        }
        if (seg.length() > 0) words.add("=?UTF-8?B?" + b64.encodeToString(seg.toString().getBytes(StandardCharsets.UTF_8)) + "?=");
        String id = "<" + UUID.randomUUID().toString().toLowerCase(Locale.ROOT) + "@stupidmail.local>";
        String date = new SimpleDateFormat("EEE, dd MMM yyyy HH:mm:ss Z", Locale.US).format(new Date());
        StringBuilder h = new StringBuilder("From: " + from + "\r\nTo: " + to + "\r\nSubject: " + String.join("\r\n ", words) + "\r\nDate: " + date + "\r\nMessage-ID: " + id
            + "\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n");
        if (replyTo != null && replyTo.matches("^<[^<>\\s]+@[^<>\\s]+>$") && replyTo.length() < 250) h.append("In-Reply-To: ").append(replyTo).append("\r\nReferences: ").append(replyTo).append("\r\n");
        String encoded = Base64.getMimeEncoder(76, CRLF).encodeToString(body.replace("\r\n", "\n").replace("\n", "\r\n").getBytes(StandardCharsets.UTF_8));
        return (h + "\r\n" + encoded + "\r\n").getBytes(StandardCharsets.UTF_8);
    }
    static String messageId(byte[] data) { return split(data).headers.getOrDefault("message-id", ""); }

    static String send(Credentials c, String to, String subject, String body, String replyTo) throws MailFailure {
        byte[] data = outgoing(c.email, to, subject, body, replyTo);
        Server s = c.mailServer(); if (s == null) throw new MailFailure("Connect your mail account again.");
        try { Smtp.deliver(s, c.email, c.password, to, data); }
        catch (Smtp.Rejected e) { throw new MailFailure(e.auth ? "The provider rejected sign-in. Check the email address and generate a new app password." : "Sending was not confirmed. Your draft is safe. Check your provider’s Sent folder before retrying, because the server may have accepted it."); }
        return messageId(data);
    }
}
