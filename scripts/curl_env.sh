#!/bin/bash

set -a
source .env
set +a

ACCESS_TOKEN=$(shuf -n 1 wrk/tokens_RS256.txt)

curl -X POST "http://localhost:${SERVER_PORT}/introspect" \
    -H "Accept: application/json" \
    -H "X-Gateway-Secret: $GATEWAY_SECRET" \
    --data "token=$ACCESS_TOKEN"
echo
