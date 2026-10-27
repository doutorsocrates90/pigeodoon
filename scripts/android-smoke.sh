#!/usr/bin/env bash
# Installs the APK on the running emulator, starts the app, and records what happens
# (screenshots, process state and the Android log) in smoke-output/.
set -u
APK="$1"
PKG=io.pigeodoon.discuss
OUT=smoke-output
mkdir -p "$OUT"

adb install -r "$APK" 2>&1 | tee "$OUT/install.txt"
adb shell pm grant "$PKG" android.permission.POST_NOTIFICATIONS 2>&1 | tee -a "$OUT/install.txt" || true
adb logcat -c
adb shell monkey -p "$PKG" -c android.intent.category.LAUNCHER 1 > "$OUT/launch.txt" 2>&1

for s in 5 15 40; do
  sleep $(( s == 5 ? 5 : (s == 15 ? 10 : 25) ))
  adb exec-out screencap -p > "$OUT/screen_${s}s.png"
  { echo "--- after ~${s}s"; adb shell pidof "$PKG" || echo "process NOT running"; } >> "$OUT/process.txt"
done

adb shell dumpsys activity activities 2>/dev/null | grep -E "topResumedActivity|mResumedActivity" > "$OUT/activity.txt" || true
adb shell dumpsys webviewupdate > "$OUT/webview.txt" 2>&1 || true
adb logcat -d -v threadtime > "$OUT/logcat_full.txt"
grep -iE "pigeodoon|tauri|RustStdoutStderr|chromium|cr_|WebView|FATAL|AndroidRuntime|panick|ActivityManager.*(ANR|crash)" "$OUT/logcat_full.txt" | tail -400 > "$OUT/logcat_filtered.txt"
echo "smoke test finished"
