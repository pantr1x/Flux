#!/usr/bin/env bash
# Inštalácia Flux Native (Rust, malý a rýchly) na Linux jedným príkazom:
#   curl -fsSL https://pantr1x.github.io/Flux/install.sh | bash
# Stiahne program z vetvy ci-native-linux do ~/.local/share/flux, pridá príkaz `flux` a ikonu do menu aplikácií.
# Ďalšie aktualizácie si Flux robí sám (Nastavenia → General → About & updates, alebo automaticky).
# Odinštalovanie:  curl -fsSL https://pantr1x.github.io/Flux/install.sh | bash -s -- --uninstall
set -euo pipefail

BASE="${FLUX_INSTALL_URL:-https://raw.githubusercontent.com/pantr1x/Flux/ci-native-linux}"
SITE="${FLUX_SITE_URL:-https://pantr1x.github.io/Flux}"
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
command -v sha256sum >/dev/null || die "sha256sum is needed."

# knižnice, ktoré okno potrebuje (X11/Wayland, OpenGL, klávesnica) – doinštalovať, ak chýbajú
need=0
have_lib() {
  { ldconfig -p 2>/dev/null || /sbin/ldconfig -p 2>/dev/null || true; } | grep -q "$1" && return 0
  for d in /usr/lib /usr/lib64 /usr/lib/x86_64-linux-gnu /lib/x86_64-linux-gnu /lib64; do [ -e "$d/$1" ] && return 0; done
  return 1
}
for lib in libxkbcommon.so.0 libGL.so.1 libxcb.so.1 libX11.so.6; do
  have_lib "$lib" || need=1
done
if [ "$need" = 1 ]; then
  say "Installing the libraries Flux needs – you may be asked for your password"
  if command -v pacman >/dev/null; then sudo pacman -S --needed --noconfirm libxkbcommon mesa libxcb libx11 libglvnd
  elif command -v apt-get >/dev/null; then sudo apt-get install -y libxkbcommon0 libxkbcommon-x11-0 libgl1 libxcb1 libx11-6 libwayland-client0 libegl1 || true
  elif command -v dnf >/dev/null; then sudo dnf install -y libxkbcommon mesa-libGL libxcb libX11 libwayland-client mesa-libEGL || true
  elif command -v zypper >/dev/null; then sudo zypper install -y libxkbcommon0 Mesa-libGL1 libxcb1 libX11-6 libwayland-client0 Mesa-libEGL1 || true
  else say "Could not install the libraries automatically – if Flux does not start, install libxkbcommon, OpenGL (mesa) and libxcb."; fi
fi

say "Looking for the newest Flux"
info=$(curl -fsSL "$BASE/native.json") || die "Could not reach $BASE (no Linux build published yet?)"
field() { printf '%s' "$info" | tr -d '\n ' | grep -o "\"$1\":\"[^\"]*\"" | head -n1 | cut -d'"' -f4; }
ver=$(field version); want=$(field sha256)
[ -n "$ver" ] && [ -n "$want" ] || die "Unexpected answer from $BASE/native.json"

say "Downloading Flux $ver"
# starý Flux (Electron AppImage z install-electron.sh) by sa inak mohol spúšťať namiesto nového
if [ -e "$DIR/Flux.AppImage" ] || ls "$APPS"/appimagekit*[Ff]lux*.desktop >/dev/null 2>&1; then
  say "Removing the old Flux (Electron AppImage)"
  rm -f "$DIR/Flux.AppImage" "$APPS"/appimagekit*[Ff]lux*.desktop
fi
mkdir -p "$DIR" "$BIN" "$APPS" "$ICONS"
curl -fL --progress-bar -o "$DIR/flux-native.part" "$BASE/Flux-Native"
got=$(sha256sum "$DIR/flux-native.part" | cut -d' ' -f1)
[ "$got" = "$want" ] || { rm -f "$DIR/flux-native.part"; die "The download is damaged (checksum mismatch). Try again."; }
chmod +x "$DIR/flux-native.part"
mv -f "$DIR/flux-native.part" "$DIR/flux-native"

# príkaz `flux`
cat >"$BIN/flux" <<WRAP
#!/bin/sh
exec "$DIR/flux-native" "\$@"
WRAP
chmod +x "$BIN/flux"

curl -fsSL -o "$DIR/flux.png" "$SITE/icon.png" || true
[ -s "$DIR/flux.png" ] && cp -f "$DIR/flux.png" "$ICONS/flux.png" 2>/dev/null || true
cat >"$APPS/flux.desktop" <<DESK
[Desktop Entry]
Type=Application
Name=Flux
GenericName=Code Editor
Comment=A small, good-looking code editor
Exec=$BIN/flux %F
Icon=$DIR/flux.png
Terminal=false
StartupWMClass=flux
Categories=Development;TextEditor;IDE;
MimeType=text/plain;text/x-python;text/html;text/css;application/javascript;application/json;
DESK
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" 2>/dev/null || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

say "Flux $ver is installed. Open it from your app menu, or run: flux"
say "It updates itself from now on."
case ":$PATH:" in
  *":$BIN:"*) ;;
  *) say "Tip: ~/.local/bin is not in your PATH, so the 'flux' command is not found yet. Add it:"
     printf '     bash/zsh:  echo '\''export PATH="$HOME/.local/bin:$PATH"'\'' >> ~/.%src\n' "$(basename "${SHELL:-bash}")"
     printf '     fish:      fish_add_path ~/.local/bin\n' ;;
esac
# iný „flux“ skôr v PATH (napr. starý Flux z balíčka) by sa spúšťal namiesto tohto
other=$(command -v flux 2>/dev/null || true)
if [ -n "$other" ] && [ "$other" != "$BIN/flux" ]; then
  say "Warning: 'flux' in your terminal runs $other, not the new Flux. Run $BIN/flux or remove the other one."
fi
say "Check: $DIR/flux-native --version"
