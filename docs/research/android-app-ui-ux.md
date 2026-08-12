# Eisen Android client — UI/UX research

This document captures the UI/UX, navigation, data model and behavior of the existing Android client at `clients/android` (package `com.example.myapplication`).
It is based entirely on the source files in this repo; no source files were modified.

---

## 1. Project structure and entry points

* **Build configuration**
  * `clients/android/app/build.gradle.kts:9-39` — Android application with namespace `com.example.myapplication`, `minSdk 34`, `targetSdk 36`, `compileSdk 37`, `versionCode 1`, `versionName "1.0"`.
  * `clients/android/app/build.gradle.kts:47-74` — Compose, Material3, Room, Navigation Compose, WorkManager, Kotlin coroutines; KSP for Room.
  * `clients/android/build.gradle.kts:2-6` — top-level plugin aliases.
  * `clients/android/settings.gradle.kts:25-26` — project name `My Application`, single module `:app`.
* **Manifest**
  * `clients/android/app/src/main/AndroidManifest.xml:5-27` — single permission `POST_NOTIFICATIONS`, one `MainActivity`, `android:allowBackup="false"`, `android:theme="@style/Theme.MyApplication"`, `windowSoftInputMode="adjustResize"`.
* **Entry point**
  * `clients/android/app/src/main/java/com/example/myapplication/MainActivity.kt:25-67` — `ComponentActivity`:
    * `onCreate` calls `enableEdgeToEdge()` (line 30).
    * Creates the notification channel `TaskReminderNotifications.createChannel(applicationContext)` (line 32).
    * Builds the Room database via `DatabaseProvider.getDatabase(applicationContext)` and `LocalTaskRepository` (lines 33-34).
    * Instantiates `HomeViewModel` and `HistoryViewModel` through custom `ViewModelProvider.Factory` implementations (lines 36-44).
    * Reads any `EXTRA_TASK_ID` from the launch `Intent` (line 46).
    * `setContent { MyApplicationTheme { PriorityLedgerApp(...) } }` (lines 48-57).
    * `onNewIntent` updates `initialTaskId` from a notification extra (lines 61-67).

---

## 2. Screens and navigation flow

* **Root navigation host**
  * `clients/android/app/src/main/java/com/example/myapplication/ui/navigation/PriorityLedgerApp.kt:65-77` — `PriorityLedgerRoutes` defines routes: `LEDGER`, `HISTORY`, `SETTINGS`, `KEYBOARD_SHORTCUTS`, `NEW_TASK` (`new-task/{defaultCategory}`), `TASK_DETAIL` (`task-detail/{taskId}`). Helper functions `newTask(...)` and `taskDetail(...)` build URLs.
  * `PriorityLedgerApp.kt:90-236` — `NavHost` with `startDestination = PriorityLedgerRoutes.LEDGER`.
    * `LEDGER` -> `PriorityLedgerHomeRoute` (lines 145-160).
    * `HISTORY` -> `HistoryRoute` (lines 162-174).
    * `SETTINGS` -> `SettingsScreen` (lines 176-184).
    * `KEYBOARD_SHORTCUTS` -> `KeyboardShortcutsScreen` (lines 186-192).
    * `TASK_DETAIL` with `navArgument("taskId") { type = NavType.LongType }` (lines 193-208) — creates a `TaskDetailViewModel` with a per-route `ViewModelProvider.Factory`, then `TaskDetailScreen`.
    * `NEW_TASK` with `navArgument("defaultCategory") { type = NavType.StringType }` (lines 209-234) — parses category default, then `NewTaskScreen`.
* **Top-level navigation is a Modal navigation drawer, not a bottom bar.**
  * `PriorityLedgerApp.kt:98-138` — `ModalNavigationDrawer` wrapping `NavHost`; `BackHandler` closes the drawer (line 112).
  * `PriorityLedgerApp.kt:240-282` — `LedgerNavigationDrawer` with four destinations: Home (`ViewList`), History (`History`), Settings (`Settings`), Keyboard Shortcuts (`Keyboard`). Each item has a `testTag` of `$DrawerItemTagPrefix${destination.route}` (line 278).
  * `PriorityLedgerApp.kt:116-127` — `navigateToTopLevel` uses `popUpTo(LEDGER) { saveState = true }`, `launchSingleTop = true`, `restoreState = true`.
