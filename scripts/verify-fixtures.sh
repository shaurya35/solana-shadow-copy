#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")/../fixtures"

if command -v sha256sum >/dev/null 2>&1; then
  sha256sum --check SHA256SUMS
else
  shasum --algorithm 256 --check SHA256SUMS
fi
