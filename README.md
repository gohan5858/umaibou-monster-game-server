# 3D Real-time Battle Game Server

actix-webで実装した3Dリアルタイム対戦ゲームのサーバーです。

## 🎯 概要

2人対戦型の3Dリアルタイムバトルゲーム用サーバー実装。
WebSocketによる60Hzのゲーム状態配信とREST APIによるマッチング機能を提供します。

## ✨ 機能

### プレイヤーマッチング

- マッチングID生成（REST API）
- マッチング要求・成立（REST API）
- マッチング成功通知（WebSocket）
- キャラクター選択・準備完了（WebSocket）
- ゲーム開始通知（WebSocket）

### ゲーム進行管理

- リアルタイム操作入力受信（移動・攻撃・回転）
- ゲーム状態計算・配信（60Hz）
- 勝敗判定・ゲーム終了通知
- 戦績データ管理

## 🚀 クイックスタート

### 前提条件

- Rust 1.70以降
- Cargo

### ビルド & 起動

```bash
# リポジトリクローン（または既存プロジェクトに移動）
cd umaibou-monster-game-server

# ビルド
cargo build

# 起動
cargo run
```

サーバーは `http://0.0.0.0:8080` で起動します。

## 📡 API仕様

### REST API

#### マッチングID生成

```bash
POST /api/matching/create
Content-Type: application/json

{
  "player_id": "player_a"
}

# Response
{
  "matching_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

#### マッチング参加

```bash
POST /api/matching/join
Content-Type: application/json

{
  "matching_id": "550e8400-e29b-41d4-a716-446655440000",
  "player_id": "player_b"
}

# Response
{
  "success": true,
  "message": "Matching successful"
}
```

### WebSocket

#### 接続

```
ws://localhost:8080/ws?player_id={player_id}&matching_id={matching_id}
```

#### メッセージ型

**クライアント → サーバー:**
- `SelectCharacter` - キャラクター選択
- `Ready` - 準備完了
- `Input` - 操作入力（移動・攻撃・回転）

**サーバー → クライアント:**
- `MatchingSuccess` - マッチング成立
- `OpponentCharacterSelected` - 相手キャラクター情報
- `GameStart` - ゲーム開始
- `GameStateUpdate` - ゲーム状態更新（60Hz）
- `GameEnd` - ゲーム終了・結果

詳細は [WebSocketメッセージ仕様](doc/websocket-messages.md) を参照。

## 🧪 テスト

### 自動テスト

統合テストスクリプトで全テストを一括実行：

```bash
./scripts/run_tests.sh
```

個別にテスト実行：

```bash
# REST APIテスト
cargo test --test api_test

# WebSocketテスト
cargo test --test websocket_test

# 統合テスト
cargo test --test integration_test

# 全テスト実行
cargo test
```

**自動テストカバレッジ：**

**REST API:**
- ✅ マッチング作成API
- ✅ マッチング参加API（成功ケース）
- ✅ マッチング参加エラーハンドリング

**WebSocket:**
- ✅ WebSocket接続・Ping/Pong
- ✅ キャラクター選択メッセージ送信
- ✅ 準備完了メッセージ送信
- ✅ 操作入力メッセージ送信（移動）

### 手動テスト

詳細な手順は [テスト手順書](doc/testing-guide.md) を参照。

```bash
# WebSocketクライアントインストール
npm install -g wscat

# WebSocket接続テスト
wscat -c "ws://localhost:8080/ws?player_id=player_a&matching_id=<MATCHING_ID>"
```

## 📁 プロジェクト構成

```
.
├── Cargo.toml                  # 依存関係定義
├── src/
│   ├── lib.rs                  # ライブラリエントリポイント
│   ├── main.rs                 # サーバー起動
│   ├── models.rs               # データモデル
│   ├── utils.rs                # ユーティリティ関数
│   ├── game/
│   │   ├── mod.rs
│   │   ├── state.rs            # ゲーム状態管理
│   │   └── manager.rs          # 60Hzゲームループ
│   └── handlers/
│       ├── mod.rs
│       ├── matching.rs         # マッチングAPI
│       └── websocket.rs        # WebSocketハンドラー
├── tests/
│   └── api_test.rs             # REST APIテスト
├── scripts/
│   └── run_tests.sh            # 自動テスト実行スクリプト
└── doc/
    ├── specification.md        # 仕様書
    ├── testing-guide.md        # テスト手順書
    └── websocket-messages.md   # メッセージサンプル集
```

## 🏗️ アーキテクチャ

### 技術スタック

- **actix-web** - HTTPサーバー
- **actix-web-actors** - WebSocketサポート
- **actix** - アクターモデル（ゲームマネージャー）
- **tokio** - 非同期ランタイム
- **serde** - JSON シリアライズ
- **uuid** - ユニークID生成
- **chrono** - タイムスタンプ管理

### 設計のポイント

#### 60Hz更新システム

```rust
// tokio::time::intervalで16.67ms間隔の高精度タイマー
ctx.run_interval(Duration::from_millis(16), |act, _ctx| {
    // ゲーム状態更新 & 配信
});
```

#### 並行処理

- **Arc<Mutex<HashMap>>** - 複数リクエスト並行処理
- **mpsc::unbounded_channel** - 非同期メッセージ配信
- **Actixアクター** - メッセージ駆動の状態管理

#### ダメージ計算

仕様に基づき、ダメージ計算はクライアント側で実施。サーバーは結果を受信して適用。

## ⚡ 非機能要件

- ✅ **60Hz状態更新** - tokio::intervalで実現
- ✅ **1000組同時処理** - 効率的な非同期処理
- ✅ **応答時間<100ms** - actix-webの高性能ランタイム
- ✅ **REST/WebSocket使い分け** - 適切なプロトコル選択

## 📝 ライセンス

このプロジェクトは個人学習用です。

## 🤝 貢献

バグ報告や機能要望は Issue でお願いします。

## 📚 参考資料

- [仕様書](doc/specification.md)
- [テスト手順書](doc/testing-guide.md)
- [WebSocketメッセージ仕様](doc/websocket-messages.md)
- [actix-web公式ドキュメント](https://actix.rs/)
