package com.emil.stupidmail;

import android.content.Intent;
import android.net.Uri;
import android.provider.CalendarContract;
import android.webkit.JavascriptInterface;

import com.emil.stupidmail.Dialogs.Field;
import com.emil.stupidmail.MailCore.Credentials;
import com.emil.stupidmail.MailCore.MailFailure;
import com.emil.stupidmail.MailCore.Provider;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

import java.time.ZonedDateTime;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.UUID;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;

/** The interface's requests (port of macos/NativeBridge.swift and windows/src/bridge.rs): same actions, checks and
 *  wording, Android dialogs instead of macOS sheets. Only the app's own page can call it (checked per request). */
final class Bridge {
    private final MainActivity app;
    private final Store store;
    private final ExecutorService work = Executors.newCachedThreadPool();
    final AtomicBoolean mailBusy = new AtomicBoolean(false), buddyBusy = new AtomicBoolean(false);
    private boolean dialogOpen;

    Bridge(MainActivity app, Store store) { this.app = app; this.store = store; }

    interface Job { Object run() throws Exception; }
    private void background(String id, AtomicBoolean busy, Job job) {
        work.execute(() -> {
            Object result; String error = null;
            try { result = job.run(); } catch (MailFailure e) { result = null; error = e.getMessage(); } catch (Exception e) { result = null; error = "Something went wrong. Please try again."; }
            if (busy != null) busy.set(false);
            final Object r = result; final String err = error;
            app.runOnUiThread(() -> respond(id, r, err));
        });
    }
    void respond(String id, Object data, String error) {
        try {
            JSONObject p = new JSONObject().put("id", id).put("ok", error == null).put("data", data == null ? new JSONObject() : data);
            if (error != null) p.put("error", error);
            app.js("window.nativeResponse(" + p + ")");
        } catch (JSONException ignored) { }
    }
    private void dialog(String title, String text, List<Field> fields, String[] buttons, Dialogs.Done done) {
        dialogOpen = true;
        Dialogs.show(app, title, text, fields, buttons, (b, v) -> { dialogOpen = false; done.done(b, v); });
    }
    private boolean busy() { return mailBusy.get() || dialogOpen; }
    private Credentials credentials() { JSONObject o = store.loadSecret("mail"); return o == null ? null : Credentials.from(o); }

    /** From the page: window.webkit.messageHandlers.mail.postMessage(m) → here (see MainActivity's shim). */
    @JavascriptInterface
    public void post(String body) {
        app.runOnUiThread(() -> {
            if (!app.isOwnPage()) return;
            try { handle(new JSONObject(body)); } catch (JSONException ignored) { }
        });
    }

