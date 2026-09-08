# Sign-in silent failure — diagnosis

**Date:** 2026-09-08  
**Question:** Why does production Sign in at `https://eisen-web.pages.dev/sign-in` appear to do nothing when the user taps Sign in, in every browser?  
**Method:** Primary sources (Svelte 5 runtime docs, HTML living standard, SvelteKit service-worker docs, Better Auth 1.7, Cloudflare Error 1102, Konsta Button source, Eisen repo) plus a trusted Playwright tap on production. No code fix.

---

## Verdict

**Root cause:** a Svelte 5 infinite `$effect` in the root layout that reads and writes the same `$state` (`tick` in `bindWorkspace`). That matches the Playwright load-time `effect_update_depth_exceeded` error (stack in Svelte runtime, originating in layout `nodes/0.*.js`). It runs on every logged-out visit, including `/sign-in`, in every browser.

**Tap with no visible change (proven 2026-09-08):** a trusted Playwright click on production Sign in ran `handleSignIn`. Dummy credentials produced `POST /api/auth/sign-in/email` with the PBKDF2 verifier body and **status 401**. The URL stayed `/sign-in`, cookies stayed empty, and neither “Signing in…” nor the error paragraph appeared. The handler ran; the UI did not update.

**Ruled out as the tap swallow:** Better Auth cookies / Safari ITP, Cloudflare Error 1102 on the HTML document, a registered service worker intercepting `/api/auth`, Konsta rest-spread stealing `onclick`, and `<form method="dialog">` (auth POST means `onsubmit` ran and `preventDefault` canceled native submission).

---

## Production markup (live)

Fetched `https://eisen-web.pages.dev/sign-in` on 2026-09-08. SSR HTML contains:

```html
<form method="dialog" class="space-y-4">
  … <input name="email" type="email" …>
  … <input name="password" type="password" …>
  <button … type="submit" role="button" tabindex="0">Sign in</button>
</form>
```

There is no ancestor `<dialog>`. This live markup matches the working tree of `src/routes/sign-in/+page.svelte`, not `origin/main` HEAD.

---

## Hypothesis A — layout `$effect` ↔ `bindWorkspace` `tick += 1` (primary, confirmed)

### Claim

On `/sign-in` with no session user, the root layout’s second `$effect` calls `bindWorkspace(null)` on every run. `bindWorkspace` always does `tick += 1`. `$effect` tracks `$state` that is read **synchronously inside called functions**. `tick += 1` both reads and writes `tick`, which is the documented infinite-loop pattern. Svelte then throws `effect_update_depth_exceeded`. After that, `busy` / `error` `$state` writes from `handleSignIn` do not paint.

### Repo

`src/routes/+layout.svelte`:

- `user` is `$derived($session.data?.user ?? data.user)` from `authClient.useSession()` plus `data.user`.
- `isPublic` is `$derived` from `$page.url.pathname` (`/sign-in` or `/sign-up`).
- `open = $derived(currentOpen())`.
- First `$effect`: reads `open?.sync.lastError?.code === 'session-expired'` then `goto('/sign-in')`.
- Second `$effect`: if `!user?.id`, calls `bindWorkspace(null)` and returns. That branch does **not** wait for `isPublic`; logged-out sign-in always takes it.

`src/lib/workspace/current.svelte.ts`:

```ts
let bound: Workspace | null = $state(null);
let tick = $state(0);

export function bindWorkspace(ws: Workspace | null) {
	unsub?.();
	unsub = null;
	if (bound && bound !== ws) bound.close();
	bound = ws;
	tick += 1; // read + write, even when ws is already null
	if (ws) {
		unsub = ws.subscribe(() => {
			tick += 1;
		});
	}
}

export function currentOpen(): OpenState | null {
	void tick;
	if (!bound) return null;
	const state = bound.state;
	return state.status === 'open' ? state : null;
}
```

The logged-in `bindWorkspace(ws)` in the layout sits after `await vaultSession.resume(...)`, so those `$state` writes are **not** tracked as effect dependencies (async reads are untracked). The logged-out path is synchronous. That is why the error shows up on the public sign-in page.

The first layout `$effect` also reads `open` → `currentOpen()` → `tick`, so it sits in the same reactive graph, but it does not write `tick`. The second effect is the writer.

### Primary docs

