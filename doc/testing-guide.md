# 動作確認手順書

3Dリアルタイム対戦ゲームサーバーの動作確認手順です。

## 📋 目次

1. [環境準備](#環境準備)
2. [サーバー起動](#サーバー起動)
3. [REST API動作確認（レガシー）](#rest-api動作確認レガシー)
4. [🆕 WebSocket動作確認（新仕様）](#websocket動作確認新仕様)
5. [エンドツーエンドフロー確認](#エンドツーエンドフロー確認)
6. [🆕 WebSocketマッチングフローの利点](#websocketマッチングフローの利点)
7. [トラブルシューティング](#トラブルシューティング)
8. [パフォーマンステスト](#パフォーマンステスト)
9. [まとめ](#まとめ)

---

## 環境準備

### 必要なツール

```bash
# Rustがインストールされているか確認
rustc --version

# curlがインストールされているか確認
curl --version

# wscat（WebSocketクライアント）をインストール
npm install -g wscat
```

### プロジェクトビルド

```bash
# プロジェクトディレクトリに移動
cd /Users/gohan/Projects/rust/umaibou-monster-game-server

# ビルド
cargo build

# または、リリースビルド（最適化）
cargo build --release
```

---

## サーバー起動

### 起動コマンド

```bash
# 開発モードで起動
cargo run

# または、リリースビルドで起動
cargo run --release
```

### 起動確認

以下のメッセージが表示されればOK：

```
🎮 Starting 3D Real-time Battle Game Server...
✅ Server initialized
🌐 Listening on http://0.0.0.0:8080
```

### ヘルスチェック

別のターミナルで確認：

```bash
curl http://localhost:8080/
# 404エラーが返れば正常（ルートパスは未定義だが、サーバーは稼働中）
```

---

## REST API動作確認（レガシー）

**注意：** REST APIによるマッチングは旧仕様です。新しいWebSocketベースのマッチングフローを使用してください（次のセクション参照）。

### 1. マッチングID生成（プレイヤーA）- レガシー

```bash
# マッチング作成（非推奨）
curl -X POST http://localhost:8080/api/matching/create \
  -H "Content-Type: application/json" \
  -d '{"player_id": "player_a"}'

# レスポンス例:
# {"matching_id":"550e8400-e29b-41d4-a716-446655440000"}
```

**確認ポイント：**
- ✅ ステータスコード200
- ✅ `matching_id`フィールドが返る（UUID形式）

### 2. マッチング参加（プレイヤーB）- レガシー

```bash
# 上記で取得したmatching_idを使用（非推奨）
curl -X POST http://localhost:8080/api/matching/join \
  -H "Content-Type: application/json" \
  -d '{
    "matching_id": "550e8400-e29b-41d4-a716-446655440000",
    "player_id": "player_b"
  }'

# レスポンス例:
# {"success":true,"message":"Matching successful"}
```

---

## WebSocket動作確認（新仕様）

### 🆕 WebSocketベースのマッチングフロー

**重要な変更点：**
- ✅ REST APIでのマッチング作成・参加は不要
- ✅ WebSocket接続後に`CreateMatching`メッセージを送信
- ✅ リアルタイムでマッチング一覧が更新される
- ✅ `JoinMatch`メッセージでマッチングに参加

### 準備

WebSocketクライアントツールのインストール確認：

```bash
# wscatがインストールされているか確認
wscat --version

# インストールされていない場合
npm install -g wscat
```

### 3Dモデルのアップロード

ゲームで使用する3Dモデル（GLB形式）を事前にアップロードし、`model_id`を取得します。

```bash
# 3Dモデルをアップロード（GLB形式）
curl -X POST http://localhost:8080/api/models/upload \
  -F "file=@/path/to/your/model.glb"

# レスポンス例:
# {
#   "model_id": "9e7d246b-57cd-47de-94f1-4192f3dc075e",
#   "file_name": "model.glb",
#   "file_size": 1024000,
#   "uploaded_at": "2025-11-22T12:00:00.000Z"
# }
```

**確認ポイント:**
- ✅ ファイルサイズ < 50MB
- ✅ GLB形式（model/gltf-binary）
- ✅ `model_id`をメモしておく（キャラクター選択で使用）

**アップロード済みモデル一覧の取得:**

```bash
# 利用可能なモデル一覧を取得
curl http://localhost:8080/api/models

# レスポンス例:
# {
#   "models": [
#     {
#       "model_id": "9e7d246b-57cd-47de-94f1-4192f3dc075e",
#       "file_name": "warrior.glb",
#       "file_size": 1024000,
#       "is_used": false,
#       "uploaded_at": "2025-11-22T12:00:00.000Z"
#     }
#   ]
# }
```

### WebSocket接続とマッチング作成（プレイヤーA）

#### ステップ1: WebSocket接続

```bash
# ターミナル1: プレイヤーA
wscat -c "ws://localhost:8080/ws"
```

**期待される動作：**
- WebSocket接続が確立
- サーバーが自動的にplayer_idを生成

#### ステップ2: マッチング作成

接続後、以下のメッセージを送信：

```json
{"type":"CreateMatching","data":{"username":"Taro"}}
```

**プレイヤーAが受信するメッセージ：**

```json
{
  "type": "MatchingCreated",
  "data": {
    "matching_id": "550e8400-e29b-41d4-a716-446655440000",
    "current_matchings": [
      {
        "matching_id": "e5f6g7h8-...",
        "creator_username": "Hanako",
        "created_at": "2025-11-22T12:00:00Z",
        "status": "Waiting"
      }
    ],
    "timestamp": "2025-11-22T12:34:56.789Z"
  }
}
```

**補足:** `current_matchings`には、現在待機中の他のマッチング情報が含まれます。自分自身のマッチングは含まれません。

**確認ポイント：**
- ✅ `matching_id`が返される（マッチング待機中）
- ✅ `current_matchings`は空（自分以外のマッチング待ち無し）

**matching_idをメモしておく**

**補足:** この時点で、ロビーに接続している他の全プレイヤー（マッチング未参加者）にも`UpdateMatchings`がブロードキャストされます。

### WebSocket接続とマッチング参加（プレイヤーB）

#### ステップ1: WebSocket接続

```bash
# ターミナル2: プレイヤーB
wscat -c "ws://localhost:8080/ws"
```

#### ステップ2: マッチング一覧の自動受信

**プレイヤーBが自動的に受信するメッセージ：**

```json
{
  "type": "UpdateMatchings",
  "data": {
    "current_matchings": [
      {
        "matching_id": "550e8400-e29b-41d4-a716-446655440000",
        "creator_username": "Taro",
        "created_at": "2025-11-22T12:34:56.789Z",
        "status": "Waiting"
      }
    ],
    "timestamp": "2025-11-22T12:35:00.123Z"
  }
}
```

**確認ポイント：**
- ✅ プレイヤーAのmatching_idが表示される

#### ステップ3: マッチング参加

プレイヤーBが以下のメッセージを送信：

```json
{"type":"JoinMatch","data":{"matching_id":"d7f79dec-5a7b-479e-86b2-d05aaac0478e"}}
```

**両プレイヤーが受信するメッセージ：**

```json
{
  "type": "MatchingEstablished",
  "data": {
    "matching_id": "550e8400-e29b-41d4-a716-446655440000",
    "opponent_id": "player_a",
    "timestamp": "2025-11-22T12:35:05.456Z"
  }
}
```

**確認ポイント：**
- ✅ 両プレイヤーに`MatchingEstablished`が届く
- ✅ `opponent_id`で相手のIDが分かる
- ✅ マッチング成立

### キャラクター選択とゲーム開始

#### 1. キャラクター選択（プレイヤーA）

**注意：** 3Dモデルをアップロードしてmodel_idを取得する必要があります。

ターミナル1で送信：

```json
{"type":"Ready","data":{"selected_model_id":"9e7d246b-57cd-47de-94f1-4192f3dc075e"}}
```

**プレイヤーB側で受信確認：**

```json
{
  "type": "OpponentCharacterSelected",
  "data": {
    "character": {
      "model_id": "9e7d246b-57cd-47de-94f1-4192f3dc075e",
      "position": {"x": 0.0, "y": 0.0, "z": 0.0},
      "rotation": {"x": 0.0, "y": 0.0, "z": 0.0},
      "hp": 100,
      "max_hp": 100
    },
    "timestamp": "2025-11-22T12:35:10.123Z"
  }
}
```

#### 2. キャラクター選択（プレイヤーB）

ターミナル2で送信：

```json
{"type":"Ready","data":{"selected_model_id":"7d246bdd-57cd-47de-94f1-4192f3dc075e"}}
```

**プレイヤーA側で受信確認：**

```json
{
  "type": "OpponentCharacterSelected",
  "data": {
    "character": {
      "model_id": "7d246bdd-57cd-47de-94f1-4192f3dc075e",
      "position": {"x": 0.0, "y": 0.0, "z": 0.0},
      "rotation": {"x": 0.0, "y": 0.0, "z": 0.0},
      "hp": 100,
      "max_hp": 100
    },
    "timestamp": "2025-11-22T12:35:12.456Z"
  }
}
```

#### 3. ゲーム開始

両プレイヤーがキャラクター選択（Ready送信）すると、自動的にゲームが開始されます。

**両プレイヤーが受信するメッセージ：**

```json
{
  "type": "GameStart",
  "data": {
    "opponent_character": {
      "model_id": "7d246bdd-57cd-47de-94f1-4192f3dc075e",
      "position": {"x": 0.0, "y": 0.0, "z": 0.0},
      "rotation": {"x": 0.0, "y": 0.0, "z": 0.0},
      "hp": 100,
      "max_hp": 100
    },
    "your_player_id": "player_a",
    "timestamp": "2025-11-22T12:35:12.789Z"
  }
}
```

**確認ポイント：**
- ✅ `opponent_character`に相手のキャラクター情報
- ✅ `your_player_id`で自分のIDが分かる

#### 5. 状態更新の送信テスト（新機能）

ゲーム開始後、**クライアントが位置/回転を更新した時にサーバーに送信**：

```json
{
  "type": "StateUpdate",
  "data": {
    "position": {"x": 5.0, "y": 0.0, "z": 3.0},
    "rotation": {"x": 0.0, "y": 45.0, "z": 0.0}
  }
}
```

**相手プレイヤー側で受信確認：**

```json
{
  "type": "OpponentStateUpdate",
  "data": {
    "opponent": {
      "model_id": "9e7d246b-57cd-47de-94f1-4192f3dc075e",
      "position": {"x": 5.0, "y": 0.0, "z": 3.0},
      "rotation": {"x": 0.0, "y": 45.0, "z": 0.0},
      "hp": 100,
      "max_hp": 100
    },
    "timestamp": "2025-11-22T12:34:56.789Z"
  }
}
```

**重要な変更点：**
- ✅ **60Hz自動状態送信は廃止**されました（60Hzゲームループは勝敗判定のみに使用）
- ✅ **更新時のみ送信**するイベント駆動型に変更
- ✅ ネットワーク負荷が75-90%削減されます

#### 6. 移動入力テスト

プレイヤーAで送信：

```json
{
  "type": "Input",
  "data": {
    "action": {
      "Move": {
        "direction": {"x": 1.0, "y": 0.0, "z": 0.0},
        "speed": 5.0
      }
    }
  }
}
```

**確認：** プレイヤーBが`OpponentStateUpdate`を受信してpositionが変化

**注意：** Inputメッセージは引き続き使用可能で、入力処理後に自動的に相手に通知されます

#### 7. 回転入力テスト

```json
{
  "type": "Input",
  "data": {
    "action": {
      "Rotate": {
        "rotation": {"x": 0.0, "y": 45.0, "z": 0.0}
      }
    }
  }
}
```

#### 8. 攻撃入力テスト

**近距離攻撃:**

```json
{
  "type": "Input",
  "data": {
    "action": {
      "Attack": {
        "attack_type": "Melee",
        "position": {"x": 10.0, "y": 0.0, "z": 5.0},
        "direction": {"x": 1.0, "y": 0.0, "z": 0.0}
      }
    }
  }
}
```

**遠距離攻撃:**

```json
{
  "type": "Input",
  "data": {
    "action": {
      "Attack": {
        "attack_type": "Ranged",
        "position": {"x": 15.0, "y": 2.0, "z": 8.0},
        "direction": {"x": 0.0, "y": 0.0, "z": 1.0}
      }
    }
  }
}
```

**パラメータ:**
- `attack_type`: 攻撃種別（`"Melee"`: 近距離、`"Ranged"`: 遠距離）
- `position`: 攻撃を行った位置（3Dベクトル）
- `direction`: 攻撃の方向（3Dベクトル）

---

## エンドツーエンドフロー確認

### シナリオ: 完全なゲームフロー（新仕様）

```bash
# ターミナル1: サーバー起動
cargo run

# ターミナル2: プレイヤーA WebSocket接続
wscat -c "ws://localhost:8080/ws"

# ターミナル3: プレイヤーB WebSocket接続（ロビー待機）
wscat -c "ws://localhost:8080/ws"

# プレイヤーA: マッチング作成
> {"type":"CreateMatching","data":{"username":"Taro"}}
# => プレイヤーA: MatchingCreatedを受信
# => プレイヤーB: UpdateMatchingsを自動受信（リアルタイム通知）

# プレイヤーB: マッチング参加
> {"type":"JoinMatch","data":{"matching_id":"<matching_id>"}}

# 両方で"MatchingEstablished"を受信確認

# プレイヤーA: キャラクター選択（Ready + model_id）
> {"type":"Ready","data":{"selected_model_id":"9e7d246b-57cd-47de-94f1-4192f3dc075e"}}

# プレイヤーBで"OpponentCharacterSelected"を受信

# プレイヤーB: キャラクター選択（Ready + model_id）
> {"type":"Ready","data":{"selected_model_id":"7d246bdd-57cd-47de-94f1-4192f3dc075e"}}

# プレイヤーAで"OpponentCharacterSelected"を受信

# 両方で"GameStart"を受信確認

# プレイヤーA: 状態更新送信（新機能）
> {"type":"StateUpdate","data":{"position":{"x":5,"y":0,"z":3},"rotation":{"x":0,"y":45,"z":0}}}

# プレイヤーBで"OpponentStateUpdate"受信を確認

# プレイヤーA: 移動入力（従来通り動作）
> {"type":"Input","data":{"action":{"Move":{"direction":{"x":1,"y":0,"z":0},"speed":5}}}}

# プレイヤーBで"OpponentStateUpdate"受信を確認
```

### 確認チェックリスト（新仕様）

**マッチングフロー：**
- [ ] WebSocket接続成功（両プレイヤー）
- [ ] プレイヤーA: CreateMatching送信成功
- [ ] プレイヤーA: MatchingCreated受信（matching_id取得）
- [ ] プレイヤーB: UpdateMatchings自動受信（ロビー待機中に受信することを確認）
- [ ] プレイヤーB: JoinMatch送信成功
- [ ] 両プレイヤー: MatchingEstablished受信確認

**ゲームフロー：**
- [ ] プレイヤーA: Ready送信でキャラクター選択
- [ ] プレイヤーB: OpponentCharacterSelected受信
- [ ] プレイヤーB: Ready送信でキャラクター選択
- [ ] プレイヤーA: OpponentCharacterSelected受信
- [ ] 両プレイヤー: GameStart受信

**状態更新：**
- [ ] StateUpdate送信で相手にOpponentStateUpdate届く（新機能）
- [ ] Input送信で相手にOpponentStateUpdate届く
- [ ] 移動入力でposition変化
- [ ] 回転入力でrotation変化
- [ ] 60Hz自動送信が停止していることを確認（ネットワーク負荷削減）

---

## 🆕 WebSocketマッチングフローの利点

### 従来のREST API方式との比較

**旧方式（REST API）：**
1. プレイヤーA → `POST /api/matching/create` → matching_id取得
2. プレイヤーB → `POST /api/matching/join` → マッチング成立
3. 両プレイヤー → WebSocket接続

**新方式（WebSocket）：**
1. プレイヤーA → WebSocket接続 → `CreateMatching`送信
2. プレイヤーB → WebSocket接続 → `UpdateMatchings`自動受信
3. プレイヤーB → `JoinMatch`送信 → マッチング成立

### メリット

✅ **リアルタイム性の向上**
- マッチング一覧がリアルタイムで更新される
- REST APIのポーリング不要

✅ **接続数の削減**
- マッチングとゲームで同じWebSocket接続を使用
- サーバーリソースの節約

✅ **ユーザー体験の改善**
- マッチング待機中に他のプレイヤーを即座に確認できる
- マッチング成立が即座に通知される

✅ **実装のシンプル化**
- REST APIとWebSocketの切り替え不要
- 統一されたメッセージフォーマット

---

## トラブルシューティング

### サーバーが起動しない

```bash
# ポート8080が使用中かチェック
lsof -i :8080

# プロセスを終了
kill -9 <PID>

# または別のポートを使用（main.rsを編集）
```

### WebSocket接続できない

```bash
# サーバーログを確認
# エラーメッセージをチェック

# wscatが正しくインストールされているか確認
wscat --version

# 別のWebSocketクライアントを試す（Postman, websocat等）
```

### ゲーム状態更新が届かない

**重要：** 新バージョンでは60Hz自動状態送信は廃止されました（60Hzゲームループは勝敗判定のみに使用）。

**確認項目：**
1. 両プレイヤーが準備完了したか
2. キャラクターが選択されているか（有効なmodel_idを使用しているか）
3. **StateUpdateまたはInputメッセージを送信したか**（更新時のみ送信）
4. サーバーログにエラーがないか
5. アップロードした3Dモデルのmodel_idが正しいか確認

### JSONパースエラー

**よくある原因：**
- シングルクォートではなくダブルクォートを使用
- 末尾のカンマ
- フィールド名の誤字

**正しい例：**

```json
{"type":"Ready"}
```

**間違った例：**

```json
{'type':'Ready'}  // シングルクォート
{"type":"Ready",}  // 末尾カンマ
```

---

## パフォーマンステスト

### イベント駆動型更新確認（新機能）

```bash
# WebSocket接続後、StateUpdateを送信
# 期待値: 送信時のみOpponentStateUpdateを受信（60Hz自動状態送信なし）

# ネットワーク負荷測定
# 旧バージョン: 120メッセージ/秒（60Hz × 2プレイヤー）
# 新バージョン: 10-30メッセージ/秒（更新時のみ）
# 削減率: 75-90%

# ゲームループの仕様:
# - 60Hzループは勝敗判定のみに使用（16ms間隔）
# - 状態送信はイベント駆動型（StateUpdate/Inputメッセージ送信時のみ）
```

### 複数セッション同時実行

```bash
# 複数のマッチングセッションを作成
for i in {1..10}; do
  curl -X POST http://localhost:8080/api/matching/create \
    -H "Content-Type: application/json" \
    -d "{\"player_id\":\"player_a_$i\"}" &
done

# サーバーが安定して動作することを確認
```

---

## まとめ

以上の手順で以下を確認できます：

✅ **機能要件**
- **WebSocketベースのリアルタイムマッチング（新機能）**
  - CreateMatching: マッチング作成
  - UpdateMatchings: マッチング一覧の自動更新
  - JoinMatch: マッチング参加
  - MatchingEstablished: マッチング成立通知
- **3Dモデル管理**
  - POST /api/models/upload: GLB形式モデルのアップロード
  - GET /api/models: アップロード済みモデル一覧取得
- キャラクター選択・準備完了（Ready + model_id）
- ゲーム進行管理（入力処理・状態更新）
- **イベント駆動型状態更新（StateUpdate）**

✅ **非機能要件**
- **ネットワーク負荷75-90%削減（イベント駆動型更新）**
- **60Hzゲームループによる勝敗判定**
- **リアルタイムマッチング一覧更新**
- **単一WebSocket接続での完結**
- 複数セッション同時処理
- 更新時のみ通信するイベント駆動型アーキテクチャ

✅ **改善点**
- REST APIポーリング不要
- マッチング待機中のリアルタイム更新
- サーバーリソースの効率化
- 統一されたメッセージフォーマット

問題があれば、サーバーログとクライアント側のメッセージを確認してください。
