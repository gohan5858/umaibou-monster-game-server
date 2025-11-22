# システムユーティリティ（macOS/Darwin）

## 基本コマンド

### ファイル・ディレクトリ操作

```bash
# ディレクトリ内容表示
ls          # 基本表示
ls -la      # 詳細表示（隠しファイル含む）

# ディレクトリ移動
cd <path>

# 現在のディレクトリパス表示
pwd

# ファイル検索
find . -name "*.rs"                    # 名前で検索
find . -type f -name "Cargo.toml"     # ファイルタイプで検索

# テキスト検索
grep -r "pattern" .                    # 再帰的検索
grep -rn "pattern" src/                # 行番号付き検索
```

### ファイル内容確認

```bash
cat <file>          # ファイル全体表示
head -n 20 <file>   # 先頭20行表示
tail -n 20 <file>   # 末尾20行表示
less <file>         # ページャーで表示
```

### Git操作

```bash
# 状態確認
git status

# 変更確認
git diff
git diff <file>

# ステージング
git add <file>
git add .

# コミット
git commit -m "message"

# ログ確認
git log
git log --oneline

# ブランチ操作
git branch              # ブランチ一覧
git checkout <branch>   # ブランチ切替
git checkout -b <new>   # 新ブランチ作成・切替
```

### プロセス管理

```bash
# プロセス一覧
ps aux | grep cargo

# プロセス終了
kill <PID>
killall <process-name>

# ポート使用確認
lsof -i :<port>         # 例: lsof -i :8080
```

### ネットワーク

```bash
# HTTPリクエスト
curl http://localhost:8080/api/health

# WebSocket接続（wscatがインストールされている場合）
wscat -c "ws://localhost:8080/ws?player_id=test&matching_id=test-id"
```

## macOS特有のコマンド

### Homebrewパッケージ管理

```bash
brew install <package>
brew update
brew upgrade
```

### ファイル監視

```bash
# fswatch（Homebrewでインストール可能）
fswatch -o . | xargs -n1 cargo test
```

## プロジェクト固有

### WebSocketテスト

```bash
# wscatインストール（Node.js必要）
npm install -g wscat

# WebSocket接続
wscat -c "ws://localhost:8080/ws?player_id=player_a&matching_id=<ID>"
```

### ログ確認

```bash
# サーバーログ（cargo runの出力）
cargo run 2>&1 | tee server.log
```

### ポート使用確認

```bash
# 8080ポート使用確認
lsof -i :8080

# プロセス強制終了
lsof -ti :8080 | xargs kill -9
```
