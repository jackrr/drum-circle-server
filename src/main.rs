use std::{collections::HashMap, env, io::Error as IoError, net::SocketAddr, sync::Arc};

use futures::lock::Mutex;
use futures::{channel::mpsc::unbounded, SinkExt};
use futures::{future, pin_mut, stream::TryStreamExt, StreamExt};

mod drum_circle;
use crate::drum_circle::{CircleId, CircleMember, DrumCircle, UserId};

mod message;
use crate::message::{deserialize, serialize, WSPayload};

use tokio::net::{TcpListener, TcpStream};

use uuid::Uuid;
type WorldOfCircles = Arc<Mutex<HashMap<CircleId, DrumCircle>>>;
type NextCircleId = Arc<Mutex<u32>>;

async fn handle_connection(
    world: WorldOfCircles,
    next_circle_id: NextCircleId,
    raw_stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    let (outgoing, mut incoming) = ws_stream.split();

    let user = CircleMember {
        id: Uuid::new_v4().to_string(),
        stream: outgoing,
    };
    let user_wrapper = Arc::new(Mutex::new(user));

    let user_ptr = user_wrapper.clone();
    while let Some(msg) = incoming.next().await {
        let msg = msg?;
        if msg.is_text() || msg.is_binary() {
            let m = deserialize(msg.to_text()?);

            match m.name.as_ref() {
                "new_circle" => {
                    println!("Making a new circle");
                    // create a drum circle, send response payload back
                    let mut user = user_ptr.lock().await;
                    let mut circle_idx = next_circle_id.lock().await;
                    let circle_id = circle_idx.to_string();
                    let mut circle = DrumCircle::new();

                    *circle_idx += 1;

                    circle.insert(user.id.clone(), user_ptr.clone());
                    world.lock().await.insert(circle_id.clone(), circle);

                    let response = WSPayload {
                        name: "circle_created".to_string(),
                        circle_id: circle_id.into(),
                        ..WSPayload::default()
                    };

                    user.stream.send(serialize(response)).await?;
                }
                "join_circle" => {
                    // find drum circle, send membership list response back
                    let circle_id = m.circle_id.unwrap();
                    println!("Joining circle {}", circle_id);
                    let circle = world.lock().await.get(&circle_id).unwrap();

		    // gah i don't know rust
                    let members: Vec<UserId> = new vec[0];
                    for key in circle.keys() {
                        members.push(key.clone());
                    }

                    let response = WSPayload {
                        members: circle.keys().cloned().collect::<Vec<String>>(),
                        name: "circle_discovery".to_string(),
                        circle_id: circle_id.into(),
                        ..WSPayload::default()
                    };
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
                _ => {
                    println!("Unexpected message name: {}", m.name);
                }
            }
        }
    }

    println!("{} disconnected", &addr);
    // TODO: Remove member from their circle
    // peer_map.lock().unwrap().remove(&addr);
    Ok(())
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
