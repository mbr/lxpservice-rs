#!/bin/sh
set -eu

cd "$(dirname "$0")"

unset LXP_USERNAME LXP_API_KEY LXP_MODE LXP_STATE_DIR
cargo test --locked
