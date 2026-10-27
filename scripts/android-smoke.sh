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
adb shell settings put global hide_error_dialogs 1 || true   # the slow emulator launcher otherwise covers the screen
adb logcat -c
adb shell monkey -p "$PKG" -c android.intent.category.LAUNCHER 1 > "$OUT/launch.txt" 2>&1

for s in 5 15 40; do
  sleep $(( s == 5 ? 5 : (s == 15 ? 10 : 25) ))
  adb exec-out screencap -p > "$OUT/screen_${s}s.png"
  { echo "--- after ~${s}s"; adb shell pidof "$PKG" || echo "process NOT running"; } >> "$OUT/process.txt"
done

# Second part: type the GIG server address and press Continue (1080x2400 screen: field ~y1306, button ~y1566).
adb shell input tap 540 1306
sleep 2
adb shell input text "odoo.globalinnovationgathering.org"
sleep 1
adb exec-out screencap -p > "$OUT/screen_typed.png"
adb shell input tap 540 1566
sleep 20
adb exec-out screencap -p > "$OUT/screen_after_continue.png"
adb shell input keyevent KEYCODE_BACK   # closes the keyboard if it is still open
sleep 2
adb exec-out screencap -p > "$OUT/screen_after_continue_2.png"
adb shell uiautomator dump /sdcard/ui.xml > /dev/null 2>&1 && adb pull /sdcard/ui.xml "$OUT/ui.xml" > /dev/null 2>&1 || true
adb shell dumpsys activity activities 2>/dev/null | grep -E "topResumedActivity|mResumedActivity" > "$OUT/activity.txt" || true
adb shell dumpsys webviewupdate > "$OUT/webview.txt" 2>&1 || true
adb logcat -d -v threadtime > "$OUT/logcat_full.txt"
grep -iE "pigeodoon|tauri|RustStdoutStderr|chromium|cr_|WebView|FATAL|AndroidRuntime|panick|ActivityManager.*(ANR|crash)" "$OUT/logcat_full.txt" | tail -400 > "$OUT/logcat_filtered.txt"
echo "smoke test finished"
