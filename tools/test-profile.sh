#!/bin/bash

PREFIX="${PRINTNANNY_PREFIX:-.tmp/test}"

KEYS="${PREFIX}/keys"
mkdir -p "$KEYS"
openssl ecparam -genkey -name prime256v1 -noout -out "${KEYS}/ec_private.pem"
openssl ec -in "${KEYS}/ec_private.pem" -pubout -out "${KEYS}/ec_public.pem"

echo "Creating ${KEYS}/janus_admin_secret"
echo "test_janus_admin_secret" > "${KEYS}/janus_admin_secret"
echo "Creating ${KEYS}/janus_token"
echo "test_janus_token" > "${KEYS}/janus_token"