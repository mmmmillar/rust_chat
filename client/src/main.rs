mod components;
mod services;

use crate::components::chat::*;
use leptos::*;

fn main() {
    mount_to_body(|| view! { <App/> });
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Chat />
    }
}
