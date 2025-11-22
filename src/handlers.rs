pub mod matching;
pub mod model_upload;
pub mod websocket;

pub use matching::{create_matching, join_matching, MatchingSessions};
pub use model_upload::upload_model;
pub use websocket::ws_handler;

use crate::models::WsMessage;
use tokio::sync::mpsc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// matching_id → (player_id → sender)
pub type WsChannels = Arc<Mutex<HashMap<Uuid, HashMap<String, mpsc::UnboundedSender<WsMessage>>>>>;

// マッチング待ちプレイヤー管理: player_id → (matching_id, sender)
pub type WaitingPlayers = Arc<Mutex<HashMap<String, (Uuid, mpsc::UnboundedSender<WsMessage>)>>>;
