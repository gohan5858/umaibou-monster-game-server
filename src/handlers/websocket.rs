use crate::db::models::Model3D;
use crate::game::manager::{GameManager, ProcessInput, StartGame};
use crate::game::state::GameStateManager;
use crate::handlers::{MatchingSessions, WsChannels, WaitingPlayers};
use crate::models::{Character, MatchingStatus, WsMessage};
use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use sqlx::SqlitePool;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

/// WebSocketアクター
pub struct WsSession {
    /// ハートビート最終時刻
    hb: Instant,
    /// プレイヤーID
    player_id: Option<String>,
    /// マッチングID
    matching_id: Option<Uuid>,
    /// 共有セッション管理
    sessions: MatchingSessions,
    /// WebSocketチャンネル管理
    ws_channels: WsChannels,
    /// マッチング待ちプレイヤー管理
    waiting_players: WaitingPlayers,
    /// ゲームマネージャーアドレス
    game_manager: Addr<GameManager>,
    /// データベースプール
    db_pool: SqlitePool,
    /// メッセージ受信チャンネル
    rx: Option<mpsc::UnboundedReceiver<WsMessage>>,
    /// メッセージ送信チャンネル
    tx: mpsc::UnboundedSender<WsMessage>,
}

impl WsSession {
    pub fn new(
        sessions: MatchingSessions,
        ws_channels: WsChannels,
        waiting_players: WaitingPlayers,
        game_manager: Addr<GameManager>,
        db_pool: SqlitePool,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            hb: Instant::now(),
            player_id: None,
            matching_id: None,
            sessions,
            ws_channels,
            waiting_players,
            game_manager,
            db_pool,
            rx: Some(rx),
            tx,
        }
    }

    /// ハートビート送信
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            if Instant::now().duration_since(act.hb) > Duration::from_secs(10) {
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    /// メッセージポーリング
    fn poll_messages(&mut self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_millis(10), |act, ctx| {
            if let Some(rx) = &mut act.rx {
                while let Ok(msg) = rx.try_recv() {
                    println!("📤 Sending message to client (player_id={:?}): {:?}", act.player_id, msg);
                    if let Ok(json) = serde_json::to_string(&msg) {
                        ctx.text(json);
                    }
                }
            }
        });
    }


    /// マッチング作成処理
    fn handle_create_matching(&mut self, model_id: String, ctx: &mut ws::WebsocketContext<Self>) {
        let Some(player_id) = &self.player_id else {
            println!("❌ handle_create_matching: player_id is None");
            return;
        };

        println!("🎯 handle_create_matching: player_id={}, model_id={}", player_id, model_id);

        // モデルIDの検証（非同期）
        let db_pool = self.db_pool.clone();
        let model_id_clone = model_id.clone();
        let player_id_clone = player_id.clone();
        let sessions = self.sessions.clone();
        let waiting_players = self.waiting_players.clone();
        let tx = self.tx.clone();

        ctx.spawn(async move {
            match crate::db::models::Model3D::find_by_id(&db_pool, &model_id_clone).await {
                Ok(Some(_)) => {
                    println!("✅ Model ID validated: {}", model_id_clone);

                    // マッチングセッションを作成
                    let session = crate::models::MatchingSession::new_with_model(player_id_clone.clone(), model_id_clone.clone());
                    let matching_id = session.matching_id;

                    // セッションに保存
                    let mut sessions_lock = sessions.lock().unwrap();
                    sessions_lock.insert(matching_id, session);
                    drop(sessions_lock);

                    // マッチング待ちリストに追加
                    let mut waiting_players_lock = waiting_players.lock().unwrap();
                    waiting_players_lock.insert(player_id_clone.clone(), (matching_id, tx.clone()));

                    // 自分以外のマッチング一覧を取得
                    let current_matchings: Vec<uuid::Uuid> = waiting_players_lock
                        .iter()
                        .filter(|(pid, _)| *pid != &player_id_clone)
                        .map(|(_, (mid, _))| *mid)
                        .collect();
                    drop(waiting_players_lock);

                    // MatchingCreatedを送信
                    let msg = crate::models::WsMessage::MatchingCreated {
                        matching_id,
                        current_matchings: current_matchings.clone(),
                        timestamp: chrono::Utc::now(),
                    };
                    let _ = tx.send(msg);

                    println!("✅ Matching created: matching_id={}, current_matchings={:?}", matching_id, current_matchings);
                }
                Ok(None) => {
                    println!("❌ Model ID not found: {}", model_id_clone);
                    let error_msg = crate::models::WsMessage::Error {
                        message: format!("Model ID '{}' not found. Please upload a 3D model first.", model_id_clone),
                    };
                    let _ = tx.send(error_msg);
                    return;
                }
                Err(e) => {
                    println!("❌ Database error while validating model ID: {}", e);
                    let error_msg = crate::models::WsMessage::Error {
                        message: "Failed to validate model ID".to_string(),
                    };
                    let _ = tx.send(error_msg);
                    return;
                }
            }
        }.into_actor(self));
    }


    /// UpdateMatchingsをブロードキャスト
    fn broadcast_update_matchings(&self) {
        let waiting_players = self.waiting_players.lock().unwrap();

        println!("📢 Broadcasting UpdateMatchings to {} players", waiting_players.len());

        for (player_id, (_, sender)) in waiting_players.iter() {
            // 自分以外のマッチング一覧
            let filtered_matchings: Vec<Uuid> = waiting_players
                .iter()
                .filter(|(pid, _)| *pid != player_id)
                .map(|(_, (mid, _))| *mid)
                .collect();

            let msg = WsMessage::UpdateMatchings {
                current_matchings: filtered_matchings,
                timestamp: chrono::Utc::now(),
            };
            let _ = sender.send(msg);
        }
    }

    /// マッチング参加処理
    fn handle_join_match(&mut self, matching_id: Uuid, model_id: String, ctx: &mut ws::WebsocketContext<Self>) {
        let Some(player_id) = &self.player_id else {
            println!("❌ handle_join_match: player_id is None");
            return;
        };

        println!("🎯 handle_join_match: player_id={}, matching_id={}, model_id={}", player_id, matching_id, model_id);

        // モデルIDの検証と参加処理（非同期）
        let db_pool = self.db_pool.clone();
        let model_id_clone = model_id.clone();
        let player_id_clone = player_id.clone();
        let sessions = self.sessions.clone();
        let waiting_players = self.waiting_players.clone();
        let ws_channels = self.ws_channels.clone();
        let tx = self.tx.clone();

        ctx.spawn(async move {
            // モデルIDの検証
            let player_b_model = match crate::db::models::Model3D::find_by_id(&db_pool, &model_id_clone).await {
                Ok(Some(model)) => model,
                Ok(None) => {
                    println!("❌ Model ID not found: {}", model_id_clone);
                    let error_msg = crate::models::WsMessage::Error {
                        message: format!("Model ID '{}' not found. Please upload a 3D model first.", model_id_clone),
                    };
                    let _ = tx.send(error_msg);
                    return;
                }
                Err(e) => {
                    println!("❌ Database error while validating model ID: {}", e);
                    let error_msg = crate::models::WsMessage::Error {
                        message: "Failed to validate model ID".to_string(),
                    };
                    let _ = tx.send(error_msg);
                    return;
                }
            };

            let mut sessions_lock = sessions.lock().unwrap();
            let session = match sessions_lock.get_mut(&matching_id) {
                Some(s) => s,
                None => {
                    println!("❌ Matching session not found: matching_id={}", matching_id);
                    let error_msg = crate::models::WsMessage::Error {
                        message: "Matching session not found".to_string(),
                    };
                    let _ = tx.send(error_msg);
                    return;
                }
            };

            // 既にマッチング済みチェック
            if session.status != crate::models::MatchingStatus::Waiting {
                println!("❌ Matching session is not available: status={:?}", session.status);
                let error_msg = crate::models::WsMessage::Error {
                    message: "This matching session is not available".to_string(),
                };
                let _ = tx.send(error_msg);
                return;
            }

            // 同じプレイヤーIDチェック
            if session.player_a.id == player_id_clone {
                println!("❌ Cannot join your own matching session");
                let error_msg = crate::models::WsMessage::Error {
                    message: "Cannot join your own matching session".to_string(),
                };
                let _ = tx.send(error_msg);
                return;
            }

            // プレイヤーAのモデルIDを取得
            let player_a_model_id = session.player_a.selected_model_id.clone();

            // プレイヤーAのモデルデータを取得
            let player_a_model = if let Some(model_id) = &player_a_model_id {
                match crate::db::models::Model3D::find_by_id(&db_pool, model_id).await {
                    Ok(Some(model)) => Some(model),
                    _ => None,
                }
            } else {
                None
            };

            // プレイヤーBを設定してマッチング成立
            let player_a_id = session.player_a.id.clone();
            session.player_b = Some(crate::models::Player::new_with_model(player_id_clone.clone(), model_id_clone.clone()));
            session.status = crate::models::MatchingStatus::Matched;
            drop(sessions_lock);

            println!("✅ Matching successful: player_a={}, player_b={}", player_a_id, player_id_clone);

            // 待機リストから両者を削除
            let mut waiting_players_lock = waiting_players.lock().unwrap();
            let player_a_sender = waiting_players_lock.remove(&player_a_id);
            waiting_players_lock.remove(&player_id_clone);
            drop(waiting_players_lock);

            // WsChannelsに両者を登録
            let mut channels = ws_channels.lock().unwrap();
            let player_map = channels.entry(matching_id).or_default();
            player_map.insert(player_a_id.clone(), player_a_sender.unwrap().1);
            player_map.insert(player_id_clone.clone(), tx.clone());
            drop(channels);

            // 両者にMatchingEstablishedを送信（相手のモデルデータ付き）
            let channels = ws_channels.lock().unwrap();
            if let Some(player_map) = channels.get(&matching_id) {
                // プレイヤーAに送信（プレイヤーBのモデルデータ）
                if let Some(sender_a) = player_map.get(&player_a_id) {
                    let msg = crate::models::WsMessage::MatchingEstablished {
                        matching_id,
                        opponent_id: player_id_clone.clone(),
                        opponent_model: Some(player_b_model.clone()),
                        timestamp: chrono::Utc::now(),
                    };
                    let _ = sender_a.send(msg);
                }

                // プレイヤーBに送信（プレイヤーAのモデルデータ）
                if let Some(sender_b) = player_map.get(&player_id_clone) {
                    let msg = crate::models::WsMessage::MatchingEstablished {
                        matching_id,
                        opponent_id: player_a_id.clone(),
                        opponent_model: player_a_model,
                        timestamp: chrono::Utc::now(),
                    };
                    let _ = sender_b.send(msg);
                }
            }
        }.into_actor(self));
    }

    /// 準備完了処理（キャラクター選択を含む）
    fn handle_ready(&mut self, model_id: String, ctx: &mut ws::WebsocketContext<Self>) {
        let Some(player_id) = &self.player_id else {
            println!("❌ handle_ready: player_id is None");
            return;
        };
        let Some(matching_id) = &self.matching_id else {
            println!("❌ handle_ready: matching_id is None");
            return;
        };

        println!("🎯 handle_ready: player_id={}, matching_id={}, model_id={}", player_id, matching_id, model_id);

        // モデルIDの検証（非同期）
        let db_pool = self.db_pool.clone();
        let model_id_clone = model_id.clone();
        let tx = self.tx.clone();

        ctx.spawn(async move {
            match Model3D::find_by_id(&db_pool, &model_id_clone).await {
                Ok(Some(_)) => {
                    println!("✅ Model ID validated: {}", model_id_clone);
                }
                Ok(None) => {
                    println!("❌ Model ID not found: {}", model_id_clone);
                    let error_msg = WsMessage::Error {
                        message: format!("Model ID '{}' not found. Please upload a 3D model first.", model_id_clone),
                    };
                    let _ = tx.send(error_msg);
                    return;
                }
                Err(e) => {
                    println!("❌ Database error while validating model ID: {}", e);
                    let error_msg = WsMessage::Error {
                        message: "Failed to validate model ID".to_string(),
                    };
                    let _ = tx.send(error_msg);
                    return;
                }
            }
        }.into_actor(self));

        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(matching_id) {
            let character = Character::new(model_id.clone());

            // プレイヤーA or Bの判定と相手IDの取得
            let opponent_id = if session.player_a.id == *player_id {
                println!("📌 Player is player_a, setting character and ready");
                session.player_a.character = Some(character.clone());
                session.player_a.ready = true;
                session.player_b.as_ref().map(|p| p.id.clone())
            } else if let Some(ref mut player_b) = session.player_b {
                if player_b.id == *player_id {
                    println!("📌 Player is player_b, setting character and ready");
                    player_b.character = Some(character.clone());
                    player_b.ready = true;
                    Some(session.player_a.id.clone())
                } else {
                    println!("❌ Player ID mismatch");
                    None
                }
            } else {
                println!("❌ player_b is None");
                None
            };

            println!("🎯 opponent_id: {:?}", opponent_id);

            // 相手に通知
            if let Some(opponent_id) = opponent_id {
                let msg = WsMessage::OpponentCharacterSelected {
                    character,
                    timestamp: chrono::Utc::now(),
                };
                let channels = self.ws_channels.lock().unwrap();
                println!("📋 WsChannels for matching_id {}: {:?}", matching_id, channels.get(matching_id).map(|m| m.keys().collect::<Vec<_>>()));

                if let Some(player_map) = channels.get(matching_id) {
                    if let Some(opponent_sender) = player_map.get(&opponent_id) {
                        println!("✅ Sending OpponentCharacterSelected to opponent: {}", opponent_id);
                        let _ = opponent_sender.send(msg);
                    } else {
                        println!("❌ opponent_sender not found for opponent_id: {}", opponent_id);
                    }
                } else {
                    println!("❌ player_map not found for matching_id: {}", matching_id);
                }
            } else {
                println!("❌ opponent_id is None, cannot send message");
            }

            println!("📊 Ready status: player_a={}, player_b={}",
                session.player_a.ready,
                session.player_b.as_ref().map_or(false, |p| p.ready)
            );

            // 両者準備完了でゲーム開始
            if session.is_both_ready() {
                println!("🎮 Both players ready, starting game...");

                // キャラクター選択チェック
                let player_a_char = match session.player_a.character.clone() {
                    Some(c) => c,
                    None => {
                        println!("❌ player_a has not selected a character yet");
                        let error_msg = WsMessage::Error {
                            message: "Player A has not selected a character".to_string(),
                        };
                        let _ = self.tx.send(error_msg);
                        return;
                    }
                };

                let player_b_char = match session.player_b.as_ref().and_then(|p| p.character.clone()) {
                    Some(c) => c,
                    None => {
                        println!("❌ player_b has not selected a character yet");
                        let error_msg = WsMessage::Error {
                            message: "Player B has not selected a character".to_string(),
                        };
                        let _ = self.tx.send(error_msg);
                        return;
                    }
                };

                println!("✅ Both players have selected characters");
                session.status = MatchingStatus::InGame;

                let game = GameStateManager::new(
                    *matching_id,
                    session.player_a.id.clone(),
                    session.player_b.as_ref().unwrap().id.clone(),
                    player_a_char,
                    player_b_char,
                );

                // ゲームマネージャーに開始を通知
                let channels = self.ws_channels.lock().unwrap();
                let ws_senders = channels
                    .get(matching_id)
                    .cloned()
                    .unwrap_or_default();

                self.game_manager.do_send(StartGame {
                    game,
                    ws_senders,
                });
            }
        }
    }

    /// 入力処理
    fn handle_input(&mut self, action: crate::models::InputAction) {
        let Some(player_id) = &self.player_id else { return };
        let Some(matching_id) = &self.matching_id else { return };

        let input = crate::models::PlayerInput {
            player_id: player_id.clone(),
            action,
            timestamp: chrono::Utc::now(),
        };

        self.game_manager.do_send(ProcessInput {
            matching_id: *matching_id,
            input,
        });
    }

    /// 状態更新処理
    fn handle_state_update(&mut self, position: crate::models::Vector3, rotation: crate::models::Vector3) {
        let Some(player_id) = &self.player_id else { return };
        let Some(matching_id) = &self.matching_id else { return };

        use crate::game::manager::ProcessStateUpdate;
        self.game_manager.do_send(ProcessStateUpdate {
            matching_id: *matching_id,
            player_id: player_id.clone(),
            position,
            rotation,
        });
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        self.poll_messages(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        // マッチング待ちリストから自分を削除
        if let Some(player_id) = &self.player_id {
            let mut waiting_players = self.waiting_players.lock().unwrap();
            waiting_players.remove(player_id);
            drop(waiting_players);

            // 他の待機中プレイヤーにUpdateMatchingsを送信
            self.broadcast_update_matchings();
        }

        // WsChannelsから自分を削除
        if let (Some(matching_id), Some(player_id)) = (self.matching_id, &self.player_id) {
            let mut channels = self.ws_channels.lock().unwrap();
            if let Some(player_map) = channels.get_mut(&matching_id) {
                player_map.remove(player_id);
                // マッチングIDに対応するエントリが空になったら、そのエントリ自体を削除
                if player_map.is_empty() {
                    channels.remove(&matching_id);
                }
            }
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                println!("📨 Received WebSocket message: {}", text);
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                    match ws_msg {
                        WsMessage::CreateMatching { selected_model_id } => {
                            println!("✅ Handling CreateMatching: selected_model_id={}", selected_model_id);
                            self.handle_create_matching(selected_model_id, ctx);
                        }
                        WsMessage::JoinMatch { matching_id, selected_model_id } => {
                            println!("✅ Handling JoinMatch: matching_id={}, selected_model_id={}", matching_id, selected_model_id);
                            self.handle_join_match(matching_id, selected_model_id, ctx);
                        }
                        WsMessage::Ready { selected_model_id } => {
                            println!("✅ Handling Ready: selected_model_id={}", selected_model_id);
                            self.handle_ready(selected_model_id, ctx);
                        }
                        WsMessage::Input { action } => {
                            println!("🎯 Handling Input: action={:?}", action);
                            self.handle_input(action);
                        }
                        WsMessage::StateUpdate { position, rotation } => {
                            println!("🔄 Handling StateUpdate: position={:?}, rotation={:?}", position, rotation);
                            self.handle_state_update(position, rotation);
                        }
                        _ => {
                            println!("⚠️ Unhandled message type");
                        }
                    }
                } else {
                    println!("❌ Failed to deserialize WsMessage: {}", text);

                    // 旧形式のSelectCharacterメッセージをチェック
                    if text.contains("\"type\":\"SelectCharacter\"") {
                        let error_msg = WsMessage::Error {
                            message: "SelectCharacter is deprecated. Use Ready with selected_model_id instead. Example: {\"type\":\"Ready\",\"data\":{\"selected_model_id\":\"your_model_id\"}}".to_string(),
                        };
                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            ctx.text(json);
                        }
                    } else {
                        let error_msg = WsMessage::Error {
                            message: format!("Invalid message format: {}", text),
                        };
                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            ctx.text(json);
                        }
                    }
                }
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => {}
        }
    }
}

