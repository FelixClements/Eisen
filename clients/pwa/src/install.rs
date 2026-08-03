use js_sys::{Function, Reflect};
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Event, MediaQueryList};

struct JsClosure(Closure<dyn FnMut(Event)>);

// wasm32 is single-threaded; these closures are only accessed from the main
// thread. Marking the wrapper Send + Sync lets it satisfy Leptos bounds.
unsafe impl Send for JsClosure {}
unsafe impl Sync for JsClosure {}

#[component]
pub fn InstallPrompt() -> impl IntoView {
    let (can_install, set_can_install) = signal(false);
    let (is_installed, set_is_installed) = signal(false);
    let (show_url, set_show_url) = signal(false);
    let (copied, set_copied) = signal(false);

    let window = web_sys::window().expect("window");
    let window = StoredValue::new_local(window);

    // Already running as an installed app?
    let standalone = window
        .with_value(|w| w.match_media("(display-mode: standalone)"))
        .ok()
        .flatten()
        .map(|m: MediaQueryList| m.matches())
        .unwrap_or(false);
    let ios_standalone = window
        .with_value(|w| {
            Reflect::get(&w.navigator(), &"standalone".into())
                .ok()
                .and_then(|v| v.as_bool())
        })
        .unwrap_or(false);
    if standalone || ios_standalone {
        set_is_installed.set(true);
    }

    let deferred = StoredValue::new_local(None::<JsValue>);

    // Listen for beforeinstallprompt / appinstalled.
    Effect::new({
        let window = window.clone();
        let set_can_install = set_can_install.clone();
        let set_is_installed = set_is_installed.clone();
        let deferred = deferred.clone();
        move || {
            let before = Closure::wrap(Box::new(move |e: Event| {
                e.prevent_default();
                let js: JsValue = e.unchecked_into();
                deferred.set_value(Some(js));
                set_can_install.set(true);
            }) as Box<dyn FnMut(Event)>);

            let installed = Closure::wrap(Box::new(move |_: Event| {
                set_is_installed.set(true);
                set_can_install.set(false);
            }) as Box<dyn FnMut(Event)>);

            let before: &'static JsClosure = Box::leak(Box::new(JsClosure(before)));
            let installed: &'static JsClosure = Box::leak(Box::new(JsClosure(installed)));

            window.with_value(|w| {
                _ = w.add_event_listener_with_callback(
                    "beforeinstallprompt",
                    before.0.as_ref().unchecked_ref(),
                );
                _ = w.add_event_listener_with_callback(
                    "appinstalled",
                    installed.0.as_ref().unchecked_ref(),
                );
            });

            on_cleanup(move || {
                window.with_value(|w| {
                    _ = w.remove_event_listener_with_callback(
                        "beforeinstallprompt",
                        before.0.as_ref().unchecked_ref(),
                    );
                    _ = w.remove_event_listener_with_callback(
                        "appinstalled",
                        installed.0.as_ref().unchecked_ref(),
                    );
                });
            });
        }
    });

    let on_install = {
        let deferred = deferred.clone();
        move |_| {
            deferred.with_value(|prompt| {
                if let Some(prompt) = prompt {
                    if let Ok(prompt_fn) = Reflect::get(prompt, &"prompt".into()) {
                        if let Ok(prompt_fn) = prompt_fn.dyn_into::<Function>() {
                            let _ = prompt_fn.call0(prompt);
                        }
                    }
                }
            });
        }
    };

    let location = window.with_value(|w| w.location().href().unwrap_or_default());
    let url = {
        let location = location.clone();
        move || location.clone()
    };
    let url_for_show_url = url.clone();

    let on_copy = {
        let window = window.clone();
        let location = location.clone();
        let set_copied = set_copied.clone();
        move |_| {
            let url = location.clone();
            window.with_value(|w| {
                let navigator = w.navigator();
                if let Some(clipboard) = Reflect::get(&navigator, &"clipboard".into())
                    .ok()
                    .and_then(|v| v.dyn_into::<web_sys::Clipboard>().ok())
                {
                    spawn_local(async move {
                        let promise = clipboard.write_text(&url);
                        let _ = JsFuture::from(promise).await;
                    });
                }
            });
            set_copied.set(true);
        }
    };
    let on_copy_for_show_url = on_copy.clone();

    view! {
        <div class="install-prompt">
            {move || {
                if is_installed.get() {
                    view! {
                        <p class="install-note">
                            "Eisen is installed. Open it from your home screen or app launcher."
                        </p>
                    }
                    .into_any()
                } else if can_install.get() {
                    view! {
                        <div class="install-offer">
                            <p>"Install Eisen for offline use and easy access from your home screen."</p>
                            <button on:click=on_install.clone()>"Install Eisen"</button>
                            <button
                                class="secondary"
                                on:click=move |_| set_show_url.set(!show_url.get())
                            >
                                "Use URL instead"
                            </button>
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <div class="install-fallback">
                            <p>"Your browser or policy does not allow installation. Eisen still works from this URL."</p>
                            <p class="url">{url()}</p>
                            <button on:click=on_copy.clone()>"Copy URL"</button>
                            {move || if copied.get() {
                                view! { <span class="copied-hint">"Copied!"</span> }.into_any()
                            } else {
                                view! {}.into_any()
                            }}
                        </div>
                    }
                    .into_any()
                }
            }}

            {move || if show_url.get() {
                view! {
                    <div class="install-fallback">
                        <p>"Eisen works from this URL without installing. You can bookmark it for quick access."</p>
                        <p class="url">{url_for_show_url()}</p>
                        <button on:click=on_copy_for_show_url.clone()>"Copy URL"</button>
                    </div>
                }
                .into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
