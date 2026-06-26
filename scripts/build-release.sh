#!/usr/bin/env bash
set -euo pipefail

# Build release artifacts for IntentLayer.
# No live API calls. No secrets committed.

VERSION="$(cargo metadata --format-version 1 --no-deps 2>/dev/null | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "0.1.0")"
TARGET_DIR="target/release"
DIST_DIR="dist/${VERSION}"

echo "=== Building IntentLayer v${VERSION} ==="

# Build release binary
cargo build --release --locked

# Create dist directory
mkdir -p "${DIST_DIR}"

# Copy binary
cp "${TARGET_DIR}/intentlayer" "${DIST_DIR}/intentlayer-linux-x86_64"

# Generate checksums
if command -v sha256sum &>/dev/null; then
    sha256sum "${DIST_DIR}/intentlayer-linux-x86_64" > "${DIST_DIR}/sha256sums.txt"
elif command -v shasum &>/dev/null; then
    shasum -a 256 "${DIST_DIR}/intentlayer-linux-x86_64" > "${DIST_DIR}/sha256sums.txt"
fi

# Copy docs
cp README.md LICENSE CHANGELOG.md "${DIST_DIR}/" 2>/dev/null || true

echo "=== Artifacts ==="
ls -la "${DIST_DIR}/"

echo "=== Done ==="
