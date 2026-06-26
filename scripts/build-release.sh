#!/usr/bin/env bash
set -euo pipefail

# Build release artifacts for IntentLayer.
# No live API calls. No secrets committed.

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
VERSION="$(cargo metadata --format-version 1 --no-deps --locked 2>/dev/null | grep -o '"version":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "0.1.0")"
ARTIFACT="intentlayer-${OS}-${ARCH}"
BUILD_TARGET_DIR="${CARGO_TARGET_DIR:-target}"
RELEASE_DIR="${BUILD_TARGET_DIR}/release"
DIST_DIR="dist/${VERSION}"

echo "=== Building IntentLayer v${VERSION} (${OS}-${ARCH}) ==="

# Build release binary (locked — no Cargo.lock mutation)
cargo build --release --locked --target-dir "${BUILD_TARGET_DIR}"

# Validate binary exists
if [[ ! -x "${RELEASE_DIR}/intentlayer" ]]; then
  echo "Release binary not found: ${RELEASE_DIR}/intentlayer" >&2
  exit 1
fi

# Create dist directory
mkdir -p "${DIST_DIR}"

# Copy binary
cp "${RELEASE_DIR}/intentlayer" "${DIST_DIR}/${ARTIFACT}"

# Copy docs
cp README.md LICENSE CHANGELOG.md "${DIST_DIR}/" 2>/dev/null || true

# Generate checksums (artifact-relative paths inside DIST_DIR)
(
  cd "${DIST_DIR}"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${ARTIFACT}" > sha256sums.txt
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "${ARTIFACT}" > sha256sums.txt
  else
    echo "No SHA-256 tool found" >&2
    exit 1
  fi
)

# Verify checksums
(
  cd "${DIST_DIR}"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum -c sha256sums.txt
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 -c sha256sums.txt
  else
    echo "No SHA-256 tool found for verification" >&2
    exit 1
  fi
)

echo "=== Artifacts ==="
ls -la "${DIST_DIR}/"

echo "=== Done ==="