* **Notification-driven deep links**
  * `PriorityLedgerApp.kt:92-97` — on launch from a reminder notification, `initialTaskId` triggers `navController.navigate(PriorityLedgerRoutes.taskDetail(initialTaskId))`.
  * `TaskReminderWorker.kt:74-82` — notification `PendingIntent` opens `MainActivity` with `EXTRA_TASK_ID`.

---

## 3. Task UI/UX components

### 3.1 Home / ledger screen

* `clients/android/app/src/main/java/com/example/myapplication/ui/home/PriorityLedgerHomeScreen.kt:105-129` — `PriorityLedgerHomeRoute` collects `HomeUiState` and `SharedFlow<HomeUiEvent>` from `HomeViewModel`.
* **Loading / error / empty states**
  * `PriorityLedgerHomeScreen.kt:307-354` — `CircularProgressIndicator` when `isLoading`; error with `Button(onClick = onRetry)`; empty state message from `R.string.ledger_empty` or `R.string.search_empty`.
* **Top search overlay**
  * `PriorityLedgerHomeScreen.kt:454-517` — `LedgerTopOverlay` is a 56.dp height row. Inactive: `Menu` drawer button and `Search` icon. Active: `ArrowBack` and an `OutlinedTextField` that searches title/notes.
  * Search state is `rememberSaveable` (`isSearchActive`, `searchQuery`) and focus is requested via `searchFocusRequester`.
  * `HomeViewModel.kt:71-73` — `search(query)` updates `searchQuery` StateFlow; repository returns `searchTasks(query)` when non-blank, otherwise `observeActiveTasks()`.
* **Task list / rows**
  * `PriorityLedgerHomeScreen.kt:357-429` — `LazyColumn` with `contentPadding = PaddingValues(start = 16.dp, top = TopBarHeight, end = 16.dp, bottom = 96.dp)` and `verticalArrangement = Arrangement.spacedBy(12.dp)`.
  * Section headers (`CategorySectionHeader`, lines 519-590) and `PriorityTaskRow` (lines 592-700) alternate.
  * `PriorityTaskRow` uses an `OutlinedCard` with a `Checkbox` on the left, title, status line, optional pin and reminder-error icons, and an `Archive` icon button on the right.
  * `taskStatusLine` (`PriorityLedgerHomeScreen.kt:809-816`) joins `DateTimeUtils.formatDueDate(...)` and the optional `category` label with ` · `.
  * The row border becomes 3.dp primary color when keyboard-focused (lines 626-630) and has `CustomAccessibilityAction`s for complete/incomplete and archive (lines 615-624).
* **Floating action button**
  * `PriorityLedgerHomeScreen.kt:287-296` — `ExtendedFloatingActionButton` labeled "[A] Add", content description "Add task. Keyboard shortcut A", opens the composer with `defaultNewTaskCategory()`.
* **Keyboard navigation**
  * `PriorityLedgerHomeScreen.kt:361-394` — `onPreviewKeyEvent` on the list:
    * `J`/`K` — move focus down/up.
    * `Q`/`W`/`E`/`R` — jump to Do Now / Schedule / Delegate / Eliminate section.
    * `Space` — toggle complete on focused task.
    * `Backspace` — archive focused task.
    * `A` — open New Task composer.
    * `M` — open navigation drawer.
    * `?` (`Shift + /`) — open keyboard shortcuts screen.
  * `PriorityLedgerHomeScreen.kt:239-249`/`251-259` — `moveFocus` and `jumpToCategory` animate the list to the relevant index and update `focusedTaskId`.
