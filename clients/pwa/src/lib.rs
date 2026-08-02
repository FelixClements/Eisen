use leptos::prelude::*;
use leptos_meta::*;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let (count, set_count) = signal(0);

    view! {
        <Html attr:lang="en" attr:dir="ltr" />
        <Title text="Eisen" />
        <Meta charset="UTF-8" />
        <Meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <Meta name="theme-color" content="#0f766e" />
        <Link rel="manifest" href="/manifest.json" />

        <main>
            <h1>"Eisen"</h1>
            <p>"A local-first, installable Leptos PWA."</p>
            <button on:click=move |_| set_count.update(|n| *n += 1)>
                "Clicked " {count} " times"
            </button>
            <p class="install-hint">
                "This prototype is installable from the browser's address bar or menu."
            </p>
        </main>
    }
}
