COMMIT_HASH=$(git rev-parse --short HEAD)

if [[ "$OSTYPE" == "darwin"* ]]; then
  # macOS
  sed -i '' "s/^version = \"\([0-9]*\.[0-9]*\.[0-9]*\)\"/version = \"\1-$COMMIT_HASH\"/" Cargo.toml
else
  # Linux
  sed -i "s/^version = \"\([0-9]*\.[0-9]*\.[0-9]*\)\"/version = \"\1-$COMMIT_HASH\"/" Cargo.toml
fi
