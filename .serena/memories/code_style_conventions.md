# コードスタイルと規約

## 基本方針

標準的なRust規約に従う

## 命名規則

- **変数・関数**: `snake_case`
  - 例: `player_id`, `matching_session`, `create_matching`
- **型・構造体・列挙型**: `PascalCase`
  - 例: `Player`, `MatchingSession`, `GameState`, `WsMessage`
- **定数**: `SCREAMING_SNAKE_CASE`
  - 例: `MAX_PLAYERS`, `UPDATE_INTERVAL_MS`
- **モジュール**: `snake_case`
  - 例: `models`, `handlers`, `game`

## ドキュメント

- パブリック関数・型には**ドキュメントコメント**（`///`）を追加
- 目的、引数、戻り値を明確に説明する

## フォーマットとリント

- **フォーマット**: `cargo fmt` で自動整形
- **リント**: `cargo clippy` で静的解析・スタイルチェック

## コード構成

- モジュールは機能ごとに分割（`handlers/`, `game/`, `db/`）
- データモデルは `models.rs` に集約
- ユーティリティ関数は `utils.rs`

## 依存関係管理

- `Cargo.toml` で明示的にバージョン指定
- features指定で必要な機能のみ有効化
- dev-dependenciesはテスト用のみ

## エラー処理

- Result型を適切に使用
- エラーは明確なメッセージとともに伝播
- パニックは避け、適切なエラーハンドリング

## 非同期処理

- tokio::spawnで非同期タスク生成
- async/awaitパターンを適切に使用
- Arc<Mutex<T>>で状態共有
