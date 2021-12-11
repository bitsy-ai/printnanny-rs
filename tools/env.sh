#!/usr/bin/env bash

set +eu

LICENSE_JSON="$(./tools/run.sh api get license)"
export JANUS_TOKEN="$(jq .janus_token <<< "$LICENSE_JSON")"
export JANUS_ADMIN_SECRET=$(jq .janus_admin_secret <<< "$LICENSE_JSON")