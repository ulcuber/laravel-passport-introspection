#!/bin/bash

set -a
source .env
set +a

RESPONSE=$(curl -i -X GET "$APP_URL" \
    -H "Accept: application/json" \
    -H "X-Gateway-Secret: $APP_GATEWAY_SECRET" \
    -H "X-User-Id: 1" \
    -H "X-Client-Id: $CLIENT_ID" \
    -H "X-Scope: openid")

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
