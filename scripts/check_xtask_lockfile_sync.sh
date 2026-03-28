#!/bin/bash

set -euo pipefail

ROOT_MANIFEST="Cargo.toml"
XTASK_LOCKFILE="xtask/Cargo.lock"
WORKSPACE_LOCKFILE="Cargo.lock"

python3 - "$ROOT_MANIFEST" "$XTASK_LOCKFILE" "$WORKSPACE_LOCKFILE" <<'PY'
import pathlib
import re
import sys

root_manifest = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
xtask_lockfile_path = pathlib.Path(sys.argv[2])
workspace_lockfile_path = pathlib.Path(sys.argv[3])

if xtask_lockfile_path.exists():
    xtask_lockfile = xtask_lockfile_path.read_text(encoding="utf-8")
    lockfile_label = str(xtask_lockfile_path)
elif workspace_lockfile_path.exists():
    xtask_lockfile = workspace_lockfile_path.read_text(encoding="utf-8")
    lockfile_label = str(workspace_lockfile_path)
else:
    raise SystemExit(
        "Could not find xtask/Cargo.lock or workspace Cargo.lock for sync check"
    )

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
    raise SystemExit(f"Could not determine provenant-cli version from {lockfile_label}")

if root_version != xtask_version:
    raise SystemExit(
        f"{lockfile_label} is out of sync with Cargo.toml: "
        f"root crate is {root_version}, lockfile has {xtask_version}.\n"
        "Refresh it with: cargo generate-lockfile --manifest-path xtask/Cargo.toml"
    )
PY
