use futures::stream::SplitSink;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;

pub type DrummerId = String;
pub type CircleId = String;
#[derive(Debug)]
pub struct Drummer {
    pub id: DrummerId,
    pub stream: SplitSink<WebSocketStream<TcpStream>, Message>,
}
pub type DrumCircle = HashMap<DrummerId, Arc<Mutex<Drummer>>>;
