#!/usr/bin/env bash
set -eu
bump2version --current-version $(cat version.txt) --new-version "$1" patch
cargo check
git commit --amend --no-edit
git push
git push --tags