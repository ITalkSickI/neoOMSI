#!/bin/sh
# Render the application icons from assets/icons/app/*.svg: neoomsi.icns (macOS),
# neoomsi.ico (Windows) and the PNG window icons. Needs resvg (cargo install resvg),
# iconutil (macOS) and Python 3 with Pillow.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/../assets/icons/app" && pwd)"
tmp=$(mktemp -d)
mkdir "$tmp/icon.iconset"
for n in 16 32 128 256 512; do
  resvg -w "$n" neoomsi-macos.svg "$tmp/icon.iconset/icon_${n}x${n}.png"
  resvg -w $((n * 2)) neoomsi-macos.svg "$tmp/icon.iconset/icon_${n}x${n}@2x.png"
done
iconutil -c icns "$tmp/icon.iconset" -o neoomsi.icns
for n in 16 24 32 48 64 128 256; do resvg -w "$n" neoomsi.svg "$tmp/w$n.png"; done
cp "$tmp/w256.png" neoomsi-256.png
resvg -w 512 neoomsi.svg neoomsi-512.png
python3 - "$tmp" <<'PY'
import sys
from PIL import Image
t = sys.argv[1]
ims = [Image.open(f"{t}/w{n}.png") for n in (256, 128, 64, 48, 32, 24, 16)]
ims[0].save("neoomsi.ico", sizes=[i.size for i in ims], append_images=ims[1:])
PY
rm -rf "$tmp"
echo "icons written to assets/icons/app"
