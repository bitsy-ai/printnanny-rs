#!/usr/bin/env bash

set +eu

CONFIG="$PWD/.tmp"

cargo run -- -vv --config="$CONFIG" "$@"