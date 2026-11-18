package com.eadxmz.ferrodeck;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.AccessibilityServiceInfo;
import android.content.Context;
import android.content.SharedPreferences;
import android.os.Bundle;
import android.view.KeyEvent;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;

/**
 * System-wide hardware remote button filter & active window tracker.
 * - Tracks `foregroundPackage` so Voice Assistant knows when you are inside YouTube,
 *   TizenTube, Koji TV Browser, etc. and triggers in-app search automatically.
 * - Captures custom Voice Assistant keybind (`keyCode` + `scanCode`) across all apps.
 */
public class RemoteFilterService extends AccessibilityService {
    public static volatile RemoteFilterService instance;
    public static volatile String foregroundPackage = "";

    @Override
    protected void onServiceConnected() {
        super.onServiceConnected();
        instance = this;
        AccessibilityServiceInfo info = getServiceInfo();
        if (info != null) {
            info.flags |= AccessibilityServiceInfo.FLAG_REQUEST_FILTER_KEY_EVENTS
                    | AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS;
            setServiceInfo(info);
        }
    }

    @Override
    public void onDestroy() {
        instance = null;
        super.onDestroy();
    }

    @Override
    protected boolean onKeyEvent(KeyEvent event) {
        if (event == null) return false;
        int kc = event.getKeyCode();
        int sc = event.getScanCode();

        if (kc == KeyEvent.KEYCODE_DPAD_UP || kc == KeyEvent.KEYCODE_DPAD_DOWN
                || kc == KeyEvent.KEYCODE_DPAD_LEFT || kc == KeyEvent.KEYCODE_DPAD_RIGHT) {
            return false;
        }

        if (event.getAction() == KeyEvent.ACTION_DOWN && event.getRepeatCount() == 0) {
            // Check if user configured a custom Voice Assistant keybind
            SharedPreferences sp = getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE);
            int customVoiceKc = sp.getInt("voice_key_code", -1);
            int customVoiceSc = sp.getInt("voice_scan_code", -1);
            boolean matchesCustomVoice = (customVoiceKc > 0 && kc == customVoiceKc)
                    || (customVoiceSc > 0 && sc == customVoiceSc);

            boolean isDefaultVoiceKey = (kc == KeyEvent.KEYCODE_UNKNOWN && sc > 0)
                    || kc == KeyEvent.KEYCODE_VOICE_ASSIST
                    || kc == KeyEvent.KEYCODE_ASSIST
                    || kc == KeyEvent.KEYCODE_PROG_YELLOW;

            if (matchesCustomVoice || isDefaultVoiceKey) {
                // If user is currently capturing a keybind inside FerroDeck settings, forward raw key
                if (MainActivity.isKeyCaptureActive) {
                    try {
                        MainActivity.nativeOnRemoteKey(kc, sc);
                    } catch (Throwable ignored) { }
                    return true;
                }
                VoiceAssistant.startListening(this);
                return true;
            }
        }
        return false;
    }

    @Override
    public void onAccessibilityEvent(AccessibilityEvent event) {
        if (event == null) return;
        if (event.getEventType() == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED
                && event.getPackageName() != null) {
            String pkg = event.getPackageName().toString();
            if (!pkg.isEmpty() && !getPackageName().equals(pkg) && !"com.android.systemui".equals(pkg)) {
                foregroundPackage = pkg;
            }
        }
    }

    /** Fills an active EditText search box in the foreground app if available. */
    public boolean tryFillSearchFieldInForeground(String query) {
        try {
            AccessibilityNodeInfo root = getRootInActiveWindow();
            if (root == null) return false;
            AccessibilityNodeInfo edit = findEditableNode(root);
            if (edit != null) {
                Bundle args = new Bundle();
                args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, query);
                edit.performAction(AccessibilityNodeInfo.ACTION_FOCUS);
                boolean ok = edit.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args);
                edit.performAction(AccessibilityNodeInfo.ACTION_CLICK);
                return ok;
            }
        } catch (Throwable ignored) { }
        return false;
    }

    private static AccessibilityNodeInfo findEditableNode(AccessibilityNodeInfo node) {
        if (node == null) return null;
        if (node.isEditable() && node.isVisibleToUser()) return node;
        for (int i = 0; i < node.getChildCount(); i++) {
            AccessibilityNodeInfo res = findEditableNode(node.getChild(i));
            if (res != null) return res;
        }
        return null;
    }

    @Override
    public void onInterrupt() { }
}
