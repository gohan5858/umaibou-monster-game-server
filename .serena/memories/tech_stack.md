# 技術スタック

## 言語

- **Rust** (edition 2024)

## メインフレームワーク

- **actix-web** 4.12.0 - HTTPサーバー
- **actix-web-actors** 4.3.0 - WebSocketサポート
- **actix** 0.13.5 - アクターモデル（ゲームマネージャー）
- **actix-rt** 2.11.0 - Actix非同期ランタイム

## 非同期処理

- **tokio** 1.41 (features: full) - 非同期ランタイム

## データ処理

- **serde** 1.0 (features: derive) - シリアライズ/デシリアライズ
- **serde_json** 1.0 - JSON処理

## データベース

- **sqlx** 0.7 (features: runtime-tokio-native-tls, sqlite) - SQLite非同期アクセス

## ユーティリティ

- **uuid** 1.11 (features: v4, serde) - ユニークID生成
- **chrono** 0.4 (features: serde) - 日時管理
- **rand** 0.9.2 - ランダム生成
- **futures-util** 0.3 - Future関連ユーティリティ
- **actix-files** 0.6 - 静的ファイル配信
- **actix-multipart** 0.6 - マルチパートリクエスト処理

## テスト用

- **actix-test** 0.1 - Actixテストユーティリティ
- **awc** 3.5 - ActixのWebクライアント
- **tokio-tungstenite** 0.26 - WebSocketクライアント（テスト用）
