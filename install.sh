#!/usr/bin/env bash
set -euo pipefail

# workctl installer
# Usage: ./install.sh [--prefix /usr/local]

PREFIX="${PREFIX:-$HOME/.local}"
INSTALL_DIR="$PREFIX/bin"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --prefix)
            PREFIX="$2"
            INSTALL_DIR="$PREFIX/bin"
            shift 2
            ;;
        --system)
            PREFIX="/usr/local"
            INSTALL_DIR="$PREFIX/bin"
            shift
            ;;
        -h|--help)
            echo "Usage: ./install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --prefix DIR   Install to DIR/bin (default: ~/.local)"
            echo "  --system       Install to /usr/local/bin (requires sudo)"
            echo "  -h, --help     Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "Building workctl..."
cargo build --release --package workctl-cli

echo "Installing to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"

if [[ "$PREFIX" == "/usr/local" ]]; then
    sudo cp target/release/workctl "$INSTALL_DIR/"
    sudo chmod +x "$INSTALL_DIR/workctl"
else
    cp target/release/workctl "$INSTALL_DIR/"
    chmod +x "$INSTALL_DIR/workctl"
fi

echo ""
echo "✓ Installed workctl to $INSTALL_DIR/workctl"

# Check if in PATH
if ! command -v workctl &> /dev/null; then
    echo ""
    echo "⚠ $INSTALL_DIR is not in your PATH."
    echo "  Add this to your shell profile (~/.bashrc or ~/.zshrc):"
    echo ""
    echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
    echo ""
fi
