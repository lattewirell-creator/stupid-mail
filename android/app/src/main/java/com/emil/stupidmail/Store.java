package com.emil.stupidmail;

import android.content.Context;
import android.content.SharedPreferences;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;
import org.json.JSONTokener;

import java.io.File;
import java.io.FileOutputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.security.KeyStore;
import java.security.MessageDigest;
import java.util.Base64;

import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/** Local storage on Android (macos/MailCache.swift, the Keychain stores and UserDefaults): settings, the mail
 *  cache per address (app-private files), and secrets (passwords, tokens, AI keys, webhook URLs) encrypted with a
 *  key in the Android Keystore that never leaves the phone's secure hardware. Nothing here is backed up. */
final class Store {
    private final SharedPreferences prefs, secrets;
    private final File accounts;
    private static final String KEY = "stupid-mail-secrets";

    Store(Context c) {
        prefs = c.getSharedPreferences("mail", Context.MODE_PRIVATE);
        secrets = c.getSharedPreferences("secrets", Context.MODE_PRIVATE);
        accounts = new File(c.getFilesDir(), "Accounts");
        accounts.mkdirs();
    }

    String setting(String key) { return prefs.getString(key, ""); }
    void setSetting(String key, String value) { if (value == null) prefs.edit().remove(key).apply(); else prefs.edit().putString(key, value).apply(); }

    /* ---------------- mail cache ---------------- */
    private File cache(String account) throws MailCore.MailFailure {
        if (!(account.isEmpty() || MailCore.Validation.address(account))) throw new MailCore.MailFailure("Invalid local account.");
        try {
            byte[] hash = MessageDigest.getInstance("SHA-256").digest((account.isEmpty() ? "demo" : account.toLowerCase()).getBytes(StandardCharsets.UTF_8));
            StringBuilder hex = new StringBuilder(); for (byte b : hash) hex.append(String.format("%02x", b));
            return new File(accounts, hex + ".json");
        } catch (Exception e) { throw new MailCore.MailFailure("Invalid local account."); }
    }
    private static Object decode(String text) throws MailCore.MailFailure {
        try { if (text.length() <= 80 * 1024 * 1024) { Object v = new JSONTokener(text).nextValue(); if (v instanceof JSONArray) return v; } } catch (JSONException ignored) { }
        throw new MailCore.MailFailure("Invalid local mailbox data.");
    }
    Object readCache(String account) throws MailCore.MailFailure {
        File f = cache(account);
        if (!f.exists()) return JSONObject.NULL;
        try { return decode(new String(Files.readAllBytes(f.toPath()), StandardCharsets.UTF_8)); }
        catch (MailCore.MailFailure e) { throw e; } catch (Exception e) { throw new MailCore.MailFailure("Could not read the local mailbox."); }
    }
    void writeCache(String account, String contents) throws MailCore.MailFailure {
        decode(contents);
        File f = cache(account), tmp = new File(f.getPath() + ".tmp");
        try (FileOutputStream o = new FileOutputStream(tmp)) { o.write(contents.getBytes(StandardCharsets.UTF_8)); o.getFD().sync(); }
        catch (Exception e) { throw new MailCore.MailFailure("Could not save your mailbox on this phone. Check free space before closing the app."); }
        if (!tmp.renameTo(f)) throw new MailCore.MailFailure("Could not save your mailbox on this phone. Check free space before closing the app.");
    }

    /* ---------------- secrets ---------------- */
    private static SecretKey key() throws Exception {
        KeyStore ks = KeyStore.getInstance("AndroidKeyStore"); ks.load(null);
        if (ks.containsAlias(KEY)) return ((KeyStore.SecretKeyEntry) ks.getEntry(KEY, null)).getSecretKey();
        KeyGenerator g = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
        g.init(new KeyGenParameterSpec.Builder(KEY, KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).setKeySize(256).build());
        return g.generateKey();
    }
    void saveSecret(String name, JSONObject value) throws MailCore.MailFailure {
        try {
            Cipher c = Cipher.getInstance("AES/GCM/NoPadding"); c.init(Cipher.ENCRYPT_MODE, key());
            byte[] sealed = c.doFinal(value.toString().getBytes(StandardCharsets.UTF_8)), iv = c.getIV();
            Base64.Encoder b = Base64.getEncoder();
            if (!secrets.edit().putString(name, b.encodeToString(iv) + ":" + b.encodeToString(sealed)).commit()) throw new Exception("not saved");
        } catch (Exception e) { throw new MailCore.MailFailure("This phone could not protect this secret. Nothing was saved."); }
    }
    JSONObject loadSecret(String name) {
        String stored = secrets.getString(name, null);
        if (stored == null || !stored.contains(":")) return null;
        try {
            String[] parts = stored.split(":", 2); Base64.Decoder b = Base64.getDecoder();
            Cipher c = Cipher.getInstance("AES/GCM/NoPadding"); c.init(Cipher.DECRYPT_MODE, key(), new GCMParameterSpec(128, b.decode(parts[0])));
            return new JSONObject(new String(c.doFinal(b.decode(parts[1])), StandardCharsets.UTF_8));
        } catch (Exception e) { return null; }
    }
    void removeSecret(String name) { secrets.edit().remove(name).commit(); }
}
