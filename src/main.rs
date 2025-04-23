use std::time::Duration;
use std::{collections::HashMap, env, io::Error as IoError, net::SocketAddr, sync::Arc};

use futures::SinkExt;
use futures::StreamExt;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

mod drum_circle;
use crate::drum_circle::{CircleId, DrumCircle, Drummer, DrummerId};

mod message;
use crate::message::{deserialize, serialize, WSPayload};

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use uuid::Uuid;
type WorldOfCircles = HashMap<CircleId, DrumCircle>;
type NextCircleId = Arc<Mutex<u32>>;

async fn process_message(
    msg: Message,
    user_mtx: Arc<Mutex<Drummer>>,
    next_circle_id: NextCircleId,
    world: Arc<Mutex<WorldOfCircles>>,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    if msg.is_text() || msg.is_binary() {
        let m = deserialize(msg.to_text()?);

        match m.name.as_ref() {
            "new_circle" => {
                // create a drum circle, send response payload back
                let mut user = user_mtx.lock().await;

                // Use a block for accessing world to prevent lingering locks
                let response = {
                    let mut circle_idx = next_circle_id.lock().await;
                    let circle_id = circle_idx.to_string();
                    let mut circle = DrumCircle::new();

                    *circle_idx += 1;

                    circle.insert(user.id.clone(), user_mtx.clone());
                    let mut world = world.lock().await;

                    world.insert(circle_id.clone(), circle);

                    let response = WSPayload {
                        name: "circle_created".to_string(),
                        circle_id: circle_id.into(),
                        ..WSPayload::default()
                    };
                    response
                };

                user.stream.send(serialize(response)).await?;
            }
            "join_circle" => {
                // find drum circle, send membership list response back
                let mut user = user_mtx.lock().await;

                let response = {
                    let circle_id = m.circle_id.unwrap();
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
                    response
                };

                user.stream.send(serialize(response)).await?;
            }
            "new_member_rtc_offer" | "new_member_rtc_answer" | "ice_candidate" => {
                let user = user_mtx.lock().await;
                let circle_id = m.circle_id.clone().unwrap();
                let world = world.lock().await;
                let circle = world.get(&circle_id).unwrap();

                let peer_id = m.member_id.clone().unwrap();
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

    Ok(())
}

async fn handle_connection(
    world: Arc<Mutex<WorldOfCircles>>,
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
        if let Err(_) = timeout(
            Duration::from_millis(100),
            process_message(
                msg?,
                user_mtx.clone(),
                next_circle_id.clone(),
                world.clone(),
            ),
        )
        .await
        {
            println!("Failed to process msg within 100ms")
        }
    }

    let user = user_mtx.lock().await;
    let to_delete_id = user.id.clone();
    println!(
        "{} {} disconnected, removing from circle",
        &addr, to_delete_id
    );
    let mut world = world.lock().await;
    let circle = world
        .values_mut()
        .find(|circle| circle.keys().any(|id| *id == user.id));
    match circle {
        Some(c) => {
            c.remove(&to_delete_id);
            println!("Removed user");
        }
        None => println!("No circle found to remove disconnecting user from."),
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let world = Arc::new(Mutex::new(WorldOfCircles::new()));
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
