#!/usr/bin/env bash
set -euo pipefail

# IntentLayer agent wrapper — compiles a request JSON file and emits
# the compiled prompt only. Exits non-zero on IntentLayer failure.

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

if ! command -v intentlayer >/dev/null 2>&1; then
  echo "intentlayer binary not found in PATH" >&2
  echo "Build it first: cargo build --release" >&2
  exit 1
fi

COMPILED_PROMPT="$(intentlayer --input "$INPUT_FILE" --compiled-only)"

printf '%s\n' "$COMPILED_PROMPT"
