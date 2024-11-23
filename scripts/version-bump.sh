#!/bin/bash

# Check if the correct arguments are provided
if [ $# -ne 1 ]; then
  echo "Usage: $0 <patch|minor|major|preview>"
  exit 1
fi

# Determine which part to bump
BUMP_TYPE=$1

# Extract the current version from Cargo.toml
CURRENT_VERSION=$(grep -E '^version\s*=\s*"[0-9]+\.[0-9]+\.[0-9]+"' Cargo.toml | cut -d'"' -f2)
if [ -z "$CURRENT_VERSION" ]; then
  echo "Error: Could not find the version in Cargo.toml"
  exit 1
fi

# Split the version into components
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"

# Bump the appropriate part of the version or create a preview version
case $BUMP_TYPE in
  patch)
    PATCH=$((PATCH + 1))
    ;;
  minor)
    MINOR=$((MINOR + 1))
    PATCH=0
    ;;
  major)
    MAJOR=$((MAJOR + 1))
    MINOR=0
    PATCH=0
    ;;
  preview)
    TIMESTAMP=$(date +%s)
    GIT_SHA=$(git rev-parse --short HEAD)
    NEW_VERSION="${CURRENT_VERSION}-${TIMESTAMP}.${GIT_SHA}"
    ;;
  *)
    echo "Error: Invalid bump type. Use 'patch', 'minor', 'major', or 'preview'."
    exit 1
    ;;
esac

# Construct the new version if not already set
if [ "$BUMP_TYPE" != "preview" ]; then
  NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
fi

# Write the new version to GITHUB_OUTPUT if available
if [ -n "$GITHUB_OUTPUT" ]; then
  echo "new_version=$NEW_VERSION" >> "$GITHUB_OUTPUT"
fi

echo "New version: $NEW_VERSION"
