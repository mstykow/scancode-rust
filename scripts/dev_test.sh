#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")/.."

usage() {
    cat <<'EOF'
Usage:
  ./scripts/dev_test.sh [all]
  ./scripts/dev_test.sh full
  ./scripts/dev_test.sh unit [cargo-test-filter-or-args...]
  ./scripts/dev_test.sh integration [integration-target-or-filter] [cargo-test-filter-or-args...]
  ./scripts/dev_test.sh doc [cargo-test-args...]
  ./scripts/dev_test.sh golden [cargo-test-filter-or-args...]
  ./scripts/dev_test.sh parser-golden [parser-module-or-filter...] [-- <test-binary-args...>]
  ./scripts/dev_test.sh cargo <cargo-subcommand> [args...]

Modes:
  all            Run common local test phases serially: lib, integration, doctests.
  full           Run `all`, then golden tests.
  unit           Run `cargo test --lib`.
  integration    Run all integration tests, or a specific integration target when
                 the first argument matches `tests/<name>.rs` or `<name>`.
  doc            Run `cargo test --doc`.
  golden         Run `cargo test --lib --features golden-tests`.
  parser-golden  Compile parser golden tests once, then run parser-module filters
                 directly from the built test binary. Accepts bare module names,
                 parser file paths, or exact test filters.
  cargo          Run an arbitrary Cargo subcommand through the wrapper.

Examples:
  ./scripts/dev_test.sh
  ./scripts/dev_test.sh unit npm_test
  ./scripts/dev_test.sh integration scanner_integration test_scanner_discovers_all_registered_parsers
  ./scripts/dev_test.sh integration tests/output_format_golden.rs test_spdx
  ./scripts/dev_test.sh golden cargo_golden
  ./scripts/dev_test.sh parser-golden about cargo
  ./scripts/dev_test.sh parser-golden src/parsers/about_golden_test.rs
  ./scripts/dev_test.sh cargo build
  ./scripts/dev_test.sh parser-golden about -- --nocapture
EOF
}

run_cargo() {
    printf '==> cargo'
    for arg in "$@"; do
        printf ' %q' "$arg"
    done
    printf '\n'

    cargo "$@"
}

resolve_lib_test_binary() {
    local target_dir="${CARGO_TARGET_DIR:-target}"
    local candidate=''

    cargo test --lib --features golden-tests --no-run

    while IFS= read -r candidate; do
        if [[ -f "$candidate" && -x "$candidate" ]]; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done < <(ls -t "$target_dir"/debug/deps/scancode_rust-* 2>/dev/null)

    echo "Could not determine parser golden test binary path" >&2
    return 1
}

run_parser_golden() {
    local modules=()
    local harness_args=()
    local parsing_modules=true
    local module
    local filter

    for arg in "$@"; do
        if [[ "$arg" == "--" && "$parsing_modules" == true ]]; then
            parsing_modules=false
            continue
        fi

        if [[ "$parsing_modules" == true ]]; then
            modules+=("$arg")
        else
            harness_args+=("$arg")
        fi
    done

    local test_binary
    test_binary="$(resolve_lib_test_binary)"

    if (( ${#modules[@]} == 0 )); then
        printf '==> %q %q' "$test_binary" '_golden_test::'
        for arg in "${harness_args[@]}"; do
            printf ' %q' "$arg"
        done
        printf '\n'
        "$test_binary" '_golden_test::' "${harness_args[@]}"
        return
    fi

    for module in "${modules[@]}"; do
        if [[ "$module" == *.rs ]]; then
            module="${module##*/}"
            module="${module%.rs}"
        fi

        if [[ "$module" == *"::"* ]]; then
            if [[ "$module" == parsers::* ]]; then
                filter="$module"
            else
                filter="parsers::${module}"
            fi
        else
            if [[ "$module" != *_golden_test ]]; then
                module="${module}_golden_test"
            fi

            filter="parsers::${module}::"
        fi

        printf '==> %q %q' "$test_binary" "$filter"
        for arg in "${harness_args[@]}"; do
            printf ' %q' "$arg"
        done
        printf '\n'
        "$test_binary" "$filter" "${harness_args[@]}"
    done
}

mode="${1:-all}"
if (( $# > 0 )); then
    shift
fi

case "$mode" in
all)
    run_cargo test --lib "$@"
    run_cargo test --tests
    run_cargo test --doc
    ;;
full)
    run_cargo test --lib "$@"
    run_cargo test --tests
    run_cargo test --doc
    run_cargo test --lib --features golden-tests
    ;;
unit)
    run_cargo test --lib "$@"
    ;;
integration)
    if (( $# > 0 )); then
        integration_target="$1"
        integration_target="${integration_target#tests/}"
        integration_target="${integration_target%.rs}"

        if [[ -f "tests/${integration_target}.rs" ]]; then
            shift
            run_cargo test --test "$integration_target" "$@"
        else
            run_cargo test --tests "$@"
        fi
    else
        run_cargo test --tests
    fi
    ;;
doc)
    run_cargo test --doc "$@"
    ;;
golden)
    run_cargo test --lib --features golden-tests "$@"
    ;;
parser-golden)
    run_parser_golden "$@"
    ;;
cargo)
    if (( $# == 0 )); then
        echo "Usage: ./scripts/dev_test.sh cargo <cargo-subcommand> [args...]" >&2
        exit 1
    fi
    run_cargo "$@"
    ;;
-h|--help|help)
    usage
    ;;
*)
    echo "Unknown mode: $mode" >&2
    usage >&2
    exit 1
    ;;
esac
