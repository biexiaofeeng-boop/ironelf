#!/usr/bin/env bash
set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
  echo "ERROR: please run as root (for example: sudo bash deploy/restart.sh)"
  exit 1
fi

WITH_PROXY=false
TAIL_LINES="${TAIL_LINES:-80}"
SERVICE_NAME="${SERVICE_NAME:-}"

resolve_service_name() {
  if [ -n "${SERVICE_NAME}" ]; then
    printf '%s\n' "${SERVICE_NAME}"
    return 0
  fi

  if systemctl list-unit-files --type=service 2>/dev/null | grep -q '^chimera-iceclaw\.service'; then
    printf '%s\n' "chimera-iceclaw"
    return 0
  fi

  printf '%s\n' "ironclaw"
}

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
    --service)
      SERVICE_NAME="${2:-}"
      if [ -z "${SERVICE_NAME}" ]; then
        echo "ERROR: --service requires a value"
        exit 1
      fi
      shift 2
      ;;
    *)
      echo "Usage: sudo bash deploy/restart.sh [--with-proxy] [--tail N] [--service chimera-iceclaw|ironclaw]"
      exit 1
      ;;
  esac
done

SERVICE_NAME="$(resolve_service_name)"

echo "==> Reloading systemd units"
systemctl daemon-reload

if [ "$WITH_PROXY" = true ]; then
  echo "==> Restarting cloud-sql-proxy"
  systemctl restart cloud-sql-proxy
fi

echo "==> Restarting ${SERVICE_NAME}"
systemctl restart "${SERVICE_NAME}"

echo "==> Service status"
systemctl --no-pager --full status "${SERVICE_NAME}"

if [ "$WITH_PROXY" = true ]; then
  echo ""
  systemctl --no-pager --full status cloud-sql-proxy
fi

echo ""
echo "==> Recent container logs (last ${TAIL_LINES} lines)"
docker logs --tail "${TAIL_LINES}" "${SERVICE_NAME}" || docker logs --tail "${TAIL_LINES}" ironclaw
