#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")/.."

usage() {
    cat <<'EOF'
Usage:
  ./scripts/dev.sh [all|full|unit|integration|doc|golden|parser-golden] [args...]
  ./scripts/dev.sh cargo <cargo-subcommand> [args...]
  ./scripts/dev.sh isolated [--name <name>] <cargo-subcommand> [args...]
  ./scripts/dev.sh help

Default modes delegate to ./scripts/dev_test.sh:
  all            Run lib + integration + doctests serially.
  full           Run all, then golden tests.
  unit           Run cargo test --lib.
  integration    Run all integration tests, or a selected integration target.
  doc            Run cargo test --doc.
  golden         Run cargo test --lib --features golden-tests.
  parser-golden  Compile parser golden tests once, then run parser filters.
  cargo          Run an arbitrary Cargo subcommand on the shared target dir,
                 protected by the wrapper lock.

The isolated mode delegates to ./scripts/cargo_isolated.sh so parallel Cargo
commands can use separate target directories.

Environment:
  SCANCODE_RUST_DEV_NO_LOCK=1 Disable the shared-target workflow lock.

Examples:
  ./scripts/dev.sh
  ./scripts/dev.sh unit npm_test
  ./scripts/dev.sh cargo clippy --lib --bins --all-features -- -D warnings
  ./scripts/dev.sh parser-golden about cargo
  ./scripts/dev.sh isolated test --lib --no-run
  ./scripts/dev.sh isolated --name golden test --lib --features golden-tests
EOF
}

mode="${1:-all}"
if (( $# > 0 )); then
    shift
fi

shared_lock_dir="${SCANCODE_RUST_DEV_LOCK_DIR:-.scancode-rust-dev-lock}"

lock_owner_is_alive() {
    local owner_pid=''

    if [[ ! -f "$shared_lock_dir/pid" ]]; then
        return 1
    fi

    owner_pid="$(<"$shared_lock_dir/pid")"
    [[ -n "$owner_pid" ]] && kill -0 "$owner_pid" 2>/dev/null
}

with_shared_lock() {
    if [[ "${SCANCODE_RUST_DEV_NO_LOCK:-0}" == "1" ]]; then
        "$@"
        return
    fi

    while ! mkdir "$shared_lock_dir" 2>/dev/null; do
        if ! lock_owner_is_alive; then
            rm -rf "$shared_lock_dir"
            continue
        fi

        local owner_pid='unknown'
        local owner_mode='unknown'

        if [[ -f "$shared_lock_dir/pid" ]]; then
            owner_pid="$(<"$shared_lock_dir/pid")"
        fi
        if [[ -f "$shared_lock_dir/mode" ]]; then
            owner_mode="$(<"$shared_lock_dir/mode")"
        fi

        echo "Another shared-target dev workflow is running (mode=${owner_mode}, pid=${owner_pid}). Waiting for the lock..." >&2
        sleep 1
    done

    printf '%s\n' "${BASHPID:-$$}" > "$shared_lock_dir/pid"
    printf '%s\n' "$mode" > "$shared_lock_dir/mode"
    trap 'rm -rf "$shared_lock_dir"' EXIT INT TERM HUP

    "$@"
}

case "$mode" in
all|full|unit|integration|doc|golden|parser-golden|cargo)
    with_shared_lock ./scripts/dev_test.sh "$mode" "$@"
    ;;
isolated)
    exec ./scripts/cargo_isolated.sh "$@"
    ;;
help|-h|--help)
    usage
    ;;
*)
    echo "Unknown mode: $mode" >&2
    usage >&2
    exit 1
    ;;
esac
