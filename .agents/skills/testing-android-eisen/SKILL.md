---
name: testing-android-eisen
description: How to build, run, and UI-test the Eisen Android app (clients/android, Kotlin + Jetpack Compose) on an emulator.
---

# Testing the Eisen Android app

The app (`clients/android`, package `com.example.myapplication`) is a fully local Priority Ledger
task manager backed by a Room DB — **no backend/server is needed** to test the UI.

## Environment setup
- Android SDK lives at `$HOME/android-sdk`. Set `ANDROID_HOME=$HOME/android-sdk` and ensure
  `clients/android/local.properties` contains `sdk.dir=/home/ubuntu/android-sdk`.
- SDK packages needed: `platform-tools`, `emulator`, `platforms;android-37.0`, `build-tools;37.0.0`,
  and an emulator system image (`system-images;android-35;google_apis;x86_64` worked).
- **KVM permission gotcha:** the x86_64 emulator fails to start with "This user doesn't have permissions
  to use KVM". Fix once per box: `sudo gpasswd -a $USER kvm; sudo chmod 666 /dev/kvm` (chmod takes
  effect immediately, no re-login).
- Create + start emulator:
  `avdmanager create avd -n eisen -k "system-images;android-35;google_apis;x86_64"`
  then `emulator -avd eisen -no-audio -no-snapshot -gpu swiftshader_indirect -no-boot-anim`.
  Wait for `adb shell getprop sys.boot_completed` == 1.

## Build / install
- Build: `(cd clients/android && ANDROID_HOME=$HOME/android-sdk ./gradlew assembleDebug)` (host JDK 17 works).
- Install: `adb install -r clients/android/app/build/outputs/apk/debug/app-debug.apk`.
- Launch: `adb shell monkey -p com.example.myapplication -c android.intent.category.LAUNCHER 1`.

## UI navigation
- Home = **Priority Ledger**. Each Eisenhower category card + the "[A] Add" button opens **New Task**.
- Tapping any task row opens **Task Detail**.
- Task form Due date / Reminder rows are `MetadataRow`s near the bottom (scroll down). Tapping them opens
  `DueDatePickerDialog` (confirm "Done"), `ReminderDatePickerDialog` (confirm "Next" → then
  `ReminderTimePickerDialog` titled "Reminder time", confirm "Done"). The X button clears the value.

## Emulator input gotcha
- Text fields auto-focus and pop the Gboard **floating** keyboard toolbar that overlaps the form's left edge.
  Reliable workaround: type into focused fields via `adb shell input text "Some%stext"` (`%s` = space)
  instead of clicking soft keys. Optionally `adb shell settings put secure show_ime_with_hard_keyboard 0`.
- The IME "Done" action on the title field triggers Save — don't tap the floating toolbar's checkmark unless
  you intend to save.
