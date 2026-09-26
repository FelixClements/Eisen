# Live Sync fanout

## Problem

Sync uploads and downloads encrypted Task blobs for an Account. It runs when the Workspace opens, when this device edits a Task, and when the user presses Sync now. An open Workspace that is sitting idle does not learn about another device's edits until one of those triggers.

## Goal

While a Workspace is open and the tab is visible, a change accepted on another device appears within a second or two, without a manual Sync. Edits on this device still upload immediately. Returning to a hidden tab runs one Sync immediately. Hiding the tab, locking the phone, or sleeping the laptop does not sync in the background. Manual Sync and the Sync on Workspace open stay as they are.

## Non-goals

- Web Push, wake-clock, or any other background nudge for Sync.
- Polling as the way an open tab hears about changes.
- Changing record last-write-wins, the Vault key, or what `POST /api/sync` returns.
- Putting Task plaintext or ciphertext on the live channel.
- Driving a real Durable Object from the automated suite.

This design follows [ADR-013](../../adr/013-web-one-password-e2ee.md). Merge stays whole-record last-write-wins on `(updatedAt, deviceId)`. The server remains an opaque blob store.

## Signal path

Task blobs move only through the existing Sync exchange, `POST /api/sync`. A new Worker, `eisen-sync-fanout`, holds one Durable Object per Account. The object remembers which Workspaces are listening and tells them a version number. It does not read D1 and it never sees a Task blob.

The browser opens the listener through Pages, on the same-origin WebSocket `GET /api/sync/watch`. The Pages route checks the Better Auth session, then attaches the socket to the object for that Account. The Account id comes from the session. The socket stays up only while the Workspace is open and `document.visibilityState` is `visible`.

When an exchange advances the Account's sync version, Pages tells the object the new version. The object sends `{ "type": "changed", "version": <number> }` to every attached socket, including the device that just wrote. Each Workspace compares that version with the one it has already applied. A newer version runs the same Sync used on open and on Sync now. An equal or older version does nothing.

Hiding the tab closes the socket. Showing the tab runs one Sync, then opens the socket again. Signing out closes it. If the Worker binding is absent, Sync still succeeds and the live signal is skipped.

## Client lifecycle

`sync-watch` owns the socket, the visibility listener, the reconnect timer, and the follow-up loop. It calls `syncEngine.run()` and reads the last applied version. The Workspace starts it once the Workspace is open and the opening Sync attempt has settled, whether that attempt succeeded or failed. It connects only while the tab is visible. Local edits keep calling `syncEngine.run()` themselves, as they do today. That upload is what advances the version and causes the ping.

The listener stores the newest ping version it has seen. A ping less than or equal to the applied version is ignored. Otherwise it starts Sync. If Sync is already in flight, a second `run()` joins that flight and would not pull a write that landed after the snapshot. The listener therefore waits for the flight to finish, then runs Sync once more while the newest ping version is still greater than the applied version. One extra pull is enough for every version that landed during the flight, because Sync downloads every record after the applied version. A ping that arrives during that extra pull schedules another, under the same rule. The writer often takes this extra pull: its own ping can arrive before it has stored the new version. That pull uploads nothing new.

If the socket drops while the tab is still visible, the listener reconnects with delays of 1s, 2s, 4s, 8s, 16s, and then 30s. A successful connect resets the delay and runs one Sync, which repairs a missed ping. A 401 on the watch route or on Sync stops reconnecting. The Workspace sets `sync.lastError` to `{ code: "session-expired" }`, the same error Settings already shows when Sync itself gets a 401. Sign-out closes the socket and cancels the timer. The tab becoming visible, or the browser coming back online, tries again.

`GET /api/sync/watch` returns 404 when the fanout binding is missing. `sync-watch` treats 404 as "not deployed": it does not reconnect on a timer. The next time the tab becomes visible, it tries once more. Sync is unaffected.

## Components

### `sync-watch`

A Workspace module. The Workspace starts it after the opening Sync attempt settles. `signOut` stops it. It does not encrypt, merge, or touch Dexie.

### `POST /api/sync`

Stays the only exchange. The handler reads the Account's sync version, runs the exchange, then reads it again. If the version advanced, it tells the fanout. If that tell fails, or the binding is missing, the handler still returns `{ changes, lastVersion }`. An empty pull, or a push rejected by last-write-wins, does not advance the version and does not ping anyone. A concurrent exchange may advance the version between the two reads and cause an extra ping. Clients ignore a version they already have. The HTTP body stays `{ changes, lastVersion }`.

### `GET /api/sync/watch`

Requires a signed-in Account. It proxies the WebSocket to the Worker over a service binding. Pages sets two headers the browser cannot set: `X-Fanout-Secret`, whose value is the `FANOUT_SECRET` env var shared with the Worker, and `X-Account-Id`, the Account id from the session.

### `eisen-sync-fanout`

Deployed beside the push-cron Worker, because Pages cannot hold the long-lived connection. The Worker rejects any request whose `X-Fanout-Secret` header does not match `FANOUT_SECRET`, so a direct hit on the Worker host cannot attach or notify.

One Durable Object per Account, addressed by the Account id, using hibernated WebSockets. It stores no Task data and does not read D1. Notify is an internal `POST /notify` with `{ accountId, version }`. The object sends `{ "type": "changed", "version": <number> }` to each attached socket. If the object restarts, the sockets drop. Each visible Workspace reconnects and runs one Sync.

## Error handling

| Situation | Behavior |
| --- | --- |
| Notify fails, or the binding is missing | Sync response is unchanged. Other devices catch up on the next successful ping, on visibility, or on the next Sync. |
| Watch returns 404 | Stop the reconnect timer. Try again the next time the tab becomes visible. |
| Socket drops while visible | Backoff, then connect and run one Sync. |
| 401 on watch or Sync | Stop reconnecting. Surface session-expired. |
| Tab hidden, or sign-out | Close the socket and cancel the timer. |
| Offline | The existing Sync phase reports offline. Visibility or coming back online tries again. |

## Tests

Vitest, with fakes. No real Durable Object.

`sync-watch`:

- An open, visible Workspace connects.
- Hiding the tab closes the socket and leaves it closed.
- Showing the tab runs one Sync, then connects.
- A `changed` version newer than the last applied version runs Sync. An equal or older version does not.
- A newer ping that arrives while Sync is in flight causes one more Sync after that flight, and only if the applied version is still behind. When the in-flight Sync already reaches the ping's version, there is no extra run.
- A drop while visible waits through backoff, connects again, and runs one Sync on that connect.
- A 401 stops reconnecting and reports session-expired.
- A 404 does not start the reconnect timer.
- Sign-out closes the socket and cancels the timer.

Mirror exchange:

- A write that advances the sync version records a notify with that version.
- An empty pull or a last-write-wins rejection records none.

Sync route:

- A failed notify, or a missing binding, still returns `{ changes, lastVersion }`.

Fanout object, with fake sockets:

- A notify sends `{ "type": "changed", "version": <number> }` to every attached socket.
- A request without the shared secret is rejected.

Workspace:

- Two catalogs share an in-memory fanout. An edit on the first becomes a `changed` ping, and the second pulls the Task without anyone calling Sync by hand.

A live check with two browsers is a manual pass after deploy.

## Boundaries

`sync-watch` decides when to call Sync. `syncEngine` still performs one exchange and merges records. The Pages route decides whether the Account is signed in. The Worker decides who is listening and forwards a version. D1 remains the only store of Task blobs.
