use js_sys::Reflect;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{MessageEvent, Worker, WorkerOptions, WorkerType};

pub struct WorkerClient {
    worker: Worker,
    pending: Rc<RefCell<HashMap<u64, Box<dyn Fn(Result<String, String>) + Send>>>>,
    next_id: Rc<RefCell<u64>>,
}

impl WorkerClient {
    pub fn new() -> Result<Self, JsValue> {
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let worker = Worker::new_with_options("/worker.js", &options)?;

        let pending: Rc<RefCell<HashMap<u64, Box<dyn Fn(Result<String, String>) + Send>>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let next_id = Rc::new(RefCell::new(1u64));

        let pending_for_handler = pending.clone();
        let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
            let data = event.data();
            if data.is_undefined() || data.is_null() {
                return;
            }
            let id = Reflect::get(&data, &"id".into())
                .ok()
                .and_then(|v| v.as_f64())
                .map(|f| f as u64)
                .unwrap_or(0);
            let status = Reflect::get(&data, &"status".into())
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            let result = Reflect::get(&data, &"result".into())
                .ok()
                .and_then(|v| v.as_string());
            let error = Reflect::get(&data, &"error".into())
                .ok()
                .and_then(|v| v.as_string());

            if let Some(callback) = pending_for_handler.borrow_mut().remove(&id) {
                if status == "ok" {
                    callback(Ok(result.unwrap_or_default()));
                } else {
                    callback(Err(error.unwrap_or_else(|| "worker error".into())));
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();

        Ok(Self {
            worker,
            pending,
            next_id,
        })
    }

    pub fn send(
        &self,
        action: &str,
        passphrase: &str,
        callback: Box<dyn Fn(Result<String, String>) + Send>,
    ) {
        let id = *self.next_id.borrow();
        *self.next_id.borrow_mut() = id.wrapping_add(1);
        self.pending.borrow_mut().insert(id, callback);

        let data = js_sys::Object::new();
        _ = Reflect::set(&data, &"id".into(), &JsValue::from_f64(id as f64));
        _ = Reflect::set(&data, &"action".into(), &JsValue::from_str(action));
        _ = Reflect::set(&data, &"passphrase".into(), &JsValue::from_str(passphrase));

        if let Err(e) = self.worker.post_message(&data) {
            log::error!("failed to post to worker: {:?}", e);
            if let Some(callback) = self.pending.borrow_mut().remove(&id) {
                callback(Err("failed to post to worker".into()));
            }
        }
    }
}

// wasm32 is single-threaded; these JS handles are only accessed from the main
// thread that created them. Marking them Send + Sync lets them satisfy Leptos
// reactive bounds. This is safe only because the wasm32 target has no threads.
unsafe impl Send for WorkerClient {}
unsafe impl Sync for WorkerClient {}
