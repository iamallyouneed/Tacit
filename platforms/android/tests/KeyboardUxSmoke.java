package io.github.iamallyouneed.tacit;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.ClipData;
import android.content.ClipDescription;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.SharedPreferences;
import android.content.res.Configuration;
import android.inputmethodservice.Keyboard;
import android.inputmethodservice.KeyboardView;
import android.os.Bundle;
import android.os.PersistableBundle;
import android.os.SystemClock;
import android.text.Editable;
import android.text.InputType;
import android.text.Selection;
import android.text.SpannableStringBuilder;
import android.view.ContextThemeWrapper;
import android.view.KeyEvent;
import android.view.LayoutInflater;
import android.view.MotionEvent;
import android.view.View;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.widget.LinearLayout;
import android.widget.TextView;

import java.util.HashMap;
import java.util.Map;

/** Real Android views, composing spans and clipboard; no external test framework. */
@SuppressWarnings("deprecation")
public final class KeyboardUxSmoke extends Instrumentation {
    private static final class Service extends TacitIme {
        InputConnection connection;
        EditorInfo info = new EditorInfo();
        View inputView;
        Service(Context context) { attachBaseContext(context); }
        @Override public LayoutInflater getLayoutInflater() { return LayoutInflater.from(this); }
        @Override public InputConnection getCurrentInputConnection() { return connection; }
        @Override public EditorInfo getCurrentInputEditorInfo() { return info; }
        @Override public void setInputView(View view) { inputView = view; }
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

