# コードベース構造

## ディレクトリ構成

```
.
├── Cargo.toml              # 依存関係定義
├── Cargo.lock              # ロックファイル
├── README.md               # プロジェクトドキュメント
├── .gitignore             # Git除外設定
├── src/                   # ソースコード
│   ├── lib.rs             # ライブラリエントリポイント
│   ├── main.rs            # サーバー起動（メイン）
│   ├── models.rs          # データモデル（Vector3, Character, Player, MatchingSession等）
│   ├── utils.rs           # ユーティリティ関数
│   ├── db/                # データベース関連
│   │   ├── mod.rs
│   │   └── models.rs      # DBモデル
│   ├── game/              # ゲームロジック
│   │   ├── mod.rs
│   │   ├── state.rs       # ゲーム状態管理
│   │   └── manager.rs     # 60Hzゲームループ管理
│   └── handlers/          # HTTPハンドラー
│       ├── mod.rs
│       ├── matching.rs    # マッチングAPI（REST）
│       ├── model_upload.rs # 3Dモデルアップロード
│       └── websocket.rs   # WebSocketハンドラー
├── tests/                 # テストコード
│   ├── api_test.rs        # REST APIテスト
│   └── websocket_test.rs  # WebSocketテスト
├── scripts/               # スクリプト
│   ├── test_api.sh        # APIテストスクリプト
│   └── demo_model_download.sh
├── doc/                   # ドキュメント
│   ├── specification.md
│   ├── testing-guide.md
│   ├── websocket-messages.md
│   ├── project_overview.md
│   ├── development_commands.md
│   └── style_and_conventions.md
├── data/                  # データディレクトリ
└── uploads/               # アップロードファイル保存先
```

## 主要モジュール

### models.rs

- Vector3: 3D座標
- Character: キャラクター情報
- Player: プレイヤー状態
- MatchingSession: マッチングセッション
- GameState: ゲーム状態
- WsMessage: WebSocketメッセージ型定義
- 各種リクエスト/レスポンス型

### handlers/

- matching.rs: マッチング作成・参加API
- websocket.rs: WebSocket接続・メッセージ処理
- model_upload.rs: 3Dモデルアップロード

### game/

- state.rs: ゲーム状態管理ロジック
- manager.rs: Actixアクターによる60Hzゲームループ