* **Undo snackbars**
  * `PriorityLedgerHomeScreen.kt:187-217` — `HomeUiEvent.TaskCompleted` and `TaskArchived` trigger `SnackbarDuration.Short` with action "Undo". On action the UI reverts the operation by calling the supplied callback.

### 3.2 New Task composer

* `clients/android/app/src/main/java/com/example/myapplication/ui/task/NewTaskScreen.kt:104-215` — `NewTaskScreen` is a full-screen, not a dialog or bottom sheet.
  * `Scaffold` with a `BottomAppBar` containing a full-width `Button` (Save / "Saving…").
  * `LaunchedEffect(Unit)` requests title focus immediately (line 148).
  * Draft fields: `title`, `notes`, `taskCategory`, `dueDate`, `reminderAt`, `reminderDate`, `selectedCategory`, all `rememberSaveable`.
  * `hasDraft()` (lines 152-159) and `BackHandler` (line 236) trigger `DiscardConfirmationDialog` if a draft exists.
  * `save()` (lines 173-215) trims the title, validates non-blank, warns on a past reminder, then calls `onSaveTask`. On failure it shows a `Retry` snackbar.
* **Category selector**
  * `NewTaskScreen.kt:543-645` — `CategoryGrid` -> `CategorySegmentedRadioRow` -> `CategorySegmentedRadioButton`, four horizontal cells (one per `EisenhowerCategory`).
  * Selected cell uses `categoryColors.container`/`onContainer` and a 2.dp outline; unselected uses `MaterialTheme.colorScheme.outlineVariant`.
  * `Alt + Q/W/E/R` keyboard shortcut (lines 709-719) selects the quadrant while typing; `Enter` on the title field saves (lines 327-334).
* **Details**
  * `NewTaskScreen.kt:480-540` — `DetailsArea`: notes (max 6 lines), free-text `taskCategory`, `MetadataRow` for due date (calendar icon), `MetadataRow` for reminder (notifications icon).
  * Date / time pickers from `TaskFormComponents.kt:70-169`: `DueDatePickerDialog`, `ReminderDatePickerDialog`, `ReminderTimePickerDialog`.
* **Notification permission**
  * `NewTaskScreen.kt:217-226` — requests `POST_NOTIFICATIONS` at runtime if not granted, when a reminder is set.

### 3.3 Task detail / edit screen

* `clients/android/app/src/main/java/com/example/myapplication/ui/task/TaskDetailScreen.kt:74-342` — `TaskDetailScreen` shows a single task.
  * `TaskDetailViewModel.kt:27-32` — `task: StateFlow<Task?>` from `repository.getTaskById(taskId)` with `SharingStarted.WhileSubscribed(5000)`.
  * Edits are immediate: every `OutlinedTextField.onValueChange` calls `viewModel.updateTask(currentTask.copy(...))` (title lines 150-163, category 165-176, description 178-189).
  * `CategorySegmentedRadioRow` lines 344-397 lets the user change quadrant; clicking maps to `isImportant`/`isUrgent`.
  * `MetadataRow` for due date and reminder, with remove actions; past reminders show a warning snackbar but still update (lines 326-332).
  * Status list item (lines 236-257) toggles `isCompleted`.
  * Top app bar actions: pin/unpin, archive if active, unarchive if archived (lines 267-289).
  * Back arrow returns via `navController.popBackStack()` (set in `PriorityLedgerApp.kt:203`).

### 3.4 Sorting and grouping

* `clients/android/app/src/main/java/com/example/myapplication/ui/home/HomeTaskSorter.kt:6-18` — tasks are grouped by `EisenhowerCategory`, then sorted with:
  1. pinned first,
  2. tasks with a `dueDate` before those without,
  3. `dueDate` ascending,
  4. `createdAt` descending.
* `HomeUiState.kt:6-12` — `HomeUiState` carries `activeTasks`, `groupedTasks`, `isLoading`, `error`, `reminderErrors`.

---

## 4. Eisenhower matrix / quadrant UI

