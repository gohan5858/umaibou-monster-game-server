# WebSocketメッセージサンプル集

wscatでのテスト時にコピー&ペーストで使用できるメッセージ集です。

## 接続コマンド

```bash
# プレイヤーA
wscat -c "ws://localhost:8080/ws?player_id=player_a&matching_id=<MATCHING_ID>"

# プレイヤーB
wscat -c "ws://localhost:8080/ws?player_id=player_b&matching_id=<MATCHING_ID>"
```

---

## クライアント → サーバー

### 1. キャラクター選択

#### 戦士キャラクター
```json
{"type":"SelectCharacter","data":{"model_id":"character_warrior"}}
```

#### 魔法使いキャラクター
```json
{"type":"SelectCharacter","data":{"model_id":"character_mage"}}
```

#### アーチャーキャラクター
```json
{"type":"SelectCharacter","data":{"model_id":"character_archer"}}
```

### 2. 準備完了

```json
{"type":"Ready"}
```

### 3. 操作入力 - 移動

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

#### 斜め移動（前右）
```json
{"type":"Input","data":{"action":{"Move":{"direction":{"x":0.707,"y":0,"z":0.707},"speed":5}}}}
```

#### ダッシュ（速度10）
```json
{"type":"Input","data":{"action":{"Move":{"direction":{"x":0,"y":0,"z":1},"speed":10}}}}
```

### 4. 操作入力 - 回転

#### Y軸回転（右向き45度）
```json
{"type":"Input","data":{"action":{"Rotate":{"rotation":{"x":0,"y":45,"z":0}}}}}
```

#### Y軸回転（左向き45度）
```json
{"type":"Input","data":{"action":{"Rotate":{"rotation":{"x":0,"y":-45,"z":0}}}}}
```

#### Y軸回転（180度反転）
```json
{"type":"Input","data":{"action":{"Rotate":{"rotation":{"x":0,"y":180,"z":0}}}}}
```

#### X軸回転（上向き30度）
```json
{"type":"Input","data":{"action":{"Rotate":{"rotation":{"x":30,"y":0,"z":0}}}}}
```

### 5. 操作入力 - 攻撃

#### 正面攻撃
```json
{"type":"Input","data":{"action":{"Attack":{"target_position":{"x":0,"y":0,"z":10}}}}}
```

#### 右前方攻撃
```json
{"type":"Input","data":{"action":{"Attack":{"target_position":{"x":5,"y":0,"z":10}}}}}
```

#### 左前方攻撃
```json
{"type":"Input","data":{"action":{"Attack":{"target_position":{"x":-5,"y":0,"z":10}}}}}
```

#### 上空攻撃
```json
{"type":"Input","data":{"action":{"Attack":{"target_position":{"x":0,"y":5,"z":10}}}}}
```

---

## サーバー → クライアント（受信メッセージ）

### 1. MatchingSuccess

マッチング成立通知

```json
{
  "type": "MatchingSuccess",
  "data": {
    "matching_id": "550e8400-e29b-41d4-a716-446655440000",
    "opponent_id": "player_b"
  }
}
```

### 2. OpponentCharacterSelected

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
    }
  }
}
```

### 3. GameStart

ゲーム開始通知

```json
{
  "type": "GameStart",
  "data": {
    "game_state": {
      "matching_id": "550e8400-e29b-41d4-a716-446655440000",
      "player_a": {
        "model_id": "character_warrior",
        "position": {"x": 0.0, "y": 0.0, "z": 0.0},
        "rotation": {"x": 0.0, "y": 0.0, "z": 0.0},
        "hp": 100,
        "max_hp": 100
      },
      "player_b": {
        "model_id": "character_mage",
        "position": {"x": 0.0, "y": 0.0, "z": 0.0},
        "rotation": {"x": 0.0, "y": 0.0, "z": 0.0},
        "hp": 100,
        "max_hp": 100
      },
      "timestamp": "2025-11-19T12:34:56.789123Z"
    }
  }
}
```

### 4. GameStateUpdate

ゲーム状態更新（60Hz配信）

```json
{
  "type": "GameStateUpdate",
  "data": {
    "game_state": {
      "matching_id": "550e8400-e29b-41d4-a716-446655440000",
      "player_a": {
        "model_id": "character_warrior",
        "position": {"x": 1.5, "y": 0.0, "z": 2.3},
        "rotation": {"x": 0.0, "y": 45.0, "z": 0.0},
        "hp": 85,
        "max_hp": 100
      },
      "player_b": {
        "model_id": "character_mage",
        "position": {"x": -2.1, "y": 0.0, "z": 1.8},
        "rotation": {"x": 0.0, "y": -30.0, "z": 0.0},
        "hp": 92,
        "max_hp": 100
      },
      "timestamp": "2025-11-19T12:34:57.123456Z"
    }
  }
}
```

### 5. GameEnd

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
      "finished_at": "2025-11-19T12:36:56.789123Z"
    }
  }
}
```

### 6. Error

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
# 1. WebSocket接続
wscat -c "ws://localhost:8080/ws?player_id=player_a&matching_id=<MATCHING_ID>"

# 2. 受信: MatchingSuccess

# 3. キャラクター選択
> {"type":"SelectCharacter","data":{"model_id":"character_warrior"}}

# 4. 受信: OpponentCharacterSelected（相手が選択した後）

# 5. 準備完了
> {"type":"Ready"}

# 6. 受信: GameStart

# 7. 受信: GameStateUpdate（60Hz）

# 8. 移動
> {"type":"Input","data":{"action":{"Move":{"direction":{"x":1,"y":0,"z":0},"speed":5}}}}

# 9. 受信: GameStateUpdate（位置が変化）

# 10. 攻撃
> {"type":"Input","data":{"action":{"Attack":{"target_position":{"x":10,"y":0,"z":5}}}}}
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

### シナリオ3: 戦闘シミュレーション

```bash
# 接近
> {"type":"Input","data":{"action":{"Move":{"direction":{"x":1,"y":0,"z":1},"speed":5}}}}

# 相手の方を向く
> {"type":"Input","data":{"action":{"Rotate":{"rotation":{"x":0,"y":90,"z":0}}}}}

# 攻撃
> {"type":"Input","data":{"action":{"Attack":{"target_position":{"x":10,"y":0,"z":0}}}}}

# 後退
> {"type":"Input","data":{"action":{"Move":{"direction":{"x":-1,"y":0,"z":0},"speed":5}}}}
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
