#!/bin/bash

# Release script for cargo-release
# Usage: ./release.sh <patch|minor|major> [--execute]

set -e

if [ -z "$1" ]; then
    echo "Usage: ./release.sh <patch|minor|major> [--execute]"
    echo "  --execute: Actually perform the release (default is dry-run)"
    exit 1
fi

RELEASE_TYPE=$1
EXECUTE_FLAG=""

if [ "$2" = "--execute" ]; then
    EXECUTE_FLAG="--execute"
    echo "⚠️  This will perform an actual release!"
else
    echo "ℹ️  Dry-run mode (use --execute to perform actual release)"
fi

echo "📦 Preparing for $RELEASE_TYPE release..."

# Run cargo-release
echo "🚀 Running cargo-release $RELEASE_TYPE..."
if [ -n "$EXECUTE_FLAG" ]; then
    cargo release $RELEASE_TYPE --execute
else
    cargo release $RELEASE_TYPE
fi

RELEASE_EXIT_CODE=$?

if [ $RELEASE_EXIT_CODE -eq 0 ]; then
    echo "✅ Release completed successfully!"
else
    echo "❌ Release failed with exit code $RELEASE_EXIT_CODE"
    exit $RELEASE_EXIT_CODE
fi
