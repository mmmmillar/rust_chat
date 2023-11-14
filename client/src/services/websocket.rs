use futures::{channel::mpsc::Sender, SinkExt, StreamExt};
use leptos::logging;
use reqwasm::websocket::{futures::WebSocket, Message};
use serde::Deserialize;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
pub struct WebsocketService {
    pub tx: Sender<String>,
}

#[derive(Clone, Deserialize, PartialEq)]
pub enum ChatEventType {
    IJoined,
    TheyJoined,
    MessageSent,
    MessageReceived,
    TheyDisconnected,
}

#[derive(Clone, Deserialize)]
pub struct ChatEvent {
    pub sender: usize,
    pub event_type: ChatEventType,
    pub message: String,
}

impl WebsocketService {
    pub fn new<F>(on_event: F) -> Self
    where
        F: Fn(ChatEvent) + 'static,
    {
        let ws = WebSocket::open("ws://127.0.0.1:3030/chat").unwrap();
        logging::log!("Opening websocket");

        let (mut write, mut read) = ws.split();

        let (tx, mut rx) = futures::channel::mpsc::channel::<String>(1000);

        spawn_local(async move {
            while let Some(s) = rx.next().await {
                logging::log!("got event from channel! {}", s);
                write.send(Message::Text(s)).await.unwrap();
            }
        });

        spawn_local(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(data)) => {
                        logging::log!("from websocket: {}", data);
                        match serde_json::from_str::<ChatEvent>(&data) {
                            Ok(event) => on_event(event),
                            Err(_) => logging::error!("unrecognized message format"),
                        }
                    }
                    Ok(Message::Bytes(b)) => {
                        let decoded = std::str::from_utf8(&b);
                        if let Ok(val) = decoded {
                            logging::log!("from websocket: {}", val);
                        }
                    }
                    Err(e) => {
                        logging::error!("ws: {:?}", e)
                    }
                }
            }
            logging::log!("WebSocket Closed");
        });

        Self { tx }
    }
}
