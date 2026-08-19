# PWA Installability: Eisen Web (`clients/web`)

Research date: 2026-08-19

## Executive summary

Eisen web ships a valid `manifest.webmanifest`, icons, service worker, and Apple meta tags, but **pages do not include a `<link rel="manifest">` in the document head**. Chromium-based browsers require that link (plus a controlling service worker) before they promote a site as an installable PWA. Without it, Android/desktop Chrome fall back to a **browser-badged home-screen shortcut** rather than a WebAPK / “Install app” experience.

On iOS/iPadOS, **there is no install prompt at all** — “Add to Home Screen” is the only path, and that is normal platform behavior, not a misconfiguration.

The predecessor `clients/pwa-svelte` app worked around the SvelteKit gap by adding an explicit manifest link in `+layout.svelte`. Eisen web does not.

---

## 1. Current state of Eisen web PWA config

### What is configured

| Area | Status | Details |
|------|--------|---------|
| **vite-pwa / SvelteKitPWA** | Present | `@vite-pwa/sveltekit` in `vite.config.ts` |
| **Strategy** | `injectManifest` | Custom `src/service-worker.ts` with Workbox precache + push handlers |
| **Manifest file** | Generated at build | `.svelte-kit/cloudflare/manifest.webmanifest` |
| **Service worker** | Built + registered | `service-worker.js` precaches 41 entries (~626 KiB) |
| **Icons** | 192 + 512 PNG + SVG | `/icon-192x192.png`, `/icon-512x512.png`, `/icon.svg` |
| **Manifest fields** | Partial | `name`, `short_name`, `start_url: /`, `display: standalone`, `theme_color`, `background_color` |
| **Apple meta tags** | Present | `apple-mobile-web-app-capable`, `apple-touch-icon`, `apple-mobile-web-app-title` in `app.html` |
| **HTTPS / Cloudflare Pages** | OK | Static PWA assets excluded from Worker in `_routes.json`; served with correct MIME types |
| **Auth gate** | Present | Unauthenticated users redirect to `/sign-in` (`+layout.server.ts`) |

### Production build artifacts (verified locally)

```
.svelte-kit/cloudflare/
  manifest.webmanifest     # application/manifest+json ✓
  service-worker.js        # application/javascript ✓
  registerSW.js            # generated but NOT injected into HTML
  _headers                 # SvelteKit immutable asset headers only (no SW blocking)
  _routes.json             # excludes manifest, SW, icons from Worker
```

Manifest content:

```json
{
  "name": "Eisen",
  "short_name": "Eisen",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#ffffff",
  "theme_color": "#0f766e",
  "scope": "./",
  "icons": [
    { "src": "/icon.svg", "sizes": "any", "type": "image/svg+xml" },
    { "src": "/icon-192x192.png", "sizes": "192x192", "type": "image/png" },
    { "src": "/icon-512x512.png", "sizes": "512x512", "type": "image/png" }
  ]
}
```

### What is missing or misconfigured

