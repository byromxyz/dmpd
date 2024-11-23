#!/bin/bash

# Check if the argument is provided
if [ -z "$1" ]; then
  echo "Usage: $0 <NEW_VERSION>"
  exit 1
fi

# Get the first argument as NEW_VERSION
NEW_VERSION=$1

# Determine the platform and update Cargo.toml
if [[ "$OSTYPE" == "darwin"* ]]; then
  # macOS
  sed -i '' "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
else
  # Linux
  sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
fi

echo "Version updated to $NEW_VERSION"
