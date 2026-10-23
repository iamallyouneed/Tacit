package io.github.iamallyouneed.tacit;

import android.content.ClipData;
import android.content.ClipDescription;
import android.content.ClipboardManager;
import android.inputmethodservice.InputMethodService;
import android.inputmethodservice.Keyboard;
import android.inputmethodservice.KeyboardView;
import android.text.InputType;
import android.text.TextUtils;
import android.os.SystemClock;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.HapticFeedbackConstants;
import android.view.KeyCharacterMap;
import android.view.KeyEvent;
import android.view.View;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import android.widget.LinearLayout;
import android.widget.Button;
import android.widget.ImageButton;
import android.widget.PopupMenu;
import android.widget.TextView;

import java.util.HashSet;
import java.util.Set;

/**
 * Input from a hardware keyboard and from the on-screen one: keys go to Rust, its edit
 * commands come back here. The on-screen keyboard is the framework's (deprecated, but in every
 * Android version since 1.5 and not a dependency).
 */
@SuppressWarnings("deprecation")
public class TacitIme extends InputMethodService implements KeyboardView.OnKeyboardActionListener {
    private static final int BEFORE_CARET = 64;
    private static final int CODE_MODE = -101;
    private static final int CODE_SYMBOLS = -102;
    private static final int CODE_LETTERS = -103;

    private long ime;
    private boolean active;
    private boolean composing;
    /** Keys whose down event was consumed: their up event is too. */
    private final Set<Integer> swallowed = new HashSet<>();

    private DualKeyboardView keyboardView;
    private LinearLayout readingsView;
    /** What the chips were built from, so refresh() rebuilds them only when it changes. */
    private String shownReadings;
    private Keyboard letters;
    private Keyboard symbols;
    private View clipboardPanel;
    private TextView clipboardPreview;
    private Button clipboardPaste;
    private ImageButton toolsToggle;
    private boolean toolsOpen;
    private boolean cursorTools;
    private boolean hapticsEnabled = true;
    private boolean keyPreviewEnabled = true;
    private boolean capsLocked;
    private long lastShiftTap;
    private ClipboardManager clipboardManager;
    private final ClipboardManager.OnPrimaryClipChangedListener clipboardChanged = () -> {
        if (clipboardPanel != null && clipboardPanel.getVisibility() == View.VISIBLE) openClipboard();
    };

    @Override
    public void onCreate() {
        super.onCreate();
        ime = Native.nativeCreate(getFilesDir().getAbsolutePath());
    }

    @Override
    public void onDestroy() {
        closeClipboard();
        Native.nativeDestroy(ime);
        ime = 0;
        super.onDestroy();
    }

