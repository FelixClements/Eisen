import { test, expect } from '@playwright/test';
import { createVault, createTask, unlockVault } from './helpers';

test('tasks survive a page reload', async ({ page }) => {
  await createVault(page);
  await createTask(page, 'Persisted', 'still here');
  await page.reload();
  await unlockVault(page);
  await expect(page.locator('.task-title', { hasText: 'Persisted' })).toBeVisible();
});

test('tasks can be created while offline and survive reload', async ({ page }) => {
  await createVault(page);
  await page.context().setOffline(true);
  await createTask(page, 'Offline task');
  await page.context().setOffline(false);
  await page.reload();
  await unlockVault(page);
  await expect(page.locator('.task-title', { hasText: 'Offline task' })).toBeVisible();
});
