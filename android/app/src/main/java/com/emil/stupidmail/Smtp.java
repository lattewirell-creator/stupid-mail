package com.emil.stupidmail;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.charset.StandardCharsets;
import java.util.Base64;

/** Sending over SMTP: SSL/TLS from the start, or STARTTLS before anything private is sent (plain text is never
 *  allowed). Signs in with AUTH PLAIN (or LOGIN), sends one message to one recipient, and checks every reply. */
final class Smtp {
    private Smtp() {}

    static final class Rejected extends Exception { final boolean auth; Rejected(boolean auth) { this.auth = auth; } }

    static void deliver(MailCore.Server server, String user, String password, String to, byte[] message) throws Rejected, MailCore.MailFailure {
        Socket socket = null;
        try {
            if (server.smtpImplicitTLS) socket = MailCore.tls(server.smtpHost, server.smtpPort);
            else { Socket raw = new Socket(); raw.connect(new InetSocketAddress(server.smtpHost, server.smtpPort), 20000); raw.setSoTimeout(90000); socket = raw; }
            Session s = new Session(socket.getInputStream(), socket.getOutputStream());
            s.expect(220);
            String ehlo = s.command("EHLO stupidmail.local", 250);
            if (!server.smtpImplicitTLS) {
                if (!ehlo.toUpperCase().contains("STARTTLS")) throw new Rejected(false);   // never send a password unencrypted
                s.command("STARTTLS", 220);
                socket = MailCore.upgrade(socket, server.smtpHost, server.smtpPort);
                s = new Session(socket.getInputStream(), socket.getOutputStream());
                ehlo = s.command("EHLO stupidmail.local", 250);
            }
            authenticate(s, ehlo, user, password);
            s.command("MAIL FROM:<" + user + ">", 250);
            s.command("RCPT TO:<" + to + ">", 250, 251);
            s.command("DATA", 354);
            s.write(dotStuff(message));
            s.write(".\r\n".getBytes(StandardCharsets.US_ASCII));
            s.expect(250);
            try { s.command("QUIT", 221); } catch (Exception ignored) { }
        } catch (IOException e) {
            throw new Rejected(false);
        } finally {
            if (socket != null) try { socket.close(); } catch (IOException ignored) { }
        }
    }

    static void authenticate(Session s, String ehlo, String user, String password) throws IOException, Rejected {
        String caps = ehlo.toUpperCase();
        Base64.Encoder b64 = Base64.getEncoder();
        try {
            if (caps.contains("PLAIN") || !caps.contains("LOGIN")) {
                s.command("AUTH PLAIN " + b64.encodeToString(("\0" + user + "\0" + password).getBytes(StandardCharsets.UTF_8)), 235);
            } else {
                s.command("AUTH LOGIN", 334);
                s.command(b64.encodeToString(user.getBytes(StandardCharsets.UTF_8)), 334);
                s.command(b64.encodeToString(password.getBytes(StandardCharsets.UTF_8)), 235);
            }
        } catch (Rejected r) { throw new Rejected(true); }
    }

    /** Lines starting with "." get an extra "." so the message can't end the DATA section early. */
    static byte[] dotStuff(byte[] message) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        boolean lineStart = true;
        for (byte b : message) { if (lineStart && b == '.') out.write('.'); out.write(b); lineStart = b == '\n'; }
        if (!lineStart) { out.write('\r'); out.write('\n'); }
        return out.toByteArray();
    }

    static final class Session {
        final InputStream in; final OutputStream out;
        Session(InputStream in, OutputStream out) { this.in = in; this.out = out; }
        void write(byte[] b) throws IOException { out.write(b); out.flush(); }
        String command(String line, int... ok) throws IOException, Rejected {
            if (MailCore.hasControl(line, 32)) throw new Rejected(false);
            write((line + "\r\n").getBytes(StandardCharsets.UTF_8));
            return expect(ok);
        }
        /** Reads one (possibly multi-line) reply and checks its code. */
        String expect(int... ok) throws IOException, Rejected {
            StringBuilder all = new StringBuilder();
            while (true) {
                String line = readLine();
                all.append(line).append('\n');
                if (line.length() < 4 || line.charAt(3) != '-') {
                    int code; try { code = Integer.parseInt(line.substring(0, 3)); } catch (Exception e) { throw new Rejected(false); }
                    for (int c : ok) if (c == code) return all.toString();
                    throw new Rejected(false);
                }
            }
        }
        private String readLine() throws IOException {
            ByteArrayOutputStream b = new ByteArrayOutputStream();
            int c;
            while ((c = in.read()) != -1) { if (c == '\n') break; if (c != '\r') b.write(c); if (b.size() > 8192) throw new IOException("long line"); }
            if (c == -1 && b.size() == 0) throw new IOException("closed");
            return b.toString(StandardCharsets.UTF_8.name());
        }
    }
}
