#!/usr/bin/env bash
# Compiles .changes/*.md fragments into CHANGELOG.md for a release and removes processed fragments.
# Usage: ./scripts/compile-changelog.sh [version] [date]
# Example: ./scripts/compile-changelog.sh 0.2.0 2026-10-03
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

VERSION="${1:-$(tr -d ' \r\n' < VERSION)}"
DATE="${2:-$(date +%Y-%m-%d)}"

# Clean up version string if prefixed with 'v'
VERSION="${VERSION#v}"

# Collect all fragment files except README.md
shopt -s nullglob
FRAGMENTS=(.changes/*.md)
shopt -u nullglob

VALID_FRAGMENTS=()
for f in "${FRAGMENTS[@]}"; do
  [ "$(basename "$f")" = "README.md" ] && continue
  [ -f "$f" ] && VALID_FRAGMENTS+=("$f")
done

if [ ${#VALID_FRAGMENTS[@]} -eq 0 ]; then
  echo "No changelog fragments found in .changes/. Nothing to compile."
  exit 0
fi

echo "Compiling ${#VALID_FRAGMENTS[@]} fragment(s) for version $VERSION ($DATE)..."

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

CATEGORIES=("breaking" "feature" "parity" "fix" "performance" "internal" "other")

declare -A CATEGORY_TITLES=(
  ["breaking"]="Breaking changes"
  ["feature"]="Features"
  ["parity"]="Parity & OMSI 2 compatibility"
  ["fix"]="Bug fixes"
  ["performance"]="Performance"
  ["internal"]="Internal & tooling"
  ["other"]="Other changes"
)

for cat in "${CATEGORIES[@]}"; do
  touch "$TMP_DIR/$cat.txt"
done

for f in "${VALID_FRAGMENTS[@]}"; do
  base="$(basename "$f")"
  pr=""
  cat="other"

  if [[ "$base" =~ ^([0-9]+)\.([a-z0-9_-]+)\.md$ ]]; then
    pr="${BASH_REMATCH[1]}"
    matched_cat="${BASH_REMATCH[2]}"
    if [[ " ${CATEGORIES[*]} " =~ " ${matched_cat} " ]]; then
      cat="$matched_cat"
    fi
  fi

  content="$(sed '/^[[:space:]]*$/d' "$f")"
  if [ -z "$content" ]; then
    continue
  fi

  formatted=""
  first_line=true
  while IFS= read -r line; do
    if [ "$first_line" = true ]; then
      first_line=false
      if [[ "$line" =~ ^[[:space:]]*[-*][[:space:]] ]]; then
        formatted="$line"
      else
        formatted="- $line"
      fi
    else
      formatted+=$'\n'"  $line"
    fi
  done <<< "$content"

  if [ -n "$pr" ] && ! [[ "$formatted" =~ \#$pr ]]; then
    formatted="$formatted [#$pr](https://github.com/neoOMSI/neoOMSI/pull/$pr)"
  fi

  echo "$formatted" >> "$TMP_DIR/$cat.txt"
done

RELEASE_BLOCK="## $VERSION - $DATE"$'\n'

has_entries=false
for cat in "${CATEGORIES[@]}"; do
  cat_file="$TMP_DIR/$cat.txt"
  if [ -s "$cat_file" ]; then
    has_entries=true
    title="${CATEGORY_TITLES[$cat]}"
    RELEASE_BLOCK+=$'\n'"### $title"$'\n'
    RELEASE_BLOCK+="$(cat "$cat_file")"$'\n'
  fi
done

if [ "$has_entries" = false ]; then
  echo "Fragments contained no readable entries. Nothing compiled."
  exit 0
fi

CHANGELOG="CHANGELOG.md"
if [ ! -f "$CHANGELOG" ]; then
  echo -e "# Changelog\n\n$RELEASE_BLOCK" > "$CHANGELOG"
else
  FIRST_RELEASE_LINE=$(grep -n '^## ' "$CHANGELOG" | head -n 1 | cut -d: -f1 || true)
  if [ -n "$FIRST_RELEASE_LINE" ]; then
    head -n $((FIRST_RELEASE_LINE - 1)) "$CHANGELOG" > "$TMP_DIR/new_changelog.md"
    echo "$RELEASE_BLOCK" >> "$TMP_DIR/new_changelog.md"
    tail -n "+$FIRST_RELEASE_LINE" "$CHANGELOG" >> "$TMP_DIR/new_changelog.md"
    mv "$TMP_DIR/new_changelog.md" "$CHANGELOG"
  else
    echo -e "\n$RELEASE_BLOCK" >> "$CHANGELOG"
  fi
fi

for f in "${VALID_FRAGMENTS[@]}"; do
  rm -f "$f"
done

echo "Successfully compiled changelog for $VERSION into $CHANGELOG and removed ${#VALID_FRAGMENTS[@]} fragment(s)."
