package com.eadxmz.ferrodeck;

import android.app.ActivityManager;
import android.app.AlertDialog;
import android.app.NativeActivity;
import android.app.NotificationManager;
import android.bluetooth.BluetoothAdapter;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.drawable.Drawable;
import android.net.ConnectivityManager;
import android.net.NetworkInfo;
import android.net.Uri;
import android.net.wifi.WifiInfo;
import android.net.wifi.WifiManager;
import android.os.Bundle;
import android.provider.Settings;
import android.text.InputType;
import android.view.KeyEvent;
import android.view.View;
import android.view.WindowManager;
import android.view.inputmethod.EditorInfo;
import android.widget.EditText;

import java.net.InetSocketAddress;
import java.net.Socket;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;

public class MainActivity extends NativeActivity {
    static {
        System.loadLibrary("ferrodeck");
    }

    public static volatile MainActivity activeInstance;
    public static volatile String lastLaunchedPackage = "";
    public static volatile boolean isKeyCaptureActive = false;

    public static native void nativeOnHomePressed();
    public static native void nativeOnRemoteKey(int keyCode, int scanCode);
    public static native void nativeSetApps(String[] packages, String[] labels, byte[][] icons, boolean[] isBuiltin, int iconSize);
    public static native void nativeSetHiddenApps(String[] packages, String[] labels);
    public static native void nativeOpenSearchWithQuery(String query);
    public static native void nativeSetVoiceHud(boolean active, String title, String subtitle);
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
        activeInstance = this;
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
        activeInstance = this;
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

    private long lastVoiceTriggerMs = 0L;

