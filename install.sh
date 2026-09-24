#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

echo "==> cargo build --release"
cargo build --release

BIN="target/release/mget"
if [[ ! -f "$BIN" ]]; then
  echo "error: missing $BIN after build" >&2
  exit 1
fi

echo "==> install to /usr/local/bin/mget (requires sudo)"
sudo install -m 755 "$BIN" /usr/local/bin/mget

echo "==> done: $(command -v mget)"
