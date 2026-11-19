package com.eadxmz.ferrodeck;

import android.app.ActivityManager;
import android.app.AlertDialog;
import android.app.Dialog;
import android.app.NativeActivity;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.ColorDrawable;
import android.graphics.drawable.GradientDrawable;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.ViewGroup;
import android.view.Window;
import android.widget.LinearLayout;
import android.widget.TextView;
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

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
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
    public static native void nativeApplyLoadedPrefs(
            boolean setupDone,
            int setupStep,
            String profileName,
            String langCode,
            boolean classicLook,
            boolean weatherEnabled,
            boolean weatherAuto,
            String weatherCity);
    public static native void nativeUpdateWeather(String city, String tempText, String descText);
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
        loadAndPushSavedPrefs();
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

    /**
     * Returns true ONLY for Android OS / system utilities and hardware vendor daemons that
     * must NEVER appear in the dock (Settings, Dialer, Contacts, Gallery, Camera, DocumentsUI,
     * SystemUI, SetupWizard, WebView Shell, Traceur, Keyboards, Launchers, SoC services).
     * Real user-facing apps — even when pre-installed with FLAG_SYSTEM on TV boxes or emulators
     * (e.g. YouTube, Chrome, TikTok, Instagram, WhatsApp, Messenger, 2048, Get apps, etc.) —
     * are allowed in the dock and tagged with `isBuiltin = true` so their menu shows "Disable".
     */
    private static boolean isSystemUtilityPackage(String pkg) {
        if (pkg == null || pkg.isEmpty()) return true;
        String p = pkg.toLowerCase(java.util.Locale.ROOT);
        if (p.equals("android")
                || p.equals("com.android.settings")
                || p.equals("com.android.tv.settings")
                || p.equals("com.android.systemui")
                || p.equals("com.android.documentsui")
                || p.equals("com.google.android.documentsui")
                || p.equals("com.android.dialer")
                || p.equals("com.google.android.dialer")
                || p.equals("com.android.contacts")
                || p.equals("com.google.android.contacts")
                || p.equals("com.android.messaging")
                || p.equals("com.android.mms")
                || p.equals("com.android.deskclock")
                || p.equals("com.android.calculator2")
                || p.equals("com.android.calendar")
                || p.equals("com.android.gallery3d")
                || p.equals("com.android.camera")
                || p.equals("com.android.camera2")
                || p.equals("org.chromium.webview_shell")
                || p.equals("com.android.webview")
                || p.equals("com.google.android.webview")
                || p.contains(".settings")
                || p.contains("launcher")
                || p.contains("systemui")
                || p.contains("setupwizard")
                || p.contains("provider")
                || p.contains("inputmethod")
                || p.contains("keyboard")
                || p.contains("permissioncontroller")
                || p.contains("packageinstaller")
                || p.contains("traceur")
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
            return true;
        }
        return false;
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

            // 3. Strictly exclude Android OS / system utilities & hardware daemons from the dock
            if (isSystemUtilityPackage(pkg)) {
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

    // ------------------------------------------------------------------ Setup Prefs, Profile & Live Weather

    private void loadAndPushSavedPrefs() {
        SharedPreferences sp = getSharedPreferences("ferrodeck_prefs", Context.MODE_PRIVATE);
        boolean setupDone = sp.getBoolean("setup_done", false);
        int setupStep = sp.getInt("setup_step", 0);
        String profileName = sp.getString("profile_name", "Home");
        String langCode = sp.getString("lang_code", "en");
        boolean classicLook = sp.getBoolean("classic_look", false);
        boolean weatherEnabled = sp.getBoolean("weather_enabled", true);
        boolean weatherAuto = sp.getBoolean("weather_auto", true);
        String weatherCity = sp.getString("weather_city", "");

        nativeApplyLoadedPrefs(
                setupDone,
                setupStep,
                profileName,
                langCode,
                classicLook,
                weatherEnabled,
                weatherAuto,
                weatherCity);

        if (weatherEnabled) {
            fetchWeatherFromRust(weatherAuto, weatherCity);
        }
    }

    public void savePrefsFromRust(
            boolean setupDone,
            int setupStep,
            String profileName,
            String langCode,
            boolean classicLook,
            boolean weatherEnabled,
            boolean weatherAuto,
            String weatherCity) {
        getSharedPreferences("ferrodeck_prefs", Context.MODE_PRIVATE)
                .edit()
                .putBoolean("setup_done", setupDone)
                .putInt("setup_step", setupStep)
                .putString("profile_name", (profileName != null && !profileName.isEmpty()) ? profileName : "Home")
                .putString("lang_code", langCode != null ? langCode : "en")
                .putBoolean("classic_look", classicLook)
                .putBoolean("weather_enabled", weatherEnabled)
                .putBoolean("weather_auto", weatherAuto)
                .putString("weather_city", weatherCity != null ? weatherCity : "")
                .apply();
    }

    private interface InputDialogCallback {
        void onSubmit(String value);
    }

    private int dpPx(float dp) {
        return Math.round(TypedValue.applyDimension(
                TypedValue.COMPLEX_UNIT_DIP, dp, getResources().getDisplayMetrics()));
    }

    private TextView buildGlassPillButton(String label, final boolean primary, final Runnable onClick) {
        final TextView btn = new TextView(this);
        btn.setText(label);
        btn.setSingleLine(true);
        btn.setGravity(Gravity.CENTER);
        btn.setTextSize(TypedValue.COMPLEX_UNIT_SP, 13.5f);
        btn.setTypeface(Typeface.create("sans-serif-medium", Typeface.NORMAL));
        btn.setPadding(dpPx(20), dpPx(10), dpPx(20), dpPx(10));
        btn.setFocusable(true);
        btn.setFocusableInTouchMode(false);
        btn.setClickable(true);

        final int radius = dpPx(999);
        final Runnable applyStyle = () -> {
            boolean focused = btn.isFocused();
            GradientDrawable bg = new GradientDrawable();
            bg.setCornerRadius(radius);
            if (focused) {
                bg.setColor(0xFFF5F7FA);
                bg.setStroke(dpPx(1.5f), 0xFFFFFFFF);
                btn.setTextColor(0xFF10121A);
            } else if (primary) {
                bg.setColor(0x38FFFFFF);
                bg.setStroke(dpPx(1f), 0x77FFFFFF);
                btn.setTextColor(0xFFF6F8FC);
            } else {
                bg.setColor(0x1AFFFFFF);
                bg.setStroke(dpPx(1f), 0x38FFFFFF);
                btn.setTextColor(0xFFD2DAE8);
            }
            btn.setBackground(bg);
        };
        applyStyle.run();
        btn.setOnFocusChangeListener((v, hasFocus) -> applyStyle.run());
        btn.setOnClickListener(v -> {
            if (onClick != null) onClick.run();
        });
        return btn;
    }

    private void showGlassInputDialog(
            final String title,
            final String subtitle,
            final String initialValue,
            final String hint,
            final String positiveLabel,
            final String neutralLabel,
            final Runnable onNeutral,
            final InputDialogCallback onSave) {
        runOnUiThread(() -> {
            try {
                final Dialog dlg = new Dialog(this);
                dlg.requestWindowFeature(Window.FEATURE_NO_TITLE);
                dlg.setCancelable(true);

                LinearLayout card = new LinearLayout(this);
                card.setOrientation(LinearLayout.VERTICAL);
                card.setPadding(dpPx(26), dpPx(24), dpPx(26), dpPx(22));
                GradientDrawable cardBg = new GradientDrawable();
                cardBg.setColor(0xF0151926);
                cardBg.setCornerRadius(dpPx(24));
                cardBg.setStroke(dpPx(1.2f), 0x52FFFFFF);
                card.setBackground(cardBg);

                TextView titleView = new TextView(this);
                titleView.setText(title);
                titleView.setTextColor(0xFFFFFFFF);
                titleView.setTextSize(TypedValue.COMPLEX_UNIT_SP, 19f);
                titleView.setTypeface(Typeface.create("sans-serif-medium", Typeface.BOLD));
                card.addView(titleView);

                if (subtitle != null && !subtitle.isEmpty()) {
                    TextView subView = new TextView(this);
                    subView.setText(subtitle);
                    subView.setTextColor(0xFFB4C0D4);
                    subView.setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f);
                    LinearLayout.LayoutParams slp = new LinearLayout.LayoutParams(
                            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
                    slp.topMargin = dpPx(5);
                    card.addView(subView, slp);
                }

                final EditText input = new EditText(this);
                input.setSingleLine(true);
                input.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_CAP_WORDS);
                input.setImeOptions(EditorInfo.IME_ACTION_DONE);
                input.setTextColor(0xFFFFFFFF);
                input.setHintTextColor(0x88B4C0D4);
                input.setTextSize(TypedValue.COMPLEX_UNIT_SP, 15.5f);
                input.setPadding(dpPx(16), dpPx(12), dpPx(16), dpPx(12));
                if (hint != null) input.setHint(hint);
                if (initialValue != null) {
                    input.setText(initialValue);
                    input.setSelection(initialValue.length());
                }

                final Runnable styleInput = () -> {
                    GradientDrawable ibg = new GradientDrawable();
                    ibg.setCornerRadius(dpPx(14));
                    if (input.isFocused()) {
                        ibg.setColor(0x2BFFFFFF);
                        ibg.setStroke(dpPx(1.6f), 0xE6FFFFFF);
                    } else {
                        ibg.setColor(0x1CFFFFFF);
                        ibg.setStroke(dpPx(1f), 0x44FFFFFF);
                    }
                    input.setBackground(ibg);
                };
                styleInput.run();
                input.setOnFocusChangeListener((v, hasFocus) -> styleInput.run());

                LinearLayout.LayoutParams ilp = new LinearLayout.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
                ilp.topMargin = dpPx(18);
                ilp.bottomMargin = dpPx(20);
                card.addView(input, ilp);

                LinearLayout btnRow = new LinearLayout(this);
                btnRow.setOrientation(LinearLayout.HORIZONTAL);
                btnRow.setGravity(Gravity.END | Gravity.CENTER_VERTICAL);

                if (neutralLabel != null && onNeutral != null) {
                    TextView neutralBtn = buildGlassPillButton(neutralLabel, false, () -> {
                        dlg.dismiss();
                        onNeutral.run();
                    });
                    LinearLayout.LayoutParams nlp = new LinearLayout.LayoutParams(
                            ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
                    nlp.rightMargin = dpPx(10);
                    btnRow.addView(neutralBtn, nlp);
                }

                TextView cancelBtn = buildGlassPillButton("Cancel", false, dlg::dismiss);
                LinearLayout.LayoutParams clp = new LinearLayout.LayoutParams(
                        ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
                clp.rightMargin = dpPx(10);
                btnRow.addView(cancelBtn, clp);

                final Runnable submitAction = () -> {
                    String val = input.getText().toString().trim();
                    dlg.dismiss();
                    if (onSave != null) onSave.onSubmit(val);
                };
                TextView saveBtn = buildGlassPillButton(positiveLabel, true, submitAction);
                btnRow.addView(saveBtn, new LinearLayout.LayoutParams(
                        ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT));

                input.setOnEditorActionListener((v, actionId, event) -> {
                    if (actionId == EditorInfo.IME_ACTION_DONE
                            || (event != null && event.getKeyCode() == KeyEvent.KEYCODE_ENTER
                                && event.getAction() == KeyEvent.ACTION_DOWN)) {
                        submitAction.run();
                        return true;
                    }
                    return false;
                });

                card.addView(btnRow, new LinearLayout.LayoutParams(
                        ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));

                dlg.setContentView(card);
                Window win = dlg.getWindow();
                if (win != null) {
                    win.setBackgroundDrawable(new ColorDrawable(Color.TRANSPARENT));
                    WindowManager.LayoutParams wlp = win.getAttributes();
                    wlp.width = dpPx(430);
                    wlp.height = ViewGroup.LayoutParams.WRAP_CONTENT;
                    wlp.gravity = Gravity.CENTER;
                    wlp.dimAmount = 0.62f;
                    win.setAttributes(wlp);
                    win.addFlags(WindowManager.LayoutParams.FLAG_DIM_BEHIND);
                    win.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_STATE_VISIBLE);
                }
                dlg.show();
                input.requestFocus();
            } catch (Throwable ignored) { }
        });
    }

    public void promptProfileNameFromRust(final String currentName) {
        showGlassInputDialog(
                "Profile Name",
                "Enter the name displayed in your Home greeting.",
                currentName,
                "e.g. Home, Sadim, Living Room",
                "Save",
                null,
                null,
                next -> {
                    if (!next.isEmpty()) {
                        SharedPreferences sp = getSharedPreferences("ferrodeck_prefs", Context.MODE_PRIVATE);
                        sp.edit().putString("profile_name", next).apply();
                        nativeApplyLoadedPrefs(
                                sp.getBoolean("setup_done", false),
                                sp.getInt("setup_step", 1),
                                next,
                                sp.getString("lang_code", "en"),
                                sp.getBoolean("classic_look", false),
                                sp.getBoolean("weather_enabled", true),
                                sp.getBoolean("weather_auto", true),
                                sp.getString("weather_city", ""));
                    }
                });
    }

    public void promptWeatherCityFromRust(final String currentCity) {
        showGlassInputDialog(
                "Choose Weather City",
                "Enter any city name to fetch live Home weather.",
                currentCity,
                "e.g. Srinagar, Mumbai, New Delhi",
                "Save & Fetch",
                null,
                null,
                city -> {
                    if (!city.isEmpty()) {
                        SharedPreferences sp = getSharedPreferences("ferrodeck_prefs", Context.MODE_PRIVATE);
                        sp.edit()
                                .putBoolean("weather_enabled", true)
                                .putBoolean("weather_auto", false)
                                .putString("weather_city", city)
                                .apply();
                        nativeUpdateWeather(city, "Loading...", "");
                        fetchWeatherFromRust(false, city);
                    }
                });
    }

    public void fetchWeatherFromRust(final boolean autoDetect, final String manualCity) {
        new Thread(() -> {
            try {
                double lat = Double.NaN;
                double lon = Double.NaN;
                String resolvedCity = manualCity != null ? manualCity.trim() : "";

                if (autoDetect || resolvedCity.isEmpty()) {
                    // Auto-detect city & coordinates via lightweight IP geolocation
                    String ipJson = fetchUrlText("https://ipwho.is/");
                    if (ipJson != null) {
                        JSONObject j = new JSONObject(ipJson);
                        if (j.optBoolean("success", false)) {
                            lat = j.optDouble("latitude", Double.NaN);
                            lon = j.optDouble("longitude", Double.NaN);
                            resolvedCity = j.optString("city", "");
                        }
                    }
                    if (Double.isNaN(lat)) {
                        String ip2 = fetchUrlText("http://ip-api.com/json/?fields=status,city,lat,lon");
                        if (ip2 != null) {
                            JSONObject j = new JSONObject(ip2);
                            if ("success".equalsIgnoreCase(j.optString("status"))) {
                                lat = j.optDouble("lat", Double.NaN);
                                lon = j.optDouble("lon", Double.NaN);
                                resolvedCity = j.optString("city", "");
                            }
                        }
                    }
                } else {
                    // Manual city geocoding via Open-Meteo Geocoding API
                    String geoUrl = "https://geocoding-api.open-meteo.com/v1/search?name="
                            + URLEncoder.encode(resolvedCity, StandardCharsets.UTF_8)
                            + "&count=1&language=en&format=json";
                    String geoBody = fetchUrlText(geoUrl);
                    if (geoBody != null) {
                        JSONObject gj = new JSONObject(geoBody);
                        JSONArray arr = gj.optJSONArray("results");
                        if (arr != null && arr.length() > 0) {
                            JSONObject first = arr.getJSONObject(0);
                            lat = first.optDouble("latitude", Double.NaN);
                            lon = first.optDouble("longitude", Double.NaN);
                            resolvedCity = first.optString("name", resolvedCity);
                        }
                    }
                }

                if (Double.isNaN(lat) || Double.isNaN(lon)) {
                    nativeUpdateWeather(resolvedCity, "", "");
                    return;
                }

                String wUrl = String.format(
                        java.util.Locale.US,
                        "https://api.open-meteo.com/v1/forecast?latitude=%.4f&longitude=%.4f&current_weather=true",
                        lat, lon);
                String wBody = fetchUrlText(wUrl);
                if (wBody != null) {
                    JSONObject wj = new JSONObject(wBody);
                    JSONObject cw = wj.optJSONObject("current_weather");
                    if (cw != null) {
                        int tempC = (int) Math.round(cw.optDouble("temperature", 0.0));
                        int code = cw.optInt("weathercode", 0);
                        String desc = describeWmoCode(code);
                        getSharedPreferences("ferrodeck_prefs", Context.MODE_PRIVATE)
                                .edit()
                                .putString("weather_city", resolvedCity)
                                .apply();
                        nativeUpdateWeather(resolvedCity, tempC + "°C", desc);
                    }
                }
            } catch (Throwable ignored) { }
        }, "ferrodeck-weather").start();
    }

    private static String describeWmoCode(int code) {
        if (code == 0) return "Clear";
        if (code <= 3) return "Cloudy";
        if (code == 45 || code == 48) return "Foggy";
        if (code >= 51 && code <= 67) return "Rain";
        if (code >= 71 && code <= 77) return "Snow";
        if (code >= 80 && code <= 82) return "Showers";
        if (code >= 95) return "Storm";
        return "Mild";
    }

    private static String fetchUrlText(String urlStr) {
        HttpURLConnection conn = null;
        try {
            conn = (HttpURLConnection) new URL(urlStr).openConnection();
            conn.setConnectTimeout(4000);
            conn.setReadTimeout(4500);
            conn.setRequestProperty("User-Agent", "FerroDeck/0.5 (AndroidTV)");
            if (conn.getResponseCode() != 200) return null;
            BufferedReader br = new BufferedReader(new InputStreamReader(conn.getInputStream(), StandardCharsets.UTF_8));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = br.readLine()) != null) sb.append(line);
            br.close();
            return sb.toString();
        } catch (Throwable ignored) {
            return null;
        } finally {
            if (conn != null) conn.disconnect();
        }
    }

    // ------------------------------------------------------------------ App Long-Press Actions (JNI)

    public void promptRenameAppFromRust(final String pkg, final String currentLabel) {
        if (pkg == null || pkg.isEmpty()) return;
        showGlassInputDialog(
                "Rename App",
                "Voice Assistant will also recognize this custom name.",
                currentLabel,
                "e.g. YouTube",
                "Save",
                "Reset Default",
                () -> {
                    getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE)
                            .edit().remove("rename_" + pkg).apply();
                    refreshInstalledApps();
                },
                next -> {
                    SharedPreferences sp = getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE);
                    if (next.isEmpty()) {
                        sp.edit().remove("rename_" + pkg).apply();
                    } else {
                        sp.edit().putString("rename_" + pkg, next).apply();
                    }
                    refreshInstalledApps();
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
            final boolean portOpen = isLocalAdbPortOpen();
            final String pkg = getPackageName();
            if (!portOpen) {
                refreshSystemSnapshotWithMsg("Local ADB (port 5555) is disabled on this device.");
                runOnUiThread(() -> {
                    try {
                        new AlertDialog.Builder(this, android.R.style.Theme_DeviceDefault_Dialog_Alert)
                                .setTitle("ADB Debugging Not Enabled")
                                .setMessage("Local ADB port 127.0.0.1:5555 is closed.\n\n"
                                        + "To enable Easy Setup on your TV:\n"
                                        + "1. Open Developer Options and enable USB / Network Debugging.\n"
                                        + "2. Or run from your PC:\n"
                                        + "adb shell pm grant " + pkg + " android.permission.WRITE_SECURE_SETTINGS")
                                .setPositiveButton("Open Developer Options", (d, w) ->
                                        openSafeIntent(Settings.ACTION_APPLICATION_DEVELOPMENT_SETTINGS, Settings.ACTION_SETTINGS))
                                .setNegativeButton("Close", null)
                                .show();
                    } catch (Throwable ignored) { }
                });
                return;
            }

            refreshSystemSnapshotWithMsg("ADB port 5555 detected — ready for permission grant.");
            runOnUiThread(() -> {
                try {
                    new AlertDialog.Builder(this, android.R.style.Theme_DeviceDefault_Dialog_Alert)
                            .setTitle("Local ADB Connected (127.0.0.1:5555)")
                            .setMessage("ADB port 5555 is open and reachable.\n\n"
                                    + "Mode: " + (secureMode ? "Secure Mode (grant & close)" : "Persistent ADB Mode") + "\n\n"
                                    + "You can also grant all permissions instantly from PC via:\n"
                                    + "adb shell pm grant " + pkg + " android.permission.WRITE_SECURE_SETTINGS\n"
                                    + "adb shell pm grant " + pkg + " android.permission.RECORD_AUDIO")
                            .setPositiveButton("Open Accessibility Settings", (d, w) ->
                                    openSafeIntent(Settings.ACTION_ACCESSIBILITY_SETTINGS, Settings.ACTION_SETTINGS))
                            .setNegativeButton("Done", null)
                            .show();
                } catch (Throwable ignored) { }
            });
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
