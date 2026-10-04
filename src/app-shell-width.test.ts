import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const css = readFileSync(new URL('./app.css', import.meta.url), 'utf8');
const tabbar = readFileSync(new URL('./lib/components/AppTabbar.svelte', import.meta.url), 'utf8');

describe('app shell width', () => {
	it('keeps the 32rem column off phone-width screens', () => {
		const shell = css.match(/\.app-shell\s*\{[^}]*\}/)?.[0] ?? '';
		expect(shell).not.toMatch(/max-width:\s*32rem/);
		expect(css).toMatch(
			/@media\s*\(\s*pointer:\s*fine\s*\)\s*and\s*\(\s*min-width:\s*64rem\s*\)\s*\{[^}]*\.app-shell\s*\{[^}]*max-width:\s*32rem/
		);
		expect(tabbar).not.toMatch(/max-w-lg/);
	});
});
