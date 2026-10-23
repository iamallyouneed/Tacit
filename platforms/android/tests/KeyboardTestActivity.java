package io.github.iamallyouneed.tacit;

import android.app.Activity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.os.Bundle;
import android.text.InputType;
import android.view.inputmethod.EditorInfo;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;

/** Isolated editor for emulator UI checks; never packaged in the shipping APK. */
public final class KeyboardTestActivity extends Activity {
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        float density = getResources().getDisplayMetrics().density;
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        int padding = (int) (24 * density);
        root.setPadding(padding, padding, padding, padding);
        TextView title = new TextView(this);
        title.setText("메모");
        title.setTextSize(24);
        root.addView(title);
        EditText editor = new EditText(this);
        editor.setHint("생각을 자연스럽게 적어보세요.");
        editor.setTextSize(18);
        editor.setGravity(android.view.Gravity.TOP | android.view.Gravity.START);
        editor.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE);
        editor.setImeOptions(EditorInfo.IME_FLAG_NO_EXTRACT_UI | EditorInfo.IME_FLAG_NO_ENTER_ACTION);
        root.addView(editor, new LinearLayout.LayoutParams(-1, (int) (180 * density)));
        EditText search = new EditText(this);
        search.setSingleLine(true);
        search.setHint("검색");
        search.setImeOptions(EditorInfo.IME_ACTION_SEARCH | EditorInfo.IME_FLAG_NO_EXTRACT_UI);
        root.addView(search, new LinearLayout.LayoutParams(-1, -2));
        setContentView(root);
        editor.requestFocus();
        getWindow().setSoftInputMode(android.view.WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_VISIBLE
                | android.view.WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE);
        if (getIntent().getBooleanExtra("clipboard", false)) {
            ClipboardManager clipboard = (ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
            clipboard.setPrimaryClip(ClipData.newPlainText("UX test", "안녕하세요.\n내일 오후 2시에 만나요."));
        }
    }
}
