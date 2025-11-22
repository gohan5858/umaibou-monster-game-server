# WebSocketメッセージサンプル集

wscatでのテスト時にコピー&ペーストで使用できるメッセージ集です。

## 接続コマンド

```bash
# プレイヤーA（新規作成）
wscat -c "ws://localhost:8080/ws?player_id=player_a"

# プレイヤーB（参加）
wscat -c "ws://localhost:8080/ws?player_id=player_b"
```

---

## クライアント → サーバー

### 1. マッチング作成

#### ユーザー名あり

```json
{"type":"CreateMatching","data":{"username":"Taro"}}
```

#### ユーザー名なし

```json
{"type":"CreateMatching","data":{"username":null}}
```

### 2. マッチング参加

```json
{"type":"JoinMatch","data":{"matching_id":"<MATCHING_ID>"}}
```

### 3. 準備完了（キャラクター選択）

#### 戦士キャラクター

```json
{"type":"Ready","data":{"selected_model_id":"character_warrior"}}
```

#### 魔法使いキャラクター

```json
{"type":"Ready","data":{"selected_model_id":"character_mage"}}
```

### 4. 操作入力 - 移動

#### 前進（Z軸正方向）

```json
{"type":"Input","data":{"action":{"Move":{"direction":{"x":0,"y":0,"z":1},"speed":5}}}}
```

#### 後退（Z軸負方向）

```json
{"type":"Input","data":{"action":{"Move":{"direction":{"x":0,"y":0,"z":-1},"speed":5}}}}
```

#### 右移動（X軸正方向）

```json
{"type":"Input","data":{"action":{"Move":{"direction":{"x":1,"y":0,"z":0},"speed":5}}}}
```

#### 左移動（X軸負方向）

```json
{"type":"Input","data":{"action":{"Move":{"direction":{"x":-1,"y":0,"z":0},"speed":5}}}}
```

### 5. 操作入力 - 回転

#### Y軸回転（右向き45度）

```json
{"type":"Input","data":{"action":{"Rotate":{"rotation":{"x":0,"y":45,"z":0}}}}}
```

### 6. 操作入力 - 攻撃

#### 正面攻撃

```json
{"type":"Input","data":{"action":{"Attack":{"target_position":{"x":0,"y":0,"z":10}}}}}
```

---

## サーバー → クライアント（受信メッセージ）

### 1. MatchingCreated

マッチング作成完了通知

```json
{
  "type": "MatchingCreated",
  "data": {
    "matching_id": "550e8400-e29b-41d4-a716-446655440000",
    "current_matchings": [
      {
        "matching_id": "...",
        "creator_username": "Hanako",
        "created_at": "2025-11-22T14:00:00Z",
        "status": "Waiting"
      }
    ],
    "timestamp": "2025-11-22T14:30:00Z"
  }
}
```

### 2. UpdateMatchings

マッチング一覧更新通知（他のプレイヤーが待機中になった時など）

```json
{
  "type": "UpdateMatchings",
  "data": {
    "current_matchings": [
      {
        "matching_id": "550e8400-e29b-41d4-a716-446655440000",
        "creator_username": "Taro",
        "created_at": "2025-11-22T14:30:00Z",
        "status": "Waiting"
      }
    ],
    "timestamp": "2025-11-22T14:30:05Z"
  }
}
```

### 3. MatchingEstablished

マッチング成立通知

```json
{
  "type": "MatchingEstablished",
  "data": {
    "matching_id": "550e8400-e29b-41d4-a716-446655440000",
    "opponent_id": "player_b",
    "model_data": null,
    "timestamp": "2025-11-22T14:31:00Z"
  }
}
```

### 4. OpponentCharacterSelected

相手のキャラクター選択通知

```json
{
  "type": "OpponentCharacterSelected",
  "data": {
    "character": {
      "model_id": "character_warrior",
      "position": {"x": 0.0, "y": 0.0, "z": 0.0},
      "rotation": {"x": 0.0, "y": 0.0, "z": 0.0},
      "hp": 100,
      "max_hp": 100
    },
    "timestamp": "2025-11-22T14:31:05Z"
  }
}
```

