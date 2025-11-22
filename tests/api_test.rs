use actix::Actor;
use actix_web::{test, web, App};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use webscoket_realtime_prac::game::manager::GameManager;
use webscoket_realtime_prac::handlers::{create_matching, join_matching, MatchingSessions};
use webscoket_realtime_prac::models::{CreateMatchingResponse, JoinMatchingResponse};

#[actix_web::test]
async fn test_create_matching() {
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    let game_manager = GameManager::new().start();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(matching_sessions.clone()))
            .app_data(web::Data::new(game_manager.clone()))
            .route("/api/matching/create", web::post().to(create_matching))
            .route("/api/matching/join", web::post().to(join_matching)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/matching/create")
        .set_json(json!({
            "player_id": "test_player_a"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: CreateMatchingResponse = test::read_body_json(resp).await;
    assert_ne!(body.matching_id, Uuid::nil());
}

#[actix_web::test]
async fn test_join_matching_success() {
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    let game_manager = GameManager::new().start();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(matching_sessions.clone()))
            .app_data(web::Data::new(game_manager.clone()))
            .route("/api/matching/create", web::post().to(create_matching))
            .route("/api/matching/join", web::post().to(join_matching)),
    )
    .await;

    // マッチング作成
    let create_req = test::TestRequest::post()
        .uri("/api/matching/create")
        .set_json(json!({
            "player_id": "player_a"
        }))
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    let create_body: CreateMatchingResponse = test::read_body_json(create_resp).await;
    let matching_id = create_body.matching_id;

    // マッチング参加
    let join_req = test::TestRequest::post()
        .uri("/api/matching/join")
        .set_json(json!({
            "matching_id": matching_id,
            "player_id": "player_b"
        }))
        .to_request();

    let join_resp = test::call_service(&app, join_req).await;
    assert!(join_resp.status().is_success());

    let join_body: JoinMatchingResponse = test::read_body_json(join_resp).await;
    assert!(join_body.success);
}

#[actix_web::test]
async fn test_join_matching_not_found() {
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    let game_manager = GameManager::new().start();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(matching_sessions.clone()))
            .app_data(web::Data::new(game_manager.clone()))
            .route("/api/matching/create", web::post().to(create_matching))
            .route("/api/matching/join", web::post().to(join_matching)),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/api/matching/join")
        .set_json(json!({
            "matching_id": Uuid::new_v4(),
            "player_id": "player_b"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);

    let body: JoinMatchingResponse = test::read_body_json(resp).await;
    assert!(!body.success);
}
