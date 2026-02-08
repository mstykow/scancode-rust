#!/usr/bin/env bash
# Extract all URLs from documentation and docstrings for validation

set -euo pipefail

PROJECT_ROOT="${1:-.}"
cd "$PROJECT_ROOT"

echo "=== Extracting URLs from Documentation ==="
echo ""

# Extract from markdown docs (excluding archived, testdata, and fixtures)
echo "## Markdown Documentation URLs"
echo ""
find . \( -name "*.md" \) \
  ! -path "*/archived/*" \
  ! -path "*/testdata/*" \
  ! -path "*/tests/*" \
  ! -path "*/target/*" \
  ! -path "*/.git/*" \
  -type f \
  -exec grep -HnEo 'https?://[^)[:space:]">]+' {} \; 2>/dev/null | \
  sed 's/:/ | /' | \
  sort -t'|' -k1,1 -k2,2n | \
  awk -F'|' '{printf "%s:%s | %s\n", $1, $2, $3}'

echo ""
echo "## Rust Docstring URLs"
echo ""

# Extract from Rust docstrings (//! and ///) - excluding tests
find src/ \( -name "*.rs" \) \
  ! -name "*_test.rs" \
  ! -path "*/tests/*" \
  -type f \
  -exec grep -HnE '^[[:space:]]*(///|//!)' {} \; 2>/dev/null | \
  grep -Eo 'https?://[^)[:space:]">]+|<https?://[^>]+>' | \
  grep -v '^[[:space:]]*$'

echo ""
echo "=== Summary ==="
echo ""

MD_COUNT=$(find . \( -name "*.md" \) \
  ! -path "*/archived/*" \
  ! -path "*/testdata/*" \
  ! -path "*/tests/*" \
  ! -path "*/target/*" \
  ! -path "*/.git/*" \
  -type f \
  -exec grep -HnEo 'https?://[^)[:space:]">]+' {} \; 2>/dev/null | wc -l | tr -d ' ')

RS_COUNT=$(find src/ \( -name "*.rs" \) \
  ! -name "*_test.rs" \
  ! -path "*/tests/*" \
  -type f \
  -exec grep -HnE '^[[:space:]]*(///|//!).*https?://' {} \; 2>/dev/null | wc -l | tr -d ' ')

echo "Markdown URLs: $MD_COUNT"
echo "Docstring URLs: $RS_COUNT"
echo "Total: $((MD_COUNT + RS_COUNT))"
