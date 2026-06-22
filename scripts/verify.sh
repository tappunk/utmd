#!/bin/bash
set -euo pipefail

echo "[proc] running cargo fmt --check"
cargo fmt --check

echo "[proc] running cargo clippy -- -D warnings"
cargo clippy -- -D warnings

echo "[proc] running cargo test"
cargo test

echo "[success] verification passed"
