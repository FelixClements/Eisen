import { writable } from 'svelte/store';

export type UiTheme = 'ios' | 'material';
export type AppearanceMode = 'system' | 'ios' | 'material';

export const APPEARANCE_STORAGE_KEY = 'eisen-appearance';

export function detectPlatformTheme(): UiTheme {
	if (typeof navigator === 'undefined') return 'ios';
	const ua = navigator.userAgent;
	if (/android/i.test(ua)) return 'material';
	if (/iphone|ipad|ipod/i.test(ua)) return 'ios';
	return 'ios';
}

export function resolveAppearance(mode: AppearanceMode): UiTheme {
	if (mode === 'system') return detectPlatformTheme();
	return mode;
}

export function readAppearanceMode(): AppearanceMode {
	if (typeof localStorage === 'undefined') return 'system';
	const stored = localStorage.getItem(APPEARANCE_STORAGE_KEY);
	if (stored === 'ios' || stored === 'material' || stored === 'system') return stored;
	return 'system';
}

export function applyThemeToDocument(theme: UiTheme): void {
	if (typeof document === 'undefined') return;
	const root = document.documentElement;
	root.classList.remove('ios', 'md');
	root.classList.add(theme === 'material' ? 'md' : 'ios');
	root.dataset.eisenTheme = theme;
}

export function readResolvedThemeFromDocument(): UiTheme {
	if (typeof document === 'undefined') return 'ios';
	const fromDom = document.documentElement.dataset.eisenTheme;
	if (fromDom === 'ios' || fromDom === 'material') return fromDom;
	return resolveAppearance(readAppearanceMode());
}

export const appearanceMode = writable<AppearanceMode>('system');
export const resolvedTheme = writable<UiTheme>('ios');

export function initTheme(): void {
	const mode = readAppearanceMode();
	const theme = readResolvedThemeFromDocument();
	appearanceMode.set(mode);
	resolvedTheme.set(theme);
	applyThemeToDocument(theme);
}

export function setAppearanceMode(mode: AppearanceMode): void {
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem(APPEARANCE_STORAGE_KEY, mode);
	}
	const theme = resolveAppearance(mode);
	appearanceMode.set(mode);
	resolvedTheme.set(theme);
	applyThemeToDocument(theme);
}
