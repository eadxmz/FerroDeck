package com.eadxmz.ferrodeck;

import android.app.NativeActivity;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.drawable.Drawable;
import android.os.Bundle;
import android.provider.Settings;
import android.view.KeyEvent;
import android.view.View;

import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public class MainActivity extends NativeActivity {
    static {
        System.loadLibrary("ferrodeck");
    }

    public static native void nativeOnHomePressed();
    public static native void nativeOnRemoteKey(int keyCode, int scanCode);
    public static native void nativeSetApps(String[] packages, String[] labels, byte[][] icons, int iconSize);

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

    private static byte[] drawableToRgba(Drawable d, int size) {
        if (d == null) return null;
        try {
            Bitmap bmp = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888);
            Canvas c = new Canvas(bmp);
            d.setBounds(0, 0, size, size);
            d.draw(c);
            ByteBuffer buf = ByteBuffer.allocate(size * size * 4);
            bmp.copyPixelsToBuffer(buf);
            bmp.recycle();
            return buf.array();
        } catch (Throwable t) {
            return null;
        }
    }

    public void refreshInstalledApps() {
        PackageManager pm = getPackageManager();
        Map<String, ResolveInfo> found = new LinkedHashMap<>();

        Intent leanback = new Intent(Intent.ACTION_MAIN);
        leanback.addCategory(Intent.CATEGORY_LEANBACK_LAUNCHER);
        for (ResolveInfo ri : pm.queryIntentActivities(leanback, 0)) {
            if (ri.activityInfo == null) continue;
            String pkg = ri.activityInfo.packageName;
            if (!getPackageName().equals(pkg)) {
                found.put(pkg, ri);
            }
        }

        Intent standard = new Intent(Intent.ACTION_MAIN);
        standard.addCategory(Intent.CATEGORY_LAUNCHER);
        for (ResolveInfo ri : pm.queryIntentActivities(standard, 0)) {
            if (ri.activityInfo == null) continue;
            String pkg = ri.activityInfo.packageName;
            if (!getPackageName().equals(pkg) && !found.containsKey(pkg)) {
                found.put(pkg, ri);
            }
        }

        List<Map.Entry<String, ResolveInfo>> entries = new ArrayList<>(found.entrySet());
        Collections.sort(entries, (a, b) -> a.getValue().loadLabel(pm).toString()
                .compareToIgnoreCase(b.getValue().loadLabel(pm).toString()));

        final int iconSize = 96;
        String[] pkgs = new String[entries.size()];
        String[] lbls = new String[entries.size()];
        byte[][] icons = new byte[entries.size()][];
        for (int i = 0; i < entries.size(); i++) {
            String pkg = entries.get(i).getKey();
            ResolveInfo ri = entries.get(i).getValue();
            pkgs[i] = pkg;
            lbls[i] = ri.loadLabel(pm).toString();
            Drawable iconDrw = null;
            try {
                iconDrw = pm.getApplicationIcon(pkg);
            } catch (Throwable ignored) {
                iconDrw = ri.loadIcon(pm);
            }
            icons[i] = drawableToRgba(iconDrw, iconSize);
        }
        nativeSetApps(pkgs, lbls, icons, iconSize);
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

    public void openSystemActionFromRust(String action) {
        try {
            Intent i = new Intent(action != null && !action.isEmpty() ? action : Settings.ACTION_SETTINGS);
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(i);
        } catch (Throwable ignored) {
            try {
                Intent fallback = new Intent(Settings.ACTION_SETTINGS);
                fallback.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                startActivity(fallback);
            } catch (Throwable ignored2) { }
        }
    }
}
