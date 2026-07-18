# Priority Ledger Roadmap

Priority Ledger is an offline-first Android task app that turns the Eisenhower
Matrix into a keyboard-friendly, grouped task list for a physical-keyboard phone.

## Current status

The following work is implemented now:

- Priority Ledger home screen with four Eisenhower priority sections.
- Full-screen **New Task** composer; the old inline Quick Add design is retired.
- Task notes, due-date selection and local-time date-only behavior.
- Persisted reminder times with battery-friendly WorkManager notifications.
- Local Room storage, repository/ViewModel data flow, and existing Room migration
  support for reminders.
- Keyboard controls for navigating the ledger and creating or acting on tasks.
- Android-14-only Material You dynamic color using system light/dark color schemes.

## NEXT UP

- [ ] Build a task detail/edit screen.
  - Open it from a ledger task and show the complete task information.
  - Edit title, notes, Eisenhower category, due date, and reminder.
  - Validate and persist edits through the ViewModel and repository.
  - Safely cancel the old reminder and schedule the new reminder when its time
    changes; cancel it when the reminder is removed.
  - Keep reminder cancellation correct for completion, archive, and deletion.
  - Test editing, removing, and persisting task fields after an app restart.

- [ ] Add a completed/archive screen.
  - Show completed and archived tasks in clearly separated states or tabs.
  - Restore a completed task to active work and restore an archived task safely.
  - Require confirmation before permanent deletion.
  - Keep restored tasks in their stored Eisenhower category and preserve details.
  - Test complete, archive, restore, and delete flows.

- [ ] Surface reminder status in the Priority Ledger and task detail screen.
  - Show whether a reminder is set and its scheduled time where space permits.
  - Make overdue, due-date, and reminder information easy to distinguish.
  - Cover scheduling and cancellation behavior with tests, including near-future
    reminders on a device or emulator.

- [ ] Verify keyboard-only and touch accessibility.
  - Check focus order, visible focus, and all primary flows without touch input.
  - Confirm text fields accept normal typing without ledger shortcuts firing.
  - Add labels/content descriptions and verify text contrast and touch targets.
  - Test destructive-action confirmations and Back behavior with both input modes.

## Current feature specifications

### Priority Ledger

- The home screen is a vertical grouped list, not a 2x2 matrix.
- Active tasks are grouped as **Do Now**, **Schedule**, **Delegate / Waiting**,
  and **Eliminate / Later**.
- Each section has a distinct accent, a visible key hint, task count, and a
  collapsible grouped-list presentation.
- A task row shows completion state, title, category accent, and due/overdue
  information when available. Focused rows have an obvious visual focus state.
- The ledger is designed for the near-square 1080 x 1200 display and remains
  useful with a physical keyboard as the primary input device.

### Keyboard controls

The app must not depend on arrow keys, Tab, Ctrl, or `/`, because those are not
normal physical keys on the target phone.

| Key | Action |
|---|---|
| `J` / `K` | Move ledger focus down / up |
| `Q` / `W` / `E` / `R` | Jump to Do Now / Schedule / Delegate / Eliminate |
| `Space` | Complete or uncomplete the focused task |
| `A` | Open the New Task composer |
| `Back` | Close, cancel, or navigate back |
| `Backspace` | Archive the focused task |
| Composer `Enter` | Save the new task |
| Composer `Alt + Q/W/E/R` | Select composer category |

- Destructive shortcuts must not run while a text field is receiving input.
- Section headers expose the category hints: `[Q]`, `[W]`, `[E]`, and `[R]`.
- Planned keyboard work: `H/L` section controls, `Enter` task detail, `M` move
  palette, `D` due-date editing, and `Shift + Backspace` deletion confirmation.

### New Task composer

- The composer is a dedicated full-screen route, not an inline card, dialog, or
  bottom sheet.
- Opening it focuses the title field immediately.
- `A` from the ledger and the touch **[A] Add** control open the composer.
- The default category is the focused task's category, then the last jumped
  section, then **Do Now** when no context exists.
- Four large category chips use a 2x2 layout; `Alt + Q/W/E/R` changes category
  without interfering with normal title typing.
- The composer supports title, notes, due date, and reminder fields.
- Saving trims the title and rejects empty or whitespace-only titles with
  **Task title is required.**
