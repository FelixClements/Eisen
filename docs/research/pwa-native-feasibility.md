# PWA Native-Feel Feasibility for Eisen

## Summary

The Eisen PWA can get a much more native feel on Android and iOS **without** becoming "mostly online" and **without** changing Cloudflare D1 to store one plaintext row per task. Install, standalone display, splash screen, home-screen icon, shortcuts, share target, file handling, badging, and offline-first use all work as a local-first, end-to-end-encrypted (E2EE) app. The few capabilities that genuinely need a network or server trigger — Web Push notifications, one-off background sync, and periodic background sync — can be implemented so the server only sees device metadata or a wake-up timestamp, never the plaintext task content.

Storing one plaintext task row per D1 record would not improve the native feel in any material way; it would break Eisen’s stated “server is an opaque blob store” rule and expose user data. If exact-time local alarms, deep bi/OS integration, or guaranteed background sync on iOS are required, the right escape hatch is a native wrapper (Trusted Web Activity / Capacitor / Kotlin) rather than moving the whole product online-first.

## 1. What currently makes a PWA feel “native,” and which require being online

| Native-feel feature | Online required? | Notes / primary source |
|---------------------|------------------|------------------------|
| **Home screen icon + app launcher entry** | No | The Web App Manifest (`icons`, `short_name`) and WebAPK install flow on Android create a launcher icon and Settings entry; Chrome mints an APK behind the scenes. `[1]` `[2]` |
| **Standalone / fullscreen display, splash, theme colors** | No | Manifest `display: "standalone"`, `theme_color`, and `background_color` control the window chrome, status bar, and auto-generated splash screen. `[3]` `[4]` `[5]` |
| **App shortcuts** | No | Manifest `shortcuts` list deep links; runs inside the installed app, purely local after launch. `[6]` |
| **Share target** | Only at receive time if the POST endpoint is network-backed | The OS launches the PWA with shared data via `share_target`; the app can write locally and sync later. `[7]` `[8]` |
| **File handlers** | No for opening; sync later if desired | Desktop Chromium only so far; declares MIME types the PWA can open. `[9]` `[10]` |
| **Badging (icon dot/count)** | No on iOS; notification-driven dot on Android | Badging API is available for installed PWAs on iOS 16.4+ and desktop Chrome; on Android the dot is shown automatically when a notification is pending, not via a badge API. `[11]` `[12]` `[13]` |
| **Navigation gestures / back button** | No | Standalone PWAs hide browser chrome, but the OS back gesture is fixed by the platform; there is no web API to override it. `[3]` `[4]` |
| **Offline use and local storage** | No | Service worker + Cache API + IndexedDB / OPFS let the app run with no network. `[14]` `[15]` `[16]` |
| **Background sync (one-off)** | Only when the device comes back online | Defers a task (e.g., a pending sync) until the browser has a stable network connection. Not supported on iOS/Safari. `[17]` `[18]` |
| **Periodic background sync** | Only when it fires (network usually) | Chromium-only; not on iOS. It is a best-effort periodic wake, not an exact alarm. `[19]` `[20]` `[21]` |
| **Web Push notifications** | Yes, a server must send the push | The push service wakes the service worker; the payload itself is encrypted end-to-end between server and browser (RFC 8291). `[22]` `[23]` `[24]` |

Only **Web Push**, **background sync**, and **periodic background sync** intrinsically need the network — and then only for delivery/triggering, not for the app logic to be online-first.

## 2. Native-feel features that are impossible or severely limited in a purely offline PWA

| Capability | Offline-only status | Standard / practical fix |
|------------|--------------------|--------------------------|
| **Exact-time local alarms / reminders while the PWA is closed** | Not reliably possible | No shipped web API. The Notification Triggers API was dropped by Chrome after its origin trial. `[25]` `[26]` The practical choices are: (a) Web Push wake-up from a server, (b) periodic background sync as a best-effort refresh, or (c) a native wrapper with platform alarms. |
| **Push notifications** | Requires a server to trigger the push | Use Web Push with an encrypted payload (RFC 8291). Eisen can use a server as a privacy-preserving wake-up clock rather than as a reader of task content. `[22]` `[24]` |
| **One-off Background Sync on iOS** | Not supported | Safari does not fire the `sync` event. A workbox-style queue that replays on service-worker restart is the usual fallback. `[17]` `[18]` `[27]` |
| **Periodic Background Sync on iOS** | Not supported | Chromium-only. Not a solution for exact-time reminders. `[19]` `[20]` `[21]` |
| **File handling on mobile** | Effectively unsupported | Currently desktop Chromium. Not a blocker for a mobile-first todo app. `[9]` |
| **Numeric badge on Android** | Not settable via API | Android shows a dot when a notification exists, not a count from `setAppBadge()`. `[11]` `[13]` |
| **Deep OS biometrics / Face ID / passkey unlock** | Partial | WebAuthn is available but is not the same as OS-level face/biometric unlock. Native wrappers add the latter. `[28]` (internal spec) |
| **Override system back gesture** | Not possible | Standalone `display` modes remove browser chrome but the OS controls gestures. `[3]` `[4]` |

