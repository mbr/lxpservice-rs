#!/bin/sh
set -eu

cd "$(dirname "$0")"

cargo fmt -- "$@"
nixfmt "$@" flake.nix
