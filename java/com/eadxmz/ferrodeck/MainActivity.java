package com.eadxmz.ferrodeck;

import android.app.NativeActivity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.os.Bundle;
import android.provider.Settings;
import android.view.KeyEvent;
import android.view.View;

import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Minimal NativeActivity host for FerroDeck.
 * Fixes the Koji Browser Home-button D-Pad focus lock by clearing Android Touch Mode
 * and resetting native Rust focus state immediately on onNewIntent / onWindowFocusChanged.
 */
public class MainActivity extends NativeActivity {
    static {
        System.loadLibrary("ferrodeck");
    }

    public static native void nativeOnHomePressed();
    public static native void nativeOnRemoteKey(int keyCode, int scanCode);
    public static native void nativeSetApps(String[] packages, String[] labels);

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        resetTvFocusLock();
        refreshInstalledApps();
        handleSpecialIntent(getIntent());
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        resetTvFocusLock();
        nativeOnHomePressed();
        handleSpecialIntent(intent);
    }

    @Override
    protected void onResume() {
        super.onResume();
        resetTvFocusLock();
        refreshInstalledApps();
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus) {
            resetTvFocusLock();
        }
    }

    private void resetTvFocusLock() {
        try {
            View decor = getWindow().getDecorView();
            decor.setFocusable(true);
            decor.setFocusableInTouchMode(true);
            decor.requestFocusFromTouch();
            decor.requestFocus();
        } catch (Throwable ignored) { }
    }

    private void handleSpecialIntent(Intent intent) {
        if (intent == null) return;
        String action = intent.getAction();
        if (Intent.ACTION_ASSIST.equals(action)
                || "android.speech.action.WEB_SEARCH".equals(action)
                || "android.intent.action.VOICE_COMMAND".equals(action)) {
            nativeOnRemoteKey(KeyEvent.KEYCODE_VOICE_ASSIST, 0);
        }
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        if (event != null && event.getAction() == KeyEvent.ACTION_DOWN) {
            int kc = event.getKeyCode();
            int sc = event.getScanCode();
            if ((kc == KeyEvent.KEYCODE_UNKNOWN && sc > 0)
                    || kc == KeyEvent.KEYCODE_VOICE_ASSIST
                    || kc == KeyEvent.KEYCODE_ASSIST
                    || kc == KeyEvent.KEYCODE_PROG_YELLOW) {
                nativeOnRemoteKey(kc, sc);
                return true;
            }
        }
        return super.dispatchKeyEvent(event);
    }

    public void refreshInstalledApps() {
        PackageManager pm = getPackageManager();
        Map<String, String> found = new LinkedHashMap<>();

        Intent leanback = new Intent(Intent.ACTION_MAIN);
        leanback.addCategory(Intent.CATEGORY_LEANBACK_LAUNCHER);
        for (ResolveInfo ri : pm.queryIntentActivities(leanback, 0)) {
            if (ri.activityInfo == null) continue;
            String pkg = ri.activityInfo.packageName;
            if (!getPackageName().equals(pkg)) {
                found.put(pkg, ri.loadLabel(pm).toString());
            }
        }

        Intent standard = new Intent(Intent.ACTION_MAIN);
        standard.addCategory(Intent.CATEGORY_LAUNCHER);
        for (ResolveInfo ri : pm.queryIntentActivities(standard, 0)) {
            if (ri.activityInfo == null) continue;
            String pkg = ri.activityInfo.packageName;
            if (!getPackageName().equals(pkg) && !found.containsKey(pkg)) {
                found.put(pkg, ri.loadLabel(pm).toString());
            }
        }

        List<Map.Entry<String, String>> entries = new ArrayList<>(found.entrySet());
        Collections.sort(entries, (a, b) -> a.getValue().compareToIgnoreCase(b.getValue()));

        String[] pkgs = new String[entries.size()];
        String[] lbls = new String[entries.size()];
        for (int i = 0; i < entries.size(); i++) {
            pkgs[i] = entries.get(i).getKey();
            lbls[i] = entries.get(i).getValue();
        }
        nativeSetApps(pkgs, lbls);
    }

    public void launchPackageFromRust(String pkg) {
        if (pkg == null || pkg.isEmpty()) return;
        try {
            PackageManager pm = getPackageManager();
            Intent i = pm.getLeanbackLaunchIntentForPackage(pkg);
            if (i == null) i = pm.getLaunchIntentForPackage(pkg);
            if (i != null) {
                i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                startActivity(i);
            }
        } catch (Throwable ignored) { }
    }

    public void triggerVoiceAssistFromRust() {
        try {
            Intent voice = new Intent("android.speech.action.WEB_SEARCH");
            voice.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(voice);
        } catch (Throwable ignored) { }
    }

    public void openAccessibilitySettingsFromRust() {
        try {
            Intent i = new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS);
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(i);
        } catch (Throwable ignored) { }
    }
}
