package com.eadxmz.ferrodeck;

import android.app.ActivityManager;
import android.app.NativeActivity;
import android.app.NotificationManager;
import android.bluetooth.BluetoothAdapter;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.drawable.Drawable;
import android.net.ConnectivityManager;
import android.net.NetworkInfo;
import android.net.wifi.WifiInfo;
import android.net.wifi.WifiManager;
import android.os.Bundle;
import android.provider.Settings;
import android.view.KeyEvent;
import android.view.View;
import android.view.WindowManager;

import java.net.InetSocketAddress;
import java.net.Socket;
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
    public static native void nativeUpdateSystemStatus(
            String wifiLabel,
            boolean btOn,
            boolean dndOn,
            int ramUsedMb,
            int ramTotalMb,
            boolean adbPortOpen,
            boolean a11yEnabled,
            boolean defaultHome,
            boolean writeSecure,
            String statusMsg);

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        resetTvFocusLock();
        refreshInstalledApps();
        refreshSystemSnapshot();
        handleSpecialIntent(getIntent());
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        resetTvFocusLock();
        nativeOnHomePressed();
        refreshSystemSnapshot();
        handleSpecialIntent(intent);
    }

    @Override
    protected void onResume() {
        super.onResume();
        resetTvFocusLock();
        refreshInstalledApps();
        refreshSystemSnapshot();
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

    public void refreshSystemSnapshot() {
        refreshSystemSnapshotWithMsg("");
    }

    private void refreshSystemSnapshotWithMsg(final String msg) {
        new Thread(() -> {
            String wifiLabel = "Connected";
            try {
                ConnectivityManager cm = (ConnectivityManager) getSystemService(Context.CONNECTIVITY_SERVICE);
                NetworkInfo ni = cm != null ? cm.getActiveNetworkInfo() : null;
                if (ni == null || !ni.isConnected()) {
                    wifiLabel = "Disconnected";
                } else if (ni.getType() == ConnectivityManager.TYPE_ETHERNET) {
                    wifiLabel = "Ethernet";
                } else {
                    WifiManager wm = (WifiManager) getApplicationContext().getSystemService(Context.WIFI_SERVICE);
                    WifiInfo wi = wm != null ? wm.getConnectionInfo() : null;
                    if (wi != null && wi.getSSID() != null) {
                        String s = wi.getSSID().replace("\"", "");
                        if (!s.isEmpty() && !"<unknown ssid>".equalsIgnoreCase(s)) {
                            wifiLabel = s;
                        }
                    }
                }
            } catch (Throwable ignored) { }

            boolean btOn = false;
            try {
                BluetoothAdapter ba = BluetoothAdapter.getDefaultAdapter();
                btOn = ba != null && ba.isEnabled();
            } catch (Throwable ignored) { }

            boolean dndOn = false;
            try {
                NotificationManager nm = (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
                if (nm != null) {
                    dndOn = nm.getCurrentInterruptionFilter() != NotificationManager.INTERRUPTION_FILTER_ALL;
                }
            } catch (Throwable ignored) { }

            int usedMb = 1024;
            int totalMb = 2048;
            try {
                ActivityManager am = (ActivityManager) getSystemService(Context.ACTIVITY_SERVICE);
                ActivityManager.MemoryInfo mi = new ActivityManager.MemoryInfo();
                if (am != null) {
                    am.getMemoryInfo(mi);
                    totalMb = (int) (mi.totalMem / (1024L * 1024L));
                    usedMb = (int) ((mi.totalMem - mi.availMem) / (1024L * 1024L));
                }
            } catch (Throwable ignored) { }

            boolean adbPortOpen = isLocalAdbPortOpen();
            boolean a11yEnabled = isRemoteServiceEnabled();
            boolean defaultHome = isDefaultLauncher();
            boolean writeSecure = checkSelfPermission("android.permission.WRITE_SECURE_SETTINGS")
                    == PackageManager.PERMISSION_GRANTED;

            nativeUpdateSystemStatus(
                    wifiLabel, btOn, dndOn, usedMb, totalMb,
                    adbPortOpen, a11yEnabled, defaultHome, writeSecure, msg);
        }, "ferrodeck-sys").start();
    }

    private static boolean isLocalAdbPortOpen() {
        try (Socket s = new Socket()) {
            s.connect(new InetSocketAddress("127.0.0.1", 5555), 180);
            return true;
        } catch (Throwable ignored) {
            return false;
        }
    }

    private boolean isRemoteServiceEnabled() {
        try {
            String enabled = Settings.Secure.getString(
                    getContentResolver(), Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES);
            return enabled != null && enabled.contains(getPackageName());
        } catch (Throwable ignored) {
            return false;
        }
    }

    private boolean isDefaultLauncher() {
        try {
            Intent i = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_HOME);
            ResolveInfo ri = getPackageManager().resolveActivity(i, PackageManager.MATCH_DEFAULT_ONLY);
            return ri != null && ri.activityInfo != null
                    && getPackageName().equals(ri.activityInfo.packageName);
        } catch (Throwable ignored) {
            return false;
        }
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

    public void killBackgroundPackageFromRust(String pkg) {
        if (pkg == null || pkg.isEmpty()) return;
        try {
            ActivityManager am = (ActivityManager) getSystemService(Context.ACTIVITY_SERVICE);
            if (am != null) {
                am.killBackgroundProcesses(pkg);
            }
        } catch (Throwable ignored) { }
        refreshSystemSnapshot();
    }

    public void cleanMemoryFromRust() {
        try {
            ActivityManager am = (ActivityManager) getSystemService(Context.ACTIVITY_SERVICE);
            PackageManager pm = getPackageManager();
            if (am != null) {
                Intent main = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER);
                for (ResolveInfo ri : pm.queryIntentActivities(main, 0)) {
                    if (ri.activityInfo != null && !getPackageName().equals(ri.activityInfo.packageName)) {
                        am.killBackgroundProcesses(ri.activityInfo.packageName);
                    }
                }
            }
        } catch (Throwable ignored) { }
        refreshSystemSnapshot();
    }

    public void triggerVoiceAssistFromRust() {
        try {
            Intent voice = new Intent("android.speech.action.WEB_SEARCH");
            voice.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(voice);
        } catch (Throwable ignored) { }
    }

    public void openWifiSettingsFromRust() {
        openSafeIntent(Settings.ACTION_WIFI_SETTINGS, Settings.ACTION_SETTINGS);
    }

    public void openBluetoothSettingsFromRust() {
        openSafeIntent(Settings.ACTION_BLUETOOTH_SETTINGS, Settings.ACTION_SETTINGS);
    }

    public void openSoundSettingsFromRust() {
        openSafeIntent(Settings.ACTION_SOUND_SETTINGS, Settings.ACTION_SETTINGS);
    }

    public void openDisplaySettingsFromRust() {
        openSafeIntent(Settings.ACTION_DISPLAY_SETTINGS, Settings.ACTION_SETTINGS);
    }

    public void openInputsFromRust() {
        openSafeIntent("android.settings.CHANNEL_SETUP", Settings.ACTION_INPUT_METHOD_SETTINGS);
    }

    public void openAccessibilitySettingsFromRust() {
        openSafeIntent(Settings.ACTION_ACCESSIBILITY_SETTINGS, Settings.ACTION_SETTINGS);
    }

    public void openAndroidSettingsFromRust() {
        openSafeIntent(Settings.ACTION_SETTINGS, Settings.ACTION_SETTINGS);
    }

    public void openHomePickerFromRust() {
        openSafeIntent(Settings.ACTION_HOME_SETTINGS, Settings.ACTION_SETTINGS);
    }

    public void lockScreenFromRust() {
        openSafeIntent(Settings.ACTION_DREAM_SETTINGS, Settings.ACTION_DISPLAY_SETTINGS);
    }

    public void setKeepScreenOnFromRust(final boolean on) {
        runOnUiThread(() -> {
            if (on) {
                getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
            } else {
                getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
            }
        });
    }

    public void toggleAdbPortFromRust() {
        try {
            boolean cur = Settings.Global.getInt(getContentResolver(), Settings.Global.ADB_ENABLED, 0) == 1;
            Settings.Global.putInt(getContentResolver(), Settings.Global.ADB_ENABLED, cur ? 0 : 1);
            refreshSystemSnapshotWithMsg(cur ? "ADB debugging turned off." : "ADB debugging enabled.");
        } catch (SecurityException se) {
            openSafeIntent(Settings.ACTION_APPLICATION_DEVELOPMENT_SETTINGS, Settings.ACTION_SETTINGS);
            refreshSystemSnapshotWithMsg("Enable USB / Network Debugging in Developer Options.");
        }
    }

    public void runLocalAdbPairFromRust(final boolean secureMode) {
        new Thread(() -> {
            if (!isLocalAdbPortOpen()) {
                openSafeIntent(Settings.ACTION_APPLICATION_DEVELOPMENT_SETTINGS, Settings.ACTION_SETTINGS);
                refreshSystemSnapshotWithMsg("Turn on Network Debugging (port 5555), then press Pair again.");
                return;
            }
            refreshSystemSnapshotWithMsg("ADB port 5555 reachable — ready for permission grant.");
        }, "ferrodeck-adb").start();
    }

    private void openSafeIntent(String primary, String fallback) {
        try {
            Intent i = new Intent(primary).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(i);
        } catch (Throwable ignored) {
            try {
                Intent fb = new Intent(fallback).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                startActivity(fb);
            } catch (Throwable ignored2) { }
        }
    }
}
