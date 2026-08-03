import init, { worker_create_vault, worker_open_vault } from './eisen-pwa.js';

let ready = false;
const pending = [];

async function initWorker() {
  await init();
  ready = true;
  for (const msg of pending) {
    handle(msg);
  }
  pending.length = 0;
}

initWorker();

self.onmessage = (event) => {
  if (ready) {
    handle(event.data);
  } else {
    pending.push(event.data);
  }
};

async function handle(data) {
  const { id, action, passphrase } = data;
  try {
    let result;
    if (action === 'create') {
      result = await worker_create_vault(passphrase);
    } else if (action === 'open') {
      result = await worker_open_vault(passphrase);
    } else {
      throw new Error('unknown action: ' + action);
    }
    self.postMessage({ id, status: 'ok', result });
  } catch (err) {
    self.postMessage({ id, status: 'error', error: String(err) });
  }
}
