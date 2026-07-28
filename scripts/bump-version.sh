#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd $SCRIPT_DIR/..

OLD_VERSION="${1}"
NEW_VERSION="${2}"

echo "Bumping version: ${NEW_VERSION}"

find . -name Cargo.toml -type f -exec sed -i '' -e "s/^version.*/version = \"$NEW_VERSION\"/" {} \;

# Keep Cargo.lock in sync. `--workspace` only touches our own packages, so this
# does not pull in unrelated dependency updates.
cargo update --workspace --offline
