#!/usr/bin/env bash
# .deb z hotového programu: build-deb.sh <flux-native> <version> <icon.png> <výstup.deb>
set -euo pipefail
bin=$1 ver=$2 icon=$3 out=$4
root=$(mktemp -d)
install -Dm755 "$bin" "$root/usr/bin/flux"
install -Dm644 "$icon" "$root/usr/share/icons/hicolor/512x512/apps/flux.png"
mkdir -p "$root/usr/share/applications" "$root/DEBIAN"
cat >"$root/usr/share/applications/flux.desktop" <<DESK
[Desktop Entry]
Type=Application
Name=Flux
GenericName=Code Editor
Comment=A small, good-looking code editor
Exec=flux %F
Icon=flux
Terminal=false
StartupWMClass=flux
Categories=Development;TextEditor;IDE;
MimeType=text/plain;text/x-python;text/html;text/css;application/javascript;application/json;
DESK
cat >"$root/DEBIAN/control" <<CTL
Package: flux-editor
Version: $ver
Architecture: amd64
Maintainer: pantr1x <https://github.com/pantr1x/Flux>
Depends: libc6, libxkbcommon0, libgl1, libx11-6, libxcb1, libegl1
Recommends: libwebkit2gtk-4.1-0
Section: devel
Priority: optional
Homepage: https://pantr1x.github.io/Flux/
Description: Flux – a small, fast code editor
 Run code with one click, see websites live while you type, Claude AI built in.
CTL
dpkg-deb --root-owner-group --build "$root" "$out"
