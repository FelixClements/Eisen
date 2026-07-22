---
name: Testing the Eisen Android client
description: How to build, run on an emulator, and end-to-end test the Eisen Android app (clients/android, package com.example.myapplication). Covers emulator/AVD setup, KVM permission fix, scrcpy mirroring, reliable text input, and DB verification.
---

# Testing the Eisen Android client

App module: `clients/android`, package `com.example.myapplication` (Jetpack Compose + Room + WorkManager).
SDK at `$HOME/android-sdk`; `clients/android/local.properties` has `sdk.dir`. JDK 17 works.

## Build
```
export ANDROID_HOME=$HOME/android-sdk ANDROID_SDK_ROOT=$HOME/android-sdk
(cd clients/android && ./gradlew :app:assembleDebug)
```
APK: `clients/android/app/build/outputs/apk/debug/app-debug.apk` (~50s build).
Compile/target SDK 36, minSdk 34.

## Emulator setup (nothing pre-created)
No emulator package/system-image/AVD exist by default. Install and create one:
```
yes | $HOME/android-sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root=$HOME/android-sdk \
  "emulator" "system-images;android-34;google_apis;x86_64" "platform-tools"
echo no | $HOME/android-sdk/cmdline-tools/latest/bin/avdmanager create avd --force \
  --name eisen_test --package "system-images;android-34;google_apis;x86_64" --device pixel_6
```

## KVM permission fix (REQUIRED)
The `ubuntu` user is NOT in the `kvm` group initially, so the emulator fails with
"This user doesn't have permissions to use KVM". Fix:
```
sudo gpasswd -a ubuntu kvm     # takes effect only in a NEW login/group session
```
Then launch the emulator wrapped in `sg kvm` so it picks up the group without re-login:
```
DISPLAY=:0 nohup sg kvm -c "$HOME/android-sdk/emulator/emulator -avd eisen_test \
  -no-audio -no-boot-anim -gpu swiftshader_indirect -no-snapshot" > /tmp/emulator.log 2>&1 &
```
Boot takes ~30-60s; poll `adb shell getprop sys.boot_completed` == 1.

## Making the emulator visible for recording
The emulator's own Qt window does NOT reliably map onto the desktop (DISPLAY :0) here —
it won't appear in `wmctrl -l`. Use **scrcpy** to mirror instead:
```
sudo apt-get install -y scrcpy
DISPLAY=:0 nohup scrcpy --window-title "Eisen Emulator" -m 1024 > /tmp/scrcpy.log 2>&1 &
```
(The `NoSuchMethodException ... addPrimaryClipChangedListener` clipboard error in scrcpy
logs is harmless; mirroring still works.)

## Reliable text input
Typing via computer-use `type`/scrcpy DROPS leading characters intermittently. Instead:
tap the field with computer-use, then type with adb (use `%s` for spaces):
```
adb shell input text "My%stask%stitle"
adb shell input keyevent 111   # KEYCODE_ESCAPE — dismiss the soft keyboard
```

## Verifying state via Room DB (ground truth)
```
D=/data/data/com.example.myapplication/databases/eisenhower_tasks.db
adb shell "run-as com.example.myapplication sqlite3 $D 'SELECT id,title,isCompleted,isArchived,isPinned FROM tasks;'"
```
Key columns: title, category, description, isImportant, isUrgent (together = Eisenhower
quadrant), dueDate, reminderAt, isPinned, isCompleted, isArchived.

## UI navigation notes
- Home ("Priority Ledger" / "Active tasks"): FAB "[A] Add" opens New Task with a
  default quadrant; pick quadrant via the category selector row, title field, then "Done".
- Each task row: checkbox (left) = complete; archive icon (right) = archive.
- Menu (top-left) opens drawer → History / Settings / Keyboard shortcuts.
- History has "Completed" and "Archived" tabs; the trailing icon on each row is
  Undo (uncomplete) / Restore (unarchive).
- Task detail: edits auto-save on change (no explicit save). Pin & archive icons in the
  top app bar; complete toggle in the Status row.

## Snackbar timing gotcha
Complete/Archive snackbars use `SnackbarDuration.Short` (~4s) with an Undo action. Tool
round-trips can exceed 4s, so a separate screenshot-then-click may miss the window and the
tap hits empty space (looks like "Undo didn't work" — it's a timing artifact). Issue the
action and the Undo tap **in the same computer-use action batch**, then verify via DB.

## Devin Secrets Needed
None.
