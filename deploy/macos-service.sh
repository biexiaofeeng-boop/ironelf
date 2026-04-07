#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BASE_DIR="${CHIMERA_ICECLAW_BASE_DIR:-${IRONCLAW_BASE_DIR:-${HOME}/.ironclaw}}"
RUN_DIR="${BASE_DIR}/run"
LOG_DIR="${BASE_DIR}/logs"
PID_FILE="${RUN_DIR}/ironclaw.pid"
CURRENT_LOG="${LOG_DIR}/ironclaw.current.log"
DEFAULT_RUST_LOG="${RUST_LOG:-ironclaw=info}"
SERVICE_LABEL="${CHIMERA_ICECLAW_SERVICE_LABEL:-chimera-iceclaw}"
LEGACY_LABEL="${IRONCLAW_LEGACY_LABEL:-ironclaw}"

usage() {
  cat <<'EOF'
Usage:
  bash deploy/macos-service.sh start [--build]
  bash deploy/macos-service.sh stop
  bash deploy/macos-service.sh restart [--build]
  bash deploy/macos-service.sh status
  bash deploy/macos-service.sh logs [tail lines]

Notes:
  - Intended for macOS single-machine deployment/development.
  - Canonical service label is chimera-iceclaw; the binary remains ironclaw.
  - Logs are written to ~/.ironclaw/logs/ by default.
  - PID file is stored at ~/.ironclaw/run/ironclaw.pid for compatibility.
EOF
}

resolve_target_dir() {
  if [ -n "${CARGO_TARGET_DIR:-}" ]; then
    printf '%s\n' "${CARGO_TARGET_DIR}"
    return
  fi

  if [ -f "${REPO_ROOT}/.cargo/config.toml" ]; then
    local configured
    configured="$(
      sed -nE 's/^[[:space:]]*target-dir[[:space:]]*=[[:space:]]*"([^"]+)"[[:space:]]*$/\1/p' \
        "${REPO_ROOT}/.cargo/config.toml" | head -n 1
    )"
    if [ -n "${configured}" ]; then
      printf '%s\n' "${configured}"
      return
    fi
  fi

  printf '%s\n' "${REPO_ROOT}/target"
}

target_dir="$(resolve_target_dir)"
binary_path="${target_dir}/debug/ironclaw"

ensure_dirs() {
  mkdir -p "${RUN_DIR}" "${LOG_DIR}"
}

pid_value() {
  if [ -f "${PID_FILE}" ]; then
    tr -d '[:space:]' < "${PID_FILE}"
  fi
}

discover_pid() {
  local pid cmdline

  pid="$(pid_value)"
  if [ -n "${pid}" ] && kill -0 "${pid}" 2>/dev/null; then
    cmdline="$(ps -p "${pid}" -o command= 2>/dev/null || true)"
    if [[ "${cmdline}" == *ironclaw* ]]; then
      printf '%s\n' "${pid}"
      return 0
    fi
  fi

  pid="$(
    ps -axo pid=,command= | awk '
      /\/ironclaw([[:space:]]|$)/ && $0 !~ /macos-service\.sh/ { print $1; exit }
    ' | tr -d '[:space:]'
  )"
  if [ -n "${pid}" ] && kill -0 "${pid}" 2>/dev/null; then
    printf '%s\n' "${pid}"
    return 0
  fi

  return 1
}

is_running() {
  discover_pid >/dev/null 2>&1
}

build_binary() {
  echo "==> Building ${SERVICE_LABEL} (binary: ${LEGACY_LABEL})"
  (
    cd "${REPO_ROOT}"
    CARGO_TARGET_DIR="${target_dir}" cargo build
  )
}

start_service() {
  local build_requested="${1:-false}"
  ensure_dirs

  if is_running; then
    local existing_pid
    existing_pid="$(discover_pid)"
    printf '%s\n' "${existing_pid}" > "${PID_FILE}"
    echo "${SERVICE_LABEL} is already running (pid ${existing_pid})"
    echo "log: ${CURRENT_LOG}"
    return 0
  fi

  if [ "${build_requested}" = "true" ] || [ ! -x "${binary_path}" ]; then
    build_binary
  fi

  if [ ! -x "${binary_path}" ]; then
    echo "ERROR: binary not found at ${binary_path}"
    exit 1
  fi

  local ts log_file
  ts="$(date '+%Y%m%d-%H%M%S')"
  log_file="${LOG_DIR}/ironclaw-${ts}.log"
  : > "${log_file}"
  ln -sfn "${log_file}" "${CURRENT_LOG}"

  echo "==> Starting ${SERVICE_LABEL}"
  echo "repo: ${REPO_ROOT}"
  echo "binary: ${binary_path}"
  echo "log: ${log_file}"

  (
    cd "${REPO_ROOT}"
    nohup env \
      CLI_ENABLED=false \
      RUST_LOG="${DEFAULT_RUST_LOG}" \
      "${binary_path}" run >> "${log_file}" 2>&1 &
    echo $! > "${PID_FILE}"
  )

  sleep 2
  if is_running; then
    echo "${SERVICE_LABEL} started (pid $(pid_value))"
    echo "tail logs with: bash deploy/macos-service.sh logs"
  else
    echo "ERROR: ${SERVICE_LABEL} failed to start"
    echo "check log: ${log_file}"
    exit 1
  fi
}

stop_service() {
  ensure_dirs
  if ! is_running; then
    rm -f "${PID_FILE}"
    echo "${SERVICE_LABEL} is not running"
    return 0
  fi

  local pid
  pid="$(discover_pid)"
  echo "==> Stopping ${SERVICE_LABEL} (pid ${pid})"
  kill "${pid}" 2>/dev/null || true

  local waited=0
  while kill -0 "${pid}" 2>/dev/null; do
    if [ "${waited}" -ge 20 ]; then
      echo "==> Escalating to SIGKILL"
      kill -9 "${pid}" 2>/dev/null || true
      break
    fi
    sleep 1
    waited=$((waited + 1))
  done

  rm -f "${PID_FILE}"
  echo "${SERVICE_LABEL} stopped"
}

status_service() {
  ensure_dirs
  if is_running; then
    local pid
    pid="$(discover_pid)"
    printf '%s\n' "${pid}" > "${PID_FILE}"
    echo "status: running"
    echo "pid: ${pid}"
    echo "binary: ${binary_path}"
    echo "log: ${CURRENT_LOG}"
    tail -n 20 "${CURRENT_LOG}" 2>/dev/null || true
  else
    echo "status: stopped"
    echo "binary: ${binary_path}"
    echo "log: ${CURRENT_LOG}"
  fi
}

logs_service() {
  ensure_dirs
  local mode="${1:-follow}"
  if [ ! -e "${CURRENT_LOG}" ]; then
    echo "No log file at ${CURRENT_LOG}"
    exit 1
  fi

  if [ "${mode}" = "follow" ]; then
    tail -f "${CURRENT_LOG}"
  else
    tail -n "${mode}" "${CURRENT_LOG}"
  fi
}

cmd="${1:-}"
build_requested="false"

case "${cmd}" in
  start)
    shift
    if [ "${1:-}" = "--build" ]; then
      build_requested="true"
    fi
    start_service "${build_requested}"
    ;;
  stop)
    stop_service
    ;;
  restart)
    shift
    if [ "${1:-}" = "--build" ]; then
      build_requested="true"
    fi
    stop_service
    start_service "${build_requested}"
    ;;
  status)
    status_service
    ;;
  logs)
    shift || true
    logs_service "${1:-follow}"
    ;;
  *)
    usage
    exit 1
    ;;
esac
