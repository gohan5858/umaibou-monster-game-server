use actix::Actor;
use actix_test;
use actix_web::{App, test, web};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use uuid::Uuid;
use webscoket_realtime_prac::db::models::Model3D;
use webscoket_realtime_prac::game::manager::GameManager;
use webscoket_realtime_prac::handlers::{
    MatchingSessions, WaitingPlayers, WsChannels, create_matching, join_matching, list_models,
    ws_handler,
};
use webscoket_realtime_prac::models::WsMessage;

#[actix_rt::test]
async fn test_model_one_time_use() {
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    let ws_channels: WsChannels = Arc::new(Mutex::new(HashMap::new()));
    let waiting_players: WaitingPlayers = Arc::new(Mutex::new(HashMap::new()));
    let game_manager = GameManager::new(matching_sessions.clone()).start();

    // 1. Manually insert a model into the DB (simulating upload)
    // We do this because multipart upload in test is verbose
    let model_id = Uuid::new_v4().to_string();

    // Use a local file for test db
    let db_path = std::env::temp_dir().join(format!("test_{}.db", Uuid::new_v4()));
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

    // Insert a model directly
    let model = Model3D::new(
        model_id.clone(),
        "test.glb".to_string(),
        "uploads/test.glb".to_string(),
        1024,
        "model/gltf-binary".to_string(),
    );
    model.insert(&pool).await.expect("Failed to insert model");

    // Start server with this pool
    let pool_clone = pool.clone();
    let srv = actix_test::start(move || {
        App::new()
            .app_data(web::Data::new(pool_clone.clone()))
            .app_data(web::Data::new(matching_sessions.clone()))
            .app_data(web::Data::new(ws_channels.clone()))
            .app_data(web::Data::new(waiting_players.clone()))
            .app_data(web::Data::new(game_manager.clone()))
            .route("/api/matching/create", web::post().to(create_matching))
            .route("/api/matching/join", web::post().to(join_matching))
            .route("/api/models", web::get().to(list_models))
            .route("/ws", web::get().to(ws_handler))
    });

    // 2. Verify it is unused via GET /api/models
    let req = srv.get("/api/models");
    let mut resp = req.send().await.unwrap();
    assert!(resp.status().is_success());
    let models: Vec<Model3D> = resp.json().await.unwrap();
    assert!(models.iter().any(|m| m.id == model_id));

    // 3. Connect via WebSocket and create matching with the model
    let ws_url = format!("ws://127.0.0.1:{}/ws?player_id=player1", srv.addr().port());
    let (ws_stream, _) = connect_async(&ws_url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Send CreateMatching with the model
    let create_msg = json!({
        "type": "CreateMatching",
        "data": {
            "selected_model_id": model_id
        }
    });
    write
        .send(Message::Text(create_msg.to_string().into()))
        .await
        .unwrap();

    // Expect MatchingCreated
    let msg = timeout(Duration::from_secs(2), read.next())
        .await
        .expect("Timeout waiting for MatchingCreated")
        .unwrap()
        .unwrap();
    if let Message::Text(text) = msg {
        let ws_msg: WsMessage = serde_json::from_str(&text).unwrap();
        match ws_msg {
            WsMessage::MatchingCreated { matching_id, .. } => {
                println!("Matching created: {}", matching_id);
            }
            _ => panic!("Expected MatchingCreated, got {:?}", ws_msg),
        }
    } else {
        panic!("Expected Text message");
    }

    // 4. Verify the model is marked as used (not in GET /api/models)
    let req = srv.get("/api/models");
    let mut resp = req.send().await.unwrap();
    assert!(resp.status().is_success());
    let models: Vec<Model3D> = resp.json().await.unwrap();
    assert!(
        !models.iter().any(|m| m.id == model_id),
        "Model should be marked as used"
    );

    // 5. Try to select the same model again (join matching) -> should fail
    // We need another player to try to join
    let ws_url2 = format!("ws://127.0.0.1:{}/ws?player_id=player2", srv.addr().port());
    let (ws_stream2, _) = connect_async(&ws_url2).await.unwrap();
    let (mut write2, mut read2) = ws_stream2.split();

    // We need a valid matching ID to join.
    // But wait, the logic checks model usage BEFORE checking matching ID in `handle_join_match`?
    // Let's check the code.
    // In `handle_join_match`:
    // 1. Check model ID (and usage).
    // 2. Check matching session.
    // So we can pass ANY matching ID, and if the model is used, it should fail at step 1.

    let join_msg = json!({
        "type": "JoinMatch",
        "data": {
            "matching_id": Uuid::new_v4(), // Random ID
            "selected_model_id": model_id
        }
    });
    write2
        .send(Message::Text(join_msg.to_string().into()))
        .await
        .unwrap();

    // Expect Error
    let msg = timeout(Duration::from_secs(2), read2.next())
        .await
        .expect("Timeout waiting for Error")
        .unwrap()
        .unwrap();
    if let Message::Text(text) = msg {
        let ws_msg: WsMessage = serde_json::from_str(&text).unwrap();
        match ws_msg {
            WsMessage::Error { message } => {
                assert!(message.contains("already been used"));
            }
            _ => panic!("Expected Error, got {:?}", ws_msg),
        }
    } else {
        panic!("Expected Text message");
    }

    // Cleanup
    let _ = std::fs::remove_file(db_path);
}
