#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

VERIFY=false
if [[ "${1:-}" == "--verify" ]]; then
  VERIFY=true
elif [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<'USAGE'
Usage: ./script/build_and_run.sh [--verify]

Starts the Grimoire Tauri dev app for macOS.

Options:
  --verify   Launch and then check for a Grimoire process.
USAGE
  exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust is required for the Tauri macOS shell, but cargo was not found on PATH." >&2
  echo "Install Rust, then re-run ./script/build_and_run.sh." >&2
  exit 127
fi

if ! command -v npm >/dev/null 2>&1; then
  echo "npm was not found on PATH." >&2
  exit 127
fi

pkill -x "Grimoire" >/dev/null 2>&1 || true
pkill -f "target/debug/grimoire" >/dev/null 2>&1 || true

if [[ ! -d node_modules ]]; then
  npm install
fi

if [[ "$VERIFY" == true ]]; then
  npm run tauri dev &
  TAURI_PID=$!
  for _ in $(seq 1 90); do
    if pgrep -x "Grimoire" >/dev/null 2>&1 || pgrep -f "target/debug/grimoire" >/dev/null 2>&1; then
      echo "Grimoire launched."
      pkill -x "Grimoire" >/dev/null 2>&1 || true
      pkill -f "target/debug/grimoire" >/dev/null 2>&1 || true
      pkill -f "vite --host 127.0.0.1 --port 1420" >/dev/null 2>&1 || true
      kill "$TAURI_PID" >/dev/null 2>&1 || true
      wait "$TAURI_PID" >/dev/null 2>&1 || true
      exit 0
    fi

    if ! kill -0 "$TAURI_PID" >/dev/null 2>&1; then
      echo "Grimoire dev process exited before launch." >&2
      wait "$TAURI_PID"
      exit $?
    fi

    sleep 1
  done

  echo "Grimoire did not appear to launch within 90 seconds." >&2
  kill "$TAURI_PID" >/dev/null 2>&1 || true
  exit 1
else
  npm run tauri dev
fi
