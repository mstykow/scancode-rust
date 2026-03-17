#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")/.."

usage() {
    cat <<'EOF'
Usage:
  ./scripts/cargo_isolated.sh [--name <name>] <cargo-subcommand> [args...]

Runs a Cargo command with an isolated target directory under:
  target/isolated/<name>

Use this only when you intentionally want multiple Cargo commands to run in
parallel on the same machine without contending on the default target dir.

Examples:
  ./scripts/cargo_isolated.sh --name golden test --lib --features golden-tests
  ./scripts/cargo_isolated.sh golden test --lib --features golden-tests
  ./scripts/cargo_isolated.sh test --lib --no-run
  ./scripts/cargo_isolated.sh cli run --bin scancode-rust -- --help
EOF
}

auto_name() {
    local candidate=''
    local env_key

    for env_key in \
        OPENCODE_SESSION_ID \
        OPENCODE_TASK_ID \
        OMO_SESSION_ID \
        OMO_TASK_ID \
        OPENCODE_AGENT_NAME \
        OMO_AGENT_NAME \
        AGENT_NAME; do
        candidate="${!env_key:-}"
        if [[ -n "$candidate" ]]; then
            break
        fi
    done

    if [[ -z "$candidate" ]]; then
        candidate="pid-${BASHPID:-$$}"
    else
        candidate="${candidate}-pid-${BASHPID:-$$}"
    fi

    printf '%s' "$candidate"
}

if (( $# < 1 )); then
    usage >&2
    exit 1
fi

if [[ "${1:-}" == "--name" ]]; then
    if (( $# < 3 )); then
        usage >&2
        exit 1
    fi

    name="$2"
    shift 2
else
    name="$(auto_name)"
fi

if (( $# < 1 )); then
    usage >&2
    exit 1
fi

safe_name="$(printf '%s' "$name" | tr -cs '[:alnum:]._-' '-')"
safe_name="${safe_name#-}"
safe_name="${safe_name%-}"

if [[ -z "$safe_name" ]]; then
    echo "Isolated target-dir name must contain at least one alphanumeric character" >&2
    exit 1
fi

export CARGO_TARGET_DIR="target/isolated/${safe_name}"

printf '==> CARGO_TARGET_DIR=%q cargo' "$CARGO_TARGET_DIR"
for arg in "$@"; do
    printf ' %q' "$arg"
done
printf '\n'

exec cargo "$@"
