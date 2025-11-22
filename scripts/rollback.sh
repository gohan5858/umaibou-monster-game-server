#!/bin/bash
set -e

# 環境変数のデフォルト値
DEPLOY_USER="${DEPLOY_USER:-$(whoami)}"
DEPLOY_SERVER="${DEPLOY_SERVER:-ct108}"
DEPLOY_PATH="Projects/umaibou-monster-game-server"

# カラー出力
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}Rolling back to previous version${NC}"
echo -e "${YELLOW}========================================${NC}"

# バックアップの存在確認
echo "Checking for backup..."
BACKUP_EXISTS=$(tsh ssh ${DEPLOY_USER}@${DEPLOY_SERVER} "[ -d ~/Projects/umaibou-monster-game-server.backup ] && echo 'yes' || echo 'no'")

if [ "$BACKUP_EXISTS" != "yes" ]; then
    echo -e "${RED}Error: No backup found. Cannot rollback.${NC}"
    exit 1
fi

# サービス停止
echo -e "${YELLOW}Stopping current service...${NC}"
tsh ssh ${DEPLOY_USER}@${DEPLOY_SERVER} << 'EOF'
    if pgrep -f umaibou-monster-game-server > /dev/null; then
        echo "Stopping service..."
        pkill -TERM -f umaibou-monster-game-server || true
        sleep 2
        if pgrep -f umaibou-monster-game-server > /dev/null; then
            pkill -KILL -f umaibou-monster-game-server || true
        fi
    fi
EOF

# ロールバック実行
echo -e "${YELLOW}Restoring backup...${NC}"
tsh ssh ${DEPLOY_USER}@${DEPLOY_SERVER} << 'EOF'
    cd ~/Projects

    # 現在のバージョンを一時的に保存
    if [ -d "umaibou-monster-game-server" ]; then
        mv umaibou-monster-game-server umaibou-monster-game-server.failed
    fi

    # バックアップを復元
    mv umaibou-monster-game-server.backup umaibou-monster-game-server

    echo "Backup restored"
EOF

# サービス再起動
echo -e "${YELLOW}Starting previous version...${NC}"
tsh ssh ${DEPLOY_USER}@${DEPLOY_SERVER} << 'EOF'
    cd ~/Projects/umaibou-monster-game-server
    nohup ./umaibou-monster-game-server > server.log 2>&1 &
    echo $! > server.pid
    echo "Service restarted"
EOF

# ヘルスチェック
sleep 3
tsh ssh ${DEPLOY_USER}@${DEPLOY_SERVER} << 'EOF'
    if pgrep -f umaibou-monster-game-server > /dev/null; then
        echo "✓ Service is running"
    else
        echo "✗ Service failed to start"
        exit 1
    fi
EOF

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Rollback completed successfully${NC}"
echo -e "${GREEN}========================================${NC}"
