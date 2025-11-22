use crate::db::models::Model3D;
use crate::models::UploadModelResponse;
use actix_multipart::Multipart;
use actix_web::{web, HttpResponse, Responder};
use futures_util::TryStreamExt;
use sqlx::SqlitePool;
use uuid::Uuid;

const MAX_FILE_SIZE: usize = 50 * 1024 * 1024; // 50MB (GLBは大きくなる可能性)
const ALLOWED_MIME_TYPES: &[&str] = &[
    "model/gltf-binary",            // GLB (glTF Binary) - 標準MIMEタイプ
    "application/octet-stream",     // 汎用バイナリ（.glbなど）
    "model/gltf+json",              // glTF JSON形式
];

/// POST /api/models/upload - 3Dモデルアップロード
pub async fn upload_model(
    mut payload: Multipart,
    pool: web::Data<SqlitePool>,
) -> impl Responder {
    println!("📥 POST /api/models/upload");

    let mut file_data = Vec::new();
    let mut file_name = String::new();
    let mut content_type = String::new();

    // Multipartフィールドを処理
    while let Ok(Some(mut field)) = payload.try_next().await {
        // ファイル名とContent-Typeを取得
        let content_disposition = field.content_disposition();
        if let Some(filename) = content_disposition.get_filename() {
            file_name = sanitize_filename(filename);
            println!("📄 file_name: {}", file_name);
        }

        content_type = field.content_type().map(|ct| ct.to_string()).unwrap_or_default();
        println!("📋 content_type: {}", content_type);

        // ファイルデータを読み込み
        while let Ok(Some(chunk)) = field.try_next().await {
            file_data.extend_from_slice(&chunk);

            // ファイルサイズチェック
            if file_data.len() > MAX_FILE_SIZE {
                println!("❌ File size exceeds limit: {} bytes", file_data.len());
                return HttpResponse::PayloadTooLarge().json(serde_json::json!({
                    "error": format!("File size exceeds {} MB limit", MAX_FILE_SIZE / 1024 / 1024)
                }));
            }
        }
    }

    // ファイル名チェック
    if file_name.is_empty() {
        println!("❌ No file name provided");
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "No file provided"
        }));
    }

    // MIMEタイプチェック（ファイル拡張子でも判定）
    let is_valid_mime = ALLOWED_MIME_TYPES.contains(&content_type.as_str());
    let is_glb_file = file_name.to_lowercase().ends_with(".glb");
    let is_gltf_file = file_name.to_lowercase().ends_with(".gltf");

    // デバッグログ強化
    println!("🔍 Validation check:");
    println!("   - file_name: {}", file_name);
    println!("   - content_type: {}", content_type);
    println!("   - is_glb_file: {}", is_glb_file);
    println!("   - is_gltf_file: {}", is_gltf_file);
    println!("   - is_valid_mime: {}", is_valid_mime);

    // 拡張子が.glbまたは.gltfの場合は、MIMEタイプに関わらず受け入れる
    let is_valid_file = is_glb_file || is_gltf_file || is_valid_mime;

    if !is_valid_file {
        println!("❌ Invalid file type: {} for file: {}", content_type, file_name);
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Invalid file type. Allowed: .glb/.gltf files or MIME types: {:?}", ALLOWED_MIME_TYPES)
        }));
    }

    if (is_glb_file || is_gltf_file) && !is_valid_mime {
        println!("⚠️  MIME type '{}' not in allowed list, but file extension is valid", content_type);
    }

    // ファイル拡張子を取得
    let extension = std::path::Path::new(&file_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("bin");

    // UUIDを生成
    let model_id = Uuid::new_v4().to_string();
    let storage_filename = format!("{}.{}", model_id, extension);
    let file_path = format!("uploads/models/{}", storage_filename);

    // ファイルを保存
    match save_file(&file_path, &file_data).await {
        Ok(_) => {
            println!("✅ File saved: {}", file_path);
        }
        Err(e) => {
            println!("❌ Failed to save file: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to save file"
            }));
        }
    }

    // データベースに記録
    let model = Model3D::new(
        model_id.clone(),
        file_name.clone(),
        file_path.clone(),
        file_data.len() as i64,
        content_type,
    );

    match model.insert(&pool).await {
        Ok(_) => {
            println!("✅ Model saved to database: {}", model_id);

            HttpResponse::Ok().json(UploadModelResponse {
                model_id,
                file_name,
                file_size: file_data.len() as i64,
            })
        }
        Err(e) => {
            println!("❌ Database error: {}", e);
            // ファイルを削除
            let _ = tokio::fs::remove_file(&file_path).await;

            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to save model metadata"
            }))
        }
    }
}

/// ファイルを保存
async fn save_file(path: &str, data: &[u8]) -> std::io::Result<()> {
    // ディレクトリを作成
    if let Some(parent) = std::path::Path::new(path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // ファイルを書き込み
    tokio::fs::write(path, data).await?;

    Ok(())
}

/// ファイル名をサニタイズ（パストラバーサル防止）
fn sanitize_filename(filename: &str) -> String {
    filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
        .collect()
}