* The app does **not** render a literal 2x2 matrix. Instead it uses a **vertical grouped list** (the Priority Ledger) with four sections.
* `clients/android/app/src/main/java/com/example/myapplication/domain/EisenhowerCategory.kt:3-37` — the four categories are derived from `isImportant` and `isUrgent`:
  * `DO_NOW` — important & urgent.
  * `SCHEDULE` — important, not urgent.
  * `DELEGATE_WAITING` — not important, urgent.
  * `ELIMINATE_LATER` — not important, not urgent.
* `clients/android/app/src/main/java/com/example/myapplication/domain/Task.kt:18-22` — the domain `Task` exposes `eisenhowerCategory` via `EisenhowerCategory.from(isImportant, isUrgent)`.
* `PriorityLedgerHomeScreen.kt:790-807` — section order is fixed: Do Now, Schedule, Delegate/Waiting, Eliminate/Later, each with a short description.
* `PriorityLedgerHomeScreen.kt:519-590` — each section header is a colored `OutlinedCard` with an icon, label, description, a keycap showing the keyboard shortcut (e.g. `Q`), and a `Badge` showing the task count.
* `clients/android/app/src/main/java/com/example/myapplication/ui/category/CategoryPresentation.kt:18-43` — category presentation objects map icons and labels:
  * Do Now: `Icons.Filled.PriorityHigh`, "Do Now", shortcut `Q`.
  * Schedule: `Icons.Filled.Event`, shortcut `W`.
  * Delegate/Waiting: `Icons.AutoMirrored.Filled.ForwardToInbox`, shortcut `E`.
  * Eliminate/Later: `Icons.Filled.LowPriority`, shortcut `R`.
* `clients/android/app/src/main/java/com/example/myapplication/ui/theme/LedgerCategoryColors.kt:37-104` — light and dark palettes for every quadrant (red/orange/blue/gray). Each `LedgerCategoryColor` has `accent`, `onAccent`, `container`, `onContainer`, `outline`, `focus`.
  * `LedgerCategoryColors.kt:107-110` — `LocalLedgerCategoryColors` composition local and `EisenhowerCategory.ledgerCategoryColors()` extension.
  * These colors color section headers, selected category chips in the composer, and the category selector in the detail screen.
* **Creating a task in a quadrant**
  * `PriorityLedgerApp.kt:209-234` — the New Task route reads the `defaultCategory` argument and pre-selects it.
  * `PriorityLedgerHomeScreen.kt:261-272` — `defaultNewTaskCategory()` uses (1) the focused task's category, (2) the last jumped section, or (3) `DO_NOW`.
  * `HomeViewModel.kt:112-152` — `addTask` builds a `Task` from the selected `EisenhowerCategory`, setting `isImportant` and `isUrgent` accordingly.
* **Changing a quadrant**
  * `TaskDetailScreen.kt:200-209` — selecting a category in the detail `CategorySegmentedRadioRow` updates the task with the matching `isImportant`/`isUrgent` values.

---

## 5. Data model and persistence

* **Domain model**
  * `clients/android/app/src/main/java/com/example/myapplication/domain/Task.kt:3-23` — `Task(id, title, description, isImportant, isUrgent, dueDate, reminderAt, isCompleted, isArchived, isPinned, category, createdAt, updatedAt)`.
* **Database entity**
  * `clients/android/app/src/main/java/com/example/myapplication/data/local/TaskEntity.kt:6-22` — `@Entity(tableName = "tasks")` with identical fields and `@PrimaryKey(autoGenerate = true) val id: Long = 0`.
* **Mappers**
  * `clients/android/app/src/main/java/com/example/myapplication/data/local/TaskMappers.kt:5-35` — one-to-one field mapping, no transformation.
* **Database and migrations**
  * `clients/android/app/src/main/java/com/example/myapplication/data/local/AppDatabase.kt:8-23` — Room database with `TaskEntity`, version 2, `MIGRATION_1_2` adds the `reminderAt` column.
  * `clients/android/app/src/main/java/com/example/myapplication/data/local/DatabaseProvider.kt:6-21` — singleton `AppDatabase` named `eisenhower_tasks.db`, built with `addMigrations(AppDatabase.MIGRATION_1_2)`.
