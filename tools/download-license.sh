#!/usr/bin/env bash

set +eux

PRINTNANNY_WEBAPP_REPO="${PRINTNANNY_WEBAPP_REPO:-$HOME/projects/octoprint-nanny-webapp}"

get_test_api_token(){
    make -sC "$PRINTNANNY_WEBAPP_REPO" token
}

PRINTNANNY_API_URL="${PRINTNANNY_API_URL:-http://aurora:8000}"
PRINTNANNY_API_TOKEN="${PRINTNANNY_API_TOKEN:-$(get_test_api_token)}"
PRINTNANNY_DEVICE_HOSTNAME="${PRINTNANNY_DEVICE_HOSTNAME:-test}"
PRINTNANNY_INSTALL_DIR="${PRINTNANNY_INSTALL_DIR:-.tmp/test}"
PRINTNANNY_DATA_DIR="${PRINTNANNY_DATA_DIR:-.tmp/test/data}"
PRINTNANNY_CACERT_DIR="${PRINTNANNY_CACERT_DIR:-.tmp/test/ca-certificates}"

echo "$PRINTNANNY_API_TOKEN"

download_ca_certs(){
    wget https://pki.goog/gtsltsr/gtsltsr.crt -P "$PRINTNANNY_CACERT_DIR"
    wget https://pki.goog/gsr4/GSR4.crt -P "$PRINTNANNY_CACERT_DIR"
}

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
    filename="$PRINTNANNY_DATA_DIR/printnanny_license.zip"
    curl -sX "GET" \
        "$PRINTNANNY_API_URL/api/devices/$device_id/generate-license/" \
        -H "accept: application/json" \
        -H "Authorization: Bearer $PRINTNANNY_API_TOKEN" \
        --output $filename
    echo "Created $filename"
}

mkdir -p $PRINTNANNY_DATA_DIR
download_ca_certs
download_license
cd "$PRINTNANNY_DATA_DIR" && unzip -o printnanny_license.zip