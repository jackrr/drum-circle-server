use futures::lock::Mutex;
use futures::stream::SplitSink;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;

pub type UserId = String;
pub type CircleId = String;
pub struct CircleMember {
    pub id: UserId,
    pub stream: SplitSink<WebSocketStream<TcpStream>, Message>,
}
pub type DrumCircle = HashMap<UserId, Arc<Mutex<CircleMember>>>;
