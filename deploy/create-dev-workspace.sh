#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

DEFAULT_BRANCH="$(git -C "${REPO_ROOT}" branch --show-current)"
DEFAULT_REMOTE_URL="git@github.com:biexiaofeeng-boop/chimera-iceclaw.git"
DEFAULT_LEGACY_REMOTE_URL="git@github.com:biexiaofeeng-boop/ironelf.git"
DEFAULT_DEV_DIR="/Users/sourcefire/X-lab/chimera-iceclaw-dev"

BRANCH="${BRANCH:-${DEFAULT_BRANCH}}"
DEV_DIR="${DEV_DIR:-${DEFAULT_DEV_DIR}}"
REMOTE_URL="${REMOTE_URL:-${DEFAULT_REMOTE_URL}}"
LEGACY_REMOTE_URL="${LEGACY_REMOTE_URL:-${DEFAULT_LEGACY_REMOTE_URL}}"

usage() {
  cat <<EOF
Usage:
  bash deploy/create-dev-workspace.sh
  BRANCH=main bash deploy/create-dev-workspace.sh
  DEV_DIR=/Users/sourcefire/X-lab/chimera-iceclaw-dev bash deploy/create-dev-workspace.sh

Behavior:
  - Clones ${REMOTE_URL} into ${DEFAULT_DEV_DIR} if missing
  - Ensures origin points to chimera-iceclaw
  - Ensures ironelf-legacy remote exists for rollback/reference
  - Checks out the requested branch and fast-forwards to origin/<branch>
EOF
}

if [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

if [ ! -d "${DEV_DIR}/.git" ]; then
  echo "==> Cloning dev workspace into ${DEV_DIR}"
  git clone "${REMOTE_URL}" "${DEV_DIR}"
else
  echo "==> Reusing existing dev workspace ${DEV_DIR}"
fi

echo "==> Aligning remotes"
git -C "${DEV_DIR}" remote set-url origin "${REMOTE_URL}"
if git -C "${DEV_DIR}" remote get-url ironelf-legacy >/dev/null 2>&1; then
  git -C "${DEV_DIR}" remote set-url ironelf-legacy "${LEGACY_REMOTE_URL}"
else
  git -C "${DEV_DIR}" remote add ironelf-legacy "${LEGACY_REMOTE_URL}"
fi

echo "==> Fetching origin"
git -C "${DEV_DIR}" fetch origin --prune

if git -C "${DEV_DIR}" show-ref --verify --quiet "refs/remotes/origin/${BRANCH}"; then
  if git -C "${DEV_DIR}" show-ref --verify --quiet "refs/heads/${BRANCH}"; then
    git -C "${DEV_DIR}" checkout "${BRANCH}"
  else
    git -C "${DEV_DIR}" checkout -b "${BRANCH}" "origin/${BRANCH}"
  fi

  echo "==> Fast-forwarding ${BRANCH}"
  git -C "${DEV_DIR}" merge --ff-only "origin/${BRANCH}"
else
  echo "ERROR: branch origin/${BRANCH} not found in ${REMOTE_URL}"
  exit 1
fi

echo ""
echo "==> Dev workspace ready"
echo "dir: ${DEV_DIR}"
echo "branch: $(git -C "${DEV_DIR}" branch --show-current)"
echo "commit: $(git -C "${DEV_DIR}" rev-parse HEAD)"
echo "origin: $(git -C "${DEV_DIR}" remote get-url origin)"
echo "legacy: $(git -C "${DEV_DIR}" remote get-url ironelf-legacy)"
git -C "${DEV_DIR}" status --short --branch
