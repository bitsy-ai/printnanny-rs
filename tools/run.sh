#!/usr/bin/env bash

set +eu

CONFIG="$PWD/.tmp/printnanny"

cargo run -- -vv --config="$CONFIG" "$@"