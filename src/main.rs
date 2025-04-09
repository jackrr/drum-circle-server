use std::{collections::HashMap, env, io::Error as IoError, net::SocketAddr, sync::Arc};

use futures::lock::Mutex;
use futures::SinkExt;
use futures::StreamExt;

mod drum_circle;
use crate::drum_circle::{CircleId, DrumCircle, Drummer, DrummerId};

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

    let user = Drummer {
        id: Uuid::new_v4().to_string(),
        stream: outgoing,
    };
    let user_wrapper = Arc::new(Mutex::new(user));

    let user_mtx = user_wrapper.clone();
    while let Some(msg) = incoming.next().await {
        let msg = msg?;
        if msg.is_text() || msg.is_binary() {
            let m = deserialize(msg.to_text()?);

            match m.name.as_ref() {
                "new_circle" => {
                    println!("Making a new circle");
                    // create a drum circle, send response payload back
                    let mut user = user_mtx.lock().await;
                    let mut circle_idx = next_circle_id.lock().await;
                    let circle_id = circle_idx.to_string();
                    let mut circle = DrumCircle::new();

                    *circle_idx += 1;

                    circle.insert(user.id.clone(), user_mtx.clone());
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
                    let mut user = user_mtx.lock().await;
                    let circle_id = m.circle_id.unwrap();
                    println!("Joining circle {}", circle_id);
                    let mut world = world.lock().await;
                    let circle = world.get_mut(&circle_id).unwrap();

                    // TODO: less clunky way to get a vec of strings of user ids?
                    let mut members: Vec<DrummerId> = Vec::new();
                    for key in circle.keys() {
                        members.push(key.clone());
                    }

                    circle.insert(user.id.clone(), user_mtx.clone());

                    let response = WSPayload {
                        members: members.into(),
                        name: "circle_discovery".to_string(),
                        circle_id: circle_id.into(),
                        ..WSPayload::default()
                    };

                    user.stream.send(serialize(response)).await?;
                }
                "new_member_rtc_offer" | "new_member_rtc_answer" | "ice_candidate" => {
                    println!("Got message: {:?}", m);
                    let user = user_mtx.lock().await;
                    let circle_id = m.circle_id.clone().unwrap();
                    let world = world.lock().await;
                    let circle = world.get(&circle_id).unwrap();
                    let peer_id = m.member_id.clone().unwrap();
                    println!("Peer id is {}", peer_id);
                    let mut peer = circle.get(&peer_id).unwrap().lock().await;

                    // Forward original payload, swapping originator's id for dest id
                    let response = WSPayload {
                        member_id: user.id.clone().into(),
                        ..m
                    };
                    peer.stream.send(serialize(response)).await?;
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
