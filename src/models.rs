use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

// 3Dモデルキャラクター情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub model_id: String,  // 3Dモデル識別子
    pub position: Vector3, // 位置
    pub rotation: Vector3, // 向き（オイラー角）
    pub hp: i32,           // HP
    pub max_hp: i32,       // 最大HP
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
    pub id: String,                        // プレイヤーID
    pub username: Option<String>,          // ユーザー名
    pub selected_model_id: Option<String>, // マッチング作成/参加時に選択したモデルID
    pub character: Option<Character>,      // 選択したキャラクター
    pub ready: bool,                       // 準備完了フラグ
}

impl Player {
    pub fn new(id: String) -> Self {
        Self {
            id,
            username: None,
            selected_model_id: None,
            character: None,
            ready: false,
        }
    }

    pub fn new_with_username(id: String, username: Option<String>) -> Self {
        Self {
            id,
            username,
            selected_model_id: None,
            character: None,
            ready: false,
        }
    }
}

// マッチングセッション状態
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchingStatus {
    Waiting,   // マッチング待ち
    Matched,   // マッチング成立
    Preparing, // 準備中
    InGame,    // ゲーム中
    Finished,  // 終了
}

// マッチングセッション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingSession {
    pub matching_id: Uuid,
    pub creator_username: Option<String>, // 作成者のユーザー名
    pub player_a: Player,
    pub player_b: Option<Player>,
    pub status: MatchingStatus,
    pub created_at: DateTime<Utc>,
    pub last_active_at: Option<DateTime<Utc>>, // 最後のプレイヤーが切断した時刻
    pub is_battle_started: bool,               // バトル開始済みフラグ
    pub is_battle_finished: bool,              // バトル終了済みフラグ
}

impl MatchingSession {
    pub fn new(player_a_id: String) -> Self {
        Self {
            matching_id: Uuid::new_v4(),
            creator_username: None,
            player_a: Player::new(player_a_id),
            player_b: None,
            status: MatchingStatus::Waiting,
            created_at: Utc::now(),
            last_active_at: None,
            is_battle_started: false,
            is_battle_finished: false,
        }
    }

    pub fn new_with_username(player_a_id: String, username: Option<String>) -> Self {
        Self {
            matching_id: Uuid::new_v4(),
            creator_username: username.clone(),
            player_a: Player::new_with_username(player_a_id, username),
            player_b: None,
            status: MatchingStatus::Waiting,
            created_at: Utc::now(),
            last_active_at: None,
            is_battle_started: false,
            is_battle_finished: false,
        }
    }

    pub fn is_both_ready(&self) -> bool {
        self.player_a.ready && self.player_b.as_ref().map_or(false, |p| p.ready)
    }

    /// マッチングが有効かどうか判定
    /// - バトル終了後は無効
    /// - 両方切断してから60秒経過したら無効
    pub fn is_valid(&self) -> bool {
        if self.is_battle_finished {
            return false;
        }

        if let Some(last_active) = self.last_active_at {
            let now = Utc::now();
            let duration = now.signed_duration_since(last_active);
            if duration.num_seconds() > 60 {
                return false;
            }
        }

        true
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

// マッチング情報（一覧表示用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingInfo {
    pub matching_id: Uuid,
    pub creator_username: Option<String>,
    pub created_at: DateTime<Utc>,
    pub status: MatchingStatus,
}

// WebSocketメッセージ種別
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    // クライアント→サーバー
    CreateMatching {
        username: Option<String>,
    }, // マッチング作成要求
    JoinMatch {
        matching_id: Uuid,
    }, // マッチング参加要求
    Ready {
        selected_model_id: String,
    },
    Input {
        action: InputAction,
    },
    StateUpdate {
        position: Vector3,
        rotation: Vector3,
    },

    // サーバー→クライアント
    MatchingCreated {
        matching_id: Uuid,
        current_matchings: Vec<MatchingInfo>, // 自分以外のマッチング一覧
        timestamp: DateTime<Utc>,
    },
    UpdateMatchings {
        current_matchings: Vec<MatchingInfo>, // 現在のマッチング一覧
        timestamp: DateTime<Utc>,
    },
    MatchingEstablished {
        matching_id: Uuid,
        opponent_id: String,
        model_data: Option<crate::db::models::Model3D>,
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
        opponent_character: Character, // 相手のキャラクター情報のみ
        your_player_id: String,        // 自分のプレイヤーID（識別用）
        timestamp: DateTime<Utc>,
    },
    OpponentStateUpdate {
        opponent: Character,
        timestamp: DateTime<Utc>, // サーバー送信時刻（レイテンシ計測用）
    },
    GameEnd {
        result: GameResult,
        timestamp: DateTime<Utc>,
    },

    // エラー
    Error {
        message: String,
    },
}

// 3Dモデルアップロード関連
#[derive(Debug, Serialize, Deserialize)]
pub struct UploadModelResponse {
    pub model_id: String,
    pub file_name: String,
    pub file_size: i64,
}
