#!/usr/bin/env bash
set -euo pipefail

BIN_NAME="tick"
REPO="${TICK_INSTALL_REPO:-https://github.com/0xBoji/task_invocation_cron_kernel.git}"
REF="${TICK_INSTALL_REF:-main}"

BOLD="$(tput bold 2>/dev/null || echo '')"
GREY="$(tput setaf 8 2>/dev/null || echo '')"
BLUE="$(tput setaf 4 2>/dev/null || echo '')"
CYAN="$(tput setaf 6 2>/dev/null || echo '')"
GREEN="$(tput setaf 2 2>/dev/null || echo '')"
YELLOW="$(tput setaf 3 2>/dev/null || echo '')"
RED="$(tput setaf 1 2>/dev/null || echo '')"
RESET="$(tput sgr0 2>/dev/null || echo '')"

info() { echo -e "${CYAN}${BOLD}info:${RESET} $1"; }
warn() { echo -e "${YELLOW}${BOLD}warn:${RESET} $1" >&2; }
error() { echo -e "${RED}${BOLD}error:${RESET} $1" >&2; }
success() { echo -e "${GREEN}${BOLD}success:${RESET} $1"; }

banner() {
  cat <<'EOF'

████████╗██╗ ██████╗██╗  ██╗
╚══██╔══╝██║██╔════╝██║ ██╔╝
   ██║   ██║██║     █████╔╝
   ██║   ██║██║     ██╔═██╗
   ██║   ██║╚██████╗██║  ██╗
   ╚═╝   ╚═╝ ╚═════╝╚═╝  ╚═╝
EOF

  echo -e "${BLUE}╔══════════════════════════════════════════════════════╗"
  echo -e "║ Task Invocation Cron Kernel • mesh-aware cron runner ║"
  echo -e "╚══════════════════════════════════════════════════════╝${RESET}"
  echo
  echo -e "${BOLD}TICK Installer${RESET}"
  echo -e "${GREY}Installs the tick CLI from GitHub with Cargo${RESET}"
  echo
}

usage() {
  cat <<EOF
Usage:
  install.sh [options]

Options:
  -h, --help   Show this help message

Environment:
  TICK_INSTALL_REPO   Git repository to install from
  TICK_INSTALL_REF    Git branch or ref to install

Examples:
  curl -fsSL https://raw.githubusercontent.com/0xBoji/task_invocation_cron_kernel/main/scripts/install.sh | bash
  TICK_INSTALL_REF=main curl -fsSL https://raw.githubusercontent.com/0xBoji/task_invocation_cron_kernel/main/scripts/install.sh | bash
EOF
}

require_cmd() {
  if command -v "$1" >/dev/null 2>&1; then
    return 0
  fi

  error "required command not found: $1"
  if [[ "$1" == "cargo" ]]; then
    info "Rust/Cargo is required to build ${BIN_NAME}."
    info "Install it from https://rustup.rs/ and try again."
  fi
  exit 1
}

case "${1:-}" in
  -h|--help)
    banner
    usage
    exit 0
    ;;
esac

banner
require_cmd cargo
require_cmd git

info "Installing ${BOLD}${BIN_NAME}${RESET} from ${REPO} @ ${REF}"
cargo install \
  --git "${REPO}" \
  --branch "${REF}" \
  --bin "${BIN_NAME}" \
  --locked \
  --force

echo
success "Installed ${BOLD}${BIN_NAME}${RESET} successfully."
warn "Ensure Cargo bin dir is on PATH (typically \$HOME/.cargo/bin)."
echo
info "Try it out:"
echo -e "  ${BOLD}${BIN_NAME} daemon --help${RESET}"
echo -e "  ${BOLD}${BIN_NAME} add --help${RESET}"
