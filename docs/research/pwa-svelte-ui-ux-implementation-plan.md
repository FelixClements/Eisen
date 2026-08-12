# Implementation Plan: Rebuild Android Priority Ledger in `clients/pwa-svelte`

This document translates the Android client UI/UX research in `android-app-ui-ux.md` into a concrete implementation plan for the SvelteKit PWA.

## Goal

Replace the current one-page todo demo in `clients/pwa-svelte` with a full SvelteKit PWA that matches the Android client: a left-drawer navigation, vertical grouped Eisenhower ledger, full-screen composer, detail editor, history, settings, and the same category colors and sorting behavior. The E2EE Cloudflare sync already in place should continue to work underneath.

---

## Phase 1 — Routing and App Shell

1. Expand `src/routes/+layout.svelte` into a `ModalNavigationDrawer` equivalent.
2. Add routes:
   - `/` — Home (Priority Ledger)
   - `/new-task` and `/new-task/[category]` — New task composer
   - `/task/[taskId]` — Task detail
   - `/history` — History
   - `/settings` — Settings
   - `/keyboard-shortcuts` — Shortcuts help
3. Drawer destinations: Home, History, Settings, Keyboard Shortcuts.
4. Keep the PWA manifest; refine `theme-color` to match the active category.

---

## Phase 2 — Data Model

1. Rename the `todos` Dexie table to `tasks` and expand the schema to match the Android `Task` fields:
   - `id`, `title`, `description`, `isImportant`, `isUrgent`
   - `dueDate`, `reminderAt`, `isCompleted`, `isArchived`, `isPinned`
   - `category` (free-form), `createdAt`, `updatedAt`
   - `sync_version`, `deleted`, `encrypted_blob` for the existing sync layer
2. Add Dexie indexes for `isArchived`, `isCompleted`, `updatedAt`, `dueDate`.
3. Rewrite `src/lib/db.ts` helpers and live queries for active, completed, archived, and search.

---

## Phase 3 — Home / Priority Ledger

1. Build a vertical grouped list with four fixed sections: **Do Now**, **Schedule**, **Delegate/Waiting**, **Eliminate/Later**.
2. Each section gets a colored header with icon, label, count badge, and shortcut keycap.
3. Use the same color tokens from `LedgerCategoryColors.kt` (red, amber, blue, gray) as CSS variables.
4. Task rows are card-style with checkbox, title, status line, pin/reminder icons, archive button.
5. Add the top search overlay and the "[A] Add" floating action button.
6. Add undo snackbars for complete / archive.
7. Keyboard shortcuts are optional for PWA; `A` for add and `Q/W/E/R` for section jumps are nice-to-have.

---

## Phase 4 — New Task Composer

1. Full-screen `/new-task` route, not a dialog.
2. Title input autofocus.
3. 4-cell quadrant selector with the same colored cells.
4. Notes textarea, free-form category input, due date and reminder pickers (`<input type="date">` / `datetime-local`).
5. Discard confirmation if the user leaves with a draft.
6. Save validates title and warns on a past reminder.

---

## Phase 5 — Task Detail

1. `/task/[taskId]` with immediate-edit fields.
2. Toggle completion, pin, and archive.
3. Quadrant selector updates `isImportant` / `isUrgent`.
4. Due/reminder rows with a remove action.

---

## Phase 6 — History

1. `/history` with tabs for Completed and Archived.
2. Sorted by `updatedAt` descending.
3. Restore action with snackbar.

---

## Phase 7 — Theming and Settings

1. Add CSS variables for the four quadrant palettes in light/dark.
2. Implement a dark-mode toggle (`prefers-color-scheme` + saved class).
3. `/settings` page with notification permission status and local-only messaging.
4. Style the app to look like a Material 3 card-based UI.

---

## Phase 8 — Reminders

1. Request `Notification.permission` in settings.
2. Show browser `Notification` from a `setTimeout` while the app is running.
3. For reliable background reminders, later add Web Push through Cloudflare Workers; for the prototype, a best-effort client-side notification is acceptable.

---

## Phase 9 — Sync Integration

1. Keep `crypto.ts` and `sync.ts`.
2. Encrypt the expanded task JSON before sending.
3. Decrypt and apply LWW server records.
4. Trigger sync on unlock and after edits (debounced).

---

## Phase 10 — Testing and Polish

1. Add Playwright tests for navigation, create, complete, archive, search, detail, and sync.
2. Test mobile responsiveness and PWA installability.
3. Commit and push.

---

## Mapping from Android to Svelte

| Android | Svelte/PWA |
|---|---|
| `NavHost` + `ModalNavigationDrawer` | `+layout.svelte` + SvelteKit routes |
| `LazyColumn` sections | grouped `<ul>`/`<li>` |
| `ViewModel` + `StateFlow` | Svelte 5 runes + Dexie `liveQuery` |
| `rememberSaveable` | `localStorage` for drafts/UI state |
| `OutlinedCard` | CSS card component |
| `MaterialTheme` colors | CSS custom properties |
| `WorkManager` | `Notification` + `setTimeout` / later Web Push |
| `POST_NOTIFICATIONS` | `Notification.permission` |
| Room DAO | Dexie `where`/`and`/`sortBy` |

---

## Open Questions

1. **Matrix style:** The Android app uses a vertical grouped ledger, not a 2×2 grid. Do you want the same, or a true 2×2 matrix? answer: vertical grouped ledger
2. **Keyboard shortcuts:** Do you want `A`, `Q/W/E/R`, `?` etc. for desktop, or skip them for mobile? answer: skip for mobile answer: skip for now
3. **Scope first:** Should I start with the full plan, or just home + composer + detail and do history/settings later? answer: start with the full plan Answer: start with the full plan
