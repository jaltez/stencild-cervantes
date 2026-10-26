#!/bin/sh
# stencild installer (generated). Usage: sh install.sh --prefix DIR
set -e
PREFIX="${HOME}/.local"
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) PREFIX="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
HERE="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$PREFIX/bin" "$PREFIX/share/stencild"
cp "$HERE/stencild" "$PREFIX/bin/stencild"
chmod +x "$PREFIX/bin/stencild"
cp "$HERE/tray-icon.png" "$PREFIX/share/stencild/tray-icon.png"
echo "installed: $PREFIX/bin/stencild"