The standard fixes are **not** “go online-first.” They are: use a server as a wake-up clock (Web Push), use background sync for deferred uploads, and use a native wrapper if the platform gap is non-negotiable.

## 3. Is a D1 row per plaintext task necessary for native feel?

No.

Eisen already stores one encrypted record per task in `vault_records` (`record_id`, `owner_id`, `encrypted_blob`, `modified_at`, `sync_version`, `deleted`). `[29]` `[30]` The local Dexie/IndexedDB keeps the plaintext for the UI; the ciphertext is what leaves the device. `[29]`

Moving to a D1 schema with one plaintext row per task would:

- Expose task titles, descriptions, due dates, reminder times, completion state, and task count to the server and any database compromise.
- Make it impossible to keep the cloud as an “opaque blob store,” violating the architecture rule that **plaintext task content never leaves the device**. `[31]`
- Provide almost no native-feel benefit: none of the manifest/install/display features, shortcuts, offline use, or local badging need server-side plaintext.

For the few features that need a server trigger, the server only needs an opaque wake-up record. For example, the existing `android-notifications.md` research proposes the client send `{ deviceId, wakeAt, nonce }` and the server push a generic wake message at `wakeAt`; the worker then reads local `reminderAt` values and shows the actual reminder. `[30]`

## 4. What Eisen currently stores locally vs. in Cloudflare D1

Current `pwa-svelte` implementation (`docs/research/local-vs-d1-data.md`):

| Data | Local (browser) | Cloud (D1 / R2 / KV) |
|------|-----------------|------------------------|
| Tasks (plaintext fields) | Dexie/IndexedDB `tasks` | `vault_records.encrypted_blob` only; opaque to the server. `[29]` |
| Master key | Non-extractable `CryptoKey` in `sessions` (optional “Keep me signed in”) or in memory only. | Never sent or stored. `[29]` `[31]` |
| Account / device metadata | IndexedDB `accounts`, `deviceState` | D1 `accounts`, `devices` for routing/pairing. `[29]` |
| Recovery package | Exported user-held file | R2/D1 as ciphertext only. `[29]` |
| Pairing codes | None (ephemeral in memory) | Workers KV (transient, TTL). `[29]` |

The architecture is already well-suited for a native-feel offline-first PWA: the UI, search, sort, and filtering happen locally against decrypted data; the cloud is only a sync/backup relay. `[31]` `[32]`

## 5. The specific technical blocker: notifications, background sync, and auto-unlock without server plaintext

### Notifications

Yes, a PWA can show notifications without the server knowing task content. The flow:

1. The client computes the next `reminderAt` from its local `tasks` table and posts a tiny opaque wake request: `{ deviceId, wakeAt, nonce }`. `[30]`
2. The server stores only the wake timestamp per device.
3. At `wakeAt` the server sends a generic push (or an empty push) to the device. Web Push payloads are encrypted per subscription using the browser’s push-subscription keys (RFC 8291), so the push service also cannot read the message. `[22]` `[24]`
4. The service worker wakes, retrieves the persisted session key from IndexedDB, unwraps the master key, reads local `reminderAt` values, and shows the notification. `[30]`

Because the notification content is assembled locally, the server never learns *which* task or *what* the reminder is.

### Background sync

Yes, sync can happen without exposing plaintext. When the app is offline, changes are queued locally (e.g., in a Dexie/IndexedDB outbox). When the browser fires the `sync` event, the service worker uploads only the already-encrypted `vault_records` to `/api/sync`. `[17]` `[18]` The server only sees ciphertext. On iOS this one-off sync is not available, so a fallback queue that replays when the service worker starts is the pragmatic alternative. `[27]`

### Auto-unlock

Yes, this can be purely local. The existing `pwa-keep-me-signed-in-spec.md` stores a non-extractable session `CryptoKey` plus the wrapped master key in IndexedDB. `[28]` The PWA can load on next launch, unwrap the key, and skip the passphrase prompt — no server call and no plaintext exposure. The main security caveat is that anyone with the device can open the app while the session is persisted.

## 6. Alternatives to “mostly online” / “row per task”

If the PWA alone is not enough, the alternatives (in order of invasiveness) are:

1. **Service worker patterns** — precache the app shell, use stale-while-revalidate, and queue outbound changes in IndexedDB until online. This is the standard local-first PWA pattern. `[14]` `[15]`
2. **Background Sync** — for reliable “send when back online” behavior on Android/desktop. `[17]` `[18]`
3. **Periodic Background Sync** — for best-effort daily/periodic content refreshes on Chromium. Not exact alarms and not on iOS. `[19]` `[20]`
4. **Web Push as a wake-up clock** — server sends an empty/encrypted push; service worker creates the real notification from local data. `[22]` `[30]`
5. **Web Locks + OPFS / IndexedDB** — for atomic local storage and sync coordination. OPFS gives high-performance, private origin storage and works in workers. `[15]` `[16]`
6. **Trusted Web Activity (TWA)** — package the PWA as an installable Play Store APK. It uses the same web runtime but gets an APK icon, app settings, and intent filters; it does not require rewriting the app or exposing plaintext. `[33]` `[34]`
7. **Capacitor / Ionic / Kotlin wrapper** — add native plugins for exact alarms, biometrics, or local notifications while keeping the web UI. This is heavier but is the real fix for OS-level features the web platform cannot provide. `[35]`