    @Override
    public View onCreateInputView() {
        letters = new Keyboard(this, R.xml.qwerty);
        symbols = new Keyboard(this, R.xml.symbols);
        View root = getLayoutInflater().inflate(R.layout.input, null);
        readingsView = root.findViewById(R.id.readings);
        toolsToggle = root.findViewById(R.id.tools_toggle);
        clipboardPanel = root.findViewById(R.id.clipboard_panel);
        clipboardPreview = root.findViewById(R.id.clipboard_preview);
        clipboardPaste = root.findViewById(R.id.clipboard_paste);
        clipboardManager = (ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
        hapticsEnabled = getSharedPreferences("keyboard", MODE_PRIVATE).getBoolean("haptics", true);
        keyPreviewEnabled = getSharedPreferences("keyboard", MODE_PRIVATE).getBoolean("preview", true);
        shownReadings = null;
        keyboardView = root.findViewById(R.id.keyboard);
        keyboardView.setOnKeyboardActionListener(this);
        keyboardView.setPreviewEnabled(keyPreviewEnabled);
        keyboardView.setKeyboard(letters);
        toolsToggle.setOnClickListener(v -> {
            feedback();
            if (clipboardPanel.getVisibility() == View.VISIBLE) {
                closeClipboard();
                refresh();
            } else if (!active || Native.nativeReadings(ime).isEmpty()) {
                requestHideSelf(0);
            } else {
                toolsOpen = !toolsOpen;
                cursorTools = false;
                showReadings();
            }
        });
        root.findViewById(R.id.keyboard_menu).setOnClickListener(v -> showModeMenu(v));
        root.findViewById(R.id.clipboard_close).setOnClickListener(v -> {
            feedback();
            closeClipboard();
            refresh();
        });
        clipboardPaste.setOnClickListener(v -> pasteClipboard());

        return root;
    }

    @Override
    public void onStartInput(EditorInfo info, boolean restarting) {
        super.onStartInput(info, restarting);
        active = ime != 0 && isTextField(info);
        composing = false;
        swallowed.clear();
        toolsOpen = false;
        closeClipboard();
        if (active) Native.nativeActivate(ime, info.packageName);
    }

    @Override
    public void onStartInputView(EditorInfo info, boolean restarting) {
        super.onStartInputView(info, restarting);
        closeClipboard();
        if (keyboardView != null) {
            capsLocked = false;
            lastShiftTap = 0;
            letters.setShifted(false);
            updateShiftLabel();
            keyboardView.setKeyboard(letters);
        }
        refresh();
    }

    @Override
    public void onFinishInput() {
        closeClipboard();
        composing = false;
        InputConnection ic = getCurrentInputConnection();
        if (ic != null) ic.finishComposingText();
        if (ime != 0) Native.nativeReset(ime);
        super.onFinishInput();
    }

    @Override
    public void onFinishInputView(boolean finishingInput) {
        closeClipboard();
        super.onFinishInputView(finishingInput);
    }

    @Override
    public void onUpdateSelection(int oldSelStart, int oldSelEnd, int newSelStart, int newSelEnd,
            int candidatesStart, int candidatesEnd) {
        super.onUpdateSelection(oldSelStart, oldSelEnd, newSelStart, newSelEnd,
                candidatesStart, candidatesEnd);
        if (!active || !composing) return;
        if (newSelStart == candidatesEnd && newSelEnd == candidatesEnd) return;
        // A moved caret must detach both the editor's composing span and Rust's old word.
        composing = false;
        Native.nativeReset(ime);
        InputConnection ic = getCurrentInputConnection();
        if (ic != null) ic.finishComposingText();
        refresh();
    }

    /** Passwords and fields that are not text are left to the system. */
    private static boolean isTextField(EditorInfo info) {
        int type = info.inputType;
        if ((type & InputType.TYPE_MASK_CLASS) != InputType.TYPE_CLASS_TEXT) return false;
        switch (type & InputType.TYPE_MASK_VARIATION) {
            case InputType.TYPE_TEXT_VARIATION_PASSWORD:
            case InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD:
            case InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD:
                return false;
            default:
                return true;
        }
    }

    // MARK: - Hardware keyboard

    @Override
    public boolean onKeyDown(int keyCode, KeyEvent event) {
        InputConnection ic = getCurrentInputConnection();
        if (!active || ic == null || isModifier(keyCode)) return super.onKeyDown(keyCode, event);

        boolean consumed;
        if (keyCode == KeyEvent.KEYCODE_LANGUAGE_SWITCH) {
            apply(ic, Native.nativeCycleMode(ime, before(ic)));
            consumed = true;
        } else {
            int kind = kindOf(keyCode, event);
            consumed = send(ic, kind, charOf(keyCode, event, kind), keyCode, false);
        }
        refresh();
        if (consumed) {
            swallowed.add(keyCode);
            return true;
        }
        return super.onKeyDown(keyCode, event);
    }

    @Override
    public boolean onKeyUp(int keyCode, KeyEvent event) {
        if (swallowed.remove(keyCode)) return true;
        return super.onKeyUp(keyCode, event);
    }

    private static boolean isModifier(int keyCode) {
        switch (keyCode) {
            case KeyEvent.KEYCODE_SHIFT_LEFT:
            case KeyEvent.KEYCODE_SHIFT_RIGHT:
            case KeyEvent.KEYCODE_CTRL_LEFT:
            case KeyEvent.KEYCODE_CTRL_RIGHT:
            case KeyEvent.KEYCODE_ALT_LEFT:
            case KeyEvent.KEYCODE_ALT_RIGHT:
            case KeyEvent.KEYCODE_META_LEFT:
            case KeyEvent.KEYCODE_META_RIGHT:
            case KeyEvent.KEYCODE_CAPS_LOCK:
            case KeyEvent.KEYCODE_NUM_LOCK:
                return true;
            default:
                return false;
        }
    }

    private static int kindOf(int keyCode, KeyEvent event) {
        if (event.isCtrlPressed() || event.isAltPressed() || event.isMetaPressed()) {
            return Native.OTHER; // a shortcut: the word goes in, the app gets the key
        }
        if (keyCode >= KeyEvent.KEYCODE_A && keyCode <= KeyEvent.KEYCODE_Z) return Native.LETTER;
        switch (keyCode) {
            case KeyEvent.KEYCODE_DEL:
                return Native.DELETE;
            case KeyEvent.KEYCODE_ESCAPE:
                return event.isShiftPressed() ? Native.CYCLE : Native.ESCAPE;
            case KeyEvent.KEYCODE_ENTER:
            case KeyEvent.KEYCODE_NUMPAD_ENTER:
                return Native.ENTER;
            case KeyEvent.KEYCODE_TAB:
                return event.isShiftPressed() ? Native.OTHER : Native.TAB;
            default:
                int c = event.getUnicodeChar();
                boolean text = c >= 32 && (c & KeyCharacterMap.COMBINING_ACCENT) == 0;
                return text ? Native.PRINTABLE : Native.OTHER;
        }
    }

    private static int charOf(int keyCode, KeyEvent event, int kind) {
        if (kind == Native.LETTER) {
            char c = (char) ('a' + keyCode - KeyEvent.KEYCODE_A);
            return event.isShiftPressed() != event.isCapsLockOn() ? Character.toUpperCase(c) : c;
        }
        return kind == Native.PRINTABLE ? event.getUnicodeChar() : 0;
    }

    // MARK: - On-screen keyboard

    @Override
    public void onKey(int code, int[] keyCodes) {
        switch (code) {
            case Keyboard.KEYCODE_SHIFT:
                long now = SystemClock.uptimeMillis();
                if (capsLocked) {
                    capsLocked = false;
                    letters.setShifted(false);
                } else if (letters.isShifted() && now - lastShiftTap < 350) {
                    capsLocked = true;
                } else {
                    letters.setShifted(!letters.isShifted());
                }
                lastShiftTap = now;
                updateShiftLabel();
                keyboardView.invalidateAllKeys();
                return;
            case CODE_SYMBOLS:
                keyboardView.setKeyboard(symbols);
                refresh();
                return;
            case CODE_LETTERS:
                keyboardView.setKeyboard(letters);
                refresh();
                return;
            case CODE_MODE:
                InputConnection ic = getCurrentInputConnection();
                if (ic != null && active) apply(ic, Native.nativeCycleMode(ime, before(ic)));
                refresh();
                return;
            case Keyboard.KEYCODE_DELETE:
                soft(Native.DELETE, 0, KeyEvent.KEYCODE_DEL);
                return;
            case 10:
                soft(Native.ENTER, 0, KeyEvent.KEYCODE_ENTER);
                return;
            default:
                break;
        }
        char c = (char) code;
        if (Character.isLetter(c)) {
            boolean shifted = keyboardView.getKeyboard() == letters && letters.isShifted();
            soft(Native.LETTER, shifted ? Character.toUpperCase(c) : c, 0);
            if (shifted && !capsLocked) {
                letters.setShifted(false); // one capital at a time
                updateShiftLabel();
                keyboardView.invalidateAllKeys();
            }
        } else {
            soft(Native.PRINTABLE, c, 0);
        }
    }

    private void soft(int kind, int ch, int keyCode) {
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return;
        if (active) {
            send(ic, kind, ch, keyCode, true);
        } else {
            passThrough(ic, kind, ch, keyCode); // passwords and the like: no input method logic
        }
        refresh();
    }

    @Override
    public void onPress(int primaryCode) {
        feedback();
    }

    @Override
    public void onRelease(int primaryCode) {}

    @Override
    public void onText(CharSequence text) {}

    @Override
    public void swipeLeft() {}

    @Override
    public void swipeRight() {}

    @Override
    public void swipeDown() {}

    @Override
    public void swipeUp() {}

    /** The mode key shows the mode; the strip above the keys shows the word's readings. */
    private void refresh() {
        if (keyboardView == null || ime == 0) return;
        int mode = active ? Native.nativeMode(ime) : Native.MODE_AUTO;
        String label = active ? "한/영" : "ABC";
        boolean changed = false;
        for (Keyboard board : new Keyboard[] {letters, symbols}) {
            for (Keyboard.Key key : board.getKeys()) {
                String next = null;
                if (key.codes[0] == CODE_MODE) next = label;
                if (key.codes[0] == 32) next = "tacit · " + (active ? modeLabel(mode) : "English");
                if (key.codes[0] == 10) {
                    String action = enterLabel();
                    CharSequence text = "⏎".equals(action) ? null : action;
                    if (!TextUtils.equals(key.label, text) || (text == null && key.icon == null)) {
                        key.label = text;
                        key.icon = text == null ? getDrawable(R.drawable.ic_corner_down_left) : null;
                        changed = true;
                    }
                    key.on = true;
                    continue;
                }
                if (next != null && !next.contentEquals(key.label)) {
                    key.label = next;
                    changed = true;
                }
            }
        }
        keyboardView.setMode(active ? mode : Native.MODE_ENGLISH);
        if (changed) keyboardView.invalidateAllKeys();
        showReadings();
    }

    private String modeLabel(int mode) {
        switch (mode) {
            case Native.MODE_KOREAN:
                return getString(R.string.mode_korean);
            case Native.MODE_ENGLISH:
                return "English";
            default:
                return getString(R.string.mode_auto);
        }
    }

    private String enterLabel() {
        if (composing) return getString(R.string.confirm);
        EditorInfo info = getCurrentInputEditorInfo();
        if (info == null || (info.imeOptions & EditorInfo.IME_FLAG_NO_ENTER_ACTION) != 0) return "⏎";
        switch (info.imeOptions & EditorInfo.IME_MASK_ACTION) {
            case EditorInfo.IME_ACTION_SEARCH: return getString(R.string.action_search);
            case EditorInfo.IME_ACTION_SEND: return getString(R.string.action_send);
            case EditorInfo.IME_ACTION_NEXT: return getString(R.string.action_next);
            case EditorInfo.IME_ACTION_DONE: return getString(R.string.action_done);
            case EditorInfo.IME_ACTION_GO: return getString(R.string.action_go);
            case EditorInfo.IME_ACTION_PREVIOUS: return getString(R.string.action_previous);
            default: return "⏎";
        }
    }

    private void updateShiftLabel() {
        for (Keyboard.Key key : letters.getKeys()) {
            if (key.codes[0] == Keyboard.KEYCODE_SHIFT) {
                key.label = null;
                key.icon = getDrawable(capsLocked ? R.drawable.ic_arrow_big_up_dash : R.drawable.ic_arrow_big_up);
                key.on = letters.isShifted();
            }
        }
    }

    private void feedback() {
        if (hapticsEnabled && keyboardView != null) {
            keyboardView.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP);
        }
    }

