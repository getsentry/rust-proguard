#!/usr/bin/env bash
set -euo pipefail

TRACKED_COMMIT="e212426be84283876b6bb832630c0de25a2a7bc5"

# tail -n +2 to remove the magic anti-XSSI prefix from the Gitiles JSON response
LATEST_COMMIT=$(curl -sf \
  'https://r8.googlesource.com/r8/+log/refs/heads/main/doc/retrace.md?format=JSON' \
  | tail -n +2 \
  | jq -r '.log[0].commit')

if [ -z "$LATEST_COMMIT" ] || [ "$LATEST_COMMIT" = "null" ]; then
  echo "ERROR: Failed to fetch latest commit from Gitiles" >&2
  exit 1
fi

echo "Tracked commit: $TRACKED_COMMIT"
echo "Latest commit:  $LATEST_COMMIT"

if [ "$LATEST_COMMIT" != "$TRACKED_COMMIT" ]; then
  echo "Spec has been updated! Latest: https://r8.googlesource.com/r8/+/${LATEST_COMMIT}/doc/retrace.md"
  exit 1
fi

echo "Spec is up to date."
