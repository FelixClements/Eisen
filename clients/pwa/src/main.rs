use leptos::prelude::*;
use leptos_meta::*;

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Html attr:lang="en" attr:dir="ltr" />
        <Title text="Eisen" />
        <Meta charset="UTF-8" />
        <Meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <Meta name="theme-color" content="#0f766e" />
        <Link rel="manifest" href="/manifest.json" />

        <main>
            <h1>"Eisen"</h1>
            <p>"A local-first, installable PWA for secure task management."</p>
            <p class="install-hint">"Install this app from your browser address bar or menu."</p>
        </main>
    }
}
