import { type Page } from '@playwright/test';

export const PASSPHRASE = 'correct horse battery staple';

export async function unlock(page: Page) {
	await page.goto('/');
	await page.getByPlaceholder('Passphrase').fill(PASSPHRASE);
	await page.getByRole('button', { name: 'Unlock' }).click();
	await page.getByRole('link', { name: '+ Add' }).waitFor({ timeout: 10000 });
}

export async function addTask(
	page: Page,
	title: string,
	category: 'Do Now' | 'Schedule' | 'Delegate' | 'Eliminate' = 'Do Now'
) {
	await page.getByRole('link', { name: '+ Add' }).click();
	await page.waitForURL(/\/new-task/);
	await page.locator('input[placeholder="Title"]').fill(title);
	await page.getByRole('button', { name: new RegExp('^' + category) }).click();
	await page.getByRole('button', { name: 'Save', exact: true }).click();
	await page.waitForURL(/\/$/);
}