[effect_update_depth_exceeded](https://svelte.dev/e/effect_update_depth_exceeded):

> Maximum update depth exceeded. This typically indicates that an effect reads and writes the same piece of state.

The official example is `count += 1` inside `$effect`. Svelte “intervenes before this can crash your browser tab.”

[$effect](https://svelte.dev/docs/svelte/$effect):

- Effects run in the browser after mount / after DOM updates, not during SSR.
- “`$effect` automatically picks up any reactive values (`$state`, `$derived`, `$props`) that are **synchronously** read inside its function body (**including indirectly, via function calls**) and registers them as dependencies.”
- Values read after `await` are not tracked.
- “Generally speaking, you should not update state inside effects… never-ending update cycles.”

### Playwright tap (2026-09-08)

Trusted click on production `/sign-in` with `probe@example.com` / dummy password:

| Check | Result |
| --- | --- |
| Pageerror on load | `Error: https://svelte.dev/e/effect_update_depth_exceeded` |
| After click: URL | still `/sign-in` |
| After click: “Signing in…” | count `0` |
| After click: error paragraph | count `0` |
| After click: `POST /api/auth/sign-in/email` | yes; body was the derived verifier (base64); **status 401** |
| Cookies | none |

`handleSignIn` sets `busy`, then `vaultSession.unlock` (PBKDF2 + `signIn.email`), then `error` on throw. The POST is that path. The missing busy/error text is the failed UI flush.

### Falsify

- Rebuild without calling `bindWorkspace` from the logged-out `$effect` (or without `tick += 1` when `ws` is already `null`). If the load-time `effect_update_depth_exceeded` **remains**, this is not the source of that error (look at Konsta `App` / `KonstaProvider` `$effect`s that write `KonstaStore`, or the sign-in page’s own `$effect`).
- If the error **vanishes** but a tap still paints nothing after a 401, this hypothesis does not explain the tap.
- If after the error a tap still shows “Signing in…” or an error string, the “flush is dead” claim is false.

---

## Hypothesis B — `<form method="dialog">` with no ancestor dialog (ruled out as primary)

### Claim

The HTML living standard defines `method="dialog"` as: close the dialog the form is in, **and otherwise not submit**. Production has that method and no ancestor `<dialog>`.

### Primary spec

[Form submission algorithm](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#form-submission-algorithm) (WHATWG):

1. Fire a cancelable `submit` event at the form (`shouldContinue`).
2. If that event is canceled (`preventDefault`), **return**.
3. Later: **If method is dialog:** **If form does not have an ancestor `dialog` element, then return.**

The `method` keyword table (same page): `dialog` = “Indicates the form is intended to close the dialog box in which the form finds itself, if any, **and otherwise not submit**.”

Repo: `src/routes/sign-in/+page.svelte` has `<form method="dialog" onsubmit={handleSignIn}>`. `handleSignIn` starts with `e.preventDefault()`.

### Status

The Playwright tap produced `POST /api/auth/sign-in/email`. `vaultSession.unlock` ran, so `onsubmit` ran and `preventDefault` canceled native submission at step 2. `method=dialog` never reached the “no ancestor → return” step. It can still prevent a GET navigation if JS is missing; it is not why this tap looked dead.

---

## Hypothesis C — Konsta `Button` rest props spread after `onclick` (weak for live markup)

### Claim

Konsta’s Svelte Button sets `onclick={onClick || onclick}` and then `{...attrs}` on the same `<svelte:element>`. In Svelte, a later spread wins. A rest `onclick` could clobber the explicit handler.

### Source

`node_modules/konsta/svelte/components/Button.svelte` (string `component` branch):

```svelte
<svelte:element
  this={Component}
  onclick={onClick || onclick}
  bind:this={el}
  class={classes}
  {disabled}
  {...attrs}
  role="button"
  tabindex="0"
>
```

`attrs` is `$derived({ href, ...restProps })`. `onclick` / `onClick` are **named** `$props()`, so they are not in `restProps` for a normal `onclick={...}` caller.

Live production Sign in control is `type="submit"` with no HTML `onclick`. The working-tree page uses `type="submit"` and form `onsubmit`, not Button `onclick`. A Playwright role click produced the auth POST. Konsta is not the gate.

---

## Hypothesis D — service worker evaluation failed (ruled out for the tap)

### Observed

Playwright console: `Failed to register a ServiceWorker for scope ('https://eisen-web.pages.dev/') with script ('https://eisen-web.pages.dev/service-worker.js'): ServiceWorker script evaluation failed`, logged from layout `nodes/0.*.js`.

### Repo + SvelteKit docs

[SvelteKit service workers](https://svelte.dev/docs/kit/service-workers): if `src/service-worker.js` (or `src/service-worker/index.js`) exists, SvelteKit **automatically bundles and registers** it, unless registration is disabled.

Eisen disables that:

- `svelte.config.js` — `kit.serviceWorker.register: false`
- `vite.config.ts` — `@vite-pwa/sveltekit` with `injectRegister: false`, `strategies: 'injectManifest'`, `filename: 'service-worker.ts'`
- `src/routes/+layout.svelte` — `useRegisterSW({ onRegisterError })` from `virtual:pwa-register/svelte`, which `console.error`s the observed string

`src/service-worker.ts` calls `precacheAndRoute(self.__WB_MANIFEST)` at evaluation time. `vite.config.ts` sets `injectManifest.injectionPoint: false`. If `__WB_MANIFEST` is missing, Workbox throws during script evaluation — which is exactly `ServiceWorker script evaluation failed`.

A worker whose script **fails to evaluate is not installed**. It cannot intercept fetches. Push/offline can break; sign-in click handling does not depend on a controller.

---

## Hypothesis E — Better Auth 1.7 cookies (ruled out as “every browser, tap does nothing”)

Installed: `better-auth@1.7.1`. Docs: Better Auth **v1.7**.

[SvelteKit integration](https://better-auth.com/docs/integrations/svelte-kit.md): mount `svelteKitHandler` in `hooks.server.ts`; `sveltekitCookies(getRequestEvent)` is for **server-action** cookies and must be last.

[Cookies](https://better-auth.com/docs/concepts/cookies.md): production cookies are `httpOnly` and `secure`. Safari ITP blocks **third-party** cookies when the auth API is on a **different site**. Same-origin `/api/auth` is first-party.

Dummy sign-in returned **401** with **no Set-Cookie**, which is correct for a wrong password. Cookie/origin mismatch would matter only after a **200**. Switching browsers would have changed a Safari-only cookie bug; it did not.

---

## Hypothesis F — Cloudflare Error 1102 (ruled out for this tap)

[Error 1102: Worker exceeded resource limits](https://developers.cloudflare.com/support/troubleshooting/http-status-codes/cloudflare-1xxx-errors/error-1102/): Worker CPU or 128 MB memory. Playwright loaded `/sign-in` and the auth POST returned **401**, not 1102.

---

## Other layout / theme notes (not the primary loop)

`src/lib/theme.ts`: `resolvedTheme` is a **Svelte store**, set in `initTheme()` once on the client. Layout: `<App theme={$resolvedTheme} …>`.

Konsta `App.svelte` / `KonstaProvider.svelte` write `KonstaStore.theme` inside `$effect`. Those effects assign store fields from props; they do not increment a counter on every run. Plausible **alternate** `effect_update_depth_exceeded` source **only if** hypothesis A is falsified.

The sign-in page’s own `$effect` calls `authClient.getSession()` then possibly `goto('/')` **after `await`**, so those reads are untracked. It does not write `tick`.

---

## How the hypotheses relate

| Hypothesis | Explains load-time Svelte error? | Explains SW console error? | Explains tap with no navigation in every browser? |
| --- | --- | --- | --- |
| A. Layout `tick += 1` in `$effect` | Yes (code + official error) | No | Yes: handler ran (auth POST 401); busy/error never painted |
| B. `method=dialog` without `<dialog>` | No | No | No as primary: auth POST means `handleSignIn` ran |
| C. Konsta spread-after-`onclick` | No | No | Unlikely for live `type=submit` + form `onsubmit` |
| D. SW evaluation failed | No | Yes | No (worker did not activate) |
| E. Better Auth cookies | No | No | No for “tap does nothing”; maybe later “no session after 200” |
| F. Error 1102 | No | No | No for a page that already rendered; this POST was 401 |

**Joint reading:** A is true on load (console) and explains the tap (auth POST + no UI). D is a separate PWA failure. B is leftover markup, not the swallow. E and F are not required.

**Remaining experiment for a fix:** rebuild without the logged-out `tick += 1` loop. The load-time Svelte error must disappear, and a wrong-password tap must paint the existing error string.

---

## Sources

| Claim | Source |
| --- | --- |
| `effect_update_depth_exceeded` text and `count += 1` example | https://svelte.dev/e/effect_update_depth_exceeded |
| `$effect` tracks sync reads in nested calls; do not write deps | https://svelte.dev/docs/svelte/$effect |
| Form `method=dialog` “otherwise not submit”; algorithm returns without ancestor dialog after `submit` | https://html.spec.whatwg.org/multipage/form-control-infrastructure.html |
| SvelteKit auto-registers `src/service-worker.js` unless disabled | https://svelte.dev/docs/kit/service-workers |
| Better Auth 1.7 SvelteKit handler, locals, `sveltekitCookies` | https://better-auth.com/docs/integrations/svelte-kit.md |
| Better Auth cookies, secure/httpOnly, Safari ITP is cross-site | https://better-auth.com/docs/concepts/cookies.md |
| Error 1102 = Worker CPU or memory limit | https://developers.cloudflare.com/support/troubleshooting/http-status-codes/cloudflare-1xxx-errors/error-1102/ |
| Konsta Button `onclick` then `{...attrs}` | `node_modules/konsta/svelte/components/Button.svelte` |
| Layout effects, `bindWorkspace`, sign-in form | `src/routes/+layout.svelte`, `src/lib/workspace/current.svelte.ts`, `src/routes/sign-in/+page.svelte` |
| Live form markup | `GET https://eisen-web.pages.dev/sign-in` (2026-09-08) |
| Tap: auth POST 401, no busy/error UI | Playwright against production, 2026-09-08 |
