#!/bin/bash
# Regenerate the embedded license loader artifact.
#
# This script should be run when the license rules or licenses data changes.
# The generated artifact is committed to the repository so that builds don't
# require the reference submodule.

set -e

echo "Regenerating license loader artifact..."
cargo run --bin generate-license-loader-artifact

echo ""
echo "Artifact generated at: resources/license_detection/license_index_loader.msgpack.zst"
echo "Remember to commit this file if the rules/licenses data has changed."