- `Enter` saves with the selected category. `Back` cancels; a non-empty draft
  requires a discard confirmation.
- Save returns to the ledger.

### Due dates

- A due date is optional and can be set or removed in the composer.
- A date-only due date is interpreted in the device's local time zone.
- Ledger rows show the due date when present and visually identify overdue tasks.
- Editing a due date belongs on the upcoming task detail/edit screen.

### Reminders

- A reminder time is optional and stored independently of the due date.
- Selecting a reminder requests notification permission on Android 13+ when
  needed and uses the task-reminder notification channel.
- WorkManager schedules reminder work and is intentionally battery-friendly
  rather than exact to the minute.
- Completing or archiving a task cancels its scheduled reminder.
- Editing, deleting, or removing a reminder must cancel or replace scheduled
  work safely; this is part of the next detail/edit-screen work.

## Later roadmap

- [ ] Add recurring tasks with structured rules for daily, weekdays, weekly,
  monthly, and custom intervals.
  - Create the next occurrence safely when a recurring task is completed.
  - Preserve title, notes, category, due-date pattern, reminder rule, and tags.
  - Prevent duplicate occurrences and test month ends, daylight saving, and time
    zones.

- [ ] Add tags with a `tags` table and a many-to-many task/tag cross-reference.
  - Support creating, renaming, deleting, assigning, and removing tags.
  - Define what happens to task/tag links when a tag is deleted.

- [ ] Add search, filters, and the move palette.
  - Search task titles and notes.
  - Filter active, completed, and archived tasks.
  - Provide the compact `M` move palette using `Q/W/E/R` choices.

- [ ] Add calendar integration and a home-screen widget.

- [ ] Add optional cloud sync only after offline behavior and conflict handling
  are clearly defined.

## Data and architecture

### Task model

The persisted task model includes the fields needed by the current UI:

```kotlin
data class Task(
    val id: Long,
    val title: String,
    val description: String?,
    val isImportant: Boolean,
    val isUrgent: Boolean,
    val dueDate: Long?,
    val reminderAt: Long?,
    val isCompleted: Boolean,
    val isArchived: Boolean,
    val isPinned: Boolean,
    val createdAt: Long,
    val updatedAt: Long,
)
```

- The Eisenhower category is derived from `isImportant` and `isUrgent`.
- `description` stores task notes; `dueDate` and `reminderAt` are nullable epoch
  timestamps.
- Future tags require tag and cross-reference models. Future recurrence requires
  a structured recurrence field rather than display text alone.

### Data flow

- Room is the local, offline source of truth.
- DAO queries feed the local repository; the repository maps database entities to
  domain `Task` values.
- ViewModels expose state to Compose screens using coroutines and Flow.
- Compose screens send user events to ViewModels; ViewModels update the repository
  and refresh UI from persisted data.
- Completing or archiving a task cancels its reminder work. The upcoming detail/edit
  screen must add equivalent handling for changed, removed, and deleted reminders.

### Database changes

- Every new persisted column or table must increment the Room database version.
- Supply tested Room migrations that preserve existing tasks; never rely on
  destructive migration for user task data.
- Update entities, mappers, DAO queries, repository methods, ViewModels, and
  migration tests together for every schema change.

## Verification

Run the debug build and unit tests with Android Studio's bundled JDK:

```bash
JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" ./gradlew :app:assembleDebug :app:testDebugUnitTest
```

For each changed feature, also manually open the affected screen and verify the
keyboard-only flow as well as the touch flow. Use a device or emulator for
notification-permission and reminder checks.

## Decisions and limitations

- `minSdk` is 34. Material You dynamic color therefore targets Android 14+ only.
- Dynamic color uses `dynamicLightColorScheme(context)` and
  `dynamicDarkColorScheme(context)` with no SDK checks or static fallbacks.
- WorkManager reminders are battery-friendly and inexact; they are not a promise
  of exact alarm delivery at the selected minute.
- If notification permission is denied when a reminder is due, that reminder does
  not fire late if permission is granted afterward.
- Permanent deletion always needs confirmation; archive is the recoverable normal
  removal path.
- The app is offline-first. Cloud sync is later work, not a current capability.
