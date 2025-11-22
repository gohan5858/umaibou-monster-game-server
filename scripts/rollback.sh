#!/bin/bash
set -e

# 環境変数のデフォルト値
DEPLOY_USER="${DEPLOY_USER:-$(whoami)}"
DEPLOY_SERVER="${DEPLOY_SERVER:-ct108}"
DEPLOY_PATH="${DEPLOY_PATH:-Projects}"

# カラー出力
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}Rolling back to previous version${NC}"
echo -e "${YELLOW}========================================${NC}"

REMOTE_TARGET="${DEPLOY_USER}@${DEPLOY_SERVER}"
REMOTE_BASE_DIR="/home/${DEPLOY_USER}/${DEPLOY_PATH}"
REMOTE_APP_DIR="${REMOTE_BASE_DIR}/umaibou-monster-game-server"
BACKUP_DIR="${REMOTE_BASE_DIR}/umaibou-monster-game-server.backup"
FAILED_DIR="${REMOTE_BASE_DIR}/umaibou-monster-game-server.failed"

# バックアップの存在確認
echo "Checking for backup..."
BACKUP_EXISTS=$(tsh ssh ${REMOTE_TARGET} "[ -d ${BACKUP_DIR} ] && echo 'yes' || echo 'no'")

if [ "$BACKUP_EXISTS" != "yes" ]; then
    echo -e "${RED}Error: No backup found. Cannot rollback.${NC}"
    exit 1
fi

# サービス停止
echo -e "${YELLOW}Stopping current service...${NC}"
tsh ssh ${REMOTE_TARGET} << EOF
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
tsh ssh ${REMOTE_TARGET} << EOF
    # 現在のバージョンを一時的に保存
    if [ -d "${REMOTE_APP_DIR}" ]; then
        rm -rf "${FAILED_DIR}"
        mv "${REMOTE_APP_DIR}" "${FAILED_DIR}"
    fi

    # バックアップを復元
    mv "${BACKUP_DIR}" "${REMOTE_APP_DIR}"

    echo "Backup restored"
EOF

# サービス再起動
echo -e "${YELLOW}Starting previous version...${NC}"
tsh ssh ${REMOTE_TARGET} << EOF
    cd ${REMOTE_APP_DIR}
    nohup ./umaibou-monster-game-server > server.log 2>&1 &
    echo \$! > server.pid
    echo "Service restarted"
EOF

# ヘルスチェック
sleep 3
tsh ssh ${REMOTE_TARGET} << EOF
    if pgrep -f umaibou-monster-game-server > /dev/null; then
        echo "✓ Service is running"
    else
        echo "✗ Service failed to start"
        exit 1
    fi
EOF

echo -e "${GREEN}Rollback completed successfully${NC}"
