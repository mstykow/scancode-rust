#!/bin/bash

set -euo pipefail

ROOT_MANIFEST="Cargo.toml"
XTASK_LOCKFILE="xtask/Cargo.lock"

python3 - "$ROOT_MANIFEST" "$XTASK_LOCKFILE" <<'PY'
import pathlib
import re
import sys

root_manifest = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
xtask_lockfile = pathlib.Path(sys.argv[2]).read_text(encoding="utf-8")

root_version_match = re.search(r'^version = "([^"]+)"$', root_manifest, re.MULTILINE)
if root_version_match is None:
    raise SystemExit("Could not determine root crate version from Cargo.toml")

root_version = root_version_match.group(1)

xtask_version = None
for block in xtask_lockfile.split("[[package]]"):
    if 'name = "provenant-cli"' not in block:
        continue
    version_match = re.search(r'^version = "([^"]+)"$', block, re.MULTILINE)
    if version_match is not None:
        xtask_version = version_match.group(1)
        break

if xtask_version is None:
    raise SystemExit("Could not determine provenant-cli version from xtask/Cargo.lock")

if root_version != xtask_version:
    raise SystemExit(
        "xtask/Cargo.lock is out of sync with Cargo.toml: "
        f"root crate is {root_version}, xtask lockfile has {xtask_version}.\n"
        "Refresh it with: cargo generate-lockfile --manifest-path xtask/Cargo.toml\n"
        "Then stage the updated xtask/Cargo.lock."
    )
PY