    private void handle(JSONObject r) throws JSONException {
        String id = r.optString("id"), action = r.optString("action");
        if (id.isEmpty()) return;
        switch (action) {
            case "__flushed": app.flushed(r.optBoolean("ok")); break;
            case "status": {
                String email = store.setting("icloudEmail"), provider = store.setting("mailProvider");
                respond(id, new JSONObject().put("email", email).put("connected", !email.isEmpty()).put("provider", provider.isEmpty() ? "icloud" : provider), null); break;
            }
            case "readCache": { String account = r.optString("account"); background(id, null, () -> new JSONObject().put("messages", store.readCache(account))); break; }
            case "writeCache": {
                if (!r.has("contents")) { respond(id, null, "Missing mailbox data."); break; }
                String account = r.optString("account"), contents = r.optString("contents");
                background(id, null, () -> { store.writeCache(account, contents); return new JSONObject(); }); break;
            }
            case "connect": connect(id); break;
            case "disconnect": disconnect(id); break;
            case "sync": runMail(id, c -> new JSONObject().put("email", c.email).put("provider", c.provider.raw).put("messages", new JSONArray(c.provider == Provider.OUTLOOK ? Net.outlookSync(c) : MailCore.sync(c)))); break;
            case "send": confirmSend(id, r); break;
            case "calendarSuggest": {
                JSONArray items = r.optJSONArray("items"), out = new JSONArray();
                for (int i = 0; items != null && i < Math.min(40, items.length()) && out.length() < 30; i++) {
                    JSONObject item = items.getJSONObject(i);
                    for (JSONObject s : Meetings.suggestions(item.optString("text"), item.optString("title", "Meeting"), ZonedDateTime.now())) {
                        String source = item.optString("source"); s.put("source", source.length() > 100 ? source.substring(0, 100) : source); out.put(s);
                        if (out.length() == 30) break;
                    }
                }
                respond(id, new JSONObject().put("suggestions", out), null); break;
            }
            // Events go to the phone's calendar app, where you check and save them.
            case "calendarConnect": case "calendarList":
                respond(id, new JSONObject().put("calendars", new JSONArray().put(new JSONObject().put("id", "phone").put("title", "Your phone’s calendar app"))).put("events", new JSONArray()), null); break;
            case "calendarCreate": calendarCreate(id, r); break;
            case "workspaceOpen": case "workspaceSettings": case "workspaceShare": workspace(id, action, r); break;
            case "buddyStatus": respond(id, buddyStatus(), null); break;
            case "buddySettings": buddySettings(id); break;
            case "buddyAsk": buddyAsk(id, r); break;
            default: respond(id, null, "Unknown action.");
        }
    }

    interface MailJob { Object run(Credentials c) throws Exception; }
    private void runMail(String id, MailJob job) {
        if (!mailBusy.compareAndSet(false, true)) { respond(id, null, "A mail operation is already running. Please wait."); return; }
        background(id, mailBusy, () -> {
            Credentials c = credentials();
            if (c == null) throw new MailFailure("Connect your mail account first.");
            if (c.provider == Provider.OUTLOOK) c = Net.refreshed(c, next -> { try { store.saveSecret("mail", next.json()); } catch (JSONException e) { throw new MailFailure("Could not save the Microsoft sign-in."); } });
            return job.run(c);
        });
    }

