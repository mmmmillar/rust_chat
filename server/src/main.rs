use futures_util::{SinkExt, StreamExt, TryFutureExt};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::filters::ws::WebSocket;
use warp::ws::{Message, Ws};
use warp::Filter;

type Clients = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Serialize)]
enum ChatEventType {
    IJoined,
    TheyJoined,
    MessageSent,
    MessageReceived,
    TheyDisconnected,
}

#[derive(Clone, Serialize)]
struct ChatEvent {
    sender: usize,
    event_type: ChatEventType,
    message: String,
}

#[tokio::main]
async fn main() {
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    let chat = warp::path!("chat")
        .and(warp::ws())
        .and(warp::any().map(move || clients.clone()))
        .map(|ws: Ws, clients: Clients| ws.on_upgrade(move |socket| on_connect(socket, clients)));

    warp::serve(chat).run(([127, 0, 0, 1], 3030)).await;
}

async fn on_connect(ws: WebSocket, clients: Clients) {
    // thread safe user id
    let id = NEXT_USER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // split the socket into a sender and receiver of messages.
    let (mut ws_sender, mut ws_receiver) = ws.split();

    // use an unbounded channel to handle buffering and flushing of messages to the websocket...
    let (client_sender, client_receiver) = mpsc::unbounded_channel();
    let mut client_receiver = UnboundedReceiverStream::new(client_receiver);

    // send messages to client
    tokio::task::spawn(async move {
        while let Some(message) = client_receiver.next().await {
            ws_sender
                .send(message)
                .unwrap_or_else(|e| {
                    eprintln!("websocket send error: {}", e);
                })
                .await;
        }
    });

    clients.write().await.insert(id, client_sender);

    register(id, &clients).await;

    eprintln!("new client connected: {}", id);
    eprintln!("active clients: {:?}", clients.read().await.keys());

    // when message is received on the websocket from the client...
    while let Some(result) = ws_receiver.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("error receiving ws message for id: {}): {}", id, e);
                break;
            }
        };

        match msg.is_close() {
            true => {
                eprintln!("client disconnected: {}", id);
                disconnect(id, &clients).await;
            }
            _ => broadcast_message(id, msg.to_str().unwrap(), &clients).await,
        }
    }

    eprintln!("remove {} from clients", id);
    clients.write().await.remove(&id);
    eprintln!("active clients: {:?}", clients.read().await.keys());
}

async fn register(sender: usize, clients: &Clients) {
    clients.read().await.iter().for_each(|(client_id, tx)| {
        let event = ChatEvent {
            sender,
            event_type: if &sender == client_id {
                ChatEventType::IJoined
            } else {
                ChatEventType::TheyJoined
            },
            message: String::from(""),
        };

        let message = Message::text(serde_json::to_string(&event).unwrap());

        eprintln!(
            "sending message from {} to {}: {:?}",
            &event.sender, client_id, &event.message
        );

        if let Err(_) = tx.send(message) {}
    })
}

async fn disconnect(sender: usize, clients: &Clients) {
    clients
        .read()
        .await
        .iter()
        .filter(|(&client_id, _)| client_id != sender)
        .for_each(|(client_id, tx)| {
            let event = ChatEvent {
                sender,
                event_type: ChatEventType::TheyDisconnected,
                message: String::from(""),
            };

            let message = Message::text(serde_json::to_string(&event).unwrap());

            eprintln!(
                "sending message from {} to {}: {:?}",
                &event.sender, client_id, &event.message
            );

            if let Err(_) = tx.send(message) {}
        })
}

async fn broadcast_message(sender: usize, msg: &str, clients: &Clients) {
    clients.read().await.iter().for_each(|(client_id, tx)| {
        let event = ChatEvent {
            sender,
            event_type: if &sender == client_id {
                ChatEventType::MessageSent
            } else {
                ChatEventType::MessageReceived
            },
            message: String::from(msg),
        };

        let message = Message::text(serde_json::to_string(&event).unwrap());

        eprintln!(
            "sending message from {} to {}: {:?}",
            &event.sender, client_id, &event.message
        );

        if let Err(_) = tx.send(message) {}
    })
}
