use actix::Actor;
use actix_rt;
use chrono::{Duration, Utc};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::sleep;
use uuid::Uuid;
use webscoket_realtime_prac::game::manager::GameManager;
use webscoket_realtime_prac::handlers::MatchingSessions;
use webscoket_realtime_prac::models::{MatchingSession, MatchingStatus, Player};

#[actix_rt::test]
async fn test_matching_validity_logic() {
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    let _game_manager = GameManager::new(matching_sessions.clone()).start();

    let matching_id = Uuid::new_v4();
    let player_a_id = "player_a".to_string();
    let player_b_id = "player_b".to_string();

    // 1. Create a session
    let mut session = MatchingSession {
        matching_id,
        creator_username: None,
        player_a: Player::new(player_a_id.clone()),
        player_b: Some(Player::new(player_b_id.clone())),
        status: MatchingStatus::Matched,
        created_at: Utc::now(),
        last_active_at: None,
        is_battle_started: false,
        is_battle_finished: false,
    };

    // 2. Verify valid initially
    assert!(session.is_valid());

    // 3. Verify valid if one player disconnects (last_active_at is None if one remains?
    // No, logic is: last_active_at is set ONLY when ALL disconnect.
    // So if one remains, last_active_at is None, so it is valid.)
    session.last_active_at = None;
    assert!(session.is_valid());

    // 4. Verify valid if both disconnect but < 60s
    session.last_active_at = Some(Utc::now() - Duration::seconds(30));
    assert!(session.is_valid());

    // 5. Verify invalid if both disconnect and > 60s
    session.last_active_at = Some(Utc::now() - Duration::seconds(61));
    assert!(!session.is_valid());

    // 6. Verify invalid if battle finished
    session.last_active_at = None;
    session.is_battle_finished = true;
    assert!(!session.is_valid());
}

#[actix_rt::test]
async fn test_cleanup_task() {
    let matching_sessions: MatchingSessions = Arc::new(Mutex::new(HashMap::new()));
    // Start GameManager which runs the cleanup task
    let _game_manager = GameManager::new(matching_sessions.clone()).start();

    let matching_id = Uuid::new_v4();

    // Insert an expired session
    let session = MatchingSession {
        matching_id,
        creator_username: None,
        player_a: Player::new("a".to_string()),
        player_b: Some(Player::new("b".to_string())),
        status: MatchingStatus::Matched,
        created_at: Utc::now(),
        last_active_at: Some(Utc::now() - Duration::seconds(65)), // Expired
        is_battle_started: false,
        is_battle_finished: false,
    };

    matching_sessions
        .lock()
        .unwrap()
        .insert(matching_id, session);

    // Wait for cleanup (runs every 1s)
    sleep(std::time::Duration::from_millis(1500)).await;

    // Verify removed
    assert!(
        matching_sessions
            .lock()
            .unwrap()
            .get(&matching_id)
            .is_none()
    );
}