    private void connect(String id) {
        if (busy()) { respond(id, null, "Finish the current operation first."); return; }
        List<Provider> list = Provider.onPhone();
        String[] names = new String[list.size()]; for (int i = 0; i < names.length; i++) names[i] = list.get(i).menu;
        dialog("Connect your mail", "Choose your provider. One account is active at a time; connecting another replaces the saved sign-in. Drafts and sorting stay separate for each email address.",
            Arrays.asList(Field.choice("provider", names, 0)), new String[]{"Continue", "Cancel"}, (b, v) -> {
                if (b != 0) { respond(id, null, "Cancelled."); return; }
                Provider p = list.get(Math.min(list.size() - 1, Integer.parseInt(v.getOrDefault("provider", "0"))));
                if (p == Provider.OUTLOOK) connectOutlook(id); else connectPassword(id, p);
            });
    }
    private void connectPassword(String id, Provider p) {
        boolean custom = p == Provider.CUSTOM;
        List<Field> fields = new ArrayList<>(Arrays.asList(Field.email("email", p == Provider.ICLOUD ? "you@icloud.com" : p == Provider.GMAIL ? "you@gmail.com" : "you@example.com", store.setting("icloudEmail")),
            Field.secret("password", custom ? "Password or app password" : "App password")));
        if (custom) fields.addAll(Arrays.asList(Field.text("imap", "IMAP server, e.g. imap.example.com", ""), Field.number("imapPort", "IMAP port (SSL/TLS)", "993"),
            Field.text("smtp", "SMTP server, e.g. smtp.example.com", ""), Field.number("smtpPort", "SMTP port", "465"),
            Field.choice("security", new String[]{"Sending security: SSL/TLS", "Sending security: STARTTLS"}, 0)));
        String[] buttons = p.passwordPage() != null ? new String[]{"Connect", "Cancel", "Create app password"} : new String[]{"Connect", "Cancel"};
        dialog(custom ? "Connect your mail service" : "Connect " + p.title + " Mail", p.help() + " Your password is stored encrypted on this phone. Latest 40 inbox messages; local sorting and drafts.\n\nUse an app password where your provider offers one.",
            fields, buttons, (b, v) -> {
                if (b == 2) { app.openExternal(p.passwordPage()); respond(id, null, "Create an app-specific password, then connect again."); return; }
                if (b != 0) { respond(id, null, "Cancelled."); return; }
                String address = v.getOrDefault("email", "").toLowerCase();
                String secret = v.getOrDefault("password", "");
                if (Arrays.asList(Provider.ICLOUD, Provider.GMAIL, Provider.YAHOO, Provider.AOL, Provider.FASTMAIL, Provider.ZOHO).contains(p)) secret = secret.replace(" ", "");
                MailCore.Server server = null;
                if (custom) {
                    int ip, sp; try { ip = Integer.parseInt(v.get("imapPort")); sp = Integer.parseInt(v.get("smtpPort")); } catch (Exception e) { ip = 0; sp = 0; }
                    server = new MailCore.Server(v.getOrDefault("imap", "").toLowerCase(), ip, v.getOrDefault("smtp", "").toLowerCase(), sp, !"1".equals(v.get("security")));
                    if (!MailCore.Server.valid(server.imapHost, server.imapPort) || !MailCore.Server.valid(server.smtpHost, server.smtpPort)) { respond(id, null, "Check the server names and ports. Use your provider's IMAP and SMTP settings."); return; }
                }
                boolean okAddress = p == Provider.ICLOUD ? MailCore.Validation.icloudAddress(address) : MailCore.Validation.address(address);
                if (!okAddress || !p.validPassword(secret)) { respond(id, null, "Check your address and the app password from your provider."); return; }
                Credentials c = new Credentials(address, secret, p); c.server = server;
                mailBusy.set(true);
                background(id, mailBusy, () -> {
                    JSONArray messages = new JSONArray(MailCore.sync(c));
                    store.saveSecret("mail", c.json());
                    store.setSetting("icloudEmail", address); store.setSetting("mailProvider", p.raw);
                    return new JSONObject().put("email", address).put("provider", p.raw).put("messages", messages);
                });
            });
    }
    private void connectOutlook(String id) {
        dialog("Connect Outlook", "Sign in with Microsoft in your browser. This build needs an application client ID from a Microsoft app registration. No client secret or Microsoft password goes here.\n\nRegistration: personal + work/school accounts, public client flows enabled, and delegated User.Read, Mail.Read, Mail.Send permissions. See the setup guide for steps.",
            Arrays.asList(Field.text("client", "Microsoft application (client) ID", store.setting("microsoftClientID"))), new String[]{"Sign in", "Cancel", "Setup guide"}, (b, v) -> {
                if (b == 2) { app.openExternal("https://github.com/lattewirell-creator/stupid-mail/blob/main/docs/accounts.md"); respond(id, null, "Follow the Outlook setup guide, then connect again."); return; }
                if (b != 0) { respond(id, null, "Cancelled."); return; }
                String client = v.getOrDefault("client", "");
                try { UUID.fromString(client); } catch (Exception e) { respond(id, null, "Enter the application client ID from your Microsoft app registration."); return; }
                store.setSetting("microsoftClientID", client);
                mailBusy.set(true);
                work.execute(() -> {
                    JSONObject device;
                    try { Map<String, String> f = new TreeMap<>(); f.put("client_id", client); f.put("scope", Net.SCOPE); device = Net.auth("devicecode", f); }
                    catch (MailFailure e) { mailBusy.set(false); app.runOnUiThread(() -> respond(id, null, e.getMessage())); return; }
                    String code = device.optString("user_code", ""), secret = device.optString("device_code", "");
                    if (code.isEmpty() || secret.isEmpty()) { mailBusy.set(false); app.runOnUiThread(() -> respond(id, null, "Microsoft could not start sign-in. Check the client ID and enable public client flows in the registration.")); return; }
                    AtomicBoolean cancelled = new AtomicBoolean(false);
                    android.app.AlertDialog[] prompt = new android.app.AlertDialog[1];
                    app.runOnUiThread(() -> {
                        prompt[0] = Dialogs.show(app, "Finish Microsoft sign-in", "Open microsoft.com/devicelogin and enter this code (tap it to copy). Approve access only for your The Stupid Mail registration. This closes when sign-in finishes.",
                            Arrays.asList(Field.code("code", code)), new String[]{"Cancel sign-in"}, (bb, vv) -> cancelled.set(true));
                        app.openExternal("https://microsoft.com/devicelogin");
                    });
                    long deadline = System.currentTimeMillis() + Math.min(device.optLong("expires_in", 900), 900) * 1000;
                    long interval = Math.max(device.optLong("interval", 5), 5);
                    Object result = null; String error = null;
                    try {
                        Credentials provisional = null;
                        while (System.currentTimeMillis() < deadline && !cancelled.get()) {
                            Thread.sleep(interval * 1000);
                            if (cancelled.get()) break;
                            Map<String, String> f = new TreeMap<>(); f.put("client_id", client); f.put("device_code", secret); f.put("grant_type", "urn:ietf:params:oauth:grant-type:device_code");
                            JSONObject token = Net.auth("token", f);
                            String err = token.optString("error", "");
                            if (err.equals("authorization_pending")) continue;
                            if (err.equals("slow_down")) { interval += 5; continue; }
                            provisional = Net.credentials(token, "", client, null); break;
                        }
                        if (provisional == null || cancelled.get()) throw new MailFailure("Microsoft sign-in was cancelled or timed out. Connect again when ready.");
                        provisional.email = Net.outlookEmail(provisional.password);
                        JSONArray messages = new JSONArray(Net.outlookSync(provisional));
                        if (cancelled.get()) throw new MailFailure("Cancelled.");
                        store.saveSecret("mail", provisional.json());
                        store.setSetting("icloudEmail", provisional.email); store.setSetting("mailProvider", "outlook");
                        result = new JSONObject().put("email", provisional.email).put("provider", "outlook").put("messages", messages);
                    } catch (MailFailure e) { error = e.getMessage(); } catch (Exception e) { error = "Microsoft sign-in didn’t finish. Try again."; }
                    mailBusy.set(false);
                    final Object rr = result; final String ee = error;
                    app.runOnUiThread(() -> { if (prompt[0] != null && prompt[0].isShowing()) { cancelled.set(false); prompt[0].setOnDismissListener(null); prompt[0].dismiss(); } respond(id, rr, ee); });
                });
            });
    }
    private void disconnect(String id) {
        if (busy()) { respond(id, null, "Wait for the current operation to finish."); return; }
        dialog("Disconnect this account?", "This removes the saved sign-in from this phone. Your mail on the server is unchanged. Local drafts and sorting are kept for when you reconnect.",
            new ArrayList<>(), new String[]{"Disconnect", "Cancel"}, (b, v) -> {
                if (b != 0) { respond(id, null, "Cancelled."); return; }
                store.removeSecret("mail"); store.setSetting("icloudEmail", null); store.setSetting("mailProvider", null);
                respond(id, new JSONObject(), null);
            });
    }
    private void confirmSend(String id, JSONObject r) {
        if (busy()) { respond(id, null, "Finish the current operation first."); return; }
        String to = r.optString("to"), subject = r.optString("subject"), body = r.optString("body"), replyTo = r.has("replyTo") && !r.isNull("replyTo") ? r.optString("replyTo") : null;
        if (!MailCore.Validation.address(to) || subject.length() > 900 || body.length() > 1024 * 1024 || !r.has("subject") || !r.has("body")) { respond(id, null, "Check the recipient and message length."); return; }
        dialog("Send this email?", "To: " + to + "\nSubject: " + subject + "\n\nThis will send a real email from your connected mail account.", new ArrayList<>(), new String[]{"Send", "Cancel"}, (b, v) -> {
            if (b != 0) { respond(id, null, "Cancelled."); return; }
            runMail(id, c -> new JSONObject().put("sent", true).put("messageID", c.provider == Provider.OUTLOOK ? Net.outlookSend(c, to, subject, body, replyTo) : MailCore.send(c, to, subject, body, replyTo)));
        });
    }

