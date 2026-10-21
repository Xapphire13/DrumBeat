#!/bin/sh
# Builds DrumBeat for iPhone and installs it on a connected device.
#
#   scripts/build-ios.sh [device]
#
# `device` is a name or UDID (defaults to the first connected iPhone/iPad).
# Set SIGNING_IDENTITY to pick a certificate (defaults to the first
# "Apple Development" one in the keychain). Needs a development provisioning
# profile for com.xapphire13.drumbeat, which Xcode creates the first time any
# project with that bundle identifier is run on the device.
#
# On top of `dx bundle`, this:
# - Marks the binary as built with the iOS 26 SDK. iOS 27 refuses to launch apps
#   built with its SDK that don't adopt the UIScene lifecycle, and the tao
#   version Dioxus 0.7 uses doesn't (tao 0.35+ does, with Dioxus 0.8).
# - Adds the app icon, which dx leaves out of iOS bundles.
set -eu
cd "$(dirname "$0")/.."

device=${1:-}
if [ -z "$device" ]; then
  json=$(mktemp)
  xcrun devicectl list devices --quiet --json-output "$json" >/dev/null
  device=$(python3 -c '
import json, sys
for d in json.load(open(sys.argv[1]))["result"]["devices"]:
    if d["connectionProperties"].get("pairingState") == "paired" and d["hardwareProperties"].get("platform") == "iOS" and d["hardwareProperties"].get("reality") == "physical":
        print(d["hardwareProperties"]["udid"]); break
' "$json")
  rm "$json"
  [ -n "$device" ] || { echo "No connected iPhone found" >&2; exit 1; }
fi

identity=${SIGNING_IDENTITY:-$(security find-identity -v -p codesigning | sed -n 's/.*"\(Apple Development: .*\)"/\1/p' | head -1)}
[ -n "$identity" ] || { echo "No Apple Development signing identity found" >&2; exit 1; }

[ -f assets/icon-ios.png ] || scripts/build-icon.sh

# A separate target dir, so this doesn't disturb a running `dx serve`
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target-ios}
dx bundle --ios --release --device "$device" --codesign --apple-team-id "$identity"
app="$CARGO_TARGET_DIR/dx/drum-beat/release/ios/DrumBeat.app"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

vtool -set-build-version ios 16.0 26.0 -replace -output "$work/drum-beat" "$app/drum-beat" 2>/dev/null
mv "$work/drum-beat" "$app/drum-beat"

iconset="$work/Assets.xcassets/AppIcon.appiconset"
mkdir -p "$iconset"
cp assets/icon-ios.png "$iconset/icon.png"
cat >"$iconset/Contents.json" <<'EOF'
{
  "images": [{ "filename": "icon.png", "idiom": "universal", "platform": "ios", "size": "1024x1024" }],
  "info": { "author": "xcode", "version": 1 }
}
EOF
xcrun actool "$work/Assets.xcassets" --compile "$app" --platform iphoneos \
  --minimum-deployment-target 16.0 --target-device iphone --target-device ipad \
  --app-icon AppIcon --output-partial-info-plist "$work/icon.plist" \
  --output-format human-readable-text --notices --warnings >/dev/null
/usr/libexec/PlistBuddy -c "Merge $work/icon.plist" "$app/Info.plist" >/dev/null

# Both changes invalidate dx's signature, so sign again with its entitlements
codesign -d --entitlements :- --xml "$app" >"$work/entitlements.plist" 2>/dev/null
codesign --force --sign "$identity" --entitlements "$work/entitlements.plist" "$app"

xcrun devicectl device install app --device "$device" "$app"
