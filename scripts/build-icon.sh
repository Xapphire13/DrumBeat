#!/bin/sh
# Renders assets/icon.svg to the PNG that `dx bundle` turns into the macOS .icns.
# Needs librsvg (brew install librsvg).
set -eu
cd "$(dirname "$0")/.."
rsvg-convert -w 1024 -h 1024 assets/icon.svg -o assets/icon.png
