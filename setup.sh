#!/bin/bash
#
# Setup script for Provenant development
#
# This script initializes git submodules and installs pre-commit hooks.
#
# The license detection index is already embedded in the binary at:
#   resources/license_detection/license_index_loader.msgpack.zst
#
# License rules and licenses are in the reference/scancode-toolkit submodule.
#
# You only need to run this script if you:
# - Are building from source for the first time
# - Want to install pre-commit hooks
# - Want to update to the latest license rules/licenses
#
# Run this script:
# - Before building from source for the first time

set -e

echo "Initializing submodules..."
git submodule update --init --filter=blob:none

echo ""
echo "Installing pre-commit hooks..."
if command -v pre-commit >/dev/null 2>&1; then
    pre-commit install
    echo "✅ Pre-commit hooks installed"
else
    echo "⚠️  pre-commit is not installed. Install it, then run:"
    echo "   pre-commit install"
fi

echo ""
echo "Setup complete."
echo ""
echo "To build: cargo build --release"
echo ""
