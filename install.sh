#!/usr/bin/env bash
# Inštalácia Fluxu na Linux jedným príkazom:
#   curl -fsSL https://pantr1x.github.io/Flux/install.sh | bash
# Stiahne najnovší Flux AppImage do ~/.local/share/flux, pridá príkaz `flux` a ikonu do menu aplikácií.
# Ďalšie aktualizácie si Flux robí sám. Odinštalovanie:  curl -fsSL …/install.sh | bash -s -- --uninstall
set -euo pipefail

REPO="pantr1x/Flux"
DIR="$HOME/.local/share/flux"
BIN="$HOME/.local/bin"
APPS="$HOME/.local/share/applications"
ICONS="$HOME/.local/share/icons/hicolor/512x512/apps"

say() { printf '\033[36m==>\033[0m %s\n' "$*"; }
die() { printf '\033[31mError:\033[0m %s\n' "$*" >&2; exit 1; }

if [ "${1:-}" = "--uninstall" ]; then
  rm -rf "$DIR" "$BIN/flux" "$APPS/flux.desktop" "$ICONS/flux.png"
  command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" 2>/dev/null || true
  say "Flux removed. Your settings stay in ~/.config/Flux (delete that folder to remove them too)."
  exit 0
fi

[ "$(uname -s)" = Linux ] || die "This installer is for Linux."
[ "$(uname -m)" = x86_64 ] || die "Flux for Linux is built for x86_64 only (you have $(uname -m))."
command -v curl >/dev/null || die "curl is needed."

# AppImage potrebuje FUSE 2 (libfuse.so.2) – doinštalovať podľa distribúcie
if ! ldconfig -p 2>/dev/null | grep -q 'libfuse\.so\.2' && [ ! -e /usr/lib/libfuse.so.2 ] && [ ! -e /usr/lib64/libfuse.so.2 ]; then
  say "Installing FUSE 2 (needed by AppImage) – you may be asked for your password"
  if command -v pacman >/dev/null; then sudo pacman -S --needed --noconfirm fuse2
  elif command -v apt-get >/dev/null; then sudo apt-get install -y libfuse2t64 2>/dev/null || sudo apt-get install -y libfuse2
  elif command -v dnf >/dev/null; then sudo dnf install -y fuse-libs
  elif command -v zypper >/dev/null; then sudo zypper install -y libfuse2
  else say "Could not install FUSE 2 automatically – Flux will unpack itself on each start instead."; fi
fi

# najnovšie vydanie, ktoré má AppImage (API vracia vydania od najnovšieho)
say "Looking for the newest Flux for Linux"
url=$(curl -fsSL -H 'User-Agent: flux-install' "https://api.github.com/repos/$REPO/releases?per_page=30" |
  grep -o '"browser_download_url": *"[^"]*\.AppImage"' | head -n1 | cut -d'"' -f4) || true
[ -n "$url" ] || die "No Linux build found on https://github.com/$REPO/releases"
ver=$(basename "$url" .AppImage); ver=${ver#Flux-}

say "Downloading Flux $ver"
mkdir -p "$DIR" "$BIN" "$APPS" "$ICONS"
curl -fL --progress-bar -o "$DIR/Flux.AppImage.part" "$url"
chmod +x "$DIR/Flux.AppImage.part"
mv -f "$DIR/Flux.AppImage.part" "$DIR/Flux.AppImage"

# príkaz `flux` (bez FUSE sa AppImage rozbalí pri každom štarte)
cat >"$BIN/flux" <<EOF
#!/bin/sh
if [ ! -e /dev/fuse ] || ! ldconfig -p 2>/dev/null | grep -q 'libfuse\.so\.2'; then export APPIMAGE_EXTRACT_AND_RUN=1; fi
exec "$DIR/Flux.AppImage" "\$@"
EOF
chmod +x "$BIN/flux"

# ikona priamo z AppImage (záloha: z webu); v .desktop s plnou cestou – funguje v každom menu
tmp=$(mktemp -d)
(cd "$tmp" && "$DIR/Flux.AppImage" --appimage-extract 'usr/share/icons/hicolor/512x512/apps/flux.png' >/dev/null 2>&1) || true
if [ -s "$tmp/squashfs-root/usr/share/icons/hicolor/512x512/apps/flux.png" ]; then
  cp -f "$tmp/squashfs-root/usr/share/icons/hicolor/512x512/apps/flux.png" "$DIR/flux.png"
else
  curl -fsSL -o "$DIR/flux.png" "https://pantr1x.github.io/Flux/icon.png" || true
fi
rm -rf "$tmp"
cp -f "$DIR/flux.png" "$ICONS/flux.png" 2>/dev/null || true
cat >"$APPS/flux.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Flux
GenericName=Code Editor
Comment=A small, good-looking code editor
Exec=$BIN/flux %F
Icon=$DIR/flux.png
Terminal=false
Categories=Development;TextEditor;IDE;
MimeType=text/plain;text/x-python;text/html;text/css;application/javascript;application/json;
EOF
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

say "Flux $ver is installed. Open it from your app menu, or run: flux"
case ":$PATH:" in *":$BIN:"*) ;; *) say "Tip: add ~/.local/bin to PATH to use the 'flux' command." ;; esac
