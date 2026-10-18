package com.emil.stupidmail;

import android.annotation.SuppressLint;
import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.content.res.AssetManager;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.view.WindowInsets;
import android.webkit.ValueCallback;
import android.webkit.WebChromeClient;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.window.OnBackInvokedDispatcher;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;

/** One full-screen WebView showing the shared mail interface (dist/, packed into assets/web). The page is served
 *  from a private https origin inside the app; nothing loads from the internet and only that page can reach the
 *  bridge. Back closes an open dialog first. */
public final class MainActivity extends Activity {
    static final String ORIGIN = "https://appassets.androidplatform.net";
    private static final String SHIM = "window.webkit={messageHandlers:{mail:{postMessage:m=>StupidMailNative.post(JSON.stringify(m))}}};";
    private WebView web;
    private Bridge bridge;
    private ValueCallback<Uri[]> chooser;
    private String pendingMailto;

    @SuppressLint({"SetJavaScriptEnabled", "AddJavascriptInterface"})
    @Override protected void onCreate(Bundle saved) {
        super.onCreate(saved);
        web = new WebView(this);
        web.setBackgroundColor((getResources().getConfiguration().uiMode & android.content.res.Configuration.UI_MODE_NIGHT_MASK) == android.content.res.Configuration.UI_MODE_NIGHT_YES ? 0xFF1E1C19 : 0xFFF7F4EE);
        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setAllowFileAccess(false);
        s.setAllowContentAccess(false);
        s.setGeolocationEnabled(false);
        s.setMixedContentMode(WebSettings.MIXED_CONTENT_NEVER_ALLOW);
        s.setSupportMultipleWindows(false);
        s.setMediaPlaybackRequiresUserGesture(false);
        bridge = new Bridge(this, new Store(this));
        web.addJavascriptInterface(bridge, "StupidMailNative");
        web.setWebViewClient(new WebViewClient() {
            @Override public WebResourceResponse shouldInterceptRequest(WebView v, WebResourceRequest r) { return serve(r.getUrl()); }
            @Override public boolean shouldOverrideUrlLoading(WebView v, WebResourceRequest r) {
                Uri u = r.getUrl();
                if (ORIGIN.equals(u.getScheme() + "://" + u.getHost())) return false;
                if (r.hasGesture() && ("https".equals(u.getScheme()) || "mailto".equals(u.getScheme()))) openExternal(u.toString());
                return true;
            }
            @Override public void onPageFinished(WebView v, String url) { if (pendingMailto != null) { compose(pendingMailto); pendingMailto = null; } }
        });
        web.setWebChromeClient(new WebChromeClient() {
            @Override public boolean onShowFileChooser(WebView v, ValueCallback<Uri[]> callback, FileChooserParams params) {
                if (chooser != null) chooser.onReceiveValue(null);
                chooser = callback;
                try { startActivityForResult(params.createIntent(), 7); } catch (ActivityNotFoundException e) { chooser = null; return false; }
                return true;
            }
        });
        // Edge to edge: keep the page clear of the status bar, navigation bar and keyboard.
        web.setOnApplyWindowInsetsListener((v, insets) -> {
            android.graphics.Insets i = insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.ime() | WindowInsets.Type.displayCutout());
            v.setPadding(i.left, i.top, i.right, i.bottom);
            return WindowInsets.CONSUMED;
        });
        setContentView(web);
        if (Build.VERSION.SDK_INT >= 33) getOnBackInvokedDispatcher().registerOnBackInvokedCallback(OnBackInvokedDispatcher.PRIORITY_DEFAULT, this::back);
        pendingMailto = mailto(getIntent());
        web.loadUrl(ORIGIN + "/index.html");
        checkForUpdate();
    }

    /* ---------------- the page ---------------- */
    private WebResourceResponse serve(Uri u) {
        Map<String, String> headers = new HashMap<>();
        headers.put("Cache-Control", "no-store");
        headers.put("X-Content-Type-Options", "nosniff");
        if (!ORIGIN.equals(u.getScheme() + "://" + u.getHost())) return new WebResourceResponse("text/plain", "utf-8", 403, "Forbidden", headers, new ByteArrayInputStream(new byte[0]));
        String path = u.getPath() == null || u.getPath().equals("/") ? "/index.html" : u.getPath();
        if (path.contains("..")) return new WebResourceResponse("text/plain", "utf-8", 404, "Not Found", headers, new ByteArrayInputStream(new byte[0]));
        try {
            if (path.equals("/android-shim.js")) return new WebResourceResponse("text/javascript", "utf-8", 200, "OK", headers, new ByteArrayInputStream(SHIM.getBytes(StandardCharsets.UTF_8)));
            InputStream in = getAssets().open("web" + path, AssetManager.ACCESS_STREAMING);
            if (path.equals("/index.html")) {
                // The bridge shim must exist before the page's own scripts run.
                String html = new String(readAll(in), StandardCharsets.UTF_8).replaceFirst("<script src=\"mail-logic.js\">", "<script src=\"android-shim.js\"></script><script src=\"mail-logic.js\">");
                in = new ByteArrayInputStream(html.getBytes(StandardCharsets.UTF_8));
            }
            return new WebResourceResponse(type(path), type(path).startsWith("text") || path.endsWith(".js") || path.endsWith(".json") ? "utf-8" : null, 200, "OK", headers, in);
        } catch (Exception e) {
            return new WebResourceResponse("text/plain", "utf-8", 404, "Not Found", headers, new ByteArrayInputStream(new byte[0]));
        }
    }
    private static byte[] readAll(InputStream in) throws java.io.IOException {
        ByteArrayOutputStream out = new ByteArrayOutputStream(); byte[] b = new byte[16384]; int n;
        while ((n = in.read(b)) > 0) out.write(b, 0, n);
        in.close(); return out.toByteArray();
    }
    private static String type(String p) {
        String e = p.substring(p.lastIndexOf('.') + 1).toLowerCase();
        switch (e) {
            case "html": return "text/html"; case "js": case "mjs": return "text/javascript"; case "css": return "text/css"; case "json": return "application/json";
            case "png": return "image/png"; case "jpg": case "jpeg": return "image/jpeg"; case "gif": return "image/gif"; case "svg": return "image/svg+xml"; case "webp": return "image/webp";
            case "woff2": return "font/woff2"; case "woff": return "font/woff"; case "ttf": return "font/ttf"; case "mp3": return "audio/mpeg"; case "wav": return "audio/wav"; case "ogg": return "audio/ogg";
            case "glb": return "model/gltf-binary"; case "wasm": return "application/wasm";
            default: return "application/octet-stream";
        }
    }
    boolean isOwnPage() {
        String url = web.getUrl();
        return url != null && (url.equals(ORIGIN) || url.startsWith(ORIGIN + "/"));
    }
    void js(String code) { if (web != null) web.evaluateJavascript(code, null); }

    void openExternal(String url) {
        if (url == null) return;
        try { startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(url)).addCategory(Intent.CATEGORY_BROWSABLE)); }
        catch (ActivityNotFoundException e) { Dialogs.notice(this, "No app on this phone can open that link."); }
    }

    /* ---------------- drafts are saved when you leave ---------------- */
    @Override protected void onPause() {
        super.onPause();
        js("(async()=>{let ok=false;try{ok=window.flushDraft?await window.flushDraft():true}catch(e){}window.webkit.messageHandlers.mail.postMessage({action:'__flushed',id:'pause',ok:!!ok})})()");
    }
    void flushed(boolean ok) { /* The page writes through writeCache; nothing else to do on Android. */ }

    private void back() {
        web.evaluateJavascript("(()=>{const d=[...document.querySelectorAll('dialog[open]')].pop();if(!d)return false;d.dispatchEvent(new Event('cancel',{cancelable:true}))&&d.close();return true})()", r -> {
            if (!"true".equals(r)) moveTaskToBack(true);
        });
    }
    @SuppressWarnings("deprecation")
    @Override public void onBackPressed() { back(); }

    /* ---------------- mailto: links from other apps ---------------- */
    @Override protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        String m = mailto(intent);
        if (m != null) compose(m);
    }
    private static String mailto(Intent intent) {
        if (intent == null || intent.getData() == null || !"mailto".equals(intent.getData().getScheme())) return null;
        return intent.getData().toString();
    }
    private void compose(String mailto) {
        try {
            String rest = mailto.substring("mailto:".length());
            String to = rest.contains("?") ? rest.substring(0, rest.indexOf('?')) : rest;
            Uri q = Uri.parse("mailto:x" + (rest.contains("?") ? rest.substring(rest.indexOf('?')) : ""));
            JSONObject d = new JSONObject().put("email", Uri.decode(to).split(",")[0].trim())
                .put("subject", q.getQueryParameter("subject") == null ? "" : q.getQueryParameter("subject"))
                .put("body", q.getQueryParameter("body") == null ? "" : q.getQueryParameter("body"));
            js("(function t(n){if(typeof compose==='function'&&typeof ready!=='undefined'&&ready&&!document.querySelector('#composer').open)compose(" + d + ");else if(n<60)setTimeout(()=>t(n+1),250)})(0)");
        } catch (Exception ignored) { }
    }

    @Override protected void onActivityResult(int request, int result, Intent data) {
        if (request == 7 && chooser != null) { chooser.onReceiveValue(WebChromeClient.FileChooserParams.parseResult(result, data)); chooser = null; return; }
        super.onActivityResult(request, result, data);
    }

    /* ---------------- updates: a newer APK on GitHub, at most once a day ---------------- */
    private void checkForUpdate() {
        SharedPreferences p = getSharedPreferences("mail", MODE_PRIVATE);
        long now = System.currentTimeMillis();
        if (now - p.getLong("updateChecked", 0) < 86400000L) return;
        p.edit().putLong("updateChecked", now).apply();
        String current;
        try { current = getPackageManager().getPackageInfo(getPackageName(), 0).versionName; } catch (PackageManager.NameNotFoundException e) { return; }
        new Thread(() -> {
            try {
                Net.Reply r = Net.request("https://api.github.com/repos/lattewirell-creator/stupid-mail/releases/latest", null, null, null, null, 15000);
                if (r.status != 200) return;
                JSONObject release = new JSONObject(r.body);
                String tag = release.optString("tag_name").replaceFirst("^v", "");
                JSONArray assets = release.optJSONArray("assets");
                String apk = null;
                for (int i = 0; assets != null && i < assets.length(); i++) {
                    String name = assets.getJSONObject(i).optString("name"), url = assets.getJSONObject(i).optString("browser_download_url");
                    if (name.endsWith("-android.apk") && url.startsWith("https://github.com/lattewirell-creator/stupid-mail/releases/download/")) apk = url;
                }
                if (apk == null || !newer(tag, current)) return;
                final String link = apk;
                runOnUiThread(() -> Dialogs.show(this, "Update available", "The Stupid Mail " + tag + " is out. Download the new APK and open it to update. Your mail and settings stay.",
                    new java.util.ArrayList<>(), new String[]{"Download", "Later"}, (b, v) -> { if (b == 0) openExternal(link); }));
            } catch (Exception ignored) { }
        }).start();
    }
    static boolean newer(String a, String b) {
        String[] x = a.split("\\."), y = b.split("\\.");
        for (int i = 0; i < Math.max(x.length, y.length); i++) {
            int p, q;
            try { p = i < x.length ? Integer.parseInt(x[i].replaceAll("\\D.*", "")) : 0; q = i < y.length ? Integer.parseInt(y[i].replaceAll("\\D.*", "")) : 0; } catch (NumberFormatException e) { return false; }
            if (p != q) return p > q;
        }
        return false;
    }

    @Override protected void onDestroy() {
        if (web != null) { web.removeJavascriptInterface("StupidMailNative"); web.destroy(); web = null; }
        super.onDestroy();
    }
}