* **DAO**
  * `clients/android/app/src/main/java/com/example/myapplication/data/local/TaskDao.kt:12-73`:
    * `observeActiveTasks()` — `isArchived = 0 AND isCompleted = 0`, sorted pinned/dueDate/createdAt.
    * `observeCompletedTasks()` — `isCompleted = 1 AND isArchived = 0`, `updatedAt DESC`.
    * `observeArchivedTasks()` — `isArchived = 1`, `updatedAt DESC`.
    * `observeAllTasks()` — all, `updatedAt DESC`.
    * `searchTasks(query)` — active, unarchived, matches title or description.
    * `getTaskById(id)`, `insertTask`, `updateTask`, `unarchiveTask`, `deleteTaskById`.
* **Repository**
  * `clients/android/app/src/main/java/com/example/myapplication/data/LocalTaskRepository.kt:13-79` — `LocalTaskRepository` implements `TaskRepository` (`domain/TaskRepository.kt:5-32`), exposing all flows, and updating `updatedAt` from the clock on every write.
* **Security / backup**
  * The local Room database is **not encrypted at rest in the current code**; `data_extraction_rules.xml` and `backup_rules.xml` explicitly exclude `eisenhower_tasks.db` from backup/transfer as defense-in-depth.
  * `AndroidManifest.xml:8-9` — `android:allowBackup="false"`.
  * `clients/android/app/src/main/res/xml/data_extraction_rules.xml:8-19` and `backup_rules.xml:7-11` exclude the DB, WAL and SHM files.
  * The `README.md:10` and `PLAN-Android-App.md` mention future goals for `Android Keystore + EncryptedSharedPreferences/Tink` and SQLCipher, but no implementation exists in the code.
* **Offline behavior**
  * All data is local Room. No cloud sync or network calls in the repository. `SettingsScreen.kt:155-157` tells the user "All task data is stored locally on this device."

---

## 6. State management

* **ViewModel + StateFlow**
  * `HomeViewModel.kt:41-65` — `uiState: StateFlow<HomeUiState>` built by `combine(repository.observeActiveTasks()/searchTasks(), reminderErrors)`. Emits loading, grouped tasks, errors. `SharingStarted.Eagerly`.
  * `HomeViewModel.kt:36-37` — one-shot UI events via `MutableSharedFlow<HomeUiEvent>` as `events`.
  * `HistoryViewModel.kt:29-41` — `uiState: StateFlow<HistoryUiState>` from `combine(observeCompletedTasks(), observeArchivedTasks())`, `SharingStarted.Eagerly`.
  * `TaskDetailViewModel.kt:27-32` — `task: StateFlow<Task?>` from `repository.getTaskById(taskId)`, `SharingStarted.WhileSubscribed(5000)`.
* **Repository as reactive source**
  * `LocalTaskRepository` returns Kotlin `Flow`s from Room, maps `TaskEntity` to `Task`.
  * UI uses `collectAsState()` to turn flows into Compose state.
* **Configuration change survival**
  * `MainActivity.kt:36-44` — `ViewModelProvider` creates `HomeViewModel`/`HistoryViewModel` scoped to the activity; they survive rotation.
  * Compose `rememberSaveable` preserves lightweight UI state such as `isSearchActive`, `searchQuery`, `selectedTab` in History, and draft fields in the composer.
  * No `SharedPreferences`, no in-memory cache beyond `DatabaseProvider` singleton and `ViewModel`s.
* **Reminder reconciliation**
  * `HomeViewModel.kt:75-110` — `init` starts `reconcileReminders()`. It observes all tasks and scheduled WorkManager IDs and (re)schedules/cancels reminders for active, non-completed, non-archived tasks whose `reminderAt` is in the future. `reminderErrors` is populated when scheduling fails.

---

## 7. Theming and styling

