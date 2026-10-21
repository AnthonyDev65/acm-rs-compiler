#!/bin/sh
# Instalador de acm para macOS y Linux (no necesita Rust ni cargo).
#
#   curl -fsSL https://raw.githubusercontent.com/AnthonyDev65/acm-rs-compiler/main/install.sh | sh
#
# Descarga el binario de la última Release de GitHub y lo deja en ~/.local/bin
# (o en $ACM_INSTALL_DIR). Para una versión concreta: ACM_VERSION=v0.1.0
set -eu

repo="AnthonyDev65/acm-rs-compiler"

case "$(uname -s)" in
  Darwin) os=macos ;;
  Linux) os=linux ;;
  *) echo "Sistema no soportado: $(uname -s). En Windows usa install.ps1." >&2; exit 1 ;;
esac
case "$(uname -m)" in
  x86_64 | amd64) arch=x86_64 ;;
  arm64 | aarch64) arch=aarch64 ;;
  *) echo "Arquitectura no soportada: $(uname -m)" >&2; exit 1 ;;
esac
if [ "$os" = linux ] && [ "$arch" != x86_64 ]; then
  echo "En Linux solo hay binarios para x86_64." >&2
  exit 1
fi

asset="acm-$os-$arch"
if [ -n "${ACM_VERSION:-}" ]; then
  url="https://github.com/$repo/releases/download/$ACM_VERSION/$asset"
else
  url="https://github.com/$repo/releases/latest/download/$asset"
fi

bin_dir="${ACM_INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$bin_dir"
tmp="$bin_dir/acm.download"

echo "Descargando $url"
if command -v curl >/dev/null 2>&1; then
  curl -fsSL "$url" -o "$tmp"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$tmp" "$url"
else
  echo "Hace falta curl o wget." >&2
  exit 1
fi

chmod +x "$tmp"
# macOS marca los binarios descargados y Gatekeeper los bloquea.
if [ "$os" = macos ]; then xattr -d com.apple.quarantine "$tmp" 2>/dev/null || true; fi
mv -f "$tmp" "$bin_dir/acm"

echo "$("$bin_dir/acm" --version) instalado en $bin_dir/acm"
case ":$PATH:" in
  *":$bin_dir:"*) ;;
  *) echo "Añade $bin_dir al PATH, por ejemplo en ~/.bashrc o ~/.zshrc:"
     echo "  export PATH=\"$bin_dir:\$PATH\"" ;;
esac
