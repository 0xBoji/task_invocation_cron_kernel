#!/usr/bin/env bash
set -euo pipefail

REPO="${TICK_INSTALL_REPO:-https://github.com/0xBoji/task_invocation_cron_kernel.git}"
REF="${TICK_INSTALL_REF:-main}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "tick installer: cargo is required but was not found in PATH" >&2
  echo "Install Rust first: https://rustup.rs" >&2
  exit 1
fi

if ! command -v git >/dev/null 2>&1; then
  echo "tick installer: git is required but was not found in PATH" >&2
  exit 1
fi

echo "==> Installing tick from ${REPO} @ ${REF}"
cargo install \
  --git "${REPO}" \
  --branch "${REF}" \
  --bin tick \
  --locked \
  --force

echo "==> tick installed"
echo "==> Ensure Cargo bin dir is on PATH (typically \$HOME/.cargo/bin)"
