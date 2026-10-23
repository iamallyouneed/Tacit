package io.github.iamallyouneed.tacit;

import android.content.Context;
import android.inputmethodservice.Keyboard;

/** Keep the framework's key layout while changing row height and touch geometry together. */
@SuppressWarnings("deprecation")
public final class SizedKeyboard extends Keyboard {
    private final int height;
    private final int[] indices;
    private static final int[] NONE = new int[0];

    public SizedKeyboard(Context context, int layout, int heightDp) {
        super(context, layout);
        float scale = heightDp * context.getResources().getDisplayMetrics().density / getKeys().get(0).height;
        height = Math.round(super.getHeight() * scale);
        indices = new int[getKeys().size()];
        for (int i = 0; i < indices.length; i++) {
            indices[i] = i;
            Key key = getKeys().get(i);
            key.y = Math.round(key.y * scale);
            key.height = Math.round(key.height * scale);
        }
    }

    @Override public int getHeight() { return height; }

    @Override public int[] getNearestKeys(int x, int y) {
        // shortcut: scan <50 keys to avoid the old-height grid; index again if layouts grow substantially.
        return x < 0 || x >= getMinWidth() || y < 0 || y >= height ? NONE : indices;
    }
}
