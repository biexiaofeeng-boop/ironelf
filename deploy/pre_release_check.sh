#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

GATEWAY_BASE="${GATEWAY_BASE:-http://127.0.0.1:3000}"
AUTH_TOKEN="${AUTH_TOKEN:-dev-token}"
SKIP_BUILD="${SKIP_BUILD:-false}"
SKIP_HTTP="${SKIP_HTTP:-false}"

usage() {
  cat <<EOF
Usage:
  bash deploy/pre_release_check.sh
  SKIP_BUILD=true bash deploy/pre_release_check.sh
  AUTH_TOKEN=xxx GATEWAY_BASE=http://127.0.0.1:3000 bash deploy/pre_release_check.sh

Checks:
  - current git branch / commit / remotes
  - cargo build (unless SKIP_BUILD=true)
  - macOS service status script
  - GET /api/gateway/status
  - GET /api/runtime/health
  - GET /v1/models
EOF
}

if [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

echo "==> Repo baseline"
echo "repo: ${REPO_ROOT}"
echo "branch: $(git -C "${REPO_ROOT}" branch --show-current)"
echo "commit: $(git -C "${REPO_ROOT}" rev-parse HEAD)"
git -C "${REPO_ROOT}" remote -v
echo ""

if [ "${SKIP_BUILD}" != "true" ]; then
  echo "==> cargo build"
  (
    cd "${REPO_ROOT}"
    cargo build
  )
  echo ""
fi

echo "==> Service status"
(
  cd "${REPO_ROOT}"
  bash deploy/macos-service.sh status
)
echo ""

if [ "${SKIP_HTTP}" != "true" ]; then
  auth_header="Authorization: Bearer ${AUTH_TOKEN}"
  echo "==> GET ${GATEWAY_BASE}/api/gateway/status"
  curl -fsS -H "${auth_header}" "${GATEWAY_BASE}/api/gateway/status"
  echo ""
  echo ""

  echo "==> GET ${GATEWAY_BASE}/api/runtime/health"
  curl -fsS -H "${auth_header}" "${GATEWAY_BASE}/api/runtime/health"
  echo ""
  echo ""

  echo "==> GET ${GATEWAY_BASE}/v1/models"
  curl -fsS -H "${auth_header}" "${GATEWAY_BASE}/v1/models"
  echo ""
fi
