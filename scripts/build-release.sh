#!/usr/bin/env bash
set -euo pipefail

# Build release artifacts for IntentLayer.
# No live API calls. No secrets committed.

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
VERSION="$(cargo metadata --format-version 1 --no-deps --locked 2>/dev/null | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "0.1.0")"
ARTIFACT="intentlayer-${OS}-${ARCH}"
TARGET_DIR="target/release"
DIST_DIR="dist/${VERSION}"

echo "=== Building IntentLayer v${VERSION} (${OS}-${ARCH}) ==="

# Build release binary (locked — no Cargo.lock mutation)
cargo build --release --locked

# Create dist directory
mkdir -p "${DIST_DIR}"

# Copy binary
cp "${TARGET_DIR}/intentlayer" "${DIST_DIR}/${ARTIFACT}"

# Generate checksums
if command -v sha256sum &>/dev/null; then
    sha256sum "${DIST_DIR}/${ARTIFACT}" > "${DIST_DIR}/sha256sums.txt"
elif command -v shasum &>/dev/null; then
    shasum -a 256 "${DIST_DIR}/${ARTIFACT}" > "${DIST_DIR}/sha256sums.txt"
fi

# Copy docs
cp README.md LICENSE CHANGELOG.md "${DIST_DIR}/" 2>/dev/null || true

echo "=== Artifacts ==="
ls -la "${DIST_DIR}/"

echo "=== Done ==="
