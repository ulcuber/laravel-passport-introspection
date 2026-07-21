#!/bin/bash

set -a
source .env
set +a

ACCESS_TOKEN=$(shuf -n 1 wrk/tokens_RS256.txt)

# -v to print request, -X POST
RESPONSE=$(curl -v -i "$PROXY_URL" \
    -H "Accept: application/json" \
    -H "Authorization: $ACCESS_TOKEN")

# handle Laravel dd()
HTML=$(echo "$RESPONSE" | grep -A1000 "Sfdump")
if [[ -n "$HTML" ]]; then
    echo "$RESPONSE" | grep -C1000 "^(Sfdump)"
    echo "$HTML" > response.html
    echo "dd() detected. See response.html in browser"
    if [[ -n "$BROWSER" ]]; then
        $BROWSER response.html
    fi
else
    echo "$RESPONSE"
fi
