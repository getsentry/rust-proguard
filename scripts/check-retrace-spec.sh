#!/usr/bin/env bash
set -euo pipefail

# Blob hash of doc/retrace.md at the last known state.
# To update: run this script's fetch logic manually and replace the hash.
TRACKED_BLOB="ae22ff183a460d25fc0cecdd2d34f5b32ae216d9"

WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

git init -q "$WORK_DIR"
git -C "$WORK_DIR" remote add origin https://r8.googlesource.com/r8
git -C "$WORK_DIR" fetch --quiet --depth=1 --filter=blob:none origin refs/heads/main

LATEST_BLOB=$(git -C "$WORK_DIR" ls-tree FETCH_HEAD -- doc/retrace.md | awk '{print $3}')

if [ -z "$LATEST_BLOB" ]; then
  echo "ERROR: Failed to read doc/retrace.md blob from r8 repo" >&2
  exit 1
fi

echo "Tracked blob: $TRACKED_BLOB"
echo "Latest blob:  $LATEST_BLOB"

if [ "$LATEST_BLOB" != "$TRACKED_BLOB" ]; then
  echo "Spec has been updated! View at: https://r8.googlesource.com/r8/+/refs/heads/main/doc/retrace.md"
  exit 1
fi

echo "Spec is up to date."