    /* ---------------- Buddy ---------------- */
    private JSONObject buddyStatus() throws JSONException {
        JSONObject b = store.loadSecret("buddy");
        return b == null ? new JSONObject().put("ready", false).put("provider", "").put("model", "") : new JSONObject().put("ready", true).put("provider", b.optString("provider")).put("model", b.optString("model"));
    }
    private void buddySettings(String id) {
        if (dialogOpen) { respond(id, null, "Close the current dialog first."); return; }
        JSONObject current = store.loadSecret("buddy");
        String[] names = {"ollama", "openai", "anthropic"};
        int selected = current == null ? 1 : Math.max(0, Arrays.asList(names).indexOf(current.optString("provider")));
        dialog("Wake up Buddy", "Choose an AI and enter its model name. Cloud providers receive your questions, plus the selected email only when you enable ‘Include this email’. API usage may be billed by that provider. Ollama needs Ollama running on this phone.\n\nKeys are saved encrypted on this phone.",
            Arrays.asList(Field.choice("provider", new String[]{"Ollama — local", "OpenAI", "Claude — Anthropic"}, selected), Field.text("model", "Exact model name from your provider", current == null ? "" : current.optString("model")), Field.secret("key", "API key (not needed for Ollama)")),
            new String[]{"Save", "Cancel"}, (b, v) -> {
                if (b != 0) { respond(id, null, "Cancelled."); return; }
                String p = names[Math.min(2, Integer.parseInt(v.getOrDefault("provider", "0")))], m = v.getOrDefault("model", ""), typed = v.getOrDefault("key", "");
                String key = typed.isEmpty() && current != null && current.optString("provider").equals(p) ? current.optString("key") : typed;
                if (m.isEmpty() || m.length() > 160 || !m.matches("^[A-Za-z0-9._:/-]+$") || !(p.equals("ollama") || (!key.isEmpty() && key.length() <= 512 && !key.contains("\n")))) { respond(id, null, "Enter a model name and, for cloud providers, an API key."); return; }
                try { store.saveSecret("buddy", new JSONObject().put("provider", p).put("model", m).put("key", key)); respond(id, buddyStatus(), null); }
                catch (MailFailure | JSONException e) { respond(id, null, e.getMessage()); }
            });
    }
    private void buddyAsk(String id, JSONObject r) {
        if (buddyBusy.get()) { respond(id, null, "Buddy is already thinking."); return; }
        JSONObject b = store.loadSecret("buddy"); String q = r.optString("question");
        if (b == null || q.isEmpty()) { respond(id, null, "Open Buddy settings and choose an AI first."); return; }
        String context = r.optString("context");
        buddyBusy.set(true);
        background(id, buddyBusy, () -> new JSONObject().put("answer", Net.buddyAsk(b.optString("provider"), b.optString("model"), b.optString("key"), q, context)));
    }

