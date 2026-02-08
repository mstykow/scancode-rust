#!/usr/bin/env bash
#
# Convenience wrapper for the generate-test-expected binary.
# Generates .expected.json files for golden tests.
#
# Usage: ./scripts/generate_test_expected.sh <parser_type> <input_file> <output_file>
#
# Example:
#   ./scripts/generate_test_expected.sh deb \
#     testdata/debian/deb/adduser.deb \
#     testdata/debian/deb/adduser.deb.expected.json

set -euo pipefail

cd "$(dirname "$0")/.."

exec cargo run --quiet --bin generate-test-expected -- "$@"
