#!/bin/sh
# Print the current project version from the VERSION file.
# See docs/RELEASING.md.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
tr -d ' \r\n' < VERSION
