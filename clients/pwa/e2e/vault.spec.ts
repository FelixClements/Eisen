import { test, expect } from '@playwright/test';
import { createVault, unlockVault, PASSPHRASE } from './helpers';

test('create vault and see the unlocked state', async ({ page }) => {
  await createVault(page);
  await expect(page.getByText(/Unlocked vault:/)).toBeVisible();
  await expect(page.getByText('Local-only:')).toBeVisible();
});

test('unlock a previously created vault after reload', async ({ page }) => {
  await createVault(page);
  await page.reload();
  await unlockVault(page);
  await expect(page.getByText(/Unlocked vault:/)).toBeVisible();
});

test('wrong passphrase shows a locked vault notice', async ({ page }) => {
  await createVault(page);
  await page.getByRole('button', { name: 'Unlock' }).click();
  await page.getByPlaceholder('Passphrase').first().fill('wrong passphrase');
  await page.getByRole('button', { name: 'Unlock vault' }).click();
  await expect(page.getByText(/Cannot unlock vault|locked/i)).toBeVisible();
});

test('create and confirm passphrases must match', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Create', exact: true }).click();
  await page.getByPlaceholder('Passphrase').first().fill(PASSPHRASE);
  await page.getByPlaceholder('Confirm passphrase').fill('different');
  await page.getByRole('button', { name: 'Create vault' }).click();
  await expect(page.getByText(/Passphrases do not match/)).toBeVisible();
});