* **Material 3 / dynamic color**
  * `clients/android/app/src/main/java/com/example/myapplication/ui/theme/Theme.kt:102-127` — `MyApplicationTheme`:
    * Dark mode follows the system (`darkTheme: Boolean = isSystemInDarkTheme()`).
    * Dynamic color enabled by default (`dynamicColor: Boolean = true`), using `dynamicLightColorScheme(context)` / `dynamicDarkColorScheme(context)` (Android 14+).
    * Static fallbacks `StaticLightColorScheme` (lines 24-61) and `StaticDarkColorScheme` (lines 63-100) are the Material baseline purple scheme.
    * Category colors are supplied through a `CompositionLocalProvider(LocalLedgerCategoryColors ...)`.
  * `clients/android/app/src/main/res/values/themes.xml:4` — Android manifest theme is `Theme.DeviceDefault.DayNight` with `windowActionBar` false.
* **Category color tokens**
  * `ui/theme/LedgerCategoryColors.kt:37-104` — four quadrant palettes (red `DO_NOW`, amber `SCHEDULE`, blue `DELEGATE_WAITING`, gray `ELIMINATE_LATER`) in light and dark modes.
* **Typography / layout constants**
  * `clients/android/app/src/main/java/com/example/myapplication/ui/components/ScreenTopBar.kt:15` — `TopBarHeight = 56.dp`.
  * `ScreenTopBar.kt:18-60` — custom top bar with a center title and left/right slots.
  * Home: `LedgerCardOuterGutter = 16.dp`, `LedgerLeadingRailWidth = 48.dp`, `LedgerLeadingRailTextGap = 8.dp`.
  * Composer: `FormMaxWidth = 720.dp`, `FormHorizontalGutter = 24.dp`.
  * `WindowInsets.navigationBars` used in `Scaffold`s to support edge-to-edge.
* **Icons**
  * Compose Material `Icons.Filled.*` for all UI icons: `Add`, `Search`, `Menu`, `Archive`, `PushPin`, `Warning`, `History`, `Settings`, `Keyboard`, `Restore`, `Notifications`, `Event`, `PriorityHigh`, `ForwardToInbox`, `LowPriority`, `ArrowBack`, `ArrowForward`, `HelpOutline`, `MoreVert`, `Close`, `Check`, `Unarchive`, `Delete`, `Info`, `Storage`.
  * `clients/android/app/src/main/res/drawable/*.xml` and `mipmap-anydpi-v26/*.xml` provide launcher icons only.

---

## 8. Lifecycle behavior

* **Activity lifecycle**
  * `MainActivity.kt:28-67` — `onCreate` is the only lifecycle method; it wires the DB, notification channel, ViewModels and Compose.
  * `onNewIntent` is used to route notification taps into `initialTaskId`.
* **Foreground / background / reminders**
  * `TaskReminderNotifications.kt:13-22` — creates the `task_reminders` notification channel on first run and in the worker.
  * `WorkManagerTaskReminderScheduler.kt:19-66` — schedules/cancels `OneTimeWorkRequestBuilder<TaskReminderWorker>` using unique work names (`task-reminder:$taskId`) and tags.
  * `TaskReminderWorker.kt:21-98` — on execution, re-fetches the task from the DB; posts a notification only if the task is still active, uncompleted, unarchived, the reminder time has passed, and notification permission is granted. It opens `MainActivity` with the task ID.
  * `HistoryViewModel.kt:43-77` — restoring completed or archived tasks reschedules a future reminder.
* **Settings permission refresh**
  * `SettingsScreen.kt:69-80` — `DisposableEffect` adds a `LifecycleEventObserver` that re-checks the `POST_NOTIFICATIONS` permission on `ON_RESUME`.
* **Data lifecycle**
  * All writes go to Room; the database is the single source of truth.
  * `allowBackup="false"` + `data_extraction_rules` + `backup_rules` prevent the unencrypted DB from leaving the device.
