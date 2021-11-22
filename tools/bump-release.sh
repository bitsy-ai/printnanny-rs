#!/usr/bin/env bash
set -eu
bump2version --current-version $(cat version.txt) --new-version "$1" patch
git push
git push --tags