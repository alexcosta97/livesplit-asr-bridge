#!/usr/bin/env bash
# Packages the release build for one target into dist/.
#   package.sh <target> <version>
set -euo pipefail

target=$1
version=$2
name=livesplit-asr-bridge
bin_dir="target/$target/release"
out="$(pwd)/dist"
stage=$(mktemp -d)
mkdir -p "$out"

case "$target" in
  x86_64-unknown-linux-gnu)
    mkdir -p "$stage/$name"
    cp "$bin_dir/$name" README.md LICENSE-MIT LICENSE-APACHE \
      "assets/linux/$name.desktop" "$stage/$name/"
    # The icons in the hicolor theme layout, ready to copy into
    # ~/.local/share/icons (see the README).
    for size in 16 24 32 48 64 128 256 512; do
      dir="$stage/$name/icons/hicolor/${size}x${size}/apps"
      mkdir -p "$dir"
      cp "assets/brand/icons/icon-$size.png" "$dir/$name.png"
    done
    tar -C "$stage" -czf "$out/$name-$version-x86_64-linux.tar.gz" "$name"
    ;;
  aarch64-apple-darwin | x86_64-apple-darwin)
    arch=${target%%-*}
    [[ "$arch" == aarch64 ]] && arch=arm64
    app="$stage/$name.app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    cp "$bin_dir/$name" "$app/Contents/MacOS/$name"
    chmod +x "$app/Contents/MacOS/$name"
    cp README.md LICENSE-MIT LICENSE-APACHE "$app/Contents/Resources/"
    # Bundle versions must be numeric, so candidates use the X.Y.Z part.
    short=${version%%-*}
    cat >"$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>$name</string>
  <key>CFBundleIdentifier</key><string>io.github.alexcosta97.livesplit-asr-bridge</string>
  <key>CFBundleName</key><string>$name</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleSignature</key><string>????</string>
  <key>CFBundleShortVersionString</key><string>$short</string>
  <key>CFBundleVersion</key><string>$short</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
    # Sign the bundle ad hoc. Without a bundle signature, macOS reports a
    # downloaded app as damaged instead of offering Open Anyway. codesign
    # refuses files with extended attributes, so clear them first. The verify
    # step fails the build if the bundle isn't signed properly. Developer ID
    # signing and notarisation are in the backlog (spec §16).
    xattr -cr "$app"
    codesign --force --sign - "$app"
    codesign --verify --deep --strict --verbose=2 "$app"
    ditto -c -k --sequesterRsrc --keepParent "$app" "$out/$name-$version-$arch-macos.zip"
    ;;
  x86_64-pc-windows-msvc)
    mkdir -p "$stage/$name"
    cp "$bin_dir/$name.exe" README.md LICENSE-MIT LICENSE-APACHE "$stage/$name/"
    (cd "$stage" && 7z a -tzip "$out/$name-$version-x86_64-windows.zip" "$name" >/dev/null)
    ;;
  *)
    echo "unsupported target: $target" >&2
    exit 1
    ;;
esac

ls "$out"