    /* ---------------- calendar ---------------- */
    private void calendarCreate(String id, JSONObject r) {
        if (busy()) { respond(id, null, "Finish the current operation first."); return; }
        String title = r.optString("title").trim();
        java.time.Instant start = Meetings.parseIso(r.optString("start")), end = Meetings.parseIso(r.optString("end"));
        if (title.isEmpty() || title.length() > 200 || start == null || end == null || !end.isAfter(start) || end.getEpochSecond() - start.getEpochSecond() > 7 * 86400) { respond(id, null, "Check the title, times, and destination calendar."); return; }
        try {
            app.startActivity(new Intent(Intent.ACTION_INSERT).setData(CalendarContract.Events.CONTENT_URI).putExtra(CalendarContract.Events.TITLE, title)
                .putExtra(CalendarContract.EXTRA_EVENT_BEGIN_TIME, start.toEpochMilli()).putExtra(CalendarContract.EXTRA_EVENT_END_TIME, end.toEpochMilli()));
            respond(id, new JSONObject().put("saved", true), null);
        } catch (Exception e) { respond(id, null, "No calendar app opened the event. Check that a calendar app is installed."); }
    }

    /* ---------------- Slack / Teams ---------------- */
    private void workspace(String id, String action, JSONObject r) throws JSONException {
        String provider = r.optString("provider");
        if (!provider.equals("slack") && !provider.equals("teams")) { respond(id, null, "Choose Slack or Teams."); return; }
        String name = provider.equals("slack") ? "Slack" : "Teams", key = "workspace-" + provider;
        if (action.equals("workspaceOpen")) { app.openExternal(provider.equals("slack") ? "https://app.slack.com/client/" : "https://teams.microsoft.com/v2/"); respond(id, new JSONObject(), null); return; }
        if (action.equals("workspaceSettings")) {
            if (dialogOpen) { respond(id, null, "Close the current dialog first."); return; }
            JSONObject saved = store.loadSecret(key);
            dialog("Share emails to " + name, provider.equals("slack") ? "Create a Slack Incoming Webhook for your chosen channel. Give it a recognizable destination label and paste the secret webhook URL below. The URL is stored encrypted. Chat sign-in is separate."
                    : "Create a Teams Workflow using ‘When a Teams webhook request is received’ and a post-card action. Use a signed URL that accepts unauthenticated webhook calls. Tenant-authenticated workflows are not supported. The URL is stored encrypted. Chat sign-in is separate.",
                Arrays.asList(Field.text("label", "Destination label, e.g. Design team / #inbox", saved == null ? "" : saved.optString("label")), Field.secret("url", "Secret webhook URL")),
                new String[]{"Save", "Cancel", "Remove connection"}, (b, v) -> {
                    try {
                        if (b == 2) { store.removeSecret(key); respond(id, new JSONObject(), null); return; }
                        if (b != 0) { respond(id, null, "Cancelled."); return; }
                        String label = v.getOrDefault("label", ""), url = v.getOrDefault("url", "");
                        if (label.isEmpty() || label.length() > 100 || !Net.webhookValid(url, provider)) { respond(id, null, "Enter a destination label and a valid " + name + " webhook URL. See docs/workspaces.md for setup."); return; }
                        store.saveSecret(key, new JSONObject().put("provider", provider).put("label", label).put("webhook", url)); respond(id, new JSONObject(), null);
                    } catch (Exception e) { respond(id, null, "Could not save the workspace connection."); }
                });
            return;
        }
        if (busy()) { respond(id, null, "Finish the current operation first."); return; }
        JSONObject dest = store.loadSecret(key);
        if (dest == null || !Net.webhookValid(dest.optString("webhook"), provider)) { respond(id, null, "Set up this workspace’s sharing connection first."); return; }
        String text = r.optString("text"); byte[] payload;
        try { payload = Net.webhookPayload(text, provider); } catch (MailFailure e) { respond(id, null, e.getMessage()); return; }
        dialog("Share to " + name + "?", "Destination label: " + dest.optString("label") + "\nThe webhook determines the actual channel/chat. This text will be shared with everyone who can access it. No attachments are included.",
            Arrays.asList(Field.preview("text", text)), new String[]{"Share now", "Cancel"}, (b, v) -> {
                if (b != 0) { respond(id, null, "Cancelled."); return; }
                mailBusy.set(true);
                background(id, mailBusy, () -> {
                    try { Net.webhookPost(dest.optString("webhook"), payload); } catch (MailFailure e) { throw new MailFailure("Sharing was not confirmed. Check the destination before retrying to avoid a duplicate. The connection may need updating."); }
                    return new JSONObject().put("accepted", true);
                });
            });
    }
}
