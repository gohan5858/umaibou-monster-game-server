# タスク完了時チェックリスト

## 必須ステップ

タスク完了時には以下を順番に実行してください:

### 1. コード整形

```bash
cargo fmt
```

- Rustの標準スタイルに自動整形
- コミット前に必ず実行

### 2. 静的解析

```bash
cargo clippy
```

- 一般的なミスやスタイル問題を検出
- 警告を全て解消することを推奨

### 3. テスト実行

```bash
cargo test
```

- 全テストが通ることを確認
- 新機能追加時は新しいテストも追加

### 4. ビルド確認

```bash
cargo build
```

- コンパイルエラーがないことを確認
- リリースビルドも確認する場合: `cargo build --release`

## オプション（推奨）

### ドキュメント確認

```bash
cargo doc --open
```

- パブリックAPIのドキュメントが適切か確認

### 依存関係チェック

```bash
cargo tree
```

- 意図しない依存関係が追加されていないか確認

## CI/CD環境での実行

以下のコマンドは継続的インテグレーション環境で使用:

```bash
# フォーマットチェック（変更なし）
cargo fmt -- --check

# Clippy（警告をエラー扱い）
cargo clippy -- -D warnings

# テスト実行
cargo test

# ビルド
cargo build --release
```

## チェックリスト例

- [ ] `cargo fmt` 実行済み
- [ ] `cargo clippy` で警告なし
- [ ] `cargo test` 全テスト通過
- [ ] `cargo build` 成功
- [ ] 新機能の場合、テスト追加済み
- [ ] パブリックAPIにドキュメントコメント追加済み
- [ ] README.mdまたはドキュメント更新（必要な場合）
