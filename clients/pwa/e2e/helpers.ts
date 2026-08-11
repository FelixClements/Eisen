import { Page } from '@playwright/test';

export const PASSPHRASE = 'correct horse battery staple';

export async function createVault(page: Page, passphrase = PASSPHRASE) {
  await page.goto('/');
  await page.getByRole('button', { name: 'Create', exact: true }).click();
  await page.getByPlaceholder('Passphrase').first().fill(passphrase);
  await page.getByPlaceholder('Confirm passphrase').fill(passphrase);
  await page.getByRole('button', { name: 'Create vault' }).click();
  await page.getByText(/Unlocked vault:/).waitFor();
}

export async function unlockVault(page: Page, passphrase = PASSPHRASE) {
  await page.goto('/');
  await page.getByRole('button', { name: 'Unlock', exact: true }).click();
  await page.getByPlaceholder('Passphrase').first().fill(passphrase);
  await page.getByRole('button', { name: 'Unlock vault' }).click();
  await page.getByText(/Unlocked vault:/).waitFor();
}

export async function createTask(
  page: Page,
  title: string,
  notes = '',
  quadrant = 'Urgent & Important'
) {
  await page.getByPlaceholder('Title').fill(title);
  if (notes) {
    await page.getByPlaceholder('Notes').fill(notes);
  }
  await page.locator('.task-form select').selectOption(quadrant);
  await page.getByRole('button', { name: 'Add' }).click();
  await page.locator('.task-title', { hasText: title }).waitFor();
}

export async function editTask(
  page: Page,
  oldTitle: string,
  newTitle: string,
  notes = ''
) {
  await page.locator('.task-title', { hasText: oldTitle }).first().click();
  await page.getByPlaceholder('Title').fill(newTitle);
  if (notes) {
    await page.getByPlaceholder('Notes').fill(notes);
  }
  await page.getByRole('button', { name: 'Save' }).click();
  await page.locator('.task-title', { hasText: newTitle }).waitFor();
}

export async function completeTask(page: Page, title: string) {
  const task = page.locator('.task-item', { hasText: title });
  await task.getByRole('button', { name: 'Complete' }).click();
  await task.getByRole('button', { name: 'Restore' }).waitFor();
}

export async function deleteTask(page: Page, title: string) {
  const task = page.locator('.task-item', { hasText: title });
  await task.getByRole('button', { name: 'Delete' }).click();
  await page.locator('.task-title', { hasText: title }).waitFor({ state: 'detached' });
}

export async function restoreTask(page: Page, title: string) {
  const task = page.locator('.task-item', { hasText: title });
  await task.getByRole('button', { name: 'Restore' }).click();
  await task.getByRole('button', { name: 'Complete' }).waitFor();
}

export async function moveTask(page: Page, title: string, quadrant: string) {
  const task = page.locator('.task-item', { hasText: title });
  await task.locator('select').selectOption(quadrant);
  await page
    .locator('.quadrant', { hasText: quadrant })
    .locator('.task-title', { hasText: title })
    .waitFor();
}

export async function exportVault(page: Page, passphrase = PASSPHRASE) {
  await page.getByRole('button', { name: 'Backup & Recovery', exact: true }).click();
  await page.getByPlaceholder('Current vault passphrase').fill(passphrase);
  await page.getByRole('button', { name: 'Export vault' }).click();
  const [download] = await Promise.all([
    page.waitForEvent('download'),
    page.getByText('Download export file').click(),
  ]);
  const path = `test-results/export-${Date.now()}.bin`;
  await download.saveAs(path);
  return path;
}

export async function importVault(page: Page, filePath: string, passphrase = PASSPHRASE) {
  await page.getByRole('button', { name: 'Backup & Recovery', exact: true }).click();
  await page.getByPlaceholder('Current vault passphrase').fill(passphrase);
  await page.locator('input[type="file"]').first().setInputFiles(filePath);
  await page.getByRole('button', { name: 'Import vault' }).click();
  await page.getByText(/Import completed|Unlocked vault:/).waitFor();
}

export async function createRecoveryPackage(page: Page, passphrase = PASSPHRASE, locator = '') {
  await page.getByRole('button', { name: 'Backup & Recovery', exact: true }).click();
  if (locator) {
    await page.getByPlaceholder('Optional locator').fill(locator);
  }
  await page.getByPlaceholder('Current vault passphrase').fill(passphrase);
  await page.getByRole('button', { name: 'Create recovery package' }).click();
  const [download] = await Promise.all([
    page.waitForEvent('download'),
    page.getByText('Download recovery package').click(),
  ]);
  const path = `test-results/recovery-${Date.now()}.bin`;
  await download.saveAs(path);
  return path;
}

export async function restoreRecoveryPackage(page: Page, filePath: string, passphrase = PASSPHRASE) {
  await page.getByRole('button', { name: 'Backup & Recovery', exact: true }).click();
  await page.getByPlaceholder('Current vault passphrase').fill(passphrase);
  await page.locator('input[type="file"]').nth(1).setInputFiles(filePath);
  await page.getByRole('button', { name: 'Restore vault' }).click();
  await page.getByText(/Unlocked vault:/).waitFor();
}
