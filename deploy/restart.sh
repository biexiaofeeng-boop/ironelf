#!/usr/bin/env bash
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "ERROR: please run as root (for example: sudo bash deploy/restart.sh)"
  exit 1
fi

WITH_PROXY=false
TAIL_LINES="${TAIL_LINES:-80}"

while [ $# -gt 0 ]; do
  case "$1" in
    --with-proxy)
      WITH_PROXY=true
      shift
      ;;
    --tail)
      TAIL_LINES="${2:-80}"
      shift 2
      ;;
    *)
      echo "Usage: sudo bash deploy/restart.sh [--with-proxy] [--tail N]"
      exit 1
      ;;
  esac
done

echo "==> Reloading systemd units"
systemctl daemon-reload

if [ "$WITH_PROXY" = true ]; then
  echo "==> Restarting cloud-sql-proxy"
  systemctl restart cloud-sql-proxy
fi

echo "==> Restarting ironclaw"
systemctl restart ironclaw

echo "==> Service status"
systemctl --no-pager --full status ironclaw

if [ "$WITH_PROXY" = true ]; then
  echo ""
  systemctl --no-pager --full status cloud-sql-proxy
fi

echo ""
echo "==> Recent container logs (last ${TAIL_LINES} lines)"
docker logs --tail "${TAIL_LINES}" ironclaw
