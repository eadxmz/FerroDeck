package com.eadxmz.ferrodeck;

import android.Manifest;
import android.app.Activity;
import android.app.SearchManager;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.content.pm.ResolveInfo;
import android.graphics.Color;
import android.graphics.PixelFormat;
import android.graphics.drawable.GradientDrawable;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.speech.RecognitionListener;
import android.speech.RecognizerIntent;
import android.speech.SpeechRecognizer;
import android.speech.tts.TextToSpeech;
import android.speech.tts.Voice;
import android.util.TypedValue;
import android.view.Gravity;
import android.view.ViewGroup;
import android.view.WindowManager;
import android.widget.LinearLayout;
import android.widget.TextView;

import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;

/**
 * Low-footprint on-demand Voice Assistant for FerroDeck.
 * - Zero background CPU/RAM: SpeechRecognizer & TextToSpeech are created strictly on button press
 *   and destroyed immediately after finishing.
 * - Resolves custom renamed apps (e.g., TizenTube renamed to "YouTube" opens on "open youtube").
 * - Context-aware in-app search: when inside YouTube, TizenTube, SmartTube, Koji TV Browser,
 *   or any app, saying "MrBeast" immediately searches "MrBeast" inside that app.
 * - Silent for actions; speaks with a natural neural/enhanced voice only when asked a question
 *   like "what is 15x2" or "who is the president of India".
 */
public final class VoiceAssistant {

    private static final Handler MAIN = new Handler(Looper.getMainLooper());
    private static SpeechRecognizer activeRecognizer;
    private static TextToSpeech activeTts;
    private static WindowManager overlayWm;
    private static LinearLayout overlayPill;
    private static TextView overlayTitle;
    private static TextView overlaySub;
    private static boolean isListening = false;

    private static final Runnable AUTO_HIDE_HUD = VoiceAssistant::hideHud;

    private VoiceAssistant() { }

