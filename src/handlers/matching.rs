use crate::models::{
    CreateMatchingRequest, CreateMatchingResponse, JoinMatchingRequest, JoinMatchingResponse,
    MatchingSession, MatchingStatus,
};
use actix_web::{HttpResponse, Responder, web};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// 共有マッチングセッション管理
pub type MatchingSessions = Arc<Mutex<HashMap<Uuid, MatchingSession>>>;

/// POST /api/matching/create - マッチングID生成
pub async fn create_matching(
    sessions: web::Data<MatchingSessions>,
    req: web::Json<CreateMatchingRequest>,
) -> impl Responder {
    println!("📥 POST /api/matching/create: player_id={}", req.player_id);
    let session = MatchingSession::new(req.player_id.clone());
    let matching_id = session.matching_id;

    // マッチングセッションを登録
    sessions.lock().unwrap().insert(matching_id, session);
    println!("✅ Matching created: matching_id={}", matching_id);

    HttpResponse::Ok().json(CreateMatchingResponse { matching_id })
}

/// POST /api/matching/join - マッチング要求
pub async fn join_matching(
    sessions: web::Data<MatchingSessions>,
    req: web::Json<JoinMatchingRequest>,
) -> impl Responder {
    println!(
        "📥 POST /api/matching/join: matching_id={}, player_id={}",
        req.matching_id, req.player_id
    );
    let mut sessions = sessions.lock().unwrap();

    // マッチングセッションを取得
    if let Some(session) = sessions.get_mut(&req.matching_id) {
        // 既にマッチング済みチェック
        if session.status != MatchingStatus::Waiting {
            // 再参加チェック (Player B)
            let is_rejoin = if let Some(ref player_b) = session.player_b {
                player_b.id == req.player_id
            } else {
                false
            };

            if is_rejoin && session.is_valid() {
                println!(
                    "✅ Rejoining matching session: matching_id={}, player_id={}",
                    req.matching_id, req.player_id
                );
                return HttpResponse::Ok().json(JoinMatchingResponse {
                    success: true,
                    message: Some("Rejoined matching session".to_string()),
                });
            }

            println!(
                "❌ Matching session is not available: status={:?}",
                session.status
            );
            return HttpResponse::BadRequest().json(JoinMatchingResponse {
                success: false,
                message: Some("This matching session is not available".to_string()),
            });
        }

        // 同じプレイヤーIDチェック
        if session.player_a.id == req.player_id {
            println!("❌ Cannot join your own matching session");
            return HttpResponse::BadRequest().json(JoinMatchingResponse {
                success: false,
                message: Some("Cannot join your own matching session".to_string()),
            });
        }

        // プレイヤーBを設定してマッチング成立
        session.player_b = Some(crate::models::Player::new(req.player_id.clone()));
        session.status = MatchingStatus::Matched;
        println!(
            "✅ Matching successful: player_a={}, player_b={}",
            session.player_a.id, req.player_id
        );

        HttpResponse::Ok().json(JoinMatchingResponse {
            success: true,
            message: Some("Matching successful".to_string()),
        })
    } else {
        println!(
            "❌ Matching session not found: matching_id={}",
            req.matching_id
        );
        HttpResponse::NotFound().json(JoinMatchingResponse {
            success: false,
            message: Some("Matching session not found".to_string()),
        })
    }
}
