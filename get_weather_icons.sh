#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# Define constants for clarity and easy maintenance.
REPO_URL="https://github.com/Makin-Things/weather-icons.git"
ASSETS_DIR="./assets"
STATIC_DIR="${ASSETS_DIR}/static"

# Create a temporary directory for cloning the repository.
# This is safer than cloning into a fixed-name directory.
TMP_DIR=$(mktemp -d)

# Setup a trap to ensure the temporary directory is cleaned up on exit,
# even if an error occurs.
trap 'echo "Cleaning up temporary files..."; rm -rf "$TMP_DIR"' EXIT

echo "Cloning weather icons repository into a temporary directory..."
git clone --quiet --depth 1 "$REPO_URL" "$TMP_DIR"

echo "Preparing asset directories..."
# Clean and create the target directory. The upstream "animated" set is
# CSS-@keyframes-only (silently inert once rasterized by iced/resvg) and
# was only ever used as a reference while hand-authoring assets/lottie/ --
# it's intentionally not fetched or tracked anymore.
rm -rf "$STATIC_DIR"
mkdir -p "$STATIC_DIR"

# Check if the static icons directory exists in the cloned repo.
if [ -d "$TMP_DIR/static" ]; then
    echo "Copying static icons..."
    rsync -a --delete --exclude 'README.md' "$TMP_DIR/static/" "$STATIC_DIR/"
else
    echo "No static icons found in the repository, skipping."
fi

echo "Icon download and setup complete."