    public static void startListening(final Context rawCtx) {
        if (rawCtx == null) return;
        final Context ctx = rawCtx.getApplicationContext();

        MAIN.post(() -> {
            if (isListening) {
                stopAndDestroyRecognizer();
                hideHud();
                return;
            }

            if (ctx.checkSelfPermission(Manifest.permission.RECORD_AUDIO) != PackageManager.PERMISSION_GRANTED) {
                Activity act = MainActivity.activeInstance;
                if (act != null) {
                    act.requestPermissions(new String[]{Manifest.permission.RECORD_AUDIO}, 4401);
                    showHud(ctx, "Microphone Permission", "Allow microphone access, then press Voice button again.", 4500L);
                } else {
                    Intent i = new Intent(ctx, MainActivity.class)
                            .putExtra("request_mic_perm", true)
                            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                    ctx.startActivity(i);
                }
                return;
            }

            if (!SpeechRecognizer.isRecognitionAvailable(ctx)) {
                showHud(ctx, "Voice Assistant", "No speech recognition service installed on this device.", 3500L);
                return;
            }

            stopAndDestroyRecognizer();
            isListening = true;
            showHud(ctx, "Listening…", "Say "Open YouTube", search a video, or ask a question", 0L);
            MainActivity.nativeSetVoiceHud(true, "Listening…", "Speak now…");

            try {
                activeRecognizer = SpeechRecognizer.createSpeechRecognizer(ctx);
                activeRecognizer.setRecognitionListener(new RecognitionListener() {
                    @Override public void onReadyForSpeech(Bundle params) {
                        updateHud("Listening…", "Speak your command…");
                    }
                    @Override public void onBeginningOfSpeech() {
                        updateHud("Listening…", "Hearing you…");
                    }
                    @Override public void onRmsChanged(float rmsdB) { }
                    @Override public void onBufferReceived(byte[] buffer) { }
                    @Override public void onEndOfSpeech() {
                        updateHud("Thinking…", "Processing command…");
                    }
                    @Override
                    public void onError(int error) {
                        isListening = false;
                        stopAndDestroyRecognizer();
                        MainActivity.nativeSetVoiceHud(false, "", "");
                        showHud(ctx, "Voice Assistant", "Didn't catch that — press the button to try again.", 2400L);
                    }
                    @Override
                    public void onResults(Bundle results) {
                        isListening = false;
                        ArrayList<String> matches = results != null
                                ? results.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                                : null;
                        stopAndDestroyRecognizer();
                        MainActivity.nativeSetVoiceHud(false, "", "");
                        String spoken = (matches != null && !matches.isEmpty()) ? matches.get(0) : "";
                        if (spoken.isEmpty()) {
                            hideHud();
                        } else {
                            handleSpokenCommand(ctx, spoken);
                        }
                    }
                    @Override
                    public void onPartialResults(Bundle partialResults) {
                        ArrayList<String> partial = partialResults != null
                                ? partialResults.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                                : null;
                        if (partial != null && !partial.isEmpty() && !partial.get(0).isEmpty()) {
                            updateHud("Listening…", "“" + partial.get(0) + "”");
                            MainActivity.nativeSetVoiceHud(true, "Listening…", partial.get(0));
                        }
                    }
                    @Override public void onEvent(int eventType, Bundle params) { }
                });

                Intent intent = new Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH);
                intent.putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM);
                intent.putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true);
                intent.putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 3);
                intent.putExtra(RecognizerIntent.EXTRA_CALLING_PACKAGE, ctx.getPackageName());
                activeRecognizer.startListening(intent);
            } catch (Throwable t) {
                isListening = false;
                stopAndDestroyRecognizer();
                MainActivity.nativeSetVoiceHud(false, "", "");
                showHud(ctx, "Voice Assistant", "Unable to start microphone.", 2500L);
            }
        });
    }

    private static void stopAndDestroyRecognizer() {
        try {
            if (activeRecognizer != null) {
                activeRecognizer.cancel();
                activeRecognizer.destroy();
            }
        } catch (Throwable ignored) { }
        activeRecognizer = null;
    }

    public static void handleSpokenCommand(final Context ctx, final String rawSpoken) {
        final String text = rawSpoken.trim();
        final String lower = text.toLowerCase(Locale.ROOT)
                .replaceAll("[?!.,]+$", "")
                .trim();

        // 1. Check if it is a question or math calculation -> answer naturally with voice + HUD!
        if (isQuestionOrMath(lower)) {
            showHud(ctx, "“" + text + "”", "Finding answer…", 0L);
            new Thread(() -> {
                String answer = resolveQuestionAnswer(text, lower);
                MAIN.post(() -> {
                    showHud(ctx, "“" + text + "”", answer, 6500L);
                    speakNatural(ctx, answer);
                });
            }, "ferrodeck-qa").start();
            return;
        }

        // 2. System navigation / Control Center commands (silent execution)
        if (lower.equals("go home") || lower.equals("home screen") || lower.equals("open home")) {
            hideHud();
            Intent home = new Intent(ctx, MainActivity.class)
                    .setAction(Intent.ACTION_MAIN)
                    .addCategory(Intent.CATEGORY_HOME)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(home);
            return;
        }
        if (lower.contains("control center") || lower.contains("control panel") || lower.contains("quick settings")) {
            hideHud();
            MainActivity.nativeOnRemoteKey(82, 0); // OpenControlCenter
            return;
        }
        if (lower.equals("open settings") || lower.equals("settings") || lower.equals("home settings")) {
            hideHud();
            MainActivity.nativeOnRemoteKey(176, 0); // OpenSettings
            return;
        }
        if (lower.contains("clean memory") || lower.contains("free ram") || lower.contains("boost memory")) {
            showHud(ctx, "Memory Cleaned", "Background apps cleared.", 2000L);
            if (MainActivity.activeInstance != null) {
                MainActivity.activeInstance.cleanMemoryFromRust();
            }
            return;
        }

        // 3. Compound command: "search <query> on/in <app>" or "open <app> and search <query>"
        String compoundApp = null;
        String compoundQuery = null;
        if (lower.startsWith("search ") || lower.startsWith("find ") || lower.startsWith("play ")) {
            String rest = text.substring(text.indexOf(' ') + 1).trim();
            String restLower = rest.toLowerCase(Locale.ROOT);
            int splitIdx = restLower.lastIndexOf(" on ");
            int sepLen = 4;
            if (splitIdx < 0) {
                splitIdx = restLower.lastIndexOf(" in ");
            }
            if (splitIdx > 0) {
                compoundQuery = rest.substring(0, splitIdx).trim();
                compoundApp = rest.substring(splitIdx + sepLen).trim();
            }
        } else if ((lower.startsWith("open ") || lower.startsWith("launch ")) && lower.contains(" and search ")) {
            int idx = lower.indexOf(" and search ");
            int startApp = text.indexOf(' ') + 1;
            compoundApp = text.substring(startApp, idx).trim();
            compoundQuery = text.substring(idx + " and search ".length()).trim();
        }

        if (compoundApp != null && compoundQuery != null && !compoundQuery.isEmpty()) {
            String pkg = findAppPackageByCustomOrRealName(ctx, compoundApp);
            if (pkg != null) {
                String disp = getEffectiveAppName(ctx, pkg);
                showHud(ctx, "Searching " + disp, "“" + compoundQuery + "”", 2200L);
                dispatchInAppSearch(ctx, pkg, compoundQuery);
                return;
            }
        }

        // 4. Explicit app launch: "open <app name>" / "launch <app name>" / "start <app name>"
        if (lower.startsWith("open ") || lower.startsWith("launch ") || lower.startsWith("start ")) {
            String targetName = text.substring(text.indexOf(' ') + 1).trim();
            String pkg = findAppPackageByCustomOrRealName(ctx, targetName);
            if (pkg != null) {
                String disp = getEffectiveAppName(ctx, pkg);
                showHud(ctx, "Opening " + disp, "", 1600L);
                launchPackage(ctx, pkg);
                return;
            }
        }

        // 5. Contextual in-app search: if user is currently inside YouTube, TizenTube,
        //    SmartTube, Koji TV Browser, or any other foreground app, search the query inside it!
        String fgPkg = getActiveForegroundApp(ctx);
        if (fgPkg != null && !fgPkg.isEmpty() && !ctx.getPackageName().equals(fgPkg)) {
            String query = text;
            if (lower.startsWith("search for ")) {
                query = text.substring("search for ".length()).trim();
            } else if (lower.startsWith("search ")) {
                query = text.substring("search ".length()).trim();
            } else if (lower.startsWith("look up ")) {
                query = text.substring("look up ".length()).trim();
            }
            String disp = getEffectiveAppName(ctx, fgPkg);
            showHud(ctx, "Searching in " + disp, "“" + query + "”", 2200L);
            dispatchInAppSearch(ctx, fgPkg, query);
            return;
        }

        // 6. On Home Screen without "open": check if spoken phrase directly matches a renamed/installed app
        String directPkg = findAppPackageByCustomOrRealName(ctx, text);
        if (directPkg != null) {
            String disp = getEffectiveAppName(ctx, directPkg);
            showHud(ctx, "Opening " + disp, "", 1600L);
            launchPackage(ctx, directPkg);
            return;
        }

        // 7. Fallback on Home Screen: open FerroDeck's native Search Overlay pre-filled with the query!
        hideHud();
        String cleanQuery = lower.startsWith("search ") ? text.substring(7).trim() : text;
        MainActivity.nativeOpenSearchWithQuery(cleanQuery);
    }

    private static String getActiveForegroundApp(Context ctx) {
        String fg = RemoteFilterService.foregroundPackage;
        if (fg != null && !fg.isEmpty() && !ctx.getPackageName().equals(fg)
                && !"com.android.systemui".equals(fg)) {
            return fg;
        }
        return MainActivity.lastLaunchedPackage;
    }

    /**
     * Resolves an app package by matching:
     *  1. Custom renamed label saved in `ferrodeck_apps` (e.g., TizenTube renamed to "YouTube")
     *  2. Installed app label from PackageManager
     *  3. Package name / common aliases
     */
    public static String findAppPackageByCustomOrRealName(Context ctx, String queryName) {
        if (queryName == null || queryName.trim().isEmpty()) return null;
        String needle = normalizeName(queryName);
        if (needle.isEmpty()) return null;

        PackageManager pm = ctx.getPackageManager();
        SharedPreferences sp = ctx.getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE);

        // Pass 1: Exact match against custom renamed names first (highest priority!)
        for (Map.Entry<String, ?> entry : sp.getAll().entrySet()) {
            String k = entry.getKey();
            if (k.startsWith("rename_") && entry.getValue() instanceof String) {
                String pkg = k.substring("rename_".length());
                String customNorm = normalizeName((String) entry.getValue());
                if (!customNorm.isEmpty() && customNorm.equals(needle) && isPackageInstalled(pm, pkg)) {
                    return pkg;
                }
            }
        }

        // Pass 2: Partial match against custom renamed names
        for (Map.Entry<String, ?> entry : sp.getAll().entrySet()) {
            String k = entry.getKey();
            if (k.startsWith("rename_") && entry.getValue() instanceof String) {
                String pkg = k.substring("rename_".length());
                String customNorm = normalizeName((String) entry.getValue());
                if (!customNorm.isEmpty()
                        && (customNorm.contains(needle) || needle.contains(customNorm))
                        && isPackageInstalled(pm, pkg)) {
                    return pkg;
                }
            }
        }

        // Pass 3: Collect all launchable apps (Leanback + standard) and match effective label
        List<ResolveInfo> all = new ArrayList<>();
        Intent leanback = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LEANBACK_LAUNCHER);
        all.addAll(pm.queryIntentActivities(leanback, 0));
        Intent launcher = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER);
        all.addAll(pm.queryIntentActivities(launcher, 0));

        String bestPartialPkg = null;
        int bestScore = 0;

        for (ResolveInfo ri : all) {
            if (ri.activityInfo == null) continue;
            String pkg = ri.activityInfo.packageName;
            if (ctx.getPackageName().equals(pkg)) continue;

            String customName = sp.getString("rename_" + pkg, null);
            String effective = (customName != null && !customName.trim().isEmpty())
                    ? customName
                    : ri.loadLabel(pm).toString();
            String normLabel = normalizeName(effective);
            String rawNorm = normalizeName(ri.loadLabel(pm).toString());

            if (normLabel.equals(needle) || rawNorm.equals(needle)) {
                return pkg;
            }
            if (normLabel.startsWith(needle) || rawNorm.startsWith(needle)) {
                if (bestScore < 3) {
                    bestScore = 3;
                    bestPartialPkg = pkg;
                }
            } else if (normLabel.contains(needle) || rawNorm.contains(needle)
                    || (needle.length() >= 4 && (needle.contains(normLabel) || needle.contains(rawNorm)))) {
                if (bestScore < 2) {
                    bestScore = 2;
                    bestPartialPkg = pkg;
                }
            }
        }
        return bestPartialPkg;
    }

    private static boolean isPackageInstalled(PackageManager pm, String pkg) {
        try {
            pm.getPackageInfo(pkg, 0);
            return true;
        } catch (Throwable ignored) {
            return false;
        }
    }

    private static String normalizeName(String s) {
        return s == null ? "" : s.toLowerCase(Locale.ROOT).replaceAll("[^a-z0-9\p{L}]", "");
    }

    public static String getEffectiveAppName(Context ctx, String pkg) {
        SharedPreferences sp = ctx.getSharedPreferences("ferrodeck_apps", Context.MODE_PRIVATE);
        String custom = sp.getString("rename_" + pkg, null);
        if (custom != null && !custom.trim().isEmpty()) {
            return custom.trim();
        }
        try {
            PackageManager pm = ctx.getPackageManager();
            return pm.getApplicationLabel(pm.getApplicationInfo(pkg, 0)).toString();
        } catch (Throwable ignored) {
            return pkg;
        }
    }

    public static void launchPackage(Context ctx, String pkg) {
        if (pkg == null || pkg.isEmpty()) return;
        try {
            PackageManager pm = ctx.getPackageManager();
            Intent i = pm.getLeanbackLaunchIntentForPackage(pkg);
            if (i == null) i = pm.getLaunchIntentForPackage(pkg);
            if (i != null) {
                MainActivity.lastLaunchedPackage = pkg;
                i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                ctx.startActivity(i);
            }
        } catch (Throwable ignored) { }
    }

    /**
     * Dispatches an in-app search directly into `targetPkg` (YouTube, TizenTube, SmartTube,
     * Koji TV Browser, Chrome, Play Store, etc.) and brings search results forward.
     */
    public static void dispatchInAppSearch(Context ctx, String targetPkg, String query) {
        if (targetPkg == null || targetPkg.isEmpty() || query == null || query.trim().isEmpty()) return;
        String q = query.trim();
        MainActivity.lastLaunchedPackage = targetPkg;
        PackageManager pm = ctx.getPackageManager();
        String pkgLower = targetPkg.toLowerCase(Locale.ROOT);
        String encoded = URLEncoder.encode(q, StandardCharsets.UTF_8);

        // 1. YouTube / TizenTube / SmartTube clients: open YouTube search results URL scoped to that app
        if (pkgLower.contains("youtube") || pkgLower.contains("tizentube") || pkgLower.contains("smarttube")) {
            try {
                Intent ytSearch = new Intent(Intent.ACTION_SEARCH);
                ytSearch.setPackage(targetPkg);
                ytSearch.putExtra(SearchManager.QUERY, q);
                ytSearch.putExtra("query", q);
                ytSearch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                if (ytSearch.resolveActivity(pm) != null) {
                    ctx.startActivity(ytSearch);
                    return;
                }
            } catch (Throwable ignored) { }

            try {
                Intent ytUrl = new Intent(
                        Intent.ACTION_VIEW,
                        Uri.parse("https://www.youtube.com/results?search_query=" + encoded));
                ytUrl.setPackage(targetPkg);
                ytUrl.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                ctx.startActivity(ytUrl);
                return;
            } catch (Throwable ignored) { }
        }

        // 2. Browsers (Koji TV Browser, Chrome, Firefox, TV Bro, etc.): open Google search URL in that browser
        if (pkgLower.contains("koji") || pkgLower.contains("browser") || pkgLower.contains("chrome")
                || pkgLower.contains("firefox") || pkgLower.contains("webview") || pkgLower.contains("opera")) {
            try {
                Intent webSearch = new Intent(
                        Intent.ACTION_VIEW,
                        Uri.parse("https://www.google.com/search?q=" + encoded));
                webSearch.setPackage(targetPkg);
                webSearch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                ctx.startActivity(webSearch);
                return;
            } catch (Throwable ignored) { }
        }

        // 3. Generic Android app ACTION_SEARCH / MEDIA_PLAY_FROM_SEARCH
        String[] searchActions = new String[]{
            Intent.ACTION_SEARCH,
            "android.media.action.MEDIA_PLAY_FROM_SEARCH",
            "com.google.android.gms.actions.SEARCH_ACTION"
        };
        for (String act : searchActions) {
            try {
                Intent si = new Intent(act);
                si.setPackage(targetPkg);
                si.putExtra(SearchManager.QUERY, q);
                si.putExtra("query", q);
                si.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                if (si.resolveActivity(pm) != null) {
                    ctx.startActivity(si);
                    return;
                }
            } catch (Throwable ignored) { }
        }

        // 4. Accessibility UI fallback: if the app is already open with a search box, fill it directly
        if (RemoteFilterService.instance != null
                && RemoteFilterService.instance.tryFillSearchFieldInForeground(q)) {
            return;
        }

        // 5. Final fallback: launch the app and open web search if it accepts https URLs
        try {
            Intent viewFallback = new Intent(
                    Intent.ACTION_VIEW,
                    Uri.parse("https://www.google.com/search?q=" + encoded));
            viewFallback.setPackage(targetPkg);
            viewFallback.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            if (viewFallback.resolveActivity(pm) != null) {
                ctx.startActivity(viewFallback);
                return;
            }
        } catch (Throwable ignored) { }

        launchPackage(ctx, targetPkg);
    }

    // ------------------------------------------------------------------ Question & Math Engine

    private static boolean isQuestionOrMath(String lower) {
        if (lower.startsWith("what ") || lower.startsWith("what's ") || lower.startsWith("whats ")
                || lower.startsWith("who ") || lower.startsWith("who's ") || lower.startsWith("whos ")
                || lower.startsWith("where ") || lower.startsWith("when ")
                || lower.startsWith("why ") || lower.startsWith("how ")
                || lower.startsWith("calculate ") || lower.startsWith("define ")
                || lower.startsWith("tell me about ")) {
            return true;
        }
        return extractMathExpression(lower) != null;
    }

    private static String extractMathExpression(String lower) {
        String expr = lower
                .replace("what is", "")
                .replace("what's", "")
                .replace("whats", "")
                .replace("calculate", "")
                .replace("multiplied by", "*")
                .replace("divided by", "/")
                .replace("times", "*")
                .replace("into", "*")
                .replace("plus", "+")
                .replace("minus", "-")
                .replace("x", "*")
                .replace("×", "*")
                .replace("÷", "/")
                .trim();
        if (expr.matches("[0-9.+\-*/()\s]+") && expr.matches(".*[+\-*/].*") && expr.matches(".*[0-9].*")) {
            return expr;
        }
        return null;
    }

    private static String resolveQuestionAnswer(String rawText, String lower) {
        // 1. Fast local arithmetic evaluation (e.g. "what is 15x2" -> "15 times 2 is 30.")
        String mathExpr = extractMathExpression(lower);
        if (mathExpr != null) {
            try {
                double val = evalSimpleMath(mathExpr);
                if (!Double.isNaN(val) && !Double.isInfinite(val)) {
                    String formatted = (Math.abs(val - Math.rint(val)) < 1e-9)
                            ? String.valueOf((long) Math.rint(val))
                            : String.format(Locale.US, "%.4f", val).replaceAll("0+$", "").replaceAll("\.$", "");
                    return "The answer is " + formatted + ".";
                }
            } catch (Throwable ignored) { }
        }

        // 2. Clean topic query for background knowledge search
        String topic = rawText
                .replaceAll("(?i)^(what is|what's|whats|who is|who's|whos|where is|when was|tell me about|define)\s+", "")
                .replaceAll("[?!.]+$", "")
                .trim();
        if (topic.isEmpty()) topic = rawText;

        // 3. Query DuckDuckGo Instant Answer API
        try {
            String urlStr = "https://api.duckduckgo.com/?q="
                    + URLEncoder.encode(topic, StandardCharsets.UTF_8)
                    + "&format=json&no_html=1&skip_disambig=1";
            String body = httpGet(urlStr);
            if (body != null && !body.isEmpty()) {
                JSONObject json = new JSONObject(body);
                String ans = json.optString("Answer", "").trim();
                if (!ans.isEmpty()) return trimToTwoSentences(ans);
                String abs = json.optString("AbstractText", "").trim();
                if (!abs.isEmpty()) return trimToTwoSentences(abs);
            }
        } catch (Throwable ignored) { }

        // 4. Fallback to Wikipedia REST summary
        try {
            String wikiTitle = URLEncoder.encode(topic.replace(' ', '_'), StandardCharsets.UTF_8);
            String wikiBody = httpGet("https://en.wikipedia.org/api/rest_v1/page/summary/" + wikiTitle);
            if (wikiBody != null && !wikiBody.isEmpty()) {
                JSONObject wj = new JSONObject(wikiBody);
                String extract = wj.optString("extract", "").trim();
                if (!extract.isEmpty()) return trimToTwoSentences(extract);
            }
        } catch (Throwable ignored) { }

        return "Here is what I found for " + topic + ".";
    }

    private static String trimToTwoSentences(String text) {
        String clean = text.replaceAll("\s+", " ").trim();
        int firstEnd = clean.indexOf(". ");
        if (firstEnd > 0) {
            int secondEnd = clean.indexOf(". ", firstEnd + 2);
            if (secondEnd > 0 && secondEnd < 240) {
                return clean.substring(0, secondEnd + 1);
            }
            if (firstEnd < 240) {
                return clean.substring(0, firstEnd + 1);
            }
        }
        return clean.length() > 220 ? clean.substring(0, 217) + "..." : clean;
    }

    private static String httpGet(String urlStr) {
        HttpURLConnection conn = null;
        try {
            conn = (HttpURLConnection) new URL(urlStr).openConnection();
            conn.setConnectTimeout(3500);
            conn.setReadTimeout(4000);
            conn.setRequestProperty("User-Agent", "FerroDeck/0.4 (AndroidTV)");
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

    // Recursive-descent arithmetic evaluator (+, -, *, /, parentheses)
    private static double evalSimpleMath(final String str) {
        return new Object() {
            int pos = -1, ch;
            void nextChar() { ch = (++pos < str.length()) ? str.charAt(pos) : -1; }
            boolean eat(int charToEat) {
                while (ch == ' ') nextChar();
                if (ch == charToEat) { nextChar(); return true; }
                return false;
            }
            double parse() {
                nextChar();
                double x = parseExpression();
                return (pos < str.length()) ? Double.NaN : x;
            }
            double parseExpression() {
                double x = parseTerm();
                for (;;) {
                    if (eat('+')) x += parseTerm();
                    else if (eat('-')) x -= parseTerm();
                    else return x;
                }
            }
            double parseTerm() {
                double x = parseFactor();
                for (;;) {
                    if (eat('*')) x *= parseFactor();
                    else if (eat('/')) x /= parseFactor();
                    else return x;
                }
            }
            double parseFactor() {
                if (eat('+')) return parseFactor();
                if (eat('-')) return -parseFactor();
                double x;
                int startPos = this.pos;
                if (eat('(')) {
                    x = parseExpression();
                    eat(')');
                } else if ((ch >= '0' && ch <= '9') || ch == '.') {
                    while ((ch >= '0' && ch <= '9') || ch == '.') nextChar();
                    x = Double.parseDouble(str.substring(startPos, this.pos));
                } else {
                    return Double.NaN;
                }
                return x;
            }
        }.parse();
    }

    // ------------------------------------------------------------------ Natural Neural TTS

    private static void speakNatural(final Context ctx, final String text) {
        if (text == null || text.trim().isEmpty()) return;
        try {
            if (activeTts != null) {
                activeTts.stop();
                activeTts.shutdown();
                activeTts = null;
            }
        } catch (Throwable ignored) { }

        activeTts = new TextToSpeech(ctx, status -> {
            if (status == TextToSpeech.SUCCESS && activeTts != null) {
                try {
                    activeTts.setLanguage(Locale.US);
                    // Prefer highest-quality natural/neural voice over robotic default
                    if (Build.VERSION.SDK_INT >= 21) {
                        Set<Voice> voices = activeTts.getVoices();
                        if (voices != null) {
                            Voice best = null;
                            int bestRank = -1;
                            for (Voice v : voices) {
                                if (v == null || v.getLocale() == null) continue;
                                if (!"en".equalsIgnoreCase(v.getLocale().getLanguage())) continue;
                                int rank = v.getQuality();
                                String nameLow = v.getName().toLowerCase(Locale.ROOT);
                                if (nameLow.contains("neural") || nameLow.contains("wavenet")
                                        || nameLow.contains("natural") || nameLow.contains("studio")) {
                                    rank += 300;
                                }
                                if (!v.isNetworkConnectionRequired()) {
                                    rank += 50; // Prefer fast, zero-lag local enhanced voice
                                }
                                if (rank > bestRank) {
                                    bestRank = rank;
                                    best = v;
                                }
                            }
                            if (best != null) {
                                activeTts.setVoice(best);
                            }
                        }
                    }
                    activeTts.setSpeechRate(0.98f);
                    activeTts.setPitch(1.02f);
                    activeTts.speak(text, TextToSpeech.QUEUE_FLUSH, null, "ferrodeck_qa");
                } catch (Throwable ignored) { }
            }
        });
    }

    // ------------------------------------------------------------------ Floating Liquid Glass HUD

    private static void showHud(Context ctx, String title, String sub, long autoHideMs) {
        MAIN.removeCallbacks(AUTO_HIDE_HUD);
        try {
            if (overlayPill == null) {
                WindowManager wm = (WindowManager) ctx.getSystemService(Context.WINDOW_SERVICE);
                if (wm == null) return;
                overlayWm = wm;

                LinearLayout box = new LinearLayout(ctx);
                box.setOrientation(LinearLayout.VERTICAL);
                box.setGravity(Gravity.CENTER_HORIZONTAL);
                box.setPadding(44, 24, 44, 24);

                GradientDrawable bg = new GradientDrawable();
                bg.setColor(0xEB151924);
                bg.setCornerRadius(999f);
                bg.setStroke(2, 0x55FFFFFF);
                box.setBackground(bg);

                TextView t1 = new TextView(ctx);
                t1.setTextColor(Color.WHITE);
                t1.setTextSize(TypedValue.COMPLEX_UNIT_SP, 15f);
                t1.setGravity(Gravity.CENTER);
                box.addView(t1);

                TextView t2 = new TextView(ctx);
                t2.setTextColor(0xFF93D5FF);
                t2.setTextSize(TypedValue.COMPLEX_UNIT_SP, 13f);
                t2.setGravity(Gravity.CENTER);
                t2.setMaxWidth(920);
                box.addView(t2);

                int type = (RemoteFilterService.instance != null && Build.VERSION.SDK_INT >= 22)
                        ? WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY
                        : (Build.VERSION.SDK_INT >= 26
                            ? WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
                            : WindowManager.LayoutParams.TYPE_PHONE);

                WindowManager.LayoutParams lp = new WindowManager.LayoutParams(
                        ViewGroup.LayoutParams.WRAP_CONTENT,
                        ViewGroup.LayoutParams.WRAP_CONTENT,
                        type,
                        WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
                                | WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE
                                | WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
                        PixelFormat.TRANSLUCENT);
                lp.gravity = Gravity.TOP | Gravity.CENTER_HORIZONTAL;
                lp.y = 42;

                overlayPill = box;
                overlayTitle = t1;
                overlaySub = t2;
                wm.addView(box, lp);
            }
            updateHud(title, sub);
            if (autoHideMs > 0L) {
                MAIN.postDelayed(AUTO_HIDE_HUD, autoHideMs);
            }
        } catch (Throwable ignored) { }
    }

    private static void updateHud(String title, String sub) {
        if (overlayTitle != null) overlayTitle.setText(title);
        if (overlaySub != null) {
            if (sub == null || sub.isEmpty()) {
                overlaySub.setVisibility(android.view.View.GONE);
            } else {
                overlaySub.setVisibility(android.view.View.VISIBLE);
                overlaySub.setText(sub);
            }
        }
    }

    public static void hideHud() {
        MAIN.removeCallbacks(AUTO_HIDE_HUD);
        try {
            if (overlayWm != null && overlayPill != null) {
                overlayWm.removeView(overlayPill);
            }
        } catch (Throwable ignored) { }
        overlayPill = null;
        overlayTitle = null;
        overlaySub = null;
    }
}
