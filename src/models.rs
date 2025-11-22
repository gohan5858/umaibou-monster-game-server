use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

// 3Dベクトル（位置・方向）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }
}

// 3Dモデルキャラクター情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub model_id: String,          // 3Dモデル識別子
    pub position: Vector3,         // 位置
    pub rotation: Vector3,         // 向き（オイラー角）
    pub hp: i32,                   // HP
    pub max_hp: i32,               // 最大HP
}

impl Character {
    pub fn new(model_id: String) -> Self {
        Self {
            model_id,
            position: Vector3::zero(),
            rotation: Vector3::zero(),
            hp: 100,
            max_hp: 100,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}

// プレイヤー情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub id: String,                // プレイヤーID
    pub selected_model_id: Option<String>, // マッチング作成/参加時に選択したモデルID
    pub character: Option<Character>, // 選択したキャラクター
    pub ready: bool,               // 準備完了フラグ
}

impl Player {
    pub fn new(id: String) -> Self {
        Self {
            id,
            selected_model_id: None,
            character: None,
            ready: false,
        }
    }

    pub fn new_with_model(id: String, model_id: String) -> Self {
        Self {
            id,
            selected_model_id: Some(model_id),
            character: None,
            ready: false,
        }
    }
}

// マッチングセッション状態
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchingStatus {
    Waiting,      // マッチング待ち
    Matched,      // マッチング成立
    Preparing,    // 準備中
    InGame,       // ゲーム中
    Finished,     // 終了
}

// マッチングセッション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingSession {
    pub matching_id: Uuid,
    pub player_a: Player,
    pub player_b: Option<Player>,
    pub status: MatchingStatus,
    pub created_at: DateTime<Utc>,
}

impl MatchingSession {
    pub fn new(player_a_id: String) -> Self {
        Self {
            matching_id: Uuid::new_v4(),
            player_a: Player::new(player_a_id),
            player_b: None,
            status: MatchingStatus::Waiting,
            created_at: Utc::now(),
        }
    }

    pub fn new_with_model(player_a_id: String, model_id: String) -> Self {
        Self {
            matching_id: Uuid::new_v4(),
            player_a: Player::new_with_model(player_a_id, model_id),
            player_b: None,
            status: MatchingStatus::Waiting,
            created_at: Utc::now(),
        }
    }

    pub fn is_both_ready(&self) -> bool {
        self.player_a.ready && self.player_b.as_ref().map_or(false, |p| p.ready)
    }
}

// 操作入力種別
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputAction {
    Move { direction: Vector3, speed: f32 },
    Attack { target_position: Vector3 },
    Rotate { rotation: Vector3 },
}

// プレイヤー操作入力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInput {
    pub player_id: String,
    pub action: InputAction,
    pub timestamp: DateTime<Utc>,
}

// ゲーム状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub matching_id: Uuid,
    pub player_a: Character,
    pub player_b: Character,
    pub timestamp: DateTime<Utc>,
}

// ゲーム結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameResult {
    pub matching_id: Uuid,
    pub winner_id: String,
    pub loser_id: String,
    pub player_a_id: String,
    pub player_b_id: String,
    pub play_time_seconds: i64,
    pub finished_at: DateTime<Utc>,
}

// WebSocketメッセージ種別
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    // クライアント→サーバー
    CreateMatching { selected_model_id: String },  // マッチング作成要求
    JoinMatch { matching_id: Uuid, selected_model_id: String },  // マッチング参加要求
    Ready { selected_model_id: String },
    Input { action: InputAction },
    StateUpdate { position: Vector3, rotation: Vector3 },

    // サーバー→クライアント
    MatchingCreated {
        matching_id: Uuid,
        current_matchings: Vec<Uuid>,  // 自分以外のマッチング一覧
        timestamp: DateTime<Utc>,
    },
    UpdateMatchings {
        current_matchings: Vec<Uuid>,  // 現在のマッチング一覧
        timestamp: DateTime<Utc>,
    },
    MatchingEstablished {
        matching_id: Uuid,
        opponent_id: String,
        opponent_model: Option<crate::db::models::Model3D>,
        timestamp: DateTime<Utc>,
    },
    MatchingSuccess {
        matching_id: Uuid,
        opponent_id: String,
        timestamp: DateTime<Utc>,
    },
    OpponentCharacterSelected {
        character: Character,
        timestamp: DateTime<Utc>,
    },
    GameStart {
        opponent_character: Character,  // 相手のキャラクター情報のみ
        your_player_id: String,         // 自分のプレイヤーID（識別用）
        timestamp: DateTime<Utc>,
    },
    OpponentStateUpdate {
        opponent: Character,
        timestamp: DateTime<Utc>,  // サーバー送信時刻（レイテンシ計測用）
    },
    GameEnd {
        result: GameResult,
        timestamp: DateTime<Utc>,
    },

    // エラー
    Error { message: String },
}

// REST APIリクエスト/レスポンス
#[derive(Debug, Deserialize)]
pub struct CreateMatchingRequest {
    pub player_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMatchingResponse {
    pub matching_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct JoinMatchingRequest {
    pub matching_id: Uuid,
    pub player_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinMatchingResponse {
    pub success: bool,
    pub message: Option<String>,
}

// 3Dモデルアップロード関連
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadModelResponse {
    pub model_id: String,
    pub file_name: String,
    pub file_size: i64,
}
