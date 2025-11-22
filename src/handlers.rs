pub mod model_upload;
pub mod websocket;

pub use model_upload::{list_models, upload_model};
pub use websocket::ws_handler;

use crate::models::{MatchingSession, WsMessage};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use uuid::Uuid;

// matching_id → (player_id → (sender, session_id))
pub type WsChannels =
    Arc<Mutex<HashMap<Uuid, HashMap<String, (mpsc::UnboundedSender<WsMessage>, Uuid)>>>>;

// マッチング待ちプレイヤー管理: player_id → (matching_id, sender, session_id)
pub type WaitingPlayers =
    Arc<Mutex<HashMap<String, (Uuid, mpsc::UnboundedSender<WsMessage>, Uuid)>>>;

// ロビー待機プレイヤー管理: player_id → (sender, session_id)
pub type LobbyPlayers = Arc<Mutex<HashMap<String, (mpsc::UnboundedSender<WsMessage>, Uuid)>>>;

/// 共有マッチングセッション管理
pub type MatchingSessions = Arc<Mutex<HashMap<Uuid, MatchingSession>>>;
