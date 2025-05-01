#!/usr/bin/env bash

set -euo pipefail

# =============================================================================
# Define helper functions
# =============================================================================

text_bold() {
    echo -e "\033[1m$1\033[0m"
}
text_title() {
    echo ""
    text_bold "$1"
    if [ "$2" != "" ]; then echo "$2"; fi
}
text_title_error() {
    echo ""
    echo -e "\033[1;31m$1\033[00m"
}

# =============================================================================
# Define base variables
# =============================================================================

NAME="dmpd"
# Latest
GITHUB_REPO="byromxyz/dmpd"
VERSION=$(curl -s https://api.github.com/repos/$GITHUB_REPO/releases/latest | grep -oE '"tag_name": "[^"]+"' | cut -d '"' -f 4)

DOWNLOAD_BASE_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION"

# =============================================================================
# Get the user's OS and Arch
# =============================================================================

OS="$(uname -s)"
ARCH="$(uname -m)"
SYSTEM="${OS}:${ARCH}"

# =============================================================================
# Define binary list for supported OS & Arch
# - this is a map of "OS:Arch" -> "download binary name"
# - you can remove or add to this list as needed
# =============================================================================

get_binary_name() {
    case "${OS}:${ARCH}" in
    "Darwin:x86_64") echo "dmpd-${VERSION}-x86_64-apple-darwin" ;;
    "Darwin:arm64") echo "dmpd-${VERSION}-aarch64-apple-darwin" ;;
    *)
        echo ""
        return 1
        ;;
    esac
}

BINARY=$(get_binary_name)
if [ -z "$BINARY" ]; then
    text_title_error "Error"
    echo " Unsupported OS or arch: ${OS}:${ARCH}"
    echo ""
    exit 1
fi

# =============================================================================
# Set the default installation variables
# =============================================================================

INSTALL_DIR="/usr/local/bin"
DOWNLOAD_URL="$DOWNLOAD_BASE_URL/$BINARY"

# =============================================================================
# Create and change to temp directory
# =============================================================================

cd "$(mktemp -d)"

# =============================================================================
# Download binary
# =============================================================================

text_title "Downloading Binary" " $DOWNLOAD_URL"
curl -LO --proto '=https' --tlsv1.2 -sSf "$DOWNLOAD_URL"

# =============================================================================
# Make binary executable and move to install directory with appropriate name
# =============================================================================

text_title "Installing Binary" " $INSTALL_DIR/$NAME"
chmod +x "$BINARY"
mv "$BINARY" "$INSTALL_DIR/$NAME"

# =============================================================================
# Display post install message
# =============================================================================

text_title "Installation Complete" " Run $NAME --help for more information"
echo ""
