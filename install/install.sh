#!/usr/bin/env bash
# Installs the pathfinder CLI for Satisfactory factory planning.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/korti11/satisfactory-pathfinder/master/install/install.sh | bash
#
# Options (environment variables):
#   INSTALL_DIR   Override the default install directory (default: ~/.local/bin)

set -euo pipefail

REPO="korti11/satisfactory-pathfinder"
BINARY_NAME="pathfinder"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Colours
CYAN='\033[0;36m'
GREEN='\033[0;32m'
GRAY='\033[0;90m'
WHITE='\033[0;97m'
RESET='\033[0m'

step()    { echo -e "  ${CYAN}$*${RESET}"; }
success() { echo -e "  ${GREEN}$*${RESET}"; }

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64) ARCHIVE="pathfinder-linux-x86_64.tar.gz" ;;
            *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
        esac
        ;;
    Darwin)
        case "$ARCH" in
            arm64)  ARCHIVE="pathfinder-macos-arm64.tar.gz" ;;
            x86_64) ARCHIVE="pathfinder-macos-x86_64.tar.gz" ;;
            *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
        esac
        ;;
    *)
        echo "Unsupported OS: $OS" >&2
        exit 1
        ;;
esac

echo ""
echo -e "${WHITE}pathfinder installer${RESET}"
echo -e "${WHITE}====================${RESET}"

# Fetch latest release version
step "Fetching latest release..."
VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
step "Latest version: $VERSION"

# Download and extract
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/$ARCHIVE"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

step "Downloading $ARCHIVE..."
curl -fsSL "$DOWNLOAD_URL" | tar -xz -C "$TMP_DIR"

# Install binary
step "Installing to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
install -m 755 "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"

# PATH hint if needed
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo ""
    step "Add $INSTALL_DIR to your PATH by adding this to your shell profile:"
    echo ""
    echo -e "  ${WHITE}export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}"
fi

echo ""
success "pathfinder $VERSION installed successfully."
echo ""
echo -e "  ${GRAY}Run to verify:${RESET}"
echo -e "  ${WHITE}pathfinder --version${RESET}"
echo ""
echo -e "  ${GRAY}To install the companion agent for Claude Code:${RESET}"
echo -e "  ${WHITE}pathfinder companion install --global${RESET}"
echo ""
