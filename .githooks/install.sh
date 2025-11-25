#!/usr/bin/env bash
set -euo pipefail
git config core.hooksPath .githooks
echo "Installed hooks: core.hooksPath=.githooks"

