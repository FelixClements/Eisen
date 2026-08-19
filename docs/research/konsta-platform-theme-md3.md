# Konsta platform themes & MD3 for Eisen Web

**Question:** Can Eisen Web (`clients/web`) offer an Android MD3 look and automatically show iOS vs Android styling based on the user's device?

**Short answer:** **Partially yes.** Konsta UI v5 already ships two native-like themes (`ios` and `material`). You can switch between them and auto-pick on the client. Konsta's **Material** theme is Material-styled mobile UI — it is **not** a drop-in match for the native Android app's Compose **Material 3** token set. Eisen's custom Eisenhower category colors stay separate CSS and work on either theme.

---

## Current state in `clients/web`

`+layout.svelte` hard-codes iOS:

```svelte
<App theme="ios" safeAreas>
```

Brand colors are Tailwind v4 `@theme` tokens plus Konsta `k-color-*` classes (`app.css`). This is independent of the `ios` / `material` shell theme.

Sources:
- [Konsta App component](https://konstaui.com/svelte/app)
- [Konsta colors](https://konstaui.com/svelte/colors)
- `clients/web/src/routes/+layout.svelte`

---

## What Konsta provides (primary sources)

### 1. Two shell themes: `ios` | `material`

The root `<App>` (or `<KonstaProvider>`) accepts:

| Prop | Values | Effect |
|------|--------|--------|
| `theme` | `'ios'`, `'material'`, `'parent'` | Switches component chrome (navbars, lists, buttons, ripples vs iOS highlights) |
| `dark` | boolean | Dark variants |
| `materialTouchRipple` | boolean | MD touch ripple |
| `iosHoverHighlight` | boolean | iOS tap highlight |
| `safeAreas` | boolean | Notch / home-indicator insets |

`theme="parent"` reads the root element for `ios` or `md` classes ([KonstaProvider](https://konstaui.com/svelte/konsta-provider)).

`useTheme()` returns the active theme inside components ([useTheme](https://konstaui.com/svelte/use-theme)).

**There is no built-in “detect device and set theme” API** — you implement detection and pass `theme` yourself.

### 2. Material color schemes (not full MD3 codegen)

For Material theme only, Konsta supports optional scheme classes on a wrapper:

- `k-md-monochrome`
- `k-md-vibrant`

([Konsta colors](https://konstaui.com/svelte/colors))

Brand colors use `--color-brand-*` in `@theme` (Tailwind v4). Roboto is recommended for Material; iOS uses system font ([existing Eisen research](docs/research/sveltekit-konsta-better-auth-rebuild.md)).

### 3. Relation to Google MD3

Google MD3 defines **design tokens** (color roles, type scale, elevation) in the spec and `material-web` / Compose Material3 ([M3 color](https://m3.material.io/styles/color/static/baseline), [Material Web theming](https://material-web.dev/theming/color/)).

Konsta **reimplements mobile Material/iOS patterns in Tailwind** — it does not consume Android `Theme.kt` or M3 dynamic color from the device. Matching the native Eisen Android app exactly would require **mapping** `clients/android` MD3 tokens → Konsta `--color-brand-*` (and possibly custom CSS), not flipping a single flag.

---

## Recommended approach for Eisen

### A. Three-way theme policy (recommended)

1. **Default:** auto-detect on first visit (client-only).
2. **Override:** Settings → Appearance: *System / iOS / Material*.
3. **Persist:** `localStorage` (and optionally sync to user profile later).

```ts
// clients/web/src/lib/theme.ts (sketch)
export type UiTheme = 'ios' | 'material';
export type ThemeMode = 'auto' | UiTheme;

export function detectPlatformTheme(): UiTheme {
  if (typeof navigator === 'undefined') return 'ios';
  const ua = navigator.userAgent;
  if (/android/i.test(ua)) return 'material';
  if (/iphone|ipad|ipod/i.test(ua)) return 'ios';
  // Desktop / unknown: pick a default or use prefers-color-scheme only for dark, not platform
  return 'ios'; // or 'material' — product decision
}

export function resolveTheme(mode: ThemeMode): UiTheme {
  return mode === 'auto' ? detectPlatformTheme() : mode;
}
```

Wire in layout:

```svelte
<script>
  import { App } from 'konsta/svelte';
  import { themeMode, resolvedTheme } from '$lib/theme'; // store from localStorage
</script>

<App theme={$resolvedTheme} safeAreas>
```

### B. Auto-detection signals (tradeoffs)

| Signal | Pros | Cons |
|--------|------|------|
| `navigator.userAgent` | Simple; works for most phones | Fragile; iPad desktop UA; privacy reduction in some browsers |
| `navigator.userAgentData?.platform` (UA-CH) | Cleaner when available | Not universal; still client-only |
| PWA `display-mode: standalone` + UA | Good for installed PWA | Browser tab ≠ installed app |
| CSS `@media` | No JS | **Cannot** distinguish iOS vs Android look — only viewport, hover, color scheme |
| `prefers-color-scheme` | Good for **dark/light** | Not platform chrome |

**PWA note:** Installed Eisen on Android should get `material`; on iPhone `ios`. Desktop install (Chrome/Edge) is ambiguous — treat as user preference.

### C. SSR / hydration

`theme` on `<App>` affects class names on first paint. If the server renders `ios` but the client detects `material`, users may see a **flash**.

Mitigations (pick one):

1. **Client-only theme** — default `theme="ios"` on server; set store in `onMount` (brief flash).
2. **Cookie + `hooks.server.ts`** — read `appearance` cookie; pass `data.theme` to layout; server and client agree (best UX).
3. **Inline boot script** in `app.html` — read `localStorage` before hydration (no flash; slightly more setup).

SvelteKit: use `$app/environment` `browser` guard for detection ([`$app/environment`](https://svelte.dev/docs/kit/$app-environment)).

### D. What changes vs what does not

| Layer | iOS ↔ Material switch | Notes |
|-------|----------------------|-------|
| Konsta components (Navbar, List, Button, Fab, …) | **Yes** | Main win |
| Custom home ledger (`.section-header`, `.task-card`) | **No** | Custom CSS; already shared; tune per theme if needed |
| Eisenhower quadrant colors | **Optional** | Could align Material quadrants closer to Android `ledgerCategoryColors` |
| Auth / vault screens | **Yes** | Konsta forms benefit automatically |

### E. Android MD3 parity level

| Goal | Effort |
|------|--------|
| “Feels Android on Android” via Konsta `material` | **Low** — theme prop + detection + settings |
| Match native Eisen Android colors/type | **Medium** — token mapping from `CategoryPresentation` / theme files |
| Full MD3 dynamic color from wallpaper | **High** — not supported by Konsta; custom work |

---

## Implementation checklist

1. Add `$lib/theme.ts` with `auto | ios | material`, detection, `localStorage`.
2. Change `<App theme={resolved}>` in `+layout.svelte`.
3. Settings → Appearance row (reuse pattern from `clients/android` settings if desired).
4. Optional: cookie in `hooks.server.ts` to avoid hydration flash.
5. Optional: `class:k-md-vibrant` on `<html>` when `material` + product wants it.
6. Visual QA on iPhone Safari, Android Chrome, desktop Chrome, installed PWA.

---

## Sources

- Konsta App: https://konstaui.com/svelte/app  
- Konsta useTheme: https://konstaui.com/svelte/use-theme  
- Konsta KonstaProvider (`parent` theme): https://konstaui.com/svelte/konsta-provider  
- Konsta colors / MD schemes: https://konstaui.com/svelte/colors  
- Konsta usage: https://konstaui.com/svelte/usage  
- SvelteKit `$app/environment`: https://svelte.dev/docs/kit/$app-environment  
- M3 color system: https://m3.material.io/styles/color/static/baseline  
- Material Web theming: https://material-web.dev/theming/color/  
- Eisen Android UI research: `docs/research/android-app-ui-ux.md`  
- Eisen web Konsta setup: `docs/research/sveltekit-konsta-better-auth-rebuild.md`  
