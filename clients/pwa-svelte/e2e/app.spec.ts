import { test, expect } from '@playwright/test';
import { PASSPHRASE, unlock, addTask } from './helpers';

const sampleCategories = [
	{ title: 'Urgent meeting', category: 'Do Now' as const },
	{ title: 'Plan vacation', category: 'Schedule' as const },
	{ title: 'Reply to assistant', category: 'Delegate' as const },
	{ title: 'Old magazine', category: 'Eliminate' as const }
];

test.describe('vault', () => {
	test('requires a passphrase to enter', async ({ page }) => {
		await page.goto('/');
		await expect(page.getByPlaceholder('Passphrase')).toBeVisible();
		await expect(page.getByRole('button', { name: /Create account|Unlock/ })).toBeVisible();
	});

	test('unlocks with a passphrase', async ({ page }) => {
		await unlock(page);
		await expect(page.getByRole('link', { name: '+ Add' })).toBeVisible();
	});

	test('locks from the header', async ({ page }) => {
		await unlock(page);
		await page.getByRole('button', { name: 'Lock' }).click();
		await expect(page.getByPlaceholder('Passphrase')).toBeVisible();
	});
});

test.describe('home ledger', () => {
	test.beforeEach(async ({ page }) => {
		await unlock(page);
	});

	test('add button opens the composer', async ({ page }) => {
		await page.getByRole('link', { name: '+ Add' }).click();
		await page.waitForURL(/\/new-task/);
		await expect(page.locator('input[placeholder="Title"]')).toBeVisible();
	});

	test('displays four empty quadrant sections', async ({ page }) => {
		for (const title of ['Do Now', 'Schedule', 'Delegate', 'Eliminate']) {
			await expect(page.locator('.section-header', { hasText: title })).toBeVisible();
		}
	});

	test('groups new tasks by quadrant', async ({ page }) => {
		for (const item of sampleCategories) {
			await addTask(page, item.title, item.category);
		}
		for (const item of sampleCategories) {
			await expect(page.locator('.section-header', { hasText: item.category })).toBeVisible();
			await expect(page.locator('.card', { hasText: item.title })).toBeVisible();
		}
	});

	test('completes a task and moves it to history', async ({ page }) => {
		await addTask(page, 'Complete me');
		const row = page.locator('.card', { hasText: 'Complete me' });
		await row.getByRole('checkbox').check();
		await row.waitFor({ state: 'hidden' });
		await page.getByRole('button', { name: 'Open menu' }).click();
		await page.getByRole('link', { name: 'History' }).click();
		await page.waitForURL(/\/history/);
		await expect(page.locator('.card', { hasText: 'Complete me' }).first()).toBeVisible();
	});

	test('archives a task', async ({ page }) => {
		await addTask(page, 'Archive me');
		const row = page.locator('.card', { hasText: 'Archive me' });
		await row.getByRole('button', { name: 'Archive' }).click();
		await row.waitFor({ state: 'hidden' });
		await page.getByRole('button', { name: 'Open menu' }).click();
		await page.getByRole('link', { name: 'History' }).click();
		await page.waitForURL(/\/history/);
		await page.getByRole('button', { name: 'Archived' }).click();
		await expect(page.locator('.card', { hasText: 'Archive me' }).first()).toBeVisible();
	});

	test('pins a task', async ({ page }) => {
		await addTask(page, 'Pin me');
		await page.locator('.card', { hasText: 'Pin me' }).getByRole('button', { name: 'Pin' }).click();
		await page.locator('.card', { hasText: 'Pin me' }).getByText('📌').waitFor();
	});

	test('searches tasks by title', async ({ page }) => {
		await addTask(page, 'Search target');
		await page.getByRole('button', { name: 'Search' }).click();
		await page.getByPlaceholder('Search tasks…').fill('Search target');
		await expect(page.locator('.card', { hasText: 'Search target' })).toBeVisible();
		await expect(page.locator('.card', { hasText: 'Pin me' })).not.toBeVisible();
	});
});

test.describe('composer', () => {
	test.beforeEach(async ({ page }) => {
		await unlock(page);
		await page.getByRole('link', { name: '+ Add' }).click();
		await page.waitForURL(/\/new-task/);
	});

	test('saves a Do Now task', async ({ page }) => {
		await page.locator('input[placeholder="Title"]').fill('New task');
		await page.getByRole('button', { name: /Do Now/ }).click();
		await page.getByRole('button', { name: 'Save', exact: true }).click();
		await page.waitForURL(/\/$/);
		await expect(page.locator('.card', { hasText: 'New task' })).toBeVisible();
	});

	test('warns when title is empty', async ({ page }) => {
		await page.getByRole('button', { name: 'Save', exact: true }).click();
		await expect(page.getByText('Title is required')).toBeVisible();
	});

	test('allows selecting each quadrant', async ({ page }) => {
		for (const cat of ['Do Now', 'Schedule', 'Delegate', 'Eliminate']) {
			await page.getByRole('button', { name: new RegExp('^' + cat) }).click();
			await expect(page.getByRole('button', { name: new RegExp('^' + cat) })).toHaveClass(/selected/);
		}
	});
});

test.describe('task detail', () => {
	test.beforeEach(async ({ page }) => {
		await unlock(page);
		await addTask(page, 'Edit me');
		await page.locator('.card', { hasText: 'Edit me' }).getByText('Edit me').click();
		await page.waitForURL(/\/task\//);
	});

	test('edits the title', async ({ page }) => {
		const input = page.locator('input[placeholder="Title"]');
		await input.fill('Edited title');
		await page.getByRole('button', { name: '←' }).click();
		await page.waitForURL(/\/$/);
		await expect(page.locator('.card', { hasText: 'Edited title' })).toBeVisible();
	});

	test('changes quadrant', async ({ page }) => {
		await page.getByRole('button', { name: /^Schedule/ }).click();
		await page.getByRole('button', { name: '←' }).click();
		await page.waitForURL(/\/$/);
		await expect(page.locator('.schedule ~ .card', { hasText: 'Edit me' }).first()).toBeVisible();
	});

	test('marks complete from detail', async ({ page }) => {
		await page.getByRole('button', { name: 'Mark complete' }).click();
		await page.getByRole('button', { name: 'Mark active' }).waitFor();
		await page.getByRole('button', { name: '←' }).click();
		await page.waitForURL(/\/$/);
		await expect(page.locator('.card', { hasText: 'Edit me' })).not.toBeVisible();
		await page.getByRole('button', { name: 'Open menu' }).click();
		await page.getByRole('link', { name: 'History' }).click();
		await page.waitForURL(/\/history/);
		await expect(page.locator('.card', { hasText: 'Edit me' }).first()).toBeVisible();
	});
});

test.describe('settings', () => {
	test.beforeEach(async ({ page }) => {
		await unlock(page);
	});

	test('shows the lock button', async ({ page }) => {
		await page.getByRole('button', { name: 'Open menu' }).click();
		await page.getByRole('link', { name: 'Settings' }).click();
		await page.waitForURL(/\/settings/);
		await page.getByRole('button', { name: 'Lock and clear key' }).waitFor();
	});
});

test.describe('navigation drawer', () => {
	test.beforeEach(async ({ page }) => {
		await unlock(page);
	});

	test('opens each drawer destination', async ({ page }) => {
		await page.getByRole('button', { name: 'Open menu' }).click();
		await page.getByRole('link', { name: 'History' }).click();
		await page.waitForURL(/\/history/);
		await expect(page.getByRole('heading', { name: 'History' })).toBeVisible();
	});
});
