use actix::Actor;
use actix_test;
use actix_web::{App, web};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;
use webscoket_realtime_prac::db::models::Model3D;
use webscoket_realtime_prac::game::manager::GameManager;
use webscoket_realtime_prac::handlers::{
    LobbyPlayers, MatchingSessions, WaitingPlayers, WsChannels, list_models, ws_handler,
};
use webscoket_realtime_prac::models::WsMessage;

#[actix_rt::test]
async fn test_game_start_notification() {
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    let ws_channels: WsChannels = Arc::new(Mutex::new(HashMap::new()));
    let waiting_players: WaitingPlayers = Arc::new(Mutex::new(HashMap::new()));
    let lobby_players: LobbyPlayers = Arc::new(Mutex::new(HashMap::new()));
    let game_manager = GameManager::new(matching_sessions.clone()).start();

    // Setup DB
    let db_path = std::env::temp_dir().join(format!("test_gamestart_{}.db", Uuid::new_v4()));
    let db_path_str = db_path.to_str().unwrap();
    let db_url = format!("sqlite:{}?mode=rwc", db_path_str);
    println!("Using database at: {}", db_path_str);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to create test database pool");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Insert 2 models
    let model_id_1 = Uuid::new_v4().to_string();
    let model_1 = Model3D::new(
        model_id_1.clone(),
        "test1.glb".to_string(),
        "uploads/test1.glb".to_string(),
        1024,
        "model/gltf-binary".to_string(),
    );
    model_1
        .insert(&pool)
        .await
        .expect("Failed to insert model 1");

    let model_id_2 = Uuid::new_v4().to_string();
    let model_2 = Model3D::new(
        model_id_2.clone(),
        "test2.glb".to_string(),
        "uploads/test2.glb".to_string(),
        1024,
        "model/gltf-binary".to_string(),
    );
    model_2
        .insert(&pool)
        .await
        .expect("Failed to insert model 2");

    // Start server
    let pool_clone = pool.clone();
    let srv = actix_test::start(move || {
        App::new()
            .app_data(web::Data::new(pool_clone.clone()))
            .app_data(web::Data::new(matching_sessions.clone()))
            .app_data(web::Data::new(ws_channels.clone()))
            .app_data(web::Data::new(waiting_players.clone()))
            .app_data(web::Data::new(lobby_players.clone()))
            .app_data(web::Data::new(game_manager.clone()))
            .route("/api/models", web::get().to(list_models))
            .route("/ws", web::get().to(ws_handler))
    });

    // Player 1 connects
    let ws_url1 = format!("ws://127.0.0.1:{}/ws?player_id=player1", srv.addr().port());
    let (ws_stream1, _) = connect_async(&ws_url1).await.unwrap();
    let (mut write1, mut read1) = ws_stream1.split();

    // Player 1 creates matching
    let create_msg = json!({
        "type": "CreateMatching",
        "data": { "username": "Player1" }
    });
    write1
        .send(Message::Text(create_msg.to_string().into()))
        .await
        .unwrap();

    let matching_id = loop {
        let msg = timeout(Duration::from_secs(2), read1.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if let Message::Text(text) = msg {
            let ws_msg: WsMessage = serde_json::from_str(&text).unwrap();
            if let WsMessage::MatchingCreated { matching_id, .. } = ws_msg {
                break matching_id;
            }
        }
    };

    // Player 2 connects
    let ws_url2 = format!("ws://127.0.0.1:{}/ws?player_id=player2", srv.addr().port());
    let (ws_stream2, _) = connect_async(&ws_url2).await.unwrap();
    let (mut write2, mut read2) = ws_stream2.split();

    // Player 2 joins
    let join_msg = json!({
        "type": "JoinMatch",
        "data": { "matching_id": matching_id }
    });
    write2
        .send(Message::Text(join_msg.to_string().into()))
        .await
        .unwrap();

    // Both should receive MatchingEstablished
    // Player 1
    loop {
        let msg = timeout(Duration::from_secs(2), read1.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if let Message::Text(text) = msg {
            let ws_msg: WsMessage = serde_json::from_str(&text).unwrap();
            if let WsMessage::MatchingEstablished { .. } = ws_msg {
                println!("Player 1 received MatchingEstablished");
                break;
            }
        }
    }
    // Player 2
    loop {
        let msg = timeout(Duration::from_secs(2), read2.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if let Message::Text(text) = msg {
            let ws_msg: WsMessage = serde_json::from_str(&text).unwrap();
            if let WsMessage::MatchingEstablished { .. } = ws_msg {
                println!("Player 2 received MatchingEstablished");
                break;
            }
        }
    }

    // Player 1 sends Ready
    let ready_msg1 = json!({
        "type": "Ready",
        "data": { "selected_model_id": model_id_1 }
    });
    write1
        .send(Message::Text(ready_msg1.to_string().into()))
        .await
        .unwrap();

    // Player 2 sends Ready
    let ready_msg2 = json!({
        "type": "Ready",
        "data": { "selected_model_id": model_id_2 }
    });
    write2
        .send(Message::Text(ready_msg2.to_string().into()))
        .await
        .unwrap();

    // Both should receive GameStart
    let mut p1_game_start = false;
    let mut p2_game_start = false;

    // Check Player 1
    let check_p1 = async {
        loop {
            match timeout(Duration::from_secs(5), read1.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    let ws_msg: WsMessage = serde_json::from_str(&text).unwrap();
                    if let WsMessage::GameStart { .. } = ws_msg {
                        println!("Player 1 received GameStart");
                        return true;
                    }
                }
                _ => return false,
            }
        }
    };

    // Check Player 2
    let check_p2 = async {
        loop {
            match timeout(Duration::from_secs(5), read2.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    let ws_msg: WsMessage = serde_json::from_str(&text).unwrap();
                    if let WsMessage::GameStart { .. } = ws_msg {
                        println!("Player 2 received GameStart");
                        return true;
                    }
                }
                _ => return false,
            }
        }
    };

    let (r1, r2) = tokio::join!(check_p1, check_p2);

    assert!(r1, "Player 1 did not receive GameStart");
    assert!(r2, "Player 2 did not receive GameStart");

    // Cleanup
    let _ = std::fs::remove_file(db_path);
}
