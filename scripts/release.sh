#!/bin/bash
# Release script for helix-anywhere
# This script builds the binary for macOS arm64 and x86_64
# and creates tarballs for distribution

set -e

VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
echo "Building helix-anywhere v$VERSION"

# Create dist directory
rm -rf dist
mkdir -p dist

# Build for arm64 (Apple Silicon)
echo "Building for arm64..."
cargo build --release --target aarch64-apple-darwin
cp target/aarch64-apple-darwin/release/helix-anywhere dist/

# Create tarball for arm64
cd dist
tar -czvf helix-anywhere-darwin-arm64.tar.gz helix-anywhere
ARM64_SHA=$(shasum -a 256 helix-anywhere-darwin-arm64.tar.gz | awk '{print $1}')
echo "arm64 SHA256: $ARM64_SHA"
rm helix-anywhere
cd ..

# Build for x86_64 (Intel)
echo "Building for x86_64..."
cargo build --release --target x86_64-apple-darwin
cp target/x86_64-apple-darwin/release/helix-anywhere dist/

# Create tarball for x86_64
cd dist
tar -czvf helix-anywhere-darwin-x86_64.tar.gz helix-anywhere
X86_SHA=$(shasum -a 256 helix-anywhere-darwin-x86_64.tar.gz | awk '{print $1}')
echo "x86_64 SHA256: $X86_SHA"
rm helix-anywhere
cd ..

echo ""
echo "=== Release files created in dist/ ==="
ls -la dist/
echo ""
echo "=== Update Formula/helix-anywhere.rb with these SHA256 values ==="
echo "arm64:  $ARM64_SHA"
echo "x86_64: $X86_SHA"
echo ""
echo "=== Next steps ==="
echo "1. Create a GitHub release v$VERSION"
echo "2. Upload dist/helix-anywhere-darwin-arm64.tar.gz"
echo "3. Upload dist/helix-anywhere-darwin-x86_64.tar.gz"
echo "4. Create repo: sylvainhellin/homebrew-helix-anywhere"
echo "5. Copy Formula/helix-anywhere.rb to that repo (update SHA256 values)"