### 5. GameStart

ゲーム開始通知

```json
{
  "type": "GameStart",
  "data": {
    "opponent_character": {
      "model_id": "character_mage",
      "position": {"x": 0.0, "y": 0.0, "z": 0.0},
      "rotation": {"x": 0.0, "y": 0.0, "z": 0.0},
      "hp": 100,
      "max_hp": 100
    },
    "your_player_id": "player_a",
    "timestamp": "2025-11-22T14:31:10Z"
  }
}
```

### 6. OpponentStateUpdate

相手の状態更新

```json
{
  "type": "OpponentStateUpdate",
  "data": {
    "opponent": {
      "model_id": "character_mage",
      "position": {"x": -2.1, "y": 0.0, "z": 1.8},
      "rotation": {"x": 0.0, "y": -30.0, "z": 0.0},
      "hp": 92,
      "max_hp": 100
    },
    "timestamp": "2025-11-22T14:31:10.123456Z"
  }
}
```

### 7. GameEnd

ゲーム終了・結果通知

```json
{
  "type": "GameEnd",
  "data": {
    "result": {
      "matching_id": "550e8400-e29b-41d4-a716-446655440000",
      "winner_id": "player_a",
      "loser_id": "player_b",
      "player_a_id": "player_a",
      "player_b_id": "player_b",
      "play_time_seconds": 120,
      "finished_at": "2025-11-22T14:33:10Z"
    },
    "timestamp": "2025-11-22T14:33:10Z"
  }
}
```

### 8. Error

エラー通知

```json
{
  "type": "Error",
  "data": {
    "message": "Invalid input format"
  }
}
```

---

## テストシナリオ例

### シナリオ1: 基本的なゲームフロー

```bash
# 1. WebSocket接続（Player A）
wscat -c "ws://localhost:8080/ws?player_id=player_a"

# 2. マッチング作成
> {"type":"CreateMatching","data":{"username":"Taro"}}

# 3. 受信: MatchingCreated

# 4. WebSocket接続（Player B）
wscat -c "ws://localhost:8080/ws?player_id=player_b"

# 5. 受信: UpdateMatchings（マッチング一覧確認）

# 6. マッチング参加
> {"type":"JoinMatch","data":{"matching_id":"<MATCHING_ID>"}}

# 7. 受信: MatchingEstablished（両者）

# 8. 準備完了（キャラクター選択）
> {"type":"Ready","data":{"selected_model_id":"character_warrior"}}

# 9. 受信: OpponentCharacterSelected（相手が選択した後）

# 10. 受信: GameStart（両者準備完了後）

# 11. 移動
> {"type":"Input","data":{"action":{"Move":{"direction":{"x":1,"y":0,"z":0},"speed":5}}}}

# 12. 受信: OpponentStateUpdate（相手の移動を受信）
```

### シナリオ2: 連続移動

```bash
# 前進開始
> {"type":"Input","data":{"action":{"Move":{"direction":{"x":0,"y":0,"z":1},"speed":5}}}}

# 右に曲がる
> {"type":"Input","data":{"action":{"Rotate":{"rotation":{"x":0,"y":45,"z":0}}}}}

# 移動継続
> {"type":"Input","data":{"action":{"Move":{"direction":{"x":0,"y":0,"z":1},"speed":5}}}}

# 停止（速度0）
> {"type":"Input","data":{"action":{"Move":{"direction":{"x":0,"y":0,"z":0},"speed":0}}}}
```

---

## トラブルシューティング

### JSONパースエラー

❌ **間違い:**

```json
{'type':'Ready'}  // シングルクォート
{"type":"Ready",}  // 末尾カンマ
{type:"Ready"}    // キー名にクォートなし
```

✅ **正しい:**

```json
{"type":"Ready"}
```

### メッセージが送信できない

- wscatで接続後、`>`プロンプトが表示されていることを確認
- メッセージは1行で入力
- Enterキーで送信

### サーバーからの応答がない

- サーバーログを確認
- `player_id`と`matching_id`が正しいか確認
- 両プレイヤーが接続しているか確認