| Gap | Impact | Source |
|-----|--------|--------|
| **No `<link rel="manifest">` in rendered HTML** | **Critical** — Chromium will not treat the site as installable | Verified on `/sign-in` SSR output; [@vite-pwa/sveltekit SvelteKit guide](https://vite-pwa-org.netlify.app/frameworks/sveltekit) requires `virtual:pwa-info` in `+layout.svelte` |
| **No `virtual:pwa-info` / `virtual:pwa-register` in layout** | Manifest not linked; vite-pwa registration script unused | `clients/web/src/routes/+layout.svelte` |
| **`svelte.config.js` lacks `serviceWorker.register: false`** | SvelteKit registers SW inline instead of vite-pwa’s `registerSW.js`; works but bypasses vite-pwa integration | [@vite-pwa/sveltekit docs](https://vite-pwa-org.netlify.app/frameworks/sveltekit) |
| **No manifest `description`** | Lighthouse / richer Android install sheet | [vite-pwa minimal requirements](https://vite-pwa-org.netlify.app/guide/pwa-minimal-requirements.html) |
| **No maskable icon (`purpose: "maskable"`)** | Suboptimal Android adaptive icons; not a hard install blocker | [web.dev installation](https://web.dev/learn/pwa/installation) |
| **No `robots.txt`** | vite-pwa / Lighthouse recommendation | [vite-pwa minimal requirements](https://vite-pwa-org.netlify.app/guide/pwa-minimal-requirements.html) |
| **No `<meta name="description">` in `app.html`** | Lighthouse PWA SEO checklist | vite-pwa minimal requirements |
| **No custom `beforeinstallprompt` UI** | Users only see browser chrome install affordance (when criteria met) | [web.dev install criteria](https://web.dev/articles/install-criteria) |
| **`mode: 'development'` in `SvelteKitPWA({...})`** | Likely accidental; **ignored** for `injectManifest` since vite-plugin-pwa v0.18 | `vite-plugin-pwa` types — use `devOptions.enabled` instead |
| **Relative `scope: "./"` in manifest** | Usually resolves to `/` at site root; absolute `scope: "/"` is clearer | W3C manifest spec |

### `vite.config.ts` snapshot

```ts
SvelteKitPWA({
  srcDir: 'src',
  mode: 'development',          // ← not devOptions; ignored for injectManifest
  strategies: 'injectManifest',
  filename: 'service-worker.ts',
  manifest: { /* name, icons, display: standalone, ... */ },
  injectManifest: { globPatterns: ['client/**/*.{js,css,...}'] },
  devOptions: { enabled: true, type: 'module' }
})
```

### Service worker (`src/service-worker.ts`)

- Workbox `precacheAndRoute(self.__WB_MANIFEST)` + `clientsClaim()` — satisfies Chromium SW requirement.
- Custom push / notificationclick handlers for Eisen reminders.
- Build warning: `prerendered/**/*.{html,json}` glob matches nothing (SSR app, no prerendered pages). Harmless but indicates no offline HTML shell beyond SW precache.

### Cloudflare Pages headers

`_headers` contains only SvelteKit-generated immutable asset rules. **No rules block or mis-serve the service worker.**

Verified locally via `wrangler pages dev`:

- `manifest.webmanifest` → `Content-Type: application/manifest+json`
- `service-worker.js` → `Content-Type: application/javascript`

---

## 2. Comparison with `clients/pwa-svelte`

| Item | `pwa-svelte` | `clients/web` |
|------|--------------|---------------|
| PWA plugin | `@vite-pwa/sveltekit` | Same |
| SW strategy | `generateSW` (default) → `sw.js` | `injectManifest` → `service-worker.js` |
| Manifest link in HTML | **Yes** — explicit in `+layout.svelte`: `<link rel="manifest" href="/manifest.webmanifest" />` | **No** |
| vite-pwa virtual modules | Not used | Not used |
| Apple meta tags | `app.html` | `app.html` (+ `viewport-fit=cover`) |
| `mask-icon` | Present in `app.html` | Missing |
| E2E install tests | None | None |

`pwa-svelte` would have had a better chance of Chromium install promotion because it manually linked the manifest. Eisen web removed that link when migrating to the Better Auth stack without adopting the official `virtual:pwa-info` pattern.

---

## 3. Platform-by-platform behavior

### iOS / iPadOS — always “Add to Home Screen”, not “Install”

| Behavior | Detail |
|----------|--------|
| Install prompt | **None.** Users must use Share → “Add to Home Screen”. |
| Standalone app | With `display: standalone` manifest **or** `apple-mobile-web-app-capable`, the icon opens without Safari chrome. |
| Without manifest link | `apple-mobile-web-app-capable` is still set in Eisen web, so Add to Home Screen should open standalone even if Chromium-style install checks fail. |
| iOS 16.4+ third-party browsers | Can add to Home Screen from Share menu; still no install prompt. |
| Perceived as “shortcut only” | **Expected UX** — Apple never shows Chrome-style “Install app”. Wording is always “Add to Home Screen”. |

Sources: [web.dev — Installation (iOS)](https://web.dev/learn/pwa/installation), [WebKit — Web Push for Home Screen web apps](https://webkit.org/blog/13878/web-apps/)

### Android — WebAPK vs shortcut

| Install type | When | User-visible difference |
|--------------|------|-------------------------|
| **WebAPK** (full install) | Chrome + GMS, manifest + SW + engagement heuristics met | App in launcher & Settings → Apps, no browser badge on icon |
| **Shortcut** (fallback) | Criteria not met, or minting unavailable | Home-screen icon with **browser badge**, not in Settings → Apps, limited capabilities |

Eisen web currently matches the **shortcut fallback** profile on Android because the manifest is not linked in HTML, so Chrome’s installability check fails.

Sources: [web.dev — Installation (Android)](https://web.dev/learn/pwa/installation), [web.dev — WebAPKs](https://web.dev/articles/webapks)

### Desktop (Chrome / Edge)

| Behavior | Detail |
|----------|--------|
| Install affordance | Install icon in URL bar or menu when [install criteria](https://web.dev/articles/install-criteria) met |
| Requirements | HTTPS, linked manifest with `name`/`short_name`, 192+512 icons, `start_url`, `display` ∈ {`standalone`,`minimal-ui`,`fullscreen`,`window-controls-overlay`}, controlling SW, user engagement (click + ~30s), not already installed |
| `beforeinstallprompt` | Fires on Chromium when installable; can drive custom in-app install button. **Not available on iOS.** |
| Safari macOS | “Add to Dock” (File menu) for any site; manifest-based promotion is limited |

Without a manifest `<link>`, desktop Chrome will not show the install promotion for Eisen web.

---

## 4. Root causes for “shortcut only” experience

### Root cause A — Missing manifest link in HTML (Chromium: Android + desktop)

**Primary technical blocker.**

`@vite-pwa/sveltekit` does not auto-inject `<link rel="manifest">` into SSR pages. The [official integration](https://vite-pwa-org.netlify.app/frameworks/sveltekit) requires:

```svelte
<script>
  import { pwaInfo } from 'virtual:pwa-info';
  $: webManifestLink = pwaInfo ? pwaInfo.webManifest.linkTag : '';
</script>

<svelte:head>
  {@html webManifestLink}
</svelte:head>
```

Rendered Eisen web `/sign-in` HTML contains service worker registration (SvelteKit inline) but **no manifest link**. `registerSW.js` exists in the build output but is never loaded.

### Root cause B — iOS platform model (not a bug)

Users expecting an “Install app” dialog on iPhone will only ever see **“Add to Home Screen.”** That is indistinguishable from a “shortcut” in Apple’s UI copy. With Eisen’s Apple meta tags, the result should still launch standalone.

### Root cause C — Incomplete manifest / asset polish (secondary)

Missing `description`, maskable icons, and `id` do not block basic Chromium installability but reduce Lighthouse PWA score and Android install-sheet quality.

### Root cause D — No in-app install UX (tertiary)

Even when installable, Chrome requires engagement heuristics before `beforeinstallprompt`. Eisen has no install button or onboarding hint.

### Not root causes (ruled out)

- **Cloudflare `_headers`** — do not block SW or manifest.
- **HTTPS** — satisfied on Pages.
- **Service worker absent** — SW registers and precaches assets.
- **Missing 192/512 icons** — present.
- **`display: standalone`** — correctly set.
- **Auth redirect** — sign-in page is still same-origin; SW scope covers it.

---

## 5. Recommended fixes (ranked by impact)

### 1. Add manifest link via `virtual:pwa-info` (HIGH)

In `clients/web/src/routes/+layout.svelte`:

```svelte
<script lang="ts">
  import { pwaInfo } from 'virtual:pwa-info';
  // ...existing imports...
  const webManifestLink = $derived(pwaInfo ? pwaInfo.webManifest.linkTag : '');
</script>

<svelte:head>
  {@html webManifestLink}
</svelte:head>
```

**Alternative (proven in `pwa-svelte`):** hard-code `<link rel="manifest" href="/manifest.webmanifest" />`.

Expected outcome: Chromium installability check can pass; Android WebAPK / desktop install icon become eligible.

### 2. Align service worker registration with vite-pwa (HIGH)

In `svelte.config.js`:

```js
kit: {
  serviceWorker: { register: false },
  // ...
}
```

In `+layout.svelte`, register via `virtual:pwa-register` on `onMount` (SSR-safe dynamic import per [vite-pwa SvelteKit guide](https://vite-pwa-org.netlify.app/frameworks/sveltekit)).

Expected outcome: Single registration path; consistent scope; access to `offlineReady` / update prompts.

### 3. Remove `mode: 'development'` from `SvelteKitPWA` config (MEDIUM)

Replace with only `devOptions: { enabled: true, ... }` for local SW testing. Avoids confusion; no effect on injectManifest builds today but clarifies intent.

### 4. Enrich manifest + `app.html` (MEDIUM)

- Add `description`, `id: "/"`, `scope: "/"`.
- Add maskable 512 icon (`purpose: "maskable"`).
- Add `<meta name="description" ...>` and optional `mask-icon` (see `pwa-svelte/app.html`).
- Add `static/robots.txt` (`Allow: /`).

Expected outcome: Better Lighthouse PWA score; richer Android install dialog.

### 5. Add optional in-app install prompt (LOW)

Listen for `beforeinstallprompt` on supported browsers; show “Install Eisen” in Settings after engagement.

Not applicable to iOS.

### 6. Verify with Lighthouse + real devices (LOW)

After fix #1:

1. `npm run build && npm run preview`
2. Chrome DevTools → Application → Manifest (should show linked manifest)
3. Lighthouse PWA audit
4. Android Chrome: confirm install offers “Install app” / WebAPK (icon without browser badge)
5. iOS Safari: Share → Add to Home Screen → confirm standalone launch (no Safari URL bar)

---

## 6. Installability criteria reference

### Chromium (Chrome / Edge / Android)

From [web.dev — What does it take to be installable?](https://web.dev/articles/install-criteria):

- Served over HTTPS
- Linked web app manifest with `short_name` or `name`, 192 + 512 icons, `start_url`, `display` mode
- Registered service worker with a fetch handler
- `prefer_related_applications` not true
- User engagement heuristics (interaction + time on site)
- Not already installed

### vite-pwa minimal requirements

From [vite-pwa minimal requirements](https://vite-pwa-org.netlify.app/guide/pwa-minimal-requirements.html):

- Entry-point meta: viewport, title, description, favicon, apple-touch-icon, theme-color
- Manifest: name, description, theme_color, 192 + 512 icons
- `robots.txt` allowing crawl
- Server: HTTPS, correct manifest MIME type

Note: vite-pwa states a service worker is **not** strictly required for “installable” in their checklist, but Chromium **does** require one.

### MDN summary

[Making PWAs installable](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable) — manifest required; HTTPS required; install UI varies by browser; iOS uses Share menu only.

---

## Sources

- [@vite-pwa/sveltekit — SvelteKit framework guide](https://vite-pwa-org.netlify.app/frameworks/sveltekit)
- [vite-pwa — PWA minimal requirements](https://vite-pwa-org.netlify.app/guide/pwa-minimal-requirements.html)
- [web.dev — Install criteria](https://web.dev/articles/install-criteria)
- [web.dev — PWA installation](https://web.dev/learn/pwa/installation)
- [web.dev — WebAPKs on Android](https://web.dev/articles/webapks)
- [MDN — Making PWAs installable](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps/Guides/Making_PWAs_installable)
- [WebKit — Web Push for Home Screen web apps](https://webkit.org/blog/13878/web-apps/)
- Eisen repo: `clients/web/vite.config.ts`, `clients/web/src/service-worker.ts`, `clients/web/src/app.html`, `clients/web/svelte.config.js`, `clients/pwa-svelte/src/routes/+layout.svelte`, `clients/pwa-svelte/vite.config.ts`
