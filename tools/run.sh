#!/usr/bin/env bash

set +eu

CONFIG="$PWD/.tmp/test"

cargo run -- -vv --config="$CONFIG" "$@"