#!/bin/bash

set -e

echo "Initializing submodule..."
git submodule update --init

echo "Configuring sparse checkout for license data..."
LICENSES_PATH="resources/licenses"
mkdir -p "$LICENSES_PATH"
cd "$LICENSES_PATH"
git sparse-checkout init --cone
git sparse-checkout set json/details
git pull --depth=1

echo "Submodule setup complete."
