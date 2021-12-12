#!/usr/bin/env bash

set +eu

LICENSE_JSON="$(./tools/run.sh api get license)"
export JANUS_TOKEN=$(jq -r .janus_token <<< "$LICENSE_JSON")
export JANUS_ADMIN_SECRET=$(jq -r .janus_admin_secret <<< "$LICENSE_JSON")