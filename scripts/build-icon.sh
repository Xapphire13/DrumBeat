#!/bin/sh
# Renders assets/icon.svg to the PNGs the app bundles use:
# - assets/icon.png, which `dx bundle` turns into the macOS .icns
# - assets/icon-ios.png, a full-bleed square for iOS, which masks the corners
#   itself (built into the app by scripts/build-ios.sh)
# Needs librsvg (brew install librsvg).
set -eu
cd "$(dirname "$0")/.."
rsvg-convert -w 1024 -h 1024 assets/icon.svg -o assets/icon.png

# For iOS: drop the tile's shadow, stretch the tile to the edges and scale the
# drum up to fill the extra room
perl -pe '
  $_ = "" if /opacity="\.28"/;
  s{<rect [^>]*fill="url\(#a-tile\)"/>}{<rect width="1024" height="1024" fill="url(#a-tile)"/>\n<g transform="translate(512 512) scale(1.15) translate(-512 -512)">};
  s{</svg>}{</g>\n</svg>};
' assets/icon.svg | rsvg-convert -w 1024 -h 1024 -b '#000' -o assets/icon-ios.png
