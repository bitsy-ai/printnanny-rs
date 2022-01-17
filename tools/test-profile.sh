#!/bin/bash

PREFIX="${PRINTNANNY_PREFIX:-.tmp/test}"

KEYS="${PREFIX}/keys"
mkdir -p "$KEYS"
openssl ecparam -genkey -name prime256v1 -noout -out "${KEYS}/id_ecdsa"
openssl ec -in "${KEYS}/id_ecdsa" -pubout -out "${KEYS}/id_ecdsa.pub"

echo "Creating ${KEYS}/janus_admin_secret"
echo "test_janus_admin_secret" > "${KEYS}/janus_admin_secret"
echo "Creating ${KEYS}/janus_token"
echo "test_janus_token" > "${KEYS}/janus_token"