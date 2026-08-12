# Eisen SvelteKit PWA Design Refresh Plan

## Executive Summary

The current Eisen PWA implementation is functionally complete but suffers from a generic "AI slop" visual language characterized by emoji-based iconography, arbitrary color choices, inconsistent spacing, and lack of a cohesive design system. The app uses emoji characters (☰, 🔄, 🔒, 📌, 🗑, ←) as icons, has no formal typography scale or spacing tokens, and lacks Material Design 3's elevation and color system principles. Additionally, there are accessibility issues including an `autofocus` attribute on the new-task title input and a non-interactive navigation element with a click handler.

This design refresh plan establishes a proper design system based on Material Design 3 specifications, replaces emoji icons with a professional icon set (Lucide or Material Symbols), implements proper color tokens and elevation levels, creates a consistent typography scale, and fixes all identified accessibility issues. The plan preserves the working core layouts (home ledger, composer, task detail) while elevating the visual language to match modern mobile-first PWA standards.

---

## Ordered Design Refresh Plan

### Phase 1: Foundation & Design Tokens

**1. Define a color token system based on Material Design 3**
- Replace the current arbitrary color variables with M3 color role tokens (primary, on-primary, primary-container, on-primary-container, surface, on-surface, surface-variant, outline, etc.)
- Keep the existing teal (#0f766e) as the primary seed but generate a full tonal palette
- Add M3 elevation level tokens (0-5) for surface tint instead of shadows
- Source: https://m3.material.io/styles/color/static/baseline and https://material-web.dev/theming/color/

**2. Define a typography scale using Material Design 3 type tokens**
- Implement M3 type scale tokens (display-large, headline-medium, title-large, body-large, body-medium, label-large, etc.)
- Replace current arbitrary sizes (1.25rem, 1rem, 0.875rem) with semantic token names
- Source: https://m3.material.io/styles/typography/type-scale-tokens

**3. Define spacing tokens based on Material Design 3 spacing system**
- Replace arbitrary spacing values (0.5rem, 0.75rem, 1rem, 1.5rem, 2rem) with M3 spacing tokens
- Use 4px base unit with scale (4, 8, 12, 16, 24, 32, 48, 64, 96, 128)
- Source: https://m3.material.io/styles/spacing/tokens

**4. Define elevation tokens using M3 elevation levels (0-5)**
- Replace the single box-shadow on the FAB with M3 elevation system
- Use surface tint colors instead of shadows for most elevation (M3 approach)
- Only use shadows when required for protection or to encourage interaction
- Source: https://m3.material.io/styles/elevation/tokens

### Phase 2: Iconography

**5. Select and integrate a professional icon set (Lucide or Material Symbols)**
- Replace all emoji icons (☰, 🔄, 🔒, 📌, 🗑, ←) with SVG icons from Lucide or Material Symbols
- Add the icon library as a dependency (lucide-svelte for Svelte, or Google Material Symbols via CDN)
- Create icon components for each icon used in the app
- Source: https://lucide.dev/ or https://fonts.google.com/icons

**6. Replace emoji icon buttons with icon-only accessible buttons**
- Update all `.icon-button` elements to use proper SVG icons instead of emoji
- Ensure each icon button has a proper `aria-label` describing the action
- Maintain the 48x48dp minimum touch target size for accessibility
- Source: https://developer.android.com/reference/kotlin/androidx/compose/material3/IconButton.composable

**7. Add proper icon sizing and color tokens**
- Define icon size tokens (small: 18px, medium: 24px, large: 32px)
- Apply M3 color tokens to icons (on-surface, on-surface-variant, primary, etc.)
- Source: Material Design 3 icon guidelines

### Phase 3: Button System

**8. Implement Material Design 3 button variants**
- Create button classes for: filled, filled-tonal, outlined, elevated, and text variants
- Update the existing `.primary` button to use the filled variant
- Add proper button padding, border-radius, and state styles (hover, focus, disabled)
- Source: https://m3.material.io/components/buttons/specs

**9. Update icon buttons to M3 icon button specification**
- Implement proper icon button sizing (48x48dp minimum touch target)
- Add ripple effect or visual feedback on press
- Use M3 icon button colors (surface container low for background, primary for icon)
- Source: https://m3.material.io/components/icon-buttons/overview

**10. Update the FAB to M3 FAB specification**
- Implement proper FAB sizing (56x56dp for standard, 40x40dp for small)
- Add elevation level 3 (6dp) for FAB resting state
- Use primary container color for FAB background, on-primary-container for icon
- Source: https://m3.material.io/components/floating-action-button/specs

### Phase 4: Header & Navigation

**11. Redesign the app header to M3 top app bar (small variant)**
- Update header to use M3 small top app bar specification (64dp height)
- Remove the bottom border and use surface container tint for separation
- Add proper elevation level 0 (no shadow) with surface tint
- Source: https://m3.material.io/components/app-bars/specs

**12. Update the navigation drawer to M3 modal navigation drawer**
- Implement proper drawer width (max 280dp) and elevation level 1
- Use modal drawer container color and scrim color from M3 tokens
- Add proper drawer item styling with selected/active states
- Source: https://m3.material.io/components/navigation-drawer/overview

### Phase 5: Cards & Content

**13. Redesign cards to M3 card specification (outlined or filled variant)**
- Update `.card` to use M3 filled or outlined card variant
- Use surface container highest for filled cards, outline for outlined cards
- Add proper border-radius (12-16dp) and padding (16dp)
- Remove arbitrary 1px border, use M3 elevation or outline variant
- Source: https://m3.material.io/components/cards/overview

**14. Redesign empty states to follow empty state pattern**
- Add an illustration or icon to empty states
- Include a heading, body text, and primary action button
- Use encouraging, second-person, active voice tone
- Source: https://kds.koder.dev/en-US/patterns/patterns-empty-state.html

**15. Redesign settings cards with proper visual hierarchy**
- Group related settings into sections with section headers
- Use M3 card styling for each settings group
- Add proper spacing between groups and within groups
- Source: Material Design 3 card and list specifications

### Phase 6: Vault/Unlock Screen

**16. Redesign the vault/unlock screen with proper visual polish**
- Add app logo/branding to the unlock screen
- Use proper form styling with M3 text field specification
- Add proper spacing and visual hierarchy
- Consider adding a subtle illustration or pattern
- Source: Material Design 3 authentication screen patterns

### Phase 7: Accessibility Fixes

**17. Remove autofocus attribute from new-task title input**
- Remove `autofocus` from line 80 in `src/routes/new-task/+page.svelte`
- Use SvelteKit's focus management or manual focus after mount instead
- Source: https://svelte.dev/docs/svelte/compiler-warnings (a11y_autofocus)

**18. Fix non-interactive nav click in drawer**
- Move the onclick handler from the `<nav>` element to the drawer panel or individual items
- Or add keyboard event handlers (onkeydown/onkeyup) to make the nav element keyboard-accessible
- Source: https://svelte.dev/docs/svelte/compiler-warnings (a11y_click_events_have_key_events)

**19. Ensure all icon buttons have proper accessible names**
- Verify all icon buttons have descriptive `aria-label` attributes
- For toggle buttons (like pin), add `aria-pressed` attribute
- Ensure labels describe the action, not just the icon name
- Source: https://www.w3.org/WAI/ARIA/apg/patterns/button/

**20. Add proper focus indicators for all interactive elements**
- Ensure focus states are visible for keyboard navigation
- Use M3 focus ring specification (2dp outline with primary color)
- Source: Material Design 3 focus and interaction states

### Phase 8: Category Colors

**21. Refine Eisenhower category colors to M3 tonal palette approach**
- Keep the existing category color logic but refine to use M3 tonal palettes
- Use container/on-container pattern for each category (like current do-now-container/do-now-on-container)
- Ensure proper contrast ratios for accessibility
- Source: https://m3.material.io/styles/color/static/baseline

### Phase 9: Responsive & Mobile-First Polish

**22. Ensure proper mobile-first responsive design**
- Verify all touch targets meet 48x48dp minimum
- Test on various screen sizes (320px to 768px+)
- Ensure proper spacing and sizing on small screens
- Source: Material Design 3 layout guidelines

**23. Add proper dark mode support**
- Implement dark mode color tokens using M3 dark scheme
- Add system preference detection (`prefers-color-scheme`)
- Add manual toggle in settings
- Source: https://m3.material.io/styles/color/dark-color

### Phase 10: Testing & Validation

**24. Update E2E tests to reflect new design**
- Update selectors in `e2e/app.spec.ts` to work with new class names and structure
- Add tests for accessibility attributes (aria-label, aria-pressed)
- Test keyboard navigation throughout the app
- Source: Current e2e/app.spec.ts

**25. Validate accessibility with screen reader testing**
- Test with VoiceOver (iOS) and TalkBack (Android)
- Verify all interactive elements are announced properly
- Verify focus order is logical
- Source: https://svelte.dev/docs/kit/accessibility

---

## No-Code Prep Checklist

Before implementing any code changes, gather or decide the following:

### Icon Set Decision
- [ ] Choose between Lucide Icons (lucide-svelte) or Google Material Symbols
- [ ] If Lucide: Confirm all needed icons are available (menu, refresh, lock, pin, trash, arrow-left, search, add, check, archive, settings, history, keyboard, notifications, calendar, clock, etc.)
- [ ] If Material Symbols: Decide between outlined, rounded, or sharp variant
- [ ] Decide on icon sizing strategy (small: 18px, medium: 24px, large: 32px)

### Color Palette
- [ ] Confirm primary color seed (current: #0f766e teal)
- [ ] Generate full M3 tonal palette from seed using Material Theme Builder or material-color-utilities
- [ ] Document exact hex values for all color roles (primary, on-primary, primary-container, on-primary-container, surface, on-surface, surface-variant, outline, error, etc.)
- [ ] Decide on light/dark mode strategy (system-only or manual toggle)

### Typography
- [ ] Choose font family (current: system-ui - keep or switch to Inter/Roboto?)
- [ ] Document type scale with exact pixel/rem values for each M3 role (display-large, headline-medium, title-large, body-large, body-medium, label-large, label-medium, label-small)
- [ ] Decide on line-height and letter-spacing values

### Spacing
- [ ] Document spacing scale in rem values (4, 8, 12, 16, 24, 32, 48, 64, 96, 128 pixels)
- [ ] Map current arbitrary values to new tokens (0.5rem → 8px, 0.75rem → 12px, 1rem → 16px, etc.)

### Elevation
- [ ] Document elevation levels 0-5 with surface tint hex values
- [ ] Decide when to use shadows vs surface tint (M3 prefers surface tint, shadows only when needed)
- [ ] Document which components get which elevation level

### Component Specifications
- [ ] Document button variants with exact padding, border-radius, and colors
- [ ] Document icon button specifications (size: 48x48dp, icon size: 24px)
- [ ] Document FAB specifications (size: 56x56dp, elevation: level 3)
- [ ] Document card specifications (padding: 16dp, border-radius: 12-16dp)
- [ ] Document top app bar specifications (height: 64dp, elevation: level 0)
- [ ] Document navigation drawer specifications (width: max 280dp, elevation: level 1)

### Empty State Content
- [ ] Write copy for home empty state (currently: "No active tasks. Add one to get started.")
- [ ] Write copy for search empty state
- [ ] Write copy for history empty states (completed/archived)
- [ ] Decide on illustration/icon for each empty state

### Accessibility
- [ ] Verify all current aria-label values are descriptive enough
- [ ] Document any new aria-label values needed for new icons
- [ ] Decide on focus management strategy for new-task composer (if not autofocus)
- [ ] Document keyboard navigation patterns to implement

### Testing
- [ ] Identify screen readers to test with (VoiceOver, TalkBack, NVDA)
- [ ] Identify devices/browsers for responsive testing
- [ ] Decide on automated a11y testing tools (axe-core, Lighthouse)
