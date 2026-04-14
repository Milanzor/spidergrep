#!/bin/sh
set -e

REPO="Milanzor/spidergrep"
BIN="spidergrep"
INSTALL_DIR="/usr/local/bin"

# Detect OS
case "$(uname -s)" in
  Linux)  OS="linux" ;;
  Darwin) OS="darwin" ;;
  *)
    echo "Unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac

# Detect architecture
case "$(uname -m)" in
  x86_64|amd64) ARCH="x86_64" ;;
  arm64|aarch64) ARCH="aarch64" ;;
  *)
    echo "Unsupported architecture: $(uname -m)" >&2
    exit 1
    ;;
esac

# Map to release target name
if [ "$OS" = "linux" ]; then
  TARGET="${ARCH}-unknown-linux-musl"
else
  TARGET="${ARCH}-apple-darwin"
fi

# Fetch the latest release tag
echo "Fetching latest release..."
TAG=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "$TAG" ]; then
  echo "Could not determine latest release tag." >&2
  exit 1
fi

ARCHIVE="${BIN}-${TAG}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${TAG}/${ARCHIVE}"

echo "Downloading ${BIN} ${TAG} for ${TARGET}..."
curl -fsSL "$URL" -o "/tmp/${ARCHIVE}"

echo "Extracting..."
tar -xzf "/tmp/${ARCHIVE}" -C /tmp

# Install (try /usr/local/bin, fall back to ~/.local/bin)
if [ -w "$INSTALL_DIR" ]; then
  mv "/tmp/${BIN}" "${INSTALL_DIR}/${BIN}"
elif command -v sudo >/dev/null 2>&1; then
  sudo mv "/tmp/${BIN}" "${INSTALL_DIR}/${BIN}"
else
  INSTALL_DIR="$HOME/.local/bin"
  mkdir -p "$INSTALL_DIR"
  mv "/tmp/${BIN}" "${INSTALL_DIR}/${BIN}"
  echo "Installed to ${INSTALL_DIR}/${BIN} (add ~/.local/bin to your PATH if needed)"
fi

chmod +x "${INSTALL_DIR}/${BIN}"
rm -f "/tmp/${ARCHIVE}"

echo "Installed ${BIN} ${TAG} to ${INSTALL_DIR}/${BIN}"
