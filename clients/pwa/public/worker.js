import init, {
  worker_create_vault,
  worker_open_vault,
  worker_persist,
  worker_create_recovery_package,
  worker_restore_recovery_package,
  worker_export_vault,
  worker_import_vault,
} from './eisen-pwa.js';

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
  const { id, action, passphrase, payload } = data;
  try {
    let result;
    if (action === 'create') {
      result = await worker_create_vault(passphrase);
    } else if (action === 'open') {
      result = await worker_open_vault(passphrase);
    } else if (action === 'persist') {
      result = worker_persist();
    } else if (action === 'recovery') {
      result = await worker_create_recovery_package(passphrase, payload || '');
    } else if (action === 'restore') {
      result = await worker_restore_recovery_package(payload || '', passphrase);
    } else if (action === 'export') {
      result = await worker_export_vault(passphrase);
    } else if (action === 'import') {
      result = await worker_import_vault(passphrase, payload || '');
    } else {
      throw new Error('unknown action: ' + action);
    }
    self.postMessage({ id, status: 'ok', result });
  } catch (err) {
    self.postMessage({ id, status: 'error', error: String(err) });
  }
}
