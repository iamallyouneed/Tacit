package io.github.iamallyouneed.tacit;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.ClipData;
import android.content.ClipDescription;
import android.content.ClipboardManager;
import android.content.Context;
import android.inputmethodservice.Keyboard;
import android.os.Bundle;
import android.os.PersistableBundle;
import android.text.Editable;
import android.text.InputType;
import android.text.Selection;
import android.text.SpannableStringBuilder;
import android.view.ContextThemeWrapper;
import android.view.KeyEvent;
import android.view.LayoutInflater;
import android.view.View;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.widget.LinearLayout;
import android.widget.TextView;

/** Real Android views, composing spans and clipboard; no external test framework. */
@SuppressWarnings("deprecation")
public final class KeyboardUxSmoke extends Instrumentation {
    private static final class Service extends TacitIme {
        InputConnection connection;
        EditorInfo info = new EditorInfo();
        Service(Context context) { attachBaseContext(context); }
        @Override public LayoutInflater getLayoutInflater() { return LayoutInflater.from(this); }
        @Override public InputConnection getCurrentInputConnection() { return connection; }
        @Override public EditorInfo getCurrentInputEditorInfo() { return info; }
    }

    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static Object field(Service service, String name) throws Exception {
        java.lang.reflect.Field field = TacitIme.class.getDeclaredField(name);
        field.setAccessible(true);
        return field.get(service);
    }

    private static void type(Service service, String keys) {
        for (char ch : keys.toCharArray()) service.onKey(ch, new int[] {ch});
    }

