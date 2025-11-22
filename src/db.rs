pub mod models;

use crate::db::models::Model3D;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::path::Path;

/// データベース接続プールを初期化
pub async fn init_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    println!("🗄️  Initializing database: {}", database_url);

    // データベースファイルのディレクトリを作成
    if let Some(parent) = Path::new(database_url.trim_start_matches("sqlite://")).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| sqlx::Error::Io(e))?;
    }

    // 接続プール作成（create_if_missingを有効化）
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("{}?mode=rwc", database_url))
        .await?;

    // マイグレーション実行
    sqlx::migrate!("./migrations").run(&pool).await?;

    println!("✅ Database initialized successfully");

    Ok(pool)
}

/// テストモデルを自動登録
pub async fn load_test_models(pool: &SqlitePool) {
    println!("🎨 Loading test models from model/ directory...");

    // model/ ディレクトリのGLBファイルを自動検出
    let mut test_models = Vec::new();

    if let Ok(entries) = tokio::fs::read_dir("model/").await {
        let mut entries = entries;
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(file_name) = entry.file_name().into_string() {
                if file_name.ends_with(".glb") {
                    let path = entry.path();
                    let path_str = path.to_str().unwrap_or("").to_string();
                    // ファイル名から拡張子を除いた部分をmodel_idに使用
                    let model_id = format!(
                        "character_{}",
                        file_name
                            .trim_end_matches(".glb")
                            .to_lowercase()
                            .replace(" ", "_")
                    );
                    test_models.push((model_id, path_str));
                }
            }
        }
    }

    if test_models.is_empty() {
        println!("  ⚠️  No GLB files found in model/ directory");
        return;
    }

    for (model_id, file_path) in test_models {
        // 既に登録されているかチェック
        if let Ok(Some(_)) = Model3D::find_by_id(pool, &model_id).await {
            println!("  ⏭️  {} already exists, skipping", model_id);
            continue;
        }

        // ファイルサイズを取得
        let file_size = match tokio::fs::metadata(&file_path).await {
            Ok(metadata) => metadata.len() as i64,
            Err(_) => {
                println!("  ⚠️  {} not found at {}", model_id, file_path);
                continue;
            }
        };

        let model = Model3D::new(
            model_id.to_string(),
            Path::new(&file_path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            file_path.to_string(),
            file_size,
            "model/gltf-binary".to_string(),
        );

        match model.insert(pool).await {
            Ok(_) => println!("  ✅ Registered test model: {} ({})", model_id, file_path),
            Err(e) => println!("  ❌ Failed to register {}: {}", model_id, e),
        }
    }

    println!("✅ Test models loaded");
}
