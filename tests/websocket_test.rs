use actix::Actor;
use actix_test;
use actix_web::{App, web};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;
use webscoket_realtime_prac::game::manager::GameManager;
use webscoket_realtime_prac::handlers::{
    MatchingSessions, WaitingPlayers, WsChannels, create_matching, join_matching, ws_handler,
};
use webscoket_realtime_prac::models::{MatchingSession, MatchingStatus, Player, WsMessage};

async fn create_test_db_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database pool");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

#[actix_rt::test]
async fn test_websocket_connection() {
    let db_pool = create_test_db_pool().await;
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    let ws_channels: WsChannels = Arc::new(Mutex::new(HashMap::new()));
    let waiting_players: WaitingPlayers = Arc::new(Mutex::new(HashMap::new()));
    let game_manager = GameManager::new(matching_sessions.clone()).start();

    let srv = actix_test::start(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(matching_sessions.clone()))
            .app_data(web::Data::new(ws_channels.clone()))
            .app_data(web::Data::new(waiting_players.clone()))
            .app_data(web::Data::new(game_manager.clone()))
            .route("/api/matching/create", web::post().to(create_matching))
            .route("/api/matching/join", web::post().to(join_matching))
            .route("/ws", web::get().to(ws_handler))
    });

    let matching_id = Uuid::new_v4();
    let ws_url = format!(
        "ws://127.0.0.1:{}/ws?player_id=test_player&matching_id={}",
        srv.addr().port(),
        matching_id
    );

    // WebSocket接続テスト
    let connect_result = timeout(Duration::from_secs(3), connect_async(&ws_url)).await;
    assert!(connect_result.is_ok(), "WebSocket connection timeout");

    let ws_result = connect_result.unwrap();
    assert!(
        ws_result.is_ok(),
        "WebSocket connection failed: {:?}",
        ws_result
    );

    let (ws_stream, _) = ws_result.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Ping送信
    write.send(Message::Ping(vec![].into())).await.unwrap();

    // Pong受信確認
    let msg_result = timeout(Duration::from_secs(2), read.next()).await;
    assert!(msg_result.is_ok(), "Pong response timeout");

    if let Some(Ok(msg)) = msg_result.unwrap() {
        assert!(
            matches!(msg, Message::Pong(_)),
            "Expected Pong message, got: {:?}",
            msg
        );
    }
}

#[actix_rt::test]
async fn test_ready_with_model_message() {
    let db_pool = create_test_db_pool().await;
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    let ws_channels: WsChannels = Arc::new(Mutex::new(HashMap::new()));
    let waiting_players: WaitingPlayers = Arc::new(Mutex::new(HashMap::new()));
    let game_manager = GameManager::new(matching_sessions.clone()).start();

    let srv = actix_test::start(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(matching_sessions.clone()))
            .app_data(web::Data::new(ws_channels.clone()))
            .app_data(web::Data::new(waiting_players.clone()))
            .app_data(web::Data::new(game_manager.clone()))
            .route("/ws", web::get().to(ws_handler))
    });

    let matching_id = Uuid::new_v4();
    let ws_url = format!(
        "ws://127.0.0.1:{}/ws?player_id=player_a&matching_id={}",
        srv.addr().port(),
        matching_id
    );

    let (ws_stream, _) = connect_async(&ws_url).await.unwrap();
    let (mut write, _read) = ws_stream.split();

    // 準備完了メッセージ（model_id含む）送信
    let ready_msg = json!({
        "type": "Ready",
        "data": {
            "selected_model_id": "warrior"
        }
    });

    let send_result = write
        .send(Message::Text(ready_msg.to_string().into()))
        .await;
    assert!(
        send_result.is_ok(),
        "Failed to send Ready message: {:?}",
        send_result
    );
}

