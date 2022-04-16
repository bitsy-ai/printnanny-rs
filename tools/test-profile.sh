#!/bin/bash

PREFIX="${PRINTNANNY_PREFIX:-.tmp/test}"

CA_CERTS="$PREFIX/ca-certificates"
KEYS="${PREFIX}/keys"
DATA_DIR = "${PREFIX}/data"
mkdir -p "$KEYS"
mkdir -p "$CA_CERTS"
mkdir -p "$DATA_DIR"
openssl ecparam -genkey -name prime256v1 | openssl pkcs8 -topk8 -nocrypt -out "${KEYS}/ec_private.pem"
openssl ec -in "${KEYS}/ec_private.pem" -pubout -out "${KEYS}/ec_public.pem"

curl https://pki.goog/gtsltsr/gtsltsr.crt > "$CA_CERTS/gtsltsr.crt"
echo "Creating ${KEYS}/janus_admin_secret"
echo "test_janus_admin_secret" > "${KEYS}/janus_admin_secret"
echo "Creating ${KEYS}/janus_token"
echo "test_janus_token" > "${KEYS}/janus_token"x