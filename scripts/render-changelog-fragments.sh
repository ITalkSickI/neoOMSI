#!/usr/bin/env bash
# Render changelog fragments as categorized Markdown without mutating them.
# Usage: bash scripts/render-changelog-fragments.sh OUTPUT [FRAGMENT...]
# With no FRAGMENT arguments, all pending fragments are rendered.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

OUTPUT="${1:?usage: render-changelog-fragments.sh OUTPUT [FRAGMENT...]}"
shift

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

CATEGORIES=("breaking" "parity" "feature" "fix" "performance" "docs" "internal")

declare -A CATEGORY_TITLES=(
  ["breaking"]="Breaking changes"
  ["parity"]="OMSI 2 parity"
  ["feature"]="Features"
  ["fix"]="Fixes"
  ["performance"]="Performance"
  ["docs"]="Website & documentation"
  ["internal"]="Internal & tooling"
)

for cat in "${CATEGORIES[@]}"; do
  : > "$TMP_DIR/$cat.list"
done

if [ "$#" -gt 0 ]; then
  FRAGMENTS=("$@")
else
  shopt -s nullglob
  FRAGMENTS=(.changes/*.md)
  shopt -u nullglob
fi

for f in "${FRAGMENTS[@]}"; do
  [ -f "$f" ] || continue
  [ "$(basename "$f")" = "README.md" ] && continue
  [ -s "$f" ] || continue

  base="$(basename "$f")"
  if ! [[ "$base" =~ ^([0-9]+)\.(breaking|parity|feature|fix|performance|docs|internal)\.md$ ]]; then
    echo "Invalid changelog fragment name: $base" >&2
    exit 1
  fi

  pr="${BASH_REMATCH[1]}"
  cat="${BASH_REMATCH[2]}"
  printf '%s\t%s\n' "$pr" "$f" >> "$TMP_DIR/$cat.list"
done

server="${GITHUB_SERVER_URL:-https://github.com}"
repo="${GITHUB_REPOSITORY:-neoOMSI/neoOMSI}"

: > "$OUTPUT"

for cat in "${CATEGORIES[@]}"; do
  list="$TMP_DIR/$cat.list"
  [ -s "$list" ] || continue

  printf '### %s\n\n' "${CATEGORY_TITLES[$cat]}" >> "$OUTPUT"

  while IFS=$'\t' read -r pr f; do
    # Fragments are intentionally one concise paragraph. Collapse wrapping/newlines
    # without changing punctuation or Markdown inline formatting.
    content="$(
      sed 's/\r$//' "$f" |
        awk '
          NF {
            gsub(/^[[:space:]]+|[[:space:]]+$/, "")
            if (out != "") out = out " "
            out = out $0
          }
          END { print out }
        '
    )"

    [ -n "$content" ] || continue
    printf -- '- %s ([#%s](%s/%s/pull/%s))\n' "$content" "$pr" "$server" "$repo" "$pr" >> "$OUTPUT"
  done < "$list"

  printf '\n' >> "$OUTPUT"
done

# Remove trailing blank lines.
awk '
  { lines[NR] = $0 }
  END {
    last = NR
    while (last > 0 && lines[last] ~ /^[[:space:]]*$/) last--
    for (i = 1; i <= last; i++) print lines[i]
  }
' "$OUTPUT" > "$TMP_DIR/rendered.md"

mv "$TMP_DIR/rendered.md" "$OUTPUT"