    private void verify() throws Exception {
        Context context = new ContextThemeWrapper(getTargetContext(), android.R.style.Theme_Material_Light_NoActionBar);
        Service service = new Service(context);
        long handle = Native.nativeCreate(context.getCacheDir().getAbsolutePath() + "/keyboard-ux-smoke");
        check(handle != 0, "native create");
        java.lang.reflect.Field ime = TacitIme.class.getDeclaredField("ime");
        ime.setAccessible(true);
        ime.set(service, handle);
        Editable text = new SpannableStringBuilder("앞😀 뒤");
        Selection.setSelection(text, "앞😀 ".length());
        int[] action = {0};
        int[] cursorKey = {0};
        boolean[] rejectCommit = {false};
        service.connection = new BaseInputConnection(new View(context), true) {
            @Override public Editable getEditable() { return text; }
            @Override public boolean commitText(CharSequence value, int cursor) {
                return !rejectCommit[0] && super.commitText(value, cursor);
            }
            @Override public boolean performEditorAction(int value) { action[0] = value; return true; }
            @Override public boolean sendKeyEvent(KeyEvent event) {
                if (event.getAction() == KeyEvent.ACTION_DOWN) cursorKey[0] = event.getKeyCode();
                return true;
            }
        };
        service.info.inputType = InputType.TYPE_CLASS_TEXT;
        service.info.imeOptions = EditorInfo.IME_ACTION_SEARCH;
        service.info.packageName = "keyboard-ux-smoke";
        ClipboardManager clipboard = (ClipboardManager) context.getSystemService(Context.CLIPBOARD_SERVICE);
        try {
            service.onStartInput(service.info, false);
            View root = service.onCreateInputView();
            service.onStartInputView(service.info, false);
            int width = context.getResources().getDisplayMetrics().widthPixels;
            root.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
                    View.MeasureSpec.makeMeasureSpec(2000, View.MeasureSpec.AT_MOST));
            root.layout(0, 0, width, root.getMeasuredHeight());
            LinearLayout strip = root.findViewById(R.id.readings);
            check(strip.getChildCount() == 3, "idle tools");
            type(service, "dkssud");
            check(text.toString().equals("앞😀 안녕뒤"), "Korean composition: " + text);
            check(strip.getChildAt(0).isSelected(), "selected reading");
            strip.getChildAt(1).performClick();
            check(text.toString().equals("앞😀 dkssud뒤"), "alternative reading uses original native index");
            strip.getChildAt(1).performClick();
            check(text.toString().equals("앞😀 안녕뒤"), "reordered candidates switch back to Korean");
            clipboard.setPrimaryClip(ClipData.newPlainText("test", "붙여넣기😀"));
            root.findViewById(R.id.tools_toggle).performClick();
            strip.getChildAt(0).performClick();
            check(root.findViewById(R.id.clipboard_panel).getVisibility() == View.VISIBLE, "clipboard panel opens");
            TextView preview = root.findViewById(R.id.clipboard_preview);
            check(preview.getText().toString().equals("붙여넣기😀"), "clipboard preview");
            root.findViewById(R.id.clipboard_paste).performClick();
            check(text.toString().equals("앞😀 안녕붙여넣기😀뒤"), "paste preserves composition and suffix: " + text);
            check(BaseInputConnection.getComposingSpanStart(text) == -1, "paste ends composition");
            check(Native.nativeReadings(handle).isEmpty(), "paste clears engine word");
            check(preview.getText().length() == 0, "close clears preview");
            check(root.findViewById(R.id.keyboard).getVisibility() == View.VISIBLE, "keyboard restored");

            Selection.setSelection(text, 0, 1);
            clipboard.setPrimaryClip(ClipData.newPlainText("test", "X"));
            strip.getChildAt(0).performClick();
            root.findViewById(R.id.clipboard_paste).performClick();
            check(text.toString().startsWith("X😀 "), "paste replaces only selection: " + text);

            ClipData secret = ClipData.newPlainText("test", "secret-not-for-preview");
            PersistableBundle extras = new PersistableBundle();
            extras.putBoolean(ClipDescription.EXTRA_IS_SENSITIVE, true);
            secret.getDescription().setExtras(extras);
            clipboard.setPrimaryClip(secret);
            strip.getChildAt(0).performClick();
            check(preview.getText().toString().equals(context.getString(R.string.clipboard_private)), "sensitive clipboard hidden");
            root.findViewById(R.id.clipboard_close).performClick();
            clipboard.setPrimaryClip(ClipData.newPlainText("test", ""));
            strip.getChildAt(0).performClick();
            check(!root.findViewById(R.id.clipboard_paste).isEnabled(), "empty clipboard cannot paste");
            service.onFinishInputView(false);
            check(preview.getText().length() == 0, "hidden keyboard clears clipboard preview");
            clipboard.setPrimaryClip(ClipData.newPlainText("test", "blocked"));
            strip.getChildAt(0).performClick();
            String unchanged = text.toString();
            rejectCommit[0] = true;
            root.findViewById(R.id.clipboard_paste).performClick();
            check(text.toString().equals(unchanged), "rejected paste leaves editor text intact");
            check(preview.getText().toString().equals(context.getString(R.string.paste_unavailable)), "rejected paste feedback");
            check(root.findViewById(R.id.clipboard_panel).getVisibility() == View.VISIBLE, "rejected paste keeps panel open");
            rejectCommit[0] = false;
            root.findViewById(R.id.clipboard_close).performClick();

            text.clear();
            Selection.setSelection(text, 0);
            service.onStartInput(service.info, false);
            service.onStartInputView(service.info, false);
            service.onKey(-101, null); // auto -> Korean -> English
            service.onKey(-101, null);
            service.onKey(Keyboard.KEYCODE_SHIFT, null);
            service.onKey(Keyboard.KEYCODE_SHIFT, null);
            type(service, "ab");
            check(text.toString().equals("AB"), "double Shift locks capitals: " + text);
            check((Boolean) field(service, "capsLocked"), "caps lock state");
            service.onKey(Keyboard.KEYCODE_SHIFT, null);
            type(service, "c");
            check(text.toString().equals("ABc"), "Shift unlocks capitals: " + text);
            service.onKey(10, null);
            Keyboard board = (Keyboard) field(service, "letters");
            check(board.getKeys().stream().anyMatch(k -> k.codes[0] == 10 && "검색".contentEquals(k.label)), "search action label");
            service.onKey(10, null);
            check(action[0] == EditorInfo.IME_ACTION_SEARCH, "search editor action");
            strip.getChildAt(1).performClick();
            strip.getChildAt(1).performClick();
            check(cursorKey[0] == KeyEvent.KEYCODE_DPAD_LEFT, "cursor left reaches editor");
        } finally {
            service.onFinishInputView(false);
            Native.nativeDestroy(handle);
        }
    }

    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }

    @Override public void onStart() {
        Bundle result = new Bundle();
        Throwable[] error = {null};
        runOnMainSync(() -> {
            try { verify(); } catch (Throwable failure) { error[0] = failure; }
        });
        if (error[0] == null) {
            result.putString("stream", "PASS: toolbar/readings, clipboard/selection/privacy, caps lock, Enter, cursor\n");
            finish(Activity.RESULT_OK, result);
        } else {
            result.putString("stream", android.util.Log.getStackTraceString(error[0]));
            finish(Activity.RESULT_CANCELED, result);
        }
    }
}