## Recommendation

**Question (a): Do we need to become mostly online?**

No. Keep the app local-first and offline-by-default. Add network touchpoints only where the web platform absolutely requires them: a tiny opaque wake-up record for Web Push, and background sync for deferred uploads. The core create/read/update/delete and search should stay local.

**Question (b): Do we need a D1 row per plaintext task?**

No. The existing `vault_records` table already holds one encrypted record per task. Storing plaintext rows would not unlock any native-feel feature that cannot already be achieved locally or via a privacy-preserving push wake-up; it would break the E2EE model. If exact-time local alarms or deeper OS integration are mandatory, evaluate a TWA/Capacitor wrapper instead of changing the data model.

---

## Sources

`[1]` Web on Android — WebAPKs on Android (web.dev): https://web.dev/articles/webapks  
`[2]` web.dev — PWA Installation / WebAPKs: https://web.dev/learn/pwa/installation  
`[3]` MDN — Web App Manifest / display modes: https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest  
`[4]` W3C — Web Application Manifest (display member): https://www.w3.org/TR/appmanifest/  
`[5]` MDN — background_color manifest reference: https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/background_color  
`[6]` MDN — Expose common actions as shortcuts: https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/How_to/Expose_common_actions_as_shortcuts  
`[7]` MDN — share_target manifest reference: https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/share_target  
`[8]` Chrome for Developers — Receiving shared data with the Web Share Target API: https://developer.chrome.com/docs/capabilities/web-apis/web-share-target  
`[9]` MDN — file_handlers manifest reference: https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Manifest/Reference/file_handlers  
`[10]` Chrome for Developers — File Handling API: https://developer.chrome.com/docs/capabilities/web-apis/file-handling  
`[11]` MDN — Badging API: https://developer.mozilla.org/en-US/docs/Web/API/Badging_API  
`[12]` WebKit — Badging for Home Screen Web Apps: https://webkit.org/blog/14112/badging-for-home-screen-web-apps/  
`[13]` Chrome for Developers — App Badging API: https://developer.chrome.com/docs/capabilities/web-apis/badging-api  
`[14]` MDN — Offline and background operation for PWAs: https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Offline_and_background_operation  
`[15]` MDN — Origin Private File System: https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system  
`[16]` web.dev — Storage for the web (OPFS / IndexedDB): https://web.dev/articles/storage-for-the-web  
`[17]` MDN — Background Synchronization API: https://developer.mozilla.org/en-US/docs/Web/API/Background_Synchronization_API  
`[18]` WICG — Background Sync specification: https://wicg.github.io/background-sync/  
`[19]` MDN — Web Periodic Background Synchronization API: https://developer.mozilla.org/en-US/docs/Web/API/Web_Periodic_Background_Synchronization_API  
`[20]` WICG — Periodic Background Sync specification: https://wicg.github.io/periodic-background-sync/  
`[21]` Can I use — Periodic Background Sync: https://caniuse.com/wf-periodic-background-sync  
`[22]` W3C — Push API: https://w3c.github.io/push-api/  
`[23]` MDN — Push API: https://developer.mozilla.org/en-US/docs/Web/API/Push_API  
`[24]` RFC 8291 — Message Encryption for Web Push: https://datatracker.ietf.org/doc/html/rfc8291/  
`[25]` Chrome for Developers — Notification Triggers API (no longer pursued): https://developer.chrome.com/docs/web-platform/notification-triggers  
`[26]` Stack Overflow / Chromium issue — Notification Triggers dropped: https://stackoverflow.com/questions/70611006 (linked to Chromium tracker)  
`[27]` GitHub — Workbox background sync iOS fallback discussion: https://github.com/GoogleChrome/workbox/issues/2516  
`[28]` Eisen internal — `docs/pwa-keep-me-signed-in-spec.md`  
`[29]` Eisen internal — `docs/research/local-vs-d1-data.md`  
`[30]` Eisen internal — `docs/research/android-notifications.md`  
`[31]` Eisen internal — `docs/architecture-data-flow.md`  
`[32]` Eisen internal — `docs/research/pwa-e2ee-cloudflare-blueprint.md`  
`[33]` Chrome for Developers — Trusted Web Activity overview: https://developer.chrome.com/docs/android/trusted-web-activity  
`[34]` web.dev — Using a PWA in your Android app (TWA): https://web.dev/articles/using-a-pwa-in-your-android-app  
`[35]` Capacitor — Getting started: https://capacitorjs.com/docs/getting-started

## Note on a missing requested file

The prompt requested `docs/research/pwa-native-like-auth.md`; no such file exists in this repository, so it was not used as a source.
