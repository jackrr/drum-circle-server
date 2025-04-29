use std::collections::HashSet;
use std::{collections::HashMap, env, io::Error as IoError, net::SocketAddr, sync::Arc};

use futures::SinkExt;
use futures::StreamExt;
use futures::stream::SplitSink;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message;

use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use uuid::Uuid;

type UserId = String;
type CircleId = String;
type SDP = String;
type ICE = String;

#[derive(Serialize, Deserialize, Debug)]
pub struct SDPOffer {
    user_id: String,
    sdp: SDP,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct WSPayload {
    pub name: String,
    pub member_id: Option<UserId>,
    pub circle_id: Option<CircleId>,
    pub members: Option<Vec<String>>,
    pub sdps: Option<Vec<SDPOffer>>,
    pub sdp: Option<SDP>,
    pub ice: Option<ICE>,
}

// pub type ParsedPayload = Result<WSPayload>;

fn deserialize(s: &str) -> WSPayload {
    match serde_json::from_str(s) {
        Err(e) => panic!("Failed to parse {:?}", e),
        Ok(payload) => payload,
    }
}

fn serialize(payload: WSPayload) -> Message {
    match serde_json::to_string(&payload) {
        Err(e) => panic!("Failed to serialize {:?}", e),
        Ok(json) => Message::text(json),
    }
}


#[derive(Debug)]
struct User {
    // id: UserId,
    stream: SplitSink<WebSocketStream<TcpStream>, Message>,
}

#[derive(Debug)]
struct Circle {
    id: CircleId,
    members: HashSet<UserId>,
}

#[derive(Debug)]
struct GlobalState {
    circles: HashMap<CircleId, Circle>,
    next_circle_id: u32,
    users: HashMap<UserId, User>,
}

type WorldMtx = Arc<Mutex<GlobalState>>;

async fn create_circle(
    world: WorldMtx,
) -> CircleId {
    let mut world = world.lock().await;

    let circle_id = world.next_circle_id.to_string();
    world.next_circle_id += 1;

    let circle = Circle {
        id: circle_id.clone(),
        members: HashSet::new()
    };

    world.circles.insert(circle.id.clone(), circle);

    circle_id
}

async fn add_circle_member(world: WorldMtx, circle_id: CircleId, user_id: UserId) {
    let mut world = world.lock().await;

    let circle = world.circles.get_mut(&circle_id).unwrap();
    circle.members.insert(user_id);
}

async fn send_to_user(world: WorldMtx, user_id: UserId, message: WSPayload) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let mut world = world.lock().await;

    let user = world.users.get_mut(&user_id).unwrap();
    user.stream.send(serialize(message)).await?;
    
    Ok(())
}

async fn process_message(
    msg: Message,
    user_id: UserId,
    world_mtx: Arc<Mutex<GlobalState>>,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    if msg.is_text() || msg.is_binary() {
        let m = deserialize(msg.to_text()?);

        match m.name.as_ref() {
            "new_circle" => {
                let circle_id = {
                    let circle_id = create_circle(world_mtx.clone()).await;
                    add_circle_member(world_mtx.clone(), circle_id.clone(), user_id.clone()).await;
                    circle_id
                };

                let response = WSPayload {
                    name: "circle_created".to_string(),
                    circle_id: circle_id.into(),
                    ..WSPayload::default()
                };

                send_to_user(world_mtx.clone(), user_id.clone(), response).await?;
            }
            "join_circle" => {
                // find drum circle, send membership list response back
                let circle_id = m.circle_id.unwrap();

                let response = {
                    let mut world = world_mtx.lock().await;

                    // TODO: handle invalid/non-existent circle_id (error payload?)
                    let circle = world.circles.get_mut(&circle_id).unwrap();

                    // TODO: less clunky way to get a vec of strings of user ids?
                    let mut members: Vec<UserId> = Vec::new();
                    for key in circle.members.iter() {
                        members.push(key.clone());
                    }

                    circle.members.insert(user_id.clone());

                    let response = WSPayload {
                        members: members.into(),
                        name: "circle_discovery".to_string(),
                        circle_id: circle_id.into(),
                        ..WSPayload::default()
                    };

                    response                    
                };

                send_to_user(world_mtx.clone(), user_id.clone(), response).await?;
            }
            "new_member_rtc_offer" | "new_member_rtc_answer" | "ice_candidate" => {
                let peer_id = m.member_id.clone().unwrap();
                let response = WSPayload {
                    member_id: user_id.clone().into(),
                    ..m
                };
                // TODO: if peer gone, send disconnect
                send_to_user(world_mtx.clone(), peer_id.clone(), response).await?;
            }
            _ => {
                println!("Unexpected message name: {}", m.name);
            }
        }
    }

    Ok(())
}

async fn handle_connection(
    world_mtx: Arc<Mutex<GlobalState>>,
    raw_stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    let (outgoing, mut incoming) = ws_stream.split();

    let user_id = Uuid::new_v4().to_string();

    let user = User {
        // id: user_id.clone(),
        stream: outgoing,
    };

    // Block to scope mutex guard
    {
        let mut world = world_mtx.lock().await;
        world.users.insert(user_id.clone(), user);
    }

    while let Some(msg) = incoming.next().await {
        process_message(
            msg?,
            user_id.clone(),
            world_mtx.clone(),
        ).await?;
    }

    // User disconnected, time to clean up
    println!(
        "{} {} disconnected, removing from circle",
        &addr, user_id
    );
    let mut world = world_mtx.lock().await;
    let circle = world.circles
        .values_mut()
        .find(|circle| circle.members.iter().any(|id| *id == user_id));
    match circle {
        Some(c) => {
            c.members.remove(&user_id);
        },
        None => println!("No circle found to remove disconnecting user from."),
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    let world = Arc::new(Mutex::new(GlobalState {
        circles: HashMap::new(),
        next_circle_id: 1,
        users: HashMap::new()
    }));

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(
            world.clone(),
            stream,
            addr,
        ));
    }

    Ok(())
}
