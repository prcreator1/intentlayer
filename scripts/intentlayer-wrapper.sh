#!/usr/bin/env bash
set -euo pipefail

# IntentLayer agent wrapper — compiles a request JSON file and emits
# the compiled prompt only. Exits non-zero on IntentLayer failure.
#
# Binary resolution order:
#   1. INTENTLAYER_BIN env var
#   2. intentlayer on PATH
#   3. ./target/release/intentlayer (repo-local release)
#   4. ./target/debug/intentlayer (repo-local debug)
#   5. fail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

USAGE="usage: $(basename "$0") <request.json>"

if [[ $# -ne 1 ]]; then
  echo "$USAGE" >&2
  exit 1
fi

INPUT_FILE="$1"

if [[ ! -r "$INPUT_FILE" ]]; then
  echo "File not readable: $INPUT_FILE" >&2
  exit 1
fi

if [[ -n "${INTENTLAYER_BIN:-}" ]]; then
  BIN="${INTENTLAYER_BIN}"
elif command -v intentlayer >/dev/null 2>&1; then
  BIN="$(command -v intentlayer)"
elif [[ -x "${REPO_ROOT}/target/release/intentlayer" ]]; then
  BIN="${REPO_ROOT}/target/release/intentlayer"
elif [[ -x "${REPO_ROOT}/target/debug/intentlayer" ]]; then
  BIN="${REPO_ROOT}/target/debug/intentlayer"
else
  echo "intentlayer binary not found" >&2
  echo "Run: cargo build --release" >&2
  echo "Or set INTENTLAYER_BIN=/path/to/intentlayer" >&2
  exit 1
fi

COMPILED_PROMPT="$("${BIN}" --input "$INPUT_FILE" --compiled-only)"

printf '%s\n' "$COMPILED_PROMPT"
