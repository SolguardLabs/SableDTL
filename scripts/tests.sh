#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

for candidate in "$HOME/.cargo/bin" /mnt/c/Users/*/.cargo/bin "/mnt/c/Program Files/nodejs"; do
  if [ -d "$candidate" ]; then
    export PATH="$candidate:$PATH"
  fi
done

run_cargo() {
  if command -v cargo >/dev/null 2>&1; then
    cargo "$@"
  elif command -v cargo.exe >/dev/null 2>&1; then
    cargo.exe "$@"
  elif command -v cmd.exe >/dev/null 2>&1; then
    cmd.exe /d /c cargo "$@"
  else
    echo "cargo no esta disponible en PATH" >&2
    return 127
  fi
}

run_node() {
  if command -v node >/dev/null 2>&1; then
    node "$@"
  elif command -v node.exe >/dev/null 2>&1; then
    node.exe "$@"
  elif command -v cmd.exe >/dev/null 2>&1; then
    cmd.exe /d /c node "$@"
  else
    echo "node no esta disponible en PATH" >&2
    return 127
  fi
}

run_cargo test --locked
run_node --test tests/node/*.test.js
