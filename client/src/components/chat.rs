use leptos::html::Input;
use leptos::*;
use web_sys::KeyboardEvent;

use crate::services::websocket::{ChatEvent, ChatEventType, WebsocketService};

#[component]
pub fn Chat() -> impl IntoView {
    let (events, set_events) = create_signal::<Vec<ChatEvent>>(Vec::new());
    let process_event = move |event: ChatEvent| set_events.update(move |events| events.push(event));
    let (wss, _) = create_signal(WebsocketService::new(process_event));
    let message_input_ref: NodeRef<Input> = create_node_ref();

    let send_message = move |evt: KeyboardEvent| {
        if evt.key_code() == 13 {
            let message = message_input_ref.get().expect("<input> to exist").value();

            if let Ok(_) = wss.get().tx.clone().try_send(message) {
                logging::log!("message sent successfully");
                message_input_ref.get().unwrap().set_value("")
            } else {
                logging::error!("error sending message");
            }
        }
    };

    let render_event = |event: ChatEvent| match event.event_type {
        ChatEventType::MessageSent => {
            view! {
                <MyMessage sender=event.sender.to_string() message=event.message.clone() />
            }
        }
        ChatEventType::MessageReceived => {
            view! {
                <TheirMessage sender=event.sender.to_string() message=event.message.clone() />
            }
        }
        ChatEventType::IJoined => {
            view! {
                <SystemMessage message=String::from(format!("Welcome to the chat {}", event.sender)) />
            }
        }
        ChatEventType::TheyJoined => {
            view! {
                <SystemMessage message=String::from(format!("{} has joined the chat", event.sender)) />
            }
        }
        ChatEventType::TheyDisconnected => {
            view! {
                <SystemMessage message=String::from(format!("{} has left the chat", event.sender)) />
            }
        }
    };

    view! {
        <div class="flex flex-col flex-grow w-full max-w-xl bg-white shadow-xl rounded-lg overflow-hidden">
            <div class="flex flex-col flex-grow h-0 p-4 overflow-auto">
                {
                    move || events.get().iter().map(|e| render_event(e.clone())).collect_view()
                }
            </div>

            <div class="bg-gray-300 p-4">
                <input node_ref={message_input_ref} on:keypress=send_message class="flex items-center h-10 w-full rounded px-3 text-sm" type="text" placeholder="Type your message…" />
            </div>
        </div>
    }
}

#[component]
fn MyMessage(sender: String, message: String) -> impl IntoView {
    view! {
        <div class="flex w-full mt-2 space-x-3 max-w-xs">
            <div>
                <span class="text-xs text-gray-500 leading-none">{sender}</span>
                <div class="bg-gray-300 p-3 rounded-r-lg rounded-bl-lg">
                    <p class="text-sm">{message}</p>
                </div>
            </div>
        </div>
    }
}

#[component]
fn TheirMessage(sender: String, message: String) -> impl IntoView {
    view! {
        <div class="flex w-full mt-2 space-x-3 max-w-xs ml-auto justify-end">
            <div>
                <span class="text-xs text-gray-500 leading-none">{sender}</span>
                <div class="bg-blue-600 text-white p-3 rounded-l-lg rounded-br-lg">
                    <p class="text-sm">{message}</p>
                </div>
            </div>
        </div>
    }
}

#[component]
fn SystemMessage(message: String) -> impl IntoView {
    view! {
        <div class="flex w-full mt-2 space-x-3">
            <span class="text-sm text-orange-500">{message}</span>
        </div>
    }
}
