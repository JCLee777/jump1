#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: macOS packaging must run on macOS" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="${APP_NAME:-Jump1}"
BUNDLE_IDENTIFIER="${BUNDLE_IDENTIFIER:-io.github.jclee777.jump1}"
SIGN_IDENTITY="${CODESIGN_IDENTITY:--}"
TARGET_TRIPLE="${CARGO_BUILD_TARGET:-}"
VERSION="$(awk -F ' *= *' '/^version *=/ { gsub(/"/, "", $2); print $2; exit }' "$ROOT_DIR/Cargo.toml")"

if [[ -z "$VERSION" ]]; then
  echo "error: could not read package version from Cargo.toml" >&2
  exit 1
fi

if [[ -n "$TARGET_TRIPLE" ]]; then
  BUILD_ARGS=(--release --locked --target "$TARGET_TRIPLE")
  BINARY_PATH="$ROOT_DIR/target/$TARGET_TRIPLE/release/jump1"
  ARCH_NAME="${TARGET_TRIPLE%%-*}"
else
  BUILD_ARGS=(--release --locked)
  BINARY_PATH="$ROOT_DIR/target/release/jump1"
  ARCH_NAME="$(uname -m)"
  if [[ "$ARCH_NAME" == "arm64" ]]; then
    ARCH_NAME="aarch64"
  fi
fi

DIST_DIR="$ROOT_DIR/dist"
WORK_DIR="$DIST_DIR/.package-macos"
APP_DIR="$WORK_DIR/$APP_NAME.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
DMG_ROOT="$WORK_DIR/dmg"
DMG_PATH="$DIST_DIR/Jump1-$VERSION-$ARCH_NAME.dmg"

cleanup() {
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

echo "Building Jump1 $VERSION for $ARCH_NAME..."
(cd "$ROOT_DIR" && cargo build "${BUILD_ARGS[@]}")

case "$ARCH_NAME" in
  aarch64) BINARY_ARCH="arm64" ;;
  x86_64) BINARY_ARCH="x86_64" ;;
  *)
    echo "error: unsupported macOS architecture: $ARCH_NAME" >&2
    exit 1
    ;;
esac

if ! lipo -verify_arch "$BINARY_ARCH" "$BINARY_PATH" >/dev/null 2>&1; then
  echo "error: $BINARY_PATH is not a $BINARY_ARCH executable" >&2
  exit 1
fi

rm -rf "$WORK_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR" "$DMG_ROOT"
install -m 755 "$BINARY_PATH" "$MACOS_DIR/jump1"

cat > "$CONTENTS_DIR/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>zh_CN</string>
  <key>CFBundleDisplayName</key>
  <string>$APP_NAME</string>
  <key>CFBundleExecutable</key>
  <string>jump1</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundleIdentifier</key>
  <string>$BUNDLE_IDENTIFIER</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleVersion</key>
  <string>$VERSION</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.utilities</string>
  <key>LSUIElement</key>
  <true/>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSHumanReadableCopyright</key>
  <string>Copyright © 2026 Jump1 contributors</string>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
</dict>
</plist>
PLIST

ICON_SOURCE="$ROOT_DIR/assets/app-icon.svg"
ICON_WORK="$WORK_DIR/icon"
ICONSET="$ICON_WORK/AppIcon.iconset"
mkdir -p "$ICON_WORK" "$ICONSET"

qlmanage -t -s 1024 -o "$ICON_WORK" "$ICON_SOURCE" >/dev/null 2>&1
ICON_PNG="$ICON_WORK/$(basename "$ICON_SOURCE").png"
if [[ ! -f "$ICON_PNG" ]]; then
  echo "error: failed to render $ICON_SOURCE" >&2
  exit 1
fi

for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$ICON_PNG" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  retina_size=$((size * 2))
  sips -z "$retina_size" "$retina_size" "$ICON_PNG" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$RESOURCES_DIR/AppIcon.icns"

xattr -cr "$APP_DIR"
if [[ "$SIGN_IDENTITY" == "-" ]]; then
  # An ad-hoc signature normally uses the binary's cdhash as its designated
  # requirement. That changes on every build and invalidates macOS privacy
  # grants. Give local builds a stable requirement based on the bundle ID.
  codesign \
    --force \
    --deep \
    --sign - \
    --identifier "$BUNDLE_IDENTIFIER" \
    --requirements "=designated => identifier \"$BUNDLE_IDENTIFIER\"" \
    "$APP_DIR"
else
  codesign --force --deep --sign "$SIGN_IDENTITY" "$APP_DIR"
fi
codesign --verify --deep --strict "$APP_DIR"

ditto "$APP_DIR" "$DMG_ROOT/$APP_NAME.app"
ln -s /Applications "$DMG_ROOT/Applications"
rm -f "$DMG_PATH"
hdiutil create \
  -volname "$APP_NAME" \
  -srcfolder "$DMG_ROOT" \
  -format UDZO \
  -ov \
  "$DMG_PATH" >/dev/null

echo "Created: $DMG_PATH"
