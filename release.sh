#!/bin/bash

# Release script that handles sparse checkout workaround for cargo-release
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

# Update license data to latest before releasing
echo "📥 Updating SPDX license data to latest version..."
if [ ! -e "resources/licenses/.git" ]; then
    echo "⚠️  Submodule not initialized. Run ./setup.sh first."
    exit 1
fi

cd resources/licenses
CURRENT_COMMIT=$(git rev-parse HEAD)
git fetch origin main --depth=1
# Suppress "detached HEAD" warning - this is expected for submodules
git -c advice.detachedHead=false checkout origin/main
NEW_COMMIT=$(git rev-parse HEAD)
cd ../..

if [ "$CURRENT_COMMIT" != "$NEW_COMMIT" ]; then
    echo "✅ License data updated: $CURRENT_COMMIT → $NEW_COMMIT"
    if [ -n "$EXECUTE_FLAG" ]; then
        git add resources/licenses
        git commit -m "chore: update SPDX license data to latest"
        echo "✅ Committed license data update"
    else
        echo "ℹ️  License data would be updated (dry-run mode)"
        git restore resources/licenses
    fi
else
    echo "✅ License data already up to date"
fi

# Temporarily disable sparse checkout to avoid cargo-release false positive
echo "🔧 Temporarily disabling sparse checkout..."
cd resources/licenses
git sparse-checkout disable
cd ../..

# Run cargo-release
echo "🚀 Running cargo-release $RELEASE_TYPE..."
if [ -n "$EXECUTE_FLAG" ]; then
    cargo release $RELEASE_TYPE --execute
else
    cargo release $RELEASE_TYPE
fi

RELEASE_EXIT_CODE=$?

# Re-enable sparse checkout
echo "🔧 Re-enabling sparse checkout..."
cd resources/licenses
git sparse-checkout init --cone
git sparse-checkout set json/details
cd ../..

if [ $RELEASE_EXIT_CODE -eq 0 ]; then
    echo "✅ Release completed successfully!"
else
    echo "❌ Release failed with exit code $RELEASE_EXIT_CODE"
    exit $RELEASE_EXIT_CODE
fi
