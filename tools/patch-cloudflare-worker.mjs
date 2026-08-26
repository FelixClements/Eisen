import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';

const workerPath = path.join('.svelte-kit', 'cloudflare', '_worker.js');
let source = readFileSync(workerPath, 'utf8');

if (source.includes('async scheduled(')) {
	console.log('cloudflare worker already patched for scheduled cron');
	process.exit(0);
}

const marker = 'var worker = {\n  async fetch(req, env, context) {';
const scheduled = `var worker = {
  async scheduled(controller, env, context) {
    if (!env.CRON_SECRET) return;
    const origin = env.BETTER_AUTH_URL || "https://localhost";
    const req = new Request(new URL("/api/push/cron", origin), {
      headers: { Authorization: "Bearer " + env.CRON_SECRET }
    });
    await this.fetch(req, env, context);
  },
  async fetch(req, env, context) {`;

if (!source.includes(marker)) {
	console.error('Could not patch _worker.js: unexpected worker format');
	process.exit(1);
}

source = source.replace(marker, scheduled);
writeFileSync(workerPath, source);
console.log('patched cloudflare worker with scheduled cron handler');
