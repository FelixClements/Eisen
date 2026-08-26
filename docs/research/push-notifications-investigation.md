# Push Notifications Investigation — Eisen

**Date:** 2026-08-25  
**Scope:** Why Web Push reminders do not work in the Eisen SvelteKit PWA  
**Method:** Source-code trace, schema review, CI/deploy config, GitHub issue search, primary API/spec references

---

## Executive Summary

Push notifications in Eisen are **architecturally implemented but not wired end-to-end**. The most likely reasons they do not work in practice, ranked:

1. **No scheduler calls `/api/push/cron`** — the server endpoint that dispatches due wakes exists, but the repository contains **zero cron triggers, Workers scheduled events, or external schedulers** to invoke it. Wake schedules are written to D1 and never processed unless something manually hits the cron route.

2. **VAPID keys are unset or mismatched** — the client requires `VITE_VAPID_PUBLIC_KEY` at build time; the server requires `VAPID_PUBLIC_KEY` + `VAPID_PRIVATE_KEY` at runtime. Both are empty/comment-only in repo config. When keys are missing, the code **silently no-ops** (`noopPushDispatch`, `subscribe()` returning `null`).

3. **`web-push` is a poor fit for Cloudflare Workers/Pages** — the server uses the Node.js `web-push` package, which relies on `node:crypto` and `node:https`. Eisen enables `nodejs_compat` but uses `compatibility_date = "2025-07-18"` without `enable_nodejs_http_modules`. Even with shims, this library is [not officially supported on Workers](https://github.com/web-push-libs/web-push/issues/718) and send failures are swallowed.

Secondary issues: manual opt-in only (Settings button), no user-visible error feedback, service worker cannot read IndexedDB directly (depends on an open browser tab), wake rows accumulate without deduplication, and iOS requires an installed PWA (iOS 16.4+).

**No GitHub issues** in this repository mention push notifications, VAPID, reminders, or wake-clock failures.

---

## Architecture Overview

Eisen uses a **privacy-preserving "wake-clock"** pattern (documented in `CONTEXT.md`): the server stores only `{ deviceId, wakeAt, nonce }` — never task titles or content. When a wake fires, the server sends a minimal Web Push payload `{ type: "wake", userId }`. The service worker then asks an open client tab for due reminders from local encrypted storage and displays notifications.

```
┌─────────────┐    POST /api/push/subscribe     ┌──────────────────┐
│   Browser   │ ──────────────────────────────► │  Cloudflare      │
│  (client)   │    POST /api/push/schedule       │  Pages + D1      │
│             │ ◄────────────────────────────── │                  │
└──────┬──────┘                                  └────────┬─────────┘
       │                                                  │
       │  Service Worker (push event)                     │  GET /api/push/cron
       │  ◄── Web Push (VAPID) ───────────────────────────┤  (nothing calls this)
       │                                                  │
       ▼                                                  ▼
  showNotification()                              web-push.sendNotification()
  (reads due tasks via postMessage                  → FCM / Mozilla / Apple push
   to open tab)                                        endpoints
```

**Stack:**

| Layer | Technology |
|-------|-----------|
| Client subscription | Web Push API + `@vite-pwa/sveltekit` service worker |
| Server dispatch | `web-push` npm package (Node crypto + HTTPS) |
| Storage | D1 tables `push_subscriptions`, `wake_schedules` |
| Hosting | Cloudflare Pages (`eisen-web`) with D1 + R2 bindings |
| Platforms | Web only — no FCM/APNs native SDKs, no OneSignal |

---

## Step-by-Step Flow with File References

### 1. Service Worker Registration

The PWA registers a custom service worker via `@vite-pwa/sveltekit` with `injectManifest` strategy.

- `vite.config.ts` — PWA plugin config, `filename: 'service-worker.ts'`, `injectRegister: false`
- `svelte.config.js` — `serviceWorker.register: false` (SvelteKit built-in SW disabled)
- `src/routes/+layout.svelte` — `useRegisterSW()` from `virtual:pwa-register/svelte`

```14:19:src/routes/+layout.svelte
	useRegisterSW({
		onRegisterError(error: Error) {
			console.error('Service worker registration failed:', error);
		}
	});
```

### 2. User Opt-In (Manual)

Push is **not enabled automatically**. The user must visit Settings and click "Enable push reminders".

- `src/routes/settings/+page.svelte` — button calls `open.reminders.enable()`
- `src/lib/workspace/workspace.ts` — `enableReminders()`:

```238:244:src/lib/workspace/workspace.ts
	async function enableReminders(): Promise<void> {
		const perm = await reminders.request();
		if (perm !== 'granted') return;
		const sub = await reminders.subscribe();
		if (sub) await wake.registerPush(sub);
		await scheduleWake();
	}
```

**Early returns with no error surfaced to the UI.** Settings always shows "Push reminders requested." regardless of outcome.

### 3. Permission + Subscription (Client)

- `src/lib/workspace/browser-reminders.ts` — `Notification.requestPermission()` then `pushManager.subscribe()`

```27:38:src/lib/workspace/browser-reminders.ts
		async subscribe() {
			if (!VAPID_PUBLIC_KEY || !('serviceWorker' in navigator) || !('PushManager' in window)) {
				return null;
			}
			const registration = await navigator.serviceWorker.ready;
			const subscription = await registration.pushManager.subscribe({
				userVisibleOnly: true,
				applicationServerKey: urlBase64ToUint8Array(VAPID_PUBLIC_KEY) as BufferSource
			});
			const json = subscription.toJSON();
			if (!json.endpoint || !json.keys?.p256dh || !json.keys?.auth) return null;
			return { endpoint: json.endpoint, p256dh: json.keys.p256dh, auth: json.keys.auth };
		}
```

`VAPID_PUBLIC_KEY` comes from `import.meta.env.VITE_VAPID_PUBLIC_KEY` — a **Vite build-time variable**. If empty, `subscribe()` returns `null` immediately.

### 4. Server Registration

- Client: `src/lib/workspace/http-cloud.ts` → `POST /api/push/subscribe`
- Server: `src/routes/api/push/subscribe/+server.ts` → `mirror.registerPushSubscription()`
- DB: `src/lib/server/adapters/d1.ts` → `INSERT INTO push_subscriptions ... ON CONFLICT(endpoint) DO UPDATE`

Schema (`migrations/0001_init.sql` lines 75–82):

```sql
CREATE TABLE IF NOT EXISTS push_subscriptions (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  endpoint TEXT NOT NULL UNIQUE,
  p256dh TEXT NOT NULL,
  auth TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
```

### 5. Wake Scheduling (Client → Server)

Whenever tasks change or sync completes, the workspace schedules the next upcoming reminder:

- `src/lib/workspace/workspace.ts` — `scheduleWake()` filters tasks with `remindAt > now`, picks earliest
- `src/lib/workspace/http-cloud.ts` → `POST /api/push/schedule`
- `src/routes/api/push/schedule/+server.ts` → `mirror.scheduleWake()`
- DB: `INSERT INTO wake_schedules (id, user_id, device_id, wake_at, nonce, sent)`

```109:124:src/lib/workspace/workspace.ts
	async function scheduleWake() {
		const now = clock.now();
		const upcoming = [...catalog.all()]
			.filter((t) => !t.deleted && !t.completed && !t.archived && t.remindAt && t.remindAt > now)
			.sort((a, b) => (a.remindAt ?? 0) - (b.remindAt ?? 0));
		if (upcoming.length === 0) return;
		try {
			await wake.scheduleWake({
				deviceId,
				wakeAt: upcoming[0].remindAt as number,
				nonce: clock.uuid()
			});
		} catch {
			// wake is best-effort
		}
	}
```

**Note:** Each schedule call inserts a **new row**; there is no cancellation or upsert of prior unsent wakes for the same device.

### 6. Cron Dispatch (Server → Push Endpoint) — **Missing Trigger**

- `src/routes/api/push/cron/+server.ts` — `GET` handler calls `mirror.dispatchDueWakes(Date.now())`
- `src/lib/server/encrypted-mirror.ts` — queries due wakes, sends push, marks sent

```166:181:src/lib/server/encrypted-mirror.ts
		async dispatchDueWakes(now) {
			const due = await ports.database.dueWakes(now);
			let sent = 0;
			for (const row of due) {
				try {
					await ports.pushDispatch.send(
						{ endpoint: row.endpoint, p256dh: row.p256dh, auth: row.auth },
						JSON.stringify({ type: 'wake', userId: row.userId })
					);
					await ports.database.markWakeSent(row.id);
					sent++;
				} catch {
					// leave unsent for retry
				}
			}
			return { sent };
		}
```

**Critical:** Repository search for `cron`, `scheduled`, `triggers`, `CRON_SECRET` returns **no matches** outside the route file itself. `wrangler.toml` has no `[triggers]` section. CI deploys Pages but does not configure any scheduler.

The cron endpoint is also **unauthenticated** — anyone who discovers the URL can trigger dispatch (security concern, separate from "not working").

### 7. Web Push Send (Server)

- `src/lib/server/adapters/web-push.ts` — wraps `web-push` npm package
- `src/lib/server/mirror-from-event.ts` — selects `webPushDispatch` or `noopPushDispatch` based on env vars

```19:22:src/lib/server/mirror-from-event.ts
		pushDispatch:
			publicKey && privateKey
				? webPushDispatch({ publicKey, privateKey, subject })
				: noopPushDispatch
```

```24:28:src/lib/server/adapters/web-push.ts
export const noopPushDispatch: PushDispatchPort = {
	async send() {
		/* VAPID unset */
	}
};
```

If `VAPID_PUBLIC_KEY` or `VAPID_PRIVATE_KEY` are unset in the Cloudflare Pages environment, pushes are silently discarded.

### 8. Service Worker Push Handler

- `src/service-worker.ts` — listens for `push` events

```14:38:src/service-worker.ts
async function handlePush(event: PushEvent) {
	let data: { type?: string; userId?: string } = {};
	try {
		data = event.data?.json() ?? {};
	} catch {
		data = { type: 'wake' };
	}

	if (data.type !== 'wake') {
		await self.registration.showNotification('Eisen', { body: 'You have a reminder.' });
		return;
	}

	const due = await getDueReminders();
	for (const reminder of due) {
		await self.registration.showNotification('Eisen reminder', {
			body: reminder.title,
			tag: reminder.id,
			data: { url: `/task/${reminder.id}` }
		});
	}

	if (due.length === 0) {
		await self.registration.showNotification('Eisen', { body: 'Check your reminders.' });
	}
}
```

### 9. Due Reminder Resolution (Service Worker ↔ Client)

The service worker **cannot read IndexedDB directly**. It posts a message to an open window client:

```52:62:src/service-worker.ts
async function getDueReminders(): Promise<ReminderRow[]> {
	const clients = await self.clients.matchAll({ type: 'window', includeUncontrolled: true });
	if (clients.length === 0) return [];

	return new Promise((resolve) => {
		const channel = new MessageChannel();
		channel.port1.onmessage = (e) => resolve((e.data as ReminderRow[]) ?? []);
		clients[0].postMessage({ type: 'GET_DUE_REMINDERS' }, [channel.port2]);
		setTimeout(() => resolve([]), 2000);
	});
}
```

Client handler in `src/routes/+layout.svelte`:

```74:80:src/routes/+layout.svelte
		navigator.serviceWorker?.addEventListener('message', async (event) => {
			if (event.data?.type !== 'GET_DUE_REMINDERS') return;
			const port = event.ports[0];
			const state = currentOpen();
			if (!port) return;
			port.postMessage(state ? state.dueReminders() : []);
		});
```

`dueReminders()` in workspace filters local tasks where `remindAt <= now`.

---

## Failure Points Ranked by Likelihood

### 1. No cron/scheduler invokes `/api/push/cron` (Very High)

| Evidence | Location |
|----------|----------|
| Cron route exists as manual GET | `src/routes/api/push/cron/+server.ts` |
| No `[triggers]` in wrangler.toml | `wrangler.toml` |
| No cron references anywhere else | repo-wide grep |
| CI deploys Pages only, no scheduler | `.github/workflows/ci.yml` |

**Impact:** Wake rows sit in D1 with `sent = 0` forever. No push is ever sent.

### 2. VAPID keys missing or client/server mismatch (Very High)

| Variable | Where | Status in repo |
|----------|-------|----------------|
| `VITE_VAPID_PUBLIC_KEY` | Client build-time (`.env`) | Empty in `.env.example` |
| `VAPID_PUBLIC_KEY` | Server runtime (Cloudflare env) | Comment-only in `wrangler.toml` |
| `VAPID_PRIVATE_KEY` | Server runtime | Comment-only |
| `VAPID_SUBJECT` | Server runtime | Defaults to `mailto:admin@eisen.app` |

CI `web-build` runs `npm run build` without injecting `VITE_VAPID_PUBLIC_KEY`, so production client bundles likely have an empty public key.

The client's `VITE_VAPID_PUBLIC_KEY` and server's `VAPID_PUBLIC_KEY` **must be the same key pair**. Mismatch causes subscription or send failures (HTTP 401/403 from push services).

### 3. `web-push` incompatible or unreliable on Cloudflare Workers runtime (High)

| Evidence | Detail |
|----------|--------|
| `web-push` uses `node:https.request` | [web-push issue #718](https://github.com/web-push-libs/web-push/issues/718) |
| Eisen `compatibility_date = "2025-07-18"` | Before `enable_nodejs_http_modules` default (2025-08-15) |
| Only `nodejs_compat` flag set | No `enable_nodejs_http_modules` |
| Send errors swallowed | `encrypted-mirror.ts` catch block |

Even if cron runs and VAPID is set, `webpush.sendNotification()` may throw at runtime. The error is caught and the wake stays unsent with no logging.

### 4. Silent failure UX hides all of the above (High)

- `enableReminders()` returns early without throwing
- Settings shows success message unconditionally
- `scheduleWake()` catches and ignores errors
- `noopPushDispatch` and `dispatchDueWakes` catch blocks produce no observability

Users and developers have no signal that anything failed.

### 5. User never completes opt-in flow (Medium)

Push requires:
1. Visit Settings
2. Click "Enable push reminders"
3. Grant notification permission in browser
4. Have `VITE_VAPID_PUBLIC_KEY` set (for subscribe to succeed)

Creating a task with a reminder alone does **not** register a push subscription.

### 6. Service worker cannot resolve task titles without open tab (Medium)

If the app is fully closed (no window clients), `getDueReminders()` returns `[]` after 2s timeout. User gets generic "Check your reminders." instead of task-specific notifications. Push technically "works" but content is lost.

### 7. Wake schedule query does not filter by device (Low–Medium)

```151:160:src/lib/server/adapters/d1.ts
		async dueWakes(now) {
			const { results } = await d1
				.prepare(
					`SELECT ws.id, ws.user_id AS userId, ps.endpoint, ps.p256dh, ps.auth
					 FROM wake_schedules ws
					 JOIN push_subscriptions ps ON ps.user_id = ws.user_id
					 WHERE ws.sent = 0 AND ws.wake_at <= ?`
				)
```

Joins on `user_id` only, not `device_id`. A wake scheduled by device A may notify device B. Multi-device users may get incorrect or duplicate notifications. Does not prevent push from working on single-device setups.

### 8. Wake rows accumulate without deduplication (Low)

Every `scheduleWake()` inserts a new row. Over time, multiple unsent wakes for the same user can fire, causing duplicate notifications once cron works.

### 9. Platform-specific limitations (Context-dependent)

| Platform | Requirement | Eisen support |
|----------|-------------|---------------|
| **Desktop Chrome/Firefox/Edge** | HTTPS, permission, VAPID | Supported if configured |
| **Desktop Safari 16+** | Same + user gesture for permission | Supported if configured |
| **iOS Safari** | PWA added to Home Screen, iOS 16.4+, permission | Web Push only; no native APNs |
| **Android Chrome** | PWA install optional; Web Push works in browser | Supported if configured |
| **Native iOS/Android app** | N/A | Not implemented (no FCM/APNs) |

Eisen has no native mobile app code paths. All push goes through the Web Push API.

### 10. No automated tests for push dispatch (Low)

`encrypted-mirror.test.ts` tests sync LWW only. No tests for `dispatchDueWakes`, `scheduleWake`, `browserReminders`, or the cron route. Regressions would go undetected.

---

## Configuration Checklist (Current State)

| Requirement | Configured? | Evidence |
|-------------|-------------|----------|
| Service worker built and registered | Yes | `vite.config.ts`, `+layout.svelte` |
| D1 tables for subscriptions + wakes | Yes | `migrations/0001_init.sql` |
| API routes for subscribe/schedule/cron | Yes | `src/routes/api/push/*` |
| VAPID public key in client build | **No** | `.env.example` empty; CI doesn't set it |
| VAPID keys in server runtime | **Unknown** (not in repo) | Must be set in Cloudflare dashboard |
| Cron trigger calling `/api/push/cron` | **No** | Not in repo |
| Push library compatible with Workers | **Doubtful** | `web-push` + partial nodejs_compat |
| Error logging / monitoring | **No** | Silent catch blocks |
| User-facing error feedback | **No** | Settings always shows success |

---

## Recommendations / Next Debugging Steps

### Immediate verification (no code changes)

1. **Check Cloudflare Pages environment variables** for `VAPID_PUBLIC_KEY`, `VAPID_PRIVATE_KEY`, `VAPID_SUBJECT`. Confirm they match a generated key pair.

2. **Check production build** for embedded public key:
   ```bash
   # After build, search client bundle
   grep -r "BJB" .svelte-kit/cloudflare/client/  # or whatever your public key starts with
   ```
   If `VITE_VAPID_PUBLIC_KEY` was not set at build time, client subscription will always fail.

3. **Query D1 for data:**
   ```sql
   SELECT COUNT(*) FROM push_subscriptions;
   SELECT COUNT(*) FROM wake_schedules WHERE sent = 0 AND wake_at <= unixepoch() * 1000;
   ```
   Empty subscriptions → opt-in or VAPID client issue. Pending wakes with no sends → cron or dispatch issue.

4. **Manually trigger cron:**
   ```bash
   curl -s "https://<your-domain>/api/push/cron"
   ```
   Check response `{ "sent": N }`. If `sent: 0` with pending wakes, inspect server logs for `web-push` errors.

5. **Browser DevTools → Application → Service Workers / Push** — verify subscription exists after clicking Enable.

### Required fixes (in priority order)

1. **Add a cron trigger** — Cloudflare Cron Trigger (separate Worker or Pages Functions with scheduled invocation) hitting `/api/push/cron` every 1–5 minutes. Protect with a shared secret header (`CRON_SECRET`).

2. **Generate and configure VAPID keys:**
   ```bash
   npx web-push generate-vapid-keys
   ```
   - Set `VITE_VAPID_PUBLIC_KEY` in CI build env (GitHub Actions secret + build step)
   - Set `VAPID_PUBLIC_KEY`, `VAPID_PRIVATE_KEY`, `VAPID_SUBJECT` in Cloudflare Pages env

3. **Replace `web-push` with a Workers-native library** — e.g. [`@pushforge/builder`](https://www.npmjs.com/package/@pushforge/builder) or [`@mmmike/web-push`](https://github.com/MMMikeM/web-push) using Web Crypto + `fetch`. Alternatively add `enable_nodejs_http_modules` and bump `compatibility_date` to `2025-08-15+` as a short-term workaround.

4. **Add observability** — log push send failures (status code, endpoint host), cron invocations, and subscription registration. Surface errors in Settings UI.

5. **Improve wake scheduling** — upsert/cancel prior unsent wakes per device; filter `dueWakes` by `device_id`.

6. **Service worker offline reminders** — persist minimal reminder metadata (id + title + remindAt) in a separate IndexedDB store readable by the service worker, so notifications work when no tab is open.

### Suggested test additions

- Integration test: `registerPushSubscription` + `scheduleWake` + `dispatchDueWakes` with `recordingPushDispatch`
- E2E: enable reminders → create task with `remindAt` → mock cron → assert notification

---

## GitHub Issues Search

Searched via `gh` CLI:

| Query | Results |
|-------|---------|
| `push notification OR FCM OR APNs OR web-push OR vapid` | 0 issues |
| `reminder OR wake OR vapid OR notification` (issue bodies) | 0 issues |
| `push` (title/body) | 0 push-specific issues |

Related closed issues (#99 "Build PWA shell") cover service worker scaffolding for an older Rust/Leptos client, not the current SvelteKit wake-clock implementation.

---

## Sources

### Eisen source code (primary)

| File | Role |
|------|------|
| `src/lib/workspace/browser-reminders.ts` | Client permission + PushManager subscription |
| `src/lib/workspace/workspace.ts` | `enableReminders()`, `scheduleWake()` |
| `src/lib/workspace/http-cloud.ts` | HTTP client for push APIs |
| `src/routes/api/push/subscribe/+server.ts` | Subscription registration |
| `src/routes/api/push/schedule/+server.ts` | Wake scheduling |
| `src/routes/api/push/cron/+server.ts` | Due wake dispatch |
| `src/lib/server/adapters/web-push.ts` | VAPID + web-push send |
| `src/lib/server/adapters/d1.ts` | D1 queries for subscriptions/wakes |
| `src/lib/server/encrypted-mirror.ts` | `dispatchDueWakes()` orchestration |
| `src/service-worker.ts` | Push event + notification display |
| `src/routes/+layout.svelte` | SW registration + `GET_DUE_REMINDERS` handler |
| `migrations/0001_init.sql` | Schema |
| `wrangler.toml` | Cloudflare bindings (no cron, no VAPID) |
| `.env.example` | Empty `VITE_VAPID_PUBLIC_KEY` |
| `.github/workflows/ci.yml` | Build/deploy (no VAPID injection) |
| `CONTEXT.md` | Wake-clock domain definition |

### External primary references

- [Web Push API — MDN](https://developer.mozilla.org/en-US/docs/Web/API/Push_API)
- [Using Web Push — MDN](https://developer.mozilla.org/en-US/docs/Web/API/Push_API/Using_Push_API)
- [VAPID specification — RFC 8292](https://datatracker.ietf.org/doc/html/rfc8292)
- [web-push npm package](https://www.npmjs.com/package/web-push)
- [web-push Cloudflare Worker support issue #718](https://github.com/web-push-libs/web-push/issues/718)
- [Cloudflare Workers Node.js compatibility](https://developers.cloudflare.com/workers/runtime-apis/nodejs/)
- [Cloudflare workerd `https.request` support (2025-08-15)](https://github.com/cloudflare/workerd/issues/3820)
- [Safari Web Push (iOS 16.4+)](https://webkit.org/blog/13878/web-push-for-web-apps-on-ios-and-ipados/)
- [@vite-pwa/sveltekit documentation](https://vite-pwa-org.netlify.app/frameworks/sveltekit.html)