    private void showReadings() {
        String readings = active ? Native.nativeReadings(ime) : "";
        boolean clipboard = clipboardPanel.getVisibility() == View.VISIBLE;
        boolean tools = clipboard || toolsOpen || readings.isEmpty();
        String state = tools ? "tools:" + cursorTools + ":" + clipboard : readings;
        toolsToggle.setImageResource(clipboard ? R.drawable.ic_keyboard : readings.isEmpty()
                ? R.drawable.ic_chevron_down : toolsOpen ? R.drawable.ic_chevron_left : R.drawable.ic_ellipsis);
        toolsToggle.setContentDescription(getString(clipboard ? R.string.back_to_keyboard : readings.isEmpty()
                ? R.string.hide_keyboard : toolsOpen ? R.string.readings : R.string.tools));
        if (state.equals(shownReadings)) return; // same controls: no rebuild per key
        shownReadings = state;
        readingsView.removeAllViews();
        if (tools) {
            if (cursorTools) {
                addStripIcon(R.drawable.ic_chevron_left, R.string.tools, v -> { cursorTools = false; showReadings(); });
                addStripIcon(R.drawable.ic_arrow_left, R.string.cursor_left, v -> moveCursor(KeyEvent.KEYCODE_DPAD_LEFT));
                addStripIcon(R.drawable.ic_arrow_right, R.string.cursor_right, v -> moveCursor(KeyEvent.KEYCODE_DPAD_RIGHT));
            } else {
                ImageButton clip = addStripIcon(R.drawable.ic_clipboard, R.string.clipboard, v -> openClipboard());
                clip.setSelected(clipboard);
                addStripIcon(R.drawable.ic_move_horizontal, R.string.cursor_tools, v -> { cursorTools = true; showReadings(); });
                addStripIcon(R.drawable.ic_settings_2, R.string.keyboard_settings, v -> showSettings(v));
            }
            return;
        }
        String[] parts = readings.split("\u0001");
        for (boolean selected : new boolean[] {true, false}) {
            for (int i = 1; i < parts.length; i++) {
                if ((parts[i].charAt(0) == '1') != selected) continue;
                addReading(parts[i].substring(1), selected, i - 1);
            }
        }
    }

