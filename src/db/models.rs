use chrono::Utc;
use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};

/// データベース3Dモデル
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Model3D {
    pub id: String,
    pub file_name: String,
    pub file_path: String,
    pub file_size: i64,
    pub mime_type: String,
    pub uploaded_at: String,
}

impl Model3D {
    /// 新規モデルを作成
    pub fn new(
        id: String,
        file_name: String,
        file_path: String,
        file_size: i64,
        mime_type: String,
    ) -> Self {
        Self {
            id,
            file_name,
            file_path,
            file_size,
            mime_type,
            uploaded_at: Utc::now().to_rfc3339(),
        }
    }

    /// モデルをデータベースに挿入
    pub async fn insert(&self, pool: &SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO models (id, file_name, file_path, file_size, mime_type, uploaded_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&self.id)
        .bind(&self.file_name)
        .bind(&self.file_path)
        .bind(self.file_size)
        .bind(&self.mime_type)
        .bind(&self.uploaded_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// IDでモデルを取得
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Model3D>, sqlx::Error> {
        let model = sqlx::query_as::<_, Model3D>(
            "SELECT * FROM models WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(model)
    }


    /// IDでモデルを削除
    pub async fn delete_by_id(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM models WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
