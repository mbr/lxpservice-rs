#!/bin/sh
set -eu

cd "$(dirname "$0")"

./format.sh --check
# Keep upstream lint warnings non-fatal while bootstrapping the fork.
cargo clippy --all-targets
./test.sh
