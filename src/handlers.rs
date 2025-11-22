pub mod model_upload;
pub mod websocket;

pub use model_upload::{list_models, upload_model};
pub use websocket::ws_handler;

use crate::models::{MatchingSession, WsMessage};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use uuid::Uuid;

// matching_id → (player_id → sender)
pub type WsChannels = Arc<Mutex<HashMap<Uuid, HashMap<String, mpsc::UnboundedSender<WsMessage>>>>>;

// マッチング待ちプレイヤー管理: player_id → (matching_id, sender)
pub type WaitingPlayers = Arc<Mutex<HashMap<String, (Uuid, mpsc::UnboundedSender<WsMessage>)>>>;

/// 共有マッチングセッション管理
pub type MatchingSessions = Arc<Mutex<HashMap<Uuid, MatchingSession>>>;
