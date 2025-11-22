#!/bin/bash
set -e  # エラーで即座に終了

# 環境変数の確認
: "${DEPLOY_SERVER:?DEPLOY_SERVER environment variable is required}"
: "${DEPLOY_NODE:?DEPLOY_NODE environment variable is required}"
: "${DEPLOY_PATH:?DEPLOY_PATH environment variable is required}"
: "${DEPLOY_USER:?DEPLOY_USER environment variable is required}"

# カラー出力
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Starting deployment to ${DEPLOY_SERVER}${NC}"
echo -e "${GREEN}========================================${NC}"

# デプロイ先のパス
REMOTE_USER="${DEPLOY_USER}"
REMOTE_HOST="${DEPLOY_SERVER}"
REMOTE_PATH="/home/${REMOTE_USER}/${DEPLOY_PATH}/umaibou-monster-game-server"
BACKUP_PATH="${REMOTE_PATH}.backup"

# 1. リモートサーバーでディレクトリ準備
echo -e "${YELLOW}Step 1: Preparing remote directory...${NC}"
tsh ssh ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
    # ディレクトリが存在しない場合は作成
    mkdir -p ~/Projects/umaibou-monster-game-server
    cd ~/Projects/umaibou-monster-game-server

    # 既存のバックアップがあれば削除
    if [ -d "../umaibou-monster-game-server.backup" ]; then
        rm -rf "../umaibou-monster-game-server.backup"
    fi
EOF

# 2. 実行中のサービスを停止（存在する場合）
echo -e "${YELLOW}Step 2: Stopping existing service...${NC}"
tsh ssh ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
    # プロセスが実行中か確認
    if pgrep -f umaibou-monster-game-server > /dev/null; then
        echo "Stopping umaibou-monster-game-server..."
        pkill -TERM -f umaibou-monster-game-server || true
        sleep 2
        # 強制終了が必要な場合
        if pgrep -f umaibou-monster-game-server > /dev/null; then
            pkill -KILL -f umaibou-monster-game-server || true
        fi
        echo "Service stopped"
    else
        echo "Service is not running"
    fi
EOF

# 3. 現在のバージョンをバックアップ
echo -e "${YELLOW}Step 3: Creating backup of current version...${NC}"
tsh ssh ${REMOTE_USER}@${REMOTE_HOST} << EOF
    cd ~/Projects
    if [ -d "umaibou-monster-game-server/target" ]; then
        echo "Creating backup..."
        cp -r umaibou-monster-game-server umaibou-monster-game-server.backup
        echo "Backup created"
    else
        echo "No existing installation found, skipping backup"
    fi
EOF

# 4. バイナリとアセットをアップロード
echo -e "${YELLOW}Step 4: Uploading new version...${NC}"

# バイナリをアップロード
echo "Uploading binary..."
tsh scp \
    target/release/umaibou-monster-game-server \
    ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_PATH}/

# マイグレーションファイルをアップロード（存在する場合）
if [ -d "migrations" ]; then
    echo "Uploading migrations..."
    tsh scp -r \
        migrations \
        ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_PATH}/
fi

# dataディレクトリをアップロード（存在する場合）
if [ -d "data" ]; then
    echo "Uploading data directory..."
    tsh scp -r \
        data \
        ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_PATH}/
fi

# 5. パーミッション設定
echo -e "${YELLOW}Step 5: Setting permissions...${NC}"
tsh ssh ${REMOTE_USER}@${REMOTE_HOST} << EOF
    cd ${REMOTE_PATH}
    chmod +x umaibou-monster-game-server
    echo "Permissions set"
EOF

# 6. データベースマイグレーション実行（必要な場合）
echo -e "${YELLOW}Step 6: Running database migrations...${NC}"
tsh ssh ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
    cd ~/Projects/umaibou-monster-game-server
    # SQLxマイグレーションの実行
    if [ -f "umaibou-monster-game-server" ] && [ -d "migrations" ]; then
        # DATABASE_URLが設定されていない場合のデフォルト値
        export DATABASE_URL="${DATABASE_URL:-sqlite://data/game.db}"

        # マイグレーションの実行は手動で行う想定
        # （sqlx-cliが必要なため）
        echo "Migration files deployed. Please run migrations manually if needed:"
        echo "  sqlx migrate run"
    fi
EOF

# 7. サービスを起動
echo -e "${YELLOW}Step 7: Starting service...${NC}"
tsh ssh ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
    cd ~/Projects/umaibou-monster-game-server

    # バックグラウンドでサービスを起動
    nohup ./umaibou-monster-game-server > server.log 2>&1 &

    # プロセスIDを保存
    echo $! > server.pid

    echo "Service started with PID: $(cat server.pid)"
EOF

# 8. ヘルスチェック
echo -e "${YELLOW}Step 8: Health check...${NC}"
sleep 3  # サービス起動を待つ

tsh ssh ${REMOTE_USER}@${REMOTE_HOST} << 'EOF'
    cd ~/Projects/umaibou-monster-game-server

    # プロセスが実行中か確認
    if pgrep -f umaibou-monster-game-server > /dev/null; then
        echo "✓ Service is running"

        # ポート8080がリッスンしているか確認
        if ss -tuln | grep -q :8080; then
            echo "✓ Service is listening on port 8080"
        else
            echo "⚠ Warning: Port 8080 is not listening yet"
        fi
    else
        echo "✗ Service failed to start"
        echo "Last 20 lines of log:"
        tail -n 20 server.log
        exit 1
    fi
EOF

# 9. デプロイ完了
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Deployment completed successfully!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "Server: ${REMOTE_HOST}"
echo "Path: ${REMOTE_PATH}"
echo "Service status: Running"
echo ""
echo "To view logs:"
echo "  tsh ssh ${REMOTE_USER}@${REMOTE_HOST} 'tail -f ~/Projects/umaibou-monster-game-server/server.log'"
echo ""
echo "To rollback:"
echo "  Run: scripts/rollback.sh"
