mod db;
mod game;
mod handlers;
mod models;
mod utils;

use actix::Actor;
use actix_files as fs;
use actix_web::{App, HttpServer, web};
use db::init_db;
use game::manager::GameManager;
use handlers::{
    MatchingSessions, WaitingPlayers, WsChannels, create_matching, join_matching, upload_model,
    ws_handler,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🎮 Starting 3D Real-time Battle Game Server...");

    // 必要なディレクトリを作成
    tokio::fs::create_dir_all("data")
        .await
        .expect("Failed to create data directory");
    tokio::fs::create_dir_all("uploads/models")
        .await
        .expect("Failed to create uploads directory");
    println!("✅ Directories created: data/, uploads/models");

    // データベース初期化
    let database_url = "sqlite://data/models.db";
    let db_pool = init_db(database_url)
        .await
        .expect("Failed to initialize database");

    // テストモデルを自動登録
    db::load_test_models(&db_pool).await;

    // 共有状態初期化
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    let ws_channels: WsChannels = Arc::new(Mutex::new(HashMap::new()));
    let waiting_players: WaitingPlayers = Arc::new(Mutex::new(HashMap::new()));

    // ゲームマネージャーアクター起動
    let game_manager = GameManager::new(matching_sessions.clone()).start();

    println!("✅ Server initialized");
    println!("🌐 Listening on http://0.0.0.0:8080");

    // HTTPサーバー起動
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(matching_sessions.clone()))
            .app_data(web::Data::new(ws_channels.clone()))
            .app_data(web::Data::new(waiting_players.clone()))
            .app_data(web::Data::new(game_manager.clone()))
            .route("/api/matching/create", web::post().to(create_matching))
            .route("/api/matching/join", web::post().to(join_matching))
            .route("/api/models/upload", web::post().to(upload_model))
            .route("/api/models", web::get().to(handlers::list_models))
            .route("/ws", web::get().to(ws_handler))
            // 静的ファイル配信（モデルファイルのダウンロード用）
            .service(fs::Files::new("/uploads", "./uploads").show_files_listing())
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
