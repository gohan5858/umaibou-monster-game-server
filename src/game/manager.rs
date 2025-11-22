use crate::game::state::GameStateManager;
use crate::models::{GameResult, WsMessage};
use actix::prelude::*;
use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

/// ゲーム状態更新間隔（60Hz = 16.67ms）
const TICK_INTERVAL_MS: u64 = 16;

/// ゲームマネージャーアクター
pub struct GameManager {
    games: HashMap<Uuid, GameStateManager>,
    /// WebSocket送信用チャンネル (matching_id -> (player_id -> sender))
    ws_senders: HashMap<Uuid, HashMap<String, mpsc::UnboundedSender<WsMessage>>>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            games: HashMap::new(),
            ws_senders: HashMap::new(),
        }
    }

    /// 各プレイヤーに相手の状態のみを送信
    fn send_opponent_states(&self, matching_id: &Uuid) {
        if let Some(game) = self.games.get(matching_id) {
            if let Some(senders) = self.ws_senders.get(matching_id) {
                let now = Utc::now();

                // プレイヤーAに相手（プレイヤーB）の状態を送信
                if let Some(sender_a) = senders.get(&game.player_a_id) {
                    let msg = WsMessage::OpponentStateUpdate {
                        opponent: game.player_b_character.clone(),
                        timestamp: now,
                    };
                    let _ = sender_a.send(msg);
                }

                // プレイヤーBに相手（プレイヤーA）の状態を送信
                if let Some(sender_b) = senders.get(&game.player_b_id) {
                    let msg = WsMessage::OpponentStateUpdate {
                        opponent: game.player_a_character.clone(),
                        timestamp: now,
                    };
                    let _ = sender_b.send(msg);
                }
            }
        }
    }

    /// 特定のプレイヤーが更新した時、相手にのみ状態を送信
    fn send_opponent_state_for_player(&self, matching_id: &Uuid, player_id: &str) {
        if let Some(game) = self.games.get(matching_id) {
            if let Some(senders) = self.ws_senders.get(matching_id) {
                let now = Utc::now();

                if player_id == &game.player_a_id {
                    // Aが更新 → Bに通知
                    if let Some(sender_b) = senders.get(&game.player_b_id) {
                        let msg = WsMessage::OpponentStateUpdate {
                            opponent: game.player_a_character.clone(),
                            timestamp: now,
                        };
                        let _ = sender_b.send(msg);
                    }
                } else if player_id == &game.player_b_id {
                    // Bが更新 → Aに通知
                    if let Some(sender_a) = senders.get(&game.player_a_id) {
                        let msg = WsMessage::OpponentStateUpdate {
                            opponent: game.player_b_character.clone(),
                            timestamp: now,
                        };
                        let _ = sender_a.send(msg);
                    }
                }
            }
        }
    }

    /// ゲーム終了通知を送信
    fn broadcast_game_end(&mut self, matching_id: &Uuid, result: GameResult) {
        if let Some(senders) = self.ws_senders.get(matching_id) {
            let msg = WsMessage::GameEnd {
                result,
                timestamp: Utc::now(),
            };
            for sender in senders.values() {
                let _ = sender.send(msg.clone());
            }
        }
        // 終了したゲームを削除
        self.games.remove(matching_id);
        self.ws_senders.remove(matching_id);
    }
}

impl Actor for GameManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // 60Hz更新ループを開始（勝敗判定のみ）
        ctx.run_interval(Duration::from_millis(TICK_INTERVAL_MS), |act, _ctx| {
            let game_ids: Vec<Uuid> = act.games.keys().cloned().collect();

            for matching_id in game_ids {
                if let Some(game) = act.games.get(&matching_id) {
                    // 勝者判定
                    if game.is_game_over() {
                        if let Some(winner_id) = game.check_winner() {
                            let loser_id = if winner_id == game.player_a_id {
                                game.player_b_id.clone()
                            } else {
                                game.player_a_id.clone()
                            };

                            let result = GameResult {
                                matching_id,
                                winner_id,
                                loser_id,
                                player_a_id: game.player_a_id.clone(),
                                player_b_id: game.player_b_id.clone(),
                                play_time_seconds: 0, // TODO: 実際のプレイ時間を計算
                                finished_at: Utc::now(),
                            };

                            act.broadcast_game_end(&matching_id, result);
                        }
                    }
                    // 状態送信は削除（更新時のみ送信するように変更）
                }
            }
        });
    }
}

// メッセージ: ゲーム開始
#[derive(Message)]
#[rtype(result = "()")]
pub struct StartGame {
    pub game: GameStateManager,
    pub ws_senders: HashMap<String, mpsc::UnboundedSender<WsMessage>>,
}

impl Handler<StartGame> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: StartGame, _ctx: &mut Self::Context) {
        let matching_id = msg.game.matching_id;

        // 各プレイヤーに相手のキャラクター情報のみを送信
        let now = Utc::now();

        if let Some(sender_a) = msg.ws_senders.get(&msg.game.player_a_id) {
            let start_msg = WsMessage::GameStart {
                opponent_character: msg.game.player_b_character.clone(),
                your_player_id: msg.game.player_a_id.clone(),
                timestamp: now,
            };
            let _ = sender_a.send(start_msg);
        }

        if let Some(sender_b) = msg.ws_senders.get(&msg.game.player_b_id) {
            let start_msg = WsMessage::GameStart {
                opponent_character: msg.game.player_a_character.clone(),
                your_player_id: msg.game.player_b_id.clone(),
                timestamp: now,
            };
            let _ = sender_b.send(start_msg);
        }

        // ゲームを登録
        self.games.insert(matching_id, msg.game);
        self.ws_senders.insert(matching_id, msg.ws_senders);
    }
}

// メッセージ: 入力処理
#[derive(Message)]
#[rtype(result = "()")]
pub struct ProcessInput {
    pub matching_id: Uuid,
    pub input: crate::models::PlayerInput,
}

impl Handler<ProcessInput> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: ProcessInput, _ctx: &mut Self::Context) {
        if let Some(game) = self.games.get_mut(&msg.matching_id) {
            let player_id = msg.input.player_id.clone();
            game.process_input(msg.input);

            // 入力処理後、相手に状態を通知
            self.send_opponent_state_for_player(&msg.matching_id, &player_id);
        }
    }
}

// メッセージ: 状態更新
#[derive(Message)]
#[rtype(result = "()")]
pub struct ProcessStateUpdate {
    pub matching_id: Uuid,
    pub player_id: String,
    pub position: crate::models::Vector3,
    pub rotation: crate::models::Vector3,
}

impl Handler<ProcessStateUpdate> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: ProcessStateUpdate, _ctx: &mut Self::Context) {
        if let Some(game) = self.games.get_mut(&msg.matching_id) {
            game.update_state(&msg.player_id, msg.position, msg.rotation);

            // 状態更新後、相手に通知
            self.send_opponent_state_for_player(&msg.matching_id, &msg.player_id);
        }
    }
}

// メッセージ: ダメージ適用
#[derive(Message)]
#[rtype(result = "()")]
pub struct ApplyDamage {
    pub matching_id: Uuid,
    pub player_id: String,
    pub damage: i32,
}

impl Handler<ApplyDamage> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: ApplyDamage, _ctx: &mut Self::Context) {
        if let Some(game) = self.games.get_mut(&msg.matching_id) {
            game.apply_damage(&msg.player_id, msg.damage);
        }
    }
}