#[actix_rt::test]
async fn test_input_move_message() {
    let db_pool = create_test_db_pool().await;
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    let ws_channels: WsChannels = Arc::new(Mutex::new(HashMap::new()));
    let waiting_players: WaitingPlayers = Arc::new(Mutex::new(HashMap::new()));
    let game_manager = GameManager::new(matching_sessions.clone()).start();

    let srv = actix_test::start(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(matching_sessions.clone()))
            .app_data(web::Data::new(ws_channels.clone()))
            .app_data(web::Data::new(waiting_players.clone()))
            .app_data(web::Data::new(game_manager.clone()))
            .route("/ws", web::get().to(ws_handler))
    });

    let matching_id = Uuid::new_v4();
    let ws_url = format!(
        "ws://127.0.0.1:{}/ws?player_id=player_a&matching_id={}",
        srv.addr().port(),
        matching_id
    );

    let (ws_stream, _) = connect_async(&ws_url).await.unwrap();
    let (mut write, _read) = ws_stream.split();

    // 移動入力メッセージ送信
    let input_msg = json!({
        "type": "Input",
        "data": {
            "action": {
                "Move": {
                    "direction": {"x": 1.0, "y": 0.0, "z": 0.0},
                    "speed": 5.0
                }
            }
        }
    });

    let send_result = write
        .send(Message::Text(input_msg.to_string().into()))
        .await;
    assert!(
        send_result.is_ok(),
        "Failed to send Move input: {:?}",
        send_result
    );
}

#[actix_rt::test]
async fn test_opponent_gets_character_selection_message() {
    // 1. サーバーセットアップ
    let db_pool = create_test_db_pool().await;
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    let ws_channels: WsChannels = Arc::new(Mutex::new(HashMap::new()));
    let waiting_players: WaitingPlayers = Arc::new(Mutex::new(HashMap::new()));
    let game_manager = GameManager::new(matching_sessions.clone()).start();

    // マッチングセッションを手動で作成
    let matching_id = Uuid::new_v4();
    let player_a_id = "player_a_test".to_string();
    let player_b_id = "player_b_test".to_string();

    let session = MatchingSession {
        matching_id,
        player_a: Player::new(player_a_id.clone()),
        player_b: Some(Player::new(player_b_id.clone())),
        status: MatchingStatus::Waiting,
        created_at: Utc::now(),
        last_active_at: None,
        is_battle_started: false,
        is_battle_finished: false,
    };
    matching_sessions
        .lock()
        .unwrap()
        .insert(matching_id, session);

    let srv = actix_test::start(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(matching_sessions.clone()))
            .app_data(web::Data::new(ws_channels.clone()))
            .app_data(web::Data::new(waiting_players.clone()))
            .app_data(web::Data::new(game_manager.clone()))
            .route("/ws", web::get().to(ws_handler))
    });

    let port = srv.addr().port();

    // 2. Player Bとして接続
    let url_b = format!(
        "ws://127.0.0.1:{}/ws?player_id={}&matching_id={}",
        port, player_b_id, matching_id
    );
    let (ws_stream_b, _) = connect_async(&url_b)
        .await
        .expect("Player B failed to connect");
    let (_, mut read_b) = ws_stream_b.split();

    // 3. Player Aとして接続
    let url_a = format!(
        "ws://127.0.0.1:{}/ws?player_id={}&matching_id={}",
        port, player_a_id, matching_id
    );
    let (ws_stream_a, _) = connect_async(&url_a)
        .await
        .expect("Player A failed to connect");
    let (mut write_a, mut read_a) = ws_stream_a.split();

    // 4. 両クライアントが `MatchingSuccess` を受信するのを待つ
    let _ = timeout(Duration::from_secs(2), read_a.next()).await;
    let _ = timeout(Duration::from_secs(2), read_b.next()).await;

    // 5. Player Aが準備完了メッセージ（キャラクター選択含む）を送信
    let ready_msg = json!({
        "type": "Ready",
        "data": {
            "selected_model_id": "knight"
        }
    });
    write_a
        .send(Message::Text(ready_msg.to_string().into()))
        .await
        .unwrap();

    // 6. Player Bが `OpponentCharacterSelected` メッセージを受信することを確認
    let msg_result = timeout(Duration::from_secs(2), read_b.next()).await;
    assert!(
        msg_result.is_ok(),
        "Player B did not receive message in time"
    );

    if let Some(Ok(received_msg)) = msg_result.unwrap() {
        match received_msg {
            Message::Text(text) => {
                let msg: WsMessage =
                    serde_json::from_str(&text).expect("Failed to parse message for B");
                match msg {
                    WsMessage::OpponentCharacterSelected { character, .. } => {
                        assert_eq!(character.model_id, "knight");
                    }
                    _ => panic!("Player B received wrong message type: {:?}", msg),
                }
            }
            other => panic!("Player B received unexpected message type: {:?}", other),
        }
    } else {
        panic!("No message received for player B");
    }
}
