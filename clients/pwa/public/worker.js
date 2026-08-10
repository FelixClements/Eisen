import init, {
  worker_create_vault,
  worker_open_vault,
  worker_persist,
  worker_create_recovery_package,
  worker_restore_recovery_package,
  worker_export_vault,
  worker_import_vault,
  worker_list_tasks,
  worker_create_task,
  worker_update_task,
  worker_complete_task,
  worker_delete_task,
  worker_restore_task,
  worker_move_task,
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
    } else if (action === 'list') {
      result = worker_list_tasks();
    } else if (action === 'create_task') {
      const obj = JSON.parse(payload || '{}');
      result = worker_create_task(obj.title, obj.notes, obj.quadrant);
    } else if (action === 'update_task') {
      const obj = JSON.parse(payload || '{}');
      result = worker_update_task(obj.id, obj.title, obj.notes, obj.quadrant);
    } else if (action === 'complete_task') {
      result = worker_complete_task(payload);
    } else if (action === 'delete_task') {
      result = worker_delete_task(payload);
    } else if (action === 'restore_task') {
      result = worker_restore_task(payload);
    } else if (action === 'move_task') {
      const obj = JSON.parse(payload || '{}');
      result = worker_move_task(obj.id, obj.quadrant);
    } else {
      throw new Error('unknown action: ' + action);
    }
    self.postMessage({ id, status: 'ok', result });
  } catch (err) {
    self.postMessage({ id, status: 'error', error: String(err) });
  }
}
