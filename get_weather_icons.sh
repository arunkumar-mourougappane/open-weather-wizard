#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# Define constants for clarity and easy maintenance.
REPO_URL="https://github.com/Makin-Things/weather-icons.git"
ASSETS_DIR="./assets"
ANIMATED_DIR="${ASSETS_DIR}/animated"
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
# Clean and create the target directories.
rm -rf "$ANIMATED_DIR" "$STATIC_DIR"
mkdir -p "$ANIMATED_DIR" "$STATIC_DIR"

echo "Copying animated icons..."
# Use rsync with --delete to ensure the destination is an exact copy.
# The trailing slash on the source directory is important.
rsync -a --delete --exclude 'README.md' "$TMP_DIR/animated/" "$ANIMATED_DIR/"

# Check if the static icons directory exists in the cloned repo.
if [ -d "$TMP_DIR/static" ]; then
    echo "Copying static icons..."
    rsync -a --delete --exclude 'README.md' "$TMP_DIR/static/" "$STATIC_DIR/"
else
    echo "No static icons found in the repository, skipping."
fi

echo "Icon download and setup complete."
