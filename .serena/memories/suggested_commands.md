# 推奨コマンド一覧

## プロジェクト実行

### サーバー起動
```bash
cargo run
```
→ サーバーが `http://0.0.0.0:8080` で起動

### ビルド
```bash
# デバッグビルド
cargo build

# リリースビルド（最適化）
cargo build --release
```

## テスト

### 全テスト実行
```bash
cargo test
```

### 個別テスト実行
```bash
# REST APIテスト
cargo test --test api_test

# WebSocketテスト
cargo test --test websocket_test

# 統合テスト
cargo test --test integration_test
```

### テストスクリプト
```bash
./scripts/run_tests.sh  # （存在する場合）
```

## コード品質

### フォーマット
```bash
# コード整形
cargo fmt

# フォーマットチェック（CI用）
cargo fmt -- --check
```

### リント
```bash
# 静的解析・スタイルチェック
cargo clippy

# 警告をエラーとして扱う（CI用）
cargo clippy -- -D warnings
```

## 依存関係管理

### 依存関係更新
```bash
# Cargo.lockを更新
cargo update

# 特定のクレート更新
cargo update -p <crate-name>
```

### 依存関係確認
```bash
# 依存ツリー表示
cargo tree
```

## ドキュメント生成

```bash
# ドキュメント生成・ブラウザで開く
cargo doc --open
```

## クリーンアップ

```bash
# ビルド成果物削除
cargo clean
```

## 開発ワークフロー

タスク完了時の推奨フロー:
1. `cargo fmt` - コード整形
2. `cargo clippy` - 静的解析
3. `cargo test` - テスト実行
4. `cargo build` - ビルド確認
