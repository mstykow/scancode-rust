#!/usr/bin/env bash
#
# Standardized benchmark script for Provenant performance evaluation
# Usage: ./scripts/benchmark.sh
#
# Requirements:
#   - git (for cloning test repository)
#   - cargo (for building provenant)
#   - /usr/bin/time (for memory measurement)
#
# This script:
#   1. Clones a fixed test repository to /tmp
#   2. Builds provenant in release mode
#   3. Runs provenant and collects performance metrics
#

set -euo pipefail

REPO_URL="https://github.com/abraemer/opossum-file.rs.git"
REPO_COMMIT="dc0d7680c73333443ccc3df9657843210440a2ac"
REPO_NAME="opossum-file.rs"
TMP_DIR="/tmp/provenant-benchmark"
OUTPUT_DIR="${TMP_DIR}/results"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=========================================="
echo "Provenant Benchmark Script"
echo "=========================================="
echo ""
echo "Configuration:"
echo "  Repository: ${REPO_URL}"
echo "  Commit:     ${REPO_COMMIT}"
echo "  Target:     ${TMP_DIR}/${REPO_NAME}"
echo ""

echo "[1/4] Cleaning up previous benchmark directory..."
rm -rf "${TMP_DIR}"
mkdir -p "${OUTPUT_DIR}"

echo "[2/4] Cloning test repository..."
git clone "${REPO_URL}" "${TMP_DIR}/${REPO_NAME}" 2>&1 | sed 's/^/  /'
cd "${TMP_DIR}/${REPO_NAME}"
git checkout "${REPO_COMMIT}" 2>&1 | sed 's/^/  /'
git log -1 --oneline
echo ""

echo "[3/4] Building provenant (release mode)..."
cd "${PROJECT_ROOT}"
cargo build --release 2>&1 | grep -E '(Compiling|Finished|error)' | sed 's/^/  /'
echo ""

PROVENANT_BIN="${PROJECT_ROOT}/target/release/provenant"
if [[ ! -x "${PROVENANT_BIN}" ]]; then
    echo "ERROR: provenant binary not found at ${PROVENANT_BIN}"
    exit 1
fi

echo "[4/4] Running provenant benchmark..."
echo ""

OUTPUT_FILE="${OUTPUT_DIR}/scan-output.json"

cd "${TMP_DIR}/${REPO_NAME}"

START_TIME=$(date +%s.%N)

/usr/bin/time -v "${PROVENANT_BIN}" \
    --json "${OUTPUT_FILE}" \
    --package \
    --license \
    --copyright \
    --email \
    --url \
    --exclude "*.git*" \
    --exclude "target/*" \
    . \
    2>&1 | tee "${OUTPUT_DIR}/provenant-stdout.txt" | sed 's/^/  /'

END_TIME=$(date +%s.%N)

ELAPSED=$(echo "${END_TIME} - ${START_TIME}" | bc)

echo ""
echo "=========================================="
echo "Benchmark Results"
echo "=========================================="
echo ""

echo "Timing:"
echo "  Wall clock time: ${ELAPSED} seconds"
echo ""

echo "Scan Statistics:"
if [[ -f "${OUTPUT_FILE}" ]]; then
    FILE_COUNT=$(python3 -c "import json; d=json.load(open('${OUTPUT_FILE}')); print(len(d.get('files', [])))" 2>/dev/null || echo "N/A")
    PACKAGE_COUNT=$(python3 -c "import json; d=json.load(open('${OUTPUT_FILE}')); print(len(d.get('packages', [])))" 2>/dev/null || echo "N/A")
    echo "  Files scanned:     ${FILE_COUNT}"
    echo "  Packages detected: ${PACKAGE_COUNT}"
    
    echo ""
    echo "Provenant Timings (from scan output):"
    grep -E "^\s+(discovery|license_detection_engine_creation|scan|assembly|output|total):" "${OUTPUT_DIR}/provenant-stdout.txt" | sed 's/^/  /'
else
    echo "  (output file not found)"
fi
echo ""

echo "Memory Usage (from /usr/bin/time):"
if grep -q "Maximum resident set size" "${OUTPUT_DIR}/provenant-stdout.txt"; then
    MAX_MEM=$(grep "Maximum resident set size" "${OUTPUT_DIR}/provenant-stdout.txt" | awk '{print $NF}')
    echo "  Peak memory: $((MAX_MEM / 1024)) MB (${MAX_MEM} KB)"
else
    echo "  (not available - /usr/bin/time may not be installed)"
fi
echo ""

echo "Output Files:"
echo "  ${OUTPUT_FILE}"
echo "  ${OUTPUT_DIR}/provenant-stdout.txt"
echo ""

echo "To clean up:"
echo "  rm -rf ${TMP_DIR}"
