import { test, expect } from '@playwright/test';
import { createVault, createTask, exportVault, PASSPHRASE } from './helpers';
import fs from 'fs/promises';
import path from 'path';

test('tampered export is rejected without overwriting the vault', async ({ page }) => {
  await createVault(page);
  await createTask(page, 'Original');
  const exported = await exportVault(page);

  const bytes = await fs.readFile(exported);
  bytes[42] ^= 0xff;
  const tampered = path.join('test-results', `tampered-${Date.now()}.bin`);
  await fs.mkdir('test-results', { recursive: true });
  await fs.writeFile(tampered, bytes);

  await page.getByRole('button', { name: 'Backup & Recovery' }).click();
  await page.getByPlaceholder('Current vault passphrase').fill(PASSPHRASE);
  await page.locator('input[type="file"]').first().setInputFiles(tampered);
  await page.getByRole('button', { name: 'Import vault' }).click();

  await expect(page.locator('.error, .repair, .locked')).toBeVisible();
  await page.getByRole('button', { name: 'Unlock' }).click();
  await page.getByPlaceholder('Passphrase').first().fill(PASSPHRASE);
  await page.getByRole('button', { name: 'Unlock vault' }).click();
  await expect(page.locator('.task-title', { hasText: 'Original' })).toBeVisible();
});
