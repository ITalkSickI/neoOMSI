#!/usr/bin/env bash
# Compile all pending .changes/*.md fragments into CHANGELOG.md for a release
# and remove the processed fragments.
# Usage: bash scripts/compile-changelog.sh [version] [date]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

VERSION="${1:-$(tr -d ' \r\n' < VERSION)}"
DATE="${2:-$(date +%Y-%m-%d)}"
VERSION="${VERSION#v}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

bash "$REPO_ROOT/scripts/render-changelog-fragments.sh" "$TMP_DIR/changes.md"

if [ ! -s "$TMP_DIR/changes.md" ]; then
  echo "No changelog fragments found in .changes/. Nothing to compile."
  exit 0
fi

shopt -s nullglob
FRAGMENTS=(.changes/*.md)
shopt -u nullglob

VALID_FRAGMENTS=()
for f in "${FRAGMENTS[@]}"; do
  [ "$(basename "$f")" = "README.md" ] && continue
  [ -s "$f" ] && VALID_FRAGMENTS+=("$f")
done

echo "Compiling ${#VALID_FRAGMENTS[@]} fragment(s) for version $VERSION ($DATE)..."

{
  printf '## %s - %s\n\n' "$VERSION" "$DATE"
  cat "$TMP_DIR/changes.md"
  printf '\n'
} > "$TMP_DIR/release.md"

CHANGELOG="CHANGELOG.md"
if [ ! -f "$CHANGELOG" ]; then
  {
    printf '# Changelog\n\n'
    cat "$TMP_DIR/release.md"
  } > "$CHANGELOG"
else
  FIRST_RELEASE_LINE="$(grep -n '^## ' "$CHANGELOG" | head -n 1 | cut -d: -f1 || true)"

  if [ -n "$FIRST_RELEASE_LINE" ]; then
    head -n $((FIRST_RELEASE_LINE - 1)) "$CHANGELOG" > "$TMP_DIR/new_changelog.md"
    cat "$TMP_DIR/release.md" >> "$TMP_DIR/new_changelog.md"
    tail -n "+$FIRST_RELEASE_LINE" "$CHANGELOG" >> "$TMP_DIR/new_changelog.md"
  else
    cat "$CHANGELOG" > "$TMP_DIR/new_changelog.md"
    printf '\n' >> "$TMP_DIR/new_changelog.md"
    cat "$TMP_DIR/release.md" >> "$TMP_DIR/new_changelog.md"
  fi

  mv "$TMP_DIR/new_changelog.md" "$CHANGELOG"
fi

for f in "${VALID_FRAGMENTS[@]}"; do
  rm -f "$f"
done

echo "Successfully compiled changelog for $VERSION into $CHANGELOG and removed ${#VALID_FRAGMENTS[@]} fragment(s)."
