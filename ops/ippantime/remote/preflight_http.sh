#!/usr/bin/env bash
set -euo pipefail
URL="${1:?missing url}"
echo "curl $URL"
curl -fsS --max-time 2 "$URL" | head -c 400
echo
