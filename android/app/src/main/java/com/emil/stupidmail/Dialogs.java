package com.emil.stupidmail;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.graphics.Typeface;
import android.text.InputType;
import android.util.TypedValue;
import android.view.View;
import android.widget.ArrayAdapter;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.TextView;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/** Native dialogs (the Mac app's sheets): passwords, tokens and keys typed here go straight to the app,
 *  never through the mail interface. Up to three buttons; button 0 is the main one. */
final class Dialogs {
    private Dialogs() {}

    static final class Field {
        final String id, kind, hint, value; final String[] options; final int selected;
        private Field(String id, String kind, String hint, String value, String[] options, int selected) { this.id = id; this.kind = kind; this.hint = hint; this.value = value; this.options = options; this.selected = selected; }
        static Field text(String id, String hint, String value) { return new Field(id, "text", hint, value, null, 0); }
        static Field email(String id, String hint, String value) { return new Field(id, "email", hint, value, null, 0); }
        static Field number(String id, String hint, String value) { return new Field(id, "number", hint, value, null, 0); }
        static Field secret(String id, String hint) { return new Field(id, "password", hint, "", null, 0); }
        static Field choice(String id, String[] options, int selected) { return new Field(id, "select", "", "", options, selected); }
        static Field code(String id, String value) { return new Field(id, "code", "", value, null, 0); }
        static Field preview(String id, String value) { return new Field(id, "preview", "", value, null, 0); }
    }
    interface Done { void done(int button, Map<String, String> values); }

    private static int dp(Context c, float v) { return (int) TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, v, c.getResources().getDisplayMetrics()); }

    /** Shows the dialog; done(-1, …) when it's dismissed without a button. */
    static AlertDialog show(Activity a, String title, String text, List<Field> fields, String[] buttons, Done done) {
        LinearLayout box = new LinearLayout(a); box.setOrientation(LinearLayout.VERTICAL); box.setPadding(dp(a, 22), dp(a, 4), dp(a, 22), 0);
        TextView body = new TextView(a); body.setText(text); body.setTextSize(14); body.setPadding(0, 0, 0, dp(a, 10)); box.addView(body);
        Map<String, View> views = new HashMap<>();
        for (Field f : fields) {
            View v;
            switch (f.kind) {
                case "select": {
                    Spinner s = new Spinner(a); s.setAdapter(new ArrayAdapter<>(a, android.R.layout.simple_spinner_dropdown_item, f.options)); s.setSelection(f.selected); v = s; break;
                }
                case "code": {
                    TextView t = new TextView(a); t.setText(f.value); t.setTextSize(26); t.setTypeface(Typeface.MONOSPACE, Typeface.BOLD); t.setTextIsSelectable(true);
                    t.setPadding(0, dp(a, 6), 0, dp(a, 6)); t.setOnClickListener(x -> { ((ClipboardManager) a.getSystemService(Context.CLIPBOARD_SERVICE)).setPrimaryClip(ClipData.newPlainText("code", f.value)); t.setText(f.value + "  ✓ copied"); });
                    v = t; break;
                }
                case "preview": {
                    TextView t = new TextView(a); t.setText(f.value); t.setTextIsSelectable(true); ScrollView sv = new ScrollView(a); sv.addView(t);
                    sv.setLayoutParams(new LinearLayout.LayoutParams(-1, dp(a, 160))); v = sv; break;
                }
                default: {
                    EditText e = new EditText(a); e.setHint(f.hint); e.setText(f.value); e.setSingleLine(true);
                    e.setInputType(f.kind.equals("password") ? InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD
                        : f.kind.equals("email") ? InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS
                        : f.kind.equals("number") ? InputType.TYPE_CLASS_NUMBER : InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI);
                    if (f.kind.equals("password")) e.setImportantForAutofill(View.IMPORTANT_FOR_AUTOFILL_YES);
                    v = e;
                }
            }
            views.put(f.id, v); box.addView(v);
        }
        ScrollView scroll = new ScrollView(a); scroll.addView(box);
        final boolean[] answered = {false};
        AlertDialog.Builder b = new AlertDialog.Builder(a).setTitle(title).setView(scroll);
        Runnable[] collect = new Runnable[1];
        java.util.function.IntConsumer answer = (i) -> {
            if (answered[0]) return; answered[0] = true;
            Map<String, String> values = new HashMap<>();
            for (Field f : fields) {
                View v = views.get(f.id);
                if (v instanceof EditText) values.put(f.id, ((EditText) v).getText().toString().trim());
                else if (v instanceof Spinner) values.put(f.id, String.valueOf(((Spinner) v).getSelectedItemPosition()));
            }
            for (View v : views.values()) if (v instanceof EditText && ((EditText) v).getInputType() == (InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD)) ((EditText) v).setText("");
            done.done(i, values);
        };
        if (buttons.length > 0) b.setPositiveButton(buttons[0], (d, w) -> answer.accept(0));
        if (buttons.length > 1) b.setNegativeButton(buttons[1], (d, w) -> answer.accept(1));
        if (buttons.length > 2) b.setNeutralButton(buttons[2], (d, w) -> answer.accept(2));
        b.setOnDismissListener(d -> answer.accept(-1));
        return b.show();
    }
    static AlertDialog notice(Activity a, String text) { return show(a, "The Stupid Mail", text, new ArrayList<>(), new String[]{"OK"}, (i, v) -> { }); }
}