    private void handleSpecialIntent(Intent intent) {
        if (intent == null) return;
        if (intent.getBooleanExtra("request_mic_perm", false)) {
            requestPermissions(new String[]{android.Manifest.permission.RECORD_AUDIO}, 4401);
            return;
        }
        String action = intent.getAction();
        if (Intent.ACTION_ASSIST.equals(action)
                || "android.speech.action.WEB_SEARCH".equals(action)
                || "android.intent.action.VOICE_COMMAND".equals(action)) {
            long now = android.os.SystemClock.uptimeMillis();
            if (now - lastVoiceTriggerMs < 1500L) {
                return;
            }
            lastVoiceTriggerMs = now;
            VoiceAssistant.startListening(this);
        }
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        if (event != null && event.getAction() == KeyEvent.ACTION_DOWN) {
            int kc = event.getKeyCode();
            int sc = event.getScanCode();

            SharedPreferences sp = getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE);
            int customVoiceKc = sp.getInt("voice_key_code", -1);
            int customVoiceSc = sp.getInt("voice_scan_code", -1);
            boolean matchesCustomVoice = (customVoiceKc > 0 && kc == customVoiceKc)
                    || (customVoiceSc > 0 && sc == customVoiceSc);

            if (!isKeyCaptureActive && (matchesCustomVoice
                    || (kc == KeyEvent.KEYCODE_UNKNOWN && sc > 0)
                    || kc == KeyEvent.KEYCODE_VOICE_ASSIST
                    || kc == KeyEvent.KEYCODE_ASSIST
                    || kc == KeyEvent.KEYCODE_PROG_YELLOW)) {
                VoiceAssistant.startListening(this);
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

    private static boolean isAllowedBuiltinMediaApp(String pkg, boolean hasLeanback) {
        if (pkg == null) return false;
        String p = pkg.toLowerCase(java.util.Locale.ROOT);
        // Never allow system settings, launchers, OS utilities, or hardware daemons in the dock
        if (p.equals("android")
                || p.contains("settings")
                || p.contains("launcher")
                || p.contains("systemui")
                || p.contains("setup")
                || p.contains("provider")
                || p.contains("inputmethod")
                || p.contains("keyboard")
                || p.contains("permission")
                || p.contains("packageinstaller")
                || p.contains("webview")
                || p.contains("traceur")
                || p.contains("documentsui")
                || p.contains("dialer")
                || p.contains("contacts")
                || p.contains("messaging")
                || p.contains("deskclock")
                || p.contains("calculator")
                || p.contains("calendar")
                || p.contains("gallery")
                || p.contains("camera")
                || p.startsWith("com.android.cts")
                || p.startsWith("com.google.android.gms")
                || p.startsWith("com.google.android.gsf")
                || p.startsWith("com.google.android.tts")
                || p.startsWith("com.google.android.katniss")
                || p.startsWith("com.google.android.backdrop")
                || p.startsWith("com.amlogic.")
                || p.startsWith("com.droidlogic.")
                || p.startsWith("com.mediatek.")
                || p.startsWith("com.realtek.")
                || p.startsWith("com.rockchip.")
                || p.startsWith("com.qualcomm.")
                || p.startsWith("com.hisilicon.")) {
            return false;
        }
        // Allow genuine built-in entertainment, store & browser apps
        if (p.equals("com.android.vending")
                || p.equals("com.android.chrome")
                || p.startsWith("com.google.android.youtube")
                || p.startsWith("com.google.android.videos")
                || p.startsWith("com.google.android.music")
                || p.startsWith("com.google.android.apps.tv")
                || p.startsWith("com.netflix.")
                || p.startsWith("com.amazon.amazonvideo")
                || p.startsWith("com.amazon.avod")
                || p.startsWith("com.disney.")
                || p.equals("in.startv.hotstar")
                || p.startsWith("com.spotify.")
                || p.startsWith("com.apple.atve")
                || p.startsWith("com.hbo.")
                || p.startsWith("com.wbd.")
                || p.startsWith("tv.twitch.")
                || p.equals("org.videolan.vlc")
                || p.equals("org.xbmc.kodi")) {
            return true;
        }
        // Other vendor pre-installed apps only if they explicitly declare LEANBACK_LAUNCHER
        return hasLeanback;
    }

    public void refreshInstalledApps() {
        PackageManager pm = getPackageManager();
        SharedPreferences sp = getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE);
        Set<String> hiddenSet = new HashSet<>(sp.getStringSet("hidden_pkgs", Collections.emptySet()));
        String orderStr = sp.getString("dock_order", "");

        byte[] defaultRobotIcon = drawableToRgba(pm.getDefaultActivityIcon(), 48);

        Map<String, ResolveInfo> found = new LinkedHashMap<>();
        Set<String> leanbackPkgs = new HashSet<>();

        Intent leanback = new Intent(Intent.ACTION_MAIN);
        leanback.addCategory(Intent.CATEGORY_LEANBACK_LAUNCHER);
        for (ResolveInfo ri : pm.queryIntentActivities(leanback, 0)) {
            if (ri.activityInfo == null) continue;
            String pkg = ri.activityInfo.packageName;
            if (!getPackageName().equals(pkg)) {
                found.put(pkg, ri);
                leanbackPkgs.add(pkg);
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

        final Map<String, Integer> orderRank = new HashMap<>();
        if (!orderStr.isEmpty()) {
            String[] parts = orderStr.split(",");
            for (int i = 0; i < parts.length; i++) {
                orderRank.put(parts[i], i);
            }
        }

        final int iconSize = 96;
        List<Map.Entry<String, ResolveInfo>> visibleEntries = new ArrayList<>();
        Map<String, byte[]> iconMap = new HashMap<>();
        Map<String, Boolean> builtinMap = new HashMap<>();
        List<String> hiddenPkgs = new ArrayList<>();
        List<String> hiddenLabels = new ArrayList<>();

        for (Map.Entry<String, ResolveInfo> e : found.entrySet()) {
            String pkg = e.getKey();
            ResolveInfo ri = e.getValue();
            ApplicationInfo ai = ri.activityInfo != null ? ri.activityInfo.applicationInfo : null;

            boolean isSystemOrUpdated = ai != null
                    && ((ai.flags & ApplicationInfo.FLAG_SYSTEM) != 0
                        || (ai.flags & ApplicationInfo.FLAG_UPDATED_SYSTEM_APP) != 0);

            // 1. Exclude any app that has no icon resource declared
            int declaredIcon = (ri.icon != 0) ? ri.icon : (ai != null ? ai.icon : 0);
            if (declaredIcon == 0 && ri.activityInfo != null) {
                declaredIcon = ri.activityInfo.icon;
            }
            if (declaredIcon == 0) {
                continue;
            }

            // 2. Render icon and exclude if missing or identical to Android's default green robot icon
            Drawable iconDrw = null;
            try {
                iconDrw = pm.getApplicationIcon(pkg);
            } catch (Throwable ignored) {
                iconDrw = ri.loadIcon(pm);
            }
            if (iconDrw == null) {
                continue;
            }
            if (defaultRobotIcon != null) {
                byte[] probe48 = drawableToRgba(iconDrw, 48);
                if (probe48 == null || Arrays.equals(probe48, defaultRobotIcon)) {
                    continue;
                }
            }

            // 3. Exclude system apps from appearing in the dock (only allow known built-in user apps)
            if (isSystemOrUpdated && !isAllowedBuiltinMediaApp(pkg, leanbackPkgs.contains(pkg))) {
                continue;
            }

            byte[] rgba96 = drawableToRgba(iconDrw, iconSize);
            if (rgba96 == null) {
                continue;
            }

            String customName = sp.getString("rename_" + pkg, null);
            String effectiveLabel = (customName != null && !customName.trim().isEmpty())
                    ? customName.trim()
                    : ri.loadLabel(pm).toString();

            if (hiddenSet.contains(pkg)) {
                hiddenPkgs.add(pkg);
                hiddenLabels.add(effectiveLabel);
            } else {
                visibleEntries.add(e);
                iconMap.put(pkg, rgba96);
                builtinMap.put(pkg, isSystemOrUpdated);
            }
        }

        Collections.sort(visibleEntries, (a, b) -> {
            Integer ra = orderRank.get(a.getKey());
            Integer rb = orderRank.get(b.getKey());
            if (ra != null && rb != null) return Integer.compare(ra, rb);
            if (ra != null) return -1;
            if (rb != null) return 1;
            String la = sp.getString("rename_" + a.getKey(), a.getValue().loadLabel(pm).toString());
            String lb = sp.getString("rename_" + b.getKey(), b.getValue().loadLabel(pm).toString());
            return la.compareToIgnoreCase(lb);
        });

        String[] pkgs = new String[visibleEntries.size()];
        String[] lbls = new String[visibleEntries.size()];
        byte[][] icons = new byte[visibleEntries.size()][];
        boolean[] builtins = new boolean[visibleEntries.size()];
        for (int i = 0; i < visibleEntries.size(); i++) {
            String pkg = visibleEntries.get(i).getKey();
            ResolveInfo ri = visibleEntries.get(i).getValue();
            pkgs[i] = pkg;
            String customName = sp.getString("rename_" + pkg, null);
            lbls[i] = (customName != null && !customName.trim().isEmpty())
                    ? customName.trim()
                    : ri.loadLabel(pm).toString();
            icons[i] = iconMap.get(pkg);
            builtins[i] = Boolean.TRUE.equals(builtinMap.get(pkg));
        }
        nativeSetApps(pkgs, lbls, icons, builtins, iconSize);
        nativeSetHiddenApps(
                hiddenPkgs.toArray(new String[0]),
                hiddenLabels.toArray(new String[0]));
    }

    // ------------------------------------------------------------------ App Long-Press Actions (JNI)

    public void promptRenameAppFromRust(final String pkg, final String currentLabel) {
        if (pkg == null || pkg.isEmpty()) return;
        runOnUiThread(() -> {
            try {
                final EditText input = new EditText(this);
                input.setSingleLine(true);
                input.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_WORDS);
                input.setImeOptions(EditorInfo.IME_ACTION_DONE);
                input.setText(currentLabel);
                input.setSelection(currentLabel != null ? currentLabel.length() : 0);

                new AlertDialog.Builder(this, android.R.style.Theme_DeviceDefault_Dialog_Alert)
                        .setTitle("Rename App")
                        .setMessage("Voice Assistant will also recognize this new name:")
                        .setView(input)
                        .setPositiveButton("Save", (d, w) -> {
                            String next = input.getText().toString().trim();
                            SharedPreferences sp = getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE);
                            if (next.isEmpty()) {
                                sp.edit().remove("rename_" + pkg).apply();
                            } else {
                                sp.edit().putString("rename_" + pkg, next).apply();
                            }
                            refreshInstalledApps();
                        })
                        .setNeutralButton("Reset Default", (d, w) -> {
                            getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE)
                                    .edit().remove("rename_" + pkg).apply();
                            refreshInstalledApps();
                        })
                        .setNegativeButton("Cancel", null)
                        .show();
            } catch (Throwable ignored) { }
        });
    }