    private static void touch(DualKeyboardView view, long start, int action, float x, float y) {
        MotionEvent event = MotionEvent.obtain(start, SystemClock.uptimeMillis(), action, x, y, 0);
        view.onTouchEvent(event);
        event.recycle();
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
        int[] cursorSteps = {0};
        boolean[] rejectCommit = {false};
        service.connection = new BaseInputConnection(new View(context), true) {
            @Override public Editable getEditable() { return text; }
            @Override public boolean commitText(CharSequence value, int cursor) {
                return !rejectCommit[0] && super.commitText(value, cursor);
            }
            @Override public boolean performEditorAction(int value) { action[0] = value; return true; }
            @Override public boolean sendKeyEvent(KeyEvent event) {
                if (event.getAction() == KeyEvent.ACTION_DOWN) {
                    cursorKey[0] = event.getKeyCode();
                    if (event.getKeyCode() == KeyEvent.KEYCODE_DPAD_LEFT) cursorSteps[0]--;
                    if (event.getKeyCode() == KeyEvent.KEYCODE_DPAD_RIGHT) cursorSteps[0]++;
                }
                return true;
            }
        };
        service.info.inputType = InputType.TYPE_CLASS_TEXT;
        service.info.imeOptions = EditorInfo.IME_ACTION_SEARCH;
        service.info.packageName = "keyboard-ux-smoke";
        ClipboardManager clipboard = (ClipboardManager) context.getSystemService(Context.CLIPBOARD_SERVICE);
        SharedPreferences settings = context.getSharedPreferences("keyboard", Context.MODE_PRIVATE);
        Map<String, ?> originalSettings = new HashMap<>(settings.getAll());
        settings.edit().putInt("theme", 0).putInt("height", 1).commit();
        try {
            service.onStartInput(service.info, false);
            View root = service.onCreateInputView();
            service.onStartInputView(service.info, false);
            int width = context.getResources().getDisplayMetrics().widthPixels;
            root.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
                    View.MeasureSpec.makeMeasureSpec(2000, View.MeasureSpec.AT_MOST));
            root.layout(0, 0, width, root.getMeasuredHeight());
            LinearLayout strip = root.findViewById(R.id.readings);
            int candidateTop = root.findViewById(R.id.readings_scroll).getTop();
            int keyboardHeight = root.getMeasuredHeight();
            check(root.findViewById(R.id.paste_tool).getVisibility() == View.VISIBLE, "direct paste visible");
            type(service, "dkssud");
            check(text.toString().equals("앞😀 안녕뒤"), "Korean composition: " + text);
            check(strip.getChildAt(0).isSelected(), "selected reading");
            check(root.findViewById(R.id.clipboard_tool).getVisibility() == View.VISIBLE, "clipboard stays visible while typing");
            check(root.findViewById(R.id.readings_scroll).getTop() == candidateTop, "candidate position stable");
            strip.getChildAt(1).performClick();
            check(text.toString().equals("앞😀 dkssud뒤"), "alternative reading uses original native index");
            strip.getChildAt(1).performClick();
            check(text.toString().equals("앞😀 안녕뒤"), "reordered candidates switch back to Korean");
            clipboard.setPrimaryClip(ClipData.newPlainText("test", "붙여넣기😀"));
            root.findViewById(R.id.clipboard_tool).performClick();
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
            root.findViewById(R.id.paste_tool).performClick();
            check(text.toString().startsWith("X😀 "), "paste replaces only selection: " + text);

            ClipData secret = ClipData.newPlainText("test", "secret-not-for-preview");
            PersistableBundle extras = new PersistableBundle();
            extras.putBoolean(ClipDescription.EXTRA_IS_SENSITIVE, true);
            secret.getDescription().setExtras(extras);
            clipboard.setPrimaryClip(secret);
            root.findViewById(R.id.clipboard_tool).performClick();
            check(preview.getText().toString().equals(context.getString(R.string.clipboard_private)), "sensitive clipboard hidden");
            root.findViewById(R.id.clipboard_close).performClick();
            clipboard.setPrimaryClip(ClipData.newPlainText("test", ""));
            root.findViewById(R.id.clipboard_tool).performClick();
            check(!root.findViewById(R.id.clipboard_paste).isEnabled(), "empty clipboard cannot paste");
            service.onFinishInputView(false);
            check(preview.getText().length() == 0, "hidden keyboard clears clipboard preview");
            clipboard.setPrimaryClip(ClipData.newPlainText("test", "blocked"));
            root.findViewById(R.id.clipboard_tool).performClick();
            String unchanged = text.toString();
            rejectCommit[0] = true;
            root.findViewById(R.id.clipboard_paste).performClick();
            check(text.toString().equals(unchanged), "rejected paste leaves editor text intact");
            check(preview.getText().toString().equals(context.getString(R.string.paste_unavailable)), "rejected paste feedback");
            check(root.findViewById(R.id.clipboard_panel).getVisibility() == View.VISIBLE, "rejected paste keeps panel open");
            rejectCommit[0] = false;
            root.findViewById(R.id.clipboard_close).performClick();

            Selection.setSelection(text, 0, 1);
            root.findViewById(R.id.emoji_tool).performClick();
            root.measure(View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
                    View.MeasureSpec.makeMeasureSpec(2000, View.MeasureSpec.AT_MOST));
            root.layout(0, 0, width, root.getMeasuredHeight());
            check(root.getMeasuredHeight() == keyboardHeight, "emoji panel keeps keyboard height");
            check(preview.getText().length() == 0, "emoji panel clears clipboard preview");
            LinearLayout rows = root.findViewById(R.id.emoji_rows);
            ((LinearLayout) rows.getChildAt(0)).getChildAt(0).performClick();
            check(text.toString().startsWith("😀😀 "), "emoji replaces only selection: " + text);
            root.findViewById(R.id.tools_toggle).performClick();
            check(root.findViewById(R.id.emoji_panel).getVisibility() == View.GONE, "back closes emoji panel");

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
            DualKeyboardView view = root.findViewById(R.id.keyboard);
            // Isolated views need the framework's attachment hook to initialize touch handling.
            java.lang.reflect.Method attach = KeyboardView.class.getDeclaredMethod("onAttachedToWindow");
            attach.setAccessible(true);
            attach.invoke(view);
            Keyboard.Key space = board.getKeys().stream().filter(k -> k.codes[0] == 32).findFirst().get();
            float density = context.getResources().getDisplayMetrics().density;
            float x = view.getPaddingLeft() + space.x + space.width / 2f;
            float y = view.getPaddingTop() + space.y + space.height / 2f;
            long start = SystemClock.uptimeMillis();
            unchanged = text.toString();
            touch(view, start, MotionEvent.ACTION_DOWN, x, y);
            touch(view, start, MotionEvent.ACTION_MOVE, x + 48 * density, y);
            touch(view, start, MotionEvent.ACTION_MOVE, x - 16 * density, y);
            touch(view, start, MotionEvent.ACTION_UP, x - 16 * density, y);
            check(cursorSteps[0] == -1, "drag advances and reverses in character steps: " + cursorSteps[0]);
            check(cursorKey[0] == KeyEvent.KEYCODE_DPAD_LEFT, "drag left reaches editor");
            check(text.toString().equals(unchanged), "space drag inserts no text");
            start = SystemClock.uptimeMillis();
            touch(view, start, MotionEvent.ACTION_DOWN, x, y);
            touch(view, start, MotionEvent.ACTION_MOVE, x + density, y);
            touch(view, start, MotionEvent.ACTION_UP, x + density, y);
            check(text.toString().equals(unchanged + " "), "small movement still types one space: " + text);
            start = SystemClock.uptimeMillis();
            unchanged = text.toString();
            touch(view, start, MotionEvent.ACTION_DOWN, x, y);
            touch(view, start, MotionEvent.ACTION_CANCEL, x, y);
            check(text.toString().equals(unchanged), "cancelled touch inserts nothing");
            type(service, "hello");
            unchanged = text.toString();
            start = SystemClock.uptimeMillis();
            touch(view, start, MotionEvent.ACTION_DOWN, x, y);
            touch(view, start, MotionEvent.ACTION_MOVE, x - 32 * density, y);
            touch(view, start, MotionEvent.ACTION_CANCEL, x - 32 * density, y);
            check(text.toString().equals(unchanged), "drag preserves displayed composition");
            check(Native.nativeReadings(handle).isEmpty(), "drag flushes native word before cursor movement");
            check(BaseInputConnection.getComposingSpanStart(text) == -1, "drag detaches composing span");
            service.onKey(-102, null);
            unchanged = text.toString();
            int stepsBeforeSymbols = cursorSteps[0];
            start = SystemClock.uptimeMillis();
            touch(view, start, MotionEvent.ACTION_DOWN, x, y);
            touch(view, start, MotionEvent.ACTION_MOVE, x + 32 * density, y);
            touch(view, start, MotionEvent.ACTION_UP, x + 32 * density, y);
            check(cursorSteps[0] == stepsBeforeSymbols + 2 && text.toString().equals(unchanged),
                    "symbols keyboard supports space drag without text");
            service.onKey(-103, null);
            java.lang.reflect.Method detach = KeyboardView.class.getDeclaredMethod("onDetachedFromWindow");
            detach.setAccessible(true);
            detach.invoke(view);

            service.onKey(-101, null); // English -> auto
            type(service, "dkssud");
            String pendingReadings = Native.nativeReadings(handle);
            check(!pendingReadings.isEmpty(), "pending reading before changing settings");
            unchanged = text.toString();
            int caret = Selection.getSelectionEnd(text);
            int regularHeight = ((Keyboard) field(service, "letters")).getHeight();
            root.findViewById(R.id.more_tool).performClick();
            root.findViewById(R.id.theme_dark).performClick();
            root = service.inputView;
            check(root != null && root.findViewById(R.id.settings_panel).getVisibility() == View.VISIBLE,
                    "settings stays open after changing theme");
            check((root.getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK)
                    == Configuration.UI_MODE_NIGHT_YES, "explicit dark theme overrides system");
            check(root.getContext().getColor(R.color.keyboard_background) == 0xff080808, "dark background is neutral black");
            check(text.toString().equals(unchanged) && Selection.getSelectionEnd(text) == caret,
                    "theme change preserves text and caret");
            check(Native.nativeReadings(handle).equals(pendingReadings), "theme change preserves pending word");
            root.findViewById(R.id.height_tall).performClick();
            root = service.inputView;
            check(((Keyboard) field(service, "letters")).getHeight() > regularHeight, "tall setting changes key geometry");
            root.findViewById(R.id.height_compact).performClick();
            root = service.inputView;
            check(((Keyboard) field(service, "letters")).getHeight() < regularHeight, "compact setting changes key geometry");
            service.onKey(-102, null);
            root.findViewById(R.id.theme_light).performClick();
            root = service.inputView;
            check(((DualKeyboardView) field(service, "keyboardView")).getKeyboard() == field(service, "symbols"),
                    "theme change preserves symbol layout");
            service.onKey(-103, null);
            check((root.getResources().getConfiguration().uiMode & Configuration.UI_MODE_NIGHT_MASK)
                    == Configuration.UI_MODE_NIGHT_NO, "light theme applies immediately");
            root.findViewById(R.id.preview_setting).performClick();
            check(settings.getBoolean("preview", true) == (Boolean) field(service, "keyPreviewEnabled"), "preview setting saved");
            root.findViewById(R.id.haptics_setting).performClick();
            check(settings.getBoolean("haptics", true) == (Boolean) field(service, "hapticsEnabled"), "haptics setting saved");
            check(text.toString().equals(unchanged) && Native.nativeReadings(handle).equals(pendingReadings),
                    "height and feedback settings preserve composition");
            service.onFinishInputView(false);
            View restored = service.onCreateInputView();
            check(restored.findViewById(R.id.theme_light).isSelected()
                    && restored.findViewById(R.id.height_compact).isSelected(), "theme and height survive view recreation");
            check(((android.widget.Switch) restored.findViewById(R.id.preview_setting)).isChecked()
                    == settings.getBoolean("preview", true), "preview survives view recreation");
        } finally {
            service.onFinishInputView(false);
            Native.nativeDestroy(handle);
            SharedPreferences.Editor restore = settings.edit();
            for (String key : new String[] {"theme", "height", "haptics", "preview"}) {
                Object value = originalSettings.get(key);
                if (value instanceof Integer) restore.putInt(key, (Integer) value);
                else if (value instanceof Boolean) restore.putBoolean(key, (Boolean) value);
                else restore.remove(key);
            }
            restore.commit();
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
