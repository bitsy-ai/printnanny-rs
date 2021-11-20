#!/usr/bin/env bash

PRINTNANNY_WEBAPP_REPO="${PRINTNANNY_WEBAPP_REPO:-$HOME/projects/octoprint-nanny-webapp}"

get_test_api_token(){
    make -sC "$PRINTNANNY_WEBAPP_REPO" token
}

PRINTNANNY_API_URL="${PRINTNANNY_API_URL:-http://localhost:8000}"
PRINTNANNY_API_TOKEN="${PRINTNANNY_API_TOKEN:-$(get_test_api_token)}"
PRINTNANNY_DEVICE_HOSTNAME="${PRINTNANNY_DEVICE_HOSTNAME:-printnanny}"
PRINTNANNY_INSTALL_DIR="${PRINTNANNY_INSTALL_DIR:-.tmp}"

echo "$PRINTNANNY_API_TOKEN"
get_test_device(){
    curl -sX "GET" \
        "$PRINTNANNY_API_URL/api/devices/$PRINTNANNY_DEVICE_HOSTNAME" \
        -H "accept: application/json" \
        -H "Authorization: Bearer $PRINTNANNY_API_TOKEN"
}

get_test_device_id(){
    get_test_device | jq '.id'
}

download_license(){
    device_id=$(get_test_device_id)
    echo "Fetching license for device_id=$device_id"
    filename="$PRINTNANNY_INSTALL_DIR/printnanny_license.zip"
    curl -sX "GET" \
        "$PRINTNANNY_API_URL/api/devices/$device_id/license/" \
        -H "accept: application/zip" \
        -H "Authorization: Bearer $PRINTNANNY_API_TOKEN" \
        --output $filename
    echo "Created $filename"
}

mkdir -p $PRINTNANNY_INSTALL_DIR
download_license