    public void setAppHiddenFromRust(String pkg, boolean hidden) {
        if (pkg == null || pkg.isEmpty()) return;
        SharedPreferences sp = getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE);
        Set<String> set = new HashSet<>(sp.getStringSet("hidden_pkgs", Collections.emptySet()));
        if (hidden) set.add(pkg);
        else set.remove(pkg);
        sp.edit().putStringSet("hidden_pkgs", set).apply();
        refreshInstalledApps();
    }

    public void saveDockOrderFromRust(String csvPackages) {
        if (csvPackages == null) return;
        getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE)
                .edit().putString("dock_order", csvPackages).apply();
    }

    public void saveVoiceKeybindFromRust(int keyCode, int scanCode) {
        getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE)
                .edit()
                .putInt("voice_key_code", keyCode)
                .putInt("voice_scan_code", scanCode)
                .apply();
    }

    public void setKeyCaptureFlagFromRust(boolean active) {
        isKeyCaptureActive = active;
    }

    public void openAppDetailsFromRust(String pkg) {
        if (pkg == null || pkg.isEmpty()) return;
        try {
            Intent i = new Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:" + pkg));
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(i);
        } catch (Throwable ignored) { }
    }

    public void uninstallAppFromRust(String pkg) {
        if (pkg == null || pkg.isEmpty()) return;
        try {
            Intent i = new Intent(Intent.ACTION_DELETE, Uri.parse("package:" + pkg));
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(i);
        } catch (Throwable ignored) { }
    }

    public void disableAppFromRust(String pkg) {
        if (pkg == null || pkg.isEmpty()) return;
        // Built-in apps cannot be uninstalled on Android; open App Info directly so user can tap Disable
        openAppDetailsFromRust(pkg);
    }

    public void startVoiceAssistantFromRust() {
        VoiceAssistant.startListening(this);
    }

    public void submitSpokenOrTypedCommandFromRust(String text) {
        if (text != null && !text.trim().isEmpty()) {
            VoiceAssistant.handleSpokenCommand(this, text.trim());
        }
    }

    public void searchInPackageFromRust(String pkg, String query) {
        VoiceAssistant.dispatchInAppSearch(this, pkg, query);
    }

    // ------------------------------------------------------------------ System Snapshot & Launchers

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
        VoiceAssistant.launchPackage(this, pkg);
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
