#!/bin/sh
set -eu

cd "$(dirname "$0")"

./format.sh --check
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --document-private-items
./test.sh
