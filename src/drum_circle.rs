use futures::lock::Mutex;
use futures::stream::SplitSink;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol::Message;

pub type DrummerId = String;
pub type CircleId = String;
pub struct Drummer {
    pub id: DrummerId,
    pub stream: SplitSink<WebSocketStream<TcpStream>, Message>,
}
pub type DrumCircle = HashMap<DrummerId, Arc<Mutex<Drummer>>>;
