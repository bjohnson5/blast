#!/bin/bash
set -e

# Define the directory to check
BLAST_DIR="$HOME/.blast"
TARGET_DIR="$BLAST_DIR/clightning"
LIGHTNINGD_FILE="$TARGET_DIR/lightningd/lightningd"
DOWNLOAD_URL="https://github.com/ElementsProject/lightning.git"

# Check if the 'lightningd' file exists in the specified directory
if [ -f "$LIGHTNINGD_FILE" ]; then
    echo "'lightningd' file exists. Skipping."
else
    echo "'lightningd' file not found. Downloading and extracting..."

    source ~/.venv/bin/activate
    git clone "$DOWNLOAD_URL" "$TARGET_DIR"
    cd "$TARGET_DIR"
    git checkout v24.11rc3
    ./configure
    make

    echo "Download and extraction complete."
    cd -
fi

cd blast_cln
cargo build
