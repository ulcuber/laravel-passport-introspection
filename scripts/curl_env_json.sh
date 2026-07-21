#!/bin/bash

set -a
source .env
set +a

ACCESS_TOKEN=$(shuf -n 1 wrk/tokens_RS256.txt)

curl -i -X POST "http://localhost:${SERVER_PORT}/introspect-json" \
        -H "Accept: application/json" \
        -H "X-Gateway-Secret: $GATEWAY_SECRET"\
        --json '{"token": "'"$ACCESS_TOKEN"'"}'
echo
