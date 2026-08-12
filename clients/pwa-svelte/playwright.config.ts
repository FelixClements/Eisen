import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
	testDir: 'e2e',
	fullyParallel: false,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	workers: 1,
	reporter: 'list',
	timeout: 60000,
	use: {
		baseURL: 'http://localhost:8788',
		trace: 'on-first-retry',
		ignoreHTTPSErrors: true,
		serviceWorkers: 'block'
	},
	projects: [
		{ name: 'chromium', use: { ...devices['Desktop Chrome'] } },
		{ name: 'Mobile Chrome', use: { ...devices['Pixel 5'] } }
	],
	webServer: {
		command: 'npm run preview',
		url: 'http://localhost:8788',
		reuseExistingServer: true,
		timeout: 120000
	}
});
