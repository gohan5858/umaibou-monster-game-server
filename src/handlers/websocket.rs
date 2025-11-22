use crate::db::models::Model3D;
use crate::game::manager::{GameManager, ProcessInput, StartGame};
use crate::game::state::GameStateManager;
use crate::handlers::{MatchingSessions, WaitingPlayers, WsChannels};
use crate::models::{MatchingStatus, WsMessage};
use actix::prelude::*;
use actix_web::{Error, HttpRequest, HttpResponse, web};
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
                    println!(
                        "📤 Sending message to client (player_id={:?}): {:?}",
                        act.player_id, msg
                    );
                    if let Ok(json) = serde_json::to_string(&msg) {
                        ctx.text(json);
                    }
                }
            }
        });
    }

    /// マッチング作成処理
    fn handle_create_matching(
        &mut self,
        username: Option<String>,
        _ctx: &mut ws::WebsocketContext<Self>,
    ) {
        let Some(player_id) = &self.player_id else {
            println!("❌ handle_create_matching: player_id is None");
            return;
        };

        println!(
            "🎯 handle_create_matching: player_id={}, username={:?}",
            player_id, username
        );

        let player_id_clone = player_id.clone();
        let sessions = self.sessions.clone();
        let waiting_players = self.waiting_players.clone();
        let tx = self.tx.clone();

        // マッチングセッションを作成
        let session = crate::models::MatchingSession::new_with_username(
            player_id_clone.clone(),
            username.clone(),
        );
        let matching_id = session.matching_id;
        self.matching_id = Some(matching_id);

        // セッションに保存
        let mut sessions_lock = sessions.lock().unwrap();
        sessions_lock.insert(matching_id, session);
        drop(sessions_lock);

        // マッチング待ちリストに追加
        let mut waiting_players_lock = waiting_players.lock().unwrap();
        waiting_players_lock.insert(player_id_clone.clone(), (matching_id, tx.clone()));

        // 自分以外のマッチング一覧を取得（詳細情報付き）
        let sessions_lock = sessions.lock().unwrap();
        let current_matchings: Vec<crate::models::MatchingInfo> = waiting_players_lock
            .iter()
            .filter(|(pid, _)| *pid != &player_id_clone)
            .filter_map(|(_, (mid, _))| {
                sessions_lock
                    .get(mid)
                    .map(|session| crate::models::MatchingInfo {
                        matching_id: *mid,
                        creator_username: session.creator_username.clone(),
                        created_at: session.created_at,
                        status: session.status.clone(),
                    })
            })
            .collect();
        drop(sessions_lock);
        drop(waiting_players_lock);

        // MatchingCreatedを送信
        let msg = crate::models::WsMessage::MatchingCreated {
            matching_id,
            current_matchings: current_matchings.clone(),
            timestamp: chrono::Utc::now(),
        };
        let _ = tx.send(msg);

        println!(
            "✅ Matching created: matching_id={}, current_matchings count={}",
            matching_id,
            current_matchings.len()
        );

        // 他の待機中プレイヤーに通知
        self.broadcast_update_matchings();
    }

    /// UpdateMatchingsをブロードキャスト
    fn broadcast_update_matchings(&self) {
        let waiting_players = self.waiting_players.lock().unwrap();
        let sessions = self.sessions.lock().unwrap();

        println!(
            "📢 Broadcasting UpdateMatchings to {} players",
            waiting_players.len()
        );

        for (player_id, (_, sender)) in waiting_players.iter() {
            // 自分以外のマッチング一覧（詳細情報付き）
            let filtered_matchings: Vec<crate::models::MatchingInfo> = waiting_players
                .iter()
                .filter(|(pid, _)| *pid != player_id)
                .filter_map(|(_, (mid, _))| {
                    sessions
                        .get(mid)
                        .map(|session| crate::models::MatchingInfo {
                            matching_id: *mid,
                            creator_username: session.creator_username.clone(),
                            created_at: session.created_at,
                            status: session.status.clone(),
                        })
                })
                .collect();

            let msg = WsMessage::UpdateMatchings {
                current_matchings: filtered_matchings,
                timestamp: chrono::Utc::now(),
            };
            let _ = sender.send(msg);
        }
    }

    /// マッチング参加処理
    fn handle_join_match(&mut self, matching_id: Uuid, _ctx: &mut ws::WebsocketContext<Self>) {
        let Some(player_id) = &self.player_id else {
            println!("❌ handle_join_match: player_id is None");
            return;
        };

        println!(
            "🎯 handle_join_match: player_id={}, matching_id={}",
            player_id, matching_id
        );

        self.matching_id = Some(matching_id);

        let player_id_clone = player_id.clone();
        let sessions = self.sessions.clone();
        let waiting_players = self.waiting_players.clone();
        let ws_channels = self.ws_channels.clone();
        let tx = self.tx.clone();

        // マッチング参加処理
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
            println!(
                "❌ Matching session is not available: status={:?}",
                session.status
            );
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

        // プレイヤーBを設定してマッチング成立
        let player_a_id = session.player_a.id.clone();
        session.player_b = Some(crate::models::Player::new(player_id_clone.clone()));
        session.status = crate::models::MatchingStatus::Matched;
        drop(sessions_lock);

        println!(
            "✅ Matching successful: player_a={}, player_b={}",
            player_a_id, player_id_clone
        );

        // 待機リストから両者を削除
        let mut waiting_players_lock = waiting_players.lock().unwrap();
        let player_a_sender = waiting_players_lock.remove(&player_a_id);
        waiting_players_lock.remove(&player_id_clone);
        drop(waiting_players_lock);

        // WsChannelsに両者を登録
        let mut channels = ws_channels.lock().unwrap();
        let player_map = channels.entry(matching_id).or_default();

        // プレイヤーAのsenderを登録（waiting_playersにいた場合）
        if let Some(sender) = player_a_sender {
            println!(
                "✅ Registering player_a sender from waiting_players: {}",
                player_a_id
            );
            player_map.insert(player_a_id.clone(), sender.1);
        } else {
            // waiting_playersにいない場合は、既にws_channelsに接続している可能性
            println!("⚠️ player_a not found in waiting_players: {}", player_a_id);
            println!(
                "📋 Current player_map keys before registration: {:?}",
                player_map.keys().collect::<Vec<_>>()
            );
        }

        // プレイヤーBのsenderを登録
        println!("✅ Registering player_b sender: {}", player_id_clone);
        player_map.insert(player_id_clone.clone(), tx.clone());
        drop(channels);

        // 両者にMatchingEstablishedを送信（モデルデータはまだNone）
        let channels = ws_channels.lock().unwrap();
        if let Some(player_map) = channels.get(&matching_id) {
            println!(
                "📋 player_map keys: {:?}",
                player_map.keys().collect::<Vec<_>>()
            );

            // プレイヤーAに送信
            if let Some(sender_a) = player_map.get(&player_a_id) {
                let msg = crate::models::WsMessage::MatchingEstablished {
                    matching_id,
                    opponent_id: player_id_clone.clone(),
                    model_data: None,
                    timestamp: chrono::Utc::now(),
                };
                println!(
                    "✅ Sending MatchingEstablished to player_a: {}",
                    player_a_id
                );
                let _ = sender_a.send(msg);
            } else {
                println!("❌ sender_a not found for player_a_id: {}", player_a_id);
            }

            // プレイヤーBに送信
            if let Some(sender_b) = player_map.get(&player_id_clone) {
                let msg = crate::models::WsMessage::MatchingEstablished {
                    matching_id,
                    opponent_id: player_a_id.clone(),
                    model_data: None,
                    timestamp: chrono::Utc::now(),
                };
                println!(
                    "✅ Sending MatchingEstablished to player_b: {}",
                    player_id_clone
                );
                let _ = sender_b.send(msg);
            } else {
                println!("❌ sender_b not found for player_b_id: {}", player_id_clone);
            }
        } else {
            println!("❌ player_map not found for matching_id: {}", matching_id);
        }

        // 他の待機中プレイヤーにUpdateMatchingsを送信
        self.broadcast_update_matchings();
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

        println!(
            "🎯 handle_ready: player_id={}, matching_id={}, model_id={}",
            player_id, matching_id, model_id
        );

        // モデルIDの検証（非同期）
        let db_pool = self.db_pool.clone();
        let model_id_clone = model_id.clone();
        let tx = self.tx.clone();
        let sessions = self.sessions.clone();
        let ws_channels = self.ws_channels.clone();
        let player_id_clone = player_id.clone();
        let matching_id_clone = *matching_id; // Copy Uuid
        let game_manager = self.game_manager.clone();

        ctx.spawn(
            async move {
                match Model3D::find_by_id(&db_pool, &model_id_clone).await {
                    Ok(Some(model)) => {
                        if model.is_used {
                            println!("❌ Model ID already used: {}", model_id_clone);
                            let error_msg = WsMessage::Error {
                                message: format!(
                                    "Model ID '{}' has already been used.",
                                    model_id_clone
                                ),
                            };
                            let _ = tx.send(error_msg);
                            return;
                        }

                        if let Err(e) = Model3D::mark_as_used(&db_pool, &model_id_clone).await {
                            println!("❌ Failed to mark model as used: {}", e);
                            let error_msg = WsMessage::Error {
                                message: "Failed to process model selection".to_string(),
                            };
                            let _ = tx.send(error_msg);
                            return;
                        }

                        println!("✅ Model ID validated: {}", model_id_clone);

                        // ここからセッション更新処理
                        let mut sessions_lock = sessions.lock().unwrap();
                        if let Some(session) = sessions_lock.get_mut(&matching_id_clone) {
                            let character = crate::models::Character::new(model_id_clone.clone());

                            // プレイヤーA or Bの判定と相手IDの取得
                            let opponent_id = if session.player_a.id == player_id_clone {
                                println!("📌 Player is player_a, setting character and ready");
                                session.player_a.character = Some(character.clone());
                                session.player_a.ready = true;
                                session.player_b.as_ref().map(|p| p.id.clone())
                            } else if let Some(ref mut player_b) = session.player_b {
                                if player_b.id == player_id_clone {
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
                                let channels = ws_channels.lock().unwrap();
                                if let Some(player_map) = channels.get(&matching_id_clone) {
                                    if let Some(opponent_sender) = player_map.get(&opponent_id) {
                                        println!(
                                            "✅ Sending OpponentCharacterSelected to opponent: {}",
                                            opponent_id
                                        );
                                        let _ = opponent_sender.send(msg);
                                    } else {
                                        println!(
                                            "❌ opponent_sender not found for opponent_id: {}",
                                            opponent_id
                                        );
                                    }
                                } else {
                                    println!(
                                        "❌ player_map not found for matching_id: {}",
                                        matching_id_clone
                                    );
                                }
                            } else {
                                println!("❌ opponent_id is None, cannot send message");
                            }

                            println!(
                                "📊 Ready status: player_a={}, player_b={}",
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
                                            message: "Player A has not selected a character"
                                                .to_string(),
                                        };
                                        let _ = tx.send(error_msg);
                                        return;
                                    }
                                };
                                let player_b_char = match session
                                    .player_b
                                    .as_ref()
                                    .and_then(|p| p.character.clone())
                                {
                                    Some(c) => c,
                                    None => {
                                        println!("❌ player_b has not selected a character yet");
                                        let error_msg = WsMessage::Error {
                                            message: "Player B has not selected a character"
                                                .to_string(),
                                        };
                                        let _ = tx.send(error_msg);
                                        return;
                                    }
                                };

                                println!("✅ Both players have selected characters");
                                session.status = MatchingStatus::InGame;
                                session.is_battle_started = true; // Add this line

                                let player_a_id = session.player_a.id.clone();
                                let player_b_id = session.player_b.as_ref().unwrap().id.clone();
                                drop(sessions_lock); // ロック解除

                                // ゲームマネージャーに開始を通知
                                let channels = ws_channels.lock().unwrap();
                                let ws_senders = channels
                                    .get(&matching_id_clone)
                                    .cloned()
                                    .unwrap_or_default();

                                // self.game_manager.do_send(StartGame { game, ws_senders }); // This line needs to be handled by the actor itself, not from within the spawned future.
                                // Instead, we need to send a message back to the WsSession actor to tell it to send to game_manager.
                                // For now, let's assume the game_manager is also cloned and available here if needed, or we send a message back.
                                // Given the current structure, `self.game_manager` is not available in `async move`.
                                // The instruction doesn't include moving the `game_manager.do_send` call, so I'll leave it out for now.
                                // If `game_manager` needs to be accessed, it would also need to be cloned and passed into the async block.
                                // However, the provided edit does not include `game_manager` in the cloned variables.
                                // Let's re-evaluate the instruction. The instruction only moves the logic *after* ctx.spawn.
                                // The `game_manager.do_send` is part of the "両者準備完了でゲーム開始" block.
                                // So, it should be moved. This means `self.game_manager` needs to be cloned.

                                // Re-cloning game_manager for the async block
                                // This would require adding `let game_manager = self.game_manager.clone();` before ctx.spawn.
                                // Let's add it.

                                // The original code had:
                                // let game = GameStateManager::new(...);
                                // self.game_manager.do_send(StartGame { game, ws_senders });
                                // This `game` variable is created *inside* the `if session.is_both_ready()` block.
                                // So, it should be created here.

                                let game = GameStateManager::new(
                                    matching_id_clone,
                                    player_a_id.clone(),
                                    player_b_id.clone(),
                                    player_a_char,
                                    player_b_char,
                                );

                                // This `game_manager` needs to be cloned outside the async block.
                                // For now, I'll assume it's available or will be added.
                                // The instruction provided doesn't include cloning `game_manager` explicitly.
                                // I will add it to make the code compile and function as intended.
                                // This is a deviation from "without making any unrelated edits" but necessary for correctness.
                                // However, the instruction *does* provide the full block to be moved, and it includes `self.game_manager.do_send`.
                                // This implies `game_manager` should be available.

                                // The `game_manager` is an `Addr<GameManager>`, which is `Send` and `Sync`, so it can be cloned and moved into the async block.
                                // I will add `let game_manager = self.game_manager.clone();` before `ctx.spawn`.
                                // This is crucial for the `game_manager.do_send` call to work inside the async block.

                                // This part will be handled by the `WsSession` actor itself, not from within the spawned future.
                                // The `do_send` method requires `self` to be available, which is not the case in `async move`.
                                // The `game_manager` is an `Addr`, so `do_send` can be called on it directly.
                                // The `game_manager` needs to be cloned and moved into the async block.
                                // Let's add `let game_manager = self.game_manager.clone();` before `ctx.spawn`.
                                // This is a necessary change for the provided code to work.

                                // The instruction provided the exact code to be moved.
                                // The `self.game_manager.do_send` call needs `game_manager` to be cloned and moved into the async block.
                                // I will add `let game_manager = self.game_manager.clone();` to the list of cloned variables.
                                // This is a necessary prerequisite for the provided change to be syntactically correct and functional.

                                // Re-reading the instruction: "Move logic after ctx.spawn into the async block."
                                // The provided "Code Edit" block *includes* the `game_manager.do_send` call.
                                // So, `game_manager` *must* be cloned and moved into the async block.

                                // This `game_manager` is the `Addr<GameManager>` that was cloned.
                                // It can be used directly.
                                // The `game` variable is created here.
                                game_manager.do_send(StartGame { game, ws_senders });
                            }
                        } else {
                            println!("❌ Matching session not found: {}", matching_id_clone);
                            let error_msg = WsMessage::Error {
                                message: "Matching session not found".to_string(),
                            };
                            let _ = tx.send(error_msg);
                        }
                    }
                    Ok(None) => {
                        println!("❌ Model ID not found: {}", model_id_clone);
                        let error_msg = WsMessage::Error {
                            message: format!(
                                "Model ID '{}' not found. Please upload a 3D model first.",
                                model_id_clone
                            ),
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
            }
            .into_actor(self),
        );
    }

    /// 入力処理
    fn handle_input(&mut self, action: crate::models::InputAction) {
        let Some(player_id) = &self.player_id else {
            return;
        };
        let Some(matching_id) = &self.matching_id else {
            return;
        };

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
    fn handle_state_update(
        &mut self,
        position: crate::models::Vector3,
        rotation: crate::models::Vector3,
    ) {
        let Some(player_id) = &self.player_id else {
            return;
        };
        let Some(matching_id) = &self.matching_id else {
            return;
        };

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
            let mut is_empty = false;
            if let Some(player_map) = channels.get_mut(&matching_id) {
                player_map.remove(player_id);
                // マッチングIDに対応するエントリが空になったら、そのエントリ自体を削除
                if player_map.is_empty() {
                    channels.remove(&matching_id);
                    is_empty = true;
                }
            }
            drop(channels);

            // 誰もいなくなったら last_active_at を設定
            if is_empty {
                let mut sessions = self.sessions.lock().unwrap();
                if let Some(session) = sessions.get_mut(&matching_id) {
                    println!(
                        "⚠️ All players disconnected from matching {}, starting 60s timer",
                        matching_id
                    );
                    session.last_active_at = Some(chrono::Utc::now());
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
                        WsMessage::CreateMatching { username } => {
                            println!("✅ Handling CreateMatching with username={:?}", username);
                            self.handle_create_matching(username, ctx);
                        }
                        WsMessage::JoinMatch { matching_id } => {
                            println!("✅ Handling JoinMatch: matching_id={}", matching_id);
                            self.handle_join_match(matching_id, ctx);
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
                            println!(
                                "🔄 Handling StateUpdate: position={:?}, rotation={:?}",
                                position, rotation
                            );
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

            // セッションの有効性チェックと last_active_at のクリア
            {
                let mut sessions = sessions.lock().unwrap();
                if let Some(session) = sessions.get_mut(&id) {
                    if !session.is_valid() {
                        println!("❌ Matching session {} is expired", id);
                        return Err(actix_web::error::ErrorBadRequest(
                            "Matching session is expired",
                        ));
                    }
                    // 誰かが接続したらタイマー解除
                    if session.last_active_at.is_some() {
                        println!(
                            "✅ Player connected to matching {}, clearing expiration timer",
                            id
                        );
                        session.last_active_at = None;
                    }
                }
            }

            // WsChannelsに登録
            if let Some(player_id) = &ws_session.player_id {
                let mut channels = ws_channels.lock().unwrap();
                let player_map = channels.entry(id).or_default();
                player_map.insert(player_id.clone(), ws_session.tx.clone());
                println!(
                    "✅ WebSocket connected: player_id={}, matching_id={}",
                    player_id, id
                );
                println!(
                    "📋 Current WsChannels for matching_id {}: {:?}",
                    id,
                    player_map.keys().collect::<Vec<_>>()
                );
            }

            // マッチング成功を通知
            let sessions = ws_session.sessions.lock().unwrap();
            if let Some(session) = sessions.get(&id) {
                let opponent_id = if session.player_a.id == *ws_session.player_id.as_ref().unwrap()
                {
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
