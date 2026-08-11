import { test, expect } from '@playwright/test';
import {
  createVault,
  createTask,
  exportVault,
  importVault,
  createRecoveryPackage,
  restoreRecoveryPackage,
} from './helpers';

test('export and import round-trips the task store', async ({ browser, page }) => {
  await createVault(page);
  await createTask(page, 'Round trip', 'local backup');
  const path = await exportVault(page);

  const ctx2 = await browser.newContext();
  const p2 = await ctx2.newPage();
  await p2.goto('/');
  await importVault(p2, path);
  await expect(p2.locator('.task-title', { hasText: 'Round trip' })).toBeVisible();
});

test('recovery package can restore a vault in a new context', async ({ browser, page }) => {
  await createVault(page);
  await createTask(page, 'Recover me');
  const path = await createRecoveryPackage(page);

  const ctx2 = await browser.newContext();
  const p2 = await ctx2.newPage();
  await p2.goto('/');
  await restoreRecoveryPackage(p2, path);
  await expect(p2.locator('.task-title', { hasText: 'Recover me' })).toBeVisible();
});
