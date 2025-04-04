use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    SinkExt,
};
use futures::{future, pin_mut, stream::TryStreamExt, StreamExt};

use serde::{Deserialize, Serialize};
use serde_json::Result;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;

use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type UserId = String;
type CircleId = String;
struct CircleMember {
    id: UserId,
    tx: Tx,
}
type DrumCircle = HashMap<UserId, CircleMember>;
type WorldOfCircles = Arc<Mutex<HashMap<CircleId, DrumCircle>>>;
type NextCircleId = Arc<Mutex<u32>>;

#[derive(Serialize, Deserialize)]
struct SDPOffer {
    user_id: UserId,
    sdp: String,
}

#[derive(Serialize, Deserialize, Default)]
struct WSPayload {
    name: String,
    member_id: UserId,
    circle_id: Option<CircleId>,
    members: Option<Vec<String>>,
    sdps: Option<Vec<SDPOffer>>,
    sdp: String,
}

async fn handle_connection(
    world: WorldOfCircles,
    next_circle_id: NextCircleId,
    raw_stream: TcpStream,
    addr: SocketAddr,
) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    let (tx, rx) = unbounded();

    let user = CircleMember {
        id: Uuid::new_v4().to_string(),
        tx,
    };

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        let message = msg.to_text().unwrap();

        println!("Received a message from {}: {}", addr, message);
        let m: WSPayload = serde_json::from_str(message);

        match m.name.as_ref() {
            "new_circle" => {
                // create a drum circle, send response payload back
                let circle_idx = next_circle_id.lock().unwrap();
                let circle_id = circle_idx.to_string();
                let circle = DrumCircle::new();

                circle.insert(user.id, user);
                world.lock().unwrap().insert(circle_id, circle);

                let response = serde_json::to_string(&WSPayload {
                    name: "circle_created".to_string(),
                    circle_id: circle_id.into(),
                    ..WSPayload::default()
                })?;

                outgoing.send(response.into());

                circle_idx += 1;
            }
            "join_circle" => {
                // find drum circle, send membership list response back
            }
            "circle_join_offers" => {
                // find drum circle, forward SDP offer to each member by ID
                // let peers = peer_map.lock().unwrap();

                // // We want to broadcast the message to everyone except ourselves.
                // let broadcast_recipients = peers
                //     .iter()
                //     .filter(|(peer_addr, _)| peer_addr != &&addr)
                //     .map(|(_, ws_sink)| ws_sink);

                // for recp in broadcast_recipients {
                //     recp.unbounded_send(msg.clone()).unwrap();
                // }
            }
            "new_member_rtc_answer" => {
                // find circle and member and forward SDP answer
            }
        }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{} disconnected", &addr);
    // TODO: Remove member from their circle
    // peer_map.lock().unwrap().remove(&addr);
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let world = WorldOfCircles::new(Mutex::new(HashMap::new()));
    let next_circle_id = Arc::new(Mutex::new(0));

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(
            world.clone(),
            next_circle_id.clone(),
            stream,
            addr,
        ));
    }

    Ok(())
}
