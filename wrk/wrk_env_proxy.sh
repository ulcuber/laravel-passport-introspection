#!/bin/bash

set -a
source .env
set +a

if [[ -z "$1" ]]; then
    echo "Provide one of script names:"
    ls wrk/*.lua | sed 's|wrk/||; s|\.lua$||'
    exit 1
fi

wrk \
        --threads "$(($(nproc) * 2))" --connections 400 --duration 60s \
        --script "./wrk/$1.lua" \
        --latency \
        "$PROXY_URL" | wrk2img "wrk/graphs/proxy_$1.png"