    private ImageButton addStripIcon(int icon, int description, View.OnClickListener action) {
        ImageButton button = new ImageButton(this);
        button.setImageResource(icon);
        button.setContentDescription(getString(description));
        int padding = (int) (12 * getResources().getDisplayMetrics().density);
        button.setPadding(padding, padding, padding, padding);
        button.setScaleType(android.widget.ImageView.ScaleType.CENTER);
        button.setMinimumWidth(padding * 4);
        button.setBackgroundResource(R.drawable.reading_chip);
        button.setOnClickListener(v -> { feedback(); action.onClick(v); });
        readingsView.addView(button, new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1));
        return button;
    }

    private void showModeMenu(View anchor) {
        feedback();
        PopupMenu menu = new PopupMenu(this, anchor);
        for (int mode = Native.MODE_AUTO; mode <= Native.MODE_ENGLISH; mode++) {
            menu.getMenu().add(0, mode, mode, modeLabel(mode)).setCheckable(true)
                    .setChecked(active && Native.nativeMode(ime) == mode).setEnabled(active);
        }
        menu.getMenu().setGroupCheckable(0, true, true);
        menu.setOnMenuItemClickListener(item -> {
            InputConnection ic = getCurrentInputConnection();
            if (ic == null || !active) return false;
            for (int i = 0; i < 3 && Native.nativeMode(ime) != item.getItemId(); i++) {
                apply(ic, Native.nativeCycleMode(ime, before(ic)));
            }
            refresh();
            return true;
        });
        menu.show();
    }

    private void showSettings(View anchor) {
        PopupMenu menu = new PopupMenu(this, anchor);
        menu.getMenu().add(0, 1, 0, R.string.haptics).setCheckable(true).setChecked(hapticsEnabled);
        menu.getMenu().add(0, 2, 1, R.string.key_preview).setCheckable(true).setChecked(keyPreviewEnabled);
        menu.getMenu().add(0, 3, 2, R.string.choose_keyboard);
        menu.setOnMenuItemClickListener(item -> {
            if (item.getItemId() == 1) {
                hapticsEnabled = !hapticsEnabled;
                getSharedPreferences("keyboard", MODE_PRIVATE).edit().putBoolean("haptics", hapticsEnabled).apply();
            } else if (item.getItemId() == 2) {
                keyPreviewEnabled = !keyPreviewEnabled;
                keyboardView.setPreviewEnabled(keyPreviewEnabled);
                getSharedPreferences("keyboard", MODE_PRIVATE).edit().putBoolean("preview", keyPreviewEnabled).apply();
            } else {
                InputMethodManager manager = (InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
                if (manager != null) manager.showInputMethodPicker();
            }
            return true;
        });
        menu.show();
    }

    private void addReading(String label, boolean selected, int index) {
        float density = getResources().getDisplayMetrics().density;
        Button button = new Button(this);
        button.setText(label);
        button.setAllCaps(false);
        button.setSelected(selected);
        button.setTextColor(getColor(selected ? R.color.keyboard_accent : R.color.keyboard_text));
        button.setTextSize(TypedValue.COMPLEX_UNIT_SP, 18);
        button.setMinWidth((int) (48 * density));
        button.setPadding((int) (12 * density), 0, (int) (12 * density), 0);
        button.setGravity(Gravity.CENTER);
        button.setBackgroundResource(R.drawable.reading_chip);
        button.setOnClickListener(v -> { feedback(); chooseReading(index); });
        readingsView.addView(button, new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.MATCH_PARENT));
    }

    private void moveCursor(int code) {
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return;
        if (active) apply(ic, Native.nativeKey(ime, Native.OTHER, 0, before(ic)));
        sendDownUpKeyEvents(code);
        refresh();
    }

    private void closeClipboard() {
        if (clipboardPanel == null) return;
        if (clipboardManager != null) clipboardManager.removePrimaryClipChangedListener(clipboardChanged);
        clipboardPanel.setVisibility(View.GONE);
        clipboardPreview.setText("");
        keyboardView.setVisibility(View.VISIBLE);
        toolsOpen = false;
        cursorTools = false;
    }

    private ClipData clipboardText() {
        return clipboardManager == null ? null : clipboardManager.getPrimaryClip();
    }

    private void openClipboard() {
        clipboardPanel.getLayoutParams().height = Math.max(keyboardView.getHeight(),
                (int) (236 * getResources().getDisplayMetrics().density));
        keyboardView.setVisibility(View.INVISIBLE); // keep the panel and keyboard at the same height
        clipboardPanel.setVisibility(View.VISIBLE);
        showReadings();
        if (clipboardManager != null) {
            clipboardManager.removePrimaryClipChangedListener(clipboardChanged);
            clipboardManager.addPrimaryClipChangedListener(clipboardChanged);
        }
        try {
            ClipData clip = clipboardText();
            CharSequence text = clip == null || clip.getItemCount() == 0 ? null : clip.getItemAt(0).getText();
            clipboardPaste.setEnabled(text != null && text.length() > 0);
            if (!clipboardPaste.isEnabled()) {
                clipboardPreview.setText(R.string.clipboard_empty);
                return;
            }
            boolean sensitive = !active || (clip.getDescription().getExtras() != null
                    && clip.getDescription().getExtras().getBoolean(ClipDescription.EXTRA_IS_SENSITIVE));
            if (sensitive) {
                clipboardPreview.setText(R.string.clipboard_private);
            } else {
                String preview = text.toString();
                if (preview.length() > 500) {
                    int end = Character.isHighSurrogate(preview.charAt(499)) ? 499 : 500;
                    preview = preview.substring(0, end) + "…";
                }
                clipboardPreview.setText(preview);
            }
        } catch (SecurityException denied) {
            clipboardPreview.setText(R.string.clipboard_unavailable);
            clipboardPaste.setEnabled(false);
        }
    }

    private void pasteClipboard() {
        InputConnection ic = getCurrentInputConnection();
        if (ic == null) return;
        try {
            ClipData clip = clipboardText(); // paste the current clipboard, not a stale preview
            CharSequence text = clip == null || clip.getItemCount() == 0 ? null : clip.getItemAt(0).getText();
            if (text == null || text.length() == 0) { openClipboard(); return; }
            feedback();
            if (active) apply(ic, Native.nativeKey(ime, Native.OTHER, 0, before(ic)));
            if (!ic.commitText(text.toString(), 1)) {
                clipboardPreview.setText(R.string.paste_unavailable);
                return;
            }
            closeClipboard();
            refresh();
        } catch (SecurityException denied) {
            clipboardPreview.setText(R.string.clipboard_unavailable);
            clipboardPaste.setEnabled(false);
        }
    }

    private void chooseReading(int index) {
        InputConnection ic = getCurrentInputConnection();
        if (ic == null || !active) return;
        apply(ic, Native.nativeChooseReading(ime, index, before(ic)));
        refresh();
    }

    // MARK: - Shared

    private static String before(InputConnection ic) {
        CharSequence text = ic.getTextBeforeCursor(BEFORE_CARET, 0);
        return text == null ? "" : text.toString();
    }

    /**
     * One key through Rust. True if it was handled here; false (hardware keys only) when the app
     * is to handle it itself.
     */
    private boolean send(InputConnection ic, int kind, int ch, int keyCode, boolean soft) {
        String reply = Native.nativeKey(ime, kind, ch, before(ic));
        if (apply(ic, reply)) return true;
        // A word just went in and the app is to get the key: the app would act on it before our
        // edit arrives (and put the caret before it), so the key follows the word on the same
        // connection instead.
        boolean wordWentIn = reply.indexOf('\u0001') >= 0;
        if (!soft && !wordWentIn) return false;
        passThrough(ic, kind, ch, keyCode);
        return true;
    }

    /** What the key does when the input method has no use for it. */
    private void passThrough(InputConnection ic, int kind, int ch, int keyCode) {
        switch (kind) {
            case Native.LETTER:
            case Native.PRINTABLE:
                ic.commitText(new String(Character.toChars(ch)), 1);
                break;
            case Native.ENTER:
                enter(ic);
                break;
            default:
                if (keyCode != 0) sendDownUpKeyEvents(keyCode);
                break;
        }
    }

    private void enter(InputConnection ic) {
        EditorInfo info = getCurrentInputEditorInfo();
        int action = info == null ? EditorInfo.IME_ACTION_NONE : info.imeOptions & EditorInfo.IME_MASK_ACTION;
        boolean plain = info == null
                || (info.imeOptions & EditorInfo.IME_FLAG_NO_ENTER_ACTION) != 0
                || action == EditorInfo.IME_ACTION_NONE
                || action == EditorInfo.IME_ACTION_UNSPECIFIED;
        if (plain) {
            sendDownUpKeyEvents(KeyEvent.KEYCODE_ENTER);
        } else {
            ic.performEditorAction(action);
        }
    }

    /** Applies Rust's edit commands; true if the key was consumed. */
    private boolean apply(InputConnection ic, String reply) {
        String[] parts = reply.split("\u0001", -1);
        if (parts.length > 1) {
            ic.beginBatchEdit();
            for (int i = 1; i < parts.length; i++) {
                String rest = parts[i].substring(1);
                switch (parts[i].charAt(0)) {
                    case 'M':
                        composing = !rest.isEmpty();
                        ic.setComposingText(rest, 1);
                        break;
                    case 'I':
                        composing = false;
                        ic.commitText(rest, 1);
                        break;
                    case 'B':
                        ic.deleteSurroundingText(Integer.parseInt(rest), 0);
                        break;
                    default:
                        break;
                }
            }
            ic.endBatchEdit();
        }
        return parts[0].equals("1");
    }
}