/// WebSocketエンドポイント
pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    db_pool: web::Data<SqlitePool>,
    sessions: web::Data<MatchingSessions>,
    ws_channels: web::Data<WsChannels>,
    waiting_players: web::Data<WaitingPlayers>,
    game_manager: web::Data<Addr<GameManager>>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, Error> {
    println!("🔌 WebSocket connection attempt: query={:?}", query);

    let mut ws_session = WsSession::new(
        sessions.get_ref().clone(),
        ws_channels.get_ref().clone(),
        waiting_players.get_ref().clone(),
        game_manager.get_ref().clone(),
        db_pool.get_ref().clone(),
    );

    // クエリパラメータからplayer_idを取得（なければ生成）
    let player_id = if let Some(player_id) = query.get("player_id") {
        println!("👤 player_id={}", player_id);
        player_id.clone()
    } else {
        let generated_id = Uuid::new_v4().to_string();
        println!("🆕 Generated player_id={}", generated_id);
        generated_id
    };
    ws_session.player_id = Some(player_id.clone());
    if let Some(matching_id) = query.get("matching_id") {
        println!("🎯 matching_id={}", matching_id);
        if let Ok(id) = Uuid::parse_str(matching_id) {
            ws_session.matching_id = Some(id);

            // WsChannelsに登録
            if let Some(player_id) = &ws_session.player_id {
                let mut channels = ws_channels.lock().unwrap();
                let player_map = channels.entry(id).or_default();
                player_map.insert(player_id.clone(), ws_session.tx.clone());
                println!("✅ WebSocket connected: player_id={}, matching_id={}", player_id, id);
                println!("📋 Current WsChannels for matching_id {}: {:?}", id, player_map.keys().collect::<Vec<_>>());
            }

            // マッチング成功を通知
            let sessions = ws_session.sessions.lock().unwrap();
            if let Some(session) = sessions.get(&id) {
                let opponent_id = if session.player_a.id == *ws_session.player_id.as_ref().unwrap() {
                    session.player_b.as_ref().map(|p| p.id.clone())
                } else {
                    Some(session.player_a.id.clone())
                };

                if let Some(opponent_id) = opponent_id {
                    let msg = WsMessage::MatchingSuccess {
                        matching_id: id,
                        opponent_id,
                        timestamp: chrono::Utc::now(),
                    };
                    let _ = ws_session.tx.send(msg);
                }
            }
        }
    }

    ws::start(ws_session, &req, stream)
}
