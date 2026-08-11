import { test, expect } from '@playwright/test';
import {
  createVault,
  createTask,
  completeTask,
  restoreTask,
  deleteTask,
  moveTask,
  editTask,
} from './helpers';

test.beforeEach(async ({ page }) => {
  await createVault(page);
});

test('create a task and see it in the matrix', async ({ page }) => {
  await createTask(page, 'First task', 'Some notes', 'Important, Not Urgent');
  await expect(
    page.locator('.quadrant', { hasText: 'Important, Not Urgent' }).locator('.task-title', { hasText: 'First task' })
  ).toBeVisible();
});

test('edit a task', async ({ page }) => {
  await createTask(page, 'Editable');
  await editTask(page, 'Editable', 'Edited task', 'Updated notes');
  await expect(page.locator('.task-title', { hasText: 'Edited task' })).toBeVisible();
  await page.locator('.task-title', { hasText: 'Edited task' }).click();
  await expect(page.getByPlaceholder('Notes')).toHaveValue('Updated notes');
});

test('complete, restore, and delete a task', async ({ page }) => {
  await createTask(page, 'Lifecycle');
  await completeTask(page, 'Lifecycle');
  await expect(page.locator('.task-item', { hasText: 'Lifecycle' })).toHaveClass(/completed/);
  await restoreTask(page, 'Lifecycle');
  await expect(page.locator('.task-item', { hasText: 'Lifecycle' })).not.toHaveClass(/completed/);
  await deleteTask(page, 'Lifecycle');
  await expect(page.locator('.task-title', { hasText: 'Lifecycle' })).toHaveCount(0);
});

test('move a task between quadrants', async ({ page }) => {
  await createTask(page, 'Mover', '', 'Urgent & Important');
  await moveTask(page, 'Mover', 'Not Urgent, Not Important');
  await expect(
    page.locator('.quadrant', { hasText: 'Not Urgent, Not Important' }).locator('.task-title', { hasText: 'Mover' })
  ).toBeVisible();
});
