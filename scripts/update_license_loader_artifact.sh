#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")/.."

exec cargo run --quiet --manifest-path xtask/Cargo.toml --bin generate-license-loader-artifact -- "$@"