* **No unlock/vault, no export/import UI, no cloud sync, no recovery UI.**
  * `data_extraction_rules.xml:2-6` explicitly calls the DB unencrypted.
  * `SettingsScreen.kt` only links to system notification settings.
  * `assets/vectors.json` and `VectorRunner.kt` contain protocol test fixtures; they are not user-facing.

---

## 9. Unique UX patterns

* **No swipe, drag, or long-press actions.** Primary row actions are visible icons (checkbox, archive) and tap to open detail.
* **Snackbars**
  * Home: `Task completed` / `Task archived` with `Undo`.
  * History: `Task restored to active tasks` / `Task restored from archive`.
  * Composer: past-reminder warning, save-failure with `Retry`, permission-denied notice.
  * Detail: operation-failure and past-reminder warnings.
* **Dialogs**
  * `NewTaskScreen.kt:648-672` — `DiscardConfirmationDialog` (discard/keep editing).
  * `NewTaskScreen.kt:674-696` — `ComposerShortcutHelpDialog`.
  * `TaskFormComponents.kt:70-169` — `DueDatePickerDialog`, `ReminderDatePickerDialog`, `ReminderTimePickerDialog`.
* **Animations**
  * Only scroll animations: `PriorityLedgerHomeScreen.kt:235` and `254` use `listState.animateScrollToItem(index)` for focus and category jumps.
  * No haptic feedback or complex motion found.
* **No home-screen widget found.**
* **Notifications**
  * One notification channel `task_reminders` for reminder Worker notifications.
  * Notification ID is `taskId.hashCode()`.
* **Accessibility / testing attributes**
  * `testTag`s in `PriorityLedgerApp.kt:278`, `PriorityLedgerHomeScreen.kt:395`, `NewTaskScreen.kt:290,335,515,600`.
  * Extensive `contentDescription`s, `heading()`, `stateDescription`, `CustomAccessibilityAction`s on task rows and category selectors.

---

## 10. Key files for rebuilding in `clients/pwa-svelte`

| Concern | File(s) |
|--------|--------|
| Build / manifest | `app/build.gradle.kts`, `app/src/main/AndroidManifest.xml` |
| Entry point | `app/src/main/java/com/example/myapplication/MainActivity.kt` |
| Navigation / routing | `app/src/main/java/com/example/myapplication/ui/navigation/PriorityLedgerApp.kt` |
| Home screen | `app/src/main/java/com/example/myapplication/ui/home/PriorityLedgerHomeScreen.kt`, `HomeViewModel.kt`, `HomeUiState.kt`, `HomeTaskSorter.kt` |
| New task | `app/src/main/java/com/example/myapplication/ui/task/NewTaskScreen.kt`, `TaskFormComponents.kt` |
| Task detail | `app/src/main/java/com/example/myapplication/ui/task/TaskDetailScreen.kt`, `TaskDetailViewModel.kt` |
| History | `app/src/main/java/com/example/myapplication/ui/history/HistoryScreen.kt`, `HistoryViewModel.kt` |
| Settings | `app/src/main/java/com/example/myapplication/ui/settings/SettingsScreen.kt` |
| Data / persistence | `data/local/TaskEntity.kt`, `TaskDao.kt`, `AppDatabase.kt`, `DatabaseProvider.kt`, `TaskMappers.kt`, `data/LocalTaskRepository.kt`, `domain/Task.kt`, `domain/TaskRepository.kt` |
| Category model | `domain/EisenhowerCategory.kt`, `ui/category/CategoryPresentation.kt`, `ui/theme/LedgerCategoryColors.kt` |
| Theming | `ui/theme/Theme.kt`, `res/values/themes.xml`, `ui/components/ScreenTopBar.kt` |
| Date formatting | `app/src/main/java/com/example/myapplication/ui/util/DateTimeUtils.kt` |
| Reminders | `data/reminder/TaskReminderWorker.kt`, `TaskReminderNotifications.kt`, `WorkManagerTaskReminderScheduler.kt`, `domain/TaskReminderScheduler.kt` |
| Strings | `app/src/main/res/values/strings.xml` |
